//! What an agent's read does on a real database: the Bookshop, made in
//! each engine (`docs/superpowers/plans/2026-10-13-mcp-server.md`, F.2).
//!
//! PostgreSQL and MySQL need their servers (see compose.yaml). Without the
//! variables those tests print "skipped" and pass.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use tabletist_db::{
    Access, ConnectSpec, Connection, Dialect, HostKeys, ScriptMode, Secrets, StatementOutcome,
    StopFlag, Value, sql,
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
