//! SQLite, opened read-only. rusqlite is blocking, so every call runs on
//! tokio's blocking pool.

use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rusqlite::config::DbConfig;
use rusqlite::{ErrorCode, OpenFlags};

use crate::{
    ColumnInfo, ColumnMeta, Dialect, Error, ForeignKeyInfo, IndexInfo, MAX_LISTED, ObjectInfo,
    ObjectKind, ObjectRef, Result, RowPage, RowQuery, ScriptOutcome, StatementOutcome,
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

/// Runs `texts` between `BEGIN` and a `ROLLBACK` that always happens.
fn script(
    connection: &rusqlite::Connection,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
) -> Result<ScriptOutcome> {
    let mut outcome = ScriptOutcome::default();
    if stop.is_stopped() {
        return Ok(outcome);
    }
    // A cancel (the session's interrupt) can land while BEGIN runs; that
    // ends the run with no results.
    match connection
        .execute_batch("BEGIN DEFERRED")
        .map_err(map_error)
    {
        Err(Error::Cancelled) => return Ok(outcome),
        Err(error) => return Err(error),
        Ok(()) => {}
    }
    // SQLite calls this every 1000 virtual machine steps; `true` interrupts
    // the running statement, which fails with SQLITE_INTERRUPT (Cancelled).
    let watching = stop.clone();
    connection.progress_handler(1_000, Some(move || watching.is_stopped()));
    let ran = statements(connection, texts, limit, stop, &mut outcome);
    // Removed before the rollback, so a stop cannot interrupt the cleanup.
    connection.progress_handler(0, None::<fn() -> bool>);
    // An error can end SQLite's transaction by itself; roll back only one
    // that is still open.
    let rolled_back = if connection.is_autocommit() {
        Ok(())
    } else {
        connection
            .execute_batch("ROLLBACK")
            .or_else(|_| connection.execute_batch("ROLLBACK"))
            .map_err(map_error)
    };
    ran?;
    rolled_back.map_err(|error| crate::script::cleanup_failed(&error))?;
    Ok(outcome)
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
            outcome.results.push(StatementResult {
                elapsed: std::time::Duration::ZERO,
                outcome: StatementOutcome::Cancelled,
            });
            break;
        }
        let started = Instant::now();
        let result = match statement(connection, text, limit) {
            Ok(result) => result,
            Err(error) => crate::script::statement_failed(error, None)?,
        };
        let last = matches!(
            result,
            StatementOutcome::Error { .. } | StatementOutcome::Cancelled
        );
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

fn statement(
    connection: &rusqlite::Connection,
    text: &str,
    limit: usize,
) -> Result<StatementOutcome> {
    // prepare refuses a second statement in the text (MultipleStatement).
    let mut statement = connection.prepare(text).map_err(map_error)?;
    let declared: Vec<(String, String)> = statement
        .columns()
        .iter()
        .map(|column| {
            (
                column.name().to_owned(),
                column.decl_type().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    if declared.is_empty() {
        let changed = statement.raw_execute().map_err(map_error)?;
        return Ok(StatementOutcome::Done {
            affected: Some(changed as u64),
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

fn from_sqlite(value: rusqlite::types::ValueRef<'_>) -> Value {
    use rusqlite::types::ValueRef;
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(number) => Value::Int(number),
        ValueRef::Real(number) => Value::Float(number),
        ValueRef::Text(bytes) => Value::Text(String::from_utf8_lossy(bytes).into()),
        ValueRef::Blob(bytes) => Value::Bytes(bytes.into()),
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
    /// Opens `path` read-only. Never creates a file.
    pub async fn open(path: &Path) -> Result<Self> {
        let path = path.to_path_buf();
        let connection = tokio::task::spawn_blocking(move || -> Result<rusqlite::Connection> {
            if !path.is_file() {
                return Err(Error::Connect(format!("{} does not exist", path.display())));
            }
            let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
            let connection = rusqlite::Connection::open_with_flags(plain_file_name(&path), flags)
                .map_err(map_error)?;
            connection
                .busy_timeout(std::time::Duration::from_secs(5))
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
            connection
                .execute_batch("PRAGMA query_only = ON; PRAGMA trusted_schema = OFF;")
                .map_err(map_error)?;
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
        result.map_err(|error| Error::Io(error.to_string()))?
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
        self.run(move |connection| script(connection, &texts, limit, &stop))
            .await
    }

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
                        name: row.get(0)?,
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
            let key = primary_key(connection, &query.object)?;
            let sql = Dialect::Sqlite.select_rows(&query, &key);
            let ordered_by_key = !key.is_empty();
            let started = Instant::now();
            let mut statement = connection.prepare(&sql.text).map_err(map_error)?;
            let declared: Vec<(String, String)> = statement
                .columns()
                .iter()
                .map(|column| {
                    (
                        column.name().to_owned(),
                        column.decl_type().unwrap_or_default().to_owned(),
                    )
                })
                .collect();
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
        let sql = Dialect::Sqlite.count_rows(query);
        self.run(move |connection| {
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
                name: row.get(0)?,
                type_name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                nullable: row.get::<_, i64>(2)? == 0,
                default: row.get(3)?,
                comment: None,
                allowed_values: None,
            })
        })
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)
}

fn primary_key(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<String>> {
    let mut statement = connection
        .prepare("SELECT name FROM pragma_table_info(?1, ?2) WHERE pk > 0 ORDER BY pk")
        .map_err(map_error)?;
    statement
        .query_map([&object.name, &object.schema], |row| {
            row.get::<_, String>(0)
        })
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
                row.get::<_, String>(0)?,
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
        let columns = info
            .query_map([&name, &object.schema], |row| {
                row.get::<_, Option<String>>(0)
            })
            .map_err(map_error)?
            .map(|column| column.map(|c| c.unwrap_or_else(|| "<expression>".into())))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(map_error)?;
        indexes.push(IndexInfo {
            name,
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
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
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
    use super::*;

    async fn fixture() -> (Conn, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.db");
        crate::fixtures::write_sqlite_demo(&path).unwrap();
        (Conn::open(&path).await.unwrap(), dir)
    }

    #[tokio::test]
    async fn a_missing_file_is_a_connect_error_and_is_not_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.db");
        assert!(matches!(Conn::open(&path).await, Err(Error::Connect(_))));
        assert!(!path.exists());
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
        assert!(matches!(Conn::open(&path).await, Err(Error::Connect(_))));
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
