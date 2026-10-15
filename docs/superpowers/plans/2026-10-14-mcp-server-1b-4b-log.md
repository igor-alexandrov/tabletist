# MCP server, Phase 1b, run 4b: the log, and a new agent. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Agents tab gets its Log view: the calls agents made, this connection's or every one's, 200 at a time, with what the selected one came to, readable with the server off. The app gets a clock, so the log has dates and the feed and the session line say times as the boards draw them. An agent that connects is told of: a notification on Linux with **Review**, the Dock's bounce on macOS.

**Architecture:** The log's file is opened to read (`Log::open_to_read`: nothing made, nothing set aside) and read a page at a time (`Log::page`). The backend answers `AgentsCommand::ReadLog` with `AgentsEvent::LogRead`, whether the server runs or not. The tab keeps its view (`AgentsView`) and its page (`AgentsLog`); a page on screen is read again when a call on its connection ends. `src/clock.rs` formats times in the system's time zone with jiff. A session whose agent has named itself is told of once (`AgentsState::told`): the backend shows the notification over D-Bus with zbus and says when Review is clicked (`AgentsEvent::Reviewed`); macOS asks for the Dock's bounce through `App::window_asks`.

**Tech Stack:** Rust 2024, egui, jiff and zbus. Both are already built in the tree (jiff through the logger, zbus through the keyring store), with the features used here: nothing new is compiled.

---

## Where this sits

Phase 1b of `docs/superpowers/plans/2026-10-13-mcp-server.md` is cut into runs (section G). Seven are merged: the server crate, the backend's gateway, the files and stdio, the Agents tab's frame, changes made outside the app, and what agents are doing.

1. Done: the server, the gateway, the files and stdio, the Agents tab, changes made outside, what agents are doing.
2. **This run, 4b: the log, and a new agent.** It ends the fourth run. Question 17 was settled on 2026-10-14: a notification.
3. Then the fifth, letting agents in (Settings › Agents, the Connections list, the editor), and Windows' way in.

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: views draw and push `Action`s, `App::apply` changes state, and disk and desktop work is the backend's. Nothing an agent sent goes into the app's log: the log's read failures name the file, and a notification that fails names nothing.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.
- `Cargo.toml` gains `jiff` (Task 3) and, on Linux, `zbus` (Task 4). Their versions are the ones in `Cargo.lock`, and each diff adds the one line the lock file needs: `cargo build --locked` checks it. Nothing needs PostgreSQL or MySQL; the full run at the end prints "skipped" for their tests without the variables.

## What was run before this plan was written

Every task's state was built, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` (3,454 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. The Log view was rendered headless in both looks, with four calls and one selected, and looked at; the scene was thrown away. Not run: a real notification (no desktop here), macOS and Windows, which CI runs.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| How the log is read | From its file, whether the server runs or not: `Log::open_to_read` makes nothing and sets nothing aside, and `Log::page` reads 200 calls at a time, the newest first, from before the last one shown | D.5: the Log view works with the server off and with access Off. The page is the "answered in pages" D.5 asks for |
| A log a crash left mid-write | Opened to write, and never written: SQLite rolls the half-done write back on open, as the server's open would | A read-only open cannot roll it back, and so cannot read the file at all. Found by the review |
| When the log on screen is read again | When it is shown, when its scope changes, and when a call on its connection ends, on a first page; a call that ends while a read is out reads again once that read is in | The log is a list as it was when read; a call that ends is what the person is looking for. Found by the review |
| Where the log is | The tab's Log view in both looks: macOS's switch at the header's left, Omarchy's `l` (and `:agents log`) | D.5. Omarchy's board stacks the log under the feed; a view of its own reads the same in both looks, and `l` is D.6's key |
| What the log shows | This connection's calls, or every one's (macOS's segmented control, Omarchy's `L`), "N calls · last 30 days · on this computer", Time, Agent, the connection under All, Tool, Request, Rows, ms, Result, **Show more**, and the selected call's line | D.5's Phase 1b part. The filters, the pane, Export and Open in SQL editor are 3b's |
| **Show log** | In the Off here and nobody here states, with `l`. Not in Server off, whose board has Settings… and Turn on | The boards. The switch and `l` reach the log from every state |
| The clock | jiff, the system's time zone: "Oct 13 10:43" in the log, "10:43:02" in the feed, "connected 10:38" on a session's line, "Oct 13 10:43:40" on a call's line | Agreed on 2026-10-14. jiff is built already, through the logger |
| When an agent is told of | Once for each session, when its agent has named itself, while `[mcp] notify` is on (by default) | D.4: one notification for each session. A name is what the notification says |
| How | Linux: a notification over D-Bus with `zbus`, with Review, waited for an hour at most. macOS: the Dock's bounce. The text names the agent, never a statement | D.4. A macOS notification needs the signed bundle |
| Review | Tabletist forward, on the Agents tab of the window in front; with Connections in front, of the last window open; Connections where none is open | Question 22. Found by the review: Connections in front is not "no window open" |

### What this run leaves out

- The log's agent, result and search filters, its detail pane, **Export…** and **Open in SQL editor**: Phase 3b (D.5).
- **Show Agents tab** and **Show log** in Settings › Agents: the fifth run.
- The notification for a write request: Phase 2.
- A notification on macOS (`UNUserNotificationCenter`) needs the signed bundle: the Dock's bounce stands for it, as D.4 has it.

## File structure

| File | What changes |
|---|---|
| `crates/tabletist-mcp/src/log.rs`, `lib.rs` | `Log::open_to_read`, `Log::page`, `Logged`, `Page` |
| `src/backend/agents.rs`, `src/backend.rs` | `AgentsCommand::ReadLog`, `AgentsEvent::LogRead`, `LOG_PAGE`; `AgentsCommand::Notify`, `AgentsEvent::Reviewed` |
| `src/backend/agents/notify.rs` | New, Linux: the notification over D-Bus, and its Review |
| `src/model.rs` | `AgentsView`, `AgentsLog`, `AgentsTab::new`; `AgentsState::told`; the Log view's actions |
| `src/app.rs` | Reading the log, a page at a time; telling of a session; Review; `window_asks` |
| `src/clock.rs`, `src/lib.rs` | New: times in the system's time zone |
| `src/ui/agents.rs` | The Activity and Log switch, the Log view, **Show log**, the clock's times |
| `src/keymap.rs`, `src/ui/keys.rs`, `src/app/editing.rs`, `src/ui/workspace.rs` | `l`, `L`, `j`/`k` in the log, `:agents log`, Omarchy's status line |
| `src/settings.rs` | `[mcp] notify` |
| `Cargo.toml`, `Cargo.lock` | `jiff`; `zbus` on Linux |
| `src/ui/agents_tests.rs` | The log in both looks; the tests read the clock's times |
| `docs/_guide/omarchy.md`, `docs/_reference/settings-and-files.md` | The keys, and `notify` |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | What this run built |

---

### Task 1: The log, read a page at a time

`Log::open_to_read` opens the log's file to read it as it is: no file is made where there is none (`None`), one of another version is an error, left where it is, and nothing is written. It is opened to write all the same: a write a crash left half done is rolled back on open, as SQLite does, which a read-only connection cannot do and so cannot read the file at all. `Log::page(connection, before, limit)` is `limit` calls the newest first, from before the call `before` names, one connection's or every one's, with how many the log holds of them and whether there are more. `Log::latest` reads through it.

**Files:**
- Modify: `crates/tabletist-mcp/src/log.rs`, `crates/tabletist-mcp/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/crates/tabletist-mcp/src/log.rs b/crates/tabletist-mcp/src/log.rs
index 3826145..29cde3c 100644
--- a/crates/tabletist-mcp/src/log.rs
+++ b/crates/tabletist-mcp/src/log.rs
@@ -407,6 +407,87 @@ mod tests {
         opened.log
     }
 
+    #[tokio::test]
+    async fn the_log_is_read_a_page_at_a_time_for_a_connection_or_for_all() {
+        let dir = tempfile::tempdir().unwrap();
+        let log = open(&dir.path().join("agents.sqlite3"));
+        for n in 0..5 {
+            let mut call = entry(n, "run_query");
+            call.connection_id = Some(if n % 2 == 0 { "shop" } else { "ledger" }.into());
+            log.record(call).await.unwrap();
+        }
+        let at = |page: &Page| {
+            page.calls
+                .iter()
+                .map(|logged| logged.entry.at_ms)
+                .collect::<Vec<_>>()
+        };
+        let first = log.page(None, None, 2).await.unwrap();
+        assert_eq!((at(&first), first.total, first.more), (vec![4, 3], 5, true));
+        let next = log.page(None, Some(first.calls[1].id), 2).await.unwrap();
+        assert_eq!((at(&next), next.more), (vec![2, 1], true));
+        let shop = log.page(Some("shop".into()), None, 10).await.unwrap();
+        assert_eq!(
+            (at(&shop), shop.total, shop.more),
+            (vec![4, 2, 0], 3, false)
+        );
+    }
+
+    #[tokio::test]
+    async fn a_log_a_crash_left_mid_write_is_read_as_it_was_before() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        let log = open(&path);
+        for n in 0..2_000 {
+            log.record(entry(n, "run_query")).await.unwrap();
+        }
+        drop(log);
+        // A writer half way through, which spills to the file: the file
+        // and its journal as a crash leaves them, copied while it holds
+        // them.
+        let writer = Connection::open(&path).unwrap();
+        writer
+            .execute_batch("PRAGMA cache_size = 1; BEGIN; UPDATE calls SET tool = 'x';")
+            .unwrap();
+        let crashed = dir.path().join("crashed.sqlite3");
+        std::fs::copy(&path, &crashed).unwrap();
+        let journal = |path: &Path| {
+            let mut name = path.as_os_str().to_owned();
+            name.push("-journal");
+            PathBuf::from(name)
+        };
+        std::fs::copy(journal(&path), journal(&crashed)).unwrap();
+        drop(writer);
+        let log = Log::open_to_read(&crashed).unwrap().unwrap();
+        let page = log.page(None, None, 1).await.unwrap();
+        assert_eq!(page.total, 2_000);
+        assert_eq!(page.calls[0].entry.tool, "run_query");
+    }
+
+    #[tokio::test]
+    async fn a_log_opened_to_read_is_left_as_it_is() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        // None yet, and none made.
+        assert!(Log::open_to_read(&path).unwrap().is_none());
+        assert!(!path.exists());
+        open(&path)
+            .record(entry(7, "describe_schema"))
+            .await
+            .unwrap();
+        let log = Log::open_to_read(&path).unwrap().unwrap();
+        assert_eq!(log.page(None, None, 10).await.unwrap().total, 1);
+        // A file that is not a log is said so, and left where it is.
+        let other = dir.path().join("other.sqlite3");
+        let made = Connection::open(&other).unwrap();
+        made.execute_batch("PRAGMA user_version = 9; CREATE TABLE t (x)")
+            .unwrap();
+        drop(made);
+        assert!(Log::open_to_read(&other).is_err());
+        assert!(other.exists());
+        assert!(!dir.path().join("other.sqlite3.bad").exists());
+    }
+
     #[tokio::test]
     async fn an_entry_is_read_back_as_it_was_written() {
         let dir = tempfile::tempdir().unwrap();
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked -p tabletist-mcp --lib log::`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0425]: cannot find type `Page` in this scope
error[E0599]: no method named `page` found for struct `log::Log` in the current scope
error[E0599]: no associated function or constant named `open_to_read` found for struct `log::Log` in the current scope
error: could not compile `tabletist-mcp` (lib test) due to 8 previous errors
```

- [ ] **Step 3: `open_to_read` and `page`**

````diff
diff --git a/crates/tabletist-mcp/src/lib.rs b/crates/tabletist-mcp/src/lib.rs
index 29cf4ab..918cf4b 100644
--- a/crates/tabletist-mcp/src/lib.rs
+++ b/crates/tabletist-mcp/src/lib.rs
@@ -28,7 +28,7 @@ mod tools;
 
 pub use answer::{MAX_ANSWER_BYTES, MAX_CELL_CHARS};
 pub use gateway::{Failure, Gateway, Listed, Schemas, Session};
-pub use log::{Entry, Log, LogError, Opened};
+pub use log::{Entry, Log, LogError, Logged, Opened, Page};
 pub use refusal::{Refused, code};
 pub use server::{CALL_CHARS, Call, Origin, Server, SessionInfo, Transport};
 pub use tools::{Tool, all as tools};
diff --git a/crates/tabletist-mcp/src/log.rs b/crates/tabletist-mcp/src/log.rs
index 29cde3c..7c9c918 100644
--- a/crates/tabletist-mcp/src/log.rs
+++ b/crates/tabletist-mcp/src/log.rs
@@ -9,7 +9,7 @@ use std::path::{Path, PathBuf};
 use std::sync::{Arc, Mutex, PoisonError};
 use std::time::Duration;
 
-use rusqlite::{Connection, ErrorCode, params};
+use rusqlite::{Connection, ErrorCode, OpenFlags, Row, params};
 
 /// The schema's version, in `PRAGMA user_version`.
 const VERSION: i64 = 1;
@@ -114,6 +114,25 @@ impl From<rusqlite::Error> for LogError {
     }
 }
 
+/// A call as the log has it, with its place there: what the next page is
+/// read from.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Logged {
+    pub id: i64,
+    pub entry: Entry,
+}
+
+/// A page of the log, the newest first.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct Page {
+    pub calls: Vec<Logged>,
+    /// How many calls the log holds of what was asked: one connection's,
+    /// or every one.
+    pub total: u64,
+    /// Whether there are older ones.
+    pub more: bool,
+}
+
 /// The log, open. A clone writes to the same file.
 #[derive(Debug, Clone)]
 pub struct Log {
@@ -264,39 +283,75 @@ impl Log {
 
     /// The last `limit` entries, the newest first.
     pub async fn latest(&self, limit: u32) -> Result<Vec<Entry>, LogError> {
-        self.blocking(move |connection| {
-            let mut statement = connection.prepare(
-                "SELECT at_ms, session, agent, project, transport, connection_id, connection, \
-                 tool, sql, params, request, result, code, message, ran_as, rows, matched, \
-                 columns, bytes, took_ms FROM calls ORDER BY id DESC LIMIT ?1",
-            )?;
-            let entries = statement
-                .query_map([limit], |row| {
-                    Ok(Entry {
-                        at_ms: row.get(0)?,
-                        session: row.get(1)?,
-                        agent: row.get(2)?,
-                        project: row.get(3)?,
-                        transport: row.get(4)?,
-                        connection_id: row.get(5)?,
-                        connection: row.get(6)?,
-                        tool: row.get(7)?,
-                        sql: row.get(8)?,
-                        params: row.get(9)?,
-                        request: row.get(10)?,
-                        result: row.get(11)?,
-                        code: row.get(12)?,
-                        message: row.get(13)?,
-                        ran_as: row.get(14)?,
-                        rows: row.get(15)?,
-                        matched: row.get(16)?,
-                        columns: row.get(17)?,
-                        bytes: row.get(18)?,
-                        took_ms: row.get(19)?,
+        let page = self.page(None, None, limit).await?;
+        Ok(page.calls.into_iter().map(|logged| logged.entry).collect())
+    }
+
+    /// The log at `path`, to read it as it is: nothing is made and nothing
+    /// is set aside, so that reading it while the server is off changes
+    /// nothing. `None` where there is no log yet. A write that a crash left
+    /// half done is rolled back as the file is read, which SQLite does on
+    /// any open, and which a connection that may not write cannot do: it
+    /// is opened to write, and never writes.
+    pub fn open_to_read(path: &Path) -> Result<Option<Self>, LogError> {
+        if !path.exists() {
+            return Ok(None);
+        }
+        let connection = Connection::open_with_flags(
+            path,
+            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
+        )?;
+        connection.busy_timeout(LOCK_WAIT)?;
+        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
+        let tables: i64 =
+            connection.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))?;
+        match (version, tables) {
+            // Made, and not written yet.
+            (0, 0) => Ok(None),
+            (VERSION, _) => Ok(Some(Self {
+                connection: Arc::new(Mutex::new(connection)),
+            })),
+            (other, _) => Err(LogError(format!(
+                "the log is of version {other}, and this Tabletist reads version {VERSION}"
+            ))),
+        }
+    }
+
+    /// A page of the log, the newest first: `limit` calls from before the
+    /// one `before` names, those on `connection` alone where it is given.
+    pub async fn page(
+        &self,
+        connection: Option<String>,
+        before: Option<i64>,
+        limit: u32,
+    ) -> Result<Page, LogError> {
+        self.blocking(move |db| {
+            let mut statement = db.prepare(&format!(
+                "SELECT id, {COLUMNS} FROM calls \
+                 WHERE (?1 IS NULL OR connection_id = ?1) AND (?2 IS NULL OR id < ?2) \
+                 ORDER BY id DESC LIMIT ?3"
+            ))?;
+            // One more than asked: whether there are more.
+            let mut calls = statement
+                .query_map(params![connection, before, i64::from(limit) + 1], |row| {
+                    Ok(Logged {
+                        id: row.get(0)?,
+                        entry: entry_at(row, 1)?,
                     })
                 })?
-                .collect::<Result<Vec<Entry>, rusqlite::Error>>()?;
-            Ok(entries)
+                .collect::<Result<Vec<Logged>, rusqlite::Error>>()?;
+            let more = calls.len() > limit as usize;
+            calls.truncate(limit as usize);
+            let total: i64 = db.query_row(
+                "SELECT count(*) FROM calls WHERE ?1 IS NULL OR connection_id = ?1",
+                [&connection],
+                |row| row.get(0),
+            )?;
+            Ok(Page {
+                calls,
+                total: u64::try_from(total).unwrap_or_default(),
+                more,
+            })
         })
         .await
     }
@@ -329,6 +384,37 @@ impl Log {
     }
 }
 
+/// The columns of an entry, in [`entry_at`]'s order.
+const COLUMNS: &str = "at_ms, session, agent, project, transport, connection_id, connection, \
+    tool, sql, params, request, result, code, message, ran_as, rows, matched, columns, bytes, \
+    took_ms";
+
+/// The entry in `row`, from its column `at` on, in [`COLUMNS`]'s order.
+fn entry_at(row: &Row<'_>, at: usize) -> Result<Entry, rusqlite::Error> {
+    Ok(Entry {
+        at_ms: row.get(at)?,
+        session: row.get(at + 1)?,
+        agent: row.get(at + 2)?,
+        project: row.get(at + 3)?,
+        transport: row.get(at + 4)?,
+        connection_id: row.get(at + 5)?,
+        connection: row.get(at + 6)?,
+        tool: row.get(at + 7)?,
+        sql: row.get(at + 8)?,
+        params: row.get(at + 9)?,
+        request: row.get(at + 10)?,
+        result: row.get(at + 11)?,
+        code: row.get(at + 12)?,
+        message: row.get(at + 13)?,
+        ran_as: row.get(at + 14)?,
+        rows: row.get(at + 15)?,
+        matched: row.get(at + 16)?,
+        columns: row.get(at + 17)?,
+        bytes: row.get(at + 18)?,
+        took_ms: row.get(at + 19)?,
+    })
+}
+
 /// Makes the file when it is not there, and makes it its owner's alone
 /// whether it was there or not: SQLite would make it with the permissions
 /// any new file gets, and a file that was there has whatever it was given.
````

- [ ] **Step 4: Run the tests**

Run: the same, then `cargo test --locked -p tabletist-mcp`

Expected: PASS, and the crate's other tests with them.

```text
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 38 filtered out
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-mcp
git commit -m "Read the log of agents' calls a page at a time, without changing it"
```

---

### Task 2: The app reads the log

- `AgentsCommand::ReadLog { log, connection, before, asked }`: the backend opens the file to read and answers `AgentsEvent::LogRead { asked, page }`, with the server off too. No file is an empty page. `LOG_PAGE` is 200.
- `AgentsTab` has a view (`AgentsView::Activity` or `Log`) and the log's page (`AgentsLog`): this connection's or all, the calls, how many, whether more, the read that is out, why it failed, the selected call.
- `ShowAgentsView` opens the tab on a view and reads the first page; `ToggleAgentsLogScope` reads every connection's or this one's again; `MoreAgentsLog` reads on after the last call; `SelectAgentsLogCall` and `MoveAgentsLogSelection` select. An answer is taken only by the tab whose read it answers.
- A call that ends on a connection reads again the first page of a Log view that shows it; where a read is out, which may not hold the call, once that read is in (`AgentsLog::stale`).

**Files:**
- Modify: `src/backend/agents.rs`, `src/backend.rs`, `src/model.rs`, `src/app.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 5d252fb..039f475 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -6177,6 +6177,153 @@ mod tests {
         assert_eq!(app.active_tab_id(), opened);
     }
 
+    /// The read of the log the app asked for last: whose calls, from
+    /// before which, and its id.
+    fn log_asked(app: &App) -> Option<(Option<String>, Option<i64>, u64)> {
+        agents_sent(app)
+            .into_iter()
+            .rev()
+            .find_map(|sent| match sent {
+                AgentsCommand::ReadLog {
+                    connection,
+                    before,
+                    asked,
+                    ..
+                } => Some((connection.clone(), *before, *asked)),
+                _ => None,
+            })
+    }
+
+    /// A page of `count` calls on `conn`, the newest first, from `top` down.
+    fn log_page(conn: &str, top: i64, count: i64, more: bool) -> tabletist_mcp::Page {
+        let calls = (0..count)
+            .map(|n| tabletist_mcp::Logged {
+                id: top - n,
+                entry: tabletist_mcp::Entry {
+                    at_ms: 1_791_880_982_000,
+                    agent: "Claude Code".into(),
+                    connection_id: Some(conn.into()),
+                    tool: "run_query".into(),
+                    sql: Some(format!("SELECT {n}")),
+                    result: "ok".into(),
+                    ..tabletist_mcp::Entry::default()
+                },
+            })
+            .collect();
+        tabletist_mcp::Page {
+            calls,
+            total: 186,
+            more,
+        }
+    }
+
+    fn agents_log(app: &App, tab: ConnTabId) -> crate::model::AgentsLog {
+        app.workspace(tab)
+            .unwrap()
+            .tabs
+            .iter()
+            .find_map(|open| match open {
+                Tab::Agents(agents) => Some(agents.log.clone()),
+                _ => None,
+            })
+            .unwrap()
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn the_log_is_read_a_page_at_a_time_for_this_connection_or_for_all() {
+        use crate::model::AgentsView;
+        let (mut app, dir) = app();
+        let (tab, _, _) = connect(&mut app);
+        let conn = app.workspace(tab).unwrap().conn_id.0.clone();
+        // Shown on its Log view: this connection's first page, from the
+        // log's file, whether the server runs or not.
+        app.apply(Action::ShowAgentsView {
+            tab,
+            view: AgentsView::Log,
+        });
+        assert!(app.workspace(tab).unwrap().agents_in_front());
+        assert!(matches!(
+            agents_sent(&app).last(),
+            Some(AgentsCommand::ReadLog { log, .. }) if *log == AppDirs::at(dir.path()).agents_log_file()
+        ));
+        let (connection, before, asked) = log_asked(&app).unwrap();
+        assert_eq!((connection.as_deref(), before), (Some(conn.as_str()), None));
+        // An answer to a read that is not the one out is not taken.
+        let page = Ok(log_page(&conn, 500, 200, true));
+        app.apply_event(Event::Agents(AgentsEvent::LogRead {
+            asked: asked + 99,
+            page: page.clone(),
+        }));
+        assert!(!agents_log(&app, tab).read);
+        app.apply_event(Event::Agents(AgentsEvent::LogRead { asked, page }));
+        let log = agents_log(&app, tab);
+        assert_eq!((log.calls.len(), log.total, log.more), (200, 186, true));
+        // More: from before the last call shown, after it.
+        app.apply(Action::SelectAgentsLogCall { tab, id: 499 });
+        app.apply(Action::MoreAgentsLog(tab));
+        let (_, before, asked) = log_asked(&app).unwrap();
+        assert_eq!(before, Some(301));
+        let page = Ok(log_page(&conn, 300, 10, false));
+        app.apply_event(Event::Agents(AgentsEvent::LogRead { asked, page }));
+        let log = agents_log(&app, tab);
+        assert_eq!(
+            (log.calls.len(), log.more, log.selected),
+            (210, false, Some(499))
+        );
+        // Every connection's: read again from the top.
+        app.apply(Action::ToggleAgentsLogScope(tab));
+        let (connection, before, asked) = log_asked(&app).unwrap();
+        assert_eq!((connection, before), (None, None));
+        // A log that cannot be read says why.
+        let page = Err("the log is of version 9".to_owned());
+        app.apply_event(Event::Agents(AgentsEvent::LogRead { asked, page }));
+        let log = agents_log(&app, tab);
+        assert_eq!(log.failed.as_deref(), Some("the log is of version 9"));
+        assert!(log.all);
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn a_call_on_the_connection_reads_the_log_again() {
+        let (mut app, _dir) = app();
+        let (tab, _, _) = connect(&mut app);
+        let conn = app.workspace(tab).unwrap().conn_id.0.clone();
+        app.apply(Action::ShowAgentsView {
+            tab,
+            view: crate::model::AgentsView::Log,
+        });
+        let (_, _, asked) = log_asked(&app).unwrap();
+        let page = Ok(log_page(&conn, 3, 3, false));
+        app.apply_event(Event::Agents(AgentsEvent::LogRead { asked, page }));
+        let call = |on: &str| tabletist_mcp::Call {
+            at: std::time::SystemTime::now(),
+            session: "one".into(),
+            connection: Some(on.into()),
+            tool: "run_query".into(),
+            request: Some("SELECT 1".into()),
+            result: "ok".into(),
+            message: None,
+            rows: Some(1),
+            columns: Some(1),
+            took: std::time::Duration::from_millis(2),
+        };
+        app.backend.sent.clear();
+        // Another connection's: nothing to read.
+        app.apply_event(Event::Agents(AgentsEvent::Called(call("other"))));
+        assert_eq!(log_asked(&app), None);
+        app.apply_event(Event::Agents(AgentsEvent::Called(call(&conn))));
+        let (on, _, asked) = log_asked(&app).unwrap();
+        assert_eq!(on.as_deref(), Some(conn.as_str()));
+        // One that ends while that read is out, which may not hold it:
+        // read again once it is in.
+        app.apply_event(Event::Agents(AgentsEvent::Called(call(&conn))));
+        assert_eq!(log_asked(&app).unwrap().2, asked);
+        let page = Ok(log_page(&conn, 4, 4, false));
+        app.apply_event(Event::Agents(AgentsEvent::LogRead { asked, page }));
+        assert_ne!(log_asked(&app).unwrap().2, asked);
+    }
+
     #[test]
     fn agents_are_told_of_each_change_to_what_they_may_use() {
         let (mut app, _dir) = app();
diff --git a/src/backend.rs b/src/backend.rs
index 20c06f7..037a9d6 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -20,7 +20,9 @@ use crate::secrets::{Keyring, SecretString};
 
 mod agents;
 
-pub use agents::{Agentable, AgentsCommand, AgentsEvent, AppGateway, agent_names, agentable};
+pub use agents::{
+    Agentable, AgentsCommand, AgentsEvent, AppGateway, LOG_PAGE, agent_names, agentable,
+};
 
 /// Wakes the UI from any thread. The default does nothing (tests).
 #[derive(Clone)]
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 59eabf4..973fd8b 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -1566,6 +1566,48 @@ mod tests {
         assert!(!path.exists());
     }
 
+    #[test]
+    fn the_log_is_read_with_the_server_off_and_reading_it_makes_nothing() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
+        let read = |backend: &mut Backend, connection: Option<&str>| {
+            backend.send(Command::Agents(AgentsCommand::ReadLog {
+                log: path.clone(),
+                connection: connection.map(str::to_owned),
+                before: None,
+                asked: 7,
+            }));
+            match told(backend, WAIT) {
+                Some(Event::Agents(AgentsEvent::LogRead { asked: 7, page })) => page.unwrap(),
+                other => panic!("{other:?}"),
+            }
+        };
+        // No log yet: an empty page, and no file made.
+        assert_eq!(read(&mut backend, None), tabletist_mcp::Page::default());
+        assert!(!path.exists());
+        let runtime = tokio::runtime::Runtime::new().unwrap();
+        runtime.block_on(async {
+            let log = Log::open(&path).unwrap().log;
+            for on in ["shop", "ledger"] {
+                let entry = tabletist_mcp::Entry {
+                    at_ms: 1,
+                    session: "s".into(),
+                    agent: "Claude Code".into(),
+                    transport: "stdio".into(),
+                    connection_id: Some(on.into()),
+                    tool: "run_query".into(),
+                    result: "ok".into(),
+                    ..tabletist_mcp::Entry::default()
+                };
+                log.record(entry).await.unwrap();
+            }
+        });
+        let shop = read(&mut backend, Some("shop"));
+        assert_eq!((shop.calls.len(), shop.total), (1, 1));
+        assert_eq!(read(&mut backend, None).total, 2);
+    }
+
     #[test]
     fn a_server_says_its_sessions_as_it_starts() {
         let dir = tempfile::tempdir().unwrap();
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- app::tests::the_log_is app::tests::a_call_on_the backend::agents::tests::the_log_is`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0432]: unresolved import `agents::LOG_PAGE`
error[E0432]: unresolved import `crate::model::AgentsView`
error[E0425]: cannot find type `AgentsLog` in module `crate::model`
error[E0599]: no variant named `ReadLog` found for enum `backend::agents::AgentsCommand`
error[E0599]: no variant named `ShowAgentsView` found for enum `model::Action`
error[E0599]: no variant named `LogRead` found for enum `backend::agents::AgentsEvent`
error: could not compile `tabletist` (lib test) due to 20 previous errors
```

- [ ] **Step 3: Read it, a page at a time**

````diff
diff --git a/src/app.rs b/src/app.rs
index 039f475..3f680f6 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1440,6 +1440,54 @@ impl App {
                     agents.selected = selected;
                 }
             }
+            Action::ShowAgentsView { tab, view } => {
+                self.show_agents(tab);
+                let shown = self
+                    .workspace_mut(tab)
+                    .and_then(crate::model::Workspace::agents_tab_mut)
+                    .map(|agents| agents.view = view);
+                // The log as it is now, each time it is shown.
+                if shown.is_some() && view == crate::model::AgentsView::Log {
+                    self.read_agents_log(tab, false);
+                }
+            }
+            Action::ToggleAgentsLogScope(tab) => {
+                if let Some(agents) = self
+                    .workspace_mut(tab)
+                    .and_then(crate::model::Workspace::agents_tab_mut)
+                {
+                    agents.log.all = !agents.log.all;
+                    agents.log.selected = None;
+                }
+                self.read_agents_log(tab, false);
+            }
+            Action::MoreAgentsLog(tab) => self.read_agents_log(tab, true),
+            Action::SelectAgentsLogCall { tab, id } => {
+                if let Some(agents) = self
+                    .workspace_mut(tab)
+                    .and_then(crate::model::Workspace::agents_tab_mut)
+                {
+                    agents.log.selected = Some(id);
+                }
+            }
+            Action::MoveAgentsLogSelection { tab, step } => {
+                if let Some(agents) = self
+                    .workspace_mut(tab)
+                    .and_then(crate::model::Workspace::agents_tab_mut)
+                {
+                    let log = &mut agents.log;
+                    let at = log
+                        .selected
+                        .and_then(|id| log.calls.iter().position(|logged| logged.id == id));
+                    let to = match at {
+                        None => 0,
+                        Some(at) => at
+                            .saturating_add_signed(step)
+                            .min(log.calls.len().saturating_sub(1)),
+                    };
+                    log.selected = log.calls.get(to).map(|logged| logged.id);
+                }
+            }
             Action::KeepAgentChanges(said) => self.keep_agent_changes(&said),
             Action::PutBackAgentChanges(said) => self.put_back_agent_changes(&said),
             Action::RunSql { tab, sql_tab, all } => {
@@ -2881,7 +2929,12 @@ impl App {
                     self.agents.sessions = open;
                 }
                 AgentsEvent::Sessions(_) => {}
-                AgentsEvent::Called(call) => self.agents.called(call),
+                AgentsEvent::Called(call) => {
+                    let on = call.connection.clone();
+                    self.agents.called(call);
+                    self.log_called(on.as_deref());
+                }
+                AgentsEvent::LogRead { asked, page } => self.agents_log_read(asked, page),
                 AgentsEvent::Started {
                     kept_aside: Some(path),
                 } => {
@@ -3724,13 +3777,121 @@ impl App {
         let shown = workspace.agents_tab().unwrap_or_else(|| {
             workspace
                 .tabs
-                .push(Tab::Agents(crate::model::AgentsTab { id, selected: None }));
+                .push(Tab::Agents(crate::model::AgentsTab::new(id)));
             id
         });
         workspace.active_tab = Some(shown);
         workspace.pane = Pane::Grid;
     }
 
+    /// Reads the first page of the Log view of `tab`, or, with `more`, the
+    /// page after its last call. One read at a time.
+    fn read_agents_log(&mut self, tab: ConnTabId, more: bool) {
+        let asked = self.next_id();
+        let log = self.dirs.agents_log_file();
+        let Some(workspace) = self.workspace_mut(tab) else {
+            return;
+        };
+        let conn = workspace.conn_id.0.clone();
+        let Some(agents) = workspace.agents_tab_mut() else {
+            return;
+        };
+        if more && (agents.log.asked.is_some() || !agents.log.more) {
+            return;
+        }
+        let before = more
+            .then(|| agents.log.calls.last().map(|logged| logged.id))
+            .flatten();
+        let connection = (!agents.log.all).then_some(conn);
+        agents.log.asked = Some((asked, more));
+        self.backend.send(Command::Agents(AgentsCommand::ReadLog {
+            log,
+            connection,
+            before,
+            asked,
+        }));
+    }
+
+    /// A page of the log, for the tab that asked for it: the first page in
+    /// the place of what was shown, a later one after it.
+    fn agents_log_read(&mut self, asked: u64, page: Result<tabletist_mcp::Page, String>) {
+        let tabs: Vec<ConnTabId> = self.open_connections().map(|(tab, _)| tab).collect();
+        let mut stale = Vec::new();
+        for tab in tabs {
+            let Some(agents) = self
+                .workspace_mut(tab)
+                .and_then(crate::model::Workspace::agents_tab_mut)
+            else {
+                continue;
+            };
+            let log = &mut agents.log;
+            let Some((_, more)) = log.asked.filter(|(id, _)| *id == asked) else {
+                continue;
+            };
+            log.asked = None;
+            log.read = true;
+            // A call that ended while this read was out: read again, from
+            // the top, once a first page is in.
+            let again = std::mem::take(&mut log.stale) && !more;
+            if again {
+                stale.push(tab);
+            }
+            match &page {
+                Ok(page) => {
+                    if !more {
+                        log.calls.clear();
+                    }
+                    log.calls.extend(page.calls.iter().cloned());
+                    log.total = page.total;
+                    log.more = page.more;
+                    log.failed = None;
+                    // A call that is not on the page any more is not
+                    // selected.
+                    if let Some(id) = log.selected
+                        && !log.calls.iter().any(|logged| logged.id == id)
+                    {
+                        log.selected = None;
+                    }
+                }
+                Err(error) => {
+                    // Its words are the file's and SQLite's, never a call's.
+                    log::warn!("the log of agents' calls could not be read: {error}");
+                    log.failed = Some(error.clone());
+                }
+            }
+        }
+        for tab in stale {
+            self.read_agents_log(tab, false);
+        }
+    }
+
+    /// A call ended: each Log view that shows its connection's calls, on
+    /// its first page, reads it again; one whose read is out, once that
+    /// read is in.
+    fn log_called(&mut self, on: Option<&str>) {
+        let tabs: Vec<ConnTabId> = self.open_connections().map(|(tab, _)| tab).collect();
+        for tab in tabs {
+            let Some(workspace) = self.workspace_mut(tab) else {
+                continue;
+            };
+            let here = on == Some(workspace.conn_id.0.as_str());
+            let Some(agents) = workspace.agents_tab_mut() else {
+                continue;
+            };
+            let log = &mut agents.log;
+            let shows = agents.view == crate::model::AgentsView::Log
+                && log.calls.len() <= crate::backend::LOG_PAGE as usize
+                && (log.all || here);
+            if !shows {
+                continue;
+            }
+            match log.asked {
+                Some(_) => log.stale = true,
+                None => self.read_agents_log(tab, false),
+            }
+        }
+    }
+
     /// Stops one agent's session. Its line goes when the backend says the
     /// sessions again.
     fn stop_agent_session(&mut self, id: String) {
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 973fd8b..61d7a77 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -147,6 +147,16 @@ pub enum AgentsCommand {
     /// Stops one session, by its id: its calls answer that the server
     /// stopped them, and its agent is let go. Nothing where it is gone.
     StopSession(String),
+    /// Reads a page of the log at `log`, whether the server runs or not:
+    /// the calls on `connection`, or on every one, from before the call
+    /// `before` names. Answered with [`AgentsEvent::LogRead`], `asked`
+    /// with it.
+    ReadLog {
+        log: PathBuf,
+        connection: Option<String>,
+        before: Option<i64>,
+        asked: u64,
+    },
     /// Stops it: every session ends, and the connections are closed.
     /// Answered with [`AgentsEvent::Stopped`].
     Stop,
@@ -169,8 +179,17 @@ pub enum AgentsEvent {
     Sessions(Vec<tabletist_mcp::SessionInfo>),
     /// A call ended, and is in the log.
     Called(tabletist_mcp::Call),
+    /// A page of the log, for the [`AgentsCommand::ReadLog`] that `asked`;
+    /// or why it could not be read.
+    LogRead {
+        asked: u64,
+        page: Result<tabletist_mcp::Page, String>,
+    },
 }
 
+/// How many calls a page of the log holds.
+pub const LOG_PAGE: u32 = 200;
+
 /// How long a connection an agent used is kept open with nothing to do.
 const IDLE: Duration = Duration::from_secs(10 * 60);
 
@@ -233,6 +252,18 @@ impl Agents {
                 self.log_days
                     .store(days, std::sync::atomic::Ordering::SeqCst);
             }
+            AgentsCommand::ReadLog {
+                log,
+                connection,
+                before,
+                asked,
+            } => {
+                let outbox = outbox.clone();
+                tokio::spawn(async move {
+                    let page = read_log(log, connection, before).await;
+                    outbox.emit(super::Event::Agents(AgentsEvent::LogRead { asked, page }));
+                });
+            }
             AgentsCommand::StopSession(id) => {
                 if let Some(running) = lock(&self.running).as_ref()
                     && running.server.stop_session(&id)
@@ -425,6 +456,26 @@ async fn sessions(
     }
 }
 
+/// A page of the log at `log`, opened to read: nothing is made or set
+/// aside. No log yet is an empty one.
+async fn read_log(
+    log: PathBuf,
+    connection: Option<String>,
+    before: Option<i64>,
+) -> Result<tabletist_mcp::Page, String> {
+    let opened = tokio::task::spawn_blocking(move || Log::open_to_read(&log))
+        .await
+        .map_err(|_| "opening it failed".to_owned())?
+        .map_err(|error| error.to_string())?;
+    match opened {
+        Some(log) => log
+            .page(connection, before, LOG_PAGE)
+            .await
+            .map_err(|error| error.to_string()),
+        None => Ok(tabletist_mcp::Page::default()),
+    }
+}
+
 /// Tells the app of each call as it ends, until the server stops. One
 /// that falls behind misses the oldest: the log has them all.
 async fn called(mut calls: tokio::sync::broadcast::Receiver<tabletist_mcp::Call>, outbox: Outbox) {
diff --git a/src/model.rs b/src/model.rs
index bf43fac..ba05728 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -630,6 +630,27 @@ pub enum Action {
         tab: ConnTabId,
         step: isize,
     },
+    /// Show the Agents tab on one of its views, opening it where it is not
+    /// open: Show log, `l`, `:agents log`.
+    ShowAgentsView {
+        tab: ConnTabId,
+        view: AgentsView,
+    },
+    /// The Log view: every connection's calls, or this one's again.
+    ToggleAgentsLogScope(ConnTabId),
+    /// The Log view: read the page after the last call shown.
+    MoreAgentsLog(ConnTabId),
+    /// The Log view: select a call, by its place in the log.
+    SelectAgentsLogCall {
+        tab: ConnTabId,
+        id: i64,
+    },
+    /// The Log view: move the selection among the calls, in the terminal
+    /// look.
+    MoveAgentsLogSelection {
+        tab: ConnTabId,
+        step: isize,
+    },
     /// Keep what was changed outside the app and lets agents do more: what
     /// the bar said, and only what is still held of it.
     KeepAgentChanges(Vec<crate::agents_shown::Raised>),
@@ -2567,6 +2588,52 @@ pub struct AgentsTab {
     /// and `enter` shows. Kept by id, since the list puts the session that
     /// called last first: a key lands on the session the person saw.
     pub selected: Option<String>,
+    /// Which of its two views shows.
+    pub view: AgentsView,
+    pub log: AgentsLog,
+}
+
+impl AgentsTab {
+    pub fn new(id: TabId) -> Self {
+        Self {
+            id,
+            selected: None,
+            view: AgentsView::default(),
+            log: AgentsLog::default(),
+        }
+    }
+}
+
+/// The Agents tab's two views: what agents do now, and what they did.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
+pub enum AgentsView {
+    #[default]
+    Activity,
+    Log,
+}
+
+/// The Agents tab's Log view: the log of calls, a page at a time, the
+/// newest first, read from the log's file whether the server runs or not.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct AgentsLog {
+    /// Every connection's calls, not this one's alone.
+    pub all: bool,
+    pub calls: Vec<tabletist_mcp::Logged>,
+    /// How many calls the log holds of what is shown.
+    pub total: u64,
+    /// Whether there are older ones to read.
+    pub more: bool,
+    /// The read that is out, and whether it reads on after the last call.
+    pub asked: Option<(u64, bool)>,
+    /// A call ended while a read was out, which may not hold it: the first
+    /// page is read again once that read is in.
+    pub stale: bool,
+    /// Why the log could not be read.
+    pub failed: Option<String>,
+    /// The selected call, by its place in the log.
+    pub selected: Option<i64>,
+    /// Whether it was read once at least.
+    pub read: bool,
 }
 
 /// What the app knows of the agent server: the sessions open on it, as
````

- [ ] **Step 4: Run the tests**

Run: the same, then `cargo test --locked --lib app::` and `cargo test --locked --lib backend::`

Expected: PASS.

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 2498 filtered out
test result: ok. 403 passed; 0 failed; 0 ignored; 0 measured; 2098 filtered out
test result: ok. 144 passed; 0 failed; 0 ignored; 0 measured; 2357 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src
git commit -m "Read the log of agents' calls for the Agents tab, this connection's or all"
```

---

### Task 3: The Log view, and the clock

- `src/clock.rs`, with `jiff` (one line in `Cargo.toml` and one in `Cargo.lock`): "10:43:02", "10:38", "Oct 13 10:43", "Oct 13 10:43:40", in the system's time zone. The feed and the session's line say the time (as the boards draw them) where they said how long ago.
- macOS: the Activity and Log switch at the header's left. The Log view: This connection or All connections, "186 calls · last 30 days · on this computer", the calls (Time, Agent, the connection under All, Tool, Request, Rows, ms, Result), **Show more**, and the selected call's line at the bottom, which says rows are not logged.
- Omarchy: the same view, `l` to show it and to leave it, `L` for every connection's calls, `j`/`k` among them (the selected call kept in sight), `:agents log`; the status line says them, naming the tab's `l`, not the prompt's line. `o` and `y` are the Activity view's alone.
- **Show log** in the Off here and nobody here states.
- The tab's tests read the clock's times; the log's tests in both looks.

**Files:**
- Create: `src/clock.rs`
- Modify: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/ui/agents.rs`, `src/keymap.rs`, `src/ui/keys.rs`, `src/app/editing.rs`, `src/ui/workspace.rs`, `src/ui/agents_tests.rs`, `docs/_guide/omarchy.md`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/Cargo.lock b/Cargo.lock
index 1f0794c..de591ac 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -5687,6 +5687,7 @@ dependencies = [
  "fastframe-text",
  "fastframe-theme",
  "image",
+ "jiff",
  "keyring-core",
  "log",
  "notify",
diff --git a/Cargo.toml b/Cargo.toml
index a86ea22..6dc95bd 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -107,6 +107,10 @@ toml = { version = "1.1", default-features = false, features = ["parse", "serde"
 # features fastframe-theme already brings in for following the desktop's
 # theme: nothing new is built.
 notify = "8.2"
+# The person's clock and time zone, for the times of agents' calls. The
+# version and the features env_logger already brings in: nothing new is
+# built.
+jiff = "0.2"
 
 [target.'cfg(target_os = "linux")'.dependencies]
 # The desktop's Secret Service (GNOME Keyring, KWallet) over D-Bus.
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index d62c735..cbc0b9e 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -38,6 +38,7 @@ back.
 | F6, Shift+F6 | Next and previous part of the window |
 | Ctrl+, | Settings |
 | `:agents` | Show the Agents tab |
+| `:agents log` | Show the Agents tab's log |
 | `?` | The list of keys |
 
 Quick open has no key in the Omarchy look. `/` in the sidebar filters its
@@ -113,6 +114,9 @@ key. Nor has copying a whole row.
 | `S` | Stop the selected session, wherever it works |
 | `X` | Stop every agent here, and turn this connection's access off |
 | Enter | Show the window of the connection the selected agent works on |
+| `l` | Show the log of agents' calls, and back |
+| `L` | In the log, every connection's calls, or this one's |
+| `j` / `k` | In the log, move among the calls |
 
 ### The Structure view
 
diff --git a/src/clock.rs b/src/clock.rs
new file mode 100644
index 0000000..06bb51a
--- /dev/null
+++ b/src/clock.rs
@@ -0,0 +1,19 @@
+//! The time by the person's clock, in their time zone: when an agent's
+//! call came, as the Agents tab says it.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn a_time_is_said_in_the_zone_it_is_read_in() {
+        // 2026-10-13 08:43:02 UTC.
+        let at = of_millis(1_791_880_982_000);
+        let berlin = TimeZone::fixed(jiff::tz::offset(2));
+        assert_eq!(format(at, &berlin, "%H:%M:%S"), "10:43:02");
+        assert_eq!(format(at, &berlin, "%H:%M"), "10:43");
+        assert_eq!(format(at, &berlin, "%b %-d %H:%M"), "Oct 13 10:43");
+        assert_eq!(format(at, &berlin, "%b %-d %H:%M:%S"), "Oct 13 10:43:02");
+        assert_eq!(format(at, &TimeZone::UTC, "%b %-d %H:%M"), "Oct 13 08:43");
+    }
+}
diff --git a/src/lib.rs b/src/lib.rs
index dc0549d..ba2ba18 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -3,6 +3,7 @@
 pub mod agents_shown;
 pub mod app;
 pub mod backend;
+pub mod clock;
 pub mod completion;
 pub mod connections;
 pub mod edit;
diff --git a/src/ui/agents.rs b/src/ui/agents.rs
index 7e987ba..57c296c 100644
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -1014,15 +1014,6 @@ pub fn ago(since: std::time::Duration, locale: crate::i18n::Locale) -> String {
 
 #[cfg(test)]
 mod tests {
-    #[test]
-    fn a_while_ago_is_said_in_minutes_then_hours() {
-        let locale = crate::i18n::Locale::default();
-        let ago = |secs| super::ago(std::time::Duration::from_secs(secs), locale);
-        assert_eq!(ago(30), "just now");
-        assert_eq!(ago(240), "4 min ago");
-        assert_eq!(ago(2 * 3600 + 60), "2 h ago");
-    }
-
     /// Where agents cannot reach the app, nothing of them shows: no entry,
     /// and no tab.
     #[cfg(not(unix))]
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index ab4c19a..152d699 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -45,13 +45,23 @@ fn session(harness: &Harness, tab: Option<ConnTabId>, agent: &str) -> tabletist_
         agent: Some(agent.to_owned()),
         project: Some("tabletist-repo".into()),
         transport: tabletist_mcp::Transport::Stdio,
-        connected_at: SystemTime::now(),
+        connected_at: at(),
         calls: 14,
         connections,
         last_call_at: Some(SystemTime::now()),
     }
 }
 
+/// When the tests' sessions connected and their calls came.
+fn at() -> SystemTime {
+    SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_791_880_982)
+}
+
+/// "connected 10:43", as the clock says it here.
+fn connected_at() -> String {
+    format!("connected {}", crate::clock::hour(at()))
+}
+
 fn said(harness: &mut Harness, event: AgentsEvent) {
     harness.app.apply(Action::Backend(Event::Agents(event)));
     harness.settle();
@@ -247,11 +257,17 @@ fn the_tab_shows_its_state_and_its_buttons_do_what_they_say() {
         // In the list at the left in the terminal look, its line over the
         // calls; in a row with Stop session in the macOS look.
         let said = if look.terminal {
-            "claude code  tabletist-repo · connected just now · 14 calls · runs as dev"
+            format!(
+                "claude code  tabletist-repo · {} · 14 calls · runs as dev",
+                connected_at()
+            )
         } else {
-            "Claude Code · tabletist-repo · connected just now · 14 calls · Runs as dev"
+            format!(
+                "Claude Code · tabletist-repo · {} · 14 calls · Runs as dev",
+                connected_at()
+            )
         };
-        assert!(harness.has(said), "{}", look.name);
+        assert!(harness.has(&said), "{}", look.name);
         assert!(
             harness.has(&look.label("No calls on this connection yet.")),
             "{}",
@@ -301,7 +317,10 @@ fn the_tab_names_the_connection_as_an_agent_does_and_counts_only_who_is_here() {
     assert!(harness.has("No agents connected to Fixture"));
     let here = session(&harness, Some(tab), "Claude Code");
     said(&mut harness, AgentsEvent::Sessions(vec![here]));
-    assert!(harness.has("Claude Code · tabletist-repo · connected just now · 14 calls"));
+    assert!(harness.has(&format!(
+        "Claude Code · tabletist-repo · {} · 14 calls",
+        connected_at()
+    )));
     assert!(harness.has("1 agent here · Server: stdio"));
     // Its access Off, the agent that worked here is not here.
     allow(&mut harness, tab, AgentAccess::Off);
@@ -351,14 +370,18 @@ fn more_sessions_than_the_tab_has_room_for_scroll_to_the_last() {
         .unwrap();
     // The last row, the agent that called first, is under the status bar,
     // out of sight, until scrolled.
-    let last = "Agent 0 · tabletist-repo · connected just now · 14 calls · Runs as dev";
+    let last = format!(
+        "Agent 0 · tabletist-repo · {} · 14 calls · Runs as dev",
+        connected_at()
+    );
+    let painted = format!("Agent 0  tabletist-repo · {} · 14 calls", connected_at());
     let seen = |harness: &Harness| {
         harness
-            .painted_rect("Agent 0  tabletist-repo · connected just now · 14 calls")
+            .painted_rect(&painted)
             .is_some_and(|row| row.bottom() <= status.top())
     };
     assert!(!seen(&harness));
-    assert!(harness.has(last));
+    assert!(harness.has(&last));
     let middle = harness.size / 2.0;
     for _ in 0..20 {
         harness.frame(vec![
@@ -485,7 +508,7 @@ fn working_elsewhere(harness: &mut Harness, agent: &str) -> crate::connections::
 /// A call of the session of `agent`'s on `tab`'s connection, or on another.
 fn call(harness: &Harness, tab: Option<ConnTabId>, sql: &str) -> tabletist_mcp::Call {
     tabletist_mcp::Call {
-        at: SystemTime::now(),
+        at: at(),
         session: "session of Claude Code".into(),
         connection: Some(tab.map_or("ledger".into(), |tab| {
             harness.app.workspace(tab).unwrap().conn_id.0.clone()
@@ -513,7 +536,7 @@ fn the_feed_shows_the_calls_on_this_connection_as_they_end() {
         said(&mut harness, AgentsEvent::Called(mine));
         let theirs = call(&harness, None, "SELECT 2");
         said(&mut harness, AgentsEvent::Called(theirs));
-        let row = look.label("just now");
+        let row = look.label(&crate::clock::time(at()));
         assert!(
             harness.has(&format!("{row} · run_query · SELECT 1 · 1 row · 2 ms")),
             "{}",
@@ -757,3 +780,204 @@ fn omarchy_lists_who_works_elsewhere_while_nobody_is_here() {
     let shown = harness.app.active_tab_id();
     assert_eq!(harness.app.workspace(shown).unwrap().conn_id, ledger);
 }
+
+/// The id of the log's read the app asked for last, and whose calls.
+fn log_asked(harness: &Harness) -> (u64, Option<String>) {
+    harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|sent| match sent {
+            crate::backend::Command::Agents(crate::backend::AgentsCommand::ReadLog {
+                asked,
+                connection,
+                ..
+            }) => Some((*asked, connection.clone())),
+            _ => None,
+        })
+        .expect("a read of the log")
+}
+
+/// Answers the log's read with one call of Claude Code's.
+fn log_answered(harness: &mut Harness) {
+    let (asked, connection) = log_asked(harness);
+    let entry = tabletist_mcp::Entry {
+        at_ms: 1_791_880_982_000,
+        session: "s".into(),
+        agent: "Claude Code".into(),
+        transport: "stdio".into(),
+        connection_id: connection,
+        connection: Some("Fixture · dev".into()),
+        tool: "run_query".into(),
+        sql: Some("SELECT  id\n  FROM books".into()),
+        result: "ok".into(),
+        rows: Some(3),
+        columns: Some(2),
+        took_ms: Some(4),
+        ..tabletist_mcp::Entry::default()
+    };
+    let page = tabletist_mcp::Page {
+        calls: vec![tabletist_mcp::Logged { id: 5, entry }],
+        total: 1,
+        more: false,
+    };
+    said(
+        harness,
+        AgentsEvent::LogRead {
+            asked,
+            page: Ok(page),
+        },
+    );
+}
+
+#[test]
+fn the_log_lists_the_calls_and_says_what_the_selected_one_came_to() {
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        harness.app.settings.mcp_enabled = true;
+        allow(&mut harness, tab, AgentAccess::Read);
+        harness.app.apply(Action::ShowAgents(tab));
+        harness.settle();
+        // From nobody here: Show log, which reads this connection's calls.
+        harness.click("Show log");
+        harness.settle();
+        let conn = harness.app.workspace(tab).unwrap().conn_id.0.clone();
+        assert_eq!(
+            log_asked(&harness).1.as_deref(),
+            Some(conn.as_str()),
+            "{}",
+            look.name
+        );
+        log_answered(&mut harness);
+        let at = crate::clock::of_millis(1_791_880_982_000);
+        let row = format!(
+            "{} · {} · run_query · SELECT id FROM books · 3 · 4 · ok",
+            look.label(&crate::clock::day(at)),
+            look.label("Claude Code"),
+        );
+        assert!(harness.has(&row), "{}", look.name);
+        assert!(
+            harness.has(&look.label("This connection · 1 call · last 30 days · on this computer")),
+            "{}",
+            look.name
+        );
+        // Selected, it says what it came to, and that rows are not kept.
+        let request = harness
+            .painted_rect("SELECT id FROM books")
+            .expect("the request");
+        super::tests::click_at(&mut harness, request.center());
+        harness.settle();
+        let detail = look.label(&format!(
+            "Claude Code · {} · 3 rows, 2 columns returned · 4 ms · rows themselves are not logged",
+            crate::clock::moment(at),
+        ));
+        assert!(harness.has(&detail), "{}", look.name);
+        // Every connection's calls.
+        if look.terminal {
+            super::tests::type_key(&mut harness, egui::Key::L, "L");
+        } else {
+            harness.click("All connections");
+            harness.settle();
+        }
+        assert_eq!(log_asked(&harness).1, None, "{}", look.name);
+    }
+}
+
+#[test]
+fn omarchy_shows_the_log_with_l_and_the_agents_log_line() {
+    let (mut harness, tab) = connected(Look::omarchy());
+    harness.app.apply(Action::ShowAgents(tab));
+    let view = |harness: &Harness| {
+        harness
+            .app
+            .workspace(tab)
+            .unwrap()
+            .tabs
+            .iter()
+            .find_map(|open| match open {
+                crate::model::Tab::Agents(agents) => Some(agents.view),
+                _ => None,
+            })
+    };
+    // With the server off too: the log is the file's.
+    super::tests::type_key(&mut harness, egui::Key::L, "l");
+    assert_eq!(view(&harness), Some(crate::model::AgentsView::Log));
+    super::tests::type_key(&mut harness, egui::Key::L, "l");
+    assert_eq!(view(&harness), Some(crate::model::AgentsView::Activity));
+    // From another tab, by the prompt's line.
+    let query = harness.add_sql_tab(tab);
+    harness.app.apply(Action::ActivateTab { tab, id: query });
+    harness.app.workspace_mut(tab).unwrap().command = Some("agents log".into());
+    harness.app.apply(Action::RunCommand(tab));
+    assert!(front(&harness, tab));
+    assert_eq!(view(&harness), Some(crate::model::AgentsView::Log));
+}
+
+#[test]
+fn in_the_log_view_the_activitys_letters_do_nothing_and_l_is_named() {
+    // Worked out, as the backend's start does off the UI thread.
+    crate::endpoint::agent_command();
+    let (mut harness, tab) = connected(Look::omarchy());
+    allow(&mut harness, tab, AgentAccess::Read);
+    harness.app.apply(Action::ShowAgents(tab));
+    harness.settle();
+    // The status line names the tab's own key for the log, not the
+    // prompt's line.
+    assert!(harness.painted_rect(":agents log").is_none());
+    super::tests::type_key(&mut harness, egui::Key::L, "l");
+    harness.settle();
+    assert!(harness.painted_rect(":agents log").is_none());
+    // The server off: `o` turns nothing on from the log.
+    super::tests::type_key(&mut harness, egui::Key::O, "o");
+    assert!(!harness.app.settings.mcp_enabled);
+    // On: `y` copies nothing from the log.
+    harness.app.settings.mcp_enabled = true;
+    super::tests::type_key(&mut harness, egui::Key::Y, "y");
+    assert_eq!(crate::ui::toast::said(&harness.ctx), None);
+}
+
+#[test]
+fn a_call_selected_with_j_is_kept_in_sight() {
+    let (mut harness, tab) = connected(Look::omarchy());
+    harness.app.apply(Action::ShowAgentsView {
+        tab,
+        view: crate::model::AgentsView::Log,
+    });
+    let (asked, _) = log_asked(&harness);
+    let calls = (0..200)
+        .map(|n| tabletist_mcp::Logged {
+            id: 1_000 - n,
+            entry: tabletist_mcp::Entry {
+                at_ms: 1_791_880_982_000,
+                agent: "Claude Code".into(),
+                tool: "run_query".into(),
+                sql: Some(format!("SELECT {n}")),
+                result: "ok".into(),
+                ..tabletist_mcp::Entry::default()
+            },
+        })
+        .collect();
+    let page = tabletist_mcp::Page {
+        calls,
+        total: 200,
+        more: false,
+    };
+    said(
+        &mut harness,
+        AgentsEvent::LogRead {
+            asked,
+            page: Ok(page),
+        },
+    );
+    for _ in 0..61 {
+        super::tests::type_key(&mut harness, egui::Key::J, "j");
+    }
+    harness.settle();
+    harness.settle();
+    let row = harness
+        .painted_rect("SELECT 60")
+        .expect("the selected call");
+    assert!(row.bottom() <= harness.size.y, "{row:?}");
+}
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- clock:: ui::agents_tests`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0425]: cannot find function `hour` in module `crate::clock`
error[E0425]: cannot find function `time` in module `crate::clock`
error[E0425]: cannot find function `of_millis` in module `crate::clock`
error[E0425]: cannot find function `day` in module `crate::clock`
error[E0425]: cannot find function `moment` in module `crate::clock`
error[E0433]: cannot find type `TimeZone` in this scope
error: could not compile `tabletist` (lib test) due to 13 previous errors; 1 warning emitted
```

- [ ] **Step 3: The view, the clock, the keys**

````diff
diff --git a/src/app/editing.rs b/src/app/editing.rs
index c1b1534..03dc240 100644
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -1737,6 +1737,12 @@ impl App {
             (Typed::Command(Command::ShowAgents), _) if crate::endpoint::AVAILABLE => {
                 Action::ShowAgents(tab)
             }
+            (Typed::Command(Command::ShowAgentsLog), _) if crate::endpoint::AVAILABLE => {
+                Action::ShowAgentsView {
+                    tab,
+                    view: crate::model::AgentsView::Log,
+                }
+            }
             (Typed::Command(Command::SaveChanges), Some((id, _))) => Action::WriteEdits { tab, id },
             (Typed::Command(Command::DiscardChanges), Some((id, _))) => {
                 Action::DiscardEdits { tab, id }
diff --git a/src/clock.rs b/src/clock.rs
index 06bb51a..ca0053b 100644
--- a/src/clock.rs
+++ b/src/clock.rs
@@ -1,6 +1,45 @@
 //! The time by the person's clock, in their time zone: when an agent's
 //! call came, as the Agents tab says it.
 
+use std::time::SystemTime;
+
+use jiff::Timestamp;
+use jiff::tz::TimeZone;
+
+/// "10:43:02": a call of the feed, which came today.
+pub fn time(at: SystemTime) -> String {
+    format(at, &TimeZone::system(), "%H:%M:%S")
+}
+
+/// "10:38": when a session connected.
+pub fn hour(at: SystemTime) -> String {
+    format(at, &TimeZone::system(), "%H:%M")
+}
+
+/// "Oct 13 10:43": a call of the log, which keeps a month and more.
+pub fn day(at: SystemTime) -> String {
+    format(at, &TimeZone::system(), "%b %-d %H:%M")
+}
+
+/// "Oct 13 10:43:40": a call of the log on its own line.
+pub fn moment(at: SystemTime) -> String {
+    format(at, &TimeZone::system(), "%b %-d %H:%M:%S")
+}
+
+/// `at` in `zone`, as `pattern` writes it. Nothing for a time no clock
+/// shows.
+fn format(at: SystemTime, zone: &TimeZone, pattern: &str) -> String {
+    Timestamp::try_from(at)
+        .map(|at| at.to_zoned(zone.clone()).strftime(pattern).to_string())
+        .unwrap_or_default()
+}
+
+/// The time a log's entry keeps, milliseconds of Unix time, as a time.
+pub fn of_millis(at_ms: i64) -> SystemTime {
+    let millis = u64::try_from(at_ms).unwrap_or_default();
+    SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(millis)
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/src/keymap.rs b/src/keymap.rs
index 98ceae5..8513890 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -338,6 +338,8 @@ commands! {
     StopAgentSession,
     StopAgentsHere,
     ShowAgentsElsewhere,
+    ShowAgentsLog,
+    ToggleAgentsLogScope,
     ToggleSidebar,
     PaneLeft,
     PaneDown,
@@ -585,6 +587,12 @@ impl Command {
             C::ShowAgentsElsewhere => {
                 agents(info("Show where the selected agent works", "", Window))
             }
+            C::ShowAgentsLog => agents(info("Show the log, or what agents do now", "", Window)),
+            C::ToggleAgentsLogScope => agents(info(
+                "Show every connection's calls in the log, or this one's",
+                "",
+                Window,
+            )),
             C::ToggleSidebar => info("Toggle sidebar", "Toggle sidebar", Navigation),
             C::PaneLeft => info("Pane to the left", PANES, Navigation),
             C::PaneDown => info("Pane below", PANES, Navigation).reserved(),
@@ -979,6 +987,9 @@ pub const BINDINGS: &[Binding] = {
         bind(C::StopAgentSession, AGENTS, NONE, keys(&[k("S")])),
         bind(C::StopAgentsHere, AGENTS, NONE, keys(&[k("X")])),
         bind(C::ShowAgentsElsewhere, AGENTS, NONE, keys(&[k("enter")])),
+        bind(C::ShowAgentsLog, GLOBAL, NONE, keys(&[k(":agents log")])),
+        bind(C::ShowAgentsLog, AGENTS, NONE, keys(&[k("l")])),
+        bind(C::ToggleAgentsLogScope, AGENTS, NONE, keys(&[k("L")])),
         bind(
             C::ToggleSidebar,
             GLOBAL,
diff --git a/src/ui/agents.rs b/src/ui/agents.rs
index 57c296c..16ed742 100644
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -8,7 +8,7 @@ use egui::{Frame, Id, Margin, Rect, pos2, vec2};
 
 use crate::app::App;
 use crate::i18n::{gettext, ngettext};
-use crate::model::{Action, AgentsHere, ConnTabId};
+use crate::model::{Action, AgentsHere, AgentsView, ConnTabId};
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
 use crate::ui::{states, widgets};
@@ -26,6 +26,9 @@ pub fn copy_command(ctx: &egui::Context, locale: crate::i18n::Locale) {
     }
 }
 
+/// A segment of the macOS look's Activity and Log switch.
+const SEGMENT: f32 = 64.0;
+
 /// The height of the tab's line at its top, and of its status bar.
 const HEADER: f32 = 40.0;
 const STATUS: f32 = 33.0;
@@ -68,7 +71,26 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .collect();
     let calls: Vec<tabletist_mcp::Call> = app.agents.calls_on(&conn).cloned().collect();
     let selected = app.agents_selected(tab).map(|session| session.id.clone());
+    // Which view shows, and the log's page: borrowed, as it may be long.
+    let agents_tab = app.workspace(tab).and_then(|workspace| {
+        workspace.tabs.iter().find_map(|open| match open {
+            crate::model::Tab::Agents(agents) => Some(agents),
+            crate::model::Tab::Object(_) | crate::model::Tab::Sql(_) => None,
+        })
+    });
+    let view = agents_tab.map_or_else(AgentsView::default, |agents| agents.view);
+    let empty = crate::model::AgentsLog::default();
+    let log = agents_tab.map_or(&empty, |agents| &agents.log);
     let layout = app.layout();
+    // The tab's own `l`, not the prompt's `:agents log`.
+    let log_key = app.keymap.key_in(
+        layout,
+        crate::keymap::Command::ShowAgentsLog,
+        crate::keymap::Scope::Agents,
+    );
+    let scope_key = app
+        .keymap
+        .key(layout, crate::keymap::Command::ToggleAgentsLogScope);
     let show_key = app.keymap.key(layout, crate::keymap::Command::ShowAgents);
     let copy_key = app
         .keymap
@@ -134,12 +156,36 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     &mut actions,
                 ) - 16.0;
             }
+            // At the left in the macOS look: Activity and Log. The terminal
+            // look has `l`.
+            let mut left = full.left() + 16.0;
+            if !look.terminal {
+                let names = [gettext(locale, "Activity"), gettext(locale, "Log")];
+                let parts = [
+                    widgets::Segment::Text(&names[0]),
+                    widgets::Segment::Text(&names[1]),
+                ];
+                let chosen = usize::from(view == AgentsView::Log);
+                let place = Rect::from_min_size(
+                    pos2(left, full.center().y - 12.0),
+                    vec2(2.0 * SEGMENT, 24.0),
+                );
+                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
+                let segment = vec2(SEGMENT, 24.0);
+                if let Some(index) =
+                    widgets::segmented(&mut child, &parts, chosen, segment, &look, &palette)
+                {
+                    let view = [AgentsView::Activity, AgentsView::Log][index];
+                    actions.push(Action::ShowAgentsView { tab, view });
+                }
+                left = child.min_rect().right().max(place.right()) + 12.0;
+            }
             let on = look.label(&gettext(locale, "Agents on"));
             // A connection's name gives way at its end to a narrow tab.
             let strong = TextRole::pick(&look, TextRole::UiBodyStrong, TextRole::OBody);
             let body = widgets::body(&look);
             let around = body.width(ui.ctx(), look.faces, &format!("{on}  · {access}"));
-            let room = (right - full.left() - 16.0 - around).max(0.0);
+            let room = (right - left - around).max(0.0);
             let name = crate::ui::grid::ellipsize(&connection, room, false, |text| {
                 strong.width(ui.ctx(), look.faces, text)
             });
@@ -152,7 +198,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     &format!(" · {access}"),
                     palette.secondary,
                 );
-            widgets::paint_text(ui, full.left() + 16.0, full.center().y, line);
+            widgets::paint_text(ui, left, full.center().y, line);
             widgets::announce(ui, full, &format!("{on} {connection} · {access}"));
         });
     if !look.terminal {
@@ -162,6 +208,26 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .frame(Frame::new().fill(palette.window))
         .show(ui, |ui| {
             let rect = ui.max_rect();
+            // The log, whatever the state: past calls stay readable with
+            // the server off and access Off.
+            if view == AgentsView::Log {
+                let days = app.settings.mcp_log_days;
+                let shown = log_view(
+                    ui,
+                    rect,
+                    (log, &scope_key, days),
+                    (&look, &palette, locale),
+                );
+                match shown {
+                    Some(LogAsk::Scope) => actions.push(Action::ToggleAgentsLogScope(tab)),
+                    Some(LogAsk::More) => actions.push(Action::MoreAgentsLog(tab)),
+                    Some(LogAsk::Select(id)) => {
+                        actions.push(Action::SelectAgentsLogCall { tab, id });
+                    }
+                    None => {}
+                }
+                return;
+            }
             match &here {
                 AgentsHere::ServerOff => {
                     let notice = states::Notice {
@@ -188,7 +254,12 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                             "Access is Off for this connection, so agents don't see it at all.",
                         ),
                     };
-                    states::empty(ui, rect, &notice, Vec::new(), &look, &palette);
+                    let label = gettext(locale, "Show log");
+                    let button = states::key_button(&label, &log_key, &look);
+                    if states::empty(ui, rect, &notice, vec![button], &look, &palette).is_some() {
+                        let view = AgentsView::Log;
+                        actions.push(Action::ShowAgentsView { tab, view });
+                    }
                 }
                 AgentsHere::NoneConnected if look.terminal && !elsewhere.is_empty() => {
                     // Who works elsewhere is listed at the left, as with
@@ -200,12 +271,23 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                         .frame(Frame::new().fill(palette.window))
                         .show(ui, |ui| {
                             let rect = ui.max_rect();
-                            none_connected(ui, rect, (&connection, &name, &access), &copy_key, (&look, &palette, locale));
+                            let keys = (&copy_key, &log_key);
+                            let said = (&connection[..], &name[..], &access[..]);
+                            let style = (&look, &palette, locale);
+                            if none_connected(ui, rect, said, keys, style) {
+                                let view = AgentsView::Log;
+                                actions.push(Action::ShowAgentsView { tab, view });
+                            }
                         });
                 }
                 AgentsHere::NoneConnected => {
                     let name = app.workspace(tab).map(|w| w.name.clone()).unwrap_or_default();
-                    none_connected(ui, rect, (&connection, &name, &access), &copy_key, (&look, &palette, locale));
+                    let keys = (&copy_key, &log_key);
+                    let said = (&connection[..], &name[..], &access[..]);
+                    if none_connected(ui, rect, said, keys, (&look, &palette, locale)) {
+                        let view = AgentsView::Log;
+                        actions.push(Action::ShowAgentsView { tab, view });
+                    }
                 }
                 AgentsHere::Connected { .. } if look.terminal => {
                     let keys = (&stop_key, &show_elsewhere_key);
@@ -291,9 +373,9 @@ fn none_connected(
     ui: &mut egui::Ui,
     rect: Rect,
     (connection, name, access): (&str, &str, &str),
-    copy_key: &crate::keymap::Written,
+    (copy_key, log_key): (&crate::keymap::Written, &crate::keymap::Written),
     (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
-) {
+) -> bool {
     let ctx = ui.ctx().clone();
     let terminal = look.terminal;
     let room = (rect.width() - 2.0 * states::INSET).max(0.0);
@@ -356,7 +438,9 @@ fn none_connected(
         + 14.0
         + height
         + 18.0
-        + facts.height();
+        + facts.height()
+        + 18.0
+        + height;
     let (x, mut y) = if terminal {
         (rect.left() + states::INSET, rect.top() + states::INSET)
     } else {
@@ -407,6 +491,13 @@ fn none_connected(
         Rect::from_min_size(pos2(facts_left, y), facts.size()),
         &format!("{on_this}: {access_word} {access}"),
     );
+    // Under them, the calls agents made here before.
+    y += facts.height() + 18.0;
+    let label = gettext(locale, "Show log");
+    let show = states::key_button(&label, log_key, look);
+    let width = show.width(ui, look);
+    let at = Rect::from_min_size(pos2(left(width), y), vec2(width, height));
+    show.show_at(ui, at, look, palette).clicked()
 }
 
 /// A session that works elsewhere, as "Also:" names it: its agent, and
@@ -611,11 +702,9 @@ fn session_rows(
     stopped
 }
 
-/// Where a session was started, since when, and how many calls it made.
+/// Where a session was started, since when by the clock, and how many
+/// calls it made.
 fn session_facts(session: &tabletist_mcp::SessionInfo, locale: crate::i18n::Locale) -> String {
-    let since = std::time::SystemTime::now()
-        .duration_since(session.connected_at)
-        .unwrap_or_default();
     let mut facts = Vec::new();
     if let Some(project) = &session.project {
         facts.push(project.clone());
@@ -623,7 +712,7 @@ fn session_facts(session: &tabletist_mcp::SessionInfo, locale: crate::i18n::Loca
     facts.push(format!(
         "{} {}",
         gettext(locale, "connected"),
-        ago(since, locale)
+        crate::clock::hour(session.connected_at)
     ));
     facts.push(
         ngettext(
@@ -845,6 +934,365 @@ fn nav_row(
     *y += 48.0;
 }
 
+/// What the Log view asks of the app.
+enum LogAsk {
+    /// Every connection's calls, or this one's again.
+    Scope,
+    /// The page after the last call shown.
+    More,
+    /// A call, by its place in the log.
+    Select(i64),
+}
+
+/// The Log view's bar at its top, its line for the selected call, and a
+/// call's line.
+const LOG_BAR: f32 = 36.0;
+const LOG_DETAIL: f32 = 30.0;
+
+/// The Log view: whose calls (this connection's or every one's) and how
+/// many the log keeps, then the calls the newest first, and under them
+/// what the selected call came to. Rows are never logged: it says so.
+fn log_view(
+    ui: &mut egui::Ui,
+    rect: Rect,
+    (log, scope_key, days): (&crate::model::AgentsLog, &crate::keymap::Written, u32),
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+) -> Option<LogAsk> {
+    let mut ask = None;
+    let body = widgets::body(look);
+    let small = widgets::secondary(look);
+    let bar = Rect::from_min_size(rect.min, vec2(rect.width(), LOG_BAR));
+    widgets::hline(ui, bar.x_range(), bar.bottom() - 0.5, palette.border);
+    let scope = match log.all {
+        false => gettext(locale, "This connection"),
+        true => gettext(locale, "All connections"),
+    };
+    let count = ngettext(
+        locale,
+        "{n} call",
+        "{n} calls",
+        u32::try_from(log.total).unwrap_or(u32::MAX),
+    )
+    .replace("{n}", &log.total.to_string());
+    let kept =
+        ngettext(locale, "last {n} day", "last {n} days", days).replace("{n}", &days.to_string());
+    let count = format!("{count} · {kept} · {}", gettext(locale, "on this computer"));
+    if look.terminal {
+        // "log  this connection · 186 calls · … · L all connections".
+        let other = match log.all {
+            false => gettext(locale, "all connections"),
+            true => gettext(locale, "this connection"),
+        };
+        let line = Text::new(look)
+            .add(
+                TextRole::OBody,
+                &look.label(&gettext(locale, "Log")),
+                palette.text,
+            )
+            .space(body, "  ")
+            .add(
+                body,
+                &look.label(&format!("{scope} · {count} · ")),
+                palette.secondary,
+            )
+            .add(body, scope_key.as_str(), palette.accent)
+            .add(body, &format!(" {other}"), palette.secondary);
+        widgets::paint_text(ui, bar.left() + 14.0, bar.center().y, line);
+        widgets::announce(ui, bar, &look.label(&format!("{scope} · {count}")));
+    } else {
+        let names = [
+            gettext(locale, "This connection"),
+            gettext(locale, "All connections"),
+        ];
+        let parts = [
+            widgets::Segment::Text(&names[0]),
+            widgets::Segment::Text(&names[1]),
+        ];
+        let place = Rect::from_min_size(
+            pos2(bar.left() + 16.0, bar.center().y - 12.0),
+            vec2(2.0 * 120.0, 24.0),
+        );
+        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
+        let segment = vec2(120.0, 24.0);
+        let chosen = usize::from(log.all);
+        if widgets::segmented(&mut child, &parts, chosen, segment, look, palette)
+            .is_some_and(|index| index != chosen)
+        {
+            ask = Some(LogAsk::Scope);
+        }
+        let laid = Text::one(look, small, &count, palette.secondary).layout(ui.ctx());
+        laid.paint_left(
+            ui.painter(),
+            bar.right() - 16.0 - laid.width(),
+            bar.center().y,
+        );
+        widgets::announce(ui, bar, &format!("{scope} · {count}"));
+    }
+    // The selected call's line, at the bottom.
+    let detail = Rect::from_min_max(pos2(rect.left(), rect.bottom() - LOG_DETAIL), rect.max);
+    let chosen = log
+        .selected
+        .and_then(|id| log.calls.iter().find(|logged| logged.id == id));
+    if let Some(logged) = chosen {
+        ui.painter().rect_filled(detail, 0.0, palette.panel);
+        widgets::hline(ui, detail.x_range(), detail.top() + 0.5, palette.border);
+        let said = log_detail(&logged.entry, locale);
+        let said = look.label(&said);
+        let shown = crate::ui::grid::ellipsize(&said, detail.width() - 32.0, false, |text| {
+            small.width(ui.ctx(), look.faces, text)
+        });
+        widgets::paint_text(
+            ui,
+            detail.left() + 16.0,
+            detail.center().y,
+            Text::one(look, small, &shown, palette.secondary),
+        );
+        widgets::announce(ui, detail, &said);
+    }
+    let bottom = if chosen.is_some() {
+        detail.top()
+    } else {
+        rect.bottom()
+    };
+    let list = Rect::from_min_max(pos2(rect.left(), bar.bottom()), pos2(rect.right(), bottom));
+    let row = feed_row(look);
+    let empty = match (&log.failed, log.read, log.all) {
+        (Some(failed), _, _) => Some(format!(
+            "{} {failed}",
+            gettext(locale, "The log could not be read:")
+        )),
+        (None, false, _) => Some(gettext(locale, "Reading the log…").into_owned()),
+        (None, true, _) if !log.calls.is_empty() => None,
+        (None, true, false) => {
+            Some(gettext(locale, "No calls on this connection in the log.").into_owned())
+        }
+        (None, true, true) => Some(gettext(locale, "No calls in the log.").into_owned()),
+    };
+    if let Some(said) = empty {
+        let said = look.label(&said);
+        let line =
+            Rect::from_min_size(pos2(list.left(), list.top() + 8.0), vec2(list.width(), row));
+        widgets::paint_text(
+            ui,
+            line.left() + 16.0,
+            line.center().y,
+            Text::one(look, small, &said, palette.dim),
+        );
+        widgets::announce(ui, line, &said);
+        return ask;
+    }
+    // Time, agent, the connection under All connections, tool, request,
+    // rows, ms, result: the request takes what is left.
+    let room = (list.width() - 32.0).max(0.0);
+    let fixed = |width: f32, share: f32| width.min(room * share);
+    let mut widths = vec![fixed(110.0, 0.14), fixed(120.0, 0.14)];
+    if log.all {
+        widths.push(fixed(150.0, 0.16));
+    }
+    widths.extend([
+        fixed(140.0, 0.16),
+        0.0,
+        fixed(60.0, 0.07),
+        fixed(60.0, 0.07),
+        fixed(110.0, 0.14),
+    ]);
+    let request = widths.len() - 4;
+    widths[request] = (room - widths.iter().sum::<f32>()).max(0.0);
+    let mut names = vec![gettext(locale, "Time"), gettext(locale, "Agent")];
+    if log.all {
+        names.push(gettext(locale, "Connection"));
+    }
+    names.extend([
+        gettext(locale, "Tool"),
+        gettext(locale, "Request"),
+        gettext(locale, "Rows"),
+        gettext(locale, "ms"),
+        gettext(locale, "Result"),
+    ]);
+    let names: Vec<String> = names.iter().map(|name| look.label(name)).collect();
+    let mono = TextRole::pick(look, TextRole::GridCell, TextRole::OBody);
+    let cells = |ui: &egui::Ui, line: Rect, texts: &[(String, TextRole, egui::Color32)]| {
+        let mut x = line.left() + 16.0;
+        for ((text, role, color), width) in texts.iter().zip(&widths) {
+            let shown = crate::ui::grid::ellipsize(text, width - 10.0, false, |text| {
+                role.width(ui.ctx(), look.faces, text)
+            });
+            Text::one(look, *role, &shown, *color)
+                .layout(ui.ctx())
+                .paint_left(ui.painter(), x, line.center().y);
+            x += width;
+        }
+    };
+    let head = Rect::from_min_size(list.min, vec2(list.width(), row));
+    ui.painter().rect_filled(head, 0.0, palette.panel);
+    widgets::hline(ui, head.x_range(), head.bottom() - 0.5, palette.border);
+    let head_cells: Vec<_> = names
+        .iter()
+        .map(|name| (name.clone(), small, palette.secondary))
+        .collect();
+    cells(ui, head, &head_cells);
+    widgets::announce(ui, head, &names.join(" · "));
+    let calls = Rect::from_min_max(pos2(list.left(), head.bottom()), list.max);
+    let mut area = ui.new_child(egui::UiBuilder::new().max_rect(calls));
+    egui::ScrollArea::vertical()
+        .id_salt("agents-log")
+        .auto_shrink([false, false])
+        .show(&mut area, |ui| {
+            let height = row * (log.calls.len() + usize::from(log.more)) as f32;
+            let (rect, _) =
+                ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
+            let mut y = rect.top();
+            // A selection that moved since the last frame is brought into
+            // sight: `j` and `k` move it past the window's edge.
+            let seen_id = ui.id().with("selected-seen");
+            let seen: Option<i64> = ui.data(|data| data.get_temp(seen_id)).flatten();
+            for logged in &log.calls {
+                let entry = &logged.entry;
+                let line = Rect::from_min_size(pos2(rect.left(), y), vec2(rect.width(), row));
+                let clicked = ui
+                    .interact(
+                        line,
+                        ui.id().with(("log-call", logged.id)),
+                        egui::Sense::click(),
+                    )
+                    .clicked();
+                if clicked {
+                    ask = Some(LogAsk::Select(logged.id));
+                }
+                if log.selected == Some(logged.id) {
+                    if seen != log.selected {
+                        ui.scroll_to_rect(line, None);
+                    }
+                    ui.painter().rect_filled(line, 0.0, palette.selection);
+                    if look.terminal {
+                        let edge = Rect::from_min_size(line.min, vec2(2.0, line.height()));
+                        ui.painter().rect_filled(edge, 0.0, palette.accent);
+                    }
+                }
+                let at = crate::clock::of_millis(entry.at_ms);
+                let asked = entry
+                    .sql
+                    .as_deref()
+                    .or(entry.request.as_deref())
+                    .map(|asked| {
+                        let start: String = asked.chars().take(400).collect();
+                        start.split_whitespace().collect::<Vec<_>>().join(" ")
+                    })
+                    .unwrap_or_default();
+                let number = |value: Option<i64>| value.map(|n| n.to_string()).unwrap_or_default();
+                let (word, color) = log_result(&entry.result, palette, locale);
+                let mut texts = vec![
+                    (look.label(&crate::clock::day(at)), small, palette.secondary),
+                    (look.label(&entry.agent), body, palette.text),
+                ];
+                if log.all {
+                    let on = entry.connection.clone().unwrap_or_default();
+                    texts.push((on, body, palette.text));
+                }
+                texts.extend([
+                    (entry.tool.clone(), body, palette.text),
+                    (asked.clone(), mono, palette.text),
+                    (number(entry.rows), small, palette.secondary),
+                    (number(entry.took_ms), small, palette.secondary),
+                    (word.clone(), small, color),
+                ]);
+                cells(ui, line, &texts);
+                let said: Vec<&str> = texts.iter().map(|(text, ..)| text.as_str()).collect();
+                widgets::announce(ui, line, &said.join(" · "));
+                if !look.terminal {
+                    widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.border);
+                }
+                y += row;
+            }
+            ui.data_mut(|data| data.insert_temp(seen_id, log.selected));
+            if log.more {
+                let label = gettext(locale, "Show more");
+                let more = widgets::ButtonSpec::new(&label).link().padding(6.0);
+                let width = more.width(ui, look);
+                let at =
+                    Rect::from_min_size(pos2(rect.left() + 10.0, y + 2.0), vec2(width, row - 4.0));
+                if more.show_at(ui, at, look, palette).clicked() {
+                    ask = Some(LogAsk::More);
+                }
+            }
+        });
+    ask
+}
+
+/// How a call in the log ended, in a word, and its colour.
+fn log_result(
+    result: &str,
+    palette: &Palette,
+    locale: crate::i18n::Locale,
+) -> (String, egui::Color32) {
+    let color = match result {
+        "ok" => states::Tone::Success.color(palette),
+        "timeout" => states::Tone::Warning.color(palette),
+        "cancelled" | "stopped" => palette.secondary,
+        _ => states::Tone::Danger.color(palette),
+    };
+    (result_word(result, locale), color)
+}
+
+/// How a call ended, in a word: what the log keeps, said for a person.
+fn result_word(result: &str, locale: crate::i18n::Locale) -> String {
+    match result {
+        "ok" => gettext(locale, "ok"),
+        "timeout" => gettext(locale, "timed out"),
+        "cancelled" => gettext(locale, "cancelled"),
+        "stopped" => gettext(locale, "stopped"),
+        "error" => gettext(locale, "failed"),
+        _ => gettext(locale, "refused"),
+    }
+    .into_owned()
+}
+
+/// The selected call's line: who, when to the second, and what it came
+/// to. A read says its rows are not logged.
+fn log_detail(entry: &tabletist_mcp::Entry, locale: crate::i18n::Locale) -> String {
+    let at = crate::clock::of_millis(entry.at_ms);
+    let count = |one: &'static str, many: &'static str, n: i64| {
+        ngettext(locale, one, many, u32::try_from(n).unwrap_or(u32::MAX))
+            .replace("{n}", &n.to_string())
+    };
+    let mut said = vec![entry.agent.clone(), crate::clock::moment(at)];
+    if entry.result == "ok" {
+        match (entry.tool.as_str(), entry.rows) {
+            ("describe_schema", Some(tables)) => {
+                said.push(count("{n} table described", "{n} tables described", tables));
+            }
+            (_, Some(rows)) => {
+                let mut returned = count("{n} row", "{n} rows", rows);
+                if let Some(columns) = entry.columns {
+                    returned = format!(
+                        "{returned}, {}",
+                        count("{n} column", "{n} columns", columns)
+                    );
+                }
+                said.push(format!("{returned} {}", gettext(locale, "returned")));
+            }
+            (_, None) => {}
+        }
+        if let Some(took) = entry.took_ms {
+            said.push(format!("{took} ms"));
+        }
+        if entry.tool == "run_query" {
+            said.push(gettext(locale, "rows themselves are not logged").into_owned());
+        }
+    } else {
+        let word = result_word(&entry.result, locale);
+        let message = entry
+            .message
+            .as_deref()
+            .map(|message| message.split_whitespace().collect::<Vec<_>>().join(" "));
+        said.push(match message {
+            Some(message) => format!("{word}: {message}"),
+            None => word,
+        });
+    }
+    said.join(" · ")
+}
+
 /// A line of the feed, and its head.
 fn feed_row(look: &Look) -> f32 {
     if look.terminal { 26.0 } else { 30.0 }
@@ -898,7 +1346,6 @@ fn feed(
         cell(ui, *column, head.center().y, small, name, palette.secondary);
     }
     widgets::announce(ui, head, &names.join(" · "));
-    let now = std::time::SystemTime::now();
     let mut y = head.bottom();
     if calls.is_empty() {
         let said = look.label(&gettext(locale, "No calls on this connection yet."));
@@ -919,7 +1366,7 @@ fn feed(
     }
     for call in calls {
         let line = Rect::from_min_size(pos2(rect.left(), y), vec2(rect.width(), row));
-        let when = ago(now.duration_since(call.at).unwrap_or_default(), locale);
+        let when = crate::clock::time(call.at);
         let (ended, color) = ended(call, palette, locale);
         let asked = call.request.as_deref().unwrap_or_default();
         let center = line.center().y;
@@ -975,43 +1422,18 @@ fn ended(
         };
         return (said, states::Tone::Success.color(palette));
     }
-    let (word, color) = match call.result.as_str() {
-        "timeout" => (
-            gettext(locale, "timed out"),
-            states::Tone::Warning.color(palette),
-        ),
-        "cancelled" => (gettext(locale, "cancelled"), palette.secondary),
-        "stopped" => (gettext(locale, "stopped"), palette.secondary),
-        "error" => (
-            gettext(locale, "failed"),
-            states::Tone::Danger.color(palette),
-        ),
-        _ => (
-            gettext(locale, "refused"),
-            states::Tone::Danger.color(palette),
-        ),
-    };
+    let (word, color) = log_result(&call.result, palette, locale);
     let said = match &call.message {
         // On one line: a database's own words may run over several.
         Some(message) => format!(
             "{word} · {}",
             message.split_whitespace().collect::<Vec<_>>().join(" ")
         ),
-        None => word.into_owned(),
+        None => word,
     };
     (said, color)
 }
 
-/// How long ago, as the person reads it: "just now", "4 min", "2 h".
-pub fn ago(since: std::time::Duration, locale: crate::i18n::Locale) -> String {
-    let minutes = since.as_secs() / 60;
-    match minutes {
-        0 => gettext(locale, "just now").into_owned(),
-        1..60 => format!("{minutes} {}", gettext(locale, "min ago")),
-        _ => format!("{} {}", minutes / 60, gettext(locale, "h ago")),
-    }
-}
-
 #[cfg(test)]
 mod tests {
     /// Where agents cannot reach the app, nothing of them shows: no entry,
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 0285632..5b28dfa 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -1679,9 +1679,8 @@ fn held_back(command: Command) -> bool {
 /// with the command to copy.
 fn agents_shows(app: &App, tab: ConnTabId, server_off: bool) -> bool {
     use crate::model::AgentsHere;
-    let front = app
-        .workspace(tab)
-        .is_some_and(crate::model::Workspace::agents_in_front);
+    // Its Activity view: the Log view shows neither Turn on nor Copy.
+    let front = agents_view(app, tab) == Some(crate::model::AgentsView::Activity);
     let here = app.agents_here(tab);
     front
         && match server_off {
@@ -1698,7 +1697,10 @@ fn agents_shows(app: &App, tab: ConnTabId, server_off: bool) -> bool {
 /// or the connection is Off, whose tabs list none.
 fn agents_lists(app: &App, tab: ConnTabId) -> bool {
     use crate::model::AgentsHere;
-    let Some(workspace) = app.workspace(tab).filter(|w| w.agents_in_front()) else {
+    if agents_view(app, tab) != Some(crate::model::AgentsView::Activity) {
+        return false;
+    }
+    let Some(workspace) = app.workspace(tab) else {
         return false;
     };
     let listing = matches!(
@@ -1708,12 +1710,21 @@ fn agents_lists(app: &App, tab: ConnTabId) -> bool {
     listing && !app.agents.listed(&workspace.conn_id).is_empty()
 }
 
+/// The view of the Agents tab of `tab`, while the tab is in front.
+fn agents_view(app: &App, tab: ConnTabId) -> Option<crate::model::AgentsView> {
+    let workspace = app
+        .workspace(tab)
+        .filter(|workspace| workspace.agents_in_front())?;
+    workspace.tabs.iter().find_map(|open| match open {
+        crate::model::Tab::Agents(agents) => Some(agents.view),
+        crate::model::Tab::Object(_) | crate::model::Tab::Sql(_) => None,
+    })
+}
+
 /// Whether the Agents tab of `tab` is in front with agents here: what Stop
 /// agents here needs.
 fn agents_here(app: &App, tab: ConnTabId) -> bool {
-    let front = app
-        .workspace(tab)
-        .is_some_and(crate::model::Workspace::agents_in_front);
+    let front = agents_view(app, tab) == Some(crate::model::AgentsView::Activity);
     front
         && matches!(
             app.agents_here(tab),
@@ -2242,6 +2253,25 @@ fn letters(
             (Some(Command::TurnAgentsOn), _) if agents_shows(app, tab, true) => {
                 actions.push(Action::TurnAgentsOn);
             }
+            (Some(Command::ShowAgentsLog), _) if agents_view(app, tab).is_some() => {
+                // `l` shows the log, and the activity again.
+                let view = match agents_view(app, tab) {
+                    Some(crate::model::AgentsView::Log) => crate::model::AgentsView::Activity,
+                    _ => crate::model::AgentsView::Log,
+                };
+                actions.push(Action::ShowAgentsView { tab, view });
+            }
+            (Some(Command::ToggleAgentsLogScope), _)
+                if agents_view(app, tab) == Some(crate::model::AgentsView::Log) =>
+            {
+                actions.push(Action::ToggleAgentsLogScope(tab));
+            }
+            (Some(Command::MoveInAgentSessions), _)
+                if agents_view(app, tab) == Some(crate::model::AgentsView::Log) =>
+            {
+                let step = if char == 'k' { -1 } else { 1 };
+                actions.push(Action::MoveAgentsLogSelection { tab, step });
+            }
             (Some(Command::MoveInAgentSessions), _) if agents_lists(app, tab) => {
                 let step = if char == 'k' { -1 } else { 1 };
                 actions.push(Action::MoveAgentSelection { tab, step });
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index 53d5bc5..8852e74 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2039,6 +2039,24 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .workspace(tab)
         .filter(|workspace| workspace.agents_in_front())
         .and_then(|_| app.agents_here(tab));
+    // The tab's own `l`, not the prompt's `:agents log`.
+    let (agents_log_key, agents_scope_key) = (
+        app.keymap.key_in(
+            app.layout(),
+            Command::ShowAgentsLog,
+            crate::keymap::Scope::Agents,
+        ),
+        key(Command::ToggleAgentsLogScope),
+    );
+    // Its Log view, while it shows, and whether every connection's.
+    let agents_log = app.workspace(tab).and_then(|workspace| {
+        workspace.tabs.iter().find_map(|open| match open {
+            crate::model::Tab::Agents(agents) if agents.view == crate::model::AgentsView::Log => {
+                Some(agents.log.all)
+            }
+            _ => None,
+        })
+    });
     // Whether its list has sessions, elsewhere ones too.
     let agents_listed = app
         .workspace(tab)
@@ -2257,6 +2275,25 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 ],
                 Some(crate::model::AgentsHere::OffHere) | None => Vec::new(),
             };
+            let log_words = [
+                gettext(locale, "log"),
+                gettext(locale, "activity"),
+                gettext(locale, "calls"),
+                gettext(locale, "all connections"),
+                gettext(locale, "this connection"),
+            ];
+            match agents_log {
+                // The Log view's keys in the place of the activity's.
+                Some(all) => {
+                    let scope = if all { &*log_words[4] } else { &*log_words[3] };
+                    agents_hints = vec![
+                        (&agents_log_key, &*log_words[1], true),
+                        (&agents_scope_key, scope, true),
+                        (&agents_move_key, &*log_words[2], true),
+                    ];
+                }
+                None => agents_hints.push((&agents_log_key, &*log_words[0], true)),
+            }
             agents_hints.push((&sidebar_key, "tables", true));
             if agents.is_some() {
                 let left = rect.left() + 12.0;
````

- [ ] **Step 4: Run the tests**

Run: the same, then `cargo test --locked --lib keymap::`, `cargo test --locked --test keys` and `docs/bin/check`

Expected: PASS, and the website's check ends `HTML-Proofer finished successfully.`

```text
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 2480 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 2482 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Look at it.** Render the Log view in both looks, by eye, in a throwaway scene in `src/shots.rs`: four calls, one selected. Remove it before the commit. No picture is committed.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add Cargo.toml Cargo.lock src docs/_guide/omarchy.md
git commit -m "Show the log of agents' calls in the Agents tab, with times by the clock"
```

---

### Task 4: An agent that connects is told of

- `[mcp] notify`, `true` unless set otherwise.
- A session is told of once, when its agent has named itself (`AgentsState::told`, which forgets the sessions that ended). On Linux `AgentsCommand::Notify`: the backend shows "Claude Code connected to Tabletist" over D-Bus with `zbus` (one line in `Cargo.toml` and one in `Cargo.lock`), with Review, and says `AgentsEvent::Reviewed` when it is clicked; it waits an hour at most. On macOS `App::window_asks` asks the frame for the Dock's bounce.
- Review brings Tabletist forward on the Agents tab of the window in front; with Connections in front, of the last window open; on Connections where none is open (question 22).
- The reference page names `notify`.

**Files:**
- Create: `src/backend/agents/notify.rs`
- Modify: `Cargo.toml`, `Cargo.lock`, `src/settings.rs`, `src/model.rs`, `src/app.rs`, `src/backend/agents.rs`, `docs/_reference/settings-and-files.md`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/Cargo.lock b/Cargo.lock
index de591ac..a945232 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -5711,6 +5711,7 @@ dependencies = [
  "windows-native-keyring-store",
  "winit",
  "winresource",
+ "zbus",
  "zbus-secret-service-keyring-store",
 ]
 
diff --git a/Cargo.toml b/Cargo.toml
index 6dc95bd..cb9092d 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -113,6 +113,10 @@ notify = "8.2"
 jiff = "0.2"
 
 [target.'cfg(target_os = "linux")'.dependencies]
+# Notifications on the desktop (mako on Omarchy), for an agent that
+# connects. The version and the features the keyring store already brings
+# in: nothing new is built.
+zbus = "5"
 # The desktop's Secret Service (GNOME Keyring, KWallet) over D-Bus.
 zbus-secret-service-keyring-store = { version = "1.0.1", features = ["rt-tokio-crypto-rust"] }
 # The Secret Service itself, which the store above is built on, with the
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 9b3b392..c4dde1c 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -45,6 +45,7 @@ sql_timeout_secs = 30
 [mcp]
 enabled = false
 log_days = 30
+notify = true
 ```
 
 | Key | Values | Default |
@@ -59,8 +60,9 @@ log_days = 30
 | `appearance.theme` | The name of a palette file in the `themes` folder | Not set |
 | `mcp.enabled` | `true` lets agents reach the connections you open to them, through `tabletist mcp`. Linux and macOS | `false` |
 | `mcp.log_days` | 1 to 3650 days. How long the log of agents' calls keeps a call | `30` |
+| `mcp.notify` | `true` tells you when an agent connects: a notification on Linux, the Dock icon bouncing on macOS | `true` |
 
-`show_system_schemas`, `theme` and the two `mcp` keys are set only in the
+`show_system_schemas`, `theme` and the three `mcp` keys are set only in the
 file. The SQL
 editor's **Limit** and **Timeout** menus set the two `editor` keys.
 
diff --git a/src/app.rs b/src/app.rs
index 3f680f6..eadd440 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -6485,6 +6485,61 @@ mod tests {
         assert_ne!(log_asked(&app).unwrap().2, asked);
     }
 
+    #[cfg(unix)]
+    #[test]
+    fn an_agent_that_connects_is_told_of_once_and_review_shows_the_tab() {
+        let (mut app, _dir) = app();
+        let (tab, _, _) = connect(&mut app);
+        app.settings.mcp_enabled = true;
+        app.check_agents();
+        app.apply(Action::KeepAgentChanges(app.agents_held.clone()));
+        let notified = |app: &App| {
+            agents_sent(app)
+                .into_iter()
+                .filter_map(|sent| match sent {
+                    AgentsCommand::Notify { summary, .. } => Some(summary.clone()),
+                    _ => None,
+                })
+                .collect::<Vec<_>>()
+        };
+        let mut session = agent_session("one", &[]);
+        session.agent = None;
+        // Not before its agent is named.
+        app.apply_event(Event::Agents(AgentsEvent::Sessions(vec![session.clone()])));
+        assert_eq!(notified(&app), Vec::<String>::new());
+        session.agent = Some("Claude Code".into());
+        for _ in 0..2 {
+            app.apply_event(Event::Agents(AgentsEvent::Sessions(vec![session.clone()])));
+        }
+        assert_eq!(notified(&app), ["Claude Code connected to Tabletist"]);
+        assert_eq!(
+            app.window_asks
+                .contains(&egui::ViewportCommand::RequestUserAttention(
+                    egui::UserAttentionType::Informational
+                )),
+            cfg!(target_os = "macos")
+        );
+        // Off in the settings: nobody is told.
+        app.settings.mcp_notify = false;
+        let other = agent_session("two", &[]);
+        app.apply_event(Event::Agents(AgentsEvent::Sessions(vec![session, other])));
+        assert_eq!(notified(&app).len(), 1);
+        // Review: the app comes forward, on the Agents tab of the window
+        // in front.
+        app.window_asks.clear();
+        app.apply_event(Event::Agents(AgentsEvent::Reviewed));
+        assert_eq!(app.window_asks, [egui::ViewportCommand::Focus]);
+        assert!(app.workspace(tab).unwrap().agents_in_front());
+        // With Connections in front, the window there is.
+        app.apply(Action::ShowConnections);
+        app.apply_event(Event::Agents(AgentsEvent::Reviewed));
+        assert_eq!(app.active_tab_id(), tab);
+        // With none open, Connections.
+        app.apply(Action::Disconnect(tab));
+        app.apply_event(Event::Agents(AgentsEvent::Reviewed));
+        assert!(app.workspace(app.active_tab_id()).is_none());
+    }
+
     #[test]
     fn agents_are_told_of_each_change_to_what_they_may_use() {
         let (mut app, _dir) = app();
diff --git a/src/settings.rs b/src/settings.rs
index f65cf06..4ed9fbd 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -775,6 +775,7 @@ sql_timeout_secs = 30  # 0 waits forever
 [mcp]
 enabled  = false
 log_days = 30
+notify   = true
 "
         );
     }
@@ -823,6 +824,7 @@ log_days = 30
             sql_timeout_secs: None,
             mcp_enabled: true,
             mcp_log_days: 7,
+            mcp_notify: false,
         };
         let text = settings.to_toml();
         let loaded = Settings::from_toml(&text);
@@ -849,6 +851,7 @@ log_days = 30
                 (Key::SqlTimeoutSecs, 13),
                 (Key::McpEnabled, 16),
                 (Key::McpLogDays, 17),
+                (Key::McpNotify, 18),
             ]
         );
     }
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- app::tests::an_agent_that_connects settings::`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0599]: no variant named `Notify` found for enum `backend::agents::AgentsCommand`
error[E0609]: no field `window_asks` on type `app::App`
error[E0609]: no field `mcp_notify` on type `settings::Settings`
error[E0599]: no variant, associated function, or constant named `Reviewed` found for enum `backend::agents::AgentsEvent` in the current scope
error[E0599]: no variant, associated function, or constant named `McpNotify` found for enum `settings::Key` in the current scope
error: could not compile `tabletist` (lib test) due to 10 previous errors
```

- [ ] **Step 3: Tell of it**

````diff
diff --git a/src/app.rs b/src/app.rs
index eadd440..e209828 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -119,6 +119,9 @@ pub struct App {
     theme_changed: bool,
     /// The window title last sent, so it is sent only when it changes.
     window_title: String,
+    /// What the next frame asks of the window: to come forward, or the
+    /// Dock's bounce.
+    pub window_asks: Vec<egui::ViewportCommand>,
     /// SQL editors closed since the last frame: what egui keeps for each
     /// (its undo history holds copies of the script) is dropped once a
     /// frame has the `egui::Context` to drop it from.
@@ -201,6 +204,7 @@ impl App {
             follow_desktop: false,
             theme_changed: false,
             window_title: "Tabletist".into(),
+            window_asks: Vec::new(),
             closed_editors: Vec::new(),
             lookup_delay: crate::key_picker::DELAY,
             lookups: Vec::new(),
@@ -2926,6 +2930,7 @@ impl App {
                 AgentsEvent::Started { kept_aside: None } => {}
                 // What a server said as it stopped is not the server now.
                 AgentsEvent::Sessions(open) if self.agents_on() => {
+                    self.tell_of_sessions(&open);
                     self.agents.sessions = open;
                 }
                 AgentsEvent::Sessions(_) => {}
@@ -2935,6 +2940,7 @@ impl App {
                     self.log_called(on.as_deref());
                 }
                 AgentsEvent::LogRead { asked, page } => self.agents_log_read(asked, page),
+                AgentsEvent::Reviewed => self.reviewed(),
                 AgentsEvent::Started {
                     kept_aside: Some(path),
                 } => {
@@ -3892,6 +3898,63 @@ impl App {
         }
     }
 
+    /// Tells the person of each session that opened with its agent named,
+    /// once: a notification on Linux, the Dock's bounce on macOS, while
+    /// `[mcp] notify` is on.
+    fn tell_of_sessions(&mut self, open: &[tabletist_mcp::SessionInfo]) {
+        self.agents
+            .told
+            .retain(|id| open.iter().any(|session| session.id == *id));
+        let mut new = Vec::new();
+        for session in open {
+            if session.agent.is_some() && self.agents.told.insert(session.id.clone()) {
+                new.push(crate::model::agent_name(session));
+            }
+        }
+        if new.is_empty() || !self.settings.mcp_notify {
+            return;
+        }
+        if cfg!(target_os = "macos") {
+            self.window_asks
+                .push(egui::ViewportCommand::RequestUserAttention(
+                    egui::UserAttentionType::Informational,
+                ));
+        }
+        for agent in new {
+            let summary = crate::i18n::gettext(self.locale, "{agent} connected to Tabletist")
+                .replace("{agent}", &agent);
+            let body = crate::i18n::gettext(
+                self.locale,
+                "It can use the connections you allow agents. Review shows what it does.",
+            )
+            .into_owned();
+            self.backend
+                .send(Command::Agents(AgentsCommand::Notify { summary, body }));
+        }
+    }
+
+    /// The person clicked Review: Tabletist comes forward, on the Agents
+    /// tab of the window in front, or on Connections where none is open
+    /// (question 22). A new agent is on no connection yet: it is under
+    /// "Also:" there.
+    fn reviewed(&mut self) {
+        self.window_asks.push(egui::ViewportCommand::Focus);
+        // The window in front; with Connections in front, the last window
+        // there is.
+        let front = self.active_tab_id();
+        let tab = match self.workspace(front) {
+            Some(_) => Some(front),
+            None => self.open_connections().last().map(|(tab, _)| tab),
+        };
+        match tab {
+            Some(tab) => {
+                self.apply(Action::ActivateConnTab(tab));
+                self.apply(Action::ShowAgents(tab));
+            }
+            None => self.apply(Action::ShowConnections),
+        }
+    }
+
     /// Stops one agent's session. Its line goes when the backend says the
     /// sessions again.
     fn stop_agent_session(&mut self, id: String) {
@@ -4797,6 +4860,9 @@ impl App {
                 .send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
             self.window_title = title;
         }
+        for ask in self.window_asks.drain(..) {
+            ui.ctx().send_viewport_cmd(ask);
+        }
         // Asked every frame until the window goes: a window that is
         // closing has nothing else to do.
         if self.closing {
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 61d7a77..540a3d9 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -29,6 +29,8 @@ use crate::secrets::{Keyring, password_account, ssh_account};
 
 #[cfg(unix)]
 mod listen;
+#[cfg(target_os = "linux")]
+mod notify;
 
 /// A connection agents may use, as the server is given it: what opens it,
 /// and what an agent calls it. Its password is not here: the gateway reads
@@ -157,6 +159,9 @@ pub enum AgentsCommand {
         before: Option<i64>,
         asked: u64,
     },
+    /// Tells the person an agent connected, where the desktop takes a
+    /// notification (Linux). Its Review is [`AgentsEvent::Reviewed`].
+    Notify { summary: String, body: String },
     /// Stops it: every session ends, and the connections are closed.
     /// Answered with [`AgentsEvent::Stopped`].
     Stop,
@@ -185,6 +190,8 @@ pub enum AgentsEvent {
         asked: u64,
         page: Result<tabletist_mcp::Page, String>,
     },
+    /// The person clicked Review on a notification.
+    Reviewed,
 }
 
 /// How many calls a page of the log holds.
@@ -264,6 +271,12 @@ impl Agents {
                     outbox.emit(super::Event::Agents(AgentsEvent::LogRead { asked, page }));
                 });
             }
+            AgentsCommand::Notify { summary, body } => {
+                #[cfg(target_os = "linux")]
+                tokio::spawn(notify::notify(summary, body, outbox.clone()));
+                #[cfg(not(target_os = "linux"))]
+                let _ = (summary, body);
+            }
             AgentsCommand::StopSession(id) => {
                 if let Some(running) = lock(&self.running).as_ref()
                     && running.server.stop_session(&id)
diff --git a/src/backend/agents/notify.rs b/src/backend/agents/notify.rs
new file mode 100644
index 0000000..269fc78
--- /dev/null
+++ b/src/backend/agents/notify.rs
@@ -0,0 +1,98 @@
+//! A notification on the Linux desktop, over D-Bus
+//! (`org.freedesktop.Notifications`): what mako shows on Omarchy. One
+//! action, Review, which the app answers by showing the Agents tab.
+
+use std::collections::HashMap;
+use std::pin::Pin;
+use std::time::Duration;
+
+use zbus::export::futures_core::Stream;
+
+use super::AgentsEvent;
+use crate::backend::Outbox;
+
+/// How long a notification's Review is waited for.
+const WAIT: Duration = Duration::from_secs(60 * 60);
+
+/// The action's key the server sends back when Review is clicked. A
+/// server that shows no buttons sends it for a click on the notification.
+const REVIEW: &str = "default";
+
+/// Shows `summary` and `body`, and says when Review is clicked, for an
+/// hour at most. Nothing where no notification server answers: the
+/// sidebar's entry says who is here all the same.
+pub(super) async fn notify(summary: String, body: String, outbox: Outbox) {
+    if let Err(error) = shown(&summary, &body, &outbox).await {
+        // Not its words: they name the agent.
+        log::warn!("a notification could not be shown: {error}");
+    }
+}
+
+async fn shown(summary: &str, body: &str, outbox: &Outbox) -> zbus::Result<()> {
+    let connection = zbus::Connection::session().await?;
+    let proxy = zbus::Proxy::new(
+        &connection,
+        "org.freedesktop.Notifications",
+        "/org/freedesktop/Notifications",
+        "org.freedesktop.Notifications",
+    )
+    .await?;
+    // Before it is shown: a click that comes at once is not missed.
+    let mut signals = zbus::MessageStream::for_match_rule(
+        zbus::MatchRule::builder()
+            .msg_type(zbus::message::Type::Signal)
+            .interface("org.freedesktop.Notifications")?
+            .build(),
+        &connection,
+        None,
+    )
+    .await?;
+    let hints: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
+    let actions = [REVIEW, "Review"];
+    let id: u32 = proxy
+        .call(
+            "Notify",
+            &(
+                "Tabletist",
+                0u32,
+                "",
+                summary,
+                body,
+                &actions[..],
+                hints,
+                -1i32,
+            ),
+        )
+        .await?;
+    let answered = async {
+        while let Some(message) = next(&mut signals).await {
+            let message = message?;
+            let header = message.header();
+            match header.member().map(|member| member.as_str()) {
+                Some("ActionInvoked") => {
+                    let (of, action): (u32, String) = message.body().deserialize()?;
+                    if of == id {
+                        if action == REVIEW {
+                            outbox.emit(crate::backend::Event::Agents(AgentsEvent::Reviewed));
+                        }
+                        return Ok(());
+                    }
+                }
+                Some("NotificationClosed") => {
+                    let (of, _why): (u32, u32) = message.body().deserialize()?;
+                    if of == id {
+                        return Ok(());
+                    }
+                }
+                _ => {}
+            }
+        }
+        Ok(())
+    };
+    tokio::time::timeout(WAIT, answered).await.unwrap_or(Ok(()))
+}
+
+/// The stream's next item.
+async fn next<S: Stream + Unpin>(stream: &mut S) -> Option<S::Item> {
+    std::future::poll_fn(|context| Pin::new(&mut *stream).poll_next(context)).await
+}
diff --git a/src/model.rs b/src/model.rs
index ba05728..b0df033 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2645,6 +2645,9 @@ pub struct AgentsState {
     /// The last [`FEED_KEPT`] calls, on every connection. The log has
     /// them all.
     pub feed: std::collections::VecDeque<tabletist_mcp::Call>,
+    /// The open sessions the person was told of: one notification for
+    /// each session, not for each call.
+    pub told: std::collections::BTreeSet<String>,
 }
 
 /// How many calls the app keeps for the feed.
diff --git a/src/settings.rs b/src/settings.rs
index 4ed9fbd..c72da64 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -64,6 +64,10 @@ pub struct Settings {
     pub mcp_enabled: bool,
     /// Days the log of agents' calls keeps a call.
     pub mcp_log_days: u32,
+    /// Whether an agent that connects is told of: a notification, or on
+    /// macOS the Dock's bounce. The Agents entry says who is here either
+    /// way.
+    pub mcp_notify: bool,
 }
 
 /// A key of the settings file.
@@ -78,12 +82,13 @@ pub enum Key {
     SqlTimeoutSecs,
     McpEnabled,
     McpLogDays,
+    McpNotify,
     Theme,
 }
 
 impl Key {
     /// In the order the file has them: a table's keys together.
-    pub const ALL: [Key; 10] = [
+    pub const ALL: [Key; 11] = [
         Self::PageSize,
         Self::Timestamps,
         Self::GroupDigits,
@@ -93,6 +98,7 @@ impl Key {
         Self::SqlTimeoutSecs,
         Self::McpEnabled,
         Self::McpLogDays,
+        Self::McpNotify,
         Self::Theme,
     ];
 
@@ -108,6 +114,7 @@ impl Key {
             Self::SqlTimeoutSecs => ("editor", "sql_timeout_secs"),
             Self::McpEnabled => ("mcp", "enabled"),
             Self::McpLogDays => ("mcp", "log_days"),
+            Self::McpNotify => ("mcp", "notify"),
             Self::Theme => ("appearance", "theme"),
         }
     }
@@ -487,6 +494,7 @@ impl Settings {
             Key::SqlTimeoutSecs => self.sql_timeout_secs.unwrap_or(0).to_string(),
             Key::McpEnabled => self.mcp_enabled.to_string(),
             Key::McpLogDays => self.mcp_log_days.to_string(),
+            Key::McpNotify => self.mcp_notify.to_string(),
             Key::Theme => toml_string(self.custom_theme.as_deref()?),
         })
     }
@@ -562,6 +570,7 @@ impl Settings {
             Key::SqlTimeoutSecs => number.map(|number| self.sql_timeout_secs = Some(number)),
             Key::McpEnabled => flag.map(|flag| self.mcp_enabled = flag),
             Key::McpLogDays => number.map(|number| self.mcp_log_days = number),
+            Key::McpNotify => flag.map(|flag| self.mcp_notify = flag),
             Key::Theme => text.map(|name| self.custom_theme = Some(name.to_owned())),
         }
         .is_some()
@@ -718,6 +727,7 @@ impl Default for Settings {
             sql_timeout_secs: Some(30),
             mcp_enabled: false,
             mcp_log_days: 30,
+            mcp_notify: true,
         }
     }
 }
````

- [ ] **Step 4: Run the tests**

Run: the same, then `docs/bin/check`

Expected: PASS, and the website's check ends `HTML-Proofer finished successfully.`

```text
test result: ok. 53 passed; 0 failed; 0 ignored; 0 measured; 2453 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add Cargo.toml Cargo.lock src docs/_reference/settings-and-files.md
git commit -m "Tell the person when an agent connects, and show it on Review"
```

---

### Task 5: The master plan says what was built

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: Correct it**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index be7b5f5..d5f3276 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -1678,9 +1678,10 @@ the boards, each comes with what it needs:
 sessions with **Stop session**, "Also:" with **Show**, **Stop agents
 here**, and the feed. Where it draws otherwise than the boards:
 
-- The feed's time says how long ago ("just now", "4 min ago"), as the
-  session's line does: the clock comes with the Log view, which needs
-  dates (the fourth run's second part).
+- The feed's time said how long ago ("just now", "4 min ago") until the
+  fourth run's second part brought the clock (`2026-10-14-mcp-server-1b-4b-log.md`): it says the time
+  now ("10:43:02"), and the session's line when it connected ("connected
+  10:38"), as the boards draw them.
 - The feed is the last 300 calls the app heard of since it started, on
   every connection, each window showing those on its own. A call's
   statement is kept on one line and cut at 200 characters; its values are
@@ -1707,6 +1708,33 @@ here**, and the feed. Where it draws otherwise than the boards:
   which offers it, comes with the fifth run, and with Off and Read alone
   the level is always Read.
 
+**As built in the second part of Phase 1b's fourth run** (`2026-10-14-mcp-server-1b-4b-log.md`): the Log
+view, **Show log**, the clock, and the notification for an agent that
+connects. Where it draws otherwise than the boards:
+
+- The log is the tab's Log view in both looks. Omarchy's board puts it
+  under the feed on one screen; there it is a view of its own too, which
+  `l` shows and leaves, and `:agents log` opens.
+- macOS chooses This connection or All connections with a segmented
+  control: the board's menu holds the agent, result and search filters
+  too, which come with Phase 3b.
+- The list is Time, Agent, the connection under All connections, Tool,
+  Request, Rows, ms and Result, 200 calls at a time with **Show more**.
+  The selected call's line under it ("Claude Code · Oct 13 10:43:40 · 3
+  rows, 2 columns returned · 4 ms · rows themselves are not logged") is
+  Phase 1b's detail; the pane is 3b's.
+- The log is read from its file as it is: with the server off nothing is
+  made and a damaged file is not set aside. A write a crash left half
+  done is rolled back as SQLite does on any open, which a read-only open
+  cannot do, so the file is opened to write and never written. A page on
+  the screen is read again when a call on its connection ends, and once
+  more after a read that was out when one ended.
+- **Show log** is in the Off here and nobody here states. The Server off
+  state keeps **Turn on** alone until Settings comes (the fifth run); its
+  log is the switch's, or `l`.
+- The clock is jiff's, in the system's time zone: dates in the log, and
+  times in the feed and on a session's line.
+
 | State | Title | Body | Actions |
 |---|---|---|---|
 | Allowed, no agents yet | No agents connected to Bookshop · dev | "Agents like Claude Code can read this database and propose changes you approve here. Connect one with this command, then ask it about Bookshop." Then the command with **Copy**, and a box "On this connection": access, hidden tables, masked columns, last agent | **Other agents…**, **Show log** |
@@ -1762,6 +1790,15 @@ and `Event::Mcp` (`src/backend.rs:52`, `:209`).
 - An agent connecting is one notification for each session, not for each
   call.
 
+**As built** (`2026-10-14-mcp-server-1b-4b-log.md`), for an agent that connects: on Linux a
+notification over D-Bus with `zbus` ("Claude Code connected to
+Tabletist", and "It can use the connections you allow agents. Review
+shows what it does."), whose Review brings Tabletist forward on the
+Agents tab of the window in front (with Connections in front, of the last
+window open), or on Connections where no window is open; on macOS the
+Dock's bounce. A session is told of once, when its agent has named itself.
+`[mcp] notify` is `true` unless set otherwise.
+
 ### D.5 The log
 
 Boards: `macOS – Agents tab, log`, and the lower half of the Omarchy board.
@@ -2068,7 +2105,7 @@ is off until the person turns it on.
 | Part | Runs | State on 2026-10-13 |
 |---|---|---|
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
-| 1b. The server over stdio, with its interface | 7, below | The first three and a half are built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md` and `2026-10-14-mcp-server-1b-4a-activity.md`). Nothing the others wait for is open |
+| 1b. The server over stdio, with its interface | 7, below | The first four are built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md` and `2026-10-14-mcp-server-1b-4b-log.md`). Nothing the others wait for is open |
 | 1b on Windows: the way in | 1 | After 1b (question 14) |
 | 1c. Hidden tables | to be cut in its plan | Waits for questions 8 and 13 |
 | 1d. HTTP with tokens | 1 | After stdio (question 9) |
@@ -2140,7 +2177,7 @@ can see what agents do and stop them.
 | **1b-3a. The Agents tab's frame** (built: `2026-10-14-mcp-server-1b-3a-agents-tab.md`) | The third kind of tab. The sidebar entry with its states but Waiting. The ways in: the key, `ga`, `:agents`, quick open. The three empty states, with **Turn on** and **Copy**. The sessions the app learns of, and those that are here. The keymap and the help | Open the tab in any state, see which state the connection is in and who is here, turn the server on from the tab | F.7: the entry's states, the ways in, one tab for a window, the empty states, nothing opening or focusing by itself |
 | **1b-3b. Changes made outside the app** (built: `2026-10-14-mcp-server-1b-3b-changes-outside.md`) | C.2: the agent settings the app last showed, kept in the state directory, and a change from outside that makes agents more able held back until the person keeps it | A program that edits the files cannot turn agents on unseen | C.2's cases |
 | **1b-4a. What agents are doing** (built: `2026-10-14-mcp-server-1b-4a-activity.md`) | `App::agents`'s feed, from `Server::calls`. The sessions that are here and "Also:". **Stop session** and **Stop agents here** | See each agent and each call as it happens, stop one agent or all on a connection | F.7: one connection, the stops, closing the tab |
-| **1b-4b. The log, and a new agent** | The Log view as a list for this connection, with All connections, and **Show log**. The notification for an agent that connects | Read the log, and hear of an agent that connects | F.7: the log's default. F.4's interface |
+| **1b-4b. The log, and a new agent** (built: `2026-10-14-mcp-server-1b-4b-log.md`) | The Log view as a list for this connection, with All connections, and **Show log**. The notification for an agent that connects | Read the log, and hear of an agent that connects | F.7: the log's default. F.4's interface |
 | **1b-5. Letting agents in** | Settings › Agents, with the switch between panes. The Connections list's Agents column, its menu and Omarchy's box. The connection editor's Agents section, with **Allow agents…** and **Show Agents tab** pointing at them. The website: a new guide, and the reference pages | Everything Phase 1 draws, without a file edited by hand | The interface tests of Settings, Connections and the editor. The keymap. `docs/bin/check` |
 
 - **A button arrives with what it opens.** The empty states of the third
````

- [ ] **Step 2: Check it**

Run: `grep -c "$(printf '\342\200\224')" docs/superpowers/plans/2026-10-13-mcp-server.md` (an em dash, written in octal so that this plan holds none)

Expected: `0`.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what the second part of its fourth run built"
```

---

### After the last task

- [ ] **The whole of it, once more**, with both servers' variables set:

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
docs/bin/check
```

Expected: all clean, and every test passes. On 2026-10-14, with both servers up: 23 test binaries, 3,454 passed, none failed, 22 ignored. The website's check ends `HTML-Proofer finished successfully.`

- [ ] **By hand, on Linux**: with the server on and a connection allowed, connect Claude Code: a notification says it connected, and Review brings Tabletist forward on the Agents tab. Ask it something; open the Log view (**Log**, or `l` on Omarchy): the calls are there with their dates, the newest first; select one and read its line. All connections shows the other connections' calls. Turn the server off: the log still reads. Set `notify = false` under `[mcp]`: a new agent is not told of. **On macOS**: the Dock's icon bounces when an agent connects.

- [ ] **Open the pull request.**

## Self-review

- **Against the master plan's row for this run** (G, as cut on 2026-10-14): the Log view for this connection with All connections and **Show log** (Tasks 2 and 3), the notification for an agent that connects (Task 4). **Against D.5**: Phase 1b's part, the list, the switch to All connections and the detail line; the rest is 3b's, in "What this run leaves out". **Against D.4's notifications**: one for each session, the text without SQL, Review to the tab, `mcp.notify`.
- **Against F.7**: the log's default is this connection (`the_log_lists_the_calls...`, both looks); no event but the person's click on Review opens or brings forward anything (`a_call_goes_into_the_feed_and_opens_nothing`, 4a's, still holds). **Against F.4's interface**: the log read by connection, in pages, with the server off.
- **Against the canvas**, read again on 2026-10-14: `macOS – Agents tab, log` for the switch, the bar, the columns and the line; `Omarchy – Agents tab, activity, approval and log` for the log's line, `L` and the detail. What is drawn otherwise is in the table above and the master plan's "As built".
- **Reviewed before it was written.** A reviewer was given the draft and asked to break it, with tests of its own. It found these, each fixed above with a test: a call that ended while a read was out never showed; `o` turned agents on and `y` copied from the Log view; the status line and Show log named `:agents log` for the key; `j` moved the selection out of sight; a log a crash left mid-write could not be read with the server off; and Review went to Connections while a window was open. And two lines of text.
- **Names.** `Logged`, `Page`, `open_to_read`, `page`, `ReadLog`, `LogRead`, `LOG_PAGE`, `AgentsView`, `AgentsLog`, `AgentsTab::new`, `ShowAgentsView`, `ToggleAgentsLogScope`, `MoreAgentsLog`, `SelectAgentsLogCall`, `MoveAgentsLogSelection`, `clock::{time, hour, day, moment, of_millis}`, `ShowAgentsLog`, `Notify`, `Reviewed`, `told`, `window_asks`, `mcp_notify`: each is used under the one name in every task.
- **Every commit stands.** After Task 1 the log can be read in pages, and nobody reads it. After Task 2 the app reads it for a tab, and shows nothing more. After Task 3 the Log view shows it, with the clock. After Task 4 an agent that connects is told of. Task 5 is the master plan's text.
- **No placeholders.** Every step has its code or its command, and what the command prints.
