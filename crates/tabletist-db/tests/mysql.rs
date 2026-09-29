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
