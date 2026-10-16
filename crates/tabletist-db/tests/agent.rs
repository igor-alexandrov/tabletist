//! What an agent's read does on a real database: the Bookshop, made in
//! each engine (`docs/superpowers/plans/2026-10-13-mcp-server.md`, F.2).
//!
//! PostgreSQL and MySQL need their servers (see compose.yaml). Without the
//! variables those tests print "skipped" and pass.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use tabletist_db::agent::{self, Answer, Limits, Param, Refusal, TableName, Tables};
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
        .run_agent_read(&read, limits, &Tables::All, &StopFlag::new())
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
    let ended = connection
        .run_agent_read(&books, LIMITS, &Tables::All, &stopped)
        .await;
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
        .run_agent_read(&read, LIMITS, &Tables::All, &StopFlag::new())
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "the statement was checked for another kind of database",
        "{on}"
    );
}

/// A table the person names, in every schema.
fn named(name: &str) -> TableName {
    TableName {
        schema: None,
        name: name.to_owned(),
    }
}

/// What the connection's database answers an agent that sends `sql` on a
/// connection that hides as `tables` says: its rows, or why not.
async fn hiding(connection: &Connection, tables: &Tables, sql: &str) -> Result<Answer, Error> {
    let read = agent::check_read(connection.dialect(), sql, &[])
        .unwrap_or_else(|refusal| panic!("{sql}: {refusal}"));
    connection
        .run_agent_read(&read, LIMITS, tables, &StopFlag::new())
        .await
}

/// Refused as hidden: `name`, read for `through` where that is given.
fn hidden_refusal(name: &str, through: Option<&str>) -> Error {
    Error::Agent(Refusal::TableHidden {
        name: name.to_owned(),
        through: through.map(str::to_owned),
    })
}

/// The hidden-table rows of the plan's F.2 (B.3), on the Bookshop.
async fn a_hidden_table_is_never_read(connection: &Connection, on: &str) {
    let customers = Tables::AllExcept(vec![named("customers")]);
    let count = |answer: Result<Answer, Error>| answer.map(|answer| answer.rows);
    // What is not hidden reads as before, views over it too.
    for (sql, rows) in [
        ("SELECT count(*) FROM books", 12),
        ("SELECT count(*) FROM recent_orders", 7),
    ] {
        let found = count(hiding(connection, &customers, sql).await);
        assert_eq!(found, Ok(vec![vec![Value::Int(rows)]]), "{on}: {sql}");
    }
    // The hidden table, by every way to it the text gives: by name, in
    // another case and in quotes, counted, behind EXISTS, in a common
    // table expression, and through a view.
    let quoted = match connection.dialect() {
        Dialect::MySql => "SELECT * FROM `CUSTOMERS`",
        Dialect::Postgres | Dialect::Sqlite => "SELECT * FROM \"CUSTOMERS\"",
    };
    for sql in [
        "SELECT * FROM customers",
        "SELECT * FROM Customers c",
        quoted,
        "SELECT count(*) FROM customers",
        "SELECT title FROM books WHERE EXISTS (SELECT 1 FROM customers)",
        "WITH c AS (SELECT * FROM customers) SELECT * FROM c",
        "WITH customers AS (SELECT 1 AS id) SELECT * FROM customers",
    ] {
        let error = hiding(connection, &customers, sql).await.unwrap_err();
        assert_eq!(error, hidden_refusal("customers", None), "{on}: {sql}");
    }
    let through = hiding(connection, &customers, "SELECT * FROM customer_names").await;
    assert_eq!(
        through.unwrap_err(),
        hidden_refusal("customers", Some("customer_names")),
        "{on}"
    );
    // In a join in parentheses, where the name is not right after the
    // FROM: read off the text, and by the database's own check besides.
    let nested = "SELECT * FROM (customers JOIN books ON true)";
    let error = hiding(connection, &customers, nested).await.unwrap_err();
    assert!(
        matches!(&error, Error::Agent(Refusal::TableHidden { name, .. }) if name == "customers"),
        "{on}: {error:?}"
    );
    // The system catalogs describe every table: closed while one is
    // hidden, open where none is.
    let catalog = match connection.dialect() {
        Dialect::Sqlite => "SELECT count(*) FROM sqlite_master",
        Dialect::Postgres | Dialect::MySql => "SELECT count(*) FROM information_schema.tables",
    };
    let error = hiding(connection, &customers, catalog).await.unwrap_err();
    assert_eq!(error, Error::Agent(Refusal::SystemCatalog), "{on}");
    assert!(
        hiding(connection, &Tables::All, catalog).await.is_ok(),
        "{on}"
    );
    // Only a list: the rest is hidden.
    let only_books = Tables::Only(vec![named("books")]);
    let found = count(hiding(connection, &only_books, "SELECT count(*) FROM books").await);
    assert_eq!(found, Ok(vec![vec![Value::Int(12)]]), "{on}");
    let error = hiding(connection, &only_books, "SELECT * FROM orders")
        .await
        .unwrap_err();
    assert_eq!(error, hidden_refusal("orders", None), "{on}");
    // A derived table's columns are no tables of their own.
    let derived = "SELECT count(*) FROM (SELECT id FROM books GROUP BY id) d";
    let found = count(hiding(connection, &only_books, derived).await);
    assert_eq!(found, Ok(vec![vec![Value::Int(12)]]), "{on}");
    // A function of the database's own, which reads what no check sees:
    // refused while anything is hidden, and so is a view that calls it.
    // The Bookshop has one on PostgreSQL alone (its fixture says why).
    match connection.dialect() {
        Dialect::Postgres => {
            for sql in [
                "SELECT customer_count()",
                "SELECT title FROM books WHERE id < customer_count()",
            ] {
                let error = hiding(connection, &customers, sql).await.unwrap_err();
                let refused = Error::Agent(Refusal::FunctionNotAllowed("customer_count".into()));
                assert_eq!(error, refused, "{on}: {sql}");
            }
            let tally = "SELECT * FROM customer_tally";
            let error = hiding(connection, &customers, tally).await.unwrap_err();
            assert_eq!(error, hidden_refusal("customer_tally", None), "{on}");
            let found = count(hiding(connection, &Tables::All, "SELECT customer_count()").await);
            assert_eq!(found, Ok(vec![vec![Value::Int(8)]]), "{on}");
            let found = count(hiding(connection, &Tables::All, tally).await);
            assert_eq!(found, Ok(vec![vec![Value::Int(8)]]), "{on}");
            // A function of a row, called as its column.
            for sql in [
                "SELECT b.book_reach FROM books b",
                "SELECT (b).book_reach FROM books b",
            ] {
                let error = hiding(connection, &customers, sql).await.unwrap_err();
                let refused = Error::Agent(Refusal::FunctionNotAllowed("book_reach".into()));
                assert_eq!(error, refused, "{on}: {sql}");
            }
            let reach = "SELECT max(b.book_reach) FROM books b";
            let found = count(hiding(connection, &Tables::All, reach).await);
            assert_eq!(found, Ok(vec![vec![Value::Int(8)]]), "{on}");
        }
        Dialect::MySql => {
            // An index hint's FOR, and an alias after it.
            let hinted = "SELECT * FROM books FORCE INDEX FOR JOIN (PRIMARY), customers c";
            let error = hiding(connection, &customers, hinted).await.unwrap_err();
            assert_eq!(error, hidden_refusal("customers", None), "{on}");
        }
        Dialect::Sqlite => {}
    }
    // Nothing hidden: the table reads.
    let found = count(hiding(connection, &Tables::All, "SELECT count(*) FROM customers").await);
    assert_eq!(found, Ok(vec![vec![Value::Int(8)]]), "{on}");
    the_bookshop_is_stocked(connection, on).await;
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
        bookshop_with("").await
    }

    /// A Bookshop to which `more` was added before its session opened.
    async fn bookshop_with(more: &str) -> (Connection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(more)
            .unwrap();
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
    async fn a_hidden_table_is_never_read_on_sqlite() {
        let (connection, _dir) = bookshop().await;
        a_hidden_table_is_never_read(&connection, "sqlite").await;
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

    /// The check reads an agent's text, and a view's text is the
    /// database's. SQLite reads both, and with the session's schema not
    /// trusted it refuses a view that reads a pragma or calls a function
    /// that is not harmless, for an agent as for anyone. So a view is no
    /// way around what an agent's own statement may not do.
    #[tokio::test]
    async fn what_a_view_may_not_do_is_sqlites_to_refuse() {
        let (connection, _dir) = bookshop_with(
            "CREATE VIEW customer_columns AS SELECT name FROM pragma_table_info('customers');
             CREATE VIEW tokenizers AS SELECT fts3_tokenizer('simple') AS found;",
        )
        .await;
        for (sql, what) in [
            ("SELECT name FROM customer_columns", "pragma_table_info"),
            ("SELECT found FROM tokenizers", "fts3_tokenizer"),
            (
                "SELECT title FROM books WHERE id IN (SELECT count(*) FROM customer_columns)",
                "pragma_table_info",
            ),
        ] {
            // The text names a view and nothing else, so the check passes.
            let error = asked(&connection, sql, &[], LIMITS).await.unwrap_err();
            let said = error.to_string();
            assert!(said.starts_with("unsafe use of "), "{sql}: {said}");
            assert!(said.contains(what), "{sql}: {said}");
        }
        // The session reads on, and nothing changed.
        the_bookshop_is_stocked(&connection, "sqlite").await;
    }

    /// The fence lists what a read does. SQLite's own full-text and R*Tree
    /// tables do more than read columns when they are first opened, and
    /// are read all the same.
    #[tokio::test]
    async fn sqlites_own_virtual_tables_are_read_for_an_agent() {
        let (connection, _dir) = bookshop_with(
            "CREATE VIRTUAL TABLE blurbs USING fts5(body);
             INSERT INTO blurbs VALUES ('A salt road to the sea'), ('Notes on bread');
             CREATE VIRTUAL TABLE shelves USING rtree(id, left, right);
             INSERT INTO shelves VALUES (1, 0, 2), (2, 5, 9);",
        )
        .await;
        let found = read(
            &connection,
            "SELECT body FROM blurbs WHERE blurbs MATCH ?",
            &[Param::Text("salt".into())],
            "sqlite",
        )
        .await;
        assert_eq!(found, [[text("A salt road to the sea")]]);
        let found = read(
            &connection,
            "SELECT id FROM shelves WHERE left >= ? ORDER BY id",
            &[Param::Int(1)],
            "sqlite",
        )
        .await;
        assert_eq!(found, [[Value::Int(2)]]);
    }

    /// What an ordinary read asks of SQLite beside its tables' columns is
    /// on the fence's list.
    #[tokio::test]
    async fn the_shapes_of_a_read_pass_sqlites_own_reading() {
        let (connection, _dir) = bookshop().await;
        for (sql, rows) in [
            // A view, and an aggregate over a join.
            ("SELECT email FROM recent_orders", 7),
            (
                "SELECT a.name, count(*), max(b.price) FROM books b \
                 JOIN authors a ON a.id = b.author_id GROUP BY a.name",
                6,
            ),
            (
                "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 3) \
                 SELECT i FROM n",
                3,
            ),
            ("SELECT value FROM json_each('[1, 2]')", 2),
            (
                "SELECT title FROM books WHERE title LIKE 'The %' ORDER BY title COLLATE NOCASE",
                3,
            ),
            (
                "SELECT lower(title), length(title), date(published) FROM books WHERE id = 3",
                1,
            ),
            ("EXPLAIN QUERY PLAN SELECT title FROM books", 1),
        ] {
            let answered = read(&connection, sql, &[], "sqlite").await;
            assert_eq!(answered.len(), rows, "{sql}");
        }
    }

    /// SQLite does not ask the progress handler while it waits for another
    /// connection's lock on the file, so the run holds that wait to its own
    /// time too: a read behind a writer is ended when its time is up, and
    /// not when the session's five seconds are.
    #[tokio::test]
    async fn the_deadline_ends_a_wait_for_a_lock_on_sqlite() {
        let (connection, dir) = bookshop().await;
        let holder = rusqlite::Connection::open(dir.path().join("bookshop.db")).unwrap();
        holder.execute_batch("BEGIN EXCLUSIVE").unwrap();
        let short = Limits {
            timeout: Duration::from_millis(200),
            ..LIMITS
        };
        let started = Instant::now();
        let ended = asked(&connection, "SELECT count(*) FROM book_covers", &[], short).await;
        let took = started.elapsed();
        holder.execute_batch("ROLLBACK").unwrap();
        assert_eq!(ended.unwrap_err(), Error::Timeout);
        assert!(took < Duration::from_secs(3), "{took:?}");
        // The lock is gone, and the session reads on.
        let covers = read(
            &connection,
            "SELECT count(*) FROM book_covers",
            &[],
            "sqlite",
        )
        .await;
        assert_eq!(covers, [[Value::Int(3)]]);
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

    #[tokio::test]
    async fn an_agent_reads_the_bookshop_on_postgres() {
        let Some(connection) = bookshop().await else {
            return;
        };
        an_agent_reads_the_bookshop(&connection, "postgres").await;
    }

    #[tokio::test]
    async fn a_hidden_table_is_never_read_on_postgres() {
        let Some(connection) = bookshop().await else {
            return;
        };
        a_hidden_table_is_never_read(&connection, "postgres").await;
    }

    #[tokio::test]
    async fn postgres_reads_a_value_as_the_type_of_its_placeholder() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let on = "postgres";
        // The ends of a 64-bit integer come back as they went.
        let ends = read(
            &connection,
            "SELECT ?::bigint, ?::bigint",
            &[Param::Int(i64::MAX), Param::Int(i64::MIN)],
            on,
        )
        .await;
        assert_eq!(ends, [[Value::Int(i64::MAX), Value::Int(i64::MIN)]]);
        // A placeholder with no type to take is text.
        let sent = read(&connection, "SELECT ? AS sent", &[Param::Int(42)], on).await;
        assert_eq!(sent, [[text("42")]]);
        // Text fills a placeholder of any type the server reads from text:
        // an array here.
        let found = read(
            &connection,
            "SELECT count(*) FROM books WHERE id = ANY(?)",
            &[Param::Text("{1, 2, 3}".into())],
            on,
        )
        .await;
        assert_eq!(found, [[Value::Int(3)]]);
        // One value for a placeholder written twice.
        let twice = "SELECT title FROM books WHERE id = $1 OR author_id = $1 ORDER BY id";
        let read_twice = agent::check_read(Dialect::Postgres, twice, &[Param::Int(3)]).unwrap();
        let found = connection
            .run_agent_read(&read_twice, LIMITS, &Tables::All, &StopFlag::new())
            .await
            .unwrap();
        assert_eq!(found.rows, [[text("Lanterns")], [text("Ninety-One Doors")]]);
        // A value its placeholder's type does not take is the server's
        // error, and the session goes on.
        let error = asked(
            &connection,
            "SELECT title FROM books WHERE id = ?",
            &[Param::Text("three".into())],
            LIMITS,
        )
        .await
        .unwrap_err();
        let Error::Query { code, .. } = &error else {
            panic!("{error:?}");
        };
        assert_eq!(code.as_deref(), Some("22P02"), "{error}");
        the_bookshop_is_stocked(&connection, on).await;
    }

    #[tokio::test]
    async fn a_placeholder_in_a_string_is_text_on_postgres() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let sql = "SELECT '$1', $q$ $1 $q$, title FROM books WHERE id = $1";
        let read = agent::check_read(Dialect::Postgres, sql, &[Param::Int(3)]).unwrap();
        let found = connection
            .run_agent_read(&read, LIMITS, &Tables::All, &StopFlag::new())
            .await
            .unwrap();
        assert_eq!(found.rows, [[text("$1"), text(" $1 "), text("Lanterns")]]);
    }

    #[tokio::test]
    async fn a_plan_takes_values_on_postgres() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let on = "postgres";
        let one = [Param::Int(3)];
        let plan = read(
            &connection,
            "EXPLAIN SELECT title FROM books WHERE id = ?",
            &one,
            on,
        )
        .await;
        let Some(Value::Text(first)) = plan.first().and_then(|line| line.first()) else {
            panic!("{plan:?}");
        };
        assert!(first.contains("books"), "{first}");
        // A plan as JSON is one value of another type.
        let plan = read(
            &connection,
            "EXPLAIN (FORMAT JSON) SELECT title FROM books WHERE id = ?",
            &one,
            on,
        )
        .await;
        let [line] = plan.as_slice() else {
            panic!("{plan:?}");
        };
        let [Value::Text(json)] = line.as_slice() else {
            panic!("{line:?}");
        };
        assert!(json.trim_start().starts_with('['), "{json}");
        // A long plan is cut at the limit like any answer.
        let cut = asked(
            &connection,
            "EXPLAIN (VERBOSE) SELECT * FROM books b JOIN authors a ON a.id = b.author_id \
             WHERE b.id = ?",
            &one,
            Limits { rows: 2, ..LIMITS },
        )
        .await
        .unwrap();
        assert_eq!(cut.rows.len(), 2);
        assert!(cut.truncated);
    }

    /// A statement's time includes its wait for a lock, so the one timeout
    /// ends a read that waits behind another session's `ALTER TABLE`.
    #[tokio::test]
    async fn the_timeout_ends_a_wait_for_a_lock_on_postgres() {
        let (Some(connection), Some(url)) = (bookshop().await, url()) else {
            return;
        };
        let holder = admin(&url, Some("bookshop")).await;
        holder
            .batch_execute("BEGIN; LOCK TABLE book_covers IN ACCESS EXCLUSIVE MODE")
            .await
            .unwrap();
        let short = Limits {
            timeout: Duration::from_millis(200),
            ..LIMITS
        };
        let started = Instant::now();
        let ended = asked(&connection, "SELECT count(*) FROM book_covers", &[], short).await;
        let took = started.elapsed();
        holder.batch_execute("ROLLBACK").await.unwrap();
        assert_eq!(ended.unwrap_err(), Error::Timeout);
        assert!(took < Duration::from_secs(5), "{took:?}");
        // The lock is gone, and the session reads on.
        let covers = read(
            &connection,
            "SELECT count(*) FROM book_covers",
            &[],
            "postgres",
        )
        .await;
        assert_eq!(covers, [[Value::Int(3)]]);
    }

    /// The timeout is the transaction's: after a run, one that ran out of
    /// time or not, the session has none, as the SQL editor expects.
    #[tokio::test]
    async fn the_timeout_ends_with_its_run_on_postgres() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let on = "postgres";
        let none = [[text("0")]];
        read(&connection, "SELECT count(*) FROM books", &[], on).await;
        assert_eq!(
            rows_of(&connection, "SHOW statement_timeout", on).await,
            none
        );
        let short = Limits {
            timeout: Duration::from_millis(50),
            ..LIMITS
        };
        let ended = asked(&connection, &slow(5), &[], short).await;
        assert_eq!(ended.unwrap_err(), Error::Timeout);
        assert_eq!(
            rows_of(&connection, "SHOW statement_timeout", on).await,
            none
        );
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

    #[tokio::test]
    async fn an_agent_reads_the_bookshop_on_mysql() {
        let Some(connection) = bookshop().await else {
            return;
        };
        an_agent_reads_the_bookshop(&connection, "mysql").await;
    }

    #[tokio::test]
    async fn a_hidden_table_is_never_read_on_mysql() {
        let Some(connection) = bookshop().await else {
            return;
        };
        a_hidden_table_is_never_read(&connection, "mysql").await;
    }

    #[tokio::test]
    async fn mysql_takes_each_value_as_its_own_type() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let on = "mysql";
        // The ends of a 64-bit integer come back as they went.
        let ends = read(
            &connection,
            "SELECT ?, ?",
            &[Param::Int(i64::MAX), Param::Int(i64::MIN)],
            on,
        )
        .await;
        assert_eq!(ends, [[Value::Int(i64::MAX), Value::Int(i64::MIN)]]);
        // Text that reads as a number is still text: no row has this id.
        let found = read(
            &connection,
            "SELECT count(*) FROM books WHERE title = ?",
            &[Param::Text("3".into())],
            on,
        )
        .await;
        assert_eq!(found, [[Value::Int(0)]]);
        // A backslash is a character of the value, whatever the server's
        // SQL mode makes of one in a literal.
        let sent = "a\\b'c\"d";
        let back = read(&connection, "SELECT ?", &[Param::Text(sent.into())], on).await;
        assert_eq!(back, [[text(sent)]]);
    }

    #[tokio::test]
    async fn a_question_mark_in_a_string_is_text_on_mysql() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let found = read(
            &connection,
            "SELECT 'where?', title FROM books WHERE id = ?",
            &[Param::Int(3)],
            "mysql",
        )
        .await;
        assert_eq!(found, [[text("where?"), text("Lanterns")]]);
    }

    #[tokio::test]
    async fn a_plan_takes_values_on_mysql() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let plan = read(
            &connection,
            "EXPLAIN SELECT title FROM books WHERE id = ?",
            &[Param::Int(3)],
            "mysql",
        )
        .await;
        assert!(!plan.is_empty());
    }

    /// MySQL holds a `SELECT` to its time limit and nothing else. Every
    /// shape of read is one to it, wherever the slow part stands.
    #[tokio::test]
    async fn the_time_limit_holds_a_read_of_any_shape_on_mysql() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let slow = slow(5);
        let short = Limits {
            timeout: Duration::from_millis(100),
            ..LIMITS
        };
        for sql in [
            format!("WITH weighed AS ({slow}) SELECT * FROM weighed"),
            format!("SELECT * FROM ({slow}) weighed"),
            format!("SELECT 1 UNION ALL ({slow})"),
            format!("SELECT title, ({slow}) FROM books"),
            format!("SELECT title FROM books WHERE id > ({slow})"),
        ] {
            let ended = asked(&connection, &sql, &[], short).await;
            assert_eq!(ended.unwrap_err(), Error::Timeout, "{sql}");
        }
        the_bookshop_is_stocked(&connection, "mysql").await;
    }

    /// A read that waits behind another session's lock on a table is ended
    /// by the same limit: MySQL's timer ends the wait as it ends the run.
    #[tokio::test]
    async fn the_time_limit_ends_a_wait_for_a_lock_on_mysql() {
        let (Some(connection), Some(url)) = (bookshop().await, url()) else {
            return;
        };
        let mut holder = admin(&url).await;
        holder
            .query_drop("LOCK TABLES book_covers WRITE")
            .await
            .unwrap();
        let short = Limits {
            timeout: Duration::from_millis(200),
            ..LIMITS
        };
        let started = Instant::now();
        let ended = asked(&connection, "SELECT count(*) FROM book_covers", &[], short).await;
        let took = started.elapsed();
        holder.query_drop("UNLOCK TABLES").await.unwrap();
        holder.disconnect().await.unwrap();
        assert_eq!(ended.unwrap_err(), Error::Timeout);
        assert!(took < Duration::from_secs(5), "{took:?}");
        // The lock is gone, and the session reads on.
        let covers = read(
            &connection,
            "SELECT count(*) FROM book_covers",
            &[],
            "mysql",
        )
        .await;
        assert_eq!(covers, [[Value::Int(3)]]);
    }

    /// The time limit is the session's, and the reset that ends a run
    /// takes it away: after a run, one that ran out of time or not, the
    /// session has none, as the SQL editor expects.
    #[tokio::test]
    async fn the_time_limit_ends_with_its_run_on_mysql() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let on = "mysql";
        let limit = "SELECT @@session.max_execution_time";
        let none = [[Value::Int(0)]];
        read(&connection, "SELECT count(*) FROM books", &[], on).await;
        assert_eq!(rows_of(&connection, limit, on).await, none);
        let short = Limits {
            timeout: Duration::from_millis(50),
            ..LIMITS
        };
        let ended = asked(&connection, &slow(5), &[], short).await;
        assert_eq!(ended.unwrap_err(), Error::Timeout);
        assert_eq!(rows_of(&connection, limit, on).await, none);
    }
}
