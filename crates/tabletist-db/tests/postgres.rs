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
    // Without a CA file, verify-ca would trust any public certificate for
    // any host, so it is refused before connecting.
    spec.tls = TlsMode::VerifyCa;
    spec.ca_file = None;
    let result = Connection::connect(&spec, &secrets).await;
    assert!(
        matches!(&result, Err(Error::Tls(message)) if message.contains("needs a CA file")),
        "{:?}",
        result.err()
    );
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
async fn enums_and_check_lists_describe_their_allowed_values() {
    let Some(connection) = connect().await else {
        return;
    };
    let structure = connection
        .describe(&ObjectRef::new("public", "tickets"))
        .await
        .unwrap();
    let allowed = |name: &str| {
        structure
            .columns
            .iter()
            .find(|c| c.name == name)
            .unwrap()
            .allowed_values
            .clone()
    };
    let list = |items: &[&str]| Some(items.iter().map(|item| (*item).to_owned()).collect());
    assert_eq!(
        allowed("size"),
        list(&["small", "medium", "large"]),
        "enum labels in their sort order"
    );
    assert_eq!(allowed("status"), list(&["open", "closed"]), "IN (...)");
    assert_eq!(
        allowed("channel"),
        list(&["web", "phone"]),
        "= ANY (ARRAY[...])"
    );
    assert_eq!(allowed("code"), None, "a CHECK that is no list");
    assert_eq!(allowed("kind"), None, "free text, however it repeats");
    assert_eq!(allowed("grade").map(|values| values.len()), Some(9));
    assert_eq!(allowed("done"), None, "booleans need no list");
    assert_eq!(allowed("id"), None);
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
    // Values are E'' literals, so the result does not hang on the session's
    // standard_conforming_strings.
    for (op, value, expected) in [
        (FilterOp::In, r"Bob, O'Brien C:\temp", vec![2, 5]),
        (FilterOp::In, r"x\', 1) OR (1=1", vec![]),
        (FilterOp::Contains, r"'brien c:\t", vec![5]),
        (FilterOp::Contains, r"\", vec![5]),
        (FilterOp::StartsWith, r"O'Brien C:\", vec![5]),
        (FilterOp::Contains, r"x\' OR 1=1 --", vec![]),
    ] {
        let rows = connection
            .fetch_rows(&filtered("name", op, value))
            .await
            .unwrap_or_else(|error| panic!("{op:?} {value}: {error}"));
        assert_eq!(ids(&rows), expected, "{op:?} {value}");
    }
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

use tabletist_db::{Dialect, ScriptOutcome, StatementOutcome, StopFlag};

/// A writable session for the bypass test's probe table.
async fn admin() -> tokio_postgres::Client {
    let mut config: tokio_postgres::Config = url().unwrap().parse().unwrap();
    config.ssl_mode(tokio_postgres::config::SslMode::Disable);
    let (client, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
    tokio::spawn(connection);
    client
}

/// The first column of each row of `object` the filter keeps, in key
/// order, after checking the count agrees.
async fn kept(
    connection: &Connection,
    object: &str,
    column: &str,
    op: FilterOp,
    value: &str,
) -> Vec<i64> {
    let mut query = RowQuery::new(ObjectRef::new("public", object), 50);
    query.filters = vec![Filter {
        column: column.into(),
        op,
        value: value.into(),
    }];
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(
        connection.count_rows(&query).await.unwrap(),
        page.rows.len() as u64,
        "{column} {op:?} {value}"
    );
    ids(&page)
}

/// A `bytea` key of sixteen bytes shows as a UUID, and other binary as `0x`
/// hex: neither text is `bytea` input, so the filter has to write the bytes.
/// A `uuid` column takes the text as it is.
#[tokio::test]
async fn a_uuid_or_hex_filter_matches_a_bytea_key_and_its_foreign_key() {
    let Some(connection) = connect().await else {
        return;
    };
    let admin = admin().await;
    admin
        .batch_execute(
            r"DROP TABLE IF EXISTS uuid_memberships, uuid_accounts;
              CREATE TABLE uuid_accounts (n int NOT NULL, id bytea PRIMARY KEY, ref uuid, slug text);
              INSERT INTO uuid_accounts VALUES
                  (1, '\x0199a3f27c1e7abc8def0123456789ab',
                   '0199a3f2-7c1e-7abc-8def-0123456789ab', '0199a3f2-7c1e-7abc-8def-0123456789ab'),
                  (2, '\x0199a3f27c1e7abc8def0123456789ac',
                   '0199a3f2-7c1e-7abc-8def-0123456789ac', '0199a3f27c1e7abc8def0123456789ac'),
                  (3, convert_to('0199a3f2-7c1e-7abc-8def-0123456789ad', 'UTF8'), NULL, '0xcafe');
              CREATE TABLE uuid_memberships (
                  id int PRIMARY KEY, account_id bytea NOT NULL REFERENCES uuid_accounts (id)
              );
              INSERT INTO uuid_memberships VALUES
                  (1, '\x0199a3f27c1e7abc8def0123456789ab'),
                  (2, '\x0199a3f27c1e7abc8def0123456789ac'),
                  (3, '\x0199a3f27c1e7abc8def0123456789ab');",
        )
        .await
        .unwrap();
    let ab = "0199a3f2-7c1e-7abc-8def-0123456789ab";
    for (column, op, value, expected) in [
        ("id", FilterOp::Eq, ab, vec![1]),
        (
            "id",
            FilterOp::Eq,
            " 0199A3F2-7C1E-7ABC-8DEF-0123456789AB ",
            vec![1],
        ),
        (
            "id",
            FilterOp::Eq,
            "0199a3f27c1e7abc8def0123456789ac",
            vec![2],
        ),
        (
            "id",
            FilterOp::Eq,
            "0x0199a3f27c1e7abc8def0123456789ab",
            vec![1],
        ),
        ("id", FilterOp::Ne, ab, vec![2, 3]),
        (
            "id",
            FilterOp::In,
            "0199a3f2-7c1e-7abc-8def-0123456789ab, 0x0199a3f27c1e7abc8def0123456789ac, nothing",
            vec![1, 2],
        ),
        // A UUID kept as text in a bytea column still matches as text.
        (
            "id",
            FilterOp::Eq,
            "0199a3f2-7c1e-7abc-8def-0123456789ad",
            vec![3],
        ),
        // uuid and text columns take the text as it is.
        ("ref", FilterOp::Eq, ab, vec![1]),
        (
            "ref",
            FilterOp::Eq,
            "0199a3f27c1e7abc8def0123456789ac",
            vec![2],
        ),
        (
            "ref",
            FilterOp::In,
            "0199a3f2-7c1e-7abc-8def-0123456789ab, 0199a3f27c1e7abc8def0123456789ac",
            vec![1, 2],
        ),
        ("slug", FilterOp::Eq, ab, vec![1]),
        ("slug", FilterOp::Eq, "0xcafe", vec![3]),
    ] {
        assert_eq!(
            kept(&connection, "uuid_accounts", column, op, value).await,
            expected,
            "{column} {op:?} {value}"
        );
    }

    // Following the foreign key: the value is what the app shows for it.
    let memberships = ObjectRef::new("public", "uuid_memberships");
    let structure = connection.describe(&memberships).await.unwrap();
    let foreign = &structure.foreign_keys[0];
    assert_eq!(foreign.columns, ["account_id"]);
    assert_eq!(foreign.ref_table, "uuid_accounts");
    let page = connection
        .fetch_rows(&RowQuery::new(memberships, 50))
        .await
        .unwrap();
    assert_eq!(page.columns[1].kind, ValueKind::Binary);
    let Value::Bytes(key) = &page.rows[1][1] else {
        panic!("account_id is {:?}", page.rows[1][1]);
    };
    let hex: String = key.iter().map(|byte| format!("{byte:02x}")).collect();
    let uuid = format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    );
    assert_eq!(
        kept(
            &connection,
            "uuid_accounts",
            &foreign.ref_columns[0],
            FilterOp::Eq,
            &uuid
        )
        .await,
        vec![2]
    );
    assert_eq!(
        kept(
            &connection,
            "uuid_memberships",
            "account_id",
            FilterOp::Eq,
            ab
        )
        .await,
        vec![1, 3]
    );
    admin
        .batch_execute("DROP TABLE uuid_memberships, uuid_accounts")
        .await
        .unwrap();
}

fn script(text: &str) -> Vec<tabletist_db::sql::Statement> {
    tabletist_db::sql::statements(Dialect::Postgres, text)
}

/// Awaits `future`, failing the test instead of hanging it.
async fn within<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), future)
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

#[tokio::test]
async fn a_script_has_typed_columns_and_truncates_at_the_limit() {
    let Some(connection) = connect().await else {
        return;
    };
    let outcome = run(&connection, "SELECT id, active FROM users ORDER BY id", 2)
        .await
        .unwrap();
    let StatementOutcome::Rows {
        columns,
        rows,
        truncated,
    } = &outcome.results[0].outcome
    else {
        panic!("rows");
    };
    assert_eq!(columns[1].kind, ValueKind::Bool);
    assert_eq!(rows.len(), 2);
    assert!(*truncated);
}

#[tokio::test]
async fn truncates_at_the_limit() {
    let Some(connection) = connect().await else {
        return;
    };
    let started = std::time::Instant::now();
    // In the select list, generate_series streams (in FROM it would
    // materialise every row first).
    let outcome = run(&connection, "SELECT generate_series(1, 50000000) AS g", 10)
        .await
        .unwrap();
    assert!(matches!(
        outcome.results[0].outcome,
        StatementOutcome::Rows {
            truncated: true,
            ..
        }
    ));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn unions_in_parentheses_and_trailing_comments_use_the_cursor() {
    let Some(connection) = connect().await else {
        return;
    };
    let outcome = run(
        &connection,
        "(SELECT 1 AS n) UNION ALL (SELECT 2) -- note",
        1,
    )
    .await
    .unwrap();
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Rows { rows, truncated: true, .. } if rows.len() == 1
    ));
}

#[tokio::test]
async fn show_and_statements_without_rows() {
    let Some(connection) = connect().await else {
        return;
    };
    let outcome = run(
        &connection,
        "SHOW search_path; SET LOCAL work_mem = '8MB'",
        10,
    )
    .await
    .unwrap();
    assert!(matches!(
        outcome.results[0].outcome,
        StatementOutcome::Rows { .. }
    ));
    // SET has no row count; the driver's 0 for its command tag is not one.
    assert_eq!(
        outcome.results[1].outcome,
        StatementOutcome::Done { affected: None }
    );
}

#[tokio::test]
async fn a_statement_error_keeps_the_session() {
    let Some(connection) = connect().await else {
        return;
    };
    let outcome = run(
        &connection,
        "SELECT 1;\nSELECT a,\n  bogus FROM users; SELECT 3",
        10,
    )
    .await
    .unwrap();
    assert_eq!(outcome.results.len(), 2);
    let StatementOutcome::Error { error, position } = &outcome.results[1].outcome else {
        panic!("error");
    };
    assert!(matches!(error, Error::Query { code: Some(code), .. } if code == "42703"));
    // Points at "a" (1-based, into the statement's own text).
    assert_eq!(*position, Some(8));
    // Not closed: the next run works.
    run(&connection, "SELECT 1", 1).await.unwrap();
}

#[tokio::test]
async fn a_declare_error_points_into_the_users_text() {
    let Some(connection) = connect().await else {
        return;
    };
    // Prepares (it is a valid WITH) but DECLARE refuses a data-modifying WITH.
    let outcome = run(
        &connection,
        "WITH gone AS (DELETE FROM users RETURNING id) SELECT * FROM gone",
        10,
    )
    .await
    .unwrap();
    // The server names no position for this one.
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Error {
            error: Error::Query { code: Some(code), .. },
            position: None,
        } if code == "0A000"
    ));
    // A parameter prepares, but DECLARE has no value for it. The server's
    // position counts the cursor prefix; the outcome's does not.
    let outcome = run(&connection, "SELECT\n  $1::int AS n", 10)
        .await
        .unwrap();
    let StatementOutcome::Error { error, position } = &outcome.results[0].outcome else {
        panic!("error");
    };
    assert!(matches!(error, Error::Query { code: Some(code), .. } if code == "42P02"));
    assert_eq!(*position, Some(10));
}

/// The count of rows in the bypass tests' `probe` table.
async fn probe_rows(admin: &tokio_postgres::Client) -> i64 {
    admin
        .query_one("SELECT count(*) FROM probe", &[])
        .await
        .unwrap()
        .get(0)
}

#[tokio::test]
async fn bypasses_cannot_write() {
    let Some(connection) = connect().await else {
        return;
    };
    let admin = admin().await;
    admin
        .batch_execute("CREATE TABLE IF NOT EXISTS probe (n int); TRUNCATE probe;")
        .await
        .unwrap();
    // The refusal stops these before anything runs. What the driver does
    // with them past the refusal is tested next to it (`pg/script.rs`).
    for attempt in [
        "SET TRANSACTION READ WRITE; INSERT INTO probe VALUES (1)",
        "ROLLBACK; SET default_transaction_read_only = off; INSERT INTO probe VALUES (1)",
        "SET \"default_transaction_read_only\" = off; INSERT INTO probe VALUES (1)",
        "SELECT set_config('default_transaction_read_only', 'off', false); INSERT INTO probe VALUES (1)",
        "COPY probe FROM STDIN",
        "PREPARE s AS INSERT INTO probe VALUES (1); EXECUTE s",
    ] {
        let ran = run(&connection, attempt, 10).await;
        assert!(
            matches!(ran, Err(Error::Refused { .. })),
            "{attempt}: {ran:?}"
        );
        assert_eq!(probe_rows(&admin).await, 0, "{attempt}");
    }
    // These reach the server, which refuses the write.
    for attempt in [
        "INSERT INTO probe VALUES (1)",
        "DO $$ BEGIN INSERT INTO probe VALUES (1); END $$",
    ] {
        let outcome = run(&connection, attempt, 10).await.unwrap();
        assert_eq!(outcome.results.len(), 1, "{attempt}");
        assert!(
            matches!(
                &outcome.results[0].outcome,
                StatementOutcome::Error { error: Error::Query { code: Some(code), .. }, .. }
                    if code == "25006"
            ),
            "{attempt}: {outcome:?}"
        );
        assert_eq!(probe_rows(&admin).await, 0, "{attempt}");
    }
    // And browsing still reads, read-only.
    assert!(connection.fetch_rows(&users(1)).await.is_ok());
}

/// Waits until a statement holding `marker` runs on the server.
async fn runs_on_the_server(admin: &tokio_postgres::Client, marker: &str) {
    within(async {
        loop {
            let running: i64 = admin
                .query_one(
                    "SELECT count(*) FROM pg_stat_activity \
                     WHERE state = 'active' AND pid <> pg_backend_pid() \
                       AND position($1 in query) > 0",
                    &[&marker],
                )
                .await
                .unwrap()
                .get(0);
            if running > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
}

/// Stops and cancels a script once its statement holding `marker` runs.
async fn cancelled_while_running(
    text: &'static str,
    limit: u32,
    marker: &str,
) -> Option<ScriptOutcome> {
    let connection = std::sync::Arc::new(connect().await?);
    let admin = admin().await;
    let cancel = connection.cancel_handle();
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        tokio::spawn(async move { connection.run_script(&script(text), limit, &stop).await })
    };
    runs_on_the_server(&admin, marker).await;
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
    assert!(outcome.stopped);
    assert!(stop.is_finishing());
    // The session survives.
    let next = run(&connection, "SELECT 1", 1).await.unwrap();
    assert!(matches!(
        next.results[0].outcome,
        StatementOutcome::Rows { .. }
    ));
    Some(outcome)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancel_stops_a_statement_that_streams_its_rows() {
    let Some(outcome) = cancelled_while_running(
        "SELECT 1; EXPLAIN ANALYZE SELECT /* tabletist streams */ pg_sleep(30)",
        10,
        "/* tabletist streams */",
    )
    .await
    else {
        return;
    };
    assert_eq!(outcome.results.len(), 2);
    assert!(matches!(
        outcome.results[0].outcome,
        StatementOutcome::Rows { .. }
    ));
    assert_eq!(outcome.results[1].outcome, StatementOutcome::Cancelled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancel_stops_a_statement_without_rows() {
    let Some(outcome) = cancelled_while_running(
        "SELECT 1; DO $$ BEGIN /* tabletist blocks */ PERFORM pg_sleep(30); END $$",
        10,
        "/* tabletist blocks */",
    )
    .await
    else {
        return;
    };
    assert_eq!(outcome.results.len(), 2);
    assert_eq!(outcome.results[1].outcome, StatementOutcome::Cancelled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stop_between_statements_lets_the_running_one_finish() {
    let Some(connection) = connect().await else {
        return;
    };
    let connection = std::sync::Arc::new(connection);
    let admin = admin().await;
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        tokio::spawn(async move {
            let text = "DO $$ BEGIN /* tabletist between */ PERFORM pg_sleep(1); END $$; SELECT 2";
            connection.run_script(&script(text), 10, &stop).await
        })
    };
    // A stop without a cancel: the first statement runs to its end, the
    // second never starts.
    runs_on_the_server(&admin, "/* tabletist between */").await;
    stop.stop();
    let outcome = within(running).await.unwrap().unwrap();
    assert_eq!(outcome.results.len(), 2);
    assert_eq!(
        outcome.results[0].outcome,
        StatementOutcome::Done { affected: None }
    );
    assert_eq!(outcome.results[1].outcome, StatementOutcome::Cancelled);
    assert!(outcome.stopped);
    assert!(stop.is_finishing());
    let next = run(&connection, "SELECT 1", 1).await.unwrap();
    assert!(!next.was_cancelled());
}

#[tokio::test]
async fn a_long_script_runs_every_statement() {
    let Some(connection) = connect().await else {
        return;
    };
    let text = (1..=300)
        .map(|n| format!("SELECT {n}"))
        .collect::<Vec<_>>()
        .join(";\n");
    let outcome = run(&connection, &text, 10).await.unwrap();
    assert_eq!(outcome.results.len(), 300);
    assert!(outcome.results.iter().all(|result| matches!(
        &result.outcome,
        StatementOutcome::Rows { rows, .. } if rows.len() == 1
    )));
    assert!(matches!(
        &outcome.results[299].outcome,
        StatementOutcome::Rows { rows, truncated: false, .. } if rows[0] == [Value::Int(300)]
    ));
}

#[tokio::test]
async fn a_nul_character_is_the_statements_error() {
    let Some(connection) = connect().await else {
        return;
    };
    // The driver cannot send a NUL. That is the statement's error: earlier
    // results stay, and the transaction is still rolled back.
    let outcome = run(&connection, "SELECT 1; SELECT '\0' AS n; SELECT 3", 10)
        .await
        .unwrap();
    assert_eq!(outcome.results.len(), 2);
    assert!(matches!(
        outcome.results[0].outcome,
        StatementOutcome::Rows { .. }
    ));
    assert_eq!(
        outcome.results[1].outcome,
        StatementOutcome::Error {
            error: Error::query("SQL text cannot contain a NUL character"),
            position: Some(9),
        }
    );
    let next = run(&connection, "SELECT 1", 1).await.unwrap();
    assert!(matches!(
        next.results[0].outcome,
        StatementOutcome::Rows { .. }
    ));
}

#[tokio::test]
async fn a_script_keeps_no_advisory_lock() {
    let Some(connection) = connect().await else {
        return;
    };
    // A session lock outlives the ROLLBACK; the run releases it.
    let outcome = run(&connection, "SELECT pg_advisory_lock(424242)", 10)
        .await
        .unwrap();
    assert!(matches!(
        outcome.results[0].outcome,
        StatementOutcome::Rows { .. }
    ));
    let other = admin().await;
    let taken: bool = other
        .query_one("SELECT pg_try_advisory_lock(424242)", &[])
        .await
        .unwrap()
        .get(0);
    assert!(taken, "the script's session still holds the lock");
    other
        .batch_execute("SELECT pg_advisory_unlock(424242)")
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancelled_script_keeps_earlier_results_and_the_session() {
    // A row query sleeps in its FETCH, which does not hold the script's
    // text. A limit no other test uses makes the FETCH this test's own.
    let Some(outcome) = cancelled_while_running(
        "SELECT 1; SELECT pg_sleep(30)",
        7,
        "FETCH 8 FROM tabletist_sql",
    )
    .await
    else {
        return;
    };
    assert_eq!(outcome.results.len(), 2);
    assert!(matches!(
        outcome.results[0].outcome,
        StatementOutcome::Rows { .. }
    ));
    assert_eq!(outcome.results[1].outcome, StatementOutcome::Cancelled);
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
}

#[tokio::test]
async fn rows_outside_a_cursor_are_cut_at_the_limit_too() {
    let Some(connection) = connect().await else {
        return;
    };
    // EXPLAIN cannot be a cursor; its plan here has several lines.
    let outcome = run(
        &connection,
        "EXPLAIN SELECT * FROM users a JOIN users b ON a.id = b.id; SELECT 1",
        1,
    )
    .await
    .unwrap();
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Rows { rows, truncated: true, .. } if rows.len() == 1
    ));
    // The rest of the plan was read off the wire: the next statement runs.
    assert!(matches!(
        &outcome.results[1].outcome,
        StatementOutcome::Rows { rows, truncated: false, .. } if rows.len() == 1
    ));
}

#[tokio::test]
async fn one_piece_cannot_hold_two_statements() {
    let Some(connection) = connect().await else {
        return;
    };
    // The splitter would never hand over such a piece; the prepare refuses
    // it anyway, and the text does not run another way.
    let mut statements = script("SELECT 1");
    statements[0].text = "SELECT 1; SELECT 2".into();
    let outcome = within(connection.run_script(&statements, 10, &StopFlag::new()))
        .await
        .unwrap();
    assert_eq!(outcome.results.len(), 1);
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Error { error: Error::Query { code: Some(code), .. }, .. } if code == "42601"
    ));
}

#[tokio::test]
async fn the_server_version_has_no_distribution_suffix() {
    let Some(connection) = connect().await else {
        return;
    };
    let version = within(connection.server_version()).await.unwrap();
    assert!(version.starts_with("PostgreSQL "), "{version}");
    assert!(!version.contains('('), "{version}");
}
