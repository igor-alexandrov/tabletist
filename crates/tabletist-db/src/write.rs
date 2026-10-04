//! Changing rows: what a save asks for, and what came of it.

use std::time::Duration;

use crate::{Error, ObjectRef, Result, Value};

/// Every change of one save, to one table. Written in one transaction, or
/// not at all.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangeSet {
    pub object: ObjectRef,
    pub rows: Vec<RowChange>,
}

/// The changes to one row.
#[derive(Debug, Clone, PartialEq)]
pub struct RowChange {
    /// The row key's columns (`Structure::row_key`) and their loaded
    /// values: what finds the row.
    pub key: Vec<(String, Value)>,
    pub set: Vec<CellChange>,
}

/// One cell's change.
#[derive(Debug, Clone, PartialEq)]
pub struct CellChange {
    pub column: String,
    /// The structure's type name (`ColumnInfo::type_name`), which decides
    /// how `new` is sent.
    pub type_name: String,
    /// What the page held. A save writes only while the row still does.
    pub loaded: Value,
    pub new: NewValue,
}

/// What a cell becomes: NULL, or text the database turns into the column's
/// type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewValue {
    Null,
    Text(String),
}

/// How a save ended. Only `Written` changed anything.
#[derive(Debug, Clone, PartialEq)]
pub enum WriteOutcome {
    /// Each row as the database now holds it, in the set's order.
    Written {
        rows: Vec<Vec<Value>>,
        elapsed: Duration,
    },
    /// Rows that are gone, or whose changed columns no longer hold what
    /// the page loaded. Nothing was written.
    Conflicts(Vec<Conflict>),
    /// The statement of `rows[row]` could not be built or failed. Nothing
    /// was written.
    Failed { row: usize, error: Error },
}

/// A row a save found changed since it was loaded.
#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    /// Its place in the set's rows.
    pub row: usize,
    /// The row as the database holds it now; `None` when it is gone.
    pub server: Option<Vec<Value>>,
}

impl ChangeSet {
    /// Refuses a set that cannot be written as it stands, before anything
    /// is sent: a statement without a key would touch every row.
    pub fn check(&self) -> Result<()> {
        if self.rows.is_empty() {
            return Err(Error::query("there is nothing to save"));
        }
        for row in &self.rows {
            if row.key.is_empty() {
                return Err(Error::query("a row to save has no key"));
            }
            if row.set.is_empty() {
                return Err(Error::query("a row to save has no change"));
            }
            if row.key.iter().any(|(_, value)| value.is_null()) {
                return Err(Error::query(
                    "a row to save cannot be found: its key is NULL",
                ));
            }
            for (index, change) in row.set.iter().enumerate() {
                if row.key.iter().any(|(column, _)| *column == change.column) {
                    return Err(Error::query(format!(
                        "{} is part of the row's key and cannot be changed",
                        change.column
                    )));
                }
                if row.set[..index]
                    .iter()
                    .any(|earlier| earlier.column == change.column)
                {
                    return Err(Error::query(format!(
                        "{} is changed twice in one row",
                        change.column
                    )));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(column: &str) -> CellChange {
        CellChange {
            column: column.into(),
            type_name: "text".into(),
            loaded: Value::Text("old".into()),
            new: NewValue::Text("new".into()),
        }
    }

    fn row(key: Vec<(&str, Value)>, set: Vec<CellChange>) -> RowChange {
        RowChange {
            key: key
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect(),
            set,
        }
    }

    fn set(rows: Vec<RowChange>) -> ChangeSet {
        ChangeSet {
            object: ObjectRef::new("main", "users"),
            rows,
        }
    }

    #[test]
    fn a_set_that_can_be_written_passes() {
        let changes = set(vec![row(vec![("id", Value::Int(1))], vec![cell("name")])]);
        assert_eq!(changes.check(), Ok(()));
    }

    #[test]
    fn a_set_that_cannot_be_written_is_refused_with_its_reason() {
        let id = || vec![("id", Value::Int(1))];
        for (changes, said) in [
            (set(Vec::new()), "nothing to save"),
            (set(vec![row(Vec::new(), vec![cell("name")])]), "no key"),
            (set(vec![row(id(), Vec::new())]), "no change"),
            (
                set(vec![row(vec![("id", Value::Null)], vec![cell("name")])]),
                "key is NULL",
            ),
            (
                set(vec![row(id(), vec![cell("id")])]),
                "part of the row's key",
            ),
            (
                set(vec![row(id(), vec![cell("name"), cell("name")])]),
                "changed twice",
            ),
        ] {
            let refused = changes.check().unwrap_err().to_string();
            assert!(refused.contains(said), "{said}: {refused}");
        }
    }
}
