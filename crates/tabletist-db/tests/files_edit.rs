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

fn table(name: &str) -> ObjectRef {
    ObjectRef::new("main", name)
}

async fn page(connection: &Connection, name: &str) -> RowPage {
    connection
        .fetch_rows(&RowQuery::new(table(name), 100))
        .await
        .unwrap()
}

/// What finds the row at `place` of a file, as `page` has its places.
fn at(page: &RowPage, place: i64) -> Vec<(String, Value)> {
    let copy = page
        .positions
        .as_ref()
        .map_or(0, |positions| positions.copy);
    vec![
        (FILE_POSITION.to_owned(), Value::Int(place)),
        (FILE_COPY.to_owned(), Value::Int(copy)),
    ]
}

/// Each row's place, as `page` has them.
fn places(page: &RowPage) -> Option<Vec<i64>> {
    page.positions
        .as_ref()
        .map(|positions| positions.rows.clone())
}

/// A change of one cell of the row at `place`.
fn change(
    page: &RowPage,
    place: i64,
    column: &str,
    type_name: &str,
    loaded: Value,
    new: &str,
) -> RowChange {
    RowChange {
        key: at(page, place),
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
    assert_eq!(places(&orders), Some((0..12).collect()));
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
    assert_eq!(places(&sorted), Some((0..12).rev().collect()));
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
        &orders,
        1,
        "total",
        "DOUBLE",
        orders.rows[1][total].clone(),
        "210.50",
    ));
    changes.deletes.push(RowDelete {
        key: at(&orders, 2),
    });
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
    assert_eq!(places(&again), Some((0..12).collect()));
    // And saves again from there.
    let mut more = set("orders");
    more.deletes.push(RowDelete {
        key: at(&again, 11),
    });
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
        let first = page(&connection, name).await;
        let rows = first.rows.len() as i64;
        let mut changes = set(name);
        changes.deletes.push(RowDelete { key: at(&first, 0) });
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
    let returns = page(&connection, "returns").await;
    let mut changed = std::fs::read_to_string(&path).unwrap();
    changed.push_str("99,2025-12-01,lost\n");
    std::fs::write(&path, &changed).unwrap();
    let mut changes = set("returns");
    changes.deletes.push(RowDelete {
        key: at(&returns, 0),
    });
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
        key: at(&again, rows as i64),
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
    let orders = page(&connection, "orders").await;
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
    // One who may write any folder (root) can't be kept from this one.
    let probe = dir.path().join("probe");
    if std::fs::write(&probe, "").is_ok() {
        std::fs::remove_file(&probe).unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let mut changes = set("orders");
    changes.deletes.push(RowDelete {
        key: at(&orders, 0),
    });
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

#[cfg(unix)]
#[tokio::test]
async fn a_file_that_cannot_be_read_back_is_not_blamed_on_its_folder() {
    use std::os::unix::fs::PermissionsExt;
    let (connection, dir) = editable().await;
    let path = dir.path().join("orders.csv");
    let orders = page(&connection, "orders").await;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o200)).unwrap();
    // One who may read any file (root) can't be kept from this one.
    if std::fs::read(&path).is_ok() {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        return;
    }
    let mut changes = set("orders");
    changes.deletes.push(RowDelete {
        key: at(&orders, 0),
    });
    let error = save(&connection, &changes).await.unwrap_err();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(
        matches!(unsaved(&error), Some(Unsaved::Failed(file, why)) if file == "orders.csv" && why.contains("ermission")),
        "{error:?}"
    );
    assert!(names(dir.path()).iter().all(|name| !name.starts_with('.')));
}

#[tokio::test]
async fn a_file_made_read_only_is_not_written_over() {
    let (connection, dir) = editable().await;
    let path = dir.path().join("orders.csv");
    let before = std::fs::read(&path).unwrap();
    let orders = page(&connection, "orders").await;
    let mut read_only = std::fs::metadata(&path).unwrap().permissions();
    read_only.set_readonly(true);
    std::fs::set_permissions(&path, read_only.clone()).unwrap();
    let mut changes = set("orders");
    changes.deletes.push(RowDelete {
        key: at(&orders, 0),
    });
    let error = save(&connection, &changes).await.unwrap_err();
    // Puts the test's own file back so its folder can be removed: who may
    // write it after doesn't matter.
    #[allow(clippy::permissions_set_readonly_false)]
    read_only.set_readonly(false);
    std::fs::set_permissions(&path, read_only).unwrap();
    assert_eq!(
        unsaved(&error),
        Some(Unsaved::Protected("orders.csv".into()))
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(names(dir.path()).iter().all(|name| !name.starts_with('.')));
}

#[cfg(unix)]
#[tokio::test]
async fn a_file_its_owner_may_not_write_is_not_written_over() {
    use std::os::unix::fs::PermissionsExt;
    let (connection, dir) = editable().await;
    let path = dir.path().join("orders.csv");
    let before = std::fs::read(&path).unwrap();
    let orders = page(&connection, "orders").await;
    // Not read-only to Rust's eye, the group may write it, but its owner may
    // not.
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o464)).unwrap();
    if std::fs::OpenOptions::new().write(true).open(&path).is_ok() {
        // One who may write any file (root).
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        return;
    }
    let mut changes = set("orders");
    changes.deletes.push(RowDelete {
        key: at(&orders, 0),
    });
    let error = save(&connection, &changes).await.unwrap_err();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        unsaved(&error),
        Some(Unsaved::Protected("orders.csv".into()))
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[cfg(unix)]
#[tokio::test]
async fn a_saved_file_keeps_its_own_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let (connection, dir) = editable().await;
    let path = dir.path().join("orders.csv");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
    let orders = page(&connection, "orders").await;
    let mut changes = set("orders");
    changes.deletes.push(RowDelete {
        key: at(&orders, 0),
    });
    save(&connection, &changes).await.unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o640);
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
    // DuckDB reads two rows where a plain count reads three.
    assert_eq!(authors.rows.len(), 2);
    assert_eq!(authors.positions, None);
    let mut changes = set("authors");
    changes.deletes.push(RowDelete {
        key: at(&authors, 0),
    });
    assert!(save(&connection, &changes).await.is_err());
}

/// A session that may write over `files`, each a name and its text, in a
/// folder of their own; `csv` sets how CSV files are read.
async fn over(
    files: &[(&str, &str)],
    csv: Option<tabletist_db::files::Reading>,
) -> (Connection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in files {
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    let mut spec = ConnectSpec::files(Source::Folder(dir.path().to_owned()));
    if let Some(csv) = csv {
        spec.csv = csv;
    }
    let connection = Connection::connect_with(
        &spec,
        &Secrets::default(),
        &HostKeys::default(),
        Access::Writable,
    )
    .await
    .unwrap();
    (connection, dir)
}

#[tokio::test]
async fn a_file_that_can_t_be_read_whole_still_shows_read_only() {
    // A bad value far past the first page and DuckDB's sample.
    let mut text = String::from("id,title\n");
    for id in 1..=30_000 {
        text.push_str(&format!("{id},Book {id}\n"));
    }
    text.push_str("not a number,Dune\n");
    let (connection, _dir) = over(&[("books.csv", &text)], None).await;
    let books = page(&connection, "books").await;
    assert_eq!(books.rows.len(), 100);
    assert_eq!(books.positions, None);
    // And again: what it was found to be is kept.
    assert_eq!(page(&connection, "books").await.positions, None);
}

#[tokio::test]
async fn a_file_with_a_column_named_rowid_is_not_edited() {
    let (connection, _dir) = over(&[("ranks.csv", "rowid,name\n1,Dune\n0,Emma\n")], None).await;
    let ranks = page(&connection, "ranks").await;
    assert_eq!(ranks.rows.len(), 2);
    assert_eq!(ranks.positions, None);
}

#[tokio::test]
async fn a_json_file_whose_records_lack_a_key_is_not_edited() {
    let (connection, _dir) = over(
        &[
            (
                "notes.json",
                r#"[{"id":1},{"id":2,"note":null},{"id":3,"note":"x"}]"#,
            ),
            (
                "whole.ndjson",
                "{\"id\":1,\"note\":null}\n{\"id\":2,\"note\":\"x\"}\n",
            ),
        ],
        None,
    )
    .await;
    assert_eq!(page(&connection, "notes").await.positions, None);
    assert!(page(&connection, "whole").await.positions.is_some());
}

#[tokio::test]
async fn a_json_file_whose_nested_objects_differ_in_keys_is_not_edited() {
    let (connection, _dir) = over(
        &[
            (
                "orders.json",
                r#"[{"id":1,"meta":{"a":1}},{"id":2,"meta":{"a":2,"b":3}}]"#,
            ),
            (
                "items.ndjson",
                "{\"id\":1,\"lines\":[{\"sku\":\"A\"},{\"sku\":\"B\",\"qty\":2}]}\n",
            ),
            (
                "even.json",
                r#"[{"id":1,"meta":{"a":1},"tags":[{"t":"x"}]},{"id":2,"meta":null,"tags":[]}]"#,
            ),
        ],
        None,
    )
    .await;
    assert_eq!(page(&connection, "orders").await.positions, None);
    assert_eq!(page(&connection, "items").await.positions, None);
    // A null where another has an object adds no key.
    assert!(page(&connection, "even").await.positions.is_some());
}

#[tokio::test]
async fn a_json_file_of_records_that_are_no_objects_is_not_edited() {
    let (connection, _dir) = over(
        &[
            ("numbers.json", "[1,2,3]"),
            // A key with a dot in it is no nested one.
            ("dotted.json", r#"[{"a":{"b":{"x":1}},"a.b":{"y":2}}]"#),
        ],
        None,
    )
    .await;
    assert_eq!(page(&connection, "numbers").await.positions, None);
    assert!(page(&connection, "dotted").await.positions.is_some());
}

#[tokio::test]
async fn a_tsv_is_split_on_its_tabs_whatever_reading_csv_sets() {
    let reading = tabletist_db::files::Reading {
        delimiter: tabletist_db::files::Delimiter::Semicolon,
        ..Default::default()
    };
    let (connection, dir) = over(
        &[(
            "authors.tsv",
            "id\tname\n1\tToni Morrison\n2\tJames Joyce\n",
        )],
        Some(reading),
    )
    .await;
    let authors = page(&connection, "authors").await;
    let mut changes = set("authors");
    changes.rows.push(change(
        &authors,
        1,
        "name",
        "VARCHAR",
        authors.rows[1][1].clone(),
        "James Augustine Joyce",
    ));
    save(&connection, &changes).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("authors.tsv")).unwrap(),
        "id\tname\n1\tToni Morrison\n2\tJames Augustine Joyce\n"
    );
}

#[tokio::test]
async fn rows_of_a_page_read_before_a_save_are_not_saved_after_it() {
    let (connection, dir) = editable().await;
    let path = dir.path().join("orders.csv");
    let orders = page(&connection, "orders").await;
    let mut first = set("orders");
    first.deletes.push(RowDelete {
        key: at(&orders, 0),
    });
    save(&connection, &first).await.unwrap();
    let written = std::fs::read(&path).unwrap();
    // The old page's second row is now first: its place names another.
    let mut stale = set("orders");
    stale.deletes.push(RowDelete {
        key: at(&orders, 1),
    });
    let error = save(&connection, &stale).await.unwrap_err();
    assert_eq!(unsaved(&error), Some(Unsaved::Reread("orders.csv".into())));
    assert_eq!(std::fs::read(&path).unwrap(), written);
}

#[tokio::test]
async fn a_hidden_file_that_cannot_be_made_says_the_system_s_words() {
    let (connection, dir) = editable().await;
    let path = dir.path().join("orders.csv");
    let before = std::fs::read(&path).unwrap();
    let orders = page(&connection, "orders").await;
    // A folder where the save's hidden file would go.
    std::fs::create_dir(dir.path().join(".orders.csv.tabletist-save")).unwrap();
    let mut changes = set("orders");
    changes.deletes.push(RowDelete {
        key: at(&orders, 0),
    });
    let error = save(&connection, &changes).await.unwrap_err();
    assert!(
        matches!(unsaved(&error), Some(Unsaved::Failed(file, _)) if file == "orders.csv"),
        "{error:?}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    // What was in the way is left, and the copy is as it was.
    assert!(dir.path().join(".orders.csv.tabletist-save").is_dir());
    assert_eq!(page(&connection, "orders").await.rows, orders.rows);
}

#[tokio::test]
async fn a_hidden_file_already_there_is_left_as_it_is() {
    let (connection, dir) = editable().await;
    let path = dir.path().join("events.ndjson");
    let before = std::fs::read(&path).unwrap();
    let hidden = dir.path().join(".events.ndjson.tabletist-save");
    std::fs::write(&hidden, "another program's").unwrap();
    let events = page(&connection, "events").await;
    let mut changes = set("events");
    changes.deletes.push(RowDelete {
        key: at(&events, 0),
    });
    let error = save(&connection, &changes).await.unwrap_err();
    assert!(
        matches!(unsaved(&error), Some(Unsaved::Failed(file, _)) if file == "events.ndjson"),
        "{error:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&hidden).unwrap(),
        "another program's"
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[tokio::test]
async fn rows_read_before_a_reconnect_are_not_saved_after_it() {
    let (connection, dir) = editable().await;
    let path = dir.path().join("orders.csv");
    let before = std::fs::read(&path).unwrap();
    let orders = page(&connection, "orders").await;
    // A session of its own, as a reconnect opens, that has read no page.
    let again = open(dir.path(), Access::Writable).await;
    let mut changes = set("orders");
    changes.deletes.push(RowDelete {
        key: at(&orders, 0),
    });
    changes.rows.push(change(
        &orders,
        1,
        "email",
        "VARCHAR",
        orders.rows[1][2].clone(),
        "reader@example.com",
    ));
    let error = save(&again, &changes).await.unwrap_err();
    assert_eq!(unsaved(&error), Some(Unsaved::Reread("orders.csv".into())));
    // Nor after it has read a page of its own.
    page(&again, "orders").await;
    let error = save(&again, &changes).await.unwrap_err();
    assert_eq!(unsaved(&error), Some(Unsaved::Reread("orders.csv".into())));
    assert_eq!(std::fs::read(&path).unwrap(), before);
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
