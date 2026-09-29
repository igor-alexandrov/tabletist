//! MySQL 8.0+. Rows and catalog queries use prepared statements (the binary
//! protocol): typed values, column types, and never more than one statement.
//! Rows are read inside a read-only transaction.

use std::borrow::Cow;
use std::path::Path;
use std::time::{Duration, Instant};

use mysql_async::consts::{ColumnFlags, ColumnType};
use mysql_async::prelude::Queryable;
use mysql_async::{DriverError, IoError, Opts, OptsBuilder, Params, SslOpts, TxOpts};

use crate::{
    ColumnInfo, ColumnMeta, ConnectSpec, Dialect, Error, ForeignKeyInfo, IndexInfo, ObjectInfo,
    ObjectKind, ObjectRef, Result, RowPage, RowQuery, Secrets, Structure, TlsMode, Value,
    ValueKind,
};

/// MySQL's `binary` character set: bytes, not text.
const BINARY_CHARSET: u16 = 63;
const ACCESS_DENIED: u16 = 1045;
const QUERY_INTERRUPTED: u16 = 1317;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Conn {
    pub(crate) conn: tokio::sync::Mutex<mysql_async::Conn>,
    /// For KILL QUERY from a second connection.
    pub(crate) opts: Opts,
    /// This session's thread id on the server.
    pub(crate) id: u32,
}

impl Conn {
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
        // Fixed statements: safe to send through the text protocol.
        conn.query_drop("SET SESSION TRANSACTION READ ONLY")
            .await
            .map_err(query_error)?;
        conn.query_drop(
            "SET SESSION sql_mode = REPLACE(@@SESSION.sql_mode, 'NO_BACKSLASH_ESCAPES', '')",
        )
        .await
        .map_err(query_error)?;
        let id = conn.id();
        Ok(Self {
            conn: tokio::sync::Mutex::new(conn),
            opts,
            id,
        })
    }

    /// A catalog query: fixed SQL, prepared, typed results.
    async fn catalog<T>(&self, sql: &str, params: impl Into<Params> + Send) -> Result<Vec<T>>
    where
        T: mysql_async::prelude::FromRow + Send + 'static,
    {
        let mut conn = self.conn.lock().await;
        conn.exec(sql, params).await.map_err(query_error)
    }

    pub async fn list_schemas(&self) -> Result<Vec<String>> {
        self.catalog(
            "SELECT schema_name FROM information_schema.schemata ORDER BY schema_name",
            Params::Empty,
        )
        .await
    }

    pub async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        let rows: Vec<(String, String, Option<u64>)> = self
            .catalog(
                "SELECT table_name, table_type, table_rows FROM information_schema.tables \
                 WHERE table_schema = ? ORDER BY table_name",
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
            .exec_first(sql.text.as_str(), params(&sql.params))
            .await
            .map_err(query_error);
        let count: Option<u64> = finish(transaction, outcome).await?;
        count.ok_or_else(|| Error::query("the count returned no number"))
    }
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
    let columns: Vec<ColumnMeta> = result
        .columns_ref()
        .iter()
        .map(|column| {
            let type_name = type_name(column.column_type(), column.flags(), column.character_set());
            ColumnMeta {
                name: column.name_str().into_owned(),
                kind: kind(&type_name),
                type_name: type_name.into_owned(),
            }
        })
        .collect();
    let mut rows = Vec::new();
    while let Some(row) = result.next().await.map_err(query_error)? {
        rows.push(
            row.unwrap()
                .into_iter()
                .zip(&columns)
                .map(|(cell, column)| value(cell, &column.type_name))
                .collect(),
        );
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
    let base = match ca_file {
        Some(path) => SslOpts::default().with_root_certs(vec![path.to_path_buf().into()]),
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
