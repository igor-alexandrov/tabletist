//! PostgreSQL. Catalog queries use the extended protocol with typed results;
//! row queries use the simple-query protocol, which sends every value as
//! text.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tokio_postgres::tls::{MakeTlsConnect, TlsConnect};
use tokio_postgres::types::{ToSql, Type};
use tokio_postgres::{SimpleQueryMessage, Socket};

use crate::adapter::Adapter;
use crate::agent::{Catalog, ColumnName, Limits, Read, Sources};
use crate::{
    Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, ConnectSpec,
    Ddl, Dialect, Error, ForeignKeyInfo, IndexInfo, Lookup, LookupPage, MAX_LISTED, Named,
    ObjectInfo, ObjectKind, ObjectRef, ReferencingKey, Result, RowPage, RowQuery, ScriptMode,
    ScriptOutcome, Secrets, Sql, StopFlag, Structure, Value, ValueKind, WriteOutcome,
    value_from_pg_text,
};
use tokio_postgres::error::SqlState;
use tokio_postgres_rustls::MakeRustlsConnect;

mod cancel;
pub(crate) use cancel::Cancel;
mod ddl;
mod hidden;
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
    cancel: Cancel,
    pub(crate) tls: MakeRustlsConnect,
    config: tokio_postgres::Config,
    requested_database: String,
    /// Whether the session runs over TLS: under `prefer` a server that
    /// declines TLS gets plain text.
    pub(crate) encrypted: bool,
    /// The shape of each table this session read last, by table: what the
    /// next page builds its statement from. It is asked again with every
    /// page, in the same flight as the page, and a page built from a shape
    /// that has changed since is read again (see `Adapter::fetch_rows`).
    shapes: std::sync::Mutex<std::collections::HashMap<ObjectRef, Shape>>,
}

/// What a statement over a table needs to know of it: its primary key's
/// columns, in key order, and its `bytea` columns. Both are empty for a
/// table that does not exist: a statement over it then fails on the
/// server, which says why.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Shape {
    key: Vec<String>,
    bytea: Vec<String>,
}

/// The query [`Shape`] reads, with the table's schema and name as `$1` and
/// `$2`. Only the key's own columns: `INCLUDE` puts more after them in
/// `indkey`, and those are no part of the key. Their count, `indnkeyatts`,
/// exists from PostgreSQL 11, so it is read through `to_jsonb`: every page
/// asks for the shape, and naming a column an older server lacks would fail
/// them all.
const SHAPE_SQL: &str = "SELECT a.attname::text, k.ord IS NOT NULL, a.atttypid = 'bytea'::regtype \
     FROM pg_class c \
     JOIN pg_namespace n ON n.oid = c.relnamespace \
     JOIN pg_attribute a \
       ON a.attrelid = c.oid AND a.attnum > 0 AND NOT a.attisdropped \
     LEFT JOIN LATERAL ( \
         SELECT k.ord FROM pg_index i \
         CROSS JOIN LATERAL unnest(i.indkey) WITH ORDINALITY AS k(attnum, ord) \
         WHERE i.indrelid = c.oid AND i.indisprimary AND k.attnum = a.attnum \
           AND k.ord <= COALESCE((to_jsonb(i) ->> 'indnkeyatts')::int, i.indnatts) \
     ) k ON true \
     WHERE n.nspname = $1 AND c.relname = $2 \
       AND (k.ord IS NOT NULL OR a.atttypid = 'bytea'::regtype) \
     ORDER BY k.ord, a.attnum";

impl Shape {
    /// The shape in [`SHAPE_SQL`]'s rows.
    fn from_rows(rows: &[tokio_postgres::Row]) -> Result<Self> {
        let mut shape = Self::default();
        for row in rows {
            let name: String = column(row, 0)?;
            if column(row, 2)? {
                shape.bytea.push(name.clone());
            }
            if column(row, 1)? {
                shape.key.push(name);
            }
        }
        Ok(shape)
    }

    /// The key, and the `bytea` columns a filter of `query` compares as
    /// bytes: none when no filter has a value that reads as bytes.
    fn for_query(&self, query: &RowQuery) -> (Vec<String>, Vec<(String, ColumnClass)>) {
        let binary = if crate::dialect::reads_bytes(query) {
            self.bytea
                .iter()
                .map(|name| (name.clone(), ColumnClass::Binary))
                .collect()
        } else {
            Vec::new()
        };
        (self.key.clone(), binary)
    }
}

/// `object`'s [`Shape`], read in one round trip.
async fn table_shape(client: &tokio_postgres::Client, object: &ObjectRef) -> Result<Shape> {
    let rows = client
        .query_typed(
            SHAPE_SQL,
            &[(&object.schema, Type::TEXT), (&object.name, Type::TEXT)],
        )
        .await
        .map_err(query_error)?;
    Shape::from_rows(&rows)
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

/// A session the driver opened, the config that opened it, and where its
/// cancels go.
type Pinned = (
    (
        tokio_postgres::Client,
        tokio_postgres::Connection<Socket, <Noted as MakeTlsConnect<Socket>>::Stream>,
    ),
    tokio_postgres::Config,
    Option<(std::net::SocketAddr, String)>,
);

/// Opens a session on one address of the host, tried in the order the
/// driver tries them, and says which one: a cancel must reach the same
/// server. A host this cannot resolve (or a Unix socket) is left to the
/// driver, which says why it cannot connect.
async fn connect_pinned(
    config: tokio_postgres::Config,
    tls: &MakeRustlsConnect,
    handshake: &Arc<AtomicBool>,
) -> std::result::Result<Pinned, tokio_postgres::Error> {
    let noted = || Noted {
        inner: tls.clone(),
        handshake: Arc::clone(handshake),
    };
    let port = config.get_ports().first().copied().unwrap_or(5432);
    if let [tokio_postgres::config::Host::Tcp(host)] = config.get_hosts() {
        let addresses = match config.get_hostaddrs() {
            // Through a tunnel: the address its local port listens on.
            [address] => vec![*address],
            [] => tokio::net::lookup_host((host.as_str(), port))
                .await
                .map(|found| found.map(|address| address.ip()).collect())
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let mut failed = None;
        for address in addresses {
            let mut pinned = config.clone();
            if pinned.get_hostaddrs().is_empty() {
                pinned.hostaddr(address);
            }
            match pinned.connect(noted()).await {
                Ok(pair) => {
                    let to = (std::net::SocketAddr::new(address, port), host.clone());
                    return Ok((pair, pinned, Some(to)));
                }
                Err(error) => failed = Some(error),
            }
        }
        if let Some(error) = failed {
            return Err(error);
        }
    }
    let pair = config.connect(noted()).await?;
    Ok((pair, config, None))
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
            named: Box::default(),
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

/// The cursor an export reads through. One export per connection, so one
/// name.
const EXPORT_CURSOR: &str = "tabletist_export";

/// How long the PgBouncer probe may take, startup and SHOW together.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// [`query_error`], where the server's own `statement_timeout` (which
/// PostgreSQL reports as a cancel) is [`Error::Timeout`]: an export sets no
/// timeout of its own, so a cancel that names it was the server's.
fn stream_error(error: tokio_postgres::Error) -> Error {
    if let Some(db) = error.as_db_error()
        && *db.code() == SqlState::QUERY_CANCELED
        && db.message().contains("statement timeout")
    {
        return Error::Timeout;
    }
    query_error(error)
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
            // The server's own fields, which do not change with the
            // language of its messages. It names a NOT NULL failure's
            // column, and the constraint of the others.
            named: Box::new(Named {
                number: None,
                constraint: db.constraint().map(str::to_owned),
                columns: db.column().map(str::to_owned).into_iter().collect(),
            }),
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
    /// `text`'s rows through a cursor in a read-only transaction, a
    /// `FETCH` of `batch` rows at a time.
    async fn stream_text(
        &self,
        text: &str,
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        let mut client = self.client.lock().await;
        let transaction = client
            .build_transaction()
            .read_only(true)
            .start()
            .await
            .map_err(stream_error)?;
        // Prepared first: its columns, and a second statement refused
        // before anything runs.
        let statement = transaction.prepare(text).await.map_err(stream_error)?;
        let metas = column_metas(&statement);
        crate::stream::send(to, crate::Streamed::Columns(metas.clone())).await?;
        transaction
            .simple_query(&format!(
                "DECLARE {EXPORT_CURSOR} NO SCROLL CURSOR FOR {text}"
            ))
            .await
            .map_err(stream_error)?;
        let fetch = format!("FETCH {batch} FROM {EXPORT_CURSOR}");
        let mut sent = 0u64;
        loop {
            if stop.is_stopped() {
                return Err(Error::Cancelled);
            }
            let messages = transaction
                .simple_query(&fetch)
                .await
                .map_err(stream_error)?;
            let mut rows = Vec::with_capacity(batch);
            for message in messages {
                if let SimpleQueryMessage::Row(row) = message {
                    rows.push(row_values(&row, &metas)?);
                }
            }
            if rows.is_empty() {
                break;
            }
            sent += rows.len() as u64;
            crate::stream::send(to, crate::Streamed::Rows(rows)).await?;
        }
        stop.finish();
        transaction.rollback().await.map_err(stream_error)?;
        Ok(sent)
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
        let connecting = connect_pinned(config, &tls, &handshake);
        let ((client, connection), config, address) =
            tokio::time::timeout(Duration::from_secs(10), connecting)
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
        let cancel = Cancel {
            token: client.cancel_token(),
            tls: tls.clone(),
            to: address,
        };
        Ok(Self {
            client: tokio::sync::Mutex::new(client),
            cancel,
            tls,
            config,
            requested_database: database.to_owned(),
            encrypted: handshake.load(Ordering::Relaxed),
            shapes: Default::default(),
        })
    }

    /// A catalog query: fixed SQL, typed results. One round trip: with the
    /// parameters' types given, nothing is prepared first.
    async fn catalog(
        &self,
        sql: &str,
        params: &[(&(dyn ToSql + Sync), Type)],
    ) -> Result<Vec<tokio_postgres::Row>> {
        let client = self.client.lock().await;
        client.query_typed(sql, params).await.map_err(query_error)
    }

    /// PgBouncer's reserved admin database is the protocol-level source of
    /// its routing aliases. A stats/admin user can list them. PostgreSQL's
    /// missing-database response allows the ordinary catalog listing. If
    /// the probe is denied or inconclusive, keep only the working alias.
    ///
    /// Owns all it needs and runs on a connection of its own, so that it
    /// can run beside the session's commands (see `database_probe`).
    fn database_listing(
        &self,
        timeout: Duration,
    ) -> impl Future<Output = Option<Vec<String>>> + Send + 'static + use<> {
        let config = self.config.clone();
        let tls = self.tls.clone();
        let requested = self.requested_database.clone();
        async move {
            // Bound both startup and SHOW. Driving the connection in this
            // future also closes the probe socket when it finishes or is
            // cancelled.
            tokio::time::timeout(timeout, Self::pgbouncer_databases(config, tls, &requested))
                .await
                .unwrap_or_else(|_| Some(vec![requested.clone()]))
        }
    }

    /// The PgBouncer probe of `list_databases`, to run apart from the
    /// session: `Some` aliases answer the listing, `None` leaves it to
    /// `catalog_databases`.
    pub(crate) fn database_probe(
        &self,
    ) -> impl Future<Output = Option<Vec<String>>> + Send + 'static + use<> {
        self.database_listing(PROBE_TIMEOUT)
    }

    /// The probe itself, on a connection of its own to the `pgbouncer`
    /// database of the server `config` names: the console's aliases, `None`
    /// where the server is PostgreSQL itself, or only `requested` (the
    /// alias that connected) where the answer is unclear.
    async fn pgbouncer_databases(
        mut config: tokio_postgres::Config,
        tls: MakeRustlsConnect,
        requested: &str,
    ) -> Option<Vec<String>> {
        let working = || Some(vec![requested.to_owned()]);
        config.dbname("pgbouncer");
        let connecting = config.connect(Noted {
            inner: tls,
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
            Err(_) => return working(),
        };
        let messages = tokio::select! {
            result = client.simple_query("SHOW DATABASES") => match result {
                Ok(messages) => messages,
                Err(error) if error.as_db_error().is_some_and(|error| {
                    // A real PostgreSQL database named pgbouncer has no
                    // configuration parameter named databases.
                    error.code() == &SqlState::UNDEFINED_OBJECT
                }) => return None,
                Err(_) => return working(),
            },
            _ = connection => return working(),
        };
        let has_database_columns = messages.iter().any(|message| {
            matches!(message, SimpleQueryMessage::RowDescription(columns)
                if columns.iter().any(|column| column.name() == "name")
                    && columns.iter().any(|column| column.name() == "database"))
        });
        if !has_database_columns {
            return working();
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
        aliases.push(requested.to_owned());
        aliases.sort();
        aliases.dedup();
        Some(aliases)
    }

    /// The databases the server's catalog names: the listing where the
    /// probe found no PgBouncer.
    pub(crate) async fn catalog_databases(&self) -> Result<Vec<String>> {
        let rows = self
            .catalog(
                "SELECT datname::text FROM pg_database \
                 WHERE datallowconn AND NOT datistemplate ORDER BY datname",
                &[],
            )
            .await?;
        rows.iter().map(|row| column(row, 0)).collect()
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
                &[(&object.schema, Type::TEXT), (&object.name, Type::TEXT)],
            )
            .await?;
        rows.iter().map(|row| column(row, 0)).collect()
    }

    /// Runs one reading statement in a read-only transaction and keeps at
    /// most one row more than `limit`. The time is the statement's, from
    /// when the session is this call's.
    async fn read(
        &self,
        sql: &Sql,
        limit: usize,
    ) -> Result<(Vec<ColumnMeta>, Vec<Vec<Value>>, Duration)> {
        // Every value is a literal in the text, so there is nothing to bind.
        debug_assert!(sql.params.is_empty());
        let client = self.client.lock().await;
        let started = Instant::now();
        let (columns, messages) = read_statement(&client, &sql.text).await?;
        let rows = kept_rows(&messages, &columns, limit)?;
        Ok((columns, rows, started.elapsed()))
    }

    /// What a statement over `query`'s table needs to know of it, in one
    /// round trip: its primary key's columns, in key order, and the `bytea`
    /// columns a filter compares as bytes when its value reads as bytes
    /// (see [`Shape`]).
    async fn shape(&self, query: &RowQuery) -> Result<(Vec<String>, Vec<(String, ColumnClass)>)> {
        let client = self.client.lock().await;
        let shape = table_shape(&client, &query.object).await?;
        self.remember(&query.object, &shape);
        Ok(shape.for_query(query))
    }

    /// The `bytea` columns of [`Conn::shape`]: empty, without asking, when
    /// no filter of `query` has a value that reads as bytes.
    async fn binary_columns(&self, query: &RowQuery) -> Result<Vec<(String, ColumnClass)>> {
        if !crate::dialect::reads_bytes(query) {
            return Ok(Vec::new());
        }
        Ok(self.shape(query).await?.1)
    }

    /// The shape this session last read of `object`, if it read one.
    fn known_shape(&self, object: &ObjectRef) -> Option<Shape> {
        let shapes = self.shapes.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        shapes.get(object).cloned()
    }

    fn remember(&self, object: &ObjectRef, shape: &Shape) {
        let mut shapes = self.shapes.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        shapes.insert(object.clone(), shape.clone());
    }
}

/// The rows of `messages`, typed by `columns`: at most one more than
/// `limit`.
fn kept_rows(
    messages: &[SimpleQueryMessage],
    columns: &[ColumnMeta],
    limit: usize,
) -> Result<Vec<Vec<Value>>> {
    let mut rows = Vec::new();
    for message in messages {
        if let SimpleQueryMessage::Row(row) = message {
            if rows.len() > limit {
                break;
            }
            rows.push(row_values(row, columns)?);
        }
    }
    Ok(rows)
}

/// What a [`read_statement`] answers: the statement's columns, as the
/// prepare typed them, and what the server answered to its run.
type Answered = (Vec<ColumnMeta>, Vec<SimpleQueryMessage>);

/// Runs `text`, one reading statement, in a read-only transaction: what
/// the server answered, and the columns, as the prepare typed them. One
/// round trip: see [`read_alongside`].
async fn read_statement(client: &tokio_postgres::Client, text: &str) -> Result<Answered> {
    read_alongside(client, text, async {}).await.1
}

/// The first step of a read's run: it fails outside a transaction block,
/// and in one that has failed. So the statement after it runs only inside
/// the `BEGIN READ ONLY` the read sent first, and only once the prepare
/// before it took the text as one statement: a failed prepare fails the
/// transaction, and a failed `BEGIN` (a cancel that landed on it) leaves
/// none. Nothing of a message runs past the step that failed.
const READ_GUARD: &str = "SAVEPOINT tabletist_read";

/// [`read_statement`], with what `alongside` sends going out first, in
/// the same flight: what it answered, and the read's answer.
///
/// All of the read goes out at once, before any answer comes back: the
/// `BEGIN READ ONLY`, the prepare (which yields the column types, and
/// fails on a text of more than one statement), the run of `text` behind
/// [`READ_GUARD`], and the `ROLLBACK`. Each goes out when it is first
/// polled, and the first poll reaches them all, in this order. So a read
/// that is dropped once it has started leaves its `ROLLBACK` on its way,
/// and one that fails, wherever it fails, is rolled back by it.
async fn read_alongside<T>(
    client: &tokio_postgres::Client,
    text: &str,
    alongside: impl Future<Output = T>,
) -> (T, Result<Answered>) {
    let run = format!("{READ_GUARD};\n{text}");
    let (alongside, begun, prepared, ran, rolled_back) = tokio::join!(
        alongside,
        client.batch_execute("BEGIN READ ONLY"),
        client.prepare(text),
        client.simple_query(&run),
        client.batch_execute("ROLLBACK"),
    );
    // The first step that failed says why: the guard's own failure after a
    // failed `BEGIN` or prepare says nothing.
    let read = begun.and(prepared).and_then(|statement| {
        let columns = column_metas(&statement);
        // Its close goes out on its own, waited on by nobody.
        drop(statement);
        Ok((columns, ran?))
    });
    let read = match rolled_back {
        Ok(()) => read.map_err(query_error),
        // A cancel that landed on the rollback leaves the transaction
        // open: once more. A rollback that fails again is a session that
        // is gone, or a transaction the next read's guard fails on and
        // its rollback ends.
        Err(error) => {
            let _ = client.batch_execute("ROLLBACK").await;
            Err(query_error(read.err().unwrap_or(error)))
        }
    };
    (alongside, read)
}

impl Adapter for Conn {
    fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    fn cancel_handle(&self) -> CancelHandle {
        CancelHandle(CancelInner::Postgres(self.cancel.clone()))
    }

    /// None: a `timestamptz` comes with its offset, and the app shows it in
    /// the zone of its own settings.
    async fn session_zone(&self) -> Result<Option<String>> {
        Ok(None)
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
        if let Some(aliases) = self.database_probe().await {
            return Ok(aliases);
        }
        self.catalog_databases().await
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
                &[(&schema, Type::TEXT)],
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

    async fn ddl(&self, object: &ObjectRef) -> Result<Ddl> {
        self.read_ddl(object).await
    }

    /// The table's structure in one round trip: its five catalog queries
    /// go out together, each finding the table by its schema and name.
    async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        /// The table's oid, from the parameters every query of `describe`
        /// takes: its schema and its name.
        macro_rules! relation {
            () => {
                "(SELECT rc.oid FROM pg_class rc \
                  JOIN pg_namespace rn ON rn.oid = rc.relnamespace \
                  WHERE rn.nspname = $1 AND rc.relname = $2)"
            };
        }
        let params: &[(&(dyn ToSql + Sync), Type)] =
            &[(&object.schema, Type::TEXT), (&object.name, Type::TEXT)];
        let client = self.client.lock().await;
        let (relation, columns, indexes, foreign_keys, referenced_by, shape) = tokio::try_join!(
            // The comment on the table or view itself, and a row only when
            // there is a table or view.
            client.query_typed(
                "SELECT obj_description(c.oid, 'pg_class') \
                 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
                 WHERE n.nspname = $1 AND c.relname = $2",
                params,
            ),
            client.query_typed(
                // An enum's labels in their sort order; a text column's
                // single-column CHECK constraints, by name. Whether the
                // column is generated is read through `to_jsonb`, for
                // PostgreSQL 11: `attgenerated` only exists from 12, and
                // naming a column the server lacks would fail the whole
                // Structure view. Nothing older is spared by it: the index
                // query below names `indnkeyatts`, which 11 brought.
                concat!(
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
                            OR COALESCE(to_jsonb(a) ->> 'attidentity', '') = 'a', \
                        COALESCE(to_jsonb(a) ->> 'attidentity', '') <> '' \
                            OR EXISTS (SELECT 1 FROM pg_depend p \
                                       WHERE p.classid = 'pg_attrdef'::regclass \
                                         AND p.objid = d.oid \
                                         AND p.refclassid = 'pg_class'::regclass \
                                         AND p.refobjid = pg_get_serial_sequence( \
                                             a.attrelid::regclass::text, \
                                             a.attname::text)::regclass) \
                 FROM pg_attribute a \
                 JOIN pg_type t ON t.oid = a.atttypid \
                 LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
                 WHERE a.attrelid = ", relation!(), " AND a.attnum > 0 AND NOT a.attisdropped \
                 ORDER BY a.attnum"
                ),
                params,
            ),
            client.query_typed(
                // An index that is not valid (a `CREATE UNIQUE INDEX
                // CONCURRENTLY` that failed) does not hold its rows unique,
                // whatever it is called. The last column is the key's
                // columns by name, or NULL when an entry is not one whole
                // column compared as the column compares: an expression (0
                // in `indkey`), another collation than the column's, or an
                // operator class that is not the default one. The text
                // `pg_get_indexdef` shows tells none of these apart.
                concat!(
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
                 WHERE i.indrelid = ", relation!(), " ORDER BY ic.relname"
                ),
                params,
            ),
            client.query_typed(
                concat!(
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
                 WHERE con.conrelid = ", relation!(), " AND con.contype = 'f' ORDER BY con.conname"
                ),
                params,
            ),
            // The keys that point at the table: the same catalog, read from
            // the other end. A partitioned table that has such a key has a
            // row for each of its partitions too, all pointing here: only the
            // key itself is listed. The copies name their parent in a column
            // servers before version 11 do not have.
            client.query_typed(
                concat!(
                    "SELECT con.conname::text, cn.nspname::text, cc.relname::text, \
                        ARRAY(SELECT a.attname::text FROM unnest(con.conkey) WITH ORDINALITY AS k(attnum, ord) \
                              JOIN pg_attribute a ON a.attrelid = con.conrelid AND a.attnum = k.attnum \
                              ORDER BY k.ord), \
                        ARRAY(SELECT a.attname::text FROM unnest(con.confkey) WITH ORDINALITY AS k(attnum, ord) \
                              JOIN pg_attribute a ON a.attrelid = con.confrelid AND a.attnum = k.attnum \
                              ORDER BY k.ord), \
                        con.confdeltype::text \
                 FROM pg_constraint con \
                 JOIN pg_class cc ON cc.oid = con.conrelid \
                 JOIN pg_namespace cn ON cn.oid = cc.relnamespace \
                 WHERE con.confrelid = ", relation!(), " AND con.contype = 'f' \
                   AND COALESCE((to_jsonb(con) ->> 'conparentid')::oid, 0) = 0 \
                 ORDER BY cn.nspname, cc.relname, con.conname"
                ),
                params,
            ),
            // What the table's first page builds its statement from, which
            // it then need not ask first (see `fetch_rows`).
            client.query_typed(SHAPE_SQL, params),
        )
        .map_err(query_error)?;
        drop(client);
        self.remember(object, &Shape::from_rows(&shape)?);
        // PostgreSQL keeps no empty comment (`COMMENT ON ... IS ''` drops
        // it), and an empty one would be none.
        let comment: Option<String> = match relation.first() {
            Some(row) => column(row, 0)?,
            None => {
                return Err(Error::query(format!(
                    "no such table or view: {}",
                    object.name
                )));
            }
        };
        let comment = comment.filter(|comment| !comment.is_empty());
        let columns: Vec<_> = columns
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
                    // An identity, of either kind, or a default that draws
                    // on the sequence the column owns: a serial's. A
                    // default that draws on some other sequence is a
                    // default like any other, and a serial whose default
                    // was dropped or set to something else keeps its
                    // sequence and is numbered by nobody.
                    identity: column(row, 8)?,
                })
            })
            .collect::<Result<_>>()?;
        let indexes: Vec<_> = indexes
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
        let foreign_keys: Vec<_> = foreign_keys
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
        let referenced_by: Vec<_> = referenced_by
            .iter()
            .map(|row| {
                let on_delete: String = column(row, 5)?;
                Ok(ReferencingKey {
                    name: Some(column(row, 0)?),
                    schema: column(row, 1)?,
                    table: column(row, 2)?,
                    columns: column(row, 3)?,
                    ref_columns: column(row, 4)?,
                    on_delete: action(&on_delete).into(),
                })
            })
            .collect::<Result<_>>()?;
        // The key is the primary index's columns, in key order. An entry
        // the index rows name no column for, which PostgreSQL does not let
        // a primary key have, sends the key's own query.
        let primary_key = match indexes.iter().find(|index: &&IndexInfo| index.primary) {
            None => Vec::new(),
            Some(IndexInfo {
                key_columns: Some(columns),
                ..
            }) => columns.clone(),
            Some(_) => self.primary_key(object).await?,
        };
        Ok(Structure {
            columns,
            primary_key,
            indexes,
            foreign_keys,
            referenced_by,
            comment,
        })
    }

    /// One page, read as [`read_statement`] reads: prepared (which refuses
    /// a second statement and yields the column types), then run through
    /// the simple-query protocol (every value as text) inside a read-only
    /// transaction, all in one round trip.
    ///
    /// The statement is built from the table's shape as this session last
    /// read it, and the shape is read again in the same flight, just before
    /// the page. A page whose statement the shape just read would not have
    /// built (a key or a `bytea` column that changed) is read again, from
    /// that shape: what stands is always the page of the shape read just
    /// before it, as when the shape is asked first. A table this session
    /// has not read the shape of yet has it asked first: one round trip
    /// more.
    async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        let limit = query.limit as usize;
        let client = self.client.lock().await;
        let (mut shape, mut checked) = match self.known_shape(&query.object) {
            Some(shape) => (shape, false),
            None => {
                let shape = table_shape(&client, &query.object).await?;
                self.remember(&query.object, &shape);
                (shape, true)
            }
        };
        loop {
            let (key, binary) = shape.for_query(query);
            let sql = Dialect::Postgres.select_rows(query, &key, &binary)?;
            // Every value is a literal in the text, so there is nothing to
            // bind.
            debug_assert!(sql.params.is_empty());
            let started = Instant::now();
            let read = if checked {
                read_statement(&client, &sql.text).await
            } else {
                let (fresh, read) =
                    read_alongside(&client, &sql.text, table_shape(&client, &query.object))
                        .await;
                let fresh = fresh?;
                self.remember(&query.object, &fresh);
                checked = true;
                if fresh.for_query(query) != (key.clone(), binary) {
                    // The page may be wrong, or have failed, for the shape
                    // it was built from.
                    shape = fresh;
                    continue;
                }
                read
            };
            let (columns, messages) = read?;
            let mut rows = kept_rows(&messages, &columns, limit)?;
            let has_more = rows.len() > limit;
            rows.truncate(limit);
            return Ok(RowPage {
                columns,
                rows,
                has_more,
                ordered_by_key: !key.is_empty(),
                elapsed: started.elapsed(),
            });
        }
    }

    async fn export_statement(&self, query: &RowQuery, columns: &[String]) -> Result<String> {
        let (key, binary) = self.shape(query).await?;
        Dialect::Postgres.shown_export(query, &key, &binary, columns)
    }

    /// An export's rows through a cursor in a read-only transaction:
    /// prepared first (which types the columns and refuses a second
    /// statement), declared, then fetched a batch at a time through the
    /// simple-query protocol, so the server holds the rest.
    async fn stream_rows(
        &self,
        query: &RowQuery,
        columns: &[String],
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        let (key, binary) = self.shape(query).await?;
        let sql = Dialect::Postgres.export_rows(query, &key, &binary, columns)?;
        // Every value is a literal in the text, so there is nothing to bind.
        debug_assert!(sql.params.is_empty());
        self.stream_text(&sql.text, batch, to, stop).await
    }

    /// A statement of the person's, as `stream_rows` streams a table's.
    async fn stream_statement(
        &self,
        text: &str,
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        self.stream_text(text, batch, to, stop).await
    }

    async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        let binary = self.binary_columns(query).await?;
        let sql = Dialect::Postgres.count_rows(query, &binary)?;
        let client = self.client.lock().await;
        let (_, messages) = read_statement(&client, &sql.text).await?;
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

    async fn lookup(&self, lookup: &Lookup) -> Result<LookupPage> {
        let sql = Dialect::Postgres.lookup(lookup);
        let (_, rows, _) = self.read(&sql, lookup.limit as usize).await?;
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
        let client = self.client.lock().await;
        hidden::catalog(&client, masked).await
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

    /// What a [`trip_counted`] session waited on the server for.
    pub(super) struct Trips(Arc<std::sync::atomic::AtomicUsize>);

    impl Trips {
        pub(super) fn count(&self) -> usize {
            self.0.load(Ordering::SeqCst)
        }
    }

    /// A read-only session through a proxy in front of the test server,
    /// and a count of its round trips: the times it sent a query, or a
    /// step of the extended protocol up to its `Sync`, while it waited on
    /// no answer. What it sends together counts once: the proxy holds back
    /// the server's answers a little, so it all arrives before they do. A
    /// statement's close that goes out on its own is waited on by nobody
    /// and does not count.
    pub(super) async fn trip_counted(url: &str) -> (Conn, Trips) {
        trip_counted_as(url, Access::ReadOnly).await
    }

    /// A [`trip_counted`] session opened as `access`.
    pub(super) async fn trip_counted_as(url: &str, access: Access) -> (Conn, Trips) {
        use std::collections::VecDeque;
        use std::sync::atomic::AtomicUsize;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        /// One message off `from`: its kind byte, unless it is the first
        /// one a client sends, and its length and body.
        async fn message(
            from: &mut (impl AsyncReadExt + Unpin),
            kind: bool,
        ) -> Option<(Option<u8>, Vec<u8>)> {
            let kind = if kind {
                Some(from.read_u8().await.ok()?)
            } else {
                None
            };
            let length = from.read_u32().await.ok()?;
            let mut message = Vec::from_iter(kind);
            message.extend_from_slice(&length.to_be_bytes());
            let start = message.len();
            message.resize(start + length as usize - 4, 0);
            from.read_exact(&mut message[start..]).await.ok()?;
            Some((kind, message))
        }

        const HELD: Duration = Duration::from_millis(20);
        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
        spec.tls = crate::TlsMode::Disable;
        let server = (spec.host.clone(), spec.port);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        spec.host = "127.0.0.1".to_owned();
        spec.port = listener.local_addr().unwrap().port();
        let trips = Arc::new(AtomicUsize::new(0));
        // For each answer still to come, whether anyone waits on it.
        let waiting = Arc::new(std::sync::Mutex::new(VecDeque::<bool>::new()));
        let (count, answered) = (Arc::clone(&trips), Arc::clone(&waiting));
        tokio::spawn(async move {
            let (client, _) = listener.accept().await.unwrap();
            let upstream = tokio::net::TcpStream::connect(server).await.unwrap();
            let (mut from_client, mut to_client) = client.into_split();
            let (mut from_server, mut to_server) = upstream.into_split();
            let (held, mut released) = tokio::sync::mpsc::unbounded_channel();
            tokio::spawn(async move {
                while let Some((kind, message)) = message(&mut from_server, true).await {
                    let due = tokio::time::Instant::now() + HELD;
                    if held.send((due, kind == Some(b'Z'), message)).is_err() {
                        return;
                    }
                }
            });
            tokio::spawn(async move {
                while let Some((due, ready, message)) = released.recv().await {
                    tokio::time::sleep_until(due).await;
                    if ready {
                        answered.lock().unwrap().pop_front();
                    }
                    if to_client.write_all(&message).await.is_err() {
                        return;
                    }
                }
            });
            // The startup message has no kind byte; every one after it has.
            let mut first = true;
            // Whether the steps since the last `Sync` are more than closes.
            let mut asks = false;
            while let Some((kind, message)) = message(&mut from_client, !first).await {
                first = false;
                let ends = match kind {
                    Some(b'Q') => {
                        asks = false;
                        Some(true)
                    }
                    Some(b'S') => Some(std::mem::take(&mut asks)),
                    Some(b'C') => None,
                    _ => {
                        asks = true;
                        None
                    }
                };
                if let Some(asks) = ends {
                    let mut waiting = waiting.lock().unwrap();
                    if asks && !waiting.contains(&true) {
                        count.fetch_add(1, Ordering::SeqCst);
                    }
                    waiting.push_back(asks);
                }
                if to_server.write_all(&message).await.is_err() {
                    return;
                }
            }
        });
        let conn = Conn::connect(&spec, &secrets, None, access).await.unwrap();
        (conn, Trips(trips))
    }

    /// What `work` answered, and the round trips it took.
    async fn counted<T>(trips: &Trips, work: impl Future<Output = T>) -> (T, usize) {
        let before = trips.count();
        let done = work.await;
        (done, trips.count() - before)
    }

    /// The counter counts a prepare and its run twice, two queries one
    /// after another twice, and the same two sent together once.
    #[tokio::test]
    async fn the_trip_counter_counts_what_goes_out_together_once() {
        let Some(url) = test_url() else {
            return;
        };
        let (conn, trips) = trip_counted(&url).await;
        let client = conn.client.lock().await;
        let typed = || client.query_typed("SELECT 1", &[]);
        let (_, asked) = counted(&trips, client.query("SELECT 1", &[])).await;
        assert_eq!(asked, 2);
        let (_, asked) = counted(&trips, async {
            typed().await.unwrap();
            typed().await.unwrap();
        })
        .await;
        assert_eq!(asked, 2);
        let (_, asked) = counted(&trips, async {
            tokio::try_join!(typed(), typed()).unwrap()
        })
        .await;
        assert_eq!(asked, 1);
    }

    /// A table whose key is two columns, in another order than the
    /// table's, with a column the key's index includes, and a `bytea`
    /// column. Made once: two tests that create it at once can fail.
    async fn round_trips_table(url: &str) -> ObjectRef {
        static MADE: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
        MADE.get_or_init(|| make_round_trips_table(url)).await;
        ObjectRef::new("public", "round_trips")
    }

    async fn make_round_trips_table(url: &str) {
        let mut config: tokio_postgres::Config = url.parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        let (client, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(connection);
        client
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS round_trips ( \
                     id int, body bytea, shelf int, name text, \
                     PRIMARY KEY (shelf, id) INCLUDE (name)); \
                 INSERT INTO round_trips VALUES (1, '\\x00ff', 1, 'one'), (2, NULL, 1, 'two') \
                 ON CONFLICT DO NOTHING",
            )
            .await
            .unwrap();
    }

    /// Whether the session is outside any transaction a read began: the
    /// first statement of a transaction starts when the transaction does.
    async fn outside_a_transaction(conn: &Conn) -> bool {
        let client = conn.client.lock().await;
        let messages = client
            .simple_query("SELECT statement_timestamp() = transaction_timestamp()")
            .await
            .unwrap();
        first_text(&messages).as_deref() == Some("t")
    }

    /// A page asks the server once: the table's shape, the `BEGIN`, the
    /// prepare, the statement and its `ROLLBACK` go out together. Only the
    /// first page of a table the session knows nothing of asks for its
    /// shape first. A count and a lookup need no key: once, and a count
    /// that compares bytes asks for the `bytea` columns first.
    #[tokio::test]
    async fn a_page_asks_the_server_once() {
        let Some(url) = test_url() else {
            return;
        };
        let object = round_trips_table(&url).await;
        let (conn, trips) = trip_counted(&url).await;
        let page = RowQuery::new(object.clone(), 10);
        let mut by_bytes = page.clone();
        by_bytes.filters = vec![crate::Filter {
            column: "body".into(),
            op: crate::FilterOp::Eq,
            value: "0x00ff".into(),
        }];
        // A raw WHERE that ends in a comment does not comment out what
        // comes after it.
        let mut commented = page.clone();
        commented.raw_where = Some("id > 0 -- every row".into());
        let mut next = RowQuery::new(object.clone(), 1);
        next.offset = 1;

        let (first, asked) = counted(&trips, conn.fetch_rows(&page)).await;
        assert_eq!(first.unwrap().rows.len(), 2);
        assert_eq!(asked, 2);
        assert!(outside_a_transaction(&conn).await);

        for (query, rows) in [(&page, 2), (&by_bytes, 1), (&commented, 2), (&next, 1)] {
            let (fetched, asked) = counted(&trips, conn.fetch_rows(query)).await;
            let fetched = fetched.unwrap();
            assert_eq!(fetched.rows.len(), rows, "{query:?}");
            assert!(fetched.ordered_by_key);
            assert_eq!(asked, 1, "{query:?}");
            assert!(outside_a_transaction(&conn).await, "{query:?}");
        }
        // Ordered by the key, (shelf, id): the second row is id 2.
        let second = conn.fetch_rows(&next).await.unwrap();
        assert_eq!(second.rows[0][0], Value::Int(2));
        assert!(!second.has_more);

        let (shape, asked) = counted(&trips, conn.shape(&by_bytes)).await;
        assert_eq!(asked, 1);
        assert_eq!(
            shape.unwrap(),
            (
                vec!["shelf".to_owned(), "id".to_owned()],
                vec![("body".to_owned(), ColumnClass::Binary)]
            )
        );
        assert_eq!(conn.shape(&page).await.unwrap().1, Vec::new());

        for (query, expected, asks) in [(&page, 2, 1), (&by_bytes, 1, 2)] {
            let (count, asked) = counted(&trips, conn.count_rows(query)).await;
            assert_eq!(count.unwrap(), expected, "{query:?}");
            assert_eq!(asked, asks, "{query:?}");
        }
        let lookup = Lookup {
            object: object.clone(),
            key: "id".into(),
            label: Some("name".into()),
            part: crate::LookupPart::Matching(String::new()),
            limit: 10,
        };
        let (found, asked) = counted(&trips, conn.lookup(&lookup)).await;
        assert_eq!(found.unwrap().rows.len(), 2);
        assert_eq!(asked, 1);
        assert!(outside_a_transaction(&conn).await);
    }

    /// A page after the table's key changed, or lost the column the last
    /// page was ordered by, is the page of the table as it is now: the
    /// shape read with it says the statement was built from an old one,
    /// and the page is read again. Two round trips, then one again.
    #[tokio::test]
    async fn a_page_after_the_key_changed_is_read_again() {
        let Some(url) = test_url() else {
            return;
        };
        let mut config: tokio_postgres::Config = url.parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        let (other, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(connection);
        other
            .batch_execute(
                "DROP TABLE IF EXISTS key_changes; \
                 CREATE TABLE key_changes (a int PRIMARY KEY, b int NOT NULL, c text); \
                 INSERT INTO key_changes VALUES (1, 2, 'one'), (2, 1, 'two')",
            )
            .await
            .unwrap();
        let object = ObjectRef::new("public", "key_changes");
        let (conn, trips) = trip_counted(&url).await;
        let query = RowQuery::new(object.clone(), 10);
        let firsts = |page: &RowPage| {
            page.rows
                .iter()
                .map(|row| row[0].clone())
                .collect::<Vec<_>>()
        };
        conn.describe(&object).await.unwrap();
        let (page, asked) = counted(&trips, conn.fetch_rows(&query)).await;
        assert_eq!(firsts(&page.unwrap()), [Value::Int(1), Value::Int(2)]);
        assert_eq!(asked, 1);

        // The key is now `b`: the rows come in its order.
        other
            .batch_execute(
                "ALTER TABLE key_changes DROP CONSTRAINT key_changes_pkey, ADD PRIMARY KEY (b)",
            )
            .await
            .unwrap();
        let (page, asked) = counted(&trips, conn.fetch_rows(&query)).await;
        let page = page.unwrap();
        assert_eq!(firsts(&page), [Value::Int(2), Value::Int(1)]);
        assert!(page.ordered_by_key);
        assert_eq!(asked, 2);
        assert!(outside_a_transaction(&conn).await);

        // `b` is gone, and with it the key: the old statement fails, and
        // the page is read without one.
        other
            .batch_execute("ALTER TABLE key_changes DROP COLUMN b")
            .await
            .unwrap();
        let (page, asked) = counted(&trips, conn.fetch_rows(&query)).await;
        let page = page.unwrap();
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.columns.len(), 2);
        assert!(!page.ordered_by_key);
        assert_eq!(asked, 2);
        assert!(outside_a_transaction(&conn).await);

        let (page, asked) = counted(&trips, conn.fetch_rows(&query)).await;
        assert_eq!(page.unwrap().rows.len(), 2);
        assert_eq!(asked, 1);
        other.batch_execute("DROP TABLE key_changes").await.unwrap();
    }

    /// A raw WHERE that ends the read's transaction and writes after it
    /// writes nothing, in a session that may write: the prepare refuses a
    /// text of more than one statement, which fails the transaction it
    /// runs in, and nothing of the run goes on in a failed one. All of it
    /// in one round trip.
    #[tokio::test]
    async fn a_raw_where_that_commits_writes_nothing() {
        let Some(url) = test_url() else {
            return;
        };
        let object = round_trips_table(&url).await;
        let (conn, trips) = trip_counted_as(&url, Access::Writable).await;
        let mut query = RowQuery::new(object.clone(), 10);
        conn.fetch_rows(&query).await.unwrap();
        query.raw_where = Some(
            "true); COMMIT; CREATE TEMP TABLE injected (n int); SELECT * FROM round_trips WHERE (true"
                .into(),
        );
        let (fetched, asked) = counted(&trips, conn.fetch_rows(&query)).await;
        let error = fetched.unwrap_err().to_string();
        assert!(error.contains("multiple commands"), "{error}");
        assert_eq!(asked, 1);
        assert!(outside_a_transaction(&conn).await);
        let client = conn.client.lock().await;
        let messages = client
            .simple_query("SELECT to_regclass('pg_temp.injected') IS NULL")
            .await
            .unwrap();
        assert_eq!(first_text(&messages).as_deref(), Some("t"));
    }

    /// The guard before a read's statement fails outside a transaction
    /// block, as after a `BEGIN` that failed, and in a failed one, as after
    /// a prepare that failed: the statement after it does not run.
    #[tokio::test]
    async fn the_read_guard_runs_nothing_outside_a_good_transaction() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session_with(&url, Access::Writable, "").await;
        let client = conn.client.lock().await;
        let run = format!("{READ_GUARD};\nCREATE TEMP TABLE guarded (n int)");
        let outside = client.simple_query(&run).await.unwrap_err();
        assert_eq!(outside.code(), Some(&SqlState::NO_ACTIVE_SQL_TRANSACTION));
        client.batch_execute("BEGIN").await.unwrap();
        assert!(client.batch_execute("SELECT 1 / 0").await.is_err());
        let failed = client.simple_query(&run).await.unwrap_err();
        assert_eq!(failed.code(), Some(&SqlState::IN_FAILED_SQL_TRANSACTION));
        client.batch_execute("ROLLBACK").await.unwrap();
        let messages = client
            .simple_query("SELECT to_regclass('pg_temp.guarded') IS NULL")
            .await
            .unwrap();
        assert_eq!(first_text(&messages).as_deref(), Some("t"));
    }

    /// A read dropped between its `BEGIN` and its statement leaves no
    /// transaction behind: the proxy holds the prepare's answer back, and
    /// the read is dropped while it waits. A writable session's next
    /// script writes: its transaction is its own, not the read's read-only
    /// one. And the session reads on.
    #[tokio::test]
    async fn a_dropped_read_leaves_no_transaction_behind() {
        let Some(url) = test_url() else {
            return;
        };
        for access in [Access::ReadOnly, Access::Writable] {
            let (conn, _) = trip_counted_as(&url, access).await;
            let read = async {
                let client = conn.client.lock().await;
                read_statement(&client, "SELECT 1").await
            };
            assert!(
                tokio::time::timeout(Duration::from_millis(10), read)
                    .await
                    .is_err()
            );
            assert!(outside_a_transaction(&conn).await, "{access:?}");
            if access == Access::Writable {
                let texts = ["CREATE TEMP TABLE dropped_read (n int)".to_owned()];
                let outcome = conn
                    .script(&texts, 10, ScriptMode::Write, &StopFlag::new())
                    .await
                    .unwrap();
                assert!(
                    matches!(
                        outcome.results[..],
                        [crate::StatementResult {
                            outcome: crate::StatementOutcome::Done { .. },
                            ..
                        }]
                    ),
                    "{outcome:?}"
                );
            }
            {
                let client = conn.client.lock().await;
                let (_, messages) = read_statement(&client, "SELECT 2").await.unwrap();
                assert_eq!(first_text(&messages).as_deref(), Some("2"));
            }
        }
    }

    /// A read that fails is rolled back by the `ROLLBACK` sent with it: no
    /// round trip more. A second statement is refused by the prepare,
    /// before anything runs, and an error of the statement's own fails the
    /// transaction the `ROLLBACK` ends.
    #[tokio::test]
    async fn a_failed_read_rolls_back_and_the_next_one_reads() {
        let Some(url) = test_url() else {
            return;
        };
        let object = round_trips_table(&url).await;
        let (conn, trips) = trip_counted(&url).await;
        conn.describe(&object).await.unwrap();
        for raw in ["true; SELECT 1", "1 / (id - id) = 0"] {
            let mut query = RowQuery::new(object.clone(), 10);
            query.raw_where = Some(raw.into());
            let (fetched, asked) = counted(&trips, conn.fetch_rows(&query)).await;
            assert!(fetched.is_err(), "{raw}");
            assert_eq!(asked, 1, "{raw}");
            assert!(outside_a_transaction(&conn).await, "{raw}");
            let page = RowQuery::new(object.clone(), 10);
            assert_eq!(conn.fetch_rows(&page).await.unwrap().rows.len(), 2);
        }
    }

    /// The structure asks the server once: its queries go out together.
    /// A table that is not there is still said to be missing.
    #[tokio::test]
    async fn the_structure_asks_the_server_once() {
        let Some(url) = test_url() else {
            return;
        };
        let object = round_trips_table(&url).await;
        let (conn, trips) = trip_counted(&url).await;
        let (structure, asked) = counted(&trips, conn.describe(&object)).await;
        let structure = structure.unwrap();
        assert_eq!(structure.primary_key, ["shelf", "id"]);
        assert_eq!(
            structure
                .columns
                .iter()
                .map(|column| column.name.as_str())
                .collect::<Vec<_>>(),
            ["id", "body", "shelf", "name"]
        );
        assert_eq!(asked, 1);
        let missing = ObjectRef::new("public", "no_round_trips");
        let (structure, asked) = counted(&trips, conn.describe(&missing)).await;
        let error = structure.unwrap_err().to_string();
        assert!(error.contains("no such table or view"), "{error}");
        assert_eq!(asked, 1);
    }

    /// Opening a table asks the server twice: once for its structure,
    /// which reads the table's shape too, and once for its first page.
    #[tokio::test]
    async fn opening_a_table_asks_the_server_twice() {
        let Some(url) = test_url() else {
            return;
        };
        let object = round_trips_table(&url).await;
        let (conn, trips) = trip_counted(&url).await;
        let (opened, asked) = counted(&trips, async {
            let structure = conn.describe(&object).await.unwrap();
            let page = conn
                .fetch_rows(&RowQuery::new(object.clone(), 10))
                .await
                .unwrap();
            (structure, page)
        })
        .await;
        assert_eq!(opened.1.rows.len(), 2);
        assert!(opened.1.ordered_by_key);
        assert_eq!(asked, 2);
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
