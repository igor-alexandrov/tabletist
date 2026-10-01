//! MySQL 8.0+. Rows and catalog queries use prepared statements (the binary
//! protocol): typed values, column types, and never more than one statement.
//! Rows are read inside a read-only transaction. A SQL editor script runs
//! its statements the same way, in one read-only transaction, and the
//! session is reset afterwards: MySQL session state is not transactional.
//!
//! MariaDB is meant to work, but the tests run against MySQL only. What is
//! written for MariaDB alone has never met a MariaDB server: the older
//! name of the read-only setting (`READ_ONLY_SETTINGS`), its version for
//! the session reset (`can_reset`), and its marking of a read-only
//! transaction in the server status (`begin`).

use std::borrow::Cow;
use std::path::Path;
use std::time::{Duration, Instant};

use mysql_async::consts::{ColumnFlags, ColumnType, StatusFlags};
use mysql_async::prelude::Queryable;
use mysql_async::{DriverError, IoError, Opts, OptsBuilder, Params, SslOpts, TxOpts};
use mysql_common::named_params::ParsedNamedParams;

use crate::script::{cleanup_failed, retry_cancelled, statement_failed};
use crate::{
    ColumnInfo, ColumnMeta, ConnectSpec, Dialect, Error, ForeignKeyInfo, IndexInfo, MAX_LISTED,
    ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery, ScriptOutcome, Secrets,
    StatementOutcome, StatementResult, StopFlag, Structure, TlsMode, Value, ValueKind,
};

/// MySQL's `binary` character set: bytes, not text.
const BINARY_CHARSET: u16 = 63;
const ACCESS_DENIED: u16 = 1045;
const QUERY_INTERRUPTED: u16 = 1317;
const UNKNOWN_SYSTEM_VARIABLE: u16 = 1193;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Conn {
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
}

impl Conn {
    /// See [`crate::Connection::run_script`]. Statements run through the
    /// prepared protocol, which cannot hold two, and `sql_select_limit`
    /// makes the server stop at `limit + 1` rows. The transaction is
    /// managed by hand, and the session is reset afterwards: what a script
    /// set must not reach table browsing or the next run.
    ///
    /// The future must be awaited to its end: a run that is dropped leaves
    /// the session inside the transaction, with the script's settings. The
    /// backend never drops one; it stops a run through `stop` and the
    /// session's cancel.
    pub async fn run_script(
        &self,
        texts: &[String],
        limit: u32,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let mut conn = self.conn.lock().await;
        // Said before anything runs, not found out by the cleanup, which
        // would close the session after every run.
        if !can_reset(self.version.contains("MariaDB"), conn.server_version()) {
            return Err(Error::Unsupported(
                "the SQL editor needs MySQL 5.7.3 or MariaDB 10.2.4 or later",
            ));
        }
        let mut outcome = ScriptOutcome::default();
        let ended = match open(&mut conn, limit).await {
            // A lost session ends the run here: nothing to close.
            Ok(()) => statements(&mut conn, texts, limit as usize, stop, &mut outcome).await?,
            // A cancel landed on the opening queries: no results.
            Err(Error::Cancelled) => {
                outcome.stopped = true;
                Ended::Unconfirmed
            }
            Err(error) if error.is_connection_lost() => return Err(error),
            // The transaction could not start. The next run would fail the
            // same way, so the session is closed, after an attempt to end
            // what is open.
            Err(error) => Ended::Broken(cannot_start(&error)),
        };
        // From here on a cancel would land on the cleanup: tell the
        // backend to stop repeating its cancel.
        stop.finish();
        close(&mut conn, ended).await?;
        Ok(outcome)
    }

    /// See [`crate::Connection::server_version`]. Asked at connect.
    pub async fn server_version(&self) -> Result<String> {
        Ok(version_name(&self.version))
    }

    /// Connects to the spec's server, or through a tunnel's local port `via`.
    pub async fn connect(spec: &ConnectSpec, secrets: &Secrets, via: Option<u16>) -> Result<Self> {
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
        prepare_session(&mut conn).await?;
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

    pub async fn list_schemas(&self) -> Result<Vec<String>> {
        self.catalog(
            &format!(
                "SELECT schema_name FROM information_schema.schemata \
                 ORDER BY schema_name LIMIT {MAX_LISTED}"
            ),
            Params::Empty,
        )
        .await
    }

    pub async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
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

    pub async fn primary_key(&self, object: &ObjectRef) -> Result<Vec<String>> {
        self.catalog(
            "SELECT column_name FROM information_schema.key_column_usage \
             WHERE table_schema = ? AND table_name = ? AND constraint_name = 'PRIMARY' \
             ORDER BY ordinal_position",
            (&object.schema, &object.name),
        )
        .await
    }

    pub async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        let at = (&object.schema, &object.name);
        let columns: Vec<(String, String, String, Option<String>, String)> = self
            .catalog(
                "SELECT column_name, column_type, is_nullable, column_default, column_comment \
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
            .map(|(name, type_name, nullable, default, comment)| ColumnInfo {
                name,
                type_name,
                nullable: nullable == "YES",
                default,
                comment: (!comment.is_empty()).then_some(comment),
                allowed_values: None,
            })
            .collect();
        let index_rows: Vec<(String, i64, String, String)> = self
            .catalog(
                "SELECT index_name, non_unique, COALESCE(column_name, '<expression>'), index_type \
                 FROM information_schema.statistics \
                 WHERE table_schema = ? AND table_name = ? ORDER BY index_name, seq_in_index",
                at,
            )
            .await?;
        let mut indexes: Vec<IndexInfo> = Vec::new();
        for (name, non_unique, column, method) in index_rows {
            if indexes.last().is_none_or(|index| index.name != name) {
                indexes.push(IndexInfo {
                    primary: name == "PRIMARY",
                    unique: non_unique == 0,
                    method: Some(method.to_lowercase()),
                    columns: Vec::new(),
                    name,
                });
            }
            if let Some(index) = indexes.last_mut() {
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
    pub async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        let key = self.primary_key(&query.object).await?;
        let sql = Dialect::MySql.select_rows(query, &key);
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

    pub async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        let sql = Dialect::MySql.count_rows(query);
        let mut conn = self.conn.lock().await;
        let mut transaction = conn
            .start_transaction(read_only())
            .await
            .map_err(query_error)?;
        let outcome = transaction
            .exec_first::<mysql_async::Row, _, _>(sql.text.as_str(), params(&sql.params))
            .await
            .map_err(query_error);
        let row = finish(transaction, outcome).await?;
        row.map(from_row::<u64>)
            .transpose()?
            .ok_or_else(|| Error::query("the count returned no number"))
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
    let mut result = transaction
        .exec_iter(sql.text.as_str(), params(&sql.params))
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

/// The character set and collation of the driver's handshake. A session
/// reset puts the server's defaults in their place, while the driver goes
/// on sending and reading UTF-8.
const NAMES: &str = "SET NAMES utf8mb4 COLLATE utf8mb4_general_ci";

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
/// again after a SQL editor script's reset, which undoes them. Read-only
/// comes first. After a reset a cancel meant for a statement can land
/// here, so a statement it interrupts runs once more: the session is not
/// left read-write because a `SET` was interrupted.
async fn prepare_session(conn: &mut mysql_async::Conn) -> Result<()> {
    let sql_mode = LEXING_MODES
        .iter()
        .fold("@@SESSION.sql_mode".to_owned(), |mode, name| {
            format!("REPLACE({mode}, '{name}', '')")
        });
    let sql_mode = format!("SET SESSION sql_mode = {sql_mode}");
    // Fixed statements: safe to send through the text protocol.
    for statement in [READ_ONLY, NAMES, sql_mode.as_str()] {
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

/// Where a session stands relative to the read-only transaction a script
/// runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    /// Inside a read-only transaction, in a read-only session.
    Inside,
    /// In no transaction: a statement ended the script's. The server
    /// commits before it runs DDL, also when it then refuses the DDL as
    /// read-only. The session is still read-only.
    Outside,
    /// Not read-only any more: the session, or the transaction it is in.
    Left,
}

/// What a run knows of its session once no more statements run.
#[derive(Debug, PartialEq)]
enum Ended {
    /// Nothing the run saw says the session left read-only. The close asks
    /// the server before it trusts this.
    Unconfirmed,
    /// The check before a statement found the session read-write.
    Left,
    /// A check could not be made, or a transaction could not start: the
    /// session is closed with this error.
    Broken(Error),
}

/// The session's read-only setting. MariaDB before 11.1 knows it only
/// under its older name.
const READ_ONLY_SETTINGS: [&str; 2] = ["transaction_read_only", "tx_read_only"];

/// A transaction that could not start closes the session: the next run
/// would fail the same way.
fn cannot_start(error: &Error) -> Error {
    Error::ConnectionLost(format!(
        "could not start the read-only transaction: {error}"
    ))
}

/// Whether the server knows `COM_RESET_CONNECTION`, which the close needs:
/// MySQL from 5.7.3, MariaDB from 10.2.4. The driver's own rule for
/// `Conn::reset`, asked before anything runs.
fn can_reset(mariadb: bool, version: (u16, u16, u16)) -> bool {
    if mariadb {
        version >= (10, 2, 4)
    } else {
        version >= (5, 7, 3)
    }
}

/// Starts a read-only transaction, and holds the server to saying so: its
/// status must mark the session as inside a transaction, and that one as
/// read-only. Every server that can reset a session does (MySQL since
/// 5.6.5, MariaDB since 10.0); `stands` reads the same marks later, so a
/// server or proxy that drops them cannot run scripts.
async fn begin(conn: &mut mysql_async::Conn) -> Result<()> {
    execute(conn, "START TRANSACTION READ ONLY").await?;
    started(status(conn))
}

/// Whether the status after `START TRANSACTION READ ONLY` says what it
/// must.
fn started(status: StatusFlags) -> Result<()> {
    if !status.contains(StatusFlags::SERVER_STATUS_IN_TRANS) {
        return Err(Error::query("the server did not start a transaction"));
    }
    if !status.contains(StatusFlags::SERVER_STATUS_IN_TRANS_READONLY) {
        return Err(Error::query(
            "the server did not mark the transaction read-only",
        ));
    }
    Ok(())
}

/// Starts the script's transaction and sets its row limit: with
/// `sql_select_limit` the server stops producing rows, and the close's
/// reset puts the default back. `Err` is a cancel that landed on these
/// queries, a lost session, or a transaction that could not start.
async fn open(conn: &mut mysql_async::Conn, limit: u32) -> Result<()> {
    begin(conn).await?;
    // One row more than the limit, to know whether more exist.
    let rows = u64::from(limit) + 1;
    execute(conn, &format!("SET SESSION sql_select_limit = {rows}")).await
}

/// Runs the statements in order, up to the first that fails or is
/// stopped, and says what that showed of the session. `Err` is a lost
/// session.
async fn statements(
    conn: &mut mysql_async::Conn,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
    outcome: &mut ScriptOutcome,
) -> Result<Ended> {
    for text in texts {
        let checked = if stop.is_stopped() {
            Err(Error::Cancelled)
        } else {
            ready(conn).await
        };
        match checked {
            Ok(Standing::Left) => return Ok(Ended::Left),
            Ok(_) => {}
            // A stop between statements, or a cancel that landed on the
            // check: this statement is the cancelled one. The close asks
            // where the session stands.
            Err(Error::Cancelled) => {
                outcome.stopped = true;
                outcome.results.push(StatementResult {
                    elapsed: Duration::ZERO,
                    outcome: StatementOutcome::Cancelled,
                });
                return Ok(Ended::Unconfirmed);
            }
            Err(error) if error.is_connection_lost() => return Err(error),
            // Not known to be read-only: nothing more runs.
            Err(error) => {
                return Ok(Ended::Broken(Error::ConnectionLost(format!(
                    "could not confirm that the session is read-only: {error}"
                ))));
            }
        }
        let started = Instant::now();
        let result = run_statement(conn, text, limit).await?;
        outcome.stopped |= matches!(result, StatementOutcome::Cancelled);
        let last = !matches!(
            result,
            StatementOutcome::Rows { .. } | StatementOutcome::Done { .. }
        );
        outcome.results.push(StatementResult {
            elapsed: started.elapsed(),
            outcome: result,
        });
        if last {
            break;
        }
    }
    Ok(Ended::Unconfirmed)
}

/// The check before every statement: where the session stands, back
/// inside a read-only transaction when a statement ended the script's.
///
/// Outside one, a statement runs in a transaction of its own, read-only
/// like the session, unless a statement the refusal missed set the next
/// transaction's mode (`SET TRANSACTION READ WRITE`), which the session's
/// setting does not show. `START TRANSACTION READ ONLY` overrides that,
/// and inside a transaction the server refuses to change the mode. So
/// every statement runs inside a read-only transaction, whatever the one
/// before it did.
///
/// `Err` is a cancel that landed on the check, a lost session, or a check
/// that could not be made.
async fn ready(conn: &mut mysql_async::Conn) -> Result<Standing> {
    match standing(conn).await? {
        Standing::Outside => {
            begin(conn).await?;
            Ok(Standing::Inside)
        }
        standing => Ok(standing),
    }
}

/// Asks the server where the session stands: its read-only setting, and
/// the transaction status that comes with the answer.
async fn standing(conn: &mut mysql_async::Conn) -> Result<Standing> {
    let read_only = read_only_setting(conn, READ_ONLY_SETTINGS).await?;
    stands(read_only, status(conn))
}

/// The session's read-only setting, asked for under `name`, or under
/// `older` when the server does not know `name`.
async fn read_only_setting(
    conn: &mut mysql_async::Conn,
    [name, older]: [&str; 2],
) -> Result<Option<i64>> {
    let row = match setting(conn, name).await {
        Err(mysql_async::Error::Server(error)) if error.code == UNKNOWN_SYSTEM_VARIABLE => {
            setting(conn, older).await
        }
        answer => answer,
    };
    row.map_err(query_error)?.map(from_row).transpose()
}

/// A session setting's value. With a `LIMIT` of its own: a script can set
/// `sql_select_limit` to 0, and the answer must still come.
async fn setting(
    conn: &mut mysql_async::Conn,
    name: &str,
) -> mysql_async::Result<Option<mysql_async::Row>> {
    conn.query_first(format!("SELECT @@session.{name} LIMIT 1"))
        .await
}

/// Where a session stands, from its read-only setting and the server's
/// status. Only an explicit 0 says the session is read-write. A
/// transaction is the script's kind only when the server marks it
/// read-only, as it marked the script's own (`begin`).
fn stands(read_only: Option<i64>, status: StatusFlags) -> Result<Standing> {
    let Some(read_only) = read_only else {
        return Err(Error::query(
            "the server did not say whether the session is read-only",
        ));
    };
    Ok(if read_only == 0 {
        Standing::Left
    } else if !status.contains(StatusFlags::SERVER_STATUS_IN_TRANS) {
        Standing::Outside
    } else if !status.contains(StatusFlags::SERVER_STATUS_IN_TRANS_READONLY) {
        Standing::Left
    } else {
        Standing::Inside
    })
}

/// Ends a script's run, whatever state it is in: confirms the session is
/// still read-only, rolls back, resets the session and applies the
/// connect-time settings again. A step a cancel interrupted runs once
/// more. `LeftReadOnly` or any step that fails closes the session (they
/// count as a lost connection).
async fn close(conn: &mut mysql_async::Conn, ended: Ended) -> Result<()> {
    let ended = match ended {
        Ended::Unconfirmed => match retry_cancelled!(standing(conn)) {
            Ok(Standing::Left) => Ended::Left,
            Ok(Standing::Inside | Standing::Outside) => Ended::Unconfirmed,
            Err(error) => Ended::Broken(cleanup_failed(&error)),
        },
        known => known,
    };
    // Whatever is open is rolled back and reset as far as that works, also
    // when the session is closed anyway.
    let rolled_back = retry_cancelled!(execute(conn, "ROLLBACK"));
    let reset = retry_cancelled!(reset(conn));
    let prepared = prepare_session(conn).await;
    match ended {
        Ended::Left => Err(Error::LeftReadOnly),
        Ended::Broken(error) => Err(error),
        Ended::Unconfirmed => rolled_back
            .and(reset)
            .and(prepared)
            .map_err(|error| cleanup_failed(&error)),
    }
}

/// Resets the session (`COM_RESET_CONNECTION`): every session setting goes
/// back to the server's default, and user variables, temporary tables,
/// prepared statements and named locks are dropped. The connection id the
/// cancel uses stays. The session is read-write until `prepare_session`
/// runs again.
async fn reset(conn: &mut mysql_async::Conn) -> Result<()> {
    if conn.reset().await.map_err(query_error)? {
        Ok(())
    } else {
        // The server predates the command. `run_script` asks first
        // (`can_reset`); this is for a driver that comes to judge otherwise.
        Err(Error::query("the server cannot reset the session"))
    }
}

/// A statement the server failed, as its outcome. `Err` is a lost session.
fn failed(error: mysql_async::Error) -> Result<StatementOutcome> {
    statement_failed(query_error(error), None)
}

/// The outcome of a statement with a parameter: the SQL editor has no
/// values for parameters. `position` is the parameter's, when known.
fn parameter(spelled: &str, position: Option<usize>) -> StatementOutcome {
    StatementOutcome::Error {
        error: Error::query(format!(
            "the statement has a parameter ({spelled}), which the SQL editor cannot fill in"
        )),
        position,
    }
}

/// The driver's `mysql_common` is the one named here: this fails to
/// compile, rather than read parameters another way than the driver does,
/// when the two versions drift apart.
const _: fn(mysql_common::params::Params) -> Params = |params| params;

/// The outcome of a statement in which the driver reads a `:name`
/// parameter. It sends the server a `?` in its place, and it reads by a
/// lexer of its own, which loses its place at a quote or a comment right
/// after a `-` or a `/` and then takes what is quoted for code
/// (`-':name'`). Such text does not run: only the text the guard read may
/// reach the server, and the driver closes the connection when it has no
/// value for a parameter, also for one the server did not count.
fn driver_parameter(text: &str) -> Option<StatementOutcome> {
    let Ok(parsed) = ParsedNamedParams::parse(text.as_bytes()) else {
        // A `:name` next to a `?`. Which it is cannot be told from here.
        return Some(parameter("? and :name", None));
    };
    let name = parsed.params().first()?;
    let spelled = format!(":{}", String::from_utf8_lossy(name));
    // The name is a slice of `text`: where it starts, its colon before it.
    let colon = (name.as_ptr() as usize)
        .checked_sub(text.as_ptr() as usize)
        .and_then(|start| start.checked_sub(1))
        .filter(|colon| text.is_char_boundary(*colon));
    let position = colon.map(|colon| text[..colon].chars().count() + 1);
    // To the server (and to `sql::tokenize`) it is no parameter when it
    // stands in a string, a quoted name or a comment.
    let misread = colon.is_some_and(|colon| {
        crate::sql::tokenize(Dialect::MySql, text)
            .iter()
            .find(|token| token.range.contains(&colon))
            .is_some_and(|token| {
                matches!(
                    token.kind,
                    crate::sql::TokenKind::String
                        | crate::sql::TokenKind::QuotedIdentifier
                        | crate::sql::TokenKind::Comment
                )
            })
    });
    if !misread {
        return Some(parameter(&spelled, position));
    }
    Some(StatementOutcome::Error {
        error: Error::query(format!(
            "the MySQL driver reads {spelled} here as a parameter; put a space after the - or / \
             that comes before the quote or comment"
        )),
        position,
    })
}

/// Whether the statement's answer carries a row count. The server reports
/// 0 for a statement without one (`SET`, `DO`), which is not "0 rows".
fn counts_rows(text: &str) -> bool {
    crate::sql::words(Dialect::MySql, text)
        .first()
        .is_some_and(|word| matches!(word.as_str(), "INSERT" | "UPDATE" | "DELETE" | "REPLACE"))
}

/// Runs one statement of a script. A statement's failure is its outcome;
/// whether it also ended the transaction, the next check asks. `Err` is a
/// lost session.
async fn run_statement(
    conn: &mut mysql_async::Conn,
    text: &str,
    limit: usize,
) -> Result<StatementOutcome> {
    if let Some(outcome) = driver_parameter(text) {
        return Ok(outcome);
    }
    // Prepare first: the server refuses a second statement hidden in one
    // piece, and what the prepared protocol lacks. A prepare error is the
    // outcome; the text never runs another way.
    let statement = match conn.prep(text).await {
        Ok(statement) => statement,
        Err(error) => return failed(error),
    };
    // The driver closes the connection when a statement runs without
    // values for its parameters, so such a statement does not run.
    if statement.num_params() > 0 {
        return Ok(parameter("?", None));
    }
    let mut result = match conn.exec_iter(&statement, Params::Empty).await {
        Ok(result) => result,
        Err(error) => return failed(error),
    };
    // From the result, not the prepared statement: EXPLAIN prepares
    // without columns.
    let columns = column_metas(result.columns_ref());
    if columns.is_empty() {
        let affected = result.affected_rows();
        if let Err(error) = result.drop_result().await {
            return failed(error);
        }
        return Ok(StatementOutcome::Done {
            affected: counts_rows(text).then_some(affected),
        });
    }
    // sql_select_limit does not bound every statement (SHOW, a SELECT with
    // its own LIMIT): keep limit + 1 rows and read the rest off the wire.
    let mut rows = Vec::new();
    loop {
        match result.next().await {
            Ok(Some(row)) if rows.len() <= limit => rows.push(row_values(row, &columns)),
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(error) => return failed(error),
        }
    }
    // A statement with several result sets shows its first.
    if let Err(error) = result.drop_result().await {
        return failed(error);
    }
    let truncated = rows.len() > limit;
    rows.truncate(limit);
    Ok(StatementOutcome::Rows {
        columns,
        rows,
        truncated,
    })
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
        match Conn::connect(&spec, &secrets, None).await {
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
    fn test_url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_MYSQL_URL")
            .ok()
            .filter(|url| !url.trim().is_empty());
        if url.is_none() {
            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
        }
        url
    }

    /// A fresh read-only session, as the app opens it.
    async fn session(url: &str) -> Conn {
        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
        spec.tls = TlsMode::Disable;
        Conn::connect(&spec, &secrets, None).await.unwrap()
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

    /// The backend spawns a script run, so its future must be `Send`. This
    /// fails to compile, not to run.
    #[test]
    fn a_script_run_can_be_spawned() {
        fn send<T: Send>(_: &T) {}
        let _check = |conn: &Conn, texts: &[String], stop: &StopFlag| {
            send(&conn.run_script(texts, 10, stop));
            send(&conn.server_version());
        };
    }

    #[test]
    fn only_data_changing_statements_report_a_count() {
        for text in [
            "INSERT INTO t VALUES (1)",
            "update t SET n = 1",
            "-- note\nDELETE FROM t",
            "/* note */ REPLACE INTO t VALUES (1)",
        ] {
            assert!(counts_rows(text), "{text}");
        }
        for text in [
            "SET @a = 1",
            "DO 1",
            "CREATE TABLE t (n int)",
            "SELECT 1",
            "",
        ] {
            assert!(!counts_rows(text), "{text}");
        }
    }

    #[test]
    fn text_the_driver_would_rewrite_is_a_parameter_error() {
        // The message, and the 1-based position of the colon.
        let read = |text: &str| match driver_parameter(text) {
            Some(StatementOutcome::Error { error, position }) => {
                Some((error.to_string(), position))
            }
            Some(other) => panic!("{other:?}"),
            None => None,
        };
        let parameter = |spelled: &str, position: usize| {
            let message = format!(
                "the statement has a parameter ({spelled}), which the SQL editor cannot fill in"
            );
            Some((message, Some(position)))
        };
        assert_eq!(read("SELECT :id"), parameter(":id", 8));
        assert_eq!(
            read("SELECT 'é' FROM t WHERE a = :a AND b = :b_2"),
            parameter(":a", 29)
        );
        // Next to a `?` the driver does not say where.
        assert_eq!(
            read("SELECT ?, :id"),
            Some((
                "the statement has a parameter (? and :name), which the SQL editor cannot fill in"
                    .to_owned(),
                None
            ))
        );
        // The driver's lexer loses its place after a `-` or a `/`, and
        // would change a string, a name or a comment. The message says it
        // is the driver's reading, and what to do.
        let misread = |spelled: &str, position: usize| {
            let message = format!(
                "the MySQL driver reads {spelled} here as a parameter; put a space after the - \
                 or / that comes before the quote or comment"
            );
            Some((message, Some(position)))
        };
        assert_eq!(read("SELECT 1 -':abc'"), misread(":abc", 12));
        assert_eq!(read("SELECT 1 /`:abc` FROM t"), misread(":abc", 12));
        assert_eq!(
            read("SELECT '{\"a\":1}' /'{\"a\":true}'"),
            misread(":true", 24)
        );
        assert_eq!(read("SELECT 1 -# :abc\n + 2"), misread(":abc", 13));
        // What it leaves alone runs, a `?` included: the server counts
        // those.
        for text in [
            "SELECT ?",
            "SELECT ':id', \":id\", `:id` FROM t -- :id\n/* :id */ # :id",
            "SELECT @a := 1",
            "SELECT '{\"a\":true}', TIME '12:30:00'",
            "SELECT 1 - ':abc'",
            "",
        ] {
            assert_eq!(read(text), None, "{text}");
        }
    }

    #[test]
    fn a_session_reset_needs_a_recent_server() {
        // The driver's own rule for `Conn::reset`.
        for (mariadb, version, resets) in [
            (false, (5, 6, 51), false),
            (false, (5, 7, 2), false),
            (false, (5, 7, 3), true),
            (false, (5, 7, 44), true),
            (false, (8, 0, 0), true),
            (false, (8, 4, 3), true),
            (false, (9, 1, 0), true),
            (true, (5, 5, 68), false),
            (true, (10, 1, 48), false),
            (true, (10, 2, 3), false),
            (true, (10, 2, 4), true),
            (true, (10, 11, 6), true),
            (true, (11, 4, 2), true),
        ] {
            assert_eq!(can_reset(mariadb, version), resets, "{mariadb} {version:?}");
        }
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

    #[test]
    fn a_session_stands_where_its_setting_and_status_say() {
        let autocommit = StatusFlags::SERVER_STATUS_AUTOCOMMIT;
        let in_transaction = autocommit | StatusFlags::SERVER_STATUS_IN_TRANS;
        let read_only = in_transaction | StatusFlags::SERVER_STATUS_IN_TRANS_READONLY;
        assert_eq!(stands(Some(1), read_only), Ok(Standing::Inside));
        assert_eq!(stands(Some(2), read_only), Ok(Standing::Inside));
        // No transaction: a statement ended it.
        assert_eq!(stands(Some(1), autocommit), Ok(Standing::Outside));
        assert_eq!(stands(Some(1), StatusFlags::empty()), Ok(Standing::Outside));
        // Only an explicit 0 is read-write, whatever the transaction.
        for status in [read_only, in_transaction, autocommit] {
            assert_eq!(stands(Some(0), status), Ok(Standing::Left));
        }
        // A transaction without the read-only mark is not the script's:
        // a missing mark is never taken for a read-only transaction.
        assert_eq!(stands(Some(1), in_transaction), Ok(Standing::Left));
        assert_eq!(
            stands(Some(1), StatusFlags::SERVER_STATUS_IN_TRANS_READONLY),
            Ok(Standing::Outside)
        );
        // No answer is not a yes.
        assert!(matches!(stands(None, read_only), Err(Error::Query { .. })));
    }

    /// A server (or a proxy in front of one) that does not mark the
    /// transaction read-only cannot run scripts: the open fails, and that
    /// closes the session.
    #[test]
    fn a_transaction_the_server_does_not_mark_read_only_fails_the_open() {
        let autocommit = StatusFlags::SERVER_STATUS_AUTOCOMMIT;
        let in_transaction = autocommit | StatusFlags::SERVER_STATUS_IN_TRANS;
        let read_only = in_transaction | StatusFlags::SERVER_STATUS_IN_TRANS_READONLY;
        assert_eq!(started(read_only), Ok(()));
        assert_eq!(
            started(in_transaction),
            Err(Error::query(
                "the server did not mark the transaction read-only"
            ))
        );
        for status in [autocommit, StatusFlags::empty()] {
            assert_eq!(
                started(status),
                Err(Error::query("the server did not start a transaction"))
            );
        }
        let closed = cannot_start(&started(in_transaction).unwrap_err());
        assert!(closed.is_connection_lost());
        assert_eq!(
            closed.to_string(),
            "the connection was lost: could not start the read-only transaction: the server did \
             not mark the transaction read-only"
        );
    }

    /// One test at a time uses the `probe` table: creating it twice at once
    /// can fail, and one test's TRUNCATE would hide another's stray row.
    static PROBE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// A writable connection, outside the adapter.
    async fn admin(url: &str) -> mysql_async::Conn {
        let opts = Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        mysql_async::Conn::new(opts).await.unwrap()
    }

    /// A writable connection with an empty `probe` table, and the table's
    /// lock, held until the test ends.
    async fn probe(url: &str) -> (mysql_async::Conn, tokio::sync::MutexGuard<'static, ()>) {
        let turn = PROBE.lock().await;
        let mut admin = admin(url).await;
        admin
            .query_drop("CREATE TABLE IF NOT EXISTS probe (n int)")
            .await
            .unwrap();
        admin.query_drop("TRUNCATE probe").await.unwrap();
        (admin, turn)
    }

    /// The rows in the `probe` table.
    async fn probe_rows(admin: &mut mysql_async::Conn) -> i64 {
        admin
            .query_first("SELECT count(*) FROM probe")
            .await
            .unwrap()
            .unwrap()
    }

    /// The session settings a script could change and the close puts back.
    const SETTINGS: [&str; 10] = [
        "transaction_read_only",
        "sql_select_limit",
        "sql_mode",
        "time_zone",
        "autocommit",
        "character_set_client",
        "character_set_connection",
        "character_set_results",
        "collation_connection",
        "completion_type",
    ];

    /// Everything of a session that a script could change and the close
    /// puts back, by name: its settings, a user variable, the default
    /// database, and whether a transaction is open.
    async fn settings(conn: &Conn) -> Vec<(&'static str, Option<String>)> {
        let mut conn = conn.conn.lock().await;
        let asked: Vec<String> = SETTINGS
            .iter()
            .map(|name| format!("@@session.{name}"))
            .collect();
        let row: mysql_async::Row = conn
            .query_first(format!(
                "SELECT {}, @tabletist_left_over, DATABASE() LIMIT 1",
                asked.join(", ")
            ))
            .await
            .unwrap()
            .unwrap();
        let open = status(&conn).contains(StatusFlags::SERVER_STATUS_IN_TRANS);
        let values = row.unwrap().into_iter().map(|value| match value {
            mysql_async::Value::NULL => None,
            mysql_async::Value::Bytes(bytes) => Some(String::from_utf8(bytes).unwrap()),
            other => Some(format!("{other:?}")),
        });
        SETTINGS
            .into_iter()
            .chain(["@tabletist_left_over", "DATABASE()"])
            .zip(values)
            .chain([("in a transaction", Some(open.to_string()))])
            .collect()
    }

    /// One of `settings` by its name.
    fn value_of<'a>(settings: &'a [(&'static str, Option<String>)], name: &str) -> Option<&'a str> {
        let (_, value) = settings
            .iter()
            .find(|(setting, _)| *setting == name)
            .unwrap_or_else(|| panic!("no setting {name}"));
        value.as_deref()
    }

    /// Waits until a statement holding `marker` runs on the server.
    async fn runs_on_the_server(admin: &mut mysql_async::Conn, marker: &str) {
        let running = async {
            loop {
                let found: Option<i64> = admin
                    .exec_first(
                        "SELECT count(*) FROM information_schema.processlist \
                         WHERE id <> CONNECTION_ID() AND info LIKE ?",
                        (format!("%{marker}%"),),
                    )
                    .await
                    .unwrap();
                if found.unwrap_or(0) > 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        };
        tokio::time::timeout(Duration::from_secs(10), running)
            .await
            .expect("the statement never ran");
    }

    /// Runs statements straight through the driver, as if the refusal had
    /// missed them.
    async fn past_the_refusal(
        conn: &Conn,
        script: &[&str],
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let texts: Vec<String> = script.iter().map(|&text| text.to_owned()).collect();
        tokio::time::timeout(Duration::from_secs(10), conn.run_script(&texts, 10, stop))
            .await
            .expect("the run hung")
    }

    /// The SQLSTATE a statement failed with.
    fn code(outcome: &StatementOutcome) -> Option<&str> {
        match outcome {
            StatementOutcome::Error {
                error: Error::Query { code, .. },
                ..
            } => code.as_deref(),
            _ => None,
        }
    }

    const INSERT: &str = "INSERT INTO probe VALUES (1)";

    /// Statements the refusal stops long before they get here. Run past
    /// it, they make the session read-write; the statement after them must
    /// not run, and the run must fail even when nothing follows them, so
    /// that the session is closed.
    #[tokio::test]
    async fn a_script_that_left_read_only_runs_nothing_more_and_fails() {
        let Some(url) = test_url() else {
            return;
        };
        let (mut admin, _turn) = probe(&url).await;
        for script in [
            &["COMMIT", "SET SESSION TRANSACTION READ WRITE", INSERT][..],
            &["SET SESSION TRANSACTION READ WRITE", INSERT][..],
            &["SET @@session.transaction_read_only = 0", INSERT][..],
            &["SET @@session.transaction_read_only = 0", "COMMIT", INSERT][..],
            // Last in the script, nothing would run after it.
            &["SELECT 1", "SET SESSION TRANSACTION READ WRITE"][..],
        ] {
            // A session of its own, dropped with the loop's turn: whatever
            // a failing guard let through cannot reach another test.
            let conn = session(&url).await;
            let connected = settings(&conn).await;
            let stop = StopFlag::new();
            let ran = past_the_refusal(&conn, script, &stop).await;
            assert_eq!(ran, Err(Error::LeftReadOnly), "{script:?}");
            assert!(stop.is_finishing(), "{script:?}");
            assert_eq!(probe_rows(&mut admin).await, 0, "{script:?}");
            // The backend closes the session on that error. The close has
            // made it read-only again all the same.
            assert_eq!(settings(&conn).await, connected, "{script:?}");
        }
    }

    /// The refusal keeps these from the driver. Past it, the server still
    /// refuses the write: every statement runs inside a read-only
    /// transaction, also after one that ended the script's, and there the
    /// server lets nothing change the transaction's mode.
    #[tokio::test]
    async fn statements_past_the_refusal_cannot_write() {
        let Some(url) = test_url() else {
            return;
        };
        let (mut admin, _turn) = probe(&url).await;
        // The script, and the SQLSTATE its last result fails with.
        for (script, failure) in [
            (&[INSERT][..], "25006"),
            // Outside the script's transaction the session is read-only.
            (&["COMMIT", INSERT][..], "25006"),
            (&["ROLLBACK", "SELECT 1", INSERT][..], "25006"),
            // This sets the next transaction's mode only, which the
            // session's setting does not show: on its own, the INSERT after
            // it would write. Inside a transaction the server refuses it.
            (
                &["COMMIT", "SET TRANSACTION READ WRITE", INSERT][..],
                "25001",
            ),
            (&["SET TRANSACTION READ WRITE", INSERT][..], "25001"),
            (
                &["COMMIT", "SET @@transaction_read_only = 0", INSERT][..],
                "25001",
            ),
        ] {
            let conn = session(&url).await;
            let connected = settings(&conn).await;
            let stop = StopFlag::new();
            let outcome = past_the_refusal(&conn, script, &stop).await.unwrap();
            let last = outcome.results.len() - 1;
            assert_eq!(
                code(&outcome.results[last].outcome),
                Some(failure),
                "{script:?}: {outcome:?}"
            );
            assert!(
                outcome.results[..last].iter().all(|result| matches!(
                    result.outcome,
                    StatementOutcome::Rows { .. } | StatementOutcome::Done { .. }
                )),
                "{script:?}: {outcome:?}"
            );
            // The write is the statement that failed, or never ran.
            assert!(
                script[last] == INSERT || script[last + 1] == INSERT,
                "{script:?}: {outcome:?}"
            );
            assert_eq!(probe_rows(&mut admin).await, 0, "{script:?}");
            assert_eq!(settings(&conn).await, connected, "{script:?}");
        }
    }

    /// The check before a statement, on its own: a statement that ended
    /// the script's transaction is followed by a new one, and a read-write
    /// transaction is not the script's.
    #[tokio::test]
    async fn the_check_puts_the_session_back_inside_a_read_only_transaction() {
        let Some(url) = test_url() else {
            return;
        };
        let (mut admin, _turn) = probe(&url).await;
        let conn = session(&url).await;
        let connected = settings(&conn).await;
        {
            let mut conn = conn.conn.lock().await;
            assert_eq!(standing(&mut conn).await, Ok(Standing::Outside));
            // The server marks the transaction read-only, or this fails.
            open(&mut conn, 10).await.unwrap();
            assert_eq!(standing(&mut conn).await, Ok(Standing::Inside));
            assert_eq!(ready(&mut conn).await, Ok(Standing::Inside));

            // What a statement that commits would have done, and one that
            // makes the next transaction read-write: the session's setting
            // does not show that, and a statement on its own would write.
            conn.query_drop("COMMIT").await.unwrap();
            conn.query_drop("SET TRANSACTION READ WRITE").await.unwrap();
            assert_eq!(standing(&mut conn).await, Ok(Standing::Outside));
            assert_eq!(ready(&mut conn).await, Ok(Standing::Inside));
            assert_eq!(standing(&mut conn).await, Ok(Standing::Inside));
            let written = conn.query_drop(INSERT).await.map_err(query_error);
            assert!(
                matches!(&written, Err(Error::Query { code: Some(code), .. }) if code == "25006"),
                "{written:?}"
            );
            assert_eq!(probe_rows(&mut admin).await, 0);

            // A transaction of another kind is not the script's.
            conn.query_drop("START TRANSACTION READ WRITE")
                .await
                .unwrap();
            assert_eq!(standing(&mut conn).await, Ok(Standing::Left));
            assert_eq!(ready(&mut conn).await, Ok(Standing::Left));
            assert_eq!(
                close(&mut conn, Ended::Unconfirmed).await,
                Err(Error::LeftReadOnly)
            );
            assert_eq!(standing(&mut conn).await, Ok(Standing::Outside));

            // A script's own sql_select_limit does not blind the check.
            open(&mut conn, 10).await.unwrap();
            conn.query_drop("SET SESSION sql_select_limit = 0")
                .await
                .unwrap();
            assert_eq!(standing(&mut conn).await, Ok(Standing::Inside));
            conn.query_drop("SET SESSION TRANSACTION READ WRITE")
                .await
                .unwrap();
            assert_eq!(standing(&mut conn).await, Ok(Standing::Left));
            assert_eq!(
                close(&mut conn, Ended::Unconfirmed).await,
                Err(Error::LeftReadOnly)
            );
        }
        assert_eq!(settings(&conn).await, connected);
        // The session was not closed, and it works.
        let ran = past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new()).await;
        assert_eq!(ran.unwrap().results.len(), 1);
    }

    /// A stop between statements ends the run without a statement failing,
    /// so the close still asks where the session stands. Here the statement
    /// before the stop made the session read-write.
    #[tokio::test]
    async fn a_stop_after_the_session_left_read_only_still_fails_the_run() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let connected = settings(&conn).await;
        {
            let mut conn = conn.conn.lock().await;
            open(&mut conn, 10).await.unwrap();
            // What the first statement of the script would have done.
            conn.query_drop("SET SESSION TRANSACTION READ WRITE")
                .await
                .unwrap();
            // The stop lands before the second.
            let stop = StopFlag::new();
            stop.stop();
            let mut outcome = ScriptOutcome::default();
            let texts = ["SELECT 1".to_owned()];
            let ended = statements(&mut conn, &texts, 10, &stop, &mut outcome).await;
            assert_eq!(ended, Ok(Ended::Unconfirmed));
            assert_eq!(outcome.results.len(), 1);
            assert_eq!(outcome.results[0].outcome, StatementOutcome::Cancelled);
            assert!(outcome.stopped);
            assert_eq!(
                close(&mut conn, Ended::Unconfirmed).await,
                Err(Error::LeftReadOnly)
            );
        }
        assert_eq!(settings(&conn).await, connected);
    }

    /// After every way a run can end, the session is as it connected:
    /// read-only, in no transaction, with the connect-time sql_mode and
    /// character set, the default sql_select_limit, and nothing a script
    /// set.
    #[tokio::test]
    async fn every_run_leaves_the_session_as_it_connected() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let connected = settings(&conn).await;
        let at_connect = |name: &str| value_of(&connected, name);
        assert_eq!(at_connect("transaction_read_only"), Some("1"));
        assert_eq!(
            at_connect("sql_select_limit"),
            Some(u64::MAX.to_string().as_str())
        );
        assert!(!at_connect("sql_mode").unwrap().contains("ANSI_QUOTES"));
        assert_eq!(at_connect("character_set_client"), Some("utf8mb4"));
        assert_eq!(
            at_connect("collation_connection"),
            Some("utf8mb4_general_ci")
        );
        assert_eq!(at_connect("@tabletist_left_over"), None);
        assert_eq!(at_connect("in a transaction"), Some("false"));
        const SETS: [&str; 6] = [
            "SET time_zone = '+05:00'",
            "SET @tabletist_left_over = 1",
            "SET collation_connection = latin1_swedish_ci",
            "SET character_set_results = latin1",
            "SET sql_select_limit = 3",
            "SET sql_mode = 'ANSI,NO_BACKSLASH_ESCAPES'",
        ];
        // Every statement works.
        let outcome = past_the_refusal(&conn, &SETS, &StopFlag::new())
            .await
            .unwrap();
        assert_eq!(outcome.results.len(), SETS.len());
        assert!(
            outcome
                .results
                .iter()
                .all(|result| result.outcome == StatementOutcome::Done { affected: None })
        );
        assert_eq!(settings(&conn).await, connected);
        // The last statement fails.
        let mut script = SETS.to_vec();
        script.push("SELECT nope");
        let outcome = past_the_refusal(&conn, &script, &StopFlag::new())
            .await
            .unwrap();
        assert_eq!(code(&outcome.results[SETS.len()].outcome), Some("42S22"));
        assert_eq!(settings(&conn).await, connected);
        // A statement ended the transaction (the server commits before it
        // refuses DDL), and the script went on.
        let outcome = past_the_refusal(&conn, &["COMMIT", SETS[0], SETS[1]], &StopFlag::new())
            .await
            .unwrap();
        assert_eq!(outcome.results.len(), 3);
        assert_eq!(settings(&conn).await, connected);
        // Stopped before the first statement.
        let stop = StopFlag::new();
        stop.stop();
        let outcome = past_the_refusal(&conn, &SETS, &stop).await.unwrap();
        assert_eq!(outcome.results.len(), 1);
        assert!(outcome.stopped);
        assert_eq!(settings(&conn).await, connected);
        // No statement at all.
        let outcome = past_the_refusal(&conn, &[], &StopFlag::new())
            .await
            .unwrap();
        assert_eq!(outcome, ScriptOutcome::default());
        assert_eq!(settings(&conn).await, connected);
        // The session left read-only: closed by the backend, and read-only
        // again all the same.
        let mut script = SETS.to_vec();
        script.push("SET SESSION TRANSACTION READ WRITE");
        let ran = past_the_refusal(&conn, &script, &StopFlag::new()).await;
        assert_eq!(ran, Err(Error::LeftReadOnly));
        assert_eq!(settings(&conn).await, connected);
    }

    /// A cancelled statement, and cancels that keep coming while the run
    /// cleans up: the session ends as it connected, or the run fails as a
    /// lost connection and the backend closes it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancelled_run_leaves_the_session_as_it_connected() {
        let Some(url) = test_url() else {
            return;
        };
        let mut admin = admin(&url).await;
        // A sleep of its own length each, for `runs_on_the_server`. With a
        // FROM the interrupted SLEEP is an error; alone it answers 1.
        for (keep_cancelling, sleep) in [(false, "SLEEP(41)"), (true, "SLEEP(42)")] {
            let conn = std::sync::Arc::new(session(&url).await);
            let connected = settings(&conn).await;
            let stop = StopFlag::new();
            let running = {
                let conn = std::sync::Arc::clone(&conn);
                let stop = stop.clone();
                tokio::spawn(async move {
                    let sleeps = format!("SELECT 1 FROM DUAL WHERE {sleep} = 0");
                    let script = ["SET time_zone = '+05:00'", sleeps.as_str()];
                    past_the_refusal(&conn, &script, &stop).await
                })
            };
            runs_on_the_server(&mut admin, sleep).await;
            stop.stop();
            while !running.is_finished() && (keep_cancelling || !stop.is_finishing()) {
                cancel(&conn.opts, conn.id, &conn.server).await.unwrap();
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            match running.await.unwrap() {
                Ok(outcome) => {
                    assert!(outcome.stopped, "{outcome:?}");
                    assert_eq!(
                        outcome.results.last().map(|result| &result.outcome),
                        Some(&StatementOutcome::Cancelled)
                    );
                    assert_eq!(settings(&conn).await, connected);
                }
                Err(error) => {
                    assert!(keep_cancelling, "{error}");
                    assert!(error.is_connection_lost(), "{error}");
                }
            }
        }
    }

    /// The reset keeps the connection id, which the cancel names: a run on
    /// a session that an earlier run reset is cancelled like the first.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancel_reaches_a_session_that_was_reset() {
        let Some(url) = test_url() else {
            return;
        };
        let mut admin = admin(&url).await;
        let conn = std::sync::Arc::new(session(&url).await);
        let connected = settings(&conn).await;
        let server_id = async |conn: &Conn| {
            conn.conn
                .lock()
                .await
                .query_first::<u32, _>("SELECT CONNECTION_ID()")
                .await
                .unwrap()
        };
        assert_eq!(server_id(&conn).await, Some(conn.id));
        for sleep in ["SLEEP(43)", "SLEEP(44)"] {
            // An earlier run, with its reset.
            let earlier = past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new()).await;
            assert_eq!(earlier.unwrap().results.len(), 1);
            assert_eq!(server_id(&conn).await, Some(conn.id));
            assert_eq!(conn.conn.lock().await.id(), conn.id);
            let stop = StopFlag::new();
            let running = {
                let conn = std::sync::Arc::clone(&conn);
                let stop = stop.clone();
                tokio::spawn(async move {
                    let sleeps = format!("SELECT 1 FROM DUAL WHERE {sleep} = 0");
                    past_the_refusal(&conn, &["SELECT 1", sleeps.as_str()], &stop).await
                })
            };
            runs_on_the_server(&mut admin, sleep).await;
            stop.stop();
            while !running.is_finished() && !stop.is_finishing() {
                cancel(&conn.opts, conn.id, &conn.server).await.unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            let outcome = running.await.unwrap().unwrap();
            assert!(outcome.stopped, "{outcome:?}");
            assert_eq!(outcome.results.len(), 2);
            assert_eq!(outcome.results[1].outcome, StatementOutcome::Cancelled);
            assert_eq!(settings(&conn).await, connected);
        }
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
            prepare_session(&mut conn).await.unwrap();
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
        }
    }

    /// MariaDB before 11.1 knows the read-only setting only under its older
    /// name. MySQL 8 knows only the newer one, so asking in the other order
    /// takes the same way round.
    #[tokio::test]
    async fn the_read_only_setting_is_asked_for_under_its_other_name() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let mut conn = conn.conn.lock().await;
        let [name, older] = READ_ONLY_SETTINGS;
        assert_eq!(
            read_only_setting(&mut conn, [name, older]).await,
            Ok(Some(1))
        );
        assert_eq!(
            read_only_setting(&mut conn, [older, name]).await,
            Ok(Some(1))
        );
        assert_eq!(
            read_only_setting(&mut conn, ["tabletist_no_such_setting", name]).await,
            Ok(Some(1))
        );
        // Unknown under both names: the server's error, not a guess.
        let unknown = ["tabletist_no_such_setting", "tabletist_nor_this"];
        assert!(matches!(
            read_only_setting(&mut conn, unknown).await,
            Err(Error::Query { .. })
        ));
        // Any other error is not a reason to ask again.
        assert!(matches!(
            read_only_setting(&mut conn, ["sql_mode + nope", name]).await,
            Err(Error::Query { .. })
        ));
    }

    /// A transaction that cannot start is not a query error to show and
    /// repeat: the session is closed.
    #[tokio::test]
    async fn a_transaction_that_cannot_start_closes_the_session() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        // Inside an XA transaction the server refuses to start another.
        conn.conn
            .lock()
            .await
            .query_drop("XA START 'tabletist_cannot_start'")
            .await
            .unwrap();
        let stop = StopFlag::new();
        let ran = past_the_refusal(&conn, &["SELECT 1"], &stop).await;
        assert!(
            matches!(&ran, Err(Error::ConnectionLost(message))
                if message.starts_with("could not start the read-only transaction")),
            "{ran:?}"
        );
        assert!(stop.is_finishing());
    }

    /// A session the server closed ends the run as a lost connection, on
    /// whichever query finds out.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_lost_session_is_the_runs_error() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = std::sync::Arc::new(session(&url).await);
        let stop = StopFlag::new();
        let running = {
            let conn = std::sync::Arc::clone(&conn);
            let stop = stop.clone();
            tokio::spawn(async move {
                let script = ["SELECT 1", "SELECT 1 FROM DUAL WHERE SLEEP(45) = 0"];
                past_the_refusal(&conn, &script, &stop).await
            })
        };
        let mut admin = admin(&url).await;
        runs_on_the_server(&mut admin, "SLEEP(45)").await;
        admin.query_drop(format!("KILL {}", conn.id)).await.unwrap();
        let ran = running.await.unwrap();
        assert!(
            matches!(&ran, Err(error) if error.is_connection_lost()),
            "{ran:?}"
        );
        // And so does the next run, without hanging.
        let next = past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new()).await;
        assert!(
            matches!(&next, Err(error) if error.is_connection_lost()),
            "{next:?}"
        );
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
