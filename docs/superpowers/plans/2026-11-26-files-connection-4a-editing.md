# Files connection 4a: editing the files a save rewrites

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Files connection's **Open read-only** box can be turned off, and then a CSV, TSV, Parquet, JSON or NDJSON file's table is edited in the grid like any other table and saved back to the file: through a hidden file renamed over it, never over a file changed on disk since it was read.

**Architecture:**
- **The db layer** (`crates/tabletist-db/src/duck.rs`, `duck/edit.rs` new, `lib.rs`):
  - `Connection::connect_with` and `connect_files` give a Files session the access they are asked for (`connect_files` gains an `access` argument). The agents' session and an export's stay read-only.
  - A session that may write attaches an in-memory database, `tabletist_edits`, before the fence, and adds to `allowed_paths` the hidden file each save of a Parquet, JSON or NDJSON file is written to first (`.orders.csv.tabletist-save` beside `orders.csv`).
  - The first page of a file's table in such a session reads the file into a copy in `tabletist_edits`, its `rowid` kept as a column, `__tabletist_position` (`FILE_POSITION`): each row's place in the file. A page of it is read from the copy and comes with `RowPage::positions`, ordered by that place. A file that changed on disk since its copy was made is read again on the next page.
  - A CSV or TSV is read a second way, by Tabletist's own record splitter, as DuckDB splits it (quotes, CRLF, a byte-order mark, blank lines). Where the two don't count the same rows, the file isn't edited (`Loaded::Unmatched`), and its page comes without positions.
  - `write` checks the file's size and modified time against when it was read (`Stamp`), then in one transaction changes the copy and writes the file: a CSV or TSV row for row (every untouched row byte for byte, a changed or new row in the file's delimiter and quote, its dates in the file's date format), Parquet, JSON and NDJSON by DuckDB's `COPY` (Parquet with its compression). The hidden file takes the original's permissions and is renamed over it. Then each row's place is numbered again.
  - A save that the file keeps from writing returns an `Error::Query` whose `Named::unsaved` says why: `Unsaved::Changed(file)` or `Unsaved::Unwritable(file)`. Nothing is written and the copy is as it was.
  - `Structure::unedited` says why a table of a session that may write isn't edited: `Unedited::Excel`, `Unedited::DuckDb` (until run 4b) or `Unedited::Attached`.
  - A script still runs in a read-only transaction, whatever mode it is sent in.
- **The core** (`crates/tabletist-core/src/model.rs`, `connections.rs`, `edit.rs`; `src/app/editing.rs`):
  - The form's box starts on for Files, whatever the environment, and can be turned off; it is saved as it shows. A saved Files connection with no `read_only` opens read-only (`connections::access` takes the driver).
  - `edit::Table::key` is `Some(vec![])` on a page with positions, and `Table::found_by(row, key)` gives the key a save finds the row by: `__tabletist_position` and its place. `change_set` builds every row's key with it.
  - `Lock::Unedited(why)` and `Lock::Unmatched` lock a Files table its connection doesn't edit, for cells, new rows and deletes.
  - A save of a page with positions always reads the page again.
  - `Workspace::sql_writes` says `NoWrites::Files` for a Files connection: its SQL editor only reads.
- **The UI** (`src/ui/connect_dialog/{mod,sheet,terminal,join}.rs`, `picker.rs`, `data_view.rs`, `pending_bar.rs`, `cell_editor.rs`, `sql_editor.rs`, `sql_results.rs`):
  - The safety box's note says what a save does, and names the largest file a save would write again.
  - The connections list says "Editable".
  - A Files table its connection doesn't edit says why first in its header.
  - A save the file kept from writing says why, and how many changes stay pending.
  - A write in the SQL editor gets a card saying files are edited in their tables.

**Tech Stack:** Rust, egui (`egui_kittest` headless tests), DuckDB (`duckdb` 1.20000.1, DuckDB 2.0.0), gettext catalogs (`assets/i18n`).

**Base:** `main` at `ad14a7e2`, branch `claude/files-connection-4a-editing`. Run 3b (#388) is merged.

---

## What was decided

On 2026-11-26 Igor picked these, with the recommended option each time:

| Question | Decision |
| --- | --- |
| How run 4 ships | **Two PRs.** 4a (this plan): the files a save rewrites (CSV, TSV, Parquet, JSON, NDJSON), the safety box's notes, the temp file and rename, the changed-on-disk check, an unwritable folder, Excel's reason in the header, "Editable" in the list. 4b: `.duckdb` files written in place, in a transaction. |
| When a file is read into memory | **When its table first opens**: the first page read of it. Only the files opened cost memory, and opening a large one takes as long as reading it. |
| The saved connections a Files connection attaches, when its files are editable | **All stay read-only.** Writing to a live server through DuckDB would skip Tabletist's own handling of that server's keys and edits. |
| A write statement in a Files connection's SQL editor | **Refused.** Files are edited in the grid and saved; the card says so. |
| What a save of a CSV or TSV does to the rows it didn't change | **Kept as written**, byte for byte. Only changed and new rows are written, with the file's delimiter and quote. A file whose rows can't be matched one for one with what DuckDB read isn't edited, and says why. |

### What the probe found (DuckDB 2.0.0)

- A table made `AS SELECT * FROM` a file's view keeps the file's order: its `rowid` is each row's place, 0 to n−1. A deleted row's `rowid` isn't reused, and the others keep theirs; a new row gets the next.
- A write in a read-only transaction is refused on the copy too ("transaction is launched in read-only mode"): scripts need no new guard.
- With the fence set (`allowed_paths`, `enable_external_access = false`, `lock_configuration = true`), `COPY` reaches a path in `allowed_paths` and is refused anywhere else.
- `sniff_csv` gives the delimiter, quote, escape, line break, header, date and timestamp formats. `parquet_metadata` gives each column chunk's compression (`ZSTD`). `COPY … (FORMAT json, ARRAY true)` writes an array, `(FORMAT json)` a line each.
- A typed rewrite of a CSV changes values: `0.50` comes back `0.5`. Hence the row-for-row writer.
- DuckDB skips a blank line of a CSV with more than one column, and reads one of a one-column CSV as a NULL row. A quoted line break, CRLF, a byte-order mark and no last line break all read as expected. A line of spaces makes the sniffer find another delimiter: the count check catches what it leads to.
- `read_csv` has no `file_row_number` (Parquet's reader has).

Canvas (version 222): **"macOS – Files, editing"** and **"Omarchy – Files, editing"** are new. Each draws:
- the safety box on and off, and the Join section's line;
- the table header of a file not edited: Excel's, and a CSV's whose rows can't be matched;
- a save refused because the file changed on disk, and one refused because the folder can't be written to;
- the SQL editor refusing a write.

The macOS board also draws the connections list's "Editable". The Omarchy board also draws a file's table being edited.

### Where this plan does more, or less, than the prompt or the canvas

| The prompt or canvas | What is built | Why |
| --- | --- | --- |
| "Production, or any connection set to open read-only, is always attached read-only, even when the files are editable." | Every attached connection is read-only, editable files or not. The Join section says "Attached connections open read-only here, even when the files are editable." An attached table says "Attached connections open read-only". | Decided above. |
| "`.duckdb` files are written in place, in a transaction." | Run 4b. Until then a `.duckdb` file's tables say ".duckdb files open read-only". | Decided above. |
| "When editing starts, load the file into an in-memory table with a hidden row number" | The first page of a file's table in a session that may write reads it into its copy. The row number is the copy's `__tabletist_position`, outside the page's columns (`RowPage::positions`). | Decided above. A page read before an edit must already know each row's place. |
| "CSV and TSV: same delimiter, header and quoting as read" | The header and every untouched row are copied byte for byte. A changed or new row is written with the file's delimiter and quote character, a field quoted only where it holds the delimiter, the quote or a line break, or starts or ends with a space. A changed value is written as typed; a date or timestamp in the file's own format where the sniffer found one. | Decided above. |
| — | A CSV whose rows Tabletist's splitter doesn't count as DuckDB does isn't edited. Its header says "orders opens read-only: Tabletist couldn't match its rows to the file's lines". | Decided above. A save could otherwise write a change into the wrong line. |
| — | Parquet, JSON and NDJSON are written by DuckDB from the copy, so a value can be written another way than it was (`12.50` as `12.5` in JSON). The docs say so. | They aren't text rows to copy. |
| "Before writing, check that the file's size and modified time haven't changed since it was loaded." | Since its copy was made. A page read after the file changed reads it again, so Reload shows the new version. | The message says to reload. |
| "your 3 changes stay pending" | The count is every pending change: changed cells, new rows and deleted rows. One change: "your 1 change stays pending." | The pending bar's own counts. |
| "If the folder isn't writable, say so and keep the changes pending." | "orders.csv couldn't be saved: its folder can't be written to. Your 1 change stays pending." Any failure to write the hidden file or rename it says this. | The prompt gives no words. |
| Omarchy board: `changes wait for ctrl+s` | `changes wait for :w` | The prompt names `:w`, the save key of the grid's vim mode. |
| "Excel files always stay read-only, with the reason shown in the table's header" | The reason is the first part of the header's subtitle, which stays when the header is narrow, and the words on a cell, Add row and Delete. Only where the connection may write: in a read-only one every table already says "This connection opens read-only". | |
| The Omarchy list line | Unchanged: it says nothing of read-only or editable. 3b's board draws `read-only` beside the name, which isn't built. | The prompt asks the macOS right column only. |
| "Agents can query the files, never change them" | An agent's session opens read-only, whatever the box says. | The prompt. |
| — | The Review pending SQL shows a file's row found by its place, `__tabletist_position`, in its `WHERE`. | It is what the save runs, on the copy. |

The new words have no AI, sparkle or confidence wording, and the UI copy has no em dashes. They're translated into Russian and Brazilian Portuguese; the Portuguese wants a native review.

## Files

| File | Change |
| --- | --- |
| `crates/tabletist-db/src/duck/edit.rs` (new) | The copies, the record splitter, the save |
| `crates/tabletist-db/src/duck.rs`, `lib.rs` | A Files session's access; the copies' database and the hidden files in the fence; a page from the copy; `write`; `Unedited`, `FILE_POSITION` |
| `crates/tabletist-db/src/error.rs`, `catalog.rs`, `query.rs`, `files.rs` | `Named::unsaved` and `Unsaved`; `Structure::unedited`; `RowPage::positions`; `Format::rewritten` |
| `crates/tabletist-db/src/{bigquery,dialect,mysql,pg,sqlite}.rs`, and the literals in `crates/tabletist-core` and `src` | The new fields, `None` |
| `crates/tabletist-core/src/backend.rs`, `backend/agents.rs`, `backend/export.rs`, `crates/tabletist-db/tests/{files,files_attach,ssh}.rs` | `connect_files` takes an access |
| `crates/tabletist-db/tests/files_edit.rs` (new) | The db's tests |
| `crates/tabletist-core/src/model.rs`, `connections.rs`, `src/app.rs`, `src/shots.rs` | The box for Files; Files read-only by default; `NoWrites::Files` |
| `crates/tabletist-core/src/edit.rs`, `src/app/editing.rs`, `src/ui/cell_editor.rs`, `src/ui/sql_editor.rs`, `src/ui/sql_results.rs`, `src/ui/data_view.rs` | Rows found by their place; the new locks and their words; a page read again after a save; the SQL editor's gate |
| `src/ui/connect_dialog/{mod,sheet,terminal,join}.rs` | The safety box's notes in both looks; the Join section's line |
| `src/ui/picker.rs`, `src/ui/data_view.rs`, `src/ui/pending_bar.rs` | "Editable"; the header's reason; a save the file kept from writing |
| `src/ui/files_edit_tests.rs` (new), `src/ui/files_tests.rs`, `src/ui/mod.rs` | Headless tests in both looks |
| `assets/i18n/*.po`, `*.pot` | The new words |
| `docs/_guide/editing-data.md`, `docs/_reference/connections.md`, `docs/_reference/settings-and-files.md` | Editing files |

Every commit passes these, the full check:
- `~/.cargo/bin/cargo fmt --check`
- `~/.cargo/bin/cargo clippy --workspace --all-targets -- -D warnings`
- `~/.cargo/bin/cargo test --workspace --no-fail-fast`, the headless tests (with the server URLs below)
- `RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --workspace --no-deps`
- `sh assets/i18n/update.sh --check`

Run cargo as `~/.cargo/bin/cargo` with `CARGO_TARGET_DIR=$PWD/target`. For the server tests, point `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` at the test servers when they're up (`ss -ltn` shows 55432 and 53306). Commits are signed: one for this plan, then one per task.

---

### Task 1: The db layer edits a file and writes it back

**Files:**
- Create: `crates/tabletist-db/src/duck/edit.rs`, `crates/tabletist-db/tests/files_edit.rs`
- Modify: `crates/tabletist-db/src/{duck,lib,error,catalog,query,files}.rs`; every `RowPage`, `Structure` and `Named` literal (`crates/tabletist-db/src/{bigquery,dialect,mysql,pg,sqlite}.rs`, `crates/tabletist-core/src/{edit,fixtures,key_picker}.rs`, `src/app/history.rs`, `src/shots.rs`, `src/ui/{data_view,delete_tests,mod,row_panel,value_tags}.rs`); `connect_files`'s callers (`crates/tabletist-core/src/backend.rs`, `backend/agents.rs`, `backend/export.rs`, `crates/tabletist-db/tests/{files_attach,ssh}.rs`); `crates/tabletist-db/tests/files.rs`

- [ ] **Step 1: Write the failing tests**

On the Bookshop's exports, a session that may write:
- a page says where each row is, sorted or not, and a read-only session's doesn't;
- a save of `orders.csv` keeps the header and every untouched row as it was, writes the changed total as typed, leaves the deleted row out and the new row last, leaves no hidden file, and saves again from the new places;
- Parquet, JSON and NDJSON are written back as they were read, Parquet with its compression;
- a file changed on disk since it was read is never written over, and is read again on the next page;
- a folder that can't be written to keeps the file and the copy as they were;
- a CSV whose rows the splitter doesn't count as DuckDB does isn't edited;
- a `.duckdb` file's tables say why they aren't edited;
- a script still reads only.

````diff
diff --git a/crates/tabletist-db/tests/files_edit.rs b/crates/tabletist-db/tests/files_edit.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-db/tests/files_edit.rs
@@ -0,0 +1,361 @@
+//! Editing a Files connection's files through the public `Connection`
+//! API, on the Bookshop's exports (`fixtures::write_bookshop_files`): a
+//! session that may write reads a file's table into a copy, each row with
+//! its place, and a save writes the file back through a hidden file
+//! renamed over it, never over a file changed since.
+
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
+use std::path::Path;
+
+use tabletist_db::files::Source;
+use tabletist_db::{
+    Access, CellChange, ChangeSet, ConnectSpec, Connection, Error, FILE_POSITION, HostKeys,
+    InsertValue, NewValue, ObjectRef, RowChange, RowDelete, RowInsert, RowPage, RowQuery, Secrets,
+    StopFlag, Unedited, Unsaved, Value, WriteOutcome,
+};
+
+/// The Bookshop's exports in a folder of their own, and a session over it
+/// that may write.
+async fn editable() -> (Connection, tempfile::TempDir) {
+    let dir = tempfile::tempdir().unwrap();
+    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
+    let connection = open(dir.path(), Access::Writable).await;
+    (connection, dir)
+}
+
+async fn open(folder: &Path, access: Access) -> Connection {
+    Connection::connect_with(
+        &ConnectSpec::files(Source::Folder(folder.to_owned())),
+        &Secrets::default(),
+        &HostKeys::default(),
+        access,
+    )
+    .await
+    .unwrap()
+}
+
+fn table(name: &str) -> ObjectRef {
+    ObjectRef::new("main", name)
+}
+
+async fn page(connection: &Connection, name: &str) -> RowPage {
+    connection
+        .fetch_rows(&RowQuery::new(table(name), 100))
+        .await
+        .unwrap()
+}
+
+/// What finds the row at `place` of a file.
+fn at(place: i64) -> Vec<(String, Value)> {
+    vec![(FILE_POSITION.to_owned(), Value::Int(place))]
+}
+
+/// A change of one cell of the row at `place`.
+fn change(place: i64, column: &str, type_name: &str, loaded: Value, new: &str) -> RowChange {
+    RowChange {
+        key: at(place),
+        set: vec![CellChange {
+            column: column.into(),
+            type_name: type_name.into(),
+            loaded,
+            new: NewValue::Text(new.into()),
+        }],
+    }
+}
+
+fn set(name: &str) -> ChangeSet {
+    ChangeSet {
+        object: table(name),
+        inserts: Vec::new(),
+        rows: Vec::new(),
+        deletes: Vec::new(),
+    }
+}
+
+async fn save(connection: &Connection, changes: &ChangeSet) -> Result<WriteOutcome, Error> {
+    connection.write(changes, &StopFlag::new()).await
+}
+
+/// Why a save of a file wrote nothing.
+fn unsaved(error: &Error) -> Option<Unsaved> {
+    match error {
+        Error::Query { named, .. } => named.unsaved.clone(),
+        _ => None,
+    }
+}
+
+/// The folder's files, the hidden ones among them.
+fn names(dir: &Path) -> Vec<String> {
+    let mut names: Vec<String> = std::fs::read_dir(dir)
+        .unwrap()
+        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
+        .collect();
+    names.sort();
+    names
+}
+
+#[tokio::test]
+async fn a_page_of_an_editable_file_says_where_each_row_is() {
+    let (connection, _dir) = editable().await;
+    assert_eq!(connection.access(), Access::Writable);
+    let orders = page(&connection, "orders").await;
+    assert_eq!(orders.positions, Some((0..12).collect()));
+    assert!(orders.ordered_by_key);
+    // The place is no column of the table's.
+    assert!(
+        orders
+            .columns
+            .iter()
+            .all(|column| column.name != FILE_POSITION)
+    );
+    // Sorted, each row keeps its own place.
+    let mut query = RowQuery::new(table("orders"), 100);
+    query.sort = vec![tabletist_db::Sort {
+        column: "id".into(),
+        dir: tabletist_db::SortDir::Desc,
+    }];
+    let sorted = connection.fetch_rows(&query).await.unwrap();
+    assert_eq!(sorted.positions, Some((0..12).rev().collect()));
+    // A session that reads only has no places.
+    let dir = tempfile::tempdir().unwrap();
+    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
+    let reading = open(dir.path(), Access::ReadOnly).await;
+    assert_eq!(page(&reading, "orders").await.positions, None);
+}
+
+#[tokio::test]
+async fn a_save_writes_a_csv_back_keeping_every_row_it_did_not_change() {
+    let (connection, dir) = editable().await;
+    let path = dir.path().join("orders.csv");
+    let before = std::fs::read_to_string(&path).unwrap();
+    let lines: Vec<&str> = before.lines().collect();
+    let orders = page(&connection, "orders").await;
+    let total = orders
+        .columns
+        .iter()
+        .position(|column| column.name == "total")
+        .unwrap();
+    let mut changes = set("orders");
+    changes.rows.push(change(
+        1,
+        "total",
+        "DOUBLE",
+        orders.rows[1][total].clone(),
+        "210.50",
+    ));
+    changes.deletes.push(RowDelete { key: at(2) });
+    changes.inserts.push(RowInsert {
+        set: vec![
+            InsertValue {
+                column: "id".into(),
+                type_name: "BIGINT".into(),
+                new: NewValue::Text("13".into()),
+            },
+            InsertValue {
+                column: "email".into(),
+                type_name: "VARCHAR".into(),
+                new: NewValue::Text("reader, new@example.com".into()),
+            },
+        ],
+    });
+    let outcome = save(&connection, &changes).await.unwrap();
+    assert!(
+        matches!(outcome, WriteOutcome::Written { .. }),
+        "{outcome:?}"
+    );
+    let after = std::fs::read_to_string(&path).unwrap();
+    let written: Vec<&str> = after.lines().collect();
+    // The header and every row not changed, as they were; the changed
+    // total as it was typed; the deleted row gone; the new row last.
+    assert_eq!(written[0], lines[0]);
+    assert_eq!(written[1], lines[1]);
+    assert_eq!(written[2], lines[2].replace(",201.70", ",210.50"));
+    assert_eq!(&written[3..12], &lines[4..13]);
+    assert_eq!(written[12], "13,,\"reader, new@example.com\",,,");
+    assert_eq!(written.len(), 13);
+    // No hidden file is left.
+    assert!(names(dir.path()).iter().all(|name| !name.starts_with('.')));
+    // The session reads the file as it is now, each row at its new place.
+    let again = page(&connection, "orders").await;
+    assert_eq!(again.rows.len(), 12);
+    assert_eq!(again.positions, Some((0..12).collect()));
+    // And saves again from there.
+    let mut more = set("orders");
+    more.deletes.push(RowDelete { key: at(11) });
+    save(&connection, &more).await.unwrap();
+    assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 12);
+}
+
+#[tokio::test]
+async fn parquet_json_and_ndjson_are_written_back_as_they_were_read() {
+    let (connection, dir) = editable().await;
+    for (name, file) in [
+        ("customers", "customers.parquet"),
+        ("books", "books.json"),
+        ("events", "events.ndjson"),
+    ] {
+        let rows = page(&connection, name).await.rows.len() as i64;
+        let mut changes = set(name);
+        changes.deletes.push(RowDelete { key: at(0) });
+        save(&connection, &changes).await.unwrap();
+        let again = page(&connection, name).await;
+        assert_eq!(again.rows.len() as i64, rows - 1, "{file}");
+        assert!(dir.path().join(file).exists(), "{file}");
+    }
+    let books = std::fs::read_to_string(dir.path().join("books.json")).unwrap();
+    assert!(books.starts_with('['), "an array: {books}");
+    let events = std::fs::read_to_string(dir.path().join("events.ndjson")).unwrap();
+    assert!(events.lines().all(|line| line.starts_with('{')), "{events}");
+    // The Parquet file keeps its compression.
+    let reading = duckdb::Connection::open_in_memory().unwrap();
+    let compression: String = reading
+        .query_row(
+            &format!(
+                "SELECT DISTINCT compression FROM parquet_metadata('{}')",
+                dir.path().join("customers.parquet").display()
+            ),
+            [],
+            |row| row.get(0),
+        )
+        .unwrap();
+    assert_eq!(compression, "ZSTD");
+}
+
+#[tokio::test]
+async fn a_file_changed_on_disk_since_it_was_read_is_never_written_over() {
+    let (connection, dir) = editable().await;
+    let path = dir.path().join("returns.csv");
+    page(&connection, "returns").await;
+    let mut changed = std::fs::read_to_string(&path).unwrap();
+    changed.push_str("99,2025-12-01,lost\n");
+    std::fs::write(&path, &changed).unwrap();
+    let mut changes = set("returns");
+    changes.deletes.push(RowDelete { key: at(0) });
+    let error = save(&connection, &changes).await.unwrap_err();
+    assert_eq!(
+        unsaved(&error),
+        Some(Unsaved::Changed("returns.csv".into()))
+    );
+    assert_eq!(std::fs::read_to_string(&path).unwrap(), changed);
+}
+
+#[tokio::test]
+async fn a_file_changed_on_disk_is_read_again_on_the_next_page() {
+    let (connection, dir) = editable().await;
+    let path = dir.path().join("returns.csv");
+    let rows = page(&connection, "returns").await.rows.len();
+    let mut changed = std::fs::read_to_string(&path).unwrap();
+    changed.push_str("99,2025-12-01,lost\n");
+    std::fs::write(&path, &changed).unwrap();
+    // A reload shows the file as it is now, and a save goes on from it.
+    let again = page(&connection, "returns").await;
+    assert_eq!(again.rows.len(), rows + 1);
+    let mut changes = set("returns");
+    changes.deletes.push(RowDelete {
+        key: at(rows as i64),
+    });
+    save(&connection, &changes).await.unwrap();
+    assert_eq!(
+        std::fs::read_to_string(&path).unwrap().lines().count(),
+        rows + 1
+    );
+}
+
+#[cfg(unix)]
+#[tokio::test]
+async fn a_folder_that_cannot_be_written_to_keeps_the_file_as_it_was() {
+    use std::os::unix::fs::PermissionsExt;
+    let (connection, dir) = editable().await;
+    let path = dir.path().join("orders.csv");
+    let before = std::fs::read(&path).unwrap();
+    page(&connection, "orders").await;
+    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
+    let mut changes = set("orders");
+    changes.deletes.push(RowDelete { key: at(0) });
+    let error = save(&connection, &changes).await.unwrap_err();
+    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
+    assert_eq!(
+        unsaved(&error),
+        Some(Unsaved::Unwritable("orders.csv".into()))
+    );
+    assert_eq!(std::fs::read(&path).unwrap(), before);
+    // The copy is as it was: every row still there.
+    assert_eq!(page(&connection, "orders").await.rows.len(), 12);
+}
+
+#[tokio::test]
+async fn a_csv_whose_rows_are_not_counted_as_duckdb_reads_them_is_not_edited() {
+    let dir = tempfile::tempdir().unwrap();
+    // A first line DuckDB's sniffer takes for no part of the table.
+    std::fs::write(
+        dir.path().join("authors.csv"),
+        "Exported from the Bookshop\nid,name\n1,Toni Morrison\n2,James Joyce\n",
+    )
+    .unwrap();
+    let connection = open(dir.path(), Access::Writable).await;
+    let authors = page(&connection, "authors").await;
+    if authors.rows.len() == 2 {
+        assert_eq!(authors.positions, None);
+        let mut changes = set("authors");
+        changes.deletes.push(RowDelete { key: at(0) });
+        assert!(save(&connection, &changes).await.is_err());
+    }
+}
+
+#[tokio::test]
+async fn a_duckdb_file_s_tables_say_why_they_are_not_edited() {
+    let (connection, _dir) = editable().await;
+    let covers = connection
+        .describe(&ObjectRef::new("bookshop", "book_covers"))
+        .await
+        .unwrap();
+    assert_eq!(covers.unedited, Some(Unedited::DuckDb));
+    let orders = connection.describe(&table("orders")).await.unwrap();
+    assert_eq!(orders.unedited, None);
+    // A save to one is refused, as on a session that reads.
+    let mut changes = set("orders");
+    changes.object = ObjectRef::new("bookshop", "book_covers");
+    changes.deletes.push(RowDelete { key: at(0) });
+    assert_eq!(
+        save(&connection, &changes).await.unwrap_err(),
+        Error::ReadOnly
+    );
+}
+
+#[tokio::test]
+async fn a_script_still_reads_only() {
+    let (connection, dir) = editable().await;
+    let before = std::fs::read(dir.path().join("orders.csv")).unwrap();
+    page(&connection, "orders").await;
+    let statements = tabletist_db::sql::statements(
+        tabletist_db::Dialect::DuckDb,
+        "UPDATE tabletist_edits.orders SET total = 0",
+    );
+    let outcome = connection
+        .run_script(
+            &statements,
+            10,
+            tabletist_db::ScriptMode::Write,
+            &StopFlag::new(),
+        )
+        .await
+        .unwrap();
+    assert!(matches!(
+        outcome.results[0].outcome,
+        tabletist_db::StatementOutcome::Error { .. }
+    ));
+    assert_eq!(
+        std::fs::read(dir.path().join("orders.csv")).unwrap(),
+        before
+    );
+    // The copy is as it was read.
+    let orders = page(&connection, "orders").await;
+    assert!(
+        orders
+            .rows
+            .iter()
+            .all(|row| !row.contains(&Value::Float(0.0)))
+    );
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist-db --test files_edit`

Expected: they don't compile. `FILE_POSITION`, `Unedited`, `Unsaved`, `RowPage::positions` and `Structure::unedited` aren't there yet.

```text
error[E0432]: unresolved imports `tabletist_db::FILE_POSITION`, `tabletist_db::Unedited`, `tabletist_db::Unsaved`
error[E0609]: no field `unsaved` on type `&Box<tabletist_db::Named>`
error[E0609]: no field `positions` on type `RowPage`
    |
error[E0609]: no field `unedited` on type `Structure`
error: could not compile `tabletist-db` (test "files_edit") due to 9 previous errors
```

- [ ] **Step 3: Read a file into its copy, and write it back**

The splitter and the writer's own unit tests are at the end of `duck/edit.rs`. Then fill the new fields in every literal the compiler names: `None` each time. `plan4a`'s `fix63.py` does it from cargo's errors, but writing each one is the same. `connect_files` gains `access` before `downloading`: the agents' and an export's sessions pass `Access::ReadOnly`, the backend's open passes the session's.

````diff
diff --git a/crates/tabletist-core/src/backend.rs b/crates/tabletist-core/src/backend.rs
--- a/crates/tabletist-core/src/backend.rs
+++ b/crates/tabletist-core/src/backend.rs
@@ -2356,7 +2356,10 @@
                         true => Connection::connect_with(&spec, &secrets, &host_keys, access).await,
                         false => match attach::attaching(attach, &keyring).await {
                             Ok(attaching) => {
-                                Connection::connect_files(&spec, &attaching, &host_keys, None).await
+                                Connection::connect_files(
+                                    &spec, &attaching, &host_keys, access, None,
+                                )
+                                .await
                             }
                             Err(error) => Err(error),
                         },
diff --git a/crates/tabletist-core/src/backend/agents.rs b/crates/tabletist-core/src/backend/agents.rs
--- a/crates/tabletist-core/src/backend/agents.rs
+++ b/crates/tabletist-core/src/backend/agents.rs
@@ -1033,9 +1033,15 @@
             let attaching = super::attach::attaching_quietly(agentable.attach.clone(), keyring)
                 .await
                 .map_err(|error| unavailable(&error.to_string()))?;
-            return Connection::connect_files(&agentable.spec, &attaching, host_keys, None)
-                .await
-                .map_err(|error| unavailable(&error.to_string()));
+            return Connection::connect_files(
+                &agentable.spec,
+                &attaching,
+                host_keys,
+                tabletist_db::Access::ReadOnly,
+                None,
+            )
+            .await
+            .map_err(|error| unavailable(&error.to_string()));
         }
         Driver::Sqlite | Driver::Postgres | Driver::MySql | Driver::BigQuery => {}
     }
diff --git a/crates/tabletist-core/src/backend/export.rs b/crates/tabletist-core/src/backend/export.rs
--- a/crates/tabletist-core/src/backend/export.rs
+++ b/crates/tabletist-core/src/backend/export.rs
@@ -52,7 +52,14 @@
         .await;
     }
     let attaching = super::attach::attaching(open.attach.clone(), keyring).await?;
-    Connection::connect_files(&open.spec, &attaching, &open.host_keys, None).await
+    Connection::connect_files(
+        &open.spec,
+        &attaching,
+        &open.host_keys,
+        tabletist_db::Access::ReadOnly,
+        None,
+    )
+    .await
 }
 
 /// Where the rows come from.
diff --git a/crates/tabletist-core/src/edit.rs b/crates/tabletist-core/src/edit.rs
--- a/crates/tabletist-core/src/edit.rs
+++ b/crates/tabletist-core/src/edit.rs
@@ -4866,6 +4866,7 @@
 
     fn page(rows: Vec<Vec<Value>>) -> RowPage {
         RowPage {
+            positions: None,
             columns: vec![
                 meta("id", "INTEGER"),
                 meta("email", "TEXT"),
diff --git a/crates/tabletist-core/src/fixtures.rs b/crates/tabletist-core/src/fixtures.rs
--- a/crates/tabletist-core/src/fixtures.rs
+++ b/crates/tabletist-core/src/fixtures.rs
@@ -120,6 +120,7 @@
 /// A page shaped like the fixture's users table.
 pub fn page(rows: usize, has_more: bool) -> RowPage {
     RowPage {
+        positions: None,
         columns: vec![
             ColumnMeta {
                 name: "id".into(),
@@ -221,6 +222,7 @@
         kind,
     };
     RowPage {
+        positions: None,
         columns: vec![
             meta("id", "INTEGER", ValueKind::Numeric),
             meta("publisher_id", "INTEGER", ValueKind::Numeric),
@@ -332,6 +334,7 @@
         kind,
     };
     RowPage {
+        positions: None,
         columns: vec![
             meta("id", "INTEGER", ValueKind::Numeric),
             meta("name", "TEXT", ValueKind::Text),
@@ -408,6 +411,7 @@
         kind,
     };
     RowPage {
+        positions: None,
         columns: vec![
             meta("id", "INTEGER", ValueKind::Numeric),
             meta("parent_id", "INTEGER", ValueKind::Numeric),
@@ -485,6 +489,7 @@
         ]
     };
     RowPage {
+        positions: None,
         columns: vec![
             meta("id", "INTEGER", ValueKind::Numeric),
             meta("isbn", "TEXT", ValueKind::Text),
diff --git a/crates/tabletist-core/src/key_picker.rs b/crates/tabletist-core/src/key_picker.rs
--- a/crates/tabletist-core/src/key_picker.rs
+++ b/crates/tabletist-core/src/key_picker.rs
@@ -1313,6 +1313,7 @@
             ..Structure::default()
         };
         let page = RowPage {
+            positions: None,
             columns: vec![
                 meta("id", ValueKind::Numeric),
                 meta("parent_id", ValueKind::Numeric),
diff --git a/crates/tabletist-db/src/bigquery.rs b/crates/tabletist-db/src/bigquery.rs
--- a/crates/tabletist-db/src/bigquery.rs
+++ b/crates/tabletist-db/src/bigquery.rs
@@ -705,6 +705,7 @@
         let has_more = rows.len() > limit;
         rows.truncate(limit);
         Ok(RowPage {
+            positions: None,
             columns,
             rows,
             has_more,
diff --git a/crates/tabletist-db/src/catalog.rs b/crates/tabletist-db/src/catalog.rs
--- a/crates/tabletist-db/src/catalog.rs
+++ b/crates/tabletist-db/src/catalog.rs
@@ -160,6 +160,10 @@
     /// The file a Files connection's table is read from; `None` for every
     /// other table.
     pub file: Option<crate::FileFacts>,
+    /// Why the table is not edited though its session may write, where a
+    /// Files connection says so: an Excel file's, a `.duckdb` file's, an
+    /// attached connection's.
+    pub unedited: Option<crate::Unedited>,
 }
 
 impl Structure {
diff --git a/crates/tabletist-db/src/dialect.rs b/crates/tabletist-db/src/dialect.rs
--- a/crates/tabletist-db/src/dialect.rs
+++ b/crates/tabletist-db/src/dialect.rs
@@ -2694,6 +2694,7 @@
             detail: None,
             hint: None,
             named: Box::new(crate::Named {
+                unsaved: None,
                 number,
                 constraint: None,
                 columns: Vec::new(),
diff --git a/crates/tabletist-db/src/duck.rs b/crates/tabletist-db/src/duck.rs
--- a/crates/tabletist-db/src/duck.rs
+++ b/crates/tabletist-db/src/duck.rs
@@ -8,12 +8,19 @@
 //! file, installing or loading an extension. And a read-only transaction
 //! around every page, count, lookup, script and agent's read.
 //!
+//! A session that may write saves edits to the files (`edit`): only a
+//! save writes, from Tabletist's own statements, which no script can run.
+//! Its fence also lets DuckDB write the hidden file each save of a
+//! Parquet, JSON or NDJSON file goes to first, and no other.
+//!
 //! Saved PostgreSQL, MySQL and SQLite connections it is told to attach are
 //! attached read-only before the fence (`attach`), and stay reachable
 //! after it: the fence keeps any other from being attached.
 
 mod attach;
-
+mod edit;
+
+use std::collections::HashMap;
 use std::path::Path;
 use std::sync::atomic::{AtomicBool, Ordering};
 use std::sync::{Arc, Mutex};
@@ -30,14 +37,19 @@
     Types,
 };
 use crate::{
-    CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, Ddl, Dialect, Error,
-    Lookup, LookupPage, MAX_LISTED, ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery,
-    ScriptMode, ScriptOutcome, StatementOutcome, StatementResult, StopFlag, Structure, Value,
-    ValueKind, WriteOutcome,
+    Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, Ddl,
+    Dialect, Error, Lookup, LookupPage, MAX_LISTED, ObjectInfo, ObjectKind, ObjectRef, Result,
+    RowPage, RowQuery, ScriptMode, ScriptOutcome, StatementOutcome, StatementResult, StopFlag,
+    Structure, Value, ValueKind, WriteOutcome,
 };
 
 /// The schema a connection's files are tables of.
 pub(crate) const MAIN: &str = "main";
+
+/// What a save finds a row of a file's table by, as a [`crate::RowChange`]'s
+/// key names it: the row's place in the file, from 0, as the page's
+/// `positions` say it.
+pub const FILE_POSITION: &str = "__tabletist_position";
 
 /// DuckDB's name for the in-memory database the views are in.
 const MEMORY: &str = "memory";
@@ -77,6 +89,18 @@
     /// connection's Reading CSV and the file's overrides included:
     /// `auto_detect = true, sample_size = 20000`.
     pub options: String,
+}
+
+/// Why a table of a Files connection is not edited, though its session
+/// may write.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Unedited {
+    /// An Excel file's: Excel files are never written.
+    Excel,
+    /// A `.duckdb` file's table or view.
+    DuckDb,
+    /// An attached connection's: those are attached read-only.
+    Attached,
 }
 
 /// An open Files connection.
@@ -98,6 +122,13 @@
     /// The saved connections it attached, in order.
     attached: Arc<Vec<Attached>>,
     version: String,
+    /// Whether a save may write the files.
+    access: Access,
+    /// How its CSV files are read, which a save writes them back by.
+    reading: Reading,
+    /// The files' tables read for editing so far, by name: those of a
+    /// session that may write whose page was read.
+    copies: Arc<Mutex<HashMap<String, edit::Loaded>>>,
 }
 
 /// A saved connection the session attached: the name SQL calls it by,
@@ -616,6 +647,7 @@
     past: bool,
     attaching: &[(Attaching, Option<u16>)],
     downloading: Option<&Downloading>,
+    writable: bool,
 ) -> Result<Made> {
     let files: Vec<&Found> = files
         .iter()
@@ -685,7 +717,8 @@
             }
             Types::Sample | Types::Text => reading,
         };
-        // A `.duckdb` file is attached read-only, as every session is.
+        // A `.duckdb` file is attached read-only, in a session that may
+        // write too: its tables are not edited yet.
         let made = make(&connection, file, reading).and_then(|(statement, options)| {
             run(&statement)?;
             Ok((statement, options))
@@ -758,6 +791,18 @@
         allowed.push(literal(&path));
     }
     allowed.extend(attached_files.iter().map(|path| literal(path)));
+    // A session that may write keeps its copies apart from the views, and
+    // may write the hidden file each save of a file DuckDB writes goes to
+    // first: none other.
+    if writable {
+        run(&format!("ATTACH ':memory:' AS {}", edit::EDITS))?;
+        for file in opened
+            .iter()
+            .filter(|file| edit::editable(file.found.format))
+        {
+            allowed.push(literal(&path_text(&edit::temp_path(&file.found.path))?));
+        }
+    }
     run(&format!("SET allowed_paths = [{}]", allowed.join(", ")))?;
     run("SET enable_external_access = false")?;
     run("SET lock_configuration = true")?;
@@ -1510,15 +1555,19 @@
     /// listed file that is gone, are its failures. The saved connections
     /// in `attaching` are attached after, each through the tunnel port
     /// beside it where it has one; `downloading` is told of each reader
-    /// DuckDB downloads first.
+    /// DuckDB downloads first. A session whose `access` may write saves
+    /// edits to the files a save writes back.
     pub(crate) async fn open(
         source: &Source,
         reading: &Reading,
         past: bool,
         attaching: Vec<(Attaching, Option<u16>)>,
         downloading: Option<Downloading>,
+        access: Access,
     ) -> Result<Self> {
         let (source, reading) = (source.clone(), reading.clone());
+        let kept = reading.clone();
+        let writable = access == Access::Writable;
         let made = tokio::task::spawn_blocking(move || {
             // A skipped file is not looked at: one that is gone, or that
             // cannot be looked at, is no matter.
@@ -1557,6 +1606,7 @@
                 past,
                 &attaching,
                 downloading.as_ref(),
+                writable,
             )
             .map(|(connection, files, mut failed, attached, version)| {
                 failed.extend(gone.iter().filter_map(|gone| {
@@ -1593,7 +1643,37 @@
             order,
             attached: Arc::new(attached),
             version,
+            access,
+            reading: kept,
+            copies: Arc::new(Mutex::new(HashMap::new())),
         })
+    }
+
+    /// The file `object` is read from, where a save may write it back.
+    fn editing(&self, object: &ObjectRef) -> Option<Opened> {
+        (self.access == Access::Writable)
+            .then(|| self.file_of(object))
+            .flatten()
+            .filter(|file| edit::editable(file.found.format))
+            .cloned()
+    }
+
+    /// Why `object` is not edited in a session that may write, where its
+    /// connection says so.
+    fn unedited(&self, object: &ObjectRef) -> Option<Unedited> {
+        if self.access != Access::Writable {
+            return None;
+        }
+        if let Some(file) = self.file_of(object) {
+            return (file.found.format == Format::Excel).then_some(Unedited::Excel);
+        }
+        if self.attached.iter().any(|each| each.name == object.schema) {
+            return Some(Unedited::Attached);
+        }
+        self.files
+            .iter()
+            .any(|file| file.found.format.is_database() && file.found.name == object.schema)
+            .then_some(Unedited::DuckDb)
     }
 
     /// Runs `work` on the blocking pool with the session.
@@ -1902,6 +1982,7 @@
             format: file.found.format,
             options: file.options.clone(),
         });
+        let unedited = self.unedited(object);
         let object = object.clone();
         let attached = Arc::clone(&self.attached);
         self.run(move |connection| {
@@ -1917,6 +1998,7 @@
             Ok(Structure {
                 columns,
                 file,
+                unedited,
                 ..Structure::default()
             })
         })
@@ -1975,14 +2057,42 @@
         .await
     }
 
+    /// A page of a file's table. In a session that may write, the first
+    /// page of a file a save writes back reads the file into its copy,
+    /// and every page is read from the copy, each row with its place.
     async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
         check_raw_where(query)?;
         let object = query.object.clone();
         let query = query.clone();
         let limit = query.limit as usize;
         let attached = Arc::clone(&self.attached);
+        let editing = self.editing(&object);
+        let copies = Arc::clone(&self.copies);
+        let reading = self.reading.clone();
         self.run(move |connection| {
+            let copied = match &editing {
+                Some(file) => {
+                    let mut copies = copies.lock().map_err(|_| {
+                        Error::ConnectionLost("the DuckDB session was poisoned".into())
+                    })?;
+                    // Read again where the file changed on disk since: a page
+                    // shows the file as it is.
+                    let fresh = copies
+                        .get(&file.found.name)
+                        .is_some_and(|loaded| !loaded.stale(&file.found.path));
+                    if !fresh {
+                        copies.remove(&file.found.name);
+                        let loaded = edit::load(connection, file, &reading)?;
+                        copies.insert(file.found.name.clone(), loaded);
+                    }
+                    matches!(copies.get(&file.found.name), Some(edit::Loaded::Copy(_)))
+                }
+                None => false,
+            };
             read_only(connection, |connection| {
+                if copied {
+                    return edit::page(connection, &query, limit);
+                }
                 let catalog = columns(connection, &query.object, &attached)?;
                 let binary = binary_of(&catalog);
                 // A file has no key: rows come in the file's order.
@@ -1997,6 +2107,7 @@
                     has_more,
                     ordered_by_key: false,
                     elapsed: started.elapsed(),
+                    positions: None,
                 })
             })
         })
@@ -2121,9 +2232,11 @@
         mode: ScriptMode,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
+        // A file is edited in its table and saved: a script reads, in a
+        // session that may write too.
         match mode {
             ScriptMode::ReadOnly => {}
-            // Editing files is not built: a Files session never writes.
+            ScriptMode::Write if self.access == Access::Writable => {}
             ScriptMode::Write => return Err(Error::ReadOnly),
         }
         let stop = stop.clone();
@@ -2174,9 +2287,27 @@
         Ok(Catalog::default())
     }
 
-    /// Editing files is not built.
-    async fn write(&self, _changes: &ChangeSet, _stop: &StopFlag) -> Result<WriteOutcome> {
-        Err(Error::ReadOnly)
+    /// Saves `changes` to the copy of the file their table reads, then to
+    /// the file (see `edit::save`). A table no save writes back is refused.
+    async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
+        let Some(file) = self.editing(&changes.object) else {
+            return Err(Error::ReadOnly);
+        };
+        let copies = Arc::clone(&self.copies);
+        let (changes, stop) = (changes.clone(), stop.clone());
+        self.run(move |connection| {
+            let mut copies = copies
+                .lock()
+                .map_err(|_| Error::ConnectionLost("the DuckDB session was poisoned".into()))?;
+            let Some(edit::Loaded::Copy(copy)) = copies.get_mut(&file.found.name) else {
+                return Err(Error::query(format!(
+                    "{} was not read for editing",
+                    file_label(&file.found)
+                )));
+            };
+            edit::save(connection, &file, copy, &changes, &stop)
+        })
+        .await
     }
 }
 
diff --git a/crates/tabletist-db/src/duck/edit.rs b/crates/tabletist-db/src/duck/edit.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-db/src/duck/edit.rs
@@ -0,0 +1,1009 @@
+//! Editing a Files connection's files. A file's table is read into an
+//! in-memory copy the first time a page of it is read in a session that
+//! may write, each row with its place in the file beside it: what a save
+//! finds the row by, since a file has no key. A save checks first that
+//! the file is as it was read, then changes the copy, writes the whole
+//! file to a hidden one in its folder and renames that over it.
+//!
+//! A CSV or TSV file is written by Tabletist: every row the save does not
+//! change is copied as it was, byte for byte, and a changed or new row is
+//! written with the file's delimiter and quote. So its rows are read here
+//! as DuckDB reads them, and a file whose rows this reading does not count
+//! as DuckDB did is not edited at all. Parquet, JSON and NDJSON are written
+//! by DuckDB from the copy, Parquet with the compression it had.
+
+use std::collections::BTreeMap;
+use std::io::{BufRead, BufReader, BufWriter, Write};
+use std::path::{Path, PathBuf};
+use std::time::{Instant, SystemTime};
+
+use super::{Opened, literal, map_error, path_text, reader_path, to_duckdb};
+use crate::files::{Format, Header, Reading};
+use crate::write::{Conflict, NewValue};
+use crate::{
+    ChangeSet, Dialect, Error, ObjectRef, Result, RowPage, RowQuery, StopFlag, Unsaved, Value,
+    WriteOutcome,
+};
+
+/// The in-memory database the copies are kept in, apart from the views.
+pub(super) const EDITS: &str = "tabletist_edits";
+
+/// The column of a copy that holds each row's place in the file.
+pub(super) const POSITION: &str = super::FILE_POSITION;
+
+/// The formats a save writes back.
+pub(super) fn editable(format: Format) -> bool {
+    format.rewritten()
+}
+
+/// The hidden file a save of `path` writes first, in its folder, which is
+/// then renamed over it.
+pub(super) fn temp_path(path: &Path) -> PathBuf {
+    let name = path
+        .file_name()
+        .map(|name| name.to_string_lossy().into_owned())
+        .unwrap_or_default();
+    path.with_file_name(format!(".{name}.tabletist-save"))
+}
+
+/// A file as it was when it was read: what a save compares it with.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub(super) struct Stamp {
+    size: u64,
+    modified: Option<SystemTime>,
+}
+
+impl Stamp {
+    fn of(path: &Path) -> std::io::Result<Self> {
+        let meta = std::fs::metadata(path)?;
+        Ok(Self {
+            size: meta.len(),
+            modified: meta.modified().ok(),
+        })
+    }
+}
+
+/// A file's table as a session that may write has read it.
+#[derive(Debug, Clone)]
+pub(super) enum Loaded {
+    Copy(Copy),
+    /// A CSV or TSV whose rows are not counted here as DuckDB counted
+    /// them, as it was when it was read: it cannot be written back row for
+    /// row, so it is not edited.
+    Unmatched(Stamp),
+}
+
+impl Loaded {
+    /// Whether the file at `path` changed on disk since it was read: the
+    /// next page reads it again.
+    pub(super) fn stale(&self, path: &Path) -> bool {
+        let stamp = match self {
+            Self::Copy(copy) => copy.stamp,
+            Self::Unmatched(stamp) => *stamp,
+        };
+        Stamp::of(path).ok() != Some(stamp)
+    }
+}
+
+/// A file's copy: where it is and what a save writes it back with.
+#[derive(Debug, Clone)]
+pub(super) struct Copy {
+    stamp: Stamp,
+    /// A CSV's or a TSV's own way of writing.
+    csv: Option<Layout>,
+    /// A Parquet file's compression, as `COPY` is told it.
+    compression: Option<String>,
+    /// The place the next new row takes.
+    next: i64,
+}
+
+impl Copy {
+    /// The rows in the copy, for the page's positions.
+    pub(super) fn object(name: &str) -> ObjectRef {
+        ObjectRef {
+            schema: EDITS.to_owned(),
+            name: name.to_owned(),
+        }
+    }
+}
+
+/// A page of a file's table, read from its copy, each row with its place:
+/// in the file's order, or that after the page's own sort.
+pub(super) fn page(
+    connection: &duckdb::Connection,
+    query: &RowQuery,
+    limit: usize,
+) -> Result<RowPage> {
+    let mut from = query.clone();
+    from.object = Copy::object(&query.object.name);
+    let catalog = super::columns(connection, &from.object, &[])?;
+    let binary = super::binary_of(&catalog);
+    let sql = Dialect::DuckDb.select_rows(&from, &[POSITION.to_owned()], &binary)?;
+    let started = Instant::now();
+    let params = sql.params.iter().map(to_duckdb).collect();
+    let (mut columns, mut rows, has_more) = super::rows_of(connection, &sql.text, params, limit)?;
+    let at = columns
+        .iter()
+        .position(|column| column.name == POSITION)
+        .ok_or_else(|| Error::query("the rows' places were not read"))?;
+    columns.remove(at);
+    let positions = rows
+        .iter_mut()
+        .map(|row| match row.remove(at) {
+            Value::Int(place) => Ok(place),
+            _ => Err(Error::query("a row's place was not read")),
+        })
+        .collect::<Result<Vec<_>>>()?;
+    super::catalog_types(&mut columns, &catalog);
+    Ok(RowPage {
+        columns,
+        rows,
+        has_more,
+        ordered_by_key: true,
+        elapsed: started.elapsed(),
+        positions: Some(positions),
+    })
+}
+
+/// How a CSV or a TSV file writes its rows, as DuckDB's sniffer and the
+/// connection's Reading CSV say.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub(super) struct Layout {
+    pub(super) delimiter: u8,
+    pub(super) quote: Option<u8>,
+    pub(super) escape: Option<u8>,
+    pub(super) header: bool,
+    pub(super) newline: &'static [u8],
+    pub(super) columns: usize,
+    /// How its dates and timestamps are written, where the sniffer found
+    /// a way: `%d/%m/%Y`.
+    pub(super) date: Option<String>,
+    pub(super) timestamp: Option<String>,
+}
+
+/// Reads `file` into its copy, from its own view. `None` where it reads
+/// the file but cannot edit it ([`Loaded::Unmatched`]).
+pub(super) fn load(
+    connection: &duckdb::Connection,
+    file: &Opened,
+    reading: &Reading,
+) -> Result<Loaded> {
+    let path = &file.found.path;
+    // Before it is read: a change made while it is read is one since.
+    let stamp = Stamp::of(path).map_err(|error| Error::Io(error.to_string()))?;
+    let copy = Dialect::DuckDb.qualified(&Copy::object(&file.found.name));
+    let view = Dialect::DuckDb.qualified(&ObjectRef {
+        schema: super::MAIN.to_owned(),
+        name: file.found.name.clone(),
+    });
+    let position = Dialect::DuckDb.quote_ident(POSITION);
+    // DuckDB keeps the file's order in the copy's rows: `rowid` is each
+    // one's place in the file, and stays its own when a row before it is
+    // deleted.
+    connection
+        .execute_batch(&format!(
+            "CREATE OR REPLACE TABLE {copy} AS SELECT * FROM {view}; \
+             ALTER TABLE {copy} ADD COLUMN {position} BIGINT; \
+             UPDATE {copy} SET {position} = rowid;"
+        ))
+        .map_err(map_error)?;
+    let rows: i64 = connection
+        .query_row(&format!("SELECT count(*) FROM {copy}"), [], |row| {
+            row.get(0)
+        })
+        .map_err(map_error)?;
+    let (csv, compression) = match file.found.format {
+        Format::Csv | Format::Tsv => {
+            let columns: usize = connection
+                .query_row(
+                    "SELECT count(*) FROM information_schema.columns \
+                     WHERE table_catalog = ? AND table_name = ? AND column_name <> ?",
+                    [EDITS, file.found.name.as_str(), POSITION],
+                    |row| row.get::<_, i64>(0),
+                )
+                .map_err(map_error)?
+                .try_into()
+                .unwrap_or(0);
+            let layout = layout(connection, path, file.found.format, reading, columns)?;
+            let counted = std::fs::File::open(path)
+                .and_then(|opened| count(BufReader::new(opened), &layout))
+                .map_err(|error| Error::Io(error.to_string()))?;
+            if i64::try_from(counted).ok() != Some(rows) {
+                connection
+                    .execute_batch(&format!("DROP TABLE {copy}"))
+                    .map_err(map_error)?;
+                return Ok(Loaded::Unmatched(stamp));
+            }
+            (Some(layout), None)
+        }
+        Format::Parquet => {
+            let compression: Option<String> = connection
+                .query_row(
+                    &format!(
+                        "SELECT compression FROM parquet_metadata({}) LIMIT 1",
+                        literal(&reader_path(path)?)
+                    ),
+                    [],
+                    |row| row.get(0),
+                )
+                .ok();
+            (None, compression.and_then(|named| compression_of(&named)))
+        }
+        Format::Json | Format::Ndjson | Format::Excel | Format::DuckDb => (None, None),
+    };
+    Ok(Loaded::Copy(Copy {
+        stamp,
+        csv,
+        compression,
+        next: rows,
+    }))
+}
+
+/// A Parquet file's compression as `parquet_metadata` names it, as `COPY`
+/// is told it; `None` for one it does not take, which then writes its own.
+fn compression_of(named: &str) -> Option<String> {
+    let named = named.to_ascii_lowercase();
+    [
+        "uncompressed",
+        "snappy",
+        "gzip",
+        "zstd",
+        "brotli",
+        "lz4",
+        "lz4_raw",
+    ]
+    .contains(&named.as_str())
+    .then_some(named)
+}
+
+/// How `path` writes its rows: DuckDB's sniffer, then what the connection's
+/// Reading CSV sets instead of detecting.
+fn layout(
+    connection: &duckdb::Connection,
+    path: &Path,
+    format: Format,
+    reading: &Reading,
+    columns: usize,
+) -> Result<Layout> {
+    let sniffed = connection
+        .query_row(
+            &format!(
+                "SELECT Delimiter, Quote, Escape, NewLineDelimiter, HasHeader, DateFormat, \
+                 TimestampFormat FROM sniff_csv({})",
+                literal(&reader_path(path)?)
+            ),
+            [],
+            |row| {
+                Ok((
+                    row.get::<_, Option<String>>(0)?,
+                    row.get::<_, Option<String>>(1)?,
+                    row.get::<_, Option<String>>(2)?,
+                    row.get::<_, Option<String>>(3)?,
+                    row.get::<_, Option<bool>>(4)?,
+                    row.get::<_, Option<String>>(5)?,
+                    row.get::<_, Option<String>>(6)?,
+                ))
+            },
+        )
+        .map_err(map_error)?;
+    let (delimiter, quote, escape, newline, header, date, timestamp) = sniffed;
+    // The sniffer writes a character it found none of as `(empty)`.
+    let byte = |text: Option<String>| -> Option<u8> {
+        let text = text?;
+        match text.as_str() {
+            "(empty)" | "" => None,
+            "\\t" => Some(b'\t'),
+            text => (text.len() == 1).then(|| text.as_bytes()[0]),
+        }
+    };
+    let delimiter = match (format, reading.delimiter.char()) {
+        (_, Some(set)) => u8::try_from(set).unwrap_or(b','),
+        (Format::Tsv, None) => b'\t',
+        _ => byte(delimiter).unwrap_or(b','),
+    };
+    let header = match reading.header {
+        Header::Yes => true,
+        Header::No => false,
+        Header::Detect => header.unwrap_or(true),
+    };
+    Ok(Layout {
+        delimiter,
+        quote: byte(quote),
+        escape: byte(escape),
+        header,
+        newline: match newline.as_deref() {
+            Some("\\r\\n") => b"\r\n",
+            _ => b"\n",
+        },
+        columns,
+        date: date.filter(|format| !format.is_empty()),
+        timestamp: timestamp.filter(|format| !format.is_empty()),
+    })
+}
+
+/// Reads a CSV file one record at a time, as DuckDB splits it: a line
+/// break inside quotes is part of the record.
+struct Records<R> {
+    reader: R,
+    quote: Option<u8>,
+    escape: Option<u8>,
+}
+
+impl<R: BufRead> Records<R> {
+    /// Puts the next record's bytes in `record` and the line break after
+    /// it in `brk` (none at the end of the file). `false` past the last.
+    fn next(&mut self, record: &mut Vec<u8>, brk: &mut Vec<u8>) -> std::io::Result<bool> {
+        record.clear();
+        brk.clear();
+        let (mut quoted, mut escaped, mut cr, mut any) = (false, false, false, false);
+        loop {
+            let buffer = self.reader.fill_buf()?;
+            if buffer.is_empty() {
+                if cr {
+                    brk.push(b'\r');
+                }
+                return Ok(any);
+            }
+            any = true;
+            let mut used = 0;
+            for &byte in buffer {
+                if cr {
+                    // A `\r` alone ends the record too; the byte after it is
+                    // the next record's.
+                    match byte {
+                        b'\n' => {
+                            brk.extend_from_slice(b"\r\n");
+                            used += 1;
+                        }
+                        _ => brk.push(b'\r'),
+                    }
+                    self.reader.consume(used);
+                    return Ok(true);
+                }
+                used += 1;
+                if quoted {
+                    record.push(byte);
+                    if escaped {
+                        escaped = false;
+                    } else if self.escape.is_some_and(|escape| escape == byte)
+                        && self.escape != self.quote
+                    {
+                        escaped = true;
+                    } else if self.quote == Some(byte) {
+                        // A doubled quote closes and opens again.
+                        quoted = false;
+                    }
+                    continue;
+                }
+                match byte {
+                    b'\n' => {
+                        brk.push(b'\n');
+                        self.reader.consume(used);
+                        return Ok(true);
+                    }
+                    b'\r' => cr = true,
+                    byte => {
+                        if self.quote == Some(byte) {
+                            quoted = true;
+                        }
+                        record.push(byte);
+                    }
+                }
+            }
+            self.reader.consume(used);
+        }
+    }
+}
+
+/// Whether DuckDB reads `record` as no row: an empty line, where a row has
+/// more than one column. With one column it is a row whose value is NULL.
+fn blank(record: &[u8], layout: &Layout) -> bool {
+    record.is_empty() && layout.columns > 1
+}
+
+/// Strips a UTF-8 byte-order mark off the start, which is written back as
+/// it was.
+fn mark<R: BufRead>(reader: &mut R) -> std::io::Result<&'static [u8]> {
+    let start = reader.fill_buf()?;
+    if start.starts_with(b"\xEF\xBB\xBF") {
+        reader.consume(3);
+        return Ok(b"\xEF\xBB\xBF");
+    }
+    Ok(b"")
+}
+
+/// The rows of a CSV file, counted as DuckDB counts them.
+fn count(mut reader: impl BufRead, layout: &Layout) -> std::io::Result<u64> {
+    mark(&mut reader)?;
+    let mut records = Records {
+        reader,
+        quote: layout.quote,
+        escape: layout.escape,
+    };
+    let (mut record, mut brk) = (Vec::new(), Vec::new());
+    let (mut header, mut rows) = (layout.header, 0);
+    while records.next(&mut record, &mut brk)? {
+        if blank(&record, layout) {
+            continue;
+        }
+        if header {
+            header = false;
+            continue;
+        }
+        rows += 1;
+    }
+    Ok(rows)
+}
+
+/// A save of a CSV file: by each changed row's place, what becomes of it.
+#[derive(Debug, Default)]
+struct Plan {
+    deleted: std::collections::BTreeSet<u64>,
+    /// Each changed row's fields, by their place, with their new text.
+    changed: BTreeMap<u64, Vec<(usize, String)>>,
+    /// The new rows, every field's text in the columns' order.
+    added: Vec<Vec<String>>,
+}
+
+/// Writes the CSV file `reader` reads to `to`, as `plan` changes it.
+fn rewrite(
+    mut reader: impl BufRead,
+    to: &mut impl Write,
+    layout: &Layout,
+    plan: &Plan,
+) -> Result<()> {
+    let io = |error: std::io::Error| Error::Io(error.to_string());
+    to.write_all(mark(&mut reader).map_err(io)?).map_err(io)?;
+    let mut records = Records {
+        reader,
+        quote: layout.quote,
+        escape: layout.escape,
+    };
+    let (mut record, mut brk) = (Vec::new(), Vec::new());
+    let (mut header, mut row, mut ended) = (layout.header, 0_u64, true);
+    while records.next(&mut record, &mut brk).map_err(io)? {
+        ended = !brk.is_empty();
+        if blank(&record, layout) || std::mem::take(&mut header) {
+            to.write_all(&record).map_err(io)?;
+            to.write_all(&brk).map_err(io)?;
+            continue;
+        }
+        let at = row;
+        row += 1;
+        if plan.deleted.contains(&at) {
+            continue;
+        }
+        match plan.changed.get(&at) {
+            Some(cells) => {
+                let mut fields: Vec<Vec<u8>> = split(&record, layout)
+                    .into_iter()
+                    .map(|field| record[field].to_vec())
+                    .collect();
+                for (place, text) in cells {
+                    let Some(field) = fields.get_mut(*place) else {
+                        return Err(Error::query(format!(
+                            "row {} has fewer fields than the file's columns",
+                            at + 1
+                        )));
+                    };
+                    *field = written(text, layout);
+                }
+                to.write_all(&fields.join(&layout.delimiter)).map_err(io)?;
+            }
+            None => to.write_all(&record).map_err(io)?,
+        }
+        to.write_all(&brk).map_err(io)?;
+    }
+    for (index, added) in plan.added.iter().enumerate() {
+        if index == 0 && !ended {
+            to.write_all(layout.newline).map_err(io)?;
+        }
+        let fields: Vec<Vec<u8>> = added.iter().map(|text| written(text, layout)).collect();
+        to.write_all(&fields.join(&layout.delimiter)).map_err(io)?;
+        to.write_all(layout.newline).map_err(io)?;
+    }
+    Ok(())
+}
+
+/// The places of `record`'s fields, split at the delimiter outside quotes.
+fn split(record: &[u8], layout: &Layout) -> Vec<std::ops::Range<usize>> {
+    let (mut fields, mut start, mut quoted, mut escaped) = (Vec::new(), 0, false, false);
+    for (at, &byte) in record.iter().enumerate() {
+        if quoted {
+            if escaped {
+                escaped = false;
+            } else if layout.escape == Some(byte) && layout.escape != layout.quote {
+                escaped = true;
+            } else if layout.quote == Some(byte) {
+                quoted = false;
+            }
+        } else if layout.quote == Some(byte) {
+            quoted = true;
+        } else if byte == layout.delimiter {
+            fields.push(start..at);
+            start = at + 1;
+        }
+    }
+    fields.push(start..record.len());
+    fields
+}
+
+/// `text` as a field of the file: quoted where it holds the delimiter, a
+/// quote or a line break, or starts or ends with a space.
+fn written(text: &str, layout: &Layout) -> Vec<u8> {
+    let quote = layout.quote.unwrap_or(b'"');
+    let bytes = text.as_bytes();
+    let needs = bytes
+        .iter()
+        .any(|&byte| byte == layout.delimiter || byte == quote || byte == b'\n' || byte == b'\r')
+        || text.starts_with(' ')
+        || text.ends_with(' ');
+    if !needs {
+        return bytes.to_vec();
+    }
+    let escape = layout.escape.unwrap_or(quote);
+    let mut out = Vec::with_capacity(bytes.len() + 2);
+    out.push(quote);
+    for &byte in bytes {
+        if byte == quote || (byte == escape && escape != quote) {
+            out.push(escape);
+        }
+        out.push(byte);
+    }
+    out.push(quote);
+    out
+}
+
+/// What became of `changes`, to the copy of `file` and the file itself.
+/// Nothing is written where the file changed since it was read, or its
+/// folder cannot be written to: the copy is as it was, and the save says
+/// why ([`Unsaved`]).
+pub(super) fn save(
+    connection: &duckdb::Connection,
+    file: &Opened,
+    copy: &mut Copy,
+    changes: &ChangeSet,
+    stop: &StopFlag,
+) -> Result<WriteOutcome> {
+    let started = Instant::now();
+    let path = &file.found.path;
+    let label = crate::files::file_name(path);
+    let unsaved = |why: Unsaved, message: String| {
+        let mut error = Error::query(message);
+        if let Error::Query { named, .. } = &mut error {
+            named.unsaved = Some(why);
+        }
+        error
+    };
+    if Stamp::of(path).ok() != Some(copy.stamp) {
+        return Err(unsaved(
+            Unsaved::Changed(label.clone()),
+            format!("{label} changed on disk since it was opened"),
+        ));
+    }
+    connection
+        .execute_batch("BEGIN TRANSACTION")
+        .map_err(map_error)?;
+    let outcome = changed(connection, file, copy, changes, stop).and_then(|outcome| {
+        if !matches!(outcome, WriteOutcome::Written { .. }) {
+            return Ok(outcome);
+        }
+        write_file(connection, file, copy, changes).map_err(|error| match error {
+            WriteError::Io(error) => unsaved(
+                Unsaved::Unwritable(label.clone()),
+                format!("{label} could not be written: {error}"),
+            ),
+            WriteError::Db(error) => error,
+        })?;
+        Ok(outcome)
+    });
+    match outcome {
+        Ok(WriteOutcome::Written {
+            inserted,
+            rows,
+            gone,
+            ..
+        }) => {
+            // Each row's place in the file as it is now: rows after a
+            // deleted one move up, and new rows are last.
+            let position = Dialect::DuckDb.quote_ident(POSITION);
+            let table = Dialect::DuckDb.qualified(&Copy::object(&file.found.name));
+            let renumbered = connection
+                .execute_batch(&format!(
+                    "UPDATE {table} SET {position} = placed.at FROM \
+                     (SELECT {position} AS was, row_number() OVER (ORDER BY {position}) - 1 AS at \
+                     FROM {table}) AS placed WHERE {table}.{position} = placed.was"
+                ))
+                .and_then(|()| connection.execute_batch("COMMIT"));
+            if let Err(error) = renumbered {
+                let _ = connection.execute_batch("ROLLBACK");
+                return Err(map_error(error));
+            }
+            copy.next = connection
+                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
+                    row.get(0)
+                })
+                .map_err(map_error)?;
+            copy.stamp = Stamp::of(path).map_err(|error| Error::Io(error.to_string()))?;
+            Ok(WriteOutcome::Written {
+                inserted,
+                rows,
+                gone,
+                elapsed: started.elapsed(),
+            })
+        }
+        other => {
+            let _ = connection.execute_batch("ROLLBACK");
+            other
+        }
+    }
+}
+
+/// Changes the copy as `changes` say, in the transaction `save` began.
+fn changed(
+    connection: &duckdb::Connection,
+    file: &Opened,
+    copy: &Copy,
+    changes: &ChangeSet,
+    stop: &StopFlag,
+) -> Result<WriteOutcome> {
+    let object = Copy::object(&file.found.name);
+    let dialect = Dialect::DuckDb;
+    let stopped = || {
+        if stop.is_stopped() {
+            return Err(Error::Cancelled);
+        }
+        Ok(())
+    };
+    let run = |sql: &crate::Sql| -> Result<usize> {
+        let params = duckdb::params_from_iter(sql.params.iter().map(to_duckdb));
+        connection.execute(&sql.text, params).map_err(map_error)
+    };
+    for (insert, row) in changes.inserts.iter().enumerate() {
+        stopped()?;
+        let mut row = row.clone();
+        row.set.push(crate::InsertValue {
+            column: POSITION.to_owned(),
+            type_name: "BIGINT".to_owned(),
+            new: NewValue::Text((copy.next + insert as i64).to_string()),
+        });
+        let statement = dialect.insert_row(&object, &row);
+        let done = statement.and_then(|statement| {
+            let params = duckdb::params_from_iter(statement.sql.params.iter().map(to_duckdb));
+            let mut prepared = connection.prepare(&statement.sql.text).map_err(map_error)?;
+            let mut rows = prepared.query(params).map_err(map_error)?;
+            while rows.next().map_err(map_error)?.is_some() {}
+            Ok(())
+        });
+        if let Err(error) = done {
+            return Ok(WriteOutcome::FailedInsert { insert, error });
+        }
+    }
+    let mut conflicts = Vec::new();
+    for (row, change) in changes.rows.iter().enumerate() {
+        stopped()?;
+        match dialect
+            .update_row(&object, change)
+            .and_then(|update| run(&update.sql))
+        {
+            Ok(0) => conflicts.push(Conflict { row, server: None }),
+            Ok(_) => {}
+            Err(error) => return Ok(WriteOutcome::Failed { row, error }),
+        }
+    }
+    if !conflicts.is_empty() {
+        return Ok(WriteOutcome::Conflicts(conflicts));
+    }
+    let mut gone = Vec::new();
+    for (delete, row) in changes.deletes.iter().enumerate() {
+        stopped()?;
+        match dialect
+            .delete_row(&object, row)
+            .and_then(|deletion| run(&deletion.sql))
+        {
+            Ok(0) => gone.push(delete),
+            Ok(_) => {}
+            Err(error) => return Ok(WriteOutcome::FailedDelete { delete, error }),
+        }
+    }
+    stopped()?;
+    // Each changed row as the copy now holds it, in the page's columns.
+    let table = dialect.qualified(&object);
+    let position = dialect.quote_ident(POSITION);
+    let mut rows = Vec::with_capacity(changes.rows.len());
+    for change in &changes.rows {
+        let Some(at) = place_of(&change.key) else {
+            continue;
+        };
+        let sql = format!("SELECT * EXCLUDE ({position}) FROM {table} WHERE {position} = {at}");
+        let (_, mut found, _) = super::rows_of(connection, &sql, Vec::new(), 1)?;
+        rows.extend(found.pop());
+    }
+    Ok(WriteOutcome::Written {
+        inserted: vec![None; changes.inserts.len()],
+        rows,
+        gone,
+        elapsed: std::time::Duration::ZERO,
+    })
+}
+
+/// The place a row's key names, where it is the copy's own.
+fn place_of(key: &[(String, Value)]) -> Option<i64> {
+    match key {
+        [(column, Value::Int(at))] if column == POSITION => Some(*at),
+        _ => None,
+    }
+}
+
+enum WriteError {
+    /// The file or its folder could not be written.
+    Io(std::io::Error),
+    Db(Error),
+}
+
+/// Writes the copy, changed, to the hidden file beside `file`, then
+/// renames it over `file`. The hidden file is gone after, whatever came of
+/// it.
+fn write_file(
+    connection: &duckdb::Connection,
+    file: &Opened,
+    copy: &Copy,
+    changes: &ChangeSet,
+) -> std::result::Result<(), WriteError> {
+    let path = &file.found.path;
+    let temp = temp_path(path);
+    let written = match &copy.csv {
+        Some(layout) => write_csv(connection, file, layout, changes, &temp),
+        None => write_copy(connection, file, copy, &temp),
+    };
+    let renamed = written.and_then(|()| {
+        // The file keeps its own permissions.
+        if let Ok(meta) = std::fs::metadata(path) {
+            std::fs::set_permissions(&temp, meta.permissions()).map_err(WriteError::Io)?;
+        }
+        std::fs::rename(&temp, path).map_err(WriteError::Io)
+    });
+    if renamed.is_err() {
+        let _ = std::fs::remove_file(&temp);
+    }
+    renamed
+}
+
+/// A Parquet, JSON or NDJSON file, written by DuckDB from the copy.
+fn write_copy(
+    connection: &duckdb::Connection,
+    file: &Opened,
+    copy: &Copy,
+    temp: &Path,
+) -> std::result::Result<(), WriteError> {
+    let options = match file.found.format {
+        Format::Parquet => match &copy.compression {
+            Some(compression) => format!("FORMAT parquet, COMPRESSION {compression}"),
+            None => "FORMAT parquet".to_owned(),
+        },
+        Format::Json => "FORMAT json, ARRAY true".to_owned(),
+        Format::Ndjson => "FORMAT json".to_owned(),
+        Format::Csv | Format::Tsv | Format::Excel | Format::DuckDb => {
+            return Err(WriteError::Db(Error::query(
+                "this file is not written by DuckDB",
+            )));
+        }
+    };
+    let position = Dialect::DuckDb.quote_ident(POSITION);
+    let table = Dialect::DuckDb.qualified(&Copy::object(&file.found.name));
+    let to = path_text(temp).map_err(WriteError::Db)?;
+    // DuckDB's own error for a folder it cannot write to.
+    connection
+        .execute_batch(&format!(
+            "COPY (SELECT * EXCLUDE ({position}) FROM {table} ORDER BY {position}) TO {} ({options})",
+            literal(&to)
+        ))
+        .map_err(|error| {
+            let error = map_error(error);
+            match error.to_string().contains("IO Error") || error.to_string().contains("Permission") {
+                true => WriteError::Io(std::io::Error::other(error.to_string())),
+                false => WriteError::Db(error),
+            }
+        })
+}
+
+/// A CSV or TSV file, written row for row as [`rewrite`] does: what a save
+/// changes is read from the copy, its dates as the file writes them.
+fn write_csv(
+    connection: &duckdb::Connection,
+    file: &Opened,
+    layout: &Layout,
+    changes: &ChangeSet,
+    temp: &Path,
+) -> std::result::Result<(), WriteError> {
+    let plan = plan(connection, file, layout, changes).map_err(WriteError::Db)?;
+    let source = std::fs::File::open(&file.found.path).map_err(WriteError::Io)?;
+    let target = std::fs::File::create(temp).map_err(WriteError::Io)?;
+    let mut target = BufWriter::new(target);
+    rewrite(BufReader::new(source), &mut target, layout, &plan).map_err(|error| match error {
+        Error::Io(message) => WriteError::Io(std::io::Error::other(message)),
+        error => WriteError::Db(error),
+    })?;
+    target.flush().map_err(WriteError::Io)?;
+    target
+        .into_inner()
+        .map_err(|error| WriteError::Io(error.into_error()))?
+        .sync_all()
+        .map_err(WriteError::Io)
+}
+
+/// What the save writes of each row it changes, deletes or adds. A cell
+/// set to text is written as it was typed, but for a date or a timestamp
+/// in a file that writes those its own way; NULL and a default are an
+/// empty field.
+fn plan(
+    connection: &duckdb::Connection,
+    file: &Opened,
+    layout: &Layout,
+    changes: &ChangeSet,
+) -> Result<Plan> {
+    let object = Copy::object(&file.found.name);
+    let table = Dialect::DuckDb.qualified(&object);
+    let position = Dialect::DuckDb.quote_ident(POSITION);
+    let columns = super::columns(connection, &object, &[])?;
+    let columns: Vec<_> = columns
+        .into_iter()
+        .filter(|column| column.name != POSITION)
+        .collect();
+    // The text the copy holds in a column for a row, in the file's way.
+    let text_at = |column: &crate::ColumnInfo, at: i64| -> Result<String> {
+        let name = Dialect::DuckDb.quote_ident(&column.name);
+        let kind = column.type_name.to_ascii_uppercase();
+        let expression = match (&layout.date, &layout.timestamp) {
+            (Some(date), _) if kind == "DATE" => format!("strftime({name}, {})", literal(date)),
+            (_, Some(stamp)) if kind.starts_with("TIMESTAMP") => {
+                format!("strftime({name}, {})", literal(stamp))
+            }
+            _ => format!("CAST({name} AS VARCHAR)"),
+        };
+        let text: Option<String> = connection
+            .query_row(
+                &format!("SELECT {expression} FROM {table} WHERE {position} = {at}"),
+                [],
+                |row| row.get(0),
+            )
+            .map_err(map_error)?;
+        Ok(text.unwrap_or_default())
+    };
+    let own_way = |type_name: &str| {
+        let kind = type_name.to_ascii_uppercase();
+        (kind == "DATE" && layout.date.is_some())
+            || (kind.starts_with("TIMESTAMP") && layout.timestamp.is_some())
+    };
+    let cell = |column: &crate::ColumnInfo, new: &NewValue, at: i64| -> Result<String> {
+        match new {
+            NewValue::Null | NewValue::Default { .. } => Ok(String::new()),
+            NewValue::Text(text) if !own_way(&column.type_name) => Ok(text.clone()),
+            NewValue::Text(_) | NewValue::Now => text_at(column, at),
+        }
+    };
+    let mut plan = Plan::default();
+    for row in &changes.rows {
+        let at = place_of(&row.key).ok_or_else(|| Error::query("a row to save has no place"))?;
+        let mut cells = Vec::with_capacity(row.set.len());
+        for change in &row.set {
+            let place = columns
+                .iter()
+                .position(|column| column.name == change.column)
+                .ok_or_else(|| Error::query(format!("no such column: {}", change.column)))?;
+            cells.push((place, cell(&columns[place], &change.new, at)?));
+        }
+        plan.changed
+            .insert(u64::try_from(at).unwrap_or(u64::MAX), cells);
+    }
+    for row in &changes.deletes {
+        let at = place_of(&row.key).ok_or_else(|| Error::query("a row to delete has no place"))?;
+        plan.deleted.insert(u64::try_from(at).unwrap_or(u64::MAX));
+    }
+    // The new rows, last in the copy in their order.
+    let first: i64 = connection
+        .query_row(
+            &format!("SELECT coalesce(max({position}), -1) FROM {table}"),
+            [],
+            |row| row.get(0),
+        )
+        .map_err(map_error)?;
+    let first = first + 1 - changes.inserts.len() as i64;
+    for (index, insert) in changes.inserts.iter().enumerate() {
+        let at = first + index as i64;
+        let mut fields = Vec::with_capacity(columns.len());
+        for column in &columns {
+            let text = match insert.set.iter().find(|set| set.column == column.name) {
+                Some(set) => cell(column, &set.new, at)?,
+                None => String::new(),
+            };
+            fields.push(text);
+        }
+        plan.added.push(fields);
+    }
+    Ok(plan)
+}
+
+#[cfg(test)]
+#[allow(clippy::unwrap_used)]
+mod tests {
+    use super::*;
+
+    fn layout(columns: usize) -> Layout {
+        Layout {
+            delimiter: b',',
+            quote: Some(b'"'),
+            escape: Some(b'"'),
+            header: true,
+            newline: b"\n",
+            columns,
+            date: None,
+            timestamp: None,
+        }
+    }
+
+    fn rewritten(text: &str, layout: &Layout, plan: &Plan) -> String {
+        let mut out = Vec::new();
+        rewrite(text.as_bytes(), &mut out, layout, plan).unwrap();
+        String::from_utf8(out).unwrap()
+    }
+
+    #[test]
+    fn rows_are_counted_as_duckdb_reads_them() {
+        let cases = [
+            ("id,title\n1,Dune\n2,Emma\n", 2, 2),
+            ("id,title\n1,Dune\n\n2,Emma\n\n", 2, 2),
+            ("id,title\n1,\"Du\nne\"\n2,Emma\n", 2, 2),
+            ("id,title\r\n1,Dune\r\n2,Emma\r\n", 2, 2),
+            ("\u{feff}id,title\n1,Dune\n2,Emma", 2, 2),
+            ("\"id\",\"title\"\n\"1\",\"Dune, \"\"the\"\" book\"\n", 2, 1),
+            ("title\nDune\n\nEmma\n", 1, 3),
+        ];
+        for (text, columns, rows) in cases {
+            assert_eq!(
+                count(text.as_bytes(), &layout(columns)).unwrap(),
+                rows,
+                "{text:?}"
+            );
+        }
+    }
+
+    #[test]
+    fn a_save_keeps_every_untouched_row_as_it_was_written() {
+        let text = "\u{feff}id,title,price\r\n1,\"Dune\",12.50\r\n\r\n2,Emma,9.00\r\n3,\"Ulysses, a novel\",15.00\r\n";
+        let mut layout = layout(3);
+        layout.newline = b"\r\n";
+        let mut plan = Plan::default();
+        plan.changed.insert(1, vec![(2, "8.5".into())]);
+        plan.deleted.insert(0);
+        plan.added
+            .push(vec!["4".into(), "Beloved, a novel".into(), String::new()]);
+        assert_eq!(
+            rewritten(text, &layout, &plan),
+            "\u{feff}id,title,price\r\n\r\n2,Emma,8.5\r\n3,\"Ulysses, a novel\",15.00\r\n\
+             4,\"Beloved, a novel\",\r\n"
+        );
+    }
+
+    #[test]
+    fn a_new_row_starts_on_a_line_of_its_own() {
+        let mut plan = Plan::default();
+        plan.added.push(vec!["3".into(), "Emma \"E\"".into()]);
+        assert_eq!(
+            rewritten("id,title\n1,Dune", &layout(2), &plan),
+            "id,title\n1,Dune\n3,\"Emma \"\"E\"\"\"\n"
+        );
+    }
+
+    #[test]
+    fn a_tsv_keeps_its_tabs_and_a_field_s_quotes() {
+        let mut layout = layout(2);
+        layout.delimiter = b'\t';
+        let mut plan = Plan::default();
+        plan.changed.insert(0, vec![(1, "Dune, Messiah".into())]);
+        assert_eq!(
+            rewritten("id\ttitle\n1\t\"Dune\"\n2\t\"Emma\"\n", &layout, &plan),
+            "id\ttitle\n1\tDune, Messiah\n2\t\"Emma\"\n"
+        );
+    }
+}
diff --git a/crates/tabletist-db/src/error.rs b/crates/tabletist-db/src/error.rs
--- a/crates/tabletist-db/src/error.rs
+++ b/crates/tabletist-db/src/error.rs
@@ -23,6 +23,18 @@
     /// The file a Files connection's table could not be read from, and
     /// why, where DuckDB's words name a row, a value or a line of it.
     pub file: Option<crate::files::Failure>,
+    /// Why a save to a Files connection's file wrote nothing, where it is
+    /// the file's own doing: the person's changes stay as they were.
+    pub unsaved: Option<Unsaved>,
+}
+
+/// Why a save to a file wrote nothing, by the file's name.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum Unsaved {
+    /// The file changed on disk since its table was read.
+    Changed(String),
+    /// The file, or its folder, could not be written to.
+    Unwritable(String),
 }
 
 /// Everything that can go wrong talking to a database. Messages are shown to
diff --git a/crates/tabletist-db/src/files.rs b/crates/tabletist-db/src/files.rs
--- a/crates/tabletist-db/src/files.rs
+++ b/crates/tabletist-db/src/files.rs
@@ -323,6 +323,15 @@
             Self::Csv | Self::Tsv | Self::Parquet | Self::Json | Self::Ndjson | Self::Excel => {
                 false
             }
+        }
+    }
+
+    /// Whether a save of an edit writes the whole file again. An Excel
+    /// file is never written, and a `.duckdb` file is not yet.
+    pub fn rewritten(self) -> bool {
+        match self {
+            Self::Csv | Self::Tsv | Self::Parquet | Self::Json | Self::Ndjson => true,
+            Self::Excel | Self::DuckDb => false,
         }
     }
 }
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -59,8 +59,8 @@
     DeleteParts, Dialect, InsertParts, InsertStatement, RowDeletion, RowUpdate, Sql, UpdateParts,
     escape_like, quote_literal,
 };
-pub use duck::{FileFacts, FilesRead, set_extension_directory};
-pub use error::{Error, Named, Result, SshStage};
+pub use duck::{FILE_POSITION, FileFacts, FilesRead, Unedited, set_extension_directory};
+pub use error::{Error, Named, Result, SshStage, Unsaved};
 pub use lookup::{Lookup, LookupPage, LookupPart, LookupRow, Matched};
 pub use query::{Filter, FilterOp, RowPage, RowQuery, Sort, SortDir};
 pub use script::{
@@ -138,7 +138,7 @@
                     .as_ref()
                     .ok_or_else(|| Error::InvalidSpec("choose a folder or files".into()))?;
                 Ok(Self::Files(Box::new(
-                    duck::Conn::open(source, &spec.csv, false, Vec::new(), None).await?,
+                    duck::Conn::open(source, &spec.csv, false, Vec::new(), None, access).await?,
                 )))
             }
         }
@@ -169,11 +169,9 @@
         // BigQuery has no transactions or row locks to save rows with,
         // and no read-only one to fence a script's writes: a session is
         // read-only whatever was asked, and its adapter runs only reads.
-        // Editing files is not built: a Files session reads, whatever
-        // access it is asked for, and says so.
         let access = match spec.driver {
-            Driver::BigQuery | Driver::Files => Access::ReadOnly,
-            Driver::Sqlite | Driver::Postgres | Driver::MySql => access,
+            Driver::BigQuery => Access::ReadOnly,
+            Driver::Sqlite | Driver::Postgres | Driver::MySql | Driver::Files => access,
         };
         let Some(ssh) = ssh else {
             return Ok(Self {
@@ -633,7 +631,15 @@
     /// where a session to browse does not open with one. Nothing is
     /// attached: see [`Connection::open_files_to_test_with`].
     pub async fn open_files_to_test(spec: &ConnectSpec) -> Result<Self> {
-        Self::open_files(spec, &[], &HostKeys::default(), true, None).await
+        Self::open_files(
+            spec,
+            &[],
+            &HostKeys::default(),
+            true,
+            None,
+            Access::ReadOnly,
+        )
+        .await
     }
 
     /// [`Connection::open_files_to_test`], with the saved connections in
@@ -644,21 +650,32 @@
         host_keys: &HostKeys,
         downloading: Option<files::Downloading>,
     ) -> Result<Self> {
-        Self::open_files(spec, attaching, host_keys, true, downloading).await
-    }
-
-    /// A Files connection's session, read-only, with the saved
-    /// connections in `attaching` attached read-only under their names,
-    /// each through its own SSH tunnel where it has one (`host_keys` are
-    /// those the user trusts). One that cannot be attached fails it.
-    /// `downloading` is told of each reader DuckDB downloads first.
+        Self::open_files(
+            spec,
+            attaching,
+            host_keys,
+            true,
+            downloading,
+            Access::ReadOnly,
+        )
+        .await
+    }
+
+    /// A Files connection's session, with the saved connections in
+    /// `attaching` attached read-only under their names, each through its
+    /// own SSH tunnel where it has one (`host_keys` are those the user
+    /// trusts). One that cannot be attached fails it. `access` says whether
+    /// a save may write the files; an attached connection is read-only
+    /// either way. `downloading` is told of each reader DuckDB downloads
+    /// first.
     pub async fn connect_files(
         spec: &ConnectSpec,
         attaching: &[files::Attaching],
         host_keys: &HostKeys,
+        access: Access,
         downloading: Option<files::Downloading>,
     ) -> Result<Self> {
-        Self::open_files(spec, attaching, host_keys, false, downloading).await
+        Self::open_files(spec, attaching, host_keys, false, downloading, access).await
     }
 
     async fn open_files(
@@ -667,6 +684,7 @@
         host_keys: &HostKeys,
         past: bool,
         downloading: Option<files::Downloading>,
+        access: Access,
     ) -> Result<Self> {
         let source = spec
             .files
@@ -709,7 +727,7 @@
             };
             each.push((attaching.clone(), via));
         }
-        let opened = duck::Conn::open(source, &spec.csv, past, each, downloading).await;
+        let opened = duck::Conn::open(source, &spec.csv, past, each, downloading, access).await;
         let conn = match opened {
             Ok(conn) => conn,
             // Where an attach failed, a tunnel's own words say more than
@@ -726,7 +744,7 @@
         let tunnels = tunnels.into_iter().map(|(_, tunnel)| tunnel).collect();
         Ok(Self {
             inner: Inner::Files(Box::new(conn)),
-            access: Access::ReadOnly,
+            access,
             tunnel: None,
             _tunnels: tunnels,
         })
diff --git a/crates/tabletist-db/src/mysql.rs b/crates/tabletist-db/src/mysql.rs
--- a/crates/tabletist-db/src/mysql.rs
+++ b/crates/tabletist-db/src/mysql.rs
@@ -575,6 +575,7 @@
             }
         }
         Structure {
+            unedited: None,
             columns,
             primary_key: self.primary_key,
             indexes,
@@ -1309,6 +1310,7 @@
         let has_more = rows.len() > limit;
         rows.truncate(limit);
         RowPage {
+            positions: None,
             columns,
             rows,
             has_more,
@@ -3261,6 +3263,7 @@
         _ => (None, Vec::new()),
     };
     Named {
+        unsaved: None,
         number: Some(number),
         constraint,
         columns,
diff --git a/crates/tabletist-db/src/pg.rs b/crates/tabletist-db/src/pg.rs
--- a/crates/tabletist-db/src/pg.rs
+++ b/crates/tabletist-db/src/pg.rs
@@ -444,6 +444,7 @@
             // language of its messages. It names a NOT NULL failure's
             // column, and the constraint of the others.
             named: Box::new(Named {
+                unsaved: None,
                 number: None,
                 constraint: db.constraint().map(str::to_owned),
                 columns: db.column().map(str::to_owned).into_iter().collect(),
@@ -1084,6 +1085,7 @@
             Some(_) => self.primary_key(object).await?,
         };
         Ok(Structure {
+            unedited: None,
             columns,
             primary_key,
             indexes,
@@ -1196,6 +1198,7 @@
     let has_more = rows.len() > limit;
     rows.truncate(limit);
     Ok(RowPage {
+        positions: None,
         columns,
         rows,
         has_more,
diff --git a/crates/tabletist-db/src/query.rs b/crates/tabletist-db/src/query.rs
--- a/crates/tabletist-db/src/query.rs
+++ b/crates/tabletist-db/src/query.rs
@@ -120,4 +120,8 @@
     /// Whether rows are ordered by a key, so paging is stable.
     pub ordered_by_key: bool,
     pub elapsed: Duration,
+    /// Each row's place in its file, for a Files connection's table that
+    /// can be edited: what a save finds the row by, since a file has no
+    /// key. `None` for every other page.
+    pub positions: Option<Vec<i64>>,
 }
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -1081,6 +1081,7 @@
                 )));
             }
             Ok(Structure {
+                unedited: None,
                 columns,
                 primary_key: primary_key(connection, &object)?,
                 indexes: indexes(connection, &object)?,
@@ -1201,6 +1202,7 @@
             values.truncate(limit);
             let columns = column_metas(declared, &values);
             Ok(RowPage {
+                positions: None,
                 columns,
                 rows: values,
                 has_more,
diff --git a/crates/tabletist-db/tests/files.rs b/crates/tabletist-db/tests/files.rs
--- a/crates/tabletist-db/tests/files.rs
+++ b/crates/tabletist-db/tests/files.rs
@@ -1,7 +1,8 @@
 //! Files connections through the public `Connection` API, on the
 //! Bookshop's exports (`fixtures::write_bookshop_files`): a view for each
 //! file, a `.duckdb` file as a schema, and a session that reads those
-//! files and nothing else, and writes nothing.
+//! files and nothing else. A read-only one writes nothing; editing is in
+//! `files_edit.rs`.
 
 // `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
 #![allow(clippy::unwrap_used)]
@@ -477,7 +478,7 @@
 }
 
 #[tokio::test]
-async fn a_session_asked_to_write_still_reads_only() {
+async fn a_session_that_may_write_still_runs_scripts_read_only() {
     let dir = tempfile::tempdir().unwrap();
     tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
     let shop = dir.path().join("bookshop.duckdb");
@@ -490,7 +491,9 @@
     )
     .await
     .unwrap();
-    assert_eq!(connection.access(), tabletist_db::Access::ReadOnly);
+    // It saves edits to the files; a script reads, and a `.duckdb` file
+    // stays attached read-only.
+    assert_eq!(connection.access(), tabletist_db::Access::Writable);
     let error = refused(
         &connection,
         "INSERT INTO bookshop.book_covers SELECT * FROM bookshop.book_covers",
diff --git a/crates/tabletist-db/tests/files_attach.rs b/crates/tabletist-db/tests/files_attach.rs
--- a/crates/tabletist-db/tests/files_attach.rs
+++ b/crates/tabletist-db/tests/files_attach.rs
@@ -26,7 +26,14 @@
 /// A session over `dir`'s files with `attaching` attached.
 async fn open(dir: &tempfile::TempDir, attaching: &[Attaching]) -> Result<Connection, Error> {
     let spec = ConnectSpec::files(Source::Folder(dir.path().to_owned()));
-    Connection::connect_files(&spec, attaching, &HostKeys::default(), None).await
+    Connection::connect_files(
+        &spec,
+        attaching,
+        &HostKeys::default(),
+        tabletist_db::Access::ReadOnly,
+        None,
+    )
+    .await
 }
 
 /// The SQLite Bookshop in `dir`, attached as `shop`.
diff --git a/crates/tabletist-db/tests/ssh.rs b/crates/tabletist-db/tests/ssh.rs
--- a/crates/tabletist-db/tests/ssh.rs
+++ b/crates/tabletist-db/tests/ssh.rs
@@ -385,7 +385,7 @@
         },
     ];
     let spec = ConnectSpec::files(tabletist_db::files::Source::Folder(dir.path().to_owned()));
-    let connection = Connection::connect_files(&spec, &attaching, &keys, None)
+    let connection = Connection::connect_files(&spec, &attaching, &keys, Access::ReadOnly, None)
         .await
         .unwrap();
     let schemas = connection.list_schemas().await.unwrap();
diff --git a/src/app/history.rs b/src/app/history.rs
--- a/src/app/history.rs
+++ b/src/app/history.rs
@@ -950,6 +950,7 @@
             ..Structure::default()
         });
         let page = || RowPage {
+            positions: None,
             columns: Vec::new(),
             rows: Vec::new(),
             has_more: false,
diff --git a/src/shots.rs b/src/shots.rs
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -145,6 +145,7 @@
         })
         .collect();
     RowPage {
+        positions: None,
         columns: vec![
             column("id", "int8", ValueKind::Numeric),
             column("book_id", "int8", ValueKind::Numeric),
@@ -181,6 +182,7 @@
         identity: name == "id",
     };
     Structure {
+        unedited: None,
         columns: vec![
             column(
                 "id",
@@ -1989,6 +1991,7 @@
             })
             .collect();
         RowPage {
+            positions: None,
             columns: vec![
                 column("id", "int8", ValueKind::Numeric),
                 column("book_id", "int8", ValueKind::Numeric),
@@ -2133,6 +2136,7 @@
             ],
         ];
         RowPage {
+            positions: None,
             columns: vec![
                 column("id", "int8", ValueKind::Numeric),
                 column("title", "text", ValueKind::Text),
@@ -2170,6 +2174,7 @@
                 .to_vec(),
         );
         Structure {
+            unedited: None,
             columns: vec![
                 column("id", "bigint", false),
                 column("title", "text", false),
@@ -2203,6 +2208,7 @@
             identity: false,
         };
         Structure {
+            unedited: None,
             columns: vec![
                 column("id", "bigint", false),
                 column("book_id", "bigint", false),
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -3124,6 +3124,7 @@
             kind,
         };
         RowPage {
+            positions: None,
             columns: vec![
                 column("id", "int8", ValueKind::Numeric),
                 column("book_id", "int8", ValueKind::Numeric),
diff --git a/src/ui/delete_tests.rs b/src/ui/delete_tests.rs
--- a/src/ui/delete_tests.rs
+++ b/src/ui/delete_tests.rs
@@ -1008,6 +1008,7 @@
         detail: None,
         hint: None,
         named: Box::new(tabletist_db::Named {
+            unsaved: None,
             number,
             constraint: constraint.map(str::to_owned),
             columns: Vec::new(),
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -8822,6 +8822,7 @@
         let tab = with_page(&mut harness);
         harness.click("Structure");
         let structure = tabletist_db::Structure {
+            unedited: None,
             columns: vec![tabletist_db::ColumnInfo {
                 name: "email".into(),
                 type_name: "TEXT".into(),
@@ -13113,6 +13114,7 @@
             .chain((1..12).map(|index| format!("a_rather_long_column_{index}")))
             .collect();
         harness.answer_structure(Structure {
+            unedited: None,
             columns: names
                 .iter()
                 .map(|name| ColumnInfo {
@@ -13134,6 +13136,7 @@
             file: None,
         });
         harness.answer_rows(RowPage {
+            positions: None,
             columns: names
                 .iter()
                 .map(|name| ColumnMeta {
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -3575,6 +3575,7 @@
             .chain(std::iter::repeat_n(0, 96))
             .collect();
         harness.answer_rows(RowPage {
+            positions: None,
             columns: vec![
                 column("title", "text", ValueKind::Text),
                 column("subtitle", "text", ValueKind::Text),
diff --git a/src/ui/value_tags.rs b/src/ui/value_tags.rs
--- a/src/ui/value_tags.rs
+++ b/src/ui/value_tags.rs
@@ -245,6 +245,7 @@
     fn free_text_is_plain_however_its_values_repeat() {
         let structure = structure(&[("kind", None)]);
         let page = RowPage {
+            positions: None,
             columns: vec![meta("kind", ValueKind::Text)],
             rows: ["bug", "bug", "task", "task", "bug"]
                 .iter()
````

- [ ] **Step 4: Run them to see them pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist-db --test files_edit`

Expected: 9 pass.

- [ ] **Step 5: Run the full check**

Run the five commands of the full check above.

Expected: all pass; 5,747 passed, 0 failed. A Files connection's form still saves it read-only, so no session of the app writes yet.

- [ ] **Step 6: Commit**

```bash
git add crates src
git commit -m "Let a Files session that may write read a file's table into a copy and save it back to the file"
```

### Task 2: The form's box turns off, and the grid finds a file's rows by their place

**Files:**
- Create: `src/ui/files_edit_tests.rs`
- Modify: `crates/tabletist-core/src/{model,connections,edit}.rs`, `src/app.rs`, `src/shots.rs`, `src/app/editing.rs`, `src/ui/{cell_editor,sql_editor,sql_results,data_view,mod}.rs`

- [ ] **Step 1: Write the failing tests**

A saved Files connection with no `read_only` opens read-only in every environment, and writable with it off. In both looks, a Files connection that may write with `orders` open:
- a save sends each changed row's key as its place, and reads the page again once written;
- an Excel file's table takes no edit and no new row;
- a file whose page has no places isn't edited, and says why;
- its SQL editor doesn't write.

````diff
diff --git a/crates/tabletist-core/src/connections.rs b/crates/tabletist-core/src/connections.rs
--- a/crates/tabletist-core/src/connections.rs
+++ b/crates/tabletist-core/src/connections.rs
@@ -1315,4 +1315,24 @@
         connection.read_only = Some(false);
         assert_eq!(connection.access(), tabletist_db::Access::Writable);
     }
-}
+
+    #[test]
+    fn files_open_read_only_whatever_the_environment_until_the_box_is_off() {
+        let mut files = saved("Bookshop exports", "/bookshop.db");
+        files.spec = tabletist_db::ConnectSpec::files(tabletist_db::files::Source::Folder(
+            "/data/bookshop-exports".into(),
+        ));
+        files.read_only = None;
+        for environment in [Environment::Local, Environment::Dev, Environment::None] {
+            files.environment = environment;
+            assert_eq!(
+                files.access(),
+                tabletist_db::Access::ReadOnly,
+                "{environment:?}"
+            );
+            assert!(files.read_only());
+        }
+        files.read_only = Some(false);
+        assert_eq!(files.access(), tabletist_db::Access::Writable);
+    }
+}
diff --git a/src/ui/files_edit_tests.rs b/src/ui/files_edit_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/files_edit_tests.rs
@@ -0,0 +1,195 @@
+//! Editing a Files connection's table, in both looks: a save finds each
+//! row by its place in the file, and a table its connection does not edit
+//! is not. On the Bookshop's `orders.csv`.
+
+use tabletist_db::files::Format;
+use tabletist_db::{
+    ColumnInfo, ColumnMeta, Driver, FILE_POSITION, FileFacts, ObjectKind, ObjectRef, RowPage,
+    Structure, Unedited, Value, ValueKind,
+};
+
+use crate::backend::Command;
+use crate::model::{Action, ConnTabId, Pane, TabId};
+use crate::testing::Harness;
+use crate::theme::Look;
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
+
+/// `orders` as DuckDB reads `orders.csv`: three columns, no key.
+fn orders_structure(format: Format, unedited: Option<Unedited>) -> Structure {
+    let column = |name: &str, type_name: &str| ColumnInfo {
+        name: name.into(),
+        type_name: type_name.into(),
+        nullable: true,
+        ..ColumnInfo::default()
+    };
+    Structure {
+        columns: vec![
+            column("id", "BIGINT"),
+            column("email", "VARCHAR"),
+            column("total", "DOUBLE"),
+        ],
+        file: Some(FileFacts {
+            path: "/data/bookshop-exports/orders.csv".into(),
+            size: 1_600_000_000,
+            format,
+            options: String::new(),
+        }),
+        unedited,
+        ..Structure::default()
+    }
+}
+
+/// Three of its rows, each at its place in the file where `placed`.
+fn orders_page(placed: bool) -> RowPage {
+    let meta = |name: &str, type_name: &str, kind| ColumnMeta {
+        name: name.into(),
+        type_name: type_name.into(),
+        kind,
+    };
+    RowPage {
+        columns: vec![
+            meta("id", "BIGINT", ValueKind::Numeric),
+            meta("email", "VARCHAR", ValueKind::Text),
+            meta("total", "DOUBLE", ValueKind::Numeric),
+        ],
+        rows: (1..=3)
+            .map(|id| {
+                vec![
+                    Value::Int(id),
+                    Value::Text(format!("reader{id}@example.com").into()),
+                    Value::Float(id as f64 * 10.5),
+                ]
+            })
+            .collect(),
+        has_more: false,
+        ordered_by_key: placed,
+        elapsed: std::time::Duration::from_millis(3),
+        positions: placed.then(|| vec![0, 1, 2]),
+    }
+}
+
+/// A Files connection that may write, with `orders` open on `structure`
+/// and `page`.
+fn orders(look: Look, structure: Structure, page: RowPage) -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_fake_as(false);
+    harness.app.workspace_mut(tab).unwrap().driver = Driver::Files;
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: ObjectRef::new("main", "orders"),
+        kind: ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(structure);
+    harness.answer_rows(page);
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
+    harness.settle();
+    (harness, tab, id)
+}
+
+/// The last save that was sent.
+fn sent(harness: &Harness) -> &tabletist_db::ChangeSet {
+    let mut sent = harness.app.backend.sent.iter().rev();
+    sent.find_map(|command| match command {
+        Command::Write { changes, .. } => Some(changes),
+        _ => None,
+    })
+    .expect("a save was sent")
+}
+
+fn place(at: i64) -> Vec<(String, Value)> {
+    vec![(FILE_POSITION.to_owned(), Value::Int(at))]
+}
+
+#[test]
+fn a_save_finds_each_row_of_a_file_by_its_place() {
+    for look in looks() {
+        let (mut harness, tab, id) =
+            orders(look, orders_structure(Format::Csv, None), orders_page(true));
+        super::insert_audit_tests::set(&mut harness, tab, id, (1, 2), "21.50");
+        harness.app.apply(Action::WriteEdits { tab, id });
+        let changes = sent(&harness);
+        assert_eq!(changes.rows.len(), 1, "{}", look.name);
+        assert_eq!(changes.rows[0].key, place(1), "{}", look.name);
+        assert_eq!(changes.rows[0].set[0].column, "total");
+        // Written: the page is read again, every row at its new place.
+        let fetches = |harness: &Harness| {
+            harness
+                .app
+                .backend
+                .sent
+                .iter()
+                .filter(|command| matches!(command, Command::FetchRows { .. }))
+                .count()
+        };
+        let before = fetches(&harness);
+        harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
+            inserted: Vec::new(),
+            rows: vec![vec![
+                Value::Int(2),
+                Value::Text("reader2@example.com".into()),
+                Value::Float(21.5),
+            ]],
+            gone: Vec::new(),
+            elapsed: std::time::Duration::from_millis(14),
+        }));
+        harness.settle();
+        assert_eq!(fetches(&harness), before + 1, "{}", look.name);
+    }
+}
+
+#[test]
+fn an_excel_file_opens_read_only() {
+    for look in looks() {
+        let (mut harness, tab, id) = orders(
+            look,
+            orders_structure(Format::Excel, Some(Unedited::Excel)),
+            orders_page(false),
+        );
+        // No cell of it is edited, and no row added.
+        let workspace = harness.app.workspace(tab).unwrap();
+        let object = workspace.object_tab(id).unwrap();
+        let table = crate::edit::Table::of(workspace, object).unwrap();
+        let lock = Some(crate::edit::Lock::Unedited(Unedited::Excel));
+        assert_eq!(table.never(), lock, "{}", look.name);
+        assert_eq!(table.no_rows(), lock, "{}", look.name);
+        harness.settle();
+    }
+}
+
+#[test]
+fn a_file_whose_rows_have_no_place_is_not_edited() {
+    let (harness, tab, id) = orders(
+        Look::macos(),
+        orders_structure(Format::Csv, None),
+        orders_page(false),
+    );
+    let workspace = harness.app.workspace(tab).unwrap();
+    let object = workspace.object_tab(id).unwrap();
+    let table = crate::edit::Table::of(workspace, object).unwrap();
+    assert_eq!(table.never(), Some(crate::edit::Lock::Unmatched));
+    assert_eq!(
+        super::cell_editor::lock_text(
+            crate::edit::Lock::Unmatched,
+            "orders",
+            crate::i18n::Locale::English
+        ),
+        "orders opens read-only: Tabletist couldn't match its rows to the file's lines"
+    );
+}
+
+#[test]
+fn a_files_script_only_reads_and_says_where_files_are_edited() {
+    let (harness, tab, _) = orders(
+        Look::macos(),
+        orders_structure(Format::Csv, None),
+        orders_page(true),
+    );
+    let workspace = harness.app.workspace(tab).unwrap();
+    assert_eq!(workspace.sql_writes(), Err(crate::model::NoWrites::Files));
+}
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -77,6 +77,8 @@
 #[cfg(test)]
 mod export_ways_tests;
 pub mod file_list;
+#[cfg(test)]
+mod files_edit_tests;
 #[cfg(test)]
 mod files_tests;
 pub mod filter_bar;
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist-core --lib -- connections::tests::files_open_read_only`, then `~/.cargo/bin/cargo test -q -p tabletist --lib -- files_edit_tests`

Expected: the first fails, a Files connection of no `read_only` in a local environment opening writable. The second doesn't compile: `Lock::Unedited`, `Lock::Unmatched` and `NoWrites::Files` aren't there yet.

```text
failures:
---- connections::tests::files_open_read_only_whatever_the_environment_until_the_box_is_off stdout ----
thread 'connections::tests::files_open_read_only_whatever_the_environment_until_the_box_is_off' (2522716) panicked at crates/tabletist-core/src/connections.rs:1328:13:
  left: Writable
    connections::tests::files_open_read_only_whatever_the_environment_until_the_box_is_off
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1168 filtered out; finished in 0.00s
error: test failed, to rerun pass `-p tabletist-core --lib`
error[E0599]: no variant, associated function, or constant named `Unedited` found for enum `tabletist_core::edit::Lock` in the current scope
    |
error[E0599]: no variant, associated function, or constant named `Unmatched` found for enum `tabletist_core::edit::Lock` in the current scope
error[E0599]: no variant, associated function, or constant named `Files` found for enum `NoWrites` in the current scope
error: could not compile `tabletist` (lib test) due to 4 previous errors
```

- [ ] **Step 3: Let the box turn off, and find a file's rows by their place**

````diff
diff --git a/crates/tabletist-core/src/connections.rs b/crates/tabletist-core/src/connections.rs
--- a/crates/tabletist-core/src/connections.rs
+++ b/crates/tabletist-core/src/connections.rs
@@ -134,21 +134,42 @@
     /// environment's default. A user who never chose follows the
     /// environment when it changes.
     pub fn read_only(&self) -> bool {
-        self.read_only
-            .unwrap_or(self.environment.read_only_by_default())
+        read_only_of(self.spec.driver, self.read_only, self.environment)
     }
 
     /// The access its sessions open with.
     pub fn access(&self) -> tabletist_db::Access {
-        access(self.read_only, self.environment)
-    }
-}
-
-/// The access a session opens with: read-only as the box says, else as
-/// `environment` has it by default. An open tab asks with its own
-/// environment, which a relabelled saved entry does not change.
-pub fn access(read_only: Option<bool>, environment: Environment) -> tabletist_db::Access {
-    if read_only.unwrap_or(environment.read_only_by_default()) {
+        access(self.spec.driver, self.read_only, self.environment)
+    }
+}
+
+/// Whether a connection of `driver` is read-only: as the box says, else as
+/// `environment` has it by default. Files are read-only until the box is
+/// turned off, whatever the environment.
+fn read_only_of(
+    driver: tabletist_db::Driver,
+    read_only: Option<bool>,
+    environment: Environment,
+) -> bool {
+    use tabletist_db::Driver;
+    read_only.unwrap_or(match driver {
+        Driver::Files => true,
+        Driver::Sqlite | Driver::Postgres | Driver::MySql | Driver::BigQuery => {
+            environment.read_only_by_default()
+        }
+    })
+}
+
+/// The access a session of `driver` opens with: read-only as the box
+/// says, else by default: Files read-only, others as `environment` has it
+/// (see `SavedConnection::read_only`). An open tab asks with its
+/// own environment, which a relabelled saved entry does not change.
+pub fn access(
+    driver: tabletist_db::Driver,
+    read_only: Option<bool>,
+    environment: Environment,
+) -> tabletist_db::Access {
+    if read_only_of(driver, read_only, environment) {
         tabletist_db::Access::ReadOnly
     } else {
         tabletist_db::Access::Writable
diff --git a/crates/tabletist-core/src/edit.rs b/crates/tabletist-core/src/edit.rs
--- a/crates/tabletist-core/src/edit.rs
+++ b/crates/tabletist-core/src/edit.rs
@@ -27,6 +27,13 @@
 pub enum Lock {
     /// The connection opens read-only.
     ReadOnly,
+    /// A Files connection's table that its connection never edits, though
+    /// its session may write: an Excel file's, a `.duckdb` file's, an
+    /// attached connection's.
+    Unedited(tabletist_db::Unedited),
+    /// A Files connection's CSV whose rows could not be read one for one
+    /// as DuckDB read them: a save could not write them back.
+    Unmatched,
     /// A view, a materialized view.
     NotATable,
     /// The row key, the columns' types and what is generated are not known
@@ -74,6 +81,8 @@
         matches!(
             self,
             Self::ReadOnly
+                | Self::Unedited(_)
+                | Self::Unmatched
                 | Self::NotATable
                 | Self::StructureLoading
                 | Self::NoKey
@@ -163,12 +172,47 @@
 impl Table<'_> {
     /// The page's columns that make the row key, by their place. `None`
     /// when the table has no key, or the page does not hold all of it.
+    /// None for a file's page that says where each row is: a row is found
+    /// by its place ([`Table::found_by`]), which is no column of the page.
     pub fn key(&self) -> Option<Vec<usize>> {
+        if self.page.positions.is_some() {
+            return Some(Vec::new());
+        }
         self.structure?
             .row_key()?
             .iter()
             .map(|name| self.place(name))
             .collect()
+    }
+
+    /// What finds the page's row `row` for a save: the values of its key's
+    /// columns, or its place in its file. `None` where the page does not
+    /// hold them.
+    pub fn found_by(&self, row: usize, key: &[usize]) -> Option<Vec<(String, Value)>> {
+        if let Some(positions) = &self.page.positions {
+            let place = *positions.get(row)?;
+            return Some(vec![(
+                tabletist_db::FILE_POSITION.to_owned(),
+                Value::Int(place),
+            )]);
+        }
+        let values = self.page.rows.get(row)?;
+        key.iter()
+            .map(|&place| {
+                let name = self.page.columns.get(place)?.name.clone();
+                Some((name, values.get(place)?.clone()))
+            })
+            .collect()
+    }
+
+    /// Why a Files connection's table is not edited, where it is one: what
+    /// its connection says, or a file whose rows have no place.
+    fn file_lock(&self) -> Option<Lock> {
+        let structure = self.structure?;
+        if let Some(why) = structure.unedited {
+            return Some(Lock::Unedited(why));
+        }
+        (structure.file.is_some() && self.page.positions.is_none()).then_some(Lock::Unmatched)
     }
 
     /// The one column of the page called `name`. Two of that name are none:
@@ -333,6 +377,9 @@
         if self.structure.is_none() {
             return Some(Lock::StructureLoading);
         }
+        if let Some(lock) = self.file_lock() {
+            return Some(lock);
+        }
         let Some(key) = self.key() else {
             return Some(Lock::NoKey);
         };
@@ -354,6 +401,9 @@
         }
         if self.structure.is_none() {
             return Some(Lock::StructureLoading);
+        }
+        if let Some(lock) = self.file_lock() {
+            return Some(lock);
         }
         if self.saving {
             return Some(Lock::Saving);
@@ -3029,15 +3079,8 @@
     } else {
         Vec::new()
     };
-    // What finds the row whose values are `values`.
-    let key_of = |values: &Vec<Value>| -> Option<Vec<(String, Value)>> {
-        key.iter()
-            .map(|&place| {
-                let name = table.page.columns.get(place)?.name.clone();
-                Some((name, values.get(place)?.clone()))
-            })
-            .collect()
-    };
+    // What finds the page's row `row`.
+    let key_of = |row: usize| table.found_by(row, &key);
     let mut rows: Vec<RowChange> = Vec::new();
     // The map is ordered by row, then column, so a row's cells are together.
     for (&(row, col), pending) in changed {
@@ -3046,7 +3089,7 @@
         if sent.rows.last() != Some(&row) {
             sent.rows.push(row);
             rows.push(RowChange {
-                key: key_of(values)?,
+                key: key_of(row)?,
                 set: Vec::new(),
             });
         }
@@ -3061,9 +3104,7 @@
     // page's order.
     let mut deletes = Vec::new();
     for row in delete_order(object, table) {
-        deletes.push(RowDelete {
-            key: key_of(table.page.rows.get(row)?)?,
-        });
+        deletes.push(RowDelete { key: key_of(row)? });
         sent.deletes.push(row);
     }
     (!rows.is_empty() || !inserts.is_empty() || !deletes.is_empty()).then(|| {
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -2741,11 +2741,11 @@
 
     /// Whether the connection is saved read-only: as set, else the
     /// environment's default, which for production is
-    /// [`Self::production_read_only`].
+    /// [`Self::production_read_only`]. Files open read-only until the box
+    /// is turned off, whatever the environment.
     pub fn read_only(&self) -> bool {
-        // Editing files is not built: a Files connection opens read-only.
         match self.driver {
-            Driver::Files => return true,
+            Driver::Files => return self.read_only.unwrap_or(true),
             Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery => {}
         }
         self.read_only.unwrap_or_else(|| {
@@ -2759,9 +2759,9 @@
     /// settings said when it was made; else not set, and the environment's
     /// rule applies.
     fn saved_read_only(&self) -> Option<bool> {
-        // A Files connection is saved read-only, as its box shows.
+        // A Files connection is saved as its box shows.
         match self.driver {
-            Driver::Files => return Some(true),
+            Driver::Files => return Some(self.read_only()),
             Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery => {}
         }
         let new_production = self.editing.is_none() && self.environment().read_only_by_default();
@@ -5827,7 +5827,10 @@
     pub fn of_new_tab(writes: Result<(), NoWrites>, environment: crate::env::Environment) -> Self {
         match writes {
             Ok(()) if !environment.read_only_by_default() => Self::ReadWrite,
-            Ok(()) | Err(NoWrites::ReadOnlyConnection | NoWrites::Unconfirmed) => Self::ReadOnly,
+            Ok(())
+            | Err(NoWrites::Files | NoWrites::ReadOnlyConnection | NoWrites::Unconfirmed) => {
+                Self::ReadOnly
+            }
         }
     }
 }
@@ -5835,6 +5838,9 @@
 /// Why no SQL editor of a workspace runs read-write, whatever its mode.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum NoWrites {
+    /// A Files connection: its scripts read, and its files are edited in
+    /// their tables and saved.
+    Files,
     /// The session was opened read-only.
     ReadOnlyConnection,
     /// The connection's environment asks before a write, and the question
@@ -6704,6 +6710,13 @@
     /// Whether a SQL editor of this workspace can run read-write, or why
     /// none can.
     pub fn sql_writes(&self) -> Result<(), NoWrites> {
+        match self.driver {
+            tabletist_db::Driver::Files => return Err(NoWrites::Files),
+            tabletist_db::Driver::Sqlite
+            | tabletist_db::Driver::Postgres
+            | tabletist_db::Driver::MySql
+            | tabletist_db::Driver::BigQuery => {}
+        }
         if self.access != tabletist_db::Access::Writable {
             return Err(NoWrites::ReadOnlyConnection);
         }
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -4319,7 +4319,8 @@
             return;
         };
         if let Some(read_only) = boxed {
-            workspace.access = crate::connections::access(read_only, workspace.environment);
+            workspace.access =
+                crate::connections::access(workspace.driver, read_only, workspace.environment);
         }
         if forced {
             workspace.access = tabletist_db::Access::ReadOnly;
diff --git a/src/app/editing.rs b/src/app/editing.rs
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -2378,6 +2378,9 @@
                 let fits = object.rows.value.as_ref().is_some_and(|page| {
                     let wide = |row: &Vec<tabletist_db::Value>| row.len() == page.columns.len();
                     !reaches_its_own
+                        // A file's rows each move to their place in the
+                        // file as written: the page is read again.
+                        && page.positions.is_none()
                         && rows.len() == saving.rows.len()
                         && rows.iter().all(wide)
                         && saving.rows.iter().all(|&at| at < page.rows.len())
diff --git a/src/shots.rs b/src/shots.rs
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -145,7 +145,6 @@
         })
         .collect();
     RowPage {
-        positions: None,
         columns: vec![
             column("id", "int8", ValueKind::Numeric),
             column("book_id", "int8", ValueKind::Numeric),
@@ -158,6 +157,7 @@
         has_more: false,
         ordered_by_key: true,
         elapsed: Duration::from_millis(3),
+        positions: None,
     }
 }
 
@@ -339,7 +339,7 @@
     workspace.name = "Bookshop".into();
     workspace.environment = Environment::Production;
     // Its box never set: production's default.
-    workspace.access = crate::connections::access(None, workspace.environment);
+    workspace.access = crate::connections::access(workspace.driver, None, workspace.environment);
     workspace.spec = spec;
     workspace.driver = Driver::Postgres;
     harness.app.apply(Action::ActivateConnTab(own));
@@ -933,7 +933,8 @@
     let tab = production_beside(harness, own);
     harness.app.apply(Action::ActivateConnTab(tab));
     let workspace = harness.app.workspace_mut(tab).unwrap();
-    workspace.access = crate::connections::access(Some(false), workspace.environment);
+    workspace.access =
+        crate::connections::access(workspace.driver, Some(false), workspace.environment);
     workspace.databases.value = Some(vec!["bookshop_production".into(), "postgres".into()]);
     tables(harness, tab);
     harness.answer_structure(structure());
@@ -1991,7 +1992,6 @@
             })
             .collect();
         RowPage {
-            positions: None,
             columns: vec![
                 column("id", "int8", ValueKind::Numeric),
                 column("book_id", "int8", ValueKind::Numeric),
@@ -2004,6 +2004,7 @@
             has_more: false,
             ordered_by_key: true,
             elapsed: Duration::from_millis(2),
+            positions: None,
         }
     }
 
@@ -2136,7 +2137,6 @@
             ],
         ];
         RowPage {
-            positions: None,
             columns: vec![
                 column("id", "int8", ValueKind::Numeric),
                 column("title", "text", ValueKind::Text),
@@ -2153,6 +2153,7 @@
             has_more: false,
             ordered_by_key: true,
             elapsed: Duration::from_millis(2),
+            positions: None,
         }
     }
 
diff --git a/src/ui/cell_editor.rs b/src/ui/cell_editor.rs
--- a/src/ui/cell_editor.rs
+++ b/src/ui/cell_editor.rs
@@ -1529,6 +1529,20 @@
     let say = |text: Msgid| text.say(locale).into_owned();
     match lock {
         Lock::ReadOnly => say(msgid("This connection opens read-only")),
+        Lock::Unedited(tabletist_db::Unedited::Excel) => say(msgid("Excel files open read-only")),
+        Lock::Unedited(tabletist_db::Unedited::DuckDb) => {
+            say(msgid(".duckdb files open read-only"))
+        }
+        Lock::Unedited(tabletist_db::Unedited::Attached) => {
+            say(msgid("Attached connections open read-only"))
+        }
+        Lock::Unmatched => fill(
+            &gettext(
+                locale,
+                "{table} opens read-only: Tabletist couldn't match its rows to the file's lines",
+            ),
+            &[("{table}", table)],
+        ),
         Lock::NotATable => say(msgid("Views cannot be edited")),
         Lock::StructureLoading => say(msgid("The table's structure is still loading")),
         Lock::NoKey => fill(
@@ -1580,7 +1594,9 @@
             ),
             &[("{table}", table)],
         ),
-        Lock::StructureLoading
+        Lock::Unedited(_)
+        | Lock::Unmatched
+        | Lock::StructureLoading
         | Lock::KeyType
         | Lock::Saving
         | Lock::Refreshing
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -1643,7 +1643,9 @@
     // A deleted row's minus stands where the row's key begins.
     let delete_marker = crate::edit::Table::of(workspace, object)
         .and_then(|table| table.key())
-        .and_then(|key| key.into_iter().min());
+        // A file's row is found by its place, not a column: the minus
+        // stands at its start.
+        .map(|key| key.into_iter().min().unwrap_or(0));
     let (unset, lacking) = new_row_columns(workspace, object);
     let target = object.edits.editor.as_ref().and_then(|editor| {
         let how = (hold, &editor_keys, calendar.as_ref());
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -563,6 +563,7 @@
 /// says where it cannot be picked.
 fn no_writes(no: NoWrites) -> Msgid {
     match no {
+        NoWrites::Files => msgid("A file is edited in its table, then saved."),
         NoWrites::ReadOnlyConnection => msgid("This connection opens read-only."),
         NoWrites::Unconfirmed => UNCONFIRMED,
     }
@@ -575,7 +576,7 @@
     let does = say(mode_does(bar.mode));
     match bar.writes {
         Ok(()) => does.into_owned(),
-        Err(no @ NoWrites::ReadOnlyConnection) => format!(
+        Err(no @ (NoWrites::Files | NoWrites::ReadOnlyConnection)) => format!(
             "{}. {}",
             say(msgid(
                 "Every query runs in a read-only transaction that is rolled back"
@@ -657,7 +658,9 @@
     // A control that is off, as the design's: all of it at 45 %.
     let fade = |color: Color32| match writes {
         Ok(()) => color,
-        Err(NoWrites::ReadOnlyConnection | NoWrites::Unconfirmed) => color.gamma_multiply(0.45),
+        Err(NoWrites::Files | NoWrites::ReadOnlyConnection | NoWrites::Unconfirmed) => {
+            color.gamma_multiply(0.45)
+        }
     };
     let painter = ui.painter().clone();
     let (pad, gap) = if look.terminal {
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -1538,6 +1538,8 @@
 /// its card says and offers follows from it.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 enum Why {
+    /// A Files connection: its scripts read (see `NoWrites::Files`).
+    Files,
     /// The connection opens read-only. `unconfirmed`: and turning its box
     /// off would not let a tab write here either (see
     /// `NoWrites::Unconfirmed`).
@@ -1617,6 +1619,7 @@
         kind(driver.dialect(), &statement.text) == StatementKind::Write
     };
     let why = match workspace.sql_writes() {
+        Err(NoWrites::Files) => Why::Files,
         Err(NoWrites::ReadOnlyConnection) => Why::Connection {
             unconfirmed: workspace.environment.confirms_writes(),
         },
@@ -1647,7 +1650,9 @@
             Why::Tab { unconfirmed: false } => Some(Offer::AllowWrites),
             // Neither the connection's box nor the tab's switch is a way
             // on here yet.
-            Why::Connection { unconfirmed: true } | Why::Tab { unconfirmed: true } => None,
+            Why::Files | Why::Connection { unconfirmed: true } | Why::Tab { unconfirmed: true } => {
+                None
+            }
             Why::TakenForRead | Why::SentBefore => self.again.then_some(Offer::RunAgain),
         }
     }
@@ -1696,6 +1701,18 @@
             format!("{why} {nothing}")
         };
         match self.why {
+            Why::Files => (
+                words.say(msgid("Files are edited in their tables")),
+                refused(
+                    msgid(
+                        "A Files connection's scripts only read, so {database} refused {statement}.",
+                    ),
+                    "",
+                ),
+                Some(words.say(msgid(
+                    "Change the rows in the file's table, then save them.",
+                ))),
+            ),
             Why::Connection { unconfirmed } => (
                 words.say(msgid("This connection opens read-only")),
                 refused(
````

- [ ] **Step 4: Run them to see them pass**

Run: the two commands of Step 2.

Expected: 1 pass, then 4 pass.

- [ ] **Step 5: Run the full check, catalogs first**

The new words of the locks and the SQL editor's gate go into the catalogs: run `sh assets/i18n/update.sh`, then fill the Russian and Brazilian Portuguese of those messages from the table in Task 3.

Run the five commands of the full check above.

Expected: all pass; 5,752 passed, 0 failed.

- [ ] **Step 6: Commit**

```bash
git add crates src assets/i18n
git commit -m "Let a Files connection's read-only box be turned off, and find a file's rows by their place"
```

### Task 3: What the form, the list, the header and a refused save say

**Files:**
- Modify: `src/ui/connect_dialog/{mod,sheet,terminal,join}.rs`, `src/ui/picker.rs`, `src/ui/data_view.rs`, `src/ui/pending_bar.rs`, `src/ui/files_tests.rs`, `src/ui/files_edit_tests.rs`, `assets/i18n/*`, `docs/_guide/editing-data.md`, `docs/_reference/connections.md`, `docs/_reference/settings-and-files.md`

- [ ] **Step 1: Write the failing tests**

In both looks:
- the Files form's box can be turned off. On, its note says a save rewrites the whole file. Off, it names the largest file a save would write again: never an Excel file or a skipped one, and none before the Source is read;
- a save refused because the file changed on disk says so with the count of changes pending, and one refused because the folder can't be written to says that;
- an Excel file's table says why in its header;
- the list row says "Editable".

````diff
diff --git a/src/ui/files_edit_tests.rs b/src/ui/files_edit_tests.rs
--- a/src/ui/files_edit_tests.rs
+++ b/src/ui/files_edit_tests.rs
@@ -1,11 +1,12 @@
 //! Editing a Files connection's table, in both looks: a save finds each
-//! row by its place in the file, and a table its connection does not edit
-//! is not. On the Bookshop's `orders.csv`.
+//! row by its place in the file, a save the file keeps from writing says
+//! why and keeps every change, and a table its connection does not edit
+//! says why in its header. On the Bookshop's `orders.csv`.
 
 use tabletist_db::files::Format;
 use tabletist_db::{
     ColumnInfo, ColumnMeta, Driver, FILE_POSITION, FileFacts, ObjectKind, ObjectRef, RowPage,
-    Structure, Unedited, Value, ValueKind,
+    Structure, Unedited, Unsaved, Value, ValueKind,
 };
 
 use crate::backend::Command;
@@ -144,12 +145,74 @@
 }
 
 #[test]
-fn an_excel_file_opens_read_only() {
+fn a_file_changed_on_disk_keeps_every_change_and_says_how_many() {
+    for look in looks() {
+        let (mut harness, tab, id) =
+            orders(look, orders_structure(Format::Csv, None), orders_page(true));
+        super::insert_audit_tests::set(&mut harness, tab, id, (0, 2), "11.00");
+        super::insert_audit_tests::set(&mut harness, tab, id, (1, 2), "22.00");
+        super::insert_audit_tests::set(&mut harness, tab, id, (2, 2), "33.00");
+        harness.app.apply(Action::WriteEdits { tab, id });
+        let mut error =
+            tabletist_db::Error::query("orders.csv changed on disk since it was opened");
+        if let tabletist_db::Error::Query { named, .. } = &mut error {
+            named.unsaved = Some(Unsaved::Changed("orders.csv".into()));
+        }
+        harness.answer_written(Err(error));
+        harness.settle();
+        let edits = super::insert_audit_tests::edits(&harness, tab, id);
+        assert_eq!(edits.counts().changes, 3, "{}", look.name);
+        let said = "orders.csv changed on disk since you opened it. Reload to see the new \
+                    version; your 3 changes stay pending.";
+        let painted = harness
+            .painted
+            .iter()
+            .any(|(text, _)| text.to_lowercase().contains(&said.to_lowercase()));
+        assert!(
+            harness.has(said) || painted,
+            "{}: {:?}",
+            look.name,
+            harness.painted
+        );
+    }
+}
+
+#[test]
+fn a_folder_that_cannot_be_written_to_keeps_the_change() {
+    let (mut harness, tab, id) = orders(
+        Look::macos(),
+        orders_structure(Format::Csv, None),
+        orders_page(true),
+    );
+    super::insert_audit_tests::set(&mut harness, tab, id, (0, 2), "11.00");
+    harness.app.apply(Action::WriteEdits { tab, id });
+    let mut error = tabletist_db::Error::query("orders.csv could not be written");
+    if let tabletist_db::Error::Query { named, .. } = &mut error {
+        named.unsaved = Some(Unsaved::Unwritable("orders.csv".into()));
+    }
+    harness.answer_written(Err(error));
+    assert!(harness.has(
+        "orders.csv couldn't be saved: its folder can't be written to. Your 1 change stays pending."
+    ));
+}
+
+#[test]
+fn an_excel_file_says_in_its_header_that_it_opens_read_only() {
     for look in looks() {
         let (mut harness, tab, id) = orders(
             look,
             orders_structure(Format::Excel, Some(Unedited::Excel)),
             orders_page(false),
+        );
+        let said = look.label("Excel files open read-only");
+        assert!(
+            harness
+                .painted
+                .iter()
+                .any(|(text, _)| text.starts_with(&said)),
+            "{}: {:?}",
+            look.name,
+            harness.painted
         );
         // No cell of it is edited, and no row added.
         let workspace = harness.app.workspace(tab).unwrap();
diff --git a/src/ui/files_tests.rs b/src/ui/files_tests.rs
--- a/src/ui/files_tests.rs
+++ b/src/ui/files_tests.rs
@@ -505,24 +505,90 @@
 }
 
 #[test]
-fn files_open_read_only_and_the_box_cannot_be_turned_off() {
+fn files_open_read_only_until_the_box_is_turned_off() {
     for look in looks() {
         let mut harness = new_files(look);
         let name = "Open read-only";
         let tree = harness.settle();
         let check = node(&tree, name, Role::CheckBox).expect("the read-only box");
         let found = &tree.nodes.iter().find(|(id, _)| *id == check).unwrap().1;
-        assert!(found.is_disabled(), "{}", look.name);
+        assert!(!found.is_disabled(), "{}", look.name);
+        assert!(form(&harness).read_only(), "{}", look.name);
+        // On, it says that editing rewrites a whole file.
+        match look.terminal {
+            true => {
+                assert!(harness.painted_rect("[x] files open read-only").is_some());
+                assert!(says(
+                    &mut harness,
+                    "· turn off to edit; saving rewrites the whole file"
+                ));
+            }
+            false => assert!(says(
+                &mut harness,
+                "Files open read-only. Turn off to edit: changes are kept until you save, and \
+                 saving rewrites the whole file."
+            )),
+        }
+        pick_folder(&mut harness, "/data/bookshop-exports");
+        source_read(&mut harness, Ok(exports("/data/bookshop-exports")));
+        assert_eq!(form(&harness).to_saved().unwrap().read_only, Some(true));
         harness.click(name);
-        assert_eq!(form(&harness).read_only, None, "{}", look.name);
-        pick_folder(&mut harness, "/data/bookshop-exports");
-        assert_eq!(form(&harness).to_saved().unwrap().read_only, Some(true));
-        // The terminal look paints its promise beside the box.
+        harness.settle();
+        assert_eq!(form(&harness).read_only, Some(false), "{}", look.name);
+        assert_eq!(form(&harness).to_saved().unwrap().read_only, Some(false));
+        // Off, it names the largest file a save would write again.
         match look.terminal {
-            true => assert!(harness.painted_rect("[x] files open read-only").is_some()),
-            false => assert!(harness.has("Files open read-only.")),
+            true => {
+                assert!(harness.painted_rect("[ ] editing on").is_some());
+                assert!(says(
+                    &mut harness,
+                    "· changes wait for :w, which rewrites the whole file (orders.csv 1.6 gb)"
+                ));
+            }
+            false => assert!(says(
+                &mut harness,
+                "Editing is on. Changes are kept until you save; saving rewrites the whole \
+                 file, so orders.csv (1.6 GB) is written again in full."
+            )),
         }
     }
+}
+
+#[test]
+fn a_files_note_names_the_largest_file_a_save_writes_again() {
+    let mut harness = new_files(Look::macos());
+    form_mut(&mut harness).read_only = Some(false);
+    harness.settle();
+    // Before the Source is read, no file.
+    assert!(says(
+        &mut harness,
+        "Editing is on. Changes are kept until you save; saving rewrites the whole file."
+    ));
+    // An Excel file is never written, and a skipped one is not read.
+    let mut scan = exports("/data/bookshop-exports");
+    scan.files.insert(
+        0,
+        Found {
+            path: PathBuf::from("/data/bookshop-exports/stock.xlsx"),
+            format: Format::Excel,
+            size: 9_000_000_000,
+            name: "stock".into(),
+        },
+    );
+    form_mut(&mut harness).files_found = FilesFound::Read(scan);
+    form_mut(&mut harness)
+        .files_csv
+        .overrides
+        .push(tabletist_db::files::Override {
+            file: "orders.csv".into(),
+            change: tabletist_db::files::Change::Skip,
+        });
+    harness.settle();
+    assert!(says(
+        &mut harness,
+        "Editing is on. Changes are kept until you save; saving rewrites the whole file, so \
+         events.ndjson (450 MB) is written again in full."
+    ));
 }
 
 #[test]
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -1871,6 +1871,14 @@
                 .map(|(writes, joins)| (writes.english(), joins.say(crate::i18n::Locale::English))),
             Some(("Read-only", "No joins".to_owned()))
         );
+        // With its box off, its files are edited.
+        let mut editable = folder.clone();
+        editable.read_only = Some(false);
+        let text = file_row_text(&editable, Some(&facts), &none).unwrap();
+        assert_eq!(
+            text.right.map(|(writes, _)| writes.english()),
+            Some("Editable")
+        );
         assert_eq!(
             terminal_files_line(&folder, Some(&facts), &none, crate::i18n::Locale::English)
                 .as_deref(),
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib -- files_tests files_edit_tests picker::tests`

Expected: they fail. The box is still disabled, the notes aren't there, a refused save says DuckDB's words, the header doesn't say why, and the row says "Read-only".

```text
failures:
---- ui::picker::tests::a_files_rows_say_how_many_files_and_what_they_become stdout ----
thread 'ui::picker::tests::a_files_rows_say_how_many_files_and_what_they_become' (2987542) panicked at src/ui/picker.rs:1878:9:
  left: Some("Read-only")
---- ui::files_edit_tests::an_excel_file_says_in_its_header_that_it_opens_read_only stdout ----
thread 'ui::files_edit_tests::an_excel_file_says_in_its_header_that_it_opens_read_only' (2987497) panicked at src/ui/files_edit_tests.rs:208:9:
---- ui::files_tests::files_open_read_only_until_the_box_is_turned_off stdout ----
thread 'ui::files_tests::files_open_read_only_until_the_box_is_turned_off' (2987515) panicked at src/ui/files_tests.rs:515:9:
---- ui::files_edit_tests::a_folder_that_cannot_be_written_to_keeps_the_change stdout ----
thread 'ui::files_edit_tests::a_folder_that_cannot_be_written_to_keeps_the_change' (2987495) panicked at src/ui/files_edit_tests.rs:194:5:
---- ui::files_tests::a_files_note_names_the_largest_file_a_save_writes_again stdout ----
thread 'ui::files_tests::a_files_note_names_the_largest_file_a_save_writes_again' (2987499) panicked at src/ui/files_tests.rs:563:5:
---- ui::files_edit_tests::a_file_changed_on_disk_keeps_every_change_and_says_how_many stdout ----
thread 'ui::files_edit_tests::a_file_changed_on_disk_keeps_every_change_and_says_how_many' (2987490) panicked at src/ui/files_edit_tests.rs:171:9:
    ui::files_edit_tests::a_file_changed_on_disk_keeps_every_change_and_says_how_many
    ui::files_edit_tests::a_folder_that_cannot_be_written_to_keeps_the_change
    ui::files_edit_tests::an_excel_file_says_in_its_header_that_it_opens_read_only
    ui::files_tests::a_files_note_names_the_largest_file_a_save_writes_again
    ui::files_tests::files_open_read_only_until_the_box_is_turned_off
    ui::picker::tests::a_files_rows_say_how_many_files_and_what_they_become
test result: FAILED. 27 passed; 6 failed; 0 ignored; 0 measured; 3143 filtered out; finished in 0.11s
error: test failed, to rerun pass `-p tabletist --lib`
```

- [ ] **Step 3: Say what a save does, and why one didn't**

````diff
diff --git a/docs/_guide/editing-data.md b/docs/_guide/editing-data.md
--- a/docs/_guide/editing-data.md
+++ b/docs/_guide/editing-data.md
@@ -505,11 +505,50 @@
 If someone deleted the row, there is nothing to reload or overwrite:
 **Discard my changes** drops your change, and Esc leaves it pending.
 
+## Edit files
+
+A Files connection opens read-only until you turn **Open read-only** off
+in its form. Then a file's table is edited like any other: change values,
+add rows and delete rows, then save. The first time a file's table opens,
+Tabletist reads the file into memory, so a large file takes as long to
+open as it takes to read, and each row keeps its place in the file.
+That place is what a save finds the row by: a file has no key.
+
+Saving writes the whole file again, to a hidden file in its folder that
+then takes the file's place:
+
+- A CSV or a TSV keeps every row you did not change as it was written,
+  byte for byte. A changed or new row is written with the file's
+  delimiter and quote, each value as you typed it. A date is written the
+  way the file writes its dates. A CSV whose rows Tabletist cannot match
+  one for one to the lines DuckDB read opens read-only, and its header
+  says why.
+- Parquet keeps its compression. JSON stays one array, and NDJSON one
+  object per line. DuckDB writes these from the table, so a value can be
+  written another way than it was: `12.50` as `12.5`.
+- Excel files always open read-only, and so do `.duckdb` files for now.
+  Their table's header says so.
+- An attached connection's tables open read-only here, even when the
+  files are editable.
+
+Before it writes, a save checks that the file is as it was when its
+table was read: the same size and the same time it was changed. If
+another program changed it since, nothing is written: "orders.csv
+changed on disk since you opened it. Reload to see the new version;
+your 3 changes stay pending." If the folder can't be written to, nothing
+is written either, and your changes stay pending.
+
+The SQL editor reads a Files connection, whether it opens read-only or
+not: a statement that would write is refused, and its card says to edit
+the file's table instead.
+
 ## What cannot be edited
 
 A cell that cannot be edited says why when you try. The common reasons:
 
 - The connection opens read-only.
+- The file is an Excel or a `.duckdb` file, or its table is an attached
+  connection's (see [Edit files](#edit-files)).
 - The object is a view.
 - The table has no primary key or unique index, so a row cannot be
   targeted safely.
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -142,9 +142,10 @@
   other schemas are reached in SQL, as
   `bookshop_development.billing.invoices`. The schema menu lists the
   attached ones after the files' own. In the connections list the row
-  says what it joins: "Joins Bookshop · dev", with "+1", "+2" for the others
+  says whether its files are "Read-only" or "Editable", and what it
+  joins: "Joins Bookshop · dev", with "+1", "+2" for the others
   (`joins bookshop_development` in the Omarchy look). An attached
-  connection is always read-only. Its password comes from the keyring as
+  connection is always read-only, even when the files are editable. Its password comes from the keyring as
   the Files connection opens, and its SSH tunnel, if it has one, is
   opened then too; a connection whose password is asked for on every
   connect, or a MySQL one that names no database, cannot be attached. If
@@ -160,10 +161,18 @@
   PostgreSQL, MySQL or SQLite, DuckDB downloads it, and Test says so
   while it does ("Downloading the PostgreSQL reader for DuckDB…"). Once
   downloaded, it is used offline.
-- A new Files connection is **local**, and lets agents read. Files open
-  read-only: nothing writes to them, and a statement that would read
-  another file on the computer, write a file or install anything is
-  refused. Agents can query the files, never change them, and cannot
+- A new Files connection is **local**, opens read-only, and lets agents
+  read. A statement that would read another file on the computer, write
+  a file or install anything is refused, and the SQL editor only reads.
+  With **Open read-only** off, a file's table is edited in the grid and
+  saved: see [Edit files]({% link _guide/editing-data.md %}#edit-files).
+  The box's note says that saving rewrites the whole file, and names
+  the largest file a save would write again: "Editing is on. Changes
+  are kept until you save; saving rewrites the whole file, so orders.csv
+  (1.6 GB) is written again in full." The Omarchy look says it on one
+  line in the warning colour: `[ ] editing on · changes wait for :w,
+  which rewrites the whole file (orders.csv 1.6 gb)`. Agents can query
+  the files, never change them, and cannot
   hide tables or mask columns on a Files connection. Its **Agents**
   section is closed, and says the access on one line. Agents reach an
   attached connection only where its own **Agents** setting is **Read**
@@ -464,7 +473,8 @@
 the box off too: see [BigQuery](#bigquery).
 
 The box is ticked by default for production, and not for the other
-environments. Turn it off to edit. A change applies from the next time
+environments. A Files connection's is ticked whatever its environment.
+Turn it off to edit. A change applies from the next time
 you connect. Where a new production connection's box starts is
 **New production connections open read-only** in
 [Settings]({% link _reference/settings-and-files.md %}#the-settings-window).
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -542,7 +542,7 @@
 | `name` | Text | Required |
 | `group` | Text: the group the picker lists the connection under. A file that holds one says `version = 9` | The picker's own: a name the connection shares with another that has no group (or that name as its group), else Local or Remote |
 | `environment` | `local`, `dev`, `staging`, `production`, `none` | `none` |
-| `read_only` | `true`, `false` | As the environment has it: read-only on production |
+| `read_only` | `true`, `false` | As the environment has it: read-only on production. A Files connection: read-only |
 | `password` | `none`, `keyring`, `ask` | `none` |
 | `ssh_secret` | `none`, `keyring`, `ask` | `none` |
 | `driver` | `postgres`, `mysql`, `sqlite`, `files` | Required |
diff --git a/src/ui/connect_dialog/join.rs b/src/ui/connect_dialog/join.rs
--- a/src/ui/connect_dialog/join.rs
+++ b/src/ui/connect_dialog/join.rs
@@ -20,8 +20,11 @@
     msgid("Attach a saved connection to join a file to its live tables in one query. Optional.");
 
 /// What the open section says under the list: an attached connection
-/// keeps its own safety.
+/// keeps its own safety, read-only as the files are, or though they are
+/// not.
 const SAFETY: Msgid = msgid("Attached connections open read-only here, as the files do.");
+const SAFETY_EDITING: Msgid =
+    msgid("Attached connections open read-only here, even when the files are editable.");
 
 /// The closed row's summary: none attached, else their names.
 pub(super) fn summary(form: &ConnectionForm, locale: Locale) -> String {
@@ -194,7 +197,11 @@
             });
     }
     ui.add_space(8.0);
-    let safety = look.label(&skin.say(SAFETY));
+    let safety = match form.read_only() {
+        true => SAFETY,
+        false => SAFETY_EDITING,
+    };
+    let safety = look.label(&skin.say(safety));
     widgets::label(ui, small, &safety, palette.dim, look);
 }
 
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -25,7 +25,8 @@
 use crate::i18n::{Locale, Msgid, fill, gettext, msgid, ngettext};
 use crate::keymap::{Command, Keymap, Scope};
 use crate::model::{
-    Action, ConnectionForm, FilesKind, LoadedTables, SourceNames, SshAuthKind, SshHints, TestState,
+    Action, ConnectionForm, FilesFound, FilesKind, LoadedTables, SourceNames, SshAuthKind,
+    SshHints, TestState,
 };
 use crate::style::Icon;
 use crate::theme::{self, Look, Palette};
@@ -65,14 +66,50 @@
     }
 }
 
-/// Whether the read-only box is on and cannot be turned off: a Files
-/// connection's, until editing files is built.
-fn read_only_fixed(form: &ConnectionForm) -> bool {
-    is_files(form)
-}
-
-/// What a Files connection's read-only box says.
-const FILES_READ_ONLY_NOTE: Msgid = msgid("Files open read-only.");
+/// What a Files connection's read-only box says while it is on.
+const FILES_READ_ONLY_NOTE: Msgid = msgid(
+    "Files open read-only. Turn off to edit: changes are kept until you save, and saving \
+     rewrites the whole file.",
+);
+
+/// The largest of the form's files a save writes again whole, with its
+/// size as the found line says it: what the note of a Files connection
+/// that edits names. `None` until the Source is read, or where it has none.
+fn largest_rewritten(form: &ConnectionForm) -> Option<(String, String)> {
+    let FilesFound::Read(scan) = &form.files_found else {
+        return None;
+    };
+    let reading = &form.files_csv;
+    let file = scan
+        .files
+        .iter()
+        .filter(|file| file.format.rewritten() && !reading.skips(&file.path))
+        .max_by_key(|file| file.size)?;
+    Some((
+        tabletist_db::files::file_name(&file.path),
+        tabletist_db::files::size_text(file.size),
+    ))
+}
+
+/// What a Files connection's read-only box says while it is off: that a
+/// save writes a whole file, and the largest it would.
+fn files_editing_note(form: &ConnectionForm, locale: Locale) -> String {
+    match largest_rewritten(form) {
+        Some((file, size)) => crate::i18n::fill(
+            &gettext(
+                locale,
+                "Editing is on. Changes are kept until you save; saving rewrites the whole file, \
+                 so {file} ({size}) is written again in full.",
+            ),
+            &[("{file}", &file), ("{size}", &size)],
+        ),
+        None => gettext(
+            locale,
+            "Editing is on. Changes are kept until you save; saving rewrites the whole file.",
+        )
+        .into_owned(),
+    }
+}
 
 /// Why a Files connection's URL tab is off, as its tooltip says it.
 const NO_URL: Msgid =
diff --git a/src/ui/connect_dialog/sheet.rs b/src/ui/connect_dialog/sheet.rs
--- a/src/ui/connect_dialog/sheet.rs
+++ b/src/ui/connect_dialog/sheet.rs
@@ -690,11 +690,11 @@
 fn safety(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
     let Skin { look, palette, .. } = *skin;
     let title = skin.say(msgid("Open read-only"));
-    // Editing files is not built: their box is on, and stays so.
-    let fixed = super::read_only_fixed(form);
-    let note = match fixed {
-        true => super::FILES_READ_ONLY_NOTE,
-        false => READ_ONLY_NOTE,
+    // A Files connection's note says what a save does to its files.
+    let note = match (super::is_files(form), form.read_only()) {
+        (true, true) => skin.say(super::FILES_READ_ONLY_NOTE),
+        (true, false) => super::files_editing_note(form, skin.locale),
+        (false, _) => skin.say(READ_ONLY_NOTE),
     };
     egui::Frame::new()
         .fill(skin.env.bar_bg())
@@ -705,9 +705,7 @@
             ui.horizontal_top(|ui| {
                 ui.spacing_mut().item_spacing.x = 10.0;
                 let on = form.read_only();
-                let mut toggled = ui
-                    .add_enabled_ui(!fixed, |ui| env_check(ui, on, &title, skin).clicked())
-                    .inner;
+                let mut toggled = env_check(ui, on, &title, skin).clicked();
                 ui.vertical(|ui| {
                     // The title toggles the box under the pointer, as a
                     // check box's own label does. It takes clicks alone:
@@ -715,14 +713,14 @@
                     let label = Text::one(look, TextRole::UiBodyStrong, &title, palette.text)
                         .layout(ui.ctx())
                         .label_sense(ui, Sense::CLICK);
-                    toggled |= label.clicked() && !fixed;
+                    toggled |= label.clicked();
                     ui.add_space(2.0);
                     // The secondary colour, warmed by the environment's
                     // text colour: 0.4 is the share at which production's
                     // red on the light palette comes out in the design's
                     // tone.
                     let tone = theme::mix(palette.secondary, skin.env_ink(&skin.env), 0.4);
-                    widgets::label(ui, TextRole::Secondary, &skin.say(note), tone, look);
+                    widgets::label(ui, TextRole::Secondary, &note, tone, look);
                 });
                 if toggled {
                     form.read_only = Some(!on);
diff --git a/src/ui/connect_dialog/terminal.rs b/src/ui/connect_dialog/terminal.rs
--- a/src/ui/connect_dialog/terminal.rs
+++ b/src/ui/connect_dialog/terminal.rs
@@ -178,6 +178,22 @@
 /// The terminal look: a label in the 150 pt column, then what `add` draws
 /// on the same line, 8 below the row before it. The row is as tall as a
 /// field.
+/// What a Files connection's read-only row says while it edits: that a
+/// save writes a whole file, and the largest it would.
+fn terminal_editing_note(form: &ConnectionForm, locale: crate::i18n::Locale) -> String {
+    match super::largest_rewritten(form) {
+        Some((file, size)) => crate::i18n::fill(
+            &gettext(
+                locale,
+                "changes wait for :w, which rewrites the whole file ({file} {size})",
+            ),
+            // The size in the look's case, as its list rows write it.
+            &[("{file}", &file), ("{size}", &size.to_lowercase())],
+        ),
+        None => gettext(locale, "changes wait for :w, which rewrites the whole file").into_owned(),
+    }
+}
+
 pub(super) fn terminal_row(
     ui: &mut Ui,
     label: &str,
@@ -376,12 +392,30 @@
             let line = role.row_height(ui.ctx(), look.faces);
             terminal_row_of(ui, &skin.say(msgid("Read-only")), line, skin, |ui, _| {
                 let name = gettext(skin.locale, "Open read-only");
-                let fixed = super::read_only_fixed(form);
-                let (promise, note) = match fixed {
-                    true => (skin.say(msgid("Files open read-only")), String::new()),
-                    false => (
+                // A Files connection's says what a save does to its files,
+                // and while it edits, in the warning's colour.
+                let (promise, note, tone) = match (super::is_files(form), form.read_only()) {
+                    // In the look's own case, as its board writes them.
+                    (true, true) => (
+                        look.label(&skin.say(msgid("Files open read-only"))),
+                        format!(
+                            "· {}",
+                            look.label(
+                                &skin
+                                    .say(msgid("Turn off to edit; saving rewrites the whole file"))
+                            )
+                        ),
+                        palette.dim,
+                    ),
+                    (true, false) => (
+                        look.label(&skin.say(msgid("Editing on"))),
+                        format!("· {}", terminal_editing_note(form, skin.locale)),
+                        palette.warning,
+                    ),
+                    (false, _) => (
                         skin.say(msgid("Block every write from this app")),
                         format!("· {}", skin.say(msgid("Default for production"))),
+                        palette.dim,
                     ),
                 };
                 // The note goes under the promise where one line has no
@@ -393,15 +427,10 @@
                     // The environment's colour, as text can take it.
                     let mark = skin.env_ink(&skin.env);
                     let mut on = form.read_only();
-                    let changed = ui
-                        .add_enabled_ui(!fixed, |ui| {
-                            terminal_check(ui, &mut on, &promise, &name, mark, skin).changed()
-                        })
-                        .inner;
-                    if changed {
+                    if terminal_check(ui, &mut on, &promise, &name, mark, skin).changed() {
                         form.read_only = Some(on);
                     }
-                    widgets::label(ui, role, &note, palette.dim, look);
+                    widgets::label(ui, role, &note, tone, look);
                 };
                 if one_line {
                     draw(ui);
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -216,10 +216,16 @@
     // pointer: the terminal's bar says it in its own tag.
     let production =
         Some(workspace.environment).filter(|env| env.is_production() && !look.terminal);
-    let parts = match (look.terminal, view) {
+    let mut parts = match (look.terminal, view) {
         (true, ObjectView::Ddl) => super::ddl::subtitle(object, locale),
         _ => subtitle(object, &look, locale),
     };
+    // A file's table its connection does not edit, though it may write,
+    // says why first: "Excel files open read-only".
+    let never = Table::of(workspace, object).and_then(|table| table.never());
+    if let Some(lock @ (crate::edit::Lock::Unedited(_) | crate::edit::Lock::Unmatched)) = never {
+        parts.insert(0, look.label(&cell_editor::lock_text(lock, &name, locale)));
+    }
     let summary = parts.join(" · ");
     // Why Add row cannot be pressed, where it cannot: the table takes no
     // row, or its page is not there yet.
diff --git a/src/ui/pending_bar.rs b/src/ui/pending_bar.rs
--- a/src/ui/pending_bar.rs
+++ b/src/ui/pending_bar.rs
@@ -266,11 +266,66 @@
         )),
         Note::NotSent => say(msgid("Not connected. Nothing was sent.")),
         Note::Cancelled => format!("{} {nothing}", say(msgid("Save cancelled."))),
-        Note::Refused(error) => format!(
-            "{} {nothing}",
-            sentence(&format::capped(&error.to_string()))
+        Note::Refused(error) => match unsaved_of(error) {
+            Some(why) => unsaved_text(why, object, locale, &own),
+            None => format!(
+                "{} {nothing}",
+                sentence(&format::capped(&error.to_string()))
+            ),
+        },
+    }
+}
+
+/// Why a save to a Files connection's file wrote nothing, where it was the
+/// file's own doing.
+fn unsaved_of(error: &tabletist_db::Error) -> Option<&tabletist_db::Unsaved> {
+    match error {
+        tabletist_db::Error::Query { named, .. } => named.unsaved.as_ref(),
+        _ => None,
+    }
+}
+
+/// What a save that the file kept from writing says: why, and that every
+/// change it had is still there to save.
+fn unsaved_text(
+    why: &tabletist_db::Unsaved,
+    object: &ObjectTab,
+    locale: Locale,
+    own: &impl Fn(&str) -> String,
+) -> String {
+    let counts = object.edits.counts();
+    let n = counts.changes + counts.added + counts.deleted;
+    let (words, file) = match why {
+        tabletist_db::Unsaved::Changed(file) => (
+            ngettext(
+                locale,
+                "{file} changed on disk since you opened it. Reload to see the new version; your \
+                 {n} change stays pending.",
+                "{file} changed on disk since you opened it. Reload to see the new version; your \
+                 {n} changes stay pending.",
+                n,
+            ),
+            file,
         ),
-    }
+        tabletist_db::Unsaved::Unwritable(file) => (
+            ngettext(
+                locale,
+                "{file} couldn't be saved: its folder can't be written to. Your {n} change stays \
+                 pending.",
+                "{file} couldn't be saved: its folder can't be written to. Your {n} changes stay \
+                 pending.",
+                n,
+            ),
+            file,
+        ),
+    };
+    crate::i18n::fill(
+        &own(&words),
+        &[
+            ("{file}", &format::display_safe(file)),
+            ("{n}", &crate::i18n::number(locale, n)),
+        ],
+    )
 }
 
 /// What a save that wrote says once it is done: `written 2 changes · 1 row
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -960,8 +960,13 @@
             Some(FileRowText {
                 second,
                 middle,
-                // Editing files is not built: every one reads only.
-                right: Some((msgid("Read-only"), Joins::of(spec, store))),
+                right: Some((
+                    match connection.read_only() {
+                        true => msgid("Read-only"),
+                        false => msgid("Editable"),
+                    },
+                    Joins::of(spec, store),
+                )),
             })
         }
     }
````

- [ ] **Step 4: The catalogs**

Run `sh assets/i18n/update.sh`, then fill these in `assets/i18n/ru.po` and `assets/i18n/pt-BR.po` (a plural's forms are split by " / "):

| msgid | ru | pt-BR |
| --- | --- | --- |
| .duckdb files open read-only | Файлы .duckdb открываются только для чтения | Arquivos .duckdb abrem em somente leitura |
| Attached connections open read-only | Присоединённые подключения открываются только для чтения | Conexões anexadas abrem em somente leitura |
| Excel files open read-only | Файлы Excel открываются только для чтения | Arquivos do Excel abrem em somente leitura |
| {table} opens read-only: Tabletist couldn't match its rows to the file's lines | {table} открывается только для чтения: Tabletist не смог сопоставить её строки со строками файла | {table} abre em somente leitura: o Tabletist não conseguiu associar as linhas dela às linhas do arquivo |
| Attached connections open read-only here, even when the files are editable. | Присоединённые подключения здесь открываются только для чтения, даже когда файлы можно изменять. | Conexões anexadas abrem em somente leitura aqui, mesmo quando os arquivos são editáveis. |
| Editing is on. Changes are kept until you save; saving rewrites the whole file, so {file} ({size}) is written again in full. | Изменение включено. Изменения хранятся до сохранения; сохранение перезаписывает файл целиком, так что {file} ({size}) записывается заново полностью. | A edição está ativada. As alterações ficam guardadas até você salvar; salvar regrava o arquivo inteiro, então {file} ({size}) é gravado de novo por completo. |
| Editing is on. Changes are kept until you save; saving rewrites the whole file. | Изменение включено. Изменения хранятся до сохранения; сохранение перезаписывает файл целиком. | A edição está ativada. As alterações ficam guardadas até você salvar; salvar regrava o arquivo inteiro. |
| Files open read-only. Turn off to edit: changes are kept until you save, and saving rewrites the whole file. | Файлы открываются только для чтения. Выключите, чтобы изменять: изменения хранятся до сохранения, а сохранение перезаписывает файл целиком. | Os arquivos abrem em somente leitura. Desative para editar: as alterações ficam guardadas até você salvar, e salvar regrava o arquivo inteiro. |
| Editing on | Изменение включено | Edição ativada |
| Turn off to edit; saving rewrites the whole file | Выключите, чтобы изменять; сохранение перезаписывает файл целиком | Desative para editar; salvar regrava o arquivo inteiro |
| changes wait for :w, which rewrites the whole file | изменения ждут :w, которое перезаписывает файл целиком | as alterações esperam :w, que regrava o arquivo inteiro |
| changes wait for :w, which rewrites the whole file ({file} {size}) | изменения ждут :w, которое перезаписывает файл целиком ({file} {size}) | as alterações esperam :w, que regrava o arquivo inteiro ({file} {size}) |
| {file} changed on disk since you opened it. Reload to see the new version; your {n} change stays pending. | {file} изменился на диске после того, как вы его открыли. Перезагрузите, чтобы увидеть новую версию; ваше {n} изменение остаётся в ожидании. / {file} изменился на диске после того, как вы его открыли. Перезагрузите, чтобы увидеть новую версию; ваши {n} изменения остаются в ожидании. / {file} изменился на диске после того, как вы его открыли. Перезагрузите, чтобы увидеть новую версию; ваши {n} изменений остаются в ожидании. | {file} mudou no disco desde que você o abriu. Recarregue para ver a nova versão; sua {n} alteração continua pendente. / {file} mudou no disco desde que você o abriu. Recarregue para ver a nova versão; suas {n} alterações continuam pendentes. |
| {file} couldn't be saved: its folder can't be written to. Your {n} change stays pending. | Не удалось сохранить {file}: в его папку нельзя записывать. Ваше {n} изменение остаётся в ожидании. / Не удалось сохранить {file}: в его папку нельзя записывать. Ваши {n} изменения остаются в ожидании. / Не удалось сохранить {file}: в его папку нельзя записывать. Ваши {n} изменений остаются в ожидании. | Não foi possível salvar {file}: não é possível gravar na pasta dele. Sua {n} alteração continua pendente. / Não foi possível salvar {file}: não é possível gravar na pasta dele. Suas {n} alterações continuam pendentes. |
| Editable | Можно изменять | Editável |
| A file is edited in its table, then saved. | Файл изменяют в его таблице, затем сохраняют. | Um arquivo é editado na tabela dele e depois salvo. |
| A Files connection's scripts only read, so {database} refused {statement}. | Скрипты подключения к файлам только читают, поэтому {database} отклонил {statement}. | Os scripts de uma conexão de arquivos só leem, então {database} recusou {statement}. |
| Change the rows in the file's table, then save them. | Измените строки в таблице файла, затем сохраните их. | Altere as linhas na tabela do arquivo e depois salve-as. |
| Files are edited in their tables | Файлы изменяют в их таблицах | Os arquivos são editados nas tabelas deles |

- [ ] **Step 5: Run them to see them pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib -- files_tests files_edit_tests picker::tests`

Expected: 33 pass.

- [ ] **Step 6: Run the full check**

Run the five commands of the full check above.

Expected: all pass; 5,755 passed, 0 failed.

- [ ] **Step 7: Commit**

```bash
git add src assets/i18n docs
git commit -m "Say what a save of a file does, why a file isn't edited, and why a save wrote nothing"
```

---

## Not done in this plan

- **The hand test**, in both looks, on a real Files connection over the Bookshop's exports:
  - turn the box off and read its note;
  - edit, add and delete rows of `orders.csv`, save, and diff the file;
  - do the same to `customers.parquet` and `events.ndjson`;
  - change a file in another program, then save and reload;
  - make the folder read-only, then save;
  - open an Excel file's table and a `.duckdb` file's;
  - compare with the two boards by eye.
- Nothing is run on macOS or Windows beyond CI. On Windows `std::fs::rename` replaces an existing file; whether it can while DuckDB reads the file is untested.
- Run 4b: `.duckdb` files written in place, in a transaction.
