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

use crate::adapter::Adapter;
use crate::{
    Access, CancelHandle, CancelInner, ChangeSet, ColumnInfo, ColumnMeta, ConnectSpec, Dialect,
    Error, ForeignKeyInfo, IndexInfo, MAX_LISTED, ObjectInfo, ObjectKind, ObjectRef, Result,
    RowPage, RowQuery, ScriptMode, ScriptOutcome, Secrets, StopFlag, Structure, Value, ValueKind,
    WriteOutcome, value_from_pg_text,
};
use tokio_postgres::error::SqlState;
use tokio_postgres_rustls::MakeRustlsConnect;

mod script;
mod write;

/// How every session prints a float and a time, whatever the server, the
/// database or the role has as a default: what a page shows is what a save
/// sends back, to find the row again and to say what it held. With
/// `extra_float_digits` at 0 or below (the default before PostgreSQL 12)
/// two neighbouring floats print alike, and the key of one finds the other.
/// A `DateStyle` other than ISO prints a zone as its abbreviation, which
/// can read back as another zone (`IST` is India's and, to the server,
/// Israel's); ISO prints the offset in numbers. A script's own `SET` of
/// either goes with its rollback.
const PRINTS_EXACTLY: &str = "SET extra_float_digits = 3; SET DateStyle = 'ISO'";

pub(crate) struct Conn {
    pub(crate) client: tokio::sync::Mutex<tokio_postgres::Client>,
    pub(crate) cancel: tokio_postgres::CancelToken,
    pub(crate) tls: MakeRustlsConnect,
    config: tokio_postgres::Config,
    requested_database: String,
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
        .dbname(database_name(spec))
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

fn database_name(spec: &ConnectSpec) -> &str {
    if spec.database.is_empty() {
        &spec.user
    } else {
        &spec.database
    }
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

/// The settings a session has from the start. A writable session keeps
/// the server's default: row fetches, counts and a script that only reads
/// open read-only transactions of their own, and none of the script
/// guard's checks read the session's default. A script that writes sends
/// these again after its run, which resets every setting.
fn session_setup(access: Access) -> String {
    let read_only = match access {
        Access::ReadOnly => "SET default_transaction_read_only = on; ",
        Access::Writable => "",
    };
    format!("{read_only}SET standard_conforming_strings = on; {PRINTS_EXACTLY}")
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
        let tls = MakeRustlsConnect::new(crate::tls::client_config(
            spec.tls,
            spec.ca_file.as_deref(),
        )?);
        let config = config(spec, secrets, via);
        Self::open(config, tls, database_name(spec), access).await
    }

    /// Opens the session `config` describes and sets it up for `access`.
    async fn open(
        config: tokio_postgres::Config,
        tls: MakeRustlsConnect,
        database: &str,
        access: Access,
    ) -> Result<Self> {
        // The config's own timeout covers only the TCP connect, which is
        // instant to a tunnel's local port: bound the whole startup instead.
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
            .batch_execute(&session_setup(access))
            .await
            .map_err(query_error)?;
        let cancel = client.cancel_token();
        Ok(Self {
            client: tokio::sync::Mutex::new(client),
            cancel,
            tls,
            config,
            requested_database: database.to_owned(),
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

    /// PgBouncer's reserved admin database is the protocol-level source of
    /// its routing aliases. A stats/admin user can list them. PostgreSQL's
    /// missing-database response allows the ordinary catalog listing. If
    /// the probe is denied or inconclusive, keep only the working alias.
    async fn database_listing(&self, timeout: Duration) -> Option<Vec<String>> {
        // Bound both startup and SHOW. Driving the connection in this future
        // also closes the probe socket when it finishes or is cancelled.
        tokio::time::timeout(timeout, self.pgbouncer_databases())
            .await
            .unwrap_or_else(|_| Some(vec![self.requested_database.clone()]))
    }

    async fn pgbouncer_databases(&self) -> Option<Vec<String>> {
        let mut config = self.config.clone();
        config.dbname("pgbouncer");
        let connecting = config.connect(Noted {
            inner: self.tls.clone(),
            handshake: Arc::new(AtomicBool::new(false)),
        });
        let (client, connection) = match connecting.await {
            Ok(pair) => pair,
            Err(error)
                if error
                    .as_db_error()
                    .is_some_and(|error| error.code() == &SqlState::INVALID_CATALOG_NAME) =>
            {
                return None;
            }
            // An auth, transport or proxy error does not establish that the
            // backend catalog's names are valid routes on this endpoint.
            Err(_) => return Some(vec![self.requested_database.clone()]),
        };
        let messages = tokio::select! {
            result = client.simple_query("SHOW DATABASES") => match result {
                Ok(messages) => messages,
                Err(error) if error.as_db_error().is_some_and(|error| {
                    // A real PostgreSQL database named pgbouncer has no
                    // configuration parameter named databases.
                    error.code() == &SqlState::UNDEFINED_OBJECT
                }) => return None,
                Err(_) => return Some(vec![self.requested_database.clone()]),
            },
            _ = connection => return Some(vec![self.requested_database.clone()]),
        };
        let has_database_columns = messages.iter().any(|message| {
            matches!(message, SimpleQueryMessage::RowDescription(columns)
                if columns.iter().any(|column| column.name() == "name")
                    && columns.iter().any(|column| column.name() == "database"))
        });
        if !has_database_columns {
            return Some(vec![self.requested_database.clone()]);
        }
        let mut aliases: Vec<String> = messages
            .iter()
            .filter_map(|message| match message {
                SimpleQueryMessage::Row(row)
                    if row.try_get("disabled").ok().flatten() != Some("1") =>
                {
                    row.try_get("name").ok().flatten()
                }
                _ => None,
            })
            .filter(|name| *name != "pgbouncer" && *name != "*")
            .map(str::to_owned)
            .collect();
        // Keep the alias that already connected, including after a reload
        // removes or disables its entry in the console's listing.
        aliases.push(self.requested_database.clone());
        aliases.sort();
        aliases.dedup();
        Some(aliases)
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
                // Only the key's own columns: `INCLUDE` puts more after them
                // in `indkey`, and those are no part of the key. Their count,
                // `indnkeyatts`, exists from PostgreSQL 11, so it is read
                // through `to_jsonb`: every page asks for the key, and naming
                // a column an older server lacks would fail them all.
                "SELECT a.attname::text \
                 FROM pg_index i \
                 JOIN pg_class c ON c.oid = i.indrelid \
                 JOIN pg_namespace n ON n.oid = c.relnamespace \
                 CROSS JOIN LATERAL unnest(i.indkey) WITH ORDINALITY AS k(attnum, ord) \
                 JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = k.attnum \
                 WHERE n.nspname = $1 AND c.relname = $2 AND i.indisprimary \
                   AND k.ord <= COALESCE((to_jsonb(i) ->> 'indnkeyatts')::int, i.indnatts) \
                 ORDER BY k.ord",
                &[&object.schema, &object.name],
            )
            .await?;
        rows.iter().map(|row| column(row, 0)).collect()
    }

    /// The `bytea` columns a filter of `query` compares as bytes when its
    /// value reads as bytes; empty, without asking, when no filter has such
    /// a value.
    async fn binary_columns(&self, query: &RowQuery) -> Result<Vec<String>> {
        if !crate::dialect::reads_bytes(query) {
            return Ok(Vec::new());
        }
        let rows = self
            .catalog(
                "SELECT a.attname::text \
                 FROM pg_attribute a \
                 JOIN pg_class c ON c.oid = a.attrelid \
                 JOIN pg_namespace n ON n.oid = c.relnamespace \
                 WHERE n.nspname = $1 AND c.relname = $2 AND a.attnum > 0 \
                   AND NOT a.attisdropped AND a.atttypid = 'bytea'::regtype",
                &[&query.object.schema, &query.object.name],
            )
            .await?;
        rows.iter().map(|row| column(row, 0)).collect()
    }
}

impl Adapter for Conn {
    fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    fn cancel_handle(&self) -> CancelHandle {
        CancelHandle(CancelInner::Postgres {
            token: self.cancel.clone(),
            tls: self.tls.clone(),
        })
    }

    /// See [`crate::Connection::server_version`].
    async fn server_version(&self) -> Result<String> {
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

    async fn list_databases(&self) -> Result<Vec<String>> {
        if let Some(aliases) = self.database_listing(Duration::from_secs(10)).await {
            return Ok(aliases);
        }
        let rows = self
            .catalog(
                "SELECT datname::text FROM pg_database \
                 WHERE datallowconn AND NOT datistemplate ORDER BY datname",
                &[],
            )
            .await?;
        rows.iter().map(|row| column(row, 0)).collect()
    }

    async fn list_schemas(&self) -> Result<Vec<String>> {
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

    async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
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

    async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        let oid = self.relation(object).await?;
        let columns = self
            .catalog(
                // An enum's labels in their sort order; a text column's
                // single-column CHECK constraints, by name. Whether the
                // column is generated is read through `to_jsonb`, for
                // PostgreSQL 11: `attgenerated` only exists from 12, and
                // naming a column the server lacks would fail the whole
                // Structure view. Nothing older is spared by it: the index
                // query below names `indnkeyatts`, which 11 brought.
                "SELECT a.attname::text, format_type(a.atttypid, a.atttypmod), NOT a.attnotnull, \
                        pg_get_expr(d.adbin, d.adrelid), col_description(a.attrelid, a.attnum), \
                        CASE WHEN t.typtype = 'e' THEN \
                            ARRAY(SELECT e.enumlabel::text FROM pg_enum e \
                                  WHERE e.enumtypid = t.oid ORDER BY e.enumsortorder) END, \
                        CASE WHEN t.oid IN ('text'::regtype, 'varchar'::regtype) THEN \
                            ARRAY(SELECT pg_get_expr(c.conbin, c.conrelid) FROM pg_constraint c \
                                  WHERE c.conrelid = a.attrelid AND c.contype = 'c' \
                                    AND c.conkey = ARRAY[a.attnum] ORDER BY c.conname) END, \
                        COALESCE(to_jsonb(a) ->> 'attgenerated', '') <> '' \
                            OR COALESCE(to_jsonb(a) ->> 'attidentity', '') = 'a' \
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
                    generated: column(row, 7)?,
                })
            })
            .collect::<Result<_>>()?;
        let indexes = self
            .catalog(
                // An index that is not valid (a `CREATE UNIQUE INDEX
                // CONCURRENTLY` that failed) does not hold its rows unique,
                // whatever it is called. The last column is the key's
                // columns by name, or NULL when an entry is not one whole
                // column compared as the column compares: an expression (0
                // in `indkey`), another collation than the column's, or an
                // operator class that is not the default one. The text
                // `pg_get_indexdef` shows tells none of these apart.
                "SELECT ic.relname::text, i.indisunique AND i.indisvalid, i.indisprimary, \
                        am.amname::text, \
                        ARRAY(SELECT pg_get_indexdef(i.indexrelid, k, true) \
                              FROM generate_series(1, i.indnkeyatts::int) AS k ORDER BY k), \
                        i.indpred IS NOT NULL, \
                        CASE WHEN NOT EXISTS ( \
                                 SELECT 1 FROM generate_series(1, i.indnkeyatts::int) AS k \
                                 LEFT JOIN pg_attribute a \
                                   ON a.attrelid = i.indrelid AND a.attnum = i.indkey[k - 1] \
                                 LEFT JOIN pg_opclass oc ON oc.oid = i.indclass[k - 1] \
                                 WHERE i.indkey[k - 1] = 0 \
                                    OR a.attcollation <> i.indcollation[k - 1] \
                                    OR NOT oc.opcdefault) \
                             THEN ARRAY(SELECT a.attname::text \
                                        FROM generate_series(1, i.indnkeyatts::int) AS k \
                                        JOIN pg_attribute a \
                                          ON a.attrelid = i.indrelid AND a.attnum = i.indkey[k - 1] \
                                        ORDER BY k) END \
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
                    key_columns: column(row, 6)?,
                    partial: column(row, 5)?,
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
    async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        let key = self.primary_key(&query.object).await?;
        let binary = self.binary_columns(query).await?;
        let sql = Dialect::Postgres.select_rows(query, &key, &binary);
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

    async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        let binary = self.binary_columns(query).await?;
        let sql = Dialect::Postgres.count_rows(query, &binary);
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

/// The first row's first value.
fn first_text(messages: &[SimpleQueryMessage]) -> Option<String> {
    messages.iter().find_map(|message| match message {
        SimpleQueryMessage::Row(row) => row.get(0).map(str::to_owned),
        _ => None,
    })
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

    enum ProbeReply {
        StartupError(&'static str, &'static str),
        Databases(Vec<[&'static str; 4]>),
        QueryError(&'static str, &'static str),
        Hang,
    }

    /// A protocol peer for the extra connection. It checks the requested
    /// database and the simple-query command, and waits for socket cleanup.
    async fn probe_server(reply: ProbeReply) -> (Conn, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        fn packet(kind: u8, body: &[u8]) -> Vec<u8> {
            let mut packet = vec![kind];
            packet.extend_from_slice(&((body.len() + 4) as u32).to_be_bytes());
            packet.extend_from_slice(body);
            packet
        }

        fn error(code: &str, message: &str) -> Vec<u8> {
            packet(b'E', format!("SFATAL\0C{code}\0M{message}\0\0").as_bytes())
        }

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let task =
            tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut ssl = [0; 8];
                socket.read_exact(&mut ssl).await.unwrap();
                socket.write_all(b"N").await.unwrap();
                let length = socket.read_u32().await.unwrap() as usize;
                let mut startup = vec![0; length - 4];
                socket.read_exact(&mut startup).await.unwrap();
                assert!(
                    startup
                        .windows(19)
                        .any(|part| part == b"database\0pgbouncer\0")
                );
                if let ProbeReply::StartupError(code, message) = reply {
                    socket.write_all(&error(code, message)).await.unwrap();
                    return;
                }
                socket
                    .write_all(b"R\0\0\0\x08\0\0\0\0Z\0\0\0\x05I")
                    .await
                    .unwrap();
                assert_eq!(socket.read_u8().await.unwrap(), b'Q');
                let length = socket.read_u32().await.unwrap() as usize;
                let mut query = vec![0; length - 4];
                socket.read_exact(&mut query).await.unwrap();
                assert_eq!(query, b"SHOW DATABASES\0");
                match reply {
                    ProbeReply::Databases(rows) => {
                        let mut description = 4u16.to_be_bytes().to_vec();
                        for name in ["name", "database", "disabled", "force_user"] {
                            description.extend_from_slice(name.as_bytes());
                            description.push(0);
                            description.extend_from_slice(&0u32.to_be_bytes());
                            description.extend_from_slice(&0u16.to_be_bytes());
                            description.extend_from_slice(&25u32.to_be_bytes());
                            description.extend_from_slice(&(-1i16).to_be_bytes());
                            description.extend_from_slice(&(-1i32).to_be_bytes());
                            description.extend_from_slice(&0u16.to_be_bytes());
                        }
                        socket.write_all(&packet(b'T', &description)).await.unwrap();
                        for row in rows {
                            let mut body = 4u16.to_be_bytes().to_vec();
                            for value in row {
                                body.extend_from_slice(&(value.len() as u32).to_be_bytes());
                                body.extend_from_slice(value.as_bytes());
                            }
                            socket.write_all(&packet(b'D', &body)).await.unwrap();
                        }
                        socket.write_all(&packet(b'C', b"SHOW\0")).await.unwrap();
                        socket.write_all(&packet(b'Z', b"I")).await.unwrap();
                    }
                    ProbeReply::QueryError(code, message) => {
                        socket.write_all(&error(code, message)).await.unwrap();
                        socket.write_all(&packet(b'Z', b"I")).await.unwrap();
                    }
                    ProbeReply::Hang => {}
                    ProbeReply::StartupError(_, _) => unreachable!(),
                }
                // The probe must release its connection, even on timeout.
                let mut rest = Vec::new();
                let closed =
                    tokio::time::timeout(Duration::from_secs(2), socket.read_to_end(&mut rest))
                        .await
                        .expect("the probe socket must close");
                assert!(closed.is_ok() || closed.is_err_and(|error|
                error.kind() == std::io::ErrorKind::ConnectionReset));
            });
        let main_port = plain_server().await;
        let (mut spec, secrets) =
            ConnectSpec::from_url(&format!("postgres://me@127.0.0.1:{main_port}/app_pool"))
                .unwrap();
        spec.tls = crate::TlsMode::Prefer;
        let mut conn = Conn::connect(&spec, &secrets, None, Access::ReadOnly)
            .await
            .unwrap();
        spec.port = port;
        conn.config = config(&spec, &secrets, None);
        (conn, task)
    }

    #[tokio::test]
    async fn a_denied_console_keeps_only_the_working_alias() {
        let (conn, task) = probe_server(ProbeReply::StartupError("08P01", "not allowed")).await;
        assert_eq!(conn.list_databases().await.unwrap(), vec!["app_pool"]);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn a_postgres_missing_database_response_uses_the_catalog() {
        let (conn, task) =
            probe_server(ProbeReply::StartupError("3D000", "database does not exist")).await;
        assert_eq!(conn.database_listing(Duration::from_secs(2)).await, None);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn an_accessible_console_lists_routes_and_keeps_the_working_alias() {
        let (conn, task) = probe_server(ProbeReply::Databases(vec![
            ["jobs_pool", "jobs", "0", "backend_user"],
            ["pgbouncer", "pgbouncer", "0", ""],
            ["*", "", "0", ""],
            ["disabled_pool", "archive", "1", ""],
            ["jobs_pool", "jobs", "0", "backend_user"],
        ]))
        .await;
        assert_eq!(
            conn.list_databases().await.unwrap(),
            vec!["app_pool", "jobs_pool"]
        );
        task.await.unwrap();
    }

    #[tokio::test]
    async fn an_empty_console_does_not_offer_physical_database_names() {
        let (conn, task) = probe_server(ProbeReply::Databases(Vec::new())).await;
        assert_eq!(conn.list_databases().await.unwrap(), vec!["app_pool"]);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn a_real_postgres_database_named_pgbouncer_uses_the_catalog() {
        let (conn, task) = probe_server(ProbeReply::QueryError(
            "42704",
            "unrecognized configuration parameter",
        ))
        .await;
        assert_eq!(conn.database_listing(Duration::from_secs(2)).await, None);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn an_inconclusive_probe_keeps_only_the_working_alias() {
        let (conn, task) = probe_server(ProbeReply::StartupError(
            "28P01",
            "password authentication failed",
        ))
        .await;
        assert_eq!(conn.list_databases().await.unwrap(), vec!["app_pool"]);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn a_stalled_console_times_out_and_releases_its_socket() {
        let (conn, task) = probe_server(ProbeReply::Hang).await;
        assert_eq!(
            conn.database_listing(Duration::from_millis(100)).await,
            Some(vec!["app_pool".to_owned()])
        );
        task.await.unwrap();
    }

    /// The test server's URL, or `None` (test skipped). See
    /// `tests/postgres.rs`.
    pub(super) fn test_url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_PG_URL")
            .ok()
            .filter(|url| !url.trim().is_empty());
        if url.is_none() {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
        }
        url
    }

    /// A fresh read-only session, as the app opens it.
    pub(super) async fn session(url: &str) -> Conn {
        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
        spec.tls = crate::TlsMode::Disable;
        Conn::connect(&spec, &secrets, None, Access::ReadOnly)
            .await
            .unwrap()
    }

    /// A fresh session opened as `access`, on a server whose own defaults
    /// are `options`: startup options give a session what a server's
    /// configuration, a database's or a role's would, and what the driver
    /// sets when it connects comes after them all the same.
    pub(super) async fn session_with(url: &str, access: Access, options: &str) -> Conn {
        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
        spec.tls = crate::TlsMode::Disable;
        let tls = MakeRustlsConnect::new(crate::tls::client_config(spec.tls, None).unwrap());
        let mut config = config(&spec, &secrets, None);
        config.options(options);
        Conn::open(config, tls, database_name(&spec), access)
            .await
            .unwrap()
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
        let conn = Conn::connect(&spec, &secrets, None, Access::ReadOnly)
            .await
            .unwrap();
        assert!(!conn.encrypted);
    }
}
