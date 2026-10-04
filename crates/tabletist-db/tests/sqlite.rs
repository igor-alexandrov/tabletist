//! The SQLite adapter through the public `Connection` API, on the shared fixture.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use tabletist_db::{
    Access, CellChange, ChangeSet, Conflict, ConnectSpec, Connection, Dialect, Driver, Error,
    Filter, FilterOp, HostKeys, NewValue, ObjectRef, RowChange, RowQuery, Secrets, Sort, SortDir,
    StatementOutcome, StopFlag, Value, ValueKind, WriteOutcome,
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

/// A session setting, as a script reads it.
async fn setting(connection: &Connection, name: &str) -> Value {
    let outcome = run(connection, &format!("PRAGMA {name}")).await.unwrap();
    match &outcome.results[0].outcome {
        StatementOutcome::Rows { rows, .. } => rows[0][0].clone(),
        other => panic!("{other:?}"),
    }
}

/// The session's `query_only`, as a script reads it.
async fn query_only(connection: &Connection) -> Value {
    setting(connection, "query_only").await
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
        // To SQLite `:a(')` is one variable token, and so is each of the
        // other four spellings up to its `)`; to our tokenizer a string or
        // a comment starts inside it and hides the `;` after it. Only
        // SQLite, asked when it prepares what follows, stops these, and
        // nothing would put these two settings back.
        let settings = async || {
            (
                setting(&connection, "foreign_keys").await,
                setting(&connection, "synchronous").await,
            )
        };
        let before = settings().await;
        // The texts set both to 0, which only shows on another value.
        assert_eq!(before, (Value::Int(1), Value::Int(2)), "{access:?}");
        // What the fence refuses, as `Conn::browse` words it.
        let fenced = |refused: &tabletist_db::Result<()>| {
            matches!(
                refused,
                Err(Error::Query { code: Some(code), message, .. })
                    if code == "23" && message == "A filter cannot use a PRAGMA \
                        with an argument, ATTACH or a transaction statement."
            )
        };
        for raw in [
            "1=1 OR :a(') IS NULL); PRAGMA foreign_keys = 0; PRAGMA synchronous = 0; SELECT ('",
            "1=1 OR $a(') IS NULL); PRAGMA foreign_keys = 0; SELECT ('",
            "1=1 OR @a(\") IS NULL); PRAGMA foreign_keys = 0; SELECT (\"",
            "1=1 OR #a(--) IS NULL); PRAGMA foreign_keys = 0; SELECT (1",
            "1=1 OR :a(/*) IS NULL); PRAGMA foreign_keys = 0; SELECT (1 /* */",
            // Hidden the same way, what else a filter may not hold. The
            // second ATTACH names its file by an expression, and reaches
            // the authorizer without the name.
            "1=1 OR :a(') IS NULL); BEGIN; SELECT ('",
            "1=1 OR :a(') IS NULL); SAVEPOINT x; SELECT ('",
            "1=1 OR :a(') IS NULL); ATTACH 'x' AS y; SELECT ('",
            "1=1 OR :a(') IS NULL); ATTACH 'x' || '' AS y; SELECT ('",
            "1=1 OR :a(') IS NULL); PRAGMA wal_checkpoint; SELECT ('",
        ] {
            query.raw_where = Some(raw.into());
            let page = connection.fetch_rows(&query).await.map(|_| ());
            assert!(fenced(&page), "{access:?} {raw}: {page:?}");
            assert_eq!(settings().await, before, "{access:?} {raw}");
            let count = connection.count_rows(&query).await.map(|_| ());
            assert!(fenced(&count), "{access:?} {raw}: {count:?}");
            assert_eq!(settings().await, before, "{access:?} {raw}");
        }
        // A `$` inside a name is where a tokenizer that knew those variables
        // would see one. Ours sees the `;`, and refuses the text itself.
        query.raw_where = Some(
            "1=1 OR EXISTS (WITH a$b(')') AS (SELECT 1) SELECT 1 FROM a$b)); \
             PRAGMA foreign_keys = 0; SELECT ('"
                .into(),
        );
        for refused in [
            connection.fetch_rows(&query).await.map(|_| ()),
            connection.count_rows(&query).await.map(|_| ()),
        ] {
            assert!(
                matches!(
                    &refused,
                    Err(Error::Query { code: None, message, .. }) if message.contains("`;`")
                ),
                "{access:?}: {refused:?}"
            );
            assert_eq!(settings().await, before, "{access:?}");
        }
        assert_eq!(connection.count_rows(&users(50)).await.unwrap(), 5);
        // A filter that only reads is none of the fence's business.
        query.raw_where = Some("id IN (SELECT id FROM users WHERE id < 3)".into());
        assert_eq!(
            ids(&connection.fetch_rows(&query).await.unwrap()),
            vec![1, 2]
        );
        assert_eq!(connection.count_rows(&query).await.unwrap(), 2);
        // A pragma read as a table is none of its business either, unless
        // it takes an argument: that reaches SQLite as the pragma's value,
        // and behind a filter a pragma with a value is what sets something.
        // An honest filter can do this, so it is told what it may not use.
        query.raw_where = Some("name IN (SELECT name FROM pragma_table_info('users'))".into());
        let page = connection.fetch_rows(&query).await.map(|_| ());
        assert!(fenced(&page), "{access:?}: {page:?}");
        let count = connection.count_rows(&query).await.map(|_| ());
        assert!(fenced(&count), "{access:?}: {count:?}");
        // A write a filter reaches is `query_only`'s to refuse, with the
        // code the app tells a refused write by: `PRAGMA optimize` would
        // run ANALYZE on `users`, whose index has no statistics.
        query.raw_where = Some("EXISTS (SELECT 1 FROM pragma_optimize)".into());
        for refused in [
            connection.fetch_rows(&query).await.map(|_| ()),
            connection.count_rows(&query).await.map(|_| ()),
        ] {
            assert!(
                matches!(
                    &refused,
                    Err(Error::Query { code: Some(code), .. }) if code == "8"
                ),
                "{access:?}: {refused:?}"
            );
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

#[tokio::test]
async fn the_fence_stands_over_names_that_are_not_utf8() {
    let (connection, _dir) = latin1_names().await;
    for name in ["t", "v"] {
        // SQLite asks about the column that is not UTF-8 for each of these,
        // with the filter's fence up.
        let mut query = RowQuery::new(ObjectRef::new("main", name), 50);
        query.raw_where = Some("id = 1".into());
        assert_eq!(connection.fetch_rows(&query).await.unwrap().rows.len(), 1);
        assert_eq!(connection.count_rows(&query).await.unwrap(), 1);
        // To SQLite `:a(')` is one variable token; to our tokenizer a string
        // starts inside it and hides the `;`. Only the fence stops these.
        for raw in [
            "1=1 OR :a(') IS NULL); PRAGMA foreign_keys = 0; SELECT ('",
            "1=1 OR :a(') IS NULL); BEGIN; SELECT ('",
        ] {
            query.raw_where = Some(raw.into());
            for refused in [
                connection.fetch_rows(&query).await.map(|_| ()),
                connection.count_rows(&query).await.map(|_| ()),
            ] {
                assert!(
                    matches!(
                        &refused,
                        Err(Error::Query { code: Some(code), message, .. })
                            if code == "23" && message.starts_with("A filter cannot use")
                    ),
                    "{name} {raw}: {refused:?}"
                );
            }
        }
    }
    // A script's fence, around a statement that reads such a column too.
    let outcome = run(
        &connection,
        "SELECT * FROM t; SELECT :a('); PRAGMA query_only = 0; --'",
    )
    .await
    .unwrap();
    assert!(
        matches!(
            outcome.results.first().map(|result| &result.outcome),
            Some(StatementOutcome::Rows { rows, .. }) if rows.len() == 1
        ),
        "{outcome:?}"
    );
    assert!(
        matches!(
            outcome.results.last().map(|result| &result.outcome),
            Some(StatementOutcome::Error {
                error: Error::Query { code: Some(code), message, .. },
                ..
            }) if code == "23" && message.contains("not authorized")
        ),
        "{outcome:?}"
    );
}

fn rename(id: i64, loaded: &str, new: &str) -> ChangeSet {
    ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: vec![RowChange {
            key: vec![("id".into(), Value::Int(id))],
            set: vec![CellChange {
                column: "name".into(),
                type_name: "TEXT".into(),
                loaded: Value::Text(loaded.into()),
                new: NewValue::Text(new.into()),
            }],
        }],
    }
}

#[tokio::test]
async fn a_read_only_connection_refuses_a_save_before_it_reads_it() {
    let (connection, dir) = fixture().await;
    let before = std::fs::read(dir.path().join("fixture.db")).unwrap();
    assert_eq!(
        connection.write(&rename(1, "Ada Lovelace", "Grace")).await,
        Err(Error::ReadOnly)
    );
    // Not even looked at: a set that could never be written gets the same.
    let empty = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: Vec::new(),
    };
    assert_eq!(connection.write(&empty).await, Err(Error::ReadOnly));
    assert_eq!(
        std::fs::read(dir.path().join("fixture.db")).unwrap(),
        before
    );
}

#[tokio::test]
async fn a_writable_connection_refuses_a_set_it_cannot_write() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    let empty = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: Vec::new(),
    };
    assert!(matches!(
        connection.write(&empty).await,
        Err(Error::Query { .. })
    ));
}

/// A second handle on a fixture's file, standing in for another program.
fn other_program(dir: &tempfile::TempDir) -> rusqlite::Connection {
    rusqlite::Connection::open(dir.path().join("fixture.db")).unwrap()
}

/// The row of `users` with this id, as a page gives it, and the names of
/// its columns.
async fn user(connection: &Connection, id: i64) -> (Vec<String>, Vec<Value>) {
    let mut query = users(10);
    query.filters.push(Filter {
        column: "id".into(),
        op: FilterOp::Eq,
        value: id.to_string(),
    });
    let page = connection.fetch_rows(&query).await.unwrap();
    (
        page.columns.into_iter().map(|column| column.name).collect(),
        page.rows.into_iter().next().unwrap(),
    )
}

/// A save of one row of `users`: each (column, declared type, new text),
/// with what the page holds now as the loaded value.
async fn save(
    connection: &Connection,
    id: i64,
    cells: &[(&str, &str, NewValue)],
) -> (ChangeSet, tabletist_db::Result<WriteOutcome>) {
    let (columns, row) = user(connection, id).await;
    let set = cells
        .iter()
        .map(|(column, type_name, new)| CellChange {
            column: (*column).into(),
            type_name: (*type_name).into(),
            loaded: row[columns.iter().position(|name| name == column).unwrap()].clone(),
            new: new.clone(),
        })
        .collect();
    let changes = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: vec![RowChange {
            key: vec![("id".into(), Value::Int(id))],
            set,
        }],
    };
    let outcome = connection.write(&changes).await;
    (changes, outcome)
}

fn to(text: &str) -> NewValue {
    NewValue::Text(text.into())
}

#[tokio::test]
async fn a_save_writes_every_kind_of_value_and_reads_the_row_back() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    let (columns, before) = user(&connection, 1).await;
    let (_, outcome) = save(
        &connection,
        1,
        &[
            ("email", "TEXT", to("new@example.com")),
            ("name", "TEXT", NewValue::Null),
            ("created_at", "DATETIME", to("2027-02-03 04:05:06")),
            ("active", "BOOLEAN", to("false")),
            ("meta", "JSON", to(r#"{"plan": "pro"}"#)),
            ("score", "REAL", to("12.5")),
        ],
    )
    .await;
    let WriteOutcome::Written { rows, .. } = outcome.unwrap() else {
        panic!("not written");
    };
    // What came back is what a page now shows.
    let (_, after) = user(&connection, 1).await;
    assert_eq!(rows, std::slice::from_ref(&after));
    let cell =
        |name: &str| after[columns.iter().position(|column| column == name).unwrap()].clone();
    assert_eq!(cell("email"), Value::Text("new@example.com".into()));
    assert_eq!(cell("name"), Value::Null);
    assert_eq!(cell("active"), Value::Int(0));
    assert_eq!(cell("score"), Value::Float(12.5));
    assert_eq!(cell("id"), before[0]);
    // And the row never conflicts with itself: each value written back
    // from what the page holds now.
    let (_, again) = save(
        &connection,
        1,
        &[
            ("email", "TEXT", to("back@example.com")),
            ("created_at", "DATETIME", to("2026-01-01 00:00:00")),
            ("active", "BOOLEAN", to("1")),
            ("meta", "JSON", NewValue::Null),
            ("score", "REAL", to("0.1")),
        ],
    )
    .await;
    assert!(
        matches!(again, Ok(WriteOutcome::Written { .. })),
        "{again:?}"
    );
    // Afterwards the session refuses writes as before.
    assert_eq!(query_only(&connection).await, Value::Int(1));
    let outcome = run(&connection, "UPDATE users SET name = 'x'")
        .await
        .unwrap();
    assert!(matches!(
        outcome.results[0].outcome,
        StatementOutcome::Error { .. }
    ));
}

#[tokio::test]
async fn a_row_changed_by_someone_else_is_a_conflict_and_nothing_is_written() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    // Loaded, then changed behind the page's back.
    let (columns, row) = user(&connection, 1).await;
    let name = columns.iter().position(|column| column == "name").unwrap();
    let changes = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: vec![
            RowChange {
                key: vec![("id".into(), Value::Int(2))],
                set: vec![CellChange {
                    column: "name".into(),
                    type_name: "TEXT".into(),
                    loaded: user(&connection, 2).await.1[name].clone(),
                    new: to("Second"),
                }],
            },
            RowChange {
                key: vec![("id".into(), Value::Int(1))],
                set: vec![CellChange {
                    column: "name".into(),
                    type_name: "TEXT".into(),
                    loaded: row[name].clone(),
                    new: to("Mine"),
                }],
            },
        ],
    };
    other_program(&dir)
        .execute("UPDATE users SET name = 'Theirs' WHERE id = 1", [])
        .unwrap();
    let outcome = connection.write(&changes).await.unwrap();
    let WriteOutcome::Conflicts(conflicts) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].row, 1);
    let server = conflicts[0].server.as_ref().unwrap();
    assert_eq!(server[name], Value::Text("Theirs".into()));
    // The other row of the set was not written either.
    assert_ne!(
        user(&connection, 2).await.1[name],
        Value::Text("Second".into())
    );
    assert_eq!(
        user(&connection, 1).await.1[name],
        Value::Text("Theirs".into())
    );
}

#[tokio::test]
async fn a_change_to_a_column_the_save_leaves_alone_is_no_conflict() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    let (columns, row) = user(&connection, 1).await;
    let name = columns.iter().position(|column| column == "name").unwrap();
    let changes = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: vec![RowChange {
            key: vec![("id".into(), Value::Int(1))],
            set: vec![CellChange {
                column: "name".into(),
                type_name: "TEXT".into(),
                loaded: row[name].clone(),
                new: to("Mine"),
            }],
        }],
    };
    other_program(&dir)
        .execute("UPDATE users SET score = 99 WHERE id = 1", [])
        .unwrap();
    assert!(matches!(
        connection.write(&changes).await,
        Ok(WriteOutcome::Written { .. })
    ));
}

#[tokio::test]
async fn a_row_that_is_gone_is_a_conflict_without_a_row() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    let (columns, row) = user(&connection, 5).await;
    let name = columns.iter().position(|column| column == "name").unwrap();
    let changes = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: vec![RowChange {
            key: vec![("id".into(), Value::Int(5))],
            set: vec![CellChange {
                column: "name".into(),
                type_name: "TEXT".into(),
                loaded: row[name].clone(),
                new: to("Late"),
            }],
        }],
    };
    // Its orders go first: the fixture has foreign keys.
    let other = other_program(&dir);
    other
        .execute("DELETE FROM orders WHERE user_id = 5", [])
        .unwrap();
    other.execute("DELETE FROM users WHERE id = 5", []).unwrap();
    assert_eq!(
        connection.write(&changes).await,
        Ok(WriteOutcome::Conflicts(vec![Conflict {
            row: 0,
            server: None
        }]))
    );
}

#[tokio::test]
async fn a_statement_that_fails_undoes_the_rows_before_it() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    let (columns, first) = user(&connection, 1).await;
    let (_, second) = user(&connection, 2).await;
    let at = |name: &str| columns.iter().position(|column| column == name).unwrap();
    let changes = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: vec![
            RowChange {
                key: vec![("id".into(), Value::Int(1))],
                set: vec![CellChange {
                    column: "name".into(),
                    type_name: "TEXT".into(),
                    loaded: first[at("name")].clone(),
                    new: to("Written first"),
                }],
            },
            RowChange {
                key: vec![("id".into(), Value::Int(2))],
                set: vec![CellChange {
                    column: "email".into(),
                    type_name: "TEXT".into(),
                    loaded: second[at("email")].clone(),
                    // NOT NULL: the database refuses it.
                    new: NewValue::Null,
                }],
            },
        ],
    };
    let outcome = connection.write(&changes).await.unwrap();
    assert!(
        matches!(outcome, WriteOutcome::Failed { row: 1, .. }),
        "{outcome:?}"
    );
    assert_eq!(user(&connection, 1).await.1, first);
    assert_eq!(query_only(&connection).await, Value::Int(1));
}

#[tokio::test]
async fn a_value_sqlite_would_store_as_text_is_refused_before_anything_is_sent() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    let before = std::fs::read(dir.path().join("fixture.db")).unwrap();
    // Another program holds the file for writing. A save that asked for
    // the file would wait for it and come back with the database's error,
    // not with the row's.
    let other = other_program(&dir);
    other.execute_batch("BEGIN IMMEDIATE").unwrap();
    let started = std::time::Instant::now();
    let (_, outcome) = save(&connection, 1, &[("active", "BOOLEAN", to("maybe"))]).await;
    assert!(
        matches!(outcome, Ok(WriteOutcome::Failed { row: 0, .. })),
        "{outcome:?}"
    );
    let (_, outcome) = save(&connection, 1, &[("score", "REAL", to("high"))]).await;
    assert!(
        matches!(outcome, Ok(WriteOutcome::Failed { row: 0, .. })),
        "{outcome:?}"
    );
    // So is a key that may not have been read exactly.
    let keyed = one_cell(
        "events",
        ("kind", Value::Text("log\u{FFFD}n".into())),
        "payload",
        "TEXT",
        Value::Text("ada".into()),
        to("mine"),
    );
    let outcome = connection.write(&keyed).await;
    assert!(
        matches!(outcome, Ok(WriteOutcome::Failed { row: 0, .. })),
        "{outcome:?}"
    );
    // None of them sat out the session's wait for a busy file.
    assert!(started.elapsed() < Duration::from_secs(4));
    other.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        std::fs::read(dir.path().join("fixture.db")).unwrap(),
        before
    );
}

#[tokio::test]
async fn a_save_does_not_inherit_what_a_script_left_on_the_session() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    other_program(&dir)
        .execute_batch(
            "CREATE TABLE kinds (id INTEGER PRIMARY KEY, kind TEXT CHECK (kind IN ('a', 'b')));
             INSERT INTO kinds VALUES (1, 'a');",
        )
        .unwrap();
    let mode = setting(&connection, "journal_mode").await;
    // The fence lets a script set these; they last for the session.
    run(
        &connection,
        "PRAGMA ignore_check_constraints = ON; PRAGMA journal_mode = MEMORY",
    )
    .await
    .unwrap();
    let changes = ChangeSet {
        object: ObjectRef::new("main", "kinds"),
        rows: vec![RowChange {
            key: vec![("id".into(), Value::Int(1))],
            set: vec![CellChange {
                column: "kind".into(),
                type_name: "TEXT".into(),
                loaded: Value::Text("a".into()),
                new: to("z"),
            }],
        }],
    };
    // The CHECK holds all the same, and the journal is the file's own.
    let outcome = connection.write(&changes).await.unwrap();
    assert!(
        matches!(outcome, WriteOutcome::Failed { row: 0, .. }),
        "{outcome:?}"
    );
    assert_eq!(setting(&connection, "journal_mode").await, mode);
}

/// Runs `text` on the session as a SQL editor would, every statement of it
/// succeeding: what it sets is left for whatever the session does next.
async fn script_leaves(connection: &Connection, text: &str) {
    let outcome = run(connection, text).await.unwrap();
    assert!(
        !outcome
            .results
            .iter()
            .any(|result| matches!(result.outcome, StatementOutcome::Error { .. })),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn a_script_cannot_make_a_saves_triggers_fire_themselves() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    other_program(&dir)
        .execute_batch(
            "CREATE TABLE counted (id INTEGER PRIMARY KEY, label TEXT, n INTEGER);
             INSERT INTO counted VALUES (1, 'a', 0);
             CREATE TRIGGER bump AFTER UPDATE ON counted WHEN NEW.n < 5 BEGIN
                 UPDATE counted SET n = n + 1 WHERE id = NEW.id;
             END;",
        )
        .unwrap();
    script_leaves(&connection, "PRAGMA recursive_triggers = ON").await;
    let changes = one_cell(
        "counted",
        ("id", Value::Int(1)),
        "label",
        "TEXT",
        Value::Text("a".into()),
        to("b"),
    );
    // Once, for the save's own update: the trigger's update does not fire
    // it again, which would count to 5.
    let outcome = connection.write(&changes).await;
    assert!(
        matches!(
            &outcome,
            Ok(WriteOutcome::Written { rows, .. })
                if rows == &[vec![Value::Int(1), Value::Text("b".into()), Value::Int(1)]]
        ),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn a_save_is_written_after_a_script_asked_for_change_counts() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    // With it an UPDATE gives a row, the count, which a save does not
    // expect of one.
    script_leaves(&connection, "PRAGMA count_changes = ON").await;
    let (_, outcome) = save(&connection, 1, &[("name", "TEXT", to("Mine"))]).await;
    assert!(
        matches!(outcome, Ok(WriteOutcome::Written { .. })),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn a_script_cannot_rename_the_columns_of_what_runs_after_it() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    let both = "PRAGMA full_column_names = ON; PRAGMA short_column_names = OFF";
    for left in [
        "PRAGMA full_column_names = ON",
        "PRAGMA short_column_names = OFF",
        both,
    ] {
        script_leaves(&connection, left).await;
        // Either of them names a column asked for by its table `users.id`,
        // and would for every script after this one.
        let outcome = run(&connection, "SELECT users.id FROM users LIMIT 1")
            .await
            .unwrap();
        let StatementOutcome::Rows { columns, .. } = &outcome.results[0].outcome else {
            panic!("{left}: {outcome:?}");
        };
        assert_eq!(columns[0].name, "id", "{left}");
    }
    // The two together name a table's own columns so, which a page shows,
    // and by which a save finds the columns it changes.
    script_leaves(&connection, both).await;
    let page = connection.fetch_rows(&users(1)).await.unwrap();
    assert_eq!(names(&page)[..3], ["id", "email", "name"]);
    let (_, outcome) = save(&connection, 1, &[("name", "TEXT", to("Mine"))]).await;
    assert!(
        matches!(outcome, Ok(WriteOutcome::Written { .. })),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn a_save_puts_back_the_journal_mode_a_script_left() {
    for left in ["PERSIST", "TRUNCATE", "MEMORY"] {
        let (connection, dir) = fixture_as(Access::Writable).await;
        let mode = setting(&connection, "journal_mode").await;
        assert_eq!(mode, Value::Text("delete".into()));
        script_leaves(&connection, &format!("PRAGMA journal_mode = {left}")).await;
        assert_eq!(
            setting(&connection, "journal_mode").await,
            Value::Text(left.to_lowercase().into()),
        );
        let (_, outcome) = save(&connection, 1, &[("name", "TEXT", to("Mine"))]).await;
        assert!(
            matches!(outcome, Ok(WriteOutcome::Written { .. })),
            "{left}: {outcome:?}"
        );
        assert_eq!(setting(&connection, "journal_mode").await, mode, "{left}");
        // The journal went when the save ended, as the file's own mode has
        // it: none is left beside the user's file.
        assert!(!dir.path().join("fixture.db-journal").exists(), "{left}");
    }
}

#[tokio::test]
async fn a_save_leaves_the_journal_mode_another_program_gave_the_file() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    assert_eq!(
        setting(&connection, "journal_mode").await,
        Value::Text("delete".into())
    );
    // WAL is the file's, not a session's: every program that opens the
    // file finds it so.
    let mode = |other: &rusqlite::Connection, sql: &str| {
        other
            .query_row(sql, [], |row| row.get::<_, String>(0))
            .unwrap()
    };
    assert_eq!(
        mode(&other_program(&dir), "PRAGMA journal_mode = WAL"),
        "wal"
    );
    let (_, outcome) = save(&connection, 1, &[("name", "TEXT", to("Mine"))]).await;
    assert!(
        matches!(outcome, Ok(WriteOutcome::Written { .. })),
        "{outcome:?}"
    );
    assert_eq!(mode(&other_program(&dir), "PRAGMA journal_mode"), "wal");
}

#[tokio::test]
async fn a_save_writes_a_decimal_as_a_number() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    let orders = ObjectRef::new("main", "orders");
    for (new, stored) in [("19.90", Value::Float(19.9)), ("20", Value::Int(20))] {
        let page = connection
            .fetch_rows(&RowQuery::new(orders.clone(), 1))
            .await
            .unwrap();
        let total = page
            .columns
            .iter()
            .position(|column| column.name == "total")
            .unwrap();
        let changes = ChangeSet {
            object: orders.clone(),
            rows: vec![RowChange {
                key: vec![("id".into(), page.rows[0][0].clone())],
                set: vec![CellChange {
                    column: "total".into(),
                    type_name: "NUMERIC(10, 2)".into(),
                    loaded: page.rows[0][total].clone(),
                    new: to(new),
                }],
            }],
        };
        let outcome = connection.write(&changes).await.unwrap();
        let WriteOutcome::Written { rows, .. } = outcome else {
            panic!("{new}: {outcome:?}");
        };
        assert_eq!(rows[0][total], stored, "{new}");
    }
}

#[tokio::test]
async fn a_table_of_an_attached_database_is_refused() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    let before = std::fs::read(dir.path().join("fixture.db")).unwrap();
    let mut changes = rename(1, "Ada Lovelace", "Grace");
    changes.object = ObjectRef::new("other", "users");
    assert!(matches!(
        connection.write(&changes).await,
        Err(Error::Unsupported(_))
    ));
    assert_eq!(
        std::fs::read(dir.path().join("fixture.db")).unwrap(),
        before
    );
}

#[tokio::test]
async fn a_key_that_matches_two_rows_is_an_error_and_nothing_is_written() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    let before = std::fs::read(dir.path().join("fixture.db")).unwrap();
    // `events` has no key: two of its rows are logins.
    let changes = ChangeSet {
        object: ObjectRef::new("main", "events"),
        rows: vec![RowChange {
            key: vec![("kind".into(), Value::Text("login".into()))],
            set: vec![CellChange {
                column: "payload".into(),
                type_name: "TEXT".into(),
                loaded: Value::Text("ada".into()),
                new: to("both"),
            }],
        }],
    };
    let refused = connection.write(&changes).await;
    assert!(
        matches!(&refused, Err(Error::Query { message, .. }) if message.contains("more than one row")),
        "{refused:?}"
    );
    assert_eq!(
        std::fs::read(dir.path().join("fixture.db")).unwrap(),
        before
    );
    assert_eq!(query_only(&connection).await, Value::Int(1));
}

#[tokio::test]
async fn a_key_a_trigger_gave_a_second_row_is_an_error_and_is_undone() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    // The key finds one row until the update's own trigger makes another
    // with it.
    let other = other_program(&dir);
    other
        .execute_batch(
            "CREATE TABLE tags (k TEXT, v TEXT);
             INSERT INTO tags VALUES ('a', 'old');
             CREATE TRIGGER tags_twin AFTER UPDATE ON tags BEGIN
                 INSERT INTO tags VALUES (NEW.k, 'twin');
             END;",
        )
        .unwrap();
    let changes = one_cell(
        "tags",
        ("k", Value::Text("a".into())),
        "v",
        "TEXT",
        Value::Text("old".into()),
        to("mine"),
    );
    let refused = connection.write(&changes).await;
    assert!(
        matches!(&refused, Err(Error::Query { message, .. }) if message.contains("more than one row")),
        "{refused:?}"
    );
    let held: Vec<String> = other
        .prepare("SELECT v FROM tags")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(held, ["old"]);
    assert_eq!(query_only(&connection).await, Value::Int(1));
}

#[tokio::test]
async fn an_update_that_does_not_change_one_row_fails_and_is_undone() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    // A view's trigger does the writing, so the update itself changes no
    // row of its own.
    other_program(&dir)
        .execute_batch(
            "CREATE VIEW named AS SELECT id, name FROM users;
             CREATE TRIGGER named_update INSTEAD OF UPDATE ON named BEGIN
                 UPDATE users SET name = NEW.name WHERE id = OLD.id;
             END;",
        )
        .unwrap();
    let (_, before) = user(&connection, 1).await;
    let changes = ChangeSet {
        object: ObjectRef::new("main", "named"),
        rows: vec![RowChange {
            key: vec![("id".into(), Value::Int(1))],
            set: vec![CellChange {
                column: "name".into(),
                type_name: "TEXT".into(),
                loaded: Value::Text("Ada Lovelace".into()),
                new: to("Through the view"),
            }],
        }],
    };
    let outcome = connection.write(&changes).await.unwrap();
    assert!(
        matches!(
            &outcome,
            WriteOutcome::Failed { row: 0, error } if error.to_string().contains("0 rows")
        ),
        "{outcome:?}"
    );
    // What the trigger wrote went with it.
    assert_eq!(user(&connection, 1).await.1, before);
    assert_eq!(query_only(&connection).await, Value::Int(1));
}

#[tokio::test]
async fn a_column_with_no_type_keeps_the_kind_of_value_it_held() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    other_program(&dir)
        .execute_batch(
            "CREATE TABLE loose (id INTEGER PRIMARY KEY, v);
             INSERT INTO loose VALUES (1, 5), (2, '5');",
        )
        .unwrap();
    // The same text typed into both: a number where the cell held a
    // number, text where it held text.
    for (id, loaded, stored) in [
        (1, Value::Int(5), Value::Int(6)),
        (2, Value::Text("5".into()), Value::Text("6".into())),
    ] {
        let changes = ChangeSet {
            object: ObjectRef::new("main", "loose"),
            rows: vec![RowChange {
                key: vec![("id".into(), Value::Int(id))],
                set: vec![CellChange {
                    column: "v".into(),
                    type_name: String::new(),
                    loaded,
                    new: to("6"),
                }],
            }],
        };
        let outcome = connection.write(&changes).await.unwrap();
        let WriteOutcome::Written { rows, .. } = outcome else {
            panic!("{id}: {outcome:?}");
        };
        assert_eq!(rows, [vec![Value::Int(id), stored]], "{id}");
    }
}

#[tokio::test]
async fn a_save_reads_a_table_with_names_that_are_not_utf8() {
    let (_, dir) = latin1_names().await;
    let path = dir.path().join("latin1.db");
    let connection = Connection::connect_with(
        &ConnectSpec::sqlite(&path),
        &Secrets::default(),
        &HostKeys::default(),
        Access::Writable,
    )
    .await
    .unwrap();
    let one =
        |table: &str, key: (&str, Value), column: &str, type_name: &str, loaded, new| ChangeSet {
            object: ObjectRef::new("main", table),
            rows: vec![RowChange {
                key: vec![(key.0.into(), key.1)],
                set: vec![CellChange {
                    column: column.into(),
                    type_name: type_name.into(),
                    loaded,
                    new,
                }],
            }],
        };
    // A column SQL can name is saved, beside one it cannot.
    let price = one(
        "t",
        ("id", Value::Int(1)),
        "price",
        LOSSY,
        Value::Float(2.5),
        to("3.5"),
    );
    assert!(matches!(
        connection.write(&price).await,
        Ok(WriteOutcome::Written { rows, .. })
            if rows == [vec![Value::Int(1), Value::Text("x".into()), Value::Float(3.5)]]
    ));
    // The column whose name is not UTF-8 cannot be named, nor can a key
    // that has such a name: neither is written.
    let before = std::fs::read(&path).unwrap();
    let unnamed = one(
        "t",
        ("id", Value::Int(1)),
        LOSSY,
        "TEXT",
        Value::Text("x".into()),
        to("y"),
    );
    let outcome = connection.write(&unnamed).await;
    assert!(
        matches!(outcome, Ok(WriteOutcome::Failed { row: 0, .. })),
        "{outcome:?}"
    );
    let keyed = one(
        "keyed",
        (LOSSY, Value::Text("a".into())),
        "note",
        "TEXT",
        Value::Text("first".into()),
        to("y"),
    );
    let outcome = connection.write(&keyed).await;
    assert!(matches!(outcome, Err(Error::Query { .. })), "{outcome:?}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(query_only(&connection).await, Value::Int(1));
}

#[tokio::test]
async fn a_save_does_not_keep_the_file_to_itself_after_a_script_asked_for_it() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    // In exclusive mode a session keeps every lock it takes.
    run(
        &connection,
        "PRAGMA locking_mode = EXCLUSIVE; SELECT count(*) FROM users",
    )
    .await
    .unwrap();
    let (_, outcome) = save(&connection, 1, &[("name", "TEXT", to("Mine"))]).await;
    assert!(
        matches!(outcome, Ok(WriteOutcome::Written { .. })),
        "{outcome:?}"
    );
    // Another program writes at once, without waiting for a lock.
    let other = other_program(&dir);
    other.busy_timeout(Duration::ZERO).unwrap();
    other
        .execute("UPDATE users SET name = 'Theirs' WHERE id = 2", [])
        .unwrap();
}

#[tokio::test]
async fn a_name_two_columns_read_as_is_refused() {
    // A name that is not UTF-8 reads with U+FFFD in its place, and another
    // column can have exactly that name, in these letters or in others:
    // SQLite matches names without regard to ASCII case.
    for twin in ["caf\u{FFFD}", "CAF\u{FFFD}"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("twins.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(&format!(
                "CREATE TABLE twins (id INTEGER PRIMARY KEY, \"caf~\" TEXT, \"{twin}\" TEXT);
                 INSERT INTO twins VALUES (1, 'first', 'twin');"
            ))
            .unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        for start in 0..bytes.len() - 3 {
            if &bytes[start..start + 4] == b"caf~" {
                bytes[start + 3] = 0xE9;
            }
        }
        std::fs::write(&path, &bytes).unwrap();
        let connection = Connection::connect_with(
            &ConnectSpec::sqlite(&path),
            &Secrets::default(),
            &HostKeys::default(),
            Access::Writable,
        )
        .await
        .unwrap();
        let object = ObjectRef::new("main", "twins");
        let page = connection
            .fetch_rows(&RowQuery::new(object.clone(), 5))
            .await
            .unwrap();
        assert_eq!(names(&page), ["id", LOSSY, twin]);
        // The cell of the column SQL cannot name: its statement would
        // name the twin, whose value the save never compared.
        let changes = ChangeSet {
            object,
            rows: vec![RowChange {
                key: vec![("id".into(), Value::Int(1))],
                set: vec![CellChange {
                    column: LOSSY.into(),
                    type_name: "TEXT".into(),
                    loaded: Value::Text("first".into()),
                    new: to("mine"),
                }],
            }],
        };
        let refused = connection.write(&changes).await;
        assert!(
            matches!(&refused, Err(Error::Query { message, .. }) if message.contains("more than one column")),
            "{twin}: {refused:?}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes, "{twin}");
    }
}

/// A writable session on the file at `path`.
async fn writable(path: &std::path::Path) -> Connection {
    Connection::connect_with(
        &ConnectSpec::sqlite(path),
        &Secrets::default(),
        &HostKeys::default(),
        Access::Writable,
    )
    .await
    .unwrap()
}

/// A save of one cell of the row of `table` that `key` finds.
fn one_cell(
    table: &str,
    key: (&str, Value),
    column: &str,
    type_name: &str,
    loaded: Value,
    new: NewValue,
) -> ChangeSet {
    ChangeSet {
        object: ObjectRef::new("main", table),
        rows: vec![RowChange {
            key: vec![(key.0.into(), key.1)],
            set: vec![CellChange {
                column: column.into(),
                type_name: type_name.into(),
                loaded,
                new,
            }],
        }],
    }
}

/// Each row of `sql`, a statement of two text columns, read by another
/// program.
fn pairs(other: &rusqlite::Connection, sql: &str) -> Vec<(String, String)> {
    other
        .prepare(sql)
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[tokio::test]
async fn a_key_that_may_not_have_been_read_exactly_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("twins.db");
    let other = rusqlite::Connection::open(&path).unwrap();
    // Text that is not UTF-8 reads with U+FFFD for its bad bytes, and
    // another row's key can be exactly that text.
    other
        .execute_batch(
            "CREATE TABLE k (code TEXT PRIMARY KEY, note TEXT);
             INSERT INTO k VALUES (CAST(X'636166E9' AS TEXT), 'same'), ('caf\u{FFFD}', 'same');",
        )
        .unwrap();
    let stored = || pairs(&other, "SELECT hex(code), note FROM k ORDER BY code");
    let before = stored();
    assert_eq!(
        before,
        [
            ("636166E9".to_owned(), "same".to_owned()),
            ("636166EFBFBD".to_owned(), "same".to_owned()),
        ]
    );
    let connection = writable(&path).await;
    let object = ObjectRef::new("main", "k");
    let page = connection
        .fetch_rows(&RowQuery::new(object, 5))
        .await
        .unwrap();
    // The page shows the two keys alike.
    assert_eq!(page.rows[0][0], Value::Text("caf\u{FFFD}".into()));
    assert_eq!(page.rows[1][0], page.rows[0][0]);
    // A save of the first row would find the second by that text.
    let changes = one_cell(
        "k",
        ("code", page.rows[0][0].clone()),
        "note",
        "TEXT",
        page.rows[0][1].clone(),
        to("mine"),
    );
    let outcome = connection.write(&changes).await;
    assert!(
        matches!(
            &outcome,
            Ok(WriteOutcome::Failed { row: 0, error })
                if error.to_string().contains("may not have been read exactly")
        ),
        "{outcome:?}"
    );
    assert_eq!(stored(), before);
}

/// A writable session on a file whose table `notes` holds text that is not
/// UTF-8 (`A\xE9`, which reads as `A` and U+FFFD): in the `body` of row 1
/// and in the `tag` of row 2. Row 3's `body` really is `A` and U+FFFD.
async fn text_that_is_not_utf8() -> (Connection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch(
            "CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT, tag TEXT);
             INSERT INTO notes VALUES
                 (1, CAST(X'41E9' AS TEXT), 'a'),
                 (2, 'plain', CAST(X'41E9' AS TEXT)),
                 (3, 'A\u{FFFD}', 'c');",
        )
        .unwrap();
    (writable(&path).await, dir)
}

/// A save of the `body` of the row of `notes` with this id, from what a
/// page holds now.
async fn body(connection: &Connection, id: i64, new: &str) -> ChangeSet {
    let page = connection
        .fetch_rows(&RowQuery::new(ObjectRef::new("main", "notes"), 5))
        .await
        .unwrap();
    let row = page
        .rows
        .iter()
        .find(|row| row[0] == Value::Int(id))
        .unwrap();
    one_cell(
        "notes",
        ("id", Value::Int(id)),
        "body",
        "TEXT",
        row[1].clone(),
        to(new),
    )
}

#[tokio::test]
async fn a_changed_column_holding_text_that_is_not_utf8_is_refused() {
    let (connection, dir) = text_that_is_not_utf8().await;
    let changes = body(&connection, 1, "mine").await;
    assert_eq!(
        changes.rows[0].set[0].loaded,
        Value::Text("A\u{FFFD}".into())
    );
    // Changed behind the page's back, to other bytes that read the same.
    let other = other_program(&dir);
    other
        .execute(
            "UPDATE notes SET body = CAST(X'41E8' AS TEXT) WHERE id = 1",
            [],
        )
        .unwrap();
    let outcome = connection.write(&changes).await;
    // Not a conflict, which would offer to overwrite.
    assert!(
        matches!(
            &outcome,
            Ok(WriteOutcome::Failed { row: 0, error })
                if error.to_string().contains("body") && error.to_string().contains("UTF-8")
        ),
        "{outcome:?}"
    );
    assert_eq!(
        pairs(&other, "SELECT hex(body), tag FROM notes WHERE id = 1"),
        [("41E8".to_owned(), "a".to_owned())]
    );
    assert_eq!(query_only(&connection).await, Value::Int(1));
}

#[tokio::test]
async fn text_that_is_not_utf8_in_another_column_does_not_stop_a_save() {
    let (connection, dir) = text_that_is_not_utf8().await;
    let changes = body(&connection, 2, "mine").await;
    let outcome = connection.write(&changes).await;
    assert!(
        matches!(
            &outcome,
            Ok(WriteOutcome::Written { rows, .. }) if rows == &[vec![
                Value::Int(2),
                Value::Text("mine".into()),
                Value::Text("A\u{FFFD}".into()),
            ]]
        ),
        "{outcome:?}"
    );
    // The other column keeps its bytes.
    assert_eq!(
        pairs(
            &other_program(&dir),
            "SELECT body, hex(tag) FROM notes WHERE id = 2"
        ),
        [("mine".to_owned(), "41E9".to_owned())]
    );
}

#[tokio::test]
async fn text_that_really_holds_the_replacement_character_is_saved() {
    let (connection, dir) = text_that_is_not_utf8().await;
    let changes = body(&connection, 3, "mine").await;
    assert_eq!(
        changes.rows[0].set[0].loaded,
        Value::Text("A\u{FFFD}".into())
    );
    let outcome = connection.write(&changes).await;
    assert!(
        matches!(outcome, Ok(WriteOutcome::Written { .. })),
        "{outcome:?}"
    );
    assert_eq!(
        pairs(
            &other_program(&dir),
            "SELECT body, tag FROM notes WHERE id = 3"
        ),
        [("mine".to_owned(), "c".to_owned())]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancel_during_a_save_undoes_it() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    let other = other_program(&dir);
    // For one name only, the UPDATE runs far longer than the test waits:
    // ten billion row pairs.
    other
        .execute_batch(
            "CREATE TRIGGER slow AFTER UPDATE OF name ON users WHEN NEW.name = 'Slow' BEGIN
                 SELECT count(*) FROM big a, big b;
             END;",
        )
        .unwrap();
    let (_, before) = user(&connection, 1).await;
    let cancel = connection.cancel_handle();
    let connection = std::sync::Arc::new(connection);
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        tokio::spawn(async move { connection.write(&rename(1, "Ada Lovelace", "Slow")).await })
    };
    // Wait until the save holds the file, and a moment more for the read of
    // its row: what the cancel then lands on is the UPDATE. Landing on an
    // earlier statement would end the save the same way, so the timing
    // decides only how much this proves, never whether it passes.
    other.busy_timeout(Duration::ZERO).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while other.execute_batch("BEGIN IMMEDIATE; ROLLBACK").is_ok() {
        assert!(
            std::time::Instant::now() < deadline,
            "the save never took the file"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    // SQLite ignores an interrupt when nothing runs, so keep cancelling
    // until the save stops.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !running.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "cancel must stop the save"
        );
        cancel.cancel().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // The save's own end, not a row's failure.
    assert_eq!(running.await.unwrap(), Err(Error::Cancelled));
    // Nothing was written, by the session's own reading and by another's.
    assert_eq!(user(&connection, 1).await.1, before);
    assert_eq!(
        pairs(&other, "SELECT name, email FROM users WHERE id = 1"),
        [("Ada Lovelace".to_owned(), "ada@example.com".to_owned())]
    );
    // The session refuses writes again, and saves as before.
    assert_eq!(query_only(&connection).await, Value::Int(1));
    let outcome = connection.write(&rename(1, "Ada Lovelace", "Quick")).await;
    assert!(
        matches!(outcome, Ok(WriteOutcome::Written { .. })),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn a_key_finds_the_row_whose_key_is_that_kind_of_value() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    let other = other_program(&dir);
    // A column with no declared type converts nothing: the number and the
    // text are two keys, and a filter's guess that `5` is a number would
    // read, and write, the wrong one of them.
    other
        .execute_batch(
            "CREATE TABLE loose (k PRIMARY KEY, n);
             INSERT INTO loose VALUES (5, 'int'), ('5', 'text');",
        )
        .unwrap();
    for (key, loaded) in [(Value::Text("5".into()), "text"), (Value::Int(5), "int")] {
        let saved = format!("{loaded}, saved");
        let changes = one_cell(
            "loose",
            ("k", key.clone()),
            "n",
            "",
            Value::Text(loaded.into()),
            to(&saved),
        );
        let outcome = connection.write(&changes).await;
        assert!(
            matches!(
                &outcome,
                Ok(WriteOutcome::Written { rows, .. })
                    if rows == &[vec![key.clone(), Value::Text(saved.as_str().into())]]
            ),
            "{key:?}: {outcome:?}"
        );
    }
    assert_eq!(
        pairs(&other, "SELECT typeof(k), n FROM loose ORDER BY typeof(k)"),
        [
            ("integer".to_owned(), "int, saved".to_owned()),
            ("text".to_owned(), "text, saved".to_owned()),
        ]
    );
}

/// Every cell of `table`, as SQLite holds it: its storage class and its
/// value.
fn stored(other: &rusqlite::Connection, table: &str) -> Vec<Vec<rusqlite::types::Value>> {
    let mut statement = other
        .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
        .unwrap();
    let width = statement.column_count();
    statement
        .query_map([], |row| {
            (0..width)
                .map(|index| row.get_ref(index).map(rusqlite::types::Value::from))
                .collect()
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[tokio::test]
async fn what_a_save_shows_stores_what_it_binds() {
    // In a UTF-16 file too: a literal that spells text by its bytes would
    // be read there as other text.
    for encoding in ["UTF-8", "UTF-16le"] {
        // Two files alike: one takes each statement as a person would copy it
        // and run it by hand, the other takes the save.
        let dir = tempfile::tempdir().unwrap();
        let fill = |name: &str| {
            let path = dir.path().join(name);
            let file = rusqlite::Connection::open(&path).unwrap();
            file.pragma_update(None, "encoding", encoding).unwrap();
            file.execute_batch(
            "CREATE TABLE vals (id INTEGER PRIMARY KEY, t TEXT, loose, n INTEGER, r REAL, d NUMERIC);
             INSERT INTO vals VALUES (1, 'old', 'old', 1, 1.5, 1), (2, 'old', 7, 1, 1.5, 1);
             CREATE TABLE named (k TEXT PRIMARY KEY, v TEXT);
             CREATE TABLE measured (k REAL PRIMARY KEY, v TEXT);
             INSERT INTO measured VALUES (9e999, 'old'), (-9e999, 'old'), (0.1, 'old');",
        )
        .unwrap();
            // A NUL ends SQL text, so these keys are bound.
            for key in ["it's", "nul\0'key", ""] {
                file.execute("INSERT INTO named VALUES (?1, 'old')", [key])
                    .unwrap();
            }
            (path, file)
        };
        let (_, by_hand) = fill("shown.db");
        let (path, bound) = fill("bound.db");
        let connection = writable(&path).await;
        let text = |value: &str| Value::Text(value.into());
        let old = || text("old");
        let id = |id: i64| ("id", Value::Int(id));
        let smallest = i64::MIN.to_string();
        let largest = i64::MAX.to_string();
        for (table, key, column, type_name, loaded, new) in [
            (
                "vals",
                id(1),
                "t",
                "TEXT",
                old(),
                to("it's \"quoted\" \\ here"),
            ),
            (
                "vals",
                id(1),
                "t",
                "TEXT",
                text("it's \"quoted\" \\ here"),
                to("a\0b'c"),
            ),
            ("vals", id(1), "t", "TEXT", text("a\0b'c"), to("")),
            ("vals", id(1), "t", "TEXT", text(""), NewValue::Null),
            // Text that reads as a number stays text where the cell held
            // text, and is a number where it held one.
            ("vals", id(1), "loose", "", old(), to("12")),
            ("vals", id(2), "loose", "", Value::Int(7), to("12")),
            ("vals", id(2), "loose", "", Value::Int(12), to("1.50")),
            ("vals", id(1), "n", "INTEGER", Value::Int(1), to(&smallest)),
            ("vals", id(2), "n", "INTEGER", Value::Int(1), to(&largest)),
            ("vals", id(1), "r", "REAL", Value::Float(1.5), to("0.1")),
            ("vals", id(2), "r", "REAL", Value::Float(1.5), to("-1e300")),
            ("vals", id(1), "r", "REAL", Value::Float(0.1), to("3")),
            ("vals", id(1), "d", "NUMERIC", Value::Int(1), to("12.50")),
            ("vals", id(2), "d", "NUMERIC", Value::Int(1), NewValue::Null),
            // And the key finds the same row either way.
            (
                "named",
                ("k", text("it's")),
                "v",
                "TEXT",
                old(),
                to("quoted"),
            ),
            (
                "named",
                ("k", text("nul\0'key")),
                "v",
                "TEXT",
                old(),
                to("nul"),
            ),
            ("named", ("k", text("")), "v", "TEXT", old(), to("empty")),
            (
                "measured",
                ("k", Value::Float(f64::INFINITY)),
                "v",
                "TEXT",
                old(),
                to("most"),
            ),
            (
                "measured",
                ("k", Value::Float(f64::NEG_INFINITY)),
                "v",
                "TEXT",
                old(),
                to("least"),
            ),
            (
                "measured",
                ("k", Value::Float(0.1)),
                "v",
                "TEXT",
                old(),
                to("a tenth"),
            ),
        ] {
            let changes = one_cell(table, key, column, type_name, loaded, new);
            let case = format!("{:?}", changes.rows[0]);
            let update = Dialect::Sqlite
                .update_row(&changes.object, &changes.rows[0])
                .unwrap();
            let touched = by_hand.execute(&update.shown, []);
            assert_eq!(touched, Ok(1), "{}: {case}", update.shown);
            let outcome = connection.write(&changes).await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Written { .. })),
                "{case}: {outcome:?}"
            );
            assert_eq!(
                stored(&by_hand, table),
                stored(&bound, table),
                "{encoding} {}: {case}",
                update.shown
            );
        }
        let kept: String = by_hand
            .query_row("PRAGMA encoding", [], |row| row.get(0))
            .unwrap();
        assert_eq!(kept, encoding);
    }
}
