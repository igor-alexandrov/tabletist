# Two databases of one connection, run 1: the database menu opens a new window: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In the connection bar's database menu, Cmd-click or Cmd+Enter (Ctrl on Windows and Linux, Ctrl+Enter in the Omarchy look) opens the row's database in a new window right after this one, in front, with a session of its own and the password this one connected with. This window stays as it was. A click or Enter still switches this window. (igor-alexandrov/tabletist#366, run 1 of 3.)

**Architecture:** Three tasks, one PR, a commit each:
1. **The new window:** `Action::OpenDatabaseInNewWindow` and `App::open_database_beside`, plus the keymap's `OpenDatabaseInNewWindow` line, held while the database menu is open.
2. **The database menu:** the modifier click and the key on the lit row (read before any other chord, so a SQL editor in front does not run), the row's hint, and the keyboard put on the chip when a key opens the menu and given back when it closes.
3. **The docs:** the guide and the references.

**Tech Stack:** Rust, egui 0.36 (the crmne fork), the keymap in `crates/tabletist-core/src/keymap.rs`, the headless `testing::Harness`, GNU gettext 1.0 for `assets/i18n/update.sh`.

**Spec:** `docs/superpowers/specs/2026-11-21-two-databases-design.md` (decisions 1, 4, 5, 7, 8 and 9 are this run's; 2 and 3 are runs 3 and 2).

**Base:** `main` at c0eb151c. Work on this worktree's branch, `claude/two-databases-366-8b21f3`. If main moves before the PR, rebase onto it, never a merge of main, and run `assets/i18n/update.sh` again.

---

## The canvas and what this run builds

The design is on the canvas's Connections page, under "Multiple open": **macOS – Two databases of one connection** (`MacTwoDatabases`) and **Omarchy – Two databases of one connection** (`OmarchyTwoDatabases`). Keep them in step with the code: when the run builds something they do not show, update the boards in the same run.

| The boards show | This run builds | Why it differs |
|---|---|---|
| The lit row's hint "New window ⌘↩" / "New window Ctrl+Enter" / "ctrl+enter new window" | As drawn | |
| ⌘-click / ⌘↩ opens a new window right after this one, in front | As drawn | |
| The new window: nothing open, its own session, no password asked | As drawn | |
| A hint on the current database's row: a second window on it | As drawn | |
| The key is the open menu's own, like Copy as's letters, and not in Settings › Keyboard | As drawn (decision 8) | |
| — | A menu opened by its key gives the keyboard to its chip, so the arrows walk its rows, and gives it back where it was when it closes | Without it the arrows never reach the rows after ctrl+shift+d or quick open (decision 9) |
| — | At quit, of two windows of one connection on its own database, the first in the bar is the one whose tables are kept | A second window opens empty and, saved after the first, wiped what the first kept |
| Picker: "Open another" and "⇧↩ open another" | Not in this run | Run 3, after #367's picker work |
| Relaunch: each window as it was | Not in this run: a relaunch reopens the new window on the saved database, as a switched window is today | Run 2 |

## Files

| File | Task | What changes |
|---|---|---|
| `crates/tabletist-core/src/model.rs` | 1 | `Action::OpenDatabaseInNewWindow { tab, database }` |
| `src/app.rs` | 1 | Its arm in `App::apply`, `App::open_database_beside`, `open_workspace`'s `reopen`, and `save_session` keeping the first window's tables |
| `crates/tabletist-core/src/keymap.rs` | 1 | `Command::OpenDatabaseInNewWindow`, `When::DatabaseMenu`, its line (prompt scope, ⌘↩ / ctrl+enter) |
| `src/ui/menu.rs` | 2 | `Item::hint(words, key)`: drawn on the lit row only, its room kept on every row |
| `src/ui/workspace.rs` | 2 | `database_menu`: the key, the modifier click, the hint on the row the key goes to, the keyboard on the chip and back; `open_database_menu_by_key`, `ask_new_window`; its callers pass the locale |
| `src/ui/keys.rs` | 2 | The menu's key read first while the menu is open; the menu opened by its key through `open_database_menu_by_key` |
| `src/ui/start.rs` | 2 | The start screen's two calls of `database_menu` pass the locale |
| `src/ui/two_databases_tests.rs`, `src/ui/mod.rs` | 1, 2 | The tests |
| `assets/i18n/{tabletist.pot,ru.po,pt-BR.po}` | 1, 2 | "Open database in a new window" (1), "New window" (2) |
| `docs/_guide/getting-started.md`, `docs/_guide/omarchy.md`, `docs/_reference/connections.md`, `docs/_reference/keyboard-shortcuts.md` | 3 | The docs |

## Checks

Each task ends with these, all passing:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
assets/i18n/update.sh --check
```

Use `~/.cargo/bin/cargo`: the mise shim fails here. Keep `CARGO_TARGET_DIR` unset (the worktree's `target/`), never under `/tmp`.

---

### Task 1: The new window

**Files:**
- Create: `src/ui/two_databases_tests.rs`
- Modify: `src/ui/mod.rs`, `crates/tabletist-core/src/model.rs`, `src/app.rs`, `crates/tabletist-core/src/keymap.rs`, `assets/i18n/tabletist.pot`, `assets/i18n/ru.po`, `assets/i18n/pt-BR.po`

What it holds: the window opens right after the one it came from and comes to the front; it is the same saved connection on the chosen database, connected with the secrets the source window holds (`Workspace::secrets`), so a connection whose password is asked on every connect asks nothing; it opens on nothing, even on the saved database, where a connect from the picker would reopen what the connection kept (`open_workspace(.., reopen: false)`); the source window, its tables and the saved connection are untouched. `Action::ActivateConnection` follows the bar's order, so ⌘1..9 move up one for the windows after it. `save_session` keeps each window's tables from the last window to the first: of two windows of one connection on its own database, the first in the bar is the one kept, so an empty second one opened on it does not wipe what the first had (a window on another database keeps nothing of the connection's, as a switched window does today).

The keymap line is in the prompt scope with `When::DatabaseMenu`, as Copy as's letters are with `When::CopyAs`: fixed, not listed in Settings › Keyboard (spec decision 8). Task 2 reads it.

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -157,6 +157,8 @@
 #[cfg(test)]
 mod tree_reload_tests;
 #[cfg(test)]
+mod two_databases_tests;
+#[cfg(test)]
 mod typing_settings_tests;
 #[cfg(test)]
 mod update_settings_tests;
diff --git a/src/ui/two_databases_tests.rs b/src/ui/two_databases_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/two_databases_tests.rs
@@ -0,0 +1,222 @@
+//! Two databases of one connection: a database opens in a new window
+//! beside this one, and leaves this one as it was.
+
+use tabletist_db::{ConnectSpec, Driver, ObjectKind, ObjectRef};
+
+use crate::backend::{Command, Event};
+use crate::connections::{PasswordMode, SavedConnection};
+use crate::model::{Action, ConnTabId, Dialog, SessionStatus};
+use crate::testing::{Harness, answer_tree, bookshop_objects};
+use crate::theme::Look;
+
+/// The Bookshop's development server, whose password is asked for on
+/// every connect.
+fn bookshop_saved() -> SavedConnection {
+    let (spec, _) =
+        ConnectSpec::from_url("postgres://dev@localhost:5433/bookshop_development").unwrap();
+    SavedConnection {
+        id: crate::connections::ConnectionId::new(),
+        name: "Bookshop".into(),
+        environment: crate::env::Environment::Dev,
+        read_only: Some(true),
+        password: PasswordMode::Ask,
+        ssh_secret: PasswordMode::None,
+        spec,
+        agents: Default::default(),
+    }
+}
+
+/// Answers the last Connect sent as connected, and returns its session.
+fn answer_connect(harness: &mut Harness) -> crate::backend::SessionId {
+    let (session, request) = harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|command| match command {
+            Command::Connect {
+                session, request, ..
+            } => Some((*session, *request)),
+            _ => None,
+        })
+        .expect("a Connect was sent");
+    harness.app.apply(Action::Backend(Event::Connected {
+        session,
+        request,
+        driver: Driver::Postgres,
+        encrypted: false,
+        access: crate::testing::asked_access(&harness.app),
+    }));
+    session
+}
+
+/// The server's databases, as its listing would have answered.
+fn list_databases(harness: &mut Harness, tab: ConnTabId) {
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    workspace.databases.value = Some(vec![
+        "bookshop_development".into(),
+        "bookshop_test".into(),
+        "postgres".into(),
+    ]);
+}
+
+/// The Bookshop connected in `look`, its password typed at the prompt,
+/// with `books` open and the server's databases listed.
+fn bookshop(look: Look) -> (Harness, ConnTabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let saved = bookshop_saved();
+    let conn = saved.id.clone();
+    harness.app.connections.upsert(saved);
+    let tab = harness.app.active_tab_id();
+    harness.app.apply(Action::Connect { tab, conn });
+    let Some(Dialog::Password(prompt)) = &mut harness.app.dialog else {
+        panic!("the password is asked for");
+    };
+    prompt.password = "secret".into();
+    harness.app.apply(Action::SubmitPassword);
+    answer_connect(&mut harness);
+    answer_tree(&mut harness.app, vec!["public".into()], bookshop_objects());
+    list_databases(&mut harness, tab);
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: ObjectRef::new("public", "books"),
+        kind: ObjectKind::Table,
+        pin: true,
+    });
+    (harness, tab)
+}
+
+/// The windows the bar shows, in its order.
+fn windows(harness: &Harness) -> Vec<ConnTabId> {
+    harness.app.open_connections().map(|(tab, _)| tab).collect()
+}
+
+/// The database `tab` is on.
+fn database(harness: &Harness, tab: ConnTabId) -> String {
+    harness.app.workspace(tab).unwrap().spec.database.clone()
+}
+#[test]
+fn opening_a_database_in_a_new_window_leaves_this_one_as_it_was() {
+    let (mut harness, first) = bookshop(Look::macos());
+    let sent = harness.app.backend.sent.len();
+    harness.app.apply(Action::OpenDatabaseInNewWindow {
+        tab: first,
+        database: "bookshop_test".into(),
+    });
+    let [one, second] = windows(&harness)[..] else {
+        panic!("two windows: {:?}", windows(&harness));
+    };
+    assert_eq!(one, first);
+    assert_eq!(
+        harness.app.active_tab_id(),
+        second,
+        "the new one is in front"
+    );
+    // It connects with the password the first one holds: nothing asked.
+    assert!(harness.app.dialog.is_none(), "no second password prompt");
+    let connect = harness.app.backend.sent[sent..]
+        .iter()
+        .find_map(|command| match command {
+            Command::Connect { spec, secrets, .. } => Some((spec, secrets)),
+            _ => None,
+        })
+        .expect("the new window connects");
+    assert_eq!(connect.0.database, "bookshop_test");
+    assert_eq!(connect.1.password.as_deref(), Some("secret"));
+    answer_connect(&mut harness);
+    // Each on its own database, both connected, the first one's table
+    // still open and its saved connection unchanged.
+    assert_eq!(database(&harness, first), "bookshop_development");
+    assert_eq!(database(&harness, second), "bookshop_test");
+    for tab in [first, second] {
+        let status = &harness.app.workspace(tab).unwrap().status;
+        assert!(matches!(status, SessionStatus::Connected), "{status:?}");
+    }
+    let tables = |tab| {
+        let workspace = harness.app.workspace(tab).unwrap();
+        workspace
+            .object_tabs()
+            .map(|object| object.object.name.clone())
+            .collect::<Vec<_>>()
+    };
+    assert_eq!(tables(first), vec!["books".to_owned()]);
+    assert!(tables(second).is_empty(), "the new window opens on nothing");
+    let conn = &harness.app.workspace(first).unwrap().conn_id;
+    let saved = harness.app.connections.get(conn).unwrap();
+    assert_eq!(saved.spec.database, "bookshop_development");
+}
+
+#[test]
+fn the_new_window_goes_right_after_the_one_it_came_from() {
+    let (mut harness, first) = bookshop(Look::macos());
+    // A second connection after it, in a window of its own.
+    let mut other = bookshop_saved();
+    other.name = "Playground".into();
+    other.password = PasswordMode::None;
+    let conn = other.id.clone();
+    harness.app.connections.upsert(other);
+    harness.app.apply(Action::ShowConnections);
+    let tab = harness.app.active_tab_id();
+    harness.app.apply(Action::Connect { tab, conn });
+    answer_connect(&mut harness);
+    let playground = harness.app.active_tab_id();
+    harness.app.apply(Action::ActivateConnTab(first));
+    harness.app.apply(Action::OpenDatabaseInNewWindow {
+        tab: first,
+        database: "bookshop_test".into(),
+    });
+    let second = harness.app.active_tab_id();
+    assert_eq!(windows(&harness), vec![first, second, playground]);
+    // The second window's key is the second place's.
+    harness.app.apply(Action::ActivateConnTab(playground));
+    harness.app.apply(Action::ActivateConnection(1));
+    assert_eq!(harness.app.active_tab_id(), second);
+}
+
+#[test]
+fn the_current_database_opens_a_second_window_on_it() {
+    let (mut harness, first) = bookshop(Look::macos());
+    harness.app.apply(Action::OpenDatabaseInNewWindow {
+        tab: first,
+        database: "bookshop_development".into(),
+    });
+    let second = harness.app.active_tab_id();
+    assert_ne!(second, first);
+    assert_eq!(database(&harness, second), "bookshop_development");
+    assert!(harness.app.workspace(second).unwrap().tabs.is_empty());
+}
+#[test]
+fn quitting_keeps_the_first_windows_tables_not_a_second_ones() {
+    let (mut harness, first) = bookshop(Look::macos());
+    let conn = harness.app.workspace(first).unwrap().conn_id.clone();
+    let kept = |harness: &Harness| {
+        let kept = harness
+            .app
+            .session
+            .connections
+            .get(&conn.0)
+            .cloned()
+            .unwrap();
+        let names: Vec<String> = kept.tabs.into_iter().map(|tab| tab.name).collect();
+        (kept.database, names)
+    };
+    let books = ("bookshop_development".to_owned(), vec!["books".to_owned()]);
+    // A second window on the database the first is on, empty.
+    harness.app.apply(Action::OpenDatabaseInNewWindow {
+        tab: first,
+        database: "bookshop_development".into(),
+    });
+    answer_connect(&mut harness);
+    harness.app.save_session();
+    assert_eq!(kept(&harness), books);
+    // And one on another database: it keeps nothing of the connection's.
+    harness.app.apply(Action::OpenDatabaseInNewWindow {
+        tab: first,
+        database: "bookshop_test".into(),
+    });
+    answer_connect(&mut harness);
+    harness.app.save_session();
+    assert_eq!(kept(&harness), books);
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib two_databases`

Expected: the test build fails, the action is not there yet:

```
error[E0599]: no variant named `OpenDatabaseInNewWindow` found for enum `tabletist_core::model::Action`
```

- [ ] **Step 3: Write the code**

```diff
diff --git a/assets/i18n/pt-BR.po b/assets/i18n/pt-BR.po
--- a/assets/i18n/pt-BR.po
+++ b/assets/i18n/pt-BR.po
@@ -7,7 +7,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-19 23:02-0500\n"
+"POT-Creation-Date: 2026-11-21 10:36-0500\n"
 "PO-Revision-Date: 2026-11-19 16:11-0500\n"
 "Last-Translator: Automatically generated\n"
 "Language-Team: none\n"
@@ -1531,6 +1531,10 @@
 msgstr "Abrir um objeto recente"
 
 #: crates/tabletist-core/src/keymap.rs
+msgid "Open database in a new window"
+msgstr "Abrir o banco de dados em uma nova janela"
+
+#: crates/tabletist-core/src/keymap.rs
 msgid "Open every group"
 msgstr "Expandir todos os grupos"
 
diff --git a/assets/i18n/ru.po b/assets/i18n/ru.po
--- a/assets/i18n/ru.po
+++ b/assets/i18n/ru.po
@@ -7,7 +7,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-19 23:02-0500\n"
+"POT-Creation-Date: 2026-11-21 10:36-0500\n"
 "PO-Revision-Date: 2026-11-17 22:38-0500\n"
 "Last-Translator: Automatically generated\n"
 "Language-Team: none\n"
@@ -1529,6 +1529,10 @@
 msgstr "Открыть недавний объект"
 
 #: crates/tabletist-core/src/keymap.rs
+msgid "Open database in a new window"
+msgstr "Открыть базу данных в новом окне"
+
+#: crates/tabletist-core/src/keymap.rs
 msgid "Open every group"
 msgstr "Раскрыть все группы"
 
diff --git a/assets/i18n/tabletist.pot b/assets/i18n/tabletist.pot
--- a/assets/i18n/tabletist.pot
+++ b/assets/i18n/tabletist.pot
@@ -8,7 +8,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-19 23:02-0500\n"
+"POT-Creation-Date: 2026-11-21 10:36-0500\n"
 "PO-Revision-Date: YEAR-MO-DA HO:MI+ZONE\n"
 "Last-Translator: FULL NAME <EMAIL@ADDRESS>\n"
 "Language-Team: LANGUAGE <LL@li.org>\n"
@@ -741,6 +741,10 @@
 
 #: crates/tabletist-core/src/keymap.rs src/app/commands.rs src/ui/start.rs
 msgid "Switch database"
+msgstr ""
+
+#: crates/tabletist-core/src/keymap.rs
+msgid "Open database in a new window"
 msgstr ""
 
 #: crates/tabletist-core/src/keymap.rs src/app/commands.rs src/menu.rs
diff --git a/crates/tabletist-core/src/keymap.rs b/crates/tabletist-core/src/keymap.rs
--- a/crates/tabletist-core/src/keymap.rs
+++ b/crates/tabletist-core/src/keymap.rs
@@ -271,6 +271,8 @@
     DeletingConnection,
     /// The Copy as menu is open: its letters pick a format.
     CopyAs,
+    /// The database menu is open: the keys of its rows.
+    DatabaseMenu,
     /// The Export sheet (the terminal look's float) is open.
     Export,
     /// A failed export's line stands in the terminal look's status line.
@@ -466,6 +468,7 @@
     Connections = "connections",
     GoToConnectionWindow = "go_to_connection_window",
     SwitchDatabase = "switch_database",
+    OpenDatabaseInNewWindow = "open_database_in_new_window",
     Settings = "settings",
     CheckForUpdates = "check_for_updates",
     LargerText = "larger_text",
@@ -853,6 +856,7 @@
                 Window,
             ),
             C::SwitchDatabase => info(msgid("Switch database"), "Switch database", Window),
+            C::OpenDatabaseInNewWindow => info(msgid("Open database in a new window"), "", Window),
             C::Settings => info(msgid("Settings"), "Settings", Window),
             C::CheckForUpdates => info(msgid("Check for Updates…"), "", Window),
             C::LargerText => info(msgid("Larger"), "", Window),
@@ -1529,6 +1533,15 @@
             keys(&[k("ctrl+shift+1..9")]),
         ),
         bind(C::SwitchDatabase, GLOBAL, NONE, keys(&[k("ctrl+shift+d")])),
+        // On a row of the database menu: the database in a new window,
+        // where Enter switches this one.
+        bind(
+            C::OpenDatabaseInNewWindow,
+            PROMPT,
+            keys(&[k("cmd+enter")]),
+            keys(&[k("ctrl+enter")]),
+        )
+        .when(When::DatabaseMenu),
         // macOS's Tabletist menu has it; Omarchy's prompt.
         bind(C::CheckForUpdates, GLOBAL, NONE, keys(&[k(":update")])),
         bind(
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -1343,6 +1343,12 @@
     CancelPassword,
     /// Reconnect this tab to another database on the same server.
     SwitchDatabase {
+        tab: ConnTabId,
+        database: String,
+    },
+    /// Open another database of this tab's connection in a new window
+    /// beside it, leaving this tab as it is.
+    OpenDatabaseInNewWindow {
         tab: ConnTabId,
         database: String,
     },
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -3282,6 +3282,9 @@
                 // session for.
                 self.reconnect(tab);
             }
+            Action::OpenDatabaseInNewWindow { tab, database } => {
+                self.open_database_beside(tab, database);
+            }
             Action::Backend(event) => self.apply_event(event),
             // Not over a question about deleting a connection: a form opened
             // there would take the form the question holds.
@@ -3869,8 +3872,11 @@
         if !self.writes_session {
             return;
         }
+        // From the last window to the first: where two windows have one
+        // connection open on its own database, the first in the bar is the
+        // one kept, not a second opened on it (empty, or with less).
         let ids: Vec<ConnTabId> = self.tabs.iter().map(|tab| tab.id).collect();
-        for id in &ids {
+        for id in ids.iter().rev() {
             self.keep_tabs(*id);
         }
         let mut open = Vec::new();
@@ -3905,15 +3911,47 @@
         });
     }
 
+    /// Opens `database` of `tab`'s connection in a new window right after
+    /// it, in front. It connects with the secrets `tab` holds, so nothing
+    /// is asked again, and `tab` stays as it was: its tables, its editors,
+    /// its edits and its session. It opens on nothing: what the
+    /// connection's window kept is its own database's.
+    fn open_database_beside(&mut self, tab: ConnTabId, database: String) {
+        let Some(workspace) = self.workspace(tab) else {
+            return;
+        };
+        let typed = workspace.secrets.clone();
+        let Some(mut saved) = self.connections.get(&workspace.conn_id).cloned() else {
+            return;
+        };
+        let Some(index) = self.tab_index(tab) else {
+            return;
+        };
+        saved.spec.database = database;
+        let window = self.picker_tab();
+        let id = window.id;
+        self.tabs.insert(index + 1, window);
+        self.active = index + 1;
+        self.open_workspace(id, saved, typed, false);
+        self.authenticate(id);
+    }
+
     /// Opens `saved` in `tab`; `typed` holds secrets typed in the dialog.
     fn connect_tab(&mut self, tab: ConnTabId, saved: SavedConnection, typed: Secrets) {
-        self.open_workspace(tab, saved, typed);
+        self.open_workspace(tab, saved, typed, true);
         self.authenticate(tab);
     }
 
     /// Puts a connecting workspace for `saved` in `tab`, without starting to
-    /// authenticate.
-    fn open_workspace(&mut self, tab: ConnTabId, saved: SavedConnection, typed: Secrets) {
+    /// authenticate. With `reopen` it puts back the tables the connection's
+    /// window kept, where the settings reopen them.
+    fn open_workspace(
+        &mut self,
+        tab: ConnTabId,
+        saved: SavedConnection,
+        typed: Secrets,
+        reopen: bool,
+    ) {
         let session = SessionId(self.next_id());
         let request = RequestId(self.next_id());
         // Where the option puts it; the grid's link switches it from there.
@@ -3932,8 +3970,9 @@
         // A relaunch after an update puts back what the restart took away,
         // whatever the settings reopen.
         let restore = self.settings.restore_tabs || self.relaunch.is_some();
-        let tables = kept
-            .filter(|kept| restore && kept.database == crate::session::database_of(&saved.spec));
+        let tables = kept.filter(|kept| {
+            reopen && restore && kept.database == crate::session::database_of(&saved.spec)
+        });
         let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) else {
             return;
         };
@@ -6430,7 +6469,7 @@
             if let (Some(kind), Some(secret)) = (SecretKind::for_ssh(&saved.spec), &typed_ssh) {
                 *kind.slot(&mut secrets) = Some(secret.clone());
             }
-            self.open_workspace(tab, saved, secrets);
+            self.open_workspace(tab, saved, secrets, true);
             // Ask for a secret whose saved copy was for the old server,
             // rather than trusting the keyring to have dropped it.
             if let Some(workspace) = self.workspace_mut(tab) {
```

The catalogs come from `assets/i18n/update.sh`. The new message has no translation until it is written: give `ru.po` and `pt-BR.po` theirs as the diff does (a test fails on an entry left untranslated).

- [ ] **Step 4: Run the tests, then the checks**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib two_databases`

Expected: `test result: ok. 4 passed`. Then every check under Checks passes; the app's library ends `test result: ok. 3046 passed; 0 failed; 16 ignored; 0 measured; 0 filtered out; finished in …`.

- [ ] **Step 5: Commit**

```bash
git add src/ui/two_databases_tests.rs src/ui/mod.rs crates/tabletist-core/src/model.rs src/app.rs crates/tabletist-core/src/keymap.rs assets/i18n
git commit -m "Open a database of a connection in a new window beside its own (#366)"
```

---

### Task 2: The database menu

**Files:**
- Modify: `src/ui/two_databases_tests.rs`, `src/ui/menu.rs`, `src/ui/workspace.rs`, `src/ui/keys.rs`, `src/ui/start.rs`, `assets/i18n/tabletist.pot`, `assets/i18n/ru.po`, `assets/i18n/pt-BR.po`

What it holds: in `database_menu`, a click with the command modifier (egui's `modifiers.command`: ⌘ on macOS, Ctrl elsewhere) or the key of `OpenDatabaseInNewWindow` on the lit row (the keyboard's row, else the pointer's) pushes `Action::OpenDatabaseInNewWindow` and closes the menu; a plain click or Enter switches this window, as before.

The key is read in `keys::handle`, which runs before the window is drawn, first among the chords while one of the active window's database menus is open; it says so to the menu (`ask_new_window`, in egui's temporary data under the menu's id). Read anywhere later, a SQL editor in front would take ⌘↩ as Run (the keymap's clash test passes either way: it holds the prompt scope apart from the editor's, and this menu is not a modal prompt), and the row under the keyboard would take it as an Enter and switch.

The hint is `Item::hint`, "New window" and the key as the keymap writes it; in the terminal look the key leads, in the accent, as the boards draw it. Every row keeps its room, so the menu does not change width as the light moves. It is drawn on one row, the one the key goes to as the last frame lit it, even where the pointer lights another.

The keyboard on the chip (spec decision 9): opened by its key (quick open's Switch database, the Omarchy look's ctrl+shift+d, the start screen's `d`), the menu found the keyboard nowhere, and the arrows never reached its rows. Those three now open it through `open_database_menu_by_key`, which keeps where the keyboard was; the menu then gives its chip the keyboard once, so the arrows walk the rows as they do after Enter on the chip, and gives the keyboard back where it was when it closes (Esc, a pick). A click opens it as before and leaves the keyboard alone.

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/two_databases_tests.rs b/src/ui/two_databases_tests.rs
--- a/src/ui/two_databases_tests.rs
+++ b/src/ui/two_databases_tests.rs
@@ -1,13 +1,17 @@
-//! Two databases of one connection: a database opens in a new window
-//! beside this one, and leaves this one as it was.
-
+//! Two databases of one connection: the database menu opens a row's
+//! database in a new window beside this one, in both looks, and leaves
+//! this one as it was.
+
+use egui::accesskit::Role;
+use egui::{Key, Modifiers};
 use tabletist_db::{ConnectSpec, Driver, ObjectKind, ObjectRef};
 
 use crate::backend::{Command, Event};
 use crate::connections::{PasswordMode, SavedConnection};
 use crate::model::{Action, ConnTabId, Dialog, SessionStatus};
-use crate::testing::{Harness, answer_tree, bookshop_objects};
+use crate::testing::{Harness, answer_tree, bookshop_objects, bounds, labels};
 use crate::theme::Look;
+use crate::ui::workspace::database_menu_id;
 
 /// The Bookshop's development server, whose password is asked for on
 /// every connect.
@@ -220,3 +224,281 @@
     harness.app.save_session();
     assert_eq!(kept(&harness), books);
 }
+
+// The menu: what opens a new window from it, and what it says.
+
+/// Opens the bar's own chip's menu of `tab`, as quick open's Switch
+/// database asks for it.
+fn open_menu(harness: &mut Harness, tab: ConnTabId) {
+    harness.settle();
+    harness.app.workspace_mut(tab).unwrap().database_menu_asked = true;
+    harness.settle();
+    assert!(egui::Popup::is_id_open(
+        &harness.ctx,
+        database_menu_id(tab, false)
+    ));
+}
+
+/// Clicks the menu's row of `name` by the pointer, with `modifiers` held,
+/// as a window tells of them: before the pointer moves.
+fn click_row(harness: &mut Harness, name: &str, modifiers: Modifiers) {
+    let tree = harness.settle();
+    let row = bounds(&tree, name, Role::Button)
+        .unwrap_or_else(|| panic!("no row {name:?}: {:?}", labels(&tree)));
+    let pos = row.left_center() + egui::vec2(40.0, 0.0);
+    let press = |pressed| egui::Event::PointerButton {
+        pos,
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers,
+    };
+    harness.frame(vec![
+        egui::Event::ModifiersChanged(modifiers),
+        egui::Event::PointerMoved(pos),
+    ]);
+    harness.frame(vec![press(true)]);
+    harness.frame(vec![press(false)]);
+    harness.frame(vec![egui::Event::ModifiersChanged(Modifiers::NONE)]);
+    harness.settle();
+}
+
+/// The key of Open database in a new window, as `look` writes it.
+fn new_window_key(harness: &Harness, look: &Look) -> String {
+    let command = crate::keymap::Command::OpenDatabaseInNewWindow;
+    crate::ui::keys::written(&harness.ctx, look, command)
+        .as_str()
+        .to_owned()
+}
+
+/// Moves the keyboard down the open menu to the row of `name`.
+fn key_to(harness: &mut Harness, name: &str) {
+    for _ in 0..4 {
+        harness.press(Key::ArrowDown, Modifiers::NONE);
+        if crate::ui::tests::focused_name(&harness.settle()) == name {
+            return;
+        }
+    }
+    panic!("the keyboard never came to {name:?}");
+}
+#[test]
+fn cmd_click_opens_a_row_in_a_new_window_and_a_click_switches() {
+    let (mut harness, first) = bookshop(Look::macos());
+    open_menu(&mut harness, first);
+    click_row(&mut harness, "bookshop_test", Modifiers::COMMAND);
+    assert_eq!(windows(&harness).len(), 2);
+    assert_eq!(database(&harness, first), "bookshop_development");
+    let second = harness.app.active_tab_id();
+    assert_eq!(database(&harness, second), "bookshop_test");
+    assert!(
+        !egui::Popup::is_id_open(&harness.ctx, database_menu_id(first, false)),
+        "the menu closed"
+    );
+    // Back in the first window, a plain click switches it.
+    answer_connect(&mut harness);
+    harness.app.apply(Action::ActivateConnTab(first));
+    open_menu(&mut harness, first);
+    click_row(&mut harness, "postgres", Modifiers::NONE);
+    assert_eq!(windows(&harness).len(), 2, "no other window");
+    assert_eq!(database(&harness, first), "postgres");
+}
+
+#[test]
+fn cmd_enter_on_the_lit_row_opens_it_in_a_new_window() {
+    let (mut harness, first) = bookshop(Look::macos());
+    open_menu(&mut harness, first);
+    key_to(&mut harness, "bookshop_test");
+    // The lit row says what the key does, and no other row does.
+    assert!(harness.painted_rect("New window").is_some());
+    let key = new_window_key(&harness, &Look::macos());
+    assert!(!key.is_empty());
+    assert!(harness.painted_rect(&key).is_some());
+    harness.press(Key::Enter, Modifiers::COMMAND);
+    let second = harness.app.active_tab_id();
+    assert_ne!(second, first);
+    assert_eq!(database(&harness, second), "bookshop_test");
+    assert_eq!(database(&harness, first), "bookshop_development");
+    assert!(!egui::Popup::is_id_open(
+        &harness.ctx,
+        database_menu_id(first, false)
+    ));
+}
+
+#[test]
+fn enter_alone_on_the_lit_row_switches_this_window() {
+    let (mut harness, first) = bookshop(Look::macos());
+    open_menu(&mut harness, first);
+    key_to(&mut harness, "bookshop_test");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(windows(&harness), vec![first]);
+    assert_eq!(database(&harness, first), "bookshop_test");
+}
+
+#[test]
+fn the_hint_shows_on_the_lit_row_only() {
+    let (mut harness, first) = bookshop(Look::macos());
+    open_menu(&mut harness, first);
+    // Nothing lit yet: the pointer is off the menu, the keyboard on the
+    // chip.
+    assert!(harness.painted_rect("New window").is_none());
+    key_to(&mut harness, "bookshop_test");
+    let lit = harness.painted_rect("New window").unwrap();
+    let tree = harness.settle();
+    let row = bounds(&tree, "bookshop_test", Role::Button).unwrap();
+    assert!(row.contains(lit.center()), "on bookshop_test's row");
+}
+
+#[test]
+fn the_omarchy_look_opens_it_with_ctrl_enter_and_says_so() {
+    let (mut harness, first) = bookshop(Look::omarchy());
+    open_menu(&mut harness, first);
+    key_to(&mut harness, "bookshop_test");
+    assert!(harness.painted_rect("new window").is_some());
+    let key = new_window_key(&harness, &Look::omarchy());
+    assert!(!key.is_empty());
+    assert!(harness.painted_rect(&key).is_some());
+    harness.press(Key::Enter, Modifiers::CTRL);
+    let second = harness.app.active_tab_id();
+    assert_ne!(second, first);
+    assert_eq!(database(&harness, second), "bookshop_test");
+}
+
+#[test]
+fn the_start_screens_database_menu_opens_a_new_window_too() {
+    let (mut harness, first) = bookshop(Look::macos());
+    // The start screen shows with no tab open.
+    let id = harness.app.workspace(first).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::CloseTab { tab: first, id });
+    harness.click("Switch database");
+    assert!(egui::Popup::is_id_open(
+        &harness.ctx,
+        database_menu_id(first, true)
+    ));
+    click_row(&mut harness, "bookshop_test", Modifiers::COMMAND);
+    let second = harness.app.active_tab_id();
+    assert_ne!(second, first);
+    assert_eq!(database(&harness, second), "bookshop_test");
+}
+
+#[test]
+fn the_omarchy_key_opens_the_menu_with_the_keyboard_on_its_rows() {
+    let (mut harness, first) = bookshop(Look::omarchy());
+    harness.settle();
+    harness.press(Key::D, Modifiers::CTRL | Modifiers::SHIFT);
+    assert!(egui::Popup::is_id_open(
+        &harness.ctx,
+        database_menu_id(first, false)
+    ));
+    key_to(&mut harness, "bookshop_test");
+    harness.press(Key::Enter, Modifiers::CTRL);
+    let second = harness.app.active_tab_id();
+    assert_ne!(second, first);
+    assert_eq!(database(&harness, second), "bookshop_test");
+}
+
+/// Sends `select 1` to a new SQL editor of `tab`, in front, and returns it.
+fn with_sql_editor(harness: &mut Harness, tab: ConnTabId) -> crate::model::TabId {
+    harness.app.apply(Action::NewSqlTab(tab));
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    let sql = workspace.sql_tab_mut(id).unwrap();
+    sql.text = "select 1".into();
+    sql.cursor = 0;
+    harness.settle();
+    id
+}
+
+#[test]
+fn the_key_opens_a_window_and_runs_no_sql_in_front() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let modifiers = if look.terminal {
+            Modifiers::CTRL
+        } else {
+            Modifiers::COMMAND
+        };
+        let (mut harness, first) = bookshop(look);
+        with_sql_editor(&mut harness, first);
+        let sent = harness.app.backend.sent.len();
+        open_menu(&mut harness, first);
+        key_to(&mut harness, "bookshop_test");
+        harness.press(Key::Enter, modifiers);
+        let runs = harness.app.backend.sent[sent..]
+            .iter()
+            .filter(|command| matches!(command, Command::RunSql { .. }))
+            .count();
+        assert_eq!(runs, 0, "the SQL ran");
+        assert_eq!(windows(&harness).len(), 2, "no new window");
+    }
+}
+
+#[test]
+fn a_click_on_the_chip_leaves_the_keyboard_where_it_was() {
+    let (mut harness, first) = bookshop(Look::macos());
+    harness.settle();
+    let before = harness.ctx.memory(|memory| memory.focused());
+    let chip = harness.painted_rect("bookshop_development").unwrap();
+    let press = |pressed| egui::Event::PointerButton {
+        pos: chip.center(),
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![egui::Event::PointerMoved(chip.center())]);
+    harness.frame(vec![press(true)]);
+    harness.frame(vec![press(false)]);
+    harness.settle();
+    assert!(egui::Popup::is_id_open(
+        &harness.ctx,
+        database_menu_id(first, false)
+    ));
+    assert_eq!(harness.ctx.memory(|memory| memory.focused()), before);
+}
+
+#[test]
+fn esc_gives_the_keyboard_back_where_it_was() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let terminal = look.terminal;
+        let (mut harness, first) = bookshop(look);
+        with_sql_editor(&mut harness, first);
+        if terminal {
+            harness.start_typing();
+        }
+        harness.settle();
+        let before = harness.ctx.memory(|memory| memory.focused());
+        assert!(before.is_some(), "the editor has the keyboard");
+        open_menu(&mut harness, first);
+        harness.press(Key::Escape, Modifiers::NONE);
+        assert!(!egui::Popup::is_id_open(
+            &harness.ctx,
+            database_menu_id(first, false)
+        ));
+        assert_eq!(
+            harness.ctx.memory(|memory| memory.focused()),
+            before,
+            "terminal: {terminal}"
+        );
+    }
+}
+
+#[test]
+fn the_hint_is_on_the_keyboards_row_when_the_pointer_is_on_another() {
+    let (mut harness, first) = bookshop(Look::macos());
+    open_menu(&mut harness, first);
+    key_to(&mut harness, "bookshop_test");
+    let tree = harness.settle();
+    let keyboard = bounds(&tree, "bookshop_test", Role::Button).unwrap();
+    let pointer = bounds(&tree, "postgres", Role::Button).unwrap();
+    harness.frame(vec![egui::Event::PointerMoved(pointer.center())]);
+    harness.settle();
+    let hints: Vec<egui::Rect> = harness
+        .text_rects
+        .iter()
+        .filter(|(text, _)| text == "New window")
+        .map(|(_, rect)| *rect)
+        .collect();
+    assert_eq!(hints.len(), 1, "{hints:?}");
+    assert!(keyboard.contains(hints[0].center()));
+    // And the key goes where the hint is.
+    harness.press(Key::Enter, Modifiers::COMMAND);
+    let second = harness.app.active_tab_id();
+    assert_eq!(database(&harness, second), "bookshop_test");
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib two_databases`

Expected: task 1's four pass, and so does the test that a click leaves the keyboard alone, which held before; the rest of the menu's fail, the click switching, the keyboard never reaching a row, the SQL running:

```
test ui::two_databases_tests::the_start_screens_database_menu_opens_a_new_window_too ... FAILED
test ui::two_databases_tests::cmd_click_opens_a_row_in_a_new_window_and_a_click_switches ... FAILED
test ui::two_databases_tests::esc_gives_the_keyboard_back_where_it_was ... FAILED
test ui::two_databases_tests::the_hint_shows_on_the_lit_row_only ... FAILED
test ui::two_databases_tests::the_omarchy_look_opens_it_with_ctrl_enter_and_says_so ... FAILED
test ui::two_databases_tests::the_omarchy_key_opens_the_menu_with_the_keyboard_on_its_rows ... FAILED
test ui::two_databases_tests::the_key_opens_a_window_and_runs_no_sql_in_front ... FAILED
test ui::two_databases_tests::cmd_enter_on_the_lit_row_opens_it_in_a_new_window ... FAILED
test ui::two_databases_tests::enter_alone_on_the_lit_row_switches_this_window ... FAILED
test ui::two_databases_tests::the_hint_is_on_the_keyboards_row_when_the_pointer_is_on_another ... FAILED
test result: FAILED. 5 passed; 10 failed; 0 ignored; 0 measured; 3058 filtered out; finished in …
```

- [ ] **Step 3: Write the code**

```diff
diff --git a/assets/i18n/pt-BR.po b/assets/i18n/pt-BR.po
--- a/assets/i18n/pt-BR.po
+++ b/assets/i18n/pt-BR.po
@@ -7,7 +7,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-21 10:36-0500\n"
+"POT-Creation-Date: 2026-11-21 09:54-0500\n"
 "PO-Revision-Date: 2026-11-19 16:11-0500\n"
 "Last-Translator: Automatically generated\n"
 "Language-Team: none\n"
@@ -10395,6 +10395,10 @@
 msgstr "Login recusado"
 
 #: src/ui/workspace.rs
+msgid "New window"
+msgstr "Nova janela"
+
+#: src/ui/workspace.rs
 msgid "Not encrypted"
 msgstr "Sem criptografia"
 
diff --git a/assets/i18n/ru.po b/assets/i18n/ru.po
--- a/assets/i18n/ru.po
+++ b/assets/i18n/ru.po
@@ -7,7 +7,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-21 10:36-0500\n"
+"POT-Creation-Date: 2026-11-21 09:54-0500\n"
 "PO-Revision-Date: 2026-11-17 22:38-0500\n"
 "Last-Translator: Automatically generated\n"
 "Language-Team: none\n"
@@ -10487,6 +10487,10 @@
 msgstr "Вход отклонён"
 
 #: src/ui/workspace.rs
+msgid "New window"
+msgstr "Новое окно"
+
+#: src/ui/workspace.rs
 msgid "Not encrypted"
 msgstr "Без шифрования"
 
diff --git a/assets/i18n/tabletist.pot b/assets/i18n/tabletist.pot
--- a/assets/i18n/tabletist.pot
+++ b/assets/i18n/tabletist.pot
@@ -8,7 +8,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-21 10:36-0500\n"
+"POT-Creation-Date: 2026-11-21 09:54-0500\n"
 "PO-Revision-Date: YEAR-MO-DA HO:MI+ZONE\n"
 "Last-Translator: FULL NAME <EMAIL@ADDRESS>\n"
 "Language-Team: LANGUAGE <LL@li.org>\n"
@@ -10196,6 +10196,10 @@
 msgstr ""
 
 #: src/ui/workspace.rs
+msgid "New window"
+msgstr ""
+
+#: src/ui/workspace.rs
 msgid "Disconnect"
 msgstr ""
 
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -718,6 +718,13 @@
     let mut menu_asked = false;
     // The DDL view's text was asked to be copied, by its chord.
     let mut copy_ddl = None;
+    // The open database menu's key, for the menu: read first, it is no SQL
+    // editor's Run, nor any other chord it shares.
+    let database_menu = [false, true]
+        .map(|start| crate::ui::workspace::database_menu_id(active, start))
+        .into_iter()
+        .find(|id| egui::Popup::is_id_open(ctx, *id));
+    let mut new_window = false;
     ctx.input_mut(|input| {
         // The keymap says which key: each line asks whether its command's
         // chord went down, in the scope it is read in. The guards say when
@@ -726,6 +733,8 @@
         let on = |input: &mut egui::InputState, command: Command, scope: Scope| {
             asked(input, keymap, layout, command, scope, normal)
         };
+        new_window = database_menu.is_some()
+            && on(input, Command::OpenDatabaseInNewWindow, Scope::Prompt).fresh;
         // The completion list's keys come first: the editor never sees
         // them, nor do the chords below.
         if let Some(editor) = in_editor {
@@ -1258,7 +1267,10 @@
     // database has none, and the key does nothing.
     if menu_asked {
         let id = crate::ui::workspace::database_menu_id(active, false);
-        egui::Popup::open_id(ctx, id);
+        crate::ui::workspace::open_database_menu_by_key(ctx, id);
+    }
+    if let Some(id) = database_menu.filter(|_| new_window) {
+        crate::ui::workspace::ask_new_window(ctx, id);
     }
     // The tab at the place its digit named.
     if let Some(index) = tab_asked
@@ -3307,7 +3319,7 @@
             }
             (Some(Command::SwitchDatabase), _) if start && switchable(workspace) => {
                 let id = crate::ui::workspace::database_menu_id(tab, true);
-                egui::Popup::open_id(ctx, id);
+                crate::ui::workspace::open_database_menu_by_key(ctx, id);
             }
             (Some(Command::StartSearchFocus), _) if start => {
                 actions.push(Action::StartSearchFocus(tab));
diff --git a/src/ui/menu.rs b/src/ui/menu.rs
--- a/src/ui/menu.rs
+++ b/src/ui/menu.rs
@@ -106,6 +106,7 @@
     current: Option<bool>,
     role: Option<TextRole>,
     detail: Option<&'a str>,
+    hint: Option<(&'a str, &'a str)>,
     disabled: Option<&'a str>,
     note: Option<&'a str>,
     lit: Option<bool>,
@@ -120,6 +121,7 @@
             current: None,
             role: None,
             detail: None,
+            hint: None,
             disabled: None,
             note: None,
             lit: None,
@@ -149,6 +151,16 @@
     /// A muted note at the row's right.
     pub fn detail(mut self, detail: &'a str) -> Self {
         self.detail = Some(detail);
+        self
+    }
+
+    /// What a key does on the row, at its right while the row is lit:
+    /// `words` and the `key` that does it, as the look writes them
+    /// (`New window ⌘↩`, the terminal's `ctrl+enter new window` with its
+    /// key in the accent). The room is kept on every row, so the menu
+    /// keeps its width as the light moves.
+    pub fn hint(mut self, words: &'a str, key: &'a str) -> Self {
+        self.hint = Some((words, key));
         self
     }
 
@@ -202,11 +214,14 @@
         let noted = self
             .detail
             .map_or(0.0, |detail| DETAIL_GAP + measure(note, detail));
+        let hinted = self.hint.map_or(0.0, |(words, key)| {
+            DETAIL_GAP + measure(note, words) + shape.gap + measure(note, key)
+        });
         let own = measure(role, self.text);
         let after = self
             .note
             .map_or(0.0, |words| shape.gap + measure(note, words));
-        let wanted = lead + own + after + noted + shape.inset;
+        let wanted = lead + own + after + noted + hinted + shape.inset;
         // A row that cannot be picked answers no press, and still takes
         // the keyboard, as a button that cannot be pressed does: the
         // keyboard can then read why. In a menu the keyboard never comes
@@ -229,6 +244,12 @@
         if let Some(detail) = self.detail {
             ui.ctx().accesskit_node_builder(response.id, |node| {
                 node.set_description(detail);
+            });
+        }
+        if let Some((words, key)) = self.hint {
+            let said = format!("{words} {key}");
+            ui.ctx().accesskit_node_builder(response.id, |node| {
+                node.set_description(said);
             });
         }
         if let Some(reason) = self.disabled {
@@ -297,6 +318,25 @@
             Text::one(look, note, detail, palette.dim)
                 .layout(ui.ctx())
                 .paint_right(painter, rect.right() - shape.inset, center);
+        }
+        if let Some((words, key)) = self.hint.filter(|_| lit) {
+            let right = rect.right() - shape.inset;
+            if look.terminal {
+                // The key leads, in the accent, as the footers write keys.
+                let words = Text::one(look, note, words, palette.dim).layout(ui.ctx());
+                let width = words.width();
+                words.paint_right(painter, right, center);
+                Text::one(look, note, key, palette.accent)
+                    .layout(ui.ctx())
+                    .paint_right(painter, right - width - shape.gap, center);
+            } else {
+                let key = Text::one(look, note, key, palette.dim).layout(ui.ctx());
+                let width = key.width();
+                key.paint_right(painter, right, center);
+                Text::one(look, note, words, palette.dim)
+                    .layout(ui.ctx())
+                    .paint_right(painter, right - width - shape.gap, center);
+            }
         }
         match self.disabled {
             Some(reason) => {
diff --git a/src/ui/start.rs b/src/ui/start.rs
--- a/src/ui/start.rs
+++ b/src/ui/start.rs
@@ -883,7 +883,7 @@
                 id,
                 skin.tab,
                 &shown.info,
-                (look, palette),
+                (look, palette, locale),
                 actions,
             );
         }
@@ -1000,7 +1000,7 @@
                     id,
                     skin.tab,
                     &shown.info,
-                    (look, palette),
+                    (look, palette, locale),
                     actions,
                 );
             }
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -1261,28 +1261,119 @@
     egui::Id::new(("database-menu", tab.0, start))
 }
 
+/// Where the keyboard was when a key opened the database menu, and
+/// whether the menu has given its chip the keyboard yet.
+#[derive(Clone, Copy)]
+struct OpenedByKey {
+    from: Option<egui::Id>,
+    given: bool,
+}
+
+/// Opens the database menu `id` as a key does (Switch database's, quick
+/// open's): its chip takes the keyboard, so that the arrows go down the
+/// rows as they do after Enter on it, and the keyboard goes back where it
+/// was when the menu closes.
+pub fn open_database_menu_by_key(ctx: &egui::Context, id: egui::Id) {
+    egui::Popup::open_id(ctx, id);
+    let from = ctx.memory(|memory| memory.focused());
+    ctx.data_mut(|data| data.insert_temp(id.with("by key"), OpenedByKey { from, given: false }));
+}
+
+/// The key of Open database in a new window went down while the menu `id`
+/// was open: the menu takes it on the row it lights. Read before anything
+/// else reads the keys, so that no other chord of it (a SQL editor's Run)
+/// takes it.
+pub fn ask_new_window(ctx: &egui::Context, id: egui::Id) {
+    ctx.data_mut(|data| data.insert_temp(id.with("new window"), true));
+}
+
 /// The pop-up of the server's other databases, under the bar's own chip
-/// or the start screen's button (`response`), with `id` its own.
+/// or the start screen's button (`response`), with `id` its own. A click
+/// or Enter switches `tab` to the row's database; with Cmd (Ctrl off
+/// macOS), or the key of Open database in a new window, it opens in a new
+/// window beside `tab`, which the lit row's hint says.
 pub(super) fn database_menu(
     response: &egui::Response,
     id: egui::Id,
     tab: ConnTabId,
     info: &BarInfo,
-    (look, palette): (&Look, &Palette),
+    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
     actions: &mut Vec<Action>,
 ) {
     let role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary);
+    let key = crate::ui::keys::written(&response.ctx, look, Command::OpenDatabaseInNewWindow);
+    let words = look.label(&gettext(locale, "New window"));
+    let ctx = &response.ctx;
+    // Opened by a key, the menu gives its chip the keyboard once, and the
+    // keyboard goes back where it was as the menu closes.
+    let by_key = id.with("by key");
+    if let Some(opened) = ctx.data(|data| data.get_temp::<OpenedByKey>(by_key)) {
+        if !egui::Popup::is_id_open(ctx, id) {
+            ctx.data_mut(|data| data.remove::<OpenedByKey>(by_key));
+            if let Some(from) = opened.from {
+                ctx.memory_mut(|memory| memory.request_focus(from));
+            }
+        } else if !opened.given {
+            response.request_focus();
+            let given = OpenedByKey {
+                given: true,
+                ..opened
+            };
+            ctx.data_mut(|data| data.insert_temp(by_key, given));
+        }
+    }
+    // The row the key goes to, as the last frame lit it: the hint is on it
+    // alone.
+    let lit_id = id.with("lit");
+    let was_lit = ctx.data(|data| data.get_temp::<String>(lit_id));
     menu::under_id(response, id, response.rect.width(), look, palette, |ui| {
+        let asked = ui
+            .ctx()
+            .data_mut(|data| data.remove_temp::<bool>(id.with("new window")))
+            .unwrap_or(false);
+        let mut lit = None;
         for database in &info.databases {
             // Database names come from the server: nothing hidden.
             let name = display_safe(database);
-            let item = menu::Item::choice(&name, *database == info.database).role(role);
-            if item.show(ui, look, palette).clicked() && *database != info.database {
+            let mut item = menu::Item::choice(&name, *database == info.database).role(role);
+            if !key.is_empty() && was_lit.as_ref() == Some(database) {
+                item = item.hint(&words, &key);
+            }
+            let row = item.show(ui, look, palette);
+            let new_window = row.clicked() && ui.input(|input| input.modifiers.command);
+            if new_window {
+                actions.push(Action::OpenDatabaseInNewWindow {
+                    tab,
+                    database: database.clone(),
+                });
+                ui.close();
+            } else if row.clicked() && *database != info.database {
                 actions.push(Action::SwitchDatabase {
                     tab,
                     database: database.clone(),
                 });
             }
+            // The keyboard's row, or else the pointer's.
+            if row.has_focus() || (lit.is_none() && row.hovered()) {
+                lit = Some(database);
+            }
+        }
+        if lit != was_lit.as_ref() {
+            let lit = lit.cloned();
+            ui.ctx().data_mut(|data| match lit {
+                Some(lit) => {
+                    data.insert_temp(lit_id, lit);
+                }
+                None => data.remove::<String>(lit_id),
+            });
+            ui.ctx().request_repaint();
+        }
+        if let Some(database) = lit.filter(|_| asked) {
+            actions.push(Action::OpenDatabaseInNewWindow {
+                tab,
+                database: database.clone(),
+            });
+            ui.close();
         }
     });
 }
@@ -1304,7 +1395,7 @@
     // Quick open's Switch database asks for the chip's menu, as its key
     // does: the menu opens as the chip is drawn.
     if std::mem::take(&mut workspace.database_menu_asked) {
-        egui::Popup::open_id(ui.ctx(), database_menu_id(tab, false));
+        open_database_menu_by_key(ui.ctx(), database_menu_id(tab, false));
     }
     let super::Paint {
         locale,
@@ -1615,7 +1706,7 @@
                     Rect::from_center_size(pos2(rect.right() - 16.0, center), vec2(12.0, 12.0)),
                 );
                 let id = database_menu_id(tab, false);
-                database_menu(&response, id, tab, info, (look, palette), actions);
+                database_menu(&response, id, tab, info, (look, palette, locale), actions);
             }
         } else if response.clicked() {
             actions.push(Action::ActivateConnTab(chip.tab));
@@ -1884,7 +1975,7 @@
                     Text::one(look, TextRole::OBody, "▾", palette.dim),
                 );
                 let id = database_menu_id(tab, false);
-                database_menu(&response, id, tab, info, (look, palette), actions);
+                database_menu(&response, id, tab, info, (look, palette, locale), actions);
             }
         } else if response.clicked() {
             actions.push(Action::ActivateConnTab(chip.tab));
```

Run `assets/i18n/update.sh` and give "New window" its translations as the diff does.

- [ ] **Step 4: Run the tests, then the checks**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib two_databases`

Expected: `test result: ok. 15 passed`. Then every check under Checks passes; the app's library ends `test result: ok. 3057 passed; 0 failed; 16 ignored; 0 measured; 0 filtered out; finished in …`.

- [ ] **Step 5: Look at it**

By eye, in a throwaway scene or the app on the demo database (never committed, never a test): the hint in both looks, light and dark, on the lit row only, and the menu's width steady as the keyboard moves. Compare with the two boards.

- [ ] **Step 6: Commit**

```bash
git add src/ui/two_databases_tests.rs src/ui/menu.rs src/ui/workspace.rs src/ui/keys.rs src/ui/start.rs assets/i18n
git commit -m "Open the database menu's row in a new window with Cmd, and say so on the row (#366)"
```

---

### Task 3: The docs

**Files:**
- Modify: `docs/_guide/getting-started.md`, `docs/_guide/omarchy.md`, `docs/_reference/connections.md`, `docs/_reference/keyboard-shortcuts.md`

- [ ] **Step 1: Write them**

```diff
diff --git a/docs/_guide/getting-started.md b/docs/_guide/getting-started.md
--- a/docs/_guide/getting-started.md
+++ b/docs/_guide/getting-started.md
@@ -56,7 +56,9 @@
 
 - **The header** holds a chip for each open connection. Click one to
   switch, or press Cmd/Ctrl+1 to 9 (Ctrl+Shift+1 to 9 in the Omarchy
-  look).
+  look). The chip of the connection you are in lists the server's other
+  databases: pick one to switch to it, or Cmd/Ctrl-click it to open it in
+  a new window beside this one.
 - **The sidebar** lists the tables and views of one schema. On macOS and
   Windows the ones you opened recently are on top. Cmd/Ctrl+B shows or
   hides it.
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -37,7 +37,7 @@
 | Ctrl+Shift+1 to 9, Ctrl+Tab, Ctrl+Shift+Tab | Switch connection |
 | Ctrl+Shift+W | Close the connection |
 | Ctrl+T | New SQL editor |
-| Ctrl+Shift+D | Switch database, where the server has another |
+| Ctrl+Shift+D | Switch database, where the server has another. In its menu Ctrl+Enter opens the database in a new window instead |
 | Alt+1 to 9 | Go to that tab |
 | Ctrl+Shift+[ and ] | Previous and next tab |
 | Ctrl+W | Close the tab. While you type in a field it deletes a word instead; in the SQL editor's normal mode it does neither |
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -30,6 +30,20 @@
 be undone: the
 connection goes with its saved password and SSH secret, its labels, and
 what was last open on it.
+
+## Another database of the server
+
+The chip of the connection you are in (the start screen's **Switch
+database** and quick open's open the same menu) lists the server's other
+databases, where it has more than one. Picking one switches the window to
+it: the tables close, and the SQL editors stay. Cmd-click or Cmd+Enter
+(Ctrl on Windows and Linux, Ctrl+Enter in the Omarchy look) opens it in a
+new window instead, right after this one and in front, with a session of
+its own. The row under the pointer or the keyboard names the key. The new
+window connects with the password this one connected with, so nothing is
+asked again, and this one keeps its tables, editors and edits. Do it on
+the database you are on for a second window on it. The chips tell the
+windows apart by their database.
 
 ## Fields
 
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -73,6 +73,13 @@
 | Mod+E | Edit |
 | Mod+D | Duplicate |
 | Mod+Backspace | Delete, once you say so. Esc keeps the connection |
+
+In the menu of the server's databases:
+
+| Keys | Do |
+| --- | --- |
+| Arrows, then Enter | Switch this window to the database |
+| Mod+Enter | Open the database in a new window |
 
 In the connection dialog:
 
```

- [ ] **Step 2: Check the site**

Run: `cd docs && bundle install && bin/check`

Expected: the site builds and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_guide docs/_reference
git commit -m "Describe opening another database in a new window (#366)"
```

---

## After the run

- Open the PR for the three commits, with "Part of #366" (runs 2 and 3 follow).
- Runs 2 (relaunch: each window as it was) and 3 (picker: "Open another") get plans of their own.
