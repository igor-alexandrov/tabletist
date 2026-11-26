//! Saved connections attached to a Files connection, through the public
//! `Connection` API: the Bookshop's exports joined to the Bookshop's own
//! database, read-only, under the name SQL calls it by. The SQLite
//! Bookshop always; PostgreSQL's and MySQL's with TABLETIST_TEST_PG_URL
//! and TABLETIST_TEST_MYSQL_URL set (their `bookshop` databases).
//!
//! DuckDB downloads its PostgreSQL, MySQL and SQLite readers the first time
//! a test needs them.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use tabletist_db::files::{Attaching, Source};
use tabletist_db::{
    ConnectSpec, Connection, Dialect, Error, HostKeys, ObjectRef, RowQuery, ScriptMode, Secrets,
    StatementOutcome, StopFlag, TlsMode, Value, sql,
};

/// The Bookshop's exports in a folder of their own.
fn exports() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
    dir
}

/// A session over `dir`'s files with `attaching` attached.
async fn open(dir: &tempfile::TempDir, attaching: &[Attaching]) -> Result<Connection, Error> {
    let spec = ConnectSpec::files(Source::Folder(dir.path().to_owned()));
    Connection::connect_files(&spec, attaching, &HostKeys::default(), None).await
}

/// The SQLite Bookshop in `dir`, attached as `shop`.
fn sqlite_shop(dir: &tempfile::TempDir) -> Attaching {
    let path = dir.path().join("shop").with_extension("sqlite3");
    tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
    Attaching {
        name: "shop".into(),
        spec: ConnectSpec::sqlite(&path),
        secrets: Secrets::default(),
    }
}

/// Runs `text` as the SQL editor does: split, then one read-only script.
async fn script(connection: &Connection, text: &str) -> Result<Vec<StatementOutcome>, Error> {
    let statements = sql::statements(Dialect::DuckDb, text);
    let outcome = connection
        .run_script(&statements, 100, ScriptMode::ReadOnly, &StopFlag::new())
        .await?;
    Ok(outcome.results.into_iter().map(|r| r.outcome).collect())
}

async fn rows(connection: &Connection, text: &str) -> Vec<Vec<Value>> {
    match script(connection, text).await.unwrap().remove(0) {
        StatementOutcome::Rows { rows, .. } => rows,
        other => panic!("{text}: {other:?}"),
    }
}

async fn refused(connection: &Connection, text: &str) -> String {
    match script(connection, text).await {
        Err(error) => error.to_string(),
        Ok(mut outcomes) => match outcomes.remove(0) {
            StatementOutcome::Error { error, .. } => error.to_string(),
            other => panic!("{text} ran: {other:?}"),
        },
    }
}

/// Every line of text the session shows of what it attached: the
/// databases' connection strings, the secrets, and each attached table's
/// statement.
async fn shown(connection: &Connection) -> String {
    let mut said = Vec::new();
    for query in [
        "SELECT coalesce(path, '') FROM duckdb_databases()",
        "SELECT secret_string FROM duckdb_secrets()",
        "SELECT coalesce(sql, '') FROM duckdb_tables()",
        "SELECT coalesce(sql, '') FROM duckdb_views()",
    ] {
        for row in rows(connection, query).await {
            said.push(format!("{row:?}"));
        }
    }
    said.join("\n")
}

#[tokio::test]
async fn a_sqlite_bookshop_is_a_schema_the_files_join() {
    let dir = exports();
    let shop = sqlite_shop(&dir);
    let connection = open(&dir, &[shop]).await.unwrap();
    assert_eq!(
        connection.list_schemas().await.unwrap(),
        ["main", "bookshop", "shop"]
    );
    let objects = connection.list_objects("shop").await.unwrap();
    assert!(
        objects.iter().any(|object| object.name == "books"),
        "{objects:?}"
    );
    let books = ObjectRef::new("shop", "books");
    let structure = connection.describe(&books).await.unwrap();
    assert_eq!(structure.columns[1].name, "title");
    let page = connection
        .fetch_rows(&RowQuery::new(books.clone(), 5))
        .await
        .unwrap();
    assert_eq!(page.rows.len(), 5);
    assert_eq!(
        connection
            .count_rows(&RowQuery::new(books, 100))
            .await
            .unwrap(),
        12
    );
    // A file's table and the attached database's, in one query.
    let joined = rows(
        &connection,
        "SELECT count(*) FROM bookshop.order_items i JOIN shop.books b ON b.id = i.book_id",
    )
    .await;
    assert!(
        matches!(joined.as_slice(), [row] if matches!(row.as_slice(), [Value::Int(count)] if *count > 0)),
        "{joined:?}"
    );
}

#[tokio::test]
async fn an_attached_connection_is_read_only() {
    let dir = exports();
    let shop = sqlite_shop(&dir);
    let connection = open(&dir, &[shop]).await.unwrap();
    for statement in [
        "CREATE TABLE shop.probe (id INTEGER)",
        "INSERT INTO shop.books (id, title) VALUES (99, 'x')",
        "DELETE FROM shop.books",
    ] {
        let words = refused(&connection, statement).await;
        assert!(words.contains("read-only"), "{statement}: {words}");
    }
    // DuckDB itself holds it read-only, whatever a script's transaction.
    assert_eq!(
        rows(
            &connection,
            "SELECT readonly FROM duckdb_databases() WHERE database_name = 'shop'"
        )
        .await,
        [[Value::Bool(true)]]
    );
    // Nor does a script let it go, or attach another.
    for statement in ["DETACH shop", "ATTACH ':memory:' AS other"] {
        assert!(
            !refused(&connection, statement).await.is_empty(),
            "{statement}"
        );
    }
    assert_eq!(
        rows(&connection, "SELECT count(*) FROM shop.books").await,
        [[Value::Int(12)]]
    );
}

#[tokio::test]
async fn an_attach_that_fails_says_which() {
    let dir = exports();
    let gone = Attaching {
        name: "shop".into(),
        spec: ConnectSpec::sqlite(dir.path().join("gone.sqlite3")),
        secrets: Secrets::default(),
    };
    let Err(error) = open(&dir, &[gone]).await else {
        panic!("a gone SQLite file was attached");
    };
    assert!(error.to_string().contains("Can't attach shop"), "{error}");
}

/// The test server's Bookshop: its `bookshop` database, the URL's login.
fn server(variable: &str) -> Option<(ConnectSpec, Secrets)> {
    let url = std::env::var(variable)
        .ok()
        .filter(|url| !url.trim().is_empty());
    let Some(url) = url else {
        eprintln!("skipped: {variable} is not set");
        return None;
    };
    let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
    spec.database = "bookshop".into();
    spec.tls = TlsMode::Disable;
    Some((spec, secrets))
}

/// What every server is asked: the Bookshop joined, its password nowhere
/// the session shows, each of `writes` refused (one that starts `then ` is
/// a step run first, which may run) and none landed, as `landed` counts,
/// and a wrong password said without it.
async fn a_server_bookshop_is_attached(
    spec: ConnectSpec,
    secrets: Secrets,
    writes: &[&str],
    landed: &str,
) {
    let dir = exports();
    let password = secrets.password.clone().unwrap();
    let attaching = Attaching {
        name: "shop".into(),
        spec: spec.clone(),
        secrets: secrets.clone(),
    };
    let connection = open(&dir, &[attaching]).await.unwrap();
    assert!(
        connection
            .list_objects("shop")
            .await
            .unwrap()
            .iter()
            .any(|object| object.name == "books")
    );
    let books = ObjectRef::new("shop", "books");
    assert_eq!(
        connection
            .count_rows(&RowQuery::new(books.clone(), 100))
            .await
            .unwrap(),
        12
    );
    assert!(connection.describe(&books).await.unwrap().columns.len() > 1);
    let joined = rows(
        &connection,
        "SELECT count(*) FROM bookshop.order_items i JOIN shop.books b ON b.id = i.book_id",
    )
    .await;
    assert!(
        matches!(joined.as_slice(), [row] if matches!(row.as_slice(), [Value::Int(count)] if *count > 0)),
        "{joined:?}"
    );
    // The test server's password is its user's name too: what matters is
    // that no password is said.
    let said = shown(&connection).await;
    assert!(!said.contains(&format!("password={password}")), "{said}");
    assert!(!said.contains(&format!("password='{password}'")), "{said}");
    // The login was let go once the database was attached.
    assert_eq!(
        rows(&connection, "SELECT count(*) FROM duckdb_secrets()").await,
        [[Value::Int(0)]]
    );
    assert_eq!(
        rows(
            &connection,
            "SELECT readonly FROM duckdb_databases() WHERE database_name = 'shop'"
        )
        .await,
        [[Value::Bool(true)]]
    );
    // Each write sent through to the server is refused, and none lands.
    // A step may run (PostgreSQL takes a COMMIT): the write after it
    // still may not.
    for statement in writes {
        match statement.strip_prefix("then ") {
            Some(step) => drop(script(&connection, step).await),
            None => assert!(
                !refused(&connection, statement).await.is_empty(),
                "{statement}"
            ),
        }
    }
    assert_eq!(
        rows(&connection, landed).await,
        [[Value::Int(0)]],
        "a write landed"
    );
    // A wrong password: said by the server, the password not.
    let wrong = Attaching {
        name: "shop".into(),
        spec,
        secrets: Secrets {
            password: Some("not-the-bookshop-password".into()),
            ..secrets
        },
    };
    let Err(error) = open(&dir, &[wrong]).await else {
        panic!("a wrong password attached");
    };
    let error = error.to_string();
    assert!(error.contains("Can't attach shop"), "{error}");
    assert!(!error.contains("not-the-bookshop-password"), "{error}");
}

#[tokio::test]
async fn a_postgres_bookshop_is_attached() {
    let Some((spec, secrets)) = server("TABLETIST_TEST_PG_URL") else {
        return;
    };
    a_server_bookshop_is_attached(
        spec,
        secrets,
        &[
            "CALL postgres_execute('shop', 'CREATE TABLE attach_probe (id integer)')",
            "CALL postgres_execute('shop', 'CREATE TABLE attach_probe (id integer)', \
             use_transaction := false)",
            "then CALL postgres_execute('shop', 'COMMIT')",
            "CALL postgres_execute('shop', 'CREATE TABLE attach_probe (id integer)')",
        ],
        "SELECT count(*) FROM postgres_query('shop', \
         'SELECT 1 FROM pg_tables WHERE tablename = ''attach_probe''')",
    )
    .await;
}

#[tokio::test]
async fn a_mysql_bookshop_is_attached() {
    let Some((spec, secrets)) = server("TABLETIST_TEST_MYSQL_URL") else {
        return;
    };
    a_server_bookshop_is_attached(
        spec,
        secrets,
        &[
            "CALL mysql_execute('shop', 'CREATE TABLE attach_probe (id integer)')",
            "CALL mysql_execute('shop', 'COMMIT')",
            "SELECT * FROM mysql_query('shop', 'CREATE TABLE attach_probe (id integer)')",
        ],
        "SELECT count(*) FROM mysql_query('shop', \
         'SELECT 1 FROM information_schema.tables WHERE table_name = ''attach_probe''')",
    )
    .await;
}
