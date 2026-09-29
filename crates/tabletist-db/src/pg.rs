//! PostgreSQL. Catalog queries use the extended protocol with typed results;
//! row queries use the simple-query protocol (Task 5), which sends every
//! value as text.

use std::time::{Duration, Instant};

use tokio_postgres::SimpleQueryMessage;

use crate::{
    ColumnInfo, ColumnMeta, ConnectSpec, Dialect, Error, ForeignKeyInfo, IndexInfo, ObjectInfo,
    ObjectKind, ObjectRef, Result, RowPage, RowQuery, Secrets, Structure, Value, ValueKind,
    value_from_pg_text,
};
use tokio_postgres::error::SqlState;
use tokio_postgres_rustls::MakeRustlsConnect;

pub struct Conn {
    pub(crate) client: tokio::sync::Mutex<tokio_postgres::Client>,
    pub(crate) cancel: tokio_postgres::CancelToken,
    pub(crate) tls: MakeRustlsConnect,
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
        let connecting = config.connect(tls.clone());
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
        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    pub async fn list_schemas(&self) -> Result<Vec<String>> {
        let rows = self
            .catalog(
                "SELECT nspname::text FROM pg_namespace ORDER BY nspname",
                &[],
            )
            .await?;
        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    pub async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        let rows = self
            .catalog(
                "SELECT c.relname::text, c.relkind::text, c.reltuples::float8 \
                 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
                 WHERE n.nspname = $1 AND c.relkind IN ('r', 'p', 'f', 'v', 'm') \
                 ORDER BY c.relname",
                &[&schema],
            )
            .await?;
        Ok(rows
            .iter()
            .map(|row| {
                let kind: String = row.get(1);
                let tuples: f64 = row.get(2);
                ObjectInfo {
                    name: row.get(0),
                    kind: match kind.as_str() {
                        "v" => ObjectKind::View,
                        "m" => ObjectKind::MaterializedView,
                        _ => ObjectKind::Table,
                    },
                    // -1 means never analyzed (PostgreSQL 14+).
                    estimated_rows: (tuples >= 0.0).then_some(tuples as u64),
                }
            })
            .collect())
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
            .map(|row| row.get(0))
            .ok_or_else(|| Error::query(format!("no such table or view: {}", object.name)))
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
        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    pub async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        let oid = self.relation(object).await?;
        let columns = self
            .catalog(
                "SELECT a.attname::text, format_type(a.atttypid, a.atttypmod), NOT a.attnotnull, \
                        pg_get_expr(d.adbin, d.adrelid), col_description(a.attrelid, a.attnum) \
                 FROM pg_attribute a \
                 LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
                 WHERE a.attrelid = $1 AND a.attnum > 0 AND NOT a.attisdropped \
                 ORDER BY a.attnum",
                &[&oid],
            )
            .await?
            .iter()
            .map(|row| ColumnInfo {
                name: row.get(0),
                type_name: row.get(1),
                nullable: row.get(2),
                default: row.get(3),
                comment: row.get(4),
            })
            .collect();
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
            .map(|row| IndexInfo {
                name: row.get(0),
                unique: row.get(1),
                primary: row.get(2),
                method: Some(row.get(3)),
                columns: row.get(4),
            })
            .collect();
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
                let on_update: String = row.get(5);
                let on_delete: String = row.get(6);
                ForeignKeyInfo {
                    name: Some(row.get(0)),
                    columns: row.get(1),
                    ref_schema: row.get(2),
                    ref_table: row.get(3),
                    ref_columns: row.get(4),
                    on_update: action(&on_update).into(),
                    on_delete: action(&on_delete).into(),
                }
            })
            .collect();
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
                        .map(|(index, column)| match row.get(index) {
                            None => Value::Null,
                            Some(text) => value_from_pg_text(&column.type_name, text),
                        })
                        .collect(),
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
                SimpleQueryMessage::Row(row) => row.get(0).and_then(|count| count.parse().ok()),
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
}
