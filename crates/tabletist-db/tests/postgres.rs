//! PostgreSQL through the public `Connection` API. Needs a server: run
//! `docker compose up -d postgres` and set TABLETIST_TEST_PG_URL (see
//! compose.yaml). Without it every test prints "skipped" and passes.

#![allow(clippy::unwrap_used)]

use tabletist_db::{ConnectSpec, Connection, Driver, Error, ObjectKind, Secrets, TlsMode};
use tokio::sync::OnceCell;

static FIXTURE: OnceCell<()> = OnceCell::const_new();

fn url() -> Option<String> {
    std::env::var("TABLETIST_TEST_PG_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
}

/// The spec and secrets from the URL, with TLS turned off unless a test asks.
fn spec() -> Option<(ConnectSpec, Secrets)> {
    let (mut spec, secrets) = ConnectSpec::from_url(&url()?).unwrap();
    spec.tls = TlsMode::Disable;
    Some((spec, secrets))
}

async fn load_fixture() {
    FIXTURE
        .get_or_init(|| async {
            let mut config: tokio_postgres::Config = url().unwrap().parse().unwrap();
            config.ssl_mode(tokio_postgres::config::SslMode::Disable);
            let (client, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
            let task = tokio::spawn(connection);
            client
                .batch_execute(tabletist_db::fixtures::POSTGRES_SQL)
                .await
                .unwrap();
            drop(client);
            let _ = task.await;
        })
        .await;
}

/// A read-only connection to the fixture, or `None` (test skipped).
async fn connect() -> Option<Connection> {
    let Some((spec, secrets)) = spec() else {
        eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
        return None;
    };
    load_fixture().await;
    Some(Connection::connect(&spec, &secrets).await.unwrap())
}

#[tokio::test]
async fn connections_report_postgres_and_list_databases() {
    let Some(connection) = connect().await else {
        return;
    };
    assert_eq!(connection.driver(), Driver::Postgres);
    let databases = connection.list_databases().await.unwrap();
    assert!(databases.contains(&"tabletist".to_owned()), "{databases:?}");
    assert!(!databases.contains(&"template0".to_owned()));
}

#[tokio::test]
async fn schemas_include_public_and_billing() {
    let Some(connection) = connect().await else {
        return;
    };
    let schemas = connection.list_schemas().await.unwrap();
    for schema in ["public", "billing", "pg_catalog"] {
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
    let objects = connection.list_objects("public").await.unwrap();
    let find = |name: &str| objects.iter().find(|object| object.name == name).unwrap();
    assert_eq!(find("users").kind, ObjectKind::Table);
    assert_eq!(find("active_users").kind, ObjectKind::View);
    assert_eq!(find("user_counts").kind, ObjectKind::MaterializedView);
    assert_eq!(find("weird \"name\"").kind, ObjectKind::Table);
    assert_eq!(find("big").estimated_rows, Some(100_000));
    let names: Vec<&str> = objects.iter().map(|object| object.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "objects are sorted by name");
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
            assert_eq!(code.as_deref(), Some("3D000"));
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
    // The compose server has a self-signed certificate.
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
        driver: Driver::Postgres,
        user: String::new(),
        ..ConnectSpec::default()
    };
    assert!(matches!(
        Connection::connect(&spec, &Secrets::default()).await,
        Err(Error::InvalidSpec(_))
    ));
}

use tabletist_db::ObjectRef;

#[tokio::test]
async fn users_structure_has_types_defaults_comments_key_and_indexes() {
    let Some(connection) = connect().await else {
        return;
    };
    let structure = connection
        .describe(&ObjectRef::new("public", "users"))
        .await
        .unwrap();
    let column = |name: &str| structure.columns.iter().find(|c| c.name == name).unwrap();
    assert_eq!(structure.columns[0].name, "id");
    assert_eq!(column("id").type_name, "integer");
    assert!(!column("id").nullable);
    assert_eq!(column("created_at").type_name, "timestamp with time zone");
    assert!(
        column("created_at")
            .default
            .as_deref()
            .unwrap()
            .contains("2026-01-01")
    );
    assert_eq!(column("balance").type_name, "numeric(14,2)");
    assert_eq!(column("tags").type_name, "text[]");
    assert_eq!(column("mood").type_name, "mood");
    assert_eq!(column("email").comment.as_deref(), Some("Login address"));
    assert_eq!(structure.primary_key, vec!["id".to_owned()]);
    let pkey = structure
        .indexes
        .iter()
        .find(|i| i.name == "users_pkey")
        .unwrap();
    assert!(pkey.primary && pkey.unique);
    assert_eq!(pkey.method.as_deref(), Some("btree"));
    let email = structure
        .indexes
        .iter()
        .find(|i| i.name == "users_email_key")
        .unwrap();
    assert!(email.unique && !email.primary);
    assert_eq!(email.columns, vec!["email".to_owned()]);
}

#[tokio::test]
async fn foreign_keys_name_their_target_and_actions() {
    let Some(connection) = connect().await else {
        return;
    };
    let orders = connection
        .describe(&ObjectRef::new("public", "orders"))
        .await
        .unwrap();
    let key = &orders.foreign_keys[0];
    assert_eq!(key.name.as_deref(), Some("orders_user_id_fkey"));
    assert_eq!(key.columns, vec!["user_id".to_owned()]);
    assert_eq!(
        (key.ref_schema.as_str(), key.ref_table.as_str()),
        ("public", "users")
    );
    assert_eq!(key.ref_columns, vec!["id".to_owned()]);
    assert_eq!(key.on_delete, "CASCADE");
    assert_eq!(key.on_update, "NO ACTION");
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
    assert_eq!(invoices.foreign_keys[0].ref_schema, "public");
    assert_eq!(invoices.foreign_keys[0].on_update, "RESTRICT");
    assert_eq!(invoices.foreign_keys[0].on_delete, "SET NULL");
}

#[tokio::test]
async fn views_and_quoted_names_describe_and_missing_objects_fail() {
    let Some(connection) = connect().await else {
        return;
    };
    let view = connection
        .describe(&ObjectRef::new("public", "active_users"))
        .await
        .unwrap();
    assert_eq!(view.columns.len(), 2);
    assert!(view.primary_key.is_empty() && view.indexes.is_empty());
    let weird = connection
        .describe(&ObjectRef::new("public", "weird \"name\""))
        .await
        .unwrap();
    assert_eq!(weird.columns[1].name, "select");
    assert!(matches!(
        connection.describe(&ObjectRef::new("public", "nope")).await,
        Err(Error::Query { .. })
    ));
}

use std::time::Duration;

use tabletist_db::{Filter, FilterOp, RowPage, RowQuery, Sort, SortDir, Value, ValueKind};

fn users(limit: u32) -> RowQuery {
    RowQuery::new(ObjectRef::new("public", "users"), limit)
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
    let page = connection.fetch_rows(&users(2)).await.unwrap();
    assert_eq!(ids(&page), vec![1, 2]);
    assert!(page.has_more && page.ordered_by_key);
    let kind = |name: &str| page.columns.iter().find(|c| c.name == name).unwrap().kind;
    assert_eq!(kind("id"), ValueKind::Numeric);
    assert_eq!(kind("active"), ValueKind::Bool);
    assert_eq!(kind("meta"), ValueKind::Json);
    assert_eq!(kind("created_at"), ValueKind::Temporal);
    assert_eq!(kind("avatar"), ValueKind::Binary);
    assert_eq!(kind("tags"), ValueKind::Other);
    let ada = &page.rows[0];
    assert_eq!(ada[4], Value::Bool(true));
    assert_eq!(ada[6], Value::Bytes(vec![0x89, 0x50, 0x4e, 0x47].into()));
    assert_eq!(ada[7], Value::Float(99.5));
    assert_eq!(ada[8], Value::Text("123456789012.34".into()));
    assert_eq!(ada[9], Value::Text("{math,engines}".into()));
    assert_eq!(ada[10], Value::Text("happy".into()));
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
    let mut deep = RowQuery::new(ObjectRef::new("public", "big"), 300);
    deep.offset = 99_900;
    let page = connection.fetch_rows(&deep).await.unwrap();
    assert_eq!(page.rows.len(), 100);
    assert!(!page.has_more);
    assert_eq!(
        connection
            .count_rows(&RowQuery::new(ObjectRef::new("public", "big"), 1))
            .await
            .unwrap(),
        100_000
    );
    let events = connection
        .fetch_rows(&RowQuery::new(ObjectRef::new("public", "events"), 50))
        .await
        .unwrap();
    assert!(!events.ordered_by_key);
}

#[tokio::test]
async fn filters_work_on_every_type() {
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
        rows(filtered("name", FilterOp::IsNull, "")).await,
        Vec::<i64>::new()
    );
    assert_eq!(
        rows(filtered("meta", FilterOp::IsNull, "")).await,
        vec![2, 4, 5]
    );
    assert_eq!(
        rows(filtered("active", FilterOp::Eq, "false")).await,
        vec![2]
    );
    assert_eq!(
        rows(filtered("name", FilterOp::Contains, "ZO")).await,
        vec![3]
    );
    assert_eq!(
        rows(filtered("name", FilterOp::Contains, "50%")).await,
        vec![4]
    );
    assert_eq!(
        rows(filtered("tags", FilterOp::Contains, "math")).await,
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
async fn quotes_and_backslashes_in_filter_values_are_just_text() {
    let Some(connection) = connect().await else {
        return;
    };
    let rows = connection
        .fetch_rows(&filtered("name", FilterOp::Eq, r"O'Brien C:\temp"))
        .await
        .unwrap();
    assert_eq!(ids(&rows), vec![5]);
    let rows = connection
        .fetch_rows(&filtered("name", FilterOp::Eq, "x' OR '1'='1"))
        .await
        .unwrap();
    assert!(rows.rows.is_empty());
}

#[tokio::test]
async fn a_raw_where_cannot_write_or_leave_read_only_mode() {
    let Some(connection) = connect().await else {
        return;
    };
    for raw in [
        "1 = 1; DELETE FROM users",
        "1=1) ; COMMIT; DELETE FROM users; SELECT (1",
        "nextval('tick') > 0",
    ] {
        let mut query = users(50);
        query.raw_where = Some(raw.into());
        assert!(connection.fetch_rows(&query).await.is_err(), "{raw}");
    }
    // Turning the session default off does not open a writable transaction.
    let mut query = users(50);
    query.raw_where =
        Some("set_config('default_transaction_read_only', 'off', false) IS NOT NULL".into());
    let _ = connection.fetch_rows(&query).await;
    query.raw_where = Some("nextval('tick') > 0".into());
    assert!(connection.fetch_rows(&query).await.is_err());
    assert_eq!(connection.count_rows(&users(1)).await.unwrap(), 5);
}

#[tokio::test]
async fn a_bad_raw_where_is_a_query_error_with_a_code() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut query = users(50);
    query.raw_where = Some("no_such_column = 1".into());
    match connection.fetch_rows(&query).await {
        Err(Error::Query { code, .. }) => assert_eq!(code.as_deref(), Some("42703")),
        other => panic!("{other:?}"),
    }
    // The session is still usable after a failed statement.
    assert!(connection.fetch_rows(&users(1)).await.is_ok());
}

#[tokio::test]
async fn quoted_names_and_views_browse() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut query = RowQuery::new(ObjectRef::new("public", "weird \"name\""), 5);
    query.sort = vec![Sort {
        column: "select".into(),
        dir: SortDir::Asc,
    }];
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(page.columns[0].name, "col with space");
    let view = connection
        .fetch_rows(&RowQuery::new(ObjectRef::new("public", "user_counts"), 5))
        .await
        .unwrap();
    assert_eq!(view.rows.len(), 2);
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
            let mut query = users(5);
            query.raw_where = Some("pg_sleep(30) IS NOT NULL".into());
            connection.count_rows(&query).await
        })
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while !running.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "cancel must stop the query"
        );
        cancel.cancel().await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert_eq!(running.await.unwrap(), Err(Error::Cancelled));
    assert!(connection.fetch_rows(&users(1)).await.is_ok());
}
