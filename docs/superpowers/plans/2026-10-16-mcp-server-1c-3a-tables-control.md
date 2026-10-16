# MCP server, Phase 1c, run 3a: the editor's Tables control. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The connection editor's Agents part gains **Tables**, in both looks: **All**, **All except** or **Only**, the names as chips with a ×, **Add table…** for a typed name, and a hint that says honestly what agents are and are not told. What the person changes there is their own; what they leave as it opened keeps what the file says.

**Architecture:** `src/model.rs`'s `TablesChoice` holds the control's state: the mode and a list for each of "All except" and "Only", so that a switch and back finds its names. `ConnectionForm` carries it, the tables it opened with, and the name being typed. `App::save_dialog` compares the saved tables with the ones the form opened with, as it does the access, and keeps a change as shown (`Raised::Tables`). `src/ui/connect_dialog/agents.rs` draws the control with the choice the access row uses, a chip of its own, and the dialog's `ButtonSpec` and field.

**Tech Stack:** Rust 2024, egui and the app's own widgets, AccessKit for the tests.

---

## Where this sits

Phase 1c of `docs/superpowers/plans/2026-10-13-mcp-server.md`, hidden tables. 1c-1 to 1c-2b (pull requests 144, 146 and 147) built the checks, the server's side and the setting in `connections.toml`. Its third run was cut in two on 2026-10-16: this one, 1c-3a, the control with a typed name; 1c-3b, **Add table…**'s list from an open session, the "not in this database" mark, and the website's guide.

## What the design asks, and what is built

Boards: `macOS – Edit connection, agent access` and `Omarchy – Connections, agent access` (the design canvas, read on 2026-10-16).

| The board | Built | Why |
|---|---|---|
| The hint "Hidden tables don't exist for the agent, not even in the schema." | "Agents are told these names, never what is in them: the rows, the columns, and the views over them. A function from an extension can still read them. For strict isolation, use a database user without access." | Agreed on 2026-10-16. Agents are told the names (question 8), and B.3 asks for what a check in the app cannot stop |
| One hint under every mode | That hint under "All except"; under "Only": "Agents may read these tables and no others, the ones made later included. A function from an extension can still read the others. For strict isolation, use a database user without access."; none under "All" | Found by the review: under "Only" the names are the readable ones, and "Agents are told these names, never what is in them" says otherwise |
| macOS: Tables beside Masked columns, half the fieldset's width | Tables alone, the whole width | Masked columns are Phase 3a's |
| Omarchy: `tables  all except [customers x]`, the chosen word alone | The three words, as the access row has them, and the names on the line under them | Agreed on 2026-10-16: the keyboard chooses among the words as on every row of the editor today |
| Omarchy's footer: `h/l change` | No `h`/`l`; Tab and Enter or Space, as every row of the editor today | Agreed on 2026-10-16. The editor has `h`/`l` on no row: adding them is a change of every row's, for another time |
| **Add table…**: the connection's tables (the master plan's D.3) | A typed name | 1c-3b, agreed on 2026-10-16 |

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: views draw and push actions or change the form they are given, no design or pixel check is ever a test, and no screenshot is committed. The Agents part shows only where agents can reach the app (`cfg(any(unix, windows))`), so its tests are those systems'.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.

## What was run before this plan was written

Every task's state was built here, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` with both test servers (3,531 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. The control was looked at in both looks, light and dark, in throwaway scenes that are not kept. Not run: CI, and a hand test in a window.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| The hint | Under "All except": "Agents are told these names, never what is in them: the rows, the columns, and the views over them. A function from an extension can still read them. For strict isolation, use a database user without access." Under "Only", one of its own; under "All", none | Agreed on 2026-10-16 for "All except", against the board's sentence, which question 8 made untrue. "Only"'s was found by the review: there the names are the readable ones |
| 1c-3's cut | Two runs: this control with a typed name; then Add table…'s list, the "not in this database" mark and the guide | Agreed on 2026-10-16 |
| The Omarchy keyboard | Tab and Enter or Space, as every row of the editor today; no `h`/`l` | Agreed on 2026-10-16. The board's footer draws `h/l change`, which no row of the editor has: a change of every row's, for another time |
| The two lists | Each kept while the other is tried; the chosen one is saved | A switch to look at "Only" and back loses nothing |
| A name typed and not added | Added by Enter, by the field losing the keyboard, and by Save; one that does not read refuses the save | Found by the review: a name the person typed and saw was dropped by Save, and agents could read the table |
| Escape in the field | Closes the field, not the dialog | Found by the review: it closed the dialog and lost every edit |
| After Enter | The field stays open, with the keyboard, for the next name; on a name that does not read, it keeps the keyboard | Found by the review: the keyboard was left nowhere, and a second name was twenty Tab stops away |
| The person's change and the file's | A change in the editor is the person's own and shown, wider or narrower; tables left as they opened keep what the file says; where the tables in force changed while the form was open, the save of a change is refused and the form shows them as they are | As the access does, and found by the review: an edit of the list saved over what the file had hidden meanwhile |
| A name's error | Under the names, not in the dialog's message, which a good name then does not clear | Found by the review |

### What this run leaves out

- **Add table…**'s list from an open session's catalog, and a listed name that matches no table marked "not in this database": 1c-3b.
- The website's guide to hiding tables: 1c-3b. The reference page for the editor is here.
- Masked columns: Phase 3a.

## File structure

| File | What changes |
|---|---|
| `src/model.rs` | `TablesMode`, `TablesChoice`; `ConnectionForm::agents_tables`, `agents_tables_opened`, `agents_table_typed`; `agents_to_saved` writes the chosen tables |
| `src/app.rs` | The editor opens with the tables in force; its save keeps the person's change as shown, or the file's |
| `src/ui/connect_dialog/agents.rs` | The control in both looks, its chip, and the hint |
| `src/ui/agents_tests.rs` | The control, through the editor, in both looks |
| `crates/tabletist-db/src/agent/hidden.rs`, `src/connections/file.rs` | An entry's error for one dot too many, which the dialog now says |
| `docs/_reference/connections.md` | Tables in the editor |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | The hint, D.3 as built, 1c-3 in two |

---

### Task 1: The form's tables, and its save

- `TablesChoice` is the control's state: `of(&Tables)` and `tables()`, which read each other back (an empty "All except" is "All"); `names()` for the chosen mode; `add(written)`, which parses as the file does, adds a name once whatever its case, and says why one does not read; `remove(at)`.
- `ConnectionForm` gains `agents_tables`, `agents_tables_opened`, `agents_table_typed`, `agents_table_error` and `agents_table_focus`, and `take_typed_table`, which adds a name typed and not yet added. `agents_to_saved` writes `agents_tables.tables()`.
- The editor opens with the tables in force: the shown ones while a change made outside the app that hides less is held.
- Its save: a name still typed is added, or the save is refused with why it does not read. Tables left as they opened keep what the file says, held or not; tables the person changed are theirs, saved and kept as shown (`Raised::Tables`), wider or narrower. Where the tables in force changed while the form was open, a change of the person's is refused, and the form shows them as they are now.

**Files:**
- Modify: `src/model.rs`, `src/app.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 78dffec..ee4ea89 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -6917,6 +6917,80 @@ mod tests {
         );
     }
 
+    #[cfg(any(unix, windows))]
+    #[test]
+    fn tables_the_person_changes_in_the_editor_are_theirs_and_shown() {
+        use crate::connections::AgentAccess;
+        use tabletist_db::agent::Tables;
+        let (mut app, _dir) = app();
+        let conn = with_saved(&mut app);
+        app.serve_agents();
+        app.apply(Action::SetAgentAccess {
+            conn: conn.clone(),
+            access: AgentAccess::Read,
+        });
+        let customers = Tables::AllExcept(vec!["customers".parse().unwrap()]);
+        let file_says = |app: &mut App, tables: Tables| {
+            let mut saved = app.connections.get(&conn).unwrap().clone();
+            saved.agents.tables = tables;
+            app.connections.upsert(saved);
+            app.check_agents();
+        };
+        file_says(&mut app, customers.clone());
+        // The file hides less: held. The editor opens with what is in force.
+        file_says(&mut app, Tables::All);
+        assert_eq!(app.agents_held.len(), 1);
+        app.apply(Action::EditConnection(conn.clone()));
+        assert_eq!(form(&mut app).agents_tables.tables(), customers);
+        // Saved as it opened: the file's, and still held.
+        app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(
+            app.connections.get(&conn).unwrap().agents.tables,
+            Tables::All
+        );
+        assert_eq!(app.agents_held.len(), 1);
+        // Changed in the editor, wider: the person's own, shown, not held.
+        app.apply(Action::EditConnection(conn.clone()));
+        form(&mut app).agents_tables.mode = crate::model::TablesMode::All;
+        app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(
+            app.connections.get(&conn).unwrap().agents.tables,
+            Tables::All
+        );
+        assert!(app.agents_held.is_empty());
+        app.check_agents();
+        assert!(app.agents_held.is_empty(), "shown, so nothing to hold");
+        // Narrower: the same.
+        app.apply(Action::EditConnection(conn.clone()));
+        form(&mut app).agents_tables = crate::model::TablesChoice::of(&customers);
+        app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(app.connections.get(&conn).unwrap().agents.tables, customers);
+        file_says(&mut app, Tables::All);
+        assert_eq!(
+            app.agents_held.len(),
+            1,
+            "what the person hid is what was shown"
+        );
+        // The file hides more while the editor is open: a change in the
+        // editor is not saved over it, and the editor shows it as it is.
+        app.apply(Action::PutBackAgentChanges(app.agents_held.clone()));
+        app.apply(Action::EditConnection(conn.clone()));
+        let more = Tables::AllExcept(vec![
+            "customers".parse().unwrap(),
+            "payroll".parse().unwrap(),
+        ]);
+        file_says(&mut app, more.clone());
+        form(&mut app).agents_tables.add("orders").unwrap();
+        app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(app.connections.get(&conn).unwrap().agents.tables, more);
+        assert_eq!(form(&mut app).agents_tables.tables(), more);
+        let said = form(&mut app).message.clone().unwrap_or_default();
+        assert!(
+            said.starts_with("The tables were changed outside"),
+            "{said}"
+        );
+    }
+
     #[cfg(any(unix, windows))]
     #[test]
     fn a_change_outside_that_hides_less_is_held_and_the_editor_keeps_the_files_tables() {
diff --git a/src/model.rs b/src/model.rs
index 2b350a5..1998c8e 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -5251,6 +5251,52 @@ mod tests {
         assert!(back.ssh_secret_is_stale(), "another port, another server");
     }
 
+    #[test]
+    fn the_tables_control_keeps_a_list_for_each_mode_and_says_what_is_chosen() {
+        use tabletist_db::agent::Tables;
+        let names = |written: &[&str]| -> Vec<tabletist_db::agent::TableName> {
+            written.iter().map(|entry| entry.parse().unwrap()).collect()
+        };
+        // Each setting reads back as it was; an empty "All except" is All.
+        for tables in [
+            Tables::All,
+            Tables::AllExcept(names(&["customers", "sales.orders"])),
+            Tables::Only(names(&["books"])),
+            Tables::Only(Vec::new()),
+        ] {
+            assert_eq!(TablesChoice::of(&tables).tables(), tables);
+        }
+        assert_eq!(
+            TablesChoice::of(&Tables::AllExcept(Vec::new())).tables(),
+            Tables::All
+        );
+        let mut choice = TablesChoice::of(&Tables::AllExcept(names(&["customers"])));
+        // A name once, in any case; one that does not read is refused.
+        assert_eq!(choice.add("Customers"), Ok(()));
+        assert_eq!(choice.add(" sales.orders "), Ok(()));
+        assert!(choice.add("a.b.c").is_err());
+        assert_eq!(
+            choice.tables(),
+            Tables::AllExcept(names(&["customers", "sales.orders"]))
+        );
+        // "Only" has a list of its own, and "All except" finds its again.
+        choice.mode = TablesMode::Only;
+        assert_eq!(choice.tables(), Tables::Only(Vec::new()));
+        choice.add("books").unwrap();
+        choice.mode = TablesMode::AllExcept;
+        choice.remove(0);
+        choice.remove(9);
+        assert_eq!(choice.tables(), Tables::AllExcept(names(&["sales.orders"])));
+        choice.remove(0);
+        assert_eq!(choice.tables(), Tables::All, "emptied, it hides nothing");
+        choice.mode = TablesMode::Only;
+        assert_eq!(choice.tables(), Tables::Only(names(&["books"])));
+        // "All" has no list to add to.
+        choice.mode = TablesMode::All;
+        assert_eq!(choice.add("orders"), Ok(()));
+        assert_eq!(choice.names(), None);
+    }
+
     #[test]
     fn an_edit_keeps_what_agents_may_do_on_the_connection() {
         let mut saved = ssh_form().to_saved().unwrap();
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- the_tables_control_keeps tables_the_person_changes`

Expected: it does not build. The tests name what is not there yet; among the errors:

```text
error[E0433]: cannot find `TablesMode` in `model`
error[E0433]: cannot find `TablesChoice` in `model`
error[E0609]: no field `agents_tables` on type `&mut model::ConnectionForm`
error: could not compile `tabletist` (lib test) due to 14 previous errors
```

- [ ] **Step 3: The choice, the form's fields, and the save**

````diff
diff --git a/src/app.rs b/src/app.rs
index ee4ea89..79e3cf4 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1959,6 +1959,9 @@ impl App {
                     // app is held: what the person was shown.
                     form.agents.access = self.agents_access(saved);
                     form.agents_opened = form.agents.access;
+                    let tables = self.agents_tables(saved);
+                    form.agents_tables = crate::model::TablesChoice::of(&tables);
+                    form.agents_tables_opened = tables;
                     self.dialog = Some(Dialog::Connection(Box::new(form)));
                     self.list_ssh_hosts();
                 }
@@ -3617,6 +3620,15 @@ impl App {
     }
 
     fn save_dialog(&mut self, connect: bool) {
+        // The tables in force now, of the connection being edited.
+        let tables_now = match &self.dialog {
+            Some(Dialog::Connection(form)) => form
+                .editing
+                .as_ref()
+                .and_then(|id| self.connections.get(id))
+                .map(|saved| self.agents_tables(saved)),
+            _ => None,
+        };
         let Some(Dialog::Connection(form)) = &mut self.dialog else {
             return;
         };
@@ -3624,6 +3636,34 @@ impl App {
         if !use_typed_url(form) {
             return;
         }
+        // A name typed and not yet added is the person's, as a URL typed
+        // is: added, or the save is refused and says why.
+        if let Err(why) = form.take_typed_table() {
+            form.message = Some(format!("The table's name {why}."));
+            form.show_agents = true;
+            return;
+        }
+        // Tables the file changed while the dialog was open are not saved
+        // over: what the person changes is what they saw. They are shown
+        // as they are now, to be changed again.
+        let tables_opened = form.agents_tables_opened.clone();
+        if let Some(now) = &tables_now {
+            use crate::model::TablesChoice;
+            let changed =
+                TablesChoice::of(&form.agents_tables.tables()) != TablesChoice::of(&tables_opened);
+            let moved = TablesChoice::of(now) != TablesChoice::of(&tables_opened);
+            if changed && moved {
+                form.agents_tables = TablesChoice::of(now);
+                form.agents_tables_opened = now.clone();
+                form.message = Some(
+                    "The tables were changed outside Tabletist while this was open. They show \
+                     as they are now: make your change again."
+                        .into(),
+                );
+                form.show_agents = true;
+                return;
+            }
+        }
         // The connection being edited was deleted while the dialog was open:
         // saving must not bring it back.
         if let Some(id) = &form.editing
@@ -3656,10 +3696,21 @@ impl App {
         // their own, and one they raised is shown, never held. The access
         // the dialog opened with keeps what the file says now: a change
         // made outside the app stays held, and one put back stays back.
-        // The tables have no control in the dialog yet: they are what the
-        // file says now, held or not.
+        // The tables likewise: those the person changed are their own and
+        // shown, wider or narrower; the same ones keep what the file says.
         if let Some(was) = self.connections.get(&saved.id) {
-            saved.agents.tables = was.agents.tables.clone();
+            let same = crate::model::TablesChoice::of(&saved.agents.tables)
+                == crate::model::TablesChoice::of(&tables_opened);
+            if same {
+                saved.agents.tables = was.agents.tables.clone();
+            } else {
+                self.agents_shown
+                    .keep(&[crate::agents_shown::Raised::Tables {
+                        id: saved.id.clone(),
+                        to: saved.agents.tables.clone(),
+                    }]);
+                self.save_agents_shown();
+            }
         }
         match self.connections.get(&saved.id) {
             Some(was) if saved.agents.access == opened => {
diff --git a/src/model.rs b/src/model.rs
index 1998c8e..a10b394 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -1007,6 +1007,106 @@ pub fn first_schema(schemas: &[String], database: &str) -> Option<String> {
         .cloned()
 }
 
+/// Which tables agents may read, as the editor's Tables control has it.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
+pub enum TablesMode {
+    #[default]
+    All,
+    AllExcept,
+    Only,
+}
+
+/// The Tables control's state: the mode chosen, and a list for each of
+/// the two that have one, so that a switch from one to the other and back
+/// finds its names again.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct TablesChoice {
+    pub mode: TablesMode,
+    pub hidden: Vec<tabletist_db::agent::TableName>,
+    pub only: Vec<tabletist_db::agent::TableName>,
+}
+
+impl TablesChoice {
+    pub fn of(tables: &tabletist_db::agent::Tables) -> Self {
+        use tabletist_db::agent::Tables;
+        match tables {
+            Tables::All => Self::default(),
+            Tables::AllExcept(names) if names.is_empty() => Self::default(),
+            Tables::AllExcept(names) => Self {
+                mode: TablesMode::AllExcept,
+                hidden: names.clone(),
+                only: Vec::new(),
+            },
+            Tables::Only(names) => Self {
+                mode: TablesMode::Only,
+                hidden: Vec::new(),
+                only: names.clone(),
+            },
+        }
+    }
+
+    /// What the control says: "All except" a list that is empty is "All".
+    pub fn tables(&self) -> tabletist_db::agent::Tables {
+        use tabletist_db::agent::Tables;
+        match self.mode {
+            TablesMode::All => Tables::All,
+            TablesMode::AllExcept if self.hidden.is_empty() => Tables::All,
+            TablesMode::AllExcept => Tables::AllExcept(self.hidden.clone()),
+            TablesMode::Only => Tables::Only(self.only.clone()),
+        }
+    }
+
+    /// The names the chosen mode lists, where it lists any.
+    pub fn names(&self) -> Option<&Vec<tabletist_db::agent::TableName>> {
+        match self.mode {
+            TablesMode::All => None,
+            TablesMode::AllExcept => Some(&self.hidden),
+            TablesMode::Only => Some(&self.only),
+        }
+    }
+
+    fn names_mut(&mut self) -> Option<&mut Vec<tabletist_db::agent::TableName>> {
+        match self.mode {
+            TablesMode::All => None,
+            TablesMode::AllExcept => Some(&mut self.hidden),
+            TablesMode::Only => Some(&mut self.only),
+        }
+    }
+
+    /// Adds `written` to the chosen mode's list, once: a name already
+    /// there in another case is the same name. The error says what is
+    /// wrong with it.
+    pub fn add(&mut self, written: &str) -> Result<(), String> {
+        let name: tabletist_db::agent::TableName = written.trim().parse()?;
+        let key = |name: &tabletist_db::agent::TableName| name.to_string().to_lowercase();
+        let Some(names) = self.names_mut() else {
+            return Ok(());
+        };
+        if !names.iter().any(|known| key(known) == key(&name)) {
+            names.push(name);
+        }
+        Ok(())
+    }
+
+    /// Takes the name at `at` out of the chosen mode's list.
+    pub fn remove(&mut self, at: usize) {
+        if let Some(names) = self.names_mut()
+            && at < names.len()
+        {
+            names.remove(at);
+        }
+    }
+}
+
+/// Where the Tables control puts the keyboard on the next frame.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum TableFocus {
+    /// The field for a name: opened, or still open after a name.
+    Field,
+    /// Add table…, once the field is closed with Escape.
+    Add,
+}
+
 /// The connection dialog's fields while it is open.
 pub struct ConnectionForm {
     /// `Some` when editing a saved connection, `None` for a new one.
@@ -1025,6 +1125,17 @@ pub struct ConnectionForm {
     /// The access in force when the dialog opened: a choice other than it
     /// is the person's own, and the same one keeps what the file says.
     pub agents_opened: crate::connections::AgentAccess,
+    /// The Tables control: what agents may not read.
+    pub agents_tables: TablesChoice,
+    /// The tables in force when the dialog opened: others are the
+    /// person's own, and the same keep what the file says.
+    pub agents_tables_opened: tabletist_db::agent::Tables,
+    /// The name being typed after Add table…, while its field is open.
+    pub agents_table_typed: Option<String>,
+    /// Why the name typed does not read, said under the names.
+    pub agents_table_error: Option<String>,
+    /// Where the Tables control puts the keyboard on the next frame.
+    pub agents_table_focus: Option<TableFocus>,
     /// Rows per answer, as typed.
     pub agents_rows: String,
     /// The statement's timeout in seconds, as typed.
@@ -1145,6 +1256,11 @@ impl Default for ConnectionForm {
             read_only: None,
             agents: crate::connections::Agents::default(),
             agents_opened: crate::connections::AgentAccess::Off,
+            agents_tables: TablesChoice::default(),
+            agents_tables_opened: tabletist_db::agent::Tables::All,
+            agents_table_typed: None,
+            agents_table_error: None,
+            agents_table_focus: None,
             agents_rows: crate::connections::Agents::default().rows.to_string(),
             agents_timeout: crate::connections::Agents::default()
                 .timeout_secs
@@ -1290,6 +1406,11 @@ impl ConnectionForm {
             read_only: saved.read_only,
             agents: saved.agents.clone(),
             agents_opened: saved.agents.access,
+            agents_tables: TablesChoice::of(&saved.agents.tables),
+            agents_tables_opened: saved.agents.tables.clone(),
+            agents_table_typed: None,
+            agents_table_error: None,
+            agents_table_focus: None,
             agents_rows: saved.agents.rows.to_string(),
             agents_timeout: saved.agents.timeout_secs.to_string(),
             show_agents: false,
@@ -1552,6 +1673,30 @@ impl ConnectionForm {
         })
     }
 
+    /// Takes into the Tables control a name typed and not yet added, as a
+    /// Save does: added, or `Err` with why it does not read, which is then
+    /// also said under the names.
+    pub fn take_typed_table(&mut self) -> Result<(), String> {
+        let Some(typed) = self.agents_table_typed.clone() else {
+            return Ok(());
+        };
+        if typed.trim().is_empty() {
+            self.agents_table_typed = None;
+            return Ok(());
+        }
+        match self.agents_tables.add(&typed) {
+            Ok(()) => {
+                self.agents_table_typed = None;
+                self.agents_table_error = None;
+                Ok(())
+            }
+            Err(why) => {
+                self.agents_table_error = Some(why.clone());
+                Err(why)
+            }
+        }
+    }
+
     /// What agents may do, as the fields say it: the rows and the timeout
     /// in the ranges a connection may be given.
     pub fn agents_to_saved(&self) -> Result<crate::connections::Agents, String> {
@@ -1570,12 +1715,11 @@ impl ConnectionForm {
                     "Statement timeout is a number of seconds from 1 to {MAX_AGENT_TIMEOUT_SECS}."
                 )
             })?;
-        // The tables have no field here yet: the ones the form opened with.
         Ok(crate::connections::Agents {
             access: self.agents.access,
             rows,
             timeout_secs,
-            tables: self.agents.tables.clone(),
+            tables: self.agents_tables.tables(),
         })
     }
 }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- the_tables_control_keeps tables_the_person_changes a_change_outside_that_hides_less`

Expected: PASS.

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 2552 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/model.rs src/app.rs
git commit -m "Keep the editor's tables as the person changes them, and the file's where they do not"
```

---

### Task 2: The control, in both looks

- After the access and its line: **Tables**, the choice of **All**, **All except** and **Only** (the one the access row uses: a track on macOS, words on Omarchy), then for the two with a list each name as a chip with a ×, and **Add table…**, which opens a field (placeholder "table, or schema.table", heard as "Table to add") that takes the keyboard.
- In the field: Enter adds the name and keeps the field, with the keyboard, for the next; a name the field holds when it loses the keyboard (Tab, a click) is added; Escape closes the field and gives the keyboard to Add table…, and is not the dialog's. A name that does not read is said under the names ("The table's name has more than one dot: …") and stays in the field, which keeps the keyboard on Enter. An empty field closes.
- The × is a button of its own, which screen readers hear as "Remove customers". The chip's text is the dialog's code role, as the board's 12 pt.
- The hint, agreed on 2026-10-16, under an "All except" list; one of its own under "Only"; none under "All".
- Omarchy: the words on the row, the names on the line under them, the hint after them. Every part is a Tab stop and Enter or Space presses it, as on every row of the editor.
- The entry's errors read after "The table's name": "has more than one dot: it is a table, or a schema and a table", "has nothing before or after a dot", "is empty".

**Files:**
- Modify: `src/ui/connect_dialog/agents.rs`, `src/ui/agents_tests.rs`, `crates/tabletist-db/src/agent/hidden.rs`, `src/connections/file.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/src/connections/file.rs b/src/connections/file.rs
index 566fdf1..737fa43 100644
--- a/src/connections/file.rs
+++ b/src/connections/file.rs
@@ -1018,8 +1018,8 @@ timeout_secs = 30
         let entry = said("hidden = [\"customers\", \"a.b.c\"]");
         assert_eq!(
             entry,
-            "connection \"Bookshop on disk\": agents' `hidden` entry 2 is a table, or a schema \
-             and a table, with one dot between"
+            "connection \"Bookshop on disk\": agents' `hidden` entry 2 has more than one dot: it \
+             is a table, or a schema and a table"
         );
         let secret = said("hidden = [\"hunter2\\\"x\"]");
         assert!(!secret.contains("hunter2"), "{secret}");
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index 763a200..8cbf39f 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -1147,6 +1147,165 @@ fn the_editors_agents_part_sets_access_rows_and_timeout() {
     }
 }
 
+/// The connection dialog's form.
+#[cfg(any(unix, windows))]
+fn form(harness: &Harness) -> &crate::model::ConnectionForm {
+    match &harness.app.dialog {
+        Some(crate::model::Dialog::Connection(form)) => form,
+        _ => panic!("no connection dialog"),
+    }
+}
+
+#[cfg(any(unix, windows))]
+fn form_mut(harness: &mut Harness) -> &mut crate::model::ConnectionForm {
+    match &mut harness.app.dialog {
+        Some(crate::model::Dialog::Connection(form)) => form,
+        _ => panic!("no connection dialog"),
+    }
+}
+
+/// Opens the editor at its Agents part, and waits for its scroll.
+#[cfg(any(unix, windows))]
+fn at_agents(harness: &mut Harness, conn: &crate::connections::ConnectionId) {
+    harness.app.apply(Action::EditAgents {
+        conn: conn.clone(),
+        allow: false,
+    });
+    for _ in 0..20 {
+        harness.settle();
+    }
+}
+
+/// `name` typed for Add table… (its field opened where it is not open
+/// already), then Enter.
+#[cfg(any(unix, windows))]
+fn add_table(harness: &mut Harness, look: &Look, name: &str) {
+    type_table(harness, look, name);
+    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+    harness.settle();
+}
+
+/// `name` typed in Add table…'s field, opened where it is not open.
+#[cfg(any(unix, windows))]
+fn type_table(harness: &mut Harness, look: &Look, name: &str) {
+    if form(harness).agents_table_typed.is_none() {
+        harness.click(&look.label("Add table…"));
+    }
+    harness.settle();
+    harness.frame(vec![egui::Event::Text(name.into())]);
+    harness.settle();
+}
+
+#[cfg(any(unix, windows))]
+#[test]
+fn the_editors_tables_control_lists_what_agents_may_not_read() {
+    use tabletist_db::agent::Tables;
+    let names = |written: &[&str]| -> Vec<tabletist_db::agent::TableName> {
+        written.iter().map(|entry| entry.parse().unwrap()).collect()
+    };
+    let too_many =
+        "The table's name has more than one dot: it is a table, or a schema and a table.";
+    for look in looks() {
+        let (mut harness, conn) = on_the_list(look);
+        at_agents(&mut harness, &conn);
+        harness.click(&look.label("Read"));
+        harness.click(&look.label("All except"));
+        harness.settle();
+        // Enter adds a name and keeps the field, with the keyboard, for
+        // the next.
+        add_table(&mut harness, &look, "customers");
+        add_table(&mut harness, &look, "sales.orders");
+        assert_eq!(form(&harness).agents_table_typed.as_deref(), Some(""));
+        // The same name again, in another case, is there once.
+        add_table(&mut harness, &look, "Customers");
+        // One that does not read is said under the names, and the field
+        // keeps it to be put right.
+        add_table(&mut harness, &look, "a.b.c");
+        assert!(harness.has(&look.label(too_many)), "{}", look.name);
+        assert_eq!(form(&harness).agents_table_typed.as_deref(), Some("a.b.c"));
+        // Escape closes the field and nothing else.
+        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
+        harness.settle();
+        assert!(harness.app.dialog.is_some(), "{}", look.name);
+        assert_eq!(form(&harness).agents_table_typed, None, "{}", look.name);
+        assert!(!harness.has(&look.label(too_many)), "{}", look.name);
+        harness.click(&look.label("Remove sales.orders"));
+        harness.settle();
+        // "Only" keeps a list of its own, and "All except" finds its again.
+        harness.click(&look.label("Only"));
+        harness.settle();
+        assert!(
+            !harness.has(&look.label("Remove customers")),
+            "{}",
+            look.name
+        );
+        let only = "Agents may read these tables and no others, the ones made later included. A \
+                    function from an extension can still read the others. For strict isolation, \
+                    use a database user without access.";
+        assert!(harness.has(&look.label(only)), "{}", look.name);
+        add_table(&mut harness, &look, "books");
+        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
+        harness.click(&look.label("All except"));
+        harness.settle();
+        assert!(
+            harness.has(&look.label("Remove customers")),
+            "{}",
+            look.name
+        );
+        // A name typed and not yet added is added by the save.
+        type_table(&mut harness, &look, "payroll");
+        harness.app.apply(Action::SaveConnection { connect: false });
+        let saved = harness.app.connections.get(&conn).unwrap();
+        assert_eq!(
+            saved.agents.tables,
+            Tables::AllExcept(names(&["customers", "payroll"])),
+            "{}",
+            look.name
+        );
+        // The person's own: shown, so that comparing again holds nothing.
+        harness.app.check_agents();
+        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
+        // Opened again it shows them. A name that does not read, typed and
+        // not added, refuses the save and says why.
+        at_agents(&mut harness, &conn);
+        assert!(
+            harness.has(&look.label("Remove customers")),
+            "{}",
+            look.name
+        );
+        type_table(&mut harness, &look, "a.b.c");
+        harness.app.apply(Action::SaveConnection { connect: false });
+        assert!(harness.app.dialog.is_some(), "{}", look.name);
+        assert_eq!(
+            form(&harness).message.as_deref(),
+            Some(too_many),
+            "{}",
+            look.name
+        );
+        form_mut(&mut harness).agents_table_typed = None;
+        form_mut(&mut harness).agents_table_error = None;
+        harness.settle();
+        // A name left in the field when Tab moves on is added.
+        type_table(&mut harness, &look, "orders");
+        harness.press(egui::Key::Tab, egui::Modifiers::NONE);
+        harness.settle();
+        assert!(harness.has(&look.label("Remove orders")), "{}", look.name);
+        assert_eq!(form(&harness).agents_table_typed, None, "{}", look.name);
+        // Widened by the person: theirs, and never held.
+        harness.click(&look.label("All"));
+        harness.settle();
+        harness.app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(
+            harness.app.connections.get(&conn).unwrap().agents.tables,
+            Tables::All,
+            "{}",
+            look.name
+        );
+        harness.app.check_agents();
+        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
+    }
+}
+
 #[cfg(any(unix, windows))]
 #[test]
 fn allow_agents_opens_the_editor_with_read_chosen() {
````

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked --lib the_editors_tables_control`

Expected: FAIL. The editor has no Tables control yet, so the test finds nothing to click:

```text
test ui::agents_tests::the_editors_tables_control_lists_what_agents_may_not_read ... FAILED
thread 'ui::agents_tests::the_editors_tables_control_lists_what_agents_may_not_read' panicked at src/testing.rs:243:32:
nothing labelled "All except": [...]
```

- [ ] **Step 3: The control**

````diff
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index 4ca325f..cae1512 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -71,6 +71,9 @@ impl std::str::FromStr for TableName {
     type Err = String;
 
     fn from_str(written: &str) -> Result<Self, String> {
+        if written.is_empty() {
+            return Err("is empty".into());
+        }
         let mut parts: Vec<String> = Vec::new();
         let mut chars = written.chars().peekable();
         loop {
@@ -102,7 +105,7 @@ impl std::str::FromStr for TableName {
                 }
             }
             if part.is_empty() {
-                return Err("has a name that is empty".into());
+                return Err("has nothing before or after a dot".into());
             }
             parts.push(part);
             match chars.next() {
@@ -120,7 +123,9 @@ impl std::str::FromStr for TableName {
             }),
             Err(parts) => match <[String; 1]>::try_from(parts) {
                 Ok([name]) => Ok(Self { schema: None, name }),
-                Err(_) => Err("is a table, or a schema and a table, with one dot between".into()),
+                Err(_) => {
+                    Err("has more than one dot: it is a table, or a schema and a table".into())
+                }
             },
         }
     }
diff --git a/src/ui/connect_dialog/agents.rs b/src/ui/connect_dialog/agents.rs
index 3471671..9058166 100644
--- a/src/ui/connect_dialog/agents.rs
+++ b/src/ui/connect_dialog/agents.rs
@@ -1,20 +1,52 @@
 //! The connection editor's Agents part: what agents may do on the
-//! connection (Off or Read), how many rows an answer holds, and how long a
-//! statement may run. Phase 1b's rows of the boards: hidden tables come
-//! with 1c, masked columns with 3a, the third level with writes.
+//! connection (Off or Read), the tables they may not read, how many rows
+//! an answer holds, and how long a statement may run. The boards' rows but
+//! masked columns, which come with 3a, and the third level, with writes.
 
-use egui::{Align, Ui};
+use egui::{
+    Align, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, vec2,
+};
 
 use super::choice::{Choice, Group, choose};
 use super::{Skin, input};
 use crate::connections::AgentAccess;
-use crate::model::ConnectionForm;
-use crate::typography::TextRole;
-use crate::ui::widgets;
+use crate::model::{ConnectionForm, TableFocus, TablesMode};
+use crate::typography::{Text, TextRole};
+use crate::ui::widgets::{self, ButtonSpec};
 
 /// The levels, as the switch says them.
 const LEVELS: [(AgentAccess, &str); 2] = [(AgentAccess::Off, "Off"), (AgentAccess::Read, "Read")];
 
+/// The Tables control's modes, as it says them.
+const MODES: [(TablesMode, &str); 3] = [
+    (TablesMode::All, "All"),
+    (TablesMode::AllExcept, "All except"),
+    (TablesMode::Only, "Only"),
+];
+
+/// What the Tables control says under it, of a list that hides: what
+/// agents are and are not told, and what a check in the app cannot stop
+/// (the MCP plan's B.3). Not the board's "Hidden tables don't exist for the
+/// agent": agents are told the names (question 8). Agreed on 2026-10-16.
+const HIDDEN_HINT: &str = "Agents are told these names, never what is in them: the rows, the \
+                           columns, and the views over them. A function from an extension can \
+                           still read them. For strict isolation, use a database user without \
+                           access.";
+
+/// The same of an "Only" list, whose names are the ones agents may read.
+const ONLY_HINT: &str = "Agents may read these tables and no others, the ones made later \
+                         included. A function from an extension can still read the others. For \
+                         strict isolation, use a database user without access.";
+
+/// The hint for the mode chosen: none for "All", which hides nothing.
+fn tables_hint(form: &ConnectionForm) -> Option<&'static str> {
+    match form.agents_tables.mode {
+        TablesMode::All => None,
+        TablesMode::AllExcept => Some(HIDDEN_HINT),
+        TablesMode::Only => Some(ONLY_HINT),
+    }
+}
+
 /// What the part says under its rows, from the board.
 const NOTE: &str =
     "When Open read-only is on, agents get Read at most. Every connection starts with agents Off.";
@@ -41,6 +73,186 @@ fn access(ui: &mut Ui, form: &mut ConnectionForm, heading: &str, group: Group, s
     }
 }
 
+/// The Tables control's switch: All, All except or Only.
+fn tables_mode(ui: &mut Ui, form: &mut ConnectionForm, heading: &str, group: Group, skin: &Skin) {
+    let labels = MODES.map(|(_, word)| skin.say(word));
+    let options: Vec<(TablesMode, Choice<'_>)> = MODES
+        .iter()
+        .zip(&labels)
+        .map(|((mode, _), label)| (*mode, Choice::plain(label)))
+        .collect();
+    let current = form.agents_tables.mode;
+    if let Some(mode) = choose(
+        ui,
+        "agents-tables",
+        Some(heading),
+        &options,
+        current,
+        group,
+        skin,
+    ) {
+        form.agents_tables.mode = mode;
+        form.agents_table_typed = None;
+        form.agents_table_error = None;
+    }
+}
+
+/// The names the chosen mode lists, each with its × to take it out, and
+/// Add table… after them, which opens a field for a name to be typed.
+/// Enter adds the name and keeps the field open, with the keyboard, for
+/// the next; a name the field still holds when it loses the keyboard is
+/// added then; Escape closes the field and nothing else. A name that does
+/// not read is said under the names, and the field keeps it, and the
+/// keyboard, to be put right. Nothing for "All".
+fn table_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
+    let Some(names) = form.agents_tables.names() else {
+        return;
+    };
+    let written: Vec<String> = names.iter().map(ToString::to_string).collect();
+    let height = if skin.look.terminal { 21.0 } else { 24.0 };
+    let mut removed = None;
+    let focus = form.agents_table_focus.take();
+    ui.horizontal_wrapped(|ui| {
+        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
+        for (at, name) in written.iter().enumerate() {
+            if chip(ui, name, at, height, skin) {
+                removed = Some(at);
+            }
+        }
+        match form.agents_table_typed.as_mut() {
+            Some(typed) => {
+                let role = widgets::code(skin.look);
+                let id = ui.id().with("agents-table-typed");
+                let placeholder = super::hint(ui, &skin.say("table, or schema.table"), role, skin);
+                let field = ui.add_sized(
+                    vec2(200.0, height),
+                    input(ui, typed, role, height, skin)
+                        .id(id)
+                        .hint_text(placeholder),
+                );
+                let name = skin.say("Table to add");
+                ui.ctx()
+                    .accesskit_node_builder(field.id, |node| node.set_label(name.as_str()));
+                if focus == Some(TableFocus::Field) {
+                    field.request_focus();
+                }
+                let pressed = |key| ui.input(|input| input.key_pressed(key));
+                if field.lost_focus() && pressed(egui::Key::Escape) {
+                    // The field's own: the dialog is not closed by it.
+                    ui.input_mut(|input| {
+                        input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
+                    });
+                    form.agents_table_typed = None;
+                    form.agents_table_error = None;
+                    form.agents_table_focus = Some(TableFocus::Add);
+                } else if field.lost_focus() {
+                    let entered = pressed(egui::Key::Enter);
+                    let empty = typed.trim().is_empty();
+                    match form.take_typed_table() {
+                        // Enter on a name: the field stays, for the next.
+                        Ok(()) if entered && !empty => {
+                            form.agents_table_typed = Some(String::new());
+                            form.agents_table_focus = Some(TableFocus::Field);
+                        }
+                        Ok(()) => form.agents_table_typed = None,
+                        Err(_) if entered => form.agents_table_focus = Some(TableFocus::Field),
+                        Err(_) => {}
+                    }
+                }
+            }
+            None => {
+                let add = skin.say("Add table…");
+                let button = ButtonSpec::new(&add).padding(8.0).role(TextRole::pick(
+                    skin.look,
+                    TextRole::Secondary,
+                    TextRole::OBody,
+                ));
+                let response = button.show(ui, height, skin.look, skin.palette);
+                if focus == Some(TableFocus::Add) {
+                    response.request_focus();
+                }
+                if response.clicked() {
+                    form.agents_table_typed = Some(String::new());
+                    form.agents_table_focus = Some(TableFocus::Field);
+                }
+            }
+        }
+    });
+    if let Some(at) = removed {
+        form.agents_tables.remove(at);
+    }
+    if let Some(why) = &form.agents_table_error {
+        let said = skin.say("The table's name {why}.").replace("{why}", why);
+        ui.add_space(4.0);
+        let room = ui.available_width();
+        Text::one(
+            skin.look,
+            TextRole::pick(skin.look, TextRole::Secondary, TextRole::OSecondary),
+            &said,
+            skin.palette.danger,
+        )
+        .wrap(room)
+        .layout(ui.ctx())
+        .label(ui);
+    }
+}
+
+/// A listed name, `customers ×`, bordered; the × takes it out, and screen
+/// readers hear it as "Remove customers". Whether it was pressed.
+fn chip(ui: &mut Ui, name: &str, at: usize, height: f32, skin: &Skin) -> bool {
+    let Skin { look, palette, .. } = *skin;
+    let role = widgets::code(look);
+    let cross = if look.terminal { "x" } else { "×" };
+    let name_width = role.width(ui.ctx(), look.faces, name);
+    let cross_width = role.width(ui.ctx(), look.faces, cross);
+    let (left, gap, right) = (8.0, 6.0, 6.0);
+    let size = vec2(
+        (left + name_width + gap + cross_width + right).ceil(),
+        height,
+    );
+    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
+    let corner = CornerRadius::same(if look.terminal {
+        0
+    } else {
+        look.radius.saturating_sub(2)
+    });
+    let painter = ui.painter().clone();
+    painter.rect_filled(rect, corner, palette.panel);
+    painter.rect_stroke(
+        rect,
+        corner,
+        Stroke::new(1.0, palette.border),
+        StrokeKind::Inside,
+    );
+    let text = Text::one(look, role, name, palette.text).layout(ui.ctx());
+    let y = rect.center().y - text.galley.size().y / 2.0;
+    painter.galley(egui::pos2(rect.left() + left, y), text.galley, palette.text);
+    let cell = Rect::from_min_max(
+        egui::pos2(rect.right() - right - cross_width - gap / 2.0, rect.top()),
+        rect.right_bottom(),
+    );
+    let response = ui.interact(
+        cell,
+        ui.id().with(("agents-table-remove", at, name)),
+        Sense::click(),
+    );
+    let label = skin.say("Remove {name}").replace("{name}", name);
+    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), &label));
+    let ink = if response.hovered() || response.has_focus() {
+        palette.text
+    } else {
+        palette.faint
+    };
+    let mark = Text::one(look, role, cross, ink).layout(ui.ctx());
+    let y = rect.center().y - mark.galley.size().y / 2.0;
+    painter.galley(
+        egui::pos2(rect.right() - right - cross_width, y),
+        mark.galley,
+        ink,
+    );
+    response.clicked()
+}
+
 /// "Agents use this connection as dev.": the environment an agent's
 /// session opens as. Nothing for a connection with none.
 fn runs_as(skin: &Skin) -> Option<String> {
@@ -98,7 +310,20 @@ pub(super) fn sheet(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
             ui.add_space(6.0);
             sentence(ui, small, &said, skin);
         }
-        ui.add_space(10.0);
+        ui.add_space(12.0);
+        let tables = skin.say("Tables");
+        widgets::label(ui, small, &tables, palette.secondary, look);
+        ui.add_space(5.0);
+        tables_mode(ui, form, &tables, Group::Track(28.0), skin);
+        if form.agents_tables.names().is_some() {
+            ui.add_space(6.0);
+            table_names(ui, form, skin);
+        }
+        if let Some(hint) = tables_hint(form) {
+            ui.add_space(6.0);
+            sentence(ui, small, &skin.say(hint), skin);
+        }
+        ui.add_space(12.0);
         let (role, height) = (skin.value_role(), skin.field_height());
         let columns = [super::Column::Fr(1.0), super::Column::Fr(1.0)];
         super::row(ui, 12.0, &columns, |ui, column| {
@@ -130,6 +355,19 @@ pub(super) fn terminal(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
     super::terminal::terminal_row_of(ui, &heading, line, skin, |ui, _| {
         access(ui, form, &heading, Group::Words, skin);
     });
+    let tables = skin.say("Tables");
+    super::terminal::terminal_row_of(ui, &tables, line, skin, |ui, _| {
+        tables_mode(ui, form, &tables, Group::Words, skin);
+    });
+    if form.agents_tables.names().is_some() {
+        super::terminal::terminal_row_of(ui, "", line + 4.0, skin, |ui, _| {
+            table_names(ui, form, skin);
+        });
+    }
+    if let Some(hint) = tables_hint(form) {
+        sentence(ui, TextRole::OSecondary, &skin.say(hint), skin);
+        ui.add_space(4.0);
+    }
     let (role, height) = (TextRole::OBody, skin.field_height());
     super::terminal::terminal_row(ui, &skin.say("Rows per answer"), skin, |ui, label| {
         ui.add(input(ui, &mut form.agents_rows, role, height, skin))
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- the_editors_tables_control the_editors_agents_part connections::file`, then `cargo test --locked -p tabletist-db --lib an_entry`

Expected: PASS.

```text
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 2539 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 482 filtered out
```

- [ ] **Step 5: Look at it**

In a throwaway `#[test] #[ignore]` scene at the end of `src/shots.rs` (built on `both(name, |harness| ..)`, a saved connection Read and hiding two tables, `Action::EditAgents`, and eight `settle`s for the scroll), run `cargo test --locked --features shots --lib shots::<scene> -- --ignored` and read `target/shots/<scene>-{macos,omarchy}-{light,dark}.png` against the boards. Remove the scene, and keep no picture: no design check is a test, and none is committed.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/ui/connect_dialog/agents.rs src/ui/agents_tests.rs crates/tabletist-db/src/agent/hidden.rs src/connections/file.rs
git commit -m "Choose in the connection editor the tables agents may not read"
```

---

### Task 3: The reference page

- `docs/_reference/connections.md`: **Tables** in the editor's Agents section, its three choices, Add table… and ×, the two lists kept, and the hint's words.

**Files:**
- Modify: `docs/_reference/connections.md`

- [ ] **Step 1: The page**

````diff
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
index 0f607b2..38ce859 100644
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -163,9 +163,23 @@ server is on.
   its "…" menu changes it. Its **Tables, columns and limits…** opens the
   connection's **Agents** section, and **Show Agents tab** shows its
   Agents tab.
-- In the connection's form, the **Agents** section has the access, **Rows
-  per answer** (1 to 5000, 500 at first) and the **Statement timeout** in
-  seconds (1 to 60, 10 at first).
+- In the connection's form, the **Agents** section has the access,
+  **Tables**, **Rows per answer** (1 to 5000, 500 at first) and the
+  **Statement timeout** in seconds (1 to 60, 10 at first).
+- **Tables** is **All**, **All except** or **Only**. With **All except**,
+  agents may read every table but the ones listed; with **Only**, the ones
+  listed and no others, a table made later included. **Add table…** opens
+  a field for a name, `customers` or `sales.customers`: Enter adds it and
+  leaves the field open for the next, Escape closes the field, and **×**
+  takes a name out. A name still in the field when you save is added with
+  the rest. Each of the two lists is kept while you try the other, and
+  what you save is the one chosen. Agents are told the names in the list,
+  never what is in the hidden tables: not their rows, their columns, or
+  the views over them. A function from an extension can still read them;
+  for strict isolation, use a database user without access.
+- If the tables change in `connections.toml` while the form is open,
+  saving a change of yours to them is refused, and the form shows them as
+  they are now, for you to make your change again.
 - An Agents tab that says agents can't use a connection has **Allow
   agents…**, which opens that section with **Read** chosen. Nothing
   changes until you save.
````

- [ ] **Step 2: Check it**

Run: `docs/bin/check` and `grep -c "$(printf '\342\200\224')" docs/_reference/connections.md` (a count of em dashes)

Expected: the site checks clean, and no em dash (`0`):

```text
HTML-Proofer finished successfully.
```

- [ ] **Step 3: Commit**

```bash
git add docs/_reference/connections.md
git commit -m "Say in the reference what the editor's Tables control does"
```

---

### Task 4: The master plan

- B.3's hint as built, D.3 as built for 1c, G's row, and 1c-3 in two.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: The edits**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index f001daf..6be2d90 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -897,6 +897,25 @@ board's hint:
 The same sentence, with the list above, goes in the documentation page for
 agents.
 
+**As built** (`2026-10-16-mcp-server-1c-3a-tables-control.md`), agreed on 2026-10-16: the board's "Hidden tables
+don't exist for the agent, not even in the schema." is not said, as agents
+are told the names (question 8), and the hint is one sentence of the two,
+under an "All except" list:
+
+> Agents are told these names, never what is in them: the rows, the
+> columns, and the views over them. A function from an extension can still
+> read them. For strict isolation, use a database user without access.
+
+Under an "Only" list, whose names are the readable ones (found by the
+run's review): "Agents may read these tables and no others, the ones made
+later included. A function from an extension can still read the others.
+For strict isolation, use a database user without access." Under "All",
+none.
+
+What the checks of 1c-1 and 1c-2 left to an app-side check is shorter than
+the list above: a view and a function the users wrote are refused, and an
+extension's function is what is left.
+
 **As built in 1c's first run** (`2026-10-15-mcp-server-1c-1-checks.md`): the checks, in
 `crates/tabletist-db/src/agent/hidden.rs` and a `hidden.rs` for each
 server, behind a `tables` argument of `Connection::run_agent_read`. The app
@@ -1801,6 +1820,38 @@ opened keeps what the file says, a change made outside staying held.
 **Allow agents…** and **Tables, columns and limits…** open the form at
 this section.
 
+**As built for 1c** (`2026-10-16-mcp-server-1c-3a-tables-control.md`). Agreed on 2026-10-16, where it went
+otherwise:
+
+- **Tables**, after the access and its line, in both looks: **All**,
+  **All except** or **Only** (the choice the access uses), the names of the
+  chosen one as chips with a ×, and **Add table…**, which opens a field
+  for a name typed as the file writes it (`customers`, `sales.customers`).
+  Enter adds it and keeps the field, with the keyboard, for the next; a
+  name the field holds when it loses the keyboard, or when the form is
+  saved, is added too. Escape closes the field, not the dialog. A name
+  that does not read is said under the names, and the field keeps it; at
+  a save it refuses the save. Each of the two lists is kept while the
+  other is tried, and what is saved is the one chosen.
+- The form opens with the tables in force; tables the person changed are
+  theirs and shown, wider or narrower, and ones left as they opened keep
+  what the file says, a change made outside staying held, as the access
+  does. Where the tables in force changed while the form was open, a
+  change of the person's is not saved over them: the save is refused,
+  and the form shows them as they are now (found by the run's review: an
+  edit of the list would have undone what the file hid meanwhile).
+- **Add table…** takes a typed name only. The list from an open session's
+  catalog, and a name that matches nothing marked "not in this database",
+  are 1c-3b's.
+- **Omarchy**: the row shows the three words, as the access row does, and
+  the names on the line under them; the board draws the chosen word alone
+  with the names after it. The keyboard works the row as every row of the
+  editor today: Tab moves through the words, each name's ×, and add
+  table…, and Enter or Space picks or presses. No `h`/`l`: the board's
+  footer draws them, and the editor has them on no row, so they would be
+  a change of every row's, for another time.
+- The hint (B.3) is under the control, in both looks.
+
 ### D.4 The Agents tab
 
 Boards: `macOS – Agents tab, activity and approval`, `macOS – Agents tab, no
@@ -2398,7 +2449,7 @@ is off until the person turns it on.
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
 | 1b. The server over stdio, with its interface | 9, below | Built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md`, `2026-10-14-mcp-server-1b-4b-log.md`, `2026-10-15-mcp-server-1b-5a-access.md` and `2026-10-15-mcp-server-1b-5b-settings.md`) |
 | 1b on Windows: the way in | 1 | Built (`2026-10-15-mcp-server-1b-6-windows.md`) |
-| 1c. Hidden tables | 4, below | The checks, the server's side and the setting built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`, `2026-10-16-mcp-server-1c-2b-setting.md`). The editor's Tables control is next |
+| 1c. Hidden tables | 5, below | The checks, the server's side, the setting and the editor's control built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`, `2026-10-16-mcp-server-1c-2b-setting.md`, `2026-10-16-mcp-server-1c-3a-tables-control.md`). Add table…'s list and the website's guide are next |
 | 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |
 
 The move of the connections to `connections.toml` came between 1a and 1b
@@ -2521,6 +2572,10 @@ build; no console target.
   server's side with nothing to set it (`2026-10-15-mcp-server-1c-2a-server.md`); 1c-2b, the keys in
   `connections.toml`, the app passing them, changes made outside the app
   held, and the reference page for the keys (`2026-10-16-mcp-server-1c-2b-setting.md`).
+- The third cut in two too, on 2026-10-16: 1c-3a, the editor's Tables
+  control in both looks with a typed name, its save and its hint
+  (`2026-10-16-mcp-server-1c-3a-tables-control.md`); 1c-3b, Add table…'s list from an open session's catalog, the
+  "not in this database" mark, and the website's guide.
 - Files: `crates/tabletist-db/src/catalog.rs`, `pg.rs`, `mysql.rs`,
   `sqlite.rs` (what views read, routines, partitions), `agent/` (names,
   dependencies, the plan check, the system catalogs), `tabletist-sqlite-ffi`
````

- [ ] **Step 2: Check the whole of it**

Run: `cargo fmt --all --check`, clippy as above, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, the count of em dashes in the master plan, and `cargo test --locked --workspace --all-targets` with both servers

Expected: fmt and clippy clean, the documentation builds without a warning, no em dash (`0`), and:

```text
3531 passed, 0 failed, 22 ignored
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what Phase 1c's run 3a built"
```

## Self-review

- **Against the design.** The boards' Tables control in both looks, with the departures listed in "What the design asks, and what is built", each agreed on 2026-10-16 or found by the review. The master plan's D.3 rows for Tables and its hint, and B.3's sentence, corrected in Task 4.
- **A review before the plan.** A reviewer drove the editor headlessly in both looks and found, each with a run but the last: a typed name Save dropped, so a table the person meant to hide was readable; a list edit saved over a narrowing the file made while the form was open; Escape in the field closing the dialog; the keyboard left nowhere after Enter, and in no field after a bad name; Tab leaving a name pending; the hint wrong under "Only"; and an error left for a field gone, an add clearing other messages, a field without a name, and an odd error for a dot with nothing beside it. All fixed, with tests. It found sound: every setting opened and saved untouched gives back what the file held; a name once whatever its case; the two lists across switches; a list emptied by × saves "All"; a change held keeps the hold when nothing is changed; the copy of a connection; every part a Tab stop with the right name, in both looks.
- **Left, and said.** Add table… takes typed names only, and a listed name that matches nothing is not marked: 1c-3b.
- **Placeholders.** None: every step has its diff or its command and what it prints. Task 2's look at the control is by eye in a throwaway scene, as `AGENTS.md` has it, and leaves nothing behind.
- **Names.** `TablesMode`, `TablesChoice`, `TableFocus`, `agents_tables`, `agents_tables_opened`, `agents_table_typed`, `agents_table_error`, `agents_table_focus` and `take_typed_table` are the same in every task that uses them.
