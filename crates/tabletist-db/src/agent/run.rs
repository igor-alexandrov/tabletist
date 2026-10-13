//! Running a statement that passed the check: how much it may take, and
//! what it answered.

use std::fmt;
use std::time::Duration;

use super::Param;
use crate::{ColumnMeta, Error, Result, ScriptOutcome, StatementOutcome, StopFlag, Value};

/// How much one read of an agent's may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most rows of an answer.
    pub rows: u32,
    /// How long the database lets the statement run, by its own clock. A
    /// time under a millisecond is taken for one: PostgreSQL and MySQL
    /// read 0 as no limit.
    pub timeout: Duration,
}

/// What a read answered.
#[derive(Clone, PartialEq)]
pub struct Answer {
    pub columns: Vec<ColumnMeta>,
    /// At most [`Limits::rows`] of them.
    pub rows: Vec<Vec<Value>>,
    /// Whether the statement had more rows than were kept.
    pub truncated: bool,
    /// How long the statement took.
    pub elapsed: Duration,
}

/// Without the rows: what a database holds is not logged.
impl fmt::Debug for Answer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Answer")
            .field("columns", &self.columns.len())
            .field("rows", &self.rows.len())
            .field("truncated", &self.truncated)
            .field("elapsed", &self.elapsed)
            .finish()
    }
}

/// What a driver's read-only run takes beyond a script's when its one
/// statement is an agent's: the values for the statement's placeholders,
/// and how long the database may take over it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Sent<'a> {
    pub params: &'a [Param],
    pub timeout: Duration,
}

impl Sent<'_> {
    /// The timeout in whole milliseconds: never 0, which PostgreSQL and
    /// MySQL read as no limit, and never more than PostgreSQL's setting
    /// holds.
    pub fn timeout_ms(&self) -> u32 {
        u32::try_from(self.timeout.as_millis())
            .unwrap_or(u32::MAX)
            .clamp(1, i32::MAX as u32)
    }
}

/// The error of a statement the database counts another number of
/// placeholders in than values were sent for. The check counted them too
/// ([`super::check_read`]), by its own reading: where the two disagree the
/// database's count is the one that holds, and nothing runs.
pub(crate) fn count_mismatch(counted: usize, sent: usize) -> Error {
    let plural = |count: usize| if count == 1 { "" } else { "s" };
    Error::query(format!(
        "the database counts {counted} parameter{} in the statement, and {sent} value{} came \
         with it",
        plural(counted),
        plural(sent)
    ))
}

/// The answer in the outcome of a read-only run of one statement, or why
/// there is none: the statement's own error, [`Error::Cancelled`] when
/// `stop` ended it, and [`Error::Timeout`] when the database did.
pub(crate) fn answer(outcome: ScriptOutcome, stop: &StopFlag) -> Result<Answer> {
    let Some(result) = outcome.results.into_iter().next() else {
        // A cancel landed before the statement began.
        return Err(Error::Cancelled);
    };
    match result.outcome {
        StatementOutcome::Rows {
            columns,
            rows,
            truncated,
        } => Ok(Answer {
            columns,
            rows,
            truncated,
            elapsed: result.elapsed,
        }),
        // The check lets through a query or an `EXPLAIN` of one, and both
        // answer rows. In the read-only transaction this wrote nothing.
        StatementOutcome::Done { .. } => Err(Error::query(
            "the statement answered no rows, so it is not a read",
        )),
        StatementOutcome::Error { error, .. } => Err(error),
        StatementOutcome::Cancelled if stop.is_stopped() => Err(Error::Cancelled),
        // Nobody here stopped it, so the database's own timeout did:
        // PostgreSQL ends a statement over its time as it ends a cancelled
        // one, and SQLite's is interrupted by the run's deadline. A
        // statement someone ended at the server reads the same.
        StatementOutcome::Cancelled => Err(Error::Timeout),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StatementResult, ValueKind};

    fn outcome_of(outcome: StatementOutcome) -> ScriptOutcome {
        ScriptOutcome {
            results: vec![StatementResult {
                elapsed: Duration::from_millis(7),
                outcome,
            }],
            ..ScriptOutcome::default()
        }
    }

    fn titles() -> StatementOutcome {
        StatementOutcome::Rows {
            columns: vec![ColumnMeta {
                name: "title".into(),
                type_name: "text".into(),
                kind: ValueKind::Text,
            }],
            rows: vec![vec![Value::Text("Lanterns".into())]],
            truncated: true,
        }
    }

    #[test]
    fn rows_are_the_answer() {
        let answer = answer(outcome_of(titles()), &StopFlag::new()).unwrap();
        assert_eq!(answer.columns[0].name, "title");
        assert_eq!(answer.rows, [[Value::Text("Lanterns".into())]]);
        assert!(answer.truncated);
        assert_eq!(answer.elapsed, Duration::from_millis(7));
    }

    #[test]
    fn an_answer_prints_without_its_rows() {
        let answer = answer(outcome_of(titles()), &StopFlag::new()).unwrap();
        assert_eq!(
            format!("{answer:?}"),
            "Answer { columns: 1, rows: 1, truncated: true, elapsed: 7ms }"
        );
    }

    #[test]
    fn a_statement_that_failed_is_its_error() {
        let failed = StatementOutcome::Error {
            error: Error::query("no such column: nope"),
            position: None,
        };
        assert_eq!(
            answer(outcome_of(failed), &StopFlag::new()),
            Err(Error::query("no such column: nope"))
        );
    }

    #[test]
    fn a_statement_that_was_ended_says_who_ended_it() {
        // Nobody stopped it, so the database's own timeout did.
        assert_eq!(
            answer(outcome_of(StatementOutcome::Cancelled), &StopFlag::new()),
            Err(Error::Timeout)
        );
        let stopped = StopFlag::new();
        stopped.stop();
        assert_eq!(
            answer(outcome_of(StatementOutcome::Cancelled), &stopped),
            Err(Error::Cancelled)
        );
        // A run that was ended before its statement began has no result.
        assert_eq!(
            answer(ScriptOutcome::default(), &stopped),
            Err(Error::Cancelled)
        );
    }

    #[test]
    fn a_statement_without_rows_is_no_read() {
        let done = StatementOutcome::Done {
            affected: None,
            warnings: 0,
        };
        let error = answer(outcome_of(done), &StopFlag::new()).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the statement answered no rows, so it is not a read"
        );
    }

    #[test]
    fn a_timeout_is_whole_milliseconds_and_never_none() {
        let sent = |timeout: Duration| Sent {
            params: &[],
            timeout,
        };
        assert_eq!(sent(Duration::from_secs(10)).timeout_ms(), 10_000);
        assert_eq!(sent(Duration::from_micros(1_999)).timeout_ms(), 1);
        // 0 would lift the limit.
        assert_eq!(sent(Duration::ZERO).timeout_ms(), 1);
        assert_eq!(sent(Duration::from_micros(10)).timeout_ms(), 1);
        assert_eq!(sent(Duration::MAX).timeout_ms(), 2_147_483_647);
    }

    #[test]
    fn a_count_the_database_disagrees_with_is_said_in_its_numbers() {
        assert_eq!(
            count_mismatch(1, 2).to_string(),
            "the database counts 1 parameter in the statement, and 2 values came with it"
        );
        assert_eq!(
            count_mismatch(2, 1).to_string(),
            "the database counts 2 parameters in the statement, and 1 value came with it"
        );
    }
}
