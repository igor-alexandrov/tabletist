//! Review SQL: the statements a save of a tab's pending changes would run,
//! as lines a person reads before saving. The reducer makes them when the
//! pending set changes; a view words the comment lines and lays the rest
//! out. Nothing here is a sentence of the app's: a comment is data.

use std::collections::BTreeMap;

use tabletist_db::{ChangeSet, Dialect, Error, ObjectRef, RowChange, RowUpdate, Value};

use crate::edit::{Pending, State, Table, change_set};
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
    /// `UPDATE`, `SET`, `WHERE`, `AND`, and a `NULL`.
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

/// What a save of a tab's pending changes would run.
#[derive(Clone, PartialEq)]
pub struct Review {
    /// How many cells the set changes, and in how many rows.
    pub changes: usize,
    pub rows: usize,
    pub lines: Vec<Line>,
    /// The first row whose statement the builder refused, by its place in
    /// the change set, and why.
    pub refused: Option<(usize, Error)>,
}

/// Without the lines: they hold what the user typed, which stays out of
/// logs and panics.
impl std::fmt::Debug for Review {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Review {{ changes: {}, rows: {}, lines: {} }}",
            self.changes,
            self.rows,
            self.lines.len()
        )
    }
}

/// The review of the pending `cells` of `table`. `None` when nothing is
/// pending.
pub fn build(
    object: &ObjectRef,
    table: &Table<'_>,
    cells: &BTreeMap<(usize, usize), Pending>,
    values: Values,
) -> Option<Review> {
    if cells.is_empty() {
        return None;
    }
    let Some((changes, places)) = change_set(object, table, cells) else {
        // The map is ordered by row: each row's cells are together.
        let mut rows: Vec<usize> = cells.keys().map(|&(row, _)| row).collect();
        rows.dedup();
        return Some(Review {
            changes: cells.len(),
            rows: rows.len(),
            lines: vec![Line::Unsendable],
            refused: None,
        });
    };
    // The columns to fix of each row of the set, in the set's order.
    let blocked: Vec<Vec<String>> = places
        .iter()
        .map(|&row| {
            cells
                .range((row, 0)..=(row, usize::MAX))
                .filter(|(_, cell)| matches!(cell.state, State::ToFix(_)))
                .filter_map(|(&(_, col), _)| table.page.columns.get(col))
                .map(|column| format::display_safe(&column.name).into_owned())
                .collect()
        })
        .collect();
    Some(of(table.dialect, &changes, &blocked, values))
}

/// The review of `changes`, as `dialect` writes them. `blocked` names, for
/// each row of the set, its columns that are to fix: a row with any is a
/// comment only.
pub fn of(
    dialect: Dialect,
    changes: &ChangeSet,
    blocked: &[Vec<String>],
    values: Values,
) -> Review {
    let mut lines = Vec::new();
    let mut refused = None;
    for (index, row) in changes.rows.iter().enumerate() {
        let name = row_name(row);
        if let Some(columns) = blocked.get(index).filter(|columns| !columns.is_empty()) {
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
                    (column, loaded(&change.loaded))
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
                refused.get_or_insert((index, error));
            }
        }
    }
    Review {
        changes: changes.rows.iter().map(|row| row.set.len()).sum(),
        rows: changes.rows.len(),
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
/// quotes, everything else as the grid shows it. A long text is cut to 59
/// characters and `…` inside its quotes. Never a line break: a comment
/// ends at one, and what followed would be read as SQL by whoever pastes
/// the text.
fn loaded(value: &Value) -> String {
    let Value::Text(text) = value else {
        return format::cell_text(value).into_owned();
    };
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
    let ink = if literal == "NULL" {
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

/// A literal as it is shown, its hidden characters written out: one that
/// would take more than `VALUE_MAX_CHARS` characters keeps its beginning,
/// then `…`, then what closes it (`'`, or `')` for SQLite's text around a
/// NUL), so it still reads as one value.
fn shortened(literal: &str) -> String {
    let (kept, whole) = head(literal, VALUE_MAX_CHARS);
    if whole {
        return format::escape_hidden(kept).into_owned();
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
    use tabletist_db::{Access, CellChange, NewValue, ObjectKind, RowPage, Structure};

    fn users() -> ObjectRef {
        ObjectRef::new("main", "users")
    }

    fn table<'a>(structure: &'a Structure, page: &'a RowPage) -> Table<'a> {
        Table {
            access: Access::Writable,
            kind: ObjectKind::Table,
            dialect: Dialect::Sqlite,
            structure: Some(structure),
            page,
            refreshing: false,
            saving: false,
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
                Some(statement) if !line.starts_with("UPDATE") => {
                    statement.push(' ');
                    statement.push_str(line.trim_start_matches(' '));
                }
                _ => statements.push(line),
            }
        }
        statements
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
        // Its comments are for reading, and stay cut.
        assert_eq!(whole.lines[1], shown.lines[1]);
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
        let text = Value::Text("o".repeat(200).into());
        assert_eq!(loaded(&text), format!("'{}…'", "o".repeat(59)));
        let fits = Value::Text("o".repeat(60).into());
        assert_eq!(loaded(&fits), format!("'{}'", "o".repeat(60)));
        let breaks = Value::Text("\n".repeat(58).into());
        assert_eq!(loaded(&breaks), format!("'{}…'", "<U+000A>".repeat(7)));
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
            rows: vec![row],
        };
        let review = of(Dialect::Postgres, &changes, &[], Values::Shown);
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
            let built = dialect
                .update_row(&changes.object, &changes.rows[0])
                .unwrap();
            // Whole, the lines are the builder's text to the byte.
            let whole = of(dialect, &changes, &[], Values::Whole);
            assert_eq!(unlaid(&whole), [format!("{};", built.shown)], "{dialect:?}");
            // Shown, they are the same lines, and each is one line.
            let shown = of(dialect, &changes, &[], Values::Shown);
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
        let (index, error) = review.refused.clone().unwrap();
        assert_eq!(index, 0);
        assert_eq!(error.to_string(), "email: INTEGER expects a whole number");
        // The row between them has its statement.
        assert_eq!(unlaid(&review).len(), 1);
        assert!(matches!(review.lines.last(), Some(Line::Refused { row, .. }) if row == "id 4"));
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
        let review = of(Dialect::Postgres, &changes, &[], Values::Shown);
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
        let review = of(Dialect::MySql, &changes, &[], Values::Shown);
        assert_eq!(sql(&review)[0], "UPDATE `shop`.`order_lines`");
        assert_eq!(sql(&review)[4], "   AND `line` = 2;");
        let review = of(Dialect::Sqlite, &changes, &[], Values::Shown);
        assert_eq!(sql(&review)[1], r#"   SET "quantity" = 3,"#);
        // For every dialect the lines are the builder's statement.
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let built = dialect
                .update_row(&changes.object, &changes.rows[0])
                .unwrap();
            let review = of(dialect, &changes, &[], Values::Whole);
            assert_eq!(
                unlaid(&review),
                [format!("{};", built.shown)],
                "{dialect:?}"
            );
        }
        // The review prints without what it holds.
        let printed = format!("{review:?}");
        assert_eq!(printed, "Review { changes: 2, rows: 1, lines: 7 }");
    }
}
