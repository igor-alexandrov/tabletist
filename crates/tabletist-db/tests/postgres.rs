//! PostgreSQL through the public `Connection` API. Needs a server: run
//! `docker compose up -d postgres` and set TABLETIST_TEST_PG_URL (see
//! compose.yaml). Without it every test prints "skipped" and passes.

#![allow(clippy::unwrap_used)]

use tabletist_db::{
    Access, ConnectSpec, Connection, Driver, Error, HostKeys, ObjectKind, Secrets, TlsMode,
};
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

/// A connection to the fixture with the given access, or `None` (test skipped).
async fn connect_as(access: Access) -> Option<Connection> {
    let Some((spec, secrets)) = spec() else {
        eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
        return None;
    };
    load_fixture().await;
    Some(
        Connection::connect_with(&spec, &secrets, &HostKeys::default(), access)
            .await
            .unwrap(),
    )
}

/// A read-only connection to the fixture, or `None` (test skipped).
async fn connect() -> Option<Connection> {
    connect_as(Access::ReadOnly).await
}

#[tokio::test]
async fn connections_report_postgres_and_list_databases() {
    let Some(connection) = connect().await else {
        return;
    };
    assert_eq!(connection.driver(), Driver::Postgres);
    let databases = connection.list_databases().await.unwrap();
    let (spec, _) = spec().unwrap();
    let configured = if spec.database.is_empty() {
        spec.user
    } else {
        spec.database
    };
    assert!(databases.contains(&configured), "{databases:?}");
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

#[tokio::test]
async fn generated_and_always_identity_columns_say_so() {
    let Some(connection) = connect().await else {
        return;
    };
    let admin = admin().await;
    admin
        .batch_execute(
            "DROP TABLE IF EXISTS catalog_generated;
             CREATE TABLE catalog_generated (
                 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                 seq bigint GENERATED BY DEFAULT AS IDENTITY,
                 title text NOT NULL,
                 slug text GENERATED ALWAYS AS (lower(title)) STORED
             )",
        )
        .await
        .unwrap();
    let structure = connection
        .describe(&ObjectRef::new("public", "catalog_generated"))
        .await;
    admin
        .batch_execute("DROP TABLE catalog_generated")
        .await
        .unwrap();
    let generated: Vec<(String, bool)> = structure
        .unwrap()
        .columns
        .into_iter()
        .map(|column| (column.name, column.generated))
        .collect();
    assert_eq!(
        generated,
        [
            ("id".to_owned(), true),
            // By default: the database takes a value when given one.
            ("seq".to_owned(), false),
            ("title".to_owned(), false),
            ("slug".to_owned(), true),
        ]
    );
}

#[tokio::test]
async fn a_partial_index_says_so_and_is_not_the_row_key() {
    let Some(connection) = connect().await else {
        return;
    };
    let admin = admin().await;
    admin
        .batch_execute(
            "DROP TABLE IF EXISTS catalog_partial;
             CREATE TABLE catalog_partial (a integer NOT NULL, b integer NOT NULL);
             CREATE UNIQUE INDEX catalog_partial_whole ON catalog_partial (a);
             CREATE UNIQUE INDEX catalog_partial_part ON catalog_partial (b) WHERE b > 0",
        )
        .await
        .unwrap();
    let structure = connection
        .describe(&ObjectRef::new("public", "catalog_partial"))
        .await;
    admin
        .batch_execute("DROP TABLE catalog_partial")
        .await
        .unwrap();
    let structure = structure.unwrap();
    let partial: Vec<(&str, bool)> = structure
        .indexes
        .iter()
        .map(|index| (index.name.as_str(), index.partial))
        .collect();
    assert_eq!(
        partial,
        [
            ("catalog_partial_part", true),
            ("catalog_partial_whole", false)
        ]
    );
    // `catalog_partial_part` comes first by name and its column cannot be
    // NULL: only its condition keeps it from being the key.
    assert_eq!(structure.row_key(), Some(vec!["a".to_owned()]));
}

/// An index shows `a` for a column under another collation or operator
/// class, and an expression as its text, which a column may be called too.
/// Only an index over whole columns, compared as they compare, is a key.
#[tokio::test]
async fn only_an_index_over_whole_columns_as_they_compare_is_the_row_key() {
    let Some(connection) = connect().await else {
        return;
    };
    let admin = admin().await;
    admin
        .batch_execute(
            r#"DROP TABLE IF EXISTS catalog_keys;
               DROP COLLATION IF EXISTS catalog_keys_nocase;
               CREATE COLLATION catalog_keys_nocase
                   (provider = icu, locale = 'und-u-ks-level2', deterministic = false);
               CREATE TABLE catalog_keys (
                   a text COLLATE catalog_keys_nocase NOT NULL,
                   b text NOT NULL,
                   "lower(b)" text NOT NULL,
                   c integer NOT NULL,
                   d text NOT NULL,
                   "Odd name" text NOT NULL,
                   e integer
               );
               CREATE UNIQUE INDEX catalog_keys_1_collated ON catalog_keys (a COLLATE "C");
               CREATE UNIQUE INDEX catalog_keys_2_expression ON catalog_keys (lower(b));
               CREATE UNIQUE INDEX catalog_keys_3_pattern ON catalog_keys (d text_pattern_ops);
               CREATE UNIQUE INDEX catalog_keys_4_mixed ON catalog_keys (c, lower(b));
               CREATE UNIQUE INDEX catalog_keys_5_plain
                   ON catalog_keys (c, "Odd name" DESC) INCLUDE (e);
               INSERT INTO catalog_keys VALUES
                   ('x', 'p', 'q', 1, 'r', 's', NULL), ('X', 't', 'u', 2, 'v', 'w', NULL)"#,
        )
        .await
        .unwrap();
    let structure = connection
        .describe(&ObjectRef::new("public", "catalog_keys"))
        .await;
    let twins = admin
        .query_one("SELECT count(*) FROM catalog_keys WHERE a = 'x'", &[])
        .await;
    admin
        .batch_execute("DROP TABLE catalog_keys; DROP COLLATION catalog_keys_nocase")
        .await
        .unwrap();
    // Why the first index is no key: unique byte for byte, and the column
    // compares without case.
    assert_eq!(twins.unwrap().get::<_, i64>(0), 2);
    let structure = structure.unwrap();
    let list =
        |names: &[&str]| -> Vec<String> { names.iter().map(|name| (*name).to_owned()).collect() };
    let indexes: Vec<_> = structure
        .indexes
        .iter()
        .map(|index| {
            (
                index.name.as_str(),
                index.columns.clone(),
                index.key_columns.clone(),
            )
        })
        .collect();
    assert_eq!(
        indexes,
        [
            ("catalog_keys_1_collated", list(&["a"]), None),
            ("catalog_keys_2_expression", list(&["lower(b)"]), None),
            ("catalog_keys_3_pattern", list(&["d"]), None),
            ("catalog_keys_4_mixed", list(&["c", "lower(b)"]), None),
            (
                "catalog_keys_5_plain",
                list(&["c", "\"Odd name\""]),
                Some(list(&["c", "Odd name"]))
            ),
        ]
    );
    assert_eq!(structure.row_key(), Some(list(&["c", "Odd name"])));
}

/// The columns a primary key only carries along (`INCLUDE`) are in its index
/// and are no part of the key: they may repeat, and may be NULL.
#[tokio::test]
async fn a_primary_keys_included_columns_are_not_key_columns() {
    let Some(connection) = connect().await else {
        return;
    };
    let admin = admin().await;
    admin
        .batch_execute(
            "DROP TABLE IF EXISTS catalog_included;
             CREATE TABLE catalog_included (
                 id integer, payload text, PRIMARY KEY (id) INCLUDE (payload)
             )",
        )
        .await
        .unwrap();
    let structure = connection
        .describe(&ObjectRef::new("public", "catalog_included"))
        .await;
    admin
        .batch_execute("DROP TABLE catalog_included")
        .await
        .unwrap();
    let structure = structure.unwrap();
    assert_eq!(structure.primary_key, vec!["id".to_owned()]);
    assert_eq!(structure.row_key(), Some(vec!["id".to_owned()]));
}

/// A `CREATE UNIQUE INDEX CONCURRENTLY` that failed leaves its index behind,
/// still called unique, over rows that are not.
#[tokio::test]
async fn an_index_left_invalid_does_not_count_as_unique() {
    let Some(connection) = connect().await else {
        return;
    };
    let admin = admin().await;
    admin
        .batch_execute(
            "DROP TABLE IF EXISTS catalog_invalid;
             CREATE TABLE catalog_invalid (a integer NOT NULL);
             INSERT INTO catalog_invalid VALUES (1), (1)",
        )
        .await
        .unwrap();
    // On its own: it cannot run in the transaction a batch is.
    let built = admin
        .batch_execute(
            "CREATE UNIQUE INDEX CONCURRENTLY catalog_invalid_broken ON catalog_invalid (a)",
        )
        .await;
    let structure = connection
        .describe(&ObjectRef::new("public", "catalog_invalid"))
        .await;
    admin
        .batch_execute("DROP TABLE catalog_invalid")
        .await
        .unwrap();
    assert!(built.is_err(), "the duplicates must fail the build");
    let structure = structure.unwrap();
    let unique: Vec<(&str, bool)> = structure
        .indexes
        .iter()
        .map(|index| (index.name.as_str(), index.unique))
        .collect();
    assert_eq!(unique, [("catalog_invalid_broken", false)]);
    assert_eq!(structure.row_key(), None);
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
    for access in [Access::ReadOnly, Access::Writable] {
        let Some(connection) = connect_as(access).await else {
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

/// A writable session for arranging and probing, outside the adapter.
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
async fn a_writable_session_keeps_the_servers_default_and_a_script_still_runs_read_only() {
    for (access, default) in [(Access::ReadOnly, "on"), (Access::Writable, "off")] {
        let Some(connection) = connect_as(access).await else {
            return;
        };
        let outcome = run(
            &connection,
            "SHOW default_transaction_read_only; SHOW transaction_read_only",
            10,
        )
        .await
        .unwrap();
        let shown = |index: usize| match &outcome.results[index].outcome {
            StatementOutcome::Rows { rows, .. } => rows[0][0].clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(shown(0), Value::Text(default.into()), "{access:?}");
        // The script's own transaction is read-only whatever the session.
        assert_eq!(shown(1), Value::Text("on".into()), "{access:?}");
    }
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
    for access in [Access::ReadOnly, Access::Writable] {
        let Some(connection) = connect_as(access).await else {
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

use std::future::Future;
use std::panic::AssertUnwindSafe;

use futures_util::FutureExt;
use tabletist_db::{CellChange, ChangeSet, Conflict, NewValue, RowChange, WriteOutcome};

/// Runs `test` on tables of its own. The fixture is loaded once and shared
/// by every test of this suite, so a test that writes never touches it, nor
/// its types (the fixture drops those without CASCADE when it loads, and a
/// table left using one would fail every test from then on). `create` makes
/// what the test writes to, under names no other test uses, and `drop`
/// removes it however the test ends, a failed assertion included, and
/// before anything else, for what a run that was killed left behind.
async fn on_its_own_tables<T>(drop: &str, create: &str, test: impl Future<Output = T>) -> T {
    let admin = admin().await;
    admin.batch_execute(drop).await.unwrap();
    let outcome = AssertUnwindSafe(async {
        admin.batch_execute(create).await.unwrap();
        test.await
    })
    .catch_unwind()
    .await;
    // A save that left its transaction open would hold the table, and the
    // drop would wait for it for good: it gives up, and the test fails.
    let dropped = admin
        .batch_execute(&format!("SET lock_timeout = '10s'; {drop}"))
        .await;
    let value = outcome.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
    dropped.unwrap();
    value
}

/// A table of three people named `table`, for a test to write to.
fn people(table: &str) -> String {
    format!(
        "CREATE TABLE {table} (
             id integer PRIMARY KEY,
             email text NOT NULL UNIQUE,
             name text,
             score double precision
         );
         INSERT INTO {table} VALUES
             (1, 'a@x', 'Ada', 0.1), (2, 'b@x', 'Bea', NULL), (3, 'c@x', 'Cy', 3)"
    )
}

fn drop_table(table: &str) -> String {
    format!("DROP TABLE IF EXISTS {table}")
}

/// The rows of `table` as a page gives them, in key order, and the names
/// of their columns.
async fn page_of(connection: &Connection, table: &str) -> (Vec<String>, Vec<Vec<Value>>) {
    let query = RowQuery::new(ObjectRef::new("public", table), 50);
    let page = connection.fetch_rows(&query).await.unwrap();
    (
        page.columns.into_iter().map(|column| column.name).collect(),
        page.rows,
    )
}

/// The row of `table` with this id, as a page gives it, and the names of
/// its columns.
async fn row_of(connection: &Connection, table: &str, id: i64) -> (Vec<String>, Vec<Value>) {
    let (columns, rows) = page_of(connection, table).await;
    let row = rows
        .into_iter()
        .find(|row| row[0] == Value::Int(id))
        .unwrap_or_else(|| panic!("{table} has no row {id}"));
    (columns, row)
}

/// One cell's change: `column`, of the type `describe` names, becomes
/// `new`, from what `row` holds.
fn cell(
    columns: &[String],
    row: &[Value],
    column: &str,
    type_name: &str,
    new: NewValue,
) -> CellChange {
    let index = columns.iter().position(|name| name == column).unwrap();
    CellChange {
        column: column.into(),
        type_name: type_name.into(),
        loaded: row[index].clone(),
        new,
    }
}

/// The changes to the row of this id.
fn by_id(id: i64, set: Vec<CellChange>) -> RowChange {
    RowChange {
        key: vec![("id".into(), Value::Int(id))],
        set,
    }
}

fn changes_to(table: &str, rows: Vec<RowChange>) -> ChangeSet {
    ChangeSet {
        object: ObjectRef::new("public", table),
        rows,
    }
}

/// A save of one row of `table`: each (column, type, new value), with what
/// the page holds now as the loaded value.
async fn save(
    connection: &Connection,
    table: &str,
    id: i64,
    cells: &[(&str, &str, NewValue)],
) -> tabletist_db::Result<WriteOutcome> {
    let (columns, row) = row_of(connection, table, id).await;
    let set = cells
        .iter()
        .map(|(column, type_name, new)| cell(&columns, &row, column, type_name, new.clone()))
        .collect();
    within(connection.write(&changes_to(table, vec![by_id(id, set)]))).await
}

fn to(text: &str) -> NewValue {
    NewValue::Text(text.into())
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

/// The server process of `connection`'s session.
async fn backend_pid(connection: &Connection) -> i32 {
    let outcome = run(connection, "SELECT pg_backend_pid()", 1).await.unwrap();
    match &outcome.results[0].outcome {
        StatementOutcome::Rows { rows, .. } => match rows[0][0] {
            Value::Int(pid) => i32::try_from(pid).unwrap(),
            ref other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

/// What the server says the session of `pid` is doing: `idle` when no
/// transaction is open on it.
async fn state_of(admin: &tokio_postgres::Client, pid: i32) -> String {
    admin
        .query_one("SELECT state FROM pg_stat_activity WHERE pid = $1", &[&pid])
        .await
        .unwrap()
        .get(0)
}

#[tokio::test]
async fn a_save_writes_every_kind_of_value_and_reads_the_row_back() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "DROP TABLE IF EXISTS write_types; DROP TYPE IF EXISTS write_mood",
        "CREATE TYPE write_mood AS ENUM ('happy', 'sad');
         CREATE TABLE write_types (
             id integer PRIMARY KEY,
             email text NOT NULL UNIQUE,
             name text,
             created_at timestamptz NOT NULL DEFAULT '2026-01-01 00:00:00+00',
             active boolean NOT NULL DEFAULT true,
             meta jsonb,
             score double precision,
             balance numeric(14, 2),
             tags text[],
             mood write_mood,
             uid uuid
         );
         INSERT INTO write_types VALUES
             (1, 'a@x', 'Ada', '2026-01-01 00:00:00+00', true, '{\"a\": 1}', 0.1, 12.50,
              '{x,y}', 'happy', '0199a3f2-7c1e-7abc-8def-0123456789ab'),
             (2, 'b@x', 'Bea', '2026-01-02 00:00:00+00', false, NULL, NULL, NULL, NULL, NULL,
              NULL)",
        async move {
            let (columns, untouched) = row_of(&connection, "write_types", 2).await;
            let outcome = save(
                &connection,
                "write_types",
                1,
                &[
                    ("email", "text", to("new@example.com")),
                    ("name", "text", NewValue::Null),
                    (
                        "created_at",
                        "timestamp with time zone",
                        to("2027-02-03 04:05:06+00"),
                    ),
                    ("active", "boolean", to("false")),
                    ("meta", "jsonb", to(r#"{"plan": "pro"}"#)),
                    ("score", "double precision", to("12.5")),
                    ("balance", "numeric(14,2)", to("99.95")),
                    ("tags", "text[]", to("{a,b,c}")),
                    ("mood", "write_mood", to("sad")),
                    ("uid", "uuid", to("11111111-2222-3333-4444-555555555555")),
                ],
            )
            .await;
            let WriteOutcome::Written { rows, .. } = outcome.unwrap() else {
                panic!("not written");
            };
            // What came back is what a page now shows.
            let (_, after) = row_of(&connection, "write_types", 1).await;
            assert_eq!(rows, std::slice::from_ref(&after));
            let at = |name: &str| {
                after[columns.iter().position(|column| column == name).unwrap()].clone()
            };
            assert_eq!(at("id"), Value::Int(1));
            assert_eq!(at("email"), text("new@example.com"));
            assert_eq!(at("name"), Value::Null);
            assert_eq!(at("created_at"), text("2027-02-03 04:05:06+00"));
            assert_eq!(at("active"), Value::Bool(false));
            assert_eq!(at("meta"), text(r#"{"plan": "pro"}"#));
            assert_eq!(at("score"), Value::Float(12.5));
            assert_eq!(at("balance"), text("99.95"));
            assert_eq!(at("tags"), text("{a,b,c}"));
            assert_eq!(at("mood"), text("sad"));
            assert_eq!(at("uid"), text("11111111-2222-3333-4444-555555555555"));
            // The other row is as it was.
            assert_eq!(row_of(&connection, "write_types", 2).await.1, untouched);
            // And the row never conflicts with itself: every one of those
            // columns written again, from what the page holds now. That is
            // what proves each type's loaded value equals itself read again.
            let again = save(
                &connection,
                "write_types",
                1,
                &[
                    ("email", "text", to("back@example.com")),
                    ("name", "text", to("Ada")),
                    (
                        "created_at",
                        "timestamp with time zone",
                        to("2026-01-01 00:00:00+00"),
                    ),
                    ("active", "boolean", to("true")),
                    ("meta", "jsonb", NewValue::Null),
                    ("score", "double precision", to("0.1")),
                    ("balance", "numeric(14,2)", to("12.50")),
                    ("tags", "text[]", to("{}")),
                    ("mood", "write_mood", to("happy")),
                    ("uid", "uuid", NewValue::Null),
                ],
            )
            .await;
            assert!(
                matches!(again, Ok(WriteOutcome::Written { .. })),
                "{again:?}"
            );
        },
    )
    .await;
}

#[tokio::test]
async fn a_row_changed_by_someone_else_is_a_conflict_and_nothing_is_written() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("write_conflict"),
        &people("write_conflict"),
        async move {
            // Loaded, then changed behind the page's back.
            let (columns, first) = row_of(&connection, "write_conflict", 1).await;
            let (_, second) = row_of(&connection, "write_conflict", 2).await;
            let changes = changes_to(
                "write_conflict",
                vec![
                    by_id(
                        2,
                        vec![cell(&columns, &second, "name", "text", to("Second"))],
                    ),
                    by_id(1, vec![cell(&columns, &first, "name", "text", to("Mine"))]),
                ],
            );
            admin()
                .await
                .batch_execute("UPDATE write_conflict SET name = 'Theirs' WHERE id = 1")
                .await
                .unwrap();
            let outcome = within(connection.write(&changes)).await.unwrap();
            let theirs = row_of(&connection, "write_conflict", 1).await.1;
            assert_eq!(theirs[2], text("Theirs"));
            // The row's place in the set, and the row as the server holds it.
            assert_eq!(
                outcome,
                WriteOutcome::Conflicts(vec![Conflict {
                    row: 1,
                    server: Some(theirs),
                }])
            );
            // The other row of the set was not written either.
            assert_eq!(row_of(&connection, "write_conflict", 2).await.1, second);
        },
    )
    .await;
}

#[tokio::test]
async fn a_change_to_a_column_the_save_leaves_alone_is_no_conflict() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("write_elsewhere"),
        &people("write_elsewhere"),
        async move {
            let (columns, row) = row_of(&connection, "write_elsewhere", 1).await;
            let changes = changes_to(
                "write_elsewhere",
                vec![by_id(
                    1,
                    vec![cell(&columns, &row, "name", "text", to("Mine"))],
                )],
            );
            admin()
                .await
                .batch_execute("UPDATE write_elsewhere SET score = 99 WHERE id = 1")
                .await
                .unwrap();
            let outcome = within(connection.write(&changes)).await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Written { .. })),
                "{outcome:?}"
            );
            // Their change stands beside this one.
            assert_eq!(
                row_of(&connection, "write_elsewhere", 1).await.1[2..],
                [text("Mine"), Value::Float(99.0)]
            );
        },
    )
    .await;
}

#[tokio::test]
async fn a_row_that_is_gone_is_a_conflict_without_a_row() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("write_gone"),
        &people("write_gone"),
        async move {
            let (columns, row) = row_of(&connection, "write_gone", 3).await;
            let changes = changes_to(
                "write_gone",
                vec![by_id(
                    3,
                    vec![cell(&columns, &row, "name", "text", to("Late"))],
                )],
            );
            admin()
                .await
                .batch_execute("DELETE FROM write_gone WHERE id = 3")
                .await
                .unwrap();
            assert_eq!(
                within(connection.write(&changes)).await,
                Ok(WriteOutcome::Conflicts(vec![Conflict {
                    row: 0,
                    server: None
                }]))
            );
        },
    )
    .await;
}

#[tokio::test]
async fn a_statement_that_fails_undoes_the_rows_before_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("write_undone"),
        &people("write_undone"),
        async move {
            let (columns, first) = row_of(&connection, "write_undone", 1).await;
            let (_, second) = row_of(&connection, "write_undone", 2).await;
            let changes = changes_to(
                "write_undone",
                vec![
                    by_id(
                        1,
                        vec![cell(&columns, &first, "name", "text", to("Written first"))],
                    ),
                    // NOT NULL: the database refuses it.
                    by_id(
                        2,
                        vec![cell(&columns, &second, "email", "text", NewValue::Null)],
                    ),
                ],
            );
            let outcome = within(connection.write(&changes)).await.unwrap();
            assert!(
                matches!(
                    &outcome,
                    WriteOutcome::Failed { row: 1, error: Error::Query { code, .. } }
                        if code.as_deref() == Some("23502")
                ),
                "{outcome:?}"
            );
            assert_eq!(row_of(&connection, "write_undone", 1).await.1, first);
            assert_eq!(row_of(&connection, "write_undone", 2).await.1, second);
        },
    )
    .await;
}

#[tokio::test]
async fn a_value_the_column_cannot_take_is_the_databases_error() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("write_refused"),
        &people("write_refused"),
        async move {
            let before = row_of(&connection, "write_refused", 1).await.1;
            let outcome = save(
                &connection,
                "write_refused",
                1,
                &[("score", "double precision", to("high"))],
            )
            .await
            .unwrap();
            assert!(
                matches!(
                    &outcome,
                    WriteOutcome::Failed { row: 0, error: Error::Query { code, .. } }
                        if code.as_deref() == Some("22P02")
                ),
                "{outcome:?}"
            );
            assert_eq!(row_of(&connection, "write_refused", 1).await.1, before);
        },
    )
    .await;
}

#[tokio::test]
async fn a_save_on_a_read_only_connection_is_refused() {
    let Some(connection) = connect().await else {
        return;
    };
    on_its_own_tables(
        &drop_table("write_read_only"),
        &people("write_read_only"),
        async move {
            let before = row_of(&connection, "write_read_only", 1).await.1;
            let outcome = save(
                &connection,
                "write_read_only",
                1,
                &[("name", "text", to("Grace"))],
            )
            .await;
            assert_eq!(outcome, Err(Error::ReadOnly));
            assert_eq!(row_of(&connection, "write_read_only", 1).await.1, before);
        },
    )
    .await;
}

/// However a save ends, its transaction ends with it: the session is in
/// none, holds no row, and is what a script and a page expect.
#[tokio::test]
async fn a_save_leaves_no_transaction_open_however_it_ends() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "DROP TABLE IF EXISTS write_session, write_session_twins",
        &format!(
            "{};
             CREATE TABLE write_session_twins (id integer, name text);
             INSERT INTO write_session_twins VALUES (1, 'a'), (1, 'b')",
            people("write_session")
        ),
        async move {
            let admin = admin().await;
            admin
                .batch_execute("SET lock_timeout = '5s'")
                .await
                .unwrap();
            let pid = backend_pid(&connection).await;
            let (columns, row) = row_of(&connection, "write_session", 1).await;
            let mut stale = row.clone();
            stale[2] = text("What the page never held");
            type Ended = fn(&tabletist_db::Result<WriteOutcome>) -> bool;
            let exits: [(&str, ChangeSet, Ended); 8] = [
                (
                    "a conflict",
                    changes_to(
                        "write_session",
                        vec![by_id(
                            1,
                            vec![cell(&columns, &stale, "name", "text", to("Mine"))],
                        )],
                    ),
                    |ended| matches!(ended, Ok(WriteOutcome::Conflicts(_))),
                ),
                (
                    "a row that is gone",
                    changes_to(
                        "write_session",
                        vec![by_id(
                            99,
                            vec![cell(&columns, &row, "name", "text", to("Mine"))],
                        )],
                    ),
                    |ended| matches!(ended, Ok(WriteOutcome::Conflicts(_))),
                ),
                (
                    "a statement the database refuses",
                    changes_to(
                        "write_session",
                        vec![by_id(
                            1,
                            vec![cell(&columns, &row, "email", "text", NewValue::Null)],
                        )],
                    ),
                    |ended| matches!(ended, Ok(WriteOutcome::Failed { row: 0, .. })),
                ),
                (
                    "a statement that cannot be sent",
                    changes_to(
                        "write_session",
                        vec![by_id(
                            1,
                            vec![cell(&columns, &row, "name", "text", to("a\0b"))],
                        )],
                    ),
                    |ended| matches!(ended, Ok(WriteOutcome::Failed { row: 0, .. })),
                ),
                (
                    "a column the table does not have",
                    changes_to(
                        "write_session",
                        vec![by_id(
                            1,
                            vec![CellChange {
                                column: "nick".into(),
                                type_name: "text".into(),
                                loaded: Value::Null,
                                new: to("Mine"),
                            }],
                        )],
                    ),
                    |ended| matches!(ended, Err(Error::Query { .. })),
                ),
                (
                    "a table that is not there",
                    changes_to(
                        "write_session_nowhere",
                        vec![by_id(
                            1,
                            vec![cell(&columns, &row, "name", "text", to("Mine"))],
                        )],
                    ),
                    |ended| matches!(ended, Err(Error::Query { .. })),
                ),
                (
                    "a key that matches two rows",
                    changes_to(
                        "write_session_twins",
                        vec![by_id(
                            1,
                            vec![CellChange {
                                column: "name".into(),
                                type_name: "text".into(),
                                loaded: text("a"),
                                new: to("Mine"),
                            }],
                        )],
                    ),
                    |ended| matches!(ended, Err(Error::Query { .. })),
                ),
                (
                    "a save that is written",
                    changes_to(
                        "write_session",
                        vec![by_id(
                            1,
                            vec![cell(&columns, &row, "name", "text", to("Grace"))],
                        )],
                    ),
                    |ended| matches!(ended, Ok(WriteOutcome::Written { .. })),
                ),
            ];
            for (exit, changes, ended_so) in exits {
                let ended = within(connection.write(&changes)).await;
                assert!(ended_so(&ended), "{exit}: {ended:?}");
                assert_eq!(state_of(&admin, pid).await, "idle", "{exit}");
                // No row is still held: someone else changes them at once.
                let free = admin
                    .batch_execute(
                        "UPDATE write_session SET score = score;
                         UPDATE write_session_twins SET name = name",
                    )
                    .await;
                assert!(free.is_ok(), "{exit}: {free:?}");
            }
            // Only the last of them wrote.
            assert_eq!(
                row_of(&connection, "write_session", 1).await.1[1..3],
                [text("a@x"), text("Grace")]
            );
            // A script is fenced as before, and the session's own default
            // is the one it connected with.
            let outcome = run(
                &connection,
                "SHOW default_transaction_read_only; SHOW transaction_read_only; \
                 INSERT INTO write_session VALUES (9, 'z@x', 'Z', 1)",
                10,
            )
            .await
            .unwrap();
            let shown = |index: usize| match &outcome.results[index].outcome {
                StatementOutcome::Rows { rows, .. } => rows[0][0].clone(),
                other => panic!("{other:?}"),
            };
            assert_eq!(shown(0), text("off"));
            assert_eq!(shown(1), text("on"));
            assert!(
                matches!(
                    &outcome.results[2].outcome,
                    StatementOutcome::Error { error: Error::Query { code: Some(code), .. }, .. }
                        if code == "25006"
                ),
                "{outcome:?}"
            );
            assert_eq!(state_of(&admin, pid).await, "idle");
        },
    )
    .await;
}

#[tokio::test]
async fn a_composite_and_a_binary_key_find_their_row() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("write_composite"),
        "CREATE TABLE write_composite (a integer, b bytea, note text, PRIMARY KEY (a, b));
         INSERT INTO write_composite VALUES
             (1, '\\x00ff10', 'first'), (1, '\\x00ff11', 'second'), (2, '\\x00ff10', 'third')",
        async move {
            let (_, before) = page_of(&connection, "write_composite").await;
            let bytes = Value::Bytes(vec![0x00, 0xff, 0x10].into());
            assert_eq!(before[0][1], bytes);
            let changes = changes_to(
                "write_composite",
                vec![RowChange {
                    key: vec![("a".into(), Value::Int(1)), ("b".into(), bytes.clone())],
                    set: vec![CellChange {
                        column: "note".into(),
                        type_name: "text".into(),
                        loaded: text("first"),
                        new: to("changed"),
                    }],
                }],
            );
            let outcome = within(connection.write(&changes)).await.unwrap();
            let changed = vec![Value::Int(1), bytes, text("changed")];
            assert!(
                matches!(&outcome, WriteOutcome::Written { rows, .. } if *rows == [changed.clone()]),
                "{outcome:?}"
            );
            // Its neighbours, which share half the key each, are as they were.
            let (_, after) = page_of(&connection, "write_composite").await;
            assert_eq!(after, [changed, before[1].clone(), before[2].clone()]);
        },
    )
    .await;
}

/// A key is a row's only when one row has it. Where two do (nothing holds
/// the column unique), the save cannot say which it means.
#[tokio::test]
async fn a_key_that_matches_two_rows_is_an_error_and_nothing_is_written() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("write_twins"),
        "CREATE TABLE write_twins (id integer, name text);
         INSERT INTO write_twins VALUES (1, 'same'), (1, 'same'), (2, 'other')",
        async move {
            let changes = changes_to(
                "write_twins",
                vec![by_id(
                    1,
                    vec![CellChange {
                        column: "name".into(),
                        type_name: "text".into(),
                        loaded: text("same"),
                        new: to("Mine"),
                    }],
                )],
            );
            let outcome = within(connection.write(&changes)).await;
            assert!(
                matches!(&outcome, Err(Error::Query { message, .. }) if message.contains("more than one row")),
                "{outcome:?}"
            );
            let admin = admin().await;
            let names: Vec<String> = admin
                .query("SELECT name FROM write_twins ORDER BY id", &[])
                .await
                .unwrap()
                .iter()
                .map(|row| row.get(0))
                .collect();
            assert_eq!(names, ["same", "same", "other"]);
        },
    )
    .await;
}

/// An `UPDATE` that changes no row (here a trigger drops it) is not a save:
/// the row before it is put back.
#[tokio::test]
async fn an_update_that_does_not_change_one_row_fails_and_is_undone() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "DROP TABLE IF EXISTS write_skipped; DROP FUNCTION IF EXISTS write_skipped_skip()",
        &format!(
            "{};
             CREATE FUNCTION write_skipped_skip() RETURNS trigger LANGUAGE plpgsql AS $$
             BEGIN
                 IF NEW.name = 'Skip' THEN RETURN NULL; END IF;
                 RETURN NEW;
             END $$;
             CREATE TRIGGER skip BEFORE UPDATE ON write_skipped
                 FOR EACH ROW EXECUTE FUNCTION write_skipped_skip()",
            people("write_skipped")
        ),
        async move {
            let (columns, first) = row_of(&connection, "write_skipped", 1).await;
            let (_, second) = row_of(&connection, "write_skipped", 2).await;
            let changes = changes_to(
                "write_skipped",
                vec![
                    by_id(
                        1,
                        vec![cell(&columns, &first, "name", "text", to("Written first"))],
                    ),
                    by_id(2, vec![cell(&columns, &second, "name", "text", to("Skip"))]),
                ],
            );
            let outcome = within(connection.write(&changes)).await.unwrap();
            assert!(
                matches!(
                    &outcome,
                    WriteOutcome::Failed { row: 1, error: Error::Query { message, .. } }
                        if message.contains("0 rows")
                ),
                "{outcome:?}"
            );
            assert_eq!(row_of(&connection, "write_skipped", 1).await.1, first);
            assert_eq!(row_of(&connection, "write_skipped", 2).await.1, second);
        },
    )
    .await;
}

/// The text a save stores, and the key it finds its row by, is the text it
/// was given, character for character.
const AWKWARD: [&str; 12] = [
    "O'Brien",
    r"C:\temp\new",
    r"a backslash before a quote \' and after '\",
    r"two of each '' \\",
    "naïve café, 漢字, 🙂",
    r#"both "double" and 'single' and \ and é"#,
    "'",
    r"\",
    "$1 $$ ; -- /* not a comment */",
    "a line\nand a\ttab",
    "E'x'",
    r"\x00ff",
];

/// A script cannot change how the server reads a save's text: what it set
/// goes with its transaction.
#[tokio::test]
async fn text_is_stored_exactly_also_after_a_script_set_how_strings_are_read() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("write_text"),
        "CREATE TABLE write_text (id text PRIMARY KEY, body text)",
        async move {
            let admin = admin().await;
            for text in AWKWARD {
                admin
                    .execute("INSERT INTO write_text VALUES ($1, 'before')", &[&text])
                    .await
                    .unwrap();
            }
            // The refusal stops a plain SET of the first two, so they are
            // set where it does not read: inside a block.
            let outcome = run(
                &connection,
                "DO $$ BEGIN
                     SET standard_conforming_strings = off;
                     SET client_encoding = 'LATIN1';
                 END $$;
                 SET backslash_quote = on;
                 SET search_path = pg_catalog;
                 SET DateStyle = 'German, DMY';
                 SHOW standard_conforming_strings",
                10,
            )
            .await
            .unwrap();
            assert!(
                matches!(
                    &outcome.results[4].outcome,
                    StatementOutcome::Rows { rows, .. } if rows[0][0] == text("off")
                ),
                "{outcome:?}"
            );
            let rows = AWKWARD
                .iter()
                .map(|awkward| RowChange {
                    key: vec![("id".into(), text(awkward))],
                    set: vec![CellChange {
                        column: "body".into(),
                        type_name: "text".into(),
                        loaded: text("before"),
                        new: to(awkward),
                    }],
                })
                .collect();
            let outcome = within(connection.write(&changes_to("write_text", rows))).await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Written { .. })),
                "{outcome:?}"
            );
            // Read by another session, through the protocol that carries
            // values as they are.
            let stored = admin
                .query("SELECT id, body FROM write_text", &[])
                .await
                .unwrap();
            let mut ids: Vec<String> = Vec::new();
            for row in &stored {
                let (id, body): (String, String) = (row.get(0), row.get(1));
                assert_eq!(body, id);
                ids.push(id);
            }
            let mut awkward = AWKWARD.map(str::to_owned).to_vec();
            awkward.sort();
            ids.sort();
            assert_eq!(ids, awkward);
        },
    )
    .await;
}

/// PostgreSQL text cannot hold a NUL, and the driver cannot send one: it
/// fails in a way that reads as a lost session. So such a save is the
/// row's failure before anything is sent, and the session lives.
#[tokio::test]
async fn text_holding_a_nul_is_refused_before_anything_is_sent() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(&drop_table("write_nul"), &people("write_nul"), async move {
        let (columns, first) = row_of(&connection, "write_nul", 1).await;
        let (_, second) = row_of(&connection, "write_nul", 2).await;
        let changes = changes_to(
            "write_nul",
            vec![
                by_id(1, vec![cell(&columns, &first, "name", "text", to("Fine"))]),
                by_id(2, vec![cell(&columns, &second, "name", "text", to("a\0b"))]),
            ],
        );
        let outcome = within(connection.write(&changes)).await;
        assert!(
            matches!(
                &outcome,
                Ok(WriteOutcome::Failed { row: 1, error: Error::Query { message, .. } })
                    if message.contains("NUL")
            ),
            "{outcome:?}"
        );
        assert_eq!(row_of(&connection, "write_nul", 1).await.1, first);
    })
    .await;
}

/// The row is read locked. While someone else's change to it is not yet
/// committed the save waits, and then compares what they committed: read
/// without the lock, it would see the row as it was, find no conflict, and
/// write over their change.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_save_waits_for_a_change_in_flight_and_then_sees_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("write_locked"),
        &people("write_locked"),
        async move {
            let (columns, row) = row_of(&connection, "write_locked", 1).await;
            let changes = changes_to(
                "write_locked",
                vec![by_id(
                    1,
                    vec![cell(&columns, &row, "name", "text", to("Mine"))],
                )],
            );
            let theirs = admin().await;
            theirs
                .batch_execute("BEGIN; UPDATE write_locked SET name = 'Theirs' WHERE id = 1")
                .await
                .unwrap();
            let connection = std::sync::Arc::new(connection);
            let saving = {
                let connection = std::sync::Arc::clone(&connection);
                tokio::spawn(async move { connection.write(&changes).await })
            };
            // Until the save waits for their lock.
            let admin = admin().await;
            within(async {
                loop {
                    let waiting: i64 = admin
                        .query_one(
                            "SELECT count(*) FROM pg_stat_activity \
                             WHERE wait_event_type = 'Lock' AND pid <> pg_backend_pid() \
                               AND position('write_locked' in query) > 0",
                            &[],
                        )
                        .await
                        .unwrap()
                        .get(0);
                    if waiting > 0 {
                        break;
                    }
                    assert!(!saving.is_finished(), "the save did not wait");
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await;
            theirs.batch_execute("COMMIT").await.unwrap();
            let outcome = within(saving).await.unwrap().unwrap();
            let now = row_of(&connection, "write_locked", 1).await.1;
            assert_eq!(now[2], text("Theirs"));
            assert_eq!(
                outcome,
                WriteOutcome::Conflicts(vec![Conflict {
                    row: 0,
                    server: Some(now),
                }])
            );
        },
    )
    .await;
}

/// A `COMMIT` the database refuses (a constraint checked only then) wrote
/// nothing: it is the save's error, on a session that lives.
#[tokio::test]
async fn a_commit_the_database_refuses_is_the_saves_error_and_nothing_is_written() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("write_deferred"),
        "CREATE TABLE write_deferred (
             id integer PRIMARY KEY,
             email text NOT NULL,
             CONSTRAINT write_deferred_email UNIQUE (email) DEFERRABLE INITIALLY DEFERRED
         );
         INSERT INTO write_deferred VALUES (1, 'a@x'), (2, 'b@x')",
        async move {
            let admin = admin().await;
            let pid = backend_pid(&connection).await;
            let before = page_of(&connection, "write_deferred").await.1;
            let outcome = save(&connection, "write_deferred", 1, &[("email", "text", to("b@x"))]).await;
            assert!(
                matches!(&outcome, Err(Error::Query { code, .. }) if code.as_deref() == Some("23505")),
                "{outcome:?}"
            );
            assert_eq!(page_of(&connection, "write_deferred").await.1, before);
            assert_eq!(state_of(&admin, pid).await, "idle");
            // And the session saves as before.
            let outcome = save(&connection, "write_deferred", 1, &[("email", "text", to("c@x"))]).await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Written { .. })),
                "{outcome:?}"
            );
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancel_during_a_save_undoes_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "DROP TABLE IF EXISTS write_cancel; DROP FUNCTION IF EXISTS write_cancel_slow()",
        // For one name only, the UPDATE runs far longer than the test waits.
        &format!(
            "{};
             CREATE FUNCTION write_cancel_slow() RETURNS trigger LANGUAGE plpgsql AS $$
             BEGIN
                 IF NEW.name = 'Slow' THEN PERFORM pg_sleep(20); END IF;
                 RETURN NEW;
             END $$;
             CREATE TRIGGER slow BEFORE UPDATE ON write_cancel
                 FOR EACH ROW EXECUTE FUNCTION write_cancel_slow()",
            people("write_cancel")
        ),
        async move {
            let admin = admin().await;
            let pid = backend_pid(&connection).await;
            let (columns, first) = row_of(&connection, "write_cancel", 1).await;
            let (_, second) = row_of(&connection, "write_cancel", 2).await;
            let changes = changes_to(
                "write_cancel",
                vec![
                    by_id(
                        1,
                        vec![cell(&columns, &first, "name", "text", to("Written first"))],
                    ),
                    by_id(2, vec![cell(&columns, &second, "name", "text", to("Slow"))]),
                ],
            );
            let cancel = connection.cancel_handle();
            let connection = std::sync::Arc::new(connection);
            let saving = {
                let connection = std::sync::Arc::clone(&connection);
                tokio::spawn(async move { connection.write(&changes).await })
            };
            runs_on_the_server(&admin, "SET \"name\" = 'Slow'").await;
            let deadline = std::time::Instant::now() + Duration::from_secs(15);
            while !saving.is_finished() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "cancel must stop the save"
                );
                cancel.cancel().await.unwrap();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            // The save's own end, not a row's failure.
            assert_eq!(saving.await.unwrap(), Err(Error::Cancelled));
            // Nothing was written, the row before the cancelled one included.
            assert_eq!(row_of(&connection, "write_cancel", 1).await.1, first);
            assert_eq!(row_of(&connection, "write_cancel", 2).await.1, second);
            assert_eq!(state_of(&admin, pid).await, "idle");
            // And the session saves as before.
            let outcome = save(
                &connection,
                "write_cancel",
                1,
                &[("name", "text", to("Quick"))],
            )
            .await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Written { .. })),
                "{outcome:?}"
            );
        },
    )
    .await;
}

/// A trigger the update fires can give a second row the key, or take the
/// key from the row. The save then has no one row to hand back: it is an
/// error, and what it wrote (and what the trigger wrote) is undone.
#[tokio::test]
async fn a_row_that_cannot_be_read_back_alone_is_an_error_and_is_undone() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "DROP TABLE IF EXISTS write_twinned, write_moved;
         DROP FUNCTION IF EXISTS write_twinned_twin(), write_moved_move()",
        "CREATE TABLE write_twinned (id integer, name text);
         INSERT INTO write_twinned VALUES (1, 'one'), (2, 'two');
         CREATE FUNCTION write_twinned_twin() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
             INSERT INTO write_twinned VALUES (NEW.id, 'twin');
             RETURN NEW;
         END $$;
         CREATE TRIGGER twin AFTER UPDATE ON write_twinned
             FOR EACH ROW EXECUTE FUNCTION write_twinned_twin();
         CREATE TABLE write_moved (id integer, name text);
         INSERT INTO write_moved VALUES (1, 'one'), (2, 'two');
         CREATE FUNCTION write_moved_move() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
             NEW.id := NEW.id + 100;
             RETURN NEW;
         END $$;
         CREATE TRIGGER move BEFORE UPDATE ON write_moved
             FOR EACH ROW EXECUTE FUNCTION write_moved_move()",
        async move {
            let admin = admin().await;
            for (table, said) in [
                ("write_twinned", "more than one row"),
                ("write_moved", "could not be read back"),
            ] {
                let changes = changes_to(
                    table,
                    vec![by_id(
                        1,
                        vec![CellChange {
                            column: "name".into(),
                            type_name: "text".into(),
                            loaded: text("one"),
                            new: to("Mine"),
                        }],
                    )],
                );
                let outcome = within(connection.write(&changes)).await;
                assert!(
                    matches!(&outcome, Err(Error::Query { message, .. }) if message.contains(said)),
                    "{table}: {outcome:?}"
                );
                let rows: Vec<(i32, String)> = admin
                    .query(
                        &format!("SELECT id, name FROM {table} ORDER BY id, name"),
                        &[],
                    )
                    .await
                    .unwrap()
                    .iter()
                    .map(|row| (row.get(0), row.get(1)))
                    .collect();
                assert_eq!(
                    rows,
                    [(1, "one".to_owned()), (2, "two".to_owned())],
                    "{table}"
                );
            }
        },
    )
    .await;
}
