//! Running a SQL editor script on PostgreSQL: one transaction managed by
//! hand, the guard's savepoint around every statement, and the close that
//! asks the server where the session stands. This file is the run that
//! only reads, in a read-only transaction that is rolled back; `write` is
//! the run that commits.

use std::time::{Duration, Instant};

use bytes::BytesMut;
use tokio_postgres::SimpleQueryMessage;
use tokio_postgres::error::SqlState;
use tokio_postgres::types::{Format, FromSql, IsNull, ToSql, Type, to_sql_checked};

use super::{Conn, column_metas, first_text, query_error, row_values, unexpected};
use crate::agent::{Limits, Param, Read, Sent, count_mismatch};
use crate::script::{cannot_start, cleanup_failed, retry_cancelled, statement_failed};
use crate::{
    ColumnMeta, Dialect, Error, Result, ScriptMode, ScriptOutcome, StatementOutcome,
    StatementResult, StopFlag, Value,
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
        let (outcome, _) = read_only(&client, texts, limit as usize, stop, None, None).await?;
        Ok(outcome)
    }

    /// See [`crate::Connection::run_agent_read`]: the run of a script of
    /// one statement that only reads, with the statement's values bound
    /// and the server's own timeout set for the transaction, with which of
    /// its result columns are a masked column's. Awaited to its end, as
    /// [`Self::script`] is.
    pub(super) async fn agent_read(
        &self,
        read: &Read,
        limits: Limits,
        sources: &crate::agent::Sources,
        stop: &StopFlag,
    ) -> Result<(ScriptOutcome, Vec<bool>)> {
        let client = self.client.lock().await;
        let texts = [read.text().to_owned()];
        let sent = Sent {
            params: read.params(),
            timeout: limits.timeout,
        };
        let hiding = sources
            .guards_anything()
            .then_some(Hiding { read, sources });
        read_only(
            &client,
            &texts,
            limits.rows as usize,
            stop,
            Some(sent),
            hiding,
        )
        .await
    }
}

/// An agent's statement, with what its connection hides and masks.
#[derive(Clone, Copy)]
struct Hiding<'a> {
    read: &'a Read,
    sources: &'a crate::agent::Sources,
}

/// The run that only reads: its transaction, its statements, and the
/// close. `sent` is there when the run is of an agent's one statement, and
/// `hiding` when its connection hides tables or masks columns: the
/// statement is checked against them in the transaction, and runs only if
/// it reads nothing it may not. With the outcome, which of the statement's
/// result columns are a masked column's (none where nothing is masked).
async fn read_only(
    client: &tokio_postgres::Client,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
    sent: Option<Sent<'_>>,
    hiding: Option<Hiding<'_>>,
) -> Result<(ScriptOutcome, Vec<bool>)> {
    let mut outcome = ScriptOutcome::default();
    let mut masked = Vec::new();
    let mode = ScriptMode::ReadOnly;
    // The SQL editor's run rides its opening and its close on its row
    // statements' flights (see `cursor_flight`). An agent's run does not:
    // its timeout and its check go between the opening and the statement.
    let editor = sent.is_none() && hiding.is_none();
    let flights = Flights {
        opens: editor && texts.first().is_some_and(|text| flies(text, mode, None)),
        closes: editor,
    };
    let opened = if flights.opens {
        Ok(Tx::Open)
    } else {
        open(client, sent).await
    };
    let stand = match opened {
        Ok(Tx::Open) => match refused_for_hiding(client, hiding).await {
            Ok(Err((error, tx))) => {
                outcome.stopped = error == Error::Cancelled;
                outcome.results.push(StatementResult {
                    elapsed: Duration::ZERO,
                    outcome: statement_failed(error, None)?,
                });
                Stand::Tx(tx)
            }
            Ok(Ok(columns)) => {
                masked = columns;
                let params = sent.map(|sent| sent.params);
                statements(
                    client,
                    texts,
                    limit,
                    mode,
                    stop,
                    params,
                    flights,
                    &mut outcome,
                )
                .await?
            }
            Err(error) => return Err(error),
        },
        Ok(_) => Stand::Unopened(Error::Cancelled),
        Err(error) => Stand::Unopened(error),
    };
    let tx = match stand {
        Stand::Tx(tx) => tx,
        // A lost session ends the run here: nothing to close.
        Stand::Unopened(error) if error.is_connection_lost() => return Err(error),
        // A cancel landed on the opening queries: no results.
        Stand::Unopened(Error::Cancelled) => {
            outcome.stopped = true;
            Tx::Aborted
        }
        // The transaction could not start, say because the session is
        // stuck in a failed one. The next run would fail the same way,
        // so the session is closed, after an attempt to end what is
        // open.
        Stand::Unopened(error) => {
            stop.finish();
            close(client, Tx::Aborted).await.ok();
            return Err(cannot_start(mode, &error));
        }
        // The last statement's flight carried the close: what it did not
        // get to runs now. Its answers are read once the backend stopped
        // repeating its cancel, since reading them may ask the server
        // again (see `confirmed`).
        Stand::Closing(closing) => {
            stop.finish();
            let ended = confirmed(client, &closing.messages, closing.failure).await;
            end(client, ended).await?;
            return Ok((outcome, masked));
        }
    };
    // From here on a cancel would land on the cleanup: tell the
    // backend to stop repeating its cancel.
    stop.finish();
    close(client, tx).await?;
    Ok((outcome, masked))
}

/// What the SQL editor's run that reads sends along with its row
/// statements (see `cursor_flight`).
#[derive(Debug, Clone, Copy, Default)]
struct Flights {
    /// The opening goes out with the first statement, which is a row
    /// statement.
    opens: bool,
    /// The close goes out with the last statement, where that is a row
    /// statement.
    closes: bool,
}

/// Where a script's statements left the session.
enum Stand {
    /// In the transaction or out of it, for the close to end.
    Tx(Tx),
    /// The last statement's flight carried the close: what the server
    /// answered to it.
    Closing(Closing),
    /// The opening that went out with the first statement failed, and
    /// nothing of the statement ran.
    Unopened(Error),
}

/// Why an agent's statement may not run on a connection that hides tables
/// or masks columns (`super::hidden`), and where that leaves the
/// transaction: open after a refusal, failed after the server's own error.
/// `Ok` where it may run, with which of its result columns are a masked
/// column's (none where nothing is hidden or masked). `Err` is a lost
/// session.
async fn refused_for_hiding(
    client: &tokio_postgres::Client,
    hiding: Option<Hiding<'_>>,
) -> Result<std::result::Result<Vec<bool>, (Error, Tx)>> {
    let Some(hiding) = hiding else {
        return Ok(Ok(Vec::new()));
    };
    match super::hidden::check(client, hiding.read, hiding.sources).await {
        Ok(Ok(masked)) => Ok(Ok(masked)),
        Ok(Err(refusal @ Error::Agent(_))) => Ok(Err((refusal, Tx::Open))),
        Ok(Err(error)) => Ok(Err((error, Tx::Aborted))),
        Err(Error::Cancelled) => Ok(Err((Error::Cancelled, Tx::Aborted))),
        Err(error) if error.is_connection_lost() => Err(error),
        Err(error) => Ok(Err((error, Tx::Aborted))),
    }
}

/// An agent's value as PostgreSQL is sent it: in text, which the server
/// reads as the type it gave the placeholder, the way it reads a quoted
/// literal. So one kind of value fills a placeholder of any type, and a
/// value its type does not take is the server's error. A value is never
/// written into the statement.
impl ToSql for Param {
    fn to_sql(
        &self,
        _: &Type,
        out: &mut BytesMut,
    ) -> std::result::Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        let text = match self {
            Self::Null => return Ok(IsNull::Yes),
            Self::Bool(true) => "true".to_owned(),
            Self::Bool(false) => "false".to_owned(),
            Self::Int(number) => number.to_string(),
            Self::Float(number) => number.to_string(),
            Self::Text(text) => text.clone(),
        };
        out.extend_from_slice(text.as_bytes());
        Ok(IsNull::No)
    }

    fn accepts(_: &Type) -> bool {
        true
    }

    fn encode_format(&self, _: &Type) -> Format {
        Format::Text
    }

    to_sql_checked!();
}

/// A value of a plan, which the server sends as it is written whatever
/// the protocol: `EXPLAIN` answers in text, JSON or XML.
pub(super) struct Plan(pub(super) String);

impl<'a> FromSql<'a> for Plan {
    fn from_sql(
        _: &Type,
        raw: &'a [u8],
    ) -> std::result::Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Ok(Self(std::str::from_utf8(raw)?.to_owned()))
    }

    fn accepts(kind: &Type) -> bool {
        *kind == Type::TEXT || *kind == Type::JSON || *kind == Type::XML
    }
}

/// `params` as the driver takes a statement's values.
fn values(params: &[Param]) -> Vec<&(dyn ToSql + Sync)> {
    params
        .iter()
        .map(|param| param as &(dyn ToSql + Sync))
        .collect()
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
///
/// An agent's run also sets the server's own statement timeout, for the
/// transaction: the rollback that ends the run takes it away. Before the
/// savepoint, so that it holds whatever the guard does with that. The
/// statement cannot lift it: the refusal stops every `SET`, and the check
/// `set_config`. No lock timeout beside it: a wait for a lock is part of
/// the statement's time.
///
/// A row statement runs as a cursor (see `run_statement`), which
/// PostgreSQL plans for the first tenth of its rows by default
/// (`cursor_tuple_fraction`): another plan than the same query gets in
/// psql, and at times a slower one. The transaction plans for all of them,
/// as a plain query is; the rollback takes the setting away.
///
/// All of it is one message, one round trip to the server. It runs in
/// order and stops at the first step that fails; past `BEGIN` that leaves
/// a failed transaction, which the close rolls back as before. The SQL
/// editor's run sends it in the same flight as its first statement, when
/// that is a row statement (see `cursor_flight`).
async fn open(client: &tokio_postgres::Client, sent: Option<Sent<'_>>) -> Result<Tx> {
    match client
        .batch_execute(&opening(sent))
        .await
        .map_err(query_error)
    {
        Ok(()) => Ok(Tx::Open),
        Err(Error::Cancelled) => Ok(Tx::Aborted),
        Err(error) => Err(error),
    }
}

/// The message [`open`] sends.
fn opening(sent: Option<Sent<'_>>) -> String {
    let timeout = sent.map(|sent| format!("SET LOCAL statement_timeout = {}; ", sent.timeout_ms()));
    format!(
        "BEGIN READ ONLY; SET LOCAL cursor_tuple_fraction = 1; SELECT 1; {}SAVEPOINT tabletist_guard",
        timeout.unwrap_or_default()
    )
}

/// Runs the statements in order, up to the first that fails or is
/// stopped, and says where that leaves the transaction. `Err` is a lost
/// session. `params` are an agent's values for its one statement: a script
/// has none.
///
/// The caller has just set the guard's savepoint, and nothing of the
/// script has run since, so the first statement goes without the guard's
/// swap: it would only find what `open` left. What may run in between, an
/// agent's check against what its connection hides, is the run's own
/// queries, which can fail the transaction but not end it, and a failure
/// there runs no statement. With `flights.opens` the opening has not been
/// sent yet: it goes out with the first statement.
///
/// A statement after a row statement of the SQL editor goes without the
/// separate swap too: that statement's flight made the swap's check before
/// its cursor, and a row statement cannot end the transaction. Its own
/// flight makes the check again.
#[allow(clippy::too_many_arguments)] // two call sites: the run that reads and the run that writes
async fn statements(
    client: &tokio_postgres::Client,
    texts: &[String],
    limit: usize,
    mode: ScriptMode,
    stop: &StopFlag,
    params: Option<&[Param]>,
    flights: Flights,
    outcome: &mut ScriptOutcome,
) -> Result<Stand> {
    // Whether the session is known to be in the script's own transaction,
    // and it did not fail.
    let mut known = true;
    for (index, text) in texts.iter().enumerate() {
        let opens = flights.opens && index == 0;
        let ready = if stop.is_stopped() {
            if opens {
                // Nothing was sent, not even the opening: no transaction
                // to ask about, and the rollback finds nothing to do.
                outcome.stopped = true;
                outcome.results.push(StatementResult {
                    elapsed: Duration::ZERO,
                    outcome: StatementOutcome::Cancelled,
                });
                return Ok(Stand::Tx(Tx::Aborted));
            }
            Err(Error::Cancelled)
        } else if known {
            Ok(Tx::Open)
        } else {
            in_transaction(client).await
        };
        let failure = match ready {
            Ok(Tx::Open) => None,
            Ok(Tx::Left) => return Ok(Stand::Tx(Tx::Left)),
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
            return Ok(Stand::Tx(tx));
        }
        let started = Instant::now();
        let flight = Flights {
            opens,
            closes: flights.closes && index + 1 == texts.len(),
        };
        let flown = flies(text, mode, params);
        let (result, tx, closing) = if flown {
            match cursor_flight(client, text, limit, flight).await? {
                Flown::Unopened(error) => return Ok(Stand::Unopened(error)),
                Flown::Ran(result, tx, closing) => (result, tx, closing),
            }
        } else {
            let (result, tx) = run_statement(client, text, limit, mode, stop, params).await?;
            (result, tx, None)
        };
        known = flown;
        outcome.stopped |= result == StatementOutcome::Cancelled;
        let last = !matches!(
            result,
            StatementOutcome::Rows { .. } | StatementOutcome::Done { .. }
        );
        outcome.results.push(StatementResult {
            elapsed: started.elapsed(),
            outcome: result,
        });
        if let Some(closing) = closing {
            return Ok(Stand::Closing(closing));
        }
        if last {
            return Ok(Stand::Tx(tx));
        }
    }
    Ok(Stand::Tx(Tx::Open))
}

/// Ends the script's transaction, whatever state it is in: confirms an
/// open one is the script's own and still read-only, rolls back, and
/// releases the session's advisory locks. `LeftReadOnly`, a failed check
/// or a failed rollback closes the session (they count as a lost
/// connection).
async fn close(client: &tokio_postgres::Client, tx: Tx) -> Result<()> {
    let ended = match tx {
        Tx::Open => confirm(client).await,
        known => Ended {
            tx: Ok(known),
            rolled_back: None,
            released: None,
        },
    };
    end(client, ended).await
}

/// The rest of [`close`] once it is known how far the close got: the
/// rollback and the release where they did not run, or a cancel landed on
/// them, and the run's answer.
async fn end(client: &tokio_postgres::Client, ended: Ended) -> Result<()> {
    // Whatever is open is rolled back as far as that works, also when the
    // session is closed anyway. A step the close's message did not reach
    // runs on its own; one a cancel landed on runs once more.
    let rolled_back = match ended.rolled_back {
        Some(Err(Error::Cancelled)) => rollback(client).await,
        Some(answer) => answer,
        None => retry_cancelled!(rollback(client)),
    };
    match (ended.tx, rolled_back) {
        (Ok(Tx::Left), _) => Err(Error::LeftReadOnly),
        (Err(error), _) | (_, Err(error)) => Err(cleanup_failed(ScriptMode::ReadOnly, &error)),
        (Ok(_), Ok(())) => unlocked(match ended.released {
            Some(Err(Error::Cancelled)) => unlock(client).await,
            Some(answer) => answer,
            None => retry_cancelled!(unlock(client)),
        }),
    }
}

/// How far the close got: where the transaction stood, and the answers to
/// the rollback and the release of the advisory locks where they ran
/// (`None` where they did not).
struct Ended {
    tx: Result<Tx>,
    rolled_back: Option<Result<()>>,
    released: Option<Result<()>>,
}

/// The close of a transaction the run believes open, as one message: the
/// guard's check, the question, the rollback and the release (see
/// `confirm`). Each step is one answer the server completes, in this order.
/// The SQL editor's run sends it at the end of its last row statement's
/// cursor message (see `cursor_flight`).
const CONFIRM_AND_END: &str = "RELEASE tabletist_guard; SAVEPOINT tabletist_guard; \
     SHOW transaction_read_only; ROLLBACK; SELECT pg_advisory_unlock_all()";

/// Where in [`CONFIRM_AND_END`] the rollback is: the steps before it are
/// the check and the question.
const ROLLBACK_STEP: usize = 3;

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
///
/// The guard's check (see `in_transaction`), the question whether the
/// transaction is still read-only, the rollback and the release of the
/// advisory locks go in one message, one round trip to the server. The
/// rollback runs whatever the answer is, as it does after the steps one by
/// one. A failed step stops the rest, so the question is only answered
/// when the check passed, and the steps the message did not reach are
/// left to [`close`], which runs them on their own.
async fn confirm(client: &tokio_postgres::Client) -> Ended {
    let (messages, failure) = answers(client, CONFIRM_AND_END).await;
    confirmed(client, &messages, failure).await
}

/// How far [`CONFIRM_AND_END`] got from what the server answered to it,
/// on its own or behind a cursor (see `cursor_flight`): its `messages`, up
/// to the step that failed, and that step's error. Where the check or the
/// question failed it asks the server whose failed block it is, so it runs
/// once the run is finishing: a repeated cancel must not land on that.
async fn confirmed(
    client: &tokio_postgres::Client,
    messages: &[SimpleQueryMessage],
    failure: Option<tokio_postgres::Error>,
) -> Ended {
    let completed = completed(messages);
    if completed < ROLLBACK_STEP {
        // The check or the question failed. Without a failure the server
        // answered something other than the five steps.
        let error = match failure {
            Some(error) => standing(error),
            None => Err(Error::query(
                "the server did not answer the close step by step",
            )),
        };
        let tx = match error {
            Ok(Tx::Left) => Ok(Tx::Left),
            // A failed block, or a cancel, which fails the block if there
            // is one, whether it landed on the check or on the question:
            // neither says whose block it is.
            Ok(_) | Err(Error::Cancelled) => retry_cancelled!(own_failed(client)),
            Err(error) => Err(error),
        };
        return Ended {
            tx,
            rolled_back: None,
            released: None,
        };
    }
    // Only an explicit `off` says it is no longer read-only.
    let tx = match first_text(messages).as_deref() {
        Some("off") => Ok(Tx::Left),
        _ => Ok(Tx::Open),
    };
    // Past the question only the rollback or the release can fail.
    let (rolled_back, released) = match failure.map(query_error) {
        None => (Some(Ok(())), Some(Ok(()))),
        Some(error) if completed == ROLLBACK_STEP => (Some(Err(error)), None),
        Some(error) => (Some(Ok(())), Some(Err(error))),
    };
    Ended {
        tx,
        rolled_back,
        released,
    }
}

/// How many steps of a message of several the server completed.
fn completed(messages: &[SimpleQueryMessage]) -> usize {
    messages
        .iter()
        .filter(|message| matches!(message, SimpleQueryMessage::CommandComplete(_)))
        .count()
}

/// What the server answered to `sql`, a message of several steps, up to the
/// step that failed, and that step's error.
async fn answers(
    client: &tokio_postgres::Client,
    sql: &str,
) -> (Vec<SimpleQueryMessage>, Option<tokio_postgres::Error>) {
    use futures_util::StreamExt;
    let stream = match client.simple_query_raw(sql).await {
        Ok(stream) => stream,
        Err(error) => return (Vec::new(), Some(error)),
    };
    let mut stream = std::pin::pin!(stream);
    let mut messages = Vec::new();
    while let Some(message) = stream.next().await {
        match message {
            Ok(message) => messages.push(message),
            Err(error) => return (messages, Some(error)),
        }
    }
    (messages, None)
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
    match client.batch_execute(GUARD).await {
        Ok(()) => Ok(Tx::Open),
        Err(error) => standing(error),
    }
}

/// The guard's swap (see [`in_transaction`]): two steps.
const GUARD: &str = "RELEASE tabletist_guard; SAVEPOINT tabletist_guard";

/// How many steps [`GUARD`] is.
const GUARD_STEPS: usize = 2;

/// Where the session stands after the guard's swap failed with `error`:
/// `Aborted` in the script's own failed transaction, `Left` anywhere else.
/// `Err` is a cancel, or a lost session.
fn standing(error: tokio_postgres::Error) -> Result<Tx> {
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

/// [`failed`] for the message that declares, fetches and closes the
/// cursor: the user's text follows `prefix`, and a position past it points
/// into the `FETCH` or `CLOSE` after it, so it is dropped.
fn failed_in_cursor(
    error: tokio_postgres::Error,
    prefix: &str,
    text: &str,
) -> Result<(StatementOutcome, Tx)> {
    let (outcome, tx) = failed(error, Some(prefix.chars().count()))?;
    Ok((within_text(outcome, text), tx))
}

/// `outcome` without an error position that falls past `text`.
fn within_text(outcome: StatementOutcome, text: &str) -> StatementOutcome {
    match outcome {
        StatementOutcome::Error { error, position } => StatementOutcome::Error {
            error,
            position: position.filter(|position| *position <= text.chars().count()),
        },
        outcome => outcome,
    }
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

/// What a row statement's cursor runs: its `DECLARE`, and the `FETCH` of
/// one row more than `limit` with the `CLOSE`.
fn cursor_steps(text: &str, limit: usize) -> (String, String) {
    // A newline, so a trailing -- comment ends there.
    let declare = format!("{CURSOR_PREFIX}{text}\n");
    let fetch = format!(
        "FETCH {} FROM tabletist_sql; CLOSE tabletist_sql",
        fetch_count(limit)
    );
    (declare, fetch)
}

/// What comes before the user's text in a flight's cursor message: the
/// guard's swap and the cursor's prefix.
const FLIGHT_PREFIX: &str = "RELEASE tabletist_guard; SAVEPOINT tabletist_guard; \
     DECLARE tabletist_sql NO SCROLL CURSOR FOR ";

/// How many steps a flight's cursor message is up to its close: the
/// guard's swap, `DECLARE`, `FETCH` and `CLOSE`.
const CURSOR_STEPS: usize = GUARD_STEPS + 3;

/// The cursor message of a row statement's flight (see `cursor_flight`):
/// the guard's swap, the cursor's steps, and with `closes` the close's.
fn cursor_message(text: &str, limit: usize, closes: bool) -> String {
    let (declare, fetch) = cursor_steps(text, limit);
    let close = if closes {
        format!("; {CONFIRM_AND_END}")
    } else {
        String::new()
    };
    format!("{GUARD}; {declare};{fetch}{close}")
}

/// Whether a statement runs as a flight (see `cursor_flight`): the SQL
/// editor's row statement in a run that reads, which the driver can send.
fn flies(text: &str, mode: ScriptMode, params: Option<&[Param]>) -> bool {
    params.is_none()
        && mode == ScriptMode::ReadOnly
        && !text.contains('\0')
        && cursor_statement(text)
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

/// A plan's rows from the extended protocol: at most `limit` of them, and
/// `truncated` when there were more.
fn plan_outcome(
    columns: Vec<ColumnMeta>,
    answered: &[tokio_postgres::Row],
    limit: usize,
) -> StatementOutcome {
    let line = |row: &tokio_postgres::Row| -> Result<Vec<Value>> {
        (0..row.len())
            .map(|index| {
                let value: Option<Plan> = row.try_get(index).map_err(unexpected)?;
                Ok(value.map_or(Value::Null, |Plan(text)| Value::Text(text.into())))
            })
            .collect()
    };
    match answered.iter().take(limit).map(line).collect() {
        Ok(rows) => StatementOutcome::Rows {
            columns,
            rows,
            truncated: answered.len() > limit,
        },
        Err(error) => StatementOutcome::Error {
            error,
            position: None,
        },
    }
}

/// Runs one statement of a script inside its transaction, and says where
/// that leaves the transaction. `Err` is a lost session.
///
/// `params` are an agent's values. The SQL editor has none, and its
/// statement with a placeholder is the server's error, as before.
async fn run_statement(
    client: &tokio_postgres::Client,
    text: &str,
    limit: usize,
    mode: ScriptMode,
    stop: &StopFlag,
    params: Option<&[Param]>,
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
    // The SQL editor's row statement in a run that reads: its prepare goes
    // out with its cursor.
    if flies(text, mode, params) {
        return match cursor_flight(client, text, limit, Flights::default()).await? {
            Flown::Ran(outcome, tx, _) => Ok((outcome, tx)),
            // Only a flight with the opening says this.
            Flown::Unopened(error) => Ok((statement_failed(error, None)?, Tx::Aborted)),
        };
    }
    // Every other statement is prepared first: that yields the column
    // types and refuses a second statement hidden in one piece. A prepare
    // error is the outcome; the text never runs another way. Only the
    // columns are kept: the prepared statement itself is closed here.
    //
    // Not in the same flight as the statement, as the cursor's is: the
    // statement is sent as it is, and a text that starts with one a failed
    // transaction still takes (`ROLLBACK`) would run past a failed
    // prepare. And an agent's values are counted against the server's
    // count before anything runs.
    let prepared = match client.prepare(text).await {
        Ok(prepared) => prepared,
        Err(error) => return failed(error, Some(0)),
    };
    let columns = column_metas(&prepared);
    if let Some(params) = params {
        // The server's own count, which is the one that holds. Nothing
        // ran: the transaction stays open.
        let counted = prepared.params().len();
        if counted != params.len() {
            let error = count_mismatch(counted, params.len());
            let position = None;
            return Ok((StatementOutcome::Error { error, position }, Tx::Open));
        }
    }
    // Without values a statement runs as it always did.
    let params = params.unwrap_or_default();
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
        let (declare, fetch) = cursor_steps(text, limit);
        let messages = if params.is_empty() {
            // An agent's statement without values: the three in one
            // message, one round trip to the server. The text cannot reach
            // into what follows it: it prepared above, so it is one
            // statement with no quote or comment left open but a -- one.
            // The server stops at the first that fails.
            match client.simple_query(&format!("{declare};{fetch}")).await {
                Ok(messages) => messages,
                Err(error) => return failed_in_cursor(error, CURSOR_PREFIX, text),
            }
        } else {
            // Values are bound where the cursor is declared, which takes
            // the other protocol: the cursor keeps them, and its rows are
            // fetched as text like any other's.
            let declared = client.execute(declare.as_str(), &values(params)).await;
            if let Err(error) = declared {
                return failed(error, Some(CURSOR_PREFIX.chars().count()));
            }
            // Nothing failed: the transaction is still usable.
            if stop.is_stopped() {
                return Ok((StatementOutcome::Cancelled, Tx::Open));
            }
            match client.simple_query(&fetch).await {
                Ok(messages) => messages,
                Err(error) => return failed(error, None),
            }
        };
        kept.extend(messages.into_iter().filter_map(|message| match message {
            SimpleQueryMessage::Row(row) => Some(row),
            _ => None,
        }));
    } else if !params.is_empty() {
        // An `EXPLAIN` with values, the one statement of an agent's that
        // is no cursor. Its rows come by the protocol that binds them.
        return match client.query(text, &values(params)).await {
            Ok(rows) => Ok((plan_outcome(columns, &rows, limit), Tx::Open)),
            Err(error) => failed(error, Some(0)),
        };
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

/// What came back from a row statement's flight (see [`cursor_flight`]).
enum Flown {
    /// The opening that went out with it failed: nothing of the statement
    /// ran.
    Unopened(Error),
    /// The statement's outcome and where it left the transaction, with
    /// what the server answered to the close where it went along and the
    /// cursor got that far.
    Ran(StatementOutcome, Tx, Option<Closing>),
}

/// What the server answered to the close's steps at the end of a cursor's
/// message: their answers, up to the step that failed, and that step's
/// error. [`confirmed`] reads them once the run is finishing.
struct Closing {
    messages: Vec<SimpleQueryMessage>,
    failure: Option<tokio_postgres::Error>,
}

/// A row statement of the SQL editor's run that reads, in one round trip:
/// the prepare and the cursor's message (the guard's swap, the cursor's
/// `DECLARE`, `FETCH` and `CLOSE`) go out together, and the client waits on
/// both at once. With `flight.opens` the opening (see [`open`]) goes out in
/// front of them, and with `flight.closes` the close's steps (see
/// [`confirm`]) go at the end of the cursor's message. Each goes out when
/// it is first polled, and the first poll reaches all of them, in this
/// order.
///
/// The server still takes them one after another, and the prepare still
/// decides whether the cursor runs. A prepare that fails (a text of two
/// statements, a quote or a comment left open, a cancel that landed on it)
/// fails the transaction the run is inside, and a failed transaction
/// refuses the guard's `RELEASE` the message starts with, whatever the text
/// is: nothing of the message runs. A prepare that works took the text as
/// one statement, with no quote or comment left open but a -- one, so the
/// text cannot reach into the `FETCH`, the `CLOSE` and the close's steps
/// after it. The prepare also types the columns: a simple query's rows
/// carry no types.
///
/// The guard's swap keeps the cursor inside the script's transaction when
/// the opening went along and failed. Past `BEGIN` that leaves a failed
/// transaction, which refuses everything after it. A cancel that lands on
/// `BEGIN` itself leaves no transaction at all, and a message of several
/// statements outside one runs them in a transaction of its own, which is
/// not read-only and commits: there the `RELEASE` fails, and the cursor
/// never runs.
///
/// The close needs no check of its own before it: a row statement cannot
/// end the transaction, so the session stands where the guard found it,
/// and the close's own steps still ask the server, after the cursor. A
/// cursor that fails stops the message before the close's steps, and the
/// close runs on its own, as after any other failed statement.
///
/// A column of a type the driver does not know yet is looked up after the
/// prepare answered, behind the cursor's message. Where the cursor failed,
/// that lookup fails too, on the failed transaction: the cursor's error is
/// the one that says why. Where the close went along, the lookup runs after
/// it, outside the transaction.
async fn cursor_flight(
    client: &tokio_postgres::Client,
    text: &str,
    limit: usize,
    flight: Flights,
) -> Result<Flown> {
    let opening = flight.opens.then(|| opening(None));
    let message = cursor_message(text, limit, flight.closes);
    let open = async {
        match &opening {
            Some(steps) => Some(client.batch_execute(steps).await.map_err(query_error)),
            None => None,
        }
    };
    let (opened, prepared, (mut messages, mut failure)) =
        tokio::join!(open, client.prepare(text), answers(client, &message));
    let steps = completed(&messages);
    if let Some(Err(error)) = opened {
        if steps < GUARD_STEPS {
            return Ok(Flown::Unopened(error));
        }
        // The guard let the cursor through all the same: the session is
        // not where the script started.
        let error = Error::query("the transaction did not start before this statement ran");
        let outcome = StatementOutcome::Error {
            error,
            position: None,
        };
        return Ok(Flown::Ran(outcome, Tx::Left, None));
    }
    // The cursor's answers, and the close's after them.
    let split = messages
        .iter()
        .enumerate()
        .filter(|(_, message)| matches!(message, SimpleQueryMessage::CommandComplete(_)))
        .nth(CURSOR_STEPS - 1)
        .map_or(messages.len(), |(index, _)| index + 1);
    let closing = messages.split_off(split);
    let close_failure = if steps >= CURSOR_STEPS {
        failure.take()
    } else {
        None
    };
    let (outcome, tx) = cursor_outcome(prepared, steps, failure, messages, text, limit)?;
    let closing = (flight.closes && steps >= CURSOR_STEPS).then(|| Closing {
        messages: closing,
        failure: close_failure,
    });
    Ok(Flown::Ran(outcome, tx, closing))
}

/// A flight's statement from what came back for it: the prepare, how many
/// steps of the cursor's message the server `completed`, the step that
/// failed, and the cursor's answers.
fn cursor_outcome(
    prepared: std::result::Result<tokio_postgres::Statement, tokio_postgres::Error>,
    completed: usize,
    failure: Option<tokio_postgres::Error>,
    answered: Vec<SimpleQueryMessage>,
    text: &str,
    limit: usize,
) -> Result<(StatementOutcome, Tx)> {
    if completed < CURSOR_STEPS {
        let Some(error) = failure else {
            let error = Error::query("the server did not answer the cursor step by step");
            let position = None;
            // Where that leaves the transaction, the close asks.
            return Ok((StatementOutcome::Error { error, position }, Tx::Open));
        };
        if completed < GUARD_STEPS {
            return guarded(prepared, error);
        }
        return match prepared {
            // The prepare's own failure, which kept the cursor from running.
            Err(own) if own.code() != Some(&SqlState::IN_FAILED_SQL_TRANSACTION) => {
                failed(own, Some(0))
            }
            _ => failed_in_cursor(error, FLIGHT_PREFIX, text),
        };
    }
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(error) => return failed(error, Some(0)),
    };
    let columns = column_metas(&prepared);
    // Its close goes out on its own, waited on by nobody.
    drop(prepared);
    // A statement without columns (`SELECT FROM shelves`) is done, as the
    // statements are that do not run as a cursor.
    if columns.is_empty() {
        let affected = None;
        return Ok((
            StatementOutcome::Done {
                affected,
                warnings: 0,
            },
            Tx::Open,
        ));
    }
    let kept: Vec<_> = answered
        .into_iter()
        .filter_map(|message| match message {
            SimpleQueryMessage::Row(row) => Some(row),
            _ => None,
        })
        .collect();
    Ok((rows_outcome(columns, &kept, limit), Tx::Open))
}

/// A flight's statement whose guard's swap failed with `error`: nothing of
/// the cursor ran.
///
/// Where the prepare failed on its own, its error is the statement's: it
/// failed the transaction, which refused the swap, or the text left a
/// quote or a comment open, and the server refused the whole message
/// before any of it ran. A flight goes only where the session is known to
/// be in the script's transaction (see `statements`), or with the opening,
/// whose own failure comes first.
fn guarded(
    prepared: std::result::Result<tokio_postgres::Statement, tokio_postgres::Error>,
    error: tokio_postgres::Error,
) -> Result<(StatementOutcome, Tx)> {
    if let Err(own) = prepared
        && own.code() != Some(&SqlState::IN_FAILED_SQL_TRANSACTION)
    {
        return failed(own, Some(0));
    }
    let refused = |message| StatementOutcome::Error {
        error: Error::query(message),
        position: None,
    };
    match standing(error) {
        // Whose failed transaction it is, the close asks.
        Ok(Tx::Aborted) => Ok((
            refused("the transaction failed before this statement ran"),
            Tx::Open,
        )),
        Ok(_) => Ok((
            refused("the transaction ended before this statement ran"),
            Tx::Left,
        )),
        // A cancel that landed on the swap. It fails the transaction block
        // the session is in; whether that is still the script's, the close
        // asks.
        Err(Error::Cancelled) => Ok((StatementOutcome::Cancelled, Tx::Open)),
        Err(error) => Err(error),
    }
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
        let _agent = |conn: &Conn, read: &Read, limits: Limits, stop: &StopFlag| {
            send(&conn.run_read(read, limits, &crate::agent::Sources::default(), stop));
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
    fn a_cursor_error_keeps_a_position_only_inside_the_users_text() {
        let error = |position| StatementOutcome::Error {
            error: Error::query("boom"),
            position,
        };
        let text = "SELECT é";
        // The last character of the text, counted in characters.
        assert_eq!(within_text(error(Some(8)), text), error(Some(8)));
        // Past it: the FETCH or CLOSE that follow.
        assert_eq!(within_text(error(Some(9)), text), error(None));
        assert_eq!(within_text(error(None), text), error(None));
        let other = StatementOutcome::Cancelled;
        assert_eq!(within_text(other.clone(), text), other);
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

    #[test]
    fn an_agents_value_is_sent_as_text_for_a_placeholder_of_any_type() {
        let sent = |param: Param| {
            let mut out = BytesMut::new();
            let null = matches!(param.to_sql(&Type::INT4, &mut out).unwrap(), IsNull::Yes);
            (null, String::from_utf8(out.to_vec()).unwrap())
        };
        let text = |text: &str| (false, text.to_owned());
        assert_eq!(sent(Param::Null), (true, String::new()));
        assert_eq!(sent(Param::Bool(true)), text("true"));
        assert_eq!(sent(Param::Bool(false)), text("false"));
        assert_eq!(sent(Param::Int(i64::MIN)), text("-9223372036854775808"));
        assert_eq!(sent(Param::Float(1.5)), text("1.5"));
        // No exponent, which is one spelling fewer for the server to read.
        assert_eq!(sent(Param::Float(1e21)), text("1000000000000000000000"));
        // As it is: nothing is quoted, because nothing is written into SQL.
        assert_eq!(sent(Param::Text("it's".into())), text("it's"));
        for kind in [Type::INT4, Type::TEXT, Type::TIMESTAMPTZ, Type::INT4_ARRAY] {
            assert!(<Param as ToSql>::accepts(&kind), "{kind}");
            let format = Param::Int(1).encode_format(&kind);
            assert!(matches!(format, Format::Text), "{kind}");
        }
    }

    #[test]
    fn a_plan_is_read_from_the_types_explain_answers_in() {
        for kind in [Type::TEXT, Type::JSON, Type::XML] {
            assert!(<Plan as FromSql>::accepts(&kind), "{kind}");
        }
        // Not from a type whose binary form is not its text.
        for kind in [Type::INT4, Type::JSONB, Type::BYTEA] {
            assert!(!<Plan as FromSql>::accepts(&kind), "{kind}");
        }
        let Plan(line) = Plan::from_sql(&Type::TEXT, b"Seq Scan on books").unwrap();
        assert_eq!(line, "Seq Scan on books");
        assert!(Plan::from_sql(&Type::TEXT, &[0xC3, 0x28]).is_err());
    }

    #[tokio::test]
    async fn the_servers_own_count_of_placeholders_is_the_one_that_holds() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let other = session(&url).await;
        let client = conn.client.lock().await;
        let stop = StopFlag::new();
        let mode = ScriptMode::ReadOnly;
        for (text, sent, counted) in [
            ("SELECT $1::int, $2::int", vec![Param::Int(1)], 2),
            ("SELECT 1", vec![Param::Int(1)], 0),
            // One placeholder written twice is one value.
            ("SELECT $1::int + $1::int", vec![Param::Int(1); 2], 1),
            // Without values, the statement's placeholder still counts.
            ("SELECT pg_advisory_lock(424253), $1::int", vec![], 1),
            (
                "SELECT pg_advisory_lock(424253), $1::int",
                vec![Param::Int(1); 2],
                1,
            ),
        ] {
            assert_eq!(open(&client, None).await, Ok(Tx::Open), "{text}");
            let ran = run_statement(&client, text, 10, mode, &stop, Some(&sent)).await;
            let error = count_mismatch(counted, sent.len());
            let position = None;
            let refused = StatementOutcome::Error { error, position };
            // Nothing ran, so nothing failed.
            assert_eq!(ran, Ok((refused, Tx::Open)), "{text}");
            // Asked before the close, which would release the lock.
            assert!(lock_is_free(&other, 424253).await, "{text}");
            assert_eq!(close(&client, Tx::Open).await, Ok(()), "{text}");
        }
    }

    #[tokio::test]
    async fn an_agents_run_has_the_servers_timeout_for_its_transaction_only() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        assert_eq!(setting(&conn, "statement_timeout").await, "0");
        {
            let client = conn.client.lock().await;
            let sent = Sent {
                params: &[],
                timeout: Duration::from_millis(250),
            };
            assert_eq!(open(&client, Some(sent)).await, Ok(Tx::Open));
        }
        assert_eq!(setting(&conn, "statement_timeout").await, "250ms");
        // A lock is waited for as long as the statement may run, no less.
        assert_eq!(setting(&conn, "lock_timeout").await, "0");
        {
            let client = conn.client.lock().await;
            close(&client, Tx::Open).await.unwrap();
        }
        assert_eq!(setting(&conn, "statement_timeout").await, "0");
    }

    #[tokio::test]
    async fn a_run_plans_its_cursors_for_every_row_in_its_transaction_only() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        // A value of the session's own, whatever the server's default is.
        {
            let client = conn.client.lock().await;
            let set = client.batch_execute("SET cursor_tuple_fraction = 0.25");
            set.await.unwrap();
        }
        assert_eq!(setting(&conn, "cursor_tuple_fraction").await, "0.25");
        {
            let client = conn.client.lock().await;
            assert_eq!(open(&client, None).await, Ok(Tx::Open));
        }
        assert_eq!(setting(&conn, "cursor_tuple_fraction").await, "1");
        {
            let client = conn.client.lock().await;
            close(&client, Tx::Open).await.unwrap();
        }
        assert_eq!(setting(&conn, "cursor_tuple_fraction").await, "0.25");
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

    /// A read-only session through a proxy in front of the test server,
    /// which sends the server `instead` for a simple query that is exactly
    /// `sql`, given as `(sql, instead)` among `swaps`. Connections after the
    /// session's, a cancel's, pass through as they are.
    async fn proxied_session(url: &str, swaps: &[(&'static str, &'static str)]) -> Conn {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut spec, secrets) = crate::ConnectSpec::from_url(url).unwrap();
        spec.tls = crate::TlsMode::Disable;
        let server = (spec.host.clone(), spec.port);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        spec.host = "127.0.0.1".to_owned();
        spec.port = listener.local_addr().unwrap().port();
        let swaps = swaps.to_vec();
        tokio::spawn(async move {
            let (client, _) = listener.accept().await.unwrap();
            let upstream = tokio::net::TcpStream::connect(server.clone())
                .await
                .unwrap();
            tokio::spawn(async move {
                while let Ok((mut other, _)) = listener.accept().await {
                    let Ok(mut upstream) = tokio::net::TcpStream::connect(server.clone()).await
                    else {
                        return;
                    };
                    tokio::spawn(async move {
                        tokio::io::copy_bidirectional(&mut other, &mut upstream)
                            .await
                            .ok();
                    });
                }
            });
            let (mut from_client, mut to_client) = client.into_split();
            let (mut from_server, mut to_server) = upstream.into_split();
            tokio::spawn(async move {
                tokio::io::copy(&mut from_server, &mut to_client).await.ok();
            });
            // The startup message has no kind byte; every one after it has.
            let mut kind = None;
            loop {
                let Ok(length) = from_client.read_u32().await else {
                    return;
                };
                let mut body = vec![0; length as usize - 4];
                if from_client.read_exact(&mut body).await.is_err() {
                    return;
                }
                let swap = swaps
                    .iter()
                    .find(|(sql, _)| body.strip_suffix(b"\0") == Some(sql.as_bytes()));
                if let (Some(b'Q'), Some((_, instead))) = (kind, swap) {
                    body = [instead.as_bytes(), b"\0"].concat();
                }
                let length = body.len() as u32 + 4;
                let mut message = Vec::from_iter(kind);
                message.extend_from_slice(&length.to_be_bytes());
                message.extend_from_slice(&body);
                if to_server.write_all(&message).await.is_err() {
                    return;
                }
                let Ok(next) = from_client.read_u8().await else {
                    return;
                };
                kind = Some(next);
            }
        });
        Conn::connect(&spec, &secrets, None, crate::Access::ReadOnly)
            .await
            .unwrap()
    }

    /// A SELECT asks the server once: the opening, the prepare and the
    /// cursor go out together, and the close's check, rollback and release
    /// ride at the end of the cursor's message. Each statement after it
    /// adds its prepare and cursor, with no swap of its own before them.
    #[tokio::test]
    async fn a_select_asks_the_server_once() {
        let Some(url) = test_url() else {
            return;
        };
        let (conn, trips) = super::super::tests::trip_counted(&url).await;
        for (script, expected) in [
            (&["SELECT 1"][..], 1),
            (&["SELECT 1", "SELECT 2"][..], 2),
            (&["SELECT 1", "SELECT 2", "SELECT 3"][..], 3),
        ] {
            let before = trips.count();
            let outcome = past_the_refusal(&conn, script, &StopFlag::new())
                .await
                .unwrap();
            assert_eq!(outcome.results.len(), script.len(), "{outcome:?}");
            assert!(
                outcome
                    .results
                    .iter()
                    .all(|result| matches!(result.outcome, StatementOutcome::Rows { .. })),
                "{outcome:?}"
            );
            assert_eq!(trips.count() - before, expected, "{script:?}");
        }
        // The close left no transaction behind.
        let client = conn.client.lock().await;
        assert_eq!(in_transaction(&client).await, Ok(Tx::Left));
    }

    #[test]
    fn a_flights_cursor_message_is_the_guard_the_cursor_and_the_close() {
        assert_eq!(FLIGHT_PREFIX, format!("{GUARD}; {CURSOR_PREFIX}"));
        let message = cursor_message("SELECT 1 -- note", 10, false);
        assert_eq!(
            message,
            format!(
                "{FLIGHT_PREFIX}SELECT 1 -- note\n;FETCH 11 FROM tabletist_sql; CLOSE tabletist_sql"
            )
        );
        let message = cursor_message("SELECT 1", 10, true);
        assert!(message.starts_with(FLIGHT_PREFIX), "{message}");
        assert!(
            message.ends_with(&format!("CLOSE tabletist_sql; {CONFIRM_AND_END}")),
            "{message}"
        );
    }

    /// A flight's cursor message runs nothing outside the script's own
    /// transaction: not outside any, where a message of several statements
    /// would run it in a transaction of its own that is not read-only, nor
    /// in another one, which does not have the guard's savepoint.
    #[tokio::test]
    async fn a_flight_runs_nothing_outside_the_scripts_transaction() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let other = session(&url).await;
        let client = conn.client.lock().await;
        let (stop, mode) = (StopFlag::new(), ScriptMode::ReadOnly);
        let text = "SELECT pg_advisory_lock(424265)";
        for begin in [None, Some("BEGIN READ ONLY")] {
            if let Some(begin) = begin {
                client.batch_execute(begin).await.unwrap();
            }
            let (outcome, tx) = run_statement(&client, text, 10, mode, &stop, None)
                .await
                .unwrap();
            assert!(
                matches!(outcome, StatementOutcome::Error { .. }),
                "{begin:?}: {outcome:?}"
            );
            assert_eq!(tx, Tx::Left, "{begin:?}");
            assert!(lock_is_free(&other, 424265).await, "{begin:?}");
            rollback(&client).await.unwrap();
        }
    }

    /// The opening goes out with the first statement. A cancel that lands
    /// on its `BEGIN` leaves no transaction, and the statement must not run
    /// in a transaction of its own: here one that would keep a setting.
    /// The run is stopped, with no results, and the session goes on.
    #[tokio::test]
    async fn a_cancel_on_the_opening_runs_nothing_of_the_statement() {
        let Some(url) = test_url() else {
            return;
        };
        let opening: &'static str = Box::leak(opening(None).into_boxed_str());
        let cancelled = "DO $$ BEGIN RAISE EXCEPTION USING ERRCODE = 'query_canceled'; END $$";
        let conn = proxied_session(&url, &[(opening, cancelled)]).await;
        let text = "SELECT set_config('tabletist.probe', 'ran', false)";
        let stop = StopFlag::new();
        let outcome = past_the_refusal(&conn, &[text], &stop).await.unwrap();
        assert!(outcome.stopped, "{outcome:?}");
        assert!(outcome.results.is_empty(), "{outcome:?}");
        assert!(stop.is_finishing());
        let client = conn.client.lock().await;
        let probe = "SELECT coalesce(current_setting('tabletist.probe', true), '')";
        let kept = client.simple_query(probe).await.unwrap();
        assert_eq!(first_text(&kept).as_deref(), Some(""));
        assert_eq!(in_transaction(&client).await, Ok(Tx::Left));
    }

    /// A cancel that lands on the rollback, or on the release of the
    /// advisory locks, that ride at the end of the cursor's message stops
    /// the rest of it; the close runs that step once more, and the rows
    /// stand. The proxy sends the server a slow statement in the step's
    /// place, as for the close on its own.
    #[tokio::test]
    async fn a_cancel_on_the_close_in_a_flight_runs_the_cancelled_step_again() {
        let Some(url) = test_url() else {
            return;
        };
        let other = session(&url).await;
        let text = "SELECT pg_advisory_lock(424266)";
        let message = cursor_message(text, 10, true);
        let slow_rollback = message.replacen("; ROLLBACK;", "; SELECT pg_sleep(10);", 1);
        let slow_release =
            message.replacen("SELECT pg_advisory_unlock_all()", "SELECT pg_sleep(10)", 1);
        let message: &'static str = Box::leak(message.into_boxed_str());
        for (step, slow) in [("rollback", slow_rollback), ("release", slow_release)] {
            assert_ne!(slow, message, "{step}");
            let slow: &'static str = Box::leak(slow.into_boxed_str());
            let conn = proxied_session(&url, &[(message, slow)]).await;
            let cancel = conn.cancel_handle();
            let cancelling = async {
                tokio::time::sleep(Duration::from_millis(300)).await;
                cancel.cancel().await
            };
            let (script, stop) = ([text], StopFlag::new());
            let (ran, cancelled) =
                tokio::join!(past_the_refusal(&conn, &script, &stop), cancelling);
            assert_eq!(cancelled, Ok(()), "{step}");
            let outcome = ran.unwrap();
            assert!(
                matches!(
                    &outcome.results[..],
                    [StatementResult {
                        outcome: StatementOutcome::Rows { .. },
                        ..
                    }]
                ),
                "{step}: {outcome:?}"
            );
            let client = conn.client.lock().await;
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left), "{step}");
            assert!(lock_is_free(&other, 424266).await, "{step}");
        }
    }

    /// A cancel that lands on the close's question at the end of the
    /// cursor's message leaves the asking whose failed block it is to the
    /// close, and the backend repeats its cancel until the run is
    /// finishing. So the close asks only once the run is finishing, and a
    /// cancel sent before then does not land on the asking twice and close
    /// the session. The proxy sends the server a slow statement in the
    /// question's place, and makes the asking slow too.
    #[tokio::test]
    async fn repeated_cancels_do_not_land_on_the_close_after_a_flight() {
        let Some(url) = test_url() else {
            return;
        };
        let text = "SELECT 1";
        let message = cursor_message(text, 10, true);
        let slow = message.replacen("SHOW transaction_read_only", "SELECT pg_sleep(10)", 1);
        assert_ne!(slow, message);
        let asking = "ROLLBACK TO SAVEPOINT tabletist_guard";
        let slow_asking = "ROLLBACK TO SAVEPOINT tabletist_guard; SELECT pg_sleep(1)";
        let message: &'static str = Box::leak(message.into_boxed_str());
        let slow: &'static str = Box::leak(slow.into_boxed_str());
        let conn = proxied_session(&url, &[(message, slow), (asking, slow_asking)]).await;
        let cancel = conn.cancel_handle();
        let (script, stop) = ([text], StopFlag::new());
        // As the backend does: a cancel every 100 ms until the run is
        // finishing, or over.
        let over = std::sync::atomic::AtomicBool::new(false);
        let running = async {
            let ran = past_the_refusal(&conn, &script, &stop).await;
            over.store(true, std::sync::atomic::Ordering::SeqCst);
            ran
        };
        let cancelling = async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let mut sent = 0;
            while !stop.is_finishing() && !over.load(std::sync::atomic::Ordering::SeqCst) {
                cancel.cancel().await.unwrap();
                sent += 1;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            sent
        };
        let (ran, sent) = tokio::join!(running, cancelling);
        assert!(sent >= 1);
        let outcome = ran.unwrap();
        assert!(
            matches!(
                &outcome.results[..],
                [StatementResult {
                    outcome: StatementOutcome::Rows { .. },
                    ..
                }]
            ),
            "{outcome:?}"
        );
        let client = conn.client.lock().await;
        assert_eq!(in_transaction(&client).await, Ok(Tx::Left));
    }

    /// The SQLSTATE a statement failed with.
    fn code(outcome: &StatementOutcome) -> Option<&str> {
        match outcome {
            StatementOutcome::Error {
                error: Error::Query { code, .. },
                ..
            } => code.as_deref(),
            _ => None,
        }
    }

    /// Whether the session advisory lock `key` is free: `other` can take
    /// it, and gives it back.
    async fn lock_is_free(other: &Conn, key: i64) -> bool {
        let client = other.client.lock().await;
        let sql = format!("SELECT pg_try_advisory_lock({key}), pg_advisory_unlock({key})");
        let taken = client.simple_query(&sql).await.unwrap();
        first_text(&taken).as_deref() == Some("t")
    }

    /// `text` run as a statement of a SQL editor run that reads, in a
    /// transaction of its own, which is left open: with where it stands.
    async fn in_open_run(client: &tokio_postgres::Client, text: &str) -> (StatementOutcome, Tx) {
        assert_eq!(open(client, None).await, Ok(Tx::Open), "{text}");
        let (stop, mode) = (StopFlag::new(), ScriptMode::ReadOnly);
        run_statement(client, text, 10, mode, &stop, None)
            .await
            .unwrap()
    }

    /// The prepare that goes out with the cursor still decides whether it
    /// runs. A text of two statements fails the prepare, and with it the
    /// transaction, so the cursor's message, which reads as the cursor and
    /// a statement after it, runs nothing: neither the first statement nor
    /// the second, which would have run past the cursor and its row cap.
    #[tokio::test]
    async fn a_text_of_two_statements_runs_neither() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let other = session(&url).await;
        let client = conn.client.lock().await;
        for text in [
            "SELECT pg_advisory_lock(424250); SELECT pg_advisory_lock(424251)",
            "SELECT pg_advisory_lock(424250);\nSELECT pg_advisory_lock(424251) \
             FROM generate_series(1, 5000)",
        ] {
            let (outcome, tx) = in_open_run(&client, text).await;
            // "cannot insert multiple commands into a prepared statement"
            assert_eq!(code(&outcome), Some("42601"), "{text}: {outcome:?}");
            assert_eq!(tx, Tx::Aborted, "{text}");
            // Asked while the transaction is still open: the close would
            // release a lock either took.
            assert!(lock_is_free(&other, 424250).await, "{text}");
            assert!(lock_is_free(&other, 424251).await, "{text}");
            assert_eq!(close(&client, tx).await, Ok(()), "{text}");
        }
    }

    /// A quote or a comment the text leaves open would swallow the `FETCH`
    /// and `CLOSE` after it. The prepare refuses it, so its message runs
    /// nothing, and no cursor is left declared: the next run works. A --
    /// comment at the end is no harm: the cursor's newline ends it.
    #[tokio::test]
    async fn a_quote_or_comment_left_open_cannot_reach_the_fetch() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let other = session(&url).await;
        {
            let client = conn.client.lock().await;
            for text in [
                "SELECT pg_advisory_lock(424252) /* note",
                "SELECT pg_advisory_lock(424252), 'note",
                "SELECT pg_advisory_lock(424252), $$note",
                "SELECT pg_advisory_lock(424252) AS \"note",
            ] {
                let (outcome, tx) = in_open_run(&client, text).await;
                assert_eq!(code(&outcome), Some("42601"), "{text}: {outcome:?}");
                assert_eq!(tx, Tx::Aborted, "{text}");
                assert!(lock_is_free(&other, 424252).await, "{text}");
                assert_eq!(close(&client, tx).await, Ok(()), "{text}");
            }
        }
        for text in ["SELECT 1 /* note */", "SELECT 1 -- note"] {
            let outcome = past_the_refusal(&conn, &[text], &StopFlag::new())
                .await
                .unwrap();
            assert!(
                matches!(
                    &outcome.results[..],
                    [StatementResult { outcome: StatementOutcome::Rows { rows, .. }, .. }]
                        if rows == &[vec![Value::Int(1)]]
                ),
                "{text}: {outcome:?}"
            );
        }
    }

    /// The prepare that goes out with the cursor types its columns, also a
    /// type the session has not met, which the driver looks up after the
    /// cursor ran. Where the cursor failed, that lookup fails too, and the
    /// cursor's error is the one the statement shows. A statement without
    /// columns is done, as it was before it ran as a cursor.
    #[tokio::test]
    async fn the_prepare_with_the_cursor_types_its_columns() {
        let Some(url) = test_url() else {
            return;
        };
        let (admin, _turn) = probe(&url).await;
        admin
            .batch_execute(
                "DO $$ BEGIN CREATE TYPE tabletist_mood AS ENUM ('calm'); \
                 EXCEPTION WHEN duplicate_object THEN NULL; END $$",
            )
            .await
            .unwrap();
        // A session of its own each time: the driver keeps the types it
        // looked up.
        let conn = session(&url).await;
        let client = conn.client.lock().await;
        let (outcome, tx) = in_open_run(
            &client,
            "SELECT 7::int4 AS n, 'x'::text AS t, 2.5::numeric AS d, true AS b, \
             now() AS at, 'calm'::tabletist_mood AS mood, \
             ARRAY['calm']::tabletist_mood[] AS moods, NULL::int8 AS gone",
        )
        .await;
        assert_eq!(tx, Tx::Open);
        let StatementOutcome::Rows {
            columns,
            rows,
            truncated: false,
        } = outcome
        else {
            panic!("{outcome:?}");
        };
        let named: Vec<_> = columns
            .iter()
            .map(|column| (column.name.as_str(), column.type_name.as_str()))
            .collect();
        assert_eq!(
            named,
            [
                ("n", "int4"),
                ("t", "text"),
                ("d", "numeric"),
                ("b", "bool"),
                ("at", "timestamptz"),
                ("mood", "tabletist_mood"),
                ("moods", "_tabletist_mood"),
                ("gone", "int8"),
            ]
        );
        assert_eq!(columns[0].kind, crate::ValueKind::Numeric);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], Value::Int(7));
        assert_eq!(rows[0][1], Value::Text("x".into()));
        assert_eq!(rows[0][3], Value::Bool(true));
        assert_eq!(rows[0][5], Value::Text("calm".into()));
        assert_eq!(rows[0][7], Value::Null);
        assert_eq!(close(&client, tx).await, Ok(()));
        drop(client);

        let conn = session(&url).await;
        let client = conn.client.lock().await;
        let (outcome, tx) = in_open_run(
            &client,
            "SELECT 'calm'::tabletist_mood AS mood, n / 0 FROM generate_series(1, 1) n",
        )
        .await;
        // Division by zero, not the lookup's failed transaction.
        assert_eq!(code(&outcome), Some("22012"), "{outcome:?}");
        assert_eq!(tx, Tx::Aborted);
        assert_eq!(close(&client, tx).await, Ok(()));

        let (outcome, tx) = in_open_run(&client, "SELECT FROM generate_series(1, 3)").await;
        let affected = None;
        assert_eq!(
            outcome,
            StatementOutcome::Done {
                affected,
                warnings: 0
            }
        );
        assert_eq!(close(&client, tx).await, Ok(()));
    }

    /// The close's one message ends the transaction and releases a lock a
    /// statement took, and a transaction that is no longer read-only still
    /// fails the run, rolled back.
    #[tokio::test]
    async fn the_close_ends_the_transaction_and_releases_locks_in_one_message() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let other = session(&url).await;
        let client = conn.client.lock().await;
        assert_eq!(open(&client, None).await, Ok(Tx::Open));
        client
            .batch_execute("SELECT pg_advisory_lock(424243)")
            .await
            .unwrap();
        assert_eq!(close(&client, Tx::Open).await, Ok(()));
        assert_eq!(in_transaction(&client).await, Ok(Tx::Left));
        let taken = other
            .client
            .lock()
            .await
            .simple_query("SELECT pg_try_advisory_lock(424243)")
            .await
            .unwrap();
        assert_eq!(first_text(&taken).as_deref(), Some("t"));

        other
            .client
            .lock()
            .await
            .batch_execute("SELECT pg_advisory_unlock(424243)")
            .await
            .unwrap();

        // A block with the guard's savepoint that is not read-only, as the
        // close would find one a statement the refusal missed had made. Its
        // lock goes too.
        client
            .batch_execute(
                "BEGIN READ WRITE; SAVEPOINT tabletist_guard; SELECT pg_advisory_lock(424244)",
            )
            .await
            .unwrap();
        assert_eq!(close(&client, Tx::Open).await, Err(Error::LeftReadOnly));
        assert_eq!(in_transaction(&client).await, Ok(Tx::Left));
        let taken = other
            .client
            .lock()
            .await
            .simple_query("SELECT pg_try_advisory_lock(424244), pg_advisory_unlock(424244)")
            .await
            .unwrap();
        assert_eq!(first_text(&taken).as_deref(), Some("t"));
    }

    /// A cancel that lands on the close's rollback, or on its release of the
    /// advisory locks, stops the rest of the close's message; the close runs
    /// that step once more, so the transaction ends and the lock goes all the
    /// same. A real `ROLLBACK` is over too soon to cancel on cue, so the
    /// proxy sends the server a slow statement in the step's place.
    #[tokio::test]
    async fn a_cancel_on_the_close_runs_the_cancelled_step_again() {
        let Some(url) = test_url() else {
            return;
        };
        let other = session(&url).await;
        let slow_rollback = "RELEASE tabletist_guard; SAVEPOINT tabletist_guard; \
             SHOW transaction_read_only; SELECT pg_sleep(10); SELECT pg_advisory_unlock_all()";
        let slow_release = "RELEASE tabletist_guard; SAVEPOINT tabletist_guard; \
             SHOW transaction_read_only; ROLLBACK; SELECT pg_sleep(10)";
        for (step, slow) in [("rollback", slow_rollback), ("release", slow_release)] {
            let conn = proxied_session(&url, &[(CONFIRM_AND_END, slow)]).await;
            let cancel = conn.cancel_handle();
            let client = conn.client.lock().await;
            assert_eq!(open(&client, None).await, Ok(Tx::Open), "{step}");
            client
                .batch_execute("SELECT pg_advisory_lock(424245)")
                .await
                .unwrap();
            let cancelling = async {
                tokio::time::sleep(Duration::from_millis(300)).await;
                cancel.cancel().await
            };
            let (closed, cancelled) = tokio::time::timeout(Duration::from_secs(5), async {
                tokio::join!(close(&client, Tx::Open), cancelling)
            })
            .await
            .expect(step);
            assert_eq!(cancelled, Ok(()), "{step}");
            assert_eq!(closed, Ok(()), "{step}");
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left), "{step}");
            let taken = other
                .client
                .lock()
                .await
                .simple_query("SELECT pg_try_advisory_lock(424245), pg_advisory_unlock(424245)")
                .await
                .unwrap();
            assert_eq!(first_text(&taken).as_deref(), Some("t"), "{step}");
        }
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
        assert_eq!(open(&client, None).await, Ok(Tx::Open));
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
            None,
            Flights::default(),
            &mut outcome,
        )
        .await;
        assert!(matches!(tx, Ok(Stand::Tx(Tx::Open))));
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
            assert_eq!(open(&client, None).await, Ok(Tx::Open));
            assert_eq!(in_transaction(&client).await, Ok(Tx::Open));
            assert_eq!(close(&client, Tx::Open).await, Ok(()));
            // Closed: no transaction is left.
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left));

            assert_eq!(open(&client, None).await, Ok(Tx::Open));
            assert!(client.batch_execute("SELECT 1 / 0").await.is_err());
            assert_eq!(in_transaction(&client).await, Ok(Tx::Aborted));
            assert_eq!(close(&client, Tx::Open).await, Ok(()));
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left));

            // Another transaction is not the script's, block or not.
            assert_eq!(open(&client, None).await, Ok(Tx::Open));
            client.batch_execute("COMMIT AND CHAIN").await.unwrap();
            assert_eq!(close(&client, Tx::Open).await, Err(Error::LeftReadOnly));
            assert_eq!(in_transaction(&client).await, Ok(Tx::Left));

            // Nor is it once it failed (a cancel that landed on a check):
            // a failed block does not say whose it is, its savepoint does.
            assert_eq!(open(&client, None).await, Ok(Tx::Open));
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
