//! Looking up the row a foreign key points at, on a Bookshop made in each
//! engine (`docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`,
//! tests 5 to 9).
//!
//! PostgreSQL and MySQL need their servers (see compose.yaml). Without the
//! variables those tests print "skipped" and pass.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use tabletist_db::{
    Access, ConnectSpec, Connection, HostKeys, Lookup, LookupPage, LookupPart, Matched, ObjectRef,
    Secrets, Value,
};

const NORTHLIGHT: i64 = 9_100_000_000_000_000_001;
const HARBOR_PRESS: i64 = 9_100_000_000_000_000_004;
const HARBOR_LIGHTS: i64 = 9_100_000_000_000_000_007;
/// No publisher has this id.
const NO_PUBLISHER: i64 = 9_100_000_000_000_000_099;

/// A read of `schema.table` by its `id` and `name`.
fn publishers(schema: &str, table: &str, part: LookupPart, limit: u32) -> Lookup {
    Lookup {
        object: ObjectRef::new(schema, table),
        key: "id".into(),
        label: Some("name".into()),
        part,
        limit,
    }
}

fn matching(text: &str) -> LookupPart {
    LookupPart::Matching(text.into())
}

/// A page's rows as (key, label, how it matched).
fn found(page: &LookupPage) -> Vec<(i64, String, Matched)> {
    page.rows
        .iter()
        .map(|row| {
            let key = match &row.key {
                Value::Int(key) => *key,
                other => panic!("a key is {other:?}"),
            };
            let label = match &row.label {
                Some(Value::Text(label)) => label.to_string(),
                other => panic!("a label is {other:?}"),
            };
            (key, label, row.matched)
        })
        .collect()
}

/// What every engine answers for the Bookshop's publishers, which are the
/// five of each module's `CREATE`.
async fn the_bookshop_is_searched(connection: &Connection, schema: &str, table: &str) {
    let read = |part: LookupPart, limit: u32| {
        let lookup = publishers(schema, table, part, limit);
        async move { connection.lookup(&lookup).await.unwrap() }
    };

    // Test 5: a label is found whatever its case, and its rows go by key.
    let page = read(matching("harb"), 50).await;
    assert!(!page.more);
    assert_eq!(
        found(&page),
        [
            (HARBOR_PRESS, "Harbor Press".to_owned(), Matched::Label),
            (HARBOR_LIGHTS, "Harbor Lights".to_owned(), Matched::Label),
        ]
    );

    // Test 6: the pinned row, and no row for a key nothing has.
    let page = read(LookupPart::Pinned(Value::Int(HARBOR_PRESS)), 1).await;
    assert_eq!(
        found(&page),
        [(HARBOR_PRESS, "Harbor Press".to_owned(), Matched::Key)]
    );
    let page = read(LookupPart::Pinned(Value::Int(NO_PUBLISHER)), 1).await;
    assert!(page.rows.is_empty() && !page.more);
    // Two keys that are one number as floats are still two rows.
    let page = read(LookupPart::Pinned(Value::Int(HARBOR_LIGHTS)), 1).await;
    assert!(!page.more);
    assert_eq!(
        found(&page),
        [(HARBOR_LIGHTS, "Harbor Lights".to_owned(), Matched::Key)]
    );

    // Test 7: the rows whose label holds the text come before the rows
    // whose key only begins with it.
    let page = read(matching("91"), 50).await;
    assert_eq!(
        found(&page),
        [
            (5, "Studio 91".to_owned(), Matched::Label),
            (NORTHLIGHT, "Northlight Books".to_owned(), Matched::KeyStart),
            (HARBOR_PRESS, "Harbor Press".to_owned(), Matched::KeyStart),
            (HARBOR_LIGHTS, "Harbor Lights".to_owned(), Matched::KeyStart),
        ]
    );

    // A text that is a key says so.
    let page = read(matching("5"), 50).await;
    assert_eq!(found(&page), [(5, "Studio 91".to_owned(), Matched::Key)]);

    // Nothing typed: the first rows by key, and whether there are more.
    let page = read(matching(""), 2).await;
    assert!(page.more);
    assert_eq!(
        found(&page).iter().map(|row| row.0).collect::<Vec<_>>(),
        [5, 6]
    );

    // A wildcard typed is a character to find.
    let page = read(matching("0%"), 50).await;
    assert_eq!(found(&page), [(6, "100% Books".to_owned(), Matched::Label)]);

    // A quote typed ends no string.
    let page = read(matching("'; DROP TABLE x; --"), 50).await;
    assert!(page.rows.is_empty());
    let page = read(matching(""), 50).await;
    assert_eq!(page.rows.len(), 5);
}

mod sqlite {
    use super::*;

    const CREATE: &str = "
        CREATE TABLE publishers (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        );
        INSERT INTO publishers VALUES
            (5, 'Studio 91'),
            (6, '100% Books'),
            (9100000000000000001, 'Northlight Books'),
            (9100000000000000004, 'Harbor Press'),
            (9100000000000000007, 'Harbor Lights');
        CREATE TABLE shelves (id INTEGER PRIMARY KEY, floor INTEGER);
        INSERT INTO shelves VALUES (12, 1), (120, 1), (3, 2);";

    /// A database of its own, made with `sql`.
    async fn made_with(sql: &str, access: Access) -> (Connection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(sql)
            .unwrap();
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

    #[tokio::test]
    async fn the_bookshop_is_searched_on_sqlite() {
        // Test 8: on a session that cannot write.
        let (connection, _dir) = made_with(CREATE, Access::ReadOnly).await;
        the_bookshop_is_searched(&connection, "main", "publishers").await;
        let (connection, _dir) = made_with(CREATE, Access::Writable).await;
        the_bookshop_is_searched(&connection, "main", "publishers").await;
    }

    #[tokio::test]
    async fn a_table_with_no_label_is_searched_by_its_key() {
        let (connection, _dir) = made_with(CREATE, Access::ReadOnly).await;
        let page = connection
            .lookup(&Lookup {
                object: ObjectRef::new("main", "shelves"),
                key: "id".into(),
                label: None,
                part: matching("12"),
                limit: 50,
            })
            .await
            .unwrap();
        let rows: Vec<_> = page
            .rows
            .iter()
            .map(|row| (row.key.clone(), row.label.clone(), row.matched))
            .collect();
        assert_eq!(
            rows,
            [
                (Value::Int(12), None, Matched::Key),
                (Value::Int(120), None, Matched::KeyStart),
            ]
        );
    }

    /// The limit holds inside the derived table: the database stops at
    /// the first rows it finds, and does not sort the table for the best
    /// ones. Row 3's key is the text, and it is not among the first two
    /// found.
    #[tokio::test]
    async fn the_matches_are_the_first_found_not_the_best_of_the_table() {
        const ROWS: &str = "
            CREATE TABLE publishers (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
            INSERT INTO publishers VALUES (1, 'x3'), (2, 'y3'), (3, 'z');";
        let (connection, _dir) = made_with(ROWS, Access::ReadOnly).await;
        let page = connection
            .lookup(&publishers("main", "publishers", matching("3"), 1))
            .await
            .unwrap();
        assert!(page.more);
        assert_eq!(found(&page), [(1, "x3".to_owned(), Matched::Label)]);
    }

    #[tokio::test]
    async fn a_table_that_is_not_there_is_the_databases_error() {
        let (connection, _dir) = made_with(CREATE, Access::ReadOnly).await;
        let error = connection
            .lookup(&publishers("main", "nowhere", matching("a"), 50))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("nowhere"), "{error}");
        // The session is usable after a failed lookup.
        let page = connection
            .lookup(&publishers("main", "publishers", matching("harb"), 50))
            .await
            .unwrap();
        assert_eq!(page.rows.len(), 2);
    }

    #[tokio::test]
    async fn a_label_that_is_null_is_a_row_all_the_same() {
        const ROWS: &str = "
            CREATE TABLE authors (id INTEGER PRIMARY KEY, name TEXT);
            INSERT INTO authors VALUES (1, 'Ada'), (2, NULL), (20, NULL);";
        let (connection, _dir) = made_with(ROWS, Access::ReadOnly).await;
        let page = connection
            .lookup(&Lookup {
                object: ObjectRef::new("main", "authors"),
                key: "id".into(),
                label: Some("name".into()),
                part: matching("2"),
                limit: 50,
            })
            .await
            .unwrap();
        let rows: Vec<_> = page
            .rows
            .iter()
            .map(|row| (row.key.clone(), row.label.clone(), row.matched))
            .collect();
        assert_eq!(
            rows,
            [
                (Value::Int(2), Some(Value::Null), Matched::Key),
                (Value::Int(20), Some(Value::Null), Matched::KeyStart),
            ]
        );
    }

    /// SQLite binds a typed text that reads as a number as a number, and
    /// the statement's cast makes it text again: `5` finds the key '5'.
    #[tokio::test]
    async fn a_text_key_that_reads_as_a_number_is_found_as_text() {
        const ROWS: &str = "
            CREATE TABLE codes (code TEXT PRIMARY KEY, name TEXT NOT NULL);
            INSERT INTO codes VALUES ('5', 'Five'), ('50', 'Fifty'), ('x', 'Other');";
        let (connection, _dir) = made_with(ROWS, Access::ReadOnly).await;
        let codes = |part: LookupPart| Lookup {
            object: ObjectRef::new("main", "codes"),
            key: "code".into(),
            label: Some("name".into()),
            part,
            limit: 50,
        };
        let page = connection.lookup(&codes(matching("5"))).await.unwrap();
        let rows: Vec<_> = page
            .rows
            .iter()
            .map(|row| (row.key.clone(), row.matched))
            .collect();
        assert_eq!(
            rows,
            [
                (Value::Text("5".into()), Matched::Key),
                (Value::Text("50".into()), Matched::KeyStart),
            ]
        );
        let page = connection
            .lookup(&codes(LookupPart::Pinned(Value::Text("5".into()))))
            .await
            .unwrap();
        let rows: Vec<_> = page.rows.iter().map(|row| row.key.clone()).collect();
        assert_eq!(rows, [Value::Text("5".into())]);
    }
}
