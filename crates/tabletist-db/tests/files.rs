//! Files connections through the public `Connection` API, on the
//! Bookshop's exports (`fixtures::write_bookshop_files`): a view for each
//! file, a `.duckdb` file as a schema, and a session that reads those
//! files and nothing else, and writes nothing.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use tabletist_db::agent::{self, Limits, Refusal, Sources, TableName, Tables};
use tabletist_db::files::{Format, Source};
use tabletist_db::{
    ConnectSpec, Connection, Dialect, Driver, Error, Filter, FilterOp, ObjectKind, ObjectRef,
    RowQuery, ScriptMode, Secrets, StatementOutcome, StopFlag, Value, ValueKind, sql,
};

/// The Bookshop's exports in a folder of their own, and a session over it.
async fn exports() -> (Connection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
    let connection = open(Source::Folder(dir.path().to_owned())).await.unwrap();
    (connection, dir)
}

async fn open(source: Source) -> Result<Connection, Error> {
    Connection::connect(&ConnectSpec::files(source), &Secrets::default()).await
}

/// Why `source` does not open.
async fn fails(source: Source) -> Error {
    match open(source).await {
        Ok(_) => panic!("it opened"),
        Err(error) => error,
    }
}

fn table(name: &str, limit: u32) -> RowQuery {
    RowQuery::new(ObjectRef::new("main", name), limit)
}

/// Runs `text` as the SQL editor does: split, then one read-only script.
async fn script(connection: &Connection, text: &str) -> Result<Vec<StatementOutcome>, Error> {
    let statements = sql::statements(Dialect::DuckDb, text);
    let outcome = connection
        .run_script(&statements, 100, ScriptMode::ReadOnly, &StopFlag::new())
        .await?;
    Ok(outcome.results.into_iter().map(|r| r.outcome).collect())
}

/// The one outcome of a script of one statement, which must be rows.
async fn rows(connection: &Connection, text: &str) -> Vec<Vec<Value>> {
    match script(connection, text).await.unwrap().remove(0) {
        StatementOutcome::Rows { rows, .. } => rows,
        other => panic!("{text}: {other:?}"),
    }
}

/// The one outcome of a script of one statement, which must be an error.
async fn refused(connection: &Connection, text: &str) -> String {
    match script(connection, text).await {
        Err(error) => error.to_string(),
        Ok(mut outcomes) => match outcomes.remove(0) {
            StatementOutcome::Error { error, .. } => error.to_string(),
            other => panic!("{text} ran: {other:?}"),
        },
    }
}

#[tokio::test]
async fn each_file_is_a_table_and_a_duckdb_file_a_schema() {
    let (connection, _dir) = exports().await;
    assert_eq!(connection.driver(), Driver::Files);
    assert_eq!(connection.dialect(), Dialect::DuckDb);
    assert!(!connection.is_encrypted());
    assert!(connection.list_databases().await.unwrap().is_empty());
    assert_eq!(
        connection.list_schemas().await.unwrap(),
        ["main", "bookshop"]
    );
    let objects = connection.list_objects("main").await.unwrap();
    let names: Vec<&str> = objects.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "authors",
            "books",
            "customers",
            "events",
            "orders",
            "returns"
        ]
    );
    assert!(objects.iter().all(|o| o.kind == ObjectKind::Table));
    // Parquet says its rows; a text file's are estimated from its start,
    // which holds the whole of these small ones.
    let rows = |name: &str| {
        objects
            .iter()
            .find(|o| o.name == name)
            .unwrap()
            .estimated_rows
    };
    assert_eq!(rows("customers"), Some(8));
    assert_eq!(rows("orders"), Some(12));
    assert_eq!(rows("events"), Some(12));
    assert_eq!(rows("books"), None);
    let shop = connection.list_objects("bookshop").await.unwrap();
    let names: Vec<&str> = shop.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(names, ["book_covers", "order_items"]);
    let version = connection.server_version().await.unwrap();
    assert!(version.starts_with("DuckDB 2."), "{version}");
    assert_eq!(
        connection.session_zone().await.unwrap().as_deref(),
        Some("UTC")
    );
}

#[tokio::test]
async fn every_format_reads_into_columns() {
    let (connection, _dir) = exports().await;
    for (name, first) in [
        ("orders", "id"),
        ("returns", "order_id"),
        ("authors", "id"),
        ("customers", "id"),
        ("books", "id"),
        ("events", "order_id"),
    ] {
        let page = connection.fetch_rows(&table(name, 3)).await.unwrap();
        assert_eq!(page.columns[0].name, first, "{name}");
        assert!(page.columns.len() > 1, "{name}: {:?}", page.columns);
        assert!(!page.rows.is_empty(), "{name}");
        assert!(!page.ordered_by_key, "{name}");
    }
    // The TSV is split on its tabs, not read as one column.
    let authors = connection.fetch_rows(&table("authors", 1)).await.unwrap();
    assert_eq!(authors.columns[1].name, "name");
    let structure = connection
        .describe(&ObjectRef::new("main", "orders"))
        .await
        .unwrap();
    let types: Vec<(&str, &str)> = structure
        .columns
        .iter()
        .map(|c| (c.name.as_str(), c.type_name.as_str()))
        .collect();
    assert_eq!(types[0], ("id", "BIGINT"));
    assert!(structure.primary_key.is_empty());
    let file = structure.file.unwrap();
    assert_eq!(file.format, Format::Csv);
    assert!(file.path.ends_with("orders.csv"));
    assert!(file.size > 0);
    assert!(
        file.options.contains("sample_size = 20000"),
        "{}",
        file.options
    );
    // A `.duckdb` file's table is no file's.
    let covers = connection
        .describe(&ObjectRef::new("bookshop", "book_covers"))
        .await
        .unwrap();
    assert!(covers.file.is_none());
    assert!(!covers.columns.is_empty());
}

#[tokio::test]
async fn values_keep_their_kinds() {
    let (connection, _dir) = exports().await;
    let page = connection.fetch_rows(&table("orders", 1)).await.unwrap();
    let kinds: Vec<(&str, ValueKind)> = page
        .columns
        .iter()
        .map(|c| (c.name.as_str(), c.kind))
        .collect();
    assert_eq!(kinds[0], ("id", ValueKind::Numeric));
    assert!(matches!(page.rows[0][0], Value::Int(_)));
    let at = rows(
        &connection,
        "SELECT DATE '2024-03-05', TIMESTAMP '2024-03-05 10:04:00', \
         TIMESTAMPTZ '2024-03-05 10:04:00+02', 12.50::DECIMAL(10,2), true, \
         '\\xAB'::BLOB, [1, 2], NULL",
    )
    .await;
    assert_eq!(
        at[0],
        [
            Value::Text("2024-03-05".into()),
            Value::Text("2024-03-05 10:04:00".into()),
            Value::Text("2024-03-05 08:04:00+00".into()),
            Value::Text("12.50".into()),
            Value::Bool(true),
            Value::Bytes(vec![0xAB].into()),
            Value::Text("[1, 2]".into()),
            Value::Null,
        ]
    );
}

#[tokio::test]
async fn pages_filter_sort_and_count() {
    let (connection, _dir) = exports().await;
    let mut query = table("orders", 2);
    query.filters.push(Filter {
        column: "id".into(),
        op: FilterOp::Gt,
        value: "2".into(),
    });
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(page.rows.len(), 2);
    assert!(page.has_more);
    assert_eq!(connection.count_rows(&query).await.unwrap(), 10);
    query.raw_where = Some("id = 3".into());
    assert_eq!(connection.count_rows(&query).await.unwrap(), 1);
    query.raw_where = Some("id = 3; DROP VIEW orders".into());
    let error = connection.count_rows(&query).await.unwrap_err();
    assert!(error.to_string().contains("holds a `;`"), "{error}");
}

#[tokio::test]
async fn the_ddl_is_the_view_over_the_file() {
    let (connection, dir) = exports().await;
    let ddl = connection
        .ddl(&ObjectRef::new("main", "orders"))
        .await
        .unwrap();
    assert_eq!(ddl.driver, Driver::Files);
    let text = ddl.text();
    let path = dir.path().join("orders.csv");
    assert_eq!(
        text,
        format!(
            "CREATE VIEW orders AS SELECT * FROM read_csv('{}', header = true, \
             auto_detect = true, sample_size = 20000);",
            path.display()
        )
    );
}

#[tokio::test]
async fn scripts_read_the_files_and_join_them() {
    let (connection, _dir) = exports().await;
    let found = rows(
        &connection,
        "SELECT count(*) FROM orders o JOIN customers c ON c.id = o.customer_id",
    )
    .await;
    assert_eq!(found, [[Value::Int(12)]]);
    let found = rows(&connection, "FROM bookshop.order_items SELECT count(*)").await;
    assert_eq!(found, [[Value::Int(40)]]);
    // A run ends at its first error.
    let outcomes = script(&connection, "SELECT 1; SELECT * FROM nonsense; SELECT 2")
        .await
        .unwrap();
    assert_eq!(outcomes.len(), 2, "the run ends at the first error");
}

#[tokio::test]
async fn nothing_writes_and_nothing_else_is_read() {
    let (connection, dir) = exports().await;
    let orders = dir.path().join("orders.csv");
    let before = std::fs::read(&orders).unwrap();
    let other = dir.path().join("secrets.csv");
    std::fs::write(&other, "a\n1\n").unwrap();
    let quoted = |path: &std::path::Path| format!("'{}'", path.display());
    for (text, says) in [
        ("CREATE TABLE notes (id INTEGER)", "read-only"),
        ("DROP VIEW orders", "read-only"),
        (&format!("COPY orders TO {}", quoted(&orders)), "COPY"),
        ("INSTALL excel", "INSTALL"),
        ("SET enable_external_access = true", "SET"),
        ("ATTACH ':memory:' AS scratch", "ATTACH"),
        ("USE bookshop", "USE"),
        ("PRAGMA enable_profiling", "PRAGMA"),
        (
            &format!("SELECT * FROM read_csv({})", quoted(&other)),
            "Permission",
        ),
        (&format!("SELECT * FROM {}", quoted(&other)), "Permission"),
        ("SELECT * FROM glob('/*')", "Permission"),
        ("BEGIN", "BEGIN"),
    ] {
        let error = refused(&connection, text).await;
        assert!(error.contains(says), "{text}: {error}");
    }
    assert_eq!(std::fs::read(&orders).unwrap(), before);
    // Still one session that reads, as it was.
    assert_eq!(
        rows(&connection, "SELECT count(*) FROM orders").await,
        [[Value::Int(12)]]
    );
    // A script that would write is refused before it is sent.
    let statements = sql::statements(Dialect::DuckDb, "SELECT 1");
    let error = connection
        .run_script(&statements, 10, ScriptMode::Write, &StopFlag::new())
        .await
        .unwrap_err();
    assert_eq!(error, Error::ReadOnly);
}

#[tokio::test]
async fn a_test_reads_every_file_and_says_how_much() {
    let (connection, dir) = exports().await;
    let read = connection.read_files().await.unwrap().unwrap();
    assert_eq!(read.files, 7);
    let size: u64 = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .sum();
    assert_eq!(read.bytes, size);
}

#[tokio::test]
async fn a_folder_that_is_gone_or_a_file_that_is_not_there_fails_to_open() {
    let dir = tempfile::tempdir().unwrap();
    let gone = dir.path().join("bookshop-exports");
    let error = fails(Source::Folder(gone)).await;
    let Error::Connect(message) = &error else {
        panic!("{error:?}");
    };
    assert!(
        message.ends_with("bookshop-exports: no such folder"),
        "{message}"
    );
    let error = fails(Source::Files(vec![dir.path().join("orders.csv")])).await;
    assert!(
        error.to_string().contains("orders.csv: no such file"),
        "{error}"
    );
    // A file that is not what its name says names itself.
    std::fs::write(dir.path().join("customers.parquet"), "not parquet").unwrap();
    let error = fails(Source::Files(vec![dir.path().join("customers.parquet")])).await;
    assert!(
        error.to_string().contains("Can't read customers.parquet: "),
        "{error}"
    );
    assert!(!error.to_string().contains('\n'), "{error}");
}

#[tokio::test]
async fn a_list_opens_only_its_files_and_clashes_take_their_extension() {
    let dir = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
    let parquet = dir.path().join("orders.parquet");
    // An orders.parquet beside orders.csv, written by DuckDB itself.
    duckdb_copy(&dir.path().join("orders.csv"), &parquet);
    let connection = open(Source::Files(vec![
        dir.path().join("orders.csv"),
        parquet.clone(),
    ]))
    .await
    .unwrap();
    let names: Vec<String> = connection
        .list_objects("main")
        .await
        .unwrap()
        .into_iter()
        .map(|o| o.name)
        .collect();
    assert_eq!(names, ["orders_csv", "orders_parquet"]);
    assert_eq!(connection.list_schemas().await.unwrap(), ["main"]);
    // The others are not the session's files.
    let error = refused(&connection, "SELECT * FROM customers").await;
    assert!(error.contains("customers"), "{error}");
}

/// Writes `csv` again as Parquet at `to`.
fn duckdb_copy(csv: &std::path::Path, to: &std::path::Path) {
    let session = duckdb::Connection::open_in_memory().unwrap();
    session
        .execute_batch(&format!(
            "COPY (SELECT * FROM read_csv('{}')) TO '{}' (FORMAT parquet)",
            csv.display(),
            to.display()
        ))
        .unwrap();
}

const LIMITS: Limits = Limits {
    rows: 100,
    timeout: Duration::from_secs(5),
};

#[tokio::test]
async fn agents_read_the_files_and_are_refused_where_a_rule_hides() {
    let (connection, _dir) = exports().await;
    let read = agent::check_read(
        Dialect::DuckDb,
        "SELECT count(*) FROM orders WHERE id > ?",
        &[agent::Param::Int(2)],
    )
    .unwrap();
    let answer = connection
        .run_agent_read(&read, LIMITS, &Sources::default(), &StopFlag::new())
        .await
        .unwrap();
    assert_eq!(answer.rows, [[Value::Int(10)]]);
    // A statement that is no read is refused before it is sent.
    assert!(agent::check_read(Dialect::DuckDb, "COPY orders TO 'x.csv'", &[]).is_err());
    assert!(agent::check_read(Dialect::DuckDb, "SELECT query('SELECT 1')", &[]).is_err());
    // A rule that hides anything refuses every read.
    let hides = Sources::person(
        Tables::AllExcept(vec!["customers".parse::<TableName>().unwrap()]),
        Vec::new(),
    );
    let error = connection
        .run_agent_read(&read, LIMITS, &hides, &StopFlag::new())
        .await
        .unwrap_err();
    assert_eq!(error, Error::Agent(Refusal::NotKeptOnFiles));
}

#[tokio::test]
async fn a_stop_ends_a_long_statement() {
    let (connection, _dir) = exports().await;
    let statements = sql::statements(
        Dialect::DuckDb,
        "SELECT count(*) FROM range(100000000000) a, range(1000) b",
    );
    let stop = StopFlag::new();
    let stopping = stop.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        stopping.stop();
    });
    let started = std::time::Instant::now();
    let outcome = connection
        .run_script(&statements, 10, ScriptMode::ReadOnly, &stop)
        .await
        .unwrap();
    assert!(outcome.was_cancelled(), "{outcome:?}");
    assert!(started.elapsed() < Duration::from_secs(10));
    // The session reads again after it.
    assert_eq!(rows(&connection, "SELECT 1").await, [[Value::Int(1)]]);
}

#[tokio::test]
async fn an_agents_read_past_its_time_is_a_timeout() {
    let (connection, _dir) = exports().await;
    let read = agent::check_read(
        Dialect::DuckDb,
        "SELECT count(*) FROM range(100000000000) a, range(1000) b",
        &[],
    )
    .unwrap();
    let limits = Limits {
        rows: 10,
        timeout: Duration::from_millis(200),
    };
    let error = connection
        .run_agent_read(&read, limits, &Sources::default(), &StopFlag::new())
        .await
        .unwrap_err();
    assert_eq!(error, Error::Timeout);
}

#[tokio::test]
async fn an_explain_that_would_run_a_copy_is_refused_and_the_file_kept() {
    let (connection, dir) = exports().await;
    let orders = dir.path().join("orders.csv");
    let before = std::fs::read(&orders).unwrap();
    for text in [
        format!(
            "EXPLAIN ANALYZE COPY (SELECT 'x' AS a) TO '{}' (USE_TMP_FILE false)",
            orders.display()
        ),
        format!(
            "EXPLAIN (ANALYZE) COPY (SELECT 'x' AS a) TO '{}' (USE_TMP_FILE false)",
            orders.display()
        ),
    ] {
        let error = refused(&connection, &text).await;
        assert!(error.contains("EXPLAIN COPY"), "{text}: {error}");
    }
    assert_eq!(std::fs::read(&orders).unwrap(), before);
}

#[tokio::test]
async fn a_session_asked_to_write_still_reads_only() {
    let dir = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
    let shop = dir.path().join("bookshop.duckdb");
    let before = std::fs::read(&shop).unwrap();
    let connection = tabletist_db::Connection::connect_with(
        &ConnectSpec::files(Source::Folder(dir.path().to_owned())),
        &Secrets::default(),
        &tabletist_db::HostKeys::default(),
        tabletist_db::Access::Writable,
    )
    .await
    .unwrap();
    assert_eq!(connection.access(), tabletist_db::Access::ReadOnly);
    let error = refused(
        &connection,
        "INSERT INTO bookshop.book_covers SELECT * FROM bookshop.book_covers",
    )
    .await;
    assert!(error.contains("read-only"), "{error}");
    connection.close().await.unwrap();
    assert_eq!(
        std::fs::read(&shop).unwrap(),
        before,
        "the .duckdb file was written"
    );
}

#[tokio::test]
async fn nested_moments_infinities_and_nulls_are_written_out() {
    let (connection, _dir) = exports().await;
    let found = rows(
        &connection,
        "SELECT [TIMESTAMPTZ '2024-01-01 12:00:00+00'], 'infinity'::TIMESTAMP, \
         '-infinity'::DATE, [1, NULL], ['a']",
    )
    .await;
    assert_eq!(
        found[0],
        [
            Value::Text("[2024-01-01 12:00:00]".into()),
            Value::Text("infinity".into()),
            Value::Text("-infinity".into()),
            Value::Text("[1, NULL]".into()),
            Value::Text("[\"a\"]".into()),
        ]
    );
}

#[tokio::test]
async fn a_json_column_is_json_by_the_catalog() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("events.duckdb");
    let session = duckdb::Connection::open(&file).unwrap();
    session
        .execute_batch("CREATE TABLE logged (id INTEGER, body JSON); INSERT INTO logged VALUES (1, '{\"a\": 1}')")
        .unwrap();
    drop(session);
    let connection = open(Source::Files(vec![file])).await.unwrap();
    let page = connection
        .fetch_rows(&RowQuery::new(ObjectRef::new("events", "logged"), 5))
        .await
        .unwrap();
    assert_eq!(page.columns[1].kind, ValueKind::Json);
    assert_eq!(page.columns[1].type_name, "JSON");
    assert_eq!(page.columns[0].type_name, "INTEGER");
}

#[tokio::test]
async fn a_call_given_up_on_stops_no_other_calls_statement() {
    let (connection, _dir) = exports().await;
    let connection = std::sync::Arc::new(connection);
    let statements = sql::statements(
        Dialect::DuckDb,
        "SELECT count(*) FROM range(100000000000) a, range(1000) b",
    );
    let stop = StopFlag::new();
    let running = {
        let (connection, stop) = (std::sync::Arc::clone(&connection), stop.clone());
        tokio::spawn(async move {
            let started = std::time::Instant::now();
            let outcome = connection
                .run_script(&statements, 10, ScriptMode::ReadOnly, &stop)
                .await
                .unwrap();
            (outcome, started.elapsed())
        })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    // A page asked for behind it, and given up on.
    let waited = tokio::time::timeout(
        Duration::from_millis(100),
        connection.fetch_rows(&table("orders", 1)),
    )
    .await;
    assert!(waited.is_err(), "the page should wait behind the script");
    tokio::time::sleep(Duration::from_millis(500)).await;
    stop.stop();
    let (outcome, took) = running.await.unwrap();
    assert!(outcome.was_cancelled());
    assert!(
        took >= Duration::from_millis(600),
        "the script stopped after {took:?}"
    );
}

/// The names of `connection`'s tables, each with the ids it reads.
async fn ids(connection: &Connection) -> Vec<(String, Vec<Vec<Value>>)> {
    let mut read = Vec::new();
    for object in connection.list_objects("main").await.unwrap() {
        let ids = rows(connection, &format!("SELECT id FROM \"{}\"", object.name)).await;
        read.push((object.name, ids));
    }
    read
}

#[tokio::test]
async fn a_file_whose_name_looks_like_a_pattern_reads_only_itself() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = vec![("report[1].csv", 10), ("report1.csv", 20)];
    // Windows keeps no `*` or `?` in a name.
    if cfg!(unix) {
        files.extend([
            ("report*.csv", 30),
            ("report?.csv", 50),
            ("report2.csv", 40),
        ]);
    }
    for (name, id) in &files {
        std::fs::write(dir.path().join(name), format!("id\n{id}\n")).unwrap();
    }
    // Each file of the folder is a table of its own rows.
    let folder = open(Source::Folder(dir.path().to_owned())).await.unwrap();
    let read = ids(&folder).await;
    let mut each: Vec<Vec<Vec<Value>>> = read.into_iter().map(|(_, ids)| ids).collect();
    each.sort_by_key(|ids| format!("{ids:?}"));
    let mut expected: Vec<Vec<Vec<Value>>> = files
        .iter()
        .map(|(_, id)| vec![vec![Value::Int(*id)]])
        .collect();
    expected.sort_by_key(|ids| format!("{ids:?}"));
    assert_eq!(each, expected);
    // A list of one reads that one, and the fence lets nothing else in.
    let list = open(Source::Files(vec![dir.path().join("report[1].csv")]))
        .await
        .unwrap();
    let read = ids(&list).await;
    assert_eq!(read.len(), 1, "{read:?}");
    assert_eq!(read[0].1, [[Value::Int(10)]], "{read:?}");
    let other = dir
        .path()
        .join("report1.csv")
        .to_string_lossy()
        .replace('\'', "''");
    let error = refused(&list, &format!("SELECT * FROM read_csv('{other}')")).await;
    assert!(error.contains("report1.csv"), "{error}");
}

#[tokio::test]
async fn a_duckdb_file_named_as_a_catalog_schema_leaves_the_others_readable() {
    let dir = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
    let shop = dir.path().join("bookshop.duckdb");
    for name in ["information_schema.duckdb", "pg_catalog.duckdb"] {
        std::fs::copy(&shop, dir.path().join(name)).unwrap();
    }
    let connection = open(Source::Folder(dir.path().to_owned())).await.unwrap();
    let names: Vec<String> = connection
        .list_objects("main")
        .await
        .unwrap()
        .into_iter()
        .map(|o| o.name)
        .collect();
    assert!(names.contains(&"orders".to_owned()), "{names:?}");
    // Structure, pages and counts of every table still read.
    let orders = ObjectRef::new("main", "orders");
    assert!(
        !connection
            .describe(&orders)
            .await
            .unwrap()
            .columns
            .is_empty()
    );
    let page = connection.fetch_rows(&table("orders", 10)).await.unwrap();
    assert!(!page.rows.is_empty());
    assert!(connection.count_rows(&table("orders", 10)).await.unwrap() > 0);
    let schemas = connection.list_schemas().await.unwrap();
    for schema in ["information_schema_duckdb", "pg_catalog_duckdb"] {
        assert!(schemas.iter().any(|s| s == schema), "{schemas:?}");
    }
}
