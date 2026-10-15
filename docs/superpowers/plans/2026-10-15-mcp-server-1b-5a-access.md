# MCP server, Phase 1b, run 5a: letting agents in, per connection. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A connection is opened to agents, and closed, without a file edited by hand. The Connections list says what agents may do on each connection and its menu changes it (Omarchy: `a` and a small box); the connection editor has an Agents section with the access, the rows per answer and the statement timeout; the Agents tab's Off state has **Allow agents…**, which opens that section with Read chosen.

**Architecture:** `Action::SetAgentAccess { conn, access }` is the list's and the box's: it saves through `save_connections`, and a raise above what is in force is the person's own, kept as shown, never held (1b-3b). The editor opens with the access in force (`ConnectionForm::agents_opened`); on Save an access the person changed is theirs, and one left as it opened keeps what the file says, so a change made outside the app stays held through an edit of something else. `Action::EditAgents { conn, allow }` opens the editor at its Agents section (`show_agents`), with Read chosen for **Allow agents…**. The terminal look's box is `Dialog::AgentAccess`, drawn in `src/ui/agent_access.rs`.

**Tech Stack:** Rust 2024, egui. Nothing new is built.

---

## Where this sits

Phase 1b of `docs/superpowers/plans/2026-10-13-mcp-server.md` is cut into runs (section G). Eight are merged, through the fourth run's log and notification.

1. Done: the server, the gateway, the files and stdio, the Agents tab, changes made outside, what agents are doing, the log.
2. **This run, 5a: letting agents in, per connection.** The fifth run is cut in two (agreed on 2026-10-15): this part, then 5b, Settings › Agents with its pane switch, and the website's guide to Agents.
3. Then Windows' way in.

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: views draw and push `Action`s and `App::apply` changes state; a key is spelled in the keymap and nowhere else (`tests/keys.rs`).
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.
- No `Cargo.toml` or `Cargo.lock` changes. Nothing needs PostgreSQL or MySQL; the full run at the end prints "skipped" for their tests without the variables.

## What was run before this plan was written

Every task's state was built, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` (3,466 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. The list with its menu open, Omarchy's box, and the editor's Agents section were rendered headless in both looks and looked at; the scene was thrown away. Not run: macOS and Windows, which CI runs; on Windows nothing of agents shows.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| What the list and the box set | `Action::SetAgentAccess`, saved through `save_connections`: a raise above what is in force is the person's own, kept as shown and never held; the person's Read keeps a raise the file made and was held; a lowering applies at once | 1b-3b: the controls that raise come with 1b-5 and mark what they raise as shown |
| What the editor shows and saves | It opens with the access in force (`agents_opened`). On Save, an access the person changed is theirs, a raise shown; one left as it opened keeps what the file says, so a change made outside stays held through an edit of something else | The rule 1b-3b's review settled for the dialog, now that it has the control |
| Rows and timeout | Typed fields, checked on Save: 1 to 5000 rows, 1 to 60 seconds, as the file's limits are. A wrong one brings the section into sight with its message | `MAX_AGENT_ROWS`, `MAX_AGENT_TIMEOUT_SECS` (1b-2a) |
| The menu's Agents part | `src/ui/menu.rs` nests no menu: the "…" and right-click menus end with an Agents part under a heading (`menu::heading`): Off and Read, Tables, columns and limits…, Show Agents tab | D.2's fallback, "the three levels go in the menu itself under a heading" |
| The levels | Off and Read. The third, and the disabled levels with their reasons, come with writes | Phase 2 |
| The column on a narrow window | 140, out of the room and the host while the host keeps 160; narrower, the column steps back and the menu has the Agents part | The board is 1440 wide. Found by the review: at 1024 "Last used" ran under Edit |
| Omarchy's box | `Dialog::AgentAccess`: `j`/`k`, a plain Enter sets, `t` the Agents tab, Esc. The list connects on a press of Enter, not on what a held one repeats | Found by the review: a held Enter closed the box and connected the row behind it |
| Allow agents… | The Off state's button and `e`: the editor at its Agents section, Read chosen, nothing until Save. Not for a connection saved no more | D.4. The level Stop agents here took away is Read: the only one there is |
| Opening at the Agents section | `show_agents`: the whole section is scrolled into sight on the frames that are seen, until it is | Found by the review: in a short window only its heading came into sight |

### What this run leaves out

- Settings › Agents and its pane switch, **Show Agents tab** and **Show log** there, the Agents tab's **Other agents…** and **Settings…**, and the website's guide to Agents: the fifth run's second part.
- "Read + approved writes", "Writes still wait for your approval each time." and the disabled levels with their reasons: writes, Phase 2.
- Tables and Masked columns in the editor: Phases 1c and 3a.
- The count of writes waiting in the list's column (question 25): Phase 2.

## File structure

| File | What changes |
|---|---|
| `src/model.rs` | `ConnectionForm`'s `agents_opened`, `agents_rows`, `agents_timeout`, `show_agents`, and their check; `SetAgentAccess`, `EditAgents`, `OpenAgentAccess`, `MoveAgentAccess`; `Dialog::AgentAccess` |
| `src/app.rs` | The actions; the editor opened with the access in force; Save's rule for the access |
| `src/ui/picker.rs`, `src/ui/menu.rs` | The Agents column, the menus' Agents part (`menu::heading`), Omarchy's line and footer |
| `src/ui/agent_access.rs` | New: Omarchy's box |
| `src/ui/connect_dialog/agents.rs`, `mod.rs`, `sheet.rs`, `terminal.rs` | New: the editor's Agents section, in both looks |
| `src/ui/agents.rs` | **Allow agents…** in the Off state |
| `src/keymap.rs`, `src/ui/keys.rs`, `src/ui/workspace.rs`, `src/ui/mod.rs` | `a`, the box's keys, `e`; the hints |
| `src/ui/agents_tests.rs` | The list, the box, the editor, Allow agents… |
| `docs/_reference/connections.md`, `docs/_guide/omarchy.md` | The Agents section, and the keys |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | What this run built, and the cut of the fifth run |

---

### Task 1: The person sets what agents may do

- `Action::SetAgentAccess { conn, access }`: the connection's access, saved through `save_connections`, which sends the backend the policy. A raise above what is in force is kept as shown (`agents_shown`), never held; the person's own Read keeps a raise the file made and was held. A lowering applies at once.
- The editor: `ConnectionForm` opens with the access in force (`agents_opened`, `agents.access`), and has `agents_rows` and `agents_timeout` as typed, checked on Save ("Rows per answer is a number from 1 to 5000.", "Statement timeout is a number of seconds from 1 to 60."). On Save, an access the person changed is theirs, and a raise of it is shown; an access left as it opened keeps what the file says.
- `Action::EditAgents { conn, allow }` opens the editor with `show_agents` set, and with Read chosen where `allow`. `Action::OpenAgentAccess` and `MoveAgentAccess` are the terminal look's box (`Dialog::AgentAccess`), drawn in Task 2.

**Files:**
- Modify: `src/model.rs`, `src/app.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index e209828..0c77984 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -6606,6 +6606,131 @@ mod tests {
         assert!(app.workspace(app.active_tab_id()).is_none());
     }
 
+    #[cfg(unix)]
+    #[test]
+    fn the_lists_access_is_the_persons_own_raised_or_lowered() {
+        use crate::connections::AgentAccess;
+        let (mut app, dir) = app();
+        let conn = with_saved(&mut app);
+        app.serve_agents();
+        app.backend.sent.clear();
+        app.apply(Action::SetAgentAccess {
+            conn: conn.clone(),
+            access: AgentAccess::Read,
+        });
+        // Saved, shown, never held, and told to the backend.
+        assert_eq!(
+            app.connections.get(&conn).unwrap().agents.access,
+            AgentAccess::Read
+        );
+        assert!(app.agents_held.is_empty());
+        assert_eq!(
+            agents_shown_saved(&app).map(|shown| shown.access_of(&conn)),
+            Some(AgentAccess::Read)
+        );
+        let path = AppDirs::at(dir.path()).connections_file();
+        assert!(matches!(
+            saved(&app, &path),
+            Some(StateFile::Connections(stored))
+                if stored.connections.iter().any(|saved| saved.agents.access == AgentAccess::Read)
+        ));
+        assert!(matches!(
+            agents_sent(&app).last(),
+            Some(AgentsCommand::Policy { connections, .. }) if connections.len() == 1
+        ));
+        // Lowered: at once.
+        app.apply(Action::SetAgentAccess {
+            conn: conn.clone(),
+            access: AgentAccess::Off,
+        });
+        assert_eq!(
+            app.connections.get(&conn).unwrap().agents.access,
+            AgentAccess::Off
+        );
+        assert!(matches!(
+            agents_sent(&app).last(),
+            Some(AgentsCommand::Policy { connections, .. }) if connections.is_empty()
+        ));
+        // The file raised it, held: the list's Read keeps it.
+        let mut raised = app.connections.get(&conn).unwrap().clone();
+        raised.agents.access = AgentAccess::Read;
+        app.connections.upsert(raised);
+        app.check_agents();
+        assert_eq!(app.agents_held.len(), 1);
+        app.apply(Action::SetAgentAccess {
+            conn: conn.clone(),
+            access: AgentAccess::Read,
+        });
+        assert!(app.agents_held.is_empty());
+        assert!(matches!(
+            agents_sent(&app).last(),
+            Some(AgentsCommand::Policy { connections, .. }) if connections.len() == 1
+        ));
+    }
+
+    #[cfg(unix)]
+    #[test]
+    fn the_editors_access_is_the_persons_own_and_allow_agents_chooses_read() {
+        use crate::connections::AgentAccess;
+        let (mut app, _dir) = app();
+        let conn = with_saved(&mut app);
+        // Allow agents…: the editor at its Agents part, Read chosen, and
+        // nothing saved until Save.
+        app.apply(Action::EditAgents {
+            conn: conn.clone(),
+            allow: true,
+        });
+        assert!(form(&mut app).show_agents);
+        assert_eq!(form(&mut app).agents.access, AgentAccess::Read);
+        assert_eq!(
+            app.connections.get(&conn).unwrap().agents.access,
+            AgentAccess::Off
+        );
+        app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(
+            app.connections.get(&conn).unwrap().agents.access,
+            AgentAccess::Read
+        );
+        assert!(
+            app.agents_held.is_empty(),
+            "the person's own raise is not held"
+        );
+        // Rows and the timeout are checked, and saved.
+        app.apply(Action::EditConnection(conn.clone()));
+        form(&mut app).agents_rows = "0".into();
+        app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(
+            form(&mut app).message.as_deref(),
+            Some("Rows per answer is a number from 1 to 5000.")
+        );
+        form(&mut app).agents_rows = "250".into();
+        form(&mut app).agents_timeout = "61".into();
+        app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(
+            form(&mut app).message.as_deref(),
+            Some("Statement timeout is a number of seconds from 1 to 60.")
+        );
+        form(&mut app).agents_timeout = "30".into();
+        app.apply(Action::SaveConnection { connect: false });
+        let agents = app.connections.get(&conn).unwrap().agents;
+        assert_eq!((agents.rows, agents.timeout_secs), (250, 30));
+        // A new connection given Read in the dialog is shown, not held.
+        app.apply(Action::NewConnection);
+        form(&mut app).name = "Disk".into();
+        form(&mut app).sqlite_path = "/tmp/disk.db".into();
+        form(&mut app).agents.access = AgentAccess::Read;
+        app.apply(Action::SaveConnection { connect: false });
+        assert!(app.agents_held.is_empty());
+        assert_eq!(
+            app.connections
+                .connections
+                .iter()
+                .filter(|saved| saved.agents.access == AgentAccess::Read)
+                .count(),
+            2
+        );
+    }
+
     #[test]
     fn agents_are_told_of_each_change_to_what_they_may_use() {
         let (mut app, _dir) = app();
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- app::tests::the_lists_access app::tests::the_editors_access`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0599]: no variant named `SetAgentAccess` found for enum `model::Action`
error[E0599]: no variant named `EditAgents` found for enum `model::Action`
error[E0609]: no field `show_agents` on type `&mut model::ConnectionForm`
error[E0609]: no field `agents_rows` on type `&mut model::ConnectionForm`
error[E0609]: no field `agents_timeout` on type `&mut model::ConnectionForm`
error: could not compile `tabletist` (lib test) due to 9 previous errors
```

- [ ] **Step 3: The actions, and the editor's fields**

````diff
diff --git a/src/app.rs b/src/app.rs
index 0c77984..9dfb61f 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1901,12 +1901,54 @@ impl App {
             }
             Action::EditConnection(id) => {
                 if let Some(saved) = self.connections.get(&id) {
-                    self.dialog = Some(Dialog::Connection(Box::new(ConnectionForm::from_saved(
-                        saved,
-                    ))));
+                    let mut form = ConnectionForm::from_saved(saved);
+                    // What is in force, where a change made outside the
+                    // app is held: what the person was shown.
+                    form.agents.access = self.agents_access(saved);
+                    form.agents_opened = form.agents.access;
+                    self.dialog = Some(Dialog::Connection(Box::new(form)));
                     self.list_ssh_hosts();
                 }
             }
+            Action::EditAgents { conn, allow } => {
+                self.apply(Action::EditConnection(conn));
+                if let Some(Dialog::Connection(form)) = &mut self.dialog {
+                    form.show_agents = true;
+                    if allow {
+                        form.agents.access = crate::connections::AgentAccess::Read;
+                    }
+                }
+            }
+            Action::SetAgentAccess { conn, access } => self.set_agent_access(&conn, access),
+            Action::OpenAgentAccess(conn) => {
+                let saved = &self.connections.connections;
+                let names = crate::backend::agent_names(saved);
+                let found = saved.iter().zip(&names).find(|(saved, _)| saved.id == conn);
+                if crate::endpoint::AVAILABLE
+                    && let Some((saved, name)) = found
+                {
+                    let chosen = self.agents_access(saved);
+                    self.dialog = Some(Dialog::AgentAccess(Box::new(
+                        crate::model::AgentAccessBox {
+                            conn,
+                            name: name.clone(),
+                            chosen,
+                        },
+                    )));
+                }
+            }
+            Action::MoveAgentAccess(step) => {
+                use crate::connections::AgentAccess;
+                if let Some(Dialog::AgentAccess(asked)) = &mut self.dialog {
+                    let levels = [AgentAccess::Off, AgentAccess::Read];
+                    let at = levels.iter().position(|level| *level == asked.chosen);
+                    let to = at
+                        .unwrap_or_default()
+                        .saturating_add_signed(step)
+                        .min(levels.len() - 1);
+                    asked.chosen = levels[to];
+                }
+            }
             Action::DuplicateConnection(id) => {
                 if self.connections.duplicate(&id).is_some() {
                     self.save_connections();
@@ -3489,6 +3531,7 @@ impl App {
         let Some(Dialog::Connection(form)) = &mut self.dialog else {
             return;
         };
+        let opened = form.agents_opened;
         if !use_typed_url(form) {
             return;
         }
@@ -3504,8 +3547,10 @@ impl App {
             Ok(saved) => saved,
             Err(message) => {
                 form.message = Some(message);
-                // The message names fields: show them.
+                // The message names fields: show them, and the agents'
+                // part where it is one of its own.
                 form.url_mode = false;
+                form.show_agents |= form.agents_to_saved().is_err();
                 return;
             }
         };
@@ -3518,14 +3563,26 @@ impl App {
         let previous = self.connections.get(&saved.id).map(|c| c.password);
         let previous_ssh = self.connections.get(&saved.id).map(|c| c.ssh_secret);
         self.dialog = None;
-        // The dialog has no control for what agents may do: an edit keeps
-        // what the file says now, not what the dialog opened with. A change
+        // What agents may do: an access the person chose in the dialog is
+        // their own, and one they raised is shown, never held. The access
+        // the dialog opened with keeps what the file says now: a change
         // made outside the app stays held, and one put back stays back.
-        saved.agents = self
-            .connections
-            .get(&saved.id)
-            .map(|saved| saved.agents)
-            .unwrap_or_default();
+        match self.connections.get(&saved.id) {
+            Some(was) if saved.agents.access == opened => {
+                saved.agents.access = was.agents.access;
+            }
+            was => {
+                let in_force = was.map(|was| self.agents_access(was)).unwrap_or_default();
+                if saved.agents.access > in_force {
+                    self.agents_shown
+                        .keep(&[crate::agents_shown::Raised::Access {
+                            id: saved.id.clone(),
+                            to: saved.agents.access,
+                        }]);
+                    self.save_agents_shown();
+                }
+            }
+        }
         self.connections.upsert(saved.clone());
         self.save_connections();
         if saved.password == PasswordMode::Keyring {
@@ -3955,6 +4012,37 @@ impl App {
         }
     }
 
+    /// Sets what agents may do on `conn`, from the app's own controls: a
+    /// raise is the person's, shown and never held; a lowering applies at
+    /// once. Saved, and the backend told, as any change of the
+    /// connections.
+    fn set_agent_access(
+        &mut self,
+        conn: &crate::connections::ConnectionId,
+        access: crate::connections::AgentAccess,
+    ) {
+        let Some(mut saved) = self.connections.get(conn).cloned() else {
+            return;
+        };
+        if access > self.agents_access(&saved) {
+            self.agents_shown
+                .keep(&[crate::agents_shown::Raised::Access {
+                    id: conn.clone(),
+                    to: access,
+                }]);
+            self.save_agents_shown();
+        }
+        if saved.agents.access == access {
+            // Kept as it was in the file: what was held is in force now.
+            self.check_agents();
+            self.send_agents_policy();
+            return;
+        }
+        saved.agents.access = access;
+        self.connections.upsert(saved);
+        self.save_connections();
+    }
+
     /// Stops one agent's session. Its line goes when the backend says the
     /// sessions again.
     fn stop_agent_session(&mut self, id: String) {
diff --git a/src/model.rs b/src/model.rs
index b0df033..7943e35 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -651,6 +651,23 @@ pub enum Action {
         tab: ConnTabId,
         step: isize,
     },
+    /// Set what agents may do on a saved connection, from the Connections
+    /// list's menu or Omarchy's box: the person's own, never held.
+    SetAgentAccess {
+        conn: ConnectionId,
+        access: crate::connections::AgentAccess,
+    },
+    /// Open the connection's editor at its Agents part: Tables, columns
+    /// and limits… in the list's menu, and, with `allow`, Allow agents… in
+    /// the Agents tab, with Read chosen. Nothing changes until Save.
+    EditAgents {
+        conn: ConnectionId,
+        allow: bool,
+    },
+    /// Open the terminal look's box of what agents may do on a connection.
+    OpenAgentAccess(ConnectionId),
+    /// Move the box's cursor among the levels.
+    MoveAgentAccess(isize),
     /// Keep what was changed outside the app and lets agents do more: what
     /// the bar said, and only what is still held of it.
     KeepAgentChanges(Vec<crate::agents_shown::Raised>),
@@ -989,9 +1006,20 @@ pub struct ConnectionForm {
     pub environment: Option<crate::env::Environment>,
     /// Read-only as saved: `None` until the user sets it.
     pub read_only: Option<bool>,
-    /// What agents may do on the connection, as it was saved. The dialog
-    /// has no control for it yet: saving takes what the file says then.
+    /// What agents may do on the connection: its access as the person
+    /// chooses it here. Rows and the timeout are typed in the two fields
+    /// below.
     pub agents: crate::connections::Agents,
+    /// The access in force when the dialog opened: a choice other than it
+    /// is the person's own, and the same one keeps what the file says.
+    pub agents_opened: crate::connections::AgentAccess,
+    /// Rows per answer, as typed.
+    pub agents_rows: String,
+    /// The statement's timeout in seconds, as typed.
+    pub agents_timeout: String,
+    /// Bring the Agents part into sight on the next frame: Allow agents…
+    /// and the menu's Tables, columns and limits… open the dialog there.
+    pub show_agents: bool,
     /// Put the keyboard in Name on the next frame (a new connection).
     pub focus_name: bool,
     pub driver: Driver,
@@ -1103,6 +1131,12 @@ impl Default for ConnectionForm {
             environment: None,
             read_only: None,
             agents: crate::connections::Agents::default(),
+            agents_opened: crate::connections::AgentAccess::Off,
+            agents_rows: crate::connections::Agents::default().rows.to_string(),
+            agents_timeout: crate::connections::Agents::default()
+                .timeout_secs
+                .to_string(),
+            show_agents: false,
             focus_name: true,
             driver: Driver::Sqlite,
             sqlite_path: String::new(),
@@ -1242,6 +1276,10 @@ impl ConnectionForm {
             environment: Some(saved.environment),
             read_only: saved.read_only,
             agents: saved.agents,
+            agents_opened: saved.agents.access,
+            agents_rows: saved.agents.rows.to_string(),
+            agents_timeout: saved.agents.timeout_secs.to_string(),
+            show_agents: false,
             focus_name: false,
             driver: spec.driver,
             sqlite_path: spec
@@ -1480,6 +1518,7 @@ impl ConnectionForm {
             }
             _ => self.ssh_secret_mode,
         };
+        let agents = self.agents_to_saved()?;
         Ok(crate::connections::SavedConnection {
             id: self.editing.clone().unwrap_or_else(ConnectionId::new),
             name: name.to_owned(),
@@ -1488,7 +1527,32 @@ impl ConnectionForm {
             password,
             ssh_secret,
             spec,
-            agents: self.agents,
+            agents,
+        })
+    }
+
+    /// What agents may do, as the fields say it: the rows and the timeout
+    /// in the ranges a connection may be given.
+    pub fn agents_to_saved(&self) -> Result<crate::connections::Agents, String> {
+        use crate::connections::{MAX_AGENT_ROWS, MAX_AGENT_TIMEOUT_SECS};
+        let within = |text: &str, most: u32| {
+            text.trim()
+                .parse::<u32>()
+                .ok()
+                .filter(|number| (1..=most).contains(number))
+        };
+        let rows = within(&self.agents_rows, MAX_AGENT_ROWS)
+            .ok_or_else(|| format!("Rows per answer is a number from 1 to {MAX_AGENT_ROWS}."))?;
+        let timeout_secs =
+            within(&self.agents_timeout, MAX_AGENT_TIMEOUT_SECS).ok_or_else(|| {
+                format!(
+                    "Statement timeout is a number of seconds from 1 to {MAX_AGENT_TIMEOUT_SECS}."
+                )
+            })?;
+        Ok(crate::connections::Agents {
+            access: self.agents.access,
+            rows,
+            timeout_secs,
         })
     }
 }
@@ -1557,6 +1621,18 @@ pub enum Dialog {
     ConfirmWrite(Box<WritePrompt>),
     /// A save found rows changed on the server: what to do with each.
     Conflict(Box<ConflictPrompt>),
+    /// What agents may do on a connection, in the terminal look's box.
+    AgentAccess(Box<AgentAccessBox>),
+}
+
+/// The terminal look's box of what agents may do on a saved connection.
+#[derive(Debug)]
+pub struct AgentAccessBox {
+    pub conn: ConnectionId,
+    /// As an agent names it: "Bookshop · dev".
+    pub name: String,
+    /// The level the box's cursor is on.
+    pub chosen: crate::connections::AgentAccess,
 }
 
 /// What waits for the user's answer about pending changes.
````

- [ ] **Step 4: Run the tests**

Run: the same, then `cargo test --locked --lib app::`

Expected: PASS.

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2506 filtered out
test result: ok. 406 passed; 0 failed; 0 ignored; 0 measured; 2102 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src
git commit -m "Let the person set what agents may do on a connection"
```

---

### Task 2: The list, the box, the editor, Allow agents…

- macOS: the Agents column, 140 wide, out of the row's room and the host while the host keeps 160: "Agents" over the level, dimmed when Off. In a narrower window it steps back and the row is as it was; the menu still has the Agents part. The "…" and right-click menus end with an Agents part (`menu::heading`): Off and Read, a check on the one in force, **Tables, columns and limits…** and **Show Agents tab**.
- Omarchy: `· agents read` at the end of the row's line; `a` opens the box "agents · Bookshop · dev": `j`/`k`, a plain `enter` sets (not with Shift, nor what a held one repeats), `t` shows the Agents tab, `esc` leaves it. The list connects on a press of Enter, not on what a held one repeats: one held in the box that closed on it connects nothing. The footer says `a agent access`, and its hints give way from the end where the window has no room beside the keyring's note.
- The editor's Agents section after Safety, in both looks: Off and Read, "Agents use this connection as dev.", Rows per answer, Statement timeout, the note. The whole section is brought into sight while `show_agents`, until it is, and again when Save finds one of its values wrong.
- The Agents tab's Off state: **Allow agents…** (`e` on Omarchy), with Show log; none of it, nor its hint, for a connection saved no more.
- Nothing of it where agents cannot reach the app.
- The website: the connections page's Agents section, Omarchy's `a` and `e`.

**Files:**
- Create: `src/ui/agent_access.rs`, `src/ui/connect_dialog/agents.rs`
- Modify: `src/ui/picker.rs`, `src/ui/menu.rs`, `src/ui/mod.rs`, `src/ui/connect_dialog/mod.rs`, `src/ui/connect_dialog/sheet.rs`, `src/ui/connect_dialog/terminal.rs`, `src/ui/agents.rs`, `src/keymap.rs`, `src/ui/keys.rs`, `src/ui/workspace.rs`, `src/ui/agents_tests.rs`, `docs/_reference/connections.md`, `docs/_guide/omarchy.md`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index cbc0b9e..8b476ce 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -56,6 +56,7 @@ list instead.
 | `yy` then `p` | Duplicate |
 | `dd` | Delete |
 | `/` | Filter the list |
+| `a` | What agents may do on the connection: `j` `k`, Enter to set it, `t` for its Agents tab, Esc |
 | Esc | Cancel connecting |
 
 {% include shot.html file="omarchy-connections" alt="The connection picker in the Omarchy look, with its keys along the bottom" %}
@@ -110,6 +111,7 @@ key. Nor has copying a whole row.
 | --- | --- |
 | `y` | Copy the command that connects an agent |
 | `o` | Turn the agent server on, while it is off |
+| `e` | Allow agents on this connection, while their access is off |
 | `j` / `k` | Move among the agents' sessions: those here, then those elsewhere |
 | `S` | Stop the selected session, wherever it works |
 | `X` | Stop every agent here, and turn this connection's access off |
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
index 91579fe..2807b82 100644
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -34,6 +34,7 @@ already open, and Shift+Enter opens it again.
 | **SSL mode**, **CA certificate** | See [TLS](#tls) |
 | **Connect through SSH tunnel** | See [SSH tunnels](#ssh-tunnels) |
 | **Open read-only** | See [Open read-only](#open-read-only) |
+| **Agents** | See [Agents](#agents) (Linux and macOS) |
 
 **Test** checks the connection without saving it.
 
@@ -143,6 +144,27 @@ save the connection. One typed when Tabletist asks for it is saved only
 after the server accepted it. The same goes for an SSH password and a
 key's passphrase.
 
+## Agents
+
+What AI agents may do on a connection, through `tabletist mcp`. Every
+connection starts with agents **Off**: they do not see it at all.
+**Read** lets them read it, in read-only transactions, while the agent
+server is on.
+
+- In the picker, the **Agents** column says each connection's access, and
+  its "…" menu changes it. Its **Tables, columns and limits…** opens the
+  connection's **Agents** section, and **Show Agents tab** shows its
+  Agents tab.
+- In the connection's form, the **Agents** section has the access, **Rows
+  per answer** (1 to 5000, 500 at first) and the **Statement timeout** in
+  seconds (1 to 60, 10 at first).
+- An Agents tab that says agents can't use a connection has **Allow
+  agents…**, which opens that section with **Read** chosen. Nothing
+  changes until you save.
+
+The file keeps this in the connection's `[connections.agents]` table: see
+[Settings and Files]({% link _reference/settings-and-files.md %}#the-connections-file).
+
 ## Open read-only
 
 A connection with **Open read-only** ticked blocks every write from
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index 528a9b1..3edd105 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -984,3 +984,313 @@ fn a_call_selected_with_j_is_kept_in_sight() {
         .expect("the selected call");
     assert!(row.bottom() <= harness.size.y, "{row:?}");
 }
+
+/// The Connections list with one saved connection, Shop · dev, selected.
+#[cfg(unix)]
+fn on_the_list(look: Look) -> (Harness, crate::connections::ConnectionId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let mut shop = crate::testing::fixture_connection();
+    shop.name = "Shop".into();
+    let conn = shop.id.clone();
+    harness.app.connections.upsert(shop);
+    let tab = harness.app.active_tab_id();
+    harness.app.apply(Action::SelectConnection {
+        tab,
+        conn: Some(conn.clone()),
+    });
+    harness.settle();
+    (harness, conn)
+}
+
+#[cfg(unix)]
+fn access_of(harness: &Harness, conn: &crate::connections::ConnectionId) -> AgentAccess {
+    harness.app.connections.get(conn).unwrap().agents.access
+}
+
+#[cfg(unix)]
+#[test]
+fn the_list_says_what_agents_may_do_and_its_menu_changes_it() {
+    let (mut harness, conn) = on_the_list(Look::macos());
+    assert!(harness.has("Agents Off"));
+    harness.click("More actions for Shop");
+    harness.settle();
+    harness.click("Agents Read");
+    harness.settle();
+    assert_eq!(access_of(&harness, &conn), AgentAccess::Read);
+    assert!(harness.has("Agents Read"));
+    // Tables, columns and limits…: the editor, at its Agents part. (A
+    // click no pointer made leaves the menu open.)
+    harness.click("Tables, columns and limits…");
+    harness.settle();
+    assert!(matches!(
+        harness.app.dialog,
+        Some(crate::model::Dialog::Connection(_))
+    ));
+    assert!(harness.has("Access through Tabletist (MCP)"));
+}
+
+#[cfg(unix)]
+#[test]
+fn omarchys_box_sets_what_agents_may_do_from_the_list() {
+    let (mut harness, conn) = on_the_list(Look::omarchy());
+    // At the end of the row's line.
+    let line_ends = |harness: &Harness, end: &str| {
+        harness
+            .text_rects
+            .iter()
+            .any(|(text, _)| text.ends_with(end))
+    };
+    assert!(line_ends(&harness, "· agents off"));
+    super::tests::type_key(&mut harness, egui::Key::A, "a");
+    assert!(matches!(
+        harness.app.dialog,
+        Some(crate::model::Dialog::AgentAccess(_))
+    ));
+    assert!(harness.has("agents · Shop · dev"));
+    // `j` to read, Enter sets it.
+    super::tests::type_key(&mut harness, egui::Key::J, "j");
+    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+    assert!(harness.app.dialog.is_none());
+    assert_eq!(access_of(&harness, &conn), AgentAccess::Read);
+    harness.settle();
+    assert!(line_ends(&harness, "· agents read"));
+    // Esc leaves it as it was.
+    super::tests::type_key(&mut harness, egui::Key::A, "a");
+    super::tests::type_key(&mut harness, egui::Key::K, "k");
+    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
+    assert!(harness.app.dialog.is_none());
+    assert_eq!(access_of(&harness, &conn), AgentAccess::Read);
+    // `t`: the connection's window, on its Agents tab.
+    super::tests::type_key(&mut harness, egui::Key::A, "a");
+    super::tests::type_key(&mut harness, egui::Key::T, "t");
+    let shown = harness.app.active_tab_id();
+    let workspace = harness.app.workspace(shown).expect("its window");
+    assert_eq!(workspace.conn_id, conn);
+    assert!(workspace.agents_in_front());
+}
+
+#[cfg(unix)]
+#[test]
+fn the_editors_agents_part_sets_access_rows_and_timeout() {
+    for look in looks() {
+        let (mut harness, conn) = on_the_list(look);
+        harness.app.apply(Action::EditConnection(conn.clone()));
+        harness.settle();
+        harness.click(&look.label("Read"));
+        harness.settle();
+        harness.app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(
+            access_of(&harness, &conn),
+            AgentAccess::Read,
+            "{}",
+            look.name
+        );
+        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
+    }
+}
+
+#[cfg(unix)]
+#[test]
+fn allow_agents_opens_the_editor_with_read_chosen() {
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        harness.app.settings.mcp_enabled = true;
+        harness.app.apply(Action::ShowAgents(tab));
+        harness.settle();
+        let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+        if look.terminal {
+            super::tests::type_key(&mut harness, egui::Key::E, "e");
+        } else {
+            harness.click("Allow agents…");
+        }
+        harness.settle();
+        let Some(crate::model::Dialog::Connection(form)) = &harness.app.dialog else {
+            panic!("{}: no editor", look.name);
+        };
+        assert_eq!(form.agents.access, AgentAccess::Read, "{}", look.name);
+        // Nothing until Save.
+        assert_eq!(
+            access_of(&harness, &conn),
+            AgentAccess::Off,
+            "{}",
+            look.name
+        );
+        harness.app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(
+            access_of(&harness, &conn),
+            AgentAccess::Read,
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[cfg(unix)]
+#[test]
+fn the_editor_opens_at_its_agents_part() {
+    for look in looks() {
+        // A short window, and a server's form: the part is under the fold.
+        let mut harness = Harness::with_size(egui::vec2(1280.0, 520.0));
+        harness.set_look(look);
+        let mut shop = crate::testing::fixture_connection();
+        let (spec, _) =
+            tabletist_db::ConnectSpec::from_url("postgres://demo@localhost:5432/bookshop").unwrap();
+        shop.spec = spec;
+        let conn = shop.id.clone();
+        harness.app.connections.upsert(shop);
+        harness.app.apply(Action::EditConnection(conn.clone()));
+        harness.settle();
+        let heading = if look.terminal {
+            look.label("Agents")
+        } else {
+            "Access through Tabletist (MCP)".to_owned()
+        };
+        let seen = |harness: &Harness| {
+            harness
+                .text_rects
+                .iter()
+                .any(|(text, rect)| *text == heading && rect.bottom() < harness.size.y - 120.0)
+        };
+        assert!(
+            !seen(&harness),
+            "{}: in sight without being asked",
+            look.name
+        );
+        harness.app.apply(Action::CloseDialog);
+        harness.app.apply(Action::EditAgents { conn, allow: false });
+        // A scroll is animated: it lands some frames on.
+        for _ in 0..20 {
+            harness.settle();
+        }
+        let shown = harness
+            .text_rects
+            .iter()
+            .rev()
+            .find(|(text, _)| *text == heading)
+            .map(|(_, rect)| *rect);
+        // In sight, above the dialog's footer.
+        let shown = shown.expect("the Agents part");
+        assert!(
+            shown.bottom() < harness.size.y - 120.0,
+            "{}: {shown:?}",
+            look.name
+        );
+    }
+}
+
+#[cfg(unix)]
+#[test]
+fn the_limits_are_in_sight_when_the_editor_opens_at_them() {
+    for (look, size) in [(Look::macos(), 600.0), (Look::omarchy(), 700.0)] {
+        let mut harness = Harness::with_size(egui::vec2(1280.0, size));
+        harness.set_look(look);
+        let mut shop = crate::testing::fixture_connection();
+        let (spec, _) =
+            tabletist_db::ConnectSpec::from_url("postgres://demo@localhost:5432/bookshop").unwrap();
+        shop.spec = spec;
+        let conn = shop.id.clone();
+        harness.app.connections.upsert(shop);
+        harness.app.apply(Action::EditAgents { conn, allow: false });
+        for _ in 0..20 {
+            harness.settle();
+        }
+        let rows = look.label("Rows per answer");
+        let shown = harness
+            .text_rects
+            .iter()
+            .rev()
+            .find(|(text, _)| *text == rows)
+            .map(|(_, rect)| *rect);
+        let shown = shown.unwrap_or_else(|| panic!("{}: no rows per answer", look.name));
+        assert!(
+            shown.bottom() < harness.size.y - 60.0,
+            "{}: {shown:?}",
+            look.name
+        );
+    }
+}
+
+#[cfg(unix)]
+#[test]
+fn an_enter_held_or_shifted_in_the_box_does_nothing_behind_it() {
+    use egui::{Key, Modifiers};
+    let (mut harness, conn) = on_the_list(Look::omarchy());
+    let windows = harness.app.tabs.len();
+    super::tests::type_key(&mut harness, Key::A, "a");
+    super::tests::type_key(&mut harness, Key::J, "j");
+    // Shift+Enter is no answer.
+    harness.press(Key::Enter, Modifiers::SHIFT);
+    assert_eq!(access_of(&harness, &conn), AgentAccess::Off);
+    assert!(harness.app.dialog.is_some());
+    // Enter held: it sets the level, and what it repeats connects nothing.
+    for _ in 0..4 {
+        harness.frame(vec![crate::testing::key(Key::Enter, Modifiers::NONE)]);
+    }
+    harness.frame(vec![crate::testing::release(Key::Enter, Modifiers::NONE)]);
+    harness.settle();
+    assert_eq!(access_of(&harness, &conn), AgentAccess::Read);
+    assert_eq!(harness.app.tabs.len(), windows);
+    assert!(
+        !harness
+            .app
+            .backend
+            .sent
+            .iter()
+            .any(|sent| matches!(sent, crate::backend::Command::Connect { .. }))
+    );
+}
+
+#[cfg(unix)]
+#[test]
+fn no_allow_agents_is_offered_for_a_connection_saved_no_more() {
+    let (mut harness, tab) = connected(Look::omarchy());
+    harness.app.settings.mcp_enabled = true;
+    harness.app.apply(Action::ShowAgents(tab));
+    harness.settle();
+    let offered = |harness: &Harness| {
+        harness
+            .text_rects
+            .iter()
+            .any(|(text, _)| text.contains("allow agents"))
+    };
+    assert!(offered(&harness));
+    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+    harness.app.connections.remove(&conn);
+    harness.settle();
+    assert!(!offered(&harness));
+}
+
+#[cfg(unix)]
+#[test]
+fn the_agents_column_and_hint_step_back_where_the_window_has_no_room() {
+    // macOS at 1024: Last used stays clear of the row's buttons.
+    let mut harness = Harness::with_size(egui::vec2(1024.0, 700.0));
+    let mut shop = crate::testing::fixture_connection();
+    shop.name = "Shop".into();
+    let conn = shop.id.clone();
+    harness.app.connections.upsert(shop);
+    let tab = harness.app.active_tab_id();
+    harness.app.apply(Action::SelectConnection {
+        tab,
+        conn: Some(conn),
+    });
+    harness.settle();
+    let used = harness.painted_rect("Last used never").expect("last used");
+    let edit = harness.painted_rect("Edit").expect("Edit");
+    assert!(used.right() < edit.left(), "{used:?} {edit:?}");
+    // Omarchy at 720: the footer's hints stay clear of the keyring's note.
+    let (mut harness, _) = on_the_list(Look::omarchy());
+    harness.size = egui::vec2(720.0, 600.0);
+    harness.settle();
+    let note = harness.painted_rect("secrets: keyring").expect("the note");
+    let hints = harness
+        .text_rects
+        .iter()
+        .filter(|(_, rect)| {
+            (rect.center().y - note.center().y).abs() < 2.0 && rect.left() < note.left()
+        })
+        .map(|(_, rect)| rect.right())
+        .fold(0.0_f32, f32::max);
+    assert!(hints < note.left(), "{hints} {note:?}");
+}
````

- [ ] **Step 2: Run them to see that they fail**

Run: `cargo test --locked --lib -- ui::agents_tests::the_list ui::agents_tests::omarchys_box ui::agents_tests::the_editor ui::agents_tests::allow_agents ui::agents_tests::the_limits ui::agents_tests::an_enter_held ui::agents_tests::no_allow ui::agents_tests::the_agents_column`

Expected: all nine FAIL. Nothing draws the column, the menu's Agents part, the box, the editor's section or Allow agents… yet:

```text
test ui::agents_tests::omarchys_box_sets_what_agents_may_do_from_the_list ... FAILED
test ui::agents_tests::an_enter_held_or_shifted_in_the_box_does_nothing_behind_it ... FAILED
test ui::agents_tests::no_allow_agents_is_offered_for_a_connection_saved_no_more ... FAILED
test ui::agents_tests::the_list_says_what_agents_may_do_and_its_menu_changes_it ... FAILED
test ui::agents_tests::the_editors_agents_part_sets_access_rows_and_timeout ... FAILED
test ui::agents_tests::allow_agents_opens_the_editor_with_read_chosen ... FAILED
test ui::agents_tests::the_agents_column_and_hint_step_back_where_the_window_has_no_room ... FAILED
test ui::agents_tests::the_limits_are_in_sight_when_the_editor_opens_at_them ... FAILED
test ui::agents_tests::the_editor_opens_at_its_agents_part ... FAILED
test result: FAILED. 0 passed; 9 failed; 0 ignored; 0 measured; 2508 filtered out
```

- [ ] **Step 3: Draw them, and their keys**

````diff
diff --git a/src/keymap.rs b/src/keymap.rs
index 8513890..670e4ae 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -143,6 +143,8 @@ pub enum When {
     ConfirmWrite,
     /// The question about a row that changed on the server.
     Conflict,
+    /// The box of what agents may do on a connection, in the terminal look.
+    AgentAccess,
     /// The question about leaving with pending changes.
     Leaving,
     /// The cursor's row is a new one, not saved yet: the row panel is its
@@ -340,6 +342,10 @@ commands! {
     ShowAgentsElsewhere,
     ShowAgentsLog,
     ToggleAgentsLogScope,
+    AllowAgents,
+    ChangeAgentAccess,
+    MoveInAgentLevels,
+    ShowAgentsFromBox,
     ToggleSidebar,
     PaneLeft,
     PaneDown,
@@ -588,6 +594,14 @@ impl Command {
                 agents(info("Show where the selected agent works", "", Window))
             }
             C::ShowAgentsLog => agents(info("Show the log, or what agents do now", "", Window)),
+            C::AllowAgents => agents(info("Allow agents on this connection…", "", Window)),
+            C::ChangeAgentAccess => agents(info(
+                "Change what agents may do on the connection",
+                "",
+                Window,
+            )),
+            C::MoveInAgentLevels => agents(info("Move among what agents may do", "", Prompts)),
+            C::ShowAgentsFromBox => agents(info("Show the connection's Agents tab", "", Prompts)),
             C::ToggleAgentsLogScope => agents(info(
                 "Show every connection's calls in the log, or this one's",
                 "",
@@ -990,6 +1004,16 @@ pub const BINDINGS: &[Binding] = {
         bind(C::ShowAgentsLog, GLOBAL, NONE, keys(&[k(":agents log")])),
         bind(C::ShowAgentsLog, AGENTS, NONE, keys(&[k("l")])),
         bind(C::ToggleAgentsLogScope, AGENTS, NONE, keys(&[k("L")])),
+        bind(C::AllowAgents, AGENTS, NONE, keys(&[k("e")])),
+        bind(C::ChangeAgentAccess, CONNECTIONS, NONE, keys(&[k("a")])),
+        bind(
+            C::MoveInAgentLevels,
+            PROMPT,
+            NONE,
+            keys(&[k("j"), k("k")]).shown("j/k"),
+        )
+        .when(When::AgentAccess),
+        bind(C::ShowAgentsFromBox, PROMPT, NONE, keys(&[k("t")])).when(When::AgentAccess),
         bind(
             C::ToggleSidebar,
             GLOBAL,
diff --git a/src/ui/agent_access.rs b/src/ui/agent_access.rs
new file mode 100644
index 0000000..556e77b
--- /dev/null
+++ b/src/ui/agent_access.rs
@@ -0,0 +1,154 @@
+//! The terminal look's box of what agents may do on a saved connection:
+//! `a` on a row of the Connections list opens it. The levels, the one in
+//! force marked, `j` and `k` among them, `enter` to set one, `t` for the
+//! connection's Agents tab, `esc` to leave it as it was.
+
+use egui::{Id, Key, Margin, Modifiers, Rect, Sense, vec2};
+
+use crate::app::App;
+use crate::connections::AgentAccess;
+use crate::i18n::gettext;
+use crate::keymap::{Command, Layout, When};
+use crate::model::{Action, Dialog};
+use crate::typography::{Text, TextRole};
+use crate::ui::{terminal_dialog, widgets};
+
+/// The levels, as the box lists them.
+const LEVELS: [(AgentAccess, &str); 2] = [(AgentAccess::Off, "off"), (AgentAccess::Read, "read")];
+
+pub fn show(app: &mut App, ctx: &egui::Context) {
+    let Some(Dialog::AgentAccess(asked)) = &app.dialog else {
+        return;
+    };
+    let (conn, name, chosen) = (asked.conn.clone(), asked.name.clone(), asked.chosen);
+    let (look, palette, locale) = (app.look, app.palette, app.locale);
+    let Some(current) = app
+        .connections
+        .get(&conn)
+        .map(|saved| app.agents_access(saved))
+    else {
+        // Deleted while it was open: nothing to set.
+        app.actions.push(Action::CloseDialog);
+        return;
+    };
+    let mut actions = Vec::new();
+    // The letters, as typed text: the box's own, by the keymap.
+    let (keymap, layout) = (crate::ui::keys::published(ctx), Layout::of(&look));
+    let typed = ctx.input_mut(|input| {
+        let mut typed = Vec::new();
+        input.events.retain(|event| match event {
+            egui::Event::Text(text) => {
+                match crate::ui::keys::prompt_letter(&keymap, layout, When::AgentAccess, text) {
+                    Some(command) => {
+                        typed.push((command, text.clone()));
+                        false
+                    }
+                    None => true,
+                }
+            }
+            _ => true,
+        });
+        typed
+    });
+    for (command, text) in typed {
+        match command {
+            Command::MoveInAgentLevels => {
+                let step = if text == "k" { -1 } else { 1 };
+                actions.push(Action::MoveAgentAccess(step));
+            }
+            Command::ShowAgentsFromBox => {
+                actions.push(Action::CloseDialog);
+                actions.push(Action::ShowAgentsOn(conn.clone()));
+            }
+            _ => {}
+        }
+    }
+    let set = |actions: &mut Vec<Action>, level: AgentAccess| {
+        if level != current {
+            actions.push(Action::SetAgentAccess {
+                conn: conn.clone(),
+                access: level,
+            });
+        }
+        actions.push(Action::CloseDialog);
+    };
+    // A plain Enter that went down here: with a modifier it is no answer,
+    // and what a held one repeats answers nothing either.
+    ctx.input_mut(|input| {
+        crate::ui::keys::drop_repeats(input, Key::Enter);
+        crate::ui::keys::drop_repeats(input, Key::Escape);
+    });
+    if ctx.input_mut(crate::ui::keys::take_enter) {
+        set(&mut actions, chosen);
+    }
+    if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
+        actions.push(Action::CloseDialog);
+    }
+    let frame = widgets::modal_frame(&look, &palette).inner_margin(Margin::ZERO);
+    let radius = look.dialog_radius.saturating_sub(frame.stroke.width as u8);
+    let say = |text: &'static str| look.label(&gettext(locale, text));
+    widgets::modal(Id::new("agent-access"), &look, &palette)
+        .frame(frame)
+        .show(ctx, |ui| {
+            ui.set_width(320.0);
+            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
+            let head = terminal_dialog::head(ui, radius, palette.panel, &palette);
+            let title = format!("{} · {name}", say("Agents"));
+            widgets::paint_text(
+                ui,
+                head.left() + 18.0,
+                terminal_dialog::head_line(head),
+                Text::one(&look, TextRole::OGroup, &title, palette.text),
+            );
+            widgets::announce(ui, head, &title);
+            ui.add_space(8.0);
+            let row = TextRole::OBody.row_height(ui.ctx(), look.faces) + 6.0;
+            for (level, word) in LEVELS {
+                let (rect, response) =
+                    ui.allocate_exact_size(vec2(ui.available_width(), row), Sense::click());
+                if level == chosen {
+                    ui.painter().rect_filled(rect, 0.0, palette.selection);
+                    let edge = Rect::from_min_size(rect.min, vec2(2.0, rect.height()));
+                    ui.painter().rect_filled(edge, 0.0, palette.accent);
+                }
+                // The level in force is checked: painted, as not every
+                // face has the glyph.
+                if level == current {
+                    let place = Rect::from_center_size(
+                        egui::pos2(rect.left() + 24.0, rect.center().y),
+                        vec2(12.0, 12.0),
+                    );
+                    crate::theme::Icon::Check
+                        .image(palette.text, 12.0)
+                        .paint_at(ui, place);
+                }
+                widgets::paint_text(
+                    ui,
+                    rect.left() + 38.0,
+                    rect.center().y,
+                    Text::one(&look, TextRole::OBody, &say(word), palette.text),
+                );
+                widgets::announce(ui, rect, &format!("{} {}", say("Agents"), say(word)));
+                if response.clicked() {
+                    set(&mut actions, level);
+                }
+            }
+            ui.add_space(8.0);
+            let foot = terminal_dialog::foot(ui, radius, &palette);
+            let opens = crate::ui::keys::written(ui.ctx(), &look, Command::ChangeAgentAccess);
+            let moves = crate::ui::keys::written(ui.ctx(), &look, Command::MoveInAgentLevels);
+            let tab = crate::ui::keys::written(ui.ctx(), &look, Command::ShowAgentsFromBox);
+            let words = format!(
+                "{opens} {} · {moves} · {tab} {}",
+                say("opens this"),
+                say("agents tab")
+            );
+            widgets::paint_text(
+                ui,
+                foot.left() + 18.0,
+                terminal_dialog::foot_line(foot),
+                Text::one(&look, widgets::secondary(&look), &words, palette.dim),
+            );
+        });
+    app.actions.extend(actions);
+}
diff --git a/src/ui/agents.rs b/src/ui/agents.rs
index db357b3..59ec6d9 100644
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -88,6 +88,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         crate::keymap::Command::ShowAgentsLog,
         crate::keymap::Scope::Agents,
     );
+    let allow_key = app.keymap.key(layout, crate::keymap::Command::AllowAgents);
     let scope_key = app
         .keymap
         .key(layout, crate::keymap::Command::ToggleAgentsLogScope);
@@ -254,11 +255,25 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                             "Access is Off for this connection, so agents don't see it at all.",
                         ),
                     };
-                    let label = gettext(locale, "Show log");
-                    let button = states::key_button(&label, &log_key, &look);
-                    if states::empty(ui, rect, &notice, vec![button], &look, &palette).is_some() {
-                        let view = AgentsView::Log;
-                        actions.push(Action::ShowAgentsView { tab, view });
+                    let (log_label, allow_label) =
+                        (gettext(locale, "Show log"), gettext(locale, "Allow agents…"));
+                    let mut buttons = vec![states::key_button(&log_label, &log_key, &look)];
+                    // A connection saved no more has nothing to allow.
+                    let saved = app.connections.get(&conn).is_some();
+                    if saved {
+                        let allow = states::key_button(&allow_label, &allow_key, &look).primary();
+                        buttons.push(allow);
+                    }
+                    match states::empty(ui, rect, &notice, buttons, &look, &palette) {
+                        Some(0) => {
+                            let view = AgentsView::Log;
+                            actions.push(Action::ShowAgentsView { tab, view });
+                        }
+                        Some(_) => actions.push(Action::EditAgents {
+                            conn: conn.clone(),
+                            allow: true,
+                        }),
+                        None => {}
                     }
                 }
                 AgentsHere::NoneConnected if look.terminal && !elsewhere.is_empty() => {
diff --git a/src/ui/connect_dialog/agents.rs b/src/ui/connect_dialog/agents.rs
new file mode 100644
index 0000000..3471671
--- /dev/null
+++ b/src/ui/connect_dialog/agents.rs
@@ -0,0 +1,149 @@
+//! The connection editor's Agents part: what agents may do on the
+//! connection (Off or Read), how many rows an answer holds, and how long a
+//! statement may run. Phase 1b's rows of the boards: hidden tables come
+//! with 1c, masked columns with 3a, the third level with writes.
+
+use egui::{Align, Ui};
+
+use super::choice::{Choice, Group, choose};
+use super::{Skin, input};
+use crate::connections::AgentAccess;
+use crate::model::ConnectionForm;
+use crate::typography::TextRole;
+use crate::ui::widgets;
+
+/// The levels, as the switch says them.
+const LEVELS: [(AgentAccess, &str); 2] = [(AgentAccess::Off, "Off"), (AgentAccess::Read, "Read")];
+
+/// What the part says under its rows, from the board.
+const NOTE: &str =
+    "When Open read-only is on, agents get Read at most. Every connection starts with agents Off.";
+
+/// The access switch, as a group of choices headed `heading`.
+fn access(ui: &mut Ui, form: &mut ConnectionForm, heading: &str, group: Group, skin: &Skin) {
+    let labels = LEVELS.map(|(_, word)| skin.say(word));
+    let options: Vec<(AgentAccess, Choice<'_>)> = LEVELS
+        .iter()
+        .zip(&labels)
+        .map(|((level, _), label)| (*level, Choice::plain(label)))
+        .collect();
+    let current = form.agents.access;
+    if let Some(level) = choose(
+        ui,
+        "agents-access",
+        Some(heading),
+        &options,
+        current,
+        group,
+        skin,
+    ) {
+        form.agents.access = level;
+    }
+}
+
+/// "Agents use this connection as dev.": the environment an agent's
+/// session opens as. Nothing for a connection with none.
+fn runs_as(skin: &Skin) -> Option<String> {
+    let named = skin.environment.named(crate::env::Platform::Native)?;
+    Some(
+        skin.say("Agents use this connection as {env}.")
+            .replace("{env}", &named),
+    )
+}
+
+/// A sentence of the part, wrapped to the room it has: no line may widen
+/// the dialog.
+fn sentence(ui: &mut Ui, role: TextRole, text: &str, skin: &Skin) {
+    let room = ui.available_width();
+    crate::typography::Text::one(skin.look, role, text, skin.palette.dim)
+        .wrap(room)
+        .layout(ui.ctx())
+        .label(ui);
+}
+
+/// Brings the whole part into sight, where the dialog was opened at it
+/// or a value of it is wrong: asked for on each frame that is seen, from
+/// `top` to where the part ends, until it is in sight, as the dialog's
+/// first frame only measures it.
+fn shown(ui: &Ui, form: &mut ConnectionForm, top: f32) {
+    if !form.show_agents || !ui.is_visible() || ui.is_sizing_pass() {
+        return;
+    }
+    let part = egui::Rect::from_min_max(
+        egui::pos2(ui.min_rect().left(), top),
+        egui::pos2(ui.min_rect().right(), ui.cursor().top()),
+    );
+    let clip = ui.clip_rect();
+    // A part taller than the room is in sight from its top.
+    let seen = clip.contains_rect(part)
+        || (part.height() > clip.height() && (part.top() - clip.top()).abs() < 1.0);
+    if seen {
+        form.show_agents = false;
+    } else {
+        ui.scroll_to_rect(part, Some(Align::TOP));
+    }
+}
+
+/// macOS: the "Agents" fieldset after Safety.
+pub(super) fn sheet(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
+    let Skin { look, palette, .. } = *skin;
+    let top = ui.cursor().top();
+    super::sheet::fieldset(ui, &skin.say("Agents"), skin, |ui| {
+        let small = TextRole::Secondary;
+        let heading = skin.say("Access through Tabletist (MCP)");
+        widgets::label(ui, small, &heading, palette.secondary, look);
+        ui.add_space(5.0);
+        access(ui, form, &heading, Group::Track(28.0), skin);
+        if let Some(said) = runs_as(skin) {
+            ui.add_space(6.0);
+            sentence(ui, small, &said, skin);
+        }
+        ui.add_space(10.0);
+        let (role, height) = (skin.value_role(), skin.field_height());
+        let columns = [super::Column::Fr(1.0), super::Column::Fr(1.0)];
+        super::row(ui, 12.0, &columns, |ui, column| {
+            let (label, text) = match column {
+                0 => (skin.say("Rows per answer"), &mut form.agents_rows),
+                _ => (
+                    skin.say("Statement timeout, seconds"),
+                    &mut form.agents_timeout,
+                ),
+            };
+            let label = widgets::label(ui, small, &label, palette.secondary, look);
+            ui.add_space(5.0);
+            ui.add(input(ui, text, role, height, skin))
+                .labelled_by(label.id);
+        });
+        ui.add_space(8.0);
+        sentence(ui, small, &skin.say(NOTE), skin);
+    });
+    shown(ui, form, top);
+}
+
+/// The terminal look: an "agents" section after Safety.
+pub(super) fn terminal(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
+    let look = skin.look;
+    let top = ui.cursor().top();
+    super::terminal::terminal_section(ui, &skin.say("Agents"), skin);
+    let line = TextRole::OBody.row_height(ui.ctx(), look.faces);
+    let heading = skin.say("Access");
+    super::terminal::terminal_row_of(ui, &heading, line, skin, |ui, _| {
+        access(ui, form, &heading, Group::Words, skin);
+    });
+    let (role, height) = (TextRole::OBody, skin.field_height());
+    super::terminal::terminal_row(ui, &skin.say("Rows per answer"), skin, |ui, label| {
+        ui.add(input(ui, &mut form.agents_rows, role, height, skin))
+            .labelled_by(label);
+    });
+    super::terminal::terminal_row(ui, &skin.say("Timeout, seconds"), skin, |ui, label| {
+        ui.add(input(ui, &mut form.agents_timeout, role, height, skin))
+            .labelled_by(label);
+    });
+    let mut said = skin.say(NOTE);
+    if let Some(runs_as) = runs_as(skin) {
+        said = format!("{runs_as} {said}");
+    }
+    ui.add_space(4.0);
+    sentence(ui, TextRole::OSecondary, &said, skin);
+    shown(ui, form, top);
+}
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
index ac11f8a..a9f073a 100644
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -3,6 +3,7 @@
 //! environment's colour; the terminal look a two-column form with its keys
 //! in the footer.
 
+mod agents;
 mod choice;
 mod sheet;
 mod terminal;
diff --git a/src/ui/connect_dialog/sheet.rs b/src/ui/connect_dialog/sheet.rs
index d9da3ba..831ad78 100644
--- a/src/ui/connect_dialog/sheet.rs
+++ b/src/ui/connect_dialog/sheet.rs
@@ -199,7 +199,7 @@ fn select<'a>(
 
 /// macOS: a group of fields in a hairline box, its heading set into the
 /// top edge.
-fn fieldset(ui: &mut Ui, legend: &str, skin: &Skin, add: impl FnOnce(&mut Ui)) {
+pub(super) fn fieldset(ui: &mut Ui, legend: &str, skin: &Skin, add: impl FnOnce(&mut Ui)) {
     let Skin { look, palette, .. } = *skin;
     let legend = Text::one(look, TextRole::Legend, legend, palette.secondary).layout(ui.ctx());
     let half = legend.height() / 2.0;
@@ -279,6 +279,11 @@ pub(super) fn body(
                 }
                 ui.add_space(18.0);
                 safety(ui, form, skin);
+                // What agents may do on it, where they can reach the app.
+                if crate::endpoint::AVAILABLE {
+                    ui.add_space(18.0);
+                    super::agents::sheet(ui, form, skin);
+                }
             }
         });
 }
diff --git a/src/ui/connect_dialog/terminal.rs b/src/ui/connect_dialog/terminal.rs
index e6a35d7..eee9144 100644
--- a/src/ui/connect_dialog/terminal.rs
+++ b/src/ui/connect_dialog/terminal.rs
@@ -145,7 +145,7 @@ pub(super) fn terminal_row(
 
 /// [`terminal_row`] `height` tall: a row of words or of buttons is as tall
 /// as they are, as the design's rows are.
-fn terminal_row_of(
+pub(super) fn terminal_row_of(
     ui: &mut Ui,
     label: &str,
     height: f32,
@@ -189,7 +189,7 @@ fn cell(ui: &mut Ui, width: f32, skin: &Skin, add: impl FnOnce(&mut Ui) -> Respo
 }
 
 /// The terminal look: a heading over a rule, 8 below the rows before it.
-fn terminal_section(ui: &mut Ui, text: &str, skin: &Skin) {
+pub(super) fn terminal_section(ui: &mut Ui, text: &str, skin: &Skin) {
     let Skin { look, palette, .. } = *skin;
     ui.add_space(8.0);
     widgets::label(ui, TextRole::OGroup, text, palette.text, look);
@@ -336,6 +336,10 @@ pub(super) fn terminal_body(
                     ui.vertical(draw);
                 }
             });
+            // What agents may do on it, where they can reach the app.
+            if crate::endpoint::AVAILABLE {
+                super::agents::terminal(ui, form, skin);
+            }
         });
 }
 
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 5b28dfa..10b4eaf 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -1875,8 +1875,10 @@ fn letters(
         let on_list = !crate::ui::focus::on_control(ctx);
         if let Some(conn) = &selected {
             if on_list {
-                let again = on(Command::ConnectAgain).count > 0;
-                if again || on(Command::Connect).count > 0 {
+                // A press, not what a held Enter repeats: one held in a box
+                // that closed on it is no connect.
+                let again = on(Command::ConnectAgain).fresh;
+                if again || on(Command::Connect).fresh {
                     actions.push(connect(conn, again));
                 }
             }
@@ -1932,6 +1934,9 @@ fn letters(
                 (Some(Command::DeleteConnection), Some(conn)) if on_list => {
                     actions.push(Action::DeleteConnection(conn.clone()));
                 }
+                (Some(Command::ChangeAgentAccess), Some(conn)) if crate::endpoint::AVAILABLE => {
+                    actions.push(Action::OpenAgentAccess(conn.clone()));
+                }
                 _ => {}
             }
             waiting.clear();
@@ -2253,6 +2258,17 @@ fn letters(
             (Some(Command::TurnAgentsOn), _) if agents_shows(app, tab, true) => {
                 actions.push(Action::TurnAgentsOn);
             }
+            (Some(Command::AllowAgents), _)
+                if agents_view(app, tab) == Some(crate::model::AgentsView::Activity)
+                    && app.agents_here(tab) == Some(crate::model::AgentsHere::OffHere) =>
+            {
+                if let Some(workspace) = app.workspace(tab)
+                    && app.connections.get(&workspace.conn_id).is_some()
+                {
+                    let conn = workspace.conn_id.clone();
+                    actions.push(Action::EditAgents { conn, allow: true });
+                }
+            }
             (Some(Command::ShowAgentsLog), _) if agents_view(app, tab).is_some() => {
                 // `l` shows the log, and the activity again.
                 let view = match agents_view(app, tab) {
diff --git a/src/ui/menu.rs b/src/ui/menu.rs
index 19622ec..a0ee8bd 100644
--- a/src/ui/menu.rs
+++ b/src/ui/menu.rs
@@ -309,6 +309,23 @@ impl<'a> Item<'a> {
     }
 }
 
+/// A heading over a part of a menu: a rule above it and its words muted,
+/// with nothing to pick (the Connections list's Agents part).
+pub fn heading(ui: &mut Ui, text: &str, look: &Look, palette: &Palette) {
+    let shape = Shape::of(ui, look);
+    let role = widgets::secondary(look);
+    let width = 2.0 * shape.inset + role.width(ui.ctx(), look.faces, text);
+    let (rect, _) = ui.allocate_at_least(vec2(width, shape.height), Sense::hover());
+    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.border);
+    widgets::paint_text(
+        ui,
+        rect.left() + shape.inset,
+        rect.center().y,
+        Text::one(look, role, text, palette.dim),
+    );
+    widgets::announce(ui, rect, text);
+}
+
 /// A menu's rows: no room between them, at least `width` across, and a
 /// scroll bar once they are taller than the look lets a menu grow.
 fn rows<R>(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 079da72..9e28d82 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -2,6 +2,7 @@
 //! state directly.
 
 pub mod about;
+pub mod agent_access;
 pub mod agents;
 // Where agents can reach the app: elsewhere nothing of them shows, which
 // the last test of `agents` holds.
@@ -89,6 +90,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
     settings::show(app, &ui.ctx().clone());
     write_prompts::show(app, &ui.ctx().clone());
     conflict_prompt::show(app, &ui.ctx().clone());
+    agent_access::show(app, &ui.ctx().clone());
     // Last: it says what was done, whatever is on screen by now.
     toast::show(app, ui.ctx());
 }
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
index c100230..161a730 100644
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -152,6 +152,9 @@ fn ssh_label(connection: &SavedConnection) -> String {
     }
 }
 
+/// The terminal look's note at the footer's right.
+const TERMINAL_SECRETS: &str = "secrets: keyring";
+
 pub fn show(app: &mut App, ui: &mut egui::Ui) {
     let locale = app.locale;
     let palette = app.palette;
@@ -238,6 +241,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
         key(Command::MoveInConnections),
         key(Command::FilterConnections),
     );
+    let agent_access = key(Command::ChangeAgentAccess);
     let new = if look.terminal {
         ButtonSpec::new(&new_label)
             .primary()
@@ -337,7 +341,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
     let footer_side = if look.terminal { 12.0 } else { side };
     let small = widgets::secondary(&look);
     if look.terminal {
-        let hints = [
+        let mut hints = vec![
             (&moves, "move", true),
             (&connect, "connect / show", true),
             (&edit, "edit", true),
@@ -346,6 +350,18 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
             (&delete, "delete", true),
             (&filter, "filter", true),
         ];
+        // Where agents can reach the app.
+        if crate::endpoint::AVAILABLE {
+            hints.push((&agent_access, "agent access", true));
+        }
+        // What does not fit beside the keyring's note gives way, from the
+        // end: the window may be as narrow as 720.
+        let note = widgets::measure(ui, Text::one(&look, small, TERMINAL_SECRETS, palette.dim));
+        let room = footer.width() - 2.0 * footer_side - note - 24.0;
+        while hints.len() > 1 && widgets::key_hints_width(ui, &hints, 16.0, &look, &palette) > room
+        {
+            hints.pop();
+        }
         widgets::key_hints(
             ui,
             (footer.left() + footer_side, footer_y),
@@ -369,7 +385,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
         }
     }
     let secrets = if look.terminal {
-        "secrets: keyring".to_owned()
+        TERMINAL_SECRETS.to_owned()
     } else if look.faces == theme::Faces::Plex {
         gettext(locale, "Passwords are stored in the macOS Keychain").into_owned()
     } else {
@@ -473,6 +489,8 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
                                 Opening {
                                     tab,
                                     open: app.tab_showing(&connection.id),
+                                    agents: crate::endpoint::AVAILABLE
+                                        .then(|| app.agents_access(connection)),
                                 },
                                 skin,
                                 &mut actions,
@@ -503,6 +521,8 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
                                 Opening {
                                     tab,
                                     open: app.tab_showing(&connection.id),
+                                    agents: crate::endpoint::AVAILABLE
+                                        .then(|| app.agents_access(connection)),
                                 },
                                 skin,
                                 &mut actions,
@@ -609,6 +629,9 @@ struct RowSkin<'a> {
 struct Opening {
     tab: crate::model::ConnTabId,
     open: Option<crate::model::ConnTabId>,
+    /// What agents may do on it, as in force; `None` where agents cannot
+    /// reach the app, which then shows nothing of them.
+    agents: Option<crate::connections::AgentAccess>,
 }
 
 impl Opening {
@@ -709,10 +732,64 @@ fn row_response(
         if item(ui, "Delete") {
             actions.push(Action::DeleteConnection(id.clone()));
         }
+        if let Some(access) = opening.agents {
+            agents_part(ui, &id, access, (look, palette, locale), actions);
+        }
     });
     response
 }
 
+/// The Agents part of a row's menus: what agents may do on it, the
+/// editor at its Agents part, and its Agents tab.
+fn agents_part(
+    ui: &mut egui::Ui,
+    conn: &ConnectionId,
+    access: crate::connections::AgentAccess,
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+    actions: &mut Vec<Action>,
+) {
+    use crate::connections::AgentAccess;
+    menu::heading(ui, &gettext(locale, "Agents"), look, palette);
+    for (level, word) in [(AgentAccess::Off, "Off"), (AgentAccess::Read, "Read")] {
+        let text = gettext(locale, word);
+        let name = format!("{} {text}", gettext(locale, "Agents"));
+        if menu::Item::choice(&text, access == level)
+            .name(&name)
+            .show(ui, look, palette)
+            .clicked()
+            && access != level
+        {
+            actions.push(Action::SetAgentAccess {
+                conn: conn.clone(),
+                access: level,
+            });
+        }
+    }
+    let limits = gettext(locale, "Tables, columns and limits…");
+    if menu::Item::action(&limits)
+        .show(ui, look, palette)
+        .clicked()
+    {
+        actions.push(Action::EditAgents {
+            conn: conn.clone(),
+            allow: false,
+        });
+    }
+    let show = gettext(locale, "Show Agents tab");
+    if menu::Item::action(&show).show(ui, look, palette).clicked() {
+        actions.push(Action::ShowAgentsOn(conn.clone()));
+    }
+}
+
+/// What agents may do, in a word: Off or Read.
+fn agents_level(access: crate::connections::AgentAccess, locale: crate::i18n::Locale) -> String {
+    match access {
+        crate::connections::AgentAccess::Off => gettext(locale, "Off"),
+        crate::connections::AgentAccess::Read => gettext(locale, "Read"),
+    }
+    .into_owned()
+}
+
 /// The words on the button that chooses a row, and its accessible name:
 /// Show for a connection that is open, Connect for another.
 fn choose_labels(
@@ -800,12 +877,24 @@ fn mac_row(
     let bottom = rect.top() + 42.0;
     // The columns: a 6 pt bar, 300 for the name, the host taking the rest,
     // 150 for TLS and SSH, 190 for last use, 110 of room, 250 of actions.
-    let fixed = 6.0 + 300.0 + 150.0 + 190.0 + 110.0 + 250.0;
-    let host_width = (rect.width() - fixed).max(120.0);
+    // Where agents can reach the app, their 140 come out of the room and
+    // the host while the host keeps 160; in a narrower window the column
+    // steps back, and the row is as it was. The menu has the Agents part.
+    let rest = rect.width() - (6.0 + 300.0 + 150.0 + 190.0 + 250.0);
+    let agents_width = match opening.agents {
+        Some(_) if rest >= 140.0 + 160.0 => 140.0,
+        _ => 0.0,
+    };
+    let room = match agents_width > 0.0 {
+        true => (rest - 140.0 - 160.0).min(110.0),
+        false => 110.0,
+    };
+    let host_width = (rest - agents_width - room).max(120.0);
     let name_x = rect.left() + 6.0 + 16.0;
     let host_x = rect.left() + 306.0 + 12.0;
     let security_x = rect.left() + 306.0 + host_width + 12.0;
-    let used_x = rect.left() + 306.0 + host_width + 150.0 + 12.0;
+    let agents_x = security_x + 150.0;
+    let used_x = agents_x + agents_width;
     let name_width = widgets::paint_text(
         ui,
         name_x,
@@ -907,6 +996,24 @@ fn mac_row(
         bottom,
         Text::one(look, small, &ssh_label(connection), palette.dim),
     );
+    // What agents may do: the word over the level, dimmed when Off.
+    if let Some(access) = opening.agents.filter(|_| agents_width > 0.0) {
+        let level = agents_level(access, locale);
+        let color = match access {
+            crate::connections::AgentAccess::Off => palette.dim,
+            crate::connections::AgentAccess::Read => palette.text,
+        };
+        let word = gettext(locale, "Agents");
+        widgets::paint_label(
+            ui,
+            agents_x,
+            top,
+            Text::one(look, small, &word, palette.dim),
+        );
+        widgets::paint_label(ui, agents_x, bottom, Text::one(look, small, &level, color));
+        let place = Rect::from_min_max(pos2(agents_x, rect.top()), pos2(used_x, rect.bottom()));
+        widgets::announce(ui, place, &format!("{word} {level}"));
+    }
     let when = crate::connections::when(store.last_used(&connection.id), now);
     widgets::paint_label(
         ui,
@@ -1000,6 +1107,9 @@ fn mac_row(
         {
             actions.push(Action::DeleteConnection(id.clone()));
         }
+        if let Some(access) = opening.agents {
+            agents_part(ui, &id, access, (look, palette, locale), actions);
+        }
     });
 }
 
@@ -1095,6 +1205,21 @@ fn terminal_row(
         Some(ssh) => format!("ssh {}", ssh.host),
         None => "no ssh".into(),
     };
+    // What agents may do, at the line's end: "· agents read".
+    let (agents, agents_color) = match opening.agents {
+        Some(access) => (
+            format!(
+                " · {} {}",
+                gettext(locale, "agents"),
+                agents_level(access, locale).to_lowercase()
+            ),
+            match access {
+                crate::connections::AgentAccess::Off => palette.dim,
+                crate::connections::AgentAccess::Read => palette.text,
+            },
+        ),
+        None => (String::new(), palette.dim),
+    };
     // "dev@localhost:5433/bookshop_development · tls verify-full · no ssh",
     // the separators kept with their neighbours as the design writes them.
     widgets::paint_label(
@@ -1106,7 +1231,8 @@ fn terminal_row(
             .space(small, " ")
             .add(small, &tls, tls_color)
             .space(small, " ")
-            .add(small, &format!("· {ssh}"), palette.dim),
+            .add(small, &format!("· {ssh}"), palette.dim)
+            .add(small, &agents, agents_color),
     );
     let when = crate::connections::when(store.last_used(&connection.id), now).to_lowercase();
     widgets::paint_text_right(
@@ -1156,6 +1282,7 @@ mod tests {
         let closed = Opening {
             tab: picker,
             open: None,
+            agents: None,
         };
         assert!(connects(closed.choose(&connection)));
         // Open: choosing it shows that tab, and it can still connect again
@@ -1163,6 +1290,7 @@ mod tests {
         let opened = Opening {
             tab: picker,
             open: Some(open),
+            agents: None,
         };
         assert!(matches!(
             opened.choose(&connection),
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index 8852e74..ce53263 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2029,6 +2029,7 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let review_key = key(Command::ReviewSql);
     let (agents_copy_key, agents_on_key) =
         (key(Command::CopyAgentCommand), key(Command::TurnAgentsOn));
+    let agents_allow_key = key(Command::AllowAgents);
     let (agents_move_key, agents_stop_key, agents_here_key) = (
         key(Command::MoveInAgentSessions),
         key(Command::StopAgentSession),
@@ -2057,6 +2058,11 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             _ => None,
         })
     });
+    // Whether its connection is saved: one saved no more has nothing to
+    // allow.
+    let agents_saved = app
+        .workspace(tab)
+        .is_some_and(|workspace| app.connections.get(&workspace.conn_id).is_some());
     // Whether its list has sessions, elsewhere ones too.
     let agents_listed = app
         .workspace(tab)
@@ -2250,6 +2256,7 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 structure_hints.push((&review_key, &*review, true));
             }
             let (copy_word, on_word) = (gettext(locale, "copy"), gettext(locale, "turn on"));
+            let allow_word = gettext(locale, "allow agents");
             let stop_words = [
                 gettext(locale, "sessions"),
                 gettext(locale, "stop session"),
@@ -2273,7 +2280,11 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     (&agents_here_key, &*stop_words[2], true),
                     (&agents_copy_key, &*copy_word, true),
                 ],
-                Some(crate::model::AgentsHere::OffHere) | None => Vec::new(),
+                Some(crate::model::AgentsHere::OffHere) if agents_saved => {
+                    vec![(&agents_allow_key, &*allow_word, true)]
+                }
+                Some(crate::model::AgentsHere::OffHere) => Vec::new(),
+                None => Vec::new(),
             };
             let log_words = [
                 gettext(locale, "log"),
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib ui::agents_tests`, then `cargo test --locked --lib ui::`, `cargo test --locked --lib keymap::`, `cargo test --locked --test keys` and `docs/bin/check`

Expected: PASS, and the website's check ends `HTML-Proofer finished successfully.`

```text
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 2484 filtered out
test result: ok. 1472 passed; 0 failed; 19 ignored; 0 measured; 1026 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 2494 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Look at it.** Render, by eye, in a throwaway scene in `src/shots.rs`: the list with the "…" menu open, Omarchy's box, and the editor opened at its Agents section, in both looks. Remove it before the commit. No picture is committed.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src docs/_reference/connections.md docs/_guide/omarchy.md
git commit -m "Show and change what agents may do from the list, the editor and the tab"
```

---

### Task 3: The master plan says what was built

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: Correct it**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index d5f3276..e8486b9 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -1500,6 +1500,28 @@ access`.
 | **Show Agents tab** (added on 2026-10-13). macOS: an item of the "…" menu's Agents part. Omarchy: `t` in the box that `a` opens. It opens the connection's window when it has none, and shows its Agents tab | `Action::ShowAgents`, D.4 |
 | The row's Agents column counts the writes waiting on a connection that has no window open ("1 waiting") | Not drawn on a board (question 25) |
 
+**As built in the first part of Phase 1b's fifth run** (`2026-10-15-mcp-server-1b-5a-access.md`). Where it
+draws otherwise than the board:
+
+- `src/ui/menu.rs` nests no menu: the "…" menu and the right-click menu
+  end with an Agents part under a heading of its own, Off and Read with a
+  check on the one in force, **Tables, columns and limits…** and **Show
+  Agents tab**. "Read + approved writes" and "Writes still wait for your
+  approval each time." come with writes (Phase 2). No level is disabled
+  until then: Read is open to every connection.
+- The column's 140 come out of the row's spare room and the host while
+  the host keeps 160; in a narrower window the column steps back, and the
+  menu has the Agents part.
+- Omarchy's row says `· agents read` at the end of its second line.
+  Omarchy's box (`Dialog::AgentAccess`) is `j`/`k`, `enter` to set, `t`
+  for the Agents tab and `esc`; its foot says `a opens this · j/k · t
+  agents tab`. A plain Enter answers it; what a held one repeats does not
+  connect the row behind it once it closes. The list's footer hints give
+  way from the end where the window has no room for them.
+- What the list and the box show and set is what is in force: a change
+  made outside the app that is held shows as what was shown, and the
+  person's own Read keeps it (1b-3b).
+
 ### D.3 The connection editor
 
 Board: `macOS – Edit connection, agent access`, and the right half of the
@@ -1517,6 +1539,17 @@ Omarchy connections board.
 Phase 1b builds access, rows and timeout. Phase 1c adds Tables. Phase 3a
 adds Masked columns.
 
+**As built** (`2026-10-15-mcp-server-1b-5a-access.md`): the section after Safety in both looks, Off and Read
+(the third level comes with writes), "Agents use this connection as dev."
+where the connection has an environment, **Rows per answer** (1 to 5000)
+and **Statement timeout** in seconds (1 to 60), checked on Save, and the
+board's note with "Every connection starts with agents Off." for its last
+sentence. The form opens with the access in force; an access the person
+changed is theirs (a raise is shown, never held), and one left as it
+opened keeps what the file says, a change made outside staying held.
+**Allow agents…** and **Tables, columns and limits…** open the form at
+this section.
+
 ### D.4 The Agents tab
 
 Boards: `macOS – Agents tab, activity and approval`, `macOS – Agents tab, no
@@ -2105,7 +2138,7 @@ is off until the person turns it on.
 | Part | Runs | State on 2026-10-13 |
 |---|---|---|
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
-| 1b. The server over stdio, with its interface | 7, below | The first four are built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md` and `2026-10-14-mcp-server-1b-4b-log.md`). Nothing the others wait for is open |
+| 1b. The server over stdio, with its interface | 7, below | The first four and a half are built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md`, `2026-10-14-mcp-server-1b-4b-log.md` and `2026-10-15-mcp-server-1b-5a-access.md`). Nothing the others wait for is open |
 | 1b on Windows: the way in | 1 | After 1b (question 14) |
 | 1c. Hidden tables | to be cut in its plan | Waits for questions 8 and 13 |
 | 1d. HTTP with tokens | 1 | After stdio (question 9) |
@@ -2178,7 +2211,8 @@ can see what agents do and stop them.
 | **1b-3b. Changes made outside the app** (built: `2026-10-14-mcp-server-1b-3b-changes-outside.md`) | C.2: the agent settings the app last showed, kept in the state directory, and a change from outside that makes agents more able held back until the person keeps it | A program that edits the files cannot turn agents on unseen | C.2's cases |
 | **1b-4a. What agents are doing** (built: `2026-10-14-mcp-server-1b-4a-activity.md`) | `App::agents`'s feed, from `Server::calls`. The sessions that are here and "Also:". **Stop session** and **Stop agents here** | See each agent and each call as it happens, stop one agent or all on a connection | F.7: one connection, the stops, closing the tab |
 | **1b-4b. The log, and a new agent** (built: `2026-10-14-mcp-server-1b-4b-log.md`) | The Log view as a list for this connection, with All connections, and **Show log**. The notification for an agent that connects | Read the log, and hear of an agent that connects | F.7: the log's default. F.4's interface |
-| **1b-5. Letting agents in** | Settings › Agents, with the switch between panes. The Connections list's Agents column, its menu and Omarchy's box. The connection editor's Agents section, with **Allow agents…** and **Show Agents tab** pointing at them. The website: a new guide, and the reference pages | Everything Phase 1 draws, without a file edited by hand | The interface tests of Settings, Connections and the editor. The keymap. `docs/bin/check` |
+| **1b-5a. Letting agents in, per connection** (built: `2026-10-15-mcp-server-1b-5a-access.md`) | The Connections list's Agents column, its menu and Omarchy's box. The connection editor's Agents section, with **Allow agents…** in the Agents tab and **Show Agents tab** in the list. The reference pages | A connection is opened to agents, and closed, without a file edited by hand | The interface tests of Connections and the editor. The keymap. `docs/bin/check` |
+| **1b-5b. Settings › Agents** | Settings › Agents, with the switch between panes, **Show Agents tab** and **Show log** there, the Agents tab's **Other agents…** and **Settings…**. The website's guide to Agents | Everything Phase 1 draws | The interface tests of Settings. The keymap. `docs/bin/check` |
 
 - **A button arrives with what it opens.** The empty states of the third
   run have **Turn on** and **Copy** and no other action. **Show log** comes
````

- [ ] **Step 2: Check it**

Run: `grep -c "$(printf '\342\200\224')" docs/superpowers/plans/2026-10-13-mcp-server.md` (an em dash, written in octal so that this plan holds none)

Expected: `0`.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what the first part of its fifth run built"
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

Expected: all clean, and every test passes. On 2026-10-15, with both servers up: 23 test binaries, 3,466 passed, none failed, 22 ignored. The website's check ends `HTML-Proofer finished successfully.`

- [ ] **By hand, on Linux or macOS**: with the server on, set a connection to Read from the list's "…" menu (Omarchy: `a`, `j`, `enter`): the column says Read, `connections.toml` says `access = "read"`, and an agent sees it. Edit the connection: its Agents section shows Read; set rows to 50 and save. Stop agents here in its Agents tab, then **Allow agents…**: the editor opens at the section with Read chosen, and nothing changes until Save.

- [ ] **Open the pull request.**

## Self-review

- **Against the master plan's row for this run** (G, as cut on 2026-10-15): the Connections list's Agents column, its menu and Omarchy's box (Task 2), the connection editor's Agents section (Tasks 1 and 2), **Allow agents…** in the Agents tab and **Show Agents tab** in the list (Task 2), the reference pages (Task 2). Settings › Agents and the guide are 5b's, in "What this run leaves out".
- **Against D.2 and D.3.** The column, the menu with a check on the level in force and the editor's link, Omarchy's `agents read` and its box with `j`/`k`, `enter`, `esc` and `t`, `SetAgentAccess` saving through `save_connections`, the footer's `a agent access`; the editor's access, rows and timeout. What is drawn otherwise is in the table above and the master plan's "As built".
- **Against C.2 and 1b-3b.** No control keeps a raise the person did not choose: the list, the box and the editor show what is in force, and the person's own choice is what they raise. Tested: a held raise kept by the list's Read, the editor's new connection, the editor left as it opened.
- **Reviewed before it was written.** A reviewer was given the draft and asked to break it, with tests of its own. It found these, each fixed above with a test: the editor opened at its Agents section with the limits out of sight; a held Enter in the box connected the row behind it; Shift+Enter in the box set the level; `e allow agents` was offered for a connection saved no more; at 1024 "Last used" ran under the row's Edit; and the list's footer ran into its note at 720. It found the held-raise rules sound.
- **Names.** `agents_opened`, `agents_rows`, `agents_timeout`, `show_agents`, `agents_to_saved`, `SetAgentAccess`, `EditAgents`, `OpenAgentAccess`, `MoveAgentAccess`, `AgentAccessBox`, `Dialog::AgentAccess`, `set_agent_access`, `menu::heading`, `agents_part`, and the keymap's `AllowAgents`, `ChangeAgentAccess`, `MoveInAgentLevels`, `ShowAgentsFromBox`: each is used under the one name in every task.
- **Every commit stands.** After Task 1 the actions set what agents may do, and nothing in the app calls them. After Task 2 the list, the box, the editor and the tab do. Task 3 is the master plan's text.
- **No placeholders.** Every step has its code or its command, and what the command prints.
