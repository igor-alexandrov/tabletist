//! Editing a table's values: what can be edited, what a column takes, the
//! pending set a tab holds and the change set a save sends. Everything here
//! is decided from the page and the structure alone; the reducer
//! (`app.rs`, `app/editing.rs`) owns every transition.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use tabletist_db::{
    Access, CellChange, ChangeSet, ColumnClass, ColumnInfo, Conflict, Dialect, Error, NewValue,
    ObjectKind, ObjectRef, RowChange, RowPage, Structure, Value, column_class,
};

use crate::backend::RequestId;
use crate::model::{CellPos, ObjectTab, Workspace};

/// The largest value an editor opens, and the largest it keeps, in bytes
/// of its text: a field that held megabytes would be laid out every frame.
pub const MAX_EDIT_BYTES: usize = 256 * 1024;

/// Why a cell cannot be edited. The view words it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lock {
    /// The connection opens read-only.
    ReadOnly,
    /// A view, a materialized view.
    NotATable,
    /// The row key, the columns' types and what is generated are not known
    /// yet.
    StructureLoading,
    /// No primary key and no unique index that names a row.
    NoKey,
    /// The key has a column of a type a save cannot match exactly.
    KeyType,
    /// A save is running.
    Saving,
    /// The page is being fetched again, and what is on screen is about to
    /// be replaced.
    Refreshing,
    NoSuchCell,
    /// A save found the row gone from the server.
    Gone,
    /// The row's key holds a NULL.
    KeyIsNull,
    /// The row's key holds text that may not have been read exactly.
    KeyInexact,
    /// The structure does not list the column, or the page names it twice.
    UnknownColumn,
    /// Computed by the database.
    Generated,
    /// One of the columns a save finds the row by.
    KeyColumn,
    Binary,
    /// Over `MAX_EDIT_BYTES`.
    TooLarge,
}

/// A table's page as editing sees it.
#[derive(Clone, Copy)]
pub struct Table<'a> {
    pub access: Access,
    pub kind: ObjectKind,
    pub dialect: Dialect,
    pub structure: Option<&'a Structure>,
    pub page: &'a RowPage,
    /// The page is being fetched again.
    pub refreshing: bool,
    pub saving: bool,
    /// The page's rows a save found gone from the server.
    pub gone: &'a BTreeSet<usize>,
}

impl<'a> Table<'a> {
    /// What editing may know of `object`, a table's tab of `workspace`:
    /// `None` while it has no page.
    pub fn of(workspace: &'a Workspace, object: &'a ObjectTab) -> Option<Self> {
        Some(Self {
            access: workspace.access,
            kind: object.kind,
            dialect: workspace.driver.dialect(),
            structure: object.structure.value.as_ref(),
            page: object.page()?,
            // The structure too: what a describe in flight brings may have
            // another key, and no edit starts on the one about to go.
            refreshing: object.rows.is_loading() || object.structure.is_loading(),
            saving: object.edits.saving.is_some(),
            gone: &object.edits.gone,
        })
    }
}

impl Table<'_> {
    /// The page's columns that make the row key, by their place. `None`
    /// when the table has no key, or the page does not hold all of it.
    pub fn key(&self) -> Option<Vec<usize>> {
        self.structure?
            .row_key()?
            .iter()
            .map(|name| self.place(name))
            .collect()
    }

    /// The one column of the page called `name`. Two of that name are none:
    /// no statement could say which it means.
    fn place(&self, name: &str) -> Option<usize> {
        let mut places = self
            .page
            .columns
            .iter()
            .enumerate()
            .filter(|(_, column)| column.name == name);
        let (place, _) = places.next()?;
        places.next().is_none().then_some(place)
    }

    /// What the structure says about the page's column `col`.
    pub fn column(&self, col: usize) -> Option<&ColumnInfo> {
        let name = &self.page.columns.get(col)?.name;
        self.place(name)?;
        self.structure?
            .columns
            .iter()
            .find(|column| column.name == *name)
    }

    /// The column's class, by the type the structure gives it.
    pub fn class(&self, col: usize) -> Option<ColumnClass> {
        self.column(col)
            .map(|column| column_class(self.dialect, &column.type_name))
    }

    /// Why no cell of the page's row `row` can be edited, or `None` when
    /// each of its cells answers for itself: what holds for the whole
    /// table, what holds for a while, and what the row's own key says. The
    /// row panel says such a reason once, for the row.
    pub fn row_lock(&self, row: usize) -> Option<Lock> {
        if let Some(lock) = self.never() {
            return Some(lock);
        }
        let Some(key) = self.key() else {
            return Some(Lock::NoKey);
        };
        if self.saving {
            return Some(Lock::Saving);
        }
        if self.refreshing {
            return Some(Lock::Refreshing);
        }
        let Some(values) = self.page.rows.get(row) else {
            return Some(Lock::NoSuchCell);
        };
        // Before anything its values say: they are of a row that is no
        // longer there.
        if self.gone.contains(&row) {
            return Some(Lock::Gone);
        }
        // A row narrower than the page has no key to read.
        let held = |col: &usize| values.get(*col);
        if key.iter().any(|col| held(col).is_none_or(Value::is_null)) {
            return Some(Lock::KeyIsNull);
        }
        let inexact = match self.dialect {
            Dialect::Sqlite => key.iter().any(
                |col| matches!(held(col), Some(Value::Text(text)) if text.contains('\u{FFFD}')),
            ),
            Dialect::Postgres | Dialect::MySql => false,
        };
        if inexact {
            return Some(Lock::KeyInexact);
        }
        None
    }

    /// Why `cell` cannot be edited, or `None` when it can. The reasons that
    /// hold for the whole table come first, so every cell of such a table
    /// says the same, then the row's (see [`Table::row_lock`]), then the
    /// cell's own.
    pub fn lock(&self, cell: CellPos) -> Option<Lock> {
        if let Some(lock) = self.row_lock(cell.row) {
            return Some(lock);
        }
        // A row with no lock of its own is a row of a table with a key.
        self.own_lock(cell, &self.key().unwrap_or_default())
    }

    /// Why `cell` cannot be edited for a reason of its own, in a row that
    /// has no [`Table::row_lock`]. `key` is the table's ([`Table::key`]):
    /// who asks for every cell of a row finds the row's lock and the key
    /// once, and asks this for each cell.
    pub fn own_lock(&self, cell: CellPos, key: &[usize]) -> Option<Lock> {
        let row = self.page.rows.get(cell.row);
        let Some(value) = row.and_then(|row| row.get(cell.col)) else {
            return Some(Lock::NoSuchCell);
        };
        let Some(column) = self.column(cell.col) else {
            return Some(Lock::UnknownColumn);
        };
        if column.generated {
            return Some(Lock::Generated);
        }
        if key.contains(&cell.col) {
            return Some(Lock::KeyColumn);
        }
        if column_class(self.dialect, &column.type_name) == ColumnClass::Binary
            || matches!(value, Value::Bytes(_))
        {
            return Some(Lock::Binary);
        }
        if matches!(value, Value::Text(text) if text.len() > MAX_EDIT_BYTES) {
            return Some(Lock::TooLarge);
        }
        None
    }

    /// Why no cell of the table is ever edited, or `None` for a table that
    /// can be: one whose cells are locked only for a while (a save, a
    /// fetch) or each for a reason of its own.
    pub fn never(&self) -> Option<Lock> {
        if self.access == Access::ReadOnly {
            return Some(Lock::ReadOnly);
        }
        if self.kind != ObjectKind::Table {
            return Some(Lock::NotATable);
        }
        if self.structure.is_none() {
            return Some(Lock::StructureLoading);
        }
        let Some(key) = self.key() else {
            return Some(Lock::NoKey);
        };
        if key.iter().any(|&col| self.unmatched(col)) {
            return Some(Lock::KeyType);
        }
        None
    }

    /// Whether a save could not match a key column of this type exactly.
    /// MySQL shows a TIMESTAMP in the session's zone without it, reads a BIT
    /// bound as bytes as a number, and misses a FLOAT bound as a double; the
    /// save refuses such a key (`tabletist-db`, `mysql/write.rs`).
    fn unmatched(&self, col: usize) -> bool {
        match self.dialect {
            Dialect::MySql => self.column(col).is_some_and(|column| {
                let word: String = column
                    .type_name
                    .chars()
                    .take_while(|letter| letter.is_ascii_alphabetic())
                    .collect::<String>()
                    .to_ascii_lowercase();
                matches!(word.as_str(), "timestamp" | "bit" | "float")
            }),
            Dialect::Postgres | Dialect::Sqlite => false,
        }
    }
}

/// Why a text is not a value its column takes. Found before anything is
/// sent; every other rule is the database's, and its refusal is a failed
/// save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// Not a whole number.
    WholeNumber,
    /// A whole number the type does not hold.
    OutOfRange { min: i128, max: i128 },
    /// Not a number.
    Number,
    /// More decimals than the type keeps, or, for a scale below zero,
    /// digits where the type keeps zeros: `stored` is what the database
    /// would have rounded it to.
    Decimals { scale: i32, stored: String },
    /// More digits before the point than the type holds: at most `whole`.
    Digits { whole: u32 },
    /// A type with no fewer decimals than digits holds no whole digit, and
    /// only zeros in its first decimals: numbers nearer to zero than
    /// `limit`, written out (`0.01`).
    Under { limit: String },
    /// More digits than the database keeps of a number: `stored` is the
    /// number it would have kept instead.
    Inexact { stored: String },
    /// Not `true`, `false`, `1` or `0`, nor, where the column holds more
    /// than a flag, a number it holds.
    Boolean,
    /// Not one of the values the column allows.
    NotOneOf(Vec<String>),
    /// It does not parse: what the parser expected, and where.
    Json {
        message: String,
        line: usize,
        column: usize,
    },
    /// More than `max` characters.
    TooLong { max: u32 },
    /// Over `MAX_EDIT_BYTES`: more than an editor opens.
    TooLarge,
}

/// Whether `text` is a value `column` takes, as far as the app can tell
/// from its type's name. A type it does not know has no check but its
/// size.
pub fn check(dialect: Dialect, column: &ColumnInfo, text: &str) -> Option<Problem> {
    // Before every other rule, and whatever the column: a text an editor
    // would not open is not one to keep, and the grid and the editor would
    // lay it out and copy it every frame. An open editor takes a paste of
    // any size where the column sets no length.
    if text.len() > MAX_EDIT_BYTES {
        return Some(Problem::TooLarge);
    }
    if let Some(allowed) = &column.allowed_values {
        return (!allowed.iter().any(|value| value == text))
            .then(|| Problem::NotOneOf(allowed.clone()));
    }
    // ASCII whitespace alone: the text is sent as it was typed, and that is
    // all a database overlooks around a value.
    let typed = text.trim_ascii();
    match column_class(dialect, &column.type_name) {
        ColumnClass::Integer { min, max } => match typed.parse::<i128>() {
            Err(_) => Some(Problem::WholeNumber),
            Ok(number) if number < min || number > max => Some(Problem::OutOfRange { min, max }),
            Ok(_) => None,
        },
        ColumnClass::Decimal { precision, scale } => {
            decimal(typed, precision, scale).or_else(|| match dialect {
                Dialect::Sqlite => inexact(typed),
                Dialect::Postgres | Dialect::MySql => None,
            })
        }
        ColumnClass::Float => {
            let word = match dialect {
                Dialect::Postgres => float_word(typed),
                Dialect::MySql | Dialect::Sqlite => false,
            };
            let number = typed.parse::<f64>().is_ok_and(f64::is_finite);
            (!word && !number).then_some(Problem::Number)
        }
        ColumnClass::Boolean => boolean(dialect, typed),
        ColumnClass::Json => serde_json::from_str::<serde::de::IgnoredAny>(text)
            .err()
            .map(|error| Problem::Json {
                message: json_message(&error.to_string()),
                line: error.line(),
                column: error.column(),
            }),
        ColumnClass::Text {
            max_chars: Some(max),
        } => (text.chars().count() > max as usize).then_some(Problem::TooLong { max }),
        ColumnClass::Text { max_chars: None } | ColumnClass::Binary | ColumnClass::Other => None,
    }
}

/// The parser's words without its own "at line 3 column 23", which the
/// view writes as `3:23`.
fn json_message(error: &str) -> String {
    error
        .split(" at line ")
        .next()
        .unwrap_or(error)
        .replace('`', "")
}

/// Whether PostgreSQL reads `typed` as a float that is no number: `nan`,
/// `inf` or `infinity`, in any case. A sign on infinity only: one on NaN is
/// read by some servers' C library and not by others'.
fn float_word(typed: &str) -> bool {
    let word = typed.to_ascii_lowercase();
    let unsigned = word.strip_prefix(['-', '+']).unwrap_or(&word);
    word == "nan" || matches!(unsigned, "inf" | "infinity")
}

/// A boolean as the save's builder takes one. MySQL's is a `tinyint(1)`,
/// which holds any tinyint, and some tables keep more than a flag there.
fn boolean(dialect: Dialect, typed: &str) -> Option<Problem> {
    if matches!(typed.to_ascii_lowercase().as_str(), "true" | "false") {
        return None;
    }
    let (min, max) = (i128::from(i8::MIN), i128::from(i8::MAX));
    match dialect {
        Dialect::MySql => match typed.parse::<i128>() {
            Ok(number) if number < min || number > max => Some(Problem::OutOfRange { min, max }),
            Ok(_) => None,
            Err(_) => Some(Problem::Boolean),
        },
        Dialect::Postgres | Dialect::Sqlite => {
            (!matches!(typed, "1" | "0")).then_some(Problem::Boolean)
        }
    }
}

/// A plain decimal's sign, the digits before its point and those after.
fn parts(typed: &str) -> (bool, &str, &str) {
    let (negative, digits) = match typed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, typed.strip_prefix('+').unwrap_or(typed)),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    (negative, whole, fraction)
}

/// What SQLite would keep of the plain decimal `typed`, when that is not
/// the number as typed. It keeps a number as an INTEGER or a REAL whatever
/// digits the declared type states: a whole number an INTEGER holds is
/// kept, and any other only when the REAL nearest to it is written with
/// the same digits.
fn inexact(typed: &str) -> Option<Problem> {
    if typed.parse::<i64>().is_ok() {
        return None;
    }
    // Past what a REAL holds at all: the save's builder sends no such
    // number.
    let Some(number) = typed.parse::<f64>().ok().filter(|real| real.is_finite()) else {
        return Some(Problem::Number);
    };
    // The typed digits as a number is written: no plus, no zeros ahead of
    // it but the one before the point, none after its last decimal.
    let (negative, whole, fraction) = parts(typed);
    let (whole, fraction) = (
        whole.trim_start_matches('0'),
        fraction.trim_end_matches('0'),
    );
    let mut written = String::with_capacity(typed.len() + 1);
    if negative {
        written.push('-');
    }
    written.push_str(if whole.is_empty() { "0" } else { whole });
    if !fraction.is_empty() {
        written.push('.');
        written.push_str(fraction);
    }
    // The shortest text that reads back as the same REAL, without an
    // exponent: what a cell would show of it.
    let stored = number.to_string();
    (stored != written).then_some(Problem::Inexact { stored })
}

/// A plain decimal number its type keeps as it is typed, by the rule the
/// databases have for the digits and the scale a type states: the number
/// is rounded to the scale, and what is left is under ten to the power of
/// the digits less the scale. No exponent: the databases take one, but
/// what it would be stored as is not what the user sees typed.
fn decimal(typed: &str, precision: Option<u32>, scale: Option<i32>) -> Option<Problem> {
    let (negative, whole, fraction) = parts(typed);
    let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty()) || !all_digits(whole) || !all_digits(fraction) {
        return Some(Problem::Number);
    }
    let scale = scale?;
    // The places after the point that are kept, and the places before it
    // that a scale below zero gives up for zeros.
    let decimals = scale.max(0).unsigned_abs() as usize;
    let zeros = scale.min(0).unsigned_abs() as usize;
    let whole = whole.trim_start_matches('0');
    let given_up = &whole[whole.len().saturating_sub(zeros)..];
    let rounds = if zeros > 0 {
        !fraction.trim_end_matches('0').is_empty() || !given_up.trim_end_matches('0').is_empty()
    } else {
        fraction.trim_end_matches('0').len() > decimals
    };
    if rounds {
        return Some(Problem::Decimals {
            scale,
            stored: rounded(negative, whole, fraction, scale),
        });
    }
    // Nothing is rounded: the number as typed is what has to fit.
    let room = i64::from(precision?) - i64::from(scale);
    if room > 0 {
        let held = u32::try_from(room).unwrap_or(u32::MAX);
        return (whole.len() as u64 > u64::from(held)).then_some(Problem::Digits { whole: held });
    }
    // More decimals than digits: nothing before the point, and zeros in
    // the decimals the digits do not reach.
    let leading = usize::try_from(-room).unwrap_or(usize::MAX);
    let reached = fraction.bytes().take(leading).all(|byte| byte == b'0');
    (!whole.is_empty() || !reached).then(|| Problem::Under {
        limit: match leading {
            0 => "1".to_owned(),
            leading => format!("0.{}1", "0".repeat(leading - 1)),
        },
    })
}

/// `whole.fraction` rounded half away from zero to `scale` decimals, as the
/// databases round a decimal; below zero, to that many zeros before the
/// point. Something is rounded away: `fraction` is longer than a `scale`
/// from zero up.
fn rounded(negative: bool, whole: &str, fraction: &str, scale: i32) -> String {
    let decimals = scale.max(0).unsigned_abs() as usize;
    let zeros = scale.min(0).unsigned_abs() as usize;
    // Zeros ahead, so that a digit stands before the ones given up.
    let whole = format!("{whole:0>width$}", width = zeros + 1);
    let kept = whole.len() - zeros + decimals;
    let mut places = whole
        .bytes()
        .chain(fraction.bytes())
        .map(|byte| byte - b'0');
    let mut digits: Vec<u8> = places.by_ref().take(kept).collect();
    // The first digit left out decides: half and over goes away from zero.
    if places.next().is_some_and(|digit| digit >= 5) {
        let mut place = digits.len();
        loop {
            if place == 0 {
                digits.insert(0, 1);
                break;
            }
            place -= 1;
            if digits[place] == 9 {
                digits[place] = 0;
            } else {
                digits[place] += 1;
                break;
            }
        }
    }
    // As a number is written: no zeros ahead of it but the one before the
    // point, and no sign on zero.
    while digits.len() > decimals + 1 && digits[0] == 0 {
        digits.remove(0);
    }
    let nothing = digits.iter().all(|&digit| digit == 0);
    let point = digits.len().saturating_sub(decimals);
    let mut text = String::with_capacity(digits.len() + zeros + 2);
    if negative && !nothing {
        text.push('-');
    }
    for (place, digit) in digits.iter().enumerate() {
        if place == point && decimals > 0 {
            text.push('.');
        }
        text.push(char::from(b'0' + digit));
    }
    if !nothing {
        text.extend(std::iter::repeat_n('0', zeros));
    }
    text
}

/// Where an editor starts: the whole value as the database gave it, never
/// the shortened text a cell shows. Empty on NULL. A boolean column reads
/// `true` or `false` whatever the driver loaded (SQLite and MySQL hold 1
/// and 0). A document is laid out a member to a line, as its tree reads:
/// a server gives it back on one.
pub fn start_text(value: &Value, class: ColumnClass) -> String {
    match (value, class) {
        (Value::Null, _) => String::new(),
        (Value::Int(1), ColumnClass::Boolean) => "true".to_owned(),
        (Value::Int(0), ColumnClass::Boolean) => "false".to_owned(),
        (value, ColumnClass::Json) => laid_out(crate::ui::format::plain_text(value)),
        (value, _) => crate::ui::format::plain_text(value),
    }
}

/// A JSON column's text laid out to be read and edited. Only the white
/// space between its pieces changes (`ui::json_text::pretty` reads no
/// value), and only where the text is a document: what such a column holds
/// that is none (SQLite keeps any text) stays as it is.
fn laid_out(text: String) -> String {
    if serde_json::from_str::<serde::de::IgnoredAny>(&text).is_err() {
        return text;
    }
    crate::ui::json_text::pretty(&text)
}

/// Whether `new` differs from what the cell loaded: a text equal to where
/// the editor starts, or NULL on a NULL cell, is no change. Nor is a
/// document that differs from the loaded one only in the white space
/// between its pieces: laid out or set back on one line, it is the
/// document that was loaded.
pub fn is_change(loaded: &Value, new: &NewValue, class: ColumnClass) -> bool {
    match new {
        NewValue::Null => !loaded.is_null(),
        NewValue::Text(_) if loaded.is_null() => true,
        NewValue::Text(text) if class == ColumnClass::Json => {
            laid_out(text.clone()) != start_text(loaded, class)
        }
        NewValue::Text(text) => *text != start_text(loaded, class),
    }
}

/// Whether a value is edited in the large editor rather than on its cell:
/// every JSON column, and text with a line break or longer than the grid's
/// cut.
pub fn opens_large(text: &str, class: ColumnClass) -> bool {
    class == ColumnClass::Json
        || text.contains('\n')
        || text.chars().count() > crate::ui::format::CELL_MAX_CHARS
}

/// One cell's new value, not yet written.
#[derive(Clone, PartialEq)]
pub struct Pending {
    pub new: NewValue,
    pub state: State,
}

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    /// Waits for a save.
    Ready,
    /// Fails its check: a save is not offered until it is fixed.
    ToFix(Problem),
    /// The last save's statement for its row failed. It is sent again by
    /// the next save.
    Failed(Error),
}

/// Where a tab's editor is drawn: on its cell in the grid (or in the
/// popover at the cell), or in the row panel, in the place of the field's
/// value. Its text becomes the same pending cell from either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorPlace {
    #[default]
    Grid,
    Panel,
}

/// The editor that is open. Only `text` is the view's to change.
pub struct Editor {
    pub cell: CellPos,
    /// The view that draws it.
    pub place: EditorPlace,
    pub text: String,
    /// The popover rather than the field on the cell.
    pub large: bool,
    /// Taken by the view when it gives the field the keyboard.
    pub focus: bool,
    /// The cursor starts at the text's start, not its end: a value of
    /// several lines opened to be read and changed, which is read from
    /// its top. One that was just typed into goes on from its end.
    pub top: bool,
    /// Whether the text was typed into. An editor that was only opened and
    /// closed changes nothing: on a NULL cell it starts empty, and the
    /// empty string is not NULL.
    pub touched: bool,
    /// What the text fails, kept up to date by `Action::EditorTyped`.
    pub problem: Option<Problem>,
}

/// What a table's tab holds while its values are edited.
#[derive(Default)]
pub struct Edits {
    /// By row and column of the loaded page.
    pub cells: BTreeMap<(usize, usize), Pending>,
    pub editor: Option<Editor>,
    /// Why the cell last asked for could not be edited.
    pub why: Option<(CellPos, Lock)>,
    /// Where that edit was asked for: the reason is said there, at the
    /// cell or under the row panel's field.
    pub why_place: EditorPlace,
    /// The save that is running.
    pub saving: Option<Saving>,
    /// The last save that wrote, for the cells' green and the status.
    pub saved: Option<Saved>,
    /// What the last save came to when it wrote nothing.
    pub note: Option<Note>,
    /// The page's rows a save found gone from the server. They are no
    /// rows to edit until the page is loaded again, and nothing of the
    /// user's: they do not hold the page.
    pub gone: BTreeSet<usize>,
    /// Review SQL is open: the drawer on macOS and Windows, the terminal
    /// look's `:diff` panel.
    pub reviewing: bool,
    /// What a save would run, as lines to read: made by the reducer while
    /// `reviewing`. `None` once the set changed, until it is made again.
    pub review: Option<crate::review::Review>,
}

/// How much is pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub changes: usize,
    pub rows: usize,
    pub to_fix: usize,
    pub failed: usize,
}

/// What a row's cells come to, for its mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowMark {
    #[default]
    None,
    Changed,
    /// One of its cells is to fix or failed.
    Trouble,
}

impl Edits {
    /// Whether the tab's page must stay: something is pending, an editor
    /// is open, or a save is running.
    pub fn holds(&self) -> bool {
        !self.cells.is_empty() || self.editor.is_some() || self.saving.is_some()
    }

    /// Drops what is pending, the open editor and what the last save left.
    /// What is known of the page itself stays: the rows that are gone.
    pub fn discard(&mut self) {
        *self = Self {
            gone: std::mem::take(&mut self.gone),
            ..Self::default()
        };
    }

    pub fn counts(&self) -> Counts {
        let mut counts = Counts {
            changes: self.cells.len(),
            ..Counts::default()
        };
        let mut last = None;
        for (&(row, _), cell) in &self.cells {
            if last != Some(row) {
                counts.rows += 1;
                last = Some(row);
            }
            match cell.state {
                State::Ready => {}
                State::ToFix(_) => counts.to_fix += 1,
                State::Failed(_) => counts.failed += 1,
            }
        }
        counts
    }

    /// The statement of the page's row `row` failed with `error`: its
    /// cells say so until the next save sends them again. With no such row
    /// (the answer names one the save did not send) no cell is marked, and
    /// the save is refused with the error.
    pub fn fail(&mut self, row: Option<usize>, error: Error) {
        let Some(row) = row else {
            self.note = Some(Note::Refused(error));
            return;
        };
        for (_, cell) in self.cells.range_mut((row, 0)..=(row, usize::MAX)) {
            cell.state = State::Failed(error.clone());
        }
        self.note = Some(Note::Failed { row, error });
    }

    pub fn row_mark(&self, row: usize) -> RowMark {
        row_mark(&self.cells, row)
    }

    /// Makes `pending` the new value of the cell at `at` (row, column).
    /// What was made of the set, its review, is stale.
    pub fn put(&mut self, at: (usize, usize), pending: Pending) {
        self.cells.insert(at, pending);
        self.review = None;
    }

    /// Takes the cell at `at` out of the set: it is as it loaded again.
    pub fn revert(&mut self, at: (usize, usize)) {
        self.cells.remove(&at);
        self.review = None;
    }
}

/// What the pending `cells` of the page's row `row` come to. A view that
/// holds the open editor's text reads the cells beside it, not through the
/// whole set.
pub fn row_mark(cells: &BTreeMap<(usize, usize), Pending>, row: usize) -> RowMark {
    let mut cells = cells.range((row, 0)..=(row, usize::MAX)).peekable();
    if cells.peek().is_none() {
        RowMark::None
    } else if cells.any(|(_, cell)| cell.state != State::Ready) {
        RowMark::Trouble
    } else {
        RowMark::Changed
    }
}

/// Without the texts: what a user typed stays out of logs and panics.
impl std::fmt::Debug for Edits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Edits {{ cells: {}, editor: {:?}, why: {:?}, saving: {}, gone: {:?} }}",
            self.cells.len(),
            self.editor.as_ref().map(|editor| editor.cell),
            self.why,
            self.saving.is_some(),
            self.gone
        )
    }
}

/// A save in flight.
#[derive(Debug)]
pub struct Saving {
    pub request: RequestId,
    /// The page's row of each row of the change set, in its order: the
    /// answer names rows by their place in the set.
    pub rows: Vec<usize>,
    pub started: Instant,
    /// What to do once everything is written.
    pub then: Option<crate::model::Held>,
}

/// A save that wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    /// When, for the cells that show it for a moment.
    pub at: Instant,
    /// In the order of the set: by row, then by column. The grid finds a
    /// cell in it by that order.
    pub cells: Vec<CellPos>,
    pub changes: usize,
    pub rows: usize,
    pub elapsed: Duration,
}

/// How long a saved cell shows it.
pub const SAVED_FOR: Duration = Duration::from_millis(1200);

/// How long a question that came up unasked has been on screen before it
/// takes an answer: the question about a row a save found changed, which
/// comes when the database answers, and the production confirmation where
/// another dialog's answer opened it. What was on its way elsewhere then
/// (a key, a click) is no answer to it, and the second click of a double
/// click is none to the question that took the first one's place.
pub const ANSWER_AFTER: Duration = Duration::from_millis(500);

/// Whether a question that came up at `shown` takes answers yet. An
/// instant still to come has lasted no time.
pub fn answers_taken(shown: Instant) -> bool {
    Instant::now().saturating_duration_since(shown) >= ANSWER_AFTER
}

/// What a save came to when it wrote nothing. The view words it.
#[derive(Debug, Clone, PartialEq)]
pub enum Note {
    /// The page's row `row` changed on the server, or is `gone`; `others`
    /// more rows conflict too.
    Conflict {
        row: usize,
        gone: bool,
        others: usize,
    },
    /// The statement of the page's row `row` failed.
    Failed { row: usize, error: Error },
    /// What was written is not known: the connection was lost while
    /// saving, or the answer is about a row the save did not send.
    Lost,
    /// The session went before the save was sent: nothing was written.
    NotSent,
    /// The save was cancelled.
    Cancelled,
    /// The save was refused or undone, with the database's or the app's
    /// reason.
    Refused(Error),
}

/// A row a save found changed on the server, by its place in the page.
#[derive(Clone, PartialEq)]
pub struct Conflicting {
    /// The page's row.
    pub row: usize,
    /// The row as the database holds it now, as wide as the page. `None`
    /// when it is gone.
    pub server: Option<Vec<Value>>,
}

/// Without the row: it is the database's, and can be megabytes.
impl std::fmt::Debug for Conflicting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Conflicting {{ row: {}, gone: {} }}",
            self.row,
            self.server.is_none()
        )
    }
}

/// A save's conflicts as rows of `page`, in the save's order. `places` is
/// the page's row of each row the save sent (`Saving::rows`): the answer
/// names rows by their place in the set. `None` when there is nothing to
/// ask from it: it names a row the save did not send or the page does not
/// hold, the same row twice, or brings a row that is not as wide as the
/// page (the table is no longer the one the page was read from).
pub fn conflicting(
    places: &[usize],
    conflicts: Vec<Conflict>,
    page: &RowPage,
) -> Option<Vec<Conflicting>> {
    let mut seen = BTreeSet::new();
    conflicts
        .into_iter()
        .map(|conflict| {
            let row = *places.get(conflict.row)?;
            page.rows.get(row)?;
            if !seen.insert(row) {
                return None;
            }
            let fits = |server: &Vec<Value>| server.len() == page.columns.len();
            if !conflict.server.as_ref().is_none_or(fits) {
                return None;
            }
            Some(Conflicting {
                row,
                server: conflict.server,
            })
        })
        .collect()
}

/// What the user answers about a row a save found changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// The server's row becomes the loaded one and the pending cells stay
    /// on top of it. For a row that is gone: its cells stay as they are.
    KeepMine,
    /// The server's row becomes the loaded one and the row's pending cells
    /// are dropped.
    UseServer,
    /// As `KeepMine`, and the save may run again once every row is
    /// answered.
    Overwrite,
    /// For a row that is gone: its pending cells are dropped.
    Discard,
}

impl Answer {
    /// Whether the question about a row offers this answer: `gone` says
    /// the row no longer exists.
    pub fn offered(self, gone: bool) -> bool {
        match self {
            Self::KeepMine => true,
            Self::UseServer | Self::Overwrite => !gone,
            Self::Discard => gone,
        }
    }
}

/// One column the user changed in a row a save found changed, as the
/// question about the row shows it.
#[derive(Clone, Copy, PartialEq)]
pub struct ConflictLine<'a> {
    /// The page's column.
    pub col: usize,
    pub loaded: &'a Value,
    /// What the database holds now. `None` when the row is gone.
    pub server: Option<&'a Value>,
    pub yours: &'a NewValue,
    /// The server's value is another than the loaded one: this column is
    /// one the conflict is about.
    pub moved: bool,
}

/// The columns the user changed in `conflict`'s row, in the page's order:
/// what the page loaded, what the server holds now and the pending value.
pub fn conflict_lines<'a>(
    page: &'a RowPage,
    cells: &'a BTreeMap<(usize, usize), Pending>,
    conflict: &'a Conflicting,
) -> Vec<ConflictLine<'a>> {
    let Some(loaded) = page.rows.get(conflict.row) else {
        return Vec::new();
    };
    let row = conflict.row;
    cells
        .range((row, 0)..=(row, usize::MAX))
        .filter_map(|(&(_, col), pending)| {
            let loaded = loaded.get(col)?;
            let server = match &conflict.server {
                Some(server) => Some(server.get(col)?),
                None => None,
            };
            Some(ConflictLine {
                col,
                loaded,
                server,
                yours: &pending.new,
                // As the save compared them: a float by its bits.
                moved: server.is_some_and(|server| !same_value(server, loaded)),
            })
        })
        .collect()
}

/// The pending cells of the page's row `row`, by their column, that are no
/// change any more once the row holds `server`: a new value equal to what
/// the server holds now is where an editor on it would start.
pub fn settled(
    table: &Table<'_>,
    cells: &BTreeMap<(usize, usize), Pending>,
    row: usize,
    server: &[Value],
) -> Vec<usize> {
    cells
        .range((row, 0)..=(row, usize::MAX))
        .filter(|&(&(_, col), pending)| {
            let class = table.class(col).unwrap_or(ColumnClass::Other);
            server
                .get(col)
                .is_some_and(|now| !is_change(now, &pending.new, class))
        })
        .map(|(&(_, col), _)| col)
        .collect()
}

/// A value as the question about a conflict shows it.
#[derive(Clone, PartialEq)]
pub enum Shown {
    Null,
    /// The value's text, or the part of it to show: at most a cell's worth
    /// of characters and one more, so the cell that draws it still marks
    /// what it cuts. `from` is how many characters of the value stand
    /// before it: more than none where the value reads up to there as
    /// another of its line does.
    Text {
        text: String,
        from: usize,
    },
}

/// One column the user changed in a row a save found changed, made ready
/// to draw once, when its question comes up: a value can be megabytes,
/// too much to compare in every frame.
#[derive(Clone, PartialEq)]
pub struct ShownLine {
    /// The column's name, as the page has it.
    pub name: String,
    pub loaded: Shown,
    /// What the database holds now. `None` when the row is gone.
    pub server: Option<Shown>,
    pub yours: Shown,
    /// The server's value is another than the loaded one.
    pub moved: bool,
}

/// How many characters stand before the first difference of two values
/// that are shown from there: enough to find the place by. A cell too
/// narrow for them gives them up first (see `ui::conflict_prompt`).
pub const LEAD: usize = 12;

/// What a cell makes of `text`: one line, a cell's worth of it. Two texts
/// with the same answer read the same in the question.
fn read(text: &str) -> String {
    use crate::ui::format::{Marks, blank_text, cell_line};
    blank_text(text, Marks::PLAIN).unwrap_or_else(|| cell_line(text, Marks::PLAIN).into_owned())
}

/// The values of one line as they are shown, in the order given (`None`
/// is NULL). Each is a cell's worth of its text from the start, unless it
/// reads there as another does that is not the same text (they differ past
/// what a cell shows): those are shown from just before the first place
/// they differ, so the difference is on screen. Pair by pair: where two of
/// three would still be one text from there, those two are shown from just
/// before their own first difference. What a cell has the room to paint of
/// each is for the view to say (see `ui::conflict_prompt`).
fn shown(values: &[Option<&str>]) -> Vec<Shown> {
    let piece = crate::ui::format::CELL_MAX_CHARS + 1;
    let part =
        |text: &str, from: usize| -> String { text.chars().skip(from).take(piece).collect() };
    // How many characters of each value stand before what is shown of it.
    let mut from = vec![0; values.len()];
    // A round moves the values that read like another from the same place
    // on. Each pair parts at its first difference, so there are no more
    // rounds than values.
    for _ in values {
        let reads: Vec<Option<String>> = values
            .iter()
            .zip(&from)
            .map(|(text, &from)| text.map(|text| read(&part(text, from))))
            .collect();
        // For each value, the first place it differs from one it reads
        // like.
        let mut differs: Vec<Option<usize>> = vec![None; values.len()];
        for (later, theirs) in values.iter().enumerate() {
            for (earlier, ours) in values.iter().enumerate().take(later) {
                let (Some(ours), Some(theirs)) = (ours, theirs) else {
                    continue;
                };
                let alike = from[earlier] == from[later] && reads[earlier] == reads[later];
                if ours == theirs || !alike {
                    continue;
                }
                let same = ours.chars().zip(theirs.chars());
                let at = same.take_while(|(ours, theirs)| ours == theirs).count();
                // Two that are shown from their difference already read
                // alike wherever they are read from (a tab against a
                // space): they hold neither back from another it differs
                // from further on.
                if at.saturating_sub(LEAD) <= from[earlier] {
                    continue;
                }
                for value in [earlier, later] {
                    differs[value] = Some(differs[value].map_or(at, |first| first.min(at)));
                }
            }
        }
        let mut moved = false;
        for (from, differs) in from.iter_mut().zip(differs) {
            let to = differs.map_or(*from, |at| at.saturating_sub(LEAD));
            moved |= to != *from;
            *from = to;
        }
        if !moved {
            break;
        }
    }
    values
        .iter()
        .zip(from)
        .map(|(text, from)| match text {
            None => Shown::Null,
            Some(text) => Shown::Text {
                text: part(text, from),
                from,
            },
        })
        .collect()
}

/// The lines the question about `conflict`'s row shows: one for each
/// column the user changed, in the page's order.
pub fn shown_lines(
    page: &RowPage,
    cells: &BTreeMap<(usize, usize), Pending>,
    conflict: &Conflicting,
) -> Vec<ShownLine> {
    // A text as it is; any other value as a cell writes it, which is short.
    fn text(value: &Value) -> Option<std::borrow::Cow<'_, str>> {
        match value {
            Value::Null => None,
            Value::Text(text) => Some(std::borrow::Cow::Borrowed(text)),
            other => Some(crate::ui::format::cell_text(other)),
        }
    }
    conflict_lines(page, cells, conflict)
        .into_iter()
        .filter_map(|line| {
            let loaded = text(line.loaded);
            let server = line.server.map(text);
            let yours = match line.yours {
                NewValue::Null => None,
                NewValue::Text(text) => Some(text.as_str()),
            };
            let mut values = vec![loaded.as_deref(), yours];
            if let Some(server) = &server {
                values.push(server.as_deref());
            }
            let mut values = shown(&values).into_iter();
            Some(ShownLine {
                name: page.columns.get(line.col)?.name.clone(),
                loaded: values.next()?,
                yours: values.next()?,
                server: values.next(),
                moved: line.moved,
            })
        })
        .collect()
}

/// The change set a save sends for the pending `cells`, and the page's row
/// of each of its rows. `None` when nothing is pending or the table has no
/// key.
pub fn change_set(
    object: &ObjectRef,
    table: &Table<'_>,
    cells: &BTreeMap<(usize, usize), Pending>,
) -> Option<(ChangeSet, Vec<usize>)> {
    let key = table.key()?;
    let mut rows: Vec<RowChange> = Vec::new();
    let mut places = Vec::new();
    // The map is ordered by row, then column, so a row's cells are together.
    for (&(row, col), pending) in cells {
        let values = table.page.rows.get(row)?;
        let column = table.column(col)?;
        if places.last() != Some(&row) {
            places.push(row);
            rows.push(RowChange {
                key: key
                    .iter()
                    .map(|&place| {
                        let name = table.page.columns.get(place)?.name.clone();
                        Some((name, values.get(place)?.clone()))
                    })
                    .collect::<Option<_>>()?,
                set: Vec::new(),
            });
        }
        rows.last_mut()?.set.push(CellChange {
            column: column.name.clone(),
            type_name: column.type_name.clone(),
            loaded: values.get(col)?.clone(),
            new: pending.new.clone(),
        });
    }
    (!rows.is_empty()).then(|| {
        (
            ChangeSet {
                object: object.clone(),
                rows,
            },
            places,
        )
    })
}

/// Whether `a` and `b` hold the same value, a float by its bits: as a
/// save compares what a row holds with what the page loaded. Not a number
/// is itself that way, where `==` says it is not, and the two zeroes are
/// two values.
fn same_value(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
        _ => a == b,
    }
}

/// Whether `a` and `b` are the same save: the same table, rows, keys and
/// cells, every value compared as `same_value` does. The sets' own `==`
/// would call a set with a NaN in it another set than itself.
pub fn same_changes(a: &ChangeSet, b: &ChangeSet) -> bool {
    // Every field by name: one added to a set is one to compare here.
    let (
        ChangeSet { object, rows },
        ChangeSet {
            object: other,
            rows: others,
        },
    ) = (a, b);
    let same_cell = |a: &CellChange, b: &CellChange| {
        let CellChange {
            column,
            type_name,
            loaded,
            new,
        } = a;
        *column == b.column
            && *type_name == b.type_name
            && same_value(loaded, &b.loaded)
            && *new == b.new
    };
    let same_row = |a: &RowChange, b: &RowChange| {
        let RowChange { key, set } = a;
        key.len() == b.key.len()
            && key
                .iter()
                .zip(&b.key)
                .all(|((name, value), (other, with))| name == other && same_value(value, with))
            && set.len() == b.set.len()
            && set.iter().zip(&b.set).all(|(a, b)| same_cell(a, b))
    };
    object == other
        && rows.len() == others.len()
        && rows.iter().zip(others).all(|(a, b)| same_row(a, b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tabletist_db::{ColumnMeta, IndexInfo, ObjectRef, ValueKind};

    #[test]
    fn two_change_sets_are_the_same_save_by_the_bits_of_their_floats() {
        let cell = |column: &str, loaded: Value, new: &str| CellChange {
            column: column.into(),
            type_name: "float8".into(),
            loaded,
            new: NewValue::Text(new.into()),
        };
        let row = |key: Value, set: Vec<CellChange>| RowChange {
            key: vec![("id".into(), key)],
            set,
        };
        let set = |rows: Vec<RowChange>| ChangeSet {
            object: ObjectRef::new("main", "readings"),
            rows,
        };
        let nan = || Value::Float(f64::NAN);
        // Not a number is itself, as a changed cell's loaded value and as a
        // row's key: the derived comparison says it is not.
        let made = || {
            set(vec![
                row(Value::Int(1), vec![cell("level", nan(), "2.5")]),
                row(nan(), vec![cell("level", Value::Float(1.5), "2.5")]),
            ])
        };
        assert_ne!(made(), made());
        assert!(same_changes(&made(), &made()));
        // The two zeroes are two values: a save tells them apart.
        let zero = |value: f64| {
            set(vec![row(
                Value::Int(1),
                vec![cell("level", Value::Float(value), "1")],
            )])
        };
        assert!(same_changes(&zero(0.0), &zero(0.0)));
        assert!(!same_changes(&zero(0.0), &zero(-0.0)));
        // Anything else that differs is another save.
        let one = || row(Value::Int(1), vec![cell("level", nan(), "2.5")]);
        let base = set(vec![one()]);
        let others = [
            // An added cell.
            set(vec![row(
                Value::Int(1),
                vec![cell("level", nan(), "2.5"), cell("depth", nan(), "2.5")],
            )]),
            // Another new value, another loaded one, another column.
            set(vec![row(Value::Int(1), vec![cell("level", nan(), "2.6")])]),
            set(vec![row(
                Value::Int(1),
                vec![cell("level", Value::Null, "2.5")],
            )]),
            set(vec![row(Value::Int(1), vec![cell("depth", nan(), "2.5")])]),
            // Another row, by its key, and one more row.
            set(vec![row(Value::Int(2), vec![cell("level", nan(), "2.5")])]),
            set(vec![one(), one()]),
            set(Vec::new()),
        ];
        // Said by its rows: a set is printed by its counts alone.
        for other in &others {
            assert!(!same_changes(&base, other), "{:?}", other.rows);
            assert!(!same_changes(other, &base), "{:?}", other.rows);
        }
        // Another type name, a NULL for a text, another key column, and
        // another table.
        let mut typed = base.clone();
        typed.rows[0].set[0].type_name = "float4".into();
        let mut null = base.clone();
        null.rows[0].set[0].new = NewValue::Null;
        let mut keyed = base.clone();
        keyed.rows[0].key[0].0 = "uuid".into();
        let mut table = base.clone();
        table.object = ObjectRef::new("main", "levels");
        for other in [typed, null, keyed, table] {
            assert!(
                !same_changes(&base, &other),
                "{:?} {:?}",
                other.object,
                other.rows
            );
        }
        assert!(same_changes(&base, &base.clone()));
    }

    fn column(name: &str, type_name: &str) -> ColumnInfo {
        ColumnInfo {
            name: name.into(),
            type_name: type_name.into(),
            nullable: true,
            ..ColumnInfo::default()
        }
    }

    fn structure() -> Structure {
        Structure {
            columns: vec![
                ColumnInfo {
                    nullable: false,
                    ..column("id", "INTEGER")
                },
                column("email", "TEXT"),
                column("meta", "JSON"),
            ],
            primary_key: vec!["id".into()],
            ..Structure::default()
        }
    }

    fn meta(name: &str, type_name: &str) -> ColumnMeta {
        ColumnMeta {
            name: name.into(),
            type_name: type_name.into(),
            kind: ValueKind::Text,
        }
    }

    fn page(rows: Vec<Vec<Value>>) -> RowPage {
        RowPage {
            columns: vec![
                meta("id", "INTEGER"),
                meta("email", "TEXT"),
                meta("meta", "JSON"),
            ],
            rows,
            has_more: false,
            ordered_by_key: true,
            elapsed: Duration::ZERO,
        }
    }

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    fn rows() -> Vec<Vec<Value>> {
        vec![
            vec![Value::Int(1), text("ada@example.com"), Value::Null],
            vec![Value::Int(2), text("bob@example.com"), text("{}")],
        ]
    }

    /// No row is gone.
    static NONE_GONE: BTreeSet<usize> = BTreeSet::new();

    fn table<'a>(structure: Option<&'a Structure>, page: &'a RowPage) -> Table<'a> {
        Table {
            access: Access::Writable,
            kind: ObjectKind::Table,
            dialect: Dialect::Sqlite,
            structure,
            page,
            refreshing: false,
            saving: false,
            gone: &NONE_GONE,
        }
    }

    fn at(row: usize, col: usize) -> CellPos {
        CellPos { row, col }
    }

    #[test]
    fn a_cell_of_a_keyed_table_on_a_writable_connection_can_be_edited() {
        let (structure, page) = (structure(), page(rows()));
        let table = table(Some(&structure), &page);
        assert_eq!(table.lock(at(0, 1)), None);
        assert_eq!(table.lock(at(1, 2)), None);
        assert_eq!(table.key(), Some(vec![0]));
    }

    #[test]
    fn what_cannot_be_edited_says_why() {
        let (structure, page) = (structure(), page(rows()));
        let ok = || table(Some(&structure), &page);
        // The whole table, in the order the reasons are given.
        let read_only = Table {
            access: Access::ReadOnly,
            ..ok()
        };
        assert_eq!(read_only.lock(at(0, 1)), Some(Lock::ReadOnly));
        let view = Table {
            kind: ObjectKind::View,
            ..ok()
        };
        assert_eq!(view.lock(at(0, 1)), Some(Lock::NotATable));
        assert_eq!(
            table(None, &page).lock(at(0, 1)),
            Some(Lock::StructureLoading)
        );
        let keyless = Structure {
            primary_key: Vec::new(),
            ..structure.clone()
        };
        assert_eq!(
            table(Some(&keyless), &page).lock(at(0, 1)),
            Some(Lock::NoKey)
        );
        let saving = Table {
            saving: true,
            ..ok()
        };
        assert_eq!(saving.lock(at(0, 1)), Some(Lock::Saving));
        let refreshing = Table {
            refreshing: true,
            ..ok()
        };
        assert_eq!(refreshing.lock(at(0, 1)), Some(Lock::Refreshing));
        // One cell.
        assert_eq!(ok().lock(at(0, 0)), Some(Lock::KeyColumn));
        assert_eq!(ok().lock(at(9, 1)), Some(Lock::NoSuchCell));
        assert_eq!(ok().lock(at(0, 9)), Some(Lock::NoSuchCell));
    }

    #[test]
    fn a_rows_lock_is_every_reason_before_the_cells_own() {
        let (structure, page) = (structure(), page(rows()));
        let ok = || table(Some(&structure), &page);
        // A row whose cells answer for themselves has none, whatever its
        // cells say: the key column of it is locked, and the row is not.
        assert_eq!(ok().row_lock(0), None);
        assert_eq!(ok().lock(at(0, 0)), Some(Lock::KeyColumn));
        // The table's reasons, and the ones that hold for a while.
        let read_only = Table {
            access: Access::ReadOnly,
            ..ok()
        };
        assert_eq!(read_only.row_lock(0), Some(Lock::ReadOnly));
        let view = Table {
            kind: ObjectKind::View,
            ..ok()
        };
        assert_eq!(view.row_lock(0), Some(Lock::NotATable));
        assert_eq!(table(None, &page).row_lock(0), Some(Lock::StructureLoading));
        let saving = Table {
            saving: true,
            ..ok()
        };
        assert_eq!(saving.row_lock(0), Some(Lock::Saving));
        let refreshing = Table {
            refreshing: true,
            ..ok()
        };
        assert_eq!(refreshing.row_lock(0), Some(Lock::Refreshing));
        // A table with no key, and one whose key a save could not match.
        let keyless = Structure {
            primary_key: Vec::new(),
            ..structure.clone()
        };
        assert_eq!(table(Some(&keyless), &page).row_lock(0), Some(Lock::NoKey));
        let mut stamped = structure.clone();
        stamped.columns[0].type_name = "timestamp".into();
        let mysql = Table {
            dialect: Dialect::MySql,
            ..table(Some(&stamped), &page)
        };
        assert_eq!(mysql.row_lock(0), Some(Lock::KeyType));
        // The row's own: it is not on the page, it is gone, its key is
        // NULL or was not read exactly.
        assert_eq!(ok().row_lock(9), Some(Lock::NoSuchCell));
        let gone = BTreeSet::from([1]);
        let with_gone = Table {
            gone: &gone,
            ..ok()
        };
        assert_eq!(with_gone.row_lock(1), Some(Lock::Gone));
        assert_eq!(with_gone.row_lock(0), None);
        let nulls = self::page(vec![vec![Value::Null, text("a"), Value::Null]]);
        assert_eq!(
            table(Some(&structure), &nulls).row_lock(0),
            Some(Lock::KeyIsNull)
        );
        let inexact = self::page(vec![vec![text("caf\u{FFFD}"), text("a"), Value::Null]]);
        assert_eq!(
            table(Some(&structure), &inexact).row_lock(0),
            Some(Lock::KeyInexact)
        );
        // Only SQLite reads a key's text that way: the others hold it.
        for dialect in [Dialect::Postgres, Dialect::MySql] {
            let exact = Table {
                dialect,
                ..table(Some(&structure), &inexact)
            };
            assert_eq!(exact.row_lock(0), None, "{dialect:?}");
        }
        // A row's reason comes before a cell's: a column the row does not
        // hold answers for the row first.
        assert_eq!(with_gone.lock(at(1, 9)), Some(Lock::Gone));
        assert_eq!(with_gone.lock(at(0, 9)), Some(Lock::NoSuchCell));
        // And every cell of a row answers its row's reason, where it has
        // one, and never another. Where the row has none, a cell's own
        // lock, asked with the key found once, is its whole answer.
        let key = with_gone.key().unwrap();
        for row in 0..2 {
            for col in 0..3 {
                let cell = at(row, col);
                match with_gone.row_lock(row) {
                    Some(lock) => assert_eq!(with_gone.lock(cell), Some(lock)),
                    None => assert_eq!(with_gone.lock(cell), with_gone.own_lock(cell, &key)),
                }
            }
        }
        assert_eq!(with_gone.own_lock(at(0, 0), &key), Some(Lock::KeyColumn));
        assert_eq!(with_gone.own_lock(at(0, 1), &key), None);
    }

    #[test]
    fn a_row_whose_key_is_null_is_locked() {
        let structure = structure();
        let page = page(vec![vec![Value::Null, text("a"), Value::Null]]);
        assert_eq!(
            table(Some(&structure), &page).lock(at(0, 1)),
            Some(Lock::KeyIsNull)
        );
    }

    #[test]
    fn generated_binary_and_huge_cells_are_locked() {
        let mut structure = structure();
        structure.columns[1].generated = true;
        structure.columns[2].type_name = "BLOB".into();
        let page = page(rows());
        let table = table(Some(&structure), &page);
        assert_eq!(table.lock(at(0, 1)), Some(Lock::Generated));
        assert_eq!(table.lock(at(0, 2)), Some(Lock::Binary));
        // Bytes in a column of another type are binary all the same.
        let structure = self::structure();
        let page = self::page(vec![vec![
            Value::Int(1),
            Value::Bytes(vec![1, 2].into()),
            text(&"x".repeat(MAX_EDIT_BYTES + 1)),
        ]]);
        let table = self::table(Some(&structure), &page);
        assert_eq!(table.lock(at(0, 1)), Some(Lock::Binary));
        assert_eq!(table.lock(at(0, 2)), Some(Lock::TooLarge));
    }

    #[test]
    fn a_column_the_structure_does_not_know_or_the_page_names_twice_is_locked() {
        let structure = structure();
        let mut page = page(rows());
        page.columns[2].name = "extra".into();
        assert_eq!(
            table(Some(&structure), &page).lock(at(0, 2)),
            Some(Lock::UnknownColumn)
        );
        // Two columns of one name: no statement can say which it means.
        page.columns[2].name = "email".into();
        let table = table(Some(&structure), &page);
        assert_eq!(table.lock(at(0, 1)), Some(Lock::UnknownColumn));
        assert_eq!(table.lock(at(0, 2)), Some(Lock::UnknownColumn));
    }

    #[test]
    fn a_key_the_save_would_refuse_locks_the_table_or_the_row() {
        // MySQL: a timestamp, bit or float key cannot be matched exactly.
        let mut structure = structure();
        for type_name in [
            "timestamp",
            "timestamp(6)",
            "bit(8)",
            "float",
            "float unsigned",
        ] {
            structure.columns[0].type_name = type_name.into();
            let page = page(rows());
            let table = Table {
                dialect: Dialect::MySql,
                ..table(Some(&structure), &page)
            };
            assert_eq!(table.lock(at(0, 1)), Some(Lock::KeyType), "{type_name}");
        }
        structure.columns[0].type_name = "datetime".into();
        let page_ok = page(rows());
        let table_ok = Table {
            dialect: Dialect::MySql,
            ..table(Some(&structure), &page_ok)
        };
        assert_eq!(table_ok.lock(at(0, 1)), None);
        // SQLite: a key whose text may not have been read exactly.
        let structure = self::structure();
        let page = self::page(vec![vec![text("caf\u{FFFD}"), text("a"), Value::Null]]);
        assert_eq!(
            self::table(Some(&structure), &page).lock(at(0, 1)),
            Some(Lock::KeyInexact)
        );
    }

    #[test]
    fn a_unique_index_is_a_key_when_there_is_no_primary_one() {
        let structure = Structure {
            primary_key: Vec::new(),
            indexes: vec![IndexInfo {
                name: "users_email".into(),
                columns: vec!["email".into()],
                key_columns: Some(vec!["email".into()]),
                unique: true,
                ..IndexInfo::default()
            }],
            columns: vec![
                column("id", "INTEGER"),
                ColumnInfo {
                    nullable: false,
                    ..column("email", "TEXT")
                },
                column("meta", "JSON"),
            ],
            ..Structure::default()
        };
        let page = page(rows());
        let table = table(Some(&structure), &page);
        assert_eq!(table.key(), Some(vec![1]));
        assert_eq!(table.lock(at(0, 1)), Some(Lock::KeyColumn));
        assert_eq!(table.lock(at(0, 0)), None);
    }

    fn typed(type_name: &str) -> ColumnInfo {
        column("c", type_name)
    }

    /// An editor opens no value over `MAX_EDIT_BYTES`, and one that is open
    /// takes a paste of any size where its column sets no length: what it
    /// then holds is no value to keep either.
    #[test]
    fn a_text_larger_than_an_editor_opens_is_refused_before_any_other_rule() {
        let over = "x".repeat(MAX_EDIT_BYTES + 1);
        let at = "x".repeat(MAX_EDIT_BYTES);
        // A column of any length, a document, and a type the app does not
        // know: none has a rule of its own that would have stopped it.
        for (dialect, type_name) in [
            (Dialect::Postgres, "text"),
            (Dialect::MySql, "longtext"),
            (Dialect::Sqlite, "TEXT"),
            (Dialect::Sqlite, ""),
            (Dialect::Postgres, "tsvector"),
        ] {
            let column = typed(type_name);
            assert_eq!(
                check(dialect, &column, &over),
                Some(Problem::TooLarge),
                "{type_name}"
            );
            // The limit itself is a value an editor opens.
            assert_eq!(check(dialect, &column, &at), None, "{type_name}");
        }
        // Bytes, as the lock counts them: fewer characters than the limit
        // can be more bytes than it.
        let wide = "é".repeat(MAX_EDIT_BYTES / 2 + 1);
        assert!(wide.chars().count() < MAX_EDIT_BYTES && wide.len() > MAX_EDIT_BYTES);
        assert_eq!(
            check(Dialect::Postgres, &typed("text"), &wide),
            Some(Problem::TooLarge)
        );
        assert_eq!(
            check(
                Dialect::Postgres,
                &typed("text"),
                &"é".repeat(MAX_EDIT_BYTES / 2)
            ),
            None
        );
        // Before the column's own rule, whose words would be about a text
        // nobody can read through: a number, a document, a length, a list.
        for type_name in ["bigint", "jsonb", "varchar(5)", "numeric(6,2)", "boolean"] {
            assert_eq!(
                check(Dialect::Postgres, &typed(type_name), &over),
                Some(Problem::TooLarge),
                "{type_name}"
            );
        }
        let mut listed = typed("text");
        listed.allowed_values = Some(vec!["print".into(), "ebook".into()]);
        assert_eq!(
            check(Dialect::Postgres, &listed, &over),
            Some(Problem::TooLarge)
        );
    }

    #[test]
    fn a_number_column_takes_numbers_its_type_holds() {
        // The names are the structure's: PostgreSQL's `format_type` says
        // `bigint`, not `int8`.
        let pg = |type_name: &str, text: &str| check(Dialect::Postgres, &typed(type_name), text);
        assert_eq!(pg("bigint", "12"), None);
        assert_eq!(pg("bigint", " -12 "), None);
        assert_eq!(
            pg("bigint", "91000000000000000a1"),
            Some(Problem::WholeNumber)
        );
        assert_eq!(pg("bigint", "1.5"), Some(Problem::WholeNumber));
        assert_eq!(pg("bigint", ""), Some(Problem::WholeNumber));
        assert_eq!(
            pg("smallint", "40000"),
            Some(Problem::OutOfRange {
                min: -32_768,
                max: 32_767
            })
        );
        assert_eq!(pg("double precision", "1e300"), None);
        assert_eq!(pg("double precision", "NaN"), None);
        assert_eq!(pg("double precision", "-Infinity"), None);
        assert_eq!(pg("double precision", "one"), Some(Problem::Number));
        // Only PostgreSQL has the two words.
        assert_eq!(
            check(Dialect::MySql, &typed("double"), "NaN"),
            Some(Problem::Number)
        );
    }

    #[test]
    fn a_decimal_is_refused_with_what_would_have_been_stored() {
        let pg = |text: &str| check(Dialect::Postgres, &typed("numeric(10,2)"), text);
        assert_eq!(pg("12.5"), None);
        assert_eq!(pg("12.50"), None);
        assert_eq!(pg("-0.05"), None);
        assert_eq!(pg(".5"), None);
        assert_eq!(
            pg("12.505"),
            Some(Problem::Decimals {
                scale: 2,
                stored: "12.51".into()
            })
        );
        assert_eq!(
            pg("-9.999"),
            Some(Problem::Decimals {
                scale: 2,
                stored: "-10.00".into()
            })
        );
        assert_eq!(
            pg("-0.001"),
            Some(Problem::Decimals {
                scale: 2,
                stored: "0.00".into()
            })
        );
        assert_eq!(
            pg("007.505"),
            Some(Problem::Decimals {
                scale: 2,
                stored: "7.51".into()
            })
        );
        assert_eq!(pg("123456789.5"), Some(Problem::Digits { whole: 8 }));
        assert_eq!(pg("12345678.5"), None);
        assert_eq!(pg("-99999999.99"), None);
        assert_eq!(pg("100000000"), Some(Problem::Digits { whole: 8 }));
        assert_eq!(pg("1e3"), Some(Problem::Number));
        assert_eq!(pg("twelve"), Some(Problem::Number));
        // Without stated digits, any number.
        assert_eq!(check(Dialect::Postgres, &typed("numeric"), "12.505"), None);
    }

    /// PostgreSQL rounds a numeric to its scale, to tens or hundreds for a
    /// scale below zero, and then holds what is left under ten to the
    /// power of the digits less the scale.
    #[test]
    fn a_scale_below_zero_rounds_whole_digits_and_holds_more_of_them() {
        let pg = |text: &str| check(Dialect::Postgres, &typed("numeric(5,-2)"), text);
        let stored = |stored: &str| {
            Some(Problem::Decimals {
                scale: -2,
                stored: stored.into(),
            })
        };
        for kept in ["12300", "-12300", "0", "100", "12300.00", "0012300", "+500"] {
            assert_eq!(pg(kept), None, "{kept}");
        }
        // Never rounded silently: what would be stored is said.
        assert_eq!(pg("12345"), stored("12300"));
        assert_eq!(pg("12350"), stored("12400"));
        assert_eq!(pg("-12345"), stored("-12300"));
        assert_eq!(pg("-12350"), stored("-12400"));
        assert_eq!(pg("12300.5"), stored("12300"));
        assert_eq!(pg("12349.99"), stored("12300"));
        // Under one hundred there is a hundred or nothing, and no sign on
        // nothing.
        assert_eq!(pg("49"), stored("0"));
        assert_eq!(pg("-49"), stored("0"));
        assert_eq!(pg("50"), stored("100"));
        assert_eq!(pg("0.5"), stored("0"));
        assert_eq!(pg("99950"), stored("100000"));
        // Five digits and two places for zeros: seven before the point.
        assert_eq!(pg("9999900"), None);
        assert_eq!(pg("-9999900"), None);
        assert_eq!(pg("10000000"), Some(Problem::Digits { whole: 7 }));
        assert_eq!(pg("-10000000"), Some(Problem::Digits { whole: 7 }));
        // Refused for its rounding first, which already says a number of
        // eight digits: typed as that, it is refused for them.
        assert_eq!(pg("9999999"), stored("10000000"));
        assert_eq!(pg("99999950"), stored("100000000"));
        assert_eq!(pg("1e3"), Some(Problem::Number));
        // The scale is PostgreSQL's alone: no such name comes from MySQL,
        // and SQLite holds a number to nothing its type states.
        assert_eq!(
            check(Dialect::MySql, &typed("decimal(5,-2)"), "12345"),
            None
        );
        assert_eq!(
            check(Dialect::Sqlite, &typed("NUMERIC(5,-2)"), "12345"),
            None
        );
    }

    /// A type's name is the server's to give: one whose scale has no
    /// opposite states nothing, and a value for it is checked as a number
    /// and no more.
    #[test]
    fn a_scale_no_number_negates_is_checked_as_a_number_only() {
        for (dialect, name) in [
            (Dialect::Postgres, "numeric(5,-2147483648)"),
            (Dialect::MySql, "decimal(5,-2147483648)"),
        ] {
            let check = |text: &str| check(dialect, &typed(name), text);
            assert_eq!(check("12345.678"), None, "{name}");
            assert_eq!(check("0"), None, "{name}");
            assert_eq!(check("abc"), Some(Problem::Number), "{name}");
        }
    }

    #[test]
    fn a_scale_past_the_digits_holds_only_small_numbers() {
        let pg = |text: &str| check(Dialect::Postgres, &typed("numeric(3,5)"), text);
        let under = || {
            Some(Problem::Under {
                limit: "0.01".into(),
            })
        };
        for kept in [
            "0.00999",
            "-0.00999",
            "0",
            "0.0",
            ".00123",
            "0.001",
            "000.00500",
            "0.0012300",
        ] {
            assert_eq!(pg(kept), None, "{kept}");
        }
        // No whole digit, and the first two decimals are zeros.
        assert_eq!(pg("0.01234"), under());
        assert_eq!(pg("0.01"), under());
        assert_eq!(pg("-0.01"), under());
        assert_eq!(pg("0.1"), under());
        assert_eq!(pg("1"), under());
        assert_eq!(pg("12.5"), under());
        // Rounded first, and what it would round to is no number the type
        // holds either.
        assert_eq!(
            pg("0.009995"),
            Some(Problem::Decimals {
                scale: 5,
                stored: "0.01000".into()
            })
        );
        assert_eq!(pg("0.01000"), under());
        assert_eq!(
            pg("0.001234"),
            Some(Problem::Decimals {
                scale: 5,
                stored: "0.00123".into()
            })
        );
        // As many decimals as digits: anything under one.
        let unit = |text: &str| check(Dialect::Postgres, &typed("numeric(2,2)"), text);
        assert_eq!(unit("0.99"), None);
        assert_eq!(unit("-.99"), None);
        assert_eq!(unit("1"), Some(Problem::Under { limit: "1".into() }));
        assert_eq!(unit("1.5"), Some(Problem::Under { limit: "1".into() }));
        // MySQL's scale is never past its digits, and can equal them.
        let my = |text: &str| check(Dialect::MySql, &typed("decimal(2,2)"), text);
        assert_eq!(my("0.99"), None);
        assert_eq!(my("1.00"), Some(Problem::Under { limit: "1".into() }));
    }

    #[test]
    fn a_scale_of_zero_rounds_to_a_whole_number() {
        let pg = |text: &str| check(Dialect::Postgres, &typed("numeric(5)"), text);
        assert_eq!(pg("12345"), None);
        assert_eq!(pg("12.0"), None);
        assert_eq!(
            pg("12.5"),
            Some(Problem::Decimals {
                scale: 0,
                stored: "13".into()
            })
        );
        assert_eq!(
            pg("-0.4"),
            Some(Problem::Decimals {
                scale: 0,
                stored: "0".into()
            })
        );
        assert_eq!(pg("123456"), Some(Problem::Digits { whole: 5 }));
    }

    #[test]
    fn booleans_json_lengths_and_lists_are_checked() {
        let pg = |type_name: &str, text: &str| check(Dialect::Postgres, &typed(type_name), text);
        for ok in ["true", "false", "1", "0", "TRUE"] {
            assert_eq!(pg("boolean", ok), None, "{ok}");
        }
        assert_eq!(pg("boolean", "yes"), Some(Problem::Boolean));
        assert_eq!(pg("jsonb", r#"{"a": [1, 2]}"#), None);
        let Some(Problem::Json { line, column, .. }) = pg("jsonb", "{\n  \"a\": 1\n  \"b\": 2}")
        else {
            panic!("expected a JSON problem");
        };
        assert_eq!((line, column), (3, 3));
        assert_eq!(pg("character varying(5)", "hello"), None);
        assert_eq!(
            pg("character varying(5)", "hello!"),
            Some(Problem::TooLong { max: 5 })
        );
        // Characters, not bytes.
        assert_eq!(pg("character varying(5)", "héllo"), None);
        let listed = ColumnInfo {
            allowed_values: Some(vec!["print".into(), "ebook".into()]),
            ..typed("text")
        };
        assert_eq!(check(Dialect::Postgres, &listed, "ebook"), None);
        assert_eq!(
            check(Dialect::Postgres, &listed, "audio"),
            Some(Problem::NotOneOf(vec!["print".into(), "ebook".into()]))
        );
        // A type the app does not know has no check.
        assert_eq!(pg("tsvector", "anything"), None);
    }

    #[test]
    fn sqlite_checks_by_affinity_and_never_a_length() {
        let lite = |type_name: &str, text: &str| check(Dialect::Sqlite, &typed(type_name), text);
        assert_eq!(lite("INTEGER", "12"), None);
        assert_eq!(lite("INTEGER", "1.5"), Some(Problem::WholeNumber));
        assert_eq!(lite("VARCHAR(3)", "longer"), None);
        assert_eq!(lite("NUMERIC(10,2)", "12.505"), None);
        assert_eq!(lite("", "anything"), None);
    }

    #[test]
    fn sqlite_refuses_a_decimal_it_would_not_keep_digit_for_digit() {
        // SQLite keeps a number as INTEGER or REAL, whatever digits the
        // declared type states.
        let lite = |text: &str| check(Dialect::Sqlite, &typed("NUMERIC(30,20)"), text);
        let inexact = |stored: &str| {
            Some(Problem::Inexact {
                stored: stored.into(),
            })
        };
        // A whole number an INTEGER holds, however it is written.
        for whole in ["12", "-12", "+12", "007", "0", "9223372036854775807"] {
            assert_eq!(lite(whole), None, "{whole}");
        }
        assert_eq!(lite("-9223372036854775808"), None);
        // A number a REAL holds digit for digit.
        for exact in [
            "12.5", "12.50", "-0.05", ".5", "5.", "+1.25", "0012.5", "12.0", "0.1",
        ] {
            assert_eq!(lite(exact), None, "{exact}");
        }
        // Past an INTEGER, but a REAL that is written the same.
        assert_eq!(lite("100000000000000000000"), None);
        // What a REAL would round, with what it would keep.
        assert_eq!(
            lite("99999999999999999999"),
            inexact("100000000000000000000")
        );
        assert_eq!(
            lite("0.12345678901234567891"),
            inexact("0.12345678901234568")
        );
        // One past the largest INTEGER is a REAL, and a cell shows a REAL by
        // the digits that tell it from its neighbours.
        assert_eq!(lite("9223372036854775808"), inexact("9223372036854776000"));
        assert_eq!(
            lite("-0.30000000000000004441"),
            inexact("-0.30000000000000004")
        );
        // Too large for a REAL at all: no number the builder sends.
        assert_eq!(lite(&"9".repeat(400)), Some(Problem::Number));
        assert_eq!(lite("twelve"), Some(Problem::Number));
        assert_eq!(lite("1e3"), Some(Problem::Number));
        // The other databases keep a decimal's digits.
        for dialect in [Dialect::Postgres, Dialect::MySql] {
            let column = typed(if dialect == Dialect::Postgres {
                "numeric"
            } else {
                "decimal(65,30)"
            });
            assert_eq!(check(dialect, &column, "0.12345678901234567891"), None);
            assert_eq!(check(dialect, &column, "99999999999999999999"), None);
        }
    }

    #[test]
    fn a_mysql_boolean_takes_any_number_a_tinyint_holds() {
        // As the save's builder does: some tables keep more than a flag in
        // a tinyint(1).
        let my = |text: &str| check(Dialect::MySql, &typed("tinyint(1)"), text);
        for ok in ["true", "FALSE", "1", "0", "5", "-128", "127", " 7 ", "+7"] {
            assert_eq!(my(ok), None, "{ok}");
        }
        let range = Some(Problem::OutOfRange {
            min: -128,
            max: 127,
        });
        assert_eq!(my("128"), range);
        assert_eq!(my("-129"), range);
        for bad in ["yes", "", "1.5", "t"] {
            assert_eq!(my(bad), Some(Problem::Boolean), "{bad}");
        }
        // Elsewhere a boolean is a flag.
        for dialect in [Dialect::Postgres, Dialect::Sqlite] {
            let type_name = if dialect == Dialect::Postgres {
                "boolean"
            } else {
                "BOOLEAN"
            };
            assert_eq!(check(dialect, &typed(type_name), "1"), None);
            assert_eq!(
                check(dialect, &typed(type_name), "5"),
                Some(Problem::Boolean)
            );
        }
    }

    #[test]
    fn a_postgres_float_takes_the_words_an_editor_starts_from() {
        let pg = |text: &str| check(Dialect::Postgres, &typed("double precision"), text);
        // Where the editor of such a cell starts.
        for value in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            let start = start_text(&Value::Float(value), ColumnClass::Float);
            assert_eq!(pg(&start), None, "{start}");
        }
        // As PostgreSQL reads them: in any case, a sign on infinity only.
        for word in [
            "inf",
            "-inf",
            "+inf",
            "Infinity",
            "-INFINITY",
            "+infinity",
            "nan",
            "NaN",
            " NAN ",
        ] {
            assert_eq!(pg(word), None, "{word}");
        }
        for bad in ["-nan", "+nan", "infinite", "in", "- inf", "1e999"] {
            assert_eq!(pg(bad), Some(Problem::Number), "{bad}");
        }
        assert_eq!(check(Dialect::Postgres, &typed("real"), "inf"), None);
        // Only PostgreSQL has them.
        for (dialect, type_name) in [(Dialect::MySql, "double"), (Dialect::Sqlite, "REAL")] {
            for word in ["inf", "nan", "Infinity"] {
                assert_eq!(
                    check(dialect, &typed(type_name), word),
                    Some(Problem::Number),
                    "{dialect:?} {word}"
                );
            }
        }
    }

    #[test]
    fn only_ascii_whitespace_around_a_value_is_overlooked() {
        // What is sent is the text as typed, and no database overlooks a
        // non-breaking space around a number.
        let pg = |type_name: &str, text: &str| check(Dialect::Postgres, &typed(type_name), text);
        assert_eq!(pg("bigint", " \t12\n"), None);
        assert_eq!(pg("bigint", "\u{a0}12"), Some(Problem::WholeNumber));
        assert_eq!(pg("bigint", "12\u{2003}"), Some(Problem::WholeNumber));
        assert_eq!(pg("numeric(10,2)", " 12.5 "), None);
        assert_eq!(pg("numeric(10,2)", "12.5\u{a0}"), Some(Problem::Number));
        assert_eq!(pg("double precision", " 1.5 "), None);
        assert_eq!(pg("double precision", "\u{a0}1.5"), Some(Problem::Number));
        assert_eq!(pg("boolean", " true "), None);
        assert_eq!(pg("boolean", "true\u{a0}"), Some(Problem::Boolean));
        assert_eq!(
            check(Dialect::MySql, &typed("tinyint(1)"), "\u{a0}1"),
            Some(Problem::Boolean)
        );
        assert_eq!(
            check(Dialect::Sqlite, &typed("NUMERIC"), "\u{a0}1.5"),
            Some(Problem::Number)
        );
    }

    #[test]
    fn an_editor_starts_from_the_whole_value() {
        let text_class = ColumnClass::Text { max_chars: None };
        assert_eq!(start_text(&Value::Null, text_class), "");
        assert_eq!(start_text(&text("a\nb"), text_class), "a\nb");
        assert_eq!(start_text(&Value::Float(0.1), ColumnClass::Float), "0.1");
        // A boolean reads true or false whatever the driver loaded.
        assert_eq!(start_text(&Value::Int(1), ColumnClass::Boolean), "true");
        assert_eq!(start_text(&Value::Int(0), ColumnClass::Boolean), "false");
        assert_eq!(start_text(&Value::Bool(true), ColumnClass::Boolean), "true");
        // But not what is no flag: MySQL keeps other numbers in a tinyint(1).
        assert_eq!(start_text(&Value::Int(5), ColumnClass::Boolean), "5");
    }

    #[test]
    fn a_value_is_a_change_only_when_it_differs_from_where_the_editor_starts() {
        let class = ColumnClass::Text { max_chars: None };
        let loaded = text("ada");
        assert!(!is_change(&loaded, &NewValue::Text("ada".into()), class));
        assert!(is_change(&loaded, &NewValue::Text("Ada".into()), class));
        assert!(is_change(&loaded, &NewValue::Null, class));
        assert!(!is_change(&Value::Null, &NewValue::Null, class));
        // The empty string is not NULL.
        assert!(is_change(
            &Value::Null,
            &NewValue::Text(String::new()),
            class
        ));
        assert!(!is_change(
            &Value::Int(1),
            &NewValue::Text("true".into()),
            ColumnClass::Boolean
        ));
    }

    #[test]
    fn a_document_starts_laid_out_and_its_white_space_is_no_change() {
        let json = ColumnClass::Json;
        let loaded = text(r#"{"a":1,"b":[true]}"#);
        let start = start_text(&loaded, json);
        assert_eq!(start, "{\n  \"a\": 1,\n  \"b\": [\n    true\n  ]\n}");
        let new = |text: &str| NewValue::Text(text.into());
        // As it opens, as it was loaded, and spaced any other way: the
        // document that was loaded.
        assert!(!is_change(&loaded, &new(&start), json));
        assert!(!is_change(&loaded, &new(r#"{"a":1,"b":[true]}"#), json));
        assert!(!is_change(
            &loaded,
            &new("{ \"a\" : 1 , \"b\" : [ true ] }"),
            json
        ));
        assert!(is_change(&loaded, &new(r#"{"a":2,"b":[true]}"#), json));
        // The white space inside a string is the string's.
        let spaced = text(r#"{"a":"x y"}"#);
        assert!(is_change(&spaced, &new(r#"{"a":"x  y"}"#), json));
        // What such a column holds that is no document stays as it is,
        // and a text broken while it was typed is a change.
        let odd = text("not json  at all");
        assert_eq!(start_text(&odd, json), "not json  at all");
        assert!(is_change(&loaded, &new(r#"{"a":1,"b":[true]"#), json));
        // A text column's JSON is its text.
        let plain = ColumnClass::Text { max_chars: None };
        assert_eq!(start_text(&loaded, plain), r#"{"a":1,"b":[true]}"#);
        assert!(is_change(&loaded, &new(&start), plain));
    }

    #[test]
    fn long_broken_and_json_values_open_the_large_editor() {
        let plain = ColumnClass::Text { max_chars: None };
        assert!(!opens_large("short", plain));
        assert!(opens_large("two\nlines", plain));
        assert!(opens_large(&"x".repeat(257), plain));
        assert!(!opens_large(&"x".repeat(256), plain));
        assert!(opens_large("{}", ColumnClass::Json));
    }

    #[test]
    fn the_set_is_printed_without_what_was_typed() {
        let mut edits = Edits::default();
        edits.cells.insert(
            (0, 1),
            Pending {
                new: NewValue::Text("a secret".into()),
                state: State::Ready,
            },
        );
        edits.editor = Some(Editor {
            cell: at(0, 2),
            place: EditorPlace::Grid,
            text: "another secret".into(),
            large: false,
            focus: false,
            top: false,
            touched: true,
            problem: None,
        });
        let printed = format!("{edits:?}");
        assert!(!printed.contains("secret"), "{printed}");
        assert!(printed.contains("cells: 1"), "{printed}");
    }

    #[test]
    fn a_change_of_the_set_leaves_its_review_stale() {
        let made = || crate::review::Review {
            changes: 1,
            rows: 1,
            lines: Vec::new(),
            refused: None,
        };
        let pending = || Pending {
            new: NewValue::Text("a secret".into()),
            state: State::Ready,
        };
        let mut edits = Edits {
            reviewing: true,
            review: Some(made()),
            ..Edits::default()
        };
        edits.put((0, 1), pending());
        assert!(edits.review.is_none() && edits.reviewing);
        assert_eq!(edits.counts().changes, 1);
        edits.review = Some(made());
        edits.revert((0, 1));
        assert!(edits.review.is_none() && edits.reviewing);
        assert!(edits.cells.is_empty());
        // A failed statement changes no statement: the review stands.
        edits.put((0, 1), pending());
        edits.review = Some(made());
        edits.fail(Some(0), Error::query("no"));
        assert!(edits.review.is_some());
        // The review is not printed either.
        let printed = format!("{edits:?}");
        assert!(!printed.contains("secret"), "{printed}");
    }

    #[test]
    fn the_change_set_names_each_row_by_its_key_and_carries_what_was_loaded() {
        let (structure, page) = (structure(), page(rows()));
        let table = table(Some(&structure), &page);
        let ready = |new: NewValue| Pending {
            new,
            state: State::Ready,
        };
        let mut cells = BTreeMap::new();
        cells.insert((1, 2), ready(NewValue::Null));
        cells.insert((1, 1), ready(NewValue::Text("b@example.com".into())));
        cells.insert((0, 1), ready(NewValue::Text("a@example.com".into())));
        let (changes, places) =
            change_set(&ObjectRef::new("main", "users"), &table, &cells).unwrap();
        // One change per row, in the page's order, and where each came from.
        assert_eq!(places, [0, 1]);
        assert_eq!(changes.rows.len(), 2);
        assert_eq!(changes.rows[0].key, [("id".to_owned(), Value::Int(1))]);
        let second = &changes.rows[1];
        assert_eq!(second.key, [("id".to_owned(), Value::Int(2))]);
        assert_eq!(
            second
                .set
                .iter()
                .map(|cell| (cell.column.as_str(), cell.type_name.as_str()))
                .collect::<Vec<_>>(),
            [("email", "TEXT"), ("meta", "JSON")]
        );
        assert_eq!(second.set[1].loaded, text("{}"));
        assert_eq!(second.set[1].new, NewValue::Null);
        assert_eq!(changes.check(), Ok(()));
        // Nothing pending is nothing to send.
        assert!(change_set(&ObjectRef::new("main", "users"), &table, &BTreeMap::new()).is_none());
    }

    #[test]
    fn a_row_a_save_found_gone_is_locked_and_stays_gone_through_a_discard() {
        let (structure, page) = (structure(), page(rows()));
        let gone = BTreeSet::from([1]);
        let table = Table {
            gone: &gone,
            ..table(Some(&structure), &page)
        };
        // Every cell of it, the key's too: the row is not there to edit.
        assert_eq!(table.lock(at(1, 1)), Some(Lock::Gone));
        assert_eq!(table.lock(at(1, 0)), Some(Lock::Gone));
        assert_eq!(table.lock(at(0, 1)), None);
        // What holds for the whole table, or for a while, comes first.
        let read_only = Table {
            access: Access::ReadOnly,
            ..table
        };
        assert_eq!(read_only.lock(at(1, 1)), Some(Lock::ReadOnly));
        let saving = Table {
            saving: true,
            ..table
        };
        assert_eq!(saving.lock(at(1, 1)), Some(Lock::Saving));
        // A gone row is nothing of the user's: it does not hold the page.
        let mut edits = Edits {
            gone: gone.clone(),
            ..Edits::default()
        };
        assert!(!edits.holds());
        // Dropping what is pending does not bring the row back.
        edits.cells.insert(
            (0, 1),
            Pending {
                new: NewValue::Null,
                state: State::Ready,
            },
        );
        edits.note = Some(Note::Cancelled);
        edits.why = Some((at(1, 1), Lock::Gone));
        edits.discard();
        assert!(edits.cells.is_empty() && edits.note.is_none() && edits.why.is_none());
        assert_eq!(edits.gone, gone);
        assert!(!edits.holds());
        assert!(format!("{edits:?}").contains("gone: {1}"));
    }

    fn pending(new: NewValue) -> Pending {
        Pending {
            new,
            state: State::Ready,
        }
    }

    #[test]
    fn a_saves_conflicts_are_rows_of_the_page_in_the_saves_order() {
        let page = page(rows());
        let server = vec![Value::Int(2), text("eve@example.com"), text("{}")];
        // The set's rows 0 and 1 were the page's rows 1 and 0.
        let places = [1, 0];
        let both = vec![
            Conflict {
                row: 0,
                server: Some(server.clone()),
            },
            Conflict {
                row: 1,
                server: None,
            },
        ];
        assert_eq!(
            conflicting(&places, both, &page),
            Some(vec![
                Conflicting {
                    row: 1,
                    server: Some(server),
                },
                Conflicting {
                    row: 0,
                    server: None,
                },
            ])
        );
        let one = |row: usize, server: Option<Vec<Value>>| vec![Conflict { row, server }];
        // Nothing to ask from: a row the save did not send, a row the page
        // does not hold, one row twice, a row of another width.
        assert_eq!(conflicting(&places, one(2, None), &page), None);
        assert_eq!(conflicting(&[7], one(0, None), &page), None);
        let twice = [one(0, None), one(0, None)].concat();
        assert_eq!(conflicting(&places, twice, &page), None);
        let narrow = one(0, Some(vec![Value::Int(2)]));
        assert_eq!(conflicting(&places, narrow, &page), None);
        // No conflict is no row.
        assert_eq!(conflicting(&places, Vec::new(), &page), Some(Vec::new()));
        // A row is printed without what the database holds in it.
        let row = Conflicting {
            row: 1,
            server: Some(vec![text("secret")]),
        };
        assert_eq!(format!("{row:?}"), "Conflicting { row: 1, gone: false }");
    }

    #[test]
    fn a_conflict_shows_the_columns_the_user_changed_and_which_of_them_moved() {
        let page = page(rows());
        let mut cells = BTreeMap::new();
        cells.insert((1, 2), pending(NewValue::Null));
        cells.insert((1, 1), pending(NewValue::Text("bobby@example.com".into())));
        cells.insert((0, 1), pending(NewValue::Text("a@example.com".into())));
        let changed = Conflicting {
            row: 1,
            server: Some(vec![Value::Int(2), text("eve@example.com"), text("{}")]),
        };
        let lines = conflict_lines(&page, &cells, &changed);
        // The row's own cells and no other's, in the page's column order.
        let cols: Vec<usize> = lines.iter().map(|line| line.col).collect();
        assert_eq!(cols, [1, 2]);
        assert_eq!(lines[0].loaded, &text("bob@example.com"));
        assert_eq!(lines[0].server, Some(&text("eve@example.com")));
        assert_eq!(lines[0].yours, &NewValue::Text("bobby@example.com".into()));
        assert_eq!(lines[1].yours, &NewValue::Null);
        // The email is another on the server, and meta is as it was loaded.
        assert!(lines[0].moved && !lines[1].moved);
        // A row that is gone has nothing on the server.
        let gone = Conflicting {
            row: 1,
            server: None,
        };
        let lines = conflict_lines(&page, &cells, &gone);
        assert_eq!(lines.len(), 2);
        assert!(
            lines
                .iter()
                .all(|line| line.server.is_none() && !line.moved)
        );
        // Not a number is the value it was, as the save compares it.
        let floats = self::page(vec![vec![
            Value::Int(1),
            Value::Float(f64::NAN),
            Value::Null,
        ]]);
        let mut cells = BTreeMap::new();
        cells.insert((0, 1), pending(NewValue::Text("1.5".into())));
        cells.insert((0, 2), pending(NewValue::Text("{}".into())));
        let same = Conflicting {
            row: 0,
            server: Some(vec![Value::Int(1), Value::Float(f64::NAN), text("[]")]),
        };
        let lines = conflict_lines(&floats, &cells, &same);
        assert!(!lines[0].moved && lines[1].moved);
        // A row the page does not hold shows nothing.
        let missing = Conflicting {
            row: 9,
            server: None,
        };
        assert!(conflict_lines(&page, &cells, &missing).is_empty());
        // Each answer is offered where it means something.
        for answer in [Answer::UseServer, Answer::Overwrite] {
            assert!(answer.offered(false) && !answer.offered(true), "{answer:?}");
        }
        assert!(Answer::Discard.offered(true) && !Answer::Discard.offered(false));
        assert!(Answer::KeepMine.offered(true) && Answer::KeepMine.offered(false));
    }

    #[test]
    fn a_pending_value_the_server_holds_now_is_no_change_any_more() {
        let (structure, page) = (structure(), page(rows()));
        let table = table(Some(&structure), &page);
        let mut cells = BTreeMap::new();
        cells.insert((1, 1), pending(NewValue::Text("eve@example.com".into())));
        cells.insert((1, 2), pending(NewValue::Null));
        cells.insert((0, 1), pending(NewValue::Text("eve@example.com".into())));
        // The server holds the new email, and still a value in meta.
        let server = [Value::Int(2), text("eve@example.com"), text("{}")];
        assert_eq!(settled(&table, &cells, 1, &server), [1]);
        // It holds NULL in meta now: both cells are what it has.
        let server = [Value::Int(2), text("eve@example.com"), Value::Null];
        assert_eq!(settled(&table, &cells, 1, &server), [1, 2]);
        // Text is never NULL: the empty string over a NULL stays a change.
        cells.insert((1, 2), pending(NewValue::Text(String::new())));
        assert_eq!(settled(&table, &cells, 1, &server), [1]);
        // A cell to fix or failed is settled the same way: its state is
        // about the new value, not about what was loaded.
        cells.insert(
            (1, 1),
            Pending {
                new: NewValue::Text("eve@example.com".into()),
                state: State::Failed(Error::query("violates check")),
            },
        );
        assert_eq!(settled(&table, &cells, 1, &server), [1]);
        // Another row's cells are not this row's to settle, and a row
        // narrower than the page settles nothing.
        let other = [Value::Int(1), text("x@example.com"), Value::Null];
        assert_eq!(settled(&table, &cells, 0, &other), Vec::<usize>::new());
        assert_eq!(
            settled(&table, &cells, 1, &[Value::Int(2)]),
            Vec::<usize>::new()
        );
        // A boolean column is compared as its editor starts: `true` is the
        // 1 SQLite holds.
        let mut flags = self::structure();
        flags.columns[2].type_name = "BOOLEAN".into();
        let table = self::table(Some(&flags), &page);
        let mut cells = BTreeMap::new();
        cells.insert((1, 2), pending(NewValue::Text("true".into())));
        let server = [Value::Int(2), text("bob@example.com"), Value::Int(1)];
        assert_eq!(settled(&table, &cells, 1, &server), [2]);
        let server = [Value::Int(2), text("bob@example.com"), Value::Int(0)];
        assert_eq!(settled(&table, &cells, 1, &server), Vec::<usize>::new());
    }

    /// A shown value by its text and how far inside the value that starts.
    /// `None` is NULL.
    fn seen(shown: &Shown) -> Option<(&str, usize)> {
        match shown {
            Shown::Null => None,
            Shown::Text { text, from } => Some((text.as_str(), *from)),
        }
    }

    #[test]
    fn two_values_of_a_line_that_differ_are_not_shown_alike() {
        // Three documents that are the same for 300 characters.
        let long = |end: &str| format!("{}{end}", "x".repeat(300));
        let loaded = vec![Value::Int(1), text("a b"), text(&long("loaded"))];
        let page = self::page(vec![loaded]);
        let mut cells = BTreeMap::new();
        cells.insert((0, 1), pending(NewValue::Text("a\nb".into())));
        cells.insert((0, 2), pending(NewValue::Text(long("yours"))));
        let conflict = Conflicting {
            row: 0,
            server: Some(vec![Value::Int(1), text("a b"), text(&long("server"))]),
        };
        let lines = shown_lines(&page, &cells, &conflict);
        assert_eq!(lines.len(), 2);
        let (email, meta) = (&lines[0], &lines[1]);
        assert_eq!((email.name.as_str(), meta.name.as_str()), ("email", "meta"));
        // A line break is not a space: a cell marks it, so the two read
        // apart as they are, each from its start.
        assert_eq!(seen(&email.loaded), Some(("a b", 0)));
        assert_eq!(seen(&email.yours), Some(("a\nb", 0)));
        assert_eq!(email.server.as_ref().map(seen), Some(Some(("a b", 0))));
        assert!(!email.moved);
        // The documents read alike for all a cell shows of them: each is
        // shown from twelve characters before the first place two differ.
        let piece = |end: &str| format!("{}{end}", "x".repeat(12));
        assert_eq!(seen(&meta.loaded), Some((piece("loaded").as_str(), 288)));
        assert_eq!(seen(&meta.yours), Some((piece("yours").as_str(), 288)));
        let server = meta.server.as_ref().map(seen);
        assert_eq!(server, Some(Some((piece("server").as_str(), 288))));
        assert!(meta.moved);
        // Only what reads like another value is shown from inside: a short
        // value beside two long ones is whole.
        cells.insert((0, 2), pending(NewValue::Text("{}".into())));
        let lines = shown_lines(&page, &cells, &conflict);
        assert_eq!(seen(&lines[1].yours), Some(("{}", 0)));
        assert_eq!(
            seen(&lines[1].loaded),
            Some((piece("loaded").as_str(), 288))
        );
        // A value the server kept is the same text, not one that reads
        // like it: both are shown from the start, a cell's worth and one
        // character more for the cell to cut at.
        let kept = Conflicting {
            row: 0,
            server: Some(vec![Value::Int(1), text("c d"), text(&long("loaded"))]),
        };
        let lines = shown_lines(&page, &cells, &kept);
        let start = "x".repeat(257);
        assert_eq!(seen(&lines[1].loaded), Some((start.as_str(), 0)));
        assert_eq!(
            lines[1].server.as_ref().map(seen),
            Some(Some((start.as_str(), 0)))
        );
        assert!(lines[0].moved && !lines[1].moved);
    }

    #[test]
    fn three_values_that_differ_in_two_places_are_each_shown_where_they_differ() {
        // A thousand characters. What was loaded and the user's are the
        // same for nine hundred; the server's is another from the three
        // hundredth on.
        let long = |early: char, late: char| {
            let mut text: Vec<char> = "x".repeat(1000).chars().collect();
            (text[300], text[900]) = (early, late);
            text.into_iter().collect::<String>()
        };
        let loaded = vec![Value::Int(1), text("a b"), text(&long('a', 'p'))];
        let page = self::page(vec![loaded]);
        let mut cells = BTreeMap::new();
        cells.insert((0, 2), pending(NewValue::Text(long('a', 'q'))));
        let now = vec![Value::Int(1), text("a b"), text(&long('b', 'p'))];
        let conflict = Conflicting {
            row: 0,
            server: Some(now),
        };
        let lines = shown_lines(&page, &cells, &conflict);
        let meta = &lines[0];
        // From twelve before the first place the three differ, the loaded
        // one and the user's would still be one text: those two are shown
        // from twelve before the place they differ.
        let late = |at: char| format!("{}{at}{}", "x".repeat(12), "x".repeat(99));
        assert_eq!(seen(&meta.loaded), Some((late('p').as_str(), 888)));
        assert_eq!(seen(&meta.yours), Some((late('q').as_str(), 888)));
        let early = format!("{}b{}", "x".repeat(12), "x".repeat(244));
        let server = meta.server.as_ref().map(seen);
        assert_eq!(server, Some(Some((early.as_str(), 288))));
        // With the server's as it was loaded, the two that are one text
        // are shown as one, and the user's apart from them.
        let kept = Conflicting {
            row: 0,
            server: Some(vec![Value::Int(1), text("a b"), text(&long('a', 'p'))]),
        };
        let lines = shown_lines(&page, &cells, &kept);
        let meta = &lines[0];
        assert_eq!(seen(&meta.loaded), Some((late('p').as_str(), 888)));
        assert_eq!(meta.server.as_ref().map(seen), Some(seen(&meta.loaded)));
        assert_eq!(seen(&meta.yours), Some((late('q').as_str(), 888)));
        // A server's value that reads as the loaded one does wherever it
        // is read from (a tab where that has a space) does not hold the
        // loaded one where the two differ: the user's differs from it
        // further on.
        let conflict = Conflicting {
            row: 0,
            server: Some(vec![Value::Int(1), text("a b"), text(&long('\t', 'p'))]),
        };
        let page = self::page(vec![vec![
            Value::Int(1),
            text("a b"),
            text(&long(' ', 'p')),
        ]]);
        cells.insert((0, 2), pending(NewValue::Text(long(' ', 'q'))));
        let lines = shown_lines(&page, &cells, &conflict);
        let meta = &lines[0];
        let late = |at: char| format!("{}{at}{}", "x".repeat(12), "x".repeat(99));
        assert_eq!(seen(&meta.loaded), Some((late('p').as_str(), 888)));
        assert_eq!(seen(&meta.yours), Some((late('q').as_str(), 888)));
        let early = format!("{}\t{}", "x".repeat(12), "x".repeat(244));
        let server = meta.server.as_ref().map(seen);
        assert_eq!(server, Some(Some((early.as_str(), 288))));
    }

    #[test]
    fn null_numbers_and_a_row_that_is_gone_are_shown_as_they_are() {
        let page = self::page(vec![vec![Value::Int(7), Value::Null, Value::Float(1.5)]]);
        let mut cells = BTreeMap::new();
        cells.insert((0, 1), pending(NewValue::Text(String::new())));
        cells.insert((0, 2), pending(NewValue::Null));
        let conflict = Conflicting {
            row: 0,
            server: Some(vec![Value::Int(7), text("x"), Value::Null]),
        };
        let lines = shown_lines(&page, &cells, &conflict);
        // NULL is NULL and the empty text is a text: the cell that draws
        // them tells them apart.
        assert_eq!(seen(&lines[0].loaded), None);
        assert_eq!(seen(&lines[0].yours), Some(("", 0)));
        assert_eq!(lines[0].server.as_ref().map(seen), Some(Some(("x", 0))));
        // A number as a cell writes it, and a value that became NULL.
        assert_eq!(seen(&lines[1].loaded), Some(("1.5", 0)));
        assert_eq!(seen(&lines[1].yours), None);
        assert_eq!(lines[1].server.as_ref().map(seen), Some(None));
        assert!(lines[0].moved && lines[1].moved);
        // A row that is gone has no third value.
        let gone = Conflicting {
            row: 0,
            server: None,
        };
        let lines = shown_lines(&page, &cells, &gone);
        assert_eq!(lines.len(), 2);
        assert!(
            lines
                .iter()
                .all(|line| line.server.is_none() && !line.moved)
        );
    }

    #[test]
    fn a_question_takes_answers_once_it_has_been_up_for_a_moment() {
        let now = Instant::now();
        assert!(!answers_taken(now));
        // An instant still to come has lasted no time.
        assert!(!answers_taken(now + Duration::from_secs(3600)));
        let earlier = now.checked_sub(ANSWER_AFTER).expect("an earlier instant");
        assert!(answers_taken(earlier));
        assert_eq!(ANSWER_AFTER, Duration::from_millis(500));
    }
}
