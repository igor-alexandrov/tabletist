# Files connection 4b: `.duckdb` files edited in place

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** With a Files connection's **Open read-only** box off, a `.duckdb` file's tables are edited in the grid like any other table and saved in place: in one transaction, never written again in full. The file stays readable by other programs while the connection is open.

**Architecture:**
- **The db layer** (`crates/tabletist-db/src/duck.rs`, `duck/database.rs` new, `duck/edit.rs`, `error.rs`, `fixtures.rs`):
  - Every session still attaches a `.duckdb` file `READ_ONLY`. That lock lets other programs read the file, never write it.
  - A process-wide gate per file (`database::Shared`, keyed by the canonical path) keeps the app's own sessions apart. DuckDB's lock can't do that: one process may attach a file twice, even read-write. Each call holds the session's files for reading (`database::reading`, in `Conn::run`). A save holds its file alone (`Conn::run_gated(false, …)` and `gate.write()`).
  - Each file has a save number (`Shared::saves`). A session whose attachment is older attaches the file again before its next call, so it reads the file as the last save left it.
  - A save, under the gate:
    1. checks the rows' keys;
    2. checks the file's size and modified time against this session's last attach (`Unsaved::Changed`), and that it isn't read-only (`Unsaved::Protected`);
    3. detaches the file and attaches it to write;
    4. runs the inserts, updates and deletes in one transaction and reads each changed row back;
    5. commits, then detaches and attaches read-only again;
    6. bumps the save number.
    - If the attach to write fails because another process holds the file, the save writes nothing and returns `Unsaved::Busy`. If the holder is this process, which happens on Windows, it returns `Unsaved::OpenHere`. DuckDB's words name the holder's PID.
  - `describe` reports a `.duckdb` table's primary key (`duckdb_constraints()`). A keyed table's page is read in key order and saved by its key.
  - A table with no key, in a session that may write, is read through a view in `tabletist_edits`: `SELECT rowid AS __tabletist_position, *`. Its page carries `RowPage::positions`, with the save number as `Positions::copy`. A save checks those keys with 4a's `own_keys` against `rowid`, and refuses ones from an older save (`Unsaved::Reread`).
  - `Unedited::DuckDb` is gone.
  - The Bookshop fixture's `bookshop.duckdb` keys `order_items` by `id`. `book_covers` keeps no key.
- **The UI** (`src/ui/pending_bar.rs`, `cell_editor.rs`, `connect_dialog/{mod,terminal}.rs`):
  - A refused save says another program has the file open, or another tab, export or agent of Tabletist.
  - The Open read-only box's note, when off, says `.duckdb` files are changed in place.
  - ".duckdb files open read-only" is gone.
- **Docs:** `docs/_guide/editing-data.md` gains "Edit a .duckdb file". `docs/_reference/connections.md` gives the notes.

**Tech Stack:** Rust, egui (`egui_kittest` headless tests), DuckDB (`duckdb` 1.20000.1, DuckDB 2.0.0), gettext catalogs (`assets/i18n`).

**Base:** `main` at `d61e158d`, branch `claude/files-connection-4b-duckdb`. Run 4a (#389) is merged.

---

## What was decided

Igor picked the recommended option of each question:

| Question | Decision |
| --- | --- |
| How a `.duckdb` file is attached when editing is on | Read-only in every session; a save attaches it to write only while it runs, holding the app's per-file gate alone, then read-only again; other sessions on the file wait and attach it again after |
| How a `.duckdb` table's rows are found | By its primary key where it has one; otherwise by `rowid`, as 4a's positions (the page is read again after a save) |
| What the Open read-only notes say with `.duckdb` files | Off, with a file a save rewrites: "…written again in full; .duckdb files are changed in place."; off, with only `.duckdb` files: "Editing is on. Changes are kept until you save; .duckdb files are changed in place, in a transaction."; on: the prompt's words |
| Windows, where another Tabletist session's attachment keeps a save from opening the file | Refused, and said so: "…another tab, export or agent of Tabletist has it open." |

### What the probe found (DuckDB 2.0.0, Linux)

- A write to a `.duckdb` file attached before the fence passes the fence with only the file's path in `allowed_paths`. Its `.wal` needs none.
- `DETACH` and `ATTACH` of an allowed path still work after `lock_configuration`. An `ATTACH` of any other path is refused.
- Across processes:
  - A read-write attach locks out every other process, even from reading.
  - A read-only attach lets others read, but not write: "Could not set lock on file …: Conflicting lock is held in … (PID n)".
- Inside one process, two DuckDB instances can both attach one file read-write with no lock, because POSIX locks are per process. One then reads stale rows.
- On Windows DuckDB's handles refuse each other ("being used by another process. File is already open in … (PID n)"), inside one process too. 4a's Windows CI runs showed it.
- `rowid` renumbers at a checkpoint after deletes, and `DETACH` checkpoints. A view in an attached in-memory database can read `SELECT rowid AS …, *` of a `.duckdb` table, and keeps working after the file is detached and attached again. A quoted `"rowid"` works in `UPDATE … WHERE` and `DELETE … WHERE`.
- `duckdb_constraints()` gives a table's primary key as `constraint_column_names`.

### Where this plan departs from the prompt or the canvas

| The prompt | Built | Why |
| --- | --- | --- |
| "`.duckdb`: `ATTACH … (READ_ONLY)`, unless editing is on." | Always read-only; read-write only while a save runs | Decided above: a read-write attach locks every other program out of the file, and two sessions of the app could both hold it read-write |
| "Files have no primary key… identify rows by [a hidden row number]" | A `.duckdb` table with a primary key is found by it; one without by `rowid` | Decided above |
| "Note when on: … saving rewrites the whole file." | Kept, with `.duckdb` files too | Decided above |
| — | On Windows, another session of the app holding the file refuses a save, with its own message | Decided above |

## Files

- `crates/tabletist-db/src/duck/database.rs` (new): the gate, attaching again, a table's key, the `rowid` view, the save in place.
- `crates/tabletist-db/src/duck.rs`: the session holds its `.duckdb` files (`databases`); `run` takes the gate; `describe`, `fetch_rows` and `write` for a `.duckdb` table; `Unedited::DuckDb` goes.
- `crates/tabletist-db/src/duck/edit.rs`: `Stamp::of`, `page`'s source, `own_keys`' column and `protected` are shared.
- `crates/tabletist-db/src/error.rs`: `Unsaved::Busy`, `Unsaved::OpenHere`.
- `crates/tabletist-db/src/fixtures.rs`: `order_items` keyed by `id`.
- `crates/tabletist-db/tests/files_duckdb.rs` (new), `tests/files_edit.rs`: 4a's ".duckdb tables say why they are not edited" goes.
- `src/ui/pending_bar.rs`, `src/ui/cell_editor.rs`, `src/ui/files_edit_tests.rs`: Task 1.
- `src/ui/connect_dialog/mod.rs`, `src/ui/connect_dialog/terminal.rs`, `src/ui/files_tests.rs`: Task 2.
- `assets/i18n/*`: made by `update.sh`, filled by the script in Task 2 Step 4.
- `docs/_guide/editing-data.md`, `docs/_reference/connections.md`: Task 2.

The commands below run from the worktree root with `CARGO_TARGET_DIR=$PWD/target` and `~/.cargo/bin/cargo`. The full check is:

```bash
~/.cargo/bin/cargo fmt --check
~/.cargo/bin/cargo clippy --workspace --all-targets -- -D warnings
TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist ~/.cargo/bin/cargo test --workspace -q --no-fail-fast
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --workspace --no-deps
assets/i18n/update.sh --check
```

---

### Task 1: The db layer saves a `.duckdb` file in place

**Files:** everything under `crates/tabletist-db/`, `src/ui/pending_bar.rs`, `src/ui/cell_editor.rs`, `src/ui/files_edit_tests.rs`.

- [ ] **Step 1: Write the failing tests**

They cover:
- a keyed table saved in place, its page by key, with nothing left beside the file;
- a keyless table by `rowid`, read again after a save, its old keys refused;
- another session reading the file as a save left it (on Windows, refused with `OpenHere`);
- a failed change writing nothing;
- a read-only session saving nothing;
- a replaced file, and a read-only one;
- another process holding the file: a child copy of the test binary, `hold_the_file`, attaches it read-only.

In the UI: a `.duckdb` table's save finds its row by `id`, and the two refusals are worded. 4a's test that `.duckdb` tables say why they aren't edited goes. The fixture keys `order_items`.

````diff
diff --git a/crates/tabletist-db/src/fixtures.rs b/crates/tabletist-db/src/fixtures.rs
--- a/crates/tabletist-db/src/fixtures.rs
+++ b/crates/tabletist-db/src/fixtures.rs
@@ -54,8 +54,8 @@
 /// export them: `orders.csv`, `returns.csv`, `authors.tsv` (tab
 /// separated), `customers.parquet`, `books.json` (an array),
 /// `events.ndjson` (an order placed, a line each), and `bookshop.duckdb`
-/// holding `book_covers` and `order_items`. Refuses a folder that holds
-/// any of them already.
+/// holding `book_covers` (no primary key) and `order_items` (keyed by
+/// `id`). Refuses a folder that holds any of them already.
 pub fn write_bookshop_files(dir: &Path) -> Result<()> {
     let io = |error: &dyn std::fmt::Display| Error::Io(error.to_string());
     if let Some(there) = BOOKSHOP_FILES.iter().find(|name| dir.join(name).exists()) {
@@ -125,6 +125,7 @@
          ATTACH {} AS shop;
          CREATE TABLE shop.book_covers AS SELECT * FROM book_covers;
          CREATE TABLE shop.order_items AS SELECT * FROM order_items;
+         ALTER TABLE shop.order_items ADD PRIMARY KEY (id);
          DETACH shop;",
         path("orders.csv"),
         path("returns.csv"),
diff --git a/crates/tabletist-db/tests/files_duckdb.rs b/crates/tabletist-db/tests/files_duckdb.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-db/tests/files_duckdb.rs
@@ -0,0 +1,397 @@
+//! Editing a Files connection's `.duckdb` files in place, through the
+//! public `Connection` API, on the Bookshop's exports
+//! (`fixtures::write_bookshop_files`): `bookshop.duckdb` holds
+//! `order_items`, keyed by `id`, and `book_covers`, with no key. Every
+//! session attaches the file read-only; a save writes it alone, in one
+//! transaction, and every session reads it as the save left it.
+
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
+use std::io::{BufRead, BufReader};
+use std::path::Path;
+use std::process::{Command, Stdio};
+
+use tabletist_db::files::Source;
+use tabletist_db::{
+    Access, CellChange, ChangeSet, ConnectSpec, Connection, Error, FILE_COPY, FILE_POSITION,
+    HostKeys, InsertValue, NewValue, ObjectRef, RowChange, RowDelete, RowInsert, RowPage, RowQuery,
+    Secrets, StopFlag, Unsaved, Value, WriteOutcome,
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
+fn shop(name: &str) -> ObjectRef {
+    ObjectRef::new("bookshop", name)
+}
+
+async fn page(connection: &Connection, name: &str) -> RowPage {
+    connection
+        .fetch_rows(&RowQuery::new(shop(name), 100))
+        .await
+        .unwrap()
+}
+
+fn set(name: &str) -> ChangeSet {
+    ChangeSet {
+        object: shop(name),
+        inserts: Vec::new(),
+        rows: Vec::new(),
+        deletes: Vec::new(),
+    }
+}
+
+/// What finds `order_items`' row `id`.
+fn by_id(id: i64) -> Vec<(String, Value)> {
+    vec![("id".to_owned(), Value::Int(id))]
+}
+
+/// What finds `book_covers`' row at `place`, as `page` has its places.
+fn at(page: &RowPage, place: i64) -> Vec<(String, Value)> {
+    let copy = page.positions.as_ref().unwrap().copy;
+    vec![
+        (FILE_POSITION.to_owned(), Value::Int(place)),
+        (FILE_COPY.to_owned(), Value::Int(copy)),
+    ]
+}
+
+/// A change of one cell of the row `key` finds.
+fn change(
+    key: Vec<(String, Value)>,
+    column: &str,
+    type_name: &str,
+    loaded: Value,
+    new: &str,
+) -> RowChange {
+    RowChange {
+        key,
+        set: vec![CellChange {
+            column: column.to_owned(),
+            type_name: type_name.to_owned(),
+            loaded,
+            new: NewValue::Text(new.to_owned()),
+        }],
+    }
+}
+
+async fn save(connection: &Connection, changes: &ChangeSet) -> tabletist_db::Result<WriteOutcome> {
+    connection.write(changes, &StopFlag::new()).await
+}
+
+fn unsaved(error: &Error) -> Option<Unsaved> {
+    match error {
+        Error::Query { named, .. } => named.unsaved.clone(),
+        _ => None,
+    }
+}
+
+/// The column `name` of `page`'s rows.
+fn column(page: &RowPage, name: &str) -> Vec<Value> {
+    let at = page
+        .columns
+        .iter()
+        .position(|column| column.name == name)
+        .unwrap();
+    page.rows.iter().map(|row| row[at].clone()).collect()
+}
+
+#[tokio::test]
+async fn a_keyed_table_is_found_by_its_key_and_saved_in_place() {
+    let (connection, dir) = editable().await;
+    let items = connection.describe(&shop("order_items")).await.unwrap();
+    assert_eq!(items.primary_key, vec!["id".to_owned()]);
+    assert_eq!(items.unedited, None);
+    let before = page(&connection, "order_items").await;
+    assert!(before.positions.is_none());
+    assert!(before.ordered_by_key);
+    let quantity = column(&before, "quantity")[1].clone();
+    let mut changes = set("order_items");
+    changes
+        .rows
+        .push(change(by_id(2), "quantity", "INTEGER", quantity, "5"));
+    changes.deletes.push(RowDelete { key: by_id(3) });
+    changes.inserts.push(RowInsert {
+        set: ["id", "order_id", "book_id", "quantity", "price"]
+            .iter()
+            .zip(["100", "1", "2", "1", "12.00"])
+            .map(|(name, value)| InsertValue {
+                column: (*name).to_owned(),
+                type_name: String::new(),
+                new: NewValue::Text(value.to_owned()),
+            })
+            .collect(),
+    });
+    let outcome = save(&connection, &changes).await.unwrap();
+    let WriteOutcome::Written { rows, gone, .. } = &outcome else {
+        panic!("{outcome:?}");
+    };
+    assert!(gone.is_empty());
+    assert_eq!(rows.len(), 1);
+    // In the file itself: a session of its own reads it, read-only.
+    drop(connection);
+    let reader = open(dir.path(), Access::ReadOnly).await;
+    let after = page(&reader, "order_items").await;
+    let ids = column(&after, "id");
+    assert!(ids.contains(&Value::Int(100)));
+    assert!(!ids.contains(&Value::Int(3)));
+    let at = ids.iter().position(|id| *id == Value::Int(2)).unwrap();
+    assert_eq!(column(&after, "quantity")[at], Value::Int(5));
+    // Nothing beside it: no hidden file, no log left.
+    let mut names: Vec<String> = std::fs::read_dir(dir.path())
+        .unwrap()
+        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
+        .filter(|name| name.starts_with("bookshop") || name.starts_with('.'))
+        .collect();
+    names.sort();
+    assert_eq!(names, vec!["bookshop.duckdb".to_owned()]);
+}
+
+#[tokio::test]
+async fn a_table_with_no_key_is_found_by_rowid_and_read_again_after_a_save() {
+    let (connection, _dir) = editable().await;
+    let covers = connection.describe(&shop("book_covers")).await.unwrap();
+    assert!(covers.primary_key.is_empty());
+    let before = page(&connection, "book_covers").await;
+    let positions = before.positions.clone().unwrap();
+    assert_eq!(positions.rows, vec![0, 1, 2]);
+    // The place isn't one of the page's columns.
+    assert!(
+        before
+            .columns
+            .iter()
+            .all(|column| column.name != FILE_POSITION)
+    );
+    let kind = column(&before, "kind")[1].clone();
+    let mut changes = set("book_covers");
+    changes.deletes.push(RowDelete {
+        key: at(&before, 0),
+    });
+    changes
+        .rows
+        .push(change(at(&before, 1), "kind", "VARCHAR", kind, "spine"));
+    assert!(matches!(
+        save(&connection, &changes).await.unwrap(),
+        WriteOutcome::Written { .. }
+    ));
+    let after = page(&connection, "book_covers").await;
+    assert_eq!(column(&after, "kind")[0], Value::Text("spine".into()));
+    assert_eq!(after.rows.len(), 2);
+    let renumbered = after.positions.unwrap();
+    assert_ne!(renumbered.copy, positions.copy);
+    // The old page's places are of the file before the save.
+    let mut again = set("book_covers");
+    again.deletes.push(RowDelete {
+        key: at(&before, 2),
+    });
+    let error = save(&connection, &again).await.unwrap_err();
+    assert_eq!(
+        unsaved(&error),
+        Some(Unsaved::Reread("bookshop.duckdb".into()))
+    );
+}
+
+// On Windows another session's hold keeps a save from opening the file
+// to write: see `on_windows_a_file_another_session_has_open_is_not_written`.
+#[cfg(not(windows))]
+#[tokio::test]
+async fn another_session_reads_the_file_as_the_save_left_it() {
+    let (connection, dir) = editable().await;
+    let reader = open(dir.path(), Access::ReadOnly).await;
+    let before = page(&reader, "order_items").await;
+    let quantity = column(&before, "quantity")[0].clone();
+    let mut changes = set("order_items");
+    changes
+        .rows
+        .push(change(by_id(1), "quantity", "INTEGER", quantity, "9"));
+    save(&connection, &changes).await.unwrap();
+    let after = page(&reader, "order_items").await;
+    assert_eq!(column(&after, "quantity")[0], Value::Int(9));
+}
+
+#[cfg(windows)]
+#[tokio::test]
+async fn on_windows_a_file_another_session_has_open_is_not_written() {
+    let (connection, dir) = editable().await;
+    let reader = open(dir.path(), Access::ReadOnly).await;
+    page(&reader, "order_items").await;
+    let mut changes = set("order_items");
+    changes.deletes.push(RowDelete { key: by_id(1) });
+    let error = save(&connection, &changes).await.unwrap_err();
+    assert_eq!(
+        unsaved(&error),
+        Some(Unsaved::OpenHere("bookshop.duckdb".into()))
+    );
+    // Saved once that session is gone.
+    drop(reader);
+    assert!(matches!(
+        save(&connection, &changes).await.unwrap(),
+        WriteOutcome::Written { .. }
+    ));
+}
+
+#[tokio::test]
+async fn a_failed_change_writes_nothing() {
+    let (connection, dir) = editable().await;
+    let before = std::fs::read(dir.path().join("bookshop.duckdb")).unwrap();
+    // id 1 is taken: the insert fails, and the update before it is undone.
+    let mut changes = set("order_items");
+    changes
+        .rows
+        .push(change(by_id(2), "quantity", "INTEGER", Value::Int(3), "7"));
+    changes.inserts.push(RowInsert {
+        set: vec![InsertValue {
+            column: "id".to_owned(),
+            type_name: String::new(),
+            new: NewValue::Text("1".to_owned()),
+        }],
+    });
+    let outcome = save(&connection, &changes).await.unwrap();
+    assert!(
+        matches!(outcome, WriteOutcome::FailedInsert { insert: 0, .. }),
+        "{outcome:?}"
+    );
+    let after = page(&connection, "order_items").await;
+    let at = column(&after, "id")
+        .iter()
+        .position(|id| *id == Value::Int(2))
+        .unwrap();
+    assert_eq!(column(&after, "quantity")[at], Value::Int(3));
+    drop(connection);
+    assert_eq!(
+        std::fs::read(dir.path().join("bookshop.duckdb")).unwrap(),
+        before
+    );
+}
+
+#[tokio::test]
+async fn a_session_that_reads_saves_nothing() {
+    let dir = tempfile::tempdir().unwrap();
+    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
+    let connection = open(dir.path(), Access::ReadOnly).await;
+    let mut changes = set("order_items");
+    changes.deletes.push(RowDelete { key: by_id(1) });
+    assert_eq!(
+        save(&connection, &changes).await.unwrap_err(),
+        Error::ReadOnly
+    );
+    // And its pages carry no places.
+    assert!(page(&connection, "book_covers").await.positions.is_none());
+}
+
+// Windows renames no file over one a program has open.
+#[cfg(unix)]
+#[tokio::test]
+async fn a_file_replaced_on_disk_is_not_written() {
+    let (connection, dir) = editable().await;
+    page(&connection, "order_items").await;
+    // Another program puts a file of its own in its place.
+    let other = tempfile::tempdir().unwrap();
+    tabletist_db::fixtures::write_bookshop_files(other.path()).unwrap();
+    let theirs = other.path().join("bookshop.duckdb");
+    {
+        let duck = duckdb::Connection::open(&theirs).unwrap();
+        duck.execute_batch("INSERT INTO book_covers SELECT * FROM book_covers")
+            .unwrap();
+    }
+    std::fs::rename(&theirs, dir.path().join("bookshop.duckdb")).unwrap();
+    let replaced = std::fs::read(dir.path().join("bookshop.duckdb")).unwrap();
+    let mut changes = set("order_items");
+    changes.deletes.push(RowDelete { key: by_id(1) });
+    let error = save(&connection, &changes).await.unwrap_err();
+    assert_eq!(
+        unsaved(&error),
+        Some(Unsaved::Changed("bookshop.duckdb".into()))
+    );
+    drop(connection);
+    assert_eq!(
+        std::fs::read(dir.path().join("bookshop.duckdb")).unwrap(),
+        replaced
+    );
+}
+
+#[cfg(unix)]
+#[tokio::test]
+async fn a_file_its_owner_may_not_write_is_not_written() {
+    use std::os::unix::fs::PermissionsExt;
+    let (connection, dir) = editable().await;
+    let path = dir.path().join("bookshop.duckdb");
+    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
+    let mut changes = set("order_items");
+    changes.deletes.push(RowDelete { key: by_id(1) });
+    let error = save(&connection, &changes).await.unwrap_err();
+    assert_eq!(
+        unsaved(&error),
+        Some(Unsaved::Protected("bookshop.duckdb".into()))
+    );
+    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
+}
+
+/// Run by `a_file_another_program_has_open_is_not_written` as the other
+/// program: holds the file at `TABLETIST_HOLD` open, read-only, until its
+/// input ends. Does nothing in a plain test run.
+#[test]
+fn hold_the_file() {
+    let Ok(path) = std::env::var("TABLETIST_HOLD") else {
+        return;
+    };
+    let duck = duckdb::Connection::open_in_memory().unwrap();
+    duck.execute_batch(&format!("ATTACH '{path}' AS held (READ_ONLY)"))
+        .unwrap();
+    println!("held");
+    let mut line = String::new();
+    while std::io::stdin().read_line(&mut line).unwrap() > 0 {}
+}
+
+#[tokio::test]
+async fn a_file_another_program_has_open_is_not_written() {
+    let (connection, dir) = editable().await;
+    let path = dir.path().join("bookshop.duckdb");
+    let mut other = Command::new(std::env::current_exe().unwrap())
+        .args([
+            "--exact",
+            "hold_the_file",
+            "--nocapture",
+            "--test-threads=1",
+        ])
+        .env("TABLETIST_HOLD", &path)
+        .stdin(Stdio::piped())
+        .stdout(Stdio::piped())
+        .spawn()
+        .unwrap();
+    let mut said = BufReader::new(other.stdout.take().unwrap()).lines();
+    // The test runner writes its own words on the line before it.
+    assert!(said.any(|line| line.unwrap().ends_with("held")));
+    let mut changes = set("order_items");
+    changes.deletes.push(RowDelete { key: by_id(1) });
+    let error = save(&connection, &changes).await.unwrap_err();
+    assert_eq!(
+        unsaved(&error),
+        Some(Unsaved::Busy("bookshop.duckdb".into()))
+    );
+    drop(other.stdin.take());
+    other.wait().unwrap();
+    // Still read, read-only, and saved once the other program lets go.
+    assert_eq!(page(&connection, "order_items").await.rows.len(), 40);
+    assert!(matches!(
+        save(&connection, &changes).await.unwrap(),
+        WriteOutcome::Written { .. }
+    ));
+}
diff --git a/crates/tabletist-db/tests/files_edit.rs b/crates/tabletist-db/tests/files_edit.rs
--- a/crates/tabletist-db/tests/files_edit.rs
+++ b/crates/tabletist-db/tests/files_edit.rs
@@ -13,7 +13,7 @@
 use tabletist_db::{
     Access, CellChange, ChangeSet, ConnectSpec, Connection, Error, FILE_COPY, FILE_POSITION,
     HostKeys, InsertValue, NewValue, ObjectRef, RowChange, RowDelete, RowInsert, RowPage, RowQuery,
-    Secrets, StopFlag, Unedited, Unsaved, Value, WriteOutcome,
+    Secrets, StopFlag, Unsaved, Value, WriteOutcome,
 };
 
 /// The Bookshop's exports in a folder of their own, and a session over it
@@ -682,29 +682,6 @@
 }
 
 #[tokio::test]
-async fn a_duckdb_file_s_tables_say_why_they_are_not_edited() {
-    let (connection, _dir) = editable().await;
-    let covers = connection
-        .describe(&ObjectRef::new("bookshop", "book_covers"))
-        .await
-        .unwrap();
-    assert_eq!(covers.unedited, Some(Unedited::DuckDb));
-    let orders = connection.describe(&table("orders")).await.unwrap();
-    assert_eq!(orders.unedited, None);
-    // A save to one is refused, as on a session that reads.
-    let mut changes = set("orders");
-    changes.object = ObjectRef::new("bookshop", "book_covers");
-    let orders = page(&connection, "orders").await;
-    changes.deletes.push(RowDelete {
-        key: at(&orders, 0),
-    });
-    assert_eq!(
-        save(&connection, &changes).await.unwrap_err(),
-        Error::ReadOnly
-    );
-}
-
-#[tokio::test]
 async fn a_script_still_reads_only() {
     let (connection, dir) = editable().await;
     let before = std::fs::read(dir.path().join("orders.csv")).unwrap();
diff --git a/src/ui/files_edit_tests.rs b/src/ui/files_edit_tests.rs
--- a/src/ui/files_edit_tests.rs
+++ b/src/ui/files_edit_tests.rs
@@ -1,7 +1,8 @@
 //! Editing a Files connection's table, in both looks: a save finds each
 //! row by its place in the file, a save the file keeps from writing says
 //! why and keeps every change, and a table its connection does not edit
-//! says why in its header. On the Bookshop's `orders.csv`.
+//! says why in its header. On the Bookshop's `orders.csv`, and the
+//! `order_items` of its `.duckdb` file, saved in place by its key.
 
 use tabletist_db::files::Format;
 use tabletist_db::{
@@ -227,6 +228,115 @@
     assert_eq!(lock(&harness), None);
 }
 
+/// The Bookshop `.duckdb` file's `order_items`, keyed by `id`, open in a
+/// Files connection that may write.
+fn order_items(look: Look) -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_fake_as(false);
+    harness.app.workspace_mut(tab).unwrap().driver = Driver::Files;
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: ObjectRef::new("bookshop", "order_items"),
+        kind: ObjectKind::Table,
+        pin: true,
+    });
+    let column = |name: &str| ColumnInfo {
+        name: name.into(),
+        type_name: "INTEGER".into(),
+        nullable: name != "id",
+        ..ColumnInfo::default()
+    };
+    harness.answer_structure(Structure {
+        columns: vec![column("id"), column("quantity")],
+        primary_key: vec!["id".into()],
+        ..Structure::default()
+    });
+    let meta = |name: &str| ColumnMeta {
+        name: name.into(),
+        type_name: "INTEGER".into(),
+        kind: ValueKind::Numeric,
+    };
+    harness.answer_rows(RowPage {
+        columns: vec![meta("id"), meta("quantity")],
+        rows: (1..=3)
+            .map(|id| vec![Value::Int(id), Value::Int(id + 1)])
+            .collect(),
+        has_more: false,
+        ordered_by_key: true,
+        elapsed: std::time::Duration::from_millis(3),
+        positions: None,
+    });
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
+    harness.settle();
+    (harness, tab, id)
+}
+
+#[test]
+fn a_duckdb_file_s_table_is_saved_by_its_key_and_says_nothing_of_read_only() {
+    for look in looks() {
+        let (mut harness, tab, id) = order_items(look);
+        assert!(
+            !harness
+                .painted
+                .iter()
+                .any(|(text, _)| text.to_lowercase().contains("open read-only")),
+            "{}: {:?}",
+            look.name,
+            harness.painted
+        );
+        super::insert_audit_tests::set(&mut harness, tab, id, (1, 1), "5");
+        harness.app.apply(Action::WriteEdits { tab, id });
+        let changes = sent(&harness);
+        assert_eq!(changes.object, ObjectRef::new("bookshop", "order_items"));
+        assert_eq!(
+            changes.rows[0].key,
+            vec![("id".to_owned(), Value::Int(2))],
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn a_duckdb_file_another_program_has_open_keeps_every_change() {
+    for look in looks() {
+        let (mut harness, tab, id) = order_items(look);
+        super::insert_audit_tests::set(&mut harness, tab, id, (0, 1), "4");
+        super::insert_audit_tests::set(&mut harness, tab, id, (2, 1), "6");
+        harness.app.apply(Action::WriteEdits { tab, id });
+        refused(&mut harness, Unsaved::Busy("bookshop.duckdb".into()));
+        harness.settle();
+        let edits = super::insert_audit_tests::edits(&harness, tab, id);
+        assert_eq!(edits.counts().changes, 2, "{}", look.name);
+        let said = "bookshop.duckdb couldn't be saved: another program has it open. Your 2 \
+                    changes stay pending.";
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
+fn a_duckdb_file_another_session_of_the_app_has_open_says_so() {
+    let (mut harness, tab, id) = order_items(Look::macos());
+    super::insert_audit_tests::set(&mut harness, tab, id, (0, 1), "4");
+    harness.app.apply(Action::WriteEdits { tab, id });
+    refused(&mut harness, Unsaved::OpenHere("bookshop.duckdb".into()));
+    assert!(harness.has(
+        "bookshop.duckdb couldn't be saved: another tab, export or agent of Tabletist has it \
+         open. Your 1 change stays pending."
+    ));
+}
+
 #[test]
 fn a_file_changed_on_disk_keeps_every_change_and_says_how_many() {
     for look in looks() {
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist-db --test files_duckdb` and `~/.cargo/bin/cargo test -q -p tabletist --lib -- files_edit_tests`

Expected: they don't compile yet, for want of `Unsaved::Busy` and `Unsaved::OpenHere`:

```text
error[E0599]: no variant, associated function, or constant named `Busy` found for enum `Unsaved` in the current scope
error: could not compile `tabletist-db` (test "files_duckdb") due to 1 previous error
error[E0599]: no variant, associated function, or constant named `OpenHere` found for enum `Unsaved` in the current scope
error: could not compile `tabletist` (lib test) due to 2 previous errors
```

- [ ] **Step 3: Hold the file, and save it in place**

````diff
diff --git a/crates/tabletist-db/src/duck.rs b/crates/tabletist-db/src/duck.rs
--- a/crates/tabletist-db/src/duck.rs
+++ b/crates/tabletist-db/src/duck.rs
@@ -18,6 +18,7 @@
 //! after it: the fence keeps any other from being attached.
 
 mod attach;
+mod database;
 mod edit;
 
 use std::collections::HashMap;
@@ -101,8 +102,6 @@
 pub enum Unedited {
     /// An Excel file's: Excel files are never written.
     Excel,
-    /// A `.duckdb` file's table or view.
-    DuckDb,
     /// An attached connection's: those are attached read-only.
     Attached,
 }
@@ -133,6 +132,9 @@
     /// The files' tables read for editing so far, by name: those of a
     /// session that may write whose page was read.
     copies: Arc<Mutex<HashMap<String, edit::Loaded>>>,
+    /// Its `.duckdb` files, by their paths: each call holds them for
+    /// reading, and a save one alone.
+    databases: Arc<Vec<database::Held>>,
 }
 
 /// A saved connection the session attached: the name SQL calls it by,
@@ -722,7 +724,8 @@
             Types::Sample | Types::Text => reading,
         };
         // A `.duckdb` file is attached read-only, in a session that may
-        // write too: its tables are not edited yet.
+        // write too: a save attaches it to write for as long as it runs
+        // (`database`).
         let made = make(&connection, file, reading).and_then(|(statement, options)| {
             run(&statement)?;
             Ok((statement, options))
@@ -1637,6 +1640,12 @@
         .map_err(|error| Error::Io(error.to_string()))??;
         let (connection, files, failed, attached, version, order) = made;
         let interrupt = connection.interrupt_handle();
+        let mut databases: Vec<database::Held> = files
+            .iter()
+            .filter(|file| file.found.format.is_database())
+            .map(|file| database::Held::new(&file.found.name, &file.found.path))
+            .collect();
+        databases.sort_by(|one, other| one.path().cmp(other.path()));
         Ok(Self {
             inner: Arc::new(Mutex::new(connection)),
             interrupt,
@@ -1650,6 +1659,7 @@
             access,
             reading: kept,
             copies: Arc::new(Mutex::new(HashMap::new())),
+            databases: Arc::new(databases),
         })
     }
 
@@ -1671,20 +1681,38 @@
         if let Some(file) = self.file_of(object) {
             return (file.found.format == Format::Excel).then_some(Unedited::Excel);
         }
-        if self.attached.iter().any(|each| each.name == object.schema) {
-            return Some(Unedited::Attached);
-        }
-        self.files
+        self.attached
             .iter()
-            .any(|file| file.found.format.is_database() && file.found.name == object.schema)
-            .then_some(Unedited::DuckDb)
-    }
-
-    /// Runs `work` on the blocking pool with the session.
+            .any(|each| each.name == object.schema)
+            .then_some(Unedited::Attached)
+    }
+
+    /// Where `object` is a table or a view of one of the session's
+    /// `.duckdb` files, which of `databases` it is.
+    fn database_of(&self, object: &ObjectRef) -> Option<usize> {
+        self.databases
+            .iter()
+            .position(|held| held.name == object.schema)
+    }
+
+    /// Runs `work` on the blocking pool with the session, holding its
+    /// `.duckdb` files for reading.
     async fn run<T: Send + 'static>(
         &self,
         work: impl FnOnce(&duckdb::Connection) -> Result<T> + Send + 'static,
     ) -> Result<T> {
+        self.run_gated(true, work).await
+    }
+
+    /// Runs `work` as [`Conn::run`] does; `reading` says whether it holds
+    /// the `.duckdb` files for reading meanwhile. A save holds its own
+    /// alone instead, and reads no other.
+    async fn run_gated<T: Send + 'static>(
+        &self,
+        reading: bool,
+        work: impl FnOnce(&duckdb::Connection) -> Result<T> + Send + 'static,
+    ) -> Result<T> {
+        let databases = Arc::clone(&self.databases);
         let inner = Arc::clone(&self.inner);
         let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
         let given_up = Arc::new(AtomicBool::new(false));
@@ -1703,6 +1731,12 @@
             // Said to be running first, then asked whether it was given up
             // on, as the guard says it gives up first, then asks: one of
             // the two always sees the other. Given up on: nothing runs.
+            // Held first: a save may keep the call waiting, and a call
+            // given up on meanwhile is asked about after.
+            let _held = match reading {
+                true => database::reading(&connection, &databases),
+                false => Vec::new(),
+            };
             running.store(call, Ordering::SeqCst);
             if given_up.load(Ordering::SeqCst) {
                 running.store(0, Ordering::SeqCst);
@@ -1987,11 +2021,19 @@
             options: file.options.clone(),
         });
         let unedited = self.unedited(object);
+        let in_database = self.database_of(object).is_some();
         let object = object.clone();
         let attached = Arc::clone(&self.attached);
         self.run(move |connection| {
-            let columns = read_only(connection, |connection| {
-                columns(connection, &object, &attached)
+            let (columns, primary_key) = read_only(connection, |connection| {
+                let columns = columns(connection, &object, &attached)?;
+                // A `.duckdb` file's table finds its rows by its own key.
+                let primary_key = match in_database {
+                    true => database::table_key(connection, &object.schema, &object.name)?
+                        .unwrap_or_default(),
+                    false => Vec::new(),
+                };
+                Ok((columns, primary_key))
             })?;
             if columns.is_empty() {
                 return Err(Error::query(format!(
@@ -2001,6 +2043,7 @@
             }
             Ok(Structure {
                 columns,
+                primary_key,
                 file,
                 unedited,
                 ..Structure::default()
@@ -2073,7 +2116,28 @@
         let editing = self.editing(&object);
         let copies = Arc::clone(&self.copies);
         let reading = self.reading.clone();
+        let database = self.database_of(&object);
+        let writable = self.access == Access::Writable;
+        let databases = Arc::clone(&self.databases);
         self.run(move |connection| {
+            // A `.duckdb` file's table: read in its key's order, or, with
+            // none, where a save may write it, each row with its `rowid`.
+            let keyed = match database {
+                Some(_) => read_only(connection, |connection| {
+                    database::table_key(connection, &query.object.schema, &query.object.name)
+                })?,
+                None => None,
+            };
+            if let (Some(at), Some(key), true) = (database, &keyed, writable)
+                && key.is_empty()
+            {
+                let from = database::placed(connection, &query.object)?;
+                let generation = databases[at].generation();
+                return read_only(connection, |connection| {
+                    edit::page(connection, &query, &from, limit, generation)
+                });
+            }
+            let key = keyed.unwrap_or_default();
             let copied = match &editing {
                 Some(file) => {
                     let mut copies = copies.lock().map_err(|_| {
@@ -2098,12 +2162,13 @@
             };
             read_only(connection, |connection| {
                 if let Some(generation) = copied {
-                    return edit::page(connection, &query, limit, generation);
+                    let from = edit::Copy::object(&query.object.name);
+                    return edit::page(connection, &query, &from, limit, generation);
                 }
                 let catalog = columns(connection, &query.object, &attached)?;
                 let binary = binary_of(&catalog);
                 // A file has no key: rows come in the file's order.
-                let sql = Dialect::DuckDb.select_rows(&query, &[], &binary)?;
+                let sql = Dialect::DuckDb.select_rows(&query, &key, &binary)?;
                 let started = Instant::now();
                 let params = sql.params.iter().map(to_duckdb).collect();
                 let (mut columns, rows, has_more) = rows_of(connection, &sql.text, params, limit)?;
@@ -2112,7 +2177,7 @@
                     columns,
                     rows,
                     has_more,
-                    ordered_by_key: false,
+                    ordered_by_key: !key.is_empty(),
                     elapsed: started.elapsed(),
                     positions: None,
                 })
@@ -2297,6 +2362,16 @@
     /// Saves `changes` to the copy of the file their table reads, then to
     /// the file (see `edit::save`). A table no save writes back is refused.
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
+        // A `.duckdb` file's table is saved in place, the file held alone.
+        if let (Some(at), Access::Writable) = (self.database_of(&changes.object), self.access) {
+            let databases = Arc::clone(&self.databases);
+            let (changes, stop) = (changes.clone(), stop.clone());
+            return self
+                .run_gated(false, move |connection| {
+                    database::save(connection, &databases[at], &changes, &stop)
+                })
+                .await;
+        }
         let Some(file) = self.editing(&changes.object) else {
             return Err(Error::ReadOnly);
         };
diff --git a/crates/tabletist-db/src/duck/database.rs b/crates/tabletist-db/src/duck/database.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-db/src/duck/database.rs
@@ -0,0 +1,451 @@
+//! A `.duckdb` file's tables, edited in place. Every session attaches the
+//! file read-only, so other programs can still read it, and none can write
+//! it. A save takes the file alone: it attaches it to write, changes it in
+//! one transaction, and attaches it read-only again.
+//!
+//! Inside one app DuckDB's own lock keeps nothing apart: two sessions are
+//! one process, which may hold the file twice. So the sessions over a file
+//! share a gate. Each call holds it with the others; a save holds it alone,
+//! and after it every other session attaches the file again before it
+//! reads it, as the save left it.
+//!
+//! A table with a primary key is saved by it. One without is saved by
+//! DuckDB's `rowid`, which a save that deletes rows numbers again: a
+//! page's rows carry the number of the save they were read after
+//! ([`crate::Positions::copy`]), and a later save refuses them.
+
+use std::path::{Path, PathBuf};
+use std::sync::atomic::{AtomicI64, Ordering};
+use std::sync::{Arc, Mutex, OnceLock, PoisonError, RwLock, RwLockReadGuard};
+use std::time::Instant;
+
+use super::edit::{self, EDITS, POSITION, Stamp};
+use super::{literal, map_error, path_text, to_duckdb};
+use crate::write::Conflict;
+use crate::{ChangeSet, Dialect, Error, ObjectRef, Result, StopFlag, Unsaved, WriteOutcome};
+
+/// The column DuckDB numbers a table's rows by, which no `SELECT *` reads.
+const ROWID: &str = "rowid";
+
+/// What every session of this app over one `.duckdb` file shares.
+struct Shared {
+    /// Held by each call together, and by a save alone.
+    gate: RwLock<()>,
+    /// The number of the save that wrote the file last, from 1.
+    saves: AtomicI64,
+}
+
+/// The `.duckdb` file at `path`, as this app's sessions share it: by its
+/// own path, however a connection spells it.
+fn shared(path: &Path) -> Arc<Shared> {
+    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
+    static FILES: OnceLock<Mutex<std::collections::HashMap<PathBuf, Arc<Shared>>>> =
+        OnceLock::new();
+    let mut files = FILES
+        .get_or_init(Default::default)
+        .lock()
+        .unwrap_or_else(PoisonError::into_inner);
+    Arc::clone(files.entry(path).or_insert_with(|| {
+        Arc::new(Shared {
+            gate: RwLock::new(()),
+            saves: AtomicI64::new(1),
+        })
+    }))
+}
+
+/// A `.duckdb` file as a session attached it.
+pub(super) struct Held {
+    /// The schema it is attached as.
+    pub(super) name: String,
+    path: PathBuf,
+    shared: Arc<Shared>,
+    /// The save the session's attachment is of. 0 until the first call
+    /// attaches it again, under the gate: the session opened without it.
+    seen: AtomicI64,
+    /// The file as it was when the session last attached it.
+    stamp: Mutex<Option<Stamp>>,
+}
+
+impl Held {
+    pub(super) fn new(name: &str, path: &Path) -> Self {
+        Self {
+            name: name.to_owned(),
+            path: path.to_owned(),
+            shared: shared(path),
+            seen: AtomicI64::new(0),
+            stamp: Mutex::new(None),
+        }
+    }
+
+    /// The path the file is attached from, which sessions sort theirs by.
+    pub(super) fn path(&self) -> &Path {
+        &self.path
+    }
+
+    /// The save the session's attachment is of: what a page's rows found
+    /// by `rowid` are of.
+    pub(super) fn generation(&self) -> i64 {
+        self.seen.load(Ordering::SeqCst)
+    }
+}
+
+/// Attaches `held` again under its name: read-only, or to write.
+fn attach(connection: &duckdb::Connection, held: &Held, write: bool) -> Result<()> {
+    let name = Dialect::DuckDb.ident(&held.name);
+    let path = literal(&path_text(&held.path)?);
+    let mode = if write { "" } else { " (READ_ONLY)" };
+    connection
+        .execute_batch(&format!(
+            "DETACH DATABASE IF EXISTS {name}; ATTACH {path} AS {name}{mode}"
+        ))
+        .map_err(map_error)?;
+    if !write {
+        *held.stamp.lock().unwrap_or_else(PoisonError::into_inner) = Stamp::of(&held.path).ok();
+    }
+    Ok(())
+}
+
+/// Holds each of `databases`, in their order (their paths'), for reading
+/// while a call runs: no save writes one meanwhile. One another session
+/// saved since this one attached it is attached again first. One that
+/// cannot be is left as it is, and tried again on the next call.
+pub(super) fn reading<'a>(
+    connection: &duckdb::Connection,
+    databases: &'a [Held],
+) -> Vec<RwLockReadGuard<'a, ()>> {
+    databases
+        .iter()
+        .map(|held| {
+            let guard = held
+                .shared
+                .gate
+                .read()
+                .unwrap_or_else(PoisonError::into_inner);
+            let saves = held.shared.saves.load(Ordering::SeqCst);
+            if held.seen.load(Ordering::SeqCst) != saves && attach(connection, held, false).is_ok()
+            {
+                held.seen.store(saves, Ordering::SeqCst);
+            }
+            guard
+        })
+        .collect()
+}
+
+/// The columns of the primary key of the table `table` of the `.duckdb`
+/// file attached as `database`, in its order: none where it has none.
+/// `None` where `table` is not a table there: a view.
+pub(super) fn table_key(
+    connection: &duckdb::Connection,
+    database: &str,
+    table: &str,
+) -> Result<Option<Vec<String>>> {
+    let tables: i64 = connection
+        .query_row(
+            "SELECT count(*) FROM duckdb_tables() \
+             WHERE database_name = ? AND schema_name = 'main' AND table_name = ?",
+            [database, table],
+            |row| row.get(0),
+        )
+        .map_err(map_error)?;
+    if tables == 0 {
+        return Ok(None);
+    }
+    primary_key(connection, database, table).map(Some)
+}
+
+/// The columns of the primary key of the table `table` of the `.duckdb`
+/// file attached as `database`, in its order: none where it has none.
+fn primary_key(
+    connection: &duckdb::Connection,
+    database: &str,
+    table: &str,
+) -> Result<Vec<String>> {
+    let found: Option<duckdb::types::Value> = connection
+        .query_row(
+            "SELECT constraint_column_names FROM duckdb_constraints() \
+             WHERE database_name = ? AND schema_name = 'main' AND table_name = ? \
+             AND constraint_type = 'PRIMARY KEY'",
+            [database, table],
+            |row| row.get(0),
+        )
+        .map(Some)
+        .or_else(|error| match error {
+            duckdb::Error::QueryReturnedNoRows => Ok(None),
+            error => Err(map_error(error)),
+        })?;
+    Ok(match found {
+        Some(duckdb::types::Value::List(names)) => names
+            .into_iter()
+            .filter_map(|name| match name {
+                duckdb::types::Value::Text(name) => Some(name),
+                _ => None,
+            })
+            .collect(),
+        _ => Vec::new(),
+    })
+}
+
+/// Where a page of `object`, a `.duckdb` file's table with no primary
+/// key, is read from: a view of it in the edits database that reads each
+/// row's `rowid` as its place. Made outside the page's read-only
+/// transaction, which may make nothing.
+pub(super) fn placed(connection: &duckdb::Connection, object: &ObjectRef) -> Result<ObjectRef> {
+    let from = ObjectRef {
+        schema: EDITS.to_owned(),
+        name: format!("{}.{}", object.schema, object.name),
+    };
+    let dialect = Dialect::DuckDb;
+    connection
+        .execute_batch(&format!(
+            "CREATE OR REPLACE VIEW {} AS SELECT {ROWID} AS {}, * FROM {}",
+            dialect.qualified(&from),
+            dialect.quote_ident(POSITION),
+            dialect.qualified(object),
+        ))
+        .map_err(map_error)?;
+    Ok(from)
+}
+
+/// Saves `changes` to `held`'s table `changes.object` in place, alone:
+/// every other session waits, and attaches the file again after. A save
+/// the file refuses writes nothing, and says why.
+pub(super) fn save(
+    connection: &duckdb::Connection,
+    held: &Held,
+    changes: &ChangeSet,
+    stop: &StopFlag,
+) -> Result<WriteOutcome> {
+    let _alone = held
+        .shared
+        .gate
+        .write()
+        .unwrap_or_else(PoisonError::into_inner);
+    let started = Instant::now();
+    let label = crate::files::file_name(&held.path);
+    let unsaved = |why: Unsaved, message: String| {
+        let mut error = Error::query(message);
+        if let Error::Query { named, .. } = &mut error {
+            named.unsaved = Some(why);
+        }
+        error
+    };
+    // Attached again first where another session saved since.
+    let saves = held.shared.saves.load(Ordering::SeqCst);
+    if held.seen.load(Ordering::SeqCst) != saves {
+        attach(connection, held, false)?;
+        held.seen.store(saves, Ordering::SeqCst);
+    }
+    let key = primary_key(connection, &held.name, &changes.object.name)?;
+    // Rows found by `rowid` are of the file as one save left it: another
+    // save since may have numbered them again.
+    let changes = match key.is_empty() {
+        true => edit::own_keys(changes, saves, ROWID).ok_or_else(|| {
+            unsaved(
+                Unsaved::Reread(label.clone()),
+                format!("{label} was read again since these rows were"),
+            )
+        })?,
+        false => changes.clone(),
+    };
+    let was = *held.stamp.lock().unwrap_or_else(PoisonError::into_inner);
+    if was.is_none() || Stamp::of(&held.path).ok() != was {
+        return Err(unsaved(
+            Unsaved::Changed(label.clone()),
+            format!("{label} changed on disk since it was opened"),
+        ));
+    }
+    if edit::protected(&held.path) {
+        return Err(unsaved(
+            Unsaved::Protected(label.clone()),
+            format!("{label} is read-only"),
+        ));
+    }
+    // Read-only again whatever comes of it.
+    let back = || {
+        if attach(connection, held, false).is_ok() {
+            held.seen.store(saves, Ordering::SeqCst);
+        }
+    };
+    if let Err(error) = attach(connection, held, true) {
+        back();
+        let said = super::plain(&error);
+        return Err(match held_by(&said, std::process::id()) {
+            Some(Holder::Other) => unsaved(
+                Unsaved::Busy(label.clone()),
+                format!("{label} is open in another program"),
+            ),
+            Some(Holder::Here) => unsaved(
+                Unsaved::OpenHere(label.clone()),
+                format!("{label} is open in another session of this app"),
+            ),
+            None => unsaved(
+                Unsaved::Failed(label.clone(), said.clone()),
+                format!("{label} could not be written: {said}"),
+            ),
+        });
+    }
+    let done = connection
+        .execute_batch("BEGIN TRANSACTION")
+        .map_err(map_error)
+        .and_then(|()| changed(connection, &changes, stop));
+    let outcome = match done {
+        Ok(outcome @ WriteOutcome::Written { .. }) => {
+            match connection.execute_batch("COMMIT").map_err(map_error) {
+                Ok(()) => outcome,
+                Err(error) => {
+                    let _ = connection.execute_batch("ROLLBACK");
+                    back();
+                    return Err(error);
+                }
+            }
+        }
+        other => {
+            let _ = connection.execute_batch("ROLLBACK");
+            back();
+            return other;
+        }
+    };
+    // Written: every session, this one too, reads the file as it is now.
+    let next = held.shared.saves.fetch_add(1, Ordering::SeqCst) + 1;
+    if attach(connection, held, false).is_ok() {
+        held.seen.store(next, Ordering::SeqCst);
+    }
+    Ok(match outcome {
+        WriteOutcome::Written {
+            inserted,
+            rows,
+            gone,
+            ..
+        } => WriteOutcome::Written {
+            inserted,
+            rows,
+            gone,
+            elapsed: started.elapsed(),
+        },
+        other => other,
+    })
+}
+
+/// Who holds a file DuckDB could not open to write.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+enum Holder {
+    Other,
+    /// This app, process `pid`: on Windows another of its sessions keeps
+    /// the file from being opened to write.
+    Here,
+}
+
+/// Who holds the file, from DuckDB's words `said` where they say another
+/// process has it: its lock (`Could not set lock on file …: Conflicting
+/// lock is held in … (PID 4242)`), or on Windows its handle (`… is being
+/// used by another process. File is already open in … (PID 4242)`). This
+/// app is `pid`. `None` where they say neither.
+fn held_by(said: &str, pid: u32) -> Option<Holder> {
+    let held =
+        said.contains("Could not set lock") || said.contains("being used by another process");
+    if !held {
+        return None;
+    }
+    Some(match said.contains(&format!("(PID {pid})")) {
+        true => Holder::Here,
+        false => Holder::Other,
+    })
+}
+
+/// Changes the table as `changes` say, in the transaction `save` began,
+/// and reads each changed row back by its key.
+fn changed(
+    connection: &duckdb::Connection,
+    changes: &ChangeSet,
+    stop: &StopFlag,
+) -> Result<WriteOutcome> {
+    let object = &changes.object;
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
+        let done = dialect.insert_row(object, row).and_then(|statement| {
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
+            .update_row(object, change)
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
+            .delete_row(object, row)
+            .and_then(|deletion| run(&deletion.sql))
+        {
+            Ok(0) => gone.push(delete),
+            Ok(_) => {}
+            Err(error) => return Ok(WriteOutcome::FailedDelete { delete, error }),
+        }
+    }
+    stopped()?;
+    let mut rows = Vec::with_capacity(changes.rows.len());
+    for change in &changes.rows {
+        let select = dialect.select_row(object, &change.key, false);
+        let params = select.params.iter().map(to_duckdb).collect();
+        let (_, mut found, _) = super::rows_of(connection, &select.text, params, 2)?;
+        if found.len() != 1 {
+            return Err(Error::query("a changed row was not read back"));
+        }
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
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn a_file_another_process_holds_is_told_from_one_this_app_holds() {
+        let unix = "IO Error: Could not set lock on file \"/data/bookshop.duckdb\": Conflicting \
+                    lock is held in /usr/bin/duckdb (PID 4242) by user reader.";
+        assert_eq!(held_by(unix, 7), Some(Holder::Other));
+        assert_eq!(held_by(unix, 4242), Some(Holder::Here));
+        let windows = "IO Error: Cannot open file \"C:\\Data\\bookshop.duckdb\": The process \
+                       cannot access the file because it is being used by another process.\r\n\n\
+                       File is already open in \nC:\\Program Files\\Tabletist\\tabletist.exe \
+                       (PID 6212)";
+        assert_eq!(held_by(windows, 6212), Some(Holder::Here));
+        assert_eq!(held_by(windows, 1), Some(Holder::Other));
+        assert_eq!(held_by("IO Error: No space left on device", 1), None);
+    }
+}
diff --git a/crates/tabletist-db/src/duck/edit.rs b/crates/tabletist-db/src/duck/edit.rs
--- a/crates/tabletist-db/src/duck/edit.rs
+++ b/crates/tabletist-db/src/duck/edit.rs
@@ -64,7 +64,7 @@
         modified: None,
     };
 
-    fn of(path: &Path) -> std::io::Result<Self> {
+    pub(super) fn of(path: &Path) -> std::io::Result<Self> {
         std::fs::metadata(path).map(|meta| Self::from(&meta))
     }
 
@@ -149,16 +149,20 @@
     }
 }
 
-/// A page of a file's table, read from its copy, each row with its place:
-/// in the file's order, or that after the page's own sort.
+/// A page of a table, read from `from`, which holds each row's place
+/// beside it (a file's copy, or a view of a `.duckdb` file's table): in
+/// the file's order, or that after the page's own sort.
 pub(super) fn page(
     connection: &duckdb::Connection,
     query: &RowQuery,
+    from: &ObjectRef,
     limit: usize,
     generation: i64,
 ) -> Result<RowPage> {
-    let mut from = query.clone();
-    from.object = Copy::object(&query.object.name);
+    let from = RowQuery {
+        object: from.clone(),
+        ..query.clone()
+    };
     let catalog = super::columns(connection, &from.object, &[])?;
     let binary = super::binary_of(&catalog);
     let sql = Dialect::DuckDb.select_rows(&from, &[POSITION.to_owned()], &binary)?;
@@ -750,7 +754,7 @@
     let _one_at_a_time = lock.lock().unwrap_or_else(PoisonError::into_inner);
     // Rows found by their places in another copy may not be where they
     // were: the file was read again, or a save numbered them again.
-    let Some(changes) = own_keys(changes, copy.generation) else {
+    let Some(changes) = own_keys(changes, copy.generation, POSITION) else {
         return Err(unsaved(
             Unsaved::Reread(label.clone()),
             format!("{label} was read again since these rows were"),
@@ -766,12 +770,7 @@
     // would replace it all the same where its folder is writable.
     // Its permissions say so, or this user may not write it: opened to
     // write, and not truncated, it says which.
-    let protected = std::fs::metadata(path).is_ok_and(|meta| meta.permissions().readonly())
-        || std::fs::OpenOptions::new()
-            .write(true)
-            .open(path)
-            .is_err_and(|error| error.kind() == std::io::ErrorKind::PermissionDenied);
-    if protected {
+    if protected(path) {
         return Err(unsaved(
             Unsaved::Protected(label.clone()),
             format!("{label} is read-only"),
@@ -945,14 +944,25 @@
     }
 }
 
-/// `changes` with each row's key as the copy finds it, its place alone.
+/// Whether the file at `path` is one its owner made read-only, or this
+/// user may not write: its permissions say so, or, opened to write and not
+/// truncated, it says which.
+pub(super) fn protected(path: &Path) -> bool {
+    std::fs::metadata(path).is_ok_and(|meta| meta.permissions().readonly())
+        || std::fs::OpenOptions::new()
+            .write(true)
+            .open(path)
+            .is_err_and(|error| error.kind() == std::io::ErrorKind::PermissionDenied)
+}
+
+/// `changes` with each row's key as `column` finds it, its place alone.
 /// `None` where a key is not a place in the copy `generation`.
-fn own_keys(changes: &ChangeSet, generation: i64) -> Option<ChangeSet> {
+pub(super) fn own_keys(changes: &ChangeSet, generation: i64, column: &str) -> Option<ChangeSet> {
     let own = |key: &[(String, Value)]| match key {
         [(place, Value::Int(at)), (of, Value::Int(copy))]
             if place == POSITION && of == FILE_COPY && *copy == generation =>
         {
-            Some(vec![(POSITION.to_owned(), Value::Int(*at))])
+            Some(vec![(column.to_owned(), Value::Int(*at))])
         }
         _ => None,
     };
diff --git a/crates/tabletist-db/src/error.rs b/crates/tabletist-db/src/error.rs
--- a/crates/tabletist-db/src/error.rs
+++ b/crates/tabletist-db/src/error.rs
@@ -41,6 +41,13 @@
     /// It was read again since the rows to save were read: they may not
     /// be where they were.
     Reread(String),
+    /// Another program has it open: a `.duckdb` file it holds can't be
+    /// opened to write.
+    Busy(String),
+    /// Another session of this app has it open, which on Windows keeps a
+    /// `.duckdb` file from being opened to write: an export, an agent, a
+    /// second window.
+    OpenHere(String),
     /// It could not be written, for another reason: the file's name and
     /// the system's words.
     Failed(String, String),
diff --git a/src/ui/cell_editor.rs b/src/ui/cell_editor.rs
--- a/src/ui/cell_editor.rs
+++ b/src/ui/cell_editor.rs
@@ -1530,9 +1530,6 @@
     match lock {
         Lock::ReadOnly => say(msgid("This connection opens read-only")),
         Lock::Unedited(tabletist_db::Unedited::Excel) => say(msgid("Excel files open read-only")),
-        Lock::Unedited(tabletist_db::Unedited::DuckDb) => {
-            say(msgid(".duckdb files open read-only"))
-        }
         Lock::Unedited(tabletist_db::Unedited::Attached) => {
             say(msgid("Attached connections open read-only"))
         }
diff --git a/src/ui/pending_bar.rs b/src/ui/pending_bar.rs
--- a/src/ui/pending_bar.rs
+++ b/src/ui/pending_bar.rs
@@ -337,6 +337,28 @@
                  now; your {n} change stays pending.",
                 "{file} was read again since you changed these rows. Reload to see it as it is \
                  now; your {n} changes stay pending.",
+                n,
+            ),
+            file,
+        ),
+        tabletist_db::Unsaved::Busy(file) => (
+            ngettext(
+                locale,
+                "{file} couldn't be saved: another program has it open. Your {n} change stays \
+                 pending.",
+                "{file} couldn't be saved: another program has it open. Your {n} changes stay \
+                 pending.",
+                n,
+            ),
+            file,
+        ),
+        tabletist_db::Unsaved::OpenHere(file) => (
+            ngettext(
+                locale,
+                "{file} couldn't be saved: another tab, export or agent of Tabletist has it open. \
+                 Your {n} change stays pending.",
+                "{file} couldn't be saved: another tab, export or agent of Tabletist has it open. \
+                 Your {n} changes stay pending.",
                 n,
             ),
             file,
````

- [ ] **Step 4: Make the catalogs**

Run `assets/i18n/update.sh`, then the script of Task 2 Step 4 with `--present`, which fills only the messages a catalog has. Then run `assets/i18n/update.sh` again.

- [ ] **Step 5: Run the full check**

Expected: every step passes; the tests 5,794 passed, 0 failed.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-db src/ui/pending_bar.rs src/ui/cell_editor.rs src/ui/files_edit_tests.rs assets/i18n
git commit -m "Save a .duckdb file's tables in place, the file held alone while a save runs"
```

### Task 2: The notes say a `.duckdb` file changes in place

**Files:** `src/ui/connect_dialog/mod.rs`, `src/ui/connect_dialog/terminal.rs`, `src/ui/files_tests.rs`, `assets/i18n/*`, `docs/_guide/editing-data.md`, `docs/_reference/connections.md`.

- [ ] **Step 1: Write the failing test**

The read-only box's note when it's off, in both looks: beside a file a save rewrites, and with only a `.duckdb` file. A `.duckdb` file is never the one the note names, however large.

````diff
diff --git a/src/ui/files_tests.rs b/src/ui/files_tests.rs
--- a/src/ui/files_tests.rs
+++ b/src/ui/files_tests.rs
@@ -555,6 +555,51 @@
 }
 
 #[test]
+fn a_files_note_says_a_duckdb_file_is_changed_in_place() {
+    for look in looks() {
+        let mut harness = new_files(look);
+        form_mut(&mut harness).read_only = Some(false);
+        let duckdb = Found {
+            path: PathBuf::from("/data/bookshop-exports/bookshop.duckdb"),
+            format: Format::DuckDb,
+            // Never the file the note names, however large.
+            size: 9_000_000_000,
+            name: "bookshop".into(),
+        };
+        // Beside files a save writes again.
+        let mut scan = exports("/data/bookshop-exports");
+        scan.files.push(duckdb.clone());
+        form_mut(&mut harness).files_found = FilesFound::Read(scan);
+        let both = match look.terminal {
+            true => {
+                "· changes wait for :w, which rewrites the whole file (orders.csv 1.6 gb); \
+                     .duckdb files change in place"
+            }
+            false => {
+                "Editing is on. Changes are kept until you save; saving rewrites the whole \
+                      file, so orders.csv (1.6 GB) is written again in full; .duckdb files are \
+                      changed in place."
+            }
+        };
+        assert!(says(&mut harness, both), "{}", look.name);
+        // Alone.
+        form_mut(&mut harness).files_found = FilesFound::Read(Scan {
+            files: vec![duckdb],
+            missing: Vec::new(),
+            unreadable: Vec::new(),
+        });
+        let alone = match look.terminal {
+            true => "· changes wait for :w; .duckdb files change in place, in a transaction",
+            false => {
+                "Editing is on. Changes are kept until you save; .duckdb files are changed \
+                      in place, in a transaction."
+            }
+        };
+        assert!(says(&mut harness, alone), "{}", look.name);
+    }
+}
+
+#[test]
 fn another_type_s_box_left_off_does_not_make_files_editable() {
     let mut harness = new_files(Look::macos());
     harness.click("PostgreSQL");
````

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib -- files_tests`

```text
---- ui::files_tests::a_files_note_says_a_duckdb_file_is_changed_in_place stdout ----
thread 'ui::files_tests::a_files_note_says_a_duckdb_file_is_changed_in_place' (3256827) panicked at src/ui/files_tests.rs:584:9:
test result: FAILED. 25 passed; 1 failed; 0 ignored; 0 measured; 3161 filtered out; finished in 0.12s
error: test failed, to rerun pass `-p tabletist --lib`
```

- [ ] **Step 3: Say it, and write the guide**

````diff
diff --git a/docs/_guide/editing-data.md b/docs/_guide/editing-data.md
--- a/docs/_guide/editing-data.md
+++ b/docs/_guide/editing-data.md
@@ -532,8 +532,9 @@
   save would give each the missing ones as `null`. So does a file that
   can't be read whole, such as one with a bad value far past its first
   page: its rows still show a page at a time, as with the box on.
-- Excel files always open read-only, and so do `.duckdb` files for now.
-  Their table's header says so.
+- Excel files always open read-only. Their table's header says so.
+- A `.duckdb` file isn't written again: its tables are changed in place
+  (see [Edit a .duckdb file](#edit-a-duckdb-file)).
 - An attached connection's tables open read-only here, even when the
   files are editable.
 
@@ -582,15 +583,46 @@
 
 The SQL editor reads a Files connection, whether it opens read-only or
 not: a statement that would write is refused, and its card says to edit
-the file's table instead.
+the file's table instead. That goes for a `.duckdb` file too.
+
+### Edit a .duckdb file
+
+A `.duckdb` file's tables are edited like a file's: change values, add
+rows and delete rows, then save. Nothing is read into memory first, and
+a save doesn't write the whole file again: it changes the tables in the
+file, in one transaction. A change that fails undoes the others, and
+nothing is written.
+
+A table with a primary key finds its rows by it, as a database's table
+does, and its page can be edited again right after a save. A table
+without one finds a row by DuckDB's `rowid`, its row's place in the
+file. A save that deletes rows moves those after them, so after every
+save its page is read again, and until it is an edit says "Rows moved in
+the file since this page was read. Reload to edit." A view in the file
+isn't edited.
+
+While the connection is open, Tabletist holds the file read-only:
+another program can read it, but not write it. A save opens it to write
+for as long as the save takes. If another program has the file open
+then, nothing is written: "bookshop.duckdb couldn't be saved: another
+program has it open. Your 3 changes stay pending." Close it there and
+save again. A file another program replaced since it was opened, or one
+made read-only, isn't written either, as with any file.
+
+Other tabs, an export or an agent reading the same file wait while a
+save runs, then read the file as the save left it. On Windows they keep
+the save from opening the file at all while they have it open, as
+another program does: "bookshop.duckdb couldn't be saved: another tab,
+export or agent of Tabletist has it open. Your 1 change stays pending."
+Let the export finish, or close the other window, and save again.
 
 ## What cannot be edited
 
 A cell that cannot be edited says why when you try. The common reasons:
 
 - The connection opens read-only.
-- The file is an Excel or a `.duckdb` file, or its table is an attached
-  connection's (see [Edit files](#edit-files)).
+- The file is an Excel file, or its table is an attached connection's
+  (see [Edit files](#edit-files)).
 - The object is a view.
 - The table has no primary key or unique index, so a row cannot be
   targeted safely.
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -171,7 +171,14 @@
   are kept until you save; saving rewrites the whole file, so orders.csv
   (1.6 GB) is written again in full." The Omarchy look says it on one
   line in the warning colour: `[ ] editing on · changes wait for :w,
-  which rewrites the whole file (orders.csv 1.6 gb)`. Agents can query
+  which rewrites the whole file (orders.csv 1.6 gb)`. A `.duckdb` file
+  is never written again in full: its tables are changed in place
+  ([Edit a .duckdb file]({% link _guide/editing-data.md %}#edit-a-duckdb-file)),
+  so with one the note adds "; .duckdb files are changed in place." (on
+  Omarchy `; .duckdb files change in place`), and with only those it
+  says "Editing is on. Changes are kept until you save; .duckdb files
+  are changed in place, in a transaction." (`[ ] editing on · changes
+  wait for :w; .duckdb files change in place, in a transaction`). Agents can query
   the files, never change them, and cannot
   hide tables or mask columns on a Files connection. Its **Agents**
   section is closed, and says the access on one line. Agents reach an
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -91,11 +91,37 @@
     ))
 }
 
+/// Whether the form's files read so far hold a `.duckdb` file a save
+/// would change in place: one not skipped.
+fn changes_in_place(form: &ConnectionForm) -> bool {
+    let FilesFound::Read(scan) = &form.files_found else {
+        return false;
+    };
+    scan.files
+        .iter()
+        .any(|file| file.format.is_database() && !form.files_csv.skips(&file.path))
+}
+
 /// What a Files connection's read-only box says while it is off: that a
-/// save writes a whole file, and the largest it would.
+/// save writes a whole file, and the largest it would, and that a
+/// `.duckdb` file is changed in place.
 fn files_editing_note(form: &ConnectionForm, locale: Locale) -> String {
-    match largest_rewritten(form) {
-        Some((file, size)) => crate::i18n::fill(
+    match (largest_rewritten(form), changes_in_place(form)) {
+        (Some((file, size)), true) => crate::i18n::fill(
+            &gettext(
+                locale,
+                "Editing is on. Changes are kept until you save; saving rewrites the whole file, \
+                 so {file} ({size}) is written again in full; .duckdb files are changed in place.",
+            ),
+            &[("{file}", &file), ("{size}", &size)],
+        ),
+        (None, true) => gettext(
+            locale,
+            "Editing is on. Changes are kept until you save; .duckdb files are changed in place, \
+             in a transaction.",
+        )
+        .into_owned(),
+        (Some((file, size)), false) => crate::i18n::fill(
             &gettext(
                 locale,
                 "Editing is on. Changes are kept until you save; saving rewrites the whole file, \
@@ -103,7 +129,7 @@
             ),
             &[("{file}", &file), ("{size}", &size)],
         ),
-        None => gettext(
+        (None, false) => gettext(
             locale,
             "Editing is on. Changes are kept until you save; saving rewrites the whole file.",
         )
diff --git a/src/ui/connect_dialog/terminal.rs b/src/ui/connect_dialog/terminal.rs
--- a/src/ui/connect_dialog/terminal.rs
+++ b/src/ui/connect_dialog/terminal.rs
@@ -176,11 +176,34 @@
 }
 
 /// What a Files connection's read-only row says while it edits: that a
-/// save writes a whole file, and the largest it would. `key` is the grid's
-/// Save changes, as the look writes it.
+/// save writes a whole file, and the largest it would, and that a
+/// `.duckdb` file changes in place. `key` is the grid's Save changes, as
+/// the look writes it.
 fn terminal_editing_note(form: &ConnectionForm, key: &str, locale: crate::i18n::Locale) -> String {
-    match super::largest_rewritten(form) {
-        Some((file, size)) => crate::i18n::fill(
+    match (
+        super::largest_rewritten(form),
+        super::changes_in_place(form),
+    ) {
+        (Some((file, size)), true) => crate::i18n::fill(
+            &gettext(
+                locale,
+                "changes wait for {key}, which rewrites the whole file ({file} {size}); .duckdb \
+                 files change in place",
+            ),
+            &[
+                ("{key}", key),
+                ("{file}", &file),
+                ("{size}", &size.to_lowercase()),
+            ],
+        ),
+        (None, true) => crate::i18n::fill(
+            &gettext(
+                locale,
+                "changes wait for {key}; .duckdb files change in place, in a transaction",
+            ),
+            &[("{key}", key)],
+        ),
+        (Some((file, size)), false) => crate::i18n::fill(
             &gettext(
                 locale,
                 "changes wait for {key}, which rewrites the whole file ({file} {size})",
@@ -192,7 +215,7 @@
                 ("{size}", &size.to_lowercase()),
             ],
         ),
-        None => crate::i18n::fill(
+        (None, false) => crate::i18n::fill(
             &gettext(
                 locale,
                 "changes wait for {key}, which rewrites the whole file",
````

- [ ] **Step 4: The catalogs**

Run `assets/i18n/update.sh`, then this script, then `assets/i18n/update.sh` again. It fills Russian and Brazilian Portuguese; the Brazilian Portuguese still needs a native speaker's review.

```python
# Fills 4b's messages in ru.po and pt-BR.po. A plural's value is a list.
import re, sys
PRESENT='--present' in sys.argv
EM="Editing is on. Changes are kept until you save; saving rewrites the whole file, so {file} ({size}) is written again in full; .duckdb files are changed in place."
EO="Editing is on. Changes are kept until you save; .duckdb files are changed in place, in a transaction."
TM="changes wait for {key}, which rewrites the whole file ({file} {size}); .duckdb files change in place"
TO="changes wait for {key}; .duckdb files change in place, in a transaction"
BU="{file} couldn't be saved: another program has it open. Your {n} change stays pending."
OH="{file} couldn't be saved: another tab, export or agent of Tabletist has it open. Your {n} change stays pending."
T={
'assets/i18n/ru.po':{
EM:"Изменение включено. Изменения хранятся до сохранения; сохранение перезаписывает файл целиком, так что {file} ({size}) записывается заново полностью; файлы .duckdb изменяются на месте.",
EO:"Изменение включено. Изменения хранятся до сохранения; файлы .duckdb изменяются на месте, в одной транзакции.",
TM:"изменения ждут {key}, которое перезаписывает файл целиком ({file} {size}); файлы .duckdb изменяются на месте",
TO:"изменения ждут {key}; файлы .duckdb изменяются на месте, в одной транзакции",
BU:["Не удалось сохранить {file}: он открыт в другой программе. Ваше {n} изменение остаётся в ожидании.",
    "Не удалось сохранить {file}: он открыт в другой программе. Ваши {n} изменения остаются в ожидании.",
    "Не удалось сохранить {file}: он открыт в другой программе. Ваши {n} изменений остаются в ожидании."],
OH:["Не удалось сохранить {file}: он открыт в другой вкладке, выгрузке или агенте Tabletist. Ваше {n} изменение остаётся в ожидании.",
    "Не удалось сохранить {file}: он открыт в другой вкладке, выгрузке или агенте Tabletist. Ваши {n} изменения остаются в ожидании.",
    "Не удалось сохранить {file}: он открыт в другой вкладке, выгрузке или агенте Tabletist. Ваши {n} изменений остаются в ожидании."],
},
'assets/i18n/pt-BR.po':{
EM:"A edição está ativada. As alterações ficam guardadas até você salvar; salvar regrava o arquivo inteiro, então {file} ({size}) é gravado de novo por completo; os arquivos .duckdb são alterados no lugar.",
EO:"A edição está ativada. As alterações ficam guardadas até você salvar; os arquivos .duckdb são alterados no lugar, em uma transação.",
TM:"as alterações esperam {key}, que regrava o arquivo inteiro ({file} {size}); os arquivos .duckdb mudam no lugar",
TO:"as alterações esperam {key}; os arquivos .duckdb mudam no lugar, em uma transação",
BU:["Não foi possível salvar {file}: outro programa está com ele aberto. Sua {n} alteração continua pendente.",
    "Não foi possível salvar {file}: outro programa está com ele aberto. Suas {n} alterações continuam pendentes."],
OH:["Não foi possível salvar {file}: outra aba, exportação ou agente do Tabletist está com ele aberto. Sua {n} alteração continua pendente.",
    "Não foi possível salvar {file}: outra aba, exportação ou agente do Tabletist está com ele aberto. Suas {n} alterações continuam pendentes."],
}}
def esc(t): return t.replace('\\','\\\\').replace('"','\\"')
for p,t in T.items():
    blocks=open(p).read().split('\n\n'); done=set()
    for i,b in enumerate(blocks):
        if b.lstrip().startswith('#~'): continue
        m=re.search(r'^msgid ((?:".*"\n)+)', b, re.M)
        if not m: continue
        mid=''.join(re.findall(r'"(.*)"', m.group(1)))
        if mid not in t: continue
        v=t[mid]
        if isinstance(v,list):
            for k,form in enumerate(v):
                b,n=re.subn(r'^msgstr\[%d\] ".*"(\n".*")*$'%k, 'msgstr[%d] "%s"'%(k,esc(form)), b, flags=re.M); assert n==1,(mid,k)
        else:
            b,n=re.subn(r'^msgstr ".*"(\n".*")*$', 'msgstr "%s"'%esc(v), b, flags=re.M); assert n==1,mid
        b=re.sub(r'^#, fuzzy\n', '', b, flags=re.M)
        blocks[i]=b; done.add(mid)
    assert PRESENT or done==set(t),(p,set(t)-done)
    open(p,'w').write('\n\n'.join(blocks))
print('ok')
```

- [ ] **Step 5: Run the full check**

Expected: every step passes; the tests 5,795 passed, 0 failed.

- [ ] **Step 6: Commit**

```bash
git add src/ui/connect_dialog src/ui/files_tests.rs assets/i18n docs
git commit -m "Say in the read-only box's note that a .duckdb file is changed in place"
```

## Not done in this plan

- **The hand test, in both looks,** on a real Files connection over the Bookshop's exports:
  - turn the box off and read both notes;
  - edit, add and delete rows of `bookshop.order_items` and `bookshop.book_covers`, save, and read the file in another program;
  - open the file read-only in another program, save, and read the refusal;
  - compare with the boards `MacFilesDuckdb` and `OmarchyFilesDuckdb` (canvas version 226).
- **Windows:** CI runs `on_windows_a_file_another_session_has_open_is_not_written` and the child-process hold. Nothing was run on Windows here.
- **The Brazilian Portuguese** of the new messages needs a native speaker's review.
- **Not handled:** a view in a `.duckdb` file stays "Views cannot be edited". Structure changes (`ALTER`) and the SQL editor's writes stay out, as in 4a.
