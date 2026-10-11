//! The picker of a foreign key cell's editor: what it searches for, what
//! it lists and what Enter takes. No UI, and nothing here talks to the
//! backend: `app::key_picker` sends what this decides.

use std::ops::Range;
use std::time::{Duration, Instant};

use tabletist_db::{
    ColumnClass, ColumnInfo, Dialect, Error, Lookup, LookupPage, LookupPart, LookupRow, Matched,
    NewValue, ObjectRef, Structure, Value, column_class,
};

use crate::backend::RequestId;
use crate::edit::{DoubtState, Edits, State, Table, new_id};
use crate::model::Advance;

/// How many matches a list holds; more are said, not paged.
pub const LIMIT: u32 = 50;
/// How long after the last keystroke a search is sent.
pub const DELAY: Duration = Duration::from_millis(150);
/// How many options show at once; the list follows the highlight.
pub const VISIBLE: usize = 8;

/// The table and the column that the one-column foreign key of `column`
/// points at. `None` for a column no such key names, and for a key whose
/// referenced column the catalog does not say.
pub fn target(structure: &Structure, column: &str) -> Option<(ObjectRef, String)> {
    let foreign = structure
        .foreign_keys
        .iter()
        .find(|foreign| foreign.columns.len() == 1 && foreign.columns[0] == column)?;
    let [key] = foreign.ref_columns.as_slice() else {
        return None;
    };
    let table = ObjectRef::new(foreign.ref_schema.clone(), foreign.ref_table.clone());
    Some((table, key.clone()))
}

/// A foreign key cell whose pending value names no row.
pub struct Unmatched {
    /// The cell, by row and column of the page.
    pub at: (usize, usize),
    /// The table its key points at, and the column there.
    pub table: String,
    pub key: String,
    /// The value, as the database reads it: the text typed, without the
    /// white space around a whole number (`key_text`).
    pub value: String,
}

/// The warnings of `edits` that stand, for the table `own` whose page
/// `table` is: a cell that was answered "no such row", still holds the
/// text that was asked about, and waits for a save with nothing to fix.
/// Not one whose key a new row of the same tab gives the referenced
/// column: a table that points at itself can have one new row name
/// another, and the save writes both.
pub fn unmatched(table: &Table<'_>, own: &ObjectRef, edits: &Edits) -> Vec<Unmatched> {
    let Some(structure) = table.structure else {
        return Vec::new();
    };
    edits
        .doubts
        .iter()
        .filter(|(_, doubt)| doubt.state == DoubtState::Missing)
        .filter_map(|(&at, doubt)| {
            let pending = edits.cells.get(&at)?;
            // A cell whose save failed shows the failure instead.
            let holds = match (&pending.new, &pending.state) {
                (NewValue::Text(text), State::Ready) => *text == doubt.text,
                (NewValue::Text(_), State::ToFix(_) | State::Failed(_)) => false,
                (
                    NewValue::Null | NewValue::Default { .. } | NewValue::Now,
                    State::Ready | State::ToFix(_) | State::Failed(_),
                ) => false,
            };
            if !holds {
                return None;
            }
            let column = table.page.columns.get(at.1)?;
            let (target, key) = self::target(structure, &column.name)?;
            if target == *own {
                // The referenced column's place in this page, and whether
                // a new row sets it to this value.
                let place = table
                    .page
                    .columns
                    .iter()
                    .position(|column| column.name == key);
                let made = place.is_some_and(|place| {
                    edits.cells.iter().any(|(&(row, col), cell)| {
                        col == place
                            && new_id(row).is_some()
                            && matches!(&cell.new, NewValue::Text(text) if *text == doubt.text)
                    })
                });
                if made {
                    return None;
                }
            }
            let class = table.class(at.1)?;
            Some(Unmatched {
                at,
                table: target.name,
                key,
                value: key_text(class, &doubt.text).to_owned(),
            })
        })
        .collect()
}

/// The column a person reads a row of the referenced table by: its first
/// text column that is not the key.
pub fn label_column(dialect: Dialect, columns: &[ColumnInfo], key: &str) -> Option<String> {
    columns
        .iter()
        .filter(|column| column.name != key)
        .find(|column| {
            matches!(
                column_class(dialect, &column.type_name),
                ColumnClass::Text { .. }
            )
        })
        .map(|column| column.name.clone())
}

/// A UUID as it is written: 36 characters with their hyphens, or 32 hex
/// digits.
fn is_uuid(text: &str) -> bool {
    let hex = |part: &str| part.bytes().all(|byte| byte.is_ascii_hexdigit());
    match text.len() {
        32 => hex(text),
        36 => {
            let mut parts = text.split('-');
            [8, 4, 4, 4, 12].into_iter().all(|len| {
                parts
                    .next()
                    .is_some_and(|part| part.len() == len && hex(part))
            }) && parts.next().is_none()
        }
        _ => false,
    }
}

/// The key a pending `text` of a foreign key column of `class` names:
/// the column's check overlooks the ASCII white space around a whole
/// number (`edit::check`), and so does the database. Any other text is
/// the key as typed: white space is part of a text key, and the check of
/// a `uuid` does not trim it.
pub fn key_text(class: ColumnClass, text: &str) -> &str {
    match class {
        ColumnClass::Integer { .. } => text.trim_ascii(),
        ColumnClass::Decimal { .. }
        | ColumnClass::Float
        | ColumnClass::Text { .. }
        | ColumnClass::Boolean
        | ColumnClass::Json
        | ColumnClass::Binary
        | ColumnClass::Other => text,
    }
}

/// `text` as the key to read one row by with `=`, where it reads as a
/// value of the key's type. `passes` says the foreign key column's own
/// check takes it. Only a key that can be compared exactly is pinned: a
/// whole number that fits an `i64` goes as one, text goes as text, and a
/// `uuid` goes as text where it is written as one (any other text is an
/// error on PostgreSQL). A whole number too wide for an `i64` is not
/// pinned, nor is a decimal or a float: MySQL compares a string with a
/// number as floats, and a neighbouring key would match. The matches'
/// read, which compares the key as text, still finds them. A key of a
/// type the app does not know is never read by `=`.
pub fn pinned(text: &str, class: ColumnClass, type_name: &str, passes: bool) -> Option<Value> {
    if text.is_empty() || !passes {
        return None;
    }
    let as_text = || Value::Text(text.into());
    match class {
        // The column's check lets a keyword through (`NULL`, `DEFAULT`),
        // which is no key, and does not parse.
        ColumnClass::Integer { .. } => text.parse::<i64>().ok().map(Value::Int),
        // Compared as floats by MySQL, so never exact.
        ColumnClass::Decimal { .. } | ColumnClass::Float => None,
        ColumnClass::Text { .. } => Some(as_text()),
        ColumnClass::Other if type_name.eq_ignore_ascii_case("uuid") => is_uuid(text).then(as_text),
        ColumnClass::Boolean | ColumnClass::Json | ColumnClass::Binary | ColumnClass::Other => None,
    }
}

/// Where `text` is first found in `label`, whatever its case: the bytes
/// of `label` to draw as the match. The engine's own comparison decided
/// that the row matched; this is only for the bold.
pub fn found_in(label: &str, text: &str) -> Option<Range<usize>> {
    if text.is_empty() {
        return None;
    }
    let needle: Vec<char> = text.chars().flat_map(char::to_lowercase).collect();
    let starts: Vec<usize> = label.char_indices().map(|(at, _)| at).collect();
    starts.iter().find_map(|&start| {
        let mut end = start;
        let mut matched = 0;
        for character in label[start..].chars() {
            for lower in character.to_lowercase() {
                if needle.get(matched) != Some(&lower) {
                    return None;
                }
                matched += 1;
            }
            end += character.len_utf8();
            if matched == needle.len() {
                return Some(start..end);
            }
        }
        None
    })
}

/// What is known of the referenced table's label column.
#[derive(Clone, PartialEq)]
pub enum Columns {
    /// Its columns are being fetched.
    Waiting,
    /// The label column, or none: the table is searched by its key alone.
    Known(Option<String>),
}

/// The search that was last sent.
pub struct Asked {
    /// What it is for: the editor's text without the spaces at its ends,
    /// or nothing.
    pub text: String,
    /// The read of the pinned row, while it runs.
    pub pinned: Option<RequestId>,
    /// The read of the matches, while it runs.
    pub matching: Option<RequestId>,
    /// The pinned row's answer once it came: the row, or none.
    pub first: Option<Option<LookupRow>>,
}

/// What Enter or Tab takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Take {
    /// The text as typed: today's commit.
    Text,
    /// The option at this place of the list.
    Option(usize),
    /// Nothing yet: the list is not the answer for the text that shows.
    Wait,
}

/// What a picker made of an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answered {
    /// Not a read this picker waits for.
    NotMine,
    /// Taken: the list is the answer's, or says why there is none.
    Taken,
    /// It came back cancelled, and the picker cancels nothing while its
    /// editor is open: the search is to be sent again.
    Resend,
}

/// The picker of an open editor.
pub struct Picker {
    /// The table the key points at.
    pub target: ObjectRef,
    /// Its column the key references.
    pub key: String,
    pub label: Columns,
    /// Which picker this is, among every one the app opened: a lookup is
    /// sent in its name, and one whose picker is gone is an editor's that
    /// closed.
    pub serial: u64,
    /// How often a read that came back cancelled unasked was sent again
    /// for the text that shows. Once is enough for a cancel that was
    /// late: twice is the session's trouble.
    pub resends: u8,
    /// The row the cell pointed at when the editor opened on its value:
    /// listed first and marked while nothing is searched for.
    pub current: Option<Value>,
    /// The text is searched for. False while an editor that opened on its
    /// value has not been typed into: the search is for nothing.
    pub searching: bool,
    /// The editor's text when it was last looked at: `Action::EditorTyped`
    /// is also sent for a text that did not change.
    seen: String,
    pub asked: Option<Asked>,
    pub options: Vec<LookupRow>,
    /// There are more matches than the list holds.
    pub more: bool,
    /// The text the last answer of the matches was for. `None` before the
    /// first answer. After a pinned row's answer the first option can
    /// belong to a newer search than this names.
    pub listed: Option<String>,
    pub highlight: Option<usize>,
    /// The user moved the highlight since the list last changed.
    pub moved: bool,
    /// When the next search is to be sent.
    pub due: Option<Instant>,
    /// Enter or Tab, pressed before the list answered the text.
    pub kept: Option<Advance>,
    /// Why there is no list: the database's words.
    pub failed: Option<Error>,
    /// A commit took the text and its column refused it. Until then what
    /// the column would say of the text is not shown: the text is a
    /// search, and `harb` is no mistake in a column of numbers.
    pub refused: bool,
    /// The editor's text is the key of an option a commit took from the
    /// list: it was just read from the table, and is not asked about
    /// again.
    pub taken: bool,
}

impl Picker {
    /// The picker of an editor that opens with `text`. `searching` is
    /// false for one opened on its cell's value.
    pub fn new(
        (target, key): (ObjectRef, String),
        label: Columns,
        current: Option<Value>,
        searching: bool,
        text: &str,
    ) -> Self {
        Self {
            target,
            key,
            label,
            serial: 0,
            resends: 0,
            current,
            searching,
            seen: text.to_owned(),
            asked: None,
            options: Vec::new(),
            more: false,
            listed: None,
            highlight: None,
            moved: false,
            due: None,
            kept: None,
            failed: None,
            refused: false,
            taken: false,
        }
    }

    /// What is searched for while the editor holds `typed`.
    pub fn text<'a>(&self, typed: &'a str) -> &'a str {
        if self.searching { typed.trim() } else { "" }
    }

    /// The editor's text is `typed` now. A text that changed is searched
    /// for `delay` from `now`, and what waited for the old one is dropped.
    pub fn typed(&mut self, typed: &str, now: Instant, delay: Duration) {
        if typed == self.seen {
            return;
        }
        // A failure does not outlive the text it was for: the list is
        // searching again, and Enter waits for its answer. Not while the
        // columns are unknown: that failure is not this text's, and
        // nothing would search again.
        if self.label != Columns::Waiting && self.failed.take().is_some() {
            self.listed = None;
        }
        self.seen = typed.to_owned();
        self.searching = true;
        self.due = Some(now + delay);
        self.kept = None;
        self.moved = false;
        self.refused = false;
        self.resends = 0;
        self.taken = false;
    }

    /// The editor's text was replaced by an option's key. It is the text
    /// the picker last saw, so the `EditorTyped` that may follow starts no
    /// search for it.
    pub fn put(&mut self, text: &str) {
        self.seen = text.to_owned();
    }

    /// Whether the last search sent was for the text the editor holds
    /// now. A commit that waits for an answer waits in vain when it was
    /// not.
    pub fn asked_for(&self, typed: &str) -> bool {
        let text = self.text(typed);
        self.asked.as_ref().is_some_and(|asked| asked.text == text)
    }

    /// The two reads of a search for `text`: the pinned row's, where `pin`
    /// names one, and the matches'.
    pub fn lookups(
        &self,
        label: Option<String>,
        text: &str,
        pin: Option<Value>,
    ) -> (Option<Lookup>, Lookup) {
        let read = |part: LookupPart, limit: u32| Lookup {
            object: self.target.clone(),
            key: self.key.clone(),
            label: label.clone(),
            part,
            limit,
        };
        (
            pin.map(|value| read(LookupPart::Pinned(value), 1)),
            read(LookupPart::Matching(text.to_owned()), LIMIT),
        )
    }

    /// The requests whose answers this picker still waits for.
    pub fn in_flight(&self) -> Vec<RequestId> {
        self.asked
            .iter()
            .flat_map(|asked| [asked.pinned, asked.matching])
            .flatten()
            .collect()
    }

    /// Whether `key` is the row the cell pointed at when the editor
    /// opened.
    pub fn is_current(&self, key: &Value) -> bool {
        self.current.as_ref() == Some(key)
    }

    /// Takes the answer of `request`, if it is one this picker waits for.
    /// Says what it made of it.
    pub fn answered(&mut self, request: RequestId, result: Result<LookupPage, Error>) -> Answered {
        let Some(asked) = self.asked.as_mut() else {
            return Answered::NotMine;
        };
        let mine = asked.pinned == Some(request) || asked.matching == Some(request);
        let cancelled = mine && matches!(result, Err(Error::Cancelled));
        if cancelled && self.due.is_some() {
            // More was typed and its search is not sent yet: this one is
            // dropped, not sent again and no failure.
            if asked.pinned == Some(request) {
                asked.pinned = None;
            } else {
                asked.matching = None;
            }
            return Answered::Taken;
        }
        if cancelled && self.resends == 0 {
            // A late cancel of the statement before it landed on this
            // one. Both reads are asked again: the reducer sends them.
            self.resends = 1;
            asked.pinned = None;
            asked.matching = None;
            return Answered::Resend;
        }
        if asked.pinned == Some(request) {
            asked.pinned = None;
            // A pinned read can fail alone, and is then taken as not
            // found. One that answers `more` found several rows for one
            // key: no row either.
            let first = result
                .ok()
                .filter(|page| !page.more)
                .and_then(|page| page.rows.into_iter().next());
            if let Some(row) = &first {
                // Shown at once, first, over whatever the list held.
                self.options.retain(|option| option.key != row.key);
                self.options.insert(0, row.clone());
                self.highlight = Some(0);
                self.moved = false;
            }
            asked.first = Some(first);
            return Answered::Taken;
        }
        if asked.matching != Some(request) {
            return Answered::NotMine;
        }
        asked.matching = None;
        self.listed = Some(asked.text.clone());
        self.moved = false;
        match result {
            Ok(page) => {
                let mut options: Vec<LookupRow> =
                    asked.first.clone().flatten().into_iter().collect();
                for row in page.rows {
                    if !options.iter().any(|option| option.key == row.key) {
                        options.push(row);
                    }
                }
                self.options = options;
                self.more = page.more;
                self.failed = None;
            }
            Err(error) => {
                self.options.clear();
                self.more = false;
                self.failed = Some(error);
            }
        }
        self.highlight = self.first_highlight();
        Answered::Taken
    }

    /// Which option an answer highlights. With a text: the first, when
    /// its key is the text or its label matched; none when the options
    /// only begin with the text in their keys, so a typed `12` is not
    /// turned into row 120. With a field the user emptied: none. With the
    /// text not yet changed: the row the cell points at.
    fn first_highlight(&self) -> Option<usize> {
        if !self.searching {
            let current = self.current.as_ref()?;
            return self
                .options
                .iter()
                .position(|option| &option.key == current);
        }
        if self.listed.as_deref() == Some("") {
            return None;
        }
        match self.options.first()?.matched {
            Matched::Key | Matched::Label => Some(0),
            Matched::KeyStart => None,
        }
    }

    /// Moves the highlight `by` options, stopping at the list's ends. From
    /// none, a step down goes to the first.
    pub fn step(&mut self, by: isize) {
        self.kept = None;
        if self.options.is_empty() {
            return;
        }
        let last = self.options.len() - 1;
        let to = match self.highlight {
            Some(at) => at.saturating_add_signed(by).min(last),
            None if by > 0 => 0,
            None => return,
        };
        self.highlight = Some(to);
        self.moved = true;
    }

    /// The user chose the option at `index`, by a click. Returns whether
    /// there was such an option. A click drops a kept Enter, as a step
    /// does.
    pub fn choose(&mut self, index: usize) -> bool {
        if index >= self.options.len() {
            return false;
        }
        self.kept = None;
        self.highlight = Some(index);
        self.moved = true;
        true
    }

    /// What Enter or Tab takes while the editor holds `typed`.
    pub fn take(&self, typed: &str) -> Take {
        // An option the user moved to is the row they chose, whatever
        // list shows.
        if self.moved {
            return self.highlight.map_or(Take::Text, Take::Option);
        }
        // Opened on its value and not typed into: nothing was searched
        // for, and nothing is taken.
        if !self.searching || self.failed.is_some() {
            return Take::Text;
        }
        // An emptied field highlights nothing, whatever the read that
        // lists everything brings: Enter does not wait for it.
        if typed.trim().is_empty() {
            return Take::Text;
        }
        if self.due.is_some() {
            return Take::Wait;
        }
        let text = typed.trim();
        let Some(asked) = self.asked.as_ref().filter(|asked| asked.text == text) else {
            return Take::Wait;
        };
        // The pinned row came back found: it is the first option, and a
        // full id and Enter does not wait for the scan.
        if let Some(Some(_)) = &asked.first {
            return Take::Option(0);
        }
        if asked.matching.is_some() || self.listed.as_deref() != Some(text) {
            return Take::Wait;
        }
        self.highlight.map_or(Take::Text, Take::Option)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeSet;

    use crate::edit::{Doubt, DoubtState, Edits, NEW_ROWS, Pending, State, Table};
    use tabletist_db::{Access, ColumnMeta, ForeignKeyInfo, ObjectKind, RowPage, ValueKind};

    const HARBOR_PRESS: i64 = 9_100_000_000_000_000_004;
    const HARBOR_LIGHTS: i64 = 9_100_000_000_000_000_007;

    fn row(key: i64, label: &str, matched: Matched) -> LookupRow {
        LookupRow {
            key: Value::Int(key),
            label: Some(Value::Text(label.into())),
            matched,
        }
    }

    fn page(rows: Vec<LookupRow>) -> Result<LookupPage, Error> {
        Ok(LookupPage { rows, more: false })
    }

    fn picker(searching: bool, text: &str) -> Picker {
        Picker::new(
            (ObjectRef::new("main", "publishers"), "id".into()),
            Columns::Known(Some("name".into())),
            None,
            searching,
            text,
        )
    }

    /// A picker that asked for `text`, with a pinned read or without one.
    fn asking(text: &str, pinned: bool) -> Picker {
        let mut picker = picker(true, text);
        picker.asked = Some(Asked {
            text: text.to_owned(),
            pinned: pinned.then_some(RequestId(1)),
            matching: Some(RequestId(2)),
            first: None,
        });
        picker
    }

    fn keys(picker: &Picker) -> Vec<Value> {
        picker
            .options
            .iter()
            .map(|option| option.key.clone())
            .collect()
    }

    #[test]
    fn a_one_column_key_has_a_target_and_a_composite_one_has_none() {
        let key = |columns: &[&str], referenced: &[&str]| tabletist_db::ForeignKeyInfo {
            columns: columns.iter().map(|name| (*name).to_owned()).collect(),
            ref_schema: "main".into(),
            ref_table: "publishers".into(),
            ref_columns: referenced.iter().map(|name| (*name).to_owned()).collect(),
            ..tabletist_db::ForeignKeyInfo::default()
        };
        let structure = Structure {
            foreign_keys: vec![
                key(&["a", "b"], &["a", "b"]),
                key(&["publisher_id"], &["id"]),
                key(&["unnamed_id"], &[]),
            ],
            ..Structure::default()
        };
        assert_eq!(
            target(&structure, "publisher_id"),
            Some((ObjectRef::new("main", "publishers"), "id".to_owned()))
        );
        assert_eq!(target(&structure, "a"), None);
        assert_eq!(target(&structure, "unnamed_id"), None);
        assert_eq!(target(&structure, "kind"), None);
    }

    #[test]
    fn the_label_is_the_first_text_column_that_is_not_the_key() {
        let column = |name: &str, type_name: &str| ColumnInfo {
            name: name.into(),
            type_name: type_name.into(),
            ..ColumnInfo::default()
        };
        // As the structure names PostgreSQL's types, which is how
        // `column_class` reads them.
        let columns = [
            column("code", "text"),
            column("founded", "integer"),
            column("name", "character varying(64)"),
            column("email", "text"),
        ];
        assert_eq!(
            label_column(Dialect::Postgres, &columns, "code"),
            Some("name".to_owned())
        );
        assert_eq!(
            label_column(Dialect::Postgres, &columns, "founded"),
            Some("code".to_owned())
        );
        assert_eq!(label_column(Dialect::Postgres, &columns[1..2], "id"), None);
    }

    #[test]
    fn a_text_is_pinned_only_where_it_reads_as_the_keys_type() {
        let whole = ColumnClass::Integer {
            min: i128::from(i64::MIN),
            max: i128::from(i64::MAX),
        };
        assert_eq!(
            pinned("9100000000000000004", whole, "int8", true),
            Some(Value::Int(HARBOR_PRESS))
        );
        // What the column's own check refuses is never sent with `=`.
        assert_eq!(pinned("harb", whole, "int8", false), None);
        assert_eq!(pinned("", whole, "int8", true), None);
        // A keyword passes the column's check and is no key.
        assert_eq!(pinned("NULL", whole, "int8", true), None);
        // Too wide for an `i64`: not pinned, the matches' read finds it.
        assert_eq!(
            pinned("18446744073709551615", whole, "bigint unsigned", true),
            None
        );
        assert_eq!(pinned("-5", whole, "int8", true), Some(Value::Int(-5)));
        // `i64::from_str` takes a leading plus.
        assert_eq!(pinned("+5", whole, "int8", true), Some(Value::Int(5)));
        let decimal = ColumnClass::Decimal {
            precision: None,
            scale: None,
        };
        assert_eq!(pinned("1.5", decimal, "numeric", true), None);
        assert_eq!(pinned("1.5", ColumnClass::Float, "float8", true), None);
        let text = ColumnClass::Text { max_chars: None };
        assert_eq!(
            pinned("DE", text, "text", true),
            Some(Value::Text("DE".into()))
        );
        let uuid = "123e4567-e89b-12d3-a456-426614174000";
        assert_eq!(
            pinned(uuid, ColumnClass::Other, "uuid", true),
            Some(Value::Text(uuid.into()))
        );
        assert_eq!(
            pinned(
                "123e4567e89b12d3a456426614174000",
                ColumnClass::Other,
                "uuid",
                true
            ),
            Some(Value::Text("123e4567e89b12d3a456426614174000".into()))
        );
        // 36 characters, a hyphen in the wrong place.
        assert_eq!(
            pinned(
                "123e4567e-89b-12d3-a456-426614174000",
                ColumnClass::Other,
                "uuid",
                true
            ),
            None
        );
        assert_eq!(pinned("harb", ColumnClass::Other, "uuid", true), None);
        assert_eq!(pinned("1", ColumnClass::Other, "ltree", true), None);
    }

    #[test]
    fn the_match_is_found_whatever_its_case_and_on_whole_characters() {
        assert_eq!(found_in("Harbor Press", "harb"), Some(0..4));
        assert_eq!(found_in("New Harbor", "HARBOR"), Some(4..10));
        assert_eq!(found_in("Zoë's", "ë"), Some(2..4));
        assert_eq!(found_in("Harbor", "x"), None);
        assert_eq!(found_in("Harbor", ""), None);
    }

    #[test]
    fn an_answer_highlights_a_label_or_a_key_and_not_the_start_of_a_key() {
        let mut picker = asking("harb", false);
        assert_eq!(
            picker.answered(
                RequestId(2),
                page(vec![
                    row(HARBOR_PRESS, "Harbor Press", Matched::Label),
                    row(HARBOR_LIGHTS, "Harbor Lights", Matched::Label),
                ])
            ),
            Answered::Taken
        );
        assert_eq!(picker.highlight, Some(0));
        assert_eq!(picker.take("harb"), Take::Option(0));

        let mut picker = asking("12", false);
        picker.answered(
            RequestId(2),
            page(vec![row(120, "Aardvark", Matched::KeyStart)]),
        );
        assert_eq!(picker.highlight, None);
        // A typed 12 is not turned into row 120.
        assert_eq!(picker.take("12"), Take::Text);
    }

    #[test]
    fn the_pinned_row_is_first_and_listed_once() {
        let mut picker = asking("5", true);
        picker.answered(RequestId(1), page(vec![row(5, "Studio 91", Matched::Key)]));
        // Found: Enter does not wait for the matches.
        assert_eq!(picker.take("5"), Take::Option(0));
        picker.answered(
            RequestId(2),
            page(vec![
                row(5, "Studio 91", Matched::Key),
                row(50, "Fifty", Matched::KeyStart),
            ]),
        );
        assert_eq!(keys(&picker), [Value::Int(5), Value::Int(50)]);
        assert_eq!(picker.highlight, Some(0));
    }

    #[test]
    fn enter_waits_for_the_answer_of_the_text_that_shows() {
        let mut picker = asking("har", false);
        picker.answered(RequestId(2), page(vec![row(1, "Harrow", Matched::Label)]));
        assert_eq!(picker.take("har"), Take::Option(0));
        // One more letter: the list is for another text now.
        let now = Instant::now();
        picker.typed("harb", now, DELAY);
        assert_eq!(picker.due, Some(now + DELAY));
        assert_eq!(picker.take("harb"), Take::Wait);
        // Sent, not yet answered.
        picker.due = None;
        picker.asked = Some(Asked {
            text: "harb".into(),
            pinned: None,
            matching: Some(RequestId(4)),
            first: None,
        });
        assert_eq!(picker.take("harb"), Take::Wait);
        // An answer for the old text is not this picker's any more.
        assert_eq!(
            picker.answered(RequestId(2), page(Vec::new())),
            Answered::NotMine
        );
        picker.answered(
            RequestId(4),
            page(vec![row(HARBOR_PRESS, "Harbor Press", Matched::Label)]),
        );
        assert_eq!(picker.take("harb"), Take::Option(0));
    }

    #[test]
    fn a_text_that_did_not_change_starts_no_search() {
        let mut picker = picker(false, "9100000000000000001");
        picker.typed("9100000000000000001", Instant::now(), DELAY);
        assert!(!picker.searching);
        assert_eq!(picker.due, None);
        assert_eq!(picker.text("9100000000000000001"), "");
        // Not typed into: Enter takes nothing, whatever is listed.
        assert_eq!(picker.take("9100000000000000001"), Take::Text);
    }

    #[test]
    fn with_no_text_the_current_row_is_highlighted_and_an_emptied_field_has_none() {
        let mut opened = picker(false, "9100000000000000004");
        opened.current = Some(Value::Int(HARBOR_PRESS));
        opened.asked = Some(Asked {
            text: String::new(),
            pinned: Some(RequestId(1)),
            matching: Some(RequestId(2)),
            first: None,
        });
        opened.answered(
            RequestId(1),
            page(vec![row(HARBOR_PRESS, "Harbor Press", Matched::Key)]),
        );
        opened.answered(
            RequestId(2),
            page(vec![
                row(5, "Studio 91", Matched::KeyStart),
                row(HARBOR_PRESS, "Harbor Press", Matched::KeyStart),
            ]),
        );
        assert_eq!(keys(&opened), [Value::Int(HARBOR_PRESS), Value::Int(5)]);
        assert_eq!(opened.highlight, Some(0));
        assert!(opened.is_current(&Value::Int(HARBOR_PRESS)));

        let mut emptied = asking("", false);
        emptied.answered(
            RequestId(2),
            page(vec![row(5, "Studio 91", Matched::KeyStart)]),
        );
        assert_eq!(emptied.highlight, None);
        assert_eq!(emptied.take(""), Take::Text);
    }

    #[test]
    fn the_arrows_move_the_highlight_and_stop_at_the_ends() {
        let mut picker = asking("9", false);
        picker.answered(
            RequestId(2),
            page(vec![
                row(90, "a", Matched::KeyStart),
                row(91, "b", Matched::KeyStart),
                row(92, "c", Matched::KeyStart),
            ]),
        );
        assert_eq!(picker.highlight, None);
        picker.step(-1);
        assert_eq!(picker.highlight, None);
        picker.step(1);
        assert_eq!(picker.highlight, Some(0));
        picker.step(8);
        assert_eq!(picker.highlight, Some(2));
        picker.step(-1);
        assert_eq!(picker.highlight, Some(1));
        // The user's choice is taken at once, from whatever list shows.
        picker.due = Some(Instant::now());
        assert_eq!(picker.take("9x"), Take::Option(1));
    }

    #[test]
    fn a_search_that_fails_takes_the_text() {
        let mut picker = asking("harb", false);
        picker.kept = Some(Advance::Down);
        picker.answered(
            RequestId(2),
            Err(Error::query("permission denied for table publishers")),
        );
        assert!(picker.failed.is_some());
        assert!(picker.options.is_empty());
        // Applying a kept Enter is the reducer's part.
        assert_eq!(picker.kept, Some(Advance::Down));
        assert_eq!(picker.take("harb"), Take::Text);
    }

    #[test]
    fn a_search_is_two_reads_when_a_row_is_pinned() {
        let picker = picker(true, "5");
        let (pinned, matching) = picker.lookups(Some("name".into()), "5", Some(Value::Int(5)));
        let pinned = pinned.unwrap();
        assert!(pinned.part == LookupPart::Pinned(Value::Int(5)));
        assert_eq!(pinned.limit, 1);
        assert!(matching.part == LookupPart::Matching("5".into()));
        assert_eq!(matching.limit, LIMIT);
        let (pinned, _) = picker.lookups(None, "harb", None);
        assert!(pinned.is_none());
    }

    #[test]
    fn a_read_cancelled_unasked_is_sent_again_once() {
        let mut picker = asking("harb", true);
        assert_eq!(
            picker.answered(RequestId(2), Err(Error::Cancelled)),
            Answered::Resend
        );
        assert!(picker.in_flight().is_empty());
        assert!(picker.failed.is_none());
        // Sent again, and cancelled again: that is the list's failure.
        picker.asked = Some(Asked {
            text: "harb".into(),
            pinned: None,
            matching: Some(RequestId(5)),
            first: None,
        });
        assert_eq!(
            picker.answered(RequestId(5), Err(Error::Cancelled)),
            Answered::Taken
        );
        assert!(picker.failed.is_some());
        // More typing starts the count again.
        picker.typed("harbo", Instant::now(), DELAY);
        assert_eq!(picker.resends, 0);
    }

    #[test]
    fn a_pinned_read_that_found_several_rows_is_no_row() {
        let mut picker = asking("5", true);
        let several = Ok(LookupPage {
            rows: vec![row(5, "a", Matched::Key), row(6, "b", Matched::Key)],
            more: true,
        });
        assert_eq!(picker.answered(RequestId(1), several), Answered::Taken);
        assert_eq!(picker.asked.as_ref().unwrap().first, Some(None));
        assert!(picker.options.is_empty());
        // The matches have not answered: Enter waits for them.
        assert_eq!(picker.take("5"), Take::Wait);
    }

    #[test]
    fn a_cancelled_answer_while_a_new_search_is_due_is_dropped() {
        let mut picker = asking("harb", true);
        picker.answered(RequestId(1), page(vec![row(5, "Harbor", Matched::Key)]));
        picker.typed("harbo", Instant::now(), DELAY);
        let before = keys(&picker);
        assert_eq!(
            picker.answered(RequestId(2), Err(Error::Cancelled)),
            Answered::Taken
        );
        // Neither sent again nor a failure, and the retry is not spent.
        assert_eq!(picker.resends, 0);
        assert!(picker.failed.is_none());
        assert_eq!(keys(&picker), before);
        assert!(picker.asked.as_ref().unwrap().matching.is_none());
        // The same for the pinned read's slot.
        let mut picker = asking("5", true);
        picker.typed("55", Instant::now(), DELAY);
        assert_eq!(
            picker.answered(RequestId(1), Err(Error::Cancelled)),
            Answered::Taken
        );
        assert_eq!(picker.resends, 0);
        assert!(picker.failed.is_none());
        assert!(picker.asked.as_ref().unwrap().pinned.is_none());
        assert!(picker.asked.as_ref().unwrap().matching.is_some());
    }

    #[test]
    fn typing_after_a_failure_searches_again_and_enter_waits() {
        let mut picker = asking("harb", false);
        picker.answered(RequestId(2), Err(Error::query("permission denied")));
        assert!(picker.failed.is_some());
        assert!(picker.listed.is_some());
        picker.typed("harbo", Instant::now(), DELAY);
        assert!(picker.failed.is_none());
        assert_eq!(picker.listed, None);
        assert_eq!(picker.take("harbo"), Take::Wait);
    }

    #[test]
    fn typing_drops_a_kept_key_a_move_and_a_refusal() {
        let mut picker = asking("har", false);
        picker.kept = Some(Advance::Down);
        picker.moved = true;
        picker.refused = true;
        picker.typed("harb", Instant::now(), DELAY);
        assert_eq!(picker.kept, None);
        assert!(!picker.moved);
        assert!(!picker.refused);
    }

    #[test]
    fn a_click_chooses_an_option_and_drops_a_kept_key() {
        let mut picker = asking("9", false);
        picker.answered(
            RequestId(2),
            page(vec![
                row(90, "a", Matched::KeyStart),
                row(91, "b", Matched::KeyStart),
            ]),
        );
        picker.kept = Some(Advance::Down);
        assert!(picker.choose(1));
        assert_eq!(picker.highlight, Some(1));
        assert!(picker.moved);
        assert_eq!(picker.kept, None);
        // Past the end: nothing changes.
        picker.kept = Some(Advance::Down);
        assert!(!picker.choose(2));
        assert_eq!(picker.highlight, Some(1));
        assert_eq!(picker.kept, Some(Advance::Down));
    }

    #[test]
    fn a_pinned_read_that_finds_no_row_leaves_the_list_to_the_matches() {
        let mut picker = asking("harb", true);
        picker.answered(RequestId(1), page(Vec::new()));
        assert_eq!(picker.asked.as_ref().unwrap().first, Some(None));
        assert!(picker.options.is_empty());
        picker.answered(
            RequestId(2),
            page(vec![row(HARBOR_PRESS, "Harbor Press", Matched::Label)]),
        );
        assert_eq!(keys(&picker), [Value::Int(HARBOR_PRESS)]);
        assert_eq!(picker.highlight, Some(0));
    }

    #[test]
    fn a_pinned_read_that_fails_alone_is_not_found_and_shows_no_failure() {
        let mut picker = asking("harb", true);
        assert_eq!(
            picker.answered(RequestId(1), Err(Error::query("invalid input syntax"))),
            Answered::Taken
        );
        assert_eq!(picker.asked.as_ref().unwrap().first, Some(None));
        assert!(picker.failed.is_none());
        picker.answered(
            RequestId(2),
            page(vec![row(HARBOR_PRESS, "Harbor Press", Matched::Label)]),
        );
        assert!(picker.failed.is_none());
        assert_eq!(keys(&picker), [Value::Int(HARBOR_PRESS)]);
    }

    #[test]
    fn matches_that_fail_after_the_pinned_row_was_found_leave_no_options() {
        let mut picker = asking("5", true);
        picker.answered(RequestId(1), page(vec![row(5, "Studio 91", Matched::Key)]));
        assert_eq!(keys(&picker), [Value::Int(5)]);
        picker.answered(RequestId(2), Err(Error::query("permission denied")));
        assert!(picker.failed.is_some());
        assert!(picker.options.is_empty());
    }

    #[test]
    fn a_resend_after_the_pinned_row_was_found_clears_both_slots() {
        let mut picker = asking("5", true);
        picker.answered(RequestId(1), page(vec![row(5, "Studio 91", Matched::Key)]));
        assert_eq!(
            picker.answered(RequestId(2), Err(Error::Cancelled)),
            Answered::Resend
        );
        let asked = picker.asked.as_ref().unwrap();
        assert!(asked.pinned.is_none() && asked.matching.is_none());
        // The other read's late answer is no one's now.
        assert_eq!(
            picker.answered(RequestId(1), page(Vec::new())),
            Answered::NotMine
        );
        assert_eq!(
            picker.answered(RequestId(2), page(Vec::new())),
            Answered::NotMine
        );
    }

    #[test]
    fn enter_waits_while_nothing_was_asked() {
        let picker = picker(true, "har");
        assert!(picker.asked.is_none());
        assert_eq!(picker.due, None);
        assert_eq!(picker.take("har"), Take::Wait);
    }

    #[test]
    fn a_needle_longer_than_the_label_is_not_found_and_ranges_fall_on_characters() {
        assert_eq!(found_in("ab", "abc"), None);
        // `İ` lowers to `i` and a combining dot: two characters for one.
        let label = "İstanbul";
        // The match is the whole `İ` (two bytes) and the `s`.
        let range = found_in(label, "i\u{307}s").unwrap();
        assert_eq!(range, 0..3);
        assert_eq!(&label[range], "İs");
    }

    #[test]
    fn an_emptied_field_takes_the_text_without_waiting_for_the_list() {
        // Searching, the read that lists everything still in flight.
        let picker = asking("", false);
        assert_eq!(picker.in_flight(), [RequestId(2)]);
        assert_eq!(picker.take(""), Take::Text);
        assert_eq!(picker.take("  "), Take::Text);
        // A highlight the user moved to is still taken.
        let mut moved = asking("", false);
        moved.options = vec![row(5, "Studio 91", Matched::KeyStart)];
        moved.step(1);
        assert_eq!(moved.take(""), Take::Option(0));
    }

    #[test]
    fn a_key_put_in_the_editor_is_the_text_last_seen() {
        let mut picker = asking("harbo", false);
        picker.put("9100000000000000004");
        // The `EditorTyped` that follows starts no search.
        picker.typed("9100000000000000004", Instant::now(), DELAY);
        assert_eq!(picker.due, None);
    }

    #[test]
    fn the_last_search_is_for_the_text_that_shows_or_it_is_not() {
        let mut picker = picker(true, "harb");
        // Nothing was sent yet.
        assert!(!picker.asked_for("harb"));
        picker.asked = Some(Asked {
            text: "harb".into(),
            pinned: None,
            matching: Some(RequestId(2)),
            first: None,
        });
        assert!(picker.asked_for("harb"));
        // The spaces at its ends are not part of a search.
        assert!(picker.asked_for(" harb "));
        assert!(!picker.asked_for("harbo"));
        // Not typed into: the search is for nothing.
        let mut opened = self::picker(false, "9");
        opened.asked = Some(Asked {
            text: String::new(),
            pinned: None,
            matching: Some(RequestId(2)),
            first: None,
        });
        assert!(opened.asked_for("9"));
    }

    #[test]
    fn typing_keeps_the_failure_of_the_columns_and_enter_takes_the_text() {
        let mut picker = picker(false, "harb");
        picker.label = Columns::Waiting;
        picker.failed = Some(Error::query("permission denied for table publishers"));
        picker.typed("harbo", Instant::now(), DELAY);
        assert!(picker.failed.is_some());
        assert_eq!(picker.take("harbo"), Take::Text);
    }

    /// `genres (id, parent_id → genres.id, name)`, two rows loaded.
    fn genres() -> (Structure, RowPage) {
        let column = |name: &str| ColumnInfo {
            name: name.into(),
            type_name: if name == "name" { "TEXT" } else { "INTEGER" }.into(),
            nullable: name == "parent_id",
            ..ColumnInfo::default()
        };
        let meta = |name: &str, kind| ColumnMeta {
            name: name.into(),
            type_name: if name == "name" { "TEXT" } else { "INTEGER" }.into(),
            kind,
        };
        let structure = Structure {
            columns: vec![column("id"), column("parent_id"), column("name")],
            primary_key: vec!["id".into()],
            foreign_keys: vec![ForeignKeyInfo {
                columns: vec!["parent_id".into()],
                ref_schema: "main".into(),
                ref_table: "genres".into(),
                ref_columns: vec!["id".into()],
                ..ForeignKeyInfo::default()
            }],
            ..Structure::default()
        };
        let page = RowPage {
            columns: vec![
                meta("id", ValueKind::Numeric),
                meta("parent_id", ValueKind::Numeric),
                meta("name", ValueKind::Text),
            ],
            rows: vec![
                vec![Value::Int(1), Value::Null, Value::Text("Fiction".into())],
                vec![Value::Int(2), Value::Int(1), Value::Text("Crime".into())],
            ],
            has_more: false,
            ordered_by_key: true,
            elapsed: Duration::ZERO,
        };
        (structure, page)
    }

    fn ready(text: &str) -> Pending {
        Pending {
            new: NewValue::Text(text.into()),
            state: State::Ready,
        }
    }

    fn missing(text: &str) -> Doubt {
        Doubt {
            text: text.into(),
            state: DoubtState::Missing,
            again: false,
        }
    }

    #[test]
    fn a_warning_stands_only_for_the_text_the_cell_still_holds() {
        let (structure, page) = genres();
        let gone = BTreeSet::new();
        let own = ObjectRef::new("main", "genres");
        let mut edits = Edits::default();
        let table = |edits: &Edits| {
            let found = unmatched(
                &Table {
                    access: Access::Writable,
                    kind: ObjectKind::Table,
                    dialect: Dialect::Sqlite,
                    structure: Some(&structure),
                    page: &page,
                    refreshing: false,
                    saving: false,
                    gone: &gone,
                    added: &edits.added,
                },
                &own,
                edits,
            );
            found
                .into_iter()
                .map(|found| (found.at, found.table, found.key, found.value))
                .collect::<Vec<_>>()
        };
        // Row 1's parent is 99, and no genre has that id.
        edits.cells.insert((1, 1), ready("99"));
        edits.doubts.insert((1, 1), missing("99"));
        assert_eq!(
            table(&edits),
            [(
                (1, 1),
                "genres".to_owned(),
                "id".to_owned(),
                "99".to_owned()
            )]
        );
        // Asked, not answered: nothing is said yet.
        edits.doubts.insert(
            (1, 1),
            Doubt {
                text: "99".into(),
                state: DoubtState::Asked(RequestId(7)),
                again: false,
            },
        );
        assert!(table(&edits).is_empty());
        // The value changed since: the answer was of another text.
        edits.doubts.insert((1, 1), missing("99"));
        edits.cells.insert((1, 1), ready("98"));
        assert!(table(&edits).is_empty());
        // A cell to fix says its own problem.
        edits.cells.insert(
            (1, 1),
            Pending {
                new: NewValue::Text("99".into()),
                state: State::ToFix(crate::edit::Problem::WholeNumber),
            },
        );
        assert!(table(&edits).is_empty());
        // A cell whose last save failed shows the failure, not the warning.
        edits.cells.insert(
            (1, 1),
            Pending {
                new: NewValue::Text("99".into()),
                state: State::Failed(tabletist_db::Error::query("duplicate key")),
            },
        );
        assert!(table(&edits).is_empty());
        // Reverted: no pending cell, no warning.
        edits.cells.remove(&(1, 1));
        assert!(table(&edits).is_empty());
    }

    /// `taken` says the editor's text is an option's key just read: typing
    /// ends it, and a text the picker already saw (`put`) leaves it.
    #[test]
    fn typing_clears_taken_and_a_text_put_leaves_it() {
        let mut picker = picker(false, "");
        let now = Instant::now();
        assert!(!picker.taken);
        picker.put("9100000000000000004");
        picker.taken = true;
        // The `EditorTyped` that follows a pick: the text is the one the
        // picker saw.
        picker.typed("9100000000000000004", now, Duration::ZERO);
        assert!(picker.taken);
        picker.typed("910000000000000000", now, Duration::ZERO);
        assert!(!picker.taken);
    }

    #[test]
    fn a_key_that_a_new_row_of_the_same_table_holds_is_no_warning() {
        let (structure, page) = genres();
        let gone = BTreeSet::new();
        let own = ObjectRef::new("main", "genres");
        let mut edits = Edits::default();
        edits.cells.insert((1, 1), ready("99"));
        edits.doubts.insert((1, 1), missing("99"));
        // A new row whose `id` is 99: the same save writes it.
        edits.cells.insert((NEW_ROWS, 0), ready("99"));
        let table = Table {
            access: Access::Writable,
            kind: ObjectKind::Table,
            dialect: Dialect::Sqlite,
            structure: Some(&structure),
            page: &page,
            refreshing: false,
            saving: false,
            gone: &gone,
            added: &edits.added,
        };
        assert!(unmatched(&table, &own, &edits).is_empty());
        // Another table's key of the same number is no row of this one.
        let other = ObjectRef::new("main", "books");
        assert_eq!(unmatched(&table, &other, &edits).len(), 1);
    }
}
