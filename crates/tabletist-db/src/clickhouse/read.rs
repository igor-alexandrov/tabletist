//! Reading what ClickHouse answers: `TabSeparatedWithNamesAndTypes` rows,
//! the cells they hold, and the errors it writes, before the rows or among
//! them.

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt};

use crate::{ColumnMeta, Error, Result, Value, ValueKind};

pub(super) type Rows = Vec<Vec<Value>>;

/// The error ClickHouse wrote, with its numeric code where it gave one
/// (in a header before the rows, in the message after).
pub(super) fn server_error(code: Option<&str>, message: &str) -> Error {
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
///
/// An error after the rows wanted is not read. The server sends a block's
/// rows only once the whole block is computed, so such an error is of
/// rows past the page, which the next page meets, as a `LIMIT` on the
/// other engines never computes them.
pub(super) async fn read_tsv(
    mut body: impl AsyncBufRead + Unpin,
    wanted: u64,
    tag: Option<&str>,
) -> Result<(Vec<ColumnMeta>, Rows)> {
    let mut line = Vec::new();
    let names = header(&mut body, &mut line, tag).await?;
    let types = header(&mut body, &mut line, tag).await?;
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
        match next(&mut body, &mut line).await? {
            Line::Row => {
                // Counted first: a zip would drop the fields of a long row.
                if fields(&line).count() != columns.len() {
                    return Err(Error::query("ClickHouse sent a row of another width"));
                }
                rows.push(
                    fields(&line)
                        .zip(&columns)
                        .map(|(field, column)| value(&column.type_name, field))
                        .collect(),
                );
            }
            Line::Exception(rest) => return Err(exception(&rest, tag)),
            Line::End => break,
        }
    }
    Ok((columns, rows))
}

/// The next line into `line`, without its `\n`.
async fn next(body: &mut (impl AsyncBufRead + Unpin), line: &mut Vec<u8>) -> Result<Line> {
    line.clear();
    match body.read_until(b'\n', line).await {
        Ok(0) => Ok(Line::End),
        Ok(_) if line.ends_with(b"\r\n") => {
            let mut rest = std::mem::take(line);
            // The old servers close the connection after the message.
            let _ = body.read_to_end(&mut rest).await;
            Ok(Line::Exception(rest))
        }
        Ok(_) if line.last() == Some(&b'\n') => {
            line.pop();
            Ok(Line::Row)
        }
        Ok(_) => Err(Error::ConnectionLost("the server stopped mid-row".into())),
        Err(error) => Err(Error::ConnectionLost(error.to_string())),
    }
}

/// The names or the types: a line of fields, unescaped.
async fn header(
    body: &mut (impl AsyncBufRead + Unpin),
    line: &mut Vec<u8>,
    tag: Option<&str>,
) -> Result<Vec<String>> {
    match next(body, line).await? {
        Line::Row => Ok(fields(line)
            .map(|field| String::from_utf8_lossy(&unescape(field)).into_owned())
            .collect()),
        Line::Exception(rest) => Err(exception(&rest, tag)),
        Line::End => Err(Error::query("ClickHouse sent no column names")),
    }
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

    async fn read(body: &str, wanted: u64, tag: Option<&str>) -> Result<(Vec<ColumnMeta>, Rows)> {
        read_tsv(body.as_bytes(), wanted, tag).await
    }

    #[tokio::test]
    async fn names_and_types_become_columns_and_rows_values() {
        let body = "a\tb\tc\td\nUInt64\tNullable(String)\tDateTime64(3, \\'UTC\\')\tArray(UInt8)\n\
                    1\t\\N\t2026-01-02 03:04:05.123\t[1,2]\n";
        let (columns, rows) = read(body, 10, None).await.unwrap();
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

    #[tokio::test]
    async fn reading_stops_at_the_rows_wanted() {
        let body = "n\nUInt8\n1\n2\n3\n4\n";
        let (_, rows) = read(body, 2, None).await.unwrap();
        assert_eq!(rows, [vec![Value::Int(1)], vec![Value::Int(2)]]);
    }

    #[tokio::test]
    async fn a_row_of_another_width_is_an_error() {
        for body in ["a\tb\nUInt8\tUInt8\n1\n", "a\tb\nUInt8\tUInt8\n1\t2\t3\n"] {
            let error = read(body, 10, None).await.unwrap_err();
            assert_eq!(
                error,
                Error::query("ClickHouse sent a row of another width"),
                "{body:?}"
            );
        }
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

    #[tokio::test]
    async fn an_error_after_the_rows_began_is_the_servers_error() {
        // 25.8: the message, then the connection closes.
        let old = "n\nUInt64\n1\n2\n__exception__\r\nCode: 395. DB::Exception: boom. \
                   (FUNCTION_THROW_IF_VALUE_IS_NON_ZERO)\n";
        // 26.10: a tag around it, and the message's length.
        let tagged = "n\nUInt64\n1\n2\n\r\n__exception__\r\ntjtg\r\nCode: 395. DB::Exception: \
                      boom. (FUNCTION_THROW_IF_VALUE_IS_NON_ZERO)\n78 tjtg\r\n__exception__\r\n";
        for (body, tag) in [(old, None), (tagged, Some("tjtg"))] {
            let error = read(body, 10, tag).await.unwrap_err();
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

    #[tokio::test]
    async fn a_body_that_ends_mid_row_is_a_lost_connection() {
        let error = read("n\nUInt8\n1\n2", 10, None).await.unwrap_err();
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
