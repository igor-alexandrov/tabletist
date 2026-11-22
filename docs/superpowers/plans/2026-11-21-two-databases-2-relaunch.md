# Two databases of one connection, run 2: a relaunch reopens each window as it was: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** At launch (Restore last session, or a relaunch to update), each connection window opens again on the database it was on, with its own tables and row panel, so two windows of one connection come back as they were at quit. (igor-alexandrov/tabletist#366, run 2 of 3.)

**Architecture:** Three tasks, one PR, a commit each:
1. **session.json `windows`:** beside `open`, each window's database, tables and row panel (a `Kept`) with its connection's id, read only where each entry is of the window at its place in `open`.
2. **The launch:** `App::save_session` writes them, and `App::open_last_session` reopens each window from its own, through `open_workspace`'s new `Reopen` (nothing, the connection's, or this window's).
3. **The docs:** the reference's Settings and files and Connections pages.

**Tech Stack:** Rust, serde and serde_json in `crates/tabletist-core/src/session.rs`, the app's state in `src/app.rs`, its startup tests in `src/app/startup_tests.rs`.

**Spec:** `docs/superpowers/specs/2026-11-21-two-databases-design.md`: decision 3 (each window as it was), decision 10 (its database comes back whatever Restore tabs and filters says), and the Relaunch section.

**Base:** run 1's branch, `claude/two-databases-366-8b21f3`, at 1ffac986 (PR #374, not yet merged): this run calls `open_workspace` as run 1 left it and opens its second window with run 1's `Action::OpenDatabaseInNewWindow`. Work on `claude/two-databases-366-run2`. Once #374 is merged, rebase onto main (never a merge of main).

---

## What this run builds

| The spec says | This run builds |
|---|---|
| session.json keeps `open` and gains `windows`, the same length: per window its database and its kept tables (`Kept`) | `Session::windows: Vec<Option<Kept>>`, beside `open`; each written as its `Kept` with `"connection": "<id>"` added, the picker as `null` |
| An older version reads `open` and ignores `windows` | It keeps `windows` among the keys it does not know (`Session::rest`) and writes them back |
| `windows` read only where each entry's connection is `open`'s at the same place, else dropped | `windows_of`: all of them dropped where they are not as many as `open`, where an entry names another connection, or where the picker has one; an entry that does not read costs only itself |
| `connections` (per connection) stays | Unchanged: the first window in the bar on the connection's own database keeps its tables there, for an open from the picker |
| Each window reopens on its own database with its own tables | `Reopen::Window`: the database through `session::put_on`, the tables where the settings reopen them (or on a relaunch to update) |
| Decision 10: the database comes back whatever Restore tabs and filters says | `put_on` runs whatever the setting; only the tables follow it |
| A relaunch to update is per window already | Unchanged: it calls `open_last_session` too, so its windows get their databases as well |

No board changes: nothing on screen is new. The canvas's boards (`MacTwoDatabases`, `OmarchyTwoDatabases`) stay as run 1 left them.

## Files

| File | Task | What changes |
|---|---|---|
| `crates/tabletist-core/src/session.rs` | 1, 2 | `Session::windows`, the file's `windows`, `windows_of`, `Session::windows_written` (1); `put_on` (2) |
| `src/app.rs` | 2 | `enum Reopen`; `open_workspace` takes it; `open_last_session` and `save_session` |
| `src/app/startup_tests.rs` | 2 | The launch's tests |
| `docs/_reference/settings-and-files.md`, `docs/_reference/connections.md` | 3 | The docs |

## Checks

Each task ends with these, all passing:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
assets/i18n/update.sh --check
```

Use `~/.cargo/bin/cargo`: the mise shim fails here. Keep `CARGO_TARGET_DIR` unset (the worktree's `target/`), never under `/tmp`. This run adds no message, so the catalogs do not change.

---

### Task 1: session.json `windows`

**Files:**
- Modify: `crates/tabletist-core/src/session.rs`

What it holds: `Session::windows` is beside `open`, the same length where it holds anything: each window's `Kept` (database, tables, the one in front, row panel), `None` for the picker and for a window the file had nothing of. It is empty where the file had nothing of any window, so that a session with none reads back as it was written, and nothing of `windows` is written then.

In the file each entry is the window's `Kept` with `"connection"` added, or `null`. They are read only where each is of the window at its place in `open`. An older version (Settings › Updates' Go back) does not know `windows`: it keeps them among the keys it does not know and writes them back beside an `open` of its own, which can be another list. Where they are not as many, where one names another connection, or where the picker has one, none is read, and the launch reopens the windows as today. An entry that does not read (a newer version's, or one changed by hand) costs only itself, as a connection's part in `connections` does.

`keys_a_newer_version_added_are_written_back` used `windows` as its example of a key this version does not know. It now uses `layout`, since `windows` is read here.

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/crates/tabletist-core/src/session.rs b/crates/tabletist-core/src/session.rs
--- a/crates/tabletist-core/src/session.rs
+++ b/crates/tabletist-core/src/session.rs
@@ -512,6 +512,133 @@
         assert_eq!(tab["filter"]["rows"][0][1], "eq");
         // What is as a new tab has it is left out.
         assert!(tab.get("pinned").is_none(), "{text}");
+    }
+
+    /// Bookshop's window on `database`, with `tabs` and the row panel
+    /// hidden.
+    fn window(database: &str, tabs: Vec<KeptTab>) -> Kept {
+        Kept {
+            database: database.into(),
+            active: (!tabs.is_empty()).then_some(0),
+            tabs,
+            row_panel: false,
+            rest: BTreeMap::new(),
+        }
+    }
+
+    #[test]
+    fn each_window_is_written_and_read_back_with_its_database_and_tables() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("session.json");
+        let bookshop = ConnectionId("6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11".into());
+        let authors = KeptTab {
+            name: "authors".into(),
+            ..books()
+        };
+        let session = Session {
+            open: vec![Some(bookshop.clone()), Some(bookshop.clone()), None],
+            active: 1,
+            windows: vec![
+                Some(window("bookshop_development", vec![books()])),
+                Some(window("bookshop_test", vec![authors])),
+                None,
+            ],
+            ..Session::default()
+        };
+        session.save(&path).unwrap();
+        assert_eq!(Session::load(&path), session);
+        let text = std::fs::read_to_string(&path).unwrap();
+        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
+        let windows = &json["windows"];
+        assert_eq!(windows[0]["connection"], bookshop.0.as_str());
+        assert_eq!(windows[1]["database"], "bookshop_test");
+        assert_eq!(windows[1]["tabs"][0]["name"], "authors");
+        assert_eq!(windows[2], serde_json::Value::Null);
+        // A session with nothing of its windows writes none.
+        let session = Session {
+            open: vec![Some(bookshop), None],
+            ..Session::default()
+        };
+        session.save(&path).unwrap();
+        let text = std::fs::read_to_string(&path).unwrap();
+        assert!(!text.contains("windows"), "{text}");
+        assert_eq!(Session::load(&path), session);
+    }
+
+    #[test]
+    fn a_file_from_before_the_windows_were_kept_has_nothing_of_them() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("session.json");
+        std::fs::write(
+            &path,
+            r#"{"version":1,"open":["a",null],"active":0,"connections":{
+                "a":{"database":"bookshop","tabs":[{"schema":"public","name":"books","kind":"table"}]}}}"#,
+        )
+        .unwrap();
+        let session = Session::load(&path);
+        assert_eq!(session.open, vec![Some(ConnectionId("a".into())), None]);
+        assert!(session.windows.is_empty());
+        assert_eq!(session.connections["a"].tabs[0].name, "books");
+    }
+
+    #[test]
+    fn windows_that_are_not_the_open_ones_are_left_out() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("session.json");
+        let window =
+            serde_json::json!({"connection": "a", "database": "bookshop_test", "tabs": []});
+        for (open, windows) in [
+            // An older version wrote back the windows with its own open.
+            (
+                serde_json::json!(["b"]),
+                serde_json::json!([window.clone()]),
+            ),
+            // As many as there were then.
+            (
+                serde_json::json!(["a", null]),
+                serde_json::json!([window.clone()]),
+            ),
+            // The picker where a window was.
+            (
+                serde_json::json!([null]),
+                serde_json::json!([window.clone()]),
+            ),
+            // Not a list.
+            (serde_json::json!(["a"]), serde_json::json!({"width": 1280})),
+        ] {
+            let text = serde_json::json!({"version": 1, "open": open, "windows": windows});
+            std::fs::write(&path, text.to_string()).unwrap();
+            let session = Session::load(&path);
+            assert!(session.windows.is_empty(), "{text}");
+            assert!(!session.open.is_empty(), "{text}");
+        }
+    }
+
+    #[test]
+    fn a_window_that_does_not_read_costs_only_itself() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("session.json");
+        let text = serde_json::json!({
+            "version": 1,
+            "open": [7, "a", "b", "c"],
+            "windows": [
+                null,
+                {"connection": "a", "tabs": "not a list"},
+                {"connection": "b", "database": "bookshop_test", "tabs": []},
+                null,
+            ],
+        });
+        std::fs::write(&path, text.to_string()).unwrap();
+        let session = Session::load(&path);
+        // Each beside its window, the one of the wrong kind left out.
+        assert_eq!(session.open.len(), 3);
+        assert_eq!(session.windows.len(), 3);
+        assert_eq!(session.windows[0], None);
+        assert_eq!(
+            session.windows[1].as_ref().unwrap().database,
+            "bookshop_test"
+        );
+        assert_eq!(session.windows[2], None);
     }
 
     #[test]
@@ -688,7 +815,7 @@
         let path = dir.path().join("session.json");
         let text = serde_json::json!({
             "version": 2,
-            "windows": {"width": 1280},
+            "layout": {"width": 1280},
             "open": ["a"],
             "connections": {"a": {
                 "database": "",
@@ -703,7 +830,7 @@
         session.save(&path).unwrap();
         let written: serde_json::Value =
             serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
-        assert_eq!(written["windows"]["width"], 1280);
+        assert_eq!(written["layout"]["width"], 1280);
         let kept = &written["connections"]["a"];
         assert_eq!(kept["pane"], "grid");
         assert_eq!(kept["tabs"][0]["hidden"][0], "price");
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-core --lib session::`

Expected: the test build fails, `Session` has no `windows` yet:

```
error[E0560]: struct `session::Session` has no field named `windows`
error[E0609]: no field `windows` on type `session::Session`
```

- [ ] **Step 3: Write the code**

```diff
diff --git a/crates/tabletist-core/src/session.rs b/crates/tabletist-core/src/session.rs
--- a/crates/tabletist-core/src/session.rs
+++ b/crates/tabletist-core/src/session.rs
@@ -1,6 +1,7 @@
-//! What was open when the app last quit, and each connection's tables as
-//! they were when its window last closed: `session.json`, in the state
-//! directory. Of this computer, and no setting: what the next launch and
+//! What was open when the app last quit, each window with its database
+//! and its tables, and each connection's tables as they were when its
+//! window last closed: `session.json`, in the state directory. Of this
+//! computer, and no setting: what the next launch and
 //! the next open of a connection make of it is the settings' (`startup`).
 
 use std::collections::BTreeMap;
@@ -23,6 +24,11 @@
     pub open: Vec<Option<ConnectionId>>,
     /// Which of `open` was in front.
     pub active: usize,
+    /// Each of `open` as it was at quit, at its place: its database, its
+    /// tables and its row panel. `None` for the picker and for a window
+    /// the file had nothing of; empty where it had nothing of any (a file
+    /// from before, or one whose `windows` are not its `open`'s).
+    pub windows: Vec<Option<Kept>>,
     /// Each saved connection's table tabs as its window last had them, by
     /// its id. Changed through [`Session::keep`] and [`Session::forget`].
     pub connections: BTreeMap<String, Kept>,
@@ -163,6 +169,11 @@
     version: serde_json::Value,
     open: Vec<serde_json::Value>,
     active: serde_json::Value,
+    /// Beside `open`, each window's database and tables, with its
+    /// connection: an older version that does not know them writes them
+    /// back with an `open` of its own.
+    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
+    windows: serde_json::Value,
     connections: BTreeMap<String, serde_json::Value>,
     #[serde(flatten)]
     rest: BTreeMap<String, serde_json::Value>,
@@ -184,6 +195,7 @@
                 }
             }
         }
+        let places = windows_of(&file.open, &file.windows);
         // A window that is neither an id nor the picker is left out.
         let mut active = None;
         let wanted = file.active.as_u64();
@@ -197,6 +209,10 @@
                 active = Some(session.open.len());
             }
             session.open.push(window);
+            session.windows.push(places.get(at).cloned().flatten());
+        }
+        if session.windows.iter().all(Option::is_none) {
+            session.windows.clear();
         }
         // Counted against the windows read: one that pointed at a window
         // left out, or past the end, is the first.
@@ -269,11 +285,70 @@
                 })
                 .collect(),
             active: self.active.into(),
+            windows: self.windows_written(),
             connections,
             rest: self.rest.clone(),
         };
         crate::util::save_json(path, &file)
     }
+
+    /// `windows` as the file has them, each beside its window in `open`
+    /// with its connection's id: none where nothing is held of any.
+    fn windows_written(&self) -> serde_json::Value {
+        if self.windows.iter().all(Option::is_none) {
+            return serde_json::Value::Null;
+        }
+        let written = self.open.iter().enumerate().map(|(at, id)| {
+            let kept = self.windows.get(at).and_then(Option::as_ref);
+            let (Some(id), Some(kept)) = (id, kept) else {
+                return serde_json::Value::Null;
+            };
+            match serde_json::to_value(kept) {
+                Ok(serde_json::Value::Object(mut window)) => {
+                    window.insert("connection".into(), id.0.clone().into());
+                    serde_json::Value::Object(window)
+                }
+                _ => serde_json::Value::Null,
+            }
+        });
+        written.collect()
+    }
+}
+
+/// What `windows` has of each of `open`, at its place. Nothing at all
+/// where they are not as many, or where one is not of the window at its
+/// place: an older version wrote them back beside an `open` of its own.
+/// A window that does not read costs only itself.
+fn windows_of(open: &[serde_json::Value], windows: &serde_json::Value) -> Vec<Option<Kept>> {
+    let Some(windows) = windows
+        .as_array()
+        .filter(|windows| windows.len() == open.len())
+    else {
+        return Vec::new();
+    };
+    let mut read = Vec::new();
+    for (entry, window) in open.iter().zip(windows) {
+        let window = match (entry, window) {
+            (_, serde_json::Value::Null) => None,
+            (serde_json::Value::String(id), serde_json::Value::Object(window))
+                if window.get("connection").and_then(|of| of.as_str()) == Some(id) =>
+            {
+                let mut window = window.clone();
+                window.remove("connection");
+                match serde_json::from_value(serde_json::Value::Object(window)) {
+                    Ok(kept) => Some(kept),
+                    Err(error) => {
+                        // Not the error's text: it can quote a stored filter value.
+                        log::warn!("a window of {id} was not read: {:?}", error.classify());
+                        None
+                    }
+                }
+            }
+            _ => return Vec::new(),
+        };
+        read.push(window);
+    }
+    read
 }
 
 impl Kept {
```

- [ ] **Step 4: Run the tests, then the checks**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-core --lib session::`

Expected: every `session::` test passes, the four new ones among them. Then every check under Checks passes; the app's library ends `test result: ok. 3064 passed; 0 failed; 16 ignored; 0 measured; 0 filtered out; finished in …`.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-core/src/session.rs
git commit -m "Keep each window's database and tables in session.json beside it (#366)"
```

---

### Task 2: The launch reopens each window as it was

**Files:**
- Modify: `crates/tabletist-core/src/session.rs`, `src/app.rs`, `src/app/startup_tests.rs`

What it holds: `save_session` writes each window's `Kept::of` beside it in `open`. The picker has none, and a session of the picker alone holds none, as the file reads it back. `connections` is kept as before, the first window in the bar on the connection's own database being the one kept there.

`open_workspace`'s run 1 `reopen: bool` becomes `Reopen`:
- `Reopen::Nothing`: a new window beside another, opening nothing.
- `Reopen::Connection`: what the connection's window kept, where it is of the database the window opens on. This is every connect from the picker, a reconnect, and a launch with nothing of the window.
- `Reopen::Window(kept)`: this window at the last quit. `session::put_on` puts the connection on the database it was on (a server's; a SQLite file or a Files connection opens what it names), whatever the settings say (spec decision 10). Its tables and its row panel are `kept`'s, the tables where the settings reopen them, or on a relaunch to update.

`open_last_session` takes each window's from `Session::windows` at its place.

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/app/startup_tests.rs b/src/app/startup_tests.rs
--- a/src/app/startup_tests.rs
+++ b/src/app/startup_tests.rs
@@ -1339,3 +1339,149 @@
         Some(("SELECT 2", RunMode::ReadWrite))
     );
 }
+
+// Each window as it was at quit: two windows of one connection, each on
+// its own database and with its own tables (#366).
+
+/// The names of the tables of the `index`th window, in order.
+fn tables(app: &App, index: usize) -> Vec<String> {
+    app.workspace(app.tabs[index].id)
+        .unwrap()
+        .object_tabs()
+        .map(|object| object.object.name.clone())
+        .collect()
+}
+
+/// The database the `index`th window is on.
+fn database(app: &App, index: usize) -> String {
+    app.workspace(app.tabs[index].id)
+        .unwrap()
+        .spec
+        .database
+        .clone()
+}
+
+/// Bookshop's development server with `books` open in its window, and a
+/// second window beside it on `second` with `authors`, in front; the
+/// files written as the app quits.
+fn quit_with_two_windows(app: &mut App, second: &str) -> crate::connections::ConnectionId {
+    let conn = bookshop_at(app, "postgres://demo@localhost:5432/bookshop_development");
+    let first = open_window(app, &conn);
+    connected_to(app, tabletist_db::Driver::Postgres);
+    books(app, first, "books");
+    app.apply(Action::OpenDatabaseInNewWindow {
+        tab: first,
+        database: second.into(),
+    });
+    let window = app.active_tab_id();
+    connected_to(app, tabletist_db::Driver::Postgres);
+    books(app, window, "authors");
+    app.save_session();
+    write_files(app);
+    conn
+}
+
+/// Writes the session the app last sent to be written, as the backend
+/// would, `change`d first.
+fn save_written(app: &App, change: impl FnOnce(&mut Session)) {
+    let mut session = written(app).unwrap();
+    change(&mut session);
+    session.save(&app.dirs.session_file()).unwrap();
+}
+
+#[test]
+fn each_window_of_a_connection_reopens_on_its_own_database_with_its_tables() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = quit_with_two_windows(&mut app, "bookshop_test");
+    save_written(&app, |_| {});
+    let app = start(dir.path());
+    assert_eq!(windows(&app), vec![Some(conn.clone()), Some(conn)]);
+    assert_eq!(app.active, 1, "the second in front again");
+    assert_eq!(database(&app, 0), "bookshop_development");
+    assert_eq!(database(&app, 1), "bookshop_test");
+    assert_eq!(tables(&app, 0), ["books"]);
+    assert_eq!(tables(&app, 1), ["authors"]);
+    // Each connects to its own, the one in front first.
+    let connects: Vec<String> = app
+        .backend
+        .sent
+        .iter()
+        .filter_map(|command| match command {
+            Command::Connect { spec, .. } => Some(spec.database.clone()),
+            _ => None,
+        })
+        .collect();
+    assert_eq!(connects, ["bookshop_test", "bookshop_development"]);
+}
+
+#[test]
+fn two_windows_on_one_database_each_get_their_own_tables_back() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    quit_with_two_windows(&mut app, "bookshop_development");
+    save_written(&app, |_| {});
+    let app = start(dir.path());
+    assert_eq!(database(&app, 1), "bookshop_development");
+    assert_eq!(tables(&app, 0), ["books"]);
+    assert_eq!(tables(&app, 1), ["authors"]);
+}
+
+#[test]
+fn with_restore_off_each_window_reopens_on_its_own_database_with_no_table() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    app.apply(Action::SetOption(
+        crate::settings::OptionValue::RestoreTabs(false),
+    ));
+    quit_with_two_windows(&mut app, "bookshop_test");
+    save_written(&app, |_| {});
+    let app = start(dir.path());
+    assert_eq!(database(&app, 0), "bookshop_development");
+    assert_eq!(database(&app, 1), "bookshop_test");
+    assert!(tables(&app, 0).is_empty());
+    assert!(tables(&app, 1).is_empty());
+}
+
+#[test]
+fn a_file_with_nothing_of_its_windows_reopens_them_as_before() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = quit_with_two_windows(&mut app, "bookshop_test");
+    // As a file from before, or one an older version wrote back.
+    save_written(&app, |session| session.windows.clear());
+    let app = start(dir.path());
+    assert_eq!(windows(&app), vec![Some(conn.clone()), Some(conn)]);
+    // On the connection's own database, with what it kept.
+    assert_eq!(database(&app, 1), "bookshop_development");
+    assert_eq!(tables(&app, 0), ["books"]);
+    assert_eq!(tables(&app, 1), ["books"]);
+}
+
+#[test]
+fn the_file_holds_each_window_beside_it_and_the_connections_own_tables() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = quit_with_two_windows(&mut app, "bookshop_test");
+    let session = written(&app).unwrap();
+    assert_eq!(session.open, vec![Some(conn.clone()), Some(conn.clone())]);
+    let kept: Vec<(String, Vec<String>)> = session
+        .windows
+        .iter()
+        .map(|window| {
+            let window = window.as_ref().unwrap();
+            let names = window.tabs.iter().map(|tab| tab.name.clone()).collect();
+            (window.database.clone(), names)
+        })
+        .collect();
+    assert_eq!(
+        kept,
+        [
+            ("bookshop_development".to_owned(), vec!["books".to_owned()]),
+            ("bookshop_test".to_owned(), vec!["authors".to_owned()]),
+        ]
+    );
+    // The connection's own, for opening it from the picker, is the first
+    // window's.
+    assert_eq!(session.connections[&conn.0].tabs[0].name, "books");
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib startup_tests`

Expected: the old file's test passes, as a launch reads it today; the other four fail, nothing being written of the windows or reopened from them:

```
test app::startup_tests::each_window_of_a_connection_reopens_on_its_own_database_with_its_tables ... FAILED
test app::startup_tests::the_file_holds_each_window_beside_it_and_the_connections_own_tables ... FAILED
test app::startup_tests::with_restore_off_each_window_reopens_on_its_own_database_with_no_table ... FAILED
test app::startup_tests::two_windows_on_one_database_each_get_their_own_tables_back ... FAILED
test result: FAILED. 40 passed; 4 failed; 0 ignored; 0 measured; 3041 filtered out; finished in …
```

- [ ] **Step 3: Write the code**

```diff
diff --git a/crates/tabletist-core/src/session.rs b/crates/tabletist-core/src/session.rs
--- a/crates/tabletist-core/src/session.rs
+++ b/crates/tabletist-core/src/session.rs
@@ -158,6 +158,22 @@
         tabletist_db::Driver::Postgres
         | tabletist_db::Driver::MySql
         | tabletist_db::Driver::BigQuery => spec.database.clone(),
+    }
+}
+
+/// Puts `spec` on `database`, a name [`database_of`] gave: a server's
+/// database. A SQLite file or a Files connection opens what it names, and
+/// a window kept with no database stays on its connection's.
+pub fn put_on(spec: &mut tabletist_db::ConnectSpec, database: &str) {
+    match spec.driver {
+        tabletist_db::Driver::Postgres
+        | tabletist_db::Driver::MySql
+        | tabletist_db::Driver::BigQuery
+            if !database.is_empty() =>
+        {
+            spec.database = database.to_owned();
+        }
+        _ => {}
     }
 }
 
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -109,6 +109,18 @@
     Reading(crate::theme::Look, u64),
     /// Its fonts are in place from this frame: it is drawn.
     Installed(crate::theme::Look),
+}
+
+/// What a window that opens on a connection puts back.
+enum Reopen {
+    /// Nothing: a new window beside another, on a database of its own.
+    Nothing,
+    /// What the connection's window kept, where it is of the database the
+    /// window opens on.
+    Connection,
+    /// What this window had at the last quit: the database it was on, its
+    /// tables and its row panel.
+    Window(crate::session::Kept),
 }
 
 pub struct App {
@@ -3728,8 +3740,9 @@
         }
     }
 
-    /// The windows open at the last quit, each connecting as a connection
-    /// does, asking for what it needs: the one in front first, so that a
+    /// The windows open at the last quit, each on the database it was on
+    /// with the tables it had, connecting as a connection does, asking
+    /// for what it needs: the one in front first, so that a
     /// question it asks at once is asked over it, and the others in their
     /// order after it. A window behind that asks before the front window
     /// does (the front one is still reading its password from the
@@ -3765,7 +3778,9 @@
             {
                 self.relaunch_windows.insert(tab.id, window);
             }
-            windows.push((tab.id, saved));
+            // What the window had at quit: its database and its tables.
+            let kept = self.session.windows.get(index).cloned().flatten();
+            windows.push((tab.id, saved, kept));
             self.tabs.push(tab);
             if index <= self.session.active {
                 active = Some(self.tabs.len() - 1 - base);
@@ -3777,9 +3792,13 @@
             let front = windows.remove(active);
             windows.insert(0, front);
         }
-        for (tab, saved) in windows {
+        for (tab, saved, kept) in windows {
             if let Some(saved) = saved {
-                self.connect_tab(tab, saved, Secrets::default());
+                // A file with nothing of the window: what its connection
+                // kept, on the connection's own database.
+                let reopen = kept.map_or(Reopen::Connection, Reopen::Window);
+                self.open_workspace(tab, saved, Secrets::default(), reopen);
+                self.authenticate(tab);
             }
         }
     }
@@ -3864,8 +3883,9 @@
     }
 
     /// Writes session.json on the backend: the connection tabs open now,
-    /// in order, the picker as none, and the tables of every window open,
-    /// with what is kept of the connections closed before. A connection
+    /// in order, the picker as none, each with its database and its
+    /// tables, and what is kept of every connection, its windows open now
+    /// and those closed before. A connection
     /// that is not saved (deleted while its window is open) is left out.
     /// The window calls it as the app quits.
     pub fn save_session(&mut self) {
@@ -3880,23 +3900,32 @@
             self.keep_tabs(*id);
         }
         let mut open = Vec::new();
+        let mut windows = Vec::new();
         let mut active = 0;
         for (index, tab) in self.tabs.iter().enumerate() {
-            let entry = match &tab.content {
-                ConnTabContent::Picker(_) => None,
+            let (entry, window) = match &tab.content {
+                ConnTabContent::Picker(_) => (None, None),
                 ConnTabContent::Workspace(workspace) => {
                     if self.connections.get(&workspace.conn_id).is_none() {
                         continue;
                     }
-                    Some(workspace.conn_id.clone())
+                    let kept = crate::session::Kept::of(workspace);
+                    (Some(workspace.conn_id.clone()), Some(kept))
                 }
             };
             if index == self.active {
                 active = open.len();
             }
             open.push(entry);
+            windows.push(window);
+        }
+        // Nothing of any window (the picker alone) is none, as the file
+        // reads back.
+        if windows.iter().all(Option::is_none) {
+            windows.clear();
         }
         self.session.open = open;
+        self.session.windows = windows;
         self.session.active = active;
         // The connections were read, or this would not write: one that is
         // not among them, deleted from connections.toml by hand, goes.
@@ -3936,25 +3965,25 @@
         let id = window.id;
         self.tabs.insert(index + 1, window);
         self.active = index + 1;
-        self.open_workspace(id, saved, typed, false);
+        self.open_workspace(id, saved, typed, Reopen::Nothing);
         self.authenticate(id);
     }
 
     /// Opens `saved` in `tab`; `typed` holds secrets typed in the dialog.
     fn connect_tab(&mut self, tab: ConnTabId, saved: SavedConnection, typed: Secrets) {
-        self.open_workspace(tab, saved, typed, true);
+        self.open_workspace(tab, saved, typed, Reopen::Connection);
         self.authenticate(tab);
     }
 
     /// Puts a connecting workspace for `saved` in `tab`, without starting to
-    /// authenticate. With `reopen` it puts back the tables the connection's
-    /// window kept, where the settings reopen them.
+    /// authenticate, with what `reopen` puts back: the tables where the
+    /// settings reopen them, and the row panel as it was left.
     fn open_workspace(
         &mut self,
         tab: ConnTabId,
-        saved: SavedConnection,
+        mut saved: SavedConnection,
         typed: Secrets,
-        reopen: bool,
+        reopen: Reopen,
     ) {
         let session = SessionId(self.next_id());
         let request = RequestId(self.next_id());
@@ -3966,16 +3995,27 @@
         // that what the window just had is what reopens, if this connection
         // was it.
         let replaced = self.keep_tabs(tab);
-        // What the connection's window last had: its row panel as it was
-        // left, and its tables, where the settings reopen them and they
-        // are of the database it opens on.
-        let kept = self.session.connections.get(&saved.id.0).cloned();
+        // What the window last had, or else the connection's window: its
+        // row panel as it was left, and its tables, where the settings
+        // reopen them and they are of the database it opens on. A window
+        // as it was at quit is on the database it was on, whatever the
+        // settings reopen.
+        let tables_back = !matches!(reopen, Reopen::Nothing);
+        let kept = match reopen {
+            Reopen::Nothing | Reopen::Connection => {
+                self.session.connections.get(&saved.id.0).cloned()
+            }
+            Reopen::Window(kept) => {
+                crate::session::put_on(&mut saved.spec, &kept.database);
+                Some(kept)
+            }
+        };
         let row_panel = kept.as_ref().is_none_or(|kept| kept.row_panel);
         // A relaunch after an update puts back what the restart took away,
         // whatever the settings reopen.
         let restore = self.settings.restore_tabs || self.relaunch.is_some();
         let tables = kept.filter(|kept| {
-            reopen && restore && kept.database == crate::session::database_of(&saved.spec)
+            tables_back && restore && kept.database == crate::session::database_of(&saved.spec)
         });
         let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) else {
             return;
@@ -6473,7 +6513,7 @@
             if let (Some(kind), Some(secret)) = (SecretKind::for_ssh(&saved.spec), &typed_ssh) {
                 *kind.slot(&mut secrets) = Some(secret.clone());
             }
-            self.open_workspace(tab, saved, secrets, true);
+            self.open_workspace(tab, saved, secrets, Reopen::Connection);
             // Ask for a secret whose saved copy was for the old server,
             // rather than trusting the keyring to have dropped it.
             if let Some(workspace) = self.workspace_mut(tab) {
```

- [ ] **Step 4: Run the tests, then the checks**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib startup_tests`

Expected: every `startup_tests` test passes, the five new ones among them. Then every check under Checks passes; the app's library ends `test result: ok. 3069 passed; 0 failed; 16 ignored; 0 measured; 0 filtered out; finished in …`.

- [ ] **Step 5: Try it**

By hand on the demo database (never Safari data, nothing committed): open a second window of the Bookshop on another database with a table open in each, quit, and start again. Each window is back on its database with its table; with Restore tabs and filters off, each is on its database with none.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-core/src/session.rs src/app.rs src/app/startup_tests.rs
git commit -m "Reopen each window on its own database with its own tables at launch (#366)"
```

---

### Task 3: The docs

**Files:**
- Modify: `docs/_reference/settings-and-files.md`, `docs/_reference/connections.md`

- [ ] **Step 1: Write them**

```diff
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -43,7 +43,9 @@
 window connects with the password this one connected with, so nothing is
 asked again, and this one keeps its tables, editors and edits. Do it on
 the database you are on for a second window on it. The chips tell the
-windows apart by their database.
+windows apart by their database. The next time Tabletist starts, each
+window opens again on its own database, with its own tables where
+**Restore tabs and filters** is on.
 
 ## Fields
 
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -13,8 +13,8 @@
 | Option | Choices | Default |
 | --- | --- | --- |
 | **Language** | System, English, Português (Brasil) or Русский. Menus, labels and messages change at once; data, SQL, keys and `:` commands stay as they are. System follows the first language of your system (macOS's Language & Region, `$LANG` and its kin on Linux, Windows's display language), where Tabletist has a translation of it; for any other language Tabletist is in English, the menu shows English, and the line under the label says the language is not translated yet. **Help translate** opens [Translating]({% link _reference/translating.md %}); on Omarchy the row's hint names the catalogs' folder, `assets/i18n`, instead. **Reset to defaults** leaves the language as it is | System |
-| **On launch** | Restore last session, Show Connections, or a saved connection, each listed with its server and database. Restore opens again the connection windows open when Tabletist last quit, in their order, each connecting and asking for what it needs; with nothing kept, it shows Connections. If the chosen connection was deleted outside Tabletist, Tabletist shows Connections, says so once, and sets this back to Show Connections; deleting it in Tabletist sets it back at once. `--demo` starts as it always does | Restore last session |
-| **Restore tabs and filters** | On or off. On, a connection that opens, at launch or from Connections, opens again the table and view tabs its window had when it last closed on that database (or SQLite file), each with its sort and filters; the one in front loads, and each other when you first show it. SQL editors are not opened again. Off, a connection opens with its tables in the sidebar and no tab | On |
+| **On launch** | Restore last session, Show Connections, or a saved connection, each listed with its server and database. Restore opens again the connection windows open when Tabletist last quit, in their order, each on the database it was on, connecting and asking for what it needs; with nothing kept, it shows Connections. If the chosen connection was deleted outside Tabletist, Tabletist shows Connections, says so once, and sets this back to Show Connections; deleting it in Tabletist sets it back at once. `--demo` starts as it always does | Restore last session |
+| **Restore tabs and filters** | On or off. On, a connection that opens, at launch or from Connections, opens again the table and view tabs its window had when it last closed on that database (or SQLite file), each with its sort and filters; at launch each window has its own back, on the database it was on, two windows of one connection too. The one in front loads, and each other when you first show it. SQL editors are not opened again. Off, a connection opens with its tables in the sidebar and no tab | On |
 | **Rows per page** | 100, 500, 1,000, 5,000 or 10,000. A table takes a new size when it next loads, not while you page through it | 1,000 |
 | **Timestamps** | To the second, or full precision | To the second |
 | **Time zone for timestamptz** | System, UTC or any zone. A PostgreSQL `timestamptz` is shown in this zone, a copy gives the value as the server sent it, and a value you type without an offset is sent with this zone's. In the Omarchy look `h`/`l` choose System, UTC or the last zone the file named; any other zone is set in the file | System |
@@ -277,7 +277,7 @@
 | --- | --- | --- |
 | `interface.language` | `"system"`, `"en"`, `"pt-BR"` or `"ru"`, in any case; a regional tag of one of them, such as `"ru-RU"`, reads as that language, and any Portuguese tag (`"pt"`, `"pt-PT"`) reads as Brazilian Portuguese. `"system"` follows the system's first language where Tabletist has a translation of it, and is English otherwise; a later language in the system's list is never used in its place. A language Tabletist has no translation of is a line that cannot be read | `"system"` |
 | `startup.on_launch` | `"restore"`, `"connections"` or `"conn:<id>"`, a saved connection's `id` from `connections.toml`. Tabletist writes the connection's name in a comment beside it, `on_launch = "conn:6a0c…"  # Bookshop`, and replaces whatever comment is beside it each time it sets the key, and again when that connection is renamed | `"restore"` |
-| `startup.restore_tabs` | `true` or `false`. Whether a connection opens again the table tabs, sorts and filters its window last had, kept in `session.json`. The row panel comes back as the window last left it either way | `true` |
+| `startup.restore_tabs` | `true` or `false`. Whether a connection opens again the table tabs, sorts and filters its window last had, kept in `session.json`; at launch each window has its own back, two windows of one connection too. The row panel, and at launch the database each window was on, come back either way | `true` |
 | `data.page_size` | 10 to 10000. A table takes a new size when it next loads (opened, refreshed, sorted, filtered or reconnected), not while you page through it | `1000` |
 | `data.timestamps` | `"second"` or `"full"` | `"second"` |
 | `data.tz` | `"system"`, `"utc"` or a zone such as `"America/New_York"`. A name the zone database does not know is a line that cannot be read. A PostgreSQL `timestamptz` value is shown and edited in this zone, a copy gives it as the server sent it, and one you type without an offset is sent with this zone's | `"system"` |
@@ -450,7 +450,7 @@
 | `last_used.json` | State | When each saved connection last connected |
 | `table_lists.json` | State | How each saved connection lists its tables, where you switched it in the sidebar |
 | `connections.moved` | State | A note that your connections are kept in `connections.toml` |
-| `session.json` | State | The connection windows open when Tabletist last quit, and each saved connection's window as it last had it: its table tabs, with their sort and filters, and whether the row panel was shown. Written at quit, when a connection's window closes or gets another connection, when a database switch drops its tables, and when a connection is deleted. Not written while `connections.toml` cannot be read. A connection deleted from `connections.toml` by hand is taken out of it at the next write |
+| `session.json` | State | The connection windows open when Tabletist last quit, each with the database it was on, its table tabs and its row panel, and each saved connection's window as it last had it: its table tabs, with their sort and filters, and whether the row panel was shown. Written at quit, when a connection's window closes or gets another connection, when a database switch drops its tables, and when a connection is deleted. Not written while `connections.toml` cannot be read. A connection deleted from `connections.toml` by hand is taken out of it at the next write |
 | `recents.json` | State | What you opened last on each saved connection, by its id: up to eight tables and views, with the database each is in and when it was opened, for the start screen. Written when something opens, when Clear is pressed, when an object the database no longer lists is dropped, and when a connection is deleted |
 | `labels.json` | State | The column you chose to label each table's rows in a foreign key's list, by saved connection and, on PostgreSQL, by database |
 | `export_options.json` | State | The options last chosen for each format of an export and of Copy as, by format, written when an export starts: CSV, TSV, JSON, JSON Lines, SQL INSERT and Markdown. An option this version does not know is kept and written back |
```

- [ ] **Step 2: Check the site**

Run: `cd docs && bundle install && bin/check`

Expected: the site builds and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_reference
git commit -m "Say that each window opens again on its own database (#366)"
```

---

## After the run

- Open the PR for the three commits once #374 is merged and this branch is rebased onto main, with "Part of #366".
- Run 3 (the picker's "Open another") gets a plan of its own, after #373's picker work.
