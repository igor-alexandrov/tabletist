//! What an agent's read does on a real database: the Bookshop, made in
//! each engine (`docs/superpowers/plans/2026-10-13-mcp-server.md`, F.2).
//!
//! PostgreSQL and MySQL need their servers (see compose.yaml). Without the
//! variables those tests print "skipped" and pass.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use tabletist_db::agent::{self, Answer, Limits, Param};
use tabletist_db::{
    Access, ConnectSpec, Connection, Dialect, Error, HostKeys, ScriptMode, Secrets,
    StatementOutcome, StopFlag, Value, sql,
};

/// How many rows each table and view of the Bookshop has.
const STOCK: [(&str, i64); 9] = [
    ("publishers", 5),
    ("authors", 6),
    ("books", 12),
    ("book_covers", 3),
    ("customers", 8),
    ("orders", 12),
    ("order_items", 40),
    ("recent_orders", 7),
    ("customer_names", 8),
];

/// What `text` answers when the SQL editor runs it and only reads: the
/// rows of its one statement.
async fn rows_of(connection: &Connection, text: &str, on: &str) -> Vec<Vec<Value>> {
    let statements = sql::statements(connection.dialect(), text);
    let outcome = connection
        .run_script(&statements, 1_000, ScriptMode::ReadOnly, &StopFlag::new())
        .await
        .unwrap_or_else(|error| panic!("{on}: {error}"));
    match outcome
        .results
        .into_iter()
        .next()
        .map(|result| result.outcome)
    {
        Some(StatementOutcome::Rows { rows, .. }) => rows,
        other => panic!("{on}: {text} answered {other:?}"),
    }
}

/// Every table and view of the Bookshop holds what its file gave it: what
/// a test asks before it reads, and again after, when nothing may have
/// changed.
async fn the_bookshop_is_stocked(connection: &Connection, on: &str) {
    for (table, count) in STOCK {
        let rows = rows_of(connection, &format!("SELECT count(*) FROM {table}"), on).await;
        assert_eq!(rows, [[Value::Int(count)]], "{on}: {table}");
    }
}

/// What a test gives a read that should not run out of either.
const LIMITS: Limits = Limits {
    rows: 100,
    timeout: Duration::from_secs(10),
};

/// A read that takes long without sleeping: every way to pick one of the
/// 40 order items `copies` times over, each weighed and none kept. Five
/// copies are a hundred million rows, which no test waits for. Four are
/// under three million, and are weighed to the end.
fn slow(copies: usize) -> String {
    let tables: Vec<String> = (1..=copies)
        .map(|copy| format!("order_items t{copy}"))
        .collect();
    let weights: Vec<String> = (1..=copies)
        .map(|copy| format!("t{copy}.quantity"))
        .collect();
    format!(
        "SELECT count(*) FROM {} WHERE {} < 0",
        tables.join(", "),
        weights.join(" + ")
    )
}

fn text(text: &str) -> Value {
    Value::Text(text.into())
}

/// `sql` with each `?` as the connection's database spells a placeholder:
/// `$1`, `$2` and so on for PostgreSQL.
fn placed(connection: &Connection, sql: &str) -> String {
    match connection.dialect() {
        Dialect::MySql | Dialect::Sqlite => sql.to_owned(),
        Dialect::Postgres => {
            let mut number = 0;
            sql.chars()
                .map(|character| match character {
                    '?' => {
                        number += 1;
                        format!("${number}")
                    }
                    other => other.to_string(),
                })
                .collect()
        }
    }
}

/// What the connection's database answers an agent that sends `sql`,
/// written with `?`, and `params`. The statement must pass the check.
async fn asked(
    connection: &Connection,
    sql: &str,
    params: &[Param],
    limits: Limits,
) -> Result<Answer, Error> {
    let sql = placed(connection, sql);
    let read = agent::check_read(connection.dialect(), &sql, params)
        .unwrap_or_else(|refusal| panic!("{sql}: {refusal}"));
    connection
        .run_agent_read(&read, limits, &StopFlag::new())
        .await
}

/// The rows of an answer that must come.
async fn read(connection: &Connection, sql: &str, params: &[Param], on: &str) -> Vec<Vec<Value>> {
    asked(connection, sql, params, LIMITS)
        .await
        .unwrap_or_else(|error| panic!("{on}: {sql}: {error}"))
        .rows
}

async fn a_read_answers_its_rows(connection: &Connection, on: &str) {
    the_bookshop_is_stocked(connection, on).await;
    // The first rows up to the limit, and whether there were more.
    let books = "SELECT id, title FROM books ORDER BY id";
    let five = Limits { rows: 5, ..LIMITS };
    let answer = asked(connection, books, &[], five).await.unwrap();
    let names: Vec<&str> = answer
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    assert_eq!(names, ["id", "title"], "{on}");
    assert_eq!(answer.rows.len(), 5, "{on}");
    assert_eq!(
        answer.rows[0],
        [Value::Int(1), text("The Salt Road")],
        "{on}"
    );
    assert!(answer.truncated, "{on}");
    let answer = asked(connection, books, &[], LIMITS).await.unwrap();
    assert_eq!(answer.rows.len(), 12, "{on}");
    assert!(!answer.truncated, "{on}");
    // A view is read as a table is.
    let recent = read(connection, "SELECT count(*) FROM recent_orders", &[], on).await;
    assert_eq!(recent, [[Value::Int(7)]], "{on}");
    // A plan is an answer too.
    let plan = read(connection, "EXPLAIN SELECT title FROM books", &[], on).await;
    assert!(!plan.is_empty(), "{on}");
    the_bookshop_is_stocked(connection, on).await;
}

async fn values_are_bound_to_their_placeholders(connection: &Connection, on: &str) {
    let found = read(
        connection,
        "SELECT title FROM books WHERE id = ?",
        &[Param::Int(3)],
        on,
    )
    .await;
    assert_eq!(found, [[text("Lanterns")]], "{on}");
    // Each value goes to its own placeholder, in their order.
    let found = read(
        connection,
        "SELECT title FROM books WHERE publisher_id = ? AND price < ? ORDER BY id",
        &[Param::Int(1), Param::Float(20.0)],
        on,
    )
    .await;
    assert_eq!(found, [[text("The Salt Road")]], "{on}");
    // Text arrives as it was sent, its quote and its letters.
    for (name, id) in [("N. K. O'Dell", 6), ("Zoë Lindqvist", 5)] {
        let found = read(
            connection,
            "SELECT id, name FROM authors WHERE name = ?",
            &[Param::Text(name.into())],
            on,
        )
        .await;
        assert_eq!(found, [[Value::Int(id), text(name)]], "{on}");
    }
    // A value is compared, never read: SQL in one finds no title.
    for value in [
        "x' OR '1'='1",
        "'; DROP TABLE books; --",
        "\\'; DELETE FROM books; --",
        "Lanterns' --",
        "$1",
        "?",
    ] {
        let found = read(
            connection,
            "SELECT count(*) FROM books WHERE title = ?",
            &[Param::Text(value.into())],
            on,
        )
        .await;
        assert_eq!(found, [[Value::Int(0)]], "{on}: {value}");
    }
    // The other kinds of value.
    for (sql, param, count) in [
        (
            "SELECT count(*) FROM books WHERE in_print = ?",
            Param::Bool(false),
            2,
        ),
        (
            "SELECT count(*) FROM books WHERE price > ?",
            Param::Float(21.5),
            5,
        ),
        (
            "SELECT count(*) FROM books WHERE published > ?",
            Param::Null,
            0,
        ),
    ] {
        let found = read(connection, sql, &[param], on).await;
        assert_eq!(found, [[Value::Int(count)]], "{on}: {sql}");
    }
    the_bookshop_is_stocked(connection, on).await;
}

async fn a_read_that_takes_too_long_is_ended_by_the_database(connection: &Connection, on: &str) {
    let short = Limits {
        timeout: Duration::from_millis(200),
        ..LIMITS
    };
    let started = Instant::now();
    let ended = asked(connection, &slow(5), &[], short).await;
    assert_eq!(ended.unwrap_err(), Error::Timeout, "{on}");
    let took = started.elapsed();
    assert!(took < Duration::from_secs(5), "{on}: {took:?}");
    // The session is as it was: the next read has its own time, and
    // answers.
    let found = read(connection, "SELECT count(*) FROM books", &[], on).await;
    assert_eq!(found, [[Value::Int(12)]], "{on}");
    the_bookshop_is_stocked(connection, on).await;
}

async fn a_read_that_is_stopped_or_fails_says_so(connection: &Connection, on: &str) {
    // Stopped before it began.
    let stopped = StopFlag::new();
    stopped.stop();
    let books = agent::check_read(connection.dialect(), "SELECT title FROM books", &[]).unwrap();
    let ended = connection.run_agent_read(&books, LIMITS, &stopped).await;
    assert_eq!(ended.unwrap_err(), Error::Cancelled, "{on}");
    // The database's own error, in its words.
    let error = asked(connection, "SELECT nope FROM books", &[], LIMITS)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Query { .. }), "{on}: {error:?}");
    assert!(error.to_string().contains("nope"), "{on}: {error}");
    // Neither ends the session.
    let found = read(connection, "SELECT count(*) FROM books", &[], on).await;
    assert_eq!(found, [[Value::Int(12)]], "{on}");
    the_bookshop_is_stocked(connection, on).await;
}

async fn a_read_checked_for_another_database_is_not_run(connection: &Connection, on: &str) {
    let other = match connection.dialect() {
        Dialect::Postgres => Dialect::MySql,
        Dialect::MySql => Dialect::Sqlite,
        Dialect::Sqlite => Dialect::Postgres,
    };
    let read = agent::check_read(other, "SELECT title FROM books", &[]).unwrap();
    let error = connection
        .run_agent_read(&read, LIMITS, &StopFlag::new())
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "the statement was checked for another kind of database",
        "{on}"
    );
}

/// What every engine is asked.
async fn an_agent_reads_the_bookshop(connection: &Connection, on: &str) {
    a_read_answers_its_rows(connection, on).await;
    values_are_bound_to_their_placeholders(connection, on).await;
    a_read_that_takes_too_long_is_ended_by_the_database(connection, on).await;
    a_read_that_is_stopped_or_fails_says_so(connection, on).await;
    a_read_checked_for_another_database_is_not_run(connection, on).await;
}

mod sqlite {
    use super::*;

    /// A Bookshop of the test's own, on a session that cannot write.
    pub(super) async fn bookshop() -> (Connection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
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
    async fn the_bookshop_is_stocked_on_sqlite() {
        let (connection, _dir) = bookshop().await;
        assert_eq!(connection.dialect(), Dialect::Sqlite);
        the_bookshop_is_stocked(&connection, "sqlite").await;
    }

    #[tokio::test]
    async fn an_agent_reads_the_bookshop_on_sqlite() {
        let (connection, _dir) = bookshop().await;
        an_agent_reads_the_bookshop(&connection, "sqlite").await;
    }

    #[tokio::test]
    async fn sqlite_takes_each_value_as_its_own_type() {
        let (connection, _dir) = bookshop().await;
        let types = read(
            &connection,
            "SELECT typeof(?), typeof(?), typeof(?), typeof(?), typeof(?)",
            &[
                Param::Null,
                Param::Bool(true),
                Param::Int(5),
                Param::Float(1.5),
                // Text that reads as a number is still text.
                Param::Text("5".into()),
            ],
            "sqlite",
        )
        .await;
        let expected = ["null", "integer", "integer", "real", "text"].map(text);
        assert_eq!(types, [expected]);
        // The ends of a 64-bit integer come back as they went.
        let ends = read(
            &connection,
            "SELECT ?, ?",
            &[Param::Int(i64::MAX), Param::Int(i64::MIN)],
            "sqlite",
        )
        .await;
        assert_eq!(ends, [[Value::Int(i64::MAX), Value::Int(i64::MIN)]]);
    }

    #[tokio::test]
    async fn a_question_mark_in_a_string_is_text_on_sqlite() {
        let (connection, _dir) = bookshop().await;
        let found = read(
            &connection,
            "SELECT 'where?', title FROM books WHERE id = ?",
            &[Param::Int(3)],
            "sqlite",
        )
        .await;
        assert_eq!(found, [[text("where?"), text("Lanterns")]]);
    }

    /// The run's deadline is taken off with the run: the SQL editor's next
    /// script on the session has no time limit of the driver's.
    #[tokio::test]
    async fn a_deadline_ends_with_its_run_on_sqlite() {
        let (connection, _dir) = bookshop().await;
        let short = Limits {
            timeout: Duration::from_millis(1),
            ..LIMITS
        };
        let ended = asked(&connection, &slow(5), &[], short).await;
        assert_eq!(ended.unwrap_err(), Error::Timeout);
        // Four copies take longer than that millisecond, and are weighed
        // to the end.
        let counted = rows_of(&connection, &slow(4), "sqlite").await;
        assert_eq!(counted, [[Value::Int(0)]]);
    }
}

mod postgres {
    use super::*;
    use tabletist_db::TlsMode;
    use tokio::sync::OnceCell;

    /// Loaded once for the tests of this file: they only read it.
    static BOOKSHOP: OnceCell<()> = OnceCell::const_new();

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_PG_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    /// A session that may do anything, on the URL's database or on
    /// `database`.
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

    /// The Bookshop has a database of its own, `bookshop`: its `orders` is
    /// not the driver fixture's.
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

    /// A session on the Bookshop that cannot write, or `None` (test
    /// skipped).
    pub(super) async fn bookshop() -> Option<Connection> {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return None;
        };
        load(&url).await;
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        spec.database = "bookshop".into();
        let connection =
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
                .await
                .unwrap();
        Some(connection)
    }

    #[tokio::test]
    async fn the_bookshop_is_stocked_on_postgres() {
        let Some(connection) = bookshop().await else {
            return;
        };
        assert_eq!(connection.dialect(), Dialect::Postgres);
        the_bookshop_is_stocked(&connection, "postgres").await;
    }
}

mod mysql {
    use super::*;
    use mysql_async::prelude::Queryable;
    use tabletist_db::TlsMode;
    use tokio::sync::OnceCell;

    /// Loaded once for the tests of this file: they only read it.
    static BOOKSHOP: OnceCell<()> = OnceCell::const_new();

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_MYSQL_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    /// A session of the driver's own on the Bookshop, which has a database
    /// of its own, `bookshop`. The server's first start makes it
    /// (compose/mysql-init.sql): the tests' user may not make one.
    async fn admin(url: &str) -> mysql_async::Conn {
        let (server, _database) = url.rsplit_once('/').unwrap();
        let opts =
            mysql_async::Opts::from_url(&format!("{server}/bookshop?prefer_socket=false")).unwrap();
        mysql_async::Conn::new(opts).await.unwrap_or_else(|error| {
            panic!(
                "the test MySQL has no `bookshop` database ({error}). \
                 compose/mysql-init.sql makes it on a new volume only: run its last two \
                 statements as root, or make the volume again"
            )
        })
    }

    async fn load(url: &str) {
        BOOKSHOP
            .get_or_init(|| async {
                let mut conn = admin(url).await;
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

    /// A session on the Bookshop that cannot write, or `None` (test
    /// skipped).
    pub(super) async fn bookshop() -> Option<Connection> {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
            return None;
        };
        load(&url).await;
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        spec.database = "bookshop".into();
        let connection =
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
                .await
                .unwrap();
        Some(connection)
    }

    #[tokio::test]
    async fn the_bookshop_is_stocked_on_mysql() {
        let Some(connection) = bookshop().await else {
            return;
        };
        assert_eq!(connection.dialect(), Dialect::MySql);
        the_bookshop_is_stocked(&connection, "mysql").await;
    }
}
