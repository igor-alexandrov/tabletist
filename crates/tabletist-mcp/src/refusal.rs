//! Why a call was not answered: a code an agent may match on, and a
//! sentence it can act on. A refusal is a tool's result, not a protocol
//! error, so that the model reads it.

use std::time::Duration;

use tabletist_db::Error;
use tabletist_db::agent::Limits;

use crate::gateway::Failure;
use crate::stop::Why;

/// The codes of this crate's own refusals. The check's are its own
/// ([`tabletist_db::agent::Refusal::code`]). None of them ever changes.
pub mod code {
    pub const UNKNOWN_CONNECTION: &str = "unknown_connection";
    pub const CONNECTION_UNAVAILABLE: &str = "connection_unavailable";
    pub const TOO_LARGE: &str = "too_large";
    pub const INVALID_PARAMS: &str = "invalid_params";
    pub const TIMEOUT: &str = "timeout";
    pub const BUSY: &str = "busy";
    pub const QUERY_FAILED: &str = "query_failed";
    pub const CANCELLED: &str = "cancelled";
    pub const SERVER_STOPPED: &str = "server_stopped";
}

/// A call that was not answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// Stable. See [`code`].
    pub code: &'static str,
    /// For the agent. It may be reworded.
    pub message: String,
    /// What the log keeps in the sentence's place, where the sentence may
    /// hold a row's value.
    logged: Option<String>,
}

/// The most of an agent's own text a sentence repeats.
const MAX_ECHO_CHARS: usize = 80;

/// The most characters of a sentence. A database writes its errors as long
/// as it likes, and with what it likes in them.
pub const MAX_SENTENCE_CHARS: usize = 2_000;

impl Refused {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        let message: String = message.into();
        let message = match message.char_indices().nth(MAX_SENTENCE_CHARS) {
            Some((end, _)) => format!("{}…", &message[..end]),
            None => message,
        };
        Self {
            code,
            message,
            logged: None,
        }
    }

    /// The sentence as the log keeps it.
    pub(crate) fn for_the_log(&self) -> &str {
        self.logged.as_deref().unwrap_or(&self.message)
    }

    /// The name matches nothing an agent may see. A connection that is Off
    /// answers exactly this.
    pub(crate) fn unknown_connection(name: &str) -> Self {
        let name: String = name.chars().take(MAX_ECHO_CHARS).collect();
        Self::new(
            code::UNKNOWN_CONNECTION,
            format!("No connection named {name}. list_connections has the names."),
        )
    }

    /// One sentence whatever went wrong: the reason can name a host, a
    /// user or a file.
    pub(crate) fn unavailable(connection: &str) -> Self {
        Self::new(
            code::CONNECTION_UNAVAILABLE,
            format!("Tabletist could not open {connection}. Open it in Tabletist to see why."),
        )
    }

    pub(crate) fn busy() -> Self {
        Self::new(
            code::BUSY,
            "Another agent's query is running on this connection. Try again.",
        )
    }

    /// A session has as many calls running as it may.
    pub(crate) fn too_many_calls() -> Self {
        Self::new(
            code::BUSY,
            "Too many calls at once. Wait for an answer before the next call.",
        )
    }

    pub(crate) fn timeout(limit: Duration) -> Self {
        Self::new(
            code::TIMEOUT,
            format!(
                "The query was stopped after {}, this connection's limit.",
                seconds(limit)
            ),
        )
    }

    pub(crate) fn cancelled() -> Self {
        Self::new(code::CANCELLED, "The call was cancelled.")
    }

    pub(crate) fn stopped() -> Self {
        Self::new(
            code::SERVER_STOPPED,
            "Tabletist's agent server was stopped.",
        )
    }

    /// What a call answers that comes to a session which has ended: that
    /// the person stopped it, or else that there is nobody to answer.
    pub(crate) fn session_ended(why: Why) -> Self {
        if why == Why::Stopped {
            Self::stopped()
        } else {
            Self::cancelled()
        }
    }

    /// What the gateway's failure is to the agent. `why` is the reason the
    /// call was stopped for, when it was.
    pub(crate) fn failed(
        failure: Failure,
        connection: &str,
        limits: Limits,
        why: Option<Why>,
    ) -> Self {
        let error = match failure {
            Failure::Unavailable => return Self::unavailable(connection),
            Failure::Busy => return Self::busy(),
            Failure::Database(error) => error,
        };
        match error {
            // SQLite read the statement as something the check did not.
            Error::Agent(refusal) => refusal.into(),
            Error::Timeout => Self::timeout(limits.timeout),
            Error::Cancelled => match why {
                Some(Why::Stopped) => Self::stopped(),
                // Or the database ended it, and nobody here asked.
                Some(Why::Client) | None => Self::cancelled(),
            },
            Error::Query {
                code,
                message,
                detail,
                hint,
                ..
            } => {
                let coded = match &code {
                    Some(code) => format!(" (code {code})"),
                    None => String::new(),
                };
                let mut sentence = format!("{message}{coded}");
                for (label, line) in [("Detail", detail), ("Hint", hint)] {
                    if let Some(line) = line {
                        sentence.push_str(&format!("\n{label}: {line}"));
                    }
                }
                // A database quotes what it could not read, and that can
                // be a row's value. The agent may read the row. The log
                // holds no row, so it keeps the error's code alone.
                Self {
                    logged: Some(format!("The database refused the statement{coded}.")),
                    ..Self::new(code::QUERY_FAILED, sentence)
                }
            }
            // Each of these is about the session, and its words may hold
            // what an agent is not told.
            Error::Connect(_)
            | Error::Auth(_)
            | Error::Tls(_)
            | Error::Ssh { .. }
            | Error::ConnectionLost(_)
            | Error::InvalidSpec(_)
            | Error::Unsupported(_)
            | Error::Io(_)
            | Error::Refused { .. }
            | Error::ReadOnly
            | Error::LeftReadOnly
            | Error::LeftTransaction => Self::unavailable(connection),
        }
    }

    /// What the log calls the call's end.
    pub(crate) fn result(&self) -> &'static str {
        match self.code {
            code::TIMEOUT => "timeout",
            code::CANCELLED => "cancelled",
            code::SERVER_STOPPED => "stopped",
            code::QUERY_FAILED | code::CONNECTION_UNAVAILABLE => "error",
            _ => "refused",
        }
    }
}

impl From<tabletist_db::agent::Refusal> for Refused {
    fn from(refusal: tabletist_db::agent::Refusal) -> Self {
        Self::new(refusal.code(), refusal.to_string())
    }
}

/// "10 s", "1.5 s".
fn seconds(time: Duration) -> String {
    let millis = time.as_millis();
    if millis.is_multiple_of(1_000) {
        format!("{} s", millis / 1_000)
    } else {
        format!("{} s", time.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use tabletist_db::Dialect;
    use tabletist_db::agent::check_read;

    use super::*;

    #[test]
    fn the_codes_are_the_ones_agents_were_told() {
        assert_eq!(
            [
                code::UNKNOWN_CONNECTION,
                code::CONNECTION_UNAVAILABLE,
                code::TOO_LARGE,
                code::INVALID_PARAMS,
                code::TIMEOUT,
                code::BUSY,
                code::QUERY_FAILED,
                code::CANCELLED,
                code::SERVER_STOPPED,
            ],
            [
                "unknown_connection",
                "connection_unavailable",
                "too_large",
                "invalid_params",
                "timeout",
                "busy",
                "query_failed",
                "cancelled",
                "server_stopped",
            ]
        );
    }

    #[test]
    fn a_sentence_is_only_so_long() {
        let refused = Refused::new(code::QUERY_FAILED, "é".repeat(600_000));
        assert_eq!(refused.message.chars().count(), MAX_SENTENCE_CHARS + 1);
        assert!(refused.message.ends_with('…'));
        // A sentence is kept in the log as it is said, unless it says
        // otherwise.
        let short = Refused::too_many_calls();
        assert_eq!(short.for_the_log(), short.message);
        assert_eq!(short.code, "busy");
    }

    #[test]
    fn a_call_to_a_session_that_has_ended_says_why() {
        let stopped = Refused::session_ended(Why::Stopped);
        assert_eq!(
            (stopped.code, stopped.result()),
            ("server_stopped", "stopped")
        );
        assert_eq!(stopped.message, "Tabletist's agent server was stopped.");
        let gone = Refused::session_ended(Why::Client);
        assert_eq!((gone.code, gone.result()), ("cancelled", "cancelled"));
    }

    fn limits() -> Limits {
        Limits {
            rows: 500,
            timeout: Duration::from_secs(10),
        }
    }

    fn database(error: Error, why: Option<Why>) -> Refused {
        Refused::failed(Failure::Database(error), "Bookshop · dev", limits(), why)
    }

    #[test]
    fn a_refusal_of_the_check_keeps_its_code_and_its_sentence() {
        let refusal = check_read(Dialect::Sqlite, "DELETE FROM book_covers", &[]).unwrap_err();
        let refused = Refused::from(refusal);
        assert_eq!(refused.code, "not_read_only");
        assert_eq!(
            refused.message,
            "run_query only runs reads, and this is a DELETE."
        );
        assert_eq!(refused.result(), "refused");
    }

    #[test]
    fn a_failure_to_open_says_one_thing_whatever_went_wrong() {
        let said = "Tabletist could not open Bookshop · dev. Open it in Tabletist to see why.";
        for error in [
            Error::Connect("db.internal.example:5432 refused".into()),
            Error::Auth("password authentication failed for user \"shop_admin\"".into()),
            Error::Tls("certificate of db.internal.example".into()),
            Error::ConnectionLost("db.internal.example closed".into()),
            Error::InvalidSpec("choose a SQLite file".into()),
            Error::Io("/home/me/shop.db is not a database".into()),
            Error::LeftReadOnly,
        ] {
            let refused = database(error, None);
            assert_eq!(refused.code, "connection_unavailable");
            assert_eq!(refused.message, said);
            assert_eq!(refused.result(), "error");
        }
        let refused = Refused::failed(Failure::Unavailable, "Bookshop · dev", limits(), None);
        assert_eq!(refused.message, said);
    }

    #[test]
    fn the_databases_own_error_comes_with_its_code_and_its_hint() {
        let error = Error::Query {
            code: Some("42703".into()),
            message: "column \"nmae\" does not exist".into(),
            detail: None,
            hint: Some("Perhaps you meant to reference the column \"authors.name\".".into()),
            named: Box::default(),
        };
        let refused = database(error, None);
        assert_eq!(refused.code, "query_failed");
        assert_eq!(
            refused.message,
            "column \"nmae\" does not exist (code 42703)\n\
             Hint: Perhaps you meant to reference the column \"authors.name\"."
        );
        assert_eq!(refused.result(), "error");
        // Its words are the agent's to read, and not the log's to keep.
        assert_eq!(
            refused.for_the_log(),
            "The database refused the statement (code 42703)."
        );
    }

    #[test]
    fn a_databases_error_without_a_code_is_kept_without_one() {
        let refused = database(Error::query("x".repeat(600_000)), None);
        assert_eq!(refused.message.chars().count(), MAX_SENTENCE_CHARS + 1);
        assert_eq!(refused.for_the_log(), "The database refused the statement.");
    }

    #[test]
    fn a_stopped_call_says_who_stopped_it() {
        let cases = [
            (Some(Why::Client), "cancelled", "cancelled"),
            (Some(Why::Stopped), "server_stopped", "stopped"),
            // The database ended it, and nobody here asked.
            (None, "cancelled", "cancelled"),
        ];
        for (why, code, result) in cases {
            let refused = database(Error::Cancelled, why);
            assert_eq!(refused.code, code);
            assert_eq!(refused.result(), result);
        }
    }

    #[test]
    fn the_databases_own_timeout_names_the_limit() {
        let refused = database(Error::Timeout, None);
        assert_eq!(
            refused.message,
            "The query was stopped after 10 s, this connection's limit."
        );
        assert_eq!(seconds(Duration::from_millis(1_500)), "1.5 s");
    }

    #[test]
    fn a_name_that_matches_nothing_is_repeated_only_so_far() {
        let refused = Refused::unknown_connection(&"x".repeat(5_000));
        assert_eq!(refused.code, "unknown_connection");
        assert!(refused.message.chars().count() < 150);
        assert_eq!(
            Refused::unknown_connection("Bookshop · production").message,
            "No connection named Bookshop · production. list_connections has the names."
        );
    }
}
