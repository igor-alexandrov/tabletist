//! A save on SQLite: one `BEGIN IMMEDIATE` transaction, with `query_only`
//! lifted for as long as it lasts and no longer.

use std::time::Instant;

use rusqlite::OptionalExtension;
use rusqlite::types::ValueRef;

use super::{end_transaction, from_sqlite, map_error};
use crate::dialect::{InsertStatement, RowUpdate};
use crate::write::{
    Applied, Stored, changed_since_loaded, conflicts_of, more_than_one, named_twice, not_read_back,
    not_stopped, same_row_twice, spelled_otherwise,
};
use crate::{
    ChangeSet, Dialect, Error, ObjectRef, Result, RowChange, Sql, StopFlag, Value, WriteOutcome,
};

pub(super) fn write(
    connection: &rusqlite::Connection,
    changes: &ChangeSet,
    journal_mode: &str,
    stop: &StopFlag,
) -> Result<WriteOutcome> {
    // Every statement is built first: a key that may not be the row's, or
    // a value that cannot be sent, fails the save before the file is even
    // asked for. The builder refuses both.
    let mut updates = Vec::with_capacity(changes.rows.len());
    for (row, change) in changes.rows.iter().enumerate() {
        let built = Dialect::Sqlite.update_row(&changes.object, change);
        match built {
            Ok(update) => updates.push(update),
            Err(error) => return Ok(WriteOutcome::Failed { row, error }),
        }
    }
    let mut inserts = Vec::with_capacity(changes.inserts.len());
    for (insert, row) in changes.inserts.iter().enumerate() {
        // SQLite takes a name in other ASCII letters for the column too.
        let built = match named_twice(row, |a, b| a.eq_ignore_ascii_case(b)) {
            Some(name) => Err(Error::query(format!("{name} is set twice in one new row"))),
            None => Dialect::Sqlite.insert_row(&changes.object, row),
        };
        match built {
            Ok(built) => inserts.push(built),
            Err(error) => return Ok(WriteOutcome::FailedInsert { insert, error }),
        }
    }
    // Stopped before it began: the session is not touched.
    not_stopped(stop)?;
    let started = Instant::now();
    let applied = begin(connection, journal_mode, stop)
        .and_then(|()| apply(connection, changes, &inserts, &updates, stop));
    // Before the COMMIT is the last moment a stop is heard. Once it runs
    // the save is written, whatever arrives after it.
    let committed = match &applied {
        Ok(Applied::Rows(_)) => {
            not_stopped(stop).and_then(|()| connection.execute_batch("COMMIT").map_err(map_error))
        }
        _ => Ok(()),
    };
    // Whatever is still open is undone (a conflict, a failure, a stop, a
    // COMMIT that did not go through), and the session refuses writes
    // again. A session that cannot be put back is closed: it may still be
    // able to write.
    let closed = end_transaction(connection).map_err(|error| {
        Error::ConnectionLost(format!("could not end the save's transaction: {error}"))
    });
    let applied = applied.and_then(|applied| committed.map(|()| applied));
    closed?;
    Ok(applied?.outcome(started))
}

/// Puts back what a script may have left on the session and a write would
/// feel (a journal mode of its own, exclusive locking, CHECK constraints
/// ignored, triggers that fire themselves, an UPDATE that gives its count
/// as a row, a transaction still open), lifts `query_only`, and takes the
/// file, unless `stop` says not to: taking it can wait for another program
/// to let go.
fn begin(connection: &rusqlite::Connection, journal_mode: &str, stop: &StopFlag) -> Result<()> {
    // From no transaction, as on the other drivers. Nothing in the app
    // leaves one open (a script's is ended and checked), and one left open
    // would refuse the save's own BEGIN. It is undone and never joined:
    // what it wrote was not this save's to commit.
    if !connection.is_autocommit() {
        connection.execute_batch("ROLLBACK").map_err(map_error)?;
    }
    // Only WAL is a property of the file: every program that opens it finds
    // it so, and it is not ours to undo, nor to bring back. Any other mode
    // is the session's own, and a script can set it (`memory`, `truncate`,
    // `persist`, which leaves a journal beside the user's file; `off` is
    // refused in defensive mode). So a mode that is no longer the one the
    // session opened with goes back to it, unless either is WAL. `main`
    // only: with no schema the pragma would set every attached database's
    // mode too.
    let now: String = connection
        .query_row("PRAGMA main.journal_mode", [], |row| row.get(0))
        .map_err(map_error)?;
    let wal = |mode: &str| mode.eq_ignore_ascii_case("wal");
    if !now.eq_ignore_ascii_case(journal_mode) && !wal(&now) && !wal(journal_mode) {
        connection
            .pragma_update(Some("main"), "journal_mode", journal_mode)
            .map_err(map_error)?;
    }
    connection
        .pragma_update(None, "locking_mode", "NORMAL")
        .and_then(|()| connection.pragma_update(None, "ignore_check_constraints", false))
        // With them on, a trigger's own statements fire it again, and so
        // write other than what the file's triggers write for anyone else.
        .and_then(|()| connection.pragma_update(None, "recursive_triggers", false))
        // With it on an UPDATE gives a row, its count, and running it as a
        // statement without rows fails.
        .and_then(|()| connection.pragma_update(None, "count_changes", false))
        .and_then(|()| connection.pragma_update(None, "query_only", false))
        .map_err(map_error)?;
    not_stopped(stop)?;
    connection
        .execute_batch("BEGIN IMMEDIATE")
        .map_err(map_error)
}

/// A value bound exactly as it is. The filter path turns text that reads
/// as a number into one (`to_sqlite`); a save does not guess.
fn exact(value: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sqlite;
    match value {
        Value::Null => Sqlite::Null,
        Value::Bool(flag) => Sqlite::Integer(i64::from(*flag)),
        Value::Int(number) => Sqlite::Integer(*number),
        Value::Float(number) => Sqlite::Real(*number),
        Value::Text(text) => Sqlite::Text(text.to_string()),
        Value::Bytes(bytes) => Sqlite::Blob(bytes.to_vec()),
    }
}

/// A row as a statement gave it.
struct Found {
    values: Vec<Value>,
    /// The places of the cells whose text is not UTF-8. Their values hold
    /// U+FFFD for the bad bytes, and so equal what other bytes read as.
    inexact: Vec<usize>,
}

/// The rows of a statement, with their columns' names.
fn read(connection: &rusqlite::Connection, sql: &Sql) -> Result<(Vec<String>, Vec<Found>)> {
    let mut statement = connection.prepare(&sql.text).map_err(map_error)?;
    // Through the driver's own reader, never rusqlite's: a name that is
    // not UTF-8 panics there.
    let columns: Vec<String> = super::declared_columns(connection, &statement, &sql.text)?
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    let mut rows = statement
        .query(rusqlite::params_from_iter(sql.params.iter().map(exact)))
        .map_err(map_error)?;
    let mut found = Vec::new();
    while let Some(row) = rows.next().map_err(map_error)? {
        let mut values = Vec::with_capacity(columns.len());
        let mut inexact = Vec::new();
        for index in 0..columns.len() {
            let cell = row.get_ref(index).map_err(map_error)?;
            if matches!(cell, ValueRef::Text(bytes) if std::str::from_utf8(bytes).is_err()) {
                inexact.push(index);
            }
            values.push(from_sqlite(cell));
        }
        found.push(Found { values, inexact });
    }
    Ok((columns, found))
}

/// A column `change` sets whose stored text, in the row `server`, is not
/// UTF-8. What the page loaded from it and what the file holds now can be
/// different bytes that read alike, so the save cannot tell whether the
/// cell changed. Only the columns the save sets are compared; such text in
/// another column is not its business.
fn unreadable<'a>(change: &'a RowChange, columns: &[String], server: &Found) -> Option<&'a str> {
    change
        .set
        .iter()
        .map(|cell| cell.column.as_str())
        .find(|name| {
            columns
                .iter()
                .position(|column| column == name)
                .is_some_and(|index| server.inexact.contains(&index))
        })
}

/// A name `change` uses, in its key or its set, that more than one of the
/// row's `columns` reads as. A name that is not UTF-8 reads with U+FFFD for
/// those bytes, and so can read as another column's name; the statement
/// would then name that other column, while the row was compared in this
/// one. SQLite matches a name without regard to ASCII case, so that is how
/// they are counted.
fn ambiguous<'a>(change: &'a RowChange, columns: &[String]) -> Option<&'a str> {
    change
        .key
        .iter()
        .map(|(name, _)| name.as_str())
        .chain(change.set.iter().map(|cell| cell.column.as_str()))
        .find(|name| {
            columns
                .iter()
                .filter(|column| column.eq_ignore_ascii_case(name))
                .count()
                > 1
        })
}

/// What finds a row an `INSERT` made, to read it as it stands at the save's
/// end.
enum FoundBy {
    /// The rowid the insert gave it.
    Rowid,
    /// The primary key's columns, of a table WITHOUT ROWID.
    Key(Vec<String>),
}

impl FoundBy {
    fn of(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Self> {
        let without_rowid = connection
            .query_row(
                "SELECT wr <> 0 FROM pragma_table_list(?1) WHERE schema = ?2",
                [&object.name, &object.schema],
                |row| row.get::<_, bool>(0),
            )
            .optional()
            .map_err(map_error)?
            .unwrap_or(false);
        if !without_rowid {
            return Ok(Self::Rowid);
        }
        let key = super::key_names(connection, object)?;
        Ok(Self::Key(key.into_iter().map(|name| name.text).collect()))
    }

    /// The read that finds the row an insert `returned`, with these
    /// `columns`, and gave `rowid`. `None` when nothing finds it for sure.
    fn select(
        &self,
        object: &ObjectRef,
        columns: &[String],
        returned: &Found,
        rowid: i64,
    ) -> Option<Sql> {
        match self {
            Self::Rowid => {
                // A column of the table can take any of the rowid's three
                // names for itself: the one it left free is the rowid's.
                let name = ["rowid", "_rowid_", "oid"].into_iter().find(|name| {
                    !columns
                        .iter()
                        .any(|column| column.eq_ignore_ascii_case(name))
                })?;
                Some(Sql {
                    text: format!(
                        "SELECT * FROM {} WHERE {name} = ? LIMIT 2",
                        Dialect::Sqlite.qualified(object)
                    ),
                    params: vec![Value::Int(rowid)],
                })
            }
            Self::Key(names) => {
                let key = names
                    .iter()
                    .map(|name| {
                        let at = columns.iter().position(|column| column == name)?;
                        // Text that was not read exactly finds another row,
                        // or none: see `Dialect::key_read_exactly`.
                        let exact = !returned.inexact.contains(&at);
                        let value = returned.values.get(at)?;
                        (exact && !value.is_null()).then(|| (name.clone(), value.clone()))
                    })
                    .collect::<Option<Vec<_>>>()?;
                (!key.is_empty()).then(|| Dialect::Sqlite.select_row(object, &key, false))
            }
        }
    }
}

/// The steps of a save inside its transaction: read and compare every row,
/// update every row, read every row back. `stop` is asked before each
/// statement, and a save it ends is undone like every other end but
/// `Rows`.
fn apply(
    connection: &rusqlite::Connection,
    changes: &ChangeSet,
    inserts: &[InsertStatement],
    updates: &[RowUpdate],
    stop: &StopFlag,
) -> Result<Applied> {
    let dialect = Dialect::Sqlite;
    // The transaction holds the file, so a row read here is the row the
    // update will find.
    // Each change's row as its read found it, kept until every change has
    // read its own: two that read the same row are one row named twice.
    let mut names = Vec::new();
    let mut rows_read = Vec::with_capacity(changes.rows.len());
    let mut conflicts = Vec::new();
    for (row, change) in changes.rows.iter().enumerate() {
        let select = dialect.select_row(&changes.object, &change.key, true);
        not_stopped(stop)?;
        let (columns, mut found) = read(connection, &select)?;
        if let Some(name) = ambiguous(change, &columns) {
            return Err(Error::query(format!(
                "{name} names more than one column of the table, so the save cannot tell which \
                 it changes"
            )));
        }
        // SQLite takes a name in other ASCII letters for the column too.
        if let Some(name) = spelled_otherwise(change, &columns) {
            return Err(Error::query(format!(
                "the table spells {name} another way, so the save cannot be sure which column \
                 it names"
            )));
        }
        if found.len() > 1 {
            return Err(more_than_one());
        }
        let server = found.pop();
        let differs = match &server {
            None => true,
            Some(server) => {
                // A failure, not a conflict: a conflict offers to write
                // over what the file holds, which would still be unknown.
                if let Some(name) = unreadable(change, &columns, server) {
                    return Ok(Applied::Failed {
                        row,
                        error: Error::query(format!(
                            "{name} holds text that is not UTF-8, so the save cannot tell \
                             whether it changed since it was loaded"
                        )),
                    });
                }
                changed_since_loaded(change, &columns, &server.values)?
            }
        };
        if differs {
            conflicts.push(row);
        }
        rows_read.push(server.map(|server| server.values));
        names = columns;
    }
    // Before a conflict is answered too: a set that names a row twice is
    // not one to offer writing over what the file holds.
    same_row_twice(&changes.rows, &names, &rows_read)?;
    if !conflicts.is_empty() {
        return Ok(Applied::Conflicts(conflicts_of(conflicts, rows_read)));
    }
    // The new rows, before any row is changed: the order a review shows
    // them in. `RETURNING *` gives each as its `INSERT` left it, which is
    // before an AFTER trigger had its say: what it gives is used to find
    // the row again, once every statement has run.
    let found_by = if inserts.is_empty() {
        None
    } else {
        Some(FoundBy::of(connection, &changes.object)?)
    };
    let mut made = Vec::with_capacity(inserts.len());
    for (insert, statement) in inserts.iter().enumerate() {
        not_stopped(stop)?;
        let (columns, mut returned) = match read(connection, &statement.sql) {
            Ok(read) => read,
            // A cancel or a lost session ends the save; anything else is
            // the statement's own failure.
            Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
            Err(error) => return Ok(Applied::FailedInsert { insert, error }),
        };
        // One statement makes one row. A trigger that ran in its place can
        // have made none, and then what was stored is not known.
        let row = returned
            .pop()
            .filter(|_| returned.is_empty())
            .ok_or_else(not_read_back)?;
        // Read now: the next insert gives the session another.
        let rowid = connection.last_insert_rowid();
        made.push(
            found_by
                .as_ref()
                .and_then(|by| by.select(&changes.object, &columns, &row, rowid)),
        );
    }
    for (row, update) in updates.iter().enumerate() {
        not_stopped(stop)?;
        let params = rusqlite::params_from_iter(update.sql.params.iter().map(exact));
        let touched = match connection
            .execute(&update.sql.text, params)
            .map_err(map_error)
        {
            Ok(touched) => touched,
            // A cancel or a lost session ends the save; anything else is
            // the statement's own failure.
            Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
            Err(error) => return Ok(Applied::Failed { row, error }),
        };
        if touched != 1 {
            return Ok(Applied::Failed {
                row,
                error: Error::query(format!(
                    "the save would have changed {touched} rows where it meant one"
                )),
            });
        }
    }
    let mut rows = Vec::with_capacity(changes.rows.len());
    for change in &changes.rows {
        let select = dialect.select_row(&changes.object, &change.key, false);
        not_stopped(stop)?;
        let (_, mut found) = read(connection, &select)?;
        // One row still: a trigger the update fired can have made another
        // that the key also finds, and then which one was saved is not
        // known.
        if found.len() > 1 {
            return Err(more_than_one());
        }
        rows.push(found.pop().ok_or_else(not_read_back)?.values);
    }
    // The new rows last, as they stand once every statement has run.
    let mut inserted = Vec::with_capacity(made.len());
    for select in made {
        let Some(select) = select else {
            inserted.push(None);
            continue;
        };
        not_stopped(stop)?;
        let (_, mut found) = read(connection, &select)?;
        if found.len() > 1 {
            return Err(more_than_one());
        }
        // None: a trigger moved the row to another key, or took it away.
        inserted.push(found.pop().map(|row| row.values));
    }
    Ok(Applied::Rows(Stored { inserted, rows }))
}
