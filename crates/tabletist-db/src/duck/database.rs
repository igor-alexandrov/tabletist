//! A `.duckdb` file's tables, edited in place. Every session attaches the
//! file read-only, so other programs can still read it, and none can write
//! it. A save takes the file alone: it attaches it to write, changes it in
//! one transaction, and attaches it read-only again.
//!
//! Inside one app DuckDB's own lock keeps nothing apart: two sessions are
//! one process, which may hold the file twice. So the sessions over a file
//! share a gate. Each call holds it with the others; a save holds it alone,
//! and after it every other session attaches the file again before it
//! reads it, as the save left it.
//!
//! A table with a primary key is saved by it. One without is saved by
//! DuckDB's `rowid`, which a save that deletes rows numbers again: a
//! page's rows carry the number of the save they were read after
//! ([`crate::Positions::copy`]), and a later save refuses them.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, RwLock, RwLockReadGuard};
use std::time::Instant;

use super::edit::{self, EDITS, POSITION, Stamp};
use super::{literal, map_error, path_text, to_duckdb};
use crate::write::Conflict;
use crate::{ChangeSet, Dialect, Error, ObjectRef, Result, StopFlag, Unsaved, WriteOutcome};

/// The column DuckDB numbers a table's rows by, which no `SELECT *` reads.
const ROWID: &str = "rowid";

/// What every session of this app over one `.duckdb` file shares.
struct Shared {
    /// Held by each call together, and by a save alone.
    gate: RwLock<()>,
    /// The number of the save that wrote the file last, from 1.
    saves: AtomicI64,
}

/// The `.duckdb` file at `path`, its own path (`own`, however a
/// connection spells it), as this app's sessions share it.
fn shared(own: &Path) -> Arc<Shared> {
    static FILES: OnceLock<Mutex<std::collections::HashMap<PathBuf, Arc<Shared>>>> =
        OnceLock::new();
    let mut files = FILES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    Arc::clone(files.entry(own.to_owned()).or_insert_with(|| {
        Arc::new(Shared {
            gate: RwLock::new(()),
            saves: AtomicI64::new(1),
        })
    }))
}

/// A `.duckdb` file as a session attached it.
pub(super) struct Held {
    /// The schema it is attached as.
    pub(super) name: String,
    path: PathBuf,
    /// Its own path, which its gate is the file's by: what sessions sort
    /// their files by, so two that spell one file two ways take its gate
    /// in the same order.
    own: PathBuf,
    shared: Arc<Shared>,
    /// The save the session's attachment is of. 0 until the first call
    /// attaches it again, under the gate: the session opened without it.
    seen: AtomicI64,
    /// The file as it was when the session last attached it.
    stamp: Mutex<Option<Stamp>>,
}

impl Held {
    pub(super) fn new(name: &str, path: &Path) -> Self {
        let own = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
        Self {
            name: name.to_owned(),
            path: path.to_owned(),
            shared: shared(&own),
            own,
            seen: AtomicI64::new(0),
            stamp: Mutex::new(None),
        }
    }

    /// The file's own path, which sessions sort theirs by.
    pub(super) fn own(&self) -> &Path {
        &self.own
    }

    /// The save the session's attachment is of: what a page's rows found
    /// by `rowid` are of.
    pub(super) fn generation(&self) -> i64 {
        self.seen.load(Ordering::SeqCst)
    }
}

/// Attaches `held` again under its name: read-only, or to write.
fn attach(connection: &duckdb::Connection, held: &Held, write: bool) -> Result<()> {
    let name = Dialect::DuckDb.ident(&held.name);
    let path = literal(&path_text(&held.path)?);
    let mode = if write { "" } else { " (READ_ONLY)" };
    connection
        .execute_batch(&format!(
            "DETACH DATABASE IF EXISTS {name}; ATTACH {path} AS {name}{mode}"
        ))
        .map_err(map_error)?;
    if !write {
        *held.stamp.lock().unwrap_or_else(PoisonError::into_inner) = Stamp::of(&held.path).ok();
    }
    Ok(())
}

/// Holds each of `databases`, in their order (their own paths'), for reading
/// while a call runs: no save writes one meanwhile. A file the session
/// reaches by two names has its gate taken once: a second read of one lock
/// in one thread can wait for good behind a save. One another session
/// saved since this one attached it is attached again first, and so is one
/// another program changed since (replaced it, most often): that counts as
/// a save, so every session reads it again and no page of it before is
/// saved. One that cannot be attached is left as it is, and tried again on
/// the next call.
pub(super) fn reading<'a>(
    connection: &duckdb::Connection,
    databases: &'a [Held],
) -> Vec<RwLockReadGuard<'a, ()>> {
    let mut guards = Vec::with_capacity(databases.len());
    for (at, held) in databases.iter().enumerate() {
        let taken = at > 0 && databases[at - 1].own == held.own;
        if !taken {
            guards.push(
                held.shared
                    .gate
                    .read()
                    .unwrap_or_else(PoisonError::into_inner),
            );
        }
        let mut saves = held.shared.saves.load(Ordering::SeqCst);
        let seen = held.seen.load(Ordering::SeqCst);
        let was = *held.stamp.lock().unwrap_or_else(PoisonError::into_inner);
        if seen == saves && was.is_some() && Stamp::of(&held.path).ok() != was {
            saves = held.shared.saves.fetch_add(1, Ordering::SeqCst) + 1;
        }
        if seen != saves && attach(connection, held, false).is_ok() {
            held.seen.store(saves, Ordering::SeqCst);
        }
    }
    guards
}

/// The columns of the primary key of the table `table` of the `.duckdb`
/// file attached as `database`, in its order: none where it has none.
/// `None` where `table` is not a table there: a view.
pub(super) fn table_key(
    connection: &duckdb::Connection,
    database: &str,
    table: &str,
) -> Result<Option<Vec<String>>> {
    let tables: i64 = connection
        .query_row(
            "SELECT count(*) FROM duckdb_tables() \
             WHERE database_name = ? AND schema_name = 'main' AND table_name = ?",
            [database, table],
            |row| row.get(0),
        )
        .map_err(map_error)?;
    if tables == 0 {
        return Ok(None);
    }
    primary_key(connection, database, table).map(Some)
}

/// Whether the table `table` of the `.duckdb` file attached as `database`
/// can be found by DuckDB's `rowid`: not where a column of its own is
/// named `rowid`, which `rowid` would read instead (two rows could then be
/// one place), or is named as the place its page reads it as.
pub(super) fn by_rowid(
    connection: &duckdb::Connection,
    database: &str,
    table: &str,
) -> Result<bool> {
    let shadowing: i64 = connection
        .query_row(
            "SELECT count(*) FROM information_schema.columns \
             WHERE table_catalog = ? AND table_schema = 'main' AND table_name = ? \
             AND lower(column_name) IN (?, ?)",
            [database, table, ROWID, &POSITION.to_lowercase()],
            |row| row.get(0),
        )
        .map_err(map_error)?;
    Ok(shadowing == 0)
}

/// The columns of the primary key of the table `table` of the `.duckdb`
/// file attached as `database`, in its order: none where it has none.
fn primary_key(
    connection: &duckdb::Connection,
    database: &str,
    table: &str,
) -> Result<Vec<String>> {
    let found: Option<duckdb::types::Value> = connection
        .query_row(
            "SELECT constraint_column_names FROM duckdb_constraints() \
             WHERE database_name = ? AND schema_name = 'main' AND table_name = ? \
             AND constraint_type = 'PRIMARY KEY'",
            [database, table],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|error| match error {
            duckdb::Error::QueryReturnedNoRows => Ok(None),
            error => Err(map_error(error)),
        })?;
    Ok(match found {
        Some(duckdb::types::Value::List(names)) => names
            .into_iter()
            .filter_map(|name| match name {
                duckdb::types::Value::Text(name) => Some(name),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    })
}

/// Where a page of `object`, a `.duckdb` file's table with no primary
/// key, is read from: a view of it in the edits database that reads each
/// row's `rowid` as its place. Made outside the page's read-only
/// transaction, which may make nothing.
pub(super) fn placed(connection: &duckdb::Connection, object: &ObjectRef) -> Result<ObjectRef> {
    let from = ObjectRef {
        schema: EDITS.to_owned(),
        name: format!("{}.{}", object.schema, object.name),
    };
    let dialect = Dialect::DuckDb;
    connection
        .execute_batch(&format!(
            "CREATE OR REPLACE VIEW {} AS SELECT {ROWID} AS {}, * FROM {}",
            dialect.qualified(&from),
            dialect.quote_ident(POSITION),
            dialect.qualified(object),
        ))
        .map_err(map_error)?;
    Ok(from)
}

/// Saves `changes` to `held`'s table `changes.object` in place, alone:
/// every other session waits, and attaches the file again after. A save
/// the file refuses writes nothing, and says why.
pub(super) fn save(
    connection: &duckdb::Connection,
    held: &Held,
    changes: &ChangeSet,
    stop: &StopFlag,
) -> Result<WriteOutcome> {
    let _alone = held
        .shared
        .gate
        .write()
        .unwrap_or_else(PoisonError::into_inner);
    let started = Instant::now();
    let label = crate::files::file_name(&held.path);
    let unsaved = |why: Unsaved, message: String| {
        let mut error = Error::query(message);
        if let Error::Query { named, .. } = &mut error {
            named.unsaved = Some(why);
        }
        error
    };
    // Attached again first where another session saved since.
    let saves = held.shared.saves.load(Ordering::SeqCst);
    if held.seen.load(Ordering::SeqCst) != saves {
        attach(connection, held, false)?;
        held.seen.store(saves, Ordering::SeqCst);
    }
    let key = primary_key(connection, &held.name, &changes.object.name)?;
    // Rows found by `rowid` are of the file as one save left it: another
    // save since may have numbered them again.
    if key.is_empty() && !by_rowid(connection, &held.name, &changes.object.name)? {
        return Err(Error::query(format!(
            "{} has a column of its own named rowid: its rows can't be found",
            changes.object.name
        )));
    }
    let changes = match key.is_empty() {
        true => edit::own_keys(changes, saves, ROWID).ok_or_else(|| {
            unsaved(
                Unsaved::Reread(label.clone()),
                format!("{label} was read again since these rows were"),
            )
        })?,
        false => changes.clone(),
    };
    let was = *held.stamp.lock().unwrap_or_else(PoisonError::into_inner);
    if was.is_none() || Stamp::of(&held.path).ok() != was {
        return Err(unsaved(
            Unsaved::Changed(label.clone()),
            format!("{label} changed on disk since it was opened"),
        ));
    }
    if edit::protected(&held.path) {
        return Err(unsaved(
            Unsaved::Protected(label.clone()),
            format!("{label} is read-only"),
        ));
    }
    // Read-only again whatever comes of it.
    let back = || {
        if attach(connection, held, false).is_ok() {
            held.seen.store(saves, Ordering::SeqCst);
        }
    };
    if let Err(error) = attach(connection, held, true) {
        back();
        let said = super::plain(&error);
        // DuckDB's whole words: on Windows the holder's PID is on a line
        // after the first.
        return Err(match held_by(&error.to_string(), std::process::id()) {
            Some(Holder::Other) => unsaved(
                Unsaved::Busy(label.clone()),
                format!("{label} is open in another program"),
            ),
            Some(Holder::Here) => unsaved(
                Unsaved::OpenHere(label.clone()),
                format!("{label} is open in another session of this app"),
            ),
            None => unsaved(
                Unsaved::Failed(label.clone(), said.clone()),
                format!("{label} could not be written: {said}"),
            ),
        });
    }
    let done = connection
        .execute_batch("BEGIN TRANSACTION")
        .map_err(map_error)
        .and_then(|()| changed(connection, &changes, !key.is_empty(), stop));
    let outcome = match done {
        Ok(outcome @ WriteOutcome::Written { .. }) => {
            match connection.execute_batch("COMMIT").map_err(map_error) {
                Ok(()) => outcome,
                Err(error) => {
                    let _ = connection.execute_batch("ROLLBACK");
                    back();
                    return Err(error);
                }
            }
        }
        other => {
            let _ = connection.execute_batch("ROLLBACK");
            back();
            return other;
        }
    };
    // Written: every session, this one too, reads the file as it is now.
    let next = held.shared.saves.fetch_add(1, Ordering::SeqCst) + 1;
    if attach(connection, held, false).is_ok() {
        held.seen.store(next, Ordering::SeqCst);
    }
    Ok(match outcome {
        WriteOutcome::Written {
            inserted,
            rows,
            gone,
            ..
        } => WriteOutcome::Written {
            inserted,
            rows,
            gone,
            elapsed: started.elapsed(),
        },
        other => other,
    })
}

/// Who holds a file DuckDB could not open to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Holder {
    Other,
    /// This app, process `pid`: on Windows another of its sessions keeps
    /// the file from being opened to write.
    Here,
}

/// Who holds the file, from DuckDB's words `said` where they say another
/// process has it: its lock (`Could not set lock on file …: Conflicting
/// lock is held in … (PID 4242)`), or on Windows its handle (`… is being
/// used by another process. File is already open in … (PID 4242)`). This
/// app is `pid`. `None` where they say neither.
fn held_by(said: &str, pid: u32) -> Option<Holder> {
    let held =
        said.contains("Could not set lock") || said.contains("being used by another process");
    if !held {
        return None;
    }
    Some(match said.contains(&format!("(PID {pid})")) {
        true => Holder::Here,
        false => Holder::Other,
    })
}

/// Changes the table as `changes` say, in the transaction `save` began,
/// and reads each changed row back by its key where the table is `keyed`.
/// A table found by `rowid` hands none back: DuckDB moves a row it updates
/// a list or a unique column of to another `rowid` until the commit, and
/// the page is read again after every save of it.
fn changed(
    connection: &duckdb::Connection,
    changes: &ChangeSet,
    keyed: bool,
    stop: &StopFlag,
) -> Result<WriteOutcome> {
    let object = &changes.object;
    let dialect = Dialect::DuckDb;
    let stopped = || {
        if stop.is_stopped() {
            return Err(Error::Cancelled);
        }
        Ok(())
    };
    let run = |sql: &crate::Sql| -> Result<usize> {
        let params = duckdb::params_from_iter(sql.params.iter().map(to_duckdb));
        connection.execute(&sql.text, params).map_err(map_error)
    };
    // A keyed row is read first, before anything is written, and compared
    // with what its page loaded in each column the save changes: another
    // session of this app may have saved it since. A row found by `rowid`
    // is of the save its page was read after, which `save` checked.
    if keyed {
        let mut conflicts = Vec::new();
        for (row, change) in changes.rows.iter().enumerate() {
            stopped()?;
            let select = dialect.select_row(object, &change.key, false);
            let params = select.params.iter().map(to_duckdb).collect();
            let (columns, mut found, _) = super::rows_of(connection, &select.text, params, 2)?;
            let names: Vec<String> = columns.into_iter().map(|column| column.name).collect();
            let server = found.pop();
            let differs = match &server {
                None => true,
                Some(server) => crate::write::changed_since_loaded(change, &names, server)?,
            };
            if differs {
                conflicts.push(Conflict { row, server });
            }
        }
        if !conflicts.is_empty() {
            return Ok(WriteOutcome::Conflicts(conflicts));
        }
    }
    for (insert, row) in changes.inserts.iter().enumerate() {
        stopped()?;
        let done = dialect.insert_row(object, row).and_then(|statement| {
            let params = duckdb::params_from_iter(statement.sql.params.iter().map(to_duckdb));
            let mut prepared = connection.prepare(&statement.sql.text).map_err(map_error)?;
            let mut rows = prepared.query(params).map_err(map_error)?;
            while rows.next().map_err(map_error)?.is_some() {}
            Ok(())
        });
        if let Err(error) = done {
            return Ok(WriteOutcome::FailedInsert { insert, error });
        }
    }
    let mut conflicts = Vec::new();
    for (row, change) in changes.rows.iter().enumerate() {
        stopped()?;
        match dialect
            .update_row(object, change)
            .and_then(|update| run(&update.sql))
        {
            Ok(0) => conflicts.push(Conflict { row, server: None }),
            Ok(_) => {}
            Err(error) => return Ok(WriteOutcome::Failed { row, error }),
        }
    }
    if !conflicts.is_empty() {
        return Ok(WriteOutcome::Conflicts(conflicts));
    }
    let mut gone = Vec::new();
    for (delete, row) in changes.deletes.iter().enumerate() {
        stopped()?;
        match dialect
            .delete_row(object, row)
            .and_then(|deletion| run(&deletion.sql))
        {
            Ok(0) => gone.push(delete),
            Ok(_) => {}
            Err(error) => return Ok(WriteOutcome::FailedDelete { delete, error }),
        }
    }
    stopped()?;
    let mut rows = Vec::with_capacity(changes.rows.len());
    for change in changes.rows.iter().filter(|_| keyed) {
        let select = dialect.select_row(object, &change.key, false);
        let params = select.params.iter().map(to_duckdb).collect();
        let (_, mut found, _) = super::rows_of(connection, &select.text, params, 2)?;
        if found.len() != 1 {
            return Err(Error::query("a changed row was not read back"));
        }
        rows.extend(found.pop());
    }
    Ok(WriteOutcome::Written {
        inserted: vec![None; changes.inserts.len()],
        rows,
        gone,
        elapsed: std::time::Duration::ZERO,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_another_process_holds_is_told_from_one_this_app_holds() {
        let unix = "IO Error: Could not set lock on file \"/data/bookshop.duckdb\": Conflicting \
                    lock is held in /usr/bin/duckdb (PID 4242) by user reader.";
        assert_eq!(held_by(unix, 7), Some(Holder::Other));
        assert_eq!(held_by(unix, 4242), Some(Holder::Here));
        let windows = "IO Error: Cannot open file \"C:\\Data\\bookshop.duckdb\": The process \
                       cannot access the file because it is being used by another process.\r\n\n\
                       File is already open in \nC:\\Program Files\\Tabletist\\tabletist.exe \
                       (PID 6212)";
        assert_eq!(held_by(windows, 6212), Some(Holder::Here));
        assert_eq!(held_by(windows, 1), Some(Holder::Other));
        assert_eq!(held_by("IO Error: No space left on device", 1), None);
        // A save reads the error's whole words: its first line has no PID.
        let error = Error::query(windows);
        assert_eq!(held_by(&error.to_string(), 6212), Some(Holder::Here));
        assert_eq!(
            held_by(&super::super::plain(&error), 6212),
            Some(Holder::Other)
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_file_reached_by_two_names_has_its_gate_taken_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.duckdb");
        duckdb::Connection::open(&path).unwrap();
        let alias = dir.path().join("alias.duckdb");
        std::os::unix::fs::symlink(&path, &alias).unwrap();
        let databases = [Held::new("alias", &alias), Held::new("bookshop", &path)];
        assert_eq!(databases[0].own(), databases[1].own());
        let connection = duckdb::Connection::open_in_memory().unwrap();
        assert_eq!(reading(&connection, &databases).len(), 1);
    }
}
