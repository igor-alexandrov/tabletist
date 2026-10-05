//! Running a SQL editor script on PostgreSQL: one transaction managed by
//! hand, the guard's savepoint around every statement, and the close that
//! asks the server where the session stands. This file is the run that
//! only reads, in a read-only transaction that is rolled back; `write` is
//! the run that commits.

use std::time::{Duration, Instant};

use tokio_postgres::SimpleQueryMessage;
use tokio_postgres::error::SqlState;

use super::{Conn, column_metas, first_text, query_error, row_values};
use crate::script::{cannot_start, cleanup_failed, retry_cancelled, statement_failed};
use crate::{
    ColumnMeta, Dialect, Error, Result, ScriptMode, ScriptOutcome, StatementOutcome,
    StatementResult, StopFlag,
};

mod write;

impl Conn {
    /// See [`crate::Connection::run_script`]. The transaction is managed by
    /// hand: `tokio_postgres::Transaction` has no streaming simple query.
    ///
    /// The future must be awaited to its end: a run that is dropped leaves
    /// the session inside the transaction. The backend never drops one; it
    /// stops a run through `stop` and the session's cancel.
    pub(super) async fn script(
        &self,
        texts: &[String],
        limit: u32,
        mode: ScriptMode,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let client = self.client.lock().await;
        if mode == ScriptMode::Write {
            return write::run(&client, texts, limit as usize, stop).await;
        }
        let mut outcome = ScriptOutcome::default();
        let tx = match open(&client).await {
            // A lost session ends the run here: nothing to close.
            Ok(Tx::Open) => {
                statements(&client, texts, limit as usize, mode, stop, &mut outcome).await?
            }
            // A cancel landed on the opening queries: no results.
            Ok(tx) => {
                outcome.stopped = true;
                tx
            }
            Err(error) if error.is_connection_lost() => return Err(error),
            // The transaction could not start, say because the session is
            // stuck in a failed one. The next run would fail the same way,
            // so the session is closed, after an attempt to end what is
            // open.
            Err(error) => {
                stop.finish();
                close(&client, Tx::Aborted).await.ok();
                return Err(cannot_start(ScriptMode::ReadOnly, &error));
            }
        };
        // From here on a cancel would land on the cleanup: tell the
        // backend to stop repeating its cancel.
        stop.finish();
        close(&client, tx).await?;
        Ok(outcome)
    }
}

/// What a SQL editor row statement runs as: `DECLARE` with this prefix,
/// then `FETCH` and `CLOSE`.
const CURSOR_PREFIX: &str = "DECLARE tabletist_sql NO SCROLL CURSOR FOR ";

/// Where a script's session stands relative to its read-only transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tx {
    /// Inside it, and nothing the run saw says it failed. The close asks
    /// the server before it trusts this.
    Open,
    /// Inside it, but it failed (an error or a cancel): it cannot write,
    /// and `ROLLBACK` ends it. A failed `PREPARE TRANSACTION` is the one
    /// failure that leaves the block; the server has then rolled it back
    /// itself and nothing runs after it, so `ROLLBACK` finds nothing to do.
    Aborted,
    /// Not where the script started: outside the transaction, or in one
    /// that is no longer read-only.
    Left,
}

/// Starts the script's transaction: `BEGIN READ ONLY`, the snapshot, and
/// the guard's savepoint (see `in_transaction`). PostgreSQL refuses to make
/// a transaction read-write once it has a snapshot, whatever a statement
/// tries. `Aborted` when a cancel landed on these queries; an `Err` that is
/// not a lost session means the transaction could not start.
async fn open(client: &tokio_postgres::Client) -> Result<Tx> {
    for step in ["BEGIN READ ONLY", "SELECT 1", "SAVEPOINT tabletist_guard"] {
        match client.batch_execute(step).await.map_err(query_error) {
            Ok(()) => {}
            Err(Error::Cancelled) => return Ok(Tx::Aborted),
            Err(error) => return Err(error),
        }
    }
    Ok(Tx::Open)
}

/// Runs the statements in order, up to the first that fails or is
/// stopped, and says where that leaves the transaction. `Err` is a lost
/// session.
async fn statements(
    client: &tokio_postgres::Client,
    texts: &[String],
    limit: usize,
    mode: ScriptMode,
    stop: &StopFlag,
    outcome: &mut ScriptOutcome,
) -> Result<Tx> {
    for text in texts {
        let ready = if stop.is_stopped() {
            Err(Error::Cancelled)
        } else {
            in_transaction(client).await
        };
        let failure = match ready {
            Ok(Tx::Open) => None,
            Ok(Tx::Left) => return Ok(Tx::Left),
            // Not after a statement that succeeded; said rather than a
            // run that ends without a word. Whose failed transaction it
            // is, the close asks.
            Ok(Tx::Aborted) => Some((
                StatementOutcome::Error {
                    error: Error::query("the transaction failed before this statement ran"),
                    position: None,
                },
                Tx::Open,
            )),
            // A stop between statements, or a cancel that landed on the
            // check: this statement is the cancelled one. A cancel fails
            // the transaction block the session is in, if it is in one;
            // whether that is still the script's, the close asks.
            Err(Error::Cancelled) => {
                outcome.stopped = true;
                Some((StatementOutcome::Cancelled, Tx::Open))
            }
            Err(error) => return Err(error),
        };
        if let Some((result, tx)) = failure {
            outcome.results.push(StatementResult {
                elapsed: Duration::ZERO,
                outcome: result,
            });
            return Ok(tx);
        }
        let started = Instant::now();
        let (result, tx) = run_statement(client, text, limit, mode, stop).await?;
        outcome.stopped |= result == StatementOutcome::Cancelled;
        let last = !matches!(
            result,
            StatementOutcome::Rows { .. } | StatementOutcome::Done { .. }
        );
        outcome.results.push(StatementResult {
            elapsed: started.elapsed(),
            outcome: result,
        });
        if last {
            return Ok(tx);
        }
    }
    Ok(Tx::Open)
}

/// Ends the script's transaction, whatever state it is in: confirms an
/// open one is the script's own and still read-only, rolls back, and
/// releases the session's advisory locks. `LeftReadOnly`, a failed check
/// or a failed rollback closes the session (they count as a lost
/// connection).
async fn close(client: &tokio_postgres::Client, tx: Tx) -> Result<()> {
    let tx = match tx {
        Tx::Open => confirm(client).await,
        known => Ok(known),
    };
    // Whatever is open is rolled back as far as that works, also when the
    // session is closed anyway.
    let rolled_back = retry_cancelled!(rollback(client));
    match (tx, rolled_back) {
        (Ok(Tx::Left), _) => Err(Error::LeftReadOnly),
        (Err(error), _) | (_, Err(error)) => Err(cleanup_failed(ScriptMode::ReadOnly, &error)),
        (Ok(_), Ok(())) => unlocked(retry_cancelled!(unlock(client))),
    }
}

/// What a failed release of the advisory locks means for a run whose
/// transaction did end: nothing, unless the session is lost. The outcome
/// stands and the session stays; a lock the script took may stay with it.
fn unlocked(released: Result<()>) -> Result<()> {
    match released {
        Err(error) if error.is_connection_lost() => Err(error),
        // Cancelled twice: given up quietly.
        Ok(()) | Err(Error::Cancelled) => Ok(()),
        Err(error) => {
            log::warn!("could not release the advisory locks after a script: {error}");
            Ok(())
        }
    }
}

/// Where a transaction the run believes open really stands, asked after
/// the last statement: a statement in last position may have ended it, and
/// what a `COMMIT` kept (a setting) must not reach later browsing.
async fn confirm(client: &tokio_postgres::Client) -> Result<Tx> {
    match in_transaction(client).await {
        Ok(Tx::Open) => {}
        Ok(Tx::Left) => return Ok(Tx::Left),
        // A failed block, or a cancel on the check, which fails the block
        // if there is one: the check cannot say whose it is.
        Ok(Tx::Aborted) | Err(Error::Cancelled) => return retry_cancelled!(own_failed(client)),
        Err(error) => return Err(error),
    }
    match still_read_only(client).await {
        Ok(true) => Ok(Tx::Open),
        Ok(false) => Ok(Tx::Left),
        // The session is inside the script's transaction (the check
        // above), and the cancel aborted it: it cannot write.
        Err(Error::Cancelled) => Ok(Tx::Aborted),
        Err(error) => Err(error),
    }
}

/// Whether a failed transaction block is the script's own (`Aborted`) or
/// not (`Left`). A failed block answers every query with 25P02, the
/// script's own and a chained one alike, except `ROLLBACK TO SAVEPOINT`,
/// which works when the block has the savepoint: only the script's does.
///
/// A cancel that lands inside the guard's swap, after its `RELEASE` and
/// before its `SAVEPOINT`, leaves the script's own transaction without the
/// savepoint. That reads as `Left` and closes the session: a safe failure
/// in a narrow window.
async fn own_failed(client: &tokio_postgres::Client) -> Result<Tx> {
    match client
        .batch_execute("ROLLBACK TO SAVEPOINT tabletist_guard")
        .await
        .map_err(query_error)
    {
        Ok(()) => Ok(Tx::Aborted),
        Err(error) if error == Error::Cancelled || error.is_connection_lost() => Err(error),
        Err(_) => Ok(Tx::Left),
    }
}

/// Where the session stands, asked before every statement and once after
/// the last. A statement the refusal missed could end the transaction
/// (`COMMIT`, `ROLLBACK`); the next one would then run on its own, where
/// the snapshot no longer keeps it read-only. So `open` sets a savepoint,
/// and this swaps it for a new one. `RELEASE` fails:
///
/// - outside a transaction block (25P01), before anything runs there;
/// - in another transaction, which does not have the savepoint (3B001):
///   `COMMIT AND CHAIN` starts one that is a block and read-only, but has
///   no snapshot yet and could be made read-write;
/// - in the script's own transaction once it failed (25P02).
///
/// Exactly one savepoint is alive at a time, however long the script. Every
/// statement runs inside its subtransaction, which PostgreSQL also refuses
/// to make read-write. The script cannot name the savepoint: the refusal
/// stops `SAVEPOINT`, `RELEASE` and `ROLLBACK`.
///
/// `Err` is a cancel that landed on the check, or a lost session.
async fn in_transaction(client: &tokio_postgres::Client) -> Result<Tx> {
    let Err(error) = client
        .batch_execute("RELEASE tabletist_guard; SAVEPOINT tabletist_guard")
        .await
    else {
        return Ok(Tx::Open);
    };
    if error.code() == Some(&SqlState::IN_FAILED_SQL_TRANSACTION) {
        return Ok(Tx::Aborted);
    }
    match query_error(error) {
        error if error == Error::Cancelled || error.is_connection_lost() => Err(error),
        // Outside the script's transaction. Any other answer is not where
        // the script started either.
        _ => Ok(Tx::Left),
    }
}

/// Ends the script's transaction. Outside one (a cancel can land on
/// `BEGIN`) the server only warns, so this is safe to run on every path.
async fn rollback(client: &tokio_postgres::Client) -> Result<()> {
    client.batch_execute("ROLLBACK").await.map_err(query_error)
}

/// Releases the session's advisory locks: a script can take one
/// (`pg_advisory_lock`), and they outlive its transaction.
async fn unlock(client: &tokio_postgres::Client) -> Result<()> {
    client
        .batch_execute("SELECT pg_advisory_unlock_all()")
        .await
        .map_err(query_error)
}

/// Whether the script's transaction is still read-only: only an explicit
/// `off` says it is not.
async fn still_read_only(client: &tokio_postgres::Client) -> Result<bool> {
    let messages = client
        .simple_query("SHOW transaction_read_only")
        .await
        .map_err(query_error)?;
    Ok(first_text(&messages).as_deref() != Some("off"))
}

/// A statement the server failed, as its outcome: the transaction is
/// aborted. `offset` is how many characters of wrapping precede the user's
/// text in what the server saw; `None` drops the position (it points into
/// other text). `Err` is a lost session.
fn failed(error: tokio_postgres::Error, offset: Option<usize>) -> Result<(StatementOutcome, Tx)> {
    use tokio_postgres::error::ErrorPosition;
    let position = error
        .as_db_error()
        .and_then(|db| match db.position()? {
            ErrorPosition::Original(position) => Some(*position as usize),
            ErrorPosition::Internal { .. } => None,
        })
        .zip(offset)
        .and_then(|(position, offset)| position.checked_sub(offset))
        .filter(|position| *position > 0);
    Ok((statement_failed(query_error(error), position)?, Tx::Aborted))
}

/// Whether a statement that returns rows can run as a cursor: its first
/// token is `SELECT`, `VALUES`, `TABLE`, `WITH` or `(`.
fn cursor_statement(text: &str) -> bool {
    let tokens = crate::sql::tokenize(Dialect::Postgres, text);
    let first = tokens.iter().find(|token| {
        !matches!(
            token.kind,
            crate::sql::TokenKind::Whitespace | crate::sql::TokenKind::Comment
        )
    });
    first.is_some_and(|token| {
        let word = text[token.range.clone()].to_ascii_uppercase();
        matches!(word.as_str(), "SELECT" | "VALUES" | "TABLE" | "WITH" | "(")
    })
}

/// Whether the statement's command tag carries a row count. tokio-postgres
/// reports 0 for a tag without one (`SET`, `DO`), which is not "0 rows".
fn counts_rows(text: &str) -> bool {
    crate::sql::words(Dialect::Postgres, text)
        .first()
        .is_some_and(|word| matches!(word.as_str(), "INSERT" | "UPDATE" | "DELETE" | "MERGE"))
}

/// How many rows to ask a cursor for: one more than the limit, to know
/// whether more exist, and never more than `FETCH` can count.
fn fetch_count(limit: usize) -> usize {
    limit.saturating_add(1).min(i32::MAX as usize)
}

/// A row statement's outcome from the rows kept: at most `limit` of them,
/// and `truncated` when one more was read.
fn rows_outcome(
    columns: Vec<ColumnMeta>,
    kept: &[tokio_postgres::SimpleQueryRow],
    limit: usize,
) -> StatementOutcome {
    let rows: Result<Vec<_>> = kept.iter().map(|row| row_values(row, &columns)).collect();
    match rows {
        Ok(mut rows) => {
            let truncated = rows.len() > limit;
            rows.truncate(limit);
            StatementOutcome::Rows {
                columns,
                rows,
                truncated,
            }
        }
        Err(error) => StatementOutcome::Error {
            error,
            position: None,
        },
    }
}

/// Runs one statement of a script inside its transaction, and says where
/// that leaves the transaction. `Err` is a lost session.
async fn run_statement(
    client: &tokio_postgres::Client,
    text: &str,
    limit: usize,
    mode: ScriptMode,
    stop: &StopFlag,
) -> Result<(StatementOutcome, Tx)> {
    // The driver cannot put a NUL in a message, and fails in a way that
    // reads as a lost session. Nothing is sent: the transaction stays open.
    if let Some(index) = text.chars().position(|character| character == '\0') {
        let error = Error::query("SQL text cannot contain a NUL character");
        return Ok((
            StatementOutcome::Error {
                error,
                position: Some(index + 1),
            },
            Tx::Open,
        ));
    }
    // Prepare first: it yields the column types and refuses a second
    // statement hidden in one piece. A prepare error is the outcome; the
    // text never runs another way. Only the columns are kept: the prepared
    // statement itself is closed here.
    let columns = match client.prepare(text).await {
        Ok(prepared) => column_metas(&prepared),
        Err(error) => return failed(error, Some(0)),
    };
    if columns.is_empty() {
        return match client.simple_query(text).await {
            Ok(messages) => {
                let affected = messages
                    .iter()
                    .find_map(|message| match message {
                        SimpleQueryMessage::CommandComplete(count) => Some(*count),
                        _ => None,
                    })
                    .filter(|_| counts_rows(text));
                Ok((
                    StatementOutcome::Done {
                        affected,
                        warnings: 0,
                    },
                    Tx::Open,
                ))
            }
            Err(error) => failed(error, Some(0)),
        };
    }
    let mut kept = Vec::new();
    // Only in a run that reads: a cursor runs its query as far as it is
    // fetched and no further, so in a run that commits, `SELECT
    // refill(id) FROM shelves` would do its work for `limit + 1` rows and
    // keep that. There every statement runs to its end, and `DECLARE`
    // would refuse a data-modifying `WITH` anyway.
    if mode == ScriptMode::ReadOnly && cursor_statement(text) {
        // Its own call, and a newline, so a trailing -- comment ends there.
        let declare = format!("{CURSOR_PREFIX}{text}\n");
        if let Err(error) = client.batch_execute(&declare).await {
            return failed(error, Some(CURSOR_PREFIX.chars().count()));
        }
        // Nothing failed: the transaction is still usable.
        if stop.is_stopped() {
            return Ok((StatementOutcome::Cancelled, Tx::Open));
        }
        let fetch = format!("FETCH {} FROM tabletist_sql", fetch_count(limit));
        let messages = match client.simple_query(&fetch).await {
            Ok(messages) => messages,
            Err(error) => return failed(error, None),
        };
        if let Err(error) = client.batch_execute("CLOSE tabletist_sql").await {
            return failed(error, None);
        }
        kept.extend(messages.into_iter().filter_map(|message| match message {
            SimpleQueryMessage::Row(row) => Some(row),
            _ => None,
        }));
    } else {
        // SHOW, EXPLAIN and the like, and every statement with rows of a
        // run that writes: stream, keep limit + 1, drop the rest.
        use futures_util::StreamExt;
        let stream = match client.simple_query_raw(text).await {
            Ok(stream) => stream,
            Err(error) => return failed(error, Some(0)),
        };
        let mut stream = std::pin::pin!(stream);
        while let Some(message) = stream.next().await {
            match message {
                Ok(SimpleQueryMessage::Row(row)) if kept.len() <= limit => kept.push(row),
                Ok(_) => {}
                Err(error) => return failed(error, Some(0)),
            }
        }
    }
    Ok((rows_outcome(columns, &kept, limit), Tx::Open))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{session, test_url};
    use super::*;
    use crate::adapter::Adapter;

    /// The backend spawns a script run, so its future must be `Send`. This
    /// fails to compile, not to run.
    #[test]
    fn a_script_run_can_be_spawned() {
        fn send<T: Send>(_: &T) {}
        let _check = |conn: &Conn, texts: Vec<String>, stop: &StopFlag| {
            send(&conn.run_script(texts, 10, ScriptMode::ReadOnly, stop));
            send(&conn.server_version());
        };
    }

    #[test]
    fn row_statements_run_as_cursors_by_their_first_token() {
        for text in [
            "SELECT 1",
            "select 1",
            "-- note\n  VALUES (1)",
            "/* c */ TABLE users",
            "WITH t AS (SELECT 1) SELECT * FROM t",
            "(SELECT 1) UNION (SELECT 2)",
        ] {
            assert!(cursor_statement(text), "{text}");
        }
        for text in ["SHOW search_path", "EXPLAIN SELECT 1", "FETCH 1 FROM c", ""] {
            assert!(!cursor_statement(text), "{text}");
        }
    }

    #[test]
    fn only_data_changing_statements_report_a_count() {
        for text in [
            "INSERT INTO t VALUES (1)",
            "update t SET n = 1",
            "-- note\nDELETE FROM t",
            "MERGE INTO t USING s ON true WHEN MATCHED THEN DO NOTHING",
        ] {
            assert!(counts_rows(text), "{text}");
        }
        for text in [
            "SET LOCAL work_mem = '8MB'",
            "DO $$ BEGIN END $$",
            "LISTEN updates",
            "",
        ] {
            assert!(!counts_rows(text), "{text}");
        }
    }

    #[test]
    fn a_cursor_is_asked_for_one_row_more_than_the_limit() {
        assert_eq!(fetch_count(0), 1);
        assert_eq!(fetch_count(1_000), 1_001);
        // FETCH counts in 32 bits.
        assert_eq!(fetch_count(i32::MAX as usize - 1), i32::MAX as usize);
        assert_eq!(fetch_count(u32::MAX as usize), i32::MAX as usize);
        assert_eq!(fetch_count(usize::MAX), i32::MAX as usize);
    }

    #[test]
    fn a_failed_lock_release_keeps_the_outcome_and_the_session() {
        assert_eq!(unlocked(Ok(())), Ok(()));
        assert_eq!(unlocked(Err(Error::query("permission denied"))), Ok(()));
        assert_eq!(unlocked(Err(Error::Cancelled)), Ok(()));
        let lost = Error::ConnectionLost("reset".into());
        assert_eq!(unlocked(Err(lost.clone())), Err(lost));
    }

    /// One test at a time uses the `probe` table: creating it twice at once
    /// can fail, and one test's TRUNCATE would hide another's stray row.
    pub(super) static PROBE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// A writable session with an empty `probe` table, and the table's
    /// lock, held until the test ends.
    pub(super) async fn probe(
        url: &str,
    ) -> (tokio_postgres::Client, tokio::sync::MutexGuard<'static, ()>) {
        let turn = PROBE.lock().await;
        let mut config: tokio_postgres::Config = url.parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        let (client, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(connection);
        client
            .batch_execute("CREATE TABLE IF NOT EXISTS probe (n int); TRUNCATE probe;")
            .await
            .unwrap();
        (client, turn)
    }

    /// The rows in the `probe` table.
    pub(super) async fn probe_rows(admin: &tokio_postgres::Client) -> i64 {
        admin
            .query_one("SELECT count(*) FROM probe", &[])
            .await
            .unwrap()
            .get(0)
    }

    /// The session's value of a setting.
    async fn setting(conn: &Conn, name: &str) -> String {
        let client = conn.client.lock().await;
        let messages = client.simple_query(&format!("SHOW {name}")).await.unwrap();
        first_text(&messages).unwrap()
    }

    /// Runs statements straight through the driver, as if the refusal had
    /// missed them.
    async fn past_the_refusal(
        conn: &Conn,
        script: &[&str],
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let texts: Vec<String> = script.iter().map(|&text| text.to_owned()).collect();
        tokio::time::timeout(
            Duration::from_secs(10),
            conn.script(&texts, 10, ScriptMode::ReadOnly, stop),
        )
        .await
        .expect("the run hung")
    }

    /// Statements the refusal stops long before they get here. Run past
    /// it, they end the script's transaction; the statement after them must
    /// not run, because it would run outside any read-only transaction, and
    /// the run must fail even when nothing follows them, so that the session
    /// is closed.
    #[tokio::test]
    async fn a_script_that_left_its_transaction_runs_nothing_more_and_fails() {
        let Some(url) = test_url() else {
            return;
        };
        let (admin, _turn) = probe(&url).await;
        for script in [
            &[
                "ROLLBACK",
                "SET default_transaction_read_only = off",
                "INSERT INTO probe VALUES (1)",
            ][..],
            &["COMMIT", "INSERT INTO probe VALUES (1)"][..],
            &["COMMIT", "BEGIN READ WRITE", "INSERT INTO probe VALUES (1)"][..],
            // Last in the script, nothing would run after them, but a
            // COMMIT keeps the settings made before it for later browsing.
            &["SET search_path = pg_catalog", "COMMIT"][..],
            &["COMMIT"][..],
            // A chained transaction is a block again, and read-only, but
            // it has no snapshot yet: it could be made read-write.
            &[
                "COMMIT AND CHAIN",
                "SET TRANSACTION READ WRITE",
                "INSERT INTO probe VALUES (1)",
                "COMMIT",
            ][..],
            &[
                "SET search_path = pg_catalog",
                "COMMIT AND CHAIN",
                "SELECT 1",
            ][..],
            &["COMMIT AND CHAIN"][..],
            &["ROLLBACK AND CHAIN", "SELECT 1"][..],
        ] {
            // A session of its own, dropped with the loop's turn: whatever
            // a failing guard let through cannot reach another test.
            let conn = session(&url).await;
            let stop = StopFlag::new();
            let ran = past_the_refusal(&conn, script, &stop).await;
            assert_eq!(ran, Err(Error::LeftReadOnly), "{script:?}");
            assert!(stop.is_finishing(), "{script:?}");
            assert_eq!(probe_rows(&admin).await, 0, "{script:?}");
            // The statement after the one that left never ran: the session
            // still defaults to read-only.
            assert_eq!(
                setting(&conn, "default_transaction_read_only").await,
                "on",
                "{script:?}"
            );
        }
    }

    /// The refusal keeps these from the driver. Past it, the server still
    /// refuses the write: the transaction has its snapshot and every
    /// statement runs under the guard's savepoint, either of which keeps it
    /// read-only, and what a statement set is rolled back.
    #[tokio::test]
    async fn statements_past_the_refusal_cannot_write() {
        let Some(url) = test_url() else {
            return;
        };
        let (admin, _turn) = probe(&url).await;
        const INSERT: &str = "INSERT INTO probe VALUES (1)";
        // The script, and the SQLSTATE its last result fails with.
        for (script, code) in [
            // The transaction cannot be made read-write any more.
            (&["SET TRANSACTION READ WRITE", INSERT][..], "25001"),
            (&["SET transaction_read_only = off", INSERT][..], "25001"),
            (
                &[
                    "SELECT set_config('transaction_read_only', 'off', true)",
                    INSERT,
                ][..],
                "25001",
            ),
            // The session default changes, but not this transaction.
            (
                &["SET \"default_transaction_read_only\" = off", INSERT][..],
                "25006",
            ),
            (
                &[
                    "SELECT set_config('default_transaction_read_only', 'off', false)",
                    INSERT,
                ][..],
                "25006",
            ),
            (&["COPY probe FROM STDIN"][..], "25006"),
            (
                &[
                    "PREPARE tabletist_probe AS INSERT INTO probe VALUES (1)",
                    "EXECUTE tabletist_probe",
                ][..],
                "25006",
            ),
        ] {
            let conn = session(&url).await;
            let stop = StopFlag::new();
            let outcome = past_the_refusal(&conn, script, &stop).await.unwrap();
            let failed = if code == "25001" { 1 } else { script.len() };
            assert_eq!(outcome.results.len(), failed, "{script:?}: {outcome:?}");
            assert!(
                matches!(
                    &outcome.results[failed - 1].outcome,
                    StatementOutcome::Error { error: Error::Query { code: Some(found), .. }, .. }
                        if found == code
                ),
                "{script:?}: {outcome:?}"
            );
            assert!(
                outcome.results[..failed - 1].iter().all(|result| matches!(
                    result.outcome,
                    StatementOutcome::Rows { .. } | StatementOutcome::Done { .. }
                )),
                "{script:?}: {outcome:?}"
            );
            assert_eq!(probe_rows(&admin).await, 0, "{script:?}");
            // The rollback took the setting back.
            assert_eq!(
                setting(&conn, "default_transaction_read_only").await,
                "on",
                "{script:?}"
            );
        }
    }

    /// A stop between statements does not fail the transaction, so the
    /// close still asks where the session stands. Here the statement before
    /// the stop ended the transaction and kept a setting.
    #[tokio::test]
    async fn a_stop_after_the_transaction_ended_still_fails_the_run() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let client = conn.client.lock().await;
        assert_eq!(open(&client).await, Ok(Tx::Open));
        // What the first two statements of the script would have done.
        client
            .batch_execute("SET search_path = pg_catalog")
            .await
            .unwrap();
        client.batch_execute("COMMIT").await.unwrap();
        // The stop lands before the third.
        let stop = StopFlag::new();
        stop.stop();
        let mut outcome = ScriptOutcome::default();
        let tx = statements(
            &client,
            &["SELECT 1".to_owned()],
            10,
            ScriptMode::ReadOnly,
            &stop,
            &mut outcome,
        )
        .await;
        assert_eq!(tx, Ok(Tx::Open));
        assert_eq!(outcome.results.len(), 1);
        assert_eq!(outcome.results[0].outcome, StatementOutcome::Cancelled);
        assert!(outcome.stopped);
        assert_eq!(close(&client, Tx::Open).await, Err(Error::LeftReadOnly));
    }

    /// A stop between statements of a script that stayed in its transaction
    /// closes cleanly, and so does a transaction that failed behind the
    /// run's back (a cancel that landed on a check). A transaction that is
    /// not the script's fails the run, failed or not.
    #[tokio::test]
    async fn the_close_asks_the_server_where_an_open_transaction_stands() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        {
            let client = conn.client.lock().await;
            assert_eq!(open(&client).await, Ok(Tx::Open));
            assert_eq!(in_transaction(&client).await, Ok(Tx::Open));
            assert_eq!(close(&client, Tx::Open).await, Ok(()));
            // Closed: no transaction is left.
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left));

            assert_eq!(open(&client).await, Ok(Tx::Open));
            assert!(client.batch_execute("SELECT 1 / 0").await.is_err());
            assert_eq!(in_transaction(&client).await, Ok(Tx::Aborted));
            assert_eq!(close(&client, Tx::Open).await, Ok(()));
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left));

            // Another transaction is not the script's, block or not.
            assert_eq!(open(&client).await, Ok(Tx::Open));
            client.batch_execute("COMMIT AND CHAIN").await.unwrap();
            assert_eq!(close(&client, Tx::Open).await, Err(Error::LeftReadOnly));
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left));

            // Nor is it once it failed (a cancel that landed on a check):
            // a failed block does not say whose it is, its savepoint does.
            assert_eq!(open(&client).await, Ok(Tx::Open));
            client.batch_execute("COMMIT AND CHAIN").await.unwrap();
            assert!(client.batch_execute("SELECT 1 / 0").await.is_err());
            assert_eq!(in_transaction(&client).await, Ok(Tx::Aborted));
            assert_eq!(close(&client, Tx::Open).await, Err(Error::LeftReadOnly));
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left));
        }
        // The session was not closed, and it works.
        let ran = past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new()).await;
        assert_eq!(ran.unwrap().results.len(), 1);
    }

    /// A session stuck in a failed transaction cannot start a script's.
    /// That is not a query error to show and repeat: the session is closed,
    /// after an attempt to end what was open.
    #[tokio::test]
    async fn a_transaction_that_cannot_start_closes_the_session() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        {
            let client = conn.client.lock().await;
            client.batch_execute("BEGIN READ ONLY").await.unwrap();
            assert!(client.batch_execute("SELECT 1 / 0").await.is_err());
        }
        let stop = StopFlag::new();
        let ran = past_the_refusal(&conn, &["SELECT 1"], &stop).await;
        assert!(
            matches!(&ran, Err(error @ Error::ConnectionLost(_)) if error.is_connection_lost()),
            "{ran:?}"
        );
        assert!(stop.is_finishing());
        // The attempt worked: the failed transaction is gone.
        let client = conn.client.lock().await;
        assert_eq!(in_transaction(&client).await, Ok(Tx::Left));
    }
}
