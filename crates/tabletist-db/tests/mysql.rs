//! MySQL through the public `Connection` API. Needs a server:
//! `docker compose up -d --wait mysql` and TABLETIST_TEST_MYSQL_URL (see
//! compose.yaml). Without it every test prints "skipped" and passes.

#![allow(clippy::unwrap_used)]

use std::time::Duration;

use mysql_async::prelude::Queryable;
use tabletist_db::{ConnectSpec, Connection, Driver, Error, ObjectKind, Secrets, TlsMode};
use tokio::sync::OnceCell;

static FIXTURE: OnceCell<()> = OnceCell::const_new();

fn url() -> Option<String> {
    std::env::var("TABLETIST_TEST_MYSQL_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
}

fn spec() -> Option<(ConnectSpec, Secrets)> {
    let (mut spec, secrets) = ConnectSpec::from_url(&url()?).unwrap();
    spec.tls = TlsMode::Disable;
    Some((spec, secrets))
}

async fn load_fixture() {
    FIXTURE
        .get_or_init(|| async {
            let opts =
                mysql_async::Opts::from_url(&format!("{}?prefer_socket=false", url().unwrap()))
                    .unwrap();
            // The server may still be starting when the first test runs.
            let deadline = std::time::Instant::now() + Duration::from_secs(90);
            let mut conn = loop {
                match mysql_async::Conn::new(opts.clone()).await {
                    Ok(conn) => break conn,
                    Err(error) if std::time::Instant::now() < deadline => {
                        eprintln!("waiting for MySQL: {error}");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    Err(error) => panic!("MySQL never answered: {error}"),
                }
            };
            for statement in tabletist_db::fixtures::MYSQL_SQL.split(";\n") {
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

async fn connect() -> Option<Connection> {
    let Some((spec, secrets)) = spec() else {
        eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
        return None;
    };
    load_fixture().await;
    Some(Connection::connect(&spec, &secrets).await.unwrap())
}

#[tokio::test]
async fn connections_report_mysql_and_list_databases_as_schemas() {
    let Some(connection) = connect().await else {
        return;
    };
    assert_eq!(connection.driver(), Driver::MySql);
    assert!(connection.list_databases().await.unwrap().is_empty());
    let schemas = connection.list_schemas().await.unwrap();
    for schema in ["tabletist", "billing", "information_schema"] {
        assert!(
            schemas.contains(&schema.to_owned()),
            "{schema} in {schemas:?}"
        );
    }
}

#[tokio::test]
async fn objects_have_kinds_and_estimates() {
    let Some(connection) = connect().await else {
        return;
    };
    let objects = connection.list_objects("tabletist").await.unwrap();
    let find = |name: &str| objects.iter().find(|object| object.name == name).unwrap();
    assert_eq!(find("users").kind, ObjectKind::Table);
    assert_eq!(find("active_users").kind, ObjectKind::View);
    assert_eq!(find("weird \"name\"").kind, ObjectKind::Table);
    // InnoDB estimates are approximate.
    let estimate = find("big").estimated_rows.unwrap();
    assert!((50_000..=150_000).contains(&estimate), "{estimate}");
    let names: Vec<&str> = objects.iter().map(|object| object.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
}

#[tokio::test]
async fn a_wrong_password_is_an_auth_error() {
    let Some((spec, _)) = spec() else { return };
    let secrets = Secrets {
        password: Some("definitely wrong".into()),
        ..Secrets::default()
    };
    let result = Connection::connect(&spec, &secrets).await;
    assert!(matches!(result, Err(Error::Auth(_))), "{:?}", result.err());
}

#[tokio::test]
/// The server was reached and refused: its own code and words, not "could
/// not connect" (which would send the user checking the host and port).
async fn a_missing_database_is_the_servers_own_error() {
    let Some((mut spec, secrets)) = spec() else {
        return;
    };
    spec.database = "no_such_database".into();
    match Connection::connect(&spec, &secrets).await {
        Err(Error::Query { code, message, .. }) => {
            assert!(code.is_some(), "{code:?}");
            assert!(message.contains("no_such_database"), "{message}");
        }
        other => panic!("{:?}", other.err()),
    }
}

#[tokio::test]
async fn tls_modes_behave_like_libpq() {
    let Some((mut spec, secrets)) = spec() else {
        return;
    };
    load_fixture().await;
    // MySQL 8 generates a self-signed certificate at first start.
    for mode in [TlsMode::Prefer, TlsMode::Require] {
        spec.tls = mode;
        let conn = Connection::connect(&spec, &secrets).await;
        assert!(conn.is_ok_and(|conn| conn.is_encrypted()), "{mode:?}");
    }
    spec.tls = TlsMode::Disable;
    let conn = Connection::connect(&spec, &secrets).await.unwrap();
    assert!(!conn.is_encrypted());
    spec.tls = TlsMode::VerifyFull;
    let result = Connection::connect(&spec, &secrets).await;
    assert!(matches!(result, Err(Error::Tls(_))), "{:?}", result.err());
}

#[tokio::test]
async fn a_user_is_required() {
    let spec = ConnectSpec {
        driver: Driver::MySql,
        user: String::new(),
        port: 3306,
        ..ConnectSpec::default()
    };
    assert!(matches!(
        Connection::connect(&spec, &Secrets::default()).await,
        Err(Error::InvalidSpec(_))
    ));
}

use tabletist_db::ObjectRef;

#[tokio::test]
async fn users_structure_has_full_types_comments_key_and_indexes() {
    let Some(connection) = connect().await else {
        return;
    };
    let structure = connection
        .describe(&ObjectRef::new("tabletist", "users"))
        .await
        .unwrap();
    let column = |name: &str| structure.columns.iter().find(|c| c.name == name).unwrap();
    assert_eq!(structure.columns[0].name, "id");
    assert_eq!(column("id").type_name, "int");
    assert!(!column("id").nullable);
    assert!(column("name").nullable);
    assert_eq!(column("email").type_name, "varchar(255)");
    assert_eq!(column("email").comment.as_deref(), Some("Login address"));
    assert_eq!(column("balance").type_name, "decimal(14,2)");
    assert_eq!(column("counter").type_name, "bigint unsigned");
    assert_eq!(column("mood").type_name, "enum('happy','sad')");
    assert!(
        column("created_at")
            .default
            .as_deref()
            .unwrap()
            .starts_with("2026-01-01 00:00:00")
    );
    assert_eq!(structure.primary_key, vec!["id".to_owned()]);
    let primary = structure
        .indexes
        .iter()
        .find(|i| i.name == "PRIMARY")
        .unwrap();
    assert!(primary.primary && primary.unique);
    assert_eq!(primary.method.as_deref(), Some("btree"));
    let email = structure
        .indexes
        .iter()
        .find(|i| i.name == "email")
        .unwrap();
    assert!(email.unique && !email.primary);
    let name = structure
        .indexes
        .iter()
        .find(|i| i.name == "users_name_idx")
        .unwrap();
    assert!(!name.unique);
}

#[tokio::test]
async fn foreign_keys_name_their_target_and_actions() {
    let Some(connection) = connect().await else {
        return;
    };
    let orders = connection
        .describe(&ObjectRef::new("tabletist", "orders"))
        .await
        .unwrap();
    let key = &orders.foreign_keys[0];
    assert_eq!(key.name.as_deref(), Some("orders_user_id_fkey"));
    assert_eq!(key.columns, vec!["user_id".to_owned()]);
    assert_eq!(
        (key.ref_schema.as_str(), key.ref_table.as_str()),
        ("tabletist", "users")
    );
    assert_eq!(key.ref_columns, vec!["id".to_owned()]);
    assert_eq!(key.on_delete, "CASCADE");
    let composite = orders
        .indexes
        .iter()
        .find(|i| i.name == "orders_user_total_idx")
        .unwrap();
    assert_eq!(
        composite.columns,
        vec!["user_id".to_owned(), "total".to_owned()]
    );
    let invoices = connection
        .describe(&ObjectRef::new("billing", "invoices"))
        .await
        .unwrap();
    assert_eq!(invoices.foreign_keys[0].ref_schema, "tabletist");
    assert_eq!(invoices.foreign_keys[0].on_update, "RESTRICT");
}

#[tokio::test]
async fn views_and_quoted_names_describe_and_missing_objects_fail() {
    let Some(connection) = connect().await else {
        return;
    };
    let view = connection
        .describe(&ObjectRef::new("tabletist", "active_users"))
        .await
        .unwrap();
    assert_eq!(view.columns.len(), 2);
    assert!(view.primary_key.is_empty() && view.indexes.is_empty());
    let weird = connection
        .describe(&ObjectRef::new("tabletist", "weird \"name\""))
        .await
        .unwrap();
    assert_eq!(weird.columns[1].name, "select");
    assert!(matches!(
        connection
            .describe(&ObjectRef::new("tabletist", "nope"))
            .await,
        Err(Error::Query { .. })
    ));
}

use tabletist_db::{Filter, FilterOp, RowPage, RowQuery, Sort, SortDir, Value, ValueKind};

fn users(limit: u32) -> RowQuery {
    RowQuery::new(ObjectRef::new("tabletist", "users"), limit)
}

fn ids(page: &RowPage) -> Vec<i64> {
    page.rows
        .iter()
        .map(|row| match row[0] {
            Value::Int(id) => id,
            ref other => panic!("id is {other:?}"),
        })
        .collect()
}

fn filtered(column: &str, op: FilterOp, value: &str) -> RowQuery {
    let mut query = users(50);
    query.filters = vec![Filter {
        column: column.into(),
        op,
        value: value.into(),
    }];
    query
}

#[tokio::test]
async fn pages_have_typed_values_and_kinds() {
    let Some(connection) = connect().await else {
        return;
    };
    let page = connection.fetch_rows(&users(3)).await.unwrap();
    assert_eq!(ids(&page), vec![1, 2, 3]);
    assert!(page.has_more && page.ordered_by_key);
    let kind = |name: &str| page.columns.iter().find(|c| c.name == name).unwrap().kind;
    assert_eq!(kind("meta"), ValueKind::Json);
    assert_eq!(kind("avatar"), ValueKind::Binary);
    assert_eq!(kind("created_at"), ValueKind::Temporal);
    assert_eq!(kind("balance"), ValueKind::Numeric);
    let ada = &page.rows[0];
    assert_eq!(ada[3], Value::Text("2026-01-02 09:00:00".into()));
    assert_eq!(ada[4], Value::Int(1));
    assert_eq!(ada[6], Value::Bytes(vec![0x89, 0x50, 0x4e, 0x47].into()));
    assert_eq!(ada[7], Value::Float(99.5));
    assert_eq!(ada[8], Value::Text("123456789012.34".into()));
    assert_eq!(ada[9], Value::Text("18446744073709551615".into()));
    assert_eq!(ada[10], Value::Text("happy".into()));
    assert_eq!(ada[11], Value::Text("1815-12-10".into()));
    assert_eq!(ada[12], Value::Text("07:30:00".into()));
    let zoe = &page.rows[2];
    assert_eq!(zoe[2], Value::Text("Zoë 🚀".into()));
    assert_eq!(zoe[3], Value::Text("2026-01-04 11:45:00.000120".into()));
    assert_eq!(zoe[12], Value::Text("-25:15:00".into()));
    assert_eq!(page.rows[1][5], Value::Null);
}

#[tokio::test]
async fn sorting_paging_and_counting_work() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut query = users(5);
    query.sort = vec![Sort {
        column: "email".into(),
        dir: SortDir::Desc,
    }];
    assert_eq!(
        ids(&connection.fetch_rows(&query).await.unwrap()),
        vec![3, 5, 4, 2, 1]
    );
    let mut deep = RowQuery::new(ObjectRef::new("tabletist", "big"), 300);
    deep.offset = 99_900;
    let page = connection.fetch_rows(&deep).await.unwrap();
    assert_eq!(page.rows.len(), 100);
    assert!(!page.has_more);
    let all = RowQuery::new(ObjectRef::new("tabletist", "big"), 1);
    assert_eq!(connection.count_rows(&all).await.unwrap(), 100_000);
    let events = connection
        .fetch_rows(&RowQuery::new(ObjectRef::new("tabletist", "events"), 50))
        .await
        .unwrap();
    assert!(!events.ordered_by_key);
}

#[tokio::test]
async fn filters_work() {
    let Some(connection) = connect().await else {
        return;
    };
    let rows = |query: RowQuery| {
        let connection = &connection;
        async move { ids(&connection.fetch_rows(&query).await.unwrap()) }
    };
    assert_eq!(rows(filtered("id", FilterOp::Ge, "4")).await, vec![4, 5]);
    assert_eq!(rows(filtered("id", FilterOp::In, "1, 3")).await, vec![1, 3]);
    assert_eq!(
        rows(filtered("meta", FilterOp::IsNull, "")).await,
        vec![2, 4, 5]
    );
    assert_eq!(
        rows(filtered("name", FilterOp::Contains, "ZO")).await,
        vec![3]
    );
    assert_eq!(rows(filtered("mood", FilterOp::Eq, "sad")).await, vec![3]);
    assert_eq!(
        rows(filtered("counter", FilterOp::Eq, "18446744073709551615")).await,
        vec![1]
    );
    assert_eq!(
        connection
            .count_rows(&filtered("id", FilterOp::Lt, "3"))
            .await
            .unwrap(),
        2
    );
}

#[tokio::test]
async fn quotes_backslashes_and_wildcards_are_just_text() {
    let Some(connection) = connect().await else {
        return;
    };
    let page = connection
        .fetch_rows(&filtered("name", FilterOp::Eq, r"O'Brien C:\temp"))
        .await
        .unwrap();
    assert_eq!(ids(&page), vec![5]);
    let page = connection
        .fetch_rows(&filtered("name", FilterOp::Contains, "50%"))
        .await
        .unwrap();
    assert_eq!(ids(&page), vec![4]);
    let page = connection
        .fetch_rows(&filtered("name", FilterOp::Contains, "_now"))
        .await
        .unwrap();
    assert_eq!(ids(&page), vec![4]);
    let page = connection
        .fetch_rows(&filtered("name", FilterOp::Contains, r"C:\temp"))
        .await
        .unwrap();
    assert_eq!(ids(&page), vec![5]);
    let page = connection
        .fetch_rows(&filtered("name", FilterOp::Eq, "x' OR '1'='1"))
        .await
        .unwrap();
    assert!(page.rows.is_empty());
}

#[tokio::test]
async fn a_raw_where_cannot_write_or_chain_statements() {
    let Some(connection) = connect().await else {
        return;
    };
    for raw in [
        "1 = 1; DELETE FROM users",
        "1=1) ; COMMIT; DELETE FROM users; SELECT (1",
    ] {
        let mut query = users(50);
        query.raw_where = Some(raw.into());
        assert!(connection.fetch_rows(&query).await.is_err(), "{raw}");
        assert!(connection.count_rows(&query).await.is_err(), "{raw}");
    }
    assert_eq!(connection.count_rows(&users(1)).await.unwrap(), 5);
}

#[tokio::test]
async fn a_bad_raw_where_is_a_query_error_and_the_session_survives() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut query = users(50);
    query.raw_where = Some("no_such_column = 1".into());
    match connection.fetch_rows(&query).await {
        Err(Error::Query { code, .. }) => assert_eq!(code.as_deref(), Some("42S22")),
        other => panic!("{other:?}"),
    }
    assert!(connection.fetch_rows(&users(1)).await.is_ok());
}

#[tokio::test]
async fn quoted_names_and_views_browse() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut query = RowQuery::new(ObjectRef::new("tabletist", "weird \"name\""), 5);
    query.sort = vec![Sort {
        column: "select".into(),
        dir: SortDir::Asc,
    }];
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(page.columns[0].name, "col with space");
    let view = connection
        .fetch_rows(&RowQuery::new(
            ObjectRef::new("tabletist", "active_users"),
            5,
        ))
        .await
        .unwrap();
    assert_eq!(view.rows.len(), 4);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_running_query_can_be_cancelled() {
    let Some(connection) = connect().await else {
        return;
    };
    let cancel = connection.cancel_handle();
    let connection = std::sync::Arc::new(connection);
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        tokio::spawn(async move {
            let mut query = RowQuery::new(ObjectRef::new("tabletist", "users"), 5);
            // Thirty seconds per row, far longer than the test waits. (A big
            // cross join is no good: MySQL counts it in half a second.)
            query.raw_where = Some("SLEEP(30) = 0".into());
            connection.count_rows(&query).await
        })
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !running.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "cancel must stop the query"
        );
        cancel.cancel().await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    assert_eq!(running.await.unwrap(), Err(Error::Cancelled));
    assert!(connection.fetch_rows(&users(1)).await.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancel_on_an_idle_session_leaves_other_sessions_alone() {
    let Some(connection) = connect().await else {
        return;
    };
    let cancel = connection.cancel_handle();
    let mut other = admin().await;
    let running = tokio::spawn(async move {
        // SLEEP answers 1 when a KILL QUERY interrupts it.
        let slept = other.query_first::<i64, _>("SELECT SLEEP(2)").await;
        let _ = other.disconnect().await;
        slept
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    // Closing a tab cancels whether or not anything runs.
    for _ in 0..3 {
        cancel.cancel().await.unwrap();
    }
    assert_eq!(running.await.unwrap().unwrap(), Some(0));
    assert!(connection.fetch_rows(&users(1)).await.is_ok());
}

/// A writable connection for arranging and probing, outside the adapter.
async fn admin() -> mysql_async::Conn {
    let opts =
        mysql_async::Opts::from_url(&format!("{}?prefer_socket=false", url().unwrap())).unwrap();
    mysql_async::Conn::new(opts).await.unwrap()
}

#[tokio::test]
async fn a_failed_query_leaves_no_transaction_holding_locks() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut admin = admin().await;
    // A table of its own, so this probe cannot disturb other tests.
    admin
        .query_drop("DROP TABLE IF EXISTS lock_probe")
        .await
        .unwrap();
    admin
        .query_drop("CREATE TABLE lock_probe (id INT PRIMARY KEY, name TEXT)")
        .await
        .unwrap();
    admin
        .query_drop("INSERT INTO lock_probe VALUES (1, 'not json')")
        .await
        .unwrap();
    let mut query = RowQuery::new(ObjectRef::new("tabletist", "lock_probe"), 5);
    // Fails while running (not while preparing), after the table is locked.
    query.raw_where = Some("CAST(name AS JSON) IS NOT NULL".into());
    assert!(connection.fetch_rows(&query).await.is_err());
    assert!(connection.count_rows(&query).await.is_err());
    // A migration must not wait behind the failed read.
    admin
        .query_drop("SET SESSION lock_wait_timeout = 3")
        .await
        .unwrap();
    let altered = admin
        .query_drop("ALTER TABLE lock_probe ADD COLUMN extra INT")
        .await;
    admin.query_drop("DROP TABLE lock_probe").await.unwrap();
    assert!(
        altered.is_ok(),
        "the failed read still holds a lock: {altered:?}"
    );
}

#[tokio::test]
async fn verify_ca_is_refused_with_an_explanation() {
    let Some((mut spec, secrets)) = spec() else {
        return;
    };
    load_fixture().await;
    spec.tls = TlsMode::VerifyCa;
    match Connection::connect(&spec, &secrets).await {
        Err(Error::Tls(message)) => assert!(message.contains("verify-ca"), "{message}"),
        other => panic!("{:?}", other.err()),
    }
}

#[tokio::test]
async fn a_cancelled_query_leaves_no_transaction_holding_locks() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut admin = admin().await;
    admin
        .query_drop("DROP TABLE IF EXISTS cancel_probe")
        .await
        .unwrap();
    admin
        .query_drop("CREATE TABLE cancel_probe (id INT PRIMARY KEY)")
        .await
        .unwrap();
    admin
        .query_drop("INSERT INTO cancel_probe VALUES (1)")
        .await
        .unwrap();
    let cancel = connection.cancel_handle();
    let connection = std::sync::Arc::new(connection);
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        tokio::spawn(async move {
            let mut query = RowQuery::new(ObjectRef::new("tabletist", "cancel_probe"), 5);
            query.raw_where = Some("SLEEP(30) = 0".into());
            connection.fetch_rows(&query).await
        })
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !running.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "cancel must stop the query"
        );
        tokio::time::sleep(Duration::from_millis(300)).await;
        cancel.cancel().await.unwrap();
    }
    assert!(running.await.unwrap().is_err());
    admin
        .query_drop("SET SESSION lock_wait_timeout = 3")
        .await
        .unwrap();
    let altered = admin
        .query_drop("ALTER TABLE cancel_probe ADD COLUMN extra INT")
        .await;
    admin.query_drop("DROP TABLE cancel_probe").await.unwrap();
    assert!(
        altered.is_ok(),
        "the cancelled read still holds a lock: {altered:?}"
    );
}

use tabletist_db::{Dialect, ScriptOutcome, StatementOutcome, StopFlag};

fn script(text: &str) -> Vec<tabletist_db::sql::Statement> {
    tabletist_db::sql::statements(Dialect::MySql, text)
}

/// Awaits `future`, failing the test instead of hanging it.
async fn within<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(15), future)
        .await
        .expect("the run hung")
}

/// Runs `text` with a fresh stop flag.
async fn run(
    connection: &Connection,
    text: &str,
    limit: u32,
) -> tabletist_db::Result<ScriptOutcome> {
    within(connection.run_script(&script(text), limit, &StopFlag::new())).await
}

/// The rows of a script's only statement.
async fn rows_of(connection: &Connection, text: &str) -> Vec<Vec<Value>> {
    let outcome = run(connection, text, 10).await.unwrap();
    assert_eq!(outcome.results.len(), 1, "{text}: {outcome:?}");
    match &outcome.results[0].outcome {
        StatementOutcome::Rows { rows, .. } => rows.clone(),
        other => panic!("{text}: {other:?}"),
    }
}

/// What a script sees of the settings every connection starts with: the
/// session is read-only, in the server's time zone, without a mode that
/// changes how text is lexed, in the character set of the handshake, and
/// without a variable an earlier script set. (`mysql.rs` compares a
/// session with itself at connect, setting by setting.)
async fn assert_connect_time_settings(connection: &Connection) {
    let rows = rows_of(
        connection,
        "SELECT @@session.transaction_read_only, @@session.time_zone = @@global.time_zone, \
                @@session.sql_mode NOT LIKE '%ANSI%' \
                    AND @@session.sql_mode NOT LIKE '%NO_BACKSLASH_ESCAPES%', \
                @@session.character_set_client, @@session.character_set_results, \
                @@session.collation_connection, @@session.autocommit, @tabletist_left_over",
    )
    .await;
    assert_eq!(
        rows[0],
        [
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
            Value::Text("utf8mb4".into()),
            Value::Text("utf8mb4".into()),
            Value::Text("utf8mb4_general_ci".into()),
            Value::Int(1),
            Value::Null,
        ]
    );
}

#[tokio::test]
async fn a_script_has_typed_columns_and_truncates_at_the_limit() {
    let Some(connection) = connect().await else {
        return;
    };
    let outcome = run(
        &connection,
        "SELECT id, email, created_at, meta, avatar, balance FROM users ORDER BY id",
        2,
    )
    .await
    .unwrap();
    assert!(!outcome.stopped && !outcome.was_cancelled());
    let StatementOutcome::Rows {
        columns,
        rows,
        truncated,
    } = &outcome.results[0].outcome
    else {
        panic!("rows");
    };
    let described: Vec<(&str, &str, ValueKind)> = columns
        .iter()
        .map(|column| (column.name.as_str(), column.type_name.as_str(), column.kind))
        .collect();
    assert_eq!(
        described,
        [
            ("id", "int", ValueKind::Numeric),
            ("email", "varchar", ValueKind::Text),
            ("created_at", "datetime", ValueKind::Temporal),
            ("meta", "json", ValueKind::Json),
            ("avatar", "varbinary", ValueKind::Binary),
            ("balance", "decimal", ValueKind::Numeric),
        ]
    );
    assert_eq!(rows.len(), 2);
    assert!(*truncated);
    assert_eq!(rows[0][0], Value::Int(1));
    assert_eq!(
        rows[0][4],
        Value::Bytes(vec![0x89, 0x50, 0x4e, 0x47].into())
    );
    assert_eq!(rows[0][5], Value::Text("123456789012.34".into()));
    assert_eq!(rows[1][3], Value::Null);
}

#[tokio::test]
async fn truncates_at_the_limit() {
    let Some(connection) = connect().await else {
        return;
    };
    for limit in [2, 3] {
        // Same text twice: mysql_async caches the prepared statement.
        let outcome = run(&connection, "SELECT id FROM users", limit)
            .await
            .unwrap();
        assert!(matches!(
            &outcome.results[0].outcome,
            StatementOutcome::Rows { rows, truncated: true, .. } if rows.len() == limit as usize
        ));
    }
    // Exactly the limit is not truncated; nor is less.
    for limit in [5, 6] {
        let outcome = run(&connection, "SELECT id FROM users", limit)
            .await
            .unwrap();
        assert!(matches!(
            &outcome.results[0].outcome,
            StatementOutcome::Rows { rows, truncated: false, .. } if rows.len() == 5
        ));
    }
}

#[tokio::test]
async fn the_server_stops_at_one_row_more_than_the_limit() {
    let Some(connection) = connect().await else {
        return;
    };
    // The server stops producing rows: nothing is read off a big table to
    // be thrown away.
    let outcome = run(
        &connection,
        "SELECT @@session.sql_select_limit; SELECT * FROM big",
        7,
    )
    .await
    .unwrap();
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Rows { rows, .. } if rows[0][0] == Value::Int(8)
    ));
    assert!(matches!(
        &outcome.results[1].outcome,
        StatementOutcome::Rows { rows, truncated: true, .. } if rows.len() == 7
    ));
    // The largest limit does not overflow the setting.
    let outcome = run(&connection, "SELECT id FROM users", u32::MAX)
        .await
        .unwrap();
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Rows { rows, truncated: false, .. } if rows.len() == 5
    ));
}

#[tokio::test]
async fn rows_outside_a_select_are_cut_at_the_limit_too() {
    let Some(connection) = connect().await else {
        return;
    };
    // sql_select_limit does not bound SHOW, nor a SELECT with a LIMIT of
    // its own.
    let outcome = run(
        &connection,
        "SHOW COLUMNS FROM users; SELECT id FROM users LIMIT 4; SELECT 1",
        1,
    )
    .await
    .unwrap();
    for statement in 0..2 {
        assert!(
            matches!(
                &outcome.results[statement].outcome,
                StatementOutcome::Rows { rows, truncated: true, .. } if rows.len() == 1
            ),
            "{outcome:?}"
        );
    }
    // The rest was read off the wire: the next statement runs.
    assert!(matches!(
        &outcome.results[2].outcome,
        StatementOutcome::Rows { rows, truncated: false, .. } if rows[0] == [Value::Int(1)]
    ));
}

#[tokio::test]
async fn statements_without_rows_and_explain() {
    let Some(connection) = connect().await else {
        return;
    };
    let outcome = run(
        &connection,
        "SET @n = 41; SELECT @n + 1; EXPLAIN SELECT * FROM users; DO 1",
        10,
    )
    .await
    .unwrap();
    assert_eq!(outcome.results.len(), 4);
    // SET has no row count; the server's 0 for it is not one.
    assert_eq!(
        outcome.results[0].outcome,
        StatementOutcome::Done { affected: None }
    );
    assert!(matches!(
        &outcome.results[1].outcome,
        StatementOutcome::Rows { rows, .. } if rows[0] == [Value::Int(42)]
    ));
    // EXPLAIN prepares without columns and answers with rows.
    assert!(matches!(
        &outcome.results[2].outcome,
        StatementOutcome::Rows { columns, rows, .. } if rows.len() == 1 && columns[0].name == "id"
    ));
    assert_eq!(
        outcome.results[3].outcome,
        StatementOutcome::Done { affected: None }
    );
}

#[tokio::test]
async fn a_statement_error_stops_the_script_and_keeps_the_session() {
    let Some(connection) = connect().await else {
        return;
    };
    let outcome = run(&connection, "SELECT 1; SELECT nope; SELECT 3", 10)
        .await
        .unwrap();
    assert_eq!(outcome.results.len(), 2);
    assert!(!outcome.stopped);
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Rows { rows, .. } if rows[0] == [Value::Int(1)]
    ));
    assert!(matches!(
        &outcome.results[1].outcome,
        StatementOutcome::Error {
            error: Error::Query { code: Some(code), .. },
            position: None,
        } if code == "42S22"
    ));
    // Not closed: the next run works, with the settings it connected with.
    assert_connect_time_settings(&connection).await;
}

#[tokio::test]
async fn what_the_prepared_protocol_refuses_is_the_statements_error() {
    let Some(connection) = connect().await else {
        return;
    };
    // The server prepares neither: its error is the outcome, and the text
    // never runs another way.
    let outcome = run(&connection, "SELECT 1; USE billing; SELECT 3", 10)
        .await
        .unwrap();
    assert_eq!(outcome.results.len(), 2);
    let StatementOutcome::Error { error, .. } = &outcome.results[1].outcome else {
        panic!("{outcome:?}");
    };
    assert!(
        error.to_string().contains("prepared statement protocol"),
        "{error}"
    );
    // The splitter would never hand over such a piece.
    let mut statements = script("SELECT 1");
    statements[0].text = "SELECT 1; SELECT 2".into();
    let outcome = within(connection.run_script(&statements, 10, &StopFlag::new()))
        .await
        .unwrap();
    assert_eq!(outcome.results.len(), 1);
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Error { error: Error::Query { code: Some(code), .. }, .. } if code == "42000"
    ));
    assert_eq!(
        rows_of(&connection, "SELECT DATABASE()").await[0],
        [Value::Text("tabletist".into())]
    );
}

#[tokio::test]
async fn a_parameter_is_the_statements_error_not_a_lost_session() {
    let Some(connection) = connect().await else {
        return;
    };
    // The driver closes the connection when it is asked to run a statement
    // without values for its parameters, so such a statement never runs.
    // `:name` is the driver's own spelling of a parameter, which it finds
    // by a lexer of its own: after a `-` it takes a string for code.
    for (text, parameter) in [
        ("SELECT 1; SELECT ?; SELECT 3", "?"),
        ("SELECT 1; SELECT :name", ":name"),
        ("SELECT 1; SELECT ?, :name", "?"),
        ("SELECT 1; SELECT HEX(-':abc')", ":abc"),
        (
            "SELECT 1; SELECT 1 -`:abc` FROM (SELECT 2 AS `:abc`) t",
            ":abc",
        ),
    ] {
        let outcome = run(&connection, text, 10).await.unwrap();
        assert_eq!(outcome.results.len(), 2, "{text}");
        assert_eq!(
            outcome.results[1].outcome,
            StatementOutcome::Error {
                error: Error::query(format!(
                    "the statement has a parameter ({parameter}), which the SQL editor cannot fill in"
                )),
                position: None,
            },
            "{text}"
        );
    }
    // In a string, a name or a comment it is text, and reaches the server
    // as it was written.
    assert_eq!(
        rows_of(
            &connection,
            "SELECT '?', ':name' AS `:name`, HEX(- ':abc'), @a := 1 /* :c */ -- :name ?"
        )
        .await[0],
        [
            Value::Text("?".into()),
            Value::Text(":name".into()),
            Value::Text("0".into()),
            Value::Int(1),
        ]
    );
    assert_connect_time_settings(&connection).await;
}

#[tokio::test]
async fn a_nul_character_does_not_lose_the_session() {
    let Some(connection) = connect().await else {
        return;
    };
    // MySQL takes a NUL in a string, and refuses one elsewhere as a syntax
    // error. Either way it is the statement's outcome.
    let outcome = run(&connection, "SELECT 'a\0b'; SELECT 1 \0 + 2; SELECT 3", 10)
        .await
        .unwrap();
    assert_eq!(outcome.results.len(), 2);
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Rows { rows, .. } if rows[0] == [Value::Text("a\0b".into())]
    ));
    assert!(matches!(
        &outcome.results[1].outcome,
        StatementOutcome::Error { error: Error::Query { code: Some(code), .. }, .. } if code == "42000"
    ));
    assert_eq!(rows_of(&connection, "SELECT 1").await[0], [Value::Int(1)]);
}

#[tokio::test]
async fn a_long_script_runs_every_statement() {
    let Some(connection) = connect().await else {
        return;
    };
    // More statements than the driver caches prepared.
    let text = (1..=300)
        .map(|n| format!("SELECT {n}"))
        .collect::<Vec<_>>()
        .join(";\n");
    let outcome = run(&connection, &text, 10).await.unwrap();
    assert_eq!(outcome.results.len(), 300);
    assert!(outcome.results.iter().all(|result| matches!(
        &result.outcome,
        StatementOutcome::Rows { rows, truncated: false, .. } if rows.len() == 1
    )));
    assert!(matches!(
        &outcome.results[299].outcome,
        StatementOutcome::Rows { rows, .. } if rows[0] == [Value::Int(300)]
    ));
}

#[tokio::test]
async fn a_run_leaves_the_session_as_it_found_it() {
    let Some(connection) = connect().await else {
        return;
    };
    assert_connect_time_settings(&connection).await;
    // A small limit, changed settings, a user variable, and a failure at
    // the end.
    let outcome = run(
        &connection,
        "SET time_zone = '+05:00'; SET @tabletist_left_over = 1; \
         SET collation_connection = latin1_swedish_ci; SELECT nope",
        2,
    )
    .await
    .unwrap();
    assert_eq!(outcome.results.len(), 4);
    assert!(matches!(
        outcome.results[3].outcome,
        StatementOutcome::Error { .. }
    ));
    assert_connect_time_settings(&connection).await;
    // sql_select_limit is back to its default: describe and paging see
    // every row (users has more columns and rows than the limit of 2).
    let structure = connection.describe(&users(1).object).await.unwrap();
    assert!(structure.columns.len() > 3);
    let page = connection.fetch_rows(&users(5)).await.unwrap();
    assert_eq!(page.rows.len(), 5);
    assert_eq!(connection.count_rows(&users(1)).await.unwrap(), 5);
}

#[tokio::test]
async fn a_script_keeps_no_lock() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut admin = admin().await;
    admin
        .query_drop("DROP TABLE IF EXISTS script_lock_probe")
        .await
        .unwrap();
    admin
        .query_drop("CREATE TABLE script_lock_probe (id INT PRIMARY KEY)")
        .await
        .unwrap();
    admin
        .query_drop("SET SESSION lock_wait_timeout = 3")
        .await
        .unwrap();
    // A named lock outlives a ROLLBACK, and a transaction left open would
    // keep the table's metadata lock: after a run that worked and one that
    // failed, the session holds neither.
    for (text, results) in [
        (
            "SELECT * FROM script_lock_probe; SELECT GET_LOCK('tabletist_script_lock', 0)",
            2,
        ),
        (
            "SELECT GET_LOCK('tabletist_script_lock', 0); SELECT * FROM script_lock_probe; SELECT nope",
            3,
        ),
    ] {
        let outcome = run(&connection, text, 10).await.unwrap();
        assert_eq!(outcome.results.len(), results, "{text}: {outcome:?}");
        let free: Option<i64> = admin
            .query_first("SELECT IS_FREE_LOCK('tabletist_script_lock')")
            .await
            .unwrap();
        assert_eq!(free, Some(1), "{text}: the session still holds the lock");
        let altered = admin
            .query_drop("ALTER TABLE script_lock_probe COMMENT = 'altered'")
            .await;
        assert!(
            altered.is_ok(),
            "{text}: the script still holds a lock: {altered:?}"
        );
    }
    admin
        .query_drop("DROP TABLE script_lock_probe")
        .await
        .unwrap();
}

/// The rows in the bypass test's `probe` table.
async fn probe_rows(admin: &mut mysql_async::Conn) -> i64 {
    admin
        .query_first("SELECT count(*) FROM probe")
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn bypasses_cannot_write() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut admin = admin().await;
    admin
        .query_drop("CREATE TABLE IF NOT EXISTS probe (n int)")
        .await
        .unwrap();
    admin.query_drop("TRUNCATE probe").await.unwrap();
    // The refusal stops these before anything runs. What the driver does
    // with transaction statements past the refusal is tested next to it
    // (`mysql.rs`).
    for attempt in [
        "COMMIT; SET SESSION TRANSACTION READ WRITE; INSERT INTO probe VALUES (1)",
        "SET @@session.transaction_read_only = 0; INSERT INTO probe VALUES (1)",
        "SET TRANSACTION READ WRITE; INSERT INTO probe VALUES (1)",
        "CALL nothing()",
        "/*!50000 COMMIT */ SELECT 1",
        "CREATE USER sneaky",
        "GRANT ALL ON *.* TO tabletist",
        "SELECT 1 INTO OUTFILE '/tmp/tabletist-probe'",
        "PREPARE s FROM 'INSERT INTO probe VALUES (1)'; EXECUTE s",
        "EXECUTE IMMEDIATE 'INSERT INTO probe VALUES (1)'",
        "SET @a = 1, NAMES gbk",
        "SET sql_mode = 'ANSI_QUOTES'; SELECT \"; INSERT INTO probe VALUES (1); \"",
        "SET GLOBAL read_only = 0",
    ] {
        // Checked before it is run: an account or server statement the
        // guard let through must fail here, not reach the server.
        let statements = script(attempt);
        assert!(
            statements
                .iter()
                .any(
                    |statement| tabletist_db::sql::refusal(Dialect::MySql, &statement.text)
                        .is_some()
                ),
            "{attempt}"
        );
        let ran = within(connection.run_script(&statements, 10, &StopFlag::new())).await;
        assert!(
            matches!(ran, Err(Error::Refused { .. })),
            "{attempt}: {ran:?}"
        );
        assert_eq!(probe_rows(&mut admin).await, 0, "{attempt}");
    }
    // These reach the server, which refuses the write.
    for attempt in [
        "INSERT INTO probe VALUES (1)",
        "SELECT 1; REPLACE INTO probe VALUES (1)",
        "DELETE FROM users",
        "INSERT INTO probe SELECT 1 FROM users",
    ] {
        let outcome = run(&connection, attempt, 10).await.unwrap();
        assert!(
            matches!(
                &outcome.results.last().unwrap().outcome,
                StatementOutcome::Error { error: Error::Query { code: Some(code), .. }, .. }
                    if code == "25006"
            ),
            "{attempt}: {outcome:?}"
        );
        assert_eq!(probe_rows(&mut admin).await, 0, "{attempt}");
    }
    // And browsing still reads, read-only.
    assert_eq!(connection.count_rows(&users(1)).await.unwrap(), 5);
    assert_connect_time_settings(&connection).await;
}

#[tokio::test]
async fn ddl_is_refused_as_read_only_and_ends_the_script() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut admin = admin().await;
    // The server commits before it refuses DDL, so this ends the script's
    // transaction; the session is read-only all the same, and the script
    // stops at the error.
    let outcome = run(
        &connection,
        "SELECT 1; CREATE TABLE tabletist_ddl_probe (n int); SELECT 3",
        10,
    )
    .await
    .unwrap();
    let made: Option<i64> = admin
        .query_first(
            "SELECT count(*) FROM information_schema.tables \
             WHERE table_schema = 'tabletist' AND table_name = 'tabletist_ddl_probe'",
        )
        .await
        .unwrap();
    admin
        .query_drop("DROP TABLE IF EXISTS tabletist_ddl_probe")
        .await
        .unwrap();
    assert_eq!(made, Some(0));
    assert_eq!(outcome.results.len(), 2, "{outcome:?}");
    assert!(matches!(
        &outcome.results[1].outcome,
        StatementOutcome::Error { error: Error::Query { code: Some(code), .. }, .. } if code == "25006"
    ));
    assert_connect_time_settings(&connection).await;
}

/// Waits until a statement holding `marker` runs on the server.
async fn runs_on_the_server(admin: &mut mysql_async::Conn, marker: &str) {
    within(async {
        loop {
            let running: Option<i64> = admin
                .exec_first(
                    "SELECT count(*) FROM information_schema.processlist \
                     WHERE id <> CONNECTION_ID() AND info LIKE ?",
                    (format!("%{marker}%"),),
                )
                .await
                .unwrap();
            if running.unwrap_or(0) > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancelled_script_keeps_earlier_results_and_the_session() {
    let Some(connection) = connect().await else {
        return;
    };
    let connection = std::sync::Arc::new(connection);
    let mut admin = admin().await;
    let cancel = connection.cancel_handle();
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        tokio::spawn(async move {
            // A sleep of its own length, for `runs_on_the_server`. With a
            // table the interrupted SLEEP is an error; alone it answers 1.
            let text = "SELECT 1; SELECT count(*) FROM users WHERE SLEEP(31) = 0; SELECT 3";
            connection.run_script(&script(text), 10, &stop).await
        })
    };
    runs_on_the_server(&mut admin, "SLEEP(31)").await;
    stop.stop();
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    // As the backend does: repeat the cancel until the cleanup begins.
    while !stop.is_finishing() && !running.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "cancel must stop the script"
        );
        cancel.cancel().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let outcome = within(running).await.unwrap().unwrap();
    assert!(outcome.stopped && outcome.was_cancelled());
    assert!(stop.is_finishing());
    assert_eq!(outcome.results.len(), 2);
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Rows { rows, .. } if rows[0] == [Value::Int(1)]
    ));
    assert_eq!(outcome.results[1].outcome, StatementOutcome::Cancelled);
    // The session survives, as it connected.
    assert_connect_time_settings(&connection).await;
    assert_eq!(connection.count_rows(&users(1)).await.unwrap(), 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancels_that_land_on_the_cleanup_leave_the_session_read_only_or_closed() {
    let Some(connection) = connect().await else {
        return;
    };
    let connection = std::sync::Arc::new(connection);
    let mut admin = admin().await;
    let cancel = connection.cancel_handle();
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        tokio::spawn(async move {
            let text = "SELECT 1; SELECT count(*) FROM users WHERE SLEEP(32) = 0";
            connection.run_script(&script(text), 10, &stop).await
        })
    };
    runs_on_the_server(&mut admin, "SLEEP(32)").await;
    stop.stop();
    // Unlike the backend, keep cancelling until the run ends: some cancels
    // land on the cleanup, which runs an interrupted step once more, so the
    // session ends read-only or closed, never read-write.
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while !running.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "cancel must stop the script"
        );
        cancel.cancel().await.unwrap();
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    match within(running).await.unwrap() {
        Ok(outcome) => {
            assert!(outcome.stopped);
            assert!(matches!(
                outcome.results[0].outcome,
                StatementOutcome::Rows { .. }
            ));
            assert_eq!(outcome.results[1].outcome, StatementOutcome::Cancelled);
            assert_connect_time_settings(&connection).await;
        }
        // The backend closes the session on this error.
        Err(error) => assert!(error.is_connection_lost(), "{error}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stop_between_statements_lets_the_running_one_finish() {
    let Some(connection) = connect().await else {
        return;
    };
    let connection = std::sync::Arc::new(connection);
    let mut admin = admin().await;
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        tokio::spawn(async move {
            let text = "SELECT SLEEP(1.5); SELECT 2";
            connection.run_script(&script(text), 10, &stop).await
        })
    };
    // A stop without a cancel: the first statement runs to its end, the
    // second never starts.
    runs_on_the_server(&mut admin, "SLEEP(1.5)").await;
    stop.stop();
    let outcome = within(running).await.unwrap().unwrap();
    assert_eq!(outcome.results.len(), 2);
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Rows { rows, .. } if rows[0] == [Value::Int(0)]
    ));
    assert_eq!(outcome.results[1].outcome, StatementOutcome::Cancelled);
    assert!(outcome.stopped);
    assert!(stop.is_finishing());
    let next = run(&connection, "SELECT 1", 1).await.unwrap();
    assert!(!next.was_cancelled());
}

#[tokio::test]
async fn a_script_stopped_before_it_starts_runs_nothing() {
    let Some(connection) = connect().await else {
        return;
    };
    let stop = StopFlag::new();
    stop.stop();
    let outcome = within(connection.run_script(&script("SELECT 1; SELECT 2"), 10, &stop))
        .await
        .unwrap();
    assert_eq!(outcome.results.len(), 1);
    assert_eq!(outcome.results[0].outcome, StatementOutcome::Cancelled);
    assert!(outcome.stopped && outcome.was_cancelled());
    assert!(stop.is_finishing());
    // The transaction was rolled back: the next run starts its own.
    let next = run(&connection, "SELECT 1", 1).await.unwrap();
    assert!(!next.was_cancelled());
    assert_connect_time_settings(&connection).await;
}

#[tokio::test]
async fn the_server_version_names_the_server() {
    let Some(connection) = connect().await else {
        return;
    };
    let version = within(connection.server_version()).await.unwrap();
    assert!(
        version.starts_with("MySQL ") || version.starts_with("MariaDB "),
        "{version}"
    );
    assert!(!version.contains('-'), "{version}");
    let number = version.split(' ').nth(1).unwrap();
    assert!(
        number.split('.').all(|part| part.parse::<u32>().is_ok()),
        "{version}"
    );
}
