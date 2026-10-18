# MCP server, Phase 3b, run 2: the selected call's pane, and Open in SQL editor. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Agents tab's Log view shows the selected call in a pane beside the list: what it ran, with its values, what it came to, and why it was refused or failed. **Open in SQL editor** puts its statement in a new SQL editor of its connection, read-only and not run, its values in a comment above it. On macOS the pane shows while a call is selected; on Omarchy Enter shows it, Esc closes it and `o` opens the statement.

**Architecture:** `App::open_agents_log_call` (`src/app.rs`) finds the window of the call's connection on its saved database (this one, another open, or a new one that connects), opens a SQL editor there with `logged_statement`'s text, read-only, in front. `LoggedValues` (`src/model.rs`) reads a call's values as the log keeps them, each one's JSON untouched. `src/ui/agents.rs` draws the pane (`log_pane`, `log_facts`) with the statement in its dialect's colours (`sql_text::highlighted`). The keymap's `When::AgentsLog` gives the Log view its own Enter and `o`.

**Tech Stack:** Rust 2024, egui, `serde_json` (its `raw_value` feature).

---

## Where this sits

Phase 3b of `docs/superpowers/plans/2026-10-13-mcp-server.md`, the Log view in full, in three runs. Run 1 (pull request 163) built the filters and the search. This run, 2, is the selected call's pane and **Open in SQL editor**. Run 3 is **Export…** and the website's guide to the log. Phase 2, approved writes, comes after 3b, and adds a write's lines to the pane.

## What the plan asks, and what is built

| The plan | Built | Why |
|---|---|---|
| D.5: the detail pane for the selected call | The macOS board's pane beside the list while a call is selected, in the place of Phase 1b's line under it | As drawn. Agreed on 2026-10-18 |
| The Omarchy board: `enter details`, no pane drawn | Enter shows the same pane beside the list and closes it, Esc closes it; the line under the list stays while it is closed | Agreed on 2026-10-18 |
| D.5: "Open in SQL editor: opens a SQL tab with the statement, in Read-only mode, not run" | The statement as it ran, after a comment line for each of its values (`-- $1 = 9100000000000000004`) | The SQL in the log, with what it ran with. Agreed on 2026-10-18 |
| D.5: "in that connection's window for another" | A window of it on its saved database; one opened and connected where none is; one whose session ended connects again | A window switched to another database is not where the call ran (found by the review) |
| D.5, for a write: the rows changed against the dry run, the wait, who approved, the reason | Not yet | Writes come with Phase 2 |

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`. The Agents tab shows only where agents can reach the app (`cfg(any(unix, windows))`). The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.

## What was run before this plan was written

Every task's state was built here, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` with both test servers (3,712 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. Not run: CI, and the pane by hand against a real log: its place and its lines were looked at by eye in headless renders of both looks.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Where the pane shows | macOS: beside the list while a call is selected, in the place of the line under it. Omarchy: beside the list while Enter has it open; Esc closes it. At most half the view | Agreed on 2026-10-18; the list keeps room (found by the review) |
| What the pane says | The tool and how it ended; the agent, the moment, and the connection under All connections; the statement in its dialect's colours; its values, the tables asked for, what it returned, the columns masked, why and its code, how it ran, how long, the project; that rows are not logged | D.5, for a read; a write's lines come with writes |
| Where Open in SQL editor opens | A new SQL editor, read-only, not run, in front, in a window of the call's connection on its saved database: this one, else one open, else one opened and connected; one whose session ended connects again | D.5; found by the review |
| What it holds | A comment line for each value, named as the driver names a parameter (`$1`, `?1`, `? #1`), the value's JSON as the log keeps it, then the statement as it ran. Values cut, or that do not read, are said to be | Agreed on 2026-10-18; a number is not rounded (found by the review) |
| A call on a connection deleted since | Its button is off, and says why; `o` does nothing and is not named | |
| The Omarchy keys | Enter and `o` are the Log view's (`When::AgentsLog`): the activity's Enter and `o` keep their own; the tree leaves Enter to the log while a call is selected | |

### What this run leaves out

- A write's lines in the pane, and Open in SQL editor for one: with writes.
- **Export…**: run 3.

## File structure

| File | What changes |
|---|---|
| `Cargo.toml` | `serde_json`'s `raw_value` feature |
| `src/model.rs` | `AgentsLog::details_open`, `LoggedValues`, the actions |
| `src/app.rs` | `open_agents_log_call`, `logged_statement`; `new_sql_tab` gives its id |
| `src/ui/agents.rs` | `log_pane`, `log_facts`, `LogCall`; the list beside the pane |
| `src/ui/sql_text.rs` | `highlighted`: a statement in its dialect's colours, not edited |
| `src/keymap.rs`, `src/ui/keys.rs`, `src/ui/workspace.rs` | Enter, `o` and Esc in the Log view; the tree leaves Enter to it; the footer |
| `src/ui/agents_tests.rs` | The pane's tests |
| `docs/_guide/agents.md`, `docs/_guide/omarchy.md` | The pane and Open in SQL editor; the keys |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | D.5 and G as built |

---

### Task 1: A call's statement opens in a SQL editor of its connection

- `App::open_agents_log_call(tab, id)`: the call's connection, still saved; a window of it on its saved database (this one, else one open, else a new one that connects; one whose session ended connects again); a new SQL editor there, read-only, in front, holding `logged_statement`'s text. Nothing runs. A call with no statement, or on a connection deleted since, opens nothing. `new_sql_tab` gives the new editor's id.
- `logged_statement(driver, sql, params)`: a comment line for each value, named as the driver names a parameter (`-- $1 = …`, `-- ?1 = …`, `-- ? #1 = …`), then the statement. `LoggedValues` reads the values as the log keeps them, each one's JSON untouched (`serde_json`'s `raw_value`), or says they were cut or do not read.
- `AgentsLog::details_open`, `Action::ShowAgentsLogDetails` and `Action::OpenAgentsLogCall`. The view left, or a filter that clears the selection, closes the pane.

**Files:**
- Modify: `Cargo.toml`, `src/model.rs`, `src/app.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/Cargo.toml b/Cargo.toml
index bf4cb6c..09ca58c 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -41,7 +41,7 @@ anyhow = "1"
 # Command-line flags: --verbose and the demo/screenshot flags.
 clap = { version = "4.5", features = ["derive"] }
 serde = { version = "1", features = ["derive"] }
-serde_json = "1"
+serde_json = { version = "1", features = ["raw_value"] }
 log = "0.4"
 # Platform config/state directories (XDG on Linux).
 directories = "6"
diff --git a/src/app.rs b/src/app.rs
index 19e6483..6121024 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -7715,6 +7715,156 @@ mod tests {
         assert!(log.all);
     }
 
+    #[test]
+    fn a_calls_statement_is_written_after_its_values_as_its_driver_names_them() {
+        let sql = "SELECT * FROM books WHERE id = $1";
+        // Each value on a comment line of its own: a line break in a string
+        // is an escape in it.
+        let params = Some(r#"[7,"a\nb",null]"#);
+        assert_eq!(
+            logged_statement(Driver::Postgres, sql, params),
+            "-- $1 = 7\n-- $2 = \"a\\nb\"\n-- $3 = null\nSELECT * FROM books WHERE id = $1"
+        );
+        assert!(logged_statement(Driver::Sqlite, sql, params).starts_with("-- ?1 = 7\n-- ?2 = "));
+        assert!(
+            logged_statement(Driver::MySql, sql, params).starts_with("-- ? #1 = 7\n-- ? #2 = ")
+        );
+        // None, or none given: the statement alone.
+        assert_eq!(logged_statement(Driver::Postgres, sql, None), sql);
+        assert_eq!(logged_statement(Driver::Postgres, sql, Some("[]")), sql);
+        // A number as the agent wrote it, not read and written again.
+        assert_eq!(
+            logged_statement(
+                Driver::Postgres,
+                sql,
+                Some("[123456789012345678901234567890, 1.10, 1e400]")
+            ),
+            format!("-- $1 = 123456789012345678901234567890\n-- $2 = 1.10\n-- $3 = 1e400\n{sql}")
+        );
+        // Values that do not read are said not to.
+        assert_eq!(
+            logged_statement(Driver::Postgres, sql, Some("[1,")),
+            format!("-- Its values could not be read from the log.\n{sql}")
+        );
+        // Values the log cut are said to be.
+        let cut = Some(r#"{"cut":"[1,2","bytes":204800}"#);
+        assert_eq!(
+            logged_statement(Driver::Postgres, sql, cut),
+            format!("-- Its values, 204800 bytes of them, were cut in the log.\n{sql}")
+        );
+    }
+
+    #[cfg(any(unix, windows))]
+    #[test]
+    fn a_calls_statement_opens_read_only_in_its_connections_window_and_runs_nothing() {
+        let (mut app, _dir) = app();
+        let (tab, _, _) = connect(&mut app);
+        let here = app.workspace(tab).unwrap().conn_id.clone();
+        // Another saved connection, with no window open.
+        let other = with_saved(&mut app);
+        app.apply(Action::ShowAgentsView {
+            tab,
+            view: crate::model::AgentsView::Log,
+        });
+        let call = |id: i64, on: &str, sql: Option<&str>| tabletist_mcp::Logged {
+            id,
+            entry: tabletist_mcp::Entry {
+                agent: "Claude Code".into(),
+                connection_id: Some(on.into()),
+                tool: "run_query".into(),
+                sql: sql.map(str::to_owned),
+                params: Some("[3]".into()),
+                result: "ok".into(),
+                ..tabletist_mcp::Entry::default()
+            },
+        };
+        let sql = "SELECT * FROM books WHERE id = ?1";
+        app.workspace_mut(tab)
+            .unwrap()
+            .agents_tab_mut()
+            .unwrap()
+            .log
+            .calls = vec![
+            call(1, &here.0, Some(sql)),
+            call(2, &other.0, Some(sql)),
+            call(3, "deleted", Some(sql)),
+            call(4, &here.0, None),
+        ];
+        let sent = app.backend.sent.len();
+        // On this connection: in this window, in front, read-only.
+        app.apply(Action::OpenAgentsLogCall { tab, id: 1 });
+        let workspace = app.workspace(tab).unwrap();
+        let editor = workspace
+            .active_tab
+            .and_then(|id| workspace.sql_tab(id))
+            .expect("an editor in front");
+        assert_eq!(editor.text, format!("-- ?1 = 3\n{sql}"));
+        assert_eq!(editor.mode, crate::model::RunMode::ReadOnly);
+        assert_eq!(app.active_tab_id(), tab);
+        // Nothing was run.
+        assert!(
+            !app.backend.sent[sent..]
+                .iter()
+                .any(|command| matches!(command, Command::RunSql { .. })),
+            "{:?}",
+            &app.backend.sent[sent..]
+        );
+        // On another: in its window, opened and connected for it.
+        let windows = app.tabs.len();
+        app.apply(Action::OpenAgentsLogCall { tab, id: 2 });
+        assert_eq!(app.tabs.len(), windows + 1);
+        let opened = app.active_tab_id();
+        let workspace = app.workspace(opened).unwrap();
+        assert_eq!(workspace.conn_id, other);
+        let editor = workspace
+            .active_tab
+            .and_then(|id| workspace.sql_tab(id))
+            .expect("an editor in its window");
+        assert_eq!(editor.mode, crate::model::RunMode::ReadOnly);
+        assert!(editor.text.ends_with(sql));
+        // Again: in that window, which is open now.
+        app.apply(Action::ActivateConnTab(tab));
+        app.apply(Action::OpenAgentsLogCall { tab, id: 2 });
+        assert_eq!(app.tabs.len(), windows + 1);
+        assert_eq!(app.active_tab_id(), opened);
+        // A window of it whose session ended connects again, and takes it.
+        app.workspace_mut(opened).unwrap().status =
+            SessionStatus::Disconnected(tabletist_db::Error::ConnectionLost("gone".into()));
+        app.apply(Action::ActivateConnTab(tab));
+        let sent = app.backend.sent.len();
+        app.apply(Action::OpenAgentsLogCall { tab, id: 2 });
+        assert_eq!((app.tabs.len(), app.active_tab_id()), (windows + 1, opened));
+        assert!(matches!(
+            app.workspace(opened).unwrap().status,
+            SessionStatus::Connecting { .. }
+        ));
+        assert!(
+            app.backend.sent[sent..]
+                .iter()
+                .any(|command| matches!(command, Command::Connect { .. }))
+        );
+        // A window of it switched to another database is not where the call
+        // ran: another window opens, on the saved one.
+        app.workspace_mut(opened).unwrap().spec.database = "other".into();
+        app.apply(Action::ActivateConnTab(tab));
+        app.apply(Action::OpenAgentsLogCall { tab, id: 2 });
+        let windows = windows + 1;
+        assert_eq!(app.tabs.len(), windows + 1);
+        let saved = app.connections.get(&other).unwrap().spec.database.clone();
+        assert_eq!(
+            app.workspace(app.active_tab_id()).unwrap().spec.database,
+            saved
+        );
+        // On a connection deleted since, or with no statement: nothing.
+        app.apply(Action::ActivateConnTab(tab));
+        let editors = |app: &App| app.workspace(tab).unwrap().sql_tabs().count();
+        let before = editors(&app);
+        app.apply(Action::OpenAgentsLogCall { tab, id: 3 });
+        app.apply(Action::OpenAgentsLogCall { tab, id: 4 });
+        assert_eq!((editors(&app), app.tabs.len()), (before, windows + 1));
+        assert_eq!(app.active_tab_id(), tab);
+    }
+
     #[cfg(any(unix, windows))]
     #[test]
     fn a_filter_set_reads_the_logs_first_page_again_with_it() {
@@ -7732,6 +7882,7 @@ mod tests {
         // The agents that called come with the first page.
         assert_eq!(agents_log(&app, tab).agents, ["Claude Code"]);
         app.apply(Action::SelectAgentsLogCall { tab, id: 499 });
+        app.apply(Action::ShowAgentsLogDetails { tab, open: true });
         let filter = |app: &App| {
             agents_sent(app)
                 .into_iter()
@@ -7765,6 +7916,8 @@ mod tests {
         };
         assert_eq!(filter(&app), (wanted.clone(), None));
         assert_eq!(agents_log(&app, tab).selected, None);
+        // And the call's pane, which nothing selected shows, is closed.
+        assert!(!agents_log(&app, tab).details_open);
         // The same again reads nothing.
         let before = reads(&app);
         set(&mut app, AgentsLogFilter::Search("books".into()));
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- a_calls_statement a_filter_set_reads`

Expected: it does not build:

```text
error[E0425]: cannot find function `logged_statement` in this scope
error[E0599]: no variant named `OpenAgentsLogCall` found for enum `model::Action`
error[E0599]: no variant named `ShowAgentsLogDetails` found for enum `model::Action`
error[E0609]: no field `details_open` on type `AgentsLog`
error: could not compile `tabletist` (lib test) due to 17 previous errors
```

- [ ] **Step 3: Open in SQL editor**

````diff
diff --git a/src/app.rs b/src/app.rs
index 6121024..d58fbda 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1721,7 +1721,9 @@ impl App {
                     self.cancel(session, pending);
                 }
             }
-            Action::NewSqlTab(tab) => self.new_sql_tab(tab),
+            Action::NewSqlTab(tab) => {
+                self.new_sql_tab(tab);
+            }
             Action::ShowAgents(tab) => self.show_agents(tab),
             Action::TurnAgentsOn => self.turn_agents_on(),
             Action::StopAgentSession(id) => self.stop_agent_session(id),
@@ -1763,6 +1765,7 @@ impl App {
                     .map(|agents| {
                         if agents.view != view {
                             agents.log.filters_open = false;
+                            agents.log.details_open = false;
                         }
                         agents.view = view;
                     });
@@ -1802,10 +1805,20 @@ impl App {
                     };
                     if changed {
                         log.selected = None;
+                        log.details_open = false;
                         self.read_agents_log(tab, false);
                     }
                 }
             }
+            Action::ShowAgentsLogDetails { tab, open } => {
+                if let Some(agents) = self
+                    .workspace_mut(tab)
+                    .and_then(crate::model::Workspace::agents_tab_mut)
+                {
+                    agents.log.details_open = open;
+                }
+            }
+            Action::OpenAgentsLogCall { tab, id } => self.open_agents_log_call(tab, id),
             Action::ShowAgentsLogFilters { tab, open } => {
                 if let Some(agents) = self
                     .workspace_mut(tab)
@@ -4841,14 +4854,13 @@ impl App {
     }
 
     /// Opens an empty SQL editor after the workspace's other tabs, with the
-    /// limit and timeout of the settings, and shows it.
-    fn new_sql_tab(&mut self, tab: ConnTabId) {
+    /// limit and timeout of the settings, and shows it: its id, where `tab`
+    /// is a workspace.
+    fn new_sql_tab(&mut self, tab: ConnTabId) -> Option<TabId> {
         let id = TabId(self.next_id());
         let request = RequestId(self.next_id());
         let (limit, timeout) = (self.settings.sql_limit, self.settings.sql_timeout());
-        let Some(workspace) = self.workspace_mut(tab) else {
-            return;
-        };
+        let workspace = self.workspace_mut(tab)?;
         workspace.push_sql_tab(id, limit, timeout);
         workspace.active_tab = Some(id);
         workspace.pane = Pane::Grid;
@@ -4862,6 +4874,79 @@ impl App {
             self.backend
                 .send(Command::ServerVersion { session, request });
         }
+        Some(id)
+    }
+
+    /// The statement of the call `id` of the Log view of `tab`, in a new
+    /// SQL editor of its connection: in this window for a call on this
+    /// connection, else in that connection's, which is opened, and
+    /// connected, where none is. The editor is read-only and nothing is
+    /// run. Its values, where it has any, are in a comment above it. A call
+    /// with no statement, or on a connection deleted since, opens nothing.
+    fn open_agents_log_call(&mut self, tab: ConnTabId, id: i64) {
+        let Some(entry) = self
+            .workspace(tab)
+            .and_then(|workspace| {
+                workspace.tabs.iter().find_map(|open| match open {
+                    Tab::Agents(agents) => Some(agents),
+                    Tab::Object(_) | Tab::Sql(_) => None,
+                })
+            })
+            .and_then(|agents| agents.log.calls.iter().find(|logged| logged.id == id))
+            .map(|logged| logged.entry.clone())
+        else {
+            return;
+        };
+        let (Some(sql), Some(on)) = (entry.sql, entry.connection_id) else {
+            return;
+        };
+        let conn = crate::connections::ConnectionId(on);
+        let Some(saved) = self.connections.get(&conn).cloned() else {
+            return;
+        };
+        // A window of the connection on its saved database, this one first:
+        // one switched to another database is not where the call ran.
+        let on_it = |workspace: &Workspace| {
+            workspace.conn_id == conn && workspace.spec.database == saved.spec.database
+        };
+        let shown = match self.workspace(tab).is_some_and(on_it) {
+            true => Some(tab),
+            false => self
+                .open_connections()
+                .find(|(_, workspace)| on_it(workspace))
+                .map(|(shown, _)| shown),
+        };
+        let target = match shown {
+            Some(shown) => {
+                // One whose session ended, or was never made, connects again.
+                let ended = self.workspace(shown).is_some_and(|workspace| {
+                    matches!(
+                        workspace.status,
+                        SessionStatus::Disconnected(_) | SessionStatus::Cancelled
+                    )
+                });
+                if ended {
+                    self.reconnect(shown);
+                }
+                shown
+            }
+            None => {
+                let picker = self.picker_tab();
+                let id = picker.id;
+                self.tabs.push(picker);
+                self.connect_tab(id, saved.clone(), Secrets::default());
+                id
+            }
+        };
+        let text = logged_statement(saved.spec.driver, &sql, entry.params.as_deref());
+        let Some(id) = self.new_sql_tab(target) else {
+            return;
+        };
+        if let Some(editor) = self.sql_tab_mut(target, id) {
+            editor.text = text;
+            editor.mode = crate::model::RunMode::ReadOnly;
+        }
+        self.apply(Action::ActivateConnTab(target));
     }
 
     /// Asks the backend for the names a completion list needs and the
@@ -6022,6 +6107,42 @@ fn row_fields(
     }
 }
 
+/// What Open in SQL editor writes of a call: a comment line for each of
+/// its values, named as `driver` names a statement's parameters, then the
+/// statement as it ran. Each value is written as the log has it, JSON on
+/// its line: a string's line breaks are escapes in it, and a number is
+/// not read and written again. Values the log cut, or that do not read,
+/// are said to be.
+fn logged_statement(driver: Driver, sql: &str, params: Option<&str>) -> String {
+    use crate::model::LoggedValues;
+    let mut text = String::new();
+    match params.map(LoggedValues::of) {
+        Some(LoggedValues::Each(values)) => {
+            for (at, value) in values.iter().enumerate() {
+                let place = at + 1;
+                let named = match driver {
+                    Driver::Postgres => format!("${place}"),
+                    Driver::Sqlite => format!("?{place}"),
+                    Driver::MySql => format!("? #{place}"),
+                };
+                text.push_str(&format!("-- {named} = {value}\n"));
+            }
+        }
+        Some(LoggedValues::Cut(Some(bytes))) => {
+            text.push_str(&format!(
+                "-- Its values, {bytes} bytes of them, were cut in the log.\n"
+            ));
+        }
+        Some(LoggedValues::Cut(None)) => text.push_str("-- Its values were cut in the log.\n"),
+        Some(LoggedValues::Unread) => {
+            text.push_str("-- Its values could not be read from the log.\n");
+        }
+        None => {}
+    }
+    text.push_str(sql);
+    text
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/src/model.rs b/src/model.rs
index deb6d4d..54d708e 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -658,6 +658,18 @@ pub enum Action {
         tab: ConnTabId,
         open: bool,
     },
+    /// The Log view, in the terminal look: show the selected call's pane,
+    /// or close it.
+    ShowAgentsLogDetails {
+        tab: ConnTabId,
+        open: bool,
+    },
+    /// The Log view: the statement of a call, by its place in the log, in
+    /// a new SQL editor of its connection's window, read-only and not run.
+    OpenAgentsLogCall {
+        tab: ConnTabId,
+        id: i64,
+    },
     /// The Log view: select a call, by its place in the log.
     SelectAgentsLogCall {
         tab: ConnTabId,
@@ -3489,6 +3501,30 @@ pub enum AgentsLogFilter {
     Search(String),
 }
 
+/// A call's values, as the log keeps them (`Entry::params`): each one's
+/// JSON as it came, not read and written again; the length of what the log
+/// cut; or nothing to read.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum LoggedValues {
+    Each(Vec<String>),
+    Cut(Option<u64>),
+    Unread,
+}
+
+impl LoggedValues {
+    pub fn of(params: &str) -> Self {
+        if let Ok(values) = serde_json::from_str::<Vec<&serde_json::value::RawValue>>(params) {
+            return Self::Each(values.iter().map(|value| value.get().to_owned()).collect());
+        }
+        match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(params) {
+            Ok(cut) if cut.contains_key("cut") => {
+                Self::Cut(cut.get("bytes").and_then(serde_json::Value::as_u64))
+            }
+            _ => Self::Unread,
+        }
+    }
+}
+
 /// How a call can end, as the log keeps it: what the result filter
 /// offers. A write's outcomes come with writes.
 pub const LOG_RESULTS: [&str; 6] = ["ok", "refused", "error", "timeout", "cancelled", "stopped"];
@@ -3513,6 +3549,9 @@ pub struct AgentsLog {
     /// How many times they were asked to open: each time, the search takes
     /// the keyboard.
     pub filters_asked: u64,
+    /// The terminal look's pane of the selected call, open beside the
+    /// list. macOS shows it whenever a call is selected.
+    pub details_open: bool,
     pub calls: Vec<tabletist_mcp::Logged>,
     /// How many calls the log holds of what is shown.
     pub total: u64,
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- a_calls_statement a_filter_set_reads`

Expected:

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 2701 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add Cargo.toml src/model.rs src/app.rs
git commit -m "Open a logged call's statement in a read-only SQL editor of its connection"
```

---

### Task 2: The selected call's pane

- `log_pane` and `log_facts` (`src/ui/agents.rs`): the tool and how it ended; the agent and the moment; the statement in its dialect's colours (`sql_text::highlighted`), plain where its connection was deleted; then line by line its values, the tables asked for, what it returned, the columns masked, why and its code, how it ran, how long and the project; that rows are not logged; and **Open in SQL editor**, off with why for a connection deleted since (`LogCall`). In a scroll area, at most half the view.
- macOS: the pane beside the list while a call is selected, in the place of the line under it.
- Omarchy: `ShowAgentsLogDetails` (Enter) and `OpenAgentsLogCall` (`o`), both `When::AgentsLog`; `CloseAgentsLogPane` (Esc, named so from 3b-1's `CloseAgentsLogFilters`) closes the filters, else the pane. The tree leaves Enter to the log while a call is selected. The bar names `enter details · o open in sql tab` where it has the room; the footer names them, `o` only where it opens something.

**Files:**
- Modify: `src/ui/agents.rs`, `src/ui/sql_text.rs`, `src/keymap.rs`, `src/ui/keys.rs`, `src/ui/workspace.rs`, `src/ui/agents_tests.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index 205efd0..2a2e50a 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -864,7 +864,8 @@ fn the_log_lists_the_calls_and_says_what_the_selected_one_came_to() {
             "{}",
             look.name
         );
-        // Selected, it says what it came to, and that rows are not kept.
+        // Selected, it says what it came to, and that rows are not kept:
+        // in the terminal look on its line, on macOS in its pane.
         let request = harness
             .painted_rect("SELECT id FROM books")
             .expect("the request");
@@ -874,7 +875,13 @@ fn the_log_lists_the_calls_and_says_what_the_selected_one_came_to() {
             "Claude Code · {} · 3 rows, 2 columns returned · 4 ms · rows themselves are not logged",
             crate::clock::moment(at),
         ));
-        assert!(harness.has(&detail), "{}", look.name);
+        assert_eq!(harness.has(&detail), look.terminal, "{}", look.name);
+        assert_eq!(
+            harness.painted_rect("3 rows, 2 columns").is_some(),
+            !look.terminal,
+            "{}",
+            look.name
+        );
         // Every connection's calls.
         if look.terminal {
             super::tests::type_key(&mut harness, egui::Key::L, "L");
@@ -1080,6 +1087,159 @@ fn omarchys_slash_opens_the_logs_filters_and_esc_keeps_them() {
     assert_eq!(view(&harness), Some((AgentsView::Log, true)));
 }
 
+/// The text of the SQL editor in front of `tab`'s window, if one is.
+fn editor_in_front(harness: &Harness, tab: ConnTabId) -> Option<(String, crate::model::RunMode)> {
+    let workspace = harness.app.workspace(tab)?;
+    let sql = workspace.active_tab.and_then(|id| workspace.sql_tab(id))?;
+    Some((sql.text.clone(), sql.mode))
+}
+
+#[test]
+fn the_selected_calls_pane_says_what_it_came_to_and_opens_its_statement() {
+    let (mut harness, tab) = on_the_log(Look::macos());
+    // No call selected: no pane.
+    assert!(
+        harness
+            .painted_rect("Rows themselves are not logged.")
+            .is_none()
+    );
+    let request = harness
+        .painted_rect("SELECT id FROM books")
+        .expect("the request");
+    super::tests::click_at(&mut harness, request.center());
+    harness.settle();
+    for said in [
+        "Returned",
+        "3 rows, 2 columns",
+        "Took",
+        "4 ms",
+        "Rows themselves are not logged.",
+    ] {
+        assert!(harness.painted_rect(said).is_some(), "{said}");
+    }
+    // A narrow window keeps half of it for the list.
+    let wide = harness.size;
+    harness.size = egui::vec2(560.0, 500.0);
+    harness.settle();
+    let pane = harness.painted_rect("Returned").expect("the pane");
+    let request = harness
+        .text_rects
+        .iter()
+        .find(|(text, _)| text.starts_with("SE") && text.ends_with("…"))
+        .map(|(_, rect)| *rect)
+        .expect("the list's request, cut short");
+    assert!(request.right() < pane.left(), "{request:?} {pane:?}");
+    harness.size = wide;
+    harness.settle();
+    // Its statement, as it ran, in a SQL editor in front: read-only.
+    harness.click("Open in SQL editor");
+    harness.settle();
+    assert_eq!(
+        editor_in_front(&harness, tab),
+        Some((
+            "SELECT  id\n  FROM books".to_owned(),
+            crate::model::RunMode::ReadOnly
+        ))
+    );
+    // A call on a connection deleted since opens nothing.
+    harness.app.apply(Action::ShowAgentsView {
+        tab,
+        view: crate::model::AgentsView::Log,
+    });
+    harness.settle();
+    let log = &mut harness
+        .app
+        .workspace_mut(tab)
+        .unwrap()
+        .agents_tab_mut()
+        .unwrap()
+        .log;
+    log.calls[0].entry.connection_id = Some("deleted".into());
+    log.selected = Some(log.calls[0].id);
+    harness.settle();
+    let editors = harness.app.workspace(tab).unwrap().sql_tabs().count();
+    harness.click("Open in SQL editor");
+    harness.settle();
+    assert_eq!(
+        harness.app.workspace(tab).unwrap().sql_tabs().count(),
+        editors
+    );
+}
+
+#[test]
+fn omarchys_enter_shows_the_calls_pane_esc_closes_it_and_o_opens_its_statement() {
+    let (mut harness, tab) = on_the_log(Look::omarchy());
+    let open = |harness: &Harness| {
+        harness
+            .app
+            .workspace(tab)
+            .unwrap()
+            .tabs
+            .iter()
+            .find_map(|open| match open {
+                crate::model::Tab::Agents(agents) => Some(agents.log.details_open),
+                _ => None,
+            })
+    };
+    super::tests::type_key(&mut harness, egui::Key::J, "j");
+    harness.settle();
+    // The bar names the keys of the call selected, where it has the room;
+    // a narrow window leaves them to the footer.
+    let hint = "/ filter · enter details · o open in sql tab";
+    let narrow = harness.size;
+    harness.size = egui::vec2(1600.0, narrow.y);
+    harness.settle();
+    assert!(harness.painted_rect(hint).is_some());
+    harness.size = egui::vec2(900.0, narrow.y);
+    harness.settle();
+    assert!(harness.painted_rect(hint).is_none());
+    assert!(harness.painted_rect("enter details").is_some());
+    harness.size = narrow;
+    harness.settle();
+    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+    harness.settle();
+    assert_eq!(open(&harness), Some(true));
+    assert!(harness.painted_rect("3 rows, 2 columns").is_some());
+    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
+    harness.settle();
+    assert_eq!(open(&harness), Some(false));
+    assert!(harness.painted_rect("3 rows, 2 columns").is_none());
+    // On a connection deleted since, `o` is offered nowhere.
+    let open_hint = "o open";
+    assert!(harness.painted_rect(open_hint).is_some());
+    let log = &mut harness
+        .app
+        .workspace_mut(tab)
+        .unwrap()
+        .agents_tab_mut()
+        .unwrap()
+        .log;
+    let kept = log.calls[0].entry.connection_id.replace("deleted".into());
+    harness.settle();
+    assert!(harness.painted_rect(open_hint).is_none());
+    let log = &mut harness
+        .app
+        .workspace_mut(tab)
+        .unwrap()
+        .agents_tab_mut()
+        .unwrap()
+        .log;
+    log.calls[0].entry.connection_id = kept;
+    harness.settle();
+    // Enter again shows it; `o` opens the statement in a SQL editor.
+    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+    harness.settle();
+    super::tests::type_key(&mut harness, egui::Key::O, "o");
+    harness.settle();
+    assert_eq!(
+        editor_in_front(&harness, tab),
+        Some((
+            "SELECT  id\n  FROM books".to_owned(),
+            crate::model::RunMode::ReadOnly
+        ))
+    );
+}
+
 #[test]
 fn omarchy_shows_the_log_with_l_and_the_agents_log_line() {
     let (mut harness, tab) = connected(Look::omarchy());
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib -- the_selected_calls_pane omarchys_enter the_log_lists_the_calls`

Expected: the two new tests fail, as nothing draws the pane yet, and so does the earlier one, which now finds the macOS line gone:

```text
test ui::agents_tests::omarchys_enter_shows_the_calls_pane_esc_closes_it_and_o_opens_its_statement ... FAILED
test ui::agents_tests::the_selected_calls_pane_says_what_it_came_to_and_opens_its_statement ... FAILED
test ui::agents_tests::the_log_lists_the_calls_and_says_what_the_selected_one_came_to ... FAILED
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 2703 filtered out
```

- [ ] **Step 3: The pane**

````diff
diff --git a/src/keymap.rs b/src/keymap.rs
index b084eb0..e8434d1 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -161,6 +161,9 @@ pub enum When {
     /// grid's keys that act on a row have nothing to act on, and the
     /// view's own act on the line of a column.
     Structure,
+    /// The Agents tab shows its Log view: its keys come before the
+    /// activity's.
+    AgentsLog,
 }
 
 /// The mode a chord holds in: with the keyboard in a text field (insert),
@@ -343,7 +346,9 @@ commands! {
     ShowAgentsLog,
     ToggleAgentsLogScope,
     FilterAgentsLog,
-    CloseAgentsLogFilters,
+    CloseAgentsLogPane,
+    ShowAgentsLogDetails,
+    OpenAgentsLogCall,
     AllowAgents,
     ShowAgentSettings,
     ChangeAgentAccess,
@@ -621,9 +626,21 @@ impl Command {
                 "",
                 Window,
             )),
-            C::CloseAgentsLogFilters => {
-                agents(info("Close the log's filters, and keep them", "", Window))
-            }
+            C::CloseAgentsLogPane => agents(info(
+                "Close the log's filters, and keep them, or the call's details",
+                "",
+                Window,
+            )),
+            C::ShowAgentsLogDetails => agents(info(
+                "Show the selected call's details in the log, or close them",
+                "",
+                Window,
+            )),
+            C::OpenAgentsLogCall => agents(info(
+                "Open the selected call's statement in a SQL editor",
+                "",
+                Window,
+            )),
             C::ToggleSidebar => info("Toggle sidebar", "Toggle sidebar", Navigation),
             C::PaneLeft => info("Pane to the left", PANES, Navigation),
             C::PaneDown => info("Pane below", PANES, Navigation).reserved(),
@@ -1035,7 +1052,9 @@ pub const BINDINGS: &[Binding] = {
         bind(C::ShowAgentsLog, AGENTS, NONE, keys(&[k("l")])),
         bind(C::ToggleAgentsLogScope, AGENTS, NONE, keys(&[k("L")])),
         bind(C::FilterAgentsLog, AGENTS, NONE, keys(&[k("/")])),
-        bind(C::CloseAgentsLogFilters, AGENTS, NONE, keys(&[k("esc")])),
+        bind(C::CloseAgentsLogPane, AGENTS, NONE, keys(&[k("esc")])),
+        bind(C::ShowAgentsLogDetails, AGENTS, NONE, keys(&[k("enter")])).when(When::AgentsLog),
+        bind(C::OpenAgentsLogCall, AGENTS, NONE, keys(&[k("o")])).when(When::AgentsLog),
         bind(C::AllowAgents, AGENTS, NONE, keys(&[k("e")])),
         bind(C::ShowAgentSettings, AGENTS, NONE, keys(&[k("s")])),
         bind(C::ChangeAgentAccess, CONNECTIONS, NONE, keys(&[k("a")])),
diff --git a/src/ui/agents.rs b/src/ui/agents.rs
index 928a78a..f26829c 100644
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -104,7 +104,29 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .key(layout, crate::keymap::Command::FilterAgentsLog);
     let close_key = app
         .keymap
-        .key(layout, crate::keymap::Command::CloseAgentsLogFilters);
+        .key(layout, crate::keymap::Command::CloseAgentsLogPane);
+    let details_key = app
+        .keymap
+        .key(layout, crate::keymap::Command::ShowAgentsLogDetails);
+    let open_key = app
+        .keymap
+        .key(layout, crate::keymap::Command::OpenAgentsLogCall);
+    // The selected call's connection, as it is saved now: whether its
+    // statement opens, and the dialect it is read in.
+    let call = log
+        .selected
+        .and_then(|id| log.calls.iter().find(|logged| logged.id == id))
+        .map(|logged| {
+            let saved = logged.entry.connection_id.as_ref().and_then(|id| {
+                app.connections
+                    .get(&crate::connections::ConnectionId(id.clone()))
+            });
+            LogCall {
+                open: logged.entry.sql.as_ref().map(|_| saved.is_some()),
+                dialect: saved.map(|saved| saved.spec.driver.dialect()),
+            }
+        })
+        .unwrap_or_default();
     let show_key = app.keymap.key(layout, crate::keymap::Command::ShowAgents);
     let copy_key = app
         .keymap
@@ -233,8 +255,15 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     scope: &scope_key,
                     filter: &filter_key,
                     close: &close_key,
+                    details: &details_key,
+                    open: &open_key,
                 };
-                let shown = log_view(ui, rect, (log, keys, days), (&look, &palette, locale));
+                let shown = log_view(
+                    ui,
+                    rect,
+                    (log, keys, days, call),
+                    (&look, &palette, locale),
+                );
                 match shown {
                     Some(LogAsk::Scope) => actions.push(Action::ToggleAgentsLogScope(tab)),
                     Some(LogAsk::Filter(filter)) => {
@@ -243,6 +272,12 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     Some(LogAsk::CloseFilters) => {
                         actions.push(Action::ShowAgentsLogFilters { tab, open: false });
                     }
+                    Some(LogAsk::CloseDetails) => {
+                        actions.push(Action::ShowAgentsLogDetails { tab, open: false });
+                    }
+                    Some(LogAsk::Open(id)) => {
+                        actions.push(Action::OpenAgentsLogCall { tab, id });
+                    }
                     Some(LogAsk::More) => actions.push(Action::MoreAgentsLog(tab)),
                     Some(LogAsk::Select(id)) => {
                         actions.push(Action::SelectAgentsLogCall { tab, id });
@@ -1015,6 +1050,10 @@ enum LogAsk {
     Filter(crate::model::AgentsLogFilter),
     /// The terminal look's filters closed, and kept.
     CloseFilters,
+    /// The terminal look's pane of the selected call closed.
+    CloseDetails,
+    /// The call's statement opened in a SQL editor, by its place.
+    Open(i64),
 }
 
 /// The Log view's bar at its top (the terminal's, and macOS's, which holds
@@ -1024,6 +1063,10 @@ const LOG_FILTER_BAR: f32 = 48.0;
 const LOG_DETAIL: f32 = 30.0;
 /// A row of the terminal look's filters.
 const FILTER_ROW: f32 = 28.0;
+/// The selected call's pane, beside the list: the board's on macOS, a
+/// mono column's more in the terminal.
+const LOG_PANE: f32 = 320.0;
+const LOG_PANE_TERMINAL: f32 = 380.0;
 
 /// The Log view: whose calls (this connection's or every one's) and how
 /// many the log keeps, then the calls the newest first, and under them
@@ -1031,7 +1074,7 @@ const FILTER_ROW: f32 = 28.0;
 fn log_view(
     ui: &mut egui::Ui,
     rect: Rect,
-    (log, keys, days): (&crate::model::AgentsLog, LogKeys<'_>, u32),
+    (log, keys, days, call): (&crate::model::AgentsLog, LogKeys<'_>, u32, LogCall),
     (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
 ) -> Option<LogAsk> {
     let mut ask = None;
@@ -1078,16 +1121,41 @@ fn log_view(
             )
             .add(body, keys.scope.as_str(), palette.accent)
             .add(body, &format!(" {other}"), palette.secondary);
-        widgets::paint_text(ui, bar.left() + 14.0, bar.center().y, line);
-        // At its right, `/` for the filters.
-        let filter = Text::new(look)
+        let used = widgets::paint_text(ui, bar.left() + 14.0, bar.center().y, line);
+        // At its right, `/` for the filters, and with a call selected its
+        // details and its statement in a SQL editor.
+        let mut filter = Text::new(look)
             .add(body, keys.filter.as_str(), palette.accent)
             .add(
                 body,
                 &format!(" {}", look.label(&gettext(locale, "filter"))),
                 palette.secondary,
             );
-        widgets::paint_text_right(ui, bar.right() - 14.0, bar.center().y, filter);
+        if log.selected.is_some() {
+            filter = filter
+                .add(body, " · ", palette.secondary)
+                .add(body, keys.details.as_str(), palette.accent)
+                .add(
+                    body,
+                    &format!(" {}", look.label(&gettext(locale, "details"))),
+                    palette.secondary,
+                );
+            if call.open == Some(true) {
+                filter = filter
+                    .add(body, " · ", palette.secondary)
+                    .add(body, keys.open.as_str(), palette.accent)
+                    .add(
+                        body,
+                        &format!(" {}", look.label(&gettext(locale, "open in sql tab"))),
+                        palette.secondary,
+                    );
+            }
+        }
+        // Only where it has the room: the footer names the keys too.
+        let laid = filter.layout(ui.ctx());
+        if bar.left() + 14.0 + used + 24.0 + laid.width() <= bar.right() - 14.0 {
+            laid.paint_right(ui.painter(), bar.right() - 14.0, bar.center().y);
+        }
         widgets::announce(ui, bar, &look.label(&format!("{scope} · {count}")));
     } else {
         let row = Rect::from_min_max(
@@ -1126,11 +1194,34 @@ fn log_view(
             &mut ask,
         );
     }
-    // The selected call's line, at the bottom.
-    let detail = Rect::from_min_max(pos2(rect.left(), rect.bottom() - LOG_DETAIL), rect.max);
     let chosen = log
         .selected
         .and_then(|id| log.calls.iter().find(|logged| logged.id == id));
+    // The selected call's pane, at the right: on macOS whenever a call is
+    // selected, in the terminal while it is open.
+    let mut right = rect.right();
+    if let Some(logged) = chosen
+        && (!look.terminal || log.details_open)
+    {
+        // At most half the view: the list keeps the rest.
+        let width = if look.terminal {
+            LOG_PANE_TERMINAL
+        } else {
+            LOG_PANE
+        };
+        let width = width.min(rect.width() / 2.0);
+        let pane = Rect::from_min_max(pos2(rect.right() - width, top), rect.max);
+        right = pane.left();
+        if let Some(asked) = log_pane(ui, pane, logged, (log.all, call), (look, palette, locale)) {
+            ask = Some(asked);
+        }
+        if look.terminal && !log.filters_open && closed(ui, look) {
+            ask = Some(LogAsk::CloseDetails);
+        }
+    }
+    // The selected call's line, at the bottom, where no pane shows it.
+    let detail = Rect::from_min_max(pos2(rect.left(), rect.bottom() - LOG_DETAIL), rect.max);
+    let chosen = chosen.filter(|_| right == rect.right());
     if let Some(logged) = chosen {
         ui.painter().rect_filled(detail, 0.0, palette.panel);
         widgets::hline(ui, detail.x_range(), detail.top() + 0.5, palette.border);
@@ -1152,7 +1243,7 @@ fn log_view(
     } else {
         rect.bottom()
     };
-    let list = Rect::from_min_max(pos2(rect.left(), top), pos2(rect.right(), bottom));
+    let list = Rect::from_min_max(pos2(rect.left(), top), pos2(right, bottom));
     let row = feed_row(look);
     let empty = match (&log.failed, log.read, log.all) {
         (Some(failed), _, _) => Some(format!(
@@ -1348,8 +1439,231 @@ pub(crate) struct LogKeys<'a> {
     pub scope: &'a crate::keymap::Written,
     /// The terminal look's filters, opened.
     pub filter: &'a crate::keymap::Written,
-    /// And closed.
+    /// And closed, and the selected call's pane with them.
     pub close: &'a crate::keymap::Written,
+    /// The terminal look's pane of the selected call, shown.
+    pub details: &'a crate::keymap::Written,
+    /// The selected call's statement, in a SQL editor.
+    pub open: &'a crate::keymap::Written,
+}
+
+/// What the Log view is told of the selected call by the app: whether its
+/// statement opens in a SQL editor (`None` for a call with none, `Some(false)`
+/// for one on a connection deleted since), and the dialect it is read in.
+#[derive(Clone, Copy, Default)]
+pub(crate) struct LogCall {
+    pub open: Option<bool>,
+    pub dialect: Option<tabletist_db::Dialect>,
+}
+
+/// Whether the keymap's close of the log's panes was pressed this frame.
+fn closed(ui: &egui::Ui, look: &Look) -> bool {
+    let keymap = crate::ui::keys::published(ui.ctx());
+    let layout = crate::keymap::Layout::of(look);
+    ui.input_mut(|input| {
+        crate::ui::keys::asked(
+            input,
+            &keymap,
+            layout,
+            crate::keymap::Command::CloseAgentsLogPane,
+            crate::keymap::Scope::Agents,
+            true,
+        )
+        .count
+            > 0
+    })
+}
+
+/// The selected call's pane, in `rect`: the tool and how it ended, who
+/// called and when, the request (a statement in its dialect's colours, or
+/// the tables asked for), and what it came to, line by line; that rows are
+/// never logged; and Open in SQL editor where it has a statement, off for
+/// a connection deleted since. What was asked of it.
+fn log_pane(
+    ui: &mut egui::Ui,
+    rect: Rect,
+    logged: &tabletist_mcp::Logged,
+    (all, call): (bool, LogCall),
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+) -> Option<LogAsk> {
+    let entry = &logged.entry;
+    let mut ask = None;
+    ui.painter().rect_filled(rect, 0.0, palette.panel);
+    widgets::vline(ui, rect.left() + 0.5, rect.y_range(), palette.border);
+    let body = widgets::body(look);
+    let small = widgets::secondary(look);
+    let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OGroup);
+    let inner = rect.shrink2(vec2(16.0, 14.0));
+    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
+    egui::ScrollArea::vertical()
+        .id_salt("agents-log-pane")
+        .auto_shrink([false, false])
+        .show(&mut child, |ui| {
+            ui.spacing_mut().item_spacing.y = 10.0;
+            let width = ui.available_width();
+            // "run_query  ok".
+            let (word, color) = log_result(&entry.result, palette, locale);
+            Text::new(look)
+                .add(strong, &entry.tool, palette.text)
+                .space(body, "  ")
+                .add(body, &look.label(&word), color)
+                .layout(ui.ctx())
+                .label(ui);
+            // Who, and when to the second, and where under All connections.
+            let at = crate::clock::of_millis(entry.at_ms);
+            let mut who = vec![entry.agent.clone(), crate::clock::moment(at)];
+            if all && let Some(on) = &entry.connection {
+                who.push(on.clone());
+            }
+            Text::one(look, body, &look.label(&who.join(" · ")), palette.secondary)
+                .wrap(width)
+                .layout(ui.ctx())
+                .label(ui);
+            // The statement, as it ran.
+            if let Some(sql) = &entry.sql {
+                let frame = egui::Frame::new()
+                    .fill(palette.window)
+                    .stroke(egui::Stroke::new(1.0, palette.border))
+                    .corner_radius(egui::CornerRadius::same(look.radius))
+                    .inner_margin(egui::Margin::symmetric(10, 8));
+                frame.show(ui, |ui| {
+                    // In its dialect's colours; plain where its connection
+                    // was deleted, and its dialect is not known.
+                    let room = width - 20.0;
+                    let laid = match call.dialect {
+                        Some(dialect) => crate::ui::sql_text::highlighted(
+                            ui.ctx(),
+                            look,
+                            palette,
+                            dialect,
+                            sql,
+                            room,
+                        ),
+                        None => {
+                            let mono = TextRole::pick(look, TextRole::Code, TextRole::OCode);
+                            Text::one(look, mono, sql, palette.text)
+                                .wrap(room)
+                                .layout(ui.ctx())
+                        }
+                    };
+                    laid.label(ui);
+                });
+            }
+            for (label, value) in log_facts(entry, locale) {
+                let label = look.label(&label);
+                ui.horizontal_top(|ui| {
+                    let name = Text::one(look, small, &label, palette.secondary).layout(ui.ctx());
+                    let room = (width - 110.0).max(40.0);
+                    ui.allocate_ui(vec2(110.0, name.galley.size().y), |ui| {
+                        ui.set_min_width(110.0);
+                        name.label(ui);
+                    });
+                    Text::one(look, body, &value, palette.text)
+                        .wrap(room)
+                        .layout(ui.ctx())
+                        .label(ui);
+                });
+            }
+            if entry.tool == "run_query" {
+                let note = gettext(locale, "Rows themselves are not logged.");
+                Text::one(look, small, &look.label(&note), palette.dim)
+                    .wrap(width)
+                    .layout(ui.ctx())
+                    .label(ui);
+            }
+            if let Some(open) = call.open {
+                let label = look.label(&gettext(locale, "Open in SQL editor"));
+                let reason = gettext(locale, "Its connection was deleted.");
+                let mut button = states::button(&label, look);
+                if !open {
+                    button = button.disabled(&reason);
+                }
+                let height = states::button_height(look);
+                if button.show(ui, height, look, palette).clicked() && open {
+                    ask = Some(LogAsk::Open(logged.id));
+                }
+            }
+        });
+    ask
+}
+
+/// What a call came to, line by line, as the pane says it: its values, the
+/// tables asked for, what it returned, the columns masked, how it ran and
+/// for how long; or why it ended as it did; and the agent's project.
+fn log_facts(entry: &tabletist_mcp::Entry, locale: crate::i18n::Locale) -> Vec<(String, String)> {
+    let count = |one: &'static str, many: &'static str, n: i64| {
+        ngettext(locale, one, many, u32::try_from(n).unwrap_or(u32::MAX))
+            .replace("{n}", &n.to_string())
+    };
+    let say = |text: &'static str| gettext(locale, text).into_owned();
+    let mut facts = Vec::new();
+    if let Some(params) = &entry.params {
+        use crate::model::LoggedValues;
+        let value = match LoggedValues::of(params) {
+            LoggedValues::Each(values) => values.join(", "),
+            LoggedValues::Cut(Some(bytes)) => count(
+                "cut in the log, of {n} byte",
+                "cut in the log, of {n} bytes",
+                i64::try_from(bytes).unwrap_or(i64::MAX),
+            ),
+            LoggedValues::Cut(None) => say("cut in the log"),
+            LoggedValues::Unread => say("not readable in the log"),
+        };
+        facts.push((say("Params"), value));
+    }
+    if let Some(tables) = &entry.request {
+        facts.push((say("Tables"), tables.clone()));
+    }
+    if entry.result == "ok" {
+        match (entry.tool.as_str(), entry.rows) {
+            ("describe_schema", Some(tables)) => {
+                facts.push((say("Described"), count("{n} table", "{n} tables", tables)));
+            }
+            ("list_connections", Some(listed)) => {
+                facts.push((
+                    say("Listed"),
+                    count("{n} connection", "{n} connections", listed),
+                ));
+            }
+            (_, Some(rows)) => {
+                let mut returned = count("{n} row", "{n} rows", rows);
+                if let Some(columns) = entry.columns {
+                    returned = format!(
+                        "{returned}, {}",
+                        count("{n} column", "{n} columns", columns)
+                    );
+                }
+                facts.push((say("Returned"), returned));
+            }
+            (_, None) => {}
+        }
+        let masked = entry
+            .masked
+            .as_deref()
+            .and_then(|masked| serde_json::from_str::<Vec<String>>(masked).ok())
+            .filter(|masked| !masked.is_empty());
+        if let Some(masked) = masked {
+            facts.push((say("Masked"), masked.join(", ")));
+        }
+    } else {
+        if let Some(message) = &entry.message {
+            let message = message.split_whitespace().collect::<Vec<_>>().join(" ");
+            facts.push((say("Why"), message));
+        }
+        if let Some(code) = &entry.code {
+            facts.push((say("Code"), code.clone()));
+        }
+    }
+    if let Some(ran) = &entry.ran_as {
+        facts.push((say("Ran as"), ran.clone()));
+    }
+    if let Some(took) = entry.took_ms {
+        facts.push((say("Took"), format!("{took} ms")));
+    }
+    if let Some(project) = &entry.project {
+        facts.push((say("Project"), project.clone()));
+    }
+    facts
 }
 
 /// The macOS look's filters, in a row of `ui`: whose calls, which agent's,
@@ -1637,7 +1951,7 @@ fn terminal_filters(
             input,
             &keymap,
             layout,
-            crate::keymap::Command::CloseAgentsLogFilters,
+            crate::keymap::Command::CloseAgentsLogPane,
             crate::keymap::Scope::Agents,
             true,
         )
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 4fcca3d..f5c044e 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -800,9 +800,11 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
                 }
             }
             // The terminal look's list of sessions has Enter of its own,
-            // unless the person went to the tree.
+            // and so has its log with a call selected, unless the person
+            // went to the tree.
+            let log_call = agents_log(app, tab).is_some_and(|log| log.selected.is_some());
             let list = terminal
-                && agents_lists(app, tab)
+                && (agents_lists(app, tab) || log_call)
                 && app
                     .workspace(tab)
                     .is_some_and(|workspace| workspace.pane != crate::model::Pane::Tree);
@@ -1710,6 +1712,18 @@ fn agents_lists(app: &App, tab: ConnTabId) -> bool {
     listing && !app.agents.listed(&workspace.conn_id).is_empty()
 }
 
+/// The Log view of the Agents tab of `tab`, while it shows.
+fn agents_log(app: &App, tab: ConnTabId) -> Option<&crate::model::AgentsLog> {
+    if agents_view(app, tab) != Some(crate::model::AgentsView::Log) {
+        return None;
+    }
+    let workspace = app.workspace(tab)?;
+    workspace.tabs.iter().find_map(|open| match open {
+        crate::model::Tab::Agents(agents) => Some(&agents.log),
+        crate::model::Tab::Object(_) | crate::model::Tab::Sql(_) => None,
+    })
+}
+
 /// The view of the Agents tab of `tab`, while the tab is in front.
 fn agents_view(app: &App, tab: ConnTabId) -> Option<crate::model::AgentsView> {
     let workspace = app
@@ -2116,6 +2130,9 @@ fn letters(
         // The Structure view shows where the rows would: its own keys
         // come before the rows' there.
         crate::keymap::When::Structure => structure_shows,
+        crate::keymap::When::AgentsLog => {
+            agents_view(app, tab) == Some(crate::model::AgentsView::Log)
+        }
         _ => false,
     };
     let review_only = |when: crate::keymap::When| when == crate::keymap::When::Review;
@@ -2207,6 +2224,30 @@ fn letters(
     {
         actions.push(Action::ShowAgentsOn(conn));
     }
+    // Enter in the Log view shows the selected call's details, and closes
+    // them; never a focused button's.
+    let log = agents_log(app, tab);
+    if let Some(log) = log
+        && log.selected.is_some()
+        && !crate::ui::focus::on_control(ctx)
+        && ctx
+            .input_mut(|input| {
+                asked(
+                    input,
+                    &app.keymap,
+                    layout,
+                    Command::ShowAgentsLogDetails,
+                    Scope::Agents,
+                    true,
+                )
+            })
+            .fresh
+    {
+        actions.push(Action::ShowAgentsLogDetails {
+            tab,
+            open: !log.details_open,
+        });
+    }
     let alone = actions.is_empty() && !pointer && keys <= typed.len() && only_edits;
     // Whether a field or the prompt was asked for: the frame's typing ends
     // there.
@@ -2297,6 +2338,13 @@ fn letters(
             {
                 actions.push(Action::ToggleAgentsLogScope(tab));
             }
+            (Some(Command::OpenAgentsLogCall), _)
+                if agents_view(app, tab) == Some(crate::model::AgentsView::Log) =>
+            {
+                if let Some(id) = log.and_then(|log| log.selected) {
+                    actions.push(Action::OpenAgentsLogCall { tab, id });
+                }
+            }
             (Some(Command::FilterAgentsLog), _)
                 if agents_view(app, tab) == Some(crate::model::AgentsView::Log) =>
             {
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
index 2606415..2b22c3c 100644
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -199,6 +199,28 @@ pub fn layouter(
     }
 }
 
+/// `text` in `dialect`'s colours, as an editor draws it, wrapped at
+/// `width`: a statement shown and not edited.
+pub fn highlighted(
+    ctx: &egui::Context,
+    look: &Look,
+    palette: &Palette,
+    dialect: Dialect,
+    text: &str,
+    width: f32,
+) -> crate::typography::Laid {
+    let tokens = sql::tokenize(dialect, text);
+    let runs = runs(&tokens, text, palette);
+    Text::in_faces(look.faces)
+        .add_runs(
+            role(look),
+            runs.iter()
+                .map(|(range, color)| (&text[range.clone()], *color)),
+        )
+        .wrap(width)
+        .layout(ctx)
+}
+
 /// The colour of the bar beside the statement Run executes: the accent in
 /// the terminal, the splitter grip's grey elsewhere.
 pub fn bar_color(look: &Look, palette: &Palette) -> Color32 {
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index 42b998e..07654cf 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2055,11 +2055,29 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         key(Command::ToggleAgentsLogScope),
         key(Command::FilterAgentsLog),
     );
-    // Its Log view, while it shows, and whether every connection's.
+    let (agents_details_key, agents_open_key) = (
+        key(Command::ShowAgentsLogDetails),
+        key(Command::OpenAgentsLogCall),
+    );
+    // Its Log view, while it shows: whether every connection's, whether a
+    // call is selected, and whether that call's statement opens (it has
+    // one, on a connection still saved).
     let agents_log = app.workspace(tab).and_then(|workspace| {
         workspace.tabs.iter().find_map(|open| match open {
             crate::model::Tab::Agents(agents) if agents.view == crate::model::AgentsView::Log => {
-                Some(agents.log.all)
+                let log = &agents.log;
+                let call = log
+                    .selected
+                    .and_then(|id| log.calls.iter().find(|logged| logged.id == id));
+                let opens = call.is_some_and(|logged| {
+                    logged.entry.sql.is_some()
+                        && logged.entry.connection_id.as_ref().is_some_and(|id| {
+                            app.connections
+                                .get(&crate::connections::ConnectionId(id.clone()))
+                                .is_some()
+                        })
+                });
+                Some((log.all, call.is_some(), opens))
             }
             _ => None,
         })
@@ -2303,10 +2321,12 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 gettext(locale, "all connections"),
                 gettext(locale, "this connection"),
                 gettext(locale, "filter"),
+                gettext(locale, "details"),
+                gettext(locale, "open"),
             ];
             match agents_log {
                 // The Log view's keys in the place of the activity's.
-                Some(all) => {
+                Some((all, selected, opens)) => {
                     let scope = if all { &*log_words[4] } else { &*log_words[3] };
                     agents_hints = vec![
                         (&agents_log_key, &*log_words[1], true),
@@ -2314,6 +2334,12 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                         (&agents_filter_key, &*log_words[5], true),
                         (&agents_move_key, &*log_words[2], true),
                     ];
+                    if selected {
+                        agents_hints.push((&agents_details_key, &*log_words[6], true));
+                    }
+                    if opens {
+                        agents_hints.push((&agents_open_key, &*log_words[7], true));
+                    }
                 }
                 None => agents_hints.push((&agents_log_key, &*log_words[0], true)),
             }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- agents_tests keymap`, then `cargo test --locked --test keys`

Expected:

```text
test result: ok. 71 passed; 0 failed; 0 ignored; 0 measured; 2635 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/ui/agents.rs src/ui/sql_text.rs src/keymap.rs src/ui/keys.rs src/ui/workspace.rs src/ui/agents_tests.rs
git commit -m "Show the selected call's pane in the agents' log, in both looks"
```

---

### Task 3: The website

- `docs/_guide/agents.md`: the pane, and Open in SQL editor with its comment of values.
- `docs/_guide/omarchy.md`: Enter, `o` and Esc in the log.

**Files:**
- Modify: `docs/_guide/agents.md`, `docs/_guide/omarchy.md`

- [ ] **Step 1: The pages**

````diff
diff --git a/docs/_guide/agents.md b/docs/_guide/agents.md
index 88e2b5d..829aff1 100644
--- a/docs/_guide/agents.md
+++ b/docs/_guide/agents.md
@@ -228,6 +228,19 @@ list, with the keyboard in the search: Tab moves to the agents and the
 results, Enter or Space picks one, and Esc closes them and keeps them. A
 line under the bar says what they are set to.
 
+A call you select shows its details beside the list: the statement as it
+ran, its values, what it returned, the columns that were masked, how it
+ran and how long it took, or why it was refused or failed. **Open in SQL
+editor** puts the statement in a new SQL editor of its connection, in
+read-only mode and not run, in a window of that connection on its saved
+database: this one for a call on this connection, another that is open,
+or one that opens and connects for it. Its values come first, as the log
+keeps them, in a comment line each (`-- $1 = 9100000000000000004` on
+PostgreSQL, `?1` on SQLite, `? #1` on MySQL), for you to put in its
+placeholders. A call on a connection
+deleted since cannot be opened. In the Omarchy look the details show
+with Enter, `o` opens the statement, and Esc closes them.
+
 When an agent connects, Tabletist tells you: a notification on Linux,
 whose **Review** shows the Agents tab, the Dock icon bouncing on macOS,
 and the taskbar button flashing on Windows.
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index ce1444d..9d5c78e 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -122,7 +122,9 @@ key. Nor has copying a whole row.
 | `l` | Show the log of agents' calls, and back |
 | `L` | In the log, every connection's calls, or this one's |
 | `/` | In the log, open its filters: the search, the agent and the result. Tab moves among them, Enter or Space picks |
-| Esc | Close the log's filters, and keep them |
+| Enter | In the log, show the selected call's details, and close them |
+| `o` | In the log, open the selected call's statement in a SQL editor, read-only and not run |
+| Esc | Close the log's filters, and keep them, or the call's details |
 | `j` / `k` | In the log, move among the calls |
 
 ### The Structure view
````

- [ ] **Step 2: Check them**

Run: `docs/bin/check` and `grep -c "$(printf '\342\200\224')" docs/_guide/agents.md docs/_guide/omarchy.md` (a count of em dashes)

Expected: `HTML-Proofer finished successfully.`, and `0` for each page.

- [ ] **Step 3: Commit**

```bash
git add docs/_guide/agents.md docs/_guide/omarchy.md
git commit -m "Say on the website what a logged call's pane shows and opens"
```

---

### Task 4: The master plan

- D.5 as built for 3b-2; 3b's run 2 and G's row: built.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: The edits**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index 89ae174..0a8041e 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -2442,6 +2442,42 @@ went otherwise:
   is fixed with a test. The view left closes them, and each `/` gives the
   search the keyboard.
 
+**As built in 3b's run 2** (`2026-10-18-mcp-server-3b-2-call-pane.md`). Agreed on 2026-10-18, where it
+went otherwise:
+
+- **The pane**: on macOS beside the list, as the board draws it, while a
+  call is selected, in the place of Phase 1b's line under the list; on
+  Omarchy, whose board draws `enter details` and no pane, the same pane,
+  which Enter shows and closes and Esc closes, the line staying while it
+  is closed. It says the tool and how the call ended, the agent and the
+  moment (and the connection under All connections), the statement in its
+  dialect's colours (plain where its connection was deleted), then line by
+  line its values, the tables asked for, what it returned, the columns
+  masked, why it was refused or failed and its code, how it ran, how long
+  it took and the agent's project; and, for a statement, that rows are not
+  logged. What a write adds (the rows changed against the dry run, the
+  wait, who approved, the reason) comes with writes.
+- **Open in SQL editor**: a new SQL editor of the call's connection, read-only
+  and not run, in front, in a window of that connection on its saved
+  database: this one for a call on this connection, else one open, else
+  one opened and connected; one whose session ended connects again. The
+  statement as it ran, after a comment line for each of its values as the
+  driver names a parameter (`-- $1 = 9100000000000000004`, `?1` on SQLite,
+  `? #1` on MySQL), each value's JSON as the log keeps it (a number is not
+  read and written again); values the log cut, or that do not read, are
+  said to be. Off, with why, for a call on a connection deleted
+  since. On Omarchy `o`, while the Log view shows (`When::AgentsLog`, so
+  that the activity's `o` and Enter keep their own).
+- The Omarchy bar names `enter details` and `o open in sql tab` with a call
+  selected, where it has the room; the footer names the keys too, `o` only
+  where it opens something. The pane takes at most half the view.
+- The run's review found a window switched to another database taken for
+  the call's, a window whose session had ended given the editor and left
+  so, a value's number read and written again (a long one rounded, one
+  out of range dropping every value), the footer naming `o` where it does
+  nothing, the list given no room in a narrow window, and the pane left
+  open as a filter cleared the selection: each is fixed with a test.
+
 ### D.6 The keymap
 
 Checked against `src/keymap.rs` and the keyboard brief
@@ -2727,7 +2763,7 @@ is off until the person turns it on.
 | 1c. Hidden tables | 6, below | Built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`, `2026-10-16-mcp-server-1c-2b-setting.md`, `2026-10-16-mcp-server-1c-3a-tables-control.md`, `2026-10-16-mcp-server-1c-3b-table-list.md`) |
 | 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |
 | 3a. Masked columns | 5, below | Built (`2026-10-17-mcp-server-3a-1a-masked-rule.md`, `2026-10-17-mcp-server-3a-1b-masked-values.md`, `2026-10-17-mcp-server-3a-2-server.md`, `2026-10-17-mcp-server-3a-3-setting.md`, `2026-10-18-mcp-server-3a-4-columns-control.md`): the rule, its stops, the engines' fences, the values, the server's side, the setting, and the editor's control with the website's guide. 3b is next (order changed on 2026-10-17) |
-| 3b. The Log view, in full | 3, below | Run 1 built (`2026-10-18-mcp-server-3b-1-log-filters.md`): the agent and result filters and the search. Run 2, the detail pane and Open in SQL editor, is next |
+| 3b. The Log view, in full | 3, below | Runs 1 and 2 built (`2026-10-18-mcp-server-3b-1-log-filters.md`, `2026-10-18-mcp-server-3b-2-call-pane.md`): the filters and the search, the detail pane and Open in SQL editor. Run 3, Export… and the guide, is next |
 | 2. Approved writes | Not cut yet | Last, after 3b (moved on 2026-10-17) |
 
 The move of the connections to `connections.toml` came between 1a and 1b
@@ -2943,7 +2979,8 @@ pull request:
 1. The agent and result filters and the search of the requests, with the
    count of what matches, in both looks
    (`2026-10-18-mcp-server-3b-1-log-filters.md`, built).
-2. The detail pane for the selected call, and **Open in SQL editor**.
+2. The detail pane for the selected call, and **Open in SQL editor**
+   (`2026-10-18-mcp-server-3b-2-call-pane.md`, built).
 3. **Export…**, and the website's guide to the log.
 
 ## Open questions
````

- [ ] **Step 2: Check the whole of it**

Run: `cargo fmt --all --check`, clippy as above, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, the count of em dashes in the master plan, and `cargo test --locked --workspace --all-targets` with both servers

Expected: fmt and clippy clean, the documentation builds without a warning, no em dash (`0`), and:

```text
3712 passed, 0 failed, 22 ignored
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan that the log's call pane is built"
```

## Self-review

- **Against the design.** D.5's detail pane and Open in SQL editor; the macOS board's pane (`MacAgentsTabLog`); the Omarchy board's `enter details · o open in sql tab` (`OmarchyMcpActivity`). The departures are in "What the plan asks, and what is built", each agreed on 2026-10-18 or found by the review.
- **A review before the plan.** A reviewer ran Open in SQL editor for every kind of window (this one, another, none, two, one switched to another database, one disconnected), wrote values meant to break out of the comment, pressed Enter, `o` and Esc in every state of the view, and scrolled a long statement. It found a window switched to another database taken for the call's, a disconnected window given the editor and left so, numbers read and written again (rounded, or every value dropped), the footer naming `o` where it does nothing, no room left for the list in a narrow window, and the pane left open as a filter cleared the selection. All fixed, with a test each.
- **By eye.** Headless renders of the pane in both looks. Not committed.
- **Placeholders.** None: every step has its diff or its command and what it prints.
- **Names.** `open_agents_log_call`, `logged_statement`, `LoggedValues`, `AgentsLog::details_open`, `Action::ShowAgentsLogDetails`, `Action::OpenAgentsLogCall`, `Command::ShowAgentsLogDetails`, `Command::OpenAgentsLogCall`, `Command::CloseAgentsLogPane`, `When::AgentsLog`, `log_pane`, `log_facts`, `LogCall` and `sql_text::highlighted` are the same in every task that uses them.
