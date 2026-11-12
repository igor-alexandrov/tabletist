//! Running a SQL editor script on MySQL: one transaction, the checks
//! around every statement, and the session reset that ends it. This file
//! is the run that only reads, in a read-only transaction that is rolled
//! back; `write` is the run that commits.

use std::time::{Duration, Instant};

use mysql_async::consts::StatusFlags;
use mysql_async::prelude::Queryable;
use mysql_async::{Params, QueryResult, TextProtocol};
use mysql_common::named_params::ParsedNamedParams;

use super::{
    Conn, READ_ONLY, Session, UNKNOWN_SYSTEM_VARIABLE, column_metas, execute, from_row,
    prepare_session, query_error, row_values, status,
};
use crate::agent::{Limits, Param, Read, Sent, count_mismatch};
use crate::script::{cannot_start, cleanup_failed, retry_cancelled, statement_failed};
use crate::{
    Access, Dialect, Error, Result, ScriptMode, ScriptOutcome, StatementOutcome, StatementResult,
    StopFlag,
};

mod write;

impl Conn {
    /// See [`crate::Connection::run_script`]. Statements run through the
    /// prepared protocol, which cannot hold two, and `sql_select_limit`
    /// makes the server stop at `limit + 1` rows. The transaction is
    /// managed by hand, and the session is reset afterwards: what a script
    /// set must not reach table browsing or the next run. A run that only
    /// reads hands its outcome back before the reset, which goes on under
    /// the session's lock (`Unfinished`).
    ///
    /// The future must be awaited to its end: a run that is dropped leaves
    /// the session inside the transaction, with the script's settings. The
    /// backend never drops one; it stops a run through `stop` and the
    /// session's cancel.
    pub(super) async fn script(
        &self,
        texts: &[String],
        limit: u32,
        mode: ScriptMode,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let mut session = std::sync::Arc::clone(&self.conn).lock_owned().await;
        let conn = session.conn()?;
        let kind = self.kind();
        // Said before anything runs, not found out by the cleanup, which
        // would close the session after every run.
        if !can_reset(kind.mariadb, conn.server_version()) {
            return Err(Error::Unsupported(
                "the SQL editor needs MySQL 5.7.3 or MariaDB 10.2.4 or later",
            ));
        }
        if mode == ScriptMode::Write {
            return write::run(conn, texts, limit as usize, stop).await;
        }
        let (outcome, _, unfinished) =
            read_only(conn, texts, limit, kind, stop, None, None).await?;
        unfinished.finish_later(session);
        Ok(outcome)
    }

    /// See [`crate::Connection::run_agent_read`]: the run of a script of
    /// one statement that only reads, with the statement's values bound
    /// and the server's own limit on its time set for the session, which
    /// the reset that ends the run takes away, with which of its result
    /// columns are a masked column's. Awaited to its end, as
    /// [`Self::script`] is.
    pub(super) async fn agent_read(
        &self,
        read: &Read,
        limits: Limits,
        sources: &crate::agent::Sources,
        stop: &StopFlag,
    ) -> Result<(ScriptOutcome, Vec<bool>)> {
        let mut session = std::sync::Arc::clone(&self.conn).lock_owned().await;
        let conn = session.conn()?;
        let kind = self.kind();
        if !can_reset(kind.mariadb, conn.server_version()) {
            return Err(Error::Unsupported(
                "a read for an agent needs MySQL 5.7.3 or MariaDB 10.2.4 or later",
            ));
        }
        let texts = [read.text().to_owned()];
        let sent = Sent {
            params: read.params(),
            timeout: limits.timeout,
        };
        let hiding = sources
            .guards_anything()
            .then_some(Hiding { read, sources });
        let (outcome, masked, unfinished) =
            read_only(conn, &texts, limits.rows, kind, stop, Some(sent), hiding).await?;
        unfinished.finish_later(session);
        Ok((outcome, masked))
    }

    /// What a run needs to know of this session from the start.
    fn kind(&self) -> SessionKind {
        SessionKind {
            access: self.access,
            mariadb: self.version.contains("MariaDB"),
        }
    }
}

/// What a run knows of its session from the start: what it was opened as,
/// and whether its server is MariaDB, whose names for some settings are
/// not MySQL's.
#[derive(Debug, Clone, Copy)]
struct SessionKind {
    access: Access,
    mariadb: bool,
}

impl SessionKind {
    /// The read-only setting's names (`READ_ONLY_SETTINGS`), the one the
    /// server knows first: MariaDB before 11.1 knows only the older one.
    /// Asked under the other too where the server does not know the first.
    fn read_only_names(self, version: (u16, u16, u16)) -> [&'static str; 2] {
        let [name, older] = READ_ONLY_SETTINGS;
        if self.mariadb && version < (11, 1, 0) {
            [older, name]
        } else {
            [name, older]
        }
    }

    /// The statements that limit a statement's time (`time_limits`), the
    /// server's own form first.
    fn time_limits(self, ms: u32) -> [String; 2] {
        let [mysql, mariadb] = time_limits(ms);
        if self.mariadb {
            [mariadb, mysql]
        } else {
            [mysql, mariadb]
        }
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
/// result columns are a masked column's (none where nothing is masked),
/// and the reset that is still to come where the run ended cleanly.
async fn read_only(
    conn: &mut mysql_async::Conn,
    texts: &[String],
    limit: u32,
    kind: SessionKind,
    stop: &StopFlag,
    sent: Option<Sent<'_>>,
    hiding: Option<Hiding<'_>>,
) -> Result<(ScriptOutcome, Vec<bool>, Unfinished)> {
    let mut outcome = ScriptOutcome::default();
    let mut masked = Vec::new();
    let names = kind.read_only_names(conn.server_version());
    let ended = match open(conn, limit, kind, sent).await {
        // A lost session ends the run here: nothing to close.
        Ok(()) => match refused_for_hiding(conn, hiding).await? {
            Err(error) => {
                outcome.stopped = error == Error::Cancelled;
                outcome.results.push(StatementResult {
                    elapsed: Duration::ZERO,
                    outcome: statement_failed(error, None)?,
                });
                Ended::Unconfirmed
            }
            Ok(columns) => {
                masked = columns;
                let params = sent.map(|sent| sent.params);
                let limit = limit as usize;
                statements(conn, texts, limit, names, stop, params, &mut outcome).await?
            }
        },
        // A cancel landed on the opening queries: no results.
        Err(Error::Cancelled) => {
            outcome.stopped = true;
            Ended::Unopened
        }
        Err(error) if error.is_connection_lost() => return Err(error),
        // The transaction could not start. The next run would fail the
        // same way, so the session is closed, after an attempt to end
        // what is open.
        Err(error) => Ended::Broken(cannot_start(ScriptMode::ReadOnly, &error)),
    };
    // From here on a cancel would land on the cleanup: tell the
    // backend to stop repeating its cancel.
    stop.finish();
    let unfinished = end(conn, ended, kind).await?;
    Ok((outcome, masked, unfinished))
}

/// Why an agent's statement may not run on a connection that hides tables
/// or masks columns (`super::hidden`): a refusal, the server's own error,
/// or a cancel. `Ok` where it may run, with which of its result columns
/// are a masked column's (none where nothing is hidden or masked). `Err`
/// is a lost session.
async fn refused_for_hiding(
    conn: &mut mysql_async::Conn,
    hiding: Option<Hiding<'_>>,
) -> Result<std::result::Result<Vec<bool>, Error>> {
    let Some(hiding) = hiding else {
        return Ok(Ok(Vec::new()));
    };
    match super::hidden::check(conn, hiding.read, hiding.sources).await {
        Ok(checked) => Ok(checked),
        Err(error) if error.is_connection_lost() => Err(error),
        Err(error) => Ok(Err(error)),
    }
}

/// Where a session stands relative to the read-only transaction a script
/// runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    /// Inside a read-only transaction, in a read-only session.
    Inside,
    /// In no transaction: a statement ended the script's. The server
    /// commits before it runs DDL, also when it then refuses the DDL as
    /// read-only. The session is still read-only.
    Outside,
    /// Not read-only any more: the session, or the transaction it is in.
    Left,
}

/// What a run knows of its session once no more statements run.
#[derive(Debug, PartialEq)]
enum Ended {
    /// Nothing the run saw says the session left read-only. The close asks
    /// the server before it trusts this.
    Unconfirmed,
    /// A cancel landed on the opening queries: no statement of the script
    /// ran, so there is nothing to confirm. On a writable session the
    /// cancel may have landed before the fence, where the session is
    /// rightly read-write.
    Unopened,
    /// The check before a statement found the session read-write.
    Left,
    /// A check could not be made, or a transaction could not start: the
    /// session is closed with this error.
    Broken(Error),
}

/// The session's read-only setting. MariaDB before 11.1 knows it only
/// under its older name.
const READ_ONLY_SETTINGS: [&str; 2] = ["transaction_read_only", "tx_read_only"];

/// Whether the server knows `COM_RESET_CONNECTION`, which the close needs:
/// MySQL from 5.7.3, MariaDB from 10.2.4. The driver's own rule for
/// `Conn::reset`, asked before anything runs.
fn can_reset(mariadb: bool, version: (u16, u16, u16)) -> bool {
    if mariadb {
        version >= (10, 2, 4)
    } else {
        version >= (5, 7, 3)
    }
}

/// Starts a read-only transaction, and holds the server to saying so: its
/// status must mark the session as inside a transaction, and that one as
/// read-only. Every server that can reset a session does (MySQL since
/// 5.6.5, MariaDB since 10.0); `stands` reads the same marks later, so a
/// server or proxy that drops them cannot run scripts.
async fn begin(conn: &mut mysql_async::Conn) -> Result<()> {
    execute(conn, BEGIN).await?;
    started(status(conn))
}

/// Starts the script's kind of transaction.
const BEGIN: &str = "START TRANSACTION READ ONLY";

/// Whether the status after `START TRANSACTION READ ONLY` says what it
/// must.
fn started(status: StatusFlags) -> Result<()> {
    if !status.contains(StatusFlags::SERVER_STATUS_IN_TRANS) {
        return Err(Error::query("the server did not start a transaction"));
    }
    if !status.contains(StatusFlags::SERVER_STATUS_IN_TRANS_READONLY) {
        return Err(Error::query(
            "the server did not mark the transaction read-only",
        ));
    }
    Ok(())
}

/// Starts the script's transaction, in a session that is read-only for
/// the run, sets its row limit, and asks the server where the session
/// stands: inside the read-only transaction, or the open fails. With
/// `sql_select_limit` the server stops producing rows, and the close's
/// reset puts the default back. `Err` is a cancel that landed on these
/// queries, a lost session, or a transaction that could not start.
///
/// An agent's run also sets the server's own limit on a statement's time
/// (`time_limits`), which the same reset takes away.
///
/// The steps go as one message, one round trip (`opening`). A failed step
/// stops the rest; where a setting the message names is unknown to the
/// server, the steps from there run on their own, under the other name.
async fn open(
    conn: &mut mysql_async::Conn,
    limit: u32,
    kind: SessionKind,
    sent: Option<Sent<'_>>,
) -> Result<()> {
    let names = kind.read_only_names(conn.server_version());
    let [own_limit, other_limit] = match sent {
        Some(sent) => kind.time_limits(sent.timeout_ms()).map(Some),
        None => [None, None],
    };
    let steps = opening(kind.access, limit, own_limit.as_deref(), names[0]);
    let (answers, failure) = answers(conn, &steps).await;
    let step = answers.len();
    if step == steps.len() {
        let last = &answers[step - 1];
        let read_only = last.rows.first().cloned().map(from_row).transpose()?;
        return opened(read_only, last.status);
    }
    let failure = failure.ok_or_else(|| Error::query("the server did not answer the open"))?;
    // A setting the server does not know under the name the message used:
    // the time limit, second last, or the question, last.
    let unknown = matches!(&failure, mysql_async::Error::Server(error)
        if error.code == UNKNOWN_SYSTEM_VARIABLE);
    let question_step = steps.len() - 1;
    let time_step = other_limit.as_ref().map(|_| question_step - 1);
    if !unknown || (step != question_step && Some(step) != time_step) {
        return Err(query_error(failure));
    }
    if Some(step) == time_step
        && let Some(other) = &other_limit
    {
        execute(conn, other).await?;
    }
    let [first, second] = names;
    let asked = if step == question_step {
        [second, first]
    } else {
        names
    };
    let read_only = read_only_setting(conn, asked).await?;
    opened(read_only, status(conn))
}

/// The open's steps, in order: a writable session is made read-only first
/// (the checks read the session's own setting, and only that setting makes
/// the server refuse DDL after the commit DDL implies; the close puts
/// read-write back), the transaction, its row limit, an agent's time limit
/// in the server's form, and last the question whether the session is
/// read-only, whose answer carries the server's status after all of them.
fn opening(access: Access, limit: u32, time_limit: Option<&str>, read_only: &str) -> Vec<String> {
    // One row more than the limit, to know whether more exist.
    let rows = u64::from(limit) + 1;
    let fence = (access == Access::Writable).then_some(READ_ONLY.to_owned());
    fence
        .into_iter()
        .chain([
            BEGIN.to_owned(),
            format!("SET SESSION sql_select_limit = {rows}"),
        ])
        .chain(time_limit.map(str::to_owned))
        .chain([question(read_only)])
        .collect()
}

/// Whether the open's last answer says what it must: the server marks the
/// session as inside a transaction, and that one as read-only (`started`),
/// and the session's own setting is not read-write.
fn opened(read_only: Option<i64>, status: StatusFlags) -> Result<()> {
    started(status)?;
    match stands(read_only, status)? {
        Standing::Inside => Ok(()),
        Standing::Outside | Standing::Left => Err(Error::query(
            "the server did not say that the session is read-only",
        )),
    }
}

/// One statement's answer in a message of several.
#[derive(Debug)]
pub(super) struct Answer {
    /// Its rows: none for a statement that answers without.
    pub(super) rows: Vec<mysql_async::Row>,
    /// The server's status as the driver had it once the answer was read.
    /// For a statement without rows that is its own; for one with rows only
    /// when it is the message's last, since reading its end also reads the
    /// next statement's first answer.
    status: StatusFlags,
}

/// What the server answered to `steps`, sent as one message: an answer for
/// each statement up to the one that failed, and that one's error. The
/// server runs them in order and stops at the first that fails.
pub(super) async fn answers(
    conn: &mut mysql_async::Conn,
    steps: &[String],
) -> (Vec<Answer>, Option<mysql_async::Error>) {
    let mut answers = Vec::new();
    // The first statement's answer, or its error, is in once this returns.
    // The rest stays on the connection, where a new `QueryResult` reads it.
    if let Err(error) = conn.query_iter(steps.join("; ")).await {
        return (answers, Some(error));
    }
    for _ in steps {
        // An answer without rows is in before its turn, with the status of
        // the statement it answers.
        let before = status(conn);
        let mut result = QueryResult::<'_, '_, TextProtocol>::new(&mut *conn);
        let without_rows = result.columns_ref().is_empty();
        let read = result.collect().await;
        match read {
            Ok(rows) => answers.push(Answer {
                status: if without_rows { before } else { status(conn) },
                rows,
            }),
            Err(error) => return (answers, Some(error)),
        }
    }
    let rest = QueryResult::<'_, '_, TextProtocol>::new(&mut *conn)
        .drop_result()
        .await;
    (answers, rest.err())
}

/// The statements that make the server end a statement over its time, as
/// MySQL and as MariaDB take them. MySQL counts `max_execution_time` in
/// milliseconds, and holds a `SELECT` to it and nothing else, which is all
/// a read is. MariaDB has no setting of that name: its
/// `max_statement_time` is in seconds. The statement cannot lift the limit:
/// the refusal stops every `SET`, and the check the optimizer hints that
/// would (`agent::check_read`).
fn time_limits(ms: u32) -> [String; 2] {
    [
        format!("SET SESSION max_execution_time = {ms}"),
        format!(
            "SET SESSION max_statement_time = {}.{:03}",
            ms / 1_000,
            ms % 1_000
        ),
    ]
}

/// Runs the statements in order, up to the first that fails or is
/// stopped, and says what that showed of the session. `Err` is a lost
/// session. `names` are the read-only setting's, the server's first.
/// `params` are an agent's values for its one statement: a script has
/// none.
///
/// The open has just asked where the session stands, and nothing of the
/// script has run since, so the first statement goes without the check:
/// it would only hear what the open did. What may run in between, an
/// agent's check against what its connection hides (`EXPLAIN`, `SHOW
/// WARNINGS`), ends no transaction and sets nothing.
async fn statements(
    conn: &mut mysql_async::Conn,
    texts: &[String],
    limit: usize,
    names: [&str; 2],
    stop: &StopFlag,
    params: Option<&[Param]>,
    outcome: &mut ScriptOutcome,
) -> Result<Ended> {
    for (index, text) in texts.iter().enumerate() {
        let checked = if stop.is_stopped() {
            Err(Error::Cancelled)
        } else if index == 0 {
            Ok(Standing::Inside)
        } else {
            ready(conn, names).await
        };
        match checked {
            Ok(Standing::Left) => return Ok(Ended::Left),
            Ok(_) => {}
            // A stop between statements, or a cancel that landed on the
            // check: this statement is the cancelled one. The close asks
            // where the session stands.
            Err(Error::Cancelled) => {
                outcome.stopped = true;
                outcome.results.push(StatementResult {
                    elapsed: Duration::ZERO,
                    outcome: StatementOutcome::Cancelled,
                });
                return Ok(Ended::Unconfirmed);
            }
            Err(error) if error.is_connection_lost() => return Err(error),
            // Not known to be read-only: nothing more runs.
            Err(error) => {
                return Ok(Ended::Broken(Error::ConnectionLost(format!(
                    "could not confirm that the session is read-only: {error}"
                ))));
            }
        }
        let started = Instant::now();
        let result = run_statement(conn, text, limit, params).await?;
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
            break;
        }
    }
    Ok(Ended::Unconfirmed)
}

/// The check before every statement: where the session stands, back
/// inside a read-only transaction when a statement ended the script's.
///
/// Outside one, a statement runs in a transaction of its own, read-only
/// like the session, unless a statement the refusal missed set the next
/// transaction's mode (`SET TRANSACTION READ WRITE`), which the session's
/// setting does not show. `START TRANSACTION READ ONLY` overrides that,
/// and inside a transaction the server refuses to change the mode. So
/// every statement runs inside a read-only transaction, whatever the one
/// before it did.
///
/// `Err` is a cancel that landed on the check, a lost session, or a check
/// that could not be made.
async fn ready(conn: &mut mysql_async::Conn, names: [&str; 2]) -> Result<Standing> {
    match standing(conn, names).await? {
        Standing::Outside => {
            begin(conn).await?;
            Ok(Standing::Inside)
        }
        standing => Ok(standing),
    }
}

/// Asks the server where the session stands: its read-only setting, under
/// `names` (the server's first), and the transaction status that comes
/// with the answer.
async fn standing(conn: &mut mysql_async::Conn, names: [&str; 2]) -> Result<Standing> {
    let read_only = read_only_setting(conn, names).await?;
    stands(read_only, status(conn))
}

/// The session's read-only setting, asked for under `name`, or under
/// `older` when the server does not know `name`.
async fn read_only_setting(
    conn: &mut mysql_async::Conn,
    [name, older]: [&str; 2],
) -> Result<Option<i64>> {
    let row = match setting(conn, name).await {
        Err(mysql_async::Error::Server(error)) if error.code == UNKNOWN_SYSTEM_VARIABLE => {
            setting(conn, older).await
        }
        answer => answer,
    };
    row.map_err(query_error)?.map(from_row).transpose()
}

/// A session setting's value (see `question`).
async fn setting(
    conn: &mut mysql_async::Conn,
    name: &str,
) -> mysql_async::Result<Option<mysql_async::Row>> {
    conn.query_first(question(name)).await
}

/// The question for a session setting's value. With a `LIMIT` of its own:
/// a script can set `sql_select_limit` to 0, and the answer must still
/// come.
fn question(name: &str) -> String {
    format!("SELECT @@session.{name} LIMIT 1")
}

/// Where a session stands, from its read-only setting and the server's
/// status. Only an explicit 0 says the session is read-write. A
/// transaction is the script's kind only when the server marks it
/// read-only, as it marked the script's own (`begin`).
fn stands(read_only: Option<i64>, status: StatusFlags) -> Result<Standing> {
    let Some(read_only) = read_only else {
        return Err(Error::query(
            "the server did not say whether the session is read-only",
        ));
    };
    Ok(if read_only == 0 {
        Standing::Left
    } else if !status.contains(StatusFlags::SERVER_STATUS_IN_TRANS) {
        Standing::Outside
    } else if !status.contains(StatusFlags::SERVER_STATUS_IN_TRANS_READONLY) {
        Standing::Left
    } else {
        Standing::Inside
    })
}

/// Ends a script's run, whatever state it is in: confirms the session is
/// still read-only, unless no statement ran, rolls back, resets the session
/// and applies the connect-time settings again. Those make a writable
/// session read-write, but only after a run that ended cleanly. A step a
/// cancel interrupted runs once more. `LeftReadOnly` or any step that fails
/// closes the session (they count as a lost connection).
///
/// The check and the rollback go as one message (`confirm`); a step it did
/// not reach runs on its own. Where they leave the run clean, the outcome
/// is known: the reset and the settings change nothing of it, and are left
/// to the caller (`Unfinished`). Otherwise they run here, before the error.
async fn end(conn: &mut mysql_async::Conn, ended: Ended, kind: SessionKind) -> Result<Unfinished> {
    let (ended, rolled_back) = match ended {
        Ended::Unconfirmed => confirm(conn, kind.read_only_names(conn.server_version())).await,
        known => (known, None),
    };
    // Whatever is open is rolled back and reset as far as that works, also
    // when the session is closed anyway.
    let rolled_back = match rolled_back {
        Some(Err(Error::Cancelled)) => execute(conn, ROLLBACK).await,
        Some(answer) => answer,
        None => retry_cancelled!(execute(conn, ROLLBACK)),
    };
    let error = match (ended, rolled_back) {
        (Ended::Unconfirmed | Ended::Unopened, Ok(())) => {
            return Ok(Unfinished {
                access: kind.access,
            });
        }
        (Ended::Unconfirmed | Ended::Unopened, Err(error)) => {
            cleanup_failed(ScriptMode::ReadOnly, &error)
        }
        (Ended::Left, _) => Error::LeftReadOnly,
        (Ended::Broken(error), _) => error,
    };
    // A run that did not end cleanly leaves the session read-only, whatever
    // it was opened as: it is about to be closed, and until then nothing
    // may write on it.
    let _ = put_back(conn, Access::ReadOnly).await;
    Err(error)
}

/// The reset and the settings that end a run which ended cleanly: its
/// transaction is rolled back, its outcome known. Until they are done the
/// session is the script's (its settings, its variables, read-write after
/// the reset), so nothing else may use it in between.
#[must_use = "the session keeps what the script set until it is finished"]
#[derive(Debug)]
struct Unfinished {
    /// What the session was opened as, which the settings put back.
    access: Access,
}

impl Unfinished {
    /// Finishes in a task of its own that holds the session's lock: what
    /// uses the session next waits for the reset and the settings, and the
    /// run's outcome does not. Where they fail, the session is closed (see
    /// `Session`), as a run that fails at them is, and its connection hung
    /// up: nothing the script left on the server outlasts the close.
    fn finish_later(self, mut session: tokio::sync::OwnedMutexGuard<Session>) {
        tokio::spawn(async move {
            let Ok(conn) = session.conn() else {
                return;
            };
            if let Err(error) = self.finish(conn).await
                && let Some(conn) = session.close(error)
            {
                // Closed first: what waits for the session gets the error
                // without waiting for the goodbye.
                drop(session);
                super::hang_up(conn).await;
            }
        });
    }

    /// Resets the session and applies the settings it was opened with.
    /// `Err` closes it.
    async fn finish(self, conn: &mut mysql_async::Conn) -> Result<()> {
        put_back(conn, self.access)
            .await
            .map_err(|error| cleanup_failed(ScriptMode::ReadOnly, &error))
    }
}

/// Resets the session and applies the connect-time settings as `access`;
/// as `Access::ReadOnly` where the reset fails, since the session is then
/// closed. Each runs once more after a cancel.
async fn put_back(conn: &mut mysql_async::Conn, access: Access) -> Result<()> {
    let reset = retry_cancelled!(reset(conn));
    let access = if reset.is_ok() {
        access
    } else {
        Access::ReadOnly
    };
    let prepared = prepare_session(conn, access).await;
    reset.and(prepared)
}

/// `end` and the reset after it, waited for.
#[cfg(test)]
async fn close(conn: &mut mysql_async::Conn, ended: Ended, kind: SessionKind) -> Result<()> {
    end(conn, ended, kind).await?.finish(conn).await
}

/// The close's check and rollback, as one message: where the session
/// stands once no more statements run, and the end of the transaction. The
/// check is needed after the last statement too: one in last position may
/// have left read-only, and the run must then fail. `DO 0` answers
/// without rows, so its answer carries the server's status as the check
/// found it (see `Answer`); the question gives the session's setting. The
/// rollback runs whatever the answer is, as it does after the steps one by
/// one, and its answer comes back with the rest (`None` where the message
/// did not reach it). Where the check did not answer, it runs on its own,
/// once more after a cancel.
async fn confirm(conn: &mut mysql_async::Conn, names: [&str; 2]) -> (Ended, Option<Result<()>>) {
    let (answers, failure) = answers(conn, &closing(names[0])).await;
    let standing = match answers.as_slice() {
        [found, asked, ..] => match asked.rows.first().cloned().map(from_row).transpose() {
            Ok(read_only) => stands(read_only, found.status),
            Err(error) => Err(error),
        },
        _ => retry_cancelled!(standing(conn, names)),
    };
    let ended = match standing {
        Ok(Standing::Left) => Ended::Left,
        Ok(Standing::Inside | Standing::Outside) => Ended::Unconfirmed,
        Err(error) => Ended::Broken(cleanup_failed(ScriptMode::ReadOnly, &error)),
    };
    let rolled_back = match (answers.len(), failure) {
        (3, _) => Some(Ok(())),
        (2, Some(error)) => Some(Err(query_error(error))),
        _ => None,
    };
    (ended, rolled_back)
}

/// The close's steps (see `confirm`), the read-only setting asked for
/// under `read_only`.
fn closing(read_only: &str) -> [String; 3] {
    ["DO 0".to_owned(), question(read_only), ROLLBACK.to_owned()]
}

/// The rollback that ends the script's transaction, whatever a script set
/// `completion_type` to: a chained one would start the next transaction,
/// and a released one would close the session.
const ROLLBACK: &str = super::write::ROLLBACK;

/// Resets the session (`COM_RESET_CONNECTION`): every session setting goes
/// back to the server's default, and user variables, temporary tables,
/// prepared statements and named locks are dropped. The connection id the
/// cancel uses stays. A read-only session is read-write until
/// `prepare_session` runs again.
async fn reset(conn: &mut mysql_async::Conn) -> Result<()> {
    if conn.reset().await.map_err(query_error)? {
        Ok(())
    } else {
        // The server predates the command. `run_script` asks first
        // (`can_reset`); this is for a driver that comes to judge otherwise.
        Err(Error::query("the server cannot reset the session"))
    }
}

/// A statement the server failed, as its outcome. `Err` is a lost session.
fn failed(error: mysql_async::Error) -> Result<StatementOutcome> {
    statement_failed(query_error(error), None)
}

/// The outcome of a statement with a parameter: the SQL editor has no
/// values for parameters. `position` is the parameter's, when known.
fn parameter(spelled: &str, position: Option<usize>) -> StatementOutcome {
    StatementOutcome::Error {
        error: Error::query(format!(
            "the statement has a parameter ({spelled}), which the SQL editor cannot fill in"
        )),
        position,
    }
}

/// The driver's `mysql_common` is the one named here: this fails to
/// compile, rather than read parameters another way than the driver does,
/// when the two versions drift apart.
const _: fn(mysql_common::params::Params) -> Params = |params| params;

/// The outcome of a statement in which the driver reads a `:name`
/// parameter. It sends the server a `?` in its place, and it reads by a
/// lexer of its own, which loses its place at a quote or a comment right
/// after a `-` or a `/` and then takes what is quoted for code
/// (`-':name'`). Such text does not run: only the text the guard read may
/// reach the server, and the driver closes the connection when it has no
/// value for a parameter, also for one the server did not count.
fn driver_parameter(text: &str) -> Option<StatementOutcome> {
    let Ok(parsed) = ParsedNamedParams::parse(text.as_bytes()) else {
        // A `:name` next to a `?`. Which it is cannot be told from here.
        return Some(parameter("? and :name", None));
    };
    let name = parsed.params().first()?;
    let spelled = format!(":{}", String::from_utf8_lossy(name));
    // The name is a slice of `text`: where it starts, its colon before it.
    let colon = (name.as_ptr() as usize)
        .checked_sub(text.as_ptr() as usize)
        .and_then(|start| start.checked_sub(1))
        .filter(|colon| text.is_char_boundary(*colon));
    let position = colon.map(|colon| text[..colon].chars().count() + 1);
    // To the server (and to `sql::tokenize`) it is no parameter when it
    // stands in a string, a quoted name or a comment.
    let misread = colon.is_some_and(|colon| {
        crate::sql::tokenize(Dialect::MySql, text)
            .iter()
            .find(|token| token.range.contains(&colon))
            .is_some_and(|token| {
                matches!(
                    token.kind,
                    crate::sql::TokenKind::String
                        | crate::sql::TokenKind::QuotedIdentifier
                        | crate::sql::TokenKind::Comment
                )
            })
    });
    if !misread {
        return Some(parameter(&spelled, position));
    }
    Some(StatementOutcome::Error {
        error: Error::query(format!(
            "the MySQL driver reads {spelled} here as a parameter; put a space after the - or / \
             that comes before the quote or comment"
        )),
        position,
    })
}

/// Whether the statement's answer carries a row count. The server reports
/// 0 for a statement without one (`SET`, `DO`), which is not "0 rows".
fn counts_rows(text: &str) -> bool {
    crate::sql::words(Dialect::MySql, text)
        .first()
        .is_some_and(|word| matches!(word.as_str(), "INSERT" | "UPDATE" | "DELETE" | "REPLACE"))
}

/// An agent's values as the driver takes a statement's: each as its own
/// type, by the protocol that binds them. A value is never written into
/// the statement.
pub(super) fn bound(params: &[Param]) -> Params {
    use mysql_async::Value as MySql;
    if params.is_empty() {
        return Params::Empty;
    }
    let value = |param: &Param| match param {
        Param::Null => MySql::NULL,
        Param::Bool(flag) => MySql::Int(i64::from(*flag)),
        Param::Int(number) => MySql::Int(*number),
        Param::Float(number) => MySql::Double(*number),
        Param::Text(text) => MySql::Bytes(text.as_bytes().to_vec()),
    };
    Params::Positional(params.iter().map(value).collect())
}

/// The outcome of an agent's statement in which the driver reads a
/// `:name`. One written as code did not pass the check, so this one stands
/// where the driver's own lexer lost its place: see `driver_parameter`.
fn driver_misread() -> StatementOutcome {
    StatementOutcome::Error {
        error: Error::query(
            "the MySQL driver reads a :name in a string, a quoted name or a comment of this \
             statement as a parameter; put a space after the - or / that comes before it",
        ),
        position: None,
    }
}

/// Runs one statement of a script. A statement's failure is its outcome;
/// whether it also ended the transaction, the next check asks. `Err` is a
/// lost session.
///
/// `params` are an agent's values. The SQL editor has none, and its
/// statement with a placeholder does not run, as before.
async fn run_statement(
    conn: &mut mysql_async::Conn,
    text: &str,
    limit: usize,
    params: Option<&[Param]>,
) -> Result<StatementOutcome> {
    if let Some(outcome) = driver_parameter(text) {
        return Ok(match params {
            None => outcome,
            Some(_) => driver_misread(),
        });
    }
    // Prepare first: the server refuses a second statement hidden in one
    // piece, and what the prepared protocol lacks. A prepare error is the
    // outcome; the text never runs another way.
    let statement = match conn.prep(text).await {
        Ok(statement) => statement,
        Err(error) => return failed(error),
    };
    let counted = usize::from(statement.num_params());
    let values = match params {
        // The driver closes the connection when a statement runs without
        // values for its parameters, so such a statement does not run.
        None if counted > 0 => return Ok(parameter("?", None)),
        None => Params::Empty,
        // The server's own count, which is the one that holds.
        Some(params) if counted != params.len() => {
            let error = count_mismatch(counted, params.len());
            let position = None;
            return Ok(StatementOutcome::Error { error, position });
        }
        Some(params) => bound(params),
    };
    let mut result = match conn.exec_iter(&statement, values).await {
        Ok(result) => result,
        Err(error) => return failed(error),
    };
    // From the result, not the prepared statement: EXPLAIN prepares
    // without columns.
    let columns = column_metas(result.columns_ref());
    if columns.is_empty() {
        let affected = result.affected_rows();
        let warnings = result.warnings();
        if let Err(error) = result.drop_result().await {
            return failed(error);
        }
        return Ok(StatementOutcome::Done {
            affected: counts_rows(text).then_some(affected),
            warnings,
        });
    }
    // sql_select_limit does not bound every statement (SHOW, a SELECT with
    // its own LIMIT): keep limit + 1 rows and read the rest off the wire.
    let mut rows = Vec::new();
    loop {
        match result.next().await {
            Ok(Some(row)) if rows.len() <= limit => rows.push(row_values(row, &columns)),
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(error) => return failed(error),
        }
    }
    // A statement with several result sets shows its first.
    if let Err(error) = result.drop_result().await {
        return failed(error);
    }
    let truncated = rows.len() > limit;
    rows.truncate(limit);
    Ok(StatementOutcome::Rows {
        columns,
        rows,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use mysql_async::Opts;

    use super::super::cancel;
    use super::super::tests::{session, test_url};
    use super::*;
    use crate::adapter::Adapter;

    /// A session on the test server, which is MySQL.
    fn on_mysql(access: Access) -> SessionKind {
        SessionKind {
            access,
            mariadb: false,
        }
    }

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
    fn only_data_changing_statements_report_a_count() {
        for text in [
            "INSERT INTO t VALUES (1)",
            "update t SET n = 1",
            "-- note\nDELETE FROM t",
            "/* note */ REPLACE INTO t VALUES (1)",
        ] {
            assert!(counts_rows(text), "{text}");
        }
        for text in [
            "SET @a = 1",
            "DO 1",
            "CREATE TABLE t (n int)",
            "SELECT 1",
            "",
        ] {
            assert!(!counts_rows(text), "{text}");
        }
    }

    #[test]
    fn text_the_driver_would_rewrite_is_a_parameter_error() {
        // The message, and the 1-based position of the colon.
        let read = |text: &str| match driver_parameter(text) {
            Some(StatementOutcome::Error { error, position }) => {
                Some((error.to_string(), position))
            }
            Some(other) => panic!("{other:?}"),
            None => None,
        };
        let parameter = |spelled: &str, position: usize| {
            let message = format!(
                "the statement has a parameter ({spelled}), which the SQL editor cannot fill in"
            );
            Some((message, Some(position)))
        };
        assert_eq!(read("SELECT :id"), parameter(":id", 8));
        assert_eq!(
            read("SELECT 'é' FROM t WHERE a = :a AND b = :b_2"),
            parameter(":a", 29)
        );
        // Next to a `?` the driver does not say where.
        assert_eq!(
            read("SELECT ?, :id"),
            Some((
                "the statement has a parameter (? and :name), which the SQL editor cannot fill in"
                    .to_owned(),
                None
            ))
        );
        // The driver's lexer loses its place after a `-` or a `/`, and
        // would change a string, a name or a comment. The message says it
        // is the driver's reading, and what to do.
        let misread = |spelled: &str, position: usize| {
            let message = format!(
                "the MySQL driver reads {spelled} here as a parameter; put a space after the - \
                 or / that comes before the quote or comment"
            );
            Some((message, Some(position)))
        };
        assert_eq!(read("SELECT 1 -':abc'"), misread(":abc", 12));
        assert_eq!(read("SELECT 1 /`:abc` FROM t"), misread(":abc", 12));
        assert_eq!(
            read("SELECT '{\"a\":1}' /'{\"a\":true}'"),
            misread(":true", 24)
        );
        assert_eq!(read("SELECT 1 -# :abc\n + 2"), misread(":abc", 13));
        // What it leaves alone runs, a `?` included: the server counts
        // those.
        for text in [
            "SELECT ?",
            "SELECT ':id', \":id\", `:id` FROM t -- :id\n/* :id */ # :id",
            "SELECT @a := 1",
            "SELECT '{\"a\":true}', TIME '12:30:00'",
            "SELECT 1 - ':abc'",
            "",
        ] {
            assert_eq!(read(text), None, "{text}");
        }
    }

    #[test]
    fn a_session_reset_needs_a_recent_server() {
        // The driver's own rule for `Conn::reset`.
        for (mariadb, version, resets) in [
            (false, (5, 6, 51), false),
            (false, (5, 7, 2), false),
            (false, (5, 7, 3), true),
            (false, (5, 7, 44), true),
            (false, (8, 0, 0), true),
            (false, (8, 4, 3), true),
            (false, (9, 1, 0), true),
            (true, (5, 5, 68), false),
            (true, (10, 1, 48), false),
            (true, (10, 2, 3), false),
            (true, (10, 2, 4), true),
            (true, (10, 11, 6), true),
            (true, (11, 4, 2), true),
        ] {
            assert_eq!(can_reset(mariadb, version), resets, "{mariadb} {version:?}");
        }
    }

    #[test]
    fn a_session_stands_where_its_setting_and_status_say() {
        let autocommit = StatusFlags::SERVER_STATUS_AUTOCOMMIT;
        let in_transaction = autocommit | StatusFlags::SERVER_STATUS_IN_TRANS;
        let read_only = in_transaction | StatusFlags::SERVER_STATUS_IN_TRANS_READONLY;
        assert_eq!(stands(Some(1), read_only), Ok(Standing::Inside));
        assert_eq!(stands(Some(2), read_only), Ok(Standing::Inside));
        // No transaction: a statement ended it.
        assert_eq!(stands(Some(1), autocommit), Ok(Standing::Outside));
        assert_eq!(stands(Some(1), StatusFlags::empty()), Ok(Standing::Outside));
        // Only an explicit 0 is read-write, whatever the transaction.
        for status in [read_only, in_transaction, autocommit] {
            assert_eq!(stands(Some(0), status), Ok(Standing::Left));
        }
        // A transaction without the read-only mark is not the script's:
        // a missing mark is never taken for a read-only transaction.
        assert_eq!(stands(Some(1), in_transaction), Ok(Standing::Left));
        assert_eq!(
            stands(Some(1), StatusFlags::SERVER_STATUS_IN_TRANS_READONLY),
            Ok(Standing::Outside)
        );
        // No answer is not a yes.
        assert!(matches!(stands(None, read_only), Err(Error::Query { .. })));
    }

    /// A server (or a proxy in front of one) that does not mark the
    /// transaction read-only cannot run scripts: the open fails, and that
    /// closes the session.
    #[test]
    fn a_transaction_the_server_does_not_mark_read_only_fails_the_open() {
        let autocommit = StatusFlags::SERVER_STATUS_AUTOCOMMIT;
        let in_transaction = autocommit | StatusFlags::SERVER_STATUS_IN_TRANS;
        let read_only = in_transaction | StatusFlags::SERVER_STATUS_IN_TRANS_READONLY;
        assert_eq!(started(read_only), Ok(()));
        assert_eq!(
            started(in_transaction),
            Err(Error::query(
                "the server did not mark the transaction read-only"
            ))
        );
        for status in [autocommit, StatusFlags::empty()] {
            assert_eq!(
                started(status),
                Err(Error::query("the server did not start a transaction"))
            );
        }
        let closed = cannot_start(ScriptMode::ReadOnly, &started(in_transaction).unwrap_err());
        assert!(closed.is_connection_lost());
        assert_eq!(
            closed.to_string(),
            "the connection was lost: could not start the read-only transaction: the server did \
             not mark the transaction read-only"
        );
    }

    /// One test at a time uses the `probe` table: creating it twice at once
    /// can fail, and one test's TRUNCATE would hide another's stray row.
    pub(super) static PROBE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// A writable connection, outside the adapter.
    pub(super) async fn admin(url: &str) -> mysql_async::Conn {
        let opts = Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        mysql_async::Conn::new(opts).await.unwrap()
    }

    /// A writable connection with an empty `probe` table, and the table's
    /// lock, held until the test ends.
    pub(super) async fn probe(
        url: &str,
    ) -> (mysql_async::Conn, tokio::sync::MutexGuard<'static, ()>) {
        let turn = PROBE.lock().await;
        let mut admin = admin(url).await;
        admin
            .query_drop("CREATE TABLE IF NOT EXISTS probe (n int)")
            .await
            .unwrap();
        admin.query_drop("TRUNCATE probe").await.unwrap();
        (admin, turn)
    }

    /// The rows in the `probe` table.
    pub(super) async fn probe_rows(admin: &mut mysql_async::Conn) -> i64 {
        admin
            .query_first("SELECT count(*) FROM probe")
            .await
            .unwrap()
            .unwrap()
    }

    /// The session settings a script could change and the close puts back.
    const SETTINGS: [&str; 10] = [
        "transaction_read_only",
        "sql_select_limit",
        "sql_mode",
        "time_zone",
        "autocommit",
        "character_set_client",
        "character_set_connection",
        "character_set_results",
        "collation_connection",
        "completion_type",
    ];

    /// Everything of a session that a script could change and the close
    /// puts back, by name: its settings, a user variable, the default
    /// database, and whether a transaction is open.
    async fn settings(conn: &Conn) -> Vec<(&'static str, Option<String>)> {
        let mut conn = conn.conn.lock().await;
        let asked: Vec<String> = SETTINGS
            .iter()
            .map(|name| format!("@@session.{name}"))
            .collect();
        let row: mysql_async::Row = conn
            .query_first(format!(
                "SELECT {}, @tabletist_left_over, DATABASE() LIMIT 1",
                asked.join(", ")
            ))
            .await
            .unwrap()
            .unwrap();
        let open = status(&conn).contains(StatusFlags::SERVER_STATUS_IN_TRANS);
        let values = row.unwrap().into_iter().map(|value| match value {
            mysql_async::Value::NULL => None,
            mysql_async::Value::Bytes(bytes) => Some(String::from_utf8(bytes).unwrap()),
            other => Some(format!("{other:?}")),
        });
        SETTINGS
            .into_iter()
            .chain(["@tabletist_left_over", "DATABASE()"])
            .zip(values)
            .chain([("in a transaction", Some(open.to_string()))])
            .collect()
    }

    /// One of `settings` by its name.
    fn value_of<'a>(settings: &'a [(&'static str, Option<String>)], name: &str) -> Option<&'a str> {
        let (_, value) = settings
            .iter()
            .find(|(setting, _)| *setting == name)
            .unwrap_or_else(|| panic!("no setting {name}"));
        value.as_deref()
    }

    /// Waits until a statement holding `marker` runs on the server.
    async fn runs_on_the_server(admin: &mut mysql_async::Conn, marker: &str) {
        let running = async {
            loop {
                let found: Option<i64> = admin
                    .exec_first(
                        "SELECT count(*) FROM information_schema.processlist \
                         WHERE id <> CONNECTION_ID() AND info LIKE ?",
                        (format!("%{marker}%"),),
                    )
                    .await
                    .unwrap();
                if found.unwrap_or(0) > 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        };
        tokio::time::timeout(Duration::from_secs(10), running)
            .await
            .expect("the statement never ran");
    }

    #[test]
    fn an_agents_values_are_bound_each_as_its_own_type() {
        use mysql_async::Value as MySql;
        assert!(matches!(bound(&[]), Params::Empty));
        let Params::Positional(values) = bound(&[
            Param::Null,
            Param::Bool(true),
            Param::Int(i64::MIN),
            Param::Float(1.5),
            Param::Text("it's".into()),
        ]) else {
            panic!("values by position");
        };
        assert_eq!(
            values,
            [
                MySql::NULL,
                MySql::Int(1),
                MySql::Int(i64::MIN),
                MySql::Double(1.5),
                // As it is: nothing is quoted, because nothing is written
                // into SQL.
                MySql::Bytes(b"it's".to_vec()),
            ]
        );
    }

    #[test]
    fn the_time_limit_is_spelled_for_mysql_and_for_mariadb() {
        assert_eq!(
            time_limits(10_000),
            [
                "SET SESSION max_execution_time = 10000",
                "SET SESSION max_statement_time = 10.000",
            ]
        );
        // MariaDB counts in seconds, and a millisecond is still some time:
        // 0 would be no limit.
        assert_eq!(
            time_limits(1),
            [
                "SET SESSION max_execution_time = 1",
                "SET SESSION max_statement_time = 0.001",
            ]
        );
        assert_eq!(
            time_limits(1_250)[1],
            "SET SESSION max_statement_time = 1.250"
        );
    }

    #[tokio::test]
    async fn the_servers_own_count_of_placeholders_is_the_one_that_holds() {
        let Some(url) = test_url() else {
            return;
        };
        let session = session(&url).await;
        let mut conn = session.conn.lock().await;
        for (text, sent, counted) in [
            ("SELECT ?, ?", vec![Param::Int(1)], 2),
            ("SELECT 1", vec![Param::Int(1)], 0),
        ] {
            let ran = run_statement(&mut conn, text, 10, Some(&sent)).await;
            let error = count_mismatch(counted, sent.len());
            let position = None;
            assert_eq!(
                ran,
                Ok(StatementOutcome::Error { error, position }),
                "{text}"
            );
        }
        // The SQL editor sends no values, and is told what it always was.
        let ran = run_statement(&mut conn, "SELECT ?", 10, None).await;
        assert_eq!(ran, Ok(parameter("?", None)));
        // A `:name` only the driver reads, beside a real placeholder: said
        // to an agent in words that are true of its statement.
        let text = "SELECT ? -':id'";
        let ran = run_statement(&mut conn, text, 10, Some(&[Param::Int(1)])).await;
        assert_eq!(ran, Ok(driver_misread()));
        let ran = run_statement(&mut conn, text, 10, None).await;
        assert_eq!(ran, Ok(parameter("? and :name", None)));
    }

    /// The open of a run of the SQL editor's on a read-only session, with a
    /// limit of 10 rows, and the close's check and rollback, as the server
    /// gets them: what a test proxy swaps.
    const OPEN: &str = "START TRANSACTION READ ONLY; SET SESSION sql_select_limit = 11; \
                        SELECT @@session.transaction_read_only LIMIT 1";
    const CLOSE: &str = "DO 0; SELECT @@session.transaction_read_only LIMIT 1; \
                         ROLLBACK AND NO CHAIN NO RELEASE";

    #[test]
    fn the_open_and_the_close_are_one_message_each() {
        let names = on_mysql(Access::ReadOnly).read_only_names((8, 4, 3));
        assert_eq!(
            opening(Access::ReadOnly, 10, None, names[0]).join("; "),
            OPEN
        );
        assert_eq!(closing(names[0]).join("; "), CLOSE);
        // A writable session's fence comes first; an agent's time limit
        // just before the question, in the server's own form.
        let mariadb = SessionKind {
            access: Access::Writable,
            mariadb: true,
        };
        let [own, _] = mariadb.time_limits(1_500);
        let names = mariadb.read_only_names((10, 11, 6));
        assert_eq!(
            opening(Access::Writable, 5, Some(&own), names[0]),
            [
                "SET SESSION TRANSACTION READ ONLY",
                "START TRANSACTION READ ONLY",
                "SET SESSION sql_select_limit = 6",
                "SET SESSION max_statement_time = 1.500",
                "SELECT @@session.tx_read_only LIMIT 1",
            ]
        );
        assert_eq!(
            mariadb.read_only_names((11, 4, 2)),
            ["transaction_read_only", "tx_read_only"]
        );
    }

    /// The open fails as a whole when any of its steps fails, or when its
    /// answer does not say the session is inside a read-only transaction:
    /// no statement runs, the run closes the session, and the close has
    /// left it as it connected.
    #[tokio::test]
    async fn an_open_that_fails_on_any_step_runs_no_statement() {
        let Some(url) = test_url() else {
            return;
        };
        for (instead, said) in [
            (
                "START TRANSACTION READ ONLY; SET SESSION sql_select_limit = 'x'; \
                 SELECT @@session.transaction_read_only LIMIT 1",
                "Incorrect argument type",
            ),
            (
                "SET SESSION sql_select_limit = 11; \
                 SELECT @@session.transaction_read_only LIMIT 1",
                "the server did not start a transaction",
            ),
            (
                "START TRANSACTION READ WRITE; SET SESSION sql_select_limit = 11; \
                 SELECT @@session.transaction_read_only LIMIT 1",
                "the server did not mark the transaction read-only",
            ),
            (
                "START TRANSACTION READ ONLY; SET SESSION sql_select_limit = 11; SELECT 0",
                "the server did not say that the session is read-only",
            ),
        ] {
            let (conn, _) =
                super::super::tests::swapping_session(&url, Access::ReadOnly, (OPEN, instead))
                    .await;
            let connected = settings(&conn).await;
            let stop = StopFlag::new();
            let ran = past_the_refusal(&conn, &["SELECT 1"], &stop).await;
            assert!(
                matches!(&ran, Err(Error::ConnectionLost(message))
                    if message.starts_with("could not start the read-only transaction")
                        && message.contains(said)),
                "{ran:?}"
            );
            assert!(stop.is_finishing());
            assert_eq!(settings(&conn).await, connected, "{instead}");
        }
    }

    /// A cancel that lands on the open stops the run before its first
    /// statement: no results, and the session as it connected.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancel_on_the_open_runs_no_statement() {
        let Some(url) = test_url() else {
            return;
        };
        let mut admin = admin(&url).await;
        let slow = "START TRANSACTION READ ONLY; SET SESSION sql_select_limit = 11; \
                    SELECT 1 FROM DUAL WHERE SLEEP(46) = 0";
        let (conn, _) =
            super::super::tests::swapping_session(&url, Access::ReadOnly, (OPEN, slow)).await;
        let conn = std::sync::Arc::new(conn);
        let connected = settings(&conn).await;
        let running = {
            let conn = std::sync::Arc::clone(&conn);
            tokio::spawn(
                async move { past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new()).await },
            )
        };
        runs_on_the_server(&mut admin, "SLEEP(46)").await;
        cancel(&conn.opts, conn.id, &conn.server, &conn.spare)
            .await
            .unwrap();
        let outcome = running.await.unwrap().unwrap();
        assert!(outcome.stopped, "{outcome:?}");
        assert!(outcome.results.is_empty(), "{outcome:?}");
        assert_eq!(settings(&conn).await, connected);
    }

    /// A cancel that lands on the close's check runs the check and the
    /// rollback on their own; one that lands on the rollback runs it once
    /// more. Either way the run ends cleanly, the session as it connected.
    /// A real check or `ROLLBACK` is over too soon to cancel on cue, so the
    /// proxy sends the server a slow statement in the step's place; the
    /// count shows the steps that ran again.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancel_on_the_close_runs_the_cancelled_step_again() {
        use std::sync::atomic::Ordering;
        let Some(url) = test_url() else {
            return;
        };
        let mut admin = admin(&url).await;
        let slow_check = "DO 0; SELECT 1 FROM DUAL WHERE SLEEP(47) = 0; \
                          ROLLBACK AND NO CHAIN NO RELEASE";
        let slow_rollback = "DO 0; SELECT @@session.transaction_read_only LIMIT 1; \
                             SELECT 1 FROM DUAL WHERE SLEEP(48) = 0";
        // The run's six, and the steps that ran again: the check and the
        // rollback, or the rollback.
        for (slow, sleep, asked) in [
            (slow_check, "SLEEP(47)", 8),
            (slow_rollback, "SLEEP(48)", 7),
        ] {
            let (conn, count) =
                super::super::tests::swapping_session(&url, Access::ReadOnly, (CLOSE, slow)).await;
            let conn = std::sync::Arc::new(conn);
            let connected = settings(&conn).await;
            let before = count.load(Ordering::SeqCst);
            let running = {
                let conn = std::sync::Arc::clone(&conn);
                tokio::spawn(async move {
                    past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new()).await
                })
            };
            runs_on_the_server(&mut admin, sleep).await;
            cancel(&conn.opts, conn.id, &conn.server, &conn.spare)
                .await
                .unwrap();
            let outcome = running.await.unwrap().unwrap();
            // Counted once the reset after the outcome is done.
            drop(conn.conn.lock().await);
            assert_eq!(count.load(Ordering::SeqCst) - before, asked, "{sleep}");
            assert!(!outcome.stopped, "{outcome:?}");
            assert!(
                matches!(
                    outcome.results.as_slice(),
                    [StatementResult {
                        outcome: StatementOutcome::Rows { .. },
                        ..
                    }]
                ),
                "{outcome:?}"
            );
            assert_eq!(settings(&conn).await, connected, "{sleep}");
        }
    }

    /// The settings a read-only session gets back after the reset that
    /// ends a run, as the close sends them.
    fn settings_after_the_reset() -> &'static str {
        let settings = super::super::session_settings(Access::ReadOnly);
        settings.trim_end_matches("; ").to_owned().leak()
    }

    /// The outcome is back before the reset and the settings: a cancel that
    /// lands on the settings, slowed down here by the proxy, finds the run
    /// already answered. The settings run once more, and the session is as
    /// it connected.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancel_on_the_settings_after_the_outcome_runs_them_again() {
        use std::sync::atomic::Ordering;
        let Some(url) = test_url() else {
            return;
        };
        let mut admin = admin(&url).await;
        let slow = "SELECT 1 FROM DUAL WHERE SLEEP(49) = 0";
        let (conn, count) = super::super::tests::swapping_session(
            &url,
            Access::ReadOnly,
            (settings_after_the_reset(), slow),
        )
        .await;
        let connected = settings(&conn).await;
        let before = count.load(Ordering::SeqCst);
        let outcome = past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new())
            .await
            .unwrap();
        assert!(!outcome.stopped, "{outcome:?}");
        assert!(
            matches!(
                outcome.results.as_slice(),
                [StatementResult {
                    outcome: StatementOutcome::Rows { .. },
                    ..
                }]
            ),
            "{outcome:?}"
        );
        runs_on_the_server(&mut admin, "SLEEP(49)").await;
        cancel(&conn.opts, conn.id, &conn.server, &conn.spare)
            .await
            .unwrap();
        // The run's six, and the settings again.
        drop(conn.conn.lock().await);
        assert_eq!(count.load(Ordering::SeqCst) - before, 7);
        assert_eq!(settings(&conn).await, connected);
    }

    /// Settings that fail after the outcome is back close the session: the
    /// outcome stands, and whatever uses the session next is told it is
    /// lost, without a word to the server.
    #[tokio::test]
    async fn settings_that_fail_after_the_outcome_close_the_session() {
        use std::sync::atomic::Ordering;
        let Some(url) = test_url() else {
            return;
        };
        let refused = "SET SESSION tabletist_no_such_setting = 1";
        let (conn, count) = super::super::tests::swapping_session(
            &url,
            Access::ReadOnly,
            (settings_after_the_reset(), refused),
        )
        .await;
        let id = conn.conn.lock().await.id();
        let outcome = past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new())
            .await
            .unwrap();
        assert!(
            matches!(
                outcome.results.as_slice(),
                [StatementResult {
                    outcome: StatementOutcome::Rows { .. },
                    ..
                }]
            ),
            "{outcome:?}"
        );
        let lost = past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new()).await;
        let asked = count.load(Ordering::SeqCst);
        assert!(
            matches!(&lost, Err(error) if error.is_connection_lost()),
            "{lost:?}"
        );
        let listed = conn.list_schemas().await;
        assert!(
            matches!(&listed, Err(error) if error.is_connection_lost()),
            "{listed:?}"
        );
        let read = conn
            .agent_read(
                &crate::agent::check_read(Dialect::MySql, "SELECT 1", &[]).unwrap(),
                Limits {
                    rows: 5,
                    timeout: Duration::from_secs(10),
                },
                &crate::agent::Sources::default(),
                &StopFlag::new(),
            )
            .await;
        assert!(
            matches!(&read, Err(error) if error.is_connection_lost()),
            "{read:?}"
        );
        assert_eq!(count.load(Ordering::SeqCst), asked);
        // The connection is hung up, not kept for the session's close.
        let mut admin = admin(&url).await;
        let mut left = true;
        for _ in 0..50 {
            let found: Option<u64> = admin
                .exec_first(
                    "SELECT ID FROM information_schema.PROCESSLIST WHERE ID = ?",
                    (id,),
                )
                .await
                .unwrap();
            left = found.is_some();
            if !left {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(!left, "connection {id} is still on the server");
    }

    /// A setting the server knows only under its other name, or in its
    /// other form: the open and the close run the steps from there on their
    /// own. Here a MySQL server is taken for an old MariaDB, whose names it
    /// does not know. The run works, with the time limit set.
    #[tokio::test]
    async fn a_setting_the_server_does_not_know_is_asked_for_under_its_other_name() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let connected = settings(&conn).await;
        let kind = SessionKind {
            access: Access::ReadOnly,
            mariadb: true,
        };
        let asked = "SELECT @@session.max_execution_time".to_owned();
        let sent = Sent {
            params: &[],
            timeout: Duration::from_secs(7),
        };
        for sent in [Some(sent), None] {
            let stop = StopFlag::new();
            let mut session = conn.conn.lock().await;
            let (outcome, _, unfinished) = read_only(
                &mut session,
                std::slice::from_ref(&asked),
                5,
                kind,
                &stop,
                sent,
                None,
            )
            .await
            .unwrap();
            unfinished.finish(&mut session).await.unwrap();
            drop(session);
            let [result] = outcome.results.as_slice() else {
                panic!("{outcome:?}");
            };
            let StatementOutcome::Rows { rows, .. } = &result.outcome else {
                panic!("{result:?}");
            };
            let limit = if sent.is_some() { 7_000 } else { 0 };
            assert_eq!(rows[0][0], crate::Value::Int(limit));
            assert_eq!(settings(&conn).await, connected);
        }
    }

    /// The check refuses each of these by name. Past it, what they leave on
    /// the session is gone with the reset that ends the run, and so are
    /// the run's own row limit and time limit.
    #[tokio::test]
    async fn what_an_agents_statement_leaves_on_the_session_is_gone_after_the_run() {
        let Some(url) = test_url() else {
            return;
        };
        let session = session(&url).await;
        let mut conn = session.conn.lock().await;
        let left = "SELECT LAST_INSERT_ID(99), GET_LOCK('tabletist_agent_lock', 0), \
                    @tabletist_agent := 7";
        let sent = Sent {
            params: &[],
            timeout: Duration::from_secs(10),
        };
        let stop = StopFlag::new();
        let (outcome, _, unfinished) = read_only(
            &mut conn,
            &[left.to_owned()],
            5,
            on_mysql(Access::ReadOnly),
            &stop,
            Some(sent),
            None,
        )
        .await
        .unwrap();
        unfinished.finish(&mut conn).await.unwrap();
        let [result] = outcome.results.as_slice() else {
            panic!("{outcome:?}");
        };
        let StatementOutcome::Rows { rows, .. } = &result.outcome else {
            panic!("{result:?}");
        };
        // It ran, and took the lock.
        assert_eq!(rows[0][1], crate::Value::Int(1));
        let after: (u64, Option<u64>, Option<i64>, u64, u64) = conn
            .query_first(
                "SELECT LAST_INSERT_ID(), IS_USED_LOCK('tabletist_agent_lock'), \
                 @tabletist_agent, @@session.max_execution_time, \
                 @@session.sql_select_limit = @@global.sql_select_limit",
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after, (0, None, None, 0, 1));
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

    const INSERT: &str = "INSERT INTO probe VALUES (1)";

    /// A SELECT's outcome is back after four round trips: the open, the
    /// prepare, the execute, the close's check with its rollback. The
    /// reset and the settings follow before anything else uses the
    /// session: six in all. The first statement needs no check of its own:
    /// the open asked. Each statement after it adds the check, the prepare
    /// and the execute. A writable session's fence goes in the open's
    /// message.
    ///
    /// On one thread, so the task that resets the session cannot run
    /// before the count is read: a test that waits for nothing yields
    /// nothing.
    #[tokio::test(flavor = "current_thread")]
    async fn a_select_waits_for_four_answers() {
        use std::sync::atomic::Ordering;
        let Some(url) = test_url() else {
            return;
        };
        for access in [Access::ReadOnly, Access::Writable] {
            let (conn, asked) = super::super::tests::counted_session(&url, access).await;
            let connected = settings(&conn).await;
            for (script, waited, all) in [
                (&["SELECT 1"][..], 4, 6),
                (&["SELECT 1", "SELECT 2"][..], 7, 9),
            ] {
                let before = asked.load(Ordering::SeqCst);
                let outcome = past_the_refusal(&conn, script, &StopFlag::new())
                    .await
                    .unwrap();
                let answered = asked.load(Ordering::SeqCst) - before;
                assert_eq!(outcome.results.len(), script.len(), "{outcome:?}");
                assert!(
                    outcome
                        .results
                        .iter()
                        .all(|result| matches!(result.outcome, StatementOutcome::Rows { .. })),
                    "{outcome:?}"
                );
                assert_eq!(answered, waited, "{access:?} {script:?}");
                // The next use waits for the reset and the settings.
                let _ = conn.conn.lock().await;
                let reset = asked.load(Ordering::SeqCst) - before;
                assert_eq!(reset, all, "{access:?} {script:?}");
                assert_eq!(settings(&conn).await, connected, "{access:?} {script:?}");
            }
        }
    }

    /// Statements the refusal stops long before they get here. Run past
    /// it, they make the session read-write; the statement after them must
    /// not run, and the run must fail even when nothing follows them, so
    /// that the session is closed.
    #[tokio::test]
    async fn a_script_that_left_read_only_runs_nothing_more_and_fails() {
        let Some(url) = test_url() else {
            return;
        };
        let (mut admin, _turn) = probe(&url).await;
        for script in [
            &["COMMIT", "SET SESSION TRANSACTION READ WRITE", INSERT][..],
            &["SET SESSION TRANSACTION READ WRITE", INSERT][..],
            &["SET @@session.transaction_read_only = 0", INSERT][..],
            &["SET @@session.transaction_read_only = 0", "COMMIT", INSERT][..],
            // Last in the script, nothing would run after it.
            &["SELECT 1", "SET SESSION TRANSACTION READ WRITE"][..],
        ] {
            // A session of its own, dropped with the loop's turn: whatever
            // a failing guard let through cannot reach another test.
            let conn = session(&url).await;
            let connected = settings(&conn).await;
            let stop = StopFlag::new();
            let ran = past_the_refusal(&conn, script, &stop).await;
            assert_eq!(ran, Err(Error::LeftReadOnly), "{script:?}");
            assert!(stop.is_finishing(), "{script:?}");
            assert_eq!(probe_rows(&mut admin).await, 0, "{script:?}");
            // The backend closes the session on that error. The close has
            // made it read-only again all the same.
            assert_eq!(settings(&conn).await, connected, "{script:?}");
        }
    }

    /// The refusal keeps these from the driver. Past it, the server still
    /// refuses the write: every statement runs inside a read-only
    /// transaction, also after one that ended the script's, and there the
    /// server lets nothing change the transaction's mode.
    #[tokio::test]
    async fn statements_past_the_refusal_cannot_write() {
        let Some(url) = test_url() else {
            return;
        };
        let (mut admin, _turn) = probe(&url).await;
        // The script, and the SQLSTATE its last result fails with.
        for (script, failure) in [
            (&[INSERT][..], "25006"),
            // Outside the script's transaction the session is read-only.
            (&["COMMIT", INSERT][..], "25006"),
            (&["ROLLBACK", "SELECT 1", INSERT][..], "25006"),
            // This sets the next transaction's mode only, which the
            // session's setting does not show: on its own, the INSERT after
            // it would write. Inside a transaction the server refuses it.
            (
                &["COMMIT", "SET TRANSACTION READ WRITE", INSERT][..],
                "25001",
            ),
            (&["SET TRANSACTION READ WRITE", INSERT][..], "25001"),
            (
                &["COMMIT", "SET @@transaction_read_only = 0", INSERT][..],
                "25001",
            ),
        ] {
            let conn = session(&url).await;
            let connected = settings(&conn).await;
            let stop = StopFlag::new();
            let outcome = past_the_refusal(&conn, script, &stop).await.unwrap();
            let last = outcome.results.len() - 1;
            assert_eq!(
                code(&outcome.results[last].outcome),
                Some(failure),
                "{script:?}: {outcome:?}"
            );
            assert!(
                outcome.results[..last].iter().all(|result| matches!(
                    result.outcome,
                    StatementOutcome::Rows { .. } | StatementOutcome::Done { .. }
                )),
                "{script:?}: {outcome:?}"
            );
            // The write is the statement that failed, or never ran.
            assert!(
                script[last] == INSERT || script[last + 1] == INSERT,
                "{script:?}: {outcome:?}"
            );
            assert_eq!(probe_rows(&mut admin).await, 0, "{script:?}");
            assert_eq!(settings(&conn).await, connected, "{script:?}");
        }
    }

    /// The check before a statement, on its own: a statement that ended
    /// the script's transaction is followed by a new one, and a read-write
    /// transaction is not the script's.
    #[tokio::test]
    async fn the_check_puts_the_session_back_inside_a_read_only_transaction() {
        let Some(url) = test_url() else {
            return;
        };
        let (mut admin, _turn) = probe(&url).await;
        let conn = session(&url).await;
        let connected = settings(&conn).await;
        {
            let mut conn = conn.conn.lock().await;
            assert_eq!(
                standing(&mut conn, READ_ONLY_SETTINGS).await,
                Ok(Standing::Outside)
            );
            // The server marks the transaction read-only, or this fails.
            open(&mut conn, 10, on_mysql(Access::ReadOnly), None)
                .await
                .unwrap();
            assert_eq!(
                standing(&mut conn, READ_ONLY_SETTINGS).await,
                Ok(Standing::Inside)
            );
            assert_eq!(
                ready(&mut conn, READ_ONLY_SETTINGS).await,
                Ok(Standing::Inside)
            );

            // What a statement that commits would have done, and one that
            // makes the next transaction read-write: the session's setting
            // does not show that, and a statement on its own would write.
            conn.query_drop("COMMIT").await.unwrap();
            conn.query_drop("SET TRANSACTION READ WRITE").await.unwrap();
            assert_eq!(
                standing(&mut conn, READ_ONLY_SETTINGS).await,
                Ok(Standing::Outside)
            );
            assert_eq!(
                ready(&mut conn, READ_ONLY_SETTINGS).await,
                Ok(Standing::Inside)
            );
            assert_eq!(
                standing(&mut conn, READ_ONLY_SETTINGS).await,
                Ok(Standing::Inside)
            );
            let written = conn.query_drop(INSERT).await.map_err(query_error);
            assert!(
                matches!(&written, Err(Error::Query { code: Some(code), .. }) if code == "25006"),
                "{written:?}"
            );
            assert_eq!(probe_rows(&mut admin).await, 0);

            // A transaction of another kind is not the script's.
            conn.query_drop("START TRANSACTION READ WRITE")
                .await
                .unwrap();
            assert_eq!(
                standing(&mut conn, READ_ONLY_SETTINGS).await,
                Ok(Standing::Left)
            );
            assert_eq!(
                ready(&mut conn, READ_ONLY_SETTINGS).await,
                Ok(Standing::Left)
            );
            assert_eq!(
                close(&mut conn, Ended::Unconfirmed, on_mysql(Access::ReadOnly)).await,
                Err(Error::LeftReadOnly)
            );
            assert_eq!(
                standing(&mut conn, READ_ONLY_SETTINGS).await,
                Ok(Standing::Outside)
            );

            // A script's own sql_select_limit does not blind the check.
            open(&mut conn, 10, on_mysql(Access::ReadOnly), None)
                .await
                .unwrap();
            conn.query_drop("SET SESSION sql_select_limit = 0")
                .await
                .unwrap();
            assert_eq!(
                standing(&mut conn, READ_ONLY_SETTINGS).await,
                Ok(Standing::Inside)
            );
            conn.query_drop("SET SESSION TRANSACTION READ WRITE")
                .await
                .unwrap();
            assert_eq!(
                standing(&mut conn, READ_ONLY_SETTINGS).await,
                Ok(Standing::Left)
            );
            assert_eq!(
                close(&mut conn, Ended::Unconfirmed, on_mysql(Access::ReadOnly)).await,
                Err(Error::LeftReadOnly)
            );
        }
        assert_eq!(settings(&conn).await, connected);
        // The session was not closed, and it works.
        let ran = past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new()).await;
        assert_eq!(ran.unwrap().results.len(), 1);
    }

    /// A stop between statements ends the run without a statement failing,
    /// so the close still asks where the session stands. Here the statement
    /// before the stop made the session read-write.
    #[tokio::test]
    async fn a_stop_after_the_session_left_read_only_still_fails_the_run() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let connected = settings(&conn).await;
        {
            let mut conn = conn.conn.lock().await;
            open(&mut conn, 10, on_mysql(Access::ReadOnly), None)
                .await
                .unwrap();
            // What the first statement of the script would have done.
            conn.query_drop("SET SESSION TRANSACTION READ WRITE")
                .await
                .unwrap();
            // The stop lands before the second.
            let stop = StopFlag::new();
            stop.stop();
            let mut outcome = ScriptOutcome::default();
            let texts = ["SELECT 1".to_owned()];
            let ended = statements(
                &mut conn,
                &texts,
                10,
                READ_ONLY_SETTINGS,
                &stop,
                None,
                &mut outcome,
            )
            .await;
            assert_eq!(ended, Ok(Ended::Unconfirmed));
            assert_eq!(outcome.results.len(), 1);
            assert_eq!(outcome.results[0].outcome, StatementOutcome::Cancelled);
            assert!(outcome.stopped);
            assert_eq!(
                close(&mut conn, Ended::Unconfirmed, on_mysql(Access::ReadOnly)).await,
                Err(Error::LeftReadOnly)
            );
        }
        assert_eq!(settings(&conn).await, connected);
    }

    /// After every way a run can end, the session is as it connected:
    /// read-only, in no transaction, with the connect-time sql_mode and
    /// character set, the default sql_select_limit, and nothing a script
    /// set.
    #[tokio::test]
    async fn every_run_leaves_the_session_as_it_connected() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let connected = settings(&conn).await;
        let at_connect = |name: &str| value_of(&connected, name);
        assert_eq!(at_connect("transaction_read_only"), Some("1"));
        assert_eq!(
            at_connect("sql_select_limit"),
            Some(u64::MAX.to_string().as_str())
        );
        assert!(!at_connect("sql_mode").unwrap().contains("ANSI_QUOTES"));
        assert_eq!(at_connect("character_set_client"), Some("utf8mb4"));
        assert_eq!(
            at_connect("collation_connection"),
            Some("utf8mb4_general_ci")
        );
        assert_eq!(at_connect("@tabletist_left_over"), None);
        assert_eq!(at_connect("in a transaction"), Some("false"));
        const SETS: [&str; 6] = [
            "SET time_zone = '+05:00'",
            "SET @tabletist_left_over = 1",
            "SET collation_connection = latin1_swedish_ci",
            "SET character_set_results = latin1",
            "SET sql_select_limit = 3",
            "SET sql_mode = 'ANSI,NO_BACKSLASH_ESCAPES'",
        ];
        // Every statement works.
        let outcome = past_the_refusal(&conn, &SETS, &StopFlag::new())
            .await
            .unwrap();
        assert_eq!(outcome.results.len(), SETS.len());
        assert!(outcome.results.iter().all(|result| result.outcome
            == StatementOutcome::Done {
                affected: None,
                warnings: 0,
            }));
        assert_eq!(settings(&conn).await, connected);
        // The last statement fails.
        let mut script = SETS.to_vec();
        script.push("SELECT nope");
        let outcome = past_the_refusal(&conn, &script, &StopFlag::new())
            .await
            .unwrap();
        assert_eq!(code(&outcome.results[SETS.len()].outcome), Some("42S22"));
        assert_eq!(settings(&conn).await, connected);
        // A statement ended the transaction (the server commits before it
        // refuses DDL), and the script went on.
        let outcome = past_the_refusal(&conn, &["COMMIT", SETS[0], SETS[1]], &StopFlag::new())
            .await
            .unwrap();
        assert_eq!(outcome.results.len(), 3);
        assert_eq!(settings(&conn).await, connected);
        // Stopped before the first statement.
        let stop = StopFlag::new();
        stop.stop();
        let outcome = past_the_refusal(&conn, &SETS, &stop).await.unwrap();
        assert_eq!(outcome.results.len(), 1);
        assert!(outcome.stopped);
        assert_eq!(settings(&conn).await, connected);
        // No statement at all.
        let outcome = past_the_refusal(&conn, &[], &StopFlag::new())
            .await
            .unwrap();
        assert_eq!(outcome, ScriptOutcome::default());
        assert_eq!(settings(&conn).await, connected);
        // The session left read-only: closed by the backend, and read-only
        // again all the same.
        let mut script = SETS.to_vec();
        script.push("SET SESSION TRANSACTION READ WRITE");
        let ran = past_the_refusal(&conn, &script, &StopFlag::new()).await;
        assert_eq!(ran, Err(Error::LeftReadOnly));
        assert_eq!(settings(&conn).await, connected);
    }

    /// A cancelled statement, and cancels that keep coming while the run
    /// cleans up: the session ends as it connected, or the run fails as a
    /// lost connection and the backend closes it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancelled_run_leaves_the_session_as_it_connected() {
        let Some(url) = test_url() else {
            return;
        };
        let mut admin = admin(&url).await;
        // A sleep of its own length each, for `runs_on_the_server`. With a
        // FROM the interrupted SLEEP is an error; alone it answers 1.
        for (keep_cancelling, sleep) in [(false, "SLEEP(41)"), (true, "SLEEP(42)")] {
            let conn = std::sync::Arc::new(session(&url).await);
            let connected = settings(&conn).await;
            let stop = StopFlag::new();
            let running = {
                let conn = std::sync::Arc::clone(&conn);
                let stop = stop.clone();
                tokio::spawn(async move {
                    let sleeps = format!("SELECT 1 FROM DUAL WHERE {sleep} = 0");
                    let script = ["SET time_zone = '+05:00'", sleeps.as_str()];
                    past_the_refusal(&conn, &script, &stop).await
                })
            };
            runs_on_the_server(&mut admin, sleep).await;
            stop.stop();
            while !running.is_finished() && (keep_cancelling || !stop.is_finishing()) {
                cancel(&conn.opts, conn.id, &conn.server, &conn.spare)
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            match running.await.unwrap() {
                Ok(outcome) => {
                    assert!(outcome.stopped, "{outcome:?}");
                    assert_eq!(
                        outcome.results.last().map(|result| &result.outcome),
                        Some(&StatementOutcome::Cancelled)
                    );
                    assert_eq!(settings(&conn).await, connected);
                }
                Err(error) => {
                    assert!(keep_cancelling, "{error}");
                    assert!(error.is_connection_lost(), "{error}");
                }
            }
        }
    }

    /// The reset keeps the connection id, which the cancel names: a run on
    /// a session that an earlier run reset is cancelled like the first.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancel_reaches_a_session_that_was_reset() {
        let Some(url) = test_url() else {
            return;
        };
        let mut admin = admin(&url).await;
        let conn = std::sync::Arc::new(session(&url).await);
        let connected = settings(&conn).await;
        let server_id = async |conn: &Conn| {
            conn.conn
                .lock()
                .await
                .query_first::<u32, _>("SELECT CONNECTION_ID()")
                .await
                .unwrap()
        };
        assert_eq!(server_id(&conn).await, Some(conn.id));
        for sleep in ["SLEEP(43)", "SLEEP(44)"] {
            // An earlier run, with its reset.
            let earlier = past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new()).await;
            assert_eq!(earlier.unwrap().results.len(), 1);
            assert_eq!(server_id(&conn).await, Some(conn.id));
            assert_eq!(conn.conn.lock().await.id(), conn.id);
            let stop = StopFlag::new();
            let running = {
                let conn = std::sync::Arc::clone(&conn);
                let stop = stop.clone();
                tokio::spawn(async move {
                    let sleeps = format!("SELECT 1 FROM DUAL WHERE {sleep} = 0");
                    past_the_refusal(&conn, &["SELECT 1", sleeps.as_str()], &stop).await
                })
            };
            runs_on_the_server(&mut admin, sleep).await;
            stop.stop();
            while !running.is_finished() && !stop.is_finishing() {
                cancel(&conn.opts, conn.id, &conn.server, &conn.spare)
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            let outcome = running.await.unwrap().unwrap();
            assert!(outcome.stopped, "{outcome:?}");
            assert_eq!(outcome.results.len(), 2);
            assert_eq!(outcome.results[1].outcome, StatementOutcome::Cancelled);
            assert_eq!(settings(&conn).await, connected);
        }
    }

    /// MariaDB before 11.1 knows the read-only setting only under its older
    /// name. MySQL 8 knows only the newer one, so asking in the other order
    /// takes the same way round.
    #[tokio::test]
    async fn the_read_only_setting_is_asked_for_under_its_other_name() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let mut conn = conn.conn.lock().await;
        let [name, older] = READ_ONLY_SETTINGS;
        assert_eq!(
            read_only_setting(&mut conn, [name, older]).await,
            Ok(Some(1))
        );
        assert_eq!(
            read_only_setting(&mut conn, [older, name]).await,
            Ok(Some(1))
        );
        assert_eq!(
            read_only_setting(&mut conn, ["tabletist_no_such_setting", name]).await,
            Ok(Some(1))
        );
        // Unknown under both names: the server's error, not a guess.
        let unknown = ["tabletist_no_such_setting", "tabletist_nor_this"];
        assert!(matches!(
            read_only_setting(&mut conn, unknown).await,
            Err(Error::Query { .. })
        ));
        // Any other error is not a reason to ask again.
        assert!(matches!(
            read_only_setting(&mut conn, ["sql_mode + nope", name]).await,
            Err(Error::Query { .. })
        ));
    }

    /// A cancel that landed on a writable session's opening queries: the
    /// session may still be read-write, and that is not a script that left.
    #[tokio::test]
    async fn closing_an_unopened_run_keeps_a_writable_session() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let mut conn = conn.conn.lock().await;
        execute(&mut conn, "SET SESSION TRANSACTION READ WRITE")
            .await
            .unwrap();
        close(&mut conn, Ended::Unopened, on_mysql(Access::Writable))
            .await
            .unwrap();
        let read_only = read_only_setting(&mut conn, READ_ONLY_SETTINGS)
            .await
            .unwrap();
        assert_eq!(read_only, Some(0));
    }

    /// A run that left read-only on a writable session does not make it
    /// read-write again: the backend closes the session, and until then
    /// nothing may write on it.
    #[tokio::test]
    async fn closing_a_run_that_left_keeps_a_writable_session_read_only() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let mut conn = conn.conn.lock().await;
        // What a statement past the refusal would have done.
        execute(&mut conn, "SET SESSION TRANSACTION READ WRITE")
            .await
            .unwrap();
        assert_eq!(
            close(&mut conn, Ended::Left, on_mysql(Access::Writable)).await,
            Err(Error::LeftReadOnly)
        );
        assert_eq!(
            read_only_setting(&mut conn, READ_ONLY_SETTINGS).await,
            Ok(Some(1))
        );
    }

    /// A transaction that cannot start is not a query error to show and
    /// repeat: the session is closed.
    #[tokio::test]
    async fn a_transaction_that_cannot_start_closes_the_session() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        // Inside an XA transaction the server refuses to start another.
        conn.conn
            .lock()
            .await
            .query_drop("XA START 'tabletist_cannot_start'")
            .await
            .unwrap();
        let stop = StopFlag::new();
        let ran = past_the_refusal(&conn, &["SELECT 1"], &stop).await;
        assert!(
            matches!(&ran, Err(Error::ConnectionLost(message))
                if message.starts_with("could not start the read-only transaction")),
            "{ran:?}"
        );
        assert!(stop.is_finishing());
    }

    /// A session the server closed ends the run as a lost connection, on
    /// whichever query finds out.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_lost_session_is_the_runs_error() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = std::sync::Arc::new(session(&url).await);
        let stop = StopFlag::new();
        let running = {
            let conn = std::sync::Arc::clone(&conn);
            let stop = stop.clone();
            tokio::spawn(async move {
                let script = ["SELECT 1", "SELECT 1 FROM DUAL WHERE SLEEP(45) = 0"];
                past_the_refusal(&conn, &script, &stop).await
            })
        };
        let mut admin = admin(&url).await;
        runs_on_the_server(&mut admin, "SLEEP(45)").await;
        admin.query_drop(format!("KILL {}", conn.id)).await.unwrap();
        let ran = running.await.unwrap();
        assert!(
            matches!(&ran, Err(error) if error.is_connection_lost()),
            "{ran:?}"
        );
        // And so does the next run, without hanging.
        let next = past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new()).await;
        assert!(
            matches!(&next, Err(error) if error.is_connection_lost()),
            "{next:?}"
        );
    }
}
