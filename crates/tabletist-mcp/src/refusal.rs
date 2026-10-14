//! Why a call was not answered: a code an agent may match on, and a
//! sentence it can act on. A refusal is a tool's result, not a protocol
//! error, so that the model reads it.

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

    /// A session has as many calls running as it may.
    pub(crate) fn too_many_calls() -> Self {
        Self::new(
            code::BUSY,
            "Too many calls at once. Wait for an answer before the next call.",
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

#[cfg(test)]
mod tests {
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
}
