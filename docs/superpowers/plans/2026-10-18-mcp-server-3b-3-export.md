# MCP server, Phase 3b, run 3: export of the log, and the guide. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Agents tab's Log view exports the calls that pass its filters, every one of them, to a file the person chooses: CSV, or JSON Lines for a name ending in `.jsonl`. On macOS it is the board's **Export…**; on Omarchy `E`. The website gains a guide to the log.

**Architecture:** `Backend::export_log` (`src/backend.rs`) asks where to save through the save dialog, as `save_bytes` does, then `agents::export_log` (`src/backend/agents.rs`) reads the log's calls by the filter a page at a time on the backend's runtime, `exported` writes each page as `LogFormat::of` the name says, and `util::write_atomic_with` saves the file as it goes, private to the user. `Action::ExportAgentsLog` sends the view's filter; the view draws the button and the keymap gives Omarchy `E` (`When::AgentsLog`).

**Tech Stack:** Rust 2024, `rfd`, `serde_json`, `jiff`, egui.

---

## Where this sits

Phase 3b of `docs/superpowers/plans/2026-10-13-mcp-server.md`, the Log view in full, in three runs. Run 1 (pull request 163) built the filters and the search, run 2 (pull request 166) the selected call's pane and Open in SQL editor. This run, 3, is the last: **Export…** and the website's guide to the log. Phase 2, approved writes, comes next.

## What the plan asks, and what is built

| The plan | Built | Why |
|---|---|---|
| C.1: "The rows that pass the current filters, as CSV or as JSON Lines, through the save dialog the app already has (`rfd`), written on the backend runtime" | One **Export…**; the name's ending decides: `.jsonl` (or `.ndjson`) is JSON Lines, anything else CSV, as the dialog's title says | Agreed on 2026-10-18; the dialog's file types pick nothing on Linux (found by the review) |
| The macOS board: **Export…** at the bar's right | As drawn, off with no call to export | |
| The Omarchy board draws no key for it | `E` in the Log view, named in its bar and footer | Agreed on 2026-10-18 |
| Not in the plan | Every column the log keeps, `at` added as RFC 3339 in UTC; the newest first as the view lists them, read a page at a time; the file private to the user | What the log holds, readable by a person and a program; a long export never keeps the server's writes waiting; its statements and values can hold data |

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`. The Agents tab shows only where agents can reach the app (`cfg(any(unix, windows))`). The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.

## What was run before this plan was written

Every task's state was built here, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` with both test servers (3,758 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. Not run: CI, and the save dialog by hand (a test asks for it and does not open it): the button and the hints were looked at by eye in headless renders of both looks.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| How the format is chosen | One Export…; the name's ending: `.jsonl` (or `.ndjson`), in any case, is JSON Lines, anything else CSV. The dialog offers no file types; its title says so | Agreed on 2026-10-18; a pick renames nothing on Linux, and rfd does not say which was picked (found by the review) |
| Omarchy's key | `E` in the Log view (`When::AgentsLog`), with a call or more; named in the bar and the footer | Agreed on 2026-10-18 |
| What the file holds | Every call that passes the view's filters, not only the pages read, the newest first as the view lists them; every column the log keeps, and `at` as RFC 3339 in UTC | What the person sees, whole |
| How it is read and written | A thousand calls at a time, each read short, written as they are read | A long read kept the server from writing the calls that ended meanwhile, and held the whole log in memory (found by the review) |
| CSV and JSON Lines | CSV: RFC 4180, the names first, CR LF, empty for nothing. JSON Lines: an object a line, its keys in the fields' order, `null` for nothing. Values as the log keeps them | Read back field for field by Python's `csv` and `json` (the review) |
| Who may read the file | The user alone (0600), as the log's own file | Its statements and values can hold data |
| What is said after | A failure: the notice a failed save gives. A success: nothing, as Export settings | |

### What this run leaves out

- A word on screen after an export that went well: a failure is a notice, as a failed save is; a success says nothing, as Export settings does.
- A cell a spreadsheet would read as a formula is written as it is, as the log keeps it.

## File structure

| File | What changes |
|---|---|
| `src/backend.rs` | `Backend::export_log` |
| `src/util.rs` | `write_atomic_with`: a file written as it goes |
| `src/backend/agents.rs` | `export_log`, `exported`, `LogFormat` |
| `src/model.rs`, `src/app.rs` | `Action::ExportAgentsLog` |
| `src/keymap.rs`, `src/ui/keys.rs` | `E` in the Log view |
| `src/ui/agents.rs`, `src/ui/workspace.rs` | **Export…**; the bar's and the footer's `E export` |
| `src/ui/agents_tests.rs` | The view's tests |
| `docs/_guide/agents.md`, `docs/_guide/omarchy.md` | "The log", with the export's fields; `E` |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | D.5 and G as built: 3b built |

---

### Task 1: The log exported

- `Backend::export_log(file, filter, name)`: the save dialog, `name` suggested, its title saying that the name's ending decides (no file types: a pick renames nothing on Linux); then, on the backend's runtime, `agents::export_log`; its end is `Event::Saved`, a failure a notice as a failed save's is. A test's backend keeps what it was asked (`exports`).
- `agents::export_log`: every call of the log that passes the filter, the newest first as the view lists them, read a thousand at a time and written as they are read (`util::write_atomic_with`, which `write_atomic` is now made of), private to the user: no read keeps the server's writes waiting, and no log is held whole. No log yet is a log of no calls, and nothing is made where it would be.
- `exported(calls, LogFormat)` and `csv_head`: every column the log keeps and `at` as RFC 3339 in UTC (`null` where no clock can say it). CSV (RFC 4180) after a line of the names; JSON Lines an object a line, its keys in the fields' order. `LogFormat::of`: `.jsonl` or `.ndjson`, in any case, is JSON Lines; anything else CSV.

**Files:**
- Modify: `src/backend.rs`, `src/backend/agents.rs`, `src/util.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index c21ec46..eea71e1 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -1695,6 +1695,147 @@ mod tests {
         assert!(!path.exists());
     }
 
+    /// A call of the log, as an export reads it.
+    fn logged(id: i64, agent: &str, result: &str, sql: Option<&str>) -> tabletist_mcp::Logged {
+        tabletist_mcp::Logged {
+            id,
+            entry: tabletist_mcp::Entry {
+                at_ms: 1_791_880_982_000 + id,
+                session: "s1".into(),
+                agent: agent.into(),
+                transport: "stdio".into(),
+                connection_id: Some("shop".into()),
+                tool: "run_query".into(),
+                sql: sql.map(str::to_owned),
+                params: Some("[7,\"a\"]".into()),
+                result: result.into(),
+                rows: Some(3),
+                took_ms: Some(4),
+                ..tabletist_mcp::Entry::default()
+            },
+        }
+    }
+
+    #[test]
+    fn an_export_is_csv_or_json_lines_by_the_name_chosen() {
+        use std::path::Path;
+        assert_eq!(LogFormat::of(Path::new("log.csv")), LogFormat::Csv);
+        assert_eq!(LogFormat::of(Path::new("log.JSONL")), LogFormat::JsonLines);
+        assert_eq!(LogFormat::of(Path::new("log.ndjson")), LogFormat::JsonLines);
+        assert_eq!(LogFormat::of(Path::new("log")), LogFormat::Csv);
+        // CSV: the names first; a field with a comma, a quote or a line
+        // break quoted, its quotes doubled; nothing for nothing.
+        let calls = [logged(
+            1,
+            "Claude, Code",
+            "ok",
+            Some("SELECT 'a'\n FROM \"t\""),
+        )];
+        assert_eq!(
+            csv_head(),
+            "at,at_ms,session,agent,project,transport,connection_id,connection,tool,sql,params,\
+             request,result,code,message,ran_as,rows,matched,columns,bytes,masked,took_ms\r\n"
+        );
+        assert_eq!(
+            exported(&calls, LogFormat::Csv),
+            "2026-10-13T08:43:02.001Z,1791880982001,s1,\"Claude, Code\",,stdio,shop,,\
+             run_query,\"SELECT 'a'\n FROM \"\"t\"\"\",\"[7,\"\"a\"\"]\",,ok,,,,3,,,,,4\r\n"
+        );
+        // JSON Lines: an object a line, its keys in the fields' order,
+        // `null` for nothing, numbers as numbers, the values as the log
+        // keeps them.
+        let lines = exported(&calls, LogFormat::JsonLines);
+        let start = r#"{"at":"2026-10-13T08:43:02.001Z","at_ms":1791880982001,"session":"s1","#;
+        assert!(lines.starts_with(start), "{lines}");
+        let line: serde_json::Value = serde_json::from_str(lines.trim_end()).unwrap();
+        assert_eq!(line["agent"], "Claude, Code");
+        assert_eq!(line["rows"], 3);
+        assert_eq!(line["project"], serde_json::Value::Null);
+        assert_eq!(line["params"], "[7,\"a\"]");
+        assert_eq!(lines.lines().count(), 1);
+        // A moment no clock can say has no `at`.
+        let mut far = logged(2, "Cursor", "ok", None);
+        far.entry.at_ms = i64::MAX;
+        let lines = exported(std::slice::from_ref(&far), LogFormat::JsonLines);
+        let line: serde_json::Value = serde_json::from_str(lines.trim_end()).unwrap();
+        assert_eq!(line["at"], serde_json::Value::Null);
+        assert!(exported(&[far], LogFormat::Csv).starts_with(&format!(",{},", i64::MAX)));
+    }
+
+    #[test]
+    fn an_export_holds_every_call_the_filter_passes_the_newest_first_and_only_the_user_reads_it() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        let runtime = tokio::runtime::Runtime::new().unwrap();
+        runtime.block_on(async {
+            let log = Log::open(&path).unwrap().log;
+            for (n, (agent, result)) in [
+                ("Claude Code", "ok"),
+                ("Cursor", "refused"),
+                ("Claude Code", "refused"),
+            ]
+            .into_iter()
+            .enumerate()
+            {
+                let call = logged(n as i64, agent, result, Some("SELECT 1"));
+                log.record(call.entry).await.unwrap();
+            }
+            let filter = tabletist_mcp::LogFilter {
+                connection: Some("shop".into()),
+                result: Some("refused".into()),
+                ..tabletist_mcp::LogFilter::default()
+            };
+            // Both formats, of the same calls.
+            for name in ["out.csv", "out.jsonl"] {
+                let to = dir.path().join(name);
+                export_log(path.clone(), filter.clone(), to.clone())
+                    .await
+                    .unwrap();
+                let text = std::fs::read_to_string(&to).unwrap();
+                let agents: Vec<&str> = text
+                    .lines()
+                    .filter_map(|line| {
+                        ["Cursor", "Claude Code"]
+                            .into_iter()
+                            .find(|agent| line.contains(agent))
+                    })
+                    .collect();
+                assert_eq!(agents, ["Claude Code", "Cursor"], "{name}: {text}");
+                #[cfg(unix)]
+                {
+                    use std::os::unix::fs::PermissionsExt;
+                    let mode = std::fs::metadata(&to).unwrap().permissions().mode() & 0o777;
+                    assert_eq!(mode, 0o600, "{name}");
+                }
+            }
+            // Past a page: every call, each once, the newest first.
+            for n in 0..2_500 {
+                let call = logged(10 + n, "Claude Code", "error", Some("SELECT 2"));
+                log.record(call.entry).await.unwrap();
+            }
+            let errors = tabletist_mcp::LogFilter {
+                result: Some("error".into()),
+                ..tabletist_mcp::LogFilter::default()
+            };
+            let to = dir.path().join("many.csv");
+            export_log(path.clone(), errors, to.clone()).await.unwrap();
+            let text = std::fs::read_to_string(&to).unwrap();
+            let at: Vec<i64> = text
+                .lines()
+                .skip(1)
+                .map(|line| line.split(',').nth(1).unwrap().parse().unwrap())
+                .collect();
+            assert_eq!(at.len(), 2_500);
+            assert!(at.windows(2).all(|pair| pair[0] > pair[1]));
+            // No log yet: a file of no calls.
+            let to = dir.path().join("none.csv");
+            let none = dir.path().join("none.sqlite3");
+            export_log(none.clone(), filter, to.clone()).await.unwrap();
+            assert_eq!(std::fs::read_to_string(&to).unwrap().lines().count(), 1);
+            assert!(!none.exists());
+        });
+    }
+
     #[test]
     fn the_log_is_read_with_the_server_off_and_reading_it_makes_nothing() {
         let dir = tempfile::tempdir().unwrap();
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- an_export`

Expected: it does not build:

```text
error[E0433]: cannot find type `LogFormat` in this scope
error[E0425]: cannot find function `csv_head` in this scope
error[E0425]: cannot find function `exported` in this scope
error[E0425]: cannot find function `export_log` in this scope
error: could not compile `tabletist` (lib test) due to 20 previous errors
```

- [ ] **Step 3: The export**

````diff
diff --git a/src/backend.rs b/src/backend.rs
index b6b0c75..2d481de 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -417,6 +417,10 @@ pub struct Backend {
     /// suggests, and the size of what is saved.
     #[cfg(test)]
     pub saves: Vec<(String, String, usize)>,
+    /// What a test asked to export of the agents' log: its filter, and
+    /// the name the dialog suggests.
+    #[cfg(test)]
+    pub exports: Vec<(tabletist_mcp::LogFilter, String)>,
     #[cfg(test)]
     watched: Watched,
 }
@@ -488,6 +492,8 @@ impl Backend {
             #[cfg(test)]
             saves: Vec::new(),
             #[cfg(test)]
+            exports: Vec::new(),
+            #[cfg(test)]
             watched,
         }
     }
@@ -509,6 +515,8 @@ impl Backend {
             #[cfg(test)]
             saves: Vec::new(),
             #[cfg(test)]
+            exports: Vec::new(),
+            #[cfg(test)]
             watched: Watched::default(),
         }
     }
@@ -618,6 +626,42 @@ impl Backend {
         });
     }
 
+    /// Asks where to export the agents' log at `file`, `name` suggested,
+    /// and writes there the calls that pass `filter`, every one of them
+    /// and the newest first: as JSON Lines where the name chosen ends in
+    /// `.jsonl`, else as CSV. Read and written off the UI thread, the file
+    /// private to the user, as the log is. A dialog closed without a
+    /// choice exports nothing; one that fails is told as a failed save is.
+    pub fn export_log(&mut self, file: PathBuf, filter: tabletist_mcp::LogFilter, name: String) {
+        #[cfg(test)]
+        self.exports.push((filter.clone(), name.clone()));
+        let Some(runtime) = &self.runtime else {
+            return;
+        };
+        // No file types to pick: a pick renames nothing on Linux, and rfd
+        // does not say which was picked. The name's ending decides, as the
+        // title says.
+        let dialog = rfd::AsyncFileDialog::new()
+            .set_title("Export the agents' log (.csv, or .jsonl for JSON Lines)")
+            .set_file_name(name)
+            .save_file();
+        let outbox = self.outbox.clone();
+        runtime.spawn(async move {
+            let Some(chosen) = dialog.await else {
+                return;
+            };
+            let path = chosen.path().to_path_buf();
+            let result = agents::export_log(file, filter, path.clone()).await;
+            if let Err(error) = &result {
+                log::error!(
+                    "could not export the agents' log to {}: {error}",
+                    path.display()
+                );
+            }
+            outbox.emit(Event::Saved { path, result });
+        });
+    }
+
     /// Reads the Host aliases in ~/.ssh/config off the UI thread.
     pub fn list_ssh_hosts(&mut self, request: RequestId) {
         let Some(runtime) = &self.runtime else {
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index eea71e1..918003b 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -494,6 +494,181 @@ async fn read_log(
     }
 }
 
+/// How an export of the log is written.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum LogFormat {
+    Csv,
+    JsonLines,
+}
+
+impl LogFormat {
+    /// By the ending of the name chosen: JSON Lines for `.jsonl` (or
+    /// `.ndjson`), in any case, else CSV.
+    pub fn of(path: &std::path::Path) -> Self {
+        let ending = path
+            .extension()
+            .and_then(|ending| ending.to_str())
+            .map(str::to_ascii_lowercase);
+        match ending.as_deref() {
+            Some("jsonl" | "ndjson") => Self::JsonLines,
+            _ => Self::Csv,
+        }
+    }
+}
+
+/// How many calls an export reads of the log at a time: each read is
+/// short, so the server's writes of the calls that end meanwhile are never
+/// kept waiting for long.
+const EXPORT_PAGE: u32 = 1000;
+
+/// The calls of the log at `file` that pass `filter`, every one of them,
+/// written to `to` as its name's ending says, the newest first as the view
+/// lists them. Read a page at a time and written as they are read: a long
+/// log is never held whole. No log yet is a log of no calls.
+pub(super) async fn export_log(
+    file: PathBuf,
+    filter: tabletist_mcp::LogFilter,
+    to: PathBuf,
+) -> Result<(), String> {
+    let opened = tokio::task::spawn_blocking(move || Log::open_to_read(&file))
+        .await
+        .map_err(|_| "opening the log failed".to_owned())?
+        .map_err(|error| error.to_string())?;
+    let format = LogFormat::of(&to);
+    let runtime = tokio::runtime::Handle::current();
+    tokio::task::spawn_blocking(move || {
+        crate::util::write_atomic_with(&to, |out| {
+            use std::io::Write;
+            if format == LogFormat::Csv {
+                out.write_all(csv_head().as_bytes())?;
+            }
+            let Some(log) = opened else {
+                return Ok(());
+            };
+            let mut before = None;
+            loop {
+                let page = runtime
+                    .block_on(log.page(filter.clone(), before, EXPORT_PAGE))
+                    .map_err(std::io::Error::other)?;
+                out.write_all(exported(&page.calls, format).as_bytes())?;
+                match (page.more, page.calls.last()) {
+                    (true, Some(last)) => before = Some(last.id),
+                    _ => return Ok(()),
+                }
+            }
+        })
+    })
+    .await
+    .map_err(|_| "writing the file failed".to_owned())?
+    .map_err(|error| error.to_string())
+}
+
+/// A field of an exported call: text, a number, or nothing.
+enum Field<'a> {
+    Text(Option<&'a str>),
+    Number(Option<i64>),
+}
+
+/// The fields an export holds of each call, by name, in order: every one
+/// the log keeps, its moment also as RFC 3339 in UTC where it can be one.
+fn fields<'a>(
+    entry: &'a tabletist_mcp::Entry,
+    at: Option<&'a str>,
+) -> [(&'static str, Field<'a>); 22] {
+    use Field::{Number, Text};
+    [
+        ("at", Text(at)),
+        ("at_ms", Number(Some(entry.at_ms))),
+        ("session", Text(Some(&entry.session))),
+        ("agent", Text(Some(&entry.agent))),
+        ("project", Text(entry.project.as_deref())),
+        ("transport", Text(Some(&entry.transport))),
+        ("connection_id", Text(entry.connection_id.as_deref())),
+        ("connection", Text(entry.connection.as_deref())),
+        ("tool", Text(Some(&entry.tool))),
+        ("sql", Text(entry.sql.as_deref())),
+        ("params", Text(entry.params.as_deref())),
+        ("request", Text(entry.request.as_deref())),
+        ("result", Text(Some(&entry.result))),
+        ("code", Text(entry.code.as_deref())),
+        ("message", Text(entry.message.as_deref())),
+        ("ran_as", Text(entry.ran_as.as_deref())),
+        ("rows", Number(entry.rows)),
+        ("matched", Number(entry.matched)),
+        ("columns", Number(entry.columns)),
+        ("bytes", Number(entry.bytes)),
+        ("masked", Text(entry.masked.as_deref())),
+        ("took_ms", Number(entry.took_ms)),
+    ]
+}
+
+/// The line of the fields' names a CSV export starts with.
+fn csv_head() -> String {
+    let names: Vec<&str> = fields(&tabletist_mcp::Entry::default(), None)
+        .iter()
+        .map(|(name, _)| *name)
+        .collect();
+    format!("{}\r\n", names.join(","))
+}
+
+/// `calls` as lines of an export of `format`, each line ended: CSV (RFC
+/// 4180: a field with a comma, a quote or a line break quoted, its quotes
+/// doubled, lines ended with CR LF, nothing for a field the call has not;
+/// [`csv_head`] goes first), or JSON Lines, an object a line, its keys in
+/// the fields' order, `null` for nothing. A value is written as the log
+/// keeps it: `params` and `masked` are the log's JSON, as text.
+pub(crate) fn exported(calls: &[tabletist_mcp::Logged], format: LogFormat) -> String {
+    let moment = |at_ms: i64| {
+        jiff::Timestamp::from_millisecond(at_ms)
+            .ok()
+            .map(|at| at.to_string())
+    };
+    let mut out = String::new();
+    for logged in calls {
+        let at = moment(logged.entry.at_ms);
+        let fields = fields(&logged.entry, at.as_deref());
+        match format {
+            LogFormat::Csv => {
+                let cell = |text: &str| match text.contains([',', '"', '\n', '\r']) {
+                    true => format!("\"{}\"", text.replace('"', "\"\"")),
+                    false => text.to_owned(),
+                };
+                let cells: Vec<String> = fields
+                    .into_iter()
+                    .map(|(_, field)| match field {
+                        Field::Text(text) => text.map(cell).unwrap_or_default(),
+                        Field::Number(number) => {
+                            number.map(|number| number.to_string()).unwrap_or_default()
+                        }
+                    })
+                    .collect();
+                out.push_str(&cells.join(","));
+                out.push_str("\r\n");
+            }
+            LogFormat::JsonLines => {
+                let pairs: Vec<String> = fields
+                    .into_iter()
+                    .map(|(name, field)| {
+                        let value = match field {
+                            Field::Text(text) => text.map_or(serde_json::Value::Null, |text| {
+                                serde_json::Value::String(text.to_owned())
+                            }),
+                            Field::Number(number) => {
+                                number.map_or(serde_json::Value::Null, serde_json::Value::from)
+                            }
+                        };
+                        format!("{}:{value}", serde_json::Value::String(name.to_owned()))
+                    })
+                    .collect();
+                out.push('{');
+                out.push_str(&pairs.join(","));
+                out.push_str("}\n");
+            }
+        }
+    }
+    out
+}
+
 /// Tells the app of each call as it ends, until the server stops. One
 /// that falls behind misses the oldest: the log has them all.
 async fn called(mut calls: tokio::sync::broadcast::Receiver<tabletist_mcp::Call>, outbox: Outbox) {
diff --git a/src/util.rs b/src/util.rs
index 8b840bf..169339e 100644
--- a/src/util.rs
+++ b/src/util.rs
@@ -143,7 +143,16 @@ pub fn resolve_link(path: &Path) -> std::io::Result<PathBuf> {
 /// file is private to the user (0600) wherever it is, and its directory is
 /// flushed after the rename so the new name survives a crash too.
 pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
-    put_atomic(path, bytes, true).map(|_| ())
+    put_atomic(path, |file| file.write_all(bytes), true).map(|_| ())
+}
+
+/// [`write_atomic`], with what is written put by `write` into the
+/// temporary file as it goes: a file too long to be held whole first.
+pub fn write_atomic_with(
+    path: &Path,
+    write: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
+) -> std::io::Result<()> {
+    put_atomic(path, write, true).map(|_| ())
 }
 
 /// Writes `bytes` to `path` as [`write_atomic`] does, but only where no
@@ -151,12 +160,16 @@ pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
 /// left as it is. Whether it wrote. Looking for the file first and writing
 /// after would replace a file made between the two.
 pub fn create_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<bool> {
-    put_atomic(path, bytes, false)
+    put_atomic(path, |file| file.write_all(bytes), false)
 }
 
 /// [`write_atomic`], or with `replace` false [`create_atomic`]. Whether it
 /// wrote.
-fn put_atomic(path: &Path, bytes: &[u8], replace: bool) -> std::io::Result<bool> {
+fn put_atomic(
+    path: &Path,
+    write: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
+    replace: bool,
+) -> std::io::Result<bool> {
     let destination = resolve_link(path)?;
     let dir = directory_of(&destination);
     if destination == path {
@@ -166,7 +179,7 @@ fn put_atomic(path: &Path, bytes: &[u8], replace: bool) -> std::io::Result<bool>
         .prefix(".tabletist-")
         .suffix(".tmp")
         .tempfile_in(dir)?;
-    file.write_all(bytes)?;
+    write(file.as_file_mut())?;
     file.as_file().sync_all()?;
     if replace {
         file.persist(&destination).map_err(|error| error.error)?;
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- an_export the_log_is_read_with_the_server_off`

Expected:

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 2747 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/backend.rs src/backend/agents.rs src/util.rs
git commit -m "Export the agents' log by a filter, as CSV or JSON Lines"
```

---

### Task 2: Export… in the Log view

- `Action::ExportAgentsLog(tab)`: the view's filter, and `tabletist-agents-log.csv` suggested.
- macOS: **Export…** at the right of the bar, off with no call to export; the filters keep to its left, the four narrower alike where the window is, and the count shows between them where it has the room.
- Omarchy: `E` (`Command::ExportAgentsLog`, `When::AgentsLog`), with a call or more; named in the bar and the footer then.

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/keymap.rs`, `src/ui/keys.rs`, `src/ui/agents.rs`, `src/ui/workspace.rs`, `src/ui/agents_tests.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index 2a2e50a..805f9a0 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -1185,7 +1185,7 @@ fn omarchys_enter_shows_the_calls_pane_esc_closes_it_and_o_opens_its_statement()
     harness.settle();
     // The bar names the keys of the call selected, where it has the room;
     // a narrow window leaves them to the footer.
-    let hint = "/ filter · enter details · o open in sql tab";
+    let hint = "/ filter · E export · enter details · o open in sql tab";
     let narrow = harness.size;
     harness.size = egui::vec2(1600.0, narrow.y);
     harness.settle();
@@ -1240,6 +1240,88 @@ fn omarchys_enter_shows_the_calls_pane_esc_closes_it_and_o_opens_its_statement()
     );
 }
 
+#[test]
+fn export_sends_the_logs_filter_to_the_save_dialog_in_both_looks() {
+    for look in looks() {
+        let (mut harness, tab) = on_the_log(look);
+        let conn = harness.app.workspace(tab).unwrap().conn_id.0.clone();
+        harness.app.apply(Action::FilterAgentsLog {
+            tab,
+            filter: crate::model::AgentsLogFilter::Result(Some("ok".into())),
+        });
+        log_answered(&mut harness);
+        if look.terminal {
+            assert!(harness.painted_rect("E export").is_some(), "{}", look.name);
+            super::tests::type_key(&mut harness, egui::Key::E, "E");
+        } else {
+            harness.click("Export…");
+        }
+        harness.settle();
+        let wanted = tabletist_mcp::LogFilter {
+            connection: Some(conn),
+            result: Some("ok".into()),
+            ..tabletist_mcp::LogFilter::default()
+        };
+        assert_eq!(
+            harness.app.backend.exports.last(),
+            Some(&(wanted, "tabletist-agents-log.csv".to_owned())),
+            "{}",
+            look.name
+        );
+        // With no call to export, nothing is asked.
+        harness.app.apply(Action::FilterAgentsLog {
+            tab,
+            filter: crate::model::AgentsLogFilter::Search("nothing".into()),
+        });
+        log_answered_empty(&mut harness);
+        let asked = harness.app.backend.exports.len();
+        if look.terminal {
+            assert!(harness.painted_rect("E export").is_none(), "{}", look.name);
+            super::tests::type_key(&mut harness, egui::Key::E, "E");
+        } else {
+            harness.click("Export…");
+        }
+        harness.settle();
+        assert_eq!(harness.app.backend.exports.len(), asked, "{}", look.name);
+    }
+}
+
+#[test]
+fn the_logs_bar_keeps_its_filters_count_and_export_apart_as_the_window_narrows() {
+    let (mut harness, _) = on_the_log(Look::macos());
+    let count = "1 call · last 30 days · on this computer";
+    for width in [1400.0, 1100.0, 900.0, 720.0] {
+        harness.size = egui::vec2(width, 700.0);
+        harness.settle();
+        let rect = |label: &str| harness.painted_rect(label);
+        let search = rect("Search requests…").expect("the search");
+        let export = rect("Export…").expect("Export…");
+        let results = rect("All results").expect("the result filter");
+        assert!(
+            results.right() < export.left(),
+            "{width}: {results:?} {export:?}"
+        );
+        assert!(
+            search.right() < export.left(),
+            "{width}: {search:?} {export:?}"
+        );
+        // The count between them, or not at all where it has no room.
+        if let Some(shown) = rect(count) {
+            assert!(
+                search.right() < shown.left(),
+                "{width}: {search:?} {shown:?}"
+            );
+            assert!(
+                shown.right() < export.left(),
+                "{width}: {shown:?} {export:?}"
+            );
+        }
+        if width >= 1400.0 {
+            assert!(rect(count).is_some(), "{width}");
+        }
+    }
+}
+
 #[test]
 fn omarchy_shows_the_log_with_l_and_the_agents_log_line() {
     let (mut harness, tab) = connected(Look::omarchy());
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib -- export_sends omarchys_enter the_logs_bar_keeps`

Expected: the two new tests fail, as nothing draws Export… yet, and so does the earlier one, whose bar now names `E export`:

```text
test ui::agents_tests::the_logs_bar_keeps_its_filters_count_and_export_apart_as_the_window_narrows ... FAILED
test ui::agents_tests::export_sends_the_logs_filter_to_the_save_dialog_in_both_looks ... FAILED
test ui::agents_tests::omarchys_enter_shows_the_calls_pane_esc_closes_it_and_o_opens_its_statement ... FAILED
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 2749 filtered out
```

- [ ] **Step 3: The button and the key**

````diff
diff --git a/src/app.rs b/src/app.rs
index bb27781..32f1a67 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1837,6 +1837,20 @@ impl App {
                 }
             }
             Action::OpenAgentsLogCall { tab, id } => self.open_agents_log_call(tab, id),
+            Action::ExportAgentsLog(tab) => {
+                let filter = self.workspace(tab).and_then(|workspace| {
+                    let conn = workspace.conn_id.0.clone();
+                    workspace.tabs.iter().find_map(|open| match open {
+                        Tab::Agents(agents) => Some(agents.log.filter(&conn)),
+                        Tab::Object(_) | Tab::Sql(_) => None,
+                    })
+                });
+                if let Some(filter) = filter {
+                    let file = self.dirs.agents_log_file();
+                    let name = "tabletist-agents-log.csv".to_owned();
+                    self.backend.export_log(file, filter, name);
+                }
+            }
             Action::ShowAgentsLogFilters { tab, open } => {
                 if let Some(agents) = self
                     .workspace_mut(tab)
diff --git a/src/keymap.rs b/src/keymap.rs
index e8434d1..bc99179 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -349,6 +349,7 @@ commands! {
     CloseAgentsLogPane,
     ShowAgentsLogDetails,
     OpenAgentsLogCall,
+    ExportAgentsLog,
     AllowAgents,
     ShowAgentSettings,
     ChangeAgentAccess,
@@ -641,6 +642,11 @@ impl Command {
                 "",
                 Window,
             )),
+            C::ExportAgentsLog => agents(info(
+                "Export the log's calls that pass its filters",
+                "",
+                Window,
+            )),
             C::ToggleSidebar => info("Toggle sidebar", "Toggle sidebar", Navigation),
             C::PaneLeft => info("Pane to the left", PANES, Navigation),
             C::PaneDown => info("Pane below", PANES, Navigation).reserved(),
@@ -1055,6 +1061,7 @@ pub const BINDINGS: &[Binding] = {
         bind(C::CloseAgentsLogPane, AGENTS, NONE, keys(&[k("esc")])),
         bind(C::ShowAgentsLogDetails, AGENTS, NONE, keys(&[k("enter")])).when(When::AgentsLog),
         bind(C::OpenAgentsLogCall, AGENTS, NONE, keys(&[k("o")])).when(When::AgentsLog),
+        bind(C::ExportAgentsLog, AGENTS, NONE, keys(&[k("E")])).when(When::AgentsLog),
         bind(C::AllowAgents, AGENTS, NONE, keys(&[k("e")])),
         bind(C::ShowAgentSettings, AGENTS, NONE, keys(&[k("s")])),
         bind(C::ChangeAgentAccess, CONNECTIONS, NONE, keys(&[k("a")])),
diff --git a/src/model.rs b/src/model.rs
index 0eeed61..d1263cb 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -664,6 +664,9 @@ pub enum Action {
         tab: ConnTabId,
         open: bool,
     },
+    /// The Log view: its calls that pass its filters, every one, to a file
+    /// the person chooses, as CSV or JSON Lines.
+    ExportAgentsLog(ConnTabId),
     /// The Log view: the statement of a call, by its place in the log, in
     /// a new SQL editor of its connection's window, read-only and not run.
     OpenAgentsLogCall {
diff --git a/src/ui/agents.rs b/src/ui/agents.rs
index f26829c..1250141 100644
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -111,6 +111,9 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let open_key = app
         .keymap
         .key(layout, crate::keymap::Command::OpenAgentsLogCall);
+    let export_key = app
+        .keymap
+        .key(layout, crate::keymap::Command::ExportAgentsLog);
     // The selected call's connection, as it is saved now: whether its
     // statement opens, and the dialect it is read in.
     let call = log
@@ -257,6 +260,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     close: &close_key,
                     details: &details_key,
                     open: &open_key,
+                    export: &export_key,
                 };
                 let shown = log_view(
                     ui,
@@ -278,6 +282,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     Some(LogAsk::Open(id)) => {
                         actions.push(Action::OpenAgentsLogCall { tab, id });
                     }
+                    Some(LogAsk::Export) => actions.push(Action::ExportAgentsLog(tab)),
                     Some(LogAsk::More) => actions.push(Action::MoreAgentsLog(tab)),
                     Some(LogAsk::Select(id)) => {
                         actions.push(Action::SelectAgentsLogCall { tab, id });
@@ -1054,6 +1059,8 @@ enum LogAsk {
     CloseDetails,
     /// The call's statement opened in a SQL editor, by its place.
     Open(i64),
+    /// The calls that pass the filters, to a file.
+    Export,
 }
 
 /// The Log view's bar at its top (the terminal's, and macOS's, which holds
@@ -1131,6 +1138,16 @@ fn log_view(
                 &format!(" {}", look.label(&gettext(locale, "filter"))),
                 palette.secondary,
             );
+        if log.total > 0 {
+            filter = filter
+                .add(body, " · ", palette.secondary)
+                .add(body, keys.export.as_str(), palette.accent)
+                .add(
+                    body,
+                    &format!(" {}", look.label(&gettext(locale, "export"))),
+                    palette.secondary,
+                );
+        }
         if log.selected.is_some() {
             filter = filter
                 .add(body, " · ", palette.secondary)
@@ -1158,9 +1175,22 @@ fn log_view(
         }
         widgets::announce(ui, bar, &look.label(&format!("{scope} · {count}")));
     } else {
+        // At its right, Export…, off with nothing to export: the filters
+        // keep to the left of it, the search giving up its width first.
+        let label = gettext(locale, "Export…");
+        let reason = gettext(locale, "No calls to export.");
+        let mut export = states::button(&label, look);
+        if log.total == 0 {
+            export = export.disabled(&reason);
+        }
+        let width = export.width(ui, look);
+        let at = Rect::from_center_size(
+            pos2(bar.right() - 16.0 - width / 2.0, bar.center().y),
+            vec2(width, 30.0),
+        );
         let row = Rect::from_min_max(
             pos2(bar.left() + 16.0, bar.center().y - 14.0),
-            pos2(bar.right() - 16.0, bar.center().y + 14.0),
+            pos2(at.left() - 12.0, bar.center().y + 14.0),
         );
         let mut child = ui.new_child(
             egui::UiBuilder::new()
@@ -1173,12 +1203,16 @@ fn log_view(
         {
             ask = Some(LogAsk::Filter(filter));
         }
+        let filters = child.min_rect().right();
+        if export.show_at(ui, at, look, palette).clicked() && log.total > 0 {
+            ask = Some(LogAsk::Export);
+        }
+        // The count between them, where it has the room.
         let laid = Text::one(look, small, &count, palette.secondary).layout(ui.ctx());
-        laid.paint_left(
-            ui.painter(),
-            bar.right() - 16.0 - laid.width(),
-            bar.center().y,
-        );
+        let left = at.left() - 12.0 - laid.width();
+        if left >= filters + 12.0 {
+            laid.paint_left(ui.painter(), left, bar.center().y);
+        }
         widgets::announce(ui, bar, &format!("{scope} · {count}"));
     }
     // The terminal look's filters, under its bar: their rows while open,
@@ -1445,6 +1479,8 @@ pub(crate) struct LogKeys<'a> {
     pub details: &'a crate::keymap::Written,
     /// The selected call's statement, in a SQL editor.
     pub open: &'a crate::keymap::Written,
+    /// The calls that pass the filters, to a file.
+    pub export: &'a crate::keymap::Written,
 }
 
 /// What the Log view is told of the selected call by the app: whether its
@@ -1679,6 +1715,13 @@ fn log_filters(
     use crate::model::AgentsLogFilter;
     use crate::ui::menu;
     let mut set = None;
+    // The board's widths, each one narrower alike where the bar has not
+    // the room: a dropdown clips what it reads.
+    let board = [170.0, 130.0, 120.0, 220.0];
+    let gaps = 3.0 * ui.spacing().item_spacing.x;
+    let share = ((ui.available_width() - gaps) / board.iter().sum::<f32>()).min(1.0);
+    let [scope_width, agent_width, result_width, search_width] =
+        board.map(|width: f32| (width * share).max(40.0).floor());
     let named = |response: &egui::Response, name: &str, value: &str| {
         response.widget_info(|| {
             let mut info = egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, name);
@@ -1689,7 +1732,7 @@ fn log_filters(
     // Whose calls.
     let this = gettext(locale, "This connection");
     let all = gettext(locale, "All connections");
-    let scopes = menu::Dropdown::new("agents-log-scope", scope, 170.0)
+    let scopes = menu::Dropdown::new("agents-log-scope", scope, scope_width)
         .height(28.0)
         .show(ui, look, palette, |ui| {
             for (every, word) in [(false, &this), (true, &all)] {
@@ -1713,7 +1756,7 @@ fn log_filters(
     {
         agents.push(chosen.clone());
     }
-    let menu = menu::Dropdown::new("agents-log-agent", &agent, 130.0)
+    let menu = menu::Dropdown::new("agents-log-agent", &agent, agent_width)
         .height(28.0)
         .show(ui, look, palette, |ui| {
             let mut choices = vec![(None, every_agent.to_string())];
@@ -1733,7 +1776,7 @@ fn log_filters(
         || every_result.to_string(),
         |result| result_word(result, locale),
     );
-    let menu = menu::Dropdown::new("agents-log-result", &result, 120.0)
+    let menu = menu::Dropdown::new("agents-log-result", &result, result_width)
         .height(28.0)
         .show(ui, look, palette, |ui| {
             let mut choices = vec![(None, every_result.to_string())];
@@ -1756,7 +1799,7 @@ fn log_filters(
     let placeholder = gettext(locale, "Search requests…");
     let role = widgets::body(look);
     let field = ui.add_sized(
-        vec2(220.0, 28.0),
+        vec2(search_width, 28.0),
         widgets::field(ui, &mut text, look, role, 28.0, 8)
             .id(Id::new("agents-log-search"))
             .hint_text(placeholder.as_ref()),
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index f5c044e..f67c8bc 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -2338,6 +2338,13 @@ fn letters(
             {
                 actions.push(Action::ToggleAgentsLogScope(tab));
             }
+            (Some(Command::ExportAgentsLog), _)
+                if agents_view(app, tab) == Some(crate::model::AgentsView::Log) =>
+            {
+                if log.is_some_and(|log| log.total > 0) {
+                    actions.push(Action::ExportAgentsLog(tab));
+                }
+            }
             (Some(Command::OpenAgentsLogCall), _)
                 if agents_view(app, tab) == Some(crate::model::AgentsView::Log) =>
             {
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index d634aa8..c501735 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2060,9 +2060,10 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         key(Command::ToggleAgentsLogScope),
         key(Command::FilterAgentsLog),
     );
-    let (agents_details_key, agents_open_key) = (
+    let (agents_details_key, agents_open_key, agents_export_key) = (
         key(Command::ShowAgentsLogDetails),
         key(Command::OpenAgentsLogCall),
+        key(Command::ExportAgentsLog),
     );
     // Its Log view, while it shows: whether every connection's, whether a
     // call is selected, and whether that call's statement opens (it has
@@ -2082,7 +2083,7 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                                 .is_some()
                         })
                 });
-                Some((log.all, call.is_some(), opens))
+                Some((log.all, call.is_some(), opens, log.total > 0))
             }
             _ => None,
         })
@@ -2328,10 +2329,11 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 gettext(locale, "filter"),
                 gettext(locale, "details"),
                 gettext(locale, "open"),
+                gettext(locale, "export"),
             ];
             match agents_log {
                 // The Log view's keys in the place of the activity's.
-                Some((all, selected, opens)) => {
+                Some((all, selected, opens, any)) => {
                     let scope = if all { &*log_words[4] } else { &*log_words[3] };
                     agents_hints = vec![
                         (&agents_log_key, &*log_words[1], true),
@@ -2345,6 +2347,9 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     if opens {
                         agents_hints.push((&agents_open_key, &*log_words[7], true));
                     }
+                    if any {
+                        agents_hints.push((&agents_export_key, &*log_words[8], true));
+                    }
                 }
                 None => agents_hints.push((&agents_log_key, &*log_words[0], true)),
             }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- agents_tests keymap`, then `cargo test --locked --test keys`

Expected:

```text
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 2679 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/model.rs src/app.rs src/keymap.rs src/ui/keys.rs src/ui/agents.rs src/ui/workspace.rs src/ui/agents_tests.rs
git commit -m "Add Export… to the agents' log, and E on Omarchy"
```

---

### Task 3: The website's guide to the log

- `docs/_guide/agents.md`: "The log", a section of its own: what it lists, its filters, a call's details, and the export, with its fields.
- `docs/_guide/omarchy.md`: `E`.

**Files:**
- Modify: `docs/_guide/agents.md`, `docs/_guide/omarchy.md`

- [ ] **Step 1: The pages**

````diff
diff --git a/docs/_guide/agents.md b/docs/_guide/agents.md
index 829aff1..f5ce60c 100644
--- a/docs/_guide/agents.md
+++ b/docs/_guide/agents.md
@@ -212,21 +212,40 @@ The tab shows:
 **Stop agents here** ends every session on the connection and sets its
 access to **Off**, so that an agent that connects again does not see it.
 
-The tab's **Log** view lists every call agents made, on this connection or
-on all of them: the tool, the statement, how many rows it returned and how
-long it took. Rows themselves are never logged. **Settings › Agents ›
-Keep the activity log** says for how many days a call is kept.
+The tab's **Log** view keeps every call: see [below](#the-log).
 
-Its bar narrows the list: **This connection** or **All connections**, one
+When an agent connects, Tabletist tells you: a notification on Linux,
+whose **Review** shows the Agents tab, the Dock icon bouncing on macOS,
+and the taskbar button flashing on Windows.
+**Notify when an agent connects** turns it off.
+
+## The log
+
+The Agents tab's **Log** view lists every call agents made, on this
+connection or on all of them, the newest first: the time, the agent, the
+tool, the statement or the tables asked for, how many rows it returned,
+how long it took, and how it ended. Rows themselves are never logged. The
+log is read from its file, so it shows past calls with the server off and
+with agents' access off. **Settings › Agents › Keep the activity log**
+says for how many days a call is kept. Open it with **Show log** or the
+switch at the tab's top, or `l` in the Omarchy look, and `:agents log`
+from anywhere in the window.
+
+### Filter the calls
+
+The bar narrows the list: **This connection** or **All connections**, one
 agent of those that called or **All agents**, one way a call ended
 (**ok**, **refused**, **failed**, **timed out**, **cancelled**,
 **stopped**) or **All results**, and **Search requests…**, which finds
 the text anywhere in a statement or in the tables `describe_schema` was
 asked for, English letters in either case. The count then says how many
-calls match. In the Omarchy look, `/` opens the same filters over the
-list, with the keyboard in the search: Tab moves to the agents and the
-results, Enter or Space picks one, and Esc closes them and keeps them. A
-line under the bar says what they are set to.
+calls match. In the Omarchy look, `L` switches between this connection
+and all of them, and `/` opens the other filters over the list, with the
+keyboard in the search: Tab moves to the agents and the results, Enter
+or Space picks one, and Esc closes them and keeps them. A line under the
+bar says what they are set to.
+
+### A call's details
 
 A call you select shows its details beside the list: the statement as it
 ran, its values, what it returned, the columns that were masked, how it
@@ -237,14 +256,30 @@ database: this one for a call on this connection, another that is open,
 or one that opens and connects for it. Its values come first, as the log
 keeps them, in a comment line each (`-- $1 = 9100000000000000004` on
 PostgreSQL, `?1` on SQLite, `? #1` on MySQL), for you to put in its
-placeholders. A call on a connection
-deleted since cannot be opened. In the Omarchy look the details show
-with Enter, `o` opens the statement, and Esc closes them.
+placeholders. A call on a connection deleted since cannot be opened. In
+the Omarchy look `j` and `k` move among the calls, Enter shows the
+details, `o` opens the statement, and Esc closes them.
 
-When an agent connects, Tabletist tells you: a notification on Linux,
-whose **Review** shows the Agents tab, the Dock icon bouncing on macOS,
-and the taskbar button flashing on Windows.
-**Notify when an agent connects** turns it off.
+### Export
+
+**Export…**, at the bar's right, or `E` in the Omarchy look, saves every
+call that passes the filters, not only those on the screen, the newest
+first as the list shows them. Name the file `.jsonl` (or `.ndjson`) for
+JSON Lines, one object a line; anything else is CSV, with a line of the
+column names first. Each call has:
+
+| Field | What it is |
+|---|---|
+| `at`, `at_ms` | When the call came, as RFC 3339 in UTC and in milliseconds since 1970 |
+| `session`, `agent`, `project`, `transport` | Who called: the session, the agent as it names itself, its project, and how it connected |
+| `connection_id`, `connection` | The connection, by its id and by its name then |
+| `tool`, `sql`, `params`, `request` | What was asked: the statement and its values (as JSON), or the tables asked for |
+| `result`, `code`, `message` | How it ended, the refusal's code, and its sentence |
+| `ran_as`, `rows`, `matched`, `columns`, `bytes`, `masked`, `took_ms` | How it ran, what it returned, the columns masked (as JSON), and how long it took |
+
+A field a call has not is empty in CSV and `null` in JSON Lines. The file
+is readable by your account alone, as the log is: its statements and
+values can hold data.
 
 ## Changes made outside Tabletist
 
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index 9d5c78e..4355c94 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -124,6 +124,7 @@ key. Nor has copying a whole row.
 | `/` | In the log, open its filters: the search, the agent and the result. Tab moves among them, Enter or Space picks |
 | Enter | In the log, show the selected call's details, and close them |
 | `o` | In the log, open the selected call's statement in a SQL editor, read-only and not run |
+| `E` | In the log, export the calls that pass its filters, as CSV or JSON Lines |
 | Esc | Close the log's filters, and keep them, or the call's details |
 | `j` / `k` | In the log, move among the calls |
 
````

- [ ] **Step 2: Check them**

Run: `docs/bin/check` and `grep -c "$(printf '\342\200\224')" docs/_guide/agents.md docs/_guide/omarchy.md` (a count of em dashes)

Expected: `HTML-Proofer finished successfully.`, and `0` for each page.

- [ ] **Step 3: Commit**

```bash
git add docs/_guide/agents.md docs/_guide/omarchy.md
git commit -m "Give the agents' log a guide of its own on the website"
```

---

### Task 4: The master plan

- D.5 as built for 3b-3; 3b's run 3 and G's row: built, and Phase 2 next.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: The edits**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index 0a8041e..07ca98b 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -2478,6 +2478,41 @@ went otherwise:
   nothing, the list given no room in a narrow window, and the pane left
   open as a filter cleared the selection: each is fixed with a test.
 
+**As built in 3b's run 3** (`2026-10-18-mcp-server-3b-3-export.md`). Agreed on 2026-10-18, where it
+went otherwise:
+
+- **Export…**, at the right of the macOS bar as the board draws it, off
+  with no call to export; on Omarchy, whose board draws no key for it,
+  `E` in the Log view (`When::AgentsLog`), named in its bar and footer.
+- The save dialog (`rfd`) offers no file types, whose pick renames nothing
+  on Linux and is not told: the name's ending decides, as its title says,
+  `.jsonl` (or `.ndjson`) JSON Lines and anything else CSV. The file holds
+  every call that passes the view's filters, not only the pages read, the
+  newest first as the view lists them, read from the log a thousand calls
+  at a time and written as they are read, on the backend's runtime
+  (`Backend::export_log`): no read keeps the server's writes waiting, and
+  no log is held whole. It is private to the user as the log is
+  (`util::write_atomic_with`). A failure is the notice a failed save gives;
+  a success says nothing, as Export settings does.
+- Each call is every column the log keeps, `at` added as RFC 3339 in UTC;
+  `params` and `masked` the log's JSON, as text. CSV is RFC 4180 with a
+  line of the names first; JSON Lines an object a line, its keys in that
+  order, `null` for nothing (and for a moment no clock can say). A value is
+  written as it is: a cell that a spreadsheet would read as a formula is
+  not changed.
+- The macOS bar's filters keep to the left of Export…, the four of them
+  narrower alike where the window is, and the count shows between them
+  where it has the room.
+- The run's review found a long export's one read keeping the server from
+  writing the calls that ended meanwhile, past its two seconds' wait, and
+  holding the whole log in memory; the dialog's file types picking
+  nothing on Linux; the macOS bar's count and Export… over its filters at
+  common widths; JSON Lines' keys in another order than the fields'; and
+  a moment out of a clock's range written as `""`: each is fixed, with a
+  test but for the dialog.
+- The guide: "The log" in `docs/_guide/agents.md`, with its filters, a
+  call's details and the export's fields.
+
 ### D.6 The keymap
 
 Checked against `src/keymap.rs` and the keyboard brief
@@ -2763,7 +2798,7 @@ is off until the person turns it on.
 | 1c. Hidden tables | 6, below | Built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`, `2026-10-16-mcp-server-1c-2b-setting.md`, `2026-10-16-mcp-server-1c-3a-tables-control.md`, `2026-10-16-mcp-server-1c-3b-table-list.md`) |
 | 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |
 | 3a. Masked columns | 5, below | Built (`2026-10-17-mcp-server-3a-1a-masked-rule.md`, `2026-10-17-mcp-server-3a-1b-masked-values.md`, `2026-10-17-mcp-server-3a-2-server.md`, `2026-10-17-mcp-server-3a-3-setting.md`, `2026-10-18-mcp-server-3a-4-columns-control.md`): the rule, its stops, the engines' fences, the values, the server's side, the setting, and the editor's control with the website's guide. 3b is next (order changed on 2026-10-17) |
-| 3b. The Log view, in full | 3, below | Runs 1 and 2 built (`2026-10-18-mcp-server-3b-1-log-filters.md`, `2026-10-18-mcp-server-3b-2-call-pane.md`): the filters and the search, the detail pane and Open in SQL editor. Run 3, Export… and the guide, is next |
+| 3b. The Log view, in full | 3, below | Built (`2026-10-18-mcp-server-3b-1-log-filters.md`, `2026-10-18-mcp-server-3b-2-call-pane.md`, `2026-10-18-mcp-server-3b-3-export.md`): the filters and the search, the detail pane and Open in SQL editor, Export… and the guide. Phase 2 is next |
 | 2. Approved writes | Not cut yet | Last, after 3b (moved on 2026-10-17) |
 
 The move of the connections to `connections.toml` came between 1a and 1b
@@ -2981,7 +3016,8 @@ pull request:
    (`2026-10-18-mcp-server-3b-1-log-filters.md`, built).
 2. The detail pane for the selected call, and **Open in SQL editor**
    (`2026-10-18-mcp-server-3b-2-call-pane.md`, built).
-3. **Export…**, and the website's guide to the log.
+3. **Export…**, and the website's guide to the log
+   (`2026-10-18-mcp-server-3b-3-export.md`, built).
 
 ## Open questions
 
````

- [ ] **Step 2: Check the whole of it**

Run: `cargo fmt --all --check`, clippy as above, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, the count of em dashes in the master plan, and `cargo test --locked --workspace --all-targets` with both servers

Expected: fmt and clippy clean, the documentation builds without a warning, no em dash (`0`), and:

```text
3758 passed, 0 failed, 22 ignored
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan that the log's export, and 3b, are built"
```

## Self-review

- **Against the design.** C.1's export ("the rows that pass the current filters, as CSV or as JSON Lines, through the save dialog the app already has, written on the backend runtime"), F.4's "Export matches the filtered view, in both formats", and the macOS board's **Export…** (`MacAgentsTabLog`). The departures are in "What the plan asks, and what is built", each agreed on 2026-10-18 or found by the review.
- **A review before the plan.** A reviewer exported logs of up to 2.5 million calls while the server wrote to them, read the CSV and JSON Lines back with Python, exported through links, into missing folders and over other files, pressed `E` in every state, and measured the macOS bar at four widths. It found a long export's one read keeping the server from writing past its two seconds' wait, the whole log held in memory, the dialog's file types picking nothing on Linux, the bar's count and Export… over its filters, JSON Lines' keys out of order, and an unsayable moment written as `""`. All fixed, with a test each but the dialog.
- **By eye.** Headless renders of both bars. Not committed.
- **Placeholders.** None: every step has its diff or its command and what it prints.
- **Names.** `Backend::export_log`, `agents::export_log`, `exported`, `csv_head`, `LogFormat`, `EXPORT_PAGE`, `util::write_atomic_with`, `Action::ExportAgentsLog` and `Command::ExportAgentsLog` are the same in every task that uses them.
