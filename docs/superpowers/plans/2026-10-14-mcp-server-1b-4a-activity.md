# MCP server, Phase 1b, run 4a: what agents are doing. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Agents tab shows what agents are doing on its connection as it happens: the sessions that are here, each with **Stop session**, those that work elsewhere ("Also:", with **Show**), **Stop agents here**, which also turns the connection's access Off, and the feed of the calls on this connection.

**Architecture:** The server tells of each call as it ends (`Server::calls`, a broadcast of `Call`: what the log keeps but the values, the statement on one line). The backend forwards each one (`AgentsEvent::Called`) and stops one session by its id (`AgentsCommand::StopSession`). The app keeps the last 300 calls in `AgentsState::feed`; the tab shows those on its connection. `StopAgentsHere` stops each session here and sets the access Off through `save_connections`, a lowering that applies at once. `ShowAgentsOn` brings forward, or opens, the window of the connection a session works on. Omarchy has a list of the sessions with a selection, and `j`/`k`, `S`, `X` and `enter`.

**Tech Stack:** Rust 2024, egui, tokio's broadcast channel. Nothing new is built.

---

## Where this sits

Phase 1b of `docs/superpowers/plans/2026-10-13-mcp-server.md` is cut into runs (section G). Six are merged: the server crate, the backend's gateway, the files and stdio, the Agents tab's frame, and the changes made outside the app.

1. Done: the server, the gateway, the files and stdio, the Agents tab, changes made outside.
2. **This run, 4a: what agents are doing.** The fourth run is cut in two (agreed on 2026-10-14): this part, then 4b, the Log view and the notification for an agent that connects (question 17, settled the same day: a notification).
3. Then the fifth, letting agents in, and Windows' way in.

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: views draw and push `Action`s and `App::apply` changes state; nothing an agent sent goes into the app's log. The feed holds an agent's statements in memory and on screen, never in `tabletist.log`.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.
- No `Cargo.toml` or `Cargo.lock` changes: tokio's `sync` feature, which has the broadcast channel, is on already. Nothing needs PostgreSQL or MySQL; the full run at the end prints "skipped" for their tests without the variables.

## What was run before this plan was written

Every task's state was built, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` (3,442 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. The tab was rendered headless in both looks, with two sessions, one elsewhere and four calls, and looked at; the scene was thrown away. Not run: macOS and Windows, which CI runs; on Windows nothing of agents shows.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| How the app learns of a call | `Server::calls`, a broadcast of `Call`, told once the call is in the log. The backend subscribes before the server listens and forwards each as `AgentsEvent::Called`. A receiver more than 256 behind loses the oldest | The log has every call; the feed is what happens now. A broadcast keeps the server from waiting on the app |
| What a call tells | When it came, its session, the connection it named, the tool, what it asked on one line cut at 200 characters, how it ended, the refusal's sentence as the log keeps it, rows, columns, time. Never its values | Values can hold personal data (question 18), and the feed has no room for them. The log keeps them |
| What the feed keeps | The last 300 calls since the app started, on every connection, the newest first; each tab shows those on its connection | "The feed's last few hundred calls" (D.4). Past calls are the Log view's (4b) |
| The feed's time | How long ago: "just now", "4 min ago" | The session's line says it so (3a): the app reads no local clock yet. The Log view brings one, with dates |
| How a call ended | Rows and time for a read ("3 rows · 4 ms"), tables and columns for `describe_schema`, and otherwise the word and the sentence: refused, failed, timed out, cancelled, stopped | The board's examples. The sentence is the one the log keeps, which never holds a row's value |
| Where a session works | The connection it named last: `SessionInfo::connections` ends with it | "Also: Cursor on Bookshop · staging" names where it is now. Found by the review: the list was in the order of first naming |
| "Also:" on macOS | In the header, in every state with the server on: the session elsewhere that called last, **Show** where its connection is saved, and "and N more" | The board draws one. Those not named are in their own windows' tabs |
| **Show** | `ShowAgentsOn(conn)`: that connection's window forward on its Agents tab, opened first where there is none, connecting as any does | D.4 |
| **Stop session** | `StopAgentSession(id)` → `AgentsCommand::StopSession` → `Server::stop_session`: its calls answer that the server stopped them, and its agent is let go | A.5. A session cannot be stopped in part |
| **Stop agents here** | Each session here stopped, and the connection's access set to Off and saved: a lowering, which applies at once. Offered while agents are here on a connection they may use. A toast says it ("Stopped 1 agent. Access is now Off for Bookshop · dev.") | A.5, question 21: a stop an agent undoes by connecting again is no stop |
| Omarchy's list | At the left, as its board: here, then elsewhere, beside the calls or beside the way to connect one. The selection is the session's id; `j`/`k`, `S`, `X`, `enter` (D.6). Enter is the list's unless the tree has the keyboard or a button has focus | A key lands on the session the person saw (D.4's rule for requests). Found by the review |

### What this run leaves out

- The Activity and Log switch, **Show log**, the Log view and its clock: the fourth run's second part, with the notification for an agent that connects.
- The level **Stop agents here** took away, kept beside the access: **Allow agents…**, which offers it, comes with the fifth run. With Off and Read alone the level is always Read.
- The write request's card, the count and the Waiting state: Phase 2.
- Arrows in Omarchy's list of sessions: `j` and `k` move in it, and the arrows stay the tree's. A count before `j` or `k` moves one.
- A sentence of its own for the calls of a session that was stopped: they say the agent server was stopped, as they did.

## File structure

| File | What changes |
|---|---|
| `crates/tabletist-mcp/src/server.rs`, `lib.rs` | `Call`, `CALL_CHARS`, `Server::calls`: each call as it ends |
| `crates/tabletist-mcp/tests/sessions.rs` | Each call is told, without its values |
| `src/backend/agents.rs` | `AgentsEvent::Called`, `AgentsCommand::StopSession`, the task that tells of calls |
| `tests/cli.rs` | Through the relay: the app hears a call, and stops the session |
| `src/model.rs` | `AgentsState::feed`, `called`, `calls_on`, `elsewhere`, `listed`; `AgentsTab::selected`; the actions |
| `src/app.rs` | The feed from `Called`; `StopAgentSession`, `StopAgentsHere`, `ShowAgentsOn`, `MoveAgentSelection`; `agents_selected` |
| `src/ui/agents.rs` | "Also:", **Stop agents here**, the sessions with **Stop session**, the feed; Omarchy's list of sessions |
| `src/keymap.rs`, `src/ui/keys.rs` | `j`/`k`, `S`, `X`, `enter` in the Agents tab, Omarchy |
| `src/ui/workspace.rs` | Omarchy's status line: the tab's keys with sessions here |
| `src/ui/agents_tests.rs` | The feed, the buttons, "Also:" and **Show**, Omarchy's keys |
| `docs/_guide/omarchy.md`, `docs/_reference/settings-and-files.md` | The keys, and what **Stop agents here** writes |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | What this run built, the cut of the fourth run, question 17 |

---

### Task 1: The server tells of each call as it ends

`tabletist_mcp::Call`: when a call arrived, its session, the connection it named, the tool, what it asked on one line and cut to `CALL_CHARS` (200), how it ended, the refusal's sentence as the log keeps it, the rows and columns, and how long it took. Never its values. `Server::calls` is a broadcast, told once the call is in the log; a receiver that falls more than 256 behind loses the oldest.

A session's connections (`SessionInfo::connections`) now end with the one it named last, so that "Also:" can say where it works now: a connection named again moves to the end.

Nothing listens yet: Task 2 does.

**Files:**
- Modify: `crates/tabletist-mcp/src/server.rs`, `crates/tabletist-mcp/src/lib.rs`, `crates/tabletist-mcp/tests/sessions.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/crates/tabletist-mcp/src/server.rs b/crates/tabletist-mcp/src/server.rs
index e9158fc..34a9961 100644
--- a/crates/tabletist-mcp/src/server.rs
+++ b/crates/tabletist-mcp/src/server.rs
@@ -451,3 +451,24 @@ fn millis(time: Duration) -> i64 {
 fn count(count: u64) -> i64 {
     i64::try_from(count).unwrap_or(i64::MAX)
 }
+
+#[cfg(test)]
+mod tests {
+    #[test]
+    fn a_request_is_told_on_one_line_and_cut() {
+        assert_eq!(
+            super::one_line("SELECT  id,\n\t title\nFROM books "),
+            "SELECT id, title FROM books"
+        );
+        let long = "x".repeat(super::CALL_CHARS + 10);
+        let told = super::one_line(&long);
+        assert_eq!(told.chars().count(), super::CALL_CHARS + 1);
+        assert!(told.ends_with('…'));
+        assert_eq!(
+            super::one_line(&"é".repeat(super::CALL_CHARS))
+                .chars()
+                .count(),
+            super::CALL_CHARS
+        );
+    }
+}
diff --git a/crates/tabletist-mcp/tests/sessions.rs b/crates/tabletist-mcp/tests/sessions.rs
index e53d7a8..30b0a79 100644
--- a/crates/tabletist-mcp/tests/sessions.rs
+++ b/crates/tabletist-mcp/tests/sessions.rs
@@ -100,6 +100,18 @@ async fn a_session_knows_where_it_works_and_the_server_says_when_that_changes()
         .call("describe_schema", json!({ "connection": "Bookshop · dev" }))
         .await;
     assert_eq!(server.open_sessions()[0].connections, ["id:Bookshop · dev"]);
+    // Another, then the first again: the one it named last is at the end,
+    // where it works now.
+    shop.list("Bookshop · staging", &shop.bookshop(), support::LIMITS);
+    for name in ["Bookshop · staging", "Bookshop · dev"] {
+        agent
+            .call("describe_schema", json!({ "connection": name }))
+            .await;
+    }
+    assert_eq!(
+        server.open_sessions()[0].connections,
+        ["id:Bookshop · staging", "id:Bookshop · dev"]
+    );
     changes.mark_unchanged();
     agent.close().await;
     assert!(changes.has_changed().unwrap(), "a session ended");
@@ -193,3 +205,51 @@ async fn stopping_the_server_ends_every_session_and_serves_no_new_one() {
     drop(transport);
     assert_eq!(server.open_sessions(), []);
 }
+
+#[tokio::test]
+async fn each_call_is_told_as_it_ends_without_its_values() {
+    let shop = Shop::new();
+    let (server, log) = server(&shop);
+    let mut calls = server.calls();
+    let agent = Agent::connect(&server).await;
+    agent
+        .call(
+            "run_query",
+            json!({
+                "connection": "Bookshop · dev",
+                "sql": "SELECT id,\n       title\n  FROM books WHERE id = ?",
+                "params": [1],
+            }),
+        )
+        .await;
+    let call = calls.recv().await.unwrap();
+    assert_eq!(call.session, server.open_sessions()[0].id);
+    assert_eq!(call.connection.as_deref(), Some("id:Bookshop · dev"));
+    assert_eq!(call.tool, "run_query");
+    assert_eq!(
+        call.request.as_deref(),
+        Some("SELECT id, title FROM books WHERE id = ?")
+    );
+    assert_eq!(
+        (call.result.as_str(), call.message.as_deref()),
+        ("ok", None)
+    );
+    assert_eq!((call.rows, call.columns), (Some(1), Some(2)));
+    // In the log by the time it is told.
+    assert_eq!(support::entries(&log).await.len(), 1);
+    // A refusal says why, as the log does; a call that names no
+    // connection is told with none.
+    agent
+        .call(
+            "run_query",
+            json!({ "connection": "Payroll", "sql": "SELECT 1" }),
+        )
+        .await;
+    let refused = calls.recv().await.unwrap();
+    assert_eq!(
+        (refused.result.as_str(), refused.connection.as_deref()),
+        ("refused", None)
+    );
+    assert!(refused.message.unwrap().contains("Payroll"));
+    agent.close().await;
+}
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked -p tabletist-mcp --test sessions -- each_call a_session_knows_where`, then `cargo test --locked -p tabletist-mcp --lib server::`

Expected: neither builds. The tests name what is not there yet:

```text
error[E0599]: no method named `calls` found for struct `Server<G>` in the current scope
error: could not compile `tabletist-mcp` (test "sessions") due to 1 previous error
error[E0425]: cannot find function `one_line` in module `super`
error[E0425]: cannot find value `CALL_CHARS` in module `super`
error: could not compile `tabletist-mcp` (lib test) due to 7 previous errors
```

- [ ] **Step 3: `Call` and `Server::calls`**

````diff
diff --git a/crates/tabletist-mcp/src/lib.rs b/crates/tabletist-mcp/src/lib.rs
index d71ebfc..29cf4ab 100644
--- a/crates/tabletist-mcp/src/lib.rs
+++ b/crates/tabletist-mcp/src/lib.rs
@@ -30,5 +30,5 @@ pub use answer::{MAX_ANSWER_BYTES, MAX_CELL_CHARS};
 pub use gateway::{Failure, Gateway, Listed, Schemas, Session};
 pub use log::{Entry, Log, LogError, Opened};
 pub use refusal::{Refused, code};
-pub use server::{Origin, Server, SessionInfo, Transport};
+pub use server::{CALL_CHARS, Call, Origin, Server, SessionInfo, Transport};
 pub use tools::{Tool, all as tools};
diff --git a/crates/tabletist-mcp/src/server.rs b/crates/tabletist-mcp/src/server.rs
index 34a9961..a453647 100644
--- a/crates/tabletist-mcp/src/server.rs
+++ b/crates/tabletist-mcp/src/server.rs
@@ -7,7 +7,7 @@ use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
 
 use serde_json::{Map, Value as Json};
 use tokio::io::{AsyncRead, AsyncWrite};
-use tokio::sync::{RwLock, watch};
+use tokio::sync::{RwLock, broadcast, watch};
 use tokio_util::sync::CancellationToken;
 
 use crate::gateway::Gateway;
@@ -51,13 +51,54 @@ pub struct SessionInfo {
     pub transport: Transport,
     pub connected_at: SystemTime,
     pub calls: u64,
-    /// The connections its calls have named, by id, in the order it first
-    /// named each: where it works.
+    /// The connections its calls have named, by id, the one it named last
+    /// at the end: where it works, and where it works now.
     pub connections: Vec<String>,
     /// When its last call came, once it has made one.
     pub last_call_at: Option<SystemTime>,
 }
 
+/// A call, as the app shows it as it ends: what the log keeps of it but
+/// its values, with its statement cut to its start.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Call {
+    /// When it arrived.
+    pub at: SystemTime,
+    pub session: String,
+    /// The id of the connection it named, where it named one an agent may
+    /// see.
+    pub connection: Option<String>,
+    pub tool: String,
+    /// What it asked: the statement, or the tables, on one line and cut
+    /// to [`CALL_CHARS`] characters.
+    pub request: Option<String>,
+    /// As the log has it: `ok`, `refused`, `error`, `timeout`,
+    /// `cancelled` or `stopped`.
+    pub result: String,
+    /// The refusal's sentence, as the log keeps it.
+    pub message: Option<String>,
+    /// The rows the agent was sent, or the tables it was described.
+    pub rows: Option<u64>,
+    pub columns: Option<u64>,
+    pub took: Duration,
+}
+
+/// The most characters of a call's request [`Server::calls`] tells.
+pub const CALL_CHARS: usize = 200;
+
+/// How many calls [`Server::calls`] holds for a receiver that is behind.
+const CALLS_HELD: usize = 256;
+
+/// `text` on one line, its runs of white space one space, and cut to
+/// [`CALL_CHARS`] characters.
+fn one_line(text: &str) -> String {
+    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
+    match line.char_indices().nth(CALL_CHARS) {
+        Some((end, _)) => format!("{}…", &line[..end]),
+        None => line,
+    }
+}
+
 /// What stands for an agent's name until it has given one.
 const UNNAMED: &str = "unknown";
 
@@ -133,14 +174,14 @@ impl Live {
         }
     }
 
-    /// Notes a call that came at `at`, and the connection it named. A call
-    /// that ends after a later one does not take the time back.
+    /// Notes a call that came at `at`, and the connection it named, which
+    /// goes to the end of those it has named. A call that ends after a
+    /// later one does not take the time back.
     fn called(&self, at: SystemTime, connection: Option<&str>) {
         let mut used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
         used.last_call_at = used.last_call_at.max(Some(at));
-        if let Some(id) = connection
-            && !used.connections.iter().any(|named| named == id)
-        {
+        if let Some(id) = connection {
+            used.connections.retain(|named| named != id);
             used.connections.push(id.to_owned());
         }
         drop(used);
@@ -239,6 +280,8 @@ struct Inner<G> {
     /// Sent to when a session opens or ends, names its agent, or makes a
     /// call: what [`Server::open_sessions`] says has changed.
     changed: Arc<watch::Sender<()>>,
+    /// Each call, as it ends.
+    calls: broadcast::Sender<Call>,
 }
 
 impl<G: Gateway> Server<G> {
@@ -253,6 +296,7 @@ impl<G: Gateway> Server<G> {
                 sessions: Mutex::default(),
                 stop: Stop::new(),
                 changed: Arc::new(watch::Sender::new(())),
+                calls: broadcast::Sender::new(CALLS_HELD),
             }),
         }
     }
@@ -295,6 +339,12 @@ impl<G: Gateway> Server<G> {
         self.inner.changed.subscribe()
     }
 
+    /// Told of each call as it ends, once it is in the log. A receiver
+    /// that falls more than a few hundred calls behind loses the oldest.
+    pub fn calls(&self) -> broadcast::Receiver<Call> {
+        self.inner.calls.subscribe()
+    }
+
     /// The sessions that are open, the oldest first.
     pub fn open_sessions(&self) -> Vec<SessionInfo> {
         self.sessions().iter().map(|live| live.info()).collect()
@@ -405,6 +455,22 @@ impl<G: Gateway> Server<G> {
             took.as_millis()
         );
         let (connection_id, connection) = note.connection.unzip();
+        let call = Call {
+            at: arrived,
+            session: live.id.clone(),
+            connection: connection_id.clone(),
+            tool: name.to_owned(),
+            request: note
+                .sql
+                .as_deref()
+                .or(note.request.as_deref())
+                .map(one_line),
+            result: result.to_owned(),
+            message: message.clone(),
+            rows: note.rows,
+            columns: note.columns,
+            took,
+        };
         let entry = Entry {
             at_ms: millis(arrived.duration_since(UNIX_EPOCH).unwrap_or_default()),
             session: live.id.clone(),
@@ -432,6 +498,8 @@ impl<G: Gateway> Server<G> {
             // turn every read into an error.
             log::error!("an agent's call is not in the log: {error}");
         }
+        // Nobody may be listening.
+        let _ = self.inner.calls.send(call);
         Ok(reply)
     }
 }
````

- [ ] **Step 4: Run the tests**

Run: the same, then `cargo test --locked -p tabletist-mcp`

Expected: PASS, and the crate's other tests with them.

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 47 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-mcp
git commit -m "Tell of each call an agent makes as it ends, without its values"
```

---

### Task 2: The app hears each call, and the backend stops a session

- `AgentsEvent::Called(Call)`: the backend's task `called`, started with the server before it listens, forwards each call and stops with the server. One that lags is let go: the log has every call.
- `AgentsCommand::StopSession(id)` stops one session through `Server::stop_session`. Nothing where it is gone.
- `AgentsState::feed` keeps the last `FEED_KEPT` (300) calls, the newest first, on every connection; `calls_on` gives those on one. A call opens nothing and moves no keys.
- The helpers of the backend's tests and of `tests/cli.rs` that skip the sessions skip the calls too.

**Files:**
- Modify: `src/backend/agents.rs`, `src/model.rs`, `src/app.rs`, `tests/cli.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 8285a43..ee497c8 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -5938,6 +5938,29 @@ mod tests {
         assert_eq!(app.agents_access(saved), AgentAccess::Off);
     }
 
+    #[test]
+    fn a_call_goes_into_the_feed_and_opens_nothing() {
+        let (mut app, _dir) = app();
+        let (tab, _, _) = connect(&mut app);
+        let conn = app.workspace(tab).unwrap().conn_id.clone();
+        let (active, tabs) = (app.active_tab_id(), app.tabs.len());
+        app.apply_event(Event::Agents(AgentsEvent::Called(tabletist_mcp::Call {
+            at: std::time::SystemTime::now(),
+            session: "one".into(),
+            connection: Some(conn.0.clone()),
+            tool: "run_query".into(),
+            request: Some("SELECT 1".into()),
+            result: "ok".into(),
+            message: None,
+            rows: Some(1),
+            columns: Some(1),
+            took: std::time::Duration::from_millis(2),
+        })));
+        assert_eq!(app.agents.calls_on(&conn).count(), 1);
+        assert_eq!((app.active_tab_id(), app.tabs.len()), (active, tabs));
+        assert_eq!(app.workspace(tab).unwrap().agents_tab(), None);
+    }
+
     #[test]
     fn agents_are_told_of_each_change_to_what_they_may_use() {
         let (mut app, _dir) = app();
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 947fceb..f210853 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -1586,12 +1586,12 @@ mod tests {
         assert!(!dir.path().join("agents.sqlite3.bad").exists());
     }
 
-    /// The backend's next event but the sessions, which it says beside
-    /// what the server does.
+    /// The backend's next event but the sessions and the calls, which it
+    /// says beside what the server does.
     fn told(backend: &mut Backend, wait: Duration) -> Option<Event> {
         loop {
             match backend.wait(wait)? {
-                Event::Agents(AgentsEvent::Sessions(_)) => {}
+                Event::Agents(AgentsEvent::Sessions(_) | AgentsEvent::Called(_)) => {}
                 event => return Some(event),
             }
         }
diff --git a/src/model.rs b/src/model.rs
index 04e88d6..229ef8b 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -3662,6 +3662,41 @@ mod tests {
         assert_eq!(state.of(true, Some(Off), &conn), AgentsHere::OffHere);
     }
 
+    #[test]
+    fn the_feed_keeps_the_last_calls_and_a_window_sees_those_on_its_connection() {
+        let call = |n: usize, conn: &str| tabletist_mcp::Call {
+            at: std::time::SystemTime::UNIX_EPOCH,
+            session: "s".into(),
+            connection: Some(conn.to_owned()),
+            tool: "run_query".into(),
+            request: Some(format!("SELECT {n}")),
+            result: "ok".into(),
+            message: None,
+            rows: Some(1),
+            columns: Some(1),
+            took: std::time::Duration::from_millis(4),
+        };
+        let mut state = AgentsState::default();
+        for n in 0..FEED_KEPT + 4 {
+            state.called(call(n, if n % 2 == 0 { "shop" } else { "ledger" }));
+        }
+        assert_eq!(state.feed.len(), FEED_KEPT);
+        let shop = ConnectionId("shop".into());
+        let newest: Vec<_> = state
+            .calls_on(&shop)
+            .take(2)
+            .map(|call| call.request.clone().unwrap())
+            .collect();
+        assert_eq!(
+            newest,
+            [
+                format!("SELECT {}", FEED_KEPT + 2),
+                format!("SELECT {FEED_KEPT}")
+            ]
+        );
+        assert_eq!(state.calls_on(&shop).count(), FEED_KEPT / 2);
+    }
+
     /// The Bookshop's covers, `rows` of them, as a tab holds them.
     fn covers(rows: usize) -> ObjectTab {
         let object = ObjectRef::new("main", "book_covers");
diff --git a/tests/cli.rs b/tests/cli.rs
index dde0e08..42a82cb 100644
--- a/tests/cli.rs
+++ b/tests/cli.rs
@@ -147,10 +147,10 @@ mod mcp {
         let deadline = Instant::now() + WAIT;
         loop {
             for event in backend.poll() {
-                // The sessions as they change are said beside what the
-                // server does.
+                // The sessions as they change, and the calls as they end,
+                // are said beside what the server does.
                 match event {
-                    Event::Agents(AgentsEvent::Sessions(_)) => {}
+                    Event::Agents(AgentsEvent::Sessions(_) | AgentsEvent::Called(_)) => {}
                     Event::Agents(event) => return event,
                     _ => {}
                 }
@@ -195,6 +195,99 @@ mod mcp {
         );
     }
 
+    /// What the app hears of an agent through the backend: each event of
+    /// agents but the sessions, until `found` says one is the one.
+    fn heard(backend: &mut Backend, found: impl Fn(&AgentsEvent) -> bool) -> AgentsEvent {
+        let deadline = Instant::now() + WAIT;
+        loop {
+            for event in backend.poll() {
+                if let Event::Agents(event) = event
+                    && found(&event)
+                {
+                    return event;
+                }
+            }
+            assert!(Instant::now() < deadline, "the backend did not say it");
+            std::thread::sleep(Duration::from_millis(20));
+        }
+    }
+
+    #[test]
+    fn the_app_hears_each_call_and_stops_a_session_it_names() {
+        let dir = tempfile::tempdir().unwrap();
+        let database = dir.path().join("shop.db");
+        tabletist_db::fixtures::write_sqlite_bookshop(&database).unwrap();
+        let connection = SavedConnection {
+            id: ConnectionId("shop".into()),
+            name: "Bookshop".into(),
+            environment: Environment::Dev,
+            read_only: None,
+            password: PasswordMode::None,
+            ssh_secret: PasswordMode::None,
+            spec: ConnectSpec::sqlite(database),
+            agents: Agents {
+                access: AgentAccess::Read,
+                ..Agents::default()
+            },
+        };
+        let endpoint: PathBuf = dir.path().join("run");
+        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
+        backend.send(Command::Agents(AgentsCommand::Policy {
+            connections: agentable(&[connection]),
+            host_keys: HostKeys::default(),
+        }));
+        backend.send(Command::Agents(AgentsCommand::Start {
+            log: dir.path().join("agents.sqlite3"),
+            log_days: 30,
+            endpoint: Some(endpoint.clone()),
+        }));
+        assert_eq!(
+            agents_said(&mut backend),
+            AgentsEvent::Started { kept_aside: None }
+        );
+        let mut child = relay(&endpoint, dir.path());
+        let mut stdin = child.stdin.take().unwrap();
+        let lines = lines_of(&mut child);
+        stdin.write_all(&line(&initialize(0))).unwrap();
+        lines.recv_timeout(WAIT).expect("initialized");
+        let initialized = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
+        stdin.write_all(&line(&initialized)).unwrap();
+        let query = "SELECT count(*)\n  FROM books";
+        let arguments = json!({ "connection": "Bookshop · dev", "sql": query, "params": [] });
+        stdin
+            .write_all(&line(&call(1, "run_query", arguments)))
+            .unwrap();
+        lines.recv_timeout(WAIT).expect("an answer");
+        // The call, as it ended: where, what, and how, without its values.
+        let AgentsEvent::Called(called) = heard(&mut backend, |event| {
+            matches!(event, AgentsEvent::Called(_))
+        }) else {
+            unreachable!()
+        };
+        assert_eq!(called.connection.as_deref(), Some("shop"));
+        assert_eq!(called.tool, "run_query");
+        assert_eq!(
+            called.request.as_deref(),
+            Some("SELECT count(*) FROM books")
+        );
+        assert_eq!((called.result.as_str(), called.rows), ("ok", Some(1)));
+        // Stopped by its id, the session lets its agent go, and the relay
+        // ends.
+        backend.send(Command::Agents(AgentsCommand::StopSession(
+            called.session.clone(),
+        )));
+        assert!(ended(&mut child).success());
+        let gone = heard(
+            &mut backend,
+            |event| matches!(event, AgentsEvent::Sessions(open) if open.is_empty()),
+        );
+        assert_eq!(gone, AgentsEvent::Sessions(Vec::new()));
+        // One that is gone is nothing to stop.
+        backend.send(Command::Agents(AgentsCommand::StopSession(called.session)));
+        backend.send(Command::Agents(AgentsCommand::Stop));
+        assert_eq!(agents_said(&mut backend), AgentsEvent::Stopped);
+    }
+
     #[test]
     fn an_agent_reads_through_the_relay_and_nothing_private_comes_back() {
         let dir = tempfile::tempdir().unwrap();
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- model::tests::the_feed app::tests::a_call_goes`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0425]: cannot find value `FEED_KEPT` in this scope
error[E0599]: no variant, associated function, or constant named `Called` found for enum `backend::agents::AgentsEvent` in the current scope
error[E0599]: no method named `calls_on` found for struct `model::AgentsState` in the current scope
error[E0599]: no method named `called` found for struct `model::AgentsState` in the current scope
error[E0609]: no field `feed` on type `model::AgentsState`
error: could not compile `tabletist` (lib test) due to 12 previous errors
```

- [ ] **Step 3: Forward the calls, keep the feed, stop a session**

````diff
diff --git a/src/app.rs b/src/app.rs
index ee497c8..5d6e98d 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -2852,6 +2852,7 @@ impl App {
                     self.agents.sessions = open;
                 }
                 AgentsEvent::Sessions(_) => {}
+                AgentsEvent::Called(call) => self.agents.called(call),
                 AgentsEvent::Started {
                     kept_aside: Some(path),
                 } => {
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index f210853..59eabf4 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -144,6 +144,9 @@ pub enum AgentsCommand {
     },
     /// The log keeps calls for these days from now on.
     LogDays(u32),
+    /// Stops one session, by its id: its calls answer that the server
+    /// stopped them, and its agent is let go. Nothing where it is gone.
+    StopSession(String),
     /// Stops it: every session ends, and the connections are closed.
     /// Answered with [`AgentsEvent::Stopped`].
     Stop,
@@ -164,6 +167,8 @@ pub enum AgentsEvent {
     /// opens or ends, names its agent, or makes a call. None is said after
     /// [`AgentsEvent::Stopped`].
     Sessions(Vec<tabletist_mcp::SessionInfo>),
+    /// A call ended, and is in the log.
+    Called(tabletist_mcp::Call),
 }
 
 /// How long a connection an agent used is kept open with nothing to do.
@@ -186,6 +191,8 @@ pub(super) struct Running {
     chores: tokio::task::JoinHandle<()>,
     /// Tells the app of the sessions as they change.
     watching: tokio::task::JoinHandle<()>,
+    /// Tells the app of each call as it ends.
+    telling: tokio::task::JoinHandle<()>,
     #[cfg(unix)]
     listening: Option<listen::Listening>,
 }
@@ -226,6 +233,13 @@ impl Agents {
                 self.log_days
                     .store(days, std::sync::atomic::Ordering::SeqCst);
             }
+            AgentsCommand::StopSession(id) => {
+                if let Some(running) = lock(&self.running).as_ref()
+                    && running.server.stop_session(&id)
+                {
+                    log::info!("a session of an agent was stopped");
+                }
+            }
             AgentsCommand::Start {
                 log,
                 log_days,
@@ -297,8 +311,10 @@ impl Agents {
                     if let Some(before) = running.take() {
                         before.stop();
                     }
-                    // Before it listens: no session opens unseen.
+                    // Before it listens: no session opens unseen, and no
+                    // call ends untold.
                     let changes = server.changes();
+                    let calls = server.calls();
                     // Once the server before it has let its socket go.
                     let listening = match listening(endpoint.as_deref(), &server) {
                         Ok(listening) => listening,
@@ -313,10 +329,12 @@ impl Agents {
                     let () = listening;
                     let chores = tokio::spawn(chores(opened.log, gateway, log_days));
                     let watching = tokio::spawn(sessions(server.clone(), changes, outbox.clone()));
+                    let telling = tokio::spawn(called(calls, outbox.clone()));
                     *running = Some(Running {
                         server,
                         chores,
                         watching,
+                        telling,
                         #[cfg(unix)]
                         listening,
                     });
@@ -349,6 +367,7 @@ impl Running {
         self.server.stop();
         self.chores.abort();
         self.watching.abort();
+        self.telling.abort();
         #[cfg(unix)]
         drop(self.listening);
     }
@@ -406,6 +425,19 @@ async fn sessions(
     }
 }
 
+/// Tells the app of each call as it ends, until the server stops. One
+/// that falls behind misses the oldest: the log has them all.
+async fn called(mut calls: tokio::sync::broadcast::Receiver<tabletist_mcp::Call>, outbox: Outbox) {
+    use tokio::sync::broadcast::error::RecvError;
+    loop {
+        match calls.recv().await {
+            Ok(call) => outbox.emit(super::Event::Agents(AgentsEvent::Called(call))),
+            Err(RecvError::Lagged(_)) => {}
+            Err(RecvError::Closed) => return,
+        }
+    }
+}
+
 /// What the server does from time to time while it runs: closes the
 /// connections no agent has used for a while, and forgets calls older than
 /// the log keeps. The first time as it starts.
diff --git a/src/model.rs b/src/model.rs
index 229ef8b..7d7207b 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2556,8 +2556,14 @@ pub struct AgentsTab {
 #[derive(Debug, Clone, Default, PartialEq, Eq)]
 pub struct AgentsState {
     pub sessions: Vec<tabletist_mcp::SessionInfo>,
+    /// The last [`FEED_KEPT`] calls, on every connection. The log has
+    /// them all.
+    pub feed: std::collections::VecDeque<tabletist_mcp::Call>,
 }
 
+/// How many calls the app keeps for the feed.
+pub const FEED_KEPT: usize = 300;
+
 /// What the sidebar's Agents entry says of a connection, and which state
 /// its tab shows. A write that waits for the person comes with writes.
 #[derive(Debug, Clone, PartialEq, Eq)]
@@ -2573,6 +2579,22 @@ pub enum AgentsHere {
 }
 
 impl AgentsState {
+    /// Takes a call that ended into the feed.
+    pub fn called(&mut self, call: tabletist_mcp::Call) {
+        self.feed.push_front(call);
+        self.feed.truncate(FEED_KEPT);
+    }
+
+    /// The feed's calls on `conn`, the newest first.
+    pub fn calls_on<'a>(
+        &'a self,
+        conn: &'a ConnectionId,
+    ) -> impl Iterator<Item = &'a tabletist_mcp::Call> + 'a {
+        self.feed
+            .iter()
+            .filter(move |call| call.connection.as_deref() == Some(conn.0.as_str()))
+    }
+
     /// The sessions that are here: those whose calls have named `conn`,
     /// the one that called last first.
     pub fn here(&self, conn: &ConnectionId) -> Vec<&tabletist_mcp::SessionInfo> {
````

- [ ] **Step 4: Run the tests**

Run: the same, then `cargo test --locked --lib backend::` and `cargo test --locked --test cli`

Expected: PASS. The test of `tests/cli.rs` runs `tabletist mcp` against the backend, on Unix.

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2485 filtered out
test result: ok. 143 passed; 0 failed; 0 ignored; 0 measured; 2344 filtered out
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src tests/cli.rs
git commit -m "Keep the calls agents make as they end, and stop one session by its id"
```

---

### Task 3: Stop session, Stop agents here, and Show

- `Action::StopAgentSession(id)` asks the backend to stop that session, wherever it works.
- `Action::StopAgentsHere(tab)` stops each session that is here, wherever else it works too, and sets the connection's access to Off and saves it. `save_connections` checks what agents may do (1b-3b): a lowering applies at once, and the policy the backend is sent leaves the connection out.
- `Action::ShowAgentsOn(conn)` brings the window of `conn` forward on its Agents tab, and opens it first where there is none. A connection saved no more: nothing.
- `AgentsState::elsewhere`, the sessions that do not work here, those that named none among them, the one that called last first; `listed`, here then elsewhere, which the terminal look's selection moves in. The selection is the session's id (`AgentsTab::selected`), so a key lands on the session the person saw however the list reorders. `Action::MoveAgentSelection` steps from the session shown selected; `App::agents_selected` is that session, or the first where it has ended.

**Files:**
- Modify: `src/model.rs`, `src/app.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 5d6e98d..7a9fbf7 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -5939,6 +5939,20 @@ mod tests {
         assert_eq!(app.agents_access(saved), AgentAccess::Off);
     }
 
+    /// A session of Claude Code's that works on `conns`, by their ids.
+    fn agent_session(id: &str, conns: &[&ConnectionId]) -> tabletist_mcp::SessionInfo {
+        tabletist_mcp::SessionInfo {
+            id: id.into(),
+            agent: Some("Claude Code".into()),
+            project: Some("tabletist-repo".into()),
+            transport: tabletist_mcp::Transport::Stdio,
+            connected_at: std::time::SystemTime::now(),
+            calls: 3,
+            connections: conns.iter().map(|conn| conn.0.clone()).collect(),
+            last_call_at: Some(std::time::SystemTime::now()),
+        }
+    }
+
     #[test]
     fn a_call_goes_into_the_feed_and_opens_nothing() {
         let (mut app, _dir) = app();
@@ -5962,6 +5976,99 @@ mod tests {
         assert_eq!(app.workspace(tab).unwrap().agents_tab(), None);
     }
 
+    #[test]
+    fn stop_session_asks_the_backend_to_stop_that_one() {
+        let (mut app, _dir) = app();
+        // Not before the app serves agents: tests never start it.
+        app.apply(Action::StopAgentSession("one".into()));
+        assert!(agents_sent(&app).is_empty());
+        app.serve_agents();
+        app.backend.sent.clear();
+        app.apply(Action::StopAgentSession("one".into()));
+        assert!(matches!(
+            agents_sent(&app)[..],
+            [AgentsCommand::StopSession(id)] if id == "one"
+        ));
+    }
+
+    #[test]
+    fn stop_agents_here_stops_who_is_here_and_turns_its_access_off() {
+        use crate::connections::AgentAccess;
+        let (mut app, dir) = app();
+        let (tab, _, _) = connect(&mut app);
+        let conn = app.workspace(tab).unwrap().conn_id.clone();
+        let mut readable = app.connections.get(&conn).unwrap().clone();
+        readable.agents.access = AgentAccess::Read;
+        app.connections.upsert(readable);
+        app.check_agents();
+        app.apply(Action::KeepAgentChanges(app.agents_held.clone()));
+        app.serve_agents();
+        let other = ConnectionId("other".into());
+        app.agents.sessions = vec![
+            agent_session("here", &[&conn]),
+            agent_session("both", &[&other, &conn]),
+            agent_session("elsewhere", &[&other]),
+        ];
+        app.backend.sent.clear();
+        app.apply(Action::StopAgentsHere(tab));
+        // Each that is here, wherever else it works too: a session cannot
+        // be stopped in part.
+        let stopped: Vec<_> = agents_sent(&app)
+            .into_iter()
+            .filter_map(|sent| match sent {
+                AgentsCommand::StopSession(id) => Some(id.as_str()),
+                _ => None,
+            })
+            .collect();
+        assert_eq!(stopped.len(), 2);
+        assert!(stopped.contains(&"here") && stopped.contains(&"both"));
+        // Off, saved, and told to the backend: an agent that connects
+        // again does not see it.
+        assert_eq!(
+            app.connections.get(&conn).unwrap().agents.access,
+            AgentAccess::Off
+        );
+        let path = AppDirs::at(dir.path()).connections_file();
+        assert!(matches!(
+            saved(&app, &path),
+            Some(StateFile::Connections(stored))
+                if stored.connections.iter().all(|saved| saved.agents.access == AgentAccess::Off)
+        ));
+        assert!(matches!(
+            agents_sent(&app).last(),
+            Some(AgentsCommand::Policy { connections, .. }) if connections.is_empty()
+        ));
+        assert!(app.agents_held.is_empty(), "a lowering is not held");
+    }
+
+    #[test]
+    fn show_brings_the_window_of_the_connection_an_agent_works_on_forward() {
+        let (mut app, _dir) = app();
+        let (first, _, _) = connect(&mut app);
+        let elsewhere = with_saved(&mut app);
+        let windows = app.tabs.len();
+        // Not open: its window opens, on its Agents tab, and connects.
+        app.apply(Action::ShowAgentsOn(elsewhere.clone()));
+        assert_eq!(app.tabs.len(), windows + 1);
+        let opened = app.active_tab_id();
+        assert_ne!(opened, first);
+        let workspace = app.workspace(opened).unwrap();
+        assert_eq!(workspace.conn_id, elsewhere);
+        assert!(workspace.agents_in_front());
+        assert!(matches!(
+            app.backend.sent.last(),
+            Some(Command::Connect { .. })
+        ));
+        // Open: brought forward, and nothing more is opened.
+        app.apply(Action::ActivateConnTab(first));
+        app.apply(Action::ShowAgentsOn(elsewhere.clone()));
+        assert_eq!(app.tabs.len(), windows + 1);
+        assert_eq!(app.active_tab_id(), opened);
+        // A connection that is saved no more: nothing.
+        app.apply(Action::ShowAgentsOn(ConnectionId("gone".into())));
+        assert_eq!(app.active_tab_id(), opened);
+    }
+
     #[test]
     fn agents_are_told_of_each_change_to_what_they_may_use() {
         let (mut app, _dir) = app();
diff --git a/src/model.rs b/src/model.rs
index 7d7207b..af54637 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -3719,6 +3719,37 @@ mod tests {
         assert_eq!(state.calls_on(&shop).count(), FEED_KEPT / 2);
     }
 
+    #[test]
+    fn a_session_that_does_not_work_here_is_elsewhere_and_one_that_named_none_too() {
+        let session = |agent: &str, connections: &[&str], at: u64| tabletist_mcp::SessionInfo {
+            id: agent.into(),
+            agent: Some(agent.into()),
+            project: None,
+            transport: tabletist_mcp::Transport::Stdio,
+            connected_at: std::time::SystemTime::UNIX_EPOCH,
+            calls: 1,
+            connections: connections.iter().map(|id| (*id).to_owned()).collect(),
+            last_call_at: (at > 0)
+                .then(|| std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(at)),
+        };
+        let state = AgentsState {
+            sessions: vec![
+                session("Zed", &[], 0),
+                session("Cursor", &["staging"], 3),
+                session("Claude Code", &["staging", "shop"], 9),
+                session("Cline", &["ledger"], 5),
+            ],
+            ..AgentsState::default()
+        };
+        let shop = ConnectionId("shop".into());
+        let names: Vec<_> = state
+            .elsewhere(&shop)
+            .into_iter()
+            .map(|session| session.id.as_str())
+            .collect();
+        assert_eq!(names, ["Cline", "Cursor", "Zed"]);
+    }
+
     /// The Bookshop's covers, `rows` of them, as a tab holds them.
     fn covers(rows: usize) -> ObjectTab {
         let object = ObjectRef::new("main", "book_covers");
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- app::tests::stop_ app::tests::show_brings model::tests::a_session_that_does_not`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0599]: no variant, associated function, or constant named `StopAgentSession` found for enum `model::Action` in the current scope
error[E0599]: no variant, associated function, or constant named `StopAgentsHere` found for enum `model::Action` in the current scope
error[E0599]: no variant, associated function, or constant named `ShowAgentsOn` found for enum `model::Action` in the current scope
error[E0599]: no method named `elsewhere` found for struct `model::AgentsState` in the current scope
error: could not compile `tabletist` (lib test) due to 7 previous errors
```

- [ ] **Step 3: The actions**

````diff
diff --git a/src/app.rs b/src/app.rs
index 7a9fbf7..2bd525c 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1411,6 +1411,35 @@ impl App {
             Action::NewSqlTab(tab) => self.new_sql_tab(tab),
             Action::ShowAgents(tab) => self.show_agents(tab),
             Action::TurnAgentsOn => self.turn_agents_on(),
+            Action::StopAgentSession(id) => self.stop_agent_session(id),
+            Action::StopAgentsHere(tab) => self.stop_agents_here(tab),
+            Action::ShowAgentsOn(conn) => self.show_agents_on(conn),
+            Action::MoveAgentSelection { tab, step } => {
+                let Some(conn) = self
+                    .workspace(tab)
+                    .map(|workspace| workspace.conn_id.clone())
+                else {
+                    return;
+                };
+                // From the session shown selected, wherever the list has
+                // put it since.
+                let shown = self.agents_selected(tab).map(|session| session.id.clone());
+                let listed = self.agents.listed(&conn);
+                let at = listed
+                    .iter()
+                    .position(|session| Some(&session.id) == shown.as_ref())
+                    .unwrap_or_default();
+                let to = at
+                    .saturating_add_signed(step)
+                    .min(listed.len().saturating_sub(1));
+                let selected = listed.get(to).map(|session| session.id.clone());
+                if let Some(agents) = self
+                    .workspace_mut(tab)
+                    .and_then(crate::model::Workspace::agents_tab_mut)
+                {
+                    agents.selected = selected;
+                }
+            }
             Action::KeepAgentChanges(said) => self.keep_agent_changes(&said),
             Action::PutBackAgentChanges(said) => self.put_back_agent_changes(&said),
             Action::RunSql { tab, sql_tab, all } => {
@@ -3695,13 +3724,87 @@ impl App {
         let shown = workspace.agents_tab().unwrap_or_else(|| {
             workspace
                 .tabs
-                .push(Tab::Agents(crate::model::AgentsTab { id }));
+                .push(Tab::Agents(crate::model::AgentsTab { id, selected: None }));
             id
         });
         workspace.active_tab = Some(shown);
         workspace.pane = Pane::Grid;
     }
 
+    /// Stops one agent's session. Its line goes when the backend says the
+    /// sessions again.
+    fn stop_agent_session(&mut self, id: String) {
+        if self.serves_agents {
+            self.backend
+                .send(Command::Agents(AgentsCommand::StopSession(id)));
+        }
+    }
+
+    /// Stops every session that is here, wherever else it works too, and
+    /// sets the connection's access to Off and saves it: a stop that an
+    /// agent undoes by connecting again is no stop (A.5, question 21).
+    fn stop_agents_here(&mut self, tab: ConnTabId) {
+        let Some(conn) = self
+            .workspace(tab)
+            .map(|workspace| workspace.conn_id.clone())
+        else {
+            return;
+        };
+        let here: Vec<String> = self
+            .agents
+            .here(&conn)
+            .into_iter()
+            .map(|session| session.id.clone())
+            .collect();
+        for id in here {
+            self.stop_agent_session(id);
+        }
+        if let Some(mut saved) = self.connections.get(&conn).cloned()
+            && saved.agents.access != crate::connections::AgentAccess::Off
+        {
+            saved.agents.access = crate::connections::AgentAccess::Off;
+            self.connections.upsert(saved);
+            self.save_connections();
+        }
+    }
+
+    /// Shows the Agents tab of `conn`'s window, opening the window first
+    /// where there is none. It connects behind the tab as any does.
+    fn show_agents_on(&mut self, conn: crate::connections::ConnectionId) {
+        let tab = match self.tab_showing(&conn) {
+            Some(tab) => tab,
+            None => {
+                let Some(saved) = self.connections.get(&conn).cloned() else {
+                    return;
+                };
+                let tab = self.picker_tab();
+                let id = tab.id;
+                self.tabs.push(tab);
+                self.connect_tab(id, saved, Secrets::default());
+                id
+            }
+        };
+        self.apply(Action::ActivateConnTab(tab));
+        self.show_agents(tab);
+    }
+
+    /// The session the Agents tab of `tab` has selected, in the terminal
+    /// look: the first of the list until one is, and where that one has
+    /// ended.
+    pub fn agents_selected(&self, tab: ConnTabId) -> Option<&tabletist_mcp::SessionInfo> {
+        let workspace = self.workspace(tab)?;
+        let selected = workspace.tabs.iter().find_map(|open| match open {
+            Tab::Agents(agents) => Some(agents.selected.as_ref()),
+            Tab::Object(_) | Tab::Sql(_) => None,
+        })?;
+        let listed = self.agents.listed(&workspace.conn_id);
+        listed
+            .iter()
+            .find(|session| Some(&session.id) == selected)
+            .or(listed.first())
+            .copied()
+    }
+
     /// What an agent calls the workspace's connection, Off or not: its
     /// saved name and environment, as `list_connections` says them. The
     /// tab's own name where the connection is saved no more.
diff --git a/src/model.rs b/src/model.rs
index af54637..bf43fac 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -616,6 +616,20 @@ pub enum Action {
     /// Turn the agent server on, from the Agents tab: a change of the
     /// settings, which starts it. No connection's access changes.
     TurnAgentsOn,
+    /// Stop one agent's session, by its id, wherever it works.
+    StopAgentSession(String),
+    /// Stop every session that is here, and set the connection's access to
+    /// Off, so that an agent that connects again does not see it.
+    StopAgentsHere(ConnTabId),
+    /// Show the Agents tab of a connection's window: the one an agent
+    /// works on elsewhere. Its window is opened first where there is none.
+    ShowAgentsOn(ConnectionId),
+    /// Move the Agents tab's selection among its sessions, in the terminal
+    /// look.
+    MoveAgentSelection {
+        tab: ConnTabId,
+        step: isize,
+    },
     /// Keep what was changed outside the app and lets agents do more: what
     /// the bar said, and only what is still held of it.
     KeepAgentChanges(Vec<crate::agents_shown::Raised>),
@@ -2546,13 +2560,18 @@ impl Tab {
 /// The Agents tab: the app's own view of what agents do on the
 /// workspace's connection. What it shows lives in [`AgentsState`], not
 /// here: closing the tab stops nothing.
-#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+#[derive(Debug, Clone, PartialEq, Eq)]
 pub struct AgentsTab {
     pub id: TabId,
+    /// The terminal look's selected session, by its id: the one `S` stops
+    /// and `enter` shows. Kept by id, since the list puts the session that
+    /// called last first: a key lands on the session the person saw.
+    pub selected: Option<String>,
 }
 
 /// What the app knows of the agent server: the sessions open on it, as
-/// the backend last said them. Empty while the server is off.
+/// the backend last said them, empty while the server is off; and the
+/// calls that ended since the app started, the newest first.
 #[derive(Debug, Clone, Default, PartialEq, Eq)]
 pub struct AgentsState {
     pub sessions: Vec<tabletist_mcp::SessionInfo>,
@@ -2595,6 +2614,27 @@ impl AgentsState {
             .filter(move |call| call.connection.as_deref() == Some(conn.0.as_str()))
     }
 
+    /// The sessions that are not here: those that work only elsewhere, and
+    /// those that have named no connection yet. The one that called last
+    /// first, and those that have not called after them.
+    pub fn elsewhere(&self, conn: &ConnectionId) -> Vec<&tabletist_mcp::SessionInfo> {
+        let mut elsewhere: Vec<_> = self
+            .sessions
+            .iter()
+            .filter(|session| !session.connections.contains(&conn.0))
+            .collect();
+        elsewhere.sort_by_key(|session| std::cmp::Reverse(session.last_call_at));
+        elsewhere
+    }
+
+    /// The sessions that are here, then those that are elsewhere: the
+    /// terminal look's list, which its selection moves in.
+    pub fn listed(&self, conn: &ConnectionId) -> Vec<&tabletist_mcp::SessionInfo> {
+        let mut listed = self.here(conn);
+        listed.extend(self.elsewhere(conn));
+        listed
+    }
+
     /// The sessions that are here: those whose calls have named `conn`,
     /// the one that called last first.
     pub fn here(&self, conn: &ConnectionId) -> Vec<&tabletist_mcp::SessionInfo> {
@@ -3471,6 +3511,14 @@ impl Workspace {
         })
     }
 
+    /// The Agents tab, where it is open.
+    pub fn agents_tab_mut(&mut self) -> Option<&mut AgentsTab> {
+        self.tabs.iter_mut().find_map(|open| match open {
+            Tab::Agents(agents) => Some(agents),
+            Tab::Object(_) | Tab::Sql(_) => None,
+        })
+    }
+
     /// Whether the tab in front is the Agents tab.
     pub fn agents_in_front(&self) -> bool {
         self.active_tab.is_some() && self.active_tab == self.agents_tab()
````

- [ ] **Step 4: Run the tests**

Run: the same, then `cargo test --locked --lib app::`

Expected: PASS.

```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 2487 filtered out
test result: ok. 401 passed; 0 failed; 0 ignored; 0 measured; 2090 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src
git commit -m "Stop one agent's session or every one here, and show where an agent works"
```

---

### Task 4: The tab shows it

- macOS: the header's "Also:" names the session elsewhere that called last, with **Show**, and counts the others; **Stop agents here** at its right while anyone is here. Each session's row has "Runs as" and **Stop session** at its right. Under the sessions, the feed: Time, Tool, Request, Result, the newest first, or "No calls on this connection yet."
- Omarchy, as its board: the sessions at the left, those here then those elsewhere, one selected; at the right the selected one's line with `S stop session`, over the feed. With nobody here, the list is there beside the way to connect one. `j`/`k` move, `S` stops the selected session, `X` stops the agents here, `enter` shows where a session elsewhere works. Enter is the list's unless the person went to the tree, or a button has the keyboard; the arrows stay the tree's. The status line names them.
- **Stop agents here** is offered, in both looks, while agents are here on a connection they may use.
- **Stop agents here** says what it did: "Stopped 1 agent. Access is now Off for Bookshop · dev." in the toast, and on Omarchy in the status line.
- Two tests of 3a read the session's line as it is now drawn.
- The website: Omarchy's keys in the tab, and what **Stop agents here** writes in `connections.toml`.

**Files:**
- Modify: `src/ui/agents.rs`, `src/keymap.rs`, `src/ui/keys.rs`, `src/ui/workspace.rs`, `src/ui/agents_tests.rs`, `docs/_guide/omarchy.md`, `docs/_reference/settings-and-files.md`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index b03649f..d62c735 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -109,6 +109,10 @@ key. Nor has copying a whole row.
 | --- | --- |
 | `y` | Copy the command that connects an agent |
 | `o` | Turn the agent server on, while it is off |
+| `j` / `k` | Move among the agents' sessions: those here, then those elsewhere |
+| `S` | Stop the selected session, wherever it works |
+| `X` | Stop every agent here, and turn this connection's access off |
+| Enter | Show the window of the connection the selected agent works on |
 
 ### The Structure view
 
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 4886654..9b3b392 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -166,7 +166,9 @@ and `timeout_secs`, how long a statement may run (1 to 60). Without the
 table, or a key of it, a connection is `off`, with 500 rows and 10 seconds.
 A file in which a connection has the table says `version = 4`, and one in
 which none has it stays `version = 3`. Agents use the table only while
-`[mcp] enabled` is `true` in `settings.toml`.
+`[mcp] enabled` is `true` in `settings.toml`. **Stop agents here**, in a
+connection's Agents tab, stops the agents working on it and sets its
+`access` to `off`, so that an agent that connects again does not see it.
 
 ### Agent access changed outside Tabletist
 
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index 4c277e1..ab4c19a 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -244,15 +244,16 @@ fn the_tab_shows_its_state_and_its_buttons_do_what_they_say() {
         allow(&mut harness, tab, AgentAccess::Read);
         let here = session(&harness, Some(tab), "Claude Code");
         said(&mut harness, AgentsEvent::Sessions(vec![here]));
-        let name = if look.terminal {
-            "claude code"
+        // In the list at the left in the terminal look, its line over the
+        // calls; in a row with Stop session in the macOS look.
+        let said = if look.terminal {
+            "claude code  tabletist-repo · connected just now · 14 calls · runs as dev"
         } else {
-            "Claude Code"
+            "Claude Code · tabletist-repo · connected just now · 14 calls · Runs as dev"
         };
+        assert!(harness.has(said), "{}", look.name);
         assert!(
-            harness.has(&format!(
-                "{name} · tabletist-repo · connected just now · 14 calls · Runs as dev"
-            )),
+            harness.has(&look.label("No calls on this connection yet.")),
             "{}",
             look.name
         );
@@ -353,7 +354,7 @@ fn more_sessions_than_the_tab_has_room_for_scroll_to_the_last() {
     let last = "Agent 0 · tabletist-repo · connected just now · 14 calls · Runs as dev";
     let seen = |harness: &Harness| {
         harness
-            .painted_rect("Agent 0  tabletist-repo · connected just now · 14 calls · Runs as dev")
+            .painted_rect("Agent 0  tabletist-repo · connected just now · 14 calls")
             .is_some_and(|row| row.bottom() <= status.top())
     };
     assert!(!seen(&harness));
@@ -466,3 +467,293 @@ fn closing_the_tab_stops_nothing() {
     assert_eq!(harness.app.agents.sessions.len(), 1);
     assert_eq!(harness.app.backend.sent.len(), sent, "nothing is sent");
 }
+
+/// A saved connection elsewhere, Ledger · staging, and a session of
+/// `agent` that works there and only there.
+fn working_elsewhere(harness: &mut Harness, agent: &str) -> crate::connections::ConnectionId {
+    let mut ledger = crate::testing::fixture_connection();
+    ledger.id = crate::connections::ConnectionId("ledger".into());
+    ledger.name = "Ledger".into();
+    ledger.environment = crate::env::Environment::Staging;
+    harness.app.connections.upsert(ledger.clone());
+    let mut session = session(harness, None, agent);
+    session.connections = vec!["ledger".into()];
+    harness.app.agents.sessions.push(session);
+    ledger.id
+}
+
+/// A call of the session of `agent`'s on `tab`'s connection, or on another.
+fn call(harness: &Harness, tab: Option<ConnTabId>, sql: &str) -> tabletist_mcp::Call {
+    tabletist_mcp::Call {
+        at: SystemTime::now(),
+        session: "session of Claude Code".into(),
+        connection: Some(tab.map_or("ledger".into(), |tab| {
+            harness.app.workspace(tab).unwrap().conn_id.0.clone()
+        })),
+        tool: "run_query".into(),
+        request: Some(sql.into()),
+        result: "ok".into(),
+        message: None,
+        rows: Some(1),
+        columns: Some(1),
+        took: std::time::Duration::from_millis(2),
+    }
+}
+
+#[test]
+fn the_feed_shows_the_calls_on_this_connection_as_they_end() {
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        harness.app.settings.mcp_enabled = true;
+        allow(&mut harness, tab, AgentAccess::Read);
+        let here = session(&harness, Some(tab), "Claude Code");
+        said(&mut harness, AgentsEvent::Sessions(vec![here]));
+        harness.app.apply(Action::ShowAgents(tab));
+        let mine = call(&harness, Some(tab), "SELECT 1");
+        said(&mut harness, AgentsEvent::Called(mine));
+        let theirs = call(&harness, None, "SELECT 2");
+        said(&mut harness, AgentsEvent::Called(theirs));
+        let row = look.label("just now");
+        assert!(
+            harness.has(&format!("{row} · run_query · SELECT 1 · 1 row · 2 ms")),
+            "{}",
+            look.name
+        );
+        assert!(
+            !harness.has(&format!("{row} · run_query · SELECT 2 · 1 row · 2 ms")),
+            "{}",
+            look.name
+        );
+        // A refusal says why, as the log does.
+        let mut refused = call(&harness, Some(tab), "DELETE FROM books");
+        refused.result = "refused".into();
+        refused.message = Some("Only SELECT is read here.".into());
+        refused.rows = None;
+        said(&mut harness, AgentsEvent::Called(refused));
+        assert!(
+            harness.has(&format!(
+                "{row} · run_query · DELETE FROM books · refused · Only SELECT is read here."
+            )),
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn stop_session_and_stop_agents_here_are_the_tabs_buttons() {
+    let (mut harness, tab) = connected(Look::macos());
+    harness.app.settings.mcp_enabled = true;
+    allow(&mut harness, tab, AgentAccess::Read);
+    harness.app.serve_agents();
+    let here = session(&harness, Some(tab), "Claude Code");
+    said(&mut harness, AgentsEvent::Sessions(vec![here]));
+    harness.app.apply(Action::ShowAgents(tab));
+    harness.settle();
+    let stopped = |harness: &Harness| {
+        harness
+            .app
+            .backend
+            .sent
+            .iter()
+            .filter(|sent| {
+                matches!(sent, crate::backend::Command::Agents(
+                    crate::backend::AgentsCommand::StopSession(id)) if id == "session of Claude Code")
+            })
+            .count()
+    };
+    harness.click("Stop session");
+    harness.settle();
+    assert_eq!(stopped(&harness), 1);
+    harness.click("Stop agents here");
+    harness.settle();
+    assert_eq!(stopped(&harness), 2);
+    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+    assert_eq!(
+        harness.app.connections.get(&conn).unwrap().agents.access,
+        AgentAccess::Off
+    );
+    assert_eq!(
+        crate::ui::toast::said(&harness.ctx).as_deref(),
+        Some("Stopped 1 agent. Access is now Off for Fixture · dev.")
+    );
+    assert!(harness.has(&entry(&Look::macos(), "Off here")));
+}
+
+#[test]
+fn also_names_who_works_elsewhere_and_show_opens_its_window_on_its_agents() {
+    let (mut harness, tab) = connected(Look::macos());
+    harness.app.settings.mcp_enabled = true;
+    allow(&mut harness, tab, AgentAccess::Read);
+    let ledger = working_elsewhere(&mut harness, "Cursor");
+    harness.app.apply(Action::ShowAgents(tab));
+    harness.settle();
+    // Said in every state that the header has, nobody here too.
+    assert!(harness.has("Also: Cursor on Ledger · staging"));
+    let mut nowhere = session(&harness, None, "Zed");
+    nowhere.last_call_at = None;
+    harness.app.agents.sessions.push(nowhere);
+    assert!(harness.has("Also: Cursor on Ledger · staging and 1 more"));
+    let windows = harness.app.tabs.len();
+    harness.click("Show");
+    harness.settle();
+    assert_eq!(harness.app.tabs.len(), windows + 1);
+    let shown = harness.app.active_tab_id();
+    let workspace = harness.app.workspace(shown).unwrap();
+    assert_eq!(workspace.conn_id, ledger);
+    assert!(workspace.agents_in_front());
+}
+
+#[test]
+fn omarchys_keys_move_among_the_sessions_stop_one_and_show_where_one_works() {
+    let (mut harness, tab) = connected(Look::omarchy());
+    harness.app.settings.mcp_enabled = true;
+    allow(&mut harness, tab, AgentAccess::Read);
+    harness.app.serve_agents();
+    let here = session(&harness, Some(tab), "Claude Code");
+    harness.app.agents.sessions.push(here);
+    let ledger = working_elsewhere(&mut harness, "Cursor");
+    harness.app.apply(Action::ShowAgents(tab));
+    harness.settle();
+    let selected = |harness: &Harness| harness.app.agents_selected(tab).map(|s| s.id.clone());
+    assert_eq!(
+        selected(&harness).as_deref(),
+        Some("session of Claude Code")
+    );
+    // Enter on a session here shows nothing more.
+    let windows = harness.app.tabs.len();
+    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+    assert_eq!(harness.app.tabs.len(), windows);
+    super::tests::type_key(&mut harness, egui::Key::J, "j");
+    assert_eq!(selected(&harness).as_deref(), Some("session of Cursor"));
+    // Past the last, it stays.
+    super::tests::type_key(&mut harness, egui::Key::J, "j");
+    assert_eq!(selected(&harness).as_deref(), Some("session of Cursor"));
+    // `S` stops the selected one, wherever it works.
+    super::tests::type_key(&mut harness, egui::Key::S, "S");
+    assert!(harness.app.backend.sent.iter().any(|sent| matches!(sent,
+        crate::backend::Command::Agents(crate::backend::AgentsCommand::StopSession(id))
+            if id == "session of Cursor")));
+    // Enter shows its connection's window, on its Agents tab.
+    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+    assert_eq!(harness.app.tabs.len(), windows + 1);
+    let shown = harness.app.active_tab_id();
+    assert_eq!(harness.app.workspace(shown).unwrap().conn_id, ledger);
+    // Back, `k`, and `X` stops who is here and turns access Off.
+    harness.app.apply(Action::ActivateConnTab(tab));
+    super::tests::type_key(&mut harness, egui::Key::K, "k");
+    super::tests::type_key(&mut harness, egui::Key::X, "X");
+    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+    assert_eq!(
+        harness.app.connections.get(&conn).unwrap().agents.access,
+        AgentAccess::Off
+    );
+    // This look floats no toast: its status line says it.
+    assert!(
+        harness
+            .has(&Look::omarchy().label("Stopped 1 agent. Access is now Off for Fixture · dev."))
+    );
+}
+
+/// Two sessions here: A, which called last, then B.
+fn two_here(harness: &mut Harness, tab: ConnTabId) {
+    let now = SystemTime::now();
+    let mut a = session(harness, Some(tab), "A");
+    a.last_call_at = Some(now);
+    let mut b = session(harness, Some(tab), "B");
+    b.last_call_at = Some(now - std::time::Duration::from_secs(5));
+    said(harness, AgentsEvent::Sessions(vec![a, b]));
+}
+
+#[test]
+fn the_selection_stays_on_the_session_the_person_saw_when_the_list_reorders() {
+    let (mut harness, tab) = connected(Look::omarchy());
+    harness.app.settings.mcp_enabled = true;
+    allow(&mut harness, tab, AgentAccess::Read);
+    harness.app.serve_agents();
+    two_here(&mut harness, tab);
+    harness.app.apply(Action::ShowAgents(tab));
+    super::tests::type_key(&mut harness, egui::Key::J, "j");
+    let selected = |harness: &Harness| harness.app.agents_selected(tab).map(|s| s.id.clone());
+    assert_eq!(selected(&harness).as_deref(), Some("session of B"));
+    // B calls: it goes first in the list, and stays the one selected.
+    let mut sessions = harness.app.agents.sessions.clone();
+    sessions[1].last_call_at = Some(SystemTime::now() + std::time::Duration::from_secs(20));
+    said(&mut harness, AgentsEvent::Sessions(sessions.clone()));
+    assert_eq!(selected(&harness).as_deref(), Some("session of B"));
+    super::tests::type_key(&mut harness, egui::Key::S, "S");
+    let stopped: Vec<_> = harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .filter_map(|sent| match sent {
+            crate::backend::Command::Agents(crate::backend::AgentsCommand::StopSession(id)) => {
+                Some(id.as_str())
+            }
+            _ => None,
+        })
+        .collect();
+    assert_eq!(stopped, ["session of B"]);
+    // B ends: the first is selected, and `k` from it stays there.
+    said(
+        &mut harness,
+        AgentsEvent::Sessions(vec![sessions[0].clone()]),
+    );
+    assert_eq!(selected(&harness).as_deref(), Some("session of A"));
+    super::tests::type_key(&mut harness, egui::Key::K, "k");
+    assert_eq!(selected(&harness).as_deref(), Some("session of A"));
+}
+
+#[test]
+fn with_sessions_here_the_arrows_still_move_in_the_tree_and_enter_leaves_a_button_its_own() {
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        harness.app.settings.mcp_enabled = true;
+        allow(&mut harness, tab, AgentAccess::Read);
+        let here = session(&harness, Some(tab), "Claude Code");
+        said(&mut harness, AgentsEvent::Sessions(vec![here]));
+        harness.app.apply(Action::ShowAgents(tab));
+        harness.settle();
+        let cursor = |harness: &Harness| harness.app.workspace(tab).unwrap().tree.cursor.clone();
+        let before = cursor(&harness);
+        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
+        assert_ne!(cursor(&harness), before, "{}", look.name);
+    }
+    // Enter on a focused button is the button's: here, Disconnect.
+    let (mut harness, tab) = connected(Look::omarchy());
+    harness.app.settings.mcp_enabled = true;
+    allow(&mut harness, tab, AgentAccess::Read);
+    let here = session(&harness, Some(tab), "Claude Code");
+    said(&mut harness, AgentsEvent::Sessions(vec![here]));
+    harness.app.apply(Action::ShowAgents(tab));
+    harness.press(egui::Key::Tab, egui::Modifiers::NONE);
+    harness.press(egui::Key::Tab, egui::Modifiers::NONE);
+    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+    assert!(harness.app.workspace(tab).is_none(), "disconnected");
+}
+
+#[test]
+fn omarchy_lists_who_works_elsewhere_while_nobody_is_here() {
+    let (mut harness, tab) = connected(Look::omarchy());
+    harness.app.settings.mcp_enabled = true;
+    allow(&mut harness, tab, AgentAccess::Read);
+    harness.app.serve_agents();
+    let ledger = working_elsewhere(&mut harness, "Cursor");
+    harness.app.apply(Action::ShowAgents(tab));
+    harness.settle();
+    assert!(harness.has(&entry(&Look::omarchy(), "None connected")));
+    assert!(harness.has("cursor · Ledger · staging · enter shows"));
+    // With the way to connect one beside it.
+    assert!(harness.has(crate::endpoint::agent_command()));
+    // `X` has nobody here to stop: nothing changes.
+    super::tests::type_key(&mut harness, egui::Key::X, "X");
+    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+    assert_eq!(
+        harness.app.connections.get(&conn).unwrap().agents.access,
+        AgentAccess::Read
+    );
+    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+    let shown = harness.app.active_tab_id();
+    assert_eq!(harness.app.workspace(shown).unwrap().conn_id, ledger);
+}
````

- [ ] **Step 2: Run them to see that they fail**

Run: `cargo test --locked --lib ui::agents_tests`

Expected: eight FAIL. Nothing draws the stops, "Also:" or the feed yet, and the terminal look has no list of sessions nor its keys:

```text
test ui::agents_tests::omarchy_lists_who_works_elsewhere_while_nobody_is_here ... FAILED
test ui::agents_tests::stop_session_and_stop_agents_here_are_the_tabs_buttons ... FAILED
test ui::agents_tests::also_names_who_works_elsewhere_and_show_opens_its_window_on_its_agents ... FAILED
test ui::agents_tests::the_selection_stays_on_the_session_the_person_saw_when_the_list_reorders ... FAILED
test ui::agents_tests::the_feed_shows_the_calls_on_this_connection_as_they_end ... FAILED
test ui::agents_tests::omarchys_keys_move_among_the_sessions_stop_one_and_show_where_one_works ... FAILED
test ui::agents_tests::the_tab_shows_its_state_and_its_buttons_do_what_they_say ... FAILED
test ui::agents_tests::more_sessions_than_the_tab_has_room_for_scroll_to_the_last ... FAILED
test result: FAILED. 12 passed; 8 failed; 0 ignored; 0 measured; 2478 filtered out
```

- [ ] **Step 3: Draw it, and its keys**

````diff
diff --git a/src/keymap.rs b/src/keymap.rs
index 24073a6..98ceae5 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -334,6 +334,10 @@ commands! {
     ShowAgents,
     CopyAgentCommand,
     TurnAgentsOn,
+    MoveInAgentSessions,
+    StopAgentSession,
+    StopAgentsHere,
+    ShowAgentsElsewhere,
     ToggleSidebar,
     PaneLeft,
     PaneDown,
@@ -571,6 +575,16 @@ impl Command {
                 agents(info("Copy the command that connects an agent", "", Window))
             }
             C::TurnAgentsOn => agents(info("Turn the agent server on", "", Window)),
+            C::MoveInAgentSessions => agents(info("Move in the agents' sessions", "", Window)),
+            C::StopAgentSession => agents(info("Stop the selected agent's session", "", Window)),
+            C::StopAgentsHere => agents(info(
+                "Stop the agents here, and turn their access off",
+                "",
+                Window,
+            )),
+            C::ShowAgentsElsewhere => {
+                agents(info("Show where the selected agent works", "", Window))
+            }
             C::ToggleSidebar => info("Toggle sidebar", "Toggle sidebar", Navigation),
             C::PaneLeft => info("Pane to the left", PANES, Navigation),
             C::PaneDown => info("Pane below", PANES, Navigation).reserved(),
@@ -956,6 +970,15 @@ pub const BINDINGS: &[Binding] = {
         bind(C::ShowAgents, TREE_AND_GRID, NONE, keys(&[k("ga")])),
         bind(C::CopyAgentCommand, AGENTS, NONE, keys(&[k("y")])),
         bind(C::TurnAgentsOn, AGENTS, NONE, keys(&[k("o")])),
+        bind(
+            C::MoveInAgentSessions,
+            AGENTS,
+            NONE,
+            keys(&[k("j"), k("k")]).shown("j/k"),
+        ),
+        bind(C::StopAgentSession, AGENTS, NONE, keys(&[k("S")])),
+        bind(C::StopAgentsHere, AGENTS, NONE, keys(&[k("X")])),
+        bind(C::ShowAgentsElsewhere, AGENTS, NONE, keys(&[k("enter")])),
         bind(
             C::ToggleSidebar,
             GLOBAL,
diff --git a/src/ui/agents.rs b/src/ui/agents.rs
index c3a9701..be04014 100644
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -1,7 +1,8 @@
 //! The Agents tab: what agents do on the window's connection. One state
 //! of four, as the sidebar's Agents entry names it: the server off, access
 //! Off here, nobody here with the command that connects one, or the
-//! sessions that are here.
+//! sessions that are here with what they called. The sessions that work
+//! elsewhere are "Also:", and each can be stopped.
 
 use egui::{Frame, Id, Margin, Rect, pos2, vec2};
 
@@ -59,12 +60,26 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     });
     let sessions: Vec<tabletist_mcp::SessionInfo> =
         app.agents.here(&conn).into_iter().cloned().collect();
+    let elsewhere: Vec<Elsewhere> = app
+        .agents
+        .elsewhere(&conn)
+        .into_iter()
+        .map(|session| Elsewhere::of(app, session))
+        .collect();
+    let calls: Vec<tabletist_mcp::Call> = app.agents.calls_on(&conn).cloned().collect();
+    let selected = app.agents_selected(tab).map(|session| session.id.clone());
     let layout = app.layout();
     let show_key = app.keymap.key(layout, crate::keymap::Command::ShowAgents);
     let copy_key = app
         .keymap
         .key(layout, crate::keymap::Command::CopyAgentCommand);
     let on_key = app.keymap.key(layout, crate::keymap::Command::TurnAgentsOn);
+    let stop_key = app
+        .keymap
+        .key(layout, crate::keymap::Command::StopAgentSession);
+    let show_elsewhere_key = app
+        .keymap
+        .key(layout, crate::keymap::Command::ShowAgentsElsewhere);
     let mut actions = Vec::new();
     egui::Panel::top(Id::new(("agents-header", tab.0)))
         .exact_size(HEADER)
@@ -83,12 +98,48 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 palette.border
             };
             widgets::hline(ui, full.x_range(), full.bottom() - 0.5, rule);
+            // At the right, and in the macOS look alone: Stop agents here
+            // while anyone is, and before it who works elsewhere. The
+            // terminal look lists those under its sessions, with its keys.
+            let mut right = full.right() - 16.0;
+            if !look.terminal && matches!(here, AgentsHere::Connected { .. }) {
+                let label = gettext(locale, "Stop agents here");
+                let button = states::button(&label, &look).drops();
+                let width = button.width(ui, &look);
+                let at = Rect::from_center_size(
+                    pos2(right - width / 2.0, full.center().y),
+                    vec2(width, 28.0),
+                );
+                if button.show_at(ui, at, &look, &palette).clicked() {
+                    stop_here(
+                        ui.ctx(),
+                        tab,
+                        sessions.len(),
+                        &connection,
+                        locale,
+                        &mut actions,
+                    );
+                }
+                right -= width + 12.0;
+            }
+            if !look.terminal
+                && let Some(first) = elsewhere.first()
+            {
+                right = also(
+                    ui,
+                    (full, right),
+                    first,
+                    elsewhere.len() - 1,
+                    (&look, &palette, locale),
+                    &mut actions,
+                ) - 16.0;
+            }
             let on = look.label(&gettext(locale, "Agents on"));
             // A connection's name gives way at its end to a narrow tab.
             let strong = TextRole::pick(&look, TextRole::UiBodyStrong, TextRole::OBody);
             let body = widgets::body(&look);
             let around = body.width(ui.ctx(), look.faces, &format!("{on}  · {access}"));
-            let room = (full.width() - 32.0 - around).max(0.0);
+            let room = (right - full.left() - 16.0 - around).max(0.0);
             let name = crate::ui::grid::ellipsize(&connection, room, false, |text| {
                 strong.width(ui.ctx(), look.faces, text)
             });
@@ -139,18 +190,37 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     };
                     states::empty(ui, rect, &notice, Vec::new(), &look, &palette);
                 }
+                AgentsHere::NoneConnected if look.terminal && !elsewhere.is_empty() => {
+                    // Who works elsewhere is listed at the left, as with
+                    // agents here, and the way to connect one beside it.
+                    let listed = (&sessions[..], &elsewhere[..], selected.as_deref());
+                    terminal_nav(ui, tab, listed, &show_elsewhere_key, (&look, &palette, locale));
+                    let name = app.workspace(tab).map(|w| w.name.clone()).unwrap_or_default();
+                    egui::CentralPanel::default()
+                        .frame(Frame::new().fill(palette.window))
+                        .show(ui, |ui| {
+                            let rect = ui.max_rect();
+                            none_connected(ui, rect, (&connection, &name, &access), &copy_key, (&look, &palette, locale));
+                        });
+                }
                 AgentsHere::NoneConnected => {
                     let name = app.workspace(tab).map(|w| w.name.clone()).unwrap_or_default();
                     none_connected(ui, rect, (&connection, &name, &access), &copy_key, (&look, &palette, locale));
                 }
+                AgentsHere::Connected { .. } if look.terminal => {
+                    let keys = (&stop_key, &show_elsewhere_key);
+                    let listed = (&sessions[..], &elsewhere[..], selected.as_deref());
+                    terminal_activity(ui, tab, listed, &calls, keys, runs_as.as_deref(), (&look, &palette, locale));
+                }
                 AgentsHere::Connected { .. } => {
-                    sessions_here(
+                    let stopped = activity(
                         ui,
                         rect,
-                        &sessions,
+                        (&sessions, &calls),
                         runs_as.as_deref(),
                         (&look, &palette, locale),
                     );
+                    actions.extend(stopped.into_iter().map(Action::StopAgentSession));
                 }
             }
         });
@@ -339,81 +409,189 @@ fn none_connected(
     );
 }
 
-/// The sessions that are here, the one that called last first: who, where
-/// it was started, since when, how many calls, and as what it runs.
-fn sessions_here(
-    ui: &mut egui::Ui,
-    rect: Rect,
-    sessions: &[tabletist_mcp::SessionInfo],
-    runs_as: Option<&str>,
-    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
-) {
-    let row = if look.terminal { 22.0 } else { 44.0 };
-    // More sessions than the tab has the height for scroll, under its
-    // header and above its status bar.
-    let mut area = ui.new_child(egui::UiBuilder::new().max_rect(rect));
-    egui::ScrollArea::vertical()
-        .id_salt("agents-sessions")
-        .auto_shrink([false, false])
-        .show(&mut area, |ui| {
-            let height = 12.0 + row * sessions.len() as f32 + 12.0;
-            let (rect, _) =
-                ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
-            session_rows(ui, rect, sessions, runs_as, (look, palette, locale));
-        });
+/// A session that works elsewhere, as "Also:" names it: its agent, and
+/// the connection it named last, by the name an agent sees, where that is
+/// a connection that is saved.
+pub struct Elsewhere {
+    pub id: String,
+    pub agent: String,
+    pub on: Option<(crate::connections::ConnectionId, String)>,
 }
 
-/// The sessions' rows in `rect`, from its top: one for each, `row` high.
+impl Elsewhere {
+    pub fn of(app: &App, session: &tabletist_mcp::SessionInfo) -> Self {
+        let saved = &app.connections.connections;
+        let names = crate::backend::agent_names(saved);
+        let on = session.connections.iter().rev().find_map(|id| {
+            saved
+                .iter()
+                .zip(&names)
+                .find(|(saved, _)| saved.id.0 == *id)
+                .map(|(saved, name)| (saved.id.clone(), name.clone()))
+        });
+        Self {
+            id: session.id.clone(),
+            agent: crate::model::agent_name(session),
+            on,
+        }
+    }
+
+    /// "Cursor on Bookshop · staging", or "Cursor · no connection yet".
+    fn said(&self, locale: crate::i18n::Locale) -> String {
+        match &self.on {
+            Some((_, name)) => format!("{} {} {name}", self.agent, gettext(locale, "on")),
+            None => format!("{} · {}", self.agent, gettext(locale, "no connection yet")),
+        }
+    }
+}
+
+/// Stops the agents here, and says so: the toast names how many and the
+/// connection whose access is Off now.
+pub fn stop_here(
+    ctx: &egui::Context,
+    tab: ConnTabId,
+    count: usize,
+    connection: &str,
+    locale: crate::i18n::Locale,
+    actions: &mut Vec<Action>,
+) {
+    actions.push(Action::StopAgentsHere(tab));
+    let said = ngettext(
+        locale,
+        "Stopped {n} agent. Access is now Off for {name}.",
+        "Stopped {n} agents. Access is now Off for {name}.",
+        u32::try_from(count).unwrap_or(u32::MAX),
+    )
+    .replace("{n}", &count.to_string())
+    .replace("{name}", connection);
+    crate::ui::toast::say(ctx, &said);
+}
+
+/// The macOS header's "Also:": the session elsewhere that called last,
+/// with Show where it works on a saved connection, and how many more there
+/// are. Drawn from `right` leftwards on `full`'s line; returns where it
+/// starts.
+fn also(
+    ui: &mut egui::Ui,
+    (full, right): (Rect, f32),
+    first: &Elsewhere,
+    more: usize,
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+    actions: &mut Vec<Action>,
+) -> f32 {
+    let mut right = right;
+    if let Some((conn, _)) = &first.on {
+        let label = gettext(locale, "Show");
+        let show = widgets::ButtonSpec::new(&label).link().padding(6.0);
+        let width = show.width(ui, look);
+        let at = Rect::from_center_size(
+            pos2(right - width / 2.0, full.center().y),
+            vec2(width, 24.0),
+        );
+        if show.show_at(ui, at, look, palette).clicked() {
+            actions.push(Action::ShowAgentsOn(conn.clone()));
+        }
+        right -= width + 4.0;
+    }
+    let mut said = format!("{} {}", gettext(locale, "Also:"), first.said(locale));
+    if more > 0 {
+        said.push_str(
+            &ngettext(
+                locale,
+                " and {n} more",
+                " and {n} more",
+                u32::try_from(more).unwrap_or(u32::MAX),
+            )
+            .replace("{n}", &more.to_string()),
+        );
+    }
+    let laid = Text::one(look, widgets::secondary(look), &said, palette.secondary).layout(ui.ctx());
+    let left = right - laid.width();
+    laid.paint_left(ui.painter(), left, full.center().y);
+    widgets::announce(
+        ui,
+        Rect::from_min_max(pos2(left, full.top()), pos2(right, full.bottom())),
+        &said,
+    );
+    left
+}
+
+/// The macOS look's activity: the sessions that are here, each with Stop
+/// session, and under them the calls on this connection. Returns the
+/// sessions whose Stop session was clicked.
+fn activity(
+    ui: &mut egui::Ui,
+    rect: Rect,
+    (sessions, calls): (&[tabletist_mcp::SessionInfo], &[tabletist_mcp::Call]),
+    runs_as: Option<&str>,
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+) -> Vec<String> {
+    let mut stopped = Vec::new();
+    // More than the tab has the height for scrolls, under its header and
+    // above its status bar.
+    let mut area = ui.new_child(egui::UiBuilder::new().max_rect(rect));
+    egui::ScrollArea::vertical()
+        .id_salt("agents-activity")
+        .auto_shrink([false, false])
+        .show(&mut area, |ui| {
+            let rows = SESSION_ROW * sessions.len() as f32;
+            let height = rows + 16.0 + feed_height(calls.len(), look);
+            let (rect, _) =
+                ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
+            stopped = session_rows(ui, rect, sessions, runs_as, (look, palette, locale));
+            let feed_rect =
+                Rect::from_min_max(pos2(rect.left(), rect.top() + rows + 16.0), rect.max);
+            feed(ui, feed_rect, calls, (look, palette, locale));
+        });
+    stopped
+}
+
+/// A session's row in the macOS look.
+const SESSION_ROW: f32 = 44.0;
+
+/// The sessions' rows in `rect`, from its top: who, where it was started,
+/// since when and how many calls; at the right as what it runs, and Stop
+/// session. Returns the sessions whose Stop session was clicked.
 fn session_rows(
     ui: &mut egui::Ui,
     rect: Rect,
     sessions: &[tabletist_mcp::SessionInfo],
     runs_as: Option<&str>,
     (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
-) {
-    let row = if look.terminal { 22.0 } else { 44.0 };
-    let mut top = rect.top() + 12.0;
-    let now = std::time::SystemTime::now();
+) -> Vec<String> {
+    let mut stopped = Vec::new();
+    let mut top = rect.top();
+    let label = gettext(locale, "Stop session");
     for session in sessions {
-        let line = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), row));
+        let line = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), SESSION_ROW));
         let agent = crate::model::agent_name(session);
-        let agent = if look.terminal {
-            look.label(&agent)
-        } else {
-            agent
-        };
-        let since = now.duration_since(session.connected_at).unwrap_or_default();
-        let mut facts = Vec::new();
-        if let Some(project) = &session.project {
-            facts.push(project.clone());
-        }
-        facts.push(format!(
-            "{} {}",
-            gettext(locale, "connected"),
-            ago(since, locale)
-        ));
-        facts.push(
-            ngettext(
-                locale,
-                "{n} call",
-                "{n} calls",
-                u32::try_from(session.calls).unwrap_or(u32::MAX),
-            )
-            .replace("{n}", &session.calls.to_string()),
+        let facts = session_facts(session, locale);
+        // Stop session at the right, and as what it runs before it.
+        let button = states::button(&label, look).salt(&session.id);
+        let width = button.width(ui, look);
+        let at = Rect::from_center_size(
+            pos2(line.right() - 16.0 - width / 2.0, line.center().y),
+            vec2(width, 26.0),
         );
-        // A connection with no environment runs as nothing to name.
+        if button.show_at(ui, at, look, palette).clicked() {
+            stopped.push(session.id.clone());
+        }
+        let mut right = at.left() - 12.0;
+        let small = widgets::secondary(look);
+        let mut said = format!("{agent} · {facts}");
         if let Some(runs_as) = runs_as {
-            facts.push(format!("{} {runs_as}", gettext(locale, "Runs as")));
+            let runs = format!("{} {runs_as}", gettext(locale, "Runs as"));
+            let laid = Text::one(look, small, &runs, palette.secondary).layout(ui.ctx());
+            laid.paint_left(ui.painter(), right - laid.width(), line.center().y);
+            right -= laid.width() + 16.0;
+            said = format!("{said} · {runs}");
         }
-        let facts = facts.join(" · ");
-        let dot = crate::ui::states::Tone::Success.color(palette);
         let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OBody);
         // What does not fit the line gives way at its end, the name last.
-        let room = (line.width() - 30.0 - 16.0).max(0.0);
+        let room = (right - line.left() - 30.0).max(0.0);
         let name_width = |text: &str| strong.width(ui.ctx(), look.faces, text);
         let shown_agent = crate::ui::grid::ellipsize(&agent, room, false, name_width);
         let after = (room - name_width(&shown_agent) - name_width("  ")).max(0.0);
-        let small = widgets::secondary(look);
         let shown_facts = crate::ui::grid::ellipsize(&facts, after, false, |text| {
             small.width(ui.ctx(), look.faces, text)
         });
@@ -422,18 +600,408 @@ fn session_rows(
             .space(strong, "  ")
             .add(small, &shown_facts, palette.secondary);
         // The dot is painted: not every face has the glyph.
+        let dot = states::Tone::Success.color(palette);
         ui.painter()
             .circle_filled(pos2(line.left() + 19.5, line.center().y), 3.5, dot);
-        let x = line.left() + 30.0;
-        widgets::paint_text(ui, x, line.center().y, text);
-        widgets::announce(ui, line, &format!("{agent} · {facts}"));
+        widgets::paint_text(ui, line.left() + 30.0, line.center().y, text);
+        widgets::announce(ui, line, &said);
+        widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.border);
+        top += SESSION_ROW;
+    }
+    stopped
+}
+
+/// Where a session was started, since when, and how many calls it made.
+fn session_facts(session: &tabletist_mcp::SessionInfo, locale: crate::i18n::Locale) -> String {
+    let since = std::time::SystemTime::now()
+        .duration_since(session.connected_at)
+        .unwrap_or_default();
+    let mut facts = Vec::new();
+    if let Some(project) = &session.project {
+        facts.push(project.clone());
+    }
+    facts.push(format!(
+        "{} {}",
+        gettext(locale, "connected"),
+        ago(since, locale)
+    ));
+    facts.push(
+        ngettext(
+            locale,
+            "{n} call",
+            "{n} calls",
+            u32::try_from(session.calls).unwrap_or(u32::MAX),
+        )
+        .replace("{n}", &session.calls.to_string()),
+    );
+    facts.join(" · ")
+}
+
+/// The terminal look's activity, as its board draws it: at the left the
+/// sessions here, then those elsewhere, one of them selected; at the right
+/// the selected one with its key, over the calls on this connection.
+fn terminal_activity(
+    ui: &mut egui::Ui,
+    tab: ConnTabId,
+    (here, elsewhere, selected): (&[tabletist_mcp::SessionInfo], &[Elsewhere], Option<&str>),
+    calls: &[tabletist_mcp::Call],
+    (stop_key, show_key): (&crate::keymap::Written, &crate::keymap::Written),
+    runs_as: Option<&str>,
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+) {
+    terminal_nav(
+        ui,
+        tab,
+        (here, elsewhere, selected),
+        show_key,
+        (look, palette, locale),
+    );
+    egui::CentralPanel::default()
+        .frame(Frame::new().fill(palette.window))
+        .show(ui, |ui| {
+            let full = ui.max_rect();
+            // The selected session's line, where it is one of those here.
+            let chosen = here
+                .iter()
+                .find(|session| selected == Some(session.id.as_str()));
+            let line = Rect::from_min_size(full.min, vec2(full.width(), 34.0));
+            if let Some(session) = chosen {
+                ui.painter().rect_filled(line, 0.0, palette.panel);
+                widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.outline);
+                let mut facts = session_facts(session, locale);
+                if let Some(runs_as) = runs_as {
+                    facts = format!("{facts} · {} {runs_as}", gettext(locale, "runs as"));
+                }
+                let agent = look.label(&crate::model::agent_name(session));
+                let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OBody);
+                let body = widgets::body(look);
+                let stop = Text::new(look)
+                    .add(body, stop_key.as_str(), palette.accent)
+                    .space(body, " ")
+                    .add(body, &gettext(locale, "stop session"), palette.secondary)
+                    .layout(ui.ctx());
+                let right = line.right() - 12.0 - stop.width();
+                stop.paint_left(ui.painter(), right, line.center().y);
+                let room = (right - line.left() - 24.0).max(0.0);
+                let text = format!("{agent}  {facts}");
+                let shown = crate::ui::grid::ellipsize(&text, room, false, |text| {
+                    body.width(ui.ctx(), look.faces, text)
+                });
+                // The name in the text colour, as far as it was not cut.
+                let (name, rest) = match shown.strip_prefix(agent.as_str()) {
+                    Some(rest) => (agent.as_str(), rest),
+                    None => (shown.as_ref(), ""),
+                };
+                let laid = Text::new(look).add(strong, name, palette.text).add(
+                    body,
+                    rest,
+                    palette.secondary,
+                );
+                widgets::paint_text(ui, line.left() + 12.0, line.center().y, laid);
+                widgets::announce(ui, line, &text);
+            }
+            let top = if chosen.is_some() {
+                line.bottom() + 14.0
+            } else {
+                full.top() + 14.0
+            };
+            let area_rect = Rect::from_min_max(
+                pos2(full.left() + 14.0, top),
+                pos2(full.right() - 14.0, full.bottom()),
+            );
+            let mut area = ui.new_child(egui::UiBuilder::new().max_rect(area_rect));
+            egui::ScrollArea::vertical()
+                .id_salt("agents-feed")
+                .auto_shrink([false, false])
+                .show(&mut area, |ui| {
+                    let height = feed_height(calls.len(), look);
+                    let (rect, _) = ui.allocate_exact_size(
+                        vec2(ui.available_width(), height),
+                        egui::Sense::hover(),
+                    );
+                    feed(ui, rect, calls, (look, palette, locale));
+                });
+        });
+}
+
+/// The terminal look's list of sessions, at the left: those here, then
+/// those elsewhere, the selected one filled.
+fn terminal_nav(
+    ui: &mut egui::Ui,
+    tab: ConnTabId,
+    (here, elsewhere, selected): (&[tabletist_mcp::SessionInfo], &[Elsewhere], Option<&str>),
+    show_key: &crate::keymap::Written,
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+) {
+    let shows = format!("{} {}", show_key.as_str(), gettext(locale, "shows"));
+    // A narrow window gives the calls the room the list would take.
+    let nav = NAV.min(ui.available_width() * 0.35);
+    egui::Panel::left(Id::new(("agents-sessions", tab.0)))
+        .exact_size(nav)
+        .resizable(false)
+        .show_separator_line(false)
+        .frame(Frame::new().fill(palette.window))
+        .show(ui, |ui| {
+            let full = ui.max_rect();
+            widgets::vline(ui, full.right() - 0.5, full.y_range(), palette.outline);
+            let mut y = full.top() + 10.0;
+            nav_group(
+                ui,
+                (full, &mut y),
+                &gettext(locale, "On this connection"),
+                (look, palette),
+            );
+            for session in here {
+                let first = look.label(&crate::model::agent_name(session));
+                let second = session_facts(session, locale);
+                let chosen = selected == Some(session.id.as_str());
+                nav_row(
+                    ui,
+                    (full, &mut y),
+                    (&first, &second),
+                    chosen,
+                    (look, palette),
+                );
+            }
+            if !elsewhere.is_empty() {
+                y += 8.0;
+                nav_group(
+                    ui,
+                    (full, &mut y),
+                    &gettext(locale, "Elsewhere"),
+                    (look, palette),
+                );
+            }
+            for other in elsewhere {
+                let first = look.label(&other.agent);
+                let second = match &other.on {
+                    Some((_, name)) => format!("{name} · {shows}"),
+                    None => gettext(locale, "no connection yet").into_owned(),
+                };
+                let chosen = selected == Some(other.id.as_str());
+                nav_row(
+                    ui,
+                    (full, &mut y),
+                    (&first, &second),
+                    chosen,
+                    (look, palette),
+                );
+            }
+        });
+}
+
+/// The terminal look's list of sessions: its width.
+const NAV: f32 = 300.0;
+
+/// A group's title in the terminal look's list of sessions.
+fn nav_group(
+    ui: &egui::Ui,
+    (full, y): (Rect, &mut f32),
+    title: &str,
+    (look, palette): (&Look, &Palette),
+) {
+    let laid = Text::one(
+        look,
+        widgets::body(look),
+        &look.label(title),
+        palette.secondary,
+    )
+    .layout(ui.ctx());
+    laid.paint_left(ui.painter(), full.left() + 14.0, *y + 11.0);
+    *y += 22.0;
+}
+
+/// A session in the terminal look's list: its agent, and a line under it.
+/// The selected one is filled, with the accent at its left edge.
+fn nav_row(
+    ui: &egui::Ui,
+    (full, y): (Rect, &mut f32),
+    (first, second): (&str, &str),
+    chosen: bool,
+    (look, palette): (&Look, &Palette),
+) {
+    let rect = Rect::from_min_size(pos2(full.left(), *y), vec2(full.width() - 1.0, 44.0));
+    if chosen {
+        ui.painter().rect_filled(rect, 0.0, palette.selection);
+        let edge = Rect::from_min_size(rect.min, vec2(2.0, rect.height()));
+        ui.painter().rect_filled(edge, 0.0, palette.accent);
+    }
+    let body = widgets::body(look);
+    let room = (rect.width() - 28.0 - 14.0).max(0.0);
+    let fit = |text: &str| -> String {
+        crate::ui::grid::ellipsize(text, room, false, |text| {
+            body.width(ui.ctx(), look.faces, text)
+        })
+        .into_owned()
+    };
+    let dot = states::Tone::Success.color(palette);
+    ui.painter()
+        .circle_filled(pos2(rect.left() + 18.0, rect.top() + 12.0), 3.5, dot);
+    let laid = Text::one(look, body, &fit(first), palette.text).layout(ui.ctx());
+    laid.paint_left(ui.painter(), rect.left() + 28.0, rect.top() + 12.0);
+    let laid = Text::one(look, body, &fit(second), palette.secondary).layout(ui.ctx());
+    laid.paint_left(ui.painter(), rect.left() + 28.0, rect.top() + 32.0);
+    widgets::announce(ui, rect, &format!("{first} · {second}"));
+    *y += 48.0;
+}
+
+/// A line of the feed, and its head.
+fn feed_row(look: &Look) -> f32 {
+    if look.terminal { 26.0 } else { 30.0 }
+}
+
+/// The feed's height: its head, and a line for each call or for saying
+/// there is none yet.
+fn feed_height(calls: usize, look: &Look) -> f32 {
+    feed_row(look) * (1 + calls.max(1)) as f32
+}
+
+/// The calls on this connection, the newest first: when, the tool, what
+/// it asked and how it ended, under a head that names the four.
+fn feed(
+    ui: &egui::Ui,
+    rect: Rect,
+    calls: &[tabletist_mcp::Call],
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+) {
+    let row = feed_row(look);
+    let body = widgets::body(look);
+    let small = widgets::secondary(look);
+    let mono = TextRole::pick(look, TextRole::GridCell, TextRole::OBody);
+    // Time, tool, request, result: the request takes what is left, and
+    // in a narrow tab the tool and the result give way first.
+    let room = (rect.width() - 24.0).max(0.0);
+    let time = 90.0_f32.min(room * 0.2);
+    let (tool, result) = (150.0_f32.min(room * 0.2), 230.0_f32.min(room * 0.3));
+    let request = (room - time - tool - result).max(0.0);
+    let columns = [
+        (rect.left() + 12.0, time),
+        (rect.left() + 12.0 + time, tool),
+        (rect.left() + 12.0 + time + tool, request),
+        (rect.right() - 12.0 - result, result),
+    ];
+    let cell =
+        |ui: &egui::Ui, (x, width): (f32, f32), y: f32, role: TextRole, text: &str, color| {
+            let shown = crate::ui::grid::ellipsize(text, width - 10.0, false, |text| {
+                role.width(ui.ctx(), look.faces, text)
+            });
+            Text::one(look, role, &shown, color)
+                .layout(ui.ctx())
+                .paint_left(ui.painter(), x, y);
+        };
+    let head = Rect::from_min_size(rect.min, vec2(rect.width(), row));
+    ui.painter().rect_filled(head, 0.0, palette.panel);
+    widgets::hline(ui, head.x_range(), head.bottom() - 0.5, palette.border);
+    let names =
+        ["Time", "Tool", "Request", "Result"].map(|name| look.label(&gettext(locale, name)));
+    for (column, name) in columns.iter().zip(&names) {
+        cell(ui, *column, head.center().y, small, name, palette.secondary);
+    }
+    widgets::announce(ui, head, &names.join(" · "));
+    let now = std::time::SystemTime::now();
+    let mut y = head.bottom();
+    if calls.is_empty() {
+        let said = look.label(&gettext(locale, "No calls on this connection yet."));
+        cell(
+            ui,
+            (columns[0].0, rect.width() - 24.0),
+            y + row / 2.0,
+            small,
+            &said,
+            palette.dim,
+        );
+        widgets::announce(
+            ui,
+            Rect::from_min_size(pos2(rect.left(), y), vec2(rect.width(), row)),
+            &said,
+        );
+        return;
+    }
+    for call in calls {
+        let line = Rect::from_min_size(pos2(rect.left(), y), vec2(rect.width(), row));
+        let when = ago(now.duration_since(call.at).unwrap_or_default(), locale);
+        let (ended, color) = ended(call, palette, locale);
+        let asked = call.request.as_deref().unwrap_or_default();
+        let center = line.center().y;
+        cell(
+            ui,
+            columns[0],
+            center,
+            small,
+            &look.label(&when),
+            palette.secondary,
+        );
+        cell(ui, columns[1], center, body, &call.tool, palette.text);
+        cell(ui, columns[2], center, mono, asked, palette.text);
+        cell(ui, columns[3], center, small, &ended, color);
+        widgets::announce(
+            ui,
+            line,
+            &format!("{when} · {} · {asked} · {ended}", call.tool),
+        );
         if !look.terminal {
             widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.border);
         }
-        top += row;
+        y += row;
     }
 }
 
+/// How a call ended, as the feed says it, and its colour: what it
+/// returned, or why it did not.
+fn ended(
+    call: &tabletist_mcp::Call,
+    palette: &Palette,
+    locale: crate::i18n::Locale,
+) -> (String, egui::Color32) {
+    let count = |one: &'static str, many: &'static str, n: u64| {
+        ngettext(locale, one, many, u32::try_from(n).unwrap_or(u32::MAX))
+            .replace("{n}", &n.to_string())
+    };
+    if call.result == "ok" {
+        let said = match (call.tool.as_str(), call.rows) {
+            ("describe_schema", Some(tables)) => {
+                let mut said = count("{n} table", "{n} tables", tables);
+                if let Some(columns) = call.columns {
+                    said = format!("{said} · {}", count("{n} column", "{n} columns", columns));
+                }
+                said
+            }
+            (_, Some(rows)) => format!(
+                "{} · {} ms",
+                count("{n} row", "{n} rows", rows),
+                call.took.as_millis()
+            ),
+            (_, None) => gettext(locale, "ok").into_owned(),
+        };
+        return (said, states::Tone::Success.color(palette));
+    }
+    let (word, color) = match call.result.as_str() {
+        "timeout" => (
+            gettext(locale, "timed out"),
+            states::Tone::Warning.color(palette),
+        ),
+        "cancelled" => (gettext(locale, "cancelled"), palette.secondary),
+        "stopped" => (gettext(locale, "stopped"), palette.secondary),
+        "error" => (
+            gettext(locale, "failed"),
+            states::Tone::Danger.color(palette),
+        ),
+        _ => (
+            gettext(locale, "refused"),
+            states::Tone::Danger.color(palette),
+        ),
+    };
+    let said = match &call.message {
+        // On one line: a database's own words may run over several.
+        Some(message) => format!(
+            "{word} · {}",
+            message.split_whitespace().collect::<Vec<_>>().join(" ")
+        ),
+        None => word.into_owned(),
+    };
+    (said, color)
+}
+
 /// How long ago, as the person reads it: "just now", "4 min", "2 h".
 pub fn ago(since: std::time::Duration, locale: crate::i18n::Locale) -> String {
     let minutes = since.as_secs() / 60;
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 85bf6bb..0285632 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -799,7 +799,14 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
                     actions.push(Action::TreeKey { tab, key });
                 }
             }
-            if on(input, Command::OpenObject, Scope::Sidebar).count > 0 {
+            // The terminal look's list of sessions has Enter of its own,
+            // unless the person went to the tree.
+            let list = terminal
+                && agents_lists(app, tab)
+                && app
+                    .workspace(tab)
+                    .is_some_and(|workspace| workspace.pane != crate::model::Pane::Tree);
+            if !list && on(input, Command::OpenObject, Scope::Sidebar).count > 0 {
                 let key = TreeKey::Enter;
                 actions.push(Action::TreeKey { tab, key });
             }
@@ -1686,6 +1693,34 @@ fn agents_shows(app: &App, tab: ConnTabId, server_off: bool) -> bool {
         }
 }
 
+/// Whether the Agents tab of `tab` is in front and lists sessions, here
+/// or elsewhere: what its keys for them need. Not while the server is off
+/// or the connection is Off, whose tabs list none.
+fn agents_lists(app: &App, tab: ConnTabId) -> bool {
+    use crate::model::AgentsHere;
+    let Some(workspace) = app.workspace(tab).filter(|w| w.agents_in_front()) else {
+        return false;
+    };
+    let listing = matches!(
+        app.agents_here(tab),
+        Some(AgentsHere::NoneConnected | AgentsHere::Connected { .. })
+    );
+    listing && !app.agents.listed(&workspace.conn_id).is_empty()
+}
+
+/// Whether the Agents tab of `tab` is in front with agents here: what Stop
+/// agents here needs.
+fn agents_here(app: &App, tab: ConnTabId) -> bool {
+    let front = app
+        .workspace(tab)
+        .is_some_and(crate::model::Workspace::agents_in_front);
+    front
+        && matches!(
+            app.agents_here(tab),
+            Some(crate::model::AgentsHere::Connected { .. })
+        )
+}
+
 /// Whether `sql` shows a result grid for the keys to move in: its last
 /// run's rows, unless the Messages pane covers them.
 fn shows_grid(sql: &crate::model::SqlTab) -> bool {
@@ -2126,6 +2161,35 @@ fn letters(
             )
         })
     };
+    // Enter shows the window of the selected session that works
+    // elsewhere, on its Agents tab. Only then, and never a focused
+    // button's.
+    let elsewhere = app
+        .agents_selected(tab)
+        .filter(|session| {
+            app.workspace(tab)
+                .is_some_and(|workspace| !session.connections.contains(&workspace.conn_id.0))
+        })
+        .and_then(|session| crate::ui::agents::Elsewhere::of(app, session).on);
+    if agents_lists(app, tab)
+        && !crate::ui::focus::on_control(ctx)
+        && let Some((conn, _)) = elsewhere
+        && ctx
+            .input_mut(|input| {
+                asked(
+                    input,
+                    &app.keymap,
+                    layout,
+                    Command::ShowAgentsElsewhere,
+                    Scope::Agents,
+                    true,
+                )
+            })
+            .count
+            > 0
+    {
+        actions.push(Action::ShowAgentsOn(conn));
+    }
     let alone = actions.is_empty() && !pointer && keys <= typed.len() && only_edits;
     // Whether a field or the prompt was asked for: the frame's typing ends
     // there.
@@ -2178,6 +2242,22 @@ fn letters(
             (Some(Command::TurnAgentsOn), _) if agents_shows(app, tab, true) => {
                 actions.push(Action::TurnAgentsOn);
             }
+            (Some(Command::MoveInAgentSessions), _) if agents_lists(app, tab) => {
+                let step = if char == 'k' { -1 } else { 1 };
+                actions.push(Action::MoveAgentSelection { tab, step });
+            }
+            (Some(Command::StopAgentSession), _) if agents_lists(app, tab) => {
+                if let Some(session) = app.agents_selected(tab) {
+                    actions.push(Action::StopAgentSession(session.id.clone()));
+                }
+            }
+            (Some(Command::StopAgentsHere), _) if agents_here(app, tab) => {
+                let here = app
+                    .workspace(tab)
+                    .map_or(0, |workspace| app.agents.here(&workspace.conn_id).len());
+                let name = app.agents_name(tab).unwrap_or_default();
+                crate::ui::agents::stop_here(ctx, tab, here, &name, app.locale, actions);
+            }
             (Some(Command::EditCell), _) if edits && alone => {
                 opened = true;
                 actions.extend(edit(crate::model::EditStart::Value));
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index dc15a4c..53d5bc5 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2029,11 +2029,20 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let review_key = key(Command::ReviewSql);
     let (agents_copy_key, agents_on_key) =
         (key(Command::CopyAgentCommand), key(Command::TurnAgentsOn));
+    let (agents_move_key, agents_stop_key, agents_here_key) = (
+        key(Command::MoveInAgentSessions),
+        key(Command::StopAgentSession),
+        key(Command::StopAgentsHere),
+    );
     // The Agents tab in front: its keys, by the state it shows.
     let agents = app
         .workspace(tab)
         .filter(|workspace| workspace.agents_in_front())
         .and_then(|_| app.agents_here(tab));
+    // Whether its list has sessions, elsewhere ones too.
+    let agents_listed = app
+        .workspace(tab)
+        .is_some_and(|workspace| !app.agents.listed(&workspace.conn_id).is_empty());
     let (run_key, run_all_key) = (key(Command::RunStatement), key(Command::RunAll));
     let (leave_key, complete_key) = (key(Command::LeaveEditor), key(Command::InsertCompletion));
     let (keep_key, next_key) = (key(Command::KeepEdit), key(Command::CommitNext));
@@ -2223,14 +2232,29 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 structure_hints.push((&review_key, &*review, true));
             }
             let (copy_word, on_word) = (gettext(locale, "copy"), gettext(locale, "turn on"));
+            let stop_words = [
+                gettext(locale, "sessions"),
+                gettext(locale, "stop session"),
+                gettext(locale, "stop agents here"),
+            ];
             let mut agents_hints: Vec<widgets::Hint<'_>> = match &agents {
                 Some(crate::model::AgentsHere::ServerOff) => {
                     vec![(&agents_on_key, &*on_word, true)]
                 }
-                Some(
-                    crate::model::AgentsHere::NoneConnected
-                    | crate::model::AgentsHere::Connected { .. },
-                ) => vec![(&agents_copy_key, &*copy_word, true)],
+                Some(crate::model::AgentsHere::NoneConnected) if agents_listed => vec![
+                    (&agents_move_key, &*stop_words[0], true),
+                    (&agents_stop_key, &*stop_words[1], true),
+                    (&agents_copy_key, &*copy_word, true),
+                ],
+                Some(crate::model::AgentsHere::NoneConnected) => {
+                    vec![(&agents_copy_key, &*copy_word, true)]
+                }
+                Some(crate::model::AgentsHere::Connected { .. }) => vec![
+                    (&agents_move_key, &*stop_words[0], true),
+                    (&agents_stop_key, &*stop_words[1], true),
+                    (&agents_here_key, &*stop_words[2], true),
+                    (&agents_copy_key, &*copy_word, true),
+                ],
                 Some(crate::model::AgentsHere::OffHere) | None => Vec::new(),
             };
             agents_hints.push((&sidebar_key, "tables", true));
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib ui::agents_tests`, then `cargo test --locked --lib keymap::`, `cargo test --locked --test keys` and `docs/bin/check`

Expected: PASS, and the website's check ends `HTML-Proofer finished successfully.`

```text
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 2478 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 2475 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Look at it.** Render the tab in both looks, by eye, in a throwaway scene in `src/shots.rs`: two sessions, one of them elsewhere, and a call of each ending. Remove it before the commit. No picture is committed.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src docs/_guide/omarchy.md docs/_reference/settings-and-files.md
git commit -m "Show who is here, who is elsewhere and what they called, with the stops"
```

---

### Task 5: The master plan says what was built

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: Correct it**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index 8c41cb2..be7b5f5 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -1674,6 +1674,39 @@ the boards, each comes with what it needs:
   which the boards do not draw. Omarchy's empty states are the look's
   empty state, from the top left, without the boards' box.
 
+**As built in the first part of Phase 1b's fourth run** (`2026-10-14-mcp-server-1b-4a-activity.md`): the
+sessions with **Stop session**, "Also:" with **Show**, **Stop agents
+here**, and the feed. Where it draws otherwise than the boards:
+
+- The feed's time says how long ago ("just now", "4 min ago"), as the
+  session's line does: the clock comes with the Log view, which needs
+  dates (the fourth run's second part).
+- The feed is the last 300 calls the app heard of since it started, on
+  every connection, each window showing those on its own. A call's
+  statement is kept on one line and cut at 200 characters; its values are
+  not kept. The log has every call whole.
+- The macOS header's "Also:" names the session elsewhere that called
+  last, with **Show**, and counts the others ("and 2 more"). Each window
+  of theirs shows them as here. Where a session works is the connection
+  it named last: a session's connections end with that one
+  (`SessionInfo::connections`).
+- Omarchy's list of sessions moves with `j` and `k`, which the board does
+  not draw: `S` stops the selected session and `enter` shows where a
+  session elsewhere works. The selection is the session's id, so a key
+  lands on the session the person saw however the list reorders. The
+  list is there with nobody here too, beside the way to connect one.
+  Enter is the list's unless the person went to the tree, or a button has
+  the keyboard; the arrows stay the tree's.
+- A call stopped by **Stop session** says, in the feed and to its agent,
+  that the agent server was stopped: the one sentence the server has for
+  a stop. Calls cut off by turning the server off are in the log, not in
+  the feed, which hears nothing once the server is stopped.
+- **Stop agents here** says what it did in the toast (on Omarchy, the
+  status line): "Stopped 2 agents. Access is now Off for Bookshop · dev."
+  The level the access had is not kept beside it yet: **Allow agents…**,
+  which offers it, comes with the fifth run, and with Off and Read alone
+  the level is always Read.
+
 | State | Title | Body | Actions |
 |---|---|---|---|
 | Allowed, no agents yet | No agents connected to Bookshop · dev | "Agents like Claude Code can read this database and propose changes you approve here. Connect one with this command, then ask it about Bookshop." Then the command with **Copy**, and a box "On this connection": access, hidden tables, masked columns, last agent | **Other agents…**, **Show log** |
@@ -2035,7 +2068,7 @@ is off until the person turns it on.
 | Part | Runs | State on 2026-10-13 |
 |---|---|---|
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
-| 1b. The server over stdio, with its interface | 7, below | The first two and a half are built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md` and `2026-10-14-mcp-server-1b-3a-agents-tab.md`). Nothing the others wait for is open |
+| 1b. The server over stdio, with its interface | 7, below | The first three and a half are built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md` and `2026-10-14-mcp-server-1b-4a-activity.md`). Nothing the others wait for is open |
 | 1b on Windows: the way in | 1 | After 1b (question 14) |
 | 1c. Hidden tables | to be cut in its plan | Waits for questions 8 and 13 |
 | 1d. HTTP with tokens | 1 | After stdio (question 9) |
@@ -2106,7 +2139,8 @@ can see what agents do and stop them.
 | **1b-2b. Turning it on from the files, and stdio** (built: `2026-10-14-mcp-server-1b-2b-stdio.md`) | The `[mcp]` keys in `settings.toml`, and the app that starts and stops the server with them and sends the backend what agents may use. The endpoint, the key and the relay, `tabletist mcp`, on Linux and macOS. The keychain on macOS (A.1). The website's reference for the files | With `enabled = true` and an access written by hand, an agent connects over stdio and reads. Nothing in the app shows it yet, and nothing in the app turns it on | F.1 over stdio, F.5 over stdio, the settings file, `tests/cli.rs` |
 | **1b-3a. The Agents tab's frame** (built: `2026-10-14-mcp-server-1b-3a-agents-tab.md`) | The third kind of tab. The sidebar entry with its states but Waiting. The ways in: the key, `ga`, `:agents`, quick open. The three empty states, with **Turn on** and **Copy**. The sessions the app learns of, and those that are here. The keymap and the help | Open the tab in any state, see which state the connection is in and who is here, turn the server on from the tab | F.7: the entry's states, the ways in, one tab for a window, the empty states, nothing opening or focusing by itself |
 | **1b-3b. Changes made outside the app** (built: `2026-10-14-mcp-server-1b-3b-changes-outside.md`) | C.2: the agent settings the app last showed, kept in the state directory, and a change from outside that makes agents more able held back until the person keeps it | A program that edits the files cannot turn agents on unseen | C.2's cases |
-| **1b-4. What agents are doing** | `App::agents`. The sessions that are here and "Also:". The feed. **Stop session** and **Stop agents here**. The Log view as a list for this connection, with All connections. The notification for an agent that connects | See each agent and each call as it happens, stop one agent or all on a connection, read the log | F.7: one connection, the stops, closing the tab, the log's default. F.4's interface |
+| **1b-4a. What agents are doing** (built: `2026-10-14-mcp-server-1b-4a-activity.md`) | `App::agents`'s feed, from `Server::calls`. The sessions that are here and "Also:". **Stop session** and **Stop agents here** | See each agent and each call as it happens, stop one agent or all on a connection | F.7: one connection, the stops, closing the tab |
+| **1b-4b. The log, and a new agent** | The Log view as a list for this connection, with All connections, and **Show log**. The notification for an agent that connects | Read the log, and hear of an agent that connects | F.7: the log's default. F.4's interface |
 | **1b-5. Letting agents in** | Settings › Agents, with the switch between panes. The Connections list's Agents column, its menu and Omarchy's box. The connection editor's Agents section, with **Allow agents…** and **Show Agents tab** pointing at them. The website: a new guide, and the reference pages | Everything Phase 1 draws, without a file edited by hand | The interface tests of Settings, Connections and the editor. The keymap. `docs/bin/check` |
 
 - **A button arrives with what it opens.** The empty states of the third
@@ -2269,9 +2303,10 @@ The ones Phase 1 waits for are marked.
 16. Settled on 2026-10-13: this file lives in `docs/superpowers/plans/`,
     which the website leaves out (`docs/_config.yml:113-115`). Under
     `docs/plans/` Jekyll would have copied it to the site as it is.
-17. A new stdio agent: a notification, as planned, or a question the first
-    time a name is seen ("Let Claude Code connect?")? The second is a real
-    gate for stdio, which has none.
+17. Settled on 2026-10-14, as planned: a notification (the fourth run's
+    second part). A new stdio agent: a notification, as planned, or a
+    question the first time a name is seen ("Let Claude Code connect?")?
+    The second is a real gate for stdio, which has none.
 18. Should the log keep parameters in full? They can hold personal data.
     The alternative keeps their types and lengths.
 19. Settled on 2026-10-14, otherwise than planned (`2026-10-14-mcp-server-1b-2a-gateway.md`): a connection
````

- [ ] **Step 2: Check it**

Run: `grep -c "$(printf '\342\200\224')" docs/superpowers/plans/2026-10-13-mcp-server.md` (an em dash, written in octal so that this plan holds none)

Expected: `0`.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what the first part of its fourth run built"
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

Expected: all clean, and every test passes. On 2026-10-14, with both servers up: 23 test binaries, 3,442 passed, none failed, 22 ignored. The website's check ends `HTML-Proofer finished successfully.`

- [ ] **By hand, on Linux or macOS**: allow a connection (`access = "read"`), turn the server on, and connect Claude Code with the tab's command. Ask it about the database: its session is listed, and each call shows in the feed as it ends. A second agent on another connection is under "Also:"; **Show** opens that connection's window on its Agents tab. **Stop session** ends the agent's session (it says the server stopped it), and **Stop agents here** ends it and sets the connection Off in `connections.toml`. On Omarchy the same with `j`, `k`, `S`, `X` and `enter`.

- [ ] **Open the pull request.**

## Self-review

- **Against the master plan's row for this run** (G, as cut on 2026-10-14): the feed from `Server::calls` (Tasks 1 and 2), the sessions here and "Also:" (Task 4), **Stop session** and **Stop agents here** (Tasks 3 and 4). What the row had and this part leaves to 4b, the Log view and the notification, is in "What this run leaves out".
- **Against D.4 and A.5.** "Also:" with Show, which opens the window first; one session stopped whole; Stop agents here holding by the access Off; the tab opened by nothing that arrives (`a_call_goes_into_the_feed_and_opens_nothing`). **Against F.7**: one connection (a session here after a call names it, "Also:" for one that named none), the stops, closing the tab (unchanged, 3a's test).
- **Against the canvas**, read again on 2026-10-14: macOS's header, rows and feed as `macOS – Agents tab, activity and approval` draws them, without the card; Omarchy's list and feed as `Omarchy – Agents tab, activity, approval and log`. What is drawn otherwise is in the table above, and in the master plan's "As built".
- **Reviewed before it was written.** A reviewer was given the draft and asked to break it, with tests of its own. It found these, each fixed above with a test: Omarchy's selection was a place in a list that reorders at each call, so `S` could stop another session than the one shown; `k` moved down once the list had shrunk; with agents here the arrows no longer reached the tree; Enter was taken from a focused button; Omarchy hid who works elsewhere while nobody was here; and "Also:" named the connection a session named first, not last. Its notes on calls cut off by turning the server off, and on the sentence a stopped session's calls get, are in the master plan's "As built".
- **Names.** `Call`, `CALL_CHARS`, `Server::calls`, `AgentsEvent::Called`, `AgentsCommand::StopSession`, `FEED_KEPT`, `called`, `calls_on`, `elsewhere`, `listed`, `AgentsTab::selected`, `StopAgentSession`, `StopAgentsHere`, `ShowAgentsOn`, `MoveAgentSelection`, `agents_selected`, `Elsewhere`, and the keymap's `MoveInAgentSessions`, `StopAgentSession`, `StopAgentsHere` and `ShowAgentsElsewhere`: each is used under the one name in every task.
- **Every commit stands.** After Task 1 the server tells of calls, and nobody listens. After Task 2 the app keeps them, and shows nothing more. After Task 3 the stops and Show work as actions, with no control yet. After Task 4 the tab shows it all, in both looks. Task 5 is the master plan's text.
- **No placeholders.** Every step has its code or its command, and what the command prints.
