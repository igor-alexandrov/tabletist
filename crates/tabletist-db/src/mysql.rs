//! MySQL 8.0+. Rows and catalog queries use prepared statements (the binary
//! protocol): typed values, column types, and never more than one statement.
//! Rows are read inside a read-only transaction. A SQL editor script runs
//! its statements the same way, in one read-only transaction, and the
//! session is reset afterwards: MySQL session state is not transactional.
//!
//! MariaDB is meant to work, but the tests run against MySQL only. What is
//! written for MariaDB alone (in `script`) has never met a MariaDB server:
//! the older name of the read-only setting (`READ_ONLY_SETTINGS`), its
//! version for the session reset (`can_reset`), and its marking of a
//! read-only transaction in the server status (`begin`).

use std::borrow::Cow;
use std::path::Path;
use std::time::{Duration, Instant};

use mysql_async::consts::{ColumnFlags, ColumnType, StatusFlags};
use mysql_async::prelude::Queryable;
use mysql_async::{DriverError, IoError, Opts, OptsBuilder, Params, SslOpts, TxOpts};
use mysql_common::named_params::ParsedNamedParams;

use crate::adapter::Adapter;
use crate::script::retry_cancelled;
use crate::{
    Access, CancelHandle, CancelInner, ChangeSet, ColumnInfo, ColumnMeta, ConnectSpec, Dialect,
    Error, ForeignKeyInfo, IndexInfo, MAX_LISTED, ObjectInfo, ObjectKind, ObjectRef, Result,
    RowPage, RowQuery, ScriptMode, ScriptOutcome, Secrets, StopFlag, Structure, TlsMode, Value,
    ValueKind, WriteOutcome,
};

/// MySQL's `binary` character set: bytes, not text.
const BINARY_CHARSET: u16 = 63;
const ACCESS_DENIED: u16 = 1045;
const QUERY_INTERRUPTED: u16 = 1317;
const UNKNOWN_SYSTEM_VARIABLE: u16 = 1193;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

mod script;
mod write;

pub(crate) struct Conn {
    pub(crate) conn: tokio::sync::Mutex<mysql_async::Conn>,
    /// For KILL QUERY from a second connection.
    pub(crate) opts: Opts,
    /// This session's thread id on the server.
    pub(crate) id: u32,
    /// Whether the session runs over TLS: under `prefer` a server without
    /// TLS gets plain text.
    pub(crate) encrypted: bool,
    /// The server that thread id belongs to (see `server_identity`).
    pub(crate) server: String,
    /// `SELECT VERSION()` at connect, like `8.4.3` or
    /// `10.11.6-MariaDB-1:10.11.6+maria~ubu2204`.
    pub(crate) version: String,
    /// What the session was opened as: a script puts it back.
    pub(crate) access: Access,
}

impl Conn {
    /// Connects to the spec's server, or through a tunnel's local port `via`.
    pub async fn connect(
        spec: &ConnectSpec,
        secrets: &Secrets,
        via: Option<u16>,
        access: Access,
    ) -> Result<Self> {
        if spec.user.trim().is_empty() {
            return Err(Error::InvalidSpec("enter a user name".into()));
        }
        if let Some(path) = &spec.ca_file
            && !path.is_file()
        {
            return Err(Error::Tls(format!("could not read {}", path.display())));
        }
        // mysql_async cannot check the chain while skipping the host name
        // with rustls 0.23: it fails every certificate, so say why up front.
        if spec.tls == TlsMode::VerifyCa {
            return Err(Error::Tls(
                "verify-ca is not available for MySQL yet; use verify-full or require".into(),
            ));
        }
        // PostgreSQL treats `require` with a CA file as verify-ca, which
        // MySQL cannot do (above): refuse rather than ignore the CA file.
        if spec.tls == TlsMode::Require && spec.ca_file.is_some() {
            return Err(Error::Tls(
                "MySQL checks a CA file only with verify-full; choose verify-full, or remove \
                 the CA file to encrypt without checking the certificate"
                    .into(),
            ));
        }
        let builder = builder(spec, secrets, via);
        let with_tls: Opts = with_tls(builder.clone(), spec, via).into();
        let (mut conn, opts) =
            match tokio::time::timeout(CONNECT_TIMEOUT, mysql_async::Conn::new(with_tls.clone()))
                .await
            {
                Err(_) => return Err(Error::Timeout),
                Ok(Ok(conn)) => (conn, with_tls),
                // `prefer`: a server without TLS gets a plain connection.
                Ok(Err(mysql_async::Error::Driver(DriverError::NoClientSslFlagFromServer)))
                    if spec.tls == TlsMode::Prefer =>
                {
                    let plain: Opts = builder.ssl_opts(None::<mysql_async::SslOpts>).into();
                    (
                        mysql_async::Conn::new(plain.clone())
                            .await
                            .map_err(connect_error)?,
                        plain,
                    )
                }
                Ok(Err(error)) => return Err(connect_error(error)),
            };
        prepare_session(&mut conn, access).await?;
        let id = conn.id();
        // mysql_async fails rather than go on in plain text when it was
        // given TLS options, so the options that connected say it.
        let encrypted = opts.ssl_opts().is_some();
        let server = server_identity(&mut conn).await.map_err(query_error)?;
        let version: Option<mysql_async::Row> = conn
            .query_first("SELECT VERSION()")
            .await
            .map_err(query_error)?;
        let version = version.map(from_row).transpose()?.unwrap_or_default();
        Ok(Self {
            conn: tokio::sync::Mutex::new(conn),
            opts,
            id,
            encrypted,
            server,
            version,
            access,
        })
    }

    /// A catalog query: fixed SQL, prepared, typed results.
    async fn catalog<T>(&self, sql: &str, params: impl Into<Params> + Send) -> Result<Vec<T>>
    where
        T: mysql_async::prelude::FromRow + Send + 'static,
    {
        let mut conn = self.conn.lock().await;
        let rows: Vec<mysql_async::Row> = conn.exec(sql, params).await.map_err(query_error)?;
        rows.into_iter().map(from_row).collect()
    }

    pub async fn primary_key(&self, object: &ObjectRef) -> Result<Vec<String>> {
        self.catalog(
            "SELECT column_name FROM information_schema.key_column_usage \
             WHERE table_schema = ? AND table_name = ? AND constraint_name = 'PRIMARY' \
             ORDER BY ordinal_position",
            (&object.schema, &object.name),
        )
        .await
    }

    /// The byte string columns a filter of `query` compares as bytes when
    /// its value reads as bytes; empty, without asking, when no filter has
    /// such a value.
    async fn binary_columns(&self, query: &RowQuery) -> Result<Vec<String>> {
        if !crate::dialect::reads_bytes(query) {
            return Ok(Vec::new());
        }
        self.catalog(
            "SELECT column_name FROM information_schema.columns \
             WHERE table_schema = ? AND table_name = ? AND data_type IN \
                   ('binary', 'varbinary', 'tinyblob', 'blob', 'mediumblob', 'longblob')",
            (&query.object.schema, &query.object.name),
        )
        .await
    }
}

impl Adapter for Conn {
    fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    fn cancel_handle(&self) -> CancelHandle {
        CancelHandle(CancelInner::MySql {
            opts: self.opts.clone(),
            id: self.id,
            server: self.server.clone(),
        })
    }

    /// See [`crate::Connection::server_version`]. Asked at connect.
    async fn server_version(&self) -> Result<String> {
        Ok(version_name(&self.version))
    }

    /// None: MySQL's databases are listed as its schemas.
    async fn list_databases(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn list_schemas(&self) -> Result<Vec<String>> {
        self.catalog(
            &format!(
                "SELECT schema_name FROM information_schema.schemata \
                 ORDER BY schema_name LIMIT {MAX_LISTED}"
            ),
            Params::Empty,
        )
        .await
    }

    async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        let rows: Vec<(String, String, Option<u64>)> = self
            .catalog(
                &format!(
                    "SELECT table_name, table_type, table_rows FROM information_schema.tables \
                     WHERE table_schema = ? ORDER BY table_name LIMIT {MAX_LISTED}"
                ),
                (schema,),
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|(name, kind, estimate)| {
                let view = kind.contains("VIEW");
                ObjectInfo {
                    name,
                    kind: if view {
                        ObjectKind::View
                    } else {
                        ObjectKind::Table
                    },
                    estimated_rows: if view { None } else { estimate },
                }
            })
            .collect())
    }

    async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        let at = (&object.schema, &object.name);
        let columns: Vec<(String, String, String, Option<String>, String, String)> = self
            .catalog(
                "SELECT column_name, column_type, is_nullable, column_default, column_comment, \
                        COALESCE(extra, '') \
                 FROM information_schema.columns \
                 WHERE table_schema = ? AND table_name = ? ORDER BY ordinal_position",
                at,
            )
            .await?;
        if columns.is_empty() {
            return Err(Error::query(format!(
                "no such table or view: {}",
                object.name
            )));
        }
        let columns = columns
            .into_iter()
            .map(
                |(name, type_name, nullable, default, comment, extra)| ColumnInfo {
                    name,
                    type_name,
                    nullable: nullable == "YES",
                    default,
                    comment: (!comment.is_empty()).then_some(comment),
                    allowed_values: None,
                    // `VIRTUAL GENERATED`, `STORED GENERATED`, and on an older
                    // MariaDB `VIRTUAL` or `PERSISTENT`. Not `DEFAULT_GENERATED`,
                    // which MySQL 8 says of a default that is an expression.
                    generated: ["VIRTUAL", "STORED", "PERSISTENT"]
                        .iter()
                        .any(|word| extra.to_ascii_uppercase().contains(word)),
                },
            )
            .collect();
        let index_rows: Vec<(String, i64, String, String, i64)> = self
            .catalog(
                // The last column: whether the entry is one whole column. An
                // expression has no column name, and an index over the first
                // characters of a column (`sub_part`) names the column all
                // the same.
                "SELECT index_name, non_unique, COALESCE(column_name, '<expression>'), index_type, \
                        column_name IS NOT NULL AND sub_part IS NULL \
                 FROM information_schema.statistics \
                 WHERE table_schema = ? AND table_name = ? ORDER BY index_name, seq_in_index",
                at,
            )
            .await?;
        let mut indexes: Vec<IndexInfo> = Vec::new();
        for (name, non_unique, column, method, whole) in index_rows {
            if indexes.last().is_none_or(|index| index.name != name) {
                indexes.push(IndexInfo {
                    primary: name == "PRIMARY",
                    unique: non_unique == 0,
                    method: Some(method.to_lowercase()),
                    columns: Vec::new(),
                    key_columns: Some(Vec::new()),
                    partial: false,
                    name,
                });
            }
            if let Some(index) = indexes.last_mut() {
                if whole == 0 {
                    index.key_columns = None;
                } else if let Some(key) = &mut index.key_columns {
                    key.push(column.clone());
                }
                index.columns.push(column);
            }
        }
        let key_rows: Vec<(String, String, String, String, String, String, String)> = self
            .catalog(
                "SELECT k.constraint_name, k.column_name, k.referenced_table_schema, \
                        k.referenced_table_name, k.referenced_column_name, r.update_rule, r.delete_rule \
                 FROM information_schema.key_column_usage k \
                 JOIN information_schema.referential_constraints r \
                   ON r.constraint_schema = k.constraint_schema AND r.constraint_name = k.constraint_name \
                  AND r.table_name = k.table_name \
                 WHERE k.table_schema = ? AND k.table_name = ? AND k.referenced_table_name IS NOT NULL \
                 ORDER BY k.constraint_name, k.ordinal_position",
                at,
            )
            .await?;
        let mut foreign_keys: Vec<ForeignKeyInfo> = Vec::new();
        for (name, column, ref_schema, ref_table, ref_column, on_update, on_delete) in key_rows {
            if foreign_keys
                .last()
                .is_none_or(|key| key.name.as_deref() != Some(name.as_str()))
            {
                foreign_keys.push(ForeignKeyInfo {
                    name: Some(name),
                    columns: Vec::new(),
                    ref_schema,
                    ref_table,
                    ref_columns: Vec::new(),
                    on_update,
                    on_delete,
                });
            }
            if let Some(key) = foreign_keys.last_mut() {
                key.columns.push(column);
                key.ref_columns.push(ref_column);
            }
        }
        Ok(Structure {
            columns,
            primary_key: self.primary_key(object).await?,
            indexes,
            foreign_keys,
        })
    }

    /// One page, through a prepared statement inside a read-only
    /// transaction. Reading stops after `limit + 1` rows.
    async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        let key = self.primary_key(&query.object).await?;
        let binary = self.binary_columns(query).await?;
        let sql = Dialect::MySql.select_rows(query, &key, &binary);
        let limit = query.limit as usize;
        let mut conn = self.conn.lock().await;
        let started = Instant::now();
        let mut transaction = conn
            .start_transaction(read_only())
            .await
            .map_err(query_error)?;
        let outcome = read_page(&mut transaction, &sql, limit).await;
        let (columns, mut rows) = finish(transaction, outcome).await?;
        let has_more = rows.len() > limit;
        rows.truncate(limit);
        Ok(RowPage {
            columns,
            rows,
            has_more,
            ordered_by_key: !key.is_empty(),
            elapsed: started.elapsed(),
        })
    }

    async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        let binary = self.binary_columns(query).await?;
        let sql = Dialect::MySql.count_rows(query, &binary);
        let mut conn = self.conn.lock().await;
        let mut transaction = conn
            .start_transaction(read_only())
            .await
            .map_err(query_error)?;
        let outcome = count(&mut transaction, &sql).await;
        let row = finish(transaction, outcome).await?;
        row.map(from_row::<u64>)
            .transpose()?
            .ok_or_else(|| Error::query("the count returned no number"))
    }

    async fn run_script(
        &self,
        texts: Vec<String>,
        limit: u32,
        mode: ScriptMode,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        self.script(&texts, limit, mode, stop).await
    }

    async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
        self.save(changes, stop).await
    }
}

/// A row as `T`. A server (or proxy) that answers with other types or a
/// NULL is a query error, not a panic: release builds abort on panic. The
/// row itself stays out of the message.
fn from_row<T: mysql_async::prelude::FromRow>(row: mysql_async::Row) -> Result<T> {
    mysql_async::from_row_opt(row).map_err(|_| {
        Error::query("unexpected data from the server: a row did not have the expected types")
    })
}

/// A result set's columns.
fn column_metas(columns: &[mysql_async::Column]) -> Vec<ColumnMeta> {
    columns
        .iter()
        .map(|column| {
            let type_name = type_name(column.column_type(), column.flags(), column.character_set());
            ColumnMeta {
                name: column.name_str().into_owned(),
                kind: kind(&type_name),
                type_name: type_name.into_owned(),
            }
        })
        .collect()
}

/// A row's values, typed by `columns`.
fn row_values(row: mysql_async::Row, columns: &[ColumnMeta]) -> Vec<Value> {
    row.unwrap()
        .into_iter()
        .zip(columns)
        .map(|(cell, column)| value(cell, &column.type_name))
        .collect()
}

/// Up to `limit + 1` rows of a page, so the caller can tell there are more.
async fn read_page(
    transaction: &mut mysql_async::Transaction<'_>,
    sql: &crate::dialect::Sql,
    limit: usize,
) -> Result<(Vec<ColumnMeta>, Vec<Vec<Value>>)> {
    let statement = prepare(transaction, sql).await?;
    let mut result = transaction
        .exec_iter(&statement, params(&sql.params))
        .await
        .map_err(query_error)?;
    let columns = column_metas(result.columns_ref());
    let mut rows = Vec::new();
    while let Some(row) = result.next().await.map_err(query_error)? {
        rows.push(row_values(row, &columns));
        if rows.len() > limit {
            break;
        }
    }
    result.drop_result().await.map_err(query_error)?;
    Ok((columns, rows))
}

/// The row of a count.
async fn count(
    transaction: &mut mysql_async::Transaction<'_>,
    sql: &crate::dialect::Sql,
) -> Result<Option<mysql_async::Row>> {
    let statement = prepare(transaction, sql).await?;
    transaction
        .exec_first(&statement, params(&sql.params))
        .await
        .map_err(query_error)
}

/// Prepares a row query's statement, and refuses one with a parameter the
/// query has no value for: the driver closes the connection when a
/// statement runs without a value for each of its parameters. The filters
/// bind theirs, so such a parameter comes from the raw WHERE.
async fn prepare(
    transaction: &mut mysql_async::Transaction<'_>,
    sql: &crate::dialect::Sql,
) -> Result<mysql_async::Statement> {
    driver_parameter(&sql.text)?;
    let statement = transaction
        .prep(sql.text.as_str())
        .await
        .map_err(query_error)?;
    if usize::from(statement.num_params()) != sql.params.len() {
        return Err(parameter("?", None));
    }
    Ok(statement)
}

/// The driver's `mysql_common` is the one named here: this fails to
/// compile, rather than read parameters another way than the driver does,
/// when the two versions drift apart.
const _: fn(mysql_common::params::Params) -> Params = |params| params;

/// Refuses text in which the driver reads a `:name` parameter. It sends
/// the server a `?` in its place, and it reads by a lexer of its own, which
/// loses its place at a quote or a comment right after a `-` or a `/` and
/// then takes what is quoted for code (`-':name'`). Such text does not
/// run: only the text checked here may reach the server.
fn driver_parameter(text: &str) -> Result<()> {
    let Ok(parsed) = ParsedNamedParams::parse(text.as_bytes()) else {
        // A `:name` next to a `?`, which may be a filter's own.
        return Err(parameter(":name", Some(MISREAD)));
    };
    match parsed.params().first() {
        None => Ok(()),
        Some(name) => Err(parameter(
            &format!(":{}", String::from_utf8_lossy(name)),
            Some(MISREAD),
        )),
    }
}

/// What to do about a `:name` the driver should not have read as one.
const MISREAD: &str = "If it stands in a quote or a comment right after a - or a /, put a space \
                       after the - or the /.";

/// A raw WHERE's parameter, as a query error: there is no value for it.
fn parameter(spelled: &str, hint: Option<&str>) -> Error {
    Error::Query {
        code: None,
        message: format!("The WHERE text has a parameter ({spelled}), which has no value."),
        detail: None,
        hint: hint.map(str::to_owned),
    }
}

/// Ends a read transaction whatever happened inside it. mysql_async only
/// rolls a dropped transaction back on the connection's next use, so a
/// failed or cancelled read would otherwise keep its locks until then.
/// The read's own error wins over one from the rollback.
async fn finish<T>(transaction: mysql_async::Transaction<'_>, outcome: Result<T>) -> Result<T> {
    let rolled_back = transaction.rollback().await;
    let value = outcome?;
    rolled_back.map_err(query_error)?;
    Ok(value)
}

/// Makes the session read-only: no transaction of its own can write.
const READ_ONLY: &str = "SET SESSION TRANSACTION READ ONLY";

/// Makes a writable session read-write: at connect, also on a server whose
/// default is read-only, and again after a script, whose fence made it
/// read-only and whose reset put the server's default back.
const READ_WRITE: &str = "SET SESSION TRANSACTION READ WRITE";

/// The character set and collation of the driver's handshake. A session
/// reset puts the server's defaults in their place, while the driver goes
/// on sending and reading UTF-8.
const NAMES: &str = "SET NAMES utf8mb4 COLLATE utf8mb4_general_ci";

/// Has the server say in a note what it rounds: a decimal with more places
/// than its column keeps is stored rounded in any mode, and only a note
/// tells. A save takes a note for the warning it is and fails. A server can
/// have notes off as its default, and a session reset puts that default
/// back.
const NOTES: &str = "SET SESSION sql_notes = 1";

/// The sql_mode names a session runs without, so that the server lexes
/// text as `sql::tokenize` does: with `ANSI_QUOTES` a `"..."` is a name,
/// and with `NO_BACKSLASH_ESCAPES` a backslash is a character. The others
/// are combination modes, which turn `ANSI_QUOTES` back on if they stay.
/// In this order: `ANSI` is also the start of `ANSI_QUOTES`.
const LEXING_MODES: [&str; 8] = [
    "NO_BACKSLASH_ESCAPES",
    "ANSI_QUOTES",
    "ANSI",
    "POSTGRESQL",
    "ORACLE",
    "MSSQL",
    "DB2",
    "MAXDB",
];

/// The session settings every connection runs with, set at connect and
/// again after a SQL editor script's reset, which undoes them. A read-only
/// session says so first, so nothing here runs in a session that could
/// write; a writable one says read-write last. After a reset a cancel
/// meant for a statement can land here, so a statement it interrupts runs
/// once more: the session is not left in the wrong mode because a `SET`
/// was interrupted.
async fn prepare_session(conn: &mut mysql_async::Conn, access: Access) -> Result<()> {
    let sql_mode = LEXING_MODES
        .iter()
        .fold("@@SESSION.sql_mode".to_owned(), |mode, name| {
            format!("REPLACE({mode}, '{name}', '')")
        });
    let sql_mode = format!("SET SESSION sql_mode = {sql_mode}");
    let statements = match access {
        Access::ReadOnly => [READ_ONLY, NAMES, sql_mode.as_str(), NOTES],
        Access::Writable => [NAMES, sql_mode.as_str(), NOTES, READ_WRITE],
    };
    // Fixed statements: safe to send through the text protocol.
    for statement in statements {
        retry_cancelled!(execute(conn, statement))?;
    }
    Ok(())
}

/// Runs one of the driver's own statements, through the text protocol.
async fn execute(conn: &mut mysql_async::Conn, statement: &str) -> Result<()> {
    conn.query_drop(statement).await.map_err(query_error)
}

/// The server's status after the last query that worked: whether the
/// session is in a transaction, and whether that one is read-only.
fn status(conn: &mysql_async::Conn) -> StatusFlags {
    conn.last_ok_packet()
        .map_or(StatusFlags::empty(), |ok| ok.status_flags())
}

/// `SELECT VERSION()` as the footer shows it: the name, and the number
/// without what a distribution adds after a `-`.
fn version_name(full: &str) -> String {
    let name = if full.contains("MariaDB") {
        "MariaDB"
    } else {
        "MySQL"
    };
    let number = full.split('-').next().unwrap_or_default();
    format!("{name} {number}")
}

/// A readable type name from a result column's metadata. Result columns do
/// not carry lengths or enum members; the Structure view shows the full
/// `column_type` from information_schema instead.
pub(crate) fn type_name(
    column_type: ColumnType,
    flags: ColumnFlags,
    charset: u16,
) -> Cow<'static, str> {
    let binary = charset == BINARY_CHARSET;
    let name: &'static str = match column_type {
        ColumnType::MYSQL_TYPE_TINY => "tinyint",
        ColumnType::MYSQL_TYPE_SHORT => "smallint",
        ColumnType::MYSQL_TYPE_INT24 => "mediumint",
        ColumnType::MYSQL_TYPE_LONG => "int",
        ColumnType::MYSQL_TYPE_LONGLONG => "bigint",
        ColumnType::MYSQL_TYPE_FLOAT => "float",
        ColumnType::MYSQL_TYPE_DOUBLE => "double",
        ColumnType::MYSQL_TYPE_DECIMAL | ColumnType::MYSQL_TYPE_NEWDECIMAL => "decimal",
        ColumnType::MYSQL_TYPE_YEAR => "year",
        ColumnType::MYSQL_TYPE_DATE | ColumnType::MYSQL_TYPE_NEWDATE => "date",
        ColumnType::MYSQL_TYPE_DATETIME | ColumnType::MYSQL_TYPE_DATETIME2 => "datetime",
        ColumnType::MYSQL_TYPE_TIMESTAMP | ColumnType::MYSQL_TYPE_TIMESTAMP2 => "timestamp",
        ColumnType::MYSQL_TYPE_TIME | ColumnType::MYSQL_TYPE_TIME2 => "time",
        ColumnType::MYSQL_TYPE_JSON => "json",
        ColumnType::MYSQL_TYPE_BIT => "bit",
        ColumnType::MYSQL_TYPE_GEOMETRY => "geometry",
        ColumnType::MYSQL_TYPE_ENUM => "enum",
        ColumnType::MYSQL_TYPE_SET => "set",
        ColumnType::MYSQL_TYPE_STRING if flags.contains(ColumnFlags::ENUM_FLAG) => "enum",
        ColumnType::MYSQL_TYPE_STRING if flags.contains(ColumnFlags::SET_FLAG) => "set",
        ColumnType::MYSQL_TYPE_STRING => {
            if binary {
                "binary"
            } else {
                "char"
            }
        }
        ColumnType::MYSQL_TYPE_VARCHAR | ColumnType::MYSQL_TYPE_VAR_STRING => {
            if binary {
                "varbinary"
            } else {
                "varchar"
            }
        }
        ColumnType::MYSQL_TYPE_TINY_BLOB
        | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
        | ColumnType::MYSQL_TYPE_LONG_BLOB
        | ColumnType::MYSQL_TYPE_BLOB => {
            if binary {
                "blob"
            } else {
                "text"
            }
        }
        _ => "unknown",
    };
    let numeric = matches!(
        name,
        "tinyint" | "smallint" | "mediumint" | "int" | "bigint" | "float" | "double" | "decimal"
    );
    if numeric && flags.contains(ColumnFlags::UNSIGNED_FLAG) {
        Cow::Owned(format!("{name} unsigned"))
    } else {
        Cow::Borrowed(name)
    }
}

/// The kind for a name from `type_name`.
pub(crate) fn kind(type_name: &str) -> ValueKind {
    match type_name.split(' ').next().unwrap_or_default() {
        "tinyint" | "smallint" | "mediumint" | "int" | "bigint" | "float" | "double"
        | "decimal" | "year" => ValueKind::Numeric,
        "date" | "datetime" | "timestamp" | "time" => ValueKind::Temporal,
        "json" => ValueKind::Json,
        "binary" | "varbinary" | "blob" | "bit" | "geometry" => ValueKind::Binary,
        "char" | "varchar" | "text" | "enum" | "set" => ValueKind::Text,
        _ => ValueKind::Other,
    }
}

fn fraction(micros: u32) -> String {
    if micros == 0 {
        String::new()
    } else {
        format!(".{micros:06}")
    }
}

/// A binary-protocol value as the grid shows it. Unsigned integers too big
/// for `i64` and decimals stay text so nothing is rounded.
pub(crate) fn value(value: mysql_async::Value, type_name: &str) -> Value {
    use mysql_async::Value as My;
    match value {
        My::NULL => Value::Null,
        My::Int(number) => Value::Int(number),
        My::UInt(number) => i64::try_from(number)
            .map(Value::Int)
            .unwrap_or_else(|_| Value::Text(number.to_string().into())),
        // Through its shortest text, so 0.1 stays 0.1 rather than 0.10000000149.
        My::Float(number) => Value::Float(number.to_string().parse().unwrap_or(f64::from(number))),
        My::Double(number) => Value::Float(number),
        My::Date(year, month, day, hour, minute, second, micros) => {
            let date = format!("{year:04}-{month:02}-{day:02}");
            if type_name == "date" {
                Value::Text(date.into())
            } else {
                Value::Text(
                    format!(
                        "{date} {hour:02}:{minute:02}:{second:02}{}",
                        fraction(micros)
                    )
                    .into(),
                )
            }
        }
        My::Time(negative, days, hours, minutes, seconds, micros) => {
            let hours = days * 24 + u32::from(hours);
            let sign = if negative { "-" } else { "" };
            Value::Text(
                format!(
                    "{sign}{hours:02}:{minutes:02}:{seconds:02}{}",
                    fraction(micros)
                )
                .into(),
            )
        }
        My::Bytes(bytes) => match kind(type_name) {
            ValueKind::Binary => Value::Bytes(bytes.into()),
            _ => Value::Text(String::from_utf8_lossy(&bytes).into()),
        },
    }
}

/// A filter value as a statement parameter.
pub(crate) fn to_mysql(value: &Value) -> mysql_async::Value {
    use mysql_async::Value as My;
    match value {
        Value::Null => My::NULL,
        Value::Bool(flag) => My::Int(i64::from(*flag)),
        Value::Int(number) => My::Int(*number),
        Value::Float(number) => My::Double(*number),
        Value::Text(text) => My::Bytes(text.as_bytes().to_vec()),
        Value::Bytes(bytes) => My::Bytes(bytes.to_vec()),
    }
}

/// Connection options without TLS; through a tunnel they point at its
/// local port.
pub(crate) fn builder(spec: &ConnectSpec, secrets: &Secrets, via: Option<u16>) -> OptsBuilder {
    let (host, port) = match via {
        Some(port) => ("127.0.0.1".to_owned(), port),
        None => (spec.host.clone(), spec.port),
    };
    OptsBuilder::default()
        .ip_or_hostname(host)
        .tcp_port(port)
        .user(Some(spec.user.clone()))
        .pass(secrets.password.clone())
        .db_name((!spec.database.is_empty()).then(|| spec.database.clone()))
        // Never switch a "localhost" connection to the Unix socket.
        .prefer_socket(false)
}

/// The options with TLS for the spec's mode; through a tunnel the
/// certificate is still checked against `spec.host`.
pub(crate) fn with_tls(builder: OptsBuilder, spec: &ConnectSpec, via: Option<u16>) -> OptsBuilder {
    let ssl = ssl_opts(spec.tls, spec.ca_file.as_deref()).map(|ssl| match via {
        Some(_) => ssl.with_danger_tls_hostname_override(Some(spec.host.clone())),
        None => ssl,
    });
    builder.ssl_opts(ssl)
}

/// `mysql_async` TLS options for our mode (`None` means no TLS).
pub(crate) fn ssl_opts(mode: TlsMode, ca_file: Option<&Path>) -> Option<SslOpts> {
    // A CA file is the only root trusted, as for PostgreSQL; without one,
    // mysql_async's bundled roots are used.
    let base = match ca_file {
        Some(path) => SslOpts::default()
            .with_root_certs(vec![path.to_path_buf().into()])
            .with_disable_built_in_roots(true),
        None => SslOpts::default(),
    };
    match mode {
        TlsMode::Disable => None,
        TlsMode::Prefer | TlsMode::Require => Some(base.with_danger_accept_invalid_certs(true)),
        TlsMode::VerifyCa => Some(base.with_danger_skip_domain_validation(true)),
        TlsMode::VerifyFull => Some(base),
    }
}

pub(crate) fn connect_error(error: mysql_async::Error) -> Error {
    if crate::tls::is_tls_error(&error) {
        return Error::Tls(error.to_string());
    }
    match error {
        mysql_async::Error::Server(server) if server.code == ACCESS_DENIED => {
            Error::Auth(server.message)
        }
        // The server was reached and said no: its own code and words.
        mysql_async::Error::Server(server) => Error::Query {
            code: Some(server.code.to_string()),
            message: server.message,
            detail: None,
            hint: None,
        },
        mysql_async::Error::Io(IoError::Tls(tls)) => Error::Tls(tls.to_string()),
        mysql_async::Error::Driver(DriverError::NoClientSslFlagFromServer) => {
            Error::Tls("the server does not support TLS".into())
        }
        other => Error::Connect(other.to_string()),
    }
}

pub(crate) fn query_error(error: mysql_async::Error) -> Error {
    match error {
        mysql_async::Error::Server(server) if server.code == QUERY_INTERRUPTED => Error::Cancelled,
        mysql_async::Error::Server(server) => Error::Query {
            code: Some(server.state),
            message: server.message,
            detail: None,
            hint: None,
        },
        mysql_async::Error::Io(_) | mysql_async::Error::Driver(DriverError::ConnectionClosed) => {
            Error::ConnectionLost(error.to_string())
        }
        other => Error::query(other.to_string()),
    }
}

/// Stops the query thread `id` runs on the server named `server`. The cancel
/// connection resolves the host again, and behind round robin DNS or a load
/// balancer it can reach another server, where `id` is someone else's
/// session: so it kills only after checking it reached the same server.
pub(crate) async fn cancel(opts: &Opts, id: u32, server: &str) -> Result<()> {
    let failed = |error: mysql_async::Error| Error::Connect(format!("could not cancel: {error}"));
    let mut conn =
        match tokio::time::timeout(CONNECT_TIMEOUT, mysql_async::Conn::new(opts.clone())).await {
            Err(_) => return Err(Error::Timeout),
            Ok(result) => result.map_err(failed)?,
        };
    let result = async {
        let reached = server_identity(&mut conn).await.map_err(failed)?;
        same_server(server, &reached)?;
        conn.query_drop(format!("KILL QUERY {id}"))
            .await
            .map_err(failed)
    }
    .await;
    let _ = conn.disconnect().await;
    result
}

/// Names the server a connection reached: `@@server_uuid`, or host name and
/// port on MariaDB, which has no server UUID.
async fn server_identity(conn: &mut mysql_async::Conn) -> mysql_async::Result<String> {
    match conn.query_first::<String, _>("SELECT @@server_uuid").await {
        Ok(Some(uuid)) => return Ok(format!("uuid {uuid}")),
        Ok(None) => {}
        Err(mysql_async::Error::Server(error)) if error.code == UNKNOWN_SYSTEM_VARIABLE => {}
        Err(error) => return Err(error),
    }
    let found: Option<(String, u16)> = conn.query_first("SELECT @@hostname, @@port").await?;
    Ok(found
        .map(|(host, port)| format!("host {host}:{port}"))
        .unwrap_or_default())
}

/// Whether a cancel connection reached the session's server. An unknown
/// server never matches.
fn same_server(session: &str, reached: &str) -> Result<()> {
    if !session.is_empty() && session == reached {
        Ok(())
    } else {
        Err(Error::Connect(
            "could not cancel: reached a different server".into(),
        ))
    }
}

fn read_only() -> TxOpts {
    let mut options = TxOpts::default();
    options.with_readonly(Some(true));
    options
}

fn params(values: &[Value]) -> Params {
    if values.is_empty() {
        Params::Empty
    } else {
        Params::Positional(values.iter().map(to_mysql).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mysql_async::Value as My;

    #[test]
    fn text_the_driver_would_rewrite_is_refused() {
        for (text, spelled) in [
            ("SELECT 1 WHERE (\nname = :x\n)", ":x"),
            ("SELECT 1 WHERE (\nid = 1 -':abc'\n)", ":abc"),
            ("SELECT 1 WHERE (\nid = 1 /`:col`\n)", ":col"),
            ("SELECT 1 WHERE `a` = ? AND (\nname = :x\n)", ":name"),
        ] {
            match driver_parameter(text) {
                Err(Error::Query {
                    code: None,
                    message,
                    hint: Some(_),
                    ..
                }) => assert!(message.contains(&format!("({spelled})")), "{message}"),
                other => panic!("{text}: {other:?}"),
            }
        }
        // A `?` is the server's to count, and a `:name` in a quote or a
        // comment is text.
        for text in [
            "SELECT 1 WHERE `a` = ? AND (\nid = ?\n)",
            "SELECT 1 WHERE (\nname <> ':x' AND `:y` = 1 /* :z */ -- :w\n)",
            "SELECT 1 WHERE (\n@n := 1\n)",
        ] {
            assert_eq!(driver_parameter(text), Ok(()), "{text}");
        }
    }

    #[test]
    fn column_types_get_readable_names() {
        let none = ColumnFlags::empty();
        let cases = [
            (ColumnType::MYSQL_TYPE_LONG, none, 63, "int"),
            (
                ColumnType::MYSQL_TYPE_LONGLONG,
                ColumnFlags::UNSIGNED_FLAG,
                63,
                "bigint unsigned",
            ),
            (ColumnType::MYSQL_TYPE_NEWDECIMAL, none, 63, "decimal"),
            (ColumnType::MYSQL_TYPE_VAR_STRING, none, 255, "varchar"),
            (ColumnType::MYSQL_TYPE_VAR_STRING, none, 63, "varbinary"),
            (ColumnType::MYSQL_TYPE_BLOB, none, 255, "text"),
            (ColumnType::MYSQL_TYPE_BLOB, none, 63, "blob"),
            (
                ColumnType::MYSQL_TYPE_STRING,
                ColumnFlags::ENUM_FLAG,
                255,
                "enum",
            ),
            (ColumnType::MYSQL_TYPE_DATETIME, none, 63, "datetime"),
            (ColumnType::MYSQL_TYPE_JSON, none, 63, "json"),
        ];
        for (column_type, flags, charset, name) in cases {
            assert_eq!(
                type_name(column_type, flags, charset),
                name,
                "{column_type:?}"
            );
        }
    }

    #[test]
    fn names_map_to_kinds() {
        assert_eq!(kind("bigint unsigned"), ValueKind::Numeric);
        assert_eq!(kind("decimal"), ValueKind::Numeric);
        assert_eq!(kind("datetime"), ValueKind::Temporal);
        assert_eq!(kind("json"), ValueKind::Json);
        assert_eq!(kind("varbinary"), ValueKind::Binary);
        assert_eq!(kind("blob"), ValueKind::Binary);
        assert_eq!(kind("text"), ValueKind::Text);
        assert_eq!(kind("enum"), ValueKind::Text);
        assert_eq!(kind("geometry"), ValueKind::Binary);
    }

    #[test]
    fn mysql_values_become_display_values() {
        assert_eq!(value(My::NULL, "int"), Value::Null);
        assert_eq!(value(My::Int(-7), "int"), Value::Int(-7));
        assert_eq!(
            value(My::UInt(u64::MAX), "bigint unsigned"),
            Value::Text("18446744073709551615".into())
        );
        assert_eq!(value(My::UInt(5), "bigint unsigned"), Value::Int(5));
        assert_eq!(value(My::Float(1.5), "float"), Value::Float(1.5));
        assert_eq!(value(My::Double(99.5), "double"), Value::Float(99.5));
        assert_eq!(
            value(My::Bytes(b"123456789012.34".to_vec()), "decimal"),
            Value::Text("123456789012.34".into())
        );
        assert_eq!(
            value(My::Date(1815, 12, 10, 0, 0, 0, 0), "date"),
            Value::Text("1815-12-10".into())
        );
        assert_eq!(
            value(My::Date(2026, 1, 2, 9, 0, 0, 0), "datetime"),
            Value::Text("2026-01-02 09:00:00".into())
        );
        assert_eq!(
            value(My::Date(2026, 1, 2, 9, 0, 0, 120), "datetime"),
            Value::Text("2026-01-02 09:00:00.000120".into())
        );
        assert_eq!(
            value(My::Time(false, 0, 7, 30, 0, 0), "time"),
            Value::Text("07:30:00".into())
        );
        assert_eq!(
            value(My::Time(true, 1, 1, 15, 0, 0), "time"),
            Value::Text("-25:15:00".into())
        );
        assert_eq!(
            value(My::Bytes(vec![0x89, 0x50]), "varbinary"),
            Value::Bytes(vec![0x89, 0x50].into())
        );
        assert_eq!(
            value(My::Bytes("Zoë".as_bytes().to_vec()), "varchar"),
            Value::Text("Zoë".into())
        );
        assert_eq!(
            value(My::Bytes(b"{\"a\": 1}".to_vec()), "json"),
            Value::Text("{\"a\": 1}".into())
        );
    }

    #[test]
    fn filter_values_bind_as_mysql_values() {
        assert_eq!(
            to_mysql(&Value::Text("O'Brien".into())),
            My::Bytes(b"O'Brien".to_vec())
        );
        assert_eq!(to_mysql(&Value::Null), My::NULL);
        assert_eq!(to_mysql(&Value::Int(3)), My::Int(3));
    }

    #[test]
    fn tls_modes_become_ssl_options() {
        assert!(ssl_opts(TlsMode::Disable, None).is_none());
        let require = ssl_opts(TlsMode::Require, None).unwrap();
        assert!(require.accept_invalid_certs());
        let prefer = ssl_opts(TlsMode::Prefer, None).unwrap();
        assert!(prefer.accept_invalid_certs());
        let ca = ssl_opts(TlsMode::VerifyCa, Some(std::path::Path::new("/ca.pem"))).unwrap();
        assert!(!ca.accept_invalid_certs() && ca.skip_domain_validation());
        assert_eq!(ca.root_certs().len(), 1);
        let full = ssl_opts(TlsMode::VerifyFull, None).unwrap();
        assert!(!full.accept_invalid_certs() && !full.skip_domain_validation());
        assert!(!full.disable_built_in_roots());
    }

    #[test]
    fn a_ca_file_is_the_only_trusted_root() {
        let full = ssl_opts(TlsMode::VerifyFull, Some(std::path::Path::new("/ca.pem"))).unwrap();
        assert_eq!(full.root_certs().len(), 1);
        assert!(full.disable_built_in_roots());
    }

    #[tokio::test]
    async fn require_with_a_ca_file_is_refused_not_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let ca = dir.path().join("ca.pem");
        std::fs::write(&ca, "").unwrap();
        let (mut spec, secrets) =
            ConnectSpec::from_url("mysql://me@127.0.0.1:1/app?ssl-mode=REQUIRED").unwrap();
        spec.ca_file = Some(ca);
        match Conn::connect(&spec, &secrets, None, Access::ReadOnly).await {
            Err(Error::Tls(message)) => assert!(message.contains("verify-full"), "{message}"),
            other => panic!("{:?}", other.map(|_| ())),
        }
    }

    fn server(code: u16, state: &str) -> mysql_async::Error {
        mysql_async::Error::Server(mysql_async::ServerError {
            code,
            message: "message".into(),
            state: state.into(),
        })
    }

    #[test]
    fn errors_map_to_our_kinds() {
        assert!(matches!(
            connect_error(server(1045, "28000")),
            Error::Auth(_)
        ));
        // The server answered: its own code and words.
        match connect_error(server(1049, "42000")) {
            Error::Query { code, message, .. } => {
                assert_eq!(code.as_deref(), Some("1049"));
                assert_eq!(message, "message");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(query_error(server(1317, "70100")), Error::Cancelled);
        match query_error(server(1054, "42S22")) {
            Error::Query { code, .. } => assert_eq!(code.as_deref(), Some("42S22")),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            query_error(mysql_async::Error::Driver(
                mysql_async::DriverError::ConnectionClosed
            )),
            Error::ConnectionLost(_)
        ));
    }

    #[test]
    fn float_columns_keep_their_short_form() {
        assert_eq!(value(My::Float(0.1), "float"), Value::Float(0.1));
        assert_eq!(
            value(My::Float(f32::MAX), "float"),
            Value::Float(3.402_823_5e38)
        );
    }

    #[test]
    fn a_cancel_kills_only_on_the_sessions_server() {
        let uuid = "uuid 3e11fa47-71ca-11e1-9e33-c80aa9429562";
        assert_eq!(same_server(uuid, uuid), Ok(()));
        let different = Err(Error::Connect(
            "could not cancel: reached a different server".into(),
        ));
        assert_eq!(
            same_server(uuid, "uuid 8a94f357-aab4-11df-86ab-c80aa9429562"),
            different
        );
        assert_eq!(same_server("host db1:3306", "host db2:3306"), different);
        assert_eq!(same_server("host db1:3306", "host db1:3307"), different);
        assert_eq!(same_server("", ""), different);
    }

    /// The test server's URL, or `None` (test skipped). See AGENTS.md and
    /// `tests/mysql.rs`.
    pub(super) fn test_url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_MYSQL_URL")
            .ok()
            .filter(|url| !url.trim().is_empty());
        if url.is_none() {
            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
        }
        url
    }

    /// A fresh read-only session, as the app opens it.
    pub(super) async fn session(url: &str) -> Conn {
        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
        spec.tls = TlsMode::Disable;
        Conn::connect(&spec, &secrets, None, Access::ReadOnly)
            .await
            .unwrap()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancel_that_reaches_another_server_kills_nothing() {
        let Some(url) = test_url() else {
            return;
        };
        let session = std::sync::Arc::new(session(&url).await);
        assert!(session.server.starts_with("uuid "), "{}", session.server);
        let running = {
            let session = std::sync::Arc::clone(&session);
            tokio::spawn(async move {
                let mut conn = session.conn.lock().await;
                // SLEEP answers 1 when a KILL QUERY interrupts it.
                conn.query_first::<i64, _>("SELECT SLEEP(2)").await
            })
        };
        tokio::time::sleep(Duration::from_millis(300)).await;
        let other = "uuid 00000000-0000-0000-0000-000000000000";
        assert_eq!(
            cancel(&session.opts, session.id, other).await,
            Err(Error::Connect(
                "could not cancel: reached a different server".into()
            ))
        );
        assert_eq!(running.await.unwrap().unwrap(), Some(0));
    }

    #[test]
    fn versions_lose_what_a_distribution_adds() {
        assert_eq!(version_name("8.4.3"), "MySQL 8.4.3");
        assert_eq!(version_name("8.0.36-0ubuntu0.22.04.1"), "MySQL 8.0.36");
        assert_eq!(
            version_name("10.11.6-MariaDB-1:10.11.6+maria~ubu2204"),
            "MariaDB 10.11.6"
        );
        assert_eq!(version_name("11.4.2-MariaDB"), "MariaDB 11.4.2");
    }

    /// The connect-time settings undo what a server's defaults (or a reset
    /// to them) can hold: a read-write session, a mode that changes how
    /// text is lexed, another character set.
    #[tokio::test]
    async fn the_session_settings_undo_modes_that_change_lexing() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let mut conn = conn.conn.lock().await;
        for mode in [
            // A combination mode holds ANSI_QUOTES, and brings it back when
            // only ANSI_QUOTES is taken out.
            "ANSI",
            "ANSI_QUOTES,NO_BACKSLASH_ESCAPES,STRICT_ALL_TABLES",
            "NO_BACKSLASH_ESCAPES,ANSI,TRADITIONAL",
            "",
        ] {
            conn.query_drop(format!("SET SESSION sql_mode = '{mode}'"))
                .await
                .unwrap();
            conn.query_drop("SET SESSION TRANSACTION READ WRITE")
                .await
                .unwrap();
            conn.query_drop("SET NAMES latin1").await.unwrap();
            prepare_session(&mut conn, Access::ReadOnly).await.unwrap();
            let (read_only, sql_mode, client, collation): (i64, String, String, String) = conn
                .query_first(
                    "SELECT @@session.transaction_read_only, @@session.sql_mode, \
                            @@session.character_set_client, @@session.collation_connection",
                )
                .await
                .unwrap()
                .unwrap();
            assert_eq!(read_only, 1, "{mode}");
            assert!(!sql_mode.contains("ANSI"), "{mode}: {sql_mode}");
            assert!(
                !sql_mode.contains("NO_BACKSLASH_ESCAPES"),
                "{mode}: {sql_mode}"
            );
            // What does not change lexing stays.
            assert_eq!(
                sql_mode.contains("STRICT_ALL_TABLES"),
                mode.contains("STRICT_ALL_TABLES") || mode.contains("TRADITIONAL"),
                "{mode}: {sql_mode}"
            );
            assert_eq!(client, "utf8mb4", "{mode}");
            assert_eq!(collation, "utf8mb4_general_ci", "{mode}");
            // A quoted string is a string, and a backslash escapes.
            let texts: Option<(String, String)> =
                conn.exec_first(r#"SELECT "a", 'b\'c'"#, ()).await.unwrap();
            assert_eq!(texts, Some(("a".into(), "b'c".into())), "{mode}");
            prepare_session(&mut conn, Access::Writable).await.unwrap();
            let (read_only, sql_mode): (i64, String) = conn
                .query_first("SELECT @@session.transaction_read_only, @@session.sql_mode")
                .await
                .unwrap()
                .unwrap();
            assert_eq!(read_only, 0, "{mode}");
            assert!(!sql_mode.contains("ANSI"), "{mode}: {sql_mode}");
        }
    }

    #[test]
    fn a_tunnelled_mysql_opts_keep_the_host_name_for_tls() {
        let (mut spec, secrets) = ConnectSpec::from_url("mysql://me@db.example.com/shop").unwrap();
        spec.tls = TlsMode::VerifyFull;
        let opts: Opts = with_tls(builder(&spec, &secrets, Some(40002)), &spec, Some(40002)).into();
        assert_eq!(opts.ip_or_hostname(), "127.0.0.1");
        assert_eq!(opts.tcp_port(), 40002);
        assert_eq!(
            opts.ssl_opts().and_then(|ssl| ssl.tls_hostname_override()),
            Some("db.example.com")
        );
        let direct: Opts = with_tls(builder(&spec, &secrets, None), &spec, None).into();
        assert_eq!(direct.ip_or_hostname(), "db.example.com");
        assert_eq!(
            direct
                .ssl_opts()
                .and_then(|ssl| ssl.tls_hostname_override()),
            None
        );
    }
}
