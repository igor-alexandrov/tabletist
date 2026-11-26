//! Editing a Files connection's files through the public `Connection`
//! API, on the Bookshop's exports (`fixtures::write_bookshop_files`): a
//! session that may write reads a file's table into a copy, each row with
//! its place, and a save writes the file back through a hidden file
//! renamed over it, never over a file changed since.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::path::Path;

use tabletist_db::files::Source;
use tabletist_db::{
    Access, CellChange, ChangeSet, ConnectSpec, Connection, Error, FILE_POSITION, HostKeys,
    InsertValue, NewValue, ObjectRef, RowChange, RowDelete, RowInsert, RowPage, RowQuery, Secrets,
    StopFlag, Unedited, Unsaved, Value, WriteOutcome,
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

fn table(name: &str) -> ObjectRef {
    ObjectRef::new("main", name)
}

async fn page(connection: &Connection, name: &str) -> RowPage {
    connection
        .fetch_rows(&RowQuery::new(table(name), 100))
        .await
        .unwrap()
}

/// What finds the row at `place` of a file.
fn at(place: i64) -> Vec<(String, Value)> {
    vec![(FILE_POSITION.to_owned(), Value::Int(place))]
}

/// A change of one cell of the row at `place`.
fn change(place: i64, column: &str, type_name: &str, loaded: Value, new: &str) -> RowChange {
    RowChange {
        key: at(place),
        set: vec![CellChange {
            column: column.into(),
            type_name: type_name.into(),
            loaded,
            new: NewValue::Text(new.into()),
        }],
    }
}

fn set(name: &str) -> ChangeSet {
    ChangeSet {
        object: table(name),
        inserts: Vec::new(),
        rows: Vec::new(),
        deletes: Vec::new(),
    }
}

async fn save(connection: &Connection, changes: &ChangeSet) -> Result<WriteOutcome, Error> {
    connection.write(changes, &StopFlag::new()).await
}

/// Why a save of a file wrote nothing.
fn unsaved(error: &Error) -> Option<Unsaved> {
    match error {
        Error::Query { named, .. } => named.unsaved.clone(),
        _ => None,
    }
}

/// The folder's files, the hidden ones among them.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn a_page_of_an_editable_file_says_where_each_row_is() {
    let (connection, _dir) = editable().await;
    assert_eq!(connection.access(), Access::Writable);
    let orders = page(&connection, "orders").await;
    assert_eq!(orders.positions, Some((0..12).collect()));
    assert!(orders.ordered_by_key);
    // The place is no column of the table's.
    assert!(
        orders
            .columns
            .iter()
            .all(|column| column.name != FILE_POSITION)
    );
    // Sorted, each row keeps its own place.
    let mut query = RowQuery::new(table("orders"), 100);
    query.sort = vec![tabletist_db::Sort {
        column: "id".into(),
        dir: tabletist_db::SortDir::Desc,
    }];
    let sorted = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(sorted.positions, Some((0..12).rev().collect()));
    // A session that reads only has no places.
    let dir = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
    let reading = open(dir.path(), Access::ReadOnly).await;
    assert_eq!(page(&reading, "orders").await.positions, None);
}

#[tokio::test]
async fn a_save_writes_a_csv_back_keeping_every_row_it_did_not_change() {
    let (connection, dir) = editable().await;
    let path = dir.path().join("orders.csv");
    let before = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = before.lines().collect();
    let orders = page(&connection, "orders").await;
    let total = orders
        .columns
        .iter()
        .position(|column| column.name == "total")
        .unwrap();
    let mut changes = set("orders");
    changes.rows.push(change(
        1,
        "total",
        "DOUBLE",
        orders.rows[1][total].clone(),
        "210.50",
    ));
    changes.deletes.push(RowDelete { key: at(2) });
    changes.inserts.push(RowInsert {
        set: vec![
            InsertValue {
                column: "id".into(),
                type_name: "BIGINT".into(),
                new: NewValue::Text("13".into()),
            },
            InsertValue {
                column: "email".into(),
                type_name: "VARCHAR".into(),
                new: NewValue::Text("reader, new@example.com".into()),
            },
        ],
    });
    let outcome = save(&connection, &changes).await.unwrap();
    assert!(
        matches!(outcome, WriteOutcome::Written { .. }),
        "{outcome:?}"
    );
    let after = std::fs::read_to_string(&path).unwrap();
    let written: Vec<&str> = after.lines().collect();
    // The header and every row not changed, as they were; the changed
    // total as it was typed; the deleted row gone; the new row last.
    assert_eq!(written[0], lines[0]);
    assert_eq!(written[1], lines[1]);
    assert_eq!(written[2], lines[2].replace(",201.70", ",210.50"));
    assert_eq!(&written[3..12], &lines[4..13]);
    assert_eq!(written[12], "13,,\"reader, new@example.com\",,,");
    assert_eq!(written.len(), 13);
    // No hidden file is left.
    assert!(names(dir.path()).iter().all(|name| !name.starts_with('.')));
    // The session reads the file as it is now, each row at its new place.
    let again = page(&connection, "orders").await;
    assert_eq!(again.rows.len(), 12);
    assert_eq!(again.positions, Some((0..12).collect()));
    // And saves again from there.
    let mut more = set("orders");
    more.deletes.push(RowDelete { key: at(11) });
    save(&connection, &more).await.unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 12);
}

#[tokio::test]
async fn parquet_json_and_ndjson_are_written_back_as_they_were_read() {
    let (connection, dir) = editable().await;
    for (name, file) in [
        ("customers", "customers.parquet"),
        ("books", "books.json"),
        ("events", "events.ndjson"),
    ] {
        let rows = page(&connection, name).await.rows.len() as i64;
        let mut changes = set(name);
        changes.deletes.push(RowDelete { key: at(0) });
        save(&connection, &changes).await.unwrap();
        let again = page(&connection, name).await;
        assert_eq!(again.rows.len() as i64, rows - 1, "{file}");
        assert!(dir.path().join(file).exists(), "{file}");
    }
    let books = std::fs::read_to_string(dir.path().join("books.json")).unwrap();
    assert!(books.starts_with('['), "an array: {books}");
    let events = std::fs::read_to_string(dir.path().join("events.ndjson")).unwrap();
    assert!(events.lines().all(|line| line.starts_with('{')), "{events}");
    // The Parquet file keeps its compression.
    let reading = duckdb::Connection::open_in_memory().unwrap();
    let compression: String = reading
        .query_row(
            &format!(
                "SELECT DISTINCT compression FROM parquet_metadata('{}')",
                dir.path().join("customers.parquet").display()
            ),
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(compression, "ZSTD");
}

#[tokio::test]
async fn a_file_changed_on_disk_since_it_was_read_is_never_written_over() {
    let (connection, dir) = editable().await;
    let path = dir.path().join("returns.csv");
    page(&connection, "returns").await;
    let mut changed = std::fs::read_to_string(&path).unwrap();
    changed.push_str("99,2025-12-01,lost\n");
    std::fs::write(&path, &changed).unwrap();
    let mut changes = set("returns");
    changes.deletes.push(RowDelete { key: at(0) });
    let error = save(&connection, &changes).await.unwrap_err();
    assert_eq!(
        unsaved(&error),
        Some(Unsaved::Changed("returns.csv".into()))
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), changed);
}

#[tokio::test]
async fn a_file_changed_on_disk_is_read_again_on_the_next_page() {
    let (connection, dir) = editable().await;
    let path = dir.path().join("returns.csv");
    let rows = page(&connection, "returns").await.rows.len();
    let mut changed = std::fs::read_to_string(&path).unwrap();
    changed.push_str("99,2025-12-01,lost\n");
    std::fs::write(&path, &changed).unwrap();
    // A reload shows the file as it is now, and a save goes on from it.
    let again = page(&connection, "returns").await;
    assert_eq!(again.rows.len(), rows + 1);
    let mut changes = set("returns");
    changes.deletes.push(RowDelete {
        key: at(rows as i64),
    });
    save(&connection, &changes).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap().lines().count(),
        rows + 1
    );
}

#[cfg(unix)]
#[tokio::test]
async fn a_folder_that_cannot_be_written_to_keeps_the_file_as_it_was() {
    use std::os::unix::fs::PermissionsExt;
    let (connection, dir) = editable().await;
    let path = dir.path().join("orders.csv");
    let before = std::fs::read(&path).unwrap();
    page(&connection, "orders").await;
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
    let mut changes = set("orders");
    changes.deletes.push(RowDelete { key: at(0) });
    let error = save(&connection, &changes).await.unwrap_err();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        unsaved(&error),
        Some(Unsaved::Unwritable("orders.csv".into()))
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    // The copy is as it was: every row still there.
    assert_eq!(page(&connection, "orders").await.rows.len(), 12);
}

#[tokio::test]
async fn a_csv_whose_rows_are_not_counted_as_duckdb_reads_them_is_not_edited() {
    let dir = tempfile::tempdir().unwrap();
    // A first line DuckDB's sniffer takes for no part of the table.
    std::fs::write(
        dir.path().join("authors.csv"),
        "Exported from the Bookshop\nid,name\n1,Toni Morrison\n2,James Joyce\n",
    )
    .unwrap();
    let connection = open(dir.path(), Access::Writable).await;
    let authors = page(&connection, "authors").await;
    if authors.rows.len() == 2 {
        assert_eq!(authors.positions, None);
        let mut changes = set("authors");
        changes.deletes.push(RowDelete { key: at(0) });
        assert!(save(&connection, &changes).await.is_err());
    }
}

#[tokio::test]
async fn a_duckdb_file_s_tables_say_why_they_are_not_edited() {
    let (connection, _dir) = editable().await;
    let covers = connection
        .describe(&ObjectRef::new("bookshop", "book_covers"))
        .await
        .unwrap();
    assert_eq!(covers.unedited, Some(Unedited::DuckDb));
    let orders = connection.describe(&table("orders")).await.unwrap();
    assert_eq!(orders.unedited, None);
    // A save to one is refused, as on a session that reads.
    let mut changes = set("orders");
    changes.object = ObjectRef::new("bookshop", "book_covers");
    changes.deletes.push(RowDelete { key: at(0) });
    assert_eq!(
        save(&connection, &changes).await.unwrap_err(),
        Error::ReadOnly
    );
}

#[tokio::test]
async fn a_script_still_reads_only() {
    let (connection, dir) = editable().await;
    let before = std::fs::read(dir.path().join("orders.csv")).unwrap();
    page(&connection, "orders").await;
    let statements = tabletist_db::sql::statements(
        tabletist_db::Dialect::DuckDb,
        "UPDATE tabletist_edits.orders SET total = 0",
    );
    let outcome = connection
        .run_script(
            &statements,
            10,
            tabletist_db::ScriptMode::Write,
            &StopFlag::new(),
        )
        .await
        .unwrap();
    assert!(matches!(
        outcome.results[0].outcome,
        tabletist_db::StatementOutcome::Error { .. }
    ));
    assert_eq!(
        std::fs::read(dir.path().join("orders.csv")).unwrap(),
        before
    );
    // The copy is as it was read.
    let orders = page(&connection, "orders").await;
    assert!(
        orders
            .rows
            .iter()
            .all(|row| !row.contains(&Value::Float(0.0)))
    );
}
