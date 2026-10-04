//! A save on SQLite: one `BEGIN IMMEDIATE` transaction, with `query_only`
//! lifted for as long as it lasts and no longer.

use std::time::Instant;

use rusqlite::types::ValueRef;

use super::{end_transaction, from_sqlite, map_error};
use crate::dialect::RowUpdate;
use crate::write::changed_since_loaded;
use crate::{ChangeSet, Conflict, Dialect, Error, Result, RowChange, Sql, Value, WriteOutcome};

/// What the statements of a save came to, before its transaction ends.
enum Applied {
    Rows(Vec<Vec<Value>>),
    Conflicts(Vec<Conflict>),
    Failed { row: usize, error: Error },
}

pub(super) fn write(
    connection: &rusqlite::Connection,
    changes: &ChangeSet,
    journal_mode: &str,
) -> Result<WriteOutcome> {
    // Every statement is built first: a key that may not be the row's, or
    // a value that cannot be sent, fails the save before the file is even
    // asked for.
    let mut updates = Vec::with_capacity(changes.rows.len());
    for (row, change) in changes.rows.iter().enumerate() {
        let built = key_read_exactly(change)
            .and_then(|()| Dialect::Sqlite.update_row(&changes.object, change));
        match built {
            Ok(update) => updates.push(update),
            Err(error) => return Ok(WriteOutcome::Failed { row, error }),
        }
    }
    let started = Instant::now();
    let applied =
        begin(connection, journal_mode).and_then(|()| apply(connection, changes, &updates));
    let committed = match &applied {
        Ok(Applied::Rows(_)) => connection.execute_batch("COMMIT").map_err(map_error),
        _ => Ok(()),
    };
    // Whatever is still open is undone (a conflict, a failure, a COMMIT
    // that did not go through), and the session refuses writes again. A
    // session that cannot be put back is closed: it may still be able to
    // write.
    let closed = end_transaction(connection).map_err(|error| {
        Error::ConnectionLost(format!("could not end the save's transaction: {error}"))
    });
    let applied = applied.and_then(|applied| committed.map(|()| applied));
    closed?;
    Ok(match applied? {
        Applied::Rows(rows) => WriteOutcome::Written {
            rows,
            elapsed: started.elapsed(),
        },
        Applied::Conflicts(conflicts) => WriteOutcome::Conflicts(conflicts),
        Applied::Failed { row, error } => WriteOutcome::Failed { row, error },
    })
}

/// Puts back what a script may have left on the session and a write would
/// feel (a journal mode of its own, exclusive locking, CHECK constraints
/// ignored, triggers that fire themselves, an UPDATE that gives its count
/// as a row), lifts `query_only`, and takes the file.
fn begin(connection: &rusqlite::Connection, journal_mode: &str) -> Result<()> {
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
        .and_then(|()| connection.execute_batch("BEGIN IMMEDIATE"))
        .map_err(map_error)
}

/// Refuses a key that may not be the row's. Text that is not UTF-8 is read
/// with U+FFFD for its bad bytes (`from_sqlite`), so a key that holds one
/// may stand for other bytes, and bound as it reads it finds another row,
/// whose key really is that text. A key that really holds U+FFFD is refused
/// with it, since the page's value cannot tell the two apart. That is
/// accepted.
fn key_read_exactly(change: &RowChange) -> Result<()> {
    let lossy = |value: &Value| matches!(value, Value::Text(text) if text.contains('\u{FFFD}'));
    if change.key.iter().any(|(_, value)| lossy(value)) {
        return Err(Error::query(
            "the row's key holds text that may not have been read exactly, so the save cannot \
             be sure which row it names",
        ));
    }
    Ok(())
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

fn apply(
    connection: &rusqlite::Connection,
    changes: &ChangeSet,
    updates: &[RowUpdate],
) -> Result<Applied> {
    let dialect = Dialect::Sqlite;
    // The transaction holds the file, so a row read here is the row the
    // update will find.
    let mut conflicts = Vec::new();
    for (row, change) in changes.rows.iter().enumerate() {
        let select = dialect.select_row(&changes.object, &change.key, true);
        let (columns, mut found) = read(connection, &select)?;
        if let Some(name) = ambiguous(change, &columns) {
            return Err(Error::query(format!(
                "{name} names more than one column of the table, so the save cannot tell which \
                 it changes"
            )));
        }
        if found.len() > 1 {
            return Err(Error::query("a row's key matches more than one row"));
        }
        match found.pop() {
            None => conflicts.push(Conflict { row, server: None }),
            Some(server) => {
                // A failure, not a conflict: a conflict offers to write
                // over what the file holds, which would still be unknown.
                if let Some(name) = unreadable(change, &columns, &server) {
                    return Ok(Applied::Failed {
                        row,
                        error: Error::query(format!(
                            "{name} holds text that is not UTF-8, so the save cannot tell \
                             whether it changed since it was loaded"
                        )),
                    });
                }
                if changed_since_loaded(change, &columns, &server.values)? {
                    conflicts.push(Conflict {
                        row,
                        server: Some(server.values),
                    });
                }
            }
        }
    }
    if !conflicts.is_empty() {
        return Ok(Applied::Conflicts(conflicts));
    }
    for (row, update) in updates.iter().enumerate() {
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
        let (_, mut found) = read(connection, &select)?;
        // One row still: a trigger the update fired can have made another
        // that the key also finds, and then which one was saved is not
        // known.
        if found.len() > 1 {
            return Err(Error::query("a row's key matches more than one row"));
        }
        rows.push(
            found
                .pop()
                .ok_or_else(|| Error::query("a saved row could not be read back"))?
                .values,
        );
    }
    Ok(Applied::Rows(rows))
}
