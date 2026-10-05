//! Running a script typed into the SQL editor: what each statement did,
//! and how a run is told to stop.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::{ColumnMeta, Error, Result, Value};

/// How a script's transaction is meant to end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScriptMode {
    /// One read-only transaction, rolled back whatever the script did.
    #[default]
    ReadOnly,
    /// One read-write transaction: committed when every statement
    /// succeeded, rolled back after the first error, cancel or timeout.
    Write,
}

impl ScriptMode {
    /// The transaction a run of this mode is in, for a message.
    pub(crate) fn transaction(self) -> &'static str {
        match self {
            Self::ReadOnly => "the read-only transaction",
            Self::Write => "the transaction",
        }
    }
}

/// Why a statement the guard refused cannot run in a script of `mode`.
pub(crate) fn refusal_sentence(mode: &ScriptMode, what: &str) -> String {
    match mode {
        ScriptMode::ReadOnly => format!(
            "Tabletist runs every query in a read-only transaction, so {what} is not allowed"
        ),
        ScriptMode::Write => format!(
            "Tabletist runs and commits the script in one transaction of its own, so {what} is \
             not allowed"
        ),
    }
}

/// What remains of a script's work once its run is over.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ScriptEnd {
    /// Nothing: the transaction was rolled back. Every read-only run ends
    /// so.
    #[default]
    RolledBack,
    /// Every statement's work is written.
    Committed,
    /// The database committed on its own before the run failed or was
    /// stopped (MySQL, at DDL): the first `committed` statements are
    /// written, the rest is not.
    Partly { committed: usize },
    /// The commit itself failed, and what it would have kept is rolled
    /// back. `committed` is 0 wherever a transaction holds a whole run: then
    /// nothing is written. On MySQL it counts the statements the database
    /// had committed on its own before that, as `Partly` does.
    CommitFailed { error: Error, committed: usize },
}

/// What a script did: one result per statement that started, in order.
/// After an `Error` or `Cancelled` outcome no further statement runs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScriptOutcome {
    pub results: Vec<StatementResult>,
    /// A stop or cancel ended the run.
    pub stopped: bool,
    /// How the run's transaction ended.
    pub end: ScriptEnd,
    /// What the database said when it could not undo everything (MySQL's
    /// non-transactional tables). With it, `RolledBack` and `Partly` no
    /// longer say that the rest is gone.
    pub rollback_warning: Option<String>,
    /// A [`ScriptMode::Write`] run only: the session could not be put back
    /// after the run and must be closed. `end` still holds.
    pub broken: Option<Error>,
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

    /// Whether every statement that started ran to its end: what a run
    /// that writes needs before it commits.
    pub(crate) fn succeeded(&self) -> bool {
        !self.stopped
            && self.results.iter().all(|result| {
                matches!(
                    result.outcome,
                    StatementOutcome::Rows { .. } | StatementOutcome::Done { .. }
                )
            })
    }

    /// The outcome of a run that writes, once its transaction has ended
    /// and the session was put back, or could not be. The end is known
    /// either way, so it is told: a run that is committed is never shown
    /// as one that may be. A session that could not be put back is closed
    /// for it (`broken`).
    pub(crate) fn put_back(mut self, session: Result<()>) -> Self {
        if let Err(error) = session {
            self.broken = Some(cleanup_failed(ScriptMode::Write, &error));
        }
        self
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
    /// the database says (`None` when it has no meaningful count), and the
    /// warnings it raised (MySQL counts them; the others have none).
    Done {
        affected: Option<u64>,
        warnings: u16,
    },
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
pub(crate) fn cleanup_failed(mode: ScriptMode, error: &Error) -> Error {
    Error::ConnectionLost(format!("could not end {}: {error}", mode.transaction()))
}

/// A transaction that could not start closes the session: the next run
/// would fail the same way.
pub(crate) fn cannot_start(mode: ScriptMode, error: &Error) -> Error {
    Error::ConnectionLost(format!("could not start {}: {error}", mode.transaction()))
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
    fn an_outcome_starts_rolled_back_and_whole() {
        let outcome = ScriptOutcome::default();
        assert_eq!(outcome.end, ScriptEnd::RolledBack);
        assert_eq!(outcome.rollback_warning, None);
        assert_eq!(outcome.broken, None);
        assert_eq!(ScriptMode::default(), ScriptMode::ReadOnly);
    }

    #[test]
    fn a_run_whose_session_could_not_be_put_back_keeps_its_end() {
        let committed = ScriptOutcome {
            end: ScriptEnd::Committed,
            ..ScriptOutcome::default()
        };
        assert_eq!(committed.clone().put_back(Ok(())), committed);
        let told = committed.put_back(Err(Error::query("no")));
        assert_eq!(told.end, ScriptEnd::Committed);
        let broken = told.broken.expect("the session is to be closed");
        assert!(broken.is_connection_lost());
        assert_eq!(
            broken.to_string(),
            "the connection was lost: could not end the transaction: no"
        );
    }

    #[test]
    fn a_run_succeeded_when_every_statement_ran_to_its_end() {
        let result = |outcome| StatementResult {
            elapsed: Duration::ZERO,
            outcome,
        };
        let done = StatementOutcome::Done {
            affected: Some(1),
            warnings: 0,
        };
        let failed = StatementOutcome::Error {
            error: Error::query("no"),
            position: None,
        };
        let of = |outcomes: Vec<StatementOutcome>, stopped| ScriptOutcome {
            results: outcomes.into_iter().map(result).collect(),
            stopped,
            ..ScriptOutcome::default()
        };
        assert!(of(vec![done.clone(), done.clone()], false).succeeded());
        assert!(of(Vec::new(), false).succeeded());
        assert!(!of(vec![done.clone(), failed], false).succeeded());
        assert!(!of(vec![done.clone(), StatementOutcome::Cancelled], true).succeeded());
        // Stopped after its last statement.
        assert!(!of(vec![done], true).succeeded());
    }

    #[test]
    fn a_failed_start_or_end_names_the_modes_transaction() {
        let error = Error::query("no");
        for (mode, start, end) in [
            (
                ScriptMode::ReadOnly,
                "the connection was lost: could not start the read-only transaction: no",
                "the connection was lost: could not end the read-only transaction: no",
            ),
            (
                ScriptMode::Write,
                "the connection was lost: could not start the transaction: no",
                "the connection was lost: could not end the transaction: no",
            ),
        ] {
            assert_eq!(cannot_start(mode, &error).to_string(), start);
            assert_eq!(cleanup_failed(mode, &error).to_string(), end);
        }
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
