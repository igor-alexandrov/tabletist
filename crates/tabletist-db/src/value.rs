//! Cell values and column metadata, the same for every driver.

/// One cell. Integers wider than `i64` and exact decimals arrive as `Text`
/// with a `Numeric` column kind, so nothing is rounded.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(Box<str>),
    Bytes(Box<[u8]>),
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

/// How the interface treats a column: alignment, pretty-printing, hex.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Numeric,
    Text,
    Json,
    Temporal,
    Binary,
    Bool,
    Other,
}

impl ValueKind {
    /// The kind for a SQLite declared column type. Follows SQLite's affinity
    /// rules, with JSON, boolean and date/time names recognised first.
    pub fn from_sqlite_decl(decl: &str) -> Self {
        let decl = decl.to_ascii_uppercase();
        let has = |needle: &str| decl.contains(needle);
        if decl.is_empty() {
            Self::Other
        } else if has("JSON") {
            Self::Json
        } else if has("BOOL") {
            Self::Bool
        } else if has("DATE") || has("TIME") {
            Self::Temporal
        } else if has("INT") {
            Self::Numeric
        } else if has("CHAR") || has("CLOB") || has("TEXT") {
            Self::Text
        } else if has("BLOB") {
            Self::Binary
        } else if has("REAL") || has("FLOA") || has("DOUB") || has("NUMERIC") || has("DEC") {
            Self::Numeric
        } else {
            Self::Other
        }
    }

    /// The kind a value's storage suggests, for columns with no declared type.
    pub fn of_value(value: &Value) -> Self {
        match value {
            Value::Null => Self::Other,
            Value::Bool(_) => Self::Bool,
            Value::Int(_) | Value::Float(_) => Self::Numeric,
            Value::Text(_) => Self::Text,
            Value::Bytes(_) => Self::Binary,
        }
    }

    /// The kind for a PostgreSQL type name (`pg_type.typname`). Arrays
    /// (`_text`), enums, ranges and extension types are `Other`.
    pub fn from_pg_type(name: &str) -> Self {
        match name {
            "bool" => Self::Bool,
            "int2" | "int4" | "int8" | "oid" | "float4" | "float8" | "numeric" | "money" => {
                Self::Numeric
            }
            "json" | "jsonb" => Self::Json,
            "date" | "time" | "timetz" | "timestamp" | "timestamptz" | "interval" => Self::Temporal,
            "bytea" => Self::Binary,
            "text" | "varchar" | "bpchar" | "char" | "name" | "uuid" | "citext" | "xml"
            | "inet" | "cidr" | "macaddr" => Self::Text,
            _ => Self::Other,
        }
    }
}

/// A PostgreSQL value as the simple-query protocol sends it (text), typed by
/// its column's type name. What does not parse stays as the server's text,
/// and exact decimals stay text so nothing is rounded.
pub fn value_from_pg_text(type_name: &str, text: &str) -> Value {
    let as_text = || Value::Text(text.into());
    match type_name {
        "bool" => match text {
            "t" => Value::Bool(true),
            "f" => Value::Bool(false),
            _ => as_text(),
        },
        "int2" | "int4" | "int8" | "oid" => {
            text.parse().map(Value::Int).unwrap_or_else(|_| as_text())
        }
        "float4" | "float8" => text.parse().map(Value::Float).unwrap_or_else(|_| as_text()),
        "bytea" => decode_bytea(text)
            .map(|bytes| Value::Bytes(bytes.into()))
            .unwrap_or_else(as_text),
        _ => as_text(),
    }
}

/// `\x0a0b...` (PostgreSQL's hex bytea output) to bytes.
fn decode_bytea(text: &str) -> Option<Vec<u8>> {
    let hex = text.strip_prefix("\\x")?;
    if hex.len() % 2 != 0 {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(hex.get(index..index + 2)?, 16).ok())
        .collect()
}

/// A result column.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnMeta {
    pub name: String,
    /// The database's own type name (`int4`, `varchar(255)`, `TEXT`), or
    /// empty when the database does not say.
    pub type_name: String,
    pub kind: ValueKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_declared_types_map_to_kinds() {
        let cases = [
            ("INTEGER", ValueKind::Numeric),
            ("bigint", ValueKind::Numeric),
            ("NUMERIC(10,2)", ValueKind::Numeric),
            ("DECIMAL", ValueKind::Numeric),
            ("REAL", ValueKind::Numeric),
            ("double precision", ValueKind::Numeric),
            ("TEXT", ValueKind::Text),
            ("VARCHAR(255)", ValueKind::Text),
            ("CLOB", ValueKind::Text),
            ("JSON", ValueKind::Json),
            ("jsonb", ValueKind::Json),
            ("BLOB", ValueKind::Binary),
            ("BOOLEAN", ValueKind::Bool),
            ("DATETIME", ValueKind::Temporal),
            ("date", ValueKind::Temporal),
            ("TIMESTAMP", ValueKind::Temporal),
            ("", ValueKind::Other),
            ("GEOMETRY", ValueKind::Other),
        ];
        for (decl, kind) in cases {
            assert_eq!(ValueKind::from_sqlite_decl(decl), kind, "{decl}");
        }
    }

    #[test]
    fn values_have_kinds_from_their_storage() {
        assert_eq!(ValueKind::of_value(&Value::Int(1)), ValueKind::Numeric);
        assert_eq!(ValueKind::of_value(&Value::Float(1.5)), ValueKind::Numeric);
        assert_eq!(
            ValueKind::of_value(&Value::Text("a".into())),
            ValueKind::Text
        );
        assert_eq!(
            ValueKind::of_value(&Value::Bytes(vec![1].into())),
            ValueKind::Binary
        );
        assert_eq!(ValueKind::of_value(&Value::Bool(true)), ValueKind::Bool);
        assert_eq!(ValueKind::of_value(&Value::Null), ValueKind::Other);
        assert!(Value::Null.is_null());
        assert!(!Value::Int(0).is_null());
    }

    #[test]
    fn postgres_type_names_map_to_kinds() {
        let cases = [
            ("bool", ValueKind::Bool),
            ("int4", ValueKind::Numeric),
            ("int8", ValueKind::Numeric),
            ("numeric", ValueKind::Numeric),
            ("float8", ValueKind::Numeric),
            ("jsonb", ValueKind::Json),
            ("json", ValueKind::Json),
            ("timestamptz", ValueKind::Temporal),
            ("interval", ValueKind::Temporal),
            ("bytea", ValueKind::Binary),
            ("text", ValueKind::Text),
            ("varchar", ValueKind::Text),
            ("uuid", ValueKind::Text),
            ("_text", ValueKind::Other),
            ("mood", ValueKind::Other),
            ("geometry", ValueKind::Other),
        ];
        for (name, kind) in cases {
            assert_eq!(ValueKind::from_pg_type(name), kind, "{name}");
        }
    }

    #[test]
    fn postgres_text_values_become_typed_values() {
        assert_eq!(value_from_pg_text("bool", "t"), Value::Bool(true));
        assert_eq!(value_from_pg_text("bool", "f"), Value::Bool(false));
        assert_eq!(value_from_pg_text("int8", "-42"), Value::Int(-42));
        assert_eq!(value_from_pg_text("float8", "99.5"), Value::Float(99.5));
        assert!(matches!(value_from_pg_text("float8", "NaN"), Value::Float(f) if f.is_nan()));
        assert_eq!(
            value_from_pg_text("float8", "-Infinity"),
            Value::Float(f64::NEG_INFINITY)
        );
        assert_eq!(
            value_from_pg_text("bytea", "\\x89504e47"),
            Value::Bytes(vec![0x89, 0x50, 0x4e, 0x47].into())
        );
        // Anything that does not parse stays as the server's text.
        assert_eq!(
            value_from_pg_text("bytea", "\\xzz"),
            Value::Text("\\xzz".into())
        );
        assert_eq!(
            value_from_pg_text("numeric", "12345678901234567890.12"),
            Value::Text("12345678901234567890.12".into())
        );
        assert_eq!(
            value_from_pg_text("_text", "{a,b}"),
            Value::Text("{a,b}".into())
        );
    }
}
