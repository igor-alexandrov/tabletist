//! Errors, worded for the user.

use std::fmt;

use crate::script::{ScriptMode, refusal_sentence};

pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong talking to a database. Messages are shown to
/// the user as they are, so they must never contain a password.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("could not connect: {0}")]
    Connect(String),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("TLS: {0}")]
    Tls(String),
    #[error("SSH ({stage}): {message}")]
    Ssh { stage: SshStage, message: String },
    #[error("{message}")]
    Query {
        /// SQLSTATE (PostgreSQL, MySQL) or extended result code (SQLite).
        code: Option<String>,
        message: String,
        detail: Option<String>,
        hint: Option<String>,
    },
    #[error("the query was cancelled")]
    Cancelled,
    /// A script holds a statement that could end or change its
    /// transaction; nothing ran. `mode` is the run's, for the sentence.
    #[error("line {line}: {}", refusal_sentence(.mode, .what))]
    Refused {
        line: usize,
        what: String,
        mode: ScriptMode,
    },
    /// A save, or a script's run that writes, was asked of a connection
    /// that opens read-only. Nothing was sent.
    #[error("this connection opens read-only")]
    ReadOnly,
    /// A script left the session read-write. The session is closed.
    #[error("the script left the read-only transaction, so the session was closed")]
    LeftReadOnly,
    /// A script that writes ended the transaction its run commits. The
    /// session is closed.
    #[error("the script ended its transaction, so the session was closed")]
    LeftTransaction,
    #[error("the operation timed out")]
    Timeout,
    #[error("the connection was lost: {0}")]
    ConnectionLost(String),
    #[error("invalid connection settings: {0}")]
    InvalidSpec(String),
    #[error("not supported: {0}")]
    Unsupported(&'static str),
    #[error("{0}")]
    Io(String),
}

impl Error {
    /// A query error with only a message.
    pub fn query(message: impl Into<String>) -> Self {
        Self::Query {
            code: None,
            message: message.into(),
            detail: None,
            hint: None,
        }
    }

    /// Whether the session is unusable and must be reconnected.
    pub fn is_connection_lost(&self) -> bool {
        matches!(
            self,
            Self::ConnectionLost(_) | Self::LeftReadOnly | Self::LeftTransaction
        )
    }
}

/// Where an SSH tunnel failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshStage {
    Connect,
    /// First connection to this host: the user must trust the key.
    HostKeyUnknown {
        /// The server the tunnel reached (a Host alias's HostName).
        host: String,
        port: u16,
        fingerprint: String,
    },
    /// The host's key changed since it was trusted. Refused.
    HostKeyMismatch {
        host: String,
        port: u16,
        fingerprint: String,
    },
    Auth,
    /// The SSH password or key passphrase is missing or wrong: ask for it.
    Secret,
    Forward,
}

impl fmt::Display for SshStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => f.write_str("connect"),
            Self::HostKeyUnknown { fingerprint, .. } => write!(f, "unknown host key {fingerprint}"),
            Self::HostKeyMismatch { fingerprint, .. } => {
                write!(f, "changed host key {fingerprint}")
            }
            Self::Auth | Self::Secret => f.write_str("authentication"),
            Self::Forward => f.write_str("port forwarding"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_errors_show_their_message() {
        let error = Error::query("no such table: nope");
        assert_eq!(error.to_string(), "no such table: nope");
        assert!(matches!(error, Error::Query { code: None, .. }));
    }

    #[test]
    fn only_lost_connections_count_as_lost() {
        assert!(Error::ConnectionLost("reset".into()).is_connection_lost());
        assert!(!Error::Cancelled.is_connection_lost());
        assert!(!Error::query("x").is_connection_lost());
    }

    #[test]
    fn ssh_errors_name_their_stage() {
        let error = Error::Ssh {
            stage: SshStage::HostKeyUnknown {
                host: "bastion".into(),
                port: 22,
                fingerprint: "SHA256:abc".into(),
            },
            message: "unknown host key".into(),
        };
        assert_eq!(
            error.to_string(),
            "SSH (unknown host key SHA256:abc): unknown host key"
        );
    }

    #[test]
    fn leaving_read_only_counts_as_a_lost_connection() {
        assert!(Error::LeftReadOnly.is_connection_lost());
        let refused = Error::Refused {
            line: 4,
            what: "COMMIT".into(),
            mode: ScriptMode::ReadOnly,
        };
        assert!(!refused.is_connection_lost());
        assert_eq!(
            refused.to_string(),
            "line 4: Tabletist runs every query in a read-only transaction, so COMMIT is not allowed"
        );
    }

    #[test]
    fn a_refusal_in_a_run_that_writes_names_its_own_transaction() {
        let refused = Error::Refused {
            line: 2,
            what: "COMMIT".into(),
            mode: ScriptMode::Write,
        };
        assert_eq!(
            refused.to_string(),
            "line 2: Tabletist runs and commits the script in one transaction of its own, so \
             COMMIT is not allowed"
        );
    }

    #[test]
    fn leaving_the_transaction_counts_as_a_lost_connection() {
        assert!(Error::LeftTransaction.is_connection_lost());
        assert!(!Error::ReadOnly.is_connection_lost());
        assert_eq!(
            Error::ReadOnly.to_string(),
            "this connection opens read-only"
        );
    }
}
