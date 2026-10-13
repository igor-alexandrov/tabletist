//! What a coding agent may send through Tabletist's MCP server, decided on
//! the statement's text alone, before anything reaches a database.
//!
//! [`Refusal`] is why a statement was not run: a code an agent may match
//! on, and a sentence it can act on.

use std::fmt;

use crate::Dialect;

/// The longest statement that is read at all, in bytes.
pub const MAX_SQL_BYTES: usize = 100_000;

/// The most parameters one statement may be sent with.
pub const MAX_PARAMS: usize = 100;

/// A value an agent sends for a placeholder of its statement.
#[derive(Clone, PartialEq)]
pub enum Param {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

/// Without the value: what an agent sends is not logged.
impl fmt::Debug for Param {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Null => "Null",
            Self::Bool(_) => "Bool(..)",
            Self::Int(_) => "Int(..)",
            Self::Float(_) => "Float(..)",
            Self::Text(_) => "Text(..)",
        })
    }
}

/// Why a statement an agent sent was not run. Nothing reached the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    TooLarge(Large),
    /// More than one statement, or none.
    TooManyStatements,
    /// Text the checker could not read. It is refused, not guessed at.
    NotParsed,
    /// Something that writes, or takes locks, where only a read may run.
    NotReadOnly(Writes),
    /// A statement, a clause or an expression agents do not get: what it
    /// is, for the sentence.
    NotAllowed(String),
    /// A function with an effect outside its result, by its name as
    /// written.
    FunctionNotAllowed(String),
    InvalidParams(ParamFault),
}

/// What was too large.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Large {
    Statement,
    Params,
}

/// What in a statement is not a read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Writes {
    Insert,
    Update,
    Delete,
    Merge,
    /// `SELECT … INTO`, which makes a table, a file or a variable.
    Into,
    /// `FOR UPDATE`, `FOR SHARE` and the like.
    Lock,
}

/// What is wrong with a statement's parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamFault {
    /// The statement has `expected` placeholders and `sent` values came.
    Count { expected: usize, sent: usize },
    /// A placeholder that is not the dialect's own (`$1` on PostgreSQL,
    /// `?` on MySQL and SQLite), as written.
    Placeholder { written: String, dialect: Dialect },
    /// The parameter at this 1-based position holds a NUL character.
    Nul(usize),
    /// The parameter at this 1-based position is not a finite number.
    NotFinite(usize),
}

impl Refusal {
    /// The refusal's code, which an agent may match on. It never changes;
    /// the sentence may be reworded.
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooLarge(_) => "too_large",
            Self::TooManyStatements => "too_many_statements",
            Self::NotParsed => "not_parsed",
            Self::NotReadOnly(_) => "not_read_only",
            Self::NotAllowed(_) => "not_allowed",
            Self::FunctionNotAllowed(_) => "function_not_allowed",
            Self::InvalidParams(_) => "invalid_params",
        }
    }
}

/// The sentence an agent is given.
impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge(Large::Statement) => write!(
                f,
                "The statement is larger than {} kB.",
                MAX_SQL_BYTES / 1_000
            ),
            Self::TooLarge(Large::Params) => {
                write!(f, "A statement takes at most {MAX_PARAMS} parameters.")
            }
            Self::TooManyStatements => f.write_str("Send one statement per call."),
            Self::NotParsed => {
                f.write_str("Tabletist could not read this statement, so it was not run.")
            }
            Self::NotReadOnly(writes) => {
                let what = match writes {
                    Writes::Insert => "an INSERT",
                    Writes::Update => "an UPDATE",
                    Writes::Delete => "a DELETE",
                    Writes::Merge => "a MERGE",
                    Writes::Into => {
                        return f.write_str(
                            "run_query only runs reads, and SELECT … INTO writes what it selects.",
                        );
                    }
                    Writes::Lock => {
                        return f.write_str(
                            "run_query only runs reads, and a locking clause such as FOR UPDATE \
                             is not one.",
                        );
                    }
                };
                write!(
                    f,
                    "run_query only runs reads, and this is {what}. Use propose_write to change \
                     data."
                )
            }
            Self::NotAllowed(what) | Self::FunctionNotAllowed(what) => {
                write!(f, "{what} is not available to agents.")
            }
            Self::InvalidParams(ParamFault::Count { expected, sent }) => {
                let (has, were) = (parameters(*expected), was(*sent));
                write!(f, "The statement has {has} and {were} sent.")
            }
            Self::InvalidParams(ParamFault::Placeholder { written, dialect }) => {
                let own = match dialect {
                    Dialect::Postgres => "$1, $2 and so on",
                    Dialect::MySql | Dialect::Sqlite => "?",
                };
                write!(
                    f,
                    "{written} is not a parameter Tabletist can fill. Write {own}."
                )
            }
            Self::InvalidParams(ParamFault::Nul(position)) => write!(
                f,
                "Parameter {position} holds a NUL character, which a database does not take."
            ),
            Self::InvalidParams(ParamFault::NotFinite(position)) => {
                write!(f, "Parameter {position} is not a finite number.")
            }
        }
    }
}

/// "1 parameter", "2 parameters".
fn parameters(count: usize) -> String {
    match count {
        1 => "1 parameter".to_owned(),
        _ => format!("{count} parameters"),
    }
}

/// "1 was", "2 were".
fn was(count: usize) -> String {
    match count {
        1 => "1 was".to_owned(),
        _ => format!("{count} were"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_refusal_has_its_code() {
        let count = ParamFault::Count {
            expected: 2,
            sent: 1,
        };
        for (refusal, code) in [
            (Refusal::TooLarge(Large::Statement), "too_large"),
            (Refusal::TooLarge(Large::Params), "too_large"),
            (Refusal::TooManyStatements, "too_many_statements"),
            (Refusal::NotParsed, "not_parsed"),
            (Refusal::NotReadOnly(Writes::Delete), "not_read_only"),
            (Refusal::NotReadOnly(Writes::Lock), "not_read_only"),
            (Refusal::NotAllowed("COPY".into()), "not_allowed"),
            (
                Refusal::FunctionNotAllowed("pg_sleep".into()),
                "function_not_allowed",
            ),
            (Refusal::InvalidParams(count), "invalid_params"),
        ] {
            assert_eq!(refusal.code(), code, "{refusal:?}");
        }
    }

    #[test]
    fn a_refusal_reads_as_a_sentence_for_the_agent() {
        let params = |fault| Refusal::InvalidParams(fault);
        let placeholder = |written: &str, dialect| ParamFault::Placeholder {
            written: written.into(),
            dialect,
        };
        for (refusal, sentence) in [
            (
                Refusal::TooLarge(Large::Statement),
                "The statement is larger than 100 kB.",
            ),
            (
                Refusal::TooLarge(Large::Params),
                "A statement takes at most 100 parameters.",
            ),
            (Refusal::TooManyStatements, "Send one statement per call."),
            (
                Refusal::NotParsed,
                "Tabletist could not read this statement, so it was not run.",
            ),
            (
                Refusal::NotReadOnly(Writes::Insert),
                "run_query only runs reads, and this is an INSERT. Use propose_write to change \
                 data.",
            ),
            (
                Refusal::NotReadOnly(Writes::Update),
                "run_query only runs reads, and this is an UPDATE. Use propose_write to change \
                 data.",
            ),
            (
                Refusal::NotReadOnly(Writes::Delete),
                "run_query only runs reads, and this is a DELETE. Use propose_write to change \
                 data.",
            ),
            (
                Refusal::NotReadOnly(Writes::Merge),
                "run_query only runs reads, and this is a MERGE. Use propose_write to change \
                 data.",
            ),
            (
                Refusal::NotReadOnly(Writes::Into),
                "run_query only runs reads, and SELECT … INTO writes what it selects.",
            ),
            (
                Refusal::NotReadOnly(Writes::Lock),
                "run_query only runs reads, and a locking clause such as FOR UPDATE is not one.",
            ),
            (
                Refusal::NotAllowed("CREATE TABLE".into()),
                "CREATE TABLE is not available to agents.",
            ),
            (
                Refusal::FunctionNotAllowed("pg_sleep".into()),
                "pg_sleep is not available to agents.",
            ),
            (
                params(ParamFault::Count {
                    expected: 2,
                    sent: 1,
                }),
                "The statement has 2 parameters and 1 was sent.",
            ),
            (
                params(ParamFault::Count {
                    expected: 1,
                    sent: 0,
                }),
                "The statement has 1 parameter and 0 were sent.",
            ),
            (
                params(placeholder(":id", Dialect::Postgres)),
                ":id is not a parameter Tabletist can fill. Write $1, $2 and so on.",
            ),
            (
                params(placeholder("?1", Dialect::Sqlite)),
                "?1 is not a parameter Tabletist can fill. Write ?.",
            ),
            (
                params(placeholder(":id", Dialect::MySql)),
                ":id is not a parameter Tabletist can fill. Write ?.",
            ),
            (
                params(ParamFault::Nul(2)),
                "Parameter 2 holds a NUL character, which a database does not take.",
            ),
            (
                params(ParamFault::NotFinite(1)),
                "Parameter 1 is not a finite number.",
            ),
        ] {
            assert_eq!(refusal.to_string(), sentence, "{refusal:?}");
        }
    }

    #[test]
    fn a_parameter_prints_without_its_value() {
        let printed = format!(
            "{:?}",
            [
                Param::Null,
                Param::Bool(true),
                Param::Int(9100000000000000004),
                Param::Float(19.5),
                Param::Text("ada@example.com".into()),
            ]
        );
        assert_eq!(printed, "[Null, Bool(..), Int(..), Float(..), Text(..)]");
    }
}
