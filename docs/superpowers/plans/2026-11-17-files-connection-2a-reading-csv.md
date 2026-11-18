# Files connections, run 2a: Reading CSV, Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Files connection reads its CSV and TSV files as its Reading CSV section says (header row, delimiter, column types, and overrides of one file), saves that in `connections.toml`, and shows it in the connection dialog as a section that opens and closes; on a Files form the Agents part becomes such a section too.

**Architecture:** `tabletist_db::files::Reading` travels as `ConnectSpec.csv`. `duck.rs` builds each CSV's and TSV's `read_csv(…)` call from it: the three options, then the file's overrides (a column read as text in `types = {…}`; a column's values read as empty by reading it as text and casting it back, in the view's `SELECT * REPLACE (…)`, to the type a `DESCRIBE` of the call with those values as DuckDB's `nullstr` found; a skipped file makes no view). `connections.toml` version 8 keeps it in `[connections.csv]` and `[[connections.override]]`. The dialog's `ConnectionForm` keeps a copy, changed through `Action::SetCsv`; a shared component, `ui::disclosure`, draws the closed row and the open box in both looks.

**Tech Stack:** Rust, egui (headless tests through `crate::testing::Harness`), DuckDB 2.0 (`duckdb` crate 1.20000.1, bundled), TOML.

**Spec:** `docs/superpowers/specs/2026-10-30-files-connection-design.md`, sections "Run 2 split" and "Run 2a: Reading CSV".

**Base:** `main` at `628ab8b4`. Branch `claude/files-connection-2-csv`.

---

## What the design asks, what is built, why

Approved by Igor on 2026-11-17 ("Yes, all seven"):

| The prompt or a board asks | Built | Why |
| --- | --- | --- |
| `[connections."Bookshop exports".csv]` keyed by name | `[connections.csv]` and `[[connections.override]]` under the connection's own `[[connections]]` entry, version 8 only when needed | Run 1's form of the file; an older Tabletist refuses the file rather than dropping the settings |
| Read 'N/A' as empty for one column | That column read as text and cast back to its detected type with `NULLIF`, so it stays a date | DuckDB's `nullstr` is the whole file's, every column |
| Overrides made from Test, with Reset | The list, Reset, storage and reading; nothing in 2a makes one (2b's fixes will) | 2b needs the storage and reading first |
| Delimiter for every CSV and TSV | For `.csv` files; a `.tsv` is always tab | A TSV is tab-separated by its name |
| Header row: Detect | Detect leaves `header` out of the call; run 1's fixed `header = true` goes | The prompt's default |
| Collapsed Reading CSV, Join, Agents | Reading CSV, and Agents on a Files form, closed each time the dialog opens; Join not shown (run 3); a request to show Agents opens its row | Run 1 put the collapsed row here |
| The closed row's summary | Follows each choice in each board's words, then "· 1 override" | The boards draw only the defaults |

## Files

| File | Task | What changes |
| --- | --- | --- |
| `crates/tabletist-db/src/files.rs` | 1 | `Reading`, `Header`, `Delimiter`, `Types`, `Override`, `Change`, `SAMPLE_ROWS`; `file_name` public |
| `crates/tabletist-db/src/duck.rs` | 1 | Each view's call from the reading; skipped files; `Opened.options` for the Structure view |
| `crates/tabletist-db/src/spec.rs`, `lib.rs` | 1 | `ConnectSpec.csv`, passed to `duck::Conn::open` |
| `crates/tabletist-db/tests/files_csv.rs` (new), `tests/files.rs` | 1 | The calls and the rows each reading gives |
| `crates/tabletist-core/src/connections/file.rs` | 1, 2 | Version 8: `CsvTable`, `OverrideTable`, the writer |
| `docs/_reference/settings-and-files.md` | 2 | The two tables |
| `crates/tabletist-core/src/model.rs` | 1, 3 | `CsvChange`, `Action::SetCsv`, the form's `files_csv`, `csv_open`, `agents_open` |
| `src/app.rs` | 3 | `Action::SetCsv` applied |
| `src/ui/disclosure.rs` (new) | 4 | The shared section that opens and closes |
| `src/ui/connect_dialog/reading.rs` (new) | 4 | The Reading CSV section, both looks |
| `src/ui/connect_dialog/agents.rs`, `files.rs`, `mod.rs`, `sheet.rs`, `terminal.rs`, `src/ui/mod.rs` | 4 | Agents closed on Files; the section placed; `R`; the found line without skipped files |
| `crates/tabletist-core/src/keymap.rs` | 4 | `FormResetOverride`, `R` in the connection form |
| `src/ui/reading_csv_tests.rs` (new), `src/ui/files_tests.rs` | 4 | The dialog's behaviour, both looks |
| `docs/_reference/connections.md` | 4 | Reading CSV and the closed Agents section |

Every command below runs from the worktree with `CARGO_TARGET_DIR=$PWD/target` and `~/.cargo/bin/cargo`. The full check at each commit: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (with `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` set where the test servers run), and `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`.

---

### Task 1: The engine reads CSV as the connection says

**Files:**
- Modify: `crates/tabletist-db/src/files.rs`, `crates/tabletist-db/src/duck.rs`, `crates/tabletist-db/src/spec.rs`, `crates/tabletist-db/src/lib.rs`
- Modify: `crates/tabletist-core/src/model.rs` (one line), `crates/tabletist-core/src/connections/file.rs` (one line): the two places that write a `ConnectSpec` field by field take the default reading
- Create: `crates/tabletist-db/tests/files_csv.rs`
- Modify: `crates/tabletist-db/tests/files.rs` (run 1's DDL text loses `header = true`)

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/crates/tabletist-db/src/spec.rs b/crates/tabletist-db/src/spec.rs
--- a/crates/tabletist-db/src/spec.rs
+++ b/crates/tabletist-db/src/spec.rs
@@ -406,6 +406,33 @@
     }
 
     #[test]
+    fn a_files_spec_keeps_how_it_reads_csv_and_says_nothing_of_the_defaults() {
+        use crate::files::{Change, Delimiter, Override, Reading, Types};
+        let mut spec = ConnectSpec::files(crate::files::Source::Folder("/data/bookshop".into()));
+        let plain = serde_json::to_string(&spec).unwrap();
+        assert!(!plain.contains("csv"), "{plain}");
+        spec.csv = Reading {
+            delimiter: Delimiter::Semicolon,
+            types: Types::All,
+            overrides: vec![Override {
+                file: "returns.csv".into(),
+                change: Change::Empty {
+                    column: "returned_at".into(),
+                    values: vec!["N/A".into()],
+                },
+            }],
+            ..Reading::default()
+        };
+        let json = serde_json::to_string(&spec).unwrap();
+        assert_eq!(serde_json::from_str::<ConnectSpec>(&json).unwrap(), spec);
+        // A spec saved before Reading CSV reads with the defaults.
+        assert_eq!(
+            serde_json::from_str::<ConnectSpec>(&plain).unwrap().csv,
+            Reading::default()
+        );
+    }
+
+    #[test]
     fn postgres_urls_parse_with_decoded_parts() {
         let (spec, secrets) = ConnectSpec::from_url(
             "postgres://app%40corp:p%40ss%2Fw0rd@db.example.com:6543/my%20db?sslmode=verify-full&sslrootcert=/etc/ca.pem",
diff --git a/crates/tabletist-db/tests/files.rs b/crates/tabletist-db/tests/files.rs
--- a/crates/tabletist-db/tests/files.rs
+++ b/crates/tabletist-db/tests/files.rs
@@ -231,8 +231,8 @@
     assert_eq!(
         text,
         format!(
-            "CREATE VIEW orders AS SELECT * FROM read_csv('{}', header = true, \
-             auto_detect = true, sample_size = 20000);",
+            "CREATE VIEW orders AS SELECT * FROM read_csv('{}', auto_detect = true, \
+             sample_size = 20000);",
             path.display()
         )
     );
diff --git a/crates/tabletist-db/tests/files_csv.rs b/crates/tabletist-db/tests/files_csv.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-db/tests/files_csv.rs
@@ -0,0 +1,385 @@
+//! How a Files connection reads its CSV and TSV files: the connection's
+//! Reading CSV (header, delimiter, column types) and a file's overrides,
+//! as the call each view is made with says them and as the rows read.
+
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
+use std::path::Path;
+
+use tabletist_db::files::{Change, Delimiter, Header, Override, Reading, Source, Types};
+use tabletist_db::{
+    ConnectSpec, Connection, Dialect, ObjectRef, ScriptMode, Secrets, StatementOutcome, StopFlag,
+    Value, sql,
+};
+
+async fn open(dir: &Path, reading: Reading) -> Result<Connection, tabletist_db::Error> {
+    let mut spec = ConnectSpec::files(Source::Folder(dir.to_owned()));
+    spec.csv = reading;
+    Connection::connect(&spec, &Secrets::default()).await
+}
+
+/// The Bookshop's exports in a folder of their own.
+fn exports() -> tempfile::TempDir {
+    let dir = tempfile::tempdir().unwrap();
+    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
+    dir
+}
+
+/// A folder holding only `returns.csv`: 30,000 returns, then one whose
+/// `returned_at` is `N/A`, past the rows a reader samples for types.
+fn late_returns() -> tempfile::TempDir {
+    let dir = tempfile::tempdir().unwrap();
+    let mut text = String::from("order_id,returned_at,reason\n");
+    for order in 1..=30_000 {
+        let day = order % 28 + 1;
+        text.push_str(&format!("{order},2026-02-{day:02},damaged\n"));
+    }
+    text.push_str("30001,N/A,lost in the post\n");
+    std::fs::write(dir.path().join("returns.csv"), text).unwrap();
+    dir
+}
+
+/// The statement a table's view was made with, without the file's path.
+async fn made(connection: &Connection, table: &str, dir: &Path) -> String {
+    let ddl = connection
+        .ddl(&ObjectRef::new("main", table))
+        .await
+        .unwrap();
+    ddl.text()
+        .replace(&dir.display().to_string(), "<dir>")
+        .trim_end_matches(';')
+        .to_owned()
+}
+
+/// The rows of a script of one statement, or its error.
+async fn read(connection: &Connection, text: &str) -> Result<Vec<Vec<Value>>, String> {
+    let statements = sql::statements(Dialect::DuckDb, text);
+    let outcome = connection
+        .run_script(&statements, 100, ScriptMode::ReadOnly, &StopFlag::new())
+        .await
+        .map_err(|error| error.to_string())?;
+    match outcome.results.into_iter().next().unwrap().outcome {
+        StatementOutcome::Rows { rows, .. } => Ok(rows),
+        StatementOutcome::Error { error, .. } => Err(error.to_string()),
+        other => panic!("{text}: {other:?}"),
+    }
+}
+
+fn text_of(value: &Value) -> String {
+    match value {
+        Value::Text(text) => text.to_string(),
+        Value::Int(number) => number.to_string(),
+        other => format!("{other:?}"),
+    }
+}
+
+fn change(file: &str, change: Change) -> Override {
+    Override {
+        file: file.to_owned(),
+        change,
+    }
+}
+
+#[tokio::test]
+async fn each_option_changes_the_call_a_csv_and_a_tsv_are_made_with() {
+    let dir = exports();
+    let cases = [
+        (
+            Reading::default(),
+            "read_csv('<dir>/orders.csv', auto_detect = true, sample_size = 20000)",
+            "read_csv('<dir>/authors.tsv', auto_detect = true, sample_size = 20000, delim = '\\t')",
+        ),
+        (
+            Reading {
+                header: Header::Yes,
+                ..Reading::default()
+            },
+            "read_csv('<dir>/orders.csv', header = true, auto_detect = true, sample_size = 20000)",
+            "read_csv('<dir>/authors.tsv', header = true, auto_detect = true, sample_size = 20000, \
+             delim = '\\t')",
+        ),
+        (
+            Reading {
+                header: Header::No,
+                types: Types::All,
+                ..Reading::default()
+            },
+            "read_csv('<dir>/orders.csv', header = false, auto_detect = true, sample_size = -1)",
+            "read_csv('<dir>/authors.tsv', header = false, auto_detect = true, sample_size = -1, \
+             delim = '\\t')",
+        ),
+        // A TSV keeps its tab whatever the CSVs are parted by.
+        (
+            Reading {
+                delimiter: Delimiter::Semicolon,
+                types: Types::Text,
+                ..Reading::default()
+            },
+            "read_csv('<dir>/orders.csv', auto_detect = true, all_varchar = true, delim = ';')",
+            "read_csv('<dir>/authors.tsv', auto_detect = true, all_varchar = true, delim = '\\t')",
+        ),
+        (
+            Reading {
+                delimiter: Delimiter::Tab,
+                ..Reading::default()
+            },
+            "read_csv('<dir>/orders.csv', auto_detect = true, sample_size = 20000, delim = '\\t')",
+            "read_csv('<dir>/authors.tsv', auto_detect = true, sample_size = 20000, delim = '\\t')",
+        ),
+        (
+            Reading {
+                delimiter: Delimiter::Pipe,
+                ..Reading::default()
+            },
+            "read_csv('<dir>/orders.csv', auto_detect = true, sample_size = 20000, delim = '|')",
+            "read_csv('<dir>/authors.tsv', auto_detect = true, sample_size = 20000, delim = '\\t')",
+        ),
+    ];
+    for (reading, orders, authors) in cases {
+        // A delimiter that is not the file's reads one column, never fails.
+        let connection = open(dir.path(), reading.clone()).await.unwrap();
+        assert_eq!(
+            made(&connection, "orders", dir.path()).await,
+            format!("CREATE VIEW orders AS SELECT * FROM {orders}"),
+            "{reading:?}"
+        );
+        assert_eq!(
+            made(&connection, "authors", dir.path()).await,
+            format!("CREATE VIEW authors AS SELECT * FROM {authors}"),
+            "{reading:?}"
+        );
+        // The other formats read as they did.
+        assert!(
+            made(&connection, "customers", dir.path())
+                .await
+                .ends_with("read_parquet('<dir>/customers.parquet')")
+        );
+    }
+}
+
+#[tokio::test]
+async fn a_value_past_the_sample_fails_the_default_reading_and_each_way_reads_it() {
+    let dir = late_returns();
+    let count = "SELECT typeof(returned_at), count(*), count(returned_at) FROM returns \
+                 GROUP BY 1";
+    let read_as = |rows: Vec<Vec<Value>>| -> Vec<String> { rows[0].iter().map(text_of).collect() };
+
+    let connection = open(dir.path(), Reading::default()).await.unwrap();
+    let error = read(&connection, count).await.unwrap_err();
+    assert!(error.contains("N/A"), "{error}");
+
+    let all = Reading {
+        types: Types::All,
+        ..Reading::default()
+    };
+    let connection = open(dir.path(), all).await.unwrap();
+    assert_eq!(
+        read_as(read(&connection, count).await.unwrap()),
+        ["VARCHAR", "30001", "30001"]
+    );
+
+    let text = Reading {
+        types: Types::Text,
+        ..Reading::default()
+    };
+    let connection = open(dir.path(), text).await.unwrap();
+    assert_eq!(
+        read_as(read(&connection, count).await.unwrap()),
+        ["VARCHAR", "30001", "30001"]
+    );
+
+    let as_text = Reading {
+        overrides: vec![change(
+            "returns.csv",
+            Change::Text {
+                column: "returned_at".into(),
+            },
+        )],
+        ..Reading::default()
+    };
+    let connection = open(dir.path(), as_text).await.unwrap();
+    assert_eq!(
+        read_as(read(&connection, count).await.unwrap()),
+        ["VARCHAR", "30001", "30001"]
+    );
+    assert_eq!(
+        made(&connection, "returns", dir.path()).await,
+        "CREATE VIEW returns AS SELECT * FROM read_csv('<dir>/returns.csv', auto_detect = true, \
+         sample_size = 20000, types = {'returned_at': 'VARCHAR'})"
+    );
+
+    // Empty: the column stays a date, and its N/A is NULL.
+    let as_empty = Reading {
+        overrides: vec![change(
+            "returns.csv",
+            Change::Empty {
+                column: "returned_at".into(),
+                values: vec!["N/A".into()],
+            },
+        )],
+        ..Reading::default()
+    };
+    let connection = open(dir.path(), as_empty).await.unwrap();
+    assert_eq!(
+        read_as(read(&connection, count).await.unwrap()),
+        ["DATE", "30001", "30000"]
+    );
+    assert_eq!(
+        made(&connection, "returns", dir.path()).await,
+        "CREATE VIEW returns AS SELECT * REPLACE (CAST(NULLIF(returned_at, 'N/A') AS DATE) AS \
+         returned_at) FROM read_csv('<dir>/returns.csv', auto_detect = true, sample_size = 20000, \
+         types = {'returned_at': 'VARCHAR'})"
+    );
+
+    let skipped = Reading {
+        overrides: vec![change("returns.csv", Change::Skip)],
+        ..Reading::default()
+    };
+    let connection = open(dir.path(), skipped).await.unwrap();
+    let error = read(&connection, count).await.unwrap_err();
+    assert!(error.contains("returns"), "{error}");
+    assert_eq!(connection.read_files().await.unwrap().unwrap().files, 0);
+}
+
+#[tokio::test]
+async fn an_override_counts_only_for_its_own_file_and_a_column_it_has() {
+    let dir = exports();
+    let reading = Reading {
+        overrides: vec![
+            // No such column in returns.csv: it does nothing.
+            change(
+                "returns.csv",
+                Change::Text {
+                    column: "shipped_at".into(),
+                },
+            ),
+            // A file the connection does not open.
+            change("archive.csv", Change::Skip),
+            // Read as text and as empty: text wins, no cast back.
+            change(
+                "returns.csv",
+                Change::Text {
+                    column: "reason".into(),
+                },
+            ),
+            change(
+                "returns.csv",
+                Change::Empty {
+                    column: "reason".into(),
+                    values: vec!["damaged".into(), "lost".into()],
+                },
+            ),
+            change(
+                "returns.csv",
+                Change::Empty {
+                    column: "returned_at".into(),
+                    values: vec!["N/A".into(), "-".into()],
+                },
+            ),
+        ],
+        ..Reading::default()
+    };
+    let connection = open(dir.path(), reading).await.unwrap();
+    assert_eq!(
+        made(&connection, "returns", dir.path()).await,
+        "CREATE VIEW returns AS SELECT * REPLACE (CAST(NULLIF(NULLIF(returned_at, 'N/A'), '-') \
+         AS DATE) AS returned_at) FROM read_csv('<dir>/returns.csv', auto_detect = true, \
+         sample_size = 20000, types = {'reason': 'VARCHAR', 'returned_at': 'VARCHAR'})"
+    );
+    // orders.csv is read as it was.
+    assert_eq!(
+        made(&connection, "orders", dir.path()).await,
+        "CREATE VIEW orders AS SELECT * FROM read_csv('<dir>/orders.csv', auto_detect = true, \
+         sample_size = 20000)"
+    );
+    // The Structure view says the call's own options.
+    let structure = connection
+        .describe(&ObjectRef::new("main", "returns"))
+        .await
+        .unwrap();
+    assert_eq!(
+        structure.file.unwrap().options,
+        "auto_detect = true, sample_size = 20000, types = {'reason': 'VARCHAR', 'returned_at': \
+         'VARCHAR'}"
+    );
+    let rows = read(
+        &connection,
+        "SELECT count(*), count(returned_at) FROM returns",
+    )
+    .await
+    .unwrap();
+    assert_eq!(rows[0][0], rows[0][1]);
+}
+
+/// A folder holding only `returns.csv` whose `returned_at` has `N/A` and
+/// `-` among its first rows, inside the sample.
+fn early_returns() -> tempfile::TempDir {
+    let dir = tempfile::tempdir().unwrap();
+    let mut text = String::from("order_id,returned_at,reason\n");
+    text.push_str("1,N/A,lost in the post\n2,-,damaged\n");
+    for order in 3..=40 {
+        let day = order % 28 + 1;
+        text.push_str(&format!("{order},2026-02-{day:02},damaged\n"));
+    }
+    std::fs::write(dir.path().join("returns.csv"), text).unwrap();
+    dir
+}
+
+#[tokio::test]
+async fn values_read_as_empty_keep_the_column_its_type_wherever_they_are() {
+    let count = "SELECT typeof(returned_at), count(*), count(returned_at) FROM returns \
+                 GROUP BY 1";
+    let read_as = |rows: Vec<Vec<Value>>| -> Vec<String> { rows[0].iter().map(text_of).collect() };
+    let empty = |column: &str, values: &[&str]| {
+        change(
+            "returns.csv",
+            Change::Empty {
+                column: column.into(),
+                values: values.iter().map(|value| (*value).to_owned()).collect(),
+            },
+        )
+    };
+    // Inside the sample, two overrides of one column, named in another
+    // case: one cast, the column a date.
+    let dir = early_returns();
+    let reading = Reading {
+        overrides: vec![empty("Returned_At", &["N/A"]), empty("returned_at", &["-"])],
+        ..Reading::default()
+    };
+    let connection = open(dir.path(), reading).await.unwrap();
+    assert_eq!(
+        read_as(read(&connection, count).await.unwrap()),
+        ["DATE", "40", "38"]
+    );
+    assert_eq!(
+        made(&connection, "returns", dir.path()).await,
+        "CREATE VIEW returns AS SELECT * REPLACE (CAST(NULLIF(NULLIF(returned_at, 'N/A'), '-') \
+         AS DATE) AS returned_at) FROM read_csv('<dir>/returns.csv', auto_detect = true, \
+         sample_size = 20000, types = {'returned_at': 'VARCHAR'})"
+    );
+    // Every row read for types: still a date.
+    let dir = late_returns();
+    let reading = Reading {
+        types: Types::All,
+        overrides: vec![empty("returned_at", &["N/A"])],
+        ..Reading::default()
+    };
+    let connection = open(dir.path(), reading).await.unwrap();
+    assert_eq!(
+        read_as(read(&connection, count).await.unwrap()),
+        ["DATE", "30001", "30000"]
+    );
+    // No values: nothing changes.
+    let dir = early_returns();
+    let reading = Reading {
+        overrides: vec![empty("returned_at", &[])],
+        ..Reading::default()
+    };
+    let connection = open(dir.path(), reading).await.unwrap();
+    assert_eq!(
+        made(&connection, "returns", dir.path()).await,
+        "CREATE VIEW returns AS SELECT * FROM read_csv('<dir>/returns.csv', auto_detect = true, \
+         sample_size = 20000)"
+    );
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p tabletist-db --lib --test files_csv`

Expected: FAIL to compile, in `tests/files_csv.rs` and in `spec.rs`'s tests, as `Reading` and its kin do not exist yet (the changed assertion in `tests/files.rs` fails at run time until Step 3):

```text
error[E0432]: unresolved imports `tabletist_db::files::Change`, `tabletist_db::files::Delimiter`, `tabletist_db::files::Header`, `tabletist_db::files::Override`, `tabletist_db::files::Reading`, `tabletist_db::files::Types`
error[E0609]: no field `csv` on type `ConnectSpec`
error[E0432]: unresolved imports `crate::files::Change`, `crate::files::Delimiter`, `crate::files::Override`, `crate::files::Reading`, `crate::files::Types`
error[E0609]: no field `csv` on type `spec::ConnectSpec`
```

- [ ] **Step 3: Read CSV from the reading**

```diff
diff --git a/crates/tabletist-db/src/files.rs b/crates/tabletist-db/src/files.rs
--- a/crates/tabletist-db/src/files.rs
+++ b/crates/tabletist-db/src/files.rs
@@ -52,6 +52,148 @@
         match self {
             Self::Folder(folder) => Some(folder),
             Self::Files(files) => files.first().and_then(|file| file.parent()),
+        }
+    }
+}
+
+/// How a Files connection reads its CSV and TSV files: the defaults the
+/// dialog's Reading CSV section sets, and the overrides for one file.
+#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(default)]
+pub struct Reading {
+    pub header: Header,
+    /// For `.csv` files: a `.tsv` is read with a tab whatever this says.
+    pub delimiter: Delimiter,
+    pub types: Types,
+    pub overrides: Vec<Override>,
+}
+
+impl Reading {
+    /// Whether it reads as a new connection does, with no override.
+    pub fn is_default(&self) -> bool {
+        *self == Self::default()
+    }
+
+    /// The overrides of the file named `file` (its name with the
+    /// extension, as `connections.toml` says it).
+    pub fn overrides_of<'a>(&'a self, file: &'a str) -> impl Iterator<Item = &'a Change> + 'a {
+        self.overrides
+            .iter()
+            .filter(move |each| each.file == file)
+            .map(|each| &each.change)
+    }
+
+    /// Whether the file named `file` is left out of the connection.
+    pub fn skips(&self, file: &str) -> bool {
+        self.overrides_of(file)
+            .any(|change| matches!(change, Change::Skip))
+    }
+}
+
+/// Whether a CSV's first row names its columns.
+#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "lowercase")]
+pub enum Header {
+    /// As DuckDB's sniffer finds it.
+    #[default]
+    Detect,
+    Yes,
+    No,
+}
+
+impl Header {
+    pub const ALL: [Self; 3] = [Self::Detect, Self::Yes, Self::No];
+}
+
+/// What a CSV's values are parted by.
+#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "lowercase")]
+pub enum Delimiter {
+    /// As DuckDB's sniffer finds it.
+    #[default]
+    Detect,
+    Comma,
+    Semicolon,
+    Tab,
+    Pipe,
+}
+
+impl Delimiter {
+    pub const ALL: [Self; 5] = [
+        Self::Detect,
+        Self::Comma,
+        Self::Semicolon,
+        Self::Tab,
+        Self::Pipe,
+    ];
+
+    /// The character, where one is chosen.
+    pub fn char(self) -> Option<char> {
+        match self {
+            Self::Detect => None,
+            Self::Comma => Some(','),
+            Self::Semicolon => Some(';'),
+            Self::Tab => Some('\t'),
+            Self::Pipe => Some('|'),
+        }
+    }
+
+    /// The delimiter whose character is `char`.
+    pub fn of(char: char) -> Option<Self> {
+        Self::ALL.into_iter().find(|each| each.char() == Some(char))
+    }
+}
+
+/// How a CSV's column types are found.
+#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "lowercase")]
+pub enum Types {
+    /// From the first rows read ([`SAMPLE_ROWS`]).
+    #[default]
+    Sample,
+    /// From every row: slower, and nothing past the sample surprises it.
+    All,
+    /// None: every column is text.
+    Text,
+}
+
+impl Types {
+    pub const ALL: [Self; 3] = [Self::Sample, Self::All, Self::Text];
+}
+
+/// How many rows of a CSV DuckDB samples for its columns' types, and how
+/// many rows of each file a Test reads.
+pub const SAMPLE_ROWS: u32 = 20_000;
+
+/// A change to how one file is read.
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Override {
+    /// The file's name with its extension: `returns.csv`.
+    pub file: String,
+    pub change: Change,
+}
+
+/// What an override changes.
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "lowercase")]
+pub enum Change {
+    /// The column is read as text.
+    Text { column: String },
+    /// These values of the column are read as empty (NULL); the column
+    /// keeps the type found for it with them taken as empty. Several of
+    /// one column are one; with no values it changes nothing.
+    Empty { column: String, values: Vec<String> },
+    /// The file is left out of the connection.
+    Skip,
+}
+
+impl Change {
+    /// The column it is about, which matches the file's in any case;
+    /// `None` for a skip.
+    pub fn column(&self) -> Option<&str> {
+        match self {
+            Self::Text { column } | Self::Empty { column, .. } => Some(column),
+            Self::Skip => None,
         }
     }
 }
@@ -348,8 +490,9 @@
         .collect()
 }
 
-/// A file's name, for a list of them.
-fn file_name(path: &Path) -> String {
+/// A file's name with its extension: for a list of them, and as an
+/// override names it.
+pub fn file_name(path: &Path) -> String {
     path.file_name()
         .map(|name| name.to_string_lossy().into_owned())
         .unwrap_or_else(|| path.display().to_string())
diff --git a/crates/tabletist-db/src/duck.rs b/crates/tabletist-db/src/duck.rs
--- a/crates/tabletist-db/src/duck.rs
+++ b/crates/tabletist-db/src/duck.rs
@@ -19,7 +19,7 @@
 
 use crate::adapter::Adapter;
 use crate::agent::{Catalog, ColumnName, Limits, Param, Read, Refusal, Sources};
-use crate::files::{Format, Found, Source};
+use crate::files::{Change, Format, Found, Header, Reading, SAMPLE_ROWS, Source, Types};
 use crate::{
     CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, Ddl, Dialect, Error,
     Lookup, LookupPage, MAX_LISTED, ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery,
@@ -32,10 +32,6 @@
 
 /// DuckDB's name for the in-memory database the views are in.
 const MEMORY: &str = "memory";
-
-/// How many rows Test reads of each file, and how many rows of a CSV the
-/// reader samples for its columns' types.
-pub(crate) const SAMPLE_ROWS: u32 = 20_000;
 
 /// Where DuckDB keeps the extensions it installs: the app's own folder,
 /// set once at start. Unset (in tests), DuckDB's own default.
@@ -62,8 +58,9 @@
     pub path: std::path::PathBuf,
     pub size: u64,
     pub format: Format,
-    /// How the reader is told to read it, as the view's call says it:
-    /// `header = true, auto_detect = true, sample_size = 20000`.
+    /// How the reader is told to read it, as the view's call says it, the
+    /// connection's Reading CSV and the file's overrides included:
+    /// `auto_detect = true, sample_size = 20000`.
     pub options: String,
 }
 
@@ -87,6 +84,9 @@
     /// The statement that made its view, which the DDL view shows: DuckDB
     /// keeps it rewritten, each option in parentheses.
     made: String,
+    /// The reader's options in that statement, as the Structure view says
+    /// them.
+    options: String,
     /// Rows, where the file says (Parquet's metadata) or a sample of it
     /// lets them be estimated.
     rows: Option<u64>,
@@ -145,15 +145,46 @@
 }
 
 /// The reader's options for a file of `format`, as its call writes them
-/// after the path.
-fn reader_options(format: Format) -> &'static str {
+/// after the path: a CSV's and a TSV's from the connection's `reading`,
+/// with `text` the columns read as text; the other formats' fixed.
+fn reader_options(format: Format, reading: &Reading, text: &[&str]) -> String {
     match format {
-        Format::Csv => "header = true, auto_detect = true, sample_size = 20000",
-        Format::Tsv => "header = true, auto_detect = true, sample_size = 20000, delim = '\\t'",
-        Format::Json => "format = 'array'",
-        Format::Ndjson => "format = 'newline_delimited'",
-        Format::Excel => "header = true",
-        Format::Parquet | Format::DuckDb => "",
+        Format::Csv | Format::Tsv => {
+            let mut options = Vec::new();
+            match reading.header {
+                Header::Detect => {}
+                Header::Yes => options.push("header = true".to_owned()),
+                Header::No => options.push("header = false".to_owned()),
+            }
+            options.push("auto_detect = true".to_owned());
+            options.push(match reading.types {
+                Types::Sample => format!("sample_size = {SAMPLE_ROWS}"),
+                Types::All => "sample_size = -1".to_owned(),
+                Types::Text => "all_varchar = true".to_owned(),
+            });
+            // A TSV is tab-separated by its name.
+            let delimiter = match format {
+                Format::Tsv => Some('\t'),
+                _ => reading.delimiter.char(),
+            };
+            match delimiter {
+                None => {}
+                Some('\t') => options.push("delim = '\\t'".to_owned()),
+                Some(char) => options.push(format!("delim = {}", literal(&char.to_string()))),
+            }
+            if !text.is_empty() {
+                let types: Vec<String> = text
+                    .iter()
+                    .map(|column| format!("{}: 'VARCHAR'", literal(column)))
+                    .collect();
+                options.push(format!("types = {{{}}}", types.join(", ")));
+            }
+            options.join(", ")
+        }
+        Format::Json => "format = 'array'".to_owned(),
+        Format::Ndjson => "format = 'newline_delimited'".to_owned(),
+        Format::Excel => "header = true".to_owned(),
+        Format::Parquet | Format::DuckDb => String::new(),
     }
 }
 
@@ -170,35 +201,151 @@
 }
 
 /// The statement that makes `file` a table or a schema of the session,
-/// its name bare where that is safe: the DDL view shows it as it is.
-fn make(file: &Found) -> Result<String> {
+/// its name bare where that is safe (the DDL view shows it as it is), and
+/// the reader's options in it. A CSV's or TSV's overrides of a column
+/// are kept only where the file has that column, which a `DESCRIBE` of
+/// the call without them says, with the type found for each.
+fn make(
+    connection: &duckdb::Connection,
+    file: &Found,
+    reading: &Reading,
+) -> Result<(String, String)> {
     let name = Dialect::DuckDb.ident(&file.name);
-    Ok(match file.format {
-        Format::DuckDb => format!(
+    let format = file.format;
+    if format == Format::DuckDb {
+        let statement = format!(
             "ATTACH {} AS {name} (READ_ONLY)",
             literal(&path_text(&file.path)?)
-        ),
-        format => {
-            let path = literal(&reader_path(&file.path)?);
-            let options = reader_options(format);
-            let call = match options {
-                "" => format!("{}({path})", reader(format)),
-                options => format!("{}({path}, {options})", reader(format)),
-            };
-            format!("CREATE VIEW {name} AS SELECT * FROM {call}")
-        }
-    })
+        );
+        return Ok((statement, String::new()));
+    }
+    let path = literal(&reader_path(&file.path)?);
+    let call = |options: &str| match options {
+        "" => format!("{}({path})", reader(format)),
+        options => format!("{}({path}, {options})", reader(format)),
+    };
+    let file_name = crate::files::file_name(&file.path);
+    // The file's changes of a column. Values read as empty that are none
+    // change nothing.
+    let changes: Vec<&Change> = match format {
+        Format::Csv | Format::Tsv => reading
+            .overrides_of(&file_name)
+            .filter(|change| match change {
+                Change::Text { .. } => true,
+                Change::Empty { values, .. } => !values.is_empty(),
+                Change::Skip => false,
+            })
+            .collect(),
+        Format::Parquet | Format::Json | Format::Ndjson | Format::Excel | Format::DuckDb => {
+            Vec::new()
+        }
+    };
+    let plain = reader_options(format, reading, &[]);
+    if changes.is_empty() {
+        let statement = format!("CREATE VIEW {name} AS SELECT * FROM {}", call(&plain));
+        return Ok((statement, plain));
+    }
+    // The file's columns, each with the type found for it while the values
+    // read as empty are taken as empty (beside DuckDB's own empty string):
+    // the type a column read as empty is cast back to.
+    let mut empty = vec![literal("")];
+    for change in &changes {
+        if let Change::Empty { values, .. } = change {
+            empty.extend(values.iter().map(|value| literal(value)));
+        }
+    }
+    let sniffed = match empty.len() {
+        1 => plain.clone(),
+        _ => format!("{plain}, nullstr = [{}]", empty.join(", ")),
+    };
+    let found = described(connection, &call(&sniffed))?;
+    // A column as the file names it, matched in any case, as DuckDB
+    // matches names.
+    let column_of = |column: &str| {
+        found
+            .iter()
+            .find(|(name, _)| name.eq_ignore_ascii_case(column))
+    };
+    let same = |a: &str, b: &str| a.eq_ignore_ascii_case(b);
+    // Read as text: the text overrides' columns and the columns whose
+    // values are read as empty, which are cast back below unless they are
+    // read as text too.
+    let mut text: Vec<&str> = Vec::new();
+    for change in &changes {
+        if let Some((column, _)) = change.column().and_then(column_of)
+            && !text.contains(&column.as_str())
+        {
+            text.push(column);
+        }
+    }
+    let mut replaced = Vec::new();
+    let mut cast: Vec<&str> = Vec::new();
+    for change in &changes {
+        let Change::Empty { column, .. } = change else {
+            continue;
+        };
+        let Some((column, kind)) = column_of(column) else {
+            continue;
+        };
+        let read_as_text = changes
+            .iter()
+            .any(|other| matches!(other, Change::Text { column: other } if same(other, column)));
+        if read_as_text || cast.contains(&column.as_str()) {
+            continue;
+        }
+        cast.push(column);
+        // Every override of the column, its values in one cast.
+        let values = changes.iter().flat_map(|other| match other {
+            Change::Empty {
+                column: other,
+                values,
+            } if same(other, column) => values.as_slice(),
+            Change::Text { .. } | Change::Empty { .. } | Change::Skip => &[],
+        });
+        let quoted = Dialect::DuckDb.ident(column).into_owned();
+        let value = values.fold(quoted.clone(), |inner, value| {
+            format!("NULLIF({inner}, {})", literal(value))
+        });
+        replaced.push(format!("CAST({value} AS {kind}) AS {quoted}"));
+    }
+    let options = reader_options(format, reading, &text);
+    let columns = match replaced.is_empty() {
+        true => "*".to_owned(),
+        false => format!("* REPLACE ({})", replaced.join(", ")),
+    };
+    let statement = format!(
+        "CREATE VIEW {name} AS SELECT {columns} FROM {}",
+        call(&options)
+    );
+    Ok((statement, options))
+}
+
+/// The columns a reader's call gives, by name, with DuckDB's type of each.
+fn described(connection: &duckdb::Connection, call: &str) -> Result<Vec<(String, String)>> {
+    let mut statement = connection
+        .prepare(&format!("DESCRIBE SELECT * FROM {call}"))
+        .map_err(map_error)?;
+    let columns = statement
+        .query_map([], |row| {
+            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
+        })
+        .map_err(map_error)?
+        .collect::<std::result::Result<Vec<_>, _>>()
+        .map_err(map_error)?;
+    Ok(columns)
 }
 
 /// Rows of a text file by a sample of its start: its lines in the first
-/// 256 KiB, scaled to its size, without a CSV's header. `None` for a
-/// format that is not a line a row, or a file that cannot be read.
-fn estimate(file: &Found) -> Option<u64> {
+/// 256 KiB, scaled to its size, without a CSV's header unless it is read
+/// as having none. `None` for a format that is not a line a row, or a file
+/// that cannot be read.
+fn estimate(file: &Found, header: Header) -> Option<u64> {
     use std::io::Read as _;
-    let header = match file.format {
-        Format::Csv | Format::Tsv => 1,
-        Format::Ndjson => 0,
-        Format::Parquet | Format::Json | Format::Excel | Format::DuckDb => return None,
+    let header = match (file.format, header) {
+        (Format::Csv | Format::Tsv, Header::No) => 0,
+        (Format::Csv | Format::Tsv, Header::Detect | Header::Yes) => 1,
+        (Format::Ndjson, _) => 0,
+        (Format::Parquet | Format::Json | Format::Excel | Format::DuckDb, _) => return None,
     };
     let mut start = Vec::with_capacity(256 * 1024);
     std::fs::File::open(&file.path)
@@ -224,8 +371,13 @@
     Some(rows.saturating_sub(header))
 }
 
-/// Opens a session over `files`: views and attaches, then the fence.
-fn opened(files: &[Found]) -> Result<(duckdb::Connection, Vec<Opened>, String)> {
+/// Opens a session over `files`, read as `reading` says, a file it skips
+/// left out: views and attaches, then the fence.
+fn opened(files: &[Found], reading: &Reading) -> Result<(duckdb::Connection, Vec<Opened>, String)> {
+    let files: Vec<&Found> = files
+        .iter()
+        .filter(|file| !reading.skips(&crate::files::file_name(&file.path)))
+        .collect();
     let config = duckdb::Config::default()
         .enable_autoload_extension(false)
         .map_err(map_error)?;
@@ -248,16 +400,14 @@
         })?;
     }
     let mut opened = Vec::with_capacity(files.len());
-    for file in files {
+    for file in &files {
+        let failed = |error: &Error| {
+            Error::Connect(format!("Can't read {}: {}", file_label(file), plain(error)))
+        };
         // A `.duckdb` file is attached read-only, as every session is.
-        let statement = make(file)?;
-        run(&statement).map_err(|error| {
-            Error::Connect(format!(
-                "Can't read {}: {}",
-                file_label(file),
-                plain(&error)
-            ))
-        })?;
+        let (statement, options) =
+            make(&connection, file, reading).map_err(|error| failed(&error))?;
+        run(&statement).map_err(|error| failed(&error))?;
         let rows = match file.format {
             Format::Parquet => connection
                 .query_row(
@@ -276,18 +426,19 @@
             | Format::Json
             | Format::Ndjson
             | Format::Excel
-            | Format::DuckDb => estimate(file),
+            | Format::DuckDb => estimate(file, reading.header),
         };
         opened.push(Opened {
-            found: file.clone(),
+            found: (*file).clone(),
             made: statement,
+            options,
             rows,
         });
     }
     // The fence: these files and no other, then no setting changed again.
     // A reader asks by its own spelling of a file, which names only it.
     let mut allowed = Vec::with_capacity(files.len());
-    for file in files {
+    for file in &files {
         let (path, read) = (path_text(&file.path)?, reader_path(&file.path)?);
         if read != path {
             allowed.push(literal(&read));
@@ -934,8 +1085,8 @@
 impl Conn {
     /// Opens `source`'s files, read-only: every one is read far enough to
     /// know its columns, and an error names the first that cannot be.
-    pub(crate) async fn open(source: &Source) -> Result<Self> {
-        let source = source.clone();
+    pub(crate) async fn open(source: &Source, reading: &Reading) -> Result<Self> {
+        let (source, reading) = (source.clone(), reading.clone());
         let (connection, files, version) = tokio::task::spawn_blocking(move || {
             let scan = crate::files::scan(&source)?;
             if let Some(gone) = scan.missing.first() {
@@ -944,7 +1095,7 @@
                     crate::files::shown(gone)
                 )));
             }
-            opened(&scan.files)
+            opened(&scan.files, &reading)
         })
         .await
         .map_err(|error| Error::Io(error.to_string()))??;
@@ -1159,7 +1310,7 @@
             path: file.found.path.clone(),
             size: file.found.size,
             format: file.found.format,
-            options: reader_options(file.found.format).to_owned(),
+            options: file.options.clone(),
         });
         let object = object.clone();
         self.run(move |connection| {
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -134,7 +134,9 @@
                     .files
                     .as_ref()
                     .ok_or_else(|| Error::InvalidSpec("choose a folder or files".into()))?;
-                Ok(Self::Files(Box::new(duck::Conn::open(source).await?)))
+                Ok(Self::Files(Box::new(
+                    duck::Conn::open(source, &spec.csv).await?,
+                )))
             }
         }
     }
diff --git a/crates/tabletist-db/src/spec.rs b/crates/tabletist-db/src/spec.rs
--- a/crates/tabletist-db/src/spec.rs
+++ b/crates/tabletist-db/src/spec.rs
@@ -122,6 +122,9 @@
     pub ssh: Option<SshSpec>,
     /// The files a Files connection opens; `None` for every other driver.
     pub files: Option<crate::files::Source>,
+    /// How a Files connection reads its CSV and TSV files.
+    #[serde(skip_serializing_if = "crate::files::Reading::is_default")]
+    pub csv: crate::files::Reading,
 }
 
 impl Default for ConnectSpec {
@@ -138,6 +141,7 @@
             ca_file: None,
             ssh: None,
             files: None,
+            csv: crate::files::Reading::default(),
         }
     }
 }
diff --git a/crates/tabletist-core/src/connections/file.rs b/crates/tabletist-core/src/connections/file.rs
--- a/crates/tabletist-core/src/connections/file.rs
+++ b/crates/tabletist-core/src/connections/file.rs
@@ -317,6 +317,7 @@
                 ca_file: self.ca_file,
                 ssh,
                 files,
+                csv: tabletist_db::files::Reading::default(),
             },
             agents,
         })
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -2752,6 +2752,7 @@
                     ca_file: (!ca_file.is_empty()).then(|| ca_file.into()),
                     ssh,
                     files: None,
+                    csv: tabletist_db::files::Reading::default(),
                 })
             }
         }
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test -p tabletist-db --test files_csv --test files` and `cargo test -p tabletist-db --lib spec::`

Expected: PASS. Then the full check: fmt and clippy clean, every test binary `ok` (5423 tests pass in this state).

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db crates/tabletist-core/src/model.rs crates/tabletist-core/src/connections/file.rs
git commit -m "Read a Files connection's CSV files as its Reading CSV says"
```

---

### Task 2: `connections.toml` version 8 keeps the reading

**Files:**
- Modify: `crates/tabletist-core/src/connections/file.rs`
- Modify: `docs/_reference/settings-and-files.md`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/crates/tabletist-core/src/connections/file.rs b/crates/tabletist-core/src/connections/file.rs
--- a/crates/tabletist-core/src/connections/file.rs
+++ b/crates/tabletist-core/src/connections/file.rs
@@ -1035,8 +1035,8 @@
         let said = |text: &str| read(text).unwrap_err().to_string();
         assert!(said("").contains("missing field `version`"), "{}", said(""));
         assert_eq!(
-            said("version = 8\n"),
-            "it is version 8 of the file, written by a newer Tabletist; this one reads version 7"
+            said("version = 9\n"),
+            "it is version 9 of the file, written by a newer Tabletist; this one reads version 8"
         );
         assert_eq!(
             said("version = 2\n"),
@@ -1404,6 +1404,192 @@
         assert!(!others.contains("version = 7"), "{others}");
     }
 
+    /// How a Files connection reads CSV, as the file writes it: version 8,
+    /// the defaults' table only where they are not the defaults, and an
+    /// override table each.
+    #[test]
+    fn how_a_files_connection_reads_csv_is_written_in_version_eight() {
+        use tabletist_db::files::{Change, Delimiter, Header, Override, Reading, Types};
+        let mut spec = ConnectSpec::files(Source::Folder(PathBuf::from("/srv/exports")));
+        spec.csv = Reading {
+            header: Header::Yes,
+            delimiter: Delimiter::Tab,
+            types: Types::All,
+            overrides: vec![
+                Override {
+                    file: "returns.csv".into(),
+                    change: Change::Text {
+                        column: "returned_at".into(),
+                    },
+                },
+                Override {
+                    file: "returns.csv".into(),
+                    change: Change::Empty {
+                        column: "reason".into(),
+                        values: vec!["N/A".into(), "-".into()],
+                    },
+                },
+                Override {
+                    file: "archive.csv".into(),
+                    change: Change::Skip,
+                },
+            ],
+        };
+        let exports = saved("exports", "Bookshop exports", spec.clone());
+        let text = write(std::slice::from_ref(&exports)).unwrap();
+        assert!(text.contains("\nversion = 8\n"), "{text}");
+        assert!(
+            text.ends_with(
+                "\n[connections.source]\n\
+                 folder = \"/srv/exports\"\n\
+                 \n[connections.csv]\n\
+                 header    = \"yes\"\n\
+                 delimiter = \"\\t\"\n\
+                 types     = \"all\"\n\
+                 \n[[connections.override]]\n\
+                 file    = \"returns.csv\"\n\
+                 column  = \"returned_at\"\n\
+                 read_as = \"text\"\n\
+                 \n[[connections.override]]\n\
+                 file        = \"returns.csv\"\n\
+                 column      = \"reason\"\n\
+                 null_values = [\"N/A\", \"-\"]\n\
+                 \n[[connections.override]]\n\
+                 file = \"archive.csv\"\n\
+                 skip = true\n"
+            ),
+            "{text}"
+        );
+        assert_eq!(read(&text).unwrap().connections, [exports]);
+        // Overrides alone: no [connections.csv], still version 8.
+        let mut overridden = spec.clone();
+        overridden.csv = Reading {
+            overrides: spec.csv.overrides[2..].to_vec(),
+            ..Reading::default()
+        };
+        let overridden = saved("exports", "Bookshop exports", overridden);
+        let text = write(std::slice::from_ref(&overridden)).unwrap();
+        assert!(text.contains("\nversion = 8\n"), "{text}");
+        assert!(!text.contains("[connections.csv]"), "{text}");
+        assert_eq!(read(&text).unwrap().connections, [overridden]);
+        // Values read as empty that are none are not written: they change
+        // nothing, and the file could not read them back.
+        let mut nothing = spec.clone();
+        nothing.csv = Reading {
+            overrides: vec![Override {
+                file: "returns.csv".into(),
+                change: Change::Empty {
+                    column: "reason".into(),
+                    values: Vec::new(),
+                },
+            }],
+            ..Reading::default()
+        };
+        let text = write(&[saved("exports", "Bookshop exports", nothing)]).unwrap();
+        assert!(!text.contains("[[connections.override]]"), "{text}");
+        assert!(text.contains("\nversion = 7\n"), "{text}");
+        assert!(read(&text).is_ok(), "{text}");
+        // Every delimiter reads back as itself.
+        for delimiter in Delimiter::ALL {
+            let mut each = spec.clone();
+            each.csv.delimiter = delimiter;
+            let each = saved("exports", "Bookshop exports", each);
+            let text = write(std::slice::from_ref(&each)).unwrap();
+            assert_eq!(read(&text).unwrap().connections, [each], "{text}");
+        }
+        // The defaults: version 7, as before.
+        let plain = saved(
+            "exports",
+            "Bookshop exports",
+            ConnectSpec::files(Source::Folder(PathBuf::from("/srv/exports"))),
+        );
+        let text = write(&[plain]).unwrap();
+        assert!(text.contains("\nversion = 7\n"), "{text}");
+        assert!(!text.contains("csv"), "{text}");
+    }
+
+    #[test]
+    fn a_csv_table_and_its_overrides_say_what_they_take() {
+        let entry = |driver: &str, rest: &str| {
+            format!(
+                "version = 8\n[[connections]]\nid = \"a\"\nname = \"A\"\n\
+                 driver = \"{driver}\"\n[connections.source]\nfolder = \"/a\"\n{rest}"
+            )
+        };
+        let files = |rest: &str| entry("files", rest);
+        for (text, says) in [
+            (
+                files("[connections.csv]\nheader = \"maybe\"\n"),
+                "csv `header` is \"maybe\"; it takes detect, yes or no",
+            ),
+            (
+                files("[connections.csv]\ndelimiter = \",;\"\n"),
+                "csv `delimiter` is \",;\"",
+            ),
+            (
+                files("[connections.csv]\ntypes = \"some\"\n"),
+                "csv `types` is \"some\"; it takes sample, all or text",
+            ),
+            (
+                files("[connections.csv]\nquote = \"'\"\n"),
+                "unknown field `quote`",
+            ),
+            (
+                files("[[connections.override]]\nfile = \"returns.csv\"\nread_as = \"text\"\n"),
+                "the override of \"returns.csv\" needs the `column` it changes",
+            ),
+            (
+                files(
+                    "[[connections.override]]\nfile = \"returns.csv\"\ncolumn = \"reason\"\n\
+                     read_as = \"date\"\n",
+                ),
+                "reads its column as \"date\"",
+            ),
+            (
+                files(
+                    "[[connections.override]]\nfile = \"returns.csv\"\ncolumn = \"reason\"\n\
+                     null_values = []\n",
+                ),
+                "has no `null_values`",
+            ),
+            (
+                files(
+                    "[[connections.override]]\nfile = \"returns.csv\"\ncolumn = \"reason\"\n\
+                     skip = true\n",
+                ),
+                "skips the file, so it names no column",
+            ),
+            (
+                files(
+                    "[[connections.override]]\nfile = \"returns.csv\"\ncolumn = \"reason\"\n\
+                     read_as = \"text\"\nnull_values = [\"N/A\"]\n",
+                ),
+                "has a `read_as`, `null_values` or `skip = true`, one of them",
+            ),
+            (
+                files("[[connections.override]]\nfile = \"returns.csv\"\n"),
+                "needs a `read_as = \"text\"`, `null_values` or `skip = true`",
+            ),
+            (
+                files("[[connections.override]]\nfile = \"returns.csv\"\ncolumn = \"reason\"\n"),
+                "needs a `read_as = \"text\"`, `null_values` or `skip = true`",
+            ),
+            (
+                files("[[connections.override]]\nfile = \" \"\nskip = true\n"),
+                "an override names no file",
+            ),
+            (
+                "version = 8\n[[connections]]\nid = \"a\"\nname = \"A\"\n\
+                     driver = \"sqlite\"\n[connections.csv]\nheader = \"yes\"\n"
+                    .to_owned(),
+                "only a Files connection reads CSV files",
+            ),
+        ] {
+            let problem = read(&text).unwrap_err();
+            assert!(problem.what.contains(says), "{text}\n{problem}");
+        }
+    }
+
     #[test]
     fn a_source_is_a_folder_or_files_and_only_a_files_connections() {
         let entry = |driver: &str, source: &str| {
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p tabletist-core --lib connections::file::tests`

Expected: FAIL: the reading is not written, and the file is still version 7:

```text
thread 'connections::file::tests::the_file_says_which_version_it_is' (2744042) panicked at crates/tabletist-core/src/connections/file.rs:1037:9:
thread 'connections::file::tests::a_csv_table_and_its_overrides_say_what_they_take' (2744027) panicked at crates/tabletist-core/src/connections/file.rs:1589:13:
thread 'connections::file::tests::how_a_files_connection_reads_csv_is_written_in_version_eight' (2744039) panicked at crates/tabletist-core/src/connections/file.rs:1440:9:
```

- [ ] **Step 3: Read and write `[connections.csv]` and `[[connections.override]]`**

````diff
diff --git a/crates/tabletist-core/src/connections/file.rs b/crates/tabletist-core/src/connections/file.rs
--- a/crates/tabletist-core/src/connections/file.rs
+++ b/crates/tabletist-core/src/connections/file.rs
@@ -12,7 +12,7 @@
 
 use serde::Deserialize;
 use tabletist_db::agent::{ColumnName, TableName, Tables};
-use tabletist_db::files::Source;
+use tabletist_db::files::{Change, Delimiter, Header, Override, Reading, Source, Types};
 use tabletist_db::{ConnectSpec, Driver, SshAuth, SshSpec, TlsMode};
 
 use super::{
@@ -24,13 +24,17 @@
 
 /// The file's format. 3 is the first that is TOML: 1 and 2 were
 /// `connections.json`. 4 has a connection's settings for agents, 5 the
-/// tables they may not read, 6 the columns they get masked, and 7 Files
-/// connections, with their `source`. A file is
+/// tables they may not read, 6 the columns they get masked, 7 Files
+/// connections, with their `source`, and 8 how a Files connection reads
+/// its CSV files (`[connections.csv]`, `[[connections.override]]`). A file is
 /// written as the oldest that holds what it has: a Tabletist that does not
 /// know a key refuses only a file it would lose it from, and one that
 /// would read agents' tables as all of them, or a masked column's values
 /// as they are, never reads the file.
-pub(super) const VERSION: u32 = 7;
+pub(super) const VERSION: u32 = 8;
+
+/// The version that has Files connections and not how they read CSV.
+const FILES_VERSION: u32 = 7;
 
 /// The version that has masked columns and no Files connection.
 const MASKED_VERSION: u32 = 6;
@@ -120,6 +124,122 @@
     ssh: Option<Tunnel>,
     agents: Option<AgentsTable>,
     source: Option<SourceTable>,
+    csv: Option<CsvTable>,
+    #[serde(default, rename = "override")]
+    overrides: Vec<OverrideTable>,
+}
+
+/// How a Files connection reads its CSV files, as the file has it: each
+/// key left out reads as a new connection does.
+#[derive(Deserialize)]
+#[serde(deny_unknown_fields)]
+struct CsvTable {
+    header: Option<String>,
+    delimiter: Option<String>,
+    types: Option<String>,
+}
+
+impl CsvTable {
+    /// The reading it says, with no override yet.
+    fn reading(self, connection: &str) -> Result<Reading, String> {
+        let wrong = |key: &str, value: &str, takes: &str| {
+            format!("connection {connection:?}: its csv `{key}` is {value:?}; it takes {takes}")
+        };
+        let header = match self.header.as_deref() {
+            None | Some("detect") => Header::Detect,
+            Some("yes") => Header::Yes,
+            Some("no") => Header::No,
+            Some(other) => return Err(wrong("header", other, "detect, yes or no")),
+        };
+        let delimiter = match self.delimiter.as_deref() {
+            None | Some("detect") => Delimiter::Detect,
+            Some(other) => {
+                let mut chars = other.chars();
+                match (chars.next().and_then(Delimiter::of), chars.next()) {
+                    (Some(delimiter), None) => delimiter,
+                    _ => {
+                        return Err(wrong(
+                            "delimiter",
+                            other,
+                            "detect, \",\", \";\", \"\\t\" or \"|\"",
+                        ));
+                    }
+                }
+            }
+        };
+        let types = match self.types.as_deref() {
+            None | Some("sample") => Types::Sample,
+            Some("all") => Types::All,
+            Some("text") => Types::Text,
+            Some(other) => return Err(wrong("types", other, "sample, all or text")),
+        };
+        Ok(Reading {
+            header,
+            delimiter,
+            types,
+            overrides: Vec::new(),
+        })
+    }
+}
+
+/// A change to how one file of a Files connection is read, as the file
+/// has it: a column read as text, a column's values read as empty, or the
+/// file skipped.
+#[derive(Deserialize)]
+#[serde(deny_unknown_fields)]
+struct OverrideTable {
+    file: String,
+    column: Option<String>,
+    read_as: Option<String>,
+    null_values: Option<Vec<String>>,
+    skip: Option<bool>,
+}
+
+impl OverrideTable {
+    fn change(self, connection: &str) -> Result<Override, String> {
+        let file = self.file;
+        let says =
+            |what: &str| format!("connection {connection:?}: the override of {file:?} {what}");
+        if file.trim().is_empty() {
+            return Err(format!(
+                "connection {connection:?}: an override names no file"
+            ));
+        }
+        let change = match (self.column, self.read_as, self.null_values, self.skip) {
+            (Some(column), Some(read_as), None, None | Some(false)) => match read_as.as_str() {
+                "text" => Change::Text { column },
+                other => {
+                    return Err(says(&format!(
+                        "reads its column as {other:?}; `read_as` takes \"text\""
+                    )));
+                }
+            },
+            (Some(_), None, Some(values), None | Some(false)) if values.is_empty() => {
+                return Err(says("has no `null_values`"));
+            }
+            (Some(column), None, Some(values), None | Some(false)) => {
+                Change::Empty { column, values }
+            }
+            (None, None, None, Some(true)) => Change::Skip,
+            (Some(_), None, None, Some(true)) => {
+                return Err(says("skips the file, so it names no column"));
+            }
+            (_, None, None, None | Some(false)) => {
+                return Err(says(
+                    "needs a `read_as = \"text\"`, `null_values` or `skip = true`",
+                ));
+            }
+            (None, Some(_) | None, Some(_) | None, None | Some(false)) => {
+                return Err(says("needs the `column` it changes"));
+            }
+            _ => {
+                return Err(says(
+                    "has a `read_as`, `null_values` or `skip = true`, one of them",
+                ));
+            }
+        };
+        Ok(Override { file, change })
+    }
 }
 
 /// What a Files connection opens, as the file has it: a folder or a list
@@ -297,6 +417,28 @@
                 ));
             }
             (Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery, None) => None,
+        };
+        let reads_csv = self.csv.is_some() || !self.overrides.is_empty();
+        let csv = match self.driver {
+            Driver::Files => {
+                let mut reading = match self.csv {
+                    Some(table) => table.reading(&self.name)?,
+                    None => Reading::default(),
+                };
+                for table in self.overrides {
+                    reading.overrides.push(table.change(&self.name)?);
+                }
+                reading
+            }
+            Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery if reads_csv => {
+                return Err(format!(
+                    "connection {:?}: only a Files connection reads CSV files",
+                    self.name
+                ));
+            }
+            Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery => {
+                Reading::default()
+            }
         };
         Ok(SavedConnection {
             id,
@@ -317,7 +459,7 @@
                 ca_file: self.ca_file,
                 ssh,
                 files,
-                csv: tabletist_db::files::Reading::default(),
+                csv,
             },
             agents,
         })
@@ -440,6 +582,77 @@
     }
 }
 
+/// A Files connection's `[connections.csv]`, where it reads CSV other than
+/// as a new one does, and an `[[connections.override]]` for each override.
+fn csv_tables(reading: &Reading, out: &mut String) {
+    if !options_are_default(reading) {
+        let mut lines = Lines::default();
+        lines.text(
+            "header",
+            match reading.header {
+                Header::Detect => "detect",
+                Header::Yes => "yes",
+                Header::No => "no",
+            },
+        );
+        match reading.delimiter.char() {
+            None => lines.text("delimiter", "detect"),
+            Some(char) => lines.text("delimiter", &char.to_string()),
+        }
+        lines.text(
+            "types",
+            match reading.types {
+                Types::Sample => "sample",
+                Types::All => "all",
+                Types::Text => "text",
+            },
+        );
+        out.push_str("\n[connections.csv]\n");
+        lines.write(out);
+    }
+    for each in written(reading) {
+        let mut lines = Lines::default();
+        lines.text("file", &each.file);
+        match &each.change {
+            Change::Text { column } => {
+                lines.text("column", column);
+                lines.text("read_as", "text");
+            }
+            Change::Empty { column, values } => {
+                lines.text("column", column);
+                let values: Vec<String> = values.iter().map(|value| toml_string(value)).collect();
+                lines.bare("null_values", format!("[{}]", values.join(", ")));
+            }
+            Change::Skip => lines.bare("skip", true),
+        }
+        out.push_str("\n[[connections.override]]\n");
+        lines.write(out);
+    }
+}
+
+/// Whether `reading`'s header, delimiter and types are a new connection's.
+fn options_are_default(reading: &Reading) -> bool {
+    Reading {
+        overrides: Vec::new(),
+        ..reading.clone()
+    }
+    .is_default()
+}
+
+/// The overrides the file holds: values read as empty that are none change
+/// nothing, and the file could not read them back.
+fn written(reading: &Reading) -> impl Iterator<Item = &Override> {
+    reading
+        .overrides
+        .iter()
+        .filter(|each| !matches!(&each.change, Change::Empty { values, .. } if values.is_empty()))
+}
+
+/// Whether `reading` is written: its `[connections.csv]` or an override.
+fn reads_csv_its_own_way(reading: &Reading) -> bool {
+    !options_are_default(reading) || written(reading).next().is_some()
+}
+
 fn password(mode: PasswordMode) -> &'static str {
     match mode {
         PasswordMode::None => "none",
@@ -481,12 +694,16 @@
         .any(|saved| saved.agents.tables.hides_anything());
     let agents = connections.iter().any(|saved| !saved.agents.is_default());
     let files = connections.iter().any(|saved| saved.spec.files.is_some());
-    let version = match (files, masks, hides, agents) {
-        (true, _, _, _) => VERSION,
-        (false, true, _, _) => MASKED_VERSION,
-        (false, false, true, _) => TABLES_VERSION,
-        (false, false, false, true) => AGENTS_VERSION,
-        (false, false, false, false) => FIRST_VERSION,
+    let csv = connections
+        .iter()
+        .any(|saved| saved.spec.files.is_some() && reads_csv_its_own_way(&saved.spec.csv));
+    let version = match (csv, files, masks, hides, agents) {
+        (true, _, _, _, _) => VERSION,
+        (false, true, _, _, _) => FILES_VERSION,
+        (false, false, true, _, _) => MASKED_VERSION,
+        (false, false, false, true, _) => TABLES_VERSION,
+        (false, false, false, false, true) => AGENTS_VERSION,
+        (false, false, false, false, false) => FIRST_VERSION,
     };
     let mut out = format!("{HEADER}version = {version}\n");
     for connection in connections {
@@ -559,6 +776,7 @@
             }
             out.push_str("\n[connections.source]\n");
             lines.write(&mut out);
+            csv_tables(&spec.csv, &mut out);
         }
         if !connection.agents.is_default() {
             let mut lines = Lines::default();
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -546,6 +546,37 @@
 | `ca_file` | The path of a certificate file | None |
 | `sqlite_path` | The path of a SQLite file | None |
 
+How a Files connection reads its CSV files is its `[connections.csv]`
+table, written only when it is not the defaults, and a change for one
+file is an `[[connections.override]]` table each:
+
+```toml
+[connections.csv]
+header    = "detect"
+delimiter = ";"
+types     = "sample"
+
+[[connections.override]]
+file    = "returns.csv"
+column  = "returned_at"
+read_as = "text"
+```
+
+| Key | Values | Without it |
+| --- | --- | --- |
+| `header` | `detect`, `yes`, `no` | `detect` |
+| `delimiter` | `detect`, `","`, `";"`, `"\t"`, `"\|"`: for `.csv` files, a `.tsv` is always read with a tab | `detect` |
+| `types` | `sample` (from the first 20,000 rows), `all` (from every row), `text` (every column as text) | `sample` |
+
+An override names its `file`, by its name with the extension, as the
+folder has it, and has one of `read_as = "text"` with the `column` it
+reads as text, `null_values = ["N/A"]` (one value or more) with the
+`column` whose values these are read as empty, or `skip = true`, which
+leaves the file out. A column matches the file's in any case. An
+override of a column the file does not have, or of a file the
+connection does not open, does nothing. A file that holds either table
+says `version = 8`.
+
 A tunnel is the connection's `[connections.ssh]` table: `host`, and
 optionally `port` and `user`, `auth` (`password`, `key-file` or `agent`),
 and with `key-file` the `key` to use.
````

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test -p tabletist-core --lib connections::`

Expected: PASS. Then the full check: fmt and clippy clean, every test binary `ok` (5425 tests pass in this state).

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-core/src/connections/file.rs docs/_reference/settings-and-files.md
git commit -m "Keep how a Files connection reads CSV in connections.toml, version 8"
```

---

### Task 3: The form keeps the reading and changes it through the App

**Files:**
- Modify: `crates/tabletist-core/src/model.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Write the failing test**

```diff
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -9115,4 +9115,72 @@
         // But not with the cursor moved back before the letter.
         assert!(!before.is_on(&completion_site(""), print("susers")));
     }
-}
+
+    /// How a Files connection reads CSV goes from the saved connection to
+    /// the form and back, and each change drops a Test that read the old
+    /// way.
+    #[test]
+    fn the_form_keeps_how_a_files_connection_reads_csv() {
+        use tabletist_db::files::{Change, Delimiter, Header, Override, Reading, Source, Types};
+        let mut spec = ConnectSpec::files(Source::Folder("/srv/exports".into()));
+        spec.csv = Reading {
+            header: Header::No,
+            overrides: vec![
+                Override {
+                    file: "returns.csv".into(),
+                    change: Change::Skip,
+                },
+                Override {
+                    file: "orders.csv".into(),
+                    change: Change::Text {
+                        column: "note".into(),
+                    },
+                },
+            ],
+            ..Reading::default()
+        };
+        let saved = SavedConnection {
+            id: ConnectionId("exports".into()),
+            name: "Bookshop exports".into(),
+            environment: crate::env::Environment::Local,
+            read_only: Some(true),
+            password: crate::connections::PasswordMode::None,
+            ssh_secret: crate::connections::PasswordMode::None,
+            spec: spec.clone(),
+            agents: crate::connections::Agents::default(),
+        };
+        let mut form = ConnectionForm::from_saved(&saved);
+        assert_eq!(form.files_csv, spec.csv);
+        assert!(!form.csv_open && !form.agents_open);
+        assert_eq!(form.to_spec().unwrap().csv, spec.csv);
+
+        let tested = || {
+            Some(tabletist_db::FilesRead {
+                files: 4,
+                bytes: 10,
+            })
+        };
+        form.files_tested = tested();
+        form.change_csv(CsvChange::Delimiter(Delimiter::Semicolon));
+        form.change_csv(CsvChange::Types(Types::Text));
+        form.change_csv(CsvChange::Header(Header::Yes));
+        assert_eq!(form.files_tested, None);
+        form.files_tested = tested();
+        form.change_csv(CsvChange::Reset(0));
+        assert_eq!(form.files_tested, None);
+        // What did not change keeps the Test.
+        form.files_tested = tested();
+        form.change_csv(CsvChange::Header(Header::Yes));
+        form.change_csv(CsvChange::Reset(7));
+        assert_eq!(form.files_tested, tested());
+        assert_eq!(
+            form.to_spec().unwrap().csv,
+            Reading {
+                header: Header::Yes,
+                delimiter: Delimiter::Semicolon,
+                types: Types::Text,
+                overrides: spec.csv.overrides[1..].to_vec(),
+            }
+        );
+    }
+}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p tabletist-core --lib the_form_keeps_how`

Expected: FAIL to compile, as `CsvChange` and the form's fields do not exist yet:

```text
error[E0609]: no field `files_csv` on type `model::ConnectionForm`
error[E0609]: no field `csv_open` on type `model::ConnectionForm`
error[E0609]: no field `agents_open` on type `model::ConnectionForm`
error[E0599]: no method named `change_csv` found for struct `model::ConnectionForm` in the current scope
error[E0433]: cannot find type `CsvChange` in this scope
Some errors have detailed explanations: E0433, E0599, E0609.
For more information about an error, try `rustc --explain E0433`.
error: could not compile `tabletist-core` (lib test) due to 15 previous errors
```

- [ ] **Step 3: Add the form's reading and `Action::SetCsv`**

```diff
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -114,6 +114,8 @@
     RemoveFile(usize),
     /// Read a Files connection's folder or files from a list instead.
     SetFilesKind(FilesKind),
+    /// A change in a Files connection's Reading CSV section.
+    SetCsv(CsvChange),
     /// Read what a Files connection's Source holds again, for the found
     /// line: after its folder is typed.
     ReadSource,
@@ -2158,6 +2160,14 @@
     pub files_found: FilesFound,
     /// Files: what the last Test that passed read.
     pub files_tested: Option<tabletist_db::FilesRead>,
+    /// Files: how its CSV and TSV files are read, overrides included.
+    pub files_csv: tabletist_db::files::Reading,
+    /// Files: whether the Reading CSV section is open. Closed each time
+    /// the dialog opens.
+    pub csv_open: bool,
+    /// Files: whether the Agents section is open, which a Files form
+    /// keeps closed until it is opened or `show_agents` asks.
+    pub agents_open: bool,
     /// Saving a used connection under another type was asked once: the
     /// next Save changes it.
     pub retype_asked: bool,
@@ -2183,6 +2193,16 @@
     Folder,
     /// Files added to a Files connection's list.
     Files,
+}
+
+/// A change in a Files connection's Reading CSV section.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum CsvChange {
+    Header(tabletist_db::files::Header),
+    Delimiter(tabletist_db::files::Delimiter),
+    Types(tabletist_db::files::Types),
+    /// Take off the override at this place.
+    Reset(usize),
 }
 
 /// Where a Files connection's files come from, as the Source choice says
@@ -2317,6 +2337,9 @@
             files_cursor: None,
             files_found: FilesFound::Unread,
             files_tested: None,
+            files_csv: tabletist_db::files::Reading::default(),
+            csv_open: false,
+            agents_open: false,
             retype_asked: false,
             agents_chosen: false,
             test_spec: None,
@@ -2349,6 +2372,26 @@
         }
         self.pick_request = None;
         self.ssh_hosts_request = None;
+    }
+
+    /// Applies a change made in the Reading CSV section. A Test read with
+    /// the old reading says nothing of the new one.
+    pub fn change_csv(&mut self, change: CsvChange) {
+        let reading = &mut self.files_csv;
+        let before = reading.clone();
+        match change {
+            CsvChange::Header(header) => reading.header = header,
+            CsvChange::Delimiter(delimiter) => reading.delimiter = delimiter,
+            CsvChange::Types(types) => reading.types = types,
+            CsvChange::Reset(at) => {
+                if at < reading.overrides.len() {
+                    reading.overrides.remove(at);
+                }
+            }
+        }
+        if self.files_csv != before {
+            self.forget_test();
+        }
     }
 
     /// Drops the Test, whether done or running, and what it read: neither
@@ -2589,6 +2632,7 @@
                     .collect(),
                 Some(Source::Folder(_)) | None => Vec::new(),
             },
+            files_csv: spec.csv.clone(),
             // A saved connection's agents are what the file says.
             agents_chosen: true,
             ..Self::default()
@@ -2685,7 +2729,10 @@
                 Ok(ConnectSpec::sqlite(path))
             }
             Driver::Files => match self.files_source() {
-                Some(source) => Ok(ConnectSpec::files(source)),
+                Some(source) => Ok(ConnectSpec {
+                    csv: self.files_csv.clone(),
+                    ..ConnectSpec::files(source)
+                }),
                 None => Err(match self.files_kind {
                     FilesKind::Folder => "Choose a folder.".into(),
                     FilesKind::Files => "Add a file.".into(),
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -3462,6 +3462,11 @@
                     self.read_form_source();
                 }
             }
+            Action::SetCsv(change) => {
+                if let Some(Dialog::Connection(form)) = &mut self.dialog {
+                    form.change_csv(change);
+                }
+            }
             Action::ReadSource => self.read_form_source(),
             Action::PickSshHost(alias) => {
                 if let Some(Dialog::Connection(form)) = &mut self.dialog {
```

- [ ] **Step 4: Run the test to see it pass**

Run: `cargo test -p tabletist-core --lib the_form_keeps_how`

Expected: PASS. Then the full check: fmt and clippy clean, every test binary `ok` (5426 tests pass in this state).

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-core/src/model.rs src/app.rs
git commit -m "Keep a Files form's reading of CSV, changed through the App"
```

---

### Task 4: The dialog's Reading CSV section, and Agents closed on Files

**Files:**
- Create: `src/ui/disclosure.rs`, `src/ui/connect_dialog/reading.rs`, `src/ui/reading_csv_tests.rs`
- Modify: `src/ui/mod.rs`, `src/ui/connect_dialog/{mod,sheet,terminal,agents,files}.rs`, `src/ui/files_tests.rs`, `crates/tabletist-core/src/keymap.rs`, `docs/_reference/connections.md`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -121,6 +121,8 @@
 pub mod quick_open;
 #[cfg(test)]
 mod quick_open_tests;
+#[cfg(test)]
+mod reading_csv_tests;
 pub mod review;
 pub mod row_form;
 pub mod row_panel;
diff --git a/src/ui/reading_csv_tests.rs b/src/ui/reading_csv_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/reading_csv_tests.rs
@@ -0,0 +1,367 @@
+//! A Files connection's Reading CSV section and its closed Agents row, in
+//! the connection dialog, in both looks: the rows open and close by click
+//! and by Enter, say their state and their summary, each option changes
+//! the form, Reset (and Omarchy's `R`) takes an override off, and Cancel
+//! keeps what was saved.
+
+use egui::accesskit::Role;
+use egui::{Key, Modifiers};
+use tabletist_db::ConnectSpec;
+use tabletist_db::files::{Change, Delimiter, Header, Override, Reading, Source, Types};
+
+use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
+use crate::env::Environment;
+use crate::model::{Action, ConnectionForm, Dialog};
+use crate::testing::{Harness, node};
+use crate::theme::Look;
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
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
+/// The closed Reading CSV row's name in `look`, for a reading with no
+/// override.
+fn closed(look: Look) -> String {
+    let types = match look.terminal {
+        true => "types first 20,000 rows",
+        false => "types from the first 20,000 rows",
+    };
+    look.label(&format!(
+        "Reading CSV, Header detect, delimiter detect, {types}"
+    ))
+}
+
+/// Whether the button named `name` is there, and open.
+fn expanded(harness: &mut Harness, name: &str) -> Option<bool> {
+    let tree = harness.settle();
+    let id = node(&tree, name, Role::Button)?;
+    tree.nodes
+        .iter()
+        .find(|(node, _)| *node == id)
+        .and_then(|(_, node)| node.is_expanded())
+}
+
+/// The label of what has the keyboard.
+fn focused_label(harness: &mut Harness) -> Option<String> {
+    let id = harness.ctx.memory(|memory| memory.focused())?;
+    let tree = harness.settle();
+    tree.nodes
+        .iter()
+        .find(|(node, _)| *node == id.accesskit_id())
+        .and_then(|(_, node)| node.label().map(str::to_owned))
+}
+
+/// Tabs until what has the keyboard is named `name`.
+fn tab_to(harness: &mut Harness, name: &str) {
+    for _ in 0..40 {
+        if focused_label(harness).as_deref() == Some(name) {
+            return;
+        }
+        harness.press(Key::Tab, Modifiers::NONE);
+    }
+    panic!("Tab never reached {name:?}");
+}
+
+fn returns_as_text() -> Override {
+    Override {
+        file: "returns.csv".into(),
+        change: Change::Text {
+            column: "returned_at".into(),
+        },
+    }
+}
+
+fn skip_archive() -> Override {
+    Override {
+        file: "archive.csv".into(),
+        change: Change::Skip,
+    }
+}
+
+#[test]
+fn reading_csv_is_a_closed_row_that_opens_by_click_and_by_enter() {
+    for look in looks() {
+        let mut harness = new_files(look);
+        let name = closed(look);
+        assert_eq!(expanded(&mut harness, &name), Some(false), "{}", look.name);
+        assert!(!harness.has(&look.label("Header row")), "{}", look.name);
+        harness.click(&name);
+        assert!(form(&harness).csv_open, "{}", look.name);
+        let title = look.label("Reading CSV");
+        assert_eq!(expanded(&mut harness, &title), Some(true), "{}", look.name);
+        for option in ["Header row", "Delimiter", "Column types"] {
+            assert!(harness.has(&look.label(option)), "{}: {option}", look.name);
+        }
+        let hint = match look.terminal {
+            true => "defaults for every csv",
+            false => "Defaults for every CSV in this connection",
+        };
+        assert!(harness.has(hint), "{}", look.name);
+        harness.click(&title);
+        assert!(!form(&harness).csv_open, "{}", look.name);
+        // The keyboard: a Tab stop, and Enter opens it and closes it.
+        tab_to(&mut harness, &name);
+        harness.press(Key::Enter, Modifiers::NONE);
+        assert!(form(&harness).csv_open, "{}: Enter opens", look.name);
+        // Tab goes from the heading into the section's choices.
+        tab_to(&mut harness, &title);
+        harness.press(Key::Tab, Modifiers::NONE);
+        assert_eq!(
+            focused_label(&mut harness).as_deref(),
+            Some(look.label("Detect").as_str()),
+            "{}: Tab into the section",
+            look.name
+        );
+        tab_to(&mut harness, &title);
+        harness.press(Key::Enter, Modifiers::NONE);
+        assert!(!form(&harness).csv_open, "{}: Enter closes", look.name);
+    }
+    // Closed again each time the dialog opens.
+    let mut harness = new_files(Look::macos());
+    form_mut(&mut harness).csv_open = true;
+    harness.app.apply(Action::CloseDialog);
+    harness.app.apply(Action::NewConnection);
+    harness.settle();
+    harness.click("Files");
+    assert!(!form(&harness).csv_open);
+}
+
+#[test]
+fn each_option_changes_how_the_form_reads_csv() {
+    for look in looks() {
+        let mut harness = new_files(look);
+        form_mut(&mut harness).csv_open = true;
+        harness.click(&look.label("Yes"));
+        assert_eq!(
+            form(&harness).files_csv.header,
+            Header::Yes,
+            "{}",
+            look.name
+        );
+        harness.click(";");
+        assert_eq!(
+            form(&harness).files_csv.delimiter,
+            Delimiter::Semicolon,
+            "{}",
+            look.name
+        );
+        harness.click(&look.label("Tab"));
+        assert_eq!(form(&harness).files_csv.delimiter, Delimiter::Tab);
+        match look.terminal {
+            true => harness.click("all text"),
+            false => {
+                harness.click("Column types");
+                harness.click("Read everything as text");
+            }
+        }
+        assert_eq!(form(&harness).files_csv.types, Types::Text, "{}", look.name);
+        // The closed row says them.
+        let title = look.label("Reading CSV");
+        harness.click(&title);
+        assert_eq!(
+            expanded(
+                &mut harness,
+                &look.label("Reading CSV, Header yes, delimiter tab, everything as text")
+            ),
+            Some(false),
+            "{}",
+            look.name
+        );
+        // And the connection saves them.
+        assert_eq!(
+            form(&harness).files_csv,
+            Reading {
+                header: Header::Yes,
+                delimiter: Delimiter::Tab,
+                types: Types::Text,
+                overrides: Vec::new(),
+            }
+        );
+    }
+}
+
+#[test]
+fn an_override_shows_under_the_options_and_reset_takes_it_off() {
+    for look in looks() {
+        let mut harness = new_files(look);
+        form_mut(&mut harness).files_csv.overrides = vec![returns_as_text(), skip_archive()];
+        let summary = closed(look).replace("rows", &look.label("rows, 2 overrides"));
+        assert_eq!(
+            expanded(&mut harness, &summary),
+            Some(false),
+            "{}",
+            look.name
+        );
+        form_mut(&mut harness).csv_open = true;
+        match look.terminal {
+            false => {
+                assert!(harness.has("returns.csv"));
+                assert!(harness.has("· returned_at read as text"));
+                assert!(harness.has("· skipped"));
+                assert!(harness.has("Reset archive.csv, skipped"));
+                harness.click("Reset returns.csv, returned_at read as text");
+            }
+            true => {
+                let row = "override returns.csv, returned_at as text";
+                assert!(node(&harness.settle(), row, Role::Button).is_some());
+                tab_to(&mut harness, row);
+                harness.press(Key::R, Modifiers::SHIFT);
+            }
+        }
+        assert_eq!(
+            form(&harness).files_csv.overrides,
+            [skip_archive()],
+            "{}",
+            look.name
+        );
+    }
+    // The file's own words keep their case in the terminal look.
+    let mut harness = new_files(Look::omarchy());
+    form_mut(&mut harness).files_csv.overrides = vec![Override {
+        file: "Returns.CSV".into(),
+        change: Change::Empty {
+            column: "Reason".into(),
+            values: vec!["N/A".into()],
+        },
+    }];
+    form_mut(&mut harness).csv_open = true;
+    let row = "override Returns.CSV, 'N/A' as empty in Reason";
+    assert!(node(&harness.settle(), row, Role::Button).is_some());
+    // `R` with nothing of the section's under the keyboard resets nothing.
+    let mut harness = new_files(Look::omarchy());
+    form_mut(&mut harness).files_csv.overrides = vec![returns_as_text()];
+    form_mut(&mut harness).csv_open = true;
+    harness.press(Key::R, Modifiers::SHIFT);
+    assert_eq!(form(&harness).files_csv.overrides, [returns_as_text()]);
+}
+
+#[test]
+fn a_skipped_file_is_left_out_of_the_found_line() {
+    let mut harness = new_files(Look::macos());
+    let scan = |names: &[&str]| tabletist_db::files::Scan {
+        files: names
+            .iter()
+            .map(|name| tabletist_db::files::Found {
+                path: format!("/data/bookshop-exports/{name}.csv").into(),
+                format: tabletist_db::files::Format::Csv,
+                size: 1024,
+                name: (*name).to_owned(),
+            })
+            .collect(),
+        missing: Vec::new(),
+    };
+    form_mut(&mut harness).files_found =
+        crate::model::FilesFound::Read(scan(&["orders", "returns"]));
+    assert!(harness.has("2 files · 2.0 KB · CSV"));
+    form_mut(&mut harness).files_csv.overrides = vec![Override {
+        file: "returns.csv".into(),
+        change: Change::Skip,
+    }];
+    assert!(harness.has("1 file · 1.0 KB · CSV"));
+    assert!(!harness.has("returns"));
+}
+
+#[test]
+fn cancel_keeps_how_the_saved_connection_reads_csv() {
+    let mut harness = Harness::new();
+    let mut spec = ConnectSpec::files(Source::Folder("/data/bookshop-exports".into()));
+    spec.csv.overrides = vec![returns_as_text()];
+    let saved = SavedConnection {
+        id: ConnectionId::new(),
+        name: "Bookshop exports".into(),
+        environment: Environment::Local,
+        read_only: Some(true),
+        password: PasswordMode::None,
+        ssh_secret: PasswordMode::None,
+        spec,
+        agents: Default::default(),
+    };
+    let id = saved.id.clone();
+    harness.app.connections.upsert(saved);
+    harness.app.apply(Action::EditConnection(id.clone()));
+    harness.settle();
+    assert_eq!(form(&harness).files_csv.overrides, [returns_as_text()]);
+    form_mut(&mut harness).csv_open = true;
+    harness.click("Reset returns.csv, returned_at read as text");
+    assert!(form(&harness).files_csv.overrides.is_empty());
+    harness.app.apply(Action::CloseDialog);
+    harness.settle();
+    assert_eq!(
+        harness.app.connections.get(&id).unwrap().spec.csv.overrides,
+        [returns_as_text()]
+    );
+}
+
+#[test]
+fn agents_on_files_is_a_closed_row_that_says_the_access() {
+    if !crate::endpoint::AVAILABLE {
+        return;
+    }
+    for look in looks() {
+        let mut harness = new_files(look);
+        let read = look.label("Agents, Read, agents can query the files, never change them");
+        assert_eq!(expanded(&mut harness, &read), Some(false), "{}", look.name);
+        assert!(
+            !harness.has(&look.label("Rows per answer")),
+            "{}",
+            look.name
+        );
+        harness.click(&read);
+        assert!(form(&harness).agents_open, "{}", look.name);
+        assert!(harness.has(&look.label("Rows per answer")), "{}", look.name);
+        harness.click(&look.label("Off"));
+        harness.click(&look.label("Agents"));
+        let off = look.label("Agents, Off, agents can't reach this connection");
+        assert_eq!(expanded(&mut harness, &off), Some(false), "{}", look.name);
+        assert_eq!(form(&harness).agents.access, AgentAccess::Off);
+    }
+    // The list menu's Agents… opens the row.
+    let mut harness = Harness::new();
+    let saved = SavedConnection {
+        id: ConnectionId::new(),
+        name: "Bookshop exports".into(),
+        environment: Environment::Local,
+        read_only: Some(true),
+        password: PasswordMode::None,
+        ssh_secret: PasswordMode::None,
+        spec: ConnectSpec::files(Source::Folder("/data/bookshop-exports".into())),
+        agents: Default::default(),
+    };
+    let id = saved.id.clone();
+    harness.app.connections.upsert(saved);
+    harness.app.apply(Action::EditAgents {
+        conn: id,
+        allow: false,
+    });
+    harness.settle();
+    assert!(form(&harness).agents_open);
+    assert!(harness.has("Rows per answer"));
+    // Every other type keeps its Agents part open, as before.
+    let mut harness = new_files(Look::macos());
+    harness.click("PostgreSQL");
+    assert!(harness.has("Rows per answer"));
+}
diff --git a/src/ui/files_tests.rs b/src/ui/files_tests.rs
--- a/src/ui/files_tests.rs
+++ b/src/ui/files_tests.rs
@@ -120,6 +120,8 @@
     harness.click("Local");
     assert_eq!(form(&harness).agents.access, AgentAccess::Read);
     // A choice of the person's own stays.
+    form_mut(&mut harness).agents_open = true;
+    harness.settle();
     harness.click("Off");
     harness.click("Dev");
     assert_eq!(form(&harness).agents.access, AgentAccess::Off);
@@ -518,6 +520,8 @@
         if !crate::endpoint::AVAILABLE {
             return;
         }
+        form_mut(&mut harness).agents_open = true;
+        harness.settle();
         assert!(
             says(
                 &mut harness,
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p tabletist --lib reading_csv_tests`

Expected: FAIL: there is no Reading CSV row to open, no Reset, and the Agents part is not a closed row. (The two run 1 tests in `files_tests.rs` open the Agents row first; they pass before and after this task.)

```text
thread 'ui::reading_csv_tests::an_override_shows_under_the_options_and_reset_takes_it_off' (2891650) panicked at src/ui/reading_csv_tests.rs:213:9:
thread 'ui::reading_csv_tests::each_option_changes_how_the_form_reads_csv' (2891653) panicked at src/testing.rs:298:32:
thread 'ui::reading_csv_tests::reading_csv_is_a_closed_row_that_opens_by_click_and_by_enter' (2891656) panicked at src/ui/reading_csv_tests.rs:110:9:
thread 'ui::reading_csv_tests::agents_on_files_is_a_closed_row_that_says_the_access' (2891649) panicked at src/ui/reading_csv_tests.rs:327:9:
thread 'ui::reading_csv_tests::a_skipped_file_is_left_out_of_the_found_line' (2891648) panicked at src/ui/reading_csv_tests.rs:284:5:
thread 'ui::reading_csv_tests::cancel_keeps_how_the_saved_connection_reads_csv' (2891652) panicked at src/testing.rs:298:32:
```

- [ ] **Step 3: Build the section, the shared component and the key**

```diff
diff --git a/crates/tabletist-core/src/keymap.rs b/crates/tabletist-core/src/keymap.rs
--- a/crates/tabletist-core/src/keymap.rs
+++ b/crates/tabletist-core/src/keymap.rs
@@ -751,6 +751,7 @@
     FormPick = "form_pick",
     FormAddFiles = "form_add_files",
     FormRemoveFile = "form_remove_file",
+    FormResetOverride = "form_reset_override",
     SettingsMove = "settings_move",
     SettingsChange = "settings_change",
     SettingsToggle = "settings_toggle",
@@ -1217,6 +1218,11 @@
                 "",
                 Prompts,
             ),
+            C::FormResetOverride => info(
+                "Reset the CSV override under the cursor of a Files connection",
+                "",
+                Prompts,
+            ),
             C::SettingsMove => info("Move in the settings", "", Prompts),
             C::SettingsChange => info("Change the option", "", Prompts),
             C::SettingsToggle => info("Toggle the option", "", Prompts),
@@ -2703,6 +2709,7 @@
         bind(C::FormPick, PROMPT, NONE, keys(&[k("ctrl+o")])).when(When::ConnectionForm),
         bind(C::FormAddFiles, PROMPT, NONE, keys(&[k("a")])).when(When::ConnectionForm),
         bind(C::FormRemoveFile, PROMPT, NONE, keys(&[k("x")])).when(When::ConnectionForm),
+        bind(C::FormResetOverride, PROMPT, NONE, keys(&[k("R")])).when(When::ConnectionForm),
         bind(
             C::SettingsMove,
             PROMPT,
diff --git a/src/ui/disclosure.rs b/src/ui/disclosure.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/disclosure.rs
@@ -0,0 +1,273 @@
+//! A section that opens and closes, as the Components board's disclosure
+//! row draws it: closed, a row of its title and a one-line summary; open,
+//! its body under a heading that closes it again.
+//!
+//! macOS: closed, a row 40 high in a hairline box with corners of 8: `›`,
+//! the title in bold in a column 190 wide, then the summary, cut to fit.
+//! Open, the box the dialog's groups are drawn in, its legend `⌄` and the
+//! title, the hint the first line inside it.
+//!
+//! The terminal look: a heading over a rule, `▸ title  summary · enter
+//! opens` closed and `▾ title  hint` open, with the bar at its left while
+//! it has the keyboard.
+//!
+//! Either is a button whose expanded state is the section's, named with
+//! its title and, closed, the summary: "Reading CSV, Header detect, …".
+
+use egui::{
+    Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType,
+    pos2, vec2,
+};
+
+use crate::style::Icon;
+use crate::theme::{Look, Palette};
+use crate::typography::{Text, TextRole};
+use crate::ui::focus::{self, Ring};
+use crate::ui::widgets;
+
+/// The closed row's height.
+const ROW: f32 = 40.0;
+/// The room at the row's sides.
+const INSET: f32 = 14.0;
+/// The closed row's title column.
+const TITLE: f32 = 190.0;
+/// The chevron's size, and the room after it.
+const CHEVRON: f32 = 10.0;
+const GAP: f32 = 12.0;
+
+/// What a section says of itself.
+pub struct Section<'a> {
+    /// Tells the section from the others in its `Ui`.
+    pub salt: &'a str,
+    pub title: &'a str,
+    /// What the closed row says after the title.
+    pub summary: &'a str,
+    /// What the open heading says after the title; may be empty.
+    pub hint: &'a str,
+}
+
+impl Section<'_> {
+    /// The name a screen reader hears: the title, and closed, the summary
+    /// as words after a comma.
+    pub fn name(&self, open: bool) -> String {
+        match open || self.summary.is_empty() {
+            true => self.title.to_owned(),
+            false => format!("{}, {}", self.title, self.summary.replace(" · ", ", ")),
+        }
+    }
+}
+
+/// The section: its closed row, or its heading and what `add` draws under
+/// it. A press of the row or the heading, by the pointer or by Enter or
+/// Space while it has the keyboard, opens or closes it. `fill` is what the
+/// open box sits on, which its legend breaks the edge with, and `rule` the
+/// colour of its edges. Returns the row's or the heading's response.
+#[allow(clippy::too_many_arguments)]
+pub fn show(
+    ui: &mut Ui,
+    section: &Section<'_>,
+    open: &mut bool,
+    look: &Look,
+    palette: &Palette,
+    fill: Color32,
+    rule: Color32,
+    add: impl FnOnce(&mut Ui),
+) -> Response {
+    if look.terminal {
+        let response = terminal_heading(ui, section, *open, look, palette);
+        if response.clicked() {
+            *open = !*open;
+        }
+        if *open {
+            add(ui);
+        }
+        return response;
+    }
+    if !*open {
+        let response = row(ui, section, look, palette, rule);
+        if response.clicked() {
+            *open = true;
+        }
+        return response;
+    }
+    open_box(ui, section, open, look, palette, fill, rule, add)
+}
+
+/// Says the toggle is a button named `name`, open or closed.
+fn announce(response: &Response, name: &str, open: bool) {
+    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, name));
+    response
+        .ctx
+        .accesskit_node_builder(response.id, |node| node.set_expanded(open));
+}
+
+/// macOS: the closed row.
+fn row(
+    ui: &mut Ui,
+    section: &Section<'_>,
+    look: &Look,
+    palette: &Palette,
+    rule: Color32,
+) -> Response {
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
+    let id = ui.id().with(("disclosure", section.salt));
+    let response = ui.interact(rect, id, Sense::click());
+    announce(&response, &section.name(false), false);
+    if ui.is_rect_visible(rect) {
+        let painter = ui.painter();
+        let ground = match response.hovered() {
+            true => palette.surface_hover,
+            false => Color32::TRANSPARENT,
+        };
+        painter.rect(
+            rect,
+            CornerRadius::same(8),
+            ground,
+            Stroke::new(widgets::hairline(ui), rule),
+            StrokeKind::Inside,
+        );
+        let middle = rect.center().y;
+        let chevron = Rect::from_center_size(
+            pos2(rect.left() + INSET + CHEVRON / 2.0, middle),
+            vec2(CHEVRON, CHEVRON),
+        );
+        Icon::ChevronRight
+            .image(palette.dim, CHEVRON)
+            .paint_at(ui, chevron);
+        let title_x = chevron.right() + GAP;
+        Text::one(look, TextRole::UiBodySemibold, section.title, palette.text)
+            .layout(ui.ctx())
+            .paint_left(painter, title_x, middle);
+        let summary_x = title_x + TITLE + GAP;
+        let room = rect.right() - INSET - summary_x;
+        if room > 0.0 && !section.summary.is_empty() {
+            let mut text = Text::one(look, TextRole::UiBody, section.summary, palette.secondary);
+            text.job_mut().wrap = egui::text::TextWrapping::truncate_at_width(room);
+            text.layout(ui.ctx()).paint_left(painter, summary_x, middle);
+        }
+    }
+    focus::hint(ui, &response, rect, Ring::Outer { radius: 8 });
+    response
+}
+
+/// macOS: the open box, its legend the toggle that closes it.
+#[allow(clippy::too_many_arguments)]
+fn open_box(
+    ui: &mut Ui,
+    section: &Section<'_>,
+    open: &mut bool,
+    look: &Look,
+    palette: &Palette,
+    fill: Color32,
+    rule: Color32,
+    add: impl FnOnce(&mut Ui),
+) -> Response {
+    let legend =
+        Text::one(look, TextRole::Legend, section.title, palette.secondary).layout(ui.ctx());
+    let half = legend.height() / 2.0;
+    ui.add_space(half);
+    // The legend breaks the edge, `⌄` before its words, 6 of `fill` at
+    // each side. It takes the keyboard before what the box holds, so Tab
+    // goes from it into the section.
+    let start = ui.cursor().min;
+    let width = CHEVRON + 6.0 + legend.width();
+    let gap = Rect::from_min_size(
+        pos2(start.x + 15.0, start.y - half),
+        vec2(width + 12.0, legend.height()),
+    );
+    let id = ui.id().with(("disclosure", section.salt));
+    let response = ui.interact(gap, id, Sense::click());
+    announce(&response, &section.name(true), true);
+    let border = ui.painter().add(egui::Shape::Noop);
+    let rect = egui::Frame::new()
+        .inner_margin(egui::Margin {
+            left: 15,
+            right: 15,
+            top: (half + 15.0).round() as i8,
+            bottom: 13,
+        })
+        .show(ui, |ui| {
+            ui.set_width(ui.available_width());
+            if !section.hint.is_empty() {
+                // The hint, under the legend, 4 above what follows.
+                ui.add_space(-4.0);
+                widgets::label(ui, TextRole::Secondary, section.hint, palette.dim, look);
+                ui.add_space(4.0);
+            }
+            add(ui);
+        })
+        .response
+        .rect;
+    ui.painter().set(
+        border,
+        egui::epaint::RectShape::stroke(
+            rect,
+            CornerRadius::same(10),
+            Stroke::new(widgets::hairline(ui), rule),
+            StrokeKind::Inside,
+        ),
+    );
+    let painter = ui.painter();
+    painter.rect_filled(gap, CornerRadius::ZERO, fill);
+    let chevron = Rect::from_center_size(
+        pos2(gap.left() + 6.0 + CHEVRON / 2.0, gap.center().y),
+        vec2(CHEVRON, CHEVRON),
+    );
+    let tint = match response.hovered() {
+        true => palette.text,
+        false => palette.dim,
+    };
+    Icon::ChevronDown.image(tint, CHEVRON).paint_at(ui, chevron);
+    legend.paint(painter, pos2(chevron.right() + 6.0, gap.top()));
+    focus::hint(ui, &response, gap, Ring::Outer { radius: 3 });
+    if response.clicked() {
+        *open = false;
+    }
+    response
+}
+
+/// The terminal look: the heading, closed or open.
+fn terminal_heading(
+    ui: &mut Ui,
+    section: &Section<'_>,
+    open: bool,
+    look: &Look,
+    palette: &Palette,
+) -> Response {
+    let (mark, after) = match open {
+        true => ("▾ ", section.hint.to_owned()),
+        false if section.summary.is_empty() => ("▸ ", "enter opens".to_owned()),
+        false => ("▸ ", format!("{} · enter opens", section.summary)),
+    };
+    let role = TextRole::OBody;
+    let mut text = Text::new(look)
+        .add(TextRole::OGroup, mark, palette.dim)
+        .add(TextRole::OGroup, section.title, palette.text);
+    if !after.is_empty() {
+        text = text.space(role, "  ").add(role, &after, palette.dim);
+    }
+    let width = ui.available_width();
+    text.job_mut().wrap = egui::text::TextWrapping::truncate_at_width(width);
+    let laid = text.layout(ui.ctx());
+    ui.add_space(8.0);
+    let height = laid.height() + 4.0;
+    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
+    // The hint, which the heading's name leaves out, for a screen reader:
+    // under the toggle, which takes the press.
+    if open && !section.hint.is_empty() {
+        widgets::announce(ui, rect, section.hint);
+    }
+    let id = ui.id().with(("disclosure", section.salt));
+    let response = ui.interact(rect, id, Sense::click());
+    announce(&response, &section.name(open), open);
+    if focus::shown(&response) {
+        ui.painter()
+            .rect_filled(rect, CornerRadius::ZERO, palette.selection);
+    }
+    laid.paint_left(ui.painter(), rect.left(), rect.top() + laid.height() / 2.0);
+    let (line, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
+    widgets::hline(ui, line.x_range(), line.center().y, palette.outline);
+    focus::hint(ui, &response, rect, Ring::Bar);
+    ui.add_space(8.0);
+    response
+}
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -46,6 +46,7 @@
 mod delete_tests;
 #[cfg(test)]
 mod dialog_focus_tests;
+pub mod disclosure;
 #[cfg(test)]
 mod duplicate_tests;
 #[cfg(test)]
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -6,6 +6,7 @@
 mod agents;
 mod choice;
 mod files;
+mod reading;
 mod sheet;
 mod terminal;
 
@@ -438,6 +439,9 @@
     }
     if modal.is_top_modal && !modal.any_popup_open {
         let typing = ctx.text_edit_focused();
+        // The override whose row has the keyboard, which `R` resets (the
+        // terminal look's key).
+        let reset = reading::focused_override(ctx, form);
         let toggle_url = ctx.input_mut(|input| {
             // The form's cancel closes the dialog, unless it is closing an
             // open list first.
@@ -476,6 +480,12 @@
                         }
                         Some(Command::FormAddFiles) if listed => {
                             actions.push(Action::PickDataFiles);
+                            false
+                        }
+                        Some(Command::FormResetOverride) if files => {
+                            if let Some(at) = reset {
+                                actions.push(Action::SetCsv(crate::model::CsvChange::Reset(at)));
+                            }
                             false
                         }
                         Some(Command::FormRemoveFile) if listed => {
diff --git a/src/ui/connect_dialog/reading.rs b/src/ui/connect_dialog/reading.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/connect_dialog/reading.rs
@@ -0,0 +1,453 @@
+//! A Files connection's Reading CSV section, in either look: how its CSV
+//! and TSV files are read (header row, delimiter, column types), and the
+//! overrides of one file, each with Reset. Closed, it says the choices on
+//! one line.
+
+use egui::{CornerRadius, Sense, Ui, vec2};
+use tabletist_db::files::{Change, Delimiter, Header, Override, Reading, Types};
+
+use super::Skin;
+use super::choice::{Choice, Group, choose};
+use crate::i18n::gettext;
+use crate::model::{Action, ConnectionForm, CsvChange};
+use crate::typography::{Text, TextRole};
+use crate::ui::disclosure::{self, Section};
+use crate::ui::{focus, menu, widgets};
+
+/// What the open section says under its title: macOS, then the terminal
+/// look's shorter words, as its board writes them.
+const HINT: &str = "Defaults for every CSV in this connection";
+const TERMINAL_HINT: &str = "Defaults for every CSV";
+
+/// The width of the macOS options' labels.
+const LABEL: f32 = 110.0;
+
+/// The id of the override at `at`'s row in the terminal look: what `R`
+/// resets while it has the keyboard.
+pub(super) fn override_id(at: usize) -> egui::Id {
+    egui::Id::new(("csv-override", at))
+}
+
+/// The override whose row has the keyboard (the terminal look).
+pub(super) fn focused_override(ctx: &egui::Context, form: &ConnectionForm) -> Option<usize> {
+    let focused = ctx.memory(|memory| memory.focused())?;
+    (0..form.files_csv.overrides.len()).find(|at| override_id(*at) == focused)
+}
+
+/// The closed row's summary: each choice as a few words, then how many
+/// overrides there are. `terminal` says the types as its choices do.
+pub(super) fn summary(reading: &Reading, terminal: bool) -> String {
+    let header = match reading.header {
+        Header::Detect => "Header detect",
+        Header::Yes => "Header yes",
+        Header::No => "Header no",
+    };
+    let delimiter = match reading.delimiter {
+        Delimiter::Detect => "delimiter detect",
+        Delimiter::Comma => "delimiter ,",
+        Delimiter::Semicolon => "delimiter ;",
+        Delimiter::Tab => "delimiter tab",
+        Delimiter::Pipe => "delimiter |",
+    };
+    let types = match (reading.types, terminal) {
+        (Types::Sample, false) => "types from the first 20,000 rows",
+        (Types::Sample, true) => "types first 20,000 rows",
+        (Types::All, false) => "types from all rows",
+        (Types::All, true) => "types all rows",
+        (Types::Text, _) => "everything as text",
+    };
+    let mut said = format!("{header} · {delimiter} · {types}");
+    match reading.overrides.len() {
+        0 => {}
+        1 => said.push_str(" · 1 override"),
+        many => said.push_str(&format!(" · {many} overrides")),
+    }
+    said
+}
+
+/// What an override's row says after the file's name: "returned_at read
+/// as text" (macOS), "returned_at as text" (Omarchy). The words are in
+/// either look's case already: the column and the values are the file's,
+/// never lowercased.
+pub(super) fn override_words(change: &Change, terminal: bool) -> String {
+    let read = if terminal { "" } else { " read" };
+    match change {
+        Change::Text { column } => format!("{column}{read} as text"),
+        Change::Empty { column, values } => {
+            let values: Vec<String> = values.iter().map(|value| format!("'{value}'")).collect();
+            format!("{}{read} as empty in {column}", values.join(", "))
+        }
+        Change::Skip => "skipped".to_owned(),
+    }
+}
+
+/// The header row's choices.
+fn headers(skin: &Skin) -> Vec<(Header, String)> {
+    let words = ["Detect", "Yes", "No"];
+    Header::ALL
+        .into_iter()
+        .zip(words)
+        .map(|(header, word)| (header, skin.say(word)))
+        .collect()
+}
+
+/// The delimiter's choices.
+fn delimiters(skin: &Skin) -> Vec<(Delimiter, String)> {
+    Delimiter::ALL
+        .into_iter()
+        .map(|delimiter| {
+            let word = match delimiter {
+                Delimiter::Detect => skin.say("Detect"),
+                Delimiter::Tab => skin.say("Tab"),
+                Delimiter::Comma | Delimiter::Semicolon | Delimiter::Pipe => {
+                    delimiter.char().map(String::from).unwrap_or_default()
+                }
+            };
+            (delimiter, word)
+        })
+        .collect()
+}
+
+/// The column types' choices: the dropdown's words on macOS, the choice
+/// row's in the terminal look.
+fn types(skin: &Skin) -> Vec<(Types, String)> {
+    let words: [&'static str; 3] = match skin.look.terminal {
+        true => ["First 20,000 rows", "All rows", "All text"],
+        false => [
+            "Detect from the first 20,000 rows",
+            "Detect from all rows (slower)",
+            "Read everything as text",
+        ],
+    };
+    Types::ALL
+        .into_iter()
+        .zip(words)
+        .map(|(types, word)| (types, skin.say(word)))
+        .collect()
+}
+
+/// A choice among `options`, as a group named `name`: the change it makes.
+fn choice<T: Copy + PartialEq>(
+    ui: &mut Ui,
+    salt: &str,
+    name: &str,
+    options: &[(T, String)],
+    current: T,
+    group: Group,
+    skin: &Skin,
+) -> Option<T> {
+    let options: Vec<(T, Choice<'_>)> = options
+        .iter()
+        .map(|(value, word)| (*value, Choice::plain(word)))
+        .collect();
+    choose(ui, salt, Some(name), &options, current, group, skin)
+}
+
+/// The section after Source, in either look.
+pub(super) fn section(
+    ui: &mut Ui,
+    form: &mut ConnectionForm,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Skin { look, palette, .. } = *skin;
+    let title = skin.say("Reading CSV");
+    let summary = look.label(&summary(&form.files_csv, look.terminal));
+    let hint = skin.say(match look.terminal {
+        true => TERMINAL_HINT,
+        false => HINT,
+    });
+    let section = Section {
+        salt: "reading-csv",
+        title: &title,
+        summary: &summary,
+        hint: &hint,
+    };
+    let mut open = form.csv_open;
+    disclosure::show(
+        ui,
+        &section,
+        &mut open,
+        look,
+        palette,
+        skin.fill,
+        skin.rule(),
+        |ui| match look.terminal {
+            true => terminal_rows(ui, form, skin, actions),
+            false => sheet_rows(ui, form, skin, actions),
+        },
+    );
+    form.csv_open = open;
+}
+
+/// macOS: a label in the options' column, then what `add` draws.
+fn option_row(ui: &mut Ui, label: &str, height: f32, skin: &Skin, add: impl FnOnce(&mut Ui)) {
+    let Skin { look, palette, .. } = *skin;
+    let center = egui::Layout::left_to_right(egui::Align::Center);
+    ui.allocate_ui_with_layout(vec2(ui.available_width(), height), center, |ui| {
+        ui.allocate_ui_with_layout(vec2(LABEL, height), center, |ui| {
+            ui.set_min_width(LABEL);
+            widgets::label(ui, TextRole::Secondary, label, palette.secondary, look);
+        });
+        add(ui);
+    });
+}
+
+/// macOS: the open section's options and overrides.
+fn sheet_rows(ui: &mut Ui, form: &ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
+    let reading = &form.files_csv;
+    let group = Group::Track(24.0);
+    let header = skin.say("Header row");
+    option_row(ui, &header, group.height(), skin, |ui| {
+        if let Some(chosen) = choice(
+            ui,
+            "csv-header",
+            &header,
+            &headers(skin),
+            reading.header,
+            group,
+            skin,
+        ) {
+            actions.push(Action::SetCsv(CsvChange::Header(chosen)));
+        }
+    });
+    ui.add_space(8.0);
+    let delimiter = skin.say("Delimiter");
+    option_row(ui, &delimiter, group.height(), skin, |ui| {
+        if let Some(chosen) = choice(
+            ui,
+            "csv-delimiter",
+            &delimiter,
+            &delimiters(skin),
+            reading.delimiter,
+            group,
+            skin,
+        ) {
+            actions.push(Action::SetCsv(CsvChange::Delimiter(chosen)));
+        }
+    });
+    ui.add_space(8.0);
+    let label = skin.say("Column types");
+    option_row(ui, &label, skin.field_height(), skin, |ui| {
+        let options = types(skin);
+        let shown = options
+            .iter()
+            .find(|(types, _)| *types == reading.types)
+            .map(|(_, word)| word.as_str())
+            .unwrap_or_default();
+        let picked = super::sheet::select(ui, "csv-types", shown, 280.0, skin).show(
+            ui,
+            skin.look,
+            skin.palette,
+            |ui| {
+                let mut chosen = reading.types;
+                for (types, word) in &options {
+                    menu::value(ui, &mut chosen, *types, word, skin.look, skin.palette);
+                }
+                chosen
+            },
+        );
+        picked.response.widget_info(|| {
+            let mut info = egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, &label);
+            info.current_text_value = Some(shown.to_owned());
+            info
+        });
+        if let Some(chosen) = picked.inner
+            && chosen != reading.types
+        {
+            actions.push(Action::SetCsv(CsvChange::Types(chosen)));
+        }
+    });
+    for (at, each) in reading.overrides.iter().enumerate() {
+        ui.add_space(8.0);
+        sheet_override(ui, at, each, skin, actions);
+    }
+}
+
+/// macOS: an override's row, on the sunken tone: the file's name, what
+/// changes, and Reset at its end.
+fn sheet_override(ui: &mut Ui, at: usize, each: &Override, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin { look, palette, .. } = *skin;
+    let words = format!("· {}", override_words(&each.change, false));
+    egui::Frame::new()
+        .fill(palette.panel)
+        .corner_radius(CornerRadius::same(8))
+        .inner_margin(egui::Margin::symmetric(10, 8))
+        .show(ui, |ui| {
+            ui.set_width(ui.available_width());
+            ui.horizontal(|ui| {
+                ui.spacing_mut().item_spacing.x = 6.0;
+                widgets::label(
+                    ui,
+                    TextRole::MonoSecondary,
+                    &each.file,
+                    palette.secondary,
+                    look,
+                );
+                widgets::label(ui, TextRole::Secondary, &words, palette.secondary, look);
+                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
+                    let name = format!(
+                        "{} {}, {}",
+                        gettext(skin.locale, "Reset"),
+                        each.file,
+                        override_words(&each.change, false)
+                    );
+                    let reset = reset_link(ui, at, &skin.say("Reset"), &name, skin);
+                    if reset.clicked() {
+                        actions.push(Action::SetCsv(CsvChange::Reset(at)));
+                    }
+                });
+            });
+        });
+}
+
+/// macOS: Reset, as a link that says `text`, named `name`: the override
+/// it resets.
+fn reset_link(ui: &mut Ui, at: usize, text: &str, name: &str, skin: &Skin) -> egui::Response {
+    let Skin { look, palette, .. } = *skin;
+    let laid = Text::one(look, TextRole::Secondary, text, palette.accent).layout(ui.ctx());
+    let (rect, _) = ui.allocate_exact_size(laid.size(), Sense::hover());
+    let response = ui.interact(rect, ui.id().with(("csv-reset", at)), Sense::click());
+    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Link, true, name));
+    laid.paint(ui.painter(), rect.min);
+    focus::hint(ui, &response, rect, focus::Ring::Outer { radius: 3 });
+    response.on_hover_cursor(egui::CursorIcon::PointingHand)
+}
+
+/// The terminal look: the open section's rows.
+fn terminal_rows(ui: &mut Ui, form: &ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin { look, palette, .. } = *skin;
+    let reading = &form.files_csv;
+    let group = Group::Words;
+    let header = skin.say("Header row");
+    super::terminal::terminal_row_of(ui, &header, group.height(), skin, |ui, _| {
+        if let Some(chosen) = choice(
+            ui,
+            "csv-header",
+            &header,
+            &headers(skin),
+            reading.header,
+            group,
+            skin,
+        ) {
+            actions.push(Action::SetCsv(CsvChange::Header(chosen)));
+        }
+    });
+    let delimiter = skin.say("Delimiter");
+    super::terminal::terminal_row_of(ui, &delimiter, group.height(), skin, |ui, _| {
+        if let Some(chosen) = choice(
+            ui,
+            "csv-delimiter",
+            &delimiter,
+            &delimiters(skin),
+            reading.delimiter,
+            group,
+            skin,
+        ) {
+            actions.push(Action::SetCsv(CsvChange::Delimiter(chosen)));
+        }
+    });
+    let label = skin.say("Column types");
+    super::terminal::terminal_row_of(ui, &label, group.height(), skin, |ui, _| {
+        if let Some(chosen) = choice(
+            ui,
+            "csv-types",
+            &label,
+            &types(skin),
+            reading.types,
+            group,
+            skin,
+        ) {
+            actions.push(Action::SetCsv(CsvChange::Types(chosen)));
+        }
+    });
+    let line = TextRole::OBody.row_height(ui.ctx(), look.faces);
+    let reset = crate::ui::keys::written(ui.ctx(), look, crate::keymap::Command::FormResetOverride);
+    for (at, each) in reading.overrides.iter().enumerate() {
+        let said = format!(
+            "{} · {} · {} {}",
+            each.file,
+            override_words(&each.change, true),
+            reset.as_str(),
+            skin.say("reset")
+        );
+        super::terminal::terminal_row_of(ui, &skin.say("Override"), line, skin, |ui, _| {
+            let laid = Text::one(look, TextRole::OBody, &said, palette.dim).layout(ui.ctx());
+            let (rect, _) =
+                ui.allocate_exact_size(vec2(ui.available_width(), line), Sense::hover());
+            let response = ui.interact(rect, override_id(at), Sense::click());
+            let name = format!(
+                "{} {}, {}",
+                skin.say("Override"),
+                each.file,
+                override_words(&each.change, true)
+            );
+            response
+                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
+            if focus::shown(&response) {
+                ui.painter()
+                    .rect_filled(rect, CornerRadius::ZERO, palette.selection);
+            }
+            laid.paint_left(ui.painter(), rect.left(), rect.center().y);
+            focus::hint(ui, &response, rect, focus::Ring::Bar);
+        });
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn the_summary_says_each_choice_and_the_overrides() {
+        let mut reading = Reading::default();
+        assert_eq!(
+            summary(&reading, false),
+            "Header detect · delimiter detect · types from the first 20,000 rows"
+        );
+        assert_eq!(
+            summary(&reading, true),
+            "Header detect · delimiter detect · types first 20,000 rows"
+        );
+        reading.header = Header::Yes;
+        reading.delimiter = Delimiter::Tab;
+        reading.types = Types::Text;
+        reading.overrides.push(Override {
+            file: "returns.csv".into(),
+            change: Change::Skip,
+        });
+        assert_eq!(
+            summary(&reading, false),
+            "Header yes · delimiter tab · everything as text · 1 override"
+        );
+        reading.types = Types::All;
+        reading.delimiter = Delimiter::Pipe;
+        reading.overrides.push(reading.overrides[0].clone());
+        assert_eq!(
+            summary(&reading, false),
+            "Header yes · delimiter | · types from all rows · 2 overrides"
+        );
+    }
+
+    #[test]
+    fn an_override_says_what_it_changes() {
+        let text = Change::Text {
+            column: "returned_at".into(),
+        };
+        let empty = Change::Empty {
+            column: "returned_at".into(),
+            values: vec!["N/A".into(), "-".into()],
+        };
+        assert_eq!(override_words(&text, false), "returned_at read as text");
+        assert_eq!(override_words(&text, true), "returned_at as text");
+        assert_eq!(
+            override_words(&empty, false),
+            "'N/A', '-' read as empty in returned_at"
+        );
+        assert_eq!(
+            override_words(&empty, true),
+            "'N/A', '-' as empty in returned_at"
+        );
+        assert_eq!(override_words(&Change::Skip, false), "skipped");
+    }
+}
diff --git a/src/ui/connect_dialog/sheet.rs b/src/ui/connect_dialog/sheet.rs
--- a/src/ui/connect_dialog/sheet.rs
+++ b/src/ui/connect_dialog/sheet.rs
@@ -190,7 +190,7 @@
 /// macOS: a dropdown `width` wide that reads `text`, as tall as the
 /// dialog's fields and filled as they are. It relies on
 /// [`super::style_controls`] having set that fill on `ui`.
-fn select<'a>(
+pub(super) fn select<'a>(
     ui: &Ui,
     salt: &'static str,
     text: &'a str,
@@ -281,6 +281,8 @@
                         fieldset(ui, &skin.say("Source"), skin, |ui| {
                             super::files::sheet(ui, form, skin, actions);
                         });
+                        ui.add_space(12.0);
+                        super::reading::section(ui, form, skin, actions);
                     }
                     Driver::Postgres | Driver::MySql => {
                         fieldset(ui, &skin.say("Server"), skin, |ui| {
diff --git a/src/ui/connect_dialog/terminal.rs b/src/ui/connect_dialog/terminal.rs
--- a/src/ui/connect_dialog/terminal.rs
+++ b/src/ui/connect_dialog/terminal.rs
@@ -346,7 +346,10 @@
                     terminal_section(ui, &skin.say("BigQuery"), skin);
                     terminal_bigquery(ui, form, skin);
                 }
-                Driver::Files => super::files::terminal(ui, form, skin, actions),
+                Driver::Files => {
+                    super::files::terminal(ui, form, skin, actions);
+                    super::reading::section(ui, form, skin, actions);
+                }
                 Driver::Postgres | Driver::MySql => {
                     terminal_section(ui, &skin.say("Server"), skin);
                     terminal_server(ui, form, skin, actions);
diff --git a/src/ui/connect_dialog/agents.rs b/src/ui/connect_dialog/agents.rs
--- a/src/ui/connect_dialog/agents.rs
+++ b/src/ui/connect_dialog/agents.rs
@@ -14,8 +14,10 @@
 use tabletist_db::Driver;
 
 use crate::connections::AgentAccess;
+use crate::i18n::gettext;
 use crate::model::{Action, ConnectionForm, TableFocus, TablesMode};
 use crate::typography::{Text, TextRole};
+use crate::ui::disclosure::{self, Section};
 use crate::ui::widgets::{self, ButtonSpec};
 
 /// The levels, as the switch says them.
@@ -901,7 +903,9 @@
     }
 }
 
-/// macOS: the "Agents" fieldset after Safety.
+/// macOS: the "Agents" fieldset after Safety; on a Files form, a section
+/// that opens and closes, closed until it is opened or the dialog is asked
+/// to show it.
 pub(super) fn sheet(
     ui: &mut Ui,
     form: &mut ConnectionForm,
@@ -910,82 +914,160 @@
 ) {
     let Skin { look, palette, .. } = *skin;
     let top = ui.cursor().top();
-    super::sheet::fieldset(ui, &skin.say("Agents"), skin, |ui| {
-        let small = TextRole::Secondary;
-        let heading = skin.say("Access through Tabletist (MCP)");
-        widgets::label(ui, small, &heading, palette.secondary, look);
+    let title = skin.say("Agents");
+    if guards_tables(form) {
+        super::sheet::fieldset(ui, &title, skin, |ui| {
+            sheet_rows(ui, form, skin, actions);
+        });
+    } else {
+        if form.show_agents {
+            form.agents_open = true;
+        }
+        let summary = look.label(&gettext(skin.locale, closed_summary(form)));
+        let section = Section {
+            salt: "agents",
+            title: &title,
+            summary: &summary,
+            hint: "",
+        };
+        let mut open = form.agents_open;
+        disclosure::show(
+            ui,
+            &section,
+            &mut open,
+            look,
+            palette,
+            skin.fill,
+            skin.rule(),
+            |ui| {
+                sheet_rows(ui, form, skin, actions);
+            },
+        );
+        form.agents_open = open;
+    }
+    shown(ui, form, top);
+}
+
+/// What a Files form's closed Agents row says.
+fn closed_summary(form: &ConnectionForm) -> &'static str {
+    match form.agents.access {
+        AgentAccess::Read => "Read · agents can query the files, never change them",
+        AgentAccess::Off => "Off · agents can't reach this connection",
+    }
+}
+
+/// macOS: what the Agents part holds.
+fn sheet_rows(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin { look, palette, .. } = *skin;
+    let small = TextRole::Secondary;
+    let heading = skin.say("Access through Tabletist (MCP)");
+    widgets::label(ui, small, &heading, palette.secondary, look);
+    ui.add_space(5.0);
+    access(ui, form, &heading, Group::Track(28.0), skin, actions);
+    if let Some(said) = runs_as(skin) {
+        ui.add_space(6.0);
+        sentence(ui, small, &said, skin);
+    }
+    ui.add_space(12.0);
+    // Tables and Masked columns side by side, as the board has them.
+    let columns = [super::Column::Fr(1.0), super::Column::Fr(1.0)];
+    let guards = guards_tables(form);
+    super::row(ui, 12.0, &columns, |ui, column| match column {
+        _ if !guards => {}
+        0 => {
+            let tables = skin.say("Tables");
+            widgets::label(ui, small, &tables, palette.secondary, look);
+            ui.add_space(5.0);
+            tables_mode(ui, form, &tables, Group::Track(28.0), skin);
+            if form.agents_tables.names().is_some() {
+                ui.add_space(6.0);
+                table_names(ui, form, skin);
+                table_error(ui, form, skin);
+                table_missing(ui, form, skin);
+            }
+            if let Some(hint) = tables_hint(form) {
+                ui.add_space(6.0);
+                sentence(ui, small, &skin.say(hint), skin);
+            }
+        }
+        _ => {
+            let masked = skin.say("Masked columns");
+            widgets::label(ui, small, &masked, palette.secondary, look);
+            ui.add_space(5.0);
+            column_names(ui, form, skin);
+            column_error(ui, form, skin);
+            column_missing(ui, form, skin);
+            ui.add_space(6.0);
+            sentence(ui, small, &skin.say(MASKED_HINT), skin);
+        }
+    });
+    if guards {
+        ui.add_space(12.0);
+    }
+    let (role, height) = (skin.value_role(), skin.field_height());
+    super::row(ui, 12.0, &columns, |ui, column| {
+        let (label, text) = match column {
+            0 => (skin.say("Rows per answer"), &mut form.agents_rows),
+            _ => (
+                skin.say("Statement timeout, seconds"),
+                &mut form.agents_timeout,
+            ),
+        };
+        let label = widgets::label(ui, small, &label, palette.secondary, look);
         ui.add_space(5.0);
-        access(ui, form, &heading, Group::Track(28.0), skin, actions);
-        if let Some(said) = runs_as(skin) {
-            ui.add_space(6.0);
-            sentence(ui, small, &said, skin);
-        }
-        ui.add_space(12.0);
-        // Tables and Masked columns side by side, as the board has them.
-        let columns = [super::Column::Fr(1.0), super::Column::Fr(1.0)];
-        let guards = guards_tables(form);
-        super::row(ui, 12.0, &columns, |ui, column| match column {
-            _ if !guards => {}
-            0 => {
-                let tables = skin.say("Tables");
-                widgets::label(ui, small, &tables, palette.secondary, look);
-                ui.add_space(5.0);
-                tables_mode(ui, form, &tables, Group::Track(28.0), skin);
-                if form.agents_tables.names().is_some() {
-                    ui.add_space(6.0);
-                    table_names(ui, form, skin);
-                    table_error(ui, form, skin);
-                    table_missing(ui, form, skin);
-                }
-                if let Some(hint) = tables_hint(form) {
-                    ui.add_space(6.0);
-                    sentence(ui, small, &skin.say(hint), skin);
-                }
-            }
-            _ => {
-                let masked = skin.say("Masked columns");
-                widgets::label(ui, small, &masked, palette.secondary, look);
-                ui.add_space(5.0);
-                column_names(ui, form, skin);
-                column_error(ui, form, skin);
-                column_missing(ui, form, skin);
-                ui.add_space(6.0);
-                sentence(ui, small, &skin.say(MASKED_HINT), skin);
-            }
-        });
-        if guards {
-            ui.add_space(12.0);
-        }
-        let (role, height) = (skin.value_role(), skin.field_height());
-        super::row(ui, 12.0, &columns, |ui, column| {
-            let (label, text) = match column {
-                0 => (skin.say("Rows per answer"), &mut form.agents_rows),
-                _ => (
-                    skin.say("Statement timeout, seconds"),
-                    &mut form.agents_timeout,
-                ),
-            };
-            let label = widgets::label(ui, small, &label, palette.secondary, look);
-            ui.add_space(5.0);
-            ui.add(input(ui, text, role, height, skin))
-                .labelled_by(label.id);
-        });
-        ui.add_space(8.0);
-        sentence(ui, small, &skin.say(note(form)), skin);
+        ui.add(input(ui, text, role, height, skin))
+            .labelled_by(label.id);
     });
-    shown(ui, form, top);
-}
-
-/// The terminal look: an "agents" section after Safety.
+    ui.add_space(8.0);
+    sentence(ui, small, &skin.say(note(form)), skin);
+}
+
+/// The terminal look: an "agents" section after Safety; on a Files form,
+/// a heading that opens and closes it, as on macOS.
 pub(super) fn terminal(
     ui: &mut Ui,
     form: &mut ConnectionForm,
     skin: &Skin,
     actions: &mut Vec<Action>,
 ) {
+    let Skin { look, palette, .. } = *skin;
+    let top = ui.cursor().top();
+    let title = skin.say("Agents");
+    if guards_tables(form) {
+        super::terminal::terminal_section(ui, &title, skin);
+        terminal_rows(ui, form, skin, actions);
+    } else {
+        if form.show_agents {
+            form.agents_open = true;
+        }
+        let summary = look.label(&gettext(skin.locale, closed_summary(form)));
+        let section = Section {
+            salt: "agents",
+            title: &title,
+            summary: &summary,
+            hint: "",
+        };
+        let mut open = form.agents_open;
+        disclosure::show(
+            ui,
+            &section,
+            &mut open,
+            look,
+            palette,
+            skin.fill,
+            skin.rule(),
+            |ui| {
+                terminal_rows(ui, form, skin, actions);
+            },
+        );
+        form.agents_open = open;
+    }
+    shown(ui, form, top);
+}
+
+/// The terminal look: what the agents section holds.
+fn terminal_rows(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
     let look = skin.look;
-    let top = ui.cursor().top();
-    super::terminal::terminal_section(ui, &skin.say("Agents"), skin);
     let line = TextRole::OBody.row_height(ui.ctx(), look.faces);
     let heading = skin.say("Access");
     super::terminal::terminal_row_of(ui, &heading, line, skin, |ui, _| {
@@ -1009,7 +1091,6 @@
     }
     ui.add_space(4.0);
     sentence(ui, TextRole::OSecondary, &said, skin);
-    shown(ui, form, top);
 }
 
 /// The terminal look's Tables and Masked rows.
diff --git a/src/ui/connect_dialog/files.rs b/src/ui/connect_dialog/files.rs
--- a/src/ui/connect_dialog/files.rs
+++ b/src/ui/connect_dialog/files.rs
@@ -163,6 +163,18 @@
     response
 }
 
+/// What the Source holds as the found line says it: a file the Reading
+/// CSV section skips is left out.
+fn read(form: &ConnectionForm) -> FilesFound {
+    let mut found = form.files_found.clone();
+    if let FilesFound::Read(scan) = &mut found {
+        let reading = &form.files_csv;
+        scan.files
+            .retain(|file| !reading.skips(&tabletist_db::files::file_name(&file.path)));
+    }
+    found
+}
+
 /// macOS: the "Source" fieldset's inside.
 pub(super) fn sheet(
     ui: &mut Ui,
@@ -223,9 +235,10 @@
             super::note(ui, &why, palette.danger, skin);
         }
         None => {
-            if let Some(line) = found_line(&form.files_found, false) {
+            let found = read(form);
+            if let Some(line) = found_line(&found, false) {
                 widgets::label(ui, small, &line, palette.secondary, look);
-                if let FilesFound::Read(scan) = &form.files_found
+                if let FilesFound::Read(scan) = &found
                     && !scan.files.is_empty()
                 {
                     ui.add_space(6.0);
@@ -333,7 +346,7 @@
     }
     let (text, color) = match trouble(&form.files_found, skin) {
         Some(why) => (Some(why), palette.danger),
-        None => (found_line(&form.files_found, true), palette.dim),
+        None => (found_line(&read(form), true), palette.dim),
     };
     if let Some(text) = text {
         super::terminal::terminal_row_of(ui, "", line, skin, |ui, _| {
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -84,16 +84,30 @@
   with anything but letters, digits and `_` turned into `_`. Two files of
   the same name each take their extension: `orders_csv`,
   `orders_parquet`. A `.duckdb` file is a schema, its tables inside it.
+- **Reading CSV**, closed until you open it, says how the connection's
+  CSV and TSV files are read: **Header row** (Detect, Yes, No),
+  **Delimiter** (Detect, `,`, `;`, Tab, `|`; a `.tsv` file is always
+  read with a tab) and **Column types** (from the first 20,000 rows, from
+  all rows, which is slower, or everything as text). Closed, it says the
+  three on one line, and how many overrides there are. A change for one
+  file, such as a column read as text, shows under them with **Reset**
+  (`R` in the Omarchy look, on the override's row); for now an override
+  is written in `connections.toml` (see [Settings and
+  Files]({% link _reference/settings-and-files.md %}#the-connections-file)).
+  An open connection reads
+  them the next time it connects.
 - A new Files connection is **local**, and lets agents read. Files open
   read-only: nothing writes to them, and a statement that would read
   another file on the computer, write a file or install anything is
   refused. Agents can query the files, never change them, and cannot
-  hide tables or mask columns on a Files connection.
+  hide tables or mask columns on a Files connection. Its **Agents**
+  section is closed, and says the access on one line.
 - **Test** reads each file's columns and its first rows, never a whole
   file. The Excel reader is downloaded the first time a connection opens
   an Excel file.
 - The SQL editor runs DuckDB's SQL, and the Structure view shows each
-  file's path, size and format beside its columns.
+  file's path, size and format beside its columns, and what it is read
+  with.
 - A connection has no URL: paste a folder's or a file's path into the
   **URL** field of a new connection instead, and it becomes a Files
   connection over it. A SQLite database's path (`.db`, `.sqlite`,
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test -p tabletist --lib reading` and `cargo test -p tabletist --lib files_tests`

Expected: PASS. Then the full check: fmt and clippy clean, every test binary `ok` (5434 tests pass in this state).

- [ ] **Step 5: Look at it**

A throwaway scene appended to `src/shots.rs` (never committed; take it out, then run the full check again before the commit) renders the Files form with Reading CSV open and two overrides, and closed, in both looks: `cargo test --features shots --lib shots::<scene> -- --ignored`, then crop with `magick` and look. Expect the boards' layout: the closed row 40 high with its summary; open, the legend `⌄ Reading CSV`, the hint, two segmented choices and the dropdown, each override on the sunken tone with Reset at its end; Omarchy `▾ reading csv`, three choice rows, `override returns.csv · 'N/A' as empty in reason · R reset` with the file's own case kept, under `▾ reading csv  defaults for every csv`.

- [ ] **Step 6: Commit**

```bash
git add src crates/tabletist-core/src/keymap.rs docs/_reference/connections.md
git commit -m "Open and close Reading CSV in the Files form, and close Agents there"
```

---

## After the four tasks

- Push the branch and open a pull request against `main`. Its description lists: what Reading CSV does and the options per format; the `connections.toml` tables (version 8); the shared disclosure component; the seven departures above; that 2b (Test's background check and fixes) makes overrides from Test.
- No screenshot goes to GitHub unless it is blurred.
