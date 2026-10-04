//! Editing a table's values: what can be edited, what a column takes, the
//! pending set a tab holds and the change set a save sends. Everything here
//! is decided from the page and the structure alone; the reducer in
//! `app.rs` owns every transition.

use tabletist_db::{
    Access, ColumnClass, ColumnInfo, Dialect, ObjectKind, RowPage, Structure, Value, column_class,
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
}
