//! Changing rows: what a save asks for, and what came of it.

use std::time::{Duration, Instant};

use crate::{Error, ObjectRef, Result, StopFlag, Value};

/// Every change of one save, to one table. Written in one transaction, or
/// not at all.
#[derive(Clone, PartialEq)]
pub struct ChangeSet {
    pub object: ObjectRef,
    pub rows: Vec<RowChange>,
}

/// Its table and its counts, without the values: a set is what a command
/// to save carries, and what a user typed and what a page loaded stay out
/// of logs and panics. A row and a cell are still printed whole.
impl std::fmt::Debug for ChangeSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cells: usize = self.rows.iter().map(|row| row.set.len()).sum();
        f.debug_struct("ChangeSet")
            .field("object", &self.object)
            .field("rows", &self.rows.len())
            .field("cells", &cells)
            .finish_non_exhaustive()
    }
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
            // A key that names a column twice would not compare as the key
            // it is: the check for the same row twice counts its columns.
            if let Some((column, _)) = row.key.iter().enumerate().find_map(|(index, named)| {
                row.key[..index]
                    .iter()
                    .any(|(earlier, _)| *earlier == named.0)
                    .then_some(named)
            }) {
                return Err(Error::query(format!(
                    "{column} is named twice in the row's key"
                )));
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

/// What the statements of a save came to, before its transaction ends.
/// Each driver ends its own: committed for `Rows`, rolled back for the
/// others.
pub(crate) enum Applied {
    Rows(Vec<Vec<Value>>),
    Conflicts(Vec<Conflict>),
    Failed { row: usize, error: Error },
}

impl Applied {
    /// The outcome of a save that began at `started`, once its transaction
    /// has ended.
    pub(crate) fn outcome(self, started: Instant) -> WriteOutcome {
        match self {
            Self::Rows(rows) => WriteOutcome::Written {
                rows,
                elapsed: started.elapsed(),
            },
            Self::Conflicts(conflicts) => WriteOutcome::Conflicts(conflicts),
            Self::Failed { row, error } => WriteOutcome::Failed { row, error },
        }
    }
}

/// Ends a save that was told to stop, as a cancelled statement ends it. A
/// cancel reaches only a statement that is running, and one that arrives
/// between two of a save's statements is lost: so each driver asks here
/// before every statement it sends, and once more before `COMMIT`. After
/// that nothing is asked, and the save is written.
pub(crate) fn not_stopped(stop: &StopFlag) -> Result<()> {
    if stop.is_stopped() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

/// A key is a row's only while one row has it.
pub(crate) fn more_than_one() -> Error {
    Error::query("a row's key matches more than one row")
}

/// The row a save changed is no longer found by its key.
pub(crate) fn not_read_back() -> Error {
    Error::query("a saved row could not be read back")
}

/// Whether two values are the same value. Floats by their bits: NaN is NaN,
/// and a conflict is never made up by a comparison.
pub(crate) fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
        _ => a == b,
    }
}

/// A name `change` uses, in its key or its set, that is not one of the
/// row's `columns` as the table spells them. MySQL and SQLite take a name
/// in other letters for the column all the same, and the save's own
/// checks, which compare names exactly, would take it for another: two
/// changes keyed by `ID` and by `id` would pass for two rows. Only a name
/// spelled as the table spells it is known to be the column it reads as.
pub(crate) fn spelled_otherwise<'a>(change: &'a RowChange, columns: &[String]) -> Option<&'a str> {
    change
        .key
        .iter()
        .map(|(name, _)| name.as_str())
        .chain(change.set.iter().map(|cell| cell.column.as_str()))
        .find(|name| !columns.iter().any(|column| column == name))
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

/// The conflicts of a save: the changes at `places` in its set, each with
/// the row its read found among `read` (`None` when it is gone).
pub(crate) fn conflicts_of(places: Vec<usize>, mut read: Vec<Option<Vec<Value>>>) -> Vec<Conflict> {
    places
        .into_iter()
        .map(|row| Conflict {
            row,
            server: read.get_mut(row).and_then(Option::take),
        })
        .collect()
}

/// Refuses a save two of whose changes read the same row. `check` has let
/// through only keys that differ as values, and the database decides which
/// row a key finds: `1` and `'1'` find the row of an integer key alike,
/// `0.0` and `-0.0` the row of a float one, and a row can be named by one
/// unique column and by another. Both changes would then be compared with
/// the row as it was, and the later written over the earlier.
///
/// `read` holds, for each of `rows`, the row its locking read found (`None`
/// when it is gone), and `columns` names the values of every one of them:
/// they are reads of one table in one transaction. Two reads found the same
/// row when their rows hold the same values, as the database returned them,
/// in the key columns of either change: each read found one row by a key
/// that names one row, so a row with that key's values is that row.
pub(crate) fn same_row_twice(
    rows: &[RowChange],
    columns: &[String],
    read: &[Option<Vec<Value>>],
) -> Result<()> {
    // Where each change's key columns are among a row's values.
    let mut keys = Vec::with_capacity(rows.len());
    for row in rows {
        let places: Vec<usize> = row
            .key
            .iter()
            .map(|(name, _)| {
                columns
                    .iter()
                    .position(|column| column == name)
                    .ok_or_else(|| Error::query(format!("no such column: {name}")))
            })
            .collect::<Result<_>>()?;
        keys.push(places);
    }
    for (later, theirs) in read.iter().enumerate() {
        let Some(theirs) = theirs else { continue };
        for (earlier, ours) in read.iter().enumerate().take(later) {
            let Some(ours) = ours else { continue };
            let alike = |key: Option<&Vec<usize>>| {
                key.is_some_and(|places| {
                    places.iter().all(|&place| {
                        matches!(
                            (ours.get(place), theirs.get(place)),
                            (Some(ours), Some(theirs)) if same(ours, theirs)
                        )
                    })
                })
            };
            if alike(keys.get(earlier)) || alike(keys.get(later)) {
                return Err(Error::query("two changes of the save name the same row"));
            }
        }
    }
    Ok(())
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

    /// A set is what a command to save carries at its top, and a command
    /// is printed in a diagnostic or a panic: by its table and its counts,
    /// never by a value the page loaded or the user typed.
    #[test]
    fn a_set_is_printed_by_its_counts_without_the_values_it_carries() {
        let cell = |column: &str, loaded: &str, new: NewValue| CellChange {
            column: column.into(),
            type_name: "text".into(),
            loaded: Value::Text(loaded.into()),
            new,
        };
        let changes = set(vec![
            row(
                vec![("id", Value::Int(4711))],
                vec![
                    cell("name", "loaded name", NewValue::Text("typed name".into())),
                    cell("email", "loaded@example.com", NewValue::Null),
                ],
            ),
            row(
                vec![("id", Value::Text("key as text".into()))],
                vec![cell("name", "another", NewValue::Text("secret".into()))],
            ),
        ]);
        let printed = format!("{changes:?}");
        assert_eq!(
            printed,
            r#"ChangeSet { object: ObjectRef { schema: "main", name: "users" }, rows: 2, cells: 3, .. }"#
        );
        // On several lines too, as a panic prints a value.
        let pretty = format!("{changes:#?}");
        assert!(
            pretty.contains("rows: 2") && pretty.contains("cells: 3"),
            "{pretty}"
        );
        for hidden in [
            "loaded name",
            "typed name",
            "loaded@example.com",
            "another",
            "secret",
            "4711",
            "key as text",
            "email",
        ] {
            assert!(!printed.contains(hidden), "{hidden}: {printed}");
            assert!(!pretty.contains(hidden), "{hidden}: {pretty}");
        }
        // A row and a cell are printed whole: a test names them in what
        // it says of a failure, and no command carries one at its top.
        assert!(format!("{:?}", changes.rows[0]).contains("typed name"));
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
            // A key with a column twice would not count as the key it is,
            // beside the same row keyed once.
            (
                set(vec![
                    row(id(), vec![cell("name")]),
                    row(
                        vec![("id", Value::Int(1)), ("id", Value::Int(1))],
                        vec![cell("email")],
                    ),
                ]),
                "named twice in the row's key",
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
            // By its rows: a set is printed by its counts alone.
            assert_eq!(changes.check(), Ok(()), "{:?}", changes.rows);
        }
    }

    #[test]
    fn a_save_told_to_stop_ends_as_a_cancelled_one() {
        let stop = StopFlag::new();
        assert_eq!(not_stopped(&stop), Ok(()));
        stop.clone().stop();
        assert_eq!(not_stopped(&stop), Err(Error::Cancelled));
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

    #[test]
    fn two_changes_that_read_the_same_row_are_refused() {
        let columns = ["id".to_owned(), "email".to_owned(), "name".to_owned()];
        let person = |id: Value, email: &str| {
            Some(vec![
                id,
                Value::Text(email.into()),
                Value::Text("old".into()),
            ])
        };
        let by =
            |column: &str, value: Value, set: &str| row(vec![(column, value)], vec![cell(set)]);
        let said = |rows: &[RowChange], read: &[Option<Vec<Value>>]| {
            same_row_twice(rows, &columns, read).map_err(|error| error.to_string())
        };
        let twice = Err("two changes of the save name the same row".to_owned());
        // One key spelled two ways: the database found the row of the
        // number for both, and the set passes `check`.
        let spelled = [
            by("id", Value::Int(1), "name"),
            by("id", Value::Text("1".into()), "email"),
        ];
        assert_eq!(set(spelled.to_vec()).check(), Ok(()));
        assert_eq!(
            said(
                &spelled,
                &[person(Value::Int(1), "a@x"), person(Value::Int(1), "a@x")]
            ),
            twice
        );
        // The same two keys where the database holds them apart (a column
        // of no type on SQLite): two rows.
        assert_eq!(
            said(
                &spelled,
                &[
                    person(Value::Int(1), "a@x"),
                    person(Value::Text("1".into()), "b@x")
                ]
            ),
            Ok(())
        );
        // One row by two of its unique columns, whichever comes first.
        let two_ways = [
            by("id", Value::Int(1), "name"),
            by("email", Value::Text("a@x".into()), "name"),
        ];
        assert_eq!(
            said(
                &two_ways,
                &[person(Value::Int(1), "a@x"), person(Value::Int(1), "a@x")]
            ),
            twice
        );
        assert_eq!(
            said(
                &two_ways,
                &[person(Value::Int(1), "a@x"), person(Value::Int(2), "b@x")]
            ),
            Ok(())
        );
        // A float key by its bits, as the database returned it: the row
        // read for `0.0` and for `-0.0` holds one of them.
        let zeros = [
            by("id", Value::Float(0.0), "name"),
            by("id", Value::Float(-0.0), "email"),
        ];
        assert_eq!(set(zeros.to_vec()).check(), Ok(()));
        assert_eq!(
            said(
                &zeros,
                &[
                    person(Value::Float(0.0), "a@x"),
                    person(Value::Float(0.0), "a@x")
                ]
            ),
            twice
        );
        assert_eq!(
            said(
                &[
                    by("id", Value::Float(f64::NAN), "name"),
                    by("email", Value::Text("a@x".into()), "name")
                ],
                &[
                    person(Value::Float(f64::NAN), "a@x"),
                    person(Value::Float(f64::NAN), "a@x")
                ]
            ),
            twice
        );
        // Not only neighbours, and a row that is gone is no row twice.
        let three = [
            by("id", Value::Int(1), "name"),
            by("id", Value::Int(2), "name"),
            by("id", Value::Text("1".into()), "email"),
        ];
        assert_eq!(
            said(
                &three,
                &[
                    person(Value::Int(1), "a@x"),
                    person(Value::Int(2), "b@x"),
                    person(Value::Int(1), "a@x")
                ]
            ),
            twice
        );
        assert_eq!(
            said(&three, &[None, person(Value::Int(2), "b@x"), None]),
            Ok(())
        );
        // A composite key is the same only in every column.
        let pairs = [
            row(
                vec![("id", Value::Int(1)), ("email", Value::Text("a@x".into()))],
                vec![cell("name")],
            ),
            row(
                vec![("id", Value::Int(1)), ("email", Value::Text("b@x".into()))],
                vec![cell("name")],
            ),
        ];
        assert_eq!(
            said(
                &pairs,
                &[person(Value::Int(1), "a@x"), person(Value::Int(1), "b@x")]
            ),
            Ok(())
        );
        // A key column the rows do not have is an error of its own.
        let unknown = [
            by("id", Value::Int(1), "name"),
            by("code", Value::Int(2), "name"),
        ];
        assert_eq!(
            said(
                &unknown,
                &[person(Value::Int(1), "a@x"), person(Value::Int(2), "b@x")]
            ),
            Err("no such column: code".to_owned())
        );
    }
}
