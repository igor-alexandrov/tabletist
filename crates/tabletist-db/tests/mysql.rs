//! MySQL through the public `Connection` API. Needs a server:
//! `docker compose up -d --wait mysql` and TABLETIST_TEST_MYSQL_URL (see
//! compose.yaml). Without it every test prints "skipped" and passes.

#![allow(clippy::unwrap_used)]

use std::time::Duration;

use mysql_async::prelude::Queryable;
use tabletist_db::{
    Access, ConnectSpec, Connection, Driver, Error, HostKeys, ObjectKind, Secrets, TlsMode,
};
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

/// A connection to the fixture with the given access, or `None` (test skipped).
async fn connect_as(access: Access) -> Option<Connection> {
    let Some((spec, secrets)) = spec() else {
        eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
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

#[tokio::test]
async fn generated_columns_say_so() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut admin = admin().await;
    for statement in [
        "DROP TABLE IF EXISTS catalog_generated",
        "CREATE TABLE catalog_generated (
             id INT PRIMARY KEY,
             title VARCHAR(50) NOT NULL,
             stamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
             slug VARCHAR(50) GENERATED ALWAYS AS (LOWER(title)) VIRTUAL,
             shout VARCHAR(50) GENERATED ALWAYS AS (UPPER(title)) STORED
         )",
    ] {
        admin.query_drop(statement).await.unwrap();
    }
    let structure = connection
        .describe(&ObjectRef::new("tabletist", "catalog_generated"))
        .await;
    admin
        .query_drop("DROP TABLE catalog_generated")
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
            ("id".to_owned(), false),
            ("title".to_owned(), false),
            // MySQL 8 calls a default that is an expression
            // `DEFAULT_GENERATED`: the column still takes a value.
            ("stamp".to_owned(), false),
            ("slug".to_owned(), true),
            ("shout".to_owned(), true),
        ]
    );
}

#[tokio::test]
async fn an_auto_increment_column_is_an_identity_column() {
    let Some(connection) = connect_as(Access::ReadOnly).await else {
        return;
    };
    on_its_own_tables(
        "identity_cols",
        &["CREATE TABLE identity_cols (
               id INT AUTO_INCREMENT PRIMARY KEY,
               title VARCHAR(20) NOT NULL DEFAULT 'none'
           )"],
        async move {
            let structure = connection
                .describe(&ObjectRef::new("tabletist", "identity_cols"))
                .await
                .unwrap();
            let identity: Vec<(&str, bool)> = structure
                .columns
                .iter()
                .map(|column| (column.name.as_str(), column.identity))
                .collect();
            assert_eq!(identity, [("id", true), ("title", false)]);
        },
    )
    .await;
}

/// The catalog names a column for an index over its first characters, and
/// nothing for an expression. Neither tells rows apart by whole columns.
#[tokio::test]
async fn only_an_index_over_whole_columns_is_the_row_key() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut admin = admin().await;
    for statement in [
        "DROP TABLE IF EXISTS catalog_keys",
        "CREATE TABLE catalog_keys (
             name VARCHAR(20) COLLATE utf8mb4_0900_ai_ci NOT NULL,
             code INT NOT NULL,
             title VARCHAR(20) NOT NULL,
             `<expression>` INT NOT NULL,
             UNIQUE KEY k0_expression ((code + 1)),
             UNIQUE KEY k1_prefix (name(1)),
             UNIQUE KEY k2_plain (code, title DESC)
         )",
        "INSERT INTO catalog_keys VALUES ('ssa', 1, 't', 1), ('ßa', 2, 'u', 2)",
    ] {
        admin.query_drop(statement).await.unwrap();
    }
    let structure = connection
        .describe(&ObjectRef::new("tabletist", "catalog_keys"))
        .await;
    // Without the index, which finds only the row whose prefix agrees.
    let twins: mysql_async::Result<Option<i64>> = admin
        .query_first(
            "SELECT COUNT(*) FROM catalog_keys IGNORE INDEX (k1_prefix) WHERE name = 'ssa'",
        )
        .await;
    admin.query_drop("DROP TABLE catalog_keys").await.unwrap();
    // Why the prefix is no key: its first characters differ, and the whole
    // names are equal as the column compares them.
    assert_eq!(twins.unwrap(), Some(2));
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
            ("k0_expression", list(&["<expression>"]), None),
            ("k1_prefix", list(&["name"]), None),
            (
                "k2_plain",
                list(&["code", "title"]),
                Some(list(&["code", "title"]))
            ),
        ]
    );
    assert_eq!(structure.row_key(), Some(list(&["code", "title"])));
}

/// MySQL takes a primary key over a prefix too, and its catalog still names
/// the column as the key.
#[tokio::test]
async fn a_primary_key_over_a_prefix_is_not_the_row_key() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut admin = admin().await;
    for statement in [
        "DROP TABLE IF EXISTS catalog_prefix_key",
        "CREATE TABLE catalog_prefix_key (
             name VARCHAR(20) NOT NULL,
             code INT NOT NULL,
             PRIMARY KEY (name(1)),
             UNIQUE KEY by_code (code)
         )",
    ] {
        admin.query_drop(statement).await.unwrap();
    }
    let structure = connection
        .describe(&ObjectRef::new("tabletist", "catalog_prefix_key"))
        .await;
    admin
        .query_drop("DROP TABLE catalog_prefix_key")
        .await
        .unwrap();
    let structure = structure.unwrap();
    assert_eq!(structure.primary_key, vec!["name".to_owned()]);
    assert_eq!(structure.row_key(), Some(vec!["code".to_owned()]));
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

/// The first column of each row of `object` the filter keeps, in key
/// order, after checking the count agrees.
async fn kept(
    connection: &Connection,
    object: &str,
    column: &str,
    op: FilterOp,
    value: &str,
) -> Vec<i64> {
    let mut query = RowQuery::new(ObjectRef::new("tabletist", object), 50);
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

/// MySQL has no UUID type: a UUID key is sixteen bytes in a `binary(16)`
/// column, which the app shows as the UUID. Typed, pasted or followed from
/// a foreign key, that text has to find the bytes.
#[tokio::test]
async fn a_uuid_or_hex_filter_matches_a_binary_key_and_its_foreign_key() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut admin = admin().await;
    for statement in [
        "DROP TABLE IF EXISTS uuid_memberships, uuid_accounts",
        "CREATE TABLE uuid_accounts (
             n INT NOT NULL, id BINARY(16) PRIMARY KEY, slug CHAR(36), tag VARBINARY(36)
         )",
        "INSERT INTO uuid_accounts VALUES
             (1, x'0199a3f27c1e7abc8def0123456789ab',
              '0199a3f2-7c1e-7abc-8def-0123456789ab', x'cafe'),
             (2, x'0199a3f27c1e7abc8def0123456789ac',
              '0199a3f27c1e7abc8def0123456789ac', '0199a3f2-7c1e-7abc-8def-0123456789ac'),
             (3, x'0199a3f27c1e7abc8def0123456789ad', '0xcafe', NULL)",
        "CREATE TABLE uuid_memberships (
             id INT PRIMARY KEY, account_id BINARY(16) NOT NULL,
             FOREIGN KEY (account_id) REFERENCES uuid_accounts (id)
         )",
        "INSERT INTO uuid_memberships VALUES
             (1, x'0199a3f27c1e7abc8def0123456789ab'),
             (2, x'0199a3f27c1e7abc8def0123456789ac'),
             (3, x'0199a3f27c1e7abc8def0123456789ab')",
    ] {
        admin.query_drop(statement).await.unwrap();
    }
    let ab = "0199a3f2-7c1e-7abc-8def-0123456789ab";
    for (column, op, value, expected) in [
        // The UUID the grid shows, with or without hyphens, and `0x` hex.
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
        // Text columns keep matching as text.
        ("slug", FilterOp::Eq, ab, vec![1]),
        (
            "slug",
            FilterOp::Eq,
            "0199a3f27c1e7abc8def0123456789ac",
            vec![2],
        ),
        ("slug", FilterOp::Eq, "0xcafe", vec![3]),
        // Binary of another length, and a UUID kept as text in a binary
        // column.
        ("tag", FilterOp::Eq, "0xcafe", vec![1]),
        (
            "tag",
            FilterOp::Eq,
            "0199a3f2-7c1e-7abc-8def-0123456789ac",
            vec![2],
        ),
    ] {
        assert_eq!(
            kept(&connection, "uuid_accounts", column, op, value).await,
            expected,
            "{column} {op:?} {value}"
        );
    }

    // Following the foreign key: the value is what the app shows for it.
    let memberships = ObjectRef::new("tabletist", "uuid_memberships");
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
        .query_drop("DROP TABLE uuid_memberships, uuid_accounts")
        .await
        .unwrap();
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
    for access in [Access::ReadOnly, Access::Writable] {
        let Some(connection) = connect_as(access).await else {
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

/// The driver closes the connection when a statement runs without a value
/// for each parameter, and it reads `:name` as one too, also in a quote
/// right after a `-`.
#[tokio::test]
async fn a_raw_where_with_a_parameter_is_a_query_error_and_the_session_survives() {
    let Some(connection) = connect().await else {
        return;
    };
    for raw in [
        "id = ?",
        "name = :x",
        "id = 1 -':abc'",
        "id = ? AND name = :x",
    ] {
        // Alone, and after a filter with a value of its own.
        for mut query in [users(50), filtered("name", FilterOp::Eq, "Ada Lovelace")] {
            query.raw_where = Some(raw.into());
            match connection.fetch_rows(&query).await {
                Err(Error::Query { code: None, .. }) => {}
                other => panic!("{raw}: {other:?}"),
            }
            assert!(connection.fetch_rows(&users(1)).await.is_ok(), "{raw}");
            match connection.count_rows(&query).await {
                Err(Error::Query { code: None, .. }) => {}
                other => panic!("{raw}: {other:?}"),
            }
            assert!(connection.fetch_rows(&users(1)).await.is_ok(), "{raw}");
        }
    }
}

#[tokio::test]
async fn a_raw_where_can_spell_a_parameter_in_a_string_or_a_comment() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut query = users(50);
    query.raw_where = Some("name <> ':x' AND name <> '?' /* :y ? */ -- :z ?".into());
    assert_eq!(connection.fetch_rows(&query).await.unwrap().rows.len(), 5);
    assert_eq!(connection.count_rows(&query).await.unwrap(), 5);
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

use tabletist_db::{Dialect, ScriptEnd, ScriptMode, ScriptOutcome, StatementOutcome, StopFlag};

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
    within(connection.run_script(&script(text), limit, ScriptMode::ReadOnly, &StopFlag::new()))
        .await
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
/// without a variable an earlier script set. (`mysql/script.rs` compares a
/// session with itself at connect, setting by setting.) On a writable
/// connection the read-only it sees is the script's fence, not a
/// connect-time setting.
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
        StatementOutcome::Done {
            affected: None,
            warnings: 0,
        }
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
        StatementOutcome::Done {
            affected: None,
            warnings: 0,
        }
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
    let outcome = run(
        &connection,
        "SELECT 1; GET DIAGNOSTICS @n = NUMBER; SELECT 3",
        10,
    )
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
    let outcome =
        within(connection.run_script(&statements, 10, ScriptMode::ReadOnly, &StopFlag::new()))
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
    let parameter = |spelled: &str| {
        format!("the statement has a parameter ({spelled}), which the SQL editor cannot fill in")
    };
    let misread = |spelled: &str| {
        format!(
            "the MySQL driver reads {spelled} here as a parameter; put a space after the - or / \
             that comes before the quote or comment"
        )
    };
    // The script, its second statement's error, and where that points.
    for (text, message, position) in [
        ("SELECT 1; SELECT ?; SELECT 3", parameter("?"), None),
        ("SELECT 1; SELECT :name", parameter(":name"), Some(8)),
        ("SELECT 1; SELECT ?, :name", parameter("? and :name"), None),
        ("SELECT 1; SELECT HEX(-':abc')", misread(":abc"), Some(14)),
        (
            "SELECT 1; SELECT 1 -`:abc` FROM (SELECT 2 AS `:abc`) t",
            misread(":abc"),
            Some(12),
        ),
    ] {
        let outcome = run(&connection, text, 10).await.unwrap();
        assert_eq!(outcome.results.len(), 2, "{text}");
        assert_eq!(
            outcome.results[1].outcome,
            StatementOutcome::Error {
                error: Error::query(message),
                position,
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
    for access in [Access::ReadOnly, Access::Writable] {
        let Some(connection) = connect_as(access).await else {
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
        // (`mysql/script.rs`).
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
            "USE billing; INSERT INTO probe VALUES (1)",
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
            let ran = within(connection.run_script(
                &statements,
                10,
                ScriptMode::ReadOnly,
                &StopFlag::new(),
            ))
            .await;
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
}

/// Whether the session is read-write between scripts. Asked through a
/// count, which runs outside any script: its WHERE reads the session's
/// own setting.
async fn writes_between_scripts(connection: &Connection) -> bool {
    let mut query = users(1);
    query.raw_where = Some("@@session.transaction_read_only = 0".into());
    connection.count_rows(&query).await.unwrap() == 5
}

#[tokio::test]
async fn a_writable_session_is_read_only_for_a_script_and_read_write_after_it() {
    let Some(writable) = connect_as(Access::Writable).await else {
        return;
    };
    assert!(writes_between_scripts(&writable).await);
    let rows = rows_of(&writable, "SELECT @@session.transaction_read_only").await;
    assert_eq!(rows[0], [Value::Int(1)]);
    assert!(writes_between_scripts(&writable).await);
    // A read-only session never is.
    let Some(read_only) = connect().await else {
        return;
    };
    assert!(!writes_between_scripts(&read_only).await);
    rows_of(&read_only, "SELECT 1").await;
    assert!(!writes_between_scripts(&read_only).await);
}

#[tokio::test]
async fn ddl_is_refused_as_read_only_and_ends_the_script() {
    for access in [Access::ReadOnly, Access::Writable] {
        let Some(connection) = connect_as(access).await else {
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
        assert_eq!(
            writes_between_scripts(&connection).await,
            access == Access::Writable,
            "{access:?}"
        );
    }
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
            connection
                .run_script(&script(text), 10, ScriptMode::ReadOnly, &stop)
                .await
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
async fn a_cancel_reaches_a_session_an_earlier_run_reset() {
    let Some(connection) = connect().await else {
        return;
    };
    let connection = std::sync::Arc::new(connection);
    let mut admin = admin().await;
    // The handle is the session's own, made before any run: the reset
    // after a run keeps the connection id the cancel names.
    let cancel = connection.cancel_handle();
    let id = rows_of(&connection, "SELECT CONNECTION_ID()").await;
    for sleep in ["SLEEP(33)", "SLEEP(34)"] {
        assert_eq!(rows_of(&connection, "SELECT CONNECTION_ID()").await, id);
        let stop = StopFlag::new();
        let running = {
            let connection = std::sync::Arc::clone(&connection);
            let stop = stop.clone();
            tokio::spawn(async move {
                let text = format!("SELECT 1; SELECT count(*) FROM users WHERE {sleep} = 0");
                connection
                    .run_script(&script(&text), 10, ScriptMode::ReadOnly, &stop)
                    .await
            })
        };
        runs_on_the_server(&mut admin, sleep).await;
        stop.stop();
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
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
        assert_eq!(outcome.results.len(), 2);
        assert_eq!(outcome.results[1].outcome, StatementOutcome::Cancelled);
    }
    assert_connect_time_settings(&connection).await;
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
            connection
                .run_script(&script(text), 10, ScriptMode::ReadOnly, &stop)
                .await
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
            connection
                .run_script(&script(text), 10, ScriptMode::ReadOnly, &stop)
                .await
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
    let outcome = within(connection.run_script(
        &script("SELECT 1; SELECT 2"),
        10,
        ScriptMode::ReadOnly,
        &stop,
    ))
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

#[tokio::test]
async fn the_words_format_uppercases_cannot_be_table_aliases() {
    use tabletist_db::Dialect;
    use tabletist_db::sql::{self, format::UPPERCASED};
    // Loads the fixture, or says the test is skipped.
    let Some(_connection) = connect().await else {
        return;
    };
    let mut conn = admin().await;
    // A keyword that can be a name is an alias here: Format leaves its
    // case alone.
    conn.query_drop("SELECT 1 FROM users first").await.unwrap();
    // The ones Format uppercases are not: the server reads each as SQL,
    // not as a name, and the statement ends too soon.
    let listed = UPPERCASED
        .iter()
        .filter(|word| sql::is_keyword(Dialect::MySql, word));
    for word in listed {
        let result = conn.query_drop(format!("SELECT 1 FROM users {word}")).await;
        assert!(
            matches!(&result, Err(mysql_async::Error::Server(error)) if error.code == 1064),
            "{word}: {result:?}"
        );
    }
    // After a dot a reserved word is a name, and this server compares
    // table names by case: Format keeps the case of a word beside a dot,
    // so the query still finds its table.
    conn.query_drop("CREATE TEMPORARY TABLE `order` (id INT)")
        .await
        .unwrap();
    let script = "select id from tabletist.order";
    let formatted = sql::format::format(Dialect::MySql, script, None, 0).unwrap();
    assert_eq!(formatted.text, "SELECT id\n  FROM tabletist.order");
    conn.query_drop(&formatted.text).await.unwrap();
    // A user variable's name touches its `@`. Format leaves it there and as
    // it is, though it spells a clause: the server reads the two as one.
    conn.query_drop("SET @from = 1").await.unwrap();
    let formatted = sql::format::format(Dialect::MySql, "select @from", None, 0).unwrap();
    assert_eq!(formatted.text, "SELECT @from");
    let value: Option<i64> = conn.query_first(&formatted.text).await.unwrap();
    assert_eq!(value, Some(1));
    conn.disconnect().await.unwrap();
}

/// Runs `text` as a script that writes, with a fresh stop flag.
async fn write(
    connection: &Connection,
    text: &str,
    limit: u32,
) -> tabletist_db::Result<ScriptOutcome> {
    within(connection.run_script(&script(text), limit, ScriptMode::Write, &StopFlag::new())).await
}

/// An empty table of this name with a key `id` and a column `n`, made
/// from outside the session under test. Each test has a table of its own:
/// they run at once.
async fn scratch(admin: &mut mysql_async::Conn, table: &str) {
    admin
        .query_drop(format!("DROP TABLE IF EXISTS {table}"))
        .await
        .unwrap();
    admin
        .query_drop(format!(
            "CREATE TABLE {table} (id int PRIMARY KEY, n int NOT NULL DEFAULT 0)"
        ))
        .await
        .unwrap();
}

/// A count, asked from outside the session under test.
async fn counted(admin: &mut mysql_async::Conn, sql: &str) -> i64 {
    admin.query_first(sql).await.unwrap().unwrap()
}

/// What each statement of a run did, as its row count, or `None` for one
/// that has none or did not end well.
fn affected(outcome: &ScriptOutcome) -> Vec<Option<u64>> {
    outcome
        .results
        .iter()
        .map(|result| match result.outcome {
            StatementOutcome::Done { affected, .. } => affected,
            _ => None,
        })
        .collect()
}

/// Whether the session refuses a write in a read-only run, as it must
/// after every run that writes.
async fn is_fenced(connection: &Connection, table: &str) -> bool {
    let outcome = run(connection, &format!("DELETE FROM {table}"), 1)
        .await
        .unwrap();
    matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Error {
            error: Error::Query { code: Some(code), .. },
            ..
        } if code == "25006"
    )
}

#[tokio::test]
async fn a_run_that_writes_is_refused_on_a_read_only_connection() {
    let Some(connection) = connect_as(Access::ReadOnly).await else {
        return;
    };
    for text in ["DELETE FROM users", "SELECT 1", "COMMIT"] {
        assert_eq!(
            write(&connection, text, 10).await,
            Err(Error::ReadOnly),
            "{text}"
        );
    }
}

/// A procedure can commit the run's transaction and open one of its own,
/// which the run could not tell from its own.
#[tokio::test]
async fn a_procedure_is_refused_in_a_run_that_writes_too() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    assert_eq!(
        write(&connection, "SELECT 1;\nCALL refill()", 10).await,
        Err(Error::Refused {
            line: 2,
            what: "CALL".into(),
            mode: ScriptMode::Write,
        })
    );
}

#[tokio::test]
async fn a_run_that_writes_is_committed_and_counts_its_rows() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    scratch(&mut admin, "script_counts").await;
    let outcome = write(
        &connection,
        "INSERT INTO script_counts (id) VALUES (1), (2), (3), (4), (5);
         UPDATE script_counts SET n = 10 WHERE id > 2;
         DELETE FROM script_counts WHERE id = 1",
        10,
    )
    .await
    .unwrap();
    assert_eq!(outcome.end, ScriptEnd::Committed);
    assert!(!outcome.was_cancelled());
    assert_eq!(outcome.broken, None);
    assert_eq!(outcome.rollback_warning, None);
    assert_eq!(affected(&outcome), [Some(5), Some(3), Some(1)]);
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_counts").await,
        4
    );
    // Read-write between scripts, as it connected, and fenced in a run
    // that only reads.
    assert!(writes_between_scripts(&connection).await);
    assert!(is_fenced(&connection, "script_counts").await);
    assert!(writes_between_scripts(&connection).await);
    admin.query_drop("DROP TABLE script_counts").await.unwrap();
}

#[tokio::test]
async fn a_statement_that_fails_rolls_the_run_back() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    scratch(&mut admin, "script_fails").await;
    let outcome = write(
        &connection,
        "INSERT INTO script_fails (id) VALUES (1);
         INSERT INTO script_fails_missing VALUES (1);
         INSERT INTO script_fails (id) VALUES (2)",
        10,
    )
    .await
    .unwrap();
    // Nothing made the server commit, so nothing is written: not `Partly`.
    assert_eq!(outcome.end, ScriptEnd::RolledBack);
    assert_eq!(outcome.rollback_warning, None);
    assert_eq!(outcome.results.len(), 2);
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_fails").await,
        0
    );
    assert!(writes_between_scripts(&connection).await);
    admin.query_drop("DROP TABLE script_fails").await.unwrap();
}

#[tokio::test]
async fn what_the_server_committed_by_itself_is_said_to_be_written() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    scratch(&mut admin, "script_partly").await;
    admin
        .query_drop("DROP TABLE IF EXISTS script_partly_made")
        .await
        .unwrap();
    // CREATE TABLE commits the first insert. The second is rolled back
    // with the statement that failed.
    let outcome = write(
        &connection,
        "INSERT INTO script_partly (id) VALUES (1);
         CREATE TABLE script_partly_made (id int);
         INSERT INTO script_partly (id) VALUES (2);
         INSERT INTO script_partly_missing VALUES (1)",
        10,
    )
    .await
    .unwrap();
    assert_eq!(outcome.end, ScriptEnd::Partly { committed: 2 });
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_partly").await,
        1
    );
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_partly_made").await,
        0
    );
    // A CREATE TABLE that fails has still committed what came before it.
    let outcome = write(
        &connection,
        "INSERT INTO script_partly (id) VALUES (3);
         CREATE TABLE script_partly_made (id int)",
        10,
    )
    .await
    .unwrap();
    assert_eq!(outcome.end, ScriptEnd::Partly { committed: 1 });
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_partly").await,
        2
    );
    // With every statement done, all of it is committed, whoever did it.
    let outcome = write(
        &connection,
        "INSERT INTO script_partly (id) VALUES (4);
         DROP TABLE script_partly_made;
         INSERT INTO script_partly (id) VALUES (5)",
        10,
    )
    .await
    .unwrap();
    assert_eq!(outcome.end, ScriptEnd::Committed);
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_partly").await,
        4
    );
    assert!(writes_between_scripts(&connection).await);
    admin.query_drop("DROP TABLE script_partly").await.unwrap();
}

#[tokio::test]
async fn a_failed_statement_no_transaction_holds_is_not_said_to_be_undone() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    scratch(&mut admin, "script_ddl").await;
    admin
        .query_drop("DROP TABLE IF EXISTS script_ddl_here, script_ddl_missing")
        .await
        .unwrap();
    admin
        .query_drop("CREATE TABLE script_ddl_here (id int)")
        .await
        .unwrap();
    // The server commits the insert, then fails the DROP on the table
    // that is missing. Whether the table that is there went with it is
    // the server's to decide: MySQL 8 drops all or none, MariaDB and older
    // MySQL drop what they find. The run cannot know, and says so.
    let outcome = write(
        &connection,
        "INSERT INTO script_ddl (id) VALUES (1);
         DROP TABLE script_ddl_here, script_ddl_missing",
        10,
    )
    .await
    .unwrap();
    assert!(matches!(
        outcome.results[1].outcome,
        StatementOutcome::Error { .. }
    ));
    assert_eq!(outcome.end, ScriptEnd::Partly { committed: 1 });
    let warning = outcome
        .rollback_warning
        .expect("the run says it cannot know");
    assert!(warning.contains("may have been applied"), "{warning}");
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_ddl").await,
        1
    );
    assert!(writes_between_scripts(&connection).await);
    admin
        .query_drop("DROP TABLE IF EXISTS script_ddl, script_ddl_here")
        .await
        .unwrap();
}

#[tokio::test]
async fn the_limit_cuts_what_is_shown_and_nothing_that_is_written() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    scratch(&mut admin, "script_limit").await;
    admin
        .query_drop("DROP TABLE IF EXISTS script_limit_copy")
        .await
        .unwrap();
    let outcome = write(
        &connection,
        "INSERT INTO script_limit (id) VALUES (1), (2), (3), (4), (5), (6), (7), (8);
         INSERT INTO script_limit (id) SELECT id + 100 FROM script_limit;
         CREATE TABLE script_limit_copy AS SELECT * FROM script_limit;
         SELECT id FROM script_limit",
        3,
    )
    .await
    .unwrap();
    assert_eq!(outcome.end, ScriptEnd::Committed);
    assert_eq!(affected(&outcome)[..2], [Some(8), Some(8)]);
    assert!(matches!(
        &outcome.results[3].outcome,
        StatementOutcome::Rows { rows, truncated: true, .. } if rows.len() == 3
    ));
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_limit").await,
        16
    );
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_limit_copy").await,
        16
    );
    admin
        .query_drop("DROP TABLE script_limit, script_limit_copy")
        .await
        .unwrap();
}

#[tokio::test]
async fn a_change_the_server_cannot_roll_back_is_said() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    admin
        .query_drop("DROP TABLE IF EXISTS script_myisam")
        .await
        .unwrap();
    admin
        .query_drop("CREATE TABLE script_myisam (id int PRIMARY KEY) ENGINE = MyISAM")
        .await
        .unwrap();
    let outcome = write(
        &connection,
        "INSERT INTO script_myisam VALUES (1); INSERT INTO script_myisam_missing VALUES (1)",
        10,
    )
    .await
    .unwrap();
    assert_eq!(outcome.end, ScriptEnd::RolledBack);
    let warning = outcome.rollback_warning.expect("the server warns");
    assert!(warning.contains("non-transactional"), "{warning}");
    // And it is true: the row stayed.
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_myisam").await,
        1
    );
    assert!(writes_between_scripts(&connection).await);
    admin.query_drop("DROP TABLE script_myisam").await.unwrap();
}

#[tokio::test]
async fn a_statement_says_how_many_warnings_it_raised() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    admin
        .query_drop("DROP TABLE IF EXISTS script_warns")
        .await
        .unwrap();
    admin
        .query_drop("CREATE TABLE script_warns (s varchar(3))")
        .await
        .unwrap();
    // IGNORE makes a warning of what strict mode would refuse.
    let outcome = write(
        &connection,
        "INSERT INTO script_warns VALUES ('abc'); INSERT IGNORE INTO script_warns VALUES ('abcdef')",
        10,
    )
    .await
    .unwrap();
    assert_eq!(outcome.end, ScriptEnd::Committed);
    assert_eq!(
        outcome.results[0].outcome,
        StatementOutcome::Done {
            affected: Some(1),
            warnings: 0,
        }
    );
    assert_eq!(
        outcome.results[1].outcome,
        StatementOutcome::Done {
            affected: Some(1),
            warnings: 1,
        }
    );
    admin.query_drop("DROP TABLE script_warns").await.unwrap();
}

/// Runs `text` as a script that writes and makes it lose a deadlock on
/// the statement holding `marker`: that statement must wait for the row
/// with id 1 of `table`, after the script has changed the row with id 2.
/// The other side has changed far more rows, so the server picks the
/// script as the one to roll back.
async fn deadlocked(table: &str, text: String, marker: &str) -> Option<ScriptOutcome> {
    let connection = std::sync::Arc::new(connect_as(Access::Writable).await?);
    let mut other = admin().await;
    other.query_drop("START TRANSACTION").await.unwrap();
    other
        .query_drop(format!(
            "INSERT INTO {table} (id) VALUES (10), (11), (12), (13), (14), (15), (16), (17), \
             (18), (19), (20), (21), (22), (23), (24), (25), (26), (27), (28), (29)"
        ))
        .await
        .unwrap();
    other
        .query_drop(format!("UPDATE {table} SET n = 1 WHERE id = 1"))
        .await
        .unwrap();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        tokio::spawn(async move {
            connection
                .run_script(&script(&text), 10, ScriptMode::Write, &StopFlag::new())
                .await
        })
    };
    let mut watcher = admin().await;
    runs_on_the_server(&mut watcher, marker).await;
    // Closes the circle: this waits for the script's row, the script waits
    // for this one's. The script loses, and this goes through.
    other
        .query_drop(format!("UPDATE {table} SET n = 1 WHERE id = 2"))
        .await
        .unwrap();
    let outcome = within(running).await.unwrap().unwrap();
    other.query_drop("ROLLBACK").await.unwrap();
    assert!(
        matches!(
            &outcome.results.last().unwrap().outcome,
            StatementOutcome::Error {
                error: Error::Query { code: Some(code), .. },
                ..
            } if code == "40001"
        ),
        "{:?}",
        outcome.results.last()
    );
    // The session survives, read-write between scripts.
    assert!(writes_between_scripts(&connection).await);
    Some(outcome)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_deadlock_rolls_the_run_back_and_is_not_taken_for_a_commit() {
    if url().is_none() {
        eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
        return;
    }
    let mut admin = admin().await;
    scratch(&mut admin, "script_deadlock").await;
    admin
        .query_drop("INSERT INTO script_deadlock (id) VALUES (1), (2), (3)")
        .await
        .unwrap();
    // The server rolls the whole transaction back and leaves the session
    // outside one, as a commit would. Nothing of the run is written.
    let text = "UPDATE script_deadlock SET n = 7 WHERE id = 3; \
                UPDATE script_deadlock SET n = 7 WHERE id = 2; \
                UPDATE script_deadlock SET n = 7 /* tabletist deadlock one */ WHERE id = 1";
    let Some(outcome) =
        deadlocked("script_deadlock", text.to_owned(), "tabletist deadlock one").await
    else {
        return;
    };
    assert_eq!(outcome.end, ScriptEnd::RolledBack);
    assert_eq!(
        counted(
            &mut admin,
            "SELECT count(*) FROM script_deadlock WHERE n = 7"
        )
        .await,
        0
    );
    // After a commit the server made earlier in the run, only what came
    // before that commit is written.
    admin
        .query_drop("DROP TABLE IF EXISTS script_deadlock_made")
        .await
        .unwrap();
    let text = "UPDATE script_deadlock SET n = 7 WHERE id = 3; \
                CREATE TABLE script_deadlock_made (id int); \
                UPDATE script_deadlock SET n = 8 WHERE id = 2; \
                UPDATE script_deadlock SET n = 8 /* tabletist deadlock two */ WHERE id = 1";
    let Some(outcome) =
        deadlocked("script_deadlock", text.to_owned(), "tabletist deadlock two").await
    else {
        return;
    };
    assert_eq!(outcome.end, ScriptEnd::Partly { committed: 2 });
    assert_eq!(
        counted(
            &mut admin,
            "SELECT count(*) FROM script_deadlock WHERE n = 7"
        )
        .await,
        1
    );
    assert_eq!(
        counted(
            &mut admin,
            "SELECT count(*) FROM script_deadlock WHERE n = 8"
        )
        .await,
        0
    );
    admin
        .query_drop("DROP TABLE script_deadlock, script_deadlock_made")
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_that_writes_and_is_cancelled_is_rolled_back() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let connection = std::sync::Arc::new(connection);
    let mut admin = admin().await;
    scratch(&mut admin, "script_cancelled").await;
    let cancel = connection.cancel_handle();
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        tokio::spawn(async move {
            let text = "INSERT INTO script_cancelled (id) VALUES (1); \
                        SELECT count(*) FROM users WHERE SLEEP(35) = 0";
            connection
                .run_script(&script(text), 10, ScriptMode::Write, &stop)
                .await
        })
    };
    // A sleep of its own length: `runs_on_the_server` sees every session.
    runs_on_the_server(&mut admin, "SLEEP(35)").await;
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
    assert!(outcome.was_cancelled());
    assert_eq!(outcome.end, ScriptEnd::RolledBack);
    // The insert ran, and is undone.
    assert_eq!(affected(&outcome), [Some(1), None]);
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_cancelled").await,
        0
    );
    // The session survives, read-write between scripts as it connected.
    assert!(writes_between_scripts(&connection).await);
    assert_connect_time_settings(&connection).await;
    admin
        .query_drop("DROP TABLE script_cancelled")
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stop_after_a_statement_that_committed_still_says_what_is_written() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let connection = std::sync::Arc::new(connection);
    let mut admin = admin().await;
    scratch(&mut admin, "script_stopped").await;
    admin
        .query_drop("DROP TABLE IF EXISTS script_stopped_made")
        .await
        .unwrap();
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        tokio::spawn(async move {
            let text = "INSERT INTO script_stopped (id) VALUES (1); \
                        CREATE TABLE script_stopped_made AS SELECT SLEEP(1) AS slept; \
                        INSERT INTO script_stopped (id) VALUES (2)";
            connection
                .run_script(&script(text), 10, ScriptMode::Write, &stop)
                .await
        })
    };
    // A stop without a cancel: the CREATE TABLE ends by itself, having
    // committed the insert before it, and the statement after it never
    // starts. The check that would have seen the commit never ran either.
    runs_on_the_server(&mut admin, "script_stopped_made").await;
    stop.stop();
    let outcome = within(running).await.unwrap().unwrap();
    assert_eq!(outcome.results.len(), 3);
    assert_eq!(outcome.results[2].outcome, StatementOutcome::Cancelled);
    assert_eq!(outcome.end, ScriptEnd::Partly { committed: 2 });
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_stopped").await,
        1
    );
    assert!(writes_between_scripts(&connection).await);
    admin
        .query_drop("DROP TABLE script_stopped, script_stopped_made")
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stop_during_a_last_statement_that_committed_is_not_said_to_be_rolled_back() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let connection = std::sync::Arc::new(connection);
    let mut admin = admin().await;
    scratch(&mut admin, "script_stopped_last").await;
    admin
        .query_drop("DROP TABLE IF EXISTS script_stopped_last_made")
        .await
        .unwrap();
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        tokio::spawn(async move {
            let text = "INSERT INTO script_stopped_last (id) VALUES (1); \
                        CREATE TABLE script_stopped_last_made AS SELECT SLEEP(1) AS slept";
            connection
                .run_script(&script(text), 10, ScriptMode::Write, &stop)
                .await
        })
    };
    // A stop without a cancel, while the last statement runs: it ends by
    // itself, having committed the insert before it. Nothing is left to
    // roll back, so the run must not say that it was.
    runs_on_the_server(&mut admin, "script_stopped_last_made").await;
    stop.stop();
    let outcome = within(running).await.unwrap().unwrap();
    assert_eq!(outcome.results.len(), 2);
    assert!(outcome.stopped);
    assert_eq!(outcome.end, ScriptEnd::Committed);
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_stopped_last").await,
        1
    );
    assert_eq!(
        counted(&mut admin, "SELECT count(*) FROM script_stopped_last_made").await,
        1
    );
    assert!(writes_between_scripts(&connection).await);
    admin
        .query_drop("DROP TABLE script_stopped_last, script_stopped_last_made")
        .await
        .unwrap();
}

#[tokio::test]
async fn a_run_that_writes_leaves_the_session_as_it_connected() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    scratch(&mut admin, "script_session").await;
    let leaves = |id: u32| {
        format!(
            "SET time_zone = '+05:00'; SET @tabletist_left_over = 1; \
             SET sql_select_limit = 1; INSERT INTO script_session (id) VALUES ({id})"
        )
    };
    let outcome = write(&connection, &leaves(1), 10).await.unwrap();
    assert_eq!(outcome.end, ScriptEnd::Committed);
    assert_eq!(outcome.broken, None);
    assert_connect_time_settings(&connection).await;
    assert!(writes_between_scripts(&connection).await);
    // After a run that failed, and so was rolled back.
    let failing = format!(
        "{}; INSERT INTO script_session_missing VALUES (1)",
        leaves(2)
    );
    let outcome = write(&connection, &failing, 10).await.unwrap();
    assert!(matches!(
        outcome.results.last().unwrap().outcome,
        StatementOutcome::Error { .. }
    ));
    assert_connect_time_settings(&connection).await;
    assert!(writes_between_scripts(&connection).await);
    assert!(is_fenced(&connection, "script_session").await);
    admin.query_drop("DROP TABLE script_session").await.unwrap();
}

use std::future::Future;
use std::panic::AssertUnwindSafe;

use futures_util::FutureExt;
use tabletist_db::{
    CellChange, ChangeSet, Conflict, InsertValue, NewValue, RowChange, RowInsert, WriteOutcome,
};

/// Runs `test` on tables of its own. The fixture is loaded once and shared
/// by every test of this suite, so a test that writes never touches it.
/// `tables` names what the test writes to, names no other test uses;
/// `create` makes them, and they are dropped however the test ends, a
/// failed assertion included, and before anything else, for what a run
/// that was killed left behind.
async fn on_its_own_tables<T>(tables: &str, create: &[&str], test: impl Future<Output = T>) -> T {
    let mut admin = admin().await;
    let drop = format!("DROP TABLE IF EXISTS {tables}");
    admin.query_drop(&drop).await.unwrap();
    let outcome = AssertUnwindSafe(async {
        for statement in create {
            admin.query_drop(*statement).await.unwrap();
        }
        test.await
    })
    .catch_unwind()
    .await;
    // A save that left its transaction open would hold the table, and the
    // drop would wait for it for good: it gives up, and the test fails.
    admin
        .query_drop("SET SESSION lock_wait_timeout = 10")
        .await
        .unwrap();
    let dropped = admin.query_drop(&drop).await;
    let value = outcome.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
    dropped.unwrap();
    value
}

/// A table of three people named `table`, for a test to write to.
fn people(table: &str) -> [String; 2] {
    [
        format!(
            "CREATE TABLE {table} (
                 id INT PRIMARY KEY,
                 email VARCHAR(255) NOT NULL UNIQUE,
                 name VARCHAR(255),
                 score DOUBLE
             )"
        ),
        format!(
            "INSERT INTO {table} VALUES
                 (1, 'a@x', 'Ada', 0.1), (2, 'b@x', 'Bea', NULL), (3, 'c@x', 'Cy', 3)"
        ),
    ]
}

/// The rows of `table` as a page gives them, in key order, and the names
/// of their columns.
async fn page_of(connection: &Connection, table: &str) -> (Vec<String>, Vec<Vec<Value>>) {
    let query = RowQuery::new(ObjectRef::new("tabletist", table), 50);
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

/// A change of one column that is given what the page held.
fn one(column: &str, type_name: &str, loaded: Value, new: NewValue) -> Vec<CellChange> {
    vec![CellChange {
        column: column.into(),
        type_name: type_name.into(),
        loaded,
        new,
    }]
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
        object: ObjectRef::new("tabletist", table),
        inserts: Vec::new(),
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
    within(connection.write(&changes_to(table, vec![by_id(id, set)]), &StopFlag::new())).await
}

fn to(text: &str) -> NewValue {
    NewValue::Text(text.into())
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

/// Whether nothing holds `table`: no transaction that read or wrote it is
/// still open. Asked by taking the whole table for a moment, which waits
/// for every such transaction.
async fn nothing_holds(admin: &mut mysql_async::Conn, table: &str) -> bool {
    admin
        .query_drop("SET SESSION lock_wait_timeout = 2")
        .await
        .unwrap();
    let taken = admin.query_drop(format!("LOCK TABLES {table} WRITE")).await;
    admin.query_drop("UNLOCK TABLES").await.unwrap();
    taken.is_ok()
}

const VARCHAR: &str = "varchar(255)";

#[tokio::test]
async fn a_save_writes_every_kind_of_value_and_reads_the_row_back() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "write_types",
        &[
            "CREATE TABLE write_types (
                 id INT PRIMARY KEY,
                 email VARCHAR(255) NOT NULL UNIQUE,
                 name VARCHAR(255),
                 created_at DATETIME(6) NOT NULL,
                 active TINYINT(1) NOT NULL,
                 meta JSON,
                 score DOUBLE,
                 balance DECIMAL(14, 2),
                 counter BIGINT UNSIGNED,
                 mood ENUM('happy', 'sad'),
                 birthday DATE,
                 alarm TIME
             )",
            "INSERT INTO write_types VALUES
                 (1, 'a@x', 'Ada', '2026-01-02 09:00:00', 1, '{\"a\": 1}', 0.1, 12.50, 5, 'happy',
                  '1815-12-10', '07:30:00'),
                 (2, 'b@x', 'Bea', '2026-01-03 10:30:00', 0, NULL, NULL, NULL, NULL, NULL, NULL,
                  NULL)",
        ],
        async move {
            let (columns, untouched) = row_of(&connection, "write_types", 2).await;
            let outcome = save(
                &connection,
                "write_types",
                1,
                &[
                    ("email", VARCHAR, to("new@example.com")),
                    ("name", VARCHAR, NewValue::Null),
                    (
                        "created_at",
                        "datetime(6)",
                        to("2027-02-03 04:05:06.000007"),
                    ),
                    ("active", "tinyint(1)", to("false")),
                    ("meta", "json", to(r#"{"plan": "pro"}"#)),
                    ("score", "double", to("12.5")),
                    ("balance", "decimal(14,2)", to("99.95")),
                    ("counter", "bigint unsigned", to("18446744073709551615")),
                    ("mood", "enum('happy','sad')", to("sad")),
                    ("birthday", "date", to("1999-12-31")),
                    ("alarm", "time", to("-25:15:00")),
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
            assert_eq!(at("created_at"), text("2027-02-03 04:05:06.000007"));
            assert_eq!(at("active"), Value::Int(0));
            assert_eq!(at("meta"), text(r#"{"plan": "pro"}"#));
            assert_eq!(at("score"), Value::Float(12.5));
            assert_eq!(at("balance"), text("99.95"));
            assert_eq!(at("counter"), text("18446744073709551615"));
            assert_eq!(at("mood"), text("sad"));
            assert_eq!(at("birthday"), text("1999-12-31"));
            assert_eq!(at("alarm"), text("-25:15:00"));
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
                    ("email", VARCHAR, to("back@example.com")),
                    ("name", VARCHAR, to("Ada")),
                    ("created_at", "datetime(6)", to("2026-01-02 09:00:00")),
                    ("active", "tinyint(1)", to("true")),
                    ("meta", "json", NewValue::Null),
                    ("score", "double", to("0.1")),
                    ("balance", "decimal(14,2)", to("12.50")),
                    ("counter", "bigint unsigned", to("5")),
                    ("mood", "enum('happy','sad')", to("happy")),
                    ("birthday", "date", NewValue::Null),
                    ("alarm", "time", to("07:30:00")),
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
    let people = people("write_conflict");
    on_its_own_tables("write_conflict", &[&people[0], &people[1]], async move {
        // Loaded, then changed behind the page's back.
        let (columns, first) = row_of(&connection, "write_conflict", 1).await;
        let (_, second) = row_of(&connection, "write_conflict", 2).await;
        let changes = changes_to(
            "write_conflict",
            vec![
                by_id(
                    2,
                    vec![cell(&columns, &second, "name", VARCHAR, to("Second"))],
                ),
                by_id(1, vec![cell(&columns, &first, "name", VARCHAR, to("Mine"))]),
            ],
        );
        admin()
            .await
            .query_drop("UPDATE write_conflict SET name = 'Theirs' WHERE id = 1")
            .await
            .unwrap();
        let outcome = within(connection.write(&changes, &StopFlag::new()))
            .await
            .unwrap();
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
    })
    .await;
}

#[tokio::test]
async fn a_change_to_a_column_the_save_leaves_alone_is_no_conflict() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let people = people("write_elsewhere");
    on_its_own_tables("write_elsewhere", &[&people[0], &people[1]], async move {
        let (columns, row) = row_of(&connection, "write_elsewhere", 1).await;
        let changes = changes_to(
            "write_elsewhere",
            vec![by_id(
                1,
                vec![cell(&columns, &row, "name", VARCHAR, to("Mine"))],
            )],
        );
        admin()
            .await
            .query_drop("UPDATE write_elsewhere SET score = 99 WHERE id = 1")
            .await
            .unwrap();
        let outcome = within(connection.write(&changes, &StopFlag::new())).await;
        assert!(
            matches!(outcome, Ok(WriteOutcome::Written { .. })),
            "{outcome:?}"
        );
        // Their change stands beside this one.
        assert_eq!(
            row_of(&connection, "write_elsewhere", 1).await.1[2..],
            [text("Mine"), Value::Float(99.0)]
        );
    })
    .await;
}

#[tokio::test]
async fn a_row_that_is_gone_is_a_conflict_without_a_row() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let people = people("write_gone");
    on_its_own_tables("write_gone", &[&people[0], &people[1]], async move {
        let (columns, row) = row_of(&connection, "write_gone", 3).await;
        let changes = changes_to(
            "write_gone",
            vec![by_id(
                3,
                vec![cell(&columns, &row, "name", VARCHAR, to("Late"))],
            )],
        );
        admin()
            .await
            .query_drop("DELETE FROM write_gone WHERE id = 3")
            .await
            .unwrap();
        assert_eq!(
            within(connection.write(&changes, &StopFlag::new())).await,
            Ok(WriteOutcome::Conflicts(vec![Conflict {
                row: 0,
                server: None
            }]))
        );
    })
    .await;
}

#[tokio::test]
async fn a_statement_that_fails_undoes_the_rows_before_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let people = people("write_undone");
    on_its_own_tables("write_undone", &[&people[0], &people[1]], async move {
        let (columns, first) = row_of(&connection, "write_undone", 1).await;
        let (_, second) = row_of(&connection, "write_undone", 2).await;
        let changes = changes_to(
            "write_undone",
            vec![
                by_id(
                    1,
                    vec![cell(&columns, &first, "name", VARCHAR, to("Written first"))],
                ),
                // NOT NULL: the database refuses it.
                by_id(
                    2,
                    vec![cell(&columns, &second, "email", VARCHAR, NewValue::Null)],
                ),
            ],
        );
        let outcome = within(connection.write(&changes, &StopFlag::new()))
            .await
            .unwrap();
        assert!(
            matches!(
                &outcome,
                WriteOutcome::Failed { row: 1, error: Error::Query { code, .. } }
                    if code.as_deref() == Some("23000")
            ),
            "{outcome:?}"
        );
        assert_eq!(row_of(&connection, "write_undone", 1).await.1, first);
        assert_eq!(row_of(&connection, "write_undone", 2).await.1, second);
    })
    .await;
}

/// Bookshop's covers, as the design's new row needs them: a counted key, a
/// required column, a unique one, and two defaults.
fn covers(table: &str) -> [String; 2] {
    [
        format!(
            "CREATE TABLE {table} (
                 id BIGINT AUTO_INCREMENT PRIMARY KEY,
                 publisher_id BIGINT NOT NULL,
                 kind VARCHAR(20) NOT NULL DEFAULT 'print',
                 isbn VARCHAR(32) UNIQUE,
                 created_at DATETIME NOT NULL DEFAULT '2026-10-07 10:42:09'
             )"
        ),
        format!(
            "INSERT INTO {table} (publisher_id, kind, isbn) VALUES
                 (9100000000000000001, 'print', '978-1-4028-9462-6'),
                 (9100000000000000001, 'ebook', NULL)"
        ),
    ]
}

fn sets(column: &str, type_name: &str, new: &str) -> InsertValue {
    InsertValue {
        column: column.into(),
        type_name: type_name.into(),
        new: to(new),
    }
}

#[tokio::test]
async fn default_and_now_are_stored_as_the_database_makes_them() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "write_words",
        &[
            "CREATE TABLE write_words (
                 id INT PRIMARY KEY,
                 kind VARCHAR(20) NOT NULL DEFAULT 'print',
                 made_at DATETIME(6) NOT NULL,
                 made_on DATE,
                 opens_at TIME
             )",
            "INSERT INTO write_words VALUES (1, 'ebook', '2020-02-02 00:00:00', NULL, NULL)",
        ],
        async move {
            // A date and a time of day each get their own word: a time cut
            // to a date is a warning there, and a save takes one for a
            // failure.
            let outcome = save(
                &connection,
                "write_words",
                1,
                &[
                    (
                        "kind",
                        "varchar(20)",
                        NewValue::Default { expression: None },
                    ),
                    ("made_at", "datetime(6)", NewValue::Now),
                    ("made_on", "date", NewValue::Now),
                    ("opens_at", "time", NewValue::Now),
                ],
            )
            .await
            .unwrap();
            assert!(
                matches!(outcome, WriteOutcome::Written { .. }),
                "{outcome:?}"
            );
            let (columns, row) = row_of(&connection, "write_words", 1).await;
            let at = |name: &str| &row[columns.iter().position(|column| column == name).unwrap()];
            assert_eq!(*at("kind"), text("print"));
            let made = format!("{:?}", at("made_at"));
            assert!(
                !made.contains("2020-02-02") && !made.contains("CURRENT"),
                "{made}"
            );
            assert!(!at("made_at").is_null() && !at("made_on").is_null());
            assert!(!at("opens_at").is_null());
            // And in a new row.
            let value = |column: &str, type_name: &str, new: NewValue| InsertValue {
                column: column.into(),
                type_name: type_name.into(),
                new,
            };
            let mut changes = changes_to("write_words", Vec::new());
            changes.inserts = vec![RowInsert {
                set: vec![
                    value("id", "int", to("2")),
                    value(
                        "kind",
                        "varchar(20)",
                        NewValue::Default { expression: None },
                    ),
                    value("made_at", "datetime(6)", NewValue::Now),
                ],
            }];
            let outcome = within(connection.write(&changes, &StopFlag::new()))
                .await
                .unwrap();
            assert!(
                matches!(outcome, WriteOutcome::Written { .. }),
                "{outcome:?}"
            );
            let (columns, row) = row_of(&connection, "write_words", 2).await;
            let at = |name: &str| &row[columns.iter().position(|column| column == name).unwrap()];
            assert_eq!(*at("kind"), text("print"));
            assert!(!at("made_at").is_null());
        },
    )
    .await;
}

#[tokio::test]
async fn a_new_row_comes_back_as_the_database_stored_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let covers = covers("covers_new");
    on_its_own_tables("covers_new", &[&covers[0], &covers[1]], async move {
        let mut changes = changes_to("covers_new", Vec::new());
        changes.inserts = vec![RowInsert {
            set: vec![sets("publisher_id", "bigint", "9100000000000000004")],
        }];
        let outcome = within(connection.write(&changes, &StopFlag::new()))
            .await
            .unwrap();
        let WriteOutcome::Written { inserted, rows, .. } = outcome else {
            panic!("the save wrote");
        };
        assert!(rows.is_empty());
        // Two covers were there: the counter gives 3.
        let (columns, stored) = row_of(&connection, "covers_new", 3).await;
        assert_eq!(inserted, [Some(stored.clone())]);
        let at = |name: &str| &stored[columns.iter().position(|column| column == name).unwrap()];
        assert_eq!(*at("publisher_id"), Value::Int(9_100_000_000_000_000_004));
        assert_eq!(*at("kind"), Value::Text("print".into()));
    })
    .await;
}

#[tokio::test]
async fn a_new_row_whose_key_was_typed_is_found_by_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let people = people("people_new");
    on_its_own_tables("people_new", &[&people[0], &people[1]], async move {
        let mut changes = changes_to("people_new", Vec::new());
        changes.inserts = vec![RowInsert {
            set: vec![sets("id", "int", "7"), sets("email", VARCHAR, "g@x")],
        }];
        let outcome = within(connection.write(&changes, &StopFlag::new()))
            .await
            .unwrap();
        let WriteOutcome::Written { inserted, .. } = outcome else {
            panic!("the save wrote");
        };
        assert_eq!(
            inserted,
            [Some(row_of(&connection, "people_new", 7).await.1)]
        );
        // In other letters than the table's: MySQL takes `ID` for `id`,
        // and the row is found all the same.
        let mut changes = changes_to("people_new", Vec::new());
        changes.inserts = vec![RowInsert {
            set: vec![sets("ID", "int", "8"), sets("Email", VARCHAR, "h@x")],
        }];
        let outcome = within(connection.write(&changes, &StopFlag::new()))
            .await
            .unwrap();
        let WriteOutcome::Written { inserted, .. } = outcome else {
            panic!("the save wrote");
        };
        assert_eq!(
            inserted,
            [Some(row_of(&connection, "people_new", 8).await.1)]
        );
    })
    .await;
}

#[tokio::test]
async fn a_new_row_that_fails_undoes_the_rows_before_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let covers = covers("covers_undone");
    on_its_own_tables("covers_undone", &[&covers[0], &covers[1]], async move {
        let (columns, second) = row_of(&connection, "covers_undone", 2).await;
        let mut changes = changes_to(
            "covers_undone",
            vec![by_id(
                2,
                vec![cell(&columns, &second, "kind", "varchar(20)", to("audio"))],
            )],
        );
        changes.inserts = vec![
            RowInsert {
                set: vec![sets("publisher_id", "bigint", "9100000000000000004")],
            },
            // UNIQUE: the first cover has it.
            RowInsert {
                set: vec![
                    sets("publisher_id", "bigint", "9100000000000000004"),
                    sets("isbn", "varchar(32)", "978-1-4028-9462-6"),
                ],
            },
        ];
        let outcome = within(connection.write(&changes, &StopFlag::new()))
            .await
            .unwrap();
        assert!(
            matches!(
                &outcome,
                // MySQL's 1062, by its SQLSTATE.
                WriteOutcome::FailedInsert { insert: 1, error: Error::Query { code, .. } }
                    if code.as_deref() == Some("23000")
            ),
            "{outcome:?}"
        );
        // Neither the first new row nor the change is there.
        assert_eq!(page_of(&connection, "covers_undone").await.1.len(), 2);
        assert_eq!(row_of(&connection, "covers_undone", 2).await.1, second);
    })
    .await;
}

/// A trigger can give a new row another key than the one it was sent with,
/// and give that one to another row: the key then finds a row, and not
/// this one. On a table with a trigger a new row is written and handed
/// back as not known.
#[tokio::test]
async fn a_new_row_of_a_table_with_a_trigger_is_written_and_not_known() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "codes_moved",
        &[
            "CREATE TABLE codes_moved (code VARCHAR(20) PRIMARY KEY, note VARCHAR(20))",
            "INSERT INTO codes_moved VALUES ('a', 'there before')",
        ],
        async move {
            // What is sent as `a2` is stored as `b`, and `b` as `a2`. With
            // its binary log on, the server lets a user without SUPER make
            // a trigger only where `log_bin_trust_function_creators` is
            // set, as `compose.yaml` sets it.
            let made = self::admin()
                .await
                .query_drop(
                    "CREATE TRIGGER codes_moved_swap BEFORE INSERT ON codes_moved FOR EACH ROW \
                     SET NEW.code = CASE NEW.code WHEN 'a2' THEN 'b' WHEN 'b' THEN 'a2' \
                                    ELSE NEW.code END",
                )
                .await;
            if let Err(error) = made {
                eprintln!("skipped: the server lets this user make no trigger: {error}");
                return;
            }
            let mut changes = changes_to("codes_moved", Vec::new());
            changes.inserts = vec![
                RowInsert {
                    set: vec![
                        sets("code", "varchar(20)", "a2"),
                        sets("note", "varchar(20)", "first"),
                    ],
                },
                RowInsert {
                    set: vec![
                        sets("code", "varchar(20)", "b"),
                        sets("note", "varchar(20)", "second"),
                    ],
                },
            ];
            let outcome = within(connection.write(&changes, &StopFlag::new()))
                .await
                .unwrap();
            let WriteOutcome::Written { inserted, .. } = outcome else {
                panic!("the save wrote");
            };
            // Each key sent finds a row, and it is the other's.
            assert_eq!(inserted, [None, None]);
            assert_eq!(page_of(&connection, "codes_moved").await.1.len(), 3);
        },
    )
    .await;
}

/// A change cannot carry a new row of its own table with it: InnoDB lets no
/// update cascade back into the table it started in, and refuses the
/// change. So a new row found by its key after the save's changes is the
/// row that was made, and the save asks nothing about foreign keys.
#[tokio::test]
async fn a_change_cannot_carry_a_new_row_of_its_own_table() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "chapters_carried",
        &[
            "CREATE TABLE chapters_carried (
                 id INT PRIMARY KEY,
                 code VARCHAR(20) UNIQUE,
                 part VARCHAR(20),
                 FOREIGN KEY (part) REFERENCES chapters_carried (code) ON UPDATE CASCADE
             )",
            "INSERT INTO chapters_carried VALUES (1, 'a', NULL)",
        ],
        async move {
            let (columns, first) = row_of(&connection, "chapters_carried", 1).await;
            let mut changes = changes_to(
                "chapters_carried",
                vec![by_id(
                    1,
                    vec![cell(&columns, &first, "code", "varchar(20)", to("z"))],
                )],
            );
            changes.inserts = vec![RowInsert {
                set: vec![
                    sets("id", "int", "2"),
                    sets("code", "varchar(20)", "b"),
                    sets("part", "varchar(20)", "a"),
                ],
            }];
            let outcome = within(connection.write(&changes, &StopFlag::new()))
                .await
                .unwrap();
            // The change of the row the new one refers to is refused, and
            // the save with it: nothing was carried anywhere.
            assert!(
                matches!(
                    &outcome,
                    WriteOutcome::Failed { row: 0, error: Error::Query { code, .. } }
                        if code.as_deref() == Some("23000")
                ),
                "{outcome:?}"
            );
            assert_eq!(page_of(&connection, "chapters_carried").await.1.len(), 1);
        },
    )
    .await;
}

/// The server fires a table's triggers for everyone and lists them only to
/// who has the TRIGGER privilege on it. A user without it is told of none,
/// which proves none: their new row is written and handed back as not
/// known. `compose/mysql-init.sql` makes that user.
#[tokio::test]
async fn a_user_who_is_shown_no_triggers_gets_a_new_row_back_as_not_known() {
    let Some((mut spec, mut secrets)) = spec() else {
        eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
        return;
    };
    load_fixture().await;
    spec.user = "writer".into();
    secrets.password = Some("writer".into());
    let connection =
        match Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::Writable)
            .await
        {
            Ok(connection) => connection,
            Err(error) => {
                eprintln!("skipped: the server has no user without TRIGGER: {error}");
                return;
            }
        };
    let people = people("people_unseen");
    on_its_own_tables("people_unseen", &[&people[0], &people[1]], async move {
        let mut changes = changes_to("people_unseen", Vec::new());
        changes.inserts = vec![RowInsert {
            set: vec![sets("id", "int", "7"), sets("email", VARCHAR, "g@x")],
        }];
        let outcome = within(connection.write(&changes, &StopFlag::new()))
            .await
            .unwrap();
        let WriteOutcome::Written { inserted, .. } = outcome else {
            panic!("the save wrote");
        };
        // The same save as `a_new_row_whose_key_was_typed_is_found_by_it`,
        // which a user who is shown the table's triggers gets its row from.
        assert_eq!(inserted, [None]);
        assert_eq!(row_of(&connection, "people_unseen", 7).await.0.len(), 4);
    })
    .await;
}

#[tokio::test]
async fn a_table_without_a_key_takes_a_new_row_and_cannot_hand_it_back() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "cover_stamps",
        &["CREATE TABLE cover_stamps (line VARCHAR(20) DEFAULT 'none', n INT DEFAULT 7)"],
        async move {
            let mut changes = changes_to("cover_stamps", Vec::new());
            // Nothing set: `() VALUES ()`.
            changes.inserts = vec![RowInsert { set: Vec::new() }];
            let outcome = within(connection.write(&changes, &StopFlag::new()))
                .await
                .unwrap();
            let WriteOutcome::Written { inserted, .. } = outcome else {
                panic!("the save wrote");
            };
            // Written, and no key to find it by.
            assert_eq!(inserted, [None]);
            assert_eq!(
                page_of(&connection, "cover_stamps").await.1,
                [vec![Value::Text("none".into()), Value::Int(7)]]
            );
        },
    )
    .await;
}

#[tokio::test]
async fn a_value_the_column_cannot_take_is_the_databases_error() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let people = people("write_refused");
    on_its_own_tables("write_refused", &[&people[0], &people[1]], async move {
        let before = row_of(&connection, "write_refused", 1).await.1;
        let outcome = save(
            &connection,
            "write_refused",
            1,
            &[("score", "double", to("high"))],
        )
        .await
        .unwrap();
        assert!(
            matches!(
                &outcome,
                WriteOutcome::Failed {
                    row: 0,
                    error: Error::Query { .. }
                }
            ),
            "{outcome:?}"
        );
        assert_eq!(row_of(&connection, "write_refused", 1).await.1, before);
    })
    .await;
}

#[tokio::test]
async fn a_save_on_a_read_only_connection_is_refused() {
    let Some(connection) = connect().await else {
        return;
    };
    let people = people("write_read_only");
    on_its_own_tables("write_read_only", &[&people[0], &people[1]], async move {
        let before = row_of(&connection, "write_read_only", 1).await.1;
        let outcome = save(
            &connection,
            "write_read_only",
            1,
            &[("name", VARCHAR, to("Grace"))],
        )
        .await;
        assert_eq!(outcome, Err(Error::ReadOnly));
        assert_eq!(row_of(&connection, "write_read_only", 1).await.1, before);
    })
    .await;
}

/// However a save ends, its transaction ends with it: the session holds
/// neither the table nor a row, and is what a script and a page expect.
#[tokio::test]
async fn a_save_leaves_no_transaction_open_however_it_ends() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let people = people("write_session");
    on_its_own_tables(
        "write_session, write_session_twins",
        &[
            &people[0],
            &people[1],
            "CREATE TABLE write_session_twins (id INT, name VARCHAR(255))",
            "INSERT INTO write_session_twins VALUES (1, 'a'), (1, 'b')",
        ],
        async move {
            let mut admin = admin().await;
            let (columns, row) = row_of(&connection, "write_session", 1).await;
            let mut stale = row.clone();
            stale[2] = text("What the page never held");
            let name =
                |row: &[Value], new: &str| vec![cell(&columns, row, "name", VARCHAR, to(new))];
            type Ended = fn(&tabletist_db::Result<WriteOutcome>) -> bool;
            let exits: [(&str, ChangeSet, Ended); 8] = [
                (
                    "a conflict",
                    changes_to("write_session", vec![by_id(1, name(&stale, "Mine"))]),
                    |ended| matches!(ended, Ok(WriteOutcome::Conflicts(_))),
                ),
                (
                    "a row that is gone",
                    changes_to("write_session", vec![by_id(99, name(&row, "Mine"))]),
                    |ended| matches!(ended, Ok(WriteOutcome::Conflicts(_))),
                ),
                (
                    "a statement the database refuses",
                    changes_to(
                        "write_session",
                        vec![by_id(
                            1,
                            vec![cell(&columns, &row, "email", VARCHAR, NewValue::Null)],
                        )],
                    ),
                    |ended| matches!(ended, Ok(WriteOutcome::Failed { row: 0, .. })),
                ),
                (
                    "a value that cannot be sent",
                    changes_to(
                        "write_session",
                        vec![by_id(
                            1,
                            one("name", "tinyint(1)", row[2].clone(), to("maybe")),
                        )],
                    ),
                    |ended| matches!(ended, Ok(WriteOutcome::Failed { row: 0, .. })),
                ),
                (
                    "a column the table does not have",
                    changes_to(
                        "write_session",
                        vec![by_id(1, one("nick", VARCHAR, Value::Null, to("Mine")))],
                    ),
                    |ended| matches!(ended, Err(Error::Query { .. })),
                ),
                (
                    "a table that is not there",
                    changes_to("write_session_nowhere", vec![by_id(1, name(&row, "Mine"))]),
                    |ended| matches!(ended, Err(Error::Unsupported(_))),
                ),
                (
                    "a key that matches two rows",
                    changes_to(
                        "write_session_twins",
                        vec![by_id(1, one("name", VARCHAR, text("a"), to("Mine")))],
                    ),
                    |ended| matches!(ended, Err(Error::Query { .. })),
                ),
                (
                    "a save that is written",
                    changes_to("write_session", vec![by_id(1, name(&row, "Grace"))]),
                    |ended| matches!(ended, Ok(WriteOutcome::Written { .. })),
                ),
            ];
            for (exit, changes, ended_so) in exits {
                let ended = within(connection.write(&changes, &StopFlag::new())).await;
                assert!(ended_so(&ended), "{exit}: {ended:?}");
                for table in ["write_session", "write_session_twins"] {
                    assert!(nothing_holds(&mut admin, table).await, "{exit}: {table}");
                }
                // The session is read-write between scripts, as it connected.
                assert!(writes_between_scripts(&connection).await, "{exit}");
            }
            // Only the last of them wrote.
            assert_eq!(
                row_of(&connection, "write_session", 1).await.1[1..3],
                [text("a@x"), text("Grace")]
            );
            // A script is fenced as before, and finds the session as every
            // script does.
            assert_connect_time_settings(&connection).await;
            let outcome = run(
                &connection,
                "INSERT INTO write_session VALUES (9, 'z@x', 'Z', 1)",
                10,
            )
            .await
            .unwrap();
            assert!(
                matches!(
                    &outcome.results[0].outcome,
                    StatementOutcome::Error { error: Error::Query { code: Some(code), .. }, .. }
                        if code == "25006"
                ),
                "{outcome:?}"
            );
            assert!(writes_between_scripts(&connection).await);
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
        "write_composite",
        &[
            "CREATE TABLE write_composite (
                 a INT, b VARBINARY(16), note VARCHAR(255), PRIMARY KEY (a, b)
             )",
            "INSERT INTO write_composite VALUES
                 (1, X'00FF10', 'first'), (1, X'00FF11', 'second'), (2, X'00FF10', 'third')",
        ],
        async move {
            let (_, before) = page_of(&connection, "write_composite").await;
            let bytes = Value::Bytes(vec![0x00, 0xff, 0x10].into());
            assert_eq!(before[0][1], bytes);
            let changes = changes_to(
                "write_composite",
                vec![RowChange {
                    key: vec![("a".into(), Value::Int(1)), ("b".into(), bytes.clone())],
                    set: one("note", VARCHAR, text("first"), to("changed")),
                }],
            );
            let outcome = within(connection.write(&changes, &StopFlag::new())).await.unwrap();
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

/// A key the save cannot match exactly is refused, and nothing is written:
/// a `TIMESTAMP` is shown without its zone, so two instants can read
/// alike; a `BIT` goes as bytes, which the server reads as a number's text;
/// a `FLOAT` goes as a double, which is not the float it was read from. A
/// `DOUBLE`, a `DATETIME` and a `DECIMAL` are matched as they are shown.
#[tokio::test]
async fn a_key_that_cannot_be_matched_exactly_is_refused() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "write_inexact",
        &[
            "CREATE TABLE write_inexact (
                 id INT PRIMARY KEY,
                 at TIMESTAMP NOT NULL UNIQUE,
                 at_micro TIMESTAMP(6) NOT NULL UNIQUE,
                 bits BIT(8) NOT NULL UNIQUE,
                 ratio FLOAT NOT NULL UNIQUE,
                 share FLOAT UNSIGNED NOT NULL UNIQUE,
                 width DOUBLE NOT NULL UNIQUE,
                 seen DATETIME NOT NULL UNIQUE,
                 amount DECIMAL(6, 2) NOT NULL UNIQUE,
                 note VARCHAR(255)
             )",
            "INSERT INTO write_inexact VALUES
                 (1, '2026-10-25 00:30:00', '2026-10-25 00:30:00.123456', b'00110001', 0.1, 0.1,
                  0.1, '2026-10-25 02:30:00', 1.50, 'before')",
        ],
        async move {
            let (columns, row) = row_of(&connection, "write_inexact", 1).await;
            let held = |name: &str| {
                let index = columns.iter().position(|column| column == name).unwrap();
                (name.to_owned(), row[index].clone())
            };
            let note = |key: Vec<(String, Value)>, loaded: &str, new: &str| {
                changes_to(
                    "write_inexact",
                    vec![RowChange {
                        key,
                        set: one("note", VARCHAR, text(loaded), to(new)),
                    }],
                )
            };
            for (name, type_name) in [
                ("at", "timestamp"),
                ("at_micro", "timestamp"),
                ("bits", "bit"),
                ("ratio", "float"),
                ("share", "float"),
            ] {
                // Alone, and beside a column that is matched exactly.
                for key in [vec![held(name)], vec![held("id"), held(name)]] {
                    let outcome =
                        within(connection.write(&note(key, "before", "after"), &StopFlag::new()))
                            .await;
                    assert!(
                        matches!(
                            &outcome,
                            Err(Error::Query { message, .. })
                                if message.contains(&format!("{name} is a {type_name} column"))
                                    && message.contains("cannot be matched exactly")
                        ),
                        "{name}: {outcome:?}"
                    );
                    assert_eq!(row_of(&connection, "write_inexact", 1).await.1, row);
                }
            }
            let mut loaded = "before";
            for name in ["width", "seen", "amount"] {
                let outcome = within(
                    connection.write(&note(vec![held(name)], loaded, name), &StopFlag::new()),
                )
                .await;
                assert!(
                    matches!(outcome, Ok(WriteOutcome::Written { .. })),
                    "{name}: {outcome:?}"
                );
                let now = row_of(&connection, "write_inexact", 1).await.1;
                assert_eq!(now.last(), Some(&text(name)), "{name}");
                loaded = name;
            }
        },
    )
    .await;
}

#[tokio::test]
async fn a_boolean_goes_as_one_or_zero() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "write_flag",
        &[
            "CREATE TABLE write_flag (id INT PRIMARY KEY, active TINYINT(1) NOT NULL)",
            "INSERT INTO write_flag VALUES (1, 1)",
        ],
        async move {
            let mut admin = admin().await;
            for (typed, stored) in [("false", 0), ("true", 1), ("0", 0), ("7", 7)] {
                let outcome = save(
                    &connection,
                    "write_flag",
                    1,
                    &[("active", "tinyint(1)", to(typed))],
                )
                .await;
                assert!(
                    matches!(outcome, Ok(WriteOutcome::Written { .. })),
                    "{typed}: {outcome:?}"
                );
                let active: Option<i64> = admin
                    .query_first("SELECT active FROM write_flag WHERE id = 1")
                    .await
                    .unwrap();
                assert_eq!(active, Some(stored), "{typed}");
            }
            // What is neither is refused before the server hears of it.
            let outcome = save(
                &connection,
                "write_flag",
                1,
                &[("active", "tinyint(1)", to("maybe"))],
            )
            .await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Failed { row: 0, .. })),
                "{outcome:?}"
            );
        },
    )
    .await;
}

/// MySQL counts the rows an `UPDATE` changed, not the ones it found: a
/// value that is already there changes none, and that is no failure.
#[tokio::test]
async fn an_unchanged_value_is_not_a_failure() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let people = people("write_unchanged");
    on_its_own_tables("write_unchanged", &[&people[0], &people[1]], async move {
        let before = row_of(&connection, "write_unchanged", 1).await.1;
        let outcome = save(
            &connection,
            "write_unchanged",
            1,
            &[("name", VARCHAR, to("Ada"))],
        )
        .await;
        assert!(
            matches!(&outcome, Ok(WriteOutcome::Written { rows, .. }) if *rows == [before.clone()]),
            "{outcome:?}"
        );
    })
    .await;
}

/// One transaction is the promise, and an engine without transactions
/// cannot keep it: a statement that failed half way would leave the rows
/// before it written.
#[tokio::test]
async fn a_table_without_transactions_is_refused() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    let engines: Vec<String> = admin
        .query("SELECT engine FROM information_schema.engines WHERE support IN ('YES', 'DEFAULT')")
        .await
        .unwrap();
    if !engines.iter().any(|engine| engine == "MyISAM") {
        eprintln!("skipped: the server has no MyISAM");
        return;
    }
    on_its_own_tables(
        "write_myisam",
        &[
            "CREATE TABLE write_myisam (id INT PRIMARY KEY, name VARCHAR(255)) ENGINE = MyISAM",
            "INSERT INTO write_myisam VALUES (1, 'Ada')",
        ],
        async move {
            let outcome = save(
                &connection,
                "write_myisam",
                1,
                &[("name", VARCHAR, to("Grace"))],
            )
            .await;
            assert!(matches!(outcome, Err(Error::Unsupported(_))), "{outcome:?}");
            assert_eq!(
                row_of(&connection, "write_myisam", 1).await.1,
                [Value::Int(1), text("Ada")]
            );
        },
    )
    .await;
}

/// Where table names keep their letters, two tables can differ in them
/// alone, and the engine asked about must be the table's own: a save to
/// the one without transactions would not roll back.
#[tokio::test]
async fn a_table_is_judged_by_its_own_engine_not_a_namesakes() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    let engines: Vec<String> = admin
        .query("SELECT engine FROM information_schema.engines WHERE support IN ('YES', 'DEFAULT')")
        .await
        .unwrap();
    let keeps_letters: Option<u8> = admin
        .query_first("SELECT @@lower_case_table_names")
        .await
        .unwrap();
    if !engines.iter().any(|engine| engine == "MyISAM") || keeps_letters != Some(0) {
        eprintln!("skipped: the server has no MyISAM, or folds table names");
        return;
    }
    on_its_own_tables(
        "write_namesake, Write_Namesake",
        &[
            "CREATE TABLE write_namesake (id INT PRIMARY KEY, name VARCHAR(255)) ENGINE = MyISAM",
            "CREATE TABLE Write_Namesake (id INT PRIMARY KEY, name VARCHAR(255)) ENGINE = InnoDB",
            "INSERT INTO write_namesake VALUES (1, 'Ada')",
            "INSERT INTO Write_Namesake VALUES (1, 'Ada')",
        ],
        async move {
            let refused = save(
                &connection,
                "write_namesake",
                1,
                &[("name", VARCHAR, to("Grace"))],
            )
            .await;
            assert!(matches!(refused, Err(Error::Unsupported(_))), "{refused:?}");
            assert_eq!(
                row_of(&connection, "write_namesake", 1).await.1,
                [Value::Int(1), text("Ada")]
            );
            let written = save(
                &connection,
                "Write_Namesake",
                1,
                &[("name", VARCHAR, to("Grace"))],
            )
            .await;
            assert!(
                matches!(written, Ok(WriteOutcome::Written { .. })),
                "{written:?}"
            );
        },
    )
    .await;
}

/// Waits until a statement of another session holding `marker` is in
/// `state` on the server.
async fn waits_on_the_server(admin: &mut mysql_async::Conn, marker: &str, state: &str) {
    within(async {
        loop {
            let waiting: Option<i64> = admin
                .exec_first(
                    "SELECT count(*) FROM information_schema.processlist \
                     WHERE id <> CONNECTION_ID() AND info LIKE ? AND state LIKE ?",
                    (format!("%{marker}%"), format!("%{state}%")),
                )
                .await
                .unwrap();
            if waiting.unwrap_or(0) > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
}

/// The engine is asked for again once the save holds the table, and before
/// it writes: until then someone can still take the table's transactions
/// away. Here they do, in the one place a test can reach: they hold the
/// table when the save starts, so the save has passed its first check and
/// waits for the table at its first read, and they change the engine
/// before they let go.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_table_that_lost_its_transactions_while_the_save_waited_is_refused() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    let engines: Vec<String> = admin
        .query("SELECT engine FROM information_schema.engines WHERE support IN ('YES', 'DEFAULT')")
        .await
        .unwrap();
    if !engines.iter().any(|engine| engine == "MyISAM") {
        eprintln!("skipped: the server has no MyISAM");
        return;
    }
    on_its_own_tables(
        "write_altered",
        // No key longer than MyISAM takes.
        &[
            "CREATE TABLE write_altered (id INT PRIMARY KEY, name VARCHAR(50))",
            "INSERT INTO write_altered VALUES (1, 'Ada')",
        ],
        async move {
            let (columns, row) = row_of(&connection, "write_altered", 1).await;
            let changes = changes_to(
                "write_altered",
                vec![by_id(
                    1,
                    vec![cell(&columns, &row, "name", "varchar(50)", to("Grace"))],
                )],
            );
            let mut theirs = self::admin().await;
            theirs
                .query_drop("LOCK TABLES write_altered WRITE")
                .await
                .unwrap();
            let connection = std::sync::Arc::new(connection);
            let saving = {
                let connection = std::sync::Arc::clone(&connection);
                tokio::spawn(async move { connection.write(&changes, &StopFlag::new()).await })
            };
            waits_on_the_server(&mut admin, "write_altered", "metadata lock").await;
            theirs
                .query_drop("ALTER TABLE write_altered ENGINE = MyISAM")
                .await
                .unwrap();
            theirs.query_drop("UNLOCK TABLES").await.unwrap();
            let outcome = within(saving).await.unwrap();
            assert!(matches!(outcome, Err(Error::Unsupported(_))), "{outcome:?}");
            // Nothing was written: on this engine it could not have been undone.
            assert_eq!(row_of(&connection, "write_altered", 1).await.1, row);
            assert!(nothing_holds(&mut admin, "write_altered").await);
        },
    )
    .await;
}

/// A save of new rows alone reads no row, and still asks for the engine
/// only once it holds the table.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_save_of_new_rows_alone_holds_its_table_before_it_asks_for_the_engine() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    let engines: Vec<String> = admin
        .query("SELECT engine FROM information_schema.engines WHERE support IN ('YES', 'DEFAULT')")
        .await
        .unwrap();
    if !engines.iter().any(|engine| engine == "MyISAM") {
        eprintln!("skipped: the server has no MyISAM");
        return;
    }
    on_its_own_tables(
        "insert_altered",
        // No key longer than MyISAM takes.
        &[
            "CREATE TABLE insert_altered (id INT PRIMARY KEY, name VARCHAR(50))",
            "INSERT INTO insert_altered VALUES (1, 'Ada')",
        ],
        async move {
            let mut changes = changes_to("insert_altered", Vec::new());
            changes.inserts = vec![RowInsert {
                set: vec![sets("id", "int", "2"), sets("name", "varchar(50)", "Grace")],
            }];
            let mut theirs = self::admin().await;
            theirs
                .query_drop("LOCK TABLES insert_altered WRITE")
                .await
                .unwrap();
            let connection = std::sync::Arc::new(connection);
            let saving = {
                let connection = std::sync::Arc::clone(&connection);
                tokio::spawn(async move { connection.write(&changes, &StopFlag::new()).await })
            };
            waits_on_the_server(&mut admin, "insert_altered", "metadata lock").await;
            theirs
                .query_drop("ALTER TABLE insert_altered ENGINE = MyISAM")
                .await
                .unwrap();
            theirs.query_drop("UNLOCK TABLES").await.unwrap();
            let outcome = within(saving).await.unwrap();
            assert!(matches!(outcome, Err(Error::Unsupported(_))), "{outcome:?}");
            // Nothing was written: on this engine it could not have been undone.
            assert_eq!(page_of(&connection, "insert_altered").await.1.len(), 1);
            assert!(nothing_holds(&mut admin, "insert_altered").await);
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
        "write_twins",
        &[
            "CREATE TABLE write_twins (id INT, name VARCHAR(255))",
            "INSERT INTO write_twins VALUES (1, 'same'), (1, 'same'), (2, 'other')",
        ],
        async move {
            let changes = changes_to(
                "write_twins",
                vec![by_id(1, one("name", VARCHAR, text("same"), to("Mine")))],
            );
            let outcome = within(connection.write(&changes, &StopFlag::new())).await;
            assert!(
                matches!(&outcome, Err(Error::Query { message, .. }) if message.contains("more than one row")),
                "{outcome:?}"
            );
            let names: Vec<String> = admin()
                .await
                .query("SELECT name FROM write_twins ORDER BY id")
                .await
                .unwrap();
            assert_eq!(names, ["same", "same", "other"]);
        },
    )
    .await;
}

/// The changes to the row `column` finds by `key`.
fn keyed(column: &str, key: Value, set: Vec<CellChange>) -> RowChange {
    RowChange {
        key: vec![(column.into(), key)],
        set,
    }
}

fn names_the_same_row(outcome: &tabletist_db::Result<WriteOutcome>) -> bool {
    matches!(outcome, Err(Error::Query { message, .. }) if message.contains("name the same row"))
}

/// The database decides which row a key finds. MySQL reads the text `'1'`
/// as a number beside an integer column, and finds the row of 1; a row has
/// two unique columns; `0.0` and `-0.0` are one double. Each pair passes
/// `check`, which compares the keys as values, and each would read the row
/// as it was twice and write the second change over the first.
#[tokio::test]
async fn changes_that_read_the_same_row_are_refused_and_nothing_is_written() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let [create, fill] = people("write_same_row");
    on_its_own_tables(
        "write_same_row, write_same_zero",
        &[
            &create,
            &fill,
            "CREATE TABLE write_same_zero (k DOUBLE PRIMARY KEY, name VARCHAR(255), note VARCHAR(255))",
            "INSERT INTO write_same_zero VALUES (0, 'zero', 'old'), (1.5, 'more', 'old')",
        ],
        async move {
            let (columns, people) = page_of(&connection, "write_same_row").await;
            let change = |row: usize, column: &str, type_name: &str, new: &str| {
                vec![cell(&columns, &people[row], column, type_name, to(new))]
            };
            for (column, key) in [("id", text("1")), ("email", text("a@x"))] {
                let changes = changes_to(
                    "write_same_row",
                    vec![
                        by_id(1, change(0, "name", VARCHAR, "Mine")),
                        keyed(column, key.clone(), change(0, "score", "double", "9")),
                    ],
                );
                assert_eq!(changes.check(), Ok(()));
                let outcome = within(connection.write(&changes, &StopFlag::new())).await;
                assert!(names_the_same_row(&outcome), "{column} {key:?}: {outcome:?}");
                assert_eq!(page_of(&connection, "write_same_row").await.1, people);
            }

            let (_, zeros) = page_of(&connection, "write_same_zero").await;
            assert_eq!(zeros[0][0], Value::Float(0.0));
            let changes = changes_to(
                "write_same_zero",
                vec![
                    keyed(
                        "k",
                        Value::Float(0.0),
                        one("name", VARCHAR, text("zero"), to("Mine")),
                    ),
                    keyed(
                        "k",
                        Value::Float(-0.0),
                        one("note", VARCHAR, text("old"), to("mine")),
                    ),
                ],
            );
            assert_eq!(changes.check(), Ok(()));
            let outcome = within(connection.write(&changes, &StopFlag::new())).await;
            assert!(names_the_same_row(&outcome), "{outcome:?}");
            assert_eq!(page_of(&connection, "write_same_zero").await.1, zeros);
            // Undone, and its locks with it.
            assert!(nothing_holds(&mut admin().await, "write_same_row").await);
            assert!(nothing_holds(&mut admin().await, "write_same_zero").await);

            // Two rows are two rows, however their keys are spelled.
            let changes = changes_to(
                "write_same_row",
                vec![
                    by_id(1, change(0, "name", VARCHAR, "Mine")),
                    keyed("id", text("2"), change(1, "name", VARCHAR, "Theirs")),
                    keyed("email", text("c@x"), change(2, "name", VARCHAR, "Third")),
                ],
            );
            let outcome = within(connection.write(&changes, &StopFlag::new())).await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Written { .. })),
                "{outcome:?}"
            );
            let names: Vec<Value> = page_of(&connection, "write_same_row")
                .await
                .1
                .into_iter()
                .map(|row| row[2].clone())
                .collect();
            assert_eq!(names, [text("Mine"), text("Theirs"), text("Third")]);
        },
    )
    .await;
}

/// A table whose rows a test finds by `k`, the length of `name`, which the
/// database works out: a change of `name` moves the row to another key.
/// Nothing makes `k` unique. (A trigger would do as well; the test
/// server's user may not make one.)
const KEYED_BY_LENGTH: [&str; 2] = [
    "(name VARCHAR(10), note VARCHAR(10), k INT AS (CHAR_LENGTH(name)) VIRTUAL)",
    "(name, note) VALUES ('a', 'first'), ('bb', 'second')",
];

fn keyed_by_length(table: &str) -> [String; 2] {
    [
        format!("CREATE TABLE {table} {}", KEYED_BY_LENGTH[0]),
        format!("INSERT INTO {table} {}", KEYED_BY_LENGTH[1]),
    ]
}

/// The row whose `k` is this, given `column` anew.
fn by_length(length: i64, column: &str, loaded: &str, new: &str) -> RowChange {
    RowChange {
        key: vec![("k".into(), Value::Int(length))],
        set: one(column, "varchar(10)", text(loaded), to(new)),
    }
}

/// The names and notes of a table keyed by length, by name.
async fn names_and_notes(table: &str) -> Vec<(String, String)> {
    admin()
        .await
        .query(format!("SELECT name, note FROM {table} ORDER BY name"))
        .await
        .unwrap()
}

/// An `UPDATE` that changes more than its one row is not a save. Here the
/// row before it took its key, so the same statement finds both.
#[tokio::test]
async fn an_update_that_changes_more_than_one_row_fails_and_is_undone() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let table = keyed_by_length("write_spread");
    on_its_own_tables("write_spread", &[&table[0], &table[1]], async move {
        let changes = changes_to(
            "write_spread",
            vec![
                // Its length becomes two, the key of the row after it.
                by_length(1, "name", "a", "cc"),
                by_length(2, "note", "second", "both"),
            ],
        );
        let outcome = within(connection.write(&changes, &StopFlag::new()))
            .await
            .unwrap();
        assert!(
            matches!(
                &outcome,
                WriteOutcome::Failed { row: 1, error: Error::Query { message, .. } }
                    if message.contains("2 rows")
            ),
            "{outcome:?}"
        );
        assert_eq!(
            names_and_notes("write_spread").await,
            [
                ("a".to_owned(), "first".to_owned()),
                ("bb".to_owned(), "second".to_owned())
            ]
        );
    })
    .await;
}

/// After its updates a save reads each row back by its key, and that must
/// find the one row again. A change can give a second row the key, or take
/// the key from the row: then there is no one row to hand back, and the
/// save is undone.
#[tokio::test]
async fn a_row_that_cannot_be_read_back_alone_is_an_error_and_is_undone() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let table = keyed_by_length("write_moved");
    on_its_own_tables("write_moved", &[&table[0], &table[1]], async move {
        for (changes, said) in [
            (
                // The second change gives its row the first one's key,
                // after the first was written.
                vec![
                    by_length(2, "note", "second", "mine"),
                    by_length(1, "name", "a", "cc"),
                ],
                "more than one row",
            ),
            // Its length becomes three: no row has the key any more.
            (
                vec![by_length(1, "name", "a", "ccc")],
                "could not be read back",
            ),
        ] {
            let outcome =
                within(connection.write(&changes_to("write_moved", changes), &StopFlag::new()))
                    .await;
            assert!(
                matches!(&outcome, Err(Error::Query { message, .. }) if message.contains(said)),
                "{said}: {outcome:?}"
            );
            assert_eq!(
                names_and_notes("write_moved").await,
                [
                    ("a".to_owned(), "first".to_owned()),
                    ("bb".to_owned(), "second".to_owned())
                ],
                "{said}"
            );
        }
    })
    .await;
}

/// MySQL matches a column's name without regard to case, and the checks of
/// a set compare names exactly: `ID` in the set and `id` in the key would
/// pass them and change the row's own key. A save takes a name only as the
/// table spells it.
#[tokio::test]
async fn a_name_in_another_case_cannot_change_the_rows_own_key() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let people = people("write_case");
    on_its_own_tables("write_case", &[&people[0], &people[1]], async move {
        // The key's own column under another spelling is refused on the
        // names alone, before anything is sent: here there is not even a
        // table to ask.
        let nowhere = changes_to(
            "write_case_nowhere",
            vec![by_id(1, one("ID", "int", Value::Int(1), to("7")))],
        );
        let outcome = within(connection.write(&nowhere, &StopFlag::new())).await;
        assert!(
            matches!(&outcome, Err(Error::Query { message, .. }) if message.contains("part of the row's key")),
            "{outcome:?}"
        );
        let before = page_of(&connection, "write_case").await.1;
        for (key, column, loaded) in [
            // The key's own column, under another spelling.
            ("id", "ID", Value::Int(1)),
            ("ID", "id", Value::Int(1)),
            // Another column, but a key that is not spelled as the table
            // spells it: the save cannot tell it from a column of the set.
            ("ID", "name", text("Ada")),
            ("id", "NAME", text("Ada")),
        ] {
            let changes = changes_to(
                "write_case",
                vec![RowChange {
                    key: vec![(key.into(), Value::Int(1))],
                    set: one(column, "int", loaded, to("7")),
                }],
            );
            let outcome = within(connection.write(&changes, &StopFlag::new())).await;
            assert!(
                matches!(outcome, Err(Error::Query { .. })),
                "{key} {column}: {outcome:?}"
            );
            assert_eq!(
                page_of(&connection, "write_case").await.1,
                before,
                "{key} {column}"
            );
        }
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
    let people = people("write_locked");
    on_its_own_tables("write_locked", &[&people[0], &people[1]], async move {
        let (columns, row) = row_of(&connection, "write_locked", 1).await;
        let changes = changes_to(
            "write_locked",
            vec![by_id(
                1,
                vec![cell(&columns, &row, "name", VARCHAR, to("Mine"))],
            )],
        );
        let mut theirs = admin().await;
        theirs.query_drop("START TRANSACTION").await.unwrap();
        theirs
            .query_drop("UPDATE write_locked SET name = 'Theirs' WHERE id = 1")
            .await
            .unwrap();
        let connection = std::sync::Arc::new(connection);
        let saving = {
            let connection = std::sync::Arc::clone(&connection);
            tokio::spawn(async move { connection.write(&changes, &StopFlag::new()).await })
        };
        // Until a statement of the save has waited a while: the read, with
        // its lock. Without it the UPDATE is what waits, and the outcome
        // below is another.
        let mut admin = admin().await;
        waits_on_the_server(&mut admin, "write_locked", "").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(!saving.is_finished(), "the save did not wait");
        theirs.query_drop("COMMIT").await.unwrap();
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
    })
    .await;
}

/// A cancel in the middle of a save, after a row of it was written: the
/// save ends as cancelled and nothing of it stays.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancel_during_a_save_undoes_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let people = people("write_cancel");
    on_its_own_tables("write_cancel", &[&people[0], &people[1]], async move {
        let mut admin = admin().await;
        let (columns, first) = row_of(&connection, "write_cancel", 1).await;
        let (_, second) = row_of(&connection, "write_cancel", 2).await;
        let changes = changes_to(
            "write_cancel",
            vec![
                by_id(
                    1,
                    vec![cell(&columns, &first, "name", VARCHAR, to("Written first"))],
                ),
                by_id(
                    2,
                    vec![cell(&columns, &second, "email", VARCHAR, to("new@x"))],
                ),
            ],
        );
        // Someone is about to take that address, and has not committed:
        // the second UPDATE waits to learn whether it is a duplicate.
        let mut theirs = self::admin().await;
        theirs.query_drop("START TRANSACTION").await.unwrap();
        theirs
            .query_drop("INSERT INTO write_cancel VALUES (9, 'new@x', 'Theirs', NULL)")
            .await
            .unwrap();
        let cancel = connection.cancel_handle();
        let connection = std::sync::Arc::new(connection);
        let saving = {
            let connection = std::sync::Arc::clone(&connection);
            tokio::spawn(async move { connection.write(&changes, &StopFlag::new()).await })
        };
        runs_on_the_server(&mut admin, "UPDATE `tabletist`.`write_cancel` SET `email`").await;
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
        theirs.query_drop("ROLLBACK").await.unwrap();
        // Nothing was written, the row before the cancelled one included.
        assert_eq!(row_of(&connection, "write_cancel", 1).await.1, first);
        assert_eq!(row_of(&connection, "write_cancel", 2).await.1, second);
        assert!(nothing_holds(&mut admin, "write_cancel").await);
        assert!(writes_between_scripts(&connection).await);
        // And the session saves as before.
        let outcome = save(
            &connection,
            "write_cancel",
            1,
            &[("name", VARCHAR, to("Quick"))],
        )
        .await;
        assert!(
            matches!(outcome, Ok(WriteOutcome::Written { .. })),
            "{outcome:?}"
        );
    })
    .await;
}

/// The names of `table`'s people, in key order, read by another session.
async fn names_of(table: &str) -> Vec<String> {
    admin()
        .await
        .query(format!("SELECT name FROM {table} ORDER BY id"))
        .await
        .unwrap()
}

/// A save told to stop before it began sends nothing, and ends as a
/// cancelled one does.
#[tokio::test]
async fn a_save_stopped_before_it_starts_writes_nothing() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let people = people("write_stopped_early");
    on_its_own_tables(
        "write_stopped_early",
        &[&people[0], &people[1]],
        async move {
            let (columns, first) = row_of(&connection, "write_stopped_early", 1).await;
            let changes = changes_to(
                "write_stopped_early",
                vec![by_id(
                    1,
                    vec![cell(&columns, &first, "name", VARCHAR, to("Mine"))],
                )],
            );
            let stop = StopFlag::new();
            stop.stop();
            let outcome = within(connection.write(&changes, &stop)).await;
            assert_eq!(outcome, Err(Error::Cancelled));
            assert_eq!(names_of("write_stopped_early").await, ["Ada", "Bea", "Cy"]);
            assert!(nothing_holds(&mut admin().await, "write_stopped_early").await);
            assert!(writes_between_scripts(&connection).await);
            // And the session saves as before.
            let outcome = within(connection.write(&changes, &StopFlag::new())).await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Written { .. })),
                "{outcome:?}"
            );
            assert_eq!(names_of("write_stopped_early").await, ["Mine", "Bea", "Cy"]);
        },
    )
    .await;
}

/// `KILL QUERY` stops a statement that is running, and does nothing between
/// two. Here the save is told to stop while its second read waits for a row
/// someone else holds, and nothing is killed: they let go, the read comes
/// back with the row as the page loaded it, and the save ends there, before
/// its first `UPDATE`, as a cancelled one does.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stop_between_two_statements_of_a_save_undoes_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let people = people("write_stopped");
    on_its_own_tables("write_stopped", &[&people[0], &people[1]], async move {
        let (columns, rows) = page_of(&connection, "write_stopped").await;
        let name =
            |row: usize, new: &str| vec![cell(&columns, &rows[row], "name", VARCHAR, to(new))];
        // The second row first: its read is over when the first row's
        // waits.
        let changes = changes_to(
            "write_stopped",
            vec![by_id(2, name(1, "Second")), by_id(1, name(0, "First"))],
        );
        let mut theirs = admin().await;
        theirs.query_drop("START TRANSACTION").await.unwrap();
        theirs
            .query_drop("SELECT 1 FROM write_stopped WHERE id = 1 FOR UPDATE")
            .await
            .unwrap();
        let stop = StopFlag::new();
        let connection = std::sync::Arc::new(connection);
        let saving = {
            let connection = std::sync::Arc::clone(&connection);
            let changes = changes.clone();
            let stop = stop.clone();
            tokio::spawn(async move { connection.write(&changes, &stop).await })
        };
        // Until a statement of the save has waited a while: the read of
        // the row they hold. The one before it waited for nothing.
        let mut admin = admin().await;
        waits_on_the_server(&mut admin, "write_stopped", "").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(!saving.is_finished(), "the save did not wait");
        stop.stop();
        theirs.query_drop("ROLLBACK").await.unwrap();
        // The save's own end, though no statement of it was killed.
        assert_eq!(within(saving).await.unwrap(), Err(Error::Cancelled));
        assert_eq!(names_of("write_stopped").await, ["Ada", "Bea", "Cy"]);
        assert!(nothing_holds(&mut admin, "write_stopped").await);
        assert!(writes_between_scripts(&connection).await);
        // And the session saves as before.
        let outcome = within(connection.write(&changes, &StopFlag::new())).await;
        assert!(
            matches!(outcome, Ok(WriteOutcome::Written { .. })),
            "{outcome:?}"
        );
        assert_eq!(names_of("write_stopped").await, ["First", "Second", "Cy"]);
    })
    .await;
}

/// Text is bound, never written into the statement: quotes, backslashes,
/// a colon the driver could take for a parameter, and letters outside
/// ASCII are stored as they are, in a value and in the key that finds the
/// row.
#[tokio::test]
async fn text_is_stored_exactly() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    const AWKWARD: [&str; 9] = [
        "O'Brien",
        r"C:\temp\new",
        r"a backslash before a quote \' and after '\",
        "naïve café, 漢字, 🙂",
        r#"both "double" and `back` and 'single'"#,
        ":name and ? and -':x'",
        r"\",
        "a line\nand a\ttab",
        "50% off_now",
    ];
    on_its_own_tables(
        "write_text",
        &["CREATE TABLE write_text (
               id VARCHAR(100) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin PRIMARY KEY,
               body TEXT CHARACTER SET utf8mb4
           )"],
        async move {
            let mut admin = admin().await;
            for awkward in AWKWARD {
                admin
                    .exec_drop("INSERT INTO write_text VALUES (?, 'before')", (awkward,))
                    .await
                    .unwrap();
            }
            let rows = AWKWARD
                .iter()
                .map(|awkward| RowChange {
                    key: vec![("id".into(), text(awkward))],
                    set: one("body", "text", text("before"), to(awkward)),
                })
                .collect();
            let outcome =
                within(connection.write(&changes_to("write_text", rows), &StopFlag::new())).await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Written { .. })),
                "{outcome:?}"
            );
            let stored: Vec<(String, String)> = admin
                .query("SELECT id, body FROM write_text")
                .await
                .unwrap();
            assert_eq!(stored.len(), AWKWARD.len());
            for (id, body) in stored {
                assert_eq!(body, id);
                assert!(AWKWARD.contains(&id.as_str()), "{id}");
            }
        },
    )
    .await;
}

/// What Review SQL shows and copies, run by hand as another client would
/// send it, stores the bytes the save's bound statement stores: a NUL is
/// written as its escape, so no raw NUL stands in the text, and beside a
/// backslash and a quote it is still the value.
#[tokio::test]
async fn the_shown_statement_stores_what_the_bound_one_does() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    const AWKWARD: [&str; 5] = [
        "nul\0 a back\\slash and it's",
        "\0",
        "\\\0'\0\\",
        // As PHP serializes a private property.
        "a:1:{s:6:\"\0A\0key\";s:3:\"it's\";}",
        r"\0 typed out is no NUL",
    ];
    on_its_own_tables(
        "write_shown",
        &["CREATE TABLE write_shown (
               id VARCHAR(100) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin,
               how VARCHAR(10) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin,
               body TEXT CHARACTER SET utf8mb4,
               PRIMARY KEY (id, how)
           )"],
        async move {
            let mut admin = admin().await;
            let object = ObjectRef::new("tabletist", "write_shown");
            for awkward in AWKWARD {
                for how in ["hand", "bound"] {
                    admin
                        .exec_drop(
                            "INSERT INTO write_shown VALUES (?, ?, 'before')",
                            (awkward, how),
                        )
                        .await
                        .unwrap();
                }
                // The key holds the value too: the shown `WHERE` finds its
                // row by it.
                let change = |how: &str| RowChange {
                    key: vec![("id".into(), text(awkward)), ("how".into(), text(how))],
                    set: one("body", "text", text("before"), to(awkward)),
                };
                let update = Dialect::MySql.update_row(&object, &change("hand")).unwrap();
                assert!(!update.shown.contains('\0'), "{:?}", update.shown);
                admin.query_drop(&update.shown).await.unwrap();
                assert_eq!(admin.affected_rows(), 1, "{:?}", update.shown);
                let changes = changes_to("write_shown", vec![change("bound")]);
                let outcome = within(connection.write(&changes, &StopFlag::new())).await;
                assert!(
                    matches!(outcome, Ok(WriteOutcome::Written { .. })),
                    "{awkward:?}: {outcome:?}"
                );
            }
            let stored: Vec<(Vec<u8>, String, Vec<u8>)> = admin
                .query("SELECT id, how, body FROM write_shown ORDER BY id, how")
                .await
                .unwrap();
            assert_eq!(stored.len(), 2 * AWKWARD.len());
            for (id, how, body) in stored {
                assert_eq!(body, id, "{how}: {:?}", String::from_utf8_lossy(&id));
                assert!(AWKWARD.iter().any(|awkward| awkward.as_bytes() == id));
            }
        },
    )
    .await;
}
