//! A SQL editor script that writes, on PostgreSQL: one read-write
//! transaction, the guard's savepoint before every statement, a commit when
//! every statement succeeded, and the session put back afterwards.

use super::super::session_setup;
use super::{Tx, in_transaction, own_failed, rollback, statements, unlock, unlocked};
use crate::pg::query_error;
use crate::script::{cannot_start, cleanup_failed, retry_cancelled};
use crate::{Access, Error, Result, ScriptEnd, ScriptMode, ScriptOutcome, StopFlag};

/// See [`crate::Connection::run_script`], for [`ScriptMode::Write`].
///
/// The future must be awaited to its end, as the read-only run's.
pub(super) async fn run(
    client: &tokio_postgres::Client,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
) -> Result<ScriptOutcome> {
    let mut outcome = ScriptOutcome::default();
    let tx = match open(client).await {
        // A lost session ends the run here: nothing to close.
        Ok(Tx::Open) => {
            statements(client, texts, limit, ScriptMode::Write, stop, &mut outcome).await?
        }
        // A cancel landed on the opening queries: no results.
        Ok(tx) => {
            outcome.stopped = true;
            tx
        }
        Err(error) if error.is_connection_lost() => return Err(error),
        // The transaction could not start. The next run would fail the
        // same way, so the session is closed, after an attempt to end what
        // is open.
        Err(error) => {
            stop.finish();
            rollback(client).await.ok();
            return Err(cannot_start(ScriptMode::Write, &error));
        }
    };
    close(client, tx, stop, outcome).await
}

/// Starts the script's transaction and sets the guard's savepoint (see
/// `in_transaction`). A plain `BEGIN`: the transaction takes the server's
/// and the role's defaults, so a role an administrator made read-only
/// stays so, and its error is the statement's. `Aborted` when a cancel
/// landed on these queries; an `Err` that is not a lost session means the
/// transaction could not start.
async fn open(client: &tokio_postgres::Client) -> Result<Tx> {
    for step in ["BEGIN", "SAVEPOINT tabletist_guard"] {
        match client.batch_execute(step).await.map_err(query_error) {
            Ok(()) => {}
            Err(Error::Cancelled) => return Ok(Tx::Aborted),
            Err(error) => return Err(error),
        }
    }
    Ok(Tx::Open)
}

/// Where a transaction the run believes open really stands, asked after
/// the last statement: the guard's swap once more. PostgreSQL never commits
/// by itself, so a transaction that is gone means a statement the refusal
/// missed ended it.
///
/// This is also what makes an answered `COMMIT` mean committed. In a
/// failed transaction PostgreSQL answers `COMMIT` with a rollback and no
/// error, and the driver does not show which; after a swap that worked the
/// transaction is not a failed one.
async fn confirm(client: &tokio_postgres::Client) -> Result<Tx> {
    match in_transaction(client).await {
        // A failed block, or a cancel on the check, which fails the block
        // if there is one: the check cannot say whose it is.
        Ok(Tx::Aborted) | Err(Error::Cancelled) => retry_cancelled!(own_failed(client)),
        other => other,
    }
}

/// Ends the run: commits when every statement succeeded, the transaction
/// is still the script's own and no stop came, and rolls back otherwise;
/// then puts the session back.
///
/// `Err` is a run whose end is not known, or a script that left its
/// transaction. Either closes the session (they count as a lost
/// connection).
async fn close(
    client: &tokio_postgres::Client,
    tx: Tx,
    stop: &StopFlag,
    mut outcome: ScriptOutcome,
) -> Result<ScriptOutcome> {
    // A run that will not commit runs nothing more of the script: the
    // backend is told to stop repeating its cancel before the check, which
    // one that is on its way can still land on.
    let committing = tx == Tx::Open && outcome.succeeded() && !stop.is_stopped();
    if !committing {
        stop.finish();
    }
    let tx = match tx {
        Tx::Open => confirm(client).await,
        known => Ok(known),
    };
    // The backend is told before the flag is read, so a stop it sets from
    // here on sends no cancel, which would land on the commit or the
    // cleanup. One set before this line still wins over the commit.
    stop.finish();
    let stopped = stop.is_stopped();
    let commit = committing && matches!(tx, Ok(Tx::Open)) && !stopped;
    if commit {
        match client.batch_execute("COMMIT").await.map_err(query_error) {
            Ok(()) => outcome.end = ScriptEnd::Committed,
            // A cancel that was on its way landed on the commit's own
            // work. The server has rolled the transaction back.
            Err(Error::Cancelled) => outcome.stopped = true,
            // Whether it went through cannot be known.
            Err(error) if error.is_connection_lost() => return Err(error),
            // A deferred constraint, a serialization failure. The server
            // has rolled the transaction back.
            Err(error) => {
                outcome.end = ScriptEnd::CommitFailed {
                    error,
                    committed: 0,
                };
            }
        }
    } else if outcome.succeeded() {
        match tx {
            // Stopped after the last statement.
            _ if stopped => outcome.stopped = true,
            // Every statement ran, and still the transaction failed: said
            // rather than a run that ends rolled back without a word.
            Ok(Tx::Aborted) => {
                outcome.end = ScriptEnd::CommitFailed {
                    error: Error::query("the transaction failed before it could be committed"),
                    committed: 0,
                };
            }
            _ => {}
        }
    }
    // Whatever is open is rolled back as far as that works, also when the
    // session is closed anyway. After a COMMIT that failed the server has
    // nothing left to roll back, and only warns.
    let rolled_back = match outcome.end {
        ScriptEnd::Committed => Ok(()),
        _ => retry_cancelled!(rollback(client)),
    };
    match (tx, rolled_back) {
        (Ok(Tx::Left), _) => Err(Error::LeftTransaction),
        (Err(error), _) | (_, Err(error)) => Err(cleanup_failed(ScriptMode::Write, &error)),
        (Ok(_), Ok(())) => Ok(outcome.put_back(put_back(client).await)),
    }
}

/// What a script can leave in a session once its transaction is committed,
/// undone: a rolled back transaction takes these with it, a committed one
/// keeps them, and none of it may reach table browsing or the next run.
/// `RESET ALL` leaves the role and the session's authorization alone, so
/// they are reset by name. Temporary tables last for the session.
const PUT_BACK: &str = "CLOSE ALL; UNLISTEN *; RESET SESSION AUTHORIZATION; RESET ROLE; RESET ALL";

/// Puts the session back as it connected: the script's cursors, channels,
/// role and settings gone, the connect-time settings set again, and the
/// session's advisory locks released. A step a cancel interrupted runs
/// once more.
async fn put_back(client: &tokio_postgres::Client) -> Result<()> {
    let connected = session_setup(Access::Writable);
    for step in [PUT_BACK, connected.as_str()] {
        retry_cancelled!(execute(client, step))?;
    }
    unlocked(retry_cancelled!(unlock(client)))
}

async fn execute(client: &tokio_postgres::Client, sql: &str) -> Result<()> {
    client.batch_execute(sql).await.map_err(query_error)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::super::tests::{probe, probe_rows};
    use super::*;
    use crate::ConnectSpec;
    use crate::pg::Conn;
    use crate::pg::tests::test_url;

    /// A fresh writable session, as the app opens it.
    async fn writable(url: &str) -> Conn {
        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
        spec.tls = crate::TlsMode::Disable;
        Conn::connect(&spec, &secrets, None, Access::Writable)
            .await
            .unwrap()
    }

    /// Runs statements that write straight through the driver, as if the
    /// refusal had missed them.
    async fn past_the_refusal(conn: &Conn, script: &[&str]) -> Result<ScriptOutcome> {
        let texts: Vec<String> = script.iter().map(|&text| text.to_owned()).collect();
        tokio::time::timeout(
            Duration::from_secs(10),
            conn.run_script(&texts, 10, ScriptMode::Write, &StopFlag::new()),
        )
        .await
        .expect("the run hung")
    }

    /// Statements the refusal stops long before they get here. Run past
    /// it, they end the transaction the run means to commit. The statement
    /// after them must not run, because it would run outside any
    /// transaction, and the run must fail even when nothing follows them,
    /// so that the session is closed.
    #[tokio::test]
    async fn a_script_that_ended_its_transaction_runs_nothing_more_and_fails() {
        let Some(url) = test_url() else {
            return;
        };
        let (admin, _turn) = probe(&url).await;
        for (script, written) in [
            // The script's own commit stands. Nothing ran after it.
            (
                vec![
                    "INSERT INTO probe VALUES (1)",
                    "COMMIT",
                    "INSERT INTO probe VALUES (2)",
                ],
                1,
            ),
            (vec!["INSERT INTO probe VALUES (1)", "COMMIT"], 1),
            (
                vec![
                    "INSERT INTO probe VALUES (1)",
                    "ROLLBACK",
                    "INSERT INTO probe VALUES (2)",
                ],
                0,
            ),
            // A new transaction, which is not the run's.
            (
                vec![
                    "INSERT INTO probe VALUES (1)",
                    "COMMIT AND CHAIN",
                    "INSERT INTO probe VALUES (2)",
                ],
                1,
            ),
        ] {
            admin.batch_execute("TRUNCATE probe").await.unwrap();
            let conn = writable(&url).await;
            let ran = past_the_refusal(&conn, &script).await;
            assert_eq!(ran, Err(Error::LeftTransaction), "{script:?}");
            assert_eq!(probe_rows(&admin).await, written, "{script:?}");
        }
    }

    /// A transaction that failed with every statement done cannot be
    /// committed: PostgreSQL would answer the COMMIT with a rollback and no
    /// error. The check before the commit sees it.
    #[tokio::test]
    async fn a_transaction_that_failed_behind_the_run_is_not_called_committed() {
        let Some(url) = test_url() else {
            return;
        };
        let (admin, _turn) = probe(&url).await;
        let conn = writable(&url).await;
        let client = conn.client.lock().await;
        assert_eq!(open(&client).await, Ok(Tx::Open));
        client
            .batch_execute("INSERT INTO probe VALUES (1)")
            .await
            .unwrap();
        // Fails the block the way no statement of a run can without the
        // run seeing it.
        assert!(client.batch_execute("SELECT 1 / 0").await.is_err());
        let outcome = close(
            &client,
            Tx::Open,
            &StopFlag::new(),
            ScriptOutcome::default(),
        )
        .await
        .unwrap();
        assert!(
            matches!(outcome.end, ScriptEnd::CommitFailed { committed: 0, .. }),
            "{:?}",
            outcome.end
        );
        assert_eq!(probe_rows(&admin).await, 0);
    }

    /// A stop that comes after the last statement still wins over the
    /// commit.
    #[tokio::test]
    async fn a_stop_before_the_commit_rolls_back() {
        let Some(url) = test_url() else {
            return;
        };
        let (admin, _turn) = probe(&url).await;
        let conn = writable(&url).await;
        let client = conn.client.lock().await;
        assert_eq!(open(&client).await, Ok(Tx::Open));
        client
            .batch_execute("INSERT INTO probe VALUES (1)")
            .await
            .unwrap();
        let stop = StopFlag::new();
        stop.stop();
        let outcome = close(&client, Tx::Open, &stop, ScriptOutcome::default())
            .await
            .unwrap();
        assert!(outcome.stopped);
        assert_eq!(outcome.end, ScriptEnd::RolledBack);
        assert!(stop.is_finishing());
        assert_eq!(probe_rows(&admin).await, 0);
    }
}
