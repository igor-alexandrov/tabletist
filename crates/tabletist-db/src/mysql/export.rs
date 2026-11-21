//! An export's reads on MySQL and MariaDB, in as few round trips as the
//! rows allow: a table's shape in one message, then its rows (or a SQL
//! result's) in one more, streamed as they arrive (see
//! [`Conn::stream_in_one`](super::Conn)).

use mysql_async::prelude::Queryable;
use mysql_async::{QueryResult, TextProtocol};

use super::{
    BINARY_COLUMNS, PRIMARY_KEY, RUN, Shape, StreamFailed, WRONG_ARGUMENTS, column_metas, floats,
    inlined, lossy, parameter, prepared_run, query_error, read_start, script, text_values, typed,
    write,
};
use crate::{Error, ObjectRef, Result, StopFlag, Streamed};

/// Whether an export reads a table through the binary protocol (see
/// [`Shape`]): where it has a FLOAT column, as [`super::LISTED_COLUMNS`]
/// tells one (any column whose type starts with it), or is a view, as
/// [`super::COMMENT`] tells one. A view's column may be an expression
/// that is a DOUBLE the text protocol cuts to a few decimals (`PI() * id`
/// to six), and `information_schema` gives it the same type as a table's
/// DOUBLE(M,D), which is stored to its D decimals and written in full. Its
/// `?`s are the schema and the table, and then the two again.
const TABLE_FLOATS: &str = "SELECT EXISTS (SELECT 1 FROM information_schema.columns \
                              WHERE table_schema = ? AND table_name = ? \
                                AND LOWER(column_type) LIKE 'float%') \
                            OR EXISTS (SELECT 1 FROM information_schema.tables \
                              WHERE table_schema = ? AND table_name = ? \
                                AND table_type LIKE '%VIEW')";

/// What an export says when what it reads changed between the reads it
/// was planned from and its run (see [`Run`]): a table's key or byte string
/// columns, or a column that came to lose digits in text.
pub(super) const CHANGED: &str = "a table the export reads changed as it started; export again";

/// What an export's one message runs (see [`stream_message`]), and so what
/// it checks before it sends a row on.
pub(super) enum Run<'a> {
    /// A table's rows, built from a [`Shape`] read a round trip before.
    /// The message asks for the table's key and byte string columns again
    /// just after its start, and `builds` says whether they build the
    /// statement sent: one built from another key or other byte string
    /// columns orders or filters the rows another way (a filter's `0x6162`
    /// compared as text finds no bytes), and fails the export. So does a
    /// FLOAT among the run's columns (see [`floats`]).
    ///
    /// Only a FLOAT: a DOUBLE whose column has a fixed number of decimals
    /// (see [`lossy`]) is a table's DOUBLE(M,D) here, stored to those
    /// decimals and so written in full. The one that loses digits is a
    /// view's expression, and a view is read the other way (see
    /// [`TABLE_FLOATS`]).
    Table {
        object: &'a ObjectRef,
        builds: &'a (dyn Fn(&Shape) -> bool + Sync),
    },
    /// A SQL result, whose columns the driver's prepare found none of
    /// that loses digits in text. The message prepares it again, and any
    /// column that does there (see [`lossy`]), a view replaced in between
    /// say, fails the export.
    Statement,
}

impl Run<'_> {
    /// Whether a column of the run as `raw` describes it, `columns` as
    /// read from it, has lost digits in text where the plan said none
    /// would (see [`Run`]).
    fn cut(&self, raw: &[mysql_async::Column], columns: &[crate::ColumnMeta]) -> bool {
        match self {
            Self::Table { .. } => floats(columns),
            Self::Statement => raw.iter().any(lossy),
        }
    }
}

/// `object`'s [`Shape`] in one round trip: its primary key, its byte
/// string columns and whether it is read through the binary protocol (see
/// [`TABLE_FLOATS`]), asked as one message.
pub(super) async fn read_shape(conn: &mut mysql_async::Conn, object: &ObjectRef) -> Result<Shape> {
    let at = [object.schema.as_str(), object.name.as_str()];
    let twice = [at[0], at[1], at[0], at[1]];
    let questions = [
        inlined(PRIMARY_KEY, &at),
        inlined(BINARY_COLUMNS, &at),
        inlined(TABLE_FLOATS, &twice),
    ];
    let (answers, error) = script::answers(conn, &questions).await;
    if let Some(error) = error {
        return Err(query_error(error));
    }
    let Ok([key, binary, floats]) = <[script::Answer; 3]>::try_from(answers) else {
        return Err(Error::query(
            "unexpected data from the server: an answer is missing",
        ));
    };
    Ok(Shape {
        key: typed(key.rows)?,
        binary: typed(binary.rows)?,
        floats: typed::<bool>(floats.rows)?
            .first()
            .copied()
            .unwrap_or(false),
    })
}

/// `text` run with `values` (as [`super::literal`]s) in one message
/// through the text protocol, as [`super::page_in_one`] runs a page: the
/// start of the read-only transaction, a table's key and byte string
/// columns where `run` is one (see [`Run::Table`]), the statement prepared
/// from a user variable, its run, the cleanup, the rollback. The run's rows
/// are sent on to `to` a batch at a time as they arrive. The server stops a
/// message at its first statement that fails, so the run happens inside the
/// transaction or not at all, and only one statement can be prepared.
///
/// What the run's rows are checked for (see [`Run`]) fails the export
/// before its columns are sent on: [`CHANGED`], [`StreamFailed::PartWay`]
/// once the run's columns are in, since its rows are on their way.
///
/// The driver reads an answer's header with the end of the answer before
/// it, and keeps an error found there for the read that comes to it. A
/// failure up to the run, the run's own in place of its columns included,
/// is [`StreamFailed::Before`]: nothing more is on its way, and the
/// transaction may be open. A run given another number of values than
/// its text has `?`s says so, as a page's does (see [`before_run`]). Any
/// failure once the run's columns are in is [`StreamFailed::PartWay`].
pub(super) async fn stream_message(
    conn: &mut mysql_async::Conn,
    text: &str,
    values: &[String],
    run: &Run<'_>,
    batch: usize,
    to: &tokio::sync::mpsc::Sender<Streamed>,
    stop: &StopFlag,
) -> std::result::Result<u64, StreamFailed> {
    let mut steps = read_start(conn);
    let asked = steps.len();
    if let Run::Table { object, .. } = run {
        let at = [object.schema.as_str(), object.name.as_str()];
        steps.extend([inlined(PRIMARY_KEY, &at), inlined(BINARY_COLUMNS, &at)]);
    }
    let ran = steps.len() + RUN;
    steps.extend(prepared_run(text, values));
    steps.push(write::ROLLBACK.to_owned());
    let before = |error| StreamFailed::Before(before_run(error));
    // The first answer is in once this returns; the rest stays on the
    // connection, where a new `QueryResult` reads it (see `script::answers`).
    conn.query_iter(steps.join("; ")).await.map_err(before)?;
    let mut answers = Vec::with_capacity(ran);
    for _ in 0..ran {
        let rows = QueryResult::<'_, '_, TextProtocol>::new(&mut *conn)
            .collect::<mysql_async::Row>()
            .await
            .map_err(before)?;
        answers.push(rows);
    }
    // Both questions are answered before the run, which is at `ran`. What
    // their answers say is weighed once the run's start is in (see
    // `text_sent_on`): the rows may be on their way by then.
    let changed = match run {
        Run::Table { builds, .. } => {
            let mut answered = |at: usize| typed(std::mem::take(&mut answers[at]));
            answered(asked).and_then(|key| {
                let binary = answered(asked + 1)?;
                Ok(!builds(&Shape {
                    key,
                    binary,
                    floats: false,
                }))
            })
        }
        Run::Statement => Ok(false),
    };
    let mut result = QueryResult::<'_, '_, TextProtocol>::new(&mut *conn);
    let sent = text_sent_on(&mut result, run, changed, batch, to, stop).await?;
    // The cleanup and the rollback are all that is left.
    stop.finish();
    QueryResult::<'_, '_, TextProtocol>::new(&mut *conn)
        .drop_result()
        .await
        .map_err(|error| StreamFailed::PartWay(query_error(error)))?;
    Ok(sent)
}

/// A failure up to the run (see [`stream_message`]). `EXECUTE` given
/// another number of values than the statement has parameters is a `?`
/// with no value, as [`super::counted`] and [`super::answered`] say.
fn before_run(error: mysql_async::Error) -> crate::Error {
    match error {
        mysql_async::Error::Server(refused) if refused.code == WRONG_ARGUMENTS => {
            parameter("?", None)
        }
        error => query_error(error),
    }
}

/// The run's rows as the text protocol sends them, read as the binary
/// protocol would have sent them (see [`super::binary_form`]) and sent on
/// to `to` a batch at a time. Where the table `changed` since the shape the
/// statement was built from, or a column has lost digits that `run` says
/// none would (see [`Run::cut`]), the export fails with [`CHANGED`] before
/// the columns are sent on: what was read before the run said otherwise,
/// so what it reads changed in between. Where the message's answers for the
/// table's shape could not be read, it fails with that error, as early.
async fn text_sent_on(
    result: &mut QueryResult<'_, '_, TextProtocol>,
    run: &Run<'_>,
    changed: Result<bool>,
    batch: usize,
    to: &tokio::sync::mpsc::Sender<Streamed>,
    stop: &StopFlag,
) -> std::result::Result<u64, StreamFailed> {
    let part_way = StreamFailed::PartWay;
    let Some(raw) = result.columns() else {
        // The run's error stands where its columns would: it did not start.
        // A statement built from a shape gone failing is that shape's
        // doing, as in `super::answered`, unless a cancel stopped it or the
        // connection was lost.
        let failed = result.next().await.err().map(before_run);
        return Err(StreamFailed::Before(match (failed, changed) {
            (Some(error @ (Error::Cancelled | Error::ConnectionLost(_))), _) => error,
            (_, Err(error)) => error,
            (_, Ok(true)) => Error::query(CHANGED),
            (Some(error), Ok(false)) => error,
            (None, Ok(false)) => {
                Error::query("unexpected data from the server: the run gave no answer")
            }
        }));
    };
    let columns = column_metas(&raw);
    if changed.map_err(part_way)? || run.cut(&raw, &columns) {
        return Err(part_way(Error::query(CHANGED)));
    }
    crate::stream::send(to, Streamed::Columns(columns.clone()))
        .await
        .map_err(part_way)?;
    let mut sent = 0u64;
    let mut rows = Vec::with_capacity(batch);
    while let Some(row) = result
        .next()
        .await
        .map_err(|error| part_way(query_error(error)))?
    {
        let values = text_values(row, &raw, &columns).ok_or_else(|| {
            part_way(Error::query(
                "unexpected data from the server: a value is not what its type writes",
            ))
        })?;
        rows.push(values);
        if rows.len() == batch {
            if stop.is_stopped() {
                return Err(part_way(Error::Cancelled));
            }
            sent += rows.len() as u64;
            let full = std::mem::replace(&mut rows, Vec::with_capacity(batch));
            crate::stream::send(to, Streamed::Rows(full))
                .await
                .map_err(part_way)?;
        }
    }
    if !rows.is_empty() {
        sent += rows.len() as u64;
        crate::stream::send(to, Streamed::Rows(rows))
            .await
            .map_err(part_way)?;
    }
    Ok(sent)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use mysql_async::prelude::Queryable;

    use super::super::Conn;
    use super::super::tests::{counted_session, in_transaction, test_url};
    use crate::adapter::Adapter;
    use crate::{Access, ColumnMeta, ObjectRef, Result, RowQuery, StopFlag, Streamed, Value};

    /// One test at a time creates the export tables: creating one twice
    /// at once can fail.
    static TABLES: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// The digits 0 to 9, as rows of `d`, to build numbered rows from on
    /// MySQL and MariaDB alike (MariaDB has no `cte_max_recursion_depth`).
    const DIGITS: &str = "(SELECT 0 d UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 \
                          UNION ALL SELECT 4 UNION ALL SELECT 5 UNION ALL SELECT 6 \
                          UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9)";

    /// The export tables, in the test database, and its name:
    /// `export_trips` (12,000 rows of an `int` key and a name),
    /// `export_floats` (one FLOAT row), `export_values` (a row of each
    /// common type, and a row of NULLs), `export_doubles` (DOUBLEs with two
    /// decimals), `export_tokens` (a row whose VARBINARY is `0x6162`) and
    /// the view `export_pi_view` (`PI()` times each of `export_trips`'
    /// keys). `export_values` took over from an `export_types` with fewer
    /// columns, which an earlier run may have left as it was.
    async fn export_tables(url: &str) -> String {
        let _turn = TABLES.lock().await;
        let opts = mysql_async::Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
        admin
            .query_drop(format!(
                "CREATE TABLE IF NOT EXISTS export_trips (id int PRIMARY KEY, name varchar(20)); \
                 INSERT IGNORE INTO export_trips (id, name) \
                 SELECT n + 1, CONCAT('row ', n + 1) FROM ( \
                   SELECT a.d + 10 * b.d + 100 * c.d + 1000 * e.d + 10000 * f.d AS n \
                   FROM {DIGITS} a, {DIGITS} b, {DIGITS} c, {DIGITS} e, {DIGITS} f) g \
                 WHERE n < 12000; \
                 CREATE TABLE IF NOT EXISTS export_floats (id int PRIMARY KEY, f float); \
                 INSERT IGNORE INTO export_floats VALUES (1, 1.2345678); \
                 CREATE TABLE IF NOT EXISTS export_values (id int PRIMARY KEY, \
                   big bigint unsigned, money decimal(10, 2), ratio double, \
                   stamp datetime(3), day date, span time, yr year, name varchar(20), \
                   bytes varbinary(4), doc json, sum double, flags bit(8), \
                   size enum('small', 'large'), tags set('red', 'blue'), at timestamp NULL, \
                   small tinyint unsigned); \
                 INSERT IGNORE INTO export_values VALUES \
                   (1, 18446744073709551615, 12.34, 0.1, '2024-01-02 03:04:05.678', \
                    '2024-01-02', '-01:02:03', 2024, 'ñame', 0x00ff10, '{{\"a\": [1, 2]}}', \
                    0.30000000000000004e0, b'10100101', 'large', 'red,blue', \
                    '2024-01-02 03:04:05', 255), \
                   (2, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, \
                    NULL, NULL, NULL, NULL, NULL); \
                 CREATE TABLE IF NOT EXISTS export_doubles (id int PRIMARY KEY, \
                   d double(10, 2)); \
                 INSERT IGNORE INTO export_doubles VALUES \
                   (1, 1.25), (2, 2.5), (3, 12345678.91), (4, NULL); \
                 CREATE TABLE IF NOT EXISTS export_tokens (id int PRIMARY KEY, \
                   token varbinary(4)); \
                 INSERT IGNORE INTO export_tokens VALUES (1, 0x6162), (2, 0x6364); \
                 CREATE OR REPLACE VIEW export_pi_view AS \
                   SELECT id, PI() * id AS p FROM export_trips"
            ))
            .await
            .unwrap();
        admin
            .query_first("SELECT DATABASE()")
            .await
            .unwrap()
            .unwrap()
    }

    /// A table export as the export runs one: the statement it shows,
    /// then its rows streamed, with what `to` was sent.
    async fn exported_table(
        conn: &Conn,
        query: &RowQuery,
        columns: &[String],
        stop: &StopFlag,
    ) -> (
        Result<String>,
        Result<u64>,
        Vec<ColumnMeta>,
        Vec<Vec<Vec<Value>>>,
    ) {
        let shown = conn.export_statement(query, columns).await;
        let (sent, metas, batches) =
            streamed(|to| async move { conn.stream_rows(query, columns, 5_000, &to, stop).await })
                .await;
        (shown, sent, metas, batches)
    }

    /// A SQL result's export: what it sent `to`.
    async fn exported_statement(
        conn: &Conn,
        text: &str,
        stop: &StopFlag,
    ) -> (Result<u64>, Vec<ColumnMeta>, Vec<Vec<Vec<Value>>>) {
        streamed(|to| async move { conn.stream_statement(text, 5_000, &to, stop).await }).await
    }

    /// Runs `stream` with a sender whose receiver is read as it goes, as
    /// the export's writer reads it: the outcome, the columns, the batches.
    async fn streamed<F, Fut>(stream: F) -> (Result<u64>, Vec<ColumnMeta>, Vec<Vec<Vec<Value>>>)
    where
        F: FnOnce(tokio::sync::mpsc::Sender<Streamed>) -> Fut,
        Fut: std::future::Future<Output = Result<u64>>,
    {
        let (to, mut from) = tokio::sync::mpsc::channel(1);
        let reading = async move {
            let (mut columns, mut batches) = (Vec::new(), Vec::new());
            while let Some(streamed) = from.recv().await {
                match streamed {
                    Streamed::Columns(metas) => columns = metas,
                    Streamed::Rows(rows) => batches.push(rows),
                }
            }
            (columns, batches)
        };
        let (sent, (columns, batches)) = tokio::join!(stream(to), reading);
        (sent, columns, batches)
    }

    fn trips_query(schema: &str) -> (RowQuery, Vec<String>) {
        (
            RowQuery::new(ObjectRef::new(schema, "export_trips"), 0),
            vec!["id".to_owned(), "name".to_owned()],
        )
    }

    /// A table export reads the table's shape once, for the statement it
    /// shows and for its rows, and then its rows in one message: two round
    /// trips.
    #[tokio::test]
    async fn a_table_export_reads_its_shape_once() {
        let Some(url) = test_url() else {
            return;
        };
        let schema = export_tables(&url).await;
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let (query, columns) = trips_query(&schema);
        let before = asked.load(Ordering::SeqCst);
        let (shown, sent, _, batches) =
            exported_table(&conn, &query, &columns, &StopFlag::new()).await;
        let trips = asked.load(Ordering::SeqCst) - before;
        assert!(shown.unwrap().contains("ORDER BY"));
        assert_eq!(sent.unwrap(), 12_000);
        let sizes: Vec<usize> = batches.iter().map(Vec::len).collect();
        assert_eq!(sizes, [5_000, 5_000, 2_000]);
        assert_eq!(trips, 2);
        assert!(!in_transaction(&conn).await);
    }

    /// A SQL result's export waits twice: the driver's prepare, which
    /// gives its column types, and the one message that runs it.
    #[tokio::test]
    async fn a_statement_export_waits_twice() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let before = asked.load(Ordering::SeqCst);
        let (sent, columns, batches) = exported_statement(
            &conn,
            "SELECT id, name FROM export_trips ORDER BY id",
            &StopFlag::new(),
        )
        .await;
        let trips = asked.load(Ordering::SeqCst) - before;
        assert_eq!(sent.unwrap(), 12_000);
        let names: Vec<&str> = columns.iter().map(|column| column.name.as_str()).collect();
        assert_eq!(names, ["id", "name"]);
        let sizes: Vec<usize> = batches.iter().map(Vec::len).collect();
        assert_eq!(sizes, [5_000, 5_000, 2_000]);
        assert_eq!(
            batches[0][0],
            vec![Value::Int(1), Value::Text("row 1".into())]
        );
        assert_eq!(
            batches[2][1_999],
            vec![Value::Int(12_000), Value::Text("row 12000".into())]
        );
        assert_eq!(trips, 2);
        assert!(!in_transaction(&conn).await);
    }

    /// The one message and the binary protocol give the same values, row
    /// for row, for each common type and for NULL.
    #[tokio::test]
    async fn one_message_gives_the_values_the_binary_protocol_gives() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, _) = counted_session(&url, Access::ReadOnly).await;
        let sql = crate::dialect::Sql {
            text: "SELECT * FROM export_values ORDER BY id".to_owned(),
            params: Vec::new(),
        };
        let stop = StopFlag::new();
        let (sent, one_columns, one) = streamed(|to| {
            let (conn, sql, stop) = (&conn, &sql, &stop);
            async move {
                conn.stream_in_one(sql, &super::Run::Statement, 10, &to, stop)
                    .await
            }
        })
        .await;
        assert_eq!(sent.unwrap(), 2);
        let (sent, binary_columns, binary) = streamed(|to| {
            let (conn, sql, stop) = (&conn, &sql, &stop);
            async move { conn.stream_sql(sql, 10, &to, stop).await }
        })
        .await;
        assert_eq!(sent.unwrap(), 2);
        assert_eq!(one_columns, binary_columns);
        assert_eq!(one, binary);
    }

    /// A FLOAT keeps every digit it has: a table or a result with one is
    /// read through the binary protocol, as a page with one is. The table's
    /// shape, the start, the prepare, the run, the rollback; for a result,
    /// the prepare, then the start, the run and the rollback.
    #[tokio::test]
    async fn a_float_keeps_its_digits_in_an_export() {
        let Some(url) = test_url() else {
            return;
        };
        let schema = export_tables(&url).await;
        let expected = vec![vec![
            Value::Int(1),
            super::super::value(mysql_async::Value::Float(1.234_567_8), "float"),
        ]];
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let query = RowQuery::new(ObjectRef::new(&schema, "export_floats"), 0);
        let columns = vec!["id".to_owned(), "f".to_owned()];
        let before = asked.load(Ordering::SeqCst);
        let (_, sent, _, batches) = exported_table(&conn, &query, &columns, &StopFlag::new()).await;
        assert_eq!(sent.unwrap(), 1);
        assert_eq!(batches, vec![expected.clone()]);
        assert_eq!(asked.load(Ordering::SeqCst) - before, 5);
        let before = asked.load(Ordering::SeqCst);
        let (sent, _, batches) =
            exported_statement(&conn, "SELECT id, f FROM export_floats", &StopFlag::new()).await;
        assert_eq!(sent.unwrap(), 1);
        assert_eq!(batches, vec![expected]);
        assert_eq!(asked.load(Ordering::SeqCst) - before, 4);
        assert!(!in_transaction(&conn).await);
    }

    /// A text of two statements is refused by the prepare, and nothing of
    /// it runs: no columns, no rows, no transaction, and every row of the
    /// table is still there.
    #[tokio::test]
    async fn an_export_of_two_statements_runs_neither() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, _) = counted_session(&url, Access::Writable).await;
        let (sent, columns, batches) = exported_statement(
            &conn,
            "SELECT 1; DELETE FROM export_trips",
            &StopFlag::new(),
        )
        .await;
        assert!(sent.is_err());
        assert!(columns.is_empty() && batches.is_empty());
        assert!(!in_transaction(&conn).await);
        let mut session = conn.conn.lock().await;
        let left: Option<u64> = session
            .conn()
            .unwrap()
            .query_first("SELECT COUNT(*) FROM export_trips")
            .await
            .unwrap();
        assert_eq!(left, Some(12_000));
    }

    /// A stop that comes before the run keeps it from running, on the one
    /// message's way and on the binary protocol's: the export waits only
    /// on the driver's prepare.
    #[tokio::test]
    async fn a_stop_before_the_run_keeps_it_from_running() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        for text in ["SELECT id FROM export_trips", "SELECT f FROM export_floats"] {
            let stop = StopFlag::new();
            stop.stop();
            let before = asked.load(Ordering::SeqCst);
            let (sent, columns, batches) = exported_statement(&conn, text, &stop).await;
            // The driver's prepare, and nothing after it.
            assert_eq!(asked.load(Ordering::SeqCst) - before, 1, "{text}");
            assert!(
                matches!(sent, Err(crate::Error::Cancelled)),
                "{text}: {sent:?}"
            );
            assert!(columns.is_empty() && batches.is_empty(), "{text}");
            assert!(!in_transaction(&conn).await, "{text}");
        }
    }

    /// The server's own time limit ends a statement the one message runs,
    /// and is told apart from other failures.
    #[tokio::test]
    async fn the_servers_timeout_ends_a_one_message_export() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, _) = counted_session(&url, Access::ReadOnly).await;
        {
            let limit = if conn.version.contains("MariaDB") {
                "SET SESSION max_statement_time = 0.3"
            } else {
                "SET SESSION max_execution_time = 300"
            };
            let mut session = conn.conn.lock().await;
            session.conn().unwrap().query_drop(limit).await.unwrap();
        }
        let (sent, _, _) = exported_statement(
            &conn,
            "SELECT COUNT(*) FROM export_trips a, export_trips b, export_trips c \
             WHERE a.id + b.id + c.id > 0",
            &StopFlag::new(),
        )
        .await;
        assert!(matches!(sent, Err(crate::Error::Timeout)), "{sent:?}");
        // The limit ends the run once its columns are in, part way: the
        // transaction is left to the export's connection, which is closed
        // after a failure (see `Conn::stream_sql`).
    }

    /// A run that fails where its columns would be has not started: it
    /// sends nothing, and its transaction ends. The run here has no value
    /// for its parameter, which the server refuses at `EXECUTE`, and the
    /// export says so as a page or a count does.
    #[tokio::test]
    async fn a_run_that_fails_at_its_start_sends_nothing() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, _) = counted_session(&url, Access::ReadOnly).await;
        let sql = crate::dialect::Sql {
            text: "SELECT id FROM export_trips WHERE id = ?".to_owned(),
            params: Vec::new(),
        };
        let stop = StopFlag::new();
        let (sent, columns, batches) = streamed(|to| {
            let (conn, sql, stop) = (&conn, &sql, &stop);
            async move {
                conn.stream_in_one(sql, &super::Run::Statement, 10, &to, stop)
                    .await
            }
        })
        .await;
        let crate::Error::Query {
            message: expected, ..
        } = super::super::parameter("?", None)
        else {
            unreachable!("a parameter's error is a query error");
        };
        assert!(
            matches!(&sent, Err(crate::Error::Query { message, .. }) if *message == expected),
            "{sent:?}"
        );
        assert!(columns.is_empty() && batches.is_empty());
        assert!(!in_transaction(&conn).await);
    }

    /// `text` exported as a SQL result, and through the binary protocol
    /// alone: what each sent, and the round trips the export took.
    async fn both_ways(
        conn: &Conn,
        asked: &std::sync::atomic::AtomicUsize,
        text: &str,
    ) -> (
        (Vec<ColumnMeta>, Vec<Vec<Vec<Value>>>),
        (Vec<ColumnMeta>, Vec<Vec<Vec<Value>>>),
        usize,
    ) {
        let before = asked.load(Ordering::SeqCst);
        let (sent, columns, batches) = exported_statement(conn, text, &StopFlag::new()).await;
        let trips = asked.load(Ordering::SeqCst) - before;
        sent.unwrap();
        let sql = crate::dialect::Sql {
            text: text.to_owned(),
            params: Vec::new(),
        };
        let stop = StopFlag::new();
        let (sent, binary_columns, binary) = streamed(|to| {
            let (sql, stop) = (&sql, &stop);
            async move { conn.stream_sql(sql, 5_000, &to, stop).await }
        })
        .await;
        sent.unwrap();
        ((columns, batches), (binary_columns, binary), trips)
    }

    /// A DOUBLE the server writes to a fixed number of decimals (`PI()`
    /// to six, a sum over a DOUBLE(10,2) to two) keeps every digit in a
    /// SQL result's export: it is read through the binary protocol. The
    /// prepare, then the start, the run and the rollback.
    #[tokio::test]
    async fn a_cut_double_keeps_its_digits_in_a_statement_export() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        for text in [
            "SELECT PI() AS p",
            "SELECT SUM(d) / 3 AS t FROM export_doubles",
        ] {
            let (exported, binary, trips) = both_ways(&conn, &asked, text).await;
            assert_eq!(exported, binary, "{text}");
            assert_eq!(trips, 4, "{text}");
        }
        assert!(!in_transaction(&conn).await);
    }

    /// A view's column may be an expression the server writes to a fixed
    /// number of decimals: a view is read through the binary protocol,
    /// and keeps every digit. The view's shape, the start, the prepare,
    /// the run, the rollback.
    #[tokio::test]
    async fn a_view_keeps_its_digits_in_an_export() {
        let Some(url) = test_url() else {
            return;
        };
        let schema = export_tables(&url).await;
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let query = RowQuery::new(ObjectRef::new(&schema, "export_pi_view"), 0);
        let columns = vec!["id".to_owned(), "p".to_owned()];
        let before = asked.load(Ordering::SeqCst);
        let (_, sent, _, batches) = exported_table(&conn, &query, &columns, &StopFlag::new()).await;
        assert_eq!(sent.unwrap(), 12_000);
        assert_eq!(asked.load(Ordering::SeqCst) - before, 5);
        assert!(conn.read_export_shape(&query.object).await.unwrap().floats);
        // Browsing is given no export's shape.
        assert_eq!(conn.known_shape(&query.object), None);
        for row in batches.iter().flatten() {
            let Value::Int(id) = row[0] else {
                panic!("{row:?}");
            };
            assert_eq!(row[1], Value::Float(std::f64::consts::PI * id as f64));
        }
        assert!(!in_transaction(&conn).await);
    }

    /// A table's DOUBLE(10,2) is stored to two decimals, which its text
    /// has in full: such a table still goes in one message, and gives the
    /// values the binary protocol gives.
    #[tokio::test]
    async fn a_tables_fixed_decimals_go_in_one_message() {
        let Some(url) = test_url() else {
            return;
        };
        let schema = export_tables(&url).await;
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let query = RowQuery::new(ObjectRef::new(&schema, "export_doubles"), 0);
        let columns = vec!["id".to_owned(), "d".to_owned()];
        let before = asked.load(Ordering::SeqCst);
        let (_, sent, _, batches) = exported_table(&conn, &query, &columns, &StopFlag::new()).await;
        assert_eq!(asked.load(Ordering::SeqCst) - before, 2);
        assert_eq!(sent.unwrap(), 4);
        assert!(!conn.read_export_shape(&query.object).await.unwrap().floats);
        let (_, (_, binary), _) = both_ways(
            &conn,
            &asked,
            "SELECT id, d FROM export_doubles ORDER BY id",
        )
        .await;
        assert_eq!(batches, binary);
        assert_eq!(batches[0][2][1], Value::Float(12_345_678.91));
    }

    /// A statement longer than the one message carries is read through
    /// the binary protocol, which sends it as it is: the prepare, then the
    /// start, the run and the rollback.
    #[tokio::test]
    async fn a_long_statement_export_takes_the_binary_protocol() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let text = format!(
            "SELECT id, name FROM export_trips /* {} */ ORDER BY id",
            "x".repeat(super::super::LONGEST)
        );
        let before = asked.load(Ordering::SeqCst);
        let (sent, _, batches) = exported_statement(&conn, &text, &StopFlag::new()).await;
        assert_eq!(asked.load(Ordering::SeqCst) - before, 4);
        assert_eq!(sent.unwrap(), 12_000);
        assert_eq!(
            batches[2][1_999],
            vec![Value::Int(12_000), Value::Text("row 12000".into())]
        );
        assert!(!in_transaction(&conn).await);
    }

    /// A table's rows streamed as `stream_rows` streams them, with no
    /// `export_statement` before: what it sent `to`.
    async fn streamed_rows(
        conn: &Conn,
        query: &RowQuery,
        columns: &[String],
    ) -> (Result<u64>, Vec<ColumnMeta>, Vec<Vec<Vec<Value>>>) {
        let stop = StopFlag::new();
        streamed(|to| {
            let stop = &stop;
            async move { conn.stream_rows(query, columns, 5_000, &to, stop).await }
        })
        .await
    }

    /// Keeps `shape` for the next export of `object` on `conn`, as
    /// `export_statement` keeps the one it read.
    fn keep_for_export(conn: &Conn, object: &ObjectRef, shape: super::super::Shape) {
        conn.export_shapes
            .lock()
            .unwrap()
            .insert(object.clone(), shape);
    }

    /// A FLOAT the shape did not know of fails the export before any row
    /// is sent. The shape was taken by the export, so that the export done
    /// again reads it afresh, and keeps every digit.
    #[tokio::test]
    async fn a_float_the_shape_missed_fails_the_export() {
        let Some(url) = test_url() else {
            return;
        };
        let schema = export_tables(&url).await;
        let (conn, _) = counted_session(&url, Access::ReadOnly).await;
        let object = ObjectRef::new(&schema, "export_floats");
        // The key as it is, so that only the FLOAT tells the change.
        let stale = super::super::Shape {
            key: vec!["id".to_owned()],
            ..Default::default()
        };
        keep_for_export(&conn, &object, stale);
        let query = RowQuery::new(object.clone(), 0);
        let columns = vec!["id".to_owned(), "f".to_owned()];
        let (sent, metas, batches) = streamed_rows(&conn, &query, &columns).await;
        assert!(
            matches!(&sent, Err(crate::Error::Query { message, .. })
                if message.contains("export again")),
            "{sent:?}"
        );
        assert!(metas.is_empty() && batches.is_empty());
        assert!(conn.export_shapes.lock().unwrap().is_empty());
        // A failed export's connection is closed: the export done again
        // goes through a new one.
        let (conn, _) = counted_session(&url, Access::ReadOnly).await;
        let (sent, _, batches) = streamed_rows(&conn, &query, &columns).await;
        assert_eq!(sent.unwrap(), 1);
        assert_eq!(
            batches,
            vec![vec![vec![
                Value::Int(1),
                super::super::value(mysql_async::Value::Float(1.234_567_8), "float"),
            ]]]
        );
    }

    /// A view browsed first leaves a shape that tells its cut DOUBLE from
    /// none by its type, and so says it has no FLOAT: its export reads a
    /// shape of its own, and keeps every digit. The view's shape, the
    /// start, the prepare, the run, the rollback.
    #[tokio::test]
    async fn a_browsed_view_keeps_its_digits_in_an_export() {
        let Some(url) = test_url() else {
            return;
        };
        let schema = export_tables(&url).await;
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let object = ObjectRef::new(&schema, "export_pi_view");
        conn.fetch_rows(&RowQuery::new(object.clone(), 10))
            .await
            .unwrap();
        assert!(!conn.known_shape(&object).unwrap().floats);
        let query = RowQuery::new(object, 0);
        let columns = vec!["id".to_owned(), "p".to_owned()];
        let before = asked.load(Ordering::SeqCst);
        let (_, sent, _, batches) = exported_table(&conn, &query, &columns, &StopFlag::new()).await;
        assert_eq!(sent.unwrap(), 12_000);
        assert_eq!(asked.load(Ordering::SeqCst) - before, 5);
        for row in batches.iter().flatten() {
            let Value::Int(id) = row[0] else {
                panic!("{row:?}");
            };
            assert_eq!(row[1], Value::Float(std::f64::consts::PI * id as f64));
        }
        assert!(!in_transaction(&conn).await);
    }

    /// `export_tokens` filtered to the row whose token is the bytes
    /// `0x6162`.
    fn tokens_query(schema: &str) -> (RowQuery, Vec<String>) {
        let mut query = RowQuery::new(ObjectRef::new(schema, "export_tokens"), 0);
        query.filters = vec![crate::Filter {
            column: "token".into(),
            op: crate::FilterOp::Eq,
            value: "0x6162".into(),
        }];
        (query, vec!["id".to_owned(), "token".to_owned()])
    }

    /// A byte string column the shape did not know of fails the export
    /// before anything is sent: the filter built without it compares the
    /// bytes as text, and would find no row. The message asks for the
    /// shape again, which costs no round trip: with the shape as it is,
    /// the export finds the row in two.
    #[tokio::test]
    async fn a_byte_column_the_shape_missed_fails_the_export() {
        let Some(url) = test_url() else {
            return;
        };
        let schema = export_tables(&url).await;
        let (conn, _) = counted_session(&url, Access::ReadOnly).await;
        let (query, columns) = tokens_query(&schema);
        let stale = super::super::Shape {
            key: vec!["id".to_owned()],
            ..Default::default()
        };
        keep_for_export(&conn, &query.object, stale);
        let (sent, metas, batches) = streamed_rows(&conn, &query, &columns).await;
        assert!(
            matches!(&sent, Err(crate::Error::Query { message, .. })
                if message.contains("export again")),
            "{sent:?}"
        );
        assert!(metas.is_empty() && batches.is_empty());
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let before = asked.load(Ordering::SeqCst);
        let (_, sent, _, batches) = exported_table(&conn, &query, &columns, &StopFlag::new()).await;
        assert_eq!(asked.load(Ordering::SeqCst) - before, 2);
        assert_eq!(sent.unwrap(), 1);
        assert_eq!(
            batches,
            vec![vec![vec![
                Value::Int(1),
                Value::Bytes(b"ab".as_slice().into())
            ]]]
        );
        // With no `export_statement` before, the shape is read for the
        // rows: two round trips all the same.
        let before = asked.load(Ordering::SeqCst);
        let (sent, _, again) = streamed_rows(&conn, &query, &columns).await;
        assert_eq!(asked.load(Ordering::SeqCst) - before, 2);
        assert_eq!(sent.unwrap(), 1);
        assert_eq!(again, batches);
        assert!(!in_transaction(&conn).await);
    }

    /// A SQL result's column that loses digits in text, which the driver's
    /// prepare did not have (a view replaced in between), fails the export
    /// before anything is sent: the message refuses any such column.
    #[tokio::test]
    async fn a_cut_column_the_prepare_missed_fails_the_export() {
        let Some(url) = test_url() else {
            return;
        };
        let (conn, _) = counted_session(&url, Access::ReadOnly).await;
        let sql = crate::dialect::Sql {
            text: "SELECT PI() AS p".to_owned(),
            params: Vec::new(),
        };
        let stop = StopFlag::new();
        let (sent, columns, batches) = streamed(|to| {
            let (conn, sql, stop) = (&conn, &sql, &stop);
            async move {
                conn.stream_in_one(sql, &super::Run::Statement, 10, &to, stop)
                    .await
            }
        })
        .await;
        assert!(
            matches!(&sent, Err(crate::Error::Query { message, .. })
                if message == super::CHANGED),
            "{sent:?}"
        );
        assert!(columns.is_empty() && batches.is_empty());
    }
}
