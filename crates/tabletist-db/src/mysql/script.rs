//! Running a SQL editor script on MySQL: one read-only transaction, the
//! checks around every statement, and the session reset that ends it.

use std::time::{Duration, Instant};

use mysql_async::Params;
use mysql_async::consts::StatusFlags;
use mysql_async::prelude::Queryable;
use mysql_common::named_params::ParsedNamedParams;

use super::{
    Conn, READ_ONLY, UNKNOWN_SYSTEM_VARIABLE, column_metas, execute, from_row, prepare_session,
    query_error, row_values, status,
};
use crate::script::{cannot_start, cleanup_failed, retry_cancelled, statement_failed};
use crate::{
    Access, Dialect, Error, Result, ScriptMode, ScriptOutcome, StatementOutcome, StatementResult,
    StopFlag,
};

impl Conn {
    /// See [`crate::Connection::run_script`]. Statements run through the
    /// prepared protocol, which cannot hold two, and `sql_select_limit`
    /// makes the server stop at `limit + 1` rows. The transaction is
    /// managed by hand, and the session is reset afterwards: what a script
    /// set must not reach table browsing or the next run.
    ///
    /// The future must be awaited to its end: a run that is dropped leaves
    /// the session inside the transaction, with the script's settings. The
    /// backend never drops one; it stops a run through `stop` and the
    /// session's cancel.
    pub async fn run_script(
        &self,
        texts: &[String],
        limit: u32,
        mode: ScriptMode,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        if mode == ScriptMode::Write {
            return Err(Error::Unsupported("read-write runs on MySQL"));
        }
        let mut conn = self.conn.lock().await;
        // Said before anything runs, not found out by the cleanup, which
        // would close the session after every run.
        if !can_reset(self.version.contains("MariaDB"), conn.server_version()) {
            return Err(Error::Unsupported(
                "the SQL editor needs MySQL 5.7.3 or MariaDB 10.2.4 or later",
            ));
        }
        let mut outcome = ScriptOutcome::default();
        let ended = match open(&mut conn, limit, self.access).await {
            // A lost session ends the run here: nothing to close.
            Ok(()) => statements(&mut conn, texts, limit as usize, stop, &mut outcome).await?,
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
        close(&mut conn, ended, self.access).await?;
        Ok(outcome)
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
    execute(conn, "START TRANSACTION READ ONLY").await?;
    started(status(conn))
}

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
/// the run, and sets its row limit: with `sql_select_limit` the server
/// stops producing rows, and the close's reset puts the default back.
/// `Err` is a cancel that landed on these queries, a lost session, or a
/// transaction that could not start.
async fn open(conn: &mut mysql_async::Conn, limit: u32, access: Access) -> Result<()> {
    // A writable session is read-write between scripts. The checks read
    // the session's own setting, and only that setting makes the server
    // refuse DDL after the commit DDL implies: so a run makes the session
    // read-only first, and the close puts read-write back.
    if access == Access::Writable {
        retry_cancelled!(execute(conn, READ_ONLY))?;
    }
    begin(conn).await?;
    // One row more than the limit, to know whether more exist.
    let rows = u64::from(limit) + 1;
    execute(conn, &format!("SET SESSION sql_select_limit = {rows}")).await
}

/// Runs the statements in order, up to the first that fails or is
/// stopped, and says what that showed of the session. `Err` is a lost
/// session.
async fn statements(
    conn: &mut mysql_async::Conn,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
    outcome: &mut ScriptOutcome,
) -> Result<Ended> {
    for text in texts {
        let checked = if stop.is_stopped() {
            Err(Error::Cancelled)
        } else {
            ready(conn).await
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
async fn ready(conn: &mut mysql_async::Conn) -> Result<Standing> {
    match standing(conn).await? {
        Standing::Outside => {
            begin(conn).await?;
            Ok(Standing::Inside)
        }
        standing => Ok(standing),
    }
}

/// Asks the server where the session stands: its read-only setting, and
/// the transaction status that comes with the answer.
async fn standing(conn: &mut mysql_async::Conn) -> Result<Standing> {
    let read_only = read_only_setting(conn, READ_ONLY_SETTINGS).await?;
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

/// A session setting's value. With a `LIMIT` of its own: a script can set
/// `sql_select_limit` to 0, and the answer must still come.
async fn setting(
    conn: &mut mysql_async::Conn,
    name: &str,
) -> mysql_async::Result<Option<mysql_async::Row>> {
    conn.query_first(format!("SELECT @@session.{name} LIMIT 1"))
        .await
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
async fn close(conn: &mut mysql_async::Conn, ended: Ended, access: Access) -> Result<()> {
    let ended = match ended {
        Ended::Unconfirmed => match retry_cancelled!(standing(conn)) {
            Ok(Standing::Left) => Ended::Left,
            Ok(Standing::Inside | Standing::Outside) => Ended::Unconfirmed,
            Err(error) => Ended::Broken(cleanup_failed(ScriptMode::ReadOnly, &error)),
        },
        known => known,
    };
    // Whatever is open is rolled back and reset as far as that works, also
    // when the session is closed anyway.
    let rolled_back = retry_cancelled!(execute(conn, "ROLLBACK"));
    let reset = retry_cancelled!(reset(conn));
    // A run that did not end cleanly leaves the session read-only, whatever
    // it was opened as: it is about to be closed, and until then nothing
    // may write on it.
    let clean = matches!(ended, Ended::Unconfirmed | Ended::Unopened)
        && rolled_back.is_ok()
        && reset.is_ok();
    let access = if clean { access } else { Access::ReadOnly };
    let prepared = prepare_session(conn, access).await;
    match ended {
        Ended::Left => Err(Error::LeftReadOnly),
        Ended::Broken(error) => Err(error),
        Ended::Unconfirmed | Ended::Unopened => rolled_back
            .and(reset)
            .and(prepared)
            .map_err(|error| cleanup_failed(ScriptMode::ReadOnly, &error)),
    }
}

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

/// Runs one statement of a script. A statement's failure is its outcome;
/// whether it also ended the transaction, the next check asks. `Err` is a
/// lost session.
async fn run_statement(
    conn: &mut mysql_async::Conn,
    text: &str,
    limit: usize,
) -> Result<StatementOutcome> {
    if let Some(outcome) = driver_parameter(text) {
        return Ok(outcome);
    }
    // Prepare first: the server refuses a second statement hidden in one
    // piece, and what the prepared protocol lacks. A prepare error is the
    // outcome; the text never runs another way.
    let statement = match conn.prep(text).await {
        Ok(statement) => statement,
        Err(error) => return failed(error),
    };
    // The driver closes the connection when a statement runs without
    // values for its parameters, so such a statement does not run.
    if statement.num_params() > 0 {
        return Ok(parameter("?", None));
    }
    let mut result = match conn.exec_iter(&statement, Params::Empty).await {
        Ok(result) => result,
        Err(error) => return failed(error),
    };
    // From the result, not the prepared statement: EXPLAIN prepares
    // without columns.
    let columns = column_metas(result.columns_ref());
    if columns.is_empty() {
        let affected = result.affected_rows();
        if let Err(error) = result.drop_result().await {
            return failed(error);
        }
        return Ok(StatementOutcome::Done {
            affected: counts_rows(text).then_some(affected),
            warnings: 0,
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

    /// The backend spawns a script run, so its future must be `Send`. This
    /// fails to compile, not to run.
    #[test]
    fn a_script_run_can_be_spawned() {
        fn send<T: Send>(_: &T) {}
        let _check = |conn: &Conn, texts: &[String], stop: &StopFlag| {
            send(&conn.run_script(texts, 10, ScriptMode::ReadOnly, stop));
            send(&conn.server_version());
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
    static PROBE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// A writable connection, outside the adapter.
    async fn admin(url: &str) -> mysql_async::Conn {
        let opts = Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        mysql_async::Conn::new(opts).await.unwrap()
    }

    /// A writable connection with an empty `probe` table, and the table's
    /// lock, held until the test ends.
    async fn probe(url: &str) -> (mysql_async::Conn, tokio::sync::MutexGuard<'static, ()>) {
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
    async fn probe_rows(admin: &mut mysql_async::Conn) -> i64 {
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
            conn.run_script(&texts, 10, ScriptMode::ReadOnly, stop),
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
            assert_eq!(standing(&mut conn).await, Ok(Standing::Outside));
            // The server marks the transaction read-only, or this fails.
            open(&mut conn, 10, Access::ReadOnly).await.unwrap();
            assert_eq!(standing(&mut conn).await, Ok(Standing::Inside));
            assert_eq!(ready(&mut conn).await, Ok(Standing::Inside));

            // What a statement that commits would have done, and one that
            // makes the next transaction read-write: the session's setting
            // does not show that, and a statement on its own would write.
            conn.query_drop("COMMIT").await.unwrap();
            conn.query_drop("SET TRANSACTION READ WRITE").await.unwrap();
            assert_eq!(standing(&mut conn).await, Ok(Standing::Outside));
            assert_eq!(ready(&mut conn).await, Ok(Standing::Inside));
            assert_eq!(standing(&mut conn).await, Ok(Standing::Inside));
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
            assert_eq!(standing(&mut conn).await, Ok(Standing::Left));
            assert_eq!(ready(&mut conn).await, Ok(Standing::Left));
            assert_eq!(
                close(&mut conn, Ended::Unconfirmed, Access::ReadOnly).await,
                Err(Error::LeftReadOnly)
            );
            assert_eq!(standing(&mut conn).await, Ok(Standing::Outside));

            // A script's own sql_select_limit does not blind the check.
            open(&mut conn, 10, Access::ReadOnly).await.unwrap();
            conn.query_drop("SET SESSION sql_select_limit = 0")
                .await
                .unwrap();
            assert_eq!(standing(&mut conn).await, Ok(Standing::Inside));
            conn.query_drop("SET SESSION TRANSACTION READ WRITE")
                .await
                .unwrap();
            assert_eq!(standing(&mut conn).await, Ok(Standing::Left));
            assert_eq!(
                close(&mut conn, Ended::Unconfirmed, Access::ReadOnly).await,
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
            open(&mut conn, 10, Access::ReadOnly).await.unwrap();
            // What the first statement of the script would have done.
            conn.query_drop("SET SESSION TRANSACTION READ WRITE")
                .await
                .unwrap();
            // The stop lands before the second.
            let stop = StopFlag::new();
            stop.stop();
            let mut outcome = ScriptOutcome::default();
            let texts = ["SELECT 1".to_owned()];
            let ended = statements(&mut conn, &texts, 10, &stop, &mut outcome).await;
            assert_eq!(ended, Ok(Ended::Unconfirmed));
            assert_eq!(outcome.results.len(), 1);
            assert_eq!(outcome.results[0].outcome, StatementOutcome::Cancelled);
            assert!(outcome.stopped);
            assert_eq!(
                close(&mut conn, Ended::Unconfirmed, Access::ReadOnly).await,
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
                cancel(&conn.opts, conn.id, &conn.server).await.unwrap();
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
                cancel(&conn.opts, conn.id, &conn.server).await.unwrap();
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
        close(&mut conn, Ended::Unopened, Access::Writable)
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
            close(&mut conn, Ended::Left, Access::Writable).await,
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
