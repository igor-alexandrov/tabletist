//! Review SQL: the statements a save of a tab's pending changes would run,
//! as lines a person reads before saving. The reducer makes them when the
//! pending set changes; a view words the comment lines and lays the rest
//! out. Nothing here is a sentence of the app's: a comment is data.

use std::collections::BTreeMap;

use tabletist_db::{
    ChangeSet, Dialect, Error, InsertStatement, ObjectRef, RowChange, RowUpdate, Value,
};

use crate::edit::{NEW_ROWS, Pending, State, Table, change_set, new_row};
use crate::ui::format;

/// The most characters of a value that are shown: a longer one is cut, with
/// `…` for the rest. Only where it is shown: the statement that runs, and
/// the text that is copied, hold the whole value.
pub const VALUE_MAX_CHARS: usize = 60;

/// How a piece of a statement reads, for the colour it is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ink {
    /// Names, `=`, commas, the semicolon.
    Plain,
    /// `UPDATE`, `SET`, `WHERE`, `AND`, `INSERT INTO`, `VALUES`,
    /// `RETURNING`, and a value that is a word: `NULL`, `DEFAULT`, the
    /// time of the save.
    Keyword,
    /// A quoted value.
    Text,
    Number,
}

/// A stretch of a statement's line in one colour.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Piece {
    pub ink: Ink,
    pub text: String,
}

/// One line of the review. Every text in it is on one line and holds no
/// hidden character, but a whole value in a [`Line::Sql`] (see [`Values`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Line {
    /// `-- row id 2`: the row the statement under it is of, by its key.
    Row(String),
    /// `-- only if kind is still 'print' and alt_text is still NULL`: the
    /// check a save makes before the statement, as each changed column and
    /// what it loaded.
    Check(Vec<(String, String)>),
    /// `-- row id 4 · blocked: fix publisher_id first`: a row with cells to
    /// fix has no statement until they are.
    Blocked { row: String, columns: Vec<String> },
    /// The builder refuses a value of the row, in its own words.
    Refused { row: String, reason: String },
    /// No statement can be made of the set: the table's key is not known.
    Unsendable,
    /// `-- new row`: the row the `INSERT` under it adds.
    New,
    /// `-- new row · blocked: fix kind first`.
    NewBlocked(Vec<String>),
    /// `-- new row · blocked: publisher_id is required`.
    NewRequired(Vec<String>),
    /// The builder refuses a value of the new row, in its own words.
    NewRefused(String),
    /// A line of a statement.
    Sql(Vec<Piece>),
}

impl Line {
    /// The text of a statement's line. `None` for a comment, which a view
    /// words.
    pub fn sql(&self) -> Option<String> {
        match self {
            Self::Sql(pieces) => Some(pieces.iter().map(|piece| piece.text.as_str()).collect()),
            _ => None,
        }
    }
}

/// How a statement's values are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Values {
    /// As a person reads them: a long one cut, and every hidden character
    /// (a line break too) written out, so no value passes for a line of
    /// its own.
    Shown,
    /// As the statement holds them: for the clipboard.
    Whole,
}

/// A part of a change set: a new row or a changed one, by its place among
/// its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Insert(usize),
    Row(usize),
}

/// What holds a new row's statement back, by its columns' names.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stuck {
    pub to_fix: Vec<String>,
    /// The required columns nothing is set in.
    pub required: Vec<String>,
}

/// What holds statements back, for each part of a set in its order. A part
/// with nothing listed has its statement.
#[derive(Debug, Clone, Copy, Default)]
pub struct Blocked<'a> {
    pub inserts: &'a [Stuck],
    /// The columns to fix of each changed row.
    pub rows: &'a [Vec<String>],
}

/// What a save of a tab's pending changes would run.
#[derive(Clone, PartialEq)]
pub struct Review {
    /// How many cells the set changes, and in how many rows.
    pub changes: usize,
    pub rows: usize,
    /// How many rows the set adds.
    pub added: usize,
    pub lines: Vec<Line>,
    /// The first part whose statement the builder refused, and why.
    pub refused: Option<(Part, Error)>,
}

/// Without the lines: they hold what the user typed, which stays out of
/// logs and panics.
impl std::fmt::Debug for Review {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Review {{ changes: {}, rows: {}, added: {}, lines: {} }}",
            self.changes,
            self.rows,
            self.added,
            self.lines.len()
        )
    }
}

/// The review of the table's new rows and the pending `cells`. `None`
/// when nothing is pending.
pub fn build(
    object: &ObjectRef,
    table: &Table<'_>,
    cells: &BTreeMap<(usize, usize), Pending>,
    values: Values,
) -> Option<Review> {
    if cells.is_empty() && table.added.is_empty() {
        return None;
    }
    let Some((changes, sent)) = change_set(object, table, cells) else {
        // The map is ordered by row: each row's cells are together. The
        // changes are the page's rows', as everywhere they are counted.
        let mut rows: Vec<usize> = cells
            .range(..(NEW_ROWS, 0))
            .map(|(&(row, _), _)| row)
            .collect();
        let changes = rows.len();
        rows.dedup();
        return Some(Review {
            changes,
            rows: rows.len(),
            added: table.added.len(),
            lines: vec![Line::Unsendable],
            refused: None,
        });
    };
    let name = |col: usize| {
        let column = table.page.columns.get(col)?;
        Some(format::display_safe(&column.name).into_owned())
    };
    // The columns to fix of a row, a page's or a new one's.
    let to_fix = |row: usize| -> Vec<String> {
        cells
            .range((row, 0)..=(row, usize::MAX))
            .filter(|(_, cell)| matches!(cell.state, State::ToFix(_)))
            .filter_map(|(&(_, col), _)| name(col))
            .collect()
    };
    let inserts: Vec<Stuck> = sent
        .inserts
        .iter()
        .map(|&id| Stuck {
            to_fix: to_fix(new_row(id)),
            required: table
                .missing(id, cells)
                .into_iter()
                .filter_map(name)
                .collect(),
        })
        .collect();
    let rows: Vec<Vec<String>> = sent.rows.iter().map(|&row| to_fix(row)).collect();
    let blocked = Blocked {
        inserts: &inserts,
        rows: &rows,
    };
    Some(of(table.dialect, &changes, blocked, values))
}

/// The review of `changes`, as `dialect` writes them: the new rows first,
/// as a save writes them first. A part `blocked` lists something for is a
/// comment only.
pub fn of(dialect: Dialect, changes: &ChangeSet, blocked: Blocked<'_>, values: Values) -> Review {
    let mut lines = Vec::new();
    let mut refused = None;
    for (index, insert) in changes.inserts.iter().enumerate() {
        let stuck = blocked.inserts.get(index);
        if let Some(stuck) = stuck.filter(|stuck| !stuck.to_fix.is_empty()) {
            lines.push(Line::NewBlocked(stuck.to_fix.clone()));
            continue;
        }
        if let Some(stuck) = stuck.filter(|stuck| !stuck.required.is_empty()) {
            lines.push(Line::NewRequired(stuck.required.clone()));
            continue;
        }
        match dialect.insert_row(&changes.object, insert) {
            Ok(statement) => {
                lines.push(Line::New);
                lines.extend(new_statement(&statement, values));
            }
            Err(error) => {
                let reason = format::capped(&error.to_string()).into_owned();
                lines.push(Line::NewRefused(
                    format::escape_hidden(&reason).into_owned(),
                ));
                refused.get_or_insert((Part::Insert(index), error));
            }
        }
    }
    for (index, row) in changes.rows.iter().enumerate() {
        let name = row_name(row);
        let stuck = blocked.rows.get(index);
        if let Some(columns) = stuck.filter(|columns| !columns.is_empty()) {
            lines.push(Line::Blocked {
                row: name,
                columns: columns.clone(),
            });
            continue;
        }
        match dialect.update_row(&changes.object, row) {
            Ok(update) => {
                lines.push(Line::Row(name));
                let check = row.set.iter().map(|change| {
                    let column = format::display_safe(&change.column).into_owned();
                    (column, loaded(&change.loaded, values))
                });
                lines.push(Line::Check(check.collect()));
                lines.extend(statement(&update, row.set.len(), values));
            }
            Err(error) => {
                let reason = format::capped(&error.to_string()).into_owned();
                lines.push(Line::Refused {
                    row: name,
                    reason: format::escape_hidden(&reason).into_owned(),
                });
                refused.get_or_insert((Part::Row(index), error));
            }
        }
    }
    Review {
        changes: changes.rows.iter().map(|row| row.set.len()).sum(),
        rows: changes.rows.len(),
        added: changes.inserts.len(),
        lines,
        refused,
    }
}

/// A row by its key, as the row panel's title names one: `id 2`, several
/// columns joined by a comma. From the key the statement finds the row by.
fn row_name(row: &RowChange) -> String {
    let parts: Vec<String> = row
        .key
        .iter()
        .map(|(column, value)| {
            format!(
                "{} {}",
                format::display_safe(column),
                format::cell_text(value)
            )
        })
        .collect();
    parts.join(", ")
}

/// What a changed cell loaded, as the check's comment says it: text in
/// quotes, everything else as the grid shows it. Shown, a long text is cut
/// to 59 characters and `…` inside its quotes; for the clipboard it is
/// whole, since the copy says each statement runs only while its row is
/// still as this comment says. Never a line break: a comment ends at one,
/// and what followed would be read as SQL by whoever pastes the text.
fn loaded(value: &Value, values: Values) -> String {
    let Value::Text(text) = value else {
        return format::cell_text(value).into_owned();
    };
    if values == Values::Whole {
        return format!("'{}'", format::escape_hidden(text));
    }
    let (kept, whole) = head(text, VALUE_MAX_CHARS);
    if whole {
        return format!("'{}'", format::escape_hidden(kept));
    }
    let (kept, _) = head(text, VALUE_MAX_CHARS - 1);
    format!("'{}…'", format::escape_hidden(kept))
}

/// The start of `text` that takes at most `room` characters once every
/// hidden character in it is written out (a line break is eight), and
/// whether that is all of `text`. Counted as it is shown, so a value of
/// line breaks is cut to what fits and not to sixty times eight.
fn head(text: &str, room: usize) -> (&str, bool) {
    let mut taken = 0;
    for (at, character) in text.char_indices() {
        let end = at + character.len_utf8();
        taken += format::escape_hidden(&text[at..end]).chars().count();
        if taken > room {
            return (&text[..at], false);
        }
    }
    (text, true)
}

/// The statement in lines, as the design lays one out: the table, each
/// value that is set, each column of the key. Only white space between
/// its words is changed, so the lines read as the statement that runs.
fn statement(update: &RowUpdate, set: usize, values: Values) -> Vec<Line> {
    lay(update, set, values).unwrap_or_else(|| {
        // Not the parts this was written for: the statement on one line.
        vec![Line::Sql(vec![
            plain(&update.shown, values),
            fixed(Ink::Plain, ";"),
        ])]
    })
}

fn lay(update: &RowUpdate, set: usize, values: Values) -> Option<Vec<Line>> {
    let RowUpdate { shown, parts, .. } = update;
    let last = parts.values.len().checked_sub(1)?;
    if set == 0 || set > last {
        return None;
    }
    let table = shown.get(..parts.set)?.strip_prefix("UPDATE")?.trim_end();
    let mut lines = vec![Line::Sql(vec![
        fixed(Ink::Keyword, "UPDATE"),
        plain(table, values),
    ])];
    if parts.columns.len() != parts.values.len() {
        return None;
    }
    for (index, (column, range)) in parts.columns.iter().zip(&parts.values).enumerate() {
        // What stands before the value: its column and `=`.
        let lead = shown.get(*column..range.start)?;
        let literal = shown.get(range.clone())?;
        // The keywords end in one column, as the design sets them.
        let mut pieces = match index {
            0 => vec![fixed(Ink::Plain, "   "), fixed(Ink::Keyword, "SET")],
            _ if index < set => vec![fixed(Ink::Plain, "      ")],
            _ if index == set => vec![fixed(Ink::Plain, " "), fixed(Ink::Keyword, "WHERE")],
            _ => vec![fixed(Ink::Plain, "   "), fixed(Ink::Keyword, "AND")],
        };
        pieces.push(plain(&format!(" {lead}"), values));
        pieces.push(value(literal, values));
        if index + 1 < set {
            pieces.push(fixed(Ink::Plain, ","));
        } else if index == last {
            pieces.push(fixed(Ink::Plain, ";"));
        }
        lines.push(Line::Sql(pieces));
    }
    Some(lines)
}

/// A new row's statement in lines, as the design lays one out: the table
/// and its columns, the values, and what it hands back. Only white space
/// between its clauses is changed.
fn new_statement(statement: &InsertStatement, values: Values) -> Vec<Line> {
    lay_new(statement, values).unwrap_or_else(|| {
        // Not the parts this was written for: the statement on one line.
        vec![Line::Sql(vec![
            plain(&statement.shown, values),
            fixed(Ink::Plain, ";"),
        ])]
    })
}

fn lay_new(statement: &InsertStatement, values: Values) -> Option<Vec<Line>> {
    let InsertStatement { shown, parts, .. } = statement;
    let into = shown.get(..parts.values)?.strip_prefix("INSERT INTO")?;
    let mut lines = vec![Line::Sql(vec![
        fixed(Ink::Keyword, "INSERT INTO"),
        plain(into.trim_end(), values),
    ])];
    // The clause ends a space before `RETURNING`, or with the statement.
    let end = match parts.back {
        Some(back) => back.checked_sub(1)?,
        None => shown.len(),
    };
    let clause = shown.get(parts.values..end)?;
    let word = ["DEFAULT VALUES", "VALUES"]
        .into_iter()
        .find(|word| clause.starts_with(word))?;
    let mut pieces = vec![fixed(Ink::Keyword, word)];
    let mut at = parts.values + word.len();
    for range in &parts.literals {
        pieces.push(plain(shown.get(at..range.start)?, values));
        pieces.push(value(shown.get(range.clone())?, values));
        at = range.end;
    }
    let rest = shown.get(at..end)?;
    if !rest.is_empty() {
        pieces.push(plain(rest, values));
    }
    let Some(back) = parts.back else {
        pieces.push(fixed(Ink::Plain, ";"));
        lines.push(Line::Sql(pieces));
        return Some(lines);
    };
    lines.push(Line::Sql(pieces));
    let handed = shown.get(back..)?.strip_prefix("RETURNING")?;
    lines.push(Line::Sql(vec![
        fixed(Ink::Keyword, "RETURNING"),
        plain(handed, values),
        fixed(Ink::Plain, ";"),
    ]));
    Some(lines)
}

/// A piece of the review's own writing.
fn fixed(ink: Ink, text: &str) -> Piece {
    Piece {
        ink,
        text: text.to_owned(),
    }
}

/// A piece of the statement that is no value: a name, an `=`.
fn plain(text: &str, values: Values) -> Piece {
    let text = match values {
        Values::Shown => format::escape_hidden(text).into_owned(),
        Values::Whole => text.to_owned(),
    };
    Piece {
        ink: Ink::Plain,
        text,
    }
}

/// A value of the statement, by the literal the builder wrote.
fn value(literal: &str, values: Values) -> Piece {
    // What the database makes itself is written as its word, not quoted.
    let keyword =
        matches!(literal, "NULL" | "DEFAULT" | "now()") || literal.starts_with("CURRENT_");
    let ink = if keyword {
        Ink::Keyword
    } else if literal.starts_with(|first: char| first.is_ascii_digit() || first == '-') {
        Ink::Number
    } else {
        Ink::Text
    };
    let text = match values {
        Values::Shown => shortened(literal),
        Values::Whole => literal.to_owned(),
    };
    Piece { ink, text }
}

/// What joins two strings in SQLite's text around a NUL, as the builder
/// writes it.
const JOINED: &str = " || char(0) || ";

/// SQLite's text around a NUL (`('a' || char(0) || 'b')`), which is longer
/// than is shown, cut so that it still reads as one value: in a string, to
/// `…')`, the string closed and then the bracket; between two strings, to
/// `…)`, and never inside what joins them, where half a `char(0)` would
/// read as nothing at all. `None` for a literal that is not of that form.
/// The form is the builder's own, and is walked as it writes it: a string
/// in quotes, a quote inside it written twice.
fn shortened_joined(literal: &str) -> Option<String> {
    let strings = literal.strip_prefix('(')?.strip_suffix(')')?;
    if !strings.starts_with('\'') {
        return None;
    }
    // What the longer of the two endings leaves.
    let (kept, _) = head(literal, VALUE_MAX_CHARS - 1 - "')".len());
    let cut = kept.len();
    // Each string in turn: where its text begins, after its quote, and
    // where the string ends, after the quote that closes it.
    let mut begins = "('".len();
    loop {
        let mut ends = begins;
        loop {
            let rest = literal.get(ends..)?;
            if rest.starts_with("''") {
                ends += 2;
            } else if rest.starts_with('\'') {
                ends += 1;
                break;
            } else {
                ends += rest.chars().next()?.len_utf8();
            }
        }
        if cut < ends {
            // In the string. A quote in it is written twice: half of one
            // before the `…` would close the string there.
            let text = literal.get(begins..cut)?;
            let quotes = text.chars().rev().take_while(|&last| last == '\'').count();
            let kept = literal.get(..cut - quotes % 2)?;
            return Some(format!("{}…')", format::escape_hidden(kept)));
        }
        let joined = ends + JOINED.len();
        if cut <= joined || !literal.get(ends..)?.starts_with(JOINED) {
            // After the string: all of what joins it to the next, or
            // none of it.
            let kept = literal.get(..if cut == joined { joined } else { ends })?;
            return Some(format!("{}…)", format::escape_hidden(kept)));
        }
        begins = joined + 1;
        if !literal.get(joined..)?.starts_with('\'') {
            return None;
        }
    }
}

/// A literal as it is shown, its hidden characters written out: one that
/// would take more than `VALUE_MAX_CHARS` characters keeps its beginning,
/// then `…`, then what closes it (`'`, or the bracket of SQLite's text
/// around a NUL, see [`shortened_joined`]), so it still reads as one value.
fn shortened(literal: &str) -> String {
    let (kept, whole) = head(literal, VALUE_MAX_CHARS);
    if whole {
        return format::escape_hidden(kept).into_owned();
    }
    if let Some(shown) = shortened_joined(literal) {
        return shown;
    }
    let open = ["E'", "x'", "('", "'"]
        .into_iter()
        .find(|open| literal.starts_with(open))
        .unwrap_or("");
    let close = if literal.ends_with("')") {
        "')"
    } else if literal.ends_with('\'') {
        "'"
    } else {
        ""
    };
    let (mut kept, _) = head(literal, VALUE_MAX_CHARS - 1 - close.len());
    // A quote in a value is written twice, and so is a backslash where it
    // escapes: half of such a pair before the `…` would read as the
    // value's end, or as an escape of what follows. The other half goes
    // too. So does the backslash of MySQL's `\0` for a NUL, cut from its
    // zero: it is the odd one of its run.
    let inside = kept.get(open.len()..).unwrap_or("");
    for mark in ['\'', '\\'] {
        let run = inside
            .chars()
            .rev()
            .take_while(|&last| last == mark)
            .count();
        if run % 2 == 1 {
            kept = &kept[..kept.len() - mark.len_utf8()];
            break;
        }
    }
    format!("{}…{close}", format::escape_hidden(kept))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::Problem;
    use crate::testing::{fixture_structure, page};
    use std::collections::BTreeSet;
    use tabletist_db::{Access, CellChange, NewValue, ObjectKind, RowPage, Structure};

    fn users() -> ObjectRef {
        ObjectRef::new("main", "users")
    }

    /// No row is gone.
    static NONE_GONE: BTreeSet<usize> = BTreeSet::new();

    fn table<'a>(structure: &'a Structure, page: &'a RowPage) -> Table<'a> {
        Table {
            access: Access::Writable,
            kind: ObjectKind::Table,
            dialect: Dialect::Sqlite,
            structure: Some(structure),
            page,
            refreshing: false,
            saving: false,
            gone: &NONE_GONE,
            added: &[],
        }
    }

    fn ready(text: &str) -> Pending {
        Pending {
            new: NewValue::Text(text.into()),
            state: State::Ready,
        }
    }

    fn cells(list: Vec<((usize, usize), Pending)>) -> BTreeMap<(usize, usize), Pending> {
        list.into_iter().collect()
    }

    /// The statements' lines, as text.
    fn sql(review: &Review) -> Vec<String> {
        review.lines.iter().filter_map(Line::sql).collect()
    }

    /// The statements of `review` with the layout's white space taken back:
    /// each as the builder wrote it, and its semicolon.
    fn unlaid(review: &Review) -> Vec<String> {
        let mut statements: Vec<String> = Vec::new();
        for line in sql(review) {
            match statements.last_mut() {
                Some(statement) if !line.starts_with("UPDATE") && !line.starts_with("INSERT") => {
                    statement.push(' ');
                    statement.push_str(line.trim_start_matches(' '));
                }
                _ => statements.push(line),
            }
        }
        statements
    }

    /// The Bookshop's covers with `count` new rows at the top, and their ids.
    fn covers(count: usize) -> (Structure, RowPage, crate::edit::Edits, Vec<usize>) {
        let mut edits = crate::edit::Edits::default();
        let ids = (0..count)
            .map(|_| edits.add_row(crate::edit::Place::Top, 3))
            .collect();
        (
            crate::testing::book_covers_structure(),
            crate::testing::book_covers_page(3),
            edits,
            ids,
        )
    }

    fn covers_ref() -> ObjectRef {
        ObjectRef::new("main", "book_covers")
    }

    #[test]
    fn a_new_row_sends_only_what_was_set() {
        let (structure, page, mut edits, ids) = covers(2);
        edits.put((new_row(ids[0]), 1), ready("9100000000000000004"));
        edits.put((new_row(ids[1]), 1), ready("9100000000000000007"));
        edits.put((new_row(ids[1]), 2), ready("ebook"));
        // And a changed row, to come after them.
        edits.put((1, 2), ready("audio"));
        let table = Table {
            added: &edits.added,
            ..table(&structure, &page)
        };
        let (changes, sent) = change_set(&covers_ref(), &table, &edits.cells).unwrap();
        assert_eq!(sent.inserts, ids);
        assert_eq!(sent.rows, [1]);
        let named: Vec<Vec<&str>> = changes
            .inserts
            .iter()
            .map(|insert| {
                let set = insert.set.iter();
                set.map(|value| value.column.as_str()).collect()
            })
            .collect();
        assert_eq!(named, [vec!["publisher_id"], vec!["publisher_id", "kind"]]);
        assert_eq!(changes.rows.len(), 1);
        let review = build(&covers_ref(), &table, &edits.cells, Values::Shown).unwrap();
        assert_eq!((review.added, review.changes, review.rows), (2, 1, 1));
        assert_eq!(review.refused, None);
        assert_eq!(review.lines[0], Line::New);
        assert_eq!(
            sql(&review)[..6],
            [
                r#"INSERT INTO "main"."book_covers" ("publisher_id")"#,
                "VALUES (9100000000000000004)",
                "RETURNING *;",
                r#"INSERT INTO "main"."book_covers" ("publisher_id", "kind")"#,
                "VALUES (9100000000000000007, 'ebook')",
                "RETURNING *;",
            ]
        );
        // The changed row's statement follows, as it reads today.
        assert_eq!(sql(&review)[6], r#"UPDATE "main"."book_covers""#);
        // Taken back out of its lines, each statement is the builder's own.
        let first = Dialect::Sqlite
            .insert_row(&covers_ref(), &changes.inserts[0])
            .unwrap();
        assert_eq!(unlaid(&review)[0], format!("{};", first.shown));
        assert_eq!(unlaid(&review).len(), 3);
    }

    #[test]
    fn a_new_row_with_nothing_set_takes_every_default() {
        let (mut structure, page, edits, _) = covers(1);
        // Nothing is required of this one.
        structure.columns[1].nullable = true;
        let table = Table {
            added: &edits.added,
            ..table(&structure, &page)
        };
        let review = build(&covers_ref(), &table, &edits.cells, Values::Shown).unwrap();
        assert_eq!((review.added, review.changes, review.rows), (1, 0, 0));
        assert_eq!(
            sql(&review),
            [
                r#"INSERT INTO "main"."book_covers""#,
                "DEFAULT VALUES",
                "RETURNING *;"
            ]
        );
        // MySQL has no such clause, and hands nothing back.
        let (changes, _) = change_set(&covers_ref(), &table, &edits.cells).unwrap();
        let mysql = of(Dialect::MySql, &changes, Blocked::default(), Values::Shown);
        assert_eq!(
            sql(&mysql),
            ["INSERT INTO `main`.`book_covers` ()", "VALUES ();"]
        );
    }

    #[test]
    fn a_long_value_of_a_new_row_is_cut_where_it_is_shown() {
        let (structure, page, mut edits, ids) = covers(1);
        edits.put((new_row(ids[0]), 1), ready("9100000000000000004"));
        let long = "x".repeat(VALUE_MAX_CHARS * 3);
        edits.put((new_row(ids[0]), 4), ready(&long));
        let table = Table {
            added: &edits.added,
            ..table(&structure, &page)
        };
        let shown = build(&covers_ref(), &table, &edits.cells, Values::Shown).unwrap();
        let values = &sql(&shown)[1];
        assert!(
            values.starts_with("VALUES (9100000000000000004, '"),
            "{values}"
        );
        assert!(!values.contains(&long) && values.contains('…'), "{values}");
        // For the clipboard it is whole.
        let whole = build(&covers_ref(), &table, &edits.cells, Values::Whole).unwrap();
        assert!(sql(&whole)[1].contains(&long));
    }

    #[test]
    fn a_new_row_that_cannot_be_sent_is_a_comment() {
        let (structure, page, mut edits, ids) = covers(2);
        // The first lacks its publisher; the second has a kind to fix.
        edits.put((new_row(ids[1]), 1), ready("9100000000000000004"));
        edits.put(
            (new_row(ids[1]), 2),
            Pending {
                new: NewValue::Text("vinyl".into()),
                state: State::ToFix(Problem::NotOneOf(vec!["print".into()])),
            },
        );
        let table = Table {
            added: &edits.added,
            ..table(&structure, &page)
        };
        let review = build(&covers_ref(), &table, &edits.cells, Values::Shown).unwrap();
        assert_eq!(
            review.lines,
            [
                Line::NewRequired(vec!["publisher_id".into()]),
                Line::NewBlocked(vec!["kind".into()]),
            ]
        );
        assert!(sql(&review).is_empty());
    }

    #[test]
    fn a_table_without_a_key_sends_its_new_rows_and_no_changed_one() {
        let (mut structure, page, mut edits, ids) = covers(1);
        structure.primary_key.clear();
        edits.put((new_row(ids[0]), 1), ready("9100000000000000004"));
        {
            let table = Table {
                added: &edits.added,
                ..table(&structure, &page)
            };
            let (changes, _) = change_set(&covers_ref(), &table, &edits.cells).unwrap();
            assert_eq!((changes.inserts.len(), changes.rows.len()), (1, 0));
        }
        // A changed cell there is found by no key: nothing can be sent.
        edits.put((0, 2), ready("audio"));
        let table = Table {
            added: &edits.added,
            ..table(&structure, &page)
        };
        assert!(change_set(&covers_ref(), &table, &edits.cells).is_none());
        let review = build(&covers_ref(), &table, &edits.cells, Values::Shown).unwrap();
        assert_eq!(review.lines, [Line::Unsendable]);
        assert_eq!((review.added, review.changes, review.rows), (1, 1, 1));
    }

    #[test]
    fn a_keyword_is_inked_as_one() {
        for literal in [
            "NULL",
            "DEFAULT",
            "now()",
            "CURRENT_TIMESTAMP",
            "CURRENT_TIMESTAMP(6)",
            "CURRENT_DATE",
        ] {
            assert_eq!(value(literal, Values::Shown).ink, Ink::Keyword, "{literal}");
        }
        // A default's expression on SQLite, and any value, is not.
        assert_eq!(value("('print')", Values::Shown).ink, Ink::Text);
        assert_eq!(value("'DEFAULT'", Values::Shown).ink, Ink::Text);
        assert_eq!(value("42", Values::Shown).ink, Ink::Number);
    }

    #[test]
    fn a_changed_row_is_its_check_and_its_statement_in_lines() {
        let (structure, page) = (fixture_structure(), page(5, false));
        let set = cells(vec![
            ((1, 1), ready("bob@example.com")),
            ((1, 2), ready("{}")),
        ]);
        let review = build(&users(), &table(&structure, &page), &set, Values::Shown).unwrap();
        assert_eq!((review.changes, review.rows), (2, 1));
        assert_eq!(review.refused, None);
        assert_eq!(review.lines[0], Line::Row("id 2".into()));
        // What a save compares before it writes: what the page loaded.
        assert_eq!(
            review.lines[1],
            Line::Check(vec![
                ("email".into(), "'user2@example.com'".into()),
                ("meta".into(), "NULL".into()),
            ])
        );
        assert_eq!(
            sql(&review),
            [
                r#"UPDATE "main"."users""#,
                r#"   SET "email" = 'bob@example.com',"#,
                r#"       "meta" = '{}'"#,
                r#" WHERE "id" = 2;"#,
            ]
        );
        assert_eq!(review.lines.len(), 6);
        // What each piece is drawn as.
        let inks = |line: &Line| match line {
            Line::Sql(pieces) => pieces.iter().map(|piece| piece.ink).collect::<Vec<_>>(),
            other => panic!("{other:?} is no statement's line"),
        };
        assert_eq!(inks(&review.lines[2]), [Ink::Keyword, Ink::Plain]);
        assert_eq!(
            inks(&review.lines[3]),
            [Ink::Plain, Ink::Keyword, Ink::Plain, Ink::Text, Ink::Plain]
        );
        assert_eq!(
            inks(&review.lines[5]),
            [
                Ink::Plain,
                Ink::Keyword,
                Ink::Plain,
                Ink::Number,
                Ink::Plain
            ]
        );
        // The lines are the statement that runs, and nothing else.
        let whole = build(&users(), &table(&structure, &page), &set, Values::Whole).unwrap();
        assert_eq!(
            unlaid(&whole),
            [
                r#"UPDATE "main"."users" SET "email" = 'bob@example.com', "meta" = '{}' WHERE "id" = 2;"#
            ]
        );
        // NULL is a keyword, as a value and in the check.
        let null = cells(vec![(
            (0, 2),
            Pending {
                new: NewValue::Null,
                state: State::Ready,
            },
        )]);
        let review = build(&users(), &table(&structure, &page), &null, Values::Shown).unwrap();
        assert_eq!(
            review.lines[1],
            Line::Check(vec![("meta".into(), r#"'{"plan":"pro"}'"#.into())])
        );
        assert_eq!(sql(&review)[1], r#"   SET "meta" = NULL"#);
        assert_eq!(inks(&review.lines[3])[3], Ink::Keyword);
    }

    #[test]
    fn a_row_with_a_cell_to_fix_is_a_comment_only() {
        let (structure, page) = (fixture_structure(), page(5, false));
        let set = cells(vec![
            ((1, 1), ready("bob@example.com")),
            ((3, 1), ready("dan@example.com")),
            (
                (3, 2),
                Pending {
                    new: NewValue::Text("{oops".into()),
                    state: State::ToFix(Problem::WholeNumber),
                },
            ),
        ]);
        let review = build(&users(), &table(&structure, &page), &set, Values::Shown).unwrap();
        // Every pending cell counts, as the bar counts them.
        assert_eq!((review.changes, review.rows), (3, 2));
        assert_eq!(
            review.lines.last(),
            Some(&Line::Blocked {
                row: "id 4".into(),
                columns: vec!["meta".into()],
            })
        );
        // The other row has its statement, and the blocked one none.
        assert_eq!(unlaid(&review).len(), 1);
        assert!(sql(&review).iter().all(|line| !line.contains("dan@")));
        assert_eq!(review.refused, None);
    }

    #[test]
    fn a_long_value_is_cut_where_it_is_shown_and_whole_where_it_is_copied() {
        let structure = fixture_structure();
        let mut page = page(5, false);
        page.rows[1][1] = Value::Text("o".repeat(200).into());
        let new = "x".repeat(100);
        let set = cells(vec![((1, 1), ready(&new))]);
        let shown = build(&users(), &table(&structure, &page), &set, Values::Shown).unwrap();
        let line = &sql(&shown)[1];
        let literal = line.strip_prefix(r#"   SET "email" = "#).unwrap();
        assert_eq!(literal.chars().count(), VALUE_MAX_CHARS);
        assert_eq!(literal, format!("'{}…'", "x".repeat(57)));
        // The check's value is cut the same way, inside its quotes.
        assert_eq!(
            shown.lines[1],
            Line::Check(vec![("email".into(), format!("'{}…'", "o".repeat(59)))])
        );
        // Sixty characters are shown whole: the quotes are two of them.
        let fits = cells(vec![((1, 1), ready(&"y".repeat(58)))]);
        let review = build(&users(), &table(&structure, &page), &fits, Values::Shown).unwrap();
        assert_eq!(
            sql(&review)[1],
            format!(r#"   SET "email" = '{}'"#, "y".repeat(58))
        );
        // The clipboard gets the statement that runs.
        let whole = build(&users(), &table(&structure, &page), &set, Values::Whole).unwrap();
        assert_eq!(
            unlaid(&whole),
            [format!(
                r#"UPDATE "main"."users" SET "email" = '{new}' WHERE "id" = 2;"#
            )]
        );
        // And the whole of what its check compares: the copy says each
        // statement runs only while its row is still as the comment says.
        assert_eq!(
            whole.lines[1],
            Line::Check(vec![("email".into(), format!("'{}'", "o".repeat(200)))])
        );
    }

    #[test]
    fn a_cut_value_still_reads_as_one_value() {
        let cut = |literal: &str| shortened(literal);
        let count = |text: &str| text.chars().count();
        // Sixty characters at most, however the value is written.
        let long = "x".repeat(100);
        for literal in [
            format!("'{long}'"),
            format!("E'{long}'"),
            format!("x'{long}'"),
        ] {
            let shown = cut(&literal);
            assert_eq!(count(&shown), VALUE_MAX_CHARS, "{shown}");
            assert!(shown.ends_with("…'"), "{shown}");
        }
        // A quote in a value is written twice. Cut between the two, the
        // first would close the value before the `…`: it goes as well.
        let quoted = format!("'{}''{long}'", "x".repeat(56));
        assert_eq!(cut(&quoted), format!("'{}…'", "x".repeat(56)));
        // Both halves kept, the pair stays.
        let quoted = format!("'{}''{long}'", "x".repeat(55));
        assert_eq!(cut(&quoted), format!("'{}''…'", "x".repeat(55)));
        // The value's own opening quote is no half of a pair.
        let quotes = format!("'{}'", "''".repeat(40));
        assert_eq!(cut(&quotes), format!("'{}…'", "''".repeat(28)));
        // So with a backslash where it is written twice.
        let slashed = format!("E'{}\\\\{long}'", "x".repeat(55));
        assert_eq!(cut(&slashed), format!("E'{}…'", "x".repeat(55)));
        // MySQL writes a NUL as a backslash and a zero: the cut never
        // falls between the two, where the backslash would be left to
        // escape what follows it.
        let nul = format!("'{}\\0{long}'", "x".repeat(56));
        assert_eq!(cut(&nul), format!("'{}…'", "x".repeat(56)));
        let nul = format!("'{}\\0{long}'", "x".repeat(55));
        assert_eq!(cut(&nul), format!("'{}\\0…'", "x".repeat(55)));
        // After a backslash of the value's own, which is written twice.
        let nul = format!("'{}\\\\\\0{long}'", "x".repeat(54));
        assert_eq!(cut(&nul), format!("'{}\\\\…'", "x".repeat(54)));
        // SQLite's text around a NUL keeps its brackets.
        let nul = format!("('{long}' || char(0) || 'b')");
        assert_eq!(cut(&nul), format!("('{}…')", "x".repeat(55)));
        // Hidden characters are counted as they are shown: a value of
        // line breaks is sixty characters too, and no marker is cut in
        // two.
        let breaks = format!("'{}'", "\n".repeat(58));
        let shown = cut(&breaks);
        assert_eq!(shown, format!("'{}…'", "<U+000A>".repeat(7)));
        assert!(count(&shown) <= VALUE_MAX_CHARS);
        // What fits is shown whole, with what it hides written out.
        assert_eq!(cut("'a\nb'"), "'a<U+000A>b'");
        assert_eq!(cut("NULL"), "NULL");
        // The check's value: fifty-nine characters and `…` in its quotes.
        let shown = |value: &Value| loaded(value, Values::Shown);
        let text = Value::Text("o".repeat(200).into());
        assert_eq!(shown(&text), format!("'{}…'", "o".repeat(59)));
        let fits = Value::Text("o".repeat(60).into());
        assert_eq!(shown(&fits), format!("'{}'", "o".repeat(60)));
        let breaks = Value::Text("\n".repeat(58).into());
        assert_eq!(shown(&breaks), format!("'{}…'", "<U+000A>".repeat(7)));
        // For the clipboard the check says the whole of what was loaded,
        // as the statement under it holds the whole of what is set: still
        // on one line, with no break to end the comment.
        let whole = |value: &Value| loaded(value, Values::Whole);
        assert_eq!(whole(&text), format!("'{}'", "o".repeat(200)));
        assert_eq!(whole(&breaks), format!("'{}'", "<U+000A>".repeat(58)));
        assert_eq!(whole(&Value::Null), shown(&Value::Null));
        assert_eq!(whole(&Value::Int(612)), "612");
    }

    #[test]
    fn sqlite_text_around_a_nul_is_cut_where_it_still_balances() {
        let count = |text: &str| text.chars().count();
        // Whether what is shown reads as one value: every string closed,
        // and the bracket with them.
        let balanced = |shown: &str| {
            let (mut inside, mut depth) = (false, 0);
            for character in shown.chars() {
                match character {
                    '\'' => inside = !inside,
                    '(' if !inside => depth += 1,
                    ')' if !inside => depth -= 1,
                    _ => {}
                }
            }
            !inside && depth == 0
        };
        let long = "y".repeat(100);
        let around = |first: &str| format!("('{first}' || char(0) || '{long}')");
        let cut = |first: &str| shortened(&around(first));
        let x = |times: usize| "x".repeat(times);
        // In the first string: closed by its quote, then the bracket.
        assert_eq!(cut(&x(55)), format!("('{}…')", x(55)));
        // Anywhere in what joins the two, the cut goes back to the end of
        // the string before it: half a `char(0)` reads as nothing.
        for kept in 40..54 {
            assert_eq!(cut(&x(kept)), format!("('{}'…)", x(kept)), "{kept}");
        }
        // Where the first string ends, the string is closed already.
        assert_eq!(cut(&x(54)), format!("('{}'…)", x(54)));
        // After the whole of it, it stays.
        assert_eq!(cut(&x(39)), format!("('{}' || char(0) || …)", x(39)));
        // In the second string: from its own quote on, and that quote is
        // no half of a pair.
        assert_eq!(cut(&x(38)), format!("('{}' || char(0) || '…')", x(38)));
        assert_eq!(cut(&x(37)), format!("('{}' || char(0) || 'y…')", x(37)));
        // A quote in a string is written twice, and is kept or dropped
        // whole.
        let quoted = format!("('{}' || char(0) || '''{long}')", x(37));
        assert_eq!(
            shortened(&quoted),
            format!("('{}' || char(0) || '…')", x(37))
        );
        let quoted = format!("('{}' || char(0) || '''{long}')", x(36));
        assert_eq!(
            shortened(&quoted),
            format!("('{}' || char(0) || '''…')", x(36))
        );
        // What joins two strings is no such thing inside one: there it is
        // the value's own text, quotes doubled, and is cut as text.
        let inside = format!("{}' || char(0) || '", x(45));
        let literal = Dialect::Sqlite
            .update_row(
                &users(),
                &RowChange {
                    key: vec![("id".into(), Value::Int(1))],
                    set: vec![CellChange {
                        column: "t".into(),
                        type_name: "TEXT".into(),
                        loaded: Value::Null,
                        new: NewValue::Text(format!("{inside}\0{long}")),
                    }],
                },
            )
            .unwrap();
        let shown = &literal.shown[literal.parts.values[0].clone()];
        assert_eq!(shortened(shown), format!("('{}'' || char…')", x(45)));
        // Wherever the cut lands, what is shown is one value of sixty
        // characters at most, and its end says there is more.
        for first in 0..70 {
            for literal in [
                around(&x(first)),
                around(&format!("{}''", x(first))),
                format!("('{}' || char(0) || '' || char(0) || '{long}')", x(first)),
                format!(
                    "('{}' || char(0) || '<\n>' || char(0) || '{long}')",
                    x(first)
                ),
            ] {
                let shown = shortened(&literal);
                assert!(balanced(&shown), "{first}: {shown}");
                assert!(count(&shown) <= VALUE_MAX_CHARS, "{first}: {shown}");
                assert!(
                    shown.ends_with("…')") || shown.ends_with("…)"),
                    "{first}: {shown}"
                );
                assert!(!shown.contains('\n'), "{first}: {shown}");
            }
        }
        // The other forms are cut as they were.
        assert_eq!(
            shortened(&format!("'{long}'")),
            format!("'{}…'", "y".repeat(57))
        );
    }

    #[test]
    fn no_value_ends_a_comment_or_passes_for_a_line() {
        let structure = fixture_structure();
        let mut page = page(5, false);
        page.rows[1][1] = Value::Text("a\nDROP TABLE users; --".into());
        let new = "b\n-- row id 9\nUPDATE users SET x = 1;\u{202E}";
        let set = cells(vec![((1, 1), ready(new))]);
        for values in [Values::Shown, Values::Whole] {
            let review = build(&users(), &table(&structure, &page), &set, values).unwrap();
            // A comment is one line, in what is shown and in what is
            // copied.
            assert_eq!(
                review.lines[1],
                Line::Check(vec![(
                    "email".into(),
                    "'a<U+000A>DROP TABLE users; --'".into()
                )]),
                "{values:?}"
            );
        }
        // Shown, the value is on its statement's line, with what it hides
        // written out, and cut as it is shown: sixty characters of it.
        let shown = build(&users(), &table(&structure, &page), &set, Values::Shown).unwrap();
        assert!(sql(&shown).iter().all(|line| !line.contains('\n')));
        assert_eq!(
            sql(&shown)[1],
            r#"   SET "email" = 'b<U+000A>-- row id 9<U+000A>UPDATE users SET x = 1;…'"#
        );
        assert_eq!(sql(&shown).len(), 3);
        // Copied, it is the value itself, inside its quotes.
        let whole = build(&users(), &table(&structure, &page), &set, Values::Whole).unwrap();
        assert_eq!(sql(&whole)[1], format!(r#"   SET "email" = '{new}'"#));
        // A name is shown as safely as a value.
        let row = RowChange {
            key: vec![("id\n".into(), Value::Text("k\n1".into()))],
            set: vec![CellChange {
                column: "na\nme".into(),
                type_name: "text".into(),
                loaded: Value::Null,
                new: NewValue::Text("v".into()),
            }],
        };
        let changes = ChangeSet {
            object: users(),
            inserts: Vec::new(),
            rows: vec![row],
        };
        let review = of(
            Dialect::Postgres,
            &changes,
            Blocked::default(),
            Values::Shown,
        );
        assert_eq!(review.lines[0], Line::Row("id<U+000A> k 1".into()));
        assert_eq!(
            review.lines[1],
            Line::Check(vec![("na<U+000A>me".into(), "NULL".into())])
        );
        assert!(sql(&review).iter().all(|line| !line.contains('\n')));
    }

    #[test]
    fn a_row_without_a_statement_is_one_line_whatever_its_columns_are_called() {
        let mut structure = fixture_structure();
        let mut page = page(5, false);
        for (index, name) in [(1, "e\nmail"), (2, "me\u{202E}ta")] {
            structure.columns[index].name = name.into();
            page.columns[index].name = name.into();
        }
        structure.columns[1].type_name = "INTEGER".into();
        let set = cells(vec![
            ((1, 1), ready("abc")),
            (
                (3, 2),
                Pending {
                    new: NewValue::Text("{oops".into()),
                    state: State::ToFix(Problem::WholeNumber),
                },
            ),
        ]);
        // The builder's words name the column it refuses, and a row to fix
        // names its own: neither ends its comment, shown or copied.
        for values in [Values::Shown, Values::Whole] {
            let review = build(&users(), &table(&structure, &page), &set, values).unwrap();
            assert_eq!(
                review.lines,
                [
                    Line::Refused {
                        row: "id 2".into(),
                        reason: "e<U+000A>mail: INTEGER expects a whole number".into(),
                    },
                    Line::Blocked {
                        row: "id 4".into(),
                        columns: vec!["me<U+202E>ta".into()],
                    },
                ],
                "{values:?}"
            );
        }
    }

    #[test]
    fn the_lines_are_the_statement_that_runs_whatever_it_holds() {
        // Names and values made of what a statement is written with, and
        // of what a line is shown without.
        let text = |column: &str, new: &str| CellChange {
            column: column.into(),
            type_name: "text".into(),
            loaded: Value::Null,
            new: NewValue::Text(new.into()),
        };
        let changes = ChangeSet {
            object: ObjectRef::new("a SET b", "c\u{202E} WHERE d"),
            inserts: Vec::new(),
            rows: vec![RowChange {
                key: vec![
                    ("id\n".into(), Value::Text("k'\n1".into())),
                    ("AND".into(), Value::Int(-2)),
                ],
                set: vec![
                    text("na\nme", "it's, SET = 'x' WHERE 1;\n-- row id 9"),
                    text("\"b\" = `c`", "caf\u{e9} \\ \u{1F600}\\"),
                    text("nul", "a\0b"),
                    text("quotes", &"'".repeat(70)),
                ],
            }],
        };
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            // All of it but the NUL where the database's text holds none:
            // such a row has no statement at all.
            let mut changes = changes.clone();
            if dialect == Dialect::Postgres {
                changes.rows[0].set.remove(2);
            }
            let built = dialect
                .update_row(&changes.object, &changes.rows[0])
                .unwrap();
            // Whole, the lines are the builder's text to the byte.
            let whole = of(dialect, &changes, Blocked::default(), Values::Whole);
            assert_eq!(unlaid(&whole), [format!("{};", built.shown)], "{dialect:?}");
            // Shown, they are the same lines, and each is one line.
            let shown = of(dialect, &changes, Blocked::default(), Values::Shown);
            assert_eq!(shown.lines.len(), whole.lines.len(), "{dialect:?}");
            assert!(
                sql(&shown)
                    .iter()
                    .all(|line| !line.contains(['\n', '\0', '\u{202E}'])),
                "{dialect:?}"
            );
        }
    }

    #[test]
    fn a_statement_the_builder_refuses_says_why_and_is_noted() {
        // A whole-number column with a text no check stopped.
        let mut structure = fixture_structure();
        structure.columns[1].type_name = "INTEGER".into();
        let page = page(5, false);
        let set = cells(vec![
            ((1, 1), ready("abc")),
            ((2, 2), ready("{}")),
            ((3, 1), ready("def")),
        ]);
        let review = build(&users(), &table(&structure, &page), &set, Values::Shown).unwrap();
        assert_eq!(
            review.lines[0],
            Line::Refused {
                row: "id 2".into(),
                reason: "email: INTEGER expects a whole number".into(),
            }
        );
        // The first of them, by its place in the set: a save fails there.
        let (part, error) = review.refused.clone().unwrap();
        assert_eq!(part, Part::Row(0));
        assert_eq!(error.to_string(), "email: INTEGER expects a whole number");
        // The row between them has its statement.
        assert_eq!(unlaid(&review).len(), 1);
        assert!(matches!(review.lines.last(), Some(Line::Refused { row, .. }) if row == "id 4"));
    }

    #[test]
    fn a_row_a_save_would_refuse_is_a_comment_and_no_statement() {
        let row = |key: Value, new: &str| RowChange {
            key: vec![("code".into(), key)],
            set: vec![CellChange {
                column: "note".into(),
                type_name: "text".into(),
                loaded: Value::Null,
                new: NewValue::Text(new.into()),
            }],
        };
        let text = |text: &str| Value::Text(text.into());
        // PostgreSQL text holds no NUL, and SQLite's key may be another
        // row's where it holds what stands for bytes it could not read: a
        // save refuses both rows, and so nothing shows a statement that
        // would not run.
        for (dialect, refused, reason) in [
            (
                Dialect::Postgres,
                row(text("b"), "nul\0"),
                "PostgreSQL text cannot hold a NUL character",
            ),
            (
                Dialect::Postgres,
                row(text("b\0"), "fine"),
                "PostgreSQL text cannot hold a NUL character",
            ),
            (
                Dialect::Sqlite,
                row(text("caf\u{FFFD}"), "fine"),
                "the row's key holds text that may not have been read exactly, so the save \
                 cannot be sure which row it names",
            ),
        ] {
            let changes = ChangeSet {
                object: users(),
                inserts: Vec::new(),
                rows: vec![row(text("a"), "fine"), refused],
            };
            for values in [Values::Shown, Values::Whole] {
                let review = of(dialect, &changes, Blocked::default(), values);
                // The row before it has its statement, and this one a
                // comment in the builder's words.
                assert_eq!(unlaid(&review).len(), 1, "{dialect:?}");
                assert!(
                    matches!(
                        review.lines.last(),
                        Some(Line::Refused { reason: said, .. }) if said == reason
                    ),
                    "{dialect:?}: {:?}",
                    review.lines.last()
                );
                assert_eq!(review.lines.len(), 6, "{dialect:?}");
                // Noted, by its place in the set: a save to production
                // asks nothing and fails the row.
                let (part, error) = review.refused.clone().unwrap();
                assert_eq!((part, error.to_string().as_str()), (Part::Row(1), reason));
            }
        }
    }

    #[test]
    fn a_set_no_change_set_comes_of_is_one_line_and_none_is_no_review() {
        let page = page(5, false);
        let keyless = Structure {
            primary_key: Vec::new(),
            ..fixture_structure()
        };
        let set = cells(vec![((1, 1), ready("bob")), ((3, 1), ready("dan"))]);
        let review = build(&users(), &table(&keyless, &page), &set, Values::Shown).unwrap();
        assert_eq!(review.lines, [Line::Unsendable]);
        assert_eq!((review.changes, review.rows), (2, 2));
        let structure = fixture_structure();
        assert_eq!(
            build(
                &users(),
                &table(&structure, &page),
                &BTreeMap::new(),
                Values::Shown
            ),
            None
        );
    }

    #[test]
    fn a_row_is_named_by_every_column_of_its_key_in_each_dialects_writing() {
        let change = |column: &str, loaded: Value, new: &str| CellChange {
            column: column.into(),
            type_name: "integer".into(),
            loaded,
            new: NewValue::Text(new.into()),
        };
        let changes = ChangeSet {
            object: ObjectRef::new("shop", "order_lines"),
            inserts: Vec::new(),
            rows: vec![RowChange {
                key: vec![
                    ("order_id".into(), Value::Int(7)),
                    ("line".into(), Value::Int(2)),
                ],
                set: vec![
                    change("quantity", Value::Int(1), "3"),
                    change("price", Value::Float(12.5), "13"),
                ],
            }],
        };
        let review = of(
            Dialect::Postgres,
            &changes,
            Blocked::default(),
            Values::Shown,
        );
        assert_eq!(review.lines[0], Line::Row("order_id 7, line 2".into()));
        // Numbers the page loaded are bare, as the grid shows them.
        assert_eq!(
            review.lines[1],
            Line::Check(vec![
                ("quantity".into(), "1".into()),
                ("price".into(), "12.5".into()),
            ])
        );
        // PostgreSQL converts a quoted literal itself.
        assert_eq!(
            sql(&review),
            [
                r#"UPDATE "shop"."order_lines""#,
                r#"   SET "quantity" = '3',"#,
                r#"       "price" = '13'"#,
                r#" WHERE "order_id" = 7"#,
                r#"   AND "line" = 2;"#,
            ]
        );
        // MySQL's names, and SQLite's numbers.
        let review = of(Dialect::MySql, &changes, Blocked::default(), Values::Shown);
        assert_eq!(sql(&review)[0], "UPDATE `shop`.`order_lines`");
        assert_eq!(sql(&review)[4], "   AND `line` = 2;");
        let review = of(Dialect::Sqlite, &changes, Blocked::default(), Values::Shown);
        assert_eq!(sql(&review)[1], r#"   SET "quantity" = 3,"#);
        // For every dialect the lines are the builder's statement.
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let built = dialect
                .update_row(&changes.object, &changes.rows[0])
                .unwrap();
            let review = of(dialect, &changes, Blocked::default(), Values::Whole);
            assert_eq!(
                unlaid(&review),
                [format!("{};", built.shown)],
                "{dialect:?}"
            );
        }
        // The review prints without what it holds.
        let printed = format!("{review:?}");
        assert_eq!(
            printed,
            "Review { changes: 2, rows: 1, added: 0, lines: 7 }"
        );
    }
}
