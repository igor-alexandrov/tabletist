# DDL view, run 2: act on it. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Copy and Open in SQL editor on the DDL view, its jump links and the Omarchy outline, and every key of the views and of the text, as the spec says.

**Architecture:** `Ddl::laid_out` gives the text with each part's lines, which the links and the outline use. The keymap gains the views' and the DDL view's commands, a `When::Ddl`, and Windows' own keys on a binding; `src/ui/keys.rs` turns them into actions. The tab holds the terminal's cursor and its selected lines; `src/app.rs` opens the text in a new SQL editor and answers `:ddl`. `src/ui/ddl.rs` draws the buttons, the links, the terminal's lines and its outline; `src/ui/workspace.rs` the terminal's status line.

**Tech Stack:** Rust, egui (crmne fork).

**Spec:** `docs/superpowers/specs/2026-10-25-ddl-view-2-act-design.md` (run 1's: `docs/superpowers/specs/2026-10-24-ddl-view-design.md`). **Base:** `2af7ec75` (run 1, pull request #222).

---

## Run 2 of three

Run 1 read and showed the definition. This run acts on it; run 3 (find in the text, the macOS View menu) gets a plan of its own. One pull request per run, based on run 1's branch until it merges.

## One task

The run is one task. Its pieces (the keymap's lines, the keys' handling in `src/ui/keys.rs`, the tab's cursor, the header's buttons, the links and the outline) are interleaved in the same files and the same functions, and the headless tests of each go through the others (a key needs the keymap, the handling and the view). Split, each task would carry half-written hunks of the next.

## What the design asks, and what is built

| The board or the prompt | Built | Why |
| --- | --- | --- |
| Omarchy's text as a field | A view of lines with a cursor | The terminal moves by line and selects whole lines (`V`) |
| `/ search` in the outline and the key line | Left out | Find is run 3 |
| The production badge in the DDL view's header | In every view's header | The table is on production whatever the view |
| A Cmd chord on a Mac holds with Control or without (`held_is`) | Only with Control as the chord says | ⌃⌘1 would be ⌘1 (Go to connection window 1); decided with Igor (2026-10-25) |
| The board's key line | The board's, then `:w` write, `:diff` review, what is pending and the read-only tag | Run 1's spec: what is pending is saved and reviewed from the view |
| `R` nowhere on the board | In the outline's keys, "read again" | Reload is a key of the view too |
| `table N` in the outline | The object's line count | The object is one statement |
| Open in SQL editor of a selection | The whole text | The board's button names no selection |
| `:ddl` with no table to show (a SQL editor in front) | Nothing happens | The terminal's line says what the prompt refused only over a table |
| A link always takes its part to the top | One near the end stays marked where it stops | A text cannot scroll past its end |
| Three tasks (keys, copy and open, links and outline) | One task | Above |

Beyond the boards, and changes to what was there:

- Esc in the DDL view is its own only while lines are selected; with none it still closes Review SQL.
- The row panel's `[` and `]` are not read in the DDL view (`When::InspectorOpen` does not hold there): the brackets begin `[[` and `]]`.
- On macOS ⇧⌘C copies the DDL in the DDL view and the row on the rows.
- Every toast is now a polite live region: a screen reader reads it out.
- The rows' `v y` copies a cell only where the rows show (before, from Structure too).
- Counts are not read in the DDL view: `3j` moves one line.
- Nothing copies, opens or selects a text the view does not show (a failed Reload keeps its old text under the error).

## Checks

The task ends with these, all passing:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist ~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

The server tests need PostgreSQL on 55432 and MySQL on 53306 (`ss -ltn`); without the two URLs they print "skipped" and pass. Nothing in this run reads a server.

## File structure

- `crates/tabletist-db/src/ddl.rs`: `Ddl::laid_out`, the text with each part's lines.
- `src/keymap.rs`, `src/keymap/overrides.rs`: the commands, `When::Ddl`, `Binding::windows`, `cc` alone for Replace cell.
- `src/model.rs`, `src/app.rs`, `src/app/editing.rs`: the tab's cursor and selected lines, the actions, Open in SQL editor and `:ddl`.
- `src/ui/keys.rs`: the views' chords and letters, the DDL view's keys, `held_is`'s Ctrl.
- `src/ui/ddl.rs`: Copy, the header's buttons, the links, the terminal's lines and outline.
- `src/ui/data_view.rs`: the header's buttons, the production badge, the view words' keys and tooltips. `src/env.rs`: its test lists `data_view.rs` among the places that paint the environment's colour.
- `src/ui/workspace.rs`: the terminal's status line. `src/ui/toast.rs`: the polite region.
- `src/ui/ddl_act_tests.rs` (new): the headless tests. `src/ui/ddl_tests.rs`, `src/ui/mod.rs`: helpers shared, and the tests that read `s` as Replace cell.
- `docs/_guide/browsing-data.md`, `docs/_guide/omarchy.md`, `docs/_guide/macos.md`, `docs/_reference/keyboard-shortcuts.md`: the user guide and the keys. `AGENTS.md`: Windows' own keys on a keymap line.

---

### Before Task 1: the spec and the plan

- [ ] Commit the spec and this plan:

```bash
git add docs/superpowers/specs/2026-10-25-ddl-view-2-act-design.md docs/superpowers/plans/2026-10-25-ddl-view-2-act.md
git commit -m "Spec and plan the DDL view, run 2"
```

---

### Task 1: Act on the DDL

**Files:**
- Modify: `AGENTS.md`
- Modify: `crates/tabletist-db/src/ddl.rs`
- Modify: `docs/_guide/browsing-data.md`
- Modify: `docs/_guide/macos.md`
- Modify: `docs/_guide/omarchy.md`
- Modify: `docs/_reference/keyboard-shortcuts.md`
- Modify: `src/app.rs`
- Modify: `src/app/editing.rs`
- Modify: `src/env.rs`
- Modify: `src/keymap.rs`
- Modify: `src/keymap/overrides.rs`
- Modify: `src/model.rs`
- Modify: `src/ui/data_view.rs`
- Modify: `src/ui/ddl.rs`
- Create: `src/ui/ddl_act_tests.rs`
- Modify: `src/ui/ddl_tests.rs`
- Modify: `src/ui/keys.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/toast.rs`
- Modify: `src/ui/workspace.rs`

- [ ] **Step 1: Write the failing tests**

`Ddl::laid_out`'s ranges; the keymap's labels and letters; the headless tests of `src/ui/ddl_act_tests.rs`, with the helpers of `src/ui/ddl_tests.rs` they share; the tests in `src/ui/mod.rs` that read `s` as Replace cell; `src/env.rs`'s list of the places that paint the environment's colour. `held_is`'s test in `src/ui/keys.rs` says ⌃⌘ is not ⌘ on a Mac.

````diff
diff --git a/crates/tabletist-db/src/ddl.rs b/crates/tabletist-db/src/ddl.rs
index c91c05d3..424383db 100644
--- a/crates/tabletist-db/src/ddl.rs
+++ b/crates/tabletist-db/src/ddl.rs
@@ -193,6 +193,15 @@ mod tests {
         );
         // The empty part is no part.
         assert_eq!(ddl.part(PartKind::Triggers), None);
+        // Each part's lines, from its comment to its last line.
+        assert_eq!(
+            ddl.laid_out().1,
+            [
+                (PartKind::Object, 0..4),
+                (PartKind::Indexes, 5..8),
+                (PartKind::Comments, 9..11),
+            ]
+        );
         assert_eq!(ddl.source(), Source::Catalog);
     }
 
@@ -246,6 +255,15 @@ mod tests {
              \n\
              CREATE TRIGGER x AFTER UPDATE ON t BEGIN SELECT 1; END;"
         );
+        // With no comments, a part starts at its first statement.
+        assert_eq!(
+            ddl.laid_out().1,
+            [
+                (PartKind::Object, 0..1),
+                (PartKind::Indexes, 2..3),
+                (PartKind::Triggers, 4..5),
+            ]
+        );
     }
 
     /// A statement whose last line ends in a `--` comment gets its `;` on
diff --git a/src/env.rs b/src/env.rs
index b65a3aa1..a6a029c7 100644
--- a/src/env.rs
+++ b/src/env.rs
@@ -896,6 +896,8 @@ mod tests {
         let surfaces = [
             OWNER,
             "src/ui/workspace.rs",
+            // The table's header names production beside the table.
+            "src/ui/data_view.rs",
             "src/ui/picker.rs",
             "src/ui/connect_dialog/mod.rs",
             "src/ui/env_tests.rs",
diff --git a/src/keymap.rs b/src/keymap.rs
index cf128f4f..d7dd798b 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -4030,7 +4030,7 @@ mod tests {
     #[test]
     fn every_spelling_reads() {
         for binding in BINDINGS {
-            for bound in [&binding.mac, &binding.omarchy] {
+            for bound in binding.every_bound() {
                 for chord in bound.chords {
                     assert!(spell(chord.keys).is_ok(), "{:?}", spell(chord.keys));
                 }
@@ -4104,7 +4104,7 @@ mod tests {
             assert!(!command.info().name.is_empty(), "{command:?}");
             for line in lines {
                 assert!(!line.scopes.is_empty(), "{command:?}");
-                for bound in [&line.mac, &line.omarchy] {
+                for bound in line.every_bound() {
                     // Keys, or what does it in their place, never both.
                     assert!(
                         bound.chords.is_empty() || bound.control.is_empty(),
@@ -4227,7 +4227,8 @@ mod tests {
         }
         // The Mac's layout names no Omarchy letters' modifier by mistake.
         for binding in BINDINGS {
-            for chord in binding.mac.chords {
+            let windows = binding.windows.iter().flat_map(|bound| bound.chords);
+            for chord in binding.mac.chords.iter().chain(windows) {
                 assert!(!chord.keys.contains("super"), "{}", chord.keys);
             }
         }
@@ -4385,7 +4386,28 @@ mod tests {
         assert_eq!(mac(Command::GoToConnectionWindow), "⌘1…9");
         assert_eq!(omarchy(Command::GoToConnectionWindow), "ctrl+shift+1..9");
         assert_eq!(omarchy(Command::EditCell), "i, enter");
-        assert_eq!(omarchy(Command::ReplaceCell), "cc, s");
+        assert_eq!(omarchy(Command::ReplaceCell), "cc");
+        // A table's views and the DDL view's own keys. Windows' Ctrl with a
+        // digit goes to a connection's window: its views' keys are Alt's.
+        let windows = |command| keymap.label(Layout::Windows, command);
+        for (command, chord, alt, letter) in [
+            (Command::ShowRows, "⌃⌘1", "Alt+1", "r"),
+            (Command::ShowStructure, "⌃⌘2", "Alt+2", "s"),
+            (Command::ShowDdl, "⌃⌘3", "Alt+3", "S"),
+        ] {
+            assert_eq!(mac(command), chord);
+            assert_eq!(windows(command), alt);
+            assert_eq!(omarchy(command), letter);
+        }
+        assert_eq!(windows(Command::GoToConnectionWindow), "Ctrl+1…9");
+        assert_eq!(mac(Command::CopyDdl), "⇧⌘C");
+        assert_eq!(windows(Command::CopyDdl), "Ctrl+Shift+C");
+        assert_eq!(omarchy(Command::CopyDdl), "y");
+        assert_eq!(mac(Command::OpenDdlInEditor), "⌥↩");
+        assert_eq!(omarchy(Command::OpenDdlInEditor), "e");
+        assert_eq!(omarchy(Command::OpenDdl), ":ddl");
+        assert_eq!(omarchy(Command::NextDdlSection), "]]");
+        assert_eq!(omarchy(Command::PreviousDdlSection), "[[");
         assert_eq!(omarchy(Command::PasteNewRows), "\"+p, ctrl+shift+v");
         assert_eq!(omarchy(Command::DuplicateConnection), "yy p");
         assert_eq!(omarchy(Command::DuplicateRow), "yy p");
@@ -4436,7 +4458,11 @@ mod tests {
         assert_eq!(omarchy(Scope::Grid, "d"), Waiting);
         assert_eq!(omarchy(Scope::Grid, "dd"), command(Command::DeleteRow));
         assert_eq!(omarchy(Scope::Grid, "i"), command(Command::EditCell));
-        assert_eq!(omarchy(Scope::Grid, "s"), command(Command::ReplaceCell));
+        // `s` is the Structure view's, from any view; `r` and `S` the
+        // rows' and the DDL's.
+        assert_eq!(omarchy(Scope::Grid, "s"), command(Command::ShowStructure));
+        assert_eq!(omarchy(Scope::Grid, "r"), command(Command::ShowRows));
+        assert_eq!(omarchy(Scope::Grid, "S"), command(Command::ShowDdl));
         assert_eq!(omarchy(Scope::Grid, "cc"), command(Command::ReplaceCell));
         assert_eq!(omarchy(Scope::Grid, "x"), command(Command::SetNull));
         assert_eq!(
diff --git a/src/ui/ddl_act_tests.rs b/src/ui/ddl_act_tests.rs
new file mode 100644
index 00000000..ac7896f5
--- /dev/null
+++ b/src/ui/ddl_act_tests.rs
@@ -0,0 +1,774 @@
+//! What the DDL view does with its text, checked against its spec
+//! (`docs/superpowers/specs/2026-10-25-ddl-view-2-act-design.md`): Copy and
+//! Open in SQL editor, the links to its parts and the terminal's outline,
+//! the views' keys, and the terminal look's lines.
+
+use egui::accesskit::Role;
+use egui::{Key, Modifiers};
+use tabletist_db::{Driver, ObjectKind, ObjectRef};
+
+use super::ddl_tests::{book_covers, covers, ddl_view, focus_text, letter, reads, set_view};
+use super::tests::{type_key, type_text};
+use crate::backend::Command;
+use crate::model::{Action, ConnTabId, ObjectView, RunMode, TabId};
+use crate::testing::{Harness, node};
+use crate::theme::Look;
+
+/// ⌃⌘ as a Mac reports it: Control, and the command key by both names.
+const CONTROL_CMD: Modifiers = Modifiers {
+    alt: false,
+    ctrl: true,
+    shift: false,
+    mac_cmd: true,
+    command: true,
+};
+
+/// ⌘ alone, as a Mac reports it.
+const CMD: Modifiers = Modifiers {
+    alt: false,
+    ctrl: false,
+    shift: false,
+    mac_cmd: true,
+    command: true,
+};
+
+fn view(harness: &Harness, tab: ConnTabId, id: TabId) -> ObjectView {
+    harness
+        .app
+        .workspace(tab)
+        .unwrap()
+        .object_tab(id)
+        .unwrap()
+        .view
+}
+
+fn cursor(harness: &Harness, tab: ConnTabId, id: TabId) -> (usize, Option<usize>) {
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    (object.ddl_cursor, object.ddl_visual)
+}
+
+/// What the toast or the terminal's line says now.
+fn said(harness: &Harness) -> Option<String> {
+    crate::ui::toast::said(&harness.ctx)
+}
+
+/// The lines `from` to `to` of PostgreSQL's `book_covers`, as copied.
+fn lines(from: usize, to: usize) -> String {
+    let text = book_covers(Driver::Postgres).text();
+    text.lines()
+        .skip(from)
+        .take(to - from)
+        .collect::<Vec<_>>()
+        .join("\n")
+}
+
+/// Copy copies all of the text where nothing is selected, and says so to a
+/// screen reader too.
+#[test]
+fn copy_copies_all_of_it_and_says_so() {
+    for look in [Look::macos(), Look::standard()] {
+        let (mut harness, _, _) = ddl_view(look, Driver::Postgres);
+        harness.click("Copy");
+        assert_eq!(
+            harness.copied.as_deref(),
+            Some(book_covers(Driver::Postgres).text().as_str()),
+            "{}",
+            look.name
+        );
+        assert_eq!(said(&harness).as_deref(), Some("Copied DDL"));
+        let tree = harness.settle();
+        let toast = node(&tree, "Copied DDL", Role::Label).expect("the toast");
+        let (_, toast) = tree.nodes.iter().find(|(id, _)| *id == toast).unwrap();
+        assert_eq!(toast.live(), Some(egui::accesskit::Live::Polite));
+    }
+}
+
+/// Copy copies only what is selected where something is: the button and
+/// the chord alike.
+#[test]
+fn copy_copies_the_selection_where_there_is_one() {
+    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
+    focus_text(&mut harness, tab, id);
+    harness.press(Key::ArrowDown, Modifiers::SHIFT);
+    harness.press(Key::ArrowDown, Modifiers::SHIFT);
+    harness.click("Copy");
+    assert_eq!(
+        harness.copied.as_deref(),
+        Some(format!("{}\n", lines(0, 2)).as_str())
+    );
+    assert_eq!(said(&harness).as_deref(), Some("Copied 2 lines"));
+    // ⇧⌘C, from the text: the same.
+    harness.copied = None;
+    focus_text(&mut harness, tab, id);
+    harness.copy(true);
+    assert_eq!(
+        harness.copied.as_deref(),
+        Some(format!("{}\n", lines(0, 2)).as_str())
+    );
+}
+
+/// ⇧⌘C copies all of it with the keyboard anywhere in the view, and
+/// Ctrl+Shift+C on Windows.
+#[test]
+fn the_chord_copies_all_of_it() {
+    for look in [Look::macos(), Look::standard()] {
+        let (mut harness, _, _) = ddl_view(look, Driver::Postgres);
+        harness.copy(true);
+        assert_eq!(
+            harness.copied.as_deref(),
+            Some(book_covers(Driver::Postgres).text().as_str()),
+            "{}",
+            look.name
+        );
+    }
+}
+
+/// The SQL editors open and in front.
+fn sql_tabs(harness: &Harness, tab: ConnTabId) -> Vec<(String, RunMode)> {
+    let workspace = harness.app.workspace(tab).unwrap();
+    workspace
+        .sql_tabs()
+        .map(|sql| (sql.text.clone(), sql.mode))
+        .collect()
+}
+
+/// Open in SQL editor puts the text in a new SQL editor, in front, which
+/// runs nothing. It opens as Settings › Editor says.
+#[test]
+fn open_in_sql_editor_opens_a_new_tab_that_runs_nothing() {
+    for writable in [false, true] {
+        let (mut harness, tab, _) = ddl_view(Look::macos(), Driver::Postgres);
+        harness.app.workspace_mut(tab).unwrap().access = match writable {
+            true => tabletist_db::Access::Writable,
+            false => tabletist_db::Access::ReadOnly,
+        };
+        harness.app.settings.new_tab = crate::settings::NewTab::Connection;
+        let from = harness.app.backend.sent.len();
+        harness.click("Open in SQL editor");
+        let text = book_covers(Driver::Postgres).text();
+        // On a connection that cannot write it is read-only, whatever the
+        // settings say.
+        let mode = if writable {
+            RunMode::ReadWrite
+        } else {
+            RunMode::ReadOnly
+        };
+        assert_eq!(
+            sql_tabs(&harness, tab),
+            [(text, mode)],
+            "writable {writable}"
+        );
+        let workspace = harness.app.workspace(tab).unwrap();
+        assert!(workspace.active_sql_tab().is_some());
+        assert!(
+            !harness.app.backend.sent[from..]
+                .iter()
+                .any(|command| matches!(command, Command::RunSql { .. }))
+        );
+    }
+}
+
+/// On production the new editor is read-only, whatever the settings and
+/// the connection say. ⌥↩ opens it too.
+#[test]
+fn on_production_the_editor_opens_read_only() {
+    let (mut harness, tab, _) = ddl_view(Look::macos(), Driver::Postgres);
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    workspace.access = tabletist_db::Access::Writable;
+    workspace.environment = crate::env::Environment::Production;
+    harness.app.settings.new_tab = crate::settings::NewTab::Connection;
+    harness.press(Key::Enter, Modifiers::ALT);
+    let text = book_covers(Driver::Postgres).text();
+    assert_eq!(sql_tabs(&harness, tab), [(text, RunMode::ReadOnly)]);
+}
+
+/// Production is named beside the table's name, in every view.
+#[test]
+fn production_is_named_beside_the_table() {
+    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
+    assert!(!harness.has("production"));
+    harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
+    for shown in [ObjectView::Ddl, ObjectView::Data, ObjectView::Structure] {
+        set_view(&mut harness, tab, id, shown);
+        assert!(harness.has("production"), "{shown:?}");
+    }
+}
+
+/// ⌃⌘1, ⌃⌘2 and ⌃⌘3 show the rows, the structure and the DDL, wherever
+/// the keyboard is; ⌘ with a digit shows none.
+#[test]
+fn control_cmd_and_a_digit_show_a_view() {
+    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
+    for (key, shown) in [
+        (Key::Num3, ObjectView::Ddl),
+        (Key::Num2, ObjectView::Structure),
+        (Key::Num1, ObjectView::Data),
+    ] {
+        harness.press(key, CONTROL_CMD);
+        assert_eq!(view(&harness, tab, id), shown, "{key:?}");
+    }
+    harness.press(Key::Num3, CMD);
+    assert_eq!(
+        view(&harness, tab, id),
+        ObjectView::Data,
+        "the command key alone with 3 is no view's"
+    );
+    // From the DDL view's text too.
+    harness.press(Key::Num3, CONTROL_CMD);
+    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
+    focus_text(&mut harness, tab, id);
+    harness.press(Key::Num2, CONTROL_CMD);
+    assert_eq!(view(&harness, tab, id), ObjectView::Structure);
+}
+
+/// With two connections open, ⌃⌘2 shows the structure and stays in the
+/// window; ⌘2 still goes to the second connection's.
+#[test]
+fn control_cmd_and_a_digit_is_no_connections_window() {
+    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
+    harness.app.apply(Action::ShowConnections);
+    let mut saved = crate::testing::fixture_connection();
+    saved.id = crate::connections::ConnectionId::new();
+    saved.name = "Other".into();
+    let other = harness.connect_with(saved, crate::testing::fixture_objects());
+    harness.app.apply(Action::ActivateConnection(0));
+    harness.settle();
+    assert_eq!(harness.app.active_tab_id(), tab);
+    harness.press(Key::Num2, CONTROL_CMD);
+    assert_eq!(harness.app.active_tab_id(), tab);
+    assert_eq!(view(&harness, tab, id), ObjectView::Structure);
+    harness.press(Key::Num2, CMD);
+    assert_eq!(harness.app.active_tab_id(), other);
+}
+
+/// On Windows, where Ctrl with a digit goes to a connection's window, the
+/// views' keys are Alt's.
+#[test]
+fn alt_and_a_digit_show_a_view_on_windows() {
+    let (mut harness, tab, id) = covers(Look::standard(), Driver::Postgres);
+    harness.press(Key::Num3, Modifiers::ALT);
+    assert_eq!(view(&harness, tab, id), ObjectView::Ddl);
+    harness.press(Key::Num2, Modifiers::ALT);
+    assert_eq!(view(&harness, tab, id), ObjectView::Structure);
+    harness.press(Key::Num3, Modifiers::COMMAND);
+    assert_eq!(
+        view(&harness, tab, id),
+        ObjectView::Structure,
+        "the command key with 3 goes to a window"
+    );
+}
+
+/// The terminal's `r`, `s` and `S` show the rows, the structure and the
+/// DDL, from any of them.
+#[test]
+fn r_s_and_shift_s_show_a_view_in_the_terminal() {
+    let (mut harness, tab, id) = covers(Look::omarchy(), Driver::Postgres);
+    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
+    letter(&mut harness, 'S');
+    assert_eq!(view(&harness, tab, id), ObjectView::Ddl);
+    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
+    letter(&mut harness, 's');
+    assert_eq!(view(&harness, tab, id), ObjectView::Structure);
+    letter(&mut harness, 'r');
+    assert_eq!(view(&harness, tab, id), ObjectView::Data);
+    // The header names each with its key.
+    for said in ["r rows", "s structure", "S ddl"] {
+        assert!(
+            harness.painted.iter().any(|(text, _)| text == said),
+            "{said}"
+        );
+    }
+}
+
+/// `:ddl` shows the DDL view of the tab in front, or opens the table under
+/// the tree's cursor in it; with neither it is no command there.
+#[test]
+fn ddl_opens_the_view_from_any_pane() {
+    let (mut harness, tab, id) = covers(Look::omarchy(), Driver::Postgres);
+    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
+    run_line(&mut harness, "ddl");
+    assert_eq!(view(&harness, tab, id), ObjectView::Ddl);
+    // From the tree, of the table under its cursor.
+    let orders = ObjectRef::new("main", "orders");
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    workspace.pane = crate::model::Pane::Tree;
+    workspace.tree.cursor = Some(crate::model::TreeNode::Object(
+        orders.clone(),
+        ObjectKind::Table,
+    ));
+    let from = harness.app.backend.sent.len();
+    run_line(&mut harness, "ddl");
+    let workspace = harness.app.workspace(tab).unwrap();
+    let front = workspace.active_object_tab().unwrap();
+    assert_eq!(
+        (front.object.clone(), front.view),
+        (orders.clone(), ObjectView::Ddl)
+    );
+    assert_eq!(reads(&harness, from), [orders]);
+    // With no table or view to show (a SQL editor in front), nothing: no
+    // tab opens or comes in front, and nothing is read.
+    harness.app.apply(Action::NewSqlTab(tab));
+    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
+    let (from, tabs) = (
+        harness.app.backend.sent.len(),
+        harness.app.workspace(tab).unwrap().tabs.len(),
+    );
+    harness.app.apply(Action::OpenDdl(tab));
+    let workspace = harness.app.workspace(tab).unwrap();
+    assert!(workspace.active_sql_tab().is_some());
+    assert_eq!(workspace.tabs.len(), tabs);
+    assert_eq!(workspace.command_error, None);
+    assert!(reads(&harness, from).is_empty());
+}
+
+/// Runs `line` in the terminal's `:` prompt.
+fn run_line(harness: &mut Harness, line: &str) {
+    type_key(harness, Key::Colon, ":");
+    type_text(harness, line);
+    harness.press(Key::Enter, Modifiers::NONE);
+}
+
+/// The terminal's DDL view with the keys on its lines.
+fn terminal_lines() -> (Harness, ConnTabId, TabId) {
+    let (mut harness, tab, id) = ddl_view(Look::omarchy(), Driver::Postgres);
+    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
+    harness.settle();
+    (harness, tab, id)
+}
+
+/// `j` and `k` move the cursor by line, `gg` and `G` to the ends, `]]`
+/// and `[[` to the next part and the one before.
+#[test]
+fn the_terminal_moves_through_the_text_by_line_and_by_part() {
+    let (mut harness, tab, id) = terminal_lines();
+    let at = |harness: &Harness| cursor(harness, tab, id).0;
+    letter(&mut harness, 'j');
+    letter(&mut harness, 'j');
+    assert_eq!(at(&harness), 2);
+    letter(&mut harness, 'k');
+    assert_eq!(at(&harness), 1);
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    assert_eq!(at(&harness), 2);
+    letter(&mut harness, 'G');
+    assert_eq!(at(&harness), 11);
+    letter(&mut harness, 'g');
+    letter(&mut harness, 'g');
+    assert_eq!(at(&harness), 0);
+    // The parts start at lines 0 (the heading), 7 (`-- Indexes`) and 10
+    // (`-- Comments`).
+    for (keys, line) in [("]]", 7), ("]]", 10), ("]]", 10), ("[[", 7), ("[[", 0)] {
+        for c in keys.chars() {
+            letter(&mut harness, c);
+        }
+        assert_eq!(at(&harness), line, "{keys}");
+    }
+    // The cursor's line of all of them, at the line's end.
+    letter(&mut harness, 'j');
+    assert!(harness.painted.iter().any(|(text, _)| text == "2/12"));
+}
+
+/// `V` selects whole lines from the cursor's; `y` copies them and ends
+/// the selection; `y` with none copies all of it. Esc ends it too.
+#[test]
+fn the_terminal_selects_lines_and_copies_them() {
+    let (mut harness, tab, id) = terminal_lines();
+    letter(&mut harness, 'j');
+    letter(&mut harness, 'V');
+    letter(&mut harness, 'j');
+    assert_eq!(cursor(&harness, tab, id), (2, Some(1)));
+    let painted =
+        |harness: &Harness, said: &str| harness.painted.iter().any(|(text, _)| text == said);
+    assert!(painted(&harness, "-- VISUAL LINE --"));
+    assert!(painted(&harness, "2 lines selected · 3/12"));
+    letter(&mut harness, 'y');
+    assert_eq!(harness.copied.as_deref(), Some(lines(1, 3).as_str()));
+    assert_eq!(said(&harness).as_deref(), Some("copied 2 lines"));
+    assert_eq!(cursor(&harness, tab, id), (2, None));
+    letter(&mut harness, 'y');
+    assert_eq!(
+        harness.copied.as_deref(),
+        Some(book_covers(Driver::Postgres).text().as_str())
+    );
+    assert_eq!(said(&harness).as_deref(), Some("copied ddl"));
+    letter(&mut harness, 'V');
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert_eq!(cursor(&harness, tab, id).1, None);
+    assert!(!painted(&harness, "-- VISUAL LINE --"));
+}
+
+/// `e` opens the text in a new SQL editor, as the button does.
+#[test]
+fn e_opens_the_text_in_a_sql_editor() {
+    let (mut harness, tab, _) = terminal_lines();
+    letter(&mut harness, 'e');
+    let text = book_covers(Driver::Postgres).text();
+    assert_eq!(sql_tabs(&harness, tab), [(text, RunMode::ReadOnly)]);
+}
+
+/// The line numbers painted down the left of `field`, top to bottom.
+fn gutter(harness: &Harness, field: egui::Rect) -> Vec<usize> {
+    let mut numbers: Vec<(usize, f32)> = harness
+        .text_rects
+        .iter()
+        .filter(|(_, rect)| rect.left() < field.left() + 80.0 && rect.center().y > field.top())
+        .filter_map(|(text, rect)| Some((text.parse().ok()?, rect.center().y)))
+        .collect();
+    numbers.sort_by(|one, other| one.1.total_cmp(&other.1));
+    numbers.into_iter().map(|(number, _)| number).collect()
+}
+
+/// The macOS links name the parts that hold anything, with their counts.
+/// One puts its part's first line at the top of a text too long for the
+/// window, and is marked then; one whose part is too near the end to come
+/// to the top is marked all the same, until the text is scrolled.
+#[test]
+fn a_link_takes_its_part_to_the_top() {
+    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
+    set_view(&mut harness, tab, id, ObjectView::Ddl);
+    let mut long = book_covers(Driver::Postgres);
+    long.parts[1].statements = (0..80)
+        .map(|n| format!("CREATE INDEX book_covers_{n} ON public.book_covers USING btree (id)"))
+        .collect();
+    long.parts[2].statements = (0..80)
+        .map(|n| format!("COMMENT ON COLUMN public.book_covers.c{n} IS 'c'"))
+        .collect();
+    let placed = long.laid_out().1;
+    harness.answer_ddl(Ok(long));
+    let selected = |harness: &mut Harness, name: &str| {
+        let tree = harness.settle();
+        let link = node(&tree, name, Role::Link).unwrap_or_else(|| panic!("{name}"));
+        let (_, link) = tree.nodes.iter().find(|(id, _)| *id == link).unwrap();
+        link.is_selected() == Some(true)
+    };
+    let tree = harness.settle();
+    let field = crate::testing::bounds(&tree, "DDL of book_covers", Role::MultilineTextInput)
+        .expect("the text");
+    assert!(selected(&mut harness, "Table"));
+    assert!(!selected(&mut harness, "Comments 80"));
+    assert!(
+        !harness.has("Triggers"),
+        "a part with nothing is not linked"
+    );
+    harness.click("Indexes 80");
+    harness.settle();
+    assert_eq!(
+        gutter(&harness, field).first(),
+        Some(&(placed[1].1.start + 1))
+    );
+    assert!(selected(&mut harness, "Indexes 80"));
+    assert!(!selected(&mut harness, "Table"));
+    // The comments end the text: their first line stays under the top.
+    harness.click("Comments 80");
+    harness.settle();
+    assert!(gutter(&harness, field).contains(&(placed[2].1.start + 1)));
+    assert!(selected(&mut harness, "Comments 80"));
+    assert!(!selected(&mut harness, "Indexes 80"));
+    harness.click("Table");
+    harness.settle();
+    assert_eq!(gutter(&harness, field).first(), Some(&1));
+    assert!(selected(&mut harness, "Table"));
+}
+
+/// The links per engine: MySQL's table and its triggers, SQLite's table
+/// and its indexes.
+#[test]
+fn each_engine_links_the_parts_it_has() {
+    // MySQL's text here is given a trigger.
+    let trigger = "CREATE TRIGGER book_covers_kind BEFORE INSERT ON `book_covers` \
+                   FOR EACH ROW SET NEW.kind = LOWER(NEW.kind)";
+    for (driver, triggers, links, gone) in [
+        (
+            Driver::MySql,
+            &[trigger][..],
+            &["Table", "Triggers 1"][..],
+            "Indexes 1",
+        ),
+        (
+            Driver::Sqlite,
+            &[][..],
+            &["Table", "Indexes 1"][..],
+            "Comments 1",
+        ),
+    ] {
+        let (mut harness, tab, id) = covers(Look::macos(), driver);
+        set_view(&mut harness, tab, id, ObjectView::Ddl);
+        let mut ddl = book_covers(driver);
+        if !triggers.is_empty() {
+            ddl.parts.push(tabletist_db::ddl::Part {
+                kind: tabletist_db::ddl::PartKind::Triggers,
+                statements: triggers.iter().map(|text| text.to_string()).collect(),
+            });
+        }
+        harness.answer_ddl(Ok(ddl));
+        harness.settle();
+        for link in links {
+            assert!(harness.has(link), "{driver:?}: {link}");
+        }
+        assert!(!harness.has(gone), "{driver:?}: {gone}");
+    }
+}
+
+/// The terminal's outline lists the parts with their counts, the cursor's
+/// marked, and takes the cursor to one clicked; PostgreSQL's says what is
+/// not in the text.
+#[test]
+fn the_outline_lists_the_parts_and_moves_to_one() {
+    let (mut harness, tab, id) = terminal_lines();
+    for said in ["table 6", "indexes 1", "comments 1"] {
+        assert!(harness.has(said), "{said}");
+    }
+    assert!(harness.has("not included"));
+    let tree = harness.settle();
+    let table = node(&tree, "table 6", Role::Button).unwrap();
+    let (_, table) = tree.nodes.iter().find(|(id, _)| *id == table).unwrap();
+    assert_eq!(table.is_selected(), Some(true));
+    harness.click("comments 1");
+    assert_eq!(cursor(&harness, tab, id).0, 10);
+    // SQLite's text leaves nothing out.
+    let (mut harness, tab, _) = ddl_view(Look::omarchy(), Driver::Sqlite);
+    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
+    assert!(!harness.has("not included"));
+}
+
+/// A text read again that is shorter keeps the cursor on a line of it,
+/// and drops what was selected.
+#[test]
+fn a_shorter_text_keeps_the_cursor_on_it() {
+    let (mut harness, tab, id) = terminal_lines();
+    letter(&mut harness, 'G');
+    letter(&mut harness, 'V');
+    harness.app.apply(Action::Refresh(tab));
+    harness.answer_ddl(Ok(book_covers(Driver::MySql)));
+    let lines = book_covers(Driver::MySql).text().lines().count();
+    assert_eq!(cursor(&harness, tab, id), (lines - 1, None));
+}
+
+/// A Reload that failed shows its error, and the text it read before stays
+/// held under it: nothing copies or opens that text, and no line of it is
+/// selected. Nor while the first read is on its way, or after it failed.
+#[test]
+fn nothing_acts_on_a_text_the_view_does_not_show() {
+    let refused = || tabletist_db::Error::query("permission denied for table book_covers");
+    // The terminal look: `y`, `e`, `V`.
+    let (mut harness, tab, id) = terminal_lines();
+    harness.app.apply(Action::Refresh(tab));
+    harness.answer_ddl(Err(refused()));
+    harness.settle();
+    harness.copied = None;
+    for key in ['y', 'e', 'V'] {
+        letter(&mut harness, key);
+    }
+    assert_eq!(harness.copied, None);
+    assert!(sql_tabs(&harness, tab).is_empty());
+    assert_eq!(cursor(&harness, tab, id).1, None);
+    assert!(!harness.has("-- VISUAL LINE --"));
+    assert!(!harness.has("copy"), "no key to copy is offered");
+    // The looks with a pointer: the chords.
+    let (mut harness, tab, _) = ddl_view(Look::macos(), Driver::Postgres);
+    harness.app.apply(Action::Refresh(tab));
+    harness.answer_ddl(Err(refused()));
+    harness.settle();
+    assert!(!harness.has("Copy"));
+    harness.copied = None;
+    harness.copy(true);
+    harness.press(Key::Enter, Modifiers::ALT);
+    assert_eq!(harness.copied, None);
+    assert!(sql_tabs(&harness, tab).is_empty());
+    // A first read on its way, then failed: `V` starts nothing.
+    let (mut harness, tab, id) = covers(Look::omarchy(), Driver::Postgres);
+    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
+    set_view(&mut harness, tab, id, ObjectView::Ddl);
+    letter(&mut harness, 'V');
+    assert_eq!(cursor(&harness, tab, id).1, None);
+    harness.answer_ddl(Err(refused()));
+    letter(&mut harness, 'V');
+    assert_eq!(cursor(&harness, tab, id).1, None);
+    assert!(!harness.has("-- VISUAL LINE --"));
+}
+
+/// The terminal: Esc ends a selection of lines, and with none it is what it
+/// is in the other views: it closes Review SQL.
+#[test]
+fn esc_with_no_lines_selected_closes_review_sql() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    let (tab, id) = harness.editable();
+    pending_change(&mut harness, tab, id);
+    run_line(&mut harness, "diff");
+    letter(&mut harness, 'S');
+    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
+    harness.settle();
+    let reviewing = |harness: &Harness| {
+        let workspace = harness.app.workspace(tab).unwrap();
+        workspace.object_tab(id).unwrap().edits.reviewing
+    };
+    assert!(reviewing(&harness));
+    letter(&mut harness, 'V');
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert_eq!(cursor(&harness, tab, id).1, None);
+    assert!(reviewing(&harness), "the first Esc ends the selection");
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert!(!reviewing(&harness));
+}
+
+/// The terminal: with the row panel open on the rows, the DDL view's
+/// brackets and Esc are its own, and the rows' cursor stays.
+#[test]
+fn with_the_row_panel_open_the_brackets_are_the_views() {
+    let (mut harness, tab, id) = covers(Look::omarchy(), Driver::Postgres);
+    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
+    harness.app.apply(Action::SelectCell {
+        tab,
+        id,
+        cell: crate::model::CellPos { row: 1, col: 1 },
+    });
+    if !harness.app.workspace(tab).unwrap().row_panel {
+        harness.app.apply(Action::ToggleRowPanel(tab));
+    }
+    harness.settle();
+    letter(&mut harness, 'S');
+    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
+    harness.settle();
+    let selection = |harness: &Harness| {
+        let workspace = harness.app.workspace(tab).unwrap();
+        workspace.object_tab(id).unwrap().selection
+    };
+    let before = selection(&harness);
+    letter(&mut harness, ']');
+    letter(&mut harness, ']');
+    assert_eq!(cursor(&harness, tab, id).0, 7, "]] goes to the indexes");
+    letter(&mut harness, '[');
+    letter(&mut harness, '[');
+    assert_eq!(cursor(&harness, tab, id).0, 0);
+    assert_eq!(selection(&harness), before);
+}
+
+/// The terminal's line in the DDL view still says what is pending in the
+/// table's rows, and offers to write it and to review it; it strikes out
+/// none of the rows' keys.
+#[test]
+fn the_terminal_line_keeps_what_is_pending() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    let (tab, id) = harness.editable();
+    pending_change(&mut harness, tab, id);
+    assert!(harness.has("1 pending · 1 row"));
+    letter(&mut harness, 'S');
+    harness.answer_ddl(Ok(book_covers(Driver::Sqlite)));
+    harness.settle();
+    assert_eq!(view(&harness, tab, id), ObjectView::Ddl);
+    let (save, review) = (
+        crate::ui::keys::written(
+            &harness.ctx,
+            &Look::omarchy(),
+            crate::keymap::Command::SaveChanges,
+        ),
+        crate::ui::keys::written(
+            &harness.ctx,
+            &Look::omarchy(),
+            crate::keymap::Command::ReviewSql,
+        ),
+    );
+    let lines = book_covers(Driver::Sqlite).text().lines().count();
+    for said in [
+        "1 pending · 1 row".to_string(),
+        format!("{} write", &*save),
+        format!("{} review", &*review),
+        format!("1/{lines}"),
+    ] {
+        let painted = harness.painted.iter().any(|(text, _)| *text == said);
+        assert!(painted, "{said}: {:?}", harness.painted);
+    }
+    assert!(
+        !harness
+            .painted
+            .iter()
+            .any(|(text, _)| text.ends_with(" new row"))
+    );
+}
+
+/// The rows' `v y` copies a cell only where the rows show: not the one the
+/// DDL view hides.
+#[test]
+fn the_rows_v_y_copies_no_cell_in_the_ddl_view() {
+    let (mut harness, tab, id) = covers(Look::omarchy(), Driver::Postgres);
+    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
+    harness.app.apply(Action::SelectCell {
+        tab,
+        id,
+        cell: crate::model::CellPos { row: 1, col: 1 },
+    });
+    letter(&mut harness, 'S');
+    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
+    harness.settle();
+    harness.copied = None;
+    letter(&mut harness, 'v');
+    letter(&mut harness, 'y');
+    let text = book_covers(Driver::Postgres).text();
+    assert!(
+        harness
+            .copied
+            .as_deref()
+            .is_none_or(|copied| copied == text),
+        "{:?}",
+        harness.copied
+    );
+}
+
+/// The terminal: a click on a line puts the cursor on it, and the keys
+/// move on from there.
+#[test]
+fn a_click_on_a_line_puts_the_cursor_there() {
+    let (mut harness, tab, id) = terminal_lines();
+    let tree = harness.settle();
+    let lines = crate::testing::bounds(&tree, "DDL of book_covers", Role::MultilineTextInput)
+        .expect("the lines");
+    let at = lines.left_top() + egui::vec2(120.0, 10.0 + 28.0 * 3.5);
+    let press = |pressed| egui::Event::PointerButton {
+        pos: at,
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![egui::Event::PointerMoved(at)]);
+    harness.frame(vec![press(true)]);
+    harness.frame(vec![press(false)]);
+    harness.settle();
+    let line = cursor(&harness, tab, id).0;
+    assert!(line > 0, "the click put the cursor on no line");
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    letter(&mut harness, 'j');
+    assert_eq!(cursor(&harness, tab, id).0, line + 2);
+}
+
+/// Leaving the DDL view ends its selection of lines.
+#[test]
+fn leaving_the_view_ends_the_selection_of_lines() {
+    let (mut harness, tab, id) = terminal_lines();
+    letter(&mut harness, 'V');
+    letter(&mut harness, 'j');
+    assert_eq!(cursor(&harness, tab, id), (1, Some(0)));
+    letter(&mut harness, 'r');
+    assert_eq!(view(&harness, tab, id), ObjectView::Data);
+    letter(&mut harness, 'S');
+    assert_eq!(cursor(&harness, tab, id), (1, None));
+    assert!(!harness.has("-- VISUAL LINE --"));
+}
+
+/// A cell of the table changed and not written, with the keyboard on the
+/// rows.
+fn pending_change(harness: &mut Harness, tab: ConnTabId, id: TabId) {
+    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
+    harness.app.apply(Action::SelectCell {
+        tab,
+        id,
+        cell: crate::model::CellPos { row: 1, col: 1 },
+    });
+    harness.settle();
+    for key in ['c', 'c'] {
+        letter(harness, key);
+    }
+    harness.frame(vec![egui::Event::Text("zzz".into())]);
+    harness.press(Key::Escape, Modifiers::NONE);
+    harness.settle();
+}
diff --git a/src/ui/ddl_tests.rs b/src/ui/ddl_tests.rs
index 44bfe37b..1391d1d4 100644
--- a/src/ui/ddl_tests.rs
+++ b/src/ui/ddl_tests.rs
@@ -18,7 +18,7 @@ use crate::theme::Look;
 /// The Bookshop's `book_covers` as `driver` writes it: a table, two
 /// indexes, two triggers (PostgreSQL's built from its catalog, with
 /// comments).
-fn book_covers(driver: Driver) -> Ddl {
+pub(super) fn book_covers(driver: Driver) -> Ddl {
     let part = |kind, statements: &[&str]| Part {
         kind,
         statements: statements.iter().map(|s| s.to_string()).collect(),
@@ -80,7 +80,7 @@ fn book_covers(driver: Driver) -> Ddl {
 }
 
 /// The Bookshop's covers on `driver`, in `look`, their rows read.
-fn covers(look: Look, driver: Driver) -> (Harness, ConnTabId, TabId) {
+pub(super) fn covers(look: Look, driver: Driver) -> (Harness, ConnTabId, TabId) {
     let mut harness = Harness::new();
     harness.set_look(look);
     let tab = harness.connect_fake();
@@ -98,7 +98,7 @@ fn covers(look: Look, driver: Driver) -> (Harness, ConnTabId, TabId) {
     (harness, tab, id)
 }
 
-fn set_view(harness: &mut Harness, tab: ConnTabId, id: TabId, view: ObjectView) {
+pub(super) fn set_view(harness: &mut Harness, tab: ConnTabId, id: TabId, view: ObjectView) {
     harness.app.apply(Action::SetView {
         tab,
         object_tab: id,
@@ -108,7 +108,7 @@ fn set_view(harness: &mut Harness, tab: ConnTabId, id: TabId, view: ObjectView)
 }
 
 /// The `ReadDdl`s sent since `from`, with what each asked for.
-fn reads(harness: &Harness, from: usize) -> Vec<ObjectRef> {
+pub(super) fn reads(harness: &Harness, from: usize) -> Vec<ObjectRef> {
     harness.app.backend.sent[from..]
         .iter()
         .filter_map(|command| match command {
@@ -119,7 +119,7 @@ fn reads(harness: &Harness, from: usize) -> Vec<ObjectRef> {
 }
 
 /// The covers' DDL view on `driver` in `look`, its definition read.
-fn ddl_view(look: Look, driver: Driver) -> (Harness, ConnTabId, TabId) {
+pub(super) fn ddl_view(look: Look, driver: Driver) -> (Harness, ConnTabId, TabId) {
     let (mut harness, tab, id) = covers(look, driver);
     set_view(&mut harness, tab, id, ObjectView::Ddl);
     harness.answer_ddl(Ok(book_covers(driver)));
@@ -488,7 +488,7 @@ fn the_age_is_said_in_its_largest_unit() {
 }
 
 /// The keys of `c` typed, as a window sends them: its key, then its text.
-fn letter(harness: &mut Harness, c: char) {
+pub(super) fn letter(harness: &mut Harness, c: char) {
     let key = Key::from_name(&c.to_ascii_uppercase().to_string()).expect("a letter's key");
     let shift = if c.is_uppercase() {
         Modifiers::SHIFT
@@ -504,7 +504,7 @@ fn letter(harness: &mut Harness, c: char) {
 }
 
 /// Gives the text the keyboard, as F6 does.
-fn focus_text(harness: &mut Harness, tab: ConnTabId, id: TabId) {
+pub(super) fn focus_text(harness: &mut Harness, tab: ConnTabId, id: TabId) {
     let field = super::ddl::text_id(tab, id);
     harness.ctx.memory_mut(|memory| memory.request_focus(field));
     harness.settle();
@@ -636,23 +636,28 @@ fn the_ddl_view_has_no_add_row() {
     assert!(harness.has("Select text to copy part of it"));
 }
 
-/// The terminal's line names the view's keys, not the rows', and how many
-/// lines the text has.
+/// The terminal's line names the view's keys, not the rows', and where
+/// its cursor is of how many lines.
 #[test]
 fn the_terminal_line_names_the_views_keys_and_its_lines() {
     let (harness, _, _) = ddl_view(Look::omarchy(), Driver::Postgres);
     let lines = book_covers(Driver::Postgres).text().lines().count();
     let painted = |said: &str| harness.painted.iter().any(|(text, _)| text == said);
-    assert!(painted(&format!("{lines} lines")));
+    assert!(painted(&format!("1/{lines}")));
     // Each key with its word, as the line paints a hint.
     let key = |command| crate::ui::keys::written(&harness.ctx, &Look::omarchy(), command);
-    let (reload, tables) = (
-        key(crate::keymap::Command::Reload),
-        key(crate::keymap::Command::ToggleSidebar),
-    );
-    assert!(painted(&format!("{} reload", &*reload)));
-    assert!(painted(&format!("{} tables", &*tables)));
-    let rows = key(crate::keymap::Command::MoveRow);
+    use crate::keymap::Command;
+    for (command, word) in [
+        (Command::MoveInDdl, "line"),
+        (Command::SelectDdlLines, "select lines"),
+        (Command::CopyDdl, "copy"),
+        (Command::OpenDdlInEditor, "open in sql"),
+        (Command::ShowRows, "rows"),
+        (Command::ShowStructure, "structure"),
+    ] {
+        assert!(painted(&format!("{} {word}", &*key(command))), "{word}");
+    }
+    let rows = key(Command::MoveRow);
     assert!(
         !painted(&format!("{} row", &*rows)),
         "the rows' keys are not named"
@@ -731,10 +736,10 @@ fn reload_reads_again_from_the_text() {
 }
 
 /// The keyboard selects in the text as in any: Shift with an arrow from
-/// where it came in, at the top.
+/// where it came in, at the top. (The terminal look selects by line.)
 #[test]
 fn shift_with_an_arrow_selects_from_the_top() {
-    for look in Look::ALL {
+    for look in [Look::macos(), Look::standard()] {
         let (mut harness, tab, id) = ddl_view(look, Driver::Postgres);
         focus_text(&mut harness, tab, id);
         harness.press(Key::ArrowDown, Modifiers::SHIFT);
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index dcae9aba..e91af11c 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -3243,14 +3243,8 @@ mod tests {
         };
         let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
         let ctrl_elsewhere = Modifiers::CTRL | Modifiers::COMMAND;
-        // The command key: Cmd on macOS, with Ctrl or without, and Ctrl
-        // elsewhere.
-        for held in [
-            Modifiers::COMMAND,
-            cmd,
-            cmd | Modifiers::CTRL,
-            ctrl_elsewhere,
-        ] {
+        // The command key: Cmd on macOS, and Ctrl elsewhere.
+        for held in [Modifiers::COMMAND, cmd, ctrl_elsewhere] {
             assert!(is(held, named(true, false)), "{held:?}");
         }
         for held in [
@@ -3261,6 +3255,18 @@ mod tests {
         ] {
             assert!(is_not(held, named(true, false)), "{held:?}");
         }
+        // On a Mac, Cmd with Control is ⌃⌘, a chord of its own (the views'
+        // ⌃⌘1), and not Cmd's.
+        assert!(!held_is(
+            Layout::Mac,
+            cmd | Modifiers::CTRL,
+            named(true, false)
+        ));
+        assert!(held_is(
+            Layout::Mac,
+            cmd | Modifiers::CTRL,
+            named(true, true)
+        ));
         // Ctrl: alone on macOS, with the command key elsewhere, never Cmd.
         for held in [Modifiers::CTRL, ctrl_elsewhere] {
             assert!(is(held, named(false, true)), "{held:?}");
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index cccdce9c..bb570eca 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -25,6 +25,8 @@ pub mod connect_dialog;
 pub mod data_view;
 pub mod ddl;
 #[cfg(test)]
+mod ddl_act_tests;
+#[cfg(test)]
 mod ddl_tests;
 #[cfg(test)]
 mod delete_many_tests;
@@ -1267,7 +1269,7 @@ value_tags   = true
         ];
         let omarchy = [
             ("i, enter", "Edit cell"),
-            ("cc, s", "Edit cell from nothing"),
+            ("cc", "Edit cell from nothing"),
             ("x", "Set NULL"),
             ("u", "Undo one cell"),
             ("ctrl+c", "Cancel cell edit"),
@@ -2847,19 +2849,26 @@ value_tags   = true
             selection(&harness, tab),
             Some(crate::model::CellPos { row: 1, col: 1 })
         );
-        // `s` and `d` showed the Structure and the Data view. They are
-        // letters of the cell and the row now, and the views are clicked.
-        for key in [Key::S, Key::D] {
-            harness.press(key, Modifiers::NONE);
-            let view = harness
-                .app
-                .workspace(tab)
-                .unwrap()
-                .active_object_tab()
-                .unwrap()
-                .view;
-            assert_eq!(view, crate::model::ObjectView::Data, "{key:?}");
-        }
+        // `s`, `S` and `r` show the Structure, the DDL and the rows
+        // again, from any view; `d` begins the row's `dd`.
+        let view = |harness: &Harness| {
+            let workspace = harness.app.workspace(tab).unwrap();
+            workspace.active_object_tab().unwrap().view
+        };
+        type_key(&mut harness, Key::D, "d");
+        assert_eq!(view(&harness), crate::model::ObjectView::Data);
+        harness.press(Key::Escape, Modifiers::NONE);
+        type_key(&mut harness, Key::S, "s");
+        assert_eq!(view(&harness), crate::model::ObjectView::Structure);
+        harness.frame(vec![
+            crate::testing::key(Key::S, Modifiers::SHIFT),
+            egui::Event::Text("S".into()),
+        ]);
+        harness.frame(vec![crate::testing::release(Key::S, Modifiers::SHIFT)]);
+        harness.settle();
+        assert_eq!(view(&harness), crate::model::ObjectView::Ddl);
+        type_key(&mut harness, Key::R, "r");
+        assert_eq!(view(&harness), crate::model::ObjectView::Data);
     }
 
     #[test]
@@ -4746,7 +4755,7 @@ value_tags   = true
     }
 
     /// Types `text` into whatever has the keyboard.
-    fn type_text(harness: &mut Harness, text: &str) {
+    pub(super) fn type_text(harness: &mut Harness, text: &str) {
         harness.frame(vec![egui::Event::Text(text.into())]);
         harness.settle();
     }
@@ -15484,15 +15493,18 @@ value_tags   = true
     }
 
     #[test]
-    fn s_replaces_the_cell_as_cc_does_and_shows_no_structure() {
+    fn cc_replaces_the_cell_and_s_shows_the_structure() {
         let (mut harness, tab, id) = normal_mode((1, 1));
+        type_key(&mut harness, Key::C, "c");
+        type_key(&mut harness, Key::C, "c");
+        assert_eq!(editor_text(&harness, tab, id).as_deref(), Some(""));
+        harness.press(Key::Escape, Modifiers::NONE);
         type_key(&mut harness, Key::S, "s");
         let workspace = harness.app.workspace(tab).unwrap();
         assert_eq!(
             workspace.object_tab(id).unwrap().view,
-            crate::model::ObjectView::Data
+            crate::model::ObjectView::Structure
         );
-        assert_eq!(editor_text(&harness, tab, id).as_deref(), Some(""));
     }
 
     #[test]
@@ -16422,14 +16434,14 @@ value_tags   = true
             changes.rows[0].set[0].new,
             tabletist_db::NewValue::Text("bob@example.comx".into())
         );
-        // `s` alone is the cell's letter: it saves nothing, and shows no
-        // other view.
+        // `s` alone saves nothing: it shows the Structure view.
         harness.answer_written(Ok(written_row("bob@example.comx")));
         type_key(&mut harness, Key::S, "s");
+        assert_eq!(writes(&harness), 2);
         let workspace = harness.app.workspace(tab).unwrap();
         assert_eq!(
             workspace.object_tab(id).unwrap().view,
-            crate::model::ObjectView::Data
+            crate::model::ObjectView::Structure
         );
         assert_eq!(writes(&harness), 2);
     }
@@ -16574,12 +16586,12 @@ value_tags   = true
         let (mut harness, tab, id) = normal_mode((1, 1));
         let palette = harness.app.palette;
         // Enter is the inspector's key, and `i` edits a cell that can be
-        // edited. The cell is copied with `v y`, and the views have no
-        // letters any more.
+        // edited. The cell is copied with `v y`. The views' letters are
+        // the header's to name, beside their words.
         assert!(painted(&harness, "enter inspect"), "{:?}", harness.painted);
         assert!(painted(&harness, "i edit"));
         assert!(painted(&harness, "v y copy"));
-        for gone in ["space inspect", "i inspector", "y copy", "s structure"] {
+        for gone in ["space inspect", "i inspector", "y copy"] {
             assert!(!painted(&harness, gone), "{gone}");
         }
         // Not on the row's key, which cannot.
@@ -16776,10 +16788,7 @@ value_tags   = true
         // Until the selection moves.
         type_key(&mut harness, Key::J, "j");
         assert!(!painted(&harness, why));
-        // `s` and `cc` ask as `i` does.
-        type_key(&mut harness, Key::S, "s");
-        assert!(painted(&harness, why));
-        type_key(&mut harness, Key::K, "k");
+        // `cc` asks as `i` does.
         type_key(&mut harness, Key::C, "c");
         type_key(&mut harness, Key::C, "c");
         assert!(painted(&harness, why));
````

- [ ] **Step 2: Run them to see them fail**

```bash
~/.cargo/bin/cargo test --locked -p tabletist-db --lib ddl::
```

Expected: it does not compile, `Ddl` has no `laid_out`:

```
error[E0599]: no method named `laid_out` found for struct `ddl::Ddl` in the current scope
error[E0599]: no method named `laid_out` found for struct `ddl::Ddl` in the current scope
```

```bash
~/.cargo/bin/cargo test --locked --lib ui::ddl_act_tests
```

Expected: it does not compile, there are no new commands, actions or tab fields:

```
error[E0599]: no method named `every_bound` found for reference `&keymap::Binding` in the current scope
error[E0599]: no method named `every_bound` found for reference `&keymap::Binding` in the current scope
error[E0609]: no field `windows` on type `&keymap::Binding`
error[E0599]: no variant, associated function, or constant named `ShowRows` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `ShowStructure` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `ShowDdl` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `CopyDdl` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `CopyDdl` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `CopyDdl` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `OpenDdlInEditor` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `OpenDdlInEditor` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `OpenDdl` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `NextDdlSection` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `PreviousDdlSection` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `ShowStructure` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `ShowRows` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `ShowDdl` found for enum `keymap::Command` in the current scope
error[E0609]: no field `ddl_cursor` on type `&model::ObjectTab`
error[E0609]: no field `ddl_visual` on type `&model::ObjectTab`
error[E0599]: no variant, associated function, or constant named `OpenDdl` found for enum `model::Action` in the current scope
error[E0599]: no method named `laid_out` found for struct `Ddl` in the current scope
error[E0599]: no variant, associated function, or constant named `MoveInDdl` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `SelectDdlLines` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `CopyDdl` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `OpenDdlInEditor` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `ShowRows` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `ShowStructure` found for enum `keymap::Command` in the current scope
```

- [ ] **Step 3: Write the keys, Copy, Open in SQL editor, the links and the outline**

````diff
diff --git a/AGENTS.md b/AGENTS.md
index b26c3bf6..48c1e1ef 100644
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -69,7 +69,8 @@ tree, a grid) marks where its arrows are by itself: its cursor, its cell.
 A key is spelled in `src/keymap.rs` (and `src/keymap/overrides.rs`, which
 lays the person's keys from `settings.toml` over it) and nowhere else: each
 command with the scopes it holds in and its keys for macOS and for Omarchy
-(Windows has the Mac's, with Ctrl for Cmd). `src/ui/keys.rs` and the
+(Windows has the Mac's, with Ctrl for Cmd, unless a line gives it keys of
+its own: `Binding::windows`). `src/ui/keys.rs` and the
 dialogs ask the keymap whether a command's key went down, and name no key
 themselves. What paints a key beside a word (a hint, a button's shortcut)
 takes a `keymap::Written`, which only the keymap makes; `tests/keys.rs`
diff --git a/crates/tabletist-db/src/ddl.rs b/crates/tabletist-db/src/ddl.rs
index 424383db..3f2e176d 100644
--- a/crates/tabletist-db/src/ddl.rs
+++ b/crates/tabletist-db/src/ddl.rs
@@ -100,6 +100,17 @@ impl Ddl {
     /// over each part after the object's. SQLite's keeps its statements
     /// apart by a blank line, and adds nothing else.
     pub fn text(&self) -> String {
+        self.laid_out().0
+    }
+
+    /// [`Self::text`], and the lines each part takes in it, from nought:
+    /// from its comment (the object's from the first line, its heading's)
+    /// to its last statement's last line, the blank line after it left out.
+    /// What lists the parts and the text are made together, so they never
+    /// disagree.
+    pub fn laid_out(&self) -> (String, Vec<(PartKind, std::ops::Range<usize>)>) {
+        let line = |text: &str| text.matches('\n').count();
+        let mut placed = Vec::with_capacity(self.parts.len());
         let mut text = String::new();
         if let Some(heading) = &self.heading {
             text.push_str("-- ");
@@ -112,12 +123,20 @@ impl Ddl {
         };
         let mut written = false;
         for part in &self.parts {
+            let mut start = None;
             for (index, statement) in part.statements.iter().enumerate() {
                 let first = index == 0;
                 if written {
                     // A part's statements are one under the other.
                     text.push_str(if sections && !first { "\n" } else { "\n\n" });
                 }
+                // The object's part starts at the heading, at the top.
+                let from = if part.kind == PartKind::Object {
+                    0
+                } else {
+                    line(&text)
+                };
+                start.get_or_insert(from);
                 if sections && first && part.kind != PartKind::Object {
                     text.push_str(part.kind.heading());
                     text.push('\n');
@@ -125,8 +144,11 @@ impl Ddl {
                 text.push_str(&terminated(statement));
                 written = true;
             }
+            if let Some(start) = start {
+                placed.push((part.kind, start..line(&text) + 1));
+            }
         }
-        text
+        (text, placed)
     }
 }
 
diff --git a/docs/_guide/browsing-data.md b/docs/_guide/browsing-data.md
index 7269f471..2b974199 100644
--- a/docs/_guide/browsing-data.md
+++ b/docs/_guide/browsing-data.md
@@ -118,6 +118,9 @@ a read-only connection. The Omarchy look does the same with `gl` and the
 
 ## DDL
 
+Ctrl+Cmd+1, 2 and 3 show a table's rows, its structure and its DDL on
+macOS, and Alt+1, 2 and 3 on Windows.
+
 **DDL**, the third view of a table, a view or a materialized view, shows
 its definition as text you can select and copy but not change, in the SQL
 editor's colours, with line numbers. Long lines do not wrap: scroll
@@ -149,6 +152,16 @@ Reading it only reads the catalog, and adds nothing to History. Where the
 connection's user may not read it, the view shows the database's error
 and **Try again**, never part of a text; Rows and Structure still work.
 
+Over the text, **Table**, **Indexes**, **Triggers** and **Comments** go to
+that part, with how many statements each holds; a part with nothing is not
+listed, and the one at the top of the text is marked. A part near the end
+cannot come to the top: its link stays marked until you scroll. **Copy** (Cmd/Ctrl+Shift+C) copies the selected text, or all of it
+with nothing selected. **Open in SQL editor** (Alt+Enter, Option+Return on macOS) puts
+the text in a new SQL editor that runs nothing until you run it. It opens
+read-only or not as Settings › Editor says for a new tab, and always
+read-only on production or on a connection that cannot write. On
+production, the table's name has the production badge beside it.
+
 ## Copying
 
 On macOS and Windows, Cmd/Ctrl+C copies the selected cell and
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
index 0f00c589..1c67417f 100644
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -181,6 +181,7 @@ keyboard.
 | Cmd+. | Cancel the running query |
 | Cmd+I | Open the row panel, with the keyboard on its fields. Esc goes back to the rows |
 | Cmd+Shift+R | Show or hide the row panel |
+| Ctrl+Cmd+1, 2, 3 | Show the rows, the structure, the DDL |
 
 In its Structure view:
 
@@ -193,6 +194,13 @@ In its Structure view:
 The last two work with the keyboard on the view's columns: click a
 column's line, or come to them with Tab or F6.
 
+In its DDL view:
+
+| Keys | Do |
+| --- | --- |
+| Cmd+Shift+C | Copy the DDL: the selected text, or all of it |
+| Option+Return | Open the DDL in a new SQL editor |
+
 ### Editing
 
 | Keys | Do |
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index a10d09f5..35080954 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -154,9 +154,10 @@ With the search open, the matches replace the start lists:
 | `v` then `y` | Copy the cell |
 | `R` | Reload |
 | Ctrl+C | Cancel the running query |
+| `r`, `s`, `S` | Show the rows, the structure or the DDL, from any of them |
+| `:ddl` | The DDL view of the table under the sidebar's cursor (with the keyboard in the sidebar), or else of the table in front |
 
-The `rows`, `structure` and `ddl` views are clicked: switching between
-them has no key yet. Nor has copying a whole row.
+Copying a whole row has no key.
 
 ### The Agents tab
 
@@ -200,11 +201,25 @@ says why.
 
 ### The DDL view
 
-`ddl` in a table's header shows its definition as text, as
+`S` (or `ddl` in a table's header) shows its definition as text, as
 [Browsing Data]({% link _guide/browsing-data.md %}#ddl) says: read-only,
 with line numbers. The header says where it comes from and from which
-server. `R` reads it again, with the keyboard in the text too, and after
-an error tries again.
+server, and the outline beside the text lists its parts with how much each
+holds, the part the cursor is in marked. Click a part to go to it.
+
+| Keys | Do |
+| --- | --- |
+| `j` `k`, Up, Down | Move by line |
+| `gg` `G` | The first line and the last |
+| `]]` `[[` | The next part and the one before |
+| `V` | Select whole lines from the cursor's, or end the selection |
+| `y` | Copy the selected lines, or all of the text with none selected |
+| `e` | Open the text in a new SQL editor, which runs nothing |
+| Esc | End the selection (with none, Esc does what it does elsewhere: it closes Review SQL) |
+| `R` | Read it again; after an error, try again |
+
+The status line says how many lines are selected and which line the
+cursor is on.
 
 ### The row panel
 
@@ -225,7 +240,7 @@ an error tries again.
 | Keys | Do |
 | --- | --- |
 | `i` | Edit the cell |
-| `cc`, `s` | Edit the cell from nothing |
+| `cc` | Edit the cell from nothing |
 | `o`, `O` | Add a row below the cursor's row, or above it |
 | `yy` | Yank the cursor's row, and put it on the clipboard |
 | `p`, `P` | Put a copy of the yanked row below the cursor's row, or above it |
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
index 65bbef90..82743558 100644
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -119,6 +119,7 @@ takes what you type (Mod+P opens the same search from anywhere):
 | Mod+Shift+R | Show or hide the row panel |
 | Mod+C | Copy the cell |
 | Mod+Shift+C | Copy the row |
+| Ctrl+Cmd+1, 2, 3 on macOS, Alt+1, 2, 3 on Windows | Show the rows, the structure, the DDL |
 
 In a table's [Structure view]({% link _guide/browsing-data.md %}#structure):
 
@@ -131,6 +132,13 @@ In a table's [Structure view]({% link _guide/browsing-data.md %}#structure):
 The last two work with the keyboard on the view's columns: click a
 column's line, or come to them with Tab or F6.
 
+In a table's [DDL view]({% link _guide/browsing-data.md %}#ddl):
+
+| Keys | Do |
+| --- | --- |
+| Mod+Shift+C | Copy the DDL: the selected text, or all of it |
+| Alt+Enter | Open the DDL in a new SQL editor |
+
 ## Editing
 
 These are the keys of the macOS and Windows looks. The Omarchy look has
diff --git a/src/app.rs b/src/app.rs
index 2934fd58..039e1f7c 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1309,6 +1309,7 @@ impl App {
                     leaves.then(|| {
                         object.ddl.started = None;
                         object.ddl.asked = false;
+                        object.ddl_visual = None;
                         object.ddl.pending.take()
                     })?
                 });
@@ -2648,6 +2649,51 @@ impl App {
                 }
                 self.read_ddl(tab, object_tab);
             }
+            Action::MoveDdlCursor { tab, id, by } => {
+                if let Some(object) = self.object_tab_mut(tab, id) {
+                    let last = object.ddl_lines().saturating_sub(1);
+                    object.ddl_cursor = object.ddl_cursor.saturating_add_signed(by).min(last);
+                }
+            }
+            Action::DdlCursorTo { tab, id, line } => {
+                if let Some(object) = self.object_tab_mut(tab, id) {
+                    object.ddl_cursor = line.min(object.ddl_lines().saturating_sub(1));
+                }
+            }
+            Action::DdlSection { tab, id, forward } => {
+                if let Some(object) = self.object_tab_mut(tab, id) {
+                    let starts: Vec<usize> = object.ddl_shown().map_or(Vec::new(), |ddl| {
+                        ddl.laid_out()
+                            .1
+                            .iter()
+                            .map(|(_, lines)| lines.start)
+                            .collect()
+                    });
+                    let cursor = object.ddl_cursor;
+                    let to = match forward {
+                        true => starts.into_iter().find(|start| *start > cursor),
+                        false => starts.into_iter().rev().find(|start| *start < cursor),
+                    };
+                    if let Some(to) = to {
+                        object.ddl_cursor = to;
+                    }
+                }
+            }
+            Action::ToggleDdlVisual { tab, id } => {
+                if let Some(object) = self.object_tab_mut(tab, id) {
+                    object.ddl_visual = match object.ddl_visual {
+                        Some(_) => None,
+                        None => object.ddl_shown().map(|_| object.ddl_cursor),
+                    };
+                }
+            }
+            Action::LeaveDdlVisual { tab, id } => {
+                if let Some(object) = self.object_tab_mut(tab, id) {
+                    object.ddl_visual = None;
+                }
+            }
+            Action::OpenDdlInEditor { tab, id } => self.open_ddl_in_editor(tab, id),
+            Action::OpenDdl(tab) => self.open_ddl(tab),
             Action::SetDriver(driver) => {
                 if let Some(Dialog::Connection(form)) = &mut self.dialog {
                     // A port left at the old driver's default follows the driver.
@@ -4165,6 +4211,11 @@ impl App {
                     if read {
                         object.ddl_read = Some(std::time::Instant::now());
                     }
+                    // A text read again may be shorter: the cursor stays on
+                    // a line of it. What was selected goes with any answer,
+                    // a failed one too.
+                    object.ddl_cursor = object.ddl_cursor.min(object.ddl_lines().saturating_sub(1));
+                    object.ddl_visual = None;
                 }
             }
             Event::SecretLoaded { request, result } => {
@@ -7234,6 +7285,59 @@ impl App {
         });
     }
 
+    /// Opens the DDL view's text of the tab `id`, as it shows, in a new
+    /// SQL editor. Nothing is run: the editor's mode is a new tab's
+    /// (Settings › Editor), read-only on production and on a connection
+    /// that cannot write whatever that says.
+    fn open_ddl_in_editor(&mut self, tab: ConnTabId, id: TabId) {
+        let Some(text) = self
+            .workspace(tab)
+            .and_then(|workspace| workspace.object_tab(id))
+            .and_then(|object| object.ddl_shown())
+            .map(tabletist_db::Ddl::text)
+        else {
+            return;
+        };
+        let Some(sql_id) = self.new_sql_tab(tab) else {
+            return;
+        };
+        if let Some(sql) = self
+            .workspace_mut(tab)
+            .and_then(|workspace| workspace.sql_tab_mut(sql_id))
+        {
+            sql.text = text;
+        }
+    }
+
+    /// The terminal's `:ddl`: the DDL view of the table or view under the
+    /// tree's cursor where the tree has the keys, opened (and kept) if it
+    /// is not, else of the table's tab in front. With neither (a SQL
+    /// editor in front, no tab), nothing: the terminal's line says what a
+    /// table's prompt refused only over a table.
+    fn open_ddl(&mut self, tab: ConnTabId) {
+        let Some(workspace) = self.workspace(tab) else {
+            return;
+        };
+        let under = match (&workspace.pane, &workspace.tree.cursor) {
+            (Pane::Tree, Some(TreeNode::Object(object, kind))) => Some((object.clone(), *kind)),
+            _ => None,
+        };
+        if let Some((object, kind)) = under {
+            self.open_object(tab, object, kind, true);
+        }
+        let front = self
+            .workspace(tab)
+            .and_then(|workspace| workspace.active_object_tab())
+            .map(|object| object.id);
+        if let Some(object_tab) = front {
+            self.apply(Action::SetView {
+                tab,
+                object_tab,
+                view: ObjectView::Ddl,
+            });
+        }
+    }
+
     /// Reload's part for the DDL view of the tab `id`: the definition is
     /// read again where the view shows, and forgotten where it does not,
     /// for the view to read when it is shown next. One under an error
diff --git a/src/app/editing.rs b/src/app/editing.rs
index 3e9205dc..44ad469d 100644
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -1855,8 +1855,12 @@ impl App {
         let structure = workspace
             .active_object_tab()
             .is_some_and(|object| object.view == crate::model::ObjectView::Structure);
+        let ddl = workspace
+            .active_object_tab()
+            .is_some_and(|object| object.view == crate::model::ObjectView::Ddl);
         let holds = |when: crate::keymap::When| match when {
             crate::keymap::When::Structure => structure,
+            crate::keymap::When::Ddl => ddl,
             _ => false,
         };
         // The prompt is Omarchy's, in whatever look it is driven.
@@ -1874,6 +1878,9 @@ impl App {
             (Typed::Command(Command::KeysDefault), _) => Action::KeysDefault,
             (Typed::Command(Command::KeysCheatsheet), _) => Action::ExportCheatSheet,
             (Typed::Command(Command::ClearHistory), _) => Action::ClearHistory(tab),
+            // From any pane: of what the tree's cursor is on, or the tab in
+            // front.
+            (Typed::Command(Command::OpenDdl), _) => Action::OpenDdl(tab),
             // From any tab of the window, where agents can reach the app.
             (Typed::Command(Command::ShowAgents), _) if crate::endpoint::AVAILABLE => {
                 Action::ShowAgents(tab)
diff --git a/src/keymap.rs b/src/keymap.rs
index d7dd798b..ad3bea04 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -253,6 +253,9 @@ pub enum When {
     /// grid's keys that act on a row have nothing to act on, and the
     /// view's own act on the line of a column.
     Structure,
+    /// The DDL view of a table shows, in place of its rows: its own keys
+    /// act on its text, by line in the terminal look.
+    Ddl,
     /// The Agents tab shows its Log view: its keys come before the
     /// activity's.
     AgentsLog,
@@ -561,6 +564,19 @@ commands! {
     LabelAutomatic = "label_automatic",
     LabelKeysOnly = "label_keys_only",
     LabelColumn = "label_column",
+    ShowRows = "show_rows",
+    ShowStructure = "show_structure",
+    ShowDdl = "show_ddl",
+    OpenDdl = "open_ddl",
+    CopyDdl = "copy_ddl",
+    OpenDdlInEditor = "open_ddl_in_editor",
+    MoveInDdl = "move_in_ddl",
+    DdlTop = "ddl_top",
+    DdlBottom = "ddl_bottom",
+    NextDdlSection = "next_ddl_section",
+    PreviousDdlSection = "previous_ddl_section",
+    SelectDdlLines = "select_ddl_lines",
+    LeaveDdlVisual = "leave_ddl_visual",
     ReviewSql = "review_sql",
     NextChange = "next_change",
     CopyReviewSql = "copy_review_sql",
@@ -924,6 +940,19 @@ impl Command {
             C::LabelAutomatic => info("Label foreign keys automatically", "", Editing),
             C::LabelKeysOnly => info("Label foreign keys by their keys alone", "", Editing),
             C::LabelColumn => info("Label foreign keys by a column", "", Editing),
+            C::ShowRows => info("Show the rows", "", Navigation),
+            C::ShowStructure => info("Show the structure", "", Navigation),
+            C::ShowDdl => info("Show the DDL", "", Navigation),
+            C::OpenDdl => info("Open the DDL view", "", Navigation),
+            C::CopyDdl => info("Copy the DDL", "", Editing),
+            C::OpenDdlInEditor => info("Open the DDL in the SQL editor", "", Sql),
+            C::MoveInDdl => info("Move by line in the DDL", "", Navigation),
+            C::DdlTop => info("Top of the DDL", "", Navigation),
+            C::DdlBottom => info("Bottom of the DDL", "", Navigation),
+            C::NextDdlSection => info("Next section of the DDL", "", Navigation),
+            C::PreviousDdlSection => info("Previous section of the DDL", "", Navigation),
+            C::SelectDdlLines => info("Select lines of the DDL", "", Navigation),
+            C::LeaveDdlVisual => info("Leave the selection of lines", "", Navigation),
             C::ReviewSql => info("Review pending SQL", "Review pending SQL", Editing),
             C::NextChange => info("Next change", "Review pending SQL", Editing).reserved(),
             C::CopyReviewSql => info("Copy the pending SQL", "", Editing),
@@ -1072,15 +1101,26 @@ pub struct Binding {
     pub when: When,
     pub mac: Bound,
     pub omarchy: Bound,
+    /// Windows' own keys, where the Mac's with Ctrl for Cmd would not do.
+    pub windows: Option<Bound>,
 }
 
 impl Binding {
     pub fn bound(&self, layout: Layout) -> &Bound {
         match layout {
-            Layout::Mac | Layout::Windows => &self.mac,
+            Layout::Mac => &self.mac,
+            Layout::Windows => self.windows.as_ref().unwrap_or(&self.mac),
             Layout::Omarchy => &self.omarchy,
         }
     }
+
+    /// Every look's keys of the line: the Mac's, Omarchy's, and Windows'
+    /// where it has its own.
+    pub fn every_bound(&self) -> impl Iterator<Item = &Bound> {
+        [Some(&self.mac), Some(&self.omarchy), self.windows.as_ref()]
+            .into_iter()
+            .flatten()
+    }
 }
 
 impl Binding {
@@ -1127,6 +1167,7 @@ const fn bind(command: Command, scopes: &'static [Scope], mac: Bound, omarchy: B
         when: When::Always,
         mac,
         omarchy,
+        windows: None,
     }
 }
 
@@ -1134,6 +1175,14 @@ impl Binding {
     const fn when(self, when: When) -> Self {
         Self { when, ..self }
     }
+
+    /// Windows' own keys for the line.
+    const fn windows(self, windows: Bound) -> Self {
+        Self {
+            windows: Some(windows),
+            ..self
+        }
+    }
 }
 
 const GLOBAL: &[Scope] = &[Scope::Global];
@@ -1558,7 +1607,8 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k("enter"), k("f2")]),
             keys(&[k("enter"), k("i")]),
         ),
-        bind(C::ReplaceCell, CELLS, NONE, keys(&[k("cc"), k("s")])),
+        // Vim's `s` is the Structure view's: `cc` does the same.
+        bind(C::ReplaceCell, CELLS, NONE, keys(&[k("cc")])),
         bind(
             C::CommitDown,
             CELL_EDITOR,
@@ -1763,6 +1813,63 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k(":label {column}")]),
         )
         .when(When::Structure),
+        // A table's views, from any of them. Windows has one Ctrl for the
+        // Mac's Control and Command, and Ctrl with a digit goes to a
+        // connection's window there: its keys are Alt's.
+        bind(
+            C::ShowRows,
+            GRID,
+            keys(&[k("ctrl+cmd+1")]).any(),
+            keys(&[k("r")]),
+        )
+        .windows(keys(&[k("alt+1")]).any()),
+        bind(
+            C::ShowStructure,
+            GRID,
+            keys(&[k("ctrl+cmd+2")]).any(),
+            keys(&[k("s")]),
+        )
+        .windows(keys(&[k("alt+2")]).any()),
+        bind(
+            C::ShowDdl,
+            GRID,
+            keys(&[k("ctrl+cmd+3")]).any(),
+            keys(&[k("S")]),
+        )
+        .windows(keys(&[k("alt+3")]).any()),
+        // The DDL view of what is under the tree's cursor, or of the tab
+        // in front, from any pane.
+        bind(C::OpenDdl, GLOBAL, NONE, keys(&[k(":ddl")])),
+        // The DDL view's own. Its text is a field in the looks with a
+        // pointer, where the chords are read in it too; the terminal look
+        // moves in it by line, and selects whole lines.
+        bind(
+            C::CopyDdl,
+            GRID,
+            keys(&[k("cmd+shift+c")]).any(),
+            keys(&[k("y")]),
+        )
+        .when(When::Ddl),
+        bind(
+            C::OpenDdlInEditor,
+            GRID,
+            keys(&[k("alt+enter")]).any(),
+            keys(&[k("e")]),
+        )
+        .when(When::Ddl),
+        bind(
+            C::MoveInDdl,
+            GRID,
+            NONE,
+            keys(&[k("j"), k("k"), k("up"), k("down")]).shown("j/k"),
+        )
+        .when(When::Ddl),
+        bind(C::DdlTop, GRID, NONE, keys(&[k("gg")])).when(When::Ddl),
+        bind(C::DdlBottom, GRID, NONE, keys(&[k("G")])).when(When::Ddl),
+        bind(C::NextDdlSection, GRID, NONE, keys(&[k("]]")])).when(When::Ddl),
+        bind(C::PreviousDdlSection, GRID, NONE, keys(&[k("[[")])).when(When::Ddl),
+        bind(C::SelectDdlLines, GRID, NONE, keys(&[k("V")])).when(When::Ddl),
+        bind(C::LeaveDdlVisual, GRID, NONE, keys(&[k("esc")])).when(When::Ddl),
         bind(
             C::ReviewSql,
             GRID,
diff --git a/src/keymap/overrides.rs b/src/keymap/overrides.rs
index 1c43e05d..99065da6 100644
--- a/src/keymap/overrides.rs
+++ b/src/keymap/overrides.rs
@@ -261,7 +261,11 @@ fn own_default(layout: Layout, row: Row, spelled: &Spelled) -> bool {
 impl Binding {
     fn bound_mut(&mut self, layout: Layout) -> &mut Bound {
         match layout {
-            Layout::Mac | Layout::Windows => &mut self.mac,
+            Layout::Mac => &mut self.mac,
+            Layout::Windows => match &mut self.windows {
+                Some(windows) => windows,
+                None => &mut self.mac,
+            },
             Layout::Omarchy => &mut self.omarchy,
         }
     }
diff --git a/src/model.rs b/src/model.rs
index 01269ec5..009920eb 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -1104,6 +1104,43 @@ pub enum Action {
         tab: ConnTabId,
         object_tab: TabId,
     },
+    /// Move the cursor of the terminal look's DDL text `by` lines.
+    MoveDdlCursor {
+        tab: ConnTabId,
+        id: TabId,
+        by: isize,
+    },
+    /// Put that cursor on `line` (the last one where it has fewer).
+    DdlCursorTo {
+        tab: ConnTabId,
+        id: TabId,
+        line: usize,
+    },
+    /// Move it to the start of the next part of the text, or of the one
+    /// before (`]]`, `[[`).
+    DdlSection {
+        tab: ConnTabId,
+        id: TabId,
+        forward: bool,
+    },
+    /// Start the selection of lines at the cursor, or end it (`V`).
+    ToggleDdlVisual {
+        tab: ConnTabId,
+        id: TabId,
+    },
+    /// End the selection of lines.
+    LeaveDdlVisual {
+        tab: ConnTabId,
+        id: TabId,
+    },
+    /// Open the DDL view's text in a new SQL editor, which runs nothing.
+    OpenDdlInEditor {
+        tab: ConnTabId,
+        id: TabId,
+    },
+    /// The terminal's `:ddl`: the DDL view of the table under the tree's
+    /// cursor, or of the tab in front.
+    OpenDdl(ConnTabId),
     /// The connection dialog's driver switch.
     SetDriver(Driver),
     /// Drop the connection dialog's saved database password: saved with
@@ -3697,6 +3734,12 @@ pub struct ObjectTab {
     /// When the definition the tab holds arrived: the view says how long
     /// ago it was read.
     pub ddl_read: Option<std::time::Instant>,
+    /// The terminal look's cursor in the DDL view's text: a line, from
+    /// nought. Held inside the text's lines.
+    pub ddl_cursor: usize,
+    /// Where the terminal look's selection of lines (`V`) started, while
+    /// it is on: it runs from there to the cursor's line.
+    pub ddl_visual: Option<usize>,
     pub selection: Option<CellPos>,
     /// The rows selected with the cursor's, where there are several.
     /// Written with the cursor, by [`ObjectTab::set_cursor`].
@@ -3946,6 +3989,8 @@ impl ObjectTab {
             structure: Fetch::default(),
             ddl: Fetch::default(),
             ddl_read: None,
+            ddl_cursor: 0,
+            ddl_visual: None,
             selection: None,
             range: None,
             estimated_rows,
@@ -3962,6 +4007,30 @@ impl ObjectTab {
         }
     }
 
+    /// The DDL view's text as the view shows it: none while it is first
+    /// read, nor where its read failed (a failed Reload keeps the text it
+    /// had, under the error).
+    pub fn ddl_shown(&self) -> Option<&tabletist_db::Ddl> {
+        self.ddl
+            .value
+            .as_ref()
+            .filter(|_| self.ddl.shown_error().is_none())
+    }
+
+    /// How many lines the DDL view's text has, as it shows: none before it
+    /// is read.
+    pub fn ddl_lines(&self) -> usize {
+        self.ddl_shown().map_or(0, |ddl| ddl.text().lines().count())
+    }
+
+    /// The lines the terminal look's selection takes, while it is on: from
+    /// its start to the cursor's line, whichever comes first.
+    pub fn ddl_selected(&self) -> Option<std::ops::Range<usize>> {
+        let anchor = self.ddl_visual?;
+        let (from, to) = (anchor.min(self.ddl_cursor), anchor.max(self.ddl_cursor));
+        Some(from..to + 1)
+    }
+
     /// Forgets the exact count: the rows it counted changed (filters,
     /// refresh). Returns the count still running for them, to cancel.
     #[must_use]
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
index efa4298c..36841e25 100644
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -186,8 +186,16 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
     let added = object.edits.added.len();
     // The terminal's DDL view says what its text is, in the place of the
     // counts.
-    // How long the DDL view's read has gone on, while it goes on.
+    // How long the DDL view's read has gone on, while it goes on, and
+    // whether its text is there to copy and to open.
     let reading = object.ddl.running_for();
+    let ddl_ready = view == ObjectView::Ddl && object.ddl_shown().is_some();
+    // Production is named beside the table's name, in the looks with a
+    // pointer: the terminal's bar says it in its own tag.
+    let production = app
+        .workspace(tab)
+        .map(|workspace| workspace.environment)
+        .filter(|env| env.is_production() && !look.terminal);
     let parts = match (look.terminal, view) {
         (true, ObjectView::Ddl) => super::ddl::subtitle(object, locale),
         _ => subtitle(object, &look, locale),
@@ -203,6 +211,16 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
         lock.map(|lock| cell_editor::lock_text(lock, &name, locale))
     });
     let mut actions = Vec::new();
+    // The DDL view's Copy was clicked: copied once the header is drawn.
+    let mut copy_ddl = false;
+    // Each view's key, for the switch: beside its word in the terminal,
+    // in its tooltip elsewhere.
+    let view_keys = [
+        crate::keymap::Command::ShowRows,
+        crate::keymap::Command::ShowStructure,
+        crate::keymap::Command::ShowDdl,
+    ]
+    .map(|command| crate::ui::keys::written(ui.ctx(), &look, command));
     // macOS: 14 above and 12 below the title and its line, 2 apart.
     // Terminal: 12 above and 10 below a line holding a 2 pt underline.
     let title_role = TextRole::pick(&look, TextRole::TableTitle, TextRole::OTableTitle);
@@ -244,22 +262,37 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
             if look.terminal {
                 let center = rect.top() + (rect.height() - 1.0 + 2.0) / 2.0;
                 let role = TextRole::OBody;
-                let widths: Vec<f32> = views
-                    .iter()
-                    .map(|(_, label, _)| width(role, &label.to_lowercase()))
-                    .collect();
+                // `r rows`: the key, then the word.
+                let words = |keyed: bool| -> Vec<String> {
+                    views
+                        .iter()
+                        .zip(&view_keys)
+                        .map(|((_, label, _), key)| match keyed && !key.is_empty() {
+                            true => format!("{} {}", &**key, label.to_lowercase()),
+                            false => label.to_lowercase(),
+                        })
+                        .collect()
+                };
                 // The views keep their place at the right, 16 clear of
                 // their 4 pt hit margin; the summary gives way, then the
-                // title is cut.
+                // views' keys, and only then is the title cut.
                 // `+1`, before the views: the summary gives way to it too.
                 let added = (added > 0).then(|| format!("+{added}"));
                 let added_room = added.as_ref().map_or(0.0, |text| width(role, text) + 14.0);
-                let room = right
-                    - (widths.iter().sum::<f32>() + 14.0 * (widths.len() - 1) as f32)
-                    - 4.0
-                    - 16.0
-                    - added_room
-                    - left;
+                let room_of = |widths: &[f32]| {
+                    right
+                        - (widths.iter().sum::<f32>() + 14.0 * (widths.len() - 1) as f32)
+                        - 4.0
+                        - 16.0
+                        - added_room
+                        - left
+                };
+                let widths_of = |keyed| -> Vec<f32> {
+                    words(keyed).iter().map(|word| width(role, word)).collect()
+                };
+                let keyed = room_of(&widths_of(true)) >= width(title_role, &name);
+                let widths = widths_of(keyed);
+                let room = room_of(&widths);
                 let shown =
                     grid::ellipsize(&name, room.max(0.0), false, |text| width(title_role, text));
                 let x =
@@ -281,7 +314,9 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                 tab_list(&list, locale);
                 let ui = &list;
                 let mut x = start;
-                for ((target, label, salt), width) in views.iter().zip(widths) {
+                for (((target, label, salt), width), key) in
+                    views.iter().zip(widths).zip(&view_keys)
+                {
                     let label = label.to_lowercase();
                     let hit = Rect::from_min_size(
                         pos2(x - 4.0, rect.top()),
@@ -306,8 +341,12 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                     } else {
                         palette.dim
                     };
-                    // The word takes the state's colour.
-                    widgets::paint_text(ui, x, center, Text::new(&look).add(role, &label, color));
+                    // The key muted, the word in the state's colour.
+                    let mut said = Text::new(&look);
+                    if keyed && !key.is_empty() {
+                        said = said.add(role, key, palette.dim).space(role, " ");
+                    }
+                    widgets::paint_text(ui, x, center, said.add(role, &label, color));
                     if selected {
                         let y = center + line(ui, role, &look) / 2.0 + 2.0;
                         ui.painter().rect_filled(
@@ -362,7 +401,12 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
             };
             // In the DDL view, what it says while it reads stands where
             // Add row would.
-            let wait = match view {
+            // Its Copy and Open in SQL editor stand there once its text
+            // is, and drop their keys before the title is cut.
+            let wait = |keyed: bool| match view {
+                ObjectView::Ddl if ddl_ready => {
+                    12.0 + super::ddl::header_buttons_width(ui, &look, locale, keyed)
+                }
                 ObjectView::Ddl => {
                     let skin = (&look, &palette, locale);
                     let width = super::ddl::header_wait_width(ui, reading, skin);
@@ -370,11 +414,23 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                 }
                 ObjectView::Data | ObjectView::Structure => 0.0,
             };
+            // The production badge 8 after the title, on its line.
+            let badge = production.map_or(0.0, |env| {
+                8.0 + super::workspace::env_badge_width(
+                    ui,
+                    env,
+                    super::workspace::Badge::Mac,
+                    &look,
+                )
+            });
+            let title_width = width(title_role, &name);
+            let free = right - left - 16.0 - 12.0 - switch - badge;
+            let keyed = free - wait(true) >= title_width;
+            let wait = wait(keyed);
             let stack = |button: Option<bool>| {
                 let button = button.map_or(0.0, |short| 12.0 + add_row(short).width(ui, &look));
-                right - left - 16.0 - 12.0 - switch - button - wait
+                free - button - wait
             };
-            let title_width = width(title_role, &name);
             // The DDL view has no Add row.
             let rows = if view == ObjectView::Ddl {
                 &[None][..]
@@ -388,7 +444,18 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                 .flatten();
             let room = stack(button).max(0.0);
             let shown = grid::ellipsize(&name, room, false, |text| width(title_role, text));
-            let title_width = paint_named(ui, left, title_y, title(&shown, palette.text), &name);
+            let mut title_width =
+                paint_named(ui, left, title_y, title(&shown, palette.text), &name);
+            if let Some(env) = production {
+                let platform = crate::env::Platform::of(&look);
+                let colors = crate::env::env_colors(env, platform, &palette);
+                let x = left + title_width + 8.0;
+                let badge = super::workspace::Badge::Mac;
+                let width = super::workspace::env_badge(ui, x, title_y, env, &colors, badge, &look);
+                let named = Rect::from_min_size(pos2(x, title_y - 8.0), vec2(width, 16.0));
+                widgets::announce(ui, named, env.label(platform));
+                title_width += 8.0 + width;
+            }
             let sub_width = fit_parts(&parts, room, |text| width(sub_role, text))
                 .map_or(0.0, |shown| {
                     paint_named(ui, left, sub_y, sub(&shown, palette.dim), &summary)
@@ -402,7 +469,9 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
             tab_list(&list, locale);
             let header = &mut *ui;
             let ui = &mut list;
-            for (index, ((target, label, _), width)) in views.iter().zip(widths).enumerate() {
+            for (index, (((target, label, _), width), key)) in
+                views.iter().zip(widths).zip(&view_keys).enumerate()
+            {
                 let cell = Rect::from_min_size(pos2(x, track.top() + 3.0), vec2(width, 28.0));
                 x += width;
                 let selected = view == *target;
@@ -419,6 +488,11 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                     WidgetInfo::selected(WidgetType::Button, true, selected, label.as_ref())
                 });
                 view_tab(ui, &response, selected);
+                // "Rows · ⌃⌘1" under the pointer.
+                let response = match key.is_empty() {
+                    true => response,
+                    false => response.on_hover_text(format!("{label} · {}", &**key)),
+                };
                 let radius = look.radius.saturating_sub(2);
                 focus::hint(ui, &response, cell, focus::Ring::Edge { radius });
                 if selected {
@@ -462,7 +536,18 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
             let ui = header;
             if view == ObjectView::Ddl {
                 let skin = (&look, &palette, locale);
-                super::ddl::header_wait(ui, reading, right, center, skin);
+                if ddl_ready {
+                    let asked = super::ddl::show_header_buttons(ui, right, center, keyed, skin);
+                    copy_ddl = asked.copy;
+                    if asked.open {
+                        actions.push(Action::OpenDdlInEditor {
+                            tab,
+                            id: object_tab,
+                        });
+                    }
+                } else {
+                    super::ddl::header_wait(ui, reading, right, center, skin);
+                }
             } else if let Some(short) = button {
                 let button = add_row(short);
                 let width = button.width(ui, &look);
@@ -485,6 +570,9 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                 }
             }
         });
+    if copy_ddl {
+        super::ddl::copy(app, ui.ctx(), tab, object_tab);
+    }
     app.actions.extend(actions);
 }
 
diff --git a/src/ui/ddl.rs b/src/ui/ddl.rs
index c385d5ab..7297421a 100644
--- a/src/ui/ddl.rs
+++ b/src/ui/ddl.rs
@@ -1,14 +1,19 @@
 //! The DDL view: a table's or a view's definition as read-only text, in
 //! the code face with the SQL editor's colours and line numbers, and where
-//! it comes from. The text is selected and copied as any text is. A read
-//! that failed says the database's error, and never shows part of a text.
+//! it comes from. In the looks with a pointer the text is a field, selected
+//! and copied as any text is, with links to its parts over it; in the
+//! terminal look it is lines with a cursor, selected by line, beside an
+//! outline of its parts. A read that failed says the database's error, and
+//! never shows part of a text.
 
 use std::borrow::Cow;
 use std::time::Duration;
 
-use egui::{Frame, Id, Margin, Rect, UiBuilder, pos2, vec2};
-use tabletist_db::ddl::Source;
-use tabletist_db::{Driver, ObjectKind};
+use egui::{
+    CornerRadius, Frame, Id, Margin, Rect, Sense, UiBuilder, WidgetInfo, WidgetType, pos2, vec2,
+};
+use tabletist_db::ddl::{PartKind, Source};
+use tabletist_db::{Ddl, Driver, ObjectKind};
 
 use crate::app::App;
 use crate::i18n::{Locale, gettext};
@@ -23,11 +28,173 @@ fn role(look: &Look) -> TextRole {
     TextRole::pick(look, TextRole::Code, TextRole::OCode)
 }
 
-/// The id of the text field of the DDL view of the tab `id`.
+/// The id of the text field of the DDL view of the tab `id`, and of the
+/// terminal look's lines in its place.
 pub fn text_id(tab: ConnTabId, id: TabId) -> Id {
     Id::new(("ddl-text", tab.0, id.0))
 }
 
+/// What a part of the text is called where the view lists it.
+fn part_name(kind: PartKind, object: ObjectKind, locale: Locale) -> Cow<'static, str> {
+    let name = match kind {
+        PartKind::Object => match object {
+            ObjectKind::Table => "Table",
+            ObjectKind::View => "View",
+            ObjectKind::MaterializedView => "Materialized view",
+        },
+        PartKind::Indexes => "Indexes",
+        PartKind::Triggers => "Triggers",
+        PartKind::Comments => "Comments",
+    };
+    gettext(locale, name)
+}
+
+/// How much a part holds, where the view counts it: the statements of
+/// the indexes, the triggers and the comments. The object's is one
+/// statement, and the outline counts its lines instead.
+fn part_count(ddl: &Ddl, kind: PartKind) -> Option<usize> {
+    match kind {
+        PartKind::Object => None,
+        PartKind::Indexes | PartKind::Triggers | PartKind::Comments => {
+            ddl.part(kind).map(|part| part.statements.len())
+        }
+    }
+}
+
+/// The part `line` is in: the last that starts at or before it.
+fn part_at(placed: &[(PartKind, std::ops::Range<usize>)], line: usize) -> Option<PartKind> {
+    placed
+        .iter()
+        .rev()
+        .find(|(_, lines)| lines.start <= line)
+        .or(placed.first())
+        .map(|(kind, _)| *kind)
+}
+
+/// Copies the text of the DDL view of the tab `id`: what is selected in
+/// it (the field's selection, or the terminal look's lines), or else all
+/// of it, and says so. Nothing before the text is read.
+pub fn copy(app: &App, ctx: &egui::Context, tab: ConnTabId, id: TabId) {
+    let Some(object) = app
+        .workspace(tab)
+        .and_then(|workspace| workspace.object_tab(id))
+    else {
+        return;
+    };
+    // Only the text on screen: not one a failed Reload keeps under its
+    // error.
+    let Some(ddl) = object.ddl_shown() else {
+        return;
+    };
+    let text = ddl.text();
+    let part = match app.look.terminal {
+        true => object.ddl_selected().map(|lines| {
+            text.lines()
+                .skip(lines.start)
+                .take(lines.len())
+                .collect::<Vec<_>>()
+                .join("\n")
+        }),
+        false => selected_text(ctx, text_id(tab, id), &text),
+    };
+    let locale = app.locale;
+    let said = match &part {
+        Some(part) => {
+            let lines = part.lines().count().max(1);
+            let count = u32::try_from(lines).unwrap_or(u32::MAX);
+            crate::i18n::ngettext(locale, "Copied {n} line", "Copied {n} lines", count)
+                .replace("{n}", &lines.to_string())
+        }
+        None => gettext(locale, "Copied DDL").into_owned(),
+    };
+    ctx.copy_text(part.unwrap_or(text));
+    crate::ui::toast::say(ctx, &app.look.label(&said));
+}
+
+/// What the field `id` has selected of `text`, where it has anything.
+fn selected_text(ctx: &egui::Context, id: Id, text: &str) -> Option<String> {
+    let range = egui::TextEdit::load_state(ctx, id)?.cursor.char_range()?;
+    let (one, other) = (range.primary.index.0, range.secondary.index.0);
+    let (from, to) = (one.min(other), one.max(other));
+    (from < to).then(|| text.chars().skip(from).take(to - from).collect())
+}
+
+/// What a click on the header's buttons asked for.
+#[derive(Default)]
+pub struct Asked {
+    pub copy: bool,
+    pub open: bool,
+}
+
+/// The macOS header's two buttons in the DDL view, Copy and Open in SQL
+/// editor, with their keys where `keyed` (they drop them when the header
+/// is short of room, before its title is cut).
+fn header_buttons<'a>(
+    words: &'a (String, String),
+    keys: &'a (crate::keymap::Written, crate::keymap::Written),
+    keyed: bool,
+) -> [widgets::ButtonSpec<'a>; 2] {
+    let copy = widgets::ButtonSpec::new(&words.0)
+        .icon(Icon::Copy)
+        .role(TextRole::UiBodyStrong);
+    let open = widgets::ButtonSpec::new(&words.1)
+        .icon(Icon::Code)
+        .primary();
+    match keyed {
+        true => [copy.shortcut(&keys.0), open.shortcut(&keys.1)],
+        false => [copy, open],
+    }
+}
+
+/// The words and keys of the header's buttons.
+fn header_words(
+    ctx: &egui::Context,
+    look: &Look,
+    locale: Locale,
+) -> (
+    (String, String),
+    (crate::keymap::Written, crate::keymap::Written),
+) {
+    (
+        (
+            gettext(locale, "Copy").into_owned(),
+            gettext(locale, "Open in SQL editor").into_owned(),
+        ),
+        (
+            crate::ui::keys::written(ctx, look, Command::CopyDdl),
+            crate::ui::keys::written(ctx, look, Command::OpenDdlInEditor),
+        ),
+    )
+}
+
+/// How wide the header's buttons draw, with the 12 between them.
+pub fn header_buttons_width(ui: &egui::Ui, look: &Look, locale: Locale, keyed: bool) -> f32 {
+    let (words, keys) = header_words(ui.ctx(), look, locale);
+    let [copy, open] = header_buttons(&words, &keys, keyed);
+    copy.width(ui, look) + 12.0 + open.width(ui, look)
+}
+
+/// The header's buttons, ending at `right`, centred on `center`.
+pub fn show_header_buttons(
+    ui: &mut egui::Ui,
+    right: f32,
+    center: f32,
+    keyed: bool,
+    skin: Skin,
+) -> Asked {
+    let (look, palette, locale) = skin;
+    let (words, keys) = header_words(ui.ctx(), look, locale);
+    let [copy, open] = header_buttons(&words, &keys, keyed);
+    let (copy_width, open_width) = (copy.width(ui, look), open.width(ui, look));
+    let place = |x: f32, width: f32| Rect::from_min_size(pos2(x, center - 16.0), vec2(width, 32.0));
+    let open_at = place(right - open_width, open_width);
+    let copy_at = place(open_at.left() - 12.0 - copy_width, copy_width);
+    Asked {
+        copy: copy.show_at(ui, copy_at, look, palette).clicked(),
+        open: open.show_at(ui, open_at, look, palette).clicked(),
+    }
+}
+
 /// Where the text comes from, as the view says it: built from the
 /// catalog, or as the database keeps it.
 fn source_words(source: Source, locale: Locale) -> Cow<'static, str> {
@@ -178,34 +345,97 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
                 }
             });
     } else if let Some(ddl) = &object.ddl.value {
-        let text = ddl.text();
+        let (text, placed) = ddl.laid_out();
         let name = gettext(locale, "DDL of {name}")
             .replace("{name}", &format::display_safe(&object.object.name));
-        let mut body = area;
-        if !look.terminal {
-            // The line over the text: where it comes from, and how long ago
-            // it was read, with the key that reads it again.
-            let strip = Rect::from_min_max(area.min, pos2(area.right(), area.top() + 44.0));
-            body.min.y = strip.bottom();
-            widgets::hline(
-                ui,
-                strip.x_range(),
-                strip.bottom() - 0.5,
-                palette.surface_hover,
-            );
-            let read = object.ddl_read.map(|at| at.elapsed());
-            if source_line(ui, strip, ddl.source(), read, &reload, skin) {
-                actions.push(Action::Refresh(tab));
-            }
-            // The age changes once a second, then once a minute.
-            let tick = match read {
-                Some(read) if read.as_secs() >= 60 => 60,
-                _ => 1,
-            };
-            ui.ctx().request_repaint_after(Duration::from_secs(tick));
-        }
-        let dialect = ddl.driver.dialect();
         let id = text_id(tab, object_tab);
+        let dialect = ddl.driver.dialect();
+        if look.terminal {
+            let lines = Lines {
+                id,
+                name: &name,
+                dialect,
+                cursor: object.ddl_cursor,
+                selected: object.ddl_selected(),
+            };
+            let outline = Outline {
+                ddl,
+                placed: &placed,
+                object: object.kind,
+                cursor: object.ddl_cursor,
+            };
+            let (body, side) = match area.width() >= 700.0 {
+                true => {
+                    let side =
+                        Rect::from_min_max(pos2(area.right() - OUTLINE, area.top()), area.max);
+                    (
+                        Rect::from_min_max(area.min, pos2(side.left(), area.bottom())),
+                        Some(side),
+                    )
+                }
+                false => (area, None),
+            };
+            if let Some(line) = lines.show(ui, body, &text, skin) {
+                actions.push(Action::DdlCursorTo {
+                    tab,
+                    id: object_tab,
+                    line,
+                });
+                actions.push(Action::GridKeys(tab));
+            }
+            if let Some(side) = side
+                && let Some(line) = outline.show(ui, side, skin)
+            {
+                actions.push(Action::DdlCursorTo {
+                    tab,
+                    id: object_tab,
+                    line,
+                });
+            }
+            if lines_gained_keys(ui, id) {
+                actions.push(Action::GridKeys(tab));
+            }
+            app.actions.extend(actions);
+            return;
+        }
+        // The line over the text: where it comes from, and how long ago it
+        // was read, with the key that reads it again; the links to its parts
+        // before it.
+        let mut body = area;
+        let strip = Rect::from_min_max(area.min, pos2(area.right(), area.top() + 44.0));
+        body.min.y = strip.bottom();
+        widgets::hline(
+            ui,
+            strip.x_range(),
+            strip.bottom() - 0.5,
+            palette.surface_hover,
+        );
+        let read = object.ddl_read.map(|at| at.elapsed());
+        let source_left = source_line(ui, strip, ddl.source(), read, &reload, skin);
+        if source_left.1 {
+            actions.push(Action::Refresh(tab));
+        }
+        // Links to the parts, before the source line: the part at the
+        // top of the text is marked.
+        let first: usize = ui
+            .data(|data| data.get_temp(id.with("first-line")))
+            .unwrap_or(0);
+        let links = Links {
+            ddl,
+            placed: &placed,
+            object: object.kind,
+            current: part_at(&placed, first),
+        };
+        let room = Rect::from_min_max(strip.min, pos2(source_left.0 - 16.0, strip.bottom()));
+        if let Some(line) = links.show(ui, room, skin) {
+            ui.data_mut(|data| data.insert_temp(id.with("jump"), line));
+        }
+        // The age changes once a second, then once a minute.
+        let tick = match read {
+            Some(read) if read.as_secs() >= 60 => 60,
+            _ => 1,
+        };
+        ui.ctx().request_repaint_after(Duration::from_secs(tick));
         let field = Field {
             id,
             name: &name,
@@ -258,7 +488,8 @@ fn grey_lines(ui: &egui::Ui, rect: Rect, palette: &Palette) {
 }
 
 /// "Read-only · built from the catalog · read 2 s ago · Reload ⌘R" at the
-/// end of `strip`: Reload is a link. Returns whether it was clicked.
+/// end of `strip`: Reload is a link. Returns where it starts, and whether
+/// Reload was clicked.
 fn source_line(
     ui: &mut egui::Ui,
     strip: Rect,
@@ -266,7 +497,7 @@ fn source_line(
     read: Option<Duration>,
     reload: &crate::keymap::Written,
     (look, palette, locale): Skin,
-) -> bool {
+) -> (f32, bool) {
     let side = 24.0;
     let center = strip.center().y;
     let role = widgets::secondary(look);
@@ -296,7 +527,360 @@ fn source_line(
     let left = at.left() - laid.paint_right(ui.painter(), at.left(), center);
     let named = Rect::from_min_max(pos2(left, center - 8.0), pos2(at.left(), center + 8.0));
     widgets::announce(ui, named, &words.join(" · "));
-    clicked
+    (left, clicked)
+}
+
+/// The macOS links to the parts of the text, over it: Table, then each
+/// other part with how many statements it holds, the one at the top of the
+/// text marked.
+struct Links<'a> {
+    ddl: &'a Ddl,
+    placed: &'a [(PartKind, std::ops::Range<usize>)],
+    object: ObjectKind,
+    current: Option<PartKind>,
+}
+
+impl Links<'_> {
+    /// The links in `room`, from its left, as many as fit. Returns the
+    /// line of the part whose link was clicked.
+    fn show(&self, ui: &mut egui::Ui, room: Rect, (look, palette, locale): Skin) -> Option<usize> {
+        let center = room.center().y;
+        let role = widgets::body(look);
+        let mut x = room.left() + 24.0;
+        let mut asked = None;
+        for (kind, lines) in self.placed {
+            let name = part_name(*kind, self.object, locale);
+            let count = part_count(self.ddl, *kind).map(|count| count.to_string());
+            let current = self.current == Some(*kind);
+            let (ink, faint) = match current {
+                true => (palette.window, palette.window),
+                false => (palette.accent, palette.dim),
+            };
+            let mut text = Text::new(look).add(role, &name, ink);
+            if let Some(count) = &count {
+                text = text.space(role, " ").add(role, count, faint);
+            }
+            let laid = text.layout(ui.ctx());
+            let width = laid.width() + 20.0;
+            if x + width > room.right() {
+                break;
+            }
+            let rect = Rect::from_min_size(pos2(x, center - 13.0), vec2(width, 26.0));
+            let label = match &count {
+                Some(count) => format!("{name} {count}"),
+                None => name.to_string(),
+            };
+            let response = ui.interact(
+                rect,
+                ui.id().with(("ddl-link", label.as_str())),
+                Sense::click(),
+            );
+            response.widget_info(|| WidgetInfo::selected(WidgetType::Link, true, current, &label));
+            ui.ctx()
+                .accesskit_node_builder(response.id, |node| node.set_selected(current));
+            let fill = match (current, response.hovered()) {
+                (true, _) => Some(palette.text),
+                (false, true) => Some(palette.surface_hover),
+                (false, false) => None,
+            };
+            if let Some(fill) = fill {
+                ui.painter().rect_filled(
+                    rect,
+                    CornerRadius::same(look.radius.saturating_sub(2)),
+                    fill,
+                );
+            }
+            laid.paint_left(ui.painter(), x + 10.0, center);
+            if response.clicked() {
+                asked = Some(lines.start);
+            }
+            x += width + 6.0;
+        }
+        asked
+    }
+}
+
+/// The terminal look's outline beside the text: its width.
+const OUTLINE: f32 = 300.0;
+
+/// Whether the terminal look's lines just got the keyboard, by F6 or a
+/// click: the rows' pane keys follow it.
+fn lines_gained_keys(ui: &egui::Ui, id: Id) -> bool {
+    let had = id.with("had-keyboard");
+    let has = ui.memory(|memory| memory.has_focus(id));
+    let came = has && !ui.data(|data| data.get_temp::<bool>(had)).unwrap_or(false);
+    ui.data_mut(|data| data.insert_temp(had, has));
+    came
+}
+
+/// The terminal look's text: lines with a cursor and a selection of
+/// whole lines, beside a gutter of their numbers. One stop of the
+/// keyboard, as the Structure view's columns are: the view's keys move in
+/// it.
+struct Lines<'a> {
+    id: Id,
+    name: &'a str,
+    dialect: tabletist_db::Dialect,
+    cursor: usize,
+    selected: Option<std::ops::Range<usize>>,
+}
+
+impl Lines<'_> {
+    /// `text` in `rect`. Returns the line a click put the cursor on.
+    fn show(
+        &self,
+        ui: &mut egui::Ui,
+        rect: Rect,
+        text: &str,
+        (look, palette, _): Skin,
+    ) -> Option<usize> {
+        let role = role(look);
+        let lines = text.lines().count().max(1);
+        let digits = lines.ilog10() as usize + 1;
+        let digit = role.width(ui.ctx(), look.faces, "0");
+        let gutter = (digits as f32 * digit + 14.0 + 8.0).ceil().max(52.0);
+        let top = 10.0;
+        let mut layouter = sql_text::layouter(look, palette, self.dialect, self.id, false);
+        let galley = layouter(ui, &text, f32::INFINITY);
+        let text_pane = Rect::from_min_max(pos2(rect.left() + gutter, rect.top()), rect.max);
+        let mut child = ui.new_child(UiBuilder::new().id_salt(self.id).max_rect(text_pane));
+        child.set_clip_rect(text_pane.intersect(ui.clip_rect()));
+        // The line the cursor was last brought into view on: a cursor that
+        // moved is brought into view, at once.
+        let seen = self.id.with("seen");
+        let scrolled = egui::ScrollArea::both()
+            .id_salt(("ddl-lines", self.id))
+            .auto_shrink([false, false])
+            .show(&mut child, |ui| {
+                let size = galley.size() + vec2(16.0, top + 16.0);
+                let (area, _) = ui.allocate_exact_size(size, Sense::hover());
+                let origin = area.min + vec2(0.0, top);
+                let row = |line: usize| {
+                    galley.rows.get(line).map(|row| {
+                        let y = origin.y + row.min_y();
+                        egui::Rangef::new(y, y + row.height())
+                    })
+                };
+                let was: Option<usize> = ui.data(|data| data.get_temp(seen));
+                if was != Some(self.cursor)
+                    && let Some(y) = row(self.cursor)
+                {
+                    ui.data_mut(|data| data.insert_temp(seen, self.cursor));
+                    let across = ui.clip_rect().x_range();
+                    let jump = egui::style::ScrollAnimation::none();
+                    ui.scroll_to_rect_animation(Rect::from_x_y_ranges(across, y), None, jump);
+                }
+                (
+                    origin,
+                    galley
+                        .rows
+                        .iter()
+                        .map(|row| row.height())
+                        .next()
+                        .unwrap_or(16.0),
+                )
+            });
+        let (origin, height) = scrolled.inner;
+        let clip = rect.intersect(ui.clip_rect());
+        let painter = ui.painter().with_clip_rect(clip);
+        let band = |line: usize| {
+            let y = origin.y + line as f32 * height;
+            Rect::from_x_y_ranges(rect.x_range(), egui::Rangef::new(y, y + height))
+        };
+        // Behind the text: the selected lines, then the cursor's, with a
+        // bar at its left. Painted before the text, whose glyphs stand
+        // over them.
+        if let Some(selected) = &self.selected {
+            for line in selected.clone() {
+                painter.rect_filled(band(line), CornerRadius::ZERO, palette.surface_active);
+            }
+        }
+        let cursor = band(self.cursor);
+        painter.rect_filled(cursor, CornerRadius::ZERO, palette.selection);
+        painter.rect_filled(
+            Rect::from_min_size(cursor.min, vec2(3.0, cursor.height())),
+            CornerRadius::ZERO,
+            palette.accent,
+        );
+        ui.painter()
+            .with_clip_rect(text_pane.intersect(ui.clip_rect()))
+            .galley(origin, galley.clone(), palette.text);
+        // The numbers of the lines in sight; the cursor's stands out.
+        let first = ((clip.top() - origin.y) / height).floor().max(0.0) as usize;
+        for line in first..lines {
+            let y = origin.y + line as f32 * height;
+            if y > clip.bottom() {
+                break;
+            }
+            let color = if line == self.cursor {
+                palette.warning
+            } else {
+                palette.dim
+            };
+            let number = Text::one(look, role, &(line + 1).to_string(), color).layout(ui.ctx());
+            number.paint_right(&painter, rect.left() + gutter - 14.0, y + height / 2.0);
+        }
+        // One stop for the keyboard, named as the text and holding it, for
+        // a screen reader. A click puts the cursor on the line under it.
+        let stop = ui.interact(clip, self.id, Sense::click() | Sense::FOCUSABLE);
+        ui.ctx().accesskit_node_builder(stop.id, |node| {
+            node.set_role(egui::accesskit::Role::MultilineTextInput);
+            node.set_label(self.name);
+            node.set_value(text);
+            node.set_read_only();
+        });
+        focus::pane(ui, &stop);
+        focus::hint(ui, &stop, clip, focus::Ring::Own);
+        focus::claim(ui, focus::Region::Grid, &stop);
+        let clicked = stop
+            .clicked()
+            .then(|| stop.interact_pointer_pos())
+            .flatten();
+        clicked.map(|at| (((at.y - origin.y) / height).floor().max(0.0) as usize).min(lines - 1))
+    }
+}
+
+/// The terminal look's outline of the text: its parts, the cursor's
+/// marked, what is not in it, and the view's keys.
+struct Outline<'a> {
+    ddl: &'a Ddl,
+    placed: &'a [(PartKind, std::ops::Range<usize>)],
+    object: ObjectKind,
+    cursor: usize,
+}
+
+impl Outline<'_> {
+    /// The outline in `rect`. Returns the line of a part that was clicked.
+    fn show(&self, ui: &mut egui::Ui, rect: Rect, (look, palette, locale): Skin) -> Option<usize> {
+        ui.painter()
+            .rect_filled(rect, CornerRadius::ZERO, palette.panel);
+        widgets::vline(ui, rect.left() + 0.5, rect.y_range(), palette.outline);
+        let role = TextRole::OBody;
+        let pitch = 22.0;
+        let left = rect.left() + 14.0;
+        let right = rect.right() - 14.0;
+        let mut y = rect.top() + 12.0 + pitch / 2.0;
+        let key = |command| crate::ui::keys::written(ui.ctx(), look, command);
+        let (next, previous) = (
+            key(Command::NextDdlSection),
+            key(Command::PreviousDdlSection),
+        );
+        let heading = gettext(locale, "sections · {next} next  {previous} prev")
+            .replace("{next}", &next)
+            .replace("{previous}", &previous);
+        widgets::paint_label(ui, left, y, Text::one(look, role, &heading, palette.dim));
+        y += pitch + 6.0;
+        let current = part_at(self.placed, self.cursor);
+        let mut asked = None;
+        for (kind, lines) in self.placed {
+            let name = part_name(*kind, self.object, locale).to_lowercase();
+            let count = part_count(self.ddl, *kind).unwrap_or(lines.len());
+            let on = current == Some(*kind);
+            let color = if on { palette.accent } else { palette.text };
+            let row = Rect::from_x_y_ranges(
+                rect.x_range(),
+                egui::Rangef::new(y - pitch / 2.0, y + pitch / 2.0),
+            );
+            let label = format!("{name} {count}");
+            let response = ui.interact(
+                row,
+                ui.id().with(("ddl-outline", label.as_str())),
+                Sense::click(),
+            );
+            response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, &label));
+            ui.ctx()
+                .accesskit_node_builder(response.id, |node| node.set_selected(on));
+            let mark = if on { "▌ " } else { "  " };
+            widgets::paint_text(
+                ui,
+                left,
+                y,
+                Text::one(look, role, &format!("{mark}{name}"), color),
+            );
+            widgets::paint_text_right(
+                ui,
+                right,
+                y,
+                Text::one(look, role, &count.to_string(), color),
+            );
+            if response.clicked() {
+                asked = Some(lines.start);
+            }
+            y += pitch;
+        }
+        if let Some(left_out) = left_out_lines(self.ddl.driver, locale) {
+            y += 14.0;
+            widgets::paint_label(
+                ui,
+                left,
+                y,
+                Text::one(look, role, &gettext(locale, "not included"), palette.dim),
+            );
+            y += pitch;
+            for line in left_out {
+                widgets::paint_label(ui, left, y, Text::one(look, role, &line, palette.dim));
+                y += pitch;
+            }
+        }
+        y += 14.0;
+        widgets::paint_label(
+            ui,
+            left,
+            y,
+            Text::one(look, role, &gettext(locale, "keys"), palette.dim),
+        );
+        y += pitch;
+        let (copy, lines) = (key(Command::CopyDdl), key(Command::SelectDdlLines));
+        let keys = [
+            (copy.to_string(), "copy all"),
+            (format!("{} {}", &*lines, &*copy), "copy lines"),
+            (
+                key(Command::OpenDdlInEditor).to_string(),
+                "open in sql editor",
+            ),
+            (
+                format!("{} {}", &*key(Command::DdlTop), &*key(Command::DdlBottom)),
+                "top · bottom",
+            ),
+            (key(Command::OpenDdl).to_string(), "open from anywhere"),
+            (key(Command::Reload).to_string(), "read again"),
+        ];
+        let column = keys
+            .iter()
+            .map(|(keys, _)| TextRole::OBody.width(ui.ctx(), look.faces, keys))
+            .fold(0.0, f32::max)
+            + 16.0;
+        for (keys, word) in keys {
+            if keys.trim().is_empty() {
+                continue;
+            }
+            let word = gettext(locale, word);
+            let said = Text::new(look).add(TextRole::OGroup, &keys, palette.accent);
+            widgets::paint_text(ui, left, y, said);
+            let width = widgets::paint_text(
+                ui,
+                left + column,
+                y,
+                Text::one(look, role, &word, palette.text),
+            );
+            // The key with what it does, for a screen reader: one line.
+            let place = Rect::from_min_size(pos2(left, y - 8.0), vec2(column + width, 16.0));
+            widgets::announce(ui, place, &format!("{keys} {word}"));
+            y += pitch;
+        }
+        asked
+    }
+}
+
+/// What PostgreSQL's text leaves out, as the outline lists it.
+fn left_out_lines(driver: Driver, locale: Locale) -> Option<[String; 2]> {
+    match driver {
+        Driver::Postgres => Some([
+            gettext(locale, "owner · grants · owned sequences").into_owned(),
+            gettext(locale, "row-level policies").into_owned(),
+        ]),
+        Driver::MySql | Driver::Sqlite => None,
+    }
 }
 
 /// The macOS footer of the DDL view: how many lines and whose, what the
@@ -533,27 +1117,58 @@ impl Field<'_> {
         ui.data_mut(|data| data.insert_temp(had, has));
         let font = role.font_id(look.faces);
         let mut layouter = sql_text::layouter(look, palette, self.dialect, self.id, false);
-        let scrolled = egui::ScrollArea::both()
+        // A link to a part asked for its line at the top: where the code's
+        // own rows lay it.
+        let jump: Option<usize> = ui.data_mut(|data| data.remove_temp(self.id.with("jump")));
+        let mut area = egui::ScrollArea::both()
             .id_salt(("ddl-scroll", self.id))
-            .auto_shrink([false, false])
-            .show(&mut child, |ui| {
-                // The text the field reads and cannot change.
-                let mut fixed: &str = text;
-                egui::TextEdit::multiline(&mut fixed)
-                    .id(self.id)
-                    .font(font)
-                    .frame(Frame::new().inner_margin(Margin {
-                        left: 0,
-                        right: 16,
-                        top: top as i8,
-                        bottom: 16,
-                    }))
-                    .desired_width(f32::INFINITY)
-                    .desired_rows(lines)
-                    .layouter(&mut layouter)
-                    .show(ui)
-            });
+            .auto_shrink([false, false]);
+        if let Some(line) = jump {
+            let galley = layouter(ui, &text, f32::INFINITY);
+            // Under the field's margin, which shows over the first line
+            // only.
+            let row = galley.rows.get(line).or(galley.rows.last());
+            let at = row
+                .filter(|_| line > 0)
+                .map_or(0.0, |row| row.min_y() + top);
+            area = area.vertical_scroll_offset(at);
+        }
+        let scrolled = area.show(&mut child, |ui| {
+            // The text the field reads and cannot change.
+            let mut fixed: &str = text;
+            egui::TextEdit::multiline(&mut fixed)
+                .id(self.id)
+                .font(font)
+                .frame(Frame::new().inner_margin(Margin {
+                    left: 0,
+                    right: 16,
+                    top: top as i8,
+                    bottom: 16,
+                }))
+                .desired_width(f32::INFINITY)
+                .desired_rows(lines)
+                .layouter(&mut layouter)
+                .show(ui)
+        });
         let output = scrolled.inner;
+        // The line at the top, for the links to mark its part. A part near
+        // the end cannot come to the top: the link that asked for it stays
+        // marked until the text is scrolled.
+        let offset = scrolled.state.offset.y;
+        let jumped = self.id.with("jumped");
+        if let Some(line) = jump {
+            ui.data_mut(|data| data.insert_temp(jumped, (line, offset)));
+        }
+        let held: Option<(usize, f32)> = ui.data(|data| data.get_temp(jumped));
+        let first = match held {
+            Some((line, at)) if (at - offset).abs() < 0.5 => line,
+            _ => {
+                ui.data_mut(|data| data.remove_temp::<(usize, f32)>(jumped));
+                let rows = &output.galley.rows;
+                rows.partition_point(|row| row.max_y() <= offset - top + 1.0)
+            }
+        };
+        ui.data_mut(|data| data.insert_temp(self.id.with("first-line"), first));
         ui.ctx().accesskit_node_builder(self.id, |node| {
             node.set_label(self.name);
             node.set_read_only();
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index e91af11c..35c1b05d 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -108,7 +108,12 @@ pub(crate) fn held_is(layout: Layout, held: Modifiers, mods: Mods) -> bool {
                 command: mods.cmd,
             };
             // As `is_press`: Cmd is held only where the chord names it.
-            held.matches_exact(pattern) && (mods.cmd || !held.mac_cmd)
+            // On a Mac, where Control is a key of its own beside Cmd, it
+            // is held only where the chord names it too: egui lets it by
+            // a chord of Cmd's, and ⌃⌘1 would be ⌘1 as well.
+            held.matches_exact(pattern)
+                && (mods.cmd || !held.mac_cmd)
+                && (!held.mac_cmd || held.ctrl == mods.ctrl)
         }
     }
 }
@@ -444,12 +449,6 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     // An editor that just opened takes the keyboard when its field is
     // first drawn, later in this frame: the keys are its own already, and
     // none of them is the grid's (Space, an arrow). So with the prompt.
-    // The DDL view's text, which cannot be typed into, has the keyboard:
-    // its keys are the field's, but for Reload's letter (below).
-    let in_ddl_text = object.is_some_and(|(tab, id)| {
-        let text = crate::ui::ddl::text_id(tab, id);
-        ctx.memory(|memory| memory.has_focus(text))
-    });
     let editing = ctx.text_edit_focused() || open || field_asked || prompt;
     // What the prompt refused is said until the next key, whatever the key
     // goes on to do. A fresh press: the Enter that ran the line may still
@@ -503,11 +502,12 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     });
     // The DDL view of the table on screen: its text has the arrows while
     // it has the keyboard, and none goes to the tree from it.
-    let ddl = object.is_some_and(|(tab, id)| {
+    let ddl_tab = object.filter(|&(tab, id)| {
         app.workspace(tab)
             .and_then(|workspace| workspace.object_tab(id))
             .is_some_and(|object| object.view == crate::model::ObjectView::Ddl)
     });
+    let ddl = ddl_tab.is_some();
     // The History tab's list is somewhere else: its arrows are its own,
     // unless the person went to the tree.
     let tree_arrows = !editing
@@ -520,6 +520,11 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     // a dialog.
     let on_structure =
         structure.filter(|_| !editing && !tree_arrows && !focused && app.dialog.is_none());
+    // The terminal look's DDL view, by the same rule: its text is lines
+    // with a cursor, not a field.
+    let on_ddl = ddl_tab.filter(|_| {
+        app.look.terminal && !editing && !tree_arrows && !focused && app.dialog.is_none()
+    });
     // The SQL editor on screen, while it has the keyboard: the completion
     // list's keys are its keys, and no other pane's. (A text field then
     // has the keyboard: no need to ask `editing` as well.)
@@ -710,6 +715,8 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     // Whether the menu of the server's other databases was asked for, by
     // the chord that opens it under the bar's own chip.
     let mut menu_asked = false;
+    // The DDL view's text was asked to be copied, by its chord.
+    let mut copy_ddl = None;
     ctx.input_mut(|input| {
         // The keymap says which key: each line asks whether its command's
         // chord went down, in the scope it is read in. The guards say when
@@ -996,6 +1003,64 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
                 actions.push(Action::UseAsLabel { tab, id, line });
             }
         }
+        // The terminal look's DDL view: its cursor by the arrows (its
+        // letters are read with the letters), and Esc ends the selection.
+        if let Some((tab, id)) = on_ddl {
+            let scope = Scope::Grid;
+            for keys in keymap.chords_now(layout, Command::MoveInDdl, scope, normal) {
+                let by = match keys {
+                    "up" => -1,
+                    "down" => 1,
+                    _ => continue,
+                };
+                let presses = presses(input, layout, keys).count;
+                actions.extend((0..presses).map(|_| Action::MoveDdlCursor { tab, id, by }));
+            }
+            // Esc is the selection's only while there is one: else it
+            // closes what it closes in the other views (Review SQL, the
+            // save's note).
+            let visual = app
+                .workspace(tab)
+                .and_then(|workspace| workspace.object_tab(id))
+                .is_some_and(|object| object.ddl_visual.is_some());
+            if visual && on(input, Command::LeaveDdlVisual, scope).fresh {
+                actions.push(Action::LeaveDdlVisual { tab, id });
+            }
+        }
+        // A table's views, wherever the keyboard is in the window, by the
+        // chords of the looks with a pointer (the terminal's are letters).
+        if !terminal
+            && app.dialog.is_none()
+            && let Some((tab, object_tab)) = object
+        {
+            use crate::model::ObjectView;
+            let views = [
+                (Command::ShowRows, ObjectView::Data),
+                (Command::ShowStructure, ObjectView::Structure),
+                (Command::ShowDdl, ObjectView::Ddl),
+            ];
+            for (command, view) in views {
+                if on(input, command, Scope::Grid).fresh {
+                    actions.push(Action::SetView {
+                        tab,
+                        object_tab,
+                        view,
+                    });
+                }
+            }
+        }
+        // The DDL view's chords, in its text too.
+        if !terminal
+            && app.dialog.is_none()
+            && let Some((tab, id)) = ddl_tab
+        {
+            if on(input, Command::CopyDdl, Scope::Grid).fresh {
+                copy_ddl = Some((tab, id));
+            }
+            if on(input, Command::OpenDdlInEditor, Scope::Grid).fresh {
+                actions.push(Action::OpenDdlInEditor { tab, id });
+            }
+        }
         if tree_arrows && !focused {
             use crate::model::TreeKey;
             let tab = active;
@@ -1108,6 +1173,9 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
             }
         }
     });
+    if let Some((tab, id)) = copy_ddl {
+        crate::ui::ddl::copy(app, ctx, tab, id);
+    }
     // The menu opens as a click on the chip opens it: a server with one
     // database has none, and the key does nothing.
     if menu_asked {
@@ -1268,16 +1336,6 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         // The keys are a field's: a first key that waited is not the
         // first of what is typed once they are the grid's again.
         forget_pending(ctx);
-        // Nothing is typed into the DDL view's text: Reload's letter
-        // reads it again from there. (Its chord is read in every field.)
-        if in_ddl_text && app.dialog.is_none() {
-            for char in typed_chars(ctx) {
-                if window_letter(app, scopes, char) == Some(Command::Reload) {
-                    take_char(ctx, char);
-                    actions.push(Action::Refresh(active));
-                }
-            }
-        }
     }
     // Ahead of what the key asked for, which may be the prompt again. Put
     // there only now: a letter that edits acts only where nothing else was
@@ -2463,6 +2521,7 @@ fn letters(
     let rows_show = object.is_some_and(|object| object.view == crate::model::ObjectView::Data);
     let structure_shows =
         object.is_some_and(|object| object.view == crate::model::ObjectView::Structure);
+    let ddl_shows = object.is_some_and(|object| object.view == crate::model::ObjectView::Ddl);
     // The cursor's row is a new one: the row panel is its form, and the
     // key that drops the row is read from the form's fields too.
     let on_new = selection.is_some_and(|cell| crate::edit::new_id(cell.row).is_some());
@@ -2616,7 +2675,9 @@ fn letters(
         offers(Command::RunWithoutTimeout),
     );
     let holds = |when: crate::keymap::When| match when {
-        crate::keymap::When::InspectorOpen => panel,
+        // The DDL view shows no row panel: its brackets are its own
+        // (`]]`, `[[`), not the panel's step.
+        crate::keymap::When::InspectorOpen => panel && !ddl_shows,
         crate::keymap::When::Review => reviewing.is_some(),
         crate::keymap::When::Note => note.is_some(),
         crate::keymap::When::RefusedWrite => has_card,
@@ -2627,6 +2688,7 @@ fn letters(
         // The Structure view shows where the rows would: its own keys
         // come before the rows' there.
         crate::keymap::When::Structure => structure_shows,
+        crate::keymap::When::Ddl => ddl_shows,
         crate::keymap::When::AgentsLog => {
             agents_view(app, tab) == Some(crate::model::AgentsView::Log)
         }
@@ -3108,7 +3170,53 @@ fn letters(
                 let line = None;
                 actions.push(Action::UseAsLabel { tab, id, line });
             }
-            (Some(Command::CopyCells), Some(_)) => {
+            // A table's views, from any of them.
+            (
+                Some(command @ (Command::ShowRows | Command::ShowStructure | Command::ShowDdl)),
+                Some(id),
+            ) => {
+                use crate::model::ObjectView;
+                let view = match command {
+                    Command::ShowStructure => ObjectView::Structure,
+                    Command::ShowDdl => ObjectView::Ddl,
+                    _ => ObjectView::Data,
+                };
+                actions.push(Action::SetView {
+                    tab,
+                    object_tab: id,
+                    view,
+                });
+            }
+            // The DDL view's text, by line.
+            (Some(Command::MoveInDdl), Some(id)) => {
+                let by = if char == 'k' { -1 } else { 1 };
+                actions.push(Action::MoveDdlCursor { tab, id, by });
+            }
+            (Some(command @ (Command::DdlTop | Command::DdlBottom)), Some(id)) => {
+                let line = if command == Command::DdlTop {
+                    0
+                } else {
+                    usize::MAX
+                };
+                actions.push(Action::DdlCursorTo { tab, id, line });
+            }
+            (Some(command @ (Command::NextDdlSection | Command::PreviousDdlSection)), Some(id)) => {
+                let forward = command == Command::NextDdlSection;
+                actions.push(Action::DdlSection { tab, id, forward });
+            }
+            (Some(Command::SelectDdlLines), Some(id)) => {
+                actions.push(Action::ToggleDdlVisual { tab, id });
+            }
+            (Some(Command::CopyDdl), Some(id)) => {
+                crate::ui::ddl::copy(app, ctx, tab, id);
+                actions.push(Action::LeaveDdlVisual { tab, id });
+            }
+            (Some(Command::OpenDdlInEditor), Some(id)) => {
+                actions.push(Action::OpenDdlInEditor { tab, id });
+            }
+            // The rows' cell, where the rows show: no other view copies
+            // one it hides.
+            (Some(Command::CopyCells), Some(_)) if rows_show => {
                 if let Some(text) = app.copy_text(false) {
                     ctx.copy_text(text);
                 }
diff --git a/src/ui/toast.rs b/src/ui/toast.rs
index bb82de1c..ab8528a5 100644
--- a/src/ui/toast.rs
+++ b/src/ui/toast.rs
@@ -78,7 +78,12 @@ pub fn show(app: &App, ctx: &egui::Context) {
                     color: palette.shadow,
                 })
                 .show(ui, |ui| {
-                    widgets::label(ui, widgets::body(look), &text, palette.window, look);
+                    let said = widgets::label(ui, widgets::body(look), &text, palette.window, look);
+                    // Read out as it appears, without taking the user's
+                    // place.
+                    ui.ctx().accesskit_node_builder(said.id, |node| {
+                        node.set_live(egui::accesskit::Live::Polite);
+                    });
                 });
         });
 }
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index 89a2de5c..905e503d 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2182,7 +2182,23 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let ddl = workspace
         .active_object_tab()
         .filter(|object| object.view == ObjectView::Ddl);
-    let reload_key = key(Command::Reload);
+    let ddl_keys = [
+        key(Command::MoveInDdl),
+        key(Command::SelectDdlLines),
+        key(Command::CopyDdl),
+        key(Command::OpenDdlInEditor),
+        app.keymap.together(
+            app.layout(),
+            &[Command::NextDdlSection, Command::PreviousDdlSection],
+        ),
+        key(Command::ShowRows),
+        key(Command::ShowStructure),
+        key(Command::LeaveDdlVisual),
+    ];
+    // The DDL view's text shows (not its read, nor its error), and the
+    // terminal look's selection of its lines is on.
+    let ddl_text = ddl.is_some_and(|object| object.ddl_shown().is_some());
+    let ddl_visual = ddl_text && ddl.is_some_and(|object| object.ddl_visual.is_some());
     // The completion list's own key, while it is open with a row to put
     // in. One whose row is about to be inserted is not open any more: the
     // mode line is drawn before the editor inserts it.
@@ -2190,18 +2206,21 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .active_sql_tab()
         .and_then(|sql| sql.completion.as_ref())
         .is_some_and(|list| !list.accept && !list.candidates.is_empty());
+    // "2 lines selected · 10/27": the selection, then the cursor's line of
+    // all of them, in the place of the rows' range.
     let lines = ddl.and_then(|object| {
-        let shown = object
-            .ddl
-            .value
-            .as_ref()
-            .filter(|_| object.ddl.shown_error().is_none());
-        let lines = shown?.text().lines().count();
-        let count = u32::try_from(lines).unwrap_or(u32::MAX);
-        Some(
-            crate::i18n::ngettext(locale, "{n} line", "{n} lines", count)
-                .replace("{n}", &lines.to_string()),
-        )
+        object.ddl_shown()?;
+        let at = format!("{}/{}", object.ddl_cursor + 1, object.ddl_lines());
+        Some(match object.ddl_selected() {
+            Some(selected) => {
+                let count = u32::try_from(selected.len()).unwrap_or(u32::MAX);
+                let said =
+                    crate::i18n::ngettext(locale, "{n} line selected", "{n} lines selected", count)
+                        .replace("{n}", &selected.len().to_string());
+                format!("{said} · {at}")
+            }
+            None => at,
+        })
     });
     let summary = editor
         .or(lines)
@@ -2380,14 +2399,32 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             if editing.pending.is_some() || editing.added.is_some() {
                 structure_hints.push((&review_key, &*review, true));
             }
-            // The DDL view's: its text is read again, and the rows' keys
-            // do nothing on it. What is pending is saved and reviewed from
+            // The DDL view's, as the board has them: Copy and Open only
+            // where the text shows, Esc only while lines are selected. What
+            // is pending in the table's rows is saved and reviewed from
             // here too.
-            let ddl_words = ["reload", "tables"].map(|word| gettext(locale, word));
-            let mut ddl_hints: Vec<widgets::Hint<'_>> = vec![
-                (&reload_key, &*ddl_words[0], true),
-                (&sidebar_key, &*ddl_words[1], true),
-            ];
+            let ddl_words = [
+                "line",
+                "select lines",
+                "copy",
+                "open in sql",
+                "section",
+                "rows",
+                "structure",
+                "normal",
+            ]
+            .map(|word| gettext(locale, word));
+            let mut ddl_hints: Vec<widgets::Hint<'_>> = ddl_keys
+                .iter()
+                .zip(&ddl_words)
+                .enumerate()
+                .filter(|(index, _)| match index {
+                    2 | 3 => ddl_text,
+                    7 => ddl_visual,
+                    _ => true,
+                })
+                .map(|(_, (key, word))| (key, &**word, true))
+                .collect();
             if !read_only {
                 ddl_hints.push((&save_key, &*write, true));
             }
@@ -2811,6 +2848,15 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 }
                 return;
             }
+            // The DDL view's selection of lines: its mode stands first, and
+            // the rest of the line is drawn after it.
+            let left = if ddl_visual {
+                let mode = gettext(locale, "-- VISUAL LINE --");
+                let mode = Text::one(&look, TextRole::OModeLine, &mode, palette.accent);
+                left + widgets::paint_label(ui, left, y, mode) + gap
+            } else {
+                left
+            };
             // A read-only connection's tag ends the keys. Without one,
             // neither it nor the gap before it takes room from them.
             let tag = read_only.then(|| {
@@ -2843,15 +2889,16 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 (mark, shown, tail, width)
             });
             let said_room = said.as_ref().map_or(0.0, |said| said.3 + gap);
-            let limit = end - fixed - said_room;
+            let limit = end - fixed - said_room - (left - rect.left() - 12.0);
             // The key that adds a row is here only where the table takes
             // none, and the line that saves only on a connection that
             // cannot write.
             let mut disabled: Vec<String> = Vec::new();
-            if !editing.can_add {
+            // The DDL view's line names no key of the rows, struck or not.
+            if !editing.can_add && ddl.is_none() {
                 disabled.push(format!("{add_key} new row"));
             }
-            if !editing.can_delete {
+            if !editing.can_delete && ddl.is_none() {
                 disabled.push(format!("{delete_key} delete"));
             }
             if read_only {
````

- [ ] **Step 4: Run the tests to see them pass**

```bash
~/.cargo/bin/cargo test --locked -p tabletist-db --lib ddl::
~/.cargo/bin/cargo test --locked --lib -- keymap ui::
```

Expected: all pass, `ui::ddl_act_tests` with 25.

- [ ] **Step 5: Run the checks** (above). Expected: every suite passes, the app's library with 3353 passed and 17 ignored.

- [ ] **Step 6: Look at it** (not a test, nothing committed)

Append a throwaway `#[test] #[ignore]` scene to `src/shots.rs` built on `both(name, |harness| ..)`: `workspace(harness)`, `SetView` to `ObjectView::Ddl`, and `harness.answer_ddl(Ok(..))` with a PostgreSQL `Ddl` of `book_images` that has indexes, triggers and comments; one scene on a production connection; one in a window too narrow for the buttons' keys; one on Omarchy with `V` and `j` pressed. Hold the pointer over the Rows tab for its tooltip. Render with `~/.cargo/bin/cargo test --locked --features shots --lib shots::<scene> -- --ignored`, compare `target/shots/*.png` with the boards by eye (the buttons, the links, the outline, the status line), then remove the scene. The PNGs stay local.

- [ ] **Step 7: Commit**

```bash
git add AGENTS.md crates/tabletist-db/src/ddl.rs src docs/_guide docs/_reference
git commit -m "Copy the DDL, open it in a SQL editor, and move through it by key"
```

---

## The pull request

Base: run 1's branch (`claude/tabletist-ddl-view-1b3c93`) while #222 is open, else `main`. Title: "DDL view, run 2: act on it". The body, without em dashes in what it quotes of the UI:

- **Summary**, quoting "There is no way to see the DDL.", and that this is run 2 of three.
- **Copy and Open in SQL editor**: what is copied, what is said, the new tab's mode.
- **Keys**: the spec's table; `s` is Structure everywhere and Replace cell keeps `cc`; Windows' Alt digits (`Binding::windows`); every Cmd chord on a Mac exact about Control, so ⌃⌘1 is not ⌘1 (decided with Igor); Esc, the row panel's brackets and ⇧⌘C in the DDL view.
- **A text the view does not show**: nothing acts on it.
- **Jump links and outline.**
- **Where the build differs from the boards**: the table above.
- No screenshots (or only blurred ones).
