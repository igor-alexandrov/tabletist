# The start screen, run 1: the recents and the screen. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A connection's window with no open tab shows the start screen in place of "Select a table or view in the sidebar": the connection in one line (the production strip on production), what was opened lately on the connection or, on a first visit, its largest tables by the catalog's estimate, and three ways to start; in the terminal look a dashboard whose numbers open the rows and whose letters start things, with `-- START --` on the status line. The recents are kept in `recents.json` by connection id, and the macOS sidebar loses its Recent list.

**Architecture:** `src/recents.rs` is the file: each saved connection's list of up to eight tables and views, newest first, keyed also by the database the window was on, loaded into `App::recents` and written by the backend (`StateFile::Recents`) when something opens, on Clear, when a listed schema no longer has an entry, and when a connection is deleted. A SQL editor closed with a script is a recent of its window (`Workspace::recent_sql`), opened again with the script. `App::start_recents` and `App::start_largest` are what the screen lists; `src/ui/start.rs` draws it in both looks under the object strip, and the terminal status line's `-- START --` with it. The keymap gets a `Start` scope for the Omarchy keys (`1..9`, `t`, `d`, `e`, `a`), read by `src/ui/keys.rs` while the window has no tab and the keyboard is not in the tree; `focus::Region::Start` is what ctrl+h and ctrl+l step to. Switch database becomes a built command: its chord and the screen's `d` open the menu of the server's other databases by its pop-up id.

**Tech Stack:** Rust 2024, egui, serde and serde_json, jiff (all dependencies already).

---

## Where this sits

Run 1 of four of `2026-10-19-start-screen.md`, the feature's master plan, written from the brief "build the start screen (no tab open)" and the four boards at the end of the "Empty, loading and error states" section of the Tabletist canvas (read on 2026-10-19). This plan is a branch of `main` at `c354a1b`. The search (run 2), the sizes (run 3) and what is waiting with the reopening of closed tabs (run 4) are later runs, each shippable on its own; the master plan says what each holds and why the brief's one pull request is four.

## Where this departs from the boards or the brief

Each row needs Igor's yes before the plan is run.

| What the board or the brief says | What this plan builds | Why |
|---|---|---|
| Brief and board: the Switch database button carries ⇧⌘D | The button carries no key on macOS; it opens the server's other databases under itself. Omarchy's `d` on the screen opens that menu under the screen's own line, and `ctrl+shift+d` anywhere opens it under the bar's own chip: Switch database is a built command now | ⇧⌘D is Review SQL's in the grid (keymap decision 8), and the settled keymap gives Switch database no macOS key |
| Brief: focus starts on the start screen on Omarchy | After a connect the keyboard stays in the sidebar, as today; ctrl+l moves it to the screen, and the screen has it once the last tab closes | `j`, `k` and Enter in the tree open the first table today, and tests pin that the tree has the keys right after a connect. Igor can overrule: one line in `after_connect` puts the keys on the screen when nothing was reopened |
| Brief: a SQL tab is a recent, named by "its first comment line or title" | A SQL editor is a recent once it is closed with a script in it: "Query 1" with the first `--` comment line of its script (or "SQL editor"), kept in the window, not in the file, and opened again as an editor with that script. An editor still open is a tab, and no start screen shows while a tab is; an empty one has nothing to offer | The screen shows only with no tab open, so a recent editor can only be a closed one |
| Brief: "An object that no longer exists is dropped silently the next time the list is shown" | Dropped when its schema is listed (at connect, on Reload), which comes before any list shows; an entry of a schema the sidebar has not listed stays | The listing is what tells whether an object exists |
| Brief: "44 tables and views" of the connection | The count is of the schema the sidebar shows, as its list has it (the terminal look says "44 objects", as its board does) | The sidebar lists one schema at a time; nothing else is read |
| Brief: SQLite "skip the estimate and list tables by name" | The section is titled "Tables" with the note "Nothing is queried.", five tables by name | A list by name is not a list of the largest |
| Brief: Clear empties the list; the Omarchy board shows no Clear | macOS has the Clear link in the section head; Omarchy has no key for it in this run | The board lists the Omarchy keys, and Clear is not among them |
| Board, macOS: the empty strip has a "+" button at its left | The empty strip has no button: New SQL query is on the screen, and SQL Editor in the sidebar | A button that shows only while no tab is open would come and go with the first tab |
| Board, Omarchy: "2h ago", "1-9 open recent"; macOS "2 h ago"; the brief's spoken row "opened 2 hours ago" | Both looks say "2 h ago" (and "Just now", "Yesterday", "Mon", "Oct 14"), the terminal in lower case, and a row's spoken name carries the same words ("opened 2 h ago"); the status line writes the range as the keymap does, `1..9` | One formatter for what is painted and what is read out, and the keymap's spelling |
| Board, Omarchy: the list is headed `recent`; the Mac's "Recent on this connection" | As the boards: `recent` in the terminal look (and `largest tables`, `tables`), "Recent on this connection" on macOS | Each board's words |
| Board, Omarchy: "postgres 16.4" | "postgresql 16.4": the server's own name, lower case as every label of the look | The version string is the server's |
| Brief: the connection line's size, the largest tables' sizes; the search; the waiting notes; ⇧⌘T and `u` | Runs 3, 2 and 4. The macOS status bar says only "No table open" in this run (the right-hand "Start typing to search" comes with the field), and the Omarchy status line lists no `/` yet | Each is a step of its own (see the master plan) |
| Brief: tests 1, 2, 3, 4, 7, 10, 11 | In this run, test 4 without the sizes. Tests 5 and 6 come with run 2, 8 and 9 with run 4 | The runs |

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Where the recents are | `recents.json` in the state directory, beside `session.json` (`AppDirs::recents_file`): `{ "<connection id>": [ { "kind": "table", "database": "bookshop_development", "schema": "public", "name": "book_covers", "opened": 1792404000 }, … ] }`, newest first, eight at most. Written by the backend (`StateFile::Recents`) at every open, on Clear, when a listed schema no longer has an entry, and when a connection is deleted. Kept whatever Restore tabs and filters says | The brief: per connection id, next to the session state, not in `settings.toml` |
| What an entry keys on | The connection's id and the database the window is on (`session::database_of`: a SQLite file's path): a window on another database of the connection shows none of the other's, and drops none | A PostgreSQL window can switch database; `session.json` keys its tables the same way |
| What a newer version wrote | An entry this version cannot read (a value that is no entry) is kept as it was and written back, counts toward the eight, and is never the one dropped, nor is the entry just noted; an entry of a kind this version does not know is read, never shown and never judged by a listing of tables and views; a connection's value that is no list is kept as it was until something opens on that connection here; keys a newer version added to an entry stay with it | As `table_lists.json` and `session.json` keep what they cannot read |
| What counts as an open | `App::open_object`: the sidebar, quick open, a recent, a foreign key's target; a tab opened again that was open already counts too. A tab activated in the strip does not, and nothing is noted for a connection deleted while its window is open | "An object opens → it moves to the top" |
| SQL editors | Closed with a script (anything but blank), an editor goes to the front of `Workspace::recent_sql` (eight kept, in memory): "Query N" with its first `--` comment line, when it was opened. Opened again, it is the editor it was, by its number, script and time (the number the new editor took goes back), and it leaves the list. Clear empties the list too | The departure above |
| What is shown | The five newest by `opened` of the file's entries of this database whose kind this version knows and the window's closed editors (`App::start_recents`). Clear empties this connection's list without asking | The brief: 8 kept, 5 shown |
| The first visit | `App::start_largest`: the tables of the schema the sidebar shows (`Tree::shown_schema`), by the catalog's estimate (`ObjectInfo::estimated_rows`, which the sidebar's listing already reads), the largest first, those with none last by name, five; "Largest tables" with "From table statistics. Nothing is queried." on PostgreSQL and MySQL, "Tables" with "Nothing is queried." on SQLite. Nothing is counted or fetched | The brief: the catalog's estimates, once per connection, never `count(*)` |
| The connection line | The environment's dot (the terminal's tag), the database in the mono face (a SQLite file's name), then "PostgreSQL 16.4 · 44 tables and views" ("44 objects" in the terminal look); the server's kind alone until its version is known, and no count until the schema is listed. The version is asked at every connect now (`App::ask_server_version`, for a SQL editor only before), a pending version read no longer counts as the session in use (`Workspace::waits`), and one left unanswered when a session is closed idle is asked of the next session | The brief: read once when the connection opens, never on every render. The idle tests found the first two |
| The production strip | The production bar's tint and rule (`env_colors(Production)`, whatever Environment color says), the environment badge, the name in the text ink, the words in the environment's ink on macOS (muted in the terminal look) with "Read-only" first where the connection is; the terminal look's strip has the rule above it in the production red | The board, and "Production always looks like this" |
| The keys | `Scope::Start` (`start` in `settings.toml`), normal mode, alongside Global alone; in it, on Omarchy: `1..9` Open a recent object (new command `OpenRecent`), `t` New SQL editor tab, `d` Switch database, `e` Edit connection, `a` Show the Agents tab. The Mac has none: its buttons carry the global keys. `keymap::spell` reads a bare `1..9` as the digits with nothing held | The brief's Omarchy keys; a key is spelled in the keymap alone |
| Switch database, built | `C::SwitchDatabase` loses `.reserved()`. Its chord (`ctrl+shift+d`, Omarchy) opens the bar's own chip's menu, and the screen's `d` the screen's button's, each by its pop-up id (`workspace::database_menu_id(tab, start)`, `menu::under_id`); on a server with one database neither key opens anything (no id is left open for a list that comes later) and the button is disabled with "This connection has no other database to switch to" | A built command answers everywhere it is bound |
| Dropping | When a listing of a schema succeeds, the entries of that database and schema it no longer has go, and the file is written; a listing that failed judges nothing, and other schemas' entries stay | The brief: dropped silently, and a failed listing says nothing of what exists |
| Where the keys are read | The window shows the screen (`Workspace::on_start`: its schema listing has answered, listed or failed, and no tab is open) and the keyboard is not in the tree (`pane == Grid`): `Scope::Start` is the letters' scope, and `OpenRecent`'s digits are read. With `pane == Tree` the sidebar's keys apply (`t` is the tree's). ctrl+h and ctrl+l step among `[Tree, Start, Grid, Panel]` (`focus::Region::Start`); on Omarchy the screen is one Tab stop named "Start", lit with the accent border while it has the keys; a click on it takes them. On macOS the rows and buttons are the Tab stops, and F6 brings the keyboard to the first row, or the first button where there is none. The empty strip is no part to step to | The brief: keys work only while the start screen has focus |
| The macOS strip and status bar | The strip is drawn empty with no tab (no row panel toggle); under the screen a 32 pt status bar in the panel's grey says "No table open" | The board |
| The Omarchy status line | `-- START --` (the mode line's role, muted) while no tab is open, whichever pane has the keys, then `1..9 open recent · t new sql · d database · e edit connection · ctrl+b tables · ? all keys`, those that fit, and "no table open" at the right | The board; the border round the screen says where the keys are |
| Edit connection of a connection deleted while open | The macOS button is disabled with "This connection is no longer saved"; the Omarchy line shows it and does nothing | Nothing to edit |
| The sidebar | The macOS Recent section, `Workspace::recent` and `model::RECENT` go; `agents_said` is shared with the screen. Omarchy's sidebar is as it was | The brief |
| Accessibility | A row is a button named in full: "book_covers, table, public, opened 2 h ago", "Query 1, SQL editor, ebook covers per publisher, opened Yesterday", "reviews, table, public, ≈ 48,200 rows"; the connection line is a label of its words; section titles are labels as written ("Recent on this connection", "Largest tables"; the terminal's "Recent", whatever case the look paints); the status texts are labels | The brief; the headless tests read them |
| Numbers | "≈ 48,200 rows" grouped as Numbers by look says; the terminal look "≈ 48.2k" | The boards |
| `when` | "Just now" under a minute, "N min ago", "N h ago" today, "Yesterday", the weekday within six days, else "Oct 14" ("Oct 14, 2025" in another year); the day is the system zone's; lower case in the terminal look | The brief's examples |
| The website | `_guide/macos.md` (the window), `_guide/omarchy.md` (a "The start screen" section with its keys, `Ctrl+Shift+D` under Everywhere), `_reference/settings-and-files.md` (`recents.json`) | AGENTS.md |
| Tests | `src/recents.rs` (the file and what it keeps of a newer version, `note` under unread entries, `when`, `first_comment`), `src/app.rs` (the version at connect, the switch of database), `src/ui/mod.rs` (the recents, the screen after the last close), `src/ui/start_tests.rs` (the brief's 1, 2, 3, 4, 7, 10 and 11; the line, the strip, the SQL recent and its number, the status line, the Mac's letters, F6 and Tab, a failed listing, a deleted connection); the keymap's generated tests cover the new scope and keys. `tests/keys.rs` lists the new test file | AGENTS.md |

## Before you start

- One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets` and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: views push actions and mutate nothing; keys are spelled in `src/keymap.rs` alone; `Environment` is matched in `src/env.rs` alone; state files stay readable, backward compatible and atomically written.
- Each step's diff applies with `git apply` from the repository's root: save the block to a file and run `git apply <file>`. The code here is formatted.
- The tests use only the Bookshop data. No test compares a picture or a position with the design.

## What was run before this plan was written

Every task's state was built on `main` at `c354a1b` and passed `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets` with the test PostgreSQL, MySQL and SSH servers (the counts are in each task's Step 4) and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Each task's tests were laid over the code before it and run alone: task 1's fail to build, task 2's fail as its Step 2 says. The four boards were read again before the draft, and the screen was rendered in every look, light and dark, through a throwaway scene of `src/shots.rs` (not committed) and compared with the boards by eye: that found the stray dot the macOS strip had after the database's name. A reviewer given the draft wrote throwaway tests against it and found eight defects, each fixed in the task it belongs to with a test there: F6 stuck on the sidebar with no tab (the empty strip registered itself as a part, and the macOS screen claimed none), a SQL editor opened again from the screen given a new number, a listing that failed wiping the schema's recents, an entry of a kind this version does not know dropped by a listing, a connection's value of another shape making the whole file unreadable, `note` able to drop the entry just noted, `first_comment` stopping at an empty `--` line, and an open in the window of a deleted connection writing its id back. Fifteen other things it tried held up: a letter typed in the sidebar's filter field, digits past the rows and `0`, two windows of one connection, a relisting without a table, a switch of database and back, a deleted connection's button, the index after Clear, narrow and short windows, the idle close with a version pending, the strip only on production, the file's order after a round trip, closed editors with blank scripts, the status line at 360 points, every row and button named once. A second reviewer read the generated plan against its diffs and the brief and found eleven points, fixed the same way: two doc comments that called a closed editor open, the terminal list headed as the Mac's rather than `recent`, the menu's id left open on a server with one database, and seven places where the prose said more or other than the code (which pop-up each key opens, when `-- START --` shows, the doc check among the checks, the strip's ink in the terminal look, a failed schema listing, the brief's spoken "2 hours ago", SQLite on the website). Every diff in this plan was applied in order to a copy of `c354a1b`, and the result is the last state. Not run: CI, macOS and Windows, and the app by hand.

## Files

| File | What changes |
|---|---|
| `src/recents.rs` (new) | `Recents`, `Recent`, `KEPT`, `SHOWN`: the file, read and written; `when`, `first_comment` |
| `src/lib.rs`, `src/paths.rs`, `src/backend.rs` | The module; `AppDirs::recents_file`; `StateFile::Recents` |
| `src/model.rs` | `Workspace::recent_sql` (for `recent`), `on_start`; `ClosedSql`, `StartRecent`; `SqlTab::opened`; `Action::OpenRecent`, `ClearRecents`; `waits` without the version |
| `src/app.rs` | `App::recents`, `save_recents`, `start_recents`, `start_largest`, `start_schema`, `reopen_sql`; what `open_object`, `CloseTab`, `SwitchDatabase`, `DeleteConnection`, `Event::Objects`, `ask_server_version` and `close_idle` do with them; the actions |
| `src/keymap.rs` | `Scope::Start`, `Command::OpenRecent`, the Start bindings, Switch database built, a bare `1..9` |
| `src/ui/keys.rs` | The Start scope, its digits and letters, the chord that opens the chip's menu, `Region::Start` as the pane to step from |
| `src/ui/focus.rs`, `src/ui/menu.rs` | `Region::Start` among the panes; `menu::under_id` |
| `src/ui/workspace.rs` | `database_menu_id`, `database_menu` with an id, `bar_info` shared; the screen and its status line drawn with no tab |
| `src/ui/sidebar.rs` | The Recent section goes; `agents_said` shared |
| `src/ui/object_tabs.rs` | The empty strip on macOS |
| `src/ui/start.rs` (new) | The screen in both looks, its status bar and status line |
| `src/env.rs` | `Environment::is_production`; the screen among the surfaces that ask for colours |
| `src/testing.rs` | `Harness::connect_with`, `fixture_objects`, `bookshop_objects` |
| `src/ui/mod.rs`, `src/ui/start_tests.rs` (new), `src/app.rs`, `tests/keys.rs` | The tests |
| `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_reference/settings-and-files.md` | The website |

---

### Task 1: The recents, and the keys of the start screen

The file and everything the screen will read: `recents.json` loaded into `App::recents` and written at every change; a closed SQL editor kept in its window; `App::start_recents` and `App::start_largest`; the version asked at every connect; the `Start` scope with its keys, read by `keys.rs` while a window has no tab, and Switch database built; `Region::Start` among the panes; the pop-up id of the database menu; the sidebar's Recent section gone. The window still says "Select a table or view in the sidebar" after this task: the screen is task 2's.

**Files:** Create `src/recents.rs`; Modify `src/lib.rs`, `src/paths.rs`, `src/backend.rs`, `src/model.rs`, `src/app.rs`, `src/keymap.rs`, `src/ui/keys.rs`, `src/ui/focus.rs`, `src/ui/menu.rs`, `src/ui/workspace.rs`, `src/ui/sidebar.rs`, `src/ui/mod.rs`, `src/testing.rs`, `docs/_reference/settings-and-files.md`.

- [ ] **Step 1: Write the failing tests**

The app's tests of the version at connect and of the database switch, and the sidebar test that reads the recents from the app now.

````diff
diff --git a/src/app.rs b/src/app.rs
index e8c1276..03de4b6 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -11434,9 +11434,21 @@ masked = ["orders.email"]
         let tab = connect_tab(&mut harness);
         let first = open(&mut harness, tab, "users", true);
         open(&mut harness, tab, "orders", true);
-        let sent = harness.app.backend.sent.len();
+        let sent = fetches(&harness);
         assert_eq!(open(&mut harness, tab, "users", false), first);
-        assert_eq!(harness.app.backend.sent.len(), sent);
+        assert_eq!(fetches(&harness), sent, "the recent's save is no fetch");
+    }
+
+    /// How many commands were sent that are not a state file's save: an
+    /// open writes the recents, which asks nothing of the database.
+    fn fetches(harness: &Harness) -> usize {
+        harness
+            .app
+            .backend
+            .sent
+            .iter()
+            .filter(|command| !matches!(command, Command::Save { .. }))
+            .count()
     }
 
     #[test]
@@ -11578,21 +11590,21 @@ masked = ["orders.email"]
     }
 
     #[test]
-    fn connecting_without_a_sql_editor_does_not_ask_for_the_version() {
+    fn connecting_asks_for_the_version_once_per_session() {
         let mut harness = Harness::new();
         let tab = connect_tab(&mut harness);
+        // The start screen's line names the server: asked with the connect,
+        // whether an editor is open or not.
+        assert_eq!(version_requests(&harness, 0).len(), 1);
         open(&mut harness, tab, "users", true);
         harness.app.apply(Action::Reconnect(tab));
-        answer_connect(&mut harness);
-        assert!(version_requests(&harness, 0).is_empty());
+        let sent = harness.app.backend.sent.len();
         assert!(
-            harness
-                .app
-                .workspace(tab)
-                .unwrap()
-                .server_version
-                .needs_load()
+            version_requests(&harness, sent).is_empty(),
+            "not before the session is up"
         );
+        answer_connect(&mut harness);
+        assert_eq!(version_requests(&harness, sent).len(), 1, "once it is");
     }
 
     #[test]
@@ -11732,13 +11744,15 @@ masked = ["orders.email"]
     }
 
     #[test]
-    fn sql_tabs_are_numbered_per_connection_and_ask_for_the_version() {
+    fn sql_tabs_are_numbered_per_connection_and_share_the_version() {
         let mut harness = Harness::new();
         let tab = harness.connect_fake();
         open(&mut harness, tab, "users", true);
         harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
         harness.app.apply(Action::NewSqlTab(tab));
-        assert!(matches!(
+        // Asked with the connect, for the start screen: the editor asks
+        // nothing more.
+        assert!(!matches!(
             last_sent(&harness.app),
             Command::ServerVersion { .. }
         ));
@@ -14298,6 +14312,9 @@ masked = ["orders.email"]
     fn switching_database_keeps_sql_editors_and_drops_object_tabs() {
         let mut harness = Harness::new();
         let tab = connect_tab(&mut harness);
+        // A server with databases to switch between: the recents are each
+        // database's own.
+        harness.app.workspace_mut(tab).unwrap().spec.driver = Driver::Postgres;
         let users = open(&mut harness, tab, "users", true);
         let first = harness.add_sql_tab(tab);
         open(&mut harness, tab, "orders", true);
@@ -14308,7 +14325,7 @@ masked = ["orders.email"]
         workspace.active_tab = Some(users);
         workspace.sql_tab_mut(second).unwrap().text = "SELECT 1".into();
         assert!(workspace.sql_tab(second).unwrap().is_running());
-        assert_eq!(workspace.recent.len(), 2);
+        assert_eq!(harness.app.start_recents(tab).len(), 2);
         harness.app.apply(Action::SwitchDatabase {
             tab,
             database: "other".into(),
@@ -14316,7 +14333,10 @@ masked = ["orders.email"]
         let workspace = harness.app.workspace(tab).unwrap();
         assert_eq!(strip(&harness, tab), vec![first, second]);
         assert_eq!(workspace.object_tabs().count(), 0);
-        assert!(workspace.recent.is_empty(), "they are the other database's");
+        assert!(
+            harness.app.start_recents(tab).is_empty(),
+            "they are the other database's"
+        );
         assert_eq!(
             workspace.active_tab,
             Some(first),
@@ -16439,14 +16459,13 @@ masked = ["orders.email"]
             let mut harness = Harness::new();
             let (tab, _, used) = loaded(&mut harness);
             harness.app.close_idle(used + 30 * MINUTE);
+            let closed = harness.app.backend.sent.len();
             harness.add_sql_tab(tab);
             assert!(
-                !harness
-                    .app
-                    .backend
-                    .sent
+                !harness.app.backend.sent[closed..]
                     .iter()
-                    .any(|c| matches!(c, Command::ServerVersion { .. }))
+                    .any(|c| matches!(c, Command::ServerVersion { .. })),
+                "nothing is asked of a closed session"
             );
             harness.app.apply(Action::Refresh(tab));
             let (session, request) = last_connect(&harness);
@@ -18946,7 +18965,7 @@ masked = ["orders.email"]
             harness
                 .app
                 .apply(Action::ActivateTab { tab, id: users_tab });
-            let before = harness.app.backend.sent.len();
+            let before = super::fetches(&harness);
             let follow = || Action::FollowForeignKey {
                 tab,
                 object: ObjectRef::new("main", "orders"),
@@ -18955,7 +18974,7 @@ masked = ["orders.email"]
             };
             harness.app.apply(follow());
             assert_eq!(leave_prompt(&harness), Some(true));
-            assert_eq!(harness.app.backend.sent.len(), before, "nothing ran");
+            assert_eq!(super::fetches(&harness), before, "nothing ran");
             let held = object(&harness, tab, orders);
             assert!(held.filter.rows.is_empty() && held.query.filters.is_empty());
             assert_eq!(held.edits.cells.len(), 1);
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index e213b30..573a969 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -490,22 +490,23 @@ mod tests {
     }
 
     #[test]
-    fn opening_an_object_puts_it_first_in_recent() {
+    fn opening_an_object_puts_it_first_in_the_recents() {
         let mut harness = Harness::new();
         let tab = harness.connect_fake();
         harness.click("orders");
         harness.click("users");
         let recent: Vec<String> = harness
             .app
-            .workspace(tab)
-            .unwrap()
-            .recent
+            .start_recents(tab)
             .iter()
-            .map(|(object, _)| object.name.clone())
+            .map(|recent| match recent {
+                crate::model::StartRecent::Object { object, .. } => object.name.clone(),
+                crate::model::StartRecent::Sql { number, .. } => format!("Query {number}"),
+            })
             .collect();
         assert_eq!(recent, ["users", "orders"]);
         harness.set_look(crate::theme::Look::macos());
-        assert!(harness.has("Recent orders"), "macOS lists them");
+        assert!(!harness.has("Recent orders"), "the sidebar lists none");
     }
 
     #[test]
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib -- start_recents opening_an_object_puts_it_first`

Expected: the build fails, as `App::start_recents` does not exist yet:

```
error[E0433]: cannot find `StartRecent` in `model`
error[E0433]: cannot find `StartRecent` in `model`
error[E0599]: no method named `start_recents` found for struct `app::App` in the current scope
error[E0599]: no method named `start_recents` found for struct `app::App` in the current scope
error[E0599]: no method named `start_recents` found for struct `app::App` in the current scope
error: could not compile `tabletist` (lib test) due to 5 previous errors
red exit 101
```

- [ ] **Step 3: Write the code**

The new module holds its own tests (the file, `when`, `first_comment`).

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 8fbe9dd..02a3f7e 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -186,6 +186,7 @@ An administrator's policy, `managed.toml`, is in a third place of its own:
 | `table_lists.json` | State | How each saved connection lists its tables, where you switched it in the sidebar |
 | `connections.moved` | State | A note that your connections are kept in `connections.toml` |
 | `session.json` | State | The connection windows open when Tabletist last quit, and each saved connection's window as it last had it: its table tabs, with their sort and filters, and whether the row panel was shown. Written at quit, when a connection's window closes or gets another connection, when a database switch drops its tables, and when a connection is deleted. Not written while `connections.toml` cannot be read. A connection deleted from `connections.toml` by hand is taken out of it at the next write |
+| `recents.json` | State | What you opened last on each saved connection, by its id: up to eight tables and views, with the database each is in and when it was opened, for the start screen. Written when something opens, when Clear is pressed, when an object the database no longer lists is dropped, and when a connection is deleted |
 | `labels.json` | State | The column you chose to label each table's rows in a foreign key's list, by saved connection and, on PostgreSQL, by database |
 | `tabletist.log`, `panic.log` | State | Logs |
 | `agents.sqlite3` | State | Every call agents made: the tool, the statement and its values, and how it ended. Never a row |
diff --git a/src/app.rs b/src/app.rs
index 03de4b6..9f8467e 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -19,8 +19,8 @@ use crate::model::{Action, ConnTab, ConnTabContent, ConnTabId, PickerState};
 use crate::model::{
     Advance, CellPos, Completion, ConnectionForm, Dialog, Fetch, FilterBar, FilterRow, Held,
     HostKeyPrompt, LeavePrompt, ObjectTab, ObjectView, Pane, PasswordPrompt, PickTarget, QuickOpen,
-    Reach, ResultPane, RunMode, SecretKind, SessionStatus, SqlTab, Tab, TabId, TestState,
-    TextPrint, Tree, TreeKey, TreeNode, Wanted, Workspace,
+    Reach, ResultPane, RunMode, SecretKind, SessionStatus, SqlTab, StartRecent, Tab, TabId,
+    TestState, TextPrint, Tree, TreeKey, TreeNode, Wanted, Workspace,
 };
 use crate::paths::AppDirs;
 use crate::secrets::{SecretString, password_account, ssh_account};
@@ -132,6 +132,8 @@ pub struct App {
     /// not be read, when no window could be open and the file would lose
     /// every connection's tables.
     writes_session: bool,
+    /// What was opened lately on each connection: `recents.json`.
+    pub recents: crate::recents::Recents,
     pub dialog: Option<Dialog>,
     /// Keyring reads in flight, and what each is for.
     pending_secrets: HashMap<RequestId, SecretPurpose>,
@@ -291,6 +293,7 @@ impl App {
         };
         let (table_lists, table_lists_unread) = load_table_lists(&dirs.table_lists_file());
         let session = crate::session::Session::load(&dirs.session_file());
+        let recents = crate::recents::Recents::load(&dirs.recents_file());
         let writes_session = loaded_connections.problem.is_none();
         // An installed data font is read once the window exists (`attach`).
         let look = crate::theme::Look::for_platform().with_text(&settings, false);
@@ -316,6 +319,7 @@ impl App {
             table_lists_unread,
             session,
             writes_session,
+            recents,
             dialog: None,
             pending_secrets: HashMap::new(),
             pending_stores: HashMap::new(),
@@ -1180,6 +1184,19 @@ impl App {
                     && let Some(index) = workspace.tabs.iter().position(|t| t.id() == id)
                 {
                     let closed = workspace.tabs.remove(index);
+                    // An editor with a script is offered again on the
+                    // start screen; an empty one has nothing to offer.
+                    if let Some(sql) = closed.as_sql().filter(|sql| !sql.text.trim().is_empty()) {
+                        workspace.recent_sql.insert(
+                            0,
+                            crate::model::ClosedSql {
+                                number: sql.number,
+                                text: sql.text.clone(),
+                                opened: sql.opened,
+                            },
+                        );
+                        workspace.recent_sql.truncate(crate::recents::KEPT);
+                    }
                     let session = workspace.session;
                     let editor = closed.as_sql().map(|sql| (tab, sql.id));
                     if workspace.active_tab == Some(id) {
@@ -1344,6 +1361,45 @@ impl App {
                     workspace.pane = Pane::Grid;
                 }
             }
+            Action::OpenRecent { tab, index } => {
+                // The screen's rows, as it numbers them: the recents, or
+                // on a first visit the largest tables.
+                let recents = self.start_recents(tab);
+                let row = if recents.is_empty() {
+                    self.start_largest(tab)
+                        .get(index)
+                        .map(|(object, _)| StartRecent::Object {
+                            object: object.clone(),
+                            kind: ObjectKind::Table,
+                            opened: 0,
+                        })
+                } else {
+                    recents.get(index).cloned()
+                };
+                match row {
+                    Some(StartRecent::Object { object, kind, .. }) => {
+                        self.apply(Action::OpenObject {
+                            tab,
+                            object,
+                            kind,
+                            pin: true,
+                        });
+                    }
+                    Some(StartRecent::Sql { closed, .. }) => self.reopen_sql(tab, closed),
+                    None => {}
+                }
+            }
+            Action::ClearRecents(tab) => {
+                let Some(conn) = self.workspace_mut(tab).map(|workspace| {
+                    workspace.recent_sql.clear();
+                    workspace.conn_id.0.clone()
+                }) else {
+                    return;
+                };
+                if self.recents.clear(&conn) {
+                    self.save_recents();
+                }
+            }
             Action::SelectCell { tab, id, cell } => {
                 // The editor's text is kept, and the note of a locked cell
                 // was about the cell the selection leaves.
@@ -2415,7 +2471,6 @@ impl App {
                     workspace
                         .tabs
                         .retain(|open| matches!(open, Tab::Sql(_) | Tab::Agents(_)));
-                    workspace.recent.clear();
                     workspace.active_tab = workspace
                         .active_tab
                         .filter(|id| workspace.tab(*id).is_some())
@@ -2531,6 +2586,10 @@ impl App {
                     if self.session.forget(&id) {
                         self.save_session();
                     }
+                    // And what was opened lately on it.
+                    if self.recents.clear(&id.0) {
+                        self.save_recents();
+                    }
                     // A launch that was to open it shows the connections.
                     let chosen = crate::settings::Launch::Connection {
                         id: id.clone(),
@@ -3599,6 +3658,8 @@ impl App {
                 schema,
                 result,
             } => {
+                // A listing that failed says nothing of what exists.
+                let listed = result.is_ok();
                 if let Some(tab) = self.tab_for_session(session)
                     && let Some(workspace) = self.workspace_mut(tab)
                     && let Some(node) = workspace.tree.nodes.get_mut(&schema)
@@ -3623,6 +3684,25 @@ impl App {
                             object.estimated_rows = Some(estimate);
                         }
                     }
+                    // A recent of the schema that the list no longer has
+                    // is gone: it is dropped, without a word.
+                    let conn = workspace.conn_id.0.clone();
+                    let database = crate::session::database_of(&workspace.spec);
+                    let listed_names: Vec<String> = workspace
+                        .tree
+                        .nodes
+                        .get(&schema)
+                        .and_then(|node| node.objects.value.as_ref())
+                        .map(|objects| objects.iter().map(|info| info.name.clone()).collect())
+                        .unwrap_or_default();
+                    let present = |name: &str| listed_names.iter().any(|known| known == name);
+                    if listed
+                        && self
+                            .recents
+                            .drop_missing(&conn, &database, &schema, present)
+                    {
+                        self.save_recents();
+                    }
                 }
             }
             Event::Rows {
@@ -4161,6 +4241,114 @@ impl App {
         });
     }
 
+    /// Saves what was opened lately on each connection, on the backend.
+    fn save_recents(&mut self) {
+        self.backend.send(Command::Save {
+            path: self.dirs.recents_file(),
+            file: StateFile::Recents(self.recents.clone()),
+        });
+    }
+
+    /// What the start screen of `tab` lists as recent, newest first and at
+    /// most [`crate::recents::SHOWN`]: the connection's tables and views of
+    /// the window's database, those whose kind this version knows, and the
+    /// window's SQL editors closed with a script.
+    pub fn start_recents(&self, tab: ConnTabId) -> Vec<StartRecent> {
+        let Some(workspace) = self.workspace(tab) else {
+            return Vec::new();
+        };
+        let database = crate::session::database_of(&workspace.spec);
+        let mut rows: Vec<StartRecent> = self
+            .recents
+            .of(&workspace.conn_id.0, &database)
+            .into_iter()
+            .filter_map(|recent| {
+                Some(StartRecent::Object {
+                    object: recent.object(),
+                    kind: recent.object_kind()?,
+                    opened: recent.opened,
+                })
+            })
+            .collect();
+        rows.extend(
+            workspace
+                .recent_sql
+                .iter()
+                .enumerate()
+                .map(|(closed, sql)| StartRecent::Sql {
+                    closed,
+                    number: sql.number,
+                    comment: crate::recents::first_comment(&sql.text),
+                    opened: sql.opened,
+                }),
+        );
+        rows.sort_by_key(|row| std::cmp::Reverse(row.opened()));
+        rows.truncate(crate::recents::SHOWN);
+        rows
+    }
+
+    /// What the start screen of `tab` lists on a first visit, with no
+    /// recent to show: the shown schema's largest tables by the catalog's
+    /// estimate, at most [`crate::recents::SHOWN`], with that estimate.
+    /// Tables the catalog has no estimate for come last, by name, so a
+    /// SQLite database lists its tables by name. Nothing is queried.
+    pub fn start_largest(&self, tab: ConnTabId) -> Vec<(ObjectRef, Option<u64>)> {
+        let Some(workspace) = self.workspace(tab) else {
+            return Vec::new();
+        };
+        let Some(schema) = self.start_schema(tab) else {
+            return Vec::new();
+        };
+        let schema = schema.as_str();
+        let mut tables: Vec<(ObjectRef, Option<u64>)> = workspace
+            .tree
+            .nodes
+            .get(schema)
+            .and_then(|node| node.objects.value.as_deref())
+            .into_iter()
+            .flatten()
+            .filter(|info| info.kind == ObjectKind::Table)
+            .map(|info| (ObjectRef::new(schema, &info.name), info.estimated_rows))
+            .collect();
+        tables.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.name.cmp(&b.0.name)));
+        tables.truncate(crate::recents::SHOWN);
+        tables
+    }
+
+    /// The schema the start screen counts and lists tables of: the one
+    /// the sidebar shows.
+    pub fn start_schema(&self, tab: ConnTabId) -> Option<String> {
+        let workspace = self.workspace(tab)?;
+        workspace
+            .tree
+            .shown_schema(workspace.driver, self.settings.show_system_schemas)
+    }
+
+    /// Opens the start screen's SQL recent at `closed`: an editor with
+    /// the script the closed one had, which then is no recent any more.
+    fn reopen_sql(&mut self, tab: ConnTabId, closed: usize) {
+        let Some(kept) = self
+            .workspace(tab)
+            .and_then(|workspace| workspace.recent_sql.get(closed).cloned())
+        else {
+            return;
+        };
+        let Some(id) = self.new_sql_tab(tab) else {
+            return;
+        };
+        if let Some(workspace) = self.workspace_mut(tab) {
+            workspace.recent_sql.remove(closed);
+            // It is the editor it was, by its number too: the one the new
+            // editor took goes back.
+            workspace.next_query -= 1;
+            if let Some(sql) = workspace.sql_tab_mut(id) {
+                sql.text = kept.text;
+                sql.opened = kept.opened;
+                sql.number = kept.number;
+            }
+        }
+    }
+
     /// Saves when each connection last connected, on the backend. A file
     /// of its own: a connect does not write the file of connections.
     fn save_last_used(&mut self) {
@@ -4814,7 +5002,6 @@ impl App {
     /// opened while the session was closed for being idle).
     fn ask_server_version(&mut self, tab: ConnTabId) {
         if let Some(workspace) = self.workspace(tab)
-            && workspace.sql_tabs().next().is_some()
             && workspace.server_version.needs_load()
         {
             let session = workspace.session;
@@ -5859,12 +6046,23 @@ impl App {
     pub fn open_object(&mut self, tab: ConnTabId, object: ObjectRef, kind: ObjectKind, pin: bool) {
         let page_size = self.settings.page_size;
         let new_id = TabId(self.next_id());
+        // Opened, it is the connection's newest recent, whether its tab
+        // was open already or not. A connection deleted while its window
+        // is open keeps nothing: its entry went with it.
+        if let Some(workspace) = self
+            .workspace(tab)
+            .filter(|workspace| self.connections.get(&workspace.conn_id).is_some())
+        {
+            let database = crate::session::database_of(&workspace.spec);
+            let conn = workspace.conn_id.0.clone();
+            let now = crate::util::now_secs();
+            let recent = crate::recents::Recent::of(&object, kind, &database, now);
+            self.recents.note(&conn, recent);
+            self.save_recents();
+        }
         let Some(workspace) = self.workspace_mut(tab) else {
             return;
         };
-        workspace.recent.retain(|(seen, _)| *seen != object);
-        workspace.recent.insert(0, (object.clone(), kind));
-        workspace.recent.truncate(crate::model::RECENT);
         workspace.tree.reveal(&object);
         let existing = workspace
             .object_tabs_mut()
@@ -6565,6 +6763,9 @@ impl App {
                 Some(left) => next = Some(next.map_or(left, |next| next.min(left))),
                 None => {
                     workspace.status = SessionStatus::Idle;
+                    // The version it was asked for, where it had not
+                    // answered, is asked of the next session.
+                    workspace.server_version.pending = None;
                     due.push(workspace.session);
                 }
             }
diff --git a/src/backend.rs b/src/backend.rs
index 1d3be00..f3ef447 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -235,6 +235,8 @@ pub enum StateFile {
     TableLists(std::collections::BTreeMap<String, serde_json::Value>),
     /// The connection tabs open, and each connection's tables.
     Session(crate::session::Session),
+    /// What was opened lately on each connection.
+    Recents(crate::recents::Recents),
 }
 
 impl StateFile {
@@ -251,6 +253,7 @@ impl StateFile {
             Self::AgentsShown(shown) => shown.save(path),
             Self::TableLists(lists) => crate::util::save_json(path, lists),
             Self::Session(session) => session.save(path),
+            Self::Recents(recents) => recents.save(path),
         }
     }
 }
diff --git a/src/keymap.rs b/src/keymap.rs
index 6d7957a..134a6b5 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -62,10 +62,12 @@ pub enum Scope {
     Prompt,
     /// The Agents tab.
     Agents,
+    /// The start screen: a connection's window with no tab open.
+    Start,
 }
 
 impl Scope {
-    pub const ALL: [Scope; 10] = [
+    pub const ALL: [Scope; 11] = [
         Scope::Global,
         Scope::Connections,
         Scope::Sidebar,
@@ -76,6 +78,7 @@ impl Scope {
         Scope::Results,
         Scope::Prompt,
         Scope::Agents,
+        Scope::Start,
     ];
 
     /// The scope's name in `settings.toml` (`[keys.<scope>]`).
@@ -91,6 +94,7 @@ impl Scope {
             Self::Results => "results",
             Self::Prompt => "prompt",
             Self::Agents => "agents",
+            Self::Start => "start",
         }
     }
 
@@ -358,6 +362,7 @@ commands! {
     ChangeAgentAccess,
     MoveInAgentLevels,
     ShowAgentsFromBox,
+    OpenRecent,
     ToggleSidebar,
     PaneLeft,
     PaneDown,
@@ -578,7 +583,7 @@ impl Command {
             C::GoToConnectionWindow => {
                 info("Go to connection window", "Go to connection window", Window)
             }
-            C::SwitchDatabase => info("Switch database", "Switch database", Window).reserved(),
+            C::SwitchDatabase => info("Switch database", "Switch database", Window),
             C::Settings => info("Settings", "Settings", Window),
             C::LargerText => info("Larger text", "", Window),
             C::SmallerText => info("Smaller text", "", Window),
@@ -623,6 +628,7 @@ impl Command {
             )),
             C::MoveInAgentLevels => agents(info("Move among what agents may do", "", Prompts)),
             C::ShowAgentsFromBox => agents(info("Show the connection's Agents tab", "", Prompts)),
+            C::OpenRecent => info("Open a recent object", "", Navigation),
             C::ToggleAgentsLogScope => agents(info(
                 "Show every connection's calls in the log, or this one's",
                 "",
@@ -901,7 +907,8 @@ impl Binding {
                 | Scope::Grid
                 | Scope::Inspector
                 | Scope::Results
-                | Scope::Agents => Mode::Normal,
+                | Scope::Agents
+                | Scope::Start => Mode::Normal,
             },
             set => set,
         }
@@ -938,6 +945,7 @@ const PROMPT: &[Scope] = &[Scope::Prompt];
 const ROWS: &[Scope] = &[Scope::Grid, Scope::Results];
 const AGENTS: &[Scope] = &[Scope::Agents];
 const TREE_AND_GRID: &[Scope] = &[Scope::Sidebar, Scope::Grid];
+const START: &[Scope] = &[Scope::Start];
 const CELLS: &[Scope] = &[Scope::Grid, Scope::Inspector];
 
 /// The keymap. The rows of the settled table first in each part, then the
@@ -1114,6 +1122,14 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k("shift+f6")]),
         ),
         bind(C::SwitchTab, GLOBAL, NONE, keys(&[k("alt+1..9")])),
+        // The start screen's own keys, while it has the keyboard: a
+        // numbered recent, and the letters of its actions. On the Mac the
+        // screen's buttons carry the global keys.
+        bind(C::OpenRecent, START, NONE, keys(&[k("1..9")])),
+        bind(C::NewSqlTab, START, NONE, keys(&[k("t")])),
+        bind(C::SwitchDatabase, START, NONE, keys(&[k("d")])),
+        bind(C::EditConnection, START, NONE, keys(&[k("e")])),
+        bind(C::ShowAgents, START, NONE, keys(&[k("a")])),
         bind(
             C::CloseTab,
             GLOBAL,
@@ -2029,6 +2045,10 @@ pub fn spell(keys: &'static str) -> Result<Spelled, Unread> {
         let mods = Mods::default();
         return Ok(Spelled::Strokes(vec![Stroke::Key { mods, key }]));
     }
+    // The digits with nothing held: the start screen's rows.
+    if keys == "1..9" {
+        return Ok(Spelled::Digits(Mods::default()));
+    }
     let parts: Vec<&str> = keys.split('+').collect();
     let modifier = |part: &str| matches!(part, "cmd" | "ctrl" | "alt" | "shift");
     if parts.len() > 1 && modifier(parts[0]) {
diff --git a/src/lib.rs b/src/lib.rs
index e72d626..2f8d129 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -21,6 +21,7 @@ pub mod macos;
 pub mod managed;
 pub mod model;
 pub mod paths;
+pub mod recents;
 pub mod relay;
 pub mod review;
 pub mod secrets;
diff --git a/src/model.rs b/src/model.rs
index 36b5528..9e88e42 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -132,6 +132,16 @@ pub enum Action {
     },
     /// Switch the sidebar between prefix groups and one flat list.
     ToggleFlatTree(ConnTabId),
+    /// Open the start screen's recent at `index`, as the screen lists
+    /// them: a table or view of the connection's list, or a SQL editor of
+    /// this window.
+    OpenRecent {
+        tab: ConnTabId,
+        index: usize,
+    },
+    /// Empty the connection's recents, and the window's SQL editors among
+    /// them. Nothing is lost, so nothing asks.
+    ClearRecents(ConnTabId),
     /// Hide or show the sidebar.
     ToggleSidebar(ConnTabId),
     /// Show timestamps in the grid to the microsecond, or to the second.
@@ -963,8 +973,11 @@ pub struct Workspace {
     pub needs_ssh_prompt: Option<String>,
     /// Where the arrow keys go.
     pub pane: Pane,
-    /// Objects opened lately, newest first.
-    pub recent: Vec<(ObjectRef, ObjectKind)>,
+    /// The SQL editors closed in this window that had a script, newest
+    /// first, at most [`crate::recents::KEPT`]: the start screen lists
+    /// them with the tables and views of `recents.json`, which are the
+    /// connection's, and opens one again with its script.
+    pub recent_sql: Vec<ClosedSql>,
     pub sidebar_hidden: bool,
     /// The grid shows timestamps in full rather than to the second.
     pub full_precision: bool,
@@ -1016,9 +1029,6 @@ pub struct Workspace {
     pub keep_access: bool,
 }
 
-/// How many objects the sidebar's Recent section keeps.
-pub const RECENT: usize = 5;
-
 #[derive(Debug)]
 pub enum SessionStatus {
     Connecting {
@@ -3034,6 +3044,48 @@ impl Tree {
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
 pub struct TabId(pub u64);
 
+/// One row of the start screen's recents: a table or view of the
+/// connection's list (`recents.json`), or a SQL editor closed in this
+/// window with a script in it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum StartRecent {
+    Object {
+        object: ObjectRef,
+        kind: ObjectKind,
+        opened: u64,
+    },
+    Sql {
+        /// Its place in the window's closed editors
+        /// (`Workspace::recent_sql`).
+        closed: usize,
+        /// The editor's number: `Query 1`.
+        number: u32,
+        /// The first comment line of its script, where it has one.
+        comment: Option<String>,
+        opened: u64,
+    },
+}
+
+/// A SQL editor closed with a script in it, as the start screen offers it
+/// again.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct ClosedSql {
+    /// The number it had: `Query 1`.
+    pub number: u32,
+    pub text: String,
+    /// When it was opened, in seconds since the Unix epoch.
+    pub opened: u64,
+}
+
+impl StartRecent {
+    /// When it was opened, in seconds since the Unix epoch.
+    pub fn opened(&self) -> u64 {
+        match self {
+            Self::Object { opened, .. } | Self::Sql { opened, .. } => *opened,
+        }
+    }
+}
+
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
 pub enum ObjectView {
     #[default]
@@ -4125,6 +4177,9 @@ pub struct SqlTab {
     pub id: TabId,
     /// "Query 3".
     pub number: u32,
+    /// When it was opened, in seconds since the Unix epoch: what the start
+    /// screen says of it once it is closed.
+    pub opened: u64,
     pub text: String,
     /// Byte offset of the editor's cursor, reported by the view.
     pub cursor: usize,
@@ -4194,6 +4249,7 @@ impl SqlTab {
         Self {
             id,
             number,
+            opened: crate::util::now_secs(),
             text: String::new(),
             cursor: 0,
             limit,
@@ -4505,7 +4561,7 @@ impl Workspace {
             save_ssh: false,
             needs_ssh_prompt: None,
             pane: Pane::Tree,
-            recent: Vec::new(),
+            recent_sql: Vec::new(),
             sidebar_hidden: false,
             full_precision: false,
             zone: None,
@@ -4524,6 +4580,12 @@ impl Workspace {
         }
     }
 
+    /// Whether the window shows the start screen: its schema listing has
+    /// answered (listed, or failed), and no tab is open.
+    pub fn on_start(&self) -> bool {
+        self.opened() && self.active_tab.is_none()
+    }
+
     pub fn tab(&self, id: TabId) -> Option<&Tab> {
         self.tabs.iter().find(|tab| tab.id() == id)
     }
@@ -4680,7 +4742,8 @@ impl Workspace {
                 .values()
                 .any(|node| node.objects.is_loading())
             || self.databases.is_loading()
-            || self.server_version.is_loading()
+            // Not the server's version: asked with every connect for the
+            // start screen's line, and no use of the session.
             || self.columns.values().any(Fetch::is_loading)
     }
 
diff --git a/src/paths.rs b/src/paths.rs
index 5fb4cb2..48e1fbc 100644
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -135,6 +135,14 @@ impl AppDirs {
         self.state.join("session.json")
     }
 
+    /// What was opened lately on each saved connection, for the start
+    /// screen. Of this computer, and written when something opens, when
+    /// Clear is pressed, when an object listed no more is dropped, and when
+    /// a connection is deleted.
+    pub fn recents_file(&self) -> PathBuf {
+        self.state.join("recents.json")
+    }
+
     /// The note that the connections are kept in `connections.toml`: made
     /// at the first start that finds that file. Once it is there, what an
     /// older version kept in `connections.json` is never read again,
diff --git a/src/recents.rs b/src/recents.rs
new file mode 100644
index 0000000..b377b0b
--- /dev/null
+++ b/src/recents.rs
@@ -0,0 +1,530 @@
+//! What was opened lately on each saved connection: the start screen's
+//! recents, kept in `recents.json` by connection id.
+//!
+//! A connection keeps up to [`KEPT`] tables and views, newest first, and
+//! the start screen shows [`SHOWN`] of them: those of the database its
+//! window is on. An entry this version cannot read (a kind a newer version
+//! added, a value that is no entry) is kept as the file had it and written
+//! back, so a newer version's recents survive an older one.
+
+use std::collections::BTreeMap;
+use std::path::Path;
+
+use jiff::Timestamp;
+use jiff::tz::TimeZone;
+use serde::{Deserialize, Serialize};
+use tabletist_db::{ObjectKind, ObjectRef};
+
+/// How many a connection keeps.
+pub const KEPT: usize = 8;
+
+/// How many the start screen shows.
+pub const SHOWN: usize = 5;
+
+/// One table or view opened on a connection.
+#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Recent {
+    /// `table`, `view` or `materialized view`.
+    pub kind: String,
+    /// The database it is in, as [`crate::session::database_of`] names
+    /// it: a window on another database of the connection does not show
+    /// it, and does not drop it.
+    pub database: String,
+    pub schema: String,
+    pub name: String,
+    /// When it was last opened, in seconds since the Unix epoch.
+    pub opened: u64,
+    /// What a newer version wrote beside these, written back as it was.
+    #[serde(flatten)]
+    pub rest: BTreeMap<String, serde_json::Value>,
+}
+
+impl Recent {
+    pub fn of(object: &ObjectRef, kind: ObjectKind, database: &str, opened: u64) -> Self {
+        Self {
+            kind: kind_name(kind).to_owned(),
+            database: database.to_owned(),
+            schema: object.schema.clone(),
+            name: object.name.clone(),
+            opened,
+            rest: BTreeMap::new(),
+        }
+    }
+
+    pub fn object(&self) -> ObjectRef {
+        ObjectRef::new(&self.schema, &self.name)
+    }
+
+    /// The kind as the sidebar knows it; none for a kind this version
+    /// does not know, which is kept and not shown.
+    pub fn object_kind(&self) -> Option<ObjectKind> {
+        match self.kind.as_str() {
+            "table" => Some(ObjectKind::Table),
+            "view" => Some(ObjectKind::View),
+            "materialized view" => Some(ObjectKind::MaterializedView),
+            _ => None,
+        }
+    }
+
+    /// Whether this is `object` of `database`.
+    fn is(&self, database: &str, object: &ObjectRef) -> bool {
+        self.database == database && self.schema == object.schema && self.name == object.name
+    }
+}
+
+/// A kind as the file writes it.
+pub fn kind_name(kind: ObjectKind) -> &'static str {
+    match kind {
+        ObjectKind::Table => "table",
+        ObjectKind::View => "view",
+        ObjectKind::MaterializedView => "materialized view",
+    }
+}
+
+/// One entry of a connection's list: read, or kept as the file had it.
+#[derive(Clone, Debug, PartialEq, Eq)]
+enum Entry {
+    Read(Recent),
+    Unread(serde_json::Value),
+}
+
+impl Entry {
+    fn read(&self) -> Option<&Recent> {
+        match self {
+            Self::Read(recent) => Some(recent),
+            Self::Unread(_) => None,
+        }
+    }
+
+    fn value(&self) -> serde_json::Value {
+        match self {
+            Self::Read(recent) => serde_json::to_value(recent).unwrap_or_default(),
+            Self::Unread(value) => value.clone(),
+        }
+    }
+}
+
+/// The file: each saved connection's recents by its id, newest first.
+#[derive(Clone, Debug, Default, PartialEq, Eq)]
+pub struct Recents {
+    lists: BTreeMap<String, Vec<Entry>>,
+    /// A connection's value that is no list: a newer version's shape,
+    /// kept as it was and written back, until something opens on that
+    /// connection here.
+    unread: BTreeMap<String, serde_json::Value>,
+}
+
+/// The file's shape on disk: each connection's value, a list here.
+type File = BTreeMap<String, serde_json::Value>;
+
+impl Recents {
+    /// Reads the file. A missing file is empty; one that is not JSON is
+    /// kept aside (see [`crate::util::load_json`]). An entry that is not
+    /// one of this version's, and a connection's value that is no list,
+    /// are kept to be written back.
+    pub fn load(path: &Path) -> Self {
+        let file: File = crate::util::load_json(path);
+        let mut recents = Self::default();
+        for (id, value) in file {
+            let serde_json::Value::Array(entries) = value else {
+                recents.unread.insert(id, value);
+                continue;
+            };
+            let entries = entries
+                .into_iter()
+                .map(
+                    |value| match serde_json::from_value::<Recent>(value.clone()) {
+                        Ok(recent) => Entry::Read(recent),
+                        Err(_) => Entry::Unread(value),
+                    },
+                )
+                .collect();
+            recents.lists.insert(id, entries);
+        }
+        recents
+    }
+
+    pub fn save(&self, path: &Path) -> std::io::Result<()> {
+        let mut file: File = self
+            .lists
+            .iter()
+            .map(|(id, entries)| {
+                let entries = entries.iter().map(Entry::value).collect();
+                (id.clone(), serde_json::Value::Array(entries))
+            })
+            .collect();
+        file.extend(
+            self.unread
+                .iter()
+                .map(|(id, value)| (id.clone(), value.clone())),
+        );
+        crate::util::save_json(path, &file)
+    }
+
+    /// Puts `recent` first in `conn`'s list, in place of the same object
+    /// opened before, and keeps the list to [`KEPT`]. What this version
+    /// cannot read counts and is never the one dropped: the oldest read
+    /// entry after the new one is. A value of the connection that was no
+    /// list gives way to the list.
+    pub fn note(&mut self, conn: &str, recent: Recent) {
+        self.unread.remove(conn);
+        let entries = self.lists.entry(conn.to_owned()).or_default();
+        entries.retain(|entry| {
+            entry
+                .read()
+                .is_none_or(|read| !read.is(&recent.database, &recent.object()))
+        });
+        entries.insert(0, Entry::Read(recent));
+        while entries.len() > KEPT {
+            let Some(oldest) = entries[1..]
+                .iter()
+                .rposition(|entry| entry.read().is_some())
+            else {
+                break;
+            };
+            entries.remove(oldest + 1);
+        }
+    }
+
+    /// `conn`'s recents of `database`, newest first.
+    pub fn of(&self, conn: &str, database: &str) -> Vec<&Recent> {
+        let mut read: Vec<&Recent> = self
+            .lists
+            .get(conn)
+            .into_iter()
+            .flatten()
+            .filter_map(Entry::read)
+            .filter(|recent| recent.database == database)
+            .collect();
+        read.sort_by_key(|recent| std::cmp::Reverse(recent.opened));
+        read
+    }
+
+    /// Takes `conn`'s whole list out, and a value of it that was no list:
+    /// Clear, or a connection deleted. Whether there was anything.
+    pub fn clear(&mut self, conn: &str) -> bool {
+        let list = self.lists.remove(conn).is_some();
+        self.unread.remove(conn).is_some() || list
+    }
+
+    /// Drops the entries of `database`'s `schema` whose name `present`
+    /// denies: the schema was listed, and they are not in it. An entry of
+    /// a kind this version does not know is not judged by a list of
+    /// tables and views. Whether any went.
+    pub fn drop_missing(
+        &mut self,
+        conn: &str,
+        database: &str,
+        schema: &str,
+        present: impl Fn(&str) -> bool,
+    ) -> bool {
+        let Some(entries) = self.lists.get_mut(conn) else {
+            return false;
+        };
+        let before = entries.len();
+        entries.retain(|entry| {
+            entry.read().is_none_or(|recent| {
+                recent.object_kind().is_none()
+                    || recent.database != database
+                    || recent.schema != schema
+                    || present(&recent.name)
+            })
+        });
+        if entries.is_empty() {
+            self.lists.remove(conn);
+        }
+        entries_len(&self.lists, conn) != before
+    }
+}
+
+/// How long `conn`'s list is, none without one.
+fn entries_len(lists: &BTreeMap<String, Vec<Entry>>, conn: &str) -> usize {
+    lists.get(conn).map_or(0, Vec::len)
+}
+
+/// The first comment line of a script (`-- ebook covers per publisher`),
+/// without its dashes: what the start screen says of a closed SQL editor
+/// beside its number. None where the script has no line comment.
+pub fn first_comment(script: &str) -> Option<String> {
+    script
+        .lines()
+        .map(str::trim)
+        .filter_map(|line| line.strip_prefix("--"))
+        .map(str::trim)
+        .find(|comment| !comment.is_empty())
+        .map(str::to_owned)
+}
+
+/// When something was opened, as the start screen says it: `Just now`,
+/// `5 min ago`, `2 h ago`, `Yesterday`, the weekday (`Mon`) within the
+/// week, else the date (`Oct 14`, and with its year in another year).
+/// `then` and `now` are seconds since the Unix epoch; the days are
+/// `zone`'s. Nothing for a time no calendar has.
+pub fn when(then: u64, now: u64, zone: &TimeZone) -> String {
+    let ago = now.saturating_sub(then);
+    if ago < 60 {
+        return "Just now".into();
+    }
+    if ago < 3_600 {
+        return format!("{} min ago", ago / 60);
+    }
+    let date = |secs: u64| {
+        let at = Timestamp::from_second(i64::try_from(secs).ok()?).ok()?;
+        Some(at.to_zoned(zone.clone()).date())
+    };
+    let (Some(then_date), Some(now_date)) = (date(then), date(now)) else {
+        return String::new();
+    };
+    let days = now_date.since(then_date).map_or(0, |span| span.get_days());
+    match days {
+        i32::MIN..=0 => format!("{} h ago", ago / 3_600),
+        1 => "Yesterday".into(),
+        2..=6 => then_date.strftime("%a").to_string(),
+        _ if then_date.year() == now_date.year() => then_date.strftime("%b %-d").to_string(),
+        _ => then_date.strftime("%b %-d, %Y").to_string(),
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn table(name: &str, opened: u64) -> Recent {
+        Recent::of(
+            &ObjectRef::new("public", name),
+            ObjectKind::Table,
+            "bookshop_development",
+            opened,
+        )
+    }
+
+    fn names(recents: &Recents, conn: &str) -> Vec<String> {
+        recents
+            .of(conn, "bookshop_development")
+            .iter()
+            .map(|recent| recent.name.clone())
+            .collect()
+    }
+
+    #[test]
+    fn an_object_opened_again_moves_to_the_top_and_eight_are_kept() {
+        let mut recents = Recents::default();
+        for (index, name) in [
+            "authors",
+            "book_covers",
+            "covers",
+            "editions",
+            "genres",
+            "publishers",
+            "ratings",
+            "reviews",
+            "series",
+        ]
+        .into_iter()
+        .enumerate()
+        {
+            recents.note("dev", table(name, 100 + index as u64));
+        }
+        assert_eq!(
+            names(&recents, "dev"),
+            [
+                "series",
+                "reviews",
+                "ratings",
+                "publishers",
+                "genres",
+                "editions",
+                "covers",
+                "book_covers"
+            ]
+        );
+        recents.note("dev", table("covers", 200));
+        let listed = names(&recents, "dev");
+        assert_eq!(listed[0], "covers");
+        assert_eq!(listed.len(), KEPT);
+    }
+
+    #[test]
+    fn each_connection_and_each_database_has_its_own_list() {
+        let mut recents = Recents::default();
+        recents.note("dev", table("book_covers", 1));
+        let mut test = table("publishers", 2);
+        test.database = "bookshop_test".into();
+        recents.note("dev", test);
+        assert_eq!(names(&recents, "dev"), ["book_covers"]);
+        assert_eq!(names(&recents, "production"), Vec::<String>::new());
+        assert!(recents.clear("dev"));
+        assert!(!recents.clear("dev"));
+        assert_eq!(recents.of("dev", "bookshop_test").len(), 0);
+    }
+
+    #[test]
+    fn what_the_schema_no_longer_has_is_dropped() {
+        let mut recents = Recents::default();
+        recents.note("dev", table("book_covers", 1));
+        recents.note("dev", table("ghost", 2));
+        let mut other = table("publishers", 3);
+        other.schema = "shop".into();
+        recents.note("dev", other);
+        let present = |name: &str| name == "book_covers";
+        assert!(recents.drop_missing("dev", "bookshop_development", "public", present));
+        assert!(!recents.drop_missing("dev", "bookshop_development", "public", present));
+        let kept: Vec<String> = names(&recents, "dev");
+        // The other schema's entry was not judged.
+        assert_eq!(kept, ["publishers", "book_covers"]);
+    }
+
+    #[test]
+    fn the_file_keeps_what_this_version_cannot_read() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("recents.json");
+        let text = r#"{
+  "dev": [
+    { "kind": "table", "database": "bookshop_development", "schema": "public", "name": "book_covers", "opened": 5, "pinned": true },
+    { "kind": "saved query", "name": "ebook covers per publisher", "opened": 4 },
+    "not an entry"
+  ]
+}"#;
+        std::fs::write(&path, text).unwrap();
+        let mut recents = Recents::load(&path);
+        assert_eq!(names(&recents, "dev"), ["book_covers"]);
+        recents.note("dev", table("publishers", 6));
+        recents.save(&path).unwrap();
+        let written: serde_json::Value =
+            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
+        let dev = written["dev"].as_array().unwrap();
+        assert_eq!(dev.len(), 4);
+        assert_eq!(dev[0]["name"], "publishers");
+        assert_eq!(dev[1]["pinned"], true, "a newer key stays with its entry");
+        assert_eq!(dev[2]["kind"], "saved query");
+        assert_eq!(dev[3], "not an entry");
+        // Kept entries count, and are never the ones dropped.
+        for index in 0..KEPT {
+            recents.note("dev", table(&format!("t{index}"), 10 + index as u64));
+        }
+        recents.save(&path).unwrap();
+        let again = Recents::load(&path);
+        assert_eq!(again.of("dev", "bookshop_development").len(), KEPT - 2);
+        let written: serde_json::Value =
+            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
+        assert_eq!(written["dev"].as_array().unwrap().len(), KEPT);
+    }
+
+    #[test]
+    fn the_first_comment_line_names_a_script() {
+        assert_eq!(
+            first_comment("SELECT 1;\n  -- ebook covers per publisher \nSELECT 2"),
+            Some("ebook covers per publisher".into())
+        );
+        assert_eq!(first_comment("--\nSELECT 1"), None);
+        assert_eq!(
+            first_comment("--\n-- ebook covers\nSELECT 1"),
+            Some("ebook covers".into()),
+            "an empty comment line is passed over"
+        );
+        assert_eq!(first_comment("SELECT 1 -- not a line comment"), None);
+        assert_eq!(first_comment(""), None);
+    }
+
+    #[test]
+    fn an_entry_of_an_unknown_kind_is_kept_and_never_judged() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("recents.json");
+        let text = r#"{"dev": [
+  { "kind": "function", "database": "bookshop_development", "schema": "public", "name": "f", "opened": 7 },
+  { "kind": "table", "database": "bookshop_development", "schema": "public", "name": "t", "opened": 6 }
+]}"#;
+        std::fs::write(&path, text).unwrap();
+        let mut recents = Recents::load(&path);
+        let shown: Vec<String> = recents
+            .of("dev", "bookshop_development")
+            .iter()
+            .filter(|recent| recent.object_kind().is_some())
+            .map(|recent| recent.name.clone())
+            .collect();
+        assert_eq!(shown, ["t"]);
+        let present = |name: &str| name == "t";
+        assert!(
+            !recents.drop_missing("dev", "bookshop_development", "public", present),
+            "a list of tables and views says nothing of a function"
+        );
+        recents.save(&path).unwrap();
+        let written: serde_json::Value =
+            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
+        assert_eq!(written["dev"].as_array().unwrap().len(), 2);
+    }
+
+    #[test]
+    fn a_connection_written_in_another_shape_is_kept() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("recents.json");
+        let text = r#"{
+  "dev": [
+    { "kind": "table", "database": "bookshop_development", "schema": "public", "name": "book_covers", "opened": 5 }
+  ],
+  "newer": { "pinned": ["a"], "entries": [] }
+}"#;
+        std::fs::write(&path, text).unwrap();
+        let mut recents = Recents::load(&path);
+        assert_eq!(names(&recents, "dev"), ["book_covers"]);
+        recents.save(&path).unwrap();
+        let written: serde_json::Value =
+            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
+        assert_eq!(written["newer"]["pinned"][0], "a", "written back as it was");
+        // Something opened on that connection here takes its place.
+        recents.note("newer", table("authors", 9));
+        recents.save(&path).unwrap();
+        let written: serde_json::Value =
+            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
+        assert_eq!(written["newer"][0]["name"], "authors");
+        assert!(recents.clear("newer"));
+    }
+
+    #[test]
+    fn note_never_drops_what_was_just_noted() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("recents.json");
+        let entries: Vec<String> = (0..KEPT)
+            .map(|index| {
+                format!(r#"{{ "kind": "saved query", "name": "q{index}", "opened": {index} }}"#)
+            })
+            .collect();
+        std::fs::write(&path, format!(r#"{{ "dev": [{}] }}"#, entries.join(","))).unwrap();
+        let mut recents = Recents::load(&path);
+        recents.note("dev", table("authors", 100));
+        assert_eq!(names(&recents, "dev"), ["authors"]);
+        recents.save(&path).unwrap();
+        let written: serde_json::Value =
+            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
+        assert_eq!(written["dev"].as_array().unwrap().len(), KEPT + 1);
+    }
+
+    #[test]
+    fn a_missing_file_is_empty() {
+        let dir = tempfile::tempdir().unwrap();
+        let recents = Recents::load(&dir.path().join("recents.json"));
+        assert_eq!(recents, Recents::default());
+    }
+
+    #[test]
+    fn when_says_how_long_ago_in_the_zone() {
+        // 2026-10-19 10:00:00 UTC, a Monday.
+        let now = 1_792_404_000;
+        let berlin = TimeZone::fixed(jiff::tz::offset(2));
+        let say = |then: u64| when(then, now, &berlin);
+        assert_eq!(say(now - 30), "Just now");
+        assert_eq!(say(now - 5 * 60), "5 min ago");
+        assert_eq!(say(now - 2 * 3_600), "2 h ago");
+        // 01:30 Berlin the same day: today, 10 hours ago.
+        assert_eq!(say(now - 10 * 3_600 - 30 * 60), "10 h ago");
+        assert_eq!(say(now - 24 * 3_600), "Yesterday");
+        assert_eq!(say(now - 3 * 86_400), "Fri");
+        assert_eq!(say(now - 6 * 86_400), "Tue");
+        assert_eq!(say(now - 7 * 86_400), "Oct 12");
+        assert_eq!(say(now - 400 * 86_400), "Sep 14, 2025");
+        // The day is the zone's: ahead of UTC it is already the 20th, and
+        // eleven hours ago was the day before.
+        let east = TimeZone::fixed(jiff::tz::offset(14));
+        assert_eq!(when(now - 11 * 3_600 - 60, now, &east), "Yesterday");
+    }
+}
diff --git a/src/testing.rs b/src/testing.rs
index fe05fdd..fc77713 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -513,6 +513,28 @@ impl Harness {
     }
 }
 
+/// What the fake session lists in schema `main`: `users`, `orders` and
+/// the view `active_users`.
+pub(crate) fn fixture_objects() -> Vec<ObjectInfo> {
+    vec![
+        ObjectInfo {
+            name: "active_users".into(),
+            kind: ObjectKind::View,
+            estimated_rows: None,
+        },
+        ObjectInfo {
+            name: "orders".into(),
+            kind: ObjectKind::Table,
+            estimated_rows: Some(3),
+        },
+        ObjectInfo {
+            name: "users".into(),
+            kind: ObjectKind::Table,
+            estimated_rows: Some(1_200_000),
+        },
+    ]
+}
+
 /// The saved connection the fake session stands for.
 pub(crate) fn fixture_connection() -> SavedConnection {
     SavedConnection {
@@ -566,6 +588,13 @@ impl Harness {
     pub fn connect_fake_as(&mut self, read_only: bool) -> ConnTabId {
         let mut saved = fixture_connection();
         saved.read_only = Some(read_only);
+        self.connect_with(saved, fixture_objects())
+    }
+
+    /// Connects `saved` in the active tab through the recording backend
+    /// and answers the tree's first requests: schema `main` with
+    /// `objects`, as [`Self::connect_fake`] does with its own.
+    pub fn connect_with(&mut self, saved: SavedConnection, objects: Vec<ObjectInfo>) -> ConnTabId {
         let conn = saved.id.clone();
         self.app.connections.upsert(saved);
         let tab = self.app.active_tab_id();
@@ -594,23 +623,6 @@ impl Harness {
         let Command::ListObjects { request, .. } = *last_sent(&self.app) else {
             panic!("expected ListObjects for the default schema");
         };
-        let objects = vec![
-            ObjectInfo {
-                name: "active_users".into(),
-                kind: ObjectKind::View,
-                estimated_rows: None,
-            },
-            ObjectInfo {
-                name: "orders".into(),
-                kind: ObjectKind::Table,
-                estimated_rows: Some(3),
-            },
-            ObjectInfo {
-                name: "users".into(),
-                kind: ObjectKind::Table,
-                estimated_rows: Some(1_200_000),
-            },
-        ];
         self.app.apply(Action::Backend(Event::Objects {
             session,
             request,
diff --git a/src/ui/focus.rs b/src/ui/focus.rs
index c71b0b5..f607eb6 100644
--- a/src/ui/focus.rs
+++ b/src/ui/focus.rs
@@ -84,6 +84,8 @@ pub enum Region {
     Toolbar,
     /// A SQL editor's script.
     Editor,
+    /// The start screen, where the rows would be with no tab open.
+    Start,
     /// The rows.
     Grid,
     /// The row panel.
@@ -92,7 +94,7 @@ pub enum Region {
 
 impl Region {
     /// The panes the terminal look steps between with ctrl+h and ctrl+l.
-    pub const PANES: [Region; 3] = [Region::Tree, Region::Grid, Region::Panel];
+    pub const PANES: [Region; 4] = [Region::Tree, Region::Start, Region::Grid, Region::Panel];
 }
 
 /// Where the regions are, and which one F6 asked for.
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index e18a7ac..bff5508 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -218,6 +218,18 @@ pub(crate) fn asked(
     all
 }
 
+/// Whether `tab`'s server has another database to switch to: the menu of
+/// Switch database has something to show.
+fn switchable(app: &App, tab: ConnTabId) -> bool {
+    app.workspace(tab).is_some_and(|workspace| {
+        workspace
+            .databases
+            .value
+            .as_ref()
+            .is_some_and(|databases| databases.len() > 1)
+    })
+}
+
 /// The digit a command bound to a range (`alt+1..9`) was asked with, from
 /// nought. With Shift held the key reports what the digit types (`!`), so
 /// the key it is on counts too.
@@ -526,6 +538,13 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     let on_tree = app
         .workspace(active)
         .is_some_and(|workspace| workspace.pane == crate::model::Pane::Tree);
+    // The start screen has the keys where the rows would: no tab is open,
+    // and the keyboard is not in the tree.
+    let start = !on_picker
+        && !on_tree
+        && app
+            .workspace(active)
+            .is_some_and(crate::model::Workspace::on_start);
     let scopes: &[Scope] = if on_picker {
         &[Scope::Connections]
     } else if open {
@@ -534,6 +553,8 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         &[Scope::Inspector]
     } else if on_tree {
         &[Scope::Sidebar]
+    } else if start {
+        &[Scope::Start]
     } else if agents {
         &[Scope::Agents]
     } else if sql.is_some() && sql_grid {
@@ -604,6 +625,9 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     }
     // The place of the tab asked for with its digit.
     let mut tab_asked = None;
+    // Whether the menu of the server's other databases was asked for, by
+    // the chord that opens it under the bar's own chip.
+    let mut menu_asked = false;
     ctx.input_mut(|input| {
         // The keymap says which key: each line asks whether its command's
         // chord went down, in the scope it is read in. The guards say when
@@ -729,6 +753,27 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         if in_workspace {
             tab_asked = digit_asked(input, keymap, layout, switch, Scope::Global, normal);
         }
+        // The start screen's numbered rows, while it has the keys.
+        if start
+            && let Some(index) = digit_asked(
+                input,
+                keymap,
+                layout,
+                Command::OpenRecent,
+                Scope::Start,
+                normal,
+            )
+        {
+            actions.push(Action::OpenRecent { tab: active, index });
+        }
+        // Where the server has another database: with one, no menu is
+        // drawn, and an id left open would show it once the list came.
+        if in_workspace
+            && switchable(app, active)
+            && on(input, Command::SwitchDatabase, Scope::Global).count > 0
+        {
+            menu_asked = true;
+        }
         if !on_sql && on(input, Command::Reload, Scope::Grid).count > 0 {
             // The tree reloads itself when it has the arrows (spec 5.10).
             actions.push(if tree_arrows {
@@ -870,6 +915,12 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
             }
         }
     });
+    // The menu opens as a click on the chip opens it: a server with one
+    // database has none, and the key does nothing.
+    if menu_asked {
+        let id = crate::ui::workspace::database_menu_id(active, false);
+        egui::Popup::open_id(ctx, id);
+    }
     // The tab at the place its digit named.
     if let Some(index) = tab_asked
         && let Some(workspace) = app.workspace(active)
@@ -887,6 +938,7 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         use crate::ui::focus::{self, Region};
         let from = match workspace.pane {
             crate::model::Pane::Tree => Region::Tree,
+            crate::model::Pane::Grid if workspace.on_start() => Region::Start,
             crate::model::Pane::Grid => Region::Grid,
         };
         // Which of `commands` was asked for first, by its place.
@@ -1990,6 +2042,8 @@ fn letters(
         return;
     };
     let tree = workspace.pane == crate::model::Pane::Tree;
+    // The start screen's letters are read while it has the keys.
+    let start = !tree && workspace.on_start();
     let panel = workspace.row_panel;
     let shown = workspace.row_panel_tab();
     let object = workspace.active_object_tab();
@@ -2405,6 +2459,20 @@ fn letters(
                 let name = app.agents_name(tab).unwrap_or_default();
                 crate::ui::agents::stop_here(ctx, tab, here, &name, app.locale, actions);
             }
+            // The start screen's letters: its actions, by their keys.
+            (Some(Command::NewSqlTab), _) if start => actions.push(Action::NewSqlTab(tab)),
+            (Some(Command::EditConnection), _) if start => {
+                if let Some(conn) = app
+                    .workspace(tab)
+                    .map(|workspace| workspace.conn_id.clone())
+                {
+                    actions.push(Action::EditConnection(conn));
+                }
+            }
+            (Some(Command::SwitchDatabase), _) if start && switchable(app, tab) => {
+                let id = crate::ui::workspace::database_menu_id(tab, true);
+                egui::Popup::open_id(ctx, id);
+            }
             (Some(Command::EditCell), _) if edits && alone => {
                 opened = true;
                 actions.extend(edit(crate::model::EditStart::Value));
diff --git a/src/ui/menu.rs b/src/ui/menu.rs
index a0ee8bd..be557eb 100644
--- a/src/ui/menu.rs
+++ b/src/ui/menu.rs
@@ -345,10 +345,25 @@ pub fn under<R>(
     look: &Look,
     palette: &Palette,
     add: impl FnOnce(&mut Ui) -> R,
+) -> Option<R> {
+    let id = egui::Popup::default_response_id(button);
+    under_id(button, id, min_width, look, palette, add)
+}
+
+/// [`under`] with the pop-up's id given, so a key can open the menu with
+/// [`egui::Popup::open_id`] as a click on `button` does.
+pub fn under_id<R>(
+    button: &Response,
+    id: egui::Id,
+    min_width: f32,
+    look: &Look,
+    palette: &Palette,
+    add: impl FnOnce(&mut Ui) -> R,
 ) -> Option<R> {
     let frame = frame(look, palette);
     let inside = min_width - frame.total_margin().sum().x;
     egui::Popup::menu(button)
+        .id(id)
         .frame(frame)
         .gap(GAP)
         .show(|ui| rows(ui, inside.max(0.0), add))
diff --git a/src/ui/sidebar.rs b/src/ui/sidebar.rs
index db2ce53..ad30b87 100644
--- a/src/ui/sidebar.rs
+++ b/src/ui/sidebar.rs
@@ -1,4 +1,4 @@
-//! The sidebar: filter, recent objects, the schema picker, the shown
+//! The sidebar: filter, the schema picker, the shown
 //! schema's objects in prefix groups or one flat list, and the button that
 //! opens a SQL editor.
 
@@ -33,9 +33,6 @@ fn field_height(look: &Look) -> f32 {
     if look.terminal { 32.0 } else { 36.0 }
 }
 
-/// Recent rows (macOS only).
-const RECENT_ROW: f32 = 28.0;
-
 /// The Reload button's spinner, and a loading row's.
 const RELOAD_SPINNER: f32 = 14.0;
 const ROW_SPINNER: f32 = 12.0;
@@ -126,11 +123,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let active = workspace
         .active_object_tab()
         .map(|object| object.object.clone());
-    let recent: Vec<(ObjectRef, ObjectKind)> = if look.terminal {
-        Vec::new()
-    } else {
-        workspace.recent.clone()
-    };
+
     // The keyboard cursor, drawn only while the tree has the arrows.
     let cursor = workspace
         .tree
@@ -213,17 +206,6 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 }
             });
             ui.add_space(if look.terminal { 6.0 } else { 8.0 });
-            if !recent.is_empty() {
-                recent_section(
-                    ui,
-                    &recent,
-                    active.as_ref(),
-                    tab,
-                    &look,
-                    &palette,
-                    &mut actions,
-                );
-            }
             schema_header(
                 ui,
                 tab,
@@ -463,7 +445,7 @@ fn footer_height(look: &Look, agents: bool) -> f32 {
 
 /// What the Agents entry says of the connection, in which colour, and
 /// whether a dot leads it: an agent is here.
-fn agents_said(
+pub(super) fn agents_said(
     here: &crate::model::AgentsHere,
     look: &Look,
     palette: &Palette,
@@ -645,58 +627,6 @@ fn sql_button(
     }
 }
 
-/// The Recent section: the objects opened lately, newest first.
-fn recent_section(
-    ui: &mut egui::Ui,
-    recent: &[(ObjectRef, ObjectKind)],
-    active: Option<&ObjectRef>,
-    tab: ConnTabId,
-    look: &Look,
-    palette: &Palette,
-    actions: &mut Vec<Action>,
-) {
-    let width = ui.available_width();
-    // The label: 4 above and below, 16 in.
-    let label_text = widgets::section_label("Recent", look, palette).layout(ui.ctx());
-    let label_height = label_text.height() + 8.0;
-    let (label, _) = ui.allocate_exact_size(vec2(width, label_height), Sense::hover());
-    label_text.paint_left(ui.painter(), label.left() + 16.0, label.center().y);
-    for (object, kind) in recent {
-        let (row, response) = ui.allocate_exact_size(vec2(width, RECENT_ROW), Sense::click());
-        let name = display_safe(&object.name);
-        response.widget_info(|| {
-            WidgetInfo::labeled(WidgetType::Button, true, format!("Recent {name}"))
-        });
-        let selected = Some(object) == active;
-        widgets::selection(ui, row, selected, response.hovered(), look, palette);
-        row_ring(ui, &response, widgets::selection_rect(row, look), look);
-        let (role, color) = if selected {
-            (TextRole::UiBodyStrong, palette.accent_hover)
-        } else {
-            (TextRole::UiBody, palette.text)
-        };
-        let room = row.width() - 32.0;
-        let shown = crate::ui::grid::ellipsize(&name, room, false, |text| {
-            role.width(ui.ctx(), look.faces, text)
-        });
-        widgets::paint_text(
-            ui,
-            row.left() + 16.0,
-            row.center().y,
-            Text::one(look, role, &shown, color),
-        );
-        if response.clicked() {
-            actions.push(Action::OpenObject {
-                tab,
-                object: object.clone(),
-                kind: *kind,
-                pin: true,
-            });
-        }
-    }
-    ui.add_space(8.0);
-}
-
 /// What the schema header draws with.
 #[derive(Clone, Copy)]
 struct SchemaHeader<'a> {
@@ -872,13 +802,6 @@ struct Marks<'a> {
     lit: bool,
 }
 
-/// A row with the keyboard is ringed inside its highlight: a ring outside
-/// it would be cut by the list it scrolls in.
-fn row_ring(ui: &egui::Ui, response: &egui::Response, highlight: Rect, look: &Look) {
-    let radius = look.radius.saturating_sub(2);
-    focus::hint(ui, response, highlight, focus::Ring::Inset { radius });
-}
-
 /// How rows are drawn: the platform look and the colour palette.
 #[derive(Clone, Copy)]
 struct Skin<'a> {
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index c501735..1863bfd 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -448,12 +448,12 @@ fn failure(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
 
 /// What the connection bar says: the open connections, and more about the
 /// one the bar belongs to.
-struct BarInfo {
-    env: crate::env::Environment,
+pub(super) struct BarInfo {
+    pub(super) env: crate::env::Environment,
     /// Whether the bar's own connection opens read-only.
-    read_only: bool,
-    database: String,
-    databases: Vec<String>,
+    pub(super) read_only: bool,
+    pub(super) database: String,
+    pub(super) databases: Vec<String>,
     tls: Option<(&'static str, Tone)>,
     ssh_host: Option<String>,
     chips: Vec<Chip>,
@@ -508,7 +508,7 @@ fn is_remote(workspace: &crate::model::Workspace) -> bool {
     }
 }
 
-fn bar_info(app: &App, tab: ConnTabId) -> Option<BarInfo> {
+pub(super) fn bar_info(app: &App, tab: ConnTabId) -> Option<BarInfo> {
     use tabletist_db::Driver;
     let workspace = app.workspace(tab)?;
     let spec = &workspace.spec;
@@ -842,16 +842,25 @@ fn announce_switcher(
     });
 }
 
-/// The pop-up of the server's other databases, under the bar's own chip.
-fn database_menu(
+/// The id of the pop-up of the server's other databases: the bar's own
+/// chip's, or the start screen's button's. A key opens one with
+/// [`egui::Popup::open_id`].
+pub fn database_menu_id(tab: ConnTabId, start: bool) -> egui::Id {
+    egui::Id::new(("database-menu", tab.0, start))
+}
+
+/// The pop-up of the server's other databases, under the bar's own chip
+/// or the start screen's button (`response`), with `id` its own.
+pub(super) fn database_menu(
     response: &egui::Response,
+    id: egui::Id,
     tab: ConnTabId,
     info: &BarInfo,
     (look, palette): (&Look, &Palette),
     actions: &mut Vec<Action>,
 ) {
     let role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary);
-    menu::under(response, response.rect.width(), look, palette, |ui| {
+    menu::under_id(response, id, response.rect.width(), look, palette, |ui| {
         for database in &info.databases {
             // Database names come from the server: nothing hidden.
             let name = display_safe(database);
@@ -1175,7 +1184,8 @@ fn mac_bar(
                     ui,
                     Rect::from_center_size(pos2(rect.right() - 16.0, center), vec2(12.0, 12.0)),
                 );
-                database_menu(&response, tab, info, (look, palette), actions);
+                let id = database_menu_id(tab, false);
+                database_menu(&response, id, tab, info, (look, palette), actions);
             }
         } else if response.clicked() {
             actions.push(Action::ActivateConnTab(chip.tab));
@@ -1443,7 +1453,8 @@ fn terminal_bar(
                     center,
                     Text::one(look, TextRole::OBody, "▾", palette.dim),
                 );
-                database_menu(&response, tab, info, (look, palette), actions);
+                let id = database_menu_id(tab, false);
+                database_menu(&response, id, tab, info, (look, palette), actions);
             }
         } else if response.clicked() {
             actions.push(Action::ActivateConnTab(chip.tab));
````

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`

Expected: all pass, among them `recents::tests::*`, `app::tests::connecting_asks_for_the_version_once_per_session`, `app::tests::sql_tabs_are_numbered_per_connection_and_share_the_version`, `app::tests::switching_database_keeps_sql_editors_and_drops_object_tabs`, `ui::tests::opening_an_object_puts_it_first_in_the_recents`, and the keymap's generated tests with the new scope.

```
fmt exit 0
clippy exit 0
test result: ok. 2870 passed; 0 failed; 20 ignored; 0 measured; 0 filtered out; finished in 9.11s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.12s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 502 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.03s
test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.81s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
test result: ok. 10 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 100 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.34s
test result: ok. 84 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.23s
test result: ok. 102 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.73s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.05s
test result: ok. 54 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.47s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.32s
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.07s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test exit 0
doc exit 0
```

- [ ] **Step 5: Commit**

```bash
git add src/recents.rs src/lib.rs src/paths.rs src/backend.rs src/model.rs src/app.rs src/keymap.rs src/ui/keys.rs src/ui/focus.rs src/ui/menu.rs src/ui/workspace.rs src/ui/sidebar.rs src/ui/mod.rs src/testing.rs docs/_reference/settings-and-files.md
git commit -S -m "Keep what was opened lately on each connection in recents.json, with the start screen's keys"
```

### Task 2: The start screen in both looks

`src/ui/start.rs` draws the screen where the empty state was, with its macOS status bar and the terminal status line's `-- START --`; the macOS strip stays with no tab; the production strip and the environment's colours; the harness's Bookshop objects; the brief's tests; the website.

**Files:** Create `src/ui/start.rs`, `src/ui/start_tests.rs`; Modify `src/ui/mod.rs`, `src/ui/workspace.rs`, `src/ui/object_tabs.rs`, `src/env.rs`, `src/testing.rs`, `tests/keys.rs`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/testing.rs b/src/testing.rs
index fc77713..9ce2c69 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -535,6 +535,33 @@ pub(crate) fn fixture_objects() -> Vec<ObjectInfo> {
     ]
 }
 
+/// The Bookshop's objects as a catalog lists them, with the planner's
+/// estimate of each table's rows: what a start screen's first visit sorts
+/// by. In schema `main`, as the fake session's are.
+pub(crate) fn bookshop_objects() -> Vec<ObjectInfo> {
+    let table = |name: &str, rows: u64| ObjectInfo {
+        name: name.into(),
+        kind: ObjectKind::Table,
+        estimated_rows: Some(rows),
+    };
+    let view = |name: &str| ObjectInfo {
+        name: name.into(),
+        kind: ObjectKind::View,
+        estimated_rows: None,
+    };
+    vec![
+        table("authors", 3_700),
+        table("book_covers", 13),
+        view("cover_counts"),
+        table("covers", 13),
+        table("editions", 9_400),
+        view("missing_covers"),
+        table("publishers", 6),
+        table("ratings", 31_900),
+        table("reviews", 48_200),
+    ]
+}
+
 /// The saved connection the fake session stands for.
 pub(crate) fn fixture_connection() -> SavedConnection {
     SavedConnection {
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 573a969..680e1e7 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -63,6 +63,8 @@ pub mod sql_complete;
 pub mod sql_editor;
 pub mod sql_results;
 pub mod sql_text;
+#[cfg(test)]
+mod start_tests;
 pub mod states;
 pub mod structure;
 pub mod terminal_dialog;
@@ -4438,7 +4440,7 @@ value_tags   = true
         harness.click("users");
         harness.click("Close users");
         assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
-        assert!(harness.has("Select a table or view in the sidebar"));
+        assert!(harness.has("No table open"), "the start screen is back");
     }
 
     #[test]
diff --git a/src/ui/start_tests.rs b/src/ui/start_tests.rs
new file mode 100644
index 0000000..a990e1c
--- /dev/null
+++ b/src/ui/start_tests.rs
@@ -0,0 +1,710 @@
+//! The start screen: what a connection's window shows with no tab open,
+//! in both looks, and the recents it lists.
+
+use egui::accesskit::Role;
+use egui::{Key, Modifiers};
+use tabletist_db::{Driver, ObjectInfo, ObjectKind, ObjectRef};
+
+use crate::backend::{Command, Event};
+use crate::connections::SavedConnection;
+use crate::env::Environment;
+use crate::model::{Action, ConnTabId, Dialog, Pane};
+use crate::recents::Recent;
+use crate::testing::{Harness, bookshop_objects, bounds, fixture_connection, labels};
+use crate::theme::Look;
+
+/// 2026-10-19 10:00:00 UTC: when the tests happen.
+const NOW: u64 = 1_792_404_000;
+
+/// A Bookshop development connection, listed, in `look`, with the clock
+/// pinned so that every row was opened just now.
+fn bookshop(look: Look) -> (Harness, ConnTabId) {
+    crate::util::pin_now(Some(NOW));
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_with(fixture_connection(), bookshop_objects());
+    (harness, tab)
+}
+
+/// Opens `name` of the Bookshop in `tab` and closes its tab again: the
+/// start screen is back, with it among the recents.
+fn visit(harness: &mut Harness, tab: ConnTabId, name: &str) {
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: ObjectRef::new("main", name),
+        kind: ObjectKind::Table,
+        pin: true,
+    });
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::CloseTab { tab, id });
+}
+
+/// The labels of the screen's rows, top to bottom.
+fn rows(harness: &mut Harness) -> Vec<String> {
+    let tree = harness.settle();
+    let mut rows: Vec<(f32, String)> = labels(&tree)
+        .into_iter()
+        .filter(|label| {
+            label.contains(", opened ") || label.contains(", table, ") || label.contains(", view, ")
+        })
+        .filter_map(|label| Some((bounds(&tree, &label, Role::Button)?.top(), label)))
+        .collect();
+    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
+    rows.into_iter().map(|(_, label)| label).collect()
+}
+
+/// The label that has `part` in it; the labels there are, where none
+/// does.
+fn label_with(harness: &mut Harness, part: &str) -> String {
+    let tree = harness.settle();
+    let labels = labels(&tree);
+    labels
+        .iter()
+        .find(|label| label.contains(part))
+        .cloned()
+        .unwrap_or_else(|| panic!("no label with {part:?} among {labels:?}"))
+}
+
+/// Whether the last frame painted `text`.
+fn painted(harness: &Harness, text: &str) -> bool {
+    harness.painted.iter().any(|(painted, _)| painted == text)
+}
+
+/// Types one key as the window sends it: the key, and the character it
+/// types.
+fn type_key(harness: &mut Harness, key: Key, text: &str) {
+    harness.settle();
+    harness.frame(vec![
+        crate::testing::key(key, Modifiers::NONE),
+        egui::Event::Text(text.into()),
+    ]);
+    harness.frame(vec![crate::testing::release(key, Modifiers::NONE)]);
+    harness.settle();
+}
+
+/// How many times the app asked for `path` to be written.
+fn saves_of(harness: &Harness, path: &std::path::Path) -> usize {
+    harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .filter(|command| matches!(command, Command::Save { path: to, .. } if to == path))
+        .count()
+}
+
+/// Where the production Bookshop is.
+fn production_spec() -> tabletist_db::ConnectSpec {
+    tabletist_db::ConnectSpec::sqlite("/tmp/bookshop_production.db")
+}
+
+/// A saved Bookshop production connection, on its own file.
+fn production() -> SavedConnection {
+    let mut saved = fixture_connection();
+    saved.name = "Bookshop · production".into();
+    saved.environment = Environment::Production;
+    saved.spec = production_spec();
+    saved
+}
+
+/// Opens `saved` in a window of its own, in front, listed with the
+/// Bookshop's objects.
+fn second_window(harness: &mut Harness, saved: SavedConnection) -> ConnTabId {
+    harness.app.apply(Action::ShowConnections);
+    harness.connect_with(saved, bookshop_objects())
+}
+
+/// Test 1: the screen shows with no tab, goes when one opens, and comes
+/// back when the last tab closes.
+#[test]
+fn the_start_screen_shows_with_no_tab_and_comes_back_when_the_last_closes() {
+    for (look, said) in [
+        (Look::macos(), "No table open"),
+        (Look::omarchy(), "no table open"),
+    ] {
+        let (mut harness, tab) = bookshop(look);
+        assert!(harness.has(said), "{}", look.name);
+        assert!(!harness.has("Select a table or view in the sidebar"));
+        harness.click("book_covers");
+        assert!(!harness.has(said), "{}: a tab is open", look.name);
+        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+        harness.app.apply(Action::CloseTab { tab, id });
+        assert!(harness.has(said), "{}: the last tab closed", look.name);
+    }
+}
+
+/// The connection line: the database, the server as it names itself once
+/// it has, and how many tables and views the schema lists.
+#[test]
+fn the_connection_line_names_the_server_and_counts_the_objects() {
+    let (mut harness, tab) = bookshop(Look::macos());
+    let line = label_with(&mut harness, " · SQLite · ");
+    assert!(line.ends_with("SQLite · 9 tables and views"), "{line}");
+    // The version was asked for with the connect, and the line takes it.
+    let asked = harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|command| match command {
+            Command::ServerVersion { session, request } => Some((*session, *request)),
+            _ => None,
+        })
+        .expect("the version was asked for at connect");
+    harness.app.apply(Action::Backend(Event::ServerVersion {
+        session: asked.0,
+        request: asked.1,
+        result: Ok("SQLite 3.46.1".into()),
+    }));
+    let line = label_with(&mut harness, "SQLite 3.46.1");
+    assert!(
+        line.ends_with("SQLite 3.46.1 · 9 tables and views"),
+        "{line}"
+    );
+    assert_eq!(harness.app.workspace(tab).unwrap().tabs.len(), 0);
+    // The terminal look counts objects.
+    harness.set_look(Look::omarchy());
+    let line = label_with(&mut harness, "sqlite 3.46.1");
+    assert!(line.ends_with("sqlite 3.46.1 · 9 objects"), "{line}");
+}
+
+/// Test 7: a production connection's line is the production strip, with
+/// Read-only where it is, whatever Environment color says.
+#[test]
+fn production_shows_the_strip_whatever_the_environment_color_says() {
+    for look in [Look::macos(), Look::omarchy()] {
+        crate::util::pin_now(Some(NOW));
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        harness.app.settings.env_bar = crate::settings::EnvBar::Label;
+        harness.connect_with(production(), bookshop_objects());
+        let line = label_with(&mut harness, &look.label("SQLite · 9"));
+        let expected = look.label("Read-only · SQLite · 9 tables and views");
+        let expected = if look.terminal {
+            "read-only · sqlite · 9 objects".to_owned()
+        } else {
+            expected
+        };
+        assert!(line.ends_with(&expected), "{}: {line}", look.name);
+        let platform = crate::env::Platform::of(&look);
+        let colors =
+            crate::env::env_colors(Environment::Production, platform, &harness.app.palette);
+        assert!(
+            harness
+                .fills
+                .iter()
+                .any(|(_, color)| *color == colors.bar_bg()),
+            "{}: the strip's tint",
+            look.name
+        );
+        let tag = Environment::Production.label(platform);
+        assert!(painted(&harness, tag), "{}: the tag", look.name);
+    }
+}
+
+/// Tests 2 and 3: recents are the connection's, newest first, five of
+/// the eight kept, and Clear empties only this connection's.
+#[test]
+fn recents_are_per_connection_newest_first_five_of_eight_and_cleared_apart() {
+    let (mut harness, dev) = bookshop(Look::macos());
+    let dev_id = harness.app.workspace(dev).unwrap().conn_id.clone();
+    let names = [
+        "authors",
+        "book_covers",
+        "covers",
+        "editions",
+        "publishers",
+        "ratings",
+        "reviews",
+        "cover_counts",
+        "missing_covers",
+    ];
+    for (index, name) in names.iter().enumerate() {
+        crate::util::pin_now(Some(NOW + index as u64));
+        visit(&mut harness, dev, name);
+    }
+    crate::util::pin_now(Some(NOW + 20));
+    let shown = rows(&mut harness);
+    assert_eq!(shown.len(), 5, "{shown:?}");
+    assert_eq!(
+        shown[0], "missing_covers, table, main, opened Just now",
+        "{shown:?}"
+    );
+    assert_eq!(shown[1], "cover_counts, table, main, opened Just now");
+    assert_eq!(shown[4], "publishers, table, main, opened Just now");
+    let database = crate::session::database_of(&harness.app.workspace(dev).unwrap().spec);
+    assert_eq!(harness.app.recents.of(&dev_id.0, &database).len(), 8);
+    let file = harness.app.dirs.recents_file();
+    assert_eq!(saves_of(&harness, &file), 9, "written at every open");
+    assert!(harness.has("Recent on this connection"));
+    // Opening one again puts it first.
+    crate::util::pin_now(Some(NOW + 30));
+    visit(&mut harness, dev, "authors");
+    assert_eq!(
+        rows(&mut harness)[0],
+        "authors, table, main, opened Just now"
+    );
+    // Production has none of them: its first visit lists the largest tables.
+    let production = second_window(&mut harness, production());
+    let prod_id = harness.app.workspace(production).unwrap().conn_id.clone();
+    assert!(harness.has("Largest tables") || harness.has("Tables"));
+    assert!(
+        !rows(&mut harness)
+            .iter()
+            .any(|row| row.contains(", opened ")),
+        "nothing was opened on production"
+    );
+    assert!(harness.app.recents.of(&prod_id.0, &database).is_empty());
+    assert!(
+        harness
+            .app
+            .recents
+            .of(&prod_id.0, &crate::session::database_of(&production_spec()))
+            .is_empty()
+    );
+    visit(&mut harness, production, "publishers");
+    assert_eq!(rows(&mut harness).len(), 1);
+    let prod_database =
+        crate::session::database_of(&harness.app.workspace(production).unwrap().spec);
+    // Clear on development empties development alone.
+    harness.app.apply(Action::ActivateConnTab(dev));
+    harness.click("Clear");
+    assert!(harness.app.recents.of(&dev_id.0, &database).is_empty());
+    assert_eq!(harness.app.recents.of(&prod_id.0, &prod_database).len(), 1);
+    assert!(!harness.has("Recent on this connection"));
+    assert_eq!(saves_of(&harness, &file), 12);
+}
+
+/// Test 3: a recent the schema lists no more is dropped, without a word,
+/// and the file written without it.
+#[test]
+fn a_recent_that_no_longer_exists_is_dropped_when_the_schema_is_listed() {
+    crate::util::pin_now(Some(NOW));
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    let saved = fixture_connection();
+    let database = crate::session::database_of(&saved.spec);
+    for (name, opened) in [("ghost", NOW - 100), ("book_covers", NOW - 200)] {
+        let recent = Recent::of(
+            &ObjectRef::new("main", name),
+            ObjectKind::Table,
+            &database,
+            opened,
+        );
+        harness.app.recents.note(&saved.id.0, recent);
+    }
+    let conn = saved.id.clone();
+    let file = harness.app.dirs.recents_file();
+    let before = saves_of(&harness, &file);
+    let tab = harness.connect_with(saved, bookshop_objects());
+    let shown = rows(&mut harness);
+    assert_eq!(shown, ["book_covers, table, main, opened 3 min ago"]);
+    assert_eq!(harness.app.recents.of(&conn.0, &database).len(), 1);
+    assert_eq!(
+        saves_of(&harness, &file),
+        before + 1,
+        "written once it was dropped"
+    );
+    assert_eq!(harness.app.start_recents(tab).len(), 1);
+}
+
+/// Test 4: a first visit lists the largest tables from the catalog's
+/// estimates, and nothing is counted or fetched for it. A database with
+/// no estimates lists its tables by name.
+#[test]
+fn the_first_visit_lists_the_largest_tables_from_the_catalog() {
+    let (mut harness, tab) = bookshop(Look::macos());
+    harness.app.workspace_mut(tab).unwrap().driver = Driver::Postgres;
+    let sent = harness.app.backend.sent.len();
+    let shown = rows(&mut harness);
+    assert_eq!(
+        shown,
+        [
+            "reviews, table, main, ≈ 48,200 rows",
+            "ratings, table, main, ≈ 31,900 rows",
+            "editions, table, main, ≈ 9,400 rows",
+            "authors, table, main, ≈ 3,700 rows",
+            "book_covers, table, main, ≈ 13 rows",
+        ]
+    );
+    assert!(harness.has("Largest tables"));
+    assert!(harness.has("From table statistics. Nothing is queried."));
+    assert!(
+        !harness.app.backend.sent[sent..]
+            .iter()
+            .any(|command| matches!(
+                command,
+                Command::CountRows { .. } | Command::FetchRows { .. } | Command::RunSql { .. }
+            )),
+        "nothing is queried for the list"
+    );
+    // The terminal look writes the estimate short.
+    harness.set_look(Look::omarchy());
+    assert_eq!(rows(&mut harness)[0], "reviews, table, main, ≈ 48.2k");
+    // SQLite has no estimates: its tables by name, under a plainer heading.
+    crate::util::pin_now(Some(NOW));
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    let by_name: Vec<ObjectInfo> = bookshop_objects()
+        .into_iter()
+        .map(|info| ObjectInfo {
+            estimated_rows: None,
+            ..info
+        })
+        .collect();
+    harness.connect_with(fixture_connection(), by_name);
+    let shown = rows(&mut harness);
+    assert_eq!(
+        shown,
+        [
+            "authors, table, main",
+            "book_covers, table, main",
+            "covers, table, main",
+            "editions, table, main",
+            "publishers, table, main",
+        ]
+    );
+    assert!(harness.has("Tables") && !harness.has("Largest tables"));
+    assert!(harness.has("Nothing is queried."));
+    // A row opens its table.
+    harness.click("authors, table, main");
+    assert!(harness.has("authors tab"));
+}
+
+/// A SQL editor closed with a script is a recent, named by its first
+/// comment line, and opens again with the script.
+#[test]
+fn a_closed_sql_editor_with_a_script_is_a_recent_that_opens_again() {
+    let (mut harness, tab) = bookshop(Look::macos());
+    harness.app.apply(Action::NewSqlTab(tab));
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::CloseTab { tab, id });
+    assert!(
+        !rows(&mut harness)
+            .iter()
+            .any(|row| row.contains(", opened ")),
+        "an empty editor is nothing to offer"
+    );
+    harness.app.apply(Action::NewSqlTab(tab));
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    let script = "-- ebook covers per publisher\nSELECT 1";
+    harness
+        .app
+        .workspace_mut(tab)
+        .unwrap()
+        .sql_tab_mut(id)
+        .unwrap()
+        .text = script.into();
+    harness.app.apply(Action::CloseTab { tab, id });
+    let shown = rows(&mut harness);
+    assert_eq!(
+        shown,
+        ["Query 2, SQL editor, ebook covers per publisher, opened Just now"]
+    );
+    harness.click(&shown[0]);
+    let workspace = harness.app.workspace(tab).unwrap();
+    let sql = workspace
+        .active_sql_tab()
+        .expect("the editor is open again");
+    assert_eq!(sql.text, script);
+    assert!(workspace.recent_sql.is_empty());
+    assert!(!harness.has("No table open"));
+}
+
+/// Test 11: the macOS sidebar lists no recents any more.
+#[test]
+fn the_mac_sidebar_has_no_recent_list() {
+    let (mut harness, _tab) = bookshop(Look::macos());
+    harness.click("book_covers");
+    harness.click("publishers");
+    assert!(!harness.has("Recent book_covers"));
+    assert!(
+        !harness
+            .painted
+            .iter()
+            .any(|(text, _)| text.eq_ignore_ascii_case("recent")),
+        "no Recent section"
+    );
+}
+
+/// The terminal look's status line with no tab: the mode, the keys, and
+/// that no table is open.
+#[test]
+fn the_terminal_status_line_says_start() {
+    let (mut harness, tab) = bookshop(Look::omarchy());
+    harness.settle();
+    assert!(painted(&harness, "-- START --"));
+    assert!(harness.has("no table open"));
+    harness.click("book_covers");
+    harness.settle();
+    assert!(!painted(&harness, "-- START --"));
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::CloseTab { tab, id });
+    harness.settle();
+    assert!(painted(&harness, "-- START --"));
+}
+
+/// Test 10: the screen's keys work only while it has the keyboard; in the
+/// sidebar `t` is the tree's, and other letters do nothing.
+#[test]
+fn the_terminal_start_keys_work_only_with_the_start_screen_focused() {
+    let (mut harness, tab) = bookshop(Look::omarchy());
+    let pane = |harness: &Harness| harness.app.workspace(tab).unwrap().pane;
+    // The tree has the keys after a connect, as ever; ctrl+l gives them to
+    // the screen.
+    assert_eq!(pane(&harness), Pane::Tree);
+    harness.press(Key::L, Modifiers::CTRL);
+    assert_eq!(pane(&harness), Pane::Grid);
+    // A number opens the row it names: the largest table on a first visit.
+    type_key(&mut harness, Key::Num1, "1");
+    assert!(harness.has("reviews tab"));
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::CloseTab { tab, id });
+    assert_eq!(
+        rows(&mut harness)[0],
+        "reviews, table, main, opened just now"
+    );
+    // Then the recents, by their numbers.
+    type_key(&mut harness, Key::Num1, "1");
+    assert!(harness.has("reviews tab"));
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::CloseTab { tab, id });
+    type_key(&mut harness, Key::Num2, "2");
+    assert!(!harness.has("reviews tab"), "there is no second row");
+    // The letters: a new SQL editor, the connection's form, the Agents tab.
+    type_key(&mut harness, Key::T, "t");
+    assert!(harness.has("Query 1 tab"));
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::CloseTab { tab, id });
+    type_key(&mut harness, Key::E, "e");
+    assert!(matches!(harness.app.dialog, Some(Dialog::Connection(_))));
+    harness.app.apply(Action::CloseDialog);
+    if crate::endpoint::AVAILABLE {
+        type_key(&mut harness, Key::A, "a");
+        assert!(harness.app.workspace(tab).unwrap().agents_in_front());
+        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+        harness.app.apply(Action::CloseTab { tab, id });
+    }
+    // The database menu, where there is another database.
+    {
+        let workspace = harness.app.workspace_mut(tab).unwrap();
+        workspace.driver = Driver::Postgres;
+        workspace.spec.database = "bookshop_development".into();
+        workspace.databases.value =
+            Some(vec!["bookshop_development".into(), "bookshop_test".into()]);
+    }
+    type_key(&mut harness, Key::D, "d");
+    let menu = crate::ui::workspace::database_menu_id(tab, true);
+    assert!(egui::Popup::is_id_open(&harness.ctx, menu));
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert!(!egui::Popup::is_id_open(&harness.ctx, menu));
+    // Its global chord opens the bar's own chip's menu, anywhere.
+    harness.press(Key::D, Modifiers::CTRL | Modifiers::SHIFT);
+    let chip = crate::ui::workspace::database_menu_id(tab, false);
+    assert!(egui::Popup::is_id_open(&harness.ctx, chip));
+    harness.press(Key::Escape, Modifiers::NONE);
+    // With the sidebar focused the keys are the tree's: `t` switches the
+    // list, the rest does nothing.
+    harness.press(Key::H, Modifiers::CTRL);
+    assert_eq!(pane(&harness), Pane::Tree);
+    let flat = harness.app.workspace(tab).unwrap().tree.flat;
+    type_key(&mut harness, Key::T, "t");
+    assert_ne!(harness.app.workspace(tab).unwrap().tree.flat, flat);
+    type_key(&mut harness, Key::Num1, "1");
+    type_key(&mut harness, Key::E, "e");
+    type_key(&mut harness, Key::X, "x");
+    assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
+    assert!(harness.app.dialog.is_none());
+    // And back to the screen.
+    harness.press(Key::L, Modifiers::CTRL);
+    assert_eq!(pane(&harness), Pane::Grid);
+    type_key(&mut harness, Key::Num1, "1");
+    assert!(harness.has("reviews tab"));
+}
+
+/// The other looks leave the screen's letters and numbers alone: typed,
+/// they open nothing.
+#[test]
+fn the_mac_look_has_no_start_letters() {
+    let (mut harness, tab) = bookshop(Look::macos());
+    type_key(&mut harness, Key::Num1, "1");
+    type_key(&mut harness, Key::T, "t");
+    assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
+    // Its buttons do the same work.
+    harness.click("New SQL query");
+    assert!(harness.has("Query 1 tab"));
+}
+
+/// The name a screen reader would read for the focused node.
+fn focused_name(tree: &egui::accesskit::TreeUpdate) -> String {
+    super::tests::focused_name(tree)
+}
+
+/// F6 and Tab reach the screen on macOS, and Enter on a focused row opens
+/// it; in the terminal look F6 reaches the pane as ctrl+l does. With no
+/// tab the strip is no part to step to.
+#[test]
+fn f6_and_tab_reach_the_start_screen() {
+    let (mut harness, tab) = bookshop(Look::macos());
+    visit(&mut harness, tab, "authors");
+    let row = "authors, table, main, opened Just now";
+    let mut stops = Vec::new();
+    for _ in 0..40 {
+        harness.press(Key::Tab, Modifiers::NONE);
+        stops.push(focused_name(&harness.settle()));
+        if stops.iter().any(|stop| stop == row) && stops.iter().any(|stop| stop == "New SQL query")
+        {
+            break;
+        }
+    }
+    assert!(stops.iter().any(|stop| stop == row), "{stops:?}");
+    assert!(
+        stops.iter().any(|stop| stop == "New SQL query"),
+        "{stops:?}"
+    );
+    while focused_name(&harness.settle()) != row {
+        harness.press(Key::Tab, Modifiers::NONE);
+    }
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(harness.has("authors tab"));
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::CloseTab { tab, id });
+    let mut seen = Vec::new();
+    for _ in 0..8 {
+        harness.press(Key::F6, Modifiers::NONE);
+        seen.push(focused_name(&harness.settle()));
+    }
+    assert!(seen.iter().any(|stop| stop == row), "{seen:?}");
+    assert!(seen.iter().any(|stop| stop == "Objects"), "{seen:?}");
+    // The terminal look's pane.
+    let (mut harness, tab) = bookshop(Look::omarchy());
+    harness.press(Key::L, Modifiers::CTRL);
+    assert_eq!(focused_name(&harness.settle()), "Start");
+    harness.press(Key::H, Modifiers::CTRL);
+    assert_eq!(focused_name(&harness.settle()), "Objects");
+    let mut seen = Vec::new();
+    for _ in 0..8 {
+        harness.press(Key::F6, Modifiers::NONE);
+        seen.push(focused_name(&harness.settle()));
+    }
+    assert!(seen.iter().any(|stop| stop == "Start"), "{seen:?}");
+    assert_eq!(harness.app.workspace(tab).unwrap().tabs.len(), 0);
+}
+
+/// A SQL editor opened again from the screen is the editor it was, by
+/// its number too, and no number is handed out for it.
+#[test]
+fn a_reopened_sql_editor_keeps_its_number() {
+    let (mut harness, tab) = bookshop(Look::macos());
+    for _ in 0..2 {
+        harness.app.apply(Action::NewSqlTab(tab));
+    }
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness
+        .app
+        .workspace_mut(tab)
+        .unwrap()
+        .sql_tab_mut(id)
+        .unwrap()
+        .text = "SELECT 1".into();
+    harness.app.apply(Action::CloseTab { tab, id });
+    let first = harness.app.workspace(tab).unwrap().tabs[0].id();
+    harness.app.apply(Action::CloseTab { tab, id: first });
+    let next = harness.app.workspace(tab).unwrap().next_query;
+    assert_eq!(rows(&mut harness), ["Query 2, SQL editor, opened Just now"]);
+    harness.app.apply(Action::OpenRecent { tab, index: 0 });
+    let workspace = harness.app.workspace(tab).unwrap();
+    let sql = workspace.active_sql_tab().expect("the editor");
+    assert_eq!((sql.number, &sql.text[..]), (2, "SELECT 1"));
+    assert_eq!(workspace.next_query, next, "no number handed out");
+    let id = sql.id;
+    assert!(harness.has("Query 2 tab"));
+    // Closed and opened again, it is still Query 2.
+    harness.app.apply(Action::CloseTab { tab, id });
+    harness.app.apply(Action::OpenRecent { tab, index: 0 });
+    let workspace = harness.app.workspace(tab).unwrap();
+    assert_eq!(workspace.active_sql_tab().unwrap().number, 2);
+    assert_eq!(workspace.next_query, next);
+}
+
+/// A listing that failed says nothing of what exists: the recents stay,
+/// and nothing is written.
+#[test]
+fn a_failed_listing_keeps_the_recents() {
+    crate::util::pin_now(Some(NOW));
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    let saved = fixture_connection();
+    let database = crate::session::database_of(&saved.spec);
+    let conn = saved.id.clone();
+    for (name, opened) in [("authors", NOW - 100), ("book_covers", NOW - 200)] {
+        let recent = Recent::of(
+            &ObjectRef::new("main", name),
+            ObjectKind::Table,
+            &database,
+            opened,
+        );
+        harness.app.recents.note(&conn.0, recent);
+    }
+    let file = harness.app.dirs.recents_file();
+    let before = saves_of(&harness, &file);
+    harness.app.connections.upsert(saved);
+    let tab = harness.app.active_tab_id();
+    harness.app.apply(Action::Connect {
+        tab,
+        conn: conn.clone(),
+    });
+    let Command::Connect {
+        session, request, ..
+    } = *crate::testing::last_sent(&harness.app)
+    else {
+        panic!("expected Connect");
+    };
+    harness.app.apply(Action::Backend(Event::Connected {
+        session,
+        request,
+        driver: Driver::Sqlite,
+        encrypted: false,
+        access: crate::testing::asked_access(&harness.app),
+    }));
+    let Command::ListSchemas { request, .. } = *crate::testing::last_sent(&harness.app) else {
+        panic!("expected ListSchemas");
+    };
+    harness.app.apply(Action::Backend(Event::Schemas {
+        session,
+        request,
+        result: Ok(vec!["main".into()]),
+    }));
+    let Command::ListObjects { request, .. } = *crate::testing::last_sent(&harness.app) else {
+        panic!("expected ListObjects");
+    };
+    harness.app.apply(Action::Backend(Event::Objects {
+        session,
+        request,
+        schema: "main".into(),
+        result: Err(tabletist_db::Error::query(
+            "permission denied for schema main",
+        )),
+    }));
+    assert_eq!(harness.app.recents.of(&conn.0, &database).len(), 2);
+    assert_eq!(saves_of(&harness, &file), before, "nothing was written");
+    assert_eq!(harness.app.start_recents(tab).len(), 2);
+}
+
+/// A connection deleted while its window is open is not written back by
+/// an open in that window.
+#[test]
+fn a_deleted_connection_is_not_written_back_by_an_open() {
+    let (mut harness, tab) = bookshop(Look::macos());
+    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+    visit(&mut harness, tab, "authors");
+    harness.app.apply(Action::DeleteConnection(conn.clone()));
+    let file = harness.app.dirs.recents_file();
+    let saves = saves_of(&harness, &file);
+    visit(&mut harness, tab, "publishers");
+    let database = crate::session::database_of(&harness.app.workspace(tab).unwrap().spec);
+    assert!(harness.app.recents.of(&conn.0, &database).is_empty());
+    assert_eq!(saves_of(&harness, &file), saves);
+    assert!(harness.app.start_recents(tab).is_empty());
+}
diff --git a/tests/keys.rs b/tests/keys.rs
index c946307..b0db622 100644
--- a/tests/keys.rs
+++ b/tests/keys.rs
@@ -19,7 +19,7 @@ const MAY: [&str; 2] = ["src/keymap.rs", "src/typography/fonts.rs"];
 
 /// The files that are tests and nothing else, or scenes for screenshots: a
 /// test says what the user presses and sees, in its own words.
-const TESTS: [&str; 10] = [
+const TESTS: [&str; 11] = [
     "src/testing.rs",
     "src/shots.rs",
     "src/ui/complete_tests.rs",
@@ -30,6 +30,7 @@ const TESTS: [&str; 10] = [
     "src/ui/insert_audit_tests.rs",
     "src/ui/key_picker_tests.rs",
     "src/ui/label_tests.rs",
+    "src/ui/start_tests.rs",
 ];
 
 /// Every Rust file of the app, by its path from the repository root, with
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib -- start_tests the_object_tab_close_button_closes_it`

Expected: the build passes (everything the tests name is task 1's), and every test of what the screen draws and does fails, as nothing draws it yet. Three pass already, as they read only what task 1 built: `the_mac_sidebar_has_no_recent_list` (the list went with task 1), `a_failed_listing_keeps_the_recents` and `a_deleted_connection_is_not_written_back_by_an_open`:

```
test ui::start_tests::the_terminal_status_line_says_start ... FAILED
test ui::start_tests::the_start_screen_shows_with_no_tab_and_comes_back_when_the_last_closes ... FAILED
test ui::start_tests::the_first_visit_lists_the_largest_tables_from_the_catalog ... FAILED
test ui::start_tests::recents_are_per_connection_newest_first_five_of_eight_and_cleared_apart ... FAILED
test ui::start_tests::a_reopened_sql_editor_keeps_its_number ... FAILED
test ui::start_tests::production_shows_the_strip_whatever_the_environment_color_says ... FAILED
test ui::start_tests::the_connection_line_names_the_server_and_counts_the_objects ... FAILED
test ui::start_tests::a_recent_that_no_longer_exists_is_dropped_when_the_schema_is_listed ... FAILED
test ui::start_tests::the_terminal_start_keys_work_only_with_the_start_screen_focused ... FAILED
test ui::start_tests::a_closed_sql_editor_with_a_script_is_a_recent_that_opens_again ... FAILED
test ui::start_tests::the_mac_look_has_no_start_letters ... FAILED
test ui::tests::the_object_tab_close_button_closes_it ... FAILED
test ui::start_tests::f6_and_tab_reach_the_start_screen ... FAILED
test result: FAILED. 3 passed; 13 failed; 0 ignored; 0 measured; 2889 filtered out; finished in 0.11s
red exit 101
```

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
index 7acf33b..7ff0ac6 100644
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -23,6 +23,16 @@ launch, are on the
   the chips of the open connections sit beside the window's buttons, as
   tabs do in Safari. In full screen the window's buttons are hidden and
   the bar takes the whole width.
+- **A window with no tab open shows the start screen.** One line says
+  which database you are on, which server serves it and how many tables
+  and views it lists (a production connection gets its red strip there,
+  with Read-only where it is). Under it, the five tables, views and SQL
+  editors you opened last on this connection, newest first; Clear empties
+  the list. On your first visit the screen lists the largest tables
+  instead, from the database's own statistics, so nothing is queried (a
+  SQLite file lists its tables by name). Then
+  three buttons: New SQL query, Switch database and Edit connection.
+  Closing the last tab brings the screen back.
 - **Light and dark follow the system.** Tabletist changes with the
   Appearance setting while it runs. **Settings › Appearance** can keep it
   light or dark whatever the system says.
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index e67311d..4ee13a7 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -30,6 +30,7 @@ back.
 | Ctrl+Shift+1 to 9, Ctrl+Tab, Ctrl+Shift+Tab | Switch connection |
 | Ctrl+Shift+W | Close the connection |
 | Ctrl+T | New SQL editor |
+| Ctrl+Shift+D | Switch database, where the server has another |
 | Alt+1 to 9 | Go to that tab |
 | Ctrl+Shift+[ and ] | Previous and next tab |
 | Ctrl+W | Close the tab. While you type in a field it deletes a word instead |
@@ -89,6 +90,26 @@ list instead.
 | `ga` | Show the Agents tab |
 | Ctrl+B | Hide the sidebar |
 
+### The start screen
+
+A window with no tab open shows the start screen in the main pane: the
+connection in one line (a production connection gets its red strip there,
+with `read-only` where it is), the five tables, views and SQL editors you
+opened last on this connection, numbered, and under `start` the ways to
+begin. On your first visit it lists the largest tables instead, from the
+database's statistics, so nothing is queried (a SQLite file lists its
+tables by name). The status line says `-- START --` while no tab is open.
+After a connect the keyboard is in the sidebar; Ctrl+L gives it to the
+screen, whose border lights, and Ctrl+H puts it back.
+
+| Keys | Do |
+| --- | --- |
+| `1` to `9` | Open that row |
+| `t` | New SQL editor |
+| `d` | Switch database, where the server has another |
+| `e` | Edit the connection |
+| `a` | Show the Agents tab |
+
 ### A table
 
 | Keys | Do |
diff --git a/src/env.rs b/src/env.rs
index e03041a..c2a5e1d 100644
--- a/src/env.rs
+++ b/src/env.rs
@@ -93,6 +93,15 @@ impl Environment {
         }
     }
 
+    /// Whether this is production: the one environment whose surfaces are
+    /// always drawn in its colours, whatever the settings say.
+    pub fn is_production(self) -> bool {
+        match self {
+            Self::Production => true,
+            Self::Local | Self::Dev | Self::Staging | Self::None => false,
+        }
+    }
+
     /// The badge text: the lower-case name in a macOS or Windows pill, the
     /// terminal's upper-case tag on Omarchy.
     pub fn label(self, platform: Platform) -> &'static str {
@@ -891,6 +900,8 @@ mod tests {
             "src/ui/connect_dialog/mod.rs",
             "src/ui/env_tests.rs",
             "src/ui/settings/preview.rs",
+            "src/ui/start.rs",
+            "src/ui/start_tests.rs",
         ];
         let mut found = Vec::new();
         for (path, text) in sources() {
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 680e1e7..f034a4e 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -63,6 +63,7 @@ pub mod sql_complete;
 pub mod sql_editor;
 pub mod sql_results;
 pub mod sql_text;
+pub mod start;
 #[cfg(test)]
 mod start_tests;
 pub mod states;
diff --git a/src/ui/object_tabs.rs b/src/ui/object_tabs.rs
index 1f338d9..7ddfc9e 100644
--- a/src/ui/object_tabs.rs
+++ b/src/ui/object_tabs.rs
@@ -75,9 +75,9 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .collect();
     let active = workspace.active_tab;
     let row_panel = workspace.row_panel;
-    if tabs.is_empty() && !look.terminal {
-        return;
-    }
+    // With no tab the strip is empty, and the start screen stands under
+    // it: nothing of a tab's to toggle.
+    let empty = tabs.is_empty();
     let mut actions = Vec::new();
     let fill = if look.terminal {
         palette.panel
@@ -91,7 +91,10 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .frame(Frame::new().fill(fill))
         .show(ui, |ui| {
             let full = ui.max_rect();
-            focus::region(ui, focus::Region::Tabs, full);
+            // With no tab there is nothing here to step to.
+            if !empty {
+                focus::region(ui, focus::Region::Tabs, full);
+            }
             let rule = if look.terminal {
                 palette.outline
             } else {
@@ -207,6 +210,9 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             // The row panel's toggle, in every look and on every tab: a SQL
             // editor's result has a row panel too. A closed panel has no
             // button of its own left to open it with.
+            if empty {
+                return;
+            }
             let cell = Rect::from_min_max(pos2(bar.right() - TOGGLE_CELL, bar.top()), bar.max);
             ui.painter().rect_filled(cell, CornerRadius::ZERO, fill);
             widgets::vline(ui, cell.left() + 0.5, bar.y_range(), rule);
diff --git a/src/ui/start.rs b/src/ui/start.rs
new file mode 100644
index 0000000..0ec20e2
--- /dev/null
+++ b/src/ui/start.rs
@@ -0,0 +1,945 @@
+//! The start screen: what a connection's window shows with no tab open.
+//! The connection in one line, what was opened lately on it (on a first
+//! visit, its largest tables) and three ways to start. On macOS a column
+//! in the middle of the pane with a status bar under it; in the terminal
+//! look a dashboard whose keys the status line lists: while the screen has
+//! the keyboard, a number opens a row and a letter starts something.
+
+use egui::{CornerRadius, Frame, Margin, Rect, Sense, Stroke, WidgetInfo, WidgetType};
+use egui::{pos2, vec2};
+use tabletist_db::{Driver, ObjectKind, ObjectRef};
+
+use crate::app::App;
+use crate::env::{Platform, env_colors};
+use crate::i18n::{Locale, gettext};
+use crate::keymap::{Command, Scope, Written};
+use crate::model::{Action, ConnTabId, Pane, StartRecent};
+use crate::theme::{Icon, Look, Palette};
+use crate::typography::{Text, TextRole};
+use crate::ui::focus::{self, Region};
+use crate::ui::format::display_safe;
+use crate::ui::widgets::{self, ButtonSpec};
+use crate::ui::workspace::{Badge, BarInfo, bar_info, database_menu, database_menu_id, env_badge};
+
+/// The column's width, and the room above it.
+fn column(look: &Look) -> (f32, f32) {
+    if look.terminal {
+        (760.0, 96.0)
+    } else {
+        (620.0, 72.0)
+    }
+}
+
+/// Between the column's parts.
+fn gap(look: &Look) -> f32 {
+    if look.terminal { 20.0 } else { 22.0 }
+}
+
+/// A row of the lists.
+fn row_height(look: &Look) -> f32 {
+    if look.terminal { 24.0 } else { 36.0 }
+}
+
+/// A row's inset, and a list's: 10 at each side.
+const INSET: f32 = 10.0;
+
+/// The macOS status bar's height.
+const STATUS_BAR: f32 = 32.0;
+
+/// The terminal status line's height.
+const STATUS_LINE: f32 = 31.0;
+
+/// What the screen shows of the window, gathered before anything is
+/// drawn.
+struct Shown {
+    conn: crate::connections::ConnectionId,
+    /// The database's name, or a SQLite file's.
+    database: String,
+    info: BarInfo,
+    driver: Driver,
+    /// The server as it names itself, once it has.
+    server: Option<String>,
+    /// How many tables and views the shown schema has, once it is listed.
+    objects: Option<usize>,
+    recents: Vec<StartRecent>,
+    /// On a first visit, the shown schema's largest tables.
+    largest: Vec<(ObjectRef, Option<u64>)>,
+    agents: Option<crate::model::AgentsHere>,
+    /// Whether the connection is still saved: Edit connection needs it.
+    saved: bool,
+    /// Whether the keys are the screen's: the keyboard is in the pane,
+    /// and on no control of it.
+    has_keys: bool,
+}
+
+fn shown(app: &App, ctx: &egui::Context, tab: ConnTabId) -> Option<Shown> {
+    let workspace = app.workspace(tab)?;
+    let info = bar_info(app, tab)?;
+    // A SQLite connection is on a file, which names it.
+    let database = match workspace.driver {
+        Driver::Sqlite => workspace
+            .spec
+            .sqlite_path
+            .as_ref()
+            .and_then(|path| path.file_name())
+            .map(|name| name.to_string_lossy().into_owned())
+            .unwrap_or_default(),
+        Driver::Postgres | Driver::MySql => info.database.clone(),
+    };
+    let objects = app
+        .start_schema(tab)
+        .and_then(|schema| workspace.tree.nodes.get(&schema))
+        .and_then(|node| node.objects.value.as_ref())
+        .map(Vec::len);
+    let recents = app.start_recents(tab);
+    let largest = if recents.is_empty() {
+        app.start_largest(tab)
+    } else {
+        Vec::new()
+    };
+    Some(Shown {
+        conn: workspace.conn_id.clone(),
+        database,
+        info,
+        driver: workspace.driver,
+        server: workspace.server_version.value.clone(),
+        objects,
+        recents,
+        largest,
+        agents: app.agents_here(tab),
+        saved: app.connections.get(&workspace.conn_id).is_some(),
+        has_keys: workspace.pane == Pane::Grid && !focus::on_control(ctx),
+    })
+}
+
+/// The keys the screen names, as the keymap writes them for the look.
+struct Keys {
+    /// New SQL query's key everywhere, and the one the screen reads.
+    sql: Written,
+    sql_here: Written,
+    /// Switch database's key everywhere, and the one the screen reads.
+    database: Written,
+    database_here: Written,
+    edit_here: Written,
+    /// Show the Agents tab's key on a table's rows (`ga`), and the one
+    /// the screen reads.
+    agents_rows: Written,
+    agents_here: Written,
+    /// The row numbers, and the keys the status line lists after them.
+    recent: Written,
+    sidebar: Written,
+    help: Written,
+}
+
+fn keys(app: &App) -> Keys {
+    let layout = app.layout();
+    let here = |command: Command| app.keymap.key_in(layout, command, Scope::Start);
+    let key = |command: Command| app.keymap.key(layout, command);
+    Keys {
+        sql: app.keymap.key_in(layout, Command::NewSqlTab, Scope::Global),
+        sql_here: here(Command::NewSqlTab),
+        database: app
+            .keymap
+            .key_in(layout, Command::SwitchDatabase, Scope::Global),
+        database_here: here(Command::SwitchDatabase),
+        edit_here: here(Command::EditConnection),
+        agents_rows: app.keymap.key_in(layout, Command::ShowAgents, Scope::Grid),
+        agents_here: here(Command::ShowAgents),
+        recent: here(Command::OpenRecent),
+        sidebar: key(Command::ToggleSidebar),
+        help: key(Command::Help),
+    }
+}
+
+/// What the screen draws with.
+#[derive(Clone, Copy)]
+struct Skin<'a> {
+    look: &'a Look,
+    palette: &'a Palette,
+    locale: Locale,
+    tab: ConnTabId,
+    /// Seconds since the Unix epoch, for how long ago a row was opened.
+    now: u64,
+    /// Whether numbers are written in threes.
+    grouped: bool,
+}
+
+impl Skin<'_> {
+    fn say(&self, text: &'static str) -> String {
+        self.look.label(&gettext(self.locale, text))
+    }
+}
+
+pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
+    let (look, palette, locale) = (app.look, app.palette, app.locale);
+    let Some(shown) = shown(app, ui.ctx(), tab) else {
+        return;
+    };
+    let keys = keys(app);
+    let skin = Skin {
+        look: &look,
+        palette: &palette,
+        locale,
+        tab,
+        now: crate::util::now_secs(),
+        grouped: app.settings.grouped(app.layout()),
+    };
+    let mut actions = Vec::new();
+    if !look.terminal {
+        status_bar(ui, skin);
+    }
+    egui::CentralPanel::default()
+        .frame(Frame::new().fill(palette.surface))
+        .show(ui, |ui| {
+            let full = ui.max_rect();
+            focus::region(ui, Region::Start, full);
+            // The terminal look's pane: one Tab stop whose keys are the
+            // screen's, lit while the keyboard is in it. The other looks
+            // put the keyboard on the screen's controls.
+            if look.terminal {
+                let pane = ui.interact(full, egui::Id::new(("start-pane", tab.0)), Sense::click());
+                let name = gettext(locale, "Start");
+                pane.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, name.clone()));
+                focus::claim(ui, Region::Start, &pane);
+                focus::pane(ui, &pane);
+                if pane.clicked() {
+                    pane.request_focus();
+                }
+                if pane.gained_focus() {
+                    actions.push(Action::GridKeys(tab));
+                }
+                if shown.has_keys {
+                    focus::pane_border(ui, full, &look, &palette);
+                }
+            }
+            let (width, top) = column(&look);
+            let width = width.min(full.width() - 48.0).max(0.0);
+            let column = Rect::from_min_size(
+                pos2(full.center().x - width / 2.0, full.top() + top),
+                vec2(width, (full.height() - top).max(0.0)),
+            );
+            let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(column));
+            ui.set_clip_rect(full.intersect(ui.clip_rect()));
+            ui.spacing_mut().item_spacing.y = 0.0;
+            connection_line(&mut ui, &shown, skin);
+            ui.add_space(gap(&look));
+            // On macOS the first row, or the first button where there is
+            // none, is where F6 brings the keyboard.
+            let mut claimed = look.terminal;
+            lists(&mut ui, &shown, &keys, skin, &mut claimed, &mut actions);
+            ui.add_space(gap(&look));
+            starts(&mut ui, &shown, &keys, skin, &mut claimed, &mut actions);
+        });
+    app.actions.extend(actions);
+}
+
+/// The words after the database's name: `Read-only` on a production
+/// connection that is, the server as it names itself (or its kind, until
+/// it has), and how many tables and views the shown schema has.
+fn line_words(shown: &Shown, skin: Skin<'_>) -> String {
+    let mut parts = Vec::new();
+    if shown.info.env.is_production() && shown.info.read_only {
+        parts.push(skin.say("Read-only"));
+    }
+    parts.push(
+        shown
+            .server
+            .clone()
+            .unwrap_or_else(|| shown.driver.label().to_owned()),
+    );
+    if let Some(count) = shown.objects {
+        let word = if skin.look.terminal {
+            "objects"
+        } else {
+            "tables and views"
+        };
+        parts.push(format!("{count} {}", skin.say(word)));
+    }
+    skin.look.label(&parts.join(" · "))
+}
+
+/// The connection in one line: its environment's dot (the terminal's
+/// tag), the database in the mono face, then [`line_words`]. On a
+/// production connection the line is the production strip, whatever the
+/// Environment color setting says of the bar.
+fn connection_line(ui: &mut egui::Ui, shown: &Shown, skin: Skin<'_>) {
+    let (look, palette) = (skin.look, skin.palette);
+    let platform = Platform::of(look);
+    let env = shown.info.env;
+    let colors = env_colors(env, platform, palette);
+    let name = display_safe(&shown.database).into_owned();
+    let words = line_words(shown, skin);
+    let whole = format!("{name} · {words}");
+    let (name_role, words_role) = if look.terminal {
+        (TextRole::OBody, TextRole::OBody)
+    } else {
+        (TextRole::MonoSecondary, widgets::secondary(look))
+    };
+    let height = if look.terminal { 20.0 } else { 18.0 };
+    if env.is_production() {
+        // The strip: the production bar's tint and rule, its tag, and the
+        // words in its ink.
+        let (corner, stroke) = if look.terminal {
+            (CornerRadius::ZERO, Stroke::NONE)
+        } else {
+            (
+                CornerRadius::same(look.radius),
+                Stroke::new(1.0, colors.bar_border()),
+            )
+        };
+        let shown_at = Frame::new()
+            .fill(colors.bar_bg())
+            .stroke(stroke)
+            .corner_radius(corner)
+            .inner_margin(Margin::symmetric(12, 8))
+            .show(ui, |ui| {
+                ui.set_width(ui.available_width());
+                let (rect, _) =
+                    ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+                let y = rect.center().y;
+                let style = if look.terminal {
+                    Badge::Tracked
+                } else {
+                    Badge::Mac
+                };
+                let mut x = rect.left();
+                x += env_badge(ui, x, y, env, &colors, style, look) + 10.0;
+                let name = Text::one(look, name_role, &name, palette.text);
+                x += widgets::paint_text(ui, x, y, name) + 8.0;
+                let ink = if look.terminal {
+                    palette.dim
+                } else {
+                    colors.base()
+                };
+                // The board's strip has no dot after the name; the
+                // terminal's has, as its line does.
+                let said = if look.terminal {
+                    format!("· {words}")
+                } else {
+                    words.clone()
+                };
+                let words = Text::one(look, words_role, &said, ink);
+                widgets::paint_text(ui, x, y, words);
+                widgets::announce(ui, rect, &whole);
+            });
+        if look.terminal {
+            // The terminal's strip has a rule in the production red above.
+            let rect = shown_at.response.rect;
+            widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.danger);
+        }
+        return;
+    }
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    let y = rect.center().y;
+    let mut x = rect.left();
+    if look.terminal {
+        x += env_badge(ui, x, y, env, &colors, Badge::Tracked, look) + 10.0;
+    } else {
+        // The dot in the environment's colour.
+        ui.painter()
+            .circle_filled(pos2(x + 4.0, y), 4.0, colors.base());
+        x += 16.0;
+    }
+    let name_ink = if look.terminal {
+        palette.text
+    } else {
+        palette.secondary
+    };
+    let name = Text::one(look, name_role, &name, name_ink);
+    x += widgets::paint_text(ui, x, y, name) + 8.0;
+    let words = Text::one(look, words_role, &format!("· {words}"), palette.dim);
+    widgets::paint_text(ui, x, y, words);
+    widgets::announce(ui, rect, &whole);
+}
+
+/// One row of a list: what it shows, what a screen reader says of it, and
+/// what a click does.
+struct Row {
+    icon: Icon,
+    name: String,
+    /// The muted words after the name: the kind and the schema, or a
+    /// script's first comment.
+    detail: String,
+    /// At the right: when it was opened, or a table's estimate.
+    meta: String,
+    label: String,
+    action: Action,
+}
+
+/// The recents, or on a first visit the largest tables.
+fn lists(
+    ui: &mut egui::Ui,
+    shown: &Shown,
+    keys: &Keys,
+    skin: Skin<'_>,
+    claimed: &mut bool,
+    actions: &mut Vec<Action>,
+) {
+    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
+    let zone = jiff::tz::TimeZone::system();
+    let kind_word = |kind: ObjectKind| {
+        skin.say(match kind {
+            ObjectKind::Table => "table",
+            ObjectKind::View => "view",
+            ObjectKind::MaterializedView => "materialized view",
+        })
+    };
+    let rows: Vec<Row> = if shown.recents.is_empty() {
+        shown
+            .largest
+            .iter()
+            .map(|(object, estimate)| {
+                let name = display_safe(&object.name).into_owned();
+                let schema = display_safe(&object.schema).into_owned();
+                let meta = estimate.map_or_else(String::new, |rows| estimate_text(rows, skin));
+                let detail = if look.terminal {
+                    kind_word(ObjectKind::Table)
+                } else {
+                    schema.clone()
+                };
+                let mut label = format!("{name}, {}, {schema}", kind_word(ObjectKind::Table));
+                if !meta.is_empty() {
+                    label = format!("{label}, {meta}");
+                }
+                Row {
+                    icon: Icon::Table,
+                    name,
+                    detail,
+                    meta,
+                    label,
+                    action: Action::OpenObject {
+                        tab: skin.tab,
+                        object: object.clone(),
+                        kind: ObjectKind::Table,
+                        pin: true,
+                    },
+                }
+            })
+            .collect()
+    } else {
+        shown
+            .recents
+            .iter()
+            .enumerate()
+            .map(|(index, recent)| {
+                let when = look.label(&crate::recents::when(recent.opened(), skin.now, &zone));
+                let action = Action::OpenRecent {
+                    tab: skin.tab,
+                    index,
+                };
+                let opened = skin.say("opened");
+                match recent {
+                    StartRecent::Object { object, kind, .. } => {
+                        let name = display_safe(&object.name).into_owned();
+                        let schema = display_safe(&object.schema).into_owned();
+                        let kind_word = kind_word(*kind);
+                        let detail = if look.terminal {
+                            kind_word.clone()
+                        } else {
+                            format!("{kind_word} · {schema}")
+                        };
+                        Row {
+                            icon: match kind {
+                                ObjectKind::Table => Icon::Table,
+                                ObjectKind::View | ObjectKind::MaterializedView => Icon::Eye,
+                            },
+                            label: format!("{name}, {kind_word}, {schema}, {opened} {when}"),
+                            name,
+                            detail,
+                            meta: when,
+                            action,
+                        }
+                    }
+                    StartRecent::Sql {
+                        number, comment, ..
+                    } => {
+                        let name = format!("{} {number}", skin.say("Query"));
+                        let editor = skin.say("SQL editor");
+                        let detail = if look.terminal {
+                            skin.say("sql")
+                        } else {
+                            comment.clone().unwrap_or_else(|| editor.clone())
+                        };
+                        let label = match comment {
+                            Some(comment) => {
+                                format!("{name}, {editor}, {comment}, {opened} {when}")
+                            }
+                            None => format!("{name}, {editor}, {opened} {when}"),
+                        };
+                        Row {
+                            icon: Icon::Code,
+                            name,
+                            detail,
+                            meta: when,
+                            label,
+                            action,
+                        }
+                    }
+                }
+            })
+            .collect()
+    };
+    // The heading, and what stands at its right: Clear, or the note that
+    // the estimates cost nothing.
+    let first_visit = shown.recents.is_empty();
+    // The terminal's board heads the list `recent`, the Mac's in full.
+    let title = if !first_visit {
+        if look.terminal {
+            gettext(locale, "Recent").into_owned()
+        } else {
+            gettext(locale, "Recent on this connection").into_owned()
+        }
+    } else {
+        match shown.driver {
+            Driver::Postgres | Driver::MySql => gettext(locale, "Largest tables").into_owned(),
+            Driver::Sqlite => gettext(locale, "Tables").into_owned(),
+        }
+    };
+    let note = if first_visit {
+        Some(match shown.driver {
+            Driver::Postgres | Driver::MySql => {
+                skin.say("From table statistics. Nothing is queried.")
+            }
+            Driver::Sqlite => skin.say("Nothing is queried."),
+        })
+    } else {
+        None
+    };
+    let width = ui.available_width();
+    let laid = widgets::section_label(&title, look, palette).layout(ui.ctx());
+    let head_height = laid.height() + if look.terminal { 2.0 } else { 6.0 };
+    let (head, _) = ui.allocate_exact_size(vec2(width, head_height), Sense::hover());
+    let inset = if look.terminal { 2.0 } else { INSET };
+    let y = head.top() + laid.height() / 2.0;
+    let mut x = head.left() + inset + laid.paint_left(ui.painter(), head.left() + inset, y);
+    // Said as the title is written, whatever case the look paints it in.
+    widgets::announce(
+        ui,
+        Rect::from_min_size(head.min, vec2(x - head.left(), head_height)),
+        &title,
+    );
+    if let Some(note) = note {
+        let role = TextRole::pick(look, TextRole::Secondary, TextRole::OCaption);
+        if look.terminal {
+            let note = Text::one(look, role, &format!(" · {note}"), palette.dim);
+            x += widgets::paint_label(ui, x, y, note);
+        } else {
+            let right = head.right() - inset;
+            let text = || Text::one(look, role, &note, palette.dim);
+            widgets::paint_text_right(ui, right, y, text());
+            let width = widgets::measure(ui, text());
+            widgets::announce(
+                ui,
+                Rect::from_min_size(pos2(right - width, head.top()), vec2(width, head_height)),
+                &note,
+            );
+        }
+    } else if !look.terminal {
+        // Clear empties the connection's list without asking: nothing is
+        // lost, the objects stay where they are.
+        let clear = gettext(locale, "Clear");
+        let button = ButtonSpec::new(&clear).link();
+        let button_width = button.width(ui, look);
+        let place = Rect::from_min_size(
+            pos2(head.right() - inset - button_width, head.top() - 2.0),
+            vec2(button_width, head_height + 2.0),
+        );
+        if button.show_at(ui, place, look, palette).clicked() {
+            actions.push(Action::ClearRecents(skin.tab));
+        }
+    }
+    let _ = x;
+    for (index, row) in rows.into_iter().enumerate() {
+        let response = list_row(ui, index, row, keys, skin, actions);
+        if !*claimed {
+            focus::claim(ui, Region::Start, &response);
+            *claimed = true;
+        }
+    }
+}
+
+/// A table's estimate, as the look writes numbers: `≈ 48,200 rows` on
+/// macOS, `≈ 48.2k` in the terminal look.
+fn estimate_text(rows: u64, skin: Skin<'_>) -> String {
+    if skin.look.terminal {
+        skin.look
+            .label(&format!("≈ {}", crate::ui::format::compact_count(rows)))
+    } else {
+        let count = if skin.grouped {
+            crate::ui::format::group_digits(rows)
+        } else {
+            rows.to_string()
+        };
+        format!("≈ {count} {}", skin.say("rows"))
+    }
+}
+
+/// One row: on macOS its icon, name, detail and what stands at the right;
+/// in the terminal look its number (the key that opens it), name, detail
+/// and the right-hand words.
+fn list_row(
+    ui: &mut egui::Ui,
+    index: usize,
+    row: Row,
+    keys: &Keys,
+    skin: Skin<'_>,
+    actions: &mut Vec<Action>,
+) -> egui::Response {
+    let (look, palette) = (skin.look, skin.palette);
+    let width = ui.available_width();
+    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height(look)), Sense::click());
+    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, row.label.clone()));
+    let hovered = response.hovered();
+    if look.terminal {
+        if hovered {
+            ui.painter()
+                .rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.08));
+        }
+    } else {
+        if hovered {
+            ui.painter()
+                .rect_filled(rect, CornerRadius::same(look.radius), palette.surface_hover);
+        }
+        let radius = look.radius.saturating_sub(2);
+        focus::hint(ui, &response, rect, focus::Ring::Inset { radius });
+    }
+    let y = rect.center().y;
+    let left = rect.left() + INSET;
+    let right = rect.right() - INSET;
+    if look.terminal {
+        // The number is the key: the keymap's digits, this one of them.
+        let number = if keys.recent.as_str().is_empty() {
+            String::new()
+        } else {
+            (index + 1).to_string()
+        };
+        widgets::paint_text(
+            ui,
+            left,
+            y,
+            Text::one(look, TextRole::OGroup, &number, palette.accent),
+        );
+        let meta = || Text::one(look, TextRole::OBody, &row.meta, palette.dim);
+        let meta_width = widgets::measure(ui, meta());
+        widgets::paint_text_right(ui, right, y, meta());
+        // The kind in a column of its own, before the right-hand words.
+        let detail_x = right - 110.0_f32.max(meta_width + 10.0) - 120.0;
+        widgets::paint_text(
+            ui,
+            detail_x,
+            y,
+            Text::one(look, TextRole::OBody, &row.detail, palette.dim),
+        );
+        let room = (detail_x - 10.0 - (left + 28.0)).max(0.0);
+        let name = crate::ui::grid::ellipsize(&row.name, room, false, |text| {
+            TextRole::OBody.width(ui.ctx(), look.faces, text)
+        });
+        widgets::paint_text(
+            ui,
+            left + 28.0,
+            y,
+            Text::one(look, TextRole::OBody, &name, palette.text),
+        );
+    } else {
+        let icon = Rect::from_center_size(pos2(left + 7.0, y), vec2(14.0, 14.0));
+        row.icon.image(palette.dim, 14.0).paint_at(ui, icon);
+        let meta = || Text::one(look, widgets::secondary(look), &row.meta, palette.dim);
+        let meta_width = widgets::measure(ui, meta());
+        widgets::paint_text_right(ui, right, y, meta());
+        let mut x = left + 18.0 + INSET;
+        let room = (right - meta_width - 10.0 - x).max(0.0);
+        let name_role = TextRole::UiBodyStrong;
+        let name = crate::ui::grid::ellipsize(&row.name, room, false, |text| {
+            name_role.width(ui.ctx(), look.faces, text)
+        });
+        x += widgets::paint_text(ui, x, y, Text::one(look, name_role, &name, palette.text)) + 8.0;
+        let room = (right - meta_width - 10.0 - x).max(0.0);
+        let detail = crate::ui::grid::ellipsize(&row.detail, room, false, |text| {
+            widgets::secondary(look).width(ui.ctx(), look.faces, text)
+        });
+        widgets::paint_text(
+            ui,
+            x,
+            y,
+            Text::one(look, widgets::secondary(look), &detail, palette.dim),
+        );
+    }
+    if response.clicked() {
+        actions.push(row.action);
+    }
+    response
+}
+
+/// The ways to start: three buttons on macOS; in the terminal look the
+/// `start` list, each line its key, what it does, and the key that does
+/// the same anywhere (or, for agents, who is here).
+fn starts(
+    ui: &mut egui::Ui,
+    shown: &Shown,
+    keys: &Keys,
+    skin: Skin<'_>,
+    claimed: &mut bool,
+    actions: &mut Vec<Action>,
+) {
+    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
+    let switchable = shown.info.databases.len() > 1;
+    let no_other = gettext(locale, "This connection has no other database to switch to");
+    let not_saved = gettext(locale, "This connection is no longer saved");
+    if !look.terminal {
+        let (sql, database, edit) = (
+            gettext(locale, "New SQL query"),
+            gettext(locale, "Switch database"),
+            gettext(locale, "Edit connection"),
+        );
+        let height = 30.0;
+        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+        let mut x = row.left() + INSET;
+        let mut place = |ui: &mut egui::Ui, button: ButtonSpec<'_>| {
+            let width = button.width(ui, look);
+            let rect = Rect::from_min_size(pos2(x, row.top()), vec2(width, height));
+            x += width + 8.0;
+            button.show_at(ui, rect, look, palette)
+        };
+        let mut button = ButtonSpec::new(&sql).icon(Icon::Code);
+        if !keys.sql.as_str().is_empty() {
+            button = button.shortcut(&keys.sql);
+        }
+        let response = place(ui, button);
+        if !*claimed {
+            focus::claim(ui, Region::Start, &response);
+            *claimed = true;
+        }
+        if response.clicked() {
+            actions.push(Action::NewSqlTab(skin.tab));
+        }
+        let mut button = ButtonSpec::new(&database).icon(Icon::Database);
+        if !keys.database.as_str().is_empty() {
+            button = button.shortcut(&keys.database);
+        }
+        if !switchable {
+            button = button.disabled(&no_other);
+        }
+        let response = place(ui, button);
+        if switchable {
+            let id = database_menu_id(skin.tab, true);
+            database_menu(
+                &response,
+                id,
+                skin.tab,
+                &shown.info,
+                (look, palette),
+                actions,
+            );
+        }
+        let mut button = ButtonSpec::new(&edit).icon(Icon::Settings);
+        if !shown.saved {
+            button = button.disabled(&not_saved);
+        }
+        if place(ui, button).clicked() {
+            actions.push(Action::EditConnection(shown.conn.clone()));
+        }
+        return;
+    }
+    let start = gettext(locale, "start");
+    let title = widgets::section_label(&start, look, palette).layout(ui.ctx());
+    let (head, _) = ui.allocate_exact_size(
+        vec2(ui.available_width(), title.height() + 2.0),
+        Sense::hover(),
+    );
+    title.paint_left(
+        ui.painter(),
+        head.left() + 2.0,
+        head.top() + title.height() / 2.0,
+    );
+    widgets::announce(ui, head, &start);
+    // Each line: its key, what it does, and the muted words after.
+    let agents_words = shown.agents.as_ref().map(|here| {
+        let (said, _, _) = crate::ui::sidebar::agents_said(here, look, palette, locale);
+        let key = keys.agents_rows.as_str();
+        if key.is_empty() {
+            said
+        } else {
+            format!("{key} · {said}")
+        }
+    });
+    let database_hint = if switchable {
+        keys.database.as_str().to_owned()
+    } else {
+        look.label(&no_other)
+    };
+    let edit_hint = if shown.saved {
+        String::new()
+    } else {
+        look.label(&not_saved)
+    };
+    let lines: Vec<(&Written, String, String, Option<Action>)> = vec![
+        (
+            &keys.sql_here,
+            skin.say("new sql query"),
+            keys.sql.as_str().to_owned(),
+            Some(Action::NewSqlTab(skin.tab)),
+        ),
+        (
+            &keys.database_here,
+            skin.say("switch database"),
+            database_hint,
+            None,
+        ),
+        (
+            &keys.edit_here,
+            skin.say("edit connection"),
+            edit_hint,
+            shown
+                .saved
+                .then(|| Action::EditConnection(shown.conn.clone())),
+        ),
+    ];
+    let agents = agents_words.map(|words| {
+        (
+            &keys.agents_here,
+            skin.say("agents"),
+            words,
+            Some(Action::ShowAgents(skin.tab)),
+        )
+    });
+    for (key, words, hint, action) in lines.into_iter().chain(agents) {
+        let width = ui.available_width();
+        let (rect, response) =
+            ui.allocate_exact_size(vec2(width, row_height(look)), Sense::click());
+        let label = if hint.is_empty() {
+            words.clone()
+        } else {
+            format!("{words}, {hint}")
+        };
+        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label.clone()));
+        if response.hovered() {
+            ui.painter()
+                .rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.08));
+        }
+        let y = rect.center().y;
+        let left = rect.left() + INSET;
+        widgets::paint_text(
+            ui,
+            left,
+            y,
+            Text::one(look, TextRole::OGroup, key.as_str(), palette.accent),
+        );
+        widgets::paint_text(
+            ui,
+            left + 28.0,
+            y,
+            Text::one(look, TextRole::OBody, &words, palette.text),
+        );
+        widgets::paint_text(
+            ui,
+            left + 28.0 + 220.0,
+            y,
+            Text::one(look, TextRole::OBody, &hint, palette.dim),
+        );
+        if words == skin.say("switch database") {
+            let id = database_menu_id(skin.tab, true);
+            if switchable {
+                database_menu(
+                    &response,
+                    id,
+                    skin.tab,
+                    &shown.info,
+                    (look, palette),
+                    actions,
+                );
+            }
+        } else if response.clicked()
+            && let Some(action) = action
+        {
+            actions.push(action);
+        }
+    }
+}
+
+/// The macOS status bar under the screen: that no table is open.
+fn status_bar(ui: &mut egui::Ui, skin: Skin<'_>) {
+    let (look, palette) = (skin.look, skin.palette);
+    egui::Panel::bottom(egui::Id::new(("start-status", skin.tab.0)))
+        .exact_size(STATUS_BAR)
+        .resizable(false)
+        .show_separator_line(false)
+        .frame(
+            Frame::new()
+                .fill(palette.panel)
+                .inner_margin(Margin::symmetric(20, 0)),
+        )
+        .show(ui, |ui| {
+            let full = ui.max_rect().expand2(vec2(20.0, 0.0));
+            widgets::hline(ui, full.x_range(), full.top() + 0.5, palette.outline);
+            // The status bar's grey, as a table's footer has it.
+            let status = palette.secondary.lerp_to_gamma(palette.dim, 0.5);
+            let text = gettext(skin.locale, "No table open");
+            let rect = ui.max_rect();
+            widgets::paint_label(
+                ui,
+                rect.left(),
+                rect.center().y + 0.5,
+                Text::one(look, widgets::secondary(look), &text, status),
+            );
+        });
+}
+
+/// The terminal look's status line with no tab open: the mode, the
+/// screen's keys (those that fit), and that no table is open.
+pub fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
+    let (look, palette, locale) = (app.look, app.palette, app.locale);
+    let keys = keys(app);
+    let words = [
+        "open recent",
+        "new sql",
+        "database",
+        "edit connection",
+        "tables",
+        "all keys",
+    ]
+    .map(|word| gettext(locale, word));
+    let hints: Vec<widgets::Hint<'_>> = vec![
+        (&keys.recent, &*words[0], true),
+        (&keys.sql_here, &*words[1], true),
+        (&keys.database_here, &*words[2], true),
+        (&keys.edit_here, &*words[3], true),
+        (&keys.sidebar, &*words[4], true),
+        (&keys.help, &*words[5], true),
+    ];
+    egui::Panel::bottom(egui::Id::new(("status-line", tab.0)))
+        .exact_size(STATUS_LINE)
+        .resizable(false)
+        .show_separator_line(false)
+        .frame(Frame::new().fill(palette.panel))
+        .show(ui, |ui| {
+            let rect = ui.max_rect();
+            widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
+            let y = rect.top() + 1.0 + 15.0;
+            let role = TextRole::OSecondary;
+            let gap = 18.0;
+            let mode = gettext(locale, "-- START --");
+            let mode = Text::one(&look, TextRole::OModeLine, &mode, palette.dim);
+            let left = rect.left() + 12.0;
+            let x = left + widgets::paint_label(ui, left, y, mode) + gap;
+            let right = rect.right() - 12.0;
+            let said = gettext(locale, "no table open");
+            let summary = || Text::one(&look, role, &said, palette.dim);
+            let summary_width = widgets::measure(ui, summary());
+            // Everything 18 apart. What does not fit before the summary
+            // gives way from the end.
+            let limit = right - summary_width - gap;
+            let shown = (0..=hints.len())
+                .rev()
+                .find(|&count| {
+                    x + widgets::key_hints_width(ui, &hints[..count], gap, &look, &palette) <= limit
+                })
+                .unwrap_or(0);
+            widgets::key_hints(ui, (x, y), &hints[..shown], gap, &look, &palette);
+            let width = widgets::paint_text_right(ui, right, y, summary());
+            widgets::announce(
+                ui,
+                Rect::from_min_size(pos2(right - width, y - 8.0), vec2(width.max(1.0), 16.0)),
+                &said,
+            );
+        });
+}
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index 1863bfd..b53f1be 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -151,18 +151,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                             }
                         });
                 }
-                None => {
-                    ui.centered_and_justified(|ui| {
-                        Text::one(
-                            &look,
-                            widgets::body(&look),
-                            &gettext(app.locale, "Select a table or view in the sidebar"),
-                            app.palette.secondary,
-                        )
-                        .layout(ui.ctx())
-                        .label(ui);
-                    });
-                }
+                None => super::start::show(app, ui, tab),
             }
         });
 }
@@ -2138,6 +2127,11 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         command_line(app, ui, tab);
         return;
     }
+    // No tab: the start screen's keys, and that no table is open.
+    if workspace.active_tab.is_none() {
+        super::start::status_line(app, ui, tab);
+        return;
+    }
     let read_only = workspace.access == tabletist_db::Access::ReadOnly;
     let editor = super::sql_editor::status_summary(app, tab);
     let on_editor = editor.is_some();
````

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`

Expected: all pass, among them every `ui::start_tests::*`, `ui::tests::the_object_tab_close_button_closes_it`, `env::tests::*`, and `tests/keys.rs`.

```
fmt exit 0
clippy exit 0
test result: ok. 2885 passed; 0 failed; 20 ignored; 0 measured; 0 filtered out; finished in 9.14s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.13s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 502 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.05s
test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.81s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 10 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 100 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.34s
test result: ok. 84 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.21s
test result: ok. 102 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.76s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.05s
test result: ok. 54 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.44s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.31s
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.06s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test exit 0
doc exit 0
```

- [ ] **Step 5: Commit**

```bash
git add src/ui/start.rs src/ui/start_tests.rs src/ui/mod.rs src/ui/workspace.rs src/ui/object_tabs.rs src/env.rs src/testing.rs tests/keys.rs docs/_guide/macos.md docs/_guide/omarchy.md
git commit -S -m "Show the start screen in a window with no tab open"
```

## The pull request

One pull request for the run, from the branch of these two commits (and the one with the plan). Its description: a summary; what shows when (the screen with no tab, after a connect with nothing restored, after Show Connections → open, after the last close; gone when anything opens), and that the macOS sidebar's Recent list is gone; the keys added (`1..9`, `t`, `d`, `e`, `a` on the start screen in the Omarchy look, `ctrl+shift+d` built) and that ⇧⌘T and `u` are run 4's, both free for the start screen in the keymap today (⇧⌘T is bound nowhere; `u` is undo in the grid and Paste URL in the connection form, and neither scope is read with the start screen); the departures above; no screenshot, or a blurred one.
