# MCP server, Phase 3b, run 1: the log's filters. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Agents tab's Log view shows the calls of one agent, the calls that ended one way, and the calls whose request holds a text, as well as this connection's or every one's. On macOS these are the board's dropdowns and search field. On Omarchy, `/` opens them over the list. The count says how many calls match.

**Architecture:** `LogFilter` (`crates/tabletist-mcp/src/log.rs`) holds the connection, the agent, the result and the search, and `Log::page` reads the calls that pass it, with their count and, on a first page, the agents that called. The backend's `ReadLog` carries it. `AgentsLog` (`src/model.rs`) keeps the filters, and `Action::FilterAgentsLog` sets one and reads the first page again. `src/ui/agents.rs` draws them: `log_filters` on macOS, `terminal_filters` on Omarchy, with the keymap's `FilterAgentsLog` (`/`) and `CloseAgentsLogFilters` (`esc`).

**Tech Stack:** Rust 2024, `rusqlite`, egui.

---

## Where this sits

Phase 3b of `docs/superpowers/plans/2026-10-13-mcp-server.md`, the Log view in full, cut into three runs on 2026-10-18. Phase 1b's fourth run built the view as a list of this connection's calls or every one's, with the selected call's line. This run, 1, adds the filters and the search. Run 2 is the detail pane and **Open in SQL editor**; run 3 is **Export…** and the website's guide to the log. Phase 2, approved writes, comes after 3b.

## What the plan asks, and what is built

| The plan | Built | Why |
|---|---|---|
| D.5: "Filters: This connection (the default) or All connections, the agent, the result, and a search of the SQL" | All four, a query to the backend answered in pages, as D.5 says | |
| The macOS board: `This connection ▾`, where Phase 1b drew a segmented control | The board's dropdown | Follows the board |
| The macOS board: "Search SQL…" | "Search requests…", which finds its text in a statement or in the tables `describe_schema` was asked for | What the Request column shows. Agreed on 2026-10-18 |
| The Omarchy board: `/ filter`, with nothing drawn for the agent and the result | `/` opens a search field and rows of words for the agent and the result over the list; Esc closes them and keeps them | Agreed on 2026-10-18 |
| The board: "186 calls · last 30 days · on this computer" | The same; with a filter set, "12 calls match" | The count is of what is shown |

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`. The Agents tab shows only where agents can reach the app (`cfg(any(unix, windows))`). The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.

## What was run before this plan was written

Every task's state was built here, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` with both test servers (3,681 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. Not run: CI, and the view by hand against a real log: the bar, the panel and its line were looked at by eye in headless renders of both looks.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| How 3b is cut | Three runs: the filters and the search; the detail pane and Open in SQL editor; Export… and the guide | As 3a went. Agreed on 2026-10-18 |
| What the search matches | Its text anywhere in the Request column: a statement, or the tables `describe_schema` was asked for. `%` and `_` are themselves; English letters match in either case (SQLite's `LIKE`) | Agreed on 2026-10-18 |
| Which agents are offered | Those that called on what is shown, read with each first page, whatever else is filtered, and the one chosen where it did not | An agent that never called there is no choice |
| Which results are offered | ok, refused, failed, timed out, cancelled, stopped: what a call can end with today | A write's outcomes come with writes |
| Omarchy's filters | `/` opens a search field, with the keyboard, and rows of words for the agent and the result, which Tab moves among and Enter or Space picks; Esc closes them and keeps them; a line under the bar says what they are set to | Agreed on 2026-10-18; the editor's rows work so today |
| A filter changed | The first page is read again, the read before it no longer taken, and nothing is selected | As a change of the scope |
| The connection's index | With a connection, the agent's and the result's conditions take no index of theirs | One connection's calls through its own index (found by the review) |

### What this run leaves out

- The detail pane and **Open in SQL editor**: run 2. **Export…**: run 3.
- Until a filter's answer comes, the list shown is the one before it, under the new count's words. A change of the scope did so already.

## File structure

| File | What changes |
|---|---|
| `crates/tabletist-mcp/src/log.rs`, `lib.rs` | `LogFilter`; `Log::page` by it; `Page::agents` |
| `src/backend/agents.rs` | `ReadLog` carries the filter |
| `src/model.rs` | `AgentsLog`'s filters, `AgentsLogFilter`, `LOG_RESULTS`; the actions; the Agents tab boxed |
| `src/app.rs` | The filters set and read with; the view left closes them |
| `src/ui/agents.rs` | `log_filters` (macOS), `terminal_filters` and `words` (Omarchy), the count |
| `src/keymap.rs`, `src/ui/keys.rs`, `src/ui/workspace.rs` | `/` and `esc`; the footer's `/ filter` |
| `src/ui/agents_tests.rs` | The view's tests |
| `docs/_guide/agents.md`, `docs/_guide/omarchy.md` | The filters; the keys |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | 3b cut; D.5 and G as built |

---

### Task 1: The log read by a filter

- `LogFilter` (`tabletist-mcp`): the connection, the agent, the result, and a search of the request (the statement, or the tables `describe_schema` was asked for). `Log::page(filter, before, limit)` reads the calls that pass each part given, and counts them; the statement is built of the parts asked alone, and with a connection the agent's and the result's conditions take no index of theirs, so that one connection's calls are read through its own. A first page says the agents that called on the filter's connection (`Page::agents`).
- The backend's `ReadLog` carries the filter.
- `AgentsLog` keeps the agent, the result, the search, the agents offered and the terminal look's filters (`filters_open`, `filters_asked`). `Action::FilterAgentsLog` sets one and, where it changed, reads the first page again and selects nothing; `Action::ShowAgentsLogFilters` opens or closes the terminal look's. The view left closes them. `Tab::Agents` is boxed, as the other tabs are.

**Files:**
- Modify: `crates/tabletist-mcp/src/log.rs`, `crates/tabletist-mcp/src/lib.rs`, `src/backend/agents.rs`, `src/model.rs`, `src/app.rs`, `src/ui/agents_tests.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/crates/tabletist-mcp/src/log.rs b/crates/tabletist-mcp/src/log.rs
index 7a2bacf..cac196c 100644
--- a/crates/tabletist-mcp/src/log.rs
+++ b/crates/tabletist-mcp/src/log.rs
@@ -540,17 +540,106 @@ mod tests {
                 .map(|logged| logged.entry.at_ms)
                 .collect::<Vec<_>>()
         };
-        let first = log.page(None, None, 2).await.unwrap();
+        let all = LogFilter::default;
+        let first = log.page(all(), None, 2).await.unwrap();
         assert_eq!((at(&first), first.total, first.more), (vec![4, 3], 5, true));
-        let next = log.page(None, Some(first.calls[1].id), 2).await.unwrap();
+        let next = log.page(all(), Some(first.calls[1].id), 2).await.unwrap();
         assert_eq!((at(&next), next.more), (vec![2, 1], true));
-        let shop = log.page(Some("shop".into()), None, 10).await.unwrap();
+        let shop = log
+            .page(LogFilter::on(Some("shop".into())), None, 10)
+            .await
+            .unwrap();
         assert_eq!(
             (at(&shop), shop.total, shop.more),
             (vec![4, 2, 0], 3, false)
         );
     }
 
+    #[tokio::test]
+    async fn a_page_holds_the_calls_of_an_agent_a_result_and_a_search() {
+        let dir = tempfile::tempdir().unwrap();
+        let log = open(&dir.path().join("agents.sqlite3"));
+        let calls = [
+            ("Claude Code", "ok", Some("SELECT * FROM books"), None),
+            (
+                "Cursor",
+                "refused",
+                Some("select name from customers"),
+                None,
+            ),
+            ("Cursor", "ok", None, Some("books, authors")),
+            ("Cursor", "ok", Some("SELECT '100%' AS sure"), None),
+            ("Claude Code", "error", Some("SELECT x_y FROM orders"), None),
+        ];
+        for (n, (agent, result, sql, request)) in calls.into_iter().enumerate() {
+            let mut call = entry(n as i64, "run_query");
+            call.agent = agent.into();
+            call.result = result.into();
+            call.sql = sql.map(str::to_owned);
+            call.request = request.map(str::to_owned);
+            call.connection_id = Some(if n == 4 { "ledger" } else { "shop" }.into());
+            log.record(call).await.unwrap();
+        }
+        let at = |filter: LogFilter| {
+            let log = log.clone();
+            async move {
+                let page = log.page(filter, None, 10).await.unwrap();
+                let at: Vec<i64> = page.calls.iter().map(|logged| logged.entry.at_ms).collect();
+                (at, page.total)
+            }
+        };
+        let search = |text: &str| LogFilter {
+            search: Some(text.into()),
+            ..LogFilter::default()
+        };
+        // An agent, a result, and both, with the connection.
+        let cursor = LogFilter {
+            agent: Some("Cursor".into()),
+            ..LogFilter::default()
+        };
+        assert_eq!(at(cursor.clone()).await, (vec![3, 2, 1], 3));
+        let refused = LogFilter {
+            result: Some("refused".into()),
+            ..cursor.clone()
+        };
+        assert_eq!(at(refused).await, (vec![1], 1));
+        let ledger = LogFilter {
+            connection: Some("ledger".into()),
+            ..cursor
+        };
+        assert_eq!(at(ledger).await, (vec![], 0));
+        // The search reads the statement and the tables asked for, in any
+        // case of English letters; `%` and `_` are themselves.
+        assert_eq!(at(search("from")).await, (vec![4, 1, 0], 3));
+        assert_eq!(at(search("authors")).await, (vec![2], 1));
+        assert_eq!(at(search("100%")).await, (vec![3], 1));
+        assert_eq!(at(search("x_y")).await, (vec![4], 1));
+        assert_eq!(at(search("x%y")).await, (vec![], 0));
+        assert_eq!(at(search("")).await, (vec![4, 3, 2, 1, 0], 5));
+        // The agents of the connection, whatever else is asked; a later
+        // page says none.
+        let page = log
+            .page(
+                LogFilter {
+                    connection: Some("shop".into()),
+                    result: Some("error".into()),
+                    ..LogFilter::default()
+                },
+                None,
+                10,
+            )
+            .await
+            .unwrap();
+        assert_eq!(page.agents, ["Claude Code", "Cursor"]);
+        assert_eq!(page.total, 0);
+        let page = log.page(LogFilter::default(), None, 1).await.unwrap();
+        let later = log
+            .page(LogFilter::default(), Some(page.calls[0].id), 1)
+            .await
+            .unwrap();
+        assert!(later.agents.is_empty());
+    }
+
     #[tokio::test]
     async fn a_log_a_crash_left_mid_write_is_read_as_it_was_before() {
         let dir = tempfile::tempdir().unwrap();
@@ -577,7 +666,7 @@ mod tests {
         std::fs::copy(journal(&path), journal(&crashed)).unwrap();
         drop(writer);
         let log = Log::open_to_read(&crashed).unwrap().unwrap();
-        let page = log.page(None, None, 1).await.unwrap();
+        let page = log.page(LogFilter::default(), None, 1).await.unwrap();
         assert_eq!(page.total, 2_000);
         assert_eq!(page.calls[0].entry.tool, "run_query");
     }
@@ -595,10 +684,49 @@ mod tests {
                 .unwrap();
             rows.map(Result::unwrap).collect::<Vec<_>>().join("; ")
         };
-        let page = plan(&page_of_one(), params!["shop", i64::MAX, 201]);
+        let (one, _) = LogFilter::on(Some("shop".into())).conditions();
+        let page = plan(&page_of(&one), params!["shop", i64::MAX, 201]);
         assert!(page.contains("calls_connection"), "{page}");
-        let count = plan(COUNT_OF_ONE, params!["shop"]);
+        let count = plan(&count_of(&one), params!["shop"]);
         assert!(count.contains("calls_connection"), "{count}");
+        // With an agent, a result or a search too, alone or together:
+        // still the connection's index, for the page and for the count.
+        let parts = |agent: bool, result: bool, search: bool| LogFilter {
+            connection: Some("shop".into()),
+            agent: agent.then(|| "Cursor".into()),
+            result: result.then(|| "ok".into()),
+            search: search.then(|| "books".into()),
+        };
+        for (agent, result, search) in [
+            (true, false, false),
+            (false, true, false),
+            (false, false, true),
+            (true, true, false),
+            (true, true, true),
+        ] {
+            let (conditions, values) = parts(agent, result, search).conditions();
+            let mut bound: Vec<&dyn rusqlite::ToSql> = values
+                .iter()
+                .map(|value| value as &dyn rusqlite::ToSql)
+                .collect();
+            let count = plan(&count_of(&conditions), &bound);
+            assert!(count.contains("calls_connection"), "{count}");
+            bound.extend([&i64::MAX as &dyn rusqlite::ToSql, &201]);
+            let page = plan(&page_of(&conditions), &bound);
+            assert!(page.contains("calls_connection"), "{page}");
+        }
+        // Without a connection, a result's own index.
+        let (conditions, values) = LogFilter {
+            result: Some("ok".into()),
+            ..LogFilter::default()
+        }
+        .conditions();
+        let bound: Vec<&dyn rusqlite::ToSql> = values
+            .iter()
+            .map(|value| value as &dyn rusqlite::ToSql)
+            .collect();
+        let count = plan(&count_of(&conditions), &bound);
+        assert!(count.contains("calls_result"), "{count}");
     }
 
     #[tokio::test]
@@ -613,7 +741,13 @@ mod tests {
             .await
             .unwrap();
         let log = Log::open_to_read(&path).unwrap().unwrap();
-        assert_eq!(log.page(None, None, 10).await.unwrap().total, 1);
+        assert_eq!(
+            log.page(LogFilter::default(), None, 10)
+                .await
+                .unwrap()
+                .total,
+            1
+        );
         // A file that is not a log is said so, and left where it is.
         let other = dir.path().join("other.sqlite3");
         let made = Connection::open(&other).unwrap();
diff --git a/src/app.rs b/src/app.rs
index 7c15355..bef8457 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -7149,11 +7149,11 @@ mod tests {
             .rev()
             .find_map(|sent| match sent {
                 AgentsCommand::ReadLog {
-                    connection,
+                    filter,
                     before,
                     asked,
                     ..
-                } => Some((connection.clone(), *before, *asked)),
+                } => Some((filter.connection.clone(), *before, *asked)),
                 _ => None,
             })
     }
@@ -7178,6 +7178,7 @@ mod tests {
             calls,
             total: 186,
             more,
+            agents: vec!["Claude Code".into()],
         }
     }
 
@@ -7247,6 +7248,86 @@ mod tests {
         assert!(log.all);
     }
 
+    #[cfg(any(unix, windows))]
+    #[test]
+    fn a_filter_set_reads_the_logs_first_page_again_with_it() {
+        use crate::model::AgentsLogFilter;
+        let (mut app, _dir) = app();
+        let (tab, _, _) = connect(&mut app);
+        let conn = app.workspace(tab).unwrap().conn_id.0.clone();
+        app.apply(Action::ShowAgentsView {
+            tab,
+            view: crate::model::AgentsView::Log,
+        });
+        let (_, _, asked) = log_asked(&app).unwrap();
+        let page = Ok(log_page(&conn, 500, 200, true));
+        app.apply_event(Event::Agents(AgentsEvent::LogRead { asked, page }));
+        // The agents that called come with the first page.
+        assert_eq!(agents_log(&app, tab).agents, ["Claude Code"]);
+        app.apply(Action::SelectAgentsLogCall { tab, id: 499 });
+        let filter = |app: &App| {
+            agents_sent(app)
+                .into_iter()
+                .rev()
+                .find_map(|sent| match sent {
+                    AgentsCommand::ReadLog { filter, before, .. } => {
+                        Some((filter.clone(), *before))
+                    }
+                    _ => None,
+                })
+                .unwrap()
+        };
+        let reads = |app: &App| {
+            agents_sent(app)
+                .iter()
+                .filter(|sent| matches!(sent, AgentsCommand::ReadLog { .. }))
+                .count()
+        };
+        // Each filter reads the first page again, and selects nothing.
+        let set = |app: &mut App, filter: AgentsLogFilter| {
+            app.apply(Action::FilterAgentsLog { tab, filter });
+        };
+        set(&mut app, AgentsLogFilter::Agent(Some("Cursor".into())));
+        set(&mut app, AgentsLogFilter::Result(Some("refused".into())));
+        set(&mut app, AgentsLogFilter::Search("books".into()));
+        let wanted = tabletist_mcp::LogFilter {
+            connection: Some(conn.clone()),
+            agent: Some("Cursor".into()),
+            result: Some("refused".into()),
+            search: Some("books".into()),
+        };
+        assert_eq!(filter(&app), (wanted.clone(), None));
+        assert_eq!(agents_log(&app, tab).selected, None);
+        // The same again reads nothing.
+        let before = reads(&app);
+        set(&mut app, AgentsLogFilter::Search("books".into()));
+        assert_eq!(reads(&app), before);
+        // Every connection's keeps the rest; an empty search is none.
+        app.apply(Action::ToggleAgentsLogScope(tab));
+        let (_, _, earlier) = log_asked(&app).unwrap();
+        set(&mut app, AgentsLogFilter::Search(String::new()));
+        let (read, _) = filter(&app);
+        assert_eq!(
+            read,
+            tabletist_mcp::LogFilter {
+                connection: None,
+                search: None,
+                ..wanted
+            }
+        );
+        // Only the last read's page is taken: the one before it is not.
+        let page = Ok(log_page(&conn, 9, 1, false));
+        app.apply_event(Event::Agents(AgentsEvent::LogRead {
+            asked: earlier,
+            page: page.clone(),
+        }));
+        assert_eq!(agents_log(&app, tab).calls.len(), 200);
+        let (_, _, asked) = log_asked(&app).unwrap();
+        assert_ne!(asked, earlier);
+        app.apply_event(Event::Agents(AgentsEvent::LogRead { asked, page }));
+        assert_eq!(agents_log(&app, tab).calls.len(), 1);
+    }
+
     #[cfg(any(unix, windows))]
     #[test]
     fn a_call_on_the_connection_reads_the_log_again() {
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 0367275..65e7cd6 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -1704,7 +1704,7 @@ mod tests {
         let read = |backend: &mut Backend, connection: Option<&str>| {
             backend.send(Command::Agents(AgentsCommand::ReadLog {
                 log: path.clone(),
-                connection: connection.map(str::to_owned),
+                filter: tabletist_mcp::LogFilter::on(connection.map(str::to_owned)),
                 before: None,
                 asked: 7,
             }));
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index 80bde1d..cc20cb9 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -792,9 +792,9 @@ fn log_asked(harness: &Harness) -> (u64, Option<String>) {
         .find_map(|sent| match sent {
             crate::backend::Command::Agents(crate::backend::AgentsCommand::ReadLog {
                 asked,
-                connection,
+                filter,
                 ..
-            }) => Some((*asked, connection.clone())),
+            }) => Some((*asked, filter.connection.clone())),
             _ => None,
         })
         .expect("a read of the log")
@@ -822,6 +822,7 @@ fn log_answered(harness: &mut Harness) {
         calls: vec![tabletist_mcp::Logged { id: 5, entry }],
         total: 1,
         more: false,
+        agents: vec!["Claude Code".into(), "Cursor".into()],
     };
     said(
         harness,
@@ -963,6 +964,7 @@ fn a_call_selected_with_j_is_kept_in_sight() {
         calls,
         total: 200,
         more: false,
+        agents: Vec::new(),
     };
     said(
         &mut harness,
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked -p tabletist-mcp --lib -- log::tests`, then `cargo test --locked --lib -- a_filter_set_reads the_log_is_read_a_page`

Expected: neither builds:

```text
error[E0425]: cannot find type `LogFilter` in this scope
error[E0609]: no field `agents` on type `log::Page`
error[E0425]: cannot find function `page_of` in this scope
error: could not compile `tabletist-mcp` (lib test) due to 26 previous errors
error[E0432]: unresolved import `crate::model::AgentsLogFilter`
error[E0026]: variant `backend::agents::AgentsCommand::ReadLog` does not have a field named `filter`
error[E0599]: no variant named `FilterAgentsLog` found for enum `model::Action`
error: could not compile `tabletist` (lib test) due to 13 previous errors
```

- [ ] **Step 3: The filter**

````diff
diff --git a/crates/tabletist-mcp/src/lib.rs b/crates/tabletist-mcp/src/lib.rs
index 918cf4b..0457c77 100644
--- a/crates/tabletist-mcp/src/lib.rs
+++ b/crates/tabletist-mcp/src/lib.rs
@@ -28,7 +28,7 @@ mod tools;
 
 pub use answer::{MAX_ANSWER_BYTES, MAX_CELL_CHARS};
 pub use gateway::{Failure, Gateway, Listed, Schemas, Session};
-pub use log::{Entry, Log, LogError, Logged, Opened, Page};
+pub use log::{Entry, Log, LogError, LogFilter, Logged, Opened, Page};
 pub use refusal::{Refused, code};
 pub use server::{CALL_CHARS, Call, Origin, Server, SessionInfo, Transport};
 pub use tools::{Tool, all as tools};
diff --git a/crates/tabletist-mcp/src/log.rs b/crates/tabletist-mcp/src/log.rs
index cac196c..5b8babe 100644
--- a/crates/tabletist-mcp/src/log.rs
+++ b/crates/tabletist-mcp/src/log.rs
@@ -129,11 +129,87 @@ pub struct Logged {
 #[derive(Debug, Clone, Default, PartialEq, Eq)]
 pub struct Page {
     pub calls: Vec<Logged>,
-    /// How many calls the log holds of what was asked: one connection's,
-    /// or every one.
+    /// How many calls the log holds of what was asked: those that pass
+    /// the filter.
     pub total: u64,
     /// Whether there are older ones.
     pub more: bool,
+    /// The agents that called, on the filter's connection or on every one,
+    /// whatever else it asks: what the agent filter offers. Read with a
+    /// first page alone; empty with a later one.
+    pub agents: Vec<String>,
+}
+
+/// Which calls a page of the log holds: those on one connection or on
+/// every one, of one agent or any, that ended one way or any, and whose
+/// request holds a text or not.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct LogFilter {
+    /// The connection's id.
+    pub connection: Option<String>,
+    /// The agent, as it names itself.
+    pub agent: Option<String>,
+    /// How the call ended, as [`Entry::result`] says it.
+    pub result: Option<String>,
+    /// Text the request holds: the statement, or the tables
+    /// `describe_schema` was asked for. English letters match in either
+    /// case.
+    pub search: Option<String>,
+}
+
+impl LogFilter {
+    /// The filter of every call on `connection`, or on every connection.
+    pub fn on(connection: Option<String>) -> Self {
+        Self {
+            connection,
+            ..Self::default()
+        }
+    }
+
+    /// The conditions a call passes, and the values they are bound to, in
+    /// order. One for each part asked: a part that may be nothing, bound
+    /// as a value, would plan the statement without its index. With a
+    /// connection, the agent and the result are written so that no index
+    /// of theirs is taken (a `+` before the column): one connection's
+    /// calls are read through its own, not every connection's that ended
+    /// one way.
+    fn conditions(&self) -> (Vec<&'static str>, Vec<String>) {
+        let mut conditions = Vec::new();
+        let mut values = Vec::new();
+        let one = self.connection.is_some();
+        if let Some(connection) = &self.connection {
+            conditions.push("connection_id = ?");
+            values.push(connection.clone());
+        }
+        if let Some(agent) = &self.agent {
+            conditions.push(if one { "+agent = ?" } else { "agent = ?" });
+            values.push(agent.clone());
+        }
+        if let Some(result) = &self.result {
+            conditions.push(if one { "+result = ?" } else { "result = ?" });
+            values.push(result.clone());
+        }
+        if let Some(search) = self.search.as_deref().filter(|search| !search.is_empty()) {
+            // The text as it is: `%`, `_` and the escape match themselves.
+            let escaped = search
+                .replace('\\', "\\\\")
+                .replace('%', "\\%")
+                .replace('_', "\\_");
+            let pattern = format!("%{escaped}%");
+            conditions.push("(sql LIKE ? ESCAPE '\\' OR request LIKE ? ESCAPE '\\')");
+            values.push(pattern.clone());
+            values.push(pattern);
+        }
+        (conditions, values)
+    }
+
+    /// `WHERE` and the conditions, or nothing for every call.
+    fn clause(conditions: &[&str]) -> String {
+        match conditions.is_empty() {
+            true => String::new(),
+            false => format!(" WHERE {}", conditions.join(" AND ")),
+        }
+    }
 }
 
 /// The log, open. A clone writes to the same file.
@@ -287,7 +363,7 @@ impl Log {
 
     /// The last `limit` entries, the newest first.
     pub async fn latest(&self, limit: u32) -> Result<Vec<Entry>, LogError> {
-        let page = self.page(None, None, limit).await?;
+        let page = self.page(LogFilter::default(), None, limit).await?;
         Ok(page.calls.into_iter().map(|logged| logged.entry).collect())
     }
 
@@ -321,11 +397,12 @@ impl Log {
         }
     }
 
-    /// A page of the log, the newest first: `limit` calls from before the
-    /// one `before` names, those on `connection` alone where it is given.
+    /// A page of the log, the newest first: `limit` calls that pass
+    /// `filter`, from before the one `before` names. A first page says the
+    /// agents of the filter's connection too.
     pub async fn page(
         &self,
-        connection: Option<String>,
+        filter: LogFilter,
         before: Option<i64>,
         limit: u32,
     ) -> Result<Page, LogError> {
@@ -339,26 +416,25 @@ impl Log {
                     entry: entry_at(row, 1)?,
                 })
             };
-            // A statement for each: one whose filter is a bound value that
-            // may be nothing is planned without the connection's index.
-            let (mut calls, total) = match &connection {
-                Some(connection) => {
-                    let calls = db
-                        .prepare(&page_of_one())?
-                        .query_map(params![connection, before, asked], logged)?
-                        .collect::<Result<Vec<Logged>, rusqlite::Error>>()?;
-                    let total: i64 = db.query_row(COUNT_OF_ONE, [connection], |row| row.get(0))?;
-                    (calls, total)
-                }
-                None => {
-                    let calls = db
-                        .prepare(&page_of_all())?
-                        .query_map(params![before, asked], logged)?
-                        .collect::<Result<Vec<Logged>, rusqlite::Error>>()?;
-                    let total: i64 =
-                        db.query_row("SELECT count(*) FROM calls", [], |row| row.get(0))?;
-                    (calls, total)
-                }
+            let (conditions, values) = filter.conditions();
+            let mut calls = {
+                let mut bound: Vec<&dyn rusqlite::ToSql> = values
+                    .iter()
+                    .map(|value| value as &dyn rusqlite::ToSql)
+                    .collect();
+                bound.extend([&before as &dyn rusqlite::ToSql, &asked]);
+                db.prepare(&page_of(&conditions))?
+                    .query_map(bound.as_slice(), logged)?
+                    .collect::<Result<Vec<Logged>, rusqlite::Error>>()?
+            };
+            let total: i64 = db.query_row(
+                &count_of(&conditions),
+                rusqlite::params_from_iter(&values),
+                |row| row.get(0),
+            )?;
+            let agents = match before == i64::MAX {
+                true => agents_of(db, filter.connection.as_deref())?,
+                false => Vec::new(),
             };
             let more = calls.len() > limit as usize;
             calls.truncate(limit as usize);
@@ -366,6 +442,7 @@ impl Log {
                 calls,
                 total: u64::try_from(total).unwrap_or_default(),
                 more,
+                agents,
             })
         })
         .await
@@ -404,21 +481,41 @@ const COLUMNS: &str = "at_ms, session, agent, project, transport, connection_id,
     tool, sql, params, request, result, code, message, ran_as, rows, matched, columns, bytes, \
     masked, took_ms";
 
-/// A page of one connection's calls, through its index.
-fn page_of_one() -> String {
+/// A page of the calls that pass `conditions`, from before a call: their
+/// values are bound first, then the call's place and the page's length.
+fn page_of(conditions: &[&str]) -> String {
+    let mut all = conditions.to_vec();
+    all.push("id < ?");
     format!(
-        "SELECT id, {COLUMNS} FROM calls WHERE connection_id = ?1 AND id < ?2 \
-         ORDER BY id DESC LIMIT ?3"
+        "SELECT id, {COLUMNS} FROM calls{} ORDER BY id DESC LIMIT ?",
+        LogFilter::clause(&all)
     )
 }
 
-/// A page of every connection's calls.
-fn page_of_all() -> String {
-    format!("SELECT id, {COLUMNS} FROM calls WHERE id < ?1 ORDER BY id DESC LIMIT ?2")
+/// How many calls pass `conditions`.
+fn count_of(conditions: &[&str]) -> String {
+    format!(
+        "SELECT count(*) FROM calls{}",
+        LogFilter::clause(conditions)
+    )
 }
 
-/// How many calls one connection has, from its index alone.
-const COUNT_OF_ONE: &str = "SELECT count(*) FROM calls WHERE connection_id = ?1";
+/// The agents that called on `connection`, or on any, by name.
+fn agents_of(db: &Connection, connection: Option<&str>) -> Result<Vec<String>, rusqlite::Error> {
+    let (sql, values): (&str, Vec<&str>) = match connection {
+        Some(connection) => (
+            "SELECT DISTINCT agent FROM calls WHERE connection_id = ? ORDER BY agent",
+            vec![connection],
+        ),
+        None => (
+            "SELECT DISTINCT agent FROM calls ORDER BY agent",
+            Vec::new(),
+        ),
+    };
+    db.prepare(sql)?
+        .query_map(rusqlite::params_from_iter(values), |row| row.get(0))?
+        .collect()
+}
 
 /// The entry in `row`, from its column `at` on, in [`COLUMNS`]'s order.
 fn entry_at(row: &Row<'_>, at: usize) -> Result<Entry, rusqlite::Error> {
diff --git a/src/app.rs b/src/app.rs
index bef8457..e2c0cb0 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1723,10 +1723,17 @@ impl App {
             }
             Action::ShowAgentsView { tab, view } => {
                 self.show_agents(tab);
+                // The log's filters close as the view is left: shown again,
+                // it shows no keyboard-less panel.
                 let shown = self
                     .workspace_mut(tab)
                     .and_then(crate::model::Workspace::agents_tab_mut)
-                    .map(|agents| agents.view = view);
+                    .map(|agents| {
+                        if agents.view != view {
+                            agents.log.filters_open = false;
+                        }
+                        agents.view = view;
+                    });
                 // The log as it is now, each time it is shown.
                 if shown.is_some() && view == crate::model::AgentsView::Log {
                     self.read_agents_log(tab, false);
@@ -1743,6 +1750,41 @@ impl App {
                 self.read_agents_log(tab, false);
             }
             Action::MoreAgentsLog(tab) => self.read_agents_log(tab, true),
+            Action::FilterAgentsLog { tab, filter } => {
+                if let Some(agents) = self
+                    .workspace_mut(tab)
+                    .and_then(crate::model::Workspace::agents_tab_mut)
+                {
+                    let log = &mut agents.log;
+                    use crate::model::AgentsLogFilter;
+                    let changed = match filter {
+                        AgentsLogFilter::Agent(agent) => {
+                            std::mem::replace(&mut log.agent, agent.clone()) != agent
+                        }
+                        AgentsLogFilter::Result(result) => {
+                            std::mem::replace(&mut log.result, result.clone()) != result
+                        }
+                        AgentsLogFilter::Search(text) => {
+                            std::mem::replace(&mut log.search, text.clone()) != text
+                        }
+                    };
+                    if changed {
+                        log.selected = None;
+                        self.read_agents_log(tab, false);
+                    }
+                }
+            }
+            Action::ShowAgentsLogFilters { tab, open } => {
+                if let Some(agents) = self
+                    .workspace_mut(tab)
+                    .and_then(crate::model::Workspace::agents_tab_mut)
+                {
+                    agents.log.filters_open = open;
+                    if open {
+                        agents.log.filters_asked += 1;
+                    }
+                }
+            }
             Action::SelectAgentsLogCall { tab, id } => {
                 if let Some(agents) = self
                     .workspace_mut(tab)
@@ -4420,7 +4462,7 @@ impl App {
         let shown = workspace.agents_tab().unwrap_or_else(|| {
             workspace
                 .tabs
-                .push(Tab::Agents(crate::model::AgentsTab::new(id)));
+                .push(Tab::Agents(Box::new(crate::model::AgentsTab::new(id))));
             id
         });
         workspace.active_tab = Some(shown);
@@ -4445,11 +4487,11 @@ impl App {
         let before = more
             .then(|| agents.log.calls.last().map(|logged| logged.id))
             .flatten();
-        let connection = (!agents.log.all).then_some(conn);
+        let filter = agents.log.filter(&conn);
         agents.log.asked = Some((asked, more));
         self.backend.send(Command::Agents(AgentsCommand::ReadLog {
             log,
-            connection,
+            filter,
             before,
             asked,
         }));
@@ -4487,6 +4529,9 @@ impl App {
                     log.calls.extend(page.calls.iter().cloned());
                     log.total = page.total;
                     log.more = page.more;
+                    if !more {
+                        log.agents = page.agents.clone();
+                    }
                     log.failed = None;
                     // A call that is not on the page any more is not
                     // selected.
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 65e7cd6..c21ec46 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -156,12 +156,11 @@ pub enum AgentsCommand {
     /// stopped them, and its agent is let go. Nothing where it is gone.
     StopSession(String),
     /// Reads a page of the log at `log`, whether the server runs or not:
-    /// the calls on `connection`, or on every one, from before the call
-    /// `before` names. Answered with [`AgentsEvent::LogRead`], `asked`
-    /// with it.
+    /// the calls that pass `filter`, from before the call `before` names.
+    /// Answered with [`AgentsEvent::LogRead`], `asked` with it.
     ReadLog {
         log: PathBuf,
-        connection: Option<String>,
+        filter: tabletist_mcp::LogFilter,
         before: Option<i64>,
         asked: u64,
     },
@@ -267,13 +266,13 @@ impl Agents {
             }
             AgentsCommand::ReadLog {
                 log,
-                connection,
+                filter,
                 before,
                 asked,
             } => {
                 let outbox = outbox.clone();
                 tokio::spawn(async move {
-                    let page = read_log(log, connection, before).await;
+                    let page = read_log(log, filter, before).await;
                     outbox.emit(super::Event::Agents(AgentsEvent::LogRead { asked, page }));
                 });
             }
@@ -479,7 +478,7 @@ async fn sessions(
 /// aside. No log yet is an empty one.
 async fn read_log(
     log: PathBuf,
-    connection: Option<String>,
+    filter: tabletist_mcp::LogFilter,
     before: Option<i64>,
 ) -> Result<tabletist_mcp::Page, String> {
     let opened = tokio::task::spawn_blocking(move || Log::open_to_read(&log))
@@ -488,7 +487,7 @@ async fn read_log(
         .map_err(|error| error.to_string())?;
     match opened {
         Some(log) => log
-            .page(connection, before, LOG_PAGE)
+            .page(filter, before, LOG_PAGE)
             .await
             .map_err(|error| error.to_string()),
         None => Ok(tabletist_mcp::Page::default()),
diff --git a/src/model.rs b/src/model.rs
index 7c235ff..98c4e89 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -646,6 +646,18 @@ pub enum Action {
     ToggleAgentsLogScope(ConnTabId),
     /// The Log view: read the page after the last call shown.
     MoreAgentsLog(ConnTabId),
+    /// The Log view: show only the calls of an agent, that ended one way,
+    /// or whose request holds a text; or every one again.
+    FilterAgentsLog {
+        tab: ConnTabId,
+        filter: AgentsLogFilter,
+    },
+    /// The Log view, in the terminal look: open its filters, or close
+    /// them and keep what they are set to.
+    ShowAgentsLogFilters {
+        tab: ConnTabId,
+        open: bool,
+    },
     /// The Log view: select a call, by its place in the log.
     SelectAgentsLogCall {
         tab: ConnTabId,
@@ -3372,7 +3384,7 @@ pub enum Tab {
     Object(Box<ObjectTab>),
     Sql(Box<SqlTab>),
     /// What agents do on the workspace's connection. One at most.
-    Agents(AgentsTab),
+    Agents(Box<AgentsTab>),
 }
 
 impl Tab {
@@ -3464,12 +3476,42 @@ pub enum AgentsView {
     Log,
 }
 
+/// One of the Log view's filters, set.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum AgentsLogFilter {
+    /// One agent's calls, or every agent's.
+    Agent(Option<String>),
+    /// The calls that ended one way, as the log says it, or every one.
+    Result(Option<String>),
+    /// The calls whose request holds this text: every call where it is
+    /// empty.
+    Search(String),
+}
+
+/// How a call can end, as the log keeps it: what the result filter
+/// offers. A write's outcomes come with writes.
+pub const LOG_RESULTS: [&str; 6] = ["ok", "refused", "error", "timeout", "cancelled", "stopped"];
+
 /// The Agents tab's Log view: the log of calls, a page at a time, the
 /// newest first, read from the log's file whether the server runs or not.
 #[derive(Debug, Clone, Default, PartialEq, Eq)]
 pub struct AgentsLog {
     /// Every connection's calls, not this one's alone.
     pub all: bool,
+    /// The agent whose calls alone are shown.
+    pub agent: Option<String>,
+    /// How the calls shown ended, as the log says it.
+    pub result: Option<String>,
+    /// Text the requests shown hold; none where it is empty.
+    pub search: String,
+    /// The agents that called on what is shown, whatever else is
+    /// filtered: what the agent filter offers. Said by each first page.
+    pub agents: Vec<String>,
+    /// The terminal look's filters, open over the list.
+    pub filters_open: bool,
+    /// How many times they were asked to open: each time, the search takes
+    /// the keyboard.
+    pub filters_asked: u64,
     pub calls: Vec<tabletist_mcp::Logged>,
     /// How many calls the log holds of what is shown.
     pub total: u64,
@@ -3488,6 +3530,24 @@ pub struct AgentsLog {
     pub read: bool,
 }
 
+impl AgentsLog {
+    /// Whether any filter but the connection's is set.
+    pub fn filtered(&self) -> bool {
+        self.agent.is_some() || self.result.is_some() || !self.search.is_empty()
+    }
+
+    /// The filter a page of it is read with, the calls on `conn` alone
+    /// where it shows this connection's.
+    pub fn filter(&self, conn: &str) -> tabletist_mcp::LogFilter {
+        tabletist_mcp::LogFilter {
+            connection: (!self.all).then(|| conn.to_owned()),
+            agent: self.agent.clone(),
+            result: self.result.clone(),
+            search: (!self.search.is_empty()).then(|| self.search.clone()),
+        }
+    }
+}
+
 /// What the app knows of the agent server: the sessions open on it, as
 /// the backend last said them, empty while the server is off; and the
 /// calls that ended since the app started, the newest first.
@@ -4448,7 +4508,7 @@ impl Workspace {
     /// The Agents tab, where it is open.
     pub fn agents_tab_mut(&mut self) -> Option<&mut AgentsTab> {
         self.tabs.iter_mut().find_map(|open| match open {
-            Tab::Agents(agents) => Some(agents),
+            Tab::Agents(agents) => Some(&mut **agents),
             Tab::Object(_) | Tab::Sql(_) => None,
         })
     }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-mcp --lib -- log::tests`, then `cargo test --locked --lib -- a_filter_set_reads the_log_is_read_a_page agents_tests`

Expected:

```text
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 39 filtered out
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 2630 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-mcp src/backend/agents.rs src/model.rs src/app.rs src/ui/agents_tests.rs
git commit -m "Read the agents' log by agent, result and request, as well as by connection"
```

---

### Task 2: The filters in the Log view

- macOS (`log_filters`): the board's bar, a dropdown each for whose calls ("Calls on": This connection or All connections), the agent ("All agents" and those that called) and the result ("All results" and the six), and **Search requests…**, with the count at the right.
- Omarchy (`terminal_filters`): `/` (`Command::FilterAgentsLog`) opens the search, with the keyboard, and rows of words for the agent and the result (`words`), which Tab moves among and Enter or Space picks; `esc` (`Command::CloseAgentsLogFilters`, read by the view) closes them and keeps them; a line under the bar says what they are set to. The bar and the footer name `/ filter`.
- With a filter set the count says "12 calls match", and a list none matches says "No calls in the log match these filters."

**Files:**
- Modify: `src/ui/agents.rs`, `src/keymap.rs`, `src/ui/keys.rs`, `src/ui/workspace.rs`, `src/ui/agents_tests.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index cc20cb9..205efd0 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -879,6 +879,8 @@ fn the_log_lists_the_calls_and_says_what_the_selected_one_came_to() {
         if look.terminal {
             super::tests::type_key(&mut harness, egui::Key::L, "L");
         } else {
+            harness.click("Calls on");
+            harness.settle();
             harness.click("All connections");
             harness.settle();
         }
@@ -886,6 +888,198 @@ fn the_log_lists_the_calls_and_says_what_the_selected_one_came_to() {
     }
 }
 
+/// The filter of the last read of the log.
+fn log_filter(harness: &Harness) -> tabletist_mcp::LogFilter {
+    harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|sent| match sent {
+            crate::backend::Command::Agents(crate::backend::AgentsCommand::ReadLog {
+                filter,
+                ..
+            }) => Some(filter.clone()),
+            _ => None,
+        })
+        .expect("a read of the log")
+}
+
+/// Answers the log's last read with no call, of a log in which Claude
+/// Code and Cursor called: what a filter that matches nothing reads.
+fn log_answered_empty(harness: &mut Harness) {
+    let (asked, _) = log_asked(harness);
+    let page = tabletist_mcp::Page {
+        agents: vec!["Claude Code".into(), "Cursor".into()],
+        ..tabletist_mcp::Page::default()
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
+/// Gives the keyboard to the field named `label`, as a screen reader
+/// would.
+fn focus_field(harness: &mut Harness, label: &str) {
+    use egui::accesskit::Role;
+    let tree = harness.settle();
+    let field = tree
+        .nodes
+        .iter()
+        .find(|(_, node)| node.role() == Role::TextInput && node.label() == Some(label))
+        .map(|(id, _)| *id)
+        .unwrap_or_else(|| panic!("no field named {label:?}"));
+    harness.frame(vec![egui::Event::AccessKitActionRequest(
+        egui::accesskit::ActionRequest {
+            target_tree: egui::accesskit::TreeId::ROOT,
+            target_node: field,
+            action: egui::accesskit::Action::Focus,
+            data: None,
+        },
+    )]);
+    harness.settle();
+}
+
+/// The Log view of `tab`'s connection, read once: one call of Claude
+/// Code's, in a log Cursor called on too.
+fn on_the_log(look: Look) -> (Harness, ConnTabId) {
+    let (mut harness, tab) = connected(look);
+    harness.app.settings.mcp_enabled = true;
+    allow(&mut harness, tab, AgentAccess::Read);
+    harness.app.apply(Action::ShowAgentsView {
+        tab,
+        view: crate::model::AgentsView::Log,
+    });
+    harness.settle();
+    log_answered(&mut harness);
+    assert!(harness.painted_rect("SELECT id FROM books").is_some());
+    (harness, tab)
+}
+
+#[test]
+fn the_logs_filters_read_an_agents_calls_a_result_and_a_search() {
+    let (mut harness, tab) = on_the_log(Look::macos());
+    let conn = harness.app.workspace(tab).unwrap().conn_id.0.clone();
+    // The agents that called on the connection are offered.
+    harness.click("Agent");
+    harness.settle();
+    assert!(harness.has("Claude Code"));
+    harness.click("Cursor");
+    harness.settle();
+    let mut wanted = tabletist_mcp::LogFilter {
+        connection: Some(conn.clone()),
+        agent: Some("Cursor".into()),
+        ..tabletist_mcp::LogFilter::default()
+    };
+    assert_eq!(log_filter(&harness), wanted);
+    // How they ended, as the list says it.
+    harness.click("Result");
+    harness.settle();
+    harness.click("refused");
+    harness.settle();
+    wanted.result = Some("refused".into());
+    assert_eq!(log_filter(&harness), wanted);
+    // What the requests hold, as it is typed.
+    focus_field(&mut harness, "Search requests");
+    harness.frame(vec![egui::Event::Text("from books".into())]);
+    harness.settle();
+    wanted.search = Some("from books".into());
+    assert_eq!(log_filter(&harness), wanted);
+    // Matched by none: said so. One that matches is counted as such.
+    log_answered_empty(&mut harness);
+    assert!(harness.has("No calls in the log match these filters."));
+    harness.frame(vec![egui::Event::Text("s".into())]);
+    harness.settle();
+    log_answered(&mut harness);
+    assert!(harness.has("This connection · 1 call matches · last 30 days · on this computer"));
+    wanted.search = Some("from bookss".into());
+    // Every agent again, and every connection: the other filters stay.
+    harness.click("Agent");
+    harness.settle();
+    harness.click("All agents");
+    harness.settle();
+    harness.click("Calls on");
+    harness.settle();
+    harness.click("All connections");
+    harness.settle();
+    wanted.agent = None;
+    wanted.connection = None;
+    assert_eq!(log_filter(&harness), wanted);
+}
+
+#[test]
+fn omarchys_slash_opens_the_logs_filters_and_esc_keeps_them() {
+    let (mut harness, tab) = on_the_log(Look::omarchy());
+    let conn = harness.app.workspace(tab).unwrap().conn_id.0.clone();
+    let view = |harness: &Harness| {
+        harness
+            .app
+            .workspace(tab)
+            .unwrap()
+            .tabs
+            .iter()
+            .find_map(|open| match open {
+                crate::model::Tab::Agents(agents) => Some((agents.view, agents.log.filters_open)),
+                _ => None,
+            })
+    };
+    use crate::model::AgentsView;
+    assert!(harness.painted_rect("/ filter").is_some());
+    super::tests::type_key(&mut harness, egui::Key::Slash, "/");
+    harness.settle();
+    assert_eq!(view(&harness), Some((AgentsView::Log, true)));
+    // The search has the keyboard: what is typed is the search's, the
+    // tab's letters among it.
+    harness.frame(vec![egui::Event::Text("all".into())]);
+    harness.settle();
+    assert_eq!(view(&harness), Some((AgentsView::Log, true)));
+    assert_eq!(log_filter(&harness).search.as_deref(), Some("all"));
+    // Tab goes to the agents, then to Claude Code's, which Space picks.
+    harness.press(egui::Key::Tab, egui::Modifiers::NONE);
+    harness.press(egui::Key::Tab, egui::Modifiers::NONE);
+    harness.press(egui::Key::Space, egui::Modifiers::NONE);
+    harness.settle();
+    assert_eq!(log_filter(&harness).agent.as_deref(), Some("Claude Code"));
+    // Esc closes them and keeps them, which the line under the bar says.
+    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
+    harness.settle();
+    assert_eq!(view(&harness), Some((AgentsView::Log, false)));
+    assert!(harness.has("filters agent claude code · search “all”"));
+    assert_eq!(
+        log_filter(&harness),
+        tabletist_mcp::LogFilter {
+            connection: Some(conn),
+            agent: Some("Claude Code".into()),
+            result: None,
+            search: Some("all".into()),
+        }
+    );
+    // Open again, the search has the keyboard again, with what it holds.
+    super::tests::type_key(&mut harness, egui::Key::Slash, "/");
+    harness.settle();
+    harness.press(egui::Key::Backspace, egui::Modifiers::NONE);
+    harness.settle();
+    assert_eq!(log_filter(&harness).search.as_deref(), Some("al"));
+    // Left from a word with `l`, and shown again: they are closed, and `/`
+    // gives the search the keyboard once more.
+    harness.press(egui::Key::Tab, egui::Modifiers::NONE);
+    super::tests::type_key(&mut harness, egui::Key::L, "l");
+    assert_eq!(view(&harness), Some((AgentsView::Activity, false)));
+    super::tests::type_key(&mut harness, egui::Key::L, "l");
+    assert_eq!(view(&harness), Some((AgentsView::Log, false)));
+    super::tests::type_key(&mut harness, egui::Key::Slash, "/");
+    harness.settle();
+    harness.frame(vec![egui::Event::Text("l".into())]);
+    harness.settle();
+    assert_eq!(log_filter(&harness).search.as_deref(), Some("all"));
+    assert_eq!(view(&harness), Some((AgentsView::Log, true)));
+}
+
 #[test]
 fn omarchy_shows_the_log_with_l_and_the_agents_log_line() {
     let (mut harness, tab) = connected(Look::omarchy());
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib -- the_logs_filters omarchys_slash the_log_lists_the_calls`

Expected: the two new tests fail, and so does the earlier one, which now opens the scope's dropdown, as nothing draws the filters yet:

```text
test ui::agents_tests::omarchys_slash_opens_the_logs_filters_and_esc_keeps_them ... FAILED
test ui::agents_tests::the_logs_filters_read_an_agents_calls_a_result_and_a_search ... FAILED
test ui::agents_tests::the_log_lists_the_calls_and_says_what_the_selected_one_came_to ... FAILED
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 2672 filtered out
```

- [ ] **Step 3: The views**

````diff
diff --git a/src/keymap.rs b/src/keymap.rs
index 4e9da81..b084eb0 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -342,6 +342,8 @@ commands! {
     ShowAgentsElsewhere,
     ShowAgentsLog,
     ToggleAgentsLogScope,
+    FilterAgentsLog,
+    CloseAgentsLogFilters,
     AllowAgents,
     ShowAgentSettings,
     ChangeAgentAccess,
@@ -614,6 +616,14 @@ impl Command {
                 "",
                 Window,
             )),
+            C::FilterAgentsLog => agents(info(
+                "Filter the log by agent, result or request",
+                "",
+                Window,
+            )),
+            C::CloseAgentsLogFilters => {
+                agents(info("Close the log's filters, and keep them", "", Window))
+            }
             C::ToggleSidebar => info("Toggle sidebar", "Toggle sidebar", Navigation),
             C::PaneLeft => info("Pane to the left", PANES, Navigation),
             C::PaneDown => info("Pane below", PANES, Navigation).reserved(),
@@ -1024,6 +1034,8 @@ pub const BINDINGS: &[Binding] = {
         bind(C::ShowAgentsLog, GLOBAL, NONE, keys(&[k(":agents log")])),
         bind(C::ShowAgentsLog, AGENTS, NONE, keys(&[k("l")])),
         bind(C::ToggleAgentsLogScope, AGENTS, NONE, keys(&[k("L")])),
+        bind(C::FilterAgentsLog, AGENTS, NONE, keys(&[k("/")])),
+        bind(C::CloseAgentsLogFilters, AGENTS, NONE, keys(&[k("esc")])),
         bind(C::AllowAgents, AGENTS, NONE, keys(&[k("e")])),
         bind(C::ShowAgentSettings, AGENTS, NONE, keys(&[k("s")])),
         bind(C::ChangeAgentAccess, CONNECTIONS, NONE, keys(&[k("a")])),
diff --git a/src/ui/agents.rs b/src/ui/agents.rs
index dc209da..928a78a 100644
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -99,6 +99,12 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let scope_key = app
         .keymap
         .key(layout, crate::keymap::Command::ToggleAgentsLogScope);
+    let filter_key = app
+        .keymap
+        .key(layout, crate::keymap::Command::FilterAgentsLog);
+    let close_key = app
+        .keymap
+        .key(layout, crate::keymap::Command::CloseAgentsLogFilters);
     let show_key = app.keymap.key(layout, crate::keymap::Command::ShowAgents);
     let copy_key = app
         .keymap
@@ -223,14 +229,20 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             // the server off and access Off.
             if view == AgentsView::Log {
                 let days = app.settings.mcp_log_days;
-                let shown = log_view(
-                    ui,
-                    rect,
-                    (log, &scope_key, days),
-                    (&look, &palette, locale),
-                );
+                let keys = LogKeys {
+                    scope: &scope_key,
+                    filter: &filter_key,
+                    close: &close_key,
+                };
+                let shown = log_view(ui, rect, (log, keys, days), (&look, &palette, locale));
                 match shown {
                     Some(LogAsk::Scope) => actions.push(Action::ToggleAgentsLogScope(tab)),
+                    Some(LogAsk::Filter(filter)) => {
+                        actions.push(Action::FilterAgentsLog { tab, filter });
+                    }
+                    Some(LogAsk::CloseFilters) => {
+                        actions.push(Action::ShowAgentsLogFilters { tab, open: false });
+                    }
                     Some(LogAsk::More) => actions.push(Action::MoreAgentsLog(tab)),
                     Some(LogAsk::Select(id)) => {
                         actions.push(Action::SelectAgentsLogCall { tab, id });
@@ -999,12 +1011,19 @@ enum LogAsk {
     More,
     /// A call, by its place in the log.
     Select(i64),
+    /// A filter set, or every call again.
+    Filter(crate::model::AgentsLogFilter),
+    /// The terminal look's filters closed, and kept.
+    CloseFilters,
 }
 
-/// The Log view's bar at its top, its line for the selected call, and a
-/// call's line.
+/// The Log view's bar at its top (the terminal's, and macOS's, which holds
+/// the filters), its line for the selected call, and a call's line.
 const LOG_BAR: f32 = 36.0;
+const LOG_FILTER_BAR: f32 = 48.0;
 const LOG_DETAIL: f32 = 30.0;
+/// A row of the terminal look's filters.
+const FILTER_ROW: f32 = 28.0;
 
 /// The Log view: whose calls (this connection's or every one's) and how
 /// many the log keeps, then the calls the newest first, and under them
@@ -1012,24 +1031,29 @@ const LOG_DETAIL: f32 = 30.0;
 fn log_view(
     ui: &mut egui::Ui,
     rect: Rect,
-    (log, scope_key, days): (&crate::model::AgentsLog, &crate::keymap::Written, u32),
+    (log, keys, days): (&crate::model::AgentsLog, LogKeys<'_>, u32),
     (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
 ) -> Option<LogAsk> {
     let mut ask = None;
     let body = widgets::body(look);
     let small = widgets::secondary(look);
-    let bar = Rect::from_min_size(rect.min, vec2(rect.width(), LOG_BAR));
+    let height = if look.terminal {
+        LOG_BAR
+    } else {
+        LOG_FILTER_BAR
+    };
+    let bar = Rect::from_min_size(rect.min, vec2(rect.width(), height));
     widgets::hline(ui, bar.x_range(), bar.bottom() - 0.5, palette.border);
     let scope = match log.all {
         false => gettext(locale, "This connection"),
         true => gettext(locale, "All connections"),
     };
-    let count = ngettext(
-        locale,
-        "{n} call",
-        "{n} calls",
-        u32::try_from(log.total).unwrap_or(u32::MAX),
-    )
+    let calls = u32::try_from(log.total).unwrap_or(u32::MAX);
+    // Of the calls a filter lets through, they match it.
+    let count = match log.filtered() {
+        false => ngettext(locale, "{n} call", "{n} calls", calls),
+        true => ngettext(locale, "{n} call matches", "{n} calls match", calls),
+    }
     .replace("{n}", &log.total.to_string());
     let kept =
         ngettext(locale, "last {n} day", "last {n} days", days).replace("{n}", &days.to_string());
@@ -1052,30 +1076,34 @@ fn log_view(
                 &look.label(&format!("{scope} · {count} · ")),
                 palette.secondary,
             )
-            .add(body, scope_key.as_str(), palette.accent)
+            .add(body, keys.scope.as_str(), palette.accent)
             .add(body, &format!(" {other}"), palette.secondary);
         widgets::paint_text(ui, bar.left() + 14.0, bar.center().y, line);
+        // At its right, `/` for the filters.
+        let filter = Text::new(look)
+            .add(body, keys.filter.as_str(), palette.accent)
+            .add(
+                body,
+                &format!(" {}", look.label(&gettext(locale, "filter"))),
+                palette.secondary,
+            );
+        widgets::paint_text_right(ui, bar.right() - 14.0, bar.center().y, filter);
         widgets::announce(ui, bar, &look.label(&format!("{scope} · {count}")));
     } else {
-        let names = [
-            gettext(locale, "This connection"),
-            gettext(locale, "All connections"),
-        ];
-        let parts = [
-            widgets::Segment::Text(&names[0]),
-            widgets::Segment::Text(&names[1]),
-        ];
-        let place = Rect::from_min_size(
-            pos2(bar.left() + 16.0, bar.center().y - 12.0),
-            vec2(2.0 * 120.0, 24.0),
+        let row = Rect::from_min_max(
+            pos2(bar.left() + 16.0, bar.center().y - 14.0),
+            pos2(bar.right() - 16.0, bar.center().y + 14.0),
         );
-        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
-        let segment = vec2(120.0, 24.0);
-        let chosen = usize::from(log.all);
-        if widgets::segmented(&mut child, &parts, chosen, segment, look, palette)
-            .is_some_and(|index| index != chosen)
+        let mut child = ui.new_child(
+            egui::UiBuilder::new()
+                .max_rect(row)
+                .layout(egui::Layout::left_to_right(egui::Align::Center)),
+        );
+        child.spacing_mut().item_spacing.x = 8.0;
+        if let Some(filter) =
+            log_filters(&mut child, log, &scope, (look, palette, locale), &mut ask)
         {
-            ask = Some(LogAsk::Scope);
+            ask = Some(LogAsk::Filter(filter));
         }
         let laid = Text::one(look, small, &count, palette.secondary).layout(ui.ctx());
         laid.paint_left(
@@ -1085,6 +1113,19 @@ fn log_view(
         );
         widgets::announce(ui, bar, &format!("{scope} · {count}"));
     }
+    // The terminal look's filters, under its bar: their rows while open,
+    // else a line that says what they are set to where any is.
+    let mut top = bar.bottom();
+    if look.terminal {
+        top = terminal_filters(
+            ui,
+            (rect, top),
+            log,
+            keys,
+            (look, palette, locale),
+            &mut ask,
+        );
+    }
     // The selected call's line, at the bottom.
     let detail = Rect::from_min_max(pos2(rect.left(), rect.bottom() - LOG_DETAIL), rect.max);
     let chosen = log
@@ -1111,7 +1152,7 @@ fn log_view(
     } else {
         rect.bottom()
     };
-    let list = Rect::from_min_max(pos2(rect.left(), bar.bottom()), pos2(rect.right(), bottom));
+    let list = Rect::from_min_max(pos2(rect.left(), top), pos2(rect.right(), bottom));
     let row = feed_row(look);
     let empty = match (&log.failed, log.read, log.all) {
         (Some(failed), _, _) => Some(format!(
@@ -1120,6 +1161,9 @@ fn log_view(
         )),
         (None, false, _) => Some(gettext(locale, "Reading the log…").into_owned()),
         (None, true, _) if !log.calls.is_empty() => None,
+        (None, true, _) if log.filtered() => {
+            Some(gettext(locale, "No calls in the log match these filters.").into_owned())
+        }
         (None, true, false) => {
             Some(gettext(locale, "No calls on this connection in the log.").into_owned())
         }
@@ -1297,6 +1341,379 @@ fn log_view(
     ask
 }
 
+/// The keys the Log view names, and reads, as the keymap has them.
+#[derive(Clone, Copy)]
+pub(crate) struct LogKeys<'a> {
+    /// Every connection's calls, or this one's.
+    pub scope: &'a crate::keymap::Written,
+    /// The terminal look's filters, opened.
+    pub filter: &'a crate::keymap::Written,
+    /// And closed.
+    pub close: &'a crate::keymap::Written,
+}
+
+/// The macOS look's filters, in a row of `ui`: whose calls, which agent's,
+/// how they ended, and a search of their requests, as the board has them.
+/// The filter set, where one was; the scope is asked through `ask`.
+fn log_filters(
+    ui: &mut egui::Ui,
+    log: &crate::model::AgentsLog,
+    scope: &str,
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+    ask: &mut Option<LogAsk>,
+) -> Option<crate::model::AgentsLogFilter> {
+    use crate::model::AgentsLogFilter;
+    use crate::ui::menu;
+    let mut set = None;
+    let named = |response: &egui::Response, name: &str, value: &str| {
+        response.widget_info(|| {
+            let mut info = egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, name);
+            info.current_text_value = Some(value.to_owned());
+            info
+        });
+    };
+    // Whose calls.
+    let this = gettext(locale, "This connection");
+    let all = gettext(locale, "All connections");
+    let scopes = menu::Dropdown::new("agents-log-scope", scope, 170.0)
+        .height(28.0)
+        .show(ui, look, palette, |ui| {
+            for (every, word) in [(false, &this), (true, &all)] {
+                let item = menu::Item::choice(word, log.all == every).show(ui, look, palette);
+                if item.clicked() {
+                    if log.all != every {
+                        *ask = Some(LogAsk::Scope);
+                    }
+                    ui.close();
+                }
+            }
+        });
+    named(&scopes.response, &gettext(locale, "Calls on"), scope);
+    // Which agent's: those that called on what is shown, and the one
+    // chosen, whether it called there or not.
+    let every_agent = gettext(locale, "All agents");
+    let agent = log.agent.clone().unwrap_or_else(|| every_agent.to_string());
+    let mut agents = log.agents.clone();
+    if let Some(chosen) = &log.agent
+        && !agents.contains(chosen)
+    {
+        agents.push(chosen.clone());
+    }
+    let menu = menu::Dropdown::new("agents-log-agent", &agent, 130.0)
+        .height(28.0)
+        .show(ui, look, palette, |ui| {
+            let mut choices = vec![(None, every_agent.to_string())];
+            choices.extend(agents.iter().map(|name| (Some(name.clone()), name.clone())));
+            for (value, word) in choices {
+                let item = menu::Item::choice(&word, log.agent == value).show(ui, look, palette);
+                if item.clicked() {
+                    set = Some(AgentsLogFilter::Agent(value));
+                    ui.close();
+                }
+            }
+        });
+    named(&menu.response, &gettext(locale, "Agent"), &agent);
+    // How they ended.
+    let every_result = gettext(locale, "All results");
+    let result = log.result.as_deref().map_or_else(
+        || every_result.to_string(),
+        |result| result_word(result, locale),
+    );
+    let menu = menu::Dropdown::new("agents-log-result", &result, 120.0)
+        .height(28.0)
+        .show(ui, look, palette, |ui| {
+            let mut choices = vec![(None, every_result.to_string())];
+            choices.extend(
+                crate::model::LOG_RESULTS
+                    .iter()
+                    .map(|result| (Some((*result).to_owned()), result_word(result, locale))),
+            );
+            for (value, word) in choices {
+                let item = menu::Item::choice(&word, log.result == value).show(ui, look, palette);
+                if item.clicked() {
+                    set = Some(AgentsLogFilter::Result(value));
+                    ui.close();
+                }
+            }
+        });
+    named(&menu.response, &gettext(locale, "Result"), &result);
+    // What the requests hold.
+    let mut text = log.search.clone();
+    let placeholder = gettext(locale, "Search requests…");
+    let role = widgets::body(look);
+    let field = ui.add_sized(
+        vec2(220.0, 28.0),
+        widgets::field(ui, &mut text, look, role, 28.0, 8)
+            .id(Id::new("agents-log-search"))
+            .hint_text(placeholder.as_ref()),
+    );
+    ui.ctx().accesskit_node_builder(field.id, |node| {
+        node.set_label(placeholder.trim_end_matches('…'));
+    });
+    if field.changed() {
+        set = Some(AgentsLogFilter::Search(text));
+    }
+    set
+}
+
+/// The terminal look's filters, under its bar at `top` of `rect`: while
+/// open, a field for the search and rows of words for the agent and the
+/// result, which Tab moves among and Enter or Space picks, until the
+/// keymap's close; while shut, the line that says what they are set to,
+/// where any is. Where the list starts under them.
+fn terminal_filters(
+    ui: &mut egui::Ui,
+    (rect, top): (Rect, f32),
+    log: &crate::model::AgentsLog,
+    keys: LogKeys<'_>,
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+    ask: &mut Option<LogAsk>,
+) -> f32 {
+    use crate::model::AgentsLogFilter;
+    let body = widgets::body(look);
+    let small = widgets::secondary(look);
+    let every = look.label(&gettext(locale, "all"));
+    // The agents that called on what is shown, and the one chosen,
+    // whether it called there or not.
+    let mut agents: Vec<Option<String>> = std::iter::once(None)
+        .chain(log.agents.iter().cloned().map(Some))
+        .collect();
+    if let Some(chosen) = &log.agent
+        && !log.agents.contains(chosen)
+    {
+        agents.push(Some(chosen.clone()));
+    }
+    let agent_words: Vec<String> = agents
+        .iter()
+        .map(|agent| {
+            agent
+                .as_deref()
+                .map_or_else(|| every.clone(), |name| look.label(name))
+        })
+        .collect();
+    let results: Vec<Option<String>> = std::iter::once(None)
+        .chain(
+            crate::model::LOG_RESULTS
+                .iter()
+                .map(|result| Some((*result).to_owned())),
+        )
+        .collect();
+    let result_words: Vec<String> = results
+        .iter()
+        .map(|result| match result {
+            None => every.clone(),
+            Some(result) => look.label(&result_word(result, locale)),
+        })
+        .collect();
+    // Each `/` gives the search the keyboard: the count of them seen last.
+    let seen_id = Id::new("agents-log-filters-seen");
+    let seen: Option<u64> = ui.data(|data| data.get_temp(seen_id));
+    ui.data_mut(|data| data.insert_temp(seen_id, log.filters_asked));
+    if !log.filters_open {
+        if !log.filtered() {
+            return top;
+        }
+        // "filters  agent cursor · result refused · search “select”".
+        let mut said = Vec::new();
+        if let Some(agent) = &log.agent {
+            said.push(format!("{} {}", gettext(locale, "agent"), agent));
+        }
+        if let Some(result) = &log.result {
+            said.push(format!(
+                "{} {}",
+                gettext(locale, "result"),
+                result_word(result, locale)
+            ));
+        }
+        if !log.search.is_empty() {
+            said.push(format!("{} “{}”", gettext(locale, "search"), log.search));
+        }
+        let line = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), FILTER_ROW));
+        let words = look.label(&said.join(" · "));
+        let filters = look.label(&gettext(locale, "filters"));
+        let text = Text::new(look)
+            .add(body, &filters, palette.secondary)
+            .space(body, "  ")
+            .add(body, &words, palette.text);
+        widgets::paint_text(ui, line.left() + 14.0, line.center().y, text);
+        widgets::announce(ui, line, &format!("{filters} {words}"));
+        widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.border);
+        return line.bottom();
+    }
+    let panel = Rect::from_min_size(
+        pos2(rect.left(), top),
+        vec2(rect.width(), 3.0 * FILTER_ROW + 26.0),
+    );
+    ui.painter().rect_filled(panel, 0.0, palette.panel);
+    let label_at = |index: usize| {
+        Rect::from_min_size(
+            pos2(
+                panel.left() + 14.0,
+                panel.top() + 4.0 + index as f32 * FILTER_ROW,
+            ),
+            vec2(panel.width() - 28.0, FILTER_ROW),
+        )
+    };
+    let labels = ["search", "agent", "result"].map(|word| look.label(&gettext(locale, word)));
+    for (index, label) in labels.iter().enumerate() {
+        let at = label_at(index);
+        widgets::paint_text(
+            ui,
+            at.left(),
+            at.center().y,
+            Text::one(look, body, label, palette.dim),
+        );
+    }
+    let cell = |index: usize| {
+        let at = label_at(index);
+        Rect::from_min_max(pos2(at.left() + 100.0, at.top()), at.max)
+    };
+    // The search, which has the keyboard as the filters open.
+    let mut text = log.search.clone();
+    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(cell(0)));
+    let field = child.add_sized(
+        vec2(cell(0).width().min(420.0), FILTER_ROW - 4.0),
+        widgets::field(&child, &mut text, look, body, FILTER_ROW - 4.0, 6)
+            .id(Id::new("agents-log-search")),
+    );
+    ui.ctx().accesskit_node_builder(field.id, |node| {
+        node.set_label(labels[0].as_str());
+    });
+    if seen != Some(log.filters_asked) {
+        field.request_focus();
+    }
+    if field.changed() {
+        *ask = Some(LogAsk::Filter(AgentsLogFilter::Search(text)));
+    }
+    let chosen = agents
+        .iter()
+        .position(|agent| *agent == log.agent)
+        .unwrap_or(0);
+    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(cell(1)));
+    let picked = words(
+        &mut child,
+        "agents-log-agent",
+        &labels[1],
+        &agent_words,
+        chosen,
+        (look, palette),
+    );
+    if let Some(at) = picked {
+        *ask = Some(LogAsk::Filter(AgentsLogFilter::Agent(agents[at].clone())));
+    }
+    let chosen = results
+        .iter()
+        .position(|result| *result == log.result)
+        .unwrap_or(0);
+    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(cell(2)));
+    let picked = words(
+        &mut child,
+        "agents-log-result",
+        &labels[2],
+        &result_words,
+        chosen,
+        (look, palette),
+    );
+    if let Some(at) = picked {
+        *ask = Some(LogAsk::Filter(AgentsLogFilter::Result(results[at].clone())));
+    }
+    // What closes them, as the keymap has it.
+    let hint = label_at(3);
+    let text = Text::new(look)
+        .add(small, keys.close.as_str(), palette.accent)
+        .add(
+            small,
+            &format!(
+                " {}",
+                look.label(&gettext(locale, "closes, and keeps them"))
+            ),
+            palette.secondary,
+        );
+    widgets::paint_text(ui, cell(3).left(), hint.top() + 11.0, text);
+    let keymap = crate::ui::keys::published(ui.ctx());
+    let layout = crate::keymap::Layout::of(look);
+    let closed = ui.input_mut(|input| {
+        crate::ui::keys::asked(
+            input,
+            &keymap,
+            layout,
+            crate::keymap::Command::CloseAgentsLogFilters,
+            crate::keymap::Scope::Agents,
+            true,
+        )
+        .count
+            > 0
+    });
+    if closed {
+        field.surrender_focus();
+        *ask = Some(LogAsk::CloseFilters);
+    }
+    widgets::hline(ui, panel.x_range(), panel.bottom() - 0.5, palette.border);
+    panel.bottom()
+}
+
+/// Words in a line, of which `chosen` is boxed in the accent, as the
+/// connection editor's rows draw them in the terminal look: the one
+/// clicked, or picked with Enter or Space. Screen readers hear them as
+/// radio buttons of a group named `name`.
+fn words(
+    ui: &mut egui::Ui,
+    salt: &str,
+    name: &str,
+    labels: &[String],
+    chosen: usize,
+    (look, palette): (&Look, &Palette),
+) -> Option<usize> {
+    let role = widgets::body(look);
+    let (pad, gap, height) = (8.0, 6.0, 23.0);
+    let mut clicked = None;
+    ui.scope_builder(egui::UiBuilder::new().id_salt(salt), |ui| {
+        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
+            node.set_role(egui::accesskit::Role::RadioGroup);
+            node.set_label(name);
+        });
+        let mut left = ui.max_rect().left();
+        let middle = ui.max_rect().center().y;
+        for (index, label) in labels.iter().enumerate() {
+            let width = (role.width(ui.ctx(), look.faces, label) + 2.0 * pad).ceil();
+            let cell = Rect::from_min_size(pos2(left, middle - height / 2.0), vec2(width, height));
+            left += width + gap;
+            let active = index == chosen;
+            let response = ui.interact(cell, ui.id().with((salt, index)), egui::Sense::click());
+            response.widget_info(|| {
+                egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, active, label)
+            });
+            if response.clicked() {
+                clicked = Some(index);
+            }
+            let corner = egui::CornerRadius::same(3);
+            let color = if active {
+                ui.painter().rect_stroke(
+                    cell,
+                    corner,
+                    egui::Stroke::new(1.0, palette.accent),
+                    egui::StrokeKind::Inside,
+                );
+                palette.accent
+            } else if response.hovered() {
+                palette.text
+            } else {
+                palette.dim
+            };
+            crate::ui::focus::hint(
+                ui,
+                &response,
+                cell,
+                crate::ui::focus::Ring::Outer { radius: corner.nw },
+            );
+            Text::one(look, role, label, color)
+                .layout(ui.ctx())
+                .paint_left(ui.painter(), cell.left() + pad, cell.center().y);
+        }
+    });
+    clicked
+}
+
 /// How a call in the log ended, in a word, and its colour.
 fn log_result(
     result: &str,
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 6af2dd1..4fcca3d 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -2297,6 +2297,12 @@ fn letters(
             {
                 actions.push(Action::ToggleAgentsLogScope(tab));
             }
+            (Some(Command::FilterAgentsLog), _)
+                if agents_view(app, tab) == Some(crate::model::AgentsView::Log) =>
+            {
+                opened = true;
+                actions.push(Action::ShowAgentsLogFilters { tab, open: true });
+            }
             (Some(Command::MoveInAgentSessions), _)
                 if agents_view(app, tab) == Some(crate::model::AgentsView::Log) =>
             {
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index d61231d..42b998e 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2046,13 +2046,14 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .filter(|workspace| workspace.agents_in_front())
         .and_then(|_| app.agents_here(tab));
     // The tab's own `l`, not the prompt's `:agents log`.
-    let (agents_log_key, agents_scope_key) = (
+    let (agents_log_key, agents_scope_key, agents_filter_key) = (
         app.keymap.key_in(
             app.layout(),
             Command::ShowAgentsLog,
             crate::keymap::Scope::Agents,
         ),
         key(Command::ToggleAgentsLogScope),
+        key(Command::FilterAgentsLog),
     );
     // Its Log view, while it shows, and whether every connection's.
     let agents_log = app.workspace(tab).and_then(|workspace| {
@@ -2301,6 +2302,7 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 gettext(locale, "calls"),
                 gettext(locale, "all connections"),
                 gettext(locale, "this connection"),
+                gettext(locale, "filter"),
             ];
             match agents_log {
                 // The Log view's keys in the place of the activity's.
@@ -2309,6 +2311,7 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     agents_hints = vec![
                         (&agents_log_key, &*log_words[1], true),
                         (&agents_scope_key, scope, true),
+                        (&agents_filter_key, &*log_words[5], true),
                         (&agents_move_key, &*log_words[2], true),
                     ];
                 }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- the_logs_filters omarchys_slash the_log_lists_the_calls agents_tests`, then `cargo test --locked --test keys`

Expected:

```text
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 2632 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/ui/agents.rs src/keymap.rs src/ui/keys.rs src/ui/workspace.rs src/ui/agents_tests.rs
git commit -m "Filter the Agents tab's log by agent, result and request, in both looks"
```

---

### Task 3: The website

- `docs/_guide/agents.md`: what the Log view's bar narrows the list by, and Omarchy's `/`.
- `docs/_guide/omarchy.md`: `/` and Esc in the Agents tab's keys.

**Files:**
- Modify: `docs/_guide/agents.md`, `docs/_guide/omarchy.md`

- [ ] **Step 1: The pages**

````diff
diff --git a/docs/_guide/agents.md b/docs/_guide/agents.md
index 84c190a..88e2b5d 100644
--- a/docs/_guide/agents.md
+++ b/docs/_guide/agents.md
@@ -217,6 +217,17 @@ on all of them: the tool, the statement, how many rows it returned and how
 long it took. Rows themselves are never logged. **Settings › Agents ›
 Keep the activity log** says for how many days a call is kept.
 
+Its bar narrows the list: **This connection** or **All connections**, one
+agent of those that called or **All agents**, one way a call ended
+(**ok**, **refused**, **failed**, **timed out**, **cancelled**,
+**stopped**) or **All results**, and **Search requests…**, which finds
+the text anywhere in a statement or in the tables `describe_schema` was
+asked for, English letters in either case. The count then says how many
+calls match. In the Omarchy look, `/` opens the same filters over the
+list, with the keyboard in the search: Tab moves to the agents and the
+results, Enter or Space picks one, and Esc closes them and keeps them. A
+line under the bar says what they are set to.
+
 When an agent connects, Tabletist tells you: a notification on Linux,
 whose **Review** shows the Agents tab, the Dock icon bouncing on macOS,
 and the taskbar button flashing on Windows.
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index 6137a69..519423b 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -121,6 +121,8 @@ key. Nor has copying a whole row.
 | Enter | Show the window of the connection the selected agent works on |
 | `l` | Show the log of agents' calls, and back |
 | `L` | In the log, every connection's calls, or this one's |
+| `/` | In the log, open its filters: the search, the agent and the result. Tab moves among them, Enter or Space picks |
+| Esc | Close the log's filters, and keep them |
 | `j` / `k` | In the log, move among the calls |
 
 ### The Structure view
````

- [ ] **Step 2: Check them**

Run: `docs/bin/check` and `grep -c "$(printf '\342\200\224')" docs/_guide/agents.md docs/_guide/omarchy.md` (a count of em dashes)

Expected: `HTML-Proofer finished successfully.`, and `0` for each page.

- [ ] **Step 3: Commit**

```bash
git add docs/_guide/agents.md docs/_guide/omarchy.md
git commit -m "Say on the website how to filter the agents' log"
```

---

### Task 4: The master plan

- 3b cut in three runs; D.5 as built for 3b-1; G's row: run 1 built.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: The edits**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index a3c7909..89ae174 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -2409,6 +2409,39 @@ Phase 1b ships the list for this connection, the switch to All connections,
 and the detail line. Phase 3b adds the other filters, the search, the detail
 pane, export and "Open in SQL editor".
 
+**As built in 3b's run 1** (`2026-10-18-mcp-server-3b-1-log-filters.md`). Agreed on 2026-10-18, where it
+went otherwise:
+
+- **macOS**: the board's bar, a dropdown each for whose calls (This
+  connection or All connections, where Phase 1b drew a segmented
+  control), the agent and the result, then the search, and the count at
+  its right. The agents offered are those that called on what is shown,
+  read with each first page (`Page::agents`), and the one chosen where it
+  did not; the results are those a call can end with today, said as the
+  list says them (writes add theirs). The search is **Search requests…**,
+  not the board's "Search SQL…": it finds its text in the Request column,
+  a statement or the tables `describe_schema` was asked for, with `LIKE`,
+  its `%` and `_` taken as themselves, English letters in either case.
+- **Omarchy**: the board draws `/ filter` alone. `/` opens the filters
+  over the list, the search with the keyboard and rows of words for the
+  agent and the result, which Tab moves among and Enter or Space picks, as
+  the editor's rows work today; the keymap's Esc (`CloseAgentsLogFilters`)
+  closes them and keeps them, and a line under the bar says what they are
+  set to.
+- With a filter set, the count says "12 calls match", and a list that none
+  matches says so. A filter's change reads the first page again, the one
+  read before it no longer taken, and selects nothing.
+- `LogFilter` (`tabletist-mcp`) holds the four, and a page's statement and
+  count are built of the parts asked alone; with a connection, the agent's
+  and the result's conditions take no index of theirs (`+result = ?`), so
+  that one connection's calls are read through its own index and not every
+  connection's that ended one way.
+- The run's review found the result's index taken in the connection's
+  place, the filters shown open again with nothing to type in after the
+  view was left from a word, and a test of them run on an empty log: each
+  is fixed with a test. The view left closes them, and each `/` gives the
+  search the keyboard.
+
 ### D.6 The keymap
 
 Checked against `src/keymap.rs` and the keyboard brief
@@ -2694,7 +2727,7 @@ is off until the person turns it on.
 | 1c. Hidden tables | 6, below | Built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`, `2026-10-16-mcp-server-1c-2b-setting.md`, `2026-10-16-mcp-server-1c-3a-tables-control.md`, `2026-10-16-mcp-server-1c-3b-table-list.md`) |
 | 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |
 | 3a. Masked columns | 5, below | Built (`2026-10-17-mcp-server-3a-1a-masked-rule.md`, `2026-10-17-mcp-server-3a-1b-masked-values.md`, `2026-10-17-mcp-server-3a-2-server.md`, `2026-10-17-mcp-server-3a-3-setting.md`, `2026-10-18-mcp-server-3a-4-columns-control.md`): the rule, its stops, the engines' fences, the values, the server's side, the setting, and the editor's control with the website's guide. 3b is next (order changed on 2026-10-17) |
-| 3b. The Log view, in full | Not cut yet | After 3a |
+| 3b. The Log view, in full | 3, below | Run 1 built (`2026-10-18-mcp-server-3b-1-log-filters.md`): the agent and result filters and the search. Run 2, the detail pane and Open in SQL editor, is next |
 | 2. Approved writes | Not cut yet | Last, after 3b (moved on 2026-10-17) |
 
 The move of the connections to `connections.toml` came between 1a and 1b
@@ -2904,6 +2937,15 @@ pull request:
 - Risks: small.
 - Tests: the rest of F.4, the UI tests of the filters.
 
+**3b in three runs** (cut on 2026-10-18, as 3a went). Each is a plan and a
+pull request:
+
+1. The agent and result filters and the search of the requests, with the
+   count of what matches, in both looks
+   (`2026-10-18-mcp-server-3b-1-log-filters.md`, built).
+2. The detail pane for the selected call, and **Open in SQL editor**.
+3. **Export…**, and the website's guide to the log.
+
 ## Open questions
 
 The brief's four, each with what the code suggests, then the plan's own.
````

- [ ] **Step 2: Check the whole of it**

Run: `cargo fmt --all --check`, clippy as above, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, the count of em dashes in the master plan, and `cargo test --locked --workspace --all-targets` with both servers

Expected: fmt and clippy clean, the documentation builds without a warning, no em dash (`0`), and:

```text
3681 passed, 0 failed, 22 ignored
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan how 3b is cut, and that the log's filters are built"
```

## Self-review

- **Against the design.** D.5's filters and count line; the macOS board's bar (`MacAgentsTabLog`), its three dropdowns, the search and the count; the Omarchy board's `/ filter` (`OmarchyMcpActivity`). The departures are in "What the plan asks, and what is built", each agreed on 2026-10-18 or the board's own.
- **A review before the plan.** A reviewer ran the draft's queries for every mix of the filters and searches with `'`, `\`, `%`, `_` and letters outside English, drove the view in both looks, and typed the tab's letters into the search. It found a result filter taking its own index in the connection's place, the Omarchy filters shown open again with nothing to type in after the view was left from a word, and a test of them run on an empty log. All fixed, with a test each.
- **By eye.** Headless renders of the macOS bar, and of Omarchy's filters open and closed. Not committed.
- **Placeholders.** None: every step has its diff or its command and what it prints.
- **Names.** `LogFilter`, `Page::agents`, `AgentsLog::agent`, `result`, `search`, `agents`, `filters_open`, `filters_asked`, `AgentsLogFilter`, `LOG_RESULTS`, `Action::FilterAgentsLog`, `Action::ShowAgentsLogFilters`, `Command::FilterAgentsLog`, `Command::CloseAgentsLogFilters`, `log_filters`, `terminal_filters` and `words` are the same in every task that uses them.
