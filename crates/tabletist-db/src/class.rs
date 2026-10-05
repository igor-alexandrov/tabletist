//! What a column takes, read from its type's name: which values a save
//! may send it, and in what form.

use crate::{Dialect, ValueKind};

/// A column's class. A type the app does not know is `Other`: the database
/// alone says what it takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnClass {
    /// Whole numbers from `min` to `max`.
    Integer {
        min: i128,
        max: i128,
    },
    /// Exact numbers, with the digits and the scale the type states. A
    /// number is rounded to `scale` places after the point, and to tens or
    /// hundreds by a scale below zero, which PostgreSQL alone takes; what
    /// is left is under ten to the power of `precision - scale`.
    Decimal {
        precision: Option<u32>,
        scale: Option<i32>,
    },
    Float,
    Boolean,
    Json,
    /// Text, at most `max_chars` characters when the type says so.
    Text {
        max_chars: Option<u32>,
    },
    /// Bytes. The app does not send these as text: a database that takes
    /// text for them stores the characters' codes, not the value meant.
    Binary,
    Other,
}

/// The class of a column whose type the catalog names `type_name`
/// (`ColumnInfo::type_name`).
pub fn column_class(dialect: Dialect, type_name: &str) -> ColumnClass {
    let name = type_name.trim().to_ascii_lowercase();
    match dialect {
        Dialect::Sqlite => sqlite(&name),
        Dialect::Postgres => postgres(&name),
        Dialect::MySql => mysql(&name),
    }
}

/// The numbers in a type's parentheses: `numeric(14,2)` gives `[14, 2]`,
/// and PostgreSQL's `numeric(5,-2)` gives `[5, -2]`. None of them when one
/// is not a whole number: the others would mislead alone.
fn arguments(name: &str) -> Vec<i32> {
    let Some((_, rest)) = name.split_once('(') else {
        return Vec::new();
    };
    rest.split(')')
        .next()
        .unwrap_or_default()
        .split(',')
        .map(|argument| argument.trim().parse().ok())
        .collect::<Option<_>>()
        .unwrap_or_default()
}

/// The length in a type's parentheses: `varchar(255)` gives 255.
fn length(name: &str) -> Option<u32> {
    let first = arguments(name).first().copied()?;
    u32::try_from(first).ok()
}

/// The type's name without its parentheses and what follows them.
fn base(name: &str) -> &str {
    name.split('(').next().unwrap_or_default().trim()
}

fn signed(bytes: u32) -> ColumnClass {
    let max = (1_i128 << (bytes * 8 - 1)) - 1;
    ColumnClass::Integer { min: -max - 1, max }
}

fn unsigned(bytes: u32) -> ColumnClass {
    ColumnClass::Integer {
        min: 0,
        max: (1_i128 << (bytes * 8)) - 1,
    }
}

/// The most digits PostgreSQL takes for a numeric, and the furthest scale
/// either side of zero. MySQL's own limits are well inside them.
const MAX_DIGITS: i32 = 1000;

fn decimal(name: &str) -> ColumnClass {
    let unstated = ColumnClass::Decimal {
        precision: None,
        scale: None,
    };
    let arguments = arguments(name);
    let Some(&digits) = arguments.first() else {
        return unstated;
    };
    // Digits without a scale: none after the point.
    let scale = arguments.get(1).copied().unwrap_or(0);
    // Past the limits it is no name a database gives, and nothing to hold
    // a value to. Asked as a range: the least `i32` has no opposite to
    // compare, and a name is whatever a server sends.
    let scales = -MAX_DIGITS..=MAX_DIGITS;
    match u32::try_from(digits) {
        Ok(precision) if (1..=MAX_DIGITS).contains(&digits) && scales.contains(&scale) => {
            ColumnClass::Decimal {
                precision: Some(precision),
                scale: Some(scale),
            }
        }
        _ => unstated,
    }
}

fn postgres(name: &str) -> ColumnClass {
    // An array of anything is the database's to judge.
    if name.ends_with("[]") {
        return ColumnClass::Other;
    }
    match base(name) {
        "smallint" => signed(2),
        "integer" => signed(4),
        "bigint" => signed(8),
        "numeric" => decimal(name),
        "real" | "double precision" => ColumnClass::Float,
        "boolean" => ColumnClass::Boolean,
        "json" | "jsonb" => ColumnClass::Json,
        "text" | "bpchar" => ColumnClass::Text { max_chars: None },
        "character varying" | "character" => ColumnClass::Text {
            max_chars: length(name),
        },
        "bytea" => ColumnClass::Binary,
        _ => ColumnClass::Other,
    }
}

fn mysql(name: &str) -> ColumnClass {
    // A boolean is a tinyint of display width one.
    if name == "tinyint(1)" {
        return ColumnClass::Boolean;
    }
    // The first word: `unsigned` and `zerofill` come after it.
    let word = base(name).split(' ').next().unwrap_or_default();
    let bytes = match word {
        "tinyint" => Some(1),
        "smallint" => Some(2),
        "mediumint" => Some(3),
        "int" => Some(4),
        "bigint" => Some(8),
        _ => None,
    };
    if let Some(bytes) = bytes {
        return if name.contains("unsigned") {
            unsigned(bytes)
        } else {
            signed(bytes)
        };
    }
    match word {
        "decimal" => match decimal(name) {
            // No scale below zero in MySQL (the scale is 0 to 30): such a
            // name is none of its own, and states nothing to check by.
            ColumnClass::Decimal {
                scale: Some(scale), ..
            } if scale < 0 => ColumnClass::Decimal {
                precision: None,
                scale: None,
            },
            class => class,
        },
        "float" | "double" => ColumnClass::Float,
        "json" => ColumnClass::Json,
        "varchar" | "char" => ColumnClass::Text {
            max_chars: length(name),
        },
        "tinytext" | "text" | "mediumtext" | "longtext" => ColumnClass::Text { max_chars: None },
        // `bit` too: given the text `1`, MySQL stores the character's code.
        "bit" | "binary" | "varbinary" | "tinyblob" | "blob" | "mediumblob" | "longblob" => {
            ColumnClass::Binary
        }
        // A spatial value arrives as its bytes, and text is not one.
        "geometry" | "point" | "linestring" | "polygon" | "multipoint" | "multilinestring"
        | "multipolygon" | "geometrycollection" | "geomcollection" => ColumnClass::Binary,
        _ => ColumnClass::Other,
    }
}

/// By the affinity SQLite gives the declared type, with JSON, boolean and
/// date and time names told apart first, as `ValueKind` tells them. Whole
/// numbers alone only for the name of an integer type.
fn sqlite(name: &str) -> ColumnClass {
    match ValueKind::from_sqlite_decl(name) {
        ValueKind::Json => ColumnClass::Json,
        ValueKind::Bool => ColumnClass::Boolean,
        ValueKind::Text => ColumnClass::Text { max_chars: None },
        ValueKind::Numeric if names_sqlite_integers(name) => signed(8),
        // Any name holding `int` has integer affinity, which keeps a number
        // that is not whole as it is: `FLOATING POINT` takes 1.5, and is
        // not a float either, since it keeps a whole number exact.
        ValueKind::Numeric
            if !name.contains("int")
                && ["real", "floa", "doub"]
                    .iter()
                    .any(|word| name.contains(word)) =>
        {
            ColumnClass::Float
        }
        ValueKind::Numeric => ColumnClass::Decimal {
            precision: None,
            scale: None,
        },
        ValueKind::Binary => ColumnClass::Binary,
        ValueKind::Temporal | ValueKind::Other => ColumnClass::Other,
    }
}

/// Whether a SQLite declared type is the name of an integer type, not just
/// a name with `int` in it.
fn names_sqlite_integers(name: &str) -> bool {
    let mut words = base(name).split_whitespace();
    match words.next() {
        Some(
            "int" | "integer" | "tinyint" | "smallint" | "mediumint" | "bigint" | "int2" | "int8",
        ) => true,
        Some("unsigned") => words.eq(["big", "int"]),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ColumnClass::{Binary, Boolean, Decimal, Float, Integer, Json, Other, Text};

    fn int(min: i128, max: i128) -> ColumnClass {
        Integer { min, max }
    }

    #[test]
    fn postgres_types_have_classes() {
        for (name, class) in [
            ("smallint", int(-32_768, 32_767)),
            ("integer", int(i128::from(i32::MIN), i128::from(i32::MAX))),
            ("bigint", int(i128::from(i64::MIN), i128::from(i64::MAX))),
            (
                "numeric(14,2)",
                Decimal {
                    precision: Some(14),
                    scale: Some(2),
                },
            ),
            (
                "numeric(5)",
                Decimal {
                    precision: Some(5),
                    scale: Some(0),
                },
            ),
            (
                "numeric",
                Decimal {
                    precision: None,
                    scale: None,
                },
            ),
            // A scale below zero rounds to tens or hundreds, and leaves
            // more whole digits than the precision: 1234500.
            (
                "numeric(5,-2)",
                Decimal {
                    precision: Some(5),
                    scale: Some(-2),
                },
            ),
            (
                "numeric(5, -2)",
                Decimal {
                    precision: Some(5),
                    scale: Some(-2),
                },
            ),
            // Past what PostgreSQL takes for either: no name it gives.
            (
                "numeric(5,-1001)",
                Decimal {
                    precision: None,
                    scale: None,
                },
            ),
            (
                "numeric(0,2)",
                Decimal {
                    precision: None,
                    scale: None,
                },
            ),
            (
                "numeric(-5,2)",
                Decimal {
                    precision: None,
                    scale: None,
                },
            ),
            ("real", Float),
            ("double precision", Float),
            ("boolean", Boolean),
            ("json", Json),
            ("jsonb", Json),
            ("text", Text { max_chars: None }),
            ("character varying", Text { max_chars: None }),
            (
                "character varying(200)",
                Text {
                    max_chars: Some(200),
                },
            ),
            ("character(5)", Text { max_chars: Some(5) }),
            // What a column declared `bpchar` prints as: text of any length.
            ("bpchar", Text { max_chars: None }),
            // One byte, not one character.
            ("\"char\"", Other),
            // More places after the point than digits: 0.00123.
            (
                "numeric(3,5)",
                Decimal {
                    precision: Some(3),
                    scale: Some(5),
                },
            ),
            ("bytea", Binary),
            ("timestamp with time zone", Other),
            ("text[]", Other),
            ("integer[]", Other),
            ("character varying(20)[]", Other),
            ("mood", Other),
            ("uuid", Other),
        ] {
            assert_eq!(column_class(Dialect::Postgres, name), class, "{name}");
        }
    }

    #[test]
    fn mysql_types_have_classes() {
        for (name, class) in [
            ("tinyint(1)", Boolean),
            ("tinyint", int(-128, 127)),
            ("tinyint(4)", int(-128, 127)),
            ("tinyint unsigned", int(0, 255)),
            ("smallint", int(-32_768, 32_767)),
            ("mediumint", int(-8_388_608, 8_388_607)),
            ("mediumint unsigned", int(0, 16_777_215)),
            // Not the boolean: `BOOLEAN` is the signed `tinyint(1)` alone.
            ("tinyint(1) unsigned zerofill", int(0, 255)),
            ("int", int(i128::from(i32::MIN), i128::from(i32::MAX))),
            ("int(11)", int(i128::from(i32::MIN), i128::from(i32::MAX))),
            ("int unsigned", int(0, i128::from(u32::MAX))),
            ("bigint", int(i128::from(i64::MIN), i128::from(i64::MAX))),
            ("bigint unsigned", int(0, i128::from(u64::MAX))),
            ("BIGINT UNSIGNED", int(0, i128::from(u64::MAX))),
            (
                "decimal(14,2)",
                Decimal {
                    precision: Some(14),
                    scale: Some(2),
                },
            ),
            (
                "decimal(10,2) unsigned",
                Decimal {
                    precision: Some(10),
                    scale: Some(2),
                },
            ),
            // MySQL takes no scale below zero: a name with one is none of
            // its own, and states nothing the app can hold a value to.
            (
                "decimal(5,-2)",
                Decimal {
                    precision: None,
                    scale: None,
                },
            ),
            ("int(10) unsigned zerofill", int(0, i128::from(u32::MAX))),
            ("float", Float),
            ("float(7,3)", Float),
            ("float unsigned", Float),
            ("double", Float),
            ("double unsigned", Float),
            ("json", Json),
            (
                "varchar(255)",
                Text {
                    max_chars: Some(255),
                },
            ),
            ("char(3)", Text { max_chars: Some(3) }),
            ("text", Text { max_chars: None }),
            ("longtext", Text { max_chars: None }),
            ("datetime(6)", Other),
            ("year", Other),
            ("enum('happy','sad')", Other),
            ("binary(16)", Binary),
            ("varbinary(16)", Binary),
            ("tinyblob", Binary),
            ("blob", Binary),
            ("mediumblob", Binary),
            ("longblob", Binary),
            ("bit(1)", Binary),
            ("bit(8)", Binary),
            // Spatial values come as bytes.
            ("geometry", Binary),
            ("point", Binary),
            ("linestring", Binary),
            ("polygon", Binary),
            ("multipoint", Binary),
            ("multilinestring", Binary),
            ("multipolygon", Binary),
            ("geometrycollection", Binary),
            ("geomcollection", Binary),
        ] {
            assert_eq!(column_class(Dialect::MySql, name), class, "{name}");
        }
    }

    /// A type's name comes from a server, and from whatever stands in for
    /// one: a scale that has no opposite must not take the app down.
    #[test]
    fn a_scale_no_number_negates_states_nothing() {
        let unstated = Decimal {
            precision: None,
            scale: None,
        };
        for scale in [i32::MIN, i32::MIN + 1, -1001, 1001, i32::MAX] {
            for (dialect, name) in [
                (Dialect::Postgres, format!("numeric(5,{scale})")),
                (Dialect::MySql, format!("decimal(5,{scale})")),
                (Dialect::Sqlite, format!("NUMERIC(5,{scale})")),
            ] {
                assert_eq!(column_class(dialect, &name), unstated, "{name}");
            }
        }
        // The furthest scale either side that is a name still.
        for scale in [-1000, 1000] {
            assert_eq!(
                column_class(Dialect::Postgres, &format!("numeric(5,{scale})")),
                Decimal {
                    precision: Some(5),
                    scale: Some(scale),
                }
            );
        }
    }

    #[test]
    fn sqlite_types_have_the_class_of_their_affinity() {
        let whole = int(i128::from(i64::MIN), i128::from(i64::MAX));
        for (name, class) in [
            ("INTEGER", whole),
            ("bigint", whole),
            ("INT UNSIGNED", whole),
            ("INT8", whole),
            ("UNSIGNED BIG INT", whole),
            ("BOOLEAN", Boolean),
            ("JSON", Json),
            ("REAL", Float),
            ("FLOAT", Float),
            ("double precision", Float),
            // Integer affinity, by the `INT` in the name, and it keeps 1.5
            // as it is: a number, whole or not.
            (
                "FLOATING POINT",
                Decimal {
                    precision: None,
                    scale: None,
                },
            ),
            (
                "POINT",
                Decimal {
                    precision: None,
                    scale: None,
                },
            ),
            // SQLite enforces neither digits nor a length.
            (
                "NUMERIC(10,2)",
                Decimal {
                    precision: None,
                    scale: None,
                },
            ),
            (
                "DECIMAL",
                Decimal {
                    precision: None,
                    scale: None,
                },
            ),
            (
                "NUMERIC(5,-2)",
                Decimal {
                    precision: None,
                    scale: None,
                },
            ),
            ("TEXT", Text { max_chars: None }),
            ("VARCHAR(255)", Text { max_chars: None }),
            ("DATETIME", Other),
            ("BLOB", Binary),
            ("blob", Binary),
            ("", Other),
        ] {
            assert_eq!(column_class(Dialect::Sqlite, name), class, "{name}");
        }
    }
}
