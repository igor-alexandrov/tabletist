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
use std::sync::{Arc, Mutex, OnceLock, PoisonError, RwLock, RwLockReadGuard, Weak};
use std::time::Instant;

use super::edit::{self, POSITION, Stamp};
use super::{Attached, literal, map_error, path_text, to_duckdb};
use crate::write::Conflict;
use crate::{
    ChangeSet, Dialect, Error, Result, RowPage, RowQuery, StopFlag, Unsaved, WriteOutcome,
};

/// The column DuckDB numbers a table's rows by, which no `SELECT *` reads.
const ROWID: &str = "rowid";

/// What every session of this app over one `.duckdb` file shares.
struct Shared {
    /// Held by each call together, and by a save alone.
    gate: RwLock<()>,
    /// The number of the file as the sessions read it: taken anew (from
    /// [`edit::next_generation`], so no number comes twice) by each save,
    /// and by each change another program made that a session found.
    saves: AtomicI64,
}

/// The `.duckdb` file at `path`, its own path (`own`, however a
/// connection spells it), as this app's sessions share it. One no session
/// holds any more is shared afresh, with a number of its own: another
/// program may have changed it since, and a page read before is of the
/// file as it was.
fn shared(own: &Path) -> Arc<Shared> {
    static FILES: OnceLock<Mutex<std::collections::HashMap<PathBuf, Weak<Shared>>>> =
        OnceLock::new();
    let mut files = FILES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(shared) = files.get(own).and_then(Weak::upgrade) {
        return shared;
    }
    let shared = Arc::new(Shared {
        gate: RwLock::new(()),
        saves: AtomicI64::new(edit::next_generation()),
    });
    files.retain(|_, file| file.strong_count() > 0);
    files.insert(own.to_owned(), Arc::downgrade(&shared));
    shared
}

/// The path a file's gate is kept by: its own, however it is spelled.
fn own_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

/// The `.duckdb` files a session opens, held for reading while it attaches
/// them: one another session is saving is attached once that save is done.
pub(super) struct Opening(Vec<Arc<Shared>>);

impl Opening {
    pub(super) fn of<'a>(paths: impl Iterator<Item = &'a Path>) -> Self {
        let mut owns: Vec<PathBuf> = paths.map(own_path).collect();
        // In their own paths' order, each once, as `reading` takes them.
        owns.sort();
        owns.dedup();
        Self(owns.iter().map(|own| shared(own)).collect())
    }

    pub(super) fn hold(&self) -> Vec<RwLockReadGuard<'_, ()>> {
        self.0
            .iter()
            .map(|file| file.gate.read().unwrap_or_else(PoisonError::into_inner))
            .collect()
    }
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
    /// 0 again where attaching it read-only again failed.
    seen: AtomicI64,
    /// The file as it was when the session last attached it.
    stamp: Mutex<Option<Stamp>>,
}

impl Held {
    pub(super) fn new(name: &str, path: &Path) -> Self {
        let own = own_path(path);
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

/// Where a save is, for a test to step in at (`tests::stepping_in`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Moment {
    /// Checked, about to attach the file to write.
    WriteAttach,
    /// Written, about to read a changed row back.
    ReadBack,
    /// About to commit.
    Commit,
}

/// What a test runs where a save is.
#[cfg(test)]
type Step = Box<dyn FnMut(Moment)>;

#[cfg(test)]
thread_local! {
    static STEP_IN: std::cell::RefCell<Option<Step>> =
        const { std::cell::RefCell::new(None) };
}

/// A save at `moment`: a test may step in there, nothing else does.
fn at(moment: Moment) {
    #[cfg(test)]
    STEP_IN.with(|hook| {
        if let Some(hook) = hook.borrow_mut().as_mut() {
            hook(moment);
        }
    });
    #[cfg(not(test))]
    let _ = moment;
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

/// Attaches `held` read-only again, as the file was after save `saves`.
/// Where that fails the file is left detached, so the attachment is of no
/// save: the next call tries again. The file is numbered anew too: what
/// is at its name by then may be another file, which no page read before,
/// its rows found by `rowid`, is saved to.
fn read_only_again(connection: &duckdb::Connection, held: &Held, saves: i64) {
    let seen = match attach(connection, held, false) {
        Ok(()) => saves,
        Err(_) => {
            held.shared
                .saves
                .store(edit::next_generation(), Ordering::SeqCst);
            0
        }
    };
    held.seen.store(seen, Ordering::SeqCst);
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
            // Another session may have found it first: its number holds.
            let fresh = edit::next_generation();
            saves = match held.shared.saves.compare_exchange(
                saves,
                fresh,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => fresh,
                Err(now) => now,
            };
        }
        if seen != saves {
            read_only_again(connection, held, saves);
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

/// A page of `query`'s table, a `.duckdb` file's with no primary key,
/// each row with its `rowid` as its place: the rows of the file as the
/// session's attachment of it is (`held`).
pub(super) fn page(
    connection: &duckdb::Connection,
    query: &RowQuery,
    attached: &[Attached],
    held: &Held,
    limit: usize,
) -> Result<RowPage> {
    let catalog = super::columns(connection, &query.object, attached)?;
    edit::page_with(connection, query, catalog, ROWID, limit, held.generation())
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
    // A keyed row found by its key alone: one found by other columns may
    // be more than one row.
    let by_key = |found: &[(String, crate::Value)]| {
        found.len() == key.len() && found.iter().all(|(column, _)| key.contains(column))
    };
    let keys = changes.rows.iter().map(|row| &row.key);
    if !key.is_empty()
        && !keys
            .chain(changes.deletes.iter().map(|row| &row.key))
            .all(|found| by_key(found))
    {
        return Err(Error::query(format!(
            "{} finds its rows by its primary key, which these rows aren't found by",
            changes.object.name
        )));
    }
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
    let back = || read_only_again(connection, held, saves);
    at(Moment::WriteAttach);
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
    // Replaced between the check and the attach: not written, and every
    // session reads the file again.
    if Stamp::of(&held.path).ok() != was {
        let next = edit::next_generation();
        held.shared.saves.store(next, Ordering::SeqCst);
        read_only_again(connection, held, next);
        return Err(unsaved(
            Unsaved::Changed(label.clone()),
            format!("{label} changed on disk since it was opened"),
        ));
    }
    let done = connection
        .execute_batch("BEGIN TRANSACTION")
        .map_err(map_error)
        .and_then(|()| changed(connection, &changes, !key.is_empty(), stop));
    at(Moment::Commit);
    let outcome = match done {
        Ok(WriteOutcome::Written { .. }) if stop.is_stopped() => {
            let _ = connection.execute_batch("ROLLBACK");
            back();
            return Err(Error::Cancelled);
        }
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
    let next = edit::next_generation();
    held.shared.saves.store(next, Ordering::SeqCst);
    read_only_again(connection, held, next);
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
            Ok(1) => {}
            Ok(many) => {
                let error = Error::query(format!("{many} rows have this row's key"));
                return Ok(WriteOutcome::Failed { row, error });
            }
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
            Ok(1) => {}
            Ok(many) => {
                let error = Error::query(format!("{many} rows have this row's key"));
                return Ok(WriteOutcome::FailedDelete { delete, error });
            }
            Err(error) => return Ok(WriteOutcome::FailedDelete { delete, error }),
        }
    }
    stopped()?;
    let mut rows = Vec::with_capacity(changes.rows.len());
    for change in changes.rows.iter().filter(|_| keyed) {
        at(Moment::ReadBack);
        stopped()?;
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

    /// The Bookshop's `bookshop.duckdb` in a folder of its own, attached
    /// read-only as `bookshop` by a session of its own.
    fn bookshop() -> (duckdb::Connection, Held, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        crate::fixtures::write_bookshop_files(dir.path()).unwrap();
        let connection = duckdb::Connection::open_in_memory().unwrap();
        let held = Held::new("bookshop", &dir.path().join("bookshop.duckdb"));
        read_only_again(&connection, &held, held.shared.saves.load(Ordering::SeqCst));
        assert_ne!(held.generation(), 0);
        (connection, held, dir)
    }

    /// A save that runs `step` wherever it is.
    fn stepping_in(step: impl FnMut(Moment) + 'static) -> impl Drop {
        struct Out;
        impl Drop for Out {
            fn drop(&mut self) {
                STEP_IN.with(|hook| *hook.borrow_mut() = None);
            }
        }
        STEP_IN.with(|hook| *hook.borrow_mut() = Some(Box::new(step)));
        Out
    }

    /// Order item `id`'s quantity, as the session reads the file.
    fn quantity(connection: &duckdb::Connection, id: i64) -> Option<i64> {
        connection
            .query_row(
                "SELECT quantity FROM bookshop.order_items WHERE id = ?",
                [id],
                |row| row.get(0),
            )
            .ok()
    }

    /// Order item 1's quantity set to 9, and order item 2 deleted.
    fn changes(connection: &duckdb::Connection) -> ChangeSet {
        let key = |id: i64| vec![("id".to_owned(), crate::Value::Int(id))];
        ChangeSet {
            object: crate::ObjectRef::new("bookshop", "order_items"),
            inserts: Vec::new(),
            rows: vec![crate::RowChange {
                key: key(1),
                set: vec![crate::CellChange {
                    column: "quantity".to_owned(),
                    type_name: "INTEGER".to_owned(),
                    loaded: crate::Value::Int(quantity(connection, 1).unwrap()),
                    new: crate::NewValue::Text("9".to_owned()),
                }],
            }],
            deletes: vec![crate::RowDelete { key: key(2) }],
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_file_replaced_just_before_a_save_opens_it_to_write_is_not_written() {
        let (connection, held, dir) = bookshop();
        let path = dir.path().join("bookshop.duckdb");
        let changes = changes(&connection);
        let saves = held.shared.saves.load(Ordering::SeqCst);
        // Another program puts a file of its own at the name: the same
        // rows, order item 1's quantity 4.
        let replace = {
            let (path, dir) = (path.clone(), dir.path().to_owned());
            move |moment| {
                if moment != Moment::WriteAttach {
                    return;
                }
                let other = dir.join("other.duckdb");
                std::fs::copy(&path, &other).unwrap();
                let duck = duckdb::Connection::open(&other).unwrap();
                duck.execute_batch("UPDATE order_items SET quantity = 4 WHERE id = 1; CHECKPOINT")
                    .unwrap();
                drop(duck);
                std::fs::rename(&other, &path).unwrap();
            }
        };
        let _in = stepping_in(replace);
        let error = save(&connection, &held, &changes, &StopFlag::new()).unwrap_err();
        let Error::Query { named, .. } = &error else {
            panic!("{error:?}");
        };
        assert_eq!(
            named.unsaved,
            Some(Unsaved::Changed("bookshop.duckdb".into()))
        );
        // The other program's file, unwritten, attached read-only again and
        // numbered anew.
        assert_eq!(quantity(&connection, 1), Some(4));
        assert!(quantity(&connection, 2).is_some());
        assert!(
            connection
                .execute_batch("DELETE FROM bookshop.order_items")
                .is_err()
        );
        assert_ne!(held.shared.saves.load(Ordering::SeqCst), saves);
        assert_eq!(held.generation(), held.shared.saves.load(Ordering::SeqCst));
    }

    #[test]
    fn a_save_stopped_after_its_changes_ran_writes_nothing() {
        for stopped_at in [Moment::ReadBack, Moment::Commit] {
            let (connection, held, _dir) = bookshop();
            let changes = changes(&connection);
            let before = quantity(&connection, 1);
            let stop = StopFlag::new();
            let _in = stepping_in({
                let stop = stop.clone();
                move |moment| {
                    if moment == stopped_at {
                        stop.stop();
                    }
                }
            });
            let outcome = save(&connection, &held, &changes, &stop);
            assert!(
                matches!(outcome, Err(Error::Cancelled)),
                "{stopped_at:?}: {outcome:?}"
            );
            // Undone, and the file attached read-only again.
            assert_eq!(quantity(&connection, 1), before, "{stopped_at:?}");
            assert!(quantity(&connection, 2).is_some(), "{stopped_at:?}");
            assert!(
                connection
                    .execute_batch("DELETE FROM bookshop.order_items")
                    .is_err(),
                "{stopped_at:?}"
            );
            assert_ne!(held.generation(), 0);
        }
    }

    #[test]
    fn a_session_opening_a_file_waits_for_a_save_of_it_however_it_spells_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.duckdb");
        std::fs::write(&path, b"").unwrap();
        let spelled = dir.path().join(".").join("bookshop.duckdb");
        let opening = Opening::of([spelled.as_path(), path.as_path()].into_iter());
        let held = opening.hold();
        // Once, though it is named twice.
        assert_eq!(held.len(), 1);
        let file = Held::new("shop", &path);
        assert!(file.shared.gate.try_write().is_err());
        drop(held);
        assert!(file.shared.gate.try_write().is_ok());
    }

    #[test]
    fn a_file_no_session_holds_is_shared_afresh() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.duckdb");
        let one = Held::new("shop", &path);
        let saves = one.shared.saves.load(Ordering::SeqCst);
        let two = Held::new("shop", &path);
        assert_eq!(two.shared.saves.load(Ordering::SeqCst), saves);
        drop((one, two));
        let again = Held::new("shop", &path);
        assert_ne!(again.shared.saves.load(Ordering::SeqCst), saves);
    }

    #[test]
    fn a_file_that_cannot_be_attached_again_is_tried_again_on_the_next_call() {
        let connection = duckdb::Connection::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let held = Held::new("shop", &dir.path().join("missing.duckdb"));
        held.seen.store(3, Ordering::SeqCst);
        // A save refused, and the file then not attached read-only either:
        // not attached as of the save it was, so the next call attaches it.
        let saves = held.shared.saves.load(Ordering::SeqCst);
        read_only_again(&connection, &held, saves);
        assert_eq!(held.generation(), 0);
        // And the file numbered anew: what is at its name when it is
        // attached again may be another file, and a page of it before, found
        // by `rowid`, is not saved to it.
        assert_ne!(held.shared.saves.load(Ordering::SeqCst), saves);
    }

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
