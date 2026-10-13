//! Deleting rows in a save, on a Bookshop made in each engine: a row goes
//! by its key, a row someone deleted first is said and the rest is saved,
//! new rows go in and rows are changed before rows go out, and a row a
//! foreign key holds stays, with everything else the save did undone.
//!
//! "Spec N" is a section of
//! `docs/superpowers/specs/2026-10-12-deleting-rows-design.md`, "test N"
//! an entry of its Tests.
//!
//! PostgreSQL and MySQL need their servers (see compose.yaml). Without the
//! variables those tests print "skipped" and pass.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use tabletist_db::{
    Access, CellChange, ChangeSet, ConnectSpec, Connection, Error, HostKeys, InsertValue, NewValue,
    ObjectRef, RowChange, RowDelete, RowInsert, RowQuery, Secrets, StopFlag, TlsMode, Value,
    WriteOutcome,
};

const NORTHLIGHT: i64 = 9_100_000_000_000_000_001;
const HARBOR_PRESS: i64 = 9_100_000_000_000_000_004;
const HARBOR_LIGHTS: i64 = 9_100_000_000_000_000_007;

/// A Bookshop's tables as one engine names and types them.
struct Shop {
    schema: String,
    publishers: String,
    covers: String,
    authors: String,
    shelves: String,
    /// The type of an id column, and of a text one, as the structure
    /// names them.
    id: &'static str,
    text: &'static str,
}

impl Shop {
    fn table(&self, name: &str) -> ObjectRef {
        ObjectRef::new(&self.schema, name)
    }
}

fn by_id(id: i64) -> RowDelete {
    RowDelete {
        key: vec![("id".to_owned(), Value::Int(id))],
    }
}

fn deleting(object: ObjectRef, deletes: Vec<RowDelete>) -> ChangeSet {
    ChangeSet {
        object,
        inserts: Vec::new(),
        rows: Vec::new(),
        deletes,
    }
}

async fn save(connection: &Connection, changes: &ChangeSet) -> WriteOutcome {
    tokio::time::timeout(
        Duration::from_secs(15),
        connection.write(changes, &StopFlag::new()),
    )
    .await
    .expect("the save hung")
    .unwrap()
}

/// The table's rows, each as text, in the order of their first column.
async fn rows(connection: &Connection, object: ObjectRef) -> Vec<Vec<String>> {
    let page = connection
        .fetch_rows(&RowQuery::new(object, 500))
        .await
        .unwrap();
    let mut rows: Vec<Vec<String>> = page
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| match value {
                    Value::Int(number) => number.to_string(),
                    Value::Text(text) => text.to_string(),
                    other => format!("{other:?}"),
                })
                .collect()
        })
        .collect();
    rows.sort();
    rows
}

/// The first column of each of the table's rows.
async fn firsts(connection: &Connection, object: ObjectRef) -> Vec<String> {
    let rows = rows(connection, object).await;
    rows.into_iter().map(|mut row| row.remove(0)).collect()
}

/// Spec 4: a row goes by its key, and the others stay.
async fn a_marked_row_is_deleted(connection: &Connection, shop: &Shop) {
    let covers = shop.table(&shop.covers);
    let outcome = save(connection, &deleting(covers.clone(), vec![by_id(2)])).await;
    let WriteOutcome::Written {
        inserted,
        rows,
        gone,
        ..
    } = outcome
    else {
        panic!("the save wrote: {outcome:?}");
    };
    assert!(inserted.is_empty() && rows.is_empty() && gone.is_empty());
    assert_eq!(firsts(connection, covers).await, ["1", "3"]);
}

/// Spec 4 and test 9: cover 2 was deleted by someone else before this is
/// called. The save says so, deletes cover 1 and adds its new row.
async fn a_row_already_gone_is_said_and_the_rest_is_saved(connection: &Connection, shop: &Shop) {
    let covers = shop.table(&shop.covers);
    let changes = ChangeSet {
        inserts: vec![RowInsert {
            set: vec![InsertValue {
                column: "publisher_id".into(),
                type_name: shop.id.into(),
                new: NewValue::Text(HARBOR_PRESS.to_string()),
            }],
        }],
        ..deleting(covers.clone(), vec![by_id(1), by_id(2)])
    };
    let outcome = save(connection, &changes).await;
    let WriteOutcome::Written { inserted, gone, .. } = outcome else {
        panic!("the save wrote: {outcome:?}");
    };
    // The second of the set's deletes, not the row with id 1.
    assert_eq!(gone, [1]);
    assert_eq!(inserted.len(), 1);
    assert_eq!(firsts(connection, covers).await, ["3", "4"]);
}

/// Spec 4 and test 8: inserts go before deletes. A new publisher cannot
/// take the name of one the same save deletes, and nothing is written.
async fn a_new_row_goes_in_before_a_row_goes_out(connection: &Connection, shop: &Shop) {
    let publishers = shop.table(&shop.publishers);
    let before = rows(connection, publishers.clone()).await;
    let value = |column: &str, type_name: &str, new: String| InsertValue {
        column: column.into(),
        type_name: type_name.into(),
        new: NewValue::Text(new),
    };
    let changes = ChangeSet {
        inserts: vec![RowInsert {
            set: vec![
                value("id", shop.id, "9100000000000000009".into()),
                value("name", shop.text, "Harbor Lights".into()),
            ],
        }],
        ..deleting(publishers.clone(), vec![by_id(HARBOR_LIGHTS)])
    };
    let outcome = save(connection, &changes).await;
    assert!(
        matches!(outcome, WriteOutcome::FailedInsert { insert: 0, .. }),
        "{outcome:?}"
    );
    assert_eq!(rows(connection, publishers).await, before);
}

/// Spec 4 and test 8: updates go before deletes. Harbor Press cannot take
/// the name of a publisher the same save deletes, and nothing is written.
async fn a_row_is_changed_before_a_row_goes_out(connection: &Connection, shop: &Shop) {
    let publishers = shop.table(&shop.publishers);
    let before = rows(connection, publishers.clone()).await;
    let changes = ChangeSet {
        rows: vec![RowChange {
            key: vec![("id".to_owned(), Value::Int(HARBOR_PRESS))],
            set: vec![CellChange {
                column: "name".into(),
                type_name: shop.text.into(),
                loaded: Value::Text("Harbor Press".into()),
                new: NewValue::Text("Harbor Lights".into()),
            }],
        }],
        ..deleting(publishers.clone(), vec![by_id(HARBOR_LIGHTS)])
    };
    let outcome = save(connection, &changes).await;
    assert!(
        matches!(outcome, WriteOutcome::Failed { row: 0, .. }),
        "{outcome:?}"
    );
    assert_eq!(rows(connection, publishers).await, before);
}

/// Spec 5 and test 8: Northlight Books has covers, so its delete is
/// refused, and what the save did before it is undone: the publisher it
/// renamed has its name, and the one it deleted first is there. The
/// engine's own error is handed back for its test to read.
async fn a_row_a_foreign_key_holds_stays_and_the_save_is_undone(
    connection: &Connection,
    shop: &Shop,
) -> Error {
    let publishers = shop.table(&shop.publishers);
    let before = rows(connection, publishers.clone()).await;
    let changes = ChangeSet {
        rows: vec![RowChange {
            key: vec![("id".to_owned(), Value::Int(HARBOR_PRESS))],
            set: vec![CellChange {
                column: "name".into(),
                type_name: shop.text.into(),
                loaded: Value::Text("Harbor Press".into()),
                new: NewValue::Text("Harbor House".into()),
            }],
        }],
        ..deleting(
            publishers.clone(),
            vec![by_id(HARBOR_LIGHTS), by_id(NORTHLIGHT)],
        )
    };
    let outcome = save(connection, &changes).await;
    let WriteOutcome::FailedDelete { delete, error } = outcome else {
        panic!("the save was to fail on a row to delete: {outcome:?}");
    };
    // The second of the set's deletes: the first went through, and was
    // undone with the rest.
    assert_eq!(delete, 1);
    assert_eq!(rows(connection, publishers).await, before);
    error
}

/// Spec 4: a key of two columns finds its one row.
async fn a_key_of_two_columns_finds_its_row(connection: &Connection, shop: &Shop) {
    let authors = shop.table(&shop.authors);
    let changes = deleting(
        authors.clone(),
        vec![RowDelete {
            key: vec![
                ("book_id".to_owned(), Value::Int(101)),
                ("author".to_owned(), Value::Text("Sam O'Neil".into())),
            ],
        }],
    );
    let outcome = save(connection, &changes).await;
    assert!(
        matches!(&outcome, WriteOutcome::Written { gone, .. } if gone.is_empty()),
        "{outcome:?}"
    );
    assert_eq!(
        rows(connection, authors).await,
        [["101", "Ada Quill"], ["102", "Ada Quill"]]
    );
}

/// Shelf 2 stands on shelf 1, and goes with it (`ON DELETE CASCADE`). A
/// save that deletes both is no failure: the second was the save's own
/// doing, and is not said to have been gone before it.
async fn a_row_the_save_took_with_another_is_deleted(connection: &Connection, shop: &Shop) {
    let shelves = shop.table(&shop.shelves);
    let outcome = save(
        connection,
        &deleting(shelves.clone(), vec![by_id(1), by_id(2)]),
    )
    .await;
    assert!(
        matches!(&outcome, WriteOutcome::Written { gone, .. } if gone.is_empty()),
        "{outcome:?}"
    );
    assert_eq!(firsts(connection, shelves).await, ["3"]);
}

mod sqlite {
    use super::*;

    const CREATE: &str = "
        CREATE TABLE publishers (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        );
        INSERT INTO publishers VALUES
            (9100000000000000001, 'Northlight Books'),
            (9100000000000000004, 'Harbor Press'),
            (9100000000000000007, 'Harbor Lights');
        CREATE TABLE book_covers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            publisher_id INTEGER NOT NULL REFERENCES publishers (id),
            kind TEXT NOT NULL DEFAULT 'print'
        );
        INSERT INTO book_covers (publisher_id, kind) VALUES
            (9100000000000000001, 'print'),
            (9100000000000000001, 'ebook'),
            (9100000000000000004, 'print');
        CREATE TABLE book_authors (
            book_id INTEGER NOT NULL,
            author TEXT NOT NULL,
            PRIMARY KEY (book_id, author)
        );
        INSERT INTO book_authors VALUES
            (101, 'Ada Quill'), (101, 'Sam O''Neil'), (102, 'Ada Quill');
        CREATE TABLE shelves (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER REFERENCES shelves (id) ON DELETE CASCADE
        );
        INSERT INTO shelves VALUES (1, NULL), (2, 1), (3, NULL)";

    fn shop() -> Shop {
        Shop {
            schema: "main".into(),
            publishers: "publishers".into(),
            covers: "book_covers".into(),
            authors: "book_authors".into(),
            shelves: "shelves".into(),
            id: "INTEGER",
            text: "TEXT",
        }
    }

    async fn bookshop() -> (Connection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(CREATE)
            .unwrap();
        let connection = Connection::connect_with(
            &ConnectSpec::sqlite(&path),
            &Secrets::default(),
            &HostKeys::default(),
            Access::Writable,
        )
        .await
        .unwrap();
        (connection, dir)
    }

    /// A second handle on the Bookshop's file, standing in for another
    /// program.
    fn other_program(dir: &tempfile::TempDir) -> rusqlite::Connection {
        rusqlite::Connection::open(dir.path().join("bookshop.db")).unwrap()
    }

    #[tokio::test]
    async fn sqlite_deletes_a_row_by_its_key() {
        let (connection, _dir) = bookshop().await;
        a_marked_row_is_deleted(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_says_a_row_was_already_gone_and_saves_the_rest() {
        let (connection, dir) = bookshop().await;
        other_program(&dir)
            .execute("DELETE FROM book_covers WHERE id = 2", [])
            .unwrap();
        a_row_already_gone_is_said_and_the_rest_is_saved(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_writes_new_rows_before_it_deletes() {
        let (connection, _dir) = bookshop().await;
        a_new_row_goes_in_before_a_row_goes_out(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_changes_rows_before_it_deletes() {
        let (connection, _dir) = bookshop().await;
        a_row_is_changed_before_a_row_goes_out(&connection, &shop()).await;
    }

    /// 787 is SQLITE_CONSTRAINT_FOREIGNKEY. SQLite names neither the key
    /// nor the table that holds the row.
    #[tokio::test]
    async fn sqlite_keeps_a_row_a_foreign_key_holds() {
        let (connection, _dir) = bookshop().await;
        let error =
            a_row_a_foreign_key_holds_stays_and_the_save_is_undone(&connection, &shop()).await;
        let Error::Query { code, message, .. } = error else {
            panic!("a query error: {error:?}");
        };
        assert_eq!(code.as_deref(), Some("787"));
        assert_eq!(message, "FOREIGN KEY constraint failed");
    }

    #[tokio::test]
    async fn sqlite_deletes_by_a_key_of_two_columns() {
        let (connection, _dir) = bookshop().await;
        a_key_of_two_columns_finds_its_row(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_deletes_a_row_another_delete_took_with_it() {
        let (connection, _dir) = bookshop().await;
        a_row_the_save_took_with_another_is_deleted(&connection, &shop()).await;
    }

    /// A key the table spells otherwise finds its row on SQLite all the
    /// same, and the save cannot be sure it is the column it reads as. It
    /// is an error, never a row that is already gone, and the row stays.
    #[tokio::test]
    async fn sqlite_refuses_a_key_the_table_spells_otherwise() {
        let (connection, _dir) = bookshop().await;
        let covers = shop().table("book_covers");
        let changes = deleting(
            covers.clone(),
            vec![RowDelete {
                key: vec![("ID".to_owned(), Value::Int(2))],
            }],
        );
        let outcome = connection.write(&changes, &StopFlag::new()).await;
        assert!(
            matches!(
                &outcome,
                Err(Error::Query { message, .. }) if message.contains("spells ID another way")
            ),
            "{outcome:?}"
        );
        assert_eq!(firsts(&connection, covers).await, ["1", "2", "3"]);
    }
}

mod postgres {
    use super::*;

    /// Written for the tables that begin with `shop`: each test has a
    /// Bookshop of its own, whose tables begin with its own word.
    const DROP: &str =
        "DROP TABLE IF EXISTS shop_shelves, shop_book_authors, shop_book_covers, shop_publishers";

    const CREATE: &str = "
        CREATE TABLE shop_publishers (
            id bigint PRIMARY KEY,
            name text NOT NULL UNIQUE
        );
        INSERT INTO shop_publishers VALUES
            (9100000000000000001, 'Northlight Books'),
            (9100000000000000004, 'Harbor Press'),
            (9100000000000000007, 'Harbor Lights');
        CREATE TABLE shop_book_covers (
            id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            publisher_id bigint NOT NULL REFERENCES shop_publishers (id),
            kind varchar NOT NULL DEFAULT 'print'
        );
        INSERT INTO shop_book_covers (publisher_id, kind) VALUES
            (9100000000000000001, 'print'),
            (9100000000000000001, 'ebook'),
            (9100000000000000004, 'print');
        CREATE TABLE shop_book_authors (
            book_id bigint NOT NULL,
            author text NOT NULL,
            PRIMARY KEY (book_id, author)
        );
        INSERT INTO shop_book_authors VALUES
            (101, 'Ada Quill'), (101, 'Sam O''Neil'), (102, 'Ada Quill');
        CREATE TABLE shop_shelves (
            id bigint PRIMARY KEY,
            parent_id bigint REFERENCES shop_shelves (id) ON DELETE CASCADE
        );
        INSERT INTO shop_shelves VALUES (1, NULL), (2, 1), (3, NULL)";

    fn named(sql: &str, prefix: &str) -> String {
        sql.replace("shop_", &format!("{prefix}_"))
    }

    fn shop(prefix: &str) -> Shop {
        Shop {
            schema: "public".into(),
            publishers: format!("{prefix}_publishers"),
            covers: format!("{prefix}_book_covers"),
            authors: format!("{prefix}_book_authors"),
            shelves: format!("{prefix}_shelves"),
            id: "bigint",
            text: "text",
        }
    }

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_PG_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    /// The Bookshop whose tables begin with `prefix`, made anew, with a
    /// session of the test's own standing in for another program. `None`
    /// without a server.
    async fn bookshop(prefix: &str) -> Option<(tokio_postgres::Client, Connection)> {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return None;
        };
        let mut config: tokio_postgres::Config = url.parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        let (admin, session) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(session);
        admin.batch_execute(&named(DROP, prefix)).await.unwrap();
        admin.batch_execute(&named(CREATE, prefix)).await.unwrap();
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        let connection =
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::Writable)
                .await
                .unwrap();
        Some((admin, connection))
    }

    #[tokio::test]
    async fn postgres_deletes_a_row_by_its_key() {
        let Some((admin, connection)) = bookshop("pgdel").await else {
            return;
        };
        a_marked_row_is_deleted(&connection, &shop("pgdel")).await;
        admin.batch_execute(&named(DROP, "pgdel")).await.unwrap();
    }

    #[tokio::test]
    async fn postgres_says_a_row_was_already_gone_and_saves_the_rest() {
        let Some((admin, connection)) = bookshop("pggone").await else {
            return;
        };
        admin
            .batch_execute("DELETE FROM pggone_book_covers WHERE id = 2")
            .await
            .unwrap();
        a_row_already_gone_is_said_and_the_rest_is_saved(&connection, &shop("pggone")).await;
        admin.batch_execute(&named(DROP, "pggone")).await.unwrap();
    }

    #[tokio::test]
    async fn postgres_writes_new_rows_before_it_deletes() {
        let Some((admin, connection)) = bookshop("pgorder").await else {
            return;
        };
        a_new_row_goes_in_before_a_row_goes_out(&connection, &shop("pgorder")).await;
        admin.batch_execute(&named(DROP, "pgorder")).await.unwrap();
    }

    #[tokio::test]
    async fn postgres_changes_rows_before_it_deletes() {
        let Some((admin, connection)) = bookshop("pgfirst").await else {
            return;
        };
        a_row_is_changed_before_a_row_goes_out(&connection, &shop("pgfirst")).await;
        admin.batch_execute(&named(DROP, "pgfirst")).await.unwrap();
    }

    /// Spec 5: 23503, and the constraint that holds the row, which is
    /// what finds the table that refers to it.
    #[tokio::test]
    async fn postgres_keeps_a_row_a_foreign_key_holds() {
        let Some((admin, connection)) = bookshop("pgheld").await else {
            return;
        };
        let error =
            a_row_a_foreign_key_holds_stays_and_the_save_is_undone(&connection, &shop("pgheld"))
                .await;
        let Error::Query {
            code, named: of, ..
        } = error
        else {
            panic!("a query error: {error:?}");
        };
        assert_eq!(code.as_deref(), Some("23503"));
        assert_eq!(
            of.constraint.as_deref(),
            Some("pgheld_book_covers_publisher_id_fkey")
        );
        admin.batch_execute(&named(DROP, "pgheld")).await.unwrap();
    }

    #[tokio::test]
    async fn postgres_deletes_by_a_key_of_two_columns() {
        let Some((admin, connection)) = bookshop("pgtwo").await else {
            return;
        };
        a_key_of_two_columns_finds_its_row(&connection, &shop("pgtwo")).await;
        admin.batch_execute(&named(DROP, "pgtwo")).await.unwrap();
    }

    #[tokio::test]
    async fn postgres_deletes_a_row_another_delete_took_with_it() {
        let Some((admin, connection)) = bookshop("pgtree").await else {
            return;
        };
        a_row_the_save_took_with_another_is_deleted(&connection, &shop("pgtree")).await;
        admin.batch_execute(&named(DROP, "pgtree")).await.unwrap();
    }
}
