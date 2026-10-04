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
        for (index, row) in self.rows.iter().enumerate() {
            // The same row twice: both changes would be compared with the
            // row as it was, and the second written over the first.
            if self.rows[..index]
                .iter()
                .any(|earlier| same_key(&earlier.key, &row.key))
            {
                return Err(Error::query("two rows to save have the same key"));
            }
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

/// Whether two values are the same value. Floats by their bits: NaN is NaN,
/// and a conflict is never made up by a comparison.
pub(crate) fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
        _ => a == b,
    }
}

/// Whether two keys find the same row: the same columns, each with the
/// same value, in whatever order each names them.
fn same_key(a: &[(String, Value)], b: &[(String, Value)]) -> bool {
    a.len() == b.len()
        && a.iter().all(|(column, value)| {
            b.iter()
                .any(|(other, theirs)| column == other && same(value, theirs))
        })
}

/// Whether the row as the database holds it (`server`, whose values
/// `columns` name) differs from what the page loaded in a column the save
/// changes. Other columns are not this save's business.
pub(crate) fn changed_since_loaded(
    row: &RowChange,
    columns: &[String],
    server: &[Value],
) -> Result<bool> {
    for change in &row.set {
        let found = columns
            .iter()
            .position(|column| *column == change.column)
            .and_then(|index| server.get(index));
        let Some(now) = found else {
            return Err(Error::query(format!("no such column: {}", change.column)));
        };
        if !same(now, &change.loaded) {
            return Ok(true);
        }
    }
    Ok(false)
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
            // Both would be compared with the row as it was, and the second
            // written over the first.
            (
                set(vec![
                    row(id(), vec![cell("name")]),
                    row(vec![("id", Value::Int(2))], vec![cell("name")]),
                    row(id(), vec![cell("email")]),
                ]),
                "the same key",
            ),
            (
                set(vec![
                    row(vec![("x", Value::Float(f64::NAN))], vec![cell("name")]),
                    row(vec![("x", Value::Float(f64::NAN))], vec![cell("name")]),
                ]),
                "the same key",
            ),
            // The order a key names its columns in does not change the
            // row it finds.
            (
                set(vec![
                    row(
                        vec![("a", Value::Int(1)), ("b", Value::Int(2))],
                        vec![cell("name")],
                    ),
                    row(
                        vec![("b", Value::Int(2)), ("a", Value::Int(1))],
                        vec![cell("email")],
                    ),
                ]),
                "the same key",
            ),
        ] {
            let refused = changes.check().unwrap_err().to_string();
            assert!(refused.contains(said), "{said}: {refused}");
        }
    }

    #[test]
    fn rows_with_different_keys_are_different_rows() {
        let pair = |a: Value, b: Value| vec![("a", a), ("b", b)];
        for (first, second) in [
            (
                vec![("id", Value::Int(1))],
                vec![("id", Value::Text("1".into()))],
            ),
            (vec![("id", Value::Int(1))], vec![("code", Value::Int(1))]),
            (
                pair(Value::Int(1), Value::Int(2)),
                pair(Value::Int(1), Value::Int(3)),
            ),
            (
                vec![("a", Value::Int(1))],
                pair(Value::Int(1), Value::Int(2)),
            ),
        ] {
            let changes = set(vec![
                row(first, vec![cell("name")]),
                row(second, vec![cell("name")]),
            ]);
            assert_eq!(changes.check(), Ok(()), "{changes:?}");
        }
    }

    #[test]
    fn floats_are_the_same_by_their_bits() {
        assert!(same(&Value::Float(f64::NAN), &Value::Float(f64::NAN)));
        assert!(!same(&Value::Float(0.1), &Value::Float(0.2)));
        assert!(same(&Value::Null, &Value::Null));
        assert!(!same(&Value::Int(1), &Value::Text("1".into())));
    }

    #[test]
    fn only_a_changed_column_makes_a_conflict() {
        let change = row(vec![("id", Value::Int(1))], vec![cell("name")]);
        let columns = ["id".to_owned(), "name".to_owned(), "email".to_owned()];
        let server = |name: &str, email: &str| {
            vec![
                Value::Int(1),
                Value::Text(name.into()),
                Value::Text(email.into()),
            ]
        };
        assert_eq!(
            changed_since_loaded(&change, &columns, &server("old", "a@x")),
            Ok(false)
        );
        // Another column changing is not this save's business.
        assert_eq!(
            changed_since_loaded(&change, &columns, &server("old", "b@x")),
            Ok(false)
        );
        assert_eq!(
            changed_since_loaded(&change, &columns, &server("theirs", "a@x")),
            Ok(true)
        );
        // A column the table does not have is an error, not a conflict.
        let gone = row(vec![("id", Value::Int(1))], vec![cell("nick")]);
        assert!(changed_since_loaded(&gone, &columns, &server("old", "a@x")).is_err());
    }
}
