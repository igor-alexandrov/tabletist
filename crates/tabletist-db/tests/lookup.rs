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
async fn the_bookshop_is_searched(connection: &Connection, schema: &str, table: &str, on: &str) {
    let read = |part: LookupPart, limit: u32| {
        let lookup = publishers(schema, table, part, limit);
        async move {
            connection
                .lookup(&lookup)
                .await
                .unwrap_or_else(|error| panic!("{on}: {error}"))
        }
    };

    // Test 5: a label is found whatever its case, and its rows go by key.
    let page = read(matching("harb"), 50).await;
    assert!(!page.more, "{on}");
    assert_eq!(
        found(&page),
        [
            (HARBOR_PRESS, "Harbor Press".to_owned(), Matched::Label),
            (HARBOR_LIGHTS, "Harbor Lights".to_owned(), Matched::Label),
        ],
        "{on}"
    );

    // Test 6: the pinned row, and no row for a key nothing has.
    let page = read(LookupPart::Pinned(Value::Int(HARBOR_PRESS)), 1).await;
    assert_eq!(
        found(&page),
        [(HARBOR_PRESS, "Harbor Press".to_owned(), Matched::Key)],
        "{on}"
    );
    let page = read(LookupPart::Pinned(Value::Int(NO_PUBLISHER)), 1).await;
    assert!(page.rows.is_empty() && !page.more, "{on}");
    // Two keys that are one number as floats are still two rows.
    let page = read(LookupPart::Pinned(Value::Int(HARBOR_LIGHTS)), 1).await;
    assert!(!page.more, "{on}");
    assert_eq!(
        found(&page),
        [(HARBOR_LIGHTS, "Harbor Lights".to_owned(), Matched::Key)],
        "{on}"
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
        ],
        "{on}"
    );

    // A text that is a key says so.
    let page = read(matching("5"), 50).await;
    assert_eq!(
        found(&page),
        [(5, "Studio 91".to_owned(), Matched::Key)],
        "{on}"
    );

    // Nothing typed: the first rows by key, and whether there are more.
    let page = read(matching(""), 2).await;
    assert!(page.more, "{on}");
    assert_eq!(
        found(&page).iter().map(|row| row.0).collect::<Vec<_>>(),
        [5, 6],
        "{on}"
    );

    // A wildcard typed is a character to find.
    let page = read(matching("0%"), 50).await;
    assert_eq!(
        found(&page),
        [(6, "100% Books".to_owned(), Matched::Label)],
        "{on}"
    );

    // A quote typed ends no string.
    let page = read(matching("'; DROP TABLE x; --"), 50).await;
    assert!(page.rows.is_empty(), "{on}");
    let page = read(matching(""), 50).await;
    assert_eq!(page.rows.len(), 5, "{on}");
}

/// A search of `schema.table`, the Bookshop's editions, by its `id` and by
/// the column `label`.
fn editions(schema: &str, table: &str, label: &str, text: &str) -> Lookup {
    Lookup {
        object: ObjectRef::new(schema, table),
        key: "id".into(),
        label: Some(label.into()),
        part: matching(text),
        limit: 50,
    }
}

/// A label that is no text is searched by the text the database writes it
/// as, which is what the list draws
/// (`docs/superpowers/specs/2026-10-12-foreign-key-label-design.md`, test
/// 14). The editions are the three of each module's `EDITIONS`. `price` is
/// the first edition's price as this engine answers it, and `drawn` the
/// text it is drawn as: a decimal keeps its scale on PostgreSQL and MySQL
/// and is a number on SQLite.
async fn labels_that_are_no_text_are_searched_as_drawn(
    connection: &Connection,
    (schema, table): (&str, &str),
    (price, drawn): (Value, &str),
    on: &str,
) {
    let read = |label: &'static str, text: &str| {
        let lookup = editions(schema, table, label, text);
        async move {
            let page = connection
                .lookup(&lookup)
                .await
                .unwrap_or_else(|error| panic!("{on}: {error}"));
            let rows = page.rows.into_iter();
            rows.map(|row| (row.key, row.label, row.matched))
                .collect::<Vec<_>>()
        }
    };
    let date = |text: &str| Some(Value::Text(text.into()));
    // A date, by a part of it and by the whole.
    assert_eq!(
        read("published_on", "1998").await,
        [
            (Value::Int(1), date("1998-04-01"), Matched::Label),
            (Value::Int(3), date("1998-12-24"), Matched::Label),
        ],
        "{on}"
    );
    assert_eq!(
        read("published_on", "1998-04-01").await,
        [(Value::Int(1), date("1998-04-01"), Matched::Label)],
        "{on}"
    );
    // A whole number.
    assert_eq!(
        read("pages", "212").await,
        [(Value::Int(1), Some(Value::Int(212)), Matched::Label)],
        "{on}"
    );
    // A decimal, by the text it is drawn as.
    assert_eq!(
        read("price", drawn).await,
        [(Value::Int(1), Some(price), Matched::Label)],
        "{on}"
    );
    // What no label holds finds nothing, and is no error on a column of
    // dates or of numbers.
    assert!(read("published_on", "harb").await.is_empty(), "{on}");
    assert!(read("pages", "harb").await.is_empty(), "{on}");
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

    const EDITIONS: &str = "
        CREATE TABLE editions (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            pages INTEGER,
            price NUMERIC(10,2),
            published_on DATE
        );
        INSERT INTO editions VALUES
            (1, 'The Lighthouse', 212, 12.50, '1998-04-01'),
            (2, 'Harbor Nights', 96, 30.00, '2004-11-30'),
            (3, 'Low Tide', 304, 9.99, '1998-12-24');";

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
        the_bookshop_is_searched(&connection, "main", "publishers", "sqlite, ReadOnly").await;
        let (connection, _dir) = made_with(CREATE, Access::Writable).await;
        the_bookshop_is_searched(&connection, "main", "publishers", "sqlite, Writable").await;
    }

    #[tokio::test]
    async fn labels_that_are_no_text_are_searched_as_drawn_on_sqlite() {
        let (connection, _dir) = made_with(EDITIONS, Access::ReadOnly).await;
        labels_that_are_no_text_are_searched_as_drawn(
            &connection,
            ("main", "editions"),
            (Value::Float(12.5), "12.5"),
            "sqlite",
        )
        .await;
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

    /// Test 9. `REFERENCES publishers` means the parent's primary key.
    #[tokio::test]
    async fn an_implicit_reference_names_the_parents_primary_key() {
        const KEYS: &str = "
            CREATE TABLE publishers (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
            CREATE TABLE pairs (a INTEGER, b INTEGER, PRIMARY KEY (b, a));
            CREATE TABLE keyless (name TEXT);
            CREATE TABLE book_covers (
                id INTEGER PRIMARY KEY,
                publisher_id INTEGER REFERENCES publishers,
                named_id INTEGER REFERENCES publishers (id),
                a INTEGER,
                b INTEGER,
                keyless_id INTEGER REFERENCES keyless,
                gone_id INTEGER REFERENCES nowhere,
                lone_id INTEGER REFERENCES pairs,
                FOREIGN KEY (a, b) REFERENCES pairs
            );";
        let (connection, _dir) = made_with(KEYS, Access::ReadOnly).await;
        let structure = connection
            .describe(&ObjectRef::new("main", "book_covers"))
            .await
            .unwrap();
        let referenced = |column: &str| {
            let key = structure
                .foreign_keys
                .iter()
                .find(|key| key.columns[0] == column)
                .unwrap_or_else(|| panic!("no key of {column}"));
            (key.ref_table.as_str(), key.ref_columns.clone())
        };
        assert_eq!(
            referenced("publisher_id"),
            ("publishers", vec!["id".to_owned()])
        );
        assert_eq!(
            referenced("named_id"),
            ("publishers", vec!["id".to_owned()])
        );
        // The parent's key in its own order, which is how SQLite pairs them.
        assert_eq!(
            referenced("a"),
            ("pairs", vec!["b".to_owned(), "a".to_owned()])
        );
        // One column onto a two-column key means nothing.
        assert_eq!(referenced("lone_id"), ("pairs", Vec::new()));
        // No primary key to mean, and no table: the key names no column.
        assert_eq!(referenced("keyless_id"), ("keyless", Vec::new()));
        assert_eq!(referenced("gone_id"), ("nowhere", Vec::new()));
    }
}

mod postgres {
    use super::*;
    use tabletist_db::TlsMode;

    const DROP: &str = "DROP TABLE IF EXISTS lookup_publishers";

    const CREATE: &str = "
        CREATE TABLE lookup_publishers (
            id bigint PRIMARY KEY,
            name text NOT NULL
        );
        INSERT INTO lookup_publishers VALUES
            (5, 'Studio 91'),
            (6, '100% Books'),
            (9100000000000000001, 'Northlight Books'),
            (9100000000000000004, 'Harbor Press'),
            (9100000000000000007, 'Harbor Lights')";

    const DROP_EDITIONS: &str = "DROP TABLE IF EXISTS lookup_editions";

    const EDITIONS: &str = "
        CREATE TABLE lookup_editions (
            id bigint PRIMARY KEY,
            title text NOT NULL,
            pages integer,
            price numeric(10,2),
            published_on date
        );
        INSERT INTO lookup_editions VALUES
            (1, 'The Lighthouse', 212, 12.50, '1998-04-01'),
            (2, 'Harbor Nights', 96, 30.00, '2004-11-30'),
            (3, 'Low Tide', 304, 9.99, '1998-12-24')";

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_PG_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    #[tokio::test]
    async fn labels_that_are_no_text_are_searched_as_drawn_on_postgres() {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return;
        };
        let mut config: tokio_postgres::Config = url.parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        let (admin, session) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(session);
        admin.batch_execute(DROP_EDITIONS).await.unwrap();
        admin.batch_execute(EDITIONS).await.unwrap();

        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        let connection =
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
                .await
                .unwrap();
        labels_that_are_no_text_are_searched_as_drawn(
            &connection,
            ("public", "lookup_editions"),
            (Value::Text("12.50".into()), "12.50"),
            "postgres",
        )
        .await;
        admin.batch_execute(DROP_EDITIONS).await.unwrap();
    }

    #[tokio::test]
    async fn the_bookshop_is_searched_on_postgres() {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return;
        };
        let mut config: tokio_postgres::Config = url.parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        let (admin, session) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(session);
        admin.batch_execute(DROP).await.unwrap();
        admin.batch_execute(CREATE).await.unwrap();

        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        // Test 8: a session that cannot write looks up as one that can.
        for access in [Access::ReadOnly, Access::Writable] {
            let connection =
                Connection::connect_with(&spec, &secrets, &HostKeys::default(), access)
                    .await
                    .unwrap();
            the_bookshop_is_searched(
                &connection,
                "public",
                "lookup_publishers",
                &format!("postgres, {access:?}"),
            )
            .await;
            // A text that is no number is no error on a bigint key: the
            // key is compared as text.
            let page = connection
                .lookup(&publishers(
                    "public",
                    "lookup_publishers",
                    matching("harbor p"),
                    50,
                ))
                .await
                .unwrap();
            assert_eq!(page.rows.len(), 1);
        }
        admin.batch_execute(DROP).await.unwrap();
    }
}

mod mysql {
    use super::*;
    use mysql_async::prelude::Queryable;
    use tabletist_db::TlsMode;

    const DROP: &str = "DROP TABLE IF EXISTS lookup_publishers";

    const CREATE: [&str; 2] = [
        "CREATE TABLE lookup_publishers (
             id BIGINT PRIMARY KEY,
             name VARCHAR(64) NOT NULL
         )",
        "INSERT INTO lookup_publishers VALUES
             (5, 'Studio 91'),
             (6, '100% Books'),
             (9100000000000000001, 'Northlight Books'),
             (9100000000000000004, 'Harbor Press'),
             (9100000000000000007, 'Harbor Lights')",
    ];

    const DROP_EDITIONS: &str = "DROP TABLE IF EXISTS lookup_editions";

    const EDITIONS: [&str; 2] = [
        "CREATE TABLE lookup_editions (
             id BIGINT PRIMARY KEY,
             title VARCHAR(64) NOT NULL,
             pages INT,
             price DECIMAL(10,2),
             published_on DATE
         )",
        "INSERT INTO lookup_editions VALUES
             (1, 'The Lighthouse', 212, 12.50, '1998-04-01'),
             (2, 'Harbor Nights', 96, 30.00, '2004-11-30'),
             (3, 'Low Tide', 304, 9.99, '1998-12-24')",
    ];

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_MYSQL_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    #[tokio::test]
    async fn labels_that_are_no_text_are_searched_as_drawn_on_mysql() {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
            return;
        };
        let opts = mysql_async::Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
        admin.query_drop(DROP_EDITIONS).await.unwrap();
        for statement in EDITIONS {
            admin.query_drop(statement).await.unwrap();
        }

        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        let connection =
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
                .await
                .unwrap();
        labels_that_are_no_text_are_searched_as_drawn(
            &connection,
            ("tabletist", "lookup_editions"),
            (Value::Text("12.50".into()), "12.50"),
            "mysql",
        )
        .await;
        admin.query_drop(DROP_EDITIONS).await.unwrap();
    }

    #[tokio::test]
    async fn the_bookshop_is_searched_on_mysql() {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
            return;
        };
        let opts = mysql_async::Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
        admin.query_drop(DROP).await.unwrap();
        for statement in CREATE {
            admin.query_drop(statement).await.unwrap();
        }

        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        // Test 8: a session that cannot write looks up as one that can.
        for access in [Access::ReadOnly, Access::Writable] {
            let connection =
                Connection::connect_with(&spec, &secrets, &HostKeys::default(), access)
                    .await
                    .unwrap();
            the_bookshop_is_searched(
                &connection,
                "tabletist",
                "lookup_publishers",
                &format!("mysql, {access:?}"),
            )
            .await;
        }
        admin.query_drop(DROP).await.unwrap();
    }
}
