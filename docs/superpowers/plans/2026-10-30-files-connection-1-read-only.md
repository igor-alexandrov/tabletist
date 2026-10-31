# Files connections, run 1: read-only end to end. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A fourth connection type, Files, that opens a folder or a list of CSV, TSV, Parquet, JSON, NDJSON, Excel and `.duckdb` files as read-only tables, browsed, described and queried like any other connection's.

**Architecture:** `tabletist-db` gains `Driver::Files` and `Dialect::DuckDb`, a `files` module (what a folder or a list holds, and the name each file becomes) and a DuckDB adapter (`duck.rs`): an in-memory DuckDB in the app's process with a view per file and each `.duckdb` file attached read-only as a schema, fenced by DuckDB's own locked configuration (it may open the connection's files and nothing else) and by a read-only transaction around every read. The app saves the source in `connections.toml` (version 7), edits it in the connection dialog's new Source group (both looks, a shared file list component, new form keys on Omarchy), and shows Files and SQLite connections' files in the connections list.

**Tech Stack:** Rust, egui (crmne fork), DuckDB 2.0 through the `duckdb` crate 1.20000.1 (bundled, with its Parquet and JSON readers), rfd for the pickers.

**Spec:** `docs/superpowers/specs/2026-10-30-files-connection-design.md`. **Base:** `d1ed10aa` (main, after v0.4.1).

---

## Run 1 of four

This plan is run 1: read-only end to end. Run 2 (Reading CSV and its overrides, the background check, the partial banner and its fixes, the macOS "not allowed" state and bookmarks, Omarchy path completions), run 3 (join with saved connections) and run 4 (editing files) get plans of their own. Each run is its own pull request (decided with Igor on 2026-10-30, against the prompt's one).

## Decided with Igor

| Question | Decision |
| --- | --- |
| One pull request | Four runs, a pull request each |
| Agents' Tables and Masked columns on a Files connection, where no catalog or plan can hold a statement to them | Off or Read, rows and timeout only. A rule from the person's settings or the administrator's policy that hides or masks anything on a Files connection refuses every agent read, with `Refusal::NotKeptOnFiles` |
| `connections.toml`'s form for Files (the prompt keys entries by name, with `type = "files"`) | The file's own: `[[connections]]`, `driver = "files"`, a `[connections.source]` table with `folder` or `files`; version 7, written only when a Files connection is saved |

## What the design asks, and what is built

| The design or the prompt | Built | Why |
| --- | --- | --- |
| Reading CSV and Join with saved connections, collapsed groups | Not shown | Runs 2 and 3; a group that cannot open would promise what is not built |
| Test's background check of whole files, the macOS "not allowed" state, security-scoped bookmarks, Omarchy's path completions (`ctrl+n/p`), the partial banner's `t`/`e`/`s` and Reset's `R` | Not built | Run 2 |
| The URL tab's tooltip "Files connections have no URL. Paste a folder or file path into Source." | "Files connections have no URL. Choose a folder or files under Source." | The macOS Source is a read-only box with **Choose…**, which takes no paste |
| Omarchy's read-only line `[x] files open read-only · turn off to edit; saving rewrites the whole file` | `[x] files open read-only` | Editing is run 4 |
| An `.xlsx` test, "skipped" where the Excel reader cannot be installed | No test reads an Excel file | Its reader is downloaded at run time; the code path is the CSV one with `read_xlsx` and the extension's install before the fence |
| Open read-only, a switch, with "Turn off to edit" in its note | On and fixed (disabled), its note "Files open read-only." | Editing is run 4 |
| Test partial: a banner with three fixes | A file that cannot be read fails Test (and the open), naming the file and DuckDB's first line | Run 2 |
| Agents as a collapsed section | The existing Agents part, open, without Tables and Masked columns, its note "Agents can query the files, never change them." | The collapsed section row comes with Reading CSV in run 2, its second user |
| DuckDB's extensions bundled | `parquet` and `json` are. `excel` installs on first use into `<state>/duckdb-extensions` and loads before the fence; offline, a connection with an Excel file does not open: "Can't read Excel files: the Excel reader for DuckDB could not be installed (…)". No "Downloading…" status | The crate builds only those two in; the install happens inside the open, which reports no progress |
| Type above **Name and Group** | Type, then Name, then Environment | The app has no Group field: connections group by equal names |
| **Paste URL** in the connections list takes a folder or file path | A path typed into a new connection's URL field makes it a Files connection over that folder (or a list of that file) | The list has no Paste URL in this build |
| Row counts in the sidebar, "≈" for estimates | Parquet's count from its metadata, and for CSV, TSV and NDJSON an estimate from the file's first 256 KiB, kept as the table's estimate for the rows view, as PostgreSQL's planner estimate is | The sidebar shows no row counts for any connection, and no board draws one |
| The failure in the footer's status ("Can't open ~/Data/bookshop-exports: no such folder") | The Source field turns red with the board's sentence under it; a failed Test's words go to the message area above the footer | As every type's failed Test does; the footer's status is a running or passed Test's |
| Saving asks "Change Bookshop from PostgreSQL to Files? Its server settings will be removed." | That question in the dialog's message line, ending "Save again to change it."; the next Save changes the type | The dialog has no second sheet for a question |
| The file row's name in medium weight | Its folder dimmer, its name in the text colour, both monospace | The look has no medium monospace text role |
| The macOS folder field, read-only | A read-only box (folder icon, path) beside **Choose…**; Omarchy's is a field to type a path into, read when the keyboard leaves it | Omarchy types paths (completions are run 2) |
| Omarchy footer `enter choose` | A keymap command `FormChoose` (Enter, Space) that the footer names; egui presses the focused choice itself | A key is written only through the keymap |
| The sidebar's attached connections, the CSV settings in Structure | Run 3, run 2 | Joins and CSV options come then |
| SQLite rows "File / No SSH" in the right column (board) | The TLS column as today ("Local") | The prompt names only the line under the name and the path |
| DDL `CREATE VIEW orders AS SELECT * FROM read_csv(…)` | The statement the app made the view with (`… read_csv('…/orders.csv', header = true, auto_detect = true, sample_size = 20000);`) | DuckDB keeps it rewritten, each option in parentheses |

## Found by breaking the draft, and fixed in it

A reviewer broke the draft with throwaway tests before this plan was cut. Each defect is fixed in the tasks below, with a test of its own:

- `EXPLAIN ANALYZE COPY … TO '<a file of the connection>' (USE_TMP_FILE false)` rewrote that file: DuckDB's `allowed_paths` lets a session write a path it may read. A DuckDB `EXPLAIN` that names a refused statement is refused as `EXPLAIN COPY` and the like (`sql::refusal`).
- A Files connection saved from the dialog had `read_only` unset and opened writable, which attached a `.duckdb` file read-write and merged its WAL. The form saves `read_only = true`, and `Connection::connect_with` opens every Files session read-only, whatever access it is asked for.
- A number added to a clashing name could give a name another file has (`orders_csv_2`), and a `.duckdb` file named `main`, `memory`, `system` or `temp` could not be attached: names are now unique, and those four take `_duckdb`.
- A moment with a zone inside a list or a struct failed the whole page (Arrow cannot read the zone's name): such columns are cast without the zone first. `infinity` and `-infinity` dates and moments, and a NULL inside a list, are written as words; text inside a list is quoted.
- A JSON column read as text: a page takes its columns' types from DuckDB's catalog.
- A call given up on (a page waiting behind a script) interrupted the script: a call now interrupts only its own statement, and one given up on before it starts never runs.
- The Omarchy list line lowered the path's case; a pasted SQLite path became a Files folder.

## Known and left

- A `UHUGEINT` above what Arrow's `DECIMAL(38,0)` holds reads wrongly, as DuckDB's Arrow export writes it; a `TIMETZ` loses its offset; a date before 1 AD is written `-0043-03-15`; a `BIT` is its bytes. DuckDB types few files hold.
- A `.duckdb` file's tables in schemas other than `main` are not listed (a query reaches them).
- SQLite's adapter has the same "a dropped call interrupts the session" pattern as the draft had; it is not changed here.
- The Omarchy file list's arrows read `FormTableMove`'s two keys by name (`up`) as Add table…'s list in `agents.rs` already does: a person who remaps them in `settings.toml` gets both moving down. A command for each way would fix both lists.
- `CREATE PERSISTENT SECRET` is not on the refused list; the reviewer found it fails at the secret folder, which the fence keeps out of reach, and it is not tested.

## New words the prompt does not give, for a yes

- "No folder chosen" (the empty folder box), `none · a adds files` (Omarchy's empty list), "missing" (a listed file that is gone), "Choose a folder." and "Add a file." (Save with no source), "Read with" (the Structure view's reader options), the file dialogs' titles "Choose a folder of data files" and "Add data files".
- The change-type question's other two sentences: "Its database file will be removed from it." (from SQLite) and "Its folder or files will be removed from it." (from Files), and, where the connection hides a table or masks a column, "What agents may not read or get masked is taken off too."
- The empty connections list: "Add a PostgreSQL, MySQL or SQLite database, or a folder of data files, to start browsing."
- The Agents part's note on every type: "When Open read-only is on, agents get Read at most. A new connection starts with agents Off, a Files one with Read." (it said every connection starts Off).

## What changes for the other types

- The Type row is a line of its own on every type's macOS form, so the form is taller; the dialog keeps room for 885 points (was 815) when it opens, and one existing test (`the_dialog_warns_when_a_remote_password_is_not_protected`) runs in a 900-point window to keep its warning in sight.
- The change-type question is asked on every type: a used connection changed between a server and a file (SQLite or Files), or between SQLite and Files. PostgreSQL to MySQL and back asks nothing: host, port and user carry over.
- A SQLite database's path typed into the URL field (`/data/shop.db`, by `.db`, `.sqlite`, `.sqlite3`, `.db3`) fills the form as SQLite; before, it was refused as no URL.
- The administrator's policy matches `path` against a Files connection's folder and takes `driver = "files"`; a rule that hides or masks anything there shuts agents out of the connection.
- `connections.toml` refuses a Files connection with hidden tables or masked columns, and the form drops them when a connection is saved as Files.
- The Omarchy footer of a Files form has no `ctrl+enter connect` (Save & Connect stays a button).

## Other platforms

Every check here ran on Linux. The `duckdb` crate compiles DuckDB's C++ through `cc` on every target, so the first build on CI's macOS and Windows runners, and on each release target (Linux aarch64, Windows arm64, macOS universal), is where it is first tried there: expect a long first build (cached afterwards) and a larger binary (DuckDB is some tens of megabytes). The workspace does not cross-compile here, and DuckDB's C++ cannot be type-checked for another target without its toolchain, so nothing was checked for macOS or Windows before the pull request.

## Checks

Every task ends with these, all passing:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist ~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

The first build compiles DuckDB's C++ (about 12 minutes on 32 cores, longer on CI's runners). The server tests need PostgreSQL on 55432 and MySQL on 53306 (`ss -ltn`); without the two URLs they print "skipped" and pass. The Files tests need neither.

## File structure

- `crates/tabletist-db/Cargo.toml`, `Cargo.lock`: `duckdb` (bundled, `parquet`, `json`), which brings `arrow` 60 and 31 crates more; no other pin moves.
- `crates/tabletist-db/src/files.rs` (new): `Source`, `Format`, `Found`, `Scan`, `scan`, `table_name`, `shown`/`expanded` (`~`), `set_home`, `size_text`.
- `crates/tabletist-db/src/duck.rs` (new): the DuckDB adapter, `FilesRead`, `FileFacts`, `set_extension_directory`.
- `crates/tabletist-db/src/{spec,lib,catalog,dialect,class,ddl,reserved,sql,sql/format,agent,agent/hidden,agent/hidden/masked,fixtures,pg,mysql,sqlite}.rs`: the driver and dialect, `Connection::read_files`, `Structure::file`, DuckDB's rules, `Refusal::NotKeptOnFiles`, `fixtures::write_bookshop_files`.
- `crates/tabletist-db/tests/files.rs` (new), `tests/ddl.rs`, `tests/agent.rs`: the adapter's tests, the every-engine DDL test, the dialect arms.
- `crates/tabletist-mcp/src/tools/connections.rs`: agents are told the engine is DuckDB.
- `src/connections/file.rs`: `[connections.source]`, version 7.
- `src/{dates,edit,env,labels,managed,session,entrypoint,paths}.rs`, `src/ui/{calendar,date_line,ddl,format,workspace,start,ddl_tests}.rs`: what each feature does on a Files connection.
- `src/model.rs`, `src/backend.rs`, `src/app.rs`: the form's Files fields, the pickers, the Source read, Test, the change-type question, a pasted path, and the list's scan.
- `src/ui/file_list.rs` (new): the shared file list and table tags. `src/ui/connect_dialog/files.rs` (new): the Source group. `src/ui/connect_dialog/{mod,sheet,terminal,choice,agents}.rs`: the Type row, the fixed read-only box, the URL tab that is off, the Omarchy header, footer and keys, the Agents part.
- `src/keymap.rs`: `FormChoose`, `FormPick`, `FormAddFiles`, `FormRemoveFile`. `src/theme.rs`, `assets/icons/{file,folder}.svg`: two Lucide icons.
- `src/ui/picker.rs`: the list's rows for Files and SQLite. `src/ui/structure.rs`: a Files table's file.
- `src/ui/files_tests.rs` (new): the dialog's and the list's headless tests. `src/ui/mod.rs`: the modules, two literals, and a taller window for a test the Type row pushed down.
- `docs/_reference/connections.md`, `docs/_reference/settings-and-files.md`, `docs/_guide/omarchy.md`: the website.

---

### Before Task 1: the spec and the plan

- [ ] Commit the spec and this plan:

```bash
git add docs/superpowers/specs/2026-10-30-files-connection-design.md docs/superpowers/plans/2026-10-30-files-connection-1-read-only.md
git commit -m "Spec and plan Files connections, run 1"
```

---

### Task 1: DuckDB reads a folder of files

The engine, end to end, and a decision for Files in every place the app matches on the driver or the dialect. The dialog cannot offer Files yet (its type list is still three). Four places get a placeholder arm that Task 2 replaces: the `match form.driver` in `sheet.rs`'s `body` and in `terminal.rs`'s `terminal_body` (`Driver::Files => {}`), `ConnectionForm::to_spec` (`Err("Choose a folder.")`), and the type change in `App::apply`'s `SetDriver` (`Driver::Sqlite | Driver::Files => {}`). `connections.toml` reads and writes a Files connection's source.

What Step 3's diffs do, by file:

- `crates/tabletist-db`: `Cargo.toml` adds `duckdb`; `files.rs` and `duck.rs` are the new modules; `spec.rs`, `lib.rs` and `catalog.rs` add the driver, `ConnectSpec::files`, `Connection::read_files` and `Structure::file`; `dialect.rs`, `class.rs`, `ddl.rs`, `reserved.rs`, `sql.rs`, `sql/format.rs` and `agent*.rs` give DuckDB its arm in each rule (PostgreSQL's lexing; `?` placeholders; the refused statements; `Refusal::NotKeptOnFiles`); `fixtures.rs` writes the Bookshop's exports; `pg.rs`, `mysql.rs` and `sqlite.rs` fill `Structure::file` with `None`; `tests/ddl.rs` reads `book_covers` from the exports' `.duckdb` file, and `tests/agent.rs` names DuckDB in its matches.
- `crates/tabletist-mcp`: agents are told the engine is DuckDB, with `?` parameters.
- The app: `dates.rs` and `ui/calendar.rs`, `ui/date_line.rs` read DuckDB's date and time types as PostgreSQL's (microseconds, `infinity`, offsets sent); `edit.rs` its booleans, floats, defaults and foreign-key failure; `ui/format.rs`'s `refuses_writes` knows DuckDB's read-only error by its words; `managed.rs` matches a Files connection's folder; `session.rs` keys a Files connection's kept tabs by its source; `app.rs`'s `logged_statement` names a Files statement's values as MySQL's (`? #1`); `env.rs`, `labels.rs`, `ui/ddl.rs`, `ui/start.rs`, `ui/workspace.rs`, `ui/picker.rs` and `model.rs` decide what a file-based connection is in each; `connections/file.rs` reads and writes the source; `entrypoint.rs` and `paths.rs` set DuckDB's extension folder and the home folder once at start; `ui/mod.rs` and `ui/ddl_tests.rs` give two literals their new field and the DDL test its Files arm; `settings-and-files.md` documents the file's version 7.

**Files:** 
- Modify: `Cargo.lock`
- Modify: `crates/tabletist-db/Cargo.toml`
- Modify: `crates/tabletist-db/src/agent.rs`
- Modify: `crates/tabletist-db/src/agent/hidden.rs`
- Modify: `crates/tabletist-db/src/agent/hidden/masked.rs`
- Modify: `crates/tabletist-db/src/catalog.rs`
- Modify: `crates/tabletist-db/src/class.rs`
- Modify: `crates/tabletist-db/src/ddl.rs`
- Modify: `crates/tabletist-db/src/dialect.rs`
- Create: `crates/tabletist-db/src/duck.rs`
- Create: `crates/tabletist-db/src/files.rs`
- Modify: `crates/tabletist-db/src/fixtures.rs`
- Modify: `crates/tabletist-db/src/lib.rs`
- Modify: `crates/tabletist-db/src/mysql.rs`
- Modify: `crates/tabletist-db/src/pg.rs`
- Modify: `crates/tabletist-db/src/reserved.rs`
- Modify: `crates/tabletist-db/src/spec.rs`
- Modify: `crates/tabletist-db/src/sql.rs`
- Modify: `crates/tabletist-db/src/sql/format.rs`
- Modify: `crates/tabletist-db/src/sqlite.rs`
- Modify: `crates/tabletist-db/tests/agent.rs`
- Modify: `crates/tabletist-db/tests/ddl.rs`
- Create: `crates/tabletist-db/tests/files.rs`
- Modify: `crates/tabletist-mcp/src/tools/connections.rs`
- Modify: `docs/_reference/settings-and-files.md`
- Modify: `src/app.rs`
- Modify: `src/connections/file.rs`
- Modify: `src/dates.rs`
- Modify: `src/edit.rs`
- Modify: `src/entrypoint.rs`
- Modify: `src/env.rs`
- Modify: `src/labels.rs`
- Modify: `src/managed.rs`
- Modify: `src/model.rs`
- Modify: `src/paths.rs`
- Modify: `src/session.rs`
- Modify: `src/ui/calendar.rs`
- Modify: `src/ui/connect_dialog/mod.rs`
- Modify: `src/ui/connect_dialog/sheet.rs`
- Modify: `src/ui/connect_dialog/terminal.rs`
- Modify: `src/ui/date_line.rs`
- Modify: `src/ui/ddl.rs`
- Modify: `src/ui/ddl_tests.rs`
- Modify: `src/ui/format.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/picker.rs`
- Modify: `src/ui/start.rs`
- Modify: `src/ui/structure.rs`
- Modify: `src/ui/workspace.rs`


- [ ] **Step 1: Write the failing tests**

The adapter's tests, on the Bookshop's exports, which `fixtures::write_bookshop_files` (Step 3) writes from the Bookshop's SQLite data:

```diff
diff --git a/crates/tabletist-db/tests/files.rs b/crates/tabletist-db/tests/files.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-db/tests/files.rs
@@ -0,0 +1,584 @@
+//! Files connections through the public `Connection` API, on the
+//! Bookshop's exports (`fixtures::write_bookshop_files`): a view for each
+//! file, a `.duckdb` file as a schema, and a session that reads those
+//! files and nothing else, and writes nothing.
+
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
+use std::time::Duration;
+
+use tabletist_db::agent::{self, Limits, Refusal, Sources, TableName, Tables};
+use tabletist_db::files::{Format, Source};
+use tabletist_db::{
+    ConnectSpec, Connection, Dialect, Driver, Error, Filter, FilterOp, ObjectKind, ObjectRef,
+    RowQuery, ScriptMode, Secrets, StatementOutcome, StopFlag, Value, ValueKind, sql,
+};
+
+/// The Bookshop's exports in a folder of their own, and a session over it.
+async fn exports() -> (Connection, tempfile::TempDir) {
+    let dir = tempfile::tempdir().unwrap();
+    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
+    let connection = open(Source::Folder(dir.path().to_owned())).await.unwrap();
+    (connection, dir)
+}
+
+async fn open(source: Source) -> Result<Connection, Error> {
+    Connection::connect(&ConnectSpec::files(source), &Secrets::default()).await
+}
+
+/// Why `source` does not open.
+async fn fails(source: Source) -> Error {
+    match open(source).await {
+        Ok(_) => panic!("it opened"),
+        Err(error) => error,
+    }
+}
+
+fn table(name: &str, limit: u32) -> RowQuery {
+    RowQuery::new(ObjectRef::new("main", name), limit)
+}
+
+/// Runs `text` as the SQL editor does: split, then one read-only script.
+async fn script(connection: &Connection, text: &str) -> Result<Vec<StatementOutcome>, Error> {
+    let statements = sql::statements(Dialect::DuckDb, text);
+    let outcome = connection
+        .run_script(&statements, 100, ScriptMode::ReadOnly, &StopFlag::new())
+        .await?;
+    Ok(outcome.results.into_iter().map(|r| r.outcome).collect())
+}
+
+/// The one outcome of a script of one statement, which must be rows.
+async fn rows(connection: &Connection, text: &str) -> Vec<Vec<Value>> {
+    match script(connection, text).await.unwrap().remove(0) {
+        StatementOutcome::Rows { rows, .. } => rows,
+        other => panic!("{text}: {other:?}"),
+    }
+}
+
+/// The one outcome of a script of one statement, which must be an error.
+async fn refused(connection: &Connection, text: &str) -> String {
+    match script(connection, text).await {
+        Err(error) => error.to_string(),
+        Ok(mut outcomes) => match outcomes.remove(0) {
+            StatementOutcome::Error { error, .. } => error.to_string(),
+            other => panic!("{text} ran: {other:?}"),
+        },
+    }
+}
+
+#[tokio::test]
+async fn each_file_is_a_table_and_a_duckdb_file_a_schema() {
+    let (connection, _dir) = exports().await;
+    assert_eq!(connection.driver(), Driver::Files);
+    assert_eq!(connection.dialect(), Dialect::DuckDb);
+    assert!(!connection.is_encrypted());
+    assert!(connection.list_databases().await.unwrap().is_empty());
+    assert_eq!(
+        connection.list_schemas().await.unwrap(),
+        ["main", "bookshop"]
+    );
+    let objects = connection.list_objects("main").await.unwrap();
+    let names: Vec<&str> = objects.iter().map(|o| o.name.as_str()).collect();
+    assert_eq!(
+        names,
+        [
+            "authors",
+            "books",
+            "customers",
+            "events",
+            "orders",
+            "returns"
+        ]
+    );
+    assert!(objects.iter().all(|o| o.kind == ObjectKind::Table));
+    // Parquet says its rows; a text file's are estimated from its start,
+    // which holds the whole of these small ones.
+    let rows = |name: &str| {
+        objects
+            .iter()
+            .find(|o| o.name == name)
+            .unwrap()
+            .estimated_rows
+    };
+    assert_eq!(rows("customers"), Some(8));
+    assert_eq!(rows("orders"), Some(12));
+    assert_eq!(rows("events"), Some(12));
+    assert_eq!(rows("books"), None);
+    let shop = connection.list_objects("bookshop").await.unwrap();
+    let names: Vec<&str> = shop.iter().map(|o| o.name.as_str()).collect();
+    assert_eq!(names, ["book_covers", "order_items"]);
+    let version = connection.server_version().await.unwrap();
+    assert!(version.starts_with("DuckDB 2."), "{version}");
+    assert_eq!(
+        connection.session_zone().await.unwrap().as_deref(),
+        Some("UTC")
+    );
+}
+
+#[tokio::test]
+async fn every_format_reads_into_columns() {
+    let (connection, _dir) = exports().await;
+    for (name, first) in [
+        ("orders", "id"),
+        ("returns", "order_id"),
+        ("authors", "id"),
+        ("customers", "id"),
+        ("books", "id"),
+        ("events", "order_id"),
+    ] {
+        let page = connection.fetch_rows(&table(name, 3)).await.unwrap();
+        assert_eq!(page.columns[0].name, first, "{name}");
+        assert!(page.columns.len() > 1, "{name}: {:?}", page.columns);
+        assert!(!page.rows.is_empty(), "{name}");
+        assert!(!page.ordered_by_key, "{name}");
+    }
+    // The TSV is split on its tabs, not read as one column.
+    let authors = connection.fetch_rows(&table("authors", 1)).await.unwrap();
+    assert_eq!(authors.columns[1].name, "name");
+    let structure = connection
+        .describe(&ObjectRef::new("main", "orders"))
+        .await
+        .unwrap();
+    let types: Vec<(&str, &str)> = structure
+        .columns
+        .iter()
+        .map(|c| (c.name.as_str(), c.type_name.as_str()))
+        .collect();
+    assert_eq!(types[0], ("id", "BIGINT"));
+    assert!(structure.primary_key.is_empty());
+    let file = structure.file.unwrap();
+    assert_eq!(file.format, Format::Csv);
+    assert!(file.path.ends_with("orders.csv"));
+    assert!(file.size > 0);
+    assert!(
+        file.options.contains("sample_size = 20000"),
+        "{}",
+        file.options
+    );
+    // A `.duckdb` file's table is no file's.
+    let covers = connection
+        .describe(&ObjectRef::new("bookshop", "book_covers"))
+        .await
+        .unwrap();
+    assert!(covers.file.is_none());
+    assert!(!covers.columns.is_empty());
+}
+
+#[tokio::test]
+async fn values_keep_their_kinds() {
+    let (connection, _dir) = exports().await;
+    let page = connection.fetch_rows(&table("orders", 1)).await.unwrap();
+    let kinds: Vec<(&str, ValueKind)> = page
+        .columns
+        .iter()
+        .map(|c| (c.name.as_str(), c.kind))
+        .collect();
+    assert_eq!(kinds[0], ("id", ValueKind::Numeric));
+    assert!(matches!(page.rows[0][0], Value::Int(_)));
+    let at = rows(
+        &connection,
+        "SELECT DATE '2024-03-05', TIMESTAMP '2024-03-05 10:04:00', \
+         TIMESTAMPTZ '2024-03-05 10:04:00+02', 12.50::DECIMAL(10,2), true, \
+         '\\xAB'::BLOB, [1, 2], NULL",
+    )
+    .await;
+    assert_eq!(
+        at[0],
+        [
+            Value::Text("2024-03-05".into()),
+            Value::Text("2024-03-05 10:04:00".into()),
+            Value::Text("2024-03-05 08:04:00+00".into()),
+            Value::Text("12.50".into()),
+            Value::Bool(true),
+            Value::Bytes(vec![0xAB].into()),
+            Value::Text("[1, 2]".into()),
+            Value::Null,
+        ]
+    );
+}
+
+#[tokio::test]
+async fn pages_filter_sort_and_count() {
+    let (connection, _dir) = exports().await;
+    let mut query = table("orders", 2);
+    query.filters.push(Filter {
+        column: "id".into(),
+        op: FilterOp::Gt,
+        value: "2".into(),
+    });
+    let page = connection.fetch_rows(&query).await.unwrap();
+    assert_eq!(page.rows.len(), 2);
+    assert!(page.has_more);
+    assert_eq!(connection.count_rows(&query).await.unwrap(), 10);
+    query.raw_where = Some("id = 3".into());
+    assert_eq!(connection.count_rows(&query).await.unwrap(), 1);
+    query.raw_where = Some("id = 3; DROP VIEW orders".into());
+    let error = connection.count_rows(&query).await.unwrap_err();
+    assert!(error.to_string().contains("holds a `;`"), "{error}");
+}
+
+#[tokio::test]
+async fn the_ddl_is_the_view_over_the_file() {
+    let (connection, dir) = exports().await;
+    let ddl = connection
+        .ddl(&ObjectRef::new("main", "orders"))
+        .await
+        .unwrap();
+    assert_eq!(ddl.driver, Driver::Files);
+    let text = ddl.text();
+    let path = dir.path().join("orders.csv");
+    assert_eq!(
+        text,
+        format!(
+            "CREATE VIEW orders AS SELECT * FROM read_csv('{}', header = true, \
+             auto_detect = true, sample_size = 20000);",
+            path.display()
+        )
+    );
+}
+
+#[tokio::test]
+async fn scripts_read_the_files_and_join_them() {
+    let (connection, _dir) = exports().await;
+    let found = rows(
+        &connection,
+        "SELECT count(*) FROM orders o JOIN customers c ON c.id = o.customer_id",
+    )
+    .await;
+    assert_eq!(found, [[Value::Int(12)]]);
+    let found = rows(&connection, "FROM bookshop.order_items SELECT count(*)").await;
+    assert_eq!(found, [[Value::Int(40)]]);
+    // A run ends at its first error.
+    let outcomes = script(&connection, "SELECT 1; SELECT * FROM nonsense; SELECT 2")
+        .await
+        .unwrap();
+    assert_eq!(outcomes.len(), 2, "the run ends at the first error");
+}
+
+#[tokio::test]
+async fn nothing_writes_and_nothing_else_is_read() {
+    let (connection, dir) = exports().await;
+    let orders = dir.path().join("orders.csv");
+    let before = std::fs::read(&orders).unwrap();
+    let other = dir.path().join("secrets.csv");
+    std::fs::write(&other, "a\n1\n").unwrap();
+    let quoted = |path: &std::path::Path| format!("'{}'", path.display());
+    for (text, says) in [
+        ("CREATE TABLE notes (id INTEGER)", "read-only"),
+        ("DROP VIEW orders", "read-only"),
+        (&format!("COPY orders TO {}", quoted(&orders)), "COPY"),
+        ("INSTALL excel", "INSTALL"),
+        ("SET enable_external_access = true", "SET"),
+        ("ATTACH ':memory:' AS scratch", "ATTACH"),
+        ("USE bookshop", "USE"),
+        ("PRAGMA enable_profiling", "PRAGMA"),
+        (
+            &format!("SELECT * FROM read_csv({})", quoted(&other)),
+            "Permission",
+        ),
+        (&format!("SELECT * FROM {}", quoted(&other)), "Permission"),
+        ("SELECT * FROM glob('/*')", "Permission"),
+        ("BEGIN", "BEGIN"),
+    ] {
+        let error = refused(&connection, text).await;
+        assert!(error.contains(says), "{text}: {error}");
+    }
+    assert_eq!(std::fs::read(&orders).unwrap(), before);
+    // Still one session that reads, as it was.
+    assert_eq!(
+        rows(&connection, "SELECT count(*) FROM orders").await,
+        [[Value::Int(12)]]
+    );
+    // A script that would write is refused before it is sent.
+    let statements = sql::statements(Dialect::DuckDb, "SELECT 1");
+    let error = connection
+        .run_script(&statements, 10, ScriptMode::Write, &StopFlag::new())
+        .await
+        .unwrap_err();
+    assert_eq!(error, Error::ReadOnly);
+}
+
+#[tokio::test]
+async fn a_test_reads_every_file_and_says_how_much() {
+    let (connection, dir) = exports().await;
+    let read = connection.read_files().await.unwrap().unwrap();
+    assert_eq!(read.files, 7);
+    let size: u64 = std::fs::read_dir(dir.path())
+        .unwrap()
+        .map(|entry| entry.unwrap().metadata().unwrap().len())
+        .sum();
+    assert_eq!(read.bytes, size);
+}
+
+#[tokio::test]
+async fn a_folder_that_is_gone_or_a_file_that_is_not_there_fails_to_open() {
+    let dir = tempfile::tempdir().unwrap();
+    let gone = dir.path().join("bookshop-exports");
+    let error = fails(Source::Folder(gone)).await;
+    let Error::Connect(message) = &error else {
+        panic!("{error:?}");
+    };
+    assert!(
+        message.ends_with("bookshop-exports: no such folder"),
+        "{message}"
+    );
+    let error = fails(Source::Files(vec![dir.path().join("orders.csv")])).await;
+    assert!(
+        error.to_string().contains("orders.csv: no such file"),
+        "{error}"
+    );
+    // A file that is not what its name says names itself.
+    std::fs::write(dir.path().join("customers.parquet"), "not parquet").unwrap();
+    let error = fails(Source::Files(vec![dir.path().join("customers.parquet")])).await;
+    assert!(
+        error.to_string().contains("Can't read customers.parquet: "),
+        "{error}"
+    );
+    assert!(!error.to_string().contains('\n'), "{error}");
+}
+
+#[tokio::test]
+async fn a_list_opens_only_its_files_and_clashes_take_their_extension() {
+    let dir = tempfile::tempdir().unwrap();
+    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
+    let parquet = dir.path().join("orders.parquet");
+    // An orders.parquet beside orders.csv, written by DuckDB itself.
+    duckdb_copy(&dir.path().join("orders.csv"), &parquet);
+    let connection = open(Source::Files(vec![
+        dir.path().join("orders.csv"),
+        parquet.clone(),
+    ]))
+    .await
+    .unwrap();
+    let names: Vec<String> = connection
+        .list_objects("main")
+        .await
+        .unwrap()
+        .into_iter()
+        .map(|o| o.name)
+        .collect();
+    assert_eq!(names, ["orders_csv", "orders_parquet"]);
+    assert_eq!(connection.list_schemas().await.unwrap(), ["main"]);
+    // The others are not the session's files.
+    let error = refused(&connection, "SELECT * FROM customers").await;
+    assert!(error.contains("customers"), "{error}");
+}
+
+/// Writes `csv` again as Parquet at `to`.
+fn duckdb_copy(csv: &std::path::Path, to: &std::path::Path) {
+    let session = duckdb::Connection::open_in_memory().unwrap();
+    session
+        .execute_batch(&format!(
+            "COPY (SELECT * FROM read_csv('{}')) TO '{}' (FORMAT parquet)",
+            csv.display(),
+            to.display()
+        ))
+        .unwrap();
+}
+
+const LIMITS: Limits = Limits {
+    rows: 100,
+    timeout: Duration::from_secs(5),
+};
+
+#[tokio::test]
+async fn agents_read_the_files_and_are_refused_where_a_rule_hides() {
+    let (connection, _dir) = exports().await;
+    let read = agent::check_read(
+        Dialect::DuckDb,
+        "SELECT count(*) FROM orders WHERE id > ?",
+        &[agent::Param::Int(2)],
+    )
+    .unwrap();
+    let answer = connection
+        .run_agent_read(&read, LIMITS, &Sources::default(), &StopFlag::new())
+        .await
+        .unwrap();
+    assert_eq!(answer.rows, [[Value::Int(10)]]);
+    // A statement that is no read is refused before it is sent.
+    assert!(agent::check_read(Dialect::DuckDb, "COPY orders TO 'x.csv'", &[]).is_err());
+    assert!(agent::check_read(Dialect::DuckDb, "SELECT query('SELECT 1')", &[]).is_err());
+    // A rule that hides anything refuses every read.
+    let hides = Sources::person(
+        Tables::AllExcept(vec!["customers".parse::<TableName>().unwrap()]),
+        Vec::new(),
+    );
+    let error = connection
+        .run_agent_read(&read, LIMITS, &hides, &StopFlag::new())
+        .await
+        .unwrap_err();
+    assert_eq!(error, Error::Agent(Refusal::NotKeptOnFiles));
+}
+
+#[tokio::test]
+async fn a_stop_ends_a_long_statement() {
+    let (connection, _dir) = exports().await;
+    let statements = sql::statements(
+        Dialect::DuckDb,
+        "SELECT count(*) FROM range(100000000000) a, range(1000) b",
+    );
+    let stop = StopFlag::new();
+    let stopping = stop.clone();
+    tokio::spawn(async move {
+        tokio::time::sleep(Duration::from_millis(200)).await;
+        stopping.stop();
+    });
+    let started = std::time::Instant::now();
+    let outcome = connection
+        .run_script(&statements, 10, ScriptMode::ReadOnly, &stop)
+        .await
+        .unwrap();
+    assert!(outcome.was_cancelled(), "{outcome:?}");
+    assert!(started.elapsed() < Duration::from_secs(10));
+    // The session reads again after it.
+    assert_eq!(rows(&connection, "SELECT 1").await, [[Value::Int(1)]]);
+}
+
+#[tokio::test]
+async fn an_agents_read_past_its_time_is_a_timeout() {
+    let (connection, _dir) = exports().await;
+    let read = agent::check_read(
+        Dialect::DuckDb,
+        "SELECT count(*) FROM range(100000000000) a, range(1000) b",
+        &[],
+    )
+    .unwrap();
+    let limits = Limits {
+        rows: 10,
+        timeout: Duration::from_millis(200),
+    };
+    let error = connection
+        .run_agent_read(&read, limits, &Sources::default(), &StopFlag::new())
+        .await
+        .unwrap_err();
+    assert_eq!(error, Error::Timeout);
+}
+
+#[tokio::test]
+async fn an_explain_that_would_run_a_copy_is_refused_and_the_file_kept() {
+    let (connection, dir) = exports().await;
+    let orders = dir.path().join("orders.csv");
+    let before = std::fs::read(&orders).unwrap();
+    for text in [
+        format!(
+            "EXPLAIN ANALYZE COPY (SELECT 'x' AS a) TO '{}' (USE_TMP_FILE false)",
+            orders.display()
+        ),
+        format!(
+            "EXPLAIN (ANALYZE) COPY (SELECT 'x' AS a) TO '{}' (USE_TMP_FILE false)",
+            orders.display()
+        ),
+    ] {
+        let error = refused(&connection, &text).await;
+        assert!(error.contains("EXPLAIN COPY"), "{text}: {error}");
+    }
+    assert_eq!(std::fs::read(&orders).unwrap(), before);
+}
+
+#[tokio::test]
+async fn a_session_asked_to_write_still_reads_only() {
+    let dir = tempfile::tempdir().unwrap();
+    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
+    let shop = dir.path().join("bookshop.duckdb");
+    let before = std::fs::read(&shop).unwrap();
+    let connection = tabletist_db::Connection::connect_with(
+        &ConnectSpec::files(Source::Folder(dir.path().to_owned())),
+        &Secrets::default(),
+        &tabletist_db::HostKeys::default(),
+        tabletist_db::Access::Writable,
+    )
+    .await
+    .unwrap();
+    assert_eq!(connection.access(), tabletist_db::Access::ReadOnly);
+    let error = refused(
+        &connection,
+        "INSERT INTO bookshop.book_covers SELECT * FROM bookshop.book_covers",
+    )
+    .await;
+    assert!(error.contains("read-only"), "{error}");
+    connection.close().await.unwrap();
+    assert_eq!(
+        std::fs::read(&shop).unwrap(),
+        before,
+        "the .duckdb file was written"
+    );
+}
+
+#[tokio::test]
+async fn nested_moments_infinities_and_nulls_are_written_out() {
+    let (connection, _dir) = exports().await;
+    let found = rows(
+        &connection,
+        "SELECT [TIMESTAMPTZ '2024-01-01 12:00:00+00'], 'infinity'::TIMESTAMP, \
+         '-infinity'::DATE, [1, NULL], ['a']",
+    )
+    .await;
+    assert_eq!(
+        found[0],
+        [
+            Value::Text("[2024-01-01 12:00:00]".into()),
+            Value::Text("infinity".into()),
+            Value::Text("-infinity".into()),
+            Value::Text("[1, NULL]".into()),
+            Value::Text("[\"a\"]".into()),
+        ]
+    );
+}
+
+#[tokio::test]
+async fn a_json_column_is_json_by_the_catalog() {
+    let dir = tempfile::tempdir().unwrap();
+    let file = dir.path().join("events.duckdb");
+    let session = duckdb::Connection::open(&file).unwrap();
+    session
+        .execute_batch("CREATE TABLE logged (id INTEGER, body JSON); INSERT INTO logged VALUES (1, '{\"a\": 1}')")
+        .unwrap();
+    drop(session);
+    let connection = open(Source::Files(vec![file])).await.unwrap();
+    let page = connection
+        .fetch_rows(&RowQuery::new(ObjectRef::new("events", "logged"), 5))
+        .await
+        .unwrap();
+    assert_eq!(page.columns[1].kind, ValueKind::Json);
+    assert_eq!(page.columns[1].type_name, "JSON");
+    assert_eq!(page.columns[0].type_name, "INTEGER");
+}
+
+#[tokio::test]
+async fn a_call_given_up_on_stops_no_other_calls_statement() {
+    let (connection, _dir) = exports().await;
+    let connection = std::sync::Arc::new(connection);
+    let statements = sql::statements(
+        Dialect::DuckDb,
+        "SELECT count(*) FROM range(100000000000) a, range(1000) b",
+    );
+    let stop = StopFlag::new();
+    let running = {
+        let (connection, stop) = (std::sync::Arc::clone(&connection), stop.clone());
+        tokio::spawn(async move {
+            let started = std::time::Instant::now();
+            let outcome = connection
+                .run_script(&statements, 10, ScriptMode::ReadOnly, &stop)
+                .await
+                .unwrap();
+            (outcome, started.elapsed())
+        })
+    };
+    tokio::time::sleep(Duration::from_millis(100)).await;
+    // A page asked for behind it, and given up on.
+    let waited = tokio::time::timeout(
+        Duration::from_millis(100),
+        connection.fetch_rows(&table("orders", 1)),
+    )
+    .await;
+    assert!(waited.is_err(), "the page should wait behind the script");
+    tokio::time::sleep(Duration::from_millis(500)).await;
+    stop.stop();
+    let (outcome, took) = running.await.unwrap();
+    assert!(outcome.was_cancelled());
+    assert!(
+        took >= Duration::from_millis(600),
+        "the script stopped after {took:?}"
+    );
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test files`

Expected: it does not compile, there being no `files` module and no `Driver::Files`:

```text
error[E0432]: unresolved import `tabletist_db::files`
  --> crates/tabletist-db/tests/files.rs:12:19
error[E0433]: cannot find module or crate `duckdb` in this scope
   --> crates/tabletist-db/tests/files.rs:370:19
error[E0433]: cannot find module or crate `duckdb` in this scope
   --> crates/tabletist-db/tests/files.rs:533:19
error[E0425]: cannot find function `write_bookshop_files` in module `tabletist_db::fixtures`
  --> crates/tabletist-db/tests/files.rs:21:29
```

- [ ] **Step 3: Write the engine, and every match's arm** (DuckDB keeps a view's statement rewritten, `read_csv('…', ("header" = true), (auto_detect = true), …)`, so `duck.rs` keeps the statement it made for the DDL view)

```diff
diff --git a/Cargo.lock b/Cargo.lock
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -167,6 +167,7 @@ source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "5a15f179cd60c4584b8a8c596927aadc462e27f2ca70c04e0071964a73ba7a75"
 dependencies = [
  "cfg-if",
+ "const-random",
  "getrandom 0.3.4",
  "once_cell",
  "serde",
@@ -343,6 +344,183 @@ version = "0.7.8"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "d3fb67a6e08acf24fdeccbac2cb6ac4305825bd1f117462e0e6f2f193345ad56"
 
+[[package]]
+name = "arrow"
+version = "60.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "d965156f2220e94dcf676e04322b7b0b41c97e85cb3e1c365b8e56d0f70be999"
+dependencies = [
+ "arrow-arith",
+ "arrow-array",
+ "arrow-buffer",
+ "arrow-cast",
+ "arrow-data",
+ "arrow-ord",
+ "arrow-row",
+ "arrow-schema",
+ "arrow-select",
+ "arrow-string",
+]
+
+[[package]]
+name = "arrow-arith"
+version = "60.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "ca16fdd1daeb1eccefb42df52ec65448ef33eec72488598675d46dfa9ce58923"
+dependencies = [
+ "arrow-array",
+ "arrow-buffer",
+ "arrow-data",
+ "arrow-schema",
+ "chrono",
+ "num-traits",
+]
+
+[[package]]
+name = "arrow-array"
+version = "60.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "0f3d22a41d3df78b71df0b685af219ae3a94188ab98d6fb29c958eb55c3247f4"
+dependencies = [
+ "ahash",
+ "arrow-buffer",
+ "arrow-data",
+ "arrow-schema",
+ "chrono",
+ "half",
+ "hashbrown 0.17.1",
+ "libc",
+ "num-complex",
+ "num-integer",
+ "num-traits",
+]
+
+[[package]]
+name = "arrow-buffer"
+version = "60.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "7bd46a530bc1a8e763102fc36c1dc0f51f3ffa150c78e7f5658ccb54b1458ade"
+dependencies = [
+ "bytes",
+ "half",
+ "num-bigint 0.5.1",
+ "num-traits",
+]
+
+[[package]]
+name = "arrow-cast"
+version = "60.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "baa83a0752814a1922d5ee75032524ca9bbfb7f60ee35a67ed4ab8b275260aaf"
+dependencies = [
+ "arrow-array",
+ "arrow-buffer",
+ "arrow-data",
+ "arrow-ord",
+ "arrow-schema",
+ "arrow-select",
+ "atoi",
+ "base64 0.23.1",
+ "chrono",
+ "comfy-table",
+ "half",
+ "lexical-core",
+ "num-traits",
+ "ryu",
+]
+
+[[package]]
+name = "arrow-cmp"
+version = "60.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "1913e57a64fbf2c4502fa6d54b547d6a1ae59e3146cbc15321d47468c937abd1"
+dependencies = [
+ "arrow-array",
+ "arrow-buffer",
+ "arrow-schema",
+]
+
+[[package]]
+name = "arrow-data"
+version = "60.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "415b7fbc4cf56b080bb5677a4ae74211b5c79e571ed37a1d547633ec07f3b43b"
+dependencies = [
+ "arrow-buffer",
+ "arrow-schema",
+ "half",
+ "num-integer",
+ "num-traits",
+]
+
+[[package]]
+name = "arrow-ord"
+version = "60.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "4c58f2ae6dd1480fc413f7320a547e39dcc499c23d4f36ca19572b685811e6e4"
+dependencies = [
+ "arrow-array",
+ "arrow-buffer",
+ "arrow-cmp",
+ "arrow-data",
+ "arrow-schema",
+ "arrow-select",
+]
+
+[[package]]
+name = "arrow-row"
+version = "60.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "68f9db8345b0638416d3b56d117d262d13070fdef788c6d75b53a180215c477e"
+dependencies = [
+ "arrow-array",
+ "arrow-buffer",
+ "arrow-data",
+ "arrow-schema",
+ "half",
+]
+
+[[package]]
+name = "arrow-schema"
+version = "60.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "de2c8e714b466404cdc1189e89f892426efea614d938e16c15518c340cd43ba3"
+dependencies = [
+ "bitflags 2.13.2",
+]
+
+[[package]]
+name = "arrow-select"
+version = "60.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "a5af3f6656c5f2c6944f93d9a357a7f204c93d5f349385a44cf46c4a26fbcf30"
+dependencies = [
+ "ahash",
+ "arrow-array",
+ "arrow-buffer",
+ "arrow-cmp",
+ "arrow-data",
+ "arrow-schema",
+ "num-traits",
+]
+
+[[package]]
+name = "arrow-string"
+version = "60.1.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "ab1be9306b9a413b5b004e2a5a19caac728870d76e8891b351f040d554c93862"
+dependencies = [
+ "arrow-array",
+ "arrow-buffer",
+ "arrow-data",
+ "arrow-schema",
+ "arrow-select",
+ "memchr",
+ "num-traits",
+ "regex",
+ "regex-syntax",
+]
+
 [[package]]
 name = "as-raw-xcb-connection"
 version = "1.0.1"
@@ -489,6 +667,15 @@ dependencies = [
  "syn 3.0.6",
 ]
 
+[[package]]
+name = "atoi"
+version = "3.1.1"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "d12c37257553c19e9aee6cba77814fdc9e07844abfc8b3a2c377cd920ee5a993"
+dependencies = [
+ "num-traits",
+]
+
 [[package]]
 name = "atomic-waker"
 version = "1.1.2"
@@ -785,6 +972,12 @@ dependencies = [
  "wayland-client",
 ]
 
+[[package]]
+name = "cast"
+version = "0.3.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "37b2a672a2cb129a2e41c10b1224bb368f9f37a2b16b612598138befd7b37eb5"
+
 [[package]]
 name = "cbc"
 version = "0.2.1"
@@ -957,6 +1150,16 @@ dependencies = [
  "memchr",
 ]
 
+[[package]]
+name = "comfy-table"
+version = "8.0.1"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "b96a46ad395a5e00676eddaca4a45ed78b29c08be5a878d35e4b071c625637dd"
+dependencies = [
+ "unicode-segmentation",
+ "unicode-width",
+]
+
 [[package]]
 name = "concat-string"
 version = "1.0.1"
@@ -984,6 +1187,26 @@ version = "0.10.2"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "a6ef517f0926dd24a1582492c791b6a4818a4d94e789a334894aa15b0d12f55c"
 
+[[package]]
+name = "const-random"
+version = "0.1.18"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "87e00182fe74b066627d63b85fd550ac2998d4b0bd86bfed477a0ae4c7c71359"
+dependencies = [
+ "const-random-macro",
+]
+
+[[package]]
+name = "const-random-macro"
+version = "0.1.16"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "f9d839f2a20b0aee515dc581a6172f2321f96cab76c1a38a4c584a194955390e"
+dependencies = [
+ "getrandom 0.2.17",
+ "once_cell",
+ "tiny-keccak",
+]
+
 [[package]]
 name = "core-foundation"
 version = "0.9.4"
@@ -1436,6 +1659,22 @@ version = "0.1.2"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "d8b14ccef22fc6f5a8f4d7d768562a182c04ce9a3b3157b91390b52ddfdf1a76"
 
+[[package]]
+name = "duckdb"
+version = "1.20000.1"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "d807ca47d4fa206f102ff568046e018fb84518eb0a0b5e65a165a2eb7bd4a7c2"
+dependencies = [
+ "arrow",
+ "cast",
+ "fallible-iterator 0.3.0",
+ "fallible-streaming-iterator",
+ "hashlink",
+ "libduckdb-sys",
+ "num-integer",
+ "strum",
+]
+
 [[package]]
 name = "dyn-clone"
 version = "1.0.20"
@@ -1997,6 +2236,16 @@ version = "0.3.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "64cd1e32ddd350061ae6edb1b082d7c54915b5c672c389143b9a63403a109f24"
 
+[[package]]
+name = "filetime"
+version = "0.2.29"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "5c287a33c7f0a620c38e641e7f60827713987b3c0f26e8ddc9462cc69cf75759"
+dependencies = [
+ "cfg-if",
+ "libc",
+]
+
 [[package]]
 name = "find-msvc-tools"
 version = "0.1.14"
@@ -3023,12 +3272,86 @@ dependencies = [
  "smallvec",
 ]
 
+[[package]]
+name = "lexical-core"
+version = "1.0.6"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "7d8d125a277f807e55a77304455eb7b1cb52f2b18c143b60e766c120bd64a594"
+dependencies = [
+ "lexical-parse-float",
+ "lexical-parse-integer",
+ "lexical-util",
+ "lexical-write-float",
+ "lexical-write-integer",
+]
+
+[[package]]
+name = "lexical-parse-float"
+version = "1.0.6"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "52a9f232fbd6f550bc0137dcb5f99ab674071ac2d690ac69704593cb4abbea56"
+dependencies = [
+ "lexical-parse-integer",
+ "lexical-util",
+]
+
+[[package]]
+name = "lexical-parse-integer"
+version = "1.0.6"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "9a7a039f8fb9c19c996cd7b2fcce303c1b2874fe1aca544edc85c4a5f8489b34"
+dependencies = [
+ "lexical-util",
+]
+
+[[package]]
+name = "lexical-util"
+version = "1.0.7"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "2604dd126bb14f13fb5d1bd6a66155079cb9fa655b37f875b3a742c705dbed17"
+
+[[package]]
+name = "lexical-write-float"
+version = "1.0.6"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "50c438c87c013188d415fbabbb1dceb44249ab81664efbd31b14ae55dabb6361"
+dependencies = [
+ "lexical-util",
+ "lexical-write-integer",
+]
+
+[[package]]
+name = "lexical-write-integer"
+version = "1.0.6"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "409851a618475d2d5796377cad353802345cba92c867d9fbcde9cf4eac4e14df"
+dependencies = [
+ "lexical-util",
+]
+
 [[package]]
 name = "libc"
 version = "0.2.189"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "3eaf3ede3fee6db1a4c2ee091bf8a8b4dccdc6d17f656fb07896ee72867612f2"
 
+[[package]]
+name = "libduckdb-sys"
+version = "1.20000.1"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "0140448c472968ee5cf5ba697435deebe558b366ef1e5c080d24d9696b600725"
+dependencies = [
+ "cc",
+ "flate2",
+ "pkg-config",
+ "serde",
+ "serde_json",
+ "sha2 0.11.0",
+ "tar",
+ "ureq",
+ "vcpkg",
+]
+
 [[package]]
 name = "libloading"
 version = "0.8.9"
@@ -5088,6 +5411,12 @@ version = "1.0.23"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "cf54715a573b99ac80df0bc206da022bcd442c974952c7b9720069370852e21f"
 
+[[package]]
+name = "ryu"
+version = "1.0.23"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "9774ba4a74de5f7b1c1451ed6cd5285a32eddb5cccb8cc655a4e50009e06477f"
+
 [[package]]
 name = "salsa20"
 version = "0.11.0"
@@ -5709,6 +6038,27 @@ version = "0.11.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "7da8b5736845d9f2fcb837ea5d9e2628564b3b043a70948a3f0b778838c5fb4f"
 
+[[package]]
+name = "strum"
+version = "0.27.2"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "af23d6f6c1a224baef9d3f61e287d2761385a5b88fdab4eb4c6f11aeb54c4bcf"
+dependencies = [
+ "strum_macros",
+]
+
+[[package]]
+name = "strum_macros"
+version = "0.27.2"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "7695ce3845ea4b33927c055a39dc438a45b059f7c1b3d91d38d10355fb8cbca7"
+dependencies = [
+ "heck",
+ "proc-macro2",
+ "quote",
+ "syn 2.0.119",
+]
+
 [[package]]
 name = "subtle"
 version = "2.6.1"
@@ -5836,6 +6186,7 @@ name = "tabletist-db"
 version = "0.1.0"
 dependencies = [
  "bytes",
+ "duckdb",
  "futures-util",
  "log",
  "mysql_async",
@@ -5894,6 +6245,17 @@ dependencies = [
  "rusqlite",
 ]
 
+[[package]]
+name = "tar"
+version = "0.4.46"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "3f6221d9a6003c78398e3b239969f352578258df48c8eb051caadae0015bc840"
+dependencies = [
+ "filetime",
+ "libc",
+ "xattr",
+]
+
 [[package]]
 name = "tempfile"
 version = "3.27.0"
@@ -5970,6 +6332,15 @@ dependencies = [
  "zune-jpeg",
 ]
 
+[[package]]
+name = "tiny-keccak"
+version = "2.0.2"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "2c9d3793400a45f954c52e73d068316d76b6f4e36977e3fcebb13a2721e80237"
+dependencies = [
+ "crunchy",
+]
+
 [[package]]
 name = "tiny-skia"
 version = "0.11.4"
@@ -7366,6 +7737,16 @@ dependencies = [
  "tls_codec",
 ]
 
+[[package]]
+name = "xattr"
+version = "1.6.1"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "32e45ad4206f6d2479085147f02bc2ef834ac85886624a23575ae137c8aa8156"
+dependencies = [
+ "libc",
+ "rustix 1.1.5",
+]
+
 [[package]]
 name = "xcursor"
 version = "0.3.11"
```

```diff
diff --git a/crates/tabletist-db/Cargo.toml b/crates/tabletist-db/Cargo.toml
--- a/crates/tabletist-db/Cargo.toml
+++ b/crates/tabletist-db/Cargo.toml
@@ -68,6 +68,11 @@ mysql_common = { version = "0.37", default-features = false }
 # tree through the app and mysql_async.
 serde_json = "1"
 
+# Files connections: DuckDB, compiled in, reads CSV, Parquet, JSON and
+# .duckdb files as tables. `parquet` and `json` build those readers into
+# the library; the rest install on first use.
+duckdb = { version = "1.20000.1", features = ["bundled", "parquet", "json"] }
+
 [dev-dependencies]
 tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync", "net"] }
 tempfile = "3"
```

```diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -21,7 +21,7 @@
 use std::fmt;
 
 use sqlparser::ast::{Query, SetExpr, Statement, UtilityOption};
-use sqlparser::dialect::{MySqlDialect, PostgreSqlDialect, SQLiteDialect};
+use sqlparser::dialect::{DuckDbDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
 use sqlparser::parser::Parser;
 use sqlparser::tokenizer::{Token, Tokenizer, Whitespace};
 
@@ -172,6 +172,11 @@ pub enum Refusal {
     /// connection until the policy is fixed. Nothing the file says is
     /// told: the entry names nothing on this database.
     PolicyNeedsFixing,
+    /// The connection's settings or the administrator's policy hide a
+    /// table or mask a column of a Files connection, which keeps neither:
+    /// a file has no catalog to say what a statement reads. Agents may not
+    /// use the connection until the rule is taken off.
+    NotKeptOnFiles,
 }
 
 /// How a statement reads a masked column otherwise than selecting it as it
@@ -259,7 +264,9 @@ impl Refusal {
             Self::TableHidden { .. } => "table_hidden",
             Self::SystemCatalog => "system_catalog",
             Self::MaskedColumnUsed { .. } => "masked_column_used",
-            Self::SettingsNeedALook { .. } | Self::PolicyNeedsFixing => "connection_unavailable",
+            Self::SettingsNeedALook { .. } | Self::PolicyNeedsFixing | Self::NotKeptOnFiles => {
+                "connection_unavailable"
+            }
         }
     }
 }
@@ -312,7 +319,7 @@ impl fmt::Display for Refusal {
             Self::InvalidParams(ParamFault::Placeholder { written, dialect }) => {
                 let own = match dialect {
                     Dialect::Postgres => "$1, $2 and so on",
-                    Dialect::MySql | Dialect::Sqlite => "?",
+                    Dialect::MySql | Dialect::Sqlite | Dialect::DuckDb => "?",
                 };
                 write!(
                     f,
@@ -364,6 +371,10 @@ impl fmt::Display for Refusal {
             Self::PolicyNeedsFixing => f.write_str(
                 "Agents may not use this connection until your administrator's policy is fixed.",
             ),
+            Self::NotKeptOnFiles => f.write_str(
+                "A Files connection cannot hide tables or mask columns, so agents may not use \
+                 it while a rule does.",
+            ),
             Self::MaskedColumnUsed { column, how, by } => masked_sentence(f, column, how, *by),
             Self::SystemCatalog => f.write_str(
                 "The system catalogs are not available on this connection. Use describe_schema.",
@@ -517,6 +528,7 @@ fn read_parsed(dialect: Dialect, text: &str, params: &[Param]) -> Result<bool, R
         Dialect::Postgres => &PostgreSqlDialect {},
         Dialect::MySql => &MySqlDialect {},
         Dialect::Sqlite => &SQLiteDialect {},
+        Dialect::DuckDb => &DuckDbDialect {},
     };
     let unread = || unread(dialect, text);
     let tokens = Tokenizer::new(parser, text)
@@ -579,7 +591,7 @@ fn read_parsed(dialect: Dialect, text: &str, params: &[Param]) -> Result<bool, R
 fn hints_read(dialect: Dialect, tokens: &[Token], text: &str) -> Result<(), Refusal> {
     match dialect {
         Dialect::MySql => {}
-        Dialect::Postgres | Dialect::Sqlite => return Ok(()),
+        Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => return Ok(()),
     }
     let parsed = tokens.iter().any(|token| match token {
         Token::Whitespace(Whitespace::MultiLineComment(body)) => body.starts_with('+'),
@@ -806,7 +818,7 @@ fn words_read(dialect: Dialect, pieces: &[Piece]) -> Result<(), Refusal> {
                 Dialect::MySql => {
                     return Err(Refusal::NotAllowed("An assignment to a variable".into()));
                 }
-                Dialect::Postgres | Dialect::Sqlite => continue,
+                Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => continue,
             },
             Piece::Open | Piece::Other => continue,
         };
@@ -869,7 +881,7 @@ fn pragma_table(dialect: Dialect, name: &str) -> bool {
         Dialect::Sqlite => name
             .get(..7)
             .is_some_and(|start| start.eq_ignore_ascii_case("pragma_")),
-        Dialect::Postgres | Dialect::MySql => false,
+        Dialect::Postgres | Dialect::MySql | Dialect::DuckDb => false,
     }
 }
 
@@ -986,6 +998,20 @@ fn effects(dialect: Dialect) -> (&'static [&'static str], &'static [&'static str
             // parenthesis to be read.
             &[],
         ),
+        // The fence keeps every file but the connection's out of reach,
+        // and the session's settings locked: what is left runs SQL held in
+        // a string, or changes what the session answers next.
+        Dialect::DuckDb => (
+            &[
+                "query",
+                "query_table",
+                "setseed",
+                "nextval",
+                "currval",
+                "getvariable",
+            ],
+            &[],
+        ),
     }
 }
 
@@ -999,7 +1025,7 @@ fn named_placeholder(dialect: Dialect, sign: char, next: Option<&Token>) -> Opti
     };
     let named = match (dialect, sign) {
         (Dialect::MySql | Dialect::Sqlite, ':') | (Dialect::Sqlite, '@') => true,
-        (Dialect::Postgres | Dialect::MySql | Dialect::Sqlite, _) => false,
+        (Dialect::Postgres | Dialect::MySql | Dialect::Sqlite | Dialect::DuckDb, _) => false,
     };
     named.then(|| format!("{sign}{}", name.value))
 }
@@ -1018,7 +1044,9 @@ fn params_match(
                 .strip_prefix('$')
                 .and_then(|number| number.parse::<usize>().ok())
                 .filter(|number| *number > 0),
-            Dialect::MySql | Dialect::Sqlite => (written == "?").then_some(expected + 1),
+            Dialect::MySql | Dialect::Sqlite | Dialect::DuckDb => {
+                (written == "?").then_some(expected + 1)
+            }
         };
         let Some(position) = position else {
             return Err(Refusal::InvalidParams(ParamFault::Placeholder {
```

```diff
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -817,8 +817,9 @@ impl Hidden {
         // On PostgreSQL a function is called as a column of a row, `b.f`
         // as `f(b)`, by every lookup that would find `f(b)`: a name after a
         // dot that is a function the users wrote is taken for one.
+        // DuckDB calls one so too: `name.upper()` is `upper(name)`.
         match self.dialect {
-            Dialect::Postgres => {
+            Dialect::Postgres | Dialect::DuckDb => {
                 for name in &names.after_dot {
                     self.check_function(name)?;
                 }
@@ -1164,6 +1165,14 @@ fn system_catalog(dialect: Dialect, schema: Option<&str>, name: &str) -> bool {
         // SQLite's own tables are `sqlite_` ones, in any schema, and
         // `dbstat` describes every table's pages.
         Dialect::Sqlite => fold(name).starts_with("sqlite_") || same(name, "dbstat"),
+        // DuckDB's catalog is its `duckdb_` functions and views, and the
+        // two schemas it keeps for PostgreSQL's and the standard's names.
+        Dialect::DuckDb => {
+            in_schema(&["information_schema", "pg_catalog"])
+                || fold(name).starts_with("duckdb_")
+                || fold(name).starts_with("pragma_")
+                || same(name, "sqlite_master")
+        }
     }
 }
 
```

```diff
diff --git a/crates/tabletist-db/src/agent/hidden/masked.rs b/crates/tabletist-db/src/agent/hidden/masked.rs
--- a/crates/tabletist-db/src/agent/hidden/masked.rs
+++ b/crates/tabletist-db/src/agent/hidden/masked.rs
@@ -176,6 +176,7 @@ fn row_id(dialect: crate::Dialect, word: &str) -> bool {
     let names: &[&str] = match dialect {
         crate::Dialect::Sqlite => &["rowid", "_rowid_", "oid"],
         crate::Dialect::MySql => &["_rowid"],
+        crate::Dialect::DuckDb => &["rowid"],
         crate::Dialect::Postgres => &[],
     };
     names.iter().any(|name| name.eq_ignore_ascii_case(word))
@@ -538,10 +539,10 @@ pub(super) fn check(read: &Read, masks: &Masks<'_>) -> Result<(), Refusal> {
 }
 
 /// Whether the dialect reads a string as a name where only a name can
-/// stand: SQLite's does.
+/// stand: SQLite's does, and DuckDB reads `FROM 'orders.csv'` as a file.
 fn strings_are_names(dialect: crate::Dialect) -> bool {
     match dialect {
-        crate::Dialect::Sqlite => true,
+        crate::Dialect::Sqlite | crate::Dialect::DuckDb => true,
         crate::Dialect::Postgres | crate::Dialect::MySql => false,
     }
 }
```

```diff
diff --git a/crates/tabletist-db/src/catalog.rs b/crates/tabletist-db/src/catalog.rs
--- a/crates/tabletist-db/src/catalog.rs
+++ b/crates/tabletist-db/src/catalog.rs
@@ -157,6 +157,9 @@ pub struct Structure {
     /// The comment on the table or view itself, where its author wrote
     /// one. SQLite keeps none.
     pub comment: Option<String>,
+    /// The file a Files connection's table is read from; `None` for every
+    /// other table.
+    pub file: Option<crate::FileFacts>,
 }
 
 impl Structure {
```

```diff
diff --git a/crates/tabletist-db/src/class.rs b/crates/tabletist-db/src/class.rs
--- a/crates/tabletist-db/src/class.rs
+++ b/crates/tabletist-db/src/class.rs
@@ -71,6 +71,7 @@ pub fn column_class(dialect: Dialect, type_name: &str) -> ColumnClass {
         Dialect::Sqlite => sqlite(&name),
         Dialect::Postgres => postgres(&name),
         Dialect::MySql => mysql(&name),
+        Dialect::DuckDb => duckdb(&name),
     }
 }
 
@@ -169,6 +170,35 @@ fn postgres(name: &str) -> ColumnClass {
     }
 }
 
+/// DuckDB's names, as `DESCRIBE` gives them (`INTEGER`, `DECIMAL(18,3)`,
+/// `VARCHAR`, `TIMESTAMP WITH TIME ZONE`), and their aliases.
+fn duckdb(name: &str) -> ColumnClass {
+    // A list or an array of anything is DuckDB's to judge, as a struct, a
+    // map or a union is below.
+    if name.ends_with(']') {
+        return ColumnClass::Other;
+    }
+    match base(name) {
+        "tinyint" | "int1" => signed(1),
+        "smallint" | "int2" | "short" => signed(2),
+        "integer" | "int4" | "int" | "signed" => signed(4),
+        "bigint" | "int8" | "long" => signed(8),
+        "hugeint" => signed(16),
+        "utinyint" => unsigned(1),
+        "usmallint" => unsigned(2),
+        "uinteger" => unsigned(4),
+        "ubigint" => unsigned(8),
+        "uhugeint" => unsigned(16),
+        "decimal" | "numeric" => decimal(name),
+        "float" | "real" | "float4" | "double" | "float8" => ColumnClass::Float,
+        "boolean" | "bool" | "logical" => ColumnClass::Boolean,
+        "json" => ColumnClass::Json,
+        "varchar" | "text" | "string" | "char" | "bpchar" => ColumnClass::Text { max_chars: None },
+        "blob" | "bytea" | "binary" | "varbinary" => ColumnClass::Binary,
+        _ => ColumnClass::Other,
+    }
+}
+
 fn mysql(name: &str) -> ColumnClass {
     // A boolean is a tinyint of display width one.
     if name == "tinyint(1)" {
@@ -299,6 +329,42 @@ mod tests {
         }
     }
 
+    #[test]
+    fn duckdb_types_have_classes() {
+        for (name, class) in [
+            ("INTEGER", int(i128::from(i32::MIN), i128::from(i32::MAX))),
+            ("BIGINT", int(i128::from(i64::MIN), i128::from(i64::MAX))),
+            ("UTINYINT", int(0, 255)),
+            ("TINYINT", int(-128, 127)),
+            (
+                "DECIMAL(18,3)",
+                Decimal {
+                    precision: Some(18),
+                    scale: Some(3),
+                },
+            ),
+            ("DOUBLE", Float),
+            ("BOOLEAN", Boolean),
+            ("JSON", Json),
+            ("VARCHAR", Text { max_chars: None }),
+            ("BLOB", Binary),
+            ("DATE", Other),
+            ("TIMESTAMP WITH TIME ZONE", Other),
+            ("INTEGER[]", Other),
+            ("STRUCT(id INTEGER)", Other),
+            ("MAP(VARCHAR, INTEGER)", Other),
+        ] {
+            assert_eq!(column_class(Dialect::DuckDb, name), class, "{name}");
+        }
+        assert_eq!(
+            temporal("TIMESTAMP WITH TIME ZONE"),
+            Some(Temporal::Timestamp)
+        );
+        assert_eq!(temporal("TIMESTAMP_NS"), Some(Temporal::Timestamp));
+        assert_eq!(temporal("TIME"), Some(Temporal::Time));
+        assert_eq!(temporal("DATE[]"), None);
+    }
+
     #[test]
     fn postgres_types_have_classes() {
         for (name, class) in [
```

```diff
diff --git a/crates/tabletist-db/src/ddl.rs b/crates/tabletist-db/src/ddl.rs
--- a/crates/tabletist-db/src/ddl.rs
+++ b/crates/tabletist-db/src/ddl.rs
@@ -86,7 +86,8 @@ impl Ddl {
     pub fn source(&self) -> Source {
         match self.driver {
             Driver::Postgres => Source::Catalog,
-            Driver::MySql | Driver::Sqlite => Source::Stored,
+            // DuckDB keeps a view's statement as it was made.
+            Driver::MySql | Driver::Sqlite | Driver::Files => Source::Stored,
         }
     }
 
@@ -119,7 +120,7 @@ impl Ddl {
         }
         let sections = match self.driver {
             Driver::Postgres | Driver::MySql => true,
-            Driver::Sqlite => false,
+            Driver::Sqlite | Driver::Files => false,
         };
         let mut written = false;
         for part in &self.parts {
```

```diff
diff --git a/crates/tabletist-db/src/dialect.rs b/crates/tabletist-db/src/dialect.rs
--- a/crates/tabletist-db/src/dialect.rs
+++ b/crates/tabletist-db/src/dialect.rs
@@ -12,6 +12,9 @@ pub enum Dialect {
     Postgres,
     MySql,
     Sqlite,
+    /// DuckDB's, for Files connections. Its parser is PostgreSQL's: it
+    /// quotes, comments and nests strings as PostgreSQL does.
+    DuckDb,
 }
 
 /// SQL text with its bound parameters.
@@ -259,7 +262,9 @@ impl Dialect {
     pub fn quote_ident(self, ident: &str) -> String {
         match self {
             Self::MySql => format!("`{}`", ident.replace('`', "``")),
-            Self::Postgres | Self::Sqlite => format!("\"{}\"", ident.replace('"', "\"\"")),
+            Self::Postgres | Self::Sqlite | Self::DuckDb => {
+                format!("\"{}\"", ident.replace('"', "\"\""))
+            }
         }
     }
 
@@ -270,7 +275,9 @@ impl Dialect {
     /// a quoted name always works.
     pub fn ident(self, name: &str) -> std::borrow::Cow<'_, str> {
         let word = |byte: u8| match self {
-            Self::Postgres => byte.is_ascii_lowercase() || byte == b'_',
+            // DuckDB keeps a bare name's case and finds it in any case:
+            // lower case reads the same in both.
+            Self::Postgres | Self::DuckDb => byte.is_ascii_lowercase() || byte == b'_',
             Self::MySql | Self::Sqlite => byte.is_ascii_alphabetic() || byte == b'_',
         };
         let mut bytes = name.bytes();
@@ -299,21 +306,22 @@ impl Dialect {
     fn as_text(self, column: &str) -> String {
         match self {
             Self::MySql => format!("CAST({column} AS CHAR)"),
-            Self::Postgres | Self::Sqlite => format!("CAST({column} AS TEXT)"),
+            Self::Postgres | Self::Sqlite | Self::DuckDb => format!("CAST({column} AS TEXT)"),
         }
     }
 
     fn like(self) -> &'static str {
         match self {
-            Self::Postgres => "ILIKE",
+            Self::Postgres | Self::DuckDb => "ILIKE",
             Self::MySql | Self::Sqlite => "LIKE",
         }
     }
 
-    /// PostgreSQL and MySQL already treat `\` as LIKE's escape character.
+    /// PostgreSQL and MySQL already treat `\` as LIKE's escape character;
+    /// SQLite and DuckDB have none until they are told.
     fn like_escape(self) -> &'static str {
         match self {
-            Self::Sqlite => " ESCAPE '\\'",
+            Self::Sqlite | Self::DuckDb => " ESCAPE '\\'",
             Self::Postgres | Self::MySql => "",
         }
     }
@@ -337,8 +345,8 @@ impl Dialect {
     fn bind(self, binds: &mut Binds, value: String) -> String {
         match self {
             Self::Postgres => quote_literal(&value),
-            Self::MySql | Self::Sqlite if binds.inline => self.literal(&value),
-            Self::MySql | Self::Sqlite => {
+            Self::MySql | Self::Sqlite | Self::DuckDb if binds.inline => self.literal(&value),
+            Self::MySql | Self::Sqlite | Self::DuckDb => {
                 binds.params.push(Value::Text(value.into()));
                 self.placeholder().to_owned()
             }
@@ -356,8 +364,8 @@ impl Dialect {
                 }
                 quote_literal(&hex)
             }
-            Self::MySql | Self::Sqlite if binds.inline => self.bytes_literal(&bytes),
-            Self::MySql | Self::Sqlite => {
+            Self::MySql | Self::Sqlite | Self::DuckDb if binds.inline => self.bytes_literal(&bytes),
+            Self::MySql | Self::Sqlite | Self::DuckDb => {
                 binds.params.push(Value::Bytes(bytes.into()));
                 self.placeholder().to_owned()
             }
@@ -457,7 +465,10 @@ impl Dialect {
     pub(crate) fn literal(self, text: &str) -> String {
         match self {
             Self::Postgres if text.contains('\\') => quote_literal(text),
-            Self::Postgres | Self::Sqlite => format!("'{}'", text.replace('\'', "''")),
+            // DuckDB's strings are standard: a backslash is itself.
+            Self::Postgres | Self::Sqlite | Self::DuckDb => {
+                format!("'{}'", text.replace('\'', "''"))
+            }
             Self::MySql => format!(
                 "'{}'",
                 text.replace('\\', "\\\\")
@@ -467,7 +478,8 @@ impl Dialect {
         }
     }
 
-    /// Bytes as a literal: PostgreSQL's hex `bytea`, `x'..'` elsewhere.
+    /// Bytes as a literal: PostgreSQL's hex `bytea`, DuckDB's escaped
+    /// `BLOB`, `x'..'` elsewhere.
     pub(crate) fn bytes_literal(self, bytes: &[u8]) -> String {
         let mut hex = String::new();
         for byte in bytes {
@@ -476,6 +488,13 @@ impl Dialect {
         match self {
             Self::Postgres => quote_literal(&format!("\\x{hex}")),
             Self::MySql | Self::Sqlite => format!("x'{hex}'"),
+            Self::DuckDb => {
+                let mut escaped = String::new();
+                for byte in bytes {
+                    let _ = write!(escaped, "\\x{byte:02X}");
+                }
+                format!("'{escaped}'::BLOB")
+            }
         }
     }
 
@@ -515,9 +534,11 @@ impl Dialect {
     /// that keeps `kind`.
     fn now(self, kind: Temporal) -> &'static str {
         match (self, kind) {
-            (Self::Postgres | Self::MySql | Self::Sqlite, Temporal::Date) => "CURRENT_DATE",
-            (Self::Postgres | Self::Sqlite, Temporal::Time) => "CURRENT_TIME",
-            (Self::Postgres, Temporal::Timestamp) => "now()",
+            (Self::Postgres | Self::MySql | Self::Sqlite | Self::DuckDb, Temporal::Date) => {
+                "CURRENT_DATE"
+            }
+            (Self::Postgres | Self::Sqlite | Self::DuckDb, Temporal::Time) => "CURRENT_TIME",
+            (Self::Postgres | Self::DuckDb, Temporal::Timestamp) => "now()",
             // To the microsecond: without the precision MySQL gives whole
             // seconds, whatever the column keeps.
             (Self::MySql, Temporal::Time) => "CURRENT_TIME(6)",
@@ -557,7 +578,11 @@ impl Dialect {
             (Self::Postgres, Value::Bool(flag)) => Operand::Text(flag.to_string()),
             (Self::Postgres, Value::Float(number)) => Operand::Text(number.to_string()),
             (Self::MySql | Self::Sqlite, Value::Bool(flag)) => Operand::Int(i64::from(*flag)),
-            (Self::MySql | Self::Sqlite, Value::Float(number)) => Operand::Float(*number),
+            (Self::MySql | Self::Sqlite | Self::DuckDb, Value::Float(number)) => {
+                Operand::Float(*number)
+            }
+            // DuckDB casts the text to the column's boolean.
+            (Self::DuckDb, Value::Bool(flag)) => Operand::Text(flag.to_string()),
         }
     }
 
@@ -599,7 +624,7 @@ impl Dialect {
             NewValue::Null => return Ok(Operand::Null),
             NewValue::Default { expression } => {
                 return Ok(Operand::Raw(match self {
-                    Self::Postgres | Self::MySql => "DEFAULT".to_owned(),
+                    Self::Postgres | Self::MySql | Self::DuckDb => "DEFAULT".to_owned(),
                     // SQLite has no DEFAULT to write in an UPDATE or among
                     // VALUES: the column's own expression, which is what
                     // the word would have come to, or NULL where it has
@@ -781,13 +806,15 @@ impl Dialect {
     pub fn insert_row(self, object: &ObjectRef, row: &RowInsert) -> Result<InsertStatement> {
         let table = self.qualified(object);
         let back = match self {
-            Self::Postgres | Self::Sqlite => " RETURNING *",
+            Self::Postgres | Self::Sqlite | Self::DuckDb => " RETURNING *",
             Self::MySql => "",
         };
         let lead = format!("INSERT INTO {table}");
         if row.set.is_empty() {
             let (text, values) = match self {
-                Self::Postgres | Self::Sqlite => (format!("{lead} DEFAULT VALUES"), lead.len() + 1),
+                Self::Postgres | Self::Sqlite | Self::DuckDb => {
+                    (format!("{lead} DEFAULT VALUES"), lead.len() + 1)
+                }
                 Self::MySql => (format!("{lead} () VALUES ()"), lead.len() + 4),
             };
             let parts = InsertParts {
@@ -839,7 +866,7 @@ impl Dialect {
         // in a message: see `update_row`.
         let refused = match self {
             Self::Postgres => text.contains('\0'),
-            Self::MySql | Self::Sqlite => false,
+            Self::MySql | Self::Sqlite | Self::DuckDb => false,
         };
         if refused {
             return Err(Error::query("PostgreSQL text cannot hold a NUL character"));
@@ -879,7 +906,7 @@ impl Dialect {
         // in a message: see `update_row`.
         let refused = match self {
             Self::Postgres => text.contains('\0'),
-            Self::MySql | Self::Sqlite => false,
+            Self::MySql | Self::Sqlite | Self::DuckDb => false,
         };
         if refused {
             return Err(Error::query("PostgreSQL text cannot hold a NUL character"));
@@ -938,9 +965,9 @@ impl Dialect {
         // A list of rows on the right of `IN`: SQLite takes one only as
         // `VALUES`.
         let open = match (self, several) {
-            (Self::Postgres | Self::MySql | Self::Sqlite, false) => " IN (",
+            (Self::Postgres | Self::MySql | Self::Sqlite | Self::DuckDb, false) => " IN (",
             (Self::Postgres | Self::MySql, true) => ") IN (",
-            (Self::Sqlite, true) => ") IN (VALUES ",
+            (Self::Sqlite | Self::DuckDb, true) => ") IN (VALUES ",
         };
         shown.push_str(open);
         sent.push_str(open);
@@ -974,7 +1001,7 @@ impl Dialect {
         // PostgreSQL text cannot hold a NUL: see `delete_row`.
         let refused = match self {
             Self::Postgres => sent.contains('\0'),
-            Self::MySql | Self::Sqlite => false,
+            Self::MySql | Self::Sqlite | Self::DuckDb => false,
         };
         if refused {
             return Err(Error::query("PostgreSQL text cannot hold a NUL character"));
@@ -1006,6 +1033,9 @@ impl Dialect {
                 .as_deref()
                 .and_then(|code| code.parse::<u32>().ok())
                 .is_some_and(|code| matches!(code & 0xff, 5 | 6)),
+            // One session in the app's own process: nothing else holds a
+            // lock it could wait for.
+            Self::DuckDb => false,
         }
     }
 
@@ -1017,11 +1047,12 @@ impl Dialect {
     pub fn select_row(self, object: &ObjectRef, key: &[(String, Value)], lock: bool) -> Sql {
         let mut params = Vec::new();
         let clause = self.key_clause(key, &mut params).sent;
-        let lock = if lock && self != Self::Sqlite {
-            " FOR UPDATE"
-        } else {
-            ""
+        // SQLite and DuckDB have no row locks: one writer at a time.
+        let row_locks = match self {
+            Self::Postgres | Self::MySql => true,
+            Self::Sqlite | Self::DuckDb => false,
         };
+        let lock = if lock && row_locks { " FOR UPDATE" } else { "" };
         Sql {
             text: format!(
                 "SELECT * FROM {}{clause} LIMIT 2{lock}",
@@ -2078,7 +2109,7 @@ mod tests {
             }
             let bound = match dialect {
                 Dialect::Postgres => 0,
-                Dialect::MySql | Dialect::Sqlite => 1,
+                Dialect::MySql | Dialect::Sqlite | Dialect::DuckDb => 1,
             };
             assert_eq!(update.sql.params.len(), bound, "{dialect:?}");
             // A new row's values are written the same way.
@@ -2161,7 +2192,7 @@ mod tests {
             let after = &shown[parts.literals[1].end..];
             let back = parts.back.map(|at| &shown[at..]);
             match dialect {
-                Dialect::Postgres | Dialect::Sqlite => {
+                Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => {
                     assert_eq!(after, ") RETURNING *");
                     assert_eq!(back, Some("RETURNING *"));
                 }
@@ -2177,7 +2208,7 @@ mod tests {
             let clause = &shown[parts.values..];
             assert!(shown[..parts.values].ends_with(' '), "{shown}");
             match dialect {
-                Dialect::Postgres | Dialect::Sqlite => {
+                Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => {
                     assert_eq!(clause, "DEFAULT VALUES RETURNING *");
                     assert_eq!(parts.back.map(|at| &shown[at..]), Some("RETURNING *"));
                 }
```

```diff
diff --git a/crates/tabletist-db/src/duck.rs b/crates/tabletist-db/src/duck.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-db/src/duck.rs
@@ -0,0 +1,1286 @@
+//! Files connections: DuckDB in the app's process, in memory, with a view
+//! for each file and each `.duckdb` file attached as a schema. DuckDB is
+//! blocking, so every call runs on tokio's blocking pool, as SQLite's do.
+//!
+//! The session is fenced twice. DuckDB's own configuration, set once the
+//! views are made and locked so no SQL can set it back: it may open the
+//! connection's files and no other, which also keeps it from writing a
+//! file, installing or loading an extension. And a read-only transaction
+//! around every page, count, lookup, script and agent's read.
+
+use std::path::Path;
+use std::sync::atomic::{AtomicBool, Ordering};
+use std::sync::{Arc, Mutex};
+use std::time::{Duration, Instant};
+
+use duckdb::arrow::array::{Array, AsArray, RecordBatch};
+use duckdb::arrow::datatypes::{DataType, Field, Float64Type, Int64Type, UInt64Type};
+use duckdb::arrow::util::display::{ArrayFormatter, FormatOptions};
+
+use crate::adapter::Adapter;
+use crate::agent::{Catalog, ColumnName, Limits, Param, Read, Refusal, Sources};
+use crate::files::{Format, Found, Source};
+use crate::{
+    CancelHandle, CancelInner, ChangeSet, ColumnInfo, ColumnMeta, Ddl, Dialect, Error, Lookup,
+    LookupPage, MAX_LISTED, ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery,
+    ScriptMode, ScriptOutcome, StatementOutcome, StatementResult, StopFlag, Structure, Value,
+    ValueKind, WriteOutcome,
+};
+
+/// The schema a connection's files are tables of.
+pub(crate) const MAIN: &str = "main";
+
+/// DuckDB's name for the in-memory database the views are in.
+const MEMORY: &str = "memory";
+
+/// How many rows Test reads of each file, and how many rows of a CSV the
+/// reader samples for its columns' types.
+pub(crate) const SAMPLE_ROWS: u32 = 20_000;
+
+/// Where DuckDB keeps the extensions it installs: the app's own folder,
+/// set once at start. Unset (in tests), DuckDB's own default.
+static EXTENSIONS: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
+
+/// Sets the folder DuckDB installs extensions into, for every Files
+/// connection opened after it. Only the first call counts.
+pub fn set_extension_directory(folder: std::path::PathBuf) {
+    let _ = EXTENSIONS.set(folder);
+}
+
+/// What a Test of a Files connection read.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct FilesRead {
+    /// The files read, `.duckdb` ones included.
+    pub files: usize,
+    /// Their size on disk, in bytes.
+    pub bytes: u64,
+}
+
+/// What the Structure view says of the file a table is read from.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct FileFacts {
+    pub path: std::path::PathBuf,
+    pub size: u64,
+    pub format: Format,
+    /// How the reader is told to read it, as the view's call says it:
+    /// `header = true, auto_detect = true, sample_size = 20000`.
+    pub options: String,
+}
+
+/// An open Files connection.
+pub(crate) struct Conn {
+    inner: Arc<Mutex<duckdb::Connection>>,
+    interrupt: Arc<duckdb::InterruptHandle>,
+    /// The call whose work holds the session now (0 for none), so that a
+    /// call given up on interrupts its own statement and no other's.
+    running: Arc<std::sync::atomic::AtomicU64>,
+    /// The number the next call takes.
+    calls: std::sync::atomic::AtomicU64,
+    files: Arc<Vec<Opened>>,
+    version: String,
+}
+
+/// A file as the session opened it.
+#[derive(Debug, Clone)]
+struct Opened {
+    found: Found,
+    /// The statement that made its view, which the DDL view shows: DuckDB
+    /// keeps it rewritten, each option in parentheses.
+    made: String,
+    /// Rows, where the file says (Parquet's metadata) or a sample of it
+    /// lets them be estimated.
+    rows: Option<u64>,
+}
+
+/// Maps DuckDB's errors onto ours. DuckDB starts its message with its kind
+/// (`Binder Error: …`, `Permission Error: …`), which stays.
+pub(crate) fn map_error(error: duckdb::Error) -> Error {
+    match &error {
+        // DuckDB's own kind for a statement it was told to stop, at the
+        // start of its words: a word "Interrupted" a message quotes is not.
+        duckdb::Error::DuckDBFailure(_, Some(message))
+            if message.starts_with("INTERRUPT Error") =>
+        {
+            Error::Cancelled
+        }
+        duckdb::Error::DuckDBFailure(_, message) => Error::query(
+            message
+                .clone()
+                .unwrap_or_else(|| error.to_string())
+                .trim_end()
+                .to_owned(),
+        ),
+        _ => Error::query(error.to_string()),
+    }
+}
+
+/// A string literal in DuckDB's SQL: its strings are standard, so a quote
+/// is doubled and a backslash is itself.
+fn literal(text: &str) -> String {
+    Dialect::DuckDb.literal(text)
+}
+
+/// A path as DuckDB is given it.
+fn path_text(path: &Path) -> Result<String> {
+    path.to_str().map(str::to_owned).ok_or_else(|| {
+        Error::Connect(format!(
+            "Can't open {}: the path is not valid UTF-8",
+            path.display()
+        ))
+    })
+}
+
+/// The reader's options for a file of `format`, as its call writes them
+/// after the path.
+fn reader_options(format: Format) -> &'static str {
+    match format {
+        Format::Csv => "header = true, auto_detect = true, sample_size = 20000",
+        Format::Tsv => "header = true, auto_detect = true, sample_size = 20000, delim = '\\t'",
+        Format::Json => "format = 'array'",
+        Format::Ndjson => "format = 'newline_delimited'",
+        Format::Excel => "header = true",
+        Format::Parquet | Format::DuckDb => "",
+    }
+}
+
+/// The table function that reads a file of `format`.
+fn reader(format: Format) -> &'static str {
+    match format {
+        Format::Csv | Format::Tsv => "read_csv",
+        Format::Parquet => "read_parquet",
+        Format::Json | Format::Ndjson => "read_json",
+        Format::Excel => "read_xlsx",
+        // Attached, not read: never asked.
+        Format::DuckDb => "",
+    }
+}
+
+/// The statement that makes `file` a table or a schema of the session,
+/// its name bare where that is safe: the DDL view shows it as it is.
+fn make(file: &Found) -> Result<String> {
+    let path = literal(&path_text(&file.path)?);
+    let name = Dialect::DuckDb.ident(&file.name);
+    Ok(match file.format {
+        Format::DuckDb => format!("ATTACH {path} AS {name} (READ_ONLY)"),
+        format => {
+            let options = reader_options(format);
+            let call = match options {
+                "" => format!("{}({path})", reader(format)),
+                options => format!("{}({path}, {options})", reader(format)),
+            };
+            format!("CREATE VIEW {name} AS SELECT * FROM {call}")
+        }
+    })
+}
+
+/// Rows of a text file by a sample of its start: its lines in the first
+/// 256 KiB, scaled to its size, without a CSV's header. `None` for a
+/// format that is not a line a row, or a file that cannot be read.
+fn estimate(file: &Found) -> Option<u64> {
+    use std::io::Read as _;
+    let header = match file.format {
+        Format::Csv | Format::Tsv => 1,
+        Format::Ndjson => 0,
+        Format::Parquet | Format::Json | Format::Excel | Format::DuckDb => return None,
+    };
+    let mut start = Vec::with_capacity(256 * 1024);
+    std::fs::File::open(&file.path)
+        .ok()?
+        .take(256 * 1024)
+        .read_to_end(&mut start)
+        .ok()?;
+    if start.is_empty() {
+        return Some(0);
+    }
+    let mut lines = start.iter().filter(|byte| **byte == b'\n').count() as u64;
+    if start.last() != Some(&b'\n') {
+        lines += 1;
+    }
+    let whole = start.len() as u64 >= file.size;
+    let rows = match whole {
+        true => lines,
+        false => {
+            let scaled = (lines as f64) * (file.size as f64) / (start.len() as f64);
+            scaled.round() as u64
+        }
+    };
+    Some(rows.saturating_sub(header))
+}
+
+/// Opens a session over `files`: views and attaches, then the fence.
+fn opened(files: &[Found]) -> Result<(duckdb::Connection, Vec<Opened>, String)> {
+    let config = duckdb::Config::default()
+        .enable_autoload_extension(false)
+        .map_err(map_error)?;
+    let connection = duckdb::Connection::open_in_memory_with_flags(config).map_err(map_error)?;
+    let run = |sql: &str| connection.execute_batch(sql).map_err(map_error);
+    if let Some(folder) = EXTENSIONS.get() {
+        run(&format!(
+            "SET extension_directory = {}",
+            literal(&path_text(folder)?)
+        ))?;
+    }
+    if files.iter().any(|file| file.format == Format::Excel) {
+        // Installed once, then found in the folder: offline, a second open
+        // still reads Excel files.
+        run("INSTALL excel; LOAD excel").map_err(|error| {
+            Error::Connect(format!(
+                "Can't read Excel files: the Excel reader for DuckDB could not be installed \
+                 ({error})"
+            ))
+        })?;
+    }
+    let mut opened = Vec::with_capacity(files.len());
+    for file in files {
+        // A `.duckdb` file is attached read-only, as every session is.
+        let statement = make(file)?;
+        run(&statement).map_err(|error| {
+            Error::Connect(format!(
+                "Can't read {}: {}",
+                file_label(file),
+                plain(&error)
+            ))
+        })?;
+        let rows = match file.format {
+            Format::Parquet => connection
+                .query_row(
+                    &format!(
+                        "SELECT sum(num_rows)::BIGINT FROM parquet_file_metadata({})",
+                        literal(&path_text(&file.path)?)
+                    ),
+                    [],
+                    |row| row.get::<_, Option<i64>>(0),
+                )
+                .ok()
+                .flatten()
+                .and_then(|rows| u64::try_from(rows).ok()),
+            Format::Csv
+            | Format::Tsv
+            | Format::Json
+            | Format::Ndjson
+            | Format::Excel
+            | Format::DuckDb => estimate(file),
+        };
+        opened.push(Opened {
+            found: file.clone(),
+            made: statement,
+            rows,
+        });
+    }
+    // The fence: these files and no other, then no setting changed again.
+    let allowed: Vec<String> = files
+        .iter()
+        .map(|file| path_text(&file.path).map(|path| literal(&path)))
+        .collect::<Result<_>>()?;
+    run(&format!("SET allowed_paths = [{}]", allowed.join(", ")))?;
+    run("SET enable_external_access = false")?;
+    run("SET lock_configuration = true")?;
+    let version: String = connection
+        .query_row("SELECT version()", [], |row| row.get(0))
+        .map_err(map_error)?;
+    Ok((
+        connection,
+        opened,
+        format!("DuckDB {}", version.trim_start_matches('v')),
+    ))
+}
+
+/// Stops call `call`'s work when dropped, unless disarmed first: a dropped
+/// future (a closed tab) stops its statement, or the work before it starts.
+/// The work of another call, waiting behind it or holding the session, is
+/// left alone.
+struct InterruptOnDrop {
+    interrupt: Arc<duckdb::InterruptHandle>,
+    running: Arc<std::sync::atomic::AtomicU64>,
+    given_up: Arc<AtomicBool>,
+    call: u64,
+    armed: bool,
+}
+
+impl InterruptOnDrop {
+    fn disarm(mut self) {
+        self.armed = false;
+    }
+}
+
+impl Drop for InterruptOnDrop {
+    fn drop(&mut self) {
+        if !self.armed {
+            return;
+        }
+        self.given_up.store(true, Ordering::SeqCst);
+        if self.running.load(Ordering::SeqCst) == self.call {
+            self.interrupt.interrupt();
+        }
+    }
+}
+
+/// Interrupts the session's statement once `stop` is set or `deadline`
+/// passes, from a thread of its own: DuckDB has no handler it calls while
+/// a statement runs. Ends with the run.
+struct Watch {
+    done: Arc<AtomicBool>,
+    thread: Option<std::thread::JoinHandle<()>>,
+}
+
+impl Watch {
+    fn start(
+        interrupt: Arc<duckdb::InterruptHandle>,
+        stop: StopFlag,
+        deadline: Option<Instant>,
+    ) -> Self {
+        let done = Arc::new(AtomicBool::new(false));
+        let ended = Arc::clone(&done);
+        let thread = std::thread::spawn(move || {
+            while !ended.load(Ordering::SeqCst) {
+                if stop.is_stopped() || deadline.is_some_and(|at| Instant::now() >= at) {
+                    interrupt.interrupt();
+                    return;
+                }
+                std::thread::sleep(Duration::from_millis(5));
+            }
+        });
+        Self {
+            done,
+            thread: Some(thread),
+        }
+    }
+}
+
+impl Drop for Watch {
+    fn drop(&mut self) {
+        self.done.store(true, Ordering::SeqCst);
+        if let Some(thread) = self.thread.take() {
+            let _ = thread.join();
+        }
+    }
+}
+
+/// Runs `work` in a read-only transaction, which is rolled back after it
+/// whatever it did.
+fn read_only<T>(
+    connection: &duckdb::Connection,
+    work: impl FnOnce(&duckdb::Connection) -> Result<T>,
+) -> Result<T> {
+    connection
+        .execute_batch("BEGIN TRANSACTION READ ONLY")
+        .map_err(map_error)?;
+    let result = work(connection);
+    let ended = connection.execute_batch("ROLLBACK").map_err(map_error);
+    let value = result?;
+    ended?;
+    Ok(value)
+}
+
+/// A value an agent sent, as DuckDB binds it.
+fn bound(param: &Param) -> duckdb::types::Value {
+    match param {
+        Param::Null => duckdb::types::Value::Null,
+        Param::Bool(flag) => duckdb::types::Value::Boolean(*flag),
+        Param::Int(number) => duckdb::types::Value::BigInt(*number),
+        Param::Float(number) => duckdb::types::Value::Double(*number),
+        Param::Text(text) => duckdb::types::Value::Text(text.clone()),
+    }
+}
+
+/// One of ours, as DuckDB binds it.
+fn to_duckdb(value: &Value) -> duckdb::types::Value {
+    match value {
+        Value::Null => duckdb::types::Value::Null,
+        Value::Bool(flag) => duckdb::types::Value::Boolean(*flag),
+        Value::Int(number) => duckdb::types::Value::BigInt(*number),
+        Value::Float(number) => duckdb::types::Value::Double(*number),
+        Value::Text(text) => duckdb::types::Value::Text(text.to_string()),
+        Value::Bytes(bytes) => duckdb::types::Value::Blob(bytes.to_vec()),
+    }
+}
+
+/// DuckDB's name for a column of `field`, and how the grid treats it.
+fn column_meta(field: &Field) -> ColumnMeta {
+    let json = field
+        .metadata()
+        .get("ARROW:extension:name")
+        .is_some_and(|name| name.ends_with("json"));
+    let (type_name, kind) = match field.data_type() {
+        _ if json => ("JSON".to_owned(), ValueKind::Json),
+        DataType::Boolean => ("BOOLEAN".to_owned(), ValueKind::Bool),
+        DataType::Int8 => ("TINYINT".to_owned(), ValueKind::Numeric),
+        DataType::Int16 => ("SMALLINT".to_owned(), ValueKind::Numeric),
+        DataType::Int32 => ("INTEGER".to_owned(), ValueKind::Numeric),
+        DataType::Int64 => ("BIGINT".to_owned(), ValueKind::Numeric),
+        DataType::UInt8 => ("UTINYINT".to_owned(), ValueKind::Numeric),
+        DataType::UInt16 => ("USMALLINT".to_owned(), ValueKind::Numeric),
+        DataType::UInt32 => ("UINTEGER".to_owned(), ValueKind::Numeric),
+        DataType::UInt64 => ("UBIGINT".to_owned(), ValueKind::Numeric),
+        DataType::Float16 | DataType::Float32 => ("FLOAT".to_owned(), ValueKind::Numeric),
+        DataType::Float64 => ("DOUBLE".to_owned(), ValueKind::Numeric),
+        DataType::Decimal32(precision, scale)
+        | DataType::Decimal64(precision, scale)
+        | DataType::Decimal128(precision, scale) => {
+            (format!("DECIMAL({precision},{scale})"), ValueKind::Numeric)
+        }
+        DataType::Decimal256(precision, scale) => {
+            (format!("DECIMAL({precision},{scale})"), ValueKind::Numeric)
+        }
+        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View => {
+            ("VARCHAR".to_owned(), ValueKind::Text)
+        }
+        DataType::Binary
+        | DataType::LargeBinary
+        | DataType::BinaryView
+        | DataType::FixedSizeBinary(_) => ("BLOB".to_owned(), ValueKind::Binary),
+        DataType::Date32 | DataType::Date64 => ("DATE".to_owned(), ValueKind::Temporal),
+        DataType::Time32(_) | DataType::Time64(_) => ("TIME".to_owned(), ValueKind::Temporal),
+        DataType::Timestamp(_, None) => ("TIMESTAMP".to_owned(), ValueKind::Temporal),
+        DataType::Timestamp(_, Some(_)) => {
+            ("TIMESTAMP WITH TIME ZONE".to_owned(), ValueKind::Temporal)
+        }
+        DataType::Interval(_) | DataType::Duration(_) => {
+            ("INTERVAL".to_owned(), ValueKind::Temporal)
+        }
+        DataType::List(_)
+        | DataType::LargeList(_)
+        | DataType::ListView(_)
+        | DataType::LargeListView(_)
+        | DataType::FixedSizeList(..) => ("LIST".to_owned(), ValueKind::Other),
+        DataType::Struct(_) => ("STRUCT".to_owned(), ValueKind::Other),
+        DataType::Map(..) => ("MAP".to_owned(), ValueKind::Other),
+        DataType::Union(..) => ("UNION".to_owned(), ValueKind::Other),
+        DataType::Dictionary(..) => ("ENUM".to_owned(), ValueKind::Text),
+        DataType::Null => (String::new(), ValueKind::Other),
+        other => (other.to_string(), ValueKind::Other),
+    };
+    ColumnMeta {
+        name: field.name().clone(),
+        type_name,
+        kind,
+    }
+}
+
+/// How a value DuckDB has no plain form for is written: dates and times
+/// as PostgreSQL writes them, the rest as DuckDB's Arrow export says it.
+/// A list, a struct or a map, whose own values are written inside it.
+fn nested(data_type: &DataType) -> bool {
+    matches!(
+        data_type,
+        DataType::List(_)
+            | DataType::LargeList(_)
+            | DataType::FixedSizeList(..)
+            | DataType::Struct(_)
+            | DataType::Map(..)
+    )
+}
+
+/// How a value DuckDB has no plain form for is written: dates and times
+/// as PostgreSQL writes them, the rest as DuckDB's Arrow export says it;
+/// inside a list, a struct or a map, NULL as the word and text quoted. A
+/// value that cannot be written (a year past what Arrow keeps) is an
+/// error of its own, not text.
+fn format_options(data_type: &DataType) -> FormatOptions<'static> {
+    FormatOptions::new()
+        .with_display_error(false)
+        .with_null("NULL")
+        .with_quoted_strings(nested(data_type))
+        .with_date_format(Some("%Y-%m-%d"))
+        .with_datetime_format(Some("%Y-%m-%d %H:%M:%S%.f"))
+        .with_timestamp_format(Some("%Y-%m-%d %H:%M:%S%.f"))
+        .with_time_format(Some("%H:%M:%S%.f"))
+}
+
+/// `data_type` with every moment's zone taken off, where it holds one
+/// inside a list, a struct or a map: DuckDB keeps those in UTC, and Arrow
+/// cannot read the zone's name without the zone database.
+fn zoneless(data_type: &DataType) -> Option<DataType> {
+    use duckdb::arrow::datatypes::Fields;
+    let field = |field: &Field, inner: DataType| Arc::new(field.clone().with_data_type(inner));
+    match data_type {
+        DataType::Timestamp(unit, Some(_)) => Some(DataType::Timestamp(*unit, None)),
+        DataType::List(inner) => {
+            zoneless(inner.data_type()).map(|t| DataType::List(field(inner, t)))
+        }
+        DataType::LargeList(inner) => {
+            zoneless(inner.data_type()).map(|t| DataType::LargeList(field(inner, t)))
+        }
+        DataType::FixedSizeList(inner, size) => {
+            zoneless(inner.data_type()).map(|t| DataType::FixedSizeList(field(inner, t), *size))
+        }
+        DataType::Map(inner, sorted) => {
+            zoneless(inner.data_type()).map(|t| DataType::Map(field(inner, t), *sorted))
+        }
+        DataType::Struct(fields) => {
+            let changed: Vec<Option<DataType>> =
+                fields.iter().map(|f| zoneless(f.data_type())).collect();
+            changed.iter().any(Option::is_some).then(|| {
+                DataType::Struct(Fields::from(
+                    fields
+                        .iter()
+                        .zip(changed)
+                        .map(|(f, t)| match t {
+                            Some(t) => field(f, t),
+                            None => Arc::clone(f),
+                        })
+                        .collect::<Vec<_>>(),
+                ))
+            })
+        }
+        _ => None,
+    }
+}
+
+/// DuckDB's infinities, where `array` is of dates or moments: `infinity`
+/// and `-infinity` for the largest value and its negation, which Arrow
+/// would take for a day past what it can write.
+fn infinities(array: &dyn Array) -> Option<Vec<Option<&'static str>>> {
+    let (wide, max) = match array.data_type() {
+        DataType::Timestamp(..) => (
+            duckdb::arrow::compute::cast(array, &DataType::Int64).ok()?,
+            i64::MAX,
+        ),
+        DataType::Date32 => {
+            let days = duckdb::arrow::compute::cast(array, &DataType::Int32).ok()?;
+            (
+                duckdb::arrow::compute::cast(&days, &DataType::Int64).ok()?,
+                i64::from(i32::MAX),
+            )
+        }
+        _ => return None,
+    };
+    let values = wide.as_primitive::<Int64Type>();
+    Some(
+        (0..values.len())
+            .map(|at| match values.is_valid(at).then(|| values.value(at)) {
+                Some(value) if value == max => Some("infinity"),
+                Some(value) if value == -max => Some("-infinity"),
+                _ => None,
+            })
+            .collect(),
+    )
+}
+
+/// The values of one column of a batch, in its rows' order.
+fn column_values(array: &dyn Array) -> Result<Vec<Value>> {
+    let rows = array.len();
+    let null = |at: usize| array.is_null(at);
+    let infinite = infinities(array);
+    let text_of = |shown: &dyn Array, suffix: &str| -> Result<Vec<Value>> {
+        let options = format_options(shown.data_type());
+        let formatter = ArrayFormatter::try_new(shown, &options)
+            .map_err(|error| Error::query(error.to_string()))?;
+        (0..rows)
+            .map(|at| {
+                if null(at) {
+                    return Ok(Value::Null);
+                }
+                if let Some(word) = infinite.as_ref().and_then(|words| words[at]) {
+                    return Ok(Value::Text(word.into()));
+                }
+                let text = formatter
+                    .value(at)
+                    .try_to_string()
+                    .map_err(|error| Error::query(error.to_string()))?;
+                Ok(Value::Text(format!("{text}{suffix}").into()))
+            })
+            .collect()
+    };
+    let cast = |to: &DataType| {
+        duckdb::arrow::compute::cast(array, to).map_err(|error| Error::query(error.to_string()))
+    };
+    Ok(match array.data_type() {
+        DataType::Null => vec![Value::Null; rows],
+        DataType::Boolean => {
+            let values = array.as_boolean();
+            (0..rows)
+                .map(|at| match null(at) {
+                    true => Value::Null,
+                    false => Value::Bool(values.value(at)),
+                })
+                .collect()
+        }
+        DataType::Int8
+        | DataType::Int16
+        | DataType::Int32
+        | DataType::Int64
+        | DataType::UInt8
+        | DataType::UInt16
+        | DataType::UInt32 => {
+            let wide = cast(&DataType::Int64)?;
+            let values = wide.as_primitive::<Int64Type>();
+            (0..rows)
+                .map(|at| match null(at) {
+                    true => Value::Null,
+                    false => Value::Int(values.value(at)),
+                })
+                .collect()
+        }
+        // Past i64 it stays text, so nothing is rounded.
+        DataType::UInt64 => {
+            let values = array.as_primitive::<UInt64Type>();
+            (0..rows)
+                .map(|at| match null(at) {
+                    true => Value::Null,
+                    false => match i64::try_from(values.value(at)) {
+                        Ok(number) => Value::Int(number),
+                        Err(_) => Value::Text(values.value(at).to_string().into()),
+                    },
+                })
+                .collect()
+        }
+        DataType::Float16 | DataType::Float32 | DataType::Float64 => {
+            let wide = cast(&DataType::Float64)?;
+            let values = wide.as_primitive::<Float64Type>();
+            (0..rows)
+                .map(|at| match null(at) {
+                    true => Value::Null,
+                    false => Value::Float(values.value(at)),
+                })
+                .collect()
+        }
+        DataType::Binary | DataType::LargeBinary | DataType::BinaryView => {
+            let wide = cast(&DataType::LargeBinary)?;
+            let values = wide.as_binary::<i64>();
+            (0..rows)
+                .map(|at| match null(at) {
+                    true => Value::Null,
+                    false => Value::Bytes(values.value(at).into()),
+                })
+                .collect()
+        }
+        // A moment with an offset is kept in UTC, and DuckDB is built
+        // without the zones' names: written as PostgreSQL writes UTC.
+        DataType::Timestamp(unit, Some(_)) => {
+            text_of(cast(&DataType::Timestamp(*unit, None))?.as_ref(), "+00")?
+        }
+        // Moments inside a list, a struct or a map, without their zone.
+        other => match zoneless(other) {
+            Some(plain) => text_of(cast(&plain)?.as_ref(), "")?,
+            None => text_of(array, "")?,
+        },
+    })
+}
+
+/// The rows of `batch`, each a row of cells.
+fn batch_rows(batch: &RecordBatch) -> Result<Vec<Vec<Value>>> {
+    let columns: Vec<Vec<Value>> = batch
+        .columns()
+        .iter()
+        .map(|column| column_values(column.as_ref()))
+        .collect::<Result<_>>()?;
+    Ok((0..batch.num_rows())
+        .map(|row| columns.iter().map(|column| column[row].clone()).collect())
+        .collect())
+}
+
+/// Runs one statement and reads at most `limit` rows of it, and whether
+/// there were more. A streaming read: DuckDB makes no more rows than are
+/// read.
+fn rows_of(
+    connection: &duckdb::Connection,
+    text: &str,
+    params: Vec<duckdb::types::Value>,
+    limit: usize,
+) -> Result<(Vec<ColumnMeta>, Vec<Vec<Value>>, bool)> {
+    let mut statement = connection.prepare(text).map_err(map_error)?;
+    let stream = statement
+        .stream_arrow(duckdb::params_from_iter(params))
+        .map_err(map_error)?;
+    let columns: Vec<ColumnMeta> = stream
+        .get_schema()
+        .fields()
+        .iter()
+        .map(|field| column_meta(field))
+        .collect();
+    let mut rows = Vec::new();
+    for batch in stream {
+        let batch = batch.map_err(map_error)?;
+        rows.extend(batch_rows(&batch)?);
+        // Never read past one extra row, even if a raw WHERE comments out
+        // the LIMIT.
+        if rows.len() > limit {
+            break;
+        }
+    }
+    let more = rows.len() > limit;
+    rows.truncate(limit);
+    Ok((columns, rows, more))
+}
+
+/// Runs `texts` in one read-only transaction, each keeping at most `limit`
+/// rows; the first that fails ends the run. `params` are bound to the
+/// first statement (an agent's read is one statement). `watch`, which
+/// interrupts a statement on a stop or at the deadline, ends before the
+/// rollback.
+fn script(
+    connection: &duckdb::Connection,
+    texts: &[String],
+    limit: usize,
+    stop: &StopFlag,
+    params: &[Param],
+    watch: Watch,
+) -> Result<ScriptOutcome> {
+    let mut outcome = ScriptOutcome::default();
+    if stop.is_stopped() {
+        outcome.stopped = true;
+        return Ok(outcome);
+    }
+    connection
+        .execute_batch("BEGIN TRANSACTION READ ONLY")
+        .map_err(map_error)?;
+    for (index, text) in texts.iter().enumerate() {
+        if stop.is_stopped() {
+            outcome.stopped = true;
+            break;
+        }
+        let started = Instant::now();
+        let sent = match index {
+            0 => params.iter().map(bound).collect(),
+            _ => Vec::new(),
+        };
+        let result = rows_of(connection, text, sent, limit);
+        let elapsed = started.elapsed();
+        let (statement, failed) = match result {
+            Ok((columns, rows, truncated)) => (
+                StatementOutcome::Rows {
+                    columns,
+                    rows,
+                    truncated,
+                },
+                false,
+            ),
+            Err(Error::Cancelled) => (StatementOutcome::Cancelled, true),
+            Err(error) => (
+                StatementOutcome::Error {
+                    error,
+                    position: None,
+                },
+                true,
+            ),
+        };
+        outcome.results.push(StatementResult {
+            elapsed,
+            outcome: statement,
+        });
+        if failed {
+            break;
+        }
+    }
+    // The watch ends before the rollback, so no interrupt reaches it.
+    drop(watch);
+    stop.finish();
+    // A failed statement leaves DuckDB's transaction aborted; the rollback
+    // ends it either way.
+    connection
+        .execute_batch("ROLLBACK")
+        .map_err(|error| crate::script::cleanup_failed(ScriptMode::ReadOnly, &map_error(error)))?;
+    Ok(outcome)
+}
+
+/// A WHERE typed in the filter bar is one condition: a `;` or a NUL in
+/// it is refused before DuckDB reads it.
+fn check_raw_where(query: &RowQuery) -> Result<()> {
+    let Some(raw) = query.raw_where.as_deref() else {
+        return Ok(());
+    };
+    if raw.contains('\0') {
+        return Err(Error::query(
+            "The WHERE text holds a NUL character. Remove it.",
+        ));
+    }
+    if crate::sql::tokenize(Dialect::DuckDb, raw)
+        .iter()
+        .any(|token| token.kind == crate::sql::TokenKind::Semicolon)
+    {
+        return Err(Error::query(
+            "The WHERE text holds a `;`. A filter is one condition, not a statement.",
+        ));
+    }
+    Ok(())
+}
+
+/// The catalog and schema DuckDB keeps `object` in: a file's view in the
+/// in-memory database's `main`, a `.duckdb` file's table in its own.
+fn catalog_of(object: &ObjectRef) -> (&str, &str) {
+    match object.schema.as_str() {
+        MAIN => (MEMORY, MAIN),
+        attached => (attached, MAIN),
+    }
+}
+
+/// The columns of `object`, as DuckDB's catalog has them.
+fn columns(connection: &duckdb::Connection, object: &ObjectRef) -> Result<Vec<ColumnInfo>> {
+    let (catalog, schema) = catalog_of(object);
+    let mut statement = connection
+        .prepare(
+            "SELECT column_name, data_type, is_nullable, column_default \
+             FROM information_schema.columns \
+             WHERE table_catalog = ? AND table_schema = ? AND table_name = ? \
+             ORDER BY ordinal_position",
+        )
+        .map_err(map_error)?;
+    let rows = statement
+        .query_map([catalog, schema, object.name.as_str()], |row| {
+            Ok(ColumnInfo {
+                name: row.get(0)?,
+                type_name: row.get(1)?,
+                nullable: row.get::<_, String>(2)? != "NO",
+                default: row.get(3)?,
+                ..ColumnInfo::default()
+            })
+        })
+        .map_err(map_error)?;
+    rows.collect::<std::result::Result<Vec<_>, _>>()
+        .map_err(map_error)
+}
+
+/// The columns of `query`'s table that hold bytes.
+fn binary_columns(connection: &duckdb::Connection, object: &ObjectRef) -> Result<Vec<String>> {
+    Ok(binary_of(&columns(connection, object)?))
+}
+
+/// The names of `columns` that hold bytes.
+fn binary_of(columns: &[ColumnInfo]) -> Vec<String> {
+    columns
+        .iter()
+        .filter(|column| {
+            crate::column_class(Dialect::DuckDb, &column.type_name) == crate::ColumnClass::Binary
+        })
+        .map(|column| column.name.clone())
+        .collect()
+}
+
+impl Conn {
+    /// Opens `source`'s files, read-only: every one is read far enough to
+    /// know its columns, and an error names the first that cannot be.
+    pub(crate) async fn open(source: &Source) -> Result<Self> {
+        let source = source.clone();
+        let (connection, files, version) = tokio::task::spawn_blocking(move || {
+            let scan = crate::files::scan(&source)?;
+            if let Some(gone) = scan.missing.first() {
+                return Err(Error::Connect(format!(
+                    "Can't open {}: no such file",
+                    crate::files::shown(gone)
+                )));
+            }
+            opened(&scan.files)
+        })
+        .await
+        .map_err(|error| Error::Io(error.to_string()))??;
+        let interrupt = connection.interrupt_handle();
+        Ok(Self {
+            inner: Arc::new(Mutex::new(connection)),
+            interrupt,
+            running: Arc::new(std::sync::atomic::AtomicU64::new(0)),
+            calls: std::sync::atomic::AtomicU64::new(0),
+            files: Arc::new(files),
+            version,
+        })
+    }
+
+    /// Runs `work` on the blocking pool with the session.
+    async fn run<T: Send + 'static>(
+        &self,
+        work: impl FnOnce(&duckdb::Connection) -> Result<T> + Send + 'static,
+    ) -> Result<T> {
+        let inner = Arc::clone(&self.inner);
+        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
+        let given_up = Arc::new(AtomicBool::new(false));
+        let running = Arc::clone(&self.running);
+        let guard = InterruptOnDrop {
+            interrupt: Arc::clone(&self.interrupt),
+            running: Arc::clone(&self.running),
+            given_up: Arc::clone(&given_up),
+            call,
+            armed: true,
+        };
+        let result = tokio::task::spawn_blocking(move || {
+            let connection = inner
+                .lock()
+                .map_err(|_| Error::ConnectionLost("the DuckDB session was poisoned".into()))?;
+            // Said to be running first, then asked whether it was given up
+            // on, as the guard says it gives up first, then asks: one of
+            // the two always sees the other. Given up on: nothing runs.
+            running.store(call, Ordering::SeqCst);
+            if given_up.load(Ordering::SeqCst) {
+                running.store(0, Ordering::SeqCst);
+                return Err(Error::Cancelled);
+            }
+            let result = work(&connection);
+            running.store(0, Ordering::SeqCst);
+            result
+        })
+        .await;
+        guard.disarm();
+        result.map_err(|error| {
+            if error.is_panic() {
+                Error::ConnectionLost("the DuckDB worker panicked".into())
+            } else {
+                Error::Io(error.to_string())
+            }
+        })?
+    }
+
+    /// Reads the first rows of every file, as a Test does: never a whole
+    /// file.
+    pub(crate) async fn read_files(&self) -> Result<FilesRead> {
+        let files = Arc::clone(&self.files);
+        self.run(move |connection| {
+            read_only(connection, |connection| {
+                for file in files.iter().filter(|file| !file.found.format.is_database()) {
+                    let sql = format!(
+                        "SELECT * FROM {} LIMIT {SAMPLE_ROWS}",
+                        Dialect::DuckDb.quote_ident(&file.found.name)
+                    );
+                    rows_of(connection, &sql, Vec::new(), SAMPLE_ROWS as usize).map_err(
+                        |error| {
+                            Error::Connect(format!(
+                                "Can't read {}: {}",
+                                file_label(&file.found),
+                                plain(&error)
+                            ))
+                        },
+                    )?;
+                }
+                Ok(())
+            })?;
+            Ok(FilesRead {
+                files: files.len(),
+                bytes: files.iter().map(|file| file.found.size).sum(),
+            })
+        })
+        .await
+    }
+
+    /// The file `object` is read from, where it is a file's table.
+    fn file_of(&self, object: &ObjectRef) -> Option<&Opened> {
+        (object.schema == MAIN)
+            .then(|| {
+                self.files
+                    .iter()
+                    .find(|file| file.found.name == object.name)
+            })
+            .flatten()
+            .filter(|file| !file.found.format.is_database())
+    }
+}
+
+/// A file's name, for a message.
+fn file_label(file: &Found) -> String {
+    file.path
+        .file_name()
+        .map_or_else(|| file.name.clone(), |name| name.to_string_lossy().into())
+}
+
+/// An error's first line, without what `Error`'s words add: DuckDB follows
+/// a message with the statement it was in, which a line of the dialog has
+/// no room for.
+fn plain(error: &Error) -> String {
+    let words = match error {
+        Error::Query { message, .. } => message.clone(),
+        other => other.to_string(),
+    };
+    words
+        .lines()
+        .next()
+        .unwrap_or_default()
+        .trim_end()
+        .to_owned()
+}
+
+impl Adapter for Conn {
+    /// Never: the files are read here, not over a network.
+    fn is_encrypted(&self) -> bool {
+        false
+    }
+
+    fn cancel_handle(&self) -> CancelHandle {
+        CancelHandle(CancelInner::Files(Arc::clone(&self.interrupt)))
+    }
+
+    async fn server_version(&self) -> Result<String> {
+        Ok(self.version.clone())
+    }
+
+    /// DuckDB is built without its time zones: a moment with an offset is
+    /// read and shown in UTC.
+    async fn session_zone(&self) -> Result<Option<String>> {
+        Ok(Some("UTC".into()))
+    }
+
+    /// None: a connection's files are one database.
+    async fn list_databases(&self) -> Result<Vec<String>> {
+        Ok(Vec::new())
+    }
+
+    /// `main`, which holds the files' tables, then each `.duckdb` file.
+    async fn list_schemas(&self) -> Result<Vec<String>> {
+        Ok(std::iter::once(MAIN.to_owned())
+            .chain(
+                self.files
+                    .iter()
+                    .filter(|file| file.found.format.is_database())
+                    .map(|file| file.found.name.clone()),
+            )
+            .take(MAX_LISTED as usize)
+            .collect())
+    }
+
+    /// In `main`, a table for each file, in the files' order; in a
+    /// `.duckdb` file's schema, its tables and views by name.
+    async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
+        if schema == MAIN {
+            return Ok(self
+                .files
+                .iter()
+                .filter(|file| !file.found.format.is_database())
+                .map(|file| ObjectInfo {
+                    name: file.found.name.clone(),
+                    kind: ObjectKind::Table,
+                    estimated_rows: file.rows,
+                })
+                .take(MAX_LISTED as usize)
+                .collect());
+        }
+        let catalog = schema.to_owned();
+        self.run(move |connection| {
+            read_only(connection, |connection| {
+                let mut statement = connection
+                    .prepare(&format!(
+                        "SELECT table_name, table_type FROM information_schema.tables \
+                         WHERE table_catalog = ? AND table_schema = 'main' \
+                         ORDER BY table_name LIMIT {MAX_LISTED}"
+                    ))
+                    .map_err(map_error)?;
+                let objects = statement
+                    .query_map([catalog.as_str()], |row| {
+                        let kind: String = row.get(1)?;
+                        Ok(ObjectInfo {
+                            name: row.get(0)?,
+                            kind: match kind.as_str() {
+                                "VIEW" => ObjectKind::View,
+                                _ => ObjectKind::Table,
+                            },
+                            estimated_rows: None,
+                        })
+                    })
+                    .map_err(map_error)?
+                    .collect::<std::result::Result<Vec<_>, _>>()
+                    .map_err(map_error)?;
+                Ok(objects)
+            })
+        })
+        .await
+    }
+
+    async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
+        let file = self.file_of(object).map(|file| FileFacts {
+            path: file.found.path.clone(),
+            size: file.found.size,
+            format: file.found.format,
+            options: reader_options(file.found.format).to_owned(),
+        });
+        let object = object.clone();
+        self.run(move |connection| {
+            let columns = read_only(connection, |connection| columns(connection, &object))?;
+            if columns.is_empty() {
+                return Err(Error::query(format!(
+                    "no such table or view: {}",
+                    object.name
+                )));
+            }
+            Ok(Structure {
+                columns,
+                file,
+                ..Structure::default()
+            })
+        })
+        .await
+    }
+
+    /// The statement DuckDB keeps of the view a file is read through, or
+    /// of a `.duckdb` file's table or view.
+    async fn ddl(&self, object: &ObjectRef) -> Result<Ddl> {
+        use crate::ddl::{Part, PartKind};
+        let version = self.version.clone();
+        let ddl = move |statement: String| {
+            Ddl::new(
+                crate::Driver::Files,
+                version.clone(),
+                None,
+                vec![Part {
+                    kind: PartKind::Object,
+                    statements: vec![statement],
+                }],
+            )
+        };
+        if let Some(file) = self.file_of(object) {
+            return Ok(ddl(file.made.clone()));
+        }
+        let object = object.clone();
+        self.run(move |connection| {
+            let (catalog, schema) = catalog_of(&object);
+            let sql: Option<String> = read_only(connection, |connection| {
+                connection
+                    .query_row(
+                        "SELECT sql FROM duckdb_views() \
+                         WHERE database_name = ? AND schema_name = ? AND view_name = ? \
+                         UNION ALL \
+                         SELECT sql FROM duckdb_tables() \
+                         WHERE database_name = ? AND schema_name = ? AND table_name = ? \
+                         LIMIT 1",
+                        [catalog, schema, &object.name, catalog, schema, &object.name],
+                        |row| row.get(0),
+                    )
+                    .map(Some)
+                    .or_else(|error| match error {
+                        duckdb::Error::QueryReturnedNoRows => Ok(None),
+                        error => Err(map_error(error)),
+                    })
+            })?;
+            let Some(sql) = sql else {
+                return Err(Error::query(format!(
+                    "no such table or view: {}",
+                    object.name
+                )));
+            };
+            Ok(ddl(sql.trim_end().trim_end_matches(';').to_owned()))
+        })
+        .await
+    }
+
+    async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
+        check_raw_where(query)?;
+        let query = query.clone();
+        let limit = query.limit as usize;
+        self.run(move |connection| {
+            read_only(connection, |connection| {
+                let catalog = columns(connection, &query.object)?;
+                let binary = binary_of(&catalog);
+                // A file has no key: rows come in the file's order.
+                let sql = Dialect::DuckDb.select_rows(&query, &[], &binary);
+                let started = Instant::now();
+                let params = sql.params.iter().map(to_duckdb).collect();
+                let (mut columns, rows, has_more) = rows_of(connection, &sql.text, params, limit)?;
+                // The catalog's types, which DuckDB's Arrow export does not
+                // all keep: a JSON column is text to Arrow.
+                for meta in &mut columns {
+                    if let Some(info) = catalog.iter().find(|info| info.name == meta.name) {
+                        meta.type_name.clone_from(&info.type_name);
+                        if crate::column_class(Dialect::DuckDb, &info.type_name)
+                            == crate::ColumnClass::Json
+                        {
+                            meta.kind = ValueKind::Json;
+                        }
+                    }
+                }
+                Ok(RowPage {
+                    columns,
+                    rows,
+                    has_more,
+                    ordered_by_key: false,
+                    elapsed: started.elapsed(),
+                })
+            })
+        })
+        .await
+    }
+
+    async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
+        check_raw_where(query)?;
+        let query = query.clone();
+        self.run(move |connection| {
+            read_only(connection, |connection| {
+                let binary = binary_columns(connection, &query.object)?;
+                let sql = Dialect::DuckDb.count_rows(&query, &binary);
+                let count: i64 = connection
+                    .query_row(
+                        &sql.text,
+                        duckdb::params_from_iter(sql.params.iter().map(to_duckdb)),
+                        |row| row.get(0),
+                    )
+                    .map_err(map_error)?;
+                Ok(u64::try_from(count).unwrap_or(0))
+            })
+        })
+        .await
+    }
+
+    async fn lookup(&self, lookup: &Lookup) -> Result<LookupPage> {
+        let lookup = lookup.clone();
+        self.run(move |connection| {
+            read_only(connection, |connection| {
+                let sql = Dialect::DuckDb.lookup(&lookup);
+                let most = lookup.limit as usize + 1;
+                let params = sql.params.iter().map(to_duckdb).collect();
+                let (_, rows, _) = rows_of(connection, &sql.text, params, most)?;
+                crate::lookup::page(rows, &lookup)
+            })
+        })
+        .await
+    }
+
+    /// See [`crate::Connection::run_script`]. One blocking job; a thread
+    /// beside it interrupts the statement when `stop` is set.
+    async fn run_script(
+        &self,
+        texts: Vec<String>,
+        limit: u32,
+        mode: ScriptMode,
+        stop: &StopFlag,
+    ) -> Result<ScriptOutcome> {
+        match mode {
+            ScriptMode::ReadOnly => {}
+            // Editing files is not built: a Files session never writes.
+            ScriptMode::Write => return Err(Error::ReadOnly),
+        }
+        let stop = stop.clone();
+        let interrupt = Arc::clone(&self.interrupt);
+        self.run(move |connection| {
+            let watch = Watch::start(interrupt, stop.clone(), None);
+            script(connection, &texts, limit as usize, &stop, &[], watch)
+        })
+        .await
+    }
+
+    /// See [`crate::Connection::run_agent_read`]. A Files connection keeps
+    /// no hidden tables and no masked columns: where `sources` ask for
+    /// either, every read is refused.
+    async fn run_read(
+        &self,
+        read: &Read,
+        limits: Limits,
+        sources: &Sources,
+        stop: &StopFlag,
+    ) -> Result<(ScriptOutcome, Vec<bool>)> {
+        if sources.guards_anything() {
+            return Err(Error::Agent(Refusal::NotKeptOnFiles));
+        }
+        let stop = stop.clone();
+        let interrupt = Arc::clone(&self.interrupt);
+        let texts = vec![read.text().to_owned()];
+        let params = read.params().to_vec();
+        self.run(move |connection| {
+            let deadline = Instant::now().checked_add(limits.timeout);
+            let watch = Watch::start(interrupt, stop.clone(), deadline);
+            let outcome = script(
+                connection,
+                &texts,
+                limits.rows as usize,
+                &stop,
+                &params,
+                watch,
+            )?;
+            Ok((outcome, Vec::new()))
+        })
+        .await
+    }
+
+    /// Nothing to read: nothing is hidden or masked here (see
+    /// `run_read`).
+    async fn agents_catalog(&self, _masked: &[ColumnName]) -> Result<Catalog> {
+        Ok(Catalog::default())
+    }
+
+    /// Editing files is not built.
+    async fn write(&self, _changes: &ChangeSet, _stop: &StopFlag) -> Result<WriteOutcome> {
+        Err(Error::ReadOnly)
+    }
+}
```

```diff
diff --git a/crates/tabletist-db/src/files.rs b/crates/tabletist-db/src/files.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-db/src/files.rs
@@ -0,0 +1,589 @@
+//! What a Files connection opens: the files of a folder or of a list, the
+//! format of each, and the table (or, for a `.duckdb` file, the schema)
+//! each becomes. Reading the folder is the only disk work here; DuckDB
+//! reads the files themselves (`duck`).
+
+use std::collections::HashMap;
+use std::path::{Path, PathBuf};
+
+use serde::{Deserialize, Serialize};
+
+use crate::{Error, Result};
+
+/// Where a Files connection's files come from.
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "lowercase")]
+pub enum Source {
+    /// Every supported file directly in the folder, read again at each
+    /// open: one added later shows then.
+    Folder(PathBuf),
+    /// These files, in this order.
+    Files(Vec<PathBuf>),
+}
+
+impl Source {
+    /// The folder, or the files' names, as one line.
+    pub fn summary(&self) -> String {
+        match self {
+            Self::Folder(folder) => shown(folder),
+            Self::Files(files) => files
+                .iter()
+                .map(|file| file_name(file))
+                .collect::<Vec<_>>()
+                .join(", "),
+        }
+    }
+
+    /// The folder the files are in: the folder itself, or the first
+    /// file's.
+    pub fn folder(&self) -> Option<&Path> {
+        match self {
+            Self::Folder(folder) => Some(folder),
+            Self::Files(files) => files.first().and_then(|file| file.parent()),
+        }
+    }
+}
+
+/// How a file is read, from its extension.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
+pub enum Format {
+    Csv,
+    Tsv,
+    Parquet,
+    /// One JSON array of objects.
+    Json,
+    /// One JSON object per line (`.ndjson`, `.jsonl`).
+    Ndjson,
+    /// The first sheet of a workbook.
+    Excel,
+    /// A DuckDB database: a schema of its own tables.
+    DuckDb,
+}
+
+impl Format {
+    /// Every format, in the order the found line names them.
+    pub const ALL: [Self; 7] = [
+        Self::Csv,
+        Self::Tsv,
+        Self::Parquet,
+        Self::Json,
+        Self::Ndjson,
+        Self::Excel,
+        Self::DuckDb,
+    ];
+
+    /// The format a file of this name is read as, by its extension in any
+    /// case; `None` for a file the connection leaves out.
+    pub fn of(path: &Path) -> Option<Self> {
+        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
+        match extension.as_str() {
+            "csv" => Some(Self::Csv),
+            "tsv" => Some(Self::Tsv),
+            "parquet" => Some(Self::Parquet),
+            "json" => Some(Self::Json),
+            "ndjson" | "jsonl" => Some(Self::Ndjson),
+            "xlsx" => Some(Self::Excel),
+            "duckdb" => Some(Self::DuckDb),
+            _ => None,
+        }
+    }
+
+    /// As the dialog and the Structure view name it.
+    pub fn label(self) -> &'static str {
+        match self {
+            Self::Csv => "CSV",
+            Self::Tsv => "TSV",
+            Self::Parquet => "Parquet",
+            Self::Json => "JSON",
+            Self::Ndjson => "NDJSON",
+            Self::Excel => "Excel",
+            Self::DuckDb => "DuckDB",
+        }
+    }
+
+    /// Whether the file becomes a schema of tables rather than a table.
+    pub fn is_database(self) -> bool {
+        match self {
+            Self::DuckDb => true,
+            Self::Csv | Self::Tsv | Self::Parquet | Self::Json | Self::Ndjson | Self::Excel => {
+                false
+            }
+        }
+    }
+}
+
+/// One file of a connection.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Found {
+    pub path: PathBuf,
+    pub format: Format,
+    /// Its size in bytes, as the folder said it.
+    pub size: u64,
+    /// The table it becomes, or for a `.duckdb` file the schema.
+    pub name: String,
+}
+
+/// What a source holds as it is on disk now.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct Scan {
+    /// The files the connection opens, each with its name.
+    pub files: Vec<Found>,
+    /// Files of a list that are not there (or are no longer files).
+    pub missing: Vec<PathBuf>,
+}
+
+impl Scan {
+    pub fn total_size(&self) -> u64 {
+        self.files.iter().map(|file| file.size).sum()
+    }
+
+    /// The formats found, each once, in [`Format::ALL`]'s order.
+    pub fn formats(&self) -> Vec<Format> {
+        Format::ALL
+            .into_iter()
+            .filter(|format| self.files.iter().any(|file| file.format == *format))
+            .collect()
+    }
+
+    /// The names the files become, in the files' order.
+    pub fn names(&self) -> Vec<&str> {
+        self.files.iter().map(|file| file.name.as_str()).collect()
+    }
+}
+
+/// The files `source` holds: a folder's supported files directly in it,
+/// by name in any case (hidden files, subfolders and other types left out), or a
+/// list's in its order. A folder that cannot be read is an error naming
+/// it; a file of a list that is gone is [`Scan::missing`].
+pub fn scan(source: &Source) -> Result<Scan> {
+    let mut entries: Vec<(PathBuf, Format, u64)> = Vec::new();
+    let mut missing = Vec::new();
+    match source {
+        Source::Folder(folder) => {
+            let read = std::fs::read_dir(folder).map_err(|error| folder_error(folder, &error))?;
+            for entry in read {
+                let entry = entry.map_err(|error| folder_error(folder, &error))?;
+                let path = entry.path();
+                let hidden = entry
+                    .file_name()
+                    .to_str()
+                    .is_none_or(|n| n.starts_with('.'));
+                let Some(format) = Format::of(&path).filter(|_| !hidden) else {
+                    continue;
+                };
+                // A link to a file counts as the file; a folder never.
+                match std::fs::metadata(&path) {
+                    Ok(meta) if meta.is_file() => entries.push((path, format, meta.len())),
+                    Ok(_) | Err(_) => {}
+                }
+            }
+            // By name, in any case, as a person reads the folder.
+            entries.sort_by_cached_key(|(path, _, _)| {
+                let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase());
+                (name, path.clone())
+            });
+        }
+        Source::Files(files) => {
+            for path in files {
+                let Some(format) = Format::of(path) else {
+                    return Err(Error::InvalidSpec(format!(
+                        "{} is not a CSV, TSV, Parquet, JSON, NDJSON, Excel or .duckdb file",
+                        file_name(path)
+                    )));
+                };
+                match std::fs::metadata(path) {
+                    Ok(meta) if meta.is_file() => entries.push((path.clone(), format, meta.len())),
+                    Ok(_) | Err(_) => missing.push(path.clone()),
+                }
+            }
+        }
+    }
+    let names = names(
+        entries
+            .iter()
+            .map(|(path, format, _)| (path.as_path(), *format)),
+    );
+    let files = entries
+        .into_iter()
+        .zip(names)
+        .map(|((path, format, size), name)| Found {
+            path,
+            format,
+            size,
+            name,
+        })
+        .collect();
+    Ok(Scan { files, missing })
+}
+
+/// Why a folder cannot be read, in the dialog's words.
+fn folder_error(folder: &Path, error: &std::io::Error) -> Error {
+    let why = match error.kind() {
+        std::io::ErrorKind::NotFound => "no such folder".to_owned(),
+        std::io::ErrorKind::NotADirectory => "not a folder".to_owned(),
+        std::io::ErrorKind::PermissionDenied => "not allowed to read it".to_owned(),
+        _ => error.to_string(),
+    };
+    Error::Connect(format!("Can't open {}: {why}", shown(folder)))
+}
+
+/// The name a file's stem becomes: lowercased, with anything outside
+/// `[a-z0-9_]` turned into `_`, and `_` before a leading digit.
+pub fn table_name(stem: &str) -> String {
+    let mut name: String = stem
+        .chars()
+        .flat_map(char::to_lowercase)
+        .map(|c| match c {
+            'a'..='z' | '0'..='9' | '_' => c,
+            _ => '_',
+        })
+        .collect();
+    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
+        name.insert(0, '_');
+    }
+    name
+}
+
+/// The names DuckDB keeps for its own databases, which a `.duckdb` file
+/// cannot be attached under.
+const RESERVED: [&str; 4] = ["main", "memory", "system", "temp"];
+
+/// Each file's name, in order: its stem's [`table_name`], and where two
+/// files would share one, each of them that name with `_` and its
+/// extension (`orders_csv`, `orders_parquet`). A `.duckdb` file named as
+/// one of DuckDB's own databases takes `_duckdb`. Should a name still be
+/// taken (the same file name in two folders, or a file named as another's
+/// suffixed name), a number follows, until it is one no other file has.
+fn names<'a>(files: impl Iterator<Item = (&'a Path, Format)>) -> Vec<String> {
+    let files: Vec<(String, String, Format)> = files
+        .map(|(path, format)| {
+            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
+            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
+            (table_name(stem), extension.to_ascii_lowercase(), format)
+        })
+        .collect();
+    let mut counts: HashMap<&str, usize> = HashMap::new();
+    for (name, _, _) in &files {
+        *counts.entry(name).or_default() += 1;
+    }
+    // The names given so far.
+    let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();
+    let wanted: Vec<String> = files
+        .iter()
+        .map(|(name, extension, format)| {
+            let name = if counts[name.as_str()] > 1 {
+                format!("{name}_{}", table_name(extension).trim_start_matches('_'))
+            } else {
+                name.clone()
+            };
+            match format.is_database() && RESERVED.contains(&name.as_str()) {
+                true => format!("{name}_duckdb"),
+                false => name,
+            }
+        })
+        .collect();
+    let mut wanted_count: HashMap<&str, usize> = HashMap::new();
+    for name in &wanted {
+        *wanted_count.entry(name).or_default() += 1;
+    }
+    let mut given: HashMap<&str, usize> = HashMap::new();
+    wanted
+        .iter()
+        .map(|name| {
+            let seen = given.entry(name).or_default();
+            *seen += 1;
+            let mut candidate = match *seen {
+                1 => name.clone(),
+                n => format!("{name}_{n}"),
+            };
+            let mut n = (*seen).max(1);
+            // Neither one given already nor one another file wants as it is.
+            while taken.contains(&candidate)
+                || (candidate != *name && wanted_count.contains_key(candidate.as_str()))
+            {
+                n += 1;
+                candidate = format!("{name}_{n}");
+            }
+            taken.insert(candidate.clone());
+            candidate
+        })
+        .collect()
+}
+
+/// A file's name, for a list of them.
+fn file_name(path: &Path) -> String {
+    path.file_name()
+        .map(|name| name.to_string_lossy().into_owned())
+        .unwrap_or_else(|| path.display().to_string())
+}
+
+/// The home folder a path under it is shown from with `~`, as the app
+/// found it at its start ([`set_home`]); asked of the system once where
+/// it was not.
+static HOME: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
+
+/// Sets the home folder `~` stands for. Only the first call counts.
+pub fn set_home(home: Option<PathBuf>) {
+    let _ = HOME.set(home);
+}
+
+fn home() -> Option<&'static Path> {
+    HOME.get_or_init(std::env::home_dir).as_deref()
+}
+
+/// A path as the app shows it: `~` for the home folder.
+pub fn shown(path: &Path) -> String {
+    if let Some(home) = home()
+        && let Ok(rest) = path.strip_prefix(home)
+    {
+        if rest.as_os_str().is_empty() {
+            return "~".to_owned();
+        }
+        return format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display());
+    }
+    path.display().to_string()
+}
+
+/// A path as it was written: one that starts with `~` is under the home
+/// folder.
+pub fn expanded(path: &Path) -> PathBuf {
+    let Some(text) = path.to_str() else {
+        return path.to_owned();
+    };
+    let rest = match text.strip_prefix('~') {
+        Some("") => "",
+        Some(rest) => match rest.strip_prefix(['/', std::path::MAIN_SEPARATOR]) {
+            Some(rest) => rest,
+            None => return path.to_owned(),
+        },
+        None => return path.to_owned(),
+    };
+    match home() {
+        Some(home) if rest.is_empty() => home.to_owned(),
+        Some(home) => home.join(rest),
+        None => path.to_owned(),
+    }
+}
+
+/// `bytes` as the found line says them: `512 B`, `12.4 MB`, `2.5 GB`.
+pub fn size_text(bytes: u64) -> String {
+    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
+    let mut value = bytes as f64;
+    let mut unit = 0;
+    while value >= 1000.0 && unit + 1 < UNITS.len() {
+        value /= 1000.0;
+        unit += 1;
+    }
+    // What would be written as 1000 of a unit is 1.0 of the next.
+    if unit > 0 && value >= 999.5 && unit + 1 < UNITS.len() {
+        value /= 1000.0;
+        unit += 1;
+    }
+    match unit {
+        0 => format!("{bytes} B"),
+        _ if value >= 100.0 => format!("{value:.0} {}", UNITS[unit]),
+        _ => format!("{value:.1} {}", UNITS[unit]),
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn touch(dir: &Path, name: &str, bytes: usize) {
+        std::fs::write(dir.join(name), vec![b'x'; bytes]).unwrap();
+    }
+
+    #[test]
+    fn a_folder_holds_its_supported_files_by_name() {
+        let dir = tempfile::tempdir().unwrap();
+        for (name, bytes) in [
+            ("orders.csv", 10),
+            ("Customers.PARQUET", 20),
+            ("events.ndjson", 5),
+            ("log.jsonl", 1),
+            ("authors.tsv", 2),
+            ("books.json", 3),
+            ("stock.xlsx", 4),
+            ("bookshop.duckdb", 6),
+            (".hidden.csv", 1),
+            ("notes.txt", 1),
+            ("README", 1),
+        ] {
+            touch(dir.path(), name, bytes);
+        }
+        std::fs::create_dir(dir.path().join("archive.csv")).unwrap();
+        std::fs::create_dir(dir.path().join("older")).unwrap();
+        touch(&dir.path().join("older"), "orders.csv", 1);
+        let scan = scan(&Source::Folder(dir.path().to_owned())).unwrap();
+        assert_eq!(
+            scan.names(),
+            [
+                "authors",
+                "books",
+                "bookshop",
+                "customers",
+                "events",
+                "log",
+                "orders",
+                "stock"
+            ]
+        );
+        assert_eq!(scan.total_size(), 51);
+        assert_eq!(
+            scan.formats(),
+            [
+                Format::Csv,
+                Format::Tsv,
+                Format::Parquet,
+                Format::Json,
+                Format::Ndjson,
+                Format::Excel,
+                Format::DuckDb
+            ]
+        );
+        assert!(scan.missing.is_empty());
+    }
+
+    #[test]
+    fn names_that_clash_take_their_extension() {
+        let dir = tempfile::tempdir().unwrap();
+        touch(dir.path(), "orders.csv", 1);
+        touch(dir.path(), "orders.parquet", 1);
+        touch(dir.path(), "Order Lines.csv", 1);
+        touch(dir.path(), "2024-returns.csv", 1);
+        let scan = scan(&Source::Folder(dir.path().to_owned())).unwrap();
+        assert_eq!(
+            scan.names(),
+            [
+                "_2024_returns",
+                "order_lines",
+                "orders_csv",
+                "orders_parquet"
+            ]
+        );
+    }
+
+    #[test]
+    fn a_number_never_gives_a_name_another_file_has() {
+        let dir = tempfile::tempdir().unwrap();
+        touch(dir.path(), "orders.csv", 1);
+        touch(dir.path(), "orders.CSV", 1);
+        touch(dir.path(), "orders_csv_2.csv", 1);
+        let scan = scan(&Source::Folder(dir.path().to_owned())).unwrap();
+        let mut names = scan.names();
+        names.sort_unstable();
+        names.dedup();
+        assert_eq!(names.len(), 3, "{:?}", scan.names());
+        assert!(scan.names().contains(&"orders_csv_2"));
+    }
+
+    #[test]
+    fn a_duckdb_file_named_as_duckdbs_own_database_takes_a_suffix() {
+        let dir = tempfile::tempdir().unwrap();
+        for name in [
+            "main.duckdb",
+            "memory.duckdb",
+            "system.duckdb",
+            "temp.duckdb",
+            "main.csv",
+        ] {
+            touch(dir.path(), name, 1);
+        }
+        let scan = scan(&Source::Folder(dir.path().to_owned())).unwrap();
+        assert_eq!(
+            scan.names(),
+            [
+                "main_csv",
+                "main_duckdb",
+                "memory_duckdb",
+                "system_duckdb",
+                "temp_duckdb"
+            ]
+        );
+    }
+
+    #[test]
+    fn a_list_keeps_its_order_and_says_what_is_gone() {
+        let dir = tempfile::tempdir().unwrap();
+        let other = tempfile::tempdir().unwrap();
+        touch(dir.path(), "orders.csv", 3);
+        touch(other.path(), "orders.csv", 4);
+        touch(dir.path(), "events.ndjson", 2);
+        let list = vec![
+            dir.path().join("events.ndjson"),
+            dir.path().join("orders.csv"),
+            other.path().join("orders.csv"),
+            dir.path().join("returns.csv"),
+        ];
+        let scan = scan(&Source::Files(list)).unwrap();
+        assert_eq!(scan.names(), ["events", "orders_csv", "orders_csv_2"]);
+        assert_eq!(scan.missing, [dir.path().join("returns.csv")]);
+        assert_eq!(scan.total_size(), 9);
+    }
+
+    #[test]
+    fn a_list_refuses_a_file_it_cannot_read() {
+        let error = scan(&Source::Files(vec!["/data/notes.txt".into()])).unwrap_err();
+        assert!(
+            error.to_string().contains("notes.txt is not a CSV"),
+            "{error}"
+        );
+    }
+
+    #[test]
+    fn a_folder_that_is_gone_says_so() {
+        let dir = tempfile::tempdir().unwrap();
+        let gone = dir.path().join("bookshop-exports");
+        let error = scan(&Source::Folder(gone)).unwrap_err();
+        let Error::Connect(message) = error else {
+            panic!("{error:?}");
+        };
+        assert!(message.starts_with("Can't open "), "{message}");
+        assert!(
+            message.ends_with("bookshop-exports: no such folder"),
+            "{message}"
+        );
+    }
+
+    #[test]
+    fn table_names_are_plain_lowercase_words() {
+        assert_eq!(table_name("Orders"), "orders");
+        assert_eq!(table_name("order-lines 2024"), "order_lines_2024");
+        assert_eq!(table_name("café"), "caf_");
+        assert_eq!(table_name("2024"), "_2024");
+        assert_eq!(table_name(""), "_");
+    }
+
+    #[test]
+    fn sizes_read_as_the_found_line_says_them() {
+        assert_eq!(size_text(512), "512 B");
+        assert_eq!(size_text(12_400_000), "12.4 MB");
+        assert_eq!(size_text(2_480_000_000), "2.5 GB");
+        assert_eq!(size_text(1_600_000_000), "1.6 GB");
+        assert_eq!(size_text(250_000), "250 KB");
+        assert_eq!(size_text(999_950), "1.0 MB");
+        assert_eq!(size_text(999), "999 B");
+    }
+
+    #[test]
+    fn home_paths_are_shown_and_read_with_a_tilde() {
+        let Some(home) = std::env::home_dir() else {
+            return;
+        };
+        let path = home.join("Data").join("bookshop-exports");
+        let shown = shown(&path);
+        assert!(shown.starts_with('~'), "{shown}");
+        assert_eq!(expanded(Path::new(&shown)), path);
+        assert_eq!(expanded(Path::new("~")), home);
+        assert_eq!(expanded(Path::new("/data/x")), Path::new("/data/x"));
+        assert_eq!(expanded(Path::new("~bob/x")), Path::new("~bob/x"));
+    }
+
+    #[test]
+    fn a_source_says_itself_in_a_line() {
+        let list = Source::Files(vec!["/d/orders.csv".into(), "/d/customers.parquet".into()]);
+        assert_eq!(list.summary(), "orders.csv, customers.parquet");
+        assert_eq!(list.folder(), Some(Path::new("/d")));
+    }
+}
```

```diff
diff --git a/crates/tabletist-db/src/fixtures.rs b/crates/tabletist-db/src/fixtures.rs
--- a/crates/tabletist-db/src/fixtures.rs
+++ b/crates/tabletist-db/src/fixtures.rs
@@ -1,5 +1,6 @@
 //! The shared fixtures: the one the driver tests and the app's demo mode
-//! use, and the Bookshop, which what an agent may read is tested on.
+//! use, the Bookshop, which what an agent may read is tested on, and the
+//! Bookshop's exports, which Files connections are tested on.
 
 use std::path::Path;
 
@@ -38,6 +39,105 @@ fn write_sqlite(path: &Path, sql: &str) -> Result<()> {
         .map_err(|error| Error::Io(error.to_string()))
 }
 
+/// The files [`write_bookshop_files`] writes, by name.
+pub const BOOKSHOP_FILES: [&str; 7] = [
+    "authors.tsv",
+    "books.json",
+    "bookshop.duckdb",
+    "customers.parquet",
+    "events.ndjson",
+    "orders.csv",
+    "returns.csv",
+];
+
+/// Writes the Bookshop's exports into the folder `dir`, as a shop would
+/// export them: `orders.csv`, `returns.csv`, `authors.tsv` (tab
+/// separated), `customers.parquet`, `books.json` (an array),
+/// `events.ndjson` (an order placed, a line each), and `bookshop.duckdb`
+/// holding `book_covers` and `order_items`. Refuses a folder that holds
+/// any of them already.
+pub fn write_bookshop_files(dir: &Path) -> Result<()> {
+    let io = |error: &dyn std::fmt::Display| Error::Io(error.to_string());
+    if let Some(there) = BOOKSHOP_FILES.iter().find(|name| dir.join(name).exists()) {
+        return Err(Error::Io(format!("{there} already exists")));
+    }
+    let shop = rusqlite::Connection::open_in_memory().map_err(|e| io(&e))?;
+    shop.execute_batch(BOOKSHOP_SQLITE_SQL)
+        .map_err(|e| io(&e))?;
+    let duck = duckdb::Connection::open_in_memory().map_err(|e| io(&e))?;
+    // Each table as DuckDB's, from its rows as literals.
+    for table in [
+        "authors",
+        "books",
+        "customers",
+        "orders",
+        "order_items",
+        "book_covers",
+    ] {
+        let mut statement = shop
+            .prepare(&format!("SELECT * FROM {table} ORDER BY id"))
+            .map_err(|e| io(&e))?;
+        let names: Vec<String> = statement
+            .column_names()
+            .into_iter()
+            .map(|name| crate::Dialect::DuckDb.quote_ident(name))
+            .collect();
+        let width = names.len();
+        let rows: Vec<String> = statement
+            .query_map([], |row| {
+                let cells: Vec<String> = (0..width)
+                    .map(|at| {
+                        use rusqlite::types::ValueRef;
+                        Ok(match row.get_ref(at)? {
+                            ValueRef::Null => "NULL".to_owned(),
+                            ValueRef::Integer(number) => number.to_string(),
+                            ValueRef::Real(number) => format!("{number:?}"),
+                            ValueRef::Text(text) => {
+                                crate::Dialect::DuckDb.literal(&String::from_utf8_lossy(text))
+                            }
+                            ValueRef::Blob(bytes) => crate::Dialect::DuckDb.bytes_literal(bytes),
+                        })
+                    })
+                    .collect::<rusqlite::Result<_>>()?;
+                Ok(format!("({})", cells.join(", ")))
+            })
+            .map_err(|e| io(&e))?
+            .collect::<rusqlite::Result<_>>()
+            .map_err(|e| io(&e))?;
+        duck.execute_batch(&format!(
+            "CREATE TABLE {table} AS SELECT * FROM (VALUES {}) AS t({})",
+            rows.join(", "),
+            names.join(", ")
+        ))
+        .map_err(|e| io(&e))?;
+    }
+    let path = |name: &str| crate::Dialect::DuckDb.literal(&dir.join(name).to_string_lossy());
+    duck.execute_batch(&format!(
+        "COPY orders TO {} (HEADER, DELIMITER ',');
+         COPY (SELECT id AS order_id, placed_at::DATE + 9 AS returned_at,
+                      'damaged' AS reason
+               FROM orders WHERE id % 3 = 0)
+           TO {} (HEADER, DELIMITER ',');
+         COPY authors TO {} (HEADER, DELIMITER '\t');
+         COPY customers TO {} (FORMAT parquet, COMPRESSION zstd);
+         COPY books TO {} (FORMAT json, ARRAY true);
+         COPY (SELECT id AS order_id, 'placed' AS kind, placed_at AS at FROM orders)
+           TO {} (FORMAT json);
+         ATTACH {} AS shop;
+         CREATE TABLE shop.book_covers AS SELECT * FROM book_covers;
+         CREATE TABLE shop.order_items AS SELECT * FROM order_items;
+         DETACH shop;",
+        path("orders.csv"),
+        path("returns.csv"),
+        path("authors.tsv"),
+        path("customers.parquet"),
+        path("books.json"),
+        path("events.ndjson"),
+        path("bookshop.duckdb"),
+    ))
+    .map_err(|e| io(&e))
+}
+
 /// The PostgreSQL fixture script, loaded by the integration tests through
 /// an admin connection.
 pub const POSTGRES_SQL: &str = include_str!("../fixtures/postgres.sql");
```

```diff
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -1,4 +1,5 @@
-//! Access to PostgreSQL, MySQL and SQLite for Tabletist.
+//! Access to PostgreSQL, MySQL, SQLite and data files (through DuckDB)
+//! for Tabletist.
 //!
 //! The crate writes to a connected database in two places:
 //! [`Connection::write`], which saves edited rows, and
@@ -17,7 +18,9 @@ mod class;
 pub mod complete;
 pub mod ddl;
 mod dialect;
+mod duck;
 mod error;
+pub mod files;
 pub mod fixtures;
 mod lookup;
 mod mysql;
@@ -48,6 +51,7 @@ pub use dialect::{
     DeleteParts, Dialect, InsertParts, InsertStatement, RowDeletion, RowUpdate, Sql, UpdateParts,
     escape_like, quote_literal,
 };
+pub use duck::{FileFacts, FilesRead, set_extension_directory};
 pub use error::{Error, Named, Result, SshStage};
 pub use lookup::{Lookup, LookupPage, LookupPart, LookupRow, Matched};
 pub use query::{Filter, FilterOp, RowPage, RowQuery, Sort, SortDir};
@@ -88,6 +92,7 @@ enum Inner {
     // Boxed: a PostgreSQL client is much larger than a SQLite handle.
     Postgres(Box<pg::Conn>),
     MySql(Box<mysql::Conn>),
+    Files(Box<duck::Conn>),
 }
 
 impl Inner {
@@ -111,6 +116,13 @@ impl Inner {
             Driver::MySql => Ok(Self::MySql(Box::new(
                 mysql::Conn::connect(spec, secrets, via, access).await?,
             ))),
+            Driver::Files => {
+                let source = spec
+                    .files
+                    .as_ref()
+                    .ok_or_else(|| Error::InvalidSpec("choose a folder or files".into()))?;
+                Ok(Self::Files(Box::new(duck::Conn::open(source).await?)))
+            }
         }
     }
 }
@@ -130,9 +142,15 @@ impl Connection {
         host_keys: &HostKeys,
         access: Access,
     ) -> Result<Self> {
+        // Editing files is not built: a Files session reads, whatever
+        // access it is asked for, and says so.
+        let access = match spec.driver {
+            Driver::Files => Access::ReadOnly,
+            Driver::Postgres | Driver::MySql | Driver::Sqlite => access,
+        };
         // A file is opened here, whatever tunnel its spec still names.
         let ssh = match spec.driver {
-            Driver::Sqlite => None,
+            Driver::Sqlite | Driver::Files => None,
             Driver::Postgres | Driver::MySql => spec.ssh.as_ref(),
         };
         let Some(ssh) = ssh else {
@@ -168,6 +186,7 @@ impl Connection {
             Inner::Sqlite(_) => Driver::Sqlite,
             Inner::Postgres(_) => Driver::Postgres,
             Inner::MySql(_) => Driver::MySql,
+            Inner::Files(_) => Driver::Files,
         }
     }
 
@@ -179,6 +198,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.is_encrypted(),
             Inner::Postgres(conn) => conn.is_encrypted(),
             Inner::MySql(conn) => conn.is_encrypted(),
+            Inner::Files(conn) => conn.is_encrypted(),
         }
     }
 
@@ -226,6 +246,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.run_script(texts, limit, mode, stop).await,
             Inner::Postgres(conn) => conn.run_script(texts, limit, mode, stop).await,
             Inner::MySql(conn) => conn.run_script(texts, limit, mode, stop).await,
+            Inner::Files(conn) => conn.run_script(texts, limit, mode, stop).await,
         }
     }
 
@@ -281,6 +302,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.run_read(read, limits, sources, stop).await,
             Inner::Postgres(conn) => conn.run_read(read, limits, sources, stop).await,
             Inner::MySql(conn) => conn.run_read(read, limits, sources, stop).await,
+            Inner::Files(conn) => conn.run_read(read, limits, sources, stop).await,
         }?;
         agent::answer(outcome, stop).map(|answer| agent::masked(answer, masked))
     }
@@ -297,6 +319,7 @@ impl Connection {
                     Inner::Sqlite(conn) => conn.agents_catalog(&masked).await?,
                     Inner::Postgres(conn) => conn.agents_catalog(&masked).await?,
                     Inner::MySql(conn) => conn.agents_catalog(&masked).await?,
+                    Inner::Files(conn) => conn.agents_catalog(&masked).await?,
                 }
             }
             false => agent::Catalog::default(),
@@ -311,6 +334,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.server_version().await,
             Inner::Postgres(conn) => conn.server_version().await,
             Inner::MySql(conn) => conn.server_version().await,
+            Inner::Files(conn) => conn.server_version().await,
         }
     }
 
@@ -323,6 +347,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.session_zone().await,
             Inner::Postgres(conn) => conn.session_zone().await,
             Inner::MySql(conn) => conn.session_zone().await,
+            Inner::Files(conn) => conn.session_zone().await,
         }
     }
 
@@ -333,6 +358,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.list_databases().await,
             Inner::Postgres(conn) => conn.list_databases().await,
             Inner::MySql(conn) => conn.list_databases().await,
+            Inner::Files(conn) => conn.list_databases().await,
         }
     }
 
@@ -342,6 +368,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.list_schemas().await,
             Inner::Postgres(conn) => conn.list_schemas().await,
             Inner::MySql(conn) => conn.list_schemas().await,
+            Inner::Files(conn) => conn.list_schemas().await,
         }
     }
 
@@ -351,6 +378,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.list_objects(schema).await,
             Inner::Postgres(conn) => conn.list_objects(schema).await,
             Inner::MySql(conn) => conn.list_objects(schema).await,
+            Inner::Files(conn) => conn.list_objects(schema).await,
         }
     }
 
@@ -359,6 +387,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.describe(object).await,
             Inner::Postgres(conn) => conn.describe(object).await,
             Inner::MySql(conn) => conn.describe(object).await,
+            Inner::Files(conn) => conn.describe(object).await,
         }
     }
 
@@ -373,6 +402,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.ddl(object).await,
             Inner::Postgres(conn) => conn.ddl(object).await,
             Inner::MySql(conn) => conn.ddl(object).await,
+            Inner::Files(conn) => conn.ddl(object).await,
         }
     }
 
@@ -385,6 +415,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.fetch_rows(query).await,
             Inner::Postgres(conn) => conn.fetch_rows(query).await,
             Inner::MySql(conn) => conn.fetch_rows(query).await,
+            Inner::Files(conn) => conn.fetch_rows(query).await,
         }
     }
 
@@ -393,6 +424,17 @@ impl Connection {
             Inner::Sqlite(conn) => conn.count_rows(query).await,
             Inner::Postgres(conn) => conn.count_rows(query).await,
             Inner::MySql(conn) => conn.count_rows(query).await,
+            Inner::Files(conn) => conn.count_rows(query).await,
+        }
+    }
+
+    /// A Files connection's Test: the first rows of every file, with the
+    /// readers browsing uses, never a whole file. `None` for any other
+    /// driver, whose Test is the connection itself.
+    pub async fn read_files(&self) -> Option<Result<FilesRead>> {
+        match &self.inner {
+            Inner::Files(conn) => Some(conn.read_files().await),
+            Inner::Sqlite(_) | Inner::Postgres(_) | Inner::MySql(_) => None,
         }
     }
 
@@ -406,6 +448,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.lookup(lookup).await,
             Inner::Postgres(conn) => conn.lookup(lookup).await,
             Inner::MySql(conn) => conn.lookup(lookup).await,
+            Inner::Files(conn) => conn.lookup(lookup).await,
         }
     }
 
@@ -452,6 +495,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.write(changes, stop).await,
             Inner::Postgres(conn) => conn.write(changes, stop).await,
             Inner::MySql(conn) => conn.write(changes, stop).await,
+            Inner::Files(conn) => conn.write(changes, stop).await,
         }
     }
 
@@ -461,6 +505,7 @@ impl Connection {
             Inner::Sqlite(conn) => conn.cancel_handle(),
             Inner::Postgres(conn) => conn.cancel_handle(),
             Inner::MySql(conn) => conn.cancel_handle(),
+            Inner::Files(conn) => conn.cancel_handle(),
         }
     }
 
@@ -479,6 +524,10 @@ impl Connection {
                 drop(conn);
                 Ok(())
             }
+            Inner::Files(conn) => {
+                drop(conn);
+                Ok(())
+            }
         };
         // The driver has closed; now its tunnel.
         drop(tunnel);
@@ -494,6 +543,7 @@ pub struct CancelHandle(CancelInner);
 #[derive(Clone)]
 enum CancelInner {
     Sqlite(Arc<rusqlite::InterruptHandle>),
+    Files(Arc<duckdb::InterruptHandle>),
     Postgres {
         token: tokio_postgres::CancelToken,
         tls: tokio_postgres_rustls::MakeRustlsConnect,
@@ -513,6 +563,10 @@ impl CancelHandle {
                 handle.interrupt();
                 Ok(())
             }
+            CancelInner::Files(handle) => {
+                handle.interrupt();
+                Ok(())
+            }
             CancelInner::MySql { opts, id, server } => mysql::cancel(opts, *id, server).await,
             CancelInner::Postgres { token, tls } => token
                 .cancel_query(tls.clone())
```

```diff
diff --git a/crates/tabletist-db/src/mysql.rs b/crates/tabletist-db/src/mysql.rs
--- a/crates/tabletist-db/src/mysql.rs
+++ b/crates/tabletist-db/src/mysql.rs
@@ -477,6 +477,7 @@ impl Adapter for Conn {
             foreign_keys,
             referenced_by,
             comment,
+            file: None,
         })
     }
 
```

```diff
diff --git a/crates/tabletist-db/src/pg.rs b/crates/tabletist-db/src/pg.rs
--- a/crates/tabletist-db/src/pg.rs
+++ b/crates/tabletist-db/src/pg.rs
@@ -783,6 +783,7 @@ impl Adapter for Conn {
             foreign_keys,
             referenced_by,
             comment,
+            file: None,
         })
     }
 
```

```diff
diff --git a/crates/tabletist-db/src/reserved.rs b/crates/tabletist-db/src/reserved.rs
--- a/crates/tabletist-db/src/reserved.rs
+++ b/crates/tabletist-db/src/reserved.rs
@@ -546,7 +546,8 @@ const SQLITE: &[&str] = &[
 /// editor highlights as a keyword there.
 pub fn is_reserved(dialect: Dialect, name: &str) -> bool {
     let words = match dialect {
-        Dialect::Postgres => POSTGRES,
+        // DuckDB's parser is PostgreSQL's, and reserves its words.
+        Dialect::Postgres | Dialect::DuckDb => POSTGRES,
         Dialect::MySql => MYSQL,
         Dialect::Sqlite => SQLITE,
     };
```

```diff
diff --git a/crates/tabletist-db/src/spec.rs b/crates/tabletist-db/src/spec.rs
--- a/crates/tabletist-db/src/spec.rs
+++ b/crates/tabletist-db/src/spec.rs
@@ -16,6 +16,8 @@ pub enum Driver {
     #[serde(rename = "mysql")]
     MySql,
     Sqlite,
+    /// Data files read as tables, through DuckDB in the app's process.
+    Files,
 }
 
 impl Driver {
@@ -23,7 +25,7 @@ impl Driver {
         match self {
             Self::Postgres => 5432,
             Self::MySql => 3306,
-            Self::Sqlite => 0,
+            Self::Sqlite | Self::Files => 0,
         }
     }
 
@@ -32,6 +34,7 @@ impl Driver {
             Self::Postgres => "PostgreSQL",
             Self::MySql => "MySQL",
             Self::Sqlite => "SQLite",
+            Self::Files => "Files",
         }
     }
 
@@ -41,6 +44,7 @@ impl Driver {
             Self::Postgres => crate::Dialect::Postgres,
             Self::MySql => crate::Dialect::MySql,
             Self::Sqlite => crate::Dialect::Sqlite,
+            Self::Files => crate::Dialect::DuckDb,
         }
     }
 }
@@ -108,6 +112,8 @@ pub struct ConnectSpec {
     pub tls: TlsMode,
     pub ca_file: Option<PathBuf>,
     pub ssh: Option<SshSpec>,
+    /// The files a Files connection opens; `None` for every other driver.
+    pub files: Option<crate::files::Source>,
 }
 
 impl Default for ConnectSpec {
@@ -122,6 +128,7 @@ impl Default for ConnectSpec {
             tls: TlsMode::Prefer,
             ca_file: None,
             ssh: None,
+            files: None,
         }
     }
 }
@@ -247,6 +254,17 @@ impl ConnectSpec {
         }
     }
 
+    /// A Files connection that opens `source`.
+    pub fn files(source: crate::files::Source) -> Self {
+        Self {
+            driver: Driver::Files,
+            host: String::new(),
+            port: 0,
+            files: Some(source),
+            ..Self::default()
+        }
+    }
+
     /// Parses `postgres://`, `postgresql://`, `mysql://`, `mariadb://` and
     /// `sqlite:` URLs. The password, if any, comes back in `Secrets`.
     pub fn from_url(url: &str) -> Result<(Self, Secrets)> {
@@ -259,12 +277,12 @@ impl ConnectSpec {
     pub fn effective_tls(&self) -> TlsMode {
         match (self.driver, self.tls, &self.ca_file) {
             (Driver::Postgres, TlsMode::Require, Some(_)) => TlsMode::VerifyCa,
-            (Driver::Postgres | Driver::MySql | Driver::Sqlite, tls, _) => tls,
+            (Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::Files, tls, _) => tls,
         }
     }
 
-    /// A one-line description without secrets: `user@host:port/db`, or the
-    /// file name for SQLite.
+    /// A one-line description without secrets: `user@host:port/db`, the
+    /// file name for SQLite, or a Files connection's folder or files.
     pub fn summary(&self) -> String {
         match self.driver {
             Driver::Sqlite => {
@@ -275,6 +293,13 @@ impl ConnectSpec {
                     .map(|name| name.to_string_lossy().into_owned())
                     .unwrap_or_default();
             }
+            Driver::Files => {
+                return self
+                    .files
+                    .as_ref()
+                    .map(crate::files::Source::summary)
+                    .unwrap_or_default();
+            }
             Driver::Postgres | Driver::MySql => {}
         }
         let mut text = String::new();
@@ -308,6 +333,16 @@ mod tests {
         assert_eq!(Driver::Postgres.dialect(), crate::Dialect::Postgres);
         assert_eq!(Driver::MySql.dialect(), crate::Dialect::MySql);
         assert_eq!(Driver::Sqlite.dialect(), crate::Dialect::Sqlite);
+        assert_eq!(Driver::Files.dialect(), crate::Dialect::DuckDb);
+    }
+
+    #[test]
+    fn a_files_spec_says_its_source() {
+        let spec = ConnectSpec::files(crate::files::Source::Folder("/data/bookshop".into()));
+        assert_eq!(spec.driver, Driver::Files);
+        assert_eq!(spec.summary(), "/data/bookshop");
+        assert_eq!(spec.driver.label(), "Files");
+        assert_eq!(spec.driver.default_port(), 0);
     }
 
     #[test]
```

```diff
diff --git a/crates/tabletist-db/src/sql.rs b/crates/tabletist-db/src/sql.rs
--- a/crates/tabletist-db/src/sql.rs
+++ b/crates/tabletist-db/src/sql.rs
@@ -129,6 +129,18 @@ const KEYWORDS: &[&str] = &[
 const POSTGRES_KEYWORDS: &[&str] = &["ILIKE", "LATERAL", "RETURNING", "SIMILAR"];
 const MYSQL_KEYWORDS: &[&str] = &["DESCRIBE", "REGEXP", "STRAIGHT_JOIN"];
 const SQLITE_KEYWORDS: &[&str] = &["GLOB", "PRAGMA", "RETURNING"];
+const DUCKDB_KEYWORDS: &[&str] = &[
+    "DESCRIBE",
+    "EXCLUDE",
+    "ILIKE",
+    "LATERAL",
+    "PIVOT",
+    "QUALIFY",
+    "RETURNING",
+    "SIMILAR",
+    "SUMMARIZE",
+    "UNPIVOT",
+];
 
 /// The keywords `dialect` highlights, in upper case: the words the editor
 /// completes too. A keyword worth completing is worth colouring, so there
@@ -138,6 +150,7 @@ pub fn keywords(dialect: Dialect) -> impl Iterator<Item = &'static str> {
         Dialect::Postgres => POSTGRES_KEYWORDS,
         Dialect::MySql => MYSQL_KEYWORDS,
         Dialect::Sqlite => SQLITE_KEYWORDS,
+        Dialect::DuckDb => DUCKDB_KEYWORDS,
     };
     KEYWORDS.iter().chain(own).copied()
 }
@@ -149,12 +162,13 @@ pub fn is_keyword(dialect: Dialect, word: &str) -> bool {
 
 /// How a dialect reads text. A guard in the tokenizer asks one of these
 /// rather than naming a dialect, so every rule answers for every dialect.
+/// DuckDB's scanner is PostgreSQL's, and answers as it does.
 impl Dialect {
     /// A byte-order mark where a token starts reads as a space.
     fn bom_is_space(self) -> bool {
         match self {
             Self::Sqlite => true,
-            Self::Postgres | Self::MySql => false,
+            Self::Postgres | Self::MySql | Self::DuckDb => false,
         }
     }
 
@@ -162,7 +176,7 @@ impl Dialect {
     fn hash_comments(self) -> bool {
         match self {
             Self::MySql => true,
-            Self::Postgres | Self::Sqlite => false,
+            Self::Postgres | Self::Sqlite | Self::DuckDb => false,
         }
     }
 
@@ -170,14 +184,14 @@ impl Dialect {
     fn runs_comments(self) -> bool {
         match self {
             Self::MySql => true,
-            Self::Postgres | Self::Sqlite => false,
+            Self::Postgres | Self::Sqlite | Self::DuckDb => false,
         }
     }
 
     /// A block comment inside a block comment has an end of its own.
     fn nests_comments(self) -> bool {
         match self {
-            Self::Postgres => true,
+            Self::Postgres | Self::DuckDb => true,
             Self::MySql | Self::Sqlite => false,
         }
     }
@@ -186,14 +200,14 @@ impl Dialect {
     fn backslash_escapes(self) -> bool {
         match self {
             Self::MySql => true,
-            Self::Postgres | Self::Sqlite => false,
+            Self::Postgres | Self::Sqlite | Self::DuckDb => false,
         }
     }
 
     /// `E'...'` is a string with backslash escapes.
     fn escape_strings(self) -> bool {
         match self {
-            Self::Postgres => true,
+            Self::Postgres | Self::DuckDb => true,
             Self::MySql | Self::Sqlite => false,
         }
     }
@@ -202,7 +216,7 @@ impl Dialect {
     pub fn backtick_names(self) -> bool {
         match self {
             Self::MySql | Self::Sqlite => true,
-            Self::Postgres => false,
+            Self::Postgres | Self::DuckDb => false,
         }
     }
 
@@ -210,14 +224,14 @@ impl Dialect {
     fn bracket_names(self) -> bool {
         match self {
             Self::Sqlite => true,
-            Self::Postgres | Self::MySql => false,
+            Self::Postgres | Self::MySql | Self::DuckDb => false,
         }
     }
 
     /// `$tag$ ... $tag$` is a string and `$1` a parameter.
     fn dollar_quotes(self) -> bool {
         match self {
-            Self::Postgres => true,
+            Self::Postgres | Self::DuckDb => true,
             Self::MySql | Self::Sqlite => false,
         }
     }
@@ -225,7 +239,7 @@ impl Dialect {
     /// `$` can stand inside a word.
     fn dollar_in_words(self) -> bool {
         match self {
-            Self::Postgres | Self::MySql => true,
+            Self::Postgres | Self::MySql | Self::DuckDb => true,
             Self::Sqlite => false,
         }
     }
@@ -288,7 +302,7 @@ fn next(dialect: Dialect, text: &str, start: usize) -> (TokenKind, usize) {
         }
         b'"' => match dialect {
             Dialect::MySql => (TokenKind::String, quoted_end(bytes, start, b'"', true)),
-            Dialect::Postgres | Dialect::Sqlite => (
+            Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => (
                 TokenKind::QuotedIdentifier,
                 quoted_end(bytes, start, b'"', false),
             ),
@@ -345,7 +359,7 @@ fn next(dialect: Dialect, text: &str, start: usize) -> (TokenKind, usize) {
 fn dash_comment(dialect: Dialect, after: Option<u8>) -> bool {
     match dialect {
         Dialect::MySql => after.is_none_or(|b| is_space(b) || b.is_ascii_control()),
-        Dialect::Postgres | Dialect::Sqlite => true,
+        Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => true,
     }
 }
 
@@ -378,7 +392,7 @@ fn is_space(byte: u8) -> bool {
 /// it at a carriage return too; MySQL and SQLite only at a line feed.
 fn line_end(dialect: Dialect, bytes: &[u8], from: usize) -> usize {
     let cr_ends = match dialect {
-        Dialect::Postgres => true,
+        Dialect::Postgres | Dialect::DuckDb => true,
         Dialect::MySql | Dialect::Sqlite => false,
     };
     scan(bytes, from, |b| b != b'\n' && !(cr_ends && b == b'\r'))
@@ -690,7 +704,9 @@ pub fn refusal(dialect: Dialect, statement: &str) -> Option<String> {
             .any(|token| token.kind == TokenKind::ExecutableComment)
             .then(|| "a /*! */ comment".to_owned()),
         // A U&"..." name can spell any name in escapes, set_config included.
-        Dialect::Postgres => unicode_name(statement, &tokens).then(|| "a U& name".to_owned()),
+        Dialect::Postgres | Dialect::DuckDb => {
+            unicode_name(statement, &tokens).then(|| "a U& name".to_owned())
+        }
         Dialect::Sqlite => refused_pragma(statement, &tokens).map(|name| format!("PRAGMA {name}")),
     };
     if hidden.is_some() {
@@ -708,9 +724,30 @@ pub fn refusal(dialect: Dialect, statement: &str) -> Option<String> {
                 words.drain(1..3);
             }
         }
-        Dialect::Postgres | Dialect::Sqlite => {}
+        Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => {}
     }
     let word = |index: usize| words.get(index).map(String::as_str).unwrap_or_default();
+    match dialect {
+        // What outlives the rollback in DuckDB, or reaches past the
+        // connection's files: a setting or a pragma (the fence locks the
+        // database's, not the session's), the default catalog, a
+        // database attached or detached, an extension, a file written.
+        // An `EXPLAIN ANALYZE` runs the statement it explains: one that
+        // names any of them anywhere is refused as that one, whatever the
+        // options between.
+        Dialect::DuckDb => {
+            let refused = |word: &str| DUCKDB_REFUSED.iter().find(|refused| **refused == word);
+            if let Some(first) = refused(word(0)) {
+                return Some((*first).to_owned());
+            }
+            if word(0) == "EXPLAIN"
+                && let Some(named) = words.iter().skip(1).find_map(|word| refused(word))
+            {
+                return Some(format!("EXPLAIN {named}"));
+            }
+        }
+        Dialect::Postgres | Dialect::MySql | Dialect::Sqlite => {}
+    }
     let guarded_name = || words.iter().skip(1).find(|name| is_guarded_setting(name));
     match word(0) {
         first @ ("BEGIN" | "START" | "COMMIT" | "END" | "ROLLBACK" | "ABORT" | "SAVEPOINT"
@@ -721,7 +758,9 @@ pub fn refusal(dialect: Dialect, statement: &str) -> Option<String> {
             Dialect::Postgres if first == "PREPARE" && word(1) == "TRANSACTION" => {
                 return Some("PREPARE TRANSACTION".into());
             }
-            Dialect::Postgres | Dialect::MySql => return Some(first.to_owned()),
+            Dialect::Postgres | Dialect::MySql | Dialect::DuckDb => {
+                return Some(first.to_owned());
+            }
             Dialect::Sqlite => {}
         },
         "SET" => {
@@ -736,6 +775,8 @@ pub fn refusal(dialect: Dialect, statement: &str) -> Option<String> {
                     second => format!("RESET {second}"),
                 });
             }
+            // Refused above, whatever it resets.
+            Dialect::DuckDb => return Some("RESET".into()),
             Dialect::Postgres | Dialect::Sqlite => {
                 if word(1) == "ALL" {
                     return Some("RESET ALL".into());
@@ -747,7 +788,7 @@ pub fn refusal(dialect: Dialect, statement: &str) -> Option<String> {
         },
         "DISCARD" if word(1) == "ALL" => return Some("DISCARD ALL".into()),
         "COPY" => match dialect {
-            Dialect::Postgres => return Some("COPY".into()),
+            Dialect::Postgres | Dialect::DuckDb => return Some("COPY".into()),
             Dialect::MySql | Dialect::Sqlite => {}
         },
         _ => {}
@@ -760,7 +801,7 @@ pub fn refusal(dialect: Dialect, statement: &str) -> Option<String> {
                 .any(|word| word == "SET_CONFIG")
                 .then(|| "set_config".to_owned());
         }
-        Dialect::Sqlite => return None,
+        Dialect::Sqlite | Dialect::DuckDb => return None,
         // The rest is MySQL's.
         Dialect::MySql => {}
     }
@@ -829,6 +870,26 @@ fn is_guarded_setting(name: &str) -> bool {
 /// session is opened in, so refusing it is belt and braces.
 const GUARDED_PRAGMAS: [&str; 2] = ["QUERY_ONLY", "WRITABLE_SCHEMA"];
 
+/// The statements a DuckDB script may not run, by their first word (see
+/// `refusal`). `FORCE` is `FORCE INSTALL`.
+const DUCKDB_REFUSED: [&str; 15] = [
+    "SET",
+    "RESET",
+    "PRAGMA",
+    "USE",
+    "ATTACH",
+    "DETACH",
+    "INSTALL",
+    "FORCE",
+    "LOAD",
+    "UPDATE_EXTENSIONS",
+    "COPY",
+    "EXPORT",
+    "IMPORT",
+    "CHECKPOINT",
+    "VACUUM",
+];
+
 /// The SQLite pragmas a script may not name in any form, since the bare
 /// form acts too. `wal_checkpoint` rewrites a file in WAL mode, and neither
 /// `query_only` nor a transaction that has read nothing stops it.
@@ -920,7 +981,7 @@ fn set_refusal(
                 return Some("SET STATEMENT".into());
             }
         }
-        Dialect::Postgres | Dialect::Sqlite => {}
+        Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => {}
     }
     let has = |wanted: &str| words.iter().skip(1).any(|word| word == wanted);
     if has("CHARACTERISTICS") {
@@ -958,6 +1019,8 @@ fn set_refusal(
             .skip(1)
             .find(|word| matches!(word.as_str(), "GLOBAL" | "PERSIST" | "PERSIST_ONLY"))
             .map(|scope| format!("SET {scope}")),
+        // Every DuckDB SET is refused before this is asked.
+        Dialect::DuckDb => Some("SET".into()),
         Dialect::Postgres | Dialect::Sqlite => None,
     }
 }
@@ -1099,6 +1162,14 @@ pub fn kind(dialect: Dialect, statement: &str) -> StatementKind {
             READS.contains(&word)
                 || match dialect {
                     Dialect::Sqlite => word == "PRAGMA",
+                    // DuckDB reads a table from its FROM alone, and says
+                    // what one holds by DESCRIBE and SUMMARIZE.
+                    Dialect::DuckDb => {
+                        matches!(
+                            word,
+                            "FROM" | "DESCRIBE" | "SUMMARIZE" | "PIVOT" | "UNPIVOT"
+                        )
+                    }
                     Dialect::Postgres | Dialect::MySql => false,
                 }
         }
@@ -1261,7 +1332,7 @@ fn unbounded_at(dialect: Dialect, statement: &str, tokens: &[Token]) -> Option<(
 fn joins_target(dialect: Dialect, verb: &Word, words: &[&Word]) -> bool {
     match dialect {
         Dialect::MySql => {}
-        Dialect::Postgres | Dialect::Sqlite => return false,
+        Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => return false,
     }
     words.iter().enumerate().any(|(at, word)| {
         if word.text != "ON" {
@@ -1355,7 +1426,7 @@ fn code_of(tokens: &[Token]) -> Vec<&Token> {
 /// the dialect has the statement.
 fn truncate_at(dialect: Dialect, statement: &str, tokens: &[Token]) -> Option<usize> {
     let truncates = match dialect {
-        Dialect::Postgres | Dialect::MySql => true,
+        Dialect::Postgres | Dialect::MySql | Dialect::DuckDb => true,
         Dialect::Sqlite => false,
     };
     let (at, first) = tokens
@@ -3142,4 +3213,84 @@ mod tests {
             }
         }
     }
+
+    #[test]
+    fn duckdb_refuses_what_outlives_the_rollback_or_reaches_past_the_files() {
+        for (statement, what) in [
+            ("SET threads = 1", "SET"),
+            ("RESET search_path", "RESET"),
+            ("PRAGMA enable_profiling", "PRAGMA"),
+            ("USE bookshop", "USE"),
+            ("ATTACH 'x.duckdb' AS x", "ATTACH"),
+            ("DETACH bookshop", "DETACH"),
+            ("INSTALL excel", "INSTALL"),
+            ("FORCE INSTALL excel", "FORCE"),
+            ("LOAD excel", "LOAD"),
+            ("COPY orders TO 'orders.csv'", "COPY"),
+            ("EXPORT DATABASE 'out'", "EXPORT"),
+            ("CHECKPOINT", "CHECKPOINT"),
+            ("PREPARE q AS SELECT 1", "PREPARE"),
+            ("BEGIN", "BEGIN"),
+            // What an EXPLAIN ANALYZE runs.
+            (
+                "EXPLAIN ANALYZE COPY (SELECT 1) TO 'orders.csv' (USE_TMP_FILE false)",
+                "EXPLAIN COPY",
+            ),
+            (
+                "EXPLAIN (ANALYZE, FORMAT json) COPY orders TO 'x.csv'",
+                "EXPLAIN COPY",
+            ),
+            ("explain analyze export database 'out'", "EXPLAIN EXPORT"),
+        ] {
+            assert_eq!(
+                refusal(Dialect::DuckDb, statement).as_deref(),
+                Some(what),
+                "{statement}"
+            );
+        }
+        for statement in [
+            "SELECT 'SET' AS word FROM orders",
+            "FROM orders SELECT count(*)",
+            "DESCRIBE orders",
+            "SUMMARIZE orders",
+            "WITH o AS (SELECT 1) SELECT * FROM o",
+            "EXPLAIN ANALYZE SELECT count(*) FROM orders",
+        ] {
+            assert_eq!(refusal(Dialect::DuckDb, statement), None, "{statement}");
+        }
+    }
+
+    #[test]
+    fn duckdb_reads_a_from_first_query_and_its_descriptions_as_reads() {
+        for statement in [
+            "FROM orders",
+            "FROM orders SELECT id",
+            "DESCRIBE orders",
+            "SUMMARIZE orders",
+            "SELECT * FROM orders",
+        ] {
+            assert_eq!(
+                kind(Dialect::DuckDb, statement),
+                StatementKind::Read,
+                "{statement}"
+            );
+        }
+        for statement in ["INSERT INTO t VALUES (1)", "CREATE TABLE t (a INT)"] {
+            assert_eq!(
+                kind(Dialect::DuckDb, statement),
+                StatementKind::Write,
+                "{statement}"
+            );
+        }
+        // Its strings, names and comments read as PostgreSQL's.
+        let tokens = tokenize(
+            Dialect::DuckDb,
+            "SELECT $$a;b$$, E'c\\';', \"d;\" /* /* e; */ */",
+        );
+        assert!(
+            !tokens
+                .iter()
+                .any(|token| token.kind == TokenKind::Semicolon)
+        );
+    }
 }
```

```diff
diff --git a/crates/tabletist-db/src/sql/format.rs b/crates/tabletist-db/src/sql/format.rs
--- a/crates/tabletist-db/src/sql/format.rs
+++ b/crates/tabletist-db/src/sql/format.rs
@@ -728,7 +728,7 @@ impl<'a> Layout<'a> {
     fn structure(&self, index: usize) -> bool {
         match self.dialect {
             Dialect::MySql => false,
-            Dialect::Postgres | Dialect::Sqlite => self.word(index, STRUCTURE),
+            Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => self.word(index, STRUCTURE),
         }
     }
 
@@ -820,7 +820,7 @@ impl<'a> Layout<'a> {
                     && self.items[index - 1].space.is_empty()
                     && minus(index - 2)
             }
-            Dialect::Postgres | Dialect::Sqlite => false,
+            Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => false,
         }
     }
 
```

```diff
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -1026,6 +1026,7 @@ impl Adapter for Conn {
                 foreign_keys: foreign_keys(connection, &object)?,
                 referenced_by: referencing_keys(connection, &object)?,
                 comment: None,
+                file: None,
             })
         })
         .await
```

```diff
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
--- a/crates/tabletist-db/tests/agent.rs
+++ b/crates/tabletist-db/tests/agent.rs
@@ -92,7 +92,7 @@ fn text(text: &str) -> Value {
 /// `$1`, `$2` and so on for PostgreSQL.
 fn placed(connection: &Connection, sql: &str) -> String {
     match connection.dialect() {
-        Dialect::MySql | Dialect::Sqlite => sql.to_owned(),
+        Dialect::MySql | Dialect::Sqlite | Dialect::DuckDb => sql.to_owned(),
         Dialect::Postgres => {
             let mut number = 0;
             sql.chars()
@@ -277,6 +277,7 @@ async fn a_read_checked_for_another_database_is_not_run(connection: &Connection,
         Dialect::Postgres => Dialect::MySql,
         Dialect::MySql => Dialect::Sqlite,
         Dialect::Sqlite => Dialect::Postgres,
+        Dialect::DuckDb => Dialect::Sqlite,
     };
     let read = agent::check_read(other, "SELECT title FROM books", &[]).unwrap();
     let error = connection
@@ -339,7 +340,7 @@ async fn a_hidden_table_is_never_read(connection: &Connection, on: &str) {
     // table expression, and through a view.
     let quoted = match connection.dialect() {
         Dialect::MySql => "SELECT * FROM `CUSTOMERS`",
-        Dialect::Postgres | Dialect::Sqlite => "SELECT * FROM \"CUSTOMERS\"",
+        Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => "SELECT * FROM \"CUSTOMERS\"",
     };
     for sql in [
         "SELECT * FROM customers",
@@ -371,7 +372,9 @@ async fn a_hidden_table_is_never_read(connection: &Connection, on: &str) {
     // hidden, open where none is.
     let catalog = match connection.dialect() {
         Dialect::Sqlite => "SELECT count(*) FROM sqlite_master",
-        Dialect::Postgres | Dialect::MySql => "SELECT count(*) FROM information_schema.tables",
+        Dialect::Postgres | Dialect::MySql | Dialect::DuckDb => {
+            "SELECT count(*) FROM information_schema.tables"
+        }
     };
     let error = hiding(connection, &customers, catalog).await.unwrap_err();
     assert_eq!(error, Error::Agent(Refusal::SystemCatalog), "{on}");
@@ -463,12 +466,12 @@ async fn a_hidden_table_is_never_read(connection: &Connection, on: &str) {
             let error = hiding(connection, &customers, hinted).await.unwrap_err();
             assert_eq!(error, hidden_refusal("customers", None), "{on}");
         }
-        Dialect::Sqlite => {}
+        Dialect::Sqlite | Dialect::DuckDb => {}
     }
     // What is hidden, asked outside a statement, as the server asks it to
     // leave hidden tables out of what it says of the schema.
     let home = match connection.dialect() {
-        Dialect::Sqlite => "main",
+        Dialect::Sqlite | Dialect::DuckDb => "main",
         Dialect::Postgres => "public",
         Dialect::MySql => "bookshop",
     };
@@ -586,6 +589,7 @@ async fn a_masked_column_is_used_only_as_it_is(connection: &Connection, on: &str
         Dialect::Postgres => "SELECT count(*) FROM pg_catalog.pg_class",
         Dialect::MySql => "SELECT count(*) FROM information_schema.TABLES",
         Dialect::Sqlite => "SELECT count(*) FROM sqlite_master",
+        Dialect::DuckDb => "SELECT count(*) FROM duckdb_tables()",
     };
     assert_eq!(
         masking(connection, &email, catalog).await.unwrap_err(),
```

```diff
diff --git a/crates/tabletist-db/tests/ddl.rs b/crates/tabletist-db/tests/ddl.rs
--- a/crates/tabletist-db/tests/ddl.rs
+++ b/crates/tabletist-db/tests/ddl.rs
@@ -819,8 +819,31 @@ mod mysql {
 /// TABLE` first: named by `Driver`, so a new engine is named here too.
 #[tokio::test]
 async fn every_engine_reads_book_covers() {
-    for driver in [Driver::Sqlite, Driver::Postgres, Driver::MySql] {
+    let exports = tempfile::tempdir().unwrap();
+    tabletist_db::fixtures::write_bookshop_files(exports.path()).unwrap();
+    for driver in [
+        Driver::Sqlite,
+        Driver::Postgres,
+        Driver::MySql,
+        Driver::Files,
+    ] {
         let ddl = match driver {
+            // The Bookshop's exports keep `book_covers` in a `.duckdb`
+            // file, a schema of the connection.
+            Driver::Files => {
+                let spec = ConnectSpec::files(tabletist_db::files::Source::Folder(
+                    exports.path().to_owned(),
+                ));
+                let connection = Connection::connect(&spec, &Secrets::default())
+                    .await
+                    .unwrap();
+                Some(
+                    connection
+                        .ddl(&ObjectRef::new("bookshop", "book_covers"))
+                        .await
+                        .unwrap(),
+                )
+            }
             Driver::Sqlite => Some(sqlite::ddl("book_covers").await),
             Driver::Postgres => match postgres::bookshop().await {
                 Some(connection) => Some(
```

```diff
diff --git a/crates/tabletist-mcp/src/tools/connections.rs b/crates/tabletist-mcp/src/tools/connections.rs
--- a/crates/tabletist-mcp/src/tools/connections.rs
+++ b/crates/tabletist-mcp/src/tools/connections.rs
@@ -58,6 +58,8 @@ fn entry(listed: &Listed) -> Json {
         Dialect::Postgres => ("PostgreSQL", "$1, $2"),
         Dialect::MySql => ("MySQL", "?"),
         Dialect::Sqlite => ("SQLite", "?"),
+        // A Files connection: its files are DuckDB's tables.
+        Dialect::DuckDb => ("DuckDB", "?"),
     };
     let mut entry = Map::new();
     entry.insert("name".into(), json!(listed.name));
```

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -442,6 +442,7 @@ An administrator's policy, `managed.toml`, is in a third place of its own:
 | `labels.json` | State | The column you chose to label each table's rows in a foreign key's list, by saved connection and, on PostgreSQL, by database |
 | `keys.default.toml` | State | Every key in force, commented, as `:keys default` writes it for you to copy lines from. Written again each time, and never read |
 | `tabletist.log`, `panic.log` | State | Logs |
+| `duckdb-extensions/` | State | The readers DuckDB installs for Files connections the first time one needs them: Excel's |
 | `agents.sqlite3` | State | Every call agents made: the tool, the statement and its values, and how it ended. Never a row |
 | `history.sqlite3` | State | The query history, by saved connection: each statement run from the SQL editor, a grid's save (its whole transaction, with the values you typed written in) and a filter or sort you set, with when, how long, its rows and how it ended, and a filter's table, filter and sort. An agent's run is kept as its call's place in `agents.sqlite3`, its tool, rows, time and result, never what it sent, its name included: the History tab reads that from `agents.sqlite3`. Never a result's rows. Kept to `editor.history.keep` per connection, and written on its own: a statement never waits for it |
 | `agents_shown.json` | State | What agents may do as Tabletist last showed it: whether the server is on, and each connection's access, hidden tables and masked columns |
@@ -500,6 +501,27 @@ driver      = "sqlite"
 sqlite_path = "/home/ada/bookshop.db"
 ```
 
+A Files connection names its folder, or its files, in a
+`[connections.source]` table of its own, a path under your home folder
+written with `~`:
+
+```toml
+[[connections]]
+id          = "c2f1a9d4-1b7e-4f31-8e0a-5d4e2b9c7a10"
+name        = "Bookshop exports"
+environment = "local"
+read_only   = true
+driver      = "files"
+
+[connections.source]
+folder = "~/Data/bookshop-exports"
+```
+
+`files = ["~/Data/bookshop-exports/orders.csv", "…"]` takes the place of
+`folder` for a list of files. A source has one of the two, and only a
+Files connection has one. A file that holds a Files connection says
+`version = 7`.
+
 | Key | Values | Without it |
 | --- | --- | --- |
 | `id` | Any text that no other connection has | Tabletist gives the connection one at the next start |
@@ -508,7 +530,7 @@ sqlite_path = "/home/ada/bookshop.db"
 | `read_only` | `true`, `false` | As the environment has it: read-only on production |
 | `password` | `none`, `keyring`, `ask` | `none` |
 | `ssh_secret` | `none`, `keyring`, `ask` | `none` |
-| `driver` | `postgres`, `mysql`, `sqlite` | Required |
+| `driver` | `postgres`, `mysql`, `sqlite`, `files` | Required |
 | `host`, `user`, `database` | Text | Empty |
 | `port` | A number up to 65535 | 5432 for PostgreSQL, 3306 for MySQL |
 | `tls` | `disable`, `prefer`, `require`, `verify-ca`, `verify-full` | `prefer` |
@@ -692,9 +714,13 @@ access_max = "off"
 A rule matches a connection by what it reaches. Each key in `match` must
 match, and `*` in it stands for any run of characters. `host` is compared
 without regard to case, and through an SSH tunnel it is the database's
-host. `path` is a SQLite file's, with `~` read as your home folder. A key
-the connection has no use for, such as `path` on PostgreSQL, does not
-match. Every rule that matches a connection applies.
+host. `path` is a SQLite file's, or a Files connection's folder (its first
+file's for a list of files), with `~` read as your home folder. `driver`
+is `postgres`, `mysql`, `sqlite` or `files`. A key the connection has no
+use for, such as `path` on PostgreSQL, does not match. Every rule that
+matches a connection applies. A Files connection can hide no table and
+mask no column: while a rule that matches one has `hidden`,
+`visible_only` or `masked`, agents may not use it at all.
 
 A rule's tables and columns apply beside your own. A table is visible
 to agents only if your Tables setting and every matching rule let it be
````

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -2904,7 +2904,7 @@ impl App {
                     let old_default = form.driver.default_port().to_string();
                     form.driver = driver;
                     match driver {
-                        Driver::Sqlite => {}
+                        Driver::Sqlite | Driver::Files => {}
                         Driver::Postgres | Driver::MySql => {
                             if form.port.trim().is_empty() || form.port.trim() == old_default {
                                 form.port = driver.default_port().to_string();
@@ -5902,7 +5902,7 @@ impl App {
         };
         let asks = match workspace.driver {
             Driver::MySql => workspace.session_zone.needs_load(),
-            Driver::Postgres | Driver::Sqlite => false,
+            Driver::Postgres | Driver::Sqlite | Driver::Files => false,
         };
         if asks {
             let session = workspace.session;
@@ -5929,7 +5929,7 @@ impl App {
                     self.backend
                         .send(Command::ListDatabases { session, request });
                 }
-                Driver::MySql | Driver::Sqlite => {}
+                Driver::MySql | Driver::Sqlite | Driver::Files => {}
             }
         }
         self.ask_server_version(tab);
@@ -7137,9 +7137,9 @@ impl App {
                             (named.name.clone(), true)
                         }
                         tabletist_db::Dialect::Postgres => (named.name.to_lowercase(), true),
-                        tabletist_db::Dialect::MySql | tabletist_db::Dialect::Sqlite => {
-                            (named.name.clone(), false)
-                        }
+                        tabletist_db::Dialect::MySql
+                        | tabletist_db::Dialect::Sqlite
+                        | tabletist_db::Dialect::DuckDb => (named.name.clone(), false),
                     };
                     let object = crate::completion::resolve(schema, &name, &catalog)
                         .filter(|object| !exact || object.name == name)?;
@@ -8306,6 +8306,8 @@ fn apply_url(form: &mut ConnectionForm) -> Result<(), String> {
             form.driver = Driver::Sqlite;
             form.sqlite_path = path;
         }
+        // No URL names a Files connection.
+        Driver::Files => {}
         Driver::Postgres | Driver::MySql => {
             if form.name.trim().is_empty() {
                 form.name = spec.summary();
@@ -8515,7 +8517,7 @@ fn logged_statement(driver: Driver, sql: &str, params: Option<&str>) -> String {
                 let named = match driver {
                     Driver::Postgres => format!("${place}"),
                     Driver::Sqlite => format!("?{place}"),
-                    Driver::MySql => format!("? #{place}"),
+                    Driver::MySql | Driver::Files => format!("? #{place}"),
                 };
                 text.push_str(&format!("-- {named} = {value}\n"));
             }
```

```diff
diff --git a/src/connections/file.rs b/src/connections/file.rs
--- a/src/connections/file.rs
+++ b/src/connections/file.rs
@@ -12,6 +12,7 @@ use std::path::{Path, PathBuf};
 
 use serde::Deserialize;
 use tabletist_db::agent::{ColumnName, TableName, Tables};
+use tabletist_db::files::Source;
 use tabletist_db::{ConnectSpec, Driver, SshAuth, SshSpec, TlsMode};
 
 use super::{
@@ -23,12 +24,16 @@ use crate::util::toml_string;
 
 /// The file's format. 3 is the first that is TOML: 1 and 2 were
 /// `connections.json`. 4 has a connection's settings for agents, 5 the
-/// tables they may not read, and 6 the columns they get masked. A file is
+/// tables they may not read, 6 the columns they get masked, and 7 Files
+/// connections, with their `source`. A file is
 /// written as the oldest that holds what it has: a Tabletist that does not
 /// know a key refuses only a file it would lose it from, and one that
 /// would read agents' tables as all of them, or a masked column's values
 /// as they are, never reads the file.
-pub(super) const VERSION: u32 = 6;
+pub(super) const VERSION: u32 = 7;
+
+/// The version that has masked columns and no Files connection.
+const MASKED_VERSION: u32 = 6;
 
 /// The version that has agents' tables and not their masked columns.
 const TABLES_VERSION: u32 = 5;
@@ -113,6 +118,36 @@ struct Entry {
     ca_file: Option<PathBuf>,
     ssh: Option<Tunnel>,
     agents: Option<AgentsTable>,
+    source: Option<SourceTable>,
+}
+
+/// What a Files connection opens, as the file has it: a folder or a list
+/// of files, either written with `~` for the home folder.
+#[derive(Deserialize)]
+#[serde(deny_unknown_fields)]
+struct SourceTable {
+    folder: Option<PathBuf>,
+    files: Option<Vec<PathBuf>>,
+}
+
+impl SourceTable {
+    fn source(self, connection: &str) -> Result<Source, String> {
+        match (self.folder, self.files) {
+            (Some(folder), None) => Ok(Source::Folder(tabletist_db::files::expanded(&folder))),
+            (None, Some(files)) if !files.is_empty() => Ok(Source::Files(
+                files
+                    .iter()
+                    .map(|file| tabletist_db::files::expanded(file))
+                    .collect(),
+            )),
+            (None, Some(_)) => Err(format!(
+                "connection {connection:?}: its source's `files` names no file"
+            )),
+            (Some(_), Some(_)) | (None, None) => Err(format!(
+                "connection {connection:?}: its source has a `folder` or `files`, one of them"
+            )),
+        }
+    }
 }
 
 /// What agents may do on a connection, as the file has it. A key left out
@@ -236,6 +271,32 @@ impl Entry {
             Some(table) => table.agents(&self.name)?,
             None => Agents::default(),
         };
+        // A Files connection keeps neither: nothing could hold a statement
+        // to them, and the dialog has no control to take them off.
+        let guards = agents.tables.hides_anything() || !agents.masked.is_empty();
+        let files = match (self.driver, self.source) {
+            (Driver::Files, Some(_)) if guards => {
+                return Err(format!(
+                    "connection {:?}: a Files connection cannot hide tables from agents or \
+                     mask columns",
+                    self.name
+                ));
+            }
+            (Driver::Files, Some(source)) => Some(source.source(&self.name)?),
+            (Driver::Files, None) => {
+                return Err(format!(
+                    "connection {:?}: a Files connection needs a [connections.source] table",
+                    self.name
+                ));
+            }
+            (Driver::Postgres | Driver::MySql | Driver::Sqlite, Some(_)) => {
+                return Err(format!(
+                    "connection {:?}: only a Files connection has a source",
+                    self.name
+                ));
+            }
+            (Driver::Postgres | Driver::MySql | Driver::Sqlite, None) => None,
+        };
         Ok(SavedConnection {
             id,
             name: self.name,
@@ -253,6 +314,7 @@ impl Entry {
                 tls: self.tls,
                 ca_file: self.ca_file,
                 ssh,
+                files,
             },
             agents,
         })
@@ -388,6 +450,7 @@ fn driver(driver: Driver) -> &'static str {
         Driver::Postgres => "postgres",
         Driver::MySql => "mysql",
         Driver::Sqlite => "sqlite",
+        Driver::Files => "files",
     }
 }
 
@@ -413,18 +476,20 @@ pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String>
         .iter()
         .any(|saved| saved.agents.tables.hides_anything());
     let agents = connections.iter().any(|saved| !saved.agents.is_default());
-    let version = match (masks, hides, agents) {
-        (true, _, _) => VERSION,
-        (false, true, _) => TABLES_VERSION,
-        (false, false, true) => AGENTS_VERSION,
-        (false, false, false) => FIRST_VERSION,
+    let files = connections.iter().any(|saved| saved.spec.files.is_some());
+    let version = match (files, masks, hides, agents) {
+        (true, _, _, _) => VERSION,
+        (false, true, _, _) => MASKED_VERSION,
+        (false, false, true, _) => TABLES_VERSION,
+        (false, false, false, true) => AGENTS_VERSION,
+        (false, false, false, false) => FIRST_VERSION,
     };
     let mut out = format!("{HEADER}version = {version}\n");
     for connection in connections {
         let spec = &connection.spec;
         let server = match spec.driver {
             Driver::Postgres | Driver::MySql => true,
-            Driver::Sqlite => false,
+            Driver::Sqlite | Driver::Files => false,
         };
         let mut lines = Lines::default();
         lines.text("id", &connection.id.0);
@@ -466,6 +531,28 @@ pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String>
         if let Some(ssh) = &spec.ssh {
             tunnel(ssh, &mut out)?;
         }
+        if let Some(source) = &spec.files {
+            let mut lines = Lines::default();
+            match source {
+                Source::Folder(folder) => lines.path("folder", &home_tilde(folder))?,
+                Source::Files(files) => {
+                    let mut written = Vec::with_capacity(files.len());
+                    for file in files {
+                        let tilde = home_tilde(file);
+                        let Some(text) = tilde.to_str() else {
+                            return Err(std::io::Error::other(format!(
+                                "{} is not valid UTF-8, so it cannot be saved",
+                                file.display()
+                            )));
+                        };
+                        written.push(toml_string(text));
+                    }
+                    lines.bare("files", format!("[{}]", written.join(", ")));
+                }
+            }
+            out.push_str("\n[connections.source]\n");
+            lines.write(&mut out);
+        }
         if !connection.agents.is_default() {
             let mut lines = Lines::default();
             lines.text("access", connection.agents.access.key());
@@ -487,6 +574,15 @@ pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String>
     Ok(out)
 }
 
+/// `path` with the home folder written `~`, as the file keeps a source.
+fn home_tilde(path: &Path) -> PathBuf {
+    let shown = tabletist_db::files::shown(path);
+    match shown.starts_with('~') {
+        true => PathBuf::from(shown),
+        false => path.to_owned(),
+    }
+}
+
 /// Tables or columns as a list of TOML strings, each written as the person
 /// writes it.
 fn entries<T: std::fmt::Display>(names: &[T]) -> String {
@@ -927,8 +1023,8 @@ auth = "key-file"
         let said = |text: &str| read(text).unwrap_err().to_string();
         assert!(said("").contains("missing field `version`"), "{}", said(""));
         assert_eq!(
-            said("version = 7\n"),
-            "it is version 7 of the file, written by a newer Tabletist; this one reads version 6"
+            said("version = 8\n"),
+            "it is version 8 of the file, written by a newer Tabletist; this one reads version 7"
         );
         assert_eq!(
             said("version = 2\n"),
@@ -1213,4 +1309,112 @@ masked       = ["orders.email", "\"odd.schema\".orders.phone"]
             "{error}"
         );
     }
+
+    /// A Files connection by its folder, and one by its files, as the file
+    /// writes them: version 7, the home folder as `~`.
+    #[test]
+    fn a_files_connection_is_written_with_its_source_in_version_seven() {
+        let Some(home) = std::env::home_dir() else {
+            return;
+        };
+        let exports = home.join("Data").join("bookshop-exports");
+        let folder = SavedConnection {
+            environment: Environment::Local,
+            read_only: Some(true),
+            ..saved(
+                "exports",
+                "Bookshop exports",
+                ConnectSpec::files(Source::Folder(exports.clone())),
+            )
+        };
+        let list = saved(
+            "orders",
+            "Bookshop orders",
+            ConnectSpec::files(Source::Files(vec![
+                exports.join("orders.csv"),
+                PathBuf::from("/srv/exports/customers.parquet"),
+            ])),
+        );
+        let text = write(&[folder.clone(), list.clone()]).unwrap();
+        let sep = std::path::MAIN_SEPARATOR;
+        assert!(text.contains("\nversion = 7\n"), "{text}");
+        let expected = format!(
+            "\n[[connections]]\n\
+             id          = \"exports\"\n\
+             name        = \"Bookshop exports\"\n\
+             environment = \"local\"\n\
+             read_only   = true\n\
+             driver      = \"files\"\n\
+             \n[connections.source]\n\
+             folder = \"~{sep}Data{sep}bookshop-exports\"\n"
+        );
+        assert!(text.contains(&expected.replace('\\', "\\\\")), "{text}");
+        assert!(
+            text.contains(
+                &format!(
+                    "files = [\"~{sep}Data{sep}bookshop-exports{sep}orders.csv\", \
+                 \"/srv/exports/customers.parquet\"]"
+                )
+                .replace('\\', "\\\\")
+            ),
+            "{text}"
+        );
+        let read = read(&text).unwrap();
+        assert_eq!(read.connections, [folder, list]);
+        // Without one, the file is as it was.
+        let others = write(&bookshop()).unwrap();
+        assert!(!others.contains("version = 7"), "{others}");
+    }
+
+    #[test]
+    fn a_source_is_a_folder_or_files_and_only_a_files_connections() {
+        let entry = |driver: &str, source: &str| {
+            format!(
+                "version = 7\n[[connections]]\nid = \"a\"\nname = \"A\"\n\
+                 driver = \"{driver}\"\n{source}"
+            )
+        };
+        for (text, says) in [
+            (entry("files", ""), "needs a [connections.source] table"),
+            (
+                entry("files", "[connections.source]\n"),
+                "has a `folder` or `files`, one of them",
+            ),
+            (
+                entry(
+                    "files",
+                    "[connections.source]\nfolder = \"/a\"\nfiles = [\"/b.csv\"]\n",
+                ),
+                "has a `folder` or `files`, one of them",
+            ),
+            (
+                entry("files", "[connections.source]\nfiles = []\n"),
+                "names no file",
+            ),
+            (
+                entry("sqlite", "[connections.source]\nfolder = \"/a\"\n"),
+                "only a Files connection has a source",
+            ),
+            (
+                entry(
+                    "files",
+                    "[connections.source]\nfolder = \"/a\"\n[connections.agents]\n\
+                     access = \"read\"\nhidden = [\"customers\"]\n",
+                ),
+                "cannot hide tables from agents or mask columns",
+            ),
+        ] {
+            let problem = read(&text).unwrap_err();
+            assert!(problem.what.contains(says), "{text}\n{problem}");
+        }
+        let read = read(&entry(
+            "files",
+            "[connections.source]\nfolder = \"/data/exports\"\n",
+        ))
+        .unwrap();
+        assert_eq!(
+            read.connections[0].spec.files,
+            Some(Source::Folder("/data/exports".into()))
+        );
+    }
 }
```

```diff
diff --git a/src/dates.rs b/src/dates.rs
--- a/src/dates.rs
+++ b/src/dates.rs
@@ -90,6 +90,16 @@ pub fn kind(dialect: Dialect, type_name: &str) -> Option<Kind> {
             "time" => Some(Kind::Time { offset: false }),
             _ => None,
         },
+        // DuckDB's names, `TIMESTAMP_MS` and the other units among them.
+        Dialect::DuckDb => match name.as_str() {
+            "date" => Some(Kind::Date),
+            "timestamp" | "datetime" | "timestamp_us" | "timestamp_ms" | "timestamp_s"
+            | "timestamp_ns" => Some(Kind::Timestamp),
+            "timestamptz" | "timestamp with time zone" => Some(Kind::Zoned),
+            "time" => Some(Kind::Time { offset: false }),
+            "timetz" | "time with time zone" => Some(Kind::Time { offset: true }),
+            _ => None,
+        },
         // SQLite has no such types: the declared name says what the
         // column is meant to keep.
         Dialect::Sqlite => {
@@ -122,7 +132,8 @@ fn kept_digits(dialect: Dialect, type_name: &str) -> u8 {
     let stated = stated_digits(type_name);
     match (dialect, stated) {
         (Dialect::Postgres | Dialect::MySql, Some(digits)) => digits.min(6),
-        (Dialect::Postgres, None) => 6,
+        (Dialect::Postgres | Dialect::DuckDb, None) => 6,
+        (Dialect::DuckDb, Some(digits)) => digits.min(6),
         (Dialect::MySql, None) => 0,
         (Dialect::Sqlite, _) => 9,
     }
@@ -264,7 +275,7 @@ impl Column {
         let kind = kind(dialect, type_name)?;
         let stored = match dialect {
             Dialect::Sqlite => Stored::of(values),
-            Dialect::Postgres | Dialect::MySql => Stored::Fresh,
+            Dialect::Postgres | Dialect::MySql | Dialect::DuckDb => Stored::Fresh,
         };
         // A number such as `20260115` is a day's, and no time's.
         let stored = match (stored, kind) {
@@ -554,11 +565,12 @@ fn written_time(time: Time, digits: u8) -> String {
 }
 
 /// Whether the engine writes a year before 1 AD with its era (`0044 BC`):
-/// PostgreSQL alone. SQLite's year 0 is `0000`, and MySQL holds none.
+/// PostgreSQL alone. SQLite's year 0 is `0000`, MySQL holds none, and
+/// DuckDB writes its own `(BC)`, which a typed value never needs.
 fn eras(dialect: Dialect) -> bool {
     match dialect {
         Dialect::Postgres => true,
-        Dialect::MySql | Dialect::Sqlite => false,
+        Dialect::MySql | Dialect::Sqlite | Dialect::DuckDb => false,
     }
 }
 
@@ -638,10 +650,12 @@ fn reading(typed: &str, column: &Column, clock: &Clock) -> Result<Reading, Refus
 /// The words a value that is no day is typed as, and SQLite's seconds.
 fn special(text: &str, column: &Column) -> Result<Option<Moment>, Refusal> {
     match column.dialect {
-        Dialect::Postgres if column.kind.has_time() && !column.kind.has_date() => {
+        Dialect::Postgres | Dialect::DuckDb
+            if column.kind.has_time() && !column.kind.has_date() =>
+        {
             Ok(end_of_day(text, column))
         }
-        Dialect::Postgres => Ok(match text {
+        Dialect::Postgres | Dialect::DuckDb => Ok(match text {
             "infinity" | "+infinity" => Some(Moment::Infinity),
             "-infinity" => Some(Moment::MinusInfinity),
             _ => None,
@@ -1305,7 +1319,9 @@ fn within(column: &Column, at: DateTime, instant: Option<Timestamp>) -> Result<(
             (instant < first || instant > last).then_some(Range::MySqlTimestamp)
         }
         // SQLite's dates run from 0000 to 9999, and the years read do.
-        (Dialect::Sqlite, _) | (Dialect::Postgres | Dialect::MySql, Kind::Time { .. }) => None,
+        // DuckDB's run millions of years either way.
+        (Dialect::Sqlite | Dialect::DuckDb, _)
+        | (Dialect::Postgres | Dialect::MySql, Kind::Time { .. }) => None,
     };
     refused.map_or(Ok(()), |range| Err(Refusal::Range(range)))
 }
@@ -1369,8 +1385,9 @@ fn resolve(parts: Parts, column: &Column, clock: &Clock) -> Result<Reading, Refu
             if parts.offset.is_some() && !keeps {
                 return Err(Refusal::NoZone);
             }
+            // DuckDB keeps PostgreSQL's limit on an offset too.
             let postgres = match column.dialect {
-                Dialect::Postgres => true,
+                Dialect::Postgres | Dialect::DuckDb => true,
                 Dialect::MySql | Dialect::Sqlite => false,
             };
             if let Some(offset) = parts.offset
@@ -1387,11 +1404,11 @@ fn resolve(parts: Parts, column: &Column, clock: &Clock) -> Result<Reading, Refu
                 // MySQL keeps as a span of a day.
                 reading.moment = match column.dialect {
                     Dialect::MySql => Moment::Span("24:00:00".to_owned()),
-                    Dialect::Postgres | Dialect::Sqlite => Moment::EndOfDay,
+                    Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => Moment::EndOfDay,
                 };
                 reading.literal = match column.dialect {
                     Dialect::MySql => "24:00:00".to_owned(),
-                    Dialect::Postgres | Dialect::Sqlite => day_end(column.stored),
+                    Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => day_end(column.stored),
                 };
             } else {
                 let time = at.time();
@@ -1420,7 +1437,7 @@ fn resolve(parts: Parts, column: &Column, clock: &Clock) -> Result<Reading, Refu
             // SQLite keeps the text it is given, an offset too.
             let keeps_any = match column.dialect {
                 Dialect::Sqlite => true,
-                Dialect::Postgres | Dialect::MySql => false,
+                Dialect::Postgres | Dialect::MySql | Dialect::DuckDb => false,
             };
             if parts.offset.is_some() && !keeps_any {
                 return Err(Refusal::NoZone);
@@ -1598,7 +1615,7 @@ fn zoned(
     // A MySQL literal has no offset to choose one of the two moments by:
     // the server takes the one it takes, and nothing is offered.
     reading.fold = match column.dialect {
-        Dialect::Postgres | Dialect::Sqlite => fold(zone, at),
+        Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => fold(zone, at),
         Dialect::MySql => None,
     };
     let (day, bc) = written_date(at.date(), column.dialect);
@@ -1607,7 +1624,7 @@ fn zoned(
         // The server reads a MySQL value in the session's zone, which is
         // this one: it is sent as that zone's clock writes it.
         Dialect::MySql => format!("{day} {time}"),
-        Dialect::Postgres | Dialect::Sqlite => {
+        Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => {
             era((format!("{day} {time}{}", written_offset(offset)), bc))
         }
     };
@@ -1789,7 +1806,7 @@ fn folded(at: DateTime, offset: Option<Offset>, column: &Column) -> Option<Offse
     let offset = offset?;
     let two = column.zone.as_ref().and_then(|zone| fold(zone, at))?;
     let sent = match column.dialect {
-        Dialect::Postgres | Dialect::Sqlite => true,
+        Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => true,
         Dialect::MySql => false,
     };
     (sent && (two.earlier.0 == offset || two.later.0 == offset)).then_some(offset)
```

```diff
diff --git a/src/edit.rs b/src/edit.rs
--- a/src/edit.rs
+++ b/src/edit.rs
@@ -205,7 +205,7 @@ impl Table<'_> {
         // (or is one the database does not know): it is sent with an
         // offset all the same, which the server reads as it is.
         let zone = match self.dialect {
-            Dialect::Postgres => Some(self.zone.unwrap_or(&UTC)),
+            Dialect::Postgres | Dialect::DuckDb => Some(self.zone.unwrap_or(&UTC)),
             Dialect::MySql => self.session_zone,
             Dialect::Sqlite => None,
         };
@@ -264,7 +264,7 @@ impl Table<'_> {
             Dialect::Sqlite => key.iter().any(
                 |col| matches!(held(col), Some(Value::Text(text)) if text.contains('\u{FFFD}')),
             ),
-            Dialect::Postgres | Dialect::MySql => false,
+            Dialect::Postgres | Dialect::MySql | Dialect::DuckDb => false,
         };
         if inexact {
             return Some(Lock::KeyInexact);
@@ -667,7 +667,7 @@ impl Table<'_> {
                     .to_ascii_lowercase();
                 matches!(word.as_str(), "timestamp" | "bit" | "float")
             }),
-            Dialect::Postgres | Dialect::Sqlite => false,
+            Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => false,
         }
     }
 }
@@ -754,12 +754,13 @@ pub fn check(dialect: Dialect, column: &ColumnInfo, text: &str) -> Option<Proble
         ColumnClass::Decimal { precision, scale } => {
             decimal(typed, precision, scale).or_else(|| match dialect {
                 Dialect::Sqlite => inexact(typed),
-                Dialect::Postgres | Dialect::MySql => None,
+                Dialect::Postgres | Dialect::MySql | Dialect::DuckDb => None,
             })
         }
         ColumnClass::Float => {
             let word = match dialect {
-                Dialect::Postgres => float_word(typed),
+                // DuckDB reads PostgreSQL's words for the infinities too.
+                Dialect::Postgres | Dialect::DuckDb => float_word(typed),
                 Dialect::MySql | Dialect::Sqlite => false,
             };
             let number = typed.parse::<f64>().is_ok_and(f64::is_finite);
@@ -812,7 +813,7 @@ fn boolean(dialect: Dialect, typed: &str) -> Option<Problem> {
             Ok(_) => None,
             Err(_) => Some(Problem::Boolean),
         },
-        Dialect::Postgres | Dialect::Sqlite => {
+        Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => {
             (!matches!(typed, "1" | "0")).then_some(Problem::Boolean)
         }
     }
@@ -1035,7 +1036,8 @@ pub fn same_moment_typed(loaded: &Value, new: &NewValue) -> bool {
 /// stored, and is shown and edited as it is.
 pub fn shows_in_zone(dialect: Dialect) -> bool {
     match dialect {
-        Dialect::Postgres => true,
+        // DuckDB's are written in UTC with their offset, as PostgreSQL's.
+        Dialect::Postgres | Dialect::DuckDb => true,
         Dialect::MySql | Dialect::Sqlite => false,
     }
 }
@@ -1389,7 +1391,7 @@ pub fn default_shown(dialect: Dialect, default: &str) -> Option<String> {
         // PostgreSQL writes a NULL with its type.
         let bare = match dialect {
             Dialect::Postgres => text.split("::").next().unwrap_or(text),
-            Dialect::MySql | Dialect::Sqlite => text,
+            Dialect::MySql | Dialect::Sqlite | Dialect::DuckDb => text,
         };
         return (!bare.eq_ignore_ascii_case("null")).then(|| text.to_owned());
     };
@@ -1420,7 +1422,7 @@ pub fn default_shown(dialect: Dialect, default: &str) -> Option<String> {
                     .chars()
                     .all(|letter| letter.is_ascii_alphanumeric() || " _.\"(),[]".contains(letter))
         }),
-        Dialect::MySql | Dialect::Sqlite => false,
+        Dialect::MySql | Dialect::Sqlite | Dialect::DuckDb => false,
     };
     Some(if after.is_empty() || cast {
         value
@@ -1446,7 +1448,7 @@ pub fn default_is_expression(dialect: Dialect, default: &str) -> bool {
     };
     match dialect {
         // Shown as it is written: no string was read out of it.
-        Dialect::Postgres | Dialect::Sqlite => {
+        Dialect::Postgres | Dialect::Sqlite | Dialect::DuckDb => {
             default_shown(dialect, text).is_some_and(|shown| shown == text) && !value(text)
         }
         Dialect::MySql => text.contains('(') || text.to_ascii_uppercase().starts_with("CURRENT_"),
@@ -1779,7 +1781,7 @@ pub fn names_table(dialect: Dialect, schema: &str, table: &str, object: &ObjectR
         // pointing at the other, on a server that tells them apart.
         Dialect::MySql => a.eq_ignore_ascii_case(b),
         // A name is matched without regard to ASCII case.
-        Dialect::Sqlite => a.eq_ignore_ascii_case(b),
+        Dialect::Sqlite | Dialect::DuckDb => a.eq_ignore_ascii_case(b),
     };
     (schema.is_empty() || same(schema, &object.schema)) && same(table, &object.name)
 }
@@ -1896,6 +1898,8 @@ pub fn is_held(dialect: Dialect, error: &Error) -> bool {
             code == Some("787")
                 || (code == Some("1811") && message == "FOREIGN KEY constraint failed")
         }
+        // DuckDB gives no code: its own words, which no setting changes.
+        Dialect::DuckDb => message.contains("Violates foreign key constraint"),
     }
 }
 
```

```diff
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -122,6 +122,11 @@ pub fn main(cli: Cli, launch: fastframe_update::Launch) -> anyhow::Result<()> {
         Some(root) => AppDirs::at(root.path()),
         None => AppDirs::discover(),
     };
+    // Files connections: the readers DuckDB installs (Excel's) are kept in
+    // the app's own folder, and a path under the home folder is shown with
+    // `~`, found once here and never in a frame.
+    tabletist_db::set_extension_directory(dirs.duckdb_extensions_dir());
+    tabletist_db::files::set_home(dirs.home.clone());
     let mut logging = fastframe_log::Logging::new("tabletist", env!("CARGO_PKG_VERSION"))
         .filter(log_filter(cli.verbose))
         .redact(without_agents_words);
```

```diff
diff --git a/src/env.rs b/src/env.rs
--- a/src/env.rs
+++ b/src/env.rs
@@ -44,7 +44,7 @@ impl Environment {
     /// [`Environment::for_spec`] from the parts the dialog holds.
     pub fn for_target(driver: Driver, host: &str, tunnelled: bool) -> Self {
         let local = match driver {
-            Driver::Sqlite => true,
+            Driver::Sqlite | Driver::Files => true,
             Driver::Postgres | Driver::MySql => !tunnelled && crate::model::is_local_host(host),
         };
         if local { Self::Local } else { Self::None }
```

```diff
diff --git a/src/labels.rs b/src/labels.rs
--- a/src/labels.rs
+++ b/src/labels.rs
@@ -361,7 +361,7 @@ pub struct Place {
 pub fn database_key(driver: Driver, database: &str) -> String {
     match driver {
         Driver::Postgres => database.to_owned(),
-        Driver::MySql | Driver::Sqlite => String::new(),
+        Driver::MySql | Driver::Sqlite | Driver::Files => String::new(),
     }
 }
 
```

```diff
diff --git a/src/managed.rs b/src/managed.rs
--- a/src/managed.rs
+++ b/src/managed.rs
@@ -102,7 +102,7 @@ impl Match {
     pub fn matches(&self, spec: &ConnectSpec, home: Option<&Path>) -> bool {
         let server = match spec.driver {
             Driver::Postgres | Driver::MySql => true,
-            Driver::Sqlite => false,
+            Driver::Sqlite | Driver::Files => false,
         };
         if let Some(driver) = &self.driver
             && !glob(driver, driver_key(spec.driver))
@@ -129,11 +129,16 @@ impl Match {
             return false;
         }
         if let Some(path) = &self.path {
-            let Some(file) = spec
-                .sqlite_path
-                .as_deref()
-                .and_then(|file| absolute(file, home))
-            else {
+            // A Files connection's path is its folder's.
+            let named = match spec.driver {
+                Driver::Sqlite => spec.sqlite_path.as_deref(),
+                Driver::Files => spec
+                    .files
+                    .as_ref()
+                    .and_then(tabletist_db::files::Source::folder),
+                Driver::Postgres | Driver::MySql => None,
+            };
+            let Some(file) = named.and_then(|file| absolute(file, home)) else {
                 return false;
             };
             if !glob(path, &file.to_string_lossy()) {
@@ -154,6 +159,7 @@ fn driver_key(driver: Driver) -> &'static str {
         Driver::Postgres => "postgres",
         Driver::MySql => "mysql",
         Driver::Sqlite => "sqlite",
+        Driver::Files => "files",
     }
 }
 
```

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -2185,7 +2185,7 @@ impl ConnectionForm {
     /// `require` with a CA file does, like libpq).
     pub fn password_can_be_intercepted(&self) -> bool {
         let unchecked = match self.driver {
-            Driver::Sqlite => false,
+            Driver::Sqlite | Driver::Files => false,
             Driver::Postgres => match self.tls {
                 TlsMode::Disable | TlsMode::Prefer => true,
                 TlsMode::Require => self.ca_file.trim().is_empty(),
@@ -2333,6 +2333,7 @@ impl ConnectionForm {
                 }
                 Ok(ConnectSpec::sqlite(path))
             }
+            Driver::Files => Err("Choose a folder.".into()),
             Driver::Postgres | Driver::MySql => {
                 let host = self.host.trim();
                 if host.is_empty() {
@@ -2392,6 +2393,7 @@ impl ConnectionForm {
                     tls: self.tls,
                     ca_file: (!ca_file.is_empty()).then(|| ca_file.into()),
                     ssh,
+                    files: None,
                 })
             }
         }
@@ -2474,7 +2476,7 @@ impl ConnectionForm {
         }
         let spec = self.to_spec()?;
         let password = match spec.driver {
-            Driver::Sqlite => PasswordMode::None,
+            Driver::Sqlite | Driver::Files => PasswordMode::None,
             // Nothing typed and nothing saved (or the saved one forgotten):
             // a server that needs no password (trust or peer
             // authentication).
@@ -3536,7 +3538,8 @@ pub fn is_system_schema(driver: Driver, name: &str) -> bool {
             name,
             "mysql" | "sys" | "performance_schema" | "information_schema"
         ),
-        Driver::Sqlite => false,
+        // A `.duckdb` file's schema is the person's.
+        Driver::Sqlite | Driver::Files => false,
     }
 }
 
@@ -5860,7 +5863,7 @@ impl Workspace {
         let name = match self.driver {
             Driver::Postgres => "public",
             Driver::MySql => self.spec.database.as_str(),
-            Driver::Sqlite => "main",
+            Driver::Sqlite | Driver::Files => "main",
         };
         let schemas = self.tree.schemas.value.as_ref()?;
         let found = schemas.iter().find(|schema| *schema == name);
```

```diff
diff --git a/src/paths.rs b/src/paths.rs
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -219,6 +219,12 @@ impl AppDirs {
         self.state.join("agents_shown.json")
     }
 
+    /// Where DuckDB keeps the readers it installs for Files connections
+    /// (Excel's): the app's, not DuckDB's own `~/.duckdb`.
+    pub fn duckdb_extensions_dir(&self) -> PathBuf {
+        self.state.join("duckdb-extensions")
+    }
+
     pub fn log_file(&self) -> PathBuf {
         self.state.join("tabletist.log")
     }
```

```diff
diff --git a/src/session.rs b/src/session.rs
--- a/src/session.rs
+++ b/src/session.rs
@@ -147,6 +147,8 @@ pub fn database_of(spec: &tabletist_db::ConnectSpec) -> String {
             .as_ref()
             .map(|path| path.display().to_string())
             .unwrap_or_default(),
+        // Its folder, or its files: what it opens is what names it.
+        tabletist_db::Driver::Files => spec.summary(),
         tabletist_db::Driver::Postgres | tabletist_db::Driver::MySql => spec.database.clone(),
     }
 }
```

```diff
diff --git a/src/ui/calendar.rs b/src/ui/calendar.rs
--- a/src/ui/calendar.rs
+++ b/src/ui/calendar.rs
@@ -204,7 +204,8 @@ pub fn model(
         Kind::Time { offset: false } | Kind::Date => None,
     };
     let postgres = match column.dialect {
-        tabletist_db::Dialect::Postgres => true,
+        // DuckDB keeps PostgreSQL's infinities.
+        tabletist_db::Dialect::Postgres | tabletist_db::Dialect::DuckDb => true,
         tabletist_db::Dialect::MySql | tabletist_db::Dialect::Sqlite => false,
     };
     Model {
```

```diff
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -959,6 +959,7 @@ fn url_field(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) -> Response {
         Driver::Sqlite => "sqlite:///path/to/file.db",
         Driver::MySql => "mysql://user@host/db",
         Driver::Postgres => "postgres://user@host/db",
+        Driver::Files => "~/Data/exports",
     };
     let hint = hint(ui, example, role, skin);
     let field = ui.add(input(ui, &mut form.url, role, skin.field_height(), skin).hint_text(hint));
```

```diff
diff --git a/src/ui/connect_dialog/sheet.rs b/src/ui/connect_dialog/sheet.rs
--- a/src/ui/connect_dialog/sheet.rs
+++ b/src/ui/connect_dialog/sheet.rs
@@ -267,6 +267,7 @@ pub(super) fn body(
                             file_field(ui, form, skin, actions);
                         });
                     }
+                    Driver::Files => {}
                     Driver::Postgres | Driver::MySql => {
                         fieldset(ui, &skin.say("Server"), skin, |ui| {
                             server(ui, form, skin, actions);
```

```diff
diff --git a/src/ui/connect_dialog/terminal.rs b/src/ui/connect_dialog/terminal.rs
--- a/src/ui/connect_dialog/terminal.rs
+++ b/src/ui/connect_dialog/terminal.rs
@@ -302,6 +302,7 @@ pub(super) fn terminal_body(
                     terminal_section(ui, &skin.say("Database"), skin);
                     file_field(ui, form, skin, actions);
                 }
+                Driver::Files => {}
                 Driver::Postgres | Driver::MySql => {
                     terminal_section(ui, &skin.say("Server"), skin);
                     terminal_server(ui, form, skin, actions);
```

```diff
diff --git a/src/ui/date_line.rs b/src/ui/date_line.rs
--- a/src/ui/date_line.rs
+++ b/src/ui/date_line.rs
@@ -268,7 +268,9 @@ fn reading_words(reading: &Reading, column: &Column, locale: Locale) -> String {
     }
     let sqlite = match column.dialect {
         tabletist_db::Dialect::Sqlite => column.kind.has_date(),
-        tabletist_db::Dialect::Postgres | tabletist_db::Dialect::MySql => false,
+        tabletist_db::Dialect::Postgres
+        | tabletist_db::Dialect::MySql
+        | tabletist_db::Dialect::DuckDb => false,
     };
     if sqlite {
         parts.push(match column.stored {
```

```diff
diff --git a/src/ui/ddl.rs b/src/ui/ddl.rs
--- a/src/ui/ddl.rs
+++ b/src/ui/ddl.rs
@@ -230,6 +230,7 @@ fn engine(driver: Driver) -> &'static str {
         Driver::Postgres => "PostgreSQL",
         Driver::MySql => "MySQL",
         Driver::Sqlite => "SQLite",
+        Driver::Files => "DuckDB",
     }
 }
 
@@ -241,7 +242,7 @@ fn left_out(driver: Driver, locale: Locale) -> Option<Cow<'static, str>> {
             locale,
             "Not included: owner, grants, sequences owned by the table, row-level policies",
         )),
-        Driver::MySql | Driver::Sqlite => None,
+        Driver::MySql | Driver::Sqlite | Driver::Files => None,
     }
 }
 
@@ -899,7 +900,7 @@ fn left_out_lines(driver: Driver, locale: Locale) -> Option<[String; 2]> {
             gettext(locale, "owner · grants · owned sequences").into_owned(),
             gettext(locale, "row-level policies").into_owned(),
         ]),
-        Driver::MySql | Driver::Sqlite => None,
+        Driver::MySql | Driver::Sqlite | Driver::Files => None,
     }
 }
 
@@ -1003,7 +1004,8 @@ impl Failed<'_> {
         match self.driver {
             Driver::MySql => matches!(named.number, Some(1044 | 1142 | 1143 | 1227)),
             Driver::Postgres => code.as_deref() == Some("42501"),
-            Driver::Sqlite => false,
+            // A file the session may not read never opens.
+            Driver::Sqlite | Driver::Files => false,
         }
     }
 
@@ -1021,7 +1023,7 @@ impl Failed<'_> {
             (Driver::MySql, ObjectKind::Table | ObjectKind::MaterializedView) => {
                 "The connection's user can't run SHOW CREATE TABLE here. Rows and Structure still work."
             }
-            (Driver::Postgres | Driver::Sqlite, _) => {
+            (Driver::Postgres | Driver::Sqlite | Driver::Files, _) => {
                 "The connection's user can't read this part of the catalog. Rows and Structure still work."
             }
         };
```

```diff
diff --git a/src/ui/ddl_tests.rs b/src/ui/ddl_tests.rs
--- a/src/ui/ddl_tests.rs
+++ b/src/ui/ddl_tests.rs
@@ -76,6 +76,16 @@ pub(super) fn book_covers(driver: Driver) -> Ddl {
                 ),
             ],
         },
+        // The Bookshop's exports keep `book_covers` in a `.duckdb` file.
+        Driver::Files => Ddl {
+            driver,
+            server: "DuckDB 2.0.0".into(),
+            heading: None,
+            parts: vec![part(
+                PartKind::Object,
+                &["CREATE TABLE book_covers(id BIGINT, book_id BIGINT, kind VARCHAR)"],
+            )],
+        },
     }
 }
 
```

```diff
diff --git a/src/ui/format.rs b/src/ui/format.rs
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -959,11 +959,11 @@ pub fn capped(text: &str) -> Cow<'_, str> {
 pub fn refuses_writes(error: &tabletist_db::Error, driver: tabletist_db::Driver) -> bool {
     use tabletist_db::{Driver, Error};
     match error {
-        Error::Query {
-            code: Some(code), ..
-        } => match driver {
-            Driver::Sqlite => code == "8",
-            Driver::Postgres | Driver::MySql => code == "25006",
+        Error::Query { code, message, .. } => match driver {
+            Driver::Sqlite => code.as_deref() == Some("8"),
+            Driver::Postgres | Driver::MySql => code.as_deref() == Some("25006"),
+            // DuckDB gives no code: its own words, which no setting changes.
+            Driver::Files => message.contains("transaction is launched in read-only mode"),
         },
         _ => false,
     }
```

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -8420,6 +8420,7 @@ value_tags   = true
             foreign_keys: Vec::new(),
             referenced_by: Vec::new(),
             comment: None,
+            file: None,
         };
         harness.answer_structure(structure);
         assert!(harness.has("Columns"));
@@ -12624,6 +12625,7 @@ value_tags   = true
             foreign_keys: Vec::new(),
             referenced_by: Vec::new(),
             comment: None,
+            file: None,
         });
         harness.answer_rows(RowPage {
             columns: names
```

```diff
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -105,7 +105,7 @@ fn is_local(connection: &SavedConnection) -> bool {
     use tabletist_db::Driver;
     let spec = &connection.spec;
     match spec.driver {
-        Driver::Sqlite => true,
+        Driver::Sqlite | Driver::Files => true,
         Driver::Postgres | Driver::MySql => {
             spec.ssh.is_none() && crate::model::is_local_host(&spec.host)
         }
@@ -680,6 +680,7 @@ pub(crate) fn target(connection: &SavedConnection) -> (String, String) {
     let spec = &connection.spec;
     match spec.driver {
         Driver::Sqlite => (spec.summary(), String::new()),
+        Driver::Files => (spec.summary(), String::new()),
         Driver::Postgres | Driver::MySql => (
             format!("{}:{}", spec.host, spec.port),
             format!("/{}", spec.database),
```

```diff
diff --git a/src/ui/start.rs b/src/ui/start.rs
--- a/src/ui/start.rs
+++ b/src/ui/start.rs
@@ -84,6 +84,19 @@ fn shown(app: &App, ctx: &egui::Context, tab: ConnTabId) -> Option<Shown> {
             .and_then(|path| path.file_name())
             .map(|name| name.to_string_lossy().into_owned())
             .unwrap_or_default(),
+        // A Files connection is on a folder, or on its first file.
+        Driver::Files => workspace
+            .spec
+            .files
+            .as_ref()
+            .map(|source| match source {
+                tabletist_db::files::Source::Folder(folder) => folder
+                    .file_name()
+                    .map(|name| name.to_string_lossy().into_owned())
+                    .unwrap_or_else(|| tabletist_db::files::shown(folder)),
+                tabletist_db::files::Source::Files(_) => source.summary(),
+            })
+            .unwrap_or_default(),
         Driver::Postgres | Driver::MySql => info.database.clone(),
     };
     let objects = app
@@ -587,7 +600,7 @@ fn lists(
     } else {
         match shown.driver {
             Driver::Postgres | Driver::MySql => gettext(locale, "Largest tables").into_owned(),
-            Driver::Sqlite => gettext(locale, "Tables").into_owned(),
+            Driver::Sqlite | Driver::Files => gettext(locale, "Tables").into_owned(),
         }
     };
     let note = if first_visit {
@@ -595,7 +608,7 @@ fn lists(
             Driver::Postgres | Driver::MySql => {
                 skin.say("From table statistics. Nothing is queried.")
             }
-            Driver::Sqlite => skin.say("Nothing is queried."),
+            Driver::Sqlite | Driver::Files => skin.say("Nothing is queried."),
         })
     } else {
         None
```

```diff
diff --git a/src/ui/structure.rs b/src/ui/structure.rs
--- a/src/ui/structure.rs
+++ b/src/ui/structure.rs
@@ -140,7 +140,8 @@ const COMMENT_WIDTH: f32 = 320.0;
 fn keeps_comments(driver: Driver) -> bool {
     match driver {
         Driver::Postgres | Driver::MySql => true,
-        Driver::Sqlite => false,
+        // A file keeps none.
+        Driver::Sqlite | Driver::Files => false,
     }
 }
 
```

```diff
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -197,7 +197,7 @@ fn connecting(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let say = |text: &'static str| look.label(&gettext(locale, text));
     let spec = &workspace.spec;
     let (reach, target) = match spec.driver {
-        Driver::Sqlite => ("Open", spec.summary()),
+        Driver::Sqlite | Driver::Files => ("Open", spec.summary()),
         Driver::Postgres | Driver::MySql => {
             let target = format!("{}:{}", spec.host, spec.port);
             let target = match &spec.ssh {
@@ -296,7 +296,7 @@ fn failure_title(
             format!("{} {}", say("Password rejected for"), spec.user),
         ),
         Error::Connect(_) | Error::Timeout => match spec.driver {
-            Driver::Sqlite => (
+            Driver::Sqlite | Driver::Files => (
                 Icon::CircleAlert,
                 format!("{} {}", say("Can't open"), spec.summary()),
             ),
@@ -500,7 +500,7 @@ struct Chip {
 fn is_remote(workspace: &crate::model::Workspace) -> bool {
     use tabletist_db::Driver;
     match workspace.driver {
-        Driver::Sqlite => false,
+        Driver::Sqlite | Driver::Files => false,
         Driver::Postgres | Driver::MySql => !crate::model::is_local_host(&workspace.spec.host),
     }
 }
@@ -521,7 +521,7 @@ pub(super) fn bar_info(app: &App, tab: ConnTabId) -> Option<BarInfo> {
         env: workspace.environment,
         read_only: workspace.access == tabletist_db::Access::ReadOnly,
         database: match workspace.driver {
-            Driver::Sqlite => String::new(),
+            Driver::Sqlite | Driver::Files => String::new(),
             Driver::Postgres | Driver::MySql => spec.database.clone(),
         },
         databases: workspace.databases.value.clone().unwrap_or_default(),
@@ -589,6 +589,15 @@ fn card_rows(
                 .unwrap_or_default();
             rows.push((say("File"), path));
         }
+        Driver::Files => match &spec.files {
+            Some(tabletist_db::files::Source::Folder(folder)) => {
+                rows.push((say("Folder"), tabletist_db::files::shown(folder)));
+            }
+            Some(source @ tabletist_db::files::Source::Files(_)) => {
+                rows.push((say("Files"), source.summary()));
+            }
+            None => {}
+        },
         Driver::Postgres | Driver::MySql => {
             rows.push((say("Host"), format!("{}:{}", spec.host, spec.port)));
             if !spec.database.is_empty() {
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test files`

Expected: `tests/files.rs`: 18 passed

- [ ] **Step 5: Run the checks** (above). Expected: every suite passes, the app's library (`Running unittests src/lib.rs (…tabletist-…)`) with 3695 tests.

- [ ] **Step 6: Commit**

```bash
git add -A crates Cargo.lock src docs/_reference/settings-and-files.md
git commit -m "Open a folder of data files through DuckDB, read-only"
```

---

### Task 2: The Files form

The dialog offers Files: the Type row on a line of its own, the Source group (Folder or Files, the pickers, the file list, what was found), the fixed read-only box, the Agents part without hiding and masking, the URL tab that is off, Test's "Read 3 files · 2.4 GB", the question before a used connection changes type, a pasted path, and the Omarchy keys.

**Files:** 
- Create: `assets/icons/file.svg`
- Create: `assets/icons/folder.svg`
- Modify: `docs/_guide/agents.md`
- Modify: `docs/_guide/omarchy.md`
- Modify: `docs/_reference/connections.md`
- Modify: `src/app.rs`
- Modify: `src/backend.rs`
- Modify: `src/keymap.rs`
- Modify: `src/model.rs`
- Modify: `src/theme.rs`
- Modify: `src/ui/connect_dialog/agents.rs`
- Modify: `src/ui/connect_dialog/choice.rs`
- Create: `src/ui/connect_dialog/files.rs`
- Modify: `src/ui/connect_dialog/mod.rs`
- Modify: `src/ui/connect_dialog/sheet.rs`
- Modify: `src/ui/connect_dialog/terminal.rs`
- Create: `src/ui/file_list.rs`
- Create: `src/ui/files_tests.rs`
- Modify: `src/ui/mod.rs`


- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/files_tests.rs b/src/ui/files_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/files_tests.rs
@@ -0,0 +1,682 @@
+//! The Files connection type in the connection dialog and the connections
+//! list, in both looks: the Type row and its kept fields, the Source (a
+//! folder or a list) and what was found there, Test, the fixed read-only
+//! box, agents, the URL tab that is off, the Omarchy keys, the question
+//! before a used connection changes type, and the list's rows.
+
+use std::path::PathBuf;
+
+use egui::accesskit::Role;
+use egui::{Key, Modifiers};
+use tabletist_db::files::{Format, Found, Scan, Source};
+use tabletist_db::{ConnectSpec, Driver, FilesRead};
+
+use crate::backend::{Command, Event};
+use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
+use crate::env::Environment;
+use crate::model::{Action, ConnectionForm, Dialog, FilesFound, FilesKind, PickTarget, TestState};
+use crate::testing::{Harness, node};
+use crate::theme::Look;
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
+
+/// Whether a node says `text`, alone or as part of what it says: the
+/// terminal look runs a part's sentences together.
+fn says(harness: &mut Harness, text: &str) -> bool {
+    let tree = harness.settle();
+    crate::testing::labels(&tree)
+        .iter()
+        .any(|label| label.contains(text))
+}
+
+fn form(harness: &Harness) -> &ConnectionForm {
+    match &harness.app.dialog {
+        Some(Dialog::Connection(form)) => form,
+        _ => panic!("no connection dialog"),
+    }
+}
+
+fn form_mut(harness: &mut Harness) -> &mut ConnectionForm {
+    match &mut harness.app.dialog {
+        Some(Dialog::Connection(form)) => form,
+        _ => panic!("no connection dialog"),
+    }
+}
+
+/// A new connection's dialog in `look`, its type Files.
+fn new_files(look: Look) -> Harness {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    harness.app.apply(Action::NewConnection);
+    harness.settle();
+    harness.click(&look.label("Files"));
+    harness
+}
+
+/// The Bookshop's exports as a scan of `folder` finds them.
+fn exports(folder: &str) -> Scan {
+    let found = |name: &str, format: Format, size: u64| Found {
+        path: PathBuf::from(folder).join(name),
+        format,
+        size,
+        name: name.split('.').next().unwrap_or_default().to_owned(),
+    };
+    Scan {
+        files: vec![
+            found("orders.csv", Format::Csv, 1_600_000_000),
+            found("customers.parquet", Format::Parquet, 400_000_000),
+            found("events.ndjson", Format::Ndjson, 450_000_000),
+            found("returns.csv", Format::Csv, 50_000_000),
+        ],
+        missing: Vec::new(),
+    }
+}
+
+/// Answers the dialog's last read of its Source with `result`.
+fn source_read(harness: &mut Harness, result: Result<Scan, String>) {
+    let FilesFound::Reading(request) = form(harness).files_found else {
+        panic!("no read of the source: {:?}", form(harness).files_found);
+    };
+    harness
+        .app
+        .apply(Action::Backend(Event::SourceRead { request, result }));
+    harness.settle();
+}
+
+/// Picks `folder` as the dialog's folder, as the file dialog answers.
+fn pick_folder(harness: &mut Harness, folder: &str) {
+    harness.app.apply(Action::PickFolder);
+    let request = form(harness).pick_request.expect("a pick in flight");
+    assert_eq!(form(harness).pick_target, PickTarget::Folder);
+    harness.app.apply(Action::Backend(Event::FilePicked {
+        request,
+        path: Some(folder.into()),
+    }));
+    harness.settle();
+}
+
+#[test]
+fn a_new_files_connection_is_local_read_only_and_lets_agents_read() {
+    for look in looks() {
+        let harness = new_files(look);
+        let form = form(&harness);
+        assert_eq!(form.driver, Driver::Files, "{}", look.name);
+        assert_eq!(form.environment(), Environment::Local);
+        assert_eq!(form.agents.access, AgentAccess::Read);
+    }
+    // Every other type still starts with agents Off, and a type chosen
+    // again moves them back while the person has not chosen.
+    let mut harness = new_files(Look::macos());
+    harness.click("PostgreSQL");
+    assert_eq!(form(&harness).agents.access, AgentAccess::Off);
+    harness.click("Files");
+    assert_eq!(form(&harness).agents.access, AgentAccess::Read);
+    // Production lets agents do nothing until the person says so, as
+    // its environment is clicked; Local lets them read again.
+    harness.click("Production");
+    assert_eq!(form(&harness).agents.access, AgentAccess::Off);
+    harness.click("Local");
+    assert_eq!(form(&harness).agents.access, AgentAccess::Read);
+    // A choice of the person's own stays.
+    harness.click("Off");
+    harness.click("Dev");
+    assert_eq!(form(&harness).agents.access, AgentAccess::Off);
+}
+
+#[test]
+fn the_type_is_a_radio_group_of_four_on_a_row_of_its_own() {
+    for look in looks() {
+        let mut harness = new_files(look);
+        let tree = harness.settle();
+        assert!(node(&tree, &look.label("Type"), Role::RadioGroup).is_some());
+        for name in ["SQLite", "PostgreSQL", "MySQL", "Files"] {
+            assert!(
+                node(&tree, &look.label(name), Role::RadioButton).is_some(),
+                "{}: {name}",
+                look.name
+            );
+        }
+        assert!(
+            node(&tree, &look.label("Source"), Role::RadioGroup).is_some(),
+            "{}: the Source choice",
+            look.name
+        );
+        // Type comes before Name: it changes what follows.
+        let top = |label: &str| {
+            harness
+                .painted_rect(&look.label(label))
+                .unwrap_or_else(|| panic!("{label} is not painted"))
+                .top()
+        };
+        assert!(top("Type") < top("Name"), "{}", look.name);
+    }
+    let mut harness = new_files(Look::macos());
+    assert!(harness.has("Opens CSV, Parquet, JSON, Excel and .duckdb files as tables"));
+    harness.click("SQLite");
+    assert!(harness.has("One database file on this computer"));
+    harness.click("PostgreSQL");
+    assert!(harness.has("Changes the fields below"));
+}
+
+#[test]
+fn changing_the_type_keeps_the_name_and_every_field_typed() {
+    let mut harness = new_files(Look::macos());
+    form_mut(&mut harness).name = "Bookshop".into();
+    form_mut(&mut harness).environment = Some(Environment::Dev);
+    harness.click("PostgreSQL");
+    form_mut(&mut harness).host = "db.bookshop.example".into();
+    harness.click("Files");
+    pick_folder(&mut harness, "/data/bookshop-exports");
+    harness.click("PostgreSQL");
+    assert_eq!(form(&harness).host, "db.bookshop.example");
+    assert_eq!(form(&harness).name, "Bookshop");
+    assert_eq!(form(&harness).environment, Some(Environment::Dev));
+    harness.click("Files");
+    assert_eq!(form(&harness).files_folder, "/data/bookshop-exports");
+}
+
+#[test]
+fn a_folder_is_read_and_its_files_found() {
+    for look in looks() {
+        let mut harness = new_files(look);
+        pick_folder(&mut harness, "/data/bookshop-exports");
+        // The folder names the connection, and the backend reads it.
+        assert_eq!(form(&harness).name, "bookshop-exports");
+        assert_eq!(
+            harness.app.backend.sources_read.last(),
+            Some(&Source::Folder("/data/bookshop-exports".into()))
+        );
+        source_read(&mut harness, Ok(exports("/data/bookshop-exports")));
+        match look.terminal {
+            true => assert!(harness.has(
+                "4 files · 2.5 gb · csv, parquet, ndjson → orders, customers, events, returns"
+            )),
+            false => {
+                assert!(harness.has("4 files · 2.5 GB · CSV, Parquet, NDJSON"));
+                for name in ["orders", "customers", "events", "returns"] {
+                    assert!(harness.has(name), "{name}");
+                }
+                assert!(harness.has(
+                    "Every CSV, Parquet, JSON, NDJSON, Excel and .duckdb file in the folder \
+                     becomes a table. Files added later show up on the next open."
+                ));
+            }
+        }
+        // Saved, it opens that folder.
+        let saved = form(&harness).to_saved().unwrap();
+        assert_eq!(
+            saved.spec.files,
+            Some(Source::Folder("/data/bookshop-exports".into()))
+        );
+        assert_eq!(saved.password, PasswordMode::None);
+        // Saved read-only, as its box shows, whatever its environment.
+        assert_eq!(saved.read_only, Some(true));
+    }
+}
+
+#[test]
+fn a_folder_that_is_gone_says_so_and_the_form_stays_editable() {
+    for look in looks() {
+        let mut harness = new_files(look);
+        pick_folder(&mut harness, "/data/bookshop-exports");
+        source_read(
+            &mut harness,
+            Err("Can't open /data/bookshop-exports: no such folder".into()),
+        );
+        assert!(harness.has(
+            &look.label("This folder isn't there any more. Choose it again, or pick another.")
+        ));
+        // Another folder is read again.
+        pick_folder(&mut harness, "/data/other");
+        assert!(matches!(form(&harness).files_found, FilesFound::Reading(_)));
+    }
+}
+
+#[test]
+fn files_are_added_and_taken_off_the_list_and_stay_on_the_disk() {
+    for look in looks() {
+        let mut harness = new_files(look);
+        harness.app.apply(Action::SetFilesKind(FilesKind::Files));
+        harness.app.apply(Action::PickDataFiles);
+        let request = form(&harness).pick_request.unwrap();
+        assert_eq!(form(&harness).pick_target, PickTarget::Files);
+        harness.app.apply(Action::Backend(Event::FilesPicked {
+            request,
+            paths: vec![
+                "/data/exports/orders.csv".into(),
+                "/data/exports/customers.parquet".into(),
+                "/data/exports/orders.csv".into(),
+            ],
+        }));
+        harness.settle();
+        assert_eq!(
+            form(&harness).files_list,
+            [
+                "/data/exports/orders.csv",
+                "/data/exports/customers.parquet"
+            ]
+        );
+        let tree = harness.settle();
+        assert!(
+            node(&tree, &look.label("Files"), Role::List).is_some(),
+            "{}",
+            look.name
+        );
+        assert!(node(&tree, "/data/exports/orders.csv", Role::ListItem).is_some());
+        harness.click("Remove orders.csv");
+        assert_eq!(
+            form(&harness).files_list,
+            ["/data/exports/customers.parquet"]
+        );
+        assert_eq!(
+            harness.app.backend.sources_read.last(),
+            Some(&Source::Files(vec![
+                "/data/exports/customers.parquet".into()
+            ]))
+        );
+        // Nothing but the read of the list went to the backend: no file
+        // is touched.
+        assert!(
+            !harness
+                .app
+                .backend
+                .sent
+                .iter()
+                .any(|sent| matches!(sent, Command::Save { .. })),
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn a_test_reads_every_file_and_says_how_much() {
+    for look in looks() {
+        let mut harness = new_files(look);
+        pick_folder(&mut harness, "/data/bookshop-exports");
+        source_read(&mut harness, Ok(exports("/data/bookshop-exports")));
+        harness.app.apply(Action::TestConnection);
+        let TestState::Running(request) = form(&harness).test else {
+            panic!("no test: {:?}", form(&harness).test);
+        };
+        assert!(matches!(
+            harness.app.backend.sent.last(),
+            Some(Command::Test { spec, .. }) if spec.files.is_some()
+        ));
+        harness.app.apply(Action::Backend(Event::FilesTested {
+            request,
+            result: Ok(FilesRead {
+                files: 3,
+                bytes: 2_400_000_000,
+            }),
+        }));
+        let said = look.label("Read 3 files · 2.4 GB");
+        let tree = harness.settle();
+        assert!(
+            crate::testing::labels(&tree)
+                .iter()
+                .any(|label| label.starts_with(&said)),
+            "{}: {:?}",
+            look.name,
+            crate::testing::labels(&tree)
+        );
+        // A folder gone by the time of the Test names it.
+        harness.app.apply(Action::TestConnection);
+        let TestState::Running(request) = form(&harness).test else {
+            panic!("no test");
+        };
+        harness.app.apply(Action::Backend(Event::FilesTested {
+            request,
+            result: Err(tabletist_db::Error::Connect(
+                "Can't open ~/Data/bookshop-exports: no such folder".into(),
+            )),
+        }));
+        assert_eq!(
+            form(&harness).test,
+            TestState::Failed("Can't open ~/Data/bookshop-exports: no such folder".into())
+        );
+    }
+}
+
+#[test]
+fn files_open_read_only_and_the_box_cannot_be_turned_off() {
+    for look in looks() {
+        let mut harness = new_files(look);
+        let name = "Open read-only";
+        let tree = harness.settle();
+        let check = node(&tree, name, Role::CheckBox).expect("the read-only box");
+        let found = &tree.nodes.iter().find(|(id, _)| *id == check).unwrap().1;
+        assert!(found.is_disabled(), "{}", look.name);
+        harness.click(name);
+        assert_eq!(form(&harness).read_only, None, "{}", look.name);
+        pick_folder(&mut harness, "/data/bookshop-exports");
+        assert_eq!(form(&harness).to_saved().unwrap().read_only, Some(true));
+        // The terminal look paints its promise beside the box.
+        match look.terminal {
+            true => assert!(harness.painted_rect("[x] files open read-only").is_some()),
+            false => assert!(harness.has("Files open read-only.")),
+        }
+    }
+}
+
+#[test]
+fn agents_have_no_tables_or_masked_columns_on_files() {
+    for look in looks() {
+        let mut harness = new_files(look);
+        if !crate::endpoint::AVAILABLE {
+            return;
+        }
+        assert!(
+            says(
+                &mut harness,
+                &look.label("Agents can query the files, never change them.")
+            ),
+            "{}",
+            look.name
+        );
+        for gone in ["Tables", "Masked columns", "Masked", "All except"] {
+            assert!(!harness.has(&look.label(gone)), "{}: {gone}", look.name);
+        }
+    }
+}
+
+#[test]
+fn the_url_tab_is_off_for_files_and_says_why() {
+    let why = "Files connections have no URL. Choose a folder or files under Source.";
+    let mut harness = new_files(Look::macos());
+    let tree = harness.settle();
+    let tab = node(&tree, "URL", Role::RadioButton).expect("the URL tab");
+    let url = &tree.nodes.iter().find(|(id, _)| *id == tab).unwrap().1;
+    assert!(url.is_disabled());
+    assert_eq!(url.description(), Some(why));
+    harness.click("URL");
+    assert!(!form(&harness).url_mode);
+    // The terminal look says so where its key would be, and `u` shows
+    // nothing.
+    let mut harness = new_files(Look::omarchy());
+    assert!(harness.has("no url for files"));
+    harness.press(Key::U, Modifiers::NONE);
+    assert!(!form(&harness).url_mode);
+}
+
+#[test]
+fn a_path_typed_as_the_url_opens_it_as_files() {
+    let mut harness = Harness::new();
+    harness.app.apply(Action::NewConnection);
+    harness.settle();
+    form_mut(&mut harness).url = "/data/bookshop-exports".into();
+    harness.app.apply(Action::ApplyUrl);
+    assert_eq!(form(&harness).driver, Driver::Files);
+    assert_eq!(form(&harness).files_kind, FilesKind::Folder);
+    assert_eq!(form(&harness).files_folder, "/data/bookshop-exports");
+    assert_eq!(form(&harness).name, "bookshop-exports");
+    assert_eq!(
+        harness.app.backend.sources_read.last(),
+        Some(&Source::Folder("/data/bookshop-exports".into()))
+    );
+    // A data file's path is a list of that file.
+    form_mut(&mut harness).url = "/data/bookshop-exports/orders.csv".into();
+    harness.app.apply(Action::ApplyUrl);
+    assert_eq!(form(&harness).files_kind, FilesKind::Files);
+    assert_eq!(
+        form(&harness).files_list,
+        ["/data/bookshop-exports/orders.csv"]
+    );
+    // A SQLite database's path stays SQLite.
+    form_mut(&mut harness).url = "/data/shop.db".into();
+    harness.app.apply(Action::ApplyUrl);
+    assert_eq!(form(&harness).driver, Driver::Sqlite);
+    assert_eq!(form(&harness).sqlite_path, "/data/shop.db");
+}
+
+/// A saved PostgreSQL connection like the one `used_bookshop` makes.
+fn harness_saved(id: &ConnectionId) -> SavedConnection {
+    let (spec, _) =
+        ConnectSpec::from_url("postgres://clerk@localhost/bookshop_development").unwrap();
+    SavedConnection {
+        id: id.clone(),
+        name: "Bookshop".into(),
+        environment: Environment::Dev,
+        read_only: None,
+        password: PasswordMode::None,
+        ssh_secret: PasswordMode::None,
+        spec,
+        agents: Default::default(),
+    }
+}
+
+/// A saved PostgreSQL connection named Bookshop, used once.
+fn used_bookshop(harness: &mut Harness) -> ConnectionId {
+    let (spec, _) =
+        ConnectSpec::from_url("postgres://clerk@localhost/bookshop_development").unwrap();
+    let saved = SavedConnection {
+        id: ConnectionId::new(),
+        name: "Bookshop".into(),
+        environment: Environment::Dev,
+        read_only: None,
+        password: PasswordMode::None,
+        ssh_secret: PasswordMode::None,
+        spec,
+        agents: Default::default(),
+    };
+    let id = saved.id.clone();
+    harness.app.connections.upsert(saved);
+    harness.app.connections.mark_used(&id, 1_790_000_000);
+    id
+}
+
+#[test]
+fn a_used_connection_saved_as_another_type_is_asked_about_first() {
+    let mut harness = Harness::new();
+    let id = used_bookshop(&mut harness);
+    harness.app.apply(Action::EditConnection(id.clone()));
+    harness.settle();
+    harness.click("Files");
+    pick_folder(&mut harness, "/data/bookshop-exports");
+    harness.app.apply(Action::SaveConnection { connect: false });
+    assert_eq!(
+        form(&harness).message.as_deref(),
+        Some(
+            "Change Bookshop from PostgreSQL to Files? Its server settings will be removed. \
+             Save again to change it."
+        )
+    );
+    assert_eq!(
+        harness.app.connections.get(&id).unwrap().spec.driver,
+        Driver::Postgres
+    );
+    harness.app.apply(Action::SaveConnection { connect: false });
+    assert!(harness.app.dialog.is_none());
+    let saved = harness.app.connections.get(&id).unwrap();
+    assert_eq!(saved.spec.driver, Driver::Files);
+    assert!(saved.spec.host.is_empty());
+    // One never used changes without a question.
+    let mut harness = Harness::new();
+    let id = used_bookshop(&mut harness);
+    harness.app.connections = crate::connections::SavedConnections::default();
+    let mut never = SavedConnection {
+        id: ConnectionId::new(),
+        ..harness_saved(&id)
+    };
+    never.name = "Bookshop".into();
+    let id = never.id.clone();
+    harness.app.connections.upsert(never);
+    harness.app.apply(Action::EditConnection(id.clone()));
+    harness.settle();
+    harness.click("Files");
+    pick_folder(&mut harness, "/data/bookshop-exports");
+    harness.app.apply(Action::SaveConnection { connect: false });
+    assert!(harness.app.dialog.is_none());
+}
+
+#[test]
+fn the_terminal_keys_pick_add_remove_and_move() {
+    // A saved one opens with the keyboard in no field: its letters are
+    // keys.
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    let id = saved_exports(&mut harness, Source::Folder("/d".into()));
+    harness.app.apply(Action::EditConnection(id));
+    harness.settle();
+    harness.press(Key::O, Modifiers::CTRL);
+    assert_eq!(form(&harness).pick_target, PickTarget::Folder);
+    assert!(form(&harness).pick_request.is_some());
+    harness.app.apply(Action::SetFilesKind(FilesKind::Files));
+    harness.settle();
+    harness.press(Key::O, Modifiers::CTRL);
+    assert_eq!(form(&harness).pick_target, PickTarget::Files);
+    form_mut(&mut harness).pick_request = None;
+    harness.press(Key::A, Modifiers::NONE);
+    let request = form(&harness).pick_request.expect("a adds files");
+    harness.app.apply(Action::Backend(Event::FilesPicked {
+        request,
+        paths: vec![
+            "/d/orders.csv".into(),
+            "/d/customers.parquet".into(),
+            "/d/events.ndjson".into(),
+        ],
+    }));
+    harness.settle();
+    assert_eq!(form(&harness).files_cursor, Some(2));
+    harness.press(Key::ArrowUp, Modifiers::NONE);
+    harness.press(Key::ArrowUp, Modifiers::NONE);
+    assert_eq!(form(&harness).files_cursor, Some(0));
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    harness.press(Key::X, Modifiers::NONE);
+    assert_eq!(
+        form(&harness).files_list,
+        ["/d/orders.csv", "/d/events.ndjson"]
+    );
+    // The footer's keys, as the keymap writes them.
+    let keymap = crate::keymap::Keymap::default();
+    for (command, word) in [
+        (crate::keymap::Command::FormNextField, "next"),
+        (crate::keymap::Command::FormChoose, "choose"),
+        (crate::keymap::Command::FormPick, "pick"),
+        (crate::keymap::Command::FormTest, "test"),
+        (crate::keymap::Command::FormSave, "save"),
+        (crate::keymap::Command::FormCancel, "cancel"),
+    ] {
+        let key = keymap.key(crate::keymap::Layout::Omarchy, command);
+        harness.settle();
+        let hint = format!("{} {word}", key.as_str());
+        assert!(harness.painted_rect(&hint).is_some(), "{hint}");
+    }
+}
+
+#[test]
+fn no_editor_key_is_h_or_l() {
+    let keymap = crate::keymap::Keymap::default();
+    for layout in [crate::keymap::Layout::Mac, crate::keymap::Layout::Omarchy] {
+        for letter in ["h", "l"] {
+            let found = keymap.typed(
+                layout,
+                crate::keymap::Scope::Prompt,
+                &|holds| holds == crate::keymap::When::ConnectionForm,
+                letter,
+            );
+            assert!(
+                matches!(found, crate::keymap::Typed::Nothing),
+                "{layout:?} {letter}: {found:?}"
+            );
+        }
+    }
+    // The same question finds the form's own letters.
+    let found = keymap.typed(
+        crate::keymap::Layout::Omarchy,
+        crate::keymap::Scope::Prompt,
+        &|holds| holds == crate::keymap::When::ConnectionForm,
+        "a",
+    );
+    assert!(matches!(
+        found,
+        crate::keymap::Typed::Command(crate::keymap::Command::FormAddFiles)
+    ));
+}
+
+#[test]
+fn the_terminal_keys_test_save_and_cancel_a_files_connection() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    let id = saved_exports(&mut harness, Source::Folder("/d".into()));
+    harness.app.apply(Action::EditConnection(id.clone()));
+    harness.settle();
+    harness.press(Key::T, Modifiers::CTRL);
+    assert!(matches!(form(&harness).test, TestState::Running(_)));
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert!(harness.app.dialog.is_none(), "the cancel key closes");
+    harness.app.apply(Action::EditConnection(id.clone()));
+    harness.settle();
+    form_mut(&mut harness).name = "Bookshop exports, renamed".into();
+    harness.press(Key::S, Modifiers::CTRL);
+    assert!(harness.app.dialog.is_none(), "the save key saves");
+    assert_eq!(
+        harness.app.connections.get(&id).unwrap().name,
+        "Bookshop exports, renamed"
+    );
+}
+
+#[test]
+fn a_connection_made_files_drops_what_agents_may_not_read() {
+    let mut harness = Harness::new();
+    let id = used_bookshop(&mut harness);
+    let mut saved = harness.app.connections.get(&id).unwrap().clone();
+    saved.agents.access = AgentAccess::Read;
+    saved.agents.tables =
+        tabletist_db::agent::Tables::AllExcept(vec!["customers".parse().unwrap()]);
+    harness.app.connections.upsert(saved);
+    harness.app.apply(Action::EditConnection(id.clone()));
+    harness.settle();
+    harness.click("Files");
+    pick_folder(&mut harness, "/data/bookshop-exports");
+    harness.app.apply(Action::SaveConnection { connect: false });
+    assert!(
+        form(&harness).message.as_deref().is_some_and(
+            |said| said.contains("What agents may not read or get masked is taken off too.")
+        ),
+        "{:?}",
+        form(&harness).message
+    );
+    harness.app.apply(Action::SaveConnection { connect: false });
+    let saved = harness.app.connections.get(&id).unwrap();
+    assert_eq!(saved.agents.tables, tabletist_db::agent::Tables::All);
+    assert!(saved.agents.masked.is_empty());
+}
+
+#[test]
+fn postgresql_made_mysql_is_not_asked_about() {
+    let mut harness = Harness::new();
+    let id = used_bookshop(&mut harness);
+    harness.app.apply(Action::EditConnection(id.clone()));
+    harness.settle();
+    harness.click("MySQL");
+    harness.app.apply(Action::SaveConnection { connect: false });
+    assert!(
+        harness.app.dialog.is_none(),
+        "{:?}",
+        harness.app.dialog.is_some()
+    );
+    assert_eq!(
+        harness.app.connections.get(&id).unwrap().spec.driver,
+        Driver::MySql
+    );
+}
+
+/// A saved Files connection over the Bookshop's exports.
+fn saved_exports(harness: &mut Harness, source: Source) -> ConnectionId {
+    let saved = SavedConnection {
+        id: ConnectionId::new(),
+        name: "Bookshop exports".into(),
+        environment: Environment::Local,
+        read_only: Some(true),
+        password: PasswordMode::None,
+        ssh_secret: PasswordMode::None,
+        spec: ConnectSpec::files(source),
+        agents: Default::default(),
+    };
+    let id = saved.id.clone();
+    harness.app.connections.upsert(saved);
+    id
+}
```

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -53,6 +53,8 @@ mod env_tests;
 pub mod every_row_prompt;
 #[cfg(test)]
 mod every_row_tests;
+#[cfg(test)]
+mod files_tests;
 pub mod filter_bar;
 pub mod focus;
 pub mod format;
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib files_tests`

Expected: it does not compile:

```text
error[E0432]: unresolved imports `crate::model::FilesFound`, `crate::model::FilesKind`
  --> src/ui/files_tests.rs:17:52
error[E0609]: no field `files_found` on type `&model::ConnectionForm`
  --> src/ui/files_tests.rs:79:54
error[E0609]: no field `files_found` on type `&model::ConnectionForm`
  --> src/ui/files_tests.rs:80:61
error[E0599]: no variant named `SourceRead` found for enum `backend::Event`
   --> src/ui/files_tests.rs:84:39
```

What Step 3's diffs do, by file: `model.rs` holds the form's Files fields (`files_kind`, `files_folder`, `files_list`, `files_cursor`, `files_found`, `files_tested`, `retype_asked`, `agents_chosen`), the new actions, `FilesKind`, `FilesFound`, `files_source`, `follow_agents_default` and `retyped_from`, and saves a Files connection read-only with no hidden tables or masked columns; `backend.rs` the folder and files pickers, the Source's read and Test's `FilesTested`; `app.rs` their handlers, the question before a type change, and a pasted path; `keymap.rs` four commands (`FormChoose`, Enter and Space, named by the footer and answered by egui's own press of the focused choice; `FormPick` `ctrl+o`, `FormAddFiles` `a`, `FormRemoveFile` `x`, all Omarchy only, which the keymap's generated clash tests check); `theme.rs` and `assets/icons` the file and folder icons; `ui/file_list.rs` the shared list and tags; `ui/connect_dialog/files.rs` the Source group; `choice.rs` the fourth type and a choice that is off with its reason (`Choice::off`), and the environment's choice moving a new Files connection's agents; `sheet.rs`, `terminal.rs` and `mod.rs` the Type row, the fixed box, the URL tab and header, the footer, the keys and Test's status; `agents.rs` the part without Tables and Masked columns; `ui/mod.rs` the modules and the taller window of one test; the two guide pages.

- [ ] **Step 3: Write the form, its keys and its component**

```diff
diff --git a/assets/icons/file.svg b/assets/icons/file.svg
new file mode 100644
--- /dev/null
+++ b/assets/icons/file.svg
@@ -0,0 +1,14 @@
+<svg
+  xmlns="http://www.w3.org/2000/svg"
+  width="24"
+  height="24"
+  viewBox="0 0 24 24"
+  fill="none"
+  stroke="#ffffff"
+  stroke-width="2"
+  stroke-linecap="round"
+  stroke-linejoin="round"
+>
+  <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
+  <path d="M14 2v4a2 2 0 0 0 2 2h4" />
+</svg>
```

```diff
diff --git a/assets/icons/folder.svg b/assets/icons/folder.svg
new file mode 100644
--- /dev/null
+++ b/assets/icons/folder.svg
@@ -0,0 +1,13 @@
+<svg
+  xmlns="http://www.w3.org/2000/svg"
+  width="24"
+  height="24"
+  viewBox="0 0 24 24"
+  fill="none"
+  stroke="#ffffff"
+  stroke-width="2"
+  stroke-linecap="round"
+  stroke-linejoin="round"
+>
+  <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
+</svg>
```

```diff
diff --git a/docs/_guide/agents.md b/docs/_guide/agents.md
--- a/docs/_guide/agents.md
+++ b/docs/_guide/agents.md
@@ -89,8 +89,9 @@ In the Omarchy look, `y` on the Agents pane copies Claude Code's command.
 
 ## Allow a connection
 
-Every connection starts with agents **Off**: they do not see it at all.
-**Read** lets them read it. Set it in any of these places:
+Every connection starts with agents **Off**: they do not see it at all,
+but a new Files connection, which starts with **Read** (**Off** on
+production). **Read** lets them read it. Set it in any of these places:
 
 - In Connections: a row's "…" menu, or `a` on a row in the Omarchy
   look.
```

```diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -94,7 +94,11 @@ numbered; once you type, it lists what matches. Its commands stay on `:`.
 | Ctrl+S | Save |
 | Ctrl+Enter | Save and connect |
 | Backspace, Delete | In the empty password field: forget the saved password |
-| Up, Down | In add table…'s or add column…'s list: move. Enter adds the one chosen, or in add column…'s puts a table chosen in the field for its columns, and Esc closes the field |
+| Up, Down | In add table…'s or add column…'s list: move. Enter adds the one chosen, or in add column…'s puts a table chosen in the field for its columns, and Esc closes the field. In a Files connection's list of files: move |
+| Enter, Space | Choose the choice the cursor is on: a type, an environment, folder or files |
+| Ctrl+O | A Files connection: pick its folder, or add files to its list, in the file chooser |
+| `a` | A Files connection's list: add files |
+| `x` | A Files connection's list: take the file under the cursor off the list. The file stays on disk |
 | Esc | Cancel |
 
 {% include shot.html file="omarchy-connection-edit" alt="The connection form in the Omarchy look, for a production database behind an SSH tunnel" width="2200" height="2032" %}
```

```diff
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -36,9 +36,10 @@ what was last open on it.
 | Field | Notes |
 | --- | --- |
 | **Name** | Left empty, it becomes `user@host:port/database` or the file's name |
-| **Type** | SQLite, PostgreSQL or MySQL |
+| **Type** | SQLite, PostgreSQL, MySQL or Files, on a line of its own above the name. Changing it keeps the name, the environment and whatever was typed for the other types until the dialog closes. Saving a connection you have used under another type asks first |
 | **Environment** | local, dev, staging, production or none. Sets the connection's color |
 | **File** | SQLite only |
+| **Source** | Files only. See [Files](#files) |
 | **Host**, **Port** | The port defaults to 5432 for PostgreSQL and 3306 for MySQL |
 | **Database** | Left empty, PostgreSQL uses the user's name and MySQL selects no database |
 | **User**, **Password** | See [Passwords](#passwords) |
@@ -51,6 +52,38 @@ what was last open on it.
 
 {% include shot.html file="macos-connection-edit" alt="The connection dialog with every field filled in for a production database behind an SSH tunnel" width="2880" height="1800" %}
 
+## Files
+
+A Files connection opens data files as tables: CSV, TSV, Parquet, JSON
+(an array), NDJSON (`.ndjson`, `.jsonl`), Excel (`.xlsx`, its first sheet)
+and DuckDB files (`.duckdb`). DuckDB reads them, inside Tabletist.
+
+- **Source** is a **Folder** or **Files**. Every supported file directly
+  in a folder becomes a table; subfolders, hidden files and other files
+  are left out, and a file added later shows the next time the connection
+  opens. **Files** is a list you add files to (**Add files…**); × takes a
+  file off the list and never off the disk.
+- Under the source, Tabletist says what it found: `4 files · 2.5 GB · CSV,
+  Parquet, NDJSON`, and the tables they become.
+- A table's name is the file's name without its extension, in lower case,
+  with anything but letters, digits and `_` turned into `_`. Two files of
+  the same name each take their extension: `orders_csv`,
+  `orders_parquet`. A `.duckdb` file is a schema, its tables inside it.
+- A new Files connection is **local**, and lets agents read. Files open
+  read-only: nothing writes to them, and a statement that would read
+  another file on the computer, write a file or install anything is
+  refused. Agents can query the files, never change them, and cannot
+  hide tables or mask columns on a Files connection.
+- **Test** reads each file's columns and its first rows, never a whole
+  file. The Excel reader is downloaded the first time a connection opens
+  an Excel file.
+- The SQL editor runs DuckDB's SQL, and the Structure view shows each
+  file's path, size and format beside its columns.
+- A connection has no URL: paste a folder's or a file's path into the
+  **URL** field of a new connection instead, and it becomes a Files
+  connection over it. A SQLite database's path (`.db`, `.sqlite`,
+  `.sqlite3`, `.db3`) opens as SQLite.
+
 ## Connection URLs
 
 The **URL** tab of the dialog (`u` in the Omarchy look) takes a connection
@@ -173,7 +206,9 @@ asks you for it again, and saves it in the keyring once it is accepted.
 ## Agents
 
 What AI agents may do on a connection, through `tabletist mcp`. Every
-connection starts with agents **Off**: they do not see it at all.
+connection starts with agents **Off**: they do not see it at all. A new
+[Files](#files) connection starts with **Read**, and with **Off** on
+production.
 **Read** lets them read it, in read-only transactions, while the agent
 server is on.
 
```

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -2903,8 +2903,19 @@ impl App {
                     // A port left at the old driver's default follows the driver.
                     let old_default = form.driver.default_port().to_string();
                     form.driver = driver;
+                    // A type changed again is asked about again.
+                    form.retype_asked = false;
+                    // A new Files connection lets agents read, until the
+                    // person chooses.
+                    form.follow_agents_default();
+                    // A Files connection has no URL to show.
                     match driver {
-                        Driver::Sqlite | Driver::Files => {}
+                        Driver::Files => form.url_mode = false,
+                        Driver::Postgres | Driver::MySql | Driver::Sqlite => {}
+                    }
+                    match driver {
+                        Driver::Sqlite => {}
+                        Driver::Files => self.read_form_source(),
                         Driver::Postgres | Driver::MySql => {
                             if form.port.trim().is_empty() || form.port.trim() == old_default {
                                 form.port = driver.default_port().to_string();
@@ -3018,6 +3029,7 @@ impl App {
                     form.agents_masked_opened = form.agents_masked.clone();
                     self.dialog = Some(Dialog::Connection(Box::new(form)));
                     self.list_ssh_hosts();
+                    self.read_form_source();
                 }
             }
             Action::LoadAgentColumns { conn, objects } => self.load_agent_columns(&conn, &objects),
@@ -3151,6 +3163,42 @@ impl App {
                     self.backend.pick_ca_file(request);
                 }
             }
+            Action::PickFolder => {
+                let request = RequestId(self.next_id());
+                if let Some(Dialog::Connection(form)) = &mut self.dialog {
+                    form.pick_request = Some(request);
+                    form.pick_target = PickTarget::Folder;
+                    self.backend.pick_folder(request);
+                }
+            }
+            Action::PickDataFiles => {
+                let request = RequestId(self.next_id());
+                if let Some(Dialog::Connection(form)) = &mut self.dialog {
+                    form.pick_request = Some(request);
+                    form.pick_target = PickTarget::Files;
+                    self.backend.pick_data_files(request);
+                }
+            }
+            Action::RemoveFile(at) => {
+                if let Some(Dialog::Connection(form)) = &mut self.dialog
+                    && at < form.files_list.len()
+                {
+                    // Off the list only: the file stays on the disk.
+                    form.files_list.remove(at);
+                    form.files_cursor = match form.files_list.len() {
+                        0 => None,
+                        rows => Some(at.min(rows - 1)),
+                    };
+                    self.read_form_source();
+                }
+            }
+            Action::SetFilesKind(kind) => {
+                if let Some(Dialog::Connection(form)) = &mut self.dialog {
+                    form.files_kind = kind;
+                    self.read_form_source();
+                }
+            }
+            Action::ReadSource => self.read_form_source(),
             Action::PickSshHost(alias) => {
                 if let Some(Dialog::Connection(form)) = &mut self.dialog {
                     form.pick_ssh_host(&alias);
@@ -3162,6 +3210,7 @@ impl App {
                 {
                     form.message = Some(message);
                 }
+                self.read_form_source();
             }
             Action::TrustHostKey => {
                 if let Some(Dialog::HostKey(prompt)) = self.dialog.take() {
@@ -4244,10 +4293,71 @@ impl App {
                                 form.name = crate::model::file_name(&form.sqlite_path);
                             }
                         }
-                        (None, _) => {}
+                        (Some(path), PickTarget::Folder) => {
+                            form.files_folder = tabletist_db::files::shown(&path);
+                            if form.name.trim().is_empty() {
+                                form.name = crate::model::file_name(&form.files_folder);
+                            }
+                            self.read_form_source();
+                        }
+                        // The dialog that adds files answers with
+                        // `FilesPicked`.
+                        (Some(_), PickTarget::Files) | (None, _) => {}
                     }
                 }
             }
+            Event::FilesPicked { request, paths } => {
+                if let Some(Dialog::Connection(form)) = &mut self.dialog
+                    && form.pick_request == Some(request)
+                {
+                    form.pick_request = None;
+                    if paths.is_empty() {
+                        return;
+                    }
+                    // Each once, after the files already listed.
+                    for path in &paths {
+                        let shown = tabletist_db::files::shown(path);
+                        if !form.files_list.contains(&shown) {
+                            form.files_list.push(shown);
+                        }
+                    }
+                    form.files_cursor = Some(form.files_list.len() - 1);
+                    self.read_form_source();
+                }
+            }
+            Event::SourceRead { request, result } => {
+                if let Some(Dialog::Connection(form)) = &mut self.dialog
+                    && form.files_found == crate::model::FilesFound::Reading(request)
+                {
+                    form.files_found = match result {
+                        Ok(scan) => crate::model::FilesFound::Read(scan),
+                        Err(why) => crate::model::FilesFound::Failed(why),
+                    };
+                }
+            }
+            Event::FilesTested { request, result } => {
+                if let Some(Dialog::Connection(form)) = &mut self.dialog
+                    && form.test == TestState::Running(request)
+                {
+                    let elapsed = form.test_started.take().map(|started| started.elapsed());
+                    form.test_took = if result.is_ok() { elapsed } else { None };
+                    form.test = match result {
+                        Ok(read) => {
+                            form.files_tested = Some(read);
+                            TestState::Passed
+                        }
+                        // A file's or a folder's own words, which name it.
+                        Err(Error::Connect(message)) => {
+                            form.files_tested = None;
+                            TestState::Failed(message)
+                        }
+                        Err(error) => {
+                            form.files_tested = None;
+                            TestState::Failed(error.to_string())
+                        }
+                    };
+                }
+            }
             Event::Schemas {
                 session,
                 request,
@@ -4888,6 +4998,28 @@ impl App {
         })
     }
 
+    /// Has the backend read what the open dialog's Files Source holds, for
+    /// its found line. A Source not chosen yet is not read.
+    fn read_form_source(&mut self) {
+        let request = RequestId(self.next_id());
+        let Some(Dialog::Connection(form)) = &mut self.dialog else {
+            return;
+        };
+        let source = match form.driver {
+            Driver::Files => form.files_source(),
+            Driver::Postgres | Driver::MySql | Driver::Sqlite => None,
+        };
+        // What a Test read was of the Source before it changed.
+        form.files_tested = None;
+        match source {
+            Some(source) => {
+                form.files_found = crate::model::FilesFound::Reading(request);
+                self.backend.read_source(request, source);
+            }
+            None => form.files_found = crate::model::FilesFound::Unread,
+        }
+    }
+
     /// Asks the backend for ~/.ssh/config's Host aliases for the open
     /// connection dialog.
     fn list_ssh_hosts(&mut self) {
@@ -5758,6 +5890,18 @@ impl App {
             form.message = Some("This connection was deleted.".into());
             return;
         }
+        // A connection that has been used, saved as another type, loses
+        // what its old type kept: asked once, and the next Save changes it.
+        if let (Some(id), Some(from)) = (&form.editing, form.retyped_from())
+            && self.connections.last_used(id).is_some()
+            && !form.retype_asked
+        {
+            form.retype_asked = true;
+            let rules =
+                form.agents_tables.tables().hides_anything() || !form.agents_masked.is_empty();
+            form.message = Some(retype_question(form.name.trim(), from, form.driver, rules));
+            return;
+        }
         let mut saved = match form.to_saved() {
             Ok(saved) => saved,
             Err(message) => {
@@ -8286,6 +8430,43 @@ fn use_typed_url(form: &mut ConnectionForm) -> bool {
 /// Fills the form from its URL field and leaves the URL mode, or says why
 /// the URL cannot be used: then the form is as it was, the URL too.
 fn apply_url(form: &mut ConnectionForm) -> Result<(), String> {
+    // A folder or a file's path: a Files connection over it.
+    if let Some(source) = files_path(&form.url) {
+        let name = match &source {
+            tabletist_db::files::Source::Folder(folder) => folder.clone(),
+            tabletist_db::files::Source::Files(files) => files
+                .first()
+                .and_then(|file| file.parent())
+                .map(std::path::Path::to_path_buf)
+                .unwrap_or_default(),
+        };
+        if form.name.trim().is_empty() {
+            form.name = crate::model::file_name(&name.display().to_string());
+        }
+        form.driver = Driver::Files;
+        match source {
+            tabletist_db::files::Source::Folder(folder) => {
+                form.files_kind = crate::model::FilesKind::Folder;
+                form.files_folder = tabletist_db::files::shown(&folder);
+            }
+            tabletist_db::files::Source::Files(files) => {
+                form.files_kind = crate::model::FilesKind::Files;
+                form.files_list = files
+                    .iter()
+                    .map(|file| tabletist_db::files::shown(file))
+                    .collect();
+            }
+        }
+        form.follow_agents_default();
+        form.url.clear();
+        form.message = None;
+        form.url_mode = false;
+        return Ok(());
+    }
+    // A SQLite database's path, as its URL would name it.
+    if let Some(path) = pasted_path(&form.url).filter(|path| sqlite_file(path)) {
+        form.url = format!("sqlite:{}", path.display());
+    }
     let tabletist_db::ParsedUrl {
         spec,
         secrets,
@@ -8345,6 +8526,60 @@ fn apply_url(form: &mut ConnectionForm) -> Result<(), String> {
     Ok(())
 }
 
+/// What Save asks before a used connection becomes another type: what it
+/// loses, and that Save again changes it.
+fn retype_question(name: &str, from: Driver, to: Driver, rules: bool) -> String {
+    let loses = match from {
+        Driver::Postgres | Driver::MySql => "Its server settings will be removed.",
+        Driver::Sqlite => "Its database file will be removed from it.",
+        Driver::Files => "Its folder or files will be removed from it.",
+    };
+    // A Files connection keeps no hidden tables and no masked columns.
+    let agents = match (to, rules) {
+        (Driver::Files, true) => " What agents may not read or get masked is taken off too.",
+        (Driver::Files, false) | (Driver::Postgres | Driver::MySql | Driver::Sqlite, _) => "",
+    };
+    format!(
+        "Change {name} from {} to {}? {loses}{agents} Save again to change it.",
+        from.label(),
+        to.label()
+    )
+}
+
+/// The path `text` is, where it is one (`/…`, `~…`, `C:\…`), with `~`
+/// read as the home folder: a URL or a word is none.
+fn pasted_path(text: &str) -> Option<std::path::PathBuf> {
+    let text = text.trim();
+    let windows_drive =
+        text.len() > 2 && text.as_bytes()[0].is_ascii_alphabetic() && text[1..].starts_with(":\\");
+    (text.starts_with('/') || text.starts_with('~') || windows_drive)
+        .then(|| tabletist_db::files::expanded(std::path::Path::new(text)))
+}
+
+/// Whether `path` names a SQLite database by its extension, as the SQLite
+/// picker's filter does.
+fn sqlite_file(path: &std::path::Path) -> bool {
+    path.extension()
+        .and_then(|extension| extension.to_str())
+        .is_some_and(|extension| {
+            ["db", "sqlite", "sqlite3", "db3"]
+                .iter()
+                .any(|known| extension.eq_ignore_ascii_case(known))
+        })
+}
+
+/// What a pasted path opens as a Files connection: a data file's path is
+/// a list of that file, any other path a folder, but a SQLite database's,
+/// which opens as SQLite. `None` for text that is no path (a URL, a
+/// word). Read from the text alone: the disk is the backend's.
+fn files_path(text: &str) -> Option<tabletist_db::files::Source> {
+    let path = pasted_path(text).filter(|path| !sqlite_file(path))?;
+    Some(match tabletist_db::files::Format::of(&path) {
+        Some(_) => tabletist_db::files::Source::Files(vec![path]),
+        None => tabletist_db::files::Source::Folder(path),
+    })
+}
+
 /// `text` with every byte outside `range` blanked, but its line breaks: the
 /// text a selection runs, each of its statements where it is in the
 /// editor. A character of several bytes is as many spaces.
```

```diff
diff --git a/src/backend.rs b/src/backend.rs
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -376,6 +376,23 @@ pub enum Event {
         request: RequestId,
         path: Option<PathBuf>,
     },
+    /// Files chosen in a dialog that takes several; none when it was
+    /// closed without a choice.
+    FilesPicked {
+        request: RequestId,
+        paths: Vec<PathBuf>,
+    },
+    /// What a Files connection's Source holds, for the dialog's found
+    /// line, or why it cannot be read.
+    SourceRead {
+        request: RequestId,
+        result: Result<tabletist_db::files::Scan, String>,
+    },
+    /// A Files connection's Test: what it read of every file.
+    FilesTested {
+        request: RequestId,
+        result: Result<tabletist_db::FilesRead, Error>,
+    },
     /// The Host aliases in ~/.ssh/config, for the connection dialog.
     SshHosts {
         request: RequestId,
@@ -509,6 +526,9 @@ pub struct Backend {
     /// the name the dialog suggests.
     #[cfg(test)]
     pub exports: Vec<(tabletist_mcp::LogFilter, String)>,
+    /// The Files connections' Sources a test asked to read.
+    #[cfg(test)]
+    pub sources_read: Vec<tabletist_db::files::Source>,
     #[cfg(test)]
     watched: Watched,
 }
@@ -582,6 +602,8 @@ impl Backend {
             #[cfg(test)]
             exports: Vec::new(),
             #[cfg(test)]
+            sources_read: Vec::new(),
+            #[cfg(test)]
             watched,
         }
     }
@@ -605,6 +627,8 @@ impl Backend {
             #[cfg(test)]
             exports: Vec::new(),
             #[cfg(test)]
+            sources_read: Vec::new(),
+            #[cfg(test)]
             watched: Watched::default(),
         }
     }
@@ -643,6 +667,64 @@ impl Backend {
         });
     }
 
+    /// Asks for a Files connection's folder.
+    pub fn pick_folder(&mut self, request: RequestId) {
+        let Some(runtime) = &self.runtime else {
+            return;
+        };
+        let dialog = rfd::AsyncFileDialog::new()
+            .set_title("Choose a folder of data files")
+            .pick_folder();
+        let outbox = self.outbox.clone();
+        runtime.spawn(async move {
+            let path = dialog.await.map(|folder| folder.path().to_path_buf());
+            outbox.emit(Event::FilePicked { request, path });
+        });
+    }
+
+    /// Asks for files to add to a Files connection, several at once.
+    pub fn pick_data_files(&mut self, request: RequestId) {
+        let Some(runtime) = &self.runtime else {
+            return;
+        };
+        let dialog = rfd::AsyncFileDialog::new()
+            .set_title("Add data files")
+            .add_filter(
+                "Data files",
+                &[
+                    "csv", "tsv", "parquet", "json", "ndjson", "jsonl", "xlsx", "duckdb",
+                ],
+            )
+            .pick_files();
+        let outbox = self.outbox.clone();
+        runtime.spawn(async move {
+            let paths = dialog
+                .await
+                .unwrap_or_default()
+                .iter()
+                .map(|file| file.path().to_path_buf())
+                .collect();
+            outbox.emit(Event::FilesPicked { request, paths });
+        });
+    }
+
+    /// Reads what a Files connection's Source holds, off the UI thread.
+    pub fn read_source(&mut self, request: RequestId, source: tabletist_db::files::Source) {
+        #[cfg(test)]
+        self.sources_read.push(source.clone());
+        let Some(runtime) = &self.runtime else {
+            return;
+        };
+        let outbox = self.outbox.clone();
+        runtime.spawn_blocking(move || {
+            let result = tabletist_db::files::scan(&source).map_err(|error| match error {
+                Error::Connect(message) | Error::InvalidSpec(message) => message,
+                other => other.to_string(),
+            });
+            outbox.emit(Event::SourceRead { request, result });
+        });
+    }
+
     /// Asks for an SSH private key, starting in ~/.ssh when it exists.
     pub fn pick_key_file(&mut self, request: RequestId) {
         let Some(runtime) = &self.runtime else {
@@ -1927,18 +2009,35 @@ impl Worker {
             } => {
                 let outbox = self.outbox.clone();
                 tokio::spawn(async move {
-                    let result = match Connection::connect_with(
-                        &spec,
-                        &secrets,
-                        &host_keys,
-                        Access::ReadOnly,
-                    )
-                    .await
-                    {
-                        Ok(connection) => connection.close().await,
-                        Err(error) => Err(error),
-                    };
-                    outbox.emit(Event::Tested { request, result });
+                    let connected =
+                        Connection::connect_with(&spec, &secrets, &host_keys, Access::ReadOnly)
+                            .await;
+                    match spec.driver {
+                        // A Files connection's Test reads the start of every
+                        // file too, and says how much it read.
+                        tabletist_db::Driver::Files => {
+                            let result = match connected {
+                                Ok(connection) => {
+                                    let read = connection.read_files().await.unwrap_or(Err(
+                                        Error::Unsupported("only a Files connection reads files"),
+                                    ));
+                                    let closed = connection.close().await;
+                                    read.and_then(|read| closed.map(|()| read))
+                                }
+                                Err(error) => Err(error),
+                            };
+                            outbox.emit(Event::FilesTested { request, result });
+                        }
+                        tabletist_db::Driver::Postgres
+                        | tabletist_db::Driver::MySql
+                        | tabletist_db::Driver::Sqlite => {
+                            let result = match connected {
+                                Ok(connection) => connection.close().await,
+                                Err(error) => Err(error),
+                            };
+                            outbox.emit(Event::Tested { request, result });
+                        }
+                    }
                 });
             }
             Command::Close { session } => {
```

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -672,6 +672,10 @@ commands! {
     FormForgetPassword = "form_forget_password",
     FormCancel = "form_cancel",
     FormTableMove = "form_table_move",
+    FormChoose = "form_choose",
+    FormPick = "form_pick",
+    FormAddFiles = "form_add_files",
+    FormRemoveFile = "form_remove_file",
     SettingsMove = "settings_move",
     SettingsChange = "settings_change",
     SettingsToggle = "settings_toggle",
@@ -1069,7 +1073,19 @@ impl Command {
             C::FormSaveAndConnect => info("Save and connect", "", Prompts),
             C::FormForgetPassword => info("Forget the saved password", "", Prompts),
             C::FormCancel => info("Cancel", "", Prompts),
-            C::FormTableMove => info("Move in Add table…'s or Add column…'s list", "", Prompts),
+            C::FormTableMove => info(
+                "Move in Add table…'s or Add column…'s list, or a Files connection's files",
+                "",
+                Prompts,
+            ),
+            C::FormChoose => info("Choose the choice under the cursor", "", Prompts),
+            C::FormPick => info("Pick a Files connection's folder or files", "", Prompts),
+            C::FormAddFiles => info("Add files to a Files connection", "", Prompts),
+            C::FormRemoveFile => info(
+                "Take the file under the cursor off a Files connection",
+                "",
+                Prompts,
+            ),
             C::SettingsMove => info("Move in the settings", "", Prompts),
             C::SettingsChange => info("Change the option", "", Prompts),
             C::SettingsToggle => info("Toggle the option", "", Prompts),
@@ -2346,7 +2362,8 @@ pub const BINDINGS: &[Binding] = {
         bind(C::FormCancel, PROMPT, keys(&[k("esc")]), keys(&[k("esc")]))
             .when(When::ConnectionForm),
         // In Add table…'s field, while its list of the window's tables
-        // shows: up moves up it, and down down.
+        // shows: up moves up it, and down down. And through a Files
+        // connection's list of files, in the terminal look.
         bind(
             C::FormTableMove,
             PROMPT,
@@ -2354,6 +2371,16 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k("up"), k("down")]),
         )
         .when(When::ConnectionForm),
+        // A Files connection's Source, in the terminal look: the portal's
+        // file chooser, and the list's own letters. The macOS sheet keeps
+        // its buttons, and no key of its own.
+        // A choice among several is a Tab stop each, chosen as a button
+        // is pressed: the footer says so.
+        bind(C::FormChoose, PROMPT, NONE, keys(&[k("enter"), k("space")]))
+            .when(When::ConnectionForm),
+        bind(C::FormPick, PROMPT, NONE, keys(&[k("ctrl+o")])).when(When::ConnectionForm),
+        bind(C::FormAddFiles, PROMPT, NONE, keys(&[k("a")])).when(When::ConnectionForm),
+        bind(C::FormRemoveFile, PROMPT, NONE, keys(&[k("x")])).when(When::ConnectionForm),
         bind(
             C::SettingsMove,
             PROMPT,
```

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -1,8 +1,10 @@
 //! Application state types and the actions that change them.
 
+use std::path::Path;
 use std::sync::Arc;
 use std::time::Duration;
 
+use tabletist_db::files::Source;
 use tabletist_db::ssh_config::{AgentSocket, ConfigHost, Proxy};
 use tabletist_db::{ConnectSpec, Driver, Filter, FilterOp, SshAuth, SshSpec, TlsMode};
 
@@ -102,6 +104,19 @@ pub enum Action {
     PickKeyFile,
     /// Open the native file dialog for the CA certificate.
     PickCaFile,
+    /// Open the native folder dialog for a Files connection's folder.
+    PickFolder,
+    /// Open the native file dialog to add files to a Files connection's
+    /// list, several at once.
+    PickDataFiles,
+    /// Take the file at this place off a Files connection's list. The
+    /// file itself stays where it is.
+    RemoveFile(usize),
+    /// Read a Files connection's folder or files from a list instead.
+    SetFilesKind(FilesKind),
+    /// Read what a Files connection's Source holds again, for the found
+    /// line: after its folder is typed.
+    ReadSource,
     /// Put this ~/.ssh/config Host alias in the SSH host field.
     PickSshHost(String),
     /// Ask an open window of the saved connection for the columns of these
@@ -1990,6 +2005,25 @@ pub struct ConnectionForm {
     pub ssh_hosts: Vec<ConfigHost>,
     /// The request whose answer fills `ssh_hosts`.
     pub ssh_hosts_request: Option<RequestId>,
+    /// Files: whether the Source is a folder or a list of files.
+    pub files_kind: FilesKind,
+    /// Files: the folder, as chosen or typed (`~` for the home folder).
+    pub files_folder: String,
+    /// Files: the list's files, in order, as they are shown.
+    pub files_list: Vec<String>,
+    /// Files: the row of the list the keyboard is on, by its place.
+    pub files_cursor: Option<usize>,
+    /// Files: what the Source holds, as the backend last read it, for the
+    /// found line and the list's rows.
+    pub files_found: FilesFound,
+    /// Files: what the last Test that passed read.
+    pub files_tested: Option<tabletist_db::FilesRead>,
+    /// Saving a used connection under another type was asked once: the
+    /// next Save changes it.
+    pub retype_asked: bool,
+    /// The person chose what agents may do: a type's default no longer
+    /// moves it.
+    pub agents_chosen: bool,
     /// What a running Test connects to, fixed when it started.
     pub test_spec: Option<ConnectSpec>,
     /// The secrets a running Test uses, filled as saved ones load.
@@ -2005,6 +2039,43 @@ pub enum PickTarget {
     Sqlite,
     KeyFile,
     CaFile,
+    /// A Files connection's folder.
+    Folder,
+    /// Files added to a Files connection's list.
+    Files,
+}
+
+/// Where a Files connection's files come from, as the Source choice says
+/// it.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
+pub enum FilesKind {
+    #[default]
+    Folder,
+    Files,
+}
+
+impl FilesKind {
+    pub const ALL: [Self; 2] = [Self::Folder, Self::Files];
+
+    pub fn label(self) -> &'static str {
+        match self {
+            Self::Folder => "Folder",
+            Self::Files => "Files",
+        }
+    }
+}
+
+/// What the backend last read of a Files connection's Source.
+#[derive(Debug, Clone, PartialEq, Eq, Default)]
+pub enum FilesFound {
+    /// Nothing chosen yet, or not read since the Source changed.
+    #[default]
+    Unread,
+    /// Asked for, by this request.
+    Reading(RequestId),
+    Read(tabletist_db::files::Scan),
+    /// Why the Source cannot be read: the folder is gone, say.
+    Failed(String),
 }
 
 /// How the SSH tunnel logs in, as the dialog offers it.
@@ -2100,6 +2171,14 @@ impl Default for ConnectionForm {
             saved_ssh_auth: None,
             ssh_hosts: Vec::new(),
             ssh_hosts_request: None,
+            files_kind: FilesKind::Folder,
+            files_folder: String::new(),
+            files_list: Vec::new(),
+            files_cursor: None,
+            files_found: FilesFound::Unread,
+            files_tested: None,
+            retype_asked: false,
+            agents_chosen: false,
             test_spec: None,
             test_secrets: tabletist_db::Secrets::default(),
             test_waiting: 0,
@@ -2211,6 +2290,11 @@ impl ConnectionForm {
     /// environment's default, which for production is
     /// [`Self::production_read_only`].
     pub fn read_only(&self) -> bool {
+        // Editing files is not built: a Files connection opens read-only.
+        match self.driver {
+            Driver::Files => return true,
+            Driver::Postgres | Driver::MySql | Driver::Sqlite => {}
+        }
         self.read_only.unwrap_or_else(|| {
             // Production is the one environment read-only by default.
             self.environment().read_only_by_default() && self.production_read_only
@@ -2222,6 +2306,11 @@ impl ConnectionForm {
     /// settings said when it was made; else not set, and the environment's
     /// rule applies.
     fn saved_read_only(&self) -> Option<bool> {
+        // A Files connection is saved read-only, as its box shows.
+        match self.driver {
+            Driver::Files => return Some(true),
+            Driver::Postgres | Driver::MySql | Driver::Sqlite => {}
+        }
         let new_production = self.editing.is_none() && self.environment().read_only_by_default();
         self.read_only
             .or_else(|| new_production.then(|| self.read_only()))
@@ -2319,10 +2408,81 @@ impl ConnectionForm {
                 SshAuth::KeyFile { .. } => SshAuthKind::KeyFile,
                 SshAuth::Agent => SshAuthKind::Agent,
             }),
+            files_kind: match &spec.files {
+                Some(Source::Files(_)) => FilesKind::Files,
+                Some(Source::Folder(_)) | None => FilesKind::Folder,
+            },
+            files_folder: match &spec.files {
+                Some(Source::Folder(folder)) => tabletist_db::files::shown(folder),
+                Some(Source::Files(_)) | None => String::new(),
+            },
+            files_list: match &spec.files {
+                Some(Source::Files(files)) => files
+                    .iter()
+                    .map(|file| tabletist_db::files::shown(file))
+                    .collect(),
+                Some(Source::Folder(_)) | None => Vec::new(),
+            },
+            // A saved connection's agents are what the file says.
+            agents_chosen: true,
             ..Self::default()
         }
     }
 
+    /// What a Files connection's Source names, as the backend reads it:
+    /// `None` until a folder or a file is chosen.
+    pub fn files_source(&self) -> Option<Source> {
+        match self.files_kind {
+            FilesKind::Folder => {
+                let folder = self.files_folder.trim();
+                (!folder.is_empty())
+                    .then(|| Source::Folder(tabletist_db::files::expanded(Path::new(folder))))
+            }
+            FilesKind::Files => (!self.files_list.is_empty()).then(|| {
+                Source::Files(
+                    self.files_list
+                        .iter()
+                        .map(|file| tabletist_db::files::expanded(Path::new(file.trim())))
+                        .collect(),
+                )
+            }),
+        }
+    }
+
+    /// Agents' access as a new connection starts it, until the person
+    /// chooses one: Read on a Files connection that is not production's,
+    /// Off everywhere else.
+    pub fn follow_agents_default(&mut self) {
+        if self.agents_chosen || self.editing.is_some() {
+            return;
+        }
+        let read = match self.driver {
+            Driver::Files => self.environment() != crate::env::Environment::Production,
+            Driver::Postgres | Driver::MySql | Driver::Sqlite => false,
+        };
+        self.agents.access = match read {
+            true => crate::connections::AgentAccess::Read,
+            false => crate::connections::AgentAccess::Off,
+        };
+    }
+
+    /// The type the connection was saved with, where the form now has
+    /// another that drops what it kept: a server's settings for a file's,
+    /// a file for a server or a folder. PostgreSQL and MySQL keep each
+    /// other's host, port and user.
+    pub fn retyped_from(&self) -> Option<Driver> {
+        let saved = self.saved_spec.as_ref()?.driver;
+        let drops = match (saved, self.driver) {
+            (Driver::Postgres | Driver::MySql, Driver::Postgres | Driver::MySql)
+            | (Driver::Sqlite, Driver::Sqlite)
+            | (Driver::Files, Driver::Files) => false,
+            (Driver::Postgres | Driver::MySql, Driver::Sqlite | Driver::Files)
+            | (Driver::Sqlite, Driver::Postgres | Driver::MySql | Driver::Files)
+            | (Driver::Files, Driver::Postgres | Driver::MySql | Driver::Sqlite) => true,
+        };
+        drops.then_some(saved)
+    }
+
     /// What to connect to, or why the fields do not describe a connection.
     pub fn to_spec(&self) -> Result<ConnectSpec, String> {
         match self.driver {
@@ -2333,7 +2493,13 @@ impl ConnectionForm {
                 }
                 Ok(ConnectSpec::sqlite(path))
             }
-            Driver::Files => Err("Choose a folder.".into()),
+            Driver::Files => match self.files_source() {
+                Some(source) => Ok(ConnectSpec::files(source)),
+                None => Err(match self.files_kind {
+                    FilesKind::Folder => "Choose a folder.".into(),
+                    FilesKind::Files => "Add a file.".into(),
+                }),
+            },
             Driver::Postgres | Driver::MySql => {
                 let host = self.host.trim();
                 if host.is_empty() {
@@ -2628,8 +2794,17 @@ impl ConnectionForm {
             access: self.agents.access,
             rows,
             timeout_secs,
-            tables: self.agents_tables.tables(),
-            masked: self.agents_masked.clone(),
+            // A Files connection keeps no hidden tables and no masked
+            // columns: a connection made another type loses its own,
+            // which nothing there could take off again.
+            tables: match self.driver {
+                Driver::Files => tabletist_db::agent::Tables::All,
+                Driver::Postgres | Driver::MySql | Driver::Sqlite => self.agents_tables.tables(),
+            },
+            masked: match self.driver {
+                Driver::Files => Vec::new(),
+                Driver::Postgres | Driver::MySql | Driver::Sqlite => self.agents_masked.clone(),
+            },
         })
     }
 }
```

```diff
diff --git a/src/theme.rs b/src/theme.rs
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -961,6 +961,8 @@ fastframe_icons::icons! {
         Code => "code",
         Database => "database",
         Download => "download",
+        File => "file",
+        Folder => "folder",
         Funnel => "funnel",
         Image => "image",
         KeyRound => "key-round",
```

```diff
diff --git a/src/ui/connect_dialog/agents.rs b/src/ui/connect_dialog/agents.rs
--- a/src/ui/connect_dialog/agents.rs
+++ b/src/ui/connect_dialog/agents.rs
@@ -2,7 +2,8 @@
 //! connection (Off or Read), the tables they may not read, the columns
 //! whose values they get masked, how many rows an answer holds, and how
 //! long a statement may run. The boards' rows but the third level, which
-//! comes with writes.
+//! comes with writes. A Files connection keeps no hidden tables and no
+//! masked columns: its part has the access, the rows and the timeout.
 
 use egui::{
     Align, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, vec2,
@@ -10,6 +11,8 @@ use egui::{
 
 use super::choice::{Choice, Group, choose};
 use super::{Skin, input};
+use tabletist_db::Driver;
+
 use crate::connections::AgentAccess;
 use crate::model::{ConnectionForm, TableFocus, TablesMode};
 use crate::typography::{Text, TextRole};
@@ -58,8 +61,29 @@ fn tables_hint(form: &ConnectionForm) -> Option<&'static str> {
 }
 
 /// What the part says under its rows, from the board.
-const NOTE: &str =
-    "When Open read-only is on, agents get Read at most. Every connection starts with agents Off.";
+const NOTE: &str = "When Open read-only is on, agents get Read at most. A new connection starts with agents Off, \
+     a Files one with Read.";
+
+/// What a Files connection's part says under its rows: agents read, and
+/// the files are never theirs to change.
+const FILES_NOTE: &str = "Agents can query the files, never change them.";
+
+/// Whether the connection keeps the Tables and Masked columns controls:
+/// a Files connection has no catalog that would hold them to a statement.
+fn guards_tables(form: &ConnectionForm) -> bool {
+    match form.driver {
+        Driver::Postgres | Driver::MySql | Driver::Sqlite => true,
+        Driver::Files => false,
+    }
+}
+
+/// The note under the part's rows.
+fn note(form: &ConnectionForm) -> &'static str {
+    match guards_tables(form) {
+        true => NOTE,
+        false => FILES_NOTE,
+    }
+}
 
 /// The access switch, as a group of choices headed `heading`.
 fn access(ui: &mut Ui, form: &mut ConnectionForm, heading: &str, group: Group, skin: &Skin) {
@@ -80,6 +104,8 @@ fn access(ui: &mut Ui, form: &mut ConnectionForm, heading: &str, group: Group, s
         skin,
     ) {
         form.agents.access = level;
+        // The person's choice: a type's default no longer moves it.
+        form.agents_chosen = true;
     }
 }
 
@@ -885,7 +911,9 @@ pub(super) fn sheet(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
         ui.add_space(12.0);
         // Tables and Masked columns side by side, as the board has them.
         let columns = [super::Column::Fr(1.0), super::Column::Fr(1.0)];
+        let guards = guards_tables(form);
         super::row(ui, 12.0, &columns, |ui, column| match column {
+            _ if !guards => {}
             0 => {
                 let tables = skin.say("Tables");
                 widgets::label(ui, small, &tables, palette.secondary, look);
@@ -913,7 +941,9 @@ pub(super) fn sheet(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
                 sentence(ui, small, &skin.say(MASKED_HINT), skin);
             }
         });
-        ui.add_space(12.0);
+        if guards {
+            ui.add_space(12.0);
+        }
         let (role, height) = (skin.value_role(), skin.field_height());
         super::row(ui, 12.0, &columns, |ui, column| {
             let (label, text) = match column {
@@ -929,7 +959,7 @@ pub(super) fn sheet(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
                 .labelled_by(label.id);
         });
         ui.add_space(8.0);
-        sentence(ui, small, &skin.say(NOTE), skin);
+        sentence(ui, small, &skin.say(note(form)), skin);
     });
     shown(ui, form, top);
 }
@@ -944,6 +974,29 @@ pub(super) fn terminal(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
     super::terminal::terminal_row_of(ui, &heading, line, skin, |ui, _| {
         access(ui, form, &heading, Group::Words, skin);
     });
+    if guards_tables(form) {
+        terminal_guards(ui, form, line, skin);
+    }
+    let (role, height) = (TextRole::OBody, skin.field_height());
+    super::terminal::terminal_row(ui, &skin.say("Rows per answer"), skin, |ui, label| {
+        ui.add(input(ui, &mut form.agents_rows, role, height, skin))
+            .labelled_by(label);
+    });
+    super::terminal::terminal_row(ui, &skin.say("Timeout, seconds"), skin, |ui, label| {
+        ui.add(input(ui, &mut form.agents_timeout, role, height, skin))
+            .labelled_by(label);
+    });
+    let mut said = skin.say(note(form));
+    if let Some(runs_as) = runs_as(skin) {
+        said = format!("{runs_as} {said}");
+    }
+    ui.add_space(4.0);
+    sentence(ui, TextRole::OSecondary, &said, skin);
+    shown(ui, form, top);
+}
+
+/// The terminal look's Tables and Masked rows.
+fn terminal_guards(ui: &mut Ui, form: &mut ConnectionForm, line: f32, skin: &Skin) {
     let tables = skin.say("Tables");
     super::terminal::terminal_row_of(ui, &tables, line, skin, |ui, _| {
         tables_mode(ui, form, &tables, Group::Words, skin);
@@ -967,20 +1020,4 @@ pub(super) fn terminal(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
     column_missing(ui, form, skin);
     sentence(ui, TextRole::OSecondary, &skin.say(MASKED_HINT), skin);
     ui.add_space(4.0);
-    let (role, height) = (TextRole::OBody, skin.field_height());
-    super::terminal::terminal_row(ui, &skin.say("Rows per answer"), skin, |ui, label| {
-        ui.add(input(ui, &mut form.agents_rows, role, height, skin))
-            .labelled_by(label);
-    });
-    super::terminal::terminal_row(ui, &skin.say("Timeout, seconds"), skin, |ui, label| {
-        ui.add(input(ui, &mut form.agents_timeout, role, height, skin))
-            .labelled_by(label);
-    });
-    let mut said = skin.say(NOTE);
-    if let Some(runs_as) = runs_as(skin) {
-        said = format!("{runs_as} {said}");
-    }
-    ui.add_space(4.0);
-    sentence(ui, TextRole::OSecondary, &said, skin);
-    shown(ui, form, top);
 }
```

```diff
diff --git a/src/ui/connect_dialog/choice.rs b/src/ui/connect_dialog/choice.rs
--- a/src/ui/connect_dialog/choice.rs
+++ b/src/ui/connect_dialog/choice.rs
@@ -14,7 +14,12 @@ use crate::ui::widgets;
 
 use super::Skin;
 
-const DRIVERS: [Driver; 3] = [Driver::Sqlite, Driver::Postgres, Driver::MySql];
+const DRIVERS: [Driver; 4] = [
+    Driver::Sqlite,
+    Driver::Postgres,
+    Driver::MySql,
+    Driver::Files,
+];
 
 /// One choice of a [`radio_group`], as it is drawn.
 #[derive(Clone, Copy)]
@@ -22,11 +27,27 @@ pub(super) struct Choice<'a> {
     label: &'a str,
     /// An environment's colours, when the choice is one.
     tint: Option<EnvColors>,
+    /// Why the choice cannot be made, where it cannot: it is still a Tab
+    /// stop, which says why to the pointer, the keyboard and a screen
+    /// reader.
+    off: Option<&'a str>,
 }
 
 impl<'a> Choice<'a> {
     pub(super) fn plain(label: &'a str) -> Self {
-        Self { label, tint: None }
+        Self {
+            label,
+            tint: None,
+            off: None,
+        }
+    }
+
+    /// A choice that cannot be made, for `why`.
+    pub(super) fn off(label: &'a str, why: &'a str) -> Self {
+        Self {
+            off: Some(why),
+            ..Self::plain(label)
+        }
     }
 }
 
@@ -154,14 +175,28 @@ fn radio_group(
         let cell = Rect::from_min_size(pos2(left, rect.top() + inset), vec2(widths[index], height));
         left += widths[index] + gap;
         let active = index == chosen;
-        let response = ui.interact(cell, ui.id().with((salt, index)), Sense::click());
+        let sense = match choice.off {
+            Some(_) => Sense::focusable_noninteractive() | Sense::hover(),
+            None => Sense::click(),
+        };
+        let mut response = ui.interact(cell, ui.id().with((salt, index)), sense);
+        let can = enabled && choice.off.is_none();
         response.widget_info(|| {
-            WidgetInfo::selected(WidgetType::RadioButton, enabled, active, choice.label)
+            WidgetInfo::selected(WidgetType::RadioButton, can, active, choice.label)
         });
-        if response.clicked() {
+        if let Some(why) = choice.off {
+            ui.ctx().accesskit_node_builder(response.id, |node| {
+                node.set_description(why);
+            });
+            if crate::ui::focus::shown(&response) {
+                response.show_tooltip_text(why);
+            }
+            response = response.on_hover_text(why);
+        }
+        if response.clicked() && choice.off.is_none() {
             clicked = Some(index);
         }
-        let hovered = response.hovered() && enabled;
+        let hovered = response.hovered() && can;
         let tint = choice.tint.as_ref();
         let color = match group {
             Group::Track(_) => {
@@ -179,6 +214,8 @@ fn radio_group(
                     tint.map_or(palette.text, |colors| skin.env_ink(colors))
                 } else if hovered {
                     palette.text
+                } else if choice.off.is_some() {
+                    palette.dim
                 } else {
                     palette.secondary
                 }
@@ -248,7 +285,7 @@ fn radio_group(
     clicked
 }
 
-/// The driver choice: SQLite, PostgreSQL or MySQL, under `heading`.
+/// The driver choice: SQLite, PostgreSQL, MySQL or Files, under `heading`.
 pub(super) fn driver_choice(
     ui: &mut Ui,
     form: &ConnectionForm,
@@ -309,6 +346,7 @@ pub(super) fn environment_choice(
             let choice = Choice {
                 label,
                 tint: Some(skin.colors(*environment)),
+                off: None,
             };
             (*environment, choice)
         })
@@ -329,5 +367,7 @@ pub(super) fn environment_choice(
     );
     if let Some(environment) = clicked {
         form.environment = Some(environment);
+        // A new Files connection's agents follow it: Off on production.
+        form.follow_agents_default();
     }
 }
```

```diff
diff --git a/src/ui/connect_dialog/files.rs b/src/ui/connect_dialog/files.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/connect_dialog/files.rs
@@ -0,0 +1,408 @@
+//! The Files form's Source: a folder, or a list of files, and what was
+//! found there, in either look. The backend reads the folder (or each
+//! file's size) after every change; this draws what it last read.
+
+use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
+
+use super::choice::{Choice, Group, choose};
+use super::{Skin, field_with_button, input};
+use crate::model::{Action, ConnectionForm, FilesFound, FilesKind};
+use crate::theme::Icon;
+use crate::typography::{Text, TextRole};
+use crate::ui::{file_list, widgets};
+
+/// What the Folder choice says under its field.
+const FOLDER_HINT: &str = "Every CSV, Parquet, JSON, NDJSON, Excel and .duckdb file in the \
+                           folder becomes a table. Files added later show up on the next open.";
+
+/// What the field says of a folder that is gone.
+const GONE: &str = "This folder isn't there any more. Choose it again, or pick another.";
+
+/// The found line: how many files, their size and their formats, and in
+/// the terminal look the tables they become after an arrow. `None` until
+/// the Source is read. Words in the look's case come from `say`.
+pub(super) fn found_line(found: &FilesFound, terminal: bool) -> Option<String> {
+    let FilesFound::Read(scan) = found else {
+        return None;
+    };
+    let count = scan.files.len();
+    let files = match count {
+        1 => "1 file".to_owned(),
+        count => format!("{count} files"),
+    };
+    let formats: Vec<&str> = scan.formats().into_iter().map(|f| f.label()).collect();
+    let mut line = format!(
+        "{files} · {}",
+        tabletist_db::files::size_text(scan.total_size())
+    );
+    if !formats.is_empty() {
+        line = format!("{line} · {}", formats.join(", "));
+    }
+    if terminal {
+        line = line.to_lowercase();
+        if count > 0 {
+            line = format!("{line} → {}", scan.names().join(", "));
+        }
+    }
+    Some(line)
+}
+
+/// Why the Source cannot be read, as the field says it: a folder that is
+/// gone in the board's words (translated), anything else as the backend
+/// said it.
+fn trouble(found: &FilesFound, skin: &Skin) -> Option<String> {
+    match found {
+        FilesFound::Failed(why) if why.ends_with(": no such folder") => Some(skin.say(GONE)),
+        // A path in it keeps its case, in either look.
+        FilesFound::Failed(why) => Some(why.clone()),
+        FilesFound::Unread | FilesFound::Reading(_) | FilesFound::Read(_) => None,
+    }
+}
+
+/// The list's rows: each file of the list, with its format and size where
+/// the backend found it, and marked where it is not there.
+pub(super) fn rows(form: &ConnectionForm) -> Vec<file_list::Row> {
+    let scan = match &form.files_found {
+        FilesFound::Read(scan) => Some(scan),
+        FilesFound::Unread | FilesFound::Reading(_) | FilesFound::Failed(_) => None,
+    };
+    form.files_list
+        .iter()
+        .map(|shown| {
+            let path = tabletist_db::files::expanded(std::path::Path::new(shown));
+            let found = scan.and_then(|scan| scan.files.iter().find(|file| file.path == path));
+            let kind = tabletist_db::files::Format::of(&path)
+                .map(|format| format.label().to_owned())
+                .unwrap_or_default();
+            file_list::Row {
+                path: shown.clone(),
+                kind,
+                size: found
+                    .map(|file| tabletist_db::files::size_text(file.size))
+                    .unwrap_or_default(),
+                missing: scan.is_some_and(|scan| scan.missing.contains(&path)),
+            }
+        })
+        .collect()
+}
+
+/// The Source's kind, as a group of choices headed `heading`.
+fn kind(
+    ui: &mut Ui,
+    form: &ConnectionForm,
+    heading: &str,
+    group: Group,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let labels = FilesKind::ALL.map(|kind| skin.say(kind.label()));
+    let options: Vec<(FilesKind, Choice<'_>)> = FilesKind::ALL
+        .iter()
+        .zip(&labels)
+        .map(|(kind, label)| (*kind, Choice::plain(label)))
+        .collect();
+    if let Some(kind) = choose(
+        ui,
+        "files-kind",
+        Some(heading),
+        &options,
+        form.files_kind,
+        group,
+        skin,
+    ) && kind != form.files_kind
+    {
+        actions.push(Action::SetFilesKind(kind));
+    }
+}
+
+/// macOS: the folder as a read-only field with its icon, red where it is
+/// gone.
+fn folder_box(ui: &mut Ui, form: &ConnectionForm, skin: &Skin) -> egui::Response {
+    let Skin { look, palette, .. } = *skin;
+    let height = skin.field_height();
+    let (rect, response) =
+        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    let gone = trouble(&form.files_found, skin).is_some();
+    let corner = CornerRadius::same(look.radius.saturating_sub(1));
+    ui.painter()
+        .rect_filled(rect, corner, palette.surface_hover);
+    ui.painter().rect_stroke(
+        rect,
+        corner,
+        Stroke::new(
+            1.0,
+            if gone {
+                palette.danger
+            } else {
+                palette.outline
+            },
+        ),
+        StrokeKind::Inside,
+    );
+    let center = rect.center().y;
+    Icon::Folder.image(palette.dim, 14.0).paint_at(
+        ui,
+        Rect::from_center_size(pos2(rect.left() + 17.0, center), vec2(14.0, 14.0)),
+    );
+    let (text, color) = match form.files_folder.trim() {
+        "" => (skin.say("No folder chosen"), palette.dim),
+        folder => (folder.to_owned(), palette.text),
+    };
+    let role = TextRole::MonoSecondary;
+    let room = rect.width() - 42.0;
+    let shown = crate::ui::grid::ellipsize(&text, room, true, |text| {
+        role.width(ui.ctx(), look.faces, text)
+    });
+    widgets::paint_label(
+        ui,
+        rect.left() + 32.0,
+        center,
+        Text::one(look, role, &shown, color),
+    );
+    widgets::announce(ui, rect, &format!("{} {text}", skin.say("Folder")));
+    response
+}
+
+/// macOS: the "Source" fieldset's inside.
+pub(super) fn sheet(
+    ui: &mut Ui,
+    form: &mut ConnectionForm,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Skin { look, palette, .. } = *skin;
+    let small = TextRole::Secondary;
+    kind(
+        ui,
+        form,
+        &skin.say("Source"),
+        Group::Track(24.0),
+        skin,
+        actions,
+    );
+    ui.add_space(10.0);
+    match form.files_kind {
+        FilesKind::Folder => {
+            let (_, clicked) =
+                field_with_button(ui, &skin.say("Choose…"), "Choose a folder", skin, |ui| {
+                    folder_box(ui, form, skin)
+                });
+            if clicked {
+                actions.push(Action::PickFolder);
+            }
+        }
+        FilesKind::Files => {
+            let rows = rows(form);
+            if !rows.is_empty() {
+                let asked = file_list::show(
+                    ui,
+                    &skin.say("Files"),
+                    &rows,
+                    form.files_cursor,
+                    (look, palette, skin.locale),
+                );
+                if let Some(at) = asked.removed {
+                    actions.push(Action::RemoveFile(at));
+                }
+                if let Some(at) = asked.chosen {
+                    form.files_cursor = Some(at);
+                }
+                ui.add_space(8.0);
+            }
+            let name = crate::i18n::gettext(skin.locale, "Add files");
+            let text = skin.say("Add files…");
+            let spec = super::side_button(&text, &name, skin);
+            if spec.show(ui, skin.field_height(), look, palette).clicked() {
+                actions.push(Action::PickDataFiles);
+            }
+        }
+    }
+    ui.add_space(8.0);
+    match trouble(&form.files_found, skin) {
+        Some(why) => {
+            super::note(ui, &why, palette.danger, skin);
+        }
+        None => {
+            if let Some(line) = found_line(&form.files_found, false) {
+                widgets::label(ui, small, &line, palette.secondary, look);
+                if let FilesFound::Read(scan) = &form.files_found
+                    && !scan.files.is_empty()
+                {
+                    ui.add_space(6.0);
+                    file_list::tags(ui, &scan.names(), look, palette);
+                }
+            }
+            match form.files_kind {
+                FilesKind::Folder => {
+                    ui.add_space(6.0);
+                    super::note(ui, &skin.say(FOLDER_HINT), palette.dim, skin);
+                }
+                FilesKind::Files => {}
+            }
+        }
+    }
+}
+
+/// The terminal look: the "source" section's rows. The folder is typed
+/// or picked (`ctrl+o`); the list's rows are moved through with the
+/// arrows, `a` adds files and `x` takes the row off.
+pub(super) fn terminal(
+    ui: &mut Ui,
+    form: &mut ConnectionForm,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Skin { look, palette, .. } = *skin;
+    let role = TextRole::OBody;
+    let line = role.row_height(ui.ctx(), look.faces);
+    super::terminal::terminal_section(ui, &skin.say("Source"), skin);
+    // The row says `kind`, as the board writes it; a screen reader hears
+    // the choice as "Source", as on macOS.
+    let source = skin.say("Source");
+    super::terminal::terminal_row_of(
+        ui,
+        &skin.say("Kind"),
+        Group::Words.height(),
+        skin,
+        |ui, _| {
+            kind(ui, form, &source, Group::Words, skin, actions);
+        },
+    );
+    match form.files_kind {
+        FilesKind::Folder => {
+            let pick = crate::ui::keys::written(ui.ctx(), look, crate::keymap::Command::FormPick);
+            super::terminal::terminal_row(ui, &skin.say("Folder"), skin, |ui, label| {
+                let hint = super::hint(
+                    ui,
+                    &format!("{} {}", pick.as_str(), skin.say("pick")),
+                    role,
+                    skin,
+                );
+                let field = ui
+                    .add(
+                        input(ui, &mut form.files_folder, role, skin.field_height(), skin)
+                            .hint_text(hint),
+                    )
+                    .labelled_by(label);
+                if field.lost_focus() {
+                    actions.push(Action::ReadSource);
+                }
+            });
+        }
+        FilesKind::Files => {
+            let rows = rows(form);
+            if rows.is_empty() {
+                super::terminal::terminal_row_of(ui, &skin.say("Files"), line, skin, |ui, _| {
+                    let add = crate::ui::keys::written(
+                        ui.ctx(),
+                        look,
+                        crate::keymap::Command::FormAddFiles,
+                    );
+                    widgets::label(
+                        ui,
+                        TextRole::OSecondary,
+                        &format!(
+                            "{} {} {}",
+                            skin.say("none ·"),
+                            add.as_str(),
+                            skin.say("adds files")
+                        ),
+                        palette.dim,
+                        look,
+                    );
+                });
+            } else {
+                let height = rows.len() as f32 * 26.0 + 2.0;
+                super::terminal::terminal_row_of(ui, &skin.say("Files"), height, skin, |ui, _| {
+                    let asked = file_list::show(
+                        ui,
+                        &skin.say("Files"),
+                        &rows,
+                        form.files_cursor.or(Some(0)),
+                        (look, palette, skin.locale),
+                    );
+                    if let Some(at) = asked.removed {
+                        actions.push(Action::RemoveFile(at));
+                    }
+                    if let Some(at) = asked.chosen {
+                        form.files_cursor = Some(at);
+                    }
+                });
+            }
+        }
+    }
+    let (text, color) = match trouble(&form.files_found, skin) {
+        Some(why) => (Some(why), palette.danger),
+        None => (found_line(&form.files_found, true), palette.dim),
+    };
+    if let Some(text) = text {
+        super::terminal::terminal_row_of(ui, "", line, skin, |ui, _| {
+            Text::one(look, TextRole::OSecondary, &text, color)
+                .wrap(ui.available_width())
+                .layout(ui.ctx())
+                .label(ui);
+        });
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use tabletist_db::files::{Format, Found, Scan};
+
+    fn scan() -> Scan {
+        let found = |name: &str, format: Format, size: u64| Found {
+            path: format!("/data/{name}").into(),
+            format,
+            size,
+            name: name.split('.').next().unwrap_or_default().to_owned(),
+        };
+        Scan {
+            files: vec![
+                found("orders.csv", Format::Csv, 1_600_000_000),
+                found("customers.parquet", Format::Parquet, 400_000_000),
+                found("events.ndjson", Format::Ndjson, 450_000_000),
+                found("returns.csv", Format::Csv, 50_000_000),
+            ],
+            missing: Vec::new(),
+        }
+    }
+
+    #[test]
+    fn the_found_line_says_the_files_their_size_and_formats() {
+        let found = FilesFound::Read(scan());
+        assert_eq!(
+            found_line(&found, false).as_deref(),
+            Some("4 files · 2.5 GB · CSV, Parquet, NDJSON")
+        );
+        assert_eq!(
+            found_line(&found, true).as_deref(),
+            Some("4 files · 2.5 gb · csv, parquet, ndjson → orders, customers, events, returns")
+        );
+        assert_eq!(found_line(&FilesFound::Unread, false), None);
+        assert_eq!(
+            found_line(&FilesFound::Read(Scan::default()), true).as_deref(),
+            Some("0 files · 0 b")
+        );
+    }
+
+    #[test]
+    fn a_folder_that_is_gone_is_said_in_the_boards_words() {
+        let look = crate::theme::Look::macos();
+        let palette = crate::theme::Palette::light();
+        let skin = Skin::new(
+            &look,
+            &palette,
+            crate::i18n::Locale::default(),
+            crate::env::Environment::Local,
+        );
+        let gone = FilesFound::Failed("Can't open ~/Data/bookshop-exports: no such folder".into());
+        assert_eq!(trouble(&gone, &skin).as_deref(), Some(GONE));
+        let other = FilesFound::Failed("Can't open ~/x: not allowed to read it".into());
+        assert_eq!(
+            trouble(&other, &skin).as_deref(),
+            Some("Can't open ~/x: not allowed to read it")
+        );
+        assert_eq!(trouble(&FilesFound::Read(scan()), &skin), None);
+    }
+}
```

```diff
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -5,6 +5,7 @@
 
 mod agents;
 mod choice;
+mod files;
 mod sheet;
 mod terminal;
 
@@ -19,7 +20,7 @@ use crate::connections::PasswordMode;
 use crate::env::{EnvColors, Environment, Platform, env_colors};
 use crate::i18n::{Locale, gettext};
 use crate::keymap::{Command, Scope};
-use crate::model::{Action, ConnectionForm, Dialog, SshAuthKind, SshHints, TestState};
+use crate::model::{Action, ConnectionForm, Dialog, FilesKind, SshAuthKind, SshHints, TestState};
 use crate::theme::{self, Icon, Look, Palette, Typeface};
 use crate::typography::{Text, TextRole};
 use crate::ui::menu;
@@ -37,6 +38,37 @@ const TLS_MODES: [(TlsMode, &str); 5] = [
     (TlsMode::VerifyFull, "verify-full"),
 ];
 
+/// What the Type row says beside its choices: what a choice changes, and
+/// what the file types are.
+fn type_hint(driver: Driver) -> &'static str {
+    match driver {
+        Driver::Sqlite => "One database file on this computer",
+        Driver::Files => "Opens CSV, Parquet, JSON, Excel and .duckdb files as tables",
+        Driver::Postgres | Driver::MySql => "Changes the fields below",
+    }
+}
+
+/// Whether the form is a Files connection's: no URL, a Source, keys of
+/// its own.
+fn is_files(form: &ConnectionForm) -> bool {
+    match form.driver {
+        Driver::Files => true,
+        Driver::Postgres | Driver::MySql | Driver::Sqlite => false,
+    }
+}
+
+/// Whether the read-only box is on and cannot be turned off: a Files
+/// connection's, until editing files is built.
+fn read_only_fixed(form: &ConnectionForm) -> bool {
+    is_files(form)
+}
+
+/// What a Files connection's read-only box says.
+const FILES_READ_ONLY_NOTE: &str = "Files open read-only.";
+
+/// Why a Files connection's URL tab is off, as its tooltip says it.
+const NO_URL: &str = "Files connections have no URL. Choose a folder or files under Source.";
+
 const INTERCEPT_WARNING: &str = "The password can be intercepted on the network. \
                                  To prevent it, verify the certificate and host, \
                                  or use an SSH tunnel.";
@@ -68,7 +100,7 @@ struct Placement {
 /// without a tunnel or a message, rounded up. The sheet's is the macOS
 /// look's, the taller of its two.
 fn reserve(look: &Look) -> f32 {
-    if look.terminal { 615.0 } else { 815.0 }
+    if look.terminal { 615.0 } else { 885.0 }
 }
 
 /// Where the top edge of a dialog `height` tall goes on a screen
@@ -312,6 +344,13 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
             if on(input, Command::FormTest).fresh && !testing {
                 actions.push(Action::TestConnection);
             }
+            // A Files connection's file chooser.
+            if is_files(form) && on(input, Command::FormPick).fresh {
+                actions.push(match form.files_kind {
+                    FilesKind::Folder => Action::PickFolder,
+                    FilesKind::Files => Action::PickDataFiles,
+                });
+            }
         });
     }
     let skin = Skin::new(&look, &palette, locale, form.environment());
@@ -403,7 +442,7 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
             // one, when it is not a letter being typed.
             let mut toggled = false;
             if !typing {
-                let form = crate::keymap::When::ConnectionForm;
+                let when = crate::keymap::When::ConnectionForm;
                 input.events.retain(|event| {
                     let egui::Event::Key {
                         key,
@@ -418,12 +457,42 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
                         .filter(|_| !(modifiers.command || modifiers.ctrl || modifiers.alt));
                     let command = letter.and_then(|letter| {
                         let typed = letter.to_string();
-                        crate::ui::keys::prompt_letter(&keymap, layout, form, &typed)
+                        crate::ui::keys::prompt_letter(&keymap, layout, when, &typed)
                     });
-                    let pastes = command == Some(Command::FormPasteUrl);
-                    toggled |= pastes;
-                    !pastes
+                    // A Files connection has no URL; its list has letters
+                    // of its own.
+                    let files = is_files(form);
+                    let listed = files && form.files_kind == FilesKind::Files;
+                    match command {
+                        Some(Command::FormPasteUrl) if !files => {
+                            toggled = true;
+                            false
+                        }
+                        Some(Command::FormAddFiles) if listed => {
+                            actions.push(Action::PickDataFiles);
+                            false
+                        }
+                        Some(Command::FormRemoveFile) if listed => {
+                            if let Some(at) = form
+                                .files_cursor
+                                .or((!form.files_list.is_empty()).then_some(0))
+                            {
+                                actions.push(Action::RemoveFile(at));
+                            }
+                            false
+                        }
+                        _ => true,
+                    }
                 });
+                // The arrows move through the list of files.
+                if is_files(form) && form.files_kind == FilesKind::Files {
+                    let rows = form.files_list.len();
+                    let step = moves(input, &keymap, layout);
+                    if rows > 0 && step != 0 {
+                        let at = form.files_cursor.unwrap_or(0) as isize + step;
+                        form.files_cursor = Some(at.clamp(0, rows as isize - 1) as usize);
+                    }
+                }
             }
             toggled
         });
@@ -436,6 +505,23 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
     app.actions.extend(actions);
 }
 
+/// How far the form's list was asked to move this frame, by the keys the
+/// keymap gives it: up by one, down by one, or not.
+fn moves(
+    input: &mut egui::InputState,
+    keymap: &crate::keymap::Keymap,
+    layout: crate::keymap::Layout,
+) -> isize {
+    let mut step = 0;
+    for keys in keymap.chords_now(layout, Command::FormTableMove, Scope::Prompt, true) {
+        let way = if keys == "up" { -1 } else { 1 };
+        if crate::ui::keys::presses(input, layout, keys).count > 0 {
+            step += way;
+        }
+    }
+    step
+}
+
 /// Shows or hides the URL field; shown, it takes the keyboard.
 fn show_url(ctx: &egui::Context, form: &mut ConnectionForm, shown: bool) {
     form.url_mode = shown;
@@ -1161,7 +1247,20 @@ fn status(form: &ConnectionForm, skin: &Skin) -> Option<Status> {
                 .test_spec
                 .as_ref()
                 .map_or(form.driver, |spec| spec.driver);
-            let mut text = format!("{} · {}", gettext(skin.locale, "Connected"), driver.label());
+            let mut text = match &form.files_tested {
+                // What a Files connection's Test read: "Read 3 files ·
+                // 2.4 GB".
+                Some(read) => format!(
+                    "{} {} · {}",
+                    gettext(skin.locale, "Read"),
+                    match read.files {
+                        1 => gettext(skin.locale, "1 file").into_owned(),
+                        files => format!("{files} {}", gettext(skin.locale, "files")),
+                    },
+                    tabletist_db::files::size_text(read.bytes)
+                ),
+                None => format!("{} · {}", gettext(skin.locale, "Connected"), driver.label()),
+            };
             if let Some(took) = form.test_took {
                 text = format!("{text} · {}", crate::ui::format::elapsed(took));
             }
```

```diff
diff --git a/src/ui/connect_dialog/sheet.rs b/src/ui/connect_dialog/sheet.rs
--- a/src/ui/connect_dialog/sheet.rs
+++ b/src/ui/connect_dialog/sheet.rs
@@ -86,10 +86,13 @@ pub(super) fn header(
     );
     let parameters = gettext(skin.locale, "Parameters");
     let url = gettext(skin.locale, "URL");
-    let tabs = [
-        (false, Choice::plain(&parameters)),
-        (true, Choice::plain(&url)),
-    ];
+    // A Files connection has no URL: its tab says why, and is not chosen.
+    let no_url = skin.say(super::NO_URL);
+    let url_choice = match super::is_files(form) {
+        true => Choice::off(&url, &no_url),
+        false => Choice::plain(&url),
+    };
+    let tabs = [(false, Choice::plain(&parameters)), (true, url_choice)];
     let clicked = choose(
         &mut tabs_ui,
         "input",
@@ -267,7 +270,11 @@ pub(super) fn body(
                             file_field(ui, form, skin, actions);
                         });
                     }
-                    Driver::Files => {}
+                    Driver::Files => {
+                        fieldset(ui, &skin.say("Source"), skin, |ui| {
+                            super::files::sheet(ui, form, skin, actions);
+                        });
+                    }
                     Driver::Postgres | Driver::MySql => {
                         fieldset(ui, &skin.say("Server"), skin, |ui| {
                             server(ui, form, skin, actions);
@@ -289,7 +296,8 @@ pub(super) fn body(
         });
 }
 
-/// The name beside the kind of database, then the environment.
+/// The kind of database on a line of its own (four do not fit beside the
+/// name), with what it changes; then the name, then the environment.
 fn identity(
     ui: &mut Ui,
     form: &mut ConnectionForm,
@@ -298,26 +306,27 @@ fn identity(
     actions: &mut Vec<Action>,
 ) {
     let Skin { look, palette, .. } = *skin;
-    row(
-        ui,
-        12.0,
-        &[Column::Fr(1.4), Column::Fr(1.0)],
-        |ui, column| {
-            if column == 0 {
-                let name = labelled(ui, &skin.say("Name"), TextRole::FormLabel, skin, |ui| {
-                    ui.add(input(ui, &mut form.name, TextRole::UiBody, 34.0, skin))
-                });
-                if focus_name {
-                    name.request_focus();
-                }
-            } else {
-                let heading = skin.say("Type");
-                widgets::label(ui, TextRole::FormLabel, &heading, palette.secondary, look);
-                ui.add_space(5.0);
-                driver_choice(ui, form, &heading, skin, actions);
-            }
-        },
-    );
+    let heading = skin.say("Type");
+    widgets::label(ui, TextRole::FormLabel, &heading, palette.secondary, look);
+    ui.add_space(5.0);
+    ui.horizontal_wrapped(|ui| {
+        ui.spacing_mut().item_spacing.x = 12.0;
+        driver_choice(ui, form, &heading, skin, actions);
+        widgets::label(
+            ui,
+            TextRole::Secondary,
+            &skin.say(super::type_hint(form.driver)),
+            palette.dim,
+            look,
+        );
+    });
+    ui.add_space(18.0);
+    let name = labelled(ui, &skin.say("Name"), TextRole::FormLabel, skin, |ui| {
+        ui.add(input(ui, &mut form.name, TextRole::UiBody, 34.0, skin))
+    });
+    if focus_name {
+        name.request_focus();
+    }
     ui.add_space(18.0);
     let heading = skin.say("Environment");
     widgets::label(ui, TextRole::FormLabel, &heading, palette.secondary, look);
@@ -576,7 +585,8 @@ fn ssh_fields(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut
 fn env_check(ui: &mut Ui, on: bool, name: &str, skin: &Skin) -> Response {
     // Three below the line's top, as the design sets the box.
     let (rect, response) = ui.allocate_exact_size(vec2(14.0, 17.0), Sense::click());
-    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, name));
+    let enabled = ui.is_enabled();
+    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, on, name));
     let mark = Rect::from_min_size(rect.min + vec2(0.0, 3.0), vec2(14.0, 14.0));
     let radius = skin.look.radius.min(4);
     // The ring goes round the box, not round the line it sits in.
@@ -621,6 +631,12 @@ const READ_ONLY_NOTE: &str =
 fn safety(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
     let Skin { look, palette, .. } = *skin;
     let title = skin.say("Open read-only");
+    // Editing files is not built: their box is on, and stays so.
+    let fixed = super::read_only_fixed(form);
+    let note = match fixed {
+        true => super::FILES_READ_ONLY_NOTE,
+        false => READ_ONLY_NOTE,
+    };
     egui::Frame::new()
         .fill(skin.env.bar_bg())
         .corner_radius(CornerRadius::same(10))
@@ -630,7 +646,9 @@ fn safety(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
             ui.horizontal_top(|ui| {
                 ui.spacing_mut().item_spacing.x = 10.0;
                 let on = form.read_only();
-                let mut toggled = env_check(ui, on, &title, skin).clicked();
+                let mut toggled = ui
+                    .add_enabled_ui(!fixed, |ui| env_check(ui, on, &title, skin).clicked())
+                    .inner;
                 ui.vertical(|ui| {
                     // The title toggles the box under the pointer, as a
                     // check box's own label does. It takes clicks alone:
@@ -638,20 +656,14 @@ fn safety(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
                     let label = Text::one(look, TextRole::UiBodyStrong, &title, palette.text)
                         .layout(ui.ctx())
                         .label_sense(ui, Sense::CLICK);
-                    toggled |= label.clicked();
+                    toggled |= label.clicked() && !fixed;
                     ui.add_space(2.0);
                     // The secondary colour, warmed by the environment's
                     // text colour: 0.4 is the share at which production's
                     // red on the light palette comes out in the design's
                     // tone.
                     let tone = theme::mix(palette.secondary, skin.env_ink(&skin.env), 0.4);
-                    widgets::label(
-                        ui,
-                        TextRole::Secondary,
-                        &skin.say(READ_ONLY_NOTE),
-                        tone,
-                        look,
-                    );
+                    widgets::label(ui, TextRole::Secondary, &skin.say(note), tone, look);
                 });
                 if toggled {
                     form.read_only = Some(!on);
```

```diff
diff --git a/src/ui/connect_dialog/terminal.rs b/src/ui/connect_dialog/terminal.rs
--- a/src/ui/connect_dialog/terminal.rs
+++ b/src/ui/connect_dialog/terminal.rs
@@ -26,10 +26,22 @@ pub(super) fn terminal_header(ui: &mut Ui, form: &mut ConnectionForm, skin: &Ski
     let Skin { look, palette, .. } = *skin;
     let rect = terminal_dialog::head(ui, skin.inner_radius(), skin.env.bar_bg(), palette);
     let y = terminal_dialog::head_line(rect);
+    // A Files connection has no URL: the hint says so, and is no key.
+    let files = super::is_files(form);
     let label = skin.say("Paste URL");
     let key = crate::ui::keys::written(ui.ctx(), look, Command::FormPasteUrl);
     let hint = [(&key, label.as_str(), true)];
-    let width = widgets::key_hints_width(ui, &hint, 0.0, look, palette);
+    let none = Text::one(
+        look,
+        TextRole::OSecondary,
+        &skin.say("No URL for files"),
+        palette.dim,
+    )
+    .layout(ui.ctx());
+    let width = match files {
+        true => none.width(),
+        false => widgets::key_hints_width(ui, &hint, 0.0, look, palette),
+    };
     let place = Rect::from_min_size(
         pos2(rect.right() - 14.0 - width, rect.top()),
         vec2(width, 40.0),
@@ -56,6 +68,11 @@ pub(super) fn terminal_header(ui: &mut Ui, form: &mut ConnectionForm, skin: &Ski
         room,
         skin,
     );
+    if files {
+        none.paint_left(ui.painter(), place.left(), y);
+        widgets::announce(ui, place, none.galley.text());
+        return;
+    }
     widgets::key_hints(ui, (place.left(), y), &hint, 0.0, look, palette);
     let name = gettext(skin.locale, "Paste URL");
     if ButtonSpec::new(&name).hidden_at(ui, place).clicked() {
@@ -63,6 +80,10 @@ pub(super) fn terminal_header(ui: &mut Ui, form: &mut ConnectionForm, skin: &Ski
     }
 }
 
+/// A key of the footer: its command, what it does, and the button it
+/// stands for with what that does.
+type FooterKey<'a> = (Command, &'a str, Option<(&'a str, Action)>);
+
 /// The terminal look: the Test's status at the left, the keys at the
 /// right. Each key's hint is its button too. The status comes first: the
 /// hints that do not fit beside it are left out, from the left, and their
@@ -80,29 +101,44 @@ pub(super) fn terminal_footer(
     // The command whose key it is, what it does, the button it stands
     // for, and what that does.
     let key = |command| crate::ui::keys::written(ui.ctx(), look, command);
-    let keys = [
-        (Command::FormNextField, "next", None),
-        (
-            Command::FormTest,
-            "test",
-            Some(("Test", Action::TestConnection)),
-        ),
-        (
-            Command::FormSave,
-            "save",
-            Some(("Save", Action::SaveConnection { connect: false })),
-        ),
-        (
-            Command::FormSaveAndConnect,
-            "connect",
-            Some(("Save & Connect", Action::SaveConnection { connect: true })),
-        ),
-        (
-            Command::FormCancel,
-            "cancel",
-            Some(("Cancel", Action::CloseDialog)),
-        ),
-    ];
+    let test = (
+        Command::FormTest,
+        "test",
+        Some(("Test", Action::TestConnection)),
+    );
+    let save = (
+        Command::FormSave,
+        "save",
+        Some(("Save", Action::SaveConnection { connect: false })),
+    );
+    let cancel = (
+        Command::FormCancel,
+        "cancel",
+        Some(("Cancel", Action::CloseDialog)),
+    );
+    // A Files connection's: Enter chooses among its choices, and its
+    // files are picked.
+    let keys: Vec<FooterKey<'_>> = match super::is_files(form) {
+        true => vec![
+            (Command::FormNextField, "next", None),
+            (Command::FormChoose, "choose", None),
+            (Command::FormPick, "pick", None),
+            test,
+            save,
+            cancel,
+        ],
+        false => vec![
+            (Command::FormNextField, "next", None),
+            test,
+            save,
+            (
+                Command::FormSaveAndConnect,
+                "connect",
+                Some(("Save & Connect", Action::SaveConnection { connect: true })),
+            ),
+            cancel,
+        ],
+    };
     let written: Vec<_> = keys.iter().map(|(command, ..)| key(*command)).collect();
     let names: Vec<_> = keys
         .iter()
@@ -223,7 +259,8 @@ fn terminal_check(
     }
     let laid = laid.layout(ui.ctx());
     let (rect, mut response) = ui.allocate_exact_size(laid.size(), Sense::click());
-    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, name));
+    let enabled = ui.is_enabled();
+    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, on, name));
     laid.paint(ui.painter(), rect.min);
     if response.clicked() {
         *checked = !*checked;
@@ -281,6 +318,11 @@ pub(super) fn terminal_body(
                     }
                 });
             }
+            // The type first: it changes every row after the name.
+            let heading = skin.say("Type");
+            terminal_row_of(ui, &heading, Group::Words.height(), skin, |ui, _| {
+                driver_choice(ui, form, &heading, skin, actions);
+            });
             terminal_row(ui, &skin.say("Name"), skin, |ui, label| {
                 let name = ui
                     .add(input(ui, &mut form.name, role, height, skin))
@@ -289,10 +331,6 @@ pub(super) fn terminal_body(
                     name.request_focus();
                 }
             });
-            let heading = skin.say("Type");
-            terminal_row_of(ui, &heading, Group::Words.height(), skin, |ui, _| {
-                driver_choice(ui, form, &heading, skin, actions);
-            });
             let heading = skin.say("Environment");
             terminal_row_of(ui, &heading, Group::Buttons.height(), skin, |ui, _| {
                 environment_choice(ui, form, &heading, skin);
@@ -302,7 +340,7 @@ pub(super) fn terminal_body(
                     terminal_section(ui, &skin.say("Database"), skin);
                     file_field(ui, form, skin, actions);
                 }
-                Driver::Files => {}
+                Driver::Files => super::files::terminal(ui, form, skin, actions),
                 Driver::Postgres | Driver::MySql => {
                     terminal_section(ui, &skin.say("Server"), skin);
                     terminal_server(ui, form, skin, actions);
@@ -315,8 +353,14 @@ pub(super) fn terminal_body(
             let line = role.row_height(ui.ctx(), look.faces);
             terminal_row_of(ui, &skin.say("Read-only"), line, skin, |ui, _| {
                 let name = gettext(skin.locale, "Open read-only");
-                let promise = skin.say("Block every write from this app");
-                let note = format!("· {}", skin.say("Default for production"));
+                let fixed = super::read_only_fixed(form);
+                let (promise, note) = match fixed {
+                    true => (skin.say("Files open read-only"), String::new()),
+                    false => (
+                        skin.say("Block every write from this app"),
+                        format!("· {}", skin.say("Default for production")),
+                    ),
+                };
                 // The note goes under the promise where one line has no
                 // room for both: no row may widen the dialog.
                 let width = |text: &str| role.width(ui.ctx(), look.faces, text);
@@ -326,7 +370,12 @@ pub(super) fn terminal_body(
                     // The environment's colour, as text can take it.
                     let mark = skin.env_ink(&skin.env);
                     let mut on = form.read_only();
-                    if terminal_check(ui, &mut on, &promise, &name, mark, skin).changed() {
+                    let changed = ui
+                        .add_enabled_ui(!fixed, |ui| {
+                            terminal_check(ui, &mut on, &promise, &name, mark, skin).changed()
+                        })
+                        .inner;
+                    if changed {
                         form.read_only = Some(on);
                     }
                     widgets::label(ui, role, &note, palette.dim, look);
```

```diff
diff --git a/src/ui/file_list.rs b/src/ui/file_list.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/file_list.rs
@@ -0,0 +1,198 @@
+//! A bordered list of files, a row each: the file's icon, its path with
+//! the file's name in the text colour and its folder dimmer, its type, its
+//! size, and × that takes it off the list. A row whose file is not there
+//! says so where its size would be. The Files connection's Source draws it
+//! in both looks; the terminal look marks the keyboard's row as its other
+//! lists do.
+
+use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
+
+use crate::i18n::{Locale, gettext};
+use crate::theme::{Icon, Look, Palette};
+use crate::typography::{Text, TextRole};
+use crate::ui::widgets;
+
+/// One file, as its row says it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Row {
+    /// The path as the app shows it (`~` for the home folder).
+    pub path: String,
+    /// Its format's name (`CSV`), or empty where its name says none.
+    pub kind: String,
+    /// Its size (`1.6 GB`), or empty until it is known.
+    pub size: String,
+    /// The file is not there any more.
+    pub missing: bool,
+}
+
+/// What a frame of the list was asked: a row's × (the file to take off),
+/// or a row clicked (the keyboard's row from then on).
+#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
+pub struct Asked {
+    pub removed: Option<usize>,
+    pub chosen: Option<usize>,
+}
+
+/// The row's height, and the widths of the type, size and × columns.
+fn metrics(look: &Look) -> (f32, f32, f32, f32) {
+    match look.terminal {
+        true => (26.0, 80.0, 80.0, 22.0),
+        false => (34.0, 86.0, 80.0, 30.0),
+    }
+}
+
+/// The list of `rows`, `cursor` the keyboard's row. Announced as a list
+/// named `name`, each row an item by its path, each × a button
+/// "Remove orders.csv".
+pub fn show(
+    ui: &mut Ui,
+    name: &str,
+    rows: &[Row],
+    cursor: Option<usize>,
+    (look, palette, locale): (&Look, &Palette, Locale),
+) -> Asked {
+    let (height, kind_width, size_width, remove_width) = metrics(look);
+    let role = TextRole::pick(look, TextRole::GridCell, TextRole::OBody);
+    let small = TextRole::pick(look, TextRole::Secondary, TextRole::OSecondary);
+    let radius = if look.terminal { 0 } else { 8 };
+    let width = ui.available_width();
+    let mut asked = Asked::default();
+    ui.scope_builder(egui::UiBuilder::new().id_salt(name), |ui| {
+        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
+            node.set_role(egui::accesskit::Role::List);
+            node.set_label(name);
+        });
+        ui.spacing_mut().item_spacing.y = 0.0;
+        let top = ui.cursor().top();
+        for (index, row) in rows.iter().enumerate() {
+            let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
+            ui.ctx().accesskit_node_builder(response.id, |node| {
+                node.set_role(egui::accesskit::Role::ListItem);
+                node.set_label(row.path.as_str());
+                if cursor == Some(index) {
+                    node.set_selected(true);
+                }
+            });
+            if response.clicked() {
+                asked.chosen = Some(index);
+            }
+            let painter = ui.painter().clone();
+            if cursor == Some(index) && look.terminal {
+                painter.rect_filled(rect, CornerRadius::ZERO, palette.selection);
+                painter.rect_filled(
+                    Rect::from_min_size(rect.min, vec2(2.0, rect.height())),
+                    CornerRadius::ZERO,
+                    palette.accent,
+                );
+            } else if response.hovered() {
+                painter.rect_filled(rect, CornerRadius::ZERO, palette.surface_hover);
+            }
+            if index > 0 {
+                widgets::hline(ui, rect.x_range(), rect.top(), palette.surface_hover);
+            }
+            let center = rect.center().y;
+            let faded = |color: egui::Color32| match row.missing {
+                true => color.gamma_multiply(0.6),
+                false => color,
+            };
+            // The icon, then the path, ending before the type.
+            let mut left = rect.left() + if look.terminal { 8.0 } else { 10.0 };
+            if !look.terminal {
+                Icon::File.image(faded(palette.dim), 14.0).paint_at(
+                    ui,
+                    Rect::from_center_size(pos2(left + 7.0, center), vec2(14.0, 14.0)),
+                );
+                left += 20.0;
+            }
+            let right = rect.right() - remove_width - size_width - kind_width - 4.0;
+            let (folder, file) = match row.path.rfind(['/', '\\']) {
+                Some(at) => row.path.split_at(at + 1),
+                None => ("", row.path.as_str()),
+            };
+            let room = (right - left).max(0.0);
+            let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
+            // The file's name whole where it fits; its folder cut first.
+            let file_width = measure(file).min(room);
+            let shown_folder = crate::ui::grid::ellipsize(folder, room - file_width, true, measure);
+            let mut x = left;
+            if !shown_folder.is_empty() {
+                x += widgets::paint_label(
+                    ui,
+                    x,
+                    center,
+                    Text::one(look, role, &shown_folder, faded(palette.dim)),
+                );
+            }
+            let shown_file = crate::ui::grid::ellipsize(file, room - (x - left), false, measure);
+            widgets::paint_label(
+                ui,
+                x,
+                center,
+                Text::one(look, role, &shown_file, faded(palette.text)),
+            );
+            // The type and the size, right-aligned in their columns.
+            let kind_right = right + kind_width;
+            Text::one(look, small, &row.kind, faded(palette.dim))
+                .layout(ui.ctx())
+                .paint_right(&painter, kind_right - 10.0, center);
+            let (size, size_color) = match row.missing {
+                true => (gettext(locale, "missing").into_owned(), palette.danger),
+                false => (row.size.clone(), palette.dim),
+            };
+            Text::one(look, small, &size, size_color)
+                .layout(ui.ctx())
+                .paint_right(&painter, kind_right + size_width - 10.0, center);
+            // ×, which takes the file off the list and never off the disk.
+            let file_name = file.trim();
+            let label = format!("{} {file_name}", gettext(locale, "Remove"));
+            let button = Rect::from_center_size(
+                pos2(rect.right() - remove_width / 2.0 - 2.0, center),
+                vec2(remove_width - 4.0, height - 8.0),
+            );
+            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(button));
+            if widgets::icon_button_sized(
+                &mut child,
+                Icon::X,
+                &label,
+                button.size(),
+                12.0,
+                look,
+                palette,
+            )
+            .clicked()
+            {
+                asked.removed = Some(index);
+            }
+        }
+        let bottom = ui.cursor().top();
+        let outline = Rect::from_min_max(
+            pos2(ui.max_rect().left(), top),
+            pos2(ui.max_rect().left() + width, bottom.max(top + 1.0)),
+        );
+        ui.painter().rect_stroke(
+            outline,
+            CornerRadius::same(radius),
+            Stroke::new(1.0, palette.outline),
+            StrokeKind::Inside,
+        );
+    });
+    asked
+}
+
+/// The table names as small tags in a line that wraps, as the found line
+/// of a Files connection shows them on macOS.
+pub fn tags(ui: &mut Ui, names: &[&str], look: &Look, palette: &Palette) {
+    ui.horizontal_wrapped(|ui| {
+        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
+        for name in names {
+            let laid =
+                Text::one(look, TextRole::MonoSecondary, name, palette.secondary).layout(ui.ctx());
+            let size = vec2(laid.width() + 14.0, 20.0);
+            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
+            ui.painter()
+                .rect_filled(rect, CornerRadius::same(6), palette.surface_hover);
+            laid.paint_left(ui.painter(), rect.left() + 7.0, rect.center().y);
+            widgets::announce(ui, rect, name);
+        }
+    });
+}
```

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -53,6 +53,7 @@ mod env_tests;
 pub mod every_row_prompt;
 #[cfg(test)]
 mod every_row_tests;
+pub mod file_list;
 #[cfg(test)]
 mod files_tests;
 pub mod filter_bar;
@@ -10714,7 +10715,9 @@ value_tags   = true
     #[test]
     fn the_dialog_warns_when_a_remote_password_is_not_protected() {
         use tabletist_db::TlsMode;
-        let mut harness = Harness::new();
+        // Tall enough for the warning under the Security group to be
+        // painted in sight: the Type row has a line of its own.
+        let mut harness = Harness::with_size(egui::vec2(1280.0, 900.0));
         new_connection(&mut harness);
         harness.click("PostgreSQL");
         let set = |harness: &mut Harness, host: &str, tls: TlsMode, ssh: bool| {
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo test --locked --lib files_tests`

Expected: 17 passed in `files_tests`

- [ ] **Step 5: Run the checks** (above). Expected: every suite passes, the app's library (`Running unittests src/lib.rs (…tabletist-…)`) with 3714 tests.

- [ ] **Step 6: Commit**

```bash
git add -A src assets docs/_reference docs/_guide
git commit -m "Make and edit Files connections in the connection dialog"
```

---

### Task 3: The list and the Structure view

The connections list says what a Files connection opens ("Files · 4 files · 2.5 GB", the folder and its tables, "Read-only", "No joins") and a SQLite connection's size and folder; the backend scans every Files and SQLite connection's files at start and after each save. The Structure view shows a Files table's file.

**Files:** 
- Modify: `src/app.rs`
- Modify: `src/backend.rs`
- Modify: `src/model.rs`
- Modify: `src/ui/files_tests.rs`
- Modify: `src/ui/picker.rs`
- Modify: `src/ui/structure.rs`


- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/files_tests.rs b/src/ui/files_tests.rs
--- a/src/ui/files_tests.rs
+++ b/src/ui/files_tests.rs
@@ -680,3 +680,71 @@ fn saved_exports(harness: &mut Harness, source: Source) -> ConnectionId {
     harness.app.connections.upsert(saved);
     id
 }
+
+#[test]
+fn the_list_says_what_a_files_connection_opens() {
+    for look in looks() {
+        // Wide enough that the folder is not cut short.
+        let mut harness = Harness::with_size(egui::vec2(1600.0, 900.0));
+        harness.set_look(look);
+        let id = saved_exports(
+            &mut harness,
+            Source::Folder("/data/bookshop-exports".into()),
+        );
+        harness.app.sources.insert(
+            id,
+            crate::model::SourceFacts::Files(exports("/data/bookshop-exports")),
+        );
+        match look.terminal {
+            true => {
+                let tree = harness.settle();
+                let line = "files · /data/bookshop-exports · 4 files · 2.5 gb";
+                assert!(
+                    crate::testing::labels(&tree)
+                        .iter()
+                        .any(|label| label.starts_with(line)),
+                    "{:?}",
+                    crate::testing::labels(&tree)
+                );
+            }
+            false => {
+                assert!(harness.has("Files · 4 files · 2.5 GB"));
+                assert!(harness.has("/data/bookshop-exports"));
+                assert!(harness.has("orders, customers, events, returns"));
+                assert!(harness.has("Read-only"));
+                assert!(harness.has("No joins"));
+            }
+        }
+    }
+    // A list's first files, then their folder; and a SQLite file's size,
+    // its folder over its name.
+    let mut harness = Harness::with_size(egui::vec2(1600.0, 900.0));
+    harness.set_look(Look::macos());
+    saved_exports(
+        &mut harness,
+        Source::Files(vec![
+            "/data/bookshop-exports/orders.csv".into(),
+            "/data/bookshop-exports/customers.parquet".into(),
+        ]),
+    );
+    let sqlite = SavedConnection {
+        id: ConnectionId::new(),
+        name: "Bookshop on disk".into(),
+        environment: Environment::Local,
+        read_only: None,
+        password: PasswordMode::None,
+        ssh_secret: PasswordMode::None,
+        spec: ConnectSpec::sqlite("/code/bookshop/storage/development.sqlite3"),
+        agents: Default::default(),
+    };
+    let id = sqlite.id.clone();
+    harness.app.connections.upsert(sqlite);
+    harness
+        .app
+        .sources
+        .insert(id, crate::model::SourceFacts::File { size: 12_400_000 });
+    assert!(harness.has("orders.csv, customers.parquet"));
+    assert!(harness.has("/data/bookshop-exports"));
+    assert!(harness.has("SQLite · 12.4 MB"));
+    assert!(harness.has("development.sqlite3"));
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib files_tests`

Expected: it does not compile:

```text
error[E0433]: cannot find `SourceFacts` in `model`
   --> src/ui/files_tests.rs:696:27
error[E0433]: cannot find `SourceFacts` in `model`
   --> src/ui/files_tests.rs:745:35
error[E0609]: no field `sources` on type `app::App`
   --> src/ui/files_tests.rs:694:21
error[E0609]: no field `sources` on type `app::App`
   --> src/ui/files_tests.rs:744:10
```

The rows' text has unit tests of its own in `picker.rs`'s tests module (`file_row_text`, `terminal_files_line`), which come with their code in Step 3.

- [ ] **Step 3: Write the rows, the scan and the file's part**

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -144,6 +144,11 @@ pub struct App {
     pub actions: Vec<Action>,
     pub backend: Backend,
     pub connections: SavedConnections,
+    /// What the saved connections' files are on disk, for the list's rows,
+    /// as the backend last read them.
+    pub sources: HashMap<crate::connections::ConnectionId, crate::model::SourceFacts>,
+    /// The read of `sources` in flight: an older one's answer is dropped.
+    pub sources_request: Option<RequestId>,
     /// The label columns chosen for the tables foreign keys point at, from
     /// labels.json. Read through `App::label_of` only.
     pub labels: crate::labels::Labels,
@@ -399,6 +404,8 @@ impl App {
             actions: Vec::new(),
             backend,
             connections,
+            sources: HashMap::new(),
+            sources_request: None,
             labels,
             table_lists,
             table_lists_unread,
@@ -480,6 +487,7 @@ impl App {
         // What the files let agents do against what was last shown: the
         // connections are read now alone, the settings at every edit too.
         app.check_agents();
+        app.read_sources();
         if adopted {
             app.save_labels();
         }
@@ -4275,6 +4283,12 @@ impl App {
                     form.ssh_hosts = hosts;
                 }
             }
+            Event::SourcesRead { request, facts } => {
+                if self.sources_request == Some(request) {
+                    self.sources_request = None;
+                    self.sources = facts.into_iter().collect();
+                }
+            }
             Event::FilePicked { request, path } => {
                 if let Some(Dialog::Connection(form)) = &mut self.dialog
                     && form.pick_request == Some(request)
@@ -5268,12 +5282,35 @@ impl App {
             path: self.dirs.connections_file(),
             file: StateFile::Connections(self.connections.clone()),
         });
+        // The list's rows say what a connection's files are.
+        self.read_sources();
         // Every change to a connection is saved: what agents may use
         // changes with it. One deleted is held no more.
         self.check_agents();
         self.send_agents_policy();
     }
 
+    /// Has the backend read what every saved connection's files are on
+    /// disk, for the connections list.
+    fn read_sources(&mut self) {
+        let specs: Vec<_> = self
+            .connections
+            .connections
+            .iter()
+            .filter(|saved| match saved.spec.driver {
+                Driver::Files | Driver::Sqlite => true,
+                Driver::Postgres | Driver::MySql => false,
+            })
+            .map(|saved| (saved.id.clone(), saved.spec.clone()))
+            .collect();
+        if specs.is_empty() {
+            return;
+        }
+        let request = RequestId(self.next_id());
+        self.sources_request = Some(request);
+        self.backend.read_sources(request, specs);
+    }
+
     /// Takes the agent server on: tells the backend what agents may use,
     /// and starts the server where the settings say. From then on the
     /// settings start and stop it. Called once, as the window opens.
```

```diff
diff --git a/src/backend.rs b/src/backend.rs
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -398,6 +398,13 @@ pub enum Event {
         request: RequestId,
         hosts: Vec<tabletist_db::ssh_config::ConfigHost>,
     },
+    /// What the saved connections' files are on disk, for the list's rows:
+    /// those of a Files connection, a SQLite file's size. A connection
+    /// whose files cannot be read is left out.
+    SourcesRead {
+        request: RequestId,
+        facts: Vec<(crate::connections::ConnectionId, crate::model::SourceFacts)>,
+    },
     SecretLoaded {
         request: RequestId,
         result: Result<Option<SecretString>, String>,
@@ -844,6 +851,42 @@ impl Backend {
         });
     }
 
+    /// Reads what the saved connections' files are on disk, off the UI
+    /// thread: each Files connection's Source, each SQLite file's size.
+    pub fn read_sources(
+        &mut self,
+        request: RequestId,
+        specs: Vec<(crate::connections::ConnectionId, ConnectSpec)>,
+    ) {
+        let Some(runtime) = &self.runtime else {
+            return;
+        };
+        let outbox = self.outbox.clone();
+        runtime.spawn_blocking(move || {
+            let facts = specs
+                .into_iter()
+                .filter_map(|(id, spec)| {
+                    let facts = match spec.driver {
+                        tabletist_db::Driver::Files => {
+                            let source = spec.files.as_ref()?;
+                            crate::model::SourceFacts::Files(
+                                tabletist_db::files::scan(source).ok()?,
+                            )
+                        }
+                        tabletist_db::Driver::Sqlite => crate::model::SourceFacts::File {
+                            size: std::fs::metadata(spec.sqlite_path.as_ref()?).ok()?.len(),
+                        },
+                        tabletist_db::Driver::Postgres | tabletist_db::Driver::MySql => {
+                            return None;
+                        }
+                    };
+                    Some((id, facts))
+                })
+                .collect();
+            outbox.emit(Event::SourcesRead { request, facts });
+        });
+    }
+
     /// Waits up to `timeout` for the saves sent so far to reach the disk,
     /// so quitting right after a change keeps it.
     pub fn flush(&mut self, timeout: std::time::Duration) {
```

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -2186,6 +2186,14 @@ impl Default for ConnectionForm {
     }
 }
 
+/// What a connection's files are on disk, for the connections list: a
+/// Files connection's files, or a SQLite file's size.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum SourceFacts {
+    Files(tabletist_db::files::Scan),
+    File { size: u64 },
+}
+
 impl ConnectionForm {
     /// Set aside while the Connections list asks whether to delete its
     /// connection: an answer to what it waits for would find no form to
```

```diff
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -443,7 +443,8 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
             "{} {keyring}",
             gettext(
                 locale,
-                "Add a PostgreSQL, MySQL or SQLite database to start browsing."
+                "Add a PostgreSQL, MySQL or SQLite database, or a folder of data files, to \
+                 start browsing."
             )
         );
         let (title, text) = (
@@ -504,7 +505,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
                             terminal_row(
                                 ui,
                                 connection,
-                                &app.connections,
+                                (&app.connections, app.sources.get(&connection.id)),
                                 selected.as_ref(),
                                 Opening {
                                     tab,
@@ -536,7 +537,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
                                 card.shrink(1.0),
                                 index,
                                 connection,
-                                &app.connections,
+                                (&app.connections, app.sources.get(&connection.id)),
                                 selected.as_ref(),
                                 Opening {
                                     tab,
@@ -688,6 +689,106 @@ pub(crate) fn target(connection: &SavedConnection) -> (String, String) {
     }
 }
 
+/// What a macOS row says of a connection that keeps files, where a
+/// server's says its user, host and security: the line under the name,
+/// the middle column's two lines, and for a Files connection what goes
+/// where TLS and SSH go.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub(crate) struct FileRowText {
+    pub second: String,
+    pub middle: (String, String),
+    pub right: Option<(&'static str, &'static str)>,
+}
+
+/// [`FileRowText`] of `connection`, with what a scan of its files found
+/// (`facts`, which may be on its way). `None` for a server's.
+pub(crate) fn file_row_text(
+    connection: &SavedConnection,
+    facts: Option<&crate::model::SourceFacts>,
+) -> Option<FileRowText> {
+    use crate::model::SourceFacts;
+    use tabletist_db::Driver;
+    use tabletist_db::files::{Source, shown, size_text};
+    let spec = &connection.spec;
+    match spec.driver {
+        Driver::Postgres | Driver::MySql => None,
+        Driver::Sqlite => {
+            let path = spec.sqlite_path.as_deref()?;
+            let second = match facts {
+                Some(SourceFacts::File { size }) => format!("SQLite · {}", size_text(*size)),
+                Some(SourceFacts::Files(_)) | None => "SQLite".to_owned(),
+            };
+            let folder = path
+                .parent()
+                .map(|folder| format!("{}{}", shown(folder), std::path::MAIN_SEPARATOR))
+                .unwrap_or_default();
+            let name = path
+                .file_name()
+                .map(|name| name.to_string_lossy().into_owned())
+                .unwrap_or_default();
+            Some(FileRowText {
+                second,
+                middle: (folder, name),
+                right: None,
+            })
+        }
+        Driver::Files => {
+            let source = spec.files.as_ref()?;
+            let scan = match facts {
+                Some(SourceFacts::Files(scan)) => Some(scan),
+                Some(SourceFacts::File { .. }) | None => None,
+            };
+            let second = match scan {
+                Some(scan) => format!(
+                    "Files · {} · {}",
+                    match scan.files.len() {
+                        1 => "1 file".to_owned(),
+                        count => format!("{count} files"),
+                    },
+                    size_text(scan.total_size())
+                ),
+                None => "Files".to_owned(),
+            };
+            let names = scan.map(|scan| scan.names().join(", ")).unwrap_or_default();
+            let middle = match source {
+                Source::Folder(folder) => (shown(folder), names),
+                Source::Files(_) => (
+                    source.summary(),
+                    source.folder().map(shown).unwrap_or_default(),
+                ),
+            };
+            Some(FileRowText {
+                second,
+                middle,
+                // Editing files is not built: every one reads only.
+                right: Some(("Read-only", "No joins")),
+            })
+        }
+    }
+}
+
+/// The terminal look's line under a Files connection's name: `files ·
+/// ~/Data/bookshop-exports · 4 files · 2.5 gb`. `None` for any other.
+pub(crate) fn terminal_files_line(
+    connection: &SavedConnection,
+    facts: Option<&crate::model::SourceFacts>,
+) -> Option<String> {
+    use tabletist_db::Driver;
+    match connection.spec.driver {
+        Driver::Files => {}
+        Driver::Postgres | Driver::MySql | Driver::Sqlite => return None,
+    }
+    let text = file_row_text(connection, facts)?;
+    let source = match &connection.spec.files {
+        Some(tabletist_db::files::Source::Folder(folder)) => tabletist_db::files::shown(folder),
+        Some(source) => source.summary(),
+        None => String::new(),
+    };
+    let counted = text.second.strip_prefix("Files").unwrap_or_default();
+    // The look's case for its words, and the path as it is.
+    Some(format!("files · {source}{}", counted.to_lowercase()))
+}
+
 /// The row's own response, answering a click by selecting and a double
 /// click, Enter or a screen reader by choosing it: an open connection
 /// shows, another connects.
@@ -967,7 +1068,10 @@ fn mac_row(
     card: Rect,
     index: usize,
     connection: &SavedConnection,
-    store: &crate::connections::SavedConnections,
+    (store, facts): (
+        &crate::connections::SavedConnections,
+        Option<&crate::model::SourceFacts>,
+    ),
     selected: Option<&ConnectionId>,
     opening: Opening,
     skin: RowSkin<'_>,
@@ -1089,11 +1193,12 @@ fn mac_row(
         laid.paint_left(ui.painter(), tag.left() + 7.0, top);
         widgets::announce(ui, tag, &text);
     }
+    let files = file_row_text(connection, facts);
     let user = &connection.spec.user;
-    let second = if user.is_empty() {
-        connection.spec.driver.label().to_owned()
-    } else {
-        format!("{} · {user}", connection.spec.driver.label())
+    let second = match &files {
+        Some(files) => files.second.clone(),
+        None if user.is_empty() => connection.spec.driver.label().to_owned(),
+        None => format!("{} · {user}", connection.spec.driver.label()),
     };
     let small = TextRole::Secondary;
     widgets::paint_label(
@@ -1102,7 +1207,10 @@ fn mac_row(
         bottom,
         Text::one(look, small, &second, palette.dim),
     );
-    let (host, database) = target(connection);
+    let (host, database) = match &files {
+        Some(files) => files.middle.clone(),
+        None => target(connection),
+    };
     let host_role = TextRole::GridCell;
     let shown = crate::ui::grid::ellipsize(&host, host_width - 24.0, false, |text| {
         host_role.width(ui.ctx(), look.faces, text)
@@ -1125,28 +1233,47 @@ fn mac_row(
             Text::one(look, database_role, &shown, palette.dim),
         );
     }
-    let (tls, verified) = tls_label(connection);
-    let tls_color = match verified {
-        Some(true) => palette.success,
-        Some(false) => palette.warning,
-        None => palette.dim,
-    };
-    Icon::Lock.image(tls_color, 11.0).paint_at(
-        ui,
-        Rect::from_center_size(pos2(security_x + 5.5, top), vec2(11.0, 11.0)),
-    );
-    widgets::paint_label(
-        ui,
-        security_x + 16.0,
-        top,
-        Text::one(look, small, &tls, tls_color),
-    );
-    widgets::paint_label(
-        ui,
-        security_x,
-        bottom,
-        Text::one(look, small, &ssh_label(connection), palette.dim),
-    );
+    // A Files connection says whether it writes and what it joins where a
+    // server says its TLS and its tunnel.
+    if let Some((writes, joins)) = files.as_ref().and_then(|files| files.right) {
+        let writes = gettext(locale, writes);
+        widgets::paint_label(
+            ui,
+            security_x,
+            top,
+            Text::one(look, small, &writes, palette.secondary),
+        );
+        let joins = gettext(locale, joins);
+        widgets::paint_label(
+            ui,
+            security_x,
+            bottom,
+            Text::one(look, small, &joins, palette.dim),
+        );
+    } else {
+        let (tls, verified) = tls_label(connection);
+        let tls_color = match verified {
+            Some(true) => palette.success,
+            Some(false) => palette.warning,
+            None => palette.dim,
+        };
+        Icon::Lock.image(tls_color, 11.0).paint_at(
+            ui,
+            Rect::from_center_size(pos2(security_x + 5.5, top), vec2(11.0, 11.0)),
+        );
+        widgets::paint_label(
+            ui,
+            security_x + 16.0,
+            top,
+            Text::one(look, small, &tls, tls_color),
+        );
+        widgets::paint_label(
+            ui,
+            security_x,
+            bottom,
+            Text::one(look, small, &ssh_label(connection), palette.dim),
+        );
+    }
     // What agents may do: the word over the level, dimmed when Off.
     if let Some(access) = opening.agents.filter(|_| agents_width > 0.0) {
         let level = agents_level(access, locale);
@@ -1270,7 +1397,10 @@ fn mac_row(
 fn terminal_row(
     ui: &mut egui::Ui,
     connection: &SavedConnection,
-    store: &crate::connections::SavedConnections,
+    (store, facts): (
+        &crate::connections::SavedConnections,
+        Option<&crate::model::SourceFacts>,
+    ),
     selected: Option<&ConnectionId>,
     opening: Opening,
     skin: RowSkin<'_>,
@@ -1373,18 +1503,17 @@ fn terminal_row(
     };
     // "dev@localhost:5433/bookshop_development · tls verify-full · no ssh",
     // the separators kept with their neighbours as the design writes them.
-    widgets::paint_label(
-        ui,
-        text_x,
-        bottom,
-        Text::new(look)
+    // A Files connection's line is its own: what it opens, and how much.
+    let said = match terminal_files_line(connection, facts) {
+        Some(files) => Text::new(look).add(small, &files, palette.dim),
+        None => Text::new(look)
             .add(small, &format!("{line} ·"), palette.dim)
             .space(small, " ")
             .add(small, &tls, tls_color)
             .space(small, " ")
-            .add(small, &format!("· {ssh}"), palette.dim)
-            .add(small, &agents, agents_color),
-    );
+            .add(small, &format!("· {ssh}"), palette.dim),
+    };
+    widgets::paint_label(ui, text_x, bottom, said.add(small, &agents, agents_color));
     let when = crate::connections::when(store.last_used(&connection.id), now).to_lowercase();
     widgets::paint_text_right(
         ui,
@@ -1449,4 +1578,118 @@ mod tests {
         ));
         assert!(connects(opened.connect(&connection)));
     }
+
+    fn saved(spec: tabletist_db::ConnectSpec) -> SavedConnection {
+        SavedConnection {
+            id: ConnectionId::new(),
+            name: "Bookshop exports".into(),
+            environment: crate::env::Environment::Local,
+            read_only: Some(true),
+            password: crate::connections::PasswordMode::None,
+            ssh_secret: crate::connections::PasswordMode::None,
+            spec,
+            agents: Default::default(),
+        }
+    }
+
+    fn scan(folder: &str, names: &[&str]) -> crate::model::SourceFacts {
+        use tabletist_db::files::{Format, Found, Scan};
+        crate::model::SourceFacts::Files(Scan {
+            files: names
+                .iter()
+                .map(|name| Found {
+                    path: format!("{folder}/{name}").into(),
+                    format: Format::of(std::path::Path::new(name)).unwrap(),
+                    size: 625_000_000,
+                    name: name.split('.').next().unwrap_or_default().to_owned(),
+                })
+                .collect(),
+            missing: Vec::new(),
+        })
+    }
+
+    #[test]
+    fn a_files_rows_say_how_many_files_and_what_they_become() {
+        use tabletist_db::files::Source;
+        let folder = saved(tabletist_db::ConnectSpec::files(Source::Folder(
+            "/data/bookshop-exports".into(),
+        )));
+        let facts = scan(
+            "/data/bookshop-exports",
+            &[
+                "orders.csv",
+                "customers.parquet",
+                "events.ndjson",
+                "returns.csv",
+            ],
+        );
+        let text = file_row_text(&folder, Some(&facts)).unwrap();
+        assert_eq!(text.second, "Files · 4 files · 2.5 GB");
+        assert_eq!(
+            text.middle,
+            (
+                "/data/bookshop-exports".to_owned(),
+                "orders, customers, events, returns".to_owned()
+            )
+        );
+        assert_eq!(text.right, Some(("Read-only", "No joins")));
+        assert_eq!(
+            terminal_files_line(&folder, Some(&facts)).as_deref(),
+            Some("files · /data/bookshop-exports · 4 files · 2.5 gb")
+        );
+        // The path keeps its case in the terminal look's line.
+        let cased = saved(tabletist_db::ConnectSpec::files(Source::Folder(
+            "/srv/Data/Bookshop-Exports".into(),
+        )));
+        assert_eq!(
+            terminal_files_line(&cased, None).as_deref(),
+            Some("files · /srv/Data/Bookshop-Exports")
+        );
+        // Before the files are read: the type alone.
+        assert_eq!(file_row_text(&folder, None).unwrap().second, "Files");
+        // A list: its first files, then their folder.
+        let list = saved(tabletist_db::ConnectSpec::files(Source::Files(vec![
+            "/data/bookshop-exports/orders.csv".into(),
+            "/data/bookshop-exports/customers.parquet".into(),
+        ])));
+        let facts = scan(
+            "/data/bookshop-exports",
+            &["orders.csv", "customers.parquet"],
+        );
+        let text = file_row_text(&list, Some(&facts)).unwrap();
+        assert_eq!(text.second, "Files · 2 files · 1.2 GB");
+        assert_eq!(
+            text.middle,
+            (
+                "orders.csv, customers.parquet".to_owned(),
+                "/data/bookshop-exports".to_owned()
+            )
+        );
+    }
+
+    #[test]
+    fn a_sqlite_row_says_its_size_and_its_folder_over_its_file() {
+        let sqlite = saved(tabletist_db::ConnectSpec::sqlite(
+            "/code/bookshop/storage/development.sqlite3",
+        ));
+        let text = file_row_text(
+            &sqlite,
+            Some(&crate::model::SourceFacts::File { size: 12_400_000 }),
+        )
+        .unwrap();
+        assert_eq!(text.second, "SQLite · 12.4 MB");
+        let sep = std::path::MAIN_SEPARATOR;
+        assert_eq!(
+            text.middle,
+            (
+                format!("/code/bookshop/storage{sep}"),
+                "development.sqlite3".to_owned()
+            )
+        );
+        assert_eq!(text.right, None);
+        assert_eq!(terminal_files_line(&sqlite, None), None);
+        // A server's row is its own.
+        let (server, _) = tabletist_db::ConnectSpec::from_url("postgres://me@db/app").unwrap();
+        assert_eq!(file_row_text(&saved(server), None), None);
+    }
 }
```

```diff
diff --git a/src/ui/structure.rs b/src/ui/structure.rs
--- a/src/ui/structure.rs
+++ b/src/ui/structure.rs
@@ -416,6 +416,39 @@ fn quiet(ui: &mut egui::Ui, text: &str, look: &Look, palette: &Palette) {
     widgets::label(ui, widgets::body(look), text, palette.secondary, look);
 }
 
+/// What a Files connection's table is read from, over its columns: the
+/// file's path, its size, its format, and how its reader is told to read
+/// it.
+fn file_part(
+    ui: &mut egui::Ui,
+    file: &tabletist_db::FileFacts,
+    look: &Look,
+    palette: &Palette,
+    locale: crate::i18n::Locale,
+) {
+    heading(ui, &gettext(locale, "File"), look, palette);
+    let mono = crate::ui::grid::data_role(look);
+    let rows = [
+        ("Path", tabletist_db::files::shown(&file.path)),
+        ("Size", tabletist_db::files::size_text(file.size)),
+        ("Format", file.format.label().to_owned()),
+        ("Read with", file.options.clone()),
+    ];
+    egui::Grid::new("structure-file")
+        .spacing([16.0, 6.0])
+        .show(ui, |ui| {
+            for (name, value) in rows {
+                if value.is_empty() {
+                    continue;
+                }
+                quiet(ui, &gettext(locale, name), look, palette);
+                widgets::label(ui, mono, &value, palette.text, look);
+                ui.end_row();
+            }
+        });
+    ui.add_space(16.0);
+}
+
 /// The table's own comment, over everything else of the view: its
 /// author's words, wrapped at `HELP` or the view's width.
 fn table_comment(ui: &mut egui::Ui, comment: &str, look: &Look, palette: &Palette) {
@@ -488,6 +521,9 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
                         if let Some(comment) = comment.filter(|comment| !comment.is_empty()) {
                             table_comment(ui, comment, &look, &palette);
                         }
+                        if let Some(file) = &structure.file {
+                            file_part(ui, file, &look, &palette, locale);
+                        }
                         if let Some(label) = &label {
                             let table = (object.object.name.as_str(), refused.as_deref());
                             asked = label_part(ui, label, table, skin);
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo test --locked --lib -- files_tests picker::tests`

Expected: 18 passed in `files_tests`, 3 in `picker::tests`

- [ ] **Step 5: Run the checks** (above). Expected: every suite passes, the app's library (`Running unittests src/lib.rs (…tabletist-…)`) with 3717 tests.

- [ ] **Step 6: Commit**

```bash
git add -A src
git commit -m "Say what a Files connection opens in the connections list"
```

---

### After Task 3: the pull request

Open one pull request for the run. Its description, from the prompt's list:

- A summary, and that this is run 1 of 4.
- The readers and DuckDB's options per format: `read_csv` with `header = true, auto_detect = true, sample_size = 20000` (and `delim = '\t'` for TSV); `read_parquet`; `read_json` with `format = 'array'` or `'newline_delimited'`; `read_xlsx` with `header = true`; `ATTACH … (READ_ONLY)`.
- In the place of "how editing writes files back" (run 4): the fence, `allowed_paths`, `enable_external_access = false`, `lock_configuration = true`, a read-only transaction around every read, the refused statements (an `EXPLAIN` of one included), and every session read-only whatever it is asked.
- Attach and credentials: run 3.
- The `connections.toml` format, version 7.
- The new components: `file_list::show`, `file_list::tags`, `Choice::off`.
- The open questions and the defaults chosen: extensions (two bundled, Excel's installed on first use); Excel writing (read-only); subfolders (not included, no checkbox yet); attached production (read-only, run 3); keys (`a`, `x`, `ctrl+o` checked against the keymap's clash tests; `t`, `e`, `s` and `R` come with run 2's banner and overrides).
- Every departure in "What the design asks, and what is built", the new words above, what changes for the other types, the known gaps, and that nothing was built or run on macOS or Windows before CI.
- No screenshot unless blurred.
