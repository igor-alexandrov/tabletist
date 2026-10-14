# MCP server, Phase 1b, run 3b: changes made outside the app. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A change to `settings.toml` or `connections.toml` made outside Tabletist that lets agents do more than the person was last shown (the server turned on, a connection's access raised) is not in force until the person keeps it. A bar under the bar that leads the window says what changed, with **Keep** and **Put back**. A change that lets agents do less applies at once.

**Architecture:** `agents_shown::Shown` is what the app last showed the person, kept in `state/agents_shown.json`. `App::check_agents` compares the files with it: raises are held in `App::agents_held`, lowerings are taken and remembered. What is in force is `App::agents_on` and `App::agents_access`, which every user of the settings asks: the policy sent to the backend, the start and stop of the server, the Agents entry and tab. The app's own controls show what they turn on: `change_settings` turning the server on, and **Turn on**. The connection dialog has no control for the access, so its save keeps what the file says then. **Keep** and **Put back** answer what the bar said, never a change that came in after it was drawn.

**Tech Stack:** Rust 2024, egui. Nothing new is built.

---

## Where this sits

Phase 1b of `docs/superpowers/plans/2026-10-13-mcp-server.md` is seven runs (section G). Four are merged: the server crate, the backend's gateway, the files and stdio, and the Agents tab's frame.

1. Done: the server, the gateway, the files and stdio, the Agents tab.
2. **This run, 3b: changes made outside the app** (C.2). It was moved here from the first run with a control that turns agents on, which is 3a's **Turn on**.
3. The fourth run, what agents are doing, and the fifth, letting agents in. Then Windows' way in.

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: views draw and push `Action`s and `App::apply` changes state, and disk work is the backend's (the remembered file is written by `Command::Save`, as the other state files are).
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.
- No `Cargo.toml` or `Cargo.lock` changes. Nothing needs PostgreSQL or MySQL; the full run at the end prints "skipped" for their tests without the variables.

## What was run before this plan was written

Every task's state was built, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` (3,425 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. The bar was rendered headless in both looks and looked at; the scene was thrown away. Not run: macOS and Windows, which CI runs; on Windows nothing is held, and the tests that hold are for Unix.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| What is remembered | `agents_shown.json` in the state directory: whether the server is on, and each connection's access by its id (one not there was Off). A connection deleted since is forgotten | C.2. `listen` and the lists come with what they are for (1d, 1c, 3a) |
| A remembered file that is not there, or cannot be read | Shows nothing: whatever lets agents in is held once, at the first start of this version | Taking the files as shown then would let a program that removes the file turn agents on unseen. A person who set agents up by hand keeps it with one click |
| When the files are compared | At the start, at every change of the settings (the file's or the app's), and at every save of the connections | The connections file is read at the start alone, and written by the app |
| More, and less | More is the server turned on, or an access above what was shown (`AgentAccess` is ordered, Off first). It is held. Less is taken at once, and remembered | C.2 |
| What is in force | `App::agents_on` and `App::agents_access`: the files, but what was shown where a raise is held. The policy sent to the backend, the start and stop of the server, the sessions it reports, the Agents entry and the tab's header all ask them | A held change must change nothing an agent or the person sees as in force |
| What is never held | What the app's own controls turn on: `change_settings` turning the server on (**Turn on**, later Settings › Agents) and **Turn on** keeping a held server. The dialog has no control for the access, so its save keeps what the file says then: a raise made outside stays held, one put back stays back | Question 23. The access controls come with 1b-5, and mark what they raise as shown |
| The bar | Under the bar that leads the window (the title bar on macOS, so the window's own buttons keep their line), above the notice, in both looks, on the notice's fill. It says what changed, then "Until you answer, agents may not. Keep it, or put it back?" No dismissal takes it away | C.2. Its place follows the notice's, moved in #132 for the same reason |
| Keep and Put back | `Action::KeepAgentChanges` and `PutBackAgentChanges` carry what the bar said, and answer only what is still held of it. Put back turns the server off, or sets the access back, in the files, and leaves the rest of them | A change that comes in after the bar was drawn waits for its own answer: found by the review |
| Windows | Nothing is held: agents cannot reach the app there yet | `endpoint::AVAILABLE` |
| Omarchy keys for the bar | None: the bar's buttons take the pointer and Tab, as the notice's **Dismiss** does | Nothing on the canvas draws one |

### What this run leaves out

- `listen`, hidden tables and masked columns: C.2 lists them among what is remembered. They come with what they are for (1d, 1c, 3a), and so does their comparison.
- A key for **Keep** and **Put back** on Omarchy. The bar's buttons are reached by the pointer and by Tab, as the notice's **Dismiss** is.
- A change made outside the app while it is closed is found at the next start: the connections file is not watched.
- Only the server and each connection's access are compared. A Read connection pointed at another server by hand is not held: an agent with the person's shell gains little by it.
- The controls that raise a connection's access in the app come with 1b-5, and mark what they raise as shown.

## File structure

| File | What changes |
|---|---|
| `src/agents_shown.rs` | New. `Shown`, `Raised`: what was shown, the comparison, keep |
| `src/connections.rs` | `AgentAccess` is ordered, the least first |
| `src/paths.rs`, `src/backend.rs`, `src/lib.rs` | `agents_shown.json`, its save, the module |
| `src/app.rs` | `agents_shown`, `agents_held`, `check_agents`, `agents_on`, `agents_access`; keep, put back, Turn on; what is in force everywhere it is read; the dialog's save keeps the file's access |
| `src/model.rs` | `Action::KeepAgentChanges`, `PutBackAgentChanges`, with what the bar said |
| `src/ui/mod.rs` | The bar, `agents_held_said`, and `notices`: the bar and the notice, under the leading bar |
| `src/ui/picker.rs`, `src/ui/workspace.rs` | Draw `notices` where they drew the notice |
| `src/ui/agents.rs` | The tab's header says the access in force |
| `src/testing.rs` | The fixture connection, for the app's tests |
| `src/ui/agents_tests.rs` | The tab's tests allow a connection as the person who keeps it; the bar's test |
| `docs/_reference/settings-and-files.md` | What is held, and `agents_shown.json` |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | What this run built |

---

### Task 1: What the app last showed of what agents may do

`agents_shown::Shown`: whether the server was on, and each connection's access, by its id (a connection not there was Off). `compare` takes what the files say: what lets agents do more comes back to be held, what lets them do less is taken into `Shown` at once, and a connection deleted since is forgotten. `keep` takes what was held. The file is `agents_shown.json` in the state directory; one that is not there, or cannot be read, is `Shown::default()`, which shows nothing. `AgentAccess` is ordered, Off before Read, so that "more" is `>`.

Nothing uses it yet: Task 2 does.

**Files:**
- Create: `src/agents_shown.rs`
- Modify: `src/connections.rs`, `src/paths.rs`, `src/backend.rs`, `src/lib.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/agents_shown.rs b/src/agents_shown.rs
new file mode 100644
index 0000000..ba5ff49
--- /dev/null
+++ b/src/agents_shown.rs
@@ -0,0 +1,101 @@
+//! What the app last showed the person of what agents may do: whether the
+//! server is on, and each connection's access. A change made outside the
+//! app (a hand, a program, a synced dotfile, a backup) that lets agents do
+//! more than that is held until the person keeps it or puts it back; one
+//! that lets them do less is taken at once.
+//!
+//! Kept in the state directory (`AppDirs::agents_shown_file`). A file that
+//! is not there, or cannot be read, shows nothing: whatever lets agents in
+//! is held then, once.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::connections::Agents;
+
+    fn saved(id: &str, access: AgentAccess) -> SavedConnection {
+        SavedConnection {
+            id: ConnectionId(id.into()),
+            name: id.into(),
+            environment: crate::env::Environment::Dev,
+            read_only: None,
+            password: crate::connections::PasswordMode::None,
+            ssh_secret: crate::connections::PasswordMode::None,
+            spec: tabletist_db::ConnectSpec::sqlite("/tmp/shop.db"),
+            agents: Agents {
+                access,
+                ..Agents::default()
+            },
+        }
+    }
+
+    #[test]
+    fn what_lets_agents_do_more_is_held_and_what_lets_them_do_less_is_taken() {
+        use AgentAccess::{Off, Read};
+        let mut shown = Shown::default();
+        // Nothing was shown: the server on and a connection Read are held.
+        let (raised, lowered) = shown.compare(true, &[saved("shop", Read), saved("old", Off)]);
+        assert_eq!(
+            raised,
+            [
+                Raised::Server,
+                Raised::Access {
+                    id: ConnectionId("shop".into()),
+                    to: Read
+                }
+            ]
+        );
+        assert!(!lowered);
+        assert_eq!(shown, Shown::default(), "nothing held is taken");
+        shown.keep(&raised);
+        assert!(shown.enabled);
+        assert_eq!(shown.access_of(&ConnectionId("shop".into())), Read);
+        // As shown: nothing to hold, nothing to take.
+        assert_eq!(
+            shown.compare(true, &[saved("shop", Read)]),
+            (Vec::new(), false)
+        );
+        // Less: taken at once.
+        let (raised, lowered) = shown.compare(false, &[saved("shop", Off)]);
+        assert_eq!(raised, []);
+        assert!(lowered);
+        assert_eq!(shown, Shown::default());
+    }
+
+    #[test]
+    fn a_connection_deleted_since_is_forgotten() {
+        let mut shown = Shown::default();
+        shown.keep(&[Raised::Access {
+            id: ConnectionId("gone".into()),
+            to: AgentAccess::Read,
+        }]);
+        let (raised, lowered) = shown.compare(false, &[]);
+        assert_eq!(raised, []);
+        assert!(lowered);
+        assert!(shown.access.is_empty());
+        // Back under the same id, it is new, and held.
+        let (raised, _) = shown.compare(false, &[saved("gone", AgentAccess::Read)]);
+        assert_eq!(raised.len(), 1);
+    }
+
+    #[test]
+    fn the_file_keeps_what_was_shown_and_a_bad_one_shows_nothing() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents_shown.json");
+        assert_eq!(Shown::load(&path), Shown::default());
+        let mut shown = Shown::default();
+        shown.keep(&[
+            Raised::Server,
+            Raised::Access {
+                id: ConnectionId("shop".into()),
+                to: AgentAccess::Read,
+            },
+        ]);
+        shown.save(&path).unwrap();
+        assert_eq!(Shown::load(&path), shown);
+        let text = std::fs::read_to_string(&path).unwrap();
+        assert!(text.contains("\"shop\": \"read\""), "{text}");
+        std::fs::write(&path, "{ not json").unwrap();
+        assert_eq!(Shown::load(&path), Shown::default());
+    }
+}
diff --git a/src/backend.rs b/src/backend.rs
index 6c37b1a..20c06f7 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -203,6 +203,8 @@ pub enum StateFile {
     KnownHosts(HostKeys),
     Settings(crate::settings::Settings),
     Labels(crate::labels::Labels),
+    /// What agents may do, as the app last showed it.
+    AgentsShown(crate::agents_shown::Shown),
 }
 
 impl StateFile {
@@ -216,6 +218,7 @@ impl StateFile {
             Self::KnownHosts(keys) => crate::known_hosts::save(path, keys),
             Self::Settings(settings) => settings.save(path),
             Self::Labels(labels) => labels.save(path),
+            Self::AgentsShown(shown) => shown.save(path),
         }
     }
 }
diff --git a/src/lib.rs b/src/lib.rs
index 42223a4..dc0549d 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,5 +1,6 @@
 //! Tabletist: a native database client.
 
+pub mod agents_shown;
 pub mod app;
 pub mod backend;
 pub mod completion;
diff --git a/src/paths.rs b/src/paths.rs
index 2955008..c882b9f 100644
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -222,6 +222,10 @@ mod tests {
             dirs.agents_log_file(),
             PathBuf::from("/root/state/agents.sqlite3")
         );
+        assert_eq!(
+            dirs.agents_shown_file(),
+            PathBuf::from("/root/state/agents_shown.json")
+        );
         // Without a runtime directory, with the state.
         assert_eq!(dirs.agents_dir(), PathBuf::from("/root/state"));
         let dirs = AppDirs {
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib agents_shown::`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0432]: unresolved import `AgentAccess`
error[E0433]: cannot find type `Shown` in this scope
error[E0433]: cannot find type `Raised` in this scope
error[E0599]: no method named `agents_shown_file` found for struct `paths::AppDirs` in the current scope
error: could not compile `tabletist` (lib test) due to 29 previous errors; 1 warning emitted
```

- [ ] **Step 3: What was shown, and the comparison**

````diff
diff --git a/src/agents_shown.rs b/src/agents_shown.rs
index ba5ff49..c147ed8 100644
--- a/src/agents_shown.rs
+++ b/src/agents_shown.rs
@@ -8,6 +8,107 @@
 //! is not there, or cannot be read, shows nothing: whatever lets agents in
 //! is held then, once.
 
+use std::collections::BTreeMap;
+use std::path::Path;
+
+use serde::{Deserialize, Serialize};
+
+use crate::connections::{AgentAccess, ConnectionId, SavedConnection};
+
+/// What agents may do, as the person last saw it.
+#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(default)]
+pub struct Shown {
+    /// Whether the agent server was on.
+    pub enabled: bool,
+    /// Each connection's access, by its id. A connection not here was Off.
+    pub access: BTreeMap<String, AgentAccess>,
+}
+
+/// A change that lets agents do more than the person was shown.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum Raised {
+    /// The agent server was turned on.
+    Server,
+    /// A connection's access went up to `to`.
+    Access { id: ConnectionId, to: AgentAccess },
+}
+
+impl Shown {
+    pub fn load(path: &Path) -> Self {
+        crate::util::load_json(path)
+    }
+
+    pub fn save(&self, path: &Path) -> std::io::Result<()> {
+        crate::util::save_json(path, self)
+    }
+
+    /// The access `id` was shown at.
+    pub fn access_of(&self, id: &ConnectionId) -> AgentAccess {
+        self.access.get(&id.0).copied().unwrap_or_default()
+    }
+
+    /// What the files say now, `enabled` and `connections`, against what
+    /// was shown: the changes that let agents do more, to be held. What
+    /// lets them do less is taken into `self` at once; whether anything
+    /// was is the second part, so that it is saved.
+    pub fn compare(
+        &mut self,
+        enabled: bool,
+        connections: &[SavedConnection],
+    ) -> (Vec<Raised>, bool) {
+        let mut raised = Vec::new();
+        let mut lowered = false;
+        match (self.enabled, enabled) {
+            (false, true) => raised.push(Raised::Server),
+            (true, false) => {
+                self.enabled = false;
+                lowered = true;
+            }
+            _ => {}
+        }
+        for saved in connections {
+            let (was, now) = (self.access_of(&saved.id), saved.agents.access);
+            if now > was {
+                raised.push(Raised::Access {
+                    id: saved.id.clone(),
+                    to: now,
+                });
+            } else if now < was {
+                self.set(&saved.id, now);
+                lowered = true;
+            }
+        }
+        // A connection deleted since lets agents do nothing.
+        let before = self.access.len();
+        self.access
+            .retain(|id, _| connections.iter().any(|saved| saved.id.0 == *id));
+        lowered |= self.access.len() != before;
+        (raised, lowered)
+    }
+
+    /// Takes what was held as the person's: they kept it.
+    pub fn keep(&mut self, raised: &[Raised]) {
+        for raise in raised {
+            match raise {
+                Raised::Server => self.enabled = true,
+                Raised::Access { id, to } => self.set(id, *to),
+            }
+        }
+    }
+
+    fn set(&mut self, id: &ConnectionId, access: AgentAccess) {
+        match access {
+            AgentAccess::Off => {
+                self.access.remove(&id.0);
+            }
+            AgentAccess::Read => {
+                self.access.insert(id.0.clone(), access);
+            }
+        }
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/src/connections.rs b/src/connections.rs
index 99cc1ae..5305c86 100644
--- a/src/connections.rs
+++ b/src/connections.rs
@@ -36,8 +36,9 @@ pub enum PasswordMode {
     Ask,
 }
 
-/// What agents may do on a connection, through the MCP server.
-#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
+/// What agents may do on a connection, through the MCP server. In the
+/// order of what it lets them do, the least first.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
 #[serde(rename_all = "snake_case")]
 pub enum AgentAccess {
     /// Agents do not see the connection at all.
diff --git a/src/paths.rs b/src/paths.rs
index c882b9f..b2647b1 100644
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -153,6 +153,12 @@ impl AppDirs {
         dir
     }
 
+    /// What agents may do, as the app last showed it: what a change made
+    /// outside the app is held against (`agents_shown`).
+    pub fn agents_shown_file(&self) -> PathBuf {
+        self.state.join("agents_shown.json")
+    }
+
     pub fn log_file(&self) -> PathBuf {
         self.state.join("tabletist.log")
     }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib agents_shown::`, then `cargo test --locked --lib paths::`

Expected: PASS.

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 2474 filtered out
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 2472 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src
git commit -m "Remember what agents may do as the app last showed it"
```

---

### Task 2: The app holds what was changed outside it

- `App::check_agents`, at the start, at every settings change and at every save of the connections: what is raised goes to `agents_held`, what is lowered is saved as shown.
- `agents_on` and `agents_access` are what is in force. The policy sent to the backend, `serve_agents`, the start and stop in `apply_settings` (from what was in force before to what is after), and `agents_here` ask them.
- The app's own controls show what they turn on: `change_settings` turning the server on, and `turn_agents_on` (a held server kept by **Turn on**). The connection dialog has no control for the access: `save_dialog` keeps what the file says then, not what the dialog opened with.
- `KeepAgentChanges` and `PutBackAgentChanges` carry what the bar said, and answer only what is still held of it. Keep takes it as shown and puts it in force. Put back writes the settings, then the connections, as they were shown.
- The sessions the server reports are taken only while it is in force.
- Two tests on `main` change: the 3a test that turned the server on by an edit of the file now finds it held, and keeps it; and the one that raised the access in the dialog's form raises it in the file and keeps it. The tab's tests allow a connection as the person who keeps it.

**Files:**
- Modify: `src/app.rs`, `src/model.rs`, `src/testing.rs`, `src/ui/mod.rs`, `src/ui/agents_tests.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 196d751..a5c6ec4 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -5504,7 +5504,8 @@ mod tests {
         app.backend.sent.clear();
         app.change_settings(|settings| settings.mcp_log_days = 7);
         assert!(agents_sent(&app).is_empty());
-        // And an edit of the file turns it on as the app does.
+        // An edit of the file that turns it on is held: the app turned it
+        // off last, and that is what the person was shown.
         let text = Settings {
             mcp_enabled: true,
             ..app.settings.clone()
@@ -5512,9 +5513,233 @@ mod tests {
         .to_toml();
         app.apply_event(Event::SettingsFile { text, own: false });
         assert!(app.settings.mcp_enabled);
+        // Where agents cannot reach the app nothing is held, and nothing
+        // starts either.
+        assert!(!start(&app, 7));
+        assert_eq!(app.agents_on(), !crate::endpoint::AVAILABLE);
+        // Kept, it starts.
+        app.apply(Action::KeepAgentChanges(app.agents_held.clone()));
         assert_eq!(start(&app, 7), crate::endpoint::AVAILABLE);
     }
 
+    /// What the files say beside an app that serves agents: the server on,
+    /// and a connection agents may read, which nobody in the app turned on.
+    #[cfg(unix)]
+    fn changed_outside(app: &mut App) -> crate::connections::ConnectionId {
+        let mut saved = crate::testing::fixture_connection();
+        saved.name = "Shop".into();
+        saved.agents.access = crate::connections::AgentAccess::Read;
+        let id = saved.id.clone();
+        app.connections.upsert(saved);
+        app.settings.mcp_enabled = true;
+        app.check_agents();
+        id
+    }
+
+    #[cfg(unix)]
+    fn agents_shown_saved(app: &App) -> Option<crate::agents_shown::Shown> {
+        app.backend
+            .sent
+            .iter()
+            .rev()
+            .find_map(|command| match command {
+                Command::Save {
+                    file: StateFile::AgentsShown(shown),
+                    ..
+                } => Some(shown.clone()),
+                _ => None,
+            })
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn a_change_made_outside_the_app_is_held_until_the_person_keeps_it() {
+        use crate::agents_shown::Raised;
+        let (mut app, _dir) = app();
+        let id = changed_outside(&mut app);
+        assert_eq!(
+            app.agents_held,
+            [
+                Raised::Server,
+                Raised::Access {
+                    id: id.clone(),
+                    to: crate::connections::AgentAccess::Read
+                }
+            ]
+        );
+        // Held: the server is off, and the connection is not sent.
+        app.serve_agents();
+        assert!(matches!(
+            agents_sent(&app)[..],
+            [AgentsCommand::Policy { connections, .. }] if connections.is_empty()
+        ));
+        assert_eq!(
+            crate::ui::agents_held_said(&app).as_deref(),
+            Some(
+                "Agent access was changed outside Tabletist: the agent server was turned on, \
+                 and agents may read Shop · dev. Until you answer, agents may not. Keep it, or \
+                 put it back?"
+            )
+        );
+        // Kept: in force, and what the person was shown from now on.
+        app.backend.sent.clear();
+        app.apply(Action::KeepAgentChanges(app.agents_held.clone()));
+        assert!(app.agents_held.is_empty());
+        assert!(matches!(
+            agents_sent(&app)[..],
+            [AgentsCommand::Policy { connections, .. }, AgentsCommand::Start { .. }]
+                if connections.len() == 1
+        ));
+        let shown = agents_shown_saved(&app).expect("what was shown is saved");
+        assert!(shown.enabled);
+        assert_eq!(shown.access_of(&id), crate::connections::AgentAccess::Read);
+        // Taken back in the file: at once, and remembered. (What an edit
+        // of the file comes to: its event's way into the settings.)
+        app.backend.sent.clear();
+        app.apply_settings(Settings {
+            mcp_enabled: false,
+            ..app.settings.clone()
+        });
+        assert!(matches!(agents_sent(&app)[..], [AgentsCommand::Stop]));
+        assert_eq!(
+            agents_shown_saved(&app).map(|shown| shown.enabled),
+            Some(false)
+        );
+        assert!(app.agents_held.is_empty());
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn put_back_writes_what_the_person_was_shown_into_the_files() {
+        let (mut app, dir) = app();
+        let id = changed_outside(&mut app);
+        app.serve_agents();
+        app.backend.sent.clear();
+        app.apply(Action::PutBackAgentChanges(app.agents_held.clone()));
+        assert!(app.agents_held.is_empty());
+        assert!(!app.settings.mcp_enabled);
+        assert!(
+            settings_saves(&app)
+                .iter()
+                .any(|settings| !settings.mcp_enabled)
+        );
+        let path = AppDirs::at(dir.path()).connections_file();
+        match saved(&app, &path) {
+            Some(StateFile::Connections(stored)) => {
+                let shop = stored
+                    .connections
+                    .iter()
+                    .find(|saved| saved.id == id)
+                    .unwrap();
+                assert_eq!(shop.agents.access, crate::connections::AgentAccess::Off);
+            }
+            other => panic!("{other:?}"),
+        }
+        assert!(
+            !agents_sent(&app)
+                .iter()
+                .any(|sent| matches!(sent, AgentsCommand::Start { .. }))
+        );
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn what_the_app_itself_turns_on_is_never_held() {
+        let (mut app, _dir) = app();
+        app.serve_agents();
+        app.apply(Action::TurnAgentsOn);
+        assert!(app.agents_held.is_empty());
+        assert!(app.agents_on());
+        // Turn on, while the file's own turning on is held, keeps it.
+        let (mut app, _dir) = self::app();
+        changed_outside(&mut app);
+        app.apply(Action::TurnAgentsOn);
+        assert!(app.agents_on());
+        assert_eq!(app.agents_held.len(), 1, "the connection is still held");
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn keep_and_put_back_answer_only_what_the_bar_said() {
+        use crate::agents_shown::Raised;
+        use crate::connections::AgentAccess;
+        // The bar says the connection; the server's turning on comes in
+        // after it was drawn, before the click is applied.
+        let outside = |app: &mut App| {
+            let mut saved = crate::testing::fixture_connection();
+            saved.agents.access = AgentAccess::Read;
+            let id = saved.id.clone();
+            app.connections.upsert(saved);
+            app.check_agents();
+            let said = app.agents_held.clone();
+            app.settings.mcp_enabled = true;
+            app.check_agents();
+            assert_eq!(app.agents_held.len(), 2);
+            (id, said)
+        };
+        let (mut app, _dir) = app();
+        app.serve_agents();
+        let (id, said) = outside(&mut app);
+        app.backend.sent.clear();
+        app.apply(Action::KeepAgentChanges(said));
+        assert_eq!(app.agents_held, [Raised::Server]);
+        assert!(!app.agents_on());
+        assert!(
+            !agents_sent(&app)
+                .iter()
+                .any(|sent| matches!(sent, AgentsCommand::Start { .. }))
+        );
+        assert_eq!(
+            agents_shown_saved(&app).map(|shown| shown.enabled),
+            Some(false)
+        );
+        assert_eq!(
+            app.agents_access(app.connections.get(&id).unwrap()),
+            AgentAccess::Read
+        );
+        // Put back is as narrow: the server's turning on stays held, and
+        // stays in the file.
+        let (mut app, _dir) = self::app();
+        let (id, said) = outside(&mut app);
+        app.apply(Action::PutBackAgentChanges(said));
+        assert_eq!(app.agents_held, [Raised::Server]);
+        assert!(app.settings.mcp_enabled);
+        assert_eq!(
+            app.connections.get(&id).unwrap().agents.access,
+            AgentAccess::Off
+        );
+        // Nothing still held of it: nothing to do.
+        app.apply(Action::KeepAgentChanges(Vec::new()));
+        assert_eq!(app.agents_held, [Raised::Server]);
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn an_edit_of_something_else_keeps_nothing_that_is_held() {
+        use crate::connections::AgentAccess;
+        let (mut app, _dir) = app();
+        let id = changed_outside(&mut app);
+        app.apply(Action::EditConnection(id.clone()));
+        form(&mut app).name = "Shop renamed".into();
+        app.apply(Action::SaveConnection { connect: false });
+        assert!(
+            app.agents_held
+                .iter()
+                .any(|raise| matches!(raise, crate::agents_shown::Raised::Access { id: held, .. } if *held == id))
+        );
+        // Put back while the dialog is open: its save keeps what the file
+        // says then, not what the dialog opened with.
+        app.apply(Action::EditConnection(id.clone()));
+        app.apply(Action::PutBackAgentChanges(app.agents_held.clone()));
+        form(&mut app).name = "Shop again".into();
+        app.apply(Action::SaveConnection { connect: false });
+        let saved = app.connections.get(&id).unwrap();
+        assert_eq!(saved.name, "Shop again");
+        assert_eq!(saved.agents.access, AgentAccess::Off);
+        assert!(app.agents_held.is_empty());
+        assert_eq!(app.agents_access(saved), AgentAccess::Off);
+    }
+
     #[test]
     fn agents_are_told_of_each_change_to_what_they_may_use() {
         let (mut app, _dir) = app();
@@ -5533,12 +5758,19 @@ mod tests {
         };
         app.serve_agents();
         assert_eq!(policy(&app).0, Vec::<String>::new());
-        // A connection agents may read, saved in the dialog.
+        // A connection saved in the dialog: Off, which the dialog has no
+        // control to change.
         app.apply(Action::NewConnection);
         form(&mut app).name = "Disk".into();
         form(&mut app).sqlite_path = "/tmp/disk.db".into();
-        form(&mut app).agents.access = crate::connections::AgentAccess::Read;
         app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(policy(&app).0, Vec::<String>::new());
+        // Agents may read it: the file says so, and the person keeps it.
+        let mut saved = app.connections.connections[0].clone();
+        saved.agents.access = crate::connections::AgentAccess::Read;
+        app.connections.upsert(saved);
+        app.save_connections();
+        app.apply(Action::KeepAgentChanges(app.agents_held.clone()));
         assert_eq!(policy(&app).0, ["Disk · local"]);
         // A host key trusted.
         app.trust("bastion", 22, "SHA256:F1");
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index 0d26d04..f7a1ee0 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -22,12 +22,16 @@ fn connected(look: Look) -> (Harness, ConnTabId) {
     (harness, tab)
 }
 
-/// Sets what agents may do on `tab`'s saved connection.
+/// Sets what agents may do on `tab`'s saved connection, as the person
+/// who keeps it: a change of the files that is not held.
 fn allow(harness: &mut Harness, tab: ConnTabId, access: AgentAccess) {
     let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
     let mut saved = harness.app.connections.get(&conn).unwrap().clone();
     saved.agents.access = access;
     harness.app.connections.upsert(saved);
+    harness.app.check_agents();
+    let held = harness.app.agents_held.clone();
+    harness.app.apply(Action::KeepAgentChanges(held));
 }
 
 /// A session of `agent` whose calls have named `tab`'s connection, or
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- app::tests::a_change_made app::tests::put_back app::tests::what_the_app_itself app::tests::an_edit_of app::tests::keep_and_put_back`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0425]: cannot find function `agents_held_said` in module `crate::ui`
error[E0603]: function `fixture_connection` is private
error[E0599]: no method named `agents_on` found for struct `app::App` in the current scope
error[E0599]: no variant, associated function, or constant named `KeepAgentChanges` found for enum `model::Action` in the current scope
error[E0609]: no field `agents_held` on type `app::App`
error[E0599]: no method named `check_agents` found for mutable reference `&mut app::App` in the current scope
error[E0599]: no variant, associated function, or constant named `PutBackAgentChanges` found for enum `model::Action` in the current scope
error[E0599]: no method named `agents_access` found for struct `app::App` in the current scope
error: could not compile `tabletist` (lib test) due to 42 previous errors
```

- [ ] **Step 3: Hold, keep, put back**

````diff
diff --git a/src/app.rs b/src/app.rs
index a5c6ec4..374890a 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -138,6 +138,12 @@ pub struct App {
     serves_agents: bool,
     /// What the agent server said of its sessions last.
     pub agents: crate::model::AgentsState,
+    /// What agents may do, as the app last showed it (`agents_shown.json`).
+    agents_shown: crate::agents_shown::Shown,
+    /// What was changed outside the app and lets agents do more than that:
+    /// held, the narrower setting in force, until the person keeps it or
+    /// puts it back.
+    pub agents_held: Vec<crate::agents_shown::Raised>,
     next_id: u64,
 }
 
@@ -166,6 +172,7 @@ impl App {
             let spec = &saved.spec;
             Some(crate::labels::database_key(spec.driver, &spec.database))
         });
+        let agents_shown = crate::agents_shown::Shown::load(&dirs.agents_shown_file());
         let mut app = Self {
             dirs,
             settings,
@@ -200,6 +207,8 @@ impl App {
             closing: false,
             serves_agents: false,
             agents: crate::model::AgentsState::default(),
+            agents_shown,
+            agents_held: Vec::new(),
             next_id: 1,
         };
         let tab = app.picker_tab();
@@ -221,6 +230,9 @@ impl App {
                 file: StateFile::ConnectionsMoved,
             });
         }
+        // What the files let agents do against what was last shown: the
+        // connections are read now alone, the settings at every edit too.
+        app.check_agents();
         if adopted {
             app.save_labels();
         }
@@ -1398,7 +1410,9 @@ impl App {
             }
             Action::NewSqlTab(tab) => self.new_sql_tab(tab),
             Action::ShowAgents(tab) => self.show_agents(tab),
-            Action::TurnAgentsOn => self.change_settings(|settings| settings.mcp_enabled = true),
+            Action::TurnAgentsOn => self.turn_agents_on(),
+            Action::KeepAgentChanges(said) => self.keep_agent_changes(&said),
+            Action::PutBackAgentChanges(said) => self.put_back_agent_changes(&said),
             Action::RunSql { tab, sql_tab, all } => {
                 if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                     sql.completion = None;
@@ -2834,7 +2848,7 @@ impl App {
                 AgentsEvent::Stopped => self.agents.sessions.clear(),
                 AgentsEvent::Started { kept_aside: None } => {}
                 // What a server said as it stopped is not the server now.
-                AgentsEvent::Sessions(open) if self.settings.mcp_enabled => {
+                AgentsEvent::Sessions(open) if self.agents_on() => {
                     self.agents.sessions = open;
                 }
                 AgentsEvent::Sessions(_) => {}
@@ -3056,7 +3070,8 @@ impl App {
             file: StateFile::Connections(self.connections.clone()),
         });
         // Every change to a connection is saved: what agents may use
-        // changes with it.
+        // changes with it. One deleted is held no more.
+        self.check_agents();
         self.send_agents_policy();
     }
 
@@ -3066,11 +3081,143 @@ impl App {
     pub fn serve_agents(&mut self) {
         self.serves_agents = true;
         self.send_agents_policy();
-        if self.settings.mcp_enabled {
+        if self.agents_on() {
             self.start_agents();
         }
     }
 
+    /// Whether the agent server is on: the settings say so, and that is not
+    /// held.
+    pub fn agents_on(&self) -> bool {
+        use crate::agents_shown::Raised;
+        self.settings.mcp_enabled && !self.agents_held.contains(&Raised::Server)
+    }
+
+    /// What agents may do on `saved` now: its access, or what was shown
+    /// where a change made outside the app is held.
+    pub fn agents_access(&self, saved: &SavedConnection) -> crate::connections::AgentAccess {
+        use crate::agents_shown::Raised;
+        let held = self
+            .agents_held
+            .iter()
+            .any(|raise| matches!(raise, Raised::Access { id, .. } if *id == saved.id));
+        match held {
+            true => self.agents_shown.access_of(&saved.id),
+            false => saved.agents.access,
+        }
+    }
+
+    /// Compares what the files let agents do with what was last shown:
+    /// what lets them do more is held, what lets them do less is taken and
+    /// remembered. Where agents cannot reach the app nothing is held.
+    pub(crate) fn check_agents(&mut self) {
+        if !crate::endpoint::AVAILABLE {
+            return;
+        }
+        let enabled = self.settings.mcp_enabled;
+        let (raised, lowered) = self
+            .agents_shown
+            .compare(enabled, &self.connections.connections);
+        if !raised.is_empty() && raised != self.agents_held {
+            // Which connections, and to what, is the person's to read.
+            log::warn!(
+                "agent access was changed outside the app, and is held until the person answers"
+            );
+        }
+        self.agents_held = raised;
+        if lowered {
+            self.save_agents_shown();
+        }
+    }
+
+    /// Writes what agents may do, as the app shows it now.
+    fn save_agents_shown(&mut self) {
+        self.backend.send(Command::Save {
+            path: self.dirs.agents_shown_file(),
+            file: StateFile::AgentsShown(self.agents_shown.clone()),
+        });
+    }
+
+    /// What is still held of what the bar said, `said`: a change that came
+    /// after the bar was drawn is not the person's answer.
+    fn held_of(&self, said: &[crate::agents_shown::Raised]) -> Vec<crate::agents_shown::Raised> {
+        self.agents_held
+            .iter()
+            .filter(|raise| said.contains(raise))
+            .cloned()
+            .collect()
+    }
+
+    /// The person keeps what was changed outside the app, as the bar said
+    /// it: it is what they were shown from now on, and in force.
+    fn keep_agent_changes(&mut self, said: &[crate::agents_shown::Raised]) {
+        let kept = self.held_of(said);
+        if kept.is_empty() {
+            return;
+        }
+        let was_on = self.agents_on();
+        self.agents_shown.keep(&kept);
+        self.save_agents_shown();
+        self.check_agents();
+        self.agents_changed(was_on);
+    }
+
+    /// The person puts back what was changed outside the app, as the bar
+    /// said it: the files say again what they were shown, the settings
+    /// first, then the connections.
+    fn put_back_agent_changes(&mut self, said: &[crate::agents_shown::Raised]) {
+        use crate::agents_shown::Raised;
+        let back = self.held_of(said);
+        if back.contains(&Raised::Server) {
+            self.change_settings(|settings| settings.mcp_enabled = false);
+        }
+        let mut changed = false;
+        for raise in back {
+            let Raised::Access { id, .. } = raise else {
+                continue;
+            };
+            if let Some(mut saved) = self.connections.get(&id).cloned() {
+                saved.agents.access = self.agents_shown.access_of(&id);
+                self.connections.upsert(saved);
+                changed = true;
+            }
+        }
+        if changed {
+            self.save_connections();
+        }
+        self.check_agents();
+    }
+
+    /// Turns the server on from the Agents tab: the person's own doing.
+    /// Where the settings say it is on already and that is held, it is
+    /// kept.
+    fn turn_agents_on(&mut self) {
+        use crate::agents_shown::Raised;
+        if !self.settings.mcp_enabled {
+            self.change_settings(|settings| settings.mcp_enabled = true);
+            return;
+        }
+        let was_on = self.agents_on();
+        self.agents_shown.keep(&[Raised::Server]);
+        self.save_agents_shown();
+        self.check_agents();
+        self.agents_changed(was_on);
+    }
+
+    /// Tells the backend what agents may use, and starts or stops the
+    /// server, after what agents may do changed.
+    fn agents_changed(&mut self, was_on: bool) {
+        if !self.serves_agents {
+            return;
+        }
+        self.send_agents_policy();
+        match (was_on, self.agents_on()) {
+            (false, true) => self.start_agents(),
+            (true, false) => self.backend.send(Command::Agents(AgentsCommand::Stop)),
+            _ => {}
+        }
+    }
+
     /// Tells the backend which connections agents may use, and the SSH host
     /// keys the person trusts. Sent while the server is off too: it keeps
     /// them for when it starts.
@@ -3078,7 +3225,18 @@ impl App {
         if !self.serves_agents {
             return;
         }
-        let connections = crate::backend::agentable(&self.connections.connections);
+        // What a change made outside the app holds back is not sent.
+        let shown: Vec<SavedConnection> = self
+            .connections
+            .connections
+            .iter()
+            .map(|saved| {
+                let mut now = saved.clone();
+                now.agents.access = self.agents_access(saved);
+                now
+            })
+            .collect();
+        let connections = crate::backend::agentable(&shown);
         self.backend.send(Command::Agents(AgentsCommand::Policy {
             connections,
             host_keys: self.host_keys.clone(),
@@ -3151,6 +3309,11 @@ impl App {
             saved,
             ..file
         };
+        // The app's own controls show what they turn on: never held.
+        if !self.settings.mcp_enabled && settings.mcp_enabled {
+            self.agents_shown.enabled = true;
+            self.save_agents_shown();
+        }
         self.apply_settings(settings);
         self.save_settings();
     }
@@ -3159,6 +3322,7 @@ impl App {
     /// Every change comes through here, from the app or from the file, and
     /// nothing is written here: saving is [`App::change_settings`]'s.
     pub fn apply_settings(&mut self, new: Settings) {
+        let was_on = self.agents_on();
         let old = std::mem::replace(&mut self.settings, new);
         if old.timestamps != self.settings.timestamps {
             let full = self.settings.timestamps == crate::settings::Timestamps::Full;
@@ -3185,8 +3349,10 @@ impl App {
             // Reading a theme needs the window: `logic` has it.
             self.theme_changed = true;
         }
+        // An edit of the file that turns the server on is held.
+        self.check_agents();
         if self.serves_agents {
-            match (old.mcp_enabled, self.settings.mcp_enabled) {
+            match (was_on, self.agents_on()) {
                 (false, true) => self.start_agents(),
                 (true, false) => self.backend.send(Command::Agents(AgentsCommand::Stop)),
                 (true, true) if old.mcp_log_days != self.settings.mcp_log_days => {
@@ -3245,7 +3411,7 @@ impl App {
             form.message = Some("This connection was deleted.".into());
             return;
         }
-        let saved = match form.to_saved() {
+        let mut saved = match form.to_saved() {
             Ok(saved) => saved,
             Err(message) => {
                 form.message = Some(message);
@@ -3263,6 +3429,14 @@ impl App {
         let previous = self.connections.get(&saved.id).map(|c| c.password);
         let previous_ssh = self.connections.get(&saved.id).map(|c| c.ssh_secret);
         self.dialog = None;
+        // The dialog has no control for what agents may do: an edit keeps
+        // what the file says now, not what the dialog opened with. A change
+        // made outside the app stays held, and one put back stays back.
+        saved.agents = self
+            .connections
+            .get(&saved.id)
+            .map(|saved| saved.agents)
+            .unwrap_or_default();
         self.connections.upsert(saved.clone());
         self.save_connections();
         if saved.password == PasswordMode::Keyring {
@@ -3546,8 +3720,11 @@ impl App {
     pub fn agents_here(&self, tab: ConnTabId) -> Option<crate::model::AgentsHere> {
         let workspace = self.workspace(tab)?;
         let conn = &workspace.conn_id;
-        let access = self.connections.get(conn).map(|saved| saved.agents.access);
-        Some(self.agents.of(self.settings.mcp_enabled, access, conn))
+        let access = self
+            .connections
+            .get(conn)
+            .map(|saved| self.agents_access(saved));
+        Some(self.agents.of(self.agents_on(), access, conn))
     }
 
     /// Opens an empty SQL editor after the workspace's other tabs, with the
diff --git a/src/model.rs b/src/model.rs
index 5df23b3..04e88d6 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -616,6 +616,12 @@ pub enum Action {
     /// Turn the agent server on, from the Agents tab: a change of the
     /// settings, which starts it. No connection's access changes.
     TurnAgentsOn,
+    /// Keep what was changed outside the app and lets agents do more: what
+    /// the bar said, and only what is still held of it.
+    KeepAgentChanges(Vec<crate::agents_shown::Raised>),
+    /// Put back what was changed outside the app: the narrower setting, in
+    /// the files. What the bar said, as Keep.
+    PutBackAgentChanges(Vec<crate::agents_shown::Raised>),
     /// Run the statement at the editor's cursor, or every statement.
     RunSql {
         tab: ConnTabId,
@@ -949,7 +955,7 @@ pub struct ConnectionForm {
     /// Read-only as saved: `None` until the user sets it.
     pub read_only: Option<bool>,
     /// What agents may do on the connection, as it was saved. The dialog
-    /// has no control for it yet, so an edit keeps it.
+    /// has no control for it yet: saving takes what the file says then.
     pub agents: crate::connections::Agents,
     /// Put the keyboard in Name on the next frame (a new connection).
     pub focus_name: bool,
diff --git a/src/testing.rs b/src/testing.rs
index a096f1e..12d3d01 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -505,7 +505,7 @@ impl Harness {
 }
 
 /// The saved connection the fake session stands for.
-fn fixture_connection() -> SavedConnection {
+pub(crate) fn fixture_connection() -> SavedConnection {
     SavedConnection {
         id: ConnectionId::new(),
         name: "Fixture".into(),
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index b5768f8..0fb92a5 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -116,6 +116,47 @@ pub(crate) fn notice(app: &mut App, ui: &mut egui::Ui) -> f32 {
     shown.response.rect.height()
 }
 
+/// What the held bar says: what was changed outside the app, and the
+/// question. `None` with nothing held.
+pub fn agents_held_said(app: &App) -> Option<String> {
+    use crate::agents_shown::Raised;
+    if app.agents_held.is_empty() {
+        return None;
+    }
+    let locale = app.locale;
+    let saved = &app.connections.connections;
+    let names = crate::backend::agent_names(saved);
+    let parts: Vec<String> = app
+        .agents_held
+        .iter()
+        .map(|raise| match raise {
+            Raised::Server => {
+                crate::i18n::gettext(locale, "the agent server was turned on").into_owned()
+            }
+            Raised::Access { id, .. } => {
+                let name = saved
+                    .iter()
+                    .zip(&names)
+                    .find(|(saved, _)| saved.id == *id)
+                    .map_or(id.0.as_str(), |(_, name)| name.as_str());
+                crate::i18n::gettext(locale, "agents may read {name}").replace("{name}", name)
+            }
+        })
+        .collect();
+    let changes = match parts.as_slice() {
+        [one] => one.clone(),
+        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
+        [] => return None,
+    };
+    Some(
+        crate::i18n::gettext(
+            locale,
+            "Agent access was changed outside Tabletist: {changes}. Until you answer, agents may not. Keep it, or put it back?",
+        )
+        .replace("{changes}", &changes),
+    )
+}
+
 /// What a notice stands on: the warning colour, faint, laid on the
 /// window's. Solid, so that nothing under it shows through: at the top of
 /// a macOS window that is the dark that stands behind the whole window.
````

- [ ] **Step 4: Run the tests**

Run: the same, then `cargo test --locked --lib app::` and `cargo test --locked --lib ui::agents`

Expected: PASS.

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 2475 filtered out
test result: ok. 396 passed; 0 failed; 0 ignored; 0 measured; 2086 filtered out
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 2470 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src
git commit -m "Hold a change made outside the app that lets agents do more, until it is kept"
```

---

### Task 3: The bar, the tab, and the website

A bar of its own under the bar that leads the window, above the notice, in both looks: what changed, "Until you answer, agents may not", **Keep** and **Put back**, which carry what the bar said. No dismissal takes it away. `notices` draws it and the notice; the picker and the workspace call it where they called `notice`. The Agents tab's header says the access in force. The reference page says what is held, and names `agents_shown.json`.

**Files:**
- Modify: `src/ui/mod.rs`, `src/ui/picker.rs`, `src/ui/workspace.rs`, `src/ui/agents.rs`, `src/ui/agents_tests.rs`, `docs/_reference/settings-and-files.md`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index dbd70ba..4886654 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -91,6 +91,7 @@ On Linux, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` move the two folders.
 | `labels.json` | State | The column you chose to label each table's rows in a foreign key's list, by saved connection and, on PostgreSQL, by database |
 | `tabletist.log`, `panic.log` | State | Logs |
 | `agents.sqlite3` | State | Every call agents made: the tool, the statement and its values, and how it ended. Never a row |
+| `agents_shown.json` | State | What agents may do as Tabletist last showed it: whether the server is on, and each connection's access |
 | `mcp.sock`, `mcp.key`, `mcp.lock` | Runtime, else State | Where `tabletist mcp` reaches the app, the key it must send, and the lock the app holds while it listens. The first two are there while agents are on |
 
 The runtime folder is `/run/user/<your user id>/tabletist` on Linux, where
@@ -167,6 +168,22 @@ A file in which a connection has the table says `version = 4`, and one in
 which none has it stays `version = 3`. Agents use the table only while
 `[mcp] enabled` is `true` in `settings.toml`.
 
+### Agent access changed outside Tabletist
+
+A change to either file that lets agents do more than Tabletist last
+showed you is not applied at once: turning `[mcp] enabled` on, or a
+connection's `access` up from `off`. Tabletist says what changed in a bar
+under the top of the window, and until you answer, agents may do what they
+could before. **Keep** applies the change. **Put back** undoes it in the
+files: it turns `[mcp] enabled` off, or sets the connection's `access`
+back to `off`, and leaves your other settings and connections as they
+are. A change that lets agents do less applies at once.
+
+Turning agents on in Tabletist itself, with **Turn on** in a connection's
+Agents tab, is never held. Tabletist keeps what it showed you in
+`agents_shown.json`. Without that file, the first time a version with this
+check starts, whatever you turned on by hand is held once.
+
 Edit the file while Tabletist is closed. Tabletist writes the whole file
 again when you change a connection, so a comment you add is not kept.
 
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index f7a1ee0..4c277e1 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -374,6 +374,82 @@ fn more_sessions_than_the_tab_has_room_for_scroll_to_the_last() {
     assert!(seen(&harness));
 }
 
+/// What the files say, changed outside the app: the server on, and agents
+/// may read `tab`'s connection.
+#[cfg(unix)]
+fn changed_outside(harness: &mut Harness, tab: ConnTabId) {
+    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+    let mut saved = harness.app.connections.get(&conn).unwrap().clone();
+    saved.agents.access = AgentAccess::Read;
+    harness.app.connections.upsert(saved);
+    harness.app.settings.mcp_enabled = true;
+    harness.app.check_agents();
+    harness.settle();
+}
+
+#[cfg(unix)]
+#[test]
+fn the_held_bar_says_what_changed_and_its_buttons_answer_it() {
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        harness.settle();
+        let bar = harness.painted_rect("Fixture").expect("the connection bar");
+        changed_outside(&mut harness, tab);
+        let said = crate::ui::agents_held_said(&harness.app).unwrap();
+        assert!(said.contains("agents may read Fixture · dev"), "{said}");
+        assert!(harness.has(&said), "{}", look.name);
+        // Under the connection bar, which stays at the window's top.
+        assert_eq!(harness.painted_rect("Fixture"), Some(bar), "{}", look.name);
+        let shown = harness.painted_rect(&said).expect("the bar's words");
+        assert!(shown.top() >= bar.bottom(), "{}", look.name);
+        // Held, the entry says what is in force.
+        assert!(harness.has(&entry(&look, "Server off")), "{}", look.name);
+        harness.click("Keep");
+        harness.settle();
+        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
+        assert!(!harness.has(&said), "{}", look.name);
+        assert!(
+            harness.has(&entry(&look, "None connected")),
+            "{}",
+            look.name
+        );
+        // Changed again, and put back.
+        harness.app.settings.mcp_enabled = false;
+        harness.app.check_agents();
+        changed_outside(&mut harness, tab);
+        assert!(!harness.app.agents_held.is_empty(), "{}", look.name);
+        harness.click("Put back");
+        harness.settle();
+        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
+        assert!(!harness.app.settings.mcp_enabled, "{}", look.name);
+        assert!(harness.has(&entry(&look, "Server off")), "{}", look.name);
+    }
+}
+
+#[cfg(unix)]
+#[test]
+fn while_a_change_is_held_the_tab_says_what_is_in_force() {
+    use crate::agents_shown::Raised;
+    let (mut harness, tab) = connected(Look::macos());
+    changed_outside(&mut harness, tab);
+    // A report of sessions while the server's turning on is held is not
+    // taken: the server is off.
+    let here = session(&harness, Some(tab), "Claude Code");
+    said(&mut harness, AgentsEvent::Sessions(vec![here.clone()]));
+    assert!(harness.app.agents.sessions.is_empty());
+    // The server's turning on is kept, the connection's access is not.
+    harness
+        .app
+        .apply(Action::KeepAgentChanges(vec![Raised::Server]));
+    harness.app.apply(Action::ShowAgents(tab));
+    harness.settle();
+    assert!(harness.has(&entry(&Look::macos(), "Off here")));
+    assert!(harness.has("Agents on Fixture · dev · Off"));
+    assert!(!harness.has("Agents on Fixture · dev · Read"));
+    said(&mut harness, AgentsEvent::Sessions(vec![here]));
+    assert_eq!(harness.app.agents.sessions.len(), 1);
+}
+
 #[test]
 fn closing_the_tab_stops_nothing() {
     let (mut harness, tab) = connected(Look::macos());
````

- [ ] **Step 2: Run them to see that they fail**

Run: `cargo test --locked --lib -- ui::agents_tests::the_held_bar ui::agents_tests::while_a_change`

Expected: both FAIL. Nothing draws the bar yet, and the tab's header says the access in the file:

```text
test ui::agents_tests::the_held_bar_says_what_changed_and_its_buttons_answer_it ... FAILED
test ui::agents_tests::while_a_change_is_held_the_tab_says_what_is_in_force ... FAILED
thread 'ui::agents_tests::the_held_bar_says_what_changed_and_its_buttons_answer_it' panicked at src/ui/agents_tests.rs:400:9:
thread 'ui::agents_tests::while_a_change_is_held_the_tab_says_what_is_in_force' panicked at src/ui/agents_tests.rs:447:5:
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2482 filtered out
```

- [ ] **Step 3: The bar**

````diff
diff --git a/src/ui/agents.rs b/src/ui/agents.rs
index 76f8b03..c3a9701 100644
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -47,8 +47,12 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .get(&conn)
         .map_or(workspace.environment, |saved| saved.environment);
     let runs_as = environment.named(crate::env::Platform::Native);
-    // What the connection allows, whether the server runs or not.
-    let allowed = app.connections.get(&conn).map(|saved| saved.agents.access);
+    // What the connection allows, whether the server runs or not: what is
+    // in force, where a change made outside the app is held.
+    let allowed = app
+        .connections
+        .get(&conn)
+        .map(|saved| app.agents_access(saved));
     let access = look.label(&match allowed {
         Some(crate::connections::AgentAccess::Read) => gettext(locale, "Read"),
         Some(crate::connections::AgentAccess::Off) | None => gettext(locale, "Off"),
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 0fb92a5..0f810a5 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -93,12 +93,18 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
     toast::show(app, ui.ctx());
 }
 
+/// What the screen draws under the bar that leads the window, which on
+/// macOS is the title bar, so that the window's own buttons keep their
+/// line: what was changed outside the app and is held, then the notice.
+/// Returns their height.
+pub(crate) fn notices(app: &mut App, ui: &mut egui::Ui) -> f32 {
+    agents_held(app, ui) + notice(app, ui)
+}
+
 /// A problem worth the user's attention that belongs to no one tab (a
-/// keyring write that failed), until dismissed. The screen draws it under
-/// the bar that leads the window, which on macOS is the title bar: the
-/// window's own buttons keep their line. Returns its height, nothing when
-/// there is no notice.
-pub(crate) fn notice(app: &mut App, ui: &mut egui::Ui) -> f32 {
+/// keyring write that failed), until dismissed. Returns its height,
+/// nothing when there is no notice.
+fn notice(app: &mut App, ui: &mut egui::Ui) -> f32 {
     let Some(message) = app.notice.clone() else {
         return 0.0;
     };
@@ -116,6 +122,56 @@ pub(crate) fn notice(app: &mut App, ui: &mut egui::Ui) -> f32 {
     shown.response.rect.height()
 }
 
+/// What was changed outside the app and lets agents do more, held until
+/// the person keeps it or puts it back: a bar of its own, in both looks,
+/// which no dismissal takes away. Returns its height, nothing when nothing
+/// is held.
+fn agents_held(app: &mut App, ui: &mut egui::Ui) -> f32 {
+    let Some(said) = agents_held_said(app) else {
+        return 0.0;
+    };
+    let (look, palette, locale) = (app.look, app.palette, app.locale);
+    // The buttons answer what the bar says, not what a later change holds.
+    let held = app.agents_held.clone();
+    let mut actions = Vec::new();
+    let shown = egui::Panel::top(egui::Id::new("agents-held"))
+        .resizable(false)
+        .show_separator_line(false)
+        .frame(
+            Frame::new()
+                .fill(notice_fill(&palette))
+                .inner_margin(egui::Margin::symmetric(12, 8)),
+        )
+        .show(ui, |ui| {
+            ui.horizontal(|ui| {
+                let (keep, back) = (
+                    crate::i18n::gettext(locale, "Keep"),
+                    crate::i18n::gettext(locale, "Put back"),
+                );
+                // The buttons' room, then the words wrap in the rest.
+                let pad = 2.0 * ui.spacing().button_padding.x;
+                let width =
+                    |text: &str| widgets::body(&look).width(ui.ctx(), look.faces, text) + pad;
+                let buttons = width(&keep) + width(&back) + 24.0;
+                let room = (ui.available_width() - buttons).max(0.0);
+                crate::typography::Text::one(&look, widgets::body(&look), &said, palette.text)
+                    .wrap(room)
+                    .layout(ui.ctx())
+                    .label(ui);
+                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
+                    if widgets::button(ui, &keep, &look).clicked() {
+                        actions.push(Action::KeepAgentChanges(held.clone()));
+                    }
+                    if widgets::button(ui, &back, &look).clicked() {
+                        actions.push(Action::PutBackAgentChanges(held.clone()));
+                    }
+                });
+            });
+        });
+    app.actions.extend(actions);
+    shown.response.rect.height()
+}
+
 /// What the held bar says: what was changed outside the app, and the
 /// question. `None` with nothing held.
 pub fn agents_held_said(app: &App) -> Option<String> {
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
index 9fc896a..c100230 100644
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -190,12 +190,12 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
     ui.painter()
         .rect_filled(header, CornerRadius::ZERO, header_fill);
     widgets::hline(ui, header.x_range(), header.bottom() - 0.5, palette.outline);
-    // The notice, under the header: the window's own buttons keep its line.
+    // The notices, under the header: the window's own buttons keep its line.
     let mut under = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(
         pos2(full.left(), header.bottom()),
         full.max,
     )));
-    let below = header.bottom() + super::notice(app, &mut under);
+    let below = header.bottom() + super::notices(app, &mut under);
     let y = header.top() + header_line(&look);
     let mut x = header.left() + side.max(inset + 12.0);
     let title = if look.terminal {
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index d444069..dc15a4c 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -47,7 +47,7 @@ pub fn bar_line(app: &App, zoom: f32) -> f32 {
 pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let look = app.look;
     top_bar(app, ui, tab);
-    super::notice(app, ui);
+    super::notices(app, ui);
     let Some(workspace) = app.workspace(tab) else {
         return;
     };
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib ui::agents_tests`, then `cargo test --locked --lib ui::tests::a_notice` and `docs/bin/check`

Expected: PASS, and the website's check ends `HTML-Proofer finished successfully.`

```text
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 2471 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 2481 filtered out
```

- [ ] **Step 5: Look at it.** Render the bar in both looks, by eye, in a throwaway scene in `src/shots.rs`; remove it before the commit. No picture is committed.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src docs/_reference/settings-and-files.md
git commit -m "Say what was changed outside the app, and let the person keep it or put it back"
```

---

### Task 4: The master plan says what was built

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: Correct it**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index 6734842..8c41cb2 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -51,6 +51,10 @@ from the fourth run, and the check of changes made outside the app (C.2)
 is the second part. What the run draws otherwise than the boards is in
 D.4.
 
+**Built on 2026-10-14: the second part of Phase 1b's third run** (`2026-10-14-mcp-server-1b-3b-changes-outside.md`):
+C.2's check of the changes made outside the app. What it settled where C.2
+left it open is there.
+
 Each phase below gets its own plan under `docs/superpowers/plans/` before any
 code is written, as every batch of work in this repository does.
 
@@ -1392,6 +1396,40 @@ can edit both files can turn agents on for itself, as E says of a shell.
 **Moved again on 2026-10-14**: the third run is cut in two, and the check is
 its second part (1b-3b), right after **Turn on** (`2026-10-14-mcp-server-1b-3a-agents-tab.md`).
 
+**As built** (`2026-10-14-mcp-server-1b-3b-changes-outside.md`). What it settled where the above left it open:
+
+- What the app last showed is `agents_shown.json` in the state directory:
+  whether the server is on, and each connection's access. `listen` and the
+  lists come with what they are for (1d, 1c, 3a).
+- **A file that is not there, or cannot be read, shows nothing.** So the
+  first start of the version that checks holds whatever was turned on by
+  hand, once, and a person who set agents up through the files keeps it
+  with one click. Taking the files as shown at that start would let a
+  program that removes the file turn agents on unseen.
+- The settings are compared at every edit of the file, the connections at
+  every start (the file is read then alone) and at every save.
+- The app's own controls are what changes the settings through the app
+  (`change_settings`: **Turn on**, later Settings › Agents). The
+  connection dialog has no control for a connection's access yet, so its
+  save keeps what the file says then: a raise made outside stays held,
+  and one put back stays back. The controls that raise it come with 1b-5
+  (`Action::SetAgentAccess`, the editor's Agents section): each marks what
+  it raises as shown, as `change_settings` does for the server.
+- The notice is a bar of its own under the bar that leads the window
+  (on macOS the title bar, so the window's own buttons keep their line),
+  in both looks, which no dismissal takes away: what changed, "Until you
+  answer, agents may not", **Keep** and **Put back**. Both answer what the
+  bar said: a change that comes in after it was drawn waits for its own
+  answer. **Put back** turns the server off or sets the access back in the
+  files, and leaves the rest of them. While a change is held, the Agents
+  entry and tab say what is in force (Server off, or Off here), and the
+  server's sessions are not taken.
+- Where agents cannot reach the app (Windows) nothing is held.
+- Only the server and each connection's access are compared. A Read
+  connection pointed at another server by hand is not held: an agent
+  with the person's shell gains little by it, and the settings and
+  connection files are the person's to edit.
+
 ### C.3 Hidden tables and masked columns when the schema changes
 
 Entries are names: `customers`, `sales.customers`, `orders.email`. A schema
@@ -2067,7 +2105,7 @@ can see what agents do and stop them.
 | **1b-2a. The backend's gateway** (built: `2026-10-14-mcp-server-1b-2a-gateway.md`) | `[connections.agents]` with access, rows and timeout, which makes the connections file version 4. `Command::Agents` and `Event::Agents`, the backend's gateway with its own sessions on each database, and the server started and stopped by the backend, with the log's retention. The SDK held out of the app's log | Nothing: no setting starts the server, and no transport reaches it | The connections file, the gateway on SQLite, PostgreSQL and MySQL, the backend's start and stop, the log's filter |
 | **1b-2b. Turning it on from the files, and stdio** (built: `2026-10-14-mcp-server-1b-2b-stdio.md`) | The `[mcp]` keys in `settings.toml`, and the app that starts and stops the server with them and sends the backend what agents may use. The endpoint, the key and the relay, `tabletist mcp`, on Linux and macOS. The keychain on macOS (A.1). The website's reference for the files | With `enabled = true` and an access written by hand, an agent connects over stdio and reads. Nothing in the app shows it yet, and nothing in the app turns it on | F.1 over stdio, F.5 over stdio, the settings file, `tests/cli.rs` |
 | **1b-3a. The Agents tab's frame** (built: `2026-10-14-mcp-server-1b-3a-agents-tab.md`) | The third kind of tab. The sidebar entry with its states but Waiting. The ways in: the key, `ga`, `:agents`, quick open. The three empty states, with **Turn on** and **Copy**. The sessions the app learns of, and those that are here. The keymap and the help | Open the tab in any state, see which state the connection is in and who is here, turn the server on from the tab | F.7: the entry's states, the ways in, one tab for a window, the empty states, nothing opening or focusing by itself |
-| **1b-3b. Changes made outside the app** | C.2: the agent settings the app last showed, kept in the state directory, and a change from outside that makes agents more able held back until the person keeps it | A program that edits the files cannot turn agents on unseen | C.2's cases |
+| **1b-3b. Changes made outside the app** (built: `2026-10-14-mcp-server-1b-3b-changes-outside.md`) | C.2: the agent settings the app last showed, kept in the state directory, and a change from outside that makes agents more able held back until the person keeps it | A program that edits the files cannot turn agents on unseen | C.2's cases |
 | **1b-4. What agents are doing** | `App::agents`. The sessions that are here and "Also:". The feed. **Stop session** and **Stop agents here**. The Log view as a list for this connection, with All connections. The notification for an agent that connects | See each agent and each call as it happens, stop one agent or all on a connection, read the log | F.7: one connection, the stops, closing the tab, the log's default. F.4's interface |
 | **1b-5. Letting agents in** | Settings › Agents, with the switch between panes. The Connections list's Agents column, its menu and Omarchy's box. The connection editor's Agents section, with **Allow agents…** and **Show Agents tab** pointing at them. The website: a new guide, and the reference pages | Everything Phase 1 draws, without a file edited by hand | The interface tests of Settings, Connections and the editor. The keymap. `docs/bin/check` |
 
````

- [ ] **Step 2: Check it**

Run: `grep -c "$(printf '\342\200\224')" docs/superpowers/plans/2026-10-13-mcp-server.md` (an em dash, written in octal so that this plan holds none)

Expected: `0`.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what the second part of its third run built"
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

Expected: all clean, and every test passes. On 2026-10-14, with both servers up: 23 test binaries, 3,425 passed, none failed, 22 ignored. The website's check ends `HTML-Proofer finished successfully.`

- [ ] **By hand, on Linux**: with Tabletist open, set `enabled = true` under `[mcp]` in `settings.toml` from a terminal. The bar, under the connection bar, says the server was turned on, the Agents entry says Server off, and `tabletist mcp` answers that Tabletist is not running. **Keep** starts it. Close the app, set a connection's `access = "read"`, open it again: the bar says so, and **Put back** sets it to `off` in the file.

- [ ] **Open the pull request.**

## Self-review

- **Against the master plan's C.2, as moved to this run.** What lets agents do more is held and what lets them do less applies at once (Task 1's `compare`, Task 2's `check_agents`). The comparison is with what the app last showed, kept in the state directory (Task 1). Keep and Put back answer it (Tasks 2 and 3). The app's own **Turn on** is never held (Task 2). The bar says what changed (Task 3). What C.2 lists and this run leaves out is in "What this run leaves out", each with the run it comes with.
- **Against #132.** The bar stands where the notice now stands, under the bar that leads the window, and `notices` draws both, so the window's own buttons keep their line on macOS (Task 3, with a test that the connection bar does not move).
- **Reviewed before it was written.** A reviewer was given the draft and asked to break it, with tests of its own. It found these, each fixed above with a test: Keep could keep a server turned on after the bar was drawn, and before the click was applied (the actions carry what the bar said); the dialog's save could keep a raise that was put back while it was open (the save keeps what the file says); the tab's header and the dialog said the access in the file, not the one in force; and the server's sessions were taken while its turning on was held. Its note that only the server and the access are compared is in "What this run leaves out".
- **Names.** `Shown`, `Raised`, `compare`, `keep`, `access_of`, `agents_shown_file`, `StateFile::AgentsShown`, `agents_shown`, `agents_held`, `check_agents`, `agents_on`, `agents_access`, `held_of`, `keep_agent_changes`, `put_back_agent_changes`, `KeepAgentChanges`, `PutBackAgentChanges`, `agents_held_said`, `notices`: each is used under the one name in every task.
- **Every commit stands.** After Task 1 the app can remember what it showed, and nothing asks. After Task 2 a change made outside is held and nothing says so but the Agents entry, which says what is in force. After Task 3 the bar says it and answers it. Task 4 is the master plan's text.
- **No placeholders.** Every step has its code or its command, and what the command prints.
