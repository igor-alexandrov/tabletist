//! Deleting rows in a save, on a Bookshop made in each engine: a row goes
//! by its key, a row someone deleted first is said and the rest is saved,
//! new rows go in and rows are changed before rows go out, and a row a
//! foreign key holds stays, with everything else the save did undone.
//!
//! "Spec N" is a section of
//! `docs/superpowers/specs/2026-10-12-deleting-rows-design.md`, "test N"
//! an entry of its Tests.
//!
//! Rows next to each other go in one statement, a run. "Many-rows spec N"
//! and "many-rows test N" are of
//! `docs/superpowers/specs/2026-10-13-deleting-many-rows-design.md`.
//!
//! PostgreSQL and MySQL need their servers (see compose.yaml). Without the
//! variables those tests print "skipped" and pass.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use mysql_async::prelude::Queryable;
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
    /// 300 rows numbered from 1, for a save of many.
    stock: String,
    /// Genres under genres, by a key that holds.
    genres: String,
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

/// The rows of the stock table, as the values of one `INSERT`: 300 rows,
/// numbered from 1.
fn stock_values() -> String {
    let rows: Vec<String> = (1..=300).map(|id| format!("({id})")).collect();
    rows.join(", ")
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

/// Many-rows spec 4 and test 15: 250 rows of one save go in three runs,
/// and are gone.
async fn many_rows_go_in_runs(connection: &Connection, shop: &Shop) {
    let stock = shop.table(&shop.stock);
    let deletes: Vec<RowDelete> = (1..=250).map(by_id).collect();
    let outcome = save(connection, &deleting(stock.clone(), deletes)).await;
    assert!(
        matches!(&outcome, WriteOutcome::Written { gone, .. } if gone.is_empty()),
        "{outcome:?}"
    );
    let left = firsts(connection, stock).await;
    assert_eq!(left.len(), 50);
    assert!(left.contains(&"251".to_owned()) && left.contains(&"300".to_owned()));
}

/// Many-rows test 15: rows 5 and 7 were deleted by someone else before
/// this is called. The save says which of its deletes they were, and
/// deletes the rest of their run.
async fn rows_already_gone_leave_the_rest_of_their_run(connection: &Connection, shop: &Shop) {
    let stock = shop.table(&shop.stock);
    let deletes: Vec<RowDelete> = (1..=10).map(by_id).collect();
    let outcome = save(connection, &deleting(stock.clone(), deletes)).await;
    let WriteOutcome::Written { gone, .. } = outcome else {
        panic!("the save wrote: {outcome:?}");
    };
    assert_eq!(gone, [4, 6]);
    assert_eq!(firsts(connection, stock).await.len(), 290);
}

/// Many-rows spec 4 and test 15: a run by a key of two columns finds its
/// rows.
async fn a_run_by_a_key_of_two_columns_finds_its_rows(connection: &Connection, shop: &Shop) {
    let authors = shop.table(&shop.authors);
    let by = |book: i64, author: &str| RowDelete {
        key: vec![
            ("book_id".to_owned(), Value::Int(book)),
            ("author".to_owned(), Value::Text(author.into())),
        ],
    };
    let changes = deleting(
        authors.clone(),
        vec![by(101, "Ada Quill"), by(101, "Sam O'Neil")],
    );
    let outcome = save(connection, &changes).await;
    assert!(
        matches!(&outcome, WriteOutcome::Written { gone, .. } if gone.is_empty()),
        "{outcome:?}"
    );
    assert_eq!(rows(connection, authors).await, [["102", "Ada Quill"]]);
}

/// Many-rows spec 4, step 6, and test 15: Crime (2) stands under Fiction
/// (1), by a key that holds. The app sends the child first, and the two
/// are one run. An engine that checks the key row by row refuses the run
/// and takes the rows one at a time; either way both are gone.
async fn a_row_and_the_row_it_points_at_go_in_one_run(connection: &Connection, shop: &Shop) {
    let genres = shop.table(&shop.genres);
    let outcome = save(
        connection,
        &deleting(genres.clone(), vec![by_id(2), by_id(1)]),
    )
    .await;
    assert!(
        matches!(&outcome, WriteOutcome::Written { gone, .. } if gone.is_empty()),
        "{outcome:?}"
    );
    assert_eq!(firsts(connection, genres).await, ["3"]);
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

    /// The Bookshop, with its stock of 300 rows and its genres.
    fn create() -> String {
        format!(
            "{CREATE};
             CREATE TABLE stock (id INTEGER PRIMARY KEY);
             INSERT INTO stock VALUES {};
             CREATE TABLE genres (
                 id INTEGER PRIMARY KEY,
                 parent_id INTEGER REFERENCES genres (id)
             );
             INSERT INTO genres VALUES (1, NULL), (2, 1), (3, NULL)",
            stock_values()
        )
    }

    fn shop() -> Shop {
        Shop {
            schema: "main".into(),
            publishers: "publishers".into(),
            covers: "book_covers".into(),
            authors: "book_authors".into(),
            shelves: "shelves".into(),
            stock: "stock".into(),
            genres: "genres".into(),
            id: "INTEGER",
            text: "TEXT",
        }
    }

    async fn bookshop() -> (Connection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(&create())
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
    async fn sqlite_deletes_many_rows_in_runs() {
        let (connection, _dir) = bookshop().await;
        many_rows_go_in_runs(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_says_which_rows_of_a_run_were_gone() {
        let (connection, dir) = bookshop().await;
        other_program(&dir)
            .execute_batch("DELETE FROM stock WHERE id IN (5, 7)")
            .unwrap();
        rows_already_gone_leave_the_rest_of_their_run(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_deletes_a_run_by_a_key_of_two_columns() {
        let (connection, _dir) = bookshop().await;
        a_run_by_a_key_of_two_columns_finds_its_rows(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_deletes_a_row_and_the_row_it_points_at_in_one_run() {
        let (connection, _dir) = bookshop().await;
        a_row_and_the_row_it_points_at_go_in_one_run(&connection, &shop()).await;
    }

    /// Many-rows spec 4, step 3, and test 16: a trigger keeps one row of
    /// a run. The save fails on that row, and nothing of it is written.
    #[tokio::test]
    async fn sqlite_fails_the_row_of_a_run_a_trigger_kept() {
        let (connection, dir) = bookshop().await;
        other_program(&dir)
            .execute_batch(
                "CREATE TRIGGER keep_seven BEFORE DELETE ON stock WHEN OLD.id = 7
                 BEGIN SELECT RAISE(IGNORE); END",
            )
            .unwrap();
        let stock = shop().table("stock");
        let deletes: Vec<RowDelete> = (5..=9).map(by_id).collect();
        let outcome = save(&connection, &deleting(stock.clone(), deletes)).await;
        let WriteOutcome::FailedDelete { delete, error } = outcome else {
            panic!("the save was to fail on a row to delete: {outcome:?}");
        };
        assert_eq!(delete, 2);
        assert!(error.to_string().contains("deleted 0 rows"), "{error}");
        assert_eq!(firsts(&connection, stock).await.len(), 300);
    }

    /// Many-rows spec 4, step 7, and test 16: a trigger rolls the
    /// transaction back under a run. Nothing more is sent, and the run's
    /// error is its first row's.
    #[tokio::test]
    async fn sqlite_sends_nothing_after_a_trigger_rolled_the_save_back() {
        let (connection, dir) = bookshop().await;
        other_program(&dir)
            .execute_batch(
                "CREATE TRIGGER stop_at_seven BEFORE DELETE ON stock WHEN OLD.id = 7
                 BEGIN SELECT RAISE(ROLLBACK, 'seven stays'); END",
            )
            .unwrap();
        let stock = shop().table("stock");
        let deletes: Vec<RowDelete> = (5..=9).map(by_id).collect();
        let outcome = save(&connection, &deleting(stock.clone(), deletes)).await;
        let WriteOutcome::FailedDelete { delete, error } = outcome else {
            panic!("the save was to fail on a row to delete: {outcome:?}");
        };
        assert_eq!(delete, 0);
        assert!(error.to_string().contains("seven stays"), "{error}");
        // Not one row went: had the rows been sent one at a time after
        // the rollback, each would have been written for good.
        assert_eq!(firsts(&connection, stock.clone()).await.len(), 300);
        // And the session saves again.
        let outcome = save(&connection, &deleting(stock, vec![by_id(1)])).await;
        assert!(
            matches!(outcome, WriteOutcome::Written { .. }),
            "{outcome:?}"
        );
    }

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
            stock: format!("{prefix}_stock"),
            genres: format!("{prefix}_genres"),
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

mod mysql {
    use super::*;

    const DROP: &str =
        "DROP TABLE IF EXISTS shop_shelves, shop_book_authors, shop_book_covers, shop_publishers";

    const CREATE: [&str; 8] = [
        "CREATE TABLE shop_publishers (
             id BIGINT PRIMARY KEY,
             name VARCHAR(64) NOT NULL UNIQUE
         )",
        "INSERT INTO shop_publishers VALUES
             (9100000000000000001, 'Northlight Books'),
             (9100000000000000004, 'Harbor Press'),
             (9100000000000000007, 'Harbor Lights')",
        "CREATE TABLE shop_book_covers (
             id BIGINT AUTO_INCREMENT PRIMARY KEY,
             publisher_id BIGINT NOT NULL,
             kind VARCHAR(64) NOT NULL DEFAULT 'print',
             FOREIGN KEY (publisher_id) REFERENCES shop_publishers (id)
         )",
        "INSERT INTO shop_book_covers (publisher_id, kind) VALUES
             (9100000000000000001, 'print'),
             (9100000000000000001, 'ebook'),
             (9100000000000000004, 'print')",
        "CREATE TABLE shop_book_authors (
             book_id BIGINT NOT NULL,
             author VARCHAR(64) NOT NULL,
             PRIMARY KEY (book_id, author)
         )",
        "INSERT INTO shop_book_authors VALUES
             (101, 'Ada Quill'), (101, 'Sam O''Neil'), (102, 'Ada Quill')",
        "CREATE TABLE shop_shelves (
             id BIGINT PRIMARY KEY,
             parent_id BIGINT,
             FOREIGN KEY (parent_id) REFERENCES shop_shelves (id) ON DELETE CASCADE
         )",
        "INSERT INTO shop_shelves VALUES (1, NULL), (2, 1), (3, NULL)",
    ];

    fn named(sql: &str, prefix: &str) -> String {
        sql.replace("shop_", &format!("{prefix}_"))
    }

    fn shop(prefix: &str) -> Shop {
        Shop {
            schema: "tabletist".into(),
            publishers: format!("{prefix}_publishers"),
            covers: format!("{prefix}_book_covers"),
            authors: format!("{prefix}_book_authors"),
            shelves: format!("{prefix}_shelves"),
            stock: format!("{prefix}_stock"),
            genres: format!("{prefix}_genres"),
            id: "bigint",
            text: "varchar(64)",
        }
    }

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_MYSQL_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    /// The Bookshop whose tables begin with `prefix`, made anew, with a
    /// session of the test's own standing in for another program. `None`
    /// without a server.
    async fn bookshop(prefix: &str) -> Option<(mysql_async::Conn, Connection)> {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
            return None;
        };
        let opts = mysql_async::Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
        admin.query_drop(named(DROP, prefix)).await.unwrap();
        for statement in CREATE {
            admin.query_drop(named(statement, prefix)).await.unwrap();
        }
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        let connection =
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::Writable)
                .await
                .unwrap();
        Some((admin, connection))
    }

    #[tokio::test]
    async fn mysql_deletes_a_row_by_its_key() {
        let Some((mut admin, connection)) = bookshop("mydel").await else {
            return;
        };
        a_marked_row_is_deleted(&connection, &shop("mydel")).await;
        admin.query_drop(named(DROP, "mydel")).await.unwrap();
    }

    #[tokio::test]
    async fn mysql_says_a_row_was_already_gone_and_saves_the_rest() {
        let Some((mut admin, connection)) = bookshop("mygone").await else {
            return;
        };
        admin
            .query_drop("DELETE FROM mygone_book_covers WHERE id = 2")
            .await
            .unwrap();
        a_row_already_gone_is_said_and_the_rest_is_saved(&connection, &shop("mygone")).await;
        admin.query_drop(named(DROP, "mygone")).await.unwrap();
    }

    #[tokio::test]
    async fn mysql_writes_new_rows_before_it_deletes() {
        let Some((mut admin, connection)) = bookshop("myorder").await else {
            return;
        };
        a_new_row_goes_in_before_a_row_goes_out(&connection, &shop("myorder")).await;
        admin.query_drop(named(DROP, "myorder")).await.unwrap();
    }

    #[tokio::test]
    async fn mysql_changes_rows_before_it_deletes() {
        let Some((mut admin, connection)) = bookshop("myfirst").await else {
            return;
        };
        a_row_is_changed_before_a_row_goes_out(&connection, &shop("myfirst")).await;
        admin.query_drop(named(DROP, "myfirst")).await.unwrap();
    }

    /// Spec 5: MySQL's 1451, under the SQLSTATE 23000 it shares with other
    /// failures, and the constraint that holds the row.
    #[tokio::test]
    async fn mysql_keeps_a_row_a_foreign_key_holds() {
        let Some((mut admin, connection)) = bookshop("myheld").await else {
            return;
        };
        let error =
            a_row_a_foreign_key_holds_stays_and_the_save_is_undone(&connection, &shop("myheld"))
                .await;
        let Error::Query {
            code, named: of, ..
        } = error
        else {
            panic!("a query error: {error:?}");
        };
        assert_eq!(code.as_deref(), Some("23000"));
        assert_eq!(of.number, Some(1451));
        assert_eq!(of.constraint.as_deref(), Some("myheld_book_covers_ibfk_1"));
        assert!(of.columns.is_empty());
        admin.query_drop(named(DROP, "myheld")).await.unwrap();
    }

    #[tokio::test]
    async fn mysql_deletes_by_a_key_of_two_columns() {
        let Some((mut admin, connection)) = bookshop("mytwo").await else {
            return;
        };
        a_key_of_two_columns_finds_its_row(&connection, &shop("mytwo")).await;
        admin.query_drop(named(DROP, "mytwo")).await.unwrap();
    }

    #[tokio::test]
    async fn mysql_deletes_a_row_another_delete_took_with_it() {
        let Some((mut admin, connection)) = bookshop("mytree").await else {
            return;
        };
        a_row_the_save_took_with_another_is_deleted(&connection, &shop("mytree")).await;
        admin.query_drop(named(DROP, "mytree")).await.unwrap();
    }
}
