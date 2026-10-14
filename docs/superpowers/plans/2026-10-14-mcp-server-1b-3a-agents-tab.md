# MCP server, Phase 1b, run 3a: the Agents tab's frame. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each connection window has an Agents tab and an Agents entry in its sidebar, which say which of four states the connection is in (the server off, agents Off here, nobody here, an agent here), with every way in but the View menu: the entry, ⌥⌘A, `ga`, `:agents` and quick open. The tab's empty states offer **Turn on** and **Copy**, and with an agent here it lists the sessions that are here.

**Architecture:** The server's sessions record the connections their calls named and when they last called, and the server says when that changes (`Server::changes`). The backend passes each change on as `AgentsEvent::Sessions`, and the app keeps the list (`App::agents`). One function (`AgentsState::of`) gives a connection's state from the settings, its access and the sessions; the entry and the tab both ask it. The Agents tab is the third kind of `Tab`, one at most for a window, opened only by what the person does. Its keys are a scope of their own (`Scope::Agents`).

**Tech Stack:** Rust 2024, egui. `tokio::sync::watch` for the server's changes. Nothing new is built.

---

## Where this sits

Phase 1b of `docs/superpowers/plans/2026-10-13-mcp-server.md` is seven runs now (section G). Three are merged: the server crate, the backend's gateway, and the files and stdio.

1. Done: the server, with nothing to start it.
2. Done: 2a, the backend's gateway; 2b, turning it on from the files, and stdio.
3. **This run, 3a: the Agents tab's frame.** The tab, its sidebar entry, the ways in, the three empty states, and who is here.
4. 3b: changes made outside the app (C.2), held back until the person keeps them.
5. The fourth run, what agents are doing, and the fifth, letting agents in. Then Windows' way in, a run of its own.

The third run was cut in two on 2026-10-14. The entry's "None connected" and the tab's "No agents connected" say that nobody is here, and the app could not know it: the server told it nothing of its sessions before the fourth run. So the sessions came forward into this run, and C.2's check went to the second part.

## Where this draws otherwise than the canvas

The boards read for it, on 2026-10-14: `macOS – Agents tab, ways in and empty states`, `macOS – Agents tab, no agents yet`, `macOS – Agents tab, activity and approval`, `Omarchy – Agents tab, ways in and empty states`, `Omarchy – Agents tab, activity, approval and log`, and the Components.

| The canvas | What this run builds | Why |
|---|---|---|
| The Activity / Log switch at the tab's top, **Show log** in two empty states, "The log still shows past calls", "Last agent" in "On this connection" | None of them | The Log view is the fourth run's. A button arrives with what it opens (G) |
| **Allow agents…**, **Other agents…**, **Settings…**, and the header's "Change in Connections" | None of them. The Off state has no button | The editor's Agents section and Settings › Agents are the fifth run's |
| "Agents like Claude Code can read this database and propose changes you approve here." | "…can read this database." | Writes are Phase 2's. So are the entry's Waiting state and the count on the tab |
| With an agent here: its line with **Stop session**, "Also:", **Stop agents here**, the feed | The sessions that are here, each on its line, and nothing else | The fourth run's |
| "connected 10:38" on a session's line | "connected 4 min ago" | The app reads no local clock |
| "Server: stdio and 127.0.0.1:7433" | "Server: stdio" | HTTP is 1d's |
| The tab's icon: a terminal in a rectangle | Lucide's `square-terminal` | The icons are Lucide's (`assets/icons/`), and this is its nearest |
| The macOS entry has no lit state on the board; the master plan wants it "highlighted while its tab is the active one" | An accent outline while the tab is in front | The button draws its own fill, over any other |
| Omarchy's empty states in a box | The look's empty state (`states::empty`), from the top left, without the box | The look draws every empty state so |
| Omarchy's strip ends with ":agents opens this tab · alt+4" | Not drawn | The strip has its `+ sql` there, and the status line names the keys |
| The View menu's Agents | Not built | There is no View menu (question 24) |

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`. Its rules this run meets: views draw and push `Action`s and `App::apply` changes state; a key is spelled in the keymap and nowhere else; nothing an agent sent goes into the app's log; and there are no em dashes.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.
- No `Cargo.toml` or `Cargo.lock` changes.
- **The servers.** Nothing in this run needs PostgreSQL or MySQL. The full run at the end prints "skipped" for their tests without the variables; with them:

  ```bash
  export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
  export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist
  ```

## What was run before this plan was written

Every task's state was built, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` (3,410 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`.

The tab and the entry were rendered headless in both looks, in each state, and compared with the boards by eye. The scene was thrown away: no picture is part of a test, and none is committed. Not run:

- **macOS and Windows.** CI runs every test there. On Windows nothing of agents shows (`endpoint::AVAILABLE`), and two tests hold that; they run there alone.
- **A real agent.** The tab learns of sessions from the backend, which the end-to-end test of `tabletist mcp` drives through the built binary. Claude Code is tried by hand after the merge.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| How the app learns who is connected | The server's `changes()`, a `watch` it sends to when a session opens or ends, names its agent, or calls. The backend subscribes before the server listens, says the sessions as it starts, then at each change while the server runs. A stop clears the list, and what is said while the server is off in the settings is not kept | A.5. A `watch` keeps the last change only: a burst of calls is one event, and the list is whole each time. Found by the draft's review: a session that opened first was missed, and one said as the server stopped could stay |
| Where a session is | Here, for a connection, from the moment its first call that names it comes, until it ends (`SessionInfo::connections`, in the order it named them). One that has named none is nowhere yet. Its last call is the latest that came, whichever ended last | A.5 and question 22. Found by the draft's review: a session was here only once its first call ended, and a long call that ended last took the time back |
| What the entry and the tab say | `AgentsState::of`: Server off when `[mcp] enabled` is false, whatever the access; else Off here when the connection's access is Off or it is saved no more; else the agent that called last of those here, else None connected. The status bar counts those here only in that last state | D.4 and question 26 |
| What the tab calls the connection | What an agent calls it (`agent_names`, the rule `agentable` has): its saved name and environment, with the start of its id where another is called the same. "Runs as" the saved environment, and nothing where it has none | Found by the draft's review: the tab built its own name from the window's, which an edit leaves as it was, and said "Fixture · none" |
| The tab | `Tab::Agents(AgentsTab)`, the third kind, unboxed (it holds its id alone). One for a window: `ShowAgents` opens it at the end of the strip, or brings it forward. Closing it stops nothing, and switching the database keeps it, as it keeps the SQL editors | D.4 |
| What opens the tab | Only what the person does: the entry, the key, the prompt's line, quick open, and a test's `Action`. No arm of `Event::Agents` touches the tabs | D.4 and F.7 |
| The keys | ⌥⌘A (Ctrl+Alt+A on Windows) from anywhere in the window; on Omarchy `ga` from the tree and the grid, and `:agents` from any tab. `y` copies and `o` turns on, in `Scope::Agents`, the tab's own, where its state offers them | D.6. A scope of its own, so that `y` and `o` are read nowhere else |
| What Copy copies | `claude mcp add tabletist -- tabletist mcp`, with this program's whole path, quoted where a shell needs it, where `tabletist` on the `PATH` is not this program (the app bundle on macOS). "Copied the command", as the toast says Copy SQL | D.1 |
| Turn on | `Action::TurnAgentsOn`: the setting, through `change_settings`, which starts the server. No connection's access changes | Question 23 |
| Quick open | An "Agents" entry first, with the connection and the key at its right, once what is typed is the start of "agents", two letters at least. Fewer letters, or letters only somewhere in the word, are the objects', as the list was | D.4 and section 2's row 13. Found by the draft's review: any letters of the word in order put the entry above `active_users` on "a", and Enter opened the tab |
| Omarchy's status line | With the tab in front, its keys: `o turn on` or `y copy`, then `ctrl+b tables`, and at its end what was just done ("copied the command") | The grid's keys do nothing there. This look floats no toast: found by the draft's review, a copy said nothing on screen |
| Long names | An agent's name, a connection's and a session's line give way at their end to the room they have, in the entry, the header and the tab | Found by the draft's review: a 40-letter agent ran over the entry's label |
| Windows | No entry, no tab, no key answers, no quick open entry. The commands are in the keymap, claimed, as unbuilt ones are | Question 14 |

### What this run leaves out

- **C.2's check** of changes made outside the app: the second part of this run.
- What the table above names, each with the run it comes with.
- **The notification** for an agent that connects (the fourth run), and the ways in from Settings and Connections (the fifth).

## File structure

| File | What changes |
|---|---|
| `crates/tabletist-mcp/src/server.rs` | `SessionInfo::connections` and `last_call_at`. `Server::changes` |
| `src/backend/agents.rs` | `AgentsEvent::Sessions`, sent on every change while the server runs |
| `src/model.rs` | `AgentsState`, `AgentsHere`, `agent_name`. `Tab::Agents`, `AgentsTab`, `Workspace::agents_tab` and `agents_in_front`. `Action::ShowAgents` and `TurnAgentsOn` |
| `src/app.rs` | `App::agents` and `agents_here`. `show_agents`, and the arms a third kind of tab needs. Quick open's `Found` |
| `src/endpoint.rs` | `agent_command`: what Copy puts on the clipboard |
| `src/keymap.rs` | `ShowAgents`, `CopyAgentCommand`, `TurnAgentsOn`, and `Scope::Agents` |
| `src/ui/agents.rs` | New. The tab: its header, its four states, its status bar |
| `src/ui/sidebar.rs` | The Agents entry, in both looks |
| `src/ui/object_tabs.rs`, `src/ui/workspace.rs`, `src/ui/row_panel.rs` | The third kind of tab in the strip, in front, and beside the row panel. Omarchy's status line for the tab |
| `src/ui/keys.rs`, `src/app/editing.rs`, `src/ui/quick_open.rs` | The keys and the prompt's line that show the tab, the tab's letters, quick open's entry |
| `src/theme.rs`, `assets/icons/square-terminal.svg` | The tab's icon |
| `src/ui/agents_tests.rs` | New. The tab and the entry, through the headless harness, in both looks |
| `tests/cli.rs` | The app is told where a session works, end to end |
| `docs/_reference/keyboard-shortcuts.md` | ⌥⌘A |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | What this run built, and where it went otherwise than planned |

---

### Task 1: A session knows where it works, and the server says when that changes

`SessionInfo` gains `connections`, the ids its calls named in the order it first named each, and `last_call_at`. Each call notes both as it comes, the connection found by the name it gives (`Live::called`), and the time never goes back. `Server::changes` is a `watch` the server sends to when a session opens or ends, names its agent, or calls. The new test runs a slow statement beside a quick call, and takes about four seconds.

**Files:**
- Modify: `crates/tabletist-mcp/src/server.rs`, `crates/tabletist-mcp/tests/sessions.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/crates/tabletist-mcp/tests/sessions.rs b/crates/tabletist-mcp/tests/sessions.rs
index 00af2d6..e53d7a8 100644
--- a/crates/tabletist-mcp/tests/sessions.rs
+++ b/crates/tabletist-mcp/tests/sessions.rs
@@ -69,6 +69,68 @@ async fn a_session_is_known_while_it_is_open() {
     assert_eq!(server.open_sessions(), []);
 }
 
+#[tokio::test]
+async fn a_session_knows_where_it_works_and_the_server_says_when_that_changes() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let mut changes = server.changes();
+    changes.mark_unchanged();
+    let agent = Agent::connect(&server).await;
+    assert!(changes.has_changed().unwrap(), "a session opened");
+    changes.mark_unchanged();
+    // A call that names no connection is a call all the same.
+    agent.call("list_connections", json!({})).await;
+    assert!(changes.has_changed().unwrap(), "a call");
+    changes.mark_unchanged();
+    let open = &server.open_sessions()[0];
+    assert_eq!(open.connections, Vec::<String>::new());
+    let first = open.last_call_at.unwrap();
+    agent
+        .call(
+            "run_query",
+            json!({ "connection": "Bookshop · dev", "sql": "SELECT 1" }),
+        )
+        .await;
+    assert!(changes.has_changed().unwrap(), "a call on a connection");
+    let open = &server.open_sessions()[0];
+    assert_eq!(open.connections, ["id:Bookshop · dev"]);
+    assert!(open.last_call_at.unwrap() >= first);
+    // Named once, however often.
+    agent
+        .call("describe_schema", json!({ "connection": "Bookshop · dev" }))
+        .await;
+    assert_eq!(server.open_sessions()[0].connections, ["id:Bookshop · dev"]);
+    changes.mark_unchanged();
+    agent.close().await;
+    assert!(changes.has_changed().unwrap(), "a session ended");
+    assert_eq!(server.open_sessions(), []);
+}
+
+#[tokio::test]
+async fn a_session_is_where_its_call_names_from_when_the_call_comes() {
+    use std::time::{Duration, Instant};
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let arguments = json!({ "connection": "Bookshop · dev", "sql": support::slow(5) });
+    let slow = agent.call("run_query", arguments);
+    let meanwhile = async {
+        // Here while its first call on the connection still runs.
+        let deadline = Instant::now() + Duration::from_millis(500);
+        while server.open_sessions()[0].connections.is_empty() {
+            assert!(Instant::now() < deadline, "not here while the call runs");
+            tokio::time::sleep(Duration::from_millis(10)).await;
+        }
+        assert_eq!(server.open_sessions()[0].connections, ["id:Bookshop · dev"]);
+        agent.call("list_connections", json!({})).await;
+        server.open_sessions()[0].last_call_at.unwrap()
+    };
+    let (_, quick) = tokio::join!(slow, meanwhile);
+    // The slow call came first and ended last: it takes no time back.
+    assert!(server.open_sessions()[0].last_call_at.unwrap() >= quick);
+    agent.close().await;
+}
+
 #[tokio::test]
 async fn a_name_a_client_gives_itself_is_a_label_a_person_can_read() {
     let shop = Shop::new();
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked -p tabletist-mcp --test sessions`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0599]: no method named `changes` found for struct `Server<G>` in the current scope
error[E0609]: no field `connections` on type `&SessionInfo`
error[E0609]: no field `last_call_at` on type `&SessionInfo`
error[E0609]: no field `connections` on type `SessionInfo`
error[E0609]: no field `last_call_at` on type `SessionInfo`
error: could not compile `tabletist-mcp` (test "sessions") due to 10 previous errors
```

- [ ] **Step 3: Where a session works, and the changes**

````diff
diff --git a/crates/tabletist-mcp/src/server.rs b/crates/tabletist-mcp/src/server.rs
index cf9df6c..e9158fc 100644
--- a/crates/tabletist-mcp/src/server.rs
+++ b/crates/tabletist-mcp/src/server.rs
@@ -7,7 +7,7 @@ use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
 
 use serde_json::{Map, Value as Json};
 use tokio::io::{AsyncRead, AsyncWrite};
-use tokio::sync::RwLock;
+use tokio::sync::{RwLock, watch};
 use tokio_util::sync::CancellationToken;
 
 use crate::gateway::Gateway;
@@ -51,6 +51,11 @@ pub struct SessionInfo {
     pub transport: Transport,
     pub connected_at: SystemTime,
     pub calls: u64,
+    /// The connections its calls have named, by id, in the order it first
+    /// named each: where it works.
+    pub connections: Vec<String>,
+    /// When its last call came, once it has made one.
+    pub last_call_at: Option<SystemTime>,
 }
 
 /// What stands for an agent's name until it has given one.
@@ -82,6 +87,17 @@ pub(crate) struct Live {
     in_flight: RwLock<()>,
     /// How many calls are running.
     running: AtomicUsize,
+    /// The connections its calls named, and when the last one came.
+    used: Mutex<Used>,
+    /// The server's, told of each change to the session.
+    changed: Arc<watch::Sender<()>>,
+}
+
+/// Where a session has worked, and when it last did.
+#[derive(Debug, Default)]
+struct Used {
+    connections: Vec<String>,
+    last_call_at: Option<SystemTime>,
 }
 
 impl Live {
@@ -92,8 +108,8 @@ impl Live {
             .map(label)
             .filter(|label| !label.is_empty())
             .unwrap_or_else(|| label(name));
-        if !label.is_empty() {
-            let _ = self.agent.set(label);
+        if !label.is_empty() && self.agent.set(label).is_ok() {
+            self.changed.send_replace(());
         }
     }
 
@@ -104,6 +120,7 @@ impl Live {
     }
 
     fn info(&self) -> SessionInfo {
+        let used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
         SessionInfo {
             id: self.id.clone(),
             agent: self.agent.get().cloned(),
@@ -111,8 +128,24 @@ impl Live {
             transport: self.origin.transport,
             connected_at: self.connected_at,
             calls: self.calls.load(Ordering::Relaxed),
+            connections: used.connections.clone(),
+            last_call_at: used.last_call_at,
         }
     }
+
+    /// Notes a call that came at `at`, and the connection it named. A call
+    /// that ends after a later one does not take the time back.
+    fn called(&self, at: SystemTime, connection: Option<&str>) {
+        let mut used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
+        used.last_call_at = used.last_call_at.max(Some(at));
+        if let Some(id) = connection
+            && !used.connections.iter().any(|named| named == id)
+        {
+            used.connections.push(id.to_owned());
+        }
+        drop(used);
+        self.changed.send_replace(());
+    }
 }
 
 /// A name a client gives itself, as a label a person reads: nothing that
@@ -159,6 +192,7 @@ impl<G: Gateway> Drop for Open<G> {
         self.server
             .sessions()
             .retain(|open| !Arc::ptr_eq(open, &self.live));
+        self.server.inner.changed.send_replace(());
     }
 }
 
@@ -202,6 +236,9 @@ struct Inner<G> {
     sessions: Mutex<Vec<Arc<Live>>>,
     /// Ended by [`Server::stop`]. Every session's stop is a child of it.
     stop: Stop,
+    /// Sent to when a session opens or ends, names its agent, or makes a
+    /// call: what [`Server::open_sessions`] says has changed.
+    changed: Arc<watch::Sender<()>>,
 }
 
 impl<G: Gateway> Server<G> {
@@ -215,6 +252,7 @@ impl<G: Gateway> Server<G> {
                 version: version.into(),
                 sessions: Mutex::default(),
                 stop: Stop::new(),
+                changed: Arc::new(watch::Sender::new(())),
             }),
         }
     }
@@ -235,8 +273,11 @@ impl<G: Gateway> Server<G> {
             stop: self.inner.stop.child(),
             in_flight: RwLock::new(()),
             running: AtomicUsize::new(0),
+            used: Mutex::default(),
+            changed: Arc::clone(&self.inner.changed),
         });
         self.sessions().push(live.clone());
+        self.inner.changed.send_replace(());
         let open = Open {
             server: self.clone(),
             live: live.clone(),
@@ -248,6 +289,12 @@ impl<G: Gateway> Server<G> {
         drop(open);
     }
 
+    /// Told when what [`Server::open_sessions`] says has changed: a
+    /// session opened or ended, its agent was named, or it made a call.
+    pub fn changes(&self) -> watch::Receiver<()> {
+        self.inner.changed.subscribe()
+    }
+
     /// The sessions that are open, the oldest first.
     pub fn open_sessions(&self) -> Vec<SessionInfo> {
         self.sessions().iter().map(|live| live.info()).collect()
@@ -296,6 +343,20 @@ impl<G: Gateway> Server<G> {
         let started = Instant::now();
         let _in_flight = live.in_flight.read().await;
         let (_running, before) = Running::count(&live.running);
+        // The session works where the call names, from the moment it comes:
+        // a call that runs for seconds is there the while.
+        let named = arguments
+            .get("connection")
+            .and_then(Json::as_str)
+            .and_then(|name| {
+                self.inner
+                    .gateway
+                    .connections()
+                    .into_iter()
+                    .find(|listed| listed.name == name)
+            })
+            .map(|listed| listed.id);
+        live.called(arrived, named.as_deref());
         let stop = live.stop.child();
         let refuse = |refused: Refused| {
             known(name).map(|()| Done {
@@ -321,6 +382,8 @@ impl<G: Gateway> Server<G> {
         }?;
         live.calls.fetch_add(1, Ordering::Relaxed);
         let Done { answer, note } = done;
+        let named = note.connection.as_ref().map(|(id, _)| id.as_str());
+        live.called(arrived, named);
         let reply = match answer {
             // A map of JSON values is always written.
             Ok(json) => Reply::Answer(serde_json::to_string(&json).unwrap_or_default()),
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-mcp --test sessions`

Expected: PASS, in about four seconds: the new test waits on a slow statement.

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-mcp
git commit -m "Know where an agent's session works, and say when the sessions change"
```

---

### Task 2: The app knows who is here

- `src/backend/agents.rs`: as the server starts and while it runs, a task sends `AgentsEvent::Sessions` at each change, subscribed before the server listens. A stop aborts it with the server's chores. `agent_names` is the rule that names the connections, Off or not, which `agentable` now uses and the tab will.
- `src/model.rs`: `AgentsState`, the sessions, with `here` (those that named a connection, the one that called last first) and `of` (the state a connection's entry and tab show). `AgentsHere`, the four states. `agent_name`.
- `src/app.rs`: `App::agents`, set from `AgentsEvent::Sessions` while the server is on in the settings, cleared by a stop. `agents_here`, a workspace's state.
- `tests/cli.rs`, and the backend's own tests: through the built binary, the backend says where the session works, and a server says its sessions as it starts. The waits for what the server did pass over the sessions, which come beside it now.

Nothing shows it yet: Task 3 does.

**Files:**
- Modify: `src/backend.rs`, `src/backend/agents.rs`, `src/model.rs`, `src/app.rs`, `tests/cli.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/backend.rs b/src/backend.rs
index f002b7e..783bea5 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -20,7 +20,7 @@ use crate::secrets::{Keyring, SecretString};
 
 mod agents;
 
-pub use agents::{Agentable, AgentsCommand, AgentsEvent, AppGateway, agentable};
+pub use agents::{Agentable, AgentsCommand, AgentsEvent, AppGateway, agent_names, agentable};
 
 /// Wakes the UI from any thread. The default does nothing (tests).
 #[derive(Clone)]
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index b6b6258..30a998f 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -1425,7 +1425,7 @@ mod tests {
             endpoint: None,
         }));
         assert_eq!(
-            match backend.wait(WAIT) {
+            match told(&mut backend, WAIT) {
                 Some(Event::Agents(event)) => event,
                 other => panic!("{other:?}"),
             },
@@ -1451,7 +1451,7 @@ mod tests {
         assert_eq!(left, 1, "the call of long ago is forgotten");
         backend.send(Command::Agents(AgentsCommand::Stop));
         assert!(matches!(
-            backend.wait(WAIT),
+            told(&mut backend, WAIT),
             Some(Event::Agents(AgentsEvent::Stopped))
         ));
         // A log that cannot be made: the server does not start, and says
@@ -1461,7 +1461,7 @@ mod tests {
             log_days: 30,
             endpoint: None,
         }));
-        match backend.wait(WAIT) {
+        match told(&mut backend, WAIT) {
             Some(Event::Agents(AgentsEvent::Failed(said))) => {
                 assert!(
                     said.starts_with("the log of agents' calls could not be opened"),
@@ -1490,7 +1490,7 @@ mod tests {
             log_days: 30,
             endpoint: Some(at),
         }));
-        match backend.wait(WAIT) {
+        match told(&mut backend, WAIT) {
             // Said as it is: the app puts its own words before it.
             Some(Event::Agents(AgentsEvent::Failed(said))) => {
                 assert_eq!(said, crate::endpoint::ANOTHER_SERVES);
@@ -1500,6 +1500,27 @@ mod tests {
         assert!(!path.exists());
     }
 
+    #[test]
+    fn a_server_says_its_sessions_as_it_starts() {
+        let dir = tempfile::tempdir().unwrap();
+        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
+        backend.send(Command::Agents(AgentsCommand::Start {
+            log: dir.path().join("agents.sqlite3"),
+            log_days: 30,
+            endpoint: None,
+        }));
+        // None yet, said without a change to wait for: a session that
+        // opened before the app heard of the start is not missed.
+        let mut sessions = None;
+        while let Some(Event::Agents(event)) = backend.wait(WAIT) {
+            if let AgentsEvent::Sessions(open) = event {
+                sessions = Some(open);
+                break;
+            }
+        }
+        assert_eq!(sessions, Some(Vec::new()));
+    }
+
     #[test]
     fn starts_and_stops_one_after_another_leave_the_last_one_standing() {
         let dir = tempfile::tempdir().unwrap();
@@ -1517,7 +1538,7 @@ mod tests {
         backend.send(Command::Agents(AgentsCommand::Stop));
         backend.send(start());
         let mut said = Vec::new();
-        while let Some(Event::Agents(event)) = backend.wait(Duration::from_secs(2)) {
+        while let Some(Event::Agents(event)) = told(&mut backend, Duration::from_secs(2)) {
             said.push(event);
         }
         assert_eq!(
@@ -1531,6 +1552,17 @@ mod tests {
         assert!(!dir.path().join("agents.sqlite3.bad").exists());
     }
 
+    /// The backend's next event but the sessions, which it says beside
+    /// what the server does.
+    fn told(backend: &mut Backend, wait: Duration) -> Option<Event> {
+        loop {
+            match backend.wait(wait)? {
+                Event::Agents(AgentsEvent::Sessions(_)) => {}
+                event => return Some(event),
+            }
+        }
+    }
+
     /// A server's connection from a test URL, its password in the keyring.
     async fn on_a_server(url_variable: &str) -> Option<(AppGateway, Dialect)> {
         let Ok(url) = std::env::var(url_variable) else {
diff --git a/src/model.rs b/src/model.rs
index f7286c7..e5b75d8 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -3511,6 +3511,49 @@ impl std::fmt::Debug for PasswordPrompt {
 mod tests {
     use super::*;
 
+    #[test]
+    fn a_connections_agents_state_is_the_server_then_its_access_then_who_called_last() {
+        use crate::connections::AgentAccess::{Off, Read};
+        let conn = ConnectionId("shop".into());
+        let session = |agent: &str, connections: &[&str], at: u64| tabletist_mcp::SessionInfo {
+            id: agent.into(),
+            agent: Some(agent.into()),
+            project: None,
+            transport: tabletist_mcp::Transport::Stdio,
+            connected_at: std::time::SystemTime::UNIX_EPOCH,
+            calls: 1,
+            connections: connections.iter().map(|id| (*id).to_owned()).collect(),
+            last_call_at: Some(
+                std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(at),
+            ),
+        };
+        let mut state = AgentsState::default();
+        // The server off says so whatever the connection allows, Off too.
+        assert_eq!(state.of(false, Some(Read), &conn), AgentsHere::ServerOff);
+        assert_eq!(state.of(false, Some(Off), &conn), AgentsHere::ServerOff);
+        assert_eq!(state.of(true, Some(Off), &conn), AgentsHere::OffHere);
+        // A connection that is saved no more is Off.
+        assert_eq!(state.of(true, None, &conn), AgentsHere::OffHere);
+        assert_eq!(state.of(true, Some(Read), &conn), AgentsHere::NoneConnected);
+        // Elsewhere, or nowhere yet, is not here.
+        state.sessions = vec![session("Cursor", &["other"], 9), session("Zed", &[], 9)];
+        assert_eq!(state.of(true, Some(Read), &conn), AgentsHere::NoneConnected);
+        state
+            .sessions
+            .push(session("Claude Code", &["other", "shop"], 5));
+        state.sessions.push(session("Cline", &["shop"], 7));
+        assert_eq!(
+            state.of(true, Some(Read), &conn),
+            AgentsHere::Connected {
+                agent: "Cline".into(),
+                count: 2
+            }
+        );
+        assert_eq!(state.here(&conn).len(), 2);
+        // Its access Off, nobody is here for it.
+        assert_eq!(state.of(true, Some(Off), &conn), AgentsHere::OffHere);
+    }
+
     /// The Bookshop's covers, `rows` of them, as a tab holds them.
     fn covers(rows: usize) -> ObjectTab {
         let object = ObjectRef::new("main", "book_covers");
diff --git a/tests/cli.rs b/tests/cli.rs
index 2270060..dde0e08 100644
--- a/tests/cli.rs
+++ b/tests/cli.rs
@@ -147,8 +147,12 @@ mod mcp {
         let deadline = Instant::now() + WAIT;
         loop {
             for event in backend.poll() {
-                if let Event::Agents(event) = event {
-                    return event;
+                // The sessions as they change are said beside what the
+                // server does.
+                match event {
+                    Event::Agents(AgentsEvent::Sessions(_)) => {}
+                    Event::Agents(event) => return event,
+                    _ => {}
                 }
             }
             assert!(
@@ -279,6 +283,20 @@ mod mcp {
         let stderr = stderr_of(&mut child);
         assert!(!seen.contains(MARKER), "{seen}");
         assert!(!stderr.contains(MARKER), "{stderr}");
+        // The app was told of the session, and where it works.
+        let deadline = Instant::now() + WAIT;
+        let told = loop {
+            let told = backend.poll().into_iter().any(|event| {
+                matches!(&event, Event::Agents(AgentsEvent::Sessions(open))
+                    if open.iter().any(|session| session.connections == ["shop"]
+                        && session.project.as_deref() == Some("bookshop-repo")))
+            });
+            if told || Instant::now() > deadline {
+                break told;
+            }
+            std::thread::sleep(Duration::from_millis(20));
+        };
+        assert!(told, "the backend says where the session works");
         // The log has the calls, under the project's name.
         let runtime = tokio::runtime::Runtime::new().unwrap();
         let entries = runtime.block_on(async {
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib model::tests::a_connections_agents`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0432]: unresolved import `agents::agent_names`
error[E0599]: no variant, associated function, or constant named `Sessions` found for enum `agents::AgentsEvent` in the current scope
error[E0433]: cannot find type `AgentsState` in this scope
error[E0433]: cannot find type `AgentsHere` in this scope
error: could not compile `tabletist` (lib test) due to 12 previous errors
```

- [ ] **Step 3: The sessions, from the backend to the app**

````diff
diff --git a/src/app.rs b/src/app.rs
index 8728e97..ef888e9 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -128,6 +128,8 @@ pub struct App {
     /// The agent server is the app's to start and stop
     /// ([`App::serve_agents`]): not in tests, nor in the demo.
     serves_agents: bool,
+    /// What the agent server said of its sessions last.
+    pub agents: crate::model::AgentsState,
     next_id: u64,
 }
 
@@ -189,6 +191,7 @@ impl App {
             lookups: Vec::new(),
             closing: false,
             serves_agents: false,
+            agents: crate::model::AgentsState::default(),
             next_id: 1,
         };
         let tab = app.picker_tab();
@@ -2805,7 +2808,15 @@ impl App {
             // What the person turned on in the file and does not get is
             // said. The backend logs the rest.
             Event::Agents(event) => match event {
-                AgentsEvent::Started { kept_aside: None } | AgentsEvent::Stopped => {}
+                // A server that stops has no session open. One that starts
+                // says its sessions itself, the first time as it starts.
+                AgentsEvent::Stopped => self.agents.sessions.clear(),
+                AgentsEvent::Started { kept_aside: None } => {}
+                // What a server said as it stopped is not the server now.
+                AgentsEvent::Sessions(open) if self.settings.mcp_enabled => {
+                    self.agents.sessions = open;
+                }
+                AgentsEvent::Sessions(_) => {}
                 AgentsEvent::Started {
                     kept_aside: Some(path),
                 } => {
@@ -3473,6 +3484,14 @@ impl App {
 
     /// Opens an empty SQL editor after the workspace's other tabs, with the
     /// limit and timeout of the settings, and shows it.
+    /// What the workspace's Agents entry and tab say of its connection.
+    pub fn agents_here(&self, tab: ConnTabId) -> Option<crate::model::AgentsHere> {
+        let workspace = self.workspace(tab)?;
+        let conn = &workspace.conn_id;
+        let access = self.connections.get(conn).map(|saved| saved.agents.access);
+        Some(self.agents.of(self.settings.mcp_enabled, access, conn))
+    }
+
     fn new_sql_tab(&mut self, tab: ConnTabId) {
         let id = TabId(self.next_id());
         let request = RequestId(self.next_id());
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 30a998f..947fceb 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -56,6 +56,30 @@ pub struct Agentable {
 /// the names of the others change with it, the bare name among them, and
 /// an agent learns the new ones from `list_connections`.
 pub fn agentable(saved: &[SavedConnection]) -> Vec<Agentable> {
+    saved
+        .iter()
+        .zip(agent_names(saved))
+        .filter(|(saved, _)| match saved.agents.access {
+            AgentAccess::Off => false,
+            AgentAccess::Read => true,
+        })
+        .map(|(saved, name)| Agentable {
+            id: saved.id.clone(),
+            name,
+            spec: saved.spec.clone(),
+            password: saved.password,
+            ssh_secret: saved.ssh_secret,
+            limits: Limits {
+                rows: saved.agents.rows,
+                timeout: Duration::from_secs(u64::from(saved.agents.timeout_secs)),
+            },
+        })
+        .collect()
+}
+
+/// What an agent calls each of `saved`, in its order, Off or not, by the
+/// rule [`agentable`] says.
+pub fn agent_names(saved: &[SavedConnection]) -> Vec<String> {
     let called = |saved: &SavedConnection| match saved.environment.named(Platform::Native) {
         Some(environment) => format!("{} · {environment}", saved.name),
         None => saved.name.clone(),
@@ -88,25 +112,7 @@ pub fn agentable(saved: &[SavedConnection]) -> Vec<Agentable> {
             }
         })
         .collect();
-    saved
-        .iter()
-        .zip(names)
-        .filter(|(saved, _)| match saved.agents.access {
-            AgentAccess::Off => false,
-            AgentAccess::Read => true,
-        })
-        .map(|(saved, name)| Agentable {
-            id: saved.id.clone(),
-            name,
-            spec: saved.spec.clone(),
-            password: saved.password,
-            ssh_secret: saved.ssh_secret,
-            limits: Limits {
-                rows: saved.agents.rows,
-                timeout: Duration::from_secs(u64::from(saved.agents.timeout_secs)),
-            },
-        })
-        .collect()
+    names
 }
 
 /// The first `length` letters and digits of an id.
@@ -154,6 +160,10 @@ pub enum AgentsEvent {
     /// listen. Why, for the person.
     Failed(String),
     Stopped,
+    /// The sessions that are open, the oldest first: said whenever one
+    /// opens or ends, names its agent, or makes a call. None is said after
+    /// [`AgentsEvent::Stopped`].
+    Sessions(Vec<tabletist_mcp::SessionInfo>),
 }
 
 /// How long a connection an agent used is kept open with nothing to do.
@@ -174,6 +184,8 @@ const CONNECT_TIME: Duration = Duration::from_secs(20);
 pub(super) struct Running {
     server: tabletist_mcp::Server<AppGateway>,
     chores: tokio::task::JoinHandle<()>,
+    /// Tells the app of the sessions as they change.
+    watching: tokio::task::JoinHandle<()>,
     #[cfg(unix)]
     listening: Option<listen::Listening>,
 }
@@ -285,6 +297,8 @@ impl Agents {
                     if let Some(before) = running.take() {
                         before.stop();
                     }
+                    // Before it listens: no session opens unseen.
+                    let changes = server.changes();
                     // Once the server before it has let its socket go.
                     let listening = match listening(endpoint.as_deref(), &server) {
                         Ok(listening) => listening,
@@ -298,9 +312,11 @@ impl Agents {
                     #[cfg(not(unix))]
                     let () = listening;
                     let chores = tokio::spawn(chores(opened.log, gateway, log_days));
+                    let watching = tokio::spawn(sessions(server.clone(), changes, outbox.clone()));
                     *running = Some(Running {
                         server,
                         chores,
+                        watching,
                         #[cfg(unix)]
                         listening,
                     });
@@ -332,6 +348,7 @@ impl Running {
     fn stop(self) {
         self.server.stop();
         self.chores.abort();
+        self.watching.abort();
         #[cfg(unix)]
         drop(self.listening);
     }
@@ -372,6 +389,23 @@ fn listening(
     }
 }
 
+/// Tells the app of the server's sessions as it starts, and each time they
+/// change, until the server stops.
+async fn sessions(
+    server: tabletist_mcp::Server<AppGateway>,
+    mut changes: tokio::sync::watch::Receiver<()>,
+    outbox: Outbox,
+) {
+    loop {
+        changes.mark_unchanged();
+        let open = server.open_sessions();
+        outbox.emit(super::Event::Agents(AgentsEvent::Sessions(open)));
+        if changes.changed().await.is_err() {
+            return;
+        }
+    }
+}
+
 /// What the server does from time to time while it runs: closes the
 /// connections no agent has used for a while, and forgets calls older than
 /// the log keeps. The first time as it starts.
diff --git a/src/model.rs b/src/model.rs
index e5b75d8..69aac75 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2526,6 +2526,75 @@ impl Tab {
     }
 }
 
+/// What the app knows of the agent server: the sessions open on it, as
+/// the backend last said them. Empty while the server is off.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct AgentsState {
+    pub sessions: Vec<tabletist_mcp::SessionInfo>,
+}
+
+/// What the sidebar's Agents entry says of a connection, and which state
+/// its tab shows. A write that waits for the person comes with writes.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum AgentsHere {
+    /// The server is off in the settings, whatever the connection allows.
+    ServerOff,
+    /// This connection's access is Off: agents do not see it.
+    OffHere,
+    /// Agents may use it, and none has named it.
+    NoneConnected,
+    /// An agent is working here: the one that called last, of `count`.
+    Connected { agent: String, count: usize },
+}
+
+impl AgentsState {
+    /// The sessions that are here: those whose calls have named `conn`,
+    /// the one that called last first.
+    pub fn here(&self, conn: &ConnectionId) -> Vec<&tabletist_mcp::SessionInfo> {
+        let mut here: Vec<_> = self
+            .sessions
+            .iter()
+            .filter(|session| session.connections.contains(&conn.0))
+            .collect();
+        here.sort_by_key(|session| std::cmp::Reverse(session.last_call_at));
+        here
+    }
+
+    /// The state of `conn` for its Agents entry and tab: the server's
+    /// switch first, then the connection's access, then who is here.
+    /// `access` is `None` for a connection that is saved no more.
+    pub fn of(
+        &self,
+        enabled: bool,
+        access: Option<crate::connections::AgentAccess>,
+        conn: &ConnectionId,
+    ) -> AgentsHere {
+        use crate::connections::AgentAccess;
+        if !enabled {
+            return AgentsHere::ServerOff;
+        }
+        match access {
+            None | Some(AgentAccess::Off) => AgentsHere::OffHere,
+            Some(AgentAccess::Read) => {
+                let here = self.here(conn);
+                match here.first() {
+                    None => AgentsHere::NoneConnected,
+                    Some(last) => AgentsHere::Connected {
+                        agent: agent_name(last),
+                        count: here.len(),
+                    },
+                }
+            }
+        }
+    }
+}
+
+/// What a session's agent is called on screen: what its client says it
+/// is, or "Agent" until it has said.
+pub fn agent_name(session: &tabletist_mcp::SessionInfo) -> String {
+    session.agent.clone().unwrap_or_else(|| "Agent".to_owned())
+}
+
 /// Which pane of a SQL tab's results shows.
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
 pub enum ResultPane {
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib model::tests::a_connections_agents`, then `cargo test --locked --test cli`

Expected: PASS. The binary's four take about two seconds: the relay tries a socket that is not there for two.

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2457 filtered out
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src tests
git commit -m "Tell the app of the agents' sessions and where each works"
```

---

### Task 3: The Agents tab and its entry in the sidebar

The largest task: a third kind of tab is a little in many places.

- `src/model.rs`: `Tab::Agents(AgentsTab)`, with its arms; `Workspace::agents_tab` and `agents_in_front`; `Action::ShowAgents` and `TurnAgentsOn`.
- `src/app.rs`: `show_agents` opens the tab or brings it forward (nothing where agents cannot reach the app); `TurnAgentsOn` changes the setting. A saved value has no Agents tab to come from, and a switch of database keeps the tab.
- `src/keymap.rs`: the three commands and `Scope::Agents`, with their keys. Task 4 makes the keys answer; the tab's buttons name them now.
- `src/endpoint.rs`: `agent_command`, what Copy copies.
- `src/ui/agents.rs`, new: the header, the four states, the status bar (macOS), and `copy_command`.
- `src/ui/sidebar.rs`: the entry above SQL Editor (macOS and Windows), or the line above the key hints (Omarchy). The sidebar's footer grows by it where agents can reach the app.
- `src/ui/object_tabs.rs`: the tab in the strip, with its icon (`src/theme.rs`, `assets/icons/square-terminal.svg`). `src/ui/workspace.rs` draws it in front; `src/ui/row_panel.rs` has no row of it.
- `src/ui/agents_tests.rs`, new, for Unix: the entry's states in both looks, the one called last of several, the entry that shows the tab and nothing that opens it, the states and their buttons, and closing the tab, which stops nothing. On Windows, a test of `src/ui/agents.rs` holds that nothing shows.

**Files:**
- Create: `src/ui/agents.rs`, `src/ui/agents_tests.rs`, `assets/icons/square-terminal.svg`
- Modify: `src/model.rs`, `src/app.rs`, `src/keymap.rs`, `src/endpoint.rs`, `src/theme.rs`, `src/ui/mod.rs`, `src/ui/sidebar.rs`, `src/ui/object_tabs.rs`, `src/ui/workspace.rs`, `src/ui/row_panel.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/assets/icons/square-terminal.svg b/assets/icons/square-terminal.svg
new file mode 100644
index 0000000..186c71a
--- /dev/null
+++ b/assets/icons/square-terminal.svg
@@ -0,0 +1,15 @@
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
+  <path d="m7 11 2-2-2-2" />
+  <path d="M11 13h4" />
+  <rect width="18" height="18" x="3" y="3" rx="2" ry="2" />
+</svg>
diff --git a/src/endpoint.rs b/src/endpoint.rs
index 3f1d8f3..f9380e2 100644
--- a/src/endpoint.rs
+++ b/src/endpoint.rs
@@ -171,6 +171,10 @@ mod tests {
             project(&format!("/x/{}", "a".repeat(100))).unwrap().len(),
             64
         );
+        assert_eq!(
+            project("/home/ada/repo\u{202e}gnp.exe\u{200b}").as_deref(),
+            Some("repognp.exe")
+        );
     }
 
     #[test]
@@ -198,6 +202,33 @@ mod tests {
         assert_eq!(newer.cwd, None);
     }
 
+    #[test]
+    fn the_command_starts_this_program_and_quotes_its_path_where_it_must() {
+        use std::path::Path;
+        let exe = Path::new("/usr/bin/tabletist");
+        assert_eq!(relay_for(Some(exe), Some(exe)), "tabletist mcp");
+        // Another tabletist on the PATH, or none: this one, by its path.
+        let other = Path::new("/opt/old/tabletist");
+        assert_eq!(relay_for(Some(exe), Some(other)), "/usr/bin/tabletist mcp");
+        assert_eq!(relay_for(Some(exe), None), "/usr/bin/tabletist mcp");
+        let bundle = Path::new("/Applications/Tabletist.app/Contents/MacOS/tabletist");
+        assert_eq!(
+            relay_for(Some(bundle), None),
+            "/Applications/Tabletist.app/Contents/MacOS/tabletist mcp"
+        );
+        let spaced = Path::new("/Users/ada/My Apps/Tabletist's/tabletist");
+        assert_eq!(
+            relay_for(Some(spaced), None),
+            "'/Users/ada/My Apps/Tabletist'\\''s/tabletist' mcp"
+        );
+        // Not known at all: the name.
+        assert_eq!(relay_for(None, None), "tabletist mcp");
+        assert_eq!(
+            command_for("tabletist mcp"),
+            "claude mcp add tabletist -- tabletist mcp"
+        );
+    }
+
     #[tokio::test]
     async fn a_line_is_read_to_its_end_and_no_further() {
         let mut from: &[u8] = b"{\"key\":1}\nafter";
diff --git a/src/ui/agents.rs b/src/ui/agents.rs
new file mode 100644
index 0000000..70f4faa
--- /dev/null
+++ b/src/ui/agents.rs
@@ -0,0 +1,38 @@
+//! The Agents tab: what agents do on the window's connection. One state
+//! of four, as the sidebar's Agents entry names it: the server off, access
+//! Off here, nobody here with the command that connects one, or the
+//! sessions that are here.
+
+#[cfg(test)]
+mod tests {
+    #[test]
+    fn a_while_ago_is_said_in_minutes_then_hours() {
+        let locale = crate::i18n::Locale::default();
+        let ago = |secs| super::ago(std::time::Duration::from_secs(secs), locale);
+        assert_eq!(ago(30), "just now");
+        assert_eq!(ago(240), "4 min ago");
+        assert_eq!(ago(2 * 3600 + 60), "2 h ago");
+    }
+
+    /// Where agents cannot reach the app, nothing of them shows: no entry,
+    /// and no tab.
+    #[cfg(not(unix))]
+    #[test]
+    fn nothing_of_agents_shows_where_they_cannot_reach_the_app() {
+        let mut harness = crate::testing::Harness::new();
+        let tab = harness.connect_fake();
+        let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+        let mut saved = harness.app.connections.get(&conn).unwrap().clone();
+        saved.agents.access = crate::connections::AgentAccess::Read;
+        harness.app.connections.upsert(saved);
+        for enabled in [false, true] {
+            harness.app.settings.mcp_enabled = enabled;
+            harness.settle();
+            for state in ["Server off", "Off here", "None connected"] {
+                assert!(!harness.has(&format!("Agents, {state}")), "{state}");
+            }
+        }
+        harness.app.apply(crate::model::Action::ShowAgents(tab));
+        assert_eq!(harness.app.workspace(tab).unwrap().agents_tab(), None);
+    }
+}
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
new file mode 100644
index 0000000..01176e9
--- /dev/null
+++ b/src/ui/agents_tests.rs
@@ -0,0 +1,229 @@
+//! The Agents tab and its entry in the sidebar, in both looks: what they
+//! say of the connection, the ways in, and that nothing the server says
+//! opens the tab or brings it forward.
+
+use std::time::SystemTime;
+
+use crate::backend::{AgentsEvent, Event};
+use crate::connections::AgentAccess;
+use crate::model::{Action, AgentsHere, ConnTabId};
+use crate::testing::Harness;
+use crate::theme::Look;
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
+
+fn connected(look: Look) -> (Harness, ConnTabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_fake();
+    harness.settle();
+    (harness, tab)
+}
+
+/// Sets what agents may do on `tab`'s saved connection.
+fn allow(harness: &mut Harness, tab: ConnTabId, access: AgentAccess) {
+    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+    let mut saved = harness.app.connections.get(&conn).unwrap().clone();
+    saved.agents.access = access;
+    harness.app.connections.upsert(saved);
+}
+
+/// A session of `agent` whose calls have named `tab`'s connection, or
+/// none.
+fn session(harness: &Harness, tab: Option<ConnTabId>, agent: &str) -> tabletist_mcp::SessionInfo {
+    let connections = tab
+        .map(|tab| vec![harness.app.workspace(tab).unwrap().conn_id.0.clone()])
+        .unwrap_or_default();
+    tabletist_mcp::SessionInfo {
+        id: format!("session of {agent}"),
+        agent: Some(agent.to_owned()),
+        project: Some("tabletist-repo".into()),
+        transport: tabletist_mcp::Transport::Stdio,
+        connected_at: SystemTime::now(),
+        calls: 14,
+        connections,
+        last_call_at: Some(SystemTime::now()),
+    }
+}
+
+fn said(harness: &mut Harness, event: AgentsEvent) {
+    harness.app.apply(Action::Backend(Event::Agents(event)));
+    harness.settle();
+}
+
+fn front(harness: &Harness, tab: ConnTabId) -> bool {
+    harness.app.workspace(tab).unwrap().agents_in_front()
+}
+
+fn agents_tabs(harness: &Harness, tab: ConnTabId) -> usize {
+    let workspace = harness.app.workspace(tab).unwrap();
+    workspace
+        .tabs
+        .iter()
+        .filter(|open| matches!(open, crate::model::Tab::Agents(_)))
+        .count()
+}
+
+/// What the entry is called on screen, in `look`'s case.
+fn entry(look: &Look, state: &str) -> String {
+    look.label(&format!("Agents, {state}"))
+}
+
+#[test]
+fn the_entry_says_the_server_then_the_access_then_who_is_here() {
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        // Off in the settings, whatever the connection allows.
+        allow(&mut harness, tab, AgentAccess::Read);
+        harness.settle();
+        assert!(harness.has(&entry(&look, "Server off")), "{}", look.name);
+        harness.app.settings.mcp_enabled = true;
+        allow(&mut harness, tab, AgentAccess::Off);
+        harness.settle();
+        assert!(harness.has(&entry(&look, "Off here")), "{}", look.name);
+        allow(&mut harness, tab, AgentAccess::Read);
+        harness.settle();
+        assert!(
+            harness.has(&entry(&look, "None connected")),
+            "{}",
+            look.name
+        );
+        // An agent that has named no connection is nowhere yet.
+        let nowhere = session(&harness, None, "Cursor");
+        said(&mut harness, AgentsEvent::Sessions(vec![nowhere.clone()]));
+        assert!(
+            harness.has(&entry(&look, "None connected")),
+            "{}",
+            look.name
+        );
+        let here = session(&harness, Some(tab), "Claude Code");
+        said(&mut harness, AgentsEvent::Sessions(vec![nowhere, here]));
+        assert!(harness.has(&entry(&look, "Claude Code")), "{}", look.name);
+        // A server that stops has nobody on it.
+        said(&mut harness, AgentsEvent::Stopped);
+        assert!(
+            harness.has(&entry(&look, "None connected")),
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn with_several_agents_here_the_one_that_called_last_is_named() {
+    let (mut harness, tab) = connected(Look::macos());
+    harness.app.settings.mcp_enabled = true;
+    allow(&mut harness, tab, AgentAccess::Read);
+    let mut earlier = session(&harness, Some(tab), "Cursor");
+    earlier.last_call_at = Some(SystemTime::UNIX_EPOCH);
+    let later = session(&harness, Some(tab), "Claude Code");
+    said(&mut harness, AgentsEvent::Sessions(vec![later, earlier]));
+    assert_eq!(
+        harness.app.agents_here(tab),
+        Some(AgentsHere::Connected {
+            agent: "Claude Code".into(),
+            count: 2
+        })
+    );
+}
+
+#[test]
+fn the_tab_shows_its_state_and_its_buttons_do_what_they_say() {
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        allow(&mut harness, tab, AgentAccess::Read);
+        harness.app.apply(Action::ShowAgents(tab));
+        harness.settle();
+        // The server off: Turn on, which changes no connection's access.
+        assert!(harness.has("The agent server is off"), "{}", look.name);
+        harness.click("Turn on");
+        harness.settle();
+        assert!(harness.app.settings.mcp_enabled, "{}", look.name);
+        assert_eq!(
+            harness.app.agents_here(tab),
+            Some(AgentsHere::NoneConnected)
+        );
+        // Nobody here: the command, and Copy.
+        let command = crate::endpoint::agent_command();
+        assert!(harness.has(command), "{}", look.name);
+        harness.click("Copy");
+        harness.settle();
+        assert_eq!(
+            crate::ui::toast::said(&harness.ctx).as_deref(),
+            Some("Copied the command"),
+            "{}",
+            look.name
+        );
+        // Off here: no command to copy.
+        allow(&mut harness, tab, AgentAccess::Off);
+        harness.settle();
+        assert!(!harness.has(command), "{}", look.name);
+        // Who is here.
+        allow(&mut harness, tab, AgentAccess::Read);
+        let here = session(&harness, Some(tab), "Claude Code");
+        said(&mut harness, AgentsEvent::Sessions(vec![here]));
+        let name = if look.terminal {
+            "claude code"
+        } else {
+            "Claude Code"
+        };
+        assert!(
+            harness.has(&format!(
+                "{name} · tabletist-repo · connected just now · 14 calls · Runs as dev"
+            )),
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn the_tab_names_the_connection_as_an_agent_does_and_counts_only_who_is_here() {
+    let (mut harness, tab) = connected(Look::macos());
+    harness.app.settings.mcp_enabled = true;
+    allow(&mut harness, tab, AgentAccess::Read);
+    // A connection with no environment is its name alone.
+    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+    let mut saved = harness.app.connections.get(&conn).unwrap().clone();
+    saved.environment = crate::env::Environment::None;
+    harness.app.connections.upsert(saved);
+    harness.app.apply(Action::ShowAgents(tab));
+    harness.settle();
+    assert!(harness.has("No agents connected to Fixture"));
+    let here = session(&harness, Some(tab), "Claude Code");
+    said(&mut harness, AgentsEvent::Sessions(vec![here]));
+    assert!(harness.has("Claude Code · tabletist-repo · connected just now · 14 calls"));
+    assert!(harness.has("1 agent here · Server: stdio"));
+    // Its access Off, the agent that worked here is not here.
+    allow(&mut harness, tab, AgentAccess::Off);
+    harness.settle();
+    assert!(harness.has("No agents here · Server: stdio"));
+}
+
+#[test]
+fn what_the_server_says_while_it_is_off_is_not_kept() {
+    let (mut harness, tab) = connected(Look::macos());
+    allow(&mut harness, tab, AgentAccess::Read);
+    let here = session(&harness, Some(tab), "Claude Code");
+    said(&mut harness, AgentsEvent::Sessions(vec![here]));
+    assert!(harness.app.agents.sessions.is_empty());
+}
+
+#[test]
+fn closing_the_tab_stops_nothing() {
+    let (mut harness, tab) = connected(Look::macos());
+    harness.app.settings.mcp_enabled = true;
+    allow(&mut harness, tab, AgentAccess::Read);
+    let here = session(&harness, Some(tab), "Claude Code");
+    said(&mut harness, AgentsEvent::Sessions(vec![here]));
+    harness.app.apply(Action::ShowAgents(tab));
+    assert!(front(&harness, tab));
+    let id = harness.app.workspace(tab).unwrap().agents_tab().unwrap();
+    let sent = harness.app.backend.sent.len();
+    harness.app.apply(Action::CloseTab { tab, id });
+    assert_eq!(agents_tabs(&harness, tab), 0);
+    assert_eq!(harness.app.agents.sessions.len(), 1);
+    assert_eq!(harness.app.backend.sent.len(), sent, "nothing is sent");
+}
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index f221129..afacb3c 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -2,6 +2,12 @@
 //! state directly.
 
 pub mod about;
+pub mod agents;
+// Where agents can reach the app: elsewhere nothing of them shows, which
+// the last test of `agents` holds.
+#[cfg(test)]
+#[cfg(unix)]
+mod agents_tests;
 pub mod cell_editor;
 #[cfg(test)]
 mod complete_tests;
diff --git a/src/ui/object_tabs.rs b/src/ui/object_tabs.rs
index cfdd78f..7b89f40 100644
--- a/src/ui/object_tabs.rs
+++ b/src/ui/object_tabs.rs
@@ -480,7 +480,7 @@ mod tests {
                                 name,
                                 pinned,
                                 active,
-                                sql: false,
+                                kind: Kind::Table,
                                 unsaved,
                             };
                             widths.push(mac_tab(ui, &tab, bar, &look, &palette).rect.width());
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib ui::agents`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0425]: cannot find function `ago` in module `super`
error[E0425]: cannot find function `agent_command` in module `crate::endpoint`
error[E0433]: cannot find type `Kind` in this scope
error[E0425]: cannot find function `relay_for` in this scope
error[E0425]: cannot find function `command_for` in this scope
error[E0599]: no method named `agents_in_front` found for reference `&Workspace` in the current scope
error[E0599]: no variant, associated function, or constant named `Agents` found for enum `model::Tab` in the current scope
error[E0599]: no variant, associated function, or constant named `ShowAgents` found for enum `model::Action` in the current scope
error[E0599]: no method named `agents_tab` found for reference `&Workspace` in the current scope
error[E0560]: struct `object_tabs::Tab<'_>` has no field named `kind`
error: could not compile `tabletist` (lib test) due to 17 previous errors
```

- [ ] **Step 3: The tab and the entry**

````diff
diff --git a/src/app.rs b/src/app.rs
index ef888e9..9746966 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -903,6 +903,7 @@ impl App {
                             let (columns, rows, _) = sql.shown_rows()?;
                             ("query", columns, rows)
                         }
+                        Tab::Agents(_) => return None,
                     };
                     let tabletist_db::Value::Bytes(bytes) = rows.get(row)?.get(col)? else {
                         return None;
@@ -1388,6 +1389,8 @@ impl App {
                 }
             }
             Action::NewSqlTab(tab) => self.new_sql_tab(tab),
+            Action::ShowAgents(tab) => self.show_agents(tab),
+            Action::TurnAgentsOn => self.change_settings(|settings| settings.mcp_enabled = true),
             Action::RunSql { tab, sql_tab, all } => {
                 if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                     sql.completion = None;
@@ -1766,8 +1769,11 @@ impl App {
                     workspace.tree = Tree::default();
                     workspace.catalog_changed();
                     // The other database has other objects, but a SQL
-                    // editor's text is the user's work: it stays.
-                    workspace.tabs.retain(|open| matches!(open, Tab::Sql(_)));
+                    // editor's text is the user's work: it stays, and so
+                    // does the connection's Agents tab.
+                    workspace
+                        .tabs
+                        .retain(|open| matches!(open, Tab::Sql(_) | Tab::Agents(_)));
                     workspace.recent.clear();
                     workspace.active_tab = workspace
                         .active_tab
@@ -3484,6 +3490,42 @@ impl App {
 
     /// Opens an empty SQL editor after the workspace's other tabs, with the
     /// limit and timeout of the settings, and shows it.
+    /// Shows the workspace's Agents tab, opening it at the end of the
+    /// strip where it is not open yet. Only what the person does calls
+    /// it: nothing the server says opens the tab or brings it forward.
+    fn show_agents(&mut self, tab: ConnTabId) {
+        if !crate::endpoint::AVAILABLE {
+            return;
+        }
+        let id = TabId(self.next_id());
+        let Some(workspace) = self.workspace_mut(tab) else {
+            return;
+        };
+        let shown = workspace.agents_tab().unwrap_or_else(|| {
+            workspace
+                .tabs
+                .push(Tab::Agents(crate::model::AgentsTab { id }));
+            id
+        });
+        workspace.active_tab = Some(shown);
+        workspace.pane = Pane::Grid;
+    }
+
+    /// What an agent calls the workspace's connection, Off or not: its
+    /// saved name and environment, as `list_connections` says them. The
+    /// tab's own name where the connection is saved no more.
+    pub fn agents_name(&self, tab: ConnTabId) -> Option<String> {
+        let workspace = self.workspace(tab)?;
+        let saved = &self.connections.connections;
+        let names = crate::backend::agent_names(saved);
+        let found = saved
+            .iter()
+            .zip(names)
+            .find(|(saved, _)| saved.id == workspace.conn_id)
+            .map(|(_, name)| name);
+        Some(found.unwrap_or_else(|| workspace.name.clone()))
+    }
+
     /// What the workspace's Agents entry and tab say of its connection.
     pub fn agents_here(&self, tab: ConnTabId) -> Option<crate::model::AgentsHere> {
         let workspace = self.workspace(tab)?;
diff --git a/src/endpoint.rs b/src/endpoint.rs
index f9380e2..0fb566c 100644
--- a/src/endpoint.rs
+++ b/src/endpoint.rs
@@ -95,7 +95,19 @@ pub fn same_key(given: &str, key: &str) -> bool {
 /// agent started in, without control characters, at most 64 characters.
 pub fn project(cwd: &str) -> Option<String> {
     let last = std::path::Path::new(cwd).file_name()?.to_string_lossy();
-    let name: String = last.chars().filter(|c| !c.is_control()).take(64).collect();
+    // Nothing that moves the cursor, turns the line around or cannot be
+    // seen, as an agent's own name (`tabletist_mcp`'s `label`).
+    let unseen = |c: char| {
+        c.is_control()
+            || matches!(
+                c,
+                '\u{200B}'..='\u{200F}'
+                    | '\u{2028}'..='\u{202E}'
+                    | '\u{2060}'..='\u{2069}'
+                    | '\u{FEFF}'
+            )
+    };
+    let name: String = last.chars().filter(|c| !unseen(*c)).take(64).collect();
     (!name.is_empty()).then_some(name)
 }
 
@@ -129,6 +141,67 @@ pub async fn read_line<R: AsyncRead + Unpin>(
     }
 }
 
+/// The command that connects Claude Code, the agent the boards name, to
+/// this Tabletist: `claude mcp add tabletist -- tabletist mcp`, with this
+/// program's whole path where `tabletist` on the `PATH` is not this
+/// program (the app bundle on macOS). Worked out once.
+pub fn agent_command() -> &'static str {
+    static COMMAND: std::sync::OnceLock<String> = std::sync::OnceLock::new();
+    COMMAND.get_or_init(|| {
+        let exe = std::env::current_exe().ok();
+        let on_path = std::env::var_os("PATH").and_then(|path| {
+            std::env::split_paths(&path)
+                .map(|dir| dir.join(EXE))
+                .find(|file| file.is_file())
+        });
+        command_for(&relay_for(exe.as_deref(), on_path.as_deref()))
+    })
+}
+
+/// This program's file name.
+const EXE: &str = if cfg!(windows) {
+    "tabletist.exe"
+} else {
+    "tabletist"
+};
+
+/// What adds `relay` to Claude Code as Tabletist's MCP server.
+fn command_for(relay: &str) -> String {
+    format!("claude mcp add tabletist -- {relay}")
+}
+
+/// How an agent starts the relay: `tabletist mcp` when `on_path`, the
+/// program a shell finds by that name, is `exe`, this program; else
+/// `exe`'s whole path, as a shell reads it.
+fn relay_for(exe: Option<&std::path::Path>, on_path: Option<&std::path::Path>) -> String {
+    let same = |a: &std::path::Path, b: &std::path::Path| match (
+        std::fs::canonicalize(a),
+        std::fs::canonicalize(b),
+    ) {
+        (Ok(a), Ok(b)) => a == b,
+        _ => a == b,
+    };
+    match exe {
+        Some(exe) if !on_path.is_some_and(|found| same(exe, found)) => {
+            format!("{} mcp", shell_word(&exe.to_string_lossy()))
+        }
+        _ => "tabletist mcp".to_owned(),
+    }
+}
+
+/// `word` as a POSIX shell reads it back: as it is when nothing in it is
+/// special, else in single quotes.
+fn shell_word(word: &str) -> String {
+    let plain = !word.is_empty()
+        && word
+            .chars()
+            .all(|c| c.is_ascii_alphanumeric() || "/._-+:@%,=".contains(c));
+    match plain {
+        true => word.to_owned(),
+        false => format!("'{}'", word.replace('\'', "'\\''")),
+    }
+}
+
 /// `value` as one line of JSON, with its end.
 pub fn line_of<T: Serialize>(value: &T) -> Vec<u8> {
     let mut line = serde_json::to_vec(value).unwrap_or_default();
diff --git a/src/keymap.rs b/src/keymap.rs
index 6a2f2de..24073a6 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -60,10 +60,12 @@ pub enum Scope {
     Results,
     /// A prompt or a dialog: nothing under it has the keys.
     Prompt,
+    /// The Agents tab.
+    Agents,
 }
 
 impl Scope {
-    pub const ALL: [Scope; 9] = [
+    pub const ALL: [Scope; 10] = [
         Scope::Global,
         Scope::Connections,
         Scope::Sidebar,
@@ -73,6 +75,7 @@ impl Scope {
         Scope::SqlEditor,
         Scope::Results,
         Scope::Prompt,
+        Scope::Agents,
     ];
 
     /// The scope's name in `settings.toml` (`[keys.<scope>]`).
@@ -87,6 +90,7 @@ impl Scope {
             Self::SqlEditor => "sql-editor",
             Self::Results => "results",
             Self::Prompt => "prompt",
+            Self::Agents => "agents",
         }
     }
 
@@ -327,6 +331,9 @@ commands! {
     // Navigation.
     FindObject,
     NewSqlTab,
+    ShowAgents,
+    CopyAgentCommand,
+    TurnAgentsOn,
     ToggleSidebar,
     PaneLeft,
     PaneDown,
@@ -514,6 +521,15 @@ impl Info {
     }
 }
 
+/// An agents' command's info: answered only where agents can reach the
+/// app (`endpoint::AVAILABLE`).
+const fn agents(info: Info) -> Info {
+    match crate::endpoint::AVAILABLE {
+        true => info,
+        false => info.reserved(),
+    }
+}
+
 impl Command {
     pub fn info(self) -> Info {
         use Command as C;
@@ -549,6 +565,12 @@ impl Command {
             C::FilterConnections => info("Filter the connections", "", Window),
             C::FindObject => info("Find any object", "Find any object", Navigation),
             C::NewSqlTab => info("New SQL editor tab", "New SQL editor tab", Navigation),
+            // Nothing of agents answers where they cannot reach the app.
+            C::ShowAgents => agents(info("Show the Agents tab", "", Navigation)),
+            C::CopyAgentCommand => {
+                agents(info("Copy the command that connects an agent", "", Window))
+            }
+            C::TurnAgentsOn => agents(info("Turn the agent server on", "", Window)),
             C::ToggleSidebar => info("Toggle sidebar", "Toggle sidebar", Navigation),
             C::PaneLeft => info("Pane to the left", PANES, Navigation),
             C::PaneDown => info("Pane below", PANES, Navigation).reserved(),
@@ -783,7 +805,8 @@ impl Binding {
                 | Scope::Sidebar
                 | Scope::Grid
                 | Scope::Inspector
-                | Scope::Results => Mode::Normal,
+                | Scope::Results
+                | Scope::Agents => Mode::Normal,
             },
             set => set,
         }
@@ -816,6 +839,8 @@ const SQL_EDITOR: &[Scope] = &[Scope::SqlEditor];
 const RESULTS: &[Scope] = &[Scope::Results];
 const PROMPT: &[Scope] = &[Scope::Prompt];
 const ROWS: &[Scope] = &[Scope::Grid, Scope::Results];
+const AGENTS: &[Scope] = &[Scope::Agents];
+const TREE_AND_GRID: &[Scope] = &[Scope::Sidebar, Scope::Grid];
 const CELLS: &[Scope] = &[Scope::Grid, Scope::Inspector];
 
 /// The keymap. The rows of the settled table first in each part, then the
@@ -922,6 +947,15 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k("cmd+t")]),
             keys(&[k("ctrl+t")]),
         ),
+        bind(
+            C::ShowAgents,
+            GLOBAL,
+            keys(&[k("cmd+alt+a")]),
+            keys(&[k(":agents")]),
+        ),
+        bind(C::ShowAgents, TREE_AND_GRID, NONE, keys(&[k("ga")])),
+        bind(C::CopyAgentCommand, AGENTS, NONE, keys(&[k("y")])),
+        bind(C::TurnAgentsOn, AGENTS, NONE, keys(&[k("o")])),
         bind(
             C::ToggleSidebar,
             GLOBAL,
diff --git a/src/model.rs b/src/model.rs
index 69aac75..5df23b3 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -610,6 +610,12 @@ pub enum Action {
     CancelQuery(ConnTabId),
     /// Open a SQL editor in the connection tab's workspace and show it.
     NewSqlTab(ConnTabId),
+    /// Show the workspace's Agents tab: open it at the end of the strip
+    /// where it is not open yet.
+    ShowAgents(ConnTabId),
+    /// Turn the agent server on, from the Agents tab: a change of the
+    /// settings, which starts it. No connection's access changes.
+    TurnAgentsOn,
     /// Run the statement at the editor's cursor, or every statement.
     RunSql {
         tab: ConnTabId,
@@ -2465,13 +2471,15 @@ impl ObjectTab {
     }
 }
 
-/// One open tab in a workspace. Both kinds are boxed: an object tab is
-/// much larger than a SQL editor, and a SQL editor much larger than a
-/// pointer (clippy's large_enum_variant either way).
+/// One open tab in a workspace. A table and a SQL editor are boxed: an
+/// object tab is much larger than a SQL editor, and a SQL editor much
+/// larger than a pointer (clippy's large_enum_variant either way).
 #[derive(Debug)]
 pub enum Tab {
     Object(Box<ObjectTab>),
     Sql(Box<SqlTab>),
+    /// What agents do on the workspace's connection. One at most.
+    Agents(AgentsTab),
 }
 
 impl Tab {
@@ -2479,42 +2487,45 @@ impl Tab {
         match self {
             Self::Object(object) => object.id,
             Self::Sql(sql) => sql.id,
+            Self::Agents(agents) => agents.id,
         }
     }
 
     pub fn as_object(&self) -> Option<&ObjectTab> {
         match self {
             Self::Object(object) => Some(object.as_ref()),
-            Self::Sql(_) => None,
+            Self::Sql(_) | Self::Agents(_) => None,
         }
     }
 
     pub fn as_object_mut(&mut self) -> Option<&mut ObjectTab> {
         match self {
             Self::Object(object) => Some(object.as_mut()),
-            Self::Sql(_) => None,
+            Self::Sql(_) | Self::Agents(_) => None,
         }
     }
 
     pub fn as_sql(&self) -> Option<&SqlTab> {
         match self {
             Self::Sql(sql) => Some(sql.as_ref()),
-            Self::Object(_) => None,
+            Self::Object(_) | Self::Agents(_) => None,
         }
     }
 
     pub fn as_sql_mut(&mut self) -> Option<&mut SqlTab> {
         match self {
             Self::Sql(sql) => Some(sql.as_mut()),
-            Self::Object(_) => None,
+            Self::Object(_) | Self::Agents(_) => None,
         }
     }
 
-    /// What the tab is loading; closing it cancels these.
+    /// What the tab is loading; closing it cancels these. The Agents tab
+    /// loads nothing of the session's.
     pub fn pending(&self) -> Vec<RequestId> {
         match self {
             Self::Object(object) => object.pending().collect(),
             Self::Sql(sql) => sql.run.pending.into_iter().collect(),
+            Self::Agents(_) => Vec::new(),
         }
     }
 
@@ -2526,6 +2537,14 @@ impl Tab {
     }
 }
 
+/// The Agents tab: the app's own view of what agents do on the
+/// workspace's connection. What it shows lives in [`AgentsState`], not
+/// here: closing the tab stops nothing.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct AgentsTab {
+    pub id: TabId,
+}
+
 /// What the app knows of the agent server: the sessions open on it, as
 /// the backend last said them. Empty while the server is off.
 #[derive(Debug, Clone, Default, PartialEq, Eq)]
@@ -3403,6 +3422,7 @@ impl Workspace {
         match self.tab(self.active_tab?)? {
             Tab::Object(object) => (object.view == ObjectView::Data).then_some(object.id),
             Tab::Sql(sql) => sql.selected_row().map(|_| sql.id),
+            Tab::Agents(_) => None,
         }
     }
 
@@ -3415,6 +3435,19 @@ impl Workspace {
         self.tabs.iter_mut().filter_map(Tab::as_object_mut)
     }
 
+    /// The Agents tab, where it is open.
+    pub fn agents_tab(&self) -> Option<TabId> {
+        self.tabs.iter().find_map(|open| match open {
+            Tab::Agents(agents) => Some(agents.id),
+            Tab::Object(_) | Tab::Sql(_) => None,
+        })
+    }
+
+    /// Whether the tab in front is the Agents tab.
+    pub fn agents_in_front(&self) -> bool {
+        self.active_tab.is_some() && self.active_tab == self.agents_tab()
+    }
+
     /// The SQL editors, in strip order.
     pub fn sql_tabs(&self) -> impl Iterator<Item = &SqlTab> {
         self.tabs.iter().filter_map(Tab::as_sql)
diff --git a/src/theme.rs b/src/theme.rs
index b3f25c2..37581c6 100644
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -701,6 +701,7 @@ fastframe_icons::icons! {
         PanelRight => "panel-right",
         Server => "server",
         ShieldAlert => "shield-alert",
+        SquareTerminal => "square-terminal",
         Table => "table-2",
         WifiOff => "wifi-off",
     }
diff --git a/src/ui/agents.rs b/src/ui/agents.rs
index 70f4faa..4dec987 100644
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -3,6 +3,410 @@
 //! Off here, nobody here with the command that connects one, or the
 //! sessions that are here.
 
+use egui::{Frame, Id, Margin, Rect, pos2, vec2};
+
+use crate::app::App;
+use crate::i18n::{gettext, ngettext};
+use crate::model::{Action, AgentsHere, ConnTabId};
+use crate::theme::{Icon, Look, Palette};
+use crate::typography::{Text, TextRole};
+use crate::ui::{states, widgets};
+
+/// What the app says once the command is on the clipboard.
+pub const COPIED_COMMAND: &str = "Copied the command";
+
+/// Puts the command that connects an agent on the clipboard, and says so.
+pub fn copy_command(ctx: &egui::Context, locale: crate::i18n::Locale) {
+    ctx.copy_text(crate::endpoint::agent_command().to_owned());
+    crate::ui::toast::say(ctx, &gettext(locale, COPIED_COMMAND));
+}
+
+/// The height of the tab's line at its top, and of its status bar.
+const HEADER: f32 = 40.0;
+const STATUS: f32 = 33.0;
+
+pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
+    let (look, palette, locale) = (app.look, app.palette, app.locale);
+    let Some(here) = app.agents_here(tab) else {
+        return;
+    };
+    let Some(workspace) = app.workspace(tab) else {
+        return;
+    };
+    // As an agent names it: "Bookshop · dev".
+    let connection = app.agents_name(tab).unwrap_or_default();
+    let conn = workspace.conn_id.clone();
+    // As the saved connection is, which the agent's session is opened as:
+    // the window keeps what it was opened with.
+    let environment = app
+        .connections
+        .get(&conn)
+        .map_or(workspace.environment, |saved| saved.environment);
+    let runs_as = environment.named(crate::env::Platform::Native);
+    // What the connection allows, whether the server runs or not.
+    let allowed = app.connections.get(&conn).map(|saved| saved.agents.access);
+    let access = look.label(&match allowed {
+        Some(crate::connections::AgentAccess::Read) => gettext(locale, "Read"),
+        Some(crate::connections::AgentAccess::Off) | None => gettext(locale, "Off"),
+    });
+    let sessions: Vec<tabletist_mcp::SessionInfo> =
+        app.agents.here(&conn).into_iter().cloned().collect();
+    let layout = app.layout();
+    let show_key = app.keymap.key(layout, crate::keymap::Command::ShowAgents);
+    let copy_key = app
+        .keymap
+        .key(layout, crate::keymap::Command::CopyAgentCommand);
+    let on_key = app.keymap.key(layout, crate::keymap::Command::TurnAgentsOn);
+    let mut actions = Vec::new();
+    egui::Panel::top(Id::new(("agents-header", tab.0)))
+        .exact_size(HEADER)
+        .resizable(false)
+        .show_separator_line(false)
+        .frame(
+            Frame::new()
+                .fill(palette.panel)
+                .inner_margin(Margin::symmetric(16, 0)),
+        )
+        .show(ui, |ui| {
+            let full = ui.max_rect().expand2(vec2(16.0, 0.0));
+            let rule = if look.terminal {
+                palette.outline
+            } else {
+                palette.border
+            };
+            widgets::hline(ui, full.x_range(), full.bottom() - 0.5, rule);
+            let on = look.label(&gettext(locale, "Agents on"));
+            // A connection's name gives way at its end to a narrow tab.
+            let strong = TextRole::pick(&look, TextRole::UiBodyStrong, TextRole::OBody);
+            let body = widgets::body(&look);
+            let around = body.width(ui.ctx(), look.faces, &format!("{on}  · {access}"));
+            let room = (full.width() - 32.0 - around).max(0.0);
+            let name = crate::ui::grid::ellipsize(&connection, room, false, |text| {
+                strong.width(ui.ctx(), look.faces, text)
+            });
+            let line = Text::new(&look)
+                .add(widgets::body(&look), &on, palette.secondary)
+                .space(widgets::body(&look), " ")
+                .add(strong, &name, palette.text)
+                .add(
+                    widgets::body(&look),
+                    &format!(" · {access}"),
+                    palette.secondary,
+                );
+            widgets::paint_text(ui, full.left() + 16.0, full.center().y, line);
+            widgets::announce(ui, full, &format!("{on} {connection} · {access}"));
+        });
+    if !look.terminal {
+        status_bar(ui, tab, (&here, &show_key), (&look, &palette, locale));
+    }
+    egui::CentralPanel::default()
+        .frame(Frame::new().fill(palette.window))
+        .show(ui, |ui| {
+            let rect = ui.max_rect();
+            match &here {
+                AgentsHere::ServerOff => {
+                    let notice = states::Notice {
+                        icon: Icon::SquareTerminal,
+                        title: &gettext(locale, "The agent server is off"),
+                        text: &gettext(
+                            locale,
+                            "Turn it on to let agents like Claude Code use the connections you allow. It runs only while Tabletist is open, on this computer.",
+                        ),
+                    };
+                    let label = gettext(locale, "Turn on");
+                    let button = states::key_button(&label, &on_key, &look).primary();
+                    if states::empty(ui, rect, &notice, vec![button], &look, &palette).is_some() {
+                        actions.push(Action::TurnAgentsOn);
+                    }
+                }
+                AgentsHere::OffHere => {
+                    let title = format!("{} {connection}", gettext(locale, "Agents can't use"));
+                    let notice = states::Notice {
+                        icon: Icon::SquareTerminal,
+                        title: &title,
+                        text: &gettext(
+                            locale,
+                            "Access is Off for this connection, so agents don't see it at all.",
+                        ),
+                    };
+                    states::empty(ui, rect, &notice, Vec::new(), &look, &palette);
+                }
+                AgentsHere::NoneConnected => {
+                    let name = app.workspace(tab).map(|w| w.name.clone()).unwrap_or_default();
+                    none_connected(ui, rect, (&connection, &name, &access), &copy_key, (&look, &palette, locale));
+                }
+                AgentsHere::Connected { .. } => {
+                    sessions_here(
+                        ui,
+                        rect,
+                        &sessions,
+                        runs_as.as_deref(),
+                        (&look, &palette, locale),
+                    );
+                }
+            }
+        });
+    app.actions.extend(actions);
+}
+
+/// The macOS look's status bar: who is here, the server, and the key that
+/// shows the tab.
+fn status_bar(
+    ui: &mut egui::Ui,
+    tab: ConnTabId,
+    (here, key): (&AgentsHere, &crate::keymap::Written),
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+) {
+    // Who is here, in the state that has anyone: an agent stays among
+    // those that named this connection once its access is Off.
+    let count = match here {
+        AgentsHere::Connected { count, .. } => *count,
+        AgentsHere::ServerOff | AgentsHere::OffHere | AgentsHere::NoneConnected => 0,
+    };
+    let people = match count {
+        0 => gettext(locale, "No agents here").into_owned(),
+        count => ngettext(
+            locale,
+            "{n} agent here",
+            "{n} agents here",
+            u32::try_from(count).unwrap_or(u32::MAX),
+        )
+        .replace("{n}", &count.to_string()),
+    };
+    let server = match here {
+        AgentsHere::ServerOff => gettext(locale, "Server: off"),
+        _ => gettext(locale, "Server: stdio"),
+    };
+    let shows = format!("{} {}", key.as_str(), gettext(locale, "shows this tab"));
+    egui::Panel::bottom(Id::new(("agents-status", tab.0)))
+        .exact_size(STATUS)
+        .resizable(false)
+        .show_separator_line(false)
+        .frame(Frame::new().fill(palette.panel))
+        .show(ui, |ui| {
+            let full = ui.max_rect();
+            widgets::hline(ui, full.x_range(), full.top() + 0.5, palette.outline);
+            let status = palette.secondary.lerp_to_gamma(palette.dim, 0.5);
+            let role = widgets::secondary(look);
+            let left = format!("{people} · {server}");
+            widgets::paint_text(
+                ui,
+                full.left() + 16.0,
+                full.center().y,
+                Text::one(look, role, &left, status),
+            );
+            widgets::announce(ui, full, &left);
+            let laid = Text::one(look, role, &shows, status).layout(ui.ctx());
+            laid.paint_left(
+                ui.painter(),
+                full.right() - 16.0 - laid.width(),
+                full.center().y,
+            );
+        });
+}
+
+/// Nobody here: how to connect an agent, the command with Copy beside it,
+/// and what agents may do on this connection. Centred under an icon tile,
+/// or in the terminal look from the top left without one, as an empty
+/// state is (`states::empty`).
+fn none_connected(
+    ui: &mut egui::Ui,
+    rect: Rect,
+    (connection, name, access): (&str, &str, &str),
+    copy_key: &crate::keymap::Written,
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+) {
+    let ctx = ui.ctx().clone();
+    let terminal = look.terminal;
+    let room = (rect.width() - 2.0 * states::INSET).max(0.0);
+    let measure = if terminal { room } else { room.min(420.0) };
+    let align = if terminal {
+        egui::Align::Min
+    } else {
+        egui::Align::Center
+    };
+    let lay = |role: TextRole, text: &str, color| {
+        let mut text = Text::one(look, role, text, color).wrap(measure);
+        text.job_mut().halign = align;
+        text.layout(&ctx)
+    };
+    let title = format!("{} {connection}", gettext(locale, "No agents connected to"));
+    let text = gettext(
+        locale,
+        "Agents like Claude Code can read this database. Connect one with this command, then ask it about {name}.",
+    )
+    .replace("{name}", name);
+    let title_laid = lay(states::title_role(look), &title, palette.text);
+    let text_laid = lay(widgets::body(look), &text, palette.secondary);
+    // The command, and Copy beside it: the command gives way in its
+    // middle to a window too narrow for it, and is copied whole.
+    let label = gettext(locale, "Copy");
+    let copy = states::key_button(&label, copy_key, look).icon(Icon::Copy);
+    let copy_width = copy.width(ui, look);
+    let height = states::button_height(look);
+    let mono = TextRole::pick(look, TextRole::GridCell, TextRole::OBody);
+    let command = crate::endpoint::agent_command();
+    let fits = (room - 8.0 - copy_width - 24.0).clamp(0.0, 520.0);
+    let shown = crate::ui::grid::ellipsize(command, fits, true, |text| {
+        mono.width(&ctx, look.faces, text)
+    });
+    let command_laid = Text::one(look, mono, &shown, palette.text).layout(&ctx);
+    let row_width = command_laid.width() + 24.0 + 8.0 + copy_width;
+    // What agents may do here.
+    let on_this = look.label(&gettext(locale, "On this connection"));
+    let access_word = look.label(&gettext(locale, "Access"));
+    let small = widgets::secondary(look);
+    let facts = Text::new(look)
+        .add(small, &on_this, palette.dim)
+        .space(small, "   ")
+        .add(small, &access_word, palette.secondary)
+        .space(small, " ")
+        .add(small, access, palette.text)
+        .layout(&ctx);
+    let (tile, under_tile) = if terminal { (0.0, 0.0) } else { (44.0, 14.0) };
+    let total = tile
+        + under_tile
+        + title_laid.height()
+        + 8.0
+        + text_laid.height()
+        + 14.0
+        + height
+        + 18.0
+        + facts.height();
+    let (x, mut y) = if terminal {
+        (rect.left() + states::INSET, rect.top() + states::INSET)
+    } else {
+        let top = rect.center().y - total / 2.0;
+        (rect.center().x, top.max(rect.top() + states::INSET))
+    };
+    // A piece's left edge, for one `width` wide.
+    let left = |width: f32| if terminal { x } else { x - width / 2.0 };
+    if !terminal {
+        let tile_rect = Rect::from_center_size(pos2(x, y + tile / 2.0), vec2(tile, tile));
+        ui.painter().rect_filled(
+            tile_rect,
+            egui::CornerRadius::same(look.radius + 4),
+            palette.surface,
+        );
+        Icon::SquareTerminal.image(palette.dim, 20.0).paint_at(
+            ui,
+            Rect::from_center_size(tile_rect.center(), vec2(20.0, 20.0)),
+        );
+        y += tile + under_tile;
+    }
+    let paint = |ui: &egui::Ui, laid: &crate::typography::Laid, said: &str, y: f32| {
+        laid.paint(ui.painter(), pos2(x, y));
+        widgets::announce(
+            ui,
+            Rect::from_min_size(pos2(left(laid.width()), y), laid.size()),
+            said,
+        );
+    };
+    paint(ui, &title_laid, &title, y);
+    y += title_laid.height() + 8.0;
+    paint(ui, &text_laid, &text, y);
+    y += text_laid.height() + 14.0;
+    let row_left = left(row_width);
+    let boxed = Rect::from_min_size(pos2(row_left, y), vec2(command_laid.width() + 24.0, height));
+    widgets::card(ui, boxed, false, look, palette);
+    command_laid.paint_left(ui.painter(), boxed.left() + 12.0, boxed.center().y);
+    widgets::announce(ui, boxed, command);
+    let at = Rect::from_min_size(pos2(boxed.right() + 8.0, y), vec2(copy_width, height));
+    if copy.show_at(ui, at, look, palette).clicked() {
+        copy_command(ui.ctx(), locale);
+    }
+    y += height + 18.0;
+    let facts_left = left(facts.width());
+    facts.paint_left(ui.painter(), facts_left, y + facts.height() / 2.0);
+    widgets::announce(
+        ui,
+        Rect::from_min_size(pos2(facts_left, y), facts.size()),
+        &format!("{on_this}: {access_word} {access}"),
+    );
+}
+
+/// The sessions that are here, the one that called last first: who, where
+/// it was started, since when, how many calls, and as what it runs.
+fn sessions_here(
+    ui: &mut egui::Ui,
+    rect: Rect,
+    sessions: &[tabletist_mcp::SessionInfo],
+    runs_as: Option<&str>,
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+) {
+    let row = if look.terminal { 22.0 } else { 44.0 };
+    let mut top = rect.top() + 12.0;
+    let now = std::time::SystemTime::now();
+    for session in sessions {
+        let line = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), row));
+        let agent = crate::model::agent_name(session);
+        let agent = if look.terminal {
+            look.label(&agent)
+        } else {
+            agent
+        };
+        let since = now.duration_since(session.connected_at).unwrap_or_default();
+        let mut facts = Vec::new();
+        if let Some(project) = &session.project {
+            facts.push(project.clone());
+        }
+        facts.push(format!(
+            "{} {}",
+            gettext(locale, "connected"),
+            ago(since, locale)
+        ));
+        facts.push(
+            ngettext(
+                locale,
+                "{n} call",
+                "{n} calls",
+                u32::try_from(session.calls).unwrap_or(u32::MAX),
+            )
+            .replace("{n}", &session.calls.to_string()),
+        );
+        // A connection with no environment runs as nothing to name.
+        if let Some(runs_as) = runs_as {
+            facts.push(format!("{} {runs_as}", gettext(locale, "Runs as")));
+        }
+        let facts = facts.join(" · ");
+        let dot = crate::ui::states::Tone::Success.color(palette);
+        let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OBody);
+        // What does not fit the line gives way at its end, the name last.
+        let room = (line.width() - 30.0 - 16.0).max(0.0);
+        let name_width = |text: &str| strong.width(ui.ctx(), look.faces, text);
+        let shown_agent = crate::ui::grid::ellipsize(&agent, room, false, name_width);
+        let after = (room - name_width(&shown_agent) - name_width("  ")).max(0.0);
+        let small = widgets::secondary(look);
+        let shown_facts = crate::ui::grid::ellipsize(&facts, after, false, |text| {
+            small.width(ui.ctx(), look.faces, text)
+        });
+        let text = Text::new(look)
+            .add(strong, &shown_agent, palette.text)
+            .space(strong, "  ")
+            .add(small, &shown_facts, palette.secondary);
+        // The dot is painted: not every face has the glyph.
+        ui.painter()
+            .circle_filled(pos2(line.left() + 19.5, line.center().y), 3.5, dot);
+        let x = line.left() + 30.0;
+        widgets::paint_text(ui, x, line.center().y, text);
+        widgets::announce(ui, line, &format!("{agent} · {facts}"));
+        if !look.terminal {
+            widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.border);
+        }
+        top += row;
+    }
+}
+
+/// How long ago, as the person reads it: "just now", "4 min", "2 h".
+pub fn ago(since: std::time::Duration, locale: crate::i18n::Locale) -> String {
+    let minutes = since.as_secs() / 60;
+    match minutes {
+        0 => gettext(locale, "just now").into_owned(),
+        1..60 => format!("{minutes} {}", gettext(locale, "min ago")),
+        _ => format!("{} {}", minutes / 60, gettext(locale, "h ago")),
+    }
+}
+
 #[cfg(test)]
 mod tests {
     #[test]
diff --git a/src/ui/object_tabs.rs b/src/ui/object_tabs.rs
index 7b89f40..1f338d9 100644
--- a/src/ui/object_tabs.rs
+++ b/src/ui/object_tabs.rs
@@ -39,10 +39,10 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let Some(workspace) = app.workspace(tab) else {
         return;
     };
-    // Each tab's id, name, whether it is pinned, whether it is a SQL
-    // editor (never a preview, so it draws like a pinned tab) and whether
-    // it holds changes that are not saved.
-    let tabs: Vec<(TabId, String, bool, bool, bool)> = workspace
+    // Each tab's id, name, whether it is pinned, its kind (a SQL editor
+    // and the Agents tab are never a preview, so they draw like a pinned
+    // tab) and whether it holds changes that are not saved.
+    let tabs: Vec<(TabId, String, bool, Kind, bool)> = workspace
         .tabs
         .iter()
         .map(|open| match open {
@@ -55,13 +55,20 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 let typed = object.edits.editor.as_ref();
                 let typed = typed.is_some_and(|editor| editor.touched);
                 let unsaved = object.edits.pending() || typed;
-                (object.id, name, object.pinned, false, unsaved)
+                (object.id, name, object.pinned, Kind::Table, unsaved)
             }
             model::Tab::Sql(sql) => (
                 sql.id,
                 format!("{} {}", gettext(locale, "Query"), sql.number),
                 true,
+                Kind::Sql,
+                false,
+            ),
+            model::Tab::Agents(agents) => (
+                agents.id,
+                gettext(locale, "Agents").to_string(),
                 true,
+                Kind::Agents,
                 false,
             ),
         })
@@ -120,7 +127,8 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     .show(ui, |ui| {
                         ui.horizontal(|ui| {
                             ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
-                            for (index, (id, name, pinned, sql, unsaved)) in tabs.iter().enumerate()
+                            for (index, (id, name, pinned, kind, unsaved)) in
+                                tabs.iter().enumerate()
                             {
                                 let is_active = Some(*id) == active;
                                 let one = Tab {
@@ -128,7 +136,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                                     name,
                                     pinned: *pinned,
                                     active: is_active,
-                                    sql: *sql,
+                                    kind: *kind,
                                     unsaved: *unsaved,
                                 };
                                 let response = if look.terminal {
@@ -293,14 +301,22 @@ fn pointer_over(ui: &egui::Ui, response: &egui::Response) -> bool {
 /// alone.
 const MARK: f32 = 8.0;
 
+/// What a tab of the strip holds.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+enum Kind {
+    /// A table or a view.
+    Table,
+    Sql,
+    Agents,
+}
+
 /// One tab to draw.
 struct Tab<'a> {
     index: usize,
     name: &'a str,
     pinned: bool,
     active: bool,
-    /// A SQL editor, not a table or view.
-    sql: bool,
+    kind: Kind,
     /// A table with pending changes.
     unsaved: bool,
 }
@@ -340,7 +356,11 @@ fn mac_tab(
     }
     widgets::vline(ui, rect.right() - 0.5, rect.y_range(), palette.border);
     let center = rect.center().y;
-    let icon = if tab.sql { Icon::Code } else { Icon::Table };
+    let icon = match tab.kind {
+        Kind::Table => Icon::Table,
+        Kind::Sql => Icon::Code,
+        Kind::Agents => Icon::SquareTerminal,
+    };
     icon.image(palette.secondary, 14.0).paint_at(
         ui,
         Rect::from_center_size(pos2(rect.left() + 21.0, center), vec2(14.0, 14.0)),
@@ -379,11 +399,11 @@ fn terminal_tab(
     // takes the text colour; a preview tab's reads fainter still.
     let role = TextRole::OBody;
     let number = (tab.index + 1).to_string();
-    // The look's lower case for an editor's name; a table keeps its own.
-    let name = if tab.sql {
-        look.label(tab.name)
-    } else {
-        tab.name.to_owned()
+    // The look's lower case for an editor's name and the Agents tab's; a
+    // table keeps its own.
+    let name = match tab.kind {
+        Kind::Sql | Kind::Agents => look.label(tab.name),
+        Kind::Table => tab.name.to_owned(),
     };
     // A table with pending changes says so after its name.
     let name = if tab.unsaved {
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
index e550459..ad72ac3 100644
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -363,6 +363,7 @@ fn source(workspace: &Workspace, id: TabId, locale: crate::i18n::Locale) -> Opti
                 table: false,
             })
         }
+        Tab::Agents(_) => None,
     }
 }
 
diff --git a/src/ui/sidebar.rs b/src/ui/sidebar.rs
index 2c47e66..db2ce53 100644
--- a/src/ui/sidebar.rs
+++ b/src/ui/sidebar.rs
@@ -72,12 +72,27 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let layout = app.layout();
     let hints = HINTS.map(|(command, label)| (app.keymap.key(layout, command), label));
     let sql_key = app.keymap.key(layout, Command::NewSqlTab);
+    // The Agents entry, where agents can reach the app: what it says, and
+    // the key that shows the tab.
+    // The terminal look names the letters from the tree and the grid, as
+    // the others name their chord.
+    let agents_key = match look.terminal {
+        true => app
+            .keymap
+            .key_in(layout, Command::ShowAgents, crate::keymap::Scope::Grid),
+        false => app.keymap.key(layout, Command::ShowAgents),
+    };
+    let agents = app
+        .agents_here(tab)
+        .filter(|_| crate::endpoint::AVAILABLE)
+        .map(|here| (here, agents_key));
     let Some(workspace) = app.workspace(tab) else {
         return;
     };
     if workspace.sidebar_hidden {
         return;
     }
+    let agents_front = workspace.agents_in_front();
     let rows = workspace.tree.visible_rows(workspace.driver, show_system);
     let schemas = workspace
         .tree
@@ -288,7 +303,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     }
                 });
             }
-            let footer = if look.terminal { FOOTER } else { SQL_FOOTER };
+            let footer = footer_height(&look, agents.is_some());
             let body = ui.available_height() - footer;
             ui.allocate_ui(vec2(full.width(), body.max(0.0)), |ui| {
                 // The tree is one Tab stop, not one for each row: with the
@@ -360,11 +375,17 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                         ui.add_space(8.0);
                     });
             });
+            let skin = (locale, &look, &palette);
             if look.terminal {
                 terminal_footer(ui, full, &hints, &look, &palette);
+                if let Some((here, key)) = &agents {
+                    agents_line(ui, full, tab, (here, key, agents_front), skin, &mut actions);
+                }
             } else {
-                let skin = (locale, &look, &palette);
                 sql_button(ui, full, tab, &sql_key, skin, &mut actions);
+                if let Some((here, key)) = &agents {
+                    agents_button(ui, full, tab, (here, key, agents_front), skin, &mut actions);
+                }
             }
             if lit {
                 focus::pane_border(ui, full, &look, &palette);
@@ -425,6 +446,164 @@ fn terminal_footer(
 /// a rule, then 10 above and 12 below a 34 pt button.
 const SQL_FOOTER: f32 = 1.0 + 10.0 + 34.0 + 12.0;
 
+/// What the Agents entry adds above the footer: a 34 pt button and 8
+/// under it, or in the terminal look a line and 6 under it.
+const AGENTS_BUTTON: f32 = 34.0 + 8.0;
+const AGENTS_LINE: f32 = 22.0 + 6.0;
+
+/// The room under the tree, with the Agents entry where it shows.
+fn footer_height(look: &Look, agents: bool) -> f32 {
+    match (look.terminal, agents) {
+        (true, false) => FOOTER,
+        (true, true) => FOOTER + AGENTS_LINE,
+        (false, false) => SQL_FOOTER,
+        (false, true) => SQL_FOOTER + AGENTS_BUTTON,
+    }
+}
+
+/// What the Agents entry says of the connection, in which colour, and
+/// whether a dot leads it: an agent is here.
+fn agents_said(
+    here: &crate::model::AgentsHere,
+    look: &Look,
+    palette: &Palette,
+    locale: Locale,
+) -> (String, egui::Color32, bool) {
+    use crate::model::AgentsHere;
+    let said = match here {
+        AgentsHere::ServerOff => gettext(locale, "Server off").into_owned(),
+        AgentsHere::OffHere => gettext(locale, "Off here").into_owned(),
+        AgentsHere::NoneConnected => gettext(locale, "None connected").into_owned(),
+        AgentsHere::Connected { agent, .. } => agent.clone(),
+    };
+    let color = match here {
+        AgentsHere::ServerOff | AgentsHere::OffHere => palette.dim,
+        AgentsHere::NoneConnected => palette.secondary,
+        AgentsHere::Connected { .. } => crate::ui::states::Tone::Success.color(palette),
+    };
+    let said = look.label(&said);
+    (said, color, matches!(here, AgentsHere::Connected { .. }))
+}
+
+/// The dot that leads an agent's name: painted, as not every face has
+/// the glyph. Its right edge at `right`.
+fn agent_dot(ui: &egui::Ui, right: f32, y: f32, color: egui::Color32) {
+    ui.painter().circle_filled(pos2(right - 3.5, y), 3.5, color);
+}
+
+/// The Agents entry of macOS and Windows: a button above SQL Editor, as
+/// wide, its state at its right, lit while the tab is in front. It shows
+/// the tab, and never opens it by itself.
+fn agents_button(
+    ui: &mut egui::Ui,
+    full: Rect,
+    tab: ConnTabId,
+    (here, key, front): (&crate::model::AgentsHere, &Written, bool),
+    (locale, look, palette): (Locale, &Look, &Palette),
+    actions: &mut Vec<Action>,
+) {
+    let top = full.bottom() - SQL_FOOTER - AGENTS_BUTTON;
+    let rect = Rect::from_min_size(
+        pos2(full.left() + 12.0, top + 1.0 + 10.0),
+        vec2(full.width() - 24.0, 34.0),
+    );
+    let label = gettext(locale, "Agents");
+    let (said, color, dot) = agents_said(here, look, palette, locale);
+    let read = format!("{label}, {said}");
+    let response = ButtonSpec::new(&label)
+        .label(&read)
+        .icon(Icon::SquareTerminal)
+        .padding(10.0)
+        .gap(8.0)
+        .justified()
+        .show_at(ui, rect, look, palette)
+        .on_hover_text(format!("{label} · {}", key.as_str()));
+    // Lit while the tab is in front.
+    if front {
+        ui.painter().rect_stroke(
+            rect,
+            egui::CornerRadius::same(look.radius),
+            egui::Stroke::new(1.5, palette.accent),
+            egui::StrokeKind::Inside,
+        );
+    }
+    // The state gives way at its end to the label and the icon: an agent
+    // may call itself 64 letters long.
+    let role = widgets::secondary(look);
+    let used = 10.0 + 14.0 + 8.0 + TextRole::UiBody.width(ui.ctx(), look.faces, &label) + 16.0;
+    let room = (rect.width() - used - 10.0 - if dot { 12.0 } else { 0.0 }).max(0.0);
+    let said = crate::ui::grid::ellipsize(&said, room, false, |text| {
+        role.width(ui.ctx(), look.faces, text)
+    });
+    let laid = Text::one(look, role, &said, color).layout(ui.ctx());
+    let left = rect.right() - 10.0 - laid.width();
+    laid.paint_left(ui.painter(), left, rect.center().y);
+    if dot {
+        agent_dot(ui, left - 5.0, rect.center().y, color);
+    }
+    if response.clicked() {
+        actions.push(Action::ShowAgents(tab));
+    }
+}
+
+/// The Agents entry of the terminal look: the line above the key hints,
+/// `agents`, its state, and the key that shows the tab.
+fn agents_line(
+    ui: &mut egui::Ui,
+    full: Rect,
+    tab: ConnTabId,
+    (here, key, front): (&crate::model::AgentsHere, &Written, bool),
+    (locale, look, palette): (Locale, &Look, &Palette),
+    actions: &mut Vec<Action>,
+) {
+    let top = full.bottom() - FOOTER - AGENTS_LINE;
+    let rect = Rect::from_min_size(
+        pos2(full.left() + 8.0, top + 2.0),
+        vec2(full.width() - 16.0, 22.0),
+    );
+    let label = look.label(&gettext(locale, "Agents"));
+    let (said, color, dot) = agents_said(here, look, palette, locale);
+    let response = ui.interact(rect, Id::new(("agents-entry", tab.0)), Sense::click());
+    response
+        .widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, format!("{label}, {said}")));
+    let fill = if front || response.hovered() {
+        palette.surface
+    } else {
+        palette.panel
+    };
+    ui.painter()
+        .rect_filled(rect, egui::CornerRadius::ZERO, fill);
+    ui.painter().rect_stroke(
+        rect,
+        egui::CornerRadius::ZERO,
+        egui::Stroke::new(1.0, palette.outline),
+        egui::StrokeKind::Inside,
+    );
+    let role = TextRole::OCaption;
+    Text::one(look, role, &label, palette.text)
+        .layout(ui.ctx())
+        .paint_left(ui.painter(), rect.left() + 8.0, rect.center().y);
+    let keys = Text::one(look, role, key.as_str(), palette.accent).layout(ui.ctx());
+    let right = rect.right() - 8.0;
+    keys.paint_left(ui.painter(), right - keys.width(), rect.center().y);
+    // The state gives way at its end to the label and the key.
+    let used = 8.0 + role.width(ui.ctx(), look.faces, &label) + 16.0;
+    let room =
+        (right - keys.width() - 10.0 - rect.left() - used - if dot { 12.0 } else { 0.0 }).max(0.0);
+    let said = crate::ui::grid::ellipsize(&said, room, false, |text| {
+        role.width(ui.ctx(), look.faces, text)
+    });
+    let state = Text::one(look, role, &said, color).layout(ui.ctx());
+    let left = right - keys.width() - 10.0 - state.width();
+    state.paint_left(ui.painter(), left, rect.center().y);
+    if dot {
+        agent_dot(ui, left - 5.0, rect.center().y, color);
+    }
+    if response.clicked() {
+        actions.push(Action::ShowAgents(tab));
+    }
+}
+
 /// The button that opens a SQL editor, with its keys: as wide as the
 /// sidebar, 12 in from its sides.
 fn sql_button(
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index 77a3a46..5e88322 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -93,8 +93,12 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             let sql = app
                 .workspace(tab)
                 .is_some_and(|workspace| workspace.active_sql_tab().is_some());
+            let agents = app
+                .workspace(tab)
+                .is_some_and(crate::model::Workspace::agents_in_front);
             match active {
                 Some(sql_tab) if sql => super::sql_editor::show(app, ui, tab, sql_tab),
+                Some(_) if agents => super::agents::show(app, ui, tab),
                 Some(object_tab) => {
                     super::data_view::header(app, ui, tab, object_tab);
                     if view == Some(ObjectView::Data) {
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib ui::agents`, then `cargo test --locked --lib endpoint::`

Expected: PASS: the tab's seven and the endpoint's five, the command's among them.

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 2459 filtered out
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 2461 filtered out
```

- [ ] **Step 5: Look at it.** Render the tab in each state, in both looks, and compare with the boards named above "Where this draws otherwise than the canvas", by eye. A throwaway scene in `src/shots.rs` (`cargo test --locked --features shots --lib shots::<scene> -- --ignored` writes to `target/shots/`). Remove the scene before the commit. No picture is committed, and none goes to GitHub.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src assets
git commit -m "Show what agents may do on a connection, in a tab of its own and in the sidebar"
```

---

### Task 4: The keys, the prompt's line and quick open

- `src/ui/keys.rs`: ⌥⌘A from anywhere in the window; with the tab in front its keys are `Scope::Agents`, where Omarchy's `y` copies and `o` turns on, each only in the state that offers it; `ga` from the tree and the grid.
- `src/app/editing.rs`: `:agents` from any tab, the Agents tab's own too.
- `src/app.rs` and `src/ui/quick_open.rs`: quick open lists `Found`: the Agents tab first where what is typed matches "agents", then the objects.
- `src/ui/workspace.rs`: Omarchy's status line names the tab's keys while it is in front.
- `docs/_reference/keyboard-shortcuts.md`: ⌥⌘A.

**Files:**
- Modify: `src/ui/keys.rs`, `src/app/editing.rs`, `src/app.rs`, `src/ui/quick_open.rs`, `src/ui/workspace.rs`, `src/ui/agents_tests.rs`, `docs/_reference/keyboard-shortcuts.md`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
index 005c1b2..018394c 100644
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -50,6 +50,7 @@ In the connection dialog:
 | --- | --- |
 | Mod+P | Quick open. In it, Up and Down move, Enter opens and Esc closes |
 | Mod+B | Show or hide the sidebar |
+| Mod+Alt+A | Show the Agents tab (Linux and macOS) |
 | Mod+W | Close the tab |
 | Mod+Shift+[ and ] | Previous and next tab |
 | F6, Shift+F6 | Next and previous part of the window |
diff --git a/src/app.rs b/src/app.rs
index 9746966..bd026c3 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -5424,6 +5424,16 @@ mod tests {
             .collect()
     }
 
+    /// Quick open has no Agents tab to offer where agents cannot reach the
+    /// app.
+    #[cfg(not(unix))]
+    #[test]
+    fn quick_open_offers_no_agents_where_they_cannot_reach_the_app() {
+        let (app, _dir) = app();
+        let tab = app.active_tab_id();
+        assert!(!app.quick_open_found(tab, "agents").contains(&Found::Agents));
+    }
+
     #[test]
     fn the_settings_start_and_stop_the_agent_server_once_the_app_serves_it() {
         let (mut app, dir) = app();
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index 01176e9..f4b5a02 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -129,6 +129,82 @@ fn with_several_agents_here_the_one_that_called_last_is_named() {
     );
 }
 
+#[test]
+fn every_way_in_shows_the_one_tab_and_none_opens_it_by_itself() {
+    let (mut harness, tab) = connected(Look::macos());
+    harness.app.settings.mcp_enabled = true;
+    allow(&mut harness, tab, AgentAccess::Read);
+    // Nothing the server says opens the tab.
+    said(&mut harness, AgentsEvent::Started { kept_aside: None });
+    let here = session(&harness, Some(tab), "Claude Code");
+    said(&mut harness, AgentsEvent::Sessions(vec![here]));
+    assert_eq!(agents_tabs(&harness, tab), 0);
+    // The entry.
+    harness.click("Agents, Claude Code");
+    harness.settle();
+    assert!(front(&harness, tab));
+    // Another tab in front: what the server says leaves it there.
+    let query = harness.add_sql_tab(tab);
+    harness.app.apply(Action::ActivateTab { tab, id: query });
+    said(&mut harness, AgentsEvent::Sessions(Vec::new()));
+    said(&mut harness, AgentsEvent::Stopped);
+    assert!(!front(&harness, tab));
+    // The key: the same tab.
+    harness.press(
+        egui::Key::A,
+        egui::Modifiers::COMMAND | egui::Modifiers::ALT,
+    );
+    harness.settle();
+    assert!(front(&harness, tab));
+    assert_eq!(agents_tabs(&harness, tab), 1);
+    // Quick open, from the words that ask for it.
+    harness.app.apply(Action::ActivateTab { tab, id: query });
+    harness.app.apply(Action::OpenQuickOpen);
+    if let Some(crate::model::Dialog::QuickOpen(open)) = &mut harness.app.dialog {
+        open.query = "age".into();
+    }
+    harness.settle();
+    assert_eq!(
+        harness.app.quick_open_found(tab, "age").first(),
+        Some(&crate::app::Found::Agents)
+    );
+    harness.app.apply(Action::QuickOpenPick);
+    assert!(front(&harness, tab));
+    assert_eq!(agents_tabs(&harness, tab), 1);
+    // One letter, or letters only somewhere in the word, are a table's.
+    for typed in ["a", "s", "gen", "act"] {
+        assert_ne!(
+            harness.app.quick_open_found(tab, typed).first(),
+            Some(&crate::app::Found::Agents),
+            "{typed}"
+        );
+    }
+    // And with nothing typed, quick open is the objects'.
+    assert!(
+        !harness
+            .app
+            .quick_open_found(tab, "")
+            .contains(&crate::app::Found::Agents)
+    );
+}
+
+#[test]
+fn omarchy_shows_the_tab_with_ga_and_with_the_agents_line() {
+    let (mut harness, tab) = connected(Look::omarchy());
+    // From the grid.
+    super::tests::type_key(&mut harness, egui::Key::G, "g");
+    super::tests::type_key(&mut harness, egui::Key::A, "a");
+    assert!(front(&harness, tab));
+    assert_eq!(agents_tabs(&harness, tab), 1);
+    // From another tab, by the prompt's line.
+    let query = harness.add_sql_tab(tab);
+    harness.app.apply(Action::ActivateTab { tab, id: query });
+    harness.app.workspace_mut(tab).unwrap().command = Some("agents".into());
+    harness.app.apply(Action::RunCommand(tab));
+    assert!(front(&harness, tab));
+    assert_eq!(agents_tabs(&harness, tab), 1);
+}
+
 #[test]
 fn the_tab_shows_its_state_and_its_buttons_do_what_they_say() {
     for look in looks() {
@@ -179,6 +255,30 @@ fn the_tab_shows_its_state_and_its_buttons_do_what_they_say() {
     }
 }
 
+#[test]
+fn omarchys_letters_copy_and_turn_on_where_the_tab_offers_them() {
+    let (mut harness, tab) = connected(Look::omarchy());
+    allow(&mut harness, tab, AgentAccess::Read);
+    harness.app.apply(Action::ShowAgents(tab));
+    harness.settle();
+    // `y` copies nothing while the server is off.
+    super::tests::type_key(&mut harness, egui::Key::Y, "y");
+    assert_eq!(crate::ui::toast::said(&harness.ctx), None);
+    super::tests::type_key(&mut harness, egui::Key::O, "o");
+    assert!(harness.app.settings.mcp_enabled);
+    // `o` once more is nothing: the server is on.
+    let before = harness.app.settings.clone();
+    super::tests::type_key(&mut harness, egui::Key::O, "o");
+    assert_eq!(harness.app.settings, before);
+    super::tests::type_key(&mut harness, egui::Key::Y, "y");
+    assert_eq!(
+        crate::ui::toast::said(&harness.ctx).as_deref(),
+        Some("Copied the command")
+    );
+    // This look floats no toast: its status line says it.
+    assert!(harness.has("copied the command"));
+}
+
 #[test]
 fn the_tab_names_the_connection_as_an_agent_does_and_counts_only_who_is_here() {
     let (mut harness, tab) = connected(Look::macos());
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib ui::agents_tests`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0433]: cannot find `Found` in `app`
error[E0599]: no method named `quick_open_found` found for struct `app::App` in the current scope
error: could not compile `tabletist` (lib test) due to 6 previous errors
```

- [ ] **Step 3: The ways in by key**

````diff
diff --git a/src/app.rs b/src/app.rs
index bd026c3..a5ac506 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -65,6 +65,14 @@ pub struct TitleBar {
     pub inset: f32,
 }
 
+/// What quick open lists.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum Found {
+    /// The window's Agents tab.
+    Agents,
+    Object(ObjectRef, ObjectKind),
+}
+
 pub struct App {
     pub dirs: AppDirs,
     pub settings: Settings,
@@ -1581,7 +1589,7 @@ impl App {
                     Some(Dialog::QuickOpen(open)) => (open.tab, open.query.clone()),
                     _ => return,
                 };
-                let count = self.quick_open_matches(tab, &query).len();
+                let count = self.quick_open_found(tab, &query).len();
                 if let Some(Dialog::QuickOpen(open)) = &mut self.dialog {
                     open.selected = (open.selected as isize + step)
                         .clamp(0, count.saturating_sub(1) as isize)
@@ -1593,15 +1601,22 @@ impl App {
                     return;
                 };
                 let (tab, query, selected) = (open.tab, open.query.clone(), open.selected);
-                let matches = self.quick_open_matches(tab, &query);
-                if let Some((object, kind)) = matches.get(selected).or(matches.first()).cloned() {
-                    self.dialog = None;
-                    self.apply(Action::OpenObject {
-                        tab,
-                        object,
-                        kind,
-                        pin: true,
-                    });
+                let found = self.quick_open_found(tab, &query);
+                match found.get(selected).or(found.first()).cloned() {
+                    Some(Found::Agents) => {
+                        self.dialog = None;
+                        self.apply(Action::ShowAgents(tab));
+                    }
+                    Some(Found::Object(object, kind)) => {
+                        self.dialog = None;
+                        self.apply(Action::OpenObject {
+                            tab,
+                            object,
+                            kind,
+                            pin: true,
+                        });
+                    }
+                    None => {}
                 }
             }
             Action::ToggleFilterBar(tab) => {
@@ -4095,6 +4110,21 @@ impl App {
 
     /// Loaded objects of `tab`'s connection that match `query`, best first,
     /// at most 50.
+    /// What quick open lists for `query`: the Agents tab first where what
+    /// is typed is the start of "agents", two letters at least, and agents
+    /// can reach the app; then the objects. Fewer letters, or letters
+    /// that are only somewhere in the word, are a table's.
+    pub fn quick_open_found(&self, tab: ConnTabId, query: &str) -> Vec<Found> {
+        let typed = query.trim().to_lowercase();
+        let agents = crate::endpoint::AVAILABLE && typed.len() >= 2 && "agents".starts_with(&typed);
+        let objects = self.quick_open_matches(tab, query).into_iter();
+        agents
+            .then_some(Found::Agents)
+            .into_iter()
+            .chain(objects.map(|(object, kind)| Found::Object(object, kind)))
+            .collect()
+    }
+
     pub fn quick_open_matches(&self, tab: ConnTabId, query: &str) -> Vec<(ObjectRef, ObjectKind)> {
         let Some(workspace) = self.workspace(tab) else {
             return Vec::new();
diff --git a/src/app/editing.rs b/src/app/editing.rs
index 815f036..c1b1534 100644
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -1714,6 +1714,7 @@ impl App {
         let sql_tab = workspace.active_sql_tab().map(|sql| sql.id);
         let scope = match table {
             Some(_) => Scope::Grid,
+            None if workspace.agents_in_front() => Scope::Agents,
             None => Scope::SqlEditor,
         };
         // The Structure view's own lines hold while it shows.
@@ -1732,6 +1733,10 @@ impl App {
             }
         };
         let action = match (read, table) {
+            // From any tab of the window, where agents can reach the app.
+            (Typed::Command(Command::ShowAgents), _) if crate::endpoint::AVAILABLE => {
+                Action::ShowAgents(tab)
+            }
             (Typed::Command(Command::SaveChanges), Some((id, _))) => Action::WriteEdits { tab, id },
             (Typed::Command(Command::DiscardChanges), Some((id, _))) => {
                 Action::DiscardEdits { tab, id }
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 315fe66..85bf6bb 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -506,10 +506,17 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     );
     // The tab in front, whatever has the keyboard in it: a cancel, a run
     // and the `:` prompt are its own from the tree too.
+    // The Agents tab in front: its own keys, and the global ones.
+    let agents = !on_picker
+        && app
+            .workspace(active)
+            .is_some_and(crate::model::Workspace::agents_in_front);
     let front = if on_picker {
         Scope::Connections
     } else if sql.is_some() {
         Scope::SqlEditor
+    } else if agents {
+        Scope::Agents
     } else {
         Scope::Grid
     };
@@ -527,6 +534,8 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         &[Scope::Inspector]
     } else if on_tree {
         &[Scope::Sidebar]
+    } else if agents {
+        &[Scope::Agents]
     } else if sql.is_some() && sql_grid {
         &[Scope::Results, Scope::SqlEditor]
     } else if sql.is_some() {
@@ -697,6 +706,13 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         if in_workspace && on(input, Command::NewSqlTab, Scope::Global).count > 0 {
             actions.push(Action::NewSqlTab(active));
         }
+        // Where agents can reach the app: nothing of them shows elsewhere.
+        if in_workspace
+            && crate::endpoint::AVAILABLE
+            && on(input, Command::ShowAgents, Scope::Global).count > 0
+        {
+            actions.push(Action::ShowAgents(active));
+        }
         if on(input, Command::Connections, Scope::Global).count > 0 {
             actions.push(Action::ShowConnections);
         }
@@ -1651,6 +1667,25 @@ fn held_back(command: Command) -> bool {
     )
 }
 
+/// Whether the Agents tab is in front of `tab`'s workspace and shows the
+/// state whose key this is: the server off (`server_off`), or the one
+/// with the command to copy.
+fn agents_shows(app: &App, tab: ConnTabId, server_off: bool) -> bool {
+    use crate::model::AgentsHere;
+    let front = app
+        .workspace(tab)
+        .is_some_and(crate::model::Workspace::agents_in_front);
+    let here = app.agents_here(tab);
+    front
+        && match server_off {
+            true => here == Some(AgentsHere::ServerOff),
+            false => matches!(
+                here,
+                Some(AgentsHere::NoneConnected | AgentsHere::Connected { .. })
+            ),
+        }
+}
+
 /// Whether `sql` shows a result grid for the keys to move in: its last
 /// run's rows, unless the Messages pane covers them.
 fn shows_grid(sql: &crate::model::SqlTab) -> bool {
@@ -2133,6 +2168,16 @@ fn letters(
         waiting.clear();
         let command = command.filter(|command| !(visual && held_back(*command)));
         match (command, active) {
+            (Some(Command::ShowAgents), _) if crate::endpoint::AVAILABLE => {
+                actions.push(Action::ShowAgents(tab));
+            }
+            // The Agents tab's own letters, by the state it shows.
+            (Some(Command::CopyAgentCommand), _) if agents_shows(app, tab, false) => {
+                crate::ui::agents::copy_command(ctx, app.locale);
+            }
+            (Some(Command::TurnAgentsOn), _) if agents_shows(app, tab, true) => {
+                actions.push(Action::TurnAgentsOn);
+            }
             (Some(Command::EditCell), _) if edits && alone => {
                 opened = true;
                 actions.extend(edit(crate::model::EditStart::Value));
diff --git a/src/ui/quick_open.rs b/src/ui/quick_open.rs
index 6eb5452..90edf6d 100644
--- a/src/ui/quick_open.rs
+++ b/src/ui/quick_open.rs
@@ -15,7 +15,12 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
         Some(Dialog::QuickOpen(open)) => (open.tab, open.query.clone()),
         _ => return,
     };
-    let matches = app.quick_open_matches(tab, &query);
+    let matches = app.quick_open_found(tab, &query);
+    // The Agents entry's right side: the window's connection, and the key.
+    let agents_note = app.agents_name(tab).map(|name| {
+        let key = keymap.key(layout, crate::keymap::Command::ShowAgents);
+        format!("{name} · {}", key.as_str())
+    });
     let Some(Dialog::QuickOpen(open)) = &mut app.dialog else {
         return;
     };
@@ -70,8 +75,17 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
             let output = egui::ScrollArea::vertical()
                 .max_height(360.0)
                 .show(ui, |ui| {
-                    for (index, (object, _)) in matches.iter().enumerate() {
-                        let label = crate::ui::format::object_title(object, true);
+                    for (index, found) in matches.iter().enumerate() {
+                        let label = match found {
+                            crate::app::Found::Agents => gettext(locale, "Agents").into_owned(),
+                            crate::app::Found::Object(object, _) => {
+                                crate::ui::format::object_title(object, true)
+                            }
+                        };
+                        let note = match found {
+                            crate::app::Found::Agents => agents_note.as_deref(),
+                            crate::app::Found::Object(..) => None,
+                        };
                         let selected = index == open.selected;
                         let (rect, response) = ui.allocate_exact_size(
                             egui::vec2(ui.available_width(), look.tree_row),
@@ -117,6 +131,20 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
                             rect.center().y,
                             Text::one(&look, role, &shown, color),
                         );
+                        if let Some(note) = note {
+                            let laid = Text::one(
+                                &look,
+                                widgets::secondary(&look),
+                                note,
+                                palette.secondary,
+                            )
+                            .layout(ui.ctx());
+                            laid.paint_left(
+                                ui.painter(),
+                                rect.right() - 10.0 - laid.width(),
+                                rect.center().y,
+                            );
+                        }
                         if reveal && index == open.selected {
                             response.scroll_to_me(None);
                         }
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index 5e88322..0c40cbb 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2026,6 +2026,13 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let (filter_key, sidebar_key) = (key(Command::WhereFilter), key(Command::ToggleSidebar));
     let (copy_key, save_key) = (key(Command::CopyCells), key(Command::SaveChanges));
     let review_key = key(Command::ReviewSql);
+    let (agents_copy_key, agents_on_key) =
+        (key(Command::CopyAgentCommand), key(Command::TurnAgentsOn));
+    // The Agents tab in front: its keys, by the state it shows.
+    let agents = app
+        .workspace(tab)
+        .filter(|workspace| workspace.agents_in_front())
+        .and_then(|_| app.agents_here(tab));
     let (run_key, run_all_key) = (key(Command::RunStatement), key(Command::RunAll));
     let (leave_key, complete_key) = (key(Command::LeaveEditor), key(Command::InsertCompletion));
     let (keep_key, next_key) = (key(Command::KeepEdit), key(Command::CommitNext));
@@ -2214,6 +2221,35 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             if editing.pending.is_some() || editing.added.is_some() {
                 structure_hints.push((&review_key, &*review, true));
             }
+            let (copy_word, on_word) = (gettext(locale, "copy"), gettext(locale, "turn on"));
+            let mut agents_hints: Vec<widgets::Hint<'_>> = match &agents {
+                Some(crate::model::AgentsHere::ServerOff) => {
+                    vec![(&agents_on_key, &*on_word, true)]
+                }
+                Some(
+                    crate::model::AgentsHere::NoneConnected
+                    | crate::model::AgentsHere::Connected { .. },
+                ) => vec![(&agents_copy_key, &*copy_word, true)],
+                Some(crate::model::AgentsHere::OffHere) | None => Vec::new(),
+            };
+            agents_hints.push((&sidebar_key, "tables", true));
+            if agents.is_some() {
+                let left = rect.left() + 12.0;
+                widgets::key_hints(ui, (left, y), &agents_hints, 18.0, &look, &palette);
+                // What was just done (the command copied), at the line's
+                // end: this look floats no toast.
+                if let Some(said) = &editing.said {
+                    let role = TextRole::OSecondary;
+                    let text = Text::one(&look, role, &said.text, said.color);
+                    let width = widgets::paint_text_right(ui, rect.right() - 12.0, y, text);
+                    let place = Rect::from_min_size(
+                        pos2(rect.right() - 12.0 - width, y - 8.0),
+                        vec2(width.max(1.0), 16.0),
+                    );
+                    widgets::announce(ui, place, &said.text);
+                }
+                return;
+            }
             let hints: &[widgets::Hint<'_>] = if on_editor {
                 &editor_hints
             } else if structure.is_some() {
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib ui::agents_tests`, then `cargo test --locked --lib keymap::` and `cargo test --locked --test keys`

Expected: PASS: the tab's nine, the keymap's, and the keys' rules for the views.

```text
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 2460 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 2446 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src docs/_reference/keyboard-shortcuts.md
git commit -m "Show the Agents tab with a key, a line of the prompt or quick open"
```

---

### Task 5: The master plan says what was built

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: Correct it**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index f6e2a74..6734842 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -42,6 +42,15 @@ the key beside the socket, the app's answer to the relay's first line
 (A.2), the website's reference for the files (G), and the check of changes
 made outside the app moved to the first control that turns agents on (C.2).
 
+**Built on 2026-10-14: the first part of Phase 1b's third run** (`2026-10-14-mcp-server-1b-3a-agents-tab.md`):
+the Agents tab, its entry in the sidebar, the ways in, the three empty
+states with **Turn on** and **Copy**, and the sessions the app learns of.
+G cuts the third run in two, agreed the same day: the app must know who is
+connected before it can say that nobody is, so the sessions came forward
+from the fourth run, and the check of changes made outside the app (C.2)
+is the second part. What the run draws otherwise than the boards is in
+D.4.
+
 Each phase below gets its own plan under `docs/superpowers/plans/` before any
 code is written, as every batch of work in this repository does.
 
@@ -480,8 +489,12 @@ struct AgentSession {
   client's title, else its name, without control characters, at most 64
   characters), the project, when it connected, its count of calls, and the
   stop. The project is given by whoever hands the transport over. Asking the
-  client for its roots is left for later. `last_call_at`, `connections` and
-  `state` come with the tab that shows them, in the fourth run.
+  client for its roots is left for later. `last_call_at` and `connections`
+  came with the Agents tab (`2026-10-14-mcp-server-1b-3a-agents-tab.md`), which needs them to say who is here;
+  `state` comes with the feed, in the fourth run. The server tells the
+  backend when a session opens or ends, names its agent, or calls
+  (`Server::changes`), and the backend tells the app
+  (`AgentsEvent::Sessions`).
 
 **Where a session is** (added on 2026-10-13, for the Agents tab). The tab
 covers one connection, and a session belongs to none: the agent names a
@@ -1376,6 +1389,8 @@ person either way.
 turn agents on, and a check that refused changes made outside the app would
 refuse the only one there is. So after Phase 1b's second run a program that
 can edit both files can turn agents on for itself, as E says of a shell.
+**Moved again on 2026-10-14**: the third run is cut in two, and the check is
+its second part (1b-3b), right after **Turn on** (`2026-10-14-mcp-server-1b-3a-agents-tab.md`).
 
 ### C.3 Hidden tables and masked columns when the schema changes
 
@@ -1599,7 +1614,27 @@ What keeps an approval the person's own:
   window's own session, so it waits for a window that is still connecting.
 
 **Empty states.** In the Activity view's place, by the state the sidebar
-entry shows:
+entry shows.
+
+**As built in Phase 1b's third run** (`2026-10-14-mcp-server-1b-3a-agents-tab.md`). Where it draws otherwise than
+the boards, each comes with what it needs:
+
+- The Activity and Log switch, **Show log**, and "The log still shows past
+  calls" come with the Log view (the fourth run). "Last agent" in the box
+  "On this connection" too: it is read from the log.
+- **Allow agents…**, **Other agents…**, **Settings…** and the header's
+  "Change in Connections" come with the editor's section and Settings ›
+  Agents (the fifth run). The Off state has no button until then.
+- "and propose changes you approve here" comes with writes (Phase 2), and
+  so do the Waiting state of the entry and the count on the tab.
+- With an agent here the tab lists the sessions that are here, without
+  **Stop session**, "Also:" or the feed (the fourth run).
+- A session's line says how long ago it connected ("connected 4 min
+  ago"): the app reads no local clock. The status bar says "Server: stdio"
+  until HTTP (1d).
+- The macOS entry is lit by an accent outline while its tab is in front,
+  which the boards do not draw. Omarchy's empty states are the look's
+  empty state, from the top left, without the boards' box.
 
 | State | Title | Body | Actions |
 |---|---|---|---|
@@ -1962,7 +1997,7 @@ is off until the person turns it on.
 | Part | Runs | State on 2026-10-13 |
 |---|---|---|
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
-| 1b. The server over stdio, with its interface | 6, below | The first two are built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md` and `2026-10-14-mcp-server-1b-2b-stdio.md`). Nothing the others wait for is open |
+| 1b. The server over stdio, with its interface | 7, below | The first two and a half are built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md` and `2026-10-14-mcp-server-1b-3a-agents-tab.md`). Nothing the others wait for is open |
 | 1b on Windows: the way in | 1 | After 1b (question 14) |
 | 1c. Hidden tables | to be cut in its plan | Waits for questions 8 and 13 |
 | 1d. HTTP with tokens | 1 | After stdio (question 9) |
@@ -2031,7 +2066,8 @@ can see what agents do and stop them.
 | **1b-1. The server, with nothing to start it** | The `tabletist-mcp` crate: the server on `rmcp`, the gateway's trait, `list_connections`, `describe_schema` and `run_query` over `agent::check_read` and `Connection::run_agent_read`, the size cap on an answer (B.5), the refusal codes (B.8), the log's store (C.1) | Nothing. No setting, no transport and no command reaches it | F.1 in process, F.4 but its interface, F.5 in process |
 | **1b-2a. The backend's gateway** (built: `2026-10-14-mcp-server-1b-2a-gateway.md`) | `[connections.agents]` with access, rows and timeout, which makes the connections file version 4. `Command::Agents` and `Event::Agents`, the backend's gateway with its own sessions on each database, and the server started and stopped by the backend, with the log's retention. The SDK held out of the app's log | Nothing: no setting starts the server, and no transport reaches it | The connections file, the gateway on SQLite, PostgreSQL and MySQL, the backend's start and stop, the log's filter |
 | **1b-2b. Turning it on from the files, and stdio** (built: `2026-10-14-mcp-server-1b-2b-stdio.md`) | The `[mcp]` keys in `settings.toml`, and the app that starts and stops the server with them and sends the backend what agents may use. The endpoint, the key and the relay, `tabletist mcp`, on Linux and macOS. The keychain on macOS (A.1). The website's reference for the files | With `enabled = true` and an access written by hand, an agent connects over stdio and reads. Nothing in the app shows it yet, and nothing in the app turns it on | F.1 over stdio, F.5 over stdio, the settings file, `tests/cli.rs` |
-| **1b-3. The Agents tab's frame** | The third kind of tab. The sidebar entry with its five states. The ways in: the key, `ga`, `:agents`, quick open. The three empty states, with **Turn on** and **Copy**. The keymap and the help | Open the tab in any state, see which state the connection is in, turn the server on from the tab | F.7: the entry's states, the ways in, one tab for a window, the empty states, nothing opening or focusing by itself |
+| **1b-3a. The Agents tab's frame** (built: `2026-10-14-mcp-server-1b-3a-agents-tab.md`) | The third kind of tab. The sidebar entry with its states but Waiting. The ways in: the key, `ga`, `:agents`, quick open. The three empty states, with **Turn on** and **Copy**. The sessions the app learns of, and those that are here. The keymap and the help | Open the tab in any state, see which state the connection is in and who is here, turn the server on from the tab | F.7: the entry's states, the ways in, one tab for a window, the empty states, nothing opening or focusing by itself |
+| **1b-3b. Changes made outside the app** | C.2: the agent settings the app last showed, kept in the state directory, and a change from outside that makes agents more able held back until the person keeps it | A program that edits the files cannot turn agents on unseen | C.2's cases |
 | **1b-4. What agents are doing** | `App::agents`. The sessions that are here and "Also:". The feed. **Stop session** and **Stop agents here**. The Log view as a list for this connection, with All connections. The notification for an agent that connects | See each agent and each call as it happens, stop one agent or all on a connection, read the log | F.7: one connection, the stops, closing the tab, the log's default. F.4's interface |
 | **1b-5. Letting agents in** | Settings › Agents, with the switch between panes. The Connections list's Agents column, its menu and Omarchy's box. The connection editor's Agents section, with **Allow agents…** and **Show Agents tab** pointing at them. The website: a new guide, and the reference pages | Everything Phase 1 draws, without a file edited by hand | The interface tests of Settings, Connections and the editor. The keymap. `docs/bin/check` |
 
````

- [ ] **Step 2: Check it**

Run: `grep -c $'\u2014' docs/superpowers/plans/2026-10-13-mcp-server.md` (an em dash, written so that this plan holds none)

Expected: `0`.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what the first part of its third run built"
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

Expected: all clean, and every test passes. On 2026-10-14, with both servers up: 23 test binaries, 3,410 passed, none failed, 22 ignored. The website's check ends `HTML-Proofer finished successfully.`

- [ ] **By hand, on Linux**, with `[mcp] enabled = true` and a connection's `access = "read"`: open it, see "None connected" in the sidebar, connect Claude Code with the command the tab copies, ask it to count the books, and see the entry turn to "● Claude Code" and the tab list it.

- [ ] **Open the pull request.** The description names what draws otherwise than the canvas, and that macOS and Windows run in CI alone.

## Self-review

- **Against the master plan's row for this run** (G, as cut): the third kind of tab (Task 3), the sidebar entry with its states but Waiting (Task 3), the ways in: the key, `ga`, `:agents`, quick open (Task 4), the three empty states with **Turn on** and **Copy** (Task 3), the sessions the app learns of (Tasks 1 and 2), the keymap and the help (Tasks 3 and 4: the help lists the keymap). Not here, and said above: C.2's check, the Log, the editor's section and Settings › Agents.
- **Against D.4.** One tab for a window; opened only by what the person does; closing it stops nothing; its place at the end of the strip; the entry in every state, the server off included; Server off before Off here; the agent that called last named. **Against F.7**, without the card and the count: the entry's states, the ways in, one tab for a window, the empty states, nothing opening or focusing by itself, Windows.
- **Against the canvas**, read again on 2026-10-14: what is drawn otherwise is in the table at the top, each with why.
- **Reviewed before it was written.** A reviewer was given the draft and asked to break it, with tests of its own. It found these, each fixed above with a test where one can be written: one letter in quick open put the Agents entry above a table, and Enter opened it; Omarchy's copy said nothing on screen; a long call that ended last took the last call's time back; a session was here only once its first call ended; the tab named the connection otherwise than the agent does, and "Runs as none"; the Off page counted an agent here; a long agent name ran over the entry; a session said as the server stopped could stay, and one that opened first was missed; a project's name could hold the characters that turn a line around; and the Windows test did not look for "None connected". What held: every place a tab's kind is matched, closing, cycling and switching tabs, one Agents tab whatever the ways in, the keys' scopes with the tab in front, nothing the server says opening or focusing the tab, nothing logged of an agent's, the command's path and quoting, and the Windows paths.
- **Names.** `SessionInfo::connections`, `last_call_at`, `Server::changes`, `AgentsEvent::Sessions`, `agent_names`, `AgentsState`, `AgentsHere`, `agent_name`, `agents_here`, `agents_name`, `Tab::Agents`, `AgentsTab`, `agents_tab`, `agents_in_front`, `ShowAgents`, `TurnAgentsOn`, `CopyAgentCommand`, `Scope::Agents`, `agent_command`, `copy_command`, `Found` are spelled the same in every task.
- **Every commit stands.** After Task 1 the server knows where its sessions work, and nobody asks. After Task 2 the app knows who is here, and shows nothing. After Task 3 the tab and the entry show it, and the entry is the one way in. After Task 4 every way in works.

## After the run

Run on 2026-10-14, inline, as written: every diff applied as it stands, every step's run said what the step says, one commit for each task. Task 3's look was rendered headless once more, with an agent's name of 46 letters, which gives way in the entry and on its line; the scene was thrown away. The whole of it at the end, with both servers up: fmt and clippy clean, 23 test binaries, 3,410 passed, none failed, 22 ignored, the documentation without a warning, and the website's check passed.

Not done here, and left for the review of the pull request:

- **The by-hand check with Claude Code** needs the app's window, and no session here has a display. It is Igor's to run once, as "After the last task" says.
- **macOS and Windows** run in CI alone. Two tests run on Windows only: that nothing of agents shows there, and that quick open offers no Agents tab.

## After the pull request's review

- **The command that Copy copies looked at the disk on the UI thread**, in every folder of the `PATH`, the first time the tab was drawn (`AGENTS.md` keeps the disk off it). The app now works it out on a thread of its own as it starts (`endpoint::work_out_agent_command`), and the tab reads it once it is ready (`agent_command_ready`), with "…" until then.
- The guides' keyboard sections name the new keys too: ⌥⌘A on macOS, `:agents` and `ga` on Omarchy, and the tab's `y` and `o`.
- Two doc comments had been left above the new functions instead of their own (`new_sql_tab`, `quick_open_matches`).
