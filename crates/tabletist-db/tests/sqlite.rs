//! The SQLite adapter through the public `Connection` API, on the shared fixture.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use tabletist_db::{
    ConnectSpec, Connection, Driver, Error, Filter, FilterOp, ObjectRef, RowQuery, Secrets, Sort,
    SortDir, Value, ValueKind,
};

async fn fixture() -> (Connection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    tabletist_db::fixtures::write_sqlite_demo(&path).unwrap();
    let connection = Connection::connect(&ConnectSpec::sqlite(&path), &Secrets::default())
        .await
        .unwrap();
    (connection, dir)
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

#[tokio::test]
async fn a_raw_where_cannot_modify_data() {
    let (connection, _dir) = fixture().await;
    let mut query = users(50);
    // A plain syntax error, and one that closes the parenthesis to chain a
    // real second statement (rusqlite refuses to prepare more than one).
    for raw in [
        "1 = 1; DELETE FROM users",
        "1=1) ; DELETE FROM users; SELECT (1",
    ] {
        query.raw_where = Some(raw.into());
        assert!(connection.fetch_rows(&query).await.is_err(), "{raw}");
    }
    assert_eq!(connection.count_rows(&users(50)).await.unwrap(), 5);
}

#[tokio::test]
async fn a_raw_where_cannot_drop_the_page_limit() {
    let (connection, _dir) = fixture().await;
    let mut query = RowQuery::new(ObjectRef::new("main", "big"), 10);
    // An unterminated comment would swallow the builder's LIMIT and OFFSET.
    for raw in ["1=1) /*", "1=1) --"] {
        query.raw_where = Some(raw.into());
        if let Ok(page) = connection.fetch_rows(&query).await {
            assert!(page.rows.len() <= 10, "{raw}: {} rows", page.rows.len());
        }
    }
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
