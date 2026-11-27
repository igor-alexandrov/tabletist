//! Editing a Files connection's `.duckdb` files in place, through the
//! public `Connection` API, on the Bookshop's exports
//! (`fixtures::write_bookshop_files`): `bookshop.duckdb` holds
//! `order_items`, keyed by `id`, and `book_covers`, with no key. Every
//! session attaches the file read-only; a save writes it alone, in one
//! transaction, and every session reads it as the save left it.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

use tabletist_db::files::Source;
use tabletist_db::{
    Access, CellChange, ChangeSet, ConnectSpec, Connection, Error, FILE_COPY, FILE_POSITION,
    HostKeys, InsertValue, NewValue, ObjectRef, RowChange, RowDelete, RowInsert, RowPage, RowQuery,
    Secrets, StopFlag, Unsaved, Value, WriteOutcome,
};

/// The Bookshop's exports in a folder of their own, and a session over it
/// that may write.
async fn editable() -> (Connection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
    let connection = open(dir.path(), Access::Writable).await;
    (connection, dir)
}

async fn open(folder: &Path, access: Access) -> Connection {
    Connection::connect_with(
        &ConnectSpec::files(Source::Folder(folder.to_owned())),
        &Secrets::default(),
        &HostKeys::default(),
        access,
    )
    .await
    .unwrap()
}

fn shop(name: &str) -> ObjectRef {
    ObjectRef::new("bookshop", name)
}

async fn page(connection: &Connection, name: &str) -> RowPage {
    connection
        .fetch_rows(&RowQuery::new(shop(name), 100))
        .await
        .unwrap()
}

fn set(name: &str) -> ChangeSet {
    ChangeSet {
        object: shop(name),
        inserts: Vec::new(),
        rows: Vec::new(),
        deletes: Vec::new(),
    }
}

/// What finds `order_items`' row `id`.
fn by_id(id: i64) -> Vec<(String, Value)> {
    vec![("id".to_owned(), Value::Int(id))]
}

/// What finds `book_covers`' row at `place`, as `page` has its places.
fn at(page: &RowPage, place: i64) -> Vec<(String, Value)> {
    let copy = page.positions.as_ref().unwrap().copy;
    vec![
        (FILE_POSITION.to_owned(), Value::Int(place)),
        (FILE_COPY.to_owned(), Value::Int(copy)),
    ]
}

/// A change of one cell of the row `key` finds.
fn change(
    key: Vec<(String, Value)>,
    column: &str,
    type_name: &str,
    loaded: Value,
    new: &str,
) -> RowChange {
    RowChange {
        key,
        set: vec![CellChange {
            column: column.to_owned(),
            type_name: type_name.to_owned(),
            loaded,
            new: NewValue::Text(new.to_owned()),
        }],
    }
}

async fn save(connection: &Connection, changes: &ChangeSet) -> tabletist_db::Result<WriteOutcome> {
    connection.write(changes, &StopFlag::new()).await
}

fn unsaved(error: &Error) -> Option<Unsaved> {
    match error {
        Error::Query { named, .. } => named.unsaved.clone(),
        _ => None,
    }
}

/// The column `name` of `page`'s rows.
fn column(page: &RowPage, name: &str) -> Vec<Value> {
    let at = page
        .columns
        .iter()
        .position(|column| column.name == name)
        .unwrap();
    page.rows.iter().map(|row| row[at].clone()).collect()
}

#[tokio::test]
async fn a_keyed_table_is_found_by_its_key_and_saved_in_place() {
    let (connection, dir) = editable().await;
    let items = connection.describe(&shop("order_items")).await.unwrap();
    assert_eq!(items.primary_key, vec!["id".to_owned()]);
    assert_eq!(items.unedited, None);
    let before = page(&connection, "order_items").await;
    assert!(before.positions.is_none());
    assert!(before.ordered_by_key);
    let quantity = column(&before, "quantity")[1].clone();
    let mut changes = set("order_items");
    changes
        .rows
        .push(change(by_id(2), "quantity", "INTEGER", quantity, "5"));
    changes.deletes.push(RowDelete { key: by_id(3) });
    changes.inserts.push(RowInsert {
        set: ["id", "order_id", "book_id", "quantity", "price"]
            .iter()
            .zip(["100", "1", "2", "1", "12.00"])
            .map(|(name, value)| InsertValue {
                column: (*name).to_owned(),
                type_name: String::new(),
                new: NewValue::Text(value.to_owned()),
            })
            .collect(),
    });
    let outcome = save(&connection, &changes).await.unwrap();
    let WriteOutcome::Written { rows, gone, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    assert!(gone.is_empty());
    assert_eq!(rows.len(), 1);
    // In the file itself: a session of its own reads it, read-only.
    drop(connection);
    let reader = open(dir.path(), Access::ReadOnly).await;
    let after = page(&reader, "order_items").await;
    let ids = column(&after, "id");
    assert!(ids.contains(&Value::Int(100)));
    assert!(!ids.contains(&Value::Int(3)));
    let at = ids.iter().position(|id| *id == Value::Int(2)).unwrap();
    assert_eq!(column(&after, "quantity")[at], Value::Int(5));
    // Nothing beside it: no hidden file, no log left.
    let mut names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("bookshop") || name.starts_with('.'))
        .collect();
    names.sort();
    assert_eq!(names, vec!["bookshop.duckdb".to_owned()]);
}

#[tokio::test]
async fn a_table_with_no_key_is_found_by_rowid_and_read_again_after_a_save() {
    let (connection, _dir) = editable().await;
    let covers = connection.describe(&shop("book_covers")).await.unwrap();
    assert!(covers.primary_key.is_empty());
    let before = page(&connection, "book_covers").await;
    let positions = before.positions.clone().unwrap();
    assert_eq!(positions.rows, vec![0, 1, 2]);
    // The place isn't one of the page's columns.
    assert!(
        before
            .columns
            .iter()
            .all(|column| column.name != FILE_POSITION)
    );
    let kind = column(&before, "kind")[1].clone();
    let mut changes = set("book_covers");
    changes.deletes.push(RowDelete {
        key: at(&before, 0),
    });
    changes
        .rows
        .push(change(at(&before, 1), "kind", "VARCHAR", kind, "spine"));
    assert!(matches!(
        save(&connection, &changes).await.unwrap(),
        WriteOutcome::Written { .. }
    ));
    let after = page(&connection, "book_covers").await;
    assert_eq!(column(&after, "kind")[0], Value::Text("spine".into()));
    assert_eq!(after.rows.len(), 2);
    let renumbered = after.positions.unwrap();
    assert_ne!(renumbered.copy, positions.copy);
    // The old page's places are of the file before the save.
    let mut again = set("book_covers");
    again.deletes.push(RowDelete {
        key: at(&before, 2),
    });
    let error = save(&connection, &again).await.unwrap_err();
    assert_eq!(
        unsaved(&error),
        Some(Unsaved::Reread("bookshop.duckdb".into()))
    );
}

#[tokio::test]
async fn a_list_of_a_table_with_no_key_is_saved_though_duckdb_moves_its_row() {
    let dir = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
    // DuckDB updates a list column, or a unique one, as a delete and an
    // insert: the row has another `rowid` until the commit.
    {
        let duck = duckdb::Connection::open(dir.path().join("bookshop.duckdb")).unwrap();
        duck.execute_batch(
            "CREATE TABLE book_tags AS \
             SELECT book_id, [kind] AS tags FROM book_covers ORDER BY id",
        )
        .unwrap();
    }
    let connection = open(dir.path(), Access::Writable).await;
    let before = page(&connection, "book_tags").await;
    let tags = column(&before, "tags")[1].clone();
    let mut changes = set("book_tags");
    changes.rows.push(change(
        at(&before, 1),
        "tags",
        "VARCHAR[]",
        tags,
        "['spine']",
    ));
    let outcome = save(&connection, &changes).await.unwrap();
    let WriteOutcome::Written { rows, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    // No row handed back: the page is read again.
    assert!(rows.is_empty());
    // The row moved: DuckDB puts the row it wrote last.
    let after = page(&connection, "book_tags").await;
    let book = column(&before, "book_id")[1].clone();
    let at = column(&after, "book_id")
        .iter()
        .rposition(|id| *id == book)
        .unwrap();
    // A list reads as text.
    assert_eq!(
        column(&after, "tags")[at],
        Value::Text("[\"spine\"]".into())
    );
    assert_eq!(after.rows.len(), before.rows.len());
}

#[tokio::test]
async fn a_table_with_a_column_named_rowid_is_not_found_by_it() {
    let dir = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
    // Its own `rowid` is what `rowid` reads: two rows of one value.
    {
        let duck = duckdb::Connection::open(dir.path().join("bookshop.duckdb")).unwrap();
        duck.execute_batch("CREATE TABLE cover_kinds AS SELECT 7 AS rowid, kind FROM book_covers")
            .unwrap();
    }
    let before = std::fs::read(dir.path().join("bookshop.duckdb")).unwrap();
    let connection = open(dir.path(), Access::Writable).await;
    let kinds = page(&connection, "cover_kinds").await;
    assert!(kinds.positions.is_none());
    // Nor is a save of rows by places.
    let mut changes = set("cover_kinds");
    changes.deletes.push(RowDelete {
        key: vec![
            (FILE_POSITION.to_owned(), Value::Int(7)),
            (FILE_COPY.to_owned(), Value::Int(1)),
        ],
    });
    assert!(save(&connection, &changes).await.is_err());
    drop(connection);
    assert_eq!(
        std::fs::read(dir.path().join("bookshop.duckdb")).unwrap(),
        before
    );
}

#[tokio::test]
async fn a_table_with_no_key_is_filtered_by_its_own_name() {
    let (connection, _dir) = editable().await;
    let mut query = RowQuery::new(shop("book_covers"), 100);
    query.raw_where = Some("book_covers.id > 1".to_owned());
    let filtered = connection.fetch_rows(&query).await.unwrap();
    assert!(filtered.positions.is_some());
    let ids = column(&filtered, "id");
    assert!(!ids.is_empty());
    assert!(ids.iter().all(|id| matches!(id, Value::Int(id) if *id > 1)));
}

#[tokio::test]
async fn a_change_found_by_other_than_the_key_is_not_saved() {
    let (connection, _dir) = editable().await;
    let before = page(&connection, "order_items").await;
    // An order of more than one item: two rows of one order_id.
    let orders = column(&before, "order_id");
    let order = orders
        .iter()
        .find(|order| orders.iter().filter(|other| other == order).count() > 1)
        .unwrap()
        .clone();
    let mut changes = set("order_items");
    changes.deletes.push(RowDelete {
        key: vec![("order_id".to_owned(), order)],
    });
    assert!(save(&connection, &changes).await.is_err());
    assert_eq!(page(&connection, "order_items").await.rows.len(), 40);
}

#[tokio::test]
async fn a_page_read_before_the_file_was_closed_is_not_saved_after() {
    let (connection, dir) = editable().await;
    let before = page(&connection, "book_covers").await;
    drop(connection);
    // Another program deletes the first row: the others move up a place.
    {
        let duck = duckdb::Connection::open(dir.path().join("bookshop.duckdb")).unwrap();
        duck.execute_batch("DELETE FROM book_covers WHERE rowid = 0; CHECKPOINT")
            .unwrap();
    }
    let connection = open(dir.path(), Access::Writable).await;
    let mut changes = set("book_covers");
    changes.deletes.push(RowDelete {
        key: at(&before, 1),
    });
    let error = save(&connection, &changes).await.unwrap_err();
    assert_eq!(
        unsaved(&error),
        Some(Unsaved::Reread("bookshop.duckdb".into()))
    );
    assert_eq!(page(&connection, "book_covers").await.rows.len(), 2);
}

#[tokio::test]
async fn a_keyed_table_is_exported_in_its_key_order() {
    let (connection, _dir) = editable().await;
    let statement = connection
        .export_statement(&RowQuery::new(shop("order_items"), 100), &[])
        .await
        .unwrap();
    assert!(statement.contains("ORDER BY \"id\""), "{statement}");
}

// On Windows another session's hold keeps a save from opening the file
// to write: see `on_windows_a_file_another_session_has_open_is_not_written`.
#[cfg(not(windows))]
#[tokio::test]
async fn another_session_reads_the_file_as_the_save_left_it() {
    let (connection, dir) = editable().await;
    let reader = open(dir.path(), Access::ReadOnly).await;
    let before = page(&reader, "order_items").await;
    let quantity = column(&before, "quantity")[0].clone();
    let mut changes = set("order_items");
    changes
        .rows
        .push(change(by_id(1), "quantity", "INTEGER", quantity, "9"));
    save(&connection, &changes).await.unwrap();
    let after = page(&reader, "order_items").await;
    assert_eq!(column(&after, "quantity")[0], Value::Int(9));
}

#[cfg(windows)]
#[tokio::test]
async fn on_windows_a_file_another_session_has_open_is_not_written() {
    let (connection, dir) = editable().await;
    let reader = open(dir.path(), Access::ReadOnly).await;
    page(&reader, "order_items").await;
    let mut changes = set("order_items");
    changes.deletes.push(RowDelete { key: by_id(1) });
    let error = save(&connection, &changes).await.unwrap_err();
    assert_eq!(
        unsaved(&error),
        Some(Unsaved::OpenHere("bookshop.duckdb".into()))
    );
    // Saved once that session is gone.
    drop(reader);
    assert!(matches!(
        save(&connection, &changes).await.unwrap(),
        WriteOutcome::Written { .. }
    ));
}

// On Windows another session's hold keeps a save from opening the file.
#[cfg(not(windows))]
#[tokio::test]
async fn a_row_another_session_saved_since_it_was_loaded_is_a_conflict() {
    let (one, dir) = editable().await;
    let other = open(dir.path(), Access::Writable).await;
    let loaded = page(&one, "order_items").await;
    let quantity = column(&loaded, "quantity")[1].clone();
    let mut theirs = set("order_items");
    theirs.rows.push(change(
        by_id(2),
        "quantity",
        "INTEGER",
        quantity.clone(),
        "9",
    ));
    save(&other, &theirs).await.unwrap();
    let mut mine = set("order_items");
    mine.rows
        .push(change(by_id(2), "quantity", "INTEGER", quantity, "5"));
    let outcome = save(&one, &mine).await.unwrap();
    let WriteOutcome::Conflicts(conflicts) = &outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].row, 0);
    // The row as the file holds it, with the other session's value.
    let server = conflicts[0].server.as_ref().unwrap();
    assert!(server.contains(&Value::Int(9)));
    let after = page(&one, "order_items").await;
    assert_eq!(column(&after, "quantity")[1], Value::Int(9));
}

#[tokio::test]
async fn a_failed_change_writes_nothing() {
    let (connection, dir) = editable().await;
    let before = std::fs::read(dir.path().join("bookshop.duckdb")).unwrap();
    // id 1 is taken: the second insert fails, and the first, written
    // before it, is undone.
    let mut changes = set("order_items");
    for id in ["200", "1"] {
        changes.inserts.push(RowInsert {
            set: vec![InsertValue {
                column: "id".to_owned(),
                type_name: String::new(),
                new: NewValue::Text(id.to_owned()),
            }],
        });
    }
    changes
        .rows
        .push(change(by_id(2), "quantity", "INTEGER", Value::Int(3), "7"));
    let outcome = save(&connection, &changes).await.unwrap();
    assert!(
        matches!(outcome, WriteOutcome::FailedInsert { insert: 1, .. }),
        "{outcome:?}"
    );
    // Undone in this session, before it lets go of the file.
    let after = page(&connection, "order_items").await;
    assert!(!column(&after, "id").contains(&Value::Int(200)));
    let at = column(&after, "id")
        .iter()
        .position(|id| *id == Value::Int(2))
        .unwrap();
    assert_eq!(column(&after, "quantity")[at], Value::Int(3));
    drop(connection);
    assert_eq!(
        std::fs::read(dir.path().join("bookshop.duckdb")).unwrap(),
        before
    );
}

#[tokio::test]
async fn a_session_that_reads_saves_nothing() {
    let dir = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
    let connection = open(dir.path(), Access::ReadOnly).await;
    let mut changes = set("order_items");
    changes.deletes.push(RowDelete { key: by_id(1) });
    assert_eq!(
        save(&connection, &changes).await.unwrap_err(),
        Error::ReadOnly
    );
    // And its pages carry no places.
    assert!(page(&connection, "book_covers").await.positions.is_none());
}

// Windows renames no file over one a program has open.
#[cfg(unix)]
#[tokio::test]
async fn a_file_replaced_on_disk_is_not_written() {
    let (connection, dir) = editable().await;
    page(&connection, "order_items").await;
    // Another program puts a file of its own in its place.
    let other = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(other.path()).unwrap();
    let theirs = other.path().join("bookshop.duckdb");
    {
        let duck = duckdb::Connection::open(&theirs).unwrap();
        duck.execute_batch("INSERT INTO book_covers SELECT * FROM book_covers")
            .unwrap();
    }
    std::fs::rename(&theirs, dir.path().join("bookshop.duckdb")).unwrap();
    let replaced = std::fs::read(dir.path().join("bookshop.duckdb")).unwrap();
    let mut changes = set("order_items");
    changes.deletes.push(RowDelete { key: by_id(1) });
    let error = save(&connection, &changes).await.unwrap_err();
    assert_eq!(
        unsaved(&error),
        Some(Unsaved::Changed("bookshop.duckdb".into()))
    );
    assert_eq!(
        std::fs::read(dir.path().join("bookshop.duckdb")).unwrap(),
        replaced
    );
    // Read again, as Reload does: the new file is read, and saved to.
    let covers = page(&connection, "book_covers").await;
    assert_eq!(covers.rows.len(), 6);
    assert!(matches!(
        save(&connection, &changes).await.unwrap(),
        WriteOutcome::Written { .. }
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn a_file_its_owner_may_not_write_is_not_written() {
    use std::os::unix::fs::PermissionsExt;
    let (connection, dir) = editable().await;
    let path = dir.path().join("bookshop.duckdb");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
    let mut changes = set("order_items");
    changes.deletes.push(RowDelete { key: by_id(1) });
    let error = save(&connection, &changes).await.unwrap_err();
    assert_eq!(
        unsaved(&error),
        Some(Unsaved::Protected("bookshop.duckdb".into()))
    );
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
}

/// Run by `a_file_another_program_has_open_is_not_written` as the other
/// program: holds the file at `TABLETIST_HOLD` open, read-only, until its
/// input ends. Does nothing in a plain test run.
#[test]
fn hold_the_file() {
    let Ok(path) = std::env::var("TABLETIST_HOLD") else {
        return;
    };
    let path = path.replace('\'', "''");
    let duck = duckdb::Connection::open_in_memory().unwrap();
    duck.execute_batch(&format!("ATTACH '{path}' AS held (READ_ONLY)"))
        .unwrap();
    println!("held");
    let mut line = String::new();
    while std::io::stdin().read_line(&mut line).unwrap() > 0 {}
}

#[tokio::test]
async fn a_file_another_program_has_open_is_not_written() {
    let (connection, dir) = editable().await;
    let path = dir.path().join("bookshop.duckdb");
    let mut other = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "hold_the_file",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("TABLETIST_HOLD", &path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut said = BufReader::new(other.stdout.take().unwrap()).lines();
    // The test runner writes its own words on the line before it.
    assert!(said.any(|line| line.unwrap().ends_with("held")));
    let mut changes = set("order_items");
    changes.deletes.push(RowDelete { key: by_id(1) });
    let error = save(&connection, &changes).await.unwrap_err();
    assert_eq!(
        unsaved(&error),
        Some(Unsaved::Busy("bookshop.duckdb".into()))
    );
    drop(other.stdin.take());
    other.wait().unwrap();
    // Still read, read-only, and saved once the other program lets go.
    assert_eq!(page(&connection, "order_items").await.rows.len(), 40);
    assert!(matches!(
        save(&connection, &changes).await.unwrap(),
        WriteOutcome::Written { .. }
    ));
}
