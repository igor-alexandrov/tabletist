# MCP server, Phase 1c, run 3b: Add table…'s list, and a name in no table. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** While a window of the connection is open, **Add table…** lists the tables and views its sidebar has loaded that match what is typed, and a listed name that is in none of them is marked **not in this database**, with a line under an "All except" list that says agents are stopped until it is put right. The website's guide says how to hide tables, what it hides, and what it cannot. This is the last run of Phase 1c.

**Architecture:** `src/model.rs`'s `LoadedTables` holds what the connection's open windows loaded: the home schema, every schema listed, and each loaded schema's names. `offered` gives the matches for the field, and `has` says whether a listed name is a table where the loaded schemas can say. `App::loaded_tables` reads them from the windows' trees, as quick open does, and the dialog gives them to the form on each frame. The control draws the matches in an `egui::Area` under the field, moves through them with a keymap command of its own (`FormTableMove`), and marks a chip from `has`.

**Tech Stack:** Rust 2024, egui, the app's keymap and widgets, AccessKit for the tests.

---

## Where this sits

Phase 1c of `docs/superpowers/plans/2026-10-13-mcp-server.md`, hidden tables. 1c-1 to 1c-3a (pull requests 144, 146, 147 and 148) built the checks, the server's side, the setting and the editor's Tables control with a typed name. This run, 1c-3b, ends Phase 1c, and with 1d dropped, Phase 1.

## What the design asks, and what is built

Boards: `macOS – Edit connection, agent access`, `Omarchy – Connections, agent access`, and the Components boards (the design canvas, read on 2026-10-16). The boards draw Add table… as a button and no list or mark for it; the Components boards' picker is the reference.

| The design | Built | Why |
|---|---|---|
| D.3: "a list from the connection's catalog when a session is open" | The tables and views that the sidebar of an open window of the connection has loaded, with the line "Tables loaded in the sidebar" | Agreed on 2026-10-16: as quick open searches, and no new reading of the catalog |
| Components: "pickers float over the pane with an accent border. j/k or ctrl+n/p move, enter picks, esc closes. Typing filters" | A list floating under the field, the accent border on Omarchy and a shadow on macOS; Up and Down move, Enter picks, the form's Cancel closes the field, typing filters | `j`/`k` would type in the field. Up and Down are what quick open's list takes, in both looks |
| C.3: a dropped table's entry "marked 'not in this database' in the editor" | The chip says "not in this database" after the name, in the warning colour; under "All except" a line says agents may not use the connection until it is taken out or put right | No board draws the mark. The line is C.3's stop (1c-2a), said where the person can act on it |

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: every key is the keymap's (`tests/keys.rs`), no design or pixel check is ever a test, and no screenshot is committed. The Agents part shows only where agents can reach the app (`cfg(any(unix, windows))`).
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.

## What was run before this plan was written

Every task's state was built here, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` with both test servers (3,534 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. The list and the mark were looked at in both looks, light and dark, in throwaway scenes that are not kept. Not run: CI, and a hand test in a window.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Where Add table…'s list and the mark come from | What the sidebars of the connection's open windows loaded, together; a window switched to another database left out | Agreed on 2026-10-16: as quick open searches, with no new reading of the catalog. Another database is another database |
| What is known missing | A name in a loaded schema that has it not; a name in a schema the database does not list; a name without a schema once every schema is loaded. Never the system's schemas or SQLite's own tables | What the sidebar has not loaded cannot be said to be missing. The system's are not loaded, and the server never stops agents for them (found by the review) |
| How an offered table is written | With its schema, but a home schema's table no other loaded schema has, where no schema was typed and the list is not an "Only" one | Under "Only", a name without its schema reads every schema's table of that name (found by the review) |
| The list's keys | Up and Down (`FormTableMove`), as quick open's list; Enter, Tab, a click away or a save adds the one chosen; the form's Cancel closes the field | `j`/`k` would type in the field. A choice the person made is never dropped for what is typed (found by the review) |
| The list's place | Under the field, over it where there is no room; floating with the Components boards' picker's look | Found by the review: in a small window it covered the field |
| The mark | "not in this database" in the chip, in the warning colour, and under "All except" the line that agents are stopped until it is put right | C.3's mark, and the stop of 1c-2a said where the person can act on it |

### What this run leaves out

- A reading of the whole catalog for the list: what the sidebar has not loaded is neither offered nor marked.
- Masked columns: Phase 3a.

## File structure

| File | What changes |
|---|---|
| `src/model.rs` | `LoadedTables`, `offered`, `has`, `OFFERED`; `ConnectionForm::agents_loaded` and `agents_table_pick` |
| `src/app.rs` | `App::loaded_tables` |
| `src/ui/connect_dialog/mod.rs` | The form gets what the connection's windows loaded, on each frame |
| `src/keymap.rs` | `Command::FormTableMove`: Up and Down in the list |
| `src/ui/connect_dialog/agents.rs` | The list under the field, its keys and clicks; the chip's mark; the line under "All except" |
| `src/ui/agents_tests.rs` | The list and the mark, through the editor, in both looks |
| `docs/_guide/agents.md` | "Hide tables" |
| `docs/_reference/connections.md`, `keyboard-shortcuts.md` | The list, the mark, and the keys |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | D.3 as built for 1c-3b; Phase 1c built |

---

### Task 1: What the connection's open windows loaded

- `LoadedTables`: the driver, the home schema (`first_schema`, as the sidebar opens it), every schema the windows list but the system's, and each loaded schema's tables' and views' names.
- `offered(typed, listed, qualify)`: the loaded names that match what is typed (`fuzzy_score`, by the name or `schema.name`, as quick open; a quote typed is passed over), best first, at most `OFFERED` (8). Each is written with its schema, but a home schema's table that no other loaded schema has, where nothing typed names a schema and `qualify` (an "Only" list) is not asked. None an entry covers already: one without a schema covers every schema's.
- `has(name)`: `Some(true)` where a loaded schema has it; `Some(false)` where the schema it names is loaded and has it not, where the database lists no such schema, and for a name without a schema where every schema is loaded and none has it; else `None`. The system's schemas and SQLite's `sqlite_` tables are never marked.
- `App::loaded_tables(conn)`: every open window of the saved connection together, one switched to another database left out, the system's schemas left out; `None` with none open. The connection dialog gives it to the form on each frame while the Tables control has a list.
- `take_typed_table` adds the table chosen in the list, where one is: Enter, Tab, a click away and a save all add what the person chose.

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/ui/connect_dialog/mod.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/src/model.rs b/src/model.rs
index a10b394..6d32180 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -5395,6 +5395,86 @@ mod tests {
         assert!(back.ssh_secret_is_stale(), "another port, another server");
     }
 
+    #[test]
+    fn what_a_window_loaded_is_offered_and_marks_a_name_in_no_table() {
+        let names = |written: &[&str]| -> Vec<tabletist_db::agent::TableName> {
+            written.iter().map(|entry| entry.parse().unwrap()).collect()
+        };
+        let name = |written: &str| names(&[written]).remove(0);
+        let written = |offered: Vec<tabletist_db::agent::TableName>| -> Vec<String> {
+            offered.iter().map(ToString::to_string).collect()
+        };
+        let loaded = LoadedTables {
+            driver: Some(Driver::Postgres),
+            home: Some("public".into()),
+            schemas: vec!["public".into(), "sales".into(), "audit".into()],
+            loaded: vec![
+                (
+                    "public".into(),
+                    vec!["books".into(), "customers".into(), "authors".into()],
+                ),
+                ("sales".into(), vec!["orders".into(), "customers".into()]),
+            ],
+        };
+        // A home table whose name no other schema has goes without its
+        // schema; one another schema shares, or under "Only", with it; and
+        // with it where the person typed a schema.
+        assert_eq!(written(loaded.offered("auth", &[], false)), ["authors"]);
+        assert_eq!(
+            written(loaded.offered("auth", &[], true)),
+            ["public.authors"]
+        );
+        assert_eq!(
+            written(loaded.offered("cust", &[], false)),
+            ["public.customers", "sales.customers"]
+        );
+        assert_eq!(
+            written(loaded.offered("public.auth", &[], false)),
+            ["public.authors"]
+        );
+        // What an entry covers already is not offered: one without a
+        // schema covers every schema's.
+        assert_eq!(
+            written(loaded.offered("cust", &names(&["Customers"]), false)),
+            Vec::<String>::new()
+        );
+        assert_eq!(
+            written(loaded.offered("cust", &names(&["public.customers"]), false)),
+            ["sales.customers"]
+        );
+        assert_eq!(
+            written(loaded.offered("\"ord", &[], false)),
+            ["sales.orders"]
+        );
+        assert!(loaded.offered("  ", &[], false).is_empty());
+        // In a loaded schema: known either way; in one not loaded: unknown;
+        // in one the database does not have: missing.
+        assert_eq!(loaded.has(&name("sales.orders")), Some(true));
+        assert_eq!(loaded.has(&name("sales.old_orders")), Some(false));
+        assert_eq!(loaded.has(&name("audit.log")), None);
+        assert_eq!(loaded.has(&name("salse.orders")), Some(false));
+        assert_eq!(loaded.has(&name("BOOKS")), Some(true));
+        // The system's are never marked.
+        assert_eq!(loaded.has(&name("pg_catalog.pg_class")), None);
+        // Without a schema: any schema's, so missing only when every one of
+        // the database's own is loaded.
+        assert_eq!(loaded.has(&name("old_orders")), None);
+        let whole = LoadedTables {
+            schemas: vec!["public".into(), "sales".into()],
+            ..loaded
+        };
+        assert_eq!(whole.has(&name("old_orders")), Some(false));
+        // SQLite's own tables are the system's.
+        let sqlite = LoadedTables {
+            driver: Some(Driver::Sqlite),
+            home: Some("main".into()),
+            schemas: vec!["main".into()],
+            loaded: vec![("main".into(), vec!["books".into()])],
+        };
+        assert_eq!(sqlite.has(&name("sqlite_stat1")), None);
+        assert_eq!(sqlite.has(&name("old_orders")), Some(false));
+    }
+
     #[test]
     fn the_tables_control_keeps_a_list_for_each_mode_and_says_what_is_chosen() {
         use tabletist_db::agent::Tables;
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked --lib what_a_window_loaded`

Expected: it does not build:

```text
error[E0422]: cannot find struct, variant or union type `LoadedTables` in this scope
error: could not compile `tabletist` (lib test) due to 3 previous errors
```

- [ ] **Step 3: `LoadedTables`, and where it comes from**

````diff
diff --git a/src/app.rs b/src/app.rs
index 79e3cf4..e1aaa85 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -305,6 +305,65 @@ impl App {
             .map(|(tab, _)| tab)
     }
 
+    /// What the open windows of the saved connection `conn` have loaded in
+    /// their sidebars: each schema's tables and views, every window's
+    /// together, the system's schemas left out. A window switched to
+    /// another database than the saved one's is another database, and is
+    /// not counted. `None` with none open.
+    pub fn loaded_tables(
+        &self,
+        conn: &crate::connections::ConnectionId,
+    ) -> Option<crate::model::LoadedTables> {
+        let saved = self.connections.get(conn)?;
+        let mut found: Option<crate::model::LoadedTables> = None;
+        for (_, workspace) in self.open_connections().filter(|(_, workspace)| {
+            workspace.conn_id == *conn && workspace.spec.database == saved.spec.database
+        }) {
+            let driver = workspace.driver;
+            let user = |schema: &String| !crate::model::is_system_schema(driver, schema);
+            let tables = found.get_or_insert_with(Default::default);
+            tables.driver = Some(driver);
+            let schemas = workspace.tree.schemas.value.clone().unwrap_or_default();
+            if tables.home.is_none() {
+                tables.home = crate::model::first_schema(&schemas, &workspace.spec.database);
+            }
+            for schema in schemas.into_iter().filter(user) {
+                if !tables.schemas.contains(&schema) {
+                    tables.schemas.push(schema);
+                }
+            }
+            for (schema, node) in workspace
+                .tree
+                .nodes
+                .iter()
+                .filter(|(schema, _)| user(schema))
+            {
+                let Some(objects) = &node.objects.value else {
+                    continue;
+                };
+                let names = objects.iter().map(|object| object.name.clone());
+                match tables
+                    .loaded
+                    .iter_mut()
+                    .find(|(loaded, _)| loaded == schema)
+                {
+                    Some((_, known)) => {
+                        for name in names {
+                            if !known.contains(&name) {
+                                known.push(name);
+                            }
+                        }
+                    }
+                    None => tables.loaded.push((schema.clone(), names.collect())),
+                }
+            }
+        }
+        if let Some(tables) = found.as_mut() {
+            tables.loaded.sort();
+        }
+        found
+    }
+
     pub fn tab_for_session(&self, session: SessionId) -> Option<ConnTabId> {
         self.tabs.iter().find_map(|tab| match &tab.content {
             ConnTabContent::Workspace(workspace) if workspace.session == session => Some(tab.id),
diff --git a/src/model.rs b/src/model.rs
index 6d32180..c0c3814 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -1098,6 +1098,137 @@ impl TablesChoice {
     }
 }
 
+/// The tables and views the open windows of a connection have loaded in
+/// their sidebars: what Add table… offers, and what a listed name is
+/// checked against to be marked as in no table of the database. Only what
+/// is loaded is known: a schema not loaded neither offers nor marks, and
+/// the system's schemas and tables, which the sidebar does not load, are
+/// left out.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct LoadedTables {
+    /// The connection's driver, for which schemas and tables are the
+    /// system's.
+    pub driver: Option<Driver>,
+    /// The schema a name without one is first looked up in.
+    pub home: Option<String>,
+    /// Every schema of the database the windows list, the system's left
+    /// out.
+    pub schemas: Vec<String>,
+    /// Each loaded schema with its tables' and views' names.
+    pub loaded: Vec<(String, Vec<String>)>,
+}
+
+/// How many tables Add table…'s list shows at once.
+pub const OFFERED: usize = 8;
+
+impl LoadedTables {
+    /// Whether `schema` is the system's, which no entry is checked in.
+    fn system_schema(&self, schema: &str) -> bool {
+        self.driver
+            .is_some_and(|driver| is_system_schema(driver, &schema.to_lowercase()))
+    }
+
+    /// Whether `name` is one of the system's own tables (SQLite's
+    /// `sqlite_` ones), which hiding never stops agents for.
+    fn system_table(&self, name: &tabletist_db::agent::TableName) -> bool {
+        match self.driver {
+            Some(Driver::Sqlite) => name.name.to_lowercase().starts_with("sqlite_"),
+            Some(Driver::Postgres | Driver::MySql) | None => false,
+        }
+    }
+
+    /// The loaded tables whose names match `typed`, best first, at most
+    /// [`OFFERED`], and none an entry of `listed` already covers. Each is
+    /// written as an entry: with its schema, except a home schema's table
+    /// whose name no other loaded schema has, where the person typed no
+    /// schema and `qualify` (an "Only" list, where a name without a schema
+    /// would read every schema's) is not asked. Nothing for nothing typed.
+    pub fn offered(
+        &self,
+        typed: &str,
+        listed: &[tabletist_db::agent::TableName],
+        qualify: bool,
+    ) -> Vec<tabletist_db::agent::TableName> {
+        use tabletist_db::agent::TableName;
+        let typed = typed.trim().replace('"', "");
+        if typed.is_empty() {
+            return Vec::new();
+        }
+        let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
+        let shared = |name: &str| {
+            self.loaded
+                .iter()
+                .filter(|(_, names)| names.iter().any(|known| same(known, name)))
+                .count()
+                > 1
+        };
+        // An entry covers a table of `schema` where its name is the
+        // table's and its schema, if it has one, is `schema`.
+        let covered = |schema: &str, name: &str| {
+            listed.iter().any(|entry| {
+                same(&entry.name, name)
+                    && entry.schema.as_deref().is_none_or(|own| same(own, schema))
+            })
+        };
+        let mut scored: Vec<(i64, String, TableName)> = self
+            .loaded
+            .iter()
+            .flat_map(|(schema, names)| names.iter().map(move |name| (schema, name)))
+            .filter(|(schema, name)| !covered(schema, name))
+            .filter_map(|(schema, name)| {
+                let qualified = format!("{schema}.{name}");
+                let score = crate::util::fuzzy_score(&typed, name)
+                    .max(crate::util::fuzzy_score(&typed, &qualified))?;
+                let home = self.home.as_deref().is_some_and(|home| same(home, schema));
+                let bare = home && !qualify && !typed.contains('.') && !shared(name);
+                let entry = TableName {
+                    schema: (!bare).then(|| schema.clone()),
+                    name: name.clone(),
+                };
+                Some((score, qualified.to_lowercase(), entry))
+            })
+            .collect();
+        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
+        scored.truncate(OFFERED);
+        scored.into_iter().map(|(_, _, entry)| entry).collect()
+    }
+
+    /// Whether `name` is a table or view of the database, where what the
+    /// windows loaded can say: `Some(false)` marks it. A name in a schema
+    /// the database does not list is missing; one in a schema not loaded is
+    /// not known; one without a schema is any schema's, so it is missing
+    /// only when every schema is loaded. The system's are never marked.
+    pub fn has(&self, name: &tabletist_db::agent::TableName) -> Option<bool> {
+        let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
+        let in_schema = |names: &Vec<String>| names.iter().any(|known| same(known, &name.name));
+        if self.system_table(name) {
+            return None;
+        }
+        match &name.schema {
+            Some(schema) if self.system_schema(schema) => None,
+            Some(schema) => match self.loaded.iter().find(|(loaded, _)| same(loaded, schema)) {
+                Some((_, names)) => Some(in_schema(names)),
+                None => {
+                    let listed = !self.schemas.is_empty()
+                        && !self.schemas.iter().any(|known| same(known, schema));
+                    listed.then_some(false)
+                }
+            },
+            None => {
+                if self.loaded.iter().any(|(_, names)| in_schema(names)) {
+                    return Some(true);
+                }
+                let all = !self.schemas.is_empty()
+                    && self
+                        .schemas
+                        .iter()
+                        .all(|schema| self.loaded.iter().any(|(loaded, _)| same(loaded, schema)));
+                all.then_some(false)
+            }
+        }
+    }
+}
+
 /// Where the Tables control puts the keyboard on the next frame.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum TableFocus {
@@ -1136,6 +1267,11 @@ pub struct ConnectionForm {
     pub agents_table_error: Option<String>,
     /// Where the Tables control puts the keyboard on the next frame.
     pub agents_table_focus: Option<TableFocus>,
+    /// What an open window of the connection has loaded, for Add table…'s
+    /// list and the mark of a name in no table; `None` with none open.
+    pub agents_loaded: Option<LoadedTables>,
+    /// The table of Add table…'s list the keyboard is on, by its place.
+    pub agents_table_pick: Option<usize>,
     /// Rows per answer, as typed.
     pub agents_rows: String,
     /// The statement's timeout in seconds, as typed.
@@ -1261,6 +1397,8 @@ impl Default for ConnectionForm {
             agents_table_typed: None,
             agents_table_error: None,
             agents_table_focus: None,
+            agents_loaded: None,
+            agents_table_pick: None,
             agents_rows: crate::connections::Agents::default().rows.to_string(),
             agents_timeout: crate::connections::Agents::default()
                 .timeout_secs
@@ -1411,6 +1549,8 @@ impl ConnectionForm {
             agents_table_typed: None,
             agents_table_error: None,
             agents_table_focus: None,
+            agents_loaded: None,
+            agents_table_pick: None,
             agents_rows: saved.agents.rows.to_string(),
             agents_timeout: saved.agents.timeout_secs.to_string(),
             show_agents: false,
@@ -1677,9 +1817,22 @@ impl ConnectionForm {
     /// Save does: added, or `Err` with why it does not read, which is then
     /// also said under the names.
     pub fn take_typed_table(&mut self) -> Result<(), String> {
-        let Some(typed) = self.agents_table_typed.clone() else {
+        let Some(mut typed) = self.agents_table_typed.clone() else {
             return Ok(());
         };
+        // The table chosen in Add table…'s list, where one is: Enter, Tab,
+        // a click away and a save all add what the person chose.
+        if let Some(at) = self.agents_table_pick.take() {
+            let listed = self.agents_tables.names().cloned().unwrap_or_default();
+            let only = self.agents_tables.mode == TablesMode::Only;
+            if let Some(picked) = self
+                .agents_loaded
+                .as_ref()
+                .and_then(|loaded| loaded.offered(&typed, &listed, only).into_iter().nth(at))
+            {
+                typed = picked.to_string();
+            }
+        }
         if typed.trim().is_empty() {
             self.agents_table_typed = None;
             return Ok(());
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
index f51d141..e3d38b9 100644
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -239,10 +239,21 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
     let on = |input: &mut egui::InputState, command: Command| {
         crate::ui::keys::asked(input, &keymap, layout, command, Scope::Prompt, true)
     };
+    // What an open window of the connection has loaded, for the Tables
+    // control: read again on each frame, as a sidebar loads.
+    // Only for a mode with a list: what it costs is spent on the list.
+    let loaded = match &app.dialog {
+        Some(Dialog::Connection(form)) if form.agents_tables.names().is_some() => form
+            .editing
+            .as_ref()
+            .and_then(|conn| app.loaded_tables(conn)),
+        _ => None,
+    };
     let Some(Dialog::Connection(form)) = &mut app.dialog else {
         ctx.data_mut(|data| data.remove::<Placement>(placement_id()));
         return;
     };
+    form.agents_loaded = loaded;
     let placement: Placement = ctx
         .data(|data| data.get_temp(placement_id()))
         .unwrap_or_default();
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- what_a_window_loaded the_tables_control_keeps`

Expected: PASS.

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2555 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/model.rs src/app.rs src/ui/connect_dialog/mod.rs
git commit -m "Know what the connection's open windows loaded, for the editor's Tables"
```

---

### Task 2: The list under the field, and the mark

- While the field holds a name and the loaded tables offer some, they float under it in an `egui::Area` (`Order::Tooltip`), or over it where the window has no room: a shadow on macOS, the accent border on Omarchy, the one the keyboard is on marked as a selection, and the line "Tables loaded in the sidebar". Each is heard as a selectable label by its name, and the field names the chosen one as its active descendant.
- `Command::FormTableMove`, Up and Down in both looks, moves in the list while the field has the keyboard, taken before the field reads them so that its caret stays; typing again puts the keyboard back on no table. Enter, Tab or a click away adds the chosen table, or what is typed where none is chosen; a click on a table adds it; a press and release on the list elsewhere gives the keyboard back to the field.
- A chip whose name `has` says is in no table says "not in this database" after the name, in the warning colour, heard as "old_orders, not in this database". Under "All except" with such a name, a line in the warning colour: "Agents may not use this connection while a hidden table is not in its database. Take the name out, or add the table's new name."

**Files:**
- Modify: `src/keymap.rs`, `src/ui/connect_dialog/agents.rs`, `src/ui/agents_tests.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index 5b3ca3b..243ffe1 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -1327,6 +1327,154 @@ fn the_editors_tables_control_lists_what_agents_may_not_read() {
     }
 }
 
+/// A press and a release of the pointer at `at`.
+#[cfg(any(unix, windows))]
+fn press_at(harness: &mut Harness, at: egui::Pos2) {
+    let press = |pressed| egui::Event::PointerButton {
+        pos: at,
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers: egui::Modifiers::NONE,
+    };
+    harness.frame(vec![egui::Event::PointerMoved(at)]);
+    harness.frame(vec![press(true)]);
+    harness.frame(vec![press(false)]);
+    harness.settle();
+}
+
+/// The tables loaded in the tree of `tab`'s window: `main`'s, these.
+#[cfg(any(unix, windows))]
+fn loaded(harness: &mut Harness, tab: ConnTabId, names: &[&str]) {
+    use tabletist_db::{ObjectInfo, ObjectKind};
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    workspace.tree.schemas.value = Some(vec!["main".into()]);
+    let node = workspace.tree.nodes.entry("main".into()).or_default();
+    node.objects.value = Some(
+        names
+            .iter()
+            .map(|name| ObjectInfo {
+                name: (*name).into(),
+                kind: ObjectKind::Table,
+                estimated_rows: None,
+            })
+            .collect(),
+    );
+}
+
+#[cfg(any(unix, windows))]
+#[test]
+fn two_windows_of_a_connection_are_known_together_and_another_database_is_not() {
+    let (mut harness, first) = connected(Look::macos());
+    // A second window, of the same connection.
+    harness.app.apply(Action::ShowConnections);
+    let second = harness.connect_fake();
+    assert_ne!(first, second);
+    // The fixture is saved anew at each connect: the second window is
+    // made the first's connection's.
+    let conn = harness.app.workspace(first).unwrap().conn_id.clone();
+    harness.app.workspace_mut(second).unwrap().conn_id = conn.clone();
+    loaded(&mut harness, first, &["books", "orders"]);
+    loaded(&mut harness, second, &["books"]);
+    let known = harness.app.loaded_tables(&conn).unwrap();
+    assert_eq!(known.has(&"orders".parse().unwrap()), Some(true));
+    // A window switched to another database is not this connection's.
+    harness.app.workspace_mut(first).unwrap().spec.database = "other".into();
+    harness.app.workspace_mut(second).unwrap().spec.database = "other".into();
+    assert_eq!(harness.app.loaded_tables(&conn), None);
+}
+
+#[cfg(any(unix, windows))]
+#[test]
+fn add_table_offers_the_windows_tables_and_marks_a_name_in_none() {
+    use tabletist_db::{ObjectInfo, ObjectKind};
+    let gone = |look: &Look| {
+        look.label(
+            "Agents may not use this connection while a hidden table is not in its database. \
+             Take the name out, or add the table's new name.",
+        )
+    };
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        // The window's sidebar has loaded its one schema.
+        let workspace = harness.app.workspace_mut(tab).unwrap();
+        workspace.tree.schemas.value = Some(vec!["main".into()]);
+        let node = workspace.tree.nodes.entry("main".into()).or_default();
+        node.objects.value = Some(
+            ["books", "customers", "orders"]
+                .iter()
+                .map(|name| ObjectInfo {
+                    name: (*name).into(),
+                    kind: ObjectKind::Table,
+                    estimated_rows: None,
+                })
+                .collect(),
+        );
+        let conn = workspace.conn_id.clone();
+        allow(&mut harness, tab, AgentAccess::Read);
+        let mut saved = harness.app.connections.get(&conn).unwrap().clone();
+        saved.agents.tables = tabletist_db::agent::Tables::AllExcept(vec![
+            "customers".parse().unwrap(),
+            "old_orders".parse().unwrap(),
+        ]);
+        harness.app.connections.upsert(saved);
+        harness.app.check_agents();
+        at_agents(&mut harness, &conn);
+        // A name in no table of the database is marked, and what that does
+        // to agents is said.
+        let mark = |name: &str| look.label(&format!("{name}, not in this database"));
+        assert!(harness.has(&mark("old_orders")), "{}", look.name);
+        assert!(!harness.has(&mark("customers")), "{}", look.name);
+        assert!(harness.has(&gone(&look)), "{}", look.name);
+        // Typed, the window's tables that match are offered; down picks
+        // the first, and Enter adds it.
+        type_table(&mut harness, &look, "ord");
+        assert!(harness.has("orders"), "{}", look.name);
+        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
+        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+        harness.settle();
+        assert!(harness.has(&look.label("Remove orders")), "{}", look.name);
+        // Up and Down move in the list, and not the field's caret.
+        type_table(&mut harness, &look, "boo");
+        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
+        harness.press(egui::Key::ArrowUp, egui::Modifiers::NONE);
+        harness.frame(vec![egui::Event::Text("k".into())]);
+        harness.settle();
+        assert_eq!(
+            form(&harness).agents_table_typed.as_deref(),
+            Some("book"),
+            "{}",
+            look.name
+        );
+        // A press on the list's note is no leaving the field.
+        let note = harness
+            .painted_rect(&look.label("Tables loaded in the sidebar"))
+            .expect("the list's note");
+        press_at(&mut harness, note.center());
+        assert_eq!(
+            form(&harness).agents_table_typed.as_deref(),
+            Some("book"),
+            "{}",
+            look.name
+        );
+        // Tab with a table chosen adds that table.
+        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
+        harness.press(egui::Key::Tab, egui::Modifiers::NONE);
+        harness.settle();
+        assert!(harness.has(&look.label("Remove books")), "{}", look.name);
+        // A name typed that is in no list is added as typed.
+        add_table(&mut harness, &look, "payroll");
+        assert!(harness.has(&mark("payroll")), "{}", look.name);
+        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
+        harness.settle();
+        // Taken out, nothing is marked and nothing said.
+        harness.click(&look.label("Remove old_orders"));
+        harness.click(&look.label("Remove payroll"));
+        harness.click(&look.label("Remove books"));
+        harness.settle();
+        assert!(!harness.has(&gone(&look)), "{}", look.name);
+    }
+}
+
 #[cfg(any(unix, windows))]
 #[test]
 fn allow_agents_opens_the_editor_with_read_chosen() {
````

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked --lib add_table_offers`

Expected: FAIL. Nothing marks a name in no table yet, so the first thing the test looks for is not there:

```text
test ui::agents_tests::add_table_offers_the_windows_tables_and_marks_a_name_in_none ... FAILED
thread 'ui::agents_tests::add_table_offers_the_windows_tables_and_marks_a_name_in_none' panicked at src/ui/agents_tests.rs:1425:9:
macos
```

- [ ] **Step 3: The list, its keys, and the mark**

````diff
diff --git a/src/keymap.rs b/src/keymap.rs
index 962d2ef..199ec4b 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -487,6 +487,7 @@ commands! {
     FormSaveAndConnect,
     FormForgetPassword,
     FormCancel,
+    FormTableMove,
     SettingsMove,
     SettingsChange,
     SettingsToggle,
@@ -781,6 +782,7 @@ impl Command {
             C::FormSaveAndConnect => info("Save and connect", "", Prompts),
             C::FormForgetPassword => info("Forget the saved password", "", Prompts),
             C::FormCancel => info("Cancel", "", Prompts),
+            C::FormTableMove => info("Move in Add table…'s list", "", Prompts),
             C::SettingsMove => info("Move in the settings", "", Prompts),
             C::SettingsChange => info("Change the option", "", Prompts),
             C::SettingsToggle => info("Toggle the option", "", Prompts),
@@ -1721,6 +1723,15 @@ pub const BINDINGS: &[Binding] = {
         .when(When::ConnectionForm),
         bind(C::FormCancel, PROMPT, keys(&[k("esc")]), keys(&[k("esc")]))
             .when(When::ConnectionForm),
+        // In Add table…'s field, while its list of the window's tables
+        // shows: up moves up it, and down down.
+        bind(
+            C::FormTableMove,
+            PROMPT,
+            keys(&[k("up"), k("down")]),
+            keys(&[k("up"), k("down")]),
+        )
+        .when(When::ConnectionForm),
         bind(
             C::SettingsMove,
             PROMPT,
diff --git a/src/ui/connect_dialog/agents.rs b/src/ui/connect_dialog/agents.rs
index 7d5f188..bfdc8e2 100644
--- a/src/ui/connect_dialog/agents.rs
+++ b/src/ui/connect_dialog/agents.rs
@@ -117,33 +117,102 @@ fn table_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
         return;
     };
     let written: Vec<String> = names.iter().map(ToString::to_string).collect();
+    // Which are in no table of the schemas an open window loaded.
+    let missing: Vec<bool> = names
+        .iter()
+        .map(|name| {
+            form.agents_loaded
+                .as_ref()
+                .and_then(|loaded| loaded.has(name))
+                == Some(false)
+        })
+        .collect();
     let height = if skin.look.terminal { 21.0 } else { 24.0 };
     let mut removed = None;
     let focus = form.agents_table_focus.take();
     ui.horizontal_wrapped(|ui| {
         ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
-        for (at, name) in written.iter().enumerate() {
-            if chip(ui, name, at, height, skin) {
+        for (at, (name, missing)) in written.iter().zip(&missing).enumerate() {
+            if chip(ui, name, at, *missing, height, skin) {
                 removed = Some(at);
             }
         }
-        match form.agents_table_typed.as_mut() {
-            Some(typed) => {
+        let only = form.agents_tables.mode == TablesMode::Only;
+        let listed = form.agents_tables.names().cloned().unwrap_or_default();
+        let offered_for = |form: &ConnectionForm, typed: &str| {
+            form.agents_loaded
+                .as_ref()
+                .map(|loaded| loaded.offered(typed, &listed, only))
+                .unwrap_or_default()
+        };
+        match form.agents_table_typed.clone() {
+            Some(before) => {
                 let role = widgets::code(skin.look);
                 let id = ui.id().with("agents-table-typed");
+                let list = id.with("list");
+                // Up and Down move in the list, taken before the field
+                // reads them, so that they do not move its caret too.
+                let offered = offered_for(form, &before);
+                if !offered.is_empty() && ui.memory(|memory| memory.has_focus(id)) {
+                    let step = moved(ui, skin);
+                    if step != 0 {
+                        let last = offered.len() as isize - 1;
+                        form.agents_table_pick = Some(match form.agents_table_pick {
+                            None if step > 0 => 0,
+                            None => last as usize,
+                            Some(at) => (at as isize + step).rem_euclid(last + 1) as usize,
+                        });
+                    }
+                }
                 let placeholder = super::hint(ui, &skin.say("table, or schema.table"), role, skin);
+                let Some(typed) = form.agents_table_typed.as_mut() else {
+                    return;
+                };
                 let field = ui.add_sized(
                     vec2(200.0, height),
                     input(ui, typed, role, height, skin)
                         .id(id)
                         .hint_text(placeholder),
                 );
-                let name = skin.say("Table to add");
-                ui.ctx()
-                    .accesskit_node_builder(field.id, |node| node.set_label(name.as_str()));
+                let typed = typed.clone();
+                if field.changed() {
+                    form.agents_table_pick = None;
+                }
+                let offered = offered_for(form, &typed);
+                if form.agents_table_pick.is_some_and(|at| at >= offered.len()) {
+                    form.agents_table_pick = None;
+                }
                 if focus == Some(TableFocus::Field) {
                     field.request_focus();
                 }
+                // A release over the list, where the field gives up the
+                // keyboard, is no leaving it: the list was last seen there.
+                // Only a release this frame: a pointer that rests there says
+                // nothing of a Tab.
+                let released = ui.input(|input| input.pointer.any_released());
+                let on_list = released
+                    && ui
+                        .ctx()
+                        .memory(|memory| memory.area_rect(list))
+                        .zip(ui.input(|input| input.pointer.latest_pos()))
+                        .is_some_and(|(rect, at)| rect.contains(at));
+                let clicked = match offered.is_empty() {
+                    true => None,
+                    false => {
+                        offered_list(ui, list, field.rect, &offered, form.agents_table_pick, skin)
+                    }
+                };
+                // Screen readers hear the table the keyboard is on.
+                let name = skin.say("Table to add");
+                let chosen = form
+                    .agents_table_pick
+                    .map(|at| list.with(at).accesskit_id());
+                ui.ctx().accesskit_node_builder(field.id, |node| {
+                    node.set_label(name.as_str());
+                    if let Some(chosen) = chosen {
+                        node.set_active_descendant(chosen);
+                    }
+                });
                 let pressed = |key| ui.input(|input| input.key_pressed(key));
                 // The form's Cancel, as the keymap has it, while the field
                 // has the keyboard or has just let it go: the field's own,
@@ -159,14 +228,27 @@ fn table_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
                             > 0
                     })
                 };
-                if cancelled {
+                if let Some(at) = clicked {
+                    form.agents_table_typed = Some(offered[at].to_string());
+                    form.agents_table_pick = None;
+                    if form.take_typed_table().is_ok() {
+                        form.agents_table_typed = Some(String::new());
+                    }
+                    form.agents_table_focus = Some(TableFocus::Field);
+                } else if cancelled {
                     field.surrender_focus();
+                    form.agents_table_pick = None;
                     form.agents_table_typed = None;
                     form.agents_table_error = None;
                     form.agents_table_focus = Some(TableFocus::Add);
+                } else if field.lost_focus() && on_list && !offered.is_empty() {
+                    // Pressed on the list, not on a table: the keyboard
+                    // goes back to the field.
+                    form.agents_table_focus = Some(TableFocus::Field);
                 } else if field.lost_focus() {
                     let entered = pressed(egui::Key::Enter);
-                    let empty = typed.trim().is_empty();
+                    let empty = typed.trim().is_empty() && form.agents_table_pick.is_none();
+                    // The table chosen in the list, or what is typed.
                     match form.take_typed_table() {
                         // Enter on a name: the field stays, for the next.
                         Ok(()) if entered && !empty => {
@@ -202,6 +284,144 @@ fn table_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
     }
 }
 
+/// How far Add table…'s list was asked to move this frame: up by one,
+/// down by one, or not, by the keys the keymap gives the form for it, as
+/// quick open's list reads its own.
+fn moved(ui: &mut Ui, skin: &Skin) -> isize {
+    use crate::keymap::{Command, Layout, Scope};
+    let keymap = crate::ui::keys::published(ui.ctx());
+    let layout = Layout::of(skin.look);
+    ui.input_mut(|input| {
+        let mut step = 0;
+        for keys in keymap.chords_now(layout, Command::FormTableMove, Scope::Prompt, true) {
+            let way = if keys == "up" { -1 } else { 1 };
+            if crate::ui::keys::presses(input, layout, keys).count > 0 {
+                step += way;
+            }
+        }
+        step
+    })
+}
+
+/// The window's tables that match what is typed, floating under the field
+/// at `under`, the one the keyboard is on (`pick`) marked, with a line that
+/// says where they come from. The place of the one clicked.
+fn offered_list(
+    ui: &mut Ui,
+    id: egui::Id,
+    under: Rect,
+    offered: &[tabletist_db::agent::TableName],
+    pick: Option<usize>,
+    skin: &Skin,
+) -> Option<usize> {
+    let Skin { look, palette, .. } = *skin;
+    let role = widgets::code(look);
+    let row = if look.terminal { 21.0 } else { 24.0 };
+    let width = under.width().max(240.0);
+    let mut clicked = None;
+    // Under the field where it has the room, else over it: never on it.
+    let tall = offered.len() as f32 * row + 40.0;
+    let room = ui.ctx().content_rect().bottom() - under.bottom();
+    let (pivot, at) = match room >= tall + 8.0 {
+        true => (egui::Align2::LEFT_TOP, under.left_bottom() + vec2(0.0, 4.0)),
+        false => (egui::Align2::LEFT_BOTTOM, under.left_top() - vec2(0.0, 4.0)),
+    };
+    egui::Area::new(id)
+        .order(egui::Order::Tooltip)
+        .pivot(pivot)
+        .fixed_pos(at)
+        .show(ui.ctx(), |ui| {
+            let corner = CornerRadius::same(look.radius);
+            let border = if look.terminal {
+                palette.accent
+            } else {
+                palette.border
+            };
+            // As the board's popovers: a soft shadow on macOS, none in the
+            // terminal look, whose list stands out by its accent border.
+            let shadow = match look.terminal {
+                true => egui::epaint::Shadow::NONE,
+                false => egui::epaint::Shadow {
+                    offset: [0, 8],
+                    blur: 24,
+                    spread: 0,
+                    color: egui::Color32::from_black_alpha(36),
+                },
+            };
+            egui::Frame::new()
+                .fill(palette.panel)
+                .stroke(Stroke::new(1.0, border))
+                .corner_radius(corner)
+                .shadow(shadow)
+                .inner_margin(egui::Margin::same(4))
+                .show(ui, |ui| {
+                    ui.set_width(width);
+                    for (at, name) in offered.iter().enumerate() {
+                        let written = name.to_string();
+                        let (rect, _) = ui.allocate_exact_size(vec2(width, row), Sense::hover());
+                        let response = ui.interact(rect, id.with(at), Sense::click());
+                        let chosen = pick == Some(at);
+                        response.widget_info(|| {
+                            WidgetInfo::selected(
+                                WidgetType::SelectableLabel,
+                                ui.is_enabled(),
+                                chosen,
+                                &written,
+                            )
+                        });
+                        widgets::selection(ui, rect, chosen, response.hovered(), look, palette);
+                        let ink = widgets::selection_text(chosen, look, palette);
+                        let text = Text::one(look, role, &written, ink).layout(ui.ctx());
+                        let y = rect.center().y - text.galley.size().y / 2.0;
+                        ui.painter()
+                            .galley(egui::pos2(rect.left() + 8.0, y), text.galley, ink);
+                        if response.clicked() {
+                            clicked = Some(at);
+                        }
+                    }
+                    let said = skin.say("Tables loaded in the sidebar");
+                    let small = TextRole::pick(look, TextRole::Secondary, TextRole::OSecondary);
+                    ui.add_space(2.0);
+                    widgets::label(ui, small, &said, palette.faint, look);
+                });
+        });
+    clicked
+}
+
+/// Under an "All except" list that names a table not in the database: what
+/// that does to agents (the MCP plan's C.3), and how to put it right.
+fn table_missing(ui: &mut Ui, form: &ConnectionForm, skin: &Skin) {
+    let Some(loaded) = &form.agents_loaded else {
+        return;
+    };
+    let gone = match form.agents_tables.mode {
+        TablesMode::AllExcept => form
+            .agents_tables
+            .hidden
+            .iter()
+            .any(|name| loaded.has(name) == Some(false)),
+        TablesMode::All | TablesMode::Only => false,
+    };
+    if !gone {
+        return;
+    }
+    let said = skin.say(
+        "Agents may not use this connection while a hidden table is not in its database. Take \
+         the name out, or add the table's new name.",
+    );
+    ui.add_space(4.0);
+    let room = ui.available_width();
+    Text::one(
+        skin.look,
+        TextRole::pick(skin.look, TextRole::Secondary, TextRole::OSecondary),
+        &said,
+        skin.palette.warning,
+    )
+    .wrap(room)
+    .layout(ui.ctx())
+    .label(ui);
+}
+
 /// Why the name typed does not read, under the names: a line of its own,
 /// after the row that holds them.
 fn table_error(ui: &mut Ui, form: &ConnectionForm, skin: &Skin) {
@@ -223,15 +443,22 @@ fn table_error(ui: &mut Ui, form: &ConnectionForm, skin: &Skin) {
 
 /// A listed name, `customers ×`, bordered; the × takes it out, and screen
 /// readers hear it as "Remove customers". Whether it was pressed.
-fn chip(ui: &mut Ui, name: &str, at: usize, height: f32, skin: &Skin) -> bool {
+fn chip(ui: &mut Ui, name: &str, at: usize, missing: bool, height: f32, skin: &Skin) -> bool {
     let Skin { look, palette, .. } = *skin;
     let role = widgets::code(look);
     let cross = if look.terminal { "x" } else { "×" };
     let name_width = role.width(ui.ctx(), look.faces, name);
     let cross_width = role.width(ui.ctx(), look.faces, cross);
+    // A name in no table of the database says so after itself.
+    let mark = skin.say("not in this database");
+    let mark_role = TextRole::pick(look, TextRole::Secondary, TextRole::OSecondary);
+    let mark_width = match missing {
+        true => 8.0 + mark_role.width(ui.ctx(), look.faces, &mark),
+        false => 0.0,
+    };
     let (left, gap, right) = (8.0, 6.0, 6.0);
     let size = vec2(
-        (left + name_width + gap + cross_width + right).ceil(),
+        (left + name_width + mark_width + gap + cross_width + right).ceil(),
         height,
     );
     let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
@@ -251,6 +478,21 @@ fn chip(ui: &mut Ui, name: &str, at: usize, height: f32, skin: &Skin) -> bool {
     let text = Text::one(look, role, name, palette.text).layout(ui.ctx());
     let y = rect.center().y - text.galley.size().y / 2.0;
     painter.galley(egui::pos2(rect.left() + left, y), text.galley, palette.text);
+    if missing {
+        let marked = Text::one(look, mark_role, &mark, palette.warning).layout(ui.ctx());
+        let x = rect.left() + left + name_width + 8.0;
+        let y = rect.center().y - marked.galley.size().y / 2.0;
+        painter.galley(egui::pos2(x, y), marked.galley, palette.warning);
+        // Heard as the name and the mark.
+        let place = Rect::from_min_max(rect.left_top(), egui::pos2(x + mark_width, rect.bottom()));
+        let said = format!("{name}, {mark}");
+        ui.interact(
+            place,
+            ui.id().with(("agents-table-missing", at, name)),
+            Sense::hover(),
+        )
+        .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &said));
+    }
     let cell = Rect::from_min_max(
         egui::pos2(rect.right() - right - cross_width - gap / 2.0, rect.top()),
         rect.right_bottom(),
@@ -343,6 +585,7 @@ pub(super) fn sheet(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
             ui.add_space(6.0);
             table_names(ui, form, skin);
             table_error(ui, form, skin);
+            table_missing(ui, form, skin);
         }
         if let Some(hint) = tables_hint(form) {
             ui.add_space(6.0);
@@ -389,6 +632,7 @@ pub(super) fn terminal(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
             table_names(ui, form, skin);
         });
         table_error(ui, form, skin);
+        table_missing(ui, form, skin);
     }
     if let Some(hint) = tables_hint(form) {
         sentence(ui, TextRole::OSecondary, &skin.say(hint), skin);
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- add_table_offers two_windows_of_a_connection the_editors_tables_control`, then `cargo test --locked --test keys`

Expected: PASS.

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 2556 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Look at it**

In a throwaway `#[test] #[ignore]` scene at the end of `src/shots.rs` (built on `both(name, |harness| ..)`: `workspace(harness)` for a window with the Bookshop's tables loaded, its saved connection Read and hiding `customers` and `old_orders`, `Action::EditAgents`, eight `settle`s, Add table…, `book` typed and Down), run `cargo test --locked --features shots --lib shots::<scene> -- --ignored` and read `target/shots/<scene>-{macos,omarchy}-{light,dark}.png`. Remove the scene, and keep no picture.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/keymap.rs src/ui/connect_dialog/agents.rs src/ui/agents_tests.rs
git commit -m "List the window's tables under Add table…, and mark a name in no table"
```

---

### Task 3: The website

- `docs/_guide/agents.md`: "Hide tables": the three choices, Add table… and its list, the mark, what a hidden table hides with it, that agents are told the names, the stop for a renamed or dropped table, what a check in the app cannot stop, and where a change made outside waits.
- `docs/_reference/connections.md`: the list and the mark in the editor's Tables.
- `docs/_reference/keyboard-shortcuts.md`: Up and Down in Add table…'s list.

**Files:**
- Modify: `docs/_guide/agents.md`, `docs/_reference/connections.md`, `docs/_reference/keyboard-shortcuts.md`

- [ ] **Step 1: The pages**

````diff
diff --git a/docs/_guide/agents.md b/docs/_guide/agents.md
index 3a5b2a8..20119bc 100644
--- a/docs/_guide/agents.md
+++ b/docs/_guide/agents.md
@@ -27,7 +27,8 @@ connection's limits:
 rows per answer and a statement timeout. Tabletist checks each statement
 before it runs. A statement that would write, lock or change the schema is
 refused, and so are several statements at once and `COPY`. An agent sees
-what the connection's database user can see, and never more.
+what the connection's database user can see, and never more. You can
+also [hide tables](#hide-tables) from them.
 
 Agents cannot write in this version.
 
@@ -101,6 +102,59 @@ Every connection starts with agents **Off**: they do not see it at all.
 [Connections]({% link _reference/connections.md %}#agents) describes the
 section.
 
+## Hide tables
+
+A connection can keep some of its tables from agents. In its form's
+**Agents** section, set **Tables**:
+
+- **All**: agents may read every table. This is where every connection
+  starts.
+- **All except**: every table but the ones you list.
+- **Only**: the ones you list and no others. A table made later is hidden
+  too, until you list it.
+
+**Add table…** takes a table's name, `customers`, or with its schema,
+`sales.customers`; a name without a schema is that table in every
+schema. While a window of the connection is open, it lists the tables
+that window's sidebar has loaded as you type. A listed name that the
+loaded schemas show is not a table is marked **not in this database**:
+one whose schema is loaded and has no such table, one whose schema the
+database does not have, or, for a name without a schema, one that no
+schema has once every schema is loaded. What the sidebar has not loaded
+is not known, and not marked.
+
+What a hidden table hides with it:
+
+- Its rows, and its columns: `describe_schema` leaves it out, and the keys
+  that point to it or from it.
+- A view over it, a materialized view, and a partition or a child table of
+  it. On MySQL, a view whose tables the server does not list.
+- The system catalogs (`information_schema`, `pg_catalog`, `mysql`,
+  `sqlite_schema` and the rest), while anything is hidden: they describe
+  every table.
+- A function your database's users wrote, while anything is hidden: what
+  it reads cannot be seen, and a view that calls one is hidden. On
+  PostgreSQL, so is a table whose row policy calls one; and while the
+  database has an operator, a cast or a type whose function its users
+  wrote, every query is refused, as such a function runs without the query
+  naming it.
+
+Agents are told the names in the list, so that they know why a table is
+missing; they never see what is in it. If a table you hid is renamed or
+dropped, agents may not use the connection at all until you take the name
+out, or put in its new one: its new name would be hidden by nothing.
+
+Tabletist checks every query before it runs, and asks the database what
+the query would read. A function that comes with an extension can still
+read a hidden table, and so can a foreign table that points at it. For
+strict isolation, give the connection a database user without access to
+those tables.
+
+A change in `connections.toml` that hides less, made outside Tabletist,
+waits for your answer: see [below](#changes-made-outside-tabletist).
+[Settings and Files]({% link _reference/settings-and-files.md %}#the-connections-file)
+has the keys.
+
 ## See what agents do
 
 Each connection window has an Agents tab for its connection. It opens
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
index 7ada4ab..190b1df 100644
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -168,10 +168,16 @@ server is on.
   **Statement timeout** in seconds (1 to 60, 10 at first).
 - **Tables** is **All**, **All except** or **Only**. With **All except**,
   agents may read every table but the ones listed; with **Only**, the ones
-  listed and no others, a table made later included. **Add table…** opens
+  listed and no others, so that a table made later is hidden too. **Add table…** opens
   a field for a name, `customers` or `sales.customers`: Enter adds it and
   leaves the field open for the next, Escape closes the field, and **×**
-  takes a name out. A name still in the field when you save, or choose
+  takes a name out. While a window of the connection is open, the field
+  lists the tables its sidebar has loaded that match what you type: Up and
+  Down move in the list, and Enter, Tab or a click adds the one chosen. A
+  listed name that the loaded schemas show is not a table is marked **not
+  in this database**, and under **All except** agents may not use the
+  connection until it is taken out or put right. A name in a schema the
+  sidebar has not loaded is not marked. A name still in the field when you save, or choose
   another of the three, is added with the rest. Each of the two lists is kept while you try the other, and
   what you save is the one chosen. Agents are told the names in the list,
   never what is in the hidden tables: not their rows, their columns, or
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
index 4f68ec6..4691ce2 100644
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -43,6 +43,7 @@ In the connection dialog:
 | Mod+T | Test |
 | Mod+Enter | Save and connect |
 | Backspace, Delete | In the empty Password field: forget the saved password |
+| Up, Down | In Add table…'s list of tables: move. Enter adds the one chosen, and Esc closes the field |
 | Esc | Cancel |
 
 ## The window
````

- [ ] **Step 2: Check them**

Run: `docs/bin/check` and `grep -c "$(printf '\342\200\224')"` on the three pages (a count of em dashes)

Expected: the site checks clean, and no em dash on any of the three pages (`0` each):

```text
HTML-Proofer finished successfully.
```

- [ ] **Step 3: Commit**

```bash
git add docs/_guide/agents.md docs/_reference/connections.md docs/_reference/keyboard-shortcuts.md
git commit -m "Say on the website how to hide tables from agents, and what that hides"
```

---

### Task 4: The master plan

- D.3 as built for 1c-3b; G's row and its 1c: built; "Phase 1c, in six runs" at the top.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: The edits**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index 6be2d90..21b2b0b 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -79,6 +79,11 @@ before the setting, so that when the setting appears everything of it is
 enforced. A hidden table that was renamed or dropped stops agents on that
 connection on PostgreSQL too: C.3 is corrected.
 
+**Built on 2026-10-16: Phase 1c, in six runs.** The checks, the server's
+side, the setting in `connections.toml`, the editor's Tables control, and
+its list of the tables an open window loaded with the website's guide
+(`2026-10-16-mcp-server-1c-3b-table-list.md`). With 1d dropped, Phase 1 is built.
+
 ## Contents
 
 1. [What the code says, in short](#1-what-the-code-says-in-short)
@@ -1843,6 +1848,34 @@ otherwise:
 - **Add table…** takes a typed name only. The list from an open session's
   catalog, and a name that matches nothing marked "not in this database",
   are 1c-3b's.
+  **As built in 1c-3b** (`2026-10-16-mcp-server-1c-3b-table-list.md`), agreed on 2026-10-16: the list is of
+  what the sidebar of an open window of the connection loaded, as quick
+  open searches it, not a new reading of the catalog: every open window
+  of the connection together, one switched to another database left
+  out, and the system's schemas and tables never. While the field holds a
+  name, the loaded tables and views that match it float under it, or over
+  it where there is no room (the Components boards' picker: a shadow on
+  macOS, the accent border on Omarchy), at most eight, with a line that
+  says where they come from. Each is written with its schema, but a home
+  schema's table that no other loaded schema has, where the person typed
+  no schema and the list is not an "Only" one (where a name without a
+  schema reads every schema's); none an entry covers already is offered.
+  Up and Down move in it (`FormTableMove`, the keymap's, taken before the
+  field reads them), and screen readers hear the table chosen; Enter, Tab,
+  a click away or a save adds the one chosen, or what is typed where none
+  is, and a click on a table adds it. A listed name that the loaded
+  schemas show is no table is marked "not in this database" in its chip,
+  in the warning colour: one whose schema is loaded and has none, one
+  whose schema the database does not have, and one without a schema only
+  when every schema is loaded. Under "All except" a line says that agents
+  may not use the connection until it is taken out or put right (C.3).
+  With no window open, nothing is offered and nothing marked. The run's
+  review found the system's schemas keeping every mark away, a schema
+  the database lacks not marked, a press on the list closing the field,
+  Tab dropping the choice, a home table offered without its schema under
+  "Only", the list over the field in a small window, Up and Down moving
+  the caret, two windows' lists not taken together, and SQLite's own
+  tables marked: each is fixed with a test, the list's place by eye.
 - **Omarchy**: the row shows the three words, as the access row does, and
   the names on the line under them; the board draws the chosen word alone
   with the names after it. The keyboard works the row as every row of the
@@ -2449,7 +2482,7 @@ is off until the person turns it on.
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
 | 1b. The server over stdio, with its interface | 9, below | Built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md`, `2026-10-14-mcp-server-1b-4b-log.md`, `2026-10-15-mcp-server-1b-5a-access.md` and `2026-10-15-mcp-server-1b-5b-settings.md`) |
 | 1b on Windows: the way in | 1 | Built (`2026-10-15-mcp-server-1b-6-windows.md`) |
-| 1c. Hidden tables | 5, below | The checks, the server's side, the setting and the editor's control built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`, `2026-10-16-mcp-server-1c-2b-setting.md`, `2026-10-16-mcp-server-1c-3a-tables-control.md`). Add table…'s list and the website's guide are next |
+| 1c. Hidden tables | 6, below | Built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`, `2026-10-16-mcp-server-1c-2b-setting.md`, `2026-10-16-mcp-server-1c-3a-tables-control.md`, `2026-10-16-mcp-server-1c-3b-table-list.md`) |
 | 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |
 
 The move of the connections to `connections.toml` came between 1a and 1b
@@ -2575,7 +2608,7 @@ build; no console target.
 - The third cut in two too, on 2026-10-16: 1c-3a, the editor's Tables
   control in both looks with a typed name, its save and its hint
   (`2026-10-16-mcp-server-1c-3a-tables-control.md`); 1c-3b, Add table…'s list from an open session's catalog, the
-  "not in this database" mark, and the website's guide.
+  "not in this database" mark, and the website's guide (`2026-10-16-mcp-server-1c-3b-table-list.md`).
 - Files: `crates/tabletist-db/src/catalog.rs`, `pg.rs`, `mysql.rs`,
   `sqlite.rs` (what views read, routines, partitions), `agent/` (names,
   dependencies, the plan check, the system catalogs), `tabletist-sqlite-ffi`
````

- [ ] **Step 2: Check the whole of it**

Run: `cargo fmt --all --check`, clippy as above, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, the count of em dashes in the master plan, and `cargo test --locked --workspace --all-targets` with both servers

Expected: fmt and clippy clean, the documentation builds without a warning, no em dash (`0`), and:

```text
3534 passed, 0 failed, 22 ignored
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan that Phase 1c is built"
```

## Self-review

- **Against the design.** D.3's "Add table…: a list from the connection's catalog when a session is open … a name that matches nothing is kept and marked (C.3)", with the departures in "What the design asks, and what is built", agreed on 2026-10-16 or found by the review. The Components boards' picker for the list. The guide's section, which B.3 asks for.
- **A review before the plan.** A reviewer drove the draft headlessly and found, most with a run: the system's schemas, which the sidebar never loads, keeping every name without a schema unmarked on PostgreSQL and MySQL; a name in a schema the database lacks unmarked; a press on the list's note closing the field with half a name added; Tab dropping the table chosen; a home table offered without its schema under "Only", which reads every schema's; the list over the field in the smallest window; Up and Down moving the caret too; two windows of a connection not taken together, and one switched to another database counted; SQLite's own tables marked; the guide claiming marks the code does not make and leaving out what the server refuses; no word to a screen reader on Up and Down; and the list rebuilt on every frame. All fixed: with a test each, the list's place by eye at 720 by 480, and the cost by reading what the windows loaded only while the control has a list.
- **Left, and said.** What a sidebar has not loaded is neither offered nor marked. The mark is the editor's: the server's stop for a renamed table (1c-2a) does not wait for it.
- **Placeholders.** None: every step has its diff or its command and what it prints. Task 2's look is by eye in a throwaway scene and leaves nothing behind.
- **Names.** `LoadedTables`, `offered`, `has`, `OFFERED`, `agents_loaded`, `agents_table_pick`, `loaded_tables`, `FormTableMove` and `take_typed_table` are the same in every task that uses them.
