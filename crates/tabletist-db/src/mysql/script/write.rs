//! A SQL editor script that writes, on MySQL: one read-write transaction,
//! a note of every commit the server makes by itself, a commit when every
//! statement succeeded, and the session reset afterwards.

use std::time::{Duration, Instant};

use mysql_async::consts::StatusFlags;
use mysql_async::prelude::Queryable;

use super::super::{execute, prepare_session, query_error, status};
use super::{reset, run_statement};
use crate::script::{cannot_start, cleanup_failed, retry_cancelled};
use crate::{
    Access, Dialect, Error, Result, ScriptEnd, ScriptMode, ScriptOutcome, StatementOutcome,
    StatementResult, StopFlag,
};

/// See [`crate::Connection::run_script`], for [`ScriptMode::Write`]. The
/// caller has made sure the server can reset its session.
///
/// No `sql_select_limit` here: a `SELECT` that calls a function that
/// writes must run for every row, and no server version is trusted to keep
/// that limit off an `INSERT ... SELECT`. Rows past `limit + 1` are read
/// and dropped.
///
/// The future must be awaited to its end, as the read-only run's.
pub(super) async fn run(
    conn: &mut mysql_async::Conn,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
) -> Result<ScriptOutcome> {
    let mut outcome = ScriptOutcome::default();
    let mut run = Run::default();
    match begin(conn).await {
        // A lost session ends the run here: nothing to close.
        Ok(()) => statements(conn, texts, limit, stop, &mut outcome, &mut run).await?,
        // A cancel landed on the opening query: no results.
        Err(Error::Cancelled) => outcome.stopped = true,
        Err(error) if error.is_connection_lost() => return Err(error),
        // The transaction could not start. The next run would fail the
        // same way, so the session is closed, after an attempt to end what
        // is open.
        Err(error) => run.broken = Some(cannot_start(ScriptMode::Write, &error)),
    }
    close(conn, run, stop, outcome).await
}

/// What a run knows of its session beside its statements' results.
#[derive(Debug, Default)]
struct Run {
    /// How many of the script's first statements the server has committed
    /// by itself: it commits before DDL and some other statements, also
    /// when the statement then fails.
    committed: usize,
    /// A check could not be made, or a transaction could not start: the
    /// session is closed with this error, and the run's end is not told.
    broken: Option<Error>,
}

/// Starts a transaction, and holds the server to saying so: `inside` reads
/// the same mark later.
async fn begin(conn: &mut mysql_async::Conn) -> Result<()> {
    execute(conn, "START TRANSACTION").await?;
    if status(conn).contains(StatusFlags::SERVER_STATUS_IN_TRANS) {
        Ok(())
    } else {
        Err(Error::query("the server did not start a transaction"))
    }
}

/// Whether the session is inside a transaction, asked with a query of its
/// own whose answer carries the server's status. The status the driver
/// holds is no answer after a statement that failed: an error packet
/// carries none, and the driver empties what it held.
///
/// The status tells a transaction from none, not one transaction from
/// another. Without stored procedures no single statement can end the
/// run's transaction and leave another open, which is why `CALL` stays
/// refused.
async fn inside(conn: &mut mysql_async::Conn) -> Result<bool> {
    execute(conn, "DO 0").await?;
    Ok(status(conn).contains(StatusFlags::SERVER_STATUS_IN_TRANS))
}

/// The first words of the statements that never make the server commit:
/// those that read or change rows, and those that set, show or explain.
/// (`SET autocommit` would, and the refusal list keeps it out.)
const NEVER_COMMITS: [&str; 14] = [
    "SELECT", "INSERT", "UPDATE", "DELETE", "REPLACE", "WITH", "VALUES", "TABLE", "SET", "DO",
    "SHOW", "EXPLAIN", "DESCRIBE", "DESC",
];

/// Whether a statement of this text can make the server commit. When the
/// session is outside a transaction after a statement that cannot has
/// failed, the server has rolled the transaction back (a deadlock, a lock
/// wait it answers so): nothing of it is written. After any other
/// statement that failed, the session is outside because the server
/// committed before it ran the statement. It errs toward "can": a commit
/// that is said and did not happen sends the user to look, one that
/// happened and is not said does not.
fn can_commit(text: &str) -> bool {
    !crate::sql::words(Dialect::MySql, text)
        .first()
        .is_some_and(|word| NEVER_COMMITS.contains(&word.as_str()))
}

/// Runs the statements in order, up to the first that fails or is
/// stopped, noting in `run` what the server committed by itself. `Err` is
/// a lost session.
async fn statements(
    conn: &mut mysql_async::Conn,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
    outcome: &mut ScriptOutcome,
    run: &mut Run,
) -> Result<()> {
    for (index, text) in texts.iter().enumerate() {
        let checked = if stop.is_stopped() {
            Err(Error::Cancelled)
        } else {
            ready(conn, index, run).await
        };
        match checked {
            Ok(()) => {}
            // A stop between statements, or a cancel that landed on the
            // check: this statement is the cancelled one. The statement
            // before it may have made the server commit, which this check
            // did not get to see.
            Err(Error::Cancelled) => {
                outcome.stopped = true;
                outcome.results.push(StatementResult {
                    elapsed: Duration::ZERO,
                    outcome: StatementOutcome::Cancelled,
                });
                return settle(conn, stop, run, Some(index)).await;
            }
            Err(error) if error.is_connection_lost() => return Err(error),
            // Where the session stands is not known: nothing more runs.
            Err(error) => {
                run.broken = Some(Error::ConnectionLost(format!(
                    "could not ask where the session stands: {error}"
                )));
                return Ok(());
            }
        }
        let started = Instant::now();
        let result = run_statement(conn, text, limit).await?;
        outcome.stopped |= matches!(result, StatementOutcome::Cancelled);
        let last = !matches!(
            result,
            StatementOutcome::Rows { .. } | StatementOutcome::Done { .. }
        );
        outcome.results.push(StatementResult {
            elapsed: started.elapsed(),
            outcome: result,
        });
        if last {
            // Outside a transaction after a statement that can make the
            // server commit: it committed what came before. After one
            // that cannot, the server rolled the transaction back, and
            // what it committed earlier stands.
            return settle(conn, stop, run, can_commit(text).then_some(index)).await;
        }
    }
    Ok(())
}

/// Asks where the session stands once no more of the script will run: the
/// rollback that follows undoes only what is still in a transaction.
/// `outside` is how many statements are written when the session is found
/// outside one, or `None` when that means the server rolled back.
///
/// Nothing more of the script runs, so the backend is told to stop
/// repeating its cancel first; one that is on its way gets the question
/// asked once more. `Err` is a lost session.
async fn settle(
    conn: &mut mysql_async::Conn,
    stop: &StopFlag,
    run: &mut Run,
    outside: Option<usize>,
) -> Result<()> {
    stop.finish();
    match retry_cancelled!(inside(conn)) {
        Ok(true) => {}
        Ok(false) => {
            if let Some(committed) = outside {
                run.committed = committed;
            }
        }
        Err(error) if error.is_connection_lost() => return Err(error),
        Err(error) => {
            run.broken = Some(Error::ConnectionLost(format!(
                "could not ask where the session stands: {error}"
            )));
        }
    }
    Ok(())
}

/// The check before the statement at `index`: a session found outside a
/// transaction is there because the statement before this one made the
/// server commit. Everything before `index` is then written, and the next
/// statement gets a new transaction: none ever runs outside one.
///
/// `Err` is a cancel that landed on the check, a lost session, or a check
/// that could not be made.
async fn ready(conn: &mut mysql_async::Conn, index: usize, run: &mut Run) -> Result<()> {
    if !inside(conn).await? {
        run.committed = index;
        begin(conn).await?;
    }
    Ok(())
}

/// Ends the run: commits when every statement succeeded and no stop came,
/// rolls back otherwise, says what of the run is written, and resets the
/// session as the read-only run's close does. A step a cancel interrupted
/// runs once more.
///
/// `Err` is a run whose end is not known. It closes the session (it counts
/// as a lost connection).
async fn close(
    conn: &mut mysql_async::Conn,
    mut run: Run,
    stop: &StopFlag,
    mut outcome: ScriptOutcome,
) -> Result<ScriptOutcome> {
    // The backend is told before the flag is read, so a stop it sets from
    // here on sends no cancel, which would land on the commit or the
    // cleanup. One set before this line still wins over the commit.
    stop.finish();
    let stopped = stop.is_stopped();
    let succeeded = run.broken.is_none() && outcome.succeeded();
    let mut commit_error = None;
    if succeeded && stopped {
        outcome.stopped = true;
    } else if succeeded {
        match execute(conn, "COMMIT").await {
            Ok(()) => outcome.end = ScriptEnd::Committed,
            // A cancel that was on its way landed on the commit. Still
            // inside the transaction, nothing was committed, and the
            // rollback below undoes it. Outside one, the server does not
            // say which way the commit went.
            Err(Error::Cancelled) => match retry_cancelled!(inside(conn)) {
                Ok(true) => outcome.stopped = true,
                Ok(false) => {
                    run.broken = Some(Error::ConnectionLost(
                        "a cancel landed on the commit, and the server does not say whether it \
                         went through"
                            .into(),
                    ));
                }
                Err(error) if error.is_connection_lost() => return Err(error),
                Err(error) => run.broken = Some(cleanup_failed(ScriptMode::Write, &error)),
            },
            // Whether it went through cannot be known.
            Err(error) if error.is_connection_lost() => return Err(error),
            Err(error) => commit_error = Some(error),
        }
    }
    // Whatever is still open is rolled back and the session reset as far
    // as that works, also when the session is closed anyway.
    let rolled_back = match outcome.end {
        ScriptEnd::Committed => Ok(None),
        _ => roll_back(conn).await,
    };
    if let Ok(warning) = &rolled_back {
        outcome.rollback_warning.clone_from(warning);
    }
    if outcome.end != ScriptEnd::Committed {
        outcome.end = match commit_error {
            Some(error) => ScriptEnd::CommitFailed {
                error,
                committed: run.committed,
            },
            None if run.committed > 0 => ScriptEnd::Partly {
                committed: run.committed,
            },
            None => ScriptEnd::RolledBack,
        };
    }
    let reset = retry_cancelled!(reset(conn));
    // A run that did not end cleanly leaves the session read-only: it is
    // about to be closed, and until then nothing may write on it.
    let clean = run.broken.is_none() && rolled_back.is_ok() && reset.is_ok();
    let access = if clean {
        Access::Writable
    } else {
        Access::ReadOnly
    };
    let prepared = prepare_session(conn, access).await;
    match (run.broken, rolled_back) {
        (Some(error), _) => Err(error),
        (None, Err(error)) => Err(cleanup_failed(ScriptMode::Write, &error)),
        (None, Ok(_)) => Ok(outcome.put_back(reset.and(prepared))),
    }
}

/// What stands for a warning whose text could not be read.
const UNREAD_WARNING: &str =
    "the server raised a warning when it rolled back, and its text could not be read";

/// Rolls the run's transaction back, and gives what the server warned of
/// when it could not undo everything: a change to a table without
/// transactions stays (warning 1196). A rollback a cancel interrupted runs
/// once more. The warning is read before anything else is sent, since the
/// next statement replaces it, and it is asked for once: after a `SHOW
/// WARNINGS` that failed, a second would show that failure.
async fn roll_back(conn: &mut mysql_async::Conn) -> Result<Option<String>> {
    retry_cancelled!(execute(conn, "ROLLBACK"))?;
    if conn.get_warnings() == 0 {
        return Ok(None);
    }
    let warnings: mysql_async::Result<Vec<(String, u16, String)>> =
        conn.query("SHOW WARNINGS").await;
    match warnings.map_err(query_error) {
        Ok(warnings) => {
            let text = warnings
                .into_iter()
                .map(|(_, _, message)| message)
                .collect::<Vec<_>>()
                .join(" ");
            Ok(Some(if text.is_empty() {
                UNREAD_WARNING.to_owned()
            } else {
                text
            }))
        }
        Err(error) if error.is_connection_lost() => Err(error),
        Err(_) => Ok(Some(UNREAD_WARNING.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{probe, probe_rows};
    use super::*;
    use crate::mysql::Conn;
    use crate::mysql::tests::test_url;
    use crate::{ConnectSpec, TlsMode};

    #[test]
    fn only_a_statement_that_reads_or_changes_rows_cannot_make_the_server_commit() {
        for text in [
            "SELECT 1",
            "insert into t values (1)",
            "UPDATE t SET a = 1",
            "DELETE FROM t",
            "REPLACE INTO t VALUES (1)",
            "WITH x AS (SELECT 1) SELECT * FROM x",
            "(SELECT 1) UNION (SELECT 2)",
            "/* first */ UPDATE t SET a = 1",
            // They can lose a deadlock too, and commit nothing.
            "SET @held = (SELECT a FROM t WHERE a = 1 FOR UPDATE)",
            "DO refill()",
            "SHOW TABLES",
            "EXPLAIN ANALYZE UPDATE t SET a = 1",
            "DESCRIBE t",
        ] {
            assert!(!can_commit(text), "{text}");
        }
        for text in [
            "CREATE TABLE t (a int)",
            "ALTER TABLE t ADD b int",
            "DROP TABLE t",
            "TRUNCATE t",
            "RENAME TABLE t TO u",
            "ANALYZE TABLE t",
            "LOAD DATA INFILE 'x' INTO TABLE t",
            "",
        ] {
            assert!(can_commit(text), "{text}");
        }
    }

    /// A fresh writable session, as the app opens it.
    async fn writable(url: &str) -> Conn {
        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
        spec.tls = TlsMode::Disable;
        Conn::connect(&spec, &secrets, None, Access::Writable)
            .await
            .unwrap()
    }

    /// A statement the refusal stops long before it gets here. Run past
    /// it, it ends the run's transaction, which the next check takes for a
    /// commit the server made: what came before is said to be written, and
    /// the statement after it runs in a transaction of its own, which the
    /// failure at the end rolls back.
    #[tokio::test]
    async fn a_script_that_ended_its_transaction_never_runs_outside_one() {
        let Some(url) = test_url() else {
            return;
        };
        let (mut admin, _turn) = probe(&url).await;
        let conn = writable(&url).await;
        let texts: Vec<String> = [
            "INSERT INTO probe VALUES (1)",
            "COMMIT",
            "INSERT INTO probe VALUES (2)",
            "INSERT INTO probe_missing VALUES (3)",
        ]
        .iter()
        .map(|&text| text.to_owned())
        .collect();
        let outcome = tokio::time::timeout(
            Duration::from_secs(10),
            conn.run_script(&texts, 10, ScriptMode::Write, &StopFlag::new()),
        )
        .await
        .expect("the run hung")
        .unwrap();
        assert_eq!(outcome.end, ScriptEnd::Partly { committed: 2 });
        assert_eq!(probe_rows(&mut admin).await, 1);
    }

    /// A stop that comes after the last statement still wins over the
    /// commit.
    #[tokio::test]
    async fn a_stop_before_the_commit_rolls_back() {
        let Some(url) = test_url() else {
            return;
        };
        let (mut admin, _turn) = probe(&url).await;
        let conn = writable(&url).await;
        let mut conn = conn.conn.lock().await;
        begin(&mut conn).await.unwrap();
        execute(&mut conn, "INSERT INTO probe VALUES (1)")
            .await
            .unwrap();
        let stop = StopFlag::new();
        stop.stop();
        let outcome = close(&mut conn, Run::default(), &stop, ScriptOutcome::default())
            .await
            .unwrap();
        assert!(outcome.stopped);
        assert_eq!(outcome.end, ScriptEnd::RolledBack);
        assert!(stop.is_finishing());
        assert_eq!(probe_rows(&mut admin).await, 0);
    }
}
