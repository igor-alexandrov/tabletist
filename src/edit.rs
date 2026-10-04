//! Editing a table's values: what can be edited, what a column takes, the
//! pending set a tab holds and the change set a save sends. Everything here
//! is decided from the page and the structure alone; the reducer in
//! `app.rs` owns every transition.

use tabletist_db::{
    Access, ColumnClass, ColumnInfo, Dialect, NewValue, ObjectKind, RowPage, Structure, Value,
    column_class,
};

use crate::model::CellPos;

/// The largest value an editor opens, in bytes of its text: a field that
/// held megabytes would be laid out every frame.
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

    /// Why `cell` cannot be edited, or `None` when it can. The reasons that
    /// hold for the whole table come first, so every cell of such a table
    /// says the same.
    pub fn lock(&self, cell: CellPos) -> Option<Lock> {
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
        if self.saving {
            return Some(Lock::Saving);
        }
        if self.refreshing {
            return Some(Lock::Refreshing);
        }
        let Some(row) = self.page.rows.get(cell.row) else {
            return Some(Lock::NoSuchCell);
        };
        let Some(value) = row.get(cell.col) else {
            return Some(Lock::NoSuchCell);
        };
        // A row narrower than the page has no key to read.
        let held = |col: &usize| row.get(*col);
        if key.iter().any(|col| held(col).is_none_or(Value::is_null)) {
            return Some(Lock::KeyIsNull);
        }
        if self.dialect == Dialect::Sqlite
            && key.iter().any(
                |col| matches!(held(col), Some(Value::Text(text)) if text.contains('\u{FFFD}')),
            )
        {
            return Some(Lock::KeyInexact);
        }
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

    /// Whether a save could not match a key column of this type exactly.
    /// MySQL shows a TIMESTAMP in the session's zone without it, reads a BIT
    /// bound as bytes as a number, and misses a FLOAT bound as a double; the
    /// save refuses such a key (`tabletist-db`, `mysql/write.rs`).
    fn unmatched(&self, col: usize) -> bool {
        self.dialect == Dialect::MySql
            && self.column(col).is_some_and(|column| {
                let word: String = column
                    .type_name
                    .chars()
                    .take_while(|letter| letter.is_ascii_alphabetic())
                    .collect::<String>()
                    .to_ascii_lowercase();
                matches!(word.as_str(), "timestamp" | "bit" | "float")
            })
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
    /// More decimals than the type keeps: `stored` is what the database
    /// would have rounded it to.
    Decimals { scale: u32, stored: String },
    /// More digits before the point than the type holds: at most `whole`.
    Digits { whole: u32 },
    /// Not `true`, `false`, `1` or `0`.
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
}

/// Whether `text` is a value `column` takes, as far as the app can tell
/// from its type's name. A type it does not know has no check.
pub fn check(dialect: Dialect, column: &ColumnInfo, text: &str) -> Option<Problem> {
    if let Some(allowed) = &column.allowed_values {
        return (!allowed.iter().any(|value| value == text))
            .then(|| Problem::NotOneOf(allowed.clone()));
    }
    let typed = text.trim();
    match column_class(dialect, &column.type_name) {
        ColumnClass::Integer { min, max } => match typed.parse::<i128>() {
            Err(_) => Some(Problem::WholeNumber),
            Ok(number) if number < min || number > max => Some(Problem::OutOfRange { min, max }),
            Ok(_) => None,
        },
        ColumnClass::Decimal { precision, scale } => decimal(typed, precision, scale),
        ColumnClass::Float => {
            let word =
                dialect == Dialect::Postgres && matches!(typed, "NaN" | "Infinity" | "-Infinity");
            let number = typed.parse::<f64>().is_ok_and(f64::is_finite);
            (!word && !number).then_some(Problem::Number)
        }
        ColumnClass::Boolean => (!matches!(
            typed.to_ascii_lowercase().as_str(),
            "true" | "false" | "1" | "0"
        ))
        .then_some(Problem::Boolean),
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

/// A plain decimal number within the digits and the scale its type states.
/// No exponent: the databases take one, but what it would be stored as is
/// not what the user sees typed.
fn decimal(typed: &str, precision: Option<u32>, scale: Option<u32>) -> Option<Problem> {
    let (negative, digits) = match typed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, typed.strip_prefix('+').unwrap_or(typed)),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty()) || !all_digits(whole) || !all_digits(fraction) {
        return Some(Problem::Number);
    }
    let scale = scale?;
    let kept = fraction.trim_end_matches('0');
    if kept.len() > scale as usize {
        return Some(Problem::Decimals {
            scale,
            stored: rounded(negative, whole, fraction, scale as usize),
        });
    }
    let whole_digits = whole.trim_start_matches('0').len() as u32;
    match precision {
        Some(precision) if whole_digits > precision.saturating_sub(scale) => {
            Some(Problem::Digits {
                whole: precision.saturating_sub(scale),
            })
        }
        _ => None,
    }
}

/// `whole.fraction` rounded half away from zero to `scale` decimals, as the
/// databases round a decimal. `fraction` is longer than `scale`.
fn rounded(negative: bool, whole: &str, fraction: &str, scale: usize) -> String {
    let whole = if whole.is_empty() { "0" } else { whole };
    let mut digits: Vec<u8> = whole
        .bytes()
        .chain(fraction.bytes().take(scale))
        .map(|byte| byte - b'0')
        .collect();
    if fraction.as_bytes()[scale] >= b'5' {
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
    while digits.len() > scale + 1 && digits[0] == 0 {
        digits.remove(0);
    }
    let point = digits.len() - scale;
    let mut text = String::with_capacity(digits.len() + 2);
    if negative && digits.iter().any(|&digit| digit != 0) {
        text.push('-');
    }
    for (place, digit) in digits.iter().enumerate() {
        if place == point && scale > 0 {
            text.push('.');
        }
        text.push(char::from(b'0' + digit));
    }
    text
}

/// Where an editor starts: the whole value as the database gave it, never
/// the shortened text a cell shows. Empty on NULL. A boolean column reads
/// `true` or `false` whatever the driver loaded (SQLite and MySQL hold 1
/// and 0).
pub fn start_text(value: &Value, class: ColumnClass) -> String {
    match (value, class) {
        (Value::Null, _) => String::new(),
        (Value::Int(1), ColumnClass::Boolean) => "true".to_owned(),
        (Value::Int(0), ColumnClass::Boolean) => "false".to_owned(),
        (value, _) => crate::ui::format::plain_text(value),
    }
}

/// Whether `new` differs from what the cell loaded: a text equal to where
/// the editor starts, or NULL on a NULL cell, is no change.
pub fn is_change(loaded: &Value, new: &NewValue, class: ColumnClass) -> bool {
    match new {
        NewValue::Null => !loaded.is_null(),
        NewValue::Text(text) => loaded.is_null() || *text != start_text(loaded, class),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tabletist_db::{ColumnMeta, IndexInfo, ValueKind};

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

    fn table<'a>(structure: Option<&'a Structure>, page: &'a RowPage) -> Table<'a> {
        Table {
            access: Access::Writable,
            kind: ObjectKind::Table,
            dialect: Dialect::Sqlite,
            structure,
            page,
            refreshing: false,
            saving: false,
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
        assert_eq!(pg("1e3"), Some(Problem::Number));
        assert_eq!(pg("twelve"), Some(Problem::Number));
        // Without stated digits, any number.
        assert_eq!(check(Dialect::Postgres, &typed("numeric"), "12.505"), None);
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
    fn long_broken_and_json_values_open_the_large_editor() {
        let plain = ColumnClass::Text { max_chars: None };
        assert!(!opens_large("short", plain));
        assert!(opens_large("two\nlines", plain));
        assert!(opens_large(&"x".repeat(257), plain));
        assert!(!opens_large(&"x".repeat(256), plain));
        assert!(opens_large("{}", ColumnClass::Json));
    }
}
