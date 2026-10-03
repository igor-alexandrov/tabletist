//! SQLite, opened read-only unless the connection is writable. rusqlite is
//! blocking, so every call runs on tokio's blocking pool.

use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rusqlite::config::DbConfig;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::{ErrorCode, OpenFlags};

use crate::{
    Access, ColumnInfo, ColumnMeta, Dialect, Error, ForeignKeyInfo, IndexInfo, MAX_LISTED,
    ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery, ScriptOutcome, StatementOutcome,
    StatementResult, StopFlag, Structure, Value, ValueKind,
};

/// An open SQLite database.
pub struct Conn {
    inner: Arc<Mutex<rusqlite::Connection>>,
    interrupt: Arc<rusqlite::InterruptHandle>,
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

/// Whether the session still refuses writes. A script can turn
/// `query_only` off by a spelling the refusal list does not know, and on a
/// writable handle nothing else would stop its next statement.
fn still_query_only(connection: &rusqlite::Connection) -> Result<bool> {
    connection
        .query_row("PRAGMA query_only", [], |row| row.get::<_, i64>(0))
        .map(|on| on == 1)
        .map_err(map_error)
}

/// Runs `texts` between `BEGIN` and a `ROLLBACK` that always happens. A
/// run that finds `query_only` off, before a statement or at its end, ends
/// with `LeftReadOnly` after that rollback.
fn script(
    connection: &rusqlite::Connection,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
) -> Result<ScriptOutcome> {
    let mut outcome = ScriptOutcome::default();
    if stop.is_stopped() {
        outcome.stopped = true;
        return Ok(outcome);
    }
    // A cancel (the session's interrupt) can land while BEGIN runs; that
    // ends the run with no results.
    match connection
        .execute_batch("BEGIN DEFERRED")
        .map_err(map_error)
    {
        Err(Error::Cancelled) => {
            outcome.stopped = true;
            // The interrupt may have left a transaction open.
            stop.finish();
            end_transaction(connection).map_err(|error| crate::script::cleanup_failed(&error))?;
            return Ok(outcome);
        }
        Err(error) => return Err(error),
        Ok(()) => {}
    }
    // SQLite calls this every 1000 virtual machine steps; `true` interrupts
    // the running statement, which fails with SQLITE_INTERRUPT (Cancelled).
    let watching = stop.clone();
    connection.progress_handler(1_000, Some(move || watching.is_stopped()));
    let ran = statements(connection, texts, limit, stop, &mut outcome);
    // Removed before the cleanup, so a stop cannot interrupt it.
    connection.progress_handler(0, None::<fn() -> bool>);
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
    ended.map_err(|error| crate::script::cleanup_failed(&error))?;
    if left {
        return Err(Error::LeftReadOnly);
    }
    Ok(outcome)
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
    texts: &[String],
    limit: usize,
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
        match still_query_only(connection) {
            Ok(true) => {}
            Ok(false) => return Err(Error::LeftReadOnly),
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
            Err(error) => return Err(error),
        }
        let started = Instant::now();
        let result = match statement(connection, text, limit) {
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
        return Ok(StatementOutcome::Done { affected: None });
    }
    // prepare refuses a second statement in the text (MultipleStatement).
    let mut statement = connection.prepare(text).map_err(map_error)?;
    let declared = declared_columns(connection, &statement, text)?;
    if declared.is_empty() {
        let changed = statement.raw_execute().map_err(map_error)?;
        return Ok(StatementOutcome::Done {
            affected: counts_changes(text).then_some(changed as u64),
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
/// schema, a busy timeout and LIKE ignoring case (the filters rely on it).
/// `open` sets them, and a script run sets them again afterwards, since a
/// script may have changed any of them.
fn set_session_pragmas(connection: &rusqlite::Connection) -> rusqlite::Result<()> {
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    connection.execute_batch(
        "PRAGMA query_only = ON; PRAGMA trusted_schema = OFF; PRAGMA case_sensitive_like = OFF;",
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

/// Refuses a raw WHERE that ends inside a `/*` comment, which SQLite would
/// accept and which would hide the page's ORDER BY, LIMIT and OFFSET.
fn check_raw_where(query: &RowQuery) -> Result<()> {
    if query
        .raw_where
        .as_deref()
        .is_some_and(crate::dialect::sqlite_ends_in_block_comment)
    {
        return Err(Error::query(
            "The WHERE text ends inside a /* comment. Close it with */.",
        ));
    }
    Ok(())
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
        let connection = tokio::task::spawn_blocking(move || -> Result<rusqlite::Connection> {
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
            Ok(connection)
        })
        .await
        .map_err(|error| Error::Io(error.to_string()))??;
        let interrupt = Arc::new(connection.get_interrupt_handle());
        Ok(Self {
            inner: Arc::new(Mutex::new(connection)),
            interrupt,
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

    /// See [`crate::Connection::run_script`]. The whole script is one
    /// blocking job; a progress handler checks `stop` while a statement
    /// runs.
    pub async fn run_script(
        &self,
        texts: Vec<String>,
        limit: u32,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let stop = stop.clone();
        let limit = limit as usize;
        // If the caller drops this future, `run` interrupts the statement
        // that is running; this stops the ones that have not begun.
        let guard = StopOnDrop(Some(stop.clone()));
        let outcome = self
            .run(move |connection| script(connection, &texts, limit, &stop))
            .await;
        guard.disarm();
        outcome
    }

    /// The server's name and version for the footer, like `SQLite 3.46.0`.
    pub async fn server_version(&self) -> Result<String> {
        self.run(|connection| {
            let version: String = connection
                .query_row("SELECT sqlite_version()", [], |row| row.get(0))
                .map_err(map_error)?;
            Ok(format!("SQLite {version}"))
        })
        .await
    }

    pub(crate) fn interrupt_handle(&self) -> Arc<rusqlite::InterruptHandle> {
        Arc::clone(&self.interrupt)
    }

    /// `main` plus attached databases (`temp` is hidden).
    pub async fn list_schemas(&self) -> Result<Vec<String>> {
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

    pub async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
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

    /// The primary key columns in key order; empty for views and keyless tables.
    #[cfg(test)]
    pub async fn primary_key(&self, object: &ObjectRef) -> Result<Vec<String>> {
        let object = object.clone();
        self.run(move |connection| primary_key(connection, &object))
            .await
    }

    pub async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
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

    pub async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        check_raw_where(query)?;
        let query = query.clone();
        let limit = query.limit as usize;
        // One blocking job for the key lookup and the select, so a cancel
        // can never fall in a gap between them.
        self.run(move |connection| {
            let key = ordering_key(connection, &query.object)?;
            let binary = binary_columns(connection, &query)?;
            let sql = Dialect::Sqlite.select_rows(&query, &key, &binary);
            let ordered_by_key = !key.is_empty();
            let started = Instant::now();
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

    pub async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        check_raw_where(query)?;
        let query = query.clone();
        self.run(move |connection| {
            let binary = binary_columns(connection, &query)?;
            let sql = Dialect::Sqlite.count_rows(&query, &binary);
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
}

fn columns(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<ColumnInfo>> {
    let mut statement = connection
        .prepare(
            "SELECT name, type, \"notnull\", dflt_value FROM pragma_table_xinfo(?1, ?2) \
             WHERE hidden <> 1 ORDER BY cid",
        )
        .map_err(map_error)?;
    statement
        .query_map([&object.name, &object.schema], |row| {
            Ok(ColumnInfo {
                name: text(row, 0)?,
                type_name: optional_text(row, 1)?.unwrap_or_default(),
                nullable: row.get::<_, i64>(2)? == 0,
                default: optional_text(row, 3)?,
                comment: None,
                allowed_values: None,
            })
        })
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)
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
        .prepare("SELECT name, \"unique\", origin FROM pragma_index_list(?1, ?2) ORDER BY name")
        .map_err(map_error)?;
    let entries = list
        .query_map([&object.name, &object.schema], |row| {
            Ok((
                row.get_ref(0)?.as_bytes()?.to_vec(),
                row.get::<_, i64>(1)? != 0,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)?;
    let mut info = connection
        .prepare("SELECT name FROM pragma_index_info(?1, ?2) ORDER BY seqno")
        .map_err(map_error)?;
    let mut indexes = Vec::new();
    for (name, unique, origin) in entries {
        // The name goes back to SQLite as the bytes it gave: one that is not
        // UTF-8 would find no index once its bytes were replaced.
        let columns = info
            .query_map(rusqlite::params![name, object.schema], |row| {
                optional_text(row, 0)
            })
            .map_err(map_error)?
            .map(|column| column.map(|c| c.unwrap_or_else(|| "<expression>".into())))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(map_error)?;
        indexes.push(IndexInfo {
            name: String::from_utf8_lossy(&name).into_owned(),
            columns,
            unique,
            primary: origin == "pk",
            method: None,
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
    use super::{code_only, counts_changes};

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

    #[tokio::test]
    async fn a_script_that_gets_query_only_off_is_stopped_and_writes_nothing() {
        for access in [Access::ReadOnly, Access::Writable] {
            for texts in [
                vec!["PRAGMA query_only = OFF", "UPDATE users SET email = 'x'"],
                // In last position, where no statement follows to be checked.
                vec!["SELECT 1", "PRAGMA query_only = OFF"],
            ] {
                let (conn, _dir) = fixture_as(access).await;
                let script = texts.iter().map(|text| (*text).to_owned()).collect();
                let ran = conn.run_script(script, 10, &StopFlag::new()).await;
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
    async fn describing_a_missing_object_is_a_query_error() {
        let (conn, _dir) = fixture().await;
        let result = conn.describe(&ObjectRef::new("main", "nope")).await;
        assert!(matches!(result, Err(Error::Query { .. })), "{result:?}");
    }
}
