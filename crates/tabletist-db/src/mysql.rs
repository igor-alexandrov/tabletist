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
use mysql_async::{DriverError, IoError, Opts, OptsBuilder, Params, SslOpts};
use mysql_common::named_params::ParsedNamedParams;

use crate::adapter::Adapter;
use crate::agent::{Catalog, ColumnName, Limits, Read, Sources};
use crate::script::retry_cancelled;
use crate::{
    Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, ConnectSpec,
    Ddl, Dialect, Error, ForeignKeyInfo, IndexInfo, Lookup, LookupPage, MAX_LISTED, Named,
    ObjectInfo, ObjectKind, ObjectRef, ReferencingKey, Result, RowPage, RowQuery, ScriptMode,
    ScriptOutcome, Secrets, StopFlag, Structure, TlsMode, Value, ValueKind, WriteOutcome,
    column_class,
};

/// A column as `describe` reads it from `information_schema.columns`: its
/// name, its type, whether it takes NULL, its default, its comment, its
/// `extra` and its collation.
type ColumnRow = (
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
);

/// MySQL's `binary` character set: bytes, not text.
const BINARY_CHARSET: u16 = 63;
const ACCESS_DENIED: u16 = 1045;
const QUERY_INTERRUPTED: u16 = 1317;
const NO_SUCH_THREAD: u16 = 1094;
const UNKNOWN_SYSTEM_VARIABLE: u16 = 1193;
/// The server's limit on connections, all of them or one user's.
const TOO_MANY_CONNECTIONS: u16 = 1040;
const TOO_MANY_USER_CONNECTIONS: u16 = 1203;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

mod hidden;
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
    /// A second connection kept for KILL QUERY (see [`cancel`]).
    pub(crate) spare: Spare,
    /// The spare's open beside the session, stopped when the session is.
    _opening: Opening,
}

/// The task that opens a session's spare. Dropped with the session, it is
/// stopped: a session closed straight after it opened (a test of the
/// connection) does not go on signing in a second one. A spare it had
/// opened closes with the last cancel handle that holds it.
struct Opening(tokio::task::AbortHandle);

impl Drop for Opening {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// The connection a session's cancels go through, signed in and checked
/// to have reached the session's server, kept between cancels: MySQL has
/// no cancel in its protocol, and a new connection for each cancel is a
/// sign-in, a check and the KILL, several round trips far from the server.
/// `None` until it is open, and after the server closed it. The lock also
/// keeps one cancel at a time.
pub(crate) type Spare = std::sync::Arc<tokio::sync::Mutex<Option<mysql_async::Conn>>>;

/// Every session's spare in this app, for [`give_up`]: a spare
/// holds a connection for as long as its session's cancel handles live.
static SPARES: std::sync::Mutex<
    Vec<std::sync::Weak<tokio::sync::Mutex<Option<mysql_async::Conn>>>>,
> = std::sync::Mutex::new(Vec::new());

/// Puts `spare` on [`SPARES`], and takes off those whose sessions are gone.
fn kept_spares(spare: &Spare) {
    let mut spares = SPARES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    spares.retain(|kept| kept.strong_count() > 0);
    spares.push(std::sync::Arc::downgrade(spare));
}

/// The spares on [`SPARES`] whose sessions are still there.
fn all_spares() -> Vec<Spare> {
    SPARES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .filter_map(std::sync::Weak::upgrade)
        .collect()
}

/// Closes the `spares` on the server `refused` names (its host, port and
/// user) that no cancel is using, and says whether it closed one. Each
/// holds a connection there, which counts towards the server's limit on
/// connections (often a few for one user on hosted MySQL): a session is
/// worth more than a quicker cancel. A cancel after this opens its spare
/// again, which a full server refuses, and then the cancel goes without
/// one.
async fn give_up(spares: Vec<Spare>, refused: &Opts) -> bool {
    let same = |opts: &Opts| {
        opts.ip_or_hostname() == refused.ip_or_hostname()
            && opts.tcp_port() == refused.tcp_port()
            && opts.user() == refused.user()
    };
    let closing: Vec<mysql_async::Conn> = spares
        .iter()
        .filter_map(|spare| {
            // A held lock is a cancel on its way, or a spare being opened.
            let mut slot = spare.try_lock().ok()?;
            if !same(slot.as_ref()?.opts()) {
                return None;
            }
            slot.take()
        })
        .collect();
    let closed = !closing.is_empty();
    // All at once, each given [`GOODBYE`]: the slot is free once the
    // server reads the goodbye, and one sent on a path that is gone must
    // not take the session's own time to sign in.
    futures_util::future::join_all(
        closing
            .into_iter()
            .map(|conn| tokio::time::timeout(GOODBYE, conn.disconnect())),
    )
    .await;
    closed
}

/// How long a spare given up is given to say goodbye. It waits for no
/// answer: the goodbye is written, and the connection closed.
const GOODBYE: Duration = Duration::from_secs(1);

/// A session's sign-in. A server at its limit on connections, where this
/// app's spares hold some, is asked again once they are closed (see
/// [`give_up`]): a few times, since the server lets a connection's slot
/// go only a moment after the connection said goodbye.
async fn sign_in(opts: Opts) -> mysql_async::Result<mysql_async::Conn> {
    let signed_in = mysql_async::Conn::new(opts.clone()).await;
    if !matches!(&signed_in, Err(error) if is_full(error)) || !give_up(all_spares(), &opts).await {
        return signed_in;
    }
    let mut tries = 0;
    loop {
        let signed_in = mysql_async::Conn::new(opts.clone()).await;
        tries += 1;
        match signed_in {
            Err(error) if is_full(&error) && tries < 5 => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            signed_in => return signed_in,
        }
    }
}

/// Whether the server refused a sign-in for its limit on connections.
fn is_full(error: &mysql_async::Error) -> bool {
    matches!(
        error,
        mysql_async::Error::Server(server)
            if server.code == TOO_MANY_CONNECTIONS || server.code == TOO_MANY_USER_CONNECTIONS
    )
}

impl Conn {
    /// An export's rows through a prepared statement inside a read-only
    /// transaction, read from the server a row at a time (the driver does
    /// not buffer a result), sent on a batch at a time: a table's
    /// statement, or a SQL editor's.
    async fn stream_sql(
        &self,
        sql: &crate::dialect::Sql,
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        let mut conn = self.conn.lock().await;
        let streamed = async {
            begin_read(&mut conn).await.map_err(StreamFailed::Before)?;
            stream_into(&mut conn, sql, batch, to, stop).await
        }
        .await;
        stop.finish();
        match streamed {
            Ok(sent) => end_read(&mut conn, Ok(sent)).await,
            // Nothing is on its way: the transaction ends here.
            Err(StreamFailed::Before(error)) => end_read(&mut conn, Err(error))
                .await
                .map_err(crate::agent::over_its_time),
            // Rows the stream did not read are still on their way: a
            // rollback now would read every one of them first. The
            // export's connection is closed after a failure, which ends
            // the transaction with it (see `Connection::stream_rows`).
            Err(StreamFailed::PartWay(error)) => Err(crate::agent::over_its_time(error)),
        }
    }

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
            match tokio::time::timeout(CONNECT_TIMEOUT, sign_in(with_tls.clone())).await {
                Err(_) => return Err(Error::Timeout),
                Ok(Ok(conn)) => (conn, with_tls),
                // `prefer`: a server without TLS gets a plain connection.
                Ok(Err(mysql_async::Error::Driver(DriverError::NoClientSslFlagFromServer)))
                    if spec.tls == TlsMode::Prefer =>
                {
                    let plain: Opts = builder.ssl_opts(None::<mysql_async::SslOpts>).into();
                    (sign_in(plain.clone()).await.map_err(connect_error)?, plain)
                }
                Ok(Err(error)) => return Err(connect_error(error)),
            };
        // The settings and the question who the server is go as one
        // message: the settings run first, and the server stops at the
        // first statement that fails.
        let (server, version) = server_identity(&mut conn, &session_settings(access)).await?;
        let id = conn.id();
        // mysql_async fails rather than go on in plain text when it was
        // given TLS options, so the options that connected say it.
        let encrypted = opts.ssl_opts().is_some();
        let spare = Spare::default();
        kept_spares(&spare);
        // Opened beside the session, so connecting waits for none of it. A
        // cancel that comes first waits for it, or opens it itself. One
        // that cannot open (a limit on connections, say) leaves each cancel
        // to open its own; one that took the slot a later session needs is
        // closed by that session's connect (see `sign_in`).
        let opening = (std::sync::Arc::clone(&spare), opts.clone(), server.clone());
        let opening = tokio::spawn(async move {
            let (spare, opts, server) = opening;
            let mut slot = spare.lock().await;
            if slot.is_none() {
                *slot = open_spare(&opts, &server).await.ok();
            }
        });
        Ok(Self {
            conn: tokio::sync::Mutex::new(conn),
            opts,
            id,
            encrypted,
            server,
            version,
            access,
            spare,
            _opening: Opening(opening.abort_handle()),
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
    async fn binary_columns(&self, query: &RowQuery) -> Result<Vec<(String, ColumnClass)>> {
        if !crate::dialect::reads_bytes(query) {
            return Ok(Vec::new());
        }
        let names: Vec<String> = self
            .catalog(
                "SELECT column_name FROM information_schema.columns \
                 WHERE table_schema = ? AND table_name = ? AND data_type IN \
                       ('binary', 'varbinary', 'tinyblob', 'blob', 'mediumblob', 'longblob')",
                (&query.object.schema, &query.object.name),
            )
            .await?;
        Ok(names
            .into_iter()
            .map(|name| (name, ColumnClass::Binary))
            .collect())
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
            spare: std::sync::Arc::clone(&self.spare),
        })
    }

    /// See [`crate::Connection::server_version`]. Asked at connect.
    async fn server_version(&self) -> Result<String> {
        Ok(version_name(&self.version))
    }

    /// See [`crate::Connection::session_zone`]: the session's zone, or the
    /// server's own where the session follows it (`SYSTEM`).
    async fn session_zone(&self) -> Result<Option<String>> {
        let zones: Vec<String> = self
            .catalog(
                "SELECT IF(@@session.time_zone = 'SYSTEM', @@global.system_time_zone, \
                 @@session.time_zone)",
                (),
            )
            .await?;
        Ok(zones.into_iter().next())
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
        let columns: Vec<ColumnRow> = self
            .catalog(
                "SELECT column_name, column_type, is_nullable, column_default, column_comment, \
                        COALESCE(extra, ''), COALESCE(collation_name, '') \
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
        // The conditions of the table's CHECK constraints that hold.
        // MariaDB is asked first, by the table's name: its constraint
        // names are a table's own, so MySQL's question below could answer
        // with another table's. MySQL has no such column there and
        // refuses, and is asked its own way. A server with no list at all
        // (MySQL before 8.0.16) has no CHECK that holds either.
        let of_table = self
            .catalog::<String>(
                "SELECT check_clause FROM information_schema.check_constraints \
                 WHERE constraint_schema = ? AND table_name = ? \
                 ORDER BY constraint_name",
                at,
            )
            .await;
        let checks: Vec<String> = match of_table {
            Ok(checks) => checks,
            Err(Error::Query { .. }) => match self
                .catalog(
                    "SELECT cc.check_clause \
                     FROM information_schema.table_constraints tc \
                     JOIN information_schema.check_constraints cc \
                       ON cc.constraint_schema = tc.constraint_schema \
                      AND cc.constraint_name = tc.constraint_name \
                     WHERE tc.table_schema = ? AND tc.table_name = ? \
                       AND tc.constraint_type = 'CHECK' AND tc.enforced = 'YES' \
                     ORDER BY cc.constraint_name",
                    at,
                )
                .await
            {
                Ok(checks) => checks,
                Err(Error::Query { .. }) => Vec::new(),
                Err(other) => return Err(other),
            },
            Err(other) => return Err(other),
        };
        let columns = columns
            .into_iter()
            .map(
                |(name, type_name, nullable, default, comment, extra, collation)| {
                    // A list is the app's to hold a typed value to, letter
                    // for letter. MySQL's default collations take `Print`
                    // for `print`, so only a text column that tells them
                    // apart has one here.
                    let class = column_class(Dialect::MySql, &type_name);
                    let text = matches!(class, ColumnClass::Text { .. });
                    let allowed_values = (text && compares_exactly(&collation))
                        .then(|| {
                            checks
                                .iter()
                                .find_map(|check| crate::check::mysql_allowed_values(check, &name))
                        })
                        .flatten();
                    ColumnInfo {
                        name,
                        type_name,
                        nullable: nullable == "YES",
                        default,
                        comment: (!comment.is_empty()).then_some(comment),
                        allowed_values,
                        // `VIRTUAL GENERATED`, `STORED GENERATED`, and on an older
                        // MariaDB `VIRTUAL` or `PERSISTENT`. Not `DEFAULT_GENERATED`,
                        // which MySQL 8 says of a default that is an expression.
                        generated: ["VIRTUAL", "STORED", "PERSISTENT"]
                            .iter()
                            .any(|word| extra.to_ascii_uppercase().contains(word)),
                        identity: extra.to_ascii_lowercase().contains("auto_increment"),
                    }
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
        // The keys that point at the table: the same views, asked by what
        // a key refers to.
        let referencing: Vec<(String, String, String, String, String, String)> = self
            .catalog(
                "SELECT k.constraint_name, k.table_schema, k.table_name, k.column_name, \
                        k.referenced_column_name, r.delete_rule \
                 FROM information_schema.key_column_usage k \
                 JOIN information_schema.referential_constraints r \
                   ON r.constraint_schema = k.constraint_schema AND r.constraint_name = k.constraint_name \
                  AND r.table_name = k.table_name \
                 WHERE k.referenced_table_schema = ? AND k.referenced_table_name = ? \
                 ORDER BY k.table_schema, k.table_name, k.constraint_name, k.ordinal_position",
                at,
            )
            .await?;
        let mut referenced_by: Vec<ReferencingKey> = Vec::new();
        for (name, schema, table, column, ref_column, on_delete) in referencing {
            let same = referenced_by.last().is_some_and(|key| {
                key.name.as_deref() == Some(name.as_str())
                    && key.schema == schema
                    && key.table == table
            });
            if !same {
                referenced_by.push(ReferencingKey {
                    name: Some(name),
                    schema,
                    table,
                    columns: Vec::new(),
                    ref_columns: Vec::new(),
                    on_delete,
                });
            }
            if let Some(key) = referenced_by.last_mut() {
                key.columns.push(column);
                key.ref_columns.push(ref_column);
            }
        }
        // The comment on the table itself. MySQL writes `VIEW` there for
        // every view, and a warning for one that no longer works: nobody
        // wrote either, so a view has none. A system view is one too, as
        // the listing takes it.
        let comment = self
            .catalog::<String>(
                "SELECT CASE WHEN table_type LIKE '%VIEW' THEN '' \
                        ELSE COALESCE(table_comment, '') END \
                 FROM information_schema.tables \
                 WHERE table_schema = ? AND table_name = ?",
                at,
            )
            .await?
            .into_iter()
            .next()
            .filter(|comment| !comment.is_empty());
        Ok(Structure {
            columns,
            primary_key: self.primary_key(object).await?,
            indexes,
            foreign_keys,
            referenced_by,
            comment,
        })
    }

    /// `SHOW CREATE TABLE`, or `SHOW CREATE VIEW` for a view, as the
    /// server returns it, then `SHOW CREATE TRIGGER` for each of the
    /// table's triggers by name, in one read-only transaction.
    async fn ddl(&self, object: &ObjectRef) -> Result<Ddl> {
        let mut conn = self.conn.lock().await;
        let outcome = async {
            begin_read(&mut conn).await?;
            show_create(&mut conn, object).await
        }
        .await;
        let parts = end_read(&mut conn, outcome).await?;
        let server = version_name(&self.version);
        Ok(Ddl::new(crate::Driver::MySql, server, None, parts))
    }

    /// One page, through a prepared statement inside a read-only
    /// transaction. Reading stops after `limit + 1` rows.
    async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        let key = self.primary_key(&query.object).await?;
        let binary = self.binary_columns(query).await?;
        let sql = Dialect::MySql.select_rows(query, &key, &binary)?;
        let limit = query.limit as usize;
        let mut conn = self.conn.lock().await;
        let started = Instant::now();
        let outcome = async {
            begin_read(&mut conn).await?;
            read_page(&mut conn, &sql, limit).await
        }
        .await;
        let (columns, mut rows) = end_read(&mut conn, outcome).await?;
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

    async fn export_statement(&self, query: &RowQuery, columns: &[String]) -> Result<String> {
        let key = self.primary_key(&query.object).await?;
        let binary = self.binary_columns(query).await?;
        Dialect::MySql.shown_export(query, &key, &binary, columns)
    }

    /// A table's export rows, through `stream_sql`.
    async fn stream_rows(
        &self,
        query: &RowQuery,
        columns: &[String],
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        let key = self.primary_key(&query.object).await?;
        let binary = self.binary_columns(query).await?;
        let sql = Dialect::MySql.export_rows(query, &key, &binary, columns)?;
        self.stream_sql(&sql, batch, to, stop).await
    }

    /// A statement of the person's, as `stream_rows` streams a table's.
    async fn stream_statement(
        &self,
        text: &str,
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        let sql = crate::dialect::Sql {
            text: text.to_owned(),
            params: Vec::new(),
        };
        self.stream_sql(&sql, batch, to, stop).await
    }

    async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        let binary = self.binary_columns(query).await?;
        let sql = Dialect::MySql.count_rows(query, &binary)?;
        let mut conn = self.conn.lock().await;
        let outcome = async {
            begin_read(&mut conn).await?;
            count(&mut conn, &sql).await
        }
        .await;
        let row = end_read(&mut conn, outcome).await?;
        row.map(from_row::<u64>)
            .transpose()?
            .ok_or_else(|| Error::query("the count returned no number"))
    }

    async fn lookup(&self, lookup: &Lookup) -> Result<LookupPage> {
        let sql = Dialect::MySql.lookup(lookup);
        let mut conn = self.conn.lock().await;
        // `read_page` stops one row past the limit.
        let outcome = async {
            begin_read(&mut conn).await?;
            read_page(&mut conn, &sql, lookup.limit as usize).await
        }
        .await;
        let (_, rows) = end_read(&mut conn, outcome).await?;
        crate::lookup::page(rows, lookup)
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

    async fn run_read(
        &self,
        read: &Read,
        limits: Limits,
        sources: &Sources,
        stop: &StopFlag,
    ) -> Result<(ScriptOutcome, Vec<bool>)> {
        self.agent_read(read, limits, sources, stop).await
    }

    async fn agents_catalog(&self, masked: &[ColumnName]) -> Result<Catalog> {
        let mut conn = self.conn.lock().await;
        hidden::catalog(&mut conn, masked).await
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
    conn: &mut mysql_async::Conn,
    sql: &crate::dialect::Sql,
    limit: usize,
) -> Result<(Vec<ColumnMeta>, Vec<Vec<Value>>)> {
    let statement = prepare(conn, sql).await?;
    let mut result = conn
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

/// An export's rows, a batch at a time (see [`Conn::stream_rows`]).
async fn stream_into(
    conn: &mut mysql_async::Conn,
    sql: &crate::dialect::Sql,
    batch: usize,
    to: &tokio::sync::mpsc::Sender<crate::Streamed>,
    stop: &StopFlag,
) -> std::result::Result<u64, StreamFailed> {
    let statement = prepare(conn, sql).await.map_err(StreamFailed::Before)?;
    let mut result = conn
        .exec_iter(&statement, params(&sql.params))
        .await
        .map_err(|error| StreamFailed::Before(query_error(error)))?;
    sent_on(&mut result, batch, to, stop)
        .await
        .map_err(StreamFailed::PartWay)
}

/// Where an export's stream failed.
enum StreamFailed {
    /// Before the statement ran: nothing is left to read.
    Before(Error),
    /// Once it ran, with rows still on their way, read or not.
    PartWay(Error),
}

/// The rows of a statement that runs, sent on to `to` a batch at a time.
async fn sent_on(
    result: &mut mysql_async::QueryResult<'_, '_, mysql_async::BinaryProtocol>,
    batch: usize,
    to: &tokio::sync::mpsc::Sender<crate::Streamed>,
    stop: &StopFlag,
) -> Result<u64> {
    let columns = column_metas(result.columns_ref());
    crate::stream::send(to, crate::Streamed::Columns(columns.clone())).await?;
    let mut sent = 0u64;
    let mut rows = Vec::with_capacity(batch);
    while let Some(row) = result.next().await.map_err(query_error)? {
        rows.push(row_values(row, &columns));
        if rows.len() == batch {
            if stop.is_stopped() {
                return Err(Error::Cancelled);
            }
            sent += rows.len() as u64;
            let full = std::mem::replace(&mut rows, Vec::with_capacity(batch));
            crate::stream::send(to, crate::Streamed::Rows(full)).await?;
        }
    }
    if !rows.is_empty() {
        sent += rows.len() as u64;
        crate::stream::send(to, crate::Streamed::Rows(rows)).await?;
    }
    Ok(sent)
}

/// The row of a count.
async fn count(
    conn: &mut mysql_async::Conn,
    sql: &crate::dialect::Sql,
) -> Result<Option<mysql_async::Row>> {
    let statement = prepare(conn, sql).await?;
    conn.exec_first(&statement, params(&sql.params))
        .await
        .map_err(query_error)
}

/// Prepares a row query's statement, and refuses one with a parameter the
/// query has no value for: the driver closes the connection when a
/// statement runs without a value for each of its parameters. The filters
/// bind theirs, so such a parameter comes from the raw WHERE.
async fn prepare(
    conn: &mut mysql_async::Conn,
    sql: &crate::dialect::Sql,
) -> Result<mysql_async::Statement> {
    driver_parameter(&sql.text)?;
    let statement = conn.prep(sql.text.as_str()).await.map_err(query_error)?;
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

/// Whether a column of this collation holds two values apart that differ
/// in a letter's case or its accent: what the app's list of a CHECK's
/// values does. By the name's end, since `_cs` inside it is a language's
/// tag (`utf8mb4_cs_0900_ai_ci` is Czech, and takes either case). MariaDB
/// has collations that tell case apart and not accents
/// (`utf8mb4_uca1400_ai_cs`): those are not exact.
///
/// One gap is left, and accepted: most of these collations pad
/// (`utf8mb4_bin` is `PAD SPACE`), so the server takes `'print '` with a
/// space after it for `print`, and the app's list refuses it.
fn compares_exactly(collation: &str) -> bool {
    ["_bin", "_cs", "_cs_ks"]
        .iter()
        .any(|end| collation.ends_with(end))
        && !collation.contains("_ai_")
}

/// A raw WHERE's parameter, as a query error: there is no value for it.
fn parameter(spelled: &str, hint: Option<&str>) -> Error {
    Error::Query {
        code: None,
        message: format!("The WHERE text has a parameter ({spelled}), which has no value."),
        detail: None,
        hint: hint.map(str::to_owned),
        named: Box::default(),
    }
}

/// The parts of `object`'s definition, read by [`Adapter::ddl`] in
/// its read-only transaction.
async fn show_create(
    conn: &mut mysql_async::Conn,
    object: &ObjectRef,
) -> Result<Vec<crate::ddl::Part>> {
    use crate::ddl::{Part, PartKind};
    let kind: Option<String> = conn
        .exec_first(
            "SELECT table_type FROM information_schema.tables \
             WHERE table_schema = ? AND table_name = ?",
            (&object.schema, &object.name),
        )
        .await
        .map_err(query_error)?;
    let Some(kind) = kind else {
        return Err(Error::query(format!(
            "no such table or view: {}",
            object.name
        )));
    };
    let name = format!(
        "{}.{}",
        Dialect::MySql.quote_ident(&object.schema),
        Dialect::MySql.quote_ident(&object.name)
    );
    let shown = if kind.contains("VIEW") {
        format!("SHOW CREATE VIEW {name}")
    } else {
        format!("SHOW CREATE TABLE {name}")
    };
    // Its second column, whichever it is: `Create Table` or `Create View`.
    let own = nth_text(conn, &shown, 1).await?;
    let names: Vec<String> = conn
        .exec(
            "SELECT trigger_name FROM information_schema.triggers \
             WHERE event_object_schema = ? AND event_object_table = ? \
             ORDER BY trigger_name",
            (&object.schema, &object.name),
        )
        .await
        .map_err(query_error)?;
    let mut triggers = Vec::with_capacity(names.len());
    for trigger in names {
        let shown = format!(
            "SHOW CREATE TRIGGER {}.{}",
            Dialect::MySql.quote_ident(&object.schema),
            Dialect::MySql.quote_ident(&trigger)
        );
        // `SQL Original Statement`, after the name and the `sql_mode`.
        triggers.push(nth_text(conn, &shown, 2).await?);
    }
    Ok(vec![
        Part {
            kind: PartKind::Object,
            statements: vec![own],
        },
        Part {
            kind: PartKind::Triggers,
            statements: triggers,
        },
    ])
}

/// Column `index` of the one row `sql` answers, as text.
async fn nth_text(conn: &mut mysql_async::Conn, sql: &str, index: usize) -> Result<String> {
    let row: Option<mysql_async::Row> = conn.query_first(sql).await.map_err(query_error)?;
    row.and_then(|row| row.get_opt::<String, _>(index))
        .transpose()
        .map_err(|_| Error::query("unexpected data from the server: the text is not text"))?
        .ok_or_else(|| Error::query(format!("{sql} returned nothing")))
}

/// Starts a read's transaction: read-only, in one statement. The
/// driver's own read-only transaction takes two (`SET TRANSACTION READ
/// ONLY`, then `START TRANSACTION`), each a round trip, and a cancel
/// between them leaves "the next transaction is read-only" pending (see
/// `write.rs`). What follows must end with [`end_read`], also when this
/// fails: a cancel can land after the server began the transaction.
///
/// The read's future is awaited to its end, as every command's is: the
/// backend runs a session's commands one at a time and drops none. One
/// dropped half-way would leave the session inside this transaction,
/// which the driver's own would have rolled back on the next query.
///
/// `START TRANSACTION` commits a transaction that is open, so one left
/// open (by a dropped save, say) is rolled back first, as the save's own
/// start does: the server's status after the last query says so, with no
/// question of its own.
async fn begin_read(conn: &mut mysql_async::Conn) -> Result<()> {
    if status(conn).contains(StatusFlags::SERVER_STATUS_IN_TRANS) {
        execute(conn, write::ROLLBACK).await?;
    }
    execute(conn, "START TRANSACTION READ ONLY").await
}

/// Ends a read's transaction whatever happened inside it, so that a failed
/// or cancelled read keeps no locks and the next catalog query does not
/// read the transaction's old snapshot. A cancel meant for the read can
/// land on the rollback: it runs once more. The read's own error wins,
/// unless the rollback failed: a session left inside the transaction is
/// closed (a lost connection).
async fn end_read<T>(conn: &mut mysql_async::Conn, outcome: Result<T>) -> Result<T> {
    let rolled_back = retry_cancelled!(execute(conn, write::ROLLBACK));
    ended(outcome, rolled_back)
}

/// What a read comes to, from its outcome and its rollback's (see
/// [`end_read`]).
fn ended<T>(outcome: Result<T>, rolled_back: Result<()>) -> Result<T> {
    match (outcome, rolled_back) {
        (outcome, Ok(())) => outcome,
        (Err(lost), Err(_)) if lost.is_connection_lost() => Err(lost),
        (_, Err(error)) => Err(Error::ConnectionLost(format!(
            "could not end the read's transaction: {error}"
        ))),
    }
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
/// again after a SQL editor script's reset, which undoes them: one
/// message of fixed statements, each ending in `; `, which the server
/// runs in order and stops at the first that fails. A read-only session
/// says so first, so nothing here runs in a session that could write; a
/// writable one says read-write last.
fn session_settings(access: Access) -> String {
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
    statements
        .map(|statement| format!("{statement}; "))
        .concat()
}

/// Sets [`session_settings`]. After a reset a cancel meant for a
/// statement can land here, so a message it interrupts runs once more: the
/// session is not left in the wrong mode because a `SET` was interrupted.
async fn prepare_session(conn: &mut mysql_async::Conn, access: Access) -> Result<()> {
    // Fixed statements: safe to send through the text protocol.
    let settings = session_settings(access);
    retry_cancelled!(execute(conn, settings.trim_end_matches("; ")))
}

/// The row a message of several statements answers last: the last row of
/// the last result set that has one. The driver keeps the error of a
/// statement after the first as a pending result, which `is_empty` does
/// not count: only `drop_result` gives it.
async fn last_row(
    conn: &mut mysql_async::Conn,
    text: &str,
) -> mysql_async::Result<Option<mysql_async::Row>> {
    let mut result = conn.query_iter(text).await?;
    let mut last = None;
    loop {
        let rows: Vec<mysql_async::Row> = result.collect().await?;
        last = rows.into_iter().last().or(last);
        if result.is_empty() {
            break;
        }
    }
    result.drop_result().await?;
    Ok(last)
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
            named: Box::default(),
        },
        mysql_async::Error::Io(IoError::Tls(tls)) => Error::Tls(tls.to_string()),
        mysql_async::Error::Driver(DriverError::NoClientSslFlagFromServer) => {
            Error::Tls("the server does not support TLS".into())
        }
        other => Error::Connect(other.to_string()),
    }
}

/// What the server's error `number` with this `message` is of. MySQL says
/// it only in the message, so the parts it quotes are read by the number:
/// never by the message's words, which follow the server's language.
pub(crate) fn named(number: u32, message: &str) -> Named {
    // What stands between the first two of `quote` from `from` on.
    let quoted = |from: &str, quote: char| -> Option<String> {
        let rest = &from[from.find(quote)? + quote.len_utf8()..];
        Some(rest[..rest.find(quote)?].to_owned())
    };
    let (constraint, columns) = match number {
        // ER_DUP_ENTRY: "... for key 'table.key'", the key last. The
        // value comes before it and may hold a quote of its own. The key
        // is what follows the first dot: its name can hold one (an older
        // Prisma's `User.email_unique`). So can a table's, and a server
        // that names no table (before 8.0.19, MariaDB) leaves no dot of
        // its own: a dotted table there, or a dotted key here, is read
        // short, and then names a key the structure does not have.
        1062 => {
            let key = message
                .strip_suffix('\'')
                .and_then(|rest| rest.rsplit('\'').next());
            let key = key.map(|key| key.split_once('.').map_or(key, |(_, key)| key).to_owned());
            (key, Vec::new())
        }
        // ER_ROW_IS_REFERENCED_2: a row another table's foreign key holds.
        // The constraint as it was defined, as 1452's. Its columns are the
        // other table's, and a caller looks for a named column in the
        // table it wrote to, so they are left out.
        1451 => {
            let constraint = message
                .split_once("CONSTRAINT ")
                .and_then(|(_, rest)| quoted(rest, '`'));
            (constraint, Vec::new())
        }
        // ER_NO_REFERENCED_ROW_2: the constraint as it was defined, which
        // is SQL and not translated.
        1452 => {
            let constraint = message
                .split_once("CONSTRAINT ")
                .and_then(|(_, rest)| quoted(rest, '`'));
            let columns = message
                .split_once("FOREIGN KEY (")
                .and_then(|(_, rest)| rest.split_once(')'))
                .map(|(columns, _)| {
                    columns
                        .split(',')
                        .map(|column| column.trim().trim_matches('`').to_owned())
                        .collect()
                })
                .unwrap_or_default();
            (constraint, columns)
        }
        // ER_BAD_NULL_ERROR, ER_NO_DEFAULT_FOR_FIELD: the column, quoted.
        1048 | 1364 => (None, quoted(message, '\'').into_iter().collect()),
        // ER_CHECK_CONSTRAINT_VIOLATED: the constraint, quoted.
        3819 => (quoted(message, '\''), Vec::new()),
        _ => (None, Vec::new()),
    };
    Named {
        number: Some(number),
        constraint,
        columns,
    }
}

pub(crate) fn query_error(error: mysql_async::Error) -> Error {
    match error {
        mysql_async::Error::Server(server) if server.code == QUERY_INTERRUPTED => Error::Cancelled,
        mysql_async::Error::Server(server) => Error::Query {
            named: Box::new(named(u32::from(server.code), &server.message)),
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

/// Stops the query thread `id` runs on the server named `server`, through
/// the session's `spare` connection: one round trip, the KILL, where it is
/// open. Where it is not, or the server closed it (`wait_timeout`, a
/// restart), or it does not answer in [`CONNECT_TIMEOUT`], a new one is
/// opened and checked, and kept for the next cancel.
///
/// Every wait on a spare has that limit: one kept idle between cancels can
/// sit on a path that a NAT or firewall, or a laptop's sleep, dropped
/// without a word, and the wait for it would last minutes, while later
/// cancels wait on the lock and `run_session` on the cancels.
pub(crate) async fn cancel(opts: &Opts, id: u32, server: &str, spare: &Spare) -> Result<()> {
    let kill = format!("KILL QUERY {id}");
    let mut slot = spare.lock().await;
    if let Some(conn) = slot.as_mut() {
        match tokio::time::timeout(CONNECT_TIMEOUT, conn.query_drop(&kill)).await {
            Ok(killed) if spare_answered(&killed) => return killed.map_err(cancel_failed),
            // Anything else may be a connection the server has closed,
            // which can answer with an error of its own first (an idle one
            // past `wait_timeout`, say), or one whose path is gone: a new
            // one below.
            _ => *slot = None,
        }
    }
    let mut conn = open_spare(opts, server).await?;
    let Ok(killed) = tokio::time::timeout(CONNECT_TIMEOUT, conn.query_drop(&kill)).await else {
        return Err(Error::Timeout);
    };
    if spare_answered(&killed) {
        *slot = Some(conn);
    }
    killed.map_err(cancel_failed)
}

/// Whether a KILL's answer shows its connection works: it was done, or the
/// server knows no such thread (the session is gone).
fn spare_answered(killed: &mysql_async::Result<()>) -> bool {
    match killed {
        Ok(()) => true,
        Err(mysql_async::Error::Server(error)) => error.code == NO_SUCH_THREAD,
        Err(_) => false,
    }
}

/// A connection for [`cancel`], checked to have reached the server named
/// `server`. It resolves the host again, and behind round robin DNS or a
/// load balancer it can reach another server, where the session's thread
/// id is someone else's: so it is used only after that check. A connection
/// stays with the server it reached, so the check holds for as long as it
/// is kept.
async fn open_spare(opts: &Opts, server: &str) -> Result<mysql_async::Conn> {
    open_checked(opts, server, SPARE_SETTINGS).await
}

/// [`open_spare`] with `settings` sent before the check.
async fn open_checked(opts: &Opts, server: &str, settings: &str) -> Result<mysql_async::Conn> {
    let mut conn =
        match tokio::time::timeout(CONNECT_TIMEOUT, mysql_async::Conn::new(opts.clone())).await {
            Err(_) => return Err(Error::Timeout),
            Ok(result) => result.map_err(cancel_failed)?,
        };
    let identity = tokio::time::timeout(CONNECT_TIMEOUT, async {
        let identity = server_identity(&mut conn, settings).await;
        if matches!(identity, Err(Error::Query { .. })) {
            // A server that refuses the setting still gets its spare,
            // which then goes stale on the server's own timeout.
            return server_identity(&mut conn, "").await;
        }
        identity
    })
    .await;
    let checked = match identity {
        // Dropped, not disconnected: the goodbye would wait as long.
        Err(_) => return Err(Error::Timeout),
        Ok(Ok((reached, _))) => same_server(server, &reached),
        Ok(Err(error)) => Err(Error::Connect(format!("could not cancel: {error}"))),
    };
    match checked {
        Ok(()) => Ok(conn),
        Err(error) => {
            let _ = conn.disconnect().await;
            Err(error)
        }
    }
}

/// Sent with the spare's server check, in the same message. The server
/// closes a connection idle past `wait_timeout`, which a hosted server
/// often sets to minutes, and a spare is idle between cancels: the cancel
/// after that signs in again, several round trips, and is the late one
/// that `run_session` waits for longest. Raised to MySQL's default of
/// eight hours, never lowered. The session's own setting needs no
/// privilege.
const SPARE_SETTINGS: &str =
    "SET SESSION wait_timeout = CAST(GREATEST(@@session.wait_timeout, 28800) AS UNSIGNED); ";

fn cancel_failed(error: mysql_async::Error) -> Error {
    Error::Connect(format!("could not cancel: {error}"))
}

/// Names the server a connection reached, with its `VERSION()`: by
/// `@@server_uuid`, or by host name and port on MariaDB, which has no
/// server UUID. `before` is statements sent in the same message, which run
/// first ([`session_settings`]); they are sent again with MariaDB's
/// question, so that one of them failing is never taken for the missing
/// UUID.
async fn server_identity(conn: &mut mysql_async::Conn, before: &str) -> Result<(String, String)> {
    let asked = last_row(conn, &format!("{before}SELECT @@server_uuid, VERSION()")).await;
    match asked {
        Ok(Some(row)) => {
            if let (Some(uuid), version) = from_row::<(Option<String>, String)>(row)? {
                return Ok((format!("uuid {uuid}"), version));
            }
        }
        Ok(None) => {}
        Err(mysql_async::Error::Server(error)) if error.code == UNKNOWN_SYSTEM_VARIABLE => {}
        Err(error) => return Err(query_error(error)),
    }
    let found = last_row(
        conn,
        &format!("{before}SELECT @@hostname, @@port, VERSION()"),
    )
    .await
    .map_err(query_error)?;
    Ok(
        match found.map(from_row::<(String, u16, String)>).transpose()? {
            Some((host, port, version)) => (format!("host {host}:{port}"), version),
            None => Default::default(),
        },
    )
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
    fn a_collation_compares_exactly_where_it_tells_case_and_accents_apart() {
        for exact in [
            "utf8mb4_bin",
            "utf8mb4_0900_bin",
            "latin1_bin",
            "latin1_general_cs",
            "utf8mb4_0900_as_cs",
            "utf8mb4_ja_0900_as_cs_ks",
        ] {
            assert!(compares_exactly(exact), "{exact}");
        }
        for loose in [
            "utf8mb4_0900_ai_ci",
            "utf8mb4_general_ci",
            "utf8mb4_0900_as_ci",
            // Czech, which takes either case: `_cs` is its language here.
            "utf8mb4_cs_0900_ai_ci",
            // MariaDB's: they tell case apart and take `resume` for
            // `résumé`.
            "utf8mb4_uca1400_ai_cs",
            "utf8mb4_uca1400_czech_ai_cs",
            // A column that is not text has none.
            "",
        ] {
            assert!(!compares_exactly(loose), "{loose}");
        }
    }

    #[test]
    fn a_failure_is_named_from_its_number_and_what_the_message_quotes() {
        let of = |number: u32, message: &str| {
            let named = named(number, message);
            (named.number, named.constraint, named.columns)
        };
        // The messages are MySQL 8.4's, read from a live server.
        assert_eq!(
            of(
                1062,
                "Duplicate entry '978-1-4028-9462-6' for key 'audit_books.isbn'"
            ),
            (Some(1062), Some("isbn".into()), vec![])
        );
        // A key's name can hold a dot (MySQL 8.4 said this one too): what
        // follows the table's is the key.
        assert_eq!(
            of(1062, "Duplicate entry 'x' for key 'books.isbn.v2'"),
            (Some(1062), Some("isbn.v2".into()), vec![])
        );
        // Before 8.0.19 the key came without its table.
        assert_eq!(
            of(1062, "Duplicate entry 'it''s' for key 'isbn'"),
            (Some(1062), Some("isbn".into()), vec![])
        );
        assert_eq!(
            of(
                1452,
                "Cannot add or update a child row: a foreign key constraint fails \
                 (`tabletist`.`audit_book_covers`, CONSTRAINT `audit_book_covers_ibfk_1` \
                 FOREIGN KEY (`publisher_id`) REFERENCES `audit_publishers` (`id`))"
            ),
            (
                Some(1452),
                Some("audit_book_covers_ibfk_1".into()),
                vec!["publisher_id".to_owned()]
            )
        );
        // ER_ROW_IS_REFERENCED_2, a delete a foreign key refuses: the
        // constraint, and no column, since the one it names is the
        // referencing table's and not the table the save wrote to.
        assert_eq!(
            of(
                1451,
                "Cannot delete or update a parent row: a foreign key constraint fails \
                 (`tabletist`.`shop_book_covers`, CONSTRAINT `shop_book_covers_ibfk_1` \
                 FOREIGN KEY (`publisher_id`) REFERENCES `shop_publishers` (`id`))"
            ),
            (Some(1451), Some("shop_book_covers_ibfk_1".into()), vec![])
        );
        assert_eq!(
            of(1048, "Column 'publisher_id' cannot be null"),
            (Some(1048), None, vec!["publisher_id".to_owned()])
        );
        assert_eq!(
            of(1364, "Field 'publisher_id' doesn't have a default value"),
            (Some(1364), None, vec!["publisher_id".to_owned()])
        );
        assert_eq!(
            of(
                3819,
                "Check constraint 'audit_book_covers_kind_check' is violated."
            ),
            (
                Some(3819),
                Some("audit_book_covers_kind_check".into()),
                vec![]
            )
        );
        // Any other error keeps its number and names nothing.
        assert_eq!(
            of(1146, "Table 'tabletist.nope' doesn't exist"),
            (Some(1146), None, vec![])
        );
    }

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
        // The check is made where a spare opens: a new one, for a session
        // on another server, refuses.
        let other = "uuid 00000000-0000-0000-0000-000000000000";
        let spare = Spare::default();
        assert_eq!(
            cancel(&session.opts, session.id, other, &spare).await,
            Err(Error::Connect(
                "could not cancel: reached a different server".into()
            ))
        );
        assert!(spare.lock().await.is_none());
        assert_eq!(running.await.unwrap().unwrap(), Some(0));
    }

    /// The session's options with `init` run as each connection opens:
    /// the spare's server sets that session's timeout so.
    fn opts_with(session: &Conn, init: &str) -> Opts {
        mysql_async::OptsBuilder::from_opts(session.opts.clone())
            .init(vec![init])
            .into()
    }

    /// A spare stays open past a short `wait_timeout`: a hosted server's
    /// can be minutes, and a cancel after it would sign in again.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_spare_outlives_a_short_wait_timeout() {
        let Some(url) = test_url() else {
            return;
        };
        let session = session(&url).await;
        let opts = opts_with(&session, "SET SESSION wait_timeout = 1");
        let mut spare = open_spare(&opts, &session.server).await.unwrap();
        tokio::time::sleep(Duration::from_millis(2500)).await;
        let timeout: Option<u64> = spare
            .query_first("SELECT @@session.wait_timeout")
            .await
            .unwrap();
        assert_eq!(timeout, Some(28800));
    }

    /// A server whose timeout is longer than the spare's keeps its own.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_spare_keeps_a_longer_wait_timeout() {
        let Some(url) = test_url() else {
            return;
        };
        let session = session(&url).await;
        let opts = opts_with(&session, "SET SESSION wait_timeout = 100000");
        let mut spare = open_spare(&opts, &session.server).await.unwrap();
        let timeout: Option<u64> = spare
            .query_first("SELECT @@session.wait_timeout")
            .await
            .unwrap();
        assert_eq!(timeout, Some(100000));
    }

    /// A server that refuses the spare's setting still gets a spare, with
    /// the server's check made all the same.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_refused_setting_still_opens_a_checked_spare() {
        let Some(url) = test_url() else {
            return;
        };
        let session = session(&url).await;
        let refused = "SET SESSION no_such_setting = 1; ";
        let spare = open_checked(&session.opts, &session.server, refused).await;
        assert!(spare.is_ok(), "{:?}", spare.err());
        let other = "uuid 00000000-0000-0000-0000-000000000000";
        assert_eq!(
            open_checked(&session.opts, other, refused).await.err(),
            Some(Error::Connect(
                "could not cancel: reached a different server".into()
            ))
        );
    }

    /// The thread id of the session's spare, once the connect has opened it.
    async fn spare_id(session: &Conn) -> u32 {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(conn) = session.spare.lock().await.as_ref() {
                return conn.id();
            }
            assert!(Instant::now() < deadline, "the spare never opened");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    /// Cancels go through one spare connection, opened beside the session;
    /// one the server closed is replaced by the next cancel, which still
    /// stops the query.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancels_reuse_one_spare_and_replace_a_closed_one() {
        let Some(url) = test_url() else {
            return;
        };
        let session = std::sync::Arc::new(session(&url).await);
        let first = spare_id(&session).await;
        assert_ne!(first, session.id);
        let (opts, id, server) = (&session.opts, session.id, session.server.as_str());
        // Nothing runs: the KILL does nothing, and the spare stays.
        for _ in 0..3 {
            cancel(opts, id, server, &session.spare).await.unwrap();
        }
        assert_eq!(spare_id(&session).await, first);

        // The server closes it, as `wait_timeout` would.
        let admin_opts =
            mysql_async::Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(admin_opts).await.unwrap();
        admin.query_drop(format!("KILL {first}")).await.unwrap();
        admin.disconnect().await.unwrap();

        let running = {
            let session = std::sync::Arc::clone(&session);
            tokio::spawn(async move {
                let mut conn = session.conn.lock().await;
                // SLEEP answers 1 when a KILL QUERY interrupts it.
                conn.query_first::<i64, _>("SELECT SLEEP(5)").await
            })
        };
        tokio::time::sleep(Duration::from_millis(300)).await;
        cancel(opts, id, server, &session.spare).await.unwrap();
        assert_eq!(running.await.unwrap().unwrap(), Some(1));
        let second = spare_id(&session).await;
        assert_ne!(second, first);
    }

    /// A session closed straight after it connected stops its spare's
    /// open: nothing signs in for a session that is gone.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_closed_session_stops_opening_its_spare() {
        let Some(url) = test_url() else {
            return;
        };
        let session = session(&url).await;
        let spare = std::sync::Arc::clone(&session.spare);
        drop(session);
        // Longer than the open takes here, so it would have finished.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let slot = tokio::time::timeout(Duration::from_secs(1), spare.lock())
            .await
            .expect("the stopped open still holds the lock");
        assert!(slot.is_none());
    }

    /// Spares give up their connections for a session that a full server
    /// refused, all but one a cancel is using; a cancel after that opens
    /// its own again.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spares_give_their_slots_to_a_session() {
        let Some(url) = test_url() else {
            return;
        };
        let idle = session(&url).await;
        let busy = session(&url).await;
        let idle_id = spare_id(&idle).await;
        spare_id(&busy).await;
        let cancelling = busy.spare.lock().await;
        let spares = vec![
            std::sync::Arc::clone(&idle.spare),
            std::sync::Arc::clone(&busy.spare),
        ];
        // Those of another server are left.
        let elsewhere: Opts = mysql_async::OptsBuilder::from_opts(idle.opts.clone())
            .tcp_port(idle.opts.tcp_port().wrapping_add(1))
            .into();
        assert!(!give_up(spares.clone(), &elsewhere).await);
        assert!(idle.spare.lock().await.is_some());
        assert!(give_up(spares.clone(), &idle.opts).await);
        assert!(idle.spare.lock().await.is_none());
        assert!(cancelling.is_some());
        drop(cancelling);
        // The server let the connection go.
        let admin_opts =
            mysql_async::Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(admin_opts).await.unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let left: Option<u64> = admin
                .exec_first(
                    "SELECT id FROM information_schema.PROCESSLIST WHERE id = ?",
                    (idle_id,),
                )
                .await
                .unwrap();
            if left.is_none() {
                break;
            }
            assert!(Instant::now() < deadline, "the spare's connection stayed");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        admin.disconnect().await.unwrap();

        let (opts, id, server) = (&idle.opts, idle.id, idle.server.as_str());
        cancel(opts, id, server, &idle.spare).await.unwrap();
        assert_ne!(spare_id(&idle).await, idle_id);
    }

    /// The list of spares lets go of the sessions that are gone.
    #[test]
    fn the_spares_kept_are_the_sessions_still_there() {
        let gone = Spare::default();
        kept_spares(&gone);
        let weak = std::sync::Arc::downgrade(&gone);
        drop(gone);
        let there = Spare::default();
        kept_spares(&there);
        let spares = SPARES.lock().unwrap();
        assert!(!spares.iter().any(|kept| kept.ptr_eq(&weak)));
        assert!(
            spares
                .iter()
                .any(|kept| kept.ptr_eq(&std::sync::Arc::downgrade(&there)))
        );
    }

    #[test]
    fn a_sign_in_the_limit_refused_is_full() {
        let refused = |code| {
            mysql_async::Error::Server(mysql_async::ServerError {
                code,
                message: String::new(),
                state: "08004".into(),
            })
        };
        assert!(is_full(&refused(TOO_MANY_CONNECTIONS)));
        assert!(is_full(&refused(TOO_MANY_USER_CONNECTIONS)));
        assert!(!is_full(&refused(ACCESS_DENIED)));
    }

    /// Options that reach the test server through a proxy that passes the
    /// sign-in, then holds back every command without closing: a path that
    /// a NAT, or a laptop's sleep, dropped without a word.
    async fn held_back(url: &str) -> Opts {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (spec, _) = ConnectSpec::from_url(url).unwrap();
        let server = (spec.host.clone(), spec.port);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (client, _) = listener.accept().await.unwrap();
            let upstream = tokio::net::TcpStream::connect(server).await.unwrap();
            let (mut from_client, mut to_client) = client.into_split();
            let (mut from_server, mut to_server) = upstream.into_split();
            tokio::spawn(async move {
                tokio::io::copy(&mut from_server, &mut to_client).await.ok();
            });
            loop {
                let mut header = [0; 4];
                if from_client.read_exact(&mut header).await.is_err() {
                    return;
                }
                let length = u32::from_le_bytes([header[0], header[1], header[2], 0]);
                let mut body = vec![0; length as usize];
                if from_client.read_exact(&mut body).await.is_err() {
                    return;
                }
                // Every packet 0 the client sends after the handshake is a
                // command: from the first, nothing goes on.
                if header[3] == 0 {
                    break;
                }
                if to_server.write_all(&header).await.is_err()
                    || to_server.write_all(&body).await.is_err()
                {
                    return;
                }
            }
            tokio::io::copy(&mut from_client, &mut tokio::io::sink())
                .await
                .ok();
            drop(to_server);
        });
        let opts = Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        // Given, so the driver does not ask the server for them, a command
        // of its own, once it signed in.
        mysql_async::OptsBuilder::from_opts(opts)
            .ip_or_hostname("127.0.0.1")
            .tcp_port(port)
            .max_allowed_packet(Some(16 << 20))
            .wait_timeout(Some(28800))
            .into()
    }

    /// A kept spare that no longer answers is given up on in time, and the
    /// cancel goes through a new one: it neither waits for the path to
    /// time out nor holds the lock the next cancel waits on.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_spare_that_does_not_answer_is_replaced_in_time() {
        let Some(url) = test_url() else {
            return;
        };
        let session = std::sync::Arc::new(session(&url).await);
        // The spare's open beside the session is done.
        spare_id(&session).await;
        let held = tokio::time::timeout(
            Duration::from_secs(5),
            mysql_async::Conn::new(held_back(&url).await),
        )
        .await
        .expect("the proxy held back the sign-in")
        .unwrap();
        let held_id = held.id();
        *session.spare.lock().await = Some(held);

        let running = {
            let session = std::sync::Arc::clone(&session);
            tokio::spawn(async move {
                let mut conn = session.conn.lock().await;
                // SLEEP answers 1 when a KILL QUERY interrupts it.
                conn.query_first::<i64, _>("SELECT SLEEP(30)").await
            })
        };
        tokio::time::sleep(Duration::from_millis(300)).await;
        let (opts, id, server) = (&session.opts, session.id, session.server.as_str());
        let started = Instant::now();
        tokio::time::timeout(
            Duration::from_secs(25),
            cancel(opts, id, server, &session.spare),
        )
        .await
        .expect("the cancel waited on a spare that does not answer")
        .unwrap();
        assert!(started.elapsed() < CONNECT_TIMEOUT + Duration::from_secs(5));
        assert_eq!(running.await.unwrap().unwrap(), Some(1));
        assert_ne!(spare_id(&session).await, held_id);
    }

    /// A spare that signs in and then does not answer its server check is
    /// given up on in time: the session's open holds the lock meanwhile.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_spare_whose_check_does_not_answer_is_given_up_in_time() {
        let Some(url) = test_url() else {
            return;
        };
        let opts = held_back(&url).await;
        // It signs in, so it is the check that is given up on.
        let signed_in = tokio::time::timeout(
            Duration::from_secs(5),
            mysql_async::Conn::new(held_back(&url).await),
        )
        .await;
        assert!(signed_in.is_ok_and(|conn| conn.is_ok()));
        let opened = tokio::time::timeout(Duration::from_secs(25), open_spare(&opts, "uuid x"))
            .await
            .expect("the open waited on a check that does not answer");
        assert_eq!(opened.err(), Some(Error::Timeout));
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

    /// A session through a proxy in front of the test server, with a count
    /// of the commands the client sends and waits on: queries, prepares,
    /// executes, resets. Far from the server each is a round trip. A
    /// command is a packet the client starts (sequence 0); closing a
    /// statement, sending its long data and quitting get no answer, and are
    /// not counted.
    pub(super) async fn counted_session(
        url: &str,
        access: Access,
    ) -> (Conn, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        proxied_session(url, access, None).await
    }

    /// [`counted_session`], whose proxy cuts the connection at the first
    /// query whose text starts with `cut`, before the server hears it.
    async fn proxied_session(
        url: &str,
        access: Access,
        cut: Option<&'static str>,
    ) -> (Conn, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        const QUIT: u8 = 0x01;
        const SEND_LONG_DATA: u8 = 0x18;
        const STATEMENT_CLOSE: u8 = 0x19;
        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
        spec.tls = TlsMode::Disable;
        let server = (spec.host.clone(), spec.port);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        spec.host = "127.0.0.1".to_owned();
        spec.port = listener.local_addr().unwrap().port();
        let asked = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&asked);
        tokio::spawn(async move {
            let (client, _) = listener.accept().await.unwrap();
            let upstream = tokio::net::TcpStream::connect(server).await.unwrap();
            let (mut from_client, mut to_client) = client.into_split();
            let (mut from_server, mut to_server) = upstream.into_split();
            let answers = tokio::spawn(async move {
                tokio::io::copy(&mut from_server, &mut to_client).await.ok();
            });
            loop {
                let mut header = [0; 4];
                if from_client.read_exact(&mut header).await.is_err() {
                    return;
                }
                let length = u32::from_le_bytes([header[0], header[1], header[2], 0]);
                let mut body = vec![0; length as usize];
                if from_client.read_exact(&mut body).await.is_err() {
                    return;
                }
                // The handshake's answer is the client's packet 1: every
                // packet 0 the client sends is a command.
                let answered = !matches!(
                    body.first(),
                    Some(&(QUIT | SEND_LONG_DATA | STATEMENT_CLOSE))
                );
                if header[3] == 0 && answered {
                    count.fetch_add(1, Ordering::SeqCst);
                }
                const QUERY: u8 = 0x03;
                let cut_here = cut.is_some_and(|cut| {
                    header[3] == 0
                        && body.first() == Some(&QUERY)
                        && body[1..].starts_with(cut.as_bytes())
                });
                if cut_here {
                    // Both ways: the client reads the end of the stream.
                    answers.abort();
                    return;
                }
                if to_server.write_all(&header).await.is_err()
                    || to_server.write_all(&body).await.is_err()
                {
                    return;
                }
            }
        });
        let conn = Conn::connect(&spec, &secrets, None, access).await.unwrap();
        (conn, asked)
    }

    /// One test at a time creates the `trips` table: creating it twice at
    /// once can fail.
    static TRIPS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// The `trips` table, with a key and three rows.
    async fn trips(url: &str) -> ObjectRef {
        let _turn = TRIPS.lock().await;
        let opts = Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
        admin
            .query_drop(
                "CREATE TABLE IF NOT EXISTS trips (id int PRIMARY KEY, name varchar(20)); \
                 INSERT IGNORE INTO trips VALUES (1, 'a'), (2, 'b'), (3, 'c')",
            )
            .await
            .unwrap();
        let schema: String = admin
            .query_first("SELECT DATABASE()")
            .await
            .unwrap()
            .unwrap();
        ObjectRef {
            schema,
            name: "trips".into(),
        }
    }

    /// Whether the session is inside a transaction, by the server's status
    /// after a query of its own.
    async fn in_transaction(conn: &Conn) -> bool {
        let mut conn = conn.conn.lock().await;
        conn.query_drop("DO 1").await.unwrap();
        status(&conn).contains(StatusFlags::SERVER_STATUS_IN_TRANS)
    }

    /// Each read starts its read-only transaction in one statement and ends
    /// it with one: a page is the key, the start, the statement and the
    /// end, once the driver has its statements prepared.
    #[tokio::test]
    async fn a_read_starts_its_transaction_in_one_statement() {
        use std::sync::atomic::Ordering;
        let Some(url) = test_url() else {
            return;
        };
        let object = trips(&url).await;
        let page = RowQuery::new(object.clone(), 10);
        let lookup = Lookup {
            object: object.clone(),
            key: "id".into(),
            label: Some("name".into()),
            part: crate::LookupPart::Matching(String::new()),
            limit: 10,
        };
        for access in [Access::ReadOnly, Access::Writable] {
            let (conn, asked) = counted_session(&url, access).await;
            // Once to prepare what the driver keeps prepared, then counted.
            for counted in [false, true] {
                let before = asked.load(Ordering::SeqCst);
                let rows = conn.fetch_rows(&page).await.unwrap();
                assert_eq!(rows.rows.len(), 3);
                let fetched = asked.load(Ordering::SeqCst) - before;
                let before = asked.load(Ordering::SeqCst);
                assert_eq!(conn.count_rows(&page).await.unwrap(), 3);
                let counted_rows = asked.load(Ordering::SeqCst) - before;
                let before = asked.load(Ordering::SeqCst);
                assert_eq!(conn.lookup(&lookup).await.unwrap().rows.len(), 3);
                let looked_up = asked.load(Ordering::SeqCst) - before;
                let before = asked.load(Ordering::SeqCst);
                conn.ddl(&object).await.unwrap();
                let shown = asked.load(Ordering::SeqCst) - before;
                if counted {
                    // The DDL: its kind, `SHOW CREATE TABLE`, its triggers.
                    assert_eq!(
                        (fetched, counted_rows, looked_up, shown),
                        (4, 3, 3, 5),
                        "{access:?}"
                    );
                }
            }
            assert!(!in_transaction(&conn).await, "{access:?}");
        }
    }

    /// A read that fails, or is cancelled, still ends its transaction.
    #[tokio::test]
    async fn a_failed_or_cancelled_read_ends_its_transaction() {
        let Some(url) = test_url() else {
            return;
        };
        let object = trips(&url).await;
        let conn = session(&url).await;
        // Fails as it runs, not as it is prepared.
        let mut failing = RowQuery::new(object.clone(), 10);
        failing.raw_where = Some("id = (SELECT 1 UNION SELECT 2)".into());
        assert!(matches!(
            conn.fetch_rows(&failing).await,
            Err(Error::Query { .. })
        ));
        assert!(!in_transaction(&conn).await);
        assert!(matches!(
            conn.count_rows(&failing).await,
            Err(Error::Query { .. })
        ));
        assert!(!in_transaction(&conn).await);
        let mut slow = RowQuery::new(object, 10);
        slow.raw_where = Some("SLEEP(5) = 0".into());
        let cancel = conn.cancel_handle();
        let cancelling = async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            cancel.cancel().await
        };
        let started = Instant::now();
        let (fetched, cancelled) = tokio::join!(conn.fetch_rows(&slow), cancelling);
        cancelled.unwrap();
        assert_eq!(fetched.unwrap_err(), Error::Cancelled);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(!in_transaction(&conn).await);
    }

    /// A read rolls back a transaction left open before it starts its
    /// own: `START TRANSACTION` would commit what that one wrote.
    #[tokio::test]
    async fn a_read_does_not_commit_what_a_transaction_left_open_wrote() {
        let Some(url) = test_url() else {
            return;
        };
        let object = trips(&url).await;
        {
            let _turn = TRIPS.lock().await;
            let opts = Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
            let mut admin = mysql_async::Conn::new(opts).await.unwrap();
            admin
                .query_drop("CREATE TABLE IF NOT EXISTS trips_left_open (id int PRIMARY KEY)")
                .await
                .unwrap();
            admin.query_drop("TRUNCATE trips_left_open").await.unwrap();
        }
        let conn = session(&url).await;
        {
            let mut conn = conn.conn.lock().await;
            conn.query_drop("START TRANSACTION READ WRITE")
                .await
                .unwrap();
            conn.query_drop("INSERT INTO trips_left_open VALUES (1)")
                .await
                .unwrap();
        }
        let page = conn.fetch_rows(&RowQuery::new(object, 10)).await.unwrap();
        assert_eq!(page.rows.len(), 3);
        assert!(!in_transaction(&conn).await);
        let opts = Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
        let left: Option<i64> = admin
            .query_first("SELECT count(*) FROM trips_left_open")
            .await
            .unwrap();
        assert_eq!(left, Some(0));
    }

    /// An export whose statement fails before a row is sent ends its
    /// transaction: nothing is on its way.
    #[tokio::test]
    async fn an_export_that_fails_before_its_rows_ends_its_transaction() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let (to, _from) = tokio::sync::mpsc::channel(4);
        let failed = conn
            .stream_statement("SELECT * FROM no_such_table", 10, &to, &StopFlag::new())
            .await;
        assert!(matches!(failed, Err(Error::Query { .. })), "{failed:?}");
        assert!(!in_transaction(&conn).await);
    }

    /// A read's error wins over its rollback's, unless the rollback failed:
    /// a session left inside the read's transaction is lost.
    #[test]
    fn a_read_whose_rollback_failed_loses_the_session() {
        let refused = || Error::query("refused");
        let lost = || Error::ConnectionLost("gone".into());
        assert_eq!(ended(Ok(1), Ok(())), Ok(1));
        assert_eq!(ended::<i32>(Err(refused()), Ok(())), Err(refused()));
        assert_eq!(
            ended::<i32>(Err(Error::Cancelled), Ok(())),
            Err(Error::Cancelled)
        );
        for outcome in [Ok(1), Err(refused()), Err(Error::Cancelled)] {
            assert_eq!(
                ended(outcome, Err(refused())),
                Err(Error::ConnectionLost(
                    "could not end the read's transaction: refused".into()
                ))
            );
        }
        assert_eq!(ended::<i32>(Err(lost()), Err(refused())), Err(lost()));
    }

    /// A read whose rollback fails loses the session: the proxy cuts the
    /// connection at the read's `ROLLBACK`.
    #[tokio::test]
    async fn a_read_whose_rollback_fails_loses_the_session() {
        let Some(url) = test_url() else {
            return;
        };
        let object = trips(&url).await;
        let (conn, _) = proxied_session(&url, Access::ReadOnly, Some("ROLLBACK")).await;
        let failed = conn.fetch_rows(&RowQuery::new(object, 10)).await;
        assert!(
            matches!(
                &failed,
                Err(Error::ConnectionLost(message))
                    if message.starts_with("could not end the read's transaction")
            ),
            "{failed:?}"
        );
    }

    /// An export whose statement ran has its rows on their way, read or
    /// not: here its receiver is gone before the columns are sent. It ends
    /// as cancelled with its transaction open, for the caller to close the
    /// connection (`Connection::stream_rows`). A read on it all the same
    /// still reads: the driver reads what is on its way first, and the
    /// read's start ends the transaction, which wrote nothing.
    #[tokio::test]
    async fn an_export_whose_statement_ran_leaves_its_end_to_the_caller() {
        let Some(url) = test_url() else {
            return;
        };
        let object = trips(&url).await;
        let conn = session(&url).await;
        let (to, from) = tokio::sync::mpsc::channel(4);
        drop(from);
        let failed = conn
            .stream_statement("SELECT * FROM trips", 10, &to, &StopFlag::new())
            .await;
        assert_eq!(failed, Err(Error::Cancelled));
        let page = conn.fetch_rows(&RowQuery::new(object, 10)).await.unwrap();
        assert_eq!(page.rows.len(), 3);
        assert!(!in_transaction(&conn).await);
    }

    /// A session is prepared, and says which server it reached and its
    /// version, in one message: two with the driver's own question at
    /// connect. A SQL editor run's close prepares it again in one.
    #[tokio::test]
    async fn a_session_is_prepared_and_named_in_one_message() {
        use std::sync::atomic::Ordering;
        let Some(url) = test_url() else {
            return;
        };
        for (access, read_only, run) in [(Access::ReadOnly, 1, 9), (Access::Writable, 0, 10)] {
            let (conn, asked) = counted_session(&url, access).await;
            assert_eq!(asked.load(Ordering::SeqCst), 2, "{access:?}");
            assert!(conn.server.starts_with("uuid "), "{}", conn.server);
            assert!(
                version_name(&conn.version).starts_with("MySQL 8."),
                "{}",
                conn.version
            );
            let settings = || async {
                let mut conn = conn.conn.lock().await;
                let settings: Option<(i64, i64, String)> = conn
                    .query_first(
                        "SELECT @@session.transaction_read_only, @@session.sql_notes, \
                                @@session.collation_connection",
                    )
                    .await
                    .unwrap();
                settings.unwrap()
            };
            let expected = (read_only, 1, "utf8mb4_general_ci".to_owned());
            assert_eq!(settings().await, expected, "{access:?}");
            let before = asked.load(Ordering::SeqCst);
            let outcome = conn
                .script(
                    &["SELECT 1".to_owned()],
                    10,
                    ScriptMode::ReadOnly,
                    &StopFlag::new(),
                )
                .await
                .unwrap();
            assert!(
                matches!(
                    outcome.results[..],
                    [crate::StatementResult {
                        outcome: crate::StatementOutcome::Rows { .. },
                        ..
                    }]
                ),
                "{outcome:?}"
            );
            assert_eq!(asked.load(Ordering::SeqCst) - before, run, "{access:?}");
            assert_eq!(settings().await, expected, "{access:?}");
        }
    }

    /// A message of statements runs them in order and stops at the first
    /// that fails, which the session settings rely on; its answer is the
    /// last row that came.
    #[tokio::test]
    async fn a_message_of_statements_stops_at_the_first_that_fails() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let mut conn = conn.conn.lock().await;
        let row = last_row(
            &mut conn,
            "SET @a = 1; SELECT 5; DO 1; SELECT @a + 1; SET @b = 1",
        )
        .await
        .unwrap();
        assert_eq!(row.map(from_row::<i64>), Some(Ok(2)));
        for failing in [
            "SET @a = 2; SELECT no_such_function(); SET @a = 3",
            "SET @a = 2; SELECT 1; SELECT no_such_function()",
        ] {
            let failed = last_row(&mut conn, failing).await;
            assert!(
                matches!(&failed, Err(mysql_async::Error::Server(error)) if error.code == 1305),
                "{failing}: {failed:?}"
            );
        }
        let a: Option<i64> = conn.query_first("SELECT @a").await.unwrap();
        assert_eq!(a, Some(2));
        let none = last_row(&mut conn, "SET @a = 4; DO 1").await.unwrap();
        assert!(none.is_none());
    }
}
