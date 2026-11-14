//! An export's stream (`Connection::stream_rows`, and
//! `Connection::stream_statement` for a SQL result) on each engine, on the
//! Bookshop. PostgreSQL and MySQL need their servers (see compose.yaml):
//! without the variables those tests print "skipped" and pass.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use tabletist_db::{
    Access, ConnectSpec, Connection, Error, Filter, FilterOp, HostKeys, ObjectRef, RowQuery,
    ScriptMode, Secrets, Sort, SortDir, StopFlag, Streamed, Value, sql,
};

/// What a stream sent: its columns' names, each batch's rows, and the
/// count it returned.
#[derive(Debug)]
struct Got {
    columns: Vec<String>,
    batches: Vec<Vec<Vec<Value>>>,
    sent: u64,
}

impl Got {
    fn rows(&self) -> Vec<Vec<Value>> {
        self.batches.iter().flatten().cloned().collect()
    }
}

/// Streams `query` in batches of `batch` through a channel that holds one
/// batch, as the app's export reads it.
async fn streamed(
    connection: &Connection,
    query: &RowQuery,
    columns: &[&str],
    batch: usize,
) -> Result<Got, Error> {
    let columns: Vec<String> = columns.iter().map(|&name| name.to_owned()).collect();
    let (to, mut from) = tokio::sync::mpsc::channel(1);
    let stop = StopFlag::new();
    let collecting = async move {
        let mut got = (Vec::new(), Vec::new());
        while let Some(streamed) = from.recv().await {
            match streamed {
                Streamed::Columns(metas) => {
                    assert!(got.0.is_empty(), "the columns come once");
                    got.0 = metas.into_iter().map(|meta| meta.name).collect();
                }
                Streamed::Rows(rows) => {
                    assert!(!rows.is_empty(), "no batch is empty");
                    got.1.push(rows);
                }
            }
        }
        got
    };
    let (sent, (names, batches)) = tokio::join!(
        async move {
            let sent = connection
                .stream_rows(query, &columns, batch, &to, &stop)
                .await;
            drop(to);
            sent
        },
        collecting
    );
    Ok(Got {
        columns: names,
        batches,
        sent: sent?,
    })
}

fn books(schema: &str) -> RowQuery {
    RowQuery::new(ObjectRef::new(schema, "books"), 0)
}

fn int(value: &Value) -> i64 {
    match value {
        Value::Int(number) => *number,
        Value::Text(text) => text.parse().unwrap(),
        other => panic!("not a number: {other:?}"),
    }
}

/// Streams `text`, a statement of the person's, as Run again without the
/// limit reads it.
async fn streamed_statement(
    connection: &Connection,
    text: &str,
    batch: usize,
    stop: &StopFlag,
) -> Result<Got, Error> {
    let statements = sql::statements(connection.dialect(), text);
    assert_eq!(statements.len(), 1, "one statement: {text}");
    let statement = statements[0].clone();
    let (to, mut from) = tokio::sync::mpsc::channel(1);
    let collecting = async move {
        let mut got = (Vec::new(), Vec::new());
        while let Some(streamed) = from.recv().await {
            match streamed {
                Streamed::Columns(metas) => {
                    got.0 = metas.into_iter().map(|meta| meta.name).collect();
                }
                Streamed::Rows(rows) => got.1.push(rows),
            }
        }
        got
    };
    let (sent, (columns, batches)) = tokio::join!(
        async move {
            let sent = connection
                .stream_statement(&statement, batch, &to, stop)
                .await;
            drop(to);
            sent
        },
        collecting
    );
    match sent {
        Ok(sent) => Ok(Got {
            columns,
            batches,
            sent,
        }),
        // A statement refused before it ran sent nothing at all.
        Err(error) if refused(&error) => {
            assert!(columns.is_empty() && batches.is_empty(), "{error:?}");
            Err(error)
        }
        Err(error) => Err(error),
    }
}

/// Whether `error` is the stream's refusal of a statement that is no read.
fn refused(error: &Error) -> bool {
    matches!(error, Error::Refused { .. })
        || error
            .to_string()
            .contains("Run again without the limit is only for a statement that reads")
}

/// Run again without the limit (spec 4, S2 and S3): a statement as it was
/// written, every row, in batches; a write refused with nothing sent; a
/// stop.
async fn streams_a_statement(connection: &Connection, on: &str) {
    let got = streamed_statement(
        connection,
        "SELECT id, title FROM books WHERE id > 2 ORDER BY id DESC;",
        4,
        &StopFlag::new(),
    )
    .await
    .unwrap();
    assert_eq!(got.columns, ["id", "title"], "{on}");
    assert_eq!(got.sent, 10, "{on}");
    assert_eq!(got.batches.len(), 3, "{on}: 4, 4 and 2");
    let ids: Vec<i64> = got.rows().iter().map(|row| int(&row[0])).collect();
    assert_eq!(ids, (3..=12).rev().collect::<Vec<_>>(), "{on}");

    let none = streamed_statement(
        connection,
        "SELECT id FROM books WHERE id < 0",
        4,
        &StopFlag::new(),
    )
    .await
    .unwrap();
    assert_eq!((none.sent, none.columns.len()), (0, 1), "{on}");

    for write in [
        "UPDATE books SET title = 'x'",
        "DELETE FROM books",
        "INSERT INTO authors (id, name) VALUES (99, 'Nobody')",
    ] {
        let answer = streamed_statement(connection, write, 4, &StopFlag::new()).await;
        assert!(
            answer.as_ref().is_err_and(refused),
            "{on}: {write} was not refused: {answer:?}"
        );
    }
    let committed = streamed_statement(connection, "COMMIT", 4, &StopFlag::new()).await;
    assert!(
        matches!(committed, Err(Error::Refused { .. })),
        "{on}: {committed:?}"
    );

    let stop = StopFlag::new();
    stop.stop();
    let stopped = streamed_statement(connection, "SELECT id FROM books", 2, &stop).await;
    assert!(
        matches!(stopped, Err(Error::Cancelled)),
        "{on}: {stopped:?}"
    );
    // The session is still there for the next one.
    let again = streamed_statement(connection, "SELECT id FROM books", 100, &StopFlag::new())
        .await
        .unwrap();
    assert_eq!(again.sent, 12, "{on}");
}

/// Test 1 and 2 of the spec, on any engine: every row in the key's order,
/// in batches of at most `batch`; the columns chosen, in their order; the
/// filter and the sort of the tab.
async fn streams_the_books(connection: &Connection, schema: &str, on: &str) {
    let all = streamed(connection, &books(schema), &[], 5).await.unwrap();
    assert_eq!(all.sent, 12, "{on}");
    assert_eq!(all.batches.len(), 3, "{on}: 5, 5 and 2");
    assert!(all.batches.iter().all(|batch| batch.len() <= 5), "{on}");
    assert_eq!(all.columns[0], "id", "{on}");
    let ids: Vec<i64> = all.rows().iter().map(|row| int(&row[0])).collect();
    assert_eq!(ids, (1..=12).collect::<Vec<_>>(), "{on}: the key's order");

    let chosen = streamed(connection, &books(schema), &["title", "id"], 100)
        .await
        .unwrap();
    assert_eq!(chosen.columns, ["title", "id"], "{on}");
    assert_eq!(chosen.batches.len(), 1, "{on}");
    assert_eq!(chosen.rows()[0].len(), 2, "{on}");

    let mut query = books(schema);
    query.filters.push(Filter {
        column: "publisher_id".into(),
        op: FilterOp::Eq,
        value: "1".into(),
    });
    query.sort.push(Sort {
        column: "id".into(),
        dir: SortDir::Desc,
    });
    let narrowed = streamed(connection, &query, &["id"], 100).await.unwrap();
    let ids: Vec<i64> = narrowed.rows().iter().map(|row| int(&row[0])).collect();
    assert!(!ids.is_empty(), "{on}");
    assert!(
        ids.windows(2).all(|pair| pair[0] > pair[1]),
        "{on}: {ids:?}"
    );
    assert_eq!(narrowed.sent, ids.len() as u64, "{on}");
}

/// Test 3 of the spec: the session an export opens cannot write.
async fn the_export_session_cannot_write(connection: &Connection, on: &str) {
    assert_eq!(connection.access(), Access::ReadOnly, "{on}");
    let statements = sql::statements(
        connection.dialect(),
        "INSERT INTO authors (id, name) VALUES (99, 'Nobody')",
    );
    let refused = connection
        .run_script(&statements, 10, ScriptMode::Write, &StopFlag::new())
        .await;
    assert!(matches!(refused, Err(Error::ReadOnly)), "{on}: {refused:?}");
}

/// A stop set before the stream starts ends it as cancelled, and a
/// receiver that is gone does too.
async fn a_stop_ends_the_stream(connection: &Connection, schema: &str, on: &str) {
    let (to, _from) = tokio::sync::mpsc::channel(1);
    let stop = StopFlag::new();
    stop.stop();
    let stopped = connection
        .stream_rows(&books(schema), &[], 2, &to, &stop)
        .await;
    assert!(
        matches!(stopped, Err(Error::Cancelled)),
        "{on}: {stopped:?}"
    );

    let (to, from) = tokio::sync::mpsc::channel(1);
    drop(from);
    let gone = connection
        .stream_rows(&books(schema), &[], 2, &to, &StopFlag::new())
        .await;
    assert!(matches!(gone, Err(Error::Cancelled)), "{on}: {gone:?}");
}

/// Test 2 of the spec on a server: `export_many`'s 200,000 rows in
/// batches of 5,000, the stream never more than a batch ahead of a slow
/// writer, in the key's order.
async fn streams_two_hundred_thousand_rows(connection: &Connection, schema: &str, on: &str) {
    let query = RowQuery::new(ObjectRef::new(schema, "export_many"), 0);
    let (to, mut from) = tokio::sync::mpsc::channel(1);
    let stop = StopFlag::new();
    let checking = async move {
        let (mut rows, mut batches, mut last) = (0usize, 0usize, 0i64);
        while let Some(streamed) = from.recv().await {
            let Streamed::Rows(batch) = streamed else {
                continue;
            };
            assert!(batch.len() <= 5_000, "{on}");
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            assert!(from.len() <= 1, "{on}: more than one batch read ahead");
            assert_eq!(int(&batch[0][0]), last + 1, "{on}");
            last = int(&batch[batch.len() - 1][0]);
            rows += batch.len();
            batches += 1;
        }
        (rows, batches)
    };
    let (sent, (rows, batches)) = tokio::join!(
        async move {
            let sent = connection.stream_rows(&query, &[], 5_000, &to, &stop).await;
            drop(to);
            sent
        },
        checking
    );
    assert_eq!(sent.unwrap(), 200_000, "{on}");
    assert_eq!((rows, batches), (200_000, 40), "{on}");
}

mod sqlite {
    use super::*;

    /// The Bookshop in a new SQLite file, with `copies` more rows in a
    /// table of their own when asked, read-only.
    async fn bookshop(copies: u32) -> (Connection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
        if copies > 0 {
            let file = rusqlite::Connection::open(&path).unwrap();
            file.execute_batch(&format!(
                "CREATE TABLE shelf_copies (id INTEGER PRIMARY KEY, book_id INTEGER, title TEXT);
                 WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < {copies})
                 INSERT INTO shelf_copies SELECT i, 1 + i % 12, 'copy ' || i FROM n;"
            ))
            .unwrap();
        }
        let connection = Connection::connect_with(
            &ConnectSpec::sqlite(&path),
            &Secrets::default(),
            &HostKeys::default(),
            Access::ReadOnly,
        )
        .await
        .unwrap();
        (connection, dir)
    }

    #[tokio::test]
    async fn the_books_stream_on_sqlite() {
        let (connection, _dir) = bookshop(0).await;
        streams_the_books(&connection, "main", "sqlite").await;
    }

    #[tokio::test]
    async fn the_export_session_cannot_write_on_sqlite() {
        let (connection, _dir) = bookshop(0).await;
        the_export_session_cannot_write(&connection, "sqlite").await;
    }

    #[tokio::test]
    async fn a_statement_streams_on_sqlite() {
        let (connection, _dir) = bookshop(0).await;
        streams_a_statement(&connection, "sqlite").await;
    }

    #[tokio::test]
    async fn a_stop_ends_the_stream_on_sqlite() {
        let (connection, _dir) = bookshop(0).await;
        a_stop_ends_the_stream(&connection, "main", "sqlite").await;
    }

    /// Test 2 of the spec: 200,000 rows go through in batches of 5,000,
    /// and the stream reads no further ahead than the batch the channel
    /// holds while the one taken is being written.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn two_hundred_thousand_rows_stream_a_batch_at_a_time() {
        let (connection, _dir) = bookshop(200_000).await;
        let query = RowQuery::new(ObjectRef::new("main", "shelf_copies"), 0);
        let (to, mut from) = tokio::sync::mpsc::channel(1);
        let stop = StopFlag::new();
        let checking = async move {
            let (mut rows, mut batches, mut last) = (0usize, 0usize, 0i64);
            while let Some(streamed) = from.recv().await {
                let Streamed::Rows(batch) = streamed else {
                    continue;
                };
                assert!(batch.len() <= 5_000);
                // A writer that takes its time: the stream must wait.
                tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                assert!(from.len() <= 1, "more than one batch read ahead");
                let first = int(&batch[0][0]);
                assert_eq!(first, last + 1, "in the key's order, none lost");
                last = int(&batch[batch.len() - 1][0]);
                rows += batch.len();
                batches += 1;
            }
            (rows, batches)
        };
        let (sent, (rows, batches)) = tokio::join!(
            async move {
                let sent = connection.stream_rows(&query, &[], 5_000, &to, &stop).await;
                drop(to);
                sent
            },
            checking
        );
        assert_eq!(sent.unwrap(), 200_000);
        assert_eq!((rows, batches), (200_000, 40));
    }
}

mod postgres {
    use super::*;
    use tabletist_db::TlsMode;
    use tokio::sync::OnceCell;

    static BOOKSHOP: OnceCell<()> = OnceCell::const_new();

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_PG_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    async fn admin(url: &str, database: Option<&str>) -> tokio_postgres::Client {
        let mut config: tokio_postgres::Config = url.parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        if let Some(database) = database {
            config.dbname(database);
        }
        let (client, session) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(session);
        client
    }

    /// The Bookshop in its own database, `bookshop`, as `tests/agent.rs`
    /// loads it.
    async fn load(url: &str) {
        BOOKSHOP
            .get_or_init(|| async {
                let server = admin(url, None).await;
                let made = server
                    .query_opt("SELECT 1 FROM pg_database WHERE datname = 'bookshop'", &[])
                    .await
                    .unwrap()
                    .is_some();
                if !made {
                    server
                        .batch_execute("CREATE DATABASE bookshop")
                        .await
                        .unwrap();
                }
                admin(url, Some("bookshop"))
                    .await
                    .batch_execute(tabletist_db::fixtures::BOOKSHOP_POSTGRES_SQL)
                    .await
                    .unwrap();
            })
            .await;
    }

    async fn bookshop() -> Option<Connection> {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return None;
        };
        load(&url).await;
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        spec.database = "bookshop".into();
        Some(
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
                .await
                .unwrap(),
        )
    }

    #[tokio::test]
    async fn the_books_stream_on_postgres() {
        let Some(connection) = bookshop().await else {
            return;
        };
        streams_the_books(&connection, "public", "postgres").await;
    }

    #[tokio::test]
    async fn the_export_session_cannot_write_on_postgres() {
        let Some(connection) = bookshop().await else {
            return;
        };
        the_export_session_cannot_write(&connection, "postgres").await;
        // The server's own rule, not only the app's: a read that would
        // move a sequence is refused inside the export's transaction.
        let mut query = books("public");
        query.raw_where = Some("nextval(pg_get_serial_sequence('price_changes', 'id')) > 0".into());
        let refused = streamed(&connection, &query, &["id"], 5).await;
        match refused {
            Err(Error::Query { message, .. }) => {
                assert!(message.contains("read-only"), "{message}")
            }
            other => panic!("{:?}", other.map(|got| got.sent)),
        }
    }

    #[tokio::test]
    async fn two_hundred_thousand_rows_stream_on_postgres() {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return;
        };
        let Some(connection) = bookshop().await else {
            return;
        };
        let admin = admin(&url, Some("bookshop")).await;
        admin
            .batch_execute(
                "DROP TABLE IF EXISTS export_many;
                 CREATE TABLE export_many (id integer PRIMARY KEY, title text);
                 INSERT INTO export_many SELECT i, 'copy ' || i FROM generate_series(1, 200000) i;",
            )
            .await
            .unwrap();
        streams_two_hundred_thousand_rows(&connection, "public", "postgres").await;
        admin.batch_execute("DROP TABLE export_many").await.unwrap();
    }

    #[tokio::test]
    async fn a_statement_streams_on_postgres() {
        let Some(connection) = bookshop().await else {
            return;
        };
        streams_a_statement(&connection, "postgres").await;
        // A statement no cursor holds fails with the server's words.
        let shown = streamed_statement(&connection, "SHOW search_path", 10, &StopFlag::new()).await;
        let Err(error) = shown else {
            panic!("{shown:?}");
        };
        assert!(error.to_string().contains("syntax error"), "{error}");
        streams_a_statement(&connection, "postgres").await;
    }

    #[tokio::test]
    async fn a_stop_ends_the_stream_on_postgres() {
        let Some(connection) = bookshop().await else {
            return;
        };
        a_stop_ends_the_stream(&connection, "public", "postgres").await;
        // The session is still good for the next export.
        streams_the_books(&connection, "public", "postgres").await;
    }

    /// The server's `statement_timeout` ends a stream as a timeout, not as
    /// a cancel. The first `FETCH` sets it for the transaction (an export
    /// sets none of its own), so the second one runs over it.
    #[tokio::test]
    async fn the_servers_timeout_is_told_apart_on_postgres() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let mut query = books("public");
        query.raw_where = Some(
            "set_config('statement_timeout', '50', true) IS NOT NULL \
             AND pg_sleep(0.2) IS NOT NULL"
                .into(),
        );
        let ended = streamed(&connection, &query, &["id"], 2).await;
        assert!(
            matches!(ended, Err(Error::Timeout)),
            "{:?}",
            ended.map(|got| got.sent)
        );
    }
}

mod mysql {
    use super::*;
    use mysql_async::prelude::Queryable;
    use tabletist_db::TlsMode;
    use tokio::sync::OnceCell;

    static BOOKSHOP: OnceCell<()> = OnceCell::const_new();

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_MYSQL_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    /// The Bookshop's own database, `bookshop`, which the server's first
    /// start makes (compose/mysql-init.sql), loaded as `tests/agent.rs`
    /// loads it.
    async fn load(url: &str) {
        BOOKSHOP
            .get_or_init(|| async {
                let (server, _database) = url.rsplit_once('/').unwrap();
                let opts =
                    mysql_async::Opts::from_url(&format!("{server}/bookshop?prefer_socket=false"))
                        .unwrap();
                let mut conn = mysql_async::Conn::new(opts).await.unwrap();
                for statement in tabletist_db::fixtures::BOOKSHOP_MYSQL_SQL.split(";\n") {
                    let statement: String = statement
                        .lines()
                        .filter(|line| !line.trim_start().starts_with("--"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if !statement.trim().is_empty() {
                        conn.query_drop(statement).await.unwrap();
                    }
                }
                conn.disconnect().await.unwrap();
            })
            .await;
    }

    async fn bookshop() -> Option<Connection> {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
            return None;
        };
        load(&url).await;
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        spec.database = "bookshop".into();
        Some(
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
                .await
                .unwrap(),
        )
    }

    #[tokio::test]
    async fn the_books_stream_on_mysql() {
        let Some(connection) = bookshop().await else {
            return;
        };
        streams_the_books(&connection, "bookshop", "mysql").await;
    }

    #[tokio::test]
    async fn a_statement_streams_on_mysql() {
        let Some(connection) = bookshop().await else {
            return;
        };
        streams_a_statement(&connection, "mysql").await;
    }

    #[tokio::test]
    async fn the_export_session_cannot_write_on_mysql() {
        let Some(connection) = bookshop().await else {
            return;
        };
        the_export_session_cannot_write(&connection, "mysql").await;
    }

    #[tokio::test]
    async fn two_hundred_thousand_rows_stream_on_mysql() {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
            return;
        };
        let Some(connection) = bookshop().await else {
            return;
        };
        let (server, _database) = url.rsplit_once('/').unwrap();
        let opts =
            mysql_async::Opts::from_url(&format!("{server}/bookshop?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
        for statement in [
            "DROP TABLE IF EXISTS export_many",
            "CREATE TABLE export_many (id integer PRIMARY KEY, title text)",
            "INSERT INTO export_many \
             WITH RECURSIVE d (n) AS (SELECT 0 UNION ALL SELECT n + 1 FROM d WHERE n < 9) \
             SELECT 1 + a.n + 10 * b.n + 100 * c.n + 1000 * e.n + 10000 * f.n + 100000 * g.n, 'copy' \
             FROM d a, d b, d c, d e, d f, d g WHERE g.n < 2",
        ] {
            admin.query_drop(statement).await.unwrap();
        }
        streams_two_hundred_thousand_rows(&connection, "bookshop", "mysql").await;
        admin.query_drop("DROP TABLE export_many").await.unwrap();
        admin.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn a_stop_ends_the_stream_on_mysql() {
        let Some(connection) = bookshop().await else {
            return;
        };
        a_stop_ends_the_stream(&connection, "bookshop", "mysql").await;
    }
}
