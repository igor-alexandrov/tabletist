//! A spike: ClickHouse over its HTTP interface, on ureq. It is not an
//! engine yet (no `Driver` names it). It runs one statement and reads a
//! page of its rows, so that we can choose how Tabletist talks to
//! ClickHouse: see
//! `docs/superpowers/specs/2026-10-30-clickhouse-connection-research.md`.
//!
//! Each statement is one POST. Rows come back as
//! `TabSeparatedWithNamesAndTypes`, read a line at a time, so a page stops
//! reading where it is full. Every statement carries its own `query_id`,
//! which is how [`Canceller`] stops it: `KILL QUERY` on another request,
//! for a dropped connection does not stop a query.

use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::{ColumnMeta, Error, Result, RowPage, Value, ValueKind};

/// Where a ClickHouse server's HTTP interface is, and who to be there.
#[derive(Clone)]
pub struct Server {
    /// `http://host:8123` or `https://host:8443`.
    pub url: String,
    pub user: String,
    pub password: String,
    /// Empty for the user's default database.
    pub database: String,
}

/// A session with one server. Cloning it shares its connections.
#[derive(Clone)]
pub struct Http {
    agent: ureq::Agent,
    server: Arc<Server>,
    /// The `query_id` of the statement running now, which [`Canceller`]
    /// kills. One slot for every clone: of two statements run at once,
    /// only the later is cancelled. An engine gives each its own.
    running: Arc<Mutex<Option<String>>>,
}

/// Stops the statement an [`Http`] runs. Doing so when nothing runs does
/// nothing.
#[derive(Clone)]
pub struct Canceller(Http);

impl Http {
    pub fn new(server: Server) -> Result<Self> {
        url::Url::parse(&server.url)
            .map_err(|error| Error::InvalidSpec(format!("not a ClickHouse URL: {error}")))?;
        let config = ureq::Agent::config_builder()
            // A status other than 200 is ClickHouse's error, read below.
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_connect(Some(Duration::from_secs(15)))
            .build();
        Ok(Self {
            agent: config.into(),
            server: Arc::new(server),
            running: Arc::default(),
        })
    }

    pub fn canceller(&self) -> Canceller {
        Canceller(self.clone())
    }

    pub async fn server_version(&self) -> Result<String> {
        let page = self.select_page("SELECT version()".into(), 1).await?;
        match page.rows.first().and_then(|row| row.first()) {
            Some(Value::Text(version)) => Ok(version.to_string()),
            _ => Err(Error::query("the server did not say its version")),
        }
    }

    /// Runs `sql`, which must not name a `FORMAT`, read-only (the
    /// server's `readonly = 2`: no writes, no DDL, settings allowed), and
    /// keeps its first `limit` rows.
    pub async fn select_page(&self, sql: String, limit: u32) -> Result<RowPage> {
        let http = self.clone();
        tokio::task::spawn_blocking(move || http.select_page_blocking(&sql, limit))
            .await
            .map_err(|error| Error::query(format!("the ClickHouse reader stopped: {error}")))?
    }

    fn select_page_blocking(&self, sql: &str, limit: u32) -> Result<RowPage> {
        let started = Instant::now();
        let id = query_id();
        *self.running.lock().unwrap_or_else(|e| e.into_inner()) = Some(id.clone());
        let read = self.read_rows(sql, limit, &id);
        self.running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take_if(|running| *running == id);
        let (columns, mut rows) = read?;
        let has_more = rows.len() > limit as usize;
        rows.truncate(limit as usize);
        Ok(RowPage {
            columns,
            rows,
            has_more,
            ordered_by_key: false,
            elapsed: started.elapsed(),
        })
    }

    fn read_rows(&self, sql: &str, limit: u32, id: &str) -> Result<(Vec<ColumnMeta>, Rows)> {
        // One row past the page says whether another page exists. The
        // server stops at about that many too, a block past it at most.
        let wanted = u64::from(limit) + 1;
        let wanted_text = wanted.to_string();
        let response = self.post(
            sql,
            &[
                ("query_id", id),
                ("readonly", "2"),
                ("default_format", "TabSeparatedWithNamesAndTypes"),
                ("max_result_rows", &wanted_text),
                ("result_overflow_mode", "break"),
            ],
        )?;
        let tag = header(&response, "x-clickhouse-exception-tag");
        let body = BufReader::new(response.into_body().into_reader());
        read_tsv(body, wanted, tag.as_deref())
    }

    /// Sends `sql` with `settings` in the URL. A status other than 200 is
    /// the server's error.
    fn post(
        &self,
        sql: &str,
        settings: &[(&str, &str)],
    ) -> Result<ureq::http::Response<ureq::Body>> {
        let mut url = url::Url::parse(&self.server.url)
            .map_err(|error| Error::InvalidSpec(format!("not a ClickHouse URL: {error}")))?;
        url.query_pairs_mut().extend_pairs(settings);
        let mut request = self
            .agent
            .post(url.as_str())
            .header("X-ClickHouse-User", &self.server.user)
            .header("X-ClickHouse-Key", &self.server.password);
        if !self.server.database.is_empty() {
            request = request.header("X-ClickHouse-Database", &self.server.database);
        }
        let mut response = request.send(sql).map_err(sent_error)?;
        if response.status() == 200 {
            return Ok(response);
        }
        let code = header(&response, "x-clickhouse-exception-code");
        let message = response
            .body_mut()
            .with_config()
            .limit(64 * 1024)
            .lossy_utf8(true)
            .read_to_string()
            .unwrap_or_default();
        Err(server_error(code.as_deref(), &message))
    }
}

impl Canceller {
    pub async fn cancel(&self) -> Result<()> {
        let http = self.0.clone();
        tokio::task::spawn_blocking(move || http.kill())
            .await
            .map_err(|error| Error::query(format!("could not cancel: {error}")))?
    }
}

impl Http {
    fn kill(&self) -> Result<()> {
        let Some(id) = self
            .running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        else {
            return Ok(());
        };
        // `id` is ours (`query_id`): letters, digits and dashes only.
        let sql = format!("KILL QUERY WHERE query_id = '{id}' ASYNC FORMAT Null");
        let mut response = self.post(&sql, &[])?;
        response
            .body_mut()
            .read_to_vec()
            .map_err(|error| Error::Connect(format!("could not cancel: {error}")))?;
        Ok(())
    }
}

type Rows = Vec<Vec<Value>>;

/// A `query_id` no other statement of this process has.
fn query_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "tabletist-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

fn header<B>(response: &ureq::http::Response<B>, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn sent_error(error: ureq::Error) -> Error {
    match error {
        ureq::Error::Timeout(_) => Error::Timeout,
        ureq::Error::Tls(message) => Error::Tls(message.to_owned()),
        other => Error::Connect(other.to_string()),
    }
}

/// The error ClickHouse wrote, with its numeric code where it gave one
/// (in a header before the rows, in the message after).
fn server_error(code: Option<&str>, message: &str) -> Error {
    let message = message.trim();
    let code = code
        .map(str::to_owned)
        .or_else(|| {
            let rest = message.strip_prefix("Code: ")?;
            let end = rest.find('.')?;
            Some(rest[..end].to_owned())
        })
        .filter(|code| !code.is_empty() && code.bytes().all(|b| b.is_ascii_digit()));
    match code.as_deref() {
        // QUERY_WAS_CANCELLED
        Some("394") => Error::Cancelled,
        // TIMEOUT_EXCEEDED
        Some("159") => Error::Timeout,
        // AUTHENTICATION_FAILED, REQUIRED_PASSWORD
        Some("516" | "194") => Error::Auth(message.to_owned()),
        _ => Error::Query {
            code,
            message: message.to_owned(),
            detail: None,
            hint: None,
            named: Box::default(),
        },
    }
}

/// Reads `TabSeparatedWithNamesAndTypes`: names, types, then at most
/// `wanted` rows, and stops reading there.
///
/// An error after the rows began is written into the body: a line ending
/// in `\r` (which a row cannot hold, for TSV escapes it) starts it.
/// Newer servers (26.10) put `__exception__` lines and the
/// `X-ClickHouse-Exception-Tag` around it; older ones (25.8) write
/// `__exception__` and the message and close the connection.
fn read_tsv(
    mut body: impl BufRead,
    wanted: u64,
    tag: Option<&str>,
) -> Result<(Vec<ColumnMeta>, Rows)> {
    let mut line = Vec::new();
    let mut next = |line: &mut Vec<u8>| -> Result<Line> {
        line.clear();
        match body.read_until(b'\n', line) {
            Ok(0) => Ok(Line::End),
            Ok(_) if line.ends_with(b"\r\n") => {
                let mut rest = std::mem::take(line);
                // The old servers close the connection after the message.
                let _ = body.read_to_end(&mut rest);
                Ok(Line::Exception(rest))
            }
            Ok(_) if line.last() == Some(&b'\n') => {
                line.pop();
                Ok(Line::Row)
            }
            Ok(_) => Err(Error::ConnectionLost("the server stopped mid-row".into())),
            Err(error) => Err(Error::ConnectionLost(error.to_string())),
        }
    };
    let mut header = |line: &mut Vec<u8>| -> Result<Vec<String>> {
        match next(line)? {
            Line::Row => Ok(fields(line)
                .map(|field| String::from_utf8_lossy(&unescape(field)).into_owned())
                .collect()),
            Line::Exception(rest) => Err(exception(&rest, tag)),
            Line::End => Err(Error::query("ClickHouse sent no column names")),
        }
    };
    let names = header(&mut line)?;
    let types = header(&mut line)?;
    if names.len() != types.len() {
        return Err(Error::query(
            "ClickHouse sent names and types that disagree",
        ));
    }
    let columns: Vec<ColumnMeta> = names
        .into_iter()
        .zip(types)
        .map(|(name, type_name)| ColumnMeta {
            kind: kind(&type_name),
            name,
            type_name,
        })
        .collect();
    let mut rows = Vec::new();
    while (rows.len() as u64) < wanted {
        match next(&mut line)? {
            Line::Row => {
                let row: Vec<Value> = fields(&line)
                    .zip(&columns)
                    .map(|(field, column)| value(&column.type_name, field))
                    .collect();
                if row.len() != columns.len() {
                    return Err(Error::query("ClickHouse sent a row of another width"));
                }
                rows.push(row);
            }
            Line::Exception(rest) => return Err(exception(&rest, tag)),
            Line::End => break,
        }
    }
    Ok((columns, rows))
}

enum Line {
    Row,
    Exception(Vec<u8>),
    End,
}

fn fields(line: &[u8]) -> impl Iterator<Item = &[u8]> {
    line.split(|&byte| byte == b'\t')
}

/// The message of an error written into the body, from the line that
/// began it to the end.
fn exception(rest: &[u8], tag: Option<&str>) -> Error {
    let text = String::from_utf8_lossy(rest);
    let message = text
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.is_empty() && *line != "__exception__" && Some(*line) != tag)
        // The tagged form ends with the message's length and the tag.
        .filter(|line| tag.is_none_or(|tag| !line.ends_with(&format!(" {tag}"))))
        .collect::<Vec<_>>()
        .join("\n");
    server_error(None, &message)
}

/// A field of TSV as the bytes it stands for.
fn unescape(field: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(field.len());
    let mut bytes = field.iter().copied();
    while let Some(byte) = bytes.next() {
        if byte != b'\\' {
            out.push(byte);
            continue;
        }
        match bytes.next() {
            Some(b'b') => out.push(0x08),
            Some(b'f') => out.push(0x0c),
            Some(b'r') => out.push(b'\r'),
            Some(b'n') => out.push(b'\n'),
            Some(b't') => out.push(b'\t'),
            Some(b'0') => out.push(0),
            Some(b'a') => out.push(0x07),
            Some(b'v') => out.push(0x0b),
            Some(other) => out.push(other),
            None => out.push(b'\\'),
        }
    }
    out
}

/// `Nullable(T)` and `LowCardinality(T)`, in either order, as `T`.
fn bare(type_name: &str) -> &str {
    let mut name = type_name.trim();
    loop {
        let inner = ["Nullable(", "LowCardinality("]
            .iter()
            .find_map(|wrapper| name.strip_prefix(wrapper)?.strip_suffix(')'));
        match inner {
            Some(inner) => name = inner.trim(),
            None => return name,
        }
    }
}

/// The type's name up to its parameters: `DateTime64(3, 'UTC')` is
/// `DateTime64`.
fn family(type_name: &str) -> &str {
    let bare = bare(type_name);
    bare.split_once('(').map_or(bare, |(family, _)| family)
}

fn kind(type_name: &str) -> ValueKind {
    match family(type_name) {
        "Int8" | "Int16" | "Int32" | "Int64" | "Int128" | "Int256" | "UInt8" | "UInt16"
        | "UInt32" | "UInt64" | "UInt128" | "UInt256" | "Float32" | "Float64" | "BFloat16"
        | "Decimal" | "Decimal32" | "Decimal64" | "Decimal128" | "Decimal256" => ValueKind::Numeric,
        "Bool" => ValueKind::Bool,
        "Date" | "Date32" | "DateTime" | "DateTime64" | "Time" | "Time64" => ValueKind::Temporal,
        "JSON" | "Object" => ValueKind::Json,
        "String" | "FixedString" | "UUID" | "Enum8" | "Enum16" | "IPv4" | "IPv6" => ValueKind::Text,
        _ => ValueKind::Other,
    }
}

/// One field as a cell. Integers that fit `i64` are `Int`, wider ones and
/// decimals stay the server's text, as on MySQL. A `String` that is not
/// UTF-8 is `Bytes`.
fn value(type_name: &str, field: &[u8]) -> Value {
    if field == b"\\N" {
        return Value::Null;
    }
    let bytes = unescape(field);
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => return Value::Bytes(error.into_bytes().into()),
    };
    let parsed = match family(type_name) {
        "Int8" | "Int16" | "Int32" | "Int64" | "Int128" | "Int256" | "UInt8" | "UInt16"
        | "UInt32" | "UInt64" | "UInt128" | "UInt256" => text.parse().ok().map(Value::Int),
        "Float32" | "Float64" | "BFloat16" => match text.as_str() {
            "nan" | "-nan" => Some(Value::Float(f64::NAN)),
            "inf" | "+inf" => Some(Value::Float(f64::INFINITY)),
            "-inf" => Some(Value::Float(f64::NEG_INFINITY)),
            other => other.parse().ok().map(Value::Float),
        },
        "Bool" => match text.as_str() {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            _ => None,
        },
        _ => None,
    };
    parsed.unwrap_or_else(|| Value::Text(text.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(body: &str, wanted: u64, tag: Option<&str>) -> Result<(Vec<ColumnMeta>, Rows)> {
        read_tsv(body.as_bytes(), wanted, tag)
    }

    #[test]
    fn names_and_types_become_columns_and_rows_values() {
        let body = "a\tb\tc\td\nUInt64\tNullable(String)\tDateTime64(3, \\'UTC\\')\tArray(UInt8)\n\
                    1\t\\N\t2026-01-02 03:04:05.123\t[1,2]\n";
        let (columns, rows) = read(body, 10, None).unwrap();
        let names: Vec<_> = columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c", "d"]);
        // A type name comes escaped as a field does.
        assert_eq!(columns[2].type_name, "DateTime64(3, 'UTC')");
        let kinds: Vec<_> = columns.iter().map(|c| c.kind).collect();
        assert_eq!(
            kinds,
            [
                ValueKind::Numeric,
                ValueKind::Text,
                ValueKind::Temporal,
                ValueKind::Other
            ]
        );
        assert_eq!(
            rows,
            [vec![
                Value::Int(1),
                Value::Null,
                Value::Text("2026-01-02 03:04:05.123".into()),
                Value::Text("[1,2]".into()),
            ]]
        );
    }

    #[test]
    fn reading_stops_at_the_rows_wanted() {
        let body = "n\nUInt8\n1\n2\n3\n4\n";
        let (_, rows) = read(body, 2, None).unwrap();
        assert_eq!(rows, [vec![Value::Int(1)], vec![Value::Int(2)]]);
    }

    #[test]
    fn escapes_are_undone_and_a_string_that_is_not_utf8_is_bytes() {
        assert_eq!(
            value("String", b"x\\ty\\\\z\\nw"),
            Value::Text("x\ty\\z\nw".into())
        );
        // A non-null `\N` comes escaped, so only the bare one is NULL.
        assert_eq!(value("String", b"\\\\N"), Value::Text("\\N".into()));
        assert_eq!(
            value("String", b"\xff\\0"),
            Value::Bytes(vec![0xff, 0].into())
        );
    }

    #[test]
    fn numbers_floats_and_booleans_are_typed() {
        assert_eq!(value("Int64", b"-42"), Value::Int(-42));
        assert_eq!(
            value("LowCardinality(Nullable(UInt32))", b"7"),
            Value::Int(7)
        );
        assert_eq!(value("UInt64", b"5"), Value::Int(5));
        assert_eq!(
            value("UInt64", b"18446744073709551615"),
            Value::Text("18446744073709551615".into())
        );
        assert_eq!(value("Decimal(10, 2)", b"1.50"), Value::Text("1.50".into()));
        assert!(matches!(value("Float64", b"nan"), Value::Float(f) if f.is_nan()));
        assert_eq!(value("Float32", b"-inf"), Value::Float(f64::NEG_INFINITY));
        assert_eq!(value("Float64", b"99.5"), Value::Float(99.5));
        assert_eq!(value("Bool", b"true"), Value::Bool(true));
    }

    #[test]
    fn an_error_after_the_rows_began_is_the_servers_error() {
        // 25.8: the message, then the connection closes.
        let old = "n\nUInt64\n1\n2\n__exception__\r\nCode: 395. DB::Exception: boom. \
                   (FUNCTION_THROW_IF_VALUE_IS_NON_ZERO)\n";
        // 26.10: a tag around it, and the message's length.
        let tagged = "n\nUInt64\n1\n2\n\r\n__exception__\r\ntjtg\r\nCode: 395. DB::Exception: \
                      boom. (FUNCTION_THROW_IF_VALUE_IS_NON_ZERO)\n78 tjtg\r\n__exception__\r\n";
        for (body, tag) in [(old, None), (tagged, Some("tjtg"))] {
            let error = read(body, 10, tag).unwrap_err();
            let Error::Query { code, message, .. } = error else {
                panic!("{error:?}");
            };
            assert_eq!(code.as_deref(), Some("395"));
            assert_eq!(
                message,
                "Code: 395. DB::Exception: boom. (FUNCTION_THROW_IF_VALUE_IS_NON_ZERO)"
            );
        }
    }

    #[test]
    fn a_body_that_ends_mid_row_is_a_lost_connection() {
        let error = read("n\nUInt8\n1\n2", 10, None).unwrap_err();
        assert!(error.is_connection_lost(), "{error:?}");
    }

    #[test]
    fn known_codes_become_their_errors() {
        assert_eq!(
            server_error(Some("394"), "Code: 394. Query was cancelled."),
            Error::Cancelled
        );
        assert_eq!(
            server_error(None, "Code: 159. Timeout exceeded"),
            Error::Timeout
        );
        assert!(matches!(
            server_error(Some("516"), "Code: 516. bad password"),
            Error::Auth(_)
        ));
        let Error::Query { code, .. } = server_error(Some("47"), "Code: 47. Unknown") else {
            panic!();
        };
        assert_eq!(code.as_deref(), Some("47"));
    }
}
