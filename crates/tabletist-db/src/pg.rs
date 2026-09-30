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

use crate::{
    ColumnInfo, ColumnMeta, ConnectSpec, Dialect, Error, ForeignKeyInfo, IndexInfo, MAX_LISTED,
    ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery, Secrets, Structure, Value,
    ValueKind, value_from_pg_text,
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

impl Conn {
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
        let columns: Vec<ColumnMeta> = statement
            .columns()
            .iter()
            .map(|column| ColumnMeta {
                name: column.name().to_owned(),
                type_name: column.type_().name().to_owned(),
                kind: ValueKind::from_pg_type(column.type_().name()),
            })
            .collect();
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
                rows.push(
                    columns
                        .iter()
                        .enumerate()
                        .map(|(index, column)| {
                            Ok(match row.try_get(index).map_err(unexpected)? {
                                None => Value::Null,
                                Some(text) => value_from_pg_text(&column.type_name, text),
                            })
                        })
                        .collect::<Result<_>>()?,
                );
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
