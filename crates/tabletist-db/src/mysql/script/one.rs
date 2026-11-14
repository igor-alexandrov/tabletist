//! A SQL editor run of one statement in one round trip: the open, the
//! statement and the close's check and rollback go to the server as one
//! message, through the text protocol. Its answer is turned into the
//! binary protocol's, but for values whose text has lost digits: then the
//! statement runs again the other way (`read_only`), after a reset.

use std::collections::HashSet;
use std::hash::{BuildHasher, RandomState};
use std::sync::Arc;
use std::time::Instant;

use mysql_async::consts::{ColumnType, StatusFlags};
use mysql_async::prelude::Queryable;
use mysql_async::{QueryResult, TextProtocol};

use super::super::{
    UNKNOWN_SYSTEM_VARIABLE, WRONG_ARGUMENTS, column_metas, from_row, hex_literal, query_error,
    status, text_values,
};
use super::{
    Ended, SessionKind, Standing, Unfinished, closing, counts_rows, driver_parameter, end, opened,
    opening, parameter, settle, stands,
};
use crate::script::{cannot_start, cleanup_failed, statement_failed};
use crate::{
    Error, Result, ScriptMode, ScriptOutcome, StatementOutcome, StatementResult, StopFlag,
};

/// What the one message came to.
#[derive(Debug)]
pub(super) enum One {
    /// The run's outcome, and the reset that is still to come.
    Ran(Box<ScriptOutcome>, Unfinished),
    /// The statement is to run the other way. The session is in no
    /// transaction. Where `lossy`, the statement ran, and the text of its
    /// answer lost digits: the session is to be reset before it runs
    /// again, so that it runs on the session as the first did, and the
    /// text is remembered (see [`Lossy`]).
    Again { lossy: bool },
}

/// The name of the statement the message prepares on the server, and of
/// the user variable that holds its text. The reset after the run drops
/// both.
const RUN: &str = "tabletist_run";

/// The longest statement the message carries, in bytes. Its text goes as
/// hex, twice as long, and a message longer than the server's
/// `max_allowed_packet` would cost the connection; a longer statement
/// runs the other way, which sends it as it is.
const LONGEST: usize = 256 * 1024;

/// What `decimals` says in a column's definition when its values are not
/// written to a fixed number of decimals. Below it, a DOUBLE's text is
/// cut to that many (`PI()` is `3.141593`).
const NOT_FIXED_DECIMALS: u8 = 31;

/// The message's steps, and where some of them are.
#[derive(Debug)]
struct Message {
    steps: Vec<String>,
    /// The question whether the session is read-only, as the open asks it.
    asked: usize,
    /// The step that sets the statement's text, but only where the
    /// session is read-only. It answers without rows, with the server's
    /// status after the open.
    guard: usize,
    /// The statement's run.
    ran: usize,
}

/// The message of a run of `text`: the open (`opening`), the guard, the
/// statement prepared on the server from the user variable the guard set
/// and run, and the close's check and rollback (`closing`).
///
/// The guard sets the variable to the statement's text where the session
/// says it is read-only, and to NULL otherwise, which the `PREPARE` then
/// refuses: the server checks the session before the statement runs, as
/// the open's answer is checked before a statement runs the other way.
/// The text goes as hex (see `hex_literal`), so nothing in it can end the
/// literal early or reach the message: the server's `PREPARE` reads it as
/// one statement, and refuses a text of two, as the binary protocol's
/// prepare does. The server stops the message at the first statement
/// that fails, so the statement runs only after the open, and alone.
fn message(kind: SessionKind, limit: u32, read_only: &str, text: &str) -> Message {
    let mut steps = opening(kind.access, limit, None, read_only);
    let asked = steps.len() - 1;
    let guard = steps.len();
    steps.push(format!(
        "SET @{RUN} = IF(@@session.{read_only} = 1, {}, NULL)",
        hex_literal("_utf8mb4", text.as_bytes())
    ));
    steps.push(format!("PREPARE {RUN} FROM @{RUN}"));
    let ran = steps.len();
    steps.push(format!("EXECUTE {RUN}"));
    steps.extend(closing(read_only));
    Message {
        steps,
        asked,
        guard,
        ran,
    }
}

/// One answer in the message's.
#[derive(Debug)]
struct Heard {
    /// Its rows: none for a statement that answers without; at most
    /// `limit + 1` for the statement's run.
    rows: Vec<mysql_async::Row>,
    columns: Arc<[mysql_async::Column]>,
    /// The server's status, its count of changed rows and of warnings as
    /// the driver had them before the answer was read: for an answer
    /// without rows these are its own.
    status: StatusFlags,
    affected: u64,
    warnings: u16,
}

/// Sends `steps` as one message and reads every answer to them, up to the
/// error of the statement that failed, if one did. The statement's run
/// may answer more than once (a `CALL`): every answer is read, so the
/// close's are the last three. Of the run's first answer, `ran`, at most
/// `limit + 1` rows are kept: `sql_select_limit` does not bound every
/// statement, and the rest is read off the wire.
async fn heard(
    conn: &mut mysql_async::Conn,
    steps: &[String],
    ran: usize,
    limit: usize,
) -> (Vec<Heard>, Option<mysql_async::Error>) {
    let mut answers = Vec::new();
    if let Err(error) = conn.query_iter(steps.join("; ")).await {
        return (answers, Some(error));
    }
    loop {
        let before = (status(conn), conn.affected_rows(), conn.get_warnings());
        let mut result = QueryResult::<'_, '_, TextProtocol>::new(&mut *conn);
        let Some(columns) = result.columns() else {
            // No answer is left, or the next is the error of the
            // statement that failed.
            return (answers, result.drop_result().await.err());
        };
        let keep = if answers.len() == ran {
            limit + 1
        } else {
            usize::MAX
        };
        let mut rows = Vec::new();
        loop {
            match result.next().await {
                Ok(Some(row)) if rows.len() < keep => rows.push(row),
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(error) => return (answers, Some(error)),
            }
        }
        let (status, affected, warnings) = before;
        answers.push(Heard {
            rows,
            columns,
            status,
            affected,
            warnings,
        });
    }
}

/// Runs `text`, one statement of the SQL editor, in one message (see
/// [`message`]), and closes the run as `end` would. `Again` where the
/// statement is to run the other way: one that is too long, has a
/// parameter the driver would read, or whose answer lost digits in text;
/// and where the server does not know the read-only setting under the name
/// the message used. `Err` is a lost session, or a run that closes it.
pub(super) async fn run(
    conn: &mut mysql_async::Conn,
    text: &str,
    limit: u32,
    kind: SessionKind,
    stop: &StopFlag,
) -> Result<One> {
    // The other way gives such a text its outcome without running it.
    if text.len() > LONGEST || driver_parameter(text).is_some() {
        return Ok(One::Again { lossy: false });
    }
    let names = kind.read_only_names(conn.server_version());
    let message = message(kind, limit, names[0], text);
    let rows = limit as usize;
    let started = Instant::now();
    let (mut answers, failure) = heard(conn, &message.steps, message.ran, rows).await;
    let elapsed = started.elapsed();
    let count = answers.len();
    let refused = |code: u16| matches!(&failure, Some(mysql_async::Error::Server(error)) if error.code == code);
    let unknown = refused(UNKNOWN_SYSTEM_VARIABLE);
    let unfilled = matches!(&failure, Some(mysql_async::Error::Server(error))
        if is_a_parameter(error));
    let error = failure.map(query_error);
    let complete = error.is_none() && count >= message.steps.len();
    let mut outcome = ScriptOutcome::default();
    if count <= message.guard {
        // The open did not answer in full: the statement did not run.
        let ended = match error.unwrap_or_else(missing) {
            _ if unknown && count >= message.asked => {
                drop(end(conn, Ended::Unopened, kind).await?);
                return Ok(One::Again { lossy: false });
            }
            Error::Cancelled => {
                outcome.stopped = true;
                Ended::Unopened
            }
            error if error.is_connection_lost() => return Err(error),
            error => Ended::Broken(cannot_start(ScriptMode::ReadOnly, &error)),
        };
        stop.finish();
        return end(conn, ended, kind)
            .await
            .map(|unfinished| One::Ran(Box::new(outcome), unfinished));
    }
    // The open answered, and the statement may have run: the answer is
    // held to what it is held to before a statement runs the other way.
    // The guard kept the statement from running where the session did
    // not say it was read-only.
    let read_only = std::mem::take(&mut answers[message.asked].rows)
        .into_iter()
        .next()
        .map(from_row)
        .transpose();
    if let Err(error) =
        read_only.and_then(|read_only| opened(read_only, answers[message.guard].status))
    {
        stop.finish();
        let ended = Ended::Broken(cannot_start(ScriptMode::ReadOnly, &error));
        return end(conn, ended, kind)
            .await
            .map(|unfinished| One::Ran(Box::new(outcome), unfinished));
    }
    // Where the close's check answered, its answers are the message's
    // last three, however many the statement gave.
    let ended = if complete {
        closed(&answers[count - 3..])
    } else {
        Ended::Unconfirmed
    };
    let result = if count <= message.ran {
        // The prepare or the run failed.
        match error {
            _ if unfilled && count == message.ran => parameter("?", None),
            error => statement_failed(error.unwrap_or_else(missing), None)?,
        }
    } else {
        let lossless = answered(&mut answers[message.ran], text, rows);
        let result = match error {
            // A cancel that landed on the close: the outcome stands, as it
            // does the other way, where the close then runs again.
            None | Some(Error::Cancelled) => lossless,
            Some(error) if error.is_connection_lost() => return Err(error),
            // A later answer of the statement's failed (a `CALL`'s), or a
            // step of the close did.
            Some(error) => Some(StatementOutcome::Error {
                error,
                position: None,
            }),
        };
        match result {
            Some(result) => result,
            // Its text lost digits: it runs again the other way, once the
            // run ended cleanly and unless the run is stopped.
            None if ended == Ended::Unconfirmed && complete && !stop.is_stopped() => {
                return Ok(One::Again { lossy: true });
            }
            None => StatementOutcome::Cancelled,
        }
    };
    outcome.stopped |= matches!(result, StatementOutcome::Cancelled);
    outcome.results.push(StatementResult {
        elapsed,
        outcome: result,
    });
    // From here on a cancel would land on the cleanup.
    stop.finish();
    let unfinished = if complete {
        settle(conn, ended, Some(Ok(())), kind).await?
    } else {
        end(conn, Ended::Unconfirmed, kind).await?
    };
    Ok(One::Ran(Box::new(outcome), unfinished))
}

/// Where the session stands, from the close's three answers (see
/// `confirm`): `DO 0`'s status, the read-only setting, the rollback.
fn closed(answers: &[Heard]) -> Ended {
    let standing = match answers {
        [found, asked, _] => asked
            .rows
            .first()
            .cloned()
            .map(from_row)
            .transpose()
            .and_then(|read_only| stands(read_only, found.status)),
        _ => Err(missing()),
    };
    match standing {
        Ok(Standing::Left) => Ended::Left,
        Ok(Standing::Inside | Standing::Outside) => Ended::Unconfirmed,
        Err(error) => Ended::Broken(cleanup_failed(ScriptMode::ReadOnly, &error)),
    }
}

/// What the server says when a statement with a parameter runs without
/// values for it: `EXECUTE` was given none.
fn is_a_parameter(refused: &mysql_async::ServerError) -> bool {
    refused.code == WRONG_ARGUMENTS && refused.message.contains("EXECUTE")
}

/// The error for a message the server stopped answering without saying
/// why.
fn missing() -> Error {
    Error::query("unexpected data from the server: an answer is missing")
}

/// The statement's outcome from its first answer, as the binary protocol
/// would have given it; `None` where a value's text lost digits (see
/// [`lossy`]) or is not what the server writes for its type.
fn answered(answer: &mut Heard, text: &str, limit: usize) -> Option<StatementOutcome> {
    let columns = column_metas(&answer.columns);
    if columns.is_empty() {
        return Some(StatementOutcome::Done {
            affected: counts_rows(text).then_some(answer.affected),
            warnings: answer.warnings,
        });
    }
    let lossy: Vec<usize> = (0..answer.columns.len())
        .filter(|&at| lossy(&answer.columns[at]))
        .collect();
    let mut rows = std::mem::take(&mut answer.rows)
        .into_iter()
        .map(|row| {
            let cut = lossy
                .iter()
                .any(|&at| row.as_ref(at) != Some(&mysql_async::Value::NULL));
            if cut {
                return None;
            }
            text_values(row, &answer.columns, &columns)
        })
        .collect::<Option<Vec<_>>>()?;
    let truncated = rows.len() > limit;
    rows.truncate(limit);
    Some(StatementOutcome::Rows {
        columns,
        rows,
        truncated,
    })
}

/// Whether the text protocol writes a value of `column` with fewer digits
/// than the binary protocol sends: a FLOAT, to six significant digits,
/// and a DOUBLE whose column has a fixed number of decimals, to that
/// many. A DOUBLE without is written in full.
fn lossy(column: &mysql_async::Column) -> bool {
    match column.column_type() {
        ColumnType::MYSQL_TYPE_FLOAT => true,
        ColumnType::MYSQL_TYPE_DOUBLE => column.decimals() < NOT_FIXED_DECIMALS,
        _ => false,
    }
}

/// The statements of this session whose answer lost digits in text: the
/// next run of one goes the other way at once. Kept as hashes, a few
/// hundred at most; a hash two texts share sends the other the other
/// way too, which only costs it the round trips.
#[derive(Debug, Default)]
pub(crate) struct Lossy {
    seen: std::sync::Mutex<HashSet<u64>>,
    hasher: RandomState,
}

impl Lossy {
    /// The most kept; past it, they are forgotten and kept afresh.
    const MOST: usize = 256;

    pub(super) fn knows(&self, text: &str) -> bool {
        let hash = self.hasher.hash_one(text);
        self.seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(&hash)
    }

    pub(super) fn remember(&self, text: &str) {
        let hash = self.hasher.hash_one(text);
        let mut seen = self
            .seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if seen.len() >= Self::MOST {
            seen.clear();
        }
        seen.insert(hash);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use mysql_async::prelude::Queryable;

    use super::super::super::Conn;
    use super::super::super::tests::{
        counted_session, on_mariadb, session, session_as, swapping_session, table_of, test_url,
    };
    use super::super::tests::{
        admin, past_the_refusal, probe, probe_rows, runs_on_the_server, settings,
    };
    use super::super::{SessionKind, read_only};
    use super::*;
    use crate::{Access, Value};

    /// The message for `text` on a read-only MySQL session, as the server
    /// gets it, with a limit of 10 rows: what a test proxy swaps.
    fn sent(text: &str) -> String {
        let kind = SessionKind {
            access: Access::ReadOnly,
            mariadb: false,
        };
        message(kind, 10, "transaction_read_only", text)
            .steps
            .join("; ")
    }

    #[test]
    fn the_open_the_statement_and_the_close_are_one_message() {
        assert_eq!(
            sent("SELECT 'é'"),
            "START TRANSACTION READ ONLY; SET SESSION sql_select_limit = 11; \
             SELECT @@session.transaction_read_only LIMIT 1; \
             SET @tabletist_run = IF(@@session.transaction_read_only = 1, \
             _utf8mb4 X'53454c4543542027c3a927', NULL); \
             PREPARE tabletist_run FROM @tabletist_run; EXECUTE tabletist_run; \
             DO 0; SELECT @@session.transaction_read_only LIMIT 1; \
             ROLLBACK AND NO CHAIN NO RELEASE"
        );
        // A writable session's fence comes first; MariaDB before 11.1 is
        // asked under the older name.
        let kind = SessionKind {
            access: Access::Writable,
            mariadb: true,
        };
        let message = message(kind, 5, "tx_read_only", "SELECT 1");
        assert_eq!(message.steps[0], "SET SESSION TRANSACTION READ ONLY");
        assert_eq!(
            message.steps[message.asked],
            "SELECT @@session.tx_read_only LIMIT 1"
        );
        assert!(
            message.steps[message.guard]
                .starts_with("SET @tabletist_run = IF(@@session.tx_read_only = 1, ")
        );
        assert_eq!(message.steps[message.ran], "EXECUTE tabletist_run");
        assert_eq!(message.steps.len(), message.ran + 4);
    }

    /// The outcome of `text` the other way, each step a round trip of its
    /// own, on a session that is as it connected afterwards.
    async fn the_other_way(conn: &Conn, text: &str) -> ScriptOutcome {
        let kind = conn.kind();
        let mut session = conn.conn.lock().await;
        let texts = [text.to_owned()];
        let stop = StopFlag::new();
        let (outcome, _, unfinished) = read_only(&mut session, &texts, 10, kind, &stop, None, None)
            .await
            .unwrap();
        unfinished.finish(&mut session).await.unwrap();
        outcome
    }

    /// Without the time each took, which differs.
    fn outcomes(outcome: &ScriptOutcome) -> (bool, Vec<&StatementOutcome>) {
        let results = outcome.results.iter().map(|result| &result.outcome);
        (outcome.stopped, results.collect())
    }

    /// A table with every kind of column, at their edges and NULL, as the
    /// page of #320 reads it, and a FLOAT and a DOUBLE with decimals.
    async fn every_kind(url: &str) -> String {
        let object = table_of(
            url,
            "editor_types",
            "id bigint PRIMARY KEY, ti tinyint, tu tinyint unsigned, si smallint, \
             mi mediumint unsigned, i int, bu bigint unsigned, de decimal(30,10), d double, \
             dm double(10,3), y year, dt date, t time(2), dtm datetime, dt3 datetime(3), \
             ts timestamp(6) NULL, c char(4), vc varchar(40) COLLATE utf8mb4_bin, \
             l varchar(10) CHARACTER SET latin1, tx text, bn binary(16), vb varbinary(8), \
             bl blob, bt bit(9), e enum('x','y'), st set('a','b'), j json, g geometry NULL, \
             f float",
            "(1, -128, 255, -32768, 16777215, -2147483648, 18446744073709551615, \
              '-12345678901234567890.0123456789', 0.1e0 + 0.2e0, 1.5, 2024, '2024-02-29', \
              '-838:59:59.50', '2024-01-02 03:04:05', '2024-01-02 03:04:05.120', \
              '2024-01-02 03:04:05.000001', 'ab', 'O''Brien C:\\\\temp', 'é', 'x', \
              UNHEX('0123456789abcdef0123456789abcdef'), x'00ff', x'', b'101010101', 'y', \
              'a,b', '{\"a\": [1, 2.5, null]}', ST_GeomFromText('POINT(1 2)'), 1.2345679), \
             (2, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, \
              NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, \
              NULL, NULL), \
             (3, 127, 0, 32767, 0, 2147483647, 9223372036854775807, '0', 5e-324, -0.0, 0, \
              '0000-00-00', '838:59:59', '0000-00-00 00:00:00', '2024-01-02 03:04:05.000', \
              '1970-01-01 00:00:01', '', '%_\\\\', '', '', x'', x'', x'00', b'0', 'x', '', \
              '[]', NULL, 16777217), \
             (4, 0, 1, 0, 1, 0, 9223372036854775808, '1e-10', -2.2250738585072014e-308, \
              123456.789, 1999, '1000-01-01', '00:00:00.01', '9999-12-31 23:59:59', \
              '2024-01-02 03:04:05.999', '2038-01-19 03:14:07.999999', 'abcd', 'abc', 'x', \
              REPEAT('y', 300), x'ffffffffffffffffffffffffffffffff', x'61', x'ff00', \
              b'111111111', 'x', 'b', '{}', ST_GeomFromText('LINESTRING(0 0, 1 1)'), 0.1)",
        )
        .await;
        two_answers(url).await;
        format!("`{}`", object.name)
    }

    /// A procedure that answers twice, `tabletist_two`.
    async fn two_answers(url: &str) {
        let mut admin = admin(url).await;
        admin
            .query_drop(
                "CREATE PROCEDURE IF NOT EXISTS tabletist_two() \
                 BEGIN SELECT 1 AS a; SELECT 2.5e0 AS b; END",
            )
            .await
            .unwrap();
    }

    /// A statement run in one message answers as it does the other way:
    /// its columns, its rows and each value's type, its row count and
    /// warnings, its error, on a read-only and a writable session. Over
    /// every kind of column, at their edges and NULL, and values the server
    /// computes; values whose text lost digits (a FLOAT, a DOUBLE with
    /// decimals) are read again the other way. Statements the server's
    /// `PREPARE` refuses fail as the binary protocol's prepare fails them,
    /// with the same error: none runs another way.
    #[tokio::test]
    async fn a_statement_in_one_message_answers_as_it_does_the_other_way() {
        let Some(url) = test_url() else {
            return;
        };
        let table = every_kind(&url).await;
        let texts = [
            format!("SELECT * FROM {table} ORDER BY id"),
            format!("SELECT id, d, dm FROM {table} ORDER BY id"),
            format!("SELECT f FROM {table} WHERE id = 2"),
            format!("SELECT d * 2, d / 3, -d, AVG(d), SUM(dm) FROM {table} GROUP BY d, dm"),
            "SELECT 0.1e0 + 0.2e0, 1e0 / 3, -2.2250738585072014e-308, 5e-324, \
             1.7976931348623157e308, CAST(-1 AS UNSIGNED), -9223372036854775808, 1 / 3, \
             TIME('-838:59:59.5'), CAST('2024-01-01' AS DATETIME(6)), b'101', x'ff00', \
             JSON_OBJECT('a', 1.5), 'Привет 😀', NULL, YEAR('2024-05-01'), \
             ST_GeomFromText('POINT(1 2)'), 1e0 * 0, 1 / 0"
                .to_owned(),
            "SELECT PI(), CAST(1.2345679 AS FLOAT), 1".to_owned(),
            "SELECT CAST(NULL AS FLOAT), PI() FROM DUAL WHERE 0".to_owned(),
            "SHOW VARIABLES LIKE 'sql_select_limit'".to_owned(),
            "SHOW COLLATION".to_owned(),
            "SELECT COLLATION_NAME FROM information_schema.COLLATIONS ORDER BY 1".to_owned(),
            "SELECT COLLATION_NAME FROM information_schema.COLLATIONS ORDER BY 1 LIMIT 500"
                .to_owned(),
            "EXPLAIN SELECT 1".to_owned(),
            "CALL tabletist_two()".to_owned(),
            "DO 1".to_owned(),
            "SET @tabletist_one = 1".to_owned(),
            "SELEC 1".to_owned(),
            "SELECT 1; SELECT 2".to_owned(),
            "SELECT 1;".to_owned(),
            "SELECT 'abc".to_owned(),
            "SELECT 1 /* x".to_owned(),
            "SELECT 1 -- x".to_owned(),
            "SELECT 1 # x".to_owned(),
            "SELECT * FROM tabletist_nowhere".to_owned(),
            "SELECT ?".to_owned(),
            "SELECT :name".to_owned(),
            "SHOW TABLES".to_owned(),
            "SHOW PROFILE".to_owned(),
            "USE tabletist".to_owned(),
            "COMMIT".to_owned(),
            String::new(),
            format!("INSERT INTO {table} (id) VALUES (99)"),
        ];
        for access in [Access::ReadOnly, Access::Writable] {
            let conn = session_as(&url, access).await;
            for text in &texts {
                let one = conn
                    .script(
                        std::slice::from_ref(text),
                        10,
                        ScriptMode::ReadOnly,
                        &StopFlag::new(),
                    )
                    .await
                    .unwrap();
                let other = the_other_way(&conn, text).await;
                assert_eq!(outcomes(&one), outcomes(&other), "{access:?} {text}");
            }
        }
    }

    /// The round trips a statement on its own waits for: one for its
    /// message, and one more for a close of its own where the message
    /// stopped at an error. The reset and the settings come after. A
    /// statement that answers more than once (a `CALL`) still has the
    /// close's answers last.
    #[tokio::test(flavor = "current_thread")]
    async fn a_statement_on_its_own_is_one_round_trip() {
        let Some(url) = test_url() else {
            return;
        };
        two_answers(&url).await;
        for access in [Access::ReadOnly, Access::Writable] {
            let (conn, asked) = counted_session(&url, access).await;
            let connected = settings(&conn).await;
            for (text, waited) in [
                ("SELECT 1", 1),
                ("SHOW COLLATION", 1),
                ("CALL tabletist_two()", 1),
                ("DO 1", 1),
                ("SELECT CAST(NULL AS FLOAT)", 1),
                ("SELEC 1", 2),
                ("SELECT ?", 2),
                ("SELECT * FROM tabletist_nowhere", 2),
            ] {
                let before = asked.load(Ordering::SeqCst);
                let outcome = past_the_refusal(&conn, &[text], &StopFlag::new())
                    .await
                    .unwrap();
                assert_eq!(outcome.results.len(), 1, "{outcome:?}");
                assert_eq!(asked.load(Ordering::SeqCst) - before, waited, "{text}");
                drop(conn.conn.lock().await);
                assert_eq!(asked.load(Ordering::SeqCst) - before, waited + 2, "{text}");
                assert_eq!(settings(&conn).await, connected, "{access:?} {text}");
            }
        }
    }

    /// A statement whose answer lost digits in text runs again through the
    /// binary protocol, after a reset: the message, the reset and the
    /// settings, and the four round trips of the other way. The session
    /// remembers it, so it runs the other way at once the next time.
    #[tokio::test(flavor = "current_thread")]
    async fn a_statement_whose_text_lost_digits_runs_again_the_other_way() {
        let Some(url) = test_url() else {
            return;
        };
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let connected = settings(&conn).await;
        let text = "SELECT CAST(1.2345679 AS FLOAT), PI(), \
                    CAST(@tabletist_counted := COALESCE(@tabletist_counted, 0) + 1 AS SIGNED)";
        for waited in [7, 4] {
            let before = asked.load(Ordering::SeqCst);
            let outcome = past_the_refusal(&conn, &[text], &StopFlag::new())
                .await
                .unwrap();
            assert_eq!(asked.load(Ordering::SeqCst) - before, waited);
            let [result] = outcome.results.as_slice() else {
                panic!("{outcome:?}");
            };
            let StatementOutcome::Rows { rows, .. } = &result.outcome else {
                panic!("{result:?}");
            };
            // The run again found the session as the first did.
            assert_eq!(
                rows[0],
                [
                    Value::Float(1.234_567_9),
                    Value::Float(std::f64::consts::PI),
                    Value::Int(1),
                ]
            );
            drop(conn.conn.lock().await);
            assert_eq!(settings(&conn).await, connected);
        }
    }

    /// Two statements in one text, or one that a quote or a comment left
    /// open, do not run: the server's `PREPARE` refuses the text, and the
    /// message stops there. The text is hex, so it cannot end the guard's
    /// literal or reach the message.
    #[tokio::test]
    async fn no_statement_hidden_in_the_text_runs() {
        let Some(url) = test_url() else {
            return;
        };
        let (mut admin, _turn) = probe(&url).await;
        for access in [Access::ReadOnly, Access::Writable] {
            let conn = session_as(&url, access).await;
            for text in [
                "SELECT 1; INSERT INTO probe VALUES (1)",
                "SELECT 1 FROM DUAL; COMMIT; INSERT INTO probe VALUES (1)",
                "SELECT '1'; INSERT INTO probe VALUES (1); SELECT '",
                "SELECT 1 -- x\n; INSERT INTO probe VALUES (1)",
                "'); INSERT INTO probe VALUES (1); -- ",
            ] {
                let outcome = past_the_refusal(&conn, &[text], &StopFlag::new())
                    .await
                    .unwrap();
                let [result] = outcome.results.as_slice() else {
                    panic!("{outcome:?}");
                };
                assert!(
                    matches!(
                        &result.outcome,
                        StatementOutcome::Error { error: Error::Query { code, .. }, .. }
                            if code.as_deref() == Some("42000")
                    ),
                    "{access:?} {text}: {result:?}"
                );
                assert_eq!(probe_rows(&mut admin).await, 0, "{access:?} {text}");
            }
        }
    }

    /// The guard: where the session is not read-only when the message gets
    /// to the statement, the statement is not prepared, so it does not
    /// run. Here the proxy sends a writable session's message with a step
    /// that sets nothing in the fence's place. The open's answer then fails
    /// the run, which closes the session; the statement, a sleep, never
    /// started.
    #[tokio::test]
    async fn a_session_that_is_not_read_only_runs_no_statement() {
        let Some(url) = test_url() else {
            return;
        };
        let text = "SELECT SLEEP(3)";
        let kind = SessionKind {
            access: Access::Writable,
            mariadb: false,
        };
        let full: &'static str = message(kind, 10, "transaction_read_only", text)
            .steps
            .join("; ")
            .leak();
        let unfenced = full.replacen("SET SESSION TRANSACTION READ ONLY", "DO 0", 1);
        let (conn, _) = swapping_session(&url, Access::Writable, (full, unfenced.leak())).await;
        if on_mariadb(&conn) && conn.conn.lock().await.server_version() < (11, 1, 0) {
            return;
        }
        let stop = StopFlag::new();
        let started = std::time::Instant::now();
        let ran = past_the_refusal(&conn, &[text], &stop).await;
        assert!(started.elapsed() < Duration::from_secs(2), "{ran:?}");
        assert!(
            matches!(&ran, Err(Error::ConnectionLost(message))
                if message.starts_with("could not start the read-only transaction")
                    && message.contains("did not say that the session is read-only")),
            "{ran:?}"
        );
        assert!(stop.is_finishing());
    }

    /// An open in the message that fails, or whose answer does not say the
    /// session is inside a read-only transaction, fails the run, as the
    /// open the other way does, and the close leaves the session as it
    /// connected. Where the open's step failed, the statement did not run.
    #[tokio::test]
    async fn an_open_in_the_message_that_fails_fails_the_run() {
        let Some(url) = test_url() else {
            return;
        };
        let full: &'static str = sent("SELECT 1").leak();
        for (instead, said) in [
            (
                full.replace("sql_select_limit = 11", "sql_select_limit = 'x'"),
                "Incorrect argument type",
            ),
            (
                full.replacen("START TRANSACTION READ ONLY; ", "", 1),
                "the server did not start a transaction",
            ),
            (
                full.replacen(
                    "START TRANSACTION READ ONLY",
                    "START TRANSACTION READ WRITE",
                    1,
                ),
                "the server did not mark the transaction read-only",
            ),
            (
                full.replacen(
                    "SELECT @@session.transaction_read_only LIMIT 1",
                    "SELECT 0",
                    1,
                ),
                "the server did not say that the session is read-only",
            ),
        ] {
            let (conn, _) =
                swapping_session(&url, Access::ReadOnly, (full, instead.clone().leak())).await;
            if on_mariadb(&conn) && conn.conn.lock().await.server_version() < (11, 1, 0) {
                return;
            }
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

    /// A cancel that lands on the open in the message stops the run before
    /// the statement: no results, and the session as it connected.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancel_on_the_open_in_the_message_runs_no_statement() {
        let Some(url) = test_url() else {
            return;
        };
        let mut admin = admin(&url).await;
        let full: &'static str = sent("SELECT 1").leak();
        let slow = full.replacen(
            "SELECT @@session.transaction_read_only LIMIT 1",
            "SELECT 1 FROM DUAL WHERE SLEEP(43) = 0",
            1,
        );
        let (conn, _) = swapping_session(&url, Access::ReadOnly, (full, slow.leak())).await;
        if on_mariadb(&conn) && conn.conn.lock().await.server_version() < (11, 1, 0) {
            return;
        }
        let conn = std::sync::Arc::new(conn);
        let connected = settings(&conn).await;
        let running = {
            let conn = std::sync::Arc::clone(&conn);
            tokio::spawn(
                async move { past_the_refusal(&conn, &["SELECT 1"], &StopFlag::new()).await },
            )
        };
        runs_on_the_server(&mut admin, "SLEEP(43)").await;
        super::super::super::cancel(&conn.opts, conn.id, &conn.server, &conn.spare)
            .await
            .unwrap();
        let outcome = running.await.unwrap().unwrap();
        assert!(outcome.stopped, "{outcome:?}");
        assert!(outcome.results.is_empty(), "{outcome:?}");
        assert_eq!(settings(&conn).await, connected);
    }

    /// A cancel that lands on the statement stops it: the message stops
    /// there, and the close's check and rollback run on their own. The
    /// session is as it connected.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cancel_on_the_statement_in_the_message_stops_the_run() {
        let Some(url) = test_url() else {
            return;
        };
        let mut admin = admin(&url).await;
        // Counted, with a proxy that lets the spare through; it swaps
        // nothing the run sends.
        let (conn, asked) =
            swapping_session(&url, Access::ReadOnly, ("tabletist sends no such text", "")).await;
        let conn = std::sync::Arc::new(conn);
        let connected = settings(&conn).await;
        let before = asked.load(Ordering::SeqCst);
        let stop = StopFlag::new();
        let running = {
            let conn = std::sync::Arc::clone(&conn);
            let stop = stop.clone();
            tokio::spawn(async move {
                past_the_refusal(&conn, &["SELECT 1 FROM DUAL WHERE SLEEP(44) = 0"], &stop).await
            })
        };
        runs_on_the_server(&mut admin, "SLEEP(44)").await;
        stop.stop();
        super::super::super::cancel(&conn.opts, conn.id, &conn.server, &conn.spare)
            .await
            .unwrap();
        let outcome = running.await.unwrap().unwrap();
        assert!(outcome.stopped, "{outcome:?}");
        assert!(
            matches!(
                outcome.results.as_slice(),
                [StatementResult {
                    outcome: StatementOutcome::Cancelled,
                    ..
                }]
            ),
            "{outcome:?}"
        );
        assert!(stop.is_finishing());
        // The message, the close, the reset and the settings.
        drop(conn.conn.lock().await);
        assert_eq!(asked.load(Ordering::SeqCst) - before, 4);
        assert_eq!(settings(&conn).await, connected);
    }

    /// A statement that leaves read-only fails the run, which closes the
    /// session: also one that answers more than once, whose answers come
    /// before the close's check.
    #[tokio::test]
    async fn a_statement_that_left_read_only_fails_the_run() {
        let Some(url) = test_url() else {
            return;
        };
        let mut admin = admin(&url).await;
        admin
            .query_drop(
                "DROP PROCEDURE IF EXISTS tabletist_leaves; \
                 CREATE PROCEDURE tabletist_leaves() BEGIN \
                 SET @@session.transaction_read_only = 0; SELECT 1 AS a; SELECT 1 AS b; END",
            )
            .await
            .unwrap();
        for text in [
            "SET @@session.transaction_read_only = 0",
            "CALL tabletist_leaves()",
        ] {
            let conn = session(&url).await;
            let connected = settings(&conn).await;
            let stop = StopFlag::new();
            let ran = past_the_refusal(&conn, &[text], &stop).await;
            assert_eq!(ran, Err(Error::LeftReadOnly), "{text}");
            assert!(stop.is_finishing(), "{text}");
            assert_eq!(settings(&conn).await, connected, "{text}");
        }
    }

    /// The row limit holds in the message as it does the other way: the
    /// server stops a SELECT at `limit + 1` rows, and the rest of a
    /// statement it does not stop is read off the wire.
    #[tokio::test]
    async fn the_row_limit_holds_in_the_message() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        for text in [
            "SELECT COLLATION_NAME FROM information_schema.COLLATIONS",
            "SELECT COLLATION_NAME FROM information_schema.COLLATIONS LIMIT 100",
            "SHOW COLLATION",
        ] {
            let outcome = past_the_refusal(&conn, &[text], &StopFlag::new())
                .await
                .unwrap();
            let [result] = outcome.results.as_slice() else {
                panic!("{outcome:?}");
            };
            let StatementOutcome::Rows {
                rows, truncated, ..
            } = &result.outcome
            else {
                panic!("{result:?}");
            };
            assert_eq!((rows.len(), *truncated), (10, true), "{text}");
        }
    }

    /// Where the server does not know the read-only setting under the name
    /// the message used, the message stops at the open, rolls back, and
    /// the statement runs the other way, which asks under the other name.
    /// Here a MySQL server is taken for an old MariaDB.
    #[tokio::test]
    async fn a_setting_the_server_does_not_know_sends_the_run_the_other_way() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        if on_mariadb(&conn) {
            // MariaDB 11.1 and later know both names.
            return;
        }
        let kind = SessionKind {
            access: Access::ReadOnly,
            mariadb: true,
        };
        let mut session = conn.conn.lock().await;
        let ran = run(&mut session, "SELECT 1", 5, kind, &StopFlag::new())
            .await
            .unwrap();
        assert!(matches!(ran, One::Again { lossy: false }), "{ran:?}");
        assert!(!status(&session).contains(StatusFlags::SERVER_STATUS_IN_TRANS));
    }
}
