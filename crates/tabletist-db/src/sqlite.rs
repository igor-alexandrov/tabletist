//! SQLite, opened read-only unless the connection is writable. rusqlite is
//! blocking, so every call runs on tokio's blocking pool. What a script or
//! a filter may do is SQLite's to refuse: it asks the connection's
//! authorizer whenever it prepares their text (see `fence`).

use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rusqlite::config::DbConfig;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::{ErrorCode, OpenFlags, OptionalExtension};

use crate::adapter::Adapter;
use crate::script::{cancelled_commit, cleanup_failed};
use crate::{
    Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, Dialect,
    Error, ForeignKeyInfo, IndexInfo, MAX_LISTED, Named, ObjectInfo, ObjectKind, ObjectRef, Result,
    RowPage, RowQuery, ScriptEnd, ScriptMode, ScriptOutcome, StatementOutcome, StatementResult,
    StopFlag, Structure, Value, ValueKind, WriteOutcome, column_class,
};

mod fence;
mod write;

use fence::{Fence, Fences, authorize};

/// An open SQLite database.
pub(crate) struct Conn {
    inner: Arc<Mutex<rusqlite::Connection>>,
    interrupt: Arc<rusqlite::InterruptHandle>,
    fences: Fences,
    /// `main`'s journal mode as the session found it, which a save puts
    /// back.
    journal_mode: String,
}

/// What a failure with this extended result `code` and `message` is of.
/// SQLite's messages are its own English and no setting's, so what follows
/// the colon is read as it stands.
fn named(code: i32, message: &str) -> Named {
    let Some((_, what)) = message.split_once(": ") else {
        return Named::default();
    };
    // `table.column`, a column for each part.
    let columns = || -> Vec<String> {
        what.split(", ")
            .map(|part| part.rsplit('.').next().unwrap_or(part).to_owned())
            .collect()
    };
    match code {
        // SQLITE_CONSTRAINT_UNIQUE, _PRIMARYKEY and _ROWID. An index over
        // an expression is named in the place of its columns.
        2067 | 1555 | 2579 => match what.strip_prefix("index '") {
            Some(index) => Named {
                constraint: Some(index.trim_end_matches('\'').to_owned()),
                ..Named::default()
            },
            None => Named {
                columns: columns(),
                ..Named::default()
            },
        },
        // SQLITE_CONSTRAINT_NOTNULL.
        1299 => Named {
            columns: columns(),
            ..Named::default()
        },
        // SQLITE_CONSTRAINT_CHECK: the constraint's name, or its text
        // where it has none.
        275 => Named {
            constraint: Some(what.to_owned()),
            ..Named::default()
        },
        _ => Named::default(),
    }
}

/// Maps rusqlite's errors onto ours.
pub(crate) fn map_error(error: rusqlite::Error) -> Error {
    match &error {
        rusqlite::Error::SqliteFailure(failure, message) => {
            let message = message.clone().unwrap_or_else(|| error.to_string());
            match failure.code {
                ErrorCode::OperationInterrupted => Error::Cancelled,
                ErrorCode::CannotOpen | ErrorCode::NotADatabase => Error::Connect(message),
                _ => Error::Query {
                    code: Some(failure.extended_code.to_string()),
                    named: Box::new(named(failure.extended_code, &message)),
                    message,
                    detail: None,
                    hint: None,
                },
            }
        }
        _ => Error::query(error.to_string()),
    }
}

/// Interrupts the connection's running statement when dropped, unless
/// disarmed first.
struct InterruptOnDrop(Option<Arc<rusqlite::InterruptHandle>>);

impl InterruptOnDrop {
    fn disarm(mut self) {
        self.0 = None;
    }
}

impl Drop for InterruptOnDrop {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.interrupt();
        }
    }
}

/// Whether the session still refuses writes. The authorizer denies a
/// script every way of turning `query_only` off; this is the check behind
/// it, since on a writable handle nothing else would stop the script's next
/// statement.
fn still_query_only(connection: &rusqlite::Connection) -> Result<bool> {
    connection
        .query_row("PRAGMA query_only", [], |row| row.get::<_, i64>(0))
        .map(|on| on == 1)
        .map_err(map_error)
}

/// Starts a script's transaction. A run that writes lifts `query_only`
/// for its length and takes the write lock at once, so a database another
/// program holds is found before any statement ran. Both are the app's own
/// statements, outside the fence.
fn begin(connection: &rusqlite::Connection, mode: ScriptMode) -> Result<()> {
    let sql = match mode {
        ScriptMode::ReadOnly => "BEGIN DEFERRED",
        ScriptMode::Write => "PRAGMA query_only = OFF; BEGIN IMMEDIATE",
    };
    connection.execute_batch(sql).map_err(map_error)
}

/// Runs `texts` in one transaction, each behind `Fence::Script`, which
/// denies it leaving the transaction or setting `query_only`.
///
/// A read-only run sits between `BEGIN` and a `ROLLBACK` that always
/// happens. Behind the fence, a run that finds `query_only` off, before a
/// statement or at its end, or its transaction gone before a statement,
/// ends with `LeftReadOnly` after that rollback.
///
/// A run that writes has `query_only` off from `BEGIN` to its end, which
/// is a `COMMIT` when every statement succeeded (see `end_write`). One
/// that finds its transaction gone before a statement ends with
/// `LeftTransaction`.
fn script(
    connection: &rusqlite::Connection,
    fences: &Fences,
    texts: &[String],
    limit: usize,
    mode: ScriptMode,
    stop: &StopFlag,
) -> Result<ScriptOutcome> {
    let mut outcome = ScriptOutcome::default();
    if stop.is_stopped() {
        outcome.stopped = true;
        return Ok(outcome);
    }
    // A cancel (the session's interrupt) can land while BEGIN runs; that
    // ends the run with no results.
    match begin(connection, mode) {
        Err(Error::Cancelled) => {
            outcome.stopped = true;
            // The interrupt may have left a transaction open.
            stop.finish();
            end_transaction(connection).map_err(|error| cleanup_failed(mode, &error))?;
            return Ok(outcome);
        }
        // A database that is locked: the run's error, in SQLite's words.
        // `query_only` was lifted before the lock was asked for, so the
        // session's settings are put back; then it is as it was.
        Err(error) if mode == ScriptMode::Write => {
            stop.finish();
            end_transaction(connection).map_err(|error| cleanup_failed(mode, &error))?;
            return Err(error);
        }
        Err(error) => return Err(error),
        Ok(()) => {}
    }
    // SQLite calls this every 1000 virtual machine steps; `true` interrupts
    // the running statement, which fails with SQLITE_INTERRUPT (Cancelled).
    let watching = stop.clone();
    connection.progress_handler(1_000, Some(move || watching.is_stopped()));
    let ran = statements(connection, fences, texts, limit, mode, stop, &mut outcome);
    // Removed before the cleanup, so a stop cannot interrupt it.
    connection.progress_handler(0, None::<fn() -> bool>);
    if mode == ScriptMode::Write {
        return end_write(connection, stop, ran, outcome);
    }
    stop.finish();
    // Asked before the rollback, which puts the setting back: a last
    // statement that turned it off must not pass unseen. A cancel can land
    // on the question, so it is asked once more; no answer counts as left.
    let left = ran.is_ok()
        && !still_query_only(connection)
            .or_else(|_| still_query_only(connection))
            .unwrap_or(false);
    let ended = end_transaction(connection);
    ran?;
    ended.map_err(|error| cleanup_failed(ScriptMode::ReadOnly, &error))?;
    if left {
        return Err(Error::LeftReadOnly);
    }
    Ok(outcome)
}

/// Ends a run that writes. Its transaction is committed when every
/// statement succeeded and no stop came before this point, and rolled back
/// otherwise; the session's settings are put back either way, `query_only`
/// first. From `finish` on the backend sends no cancel, so the run ends as
/// its commit ends.
fn end_write(
    connection: &rusqlite::Connection,
    stop: &StopFlag,
    ran: Result<()>,
    mut outcome: ScriptOutcome,
) -> Result<ScriptOutcome> {
    let succeeded = ran.is_ok() && outcome.succeeded();
    // The backend is told first, so a stop it sets from here on sends no
    // cancel. One set before this line still wins over the commit.
    stop.finish();
    let stopped = stop.is_stopped();
    // What ends the run with an error once the session is put back.
    let mut failed = None;
    if succeeded && stopped {
        outcome.stopped = true;
    } else if succeeded && connection.is_autocommit() {
        // The last statement ended the transaction. No statement followed
        // it, so the check before a statement never saw it.
        failed = Some(Error::LeftTransaction);
    } else if succeeded {
        match connection.execute_batch("COMMIT").map_err(map_error) {
            Ok(()) => outcome.end = ScriptEnd::Committed,
            // An interrupt that was on its way landed on the commit.
            // SQLite then leaves the transaction open for the rollback
            // below; one that is gone went a way that is not known.
            Err(Error::Cancelled) => match cancelled_commit(Ok(!connection.is_autocommit())) {
                Ok(()) => outcome.stopped = true,
                Err(error) => failed = Some(error),
            },
            // SQLite keeps the transaction open when a COMMIT fails (a
            // deferred foreign key, a reader holding the file): the
            // rollback below is what makes "nothing is written" true.
            Err(error) => {
                outcome.end = ScriptEnd::CommitFailed {
                    error,
                    committed: 0,
                };
            }
        }
    }
    let ended = end_transaction(connection);
    ran?;
    if let Some(error) = failed {
        return Err(error);
    }
    // A rollback that did not happen leaves the end unknown.
    if let Err(error) = &ended
        && !connection.is_autocommit()
    {
        return Err(cleanup_failed(ScriptMode::Write, error));
    }
    Ok(outcome.put_back(ended))
}

/// Rolls back what is still open and puts the connect-time settings back
/// (a script may have changed them). A cancel can interrupt the cleanup
/// itself, so it gets one more try.
fn end_transaction(connection: &rusqlite::Connection) -> Result<()> {
    let attempt = || -> rusqlite::Result<()> {
        // An error can end SQLite's transaction by itself; roll back only
        // one that is still open.
        if !connection.is_autocommit() {
            connection.execute_batch("ROLLBACK")?;
        }
        set_session_pragmas(connection)
    };
    attempt().or_else(|_| attempt()).map_err(map_error)
}

fn statements(
    connection: &rusqlite::Connection,
    fences: &Fences,
    texts: &[String],
    limit: usize,
    mode: ScriptMode,
    stop: &StopFlag,
    outcome: &mut ScriptOutcome,
) -> Result<()> {
    for text in texts {
        if stop.is_stopped() {
            outcome.stopped = true;
            outcome.results.push(StatementResult {
                elapsed: std::time::Duration::ZERO,
                outcome: StatementOutcome::Cancelled,
            });
            break;
        }
        // The open transaction is what stops what `query_only` lets
        // through (a change of journal mode, the empty file a VACUUM INTO
        // leaves), and in a run that writes it is what the run commits.
        // The statement before this one succeeded, so SQLite did not end
        // it over an error: the script did.
        if connection.is_autocommit() {
            return Err(match mode {
                ScriptMode::ReadOnly => Error::LeftReadOnly,
                ScriptMode::Write => Error::LeftTransaction,
            });
        }
        // A run that writes has `query_only` off itself.
        if mode == ScriptMode::ReadOnly {
            match still_query_only(connection) {
                Ok(true) => {}
                // A stop that landed on the check: this statement is the
                // cancelled one.
                Err(Error::Cancelled) => {
                    outcome.stopped = true;
                    outcome.results.push(StatementResult {
                        elapsed: std::time::Duration::ZERO,
                        outcome: StatementOutcome::Cancelled,
                    });
                    break;
                }
                // No answer counts as left, as at the end of the run.
                Ok(false) | Err(_) => return Err(Error::LeftReadOnly),
            }
        }
        let started = Instant::now();
        // Only the script's own text is fenced: the checks above and the
        // rollback after the run are the app's.
        let result = {
            let _fenced = fences.fence(Fence::Script);
            statement(connection, text, limit)
        };
        let result = match result {
            Ok(result) => result,
            Err(error) => crate::script::statement_failed(error, None)?,
        };
        let cancelled = result == StatementOutcome::Cancelled;
        outcome.stopped |= cancelled;
        let last = cancelled || matches!(result, StatementOutcome::Error { .. });
        outcome.results.push(StatementResult {
            elapsed: started.elapsed(),
            outcome: result,
        });
        if last {
            break;
        }
    }
    Ok(())
}

/// `text` up to the end of its last token that is not a comment.
fn code_only(text: &str) -> &str {
    let end = crate::sql::tokenize(Dialect::Sqlite, text)
        .iter()
        .rev()
        .find(|token| {
            !matches!(
                token.kind,
                crate::sql::TokenKind::Whitespace | crate::sql::TokenKind::Comment
            )
        })
        .map_or(0, |token| token.range.end);
    &text[..end]
}

/// Whether `sqlite3_changes()` describes this statement: it keeps the count
/// of the last INSERT, UPDATE or DELETE, whatever ran since.
fn counts_changes(text: &str) -> bool {
    crate::sql::words(Dialect::Sqlite, text)
        .first()
        .is_some_and(|word| matches!(word.as_str(), "INSERT" | "UPDATE" | "DELETE" | "REPLACE"))
}

fn statement(
    connection: &rusqlite::Connection,
    text: &str,
    limit: usize,
) -> Result<StatementOutcome> {
    // SQLite names an unaliased column after its text, so a comment after
    // the last token would end up in the name.
    let text = code_only(text);
    // Only comments: SQLite prepares nothing, and there is nothing to run.
    if text.is_empty() {
        return Ok(StatementOutcome::Done {
            affected: None,
            warnings: 0,
        });
    }
    // A second statement in the text is refused (MultipleStatement), but
    // only after rusqlite prepared it. SQLite asks the authorizer for it as
    // for the first, and the run's checks cover what a pragma that got past
    // would have done by then.
    let mut statement = connection.prepare(text).map_err(map_error)?;
    let declared = declared_columns(connection, &statement, text)?;
    if declared.is_empty() {
        let changed = statement.raw_execute().map_err(map_error)?;
        return Ok(StatementOutcome::Done {
            affected: counts_changes(text).then_some(changed as u64),
            warnings: 0,
        });
    }
    let mut rows = statement.raw_query();
    let mut values: Vec<Vec<Value>> = Vec::new();
    while let Some(row) = rows.next().map_err(map_error)? {
        let mut cells = Vec::with_capacity(declared.len());
        for index in 0..declared.len() {
            cells.push(from_sqlite(row.get_ref(index).map_err(map_error)?));
        }
        values.push(cells);
        if values.len() > limit {
            break;
        }
    }
    let truncated = values.len() > limit;
    values.truncate(limit);
    Ok(StatementOutcome::Rows {
        columns: column_metas(declared, &values),
        rows: values,
        truncated,
    })
}

fn to_sqlite(value: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sqlite;
    match value {
        Value::Null => Sqlite::Null,
        Value::Bool(flag) => Sqlite::Integer(i64::from(*flag)),
        Value::Int(number) => Sqlite::Integer(*number),
        Value::Float(number) => Sqlite::Real(*number),
        // Filter values arrive as text. Typeless and computed columns do no
        // conversion, so a number compared as text never matches; bind text
        // that is exactly a number's canonical form as that number instead.
        // Columns with TEXT affinity turn it back into text.
        Value::Text(text) => {
            if let Ok(number) = text.parse::<i64>()
                && number.to_string() == **text
            {
                Sqlite::Integer(number)
            } else if let Ok(number) = text.parse::<f64>()
                && number.is_finite()
                && number.to_string() == **text
            {
                Sqlite::Real(number)
            } else {
                Sqlite::Text(text.to_string())
            }
        }
        Value::Bytes(bytes) => Sqlite::Blob(bytes.to_vec()),
    }
}

fn from_sqlite(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(number) => Value::Int(number),
        ValueRef::Real(number) => Value::Float(number),
        ValueRef::Text(bytes) => Value::Text(String::from_utf8_lossy(bytes).into()),
        ValueRef::Blob(bytes) => Value::Bytes(bytes.into()),
    }
}

/// Text from the catalog, with U+FFFD for bytes that are not UTF-8. SQLite
/// keeps a name's bytes as they were written, and rusqlite refuses to read
/// such text as a `String`.
struct Lossy {
    text: String,
    /// Whether no byte was replaced. Otherwise `text` is not the name, and
    /// no SQL text can spell it.
    exact: bool,
}

impl FromSql for Lossy {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let ValueRef::Text(bytes) = value else {
            return Err(FromSqlError::InvalidType);
        };
        let text = String::from_utf8_lossy(bytes);
        Ok(Self {
            exact: matches!(text, Cow::Borrowed(_)),
            text: text.into_owned(),
        })
    }
}

/// The catalog's text in column `index` of `row`, read as [`Lossy`].
fn text(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<String> {
    Ok(row.get::<_, Lossy>(index)?.text)
}

/// [`text`], for a column that may be NULL.
fn optional_text(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Option<String>> {
    Ok(row.get::<_, Option<Lossy>>(index)?.map(|lossy| lossy.text))
}

/// The name and the declared type of each result column of `statement`,
/// which was prepared from `text`. rusqlite's own `columns()` panics on a
/// name or a type that is not UTF-8, which a file can hold, so they are read
/// as bytes instead, from a second preparation of `text`.
fn declared_columns(
    connection: &rusqlite::Connection,
    statement: &rusqlite::Statement<'_>,
    text: &str,
) -> Result<Vec<(String, String)>> {
    let columns = tabletist_sqlite_ffi::result_columns(connection, text).map_err(map_error)?;
    // Another process can change the file's schema between the two
    // preparations.
    if columns.len() != statement.column_count() {
        return Err(Error::query(
            "The file's schema changed while this was read. Try again.",
        ));
    }
    Ok(columns
        .into_iter()
        .map(|column| (column.name, column.decl_type))
        .collect())
}

/// The settings every session has from the start: read-only, an untrusted
/// schema, a busy timeout, LIKE ignoring case (the filters rely on it) and
/// result columns named by the column alone (a page shows the names, and a
/// save finds its columns by them; the other way a table's are
/// `users.id`). `open` sets them, and a script run sets them again
/// afterwards, since a script may have changed any of them. Foreign keys
/// are enforced: the bundled SQLite has them on, and a script cannot turn
/// them off (the pragma does nothing inside a transaction, which is where
/// a script runs), but a save's refusal of a child with no parent should
/// not rest on how the library was built.
///
/// Three more are put back because they change what a later write keeps,
/// a save's as much as a script's: a script may turn CHECK constraints
/// off, let triggers fire themselves, or have `ALTER TABLE ... RENAME`
/// leave the views and triggers that name the table alone. It may, for its
/// own run; the flag ends with the run. (`locking_mode`, the fourth
/// setting of that kind, is denied to a script: see `Fence::Script`.)
fn set_session_pragmas(connection: &rusqlite::Connection) -> rusqlite::Result<()> {
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    connection.execute_batch(
        "PRAGMA query_only = ON; PRAGMA trusted_schema = OFF; PRAGMA case_sensitive_like = OFF; \
         PRAGMA full_column_names = OFF; PRAGMA short_column_names = ON; \
         PRAGMA foreign_keys = ON; PRAGMA ignore_check_constraints = OFF; \
         PRAGMA recursive_triggers = OFF; PRAGMA legacy_alter_table = OFF;",
    )
}

/// Stops a script when dropped, unless disarmed first: a `run_script`
/// future that is dropped must not leave its blocking job running the
/// remaining statements.
struct StopOnDrop(Option<StopFlag>);

impl StopOnDrop {
    fn disarm(mut self) {
        self.0 = None;
    }
}

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        if let Some(stop) = self.0.take() {
            stop.stop();
        }
    }
}

/// Result columns from their declared types; a column without one takes
/// its kind from the first value that is not NULL.
fn column_metas(declared: Vec<(String, String)>, rows: &[Vec<Value>]) -> Vec<ColumnMeta> {
    declared
        .into_iter()
        .enumerate()
        .map(|(index, (name, type_name))| {
            let kind = if type_name.is_empty() {
                rows.iter()
                    .map(|row| &row[index])
                    .find(|value| !value.is_null())
                    .map_or(ValueKind::Other, ValueKind::of_value)
            } else {
                ValueKind::from_sqlite_decl(&type_name)
            };
            ColumnMeta {
                name,
                type_name,
                kind,
            }
        })
        .collect()
}

/// Refuses a raw WHERE that would end the page's statement early. One that
/// holds a NUL, where SQLite stops reading, or that ends inside a `/*`
/// comment, which SQLite accepts, would hide the page's ORDER BY, LIMIT and
/// OFFSET. One that holds a `;` has a second statement after it, and
/// rusqlite prepares that before it refuses the text (see `Conn::browse`).
/// A `;` our tokenizer does not see where SQLite does is left to
/// `Fence::Filter`.
fn check_raw_where(query: &RowQuery) -> Result<()> {
    let Some(raw) = query.raw_where.as_deref() else {
        return Ok(());
    };
    if raw.contains('\0') {
        return Err(Error::query(
            "The WHERE text holds a NUL character. Remove it.",
        ));
    }
    if crate::dialect::sqlite_ends_in_block_comment(raw) {
        return Err(Error::query(
            "The WHERE text ends inside a /* comment. Close it with */.",
        ));
    }
    if crate::sql::tokenize(Dialect::Sqlite, raw)
        .iter()
        .any(|token| token.kind == crate::sql::TokenKind::Semicolon)
    {
        return Err(Error::query(
            "The WHERE text holds a `;`. A filter is one condition, not a statement.",
        ));
    }
    Ok(())
}

/// What `Fence::Filter` refused a filter, in words. SQLite says only "not
/// authorized", or "authorization denied" when the refusal comes from
/// inside a running statement, and an honest filter can get there: a pragma
/// read as a table takes its argument as the pragma's value. The code stays
/// SQLite's (SQLITE_AUTH), which tells this refusal from `check_raw_where`'s.
fn filter_denied(error: Error) -> Error {
    match error {
        Error::Query {
            code: Some(code), ..
        } if code == "23" => Error::Query {
            code: Some(code),
            message: "A filter cannot use a PRAGMA with an argument, ATTACH or a transaction \
                      statement."
                .into(),
            detail: None,
            hint: None,
            named: Box::default(),
        },
        other => other,
    }
}

/// `path` as a name SQLite takes for a file and nothing else. The bundled
/// SQLite is built with SQLITE_USE_URI, so it reads every name starting with
/// `file:` as a URI, with or without SQLITE_OPEN_URI. It would then open
/// another file than the one named, and `immutable=1`, `nolock=1` or `vfs=`
/// in the name would switch off the locking that keeps a read consistent.
/// Only a relative path can start that way, and `./` in front of it names the
/// same file.
fn plain_file_name(path: &Path) -> Cow<'_, Path> {
    if path.as_os_str().as_encoded_bytes().starts_with(b"file:") {
        Cow::Owned(Path::new(".").join(path))
    } else {
        Cow::Borrowed(path)
    }
}

impl Conn {
    /// Opens `path`: read-only, or read-write for a writable connection.
    /// Never creates a file. Either way the session refuses writes
    /// (`PRAGMA query_only`, see `set_session_pragmas`).
    pub async fn open(path: &Path, access: Access) -> Result<Self> {
        let path = path.to_path_buf();
        let opened = move || -> Result<(rusqlite::Connection, Fences, String)> {
            if !path.is_file() {
                return Err(Error::Connect(format!("{} does not exist", path.display())));
            }
            let mode = match access {
                Access::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
                // Without SQLITE_OPEN_CREATE: a missing file stays missing.
                Access::Writable => OpenFlags::SQLITE_OPEN_READ_WRITE,
            };
            let flags = mode | OpenFlags::SQLITE_OPEN_NO_MUTEX;
            let connection = rusqlite::Connection::open_with_flags(plain_file_name(&path), flags)
                .map_err(map_error)?;
            // A double-quoted name that matches no column is an error, not a
            // string literal: a filter on a renamed column must fail rather
            // than compare against its own name and match every row.
            // Defensive mode and an untrusted schema keep a crafted file's
            // views and triggers from reaching risky functions.
            for (option, on) in [
                (DbConfig::SQLITE_DBCONFIG_DQS_DML, false),
                (DbConfig::SQLITE_DBCONFIG_DQS_DDL, false),
                (DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true),
            ] {
                connection.set_db_config(option, on).map_err(map_error)?;
            }
            set_session_pragmas(&connection).map_err(map_error)?;
            // A file that is not a database only fails on its first read.
            connection
                .query_row("SELECT count(*) FROM sqlite_master", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(|error| match map_error(error) {
                    Error::Query { message, .. } => Error::Connect(message),
                    other => other,
                })?;
            let journal_mode = connection
                .query_row("PRAGMA main.journal_mode", [], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(map_error)?;
            // From here on SQLite asks before it prepares anything: what
            // it may do depends on whose text it is (see `Fence`).
            let fences = Fences::default();
            let asked = fences.clone();
            tabletist_sqlite_ffi::set_authorizer(&connection, move |action| {
                authorize(asked.current(), action)
            })
            .map_err(map_error)?;
            Ok((connection, fences, journal_mode))
        };
        let (connection, fences, journal_mode) = tokio::task::spawn_blocking(opened)
            .await
            .map_err(|error| Error::Io(error.to_string()))??;
        let interrupt = Arc::new(connection.get_interrupt_handle());
        Ok(Self {
            inner: Arc::new(Mutex::new(connection)),
            interrupt,
            fences,
            journal_mode,
        })
    }

    /// Runs `work` on the blocking pool with the connection.
    pub(crate) async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce(&rusqlite::Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let inner = Arc::clone(&self.inner);
        // If the caller drops this future (a closed tab, a replaced query),
        // stop the statement instead of letting it hold the connection.
        let guard = InterruptOnDrop(Some(Arc::clone(&self.interrupt)));
        let result = tokio::task::spawn_blocking(move || {
            let connection = inner
                .lock()
                .map_err(|_| Error::ConnectionLost("the SQLite connection was poisoned".into()))?;
            work(&connection)
        })
        .await;
        guard.disarm();
        result.map_err(|error| {
            // A panic may have left the connection half-used (the mutex is
            // poisoned too): the session must be reconnected.
            if error.is_panic() {
                Error::ConnectionLost("the SQLite worker panicked".into())
            } else {
                Error::Io(error.to_string())
            }
        })?
    }

    /// `run` for a page or a count, whose SQL holds the user's WHERE text.
    /// rusqlite finds a second statement in a text by preparing it, and
    /// SQLite applies a flag pragma as soon as it is prepared: a WHERE that
    /// got `; PRAGMA query_only = 0` past `check_raw_where` and past
    /// `Fence::Filter`, which denies a filter every pragma with a value,
    /// would fail and still leave the session open to writes. So, as the
    /// layer behind both, a failure puts the session's settings back, and a
    /// session that cannot take them counts as lost. What the fence refused
    /// is worded for the user (see `filter_denied`).
    async fn browse<T: Send + 'static>(
        &self,
        work: impl FnOnce(&rusqlite::Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        self.run(move |connection| {
            let result = work(connection);
            if result.is_err() {
                // A cancel meant for the statement can land here instead,
                // so this gets one more try.
                set_session_pragmas(connection)
                    .or_else(|_| set_session_pragmas(connection))
                    .map_err(|error| {
                        Error::ConnectionLost(format!(
                            "could not put the session's settings back: {}",
                            map_error(error)
                        ))
                    })?;
            }
            result.map_err(filter_denied)
        })
        .await
    }

    /// The primary key columns in key order; empty for views and keyless tables.
    #[cfg(test)]
    pub async fn primary_key(&self, object: &ObjectRef) -> Result<Vec<String>> {
        let object = object.clone();
        self.run(move |connection| primary_key(connection, &object))
            .await
    }
}

impl Adapter for Conn {
    /// Never: a file is read here, not over a network.
    fn is_encrypted(&self) -> bool {
        false
    }

    fn cancel_handle(&self) -> CancelHandle {
        CancelHandle(CancelInner::Sqlite(Arc::clone(&self.interrupt)))
    }

    /// See [`crate::Connection::write`]. One blocking job. An interrupt
    /// reaches only the statement that is running and is not kept for the
    /// next, so `stop` is what ends the save between two of them.
    async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
        if changes.object.schema != "main" {
            return Err(Error::Unsupported(
                "saving to an attached database is not built yet",
            ));
        }
        let changes = changes.clone();
        let journal_mode = self.journal_mode.clone();
        let stop = stop.clone();
        self.run(move |connection| write::write(connection, &changes, &journal_mode, &stop))
            .await
    }

    /// The server's name and version for the footer, like `SQLite 3.46.0`.
    async fn server_version(&self) -> Result<String> {
        self.run(|connection| {
            let version: String = connection
                .query_row("SELECT sqlite_version()", [], |row| row.get(0))
                .map_err(map_error)?;
            Ok(format!("SQLite {version}"))
        })
        .await
    }

    /// None: a file is one database.
    async fn list_databases(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// `main` plus attached databases (`temp` is hidden).
    async fn list_schemas(&self) -> Result<Vec<String>> {
        self.run(|connection| {
            let mut statement = connection
                .prepare(&format!(
                    "SELECT name FROM pragma_database_list WHERE name <> 'temp' \
                     ORDER BY seq LIMIT {MAX_LISTED}"
                ))
                .map_err(map_error)?;
            let names = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(map_error)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(map_error)?;
            Ok(names)
        })
        .await
    }

    async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        let sql = format!(
            "SELECT name, type FROM {}.sqlite_master \
             WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' \
             ORDER BY name LIMIT {MAX_LISTED}",
            crate::Dialect::Sqlite.quote_ident(schema)
        );
        self.run(move |connection| {
            let mut statement = connection.prepare(&sql).map_err(map_error)?;
            let objects = statement
                .query_map([], |row| {
                    let kind: String = row.get(1)?;
                    Ok(ObjectInfo {
                        name: text(row, 0)?,
                        kind: if kind == "view" {
                            ObjectKind::View
                        } else {
                            ObjectKind::Table
                        },
                        estimated_rows: None,
                    })
                })
                .map_err(map_error)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(map_error)?;
            Ok(objects)
        })
        .await
    }

    async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        let object = object.clone();
        self.run(move |connection| {
            let columns = columns(connection, &object)?;
            if columns.is_empty() {
                return Err(Error::query(format!(
                    "no such table or view: {}",
                    object.name
                )));
            }
            Ok(Structure {
                columns,
                primary_key: primary_key(connection, &object)?,
                indexes: indexes(connection, &object)?,
                foreign_keys: foreign_keys(connection, &object)?,
            })
        })
        .await
    }

    async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        check_raw_where(query)?;
        let query = query.clone();
        let limit = query.limit as usize;
        let fences = self.fences.clone();
        // One blocking job for the key lookup and the select, so a cancel
        // can never fall in a gap between them.
        self.browse(move |connection| {
            let key = ordering_key(connection, &query.object)?;
            let binary = binary_columns(connection, &query)?;
            let sql = Dialect::Sqlite.select_rows(&query, &key, &binary);
            let ordered_by_key = !key.is_empty();
            let started = Instant::now();
            // The lookups above are the app's own, and the fence would
            // refuse them: a table-valued pragma's argument is a pragma's
            // value. From here to the last row the text holds the raw WHERE.
            let _fenced = fences.fence(Fence::Filter);
            let mut statement = connection.prepare(&sql.text).map_err(map_error)?;
            let declared = declared_columns(connection, &statement, &sql.text)?;
            let mut rows = statement
                .query(rusqlite::params_from_iter(sql.params.iter().map(to_sqlite)))
                .map_err(map_error)?;
            let mut values: Vec<Vec<Value>> = Vec::new();
            while let Some(row) = rows.next().map_err(map_error)? {
                let mut cells = Vec::with_capacity(declared.len());
                for index in 0..declared.len() {
                    cells.push(from_sqlite(row.get_ref(index).map_err(map_error)?));
                }
                values.push(cells);
                // Never read past one extra row, even if a raw WHERE comments
                // out the LIMIT.
                if values.len() > limit {
                    break;
                }
            }
            let has_more = values.len() > limit;
            values.truncate(limit);
            let columns = column_metas(declared, &values);
            Ok(RowPage {
                columns,
                rows: values,
                has_more,
                ordered_by_key,
                elapsed: started.elapsed(),
            })
        })
        .await
    }

    async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        check_raw_where(query)?;
        let query = query.clone();
        let fences = self.fences.clone();
        self.browse(move |connection| {
            let binary = binary_columns(connection, &query)?;
            let sql = Dialect::Sqlite.count_rows(&query, &binary);
            // As in `fetch_rows`: only the text with the raw WHERE.
            let _fenced = fences.fence(Fence::Filter);
            let count: i64 = connection
                .query_row(
                    &sql.text,
                    rusqlite::params_from_iter(sql.params.iter().map(to_sqlite)),
                    |row| row.get(0),
                )
                .map_err(map_error)?;
            Ok(u64::try_from(count).unwrap_or(0))
        })
        .await
    }

    /// See [`crate::Connection::run_script`]. The whole script is one
    /// blocking job; a progress handler checks `stop` while a statement
    /// runs.
    async fn run_script(
        &self,
        texts: Vec<String>,
        limit: u32,
        mode: ScriptMode,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let stop = stop.clone();
        let limit = limit as usize;
        // If the caller drops this future, `run` interrupts the statement
        // that is running; this stops the ones that have not begun.
        let guard = StopOnDrop(Some(stop.clone()));
        let fences = self.fences.clone();
        let outcome = self
            .run(move |connection| script(connection, &fences, &texts, limit, mode, &stop))
            .await;
        guard.disarm();
        outcome
    }
}

fn columns(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<ColumnInfo>> {
    // A table WITHOUT ROWID has no rowid for a column to be the alias of.
    let listed = connection
        .query_row(
            "SELECT wr = 0 FROM pragma_table_list(?1) WHERE schema = ?2",
            [&object.name, &object.schema],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(map_error)?;
    let rowid = listed.unwrap_or(false);
    // The alias has no index: the rowid is the key. `INTEGER PRIMARY KEY
    // DESC` reads the same in the column list and is no alias (an old
    // exception SQLite keeps): its key has an index of its own, and an
    // insert that names no value leaves the column NULL.
    let indexed = connection
        .query_row(
            "SELECT count(*) FROM pragma_index_list(?1, ?2) WHERE origin = 'pk'",
            [&object.name, &object.schema],
            |row| row.get::<_, i64>(0),
        )
        .map_err(map_error)?
        > 0;
    // The table's CHECK conditions, from the statement it was made with:
    // SQLite keeps them nowhere else. A table with no statement (a virtual
    // one's shadow, an internal one) has none, and a table that is not
    // there is not asked for: its schema may not be there either.
    let made: Option<String> = match listed {
        Some(_) => connection
            .query_row(
                &format!(
                    "SELECT CAST(sql AS TEXT) FROM {}.sqlite_master \
                     WHERE type = 'table' AND name = ?1",
                    Dialect::Sqlite.quote_ident(&object.schema)
                ),
                [&object.name],
                // Lossy, as every name of this file is read: a statement
                // can hold bytes that are no UTF-8, and a table with such
                // a name is still described. Cast, because a file can
                // keep it as a blob, which is no reason to fail either.
                |row| optional_text(row, 0),
            )
            .optional()
            .map_err(map_error)?
            .flatten(),
        None => None,
    };
    // A list is the app's to hold a typed value to, letter for letter. A
    // column that compares otherwise (`COLLATE NOCASE`) takes what the
    // list would refuse, and SQLite does not say which column that is: a
    // table whose statement names a collation anywhere has no lists.
    let made = made.filter(|sql| !crate::check::names_a_collation(sql));
    let checks = made.as_deref().map(crate::check::in_create_table);
    let checks = checks.unwrap_or_default();
    // SQLite matches a name without regard to the case of its ASCII
    // letters, and no others: `É` and `é` are two columns. The parser folds
    // a bare name the same way and keeps a quoted one as written.
    let allowed = |name: &str| {
        checks.iter().find_map(|check| {
            crate::check::allowed_values(check, name)
                .or_else(|| crate::check::allowed_values(check, &name.to_ascii_lowercase()))
        })
    };
    let mut statement = connection
        .prepare(
            "SELECT name, type, \"notnull\", dflt_value, hidden, pk \
             FROM pragma_table_xinfo(?1, ?2) WHERE hidden <> 1 ORDER BY cid",
        )
        .map_err(map_error)?;
    let columns = statement
        .query_map([&object.name, &object.schema], |row| {
            let type_name = optional_text(row, 1)?.unwrap_or_default();
            let keyed = row.get::<_, i64>(5)? > 0;
            let name = text(row, 0)?;
            // A text column's alone, as on PostgreSQL.
            let class = column_class(Dialect::Sqlite, &type_name);
            let texts = matches!(class, ColumnClass::Text { .. });
            Ok((
                ColumnInfo {
                    allowed_values: texts.then(|| allowed(&name)).flatten(),
                    name,
                    nullable: row.get::<_, i64>(2)? == 0,
                    default: optional_text(row, 3)?,
                    comment: None,
                    // 2 is a virtual generated column, 3 a stored one.
                    generated: matches!(row.get::<_, i64>(4)?, 2 | 3),
                    // Decided below, once the key's columns are counted.
                    identity: keyed && type_name.eq_ignore_ascii_case("INTEGER"),
                    type_name,
                },
                keyed,
            ))
        })
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)?;
    // The rowid's alias is the key's one column, declared INTEGER and
    // nothing else: SQLite numbers it when an insert names no value.
    let alone = columns.iter().filter(|(_, keyed)| *keyed).count() == 1;
    Ok(columns
        .into_iter()
        .map(|(mut column, _)| {
            column.identity &= rowid && alone && !indexed;
            column
        })
        .collect())
}

/// The columns a filter of `query` compares as bytes when its value reads
/// as bytes; empty, without asking, when no filter has such a value. SQLite
/// keeps a blob in a column of any type, so besides the ones declared as
/// blobs these are the ones whose type says nothing: none at all, or a name
/// like `BINARY(16)` or `UUID` that SQLite gives no blob affinity.
fn binary_columns(connection: &rusqlite::Connection, query: &RowQuery) -> Result<Vec<String>> {
    if !crate::dialect::reads_bytes(query) {
        return Ok(Vec::new());
    }
    Ok(columns(connection, &query.object)?
        .into_iter()
        .filter(|column| {
            matches!(
                ValueKind::from_sqlite_decl(&column.type_name),
                ValueKind::Binary | ValueKind::Other
            )
        })
        .map(|column| column.name)
        .collect())
}

fn primary_key(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<String>> {
    Ok(key_names(connection, object)?
        .into_iter()
        .map(|name| name.text)
        .collect())
}

/// The key a page is ordered by: the primary key, or nothing when one of its
/// names is not UTF-8, since the page's SQL cannot name that column.
fn ordering_key(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<String>> {
    let names = key_names(connection, object)?;
    if names.iter().any(|name| !name.exact) {
        return Ok(Vec::new());
    }
    Ok(names.into_iter().map(|name| name.text).collect())
}

fn key_names(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<Lossy>> {
    let mut statement = connection
        .prepare("SELECT name FROM pragma_table_info(?1, ?2) WHERE pk > 0 ORDER BY pk")
        .map_err(map_error)?;
    statement
        .query_map([&object.name, &object.schema], |row| row.get(0))
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)
}

fn indexes(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<IndexInfo>> {
    let mut list = connection
        .prepare(
            "SELECT name, \"unique\", origin, partial FROM pragma_index_list(?1, ?2) \
             ORDER BY name",
        )
        .map_err(map_error)?;
    let entries = list
        .query_map([&object.name, &object.schema], |row| {
            Ok((
                row.get_ref(0)?.as_bytes()?.to_vec(),
                row.get::<_, i64>(1)? != 0,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)? != 0,
            ))
        })
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)?;
    // The key's entries: `key` is 0 for what an index only carries along,
    // the rowid or the rest of a WITHOUT ROWID table's key.
    let mut info = connection
        .prepare(
            "SELECT name, cid FROM pragma_index_xinfo(?1, ?2) WHERE \"key\" = 1 ORDER BY seqno",
        )
        .map_err(map_error)?;
    let mut indexes = Vec::new();
    for (name, unique, origin, partial) in entries {
        // The name goes back to SQLite as the bytes it gave: one that is not
        // UTF-8 would find no index once its bytes were replaced.
        let entries = info
            .query_map(rusqlite::params![name, object.schema], |row| {
                Ok((row.get::<_, Option<Lossy>>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(map_error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(map_error)?;
        // A whole column has its place in the table as `cid` (an expression
        // has -2, the rowid -1) and a name SQL can spell. The entry's
        // collation is not compared with the column's: no pragma gives the
        // one a column was declared with. So a unique index, or a primary
        // key, that compares otherwise than its column is not detected
        // here. What stops it is a save's own check: it reads the row by
        // its key and refuses more than one.
        let key_columns = entries
            .iter()
            .map(|(name, cid)| {
                let name = name.as_ref().filter(|name| *cid >= 0 && name.exact)?;
                Some(name.text.clone())
            })
            .collect();
        let columns = entries
            .into_iter()
            .map(|(name, _)| name.map_or_else(|| "<expression>".into(), |name| name.text))
            .collect();
        indexes.push(IndexInfo {
            name: String::from_utf8_lossy(&name).into_owned(),
            key_columns,
            columns,
            unique,
            primary: origin == "pk",
            method: None,
            partial,
        });
    }
    Ok(indexes)
}

fn foreign_keys(
    connection: &rusqlite::Connection,
    object: &ObjectRef,
) -> Result<Vec<ForeignKeyInfo>> {
    let mut statement = connection
        .prepare(
            "SELECT id, \"table\", \"from\", \"to\", on_update, on_delete \
             FROM pragma_foreign_key_list(?1, ?2) ORDER BY id, seq",
        )
        .map_err(map_error)?;
    let rows = statement
        .query_map([&object.name, &object.schema], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                text(row, 1)?,
                text(row, 2)?,
                optional_text(row, 3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)?;
    let mut keys: Vec<(i64, ForeignKeyInfo)> = Vec::new();
    for (id, table, from, to, on_update, on_delete) in rows {
        if keys.last().is_none_or(|(last, _)| *last != id) {
            keys.push((
                id,
                ForeignKeyInfo {
                    name: None,
                    columns: Vec::new(),
                    ref_schema: object.schema.clone(),
                    ref_table: table,
                    ref_columns: Vec::new(),
                    on_update,
                    on_delete,
                },
            ));
        }
        let (_, key) = keys.last_mut().expect("pushed above");
        key.columns.push(from);
        if let Some(to) = to {
            key.ref_columns.push(to);
        }
    }
    Ok(keys.into_iter().map(|(_, key)| key).collect())
}

#[cfg(test)]
mod tests {
    use super::{code_only, counts_changes, named};

    #[test]
    fn a_failure_is_named_from_its_code_and_its_message() {
        let of = |code: i32, message: &str| {
            let named = named(code, message);
            (named.constraint, named.columns)
        };
        // SQLITE_CONSTRAINT_UNIQUE, of one column and of two.
        assert_eq!(
            of(2067, "UNIQUE constraint failed: books.isbn"),
            (None, vec!["isbn".to_owned()])
        );
        assert_eq!(
            of(2067, "UNIQUE constraint failed: books.title, books.format"),
            (None, vec!["title".to_owned(), "format".to_owned()])
        );
        // Of an index over an expression, which has no column to name.
        assert_eq!(
            of(2067, "UNIQUE constraint failed: index 'books_lower_isbn'"),
            (Some("books_lower_isbn".into()), vec![])
        );
        // SQLITE_CONSTRAINT_PRIMARYKEY and SQLITE_CONSTRAINT_NOTNULL.
        assert_eq!(
            of(1555, "UNIQUE constraint failed: books.id"),
            (None, vec!["id".to_owned()])
        );
        assert_eq!(
            of(1299, "NOT NULL constraint failed: book_covers.publisher_id"),
            (None, vec!["publisher_id".to_owned()])
        );
        // SQLITE_CONSTRAINT_CHECK names the constraint.
        assert_eq!(
            of(275, "CHECK constraint failed: book_covers_kind_check"),
            (Some("book_covers_kind_check".into()), vec![])
        );
        // SQLITE_CONSTRAINT_FOREIGNKEY names nothing, and no other error.
        assert_eq!(of(787, "FOREIGN KEY constraint failed"), (None, vec![]));
        assert_eq!(of(1, "no such table: nope"), (None, vec![]));
        assert_eq!(
            named(2067, "UNIQUE constraint failed: books.isbn").number,
            None
        );
    }

    #[test]
    fn only_data_changes_report_a_count() {
        for text in [
            "INSERT INTO t VALUES (1)",
            "update t set a = 1",
            "DELETE FROM t",
            "REPLACE INTO t VALUES (1)",
            "-- c\nDELETE FROM t",
        ] {
            assert!(counts_changes(text), "{text}");
        }
        for text in [
            "PRAGMA foreign_keys = ON",
            "CREATE TABLE t (a)",
            "WITH x AS (SELECT 1) DELETE FROM t",
            "ANALYZE",
            "",
        ] {
            assert!(!counts_changes(text), "{text}");
        }
    }

    #[test]
    fn trailing_comments_and_blanks_are_trimmed() {
        assert_eq!(code_only("SELECT 1 -- note\n"), "SELECT 1");
        assert_eq!(code_only("SELECT 1 /* x */ "), "SELECT 1");
        assert_eq!(code_only("/* only */ -- comments"), "");
        assert_eq!(
            code_only("SELECT '-- not a comment'"),
            "SELECT '-- not a comment'"
        );
    }
    use super::*;

    async fn fixture_as(access: Access) -> (Conn, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.db");
        crate::fixtures::write_sqlite_demo(&path).unwrap();
        (Conn::open(&path, access).await.unwrap(), dir)
    }

    async fn fixture() -> (Conn, tempfile::TempDir) {
        fixture_as(Access::ReadOnly).await
    }

    #[tokio::test]
    async fn a_missing_file_is_a_connect_error_and_is_not_created() {
        for access in [Access::ReadOnly, Access::Writable] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("missing.db");
            assert!(matches!(
                Conn::open(&path, access).await,
                Err(Error::Connect(_))
            ));
            assert!(!path.exists(), "{access:?}");
        }
    }

    #[tokio::test]
    async fn only_a_writable_handle_writes_and_only_with_query_only_lifted() {
        for access in [Access::ReadOnly, Access::Writable] {
            let (conn, _dir) = fixture_as(access).await;
            // The standing state refuses a write on both.
            let standing = conn
                .run(|connection| {
                    connection
                        .execute("UPDATE users SET email = email WHERE id = 1", [])
                        .map_err(map_error)
                })
                .await;
            assert!(standing.is_err(), "{access:?}");
            // Lifted, as a save will lift it: only the writable handle
            // writes.
            let lifted = conn
                .run(|connection| {
                    connection
                        .execute_batch("PRAGMA query_only = OFF")
                        .map_err(map_error)?;
                    let updated = connection
                        .execute("UPDATE users SET email = email WHERE id = 1", [])
                        .map_err(map_error);
                    set_session_pragmas(connection).map_err(map_error)?;
                    updated
                })
                .await;
            assert_eq!(lifted.is_ok(), access == Access::Writable, "{access:?}");
        }
    }

    /// Takes the authorizer off, for a test of the checks behind it.
    async fn without_the_authorizer(conn: &Conn) {
        conn.run(|connection| {
            tabletist_sqlite_ffi::remove_authorizer(connection).map_err(map_error)
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn a_script_that_gets_query_only_off_is_stopped_and_writes_nothing() {
        for access in [Access::ReadOnly, Access::Writable] {
            for texts in [
                vec!["PRAGMA query_only = OFF", "UPDATE users SET email = 'x'"],
                // In last position, where no statement follows to be checked.
                vec!["SELECT 1", "PRAGMA query_only = OFF"],
            ] {
                let (conn, _dir) = fixture_as(access).await;
                without_the_authorizer(&conn).await;
                let script = texts.iter().map(|text| (*text).to_owned()).collect();
                let ran = conn
                    .run_script(script, 10, ScriptMode::ReadOnly, &StopFlag::new())
                    .await;
                assert!(
                    matches!(ran, Err(Error::LeftReadOnly)),
                    "{access:?} {texts:?}: {ran:?}"
                );
                let (query_only, changed) = conn
                    .run(|connection| {
                        let one = |sql: &str| {
                            connection
                                .query_row(sql, [], |row| row.get::<_, i64>(0))
                                .map_err(map_error)
                        };
                        Ok((
                            one("PRAGMA query_only")?,
                            one("SELECT count(*) FROM users WHERE email = 'x'")?,
                        ))
                    })
                    .await
                    .unwrap();
                assert_eq!((query_only, changed), (1, 0), "{access:?} {texts:?}");
            }
        }
    }

    /// Runs `texts` past the refusal list, which only
    /// `Connection::run_script` applies.
    async fn run_unrefused(conn: &Conn, texts: &[&str]) -> Result<ScriptOutcome> {
        let script = texts.iter().map(|text| (*text).to_owned()).collect();
        conn.run_script(script, 10, ScriptMode::ReadOnly, &StopFlag::new())
            .await
    }

    /// Runs `texts` as a script that writes, past the refusal list.
    async fn write_unrefused(conn: &Conn, texts: &[&str]) -> Result<ScriptOutcome> {
        let script = texts.iter().map(|text| (*text).to_owned()).collect();
        conn.run_script(script, 10, ScriptMode::Write, &StopFlag::new())
            .await
    }

    /// How many rows of `events` have this kind.
    async fn events_of(conn: &Conn, kind: &'static str) -> i64 {
        conn.run(move |connection| {
            connection
                .query_row(
                    "SELECT count(*) FROM events WHERE kind = ?1",
                    [kind],
                    |row| row.get(0),
                )
                .map_err(map_error)
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn the_fence_denies_a_run_that_writes_what_would_end_its_transaction() {
        for text in [
            "COMMIT",
            "END",
            "ROLLBACK",
            "SAVEPOINT mine",
            "PRAGMA query_only = ON",
            "PRAGMA writable_schema = ON",
            "PRAGMA wal_checkpoint",
        ] {
            let (conn, _dir) = fixture_as(Access::Writable).await;
            let probe = "INSERT INTO events (kind) VALUES ('probe')";
            let outcome = write_unrefused(&conn, &[probe, text, probe]).await.unwrap();
            // Denied as that statement's error; the run stops and is undone.
            assert_eq!(outcome.results.len(), 2, "{text}");
            assert!(
                matches!(outcome.results[1].outcome, StatementOutcome::Error { .. }),
                "{text}: {:?}",
                outcome.results[1].outcome
            );
            assert_eq!(outcome.end, ScriptEnd::RolledBack, "{text}");
            assert_eq!(events_of(&conn, "probe").await, 0, "{text}");
            assert_eq!(standing(&conn).await, (1, true), "{text}");
        }
    }

    #[tokio::test]
    async fn a_run_that_writes_and_lost_its_transaction_runs_nothing_after_it() {
        let (conn, _dir) = fixture_as(Access::Writable).await;
        // What the refusal list and the authorizer both stop; this is the
        // check behind them.
        without_the_authorizer(&conn).await;
        let ran = write_unrefused(
            &conn,
            &[
                "INSERT INTO events (kind) VALUES ('before')",
                "COMMIT",
                "INSERT INTO events (kind) VALUES ('after')",
            ],
        )
        .await;
        assert_eq!(ran, Err(Error::LeftTransaction));
        // The script's own commit stands; nothing ran outside a transaction.
        assert_eq!(events_of(&conn, "before").await, 1);
        assert_eq!(events_of(&conn, "after").await, 0);
        assert_eq!(standing(&conn).await, (1, true));
    }

    #[tokio::test]
    async fn a_run_whose_last_statement_ended_its_transaction_fails_too() {
        let (conn, _dir) = fixture_as(Access::Writable).await;
        without_the_authorizer(&conn).await;
        // Nothing follows the COMMIT, so no check before a statement sees
        // it: the end of the run has to.
        let ran = write_unrefused(
            &conn,
            &["INSERT INTO events (kind) VALUES ('before')", "COMMIT"],
        )
        .await;
        assert_eq!(ran, Err(Error::LeftTransaction));
        assert_eq!(standing(&conn).await, (1, true));
    }

    #[tokio::test]
    async fn a_stop_before_the_commit_rolls_a_run_that_writes_back() {
        let (conn, _dir) = fixture_as(Access::Writable).await;
        let stop = StopFlag::new();
        let outcome = {
            let stop = stop.clone();
            conn.run(move |connection| {
                begin(connection, ScriptMode::Write)?;
                connection
                    .execute_batch("INSERT INTO events (kind) VALUES ('probe')")
                    .map_err(map_error)?;
                // Every statement is done, and then the stop comes.
                stop.stop();
                end_write(connection, &stop, Ok(()), ScriptOutcome::default())
            })
            .await
            .unwrap()
        };
        assert!(outcome.stopped);
        assert_eq!(outcome.end, ScriptEnd::RolledBack);
        assert!(stop.is_finishing());
        assert_eq!(events_of(&conn, "probe").await, 0);
        assert_eq!(standing(&conn).await, (1, true));
    }

    #[tokio::test]
    async fn a_locked_database_fails_a_run_that_writes_before_it_begins() {
        let (conn, dir) = fixture_as(Access::Writable).await;
        let other = rusqlite::Connection::open(dir.path().join("fixture.db")).unwrap();
        other.execute_batch("BEGIN IMMEDIATE").unwrap();
        // Not the session's five seconds, for the test's sake. The run
        // puts the session's own timeout back.
        conn.run(|connection| {
            connection
                .busy_timeout(std::time::Duration::from_millis(50))
                .map_err(map_error)
        })
        .await
        .unwrap();
        let probe = "INSERT INTO events (kind) VALUES ('probe')";
        let ran = write_unrefused(&conn, &[probe]).await;
        assert!(
            matches!(&ran, Err(Error::Query { message, .. }) if message.contains("locked")),
            "{ran:?}"
        );
        // The session is as it was: fenced, in no transaction, and usable.
        assert_eq!(standing(&conn).await, (1, true));
        other.execute_batch("ROLLBACK").unwrap();
        let outcome = write_unrefused(&conn, &[probe]).await.unwrap();
        assert_eq!(outcome.end, ScriptEnd::Committed);
        assert_eq!(events_of(&conn, "probe").await, 1);
    }

    /// The three flags a script may set for its own run, as the session
    /// has them now: CHECK constraints ignored, triggers firing themselves,
    /// and the old `ALTER TABLE ... RENAME`.
    async fn flags(conn: &Conn) -> [i64; 3] {
        conn.run(|connection| {
            let one = |sql: &str| {
                connection
                    .query_row(sql, [], |row| row.get::<_, i64>(0))
                    .map_err(map_error)
            };
            Ok([
                one("PRAGMA ignore_check_constraints")?,
                one("PRAGMA recursive_triggers")?,
                one("PRAGMA legacy_alter_table")?,
            ])
        })
        .await
        .unwrap()
    }

    /// How many rows the probe table `checked` holds.
    async fn checked_rows(conn: &Conn) -> i64 {
        conn.run(|connection| {
            connection
                .query_row("SELECT count(*) FROM checked", [], |row| row.get(0))
                .map_err(map_error)
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_flag_a_script_sets_ends_with_its_run() {
        const FLAGS: [&str; 3] = [
            "PRAGMA ignore_check_constraints = ON",
            "PRAGMA recursive_triggers = ON",
            "PRAGMA legacy_alter_table = ON",
        ];
        let (conn, _dir) = fixture_as(Access::Writable).await;
        let made = write_unrefused(&conn, &["CREATE TABLE checked (n INTEGER CHECK (n > 0))"])
            .await
            .unwrap();
        assert_eq!(made.end, ScriptEnd::Committed);
        // Set in a run that only reads, and in one that writes: neither
        // leaves the session with them.
        for mode in [ScriptMode::ReadOnly, ScriptMode::Write] {
            let script = FLAGS.iter().map(|text| (*text).to_owned()).collect();
            let outcome = conn
                .run_script(script, 10, mode, &StopFlag::new())
                .await
                .unwrap();
            assert_eq!(outcome.results.len(), 3, "{mode:?}");
            assert!(outcome.succeeded(), "{mode:?}: {outcome:?}");
            assert_eq!(flags(&conn).await, [0, 0, 0], "{mode:?}");
            assert_eq!(standing(&conn).await, (1, true), "{mode:?}");
        }
        // So the next run that writes keeps what the table's CHECK allows,
        // and no more.
        let refused = write_unrefused(&conn, &["INSERT INTO checked VALUES (-1)"])
            .await
            .unwrap();
        assert!(
            matches!(refused.results[0].outcome, StatementOutcome::Error { .. }),
            "{:?}",
            refused.results[0].outcome
        );
        assert_eq!(refused.end, ScriptEnd::RolledBack);
        assert_eq!(checked_rows(&conn).await, 0);
        // A script may still set one for its own run, which then keeps
        // what it wrote under it.
        let own = write_unrefused(&conn, &[FLAGS[0], "INSERT INTO checked VALUES (-1)"])
            .await
            .unwrap();
        assert_eq!(own.end, ScriptEnd::Committed);
        assert_eq!(checked_rows(&conn).await, 1);
        assert_eq!(flags(&conn).await, [0, 0, 0]);
        // A run that fails puts them back as well.
        let failed = write_unrefused(&conn, &[FLAGS[0], FLAGS[1], "SELECT nope"])
            .await
            .unwrap();
        assert_eq!(failed.end, ScriptEnd::RolledBack);
        assert_eq!(flags(&conn).await, [0, 0, 0]);
    }

    /// The session's `locking_mode`.
    async fn locking_mode(conn: &Conn) -> String {
        conn.run(|connection| {
            connection
                .query_row("PRAGMA locking_mode", [], |row| row.get(0))
                .map_err(map_error)
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_script_cannot_take_the_file_for_the_session() {
        for mode in [ScriptMode::ReadOnly, ScriptMode::Write] {
            for text in [
                "PRAGMA locking_mode = EXCLUSIVE",
                "PRAGMA main.locking_mode(exclusive)",
                "PRAGMA LOCKING_MODE = EXCLUSIVE",
            ] {
                let (conn, dir) = fixture_as(Access::Writable).await;
                let probe = "INSERT INTO events (kind) VALUES ('probe')";
                // In a run that writes, with a change before it, which is
                // what would have taken the lock.
                let script: Vec<String> = match mode {
                    ScriptMode::ReadOnly => vec![text.to_owned()],
                    ScriptMode::Write => vec![probe.to_owned(), text.to_owned()],
                };
                let outcome = conn
                    .run_script(script, 10, mode, &StopFlag::new())
                    .await
                    .unwrap();
                let last = outcome.results.last().unwrap();
                assert!(
                    matches!(
                        &last.outcome,
                        StatementOutcome::Error {
                            error: Error::Query { message, .. },
                            ..
                        } if message.contains("not authorized")
                    ),
                    "{mode:?} {text}: {:?}",
                    last.outcome
                );
                assert_eq!(outcome.end, ScriptEnd::RolledBack, "{mode:?} {text}");
                assert_eq!(locking_mode(&conn).await, "normal", "{mode:?} {text}");
                assert_eq!(events_of(&conn, "probe").await, 0, "{mode:?} {text}");
                // Another program can write to the file right away.
                let other = rusqlite::Connection::open(dir.path().join("fixture.db")).unwrap();
                other
                    .execute_batch("INSERT INTO events (kind) VALUES ('other')")
                    .unwrap();
            }
        }
        // Asking what the mode is stays a read like any other.
        let (conn, _dir) = fixture_as(Access::ReadOnly).await;
        let asked = run_unrefused(&conn, &["PRAGMA locking_mode"])
            .await
            .unwrap();
        assert!(
            matches!(asked.results[0].outcome, StatementOutcome::Rows { .. }),
            "{:?}",
            asked.results[0].outcome
        );
    }

    /// The session's `query_only` and whether it is out of a transaction.
    async fn standing(conn: &Conn) -> (i64, bool) {
        conn.run(|connection| {
            let query_only = connection
                .query_row("PRAGMA query_only", [], |row| row.get::<_, i64>(0))
                .map_err(map_error)?;
            Ok((query_only, connection.is_autocommit()))
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn sqlite_denies_a_script_what_would_end_its_transaction_or_lift_query_only() {
        for access in [Access::ReadOnly, Access::Writable] {
            let (conn, _dir) = fixture_as(access).await;
            for text in [
                "PRAGMA query_only = OFF",
                "PRAGMA 'query_only' = 0",
                // SQLite hands the name over in these letters.
                "PRAGMA QUERY_ONLY = 0",
                "EXPLAIN PRAGMA query_only = 0",
                "PRAGMA main.query_only(0)",
                "PRAGMA writable_schema = ON",
                "PRAGMA wal_checkpoint",
                "PRAGMA wal_checkpoint(TRUNCATE)",
                "COMMIT",
                "END",
                "ROLLBACK",
                "SAVEPOINT s",
                // A byte-order mark is whitespace to SQLite.
                "\u{feff}COMMIT",
                // Two statements: to SQLite `:a(')` is one variable, to our
                // tokenizer the rest is a string.
                "SELECT :a('); PRAGMA query_only = 0; --'",
            ] {
                // A statement's error, not a closed session.
                let ran = run_unrefused(&conn, &[text]).await.unwrap();
                assert!(
                    matches!(
                        ran.results.last().map(|result| &result.outcome),
                        Some(StatementOutcome::Error {
                            error: Error::Query { code: Some(code), message, .. },
                            ..
                        }) if code == "23" && message.contains("not authorized")
                    ),
                    "{access:?} {text}: {ran:?}"
                );
                assert_eq!(standing(&conn).await, (1, true), "{access:?} {text}");
            }
        }
    }

    #[tokio::test]
    async fn a_script_still_reads_and_sets_what_the_fence_leaves_it() {
        for access in [Access::ReadOnly, Access::Writable] {
            let (conn, _dir) = fixture_as(access).await;
            for text in [
                "PRAGMA query_only",
                "PRAGMA table_info(users)",
                // Asks for the pragma while it runs, not when it is prepared.
                "SELECT name FROM pragma_table_info('users')",
                "SELECT count(*) FROM users",
            ] {
                let ran = run_unrefused(&conn, &[text]).await.unwrap();
                assert!(
                    matches!(
                        ran.results.last().map(|result| &result.outcome),
                        Some(StatementOutcome::Rows { .. })
                    ),
                    "{access:?} {text}: {ran:?}"
                );
            }
            // A setting that is the script's to change, read back in the
            // same script.
            let ran = run_unrefused(&conn, &["PRAGMA cache_size = 1234", "PRAGMA cache_size"])
                .await
                .unwrap();
            assert!(
                matches!(
                    ran.results.last().map(|result| &result.outcome),
                    Some(StatementOutcome::Rows { rows, .. }) if rows[0][0] == Value::Int(1234)
                ),
                "{access:?}: {ran:?}"
            );
        }
    }

    #[tokio::test]
    async fn a_scripts_write_is_refused_by_query_only_not_by_the_authorizer() {
        for access in [Access::ReadOnly, Access::Writable] {
            let (conn, _dir) = fixture_as(access).await;
            let ran = run_unrefused(&conn, &["UPDATE users SET email = 'x'"])
                .await
                .unwrap();
            // SQLITE_READONLY: the app tells a refused write by it.
            assert!(
                matches!(
                    &ran.results[0].outcome,
                    StatementOutcome::Error {
                        error: Error::Query { code: Some(code), message, .. },
                        ..
                    } if code == "8" && !message.contains("not authorized")
                ),
                "{access:?}: {ran:?}"
            );
        }
    }

    /// A writable session on a file with an FTS5 and an R*Tree table.
    async fn virtual_tables() -> (Conn, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("virtual.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(
                "CREATE VIRTUAL TABLE notes USING fts5(body);
                 INSERT INTO notes (body) VALUES ('a quiet shelf'), ('a loud shelf'), ('the till');
                 CREATE VIRTUAL TABLE areas USING rtree(id, min_x, max_x);
                 INSERT INTO areas VALUES (1, 0, 1), (2, 5, 6), (3, 8, 9);",
            )
            .unwrap();
        (Conn::open(&path, Access::Writable).await.unwrap(), dir)
    }

    #[tokio::test]
    async fn virtual_tables_are_browsed_behind_the_filters_fence() {
        for (table, raw) in [("areas", "min_x >= 5"), ("notes", "notes MATCH 'shelf'")] {
            // Each in a session of its own, and the count first, so that
            // nothing of the app's has touched the table before the fence
            // is up: R*Tree prepares its write statements when the table
            // connects, and FTS5 asks for `PRAGMA data_version` when it
            // first reads.
            let (conn, _dir) = virtual_tables().await;
            let mut query = RowQuery::new(ObjectRef::new("main", table), 10);
            let rows = |page: Result<RowPage>| page.map(|page| page.rows.len());
            assert_eq!(conn.count_rows(&query).await, Ok(3), "{table}");
            assert_eq!(rows(conn.fetch_rows(&query).await), Ok(3), "{table}");
            query.raw_where = Some(raw.into());
            assert_eq!(conn.count_rows(&query).await, Ok(2), "{table} {raw}");
            assert_eq!(rows(conn.fetch_rows(&query).await), Ok(2), "{table} {raw}");
        }
    }

    #[tokio::test]
    async fn a_script_that_ended_its_transaction_runs_nothing_after_it() {
        let (conn, dir) = fixture_as(Access::Writable).await;
        without_the_authorizer(&conn).await;
        let path = dir.path().join("fixture.db");
        let before = std::fs::read(&path).unwrap();
        // query_only is still on after the COMMIT, and does not stop a
        // change of journal mode: only the open transaction does.
        let ran = run_unrefused(&conn, &["COMMIT", "PRAGMA journal_mode = WAL"]).await;
        // Not assert_eq: it would print both files.
        assert!(std::fs::read(&path).unwrap() == before, "the file changed");
        assert!(!dir.path().join("fixture.db-wal").exists());
        assert!(matches!(ran, Err(Error::LeftReadOnly)), "{ran:?}");
    }

    #[test]
    fn a_raw_where_holding_a_statement_separator_is_refused() {
        let mut query = RowQuery::new(ObjectRef::new("main", "users"), 10);
        for raw in [
            "1 = 1; DELETE FROM users",
            "1=1); PRAGMA query_only = 0; SELECT (1",
            "1=1;",
        ] {
            query.raw_where = Some(raw.into());
            let checked = check_raw_where(&query);
            assert!(
                matches!(&checked, Err(Error::Query { message, .. }) if message.contains("`;`")),
                "{raw}: {checked:?}"
            );
        }
        // In a string, a name or a comment it separates nothing.
        for raw in [
            "email LIKE '%;%'",
            "\"a;b\" = 1 OR [a;b] = 1 OR `a;b` = 1",
            "id = 1 /* one; only */",
        ] {
            query.raw_where = Some(raw.into());
            assert_eq!(check_raw_where(&query), Ok(()), "{raw}");
        }
    }

    #[test]
    fn what_the_fence_refuses_a_filter_is_put_in_words() {
        let coded = |code: &str, message: &str| Error::Query {
            code: Some(code.into()),
            message: message.into(),
            detail: None,
            hint: None,
            named: Box::default(),
        };
        // The second is what SQLite says when the refusal comes from inside
        // a running statement.
        for message in ["not authorized", "authorization denied"] {
            assert_eq!(
                filter_denied(coded("23", message)),
                coded(
                    "23",
                    "A filter cannot use a PRAGMA with an argument, ATTACH or a transaction \
                     statement."
                )
            );
        }
        for other in [
            coded("8", "attempt to write a readonly database"),
            coded("1", "no such column: nope"),
            Error::query("not authorized"),
            Error::Cancelled,
        ] {
            assert_eq!(filter_denied(other.clone()), other);
        }
    }

    #[test]
    fn a_raw_where_holding_a_nul_is_refused() {
        let mut query = RowQuery::new(ObjectRef::new("main", "users"), 10);
        // In a string too: SQLite stops reading there whatever it is in.
        for raw in ["1=1) \0", "\0", "email = 'a\0b'"] {
            query.raw_where = Some(raw.into());
            let checked = check_raw_where(&query);
            assert!(
                matches!(&checked, Err(Error::Query { message, .. }) if message.contains("NUL")),
                "{raw:?}: {checked:?}"
            );
        }
    }

    #[tokio::test]
    async fn failed_browsing_puts_the_session_settings_back() {
        for access in [Access::ReadOnly, Access::Writable] {
            let (conn, _dir) = fixture_as(access).await;
            // What a WHERE text would do if its `;` got past
            // `check_raw_where` and the fence, which this closure does not
            // put up: rusqlite prepares the second statement, which is all
            // a flag pragma needs, and then refuses the text.
            let failed = conn
                .browse(|connection| {
                    connection
                        .prepare("SELECT 1; PRAGMA query_only = 0")
                        .map(|_| ())
                        .map_err(map_error)
                })
                .await;
            assert!(matches!(failed, Err(Error::Query { .. })), "{failed:?}");
            let query_only = conn
                .run(|connection| {
                    connection
                        .query_row("PRAGMA query_only", [], |row| row.get::<_, i64>(0))
                        .map_err(map_error)
                })
                .await
                .unwrap();
            assert_eq!(query_only, 1, "{access:?}");
        }
    }

    #[tokio::test]
    async fn a_statement_after_query_only_went_off_does_not_run() {
        // The COMMIT would keep the UPDATE whatever the end of the run did,
        // so the UPDATE must not run at all.
        let (conn, _dir) = fixture_as(Access::Writable).await;
        without_the_authorizer(&conn).await;
        let ran = run_unrefused(
            &conn,
            &[
                "PRAGMA query_only = OFF",
                "UPDATE users SET email = 'x' WHERE id = 1",
                "COMMIT",
            ],
        )
        .await;
        let changed = conn
            .run(|connection| {
                connection
                    .query_row("SELECT count(*) FROM users WHERE email = 'x'", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .map_err(map_error)
            })
            .await
            .unwrap();
        assert_eq!(changed, 0);
        assert!(matches!(ran, Err(Error::LeftReadOnly)), "{ran:?}");
    }

    #[tokio::test]
    async fn a_save_that_cannot_take_the_file_leaves_the_session_as_it_was() {
        let (conn, dir) = fixture_as(Access::Writable).await;
        // Another program is in the middle of a write.
        let other = rusqlite::Connection::open(dir.path().join("fixture.db")).unwrap();
        other
            .execute_batch("BEGIN IMMEDIATE; UPDATE users SET name = 'Theirs' WHERE id = 2")
            .unwrap();
        // The session waits five seconds for a busy file, and a save does
        // not set that wait. The test shortens it here, where it can reach
        // it, rather than sit it out.
        conn.run(|connection| {
            connection
                .busy_timeout(std::time::Duration::ZERO)
                .map_err(map_error)
        })
        .await
        .unwrap();
        let changes = ChangeSet {
            object: ObjectRef::new("main", "users"),
            inserts: Vec::new(),
            rows: vec![crate::RowChange {
                key: vec![("id".into(), Value::Int(1))],
                set: vec![crate::CellChange {
                    column: "name".into(),
                    type_name: "TEXT".into(),
                    loaded: Value::Text("Ada Lovelace".into()),
                    new: crate::NewValue::Text("Mine".into()),
                }],
            }],
        };
        // The save's own error, at its BEGIN, which asks for the file. A
        // transaction that asked only at its first write would have read
        // the row and failed at the UPDATE, as that row's failure.
        let refused = conn.write(&changes, &StopFlag::new()).await;
        assert!(
            matches!(
                &refused,
                Err(Error::Query { code: Some(code), message, .. })
                    if code == "5" && message.contains("locked")
            ),
            "{refused:?}"
        );
        // The session refuses writes as before, the usual way.
        assert_eq!(standing(&conn).await, (1, true));
        let ran = run_unrefused(&conn, &["UPDATE users SET email = 'x'"])
            .await
            .unwrap();
        assert!(
            matches!(
                &ran.results[0].outcome,
                StatementOutcome::Error {
                    error: Error::Query { code: Some(code), .. },
                    ..
                } if code == "8"
            ),
            "{ran:?}"
        );
        // Once the other program lets go, the same save is written.
        other.execute_batch("ROLLBACK").unwrap();
        let outcome = conn.write(&changes, &StopFlag::new()).await;
        assert!(
            matches!(outcome, Ok(WriteOutcome::Written { .. })),
            "{outcome:?}"
        );
    }

    #[tokio::test]
    async fn a_save_does_not_commit_what_a_transaction_left_open_wrote() {
        let (conn, dir) = fixture_as(Access::Writable).await;
        // Nothing in the app leaves a transaction open; this one is planted,
        // with a write of its own in it.
        conn.run(|connection| {
            connection
                .execute_batch(
                    "PRAGMA query_only = OFF; BEGIN; \
                     UPDATE users SET name = 'Planted' WHERE id = 2",
                )
                .map_err(map_error)
        })
        .await
        .unwrap();
        let changes = ChangeSet {
            object: ObjectRef::new("main", "users"),
            inserts: Vec::new(),
            rows: vec![crate::RowChange {
                key: vec![("id".into(), Value::Int(1))],
                set: vec![crate::CellChange {
                    column: "name".into(),
                    type_name: "TEXT".into(),
                    loaded: Value::Text("Ada Lovelace".into()),
                    new: crate::NewValue::Text("Mine".into()),
                }],
            }],
        };
        // The save goes through, and only its own row is written.
        let outcome = conn.write(&changes, &StopFlag::new()).await;
        assert!(
            matches!(outcome, Ok(WriteOutcome::Written { .. })),
            "{outcome:?}"
        );
        let other = rusqlite::Connection::open(dir.path().join("fixture.db")).unwrap();
        let name = |id: i64| -> String {
            other
                .query_row("SELECT name FROM users WHERE id = ?1", [id], |row| {
                    row.get(0)
                })
                .unwrap()
        };
        assert_eq!((name(1), name(2)), ("Mine".to_owned(), "Bob".to_owned()));
        assert_eq!(standing(&conn).await, (1, true));
    }

    #[test]
    fn only_a_name_sqlite_would_read_as_a_uri_is_rewritten() {
        let dot = |name: &str| Path::new(".").join(name);
        assert_eq!(
            plain_file_name(Path::new("file:shop.db")),
            dot("file:shop.db")
        );
        assert_eq!(
            plain_file_name(Path::new("file:shop.db?immutable=1")),
            dot("file:shop.db?immutable=1")
        );
        for name in [
            "shop.db",
            "/data/file:shop.db",
            "files/shop.db",
            "FILE:shop.db",
        ] {
            assert!(matches!(
                plain_file_name(Path::new(name)),
                Cow::Borrowed(path) if path == Path::new(name)
            ));
        }
    }

    #[tokio::test]
    async fn a_non_database_file_is_a_connect_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(
            &path,
            "these are not the tables you are looking for, not at all",
        )
        .unwrap();
        assert!(matches!(
            Conn::open(&path, Access::ReadOnly).await,
            Err(Error::Connect(_))
        ));
    }

    #[tokio::test]
    async fn the_connection_refuses_writes() {
        let (conn, _dir) = fixture().await;
        let result = conn
            .run(|connection| {
                connection
                    .execute("DELETE FROM users", [])
                    .map_err(map_error)
            })
            .await;
        assert!(result.is_err());
        let count = conn
            .run(|connection| {
                connection
                    .query_row("SELECT count(*) FROM users", [], |row| row.get::<_, i64>(0))
                    .map_err(map_error)
            })
            .await
            .unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn schemas_are_main_and_attached_databases_without_temp() {
        let (conn, _dir) = fixture().await;
        assert_eq!(conn.list_schemas().await.unwrap(), vec!["main".to_owned()]);
    }

    #[tokio::test]
    async fn objects_are_sorted_tables_and_views_without_internal_tables() {
        let (conn, _dir) = fixture().await;
        let objects = conn.list_objects("main").await.unwrap();
        let names: Vec<(&str, ObjectKind)> = objects
            .iter()
            .map(|object| (object.name.as_str(), object.kind))
            .collect();
        assert_eq!(
            names,
            vec![
                ("active_users", ObjectKind::View),
                ("big", ObjectKind::Table),
                ("events", ObjectKind::Table),
                ("orders", ObjectKind::Table),
                ("users", ObjectKind::Table),
                ("weird \"name\"", ObjectKind::Table),
            ]
        );
        assert!(objects.iter().all(|object| object.estimated_rows.is_none()));
    }

    #[tokio::test]
    async fn users_structure_has_columns_key_and_indexes() {
        let (conn, _dir) = fixture().await;
        let structure = conn
            .describe(&ObjectRef::new("main", "users"))
            .await
            .unwrap();
        let columns: Vec<&str> = structure.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            columns,
            [
                "id",
                "email",
                "name",
                "created_at",
                "active",
                "meta",
                "avatar",
                "score"
            ]
        );
        let email = &structure.columns[1];
        assert_eq!(email.type_name, "TEXT");
        assert!(!email.nullable);
        let created = &structure.columns[3];
        assert_eq!(created.default.as_deref(), Some("'2026-01-01 00:00:00'"));
        assert_eq!(structure.primary_key, vec!["id".to_owned()]);
        let name_index = structure
            .indexes
            .iter()
            .find(|index| index.name == "users_name_idx")
            .unwrap();
        assert_eq!(name_index.columns, vec!["name".to_owned()]);
        assert!(!name_index.unique);
        assert!(
            structure
                .indexes
                .iter()
                .any(|index| index.unique && index.columns == vec!["email".to_owned()]),
            "the UNIQUE constraint's automatic index is listed"
        );
    }

    #[tokio::test]
    async fn orders_structure_has_a_foreign_key_and_a_composite_index() {
        let (conn, _dir) = fixture().await;
        let structure = conn
            .describe(&ObjectRef::new("main", "orders"))
            .await
            .unwrap();
        assert_eq!(structure.foreign_keys.len(), 1);
        let key = &structure.foreign_keys[0];
        assert_eq!(key.name, None);
        assert_eq!(key.columns, vec!["user_id".to_owned()]);
        assert_eq!(key.ref_table, "users");
        assert_eq!(key.ref_columns, vec!["id".to_owned()]);
        assert_eq!(key.on_delete, "CASCADE");
        let composite = structure
            .indexes
            .iter()
            .find(|index| index.name == "orders_user_total_idx")
            .unwrap();
        assert_eq!(
            composite.columns,
            vec!["user_id".to_owned(), "total".to_owned()]
        );
    }

    #[tokio::test]
    async fn views_have_columns_but_no_key() {
        let (conn, _dir) = fixture().await;
        let structure = conn
            .describe(&ObjectRef::new("main", "active_users"))
            .await
            .unwrap();
        assert_eq!(structure.columns.len(), 2);
        assert!(structure.primary_key.is_empty());
        assert!(
            conn.primary_key(&ObjectRef::new("main", "events"))
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn a_generated_column_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("generated.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(
                "CREATE TABLE books (
                     id INTEGER PRIMARY KEY,
                     title TEXT NOT NULL,
                     slug TEXT GENERATED ALWAYS AS (lower(title)) VIRTUAL,
                     shout TEXT GENERATED ALWAYS AS (upper(title)) STORED
                 )",
            )
            .unwrap();
        let conn = Conn::open(&path, Access::ReadOnly).await.unwrap();
        let structure = conn
            .describe(&ObjectRef::new("main", "books"))
            .await
            .unwrap();
        let generated: Vec<(&str, bool)> = structure
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column.generated))
            .collect();
        assert_eq!(
            generated,
            [
                ("id", false),
                ("title", false),
                ("slug", true),
                ("shout", true)
            ]
        );
    }

    #[tokio::test]
    async fn the_rowids_alias_is_an_identity_column() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(
                "CREATE TABLE books (id INTEGER PRIMARY KEY, title TEXT NOT NULL);
                 CREATE TABLE tags (name TEXT PRIMARY KEY, id INTEGER);
                 CREATE TABLE pairs (a INTEGER, b INTEGER, PRIMARY KEY (a, b));
                 CREATE TABLE codes (id INTEGER PRIMARY KEY, label TEXT) WITHOUT ROWID;
                 CREATE TABLE counts (id INT PRIMARY KEY, n INTEGER);
                 CREATE TABLE downs (id INTEGER PRIMARY KEY DESC, label TEXT);
                 CREATE TABLE lasts (id INTEGER, label TEXT, PRIMARY KEY (id DESC))",
            )
            .unwrap();
        let conn = Conn::open(&path, Access::ReadOnly).await.unwrap();
        let mut found = Vec::new();
        for table in [
            "books", "tags", "pairs", "codes", "counts", "downs", "lasts",
        ] {
            let structure = conn.describe(&ObjectRef::new("main", table)).await.unwrap();
            let identity: Vec<String> = structure
                .columns
                .into_iter()
                .filter(|column| column.identity)
                .map(|column| column.name)
                .collect();
            found.push((table, identity));
        }
        // Only a rowid's alias: one key column, declared INTEGER and
        // nothing else, in a table that has a rowid.
        assert_eq!(
            found,
            [
                ("books", vec!["id".to_owned()]),
                ("tags", Vec::new()),
                ("pairs", Vec::new()),
                ("codes", Vec::new()),
                ("counts", Vec::new()),
                // `DESC` on the column makes it a column like any other.
                ("downs", Vec::new()),
                // On the key it does not.
                ("lasts", vec!["id".to_owned()]),
            ]
        );
    }

    #[tokio::test]
    async fn a_partial_index_says_so_and_is_not_the_row_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("partial.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(
                "CREATE TABLE t (a INTEGER NOT NULL, b INTEGER NOT NULL);
                 CREATE UNIQUE INDEX whole ON t (a);
                 CREATE UNIQUE INDEX part ON t (b) WHERE b > 0;",
            )
            .unwrap();
        let conn = Conn::open(&path, Access::ReadOnly).await.unwrap();
        let structure = conn.describe(&ObjectRef::new("main", "t")).await.unwrap();
        let partial: Vec<(&str, bool)> = structure
            .indexes
            .iter()
            .map(|index| (index.name.as_str(), index.partial))
            .collect();
        assert_eq!(partial, [("part", true), ("whole", false)]);
        // `part` comes first by name and its column cannot be NULL: only
        // its condition keeps it from being the key.
        assert_eq!(structure.row_key(), Some(vec!["a".to_owned()]));
    }

    #[tokio::test]
    async fn only_an_index_over_whole_columns_is_the_row_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(
                r#"CREATE TABLE t (
                       a INTEGER NOT NULL,
                       b TEXT NOT NULL,
                       c INTEGER NOT NULL,
                       "<expression>" INTEGER NOT NULL
                   );
                   CREATE UNIQUE INDEX i1_expression ON t (a + 1);
                   CREATE UNIQUE INDEX i2_mixed ON t (c, lower(b));
                   CREATE UNIQUE INDEX i3_plain ON t (c, b DESC);"#,
            )
            .unwrap();
        let conn = Conn::open(&path, Access::ReadOnly).await.unwrap();
        let structure = conn.describe(&ObjectRef::new("main", "t")).await.unwrap();
        let list = |names: &[&str]| -> Vec<String> {
            names.iter().map(|name| (*name).to_owned()).collect()
        };
        let indexes: Vec<_> = structure
            .indexes
            .iter()
            .map(|index| {
                (
                    index.name.as_str(),
                    index.columns.clone(),
                    index.key_columns.clone(),
                )
            })
            .collect();
        assert_eq!(
            indexes,
            [
                ("i1_expression", list(&["<expression>"]), None),
                ("i2_mixed", list(&["c", "<expression>"]), None),
                ("i3_plain", list(&["c", "b"]), Some(list(&["c", "b"]))),
            ]
        );
        // Not the column that is called what an expression shows as.
        assert_eq!(structure.row_key(), Some(list(&["c", "b"])));
    }

    #[tokio::test]
    async fn a_primary_key_of_any_kind_is_the_row_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("primary.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(
                "CREATE TABLE alias (id INTEGER PRIMARY KEY, n INTEGER);
                 CREATE TABLE named (code TEXT PRIMARY KEY, n INTEGER);
                 CREATE TABLE pair (a INTEGER, b TEXT, n INTEGER, PRIMARY KEY (b, a));
                 CREATE TABLE clustered (a INTEGER, b TEXT, n INTEGER, PRIMARY KEY (b, a))
                     WITHOUT ROWID;",
            )
            .unwrap();
        let conn = Conn::open(&path, Access::ReadOnly).await.unwrap();
        for (table, key) in [
            // The rowid's alias has no index of its own.
            ("alias", vec!["id"]),
            ("named", vec!["code"]),
            ("pair", vec!["b", "a"]),
            ("clustered", vec!["b", "a"]),
        ] {
            let structure = conn.describe(&ObjectRef::new("main", table)).await.unwrap();
            let key: Vec<String> = key.into_iter().map(str::to_owned).collect();
            assert_eq!(structure.row_key(), Some(key), "{table}");
        }
    }

    #[tokio::test]
    async fn describing_a_missing_object_is_a_query_error() {
        let (conn, _dir) = fixture().await;
        let result = conn.describe(&ObjectRef::new("main", "nope")).await;
        assert!(matches!(result, Err(Error::Query { .. })), "{result:?}");
    }
}
