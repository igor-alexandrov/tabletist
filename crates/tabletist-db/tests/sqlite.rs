//! The SQLite adapter through the public `Connection` API, on the shared fixture.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use tabletist_db::{
    Access, ConnectSpec, Connection, Dialect, Driver, Error, Filter, FilterOp, HostKeys, ObjectRef,
    RowQuery, Secrets, Sort, SortDir, StatementOutcome, StopFlag, Value, ValueKind,
};

async fn fixture_as(access: Access) -> (Connection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    tabletist_db::fixtures::write_sqlite_demo(&path).unwrap();
    let connection = Connection::connect_with(
        &ConnectSpec::sqlite(&path),
        &Secrets::default(),
        &HostKeys::default(),
        access,
    )
    .await
    .unwrap();
    (connection, dir)
}

async fn fixture() -> (Connection, tempfile::TempDir) {
    fixture_as(Access::ReadOnly).await
}

#[tokio::test]
async fn a_connection_knows_the_access_it_was_opened_with() {
    let (connection, _dir) = fixture().await;
    assert_eq!(connection.access(), Access::ReadOnly);
    let (writable, _dir) = fixture_as(Access::Writable).await;
    assert_eq!(writable.access(), Access::Writable);
}

fn users(limit: u32) -> RowQuery {
    RowQuery::new(ObjectRef::new("main", "users"), limit)
}

fn ids(page: &tabletist_db::RowPage) -> Vec<i64> {
    page.rows
        .iter()
        .map(|row| match row[0] {
            Value::Int(id) => id,
            ref other => panic!("id is {other:?}"),
        })
        .collect()
}

#[tokio::test]
async fn connections_report_their_driver_and_have_no_database_list() {
    let (connection, _dir) = fixture().await;
    assert_eq!(connection.driver(), Driver::Sqlite);
    assert!(connection.list_databases().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_sqlite_spec_without_a_path_is_invalid() {
    let spec = ConnectSpec {
        driver: Driver::Sqlite,
        ..ConnectSpec::default()
    };
    assert!(matches!(
        Connection::connect(&spec, &Secrets::default()).await,
        Err(Error::InvalidSpec(_))
    ));
}

#[tokio::test]
async fn the_first_page_is_ordered_by_key_with_column_metadata() {
    let (connection, _dir) = fixture().await;
    let page = connection.fetch_rows(&users(3)).await.unwrap();
    assert_eq!(ids(&page), vec![1, 2, 3]);
    assert!(page.has_more);
    assert!(page.ordered_by_key);
    let columns: Vec<(&str, ValueKind)> = page
        .columns
        .iter()
        .map(|column| (column.name.as_str(), column.kind))
        .collect();
    assert_eq!(
        columns,
        vec![
            ("id", ValueKind::Numeric),
            ("email", ValueKind::Text),
            ("name", ValueKind::Text),
            ("created_at", ValueKind::Temporal),
            ("active", ValueKind::Bool),
            ("meta", ValueKind::Json),
            ("avatar", ValueKind::Binary),
            ("score", ValueKind::Numeric),
        ]
    );
}

#[tokio::test]
async fn paging_moves_through_the_table_and_the_last_page_has_no_more() {
    let (connection, _dir) = fixture().await;
    let mut query = users(2);
    query.offset = 4;
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(ids(&page), vec![5]);
    assert!(!page.has_more);
}

#[tokio::test]
async fn a_user_sort_orders_rows() {
    let (connection, _dir) = fixture().await;
    let mut query = users(5);
    query.sort = vec![Sort {
        column: "email".into(),
        dir: SortDir::Desc,
    }];
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(ids(&page), vec![3, 4, 5, 2, 1]);
}

#[tokio::test]
async fn filters_narrow_rows_and_counts_agree() {
    let (connection, _dir) = fixture().await;
    let mut query = users(50);
    query.filters = vec![Filter {
        column: "name".into(),
        op: FilterOp::IsNull,
        value: String::new(),
    }];
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![5]);
    assert_eq!(connection.count_rows(&query).await.unwrap(), 1);

    query.filters = vec![Filter {
        column: "id".into(),
        op: FilterOp::In,
        value: "1, 3".into(),
    }];
    assert_eq!(
        ids(&connection.fetch_rows(&query).await.unwrap()),
        vec![1, 3]
    );

    query.filters = vec![Filter {
        column: "id".into(),
        op: FilterOp::Ge,
        value: "4".into(),
    }];
    assert_eq!(
        ids(&connection.fetch_rows(&query).await.unwrap()),
        vec![4, 5]
    );
}

#[tokio::test]
async fn contains_is_case_insensitive_and_handles_unicode() {
    let (connection, _dir) = fixture().await;
    let mut query = users(50);
    // SQLite's LIKE folds ASCII letters only; non-ASCII text matches exactly.
    query.filters = vec![Filter {
        column: "name".into(),
        op: FilterOp::Contains,
        value: "ZO".into(),
    }];
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![3]);
    query.filters[0].value = "ë 🚀".into();
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![3]);
}

#[tokio::test]
async fn contains_matches_literal_percent_signs() {
    let (connection, _dir) = fixture().await;
    let mut query = users(50);
    query.filters = vec![Filter {
        column: "name".into(),
        op: FilterOp::Contains,
        value: "50%".into(),
    }];
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![4]);
    query.filters[0].value = "%".into();
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![4]);
}

/// The session's `query_only`, as a script reads it.
async fn query_only(connection: &Connection) -> Value {
    let outcome = run(connection, "PRAGMA query_only").await.unwrap();
    match &outcome.results[0].outcome {
        StatementOutcome::Rows { rows, .. } => rows[0][0].clone(),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_raw_where_cannot_modify_data() {
    for access in [Access::ReadOnly, Access::Writable] {
        let (connection, _dir) = fixture_as(access).await;
        let mut query = users(50);
        // Each chains a second statement: a plain one, one that closes the
        // parenthesis first, and a pragma SQLite applies as soon as it is
        // prepared (rusqlite prepares what follows the first statement, and
        // only then refuses the text). The `;` is refused before that.
        for raw in [
            "1 = 1; DELETE FROM users",
            "1=1) ; DELETE FROM users; SELECT (1",
            "1=1); PRAGMA query_only = 0; SELECT (1",
        ] {
            query.raw_where = Some(raw.into());
            assert!(
                connection.fetch_rows(&query).await.is_err(),
                "{access:?} {raw}"
            );
            assert_eq!(query_only(&connection).await, Value::Int(1), "{raw}");
            assert!(
                connection.count_rows(&query).await.is_err(),
                "{access:?} {raw}"
            );
            assert_eq!(query_only(&connection).await, Value::Int(1), "{raw}");
        }
        assert_eq!(connection.count_rows(&users(50)).await.unwrap(), 5);
    }
}

#[tokio::test]
async fn a_raw_where_cannot_drop_the_page_limit() {
    let (connection, _dir) = fixture().await;
    let mut query = RowQuery::new(ObjectRef::new("main", "big"), 10);
    // An unterminated comment would swallow the builder's ORDER BY, LIMIT
    // and OFFSET, so the second page would repeat the first.
    for raw in [
        "1=1) /*",
        "1=1) --",
        "1=1 /* note",
        "1=1) /* ",
        "label <> '*/' /*",
    ] {
        query.raw_where = Some(raw.into());
        for offset in [0, 10] {
            query.offset = offset;
            if let Ok(page) = connection.fetch_rows(&query).await {
                let first = offset as i64 + 1;
                assert_eq!(
                    ids(&page),
                    (first..first + 10).collect::<Vec<_>>(),
                    "{raw} at offset {offset}"
                );
            }
        }
    }
    query.offset = 10;
    query.raw_where = Some("1=1) /*".into());
    assert!(matches!(
        connection.fetch_rows(&query).await,
        Err(Error::Query { .. })
    ));
    assert!(connection.count_rows(&query).await.is_err());
    // A closed comment and comment markers in strings are fine.
    query.raw_where = Some("label <> '/*' /* note */".into());
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap())[0], 11);
}

#[tokio::test]
async fn a_raw_where_cannot_end_the_statement_at_a_nul() {
    let (connection, _dir) = fixture().await;
    // SQLite stops reading at a NUL, and rusqlite takes what follows for
    // an empty second statement: the page would lose its ORDER BY, LIMIT
    // and OFFSET, and the second page would repeat the first.
    let mut query = users(2);
    query.offset = 2;
    query.raw_where = Some("1=1) \0".into());
    let page = connection.fetch_rows(&query).await;
    assert!(
        matches!(&page, Err(Error::Query { message, .. }) if message.contains("NUL")),
        "{:?}",
        page.as_ref().map(ids)
    );
    assert!(connection.count_rows(&query).await.is_err());
}

#[tokio::test]
async fn a_filter_on_a_missing_column_fails_instead_of_matching_everything() {
    let (connection, _dir) = fixture().await;
    // Legacy SQLite read "renamed" as the string 'renamed' when no column
    // has that name, so `"renamed" <> 'x'` matched every row.
    let mut query = users(50);
    query.filters = vec![Filter {
        column: "renamed".into(),
        op: FilterOp::Ne,
        value: "x".into(),
    }];
    assert!(matches!(
        connection.fetch_rows(&query).await,
        Err(Error::Query { .. })
    ));
    assert!(matches!(
        connection.count_rows(&query).await,
        Err(Error::Query { .. })
    ));
    let mut query = users(50);
    query.raw_where = Some(r#"name = "Ada Lovelace""#.into());
    assert!(connection.fetch_rows(&query).await.is_err());
}

#[tokio::test]
async fn the_schema_is_untrusted() {
    let (connection, _dir) = fixture().await;
    let mut query = users(50);
    query.raw_where = Some("(SELECT trusted_schema FROM pragma_trusted_schema) = 0".into());
    assert_eq!(connection.fetch_rows(&query).await.unwrap().rows.len(), 5);
}

#[tokio::test]
async fn object_listings_are_capped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many.db");
    {
        let mut setup = rusqlite::Connection::open(&path).unwrap();
        let transaction = setup.transaction().unwrap();
        for index in 0..tabletist_db::MAX_LISTED + 5 {
            transaction
                .execute_batch(&format!("CREATE TABLE t{index:05} (id INTEGER);"))
                .unwrap();
        }
        transaction.commit().unwrap();
    }
    let connection = Connection::connect(&ConnectSpec::sqlite(&path), &Secrets::default())
        .await
        .unwrap();
    let objects = connection.list_objects("main").await.unwrap();
    assert_eq!(objects.len(), tabletist_db::MAX_LISTED as usize);
    assert_eq!(objects[0].name, "t00000");
}

#[tokio::test]
async fn a_bad_raw_where_is_a_query_error() {
    let (connection, _dir) = fixture().await;
    let mut query = users(50);
    query.raw_where = Some("no_such_column = 1".into());
    assert!(matches!(
        connection.fetch_rows(&query).await,
        Err(Error::Query { .. })
    ));
}

#[tokio::test]
async fn blobs_and_invalid_utf8_load_without_failing() {
    let (connection, _dir) = fixture().await;
    let page = connection.fetch_rows(&users(50)).await.unwrap();
    assert_eq!(
        page.rows[0][6],
        Value::Bytes(vec![0x89, 0x50, 0x4E, 0x47].into())
    );
    assert_eq!(page.rows[1][6], Value::Null);
    match &page.rows[4][6] {
        Value::Text(text) => assert!(text.contains('\u{FFFD}'), "{text:?}"),
        other => panic!("invalid UTF-8 text should load as lossy text, got {other:?}"),
    }
    assert_eq!(page.rows[2][2], Value::Text("Zoë 🚀".into()));
    assert_eq!(page.rows[0][7], Value::Float(99.5));
}

#[tokio::test]
async fn keyless_tables_page_without_a_stable_order() {
    let (connection, _dir) = fixture().await;
    let page = connection
        .fetch_rows(&RowQuery::new(ObjectRef::new("main", "events"), 50))
        .await
        .unwrap();
    assert_eq!(page.rows.len(), 3);
    assert!(!page.ordered_by_key);
}

#[tokio::test]
async fn a_table_with_a_quoted_name_can_be_browsed() {
    let (connection, _dir) = fixture().await;
    let object = ObjectRef::new("main", "weird \"name\"");
    let mut query = RowQuery::new(object.clone(), 50);
    query.sort = vec![Sort {
        column: "select".into(),
        dir: SortDir::Asc,
    }];
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(page.columns[0].name, "col with space");
    assert_eq!(page.rows[0][0], Value::Text("quoted".into()));
    assert_eq!(connection.describe(&object).await.unwrap().columns.len(), 2);
}

#[tokio::test]
async fn big_tables_page_deep_quickly() {
    let (connection, _dir) = fixture().await;
    let mut query = RowQuery::new(ObjectRef::new("main", "big"), 300);
    query.offset = 99_900;
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(page.rows.len(), 100);
    assert!(!page.has_more);
    assert_eq!(
        connection
            .count_rows(&RowQuery::new(ObjectRef::new("main", "big"), 300))
            .await
            .unwrap(),
        100_000
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_running_query_can_be_cancelled() {
    let (connection, _dir) = fixture().await;
    let cancel = connection.cancel_handle();
    let connection = std::sync::Arc::new(connection);
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        tokio::spawn(async move {
            let mut query = RowQuery::new(ObjectRef::new("main", "big"), 10);
            // Ten billion row pairs: far longer than the test waits.
            query.raw_where = Some("(SELECT count(*) FROM big a, big b) > 0".into());
            connection.count_rows(&query).await
        })
    };
    // SQLite ignores an interrupt when nothing runs yet, so keep cancelling
    // until the query stops. A single early cancel would leave the blocking
    // job running and hang the test runtime on shutdown.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !running.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "cancel must stop the query"
        );
        cancel.cancel().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let result = running.await.unwrap();
    assert_eq!(result, Err(Error::Cancelled));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_abandoned_query_does_not_block_the_next_one() {
    let (connection, _dir) = fixture().await;
    let connection = std::sync::Arc::new(connection);
    let slow = {
        let connection = std::sync::Arc::clone(&connection);
        tokio::spawn(async move {
            let mut query = RowQuery::new(ObjectRef::new("main", "big"), 10);
            query.raw_where = Some("(SELECT count(*) FROM big a, big b) > 0".into());
            connection.count_rows(&query).await
        })
    };
    tokio::time::sleep(Duration::from_millis(300)).await;
    // Closing a tab or replacing a query drops its future.
    slow.abort();
    let next = tokio::time::timeout(Duration::from_secs(5), connection.fetch_rows(&users(5))).await;
    let finished = matches!(next, Ok(Ok(_)));
    if !finished {
        // Stop the runaway job so the test runtime can shut down.
        connection.cancel_handle().cancel().await.unwrap();
    }
    assert!(
        finished,
        "an abandoned query must stop, not block the session"
    );
}

#[tokio::test]
async fn filters_match_numbers_in_typeless_and_computed_columns() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("typeless.db");
    {
        let setup = rusqlite::Connection::open(&path).unwrap();
        setup
            .execute_batch(
                "CREATE TABLE t (id, name);
                 INSERT INTO t VALUES (1, 'a'), (5, 'e'), (7, '007');
                 CREATE VIEW v AS SELECT id + 0 AS n FROM t;",
            )
            .unwrap();
    }
    let connection = Connection::connect(&ConnectSpec::sqlite(&path), &Secrets::default())
        .await
        .unwrap();
    let count = |object: &'static str, column: &'static str, op: FilterOp, value: &'static str| {
        let mut query = RowQuery::new(ObjectRef::new("main", object), 50);
        query.filters = vec![Filter {
            column: column.into(),
            op,
            value: value.into(),
        }];
        query
    };
    for (object, column, op, value, expected) in [
        ("t", "id", FilterOp::Eq, "5", 1),
        ("t", "id", FilterOp::Gt, "3", 2),
        ("t", "id", FilterOp::Lt, "3", 1),
        ("t", "id", FilterOp::In, "1, 5", 2),
        ("v", "n", FilterOp::Eq, "5", 1),
        // Text that only looks like a number stays text.
        ("t", "name", FilterOp::Eq, "007", 1),
    ] {
        let rows = connection
            .count_rows(&count(object, column, op, value))
            .await
            .unwrap();
        assert_eq!(rows, expected, "{object}.{column} {op:?} {value}");
    }
}

/// A UUID key kept as sixteen bytes, a foreign key to it, the same UUID as
/// text, and columns whose declared type does not say what they hold.
async fn uuid_keys() -> (Connection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("uuids.db");
    {
        let setup = rusqlite::Connection::open(&path).unwrap();
        setup
            .execute_batch(
                "CREATE TABLE accounts (
                     id blob(16) PRIMARY KEY, name TEXT NOT NULL, slug TEXT, token BINARY(16), raw
                 );
                 INSERT INTO accounts VALUES
                     (x'0199a3f27c1e7abc8def0123456789ab', 'Acme',
                      '0199a3f2-7c1e-7abc-8def-0123456789ab',
                      x'0199a3f27c1e7abc8def0123456789ab', x'cafe'),
                     (x'0199a3f27c1e7abc8def0123456789ac', 'Globex',
                      '0199a3f27c1e7abc8def0123456789ac',
                      x'0199a3f27c1e7abc8def0123456789ac', '0xcafe'),
                     ('0199a3f2-7c1e-7abc-8def-0123456789ad', 'Initech', '0xcafe', NULL, 7);
                 CREATE TABLE memberships (
                     id INTEGER PRIMARY KEY,
                     account_id blob(16) NOT NULL REFERENCES accounts (id)
                 );
                 INSERT INTO memberships VALUES
                     (1, x'0199a3f27c1e7abc8def0123456789ab'),
                     (2, x'0199a3f27c1e7abc8def0123456789ac'),
                     (3, x'0199a3f27c1e7abc8def0123456789ab');",
            )
            .unwrap();
    }
    let connection = Connection::connect(&ConnectSpec::sqlite(&path), &Secrets::default())
        .await
        .unwrap();
    (connection, dir)
}

/// The `name` of each row of `accounts` the filter keeps, in key order.
async fn accounts(connection: &Connection, column: &str, op: FilterOp, value: &str) -> Vec<String> {
    let mut query = RowQuery::new(ObjectRef::new("main", "accounts"), 50);
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
    let mut names: Vec<String> = page
        .rows
        .iter()
        .map(|row| match &row[1] {
            Value::Text(name) => name.to_string(),
            other => panic!("name is {other:?}"),
        })
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn a_uuid_or_hex_filter_matches_a_binary_key() {
    let (connection, _dir) = uuid_keys().await;
    for (column, op, value, expected) in [
        // The UUID the grid shows, with or without hyphens, and `0x` hex.
        (
            "id",
            FilterOp::Eq,
            "0199a3f2-7c1e-7abc-8def-0123456789ab",
            vec!["Acme"],
        ),
        (
            "id",
            FilterOp::Eq,
            " 0199A3F2-7C1E-7ABC-8DEF-0123456789AB ",
            vec!["Acme"],
        ),
        (
            "id",
            FilterOp::Eq,
            "0199a3f27c1e7abc8def0123456789ac",
            vec!["Globex"],
        ),
        (
            "id",
            FilterOp::Eq,
            "0x0199a3f27c1e7abc8def0123456789ab",
            vec!["Acme"],
        ),
        (
            "id",
            FilterOp::Ne,
            "0199a3f2-7c1e-7abc-8def-0123456789ab",
            vec!["Globex", "Initech"],
        ),
        (
            "id",
            FilterOp::In,
            "0199a3f2-7c1e-7abc-8def-0123456789ab, 0x0199a3f27c1e7abc8def0123456789ac, nothing",
            vec!["Acme", "Globex"],
        ),
        // A UUID stored as text in a binary column still matches as text.
        (
            "id",
            FilterOp::Eq,
            "0199a3f2-7c1e-7abc-8def-0123456789ad",
            vec!["Initech"],
        ),
        // Text columns keep matching as text.
        (
            "slug",
            FilterOp::Eq,
            "0199a3f2-7c1e-7abc-8def-0123456789ab",
            vec!["Acme"],
        ),
        (
            "slug",
            FilterOp::Eq,
            "0199a3f27c1e7abc8def0123456789ac",
            vec!["Globex"],
        ),
        ("slug", FilterOp::Eq, "0xcafe", vec!["Initech"]),
        (
            "slug",
            FilterOp::In,
            "0199a3f2-7c1e-7abc-8def-0123456789ab, 0xcafe",
            vec!["Acme", "Initech"],
        ),
        // A declared type SQLite gives no blob affinity, and none at all:
        // such a column holds bytes or text, and both match.
        (
            "token",
            FilterOp::Eq,
            "0199a3f2-7c1e-7abc-8def-0123456789ac",
            vec!["Globex"],
        ),
        ("raw", FilterOp::Eq, "0xcafe", vec!["Acme", "Globex"]),
        ("raw", FilterOp::Eq, "7", vec!["Initech"]),
    ] {
        assert_eq!(
            accounts(&connection, column, op, value).await,
            expected,
            "{column} {op:?} {value}"
        );
    }
}

#[tokio::test]
async fn a_binary_foreign_key_leads_to_its_row() {
    let (connection, _dir) = uuid_keys().await;
    let memberships = ObjectRef::new("main", "memberships");
    let structure = connection.describe(&memberships).await.unwrap();
    let foreign = &structure.foreign_keys[0];
    assert_eq!(foreign.columns, ["account_id"]);
    assert_eq!(
        (foreign.ref_table.as_str(), &foreign.ref_columns[0]),
        ("accounts", &"id".to_owned())
    );
    let page = connection
        .fetch_rows(&RowQuery::new(memberships, 50))
        .await
        .unwrap();
    assert_eq!(page.columns[1].kind, ValueKind::Binary);
    let Value::Bytes(key) = &page.rows[1][1] else {
        panic!("account_id is {:?}", page.rows[1][1]);
    };
    let hex: String = key.iter().map(|byte| format!("{byte:02x}")).collect();
    // What the app passes when a key is followed: the UUID it shows, and
    // `0x` hex for binary of any other length.
    let uuid = format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    );
    for value in [uuid, format!("0x{hex}")] {
        assert_eq!(
            accounts(&connection, &foreign.ref_columns[0], FilterOp::Eq, &value).await,
            ["Globex"],
            "{value}"
        );
    }
    // And back: the rows that point at one account.
    let mut query = RowQuery::new(ObjectRef::new("main", "memberships"), 50);
    query.filters = vec![Filter {
        column: "account_id".into(),
        op: FilterOp::Eq,
        value: "0199a3f2-7c1e-7abc-8def-0123456789ab".into(),
    }];
    assert_eq!(
        ids(&connection.fetch_rows(&query).await.unwrap()),
        vec![1, 3]
    );
}

fn script(text: &str) -> Vec<tabletist_db::sql::Statement> {
    tabletist_db::sql::statements(Dialect::Sqlite, text)
}

const FOREVER: &str =
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n) SELECT count(*) FROM n";

/// Awaits `future`, failing the test instead of hanging it.
async fn within<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), future)
        .await
        .expect("the run hung")
}

/// Runs `text` with a fresh stop flag and a generous limit.
async fn run(
    connection: &Connection,
    text: &str,
) -> tabletist_db::Result<tabletist_db::ScriptOutcome> {
    within(connection.run_script(&script(text), 100, &StopFlag::new())).await
}

#[tokio::test]
async fn a_script_returns_rows_with_types_and_truncates_at_the_limit() {
    let (connection, _dir) = fixture().await;
    let outcome = within(connection.run_script(
        &script("SELECT id, email FROM users ORDER BY id"),
        3,
        &StopFlag::new(),
    ))
    .await
    .unwrap();
    let [result] = outcome.results.as_slice() else {
        panic!("one result");
    };
    let StatementOutcome::Rows {
        columns,
        rows,
        truncated,
    } = &result.outcome
    else {
        panic!("rows");
    };
    assert_eq!(columns[0].name, "id");
    assert_eq!(columns[0].kind, ValueKind::Numeric);
    assert_eq!(rows.len(), 3);
    assert!(*truncated);
}

#[tokio::test]
async fn truncates_at_the_limit_without_reading_the_whole_table() {
    let (connection, _dir) = fixture().await;
    let started = std::time::Instant::now();
    let outcome =
        within(connection.run_script(&script("SELECT * FROM big a, big b"), 10, &StopFlag::new()))
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
async fn a_script_stops_at_the_first_error_and_keeps_earlier_results() {
    let (connection, _dir) = fixture().await;
    let outcome = run(&connection, "SELECT 1; SELECT nope FROM users; SELECT 3")
        .await
        .unwrap();
    assert_eq!(outcome.results.len(), 2);
    assert!(matches!(
        outcome.results[0].outcome,
        StatementOutcome::Rows { .. }
    ));
    assert!(matches!(
        outcome.results[1].outcome,
        StatementOutcome::Error { .. }
    ));
    assert!(!outcome.was_cancelled());
}

#[tokio::test]
async fn writes_fail_as_read_only_and_refusals_run_nothing() {
    for access in [Access::ReadOnly, Access::Writable] {
        let (connection, _dir) = fixture_as(access).await;
        let outcome = run(&connection, "DELETE FROM users").await.unwrap();
        // SQLITE_READONLY itself, none of its extended codes: the app tells a
        // refused write by it.
        assert!(
            matches!(
                &outcome.results[0].outcome,
                StatementOutcome::Error {
                    error: Error::Query { code: Some(code), .. },
                    ..
                } if code == "8"
            ),
            "{access:?}"
        );
        let refused = run(&connection, "SELECT 1;\nCOMMIT").await;
        assert!(
            matches!(refused, Err(Error::Refused { line: 2, .. })),
            "{access:?}"
        );
        let count = connection.count_rows(&users(10)).await.unwrap();
        assert_eq!(count, 5, "{access:?}");
    }
}

#[tokio::test]
async fn a_statement_without_rows_is_done_without_a_count() {
    let (connection, _dir) = fixture().await;
    // sqlite3_changes() would repeat an earlier statement's count here.
    let outcome = run(&connection, "PRAGMA foreign_keys = ON").await.unwrap();
    assert_eq!(
        outcome.results[0].outcome,
        StatementOutcome::Done { affected: None }
    );
}

#[tokio::test]
async fn a_comment_only_statement_is_done() {
    let (connection, _dir) = fixture().await;
    let piece = tabletist_db::sql::Statement {
        text: "-- nothing to run\n/* really */".into(),
        ..script("SELECT 1").remove(0)
    };
    let outcome = within(connection.run_script(&[piece], 100, &StopFlag::new()))
        .await
        .unwrap();
    assert_eq!(
        outcome.results[0].outcome,
        StatementOutcome::Done { affected: None }
    );
}

#[tokio::test]
async fn a_trailing_comment_stays_out_of_the_column_name() {
    let (connection, _dir) = fixture().await;
    let outcome = run(&connection, "SELECT 1 -- the answer").await.unwrap();
    let StatementOutcome::Rows { columns, .. } = &outcome.results[0].outcome else {
        panic!("rows");
    };
    assert_eq!(columns[0].name, "1");
}

#[tokio::test]
async fn a_hidden_second_statement_is_the_statements_error_not_a_second_run() {
    let (connection, _dir) = fixture().await;
    // Handed over as one piece, as if the splitter had missed the second.
    let piece = tabletist_db::sql::Statement {
        text: "SELECT 1; SELECT 2".into(),
        ..script("SELECT 1").remove(0)
    };
    let outcome = within(connection.run_script(&[piece], 100, &StopFlag::new()))
        .await
        .unwrap();
    let [result] = outcome.results.as_slice() else {
        panic!("one result");
    };
    assert!(
        matches!(result.outcome, StatementOutcome::Error { .. }),
        "{result:?}"
    );
}

#[tokio::test]
async fn a_failing_statement_still_rolls_the_transaction_back() {
    let (connection, _dir) = fixture().await;
    let failed = run(&connection, "SELECT 1; SELECT nope FROM users")
        .await
        .unwrap();
    assert!(matches!(
        failed.results[1].outcome,
        StatementOutcome::Error { .. }
    ));
    // A transaction left open would make this BEGIN fail.
    let after = run(&connection, "SELECT 1").await.unwrap();
    assert!(matches!(
        after.results[0].outcome,
        StatementOutcome::Rows { .. }
    ));
}

#[tokio::test]
async fn a_script_cannot_change_the_sessions_settings_for_later() {
    // Setting query_only is refused outright: on a writable file it is
    // what keeps a script from writing.
    for access in [Access::ReadOnly, Access::Writable] {
        let (connection, _dir) = fixture_as(access).await;
        let refused = run(&connection, "SELECT 1;\nPRAGMA 'query_only' = OFF").await;
        assert!(
            matches!(refused, Err(Error::Refused { line: 2, .. })),
            "{access:?}: {refused:?}"
        );
    }
    let (connection, _dir) = fixture().await;
    let changed = run(
        &connection,
        "PRAGMA trusted_schema = ON; PRAGMA busy_timeout = 0; PRAGMA case_sensitive_like = ON",
    )
    .await
    .unwrap();
    assert_eq!(changed.results.len(), 3, "{changed:?}");
    assert!(
        !changed
            .results
            .iter()
            .any(|result| matches!(result.outcome, StatementOutcome::Error { .. })),
        "{changed:?}"
    );
    let outcome = run(
        &connection,
        "PRAGMA query_only; PRAGMA trusted_schema; PRAGMA busy_timeout; SELECT 'x' LIKE 'X'",
    )
    .await
    .unwrap();
    let value = |index: usize| match &outcome.results[index].outcome {
        StatementOutcome::Rows { rows, .. } => rows[0][0].clone(),
        other => panic!("{other:?}"),
    };
    // Every setting is back as the connection opened with it.
    assert_eq!(value(0), Value::Int(1));
    assert_eq!(value(1), Value::Int(0));
    assert_eq!(value(2), Value::Int(5000));
    assert_eq!(value(3), Value::Int(1));
}

#[tokio::test]
async fn a_script_on_a_writable_file_changes_no_file() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    let path = dir.path().join("fixture.db");
    let before = std::fs::read(&path).unwrap();
    let missing = dir.path().join("missing.db");
    let copy = dir.path().join("copy.db");
    for text in [
        format!("ATTACH '{}' AS other", missing.display()),
        format!("ATTACH 'file:{}?mode=rwc' AS other", missing.display()),
        format!("VACUUM INTO '{}'", copy.display()),
        "PRAGMA journal_mode = WAL".to_owned(),
        // Refused, not harmless: it does nothing to this file, which is
        // not in WAL mode, and would rewrite one that is.
        "PRAGMA wal_checkpoint(TRUNCATE)".to_owned(),
        "UPDATE users SET email = 'x'".to_owned(),
        "CREATE TABLE made (n)".to_owned(),
    ] {
        // Refused, failed or harmless: each is fine, a changed file is not.
        let _ = run(&connection, &text).await;
        assert_eq!(std::fs::read(&path).unwrap(), before, "{text}");
        assert!(!missing.exists() && !copy.exists(), "{text}");
    }
    let mut names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["fixture.db"]);
}

#[tokio::test]
async fn an_empty_script_is_not_cancelled_but_a_stopped_one_is() {
    let (connection, _dir) = fixture().await;
    let empty = within(connection.run_script(&[], 10, &StopFlag::new()))
        .await
        .unwrap();
    assert!(empty.results.is_empty());
    assert!(!empty.was_cancelled());

    let stop = StopFlag::new();
    stop.stop();
    let stopped = within(connection.run_script(&script("SELECT 1"), 10, &stop))
        .await
        .unwrap();
    assert!(stopped.was_cancelled());
    assert!(
        !stopped
            .results
            .iter()
            .any(|result| matches!(result.outcome, StatementOutcome::Rows { .. }))
    );
    // Nothing was left open or registered.
    let after = run(&connection, "SELECT 1").await.unwrap();
    assert!(matches!(
        after.results[0].outcome,
        StatementOutcome::Rows { .. }
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stopped_script_is_cancelled_whatever_the_timing() {
    let (connection, _dir) = fixture().await;
    let connection = std::sync::Arc::new(connection);
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        tokio::spawn(async move { connection.run_script(&script(FOREVER), 10, &stop).await })
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    stop.stop();
    let outcome = within(running).await.unwrap().unwrap();
    assert!(outcome.was_cancelled());
    assert!(
        !outcome
            .results
            .iter()
            .any(|result| matches!(result.outcome, StatementOutcome::Rows { .. }))
    );
    // The session still works.
    run(&connection, "SELECT 1").await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stopped_script_never_reports_partial_rows() {
    let (connection, _dir) = fixture().await;
    let connection = std::sync::Arc::new(connection);
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        let text = format!("SELECT 1; {FOREVER}");
        tokio::spawn(async move { connection.run_script(&script(&text), 10, &stop).await })
    };
    // The first statement takes microseconds, so after this wait the second
    // is the one running. There is no way to see which statement the job is
    // in, so only what holds at any timing is asserted: a stop ends the run,
    // the last result is the cancelled one, and every result before it is
    // the first statement's rows.
    tokio::time::sleep(Duration::from_millis(300)).await;
    stop.stop();
    let outcome = within(running).await.unwrap().unwrap();
    assert!(outcome.was_cancelled());
    if let Some((last, earlier)) = outcome.results.split_last() {
        assert_eq!(last.outcome, StatementOutcome::Cancelled);
        assert!(
            earlier
                .iter()
                .all(|result| matches!(result.outcome, StatementOutcome::Rows { .. }))
        );
        assert!(earlier.len() <= 1);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_cancel_ends_the_script_as_cancelled() {
    let (connection, _dir) = fixture().await;
    let cancel = connection.cancel_handle();
    let connection = std::sync::Arc::new(connection);
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        tokio::spawn(async move {
            connection
                .run_script(&script(FOREVER), 10, &StopFlag::new())
                .await
        })
    };
    // SQLite ignores an interrupt when nothing runs yet. This test has no
    // stop flag, so it repeats the interrupt until one lands; the backend
    // sets the stop flag with its cancel instead.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !running.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "cancel must stop the script"
        );
        cancel.cancel().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let outcome = running.await.unwrap().unwrap();
    // An interrupt that lands on BEGIN ends the run with no results.
    assert!(outcome.was_cancelled());
    if let Some(last) = outcome.results.last() {
        assert_eq!(last.outcome, StatementOutcome::Cancelled);
    }
    // The session works, and the transaction was rolled back.
    let after = run(&connection, "SELECT 1").await.unwrap();
    assert!(matches!(
        after.results[0].outcome,
        StatementOutcome::Rows { .. }
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_dropped_run_stops_its_remaining_statements() {
    let (connection, _dir) = fixture().await;
    let stop = StopFlag::new();
    let text = format!("SELECT 1; {FOREVER}; {FOREVER}");
    let dropped = tokio::time::timeout(
        Duration::from_millis(300),
        connection.run_script(&script(&text), 10, &stop),
    )
    .await;
    assert!(dropped.is_err(), "the script cannot finish");
    assert!(stop.is_stopped(), "dropping the run stops it");
    // The abandoned job ends soon, so the next run is not stuck behind it.
    let after = run(&connection, "SELECT 1").await.unwrap();
    assert!(matches!(
        after.results[0].outcome,
        StatementOutcome::Rows { .. }
    ));
}

#[tokio::test]
async fn the_server_version_names_sqlite() {
    let (connection, _dir) = fixture().await;
    let version = within(connection.server_version()).await.unwrap();
    assert!(version.starts_with("SQLite 3."), "{version}");
}

/// `caf\xe9` ("café" in Latin-1) the way it reads once the byte that is not
/// UTF-8 is replaced.
const LOSSY: &str = "caf\u{FFFD}";

/// A file with names that are not UTF-8: SQLite keeps a name's bytes as they
/// were written. `t` has a column and a declared type named `caf\xe9` and an
/// index named `caf\xe9_idx` on that column, `v` gives the column another
/// name, `keyed` has it as its key and one table is named `caf\xe9s`. SQL
/// text is a `str` here, so the file is written with `caf~` and the byte is
/// put in afterwards.
async fn latin1_names() -> (Connection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("latin1.db");
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch(
            r#"CREATE TABLE t (id INTEGER PRIMARY KEY, "caf~" TEXT, price "caf~");
               INSERT INTO t VALUES (1, 'x', 2.5);
               CREATE INDEX "caf~_idx" ON t ("caf~");
               CREATE VIEW v AS SELECT id, "caf~" AS cafe FROM t;
               CREATE TABLE keyed ("caf~" TEXT PRIMARY KEY, note TEXT);
               INSERT INTO keyed VALUES ('b', 'second'), ('a', 'first');
               CREATE TABLE "caf~s" (id INTEGER PRIMARY KEY);"#,
        )
        .unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    let mut replaced = 0;
    for start in 0..bytes.len() - 3 {
        if &bytes[start..start + 4] == b"caf~" {
            bytes[start + 3] = 0xE9;
            replaced += 1;
        }
    }
    assert!(replaced >= 6, "{replaced} names were replaced");
    std::fs::write(&path, bytes).unwrap();
    let connection = Connection::connect(&ConnectSpec::sqlite(&path), &Secrets::default())
        .await
        .unwrap();
    (connection, dir)
}

fn names(page: &tabletist_db::RowPage) -> Vec<&str> {
    page.columns
        .iter()
        .map(|column| column.name.as_str())
        .collect()
}

#[tokio::test]
async fn a_table_with_names_that_are_not_utf8_is_browsed_and_described() {
    let (connection, _dir) = latin1_names().await;
    let object = ObjectRef::new("main", "t");
    let query = RowQuery::new(object.clone(), 50);
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(names(&page), ["id", LOSSY, "price"]);
    assert_eq!(page.columns[2].type_name, LOSSY);
    assert_eq!(
        page.rows,
        vec![vec![
            Value::Int(1),
            Value::Text("x".into()),
            Value::Float(2.5)
        ]]
    );
    assert!(page.ordered_by_key);
    assert_eq!(connection.count_rows(&query).await.unwrap(), 1);
    let structure = connection.describe(&object).await.unwrap();
    let columns: Vec<(&str, &str)> = structure
        .columns
        .iter()
        .map(|column| (column.name.as_str(), column.type_name.as_str()))
        .collect();
    assert_eq!(
        columns,
        [("id", "INTEGER"), (LOSSY, "TEXT"), ("price", LOSSY)]
    );
    assert_eq!(structure.primary_key, ["id"]);
    let indexes: Vec<(&str, &[String])> = structure
        .indexes
        .iter()
        .map(|index| (index.name.as_str(), index.columns.as_slice()))
        .collect();
    assert_eq!(
        indexes,
        [(
            format!("{LOSSY}_idx").as_str(),
            [LOSSY.to_owned()].as_slice()
        )]
    );
}

#[tokio::test]
async fn a_view_over_a_name_that_is_not_utf8_is_browsed_and_described() {
    let (connection, _dir) = latin1_names().await;
    let object = ObjectRef::new("main", "v");
    let query = RowQuery::new(object.clone(), 50);
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(names(&page), ["id", "cafe"]);
    assert_eq!(
        page.rows,
        vec![vec![Value::Int(1), Value::Text("x".into())]]
    );
    assert_eq!(connection.count_rows(&query).await.unwrap(), 1);
    assert_eq!(connection.describe(&object).await.unwrap().columns.len(), 2);
}

#[tokio::test]
async fn a_key_that_is_not_utf8_is_described_but_orders_no_page() {
    let (connection, _dir) = latin1_names().await;
    let object = ObjectRef::new("main", "keyed");
    let page = connection
        .fetch_rows(&RowQuery::new(object.clone(), 50))
        .await
        .unwrap();
    assert_eq!(names(&page), [LOSSY, "note"]);
    assert_eq!(page.rows.len(), 2);
    assert!(!page.ordered_by_key);
    let structure = connection.describe(&object).await.unwrap();
    assert_eq!(structure.primary_key, [LOSSY]);
    assert!(
        structure
            .indexes
            .iter()
            .any(|index| index.primary && index.columns == [LOSSY])
    );
}

#[tokio::test]
async fn a_name_that_is_not_utf8_is_listed_and_using_it_is_a_query_error() {
    let (connection, _dir) = latin1_names().await;
    let listed: Vec<String> = connection
        .list_objects("main")
        .await
        .unwrap()
        .into_iter()
        .map(|object| object.name)
        .collect();
    assert_eq!(listed, [format!("{LOSSY}s").as_str(), "keyed", "t", "v"]);
    // The name as listed is not the table's name, and no text is.
    let object = ObjectRef::new("main", format!("{LOSSY}s"));
    let query = RowQuery::new(object.clone(), 50);
    let fetched = connection.fetch_rows(&query).await;
    assert!(matches!(fetched, Err(Error::Query { .. })), "{fetched:?}");
    let counted = connection.count_rows(&query).await;
    assert!(matches!(counted, Err(Error::Query { .. })), "{counted:?}");
    let described = connection.describe(&object).await;
    assert!(
        matches!(described, Err(Error::Query { .. })),
        "{described:?}"
    );
    let mut sorted = RowQuery::new(ObjectRef::new("main", "t"), 50);
    sorted.sort = vec![Sort {
        column: LOSSY.into(),
        dir: SortDir::Asc,
    }];
    let fetched = connection.fetch_rows(&sorted).await;
    assert!(matches!(fetched, Err(Error::Query { .. })), "{fetched:?}");
}

#[tokio::test]
async fn a_script_reads_names_that_are_not_utf8() {
    let (connection, _dir) = latin1_names().await;
    let outcome = run(
        &connection,
        "SELECT * FROM t; SELECT count(*) FROM v; SELECT cafe FROM v",
    )
    .await
    .unwrap();
    let results: Vec<(Vec<&str>, &Vec<Vec<Value>>)> = outcome
        .results
        .iter()
        .map(|result| match &result.outcome {
            StatementOutcome::Rows { columns, rows, .. } => (
                columns.iter().map(|column| column.name.as_str()).collect(),
                rows,
            ),
            other => panic!("rows, not {other:?}"),
        })
        .collect();
    assert_eq!(
        results,
        [
            (
                vec!["id", LOSSY, "price"],
                &vec![vec![
                    Value::Int(1),
                    Value::Text("x".into()),
                    Value::Float(2.5)
                ]]
            ),
            (vec!["count(*)"], &vec![vec![Value::Int(1)]]),
            (vec!["cafe"], &vec![vec![Value::Text("x".into())]]),
        ]
    );
}
