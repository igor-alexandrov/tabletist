# Settings › General, step 3: Startup. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Settings › General gets its Startup group, in both looks: **On launch** opens the windows open at the last quit, the Connections picker, or a chosen saved connection; **Restore tabs and filters** has a connection that opens reopen the table tabs its window last had, with their sort and filters; and a new state file, `session.json`, keeps both, written at quit and when a connection's window closes.

**Architecture:** `src/session.rs` is the file: `Session { open, active, connections }`, each connection's `Kept { database, tabs, active }` of `KeptTab`s, read with `util::load_json` and written by the backend (`StateFile::Session`). The app keeps it in `App::session`; `App::keep_tabs` puts a window's tables in it before the window closes or takes another connection, and `App::save_session` writes it, from `Window::on_exit` at quit too. `App::open_workspace`, which every open of a connection goes through, reopens the kept tables (`App::reopen_tabs`) when `startup.restore_tabs` is on; a reopened tab is marked (`ObjectTab::reopened`) and read when it is first in front (`App::read_reopened`), the one in front with the connect. `startup.on_launch` is a `settings::Launch` (`Restore`, `Connections`, `Connection { id, name }`, compared by the id alone, the name the comment beside it in the file); `App::launch`, from `App::new`, opens what it says.

**Tech Stack:** Rust 2024, egui, serde and serde_json (already dependencies), the `toml` reader the settings already use.

---

## Where this sits

Step 3 of `docs/superpowers/specs/2026-10-17-settings-general-rows-design.md`, its last. Step 1 (the file and the Data rows) is pull request igor-alexandrov/tabletist#154, and step 2 (Safety) igor-alexandrov/tabletist#160 and igor-alexandrov/tabletist#164, with igor-alexandrov/tabletist#165 after them; all are merged. This plan is a branch of `main` at `263745d`. The spec's departures for this step were approved with it: one window, so its connection tabs are what is reopened; hidden columns are not built, so nothing of them is saved (the helper text keeps the board's words); SQL editor tabs are not reopened; and with Restore tabs off a connection opens as it does today.

## Where this departs from the spec or the boards

Each row needs Igor's yes before the plan is run.

| What the spec or the board says | What this plan builds | Why |
|---|---|---|
| macOS board: the On launch row's hint, "Or: show Connections · open a chosen connection", beside its menu | The same words as the row's small print, under "On launch", as other rows have theirs | Beside the 220-point menu the sheet has 211 points left at its full width, and the words take 285: a hint that does not fit is left out (`sheet::hint`), so it would never show |
| Spec: a deleted chosen connection "falls back" at launch | That, and also deleting the chosen connection in the app sets On launch back to Show Connections at once, without a notice | What the menu shows is what the next launch does (the spec's own reason for setting the option back); the person deleted it, so there is nothing to tell |
| Spec's file: `"connections": { "<id>": { "tabs": […], "active": 0 } }` | Each connection's entry also says `"database"`: the one its window was on, or a SQLite connection's file (`session::database_of`). Its tables are reopened only where the connection opens on that database, and a window switched to another database than its connection's keeps nothing when it closes: what the connection kept of its own database stays | A PostgreSQL window can switch database (`Action::SwitchDatabase`), and a saved SQLite connection can be pointed at another file; their tables are the other database's, and reopened on this one they would fail or, worse, show another table of one name (found by the review). A connection opens on its own database, so a switched window's tables could never be reopened, and kept they would take the place of the ones that can (found by the second review) |

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Where `session.json` is | The state directory: `~/.local/state/tabletist/session.json` on Linux, beside `table_lists.json` | The spec. Of this computer, and no setting |
| What `open` lists | Each connection window in the bar's order, by its saved connection's id, and the picker as `null`. A window whose connection was deleted while it was open is left out. `active` counts in that list | The spec. A connection that is not saved cannot be opened again |
| When the file is written | At quit (`Window::on_exit`, before the backend's flush), when a connection's window closes (its tab closed, Disconnect, another connection connected in it), and when a connection is deleted (its entry goes). Never while `connections.toml` could not be read | The spec. With no connections read no window can be open, and writing then would lose every connection's tables |
| What a connection's entry keeps | Its table and view tabs in strip order: schema, name, kind, the sort (`[["title", "asc"]]`), whether it is pinned (written only when it is not), and the filter bar: its rows (`[["country", "eq", "IE"]]`), whether it is open, whether the WHERE text is used, and that text. `active` is the table that was in front, left out where a SQL editor or the Agents tab was | "The filter is kept as the filter bar holds it" (the spec). The operator is written as its own word (`eq`, `starts_with`), not as the bar shows it |
| Not kept | The page a tab was on, the selection, the Data or Structure view, the row panel, pending edits, the Recent list | The spec names tabs, sort and filters |
| A filter row typed and not applied | Reopened applied: the query is the bar's | The bar is what is kept |
| What a newer version wrote that this one does not know | A tab of an unknown kind is left out, and so are a sort and a filter row with an unknown word. Keys this version does not know, of the file, of a connection's entry, of a tab and of a filter bar, are written back as they were (`rest`, flattened), until that connection's window closes here and writes its own; a connection's entry that does not read is kept whole as the file had it, until then or until it is deleted; a window in `open` that is neither an id nor `null`, and an `active` that is no number, cost themselves. A file that is not JSON is kept aside as `session.json.bad` (`util::load_json`) | As `table_lists.json` keeps what it cannot read (found by the review: an entry dropped was lost at the next write, and one wrong value lost the file; and by the second review: a key a newer version added was lost at the next quit) |
| What is read when a connection opens with its tables | The one in front, with the connect (`after_connect`, as any tab in front). Each other when it is first in front: `App::read_reopened`, after the frame's actions, whatever brought it there | The spec: "the one that was active is fetched". Reading every tab at once would send a page for each |
| The table in front, where the one kept was a SQL editor's place | The first table | Something is in front |
| A table that no longer exists | Its tab opens, and its read fails as an open tab's does when its table is dropped: the error, and Retry | The spec |
| A connection open in two windows | Both are reopened at launch, and both open with the same tables: those of the window that closed last, or at quit of the last of them in the bar | One entry for each connection, as the spec has it |
| Restore last session, at launch | Each window in `open` opens in its place and connects as from the picker, asking for what it needs (a password not saved, a host key): the one in front first, then the others in their order. The picker is opened once at most, wherever it was in the order; a connection deleted since is left out; with nothing to open, the picker | The spec (the picker after a window was lost, found by the second review). One question is asked at a time: asked first, it is the window in front's, and a window behind that needs one says so and offers Reconnect, as when a connect finds a dialog open (found by the review) |
| A chosen connection that is gone | The picker, the notice "The connection chosen to open at launch was deleted. Tabletist showed Connections instead.", and On launch set to Show Connections, which writes the file. With a `connections.toml` that could not be read, nothing is known to be deleted: the picker opens and the option stays | The spec. A file that could not be read says so in the notice already |
| The comment beside a chosen connection | `on_launch = "conn:<id>"  # <name>`: the app writes it each time it sets the key, replacing what was beside it, and again when that connection is renamed; it reads the name back from it. A break in a name is written as a space. Restore and Connections get `# restore \| connections \| "conn:<id>"`. In an inline table (`startup = { … }`) the value alone is set | The spec, and the Omarchy board's note. A value is compared by its id alone, so a renamed connection is the same choice. A name with a break would start a line of its own, and could set any key (found by the review), and an inline table's comment is after the whole table |
| The macOS menu | Restore last session, Show Connections, then each saved connection by name, in the order they are saved, its server and database in the muted detail at the right (`localhost:5432/bookshop_preview`; a SQLite file's name). An id the file names that is not saved shows as "Connection not found" | The spec: "three Bookshops can be told apart". What the menu shows is what the next launch does |
| The Omarchy row | `‹ restore last session ›`, `‹ show connections ›`, `‹ Bookshop (localhost:5432/bookshop_preview) ›`, stepped with `h`/`l` in that order; the hint `or connections · a chosen connection` | As drawn, with a connection's name as it is saved |
| Where the Omarchy screen's cursor opens | On On launch, the first row | The cursor opens on the first row; the board's cursor on Timestamps shows a row in use |
| The tab's lead | "How Tabletist starts, shows data and protects your work. Changes apply right away." | The macOS board's; step 2 left out "starts" until this step |
| `--demo` | Starts as before: its profile is new each time, so it has no `session.json` and the defaults open the picker, which `demo_setup` connects | The spec |
| Reset to defaults | Puts back On launch and Restore tabs with the rest of General | Reset takes every option of `OptionId::GENERAL` |
| A legacy `settings.json` | Has neither key: they take their defaults | It was written before them |

## Before you start

- One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: state files stay readable, backward compatible and atomically written; views draw text only through `TextRole`s; the website's pages follow `src/settings.rs`.
- Each step's diff applies with `git apply` from the repository's root: save the block to a file and run `git apply <file>`. The code here is formatted.
- The tests use only the Bookshop data. No test compares a picture or a position with the design.

## What was run before this plan was written

Every task's state was built on `main` at `263745d` and passed `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets` with the test PostgreSQL, MySQL and SSH servers (3,773 tests after task 1, 3,780 after task 2 and 3,798 after task 3, none failed; `the_agent_opens_the_tunnel` skipped, as `TABLETIST_TEST_SSH_AGENT` needs an agent that holds the test key, which CI loads). Each task's tests were run alone on the code before it, and failed as its Step 2 says. The last state also passed `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. The artboards "macOS – Settings, General" and "Omarchy – Settings, General" were read again before the draft; the hint that has no room in the sheet was measured with a throwaway test (285 points of words, 211 of room). A reviewer given the draft wrote throwaway tests against it and found eight defects: seven are fixed in the task they belong to, each with a test there (the rows marked "found by the review" above: an inline `startup` table broken by the comment, a name with a line break, the window behind asking for the password, a renamed connection's comment, one wrong value losing the file, an entry this version cannot read lost at the next write, a SQLite connection pointed at another file), and the eighth, two windows of one connection, is decided above. A second reviewer read the generated plan against its diffs and the spec, and found three more defects, fixed the same way (marked "found by the second review": the picker lost after a window, a switched window's tables taking the place of its connection's, a newer version's keys lost at the next quit), and where the prose, the website and a doc comment said other than the code; those were mended, and Reset's test checks On launch too. Every diff in this plan was applied in order to a copy of `263745d`, and the result is the last state. Not run: CI, macOS and Windows, and the app by hand.

## Files

| File | What changes |
|---|---|
| `src/session.rs` (new) | `Session`, `Kept`, `KeptTab`, `KeptFilter`: the file, read and written; a window's tables kept, and reopened |
| `src/lib.rs`, `src/paths.rs`, `src/backend.rs` | The module; `AppDirs::session_file`; `StateFile::Session` |
| `src/app.rs` | `App::session`, `keep_tabs`, `save_session`, `reopen_tabs`, `read_reopened`, `launch`, `open_last_session`, `say`; what `close_tab`, Disconnect, `open_workspace`, `after_connect`, `send_what_waited`, `fetch_rows` and Delete do with them |
| `src/app/startup_tests.rs` (new) | The tests of the file, of reopening and of a launch |
| `src/entrypoint.rs` | `Window::on_exit` writes the session before the flush |
| `src/model.rs` | `ObjectTab::reopened`; the screen's rows |
| `src/settings.rs` | `Launch`; `startup.on_launch` and `startup.restore_tabs`, their options and values; `Settings::note`, `stepped_launch`; the comment beside On launch |
| `src/ui/settings/{mod,sheet,terminal}.rs`, `src/ui/picker.rs` | The Startup group in both looks; the menu and the stepper of On launch |
| `src/ui/mod.rs`, `src/ui/zone_tests.rs` | Their tests |
| `docs/_reference/settings-and-files.md` | The website |

---

### Task 1: Keep the open windows and each connection's tables in session.json

The file and its writes, with nothing yet read from it at a launch or an open. `src/session.rs` holds the types, how they are read and written, and how a workspace's table tabs become a `Kept` and a `KeptTab` an `ObjectTab` again (used by task 2); what it cannot read of a connection it keeps as it was, to write it back. `App::keep_tabs` takes a window's tables before it closes, or before another connection is connected in it; `App::save_session` writes the file. `close_tab` and Disconnect now send their `Command::Close` after the session's save, so the session's close stays the last thing they send, as their tests say.

**Files:** Modify `docs/_reference/settings-and-files.md`; Modify `src/app.rs`; Create `src/app/startup_tests.rs`; Modify `src/backend.rs`; Modify `src/entrypoint.rs`; Modify `src/lib.rs`; Modify `src/paths.rs`; Create `src/session.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index bb27781..2d963f9 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -30,6 +30,8 @@ use crate::theme::{self, Catalog, Palette};
 mod editing;
 mod key_picker;
 mod labels;
+#[cfg(test)]
+mod startup_tests;
 
 /// What a keyring read is for; each names the exact request it serves, so
 /// a late answer for an earlier attempt is dropped.
diff --git a/src/app/startup_tests.rs b/src/app/startup_tests.rs
new file mode 100644
index 0000000..7509468
--- /dev/null
+++ b/src/app/startup_tests.rs
@@ -0,0 +1,270 @@
+//! What is kept of the windows for the next launch (`session.json`), and
+//! what a launch and an open of a connection make of it.
+
+use super::*;
+use crate::backend::Backend;
+use crate::session::{Kept, Session};
+use tabletist_db::{FilterOp, ObjectKind, ObjectRef, SortDir};
+
+/// An app over a directory of its own, as the tests of the other state
+/// files have it.
+fn app_in(dir: &std::path::Path) -> App {
+    App::new(
+        AppDirs::at(dir),
+        Settings::default().into(),
+        Backend::recording(),
+    )
+}
+
+/// A saved Bookshop: the demo's SQLite file, by its own id.
+fn bookshop(app: &mut App, name: &str) -> crate::connections::ConnectionId {
+    let mut saved = crate::testing::fixture_connection();
+    saved.name = name.into();
+    saved.spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop.db");
+    let id = saved.id.clone();
+    app.connections.upsert(saved);
+    id
+}
+
+/// Opens `conn` in a window of its own, in front, and says it connected.
+fn open_window(app: &mut App, conn: &crate::connections::ConnectionId) -> ConnTabId {
+    let tab = match app.picker_index() {
+        Some(index) => app.tabs[index].id,
+        None => {
+            let tab = app.picker_tab();
+            let id = tab.id;
+            app.tabs.push(tab);
+            id
+        }
+    };
+    app.apply(Action::Connect {
+        tab,
+        conn: conn.clone(),
+    });
+    let index = app.tab_index(tab).unwrap();
+    app.active = index;
+    tab
+}
+
+fn books(app: &mut App, tab: ConnTabId, name: &str) -> TabId {
+    app.open_object(tab, ObjectRef::new("main", name), ObjectKind::Table, true);
+    app.workspace(tab).unwrap().active_tab.unwrap()
+}
+
+/// The newest session.json the app sent to be written.
+fn written(app: &App) -> Option<Session> {
+    let path = app.dirs.session_file();
+    app.backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|command| match command {
+            Command::Save {
+                path: to,
+                file: StateFile::Session(session),
+            } if *to == path => Some(session.clone()),
+            _ => None,
+        })
+}
+
+#[test]
+fn closing_a_window_keeps_its_tables_with_their_sort_and_filters() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    let tab = open_window(&mut app, &conn);
+    let titles = books(&mut app, tab, "books");
+    app.apply(Action::SortBy {
+        tab,
+        object_tab: titles,
+        column: "title".into(),
+    });
+    let authors = books(&mut app, tab, "authors");
+    if let Some(object) = app.object_tab_mut(tab, authors) {
+        object.filter.open = true;
+        object.filter.rows.push(crate::model::FilterRow {
+            column: "country".into(),
+            op: FilterOp::Eq,
+            value: "IE".into(),
+        });
+    }
+    app.apply(Action::ApplyFilters {
+        tab,
+        object_tab: authors,
+    });
+    // A SQL editor is not kept, and the table in front is.
+    app.apply(Action::NewSqlTab(tab));
+    app.apply(Action::ActivateTab { tab, id: titles });
+    assert!(
+        written(&app).is_none(),
+        "nothing is written while it is open"
+    );
+    app.apply(Action::CloseConnTab(tab));
+    let session = written(&app).expect("written as the window closed");
+    let kept = &session.connections[&conn.0];
+    assert_eq!(kept.database, "/tmp/bookshop.db");
+    assert_eq!(kept.active, Some(0));
+    let names: Vec<&str> = kept.tabs.iter().map(|tab| tab.name.as_str()).collect();
+    assert_eq!(names, ["books", "authors"]);
+    assert_eq!(kept.tabs[0].sort, vec![("title".into(), "asc".into())]);
+    assert_eq!(
+        kept.tabs[1].filter.rows,
+        vec![("country".into(), "eq".into(), "IE".into())]
+    );
+    // The window is gone: the picker is what is open.
+    assert_eq!(session.open, vec![None]);
+}
+
+#[test]
+fn a_sql_editor_in_front_leaves_no_table_in_front() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    let tab = open_window(&mut app, &conn);
+    books(&mut app, tab, "books");
+    app.apply(Action::NewSqlTab(tab));
+    let kept = Kept::of(app.workspace(tab).unwrap());
+    assert_eq!(kept.tabs.len(), 1);
+    assert_eq!(kept.active, None);
+}
+
+#[test]
+fn the_file_says_which_windows_are_open_in_order_and_which_is_in_front() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let first = bookshop(&mut app, "Bookshop");
+    let second = bookshop(&mut app, "Bookshop");
+    open_window(&mut app, &first);
+    let tab = open_window(&mut app, &second);
+    books(&mut app, tab, "books");
+    app.apply(Action::ShowConnections);
+    let picker = app.active;
+    app.active = picker - 1;
+    app.save_session();
+    let session = written(&app).expect("written");
+    assert_eq!(
+        session.open,
+        vec![Some(first.clone()), Some(second.clone()), None]
+    );
+    assert_eq!(session.active, 1);
+    // Every open window's tables, whether it closed or not.
+    assert_eq!(session.connections[&second.0].tabs[0].name, "books");
+    assert!(session.connections[&first.0].tabs.is_empty());
+}
+
+#[test]
+fn another_connection_put_in_a_window_keeps_the_one_it_had() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let first = bookshop(&mut app, "Bookshop");
+    let second = bookshop(&mut app, "Bookshop copy");
+    let tab = open_window(&mut app, &first);
+    books(&mut app, tab, "books");
+    app.apply(Action::Connect {
+        tab,
+        conn: second.clone(),
+    });
+    let session = written(&app).expect("written as the first closed");
+    assert_eq!(session.connections[&first.0].tabs[0].name, "books");
+    // And a disconnect with no picker open puts the picker in its place.
+    books(&mut app, tab, "authors");
+    app.apply(Action::Disconnect(tab));
+    let session = written(&app).expect("written as the second closed");
+    assert_eq!(session.connections[&second.0].tabs[0].name, "authors");
+    assert_eq!(session.open, vec![None]);
+}
+
+#[test]
+fn a_deleted_connection_takes_its_tables_out_of_the_file() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    let tab = open_window(&mut app, &conn);
+    books(&mut app, tab, "books");
+    app.apply(Action::CloseConnTab(tab));
+    assert!(written(&app).unwrap().connections.contains_key(&conn.0));
+    app.apply(Action::DeleteConnection(conn.clone()));
+    assert!(!written(&app).unwrap().connections.contains_key(&conn.0));
+}
+
+#[test]
+fn the_file_is_read_at_the_start() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    let tab = open_window(&mut app, &conn);
+    books(&mut app, tab, "books");
+    app.apply(Action::CloseConnTab(tab));
+    // As the backend writes it.
+    let session = written(&app).unwrap();
+    session.save(&app.dirs.session_file()).unwrap();
+    assert_eq!(app_in(dir.path()).session, session);
+}
+
+#[test]
+fn nothing_is_written_over_the_file_while_the_connections_cannot_be_read() {
+    let dir = tempfile::tempdir().unwrap();
+    let dirs = AppDirs::at(dir.path());
+    std::fs::create_dir_all(&dirs.config).unwrap();
+    std::fs::write(
+        dirs.connections_file(),
+        "version = 3\n\n[[connections]]\nname = 1\n",
+    )
+    .unwrap();
+    let mut app = app_in(dir.path());
+    assert!(app.notice.is_some());
+    app.save_session();
+    assert!(written(&app).is_none());
+}
+
+#[test]
+fn a_sort_the_other_way_is_kept_as_desc() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    let tab = open_window(&mut app, &conn);
+    let id = books(&mut app, tab, "books");
+    for _ in 0..2 {
+        app.apply(Action::SortBy {
+            tab,
+            object_tab: id,
+            column: "price".into(),
+        });
+    }
+    assert_eq!(
+        app.workspace(tab)
+            .unwrap()
+            .object_tab(id)
+            .unwrap()
+            .query
+            .sort[0]
+            .dir,
+        SortDir::Desc
+    );
+    let kept = Kept::of(app.workspace(tab).unwrap());
+    assert_eq!(kept.tabs[0].sort, vec![("price".into(), "desc".into())]);
+}
+
+#[test]
+fn a_window_switched_to_another_database_keeps_what_its_connection_had() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    let tab = open_window(&mut app, &conn);
+    books(&mut app, tab, "books");
+    app.apply(Action::CloseConnTab(tab));
+    let tab = open_window(&mut app, &conn);
+    // As a switch of database leaves it: another database, its tables.
+    if let Some(workspace) = app.workspace_mut(tab) {
+        workspace.spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop_archive.db");
+    }
+    books(&mut app, tab, "archived_books");
+    app.apply(Action::CloseConnTab(tab));
+    let session = written(&app).unwrap();
+    let names: Vec<&str> = session.connections[&conn.0]
+        .tabs
+        .iter()
+        .map(|tab| tab.name.as_str())
+        .collect();
+    assert_eq!(names, ["books"]);
+}
diff --git a/src/lib.rs b/src/lib.rs
index b2fc362..d0ff3ad 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -23,6 +23,7 @@ pub mod paths;
 pub mod relay;
 pub mod review;
 pub mod secrets;
+pub mod session;
 pub mod settings;
 #[cfg(all(test, feature = "shots"))]
 mod shots;
diff --git a/src/paths.rs b/src/paths.rs
index c17d046..1c3aaca 100644
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -223,6 +223,10 @@ mod tests {
             dirs.connections_moved_file(),
             PathBuf::from("/root/state/connections.moved")
         );
+        assert_eq!(
+            dirs.session_file(),
+            PathBuf::from("/root/state/session.json")
+        );
         assert_eq!(
             dirs.known_hosts_file(),
             PathBuf::from("/root/config/known_hosts.json")
diff --git a/src/session.rs b/src/session.rs
new file mode 100644
index 0000000..0069449
--- /dev/null
+++ b/src/session.rs
@@ -0,0 +1,218 @@
+//! What was open when the app last quit, and each connection's tables as
+//! they were when its window last closed: `session.json`, in the state
+//! directory. Of this computer, and no setting: what the next launch and
+//! the next open of a connection make of it is the settings' (`startup`).
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn books() -> KeptTab {
+        KeptTab {
+            schema: "public".into(),
+            name: "books".into(),
+            kind: "table".into(),
+            sort: vec![("title".into(), "asc".into())],
+            filter: KeptFilter {
+                open: true,
+                rows: vec![("format".into(), "eq".into(), "print".into())],
+                raw: true,
+                raw_text: "price > 10".into(),
+                ..KeptFilter::default()
+            },
+            pinned: true,
+            rest: BTreeMap::new(),
+        }
+    }
+
+    #[test]
+    fn a_session_is_written_and_read_back_the_same() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("session.json");
+        let bookshop = ConnectionId("6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11".into());
+        let session = Session {
+            open: vec![Some(bookshop.clone()), None],
+            active: 1,
+            connections: BTreeMap::from([(
+                bookshop.0.clone(),
+                Kept {
+                    database: "bookshop".into(),
+                    tabs: vec![books()],
+                    active: Some(0),
+                    rest: BTreeMap::new(),
+                },
+            )]),
+            ..Session::default()
+        };
+        session.save(&path).unwrap();
+        assert_eq!(Session::load(&path), session);
+        let text = std::fs::read_to_string(&path).unwrap();
+        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
+        assert_eq!(json["version"], 1);
+        assert_eq!(json["open"][1], serde_json::Value::Null);
+        let tab = &json["connections"][&bookshop.0]["tabs"][0];
+        assert_eq!(tab["sort"], serde_json::json!([["title", "asc"]]));
+        assert_eq!(tab["filter"]["rows"][0][1], "eq");
+        // What is as a new tab has it is left out.
+        assert!(tab.get("pinned").is_none(), "{text}");
+    }
+
+    #[test]
+    fn no_file_is_no_session_and_a_bad_one_is_kept_aside() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("session.json");
+        assert_eq!(Session::load(&path), Session::default());
+        std::fs::write(&path, "{ not json").unwrap();
+        assert_eq!(Session::load(&path), Session::default());
+        assert!(dir.path().join("session.json.bad").exists());
+    }
+
+    #[test]
+    fn a_connection_whose_part_does_not_read_costs_only_itself() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("session.json");
+        std::fs::write(
+            &path,
+            r#"{"version":1,"open":["a"],"active":0,"connections":{
+                "a":{"database":"bookshop","tabs":[{"schema":"public","name":"books","kind":"table"}]},
+                "b":{"tabs":"not a list"}}}"#,
+        )
+        .unwrap();
+        let session = Session::load(&path);
+        assert_eq!(session.open, vec![Some(ConnectionId("a".into()))]);
+        assert_eq!(session.connections.len(), 1);
+        assert_eq!(session.connections["a"].tabs[0].name, "books");
+    }
+
+    #[test]
+    fn a_window_or_a_place_of_the_wrong_kind_costs_only_itself() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("session.json");
+        std::fs::write(
+            &path,
+            r#"{"version":"one","open":[7,"a",null],"active":"first","connections":{
+                "a":{"database":"","tabs":[{"schema":"main","name":"books","kind":"table"}]}}}"#,
+        )
+        .unwrap();
+        let session = Session::load(&path);
+        assert_eq!(session.open, vec![Some(ConnectionId("a".into())), None]);
+        assert_eq!(session.active, 0);
+        assert_eq!(session.connections["a"].tabs[0].name, "books");
+    }
+
+    #[test]
+    fn a_part_this_version_cannot_read_is_written_back_until_it_is_kept_again() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("session.json");
+        let newer = serde_json::json!({
+            "database": "",
+            "tabs": [{"schema": "main", "name": "books", "kind": "table",
+                      "sort": [{"column": "title", "dir": "asc"}]}]
+        });
+        let text = serde_json::json!({"version": 2, "connections": {"a": newer.clone()}});
+        std::fs::write(&path, text.to_string()).unwrap();
+        let mut session = Session::load(&path);
+        assert!(session.connections.is_empty());
+        session.save(&path).unwrap();
+        let written: serde_json::Value =
+            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
+        assert_eq!(written["connections"]["a"], newer);
+        // Its window closed here: what this version kept takes its place.
+        session.keep(&ConnectionId("a".into()), Kept::default());
+        session.save(&path).unwrap();
+        assert_eq!(Session::load(&path).connections["a"], Kept::default());
+        // And a connection deleted takes it out.
+        let mut session = Session::load(&path);
+        assert!(session.forget(&ConnectionId("a".into())));
+        assert!(!session.forget(&ConnectionId("a".into())));
+    }
+
+    #[test]
+    fn keys_a_newer_version_added_are_written_back() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("session.json");
+        let text = serde_json::json!({
+            "version": 2,
+            "windows": {"width": 1280},
+            "open": ["a"],
+            "connections": {"a": {
+                "database": "",
+                "pane": "grid",
+                "tabs": [{"schema": "main", "name": "books", "kind": "table",
+                          "hidden": ["price"], "filter": {"open": true, "split": 2}}]
+            }}
+        });
+        std::fs::write(&path, text.to_string()).unwrap();
+        let session = Session::load(&path);
+        assert_eq!(session.connections["a"].tabs[0].name, "books");
+        session.save(&path).unwrap();
+        let written: serde_json::Value =
+            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
+        assert_eq!(written["windows"]["width"], 1280);
+        let kept = &written["connections"]["a"];
+        assert_eq!(kept["pane"], "grid");
+        assert_eq!(kept["tabs"][0]["hidden"][0], "price");
+        assert_eq!(kept["tabs"][0]["filter"]["split"], 2);
+        // This version writes its own number.
+        assert_eq!(written["version"], 1);
+    }
+
+    #[test]
+    fn a_sqlite_connections_tables_are_of_its_file() {
+        let spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop.db");
+        assert_eq!(database_of(&spec), "/tmp/bookshop.db");
+        let (spec, _) =
+            tabletist_db::ConnectSpec::from_url("postgres://demo@localhost:5432/bookshop").unwrap();
+        assert_eq!(database_of(&spec), "bookshop");
+    }
+
+    #[test]
+    fn a_tab_reopens_with_its_sort_and_its_filter_applied() {
+        let tab = books().reopen(TabId(7), 1_000).unwrap();
+        assert_eq!(tab.object, ObjectRef::new("public", "books"));
+        assert_eq!(tab.kind, ObjectKind::Table);
+        assert_eq!(tab.query.limit, 1_000);
+        assert_eq!(
+            tab.query.sort,
+            vec![Sort {
+                column: "title".into(),
+                dir: SortDir::Asc
+            }]
+        );
+        assert!(tab.filter.open);
+        assert_eq!(tab.query.filters.len(), 1);
+        assert_eq!(tab.query.filters[0].op, FilterOp::Eq);
+        assert_eq!(tab.query.raw_where.as_deref(), Some("price > 10"));
+        assert!(tab.rows.needs_load());
+        // Kept again, it is what it was.
+        assert_eq!(KeptTab::of(&tab), books());
+    }
+
+    #[test]
+    fn words_this_version_does_not_know_are_left_out() {
+        let mut tab = books();
+        tab.sort.push(("price".into(), "sideways".into()));
+        tab.filter
+            .rows
+            .push(("price".into(), "near".into(), "10".into()));
+        let reopened = tab.reopen(TabId(7), 1_000).unwrap();
+        assert_eq!(reopened.query.sort.len(), 1);
+        assert_eq!(reopened.filter.rows.len(), 1);
+        tab.kind = "sequence".into();
+        assert!(tab.reopen(TabId(7), 1_000).is_none());
+    }
+
+    #[test]
+    fn every_operator_and_kind_has_its_own_word() {
+        for op in FilterOp::ALL {
+            assert_eq!(op_of(op_name(op)), Some(op));
+        }
+        for kind in [
+            ObjectKind::Table,
+            ObjectKind::View,
+            ObjectKind::MaterializedView,
+        ] {
+            assert_eq!(kind_of(kind_name(kind)), Some(kind));
+        }
+    }
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` cannot find type `Session` in this scope `` (12 times); `` cannot find function, tuple struct or tuple variant `ConnectionId` in this scope `` (6 times); `` cannot find type `ObjectKind` in this scope `` (4 times); `` no method named `session_file` found for struct `paths::AppDirs` in the current scope `` (3 times), and 23 more.

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 11d9cf7..a2d911d 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -158,6 +158,7 @@ On Linux, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` move the two folders.
 | `last_used.json` | State | When each saved connection last connected |
 | `table_lists.json` | State | How each saved connection lists its tables, where you switched it in the sidebar |
 | `connections.moved` | State | A note that your connections are kept in `connections.toml` |
+| `session.json` | State | The connection windows open when Tabletist last quit, and each saved connection's table tabs, with their sort and filters, as its window last had them. Written at quit, when a connection's window closes, and when a connection is deleted |
 | `labels.json` | State | The column you chose to label each table's rows in a foreign key's list, by saved connection and, on PostgreSQL, by database |
 | `tabletist.log`, `panic.log` | State | Logs |
 | `agents.sqlite3` | State | Every call agents made: the tool, the statement and its values, and how it ended. Never a row |
diff --git a/src/app.rs b/src/app.rs
index 2d963f9..aeebe06 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -125,6 +125,13 @@ pub struct App {
     /// id, as the file had them: a newer version may have written them,
     /// and they are written back with the others.
     table_lists_unread: std::collections::BTreeMap<String, serde_json::Value>,
+    /// What was open at the last quit, and each connection's tables as
+    /// its window last had them, from session.json.
+    pub session: crate::session::Session,
+    /// Whether session.json is written: not while the connections could
+    /// not be read, when no window could be open and the file would lose
+    /// every connection's tables.
+    writes_session: bool,
     pub dialog: Option<Dialog>,
     /// Keyring reads in flight, and what each is for.
     pending_secrets: HashMap<RequestId, SecretPurpose>,
@@ -254,6 +261,8 @@ impl App {
             crate::settings::Zone::System | crate::settings::Zone::Utc => None,
         };
         let (table_lists, table_lists_unread) = load_table_lists(&dirs.table_lists_file());
+        let session = crate::session::Session::load(&dirs.session_file());
+        let writes_session = loaded_connections.problem.is_none();
         let mut app = Self {
             dirs,
             settings,
@@ -274,6 +283,8 @@ impl App {
             labels,
             table_lists,
             table_lists_unread,
+            session,
+            writes_session,
             dialog: None,
             pending_secrets: HashMap::new(),
             pending_stores: HashMap::new(),
@@ -834,11 +845,15 @@ impl App {
                         self.close_tab(tab);
                         self.apply(Action::ShowConnections);
                     } else {
-                        self.backend.send(Command::Close { session });
+                        let kept = self.keep_tabs(tab);
                         self.close_editors(tab);
                         if let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) {
                             entry.content = ConnTabContent::Picker(PickerState::default());
                         }
+                        if kept {
+                            self.save_session();
+                        }
+                        self.backend.send(Command::Close { session });
                     }
                 }
             }
@@ -2436,6 +2451,10 @@ impl App {
                     if self.table_lists.remove(&id.0).is_some() || unread {
                         self.save_table_lists();
                     }
+                    // And the tables its window last had.
+                    if self.session.forget(&id) {
+                        self.save_session();
+                    }
                     if removed.password == PasswordMode::Keyring {
                         self.store_secret(&id, false, None, false);
                     }
@@ -2628,11 +2647,11 @@ impl App {
         let Some(index) = self.tab_index(id) else {
             return;
         };
-        if let ConnTabContent::Workspace(workspace) = &self.tabs[index].content {
-            self.backend.send(Command::Close {
-                session: workspace.session,
-            });
-        }
+        let session = match &self.tabs[index].content {
+            ConnTabContent::Workspace(workspace) => Some(workspace.session),
+            ConnTabContent::Picker(_) => None,
+        };
+        let closes_window = self.keep_tabs(id);
         self.close_editors(id);
         self.tabs.remove(index);
         if self.tabs.is_empty() {
@@ -2644,6 +2663,75 @@ impl App {
         } else {
             self.active = self.active.min(self.tabs.len() - 1);
         }
+        if closes_window {
+            self.save_session();
+        }
+        if let Some(session) = session {
+            self.backend.send(Command::Close { session });
+        }
+    }
+
+    /// Keeps the table tabs of `tab`'s window in the session, for the
+    /// next time its connection opens: before the window closes, or has
+    /// another connection put in its place. Whether there was a window of
+    /// a saved connection to keep.
+    ///
+    /// A window switched to another database than its connection's keeps
+    /// nothing: the connection opens on its own, where those tables are
+    /// not, and what it kept of that one stays.
+    fn keep_tabs(&mut self, tab: ConnTabId) -> bool {
+        let Some(workspace) = self.workspace(tab) else {
+            return false;
+        };
+        let Some(saved) = self.connections.get(&workspace.conn_id) else {
+            return false;
+        };
+        let database = crate::session::database_of(&workspace.spec);
+        if database != crate::session::database_of(&saved.spec) {
+            return true;
+        }
+        let kept = crate::session::Kept::of(workspace);
+        let id = workspace.conn_id.clone();
+        self.session.keep(&id, kept);
+        true
+    }
+
+    /// Writes session.json on the backend: the connection tabs open now,
+    /// in order, the picker as none, and the tables of every window open,
+    /// with what is kept of the connections closed before. A connection
+    /// that is not saved (deleted while its window is open) is left out.
+    /// The window calls it as the app quits.
+    pub fn save_session(&mut self) {
+        if !self.writes_session {
+            return;
+        }
+        let ids: Vec<ConnTabId> = self.tabs.iter().map(|tab| tab.id).collect();
+        for id in &ids {
+            self.keep_tabs(*id);
+        }
+        let mut open = Vec::new();
+        let mut active = 0;
+        for (index, tab) in self.tabs.iter().enumerate() {
+            let entry = match &tab.content {
+                ConnTabContent::Picker(_) => None,
+                ConnTabContent::Workspace(workspace) => {
+                    if self.connections.get(&workspace.conn_id).is_none() {
+                        continue;
+                    }
+                    Some(workspace.conn_id.clone())
+                }
+            };
+            if index == self.active {
+                active = open.len();
+            }
+            open.push(entry);
+        }
+        self.session.open = open;
+        self.session.active = active;
+        self.backend.send(Command::Save {
+            path: self.dirs.session_file(),
+            file: StateFile::Session(self.session.clone()),
+        });
     }
 
     /// Opens `saved` in `tab`; `typed` holds secrets typed in the dialog.
@@ -2661,6 +2749,8 @@ impl App {
         let full_precision = self.settings.timestamps == crate::settings::Timestamps::Full;
         let zone = self.zone();
         let flat = self.lists_flat(&saved.id);
+        // The connection the window had closes in it.
+        let replaced = self.keep_tabs(tab);
         let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) else {
             return;
         };
@@ -2669,6 +2759,9 @@ impl App {
         workspace.zone = zone;
         workspace.tree.flat = flat;
         entry.content = ConnTabContent::Workspace(Box::new(workspace));
+        if replaced {
+            self.save_session();
+        }
     }
 
     /// Fills the tab's secrets one at a time (the database password, then
diff --git a/src/backend.rs b/src/backend.rs
index b6b0c75..1bda432 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -221,6 +221,8 @@ pub enum StateFile {
     /// How each connection lists its tables where its user chose, by id:
     /// a list's name, or a value a newer version wrote, kept as it was.
     TableLists(std::collections::BTreeMap<String, serde_json::Value>),
+    /// The connection tabs open, and each connection's tables.
+    Session(crate::session::Session),
 }
 
 impl StateFile {
@@ -236,6 +238,7 @@ impl StateFile {
             Self::Labels(labels) => labels.save(path),
             Self::AgentsShown(shown) => shown.save(path),
             Self::TableLists(lists) => crate::util::save_json(path, lists),
+            Self::Session(session) => session.save(path),
         }
     }
 }
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
index 895c607..2af06d9 100644
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -380,6 +380,8 @@ impl eframe::App for Window {
     }
 
     fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
+        // What is open, for the next launch.
+        self.app.save_session();
         // Connections and known hosts saved just before quitting still land.
         self.app.backend.flush(std::time::Duration::from_secs(5));
     }
diff --git a/src/paths.rs b/src/paths.rs
index 1c3aaca..0b99bfd 100644
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -115,6 +115,13 @@ impl AppDirs {
         self.state.join("last_used.json")
     }
 
+    /// The connection tabs open at the last quit, and each connection's
+    /// tables as its window last had them. Of this computer, and written
+    /// at quit and when a connection's tab closes.
+    pub fn session_file(&self) -> PathBuf {
+        self.state.join("session.json")
+    }
+
     /// The note that the connections are kept in `connections.toml`: made
     /// at the first start that finds that file. Once it is there, what an
     /// older version kept in `connections.json` is never read again,
diff --git a/src/session.rs b/src/session.rs
index 0069449..668f378 100644
--- a/src/session.rs
+++ b/src/session.rs
@@ -3,6 +3,355 @@
 //! directory. Of this computer, and no setting: what the next launch and
 //! the next open of a connection make of it is the settings' (`startup`).
 
+use std::collections::BTreeMap;
+use std::path::Path;
+
+use serde::{Deserialize, Serialize};
+use tabletist_db::{FilterOp, ObjectKind, ObjectRef, Sort, SortDir};
+
+use crate::connections::ConnectionId;
+use crate::model::{FilterBar, FilterRow, ObjectTab, TabId, Workspace};
+
+/// The version this one writes.
+const VERSION: u32 = 1;
+
+/// The file, as the app holds it.
+#[derive(Debug, Clone, PartialEq, Default)]
+pub struct Session {
+    /// The connection tabs open at quit, in the order the bar had them:
+    /// each a saved connection's id, or `None` for the picker.
+    pub open: Vec<Option<ConnectionId>>,
+    /// Which of `open` was in front.
+    pub active: usize,
+    /// Each saved connection's table tabs as its window last had them, by
+    /// its id. Changed through [`Session::keep`] and [`Session::forget`].
+    pub connections: BTreeMap<String, Kept>,
+    /// The connections' parts this version could not read, by id, as the
+    /// file had them: a newer version may have written them, and they are
+    /// written back until the connection's window closes again here.
+    unread: BTreeMap<String, serde_json::Value>,
+    /// The file's keys this version does not know, written back as they
+    /// were.
+    rest: BTreeMap<String, serde_json::Value>,
+}
+
+/// One connection's tables, as its window had them when it closed.
+#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
+#[serde(default)]
+pub struct Kept {
+    /// The database the tables are of ([`database_of`]): the one the
+    /// window had switched to, where it had.
+    pub database: String,
+    pub tabs: Vec<KeptTab>,
+    /// Which of `tabs` was in front. `None` where none was: a SQL editor,
+    /// or the Agents tab.
+    #[serde(skip_serializing_if = "Option::is_none")]
+    pub active: Option<usize>,
+    /// What a newer version wrote of the connection that this one does
+    /// not know: written back until the connection's window closes here.
+    #[serde(flatten)]
+    pub rest: BTreeMap<String, serde_json::Value>,
+}
+
+/// A table or a view, with its sort and its filter bar.
+#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
+pub struct KeptTab {
+    pub schema: String,
+    pub name: String,
+    /// `table`, `view` or `materialized_view`.
+    pub kind: String,
+    /// Each column the rows are sorted by, first to last, with `asc` or
+    /// `desc`.
+    #[serde(default, skip_serializing_if = "Vec::is_empty")]
+    pub sort: Vec<(String, String)>,
+    #[serde(default, skip_serializing_if = "KeptFilter::is_empty")]
+    pub filter: KeptFilter,
+    /// A preview tab is not pinned: the next single click replaces it.
+    #[serde(default = "pinned", skip_serializing_if = "is_pinned")]
+    pub pinned: bool,
+    /// What a newer version wrote of the tab that this one does not know.
+    #[serde(flatten)]
+    pub rest: BTreeMap<String, serde_json::Value>,
+}
+
+/// A filter bar as it was: its rows, and its WHERE text.
+#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
+#[serde(default)]
+pub struct KeptFilter {
+    #[serde(skip_serializing_if = "is_off")]
+    pub open: bool,
+    /// Each row: its column, its operator and its value.
+    #[serde(skip_serializing_if = "Vec::is_empty")]
+    pub rows: Vec<(String, String, String)>,
+    /// Whether the WHERE text is used.
+    #[serde(skip_serializing_if = "is_off")]
+    pub raw: bool,
+    #[serde(skip_serializing_if = "String::is_empty")]
+    pub raw_text: String,
+    /// What a newer version wrote of the bar that this one does not know.
+    #[serde(flatten)]
+    pub rest: BTreeMap<String, serde_json::Value>,
+}
+
+fn pinned() -> bool {
+    true
+}
+
+fn is_pinned(pinned: &bool) -> bool {
+    *pinned
+}
+
+fn is_off(on: &bool) -> bool {
+    !*on
+}
+
+impl KeptFilter {
+    fn is_empty(&self) -> bool {
+        *self == Self::default()
+    }
+}
+
+/// The database a connection's tables are of: its name, or a SQLite
+/// file's path, which names no database of its own.
+pub fn database_of(spec: &tabletist_db::ConnectSpec) -> String {
+    match spec.driver {
+        tabletist_db::Driver::Sqlite => spec
+            .sqlite_path
+            .as_ref()
+            .map(|path| path.display().to_string())
+            .unwrap_or_default(),
+        tabletist_db::Driver::Postgres | tabletist_db::Driver::MySql => spec.database.clone(),
+    }
+}
+
+/// The file as it is written and read, each part on its own: a value of
+/// the wrong kind costs itself, not the file.
+#[derive(Default, Serialize, Deserialize)]
+#[serde(default)]
+struct File {
+    version: serde_json::Value,
+    open: Vec<serde_json::Value>,
+    active: serde_json::Value,
+    connections: BTreeMap<String, serde_json::Value>,
+    #[serde(flatten)]
+    rest: BTreeMap<String, serde_json::Value>,
+}
+
+impl Session {
+    /// The file at `path`. A missing file is no session; one that cannot be
+    /// read is kept aside (`util::load_json`), and there is none either.
+    pub fn load(path: &Path) -> Self {
+        let file: File = crate::util::load_json(path);
+        let mut session = Self::default();
+        for (id, kept) in file.connections {
+            match serde_json::from_value(kept.clone()) {
+                Ok(kept) => drop(session.connections.insert(id, kept)),
+                Err(error) => {
+                    log::warn!("the tabs of {id} were not read: {error}");
+                    session.unread.insert(id, kept);
+                }
+            }
+        }
+        // A window that is neither an id nor the picker is left out.
+        session.open = file
+            .open
+            .into_iter()
+            .filter_map(|entry| match entry {
+                serde_json::Value::String(id) => Some(Some(ConnectionId(id))),
+                serde_json::Value::Null => Some(None),
+                _ => None,
+            })
+            .collect();
+        session.active = file
+            .active
+            .as_u64()
+            .and_then(|at| usize::try_from(at).ok())
+            .unwrap_or(0);
+        session.rest = file.rest;
+        session
+    }
+
+    /// Keeps `kept` as the tables of the connection `id`, in place of
+    /// what the file had of it.
+    pub fn keep(&mut self, id: &ConnectionId, kept: Kept) {
+        self.unread.remove(&id.0);
+        self.connections.insert(id.0.clone(), kept);
+    }
+
+    /// Forgets what is kept of the connection `id`. Whether there was
+    /// anything.
+    pub fn forget(&mut self, id: &ConnectionId) -> bool {
+        let unread = self.unread.remove(&id.0).is_some();
+        self.connections.remove(&id.0).is_some() || unread
+    }
+
+    pub fn save(&self, path: &Path) -> std::io::Result<()> {
+        let mut connections = self.unread.clone();
+        connections.extend(
+            self.connections
+                .iter()
+                .filter_map(|(id, kept)| Some((id.clone(), serde_json::to_value(kept).ok()?))),
+        );
+        let file = File {
+            version: VERSION.into(),
+            open: self
+                .open
+                .iter()
+                .map(|id| {
+                    id.as_ref()
+                        .map_or(serde_json::Value::Null, |id| id.0.clone().into())
+                })
+                .collect(),
+            active: self.active.into(),
+            connections,
+            rest: self.rest.clone(),
+        };
+        crate::util::save_json(path, &file)
+    }
+}
+
+impl Kept {
+    /// The table tabs of `workspace`, in strip order, and which was in
+    /// front. SQL editors and the Agents tab are not kept.
+    pub fn of(workspace: &Workspace) -> Self {
+        let objects: Vec<&ObjectTab> = workspace.object_tabs().collect();
+        Self {
+            database: database_of(&workspace.spec),
+            tabs: objects.iter().map(|object| KeptTab::of(object)).collect(),
+            active: objects
+                .iter()
+                .position(|object| Some(object.id) == workspace.active_tab),
+            rest: BTreeMap::new(),
+        }
+    }
+}
+
+impl KeptTab {
+    fn of(object: &ObjectTab) -> Self {
+        let bar = &object.filter;
+        Self {
+            schema: object.object.schema.clone(),
+            name: object.object.name.clone(),
+            kind: kind_name(object.kind).to_owned(),
+            sort: object
+                .query
+                .sort
+                .iter()
+                .map(|sort| (sort.column.clone(), dir_name(sort.dir).to_owned()))
+                .collect(),
+            filter: KeptFilter {
+                open: bar.open,
+                rows: bar
+                    .rows
+                    .iter()
+                    .map(|row| {
+                        let op = op_name(row.op).to_owned();
+                        (row.column.clone(), op, row.value.clone())
+                    })
+                    .collect(),
+                raw: bar.raw,
+                raw_text: bar.raw_text.clone(),
+                ..KeptFilter::default()
+            },
+            pinned: object.pinned,
+            rest: BTreeMap::new(),
+        }
+    }
+
+    /// The tab this one was, as `id`, paging by `page_size`: its sort, and
+    /// its filter bar with what the bar says applied. Nothing is loaded.
+    /// `None` for a kind this version does not know. A sort or a row whose
+    /// words it does not know is left out.
+    pub fn reopen(&self, id: TabId, page_size: u32) -> Option<ObjectTab> {
+        let kind = kind_of(&self.kind)?;
+        let object = ObjectRef::new(self.schema.clone(), self.name.clone());
+        let mut tab = ObjectTab::new(id, object, kind, self.pinned, page_size, None);
+        tab.query.sort = self
+            .sort
+            .iter()
+            .filter_map(|(column, dir)| {
+                Some(Sort {
+                    column: column.clone(),
+                    dir: dir_of(dir)?,
+                })
+            })
+            .collect();
+        let filter = &self.filter;
+        tab.filter = FilterBar {
+            open: filter.open,
+            rows: filter
+                .rows
+                .iter()
+                .filter_map(|(column, op, value)| {
+                    Some(FilterRow {
+                        column: column.clone(),
+                        op: op_of(op)?,
+                        value: value.clone(),
+                    })
+                })
+                .collect(),
+            raw: filter.raw,
+            raw_text: filter.raw_text.clone(),
+            ..FilterBar::default()
+        };
+        (tab.query.filters, tab.query.raw_where) = tab.filter.to_query();
+        Some(tab)
+    }
+}
+
+fn kind_name(kind: ObjectKind) -> &'static str {
+    match kind {
+        ObjectKind::Table => "table",
+        ObjectKind::View => "view",
+        ObjectKind::MaterializedView => "materialized_view",
+    }
+}
+
+fn kind_of(name: &str) -> Option<ObjectKind> {
+    [
+        ObjectKind::Table,
+        ObjectKind::View,
+        ObjectKind::MaterializedView,
+    ]
+    .into_iter()
+    .find(|kind| kind_name(*kind) == name)
+}
+
+fn dir_name(dir: SortDir) -> &'static str {
+    match dir {
+        SortDir::Asc => "asc",
+        SortDir::Desc => "desc",
+    }
+}
+
+fn dir_of(name: &str) -> Option<SortDir> {
+    [SortDir::Asc, SortDir::Desc]
+        .into_iter()
+        .find(|dir| dir_name(*dir) == name)
+}
+
+/// An operator as the file writes it: its own word, not what the bar
+/// shows, which can be translated.
+fn op_name(op: FilterOp) -> &'static str {
+    match op {
+        FilterOp::Eq => "eq",
+        FilterOp::Ne => "ne",
+        FilterOp::Lt => "lt",
+        FilterOp::Gt => "gt",
+        FilterOp::Le => "le",
+        FilterOp::Ge => "ge",
+        FilterOp::Contains => "contains",
+        FilterOp::StartsWith => "starts_with",
+        FilterOp::IsNull => "is_null",
+        FilterOp::IsNotNull => "is_not_null",
+        FilterOp::In => "in",
+    }
+}
+
+fn op_of(name: &str) -> Option<FilterOp> {
+    FilterOp::ALL.into_iter().find(|op| op_name(*op) == name)
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_reference/settings-and-files.md src/app.rs src/app/startup_tests.rs src/backend.rs src/entrypoint.rs src/lib.rs src/paths.rs src/session.rs
git commit -m "Keep the open windows and each connection's tables in session.json"
```

---

### Task 2: Restore tabs and filters

The key `startup.restore_tabs`, the Startup group's heading and its row in both looks, over the Data group, and the tab's lead with "starts" in it. A connection that opens with the option on reopens what its window last had (`App::reopen_tabs`, from `open_workspace`). The Omarchy screen's cursor opens on the new first row: the screen's tests start on Rows per page (`settings_screen_in` moves there, and `settings_cursor` counts from it), and the zone's tests find their row by its place in `SettingsPane::rows`.

**Files:** Modify `docs/_reference/settings-and-files.md`; Modify `src/app.rs`; Modify `src/app/startup_tests.rs`; Modify `src/model.rs`; Modify `src/session.rs`; Modify `src/settings.rs`; Modify `src/ui/mod.rs`; Modify `src/ui/settings/mod.rs`; Modify `src/ui/settings/sheet.rs`; Modify `src/ui/settings/terminal.rs`; Modify `src/ui/zone_tests.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index aeebe06..78f1824 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -9439,6 +9439,7 @@ mod tests {
             settings.production_read_only = false;
             settings.confirm_close_edits = false;
             settings.idle_disconnect = crate::settings::Idle::Never;
+            settings.restore_tabs = false;
             settings.show_system_schemas = true;
             settings.sql_limit = 50;
         });
@@ -9465,6 +9466,8 @@ mod tests {
         assert!(app.settings.production_read_only);
         assert!(app.settings.confirm_close_edits);
         assert_eq!(app.settings.idle_disconnect, defaults.idle_disconnect);
+        // And the Startup group's.
+        assert!(app.settings.restore_tabs);
         assert!(app.settings.show_system_schemas);
         assert_eq!(app.settings.sql_limit, 50);
         assert_eq!(app.settings_file.text, app.settings.to_toml());
diff --git a/src/app/startup_tests.rs b/src/app/startup_tests.rs
index 7509468..bf362e0 100644
--- a/src/app/startup_tests.rs
+++ b/src/app/startup_tests.rs
@@ -268,3 +268,185 @@ fn a_window_switched_to_another_database_keeps_what_its_connection_had() {
         .collect();
     assert_eq!(names, ["books"]);
 }
+
+/// Says the newest connect is up, as the backend does.
+fn connected(app: &mut App) {
+    let (session, request) = app
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
+    app.apply(Action::Backend(crate::backend::Event::Connected {
+        session,
+        request,
+        driver: tabletist_db::Driver::Sqlite,
+        encrypted: false,
+        access: crate::testing::asked_access(app),
+    }));
+}
+
+/// The pages asked for since the `from`th command, by table.
+fn fetched(app: &App, from: usize) -> Vec<tabletist_db::RowQuery> {
+    app.backend.sent[from..]
+        .iter()
+        .filter_map(|command| match command {
+            Command::FetchRows { query, .. } => Some(query.clone()),
+            _ => None,
+        })
+        .collect()
+}
+
+/// Bookshop's window with `books` sorted by title and `authors` filtered
+/// to Ireland, `books` in front, closed: what the session keeps of it.
+fn closed_with_tables(app: &mut App) -> crate::connections::ConnectionId {
+    let conn = bookshop(app, "Bookshop");
+    let tab = open_window(app, &conn);
+    connected(app);
+    let titles = books(app, tab, "books");
+    app.apply(Action::SortBy {
+        tab,
+        object_tab: titles,
+        column: "title".into(),
+    });
+    let authors = books(app, tab, "authors");
+    if let Some(object) = app.object_tab_mut(tab, authors) {
+        object.filter.rows.push(crate::model::FilterRow {
+            column: "country".into(),
+            op: FilterOp::Eq,
+            value: "IE".into(),
+        });
+    }
+    app.apply(Action::ApplyFilters {
+        tab,
+        object_tab: authors,
+    });
+    app.apply(Action::ActivateTab { tab, id: titles });
+    app.apply(Action::CloseConnTab(tab));
+    conn
+}
+
+#[test]
+fn a_connection_opened_again_reopens_its_tables_and_reads_the_one_in_front() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = closed_with_tables(&mut app);
+    let tab = open_window(&mut app, &conn);
+    app.apply_actions();
+    let workspace = app.workspace(tab).unwrap();
+    let names: Vec<&str> = workspace
+        .object_tabs()
+        .map(|object| object.object.name.as_str())
+        .collect();
+    assert_eq!(names, ["books", "authors"]);
+    let front = workspace.active_object_tab().unwrap();
+    assert_eq!(front.object.name, "books");
+    let authors = workspace.object_tabs().nth(1).unwrap().id;
+    let before = app.backend.sent.len();
+    connected(&mut app);
+    app.apply_actions();
+    let pages = fetched(&app, before);
+    assert_eq!(pages.len(), 1, "{pages:?}");
+    assert_eq!(pages[0].object.name, "books");
+    assert_eq!(pages[0].sort[0].column, "title");
+    assert_eq!(pages[0].sort[0].dir, SortDir::Asc);
+    // The other is read when it is first shown, with its filter.
+    let before = app.backend.sent.len();
+    app.apply(Action::ActivateTab { tab, id: authors });
+    app.apply_actions();
+    let pages = fetched(&app, before);
+    assert_eq!(pages.len(), 1, "{pages:?}");
+    assert_eq!(pages[0].object.name, "authors");
+    assert_eq!(pages[0].filters[0].column, "country");
+    assert_eq!(pages[0].filters[0].value, "IE");
+    let object = app.workspace(tab).unwrap().object_tab(authors).unwrap();
+    assert_eq!(object.filter.rows.len(), 1);
+    // And only then.
+    let before = app.backend.sent.len();
+    app.apply_actions();
+    assert!(fetched(&app, before).is_empty());
+}
+
+#[test]
+fn with_restore_off_a_connection_opens_with_no_table() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = closed_with_tables(&mut app);
+    app.apply(Action::SetOption(
+        crate::settings::OptionValue::RestoreTabs(false),
+    ));
+    assert!(
+        app.settings_file
+            .text
+            .contains("[startup]\nrestore_tabs = false\n")
+    );
+    let tab = open_window(&mut app, &conn);
+    assert_eq!(app.workspace(tab).unwrap().tabs.len(), 0);
+    // What it had is still kept, for when the option is on again.
+    assert_eq!(app.session.connections[&conn.0].tabs.len(), 2);
+}
+
+#[test]
+fn tables_of_another_database_are_not_reopened() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = closed_with_tables(&mut app);
+    app.session.connections.get_mut(&conn.0).unwrap().database = "bookshop_preview".into();
+    let tab = open_window(&mut app, &conn);
+    assert_eq!(app.workspace(tab).unwrap().tabs.len(), 0);
+}
+
+#[test]
+fn a_reopened_table_that_is_gone_says_why_as_an_open_one_does() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = closed_with_tables(&mut app);
+    let tab = open_window(&mut app, &conn);
+    connected(&mut app);
+    app.apply_actions();
+    let (session, request) = app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|command| match command {
+            Command::FetchRows {
+                session, request, ..
+            } => Some((*session, *request)),
+            _ => None,
+        })
+        .unwrap();
+    app.apply(Action::Backend(crate::backend::Event::Rows {
+        session,
+        request,
+        result: Err(tabletist_db::Error::Query {
+            code: None,
+            message: "no such table: books".into(),
+            detail: None,
+            hint: None,
+            named: Default::default(),
+        }),
+    }));
+    let front = app.workspace(tab).unwrap().active_object_tab().unwrap();
+    assert!(front.rows.shown_error().is_some());
+    // The tab stays, and keeps what it was for the next time.
+    assert_eq!(front.query.sort[0].column, "title");
+}
+
+#[test]
+fn tables_of_another_sqlite_file_are_not_reopened() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = closed_with_tables(&mut app);
+    let mut saved = app.connections.get(&conn).unwrap().clone();
+    saved.spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop_archive.db");
+    app.connections.upsert(saved);
+    let tab = open_window(&mut app, &conn);
+    assert_eq!(app.workspace(tab).unwrap().tabs.len(), 0);
+}
diff --git a/src/settings.rs b/src/settings.rs
index ede06c4..c3f0852 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1920,6 +1920,7 @@ mod tests {
     /// Every key away from its default.
     fn all_set() -> Settings {
         Settings {
+            restore_tabs: false,
             page_size: 500,
             timestamps: Timestamps::Full,
             tz: Zone::Named("America/New_York".into()),
@@ -1952,6 +1953,9 @@ mod tests {
             format!(
                 "\
 # written by tabletist, safe to edit by hand
+[startup]
+restore_tabs = false
+
 [data]
 page_size = 500
 timestamps = \"full\"  # second | full
@@ -2048,26 +2052,27 @@ bar = \"label\"  # tint | label
         assert_eq!(
             loaded.lines,
             vec![
-                (Key::PageSize, 3),
-                (Key::Timestamps, 4),
-                (Key::Tz, 5),
-                (Key::GroupDigits, 6),
-                (Key::ValueTags, 7),
-                (Key::ProductionReadOnly, 10),
-                (Key::ConfirmCloseEdits, 11),
-                (Key::IdleDisconnect, 12),
-                (Key::ShowSystemSchemas, 15),
-                (Key::SqlLimit, 18),
-                (Key::SqlTimeoutSecs, 19),
-                (Key::McpEnabled, 22),
-                (Key::McpLogDays, 23),
-                (Key::McpNotify, 24),
-                (Key::Theme, 27),
-                (Key::RowHeight, 30),
-                (Key::Zebra, 31),
-                (Key::ColumnTypes, 32),
-                (Key::TableList, 33),
-                (Key::EnvBar, 36),
+                (Key::RestoreTabs, 3),
+                (Key::PageSize, 6),
+                (Key::Timestamps, 7),
+                (Key::Tz, 8),
+                (Key::GroupDigits, 9),
+                (Key::ValueTags, 10),
+                (Key::ProductionReadOnly, 13),
+                (Key::ConfirmCloseEdits, 14),
+                (Key::IdleDisconnect, 15),
+                (Key::ShowSystemSchemas, 18),
+                (Key::SqlLimit, 21),
+                (Key::SqlTimeoutSecs, 22),
+                (Key::McpEnabled, 25),
+                (Key::McpLogDays, 26),
+                (Key::McpNotify, 27),
+                (Key::Theme, 30),
+                (Key::RowHeight, 33),
+                (Key::Zebra, 34),
+                (Key::ColumnTypes, 35),
+                (Key::TableList, 36),
+                (Key::EnvBar, 39),
             ]
         );
     }
@@ -3078,6 +3083,7 @@ bar = \"label\"  # tint | label
         assert_eq!(
             keys,
             vec![
+                Key::RestoreTabs,
                 Key::PageSize,
                 Key::Timestamps,
                 Key::Tz,
@@ -3106,7 +3112,12 @@ bar = \"label\"  # tint | label
         .concat();
         // The sections of the General and Appearance panes hold their
         // options, in order.
-        let sections = [&OptionId::DATA[..], &OptionId::SAFETY[..]].concat();
+        let sections = [
+            &OptionId::STARTUP[..],
+            &OptionId::DATA[..],
+            &OptionId::SAFETY[..],
+        ]
+        .concat();
         assert_eq!(sections, OptionId::GENERAL);
         let sections = [
             &OptionId::THEME[..],
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 61aecee..7108c82 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -1070,10 +1070,15 @@ mod tests {
         settings_screen_in(Harness::with_size(egui::vec2(1920.0, 1080.0)))
     }
 
+    /// The cursor on the first Data row, Rows per page: the rows of the
+    /// Startup group are over it, and have tests of their own.
     fn settings_screen_in(mut harness: Harness) -> Harness {
         harness.set_look(crate::theme::Look::omarchy());
         harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
         assert!(settings_open(&harness));
+        for _ in crate::settings::OptionId::STARTUP {
+            harness.press(egui::Key::J, egui::Modifiers::NONE);
+        }
         harness
     }
 
@@ -1105,7 +1110,16 @@ value_tags   = true
         matches!(harness.app.dialog, Some(crate::model::Dialog::Settings(_)))
     }
 
+    /// The cursor's row, counted from Rows per page, the first of the
+    /// Data group: the tests of the rows over it read [`settings_row`].
     fn settings_cursor(harness: &Harness) -> usize {
+        settings_row(harness)
+            .checked_sub(crate::settings::OptionId::STARTUP.len())
+            .expect("the cursor is under the Startup group")
+    }
+
+    /// The cursor's row, counted from the first.
+    fn settings_row(harness: &Harness) -> usize {
         match &harness.app.dialog {
             Some(crate::model::Dialog::Settings(dialog)) => dialog.row,
             _ => panic!("the Settings screen is not open"),
@@ -1301,7 +1315,7 @@ value_tags   = true
             for text in [
                 "Settings",
                 "General",
-                "How Tabletist shows data and protects your work. Changes apply right away.",
+                "How Tabletist starts, shows data and protects your work. Changes apply right away.",
                 "Rows per page",
                 "Table view; the SQL editor has its own limit",
                 "Timestamps",
@@ -1355,6 +1369,48 @@ value_tags   = true
         }
     }
 
+    #[test]
+    fn the_startup_group_of_the_sheet_comes_first_and_sets_whether_tables_reopen() {
+        for look in SHEET_LOOKS {
+            let mut harness = settings_sheet(look());
+            for text in [
+                "STARTUP",
+                "Restore tabs and filters",
+                "Per connection, including sort and hidden columns",
+            ] {
+                assert!(harness.has(text), "{text}");
+            }
+            // Over the Data group.
+            let startup = harness.painted_rect("STARTUP").expect("the heading");
+            let data = harness.painted_rect("DATA").expect("the heading");
+            assert!(startup.bottom() < data.top(), "{startup:?} {data:?}");
+            harness.click("Restore tabs and filters");
+            assert!(!harness.app.settings.restore_tabs);
+            let text = &harness.app.settings_file.text;
+            assert!(
+                text.ends_with("[startup]\nrestore_tabs = false\n"),
+                "{text}"
+            );
+        }
+    }
+
+    #[test]
+    fn the_startup_group_of_the_screen_has_its_row_first_and_space_flips_it() {
+        let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+        harness.set_look(crate::theme::Look::omarchy());
+        harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
+        for text in ["startup", "per connection"] {
+            assert!(harness.painted_color(text).is_some(), "{text}");
+        }
+        // The cursor opens on it.
+        assert!(harness.painted_color("▌restore tabs and filters").is_some());
+        harness.press(egui::Key::Space, egui::Modifiers::NONE);
+        assert!(!harness.app.settings.restore_tabs);
+        assert!(harness.painted_color("restore_tabs = ").is_some());
+        harness.press(egui::Key::R, egui::Modifiers::SHIFT);
+        assert!(harness.app.settings.restore_tabs);
+    }
+
     #[test]
     fn the_safety_group_of_the_sheet_sets_its_three_options() {
         for look in SHEET_LOOKS {
@@ -1450,7 +1506,8 @@ value_tags   = true
                 )
             })
             .expect("the row");
-        for _ in 0..row {
+        // From Rows per page, where the screen's tests start.
+        for _ in crate::settings::OptionId::STARTUP.len()..row {
             harness.press(egui::Key::J, egui::Modifiers::NONE);
         }
         assert!(
@@ -1674,7 +1731,9 @@ value_tags   = true
             // the first of them.
             let stored = harness.painted_rect("Stored in").expect("the footer");
             assert!(stored.bottom() < size.y, "{stored:?}");
-            let row = harness.painted_rect("Rows per page").expect("a row");
+            let row = harness
+                .painted_rect("Restore tabs and filters")
+                .expect("a row");
             assert!(row.bottom() < stored.top(), "{row:?} {stored:?}");
             harness.click("Export…");
             assert_eq!(harness.app.backend.saves.len(), 1);
@@ -1701,7 +1760,7 @@ value_tags   = true
             assert!(window.contains_rect(control), "{control:?}");
             let stored = harness.painted_rect("Stored in").expect("the footer");
             assert!(control.bottom() < stored.top(), "{control:?} {stored:?}");
-            let lead = "How Tabletist shows data and protects your work. Changes apply right away.";
+            let lead = "How Tabletist starts, shows data and protects your work. Changes apply right away.";
             let lead = harness.painted_rect(lead).expect("the tab's lead");
             assert!(control.top() > lead.bottom(), "{control:?} {lead:?}");
             // And back up with it: the first row's menu.
@@ -1847,11 +1906,15 @@ value_tags   = true
         assert_eq!(settings_cursor(&harness), 2);
         harness.press(egui::Key::K, egui::Modifiers::NONE);
         harness.press(egui::Key::ArrowUp, egui::Modifiers::NONE);
-        harness.press(egui::Key::K, egui::Modifiers::NONE);
         assert_eq!(settings_cursor(&harness), 0);
         // The cursor's row says so.
         assert!(harness.painted_color("▌rows per page").is_some());
         assert!(harness.painted_color("timestamps").is_some());
+        // Up past the first row, it stays there.
+        for _ in 0..=crate::settings::OptionId::STARTUP.len() {
+            harness.press(egui::Key::K, egui::Modifiers::NONE);
+        }
+        assert_eq!(settings_row(&harness), 0);
     }
 
     #[test]
@@ -1997,7 +2060,7 @@ value_tags   = true
         // stays where it was.
         harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
         harness.press(egui::Key::J, egui::Modifiers::NONE);
-        assert_eq!(settings_cursor(&harness), 1);
+        assert_eq!(settings_row(&harness), 1);
         assert_eq!(cursor(&harness), under);
     }
 
@@ -2293,7 +2356,7 @@ value_tags   = true
         let mut harness = Harness::with_size(egui::vec2(1000.0, 700.0));
         harness.set_look(crate::theme::Look::omarchy());
         harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
-        assert!(harness.painted_color("▌rows per page").is_some());
+        assert!(harness.painted_color("▌restore tabs and filters").is_some());
         assert!(harness.painted_color("[data]").is_none());
     }
 
diff --git a/src/ui/zone_tests.rs b/src/ui/zone_tests.rs
index affbbdc..a2f3d0c 100644
--- a/src/ui/zone_tests.rs
+++ b/src/ui/zone_tests.rs
@@ -23,6 +23,18 @@ fn zone(harness: &mut Harness, zone: Zone) {
     harness.settle();
 }
 
+/// Moves the Settings screen's cursor from the first row to the zone's.
+fn to_the_zone(harness: &mut Harness) {
+    let row = crate::model::SettingsPane::General
+        .rows()
+        .iter()
+        .position(|row| *row == crate::model::SettingsRow::Option(crate::settings::OptionId::Tz))
+        .expect("the zone's row");
+    for _ in 0..row {
+        harness.press(Key::J, Modifiers::NONE);
+    }
+}
+
 fn new_york() -> Zone {
     Zone::Named("America/New_York".into())
 }
@@ -417,9 +429,7 @@ fn the_file_pane_marks_the_zones_line() {
             text: "[data]\npage_size = 500\ntz = \"utc\"\n".into(),
             own: false,
         }));
-    for _ in 0..2 {
-        harness.press(Key::J, Modifiers::NONE);
-    }
+    to_the_zone(&mut harness);
     let selection = harness.app.palette.selection;
     let line = harness.painted_rect("tz = ").expect("the zone's line");
     assert!(harness.fills.iter().any(|(rect, color)| {
@@ -440,9 +450,7 @@ fn a_step_off_the_files_zone_can_step_back_to_it() {
             text: "[data]\ntz = \"America/New_York\"\n".into(),
             own: false,
         }));
-    for _ in 0..2 {
-        harness.press(Key::J, Modifiers::NONE);
-    }
+    to_the_zone(&mut harness);
     harness.press(Key::H, Modifiers::NONE);
     assert_eq!(harness.app.settings.tz, Zone::Utc);
     harness.press(Key::L, Modifiers::NONE);
@@ -594,9 +602,7 @@ fn a_zone_the_database_turned_down_is_no_step_of_the_terminal_screen() {
     assert_eq!(harness.app.settings.tz, Zone::System);
     assert_eq!(harness.app.named_zone, None);
     harness.press(Key::Comma, Modifiers::COMMAND);
-    for _ in 0..2 {
-        harness.press(Key::J, Modifiers::NONE);
-    }
+    to_the_zone(&mut harness);
     harness.press(Key::L, Modifiers::NONE);
     assert_eq!(harness.app.settings.tz, Zone::Utc);
     // From UTC there is no step further: the refused name is not one.
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` no variant, associated function, or constant named `STARTUP` found for enum `settings::OptionId` in the current scope `` (5 times); `` no field `restore_tabs` on type `settings::Settings` `` (4 times); `` no variant, associated function, or constant named `RestoreTabs` found for enum `settings::Key` in the current scope `` (2 times); `` no field `restore_tabs` on type `&mut settings::Settings` ``, and 2 more.

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index a2d911d..306ba6e 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -12,6 +12,7 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 
 | Option | Choices | Default |
 | --- | --- | --- |
+| **Restore tabs and filters** | On or off. On, a connection that opens, at launch or from Connections, opens again the table and view tabs its window had when it last closed on that database (or SQLite file), each with its sort and filters; the one in front loads, and each other when you first show it. SQL editors are not opened again. Off, a connection opens with its tables in the sidebar and no tab | On |
 | **Rows per page** | 100, 500, 1,000, 5,000 or 10,000. A table takes a new size when it next loads, not while you page through it | 1,000 |
 | **Timestamps** | To the second, or full precision | To the second |
 | **Time zone for timestamptz** | System, UTC or any zone. A PostgreSQL `timestamptz` is shown in this zone, a copy gives the value as the server sent it, and a value you type without an offset is sent with this zone's. In the Omarchy look `h`/`l` choose System, UTC or the last zone the file named; any other zone is set in the file | System |
@@ -61,6 +62,9 @@ is every key at its default (`group_digits` has none of its own: it
 follows the look):
 
 ```toml
+[startup]
+restore_tabs = true
+
 [data]
 page_size = 1000
 timestamps = "second"
@@ -95,6 +99,7 @@ bar = "tint"
 
 | Key | Values | Default |
 | --- | --- | --- |
+| `startup.restore_tabs` | `true` or `false`. Whether a connection opens again the table tabs, sorts and filters its window last had, kept in `session.json` | `true` |
 | `data.page_size` | 10 to 10000. A table takes a new size when it next loads (opened, refreshed, sorted, filtered or reconnected), not while you page through it | `1000` |
 | `data.timestamps` | `"second"` or `"full"` | `"second"` |
 | `data.tz` | `"system"`, `"utc"` or a zone such as `"America/New_York"`. A name the zone database does not know is a line that cannot be read. A PostgreSQL `timestamptz` value is shown and edited in this zone, a copy gives it as the server sent it, and one you type without an offset is sent with this zone's | `"system"` |
diff --git a/src/app.rs b/src/app.rs
index 78f1824..5fcc490 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -717,6 +717,7 @@ impl App {
                 self.apply(action);
             }
         }
+        self.read_reopened();
         self.format_rows();
         self.make_reviews();
     }
@@ -2749,6 +2750,17 @@ impl App {
         let full_precision = self.settings.timestamps == crate::settings::Timestamps::Full;
         let zone = self.zone();
         let flat = self.lists_flat(&saved.id);
+        // The tables the connection's window last had, where the settings
+        // reopen them and they are of the database it opens on.
+        let kept = self
+            .session
+            .connections
+            .get(&saved.id.0)
+            .filter(|kept| {
+                self.settings.restore_tabs
+                    && kept.database == crate::session::database_of(&saved.spec)
+            })
+            .cloned();
         // The connection the window had closes in it.
         let replaced = self.keep_tabs(tab);
         let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) else {
@@ -2759,11 +2771,62 @@ impl App {
         workspace.zone = zone;
         workspace.tree.flat = flat;
         entry.content = ConnTabContent::Workspace(Box::new(workspace));
+        if let Some(kept) = kept {
+            self.reopen_tabs(tab, &kept);
+        }
         if replaced {
             self.save_session();
         }
     }
 
+    /// Puts the tables `kept` in `tab`'s window, in their order, with
+    /// their sort and filters, the one that was in front in front again
+    /// (the first where none was). Nothing is read here: the one in front
+    /// is read with the connect, and each other when it is first shown.
+    fn reopen_tabs(&mut self, tab: ConnTabId, kept: &crate::session::Kept) {
+        let page_size = self.settings.page_size;
+        let mut opened = Vec::new();
+        for saved in &kept.tabs {
+            let id = TabId(self.next_id());
+            opened.extend(saved.reopen(id, page_size));
+        }
+        let front = kept
+            .active
+            .and_then(|at| kept.tabs.get(at))
+            .map(|front| ObjectRef::new(front.schema.clone(), front.name.clone()));
+        let Some(workspace) = self.workspace_mut(tab) else {
+            return;
+        };
+        let active = opened
+            .iter()
+            .find(|object| Some(&object.object) == front.as_ref())
+            .or(opened.first())
+            .map(|object| object.id);
+        workspace.tabs.extend(
+            opened
+                .into_iter()
+                .map(|object| Tab::Object(Box::new(object))),
+        );
+        workspace.active_tab = active;
+    }
+
+    /// Reads the table in front of each window that was reopened and not
+    /// read yet: its structure and its first page, as when it opened. A
+    /// connect under way reads it once it is up (`App::wake_to_read`).
+    fn read_reopened(&mut self) {
+        let due: Vec<(ConnTabId, TabId)> = self
+            .open_connections()
+            .filter_map(|(tab, workspace)| {
+                let front = workspace.active_object_tab()?;
+                front.reopened.then_some((tab, front.id))
+            })
+            .collect();
+        for (tab, id) in due {
+            self.describe(tab, id);
+            self.fetch_rows(tab, id);
+        }
+    }
+
     /// Fills the tab's secrets one at a time (the database password, then
     /// the SSH secret), each from the keyring or a prompt, then connects.
     fn authenticate(&mut self, tab: ConnTabId) {
@@ -4516,6 +4579,10 @@ impl App {
                     return (object.id, false, false, count);
                 }
                 let is_active = Some(object.id) == active;
+                // Reopened behind another: read when it is first shown.
+                if object.reopened && !is_active {
+                    return (object.id, false, false, count);
+                }
                 (
                     object.id,
                     is_active
@@ -4566,7 +4633,7 @@ impl App {
         }
         let waited: Vec<(TabId, bool, bool)> = workspace
             .object_tabs()
-            .filter(|object| !object.edits.holds())
+            .filter(|object| !object.edits.holds() && !object.reopened)
             .map(|object| {
                 (
                     object.id,
@@ -5609,6 +5676,9 @@ impl App {
         let size = self.settings.page_size;
         if let Some(object) = self.object_tab_mut(tab, id) {
             object.page_size = size;
+            // Read now, whatever asked: a reopened tab is not read again
+            // when it is shown.
+            object.reopened = false;
         }
         self.fetch_page(tab, id, size);
     }
diff --git a/src/model.rs b/src/model.rs
index 0eeed61..b46ca91 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2555,6 +2555,7 @@ impl SettingsPane {
         use crate::settings::OptionId;
         match self {
             Self::General => &[
+                SettingsRow::Option(OptionId::RestoreTabs),
                 SettingsRow::Option(OptionId::PageSize),
                 SettingsRow::Option(OptionId::Timestamps),
                 SettingsRow::Option(OptionId::Tz),
@@ -3158,6 +3159,10 @@ pub struct ObjectTab {
     /// the Structure view. A picker's chooser is the picker's own
     /// (`key_picker::Picker::chooser`).
     pub label_chooser: Option<crate::labels::Chooser>,
+    /// Reopened from what the window last had (`session.json`), and not
+    /// read yet: it is read when it is first in front
+    /// (`App::read_reopened`), not with the connect.
+    pub reopened: bool,
 }
 
 /// The row panel's text for one row, formatted once when the selection or
@@ -3378,6 +3383,7 @@ impl ObjectTab {
             yanked: None,
             structure_cursor: 0,
             label_chooser: None,
+            reopened: false,
         }
     }
 
diff --git a/src/session.rs b/src/session.rs
index 668f378..6d923d0 100644
--- a/src/session.rs
+++ b/src/session.rs
@@ -295,6 +295,7 @@ impl KeptTab {
             ..FilterBar::default()
         };
         (tab.query.filters, tab.query.raw_where) = tab.filter.to_query();
+        tab.reopened = true;
         Some(tab)
     }
 }
diff --git a/src/settings.rs b/src/settings.rs
index c3f0852..73f6777 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -377,6 +377,9 @@ impl Idle {
 #[derive(Clone, Debug, PartialEq, Deserialize)]
 #[serde(default)]
 pub struct Settings {
+    /// Whether a connection that opens reopens the table tabs its window
+    /// last had, with their sort and filters (`session.json`).
+    pub restore_tabs: bool,
     /// Rows fetched per page in the data grid.
     pub page_size: u32,
     /// Timestamps in grid cells: to the second, or as the server sent them.
@@ -439,6 +442,7 @@ pub struct Settings {
 /// A key of the settings file.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum Key {
+    RestoreTabs,
     PageSize,
     Timestamps,
     Tz,
@@ -463,7 +467,8 @@ pub enum Key {
 
 impl Key {
     /// In the order the file has them: a table's keys together.
-    pub const ALL: [Key; 20] = [
+    pub const ALL: [Key; 21] = [
+        Self::RestoreTabs,
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
@@ -489,6 +494,7 @@ impl Key {
     /// The key's table and its name there.
     pub fn path(self) -> (&'static str, &'static str) {
         match self {
+            Self::RestoreTabs => ("startup", "restore_tabs"),
             Self::PageSize => ("data", "page_size"),
             Self::Timestamps => ("data", "timestamps"),
             Self::Tz => ("data", "tz"),
@@ -538,6 +544,8 @@ impl Key {
 /// An option the Settings window shows: one row of it.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum OptionId {
+    /// Whether a connection reopens its tables: `startup.restore_tabs`.
+    RestoreTabs,
     PageSize,
     Timestamps,
     /// The zone a `timestamptz` is shown in: `data.tz`.
@@ -569,7 +577,8 @@ pub enum OptionId {
 impl OptionId {
     /// Every option, the General pane's, the Appearance pane's and then
     /// the Agents pane's.
-    pub const ALL: [OptionId; 17] = [
+    pub const ALL: [OptionId; 18] = [
+        Self::RestoreTabs,
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
@@ -589,7 +598,8 @@ impl OptionId {
         Self::AgentsNotify,
     ];
     /// The General pane's, in the order it has them.
-    pub const GENERAL: [OptionId; 8] = [
+    pub const GENERAL: [OptionId; 9] = [
+        Self::RestoreTabs,
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
@@ -599,7 +609,8 @@ impl OptionId {
         Self::ConfirmCloseEdits,
         Self::IdleDisconnect,
     ];
-    /// The General pane's sections: Data, then Safety.
+    /// The General pane's sections: Startup, Data, then Safety.
+    pub const STARTUP: [OptionId; 1] = [Self::RestoreTabs];
     pub const DATA: [OptionId; 5] = [
         Self::PageSize,
         Self::Timestamps,
@@ -636,6 +647,7 @@ impl OptionId {
     /// The key of the file the option is stored under.
     pub fn key(self) -> Key {
         match self {
+            Self::RestoreTabs => Key::RestoreTabs,
             Self::PageSize => Key::PageSize,
             Self::Timestamps => Key::Timestamps,
             Self::Tz => Key::Tz,
@@ -665,6 +677,7 @@ impl OptionId {
 /// An option with a value: what a key, a click or a reset sets.
 #[derive(Clone, Debug, PartialEq, Eq)]
 pub enum OptionValue {
+    RestoreTabs(bool),
     PageSize(u32),
     Timestamps(Timestamps),
     Tz(Zone),
@@ -689,6 +702,7 @@ pub enum OptionValue {
 impl OptionValue {
     pub fn option(&self) -> OptionId {
         match self {
+            Self::RestoreTabs(_) => OptionId::RestoreTabs,
             Self::PageSize(_) => OptionId::PageSize,
             Self::Timestamps(_) => OptionId::Timestamps,
             Self::Tz(_) => OptionId::Tz,
@@ -712,6 +726,7 @@ impl OptionValue {
     /// Puts the value in `settings`.
     pub fn set(&self, settings: &mut Settings) {
         match self.clone() {
+            Self::RestoreTabs(on) => settings.restore_tabs = on,
             Self::PageSize(size) => settings.page_size = size,
             Self::Timestamps(choice) => settings.timestamps = choice,
             Self::Tz(zone) => settings.tz = zone,
@@ -1215,6 +1230,7 @@ impl Settings {
     /// What `option` is set to.
     pub fn value(&self, option: OptionId) -> OptionValue {
         match option {
+            OptionId::RestoreTabs => OptionValue::RestoreTabs(self.restore_tabs),
             OptionId::PageSize => OptionValue::PageSize(self.page_size),
             OptionId::Timestamps => OptionValue::Timestamps(self.timestamps),
             OptionId::Tz => OptionValue::Tz(self.tz.clone()),
@@ -1274,6 +1290,7 @@ impl Settings {
             OptionId::ValueTags => OptionValue::ValueTags(forward),
             OptionId::ProductionReadOnly => OptionValue::ProductionReadOnly(forward),
             OptionId::ConfirmCloseEdits => OptionValue::ConfirmCloseEdits(forward),
+            OptionId::RestoreTabs => OptionValue::RestoreTabs(forward),
             // Never is the first: the shortest wait is one to the right.
             OptionId::IdleDisconnect => {
                 let at = Idle::ALL
@@ -1366,6 +1383,7 @@ impl Settings {
             OptionId::ConfirmCloseEdits => {
                 Some(OptionValue::ConfirmCloseEdits(!self.confirm_close_edits))
             }
+            OptionId::RestoreTabs => Some(OptionValue::RestoreTabs(!self.restore_tabs)),
             OptionId::AgentsOn => Some(OptionValue::AgentsOn(!self.mcp_enabled)),
             OptionId::LogDays => None,
             OptionId::AgentsNotify => Some(OptionValue::AgentsNotify(!self.mcp_notify)),
@@ -1393,6 +1411,7 @@ impl Settings {
     /// out: a theme that is not set, and stripes left to the look.
     fn written(&self, key: Key) -> Option<String> {
         Some(match key {
+            Key::RestoreTabs => self.restore_tabs.to_string(),
             Key::PageSize => self.page_size.to_string(),
             Key::Timestamps => toml_string(self.timestamps.name()),
             Key::Tz => toml_string(self.tz.name()),
@@ -1626,6 +1645,7 @@ impl Settings {
         let flag = value.as_bool();
         let text = value.as_str();
         match key {
+            Key::RestoreTabs => flag.map(|flag| self.restore_tabs = flag),
             Key::PageSize => number.map(|number| self.page_size = number),
             Key::Timestamps => text
                 .and_then(Timestamps::from_name)
@@ -1762,6 +1782,7 @@ impl Settings {
     /// Takes `key`'s value from `other`.
     fn keep(&mut self, key: Key, other: &Settings) {
         match key {
+            Key::RestoreTabs => self.restore_tabs = other.restore_tabs,
             Key::PageSize => self.page_size = other.page_size,
             Key::Timestamps => self.timestamps = other.timestamps,
             Key::Tz => self.tz = other.tz.clone(),
@@ -1827,6 +1848,7 @@ impl Settings {
 impl Default for Settings {
     fn default() -> Self {
         Self {
+            restore_tabs: true,
             page_size: Self::DEFAULT_PAGE_SIZE,
             timestamps: Timestamps::Second,
             tz: Zone::System,
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index e315cf7..c1ec074 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -25,6 +25,7 @@ use crate::settings::{
 /// terminal look lower-cases.
 pub(super) fn label(option: OptionId) -> &'static str {
     match option {
+        OptionId::RestoreTabs => "Restore tabs and filters",
         OptionId::PageSize => "Rows per page",
         OptionId::Timestamps => "Timestamps",
         OptionId::Tz => "Time zone for timestamptz",
@@ -58,7 +59,8 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
         OptionId::Zebra => "Alternate rows",
         OptionId::ColumnTypes => "Column types",
         OptionId::EnvBar => "Connection bar",
-        OptionId::PageSize
+        OptionId::RestoreTabs
+        | OptionId::PageSize
         | OptionId::Timestamps
         | OptionId::GroupDigits
         | OptionId::ValueTags
@@ -74,6 +76,9 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
 /// What a row says under its label, where it says something. English.
 pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
     match option {
+        // As the board writes it, though hiding columns is not built: what
+        // the option will cover once it is.
+        OptionId::RestoreTabs => Some("Per connection, including sort and hidden columns"),
         OptionId::PageSize => Some("Table view; the SQL editor has its own limit"),
         OptionId::ValueTags => Some("Colors for enum, CHECK (…IN…) and boolean columns"),
         OptionId::LogDays => Some("Every call and its SQL, never the rows. On this computer only"),
@@ -141,7 +146,8 @@ pub(super) fn days(locale: Locale, days: u32) -> String {
 pub(super) fn is_on(value: &OptionValue) -> bool {
     matches!(
         value,
-        OptionValue::ValueTags(true)
+        OptionValue::RestoreTabs(true)
+            | OptionValue::ValueTags(true)
             | OptionValue::ProductionReadOnly(true)
             | OptionValue::ConfirmCloseEdits(true)
             | OptionValue::AgentsOn(true)
@@ -261,6 +267,10 @@ pub(super) fn choices(
             (Word("off"), OptionValue::ConfirmCloseEdits(false)),
             (Word("on"), OptionValue::ConfirmCloseEdits(true)),
         ],
+        OptionId::RestoreTabs => vec![
+            (Word("off"), OptionValue::RestoreTabs(false)),
+            (Word("on"), OptionValue::RestoreTabs(true)),
+        ],
         OptionId::AgentsOn => vec![
             (Word("off"), OptionValue::AgentsOn(false)),
             (Word("on"), OptionValue::AgentsOn(true)),
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index a656628..8dcf6c1 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -158,7 +158,11 @@ pub(super) fn show(
                 SettingsPane::General => content(
                     &mut pane,
                     ("General", Some(GENERAL)),
-                    &[("Data", &OptionId::DATA), ("Safety", &OptionId::SAFETY)],
+                    &[
+                        ("Startup", &OptionId::STARTUP),
+                        ("Data", &OptionId::DATA),
+                        ("Safety", &OptionId::SAFETY),
+                    ],
                     &app.settings,
                     limit,
                     &skin,
@@ -290,7 +294,8 @@ fn nav(
 }
 
 /// What the General pane says under its title.
-const GENERAL: &str = "How Tabletist shows data and protects your work. Changes apply right away.";
+const GENERAL: &str =
+    "How Tabletist starts, shows data and protects your work. Changes apply right away.";
 
 /// The tab: its title, what it is for where it says, and its options under
 /// the heading of each group. The rows end at `limit` at the latest, and
@@ -625,7 +630,8 @@ fn control_height(option: OptionId, look: &Look) -> f32 {
         // The track has 2 over its segments and under them.
         OptionId::Timestamps | OptionId::GroupDigits => SEGMENT + 4.0,
         OptionId::RowHeight | OptionId::TableList | OptionId::EnvBar => SEGMENT + 4.0,
-        OptionId::ValueTags
+        OptionId::RestoreTabs
+        | OptionId::ValueTags
         | OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
@@ -743,7 +749,8 @@ fn control(
                 actions.push(Action::SetOption(OptionValue::EnvBar(bar)));
             }
         }
-        OptionId::ValueTags
+        OptionId::RestoreTabs
+        | OptionId::ValueTags
         | OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
@@ -938,6 +945,7 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
         // What the Appearance pane's rows say is their small print, and its
         // tiles have their own line.
         OptionId::Theme
+        | OptionId::RestoreTabs
         | OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
         | OptionId::IdleDisconnect
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index 3c098c5..bbf2b62 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -346,7 +346,8 @@ fn rows(ui: &mut Ui, settings: &Settings, cursor: usize, skin: &Skin, actions: &
     );
     let mut index = 0;
     for (heading_text, top, options) in [
-        ("Data", 8.0, &OptionId::DATA[..]),
+        ("Startup", 8.0, &OptionId::STARTUP[..]),
+        ("Data", 12.0, &OptionId::DATA[..]),
         ("Safety", 12.0, &OptionId::SAFETY[..]),
     ] {
         heading(ui, heading_text, top, skin);
@@ -867,7 +868,8 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
             let segments = words.into_iter().map(|(word, _)| segment(ui, word, skin));
             segments.sum::<f32>() + between
         }
-        OptionId::ValueTags
+        OptionId::RestoreTabs
+        | OptionId::ValueTags
         | OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
@@ -1041,7 +1043,8 @@ fn option_row(
         | OptionId::EnvBar => {
             segments(ui, cell, option, settings, skin, actions);
         }
-        OptionId::ValueTags
+        OptionId::RestoreTabs
+        | OptionId::ValueTags
         | OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
@@ -1190,6 +1193,7 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
                 .space(role, "  ")
                 .add(role, "ebook", second)
         }
+        OptionId::RestoreTabs => dim(&skin.say("per connection")),
         OptionId::ProductionReadOnly => dim(&skin.say("default for new connections")),
         // As drawn: nothing to add.
         OptionId::ConfirmCloseEdits => Text::new(look),
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_reference/settings-and-files.md src/app.rs src/app/startup_tests.rs src/model.rs src/session.rs src/settings.rs src/ui/mod.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs src/ui/zone_tests.rs
git commit -m "Add Restore tabs and filters to Settings › General"
```

---

### Task 3: On launch

The key `startup.on_launch` (`settings::Launch`), its row first in the Startup group (a menu on macOS and Windows, a stepper on Omarchy), and what a launch does with it (`App::launch`, from `App::new`, in place of the picker it always pushed). The note beside the key is the connection's name for a chosen one (`Settings::note`), written again each time the key is set, and read back. Renaming the chosen connection writes its new name there (`App::rename_launch`), and deleting it sets the option back to Show Connections.

**Files:** Modify `docs/_reference/settings-and-files.md`; Modify `src/app.rs`; Modify `src/app/startup_tests.rs`; Modify `src/model.rs`; Modify `src/settings.rs`; Modify `src/ui/mod.rs`; Modify `src/ui/picker.rs`; Modify `src/ui/settings/mod.rs`; Modify `src/ui/settings/sheet.rs`; Modify `src/ui/settings/terminal.rs`.

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 5fcc490..e7705d8 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -9510,6 +9510,7 @@ mod tests {
             settings.confirm_close_edits = false;
             settings.idle_disconnect = crate::settings::Idle::Never;
             settings.restore_tabs = false;
+            settings.on_launch = crate::settings::Launch::Connections;
             settings.show_system_schemas = true;
             settings.sql_limit = 50;
         });
@@ -9538,6 +9539,7 @@ mod tests {
         assert_eq!(app.settings.idle_disconnect, defaults.idle_disconnect);
         // And the Startup group's.
         assert!(app.settings.restore_tabs);
+        assert_eq!(app.settings.on_launch, crate::settings::Launch::Restore);
         assert!(app.settings.show_system_schemas);
         assert_eq!(app.settings.sql_limit, 50);
         assert_eq!(app.settings_file.text, app.settings.to_toml());
diff --git a/src/app/startup_tests.rs b/src/app/startup_tests.rs
index bf362e0..411d1a3 100644
--- a/src/app/startup_tests.rs
+++ b/src/app/startup_tests.rs
@@ -439,6 +439,225 @@ fn a_reopened_table_that_is_gone_says_why_as_an_open_one_does() {
     assert_eq!(front.query.sort[0].column, "title");
 }
 
+/// The Bookshop on a server, by its URL, as the screenshots have it.
+fn bookshop_at(app: &mut App, url: &str) -> crate::connections::ConnectionId {
+    let (spec, _) = tabletist_db::ConnectSpec::from_url(url).unwrap();
+    let mut saved = crate::testing::fixture_connection();
+    saved.name = "Bookshop".into();
+    saved.spec = spec;
+    let id = saved.id.clone();
+    app.connections.upsert(saved);
+    id
+}
+
+/// Writes what the app holds of its connections and settings, as the
+/// backend would, for the next start to read.
+fn write_files(app: &App) {
+    app.dirs.ensure().unwrap();
+    app.connections.save(&app.dirs.connections_file()).unwrap();
+    std::fs::write(app.dirs.settings_file(), &app.settings_file.text).unwrap();
+}
+
+/// A start over `dir`, with the settings its file has.
+fn start(dir: &std::path::Path) -> App {
+    let dirs = AppDirs::at(dir);
+    let settings = Settings::load(&dirs);
+    App::new(dirs, settings, Backend::recording())
+}
+
+/// The saved connection each window of the bar has, in order: `None` for
+/// the picker.
+fn windows(app: &App) -> Vec<Option<crate::connections::ConnectionId>> {
+    app.tabs
+        .iter()
+        .map(|tab| match &tab.content {
+            ConnTabContent::Workspace(workspace) => Some(workspace.conn_id.clone()),
+            ConnTabContent::Picker(_) => None,
+        })
+        .collect()
+}
+
+#[test]
+fn a_launch_restores_the_windows_open_at_the_last_quit() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let first = bookshop(&mut app, "Bookshop");
+    let second = bookshop(&mut app, "Bookshop");
+    open_window(&mut app, &first);
+    let tab = open_window(&mut app, &second);
+    books(&mut app, tab, "books");
+    app.save_session();
+    write_files(&app);
+    written(&app)
+        .unwrap()
+        .save(&app.dirs.session_file())
+        .unwrap();
+    let app = start(dir.path());
+    assert_eq!(windows(&app), vec![Some(first), Some(second.clone())]);
+    // The one in front is in front again, connecting, with its tables.
+    assert_eq!(app.active, 1);
+    let workspace = app.workspace(app.tabs[1].id).unwrap();
+    assert_eq!(workspace.conn_id, second);
+    assert!(matches!(workspace.status, SessionStatus::Connecting { .. }));
+    assert_eq!(workspace.object_tabs().count(), 1);
+    let connects = app
+        .backend
+        .sent
+        .iter()
+        .filter(|command| matches!(command, Command::Connect { .. }))
+        .count();
+    assert_eq!(connects, 2);
+}
+
+#[test]
+fn a_launch_with_no_session_and_a_deleted_window_leaves_them_out() {
+    let dir = tempfile::tempdir().unwrap();
+    // No file: the picker.
+    let app = start(dir.path());
+    assert_eq!(windows(&app), vec![None]);
+    // A window of a connection deleted since, and the picker.
+    let mut app = app_in(dir.path());
+    let kept = bookshop(&mut app, "Bookshop");
+    write_files(&app);
+    let mut session = Session::default();
+    session.open = vec![
+        Some(crate::connections::ConnectionId("deleted".into())),
+        None,
+        Some(kept.clone()),
+    ];
+    session.active = 2;
+    session.save(&app.dirs.session_file()).unwrap();
+    let app = start(dir.path());
+    assert_eq!(windows(&app), vec![None, Some(kept)]);
+    assert_eq!(app.active, 1);
+}
+
+#[test]
+fn a_launch_set_to_connections_shows_them_whatever_was_open() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    open_window(&mut app, &conn);
+    app.save_session();
+    app.apply(Action::SetOption(crate::settings::OptionValue::OnLaunch(
+        crate::settings::Launch::Connections,
+    )));
+    write_files(&app);
+    written(&app)
+        .unwrap()
+        .save(&app.dirs.session_file())
+        .unwrap();
+    let app = start(dir.path());
+    assert_eq!(windows(&app), vec![None]);
+}
+
+#[test]
+fn a_launch_opens_the_chosen_connection_by_its_id_of_two_of_one_name() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    bookshop_at(
+        &mut app,
+        "postgres://demo@localhost:5432/bookshop_development",
+    );
+    let chosen = bookshop_at(&mut app, "postgres://demo@localhost:5432/bookshop_preview");
+    app.apply(Action::SetOption(crate::settings::OptionValue::OnLaunch(
+        crate::settings::Launch::Connection {
+            id: chosen.clone(),
+            name: "Bookshop".into(),
+        },
+    )));
+    // The id is what the file holds, and the name its comment.
+    let line = format!("on_launch = \"conn:{}\"  # Bookshop\n", chosen.0);
+    assert!(
+        app.settings_file.text.contains(&line),
+        "{}",
+        app.settings_file.text
+    );
+    write_files(&app);
+    let app = start(dir.path());
+    assert_eq!(windows(&app), vec![Some(chosen.clone())]);
+    let workspace = app.workspace(app.tabs[0].id).unwrap();
+    assert_eq!(workspace.spec.database, "bookshop_preview");
+    assert!(app.notice.is_none());
+}
+
+#[test]
+fn a_deleted_chosen_connection_shows_connections_says_so_once_and_sets_the_option_back() {
+    let dir = tempfile::tempdir().unwrap();
+    let dirs = AppDirs::at(dir.path());
+    dirs.ensure().unwrap();
+    std::fs::write(
+        dirs.settings_file(),
+        "[startup]\non_launch = \"conn:6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11\"  # Bookshop\n",
+    )
+    .unwrap();
+    let app = start(dir.path());
+    assert_eq!(windows(&app), vec![None]);
+    assert_eq!(
+        app.notice.as_deref(),
+        Some(
+            "The connection chosen to open at launch was deleted. \
+             Tabletist showed Connections instead."
+        )
+    );
+    assert_eq!(app.settings.on_launch, crate::settings::Launch::Connections);
+    let saved = app.backend.sent.iter().find_map(|command| match command {
+        Command::Save {
+            file: StateFile::Settings(text),
+            ..
+        } => Some(text.clone()),
+        _ => None,
+    });
+    let saved = saved.expect("the settings were written");
+    assert!(saved.contains("on_launch = \"connections\""), "{saved}");
+    // The next start has nothing to say.
+    std::fs::write(dirs.settings_file(), saved).unwrap();
+    let again = start(dir.path());
+    assert!(again.notice.is_none());
+    assert_eq!(windows(&again), vec![None]);
+}
+
+#[test]
+fn with_connections_that_cannot_be_read_the_chosen_one_is_not_taken_for_deleted() {
+    let dir = tempfile::tempdir().unwrap();
+    let dirs = AppDirs::at(dir.path());
+    dirs.ensure().unwrap();
+    std::fs::write(
+        dirs.connections_file(),
+        "version = 3\n\n[[connections]]\nname = 1\n",
+    )
+    .unwrap();
+    std::fs::write(
+        dirs.settings_file(),
+        "[startup]\non_launch = \"conn:6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11\"\n",
+    )
+    .unwrap();
+    let app = start(dir.path());
+    assert_eq!(windows(&app), vec![None]);
+    assert!(matches!(
+        app.settings.on_launch,
+        crate::settings::Launch::Connection { .. }
+    ));
+    let notice = app.notice.clone().unwrap();
+    assert!(!notice.contains("chosen to open at launch"), "{notice}");
+}
+
+#[test]
+fn deleting_the_chosen_connection_sets_the_launch_to_connections() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    app.apply(Action::SetOption(crate::settings::OptionValue::OnLaunch(
+        crate::settings::Launch::Connection {
+            id: conn.clone(),
+            name: "Bookshop".into(),
+        },
+    )));
+    app.apply(Action::DeleteConnection(conn));
+    assert_eq!(app.settings.on_launch, crate::settings::Launch::Connections);
+    assert!(app.notice.is_none());
+}
+
 #[test]
 fn tables_of_another_sqlite_file_are_not_reopened() {
     let dir = tempfile::tempdir().unwrap();
@@ -450,3 +669,87 @@ fn tables_of_another_sqlite_file_are_not_reopened() {
     let tab = open_window(&mut app, &conn);
     assert_eq!(app.workspace(tab).unwrap().tabs.len(), 0);
 }
+
+#[test]
+fn a_restore_asks_first_for_the_window_in_front() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let first = bookshop_at(
+        &mut app,
+        "postgres://demo@localhost:5432/bookshop_development",
+    );
+    let second = bookshop_at(&mut app, "postgres://demo@localhost:5432/bookshop_preview");
+    for id in [&first, &second] {
+        let mut saved = app.connections.get(id).unwrap().clone();
+        saved.password = crate::connections::PasswordMode::Ask;
+        app.connections.upsert(saved);
+    }
+    write_files(&app);
+    let mut session = Session::default();
+    session.open = vec![Some(first.clone()), Some(second.clone())];
+    session.active = 1;
+    session.save(&app.dirs.session_file()).unwrap();
+    let app = start(dir.path());
+    // In their order, the second in front, and asking.
+    assert_eq!(windows(&app), vec![Some(first), Some(second)]);
+    assert_eq!(app.active, 1);
+    let front = app.tabs[1].id;
+    match &app.dialog {
+        Some(crate::model::Dialog::Password(prompt)) => assert_eq!(prompt.tab, front),
+        other => panic!("{other:?}"),
+    }
+}
+
+#[test]
+fn renaming_the_chosen_connection_renames_its_comment() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    app.apply(Action::SetOption(crate::settings::OptionValue::OnLaunch(
+        crate::settings::Launch::Connection {
+            id: conn.clone(),
+            name: "Bookshop".into(),
+        },
+    )));
+    app.apply(Action::EditConnection(conn.clone()));
+    match app.dialog.as_mut() {
+        Some(crate::model::Dialog::Connection(form)) => form.name = "Bookshop staging".into(),
+        other => panic!("{other:?}"),
+    }
+    app.apply(Action::SaveConnection { connect: false });
+    let line = format!("on_launch = \"conn:{}\"  # Bookshop staging\n", conn.0);
+    assert!(
+        app.settings_file.text.contains(&line),
+        "{}",
+        app.settings_file.text
+    );
+    // Written, as any change of the settings is.
+    let saved = app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|command| match command {
+            Command::Save {
+                file: StateFile::Settings(text),
+                ..
+            } => Some(text.clone()),
+            _ => None,
+        });
+    assert_eq!(saved.as_deref(), Some(app.settings_file.text.as_str()));
+}
+
+#[test]
+fn a_launch_restores_the_picker_after_a_window_and_in_front() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let kept = bookshop(&mut app, "Bookshop");
+    write_files(&app);
+    let mut session = Session::default();
+    session.open = vec![Some(kept.clone()), None, None];
+    session.active = 1;
+    session.save(&app.dirs.session_file()).unwrap();
+    let app = start(dir.path());
+    assert_eq!(windows(&app), vec![Some(kept), None]);
+    assert_eq!(app.active, 1);
+}
diff --git a/src/settings.rs b/src/settings.rs
index 73f6777..06510e8 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1934,6 +1934,169 @@ mod tests {
         assert!(!loaded.settings.confirm_close_edits);
     }
 
+    fn chosen(id: &str, name: &str) -> Launch {
+        Launch::Connection {
+            id: crate::connections::ConnectionId(id.into()),
+            name: name.into(),
+        }
+    }
+
+    #[test]
+    fn a_launch_is_restore_connections_or_a_connection_by_its_id() {
+        for launch in [Launch::Restore, Launch::Connections, chosen("6a0c", "")] {
+            assert_eq!(Launch::from_name(&launch.name()), Some(launch));
+        }
+        assert_eq!(chosen("6a0c", "").name(), "conn:6a0c");
+        for wrong in ["conn:", "conn:  ", "Bookshop", "Restore"] {
+            assert_eq!(Launch::from_name(wrong), None, "{wrong}");
+        }
+        // The name is the comment beside the id, no part of the value.
+        assert_eq!(chosen("6a0c", "Bookshop"), chosen("6a0c", "Bookshop copy"));
+        assert_ne!(chosen("6a0c", "Bookshop"), chosen("9f1e", "Bookshop"));
+    }
+
+    #[test]
+    fn a_launch_out_of_the_set_is_ignored_and_keeps_the_last_good_one() {
+        let last = Settings {
+            on_launch: Launch::Connections,
+            ..Settings::default()
+        };
+        let text = "[startup]\non_launch = \"conn:\"\nrestore_tabs = false\n";
+        let loaded = Settings::from_toml_over(text, &last);
+        assert_eq!(loaded.invalid, [2]);
+        assert_eq!(loaded.settings.on_launch, Launch::Connections);
+        assert!(!loaded.settings.restore_tabs);
+    }
+
+    #[test]
+    fn a_chosen_connection_is_written_with_its_name_beside_it_and_read_back_so() {
+        let text = written("", |settings| {
+            settings.on_launch = chosen("6a0c", "Bookshop");
+        });
+        assert_eq!(
+            text,
+            format!("{HEADER}[startup]\non_launch = \"conn:6a0c\"  # Bookshop\n")
+        );
+        match Settings::from_toml(&text).settings.on_launch {
+            Launch::Connection { name, .. } => assert_eq!(name, "Bookshop"),
+            other => panic!("{other:?}"),
+        }
+        // Another one: the comment names it.
+        let text = written(&text, |settings| {
+            settings.on_launch = chosen("9f1e", "Bookshop preview");
+        });
+        assert!(
+            text.ends_with("on_launch = \"conn:9f1e\"  # Bookshop preview\n"),
+            "{text}"
+        );
+        // Connections: the comment says the values there are.
+        let text = written(&text, |settings| {
+            settings.on_launch = Launch::Connections;
+        });
+        assert!(
+            text.ends_with(
+                "on_launch = \"connections\"  # restore | connections | \"conn:<id>\"\n"
+            ),
+            "{text}"
+        );
+        // And Restore, the default, takes the line out.
+        let text = written(&text, |settings| settings.on_launch = Launch::Restore);
+        assert_eq!(text, HEADER);
+    }
+
+    #[test]
+    fn a_chosen_connection_in_an_inline_table_is_set_in_place_and_the_table_kept() {
+        let text = "startup = { on_launch = \"connections\", restore_tabs = false }\n";
+        let text = written(text, |settings| {
+            settings.on_launch = chosen("6a0c", "Bookshop");
+        });
+        let loaded = Settings::from_toml(&text);
+        assert!(loaded.invalid.is_empty(), "{text}");
+        assert_eq!(loaded.settings.on_launch, chosen("6a0c", ""));
+        assert!(!loaded.settings.restore_tabs, "{text}");
+    }
+
+    #[test]
+    fn a_name_with_a_line_break_is_written_on_one_line() {
+        let settings = Settings {
+            on_launch: chosen("6a0c", "Bookshop\n[data]\npage_size = 5"),
+            ..Settings::default()
+        };
+        let text = settings.to_toml();
+        let loaded = Settings::from_toml(&text);
+        assert_eq!(
+            loaded.settings.page_size,
+            Settings::DEFAULT_PAGE_SIZE,
+            "{text}"
+        );
+        assert!(
+            text.contains("on_launch = \"conn:6a0c\"  # Bookshop [data] page_size = 5\n"),
+            "{text}"
+        );
+    }
+
+    #[test]
+    fn a_launch_set_back_to_restore_over_a_rejected_twin_gets_the_note_of_the_values() {
+        let text = "[startup]\non_launch = \"conn:6a0c\"  # Bookshop\non_launch = 5\n";
+        let text = written(text, |settings| settings.on_launch = Launch::Restore);
+        assert!(
+            text.contains("on_launch = \"restore\"  # restore | connections | \"conn:<id>\"\n"),
+            "{text}"
+        );
+        // That note names no connection when a value is written by hand
+        // in front of it.
+        let text =
+            "[startup]\non_launch = \"conn:6a0c\"  # restore | connections | \"conn:<id>\"\n";
+        match Settings::from_toml(text).settings.on_launch {
+            Launch::Connection { name, .. } => assert_eq!(name, ""),
+            other => panic!("{other:?}"),
+        }
+    }
+
+    #[test]
+    fn the_launch_steps_through_restore_connections_and_each_connection() {
+        let choices = [
+            Launch::Restore,
+            Launch::Connections,
+            chosen("6a0c", "Bookshop"),
+            chosen("9f1e", "Bookshop"),
+        ];
+        let step = |launch: Launch, forward: bool| {
+            let settings = Settings {
+                on_launch: launch,
+                ..Settings::default()
+            };
+            settings.stepped_launch(&choices, forward)
+        };
+        assert_eq!(
+            step(Launch::Restore, true),
+            OptionValue::OnLaunch(Launch::Connections)
+        );
+        assert_eq!(
+            step(Launch::Connections, true),
+            OptionValue::OnLaunch(chosen("6a0c", ""))
+        );
+        assert_eq!(
+            step(chosen("9f1e", ""), true),
+            OptionValue::OnLaunch(chosen("9f1e", ""))
+        );
+        assert_eq!(
+            step(Launch::Restore, false),
+            OptionValue::OnLaunch(Launch::Restore)
+        );
+        // A connection that is not saved steps as Connections does.
+        assert_eq!(
+            step(chosen("gone", ""), false),
+            OptionValue::OnLaunch(Launch::Restore)
+        );
+        // Without the connections, the two others.
+        assert_eq!(
+            Settings::default().stepped(OptionId::OnLaunch, true),
+            OptionValue::OnLaunch(Launch::Connections)
+        );
+        assert_eq!(Settings::default().flipped(OptionId::OnLaunch), None);
+    }
+
     #[test]
     fn the_defaults_are_written_as_the_header_alone() {
         assert_eq!(Settings::default().to_toml(), HEADER);
@@ -1942,6 +2105,10 @@ mod tests {
     /// Every key away from its default.
     fn all_set() -> Settings {
         Settings {
+            on_launch: Launch::Connection {
+                id: crate::connections::ConnectionId("6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11".into()),
+                name: "Bookshop".into(),
+            },
             restore_tabs: false,
             page_size: 500,
             timestamps: Timestamps::Full,
@@ -1976,6 +2143,7 @@ mod tests {
                 "\
 # written by tabletist, safe to edit by hand
 [startup]
+on_launch = \"conn:6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11\"  # Bookshop
 restore_tabs = false
 
 [data]
@@ -2074,27 +2242,28 @@ bar = \"label\"  # tint | label
         assert_eq!(
             loaded.lines,
             vec![
-                (Key::RestoreTabs, 3),
-                (Key::PageSize, 6),
-                (Key::Timestamps, 7),
-                (Key::Tz, 8),
-                (Key::GroupDigits, 9),
-                (Key::ValueTags, 10),
-                (Key::ProductionReadOnly, 13),
-                (Key::ConfirmCloseEdits, 14),
-                (Key::IdleDisconnect, 15),
-                (Key::ShowSystemSchemas, 18),
-                (Key::SqlLimit, 21),
-                (Key::SqlTimeoutSecs, 22),
-                (Key::McpEnabled, 25),
-                (Key::McpLogDays, 26),
-                (Key::McpNotify, 27),
-                (Key::Theme, 30),
-                (Key::RowHeight, 33),
-                (Key::Zebra, 34),
-                (Key::ColumnTypes, 35),
-                (Key::TableList, 36),
-                (Key::EnvBar, 39),
+                (Key::OnLaunch, 3),
+                (Key::RestoreTabs, 4),
+                (Key::PageSize, 7),
+                (Key::Timestamps, 8),
+                (Key::Tz, 9),
+                (Key::GroupDigits, 10),
+                (Key::ValueTags, 11),
+                (Key::ProductionReadOnly, 14),
+                (Key::ConfirmCloseEdits, 15),
+                (Key::IdleDisconnect, 16),
+                (Key::ShowSystemSchemas, 19),
+                (Key::SqlLimit, 22),
+                (Key::SqlTimeoutSecs, 23),
+                (Key::McpEnabled, 26),
+                (Key::McpLogDays, 27),
+                (Key::McpNotify, 28),
+                (Key::Theme, 31),
+                (Key::RowHeight, 34),
+                (Key::Zebra, 35),
+                (Key::ColumnTypes, 36),
+                (Key::TableList, 37),
+                (Key::EnvBar, 40),
             ]
         );
     }
@@ -3105,6 +3274,7 @@ bar = \"label\"  # tint | label
         assert_eq!(
             keys,
             vec![
+                Key::OnLaunch,
                 Key::RestoreTabs,
                 Key::PageSize,
                 Key::Timestamps,
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 7108c82..7cfa27b 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -1394,21 +1394,99 @@ value_tags   = true
         }
     }
 
+    /// Two saved Bookshops of one name, on two databases of a server.
+    fn two_bookshops(harness: &mut Harness) -> [crate::connections::ConnectionId; 2] {
+        ["bookshop_development", "bookshop_preview"].map(|database| {
+            let url = format!("postgres://demo@localhost:5432/{database}");
+            let (spec, _) = tabletist_db::ConnectSpec::from_url(&url).unwrap();
+            let mut saved = crate::testing::fixture_connection();
+            saved.name = "Bookshop".into();
+            saved.spec = spec;
+            let id = saved.id.clone();
+            harness.app.connections.upsert(saved);
+            id
+        })
+    }
+
     #[test]
-    fn the_startup_group_of_the_screen_has_its_row_first_and_space_flips_it() {
+    fn on_launch_in_the_sheet_lists_each_connection_with_where_it_points() {
+        use crate::settings::Launch;
+        for look in SHEET_LOOKS {
+            let mut harness = settings_sheet(look());
+            let [_, preview] = two_bookshops(&mut harness);
+            // The board's hint, under the label: beside the menu it has
+            // no room.
+            let hint = "Or: show Connections · open a chosen connection";
+            let hint = harness.painted_rect(hint).expect("the hint");
+            let label = harness.painted_rect("On launch").expect("the label");
+            assert!(hint.top() > label.bottom(), "{hint:?} {label:?}");
+            harness.click("On launch");
+            for entry in [
+                "On launch Restore last session",
+                "On launch Show Connections",
+                "On launch Bookshop localhost:5432/bookshop_development",
+                "On launch Bookshop localhost:5432/bookshop_preview",
+            ] {
+                assert!(harness.has(entry), "{entry}");
+            }
+            harness.click("On launch Bookshop localhost:5432/bookshop_preview");
+            assert_eq!(
+                harness.app.settings.on_launch,
+                Launch::Connection {
+                    id: preview.clone(),
+                    name: String::new(),
+                }
+            );
+            let text = &harness.app.settings_file.text;
+            let line = format!("on_launch = \"conn:{}\"  # Bookshop\n", preview.0);
+            assert!(text.contains(&line), "{text}");
+            harness.click("On launch");
+            harness.click("On launch Show Connections");
+            assert_eq!(harness.app.settings.on_launch, Launch::Connections);
+        }
+    }
+
+    #[test]
+    fn the_startup_group_of_the_screen_steps_on_launch_and_flips_restore() {
+        use crate::settings::Launch;
         let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
         harness.set_look(crate::theme::Look::omarchy());
+        let [development, _] = two_bookshops(&mut harness);
         harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
-        for text in ["startup", "per connection"] {
+        for text in [
+            "startup",
+            "or connections · a chosen connection",
+            "per connection",
+        ] {
             assert!(harness.painted_color(text).is_some(), "{text}");
         }
-        // The cursor opens on it.
+        // The cursor opens on the first.
+        assert!(harness.painted_color("▌on launch").is_some());
+        assert!(harness.painted_color("‹ restore last session ›").is_some());
+        harness.press(egui::Key::L, egui::Modifiers::NONE);
+        assert_eq!(harness.app.settings.on_launch, Launch::Connections);
+        assert!(harness.painted_color("‹ show connections ›").is_some());
+        harness.press(egui::Key::L, egui::Modifiers::NONE);
+        assert_eq!(
+            harness.app.settings.on_launch,
+            Launch::Connection {
+                id: development.clone(),
+                name: String::new(),
+            }
+        );
+        let shown = "‹ Bookshop (localhost:5432/bookshop_development) ›";
+        assert!(harness.painted_color(shown).is_some());
+        let text = &harness.app.settings_file.text;
+        let line = format!("on_launch = \"conn:{}\"  # Bookshop\n", development.0);
+        assert!(text.contains(&line), "{text}");
+        harness.press(egui::Key::R, egui::Modifiers::SHIFT);
+        assert_eq!(harness.app.settings.on_launch, Launch::Restore);
+        // The row under it.
+        harness.press(egui::Key::J, egui::Modifiers::NONE);
         assert!(harness.painted_color("▌restore tabs and filters").is_some());
         harness.press(egui::Key::Space, egui::Modifiers::NONE);
         assert!(!harness.app.settings.restore_tabs);
         assert!(harness.painted_color("restore_tabs = ").is_some());
-        harness.press(egui::Key::R, egui::Modifiers::SHIFT);
-        assert!(harness.app.settings.restore_tabs);
     }
 
     #[test]
@@ -1731,9 +1809,7 @@ value_tags   = true
             // the first of them.
             let stored = harness.painted_rect("Stored in").expect("the footer");
             assert!(stored.bottom() < size.y, "{stored:?}");
-            let row = harness
-                .painted_rect("Restore tabs and filters")
-                .expect("a row");
+            let row = harness.painted_rect("On launch").expect("a row");
             assert!(row.bottom() < stored.top(), "{row:?} {stored:?}");
             harness.click("Export…");
             assert_eq!(harness.app.backend.saves.len(), 1);
@@ -2356,7 +2432,7 @@ value_tags   = true
         let mut harness = Harness::with_size(egui::vec2(1000.0, 700.0));
         harness.set_look(crate::theme::Look::omarchy());
         harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
-        assert!(harness.painted_color("▌restore tabs and filters").is_some());
+        assert!(harness.painted_color("▌on launch").is_some());
         assert!(harness.painted_color("[data]").is_none());
     }
 
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `` cannot find type `Launch` in this scope `` (22 times); `` no field `on_launch` on type `settings::Settings` `` (13 times); `` no variant, associated function, or constant named `OnLaunch` found for enum `settings::OptionValue` in the current scope `` (10 times); `` cannot find `Launch` in `settings` `` (9 times), and 7 more.

- [ ] **Step 3: Write the code**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 306ba6e..ee46261 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -12,6 +12,7 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 
 | Option | Choices | Default |
 | --- | --- | --- |
+| **On launch** | Restore last session, Show Connections, or a saved connection, each listed with its server and database. Restore opens again the connection windows open when Tabletist last quit, in their order, each connecting and asking for what it needs; with nothing kept, it shows Connections. If the chosen connection was deleted outside Tabletist, Tabletist shows Connections, says so once, and sets this back to Show Connections; deleting it in Tabletist sets it back at once. `--demo` starts as it always does | Restore last session |
 | **Restore tabs and filters** | On or off. On, a connection that opens, at launch or from Connections, opens again the table and view tabs its window had when it last closed on that database (or SQLite file), each with its sort and filters; the one in front loads, and each other when you first show it. SQL editors are not opened again. Off, a connection opens with its tables in the sidebar and no tab | On |
 | **Rows per page** | 100, 500, 1,000, 5,000 or 10,000. A table takes a new size when it next loads, not while you page through it | 1,000 |
 | **Timestamps** | To the second, or full precision | To the second |
@@ -63,6 +64,7 @@ follows the look):
 
 ```toml
 [startup]
+on_launch = "restore"
 restore_tabs = true
 
 [data]
@@ -99,6 +101,7 @@ bar = "tint"
 
 | Key | Values | Default |
 | --- | --- | --- |
+| `startup.on_launch` | `"restore"`, `"connections"` or `"conn:<id>"`, a saved connection's `id` from `connections.toml`. Tabletist writes the connection's name in a comment beside it, `on_launch = "conn:6a0c…"  # Bookshop`, and puts the comment back each time it sets the key | `"restore"` |
 | `startup.restore_tabs` | `true` or `false`. Whether a connection opens again the table tabs, sorts and filters its window last had, kept in `session.json` | `true` |
 | `data.page_size` | 10 to 10000. A table takes a new size when it next loads (opened, refreshed, sorted, filtered or reconnected), not while you page through it | `1000` |
 | `data.timestamps` | `"second"` or `"full"` | `"second"` |
diff --git a/src/app.rs b/src/app.rs
index e7705d8..21b7700 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -315,8 +315,7 @@ impl App {
             agents_held: Vec::new(),
             next_id: 1,
         };
-        let tab = app.picker_tab();
-        app.tabs.push(tab);
+        app.launch();
         // What an older version kept in connections.json is written as
         // this one keeps it, once, off the UI thread. So is a file in
         // which a connection had no id yet.
@@ -2456,6 +2455,16 @@ impl App {
                     if self.session.forget(&id) {
                         self.save_session();
                     }
+                    // A launch that was to open it shows the connections.
+                    let chosen = crate::settings::Launch::Connection {
+                        id: id.clone(),
+                        name: String::new(),
+                    };
+                    if self.settings.on_launch == chosen {
+                        self.change_settings(|settings| {
+                            settings.on_launch = crate::settings::Launch::Connections;
+                        });
+                    }
                     if removed.password == PasswordMode::Keyring {
                         self.store_secret(&id, false, None, false);
                     }
@@ -2672,6 +2681,129 @@ impl App {
         }
     }
 
+    /// Opens what the settings say a launch opens: the windows open at the
+    /// last quit, in their order and the one in front in front again, the
+    /// picker, or the chosen connection. A chosen connection that is
+    /// deleted opens the picker, says so, and sets the option to the
+    /// picker, so that the next launch does what the menu says; with
+    /// connections that could not be read, nothing is known to be
+    /// deleted, and the option stays. Whatever opens nothing opens the
+    /// picker.
+    fn launch(&mut self) {
+        match self.settings.on_launch.clone() {
+            crate::settings::Launch::Restore => self.open_last_session(),
+            crate::settings::Launch::Connections => {}
+            crate::settings::Launch::Connection { id, .. } => {
+                match self.connections.get(&id).cloned() {
+                    Some(saved) => {
+                        let tab = self.picker_tab();
+                        let id = tab.id;
+                        self.tabs.push(tab);
+                        self.connect_tab(id, saved, Secrets::default());
+                    }
+                    None if self.writes_session => {
+                        self.say(
+                            "The connection chosen to open at launch was deleted. \
+                             Tabletist showed Connections instead.",
+                        );
+                        self.change_settings(|settings| {
+                            settings.on_launch = crate::settings::Launch::Connections;
+                        });
+                    }
+                    None => {}
+                }
+            }
+        }
+        if self.tabs.is_empty() {
+            let tab = self.picker_tab();
+            self.tabs.push(tab);
+        }
+    }
+
+    /// The windows open at the last quit, each connecting as a connection
+    /// does, asking for what it needs: the one in front first, so that a
+    /// password it asks for is asked over it, and the others in their
+    /// order after it. A connection deleted since is left out, and the
+    /// picker is one window at most.
+    fn open_last_session(&mut self) {
+        let open = self.session.open.clone();
+        let mut windows = Vec::new();
+        let mut active = None;
+        // Every window opens as a picker until it connects, below: the
+        // picker kept is counted here.
+        let mut picker = false;
+        for (index, entry) in open.into_iter().enumerate() {
+            let saved = match entry {
+                Some(id) => match self.connections.get(&id).cloned() {
+                    Some(saved) => Some(saved),
+                    None => continue,
+                },
+                None if picker => continue,
+                None => {
+                    picker = true;
+                    None
+                }
+            };
+            let tab = self.picker_tab();
+            windows.push((tab.id, saved));
+            self.tabs.push(tab);
+            if index <= self.session.active {
+                active = Some(self.tabs.len() - 1);
+            }
+        }
+        let active = active.unwrap_or(0);
+        self.active = active;
+        if active < windows.len() {
+            let front = windows.remove(active);
+            windows.insert(0, front);
+        }
+        for (tab, saved) in windows {
+            if let Some(saved) = saved {
+                self.connect_tab(tab, saved, Secrets::default());
+            }
+        }
+    }
+
+    /// Writes the name of `saved` beside it in the settings file, where it
+    /// is the connection a launch opens and its name there is another:
+    /// the comment is the only thing that changes.
+    fn rename_launch(&mut self, saved: &SavedConnection) {
+        let crate::settings::Launch::Connection { id, name } = &self.settings.on_launch else {
+            return;
+        };
+        if *id != saved.id || *name == saved.name {
+            return;
+        }
+        let mut renamed = self.settings.clone();
+        renamed.on_launch = crate::settings::Launch::Connection {
+            id: id.clone(),
+            name: saved.name.clone(),
+        };
+        let text = renamed.write_into(&self.settings_file.text, &[crate::settings::Key::OnLaunch]);
+        self.settings.on_launch = renamed.on_launch;
+        if text != self.settings_file.text {
+            let loaded = Settings::from_toml_over(&text, &self.settings);
+            let live = self.settings_file.live;
+            let saved = self.settings_file.saved.take();
+            let (_, file) = loaded.into_parts();
+            self.settings_file = SettingsFile {
+                live,
+                saved,
+                ..file
+            };
+            self.save_settings();
+        }
+    }
+
+    /// Shows `text` in the notice, after what it says already: a start can
+    /// have more than one thing to say.
+    fn say(&mut self, text: &str) {
+        self.notice = Some(match self.notice.take() {
+            Some(said) => format!("{said} {text}"),
+            None => text.to_owned(),
+        });
+    }
+
     /// Keeps the table tabs of `tab`'s window in the session, for the
     /// next time its connection opens: before the window closes, or has
     /// another connection put in its place. Whether there was a window of
@@ -4468,6 +4600,7 @@ impl App {
         }
         self.connections.upsert(saved.clone());
         self.save_connections();
+        self.rename_launch(&saved);
         if saved.password == PasswordMode::Keyring {
             if typed.is_some() || stale {
                 self.store_secret(&saved.id, false, typed.clone().map(SecretString), false);
diff --git a/src/model.rs b/src/model.rs
index b46ca91..635a603 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2555,6 +2555,7 @@ impl SettingsPane {
         use crate::settings::OptionId;
         match self {
             Self::General => &[
+                SettingsRow::Option(OptionId::OnLaunch),
                 SettingsRow::Option(OptionId::RestoreTabs),
                 SettingsRow::Option(OptionId::PageSize),
                 SettingsRow::Option(OptionId::Timestamps),
diff --git a/src/settings.rs b/src/settings.rs
index 06510e8..43e6e50 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -370,6 +370,65 @@ impl Idle {
     }
 }
 
+/// What a launch opens.
+#[derive(Clone, Debug, Default)]
+pub enum Launch {
+    /// The connection windows open at the last quit (`session.json`).
+    #[default]
+    Restore,
+    /// The saved connections, to choose from.
+    Connections,
+    /// A saved connection, by its id: `conn:<id>`. `name` is what the file
+    /// writes beside it, so a person reading it knows which: it is no part
+    /// of the value, and two of one name are told apart by their ids.
+    Connection {
+        id: crate::connections::ConnectionId,
+        name: String,
+    },
+}
+
+/// By the value alone: a connection's name is the comment beside it.
+impl PartialEq for Launch {
+    fn eq(&self, other: &Self) -> bool {
+        match (self, other) {
+            (Self::Restore, Self::Restore) | (Self::Connections, Self::Connections) => true,
+            (Self::Connection { id, .. }, Self::Connection { id: other, .. }) => id == other,
+            (Self::Restore | Self::Connections | Self::Connection { .. }, _) => false,
+        }
+    }
+}
+
+impl Eq for Launch {}
+
+impl Launch {
+    /// What a chosen connection's id follows in the file.
+    const CONNECTION: &str = "conn:";
+
+    /// As the file says it.
+    pub fn name(&self) -> String {
+        match self {
+            Self::Restore => "restore".into(),
+            Self::Connections => "connections".into(),
+            Self::Connection { id, .. } => format!("{}{}", Self::CONNECTION, id.0),
+        }
+    }
+
+    /// The value the file's text names: `conn:` names no connection.
+    pub fn from_name(name: &str) -> Option<Self> {
+        match name {
+            "restore" => Some(Self::Restore),
+            "connections" => Some(Self::Connections),
+            _ => name
+                .strip_prefix(Self::CONNECTION)
+                .filter(|id| !id.trim().is_empty())
+                .map(|id| Self::Connection {
+                    id: crate::connections::ConnectionId(id.to_owned()),
+                    name: String::new(),
+                }),
+        }
+    }
+}
+
 /// Everything the user can set. A key the file lacks takes its default and
 /// one this version does not know is ignored, so older and newer files both
 /// load. The serde derive reads the `settings.json` of versions before the
@@ -377,6 +436,11 @@ impl Idle {
 #[derive(Clone, Debug, PartialEq, Deserialize)]
 #[serde(default)]
 pub struct Settings {
+    /// What a launch opens: the windows open at the last quit, the
+    /// saved connections, or one of them. Not in an older version's
+    /// `settings.json`, which had none.
+    #[serde(skip)]
+    pub on_launch: Launch,
     /// Whether a connection that opens reopens the table tabs its window
     /// last had, with their sort and filters (`session.json`).
     pub restore_tabs: bool,
@@ -442,6 +506,7 @@ pub struct Settings {
 /// A key of the settings file.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum Key {
+    OnLaunch,
     RestoreTabs,
     PageSize,
     Timestamps,
@@ -467,7 +532,8 @@ pub enum Key {
 
 impl Key {
     /// In the order the file has them: a table's keys together.
-    pub const ALL: [Key; 21] = [
+    pub const ALL: [Key; 22] = [
+        Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
         Self::Timestamps,
@@ -494,6 +560,7 @@ impl Key {
     /// The key's table and its name there.
     pub fn path(self) -> (&'static str, &'static str) {
         match self {
+            Self::OnLaunch => ("startup", "on_launch"),
             Self::RestoreTabs => ("startup", "restore_tabs"),
             Self::PageSize => ("data", "page_size"),
             Self::Timestamps => ("data", "timestamps"),
@@ -519,9 +586,11 @@ impl Key {
     }
 
     /// What the file says beside the key: the values of a closed set, or
-    /// what a value that reads oddly means.
+    /// what a value that reads oddly means. On launch's depends on its
+    /// value: see [`Settings::note`].
     fn note(self) -> Option<&'static str> {
         match self {
+            Self::OnLaunch => Some("restore | connections | \"conn:<id>\""),
             Self::Timestamps => Some("second | full"),
             Self::Tz => Some("system | utc | \"Area/City\""),
             Self::IdleDisconnect => Some("never | 5m | 15m | 30m | 1h | 4h"),
@@ -544,6 +613,8 @@ impl Key {
 /// An option the Settings window shows: one row of it.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum OptionId {
+    /// What a launch opens: `startup.on_launch`.
+    OnLaunch,
     /// Whether a connection reopens its tables: `startup.restore_tabs`.
     RestoreTabs,
     PageSize,
@@ -577,7 +648,8 @@ pub enum OptionId {
 impl OptionId {
     /// Every option, the General pane's, the Appearance pane's and then
     /// the Agents pane's.
-    pub const ALL: [OptionId; 18] = [
+    pub const ALL: [OptionId; 19] = [
+        Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
         Self::Timestamps,
@@ -598,7 +670,8 @@ impl OptionId {
         Self::AgentsNotify,
     ];
     /// The General pane's, in the order it has them.
-    pub const GENERAL: [OptionId; 9] = [
+    pub const GENERAL: [OptionId; 10] = [
+        Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
         Self::Timestamps,
@@ -610,7 +683,7 @@ impl OptionId {
         Self::IdleDisconnect,
     ];
     /// The General pane's sections: Startup, Data, then Safety.
-    pub const STARTUP: [OptionId; 1] = [Self::RestoreTabs];
+    pub const STARTUP: [OptionId; 2] = [Self::OnLaunch, Self::RestoreTabs];
     pub const DATA: [OptionId; 5] = [
         Self::PageSize,
         Self::Timestamps,
@@ -647,6 +720,7 @@ impl OptionId {
     /// The key of the file the option is stored under.
     pub fn key(self) -> Key {
         match self {
+            Self::OnLaunch => Key::OnLaunch,
             Self::RestoreTabs => Key::RestoreTabs,
             Self::PageSize => Key::PageSize,
             Self::Timestamps => Key::Timestamps,
@@ -677,6 +751,7 @@ impl OptionId {
 /// An option with a value: what a key, a click or a reset sets.
 #[derive(Clone, Debug, PartialEq, Eq)]
 pub enum OptionValue {
+    OnLaunch(Launch),
     RestoreTabs(bool),
     PageSize(u32),
     Timestamps(Timestamps),
@@ -702,6 +777,7 @@ pub enum OptionValue {
 impl OptionValue {
     pub fn option(&self) -> OptionId {
         match self {
+            Self::OnLaunch(_) => OptionId::OnLaunch,
             Self::RestoreTabs(_) => OptionId::RestoreTabs,
             Self::PageSize(_) => OptionId::PageSize,
             Self::Timestamps(_) => OptionId::Timestamps,
@@ -726,6 +802,7 @@ impl OptionValue {
     /// Puts the value in `settings`.
     pub fn set(&self, settings: &mut Settings) {
         match self.clone() {
+            Self::OnLaunch(launch) => settings.on_launch = launch,
             Self::RestoreTabs(on) => settings.restore_tabs = on,
             Self::PageSize(size) => settings.page_size = size,
             Self::Timestamps(choice) => settings.timestamps = choice,
@@ -1142,6 +1219,15 @@ fn line_start(text: &str, offset: usize) -> usize {
     text[..offset].rfind('\n').map_or(0, |at| at + 1)
 }
 
+/// The comment after `offset` on its line, without its `#`: empty where
+/// there is none.
+fn comment_after(text: &str, offset: usize) -> String {
+    let rest = &text[offset..line_end(text, offset)];
+    rest.trim_start_matches([' ', '\t'])
+        .strip_prefix('#')
+        .map_or_else(String::new, |comment| comment.trim().to_owned())
+}
+
 /// Where the line after the one `offset` is on begins: past its break, or
 /// the end of the text.
 fn line_end(text: &str, offset: usize) -> usize {
@@ -1230,6 +1316,7 @@ impl Settings {
     /// What `option` is set to.
     pub fn value(&self, option: OptionId) -> OptionValue {
         match option {
+            OptionId::OnLaunch => OptionValue::OnLaunch(self.on_launch.clone()),
             OptionId::RestoreTabs => OptionValue::RestoreTabs(self.restore_tabs),
             OptionId::PageSize => OptionValue::PageSize(self.page_size),
             OptionId::Timestamps => OptionValue::Timestamps(self.timestamps),
@@ -1291,6 +1378,11 @@ impl Settings {
             OptionId::ProductionReadOnly => OptionValue::ProductionReadOnly(forward),
             OptionId::ConfirmCloseEdits => OptionValue::ConfirmCloseEdits(forward),
             OptionId::RestoreTabs => OptionValue::RestoreTabs(forward),
+            // Without the connections to step through: the two that are
+            // not one. The screen steps with them (`stepped_launch`).
+            OptionId::OnLaunch => {
+                self.stepped_launch(&[Launch::Restore, Launch::Connections], forward)
+            }
             // Never is the first: the shortest wait is one to the right.
             OptionId::IdleDisconnect => {
                 let at = Idle::ALL
@@ -1365,10 +1457,74 @@ impl Settings {
         OptionValue::Tz(zones.swap_remove(to))
     }
 
+    /// What a launch opens one step from this, among `choices`: Restore,
+    /// Show Connections, then each saved connection. A connection that is
+    /// not among them steps as Show Connections does.
+    pub fn stepped_launch(&self, choices: &[Launch], forward: bool) -> OptionValue {
+        let at = choices
+            .iter()
+            .position(|choice| *choice == self.on_launch)
+            .unwrap_or(1)
+            .min(choices.len().saturating_sub(1));
+        let to = if forward {
+            (at + 1).min(choices.len().saturating_sub(1))
+        } else {
+            at.saturating_sub(1)
+        };
+        choices.get(to).cloned().map_or(
+            OptionValue::OnLaunch(self.on_launch.clone()),
+            OptionValue::OnLaunch,
+        )
+    }
+
+    /// What the file writes beside `key`: for a chosen connection to open
+    /// at launch, its name on one line, and nothing where none is known.
+    fn note(&self, key: Key) -> Option<std::borrow::Cow<'static, str>> {
+        match (key, &self.on_launch) {
+            (Key::OnLaunch, Launch::Connection { name, .. }) => {
+                // A name is the person's text: a break in it would start
+                // a line of the file.
+                let name: String = name
+                    .chars()
+                    .map(|c| if c.is_control() { ' ' } else { c })
+                    .collect();
+                let name = name.trim();
+                (!name.is_empty()).then(|| name.to_owned().into())
+            }
+            _ => key.note().map(Into::into),
+        }
+    }
+
+    /// The edit that sets On launch's value, at `span` of `working`, to
+    /// `literal`: with its note beside it, in place of the comment there,
+    /// where the value ends its line. In an inline table, where more of
+    /// the table follows, the value alone.
+    fn launch_edit(
+        &self,
+        working: &str,
+        span: std::ops::Range<usize>,
+        literal: String,
+    ) -> (std::ops::Range<usize>, String) {
+        let end = working[span.end..]
+            .find(['\r', '\n'])
+            .map_or(working.len(), |at| span.end + at);
+        let rest = working[span.end..end].trim_start_matches([' ', '\t']);
+        if !(rest.is_empty() || rest.starts_with('#')) {
+            return (span, literal);
+        }
+        let with = match self.note(Key::OnLaunch) {
+            Some(note) => format!("{literal}  # {note}"),
+            None => literal,
+        };
+        (span.start..end, with)
+    }
+
     /// The other value of an option that has two; `None` for one with more.
     pub fn flipped(&self, option: OptionId) -> Option<OptionValue> {
         match option {
-            OptionId::PageSize | OptionId::Tz | OptionId::IdleDisconnect => None,
+            OptionId::PageSize | OptionId::Tz | OptionId::IdleDisconnect | OptionId::OnLaunch => {
+                None
+            }
             OptionId::Timestamps => Some(OptionValue::Timestamps(match self.timestamps {
                 Timestamps::Second => Timestamps::Full,
                 Timestamps::Full => Timestamps::Second,
@@ -1411,6 +1567,7 @@ impl Settings {
     /// out: a theme that is not set, and stripes left to the look.
     fn written(&self, key: Key) -> Option<String> {
         Some(match key {
+            Key::OnLaunch => toml_string(&self.on_launch.name()),
             Key::RestoreTabs => self.restore_tabs.to_string(),
             Key::PageSize => self.page_size.to_string(),
             Key::Timestamps => toml_string(self.timestamps.name()),
@@ -1498,7 +1655,7 @@ impl Settings {
         let mut emptied: Vec<&str> = Vec::new();
         let line = |key: Key, literal: &str| {
             let name = key.path().1;
-            match key.note() {
+            match self.note(key) {
                 Some(note) => format!("{name} = {literal}  # {note}{newline}"),
                 None => format!("{name} = {literal}{newline}"),
             }
@@ -1538,12 +1695,20 @@ impl Settings {
                     .iter()
                     .all(|(_, item)| is_subtable(&working, item.get_ref(), item.span()));
             match (items.get_key_value(name), literal) {
+                // The comment beside On launch is the app's: it names the
+                // connection, or the values there are.
+                (Some((_, value)), Some(literal)) if key == Key::OnLaunch => {
+                    edits.push(self.launch_edit(&working, value.span(), literal));
+                }
                 (Some((_, value)), Some(literal)) => edits.push((value.span(), literal)),
                 // The default in place: taken out, the rejected line of
                 // the same key would be the one read.
                 (Some((_, value)), None) if shadowed.contains(&key) => {
                     if let Some(default) = Settings::default().written(key) {
-                        edits.push((value.span(), default));
+                        edits.push(match key {
+                            Key::OnLaunch => self.launch_edit(&working, value.span(), default),
+                            _ => (value.span(), default),
+                        });
                     }
                 }
                 (Some((found, value)), None) => {
@@ -1645,6 +1810,9 @@ impl Settings {
         let flag = value.as_bool();
         let text = value.as_str();
         match key {
+            Key::OnLaunch => text
+                .and_then(Launch::from_name)
+                .map(|launch| self.on_launch = launch),
             Key::RestoreTabs => flag.map(|flag| self.restore_tabs = flag),
             Key::PageSize => number.map(|number| self.page_size = number),
             Key::Timestamps => text
@@ -1758,6 +1926,16 @@ impl Settings {
             // in `working` as in `text`.
             let line = line_of(&working, found.span().start);
             if settings.take(key, value.get_ref()) {
+                // A chosen connection's name is the comment beside it.
+                if let Launch::Connection { name, .. } = &mut settings.on_launch
+                    && key == Key::OnLaunch
+                {
+                    let comment = comment_after(&working, value.span().end);
+                    // The note of the other values names no connection.
+                    if Some(comment.as_str()) != Key::OnLaunch.note() {
+                        *name = comment;
+                    }
+                }
                 lines.push((key, line));
                 kept.retain(|other| *other != key);
             } else {
@@ -1782,6 +1960,7 @@ impl Settings {
     /// Takes `key`'s value from `other`.
     fn keep(&mut self, key: Key, other: &Settings) {
         match key {
+            Key::OnLaunch => self.on_launch = other.on_launch.clone(),
             Key::RestoreTabs => self.restore_tabs = other.restore_tabs,
             Key::PageSize => self.page_size = other.page_size,
             Key::Timestamps => self.timestamps = other.timestamps,
@@ -1848,6 +2027,7 @@ impl Settings {
 impl Default for Settings {
     fn default() -> Self {
         Self {
+            on_launch: Launch::Restore,
             restore_tabs: true,
             page_size: Self::DEFAULT_PAGE_SIZE,
             timestamps: Timestamps::Second,
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
index 161a730..f7c17c3 100644
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -655,7 +655,7 @@ impl Opening {
 
 /// Where a connection points, as the rows say it: host and port, then the
 /// database (a file's name for SQLite).
-fn target(connection: &SavedConnection) -> (String, String) {
+pub(crate) fn target(connection: &SavedConnection) -> (String, String) {
     use tabletist_db::Driver;
     let spec = &connection.spec;
     match spec.driver {
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index c1ec074..1455f41 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -13,18 +13,20 @@ mod terminal;
 use std::borrow::Cow;
 
 use crate::app::App;
+use crate::connections::SavedConnection;
 use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Layout, Scope, Spelled, Stroke};
 use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, SettingsRow};
 use crate::settings::{
-    EnvBar, Idle, Mode, OptionId, OptionValue, RowHeight, Settings, TableList, Theme, ThemeChoice,
-    Timestamps, Zone,
+    EnvBar, Idle, Launch, Mode, OptionId, OptionValue, RowHeight, Settings, TableList, Theme,
+    ThemeChoice, Timestamps, Zone,
 };
 
 /// The label of an option's row. English: the caller translates, and the
 /// terminal look lower-cases.
 pub(super) fn label(option: OptionId) -> &'static str {
     match option {
+        OptionId::OnLaunch => "On launch",
         OptionId::RestoreTabs => "Restore tabs and filters",
         OptionId::PageSize => "Rows per page",
         OptionId::Timestamps => "Timestamps",
@@ -59,7 +61,8 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
         OptionId::Zebra => "Alternate rows",
         OptionId::ColumnTypes => "Column types",
         OptionId::EnvBar => "Connection bar",
-        OptionId::RestoreTabs
+        OptionId::OnLaunch
+        | OptionId::RestoreTabs
         | OptionId::PageSize
         | OptionId::Timestamps
         | OptionId::GroupDigits
@@ -79,6 +82,8 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
         // As the board writes it, though hiding columns is not built: what
         // the option will cover once it is.
         OptionId::RestoreTabs => Some("Per connection, including sort and hidden columns"),
+        // The board's hint: beside the menu it has no room in the sheet.
+        OptionId::OnLaunch => Some("Or: show Connections · open a chosen connection"),
         OptionId::PageSize => Some("Table view; the SQL editor has its own limit"),
         OptionId::ValueTags => Some("Colors for enum, CHECK (…IN…) and boolean columns"),
         OptionId::LogDays => Some("Every call and its SQL, never the rows. On this computer only"),
@@ -124,6 +129,44 @@ pub(super) fn zone_text(locale: Locale, zone: &Zone, system: Option<&str>) -> St
     }
 }
 
+/// What a launch can open, as the menu lists it and the screen steps
+/// through it: the last session, Connections, then each saved connection
+/// in the order they are saved.
+pub(super) fn launch_choices(connections: &[SavedConnection]) -> Vec<Launch> {
+    let mut choices = vec![Launch::Restore, Launch::Connections];
+    choices.extend(connections.iter().map(|saved| Launch::Connection {
+        id: saved.id.clone(),
+        name: saved.name.clone(),
+    }));
+    choices
+}
+
+/// What a launch opens, as the window writes it: a connection by its name
+/// as it is saved now. English but the name. One that is not saved (the
+/// file names another) is said to be gone: the next launch shows
+/// Connections in its place.
+pub(super) fn launch_text(
+    locale: Locale,
+    launch: &Launch,
+    connections: &[SavedConnection],
+) -> String {
+    match launch {
+        Launch::Restore => gettext(locale, "Restore last session").into_owned(),
+        Launch::Connections => gettext(locale, "Show Connections").into_owned(),
+        Launch::Connection { id, .. } => match connections.iter().find(|saved| saved.id == *id) {
+            Some(saved) => saved.name.clone(),
+            None => gettext(locale, "Connection not found").into_owned(),
+        },
+    }
+}
+
+/// Where a saved connection points, as the menu writes it beside its
+/// name: so that three of one name are told apart.
+pub(super) fn place(saved: &SavedConnection) -> String {
+    let (host, database) = crate::ui::picker::target(saved);
+    format!("{host}{database}")
+}
+
 /// How long a connection may go unused, as the menu writes it.
 pub(super) fn idle_text(locale: Locale, choice: Idle) -> String {
     let (one, many, count) = match choice {
@@ -229,14 +272,19 @@ impl Said {
 /// called: what the row draws as segments or as a check, and what a screen
 /// reader and the tests call them.
 /// `named` is the last named zone the session had, which the zone's steps
-/// reach.
+/// reach, and `launches` what On launch steps through.
 pub(super) fn choices(
     option: OptionId,
     settings: &Settings,
     named: Option<&str>,
+    launches: &[Launch],
 ) -> Vec<(Said, OptionValue)> {
     use Said::{Sample, Word};
     match option {
+        OptionId::OnLaunch => vec![
+            (Word("fewer"), settings.stepped_launch(launches, false)),
+            (Word("more"), settings.stepped_launch(launches, true)),
+        ],
         OptionId::Tz => vec![
             (Word("fewer"), settings.stepped_zone(named, false)),
             (Word("more"), settings.stepped_zone(named, true)),
@@ -522,6 +570,10 @@ fn keys(app: &App, ctx: &egui::Context, dialog: &SettingsDialog, actions: &mut V
                         Asked::Step { forward } if option == OptionId::Tz => {
                             Some(settings.stepped_zone(app.named_zone.as_deref(), forward))
                         }
+                        Asked::Step { forward } if option == OptionId::OnLaunch => {
+                            let launches = launch_choices(&app.connections.connections);
+                            Some(settings.stepped_launch(&launches, forward))
+                        }
                         Asked::Step { forward } => Some(settings.stepped(option, forward)),
                         _ => settings.flipped(option),
                     };
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index 8dcf6c1..3150f77 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -10,7 +10,7 @@ use crate::app::App;
 use crate::i18n::{Locale, gettext, ngettext};
 use crate::model::{Action, SettingsDialog, SettingsPane};
 use crate::settings::{
-    EnvBar, Idle, Mode, OptionId, OptionValue, RowHeight, Settings, TableList, ThemeChoice,
+    EnvBar, Idle, Launch, Mode, OptionId, OptionValue, RowHeight, Settings, TableList, ThemeChoice,
     Timestamps, Zone,
 };
 use crate::theme::{Look, Palette};
@@ -81,6 +81,8 @@ pub(super) struct Skin<'a> {
     /// The zone database's names, which the zone menu lists: none until
     /// the backend has read it.
     pub(super) zones: &'a [String],
+    /// The saved connections, which On launch lists.
+    pub(super) connections: &'a [crate::connections::SavedConnection],
 }
 
 impl Skin<'_> {
@@ -103,6 +105,7 @@ pub(super) fn show(
         locale: app.locale,
         system: app.system_zone.iana_name(),
         zones: app.zones.as_deref().map_or(&[], crate::zones::Zones::names),
+        connections: &app.connections.connections,
     };
     let screen = ctx.content_rect();
     let width = WIDTH.min(screen.width() - 2.0 * MARGIN).max(NAV + 320.0);
@@ -626,7 +629,7 @@ fn keyboard_came_to(ui: &Ui, place: Rect) -> bool {
 /// How tall `option`'s control is: its row is at least as tall.
 fn control_height(option: OptionId, look: &Look) -> f32 {
     match option {
-        OptionId::PageSize | OptionId::Tz => look.control_height,
+        OptionId::PageSize | OptionId::Tz | OptionId::OnLaunch => look.control_height,
         // The track has 2 over its segments and under them.
         OptionId::Timestamps | OptionId::GroupDigits => SEGMENT + 4.0,
         OptionId::RowHeight | OptionId::TableList | OptionId::EnvBar => SEGMENT + 4.0,
@@ -688,6 +691,7 @@ fn control(
             });
         }
         OptionId::Tz => zone_menu(ui, &name, settings, skin, actions),
+        OptionId::OnLaunch => launch_menu(ui, &name, settings, skin, actions),
         OptionId::Timestamps => {
             let words = [skin.say("To the second"), skin.say("Full precision")];
             let selected = usize::from(settings.timestamps == Timestamps::Full);
@@ -868,6 +872,49 @@ fn page_sizes(current: u32) -> Vec<u32> {
     sizes
 }
 
+/// What a launch opens: the last session, Connections, or a saved
+/// connection, each with where it points beside its name.
+fn launch_menu(
+    ui: &mut Ui,
+    name: &str,
+    settings: &Settings,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Skin { look, palette, .. } = *skin;
+    let shown = super::launch_text(skin.locale, &settings.on_launch, skin.connections);
+    let dropdown = menu::Dropdown::new("launch", &shown, ZONE_MENU);
+    let menu = dropdown.show(ui, look, palette, |ui| {
+        for launch in super::launch_choices(skin.connections) {
+            let text = super::launch_text(skin.locale, &launch, skin.connections);
+            let place = match &launch {
+                Launch::Connection { id, .. } => skin
+                    .connections
+                    .iter()
+                    .find(|saved| saved.id == *id)
+                    .map(super::place)
+                    .unwrap_or_default(),
+                Launch::Restore | Launch::Connections => String::new(),
+            };
+            let entry_name = format!("{name} {text} {place}");
+            let mut entry =
+                menu::Item::choice(&text, launch == settings.on_launch).name(entry_name.trim_end());
+            if !place.is_empty() {
+                entry = entry.detail(&place);
+            }
+            if entry.show(ui, look, palette).clicked() {
+                actions.push(Action::SetOption(OptionValue::OnLaunch(launch)));
+                ui.close();
+            }
+        }
+    });
+    menu.response.widget_info(|| {
+        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, name);
+        info.current_text_value = Some(shown.clone());
+        info
+    });
+}
+
 /// The zone's menu: the computer's zone and UTC, then every zone of the
 /// database, which a search field over them narrows. What was typed there
 /// is the field's own, kept in egui's memory while the menu is open.
@@ -945,6 +992,7 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
         // What the Appearance pane's rows say is their small print, and its
         // tiles have their own line.
         OptionId::Theme
+        | OptionId::OnLaunch
         | OptionId::RestoreTabs
         | OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index bbf2b62..f8bafe0 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -59,6 +59,9 @@ pub(super) struct Skin<'a> {
     pub(super) system: Option<&'a str>,
     /// The last named zone the session had, which the zone's steps reach.
     pub(super) named: Option<&'a str>,
+    /// The saved connections, and what On launch steps through.
+    pub(super) connections: &'a [crate::connections::SavedConnection],
+    pub(super) launches: &'a [crate::settings::Launch],
     /// What the theme's rows say of the desktop.
     pub(super) desktop: Desktop<'a>,
 }
@@ -121,6 +124,7 @@ pub(super) fn show(
 ) {
     let (pane, row) = (dialog.pane, dialog.row);
     let settings = super::shown(app);
+    let launches = super::launch_choices(&app.connections.connections);
     let skin = Skin {
         look: &app.look,
         palette: &app.palette,
@@ -128,6 +132,8 @@ pub(super) fn show(
         window: app.agents_window().is_some(),
         system: app.system_zone.iana_name(),
         named: app.named_zone.as_deref(),
+        connections: &app.connections.connections,
+        launches: &launches,
         desktop: Desktop::of(app),
     };
     let screen = ctx.content_rect();
@@ -809,7 +815,7 @@ fn widths(ui: &Ui, options: &[OptionId], settings: &Settings, skin: &Skin) -> (f
         .fold(0.0, f32::max);
     let hints = options
         .iter()
-        .flat_map(|option| choices(*option, settings, skin.named))
+        .flat_map(|option| choices(*option, settings, skin.named, skin.launches))
         .map(|(_, value)| {
             let mut changed = settings.clone();
             value.set(&mut changed);
@@ -854,16 +860,18 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
     let width = |text: &str| role.width(ui.ctx(), skin.look.faces, text);
     match option {
         // Between its chevrons, as the cursor's row has it.
-        OptionId::PageSize | OptionId::LogDays | OptionId::Tz | OptionId::IdleDisconnect => {
-            width(&format!("‹ {} ›", stepped_text(option, settings, skin)))
-        }
+        OptionId::PageSize
+        | OptionId::LogDays
+        | OptionId::Tz
+        | OptionId::IdleDisconnect
+        | OptionId::OnLaunch => width(&format!("‹ {} ›", stepped_text(option, settings, skin))),
         OptionId::Timestamps
         | OptionId::GroupDigits
         | OptionId::Theme
         | OptionId::RowHeight
         | OptionId::TableList
         | OptionId::EnvBar => {
-            let words = choices(option, settings, skin.named);
+            let words = choices(option, settings, skin.named, skin.launches);
             let between = width(" ") * words.len().saturating_sub(1) as f32;
             let segments = words.into_iter().map(|(word, _)| segment(ui, word, skin));
             segments.sum::<f32>() + between
@@ -898,6 +906,23 @@ fn stepped_text(option: OptionId, settings: &Settings, skin: &Skin) -> String {
             }
             (zone, system) => super::zone_text(skin.locale, zone, system),
         }),
+        // A connection by its name as it is saved, and where it points:
+        // three of one name are told apart.
+        OptionId::OnLaunch => {
+            let text = super::launch_text(skin.locale, &settings.on_launch, skin.connections);
+            match &settings.on_launch {
+                crate::settings::Launch::Connection { id, .. } => {
+                    let saved = skin.connections.iter().find(|saved| saved.id == *id);
+                    match saved.map(super::place).filter(|place| !place.is_empty()) {
+                        Some(place) => format!("{text} ({place})"),
+                        None => text,
+                    }
+                }
+                crate::settings::Launch::Restore | crate::settings::Launch::Connections => {
+                    skin.look.label(&text)
+                }
+            }
+        }
         _ => settings.page_size.to_string(),
     }
 }
@@ -1010,7 +1035,11 @@ fn option_row(
     if let Some(button) = button
         && matches!(
             option,
-            OptionId::PageSize | OptionId::LogDays | OptionId::Tz | OptionId::IdleDisconnect
+            OptionId::PageSize
+                | OptionId::LogDays
+                | OptionId::Tz
+                | OptionId::IdleDisconnect
+                | OptionId::OnLaunch
         )
     {
         let name = gettext(locale, terminal_label(option));
@@ -1032,9 +1061,11 @@ fn option_row(
         on_cursor,
     };
     match option {
-        OptionId::PageSize | OptionId::LogDays | OptionId::Tz | OptionId::IdleDisconnect => {
-            stepper(ui, cell, option, settings, skin, actions)
-        }
+        OptionId::PageSize
+        | OptionId::LogDays
+        | OptionId::Tz
+        | OptionId::IdleDisconnect
+        | OptionId::OnLaunch => stepper(ui, cell, option, settings, skin, actions),
         OptionId::Timestamps
         | OptionId::GroupDigits
         | OptionId::Theme
@@ -1078,7 +1109,9 @@ fn stepper(
     let y = cell.row.center().y;
     let value = stepped_text(option, settings, skin);
     let current = settings.value(option);
-    let Ok([fewer, more]) = <[_; 2]>::try_from(choices(option, settings, skin.named)) else {
+    let Ok([fewer, more]) =
+        <[_; 2]>::try_from(choices(option, settings, skin.named, skin.launches))
+    else {
         return;
     };
     let text = if cell.on_cursor {
@@ -1153,7 +1186,7 @@ fn check(
     );
     // One mark stands for both values, and a click on it flips it: the
     // value it is not set to is the button on top.
-    let mut values = choices(option, settings, skin.named);
+    let mut values = choices(option, settings, skin.named, skin.launches);
     values.sort_by_key(|(_, value)| *value != current);
     for (word, value) in values {
         let press = Press::Value {
@@ -1194,6 +1227,7 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
                 .add(role, "ebook", second)
         }
         OptionId::RestoreTabs => dim(&skin.say("per connection")),
+        OptionId::OnLaunch => dim(&skin.say("or connections · a chosen connection")),
         OptionId::ProductionReadOnly => dim(&skin.say("default for new connections")),
         // As drawn: nothing to add.
         OptionId::ConfirmCloseEdits => Text::new(look),
@@ -1426,7 +1460,7 @@ fn segments(
     let line = role.row_height(ui.ctx(), look.faces);
     let gap = role.width(ui.ctx(), look.faces, " ");
     let mut left = cell.left;
-    for (word, value) in choices(option, settings, skin.named) {
+    for (word, value) in choices(option, settings, skin.named, skin.launches) {
         let text = skin.written(word);
         let width = segment(ui, word, skin);
         let place = Rect::from_min_size(
````

- [ ] **Step 4: Check**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: no warning, every test passes.

- [ ] **Step 5: Commit**

```bash
git add docs/_reference/settings-and-files.md src/app.rs src/app/startup_tests.rs src/model.rs src/settings.rs src/ui/mod.rs src/ui/picker.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs
git commit -m "Add On launch to Settings › General"
```

## After the tasks: what the reviews of the run found

Each task was reviewed for the plan and for the code as it was run, and the three together once more; the fixes are commits of their own after each task, each with its tests, and every commit passes the three checks. The diffs above do not have them.

- **After task 1** (`4b1a09f`): a file a newer version wrote keeps its `version` when it is written back, so a newer reader that goes by it does not misread what was kept for it; `active` counts among the windows read, so a window of the wrong kind left out does not move the one in front; the log line for an entry that cannot be read names what kind of error it is and quotes no stored value.
- **After task 2** (`6201969`): a reopened table read first by another way than coming to the front (following a foreign key into it) gets its structure too: the first `fetch_rows` of a reopened tab describes it, and `after_connect` does not describe it a second time. A reopened table gets the row estimate of the tree once the tree's list arrives (`Event::Objects`), as one opened from the sidebar has it. Connecting a connection into its own window reopens what that window had just now.
- **After task 3** (the last commits): the window in front connects first, but a question is asked of the first window that asks one: a window behind whose password is asked for at once is asked before a front window still reading the keyring, and the other says so and offers Reconnect. This replaces the row above that says the front window is always asked first. The closed On launch menu names the chosen connection's server and database too, so a screen reader tells two Bookshops apart; "connection not found" is lower case on Omarchy. Switching database keeps the window's tables of its own database first (they were dropped), and a connection taken out of `connections.toml` by hand is taken out of `session.json` at its next write. The website says that the comment beside `startup.on_launch` is Tabletist's, and when `session.json` is not written.

## Pull request

One pull request for the three commits, on `main`. Its description has:

- a summary;
- the two rows and what each does, and `session.json`: what it keeps, and when it is written;
- the departures above, and the spec's;
- what was not run: CI, macOS and Windows, and the app by hand;
- that the website's `macos-settings` and `omarchy-settings` pictures show General without its Startup group until they are rendered again from `src/shots.rs`.

No screenshot unless blurred.
