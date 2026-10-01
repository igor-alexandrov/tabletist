//! Running a script typed into the SQL editor: what each statement did,
//! and how a run is told to stop.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::{ColumnMeta, Error, Result, Value};

/// What a script did: one result per statement that started, in order.
/// After an `Error` or `Cancelled` outcome no further statement runs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScriptOutcome {
    pub results: Vec<StatementResult>,
}

impl ScriptOutcome {
    /// Whether a stop ended the run: a statement was cancelled, or it was
    /// stopped before any statement began.
    pub fn was_cancelled(&self) -> bool {
        self.results.is_empty()
            || self
                .results
                .iter()
                .any(|result| result.outcome == StatementOutcome::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatementResult {
    pub elapsed: Duration,
    pub outcome: StatementOutcome,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatementOutcome {
    /// At most `limit` rows; `truncated` says more existed.
    Rows {
        columns: Vec<ColumnMeta>,
        rows: Vec<Vec<Value>>,
        truncated: bool,
    },
    /// A statement without a result set, with the rows it affected when
    /// the database says.
    Done { affected: Option<u64> },
    /// The statement failed. `position` is a 1-based character position in
    /// the statement's text (PostgreSQL reports one).
    Error {
        error: Error,
        position: Option<usize>,
    },
    /// Stopped while this statement ran. Its rows are dropped.
    Cancelled,
}

/// Tells a running script to stop: checked between statements, and by
/// SQLite while a statement runs. The backend sets it together with the
/// session's cancel.
#[derive(Debug, Clone, Default)]
pub struct StopFlag(Arc<AtomicBool>);

impl StopFlag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stop(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_stopped(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A statement's failure as its outcome: a cancel is `Cancelled`, a lost
/// session ends the whole run, anything else is the statement's error.
pub(crate) fn statement_failed(error: Error, position: Option<usize>) -> Result<StatementOutcome> {
    match error {
        Error::Cancelled => Ok(StatementOutcome::Cancelled),
        error if error.is_connection_lost() => Err(error),
        error => Ok(StatementOutcome::Error { error, position }),
    }
}

/// A cleanup failure closes the session: it may still be inside the
/// script's transaction.
pub(crate) fn cleanup_failed(error: &Error) -> Error {
    Error::ConnectionLost(format!("could not end the read-only transaction: {error}"))
}
