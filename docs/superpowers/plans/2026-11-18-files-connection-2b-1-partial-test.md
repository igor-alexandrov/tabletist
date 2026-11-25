# Files connections, run 2b-1: the partial Test in the dialog, Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Files connection's Test reads every file it can, says those it cannot, and then reads its CSV, TSV and NDJSON files to their end in the background; a file that fails makes the Test partial, with a banner over the dialog's footer that names the file, the row, the value and the column, and fixes (read the column as text, read the value as empty, skip the file) that save an override under Reading CSV and run the Test again.

**Architecture:** `tabletist_db` opens a Test's session past the files it cannot open (`Connection::open_files_to_test`), keeps each as a `files::Failure` whose `Why` is read from DuckDB's message, and reads one file to its end with `check_file` (`SELECT count(COLUMNS(*))`). The backend's Files `Command::Test` sends `Event::FilesTested`, then `Event::FileChecked` per file, in a task the App stops with `Command::StopCheck` once the dialog no longer waits for it. The form keeps the check (`files_check`) and the failures, and `Action::FixFile` adds the override and tests again. A shared component, `ui::banner`, draws the banner in both looks; Omarchy's `t`, `e`, `s` run the fixes.

**Tech Stack:** Rust, egui (headless tests through `crate::testing::Harness`), DuckDB 2.0 (`duckdb` crate 1.20000.1, bundled), tokio.

**Spec:** `docs/superpowers/specs/2026-10-30-files-connection-design.md`, sections "Run 2 split" and "Run 2b-1: the partial Test in the dialog".

**Base:** `claude/files-connection-2-csv` at `1710235a` (run 2a, #353, not merged when this was written: once it merges, rebase this branch onto `main`). Branch `claude/files-connection-2b-partial`. The commit that adds this plan ("Plan run 2b-1: the partial Test in the dialog") also adds the spec's "Run 2b-1" section; it is already on the branch, and Task 1 starts from it.

---

## What the design asks, what is built, why

Approved by Igor on 2026-11-18 ("Yes, all five"):

| The prompt or a board asks | Built | Why |
| --- | --- | --- |
| "Row 48,112 has 'N/A' in returned_at" | The row counts data rows: DuckDB says the file's line, and a header row (line 1) is taken off | A row of the table, as the grid numbers it |
| Three fixes for a file that failed | The three for a CSV's or TSV's value its column's type cannot take; Skip alone for anything else (a row with more or fewer values, an NDJSON line that is not JSON or whose value is of another type) | Reading CSV does not read NDJSON, and those errors name no column |
| "Read 3 of 4 files · returns.csv failed" | With several, "Read 2 of 4 files · 2 failed"; the banner says the first, the next once it is mended and the Test has run again | One banner, as the boards draw it |
| "Each fix is saved as a per-file override" | Into the form's Reading CSV, shown as its override, and the Test runs again; `connections.toml` has it when the dialog is saved | As every field of the dialog: Cancel drops it |
| Partial when one file fails | Test opens each file it can and says those it cannot; connecting with a file that cannot be opened still fails as in run 1 | A session with a table missing would hide it; the error's Edit connection leads to the Test and its fixes |

The table view's banner is 2b-2, planned once this merges.

## Files

| File | Task | What changes |
| --- | --- | --- |
| `crates/tabletist-db/src/files.rs` | 1 | `Failure`, `Why`, `Failure::takes_column_fixes` |
| `crates/tabletist-db/src/duck.rs` | 1 | `opened` goes on past a file it cannot open when told; `why` reads DuckDB's message; `read_files` says the failures and the files to check; `check`; `FilesRead.failed` and `.checks` |
| `crates/tabletist-db/src/lib.rs` | 1 | `Connection::open_files_to_test`, `Connection::check_file` |
| `crates/tabletist-db/tests/files_check.rs` (new) | 1 | The Test and the check in full, on the Bookshop's files |
| `src/ui/files_tests.rs`, `crates/tabletist-core/src/model.rs` (one test) | 1 | Run 1's Test results take the two new fields |
| `crates/tabletist-core/src/backend.rs` | 2 | `Command::StopCheck`, `Event::FileChecked`, the Files Test's check in full |
| `src/promo/live.rs` | 2 | `StopCheck` among the commands it answers with nothing |
| `crates/tabletist-core/src/model.rs` | 2 | `files_check`, `first_failure`, `fix_file`, `FileFix`, `Action::FixFile` |
| `src/app.rs` | 2 | `FileChecked` applied, `FixFile`, the check held from the Test's start and stopped once nothing waits for it |
| `src/ui/partial_test_tests.rs` (new), `src/ui/mod.rs` | 2 | The check stopped once the dialog no longer waits for it |
| `src/ui/banner.rs` (new) | 3 | The shared banner with actions |
| `src/ui/connect_dialog/partial.rs` (new) | 3 | The partial banner's words and fixes |
| `src/ui/connect_dialog/mod.rs`, `src/ui/mod.rs` (`banner`) | 3 | The banner over the footer; `t`, `e`, `s`; the footer's words |
| `crates/tabletist-core/src/keymap.rs` | 3 | `FormFixText`, `FormFixEmpty`, `FormFixSkip` |
| `src/ui/partial_test_tests.rs` | 3 | The dialog's banner, fixes and keys, both looks |
| `docs/_reference/connections.md`, `docs/_guide/omarchy.md` | 3 | The check, the partial Test, its fixes and keys |

Every command below runs from the worktree with `CARGO_TARGET_DIR=$PWD/target` and `~/.cargo/bin/cargo`. The full check at each commit, as AGENTS.md has it: `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` (and once more with `--features shots`, which builds `src/promo/live.rs`), `cargo test --locked --workspace --all-targets` (with `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` set where the test servers run), and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.

---

### Task 1: The engine says which files a Test cannot read, and reads one to its end

**Files:**
- Modify: `crates/tabletist-db/src/files.rs`, `crates/tabletist-db/src/duck.rs`, `crates/tabletist-db/src/lib.rs`
- Create: `crates/tabletist-db/tests/files_check.rs`
- Modify: `src/ui/files_tests.rs`, `crates/tabletist-core/src/model.rs` (run 1's and 2a's tests build a `FilesRead`: they take `failed` and `checks`, empty)

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/crates/tabletist-db/tests/files_check.rs b/crates/tabletist-db/tests/files_check.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-db/tests/files_check.rs
@@ -0,0 +1,225 @@
+//! A Files connection's Test: the files it reads and those it cannot, and
+//! the check in full that follows it, which finds a row past the sample
+//! that a file's reading cannot take, and says where and why.
+
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
+use std::path::Path;
+
+use tabletist_db::files::{Change, Failure, Format, Override, Reading, Source, Why};
+use tabletist_db::{ConnectSpec, Connection, Secrets};
+
+/// The Bookshop's exports, `returns.csv` among them with 30,000 returns
+/// and then one whose `returned_at` is `N/A`: past the rows a reader
+/// samples for types.
+fn exports() -> tempfile::TempDir {
+    let dir = tempfile::tempdir().unwrap();
+    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
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
+fn spec(dir: &Path, overrides: Vec<Override>) -> ConnectSpec {
+    let mut spec = ConnectSpec::files(Source::Folder(dir.to_owned()));
+    spec.csv = Reading {
+        overrides,
+        ..Reading::default()
+    };
+    spec
+}
+
+fn returns(change: Change) -> Override {
+    Override {
+        file: "returns.csv".into(),
+        change,
+    }
+}
+
+/// What a Test reads of `spec`, and what the check in full finds of each
+/// file it names.
+async fn test(spec: &ConnectSpec) -> (tabletist_db::FilesRead, Vec<Failure>) {
+    let connection = Connection::open_files_to_test(spec).await.unwrap();
+    let read = connection.read_files().await.unwrap().unwrap();
+    let mut found = Vec::new();
+    for name in &read.checks {
+        found.extend(connection.check_file(name).await.unwrap());
+    }
+    (read, found)
+}
+
+#[tokio::test]
+async fn the_check_in_full_finds_a_value_past_the_sample_and_says_where() {
+    let dir = exports();
+    let (read, found) = test(&spec(dir.path(), Vec::new())).await;
+    // The Test itself reads every file: the value is past its rows.
+    assert_eq!(read.files, 7);
+    assert!(read.failed.is_empty(), "{:?}", read.failed);
+    // The CSV, TSV and NDJSON files are checked in full, not Parquet,
+    // JSON arrays or `.duckdb` files.
+    let mut checks = read.checks.clone();
+    checks.sort();
+    assert_eq!(checks, ["authors", "events", "orders", "returns"]);
+    assert_eq!(
+        found,
+        [Failure {
+            file: "returns.csv".into(),
+            path: dir.path().join("returns.csv"),
+            format: Format::Csv,
+            // The file's line 30,002, its header row line 1.
+            why: Why::Value {
+                row: 30_001,
+                column: "returned_at".into(),
+                value: "N/A".into(),
+                kind: "DATE".into(),
+            },
+        }]
+    );
+    assert!(found[0].takes_column_fixes());
+}
+
+#[tokio::test]
+async fn each_fix_reads_the_file_to_its_end() {
+    let dir = exports();
+    for fix in [
+        returns(Change::Text {
+            column: "returned_at".into(),
+        }),
+        returns(Change::Empty {
+            column: "returned_at".into(),
+            values: vec!["N/A".into()],
+        }),
+        returns(Change::Skip),
+    ] {
+        let (read, found) = test(&spec(dir.path(), vec![fix.clone()])).await;
+        assert!(found.is_empty(), "{fix:?}: {found:?}");
+        // A skipped file is no file of the connection's.
+        let (files, checked) = match fix.change {
+            Change::Skip => (6, false),
+            Change::Text { .. } | Change::Empty { .. } => (7, true),
+        };
+        assert_eq!(read.files, files, "{fix:?}");
+        assert_eq!(read.checks.iter().any(|name| name == "returns"), checked);
+    }
+}
+
+#[tokio::test]
+async fn a_row_with_more_values_and_bad_json_lines_say_their_row_or_line() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut wide = String::from("id,title\n");
+    for id in 1..=30_000 {
+        wide.push_str(&format!("{id},Dune\n"));
+    }
+    wide.push_str("30001,Emma,Austen\n");
+    std::fs::write(dir.path().join("books.csv"), wide).unwrap();
+    let mut events = String::new();
+    for id in 1..=30_000 {
+        events.push_str(&format!("{{\"order_id\":{id},\"kind\":\"placed\"}}\n"));
+    }
+    events.push_str("{\"order_id\":\"N/A\",\"kind\":\"placed\"}\n");
+    std::fs::write(dir.path().join("events.ndjson"), events).unwrap();
+    let (read, mut found) = test(&spec(dir.path(), Vec::new())).await;
+    assert!(read.failed.is_empty(), "{:?}", read.failed);
+    found.sort_by(|a, b| a.file.cmp(&b.file));
+    let whys: Vec<&Why> = found.iter().map(|failure| &failure.why).collect();
+    assert_eq!(
+        whys,
+        [
+            &Why::Columns {
+                row: 30_001,
+                expected: 2,
+                found: 3,
+            },
+            &Why::Json {
+                line: 30_001,
+                malformed: false,
+            },
+        ]
+    );
+    // Neither is mended by a change to a column: skipping is the fix.
+    assert!(found.iter().all(|failure| !failure.takes_column_fixes()));
+    // A line that is not JSON at all.
+    std::fs::write(
+        dir.path().join("events.ndjson"),
+        "{\"order_id\":1}\n{\"order_id\": oops}\n",
+    )
+    .unwrap();
+    let (read, _) = test(&spec(dir.path(), Vec::new())).await;
+    assert_eq!(
+        read.failed
+            .iter()
+            .map(|failure| &failure.why)
+            .collect::<Vec<_>>(),
+        [&Why::Json {
+            line: 2,
+            malformed: true,
+        }]
+    );
+}
+
+#[tokio::test]
+async fn a_test_reads_the_files_it_can_where_a_session_does_not_open() {
+    let dir = exports();
+    // Not Parquet at all: the file does not open.
+    std::fs::write(dir.path().join("customers.parquet"), "id,name\n1,Ada\n").unwrap();
+    let error = Connection::connect(&spec(dir.path(), Vec::new()), &Secrets::default())
+        .await
+        .err()
+        .unwrap();
+    assert!(
+        error.to_string().contains("Can't read customers.parquet"),
+        "{error}"
+    );
+    let (read, found) = test(&spec(dir.path(), Vec::new())).await;
+    assert_eq!(read.files, 7);
+    assert_eq!(
+        read.failed
+            .iter()
+            .map(|failure| &failure.file)
+            .collect::<Vec<_>>(),
+        ["customers.parquet"]
+    );
+    assert!(matches!(read.failed[0].why, Why::Other(_)));
+    assert!(!read.failed[0].takes_column_fixes());
+    // The others are read and checked as ever.
+    assert!(read.checks.iter().any(|name| name == "returns"));
+    assert_eq!(found.len(), 1);
+}
+
+#[tokio::test]
+async fn a_quote_in_a_column_s_name_or_its_value_stays_in_it() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut text = String::from("order_id,\"size\"\"in\"\n");
+    for order in 1..=30_000 {
+        text.push_str(&format!("{order},{order}\n"));
+    }
+    // `12"`, quoted as CSV quotes it, on a row that also says ` to '`.
+    text.push_str("30001,\"12\"\"\"\n");
+    std::fs::write(dir.path().join("sizes.csv"), text).unwrap();
+    let (_, found) = test(&spec(dir.path(), Vec::new())).await;
+    assert_eq!(
+        found.iter().map(|failure| &failure.why).collect::<Vec<_>>(),
+        [&Why::Value {
+            row: 30_001,
+            column: "size\"in".into(),
+            value: "12\"".into(),
+            kind: "BIGINT".into(),
+        }]
+    );
+    // Read as empty, the value as it is found: the file reads to its end.
+    let empty = Override {
+        file: "sizes.csv".into(),
+        change: Change::Empty {
+            column: "size\"in".into(),
+            values: vec!["12\"".into()],
+        },
+    };
+    let (_, found) = test(&spec(dir.path(), vec![empty])).await;
+    assert!(found.is_empty(), "{found:?}");
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p tabletist-db --test files_check`

Expected: FAIL to compile, as `Failure`, `Why`, `open_files_to_test` and `check_file` do not exist yet:

```text
error[E0432]: unresolved imports `tabletist_db::files::Failure`, `tabletist_db::files::Why`
error[E0599]: no associated function or constant named `open_files_to_test` found for struct `Connection` in the current scope
```

- [ ] **Step 3: Open past a file, read DuckDB's words, check a file in full**

```diff
diff --git a/crates/tabletist-db/src/files.rs b/crates/tabletist-db/src/files.rs
--- a/crates/tabletist-db/src/files.rs
+++ b/crates/tabletist-db/src/files.rs
@@ -158,6 +158,47 @@
 
 impl Types {
     pub const ALL: [Self; 3] = [Self::Sample, Self::All, Self::Text];
+}
+
+/// A file a Test could not read, or its check in full found a row of that
+/// it cannot, and why.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Failure {
+    /// The file's name with its extension: `returns.csv`.
+    pub file: String,
+    pub path: PathBuf,
+    pub format: Format,
+    pub why: Why,
+}
+
+impl Failure {
+    /// Whether the fixes that change a column (read it as text, read a
+    /// value as empty) can mend it: a CSV's or TSV's value its column's
+    /// type cannot take. Any failure can be mended by skipping the file.
+    pub fn takes_column_fixes(&self) -> bool {
+        matches!(self.why, Why::Value { .. }) && matches!(self.format, Format::Csv | Format::Tsv)
+    }
+}
+
+/// Why a file could not be read, as DuckDB said it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum Why {
+    /// A CSV's value that its column's type cannot take: the data row it
+    /// is on (1 for the first after a header row), the column, the value,
+    /// and the type DuckDB found for the column (`DATE`).
+    Value {
+        row: u64,
+        column: String,
+        value: String,
+        kind: String,
+    },
+    /// A CSV's row with more or fewer values than the file's others.
+    Columns { row: u64, expected: u64, found: u64 },
+    /// An NDJSON file's line that is not JSON (`malformed`), or whose
+    /// value is not of the type the lines before it gave its key.
+    Json { line: u64, malformed: bool },
+    /// Anything else: DuckDB's first line.
+    Other(String),
 }
 
 /// How many rows of a CSV DuckDB samples for its columns' types, and how
diff --git a/crates/tabletist-db/src/duck.rs b/crates/tabletist-db/src/duck.rs
--- a/crates/tabletist-db/src/duck.rs
+++ b/crates/tabletist-db/src/duck.rs
@@ -46,10 +46,16 @@
 /// What a Test of a Files connection read.
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub struct FilesRead {
-    /// The files read, `.duckdb` ones included.
+    /// The connection's files, `.duckdb` ones included, those that failed
+    /// too.
     pub files: usize,
     /// Their size on disk, in bytes.
     pub bytes: u64,
+    /// The files it could not read, in the connection's order.
+    pub failed: Vec<crate::files::Failure>,
+    /// The files read whose whole is checked next, in the background: the
+    /// CSV, TSV and NDJSON files, by their tables' names.
+    pub checks: Vec<String>,
 }
 
 /// What the Structure view says of the file a table is read from.
@@ -74,6 +80,12 @@
     /// The number the next call takes.
     calls: std::sync::atomic::AtomicU64,
     files: Arc<Vec<Opened>>,
+    /// The files a Test's session could not open, left out of it. A
+    /// session that browses has none: it does not open with one.
+    failed: Vec<crate::files::Failure>,
+    /// Every file of the connection's, in its order: those it could not
+    /// open among them.
+    order: Vec<std::path::PathBuf>,
     version: String,
 }
 
@@ -457,9 +469,20 @@
     }
 }
 
+/// What [`opened`] made: the session, its files, those it could not open
+/// (where it was told to go on past them) and DuckDB's version.
+type Made = (
+    duckdb::Connection,
+    Vec<Opened>,
+    Vec<crate::files::Failure>,
+    String,
+);
+
 /// Opens a session over `files`, read as `reading` says, a file it skips
-/// left out: views and attaches, then the fence.
-fn opened(files: &[Found], reading: &Reading) -> Result<(duckdb::Connection, Vec<Opened>, String)> {
+/// left out: views and attaches, then the fence. A file that cannot be
+/// opened stops it, unless `past` says to go on without it (a Test, which
+/// says which failed).
+fn opened(files: &[Found], reading: &Reading, past: bool) -> Result<Made> {
     let files: Vec<&Found> = files
         .iter()
         .filter(|file| !reading.skips(&file.path))
@@ -486,6 +509,7 @@
         })?;
     }
     let mut opened = Vec::with_capacity(files.len());
+    let mut failures = Vec::new();
     for file in &files {
         let failed = |error: &Error| {
             Error::Connect(format!("Can't read {}: {}", file_label(file), plain(error)))
@@ -513,9 +537,18 @@
             Types::Sample | Types::Text => reading,
         };
         // A `.duckdb` file is attached read-only, as every session is.
-        let (statement, options) =
-            make(&connection, file, reading).map_err(|error| failed(&error))?;
-        run(&statement).map_err(|error| failed(&error))?;
+        let made = make(&connection, file, reading).and_then(|(statement, options)| {
+            run(&statement)?;
+            Ok((statement, options))
+        });
+        let (statement, options) = match made {
+            Ok(made) => made,
+            Err(error) if past => {
+                failures.push(failure(file, &error));
+                continue;
+            }
+            Err(error) => return Err(failed(&error)),
+        };
         let rows = match file.format {
             Format::Parquet => connection
                 .query_row(
@@ -562,8 +595,106 @@
     Ok((
         connection,
         opened,
+        failures,
         format!("DuckDB {}", version.trim_start_matches('v')),
     ))
+}
+
+/// Why `file` could not be read, from DuckDB's words.
+fn failure(file: &Found, error: &Error) -> crate::files::Failure {
+    crate::files::Failure {
+        file: file_label(file),
+        path: file.path.clone(),
+        format: file.format,
+        why: why(error),
+    }
+}
+
+/// Reads DuckDB's message for what its reader could not take. Its CSV
+/// errors say the file's line (the header row is line 1 where the file
+/// has one) and its settings, `header = true`, after.
+fn why(error: &Error) -> crate::files::Why {
+    use crate::files::Why;
+    let message = match error {
+        Error::Query { message, .. } => message.as_str(),
+        _ => return Why::Other(plain(error)),
+    };
+    // The number that follows `before` in the message.
+    let number = |before: &str| -> Option<u64> {
+        let at = message.find(before)? + before.len();
+        let digits: String = message[at..]
+            .trim_start()
+            .chars()
+            .take_while(char::is_ascii_digit)
+            .collect();
+        digits.parse().ok()
+    };
+    // DuckDB's line naming the column and the value, on its own: the
+    // message also quotes the file's row, which may hold any words. The
+    // pieces end at the last of their closing words, so a quote in a
+    // column's name or a value stays in it.
+    let converting = message
+        .lines()
+        .find(|line| line.starts_with("Error when converting column \""));
+    let value = converting.and_then(|line| {
+        let rest = line.strip_prefix("Error when converting column \"")?;
+        let split = rest.rfind("\". Could not convert string \"")?;
+        let column = &rest[..split];
+        let rest = &rest[split + "\". Could not convert string \"".len()..];
+        let to = rest.rfind("\" to '")?;
+        let kind = &rest[to + "\" to '".len()..];
+        // The value as the file writes it, a quote in it doubled: the
+        // text the column holds, which a fix compares, has it once.
+        Some((
+            column.to_owned(),
+            rest[..to].replace("\"\"", "\""),
+            kind[..kind.find('\'')?].to_owned(),
+        ))
+    });
+    if let Some(line) = number("CSV Error on Line:") {
+        let header = message
+            .lines()
+            .any(|line| line.trim_start().starts_with("header = true"));
+        let row = line.saturating_sub(u64::from(header));
+        if let Some((column, value, kind)) = value {
+            return Why::Value {
+                row,
+                column,
+                value,
+                kind,
+            };
+        }
+        let counts = message.lines().find_map(|line| {
+            let rest = line.strip_prefix("Expected Number of Columns:")?;
+            let (expected, found) = rest.split_once("Found:")?;
+            Some((expected.trim().parse().ok()?, found.trim().parse().ok()?))
+        });
+        if let Some((expected, found)) = counts {
+            return Why::Columns {
+                row,
+                expected,
+                found,
+            };
+        }
+    }
+    // DuckDB says the line after a malformed one.
+    if message.contains("Malformed JSON")
+        && let Some(line) = number(" in line ")
+    {
+        return Why::Json {
+            line: line.saturating_sub(1).max(1),
+            malformed: true,
+        };
+    }
+    if message.contains("JSON transform error")
+        && let Some(line) = number(" in line ")
+    {
+        return Why::Json {
+            line,
+            malformed: false,
+        };
+    }
+    Why::Other(plain(error))
 }
 
 /// Stops call `call`'s work when dropped, unless disarmed first: a dropped
@@ -1193,9 +1324,9 @@
 impl Conn {
     /// Opens `source`'s files, read-only: every one is read far enough to
     /// know its columns, and an error names the first that cannot be.
-    pub(crate) async fn open(source: &Source, reading: &Reading) -> Result<Self> {
+    pub(crate) async fn open(source: &Source, reading: &Reading, past: bool) -> Result<Self> {
         let (source, reading) = (source.clone(), reading.clone());
-        let (connection, files, version) = tokio::task::spawn_blocking(move || {
+        let (connection, files, failed, version, order) = tokio::task::spawn_blocking(move || {
             // A skipped file is not looked at: one that is gone, or that
             // cannot be looked at, is no matter.
             let scan = crate::files::scan_past(&source, |file| reading.skips(file))?;
@@ -1206,7 +1337,10 @@
                     crate::files::shown(gone)
                 )));
             }
-            opened(&scan.files, &reading)
+            let order = scan.files.iter().map(|file| file.path.clone()).collect();
+            opened(&scan.files, &reading, past).map(|(connection, files, failed, version)| {
+                (connection, files, failed, version, order)
+            })
         })
         .await
         .map_err(|error| Error::Io(error.to_string()))??;
@@ -1217,6 +1351,8 @@
             running: Arc::new(std::sync::atomic::AtomicU64::new(0)),
             calls: std::sync::atomic::AtomicU64::new(0),
             files: Arc::new(files),
+            failed,
+            order,
             version,
         })
     }
@@ -1265,31 +1401,89 @@
     }
 
     /// Reads the first rows of every file, as a Test does: never a whole
-    /// file.
+    /// file. A file it cannot read is a failure, with those the session
+    /// could not open; the others are read. Which files to check in full
+    /// next come with it.
     pub(crate) async fn read_files(&self) -> Result<FilesRead> {
         let files = Arc::clone(&self.files);
+        let mut failed = self.failed.clone();
+        let order = self.order.clone();
+        // The files it could not open count, and their size on disk.
+        let unopened = (
+            self.failed.len(),
+            self.failed
+                .iter()
+                .map(|failure| std::fs::metadata(&failure.path).map_or(0, |meta| meta.len()))
+                .sum::<u64>(),
+        );
         self.run(move |connection| {
+            let mut read = Vec::new();
             read_only(connection, |connection| {
                 for file in files.iter().filter(|file| !file.found.format.is_database()) {
                     let sql = format!(
                         "SELECT * FROM {} LIMIT {SAMPLE_ROWS}",
                         Dialect::DuckDb.quote_ident(&file.found.name)
                     );
-                    rows_of(connection, &sql, Vec::new(), SAMPLE_ROWS as usize).map_err(
-                        |error| {
-                            Error::Connect(format!(
-                                "Can't read {}: {}",
-                                file_label(&file.found),
-                                plain(&error)
-                            ))
-                        },
-                    )?;
+                    match rows_of(connection, &sql, Vec::new(), SAMPLE_ROWS as usize) {
+                        Ok(_) => read.push(file),
+                        Err(Error::Cancelled) => return Err(Error::Cancelled),
+                        Err(error) => failed.push(failure(&file.found, &error)),
+                    }
                 }
                 Ok(())
             })?;
+            // In the connection's order, those it could not open among them.
+            failed.sort_by_key(|failure| {
+                order
+                    .iter()
+                    .position(|path| *path == failure.path)
+                    .unwrap_or(usize::MAX)
+            });
+            let checks = read
+                .iter()
+                .filter(|file| {
+                    matches!(
+                        file.found.format,
+                        Format::Csv | Format::Tsv | Format::Ndjson
+                    )
+                })
+                .map(|file| file.found.name.clone())
+                .collect();
             Ok(FilesRead {
-                files: files.len(),
-                bytes: files.iter().map(|file| file.found.size).sum(),
+                files: files.len() + unopened.0,
+                bytes: files.iter().map(|file| file.found.size).sum::<u64>() + unopened.1,
+                failed,
+                checks,
+            })
+        })
+        .await
+    }
+
+    /// Reads the whole of the file whose table is `name`, every column of
+    /// every row, as a Test's check in the background does: `None` when it
+    /// reads to the end, or why it stops.
+    pub(crate) async fn check(&self, name: &str) -> Result<Option<crate::files::Failure>> {
+        let Some(file) = self
+            .files
+            .iter()
+            .find(|file| file.found.name == name)
+            .cloned()
+        else {
+            return Ok(None);
+        };
+        self.run(move |connection| {
+            // Each column counted: a column read is a column converted,
+            // where `count(*)` would read none.
+            let sql = format!(
+                "SELECT count(COLUMNS(*)) FROM {}",
+                Dialect::DuckDb.quote_ident(&file.found.name)
+            );
+            read_only(connection, |connection| {
+                match rows_of(connection, &sql, Vec::new(), 1) {
+                    Ok(_) => Ok(None),
+                    Err(Error::Cancelled) => Err(Error::Cancelled),
+                    Err(error) => Ok(Some(failure(&file.found, &error))),
+                }
             })
         })
         .await
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -135,7 +135,7 @@
                     .as_ref()
                     .ok_or_else(|| Error::InvalidSpec("choose a folder or files".into()))?;
                 Ok(Self::Files(Box::new(
-                    duck::Conn::open(source, &spec.csv).await?,
+                    duck::Conn::open(source, &spec.csv, false).await?,
                 )))
             }
         }
@@ -618,6 +618,34 @@
         }
     }
 
+    /// A Files connection's session for its Test: every file it can open,
+    /// those it cannot left out and said by [`Connection::read_files`],
+    /// where a session to browse does not open with one.
+    pub async fn open_files_to_test(spec: &ConnectSpec) -> Result<Self> {
+        let source = spec
+            .files
+            .as_ref()
+            .ok_or_else(|| Error::InvalidSpec("choose a folder or files".into()))?;
+        Ok(Self {
+            inner: Inner::Files(Box::new(duck::Conn::open(source, &spec.csv, true).await?)),
+            access: Access::ReadOnly,
+            tunnel: None,
+        })
+    }
+
+    /// The whole of a Files connection's file whose table is `name`, read
+    /// to the end, as a Test's check in the background does: `None` when
+    /// every row reads, or why one does not. `None` too for any other
+    /// driver.
+    pub async fn check_file(&self, name: &str) -> Result<Option<files::Failure>> {
+        match &self.inner {
+            Inner::Files(conn) => conn.check(name).await,
+            Inner::Sqlite(_) | Inner::Postgres(_) | Inner::MySql(_) | Inner::BigQuery(_) => {
+                Ok(None)
+            }
+        }
+    }
+
     /// A Files connection's Test: the first rows of every file, with the
     /// readers browsing uses, never a whole file. `None` for any other
     /// driver, whose Test is the connection itself.
diff --git a/src/ui/files_tests.rs b/src/ui/files_tests.rs
--- a/src/ui/files_tests.rs
+++ b/src/ui/files_tests.rs
@@ -359,6 +359,8 @@
             result: Ok(FilesRead {
                 files: 3,
                 bytes: 2_400_000_000,
+                failed: Vec::new(),
+                checks: Vec::new(),
             }),
         }));
         let said = look.label("Read 3 files · 2.4 GB");
@@ -404,6 +406,8 @@
             result: Ok(FilesRead {
                 files: 3,
                 bytes: 2_400_000_000,
+                failed: Vec::new(),
+                checks: Vec::new(),
             }),
         }));
         // A SQLite Test passed after it, the Files one's read still held.
@@ -436,6 +440,8 @@
         let read = Ok(FilesRead {
             files: 3,
             bytes: 2_400_000_000,
+            failed: Vec::new(),
+            checks: Vec::new(),
         });
         harness.app.apply(Action::TestConnection);
         let TestState::Running(request) = form(&harness).test else {
@@ -485,7 +491,12 @@
         harness.app.apply(Action::SetDriver(Driver::Sqlite));
         harness.app.apply(Action::Backend(Event::FilesTested {
             request,
-            result: Ok(FilesRead { files: 3, bytes: 1 }),
+            result: Ok(FilesRead {
+                files: 3,
+                bytes: 1,
+                failed: Vec::new(),
+                checks: Vec::new(),
+            }),
         }));
         assert_eq!(form(&harness).test, TestState::Idle, "{}", look.name);
         assert!(form(&harness).files_tested.is_none(), "{}", look.name);
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -9205,6 +9205,8 @@
             Some(tabletist_db::FilesRead {
                 files: 4,
                 bytes: 10,
+                failed: Vec::new(),
+                checks: Vec::new(),
             })
         };
         // Each change forgets the Test on its own.
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test -p tabletist-db --test files_check --test files --test files_csv`

Expected: PASS. Then the full check: fmt and clippy clean, every test binary `ok` (5458 tests pass in this state).

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db src/ui/files_tests.rs crates/tabletist-core/src/model.rs
git commit -m "Say which files a Files Test cannot read, and read one to its end"
```

---

### Task 2: The Test checks its files in the background, and a fix tests again

**Files:**
- Modify: `crates/tabletist-core/src/backend.rs`, `src/promo/live.rs`, `crates/tabletist-core/src/model.rs`, `src/app.rs`, `src/ui/mod.rs`
- Create: `src/ui/partial_test_tests.rs` (the check stopped; Task 3 adds the banner's tests)

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/crates/tabletist-core/src/backend.rs b/crates/tabletist-core/src/backend.rs
--- a/crates/tabletist-core/src/backend.rs
+++ b/crates/tabletist-core/src/backend.rs
@@ -4157,6 +4157,83 @@
         }
     }
 
+    /// A Files Test says what it read, then reads its CSV, TSV and NDJSON
+    /// files to their end, one at a time, saying each: here a value past
+    /// the sample in `returns.csv`. A stopped check says no more.
+    #[test]
+    fn a_files_test_checks_its_files_in_full_after_it_answers() {
+        let dir = tempfile::tempdir().unwrap();
+        tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
+        let mut text = String::from("order_id,returned_at,reason\n");
+        for order in 1..=30_000 {
+            text.push_str(&format!("{order},2026-02-{:02},damaged\n", order % 28 + 1));
+        }
+        text.push_str("30001,N/A,lost in the post\n");
+        std::fs::write(dir.path().join("returns.csv"), text).unwrap();
+        let spec = ConnectSpec::files(tabletist_db::files::Source::Folder(dir.path().to_owned()));
+        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
+        backend.send(Command::Test {
+            request: RequestId(7),
+            spec: spec.clone(),
+            secrets: Secrets::default(),
+            host_keys: HostKeys::default(),
+        });
+        let checks = match backend.wait(WAIT) {
+            Some(Event::FilesTested {
+                request: RequestId(7),
+                result: Ok(read),
+            }) => {
+                assert!(read.failed.is_empty(), "{read:?}");
+                read.checks.len()
+            }
+            other => panic!("expected the Test, got {other:?}"),
+        };
+        assert_eq!(checks, 4);
+        let mut failures = Vec::new();
+        for left in (0..checks).rev() {
+            match backend.wait(WAIT) {
+                Some(Event::FileChecked {
+                    request: RequestId(7),
+                    failure,
+                    left: said,
+                }) => {
+                    assert_eq!(said, left);
+                    failures.extend(failure);
+                }
+                other => panic!("expected a file checked, got {other:?}"),
+            }
+        }
+        assert_eq!(
+            failures
+                .iter()
+                .map(|failure| failure.file.as_str())
+                .collect::<Vec<_>>(),
+            ["returns.csv"]
+        );
+        // Stopped before it answers, the Test checks nothing. (This is the
+        // stop before the answer; a stop during a check drops the same
+        // task, and dropping its statement interrupts DuckDB.)
+        backend.send(Command::Test {
+            request: RequestId(8),
+            spec,
+            secrets: Secrets::default(),
+            host_keys: HostKeys::default(),
+        });
+        backend.send(Command::StopCheck {
+            request: RequestId(8),
+        });
+        let mut after = Vec::new();
+        while let Some(event) = backend.wait(std::time::Duration::from_millis(500)) {
+            after.push(event);
+        }
+        assert!(
+            after
+                .iter()
+                .all(|event| !matches!(event, Event::FileChecked { .. })),
+            "{after:?}"
+        );
+    }
+
     #[test]
     fn a_test_connects_and_closes_without_a_session() {
         let (_dir, spec) = fixture();
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -9163,6 +9163,98 @@
         assert!(!before.is_on(&completion_site(""), print("susers")));
     }
 
+    /// A fix adds the override for the first file the Test could not read
+    /// and forgets the Test, so it runs again.
+    #[test]
+    fn a_fix_mends_the_first_file_the_test_could_not_read() {
+        use tabletist_db::files::{Change, Failure, Format, Override, Why};
+        let failure = |path: &str, why: Why| Failure {
+            file: "returns.csv".into(),
+            path: path.into(),
+            format: Format::Csv,
+            why,
+        };
+        let value = Why::Value {
+            row: 48_112,
+            column: "returned_at".into(),
+            value: "N/A".into(),
+            kind: "DATE".into(),
+        };
+        let mut form = ConnectionForm {
+            driver: Driver::Files,
+            ..ConnectionForm::default()
+        };
+        // No Test, no failure: nothing to mend.
+        assert!(!form.fix_file(FileFix::Skip));
+        let tested = |failed: Vec<Failure>| {
+            Some(tabletist_db::FilesRead {
+                files: 4,
+                bytes: 10,
+                failed,
+                checks: Vec::new(),
+            })
+        };
+        for (fix, change) in [
+            (
+                FileFix::Text,
+                Change::Text {
+                    column: "returned_at".into(),
+                },
+            ),
+            (
+                FileFix::Empty,
+                Change::Empty {
+                    column: "returned_at".into(),
+                    values: vec!["N/A".into()],
+                },
+            ),
+            (FileFix::Skip, Change::Skip),
+        ] {
+            form.files_csv.overrides.clear();
+            form.test = TestState::Passed;
+            form.files_tested = tested(vec![failure("/srv/exports/returns.csv", value.clone())]);
+            assert_eq!(
+                form.first_failure().map(|failure| &failure.file),
+                Some(&"returns.csv".to_owned())
+            );
+            assert!(form.fix_file(fix));
+            assert_eq!(
+                form.files_csv.overrides,
+                [Override {
+                    file: "returns.csv".into(),
+                    change,
+                }]
+            );
+            // The Test is forgotten: it ran with the old reading.
+            assert_eq!(form.test, TestState::Idle);
+            assert!(form.files_tested.is_none());
+        }
+        // A failure that is not a value takes Skip alone.
+        form.files_csv.overrides.clear();
+        form.test = TestState::Passed;
+        let wide = Why::Columns {
+            row: 2,
+            expected: 3,
+            found: 4,
+        };
+        form.files_tested = tested(vec![failure("/srv/exports/returns.csv", wide)]);
+        assert!(!form.fix_file(FileFix::Text));
+        assert!(!form.fix_file(FileFix::Empty));
+        assert!(form.files_csv.overrides.is_empty());
+        assert!(form.fix_file(FileFix::Skip));
+        // Two files of one name in a list: the path names the one.
+        form.files_csv.overrides.clear();
+        form.files_kind = FilesKind::Files;
+        form.files_list = vec![
+            "/srv/current/returns.csv".into(),
+            "/srv/archive/returns.csv".into(),
+        ];
+        form.test = TestState::Passed;
+        form.files_tested = tested(vec![failure("/srv/archive/returns.csv", value)]);
+        assert!(form.fix_file(FileFix::Skip));
+        assert_eq!(form.files_csv.overrides[0].file, "/srv/archive/returns.csv");
+    }
+
     /// How a Files connection reads CSV goes from the saved connection to
     /// the form and back, and each change drops a Test that read the old
     /// way.
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -114,6 +114,8 @@
 mod pane_hints_tests;
 #[cfg(test)]
 mod pane_keys_tests;
+#[cfg(test)]
+mod partial_test_tests;
 pub mod password_prompt;
 pub mod pending_bar;
 pub mod picker;
diff --git a/src/ui/partial_test_tests.rs b/src/ui/partial_test_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/partial_test_tests.rs
@@ -0,0 +1,157 @@
+//! A Files connection's partial Test in the connection dialog: the
+//! check in full that follows a Test, stopped once the dialog no
+//! longer waits for it.
+
+use std::path::PathBuf;
+
+use tabletist_db::FilesRead;
+use tabletist_db::files::{Failure, Format, Found, Scan, Why};
+
+use crate::backend::{Command, Event, RequestId};
+use crate::model::{Action, ConnectionForm, CsvChange, Dialog, FilesFound, PickTarget, TestState};
+use crate::testing::Harness;
+use crate::theme::Look;
+
+fn form(harness: &Harness) -> &ConnectionForm {
+    match &harness.app.dialog {
+        Some(Dialog::Connection(form)) => form,
+        _ => panic!("no connection dialog"),
+    }
+}
+
+const FOLDER: &str = "/data/bookshop-exports";
+
+/// A new Files connection's dialog in `look`, its folder the Bookshop's
+/// exports, read.
+fn exports(look: Look) -> Harness {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    harness.app.apply(Action::NewConnection);
+    harness.settle();
+    harness.click(&look.label("Files"));
+    harness.app.apply(Action::PickFolder);
+    let request = form(&harness).pick_request.expect("a pick in flight");
+    assert_eq!(form(&harness).pick_target, PickTarget::Folder);
+    harness.app.apply(Action::Backend(Event::FilePicked {
+        request,
+        path: Some(FOLDER.into()),
+    }));
+    let FilesFound::Reading(request) = form(&harness).files_found else {
+        panic!("no read of the source");
+    };
+    let found = |name: &str, format: Format| Found {
+        path: PathBuf::from(FOLDER).join(name),
+        format,
+        size: 1_000,
+        name: name.split('.').next().unwrap_or_default().to_owned(),
+    };
+    harness.app.apply(Action::Backend(Event::SourceRead {
+        request,
+        result: Ok(Scan {
+            files: vec![
+                found("orders.csv", Format::Csv),
+                found("customers.parquet", Format::Parquet),
+                found("events.ndjson", Format::Ndjson),
+                found("returns.csv", Format::Csv),
+            ],
+            missing: Vec::new(),
+        }),
+    }));
+    harness.settle();
+    harness
+}
+
+/// `returns.csv`'s row 48,112, whose `N/A` its date column cannot take.
+fn late_na() -> Failure {
+    Failure {
+        file: "returns.csv".into(),
+        path: PathBuf::from(FOLDER).join("returns.csv"),
+        format: Format::Csv,
+        why: Why::Value {
+            row: 48_112,
+            column: "returned_at".into(),
+            value: "N/A".into(),
+            kind: "DATE".into(),
+        },
+    }
+}
+
+/// Starts a Test and answers it: `failed` the files it could not read, and
+/// `checks` the ones it checks in full next. Returns the Test's request.
+fn tested(harness: &mut Harness, failed: Vec<Failure>, checks: &[&str]) -> RequestId {
+    harness.app.apply(Action::TestConnection);
+    let TestState::Running(request) = form(harness).test else {
+        panic!("no test: {:?}", form(harness).test);
+    };
+    harness.app.apply(Action::Backend(Event::FilesTested {
+        request,
+        result: Ok(FilesRead {
+            files: 4,
+            bytes: 4_000,
+            failed,
+            checks: checks.iter().map(|name| (*name).to_owned()).collect(),
+        }),
+    }));
+    harness.settle();
+    request
+}
+
+fn checked(harness: &mut Harness, request: RequestId, failure: Option<Failure>, left: usize) {
+    harness.app.apply(Action::Backend(Event::FileChecked {
+        request,
+        failure,
+        left,
+    }));
+    harness.settle();
+}
+
+#[test]
+fn the_check_stops_once_the_dialog_no_longer_waits_for_it() {
+    let stopped = |harness: &Harness, request: RequestId| {
+        harness.app.backend.sent.iter().any(
+            |command| matches!(command, Command::StopCheck { request: stop } if *stop == request),
+        )
+    };
+    // The reading changes: the Test is forgotten, its check stopped.
+    let mut harness = exports(Look::macos());
+    let request = tested(&mut harness, Vec::new(), &["orders", "returns"]);
+    assert!(!stopped(&harness, request));
+    harness.app.apply(Action::SetCsv(CsvChange::Header(
+        tabletist_db::files::Header::Yes,
+    )));
+    assert!(stopped(&harness, request));
+    // The dialog closes.
+    let request = tested(&mut harness, Vec::new(), &["orders", "returns"]);
+    harness.app.apply(Action::CloseDialog);
+    assert!(stopped(&harness, request));
+    // The Test runs again: the last one's check is stopped, and what it
+    // still says is not taken.
+    let mut harness = exports(Look::macos());
+    let request = tested(&mut harness, Vec::new(), &["orders", "returns"]);
+    harness.app.apply(Action::TestConnection);
+    assert!(stopped(&harness, request));
+    checked(&mut harness, request, Some(late_na()), 0);
+    assert!(form(&harness).first_failure().is_none());
+    // Let go of before it answers: the check that would follow is
+    // stopped, the dialog closed or the Test run again.
+    let mut harness = exports(Look::macos());
+    harness.app.apply(Action::TestConnection);
+    let TestState::Running(request) = form(&harness).test else {
+        panic!("no test");
+    };
+    harness.app.apply(Action::CloseDialog);
+    assert!(stopped(&harness, request));
+    let mut harness = exports(Look::macos());
+    harness.app.apply(Action::TestConnection);
+    let TestState::Running(request) = form(&harness).test else {
+        panic!("no test");
+    };
+    harness.app.apply(Action::TestConnection);
+    assert!(stopped(&harness, request));
+    // A check that ends is not stopped.
+    let mut harness = exports(Look::macos());
+    let request = tested(&mut harness, Vec::new(), &["returns"]);
+    checked(&mut harness, request, None, 0);
+    harness.app.apply(Action::CloseDialog);
+    assert!(!stopped(&harness, request));
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p tabletist-core --lib -- a_fix_mends a_files_test_checks` and `cargo test -p tabletist --lib partial_test_tests`

Expected: FAIL to compile, as `Command::StopCheck`, `Event::FileChecked`, `FileFix` and `first_failure` do not exist yet:

```text
error[E0599]: no variant named `FileChecked` found for enum `backend::Event`
error[E0599]: no variant named `StopCheck` found for enum `backend::Command`
error[E0599]: no method named `fix_file` found for struct `model::ConnectionForm` in the current scope
error[E0433]: cannot find type `FileFix` in this scope
error[E0599]: no method named `first_failure` found for struct `model::ConnectionForm` in the current scope
Some errors have detailed explanations: E0433, E0599.
For more information about an error, try `rustc --explain E0433`.
error: could not compile `tabletist-core` (lib test) due to 18 previous errors
error[E0599]: no variant named `FileChecked` found for enum `tabletist_core::backend::Event`
error[E0599]: no variant named `StopCheck` found for enum `tabletist_core::backend::Command`
error[E0599]: no method named `first_failure` found for reference `&tabletist_core::model::ConnectionForm` in the current scope
For more information about this error, try `rustc --explain E0599`.
```

- [ ] **Step 3: Check in the background, hold and stop the check, fix**

```diff
diff --git a/crates/tabletist-core/src/backend.rs b/crates/tabletist-core/src/backend.rs
--- a/crates/tabletist-core/src/backend.rs
+++ b/crates/tabletist-core/src/backend.rs
@@ -77,6 +77,12 @@
         spec: ConnectSpec,
         secrets: Secrets,
         host_keys: HostKeys,
+    },
+    /// Stops the check in full that a Files connection's Test `request`
+    /// started, where it still runs: the dialog closed, or what it tested
+    /// changed.
+    StopCheck {
+        request: RequestId,
     },
     Close {
         session: SessionId,
@@ -486,6 +492,13 @@
         request: RequestId,
         result: Result<tabletist_db::FilesRead, Error>,
     },
+    /// One file of Test `request`'s check in full is read: to its end
+    /// (`failure` none), or not; `left` files are still to check.
+    FileChecked {
+        request: RequestId,
+        failure: Option<tabletist_db::files::Failure>,
+        left: usize,
+    },
     /// The Host aliases in ~/.ssh/config, for the connection dialog.
     SshHosts {
         request: RequestId,
@@ -2179,6 +2192,9 @@
     updates: updates::Updates,
     /// What stops each export that runs.
     exports: Arc<Mutex<HashMap<export::ExportId, export::Stopper>>>,
+    /// The checks in full that Files Tests run, by their Test: aborting
+    /// one drops its session, which stops DuckDB's statement.
+    checks: HashMap<RequestId, tokio::task::AbortHandle>,
     #[cfg(test)]
     watched: Watched,
 }
@@ -2200,6 +2216,7 @@
             history: history::History::default(),
             updates: updates::Updates::default(),
             exports: Arc::default(),
+            checks: HashMap::new(),
             #[cfg(test)]
             watched: Watched::default(),
         };
@@ -2339,38 +2356,77 @@
                 host_keys,
             } => {
                 let outbox = self.outbox.clone();
+                // A Files connection's Test reads the start of every file
+                // it can open, and says how much it read and which it
+                // could not; then it reads its CSV, TSV and NDJSON files to
+                // their end, one at a time, saying each.
+                let files = match spec.driver {
+                    tabletist_db::Driver::Files => true,
+                    tabletist_db::Driver::Postgres
+                    | tabletist_db::Driver::MySql
+                    | tabletist_db::Driver::Sqlite
+                    | tabletist_db::Driver::BigQuery => false,
+                };
+                if files {
+                    let task = tokio::spawn(async move {
+                        let connection = match Connection::open_files_to_test(&spec).await {
+                            Ok(connection) => connection,
+                            Err(error) => {
+                                outbox.emit(Event::FilesTested {
+                                    request,
+                                    result: Err(error),
+                                });
+                                return;
+                            }
+                        };
+                        let read =
+                            connection
+                                .read_files()
+                                .await
+                                .unwrap_or(Err(Error::Unsupported(
+                                    "only a Files connection reads files",
+                                )));
+                        let checks = match &read {
+                            Ok(read) => read.checks.clone(),
+                            Err(_) => Vec::new(),
+                        };
+                        outbox.emit(Event::FilesTested {
+                            request,
+                            result: read,
+                        });
+                        let mut left = checks.len();
+                        for name in checks {
+                            left -= 1;
+                            // A check that cannot finish says nothing of
+                            // the file: it is not a row the file has.
+                            let failure = connection.check_file(&name).await.ok().flatten();
+                            outbox.emit(Event::FileChecked {
+                                request,
+                                failure,
+                                left,
+                            });
+                        }
+                        let _ = connection.close().await;
+                    });
+                    self.checks.retain(|_, check| !check.is_finished());
+                    self.checks.insert(request, task.abort_handle());
+                    return;
+                }
                 tokio::spawn(async move {
                     let connected =
                         Connection::connect_with(&spec, &secrets, &host_keys, Access::ReadOnly)
                             .await;
-                    match spec.driver {
-                        // A Files connection's Test reads the start of every
-                        // file too, and says how much it read.
-                        tabletist_db::Driver::Files => {
-                            let result = match connected {
-                                Ok(connection) => {
-                                    let read = connection.read_files().await.unwrap_or(Err(
-                                        Error::Unsupported("only a Files connection reads files"),
-                                    ));
-                                    let closed = connection.close().await;
-                                    read.and_then(|read| closed.map(|()| read))
-                                }
-                                Err(error) => Err(error),
-                            };
-                            outbox.emit(Event::FilesTested { request, result });
-                        }
-                        tabletist_db::Driver::Postgres
-                        | tabletist_db::Driver::MySql
-                        | tabletist_db::Driver::Sqlite
-                        | tabletist_db::Driver::BigQuery => {
-                            let result = match connected {
-                                Ok(connection) => connection.close().await,
-                                Err(error) => Err(error),
-                            };
-                            outbox.emit(Event::Tested { request, result });
-                        }
-                    }
+                    let result = match connected {
+                        Ok(connection) => connection.close().await,
+                        Err(error) => Err(error),
+                    };
+                    outbox.emit(Event::Tested { request, result });
                 });
+            }
+            Command::StopCheck { request } => {
+                if let Some(check) = self.checks.remove(&request) {
+                    check.abort();
+                }
             }
             Command::Close { session } => {
                 if let Some(handle) = self.sessions.remove(&session) {
@@ -2654,6 +2710,7 @@
         | Command::ServerVersion { session, .. }
         | Command::SessionZone { session, .. } => *session,
         Command::Test { .. }
+        | Command::StopCheck { .. }
         | Command::LoadSecret { .. }
         | Command::StoreSecret { .. }
         | Command::Save { .. }
@@ -2720,6 +2777,7 @@
         | Command::SessionZone { request, .. } => vec![*request],
         Command::Connect { .. }
         | Command::Test { .. }
+        | Command::StopCheck { .. }
         | Command::Close { .. }
         | Command::Cancel { .. }
         | Command::LoadSecret { .. }
@@ -2865,6 +2923,7 @@
         },
         Command::Connect { .. }
         | Command::Test { .. }
+        | Command::StopCheck { .. }
         | Command::Close { .. }
         | Command::Cancel { .. }
         | Command::LoadSecret { .. }
@@ -3299,6 +3358,7 @@
             }
             Command::Connect { .. }
             | Command::Test { .. }
+            | Command::StopCheck { .. }
             | Command::Close { .. }
             | Command::Cancel { .. }
             | Command::LoadSecret { .. }
diff --git a/src/promo/live.rs b/src/promo/live.rs
--- a/src/promo/live.rs
+++ b/src/promo/live.rs
@@ -263,6 +263,7 @@
                 // Nothing is written, saved, stored, watched, exported,
                 // revealed, notified or closed here.
                 Command::Test { .. }
+                | Command::StopCheck { .. }
                 | Command::Close { .. }
                 | Command::Cancel { .. }
                 | Command::Export(_)
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -116,6 +116,9 @@
     SetFilesKind(FilesKind),
     /// A change in a Files connection's Reading CSV section.
     SetCsv(CsvChange),
+    /// One of the partial Test's fixes for the first file it could not
+    /// read: saved as an override in the form, and the Test run again.
+    FixFile(FileFix),
     /// Read what a Files connection's Source holds again, for the found
     /// line: after its folder is typed.
     ReadSource,
@@ -2158,8 +2161,12 @@
     /// Files: what the Source holds, as the backend last read it, for the
     /// found line and the list's rows.
     pub files_found: FilesFound,
-    /// Files: what the last Test that passed read.
+    /// Files: what the last Test that passed read, the files it could not
+    /// read among it, those its check in full found since included.
     pub files_tested: Option<tabletist_db::FilesRead>,
+    /// Files: the check in full that follows a Test, while it runs: the
+    /// Test it follows, and how many files are left to check.
+    pub files_check: Option<(RequestId, usize)>,
     /// Files: how its CSV and TSV files are read, overrides included.
     pub files_csv: tabletist_db::files::Reading,
     /// Files: whether the Reading CSV section is open. Closed each time
@@ -2193,6 +2200,17 @@
     Folder,
     /// Files added to a Files connection's list.
     Files,
+}
+
+/// A fix the partial Test offers for a file it could not read.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum FileFix {
+    /// Read the value's column as text.
+    Text,
+    /// Read the value as empty in its column.
+    Empty,
+    /// Leave the file out of the connection.
+    Skip,
 }
 
 /// A change in a Files connection's Reading CSV section.
@@ -2337,6 +2355,7 @@
             files_cursor: None,
             files_found: FilesFound::Unread,
             files_tested: None,
+            files_check: None,
             files_csv: tabletist_db::files::Reading::default(),
             csv_open: false,
             agents_open: false,
@@ -2404,6 +2423,61 @@
         self.test_secrets = tabletist_db::Secrets::default();
         self.test_waiting = 0;
         self.files_tested = None;
+        self.files_check = None;
+    }
+
+    /// The first file the last Test could not read, which the partial
+    /// banner says and its fixes mend.
+    pub fn first_failure(&self) -> Option<&tabletist_db::files::Failure> {
+        match self.test {
+            TestState::Passed => self.files_tested.as_ref()?.failed.first(),
+            TestState::Idle
+            | TestState::Running(_)
+            | TestState::Failed(_)
+            | TestState::Untrusted { .. } => None,
+        }
+    }
+
+    /// Mends the first file the last Test could not read as `fix` says: an
+    /// override under Reading CSV, which forgets the Test. False when there
+    /// is no such file, or `fix` changes a column and the failure is not of
+    /// a column's value. The override names the file by its name, or by
+    /// its path where the list holds another file of that name.
+    pub fn fix_file(&mut self, fix: FileFix) -> bool {
+        use tabletist_db::files::{Change, Override, Why};
+        let Some(failure) = self.first_failure().cloned() else {
+            return false;
+        };
+        let change = match (fix, &failure.why) {
+            (FileFix::Skip, _) => Change::Skip,
+            (FileFix::Text, Why::Value { column, .. }) if failure.takes_column_fixes() => {
+                Change::Text {
+                    column: column.clone(),
+                }
+            }
+            (FileFix::Empty, Why::Value { column, value, .. }) if failure.takes_column_fixes() => {
+                Change::Empty {
+                    column: column.clone(),
+                    values: vec![value.clone()],
+                }
+            }
+            (FileFix::Text | FileFix::Empty, _) => return false,
+        };
+        let name = tabletist_db::files::file_name(&failure.path);
+        let shared = self.files_kind == FilesKind::Files
+            && self
+                .files_list
+                .iter()
+                .map(|listed| tabletist_db::files::expanded(std::path::Path::new(listed)))
+                .filter(|listed| *listed != failure.path)
+                .any(|listed| tabletist_db::files::file_name(&listed) == name);
+        let file = match shared {
+            true => tabletist_db::files::shown(&failure.path),
+            false => name,
+        };
+        self.files_csv.overrides.push(Override { file, change });
+        self.forget_test();
+        true
     }
 
     /// The config's Host whose alias is the typed SSH host, compared as
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -337,6 +337,9 @@
     /// What a restart to update kept, while the relaunched windows open:
     /// each window's own, by its tab.
     relaunch_windows: HashMap<ConnTabId, crate::update::relaunch::Window>,
+    /// The check in full of a Files Test that the dialog waits for: it is
+    /// stopped once the dialog no longer does.
+    files_check: Option<RequestId>,
     /// The SQL editors a restart to update kept, while the relaunched
     /// windows open: each takes its own.
     relaunch: Option<crate::update::relaunch::Relaunch>,
@@ -540,6 +543,7 @@
             launch_arguments: Vec::new(),
             relaunch: None,
             relaunch_windows: HashMap::new(),
+            files_check: None,
             next_id: 1,
         };
         app.refresh_keymap();
@@ -1054,6 +1058,32 @@
     }
 
     pub fn apply(&mut self, action: Action) {
+        self.apply_action(action);
+        self.stop_stale_check();
+    }
+
+    /// Stops the check in full a Files Test started, once the dialog no
+    /// longer waits for it or its Test: it closed, or the Test was
+    /// forgotten or run again, before it answered or after.
+    fn stop_stale_check(&mut self) {
+        let Some(request) = self.files_check else {
+            return;
+        };
+        // From the Test's start: a Files Test checks its files once it
+        // answers, whether or not the dialog still waits for that answer.
+        let waited = matches!(
+            &self.dialog,
+            Some(Dialog::Connection(form))
+                if form.test == TestState::Running(request)
+                    || form.files_check.is_some_and(|(check, _)| check == request)
+        );
+        if !waited {
+            self.files_check = None;
+            self.backend.send(Command::StopCheck { request });
+        }
+    }
+
+    fn apply_action(&mut self, action: Action) {
         // What a prompt asks about must not change under it. The keys still
         // run while a dialog is open, and a click can be a frame behind it.
         if matches!(
@@ -3523,6 +3553,19 @@
                     self.apply(Action::TestConnection);
                 }
             }
+            Action::FixFile(fix) => {
+                let Some(Dialog::Connection(form)) = &mut self.dialog else {
+                    return;
+                };
+                if form.fix_file(fix) {
+                    // A skipped file leaves the found line, as it leaves
+                    // the connection.
+                    if fix == crate::model::FileFix::Skip {
+                        self.read_form_source();
+                    }
+                    self.apply(Action::TestConnection);
+                }
+            }
             Action::TestConnection => {
                 let request = RequestId(self.next_id());
                 let Some(Dialog::Connection(form)) = &mut self.dialog else {
@@ -3542,6 +3585,8 @@
                 };
                 form.message = None;
                 form.test = TestState::Running(request);
+                // The last Test's check in full speaks for it alone.
+                form.files_check = None;
                 // The clock starts when the Test is sent, not while a saved
                 // secret is read from the keyring.
                 form.test_started = None;
@@ -3591,6 +3636,7 @@
                 form.test_waiting = loads.len() as u8;
                 if loads.is_empty() {
                     form.test_started = Some(std::time::Instant::now());
+                    hold_check(&mut self.files_check, &mut self.backend, &spec, request);
                     self.backend.send(Command::Test {
                         request,
                         spec,
@@ -4633,6 +4679,24 @@
                     };
                 }
             }
+            Event::FileChecked {
+                request,
+                failure,
+                left,
+            } => {
+                if let Some(Dialog::Connection(form)) = &mut self.dialog
+                    && form.test == TestState::Passed
+                    && form.files_check.is_some_and(|(check, _)| check == request)
+                {
+                    form.files_check = (left > 0).then_some((request, left));
+                    if let (Some(failure), Some(read)) = (failure, &mut form.files_tested) {
+                        read.failed.push(failure);
+                    }
+                }
+                if left == 0 && self.files_check == Some(request) {
+                    self.files_check = None;
+                }
+            }
             Event::FilesTested { request, result } => {
                 if let Some(Dialog::Connection(form)) = &mut self.dialog
                     && form.test == TestState::Running(request)
@@ -4641,6 +4705,10 @@
                     form.test_took = if result.is_ok() { elapsed } else { None };
                     form.test = match result {
                         Ok(read) => {
+                            // Its CSV, TSV and NDJSON files are read to their
+                            // end next, in the background.
+                            form.files_check =
+                                (!read.checks.is_empty()).then_some((request, read.checks.len()));
                             form.files_tested = Some(read);
                             TestState::Passed
                         }
@@ -4654,6 +4722,10 @@
                             TestState::Failed(error.to_string())
                         }
                     };
+                    // No check follows: nothing is left to stop.
+                    if form.files_check.is_none() && self.files_check == Some(request) {
+                        self.files_check = None;
+                    }
                 }
             }
             Event::Schemas {
@@ -4975,6 +5047,12 @@
                                     && let Some(spec) = form.test_spec.clone()
                                 {
                                     form.test_started = Some(std::time::Instant::now());
+                                    hold_check(
+                                        &mut self.files_check,
+                                        &mut self.backend,
+                                        &spec,
+                                        test,
+                                    );
                                     self.backend.send(Command::Test {
                                         request: test,
                                         spec,
@@ -9772,6 +9850,33 @@
     }
     text.push_str(sql);
     text
+}
+
+/// Holds `request` as the Files Test whose check in full the dialog waits
+/// for, from the moment it is sent: the backend starts the check as soon as
+/// the Test answers, and only the App can stop it. Another one held is
+/// stopped. Nothing for another type's Test, which checks nothing.
+fn hold_check(
+    held: &mut Option<RequestId>,
+    backend: &mut Backend,
+    spec: &tabletist_db::ConnectSpec,
+    request: RequestId,
+) {
+    let files = match spec.driver {
+        tabletist_db::Driver::Files => true,
+        tabletist_db::Driver::Postgres
+        | tabletist_db::Driver::MySql
+        | tabletist_db::Driver::Sqlite
+        | tabletist_db::Driver::BigQuery => false,
+    };
+    if !files {
+        return;
+    }
+    if let Some(old) = held.replace(request)
+        && old != request
+    {
+        backend.send(Command::StopCheck { request: old });
+    }
 }
 
 #[cfg(test)]
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test -p tabletist-core --lib -- a_fix_mends a_files_test_checks` and `cargo test -p tabletist --lib partial_test_tests`

Expected: PASS. Then the full check, `--features shots` included: fmt and clippy clean, every test binary `ok` (5461 tests pass in this state).

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-core/src/backend.rs crates/tabletist-core/src/model.rs src/app.rs src/promo/live.rs src/ui/mod.rs src/ui/partial_test_tests.rs
git commit -m "Check a Files Test's files in full in the background, and test again after a fix"
```

---

### Task 3: The partial banner in the dialog, its fixes and keys

**Files:**
- Create: `src/ui/banner.rs`, `src/ui/connect_dialog/partial.rs`
- Modify: `src/ui/partial_test_tests.rs`, `src/ui/mod.rs`, `src/ui/connect_dialog/mod.rs`, `crates/tabletist-core/src/keymap.rs`, `docs/_reference/connections.md`, `docs/_guide/omarchy.md`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/partial_test_tests.rs b/src/ui/partial_test_tests.rs
--- a/src/ui/partial_test_tests.rs
+++ b/src/ui/partial_test_tests.rs
@@ -1,16 +1,25 @@
-//! A Files connection's partial Test in the connection dialog: the
-//! check in full that follows a Test, stopped once the dialog no
-//! longer waits for it.
+//! A Files connection's partial Test in the connection dialog, in both
+//! looks: the check in full that follows a Test and what the footer says
+//! while it runs, the banner over the footer for the first file that could
+//! not be read, its fixes (buttons, and `t`, `e`, `s` in the Omarchy look)
+//! that save an override and run the Test again, and the check stopped
+//! once the dialog no longer waits for it.
 
 use std::path::PathBuf;
 
+use egui::accesskit::Role;
+use egui::{Key, Modifiers};
 use tabletist_db::FilesRead;
-use tabletist_db::files::{Failure, Format, Found, Scan, Why};
+use tabletist_db::files::{Change, Failure, Format, Found, Override, Scan, Why};
 
 use crate::backend::{Command, Event, RequestId};
 use crate::model::{Action, ConnectionForm, CsvChange, Dialog, FilesFound, PickTarget, TestState};
-use crate::testing::Harness;
+use crate::testing::{Harness, node};
 use crate::theme::Look;
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
 
 fn form(harness: &Harness) -> &ConnectionForm {
     match &harness.app.dialog {
@@ -105,6 +114,256 @@
     harness.settle();
 }
 
+/// Whether a label starts with `text`.
+fn shows(harness: &mut Harness, text: &str) -> bool {
+    let tree = harness.settle();
+    crate::testing::labels(&tree)
+        .iter()
+        .any(|label| label.starts_with(text))
+}
+
+/// The fixes' buttons in `look`, as the banner names them.
+fn fixes(look: Look) -> [String; 3] {
+    match look.terminal {
+        true => [
+            "read returned_at as text".into(),
+            "read 'N/A' as empty".into(),
+            "skip returns.csv".into(),
+        ],
+        false => [
+            "Read returned_at as text".into(),
+            "Read 'N/A' as empty".into(),
+            "Skip returns.csv".into(),
+        ],
+    }
+}
+
+#[test]
+fn the_check_in_full_says_what_is_left_and_a_failure_brings_the_banner() {
+    for look in looks() {
+        let mut harness = exports(look);
+        let request = tested(&mut harness, Vec::new(), &["orders", "events", "returns"]);
+        assert!(
+            shows(&mut harness, &look.label("Read 4 files · 4.0 KB")),
+            "{}",
+            look.name
+        );
+        assert!(
+            shows(&mut harness, &look.label("checking 3 files…")) || {
+                let tree = harness.settle();
+                crate::testing::labels(&tree)
+                    .iter()
+                    .any(|label| label.ends_with(&look.label("checking 3 files…")))
+            }
+        );
+        checked(&mut harness, request, None, 2);
+        checked(&mut harness, request, None, 1);
+        let tree = harness.settle();
+        assert!(
+            crate::testing::labels(&tree)
+                .iter()
+                .any(|label| label.ends_with(&look.label("checking 1 file…"))),
+            "{}: {:?}",
+            look.name,
+            crate::testing::labels(&tree)
+        );
+        // No banner while every file reads.
+        assert!(node(&tree, &fixes(look)[2], Role::Button).is_none());
+        checked(&mut harness, request, Some(late_na()), 0);
+        let tree = harness.settle();
+        let labels = crate::testing::labels(&tree);
+        // The footer: what it read of how many, the file that failed.
+        let footer = format!(
+            "{} · returns.csv {}",
+            look.label("Read 3 of 4 files"),
+            look.label("failed")
+        );
+        assert!(labels.contains(&footer), "{}: {labels:?}", look.name);
+        // The banner, an alert, names the file, the row, the value, the
+        // column and its type, and offers three fixes.
+        let said = match look.terminal {
+            true => "! returns.csv row 48,112 has 'N/A' in returned_at, a date column",
+            false => {
+                "returns.csv couldn't be read. Row 48,112 has 'N/A' in returned_at, a date column."
+            }
+        };
+        assert!(
+            tree.nodes
+                .iter()
+                .any(|(_, node)| node.role() == Role::Alert && node.label() == Some(said)),
+            "{}: {labels:?}",
+            look.name
+        );
+        for fix in fixes(look) {
+            assert!(
+                node(&tree, &fix, Role::Button).is_some(),
+                "{}: {fix}",
+                look.name
+            );
+        }
+    }
+}
+
+#[test]
+fn each_fix_saves_an_override_under_reading_csv_and_tests_again() {
+    let overrides = [
+        Change::Text {
+            column: "returned_at".into(),
+        },
+        Change::Empty {
+            column: "returned_at".into(),
+            values: vec!["N/A".into()],
+        },
+        Change::Skip,
+    ];
+    for look in looks() {
+        for (at, change) in overrides.iter().enumerate() {
+            let mut harness = exports(look);
+            tested(&mut harness, vec![late_na()], &[]);
+            let sent = harness.app.backend.sent.len();
+            let reads = harness.app.backend.sources_read.len();
+            harness.click(&fixes(look)[at]);
+            // A skip reads the folder again, past the file it skips.
+            assert_eq!(
+                harness.app.backend.sources_read.len() - reads,
+                usize::from(matches!(change, Change::Skip)),
+                "{}: {change:?}",
+                look.name
+            );
+            assert_eq!(
+                form(&harness).files_csv.overrides,
+                [Override {
+                    file: "returns.csv".into(),
+                    change: change.clone(),
+                }],
+                "{}: {change:?}",
+                look.name
+            );
+            // The Test runs again, with the override.
+            assert!(matches!(form(&harness).test, TestState::Running(_)));
+            assert!(
+                harness.app.backend.sent[sent..]
+                    .iter()
+                    .any(|command| matches!(
+                        command,
+                        Command::Test { spec, .. } if spec.csv.overrides.len() == 1
+                    )),
+                "{}: {change:?}",
+                look.name
+            );
+        }
+    }
+}
+
+#[test]
+fn omarchy_runs_the_fixes_by_their_keys() {
+    let look = Look::omarchy();
+    for (key, change) in [
+        (
+            Key::T,
+            Change::Text {
+                column: "returned_at".into(),
+            },
+        ),
+        (
+            Key::E,
+            Change::Empty {
+                column: "returned_at".into(),
+                values: vec!["N/A".into()],
+            },
+        ),
+        (Key::S, Change::Skip),
+    ] {
+        let mut harness = exports(look);
+        tested(&mut harness, vec![late_na()], &[]);
+        // Off the name field, whose letters are typed.
+        harness.ctx.memory_mut(|memory| {
+            if let Some(focused) = memory.focused() {
+                memory.surrender_focus(focused);
+            }
+        });
+        harness.press(key, Modifiers::NONE);
+        assert_eq!(
+            form(&harness).files_csv.overrides,
+            [Override {
+                file: "returns.csv".into(),
+                change,
+            }],
+            "{key:?}"
+        );
+        assert!(matches!(form(&harness).test, TestState::Running(_)));
+        assert_eq!(form(&harness).name, "bookshop-exports", "{key:?} was typed");
+    }
+}
+
+#[test]
+fn a_failure_no_column_change_mends_offers_skip_alone() {
+    for look in looks() {
+        let mut harness = exports(look);
+        let wide = Failure {
+            why: Why::Columns {
+                row: 30_001,
+                expected: 3,
+                found: 4,
+            },
+            ..late_na()
+        };
+        tested(&mut harness, vec![wide], &[]);
+        let tree = harness.settle();
+        let [text, empty, skip] = fixes(look);
+        assert!(node(&tree, &skip, Role::Button).is_some(), "{}", look.name);
+        assert!(node(&tree, &text, Role::Button).is_none(), "{}", look.name);
+        assert!(node(&tree, &empty, Role::Button).is_none(), "{}", look.name);
+        let row = look.label("Row 30,001 has 4 values, where the file's rows have 3");
+        assert!(
+            tree.nodes.iter().any(|(_, node)| node.role() == Role::Alert
+                && node.label().is_some_and(|label| label.contains(&row))),
+            "{}",
+            look.name
+        );
+        // A column's fix does nothing here, by its key either.
+        if look.terminal {
+            harness.ctx.memory_mut(|memory| {
+                if let Some(focused) = memory.focused() {
+                    memory.surrender_focus(focused);
+                }
+            });
+            harness.press(Key::T, Modifiers::NONE);
+            assert!(form(&harness).files_csv.overrides.is_empty());
+        }
+    }
+}
+
+#[test]
+fn with_several_failed_the_footer_counts_them_and_the_banner_says_the_first() {
+    for look in looks() {
+        let mut harness = exports(look);
+        let events = Failure {
+            file: "events.ndjson".into(),
+            path: PathBuf::from(FOLDER).join("events.ndjson"),
+            format: Format::Ndjson,
+            why: Why::Json {
+                line: 7,
+                malformed: true,
+            },
+        };
+        tested(&mut harness, vec![events, late_na()], &[]);
+        let footer = look.label("Read 2 of 4 files · 2 failed");
+        assert!(shows(&mut harness, &footer), "{}", look.name);
+        let tree = harness.settle();
+        let skip = look.label("Skip ") + "events.ndjson";
+        assert!(node(&tree, &skip, Role::Button).is_some(), "{}", look.name);
+        assert!(
+            tree.nodes.iter().any(|(_, node)| node.role() == Role::Alert
+                && node
+                    .label()
+                    .is_some_and(|label| label.contains(&look.label("Line 7 isn't JSON")))),
+            "{}",
+            look.name
+        );
+    }
+}
+
 #[test]
 fn the_check_stops_once_the_dialog_no_longer_waits_for_it() {
     let stopped = |harness: &Harness, request: RequestId| {
@@ -155,3 +414,155 @@
     harness.app.apply(Action::CloseDialog);
     assert!(!stopped(&harness, request));
 }
+
+/// A file named in a list beside another of its name is named by its path.
+#[test]
+fn a_fix_names_a_file_by_its_path_where_the_list_has_two_of_its_name() {
+    let mut harness = exports(Look::macos());
+    match &mut harness.app.dialog {
+        Some(Dialog::Connection(form)) => {
+            form.files_kind = crate::model::FilesKind::Files;
+            form.files_list = vec![
+                "/data/current/returns.csv".into(),
+                "/data/archive/returns.csv".into(),
+            ];
+        }
+        _ => panic!("no dialog"),
+    }
+    let archived = Failure {
+        path: PathBuf::from("/data/archive/returns.csv"),
+        ..late_na()
+    };
+    tested(&mut harness, vec![archived], &[]);
+    harness.click("Skip returns.csv");
+    assert_eq!(
+        form(&harness).files_csv.overrides,
+        [Override {
+            file: "/data/archive/returns.csv".into(),
+            change: Change::Skip,
+        }]
+    );
+}
+
+/// A long value and name are cut short on the buttons, the banner says
+/// them in full, and the actions stay inside the banner, wrapping to more
+/// rows where they must.
+#[test]
+fn long_values_are_cut_short_on_the_buttons_and_the_actions_stay_inside() {
+    let value = "lost in the post on its way back to the shop".repeat(3);
+    for look in looks() {
+        let mut harness = exports(look);
+        let long = Failure {
+            why: Why::Value {
+                row: 9,
+                column: "returned_at_according_to_the_courier_service".into(),
+                value: value.clone(),
+                kind: "INTERVAL".into(),
+            },
+            ..late_na()
+        };
+        tested(&mut harness, vec![long], &[]);
+        let tree = harness.settle();
+        let cut: String = value.chars().take(31).collect();
+        let empty = format!("{} '{cut}…' {}", look.label("Read"), look.label("as empty"));
+        let empty = node(&tree, &empty, Role::Button)
+            .unwrap_or_else(|| panic!("{}: {:?}", look.name, crate::testing::labels(&tree)));
+        let (alert, said) = tree
+            .nodes
+            .iter()
+            .find(|(_, node)| node.role() == Role::Alert)
+            .map(|(id, node)| (*id, node.label().unwrap_or_default()))
+            .unwrap();
+        assert!(said.contains(&value), "{}: {said}", look.name);
+        assert!(
+            said.contains(&look.label("an interval column")),
+            "{}: {said}",
+            look.name
+        );
+        let bounds = |id| {
+            tree.nodes
+                .iter()
+                .find(|(each, _)| *each == id)
+                .and_then(|(_, node)| node.bounds())
+                .unwrap_or_else(|| panic!("no bounds: {id:?} alert {alert:?}"))
+        };
+        let (outer, inner) = (bounds(alert), bounds(empty));
+        assert!(
+            inner.x1 <= outer.x1 + 0.5 && inner.y1 <= outer.y1 + 0.5,
+            "{}",
+            look.name
+        );
+        let skip = node(&tree, &fixes(look)[2], Role::Button).unwrap();
+        let skip = bounds(skip);
+        assert!(
+            skip.x1 <= outer.x1 + 0.5 && skip.y1 <= outer.y1 + 0.5,
+            "{}",
+            look.name
+        );
+    }
+}
+
+/// A held fix key mends one file: its repeats do not mend the next once
+/// the Test has run again.
+#[test]
+fn a_held_fix_key_mends_one_file() {
+    let mut harness = exports(Look::omarchy());
+    let events = Failure {
+        file: "events.ndjson".into(),
+        path: PathBuf::from(FOLDER).join("events.ndjson"),
+        format: Format::Ndjson,
+        why: Why::Json {
+            line: 7,
+            malformed: true,
+        },
+    };
+    tested(&mut harness, vec![late_na(), events.clone()], &[]);
+    harness.ctx.memory_mut(|memory| {
+        if let Some(focused) = memory.focused() {
+            memory.surrender_focus(focused);
+        }
+    });
+    harness.frame(vec![crate::testing::key(Key::S, Modifiers::NONE)]);
+    // The Test runs again, and answers before the key comes up.
+    let TestState::Running(request) = form(&harness).test else {
+        panic!("no test");
+    };
+    harness.app.apply(Action::Backend(Event::FilesTested {
+        request,
+        result: Ok(FilesRead {
+            files: 3,
+            bytes: 3_000,
+            failed: vec![events],
+            checks: Vec::new(),
+        }),
+    }));
+    harness.frame(vec![crate::testing::key(Key::S, Modifiers::NONE)]);
+    harness.frame(vec![crate::testing::release(Key::S, Modifiers::NONE)]);
+    harness.settle();
+    assert_eq!(
+        form(&harness).files_csv.overrides,
+        [Override {
+            file: "returns.csv".into(),
+            change: Change::Skip,
+        }]
+    );
+}
+
+#[test]
+fn one_file_that_failed_reads_as_one_file() {
+    let mut harness = exports(Look::macos());
+    harness.app.apply(Action::TestConnection);
+    let TestState::Running(request) = form(&harness).test else {
+        panic!("no test");
+    };
+    harness.app.apply(Action::Backend(Event::FilesTested {
+        request,
+        result: Ok(FilesRead {
+            files: 1,
+            bytes: 1_000,
+            failed: vec![late_na()],
+            checks: Vec::new(),
+        }),
+    }));
+    assert!(shows(&mut harness, "Read 0 of 1 file · returns.csv failed"));
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p tabletist --lib partial_test_tests`

Expected: FAIL, 9 tests: the footer says no check, there is no banner, no fix, and `t`, `e`, `s` do nothing. (Task 2's stop test still passes.)

```text
thread 'ui::partial_test_tests::a_held_fix_key_mends_one_file' (793589) panicked at src/ui/partial_test_tests.rs:528:9:
thread 'ui::partial_test_tests::omarchy_runs_the_fixes_by_their_keys' (793595) panicked at src/ui/partial_test_tests.rs:286:9:
thread 'ui::partial_test_tests::one_file_that_failed_reads_as_one_file' (793596) panicked at src/ui/partial_test_tests.rs:567:5:
thread 'ui::partial_test_tests::with_several_failed_the_footer_counts_them_and_the_banner_says_the_first' (793600) panicked at src/ui/partial_test_tests.rs:352:9:
thread 'ui::partial_test_tests::a_fix_names_a_file_by_its_path_where_the_list_has_two_of_its_name' (793588) panicked at src/testing.rs:298:32:
thread 'ui::partial_test_tests::long_values_are_cut_short_on_the_buttons_and_the_actions_stay_inside' (793594) panicked at src/ui/partial_test_tests.rs:469:32:
thread 'ui::partial_test_tests::each_fix_saves_an_override_under_reading_csv_and_tests_again' (793592) panicked at src/testing.rs:298:32:
thread 'ui::partial_test_tests::a_failure_no_column_change_mends_offers_skip_alone' (793587) panicked at src/ui/partial_test_tests.rs:314:9:
thread 'ui::partial_test_tests::the_check_in_full_says_what_is_left_and_a_failure_brings_the_banner' (793597) panicked at src/ui/partial_test_tests.rs:151:9:
```

- [ ] **Step 3: Build the banner, the footer's words and the keys**

```diff
diff --git a/crates/tabletist-core/src/keymap.rs b/crates/tabletist-core/src/keymap.rs
--- a/crates/tabletist-core/src/keymap.rs
+++ b/crates/tabletist-core/src/keymap.rs
@@ -752,6 +752,9 @@
     FormAddFiles = "form_add_files",
     FormRemoveFile = "form_remove_file",
     FormResetOverride = "form_reset_override",
+    FormFixText = "form_fix_text",
+    FormFixEmpty = "form_fix_empty",
+    FormFixSkip = "form_fix_skip",
     SettingsMove = "settings_move",
     SettingsChange = "settings_change",
     SettingsToggle = "settings_toggle",
@@ -1223,6 +1226,21 @@
                 "",
                 Prompts,
             ),
+            C::FormFixText => info(
+                "Read the column a Files connection's Test could not read as text",
+                "",
+                Prompts,
+            ),
+            C::FormFixEmpty => info(
+                "Read the value a Files connection's Test could not read as empty",
+                "",
+                Prompts,
+            ),
+            C::FormFixSkip => info(
+                "Skip the file a Files connection's Test could not read",
+                "",
+                Prompts,
+            ),
             C::SettingsMove => info("Move in the settings", "", Prompts),
             C::SettingsChange => info("Change the option", "", Prompts),
             C::SettingsToggle => info("Toggle the option", "", Prompts),
@@ -2710,6 +2728,10 @@
         bind(C::FormAddFiles, PROMPT, NONE, keys(&[k("a")])).when(When::ConnectionForm),
         bind(C::FormRemoveFile, PROMPT, NONE, keys(&[k("x")])).when(When::ConnectionForm),
         bind(C::FormResetOverride, PROMPT, NONE, keys(&[k("R")])).when(When::ConnectionForm),
+        // The partial Test's banner: its fixes, a letter each.
+        bind(C::FormFixText, PROMPT, NONE, keys(&[k("t")])).when(When::ConnectionForm),
+        bind(C::FormFixEmpty, PROMPT, NONE, keys(&[k("e")])).when(When::ConnectionForm),
+        bind(C::FormFixSkip, PROMPT, NONE, keys(&[k("s")])).when(When::ConnectionForm),
         bind(
             C::SettingsMove,
             PROMPT,
diff --git a/src/ui/banner.rs b/src/ui/banner.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/banner.rs
@@ -0,0 +1,301 @@
+//! A banner with actions, as the Components boards draw it for Files: a
+//! sentence of what went wrong, and under it the ways to mend it.
+//!
+//! macOS: a box in the tone's tint with a line round it and corners of 8,
+//! the tone's triangle at its left, the lead in bold before the sentence,
+//! and under them the actions as buttons 28 high, 8 apart: bordered, the
+//! last one quiet (in the accent colour, no border) where it is marked so.
+//!
+//! The terminal look: a strip with a rule of 2 down its left in the tone's
+//! colour, its fill the tone's at a tenth over the panel. The lead in
+//! the tone's colour, `! returns.csv`, then the sentence; under them each
+//! action as its key in the accent colour and its words, three spaces
+//! apart.
+//!
+//! Its words are an alert for a screen reader, read out as they appear;
+//! each action is a button named by its words.
+
+use egui::{CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
+
+use crate::keymap::Written;
+use crate::style::Icon;
+use crate::theme::{self, Look, Palette};
+use crate::typography::{Laid, Text, TextRole};
+use crate::ui::states::Tone;
+use crate::ui::widgets::{self, ButtonSpec};
+
+/// The room inside the macOS box, and the terminal strip's.
+const PAD: egui::Vec2 = vec2(12.0, 10.0);
+const TERMINAL_PAD: egui::Vec2 = vec2(12.0, 8.0);
+/// The macOS icon, and the room after it.
+const ICON: f32 = 12.0;
+const ICON_GAP: f32 = 10.0;
+/// A macOS button's height and the room at its sides; the gap between two
+/// and above them.
+const BUTTON: f32 = 28.0;
+const BUTTON_PAD: f32 = 10.0;
+const GAP: f32 = 8.0;
+/// The terminal look's room between two actions: three spaces.
+const KEYS_GAP: &str = "   ";
+
+/// What a banner says, and what it offers.
+pub struct Banner<'a> {
+    pub tone: Tone,
+    /// What went wrong, in bold on macOS and in the tone's colour in the
+    /// terminal look: "returns.csv couldn't be read." or "! returns.csv".
+    pub lead: &'a str,
+    /// The sentence after it.
+    pub text: &'a str,
+    pub actions: Vec<Action<'a>>,
+    /// Tells the banner's buttons from others in its `Ui`.
+    pub salt: &'a str,
+}
+
+/// One way to mend what the banner says.
+pub struct Action<'a> {
+    /// The words, which name its button too.
+    pub label: &'a str,
+    /// Its key, shown before its words in the terminal look.
+    pub key: Option<Written>,
+    /// macOS draws it quiet: the accent colour, no border.
+    pub quiet: bool,
+}
+
+impl Banner<'_> {
+    /// The words a screen reader hears: the lead, then the sentence.
+    fn said(&self) -> String {
+        format!("{} {}", self.lead, self.text)
+    }
+
+    fn sentence(&self, ctx: &egui::Context, wrap: f32, look: &Look, palette: &Palette) -> Laid {
+        let color = self.tone.color(palette);
+        let (lead_role, role, lead_color, text_color) = match look.terminal {
+            true => (TextRole::OBody, TextRole::OBody, color, palette.text),
+            false => (
+                TextRole::UiBodySemibold,
+                TextRole::UiBody,
+                palette.text,
+                palette.text,
+            ),
+        };
+        Text::new(look)
+            .add(lead_role, self.lead, lead_color)
+            .space(role, " ")
+            .add(role, self.text, text_color)
+            .wrap(wrap)
+            .layout(ctx)
+    }
+
+    /// The terminal look's line of keys and words.
+    fn keys_line(&self, ctx: &egui::Context, look: &Look, palette: &Palette) -> Vec<(Laid, Laid)> {
+        self.actions
+            .iter()
+            .map(|action| {
+                let key = action.key.as_ref().map_or("", |key| key.as_str());
+                (
+                    Text::one(look, TextRole::OBody, key, palette.accent).layout(ctx),
+                    Text::one(look, TextRole::OBody, action.label, palette.text).layout(ctx),
+                )
+            })
+            .collect()
+    }
+
+    /// The width a macOS button takes, no wider than the row `width`.
+    fn button_width(ctx: &egui::Context, label: &str, width: f32, look: &Look) -> f32 {
+        let laid =
+            Text::one(look, widgets::body(look), label, egui::Color32::PLACEHOLDER).layout(ctx);
+        (laid.width() + 2.0 * BUTTON_PAD).ceil().min(width)
+    }
+
+    /// The width of an action in the terminal look: its key, a space, its
+    /// words. No wider than the row `width`.
+    fn key_width(ctx: &egui::Context, (key, words): &(Laid, Laid), width: f32, look: &Look) -> f32 {
+        (key.width() + space(ctx, look, " ") + words.width()).min(width)
+    }
+
+    /// Where each action goes, from the left of a row `width` wide, `wide`
+    /// each and `gap` apart: one that would pass the row's end starts the
+    /// next, `step` below.
+    fn flow(widths: &[f32], width: f32, gap: f32, step: f32) -> Vec<egui::Pos2> {
+        let (mut x, mut y) = (0.0, 0.0);
+        let mut places = Vec::with_capacity(widths.len());
+        for wide in widths {
+            if x > 0.0 && x + wide > width {
+                x = 0.0;
+                y += step;
+            }
+            places.push(pos2(x, y));
+            x += wide + gap;
+        }
+        places
+    }
+
+    /// Where each macOS button goes, and its width.
+    fn button_places(
+        &self,
+        ctx: &egui::Context,
+        width: f32,
+        look: &Look,
+    ) -> Vec<(egui::Pos2, f32)> {
+        let widths: Vec<f32> = self
+            .actions
+            .iter()
+            .map(|action| Self::button_width(ctx, action.label, width, look))
+            .collect();
+        Self::flow(&widths, width, GAP, BUTTON + GAP)
+            .into_iter()
+            .zip(widths)
+            .collect()
+    }
+
+    /// Where each of the terminal look's actions goes, by rows of `line`.
+    fn key_places(
+        &self,
+        ctx: &egui::Context,
+        width: f32,
+        look: &Look,
+        palette: &Palette,
+    ) -> Vec<(egui::Pos2, (Laid, Laid))> {
+        let line = TextRole::OBody.row_height(ctx, look.faces);
+        let laid = self.keys_line(ctx, look, palette);
+        let widths: Vec<f32> = laid
+            .iter()
+            .map(|pair| Self::key_width(ctx, pair, width, look))
+            .collect();
+        Self::flow(&widths, width, space(ctx, look, KEYS_GAP), line)
+            .into_iter()
+            .zip(laid)
+            .collect()
+    }
+
+    /// Its height `width` wide.
+    pub fn height(&self, ctx: &egui::Context, width: f32, look: &Look, palette: &Palette) -> f32 {
+        if look.terminal {
+            let inner = width - 2.0 - 2.0 * TERMINAL_PAD.x;
+            let line = TextRole::OBody.row_height(ctx, look.faces);
+            let keys = match self.key_places(ctx, inner, look, palette).last() {
+                Some((last, _)) => last.y + line,
+                None => 0.0,
+            };
+            return self.sentence(ctx, inner, look, palette).height() + keys + 2.0 * TERMINAL_PAD.y;
+        }
+        let inner = width - 2.0 * PAD.x - ICON - ICON_GAP;
+        let text = self.sentence(ctx, inner, look, palette).height();
+        let buttons = match self.button_places(ctx, inner, look).last() {
+            Some((last, _)) => GAP + last.y + BUTTON,
+            None => 0.0,
+        };
+        text + buttons + 2.0 * PAD.y
+    }
+
+    /// Draws it across the width `ui` has left. Returns the place in its
+    /// actions of the one pressed.
+    pub fn show(&self, ui: &mut Ui, look: &Look, palette: &Palette) -> Option<usize> {
+        let width = ui.available_width();
+        let height = self.height(ui.ctx(), width, look, palette);
+        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
+        let id = ui.id().with(("banner", self.salt));
+        // An alert: a screen reader reads it out when it appears.
+        let alert = ui.interact(rect, id, Sense::hover());
+        let said = self.said();
+        // Its role and words set on its node: a widget's would be a label's.
+        ui.ctx().accesskit_node_builder(alert.id, |node| {
+            node.set_role(egui::accesskit::Role::Alert);
+            node.set_label(said);
+            node.set_live(egui::accesskit::Live::Assertive);
+            node.set_bounds(egui::accesskit::Rect {
+                x0: rect.min.x.into(),
+                y0: rect.min.y.into(),
+                x1: rect.max.x.into(),
+                y1: rect.max.y.into(),
+            });
+        });
+        let color = self.tone.color(palette);
+        let mut pressed = None;
+        if look.terminal {
+            let fill = theme::mix(palette.panel, color, 0.1);
+            ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
+            let rule = Rect::from_min_size(rect.min, vec2(2.0, rect.height()));
+            ui.painter().rect_filled(rule, CornerRadius::ZERO, color);
+            let inner = width - 2.0 - 2.0 * TERMINAL_PAD.x;
+            let left = rect.left() + 2.0 + TERMINAL_PAD.x;
+            let sentence = self.sentence(ui.ctx(), inner, look, palette);
+            let top = rect.top() + TERMINAL_PAD.y;
+            sentence.paint(ui.painter(), pos2(left, top));
+            let line = TextRole::OBody.row_height(ui.ctx(), look.faces);
+            let below = top + sentence.height();
+            let space = space(ui.ctx(), look, " ");
+            for (at, ((place, (key, words)), action)) in self
+                .key_places(ui.ctx(), inner, look, palette)
+                .into_iter()
+                .zip(&self.actions)
+                .enumerate()
+            {
+                let wide = Self::key_width(ui.ctx(), &(key.clone(), words.clone()), inner, look);
+                let (x, y) = (left + place.x, below + place.y + line / 2.0);
+                let place = Rect::from_min_size(pos2(x, y - line / 2.0), vec2(wide, line));
+                let response = ui.interact(place, id.with(at), Sense::click());
+                name(&response, action.label);
+                if crate::ui::focus::shown(&response) || response.hovered() {
+                    ui.painter()
+                        .rect_filled(place, CornerRadius::ZERO, palette.selection);
+                }
+                key.paint_left(ui.painter(), x, y);
+                words.paint_left(ui.painter(), x + key.width() + space, y);
+                crate::ui::focus::hint(ui, &response, place, crate::ui::focus::Ring::Bar);
+                if response.clicked() {
+                    pressed = Some(at);
+                }
+            }
+            return pressed;
+        }
+        ui.painter().rect(
+            rect,
+            CornerRadius::same(8),
+            self.tone.fill(look, palette),
+            Stroke::new(widgets::hairline(ui), self.tone.line(look, palette)),
+            StrokeKind::Inside,
+        );
+        let icon = Rect::from_min_size(
+            pos2(rect.left() + PAD.x, rect.top() + PAD.y + 3.0),
+            vec2(ICON, ICON),
+        );
+        Icon::TriangleAlert.image(color, ICON).paint_at(ui, icon);
+        let left = icon.right() + ICON_GAP;
+        let inner = rect.right() - PAD.x - left;
+        let sentence = self.sentence(ui.ctx(), inner, look, palette);
+        sentence.paint(ui.painter(), pos2(left, rect.top() + PAD.y));
+        let top = rect.top() + PAD.y + sentence.height() + GAP;
+        let places = self.button_places(ui.ctx(), inner, look);
+        for (at, (action, (place, wide))) in self.actions.iter().zip(places).enumerate() {
+            let button =
+                Rect::from_min_size(pos2(left + place.x, top + place.y), vec2(wide, BUTTON));
+            let spec = ButtonSpec::new(action.label)
+                .padding(BUTTON_PAD)
+                .role(widgets::body(look))
+                .salt(self.salt);
+            let spec = if action.quiet { spec.link() } else { spec };
+            let mut child = ui.new_child(
+                egui::UiBuilder::new()
+                    .max_rect(button)
+                    .id_salt((self.salt, at)),
+            );
+            if spec.show_at(&mut child, button, look, palette).clicked() {
+                pressed = Some(at);
+            }
+        }
+        pressed
+    }
+}
+
+/// Says `response` is a button named `name`.
+fn name(response: &Response, name: &str) {
+    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
+}
+
+/// The width of `text` in the terminal look's body.
+fn space(ctx: &egui::Context, look: &Look, text: &str) -> f32 {
+    Text::one(look, TextRole::OBody, text, egui::Color32::PLACEHOLDER)
+        .layout(ctx)
+        .width()
+}
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -17,6 +17,7 @@
 mod agent_settings_tests;
 #[cfg(test)]
 mod appearance_settings_tests;
+pub mod banner;
 pub mod calendar;
 #[cfg(test)]
 mod calendar_tests;
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -6,6 +6,7 @@
 mod agents;
 mod choice;
 mod files;
+mod partial;
 mod reading;
 mod sheet;
 mod terminal;
@@ -490,6 +491,23 @@
                             actions.push(Action::PickDataFiles);
                             false
                         }
+                        Some(
+                            command @ (Command::FormFixText
+                            | Command::FormFixEmpty
+                            | Command::FormFixSkip),
+                        ) if files && form.first_failure().is_some() => {
+                            // Once a press: a held key would mend the next
+                            // file too once the Test runs again, unseen.
+                            if *repeat {
+                                return false;
+                            }
+                            actions.push(Action::FixFile(match command {
+                                Command::FormFixText => crate::model::FileFix::Text,
+                                Command::FormFixEmpty => crate::model::FileFix::Empty,
+                                _ => crate::model::FileFix::Skip,
+                            }));
+                            false
+                        }
                         Some(Command::FormResetOverride) if files => {
                             // Once a press: a held `R` would take off the
                             // override that moves up into its place, and on.
@@ -1145,6 +1163,9 @@
     cap: f32,
     /// An unknown SSH host key: the button that trusts it follows.
     untrusted: bool,
+    /// A Files connection's partial Test: the banner of the first file it
+    /// could not read, with its fixes, under the notices, and its height.
+    partial: Option<(partial::Partial, f32)>,
 }
 
 impl Messages {
@@ -1185,10 +1206,16 @@
                 })
                 .collect()
         };
+        let partial = partial::of(ctx, form, skin).map(|partial| {
+            let width = width - 2.0 * Self::side(skin);
+            let height = partial.banner().height(ctx, width, look, skin.palette);
+            (partial, height)
+        });
         let mut messages = Self {
             notices: lay(wrap),
             cap,
             untrusted,
+            partial,
         };
         // Texts that will scroll leave room for the scroll bar.
         if messages.texts_height(skin) > cap {
@@ -1238,7 +1265,7 @@
     /// The height the messages take in the dialog: nothing when nothing
     /// went wrong.
     fn height(&self, skin: &Skin) -> f32 {
-        if self.notices.is_empty() {
+        if self.notices.is_empty() && self.partial.is_none() {
             return 0.0;
         }
         let button = if self.untrusted {
@@ -1246,13 +1273,23 @@
         } else {
             0.0
         };
-        Self::above(skin) + self.texts_height(skin).min(self.cap) + button + Self::below(skin)
+        let banner = match (&self.partial, self.notices.is_empty()) {
+            (Some((_, height)), true) => *height,
+            (Some((_, height)), false) => Self::GAP + height,
+            (None, _) => 0.0,
+        };
+        Self::above(skin)
+            + self.texts_height(skin).min(self.cap)
+            + button
+            + banner
+            + Self::below(skin)
     }
 
     fn show(self, ui: &mut Ui, skin: &Skin, actions: &mut Vec<Action>) {
-        if self.notices.is_empty() {
+        if self.notices.is_empty() && self.partial.is_none() {
             return;
         }
+        let noticed = !self.notices.is_empty();
         let Skin { look, palette, .. } = *skin;
         let side = Self::side(skin) as i8;
         let pad = Self::pad(skin);
@@ -1265,29 +1302,44 @@
             })
             .show(ui, |ui| {
                 ui.set_width(ui.available_width());
-                egui::ScrollArea::vertical()
-                    .id_salt("messages")
-                    .max_height(self.cap)
-                    .auto_shrink([false, true])
-                    .show(ui, |ui| {
-                        for (index, (laid, color)) in self.notices.into_iter().enumerate() {
-                            if index > 0 {
-                                ui.add_space(Self::GAP);
+                if noticed {
+                    egui::ScrollArea::vertical()
+                        .id_salt("messages")
+                        .max_height(self.cap)
+                        .auto_shrink([false, true])
+                        .show(ui, |ui| {
+                            for (index, (laid, color)) in self.notices.into_iter().enumerate() {
+                                if index > 0 {
+                                    ui.add_space(Self::GAP);
+                                }
+                                if look.terminal {
+                                    laid.label(ui);
+                                    continue;
+                                }
+                                egui::Frame::new()
+                                    .fill(theme::mix(skin.fill, color, 0.08))
+                                    .corner_radius(CornerRadius::same(10))
+                                    .inner_margin(egui::Margin::symmetric(pad.x as i8, pad.y as i8))
+                                    .show(ui, |ui| {
+                                        ui.set_width(ui.available_width());
+                                        laid.label(ui);
+                                    });
                             }
-                            if look.terminal {
-                                laid.label(ui);
-                                continue;
-                            }
-                            egui::Frame::new()
-                                .fill(theme::mix(skin.fill, color, 0.08))
-                                .corner_radius(CornerRadius::same(10))
-                                .inner_margin(egui::Margin::symmetric(pad.x as i8, pad.y as i8))
-                                .show(ui, |ui| {
-                                    ui.set_width(ui.available_width());
-                                    laid.label(ui);
-                                });
-                        }
-                    });
+                        });
+                }
+                // The partial Test's banner, over the footer, with its fixes.
+                if let Some((partial, _)) = &self.partial {
+                    if noticed {
+                        ui.add_space(Self::GAP);
+                    }
+                    if let Some(fix) = partial
+                        .banner()
+                        .show(ui, look, palette)
+                        .and_then(|at| partial.fix(at))
+                    {
+                        actions.push(Action::FixFile(fix));
+                    }
+                }
                 // The way out of an unknown host key stays in reach however
                 // long the text above it is.
                 if self.untrusted {
@@ -1310,6 +1362,8 @@
 enum Status {
     Running(String),
     Passed(String),
+    /// A Files connection's Test read some of its files, not all.
+    Partial(String),
 }
 
 fn status(form: &ConnectionForm, skin: &Skin) -> Option<Status> {
@@ -1326,6 +1380,44 @@
                 Driver::Files => form.files_tested.as_ref(),
                 Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery => None,
             };
+            // What a Files connection's check in full still has to read:
+            // "checking 2 files…".
+            let checking = match form.files_check {
+                Some((_, 1)) => Some(gettext(skin.locale, "checking 1 file…").into_owned()),
+                Some((_, left)) => Some(format!(
+                    "{} {left} {}",
+                    gettext(skin.locale, "checking"),
+                    gettext(skin.locale, "files…")
+                )),
+                None => None,
+            };
+            // Some files it could not read: "Read 3 of 4 files · returns.csv
+            // failed", or with several, how many.
+            if let Some(read) = read.filter(|read| !read.failed.is_empty()) {
+                // The file's own name keeps its case: only the words take
+                // the look's.
+                let failed = skin.say("failed");
+                let failed = match read.failed.as_slice() {
+                    [one] => format!("{} {failed}", one.file),
+                    many => format!("{} {failed}", many.len()),
+                };
+                let mut text = skin.look.label(&format!(
+                    "{} {} {} {} {}",
+                    gettext(skin.locale, "Read"),
+                    read.files - read.failed.len(),
+                    gettext(skin.locale, "of"),
+                    read.files,
+                    match read.files {
+                        1 => gettext(skin.locale, "file"),
+                        _ => gettext(skin.locale, "files"),
+                    },
+                ));
+                text = format!("{text} · {failed}");
+                if let Some(checking) = checking {
+                    text = format!("{text} · {}", skin.look.label(&checking));
+                }
+                return Some(Status::Partial(text));
+            }
             let mut text = match read {
                 // What a Files connection's Test read: "Read 3 files ·
                 // 2.4 GB".
@@ -1343,6 +1435,9 @@
             if let Some(took) = form.test_took {
                 text = format!("{text} · {}", crate::format::elapsed(took));
             }
+            if let Some(checking) = checking {
+                text = format!("{text} · {checking}");
+            }
             Some(Status::Passed(skin.look.label(&text)))
         }
         TestState::Idle | TestState::Failed(_) | TestState::Untrusted { .. } => None,
@@ -1368,6 +1463,7 @@
     let (text, color) = match &status {
         Status::Running(text) => (text, palette.dim),
         Status::Passed(text) => (text, palette.success),
+        Status::Partial(text) => (text, palette.warning),
     };
     let laid = Text::one(look, role, text, color).layout(ui.ctx());
     // The mark and the gap after it. The terminal look's are smaller:
@@ -1388,6 +1484,9 @@
             ui.put(place, egui::Spinner::new().size(mark).color(color));
         }
         Status::Passed(_) => paint_check(ui.painter(), place, color),
+        Status::Partial(_) => crate::style::Icon::TriangleAlert
+            .image(color, mark)
+            .paint_at(ui, place),
     }
     let x = left + mark + gap;
     laid.paint_left(ui.painter(), x, y);
diff --git a/src/ui/connect_dialog/partial.rs b/src/ui/connect_dialog/partial.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/connect_dialog/partial.rs
@@ -0,0 +1,218 @@
+//! The partial Test of a Files connection: what the banner over the
+//! footer says of the first file the Test could not read, and the fixes it
+//! offers. A value its column's type cannot take in a CSV or TSV has three
+//! (read the column as text, read the value as empty, skip the file); any
+//! other failure has the last alone. With several, the next shows once the
+//! first is mended and the Test has run again.
+
+use tabletist_db::files::{Failure, Why};
+
+use super::Skin;
+use crate::format::group_digits;
+use crate::keymap::Command;
+use crate::model::{ConnectionForm, FileFix};
+use crate::ui::banner::{Action, Banner};
+use crate::ui::states::Tone;
+
+/// The banner's words and fixes, made for one frame.
+pub(super) struct Partial {
+    lead: String,
+    text: String,
+    fixes: Vec<Fix>,
+}
+
+struct Fix {
+    label: String,
+    fix: FileFix,
+    key: Option<crate::keymap::Written>,
+    quiet: bool,
+}
+
+impl Partial {
+    pub(super) fn banner(&self) -> Banner<'_> {
+        Banner {
+            tone: Tone::Warning,
+            lead: &self.lead,
+            text: &self.text,
+            actions: self
+                .fixes
+                .iter()
+                .map(|fix| Action {
+                    label: &fix.label,
+                    key: fix.key.clone(),
+                    quiet: fix.quiet,
+                })
+                .collect(),
+            salt: "partial",
+        }
+    }
+
+    /// The fix of the action at `at`, as the banner counts them.
+    pub(super) fn fix(&self, at: usize) -> Option<FileFix> {
+        self.fixes.get(at).map(|fix| fix.fix)
+    }
+}
+
+/// `template` with each `{name}` in it put as `pairs` say: the template is
+/// translated and in the look's case, what goes in it as it is (a file's
+/// or a column's name, a value). One pass: what goes in is not read for
+/// names again, whatever a file's value says.
+fn put(skin: &Skin, template: &'static str, pairs: &[(&str, &str)]) -> String {
+    let template = skin.say(template);
+    let mut text = String::with_capacity(template.len());
+    let mut rest = template.as_str();
+    while let Some(open) = rest.find('{') {
+        text.push_str(&rest[..open]);
+        let after = &rest[open + 1..];
+        let named = after.find('}').and_then(|close| {
+            let name = &after[..close];
+            pairs
+                .iter()
+                .find(|(each, _)| *each == name)
+                .map(|(_, value)| (close, *value))
+        });
+        match named {
+            Some((close, value)) => {
+                text.push_str(value);
+                rest = &after[close + 1..];
+            }
+            None => {
+                text.push('{');
+                rest = after;
+            }
+        }
+    }
+    text.push_str(rest);
+    text
+}
+
+/// The most of a file's value or name a fix's button says, cut short with
+/// `…` past it: the banner says it in full.
+const SHORT: usize = 32;
+
+fn short(text: &str) -> String {
+    match text.chars().count() > SHORT {
+        true => format!("{}…", text.chars().take(SHORT - 1).collect::<String>()),
+        false => text.to_owned(),
+    }
+}
+
+/// The banner for the first file `form`'s last Test could not read.
+pub(super) fn of(ctx: &egui::Context, form: &ConnectionForm, skin: &Skin) -> Option<Partial> {
+    let failure = form.first_failure()?;
+    let terminal = skin.look.terminal;
+    let file = failure.file.as_str();
+    let lead = match terminal {
+        true => format!("! {file}"),
+        false => put(skin, "{file} couldn't be read.", &[("file", file)]),
+    };
+    let mut text = sentence(failure, skin);
+    // The terminal look's line is a clause, with no stop.
+    if terminal {
+        text = text.trim_end_matches('.').to_owned();
+    }
+    let key =
+        |command: Command| terminal.then(|| crate::ui::keys::written(ctx, skin.look, command));
+    let mut fixes = Vec::new();
+    if let Why::Value { column, value, .. } = &failure.why
+        && failure.takes_column_fixes()
+    {
+        fixes.push(Fix {
+            label: put(skin, "Read {column} as text", &[("column", &short(column))]),
+            fix: FileFix::Text,
+            key: key(Command::FormFixText),
+            quiet: false,
+        });
+        fixes.push(Fix {
+            label: put(skin, "Read '{value}' as empty", &[("value", &short(value))]),
+            fix: FileFix::Empty,
+            key: key(Command::FormFixEmpty),
+            quiet: false,
+        });
+    }
+    fixes.push(Fix {
+        label: put(skin, "Skip {file}", &[("file", &short(file))]),
+        fix: FileFix::Skip,
+        key: key(Command::FormFixSkip),
+        quiet: true,
+    });
+    Some(Partial { lead, text, fixes })
+}
+
+/// What the banner says of why the file could not be read.
+fn sentence(failure: &Failure, skin: &Skin) -> String {
+    match &failure.why {
+        Why::Value {
+            row,
+            column,
+            value,
+            kind,
+        } => put(
+            skin,
+            "Row {row} has '{value}' in {column}, {kind}.",
+            &[
+                ("row", &group_digits(*row)),
+                ("value", value),
+                ("column", column),
+                ("kind", &kind_word(kind, skin)),
+            ],
+        ),
+        Why::Columns {
+            row,
+            expected,
+            found,
+        } => put(
+            skin,
+            "Row {row} has {found} values, where the file's rows have {expected}.",
+            &[
+                ("row", &group_digits(*row)),
+                ("found", &found.to_string()),
+                ("expected", &expected.to_string()),
+            ],
+        ),
+        Why::Json {
+            line,
+            malformed: true,
+        } => put(
+            skin,
+            "Line {line} isn't JSON.",
+            &[("line", &group_digits(*line))],
+        ),
+        Why::Json {
+            line,
+            malformed: false,
+        } => put(
+            skin,
+            "Line {line} has a value of another type than the lines before it.",
+            &[("line", &group_digits(*line))],
+        ),
+        Why::Other(message) => message.clone(),
+    }
+}
+
+/// DuckDB's type as the words for what its column holds: `DATE` is "a
+/// date column".
+fn kind_word(kind: &str, skin: &Skin) -> String {
+    let kind = kind.to_ascii_uppercase();
+    let words: &'static str = match kind.as_str() {
+        "DATE" => "a date column",
+        "TIME" => "a time column",
+        kind if kind.starts_with("TIMESTAMP") => "a timestamp column",
+        "BIGINT" | "INTEGER" | "SMALLINT" | "TINYINT" | "HUGEINT" | "UBIGINT" | "UINTEGER"
+        | "USMALLINT" | "UTINYINT" | "UHUGEINT" => "a whole number column",
+        "DOUBLE" | "FLOAT" => "a number column",
+        kind if kind.starts_with("DECIMAL") => "a number column",
+        "BOOLEAN" => "a true or false column",
+        "UUID" => "a UUID column",
+        "INTERVAL" => "an interval column",
+        // DuckDB's own name in lower case: "a column of type blob".
+        _ => {
+            return format!(
+                "{} {}",
+                skin.say("a column of type"),
+                kind.to_ascii_lowercase()
+            );
+        }
+    };
+    skin.say(words)
+}
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -105,7 +105,23 @@
   section is closed, and says the access on one line.
 - **Test** reads each file's columns and its first rows, never a whole
   file to the end. With **Column types** from all rows, DuckDB reads a
-  CSV in full to find its types, which takes longer on a large file. The Excel reader is downloaded the first time a connection opens
+  CSV in full to find its types, which takes longer on a large file.
+  After a Test that read, the CSV, TSV and NDJSON files are read to
+  their end in the background while the dialog is open, one at a time:
+  the footer adds "checking 2 files…". That is how a value past the
+  first rows is found, such as an `N/A` on row 48,112 of a date column.
+- A file that cannot be read makes the Test **partial**: the footer says
+  "Read 3 of 4 files · returns.csv failed" (with several, how many), and
+  a banner over it names the file, the row, the value and the column.
+  For a value its column cannot take it offers three fixes: **Read
+  returned_at as text**, **Read 'N/A' as empty** (that value becomes
+  NULL in that column) and **Skip returns.csv**; for anything else, such
+  as a row with more values than the others or a line of NDJSON that is
+  not JSON, it offers to skip the file. In the Omarchy look `t`, `e` and
+  `s` run them. Each fix is an override under Reading CSV, and the Test
+  runs again; it is saved with the connection. A file that cannot be
+  opened at all still stops the connection from opening: Test shows it
+  as partial, with its fixes. The Excel reader is downloaded the first time a connection opens
   an Excel file.
 - The SQL editor runs DuckDB's SQL, and the Structure view shows each
   file's path, size and format beside its columns, and what it is read
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -106,6 +106,7 @@
 | `a` | A Files connection's list: add files |
 | `x` | A Files connection's list: take the file under the cursor off the list. The file stays on disk |
 | `R` | A Files connection's reading csv: take the override the cursor is on off. Other overrides still apply |
+| `t`, `e`, `s` | A Files connection's partial Test: read the column of the value that could not be read as text, read that value as empty, or skip the file. Each is saved as an override under reading csv, and the Test runs again |
 | Esc | Cancel |
 
 {% include shot.html file="omarchy-connection-edit" alt="The connection form in the Omarchy look, for a production database behind an SSH tunnel" width="2200" height="2032" %}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test -p tabletist --lib partial_test_tests` and `cargo test -p tabletist --lib -- files_tests reading_csv_tests`

Expected: PASS. Then the full check: fmt and clippy clean, every test binary `ok` (5470 tests pass in this state).

- [ ] **Step 5: Look at it**

A throwaway scene appended to `src/shots.rs` (never committed; take it out, then run the full check again before the commit) renders the Files dialog with a partial Test: `returns.csv`'s row 48,112 with `N/A` in its date column `returned_at`, two files still to check. `cargo test --features shots --lib shots::<scene> -- --ignored`, then shrink with `magick` and look. Expect: macOS, the warning box over the footer with the triangle, "**returns.csv couldn't be read.** Row 48,112 has 'N/A' in returned_at, a date column.", the buttons Read returned_at as text and Read 'N/A' as empty, and Skip returns.csv in the accent colour; the footer "⚠ Read 3 of 4 files · returns.csv failed · checking 2 files…". Omarchy, the strip with its rule, `! returns.csv row 48,112 has 'N/A' in returned_at, a date column`, then `t read returned_at as text   e read 'N/A' as empty   s skip returns.csv`.

- [ ] **Step 6: Commit**

```bash
git add src/ui/banner.rs src/ui/connect_dialog/partial.rs src/ui/connect_dialog/mod.rs src/ui/mod.rs src/ui/partial_test_tests.rs crates/tabletist-core/src/keymap.rs docs/_reference/connections.md docs/_guide/omarchy.md
git commit -m "Show a partial Files Test's banner over the footer, with its fixes and keys"
```

---

## After the three tasks

- Rebase onto `main` once #353 has merged, push the branch and open a pull request against `main`. Its description lists: what the Test now reads and the check in full; the banner, its fixes and Omarchy's keys; the shared banner component; the five departures above; that 2b-2 brings the banner to the table view.
- No screenshot goes to GitHub unless it is blurred.
