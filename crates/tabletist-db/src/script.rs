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
    /// A stop or cancel ended the run.
    pub stopped: bool,
}

impl ScriptOutcome {
    /// Whether a stop or cancel ended the run: a statement was cancelled,
    /// or the run was stopped before any statement began. A script with no
    /// statements is not cancelled.
    pub fn was_cancelled(&self) -> bool {
        self.stopped
            || self
                .results
                .iter()
                .any(|result| result.outcome == StatementOutcome::Cancelled)
    }
}

/// One statement's outcome and how long it took.
#[derive(Debug, Clone, PartialEq)]
pub struct StatementResult {
    pub elapsed: Duration,
    pub outcome: StatementOutcome,
}

/// What one statement of a script did.
#[derive(Debug, Clone, PartialEq)]
pub enum StatementOutcome {
    /// At most `limit` rows; `truncated` says more existed.
    Rows {
        columns: Vec<ColumnMeta>,
        rows: Vec<Vec<Value>>,
        truncated: bool,
    },
    /// A statement without a result set, with the rows it affected when
    /// the database says (`None` when it has no meaningful count).
    Done { affected: Option<u64> },
    /// The statement failed. `position` is a 1-based character position in
    /// the statement's text (PostgreSQL reports one).
    Error {
        error: Error,
        position: Option<usize>,
    },
    /// Stopped while this statement ran, or before it began (a stop between
    /// statements records the next statement as `Cancelled`). Its rows are
    /// dropped.
    Cancelled,
}

/// Tells a running script or save to stop: checked between statements,
/// and by SQLite while a script's statement runs. The backend sets it
/// together with the session's cancel.
///
/// One flag per run: a stop cannot be undone, and dropping the run sets it.
#[derive(Debug, Clone, Default)]
pub struct StopFlag(Arc<Flags>);

#[derive(Debug, Default)]
struct Flags {
    stopped: AtomicBool,
    finishing: AtomicBool,
}

impl StopFlag {
    /// A flag that is not stopped.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stops every run holding this flag or a clone of it. Says whether
    /// this call was the first to stop it, so of a user's cancel and a
    /// timeout only one is the reason the run stopped.
    pub fn stop(&self) -> bool {
        !self.0.stopped.swap(true, Ordering::SeqCst)
    }

    /// Whether [`StopFlag::stop`] was called.
    pub fn is_stopped(&self) -> bool {
        self.0.stopped.load(Ordering::SeqCst)
    }

    /// Set by the driver when the run's cleanup begins: a cancel sent after
    /// this could land on the cleanup instead of a statement, so the backend
    /// stops repeating its cancel.
    pub fn finish(&self) {
        self.0.finishing.store(true, Ordering::SeqCst);
    }

    /// Whether the run's cleanup has begun (see [`StopFlag::finish`]).
    pub fn is_finishing(&self) -> bool {
        self.0.finishing.load(Ordering::SeqCst)
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

/// Runs a cleanup step again once when a cancel meant for a statement
/// landed on it instead: `retry_cancelled!(rollback(&client))` evaluates
/// the expression (a future of `Result<T>`) a second time. A macro, not a
/// function taking an async closure: a closure borrowing the connection
/// makes the run's future not `Send`, and the backend spawns it.
macro_rules! retry_cancelled {
    ($step:expr) => {
        match $step.await {
            Err($crate::Error::Cancelled) => $step.await,
            other => other,
        }
    };
}
pub(crate) use retry_cancelled;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopping_and_finishing_are_separate_and_shared_by_clones() {
        let flag = StopFlag::new();
        let clone = flag.clone();
        assert!(!flag.is_stopped() && !flag.is_finishing());
        clone.finish();
        assert!(flag.is_finishing());
        assert!(!flag.is_stopped());
        clone.stop();
        assert!(flag.is_stopped());
        assert!(flag.is_finishing());
    }

    #[test]
    fn only_the_first_stop_says_it_stopped_the_run() {
        let flag = StopFlag::new();
        let clone = flag.clone();
        assert!(clone.stop());
        assert!(!flag.stop());
        assert!(!clone.stop());
        assert!(flag.is_stopped());
    }
}
