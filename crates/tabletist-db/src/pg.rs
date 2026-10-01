//! PostgreSQL. Catalog queries use the extended protocol with typed results;
//! row queries use the simple-query protocol, which sends every value as
//! text.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tokio_postgres::tls::{MakeTlsConnect, TlsConnect};
use tokio_postgres::{SimpleQueryMessage, Socket};

use crate::script::{cleanup_failed, retry_cancelled, statement_failed};
use crate::{
    ColumnInfo, ColumnMeta, ConnectSpec, Dialect, Error, ForeignKeyInfo, IndexInfo, MAX_LISTED,
    ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery, ScriptOutcome, Secrets,
    StatementOutcome, StatementResult, StopFlag, Structure, Value, ValueKind, value_from_pg_text,
};
use tokio_postgres::error::SqlState;
use tokio_postgres_rustls::MakeRustlsConnect;

pub struct Conn {
    pub(crate) client: tokio::sync::Mutex<tokio_postgres::Client>,
    pub(crate) cancel: tokio_postgres::CancelToken,
    pub(crate) tls: MakeRustlsConnect,
    /// Whether the session runs over TLS: under `prefer` a server that
    /// declines TLS gets plain text.
    pub(crate) encrypted: bool,
}

/// Wraps the rustls connector to note a finished handshake. tokio-postgres
/// calls it only when the server accepts TLS, and does not say afterwards
/// whether `prefer` went on in plain text.
struct Noted {
    inner: MakeRustlsConnect,
    handshake: Arc<AtomicBool>,
}

type RustlsConnect = <MakeRustlsConnect as MakeTlsConnect<Socket>>::TlsConnect;

struct NotedConnect {
    inner: RustlsConnect,
    handshake: Arc<AtomicBool>,
}

impl MakeTlsConnect<Socket> for Noted {
    type Stream = <MakeRustlsConnect as MakeTlsConnect<Socket>>::Stream;
    type TlsConnect = NotedConnect;
    type Error = <MakeRustlsConnect as MakeTlsConnect<Socket>>::Error;

    fn make_tls_connect(&mut self, domain: &str) -> std::result::Result<NotedConnect, Self::Error> {
        Ok(NotedConnect {
            inner: MakeTlsConnect::<Socket>::make_tls_connect(&mut self.inner, domain)?,
            handshake: Arc::clone(&self.handshake),
        })
    }
}

impl TlsConnect<Socket> for NotedConnect {
    type Stream = <RustlsConnect as TlsConnect<Socket>>::Stream;
    type Error = <RustlsConnect as TlsConnect<Socket>>::Error;
    type Future =
        Pin<Box<dyn Future<Output = std::result::Result<Self::Stream, Self::Error>> + Send>>;

    fn connect(self, stream: Socket) -> Self::Future {
        let handshake = self.handshake;
        let connecting = self.inner.connect(stream);
        Box::pin(async move {
            let stream = connecting.await?;
            handshake.store(true, Ordering::Relaxed);
            Ok(stream)
        })
    }
}

/// The error and its causes, joined, for messages ("error connecting to
/// server: Connection refused (os error 111)").
fn describe(error: &(dyn std::error::Error + 'static)) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let cause_text = cause.to_string();
        if !text.contains(&cause_text) {
            text.push_str(": ");
            text.push_str(&cause_text);
        }
        source = cause.source();
    }
    text
}

/// The client config; through a tunnel the socket goes to 127.0.0.1:`via`
/// while TLS still checks `spec.host`.
pub(crate) fn config(
    spec: &ConnectSpec,
    secrets: &Secrets,
    via: Option<u16>,
) -> tokio_postgres::Config {
    let mut config = tokio_postgres::Config::new();
    config
        .host(&spec.host)
        .port(via.unwrap_or(spec.port))
        .user(&spec.user)
        // libpq's default database is the user's name.
        .dbname(if spec.database.is_empty() {
            &spec.user
        } else {
            &spec.database
        })
        .application_name("Tabletist")
        .connect_timeout(Duration::from_secs(10))
        .ssl_mode(crate::tls::ssl_mode(spec.tls));
    if via.is_some() {
        config.hostaddr(std::net::IpAddr::from([127, 0, 0, 1]));
    }
    if let Some(password) = &secrets.password {
        config.password(password);
    }
    config
}

pub(crate) fn connect_error(error: tokio_postgres::Error) -> Error {
    if let Some(db) = error.as_db_error() {
        let code = db.code();
        if *code == SqlState::INVALID_PASSWORD
            || *code == SqlState::INVALID_AUTHORIZATION_SPECIFICATION
        {
            return Error::Auth(db.message().to_owned());
        }
        // The server was reached and said no (no such database, too many
        // connections, starting up): its own code and words.
        return Error::Query {
            code: Some(code.code().to_owned()),
            message: db.message().to_owned(),
            detail: db.detail().map(str::to_owned),
            hint: db.hint().map(str::to_owned),
        };
    }
    if crate::tls::is_tls_error(&error) {
        return Error::Tls(describe(&error));
    }
    let text = describe(&error);
    if text.contains("password missing") {
        return Error::Auth("the server asks for a password".into());
    }
    Error::Connect(text)
}

pub(crate) fn query_error(error: tokio_postgres::Error) -> Error {
    if error.is_closed() {
        return Error::ConnectionLost(describe(&error));
    }
    if let Some(db) = error.as_db_error() {
        if *db.code() == SqlState::QUERY_CANCELED {
            return Error::Cancelled;
        }
        return Error::Query {
            code: Some(db.code().code().to_owned()),
            message: db.message().to_owned(),
            detail: db.detail().map(str::to_owned),
            hint: db.hint().map(str::to_owned),
        };
    }
    Error::ConnectionLost(describe(&error))
}

/// A value from a catalog row as `T`. A server (or proxy) that answers
/// with another type or a NULL is a query error, not a panic: release
/// builds abort on panic.
fn column<'a, T: tokio_postgres::types::FromSql<'a>>(
    row: &'a tokio_postgres::Row,
    index: usize,
) -> Result<T> {
    row.try_get(index).map_err(unexpected)
}

fn unexpected(error: tokio_postgres::Error) -> Error {
    Error::query(format!("unexpected data from the server: {error}"))
}

/// A prepared statement's result columns.
fn column_metas(statement: &tokio_postgres::Statement) -> Vec<ColumnMeta> {
    statement
        .columns()
        .iter()
        .map(|column| ColumnMeta {
            name: column.name().to_owned(),
            type_name: column.type_().name().to_owned(),
            kind: ValueKind::from_pg_type(column.type_().name()),
        })
        .collect()
}

/// A simple-query row's text values, typed by `columns`.
fn row_values(row: &tokio_postgres::SimpleQueryRow, columns: &[ColumnMeta]) -> Result<Vec<Value>> {
    columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            Ok(match row.try_get(index).map_err(unexpected)? {
                None => Value::Null,
                Some(text) => value_from_pg_text(&column.type_name, text),
            })
        })
        .collect()
}

impl Conn {
    /// See [`crate::Connection::run_script`]. The transaction is managed by
    /// hand: `tokio_postgres::Transaction` has no streaming simple query.
    ///
    /// The future must be awaited to its end: a run that is dropped leaves
    /// the session inside the transaction. The backend never drops one; it
    /// stops a run through `stop` and the session's cancel.
    pub async fn run_script(
        &self,
        texts: &[String],
        limit: u32,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let client = self.client.lock().await;
        let mut outcome = ScriptOutcome::default();
        let tx = match open(&client).await {
            // A lost session ends the run here: nothing to close.
            Ok(Tx::Open) => statements(&client, texts, limit as usize, stop, &mut outcome).await?,
            // A cancel landed on the opening queries: no results.
            Ok(tx) => {
                outcome.stopped = true;
                tx
            }
            Err(error) if error.is_connection_lost() => return Err(error),
            // The transaction could not start, say because the session is
            // stuck in a failed one. The next run would fail the same way,
            // so the session is closed, after an attempt to end what is
            // open.
            Err(error) => {
                stop.finish();
                close(&client, Tx::Aborted).await.ok();
                return Err(Error::ConnectionLost(format!(
                    "could not start the read-only transaction: {error}"
                )));
            }
        };
        // From here on a cancel would land on the cleanup: tell the
        // backend to stop repeating its cancel.
        stop.finish();
        close(&client, tx).await?;
        Ok(outcome)
    }

    /// See [`crate::Connection::server_version`].
    pub async fn server_version(&self) -> Result<String> {
        let client = self.client.lock().await;
        let messages = client
            .simple_query("SHOW server_version")
            .await
            .map_err(query_error)?;
        let version = first_text(&messages).unwrap_or_default();
        // "17.2 (Debian 17.2-1.pgdg120+1)" reads as "17.2".
        let version = version.split_whitespace().next().unwrap_or_default();
        Ok(format!("PostgreSQL {version}"))
    }

    /// Connects to the spec's server, or through a tunnel's local port `via`.
    pub async fn connect(spec: &ConnectSpec, secrets: &Secrets, via: Option<u16>) -> Result<Self> {
        if spec.user.trim().is_empty() {
            return Err(Error::InvalidSpec("enter a user name".into()));
        }
        let tls = MakeRustlsConnect::new(crate::tls::client_config(
            spec.tls,
            spec.ca_file.as_deref(),
        )?);
        // The config's own timeout covers only the TCP connect, which is
        // instant to a tunnel's local port: bound the whole startup instead.
        let config = config(spec, secrets, via);
        let handshake = Arc::new(AtomicBool::new(false));
        let connecting = config.connect(Noted {
            inner: tls.clone(),
            handshake: Arc::clone(&handshake),
        });
        let (client, connection) = tokio::time::timeout(Duration::from_secs(10), connecting)
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(connect_error)?;
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                log::info!("PostgreSQL connection ended: {error}");
            }
        });
        client
            .batch_execute(
                "SET default_transaction_read_only = on; SET standard_conforming_strings = on",
            )
            .await
            .map_err(query_error)?;
        let cancel = client.cancel_token();
        Ok(Self {
            client: tokio::sync::Mutex::new(client),
            cancel,
            tls,
            encrypted: handshake.load(Ordering::Relaxed),
        })
    }

    /// A catalog query: fixed SQL, typed results.
    async fn catalog(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Vec<tokio_postgres::Row>> {
        let client = self.client.lock().await;
        client.query(sql, params).await.map_err(query_error)
    }

    pub async fn list_databases(&self) -> Result<Vec<String>> {
        let rows = self
            .catalog(
                "SELECT datname::text FROM pg_database \
                 WHERE datallowconn AND NOT datistemplate ORDER BY datname",
                &[],
            )
            .await?;
        rows.iter().map(|row| column(row, 0)).collect()
    }

    pub async fn list_schemas(&self) -> Result<Vec<String>> {
        let rows = self
            .catalog(
                &format!(
                    "SELECT nspname::text FROM pg_namespace ORDER BY nspname LIMIT {MAX_LISTED}"
                ),
                &[],
            )
            .await?;
        rows.iter().map(|row| column(row, 0)).collect()
    }

    pub async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        let rows = self
            .catalog(
                &format!(
                    "SELECT c.relname::text, c.relkind::text, c.reltuples::float8 \
                     FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
                     WHERE n.nspname = $1 AND c.relkind IN ('r', 'p', 'f', 'v', 'm') \
                     ORDER BY c.relname LIMIT {MAX_LISTED}"
                ),
                &[&schema],
            )
            .await?;
        rows.iter()
            .map(|row| {
                let kind: String = column(row, 1)?;
                let tuples: f64 = column(row, 2)?;
                Ok(ObjectInfo {
                    name: column(row, 0)?,
                    kind: match kind.as_str() {
                        "v" => ObjectKind::View,
                        "m" => ObjectKind::MaterializedView,
                        _ => ObjectKind::Table,
                    },
                    // -1 means never analyzed (PostgreSQL 14+).
                    estimated_rows: (tuples >= 0.0).then_some(tuples as u64),
                })
            })
            .collect()
    }

    async fn relation(&self, object: &ObjectRef) -> Result<u32> {
        let rows = self
            .catalog(
                "SELECT c.oid FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
                 WHERE n.nspname = $1 AND c.relname = $2",
                &[&object.schema, &object.name],
            )
            .await?;
        rows.first()
            .map(|row| column(row, 0))
            .ok_or_else(|| Error::query(format!("no such table or view: {}", object.name)))?
    }

    pub async fn primary_key(&self, object: &ObjectRef) -> Result<Vec<String>> {
        let rows = self
            .catalog(
                "SELECT a.attname::text \
                 FROM pg_index i \
                 JOIN pg_class c ON c.oid = i.indrelid \
                 JOIN pg_namespace n ON n.oid = c.relnamespace \
                 CROSS JOIN LATERAL unnest(i.indkey) WITH ORDINALITY AS k(attnum, ord) \
                 JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = k.attnum \
                 WHERE n.nspname = $1 AND c.relname = $2 AND i.indisprimary \
                 ORDER BY k.ord",
                &[&object.schema, &object.name],
            )
            .await?;
        rows.iter().map(|row| column(row, 0)).collect()
    }

    pub async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        let oid = self.relation(object).await?;
        let columns = self
            .catalog(
                // An enum's labels in their sort order; a text column's
                // single-column CHECK constraints, by name.
                "SELECT a.attname::text, format_type(a.atttypid, a.atttypmod), NOT a.attnotnull, \
                        pg_get_expr(d.adbin, d.adrelid), col_description(a.attrelid, a.attnum), \
                        CASE WHEN t.typtype = 'e' THEN \
                            ARRAY(SELECT e.enumlabel::text FROM pg_enum e \
                                  WHERE e.enumtypid = t.oid ORDER BY e.enumsortorder) END, \
                        CASE WHEN t.oid IN ('text'::regtype, 'varchar'::regtype) THEN \
                            ARRAY(SELECT pg_get_expr(c.conbin, c.conrelid) FROM pg_constraint c \
                                  WHERE c.conrelid = a.attrelid AND c.contype = 'c' \
                                    AND c.conkey = ARRAY[a.attnum] ORDER BY c.conname) END \
                 FROM pg_attribute a \
                 JOIN pg_type t ON t.oid = a.atttypid \
                 LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
                 WHERE a.attrelid = $1 AND a.attnum > 0 AND NOT a.attisdropped \
                 ORDER BY a.attnum",
                &[&oid],
            )
            .await?
            .iter()
            .map(|row| {
                let name: String = column(row, 0)?;
                let labels: Option<Vec<String>> = column(row, 5)?;
                let checks: Option<Vec<String>> = column(row, 6)?;
                let allowed_values = labels.or_else(|| {
                    checks?
                        .iter()
                        .find_map(|check| crate::check::allowed_values(check, &name))
                });
                Ok(ColumnInfo {
                    name,
                    type_name: column(row, 1)?,
                    nullable: column(row, 2)?,
                    default: column(row, 3)?,
                    comment: column(row, 4)?,
                    allowed_values,
                })
            })
            .collect::<Result<_>>()?;
        let indexes = self
            .catalog(
                "SELECT ic.relname::text, i.indisunique, i.indisprimary, am.amname::text, \
                        ARRAY(SELECT pg_get_indexdef(i.indexrelid, k, true) \
                              FROM generate_series(1, i.indnkeyatts::int) AS k ORDER BY k) \
                 FROM pg_index i \
                 JOIN pg_class ic ON ic.oid = i.indexrelid \
                 JOIN pg_am am ON am.oid = ic.relam \
                 WHERE i.indrelid = $1 ORDER BY ic.relname",
                &[&oid],
            )
            .await?
            .iter()
            .map(|row| {
                Ok(IndexInfo {
                    name: column(row, 0)?,
                    unique: column(row, 1)?,
                    primary: column(row, 2)?,
                    method: Some(column(row, 3)?),
                    columns: column(row, 4)?,
                })
            })
            .collect::<Result<_>>()?;
        let foreign_keys = self
            .catalog(
                "SELECT con.conname::text, \
                        ARRAY(SELECT a.attname::text FROM unnest(con.conkey) WITH ORDINALITY AS k(attnum, ord) \
                              JOIN pg_attribute a ON a.attrelid = con.conrelid AND a.attnum = k.attnum \
                              ORDER BY k.ord), \
                        rn.nspname::text, rc.relname::text, \
                        ARRAY(SELECT a.attname::text FROM unnest(con.confkey) WITH ORDINALITY AS k(attnum, ord) \
                              JOIN pg_attribute a ON a.attrelid = con.confrelid AND a.attnum = k.attnum \
                              ORDER BY k.ord), \
                        con.confupdtype::text, con.confdeltype::text \
                 FROM pg_constraint con \
                 JOIN pg_class rc ON rc.oid = con.confrelid \
                 JOIN pg_namespace rn ON rn.oid = rc.relnamespace \
                 WHERE con.conrelid = $1 AND con.contype = 'f' ORDER BY con.conname",
                &[&oid],
            )
            .await?
            .iter()
            .map(|row| {
                let on_update: String = column(row, 5)?;
                let on_delete: String = column(row, 6)?;
                Ok(ForeignKeyInfo {
                    name: Some(column(row, 0)?),
                    columns: column(row, 1)?,
                    ref_schema: column(row, 2)?,
                    ref_table: column(row, 3)?,
                    ref_columns: column(row, 4)?,
                    on_update: action(&on_update).into(),
                    on_delete: action(&on_delete).into(),
                })
            })
            .collect::<Result<_>>()?;
        Ok(Structure {
            columns,
            primary_key: self.primary_key(object).await?,
            indexes,
            foreign_keys,
        })
    }

    /// One page. The SQL is prepared first, which refuses a second statement
    /// and yields the column types; it then runs through the simple-query
    /// protocol (every value as text) inside a read-only transaction.
    pub async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        let key = self.primary_key(&query.object).await?;
        let sql = Dialect::Postgres.select_rows(query, &key);
        let limit = query.limit as usize;
        let mut client = self.client.lock().await;
        let started = Instant::now();
        let transaction = client
            .build_transaction()
            .read_only(true)
            .start()
            .await
            .map_err(query_error)?;
        let statement = transaction.prepare(&sql.text).await.map_err(query_error)?;
        let columns = column_metas(&statement);
        let messages = transaction
            .simple_query(&sql.text)
            .await
            .map_err(query_error)?;
        transaction.rollback().await.map_err(query_error)?;
        let mut rows = Vec::new();
        for message in messages {
            if let SimpleQueryMessage::Row(row) = message {
                if rows.len() > limit {
                    break;
                }
                rows.push(row_values(&row, &columns)?);
            }
        }
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
        let sql = Dialect::Postgres.count_rows(query);
        let mut client = self.client.lock().await;
        let transaction = client
            .build_transaction()
            .read_only(true)
            .start()
            .await
            .map_err(query_error)?;
        // Prepare first: refuses a second statement before anything runs.
        transaction.prepare(&sql.text).await.map_err(query_error)?;
        let messages = transaction
            .simple_query(&sql.text)
            .await
            .map_err(query_error)?;
        transaction.rollback().await.map_err(query_error)?;
        messages
            .iter()
            .find_map(|message| match message {
                SimpleQueryMessage::Row(row) => row
                    .try_get(0)
                    .ok()
                    .flatten()
                    .and_then(|count| count.parse().ok()),
                _ => None,
            })
            .ok_or_else(|| Error::query("the count returned no number"))
    }
}

/// What a SQL editor row statement runs as: `DECLARE` with this prefix,
/// then `FETCH` and `CLOSE`.
const CURSOR_PREFIX: &str = "DECLARE tabletist_sql NO SCROLL CURSOR FOR ";

/// The first row's first value.
fn first_text(messages: &[SimpleQueryMessage]) -> Option<String> {
    messages.iter().find_map(|message| match message {
        SimpleQueryMessage::Row(row) => row.get(0).map(str::to_owned),
        _ => None,
    })
}

/// Where a script's session stands relative to its read-only transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tx {
    /// Inside it, and nothing the run saw says it failed. The close asks
    /// the server before it trusts this.
    Open,
    /// Inside it, but it failed (an error or a cancel): it cannot write,
    /// and `ROLLBACK` ends it. A failed `PREPARE TRANSACTION` is the one
    /// failure that leaves the block; the server has then rolled it back
    /// itself and nothing runs after it, so `ROLLBACK` finds nothing to do.
    Aborted,
    /// Not where the script started: outside the transaction, or in one
    /// that is no longer read-only.
    Left,
}

/// Starts the script's transaction: `BEGIN READ ONLY`, the snapshot, and
/// the guard's savepoint (see `in_transaction`). PostgreSQL refuses to make
/// a transaction read-write once it has a snapshot, whatever a statement
/// tries. `Aborted` when a cancel landed on these queries; an `Err` that is
/// not a lost session means the transaction could not start.
async fn open(client: &tokio_postgres::Client) -> Result<Tx> {
    for step in ["BEGIN READ ONLY", "SELECT 1", "SAVEPOINT tabletist_guard"] {
        match client.batch_execute(step).await.map_err(query_error) {
            Ok(()) => {}
            Err(Error::Cancelled) => return Ok(Tx::Aborted),
            Err(error) => return Err(error),
        }
    }
    Ok(Tx::Open)
}

/// Runs the statements in order, up to the first that fails or is
/// stopped, and says where that leaves the transaction. `Err` is a lost
/// session.
async fn statements(
    client: &tokio_postgres::Client,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
    outcome: &mut ScriptOutcome,
) -> Result<Tx> {
    for text in texts {
        let ready = if stop.is_stopped() {
            Err(Error::Cancelled)
        } else {
            in_transaction(client).await
        };
        let failure = match ready {
            Ok(Tx::Open) => None,
            Ok(Tx::Left) => return Ok(Tx::Left),
            // Not after a statement that succeeded; said rather than a
            // run that ends without a word. Whose failed transaction it
            // is, the close asks.
            Ok(Tx::Aborted) => Some((
                StatementOutcome::Error {
                    error: Error::query("the transaction failed before this statement ran"),
                    position: None,
                },
                Tx::Open,
            )),
            // A stop between statements, or a cancel that landed on the
            // check: this statement is the cancelled one. A cancel fails
            // the transaction block the session is in, if it is in one;
            // whether that is still the script's, the close asks.
            Err(Error::Cancelled) => {
                outcome.stopped = true;
                Some((StatementOutcome::Cancelled, Tx::Open))
            }
            Err(error) => return Err(error),
        };
        if let Some((result, tx)) = failure {
            outcome.results.push(StatementResult {
                elapsed: Duration::ZERO,
                outcome: result,
            });
            return Ok(tx);
        }
        let started = Instant::now();
        let (result, tx) = run_statement(client, text, limit, stop).await?;
        outcome.stopped |= result == StatementOutcome::Cancelled;
        let last = !matches!(
            result,
            StatementOutcome::Rows { .. } | StatementOutcome::Done { .. }
        );
        outcome.results.push(StatementResult {
            elapsed: started.elapsed(),
            outcome: result,
        });
        if last {
            return Ok(tx);
        }
    }
    Ok(Tx::Open)
}

/// Ends the script's transaction, whatever state it is in: confirms an
/// open one is the script's own and still read-only, rolls back, and
/// releases the session's advisory locks. `LeftReadOnly`, a failed check
/// or a failed rollback closes the session (they count as a lost
/// connection).
async fn close(client: &tokio_postgres::Client, tx: Tx) -> Result<()> {
    let tx = match tx {
        Tx::Open => confirm(client).await,
        known => Ok(known),
    };
    // Whatever is open is rolled back as far as that works, also when the
    // session is closed anyway.
    let rolled_back = retry_cancelled!(rollback(client));
    match (tx, rolled_back) {
        (Ok(Tx::Left), _) => Err(Error::LeftReadOnly),
        (Err(error), _) | (_, Err(error)) => Err(cleanup_failed(&error)),
        (Ok(_), Ok(())) => unlocked(retry_cancelled!(unlock(client))),
    }
}

/// What a failed release of the advisory locks means for a run whose
/// transaction did end: nothing, unless the session is lost. The outcome
/// stands and the session stays; a lock the script took may stay with it.
fn unlocked(released: Result<()>) -> Result<()> {
    match released {
        Err(error) if error.is_connection_lost() => Err(error),
        // Cancelled twice: given up quietly.
        Ok(()) | Err(Error::Cancelled) => Ok(()),
        Err(error) => {
            log::warn!("could not release the advisory locks after a script: {error}");
            Ok(())
        }
    }
}

/// Where a transaction the run believes open really stands, asked after
/// the last statement: a statement in last position may have ended it, and
/// what a `COMMIT` kept (a setting) must not reach later browsing.
async fn confirm(client: &tokio_postgres::Client) -> Result<Tx> {
    match in_transaction(client).await {
        Ok(Tx::Open) => {}
        Ok(Tx::Left) => return Ok(Tx::Left),
        // A failed block, or a cancel on the check, which fails the block
        // if there is one: the check cannot say whose it is.
        Ok(Tx::Aborted) | Err(Error::Cancelled) => return retry_cancelled!(own_failed(client)),
        Err(error) => return Err(error),
    }
    match still_read_only(client).await {
        Ok(true) => Ok(Tx::Open),
        Ok(false) => Ok(Tx::Left),
        // The session is inside the script's transaction (the check
        // above), and the cancel aborted it: it cannot write.
        Err(Error::Cancelled) => Ok(Tx::Aborted),
        Err(error) => Err(error),
    }
}

/// Whether a failed transaction block is the script's own (`Aborted`) or
/// not (`Left`). A failed block answers every query with 25P02, the
/// script's own and a chained one alike, except `ROLLBACK TO SAVEPOINT`,
/// which works when the block has the savepoint: only the script's does.
///
/// A cancel that lands inside the guard's swap, after its `RELEASE` and
/// before its `SAVEPOINT`, leaves the script's own transaction without the
/// savepoint. That reads as `Left` and closes the session: a safe failure
/// in a narrow window.
async fn own_failed(client: &tokio_postgres::Client) -> Result<Tx> {
    match client
        .batch_execute("ROLLBACK TO SAVEPOINT tabletist_guard")
        .await
        .map_err(query_error)
    {
        Ok(()) => Ok(Tx::Aborted),
        Err(error) if error == Error::Cancelled || error.is_connection_lost() => Err(error),
        Err(_) => Ok(Tx::Left),
    }
}

/// Where the session stands, asked before every statement and once after
/// the last. A statement the refusal missed could end the transaction
/// (`COMMIT`, `ROLLBACK`); the next one would then run on its own, where
/// the snapshot no longer keeps it read-only. So `open` sets a savepoint,
/// and this swaps it for a new one. `RELEASE` fails:
///
/// - outside a transaction block (25P01), before anything runs there;
/// - in another transaction, which does not have the savepoint (3B001):
///   `COMMIT AND CHAIN` starts one that is a block and read-only, but has
///   no snapshot yet and could be made read-write;
/// - in the script's own transaction once it failed (25P02).
///
/// Exactly one savepoint is alive at a time, however long the script. Every
/// statement runs inside its subtransaction, which PostgreSQL also refuses
/// to make read-write. The script cannot name the savepoint: the refusal
/// stops `SAVEPOINT`, `RELEASE` and `ROLLBACK`.
///
/// `Err` is a cancel that landed on the check, or a lost session.
async fn in_transaction(client: &tokio_postgres::Client) -> Result<Tx> {
    let Err(error) = client
        .batch_execute("RELEASE tabletist_guard; SAVEPOINT tabletist_guard")
        .await
    else {
        return Ok(Tx::Open);
    };
    if error.code() == Some(&SqlState::IN_FAILED_SQL_TRANSACTION) {
        return Ok(Tx::Aborted);
    }
    match query_error(error) {
        error if error == Error::Cancelled || error.is_connection_lost() => Err(error),
        // Outside the script's transaction. Any other answer is not where
        // the script started either.
        _ => Ok(Tx::Left),
    }
}

/// Ends the script's transaction. Outside one (a cancel can land on
/// `BEGIN`) the server only warns, so this is safe to run on every path.
async fn rollback(client: &tokio_postgres::Client) -> Result<()> {
    client.batch_execute("ROLLBACK").await.map_err(query_error)
}

/// Releases the session's advisory locks: a script can take one
/// (`pg_advisory_lock`), and they outlive its transaction.
async fn unlock(client: &tokio_postgres::Client) -> Result<()> {
    client
        .batch_execute("SELECT pg_advisory_unlock_all()")
        .await
        .map_err(query_error)
}

/// Whether the script's transaction is still read-only: only an explicit
/// `off` says it is not.
async fn still_read_only(client: &tokio_postgres::Client) -> Result<bool> {
    let messages = client
        .simple_query("SHOW transaction_read_only")
        .await
        .map_err(query_error)?;
    Ok(first_text(&messages).as_deref() != Some("off"))
}

/// A statement the server failed, as its outcome: the transaction is
/// aborted. `offset` is how many characters of wrapping precede the user's
/// text in what the server saw; `None` drops the position (it points into
/// other text). `Err` is a lost session.
fn failed(error: tokio_postgres::Error, offset: Option<usize>) -> Result<(StatementOutcome, Tx)> {
    use tokio_postgres::error::ErrorPosition;
    let position = error
        .as_db_error()
        .and_then(|db| match db.position()? {
            ErrorPosition::Original(position) => Some(*position as usize),
            ErrorPosition::Internal { .. } => None,
        })
        .zip(offset)
        .and_then(|(position, offset)| position.checked_sub(offset))
        .filter(|position| *position > 0);
    Ok((statement_failed(query_error(error), position)?, Tx::Aborted))
}

/// Whether a statement that returns rows can run as a cursor: its first
/// token is `SELECT`, `VALUES`, `TABLE`, `WITH` or `(`.
fn cursor_statement(text: &str) -> bool {
    let tokens = crate::sql::tokenize(Dialect::Postgres, text);
    let first = tokens.iter().find(|token| {
        !matches!(
            token.kind,
            crate::sql::TokenKind::Whitespace | crate::sql::TokenKind::Comment
        )
    });
    first.is_some_and(|token| {
        let word = text[token.range.clone()].to_ascii_uppercase();
        matches!(word.as_str(), "SELECT" | "VALUES" | "TABLE" | "WITH" | "(")
    })
}

/// Whether the statement's command tag carries a row count. tokio-postgres
/// reports 0 for a tag without one (`SET`, `DO`), which is not "0 rows".
fn counts_rows(text: &str) -> bool {
    crate::sql::words(Dialect::Postgres, text)
        .first()
        .is_some_and(|word| matches!(word.as_str(), "INSERT" | "UPDATE" | "DELETE" | "MERGE"))
}

/// How many rows to ask a cursor for: one more than the limit, to know
/// whether more exist, and never more than `FETCH` can count.
fn fetch_count(limit: usize) -> usize {
    limit.saturating_add(1).min(i32::MAX as usize)
}

/// A row statement's outcome from the rows kept: at most `limit` of them,
/// and `truncated` when one more was read.
fn rows_outcome(
    columns: Vec<ColumnMeta>,
    kept: &[tokio_postgres::SimpleQueryRow],
    limit: usize,
) -> StatementOutcome {
    let rows: Result<Vec<_>> = kept.iter().map(|row| row_values(row, &columns)).collect();
    match rows {
        Ok(mut rows) => {
            let truncated = rows.len() > limit;
            rows.truncate(limit);
            StatementOutcome::Rows {
                columns,
                rows,
                truncated,
            }
        }
        Err(error) => StatementOutcome::Error {
            error,
            position: None,
        },
    }
}

/// Runs one statement of a script inside its transaction, and says where
/// that leaves the transaction. `Err` is a lost session.
async fn run_statement(
    client: &tokio_postgres::Client,
    text: &str,
    limit: usize,
    stop: &StopFlag,
) -> Result<(StatementOutcome, Tx)> {
    // The driver cannot put a NUL in a message, and fails in a way that
    // reads as a lost session. Nothing is sent: the transaction stays open.
    if let Some(index) = text.chars().position(|character| character == '\0') {
        let error = Error::query("SQL text cannot contain a NUL character");
        return Ok((
            StatementOutcome::Error {
                error,
                position: Some(index + 1),
            },
            Tx::Open,
        ));
    }
    // Prepare first: it yields the column types and refuses a second
    // statement hidden in one piece. A prepare error is the outcome; the
    // text never runs another way. Only the columns are kept: the prepared
    // statement itself is closed here.
    let columns = match client.prepare(text).await {
        Ok(prepared) => column_metas(&prepared),
        Err(error) => return failed(error, Some(0)),
    };
    if columns.is_empty() {
        return match client.simple_query(text).await {
            Ok(messages) => {
                let affected = messages
                    .iter()
                    .find_map(|message| match message {
                        SimpleQueryMessage::CommandComplete(count) => Some(*count),
                        _ => None,
                    })
                    .filter(|_| counts_rows(text));
                Ok((StatementOutcome::Done { affected }, Tx::Open))
            }
            Err(error) => failed(error, Some(0)),
        };
    }
    let mut kept = Vec::new();
    if cursor_statement(text) {
        // Its own call, and a newline, so a trailing -- comment ends there.
        let declare = format!("{CURSOR_PREFIX}{text}\n");
        if let Err(error) = client.batch_execute(&declare).await {
            return failed(error, Some(CURSOR_PREFIX.chars().count()));
        }
        // Nothing failed: the transaction is still usable.
        if stop.is_stopped() {
            return Ok((StatementOutcome::Cancelled, Tx::Open));
        }
        let fetch = format!("FETCH {} FROM tabletist_sql", fetch_count(limit));
        let messages = match client.simple_query(&fetch).await {
            Ok(messages) => messages,
            Err(error) => return failed(error, None),
        };
        if let Err(error) = client.batch_execute("CLOSE tabletist_sql").await {
            return failed(error, None);
        }
        kept.extend(messages.into_iter().filter_map(|message| match message {
            SimpleQueryMessage::Row(row) => Some(row),
            _ => None,
        }));
    } else {
        // SHOW, EXPLAIN and the like: stream, keep limit + 1, drop the rest.
        use futures_util::StreamExt;
        let stream = match client.simple_query_raw(text).await {
            Ok(stream) => stream,
            Err(error) => return failed(error, Some(0)),
        };
        let mut stream = std::pin::pin!(stream);
        while let Some(message) = stream.next().await {
            match message {
                Ok(SimpleQueryMessage::Row(row)) if kept.len() <= limit => kept.push(row),
                Ok(_) => {}
                Err(error) => return failed(error, Some(0)),
            }
        }
    }
    Ok((rows_outcome(columns, &kept, limit), Tx::Open))
}

/// pg_constraint's one-letter referential actions.
fn action(code: &str) -> &'static str {
    match code {
        "r" => "RESTRICT",
        "c" => "CASCADE",
        "n" => "SET NULL",
        "d" => "SET DEFAULT",
        _ => "NO ACTION",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tunnelled_postgres_config_keeps_the_host_name_for_tls() {
        let (spec, secrets) =
            ConnectSpec::from_url("postgres://me@db.example.com:5432/app").unwrap();
        let tunnelled = config(&spec, &secrets, Some(40001));
        assert_eq!(
            tunnelled.get_hosts(),
            &[tokio_postgres::config::Host::Tcp("db.example.com".into())]
        );
        assert_eq!(
            tunnelled.get_hostaddrs(),
            &[std::net::IpAddr::from([127, 0, 0, 1])]
        );
        assert_eq!(tunnelled.get_ports(), &[40001]);
        let direct = config(&spec, &secrets, None);
        assert!(direct.get_hostaddrs().is_empty());
        assert_eq!(direct.get_ports(), &[5432]);
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
    fn row_statements_run_as_cursors_by_their_first_token() {
        for text in [
            "SELECT 1",
            "select 1",
            "-- note\n  VALUES (1)",
            "/* c */ TABLE users",
            "WITH t AS (SELECT 1) SELECT * FROM t",
            "(SELECT 1) UNION (SELECT 2)",
        ] {
            assert!(cursor_statement(text), "{text}");
        }
        for text in ["SHOW search_path", "EXPLAIN SELECT 1", "FETCH 1 FROM c", ""] {
            assert!(!cursor_statement(text), "{text}");
        }
    }

    #[test]
    fn only_data_changing_statements_report_a_count() {
        for text in [
            "INSERT INTO t VALUES (1)",
            "update t SET n = 1",
            "-- note\nDELETE FROM t",
            "MERGE INTO t USING s ON true WHEN MATCHED THEN DO NOTHING",
        ] {
            assert!(counts_rows(text), "{text}");
        }
        for text in [
            "SET LOCAL work_mem = '8MB'",
            "DO $$ BEGIN END $$",
            "LISTEN updates",
            "",
        ] {
            assert!(!counts_rows(text), "{text}");
        }
    }

    #[test]
    fn a_cursor_is_asked_for_one_row_more_than_the_limit() {
        assert_eq!(fetch_count(0), 1);
        assert_eq!(fetch_count(1_000), 1_001);
        // FETCH counts in 32 bits.
        assert_eq!(fetch_count(i32::MAX as usize - 1), i32::MAX as usize);
        assert_eq!(fetch_count(u32::MAX as usize), i32::MAX as usize);
        assert_eq!(fetch_count(usize::MAX), i32::MAX as usize);
    }

    #[test]
    fn a_failed_lock_release_keeps_the_outcome_and_the_session() {
        assert_eq!(unlocked(Ok(())), Ok(()));
        assert_eq!(unlocked(Err(Error::query("permission denied"))), Ok(()));
        assert_eq!(unlocked(Err(Error::Cancelled)), Ok(()));
        let lost = Error::ConnectionLost("reset".into());
        assert_eq!(unlocked(Err(lost.clone())), Err(lost));
    }

    /// The test server's URL, or `None` (test skipped). See
    /// `tests/postgres.rs`.
    fn test_url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_PG_URL")
            .ok()
            .filter(|url| !url.trim().is_empty());
        if url.is_none() {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
        }
        url
    }

    /// A fresh read-only session, as the app opens it.
    async fn session(url: &str) -> Conn {
        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
        spec.tls = crate::TlsMode::Disable;
        Conn::connect(&spec, &secrets, None).await.unwrap()
    }

    /// One test at a time uses the `probe` table: creating it twice at once
    /// can fail, and one test's TRUNCATE would hide another's stray row.
    static PROBE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// A writable session with an empty `probe` table, and the table's
    /// lock, held until the test ends.
    async fn probe(url: &str) -> (tokio_postgres::Client, tokio::sync::MutexGuard<'static, ()>) {
        let turn = PROBE.lock().await;
        let mut config: tokio_postgres::Config = url.parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        let (client, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(connection);
        client
            .batch_execute("CREATE TABLE IF NOT EXISTS probe (n int); TRUNCATE probe;")
            .await
            .unwrap();
        (client, turn)
    }

    /// The rows in the `probe` table.
    async fn probe_rows(admin: &tokio_postgres::Client) -> i64 {
        admin
            .query_one("SELECT count(*) FROM probe", &[])
            .await
            .unwrap()
            .get(0)
    }

    /// The session's value of a setting.
    async fn setting(conn: &Conn, name: &str) -> String {
        let client = conn.client.lock().await;
        let messages = client.simple_query(&format!("SHOW {name}")).await.unwrap();
        first_text(&messages).unwrap()
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

    /// Statements the refusal stops long before they get here. Run past
    /// it, they end the script's transaction; the statement after them must
    /// not run, because it would run outside any read-only transaction, and
    /// the run must fail even when nothing follows them, so that the session
    /// is closed.
    #[tokio::test]
    async fn a_script_that_left_its_transaction_runs_nothing_more_and_fails() {
        let Some(url) = test_url() else {
            return;
        };
        let (admin, _turn) = probe(&url).await;
        for script in [
            &[
                "ROLLBACK",
                "SET default_transaction_read_only = off",
                "INSERT INTO probe VALUES (1)",
            ][..],
            &["COMMIT", "INSERT INTO probe VALUES (1)"][..],
            &["COMMIT", "BEGIN READ WRITE", "INSERT INTO probe VALUES (1)"][..],
            // Last in the script, nothing would run after them, but a
            // COMMIT keeps the settings made before it for later browsing.
            &["SET search_path = pg_catalog", "COMMIT"][..],
            &["COMMIT"][..],
            // A chained transaction is a block again, and read-only, but
            // it has no snapshot yet: it could be made read-write.
            &[
                "COMMIT AND CHAIN",
                "SET TRANSACTION READ WRITE",
                "INSERT INTO probe VALUES (1)",
                "COMMIT",
            ][..],
            &[
                "SET search_path = pg_catalog",
                "COMMIT AND CHAIN",
                "SELECT 1",
            ][..],
            &["COMMIT AND CHAIN"][..],
            &["ROLLBACK AND CHAIN", "SELECT 1"][..],
        ] {
            // A session of its own, dropped with the loop's turn: whatever
            // a failing guard let through cannot reach another test.
            let conn = session(&url).await;
            let stop = StopFlag::new();
            let ran = past_the_refusal(&conn, script, &stop).await;
            assert_eq!(ran, Err(Error::LeftReadOnly), "{script:?}");
            assert!(stop.is_finishing(), "{script:?}");
            assert_eq!(probe_rows(&admin).await, 0, "{script:?}");
            // The statement after the one that left never ran: the session
            // still defaults to read-only.
            assert_eq!(
                setting(&conn, "default_transaction_read_only").await,
                "on",
                "{script:?}"
            );
        }
    }

    /// The refusal keeps these from the driver. Past it, the server still
    /// refuses the write: the transaction has its snapshot and every
    /// statement runs under the guard's savepoint, either of which keeps it
    /// read-only, and what a statement set is rolled back.
    #[tokio::test]
    async fn statements_past_the_refusal_cannot_write() {
        let Some(url) = test_url() else {
            return;
        };
        let (admin, _turn) = probe(&url).await;
        const INSERT: &str = "INSERT INTO probe VALUES (1)";
        // The script, and the SQLSTATE its last result fails with.
        for (script, code) in [
            // The transaction cannot be made read-write any more.
            (&["SET TRANSACTION READ WRITE", INSERT][..], "25001"),
            (&["SET transaction_read_only = off", INSERT][..], "25001"),
            (
                &[
                    "SELECT set_config('transaction_read_only', 'off', true)",
                    INSERT,
                ][..],
                "25001",
            ),
            // The session default changes, but not this transaction.
            (
                &["SET \"default_transaction_read_only\" = off", INSERT][..],
                "25006",
            ),
            (
                &[
                    "SELECT set_config('default_transaction_read_only', 'off', false)",
                    INSERT,
                ][..],
                "25006",
            ),
            (&["COPY probe FROM STDIN"][..], "25006"),
            (
                &[
                    "PREPARE tabletist_probe AS INSERT INTO probe VALUES (1)",
                    "EXECUTE tabletist_probe",
                ][..],
                "25006",
            ),
        ] {
            let conn = session(&url).await;
            let stop = StopFlag::new();
            let outcome = past_the_refusal(&conn, script, &stop).await.unwrap();
            let failed = if code == "25001" { 1 } else { script.len() };
            assert_eq!(outcome.results.len(), failed, "{script:?}: {outcome:?}");
            assert!(
                matches!(
                    &outcome.results[failed - 1].outcome,
                    StatementOutcome::Error { error: Error::Query { code: Some(found), .. }, .. }
                        if found == code
                ),
                "{script:?}: {outcome:?}"
            );
            assert!(
                outcome.results[..failed - 1].iter().all(|result| matches!(
                    result.outcome,
                    StatementOutcome::Rows { .. } | StatementOutcome::Done { .. }
                )),
                "{script:?}: {outcome:?}"
            );
            assert_eq!(probe_rows(&admin).await, 0, "{script:?}");
            // The rollback took the setting back.
            assert_eq!(
                setting(&conn, "default_transaction_read_only").await,
                "on",
                "{script:?}"
            );
        }
    }

    /// A stop between statements does not fail the transaction, so the
    /// close still asks where the session stands. Here the statement before
    /// the stop ended the transaction and kept a setting.
    #[tokio::test]
    async fn a_stop_after_the_transaction_ended_still_fails_the_run() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let client = conn.client.lock().await;
        assert_eq!(open(&client).await, Ok(Tx::Open));
        // What the first two statements of the script would have done.
        client
            .batch_execute("SET search_path = pg_catalog")
            .await
            .unwrap();
        client.batch_execute("COMMIT").await.unwrap();
        // The stop lands before the third.
        let stop = StopFlag::new();
        stop.stop();
        let mut outcome = ScriptOutcome::default();
        let tx = statements(&client, &["SELECT 1".to_owned()], 10, &stop, &mut outcome).await;
        assert_eq!(tx, Ok(Tx::Open));
        assert_eq!(outcome.results.len(), 1);
        assert_eq!(outcome.results[0].outcome, StatementOutcome::Cancelled);
        assert!(outcome.stopped);
        assert_eq!(close(&client, Tx::Open).await, Err(Error::LeftReadOnly));
    }

    /// A stop between statements of a script that stayed in its transaction
    /// closes cleanly, and so does a transaction that failed behind the
    /// run's back (a cancel that landed on a check). A transaction that is
    /// not the script's fails the run, failed or not.
    #[tokio::test]
    async fn the_close_asks_the_server_where_an_open_transaction_stands() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        {
            let client = conn.client.lock().await;
            assert_eq!(open(&client).await, Ok(Tx::Open));
            assert_eq!(in_transaction(&client).await, Ok(Tx::Open));
            assert_eq!(close(&client, Tx::Open).await, Ok(()));
            // Closed: no transaction is left.
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left));

            assert_eq!(open(&client).await, Ok(Tx::Open));
            assert!(client.batch_execute("SELECT 1 / 0").await.is_err());
            assert_eq!(in_transaction(&client).await, Ok(Tx::Aborted));
            assert_eq!(close(&client, Tx::Open).await, Ok(()));
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left));

            // Another transaction is not the script's, block or not.
            assert_eq!(open(&client).await, Ok(Tx::Open));
            client.batch_execute("COMMIT AND CHAIN").await.unwrap();
            assert_eq!(close(&client, Tx::Open).await, Err(Error::LeftReadOnly));
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left));

            // Nor is it once it failed (a cancel that landed on a check):
            // a failed block does not say whose it is, its savepoint does.
            assert_eq!(open(&client).await, Ok(Tx::Open));
            client.batch_execute("COMMIT AND CHAIN").await.unwrap();
            assert!(client.batch_execute("SELECT 1 / 0").await.is_err());
            assert_eq!(in_transaction(&client).await, Ok(Tx::Aborted));
            assert_eq!(close(&client, Tx::Open).await, Err(Error::LeftReadOnly));
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left));
        }
        // The session was not closed, and it works.
        let ran = past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new()).await;
        assert_eq!(ran.unwrap().results.len(), 1);
    }

    /// A session stuck in a failed transaction cannot start a script's.
    /// That is not a query error to show and repeat: the session is closed,
    /// after an attempt to end what was open.
    #[tokio::test]
    async fn a_transaction_that_cannot_start_closes_the_session() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        {
            let client = conn.client.lock().await;
            client.batch_execute("BEGIN READ ONLY").await.unwrap();
            assert!(client.batch_execute("SELECT 1 / 0").await.is_err());
        }
        let stop = StopFlag::new();
        let ran = past_the_refusal(&conn, &["SELECT 1"], &stop).await;
        assert!(
            matches!(&ran, Err(error @ Error::ConnectionLost(_)) if error.is_connection_lost()),
            "{ran:?}"
        );
        assert!(stop.is_finishing());
        // The attempt worked: the failed transaction is gone.
        let client = conn.client.lock().await;
        assert_eq!(in_transaction(&client).await, Ok(Tx::Left));
    }

    /// A server that declines TLS and accepts anyone, answering every
    /// query with one empty result.
    async fn plain_server() -> u16 {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            // SSLRequest: declined.
            let mut request = [0u8; 8];
            socket.read_exact(&mut request).await.unwrap();
            socket.write_all(b"N").await.unwrap();
            // StartupMessage: AuthenticationOk, then ReadyForQuery.
            let length = socket.read_u32().await.unwrap() as usize;
            let mut startup = vec![0u8; length - 4];
            socket.read_exact(&mut startup).await.unwrap();
            socket
                .write_all(b"R\0\0\0\x08\0\0\0\0Z\0\0\0\x05I")
                .await
                .unwrap();
            while let Ok(kind) = socket.read_u8().await {
                let length = socket.read_u32().await.unwrap() as usize;
                let mut body = vec![0u8; length - 4];
                socket.read_exact(&mut body).await.unwrap();
                if kind == b'Q' {
                    socket
                        .write_all(b"C\0\0\0\x08SET\0Z\0\0\0\x05I")
                        .await
                        .unwrap();
                }
            }
        });
        port
    }

    #[tokio::test]
    async fn prefer_says_when_it_fell_back_to_plain_text() {
        let port = plain_server().await;
        let (mut spec, secrets) =
            ConnectSpec::from_url(&format!("postgres://me@127.0.0.1:{port}/app")).unwrap();
        spec.tls = crate::TlsMode::Prefer;
        let conn = Conn::connect(&spec, &secrets, None).await.unwrap();
        assert!(!conn.encrypted);
    }
}
