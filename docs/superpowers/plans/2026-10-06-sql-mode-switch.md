# SQL Mode Switch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The SQL editor's Read-only or Read-write switch is drawn as the design canvas now draws it: a two-part segmented control in place of the badge and its menu. What the switch does is unchanged.

**Architecture:** `badge` and `badge_width` in `src/ui/sql_editor.rs` become `switch` and `switch_width`: two segments placed in the rectangle the toolbar already gives the badge, each one `focus::segment`, as the Data or Structure switch of `src/ui/data_view.rs` is. `Action::SetSqlMode`, `Action::ToggleSqlMode`, `Workspace::run_mode`, `Workspace::sql_writes` and `Mod+Shift+M` are untouched. Nothing outside the toolbar changes.

**Tech Stack:** Rust 2024, egui. No new dependency.

**Spec:** the design canvas, section "SQL editor" and "SQL editor · Read-write transaction": `MacSql`, `MacSqlTx`, `MacSqlTxStates`, `OmarchySql`, `OmarchySqlTx`, and the "Segmented" row of `MacComponents`, `MacComponentsDark` and `OmarchyComponents`. The request was "I updated designs for the read-only, read-write switcher", then "don't implement the Auto-commit/Transaction switcher right now" and "the switch already exists, I just changed the design". Design material is not copied into the repository.

**What was checked before this plan was written:** the whole change was built as a draft on this branch. With it applied the four checks of `AGENTS.md` passed on Linux (1745 tests of the app). The tests of Task 1 were run on the old code, where they do not compile (the expected failure below). The switch was rendered off screen in its four states (Read-write, Read-only, production, a read-only connection) in the three looks, light and dark, and compared by eye with the artboards. The diffs below are that draft. The PostgreSQL and MySQL suites did not run (no servers here); nothing in the change reaches them. Nothing was compiled for macOS or Windows; no line of it is platform code. The real window was not opened.

**State of the tree:** the draft is still applied, uncommitted. It could not be set aside: discarding uncommitted changes is not permitted in the session that wrote this plan. Executing the plan from that tree means checking each task's diff against what is there (`git diff -- <files>`), running its checks and committing; Step 2 of Task 1 (seeing the tests fail) cannot be run there, and its result below was recorded while the plan was written. On a clean tree the diffs apply in order and every step runs.

---

## Decisions

| Question | Decision |
|---|---|
| macOS and Windows | The Components' segmented control: a sunken track (2 pt round its segments), the chosen segment raised (a 1 pt shadow). The corners follow the look's own: 7 and 5 on macOS, as the design's, 3 and 1 on Windows, which has no artboard. 26 pt tall, where the badge sat, before Limit and Timeout. |
| The chosen segment | Read-only: a 10 pt lock, then the word, in the text colour. Read-write: the word in the warning colour, no mark (the pencil goes). Both a weight heavier than the segment not chosen, which reads in the secondary colour. |
| The words | "Read-only" and "Read-write". "Transaction" goes from the switch. |
| Where no editor can write | The whole switch at 45 %, Read-only chosen, as the artboard draws the control beside it that does not apply ("Applies when the tab allows writes"). Its Read-write segment cannot be picked, takes the keyboard and says why: to a screen reader, while the keyboard is on it, and on hover. That is a read-only connection (an inert pill until now) and production (a disabled choice of the menu until now). |
| Hover | A segment that can be picked says what its mode does. Where no editor can write, the chosen Read-only adds why not (the badge's sentences, unchanged), and the Read-write segment says only why it cannot be picked. |
| Keyboard | One Tab stop, on the chosen segment; with the keyboard on the switch the arrows choose, as in the app's other segmented controls. `Mod+Shift+M` as before. Where no editor can write the one stop is the Read-write segment, which has the reason to read; the chosen Read-only takes no keyboard there, so Tab does not say the mode in use (a screen reader's own keys still read both segments). |
| Omarchy | The Components' segmented control: the chosen segment in a 1 pt box, the other muted, 6 apart, before `· limit 1000 · timeout 30s`. `read-write` in the warning colour when chosen. |
| Screen readers | Two buttons, "Read-only" and "Read-write", each with whether it is chosen, as the app's other segmented controls. The combo box "Transaction" goes. |
| When the toolbar is narrow | Unchanged: the switch gives way where the badge did. |

Not built, though the same artboards draw it: the "Auto-commit" or "Transaction" control beside the switch, the bar of an open transaction, Commit and Rollback, and everything on `MacSqlTxStates`.

Where this plan departs from the artboards:

- The key. `MacSql` writes `⌥⌘W` in the switch's tooltip and `OmarchySql` writes `ctrl+w read-write` beside it. The key stays `Mod+Shift+M` (`ctrl+w` closes the tab here), and no key is written beside the switch on Omarchy: the shortcuts table says it.
- The footer. `MacSql` reads "Read-only · rolled back"; the app's footer keeps "Read-only transaction · rolled back". The footer is not the switch.
- `widgets::segmented` is not used. It lays itself out in the flow with cells of one size; the toolbar places each piece in a rectangle it worked out, and the chosen Read-only is wider by its lock. `data_view.rs` draws its own for the same reason.

## Rules of the repository that bite here

- No em dashes anywhere, in code, comments, tests or docs.
- A view names no font and no size: text goes through `TextRole`s. The switch reads in `TextRole::Secondary`, the chosen segment in `TextRole::FormLabel` (the same size, a weight heavier), and in `TextRole::OBody` on Omarchy.
- A view paints no focus ring: `focus::hint` says where it goes.
- No test compares a screen with the design. The scenes of `src/shots.rs` are for the eye.
- One topic per commit. Every commit passes all four checks:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: no output from `fmt`, `Finished` from `clippy` and `doc`, and every `test result:` line `ok` with `0 failed`. Run them with `CARGO_TARGET_DIR` at this worktree's `target/`.

## Files

- `src/ui/sql_editor.rs`: the switch, and its tests (Task 1).
- `src/ui/mod.rs`: three tests of the toolbar that named the badge, and one new test of the switch's Tab stop (Task 1).
- `src/shots.rs`: the scenes of the switch (Task 1).
- `README.md`, `docs/superpowers/specs/2026-09-30-sql-editor-core-design.md`, `docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md`, and comments in `src/app.rs` and `src/model.rs`: the words (Task 2).

---

### Task 1: The switch

**Files:**
- Modify: `src/ui/sql_editor.rs` (`badge`, `badge_width`, `badge_tip`, `badge_switches`, `TRANSACTION`, `mode_name`, the two toolbars, and the tests of the badge)
- Modify: `src/ui/mod.rs` (`a_sql_tab_shows_its_toolbar_in_every_look`, `tab_walks_the_toolbar_in_the_order_it_reads`, `the_toolbar_gives_way_in_order_and_shortens_its_menus_last`, and the new `tab_stops_once_on_the_switch_of_a_connection_that_takes_writes`)
- Modify: `src/shots.rs` (the scenes `sql-write-menu` and `sql-write-menu-production`)

- [ ] **Step 1: Write the tests**

The tests of the badge become tests of the switch. They find a segment as a button by its name, read which one is chosen from its toggled state, and click it. One is new: the arrows choose inside the switch. `read_write_is_off` holds what the read-only connection and production share.

```diff
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -1236,60 +1236,96 @@
         workspace.sql_tab(id).unwrap().mode
     }
 
-    /// What the badge's menu is set to: none where the badge is no menu.
-    fn badge_value(harness: &mut Harness) -> Option<String> {
+    /// The switch's segment named `name`.
+    fn segment<'a>(tree: &'a egui::accesskit::TreeUpdate, name: &str) -> &'a egui::accesskit::Node {
+        let id = node(tree, name, Role::Button).unwrap_or_else(|| panic!("no {name}"));
+        let (_, segment) = tree.nodes.iter().find(|(node, _)| *node == id).unwrap();
+        segment
+    }
+
+    /// The segment the switch has chosen.
+    fn chosen(harness: &mut Harness) -> Vec<&'static str> {
         let tree = harness.settle();
-        let id = node(&tree, TRANSACTION, Role::ComboBox)?;
-        let (_, badge) = tree.nodes.iter().find(|(node, _)| *node == id)?;
-        badge.value().map(str::to_owned)
+        ["Read-only", "Read-write"]
+            .into_iter()
+            .filter(|name| segment(&tree, name).toggled() == Some(egui::accesskit::Toggled::True))
+            .collect()
     }
 
-    /// The colour the badge's words are painted in, as `look` writes them.
-    fn badge_color(harness: &Harness, look: &Look, mode: RunMode) -> Option<egui::Color32> {
+    /// The colour a segment's words are painted in, as `look` writes them.
+    fn segment_color(harness: &Harness, look: &Look, mode: RunMode) -> Option<egui::Color32> {
         harness.painted_color(&look.label(mode_name(mode)))
     }
 
     #[test]
-    fn the_badge_is_a_menu_that_switches_the_tabs_mode() {
+    fn the_switch_shows_both_modes_and_a_click_sets_the_tabs() {
         for look in Look::ALL {
-            // A tab of a writable connection opens in Read-write, and its
-            // badge reads in the warning colour.
+            // A tab of a writable connection opens in Read-write, and that
+            // segment reads in the warning colour.
             let (mut harness, tab, id) = editor(look, "SELECT 1");
+            assert_eq!(chosen(&mut harness), ["Read-write"], "{}", look.name);
+            let warning = Some(harness.app.palette.warning);
             assert_eq!(
-                badge_value(&mut harness).as_deref(),
-                Some("Read-write transaction"),
+                segment_color(&harness, &look, RunMode::ReadWrite),
+                warning,
                 "{}",
                 look.name
             );
-            let warning = Some(harness.app.palette.warning);
-            assert_eq!(badge_color(&harness, &look, RunMode::ReadWrite), warning);
-            harness.click(TRANSACTION);
-            harness.click("Read-only transaction");
+            harness.click("Read-only");
             assert_eq!(
                 mode_of(&harness, tab, id),
                 RunMode::ReadOnly,
                 "{}",
                 look.name
             );
+            assert_eq!(chosen(&mut harness), ["Read-only"], "{}", look.name);
+            // Neither segment is in the warning colour now.
+            for mode in MODES {
+                let color = segment_color(&harness, &look, mode);
+                assert!(color.is_some(), "{mode:?} in {}", look.name);
+                assert_ne!(color, warning, "{mode:?} in {}", look.name);
+            }
+            // Picking the mode in use changes nothing; the other one
+            // switches back.
+            harness.click("Read-only");
+            assert_eq!(mode_of(&harness, tab, id), RunMode::ReadOnly);
+            harness.click("Read-write");
             assert_eq!(
-                badge_value(&mut harness).as_deref(),
-                Some("Read-only transaction"),
+                mode_of(&harness, tab, id),
+                RunMode::ReadWrite,
                 "{}",
                 look.name
             );
-            // The menu closed on the pick, and the read-only badge is not
-            // in the warning colour.
-            assert!(!harness.has("Read-write transaction"), "{}", look.name);
-            let quiet = badge_color(&harness, &look, RunMode::ReadOnly);
-            assert!(quiet.is_some(), "{}", look.name);
-            assert_ne!(quiet, warning, "{}", look.name);
-            // Picking the mode in use changes nothing; the other one
-            // switches back.
-            harness.click(TRANSACTION);
-            harness.click("Read-only transaction");
-            assert_eq!(mode_of(&harness, tab, id), RunMode::ReadOnly);
-            harness.click(TRANSACTION);
-            harness.click("Read-write transaction");
+        }
+    }
+
+    #[test]
+    fn the_arrows_choose_in_the_switch_and_the_keyboard_follows() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = editor(look, "SELECT 1");
+            let focus = |harness: &mut Harness, name: &str| {
+                let tree = harness.settle();
+                let target = node(&tree, name, Role::Button).expect("the segment");
+                harness.frame(vec![egui::Event::AccessKitActionRequest(
+                    egui::accesskit::ActionRequest {
+                        target_tree: egui::accesskit::TreeId::ROOT,
+                        target_node: target,
+                        action: egui::accesskit::Action::Focus,
+                        data: None,
+                    },
+                )]);
+            };
+            focus(&mut harness, "Read-write");
+            harness.press(Key::ArrowLeft, Modifiers::NONE);
+            assert_eq!(
+                mode_of(&harness, tab, id),
+                RunMode::ReadOnly,
+                "{}",
+                look.name
+            );
+            // The keyboard is on the segment it chose: the next arrow
+            // goes back.
+            harness.press(Key::ArrowRight, Modifiers::NONE);
             assert_eq!(
                 mode_of(&harness, tab, id),
                 RunMode::ReadWrite,
@@ -1336,18 +1372,44 @@
         assert!(listed);
     }
 
+    /// What a hover over the segment named `name` shows.
+    fn hovered(harness: &mut Harness, name: &str) -> Vec<String> {
+        let tree = harness.settle();
+        let at = bounds(&tree, name, Role::Button).expect("the segment");
+        crate::ui::tests::hover(harness, at.center())
+    }
+
+    /// Asserts that the Read-write segment cannot be picked and says
+    /// `reason`: to a screen reader, to the keyboard while it is on it,
+    /// and on hover.
+    fn read_write_is_off(harness: &mut Harness, reason: &str, look: &Look) {
+        let tree = harness.settle();
+        let off = node(&tree, "Read-write", Role::Button).expect("the segment");
+        let segment = segment(&tree, "Read-write");
+        assert!(segment.is_disabled(), "{}", look.name);
+        assert_eq!(segment.description(), Some(reason), "{}", look.name);
+        assert!(!harness.has(reason), "{}", look.name);
+        harness.frame(vec![egui::Event::AccessKitActionRequest(
+            egui::accesskit::ActionRequest {
+                target_tree: egui::accesskit::TreeId::ROOT,
+                target_node: off,
+                action: egui::accesskit::Action::Focus,
+                data: None,
+            },
+        )]);
+        assert!(harness.has(reason), "{}", look.name);
+        harness.press(Key::Escape, Modifiers::NONE);
+        let shown = hovered(harness, "Read-write");
+        assert!(shown.iter().any(|label| label == reason), "{}", look.name);
+    }
+
     #[test]
-    fn on_a_read_only_connection_the_badge_is_a_note_and_the_key_does_nothing() {
+    fn on_a_read_only_connection_the_switch_is_off_and_the_key_does_nothing() {
         for look in Look::ALL {
             let (mut harness, tab, id) = read_only_editor(look);
-            // As it always was: a label, and no menu.
-            assert_eq!(badge_value(&mut harness), None, "{}", look.name);
-            let tree = harness.settle();
-            assert!(
-                node(&tree, "Read-only transaction", Role::Label).is_some(),
-                "{}",
-                look.name
-            );
+            assert_eq!(chosen(&mut harness), ["Read-only"], "{}", look.name);
+            read_write_is_off(&mut harness, "This connection opens read-only.", &look);
+            harness.click("Read-write");
             harness.press(Key::M, Modifiers::COMMAND | Modifiers::SHIFT);
             assert_eq!(
                 own_mode(&harness, tab, id),
@@ -1355,9 +1417,8 @@
                 "{}",
                 look.name
             );
-            // On hover it says why.
-            let at = bounds(&tree, "Read-only transaction", Role::Label).unwrap();
-            let shown = crate::ui::tests::hover(&mut harness, at.center());
+            // On hover the segment in use says why it is the only one.
+            let shown = hovered(&mut harness, "Read-only");
             let tip = "Every query runs in a read-only transaction that is rolled back. This \
                        connection opens read-only.";
             assert!(shown.iter().any(|label| label == tip), "{}", look.name);
@@ -1365,67 +1426,47 @@
     }
 
     #[test]
-    fn the_badge_says_on_hover_what_its_mode_does() {
+    fn a_segment_says_on_hover_what_its_mode_does() {
+        let tips = [
+            ("Read-only", "Runs are rolled back. Nothing is changed."),
+            (
+                "Read-write",
+                "A run that changes data is committed when every statement succeeds.",
+            ),
+        ];
         for look in Look::ALL {
             let (mut harness, tab, id) = editor(look, "SELECT 1");
-            for (mode, tip) in [
-                (
-                    RunMode::ReadOnly,
-                    "Runs are rolled back. Nothing is changed.",
-                ),
-                (
-                    RunMode::ReadWrite,
-                    "A run that changes data is committed when every statement succeeds.",
-                ),
-            ] {
+            // The chosen segment and the other one both say it.
+            for mode in MODES {
                 set_mode(&mut harness, tab, id, mode);
-                let tree = harness.settle();
-                let at = bounds(&tree, TRANSACTION, Role::ComboBox).unwrap();
-                let shown = crate::ui::tests::hover(&mut harness, at.center());
-                assert!(
-                    shown.iter().any(|label| label == tip),
-                    "{tip} in {}",
-                    look.name
-                );
+                for (name, tip) in tips {
+                    let shown = hovered(&mut harness, name);
+                    assert!(
+                        shown.iter().any(|label| label == tip),
+                        "{name} in {mode:?} in {}",
+                        look.name
+                    );
+                }
             }
         }
     }
 
     #[test]
-    fn on_production_the_read_write_choice_is_shown_and_cannot_be_picked() {
+    fn on_production_the_read_write_segment_is_shown_and_cannot_be_picked() {
         for look in Look::ALL {
             let (mut harness, tab, id) = editor(look, "SELECT 1");
             // The tab an editor opened on production is: in Read-only.
             set_mode(&mut harness, tab, id, RunMode::ReadOnly);
             harness.app.workspace_mut(tab).unwrap().environment =
                 crate::env::Environment::Production;
-            harness.click(TRANSACTION);
-            let tree = harness.settle();
-            let off = node(&tree, "Read-write transaction", Role::Button).expect("the choice");
-            let (_, choice) = tree.nodes.iter().find(|(node, _)| *node == off).unwrap();
-            assert!(choice.is_disabled(), "{}", look.name);
-            // A screen reader is told why, and so is the keyboard: the
-            // choice takes it, as a button that cannot be pressed does, and
-            // says its reason while it has it.
-            assert_eq!(choice.description(), Some(UNCONFIRMED), "{}", look.name);
-            assert!(!harness.has(UNCONFIRMED), "{}", look.name);
-            harness.frame(vec![egui::Event::AccessKitActionRequest(
-                egui::accesskit::ActionRequest {
-                    target_tree: egui::accesskit::TreeId::ROOT,
-                    target_node: off,
-                    action: egui::accesskit::Action::Focus,
-                    data: None,
-                },
-            )]);
-            assert!(harness.has(UNCONFIRMED), "{}", look.name);
-            harness.click("Read-write transaction");
+            read_write_is_off(&mut harness, UNCONFIRMED, &look);
+            harness.click("Read-write");
             assert_eq!(
                 own_mode(&harness, tab, id),
                 RunMode::ReadOnly,
                 "{}",
                 look.name
             );
-            harness.press(Key::Escape, Modifiers::NONE);
             harness.press(Key::M, Modifiers::COMMAND | Modifiers::SHIFT);
             assert_eq!(
                 own_mode(&harness, tab, id),
@@ -1433,10 +1474,8 @@
                 "{}",
                 look.name
             );
-            // The badge says why on hover.
-            let tree = harness.settle();
-            let at = bounds(&tree, TRANSACTION, Role::ComboBox).unwrap();
-            let shown = crate::ui::tests::hover(&mut harness, at.center());
+            // The segment in use says why on hover.
+            let shown = hovered(&mut harness, "Read-only");
             let tip = format!("Runs are rolled back. Nothing is changed. {UNCONFIRMED}");
             assert!(shown.contains(&tip), "{}", look.name);
         }
```

The three toolbar tests of `src/ui/mod.rs` are on a connection that opens read-only. There the Read-write segment takes the keyboard, so Tab meets it between Format and Limit; and the note the toolbar gives way with is read by the segment only the switch paints ("Read-only" is also the connection's pill). A new test walks the toolbar of a writable connection: the switch is one stop there, on its chosen segment.

```diff
diff --git i/src/ui/mod.rs w/src/ui/mod.rs
index 9d18eb5..bd3b739 100644
--- i/src/ui/mod.rs
+++ w/src/ui/mod.rs
@@ -4589,15 +4589,18 @@ mod tests {
                     look.name
                 );
             }
-            assert!(
-                harness.has("Read-only transaction"),
-                "the transaction note in {}",
-                look.name
-            );
+            // The switch's two segments.
+            for name in ["Read-only", "Read-write"] {
+                assert!(
+                    crate::testing::node(&tree, name, egui::accesskit::Role::Button).is_some(),
+                    "{name} in {}",
+                    look.name
+                );
+            }
             let reads = if look.terminal {
-                ["limit 1000", "timeout 30s", "read-only transaction"]
+                ["limit 1000", "timeout 30s", "read-write"]
             } else {
-                ["Limit 1,000", "Timeout 30 s", "Read-only transaction"]
+                ["Limit 1,000", "Timeout 30 s", "Read-write"]
             };
             for text in reads {
                 assert!(painted(&harness, text), "{text} in {}", look.name);
@@ -4750,11 +4753,13 @@ mod tests {
         for look in crate::theme::Look::ALL {
             let (mut harness, _tab) = sql_harness(look);
             // The terminal's toolbar leads with its menus, the others'
-            // with Run.
+            // with Run. On this connection, which opens read-only, the
+            // switch's Read-write segment cannot be picked: the keyboard
+            // stops on it to read why.
             let order: &[&str] = if look.terminal {
                 &["Limit", "Timeout", "Run", "Run all"]
             } else {
-                &["Run", "Run all", "Format", "Limit", "Timeout"]
+                &["Run", "Run all", "Format", "Read-write", "Limit", "Timeout"]
             };
             let role = if look.terminal {
                 egui::accesskit::Role::ComboBox
@@ -4771,6 +4776,31 @@ mod tests {
         }
     }
 
+    #[test]
+    fn tab_stops_once_on_the_switch_of_a_connection_that_takes_writes() {
+        for look in crate::theme::Look::ALL {
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            let tab = harness.connect_fake_as(false);
+            harness.app.apply(crate::model::Action::NewSqlTab(tab));
+            // The switch is one stop, on the segment it has chosen: the
+            // arrows choose inside it. The terminal's toolbar leads with
+            // it, the others' meet it after Format.
+            let order: &[&str] = if look.terminal {
+                &["Read-write", "Limit", "Timeout", "Run"]
+            } else {
+                &["Format", "Read-write", "Limit", "Timeout"]
+            };
+            focus(&mut harness, order[0], egui::accesskit::Role::Button);
+            let mut reached = vec![focused_name(&harness.settle())];
+            for _ in 1..order.len() {
+                harness.press(Key::Tab, Modifiers::NONE);
+                reached.push(focused_name(&harness.settle()));
+            }
+            assert_eq!(reached, order, "{}", look.name);
+        }
+    }
+
     #[test]
     fn the_terminal_strip_keeps_its_row_panel_toggle_on_a_sql_tab() {
         let (mut harness, tab) = sql_harness(crate::theme::Look::omarchy());
@@ -6178,17 +6208,13 @@ mod tests {
         // left to give way.
         for look in crate::theme::Look::ALL {
             let (mut harness, _tab) = sql_harness(look);
+            // The note is the switch, read by the segment only it paints.
             let (keys, note, full, short) = if look.terminal {
-                ("ctrl+enter", "read-only transaction", "limit 1000", "1000")
+                ("ctrl+enter", "read-write", "limit 1000", "1000")
             } else if look == crate::theme::Look::macos() {
-                ("⌘↩", "Read-only transaction", "Limit 1,000", "1,000")
+                ("⌘↩", "Read-write", "Limit 1,000", "1,000")
             } else {
-                (
-                    "Ctrl+Enter",
-                    "Read-only transaction",
-                    "Limit 1,000",
-                    "1,000",
-                )
+                ("Ctrl+Enter", "Read-write", "Limit 1,000", "1,000")
             };
             // Format's key, which goes when the run buttons' keys do.
             let format_keys = if look == crate::theme::Look::macos() {
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib --no-run`

Expected: the test build fails with two errors, both ``error[E0425]: cannot find value `MODES` in this scope`` in `src/ui/sql_editor.rs`.

- [ ] **Step 3: Draw the switch**

`switch_cells` measures the two segments, `switch_width` the control, and `switch` draws it in the rectangle the toolbar gives it. `track_corner` and `segment_corner` take the corners from the look. A segment that cannot be picked is `Sense::focusable_noninteractive()` with its reason as the node's description, as a menu's disabled row is (`src/ui/menu.rs`). On Omarchy the switch gets the height of the menus beside it.

```diff
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -3,8 +3,8 @@
 //! results (`sql_results`) draw their own panes.
 
 use egui::{
-    CornerRadius, Frame, Id, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2,
-    vec2,
+    Color32, CornerRadius, Frame, Id, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType,
+    pos2, vec2,
 };
 
 use crate::app::App;
@@ -13,9 +13,9 @@
 use crate::settings::Settings;
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
+use crate::ui::focus;
 use crate::ui::format;
 use crate::ui::menu;
-use crate::ui::states;
 use crate::ui::widgets::{self, ButtonSpec};
 
 /// Left and right padding of the toolbar.
@@ -286,7 +286,7 @@
     writing: bool,
     /// The transaction the editor's runs are in now.
     mode: RunMode,
-    /// Whether the badge switches that, or why the editor cannot write.
+    /// Whether the switch changes that, or why the editor cannot write.
     writes: Result<(), NoWrites>,
     locale: Locale,
     look: &'a Look,
@@ -447,180 +447,244 @@
     }
 }
 
-/// What the badge's menu is named for screen readers: its value is the
-/// mode.
-const TRANSACTION: &str = "Transaction";
-
 /// Why a tab of a writable connection to production cannot be switched to
 /// Read-write: its runs are to be confirmed first, with their statements
 /// on screen, and that question is not built yet.
 pub(super) const UNCONFIRMED: &str =
     "Read-write runs on a production connection are not available yet.";
 
-/// A mode as the badge names it.
+/// The switch's two segments, in the order it draws them.
+const MODES: [RunMode; 2] = [RunMode::ReadOnly, RunMode::ReadWrite];
+
+/// A mode as its segment of the switch names it.
 fn mode_name(mode: RunMode) -> &'static str {
     match mode {
-        RunMode::ReadOnly => "Read-only transaction",
-        RunMode::ReadWrite => "Read-write transaction",
+        RunMode::ReadOnly => "Read-only",
+        RunMode::ReadWrite => "Read-write",
+    }
+}
+
+/// What a mode does, as its segment says on hover.
+fn mode_does(mode: RunMode) -> &'static str {
+    match mode {
+        RunMode::ReadOnly => "Runs are rolled back. Nothing is changed.",
+        RunMode::ReadWrite => "A run that changes data is committed when every statement succeeds.",
+    }
+}
+
+/// Why no editor of the connection can write, as the Read-write segment
+/// says where it cannot be picked.
+fn no_writes(no: NoWrites) -> &'static str {
+    match no {
+        NoWrites::ReadOnlyConnection => "This connection opens read-only.",
+        NoWrites::Unconfirmed => UNCONFIRMED,
     }
 }
 
-/// What the badge says on hover: what the mode does, and on a connection
-/// whose editors cannot write, why not.
-fn badge_tip(bar: &Bar<'_>) -> String {
+/// What the segment of the mode in use says on hover: what the mode does,
+/// and on a connection whose editors cannot write, why not.
+fn switch_tip(bar: &Bar<'_>) -> String {
     let say = |text: &'static str| gettext(bar.locale, text);
-    let does = match bar.mode {
-        RunMode::ReadOnly => say("Runs are rolled back. Nothing is changed."),
-        RunMode::ReadWrite => {
-            say("A run that changes data is committed when every statement succeeds.")
-        }
-    };
+    let does = say(mode_does(bar.mode));
     match bar.writes {
         Ok(()) => does.into_owned(),
-        Err(NoWrites::ReadOnlyConnection) => format!(
+        Err(no @ NoWrites::ReadOnlyConnection) => format!(
             "{}. {}",
             say("Every query runs in a read-only transaction that is rolled back"),
-            say("This connection opens read-only.")
+            say(no_writes(no))
         ),
-        Err(NoWrites::Unconfirmed) => format!("{does} {}", say(UNCONFIRMED)),
+        Err(no @ NoWrites::Unconfirmed) => format!("{does} {}", say(no_writes(no))),
     }
 }
 
-/// Whether the badge opens its menu. On a connection that opens read-only
-/// it is what it always was, a note.
-fn badge_switches(bar: &Bar<'_>) -> bool {
-    bar.writes != Err(NoWrites::ReadOnlyConnection)
+/// The role a segment of the switch reads in. macOS: the chosen one a
+/// weight heavier, as every segmented control's.
+fn switch_role(look: &Look, chosen: bool) -> TextRole {
+    match (look.terminal, chosen) {
+        (true, _) => TextRole::OBody,
+        (false, true) => TextRole::FormLabel,
+        (false, false) => TextRole::Secondary,
+    }
 }
 
-/// The badge's width. macOS: 10 at its sides, an 11 pt mark, 6, the words
-/// and, where it is a menu, 6 and a 10 pt chevron. The terminal: the
-/// words.
-fn badge_width(ui: &Ui, bar: &Bar<'_>) -> f32 {
+/// The widths of the switch's two segments. macOS: 10 at the sides of the
+/// words, and in the chosen Read-only a 10 pt lock and 5 before them. The
+/// terminal: 6 at the sides.
+fn switch_cells(ui: &Ui, bar: &Bar<'_>) -> [f32; 2] {
     let look = bar.look;
-    let words = look.label(&gettext(bar.locale, mode_name(bar.mode)));
-    let words = menu_role(look).width(ui.ctx(), look.faces, &words);
-    if look.terminal {
-        return words;
+    MODES.map(|mode| {
+        let chosen = mode == bar.mode;
+        let words = look.label(&gettext(bar.locale, mode_name(mode)));
+        let words = switch_role(look, chosen).width(ui.ctx(), look.faces, &words);
+        if look.terminal {
+            return 6.0 + words + 6.0;
+        }
+        let lock = if chosen && mode == RunMode::ReadOnly {
+            10.0 + 5.0
+        } else {
+            0.0
+        };
+        (10.0 + lock + words + 10.0).ceil()
+    })
+}
+
+/// The switch's width. macOS: a 2 pt track round its segments. The
+/// terminal: the segments, 6 apart.
+fn switch_width(ui: &Ui, bar: &Bar<'_>) -> f32 {
+    let cells: f32 = switch_cells(ui, bar).iter().sum();
+    if bar.look.terminal {
+        cells + 6.0
+    } else {
+        2.0 + cells + 2.0
     }
-    let chevron = if badge_switches(bar) { 6.0 + 10.0 } else { 0.0 };
-    10.0 + 11.0 + 6.0 + words + chevron + 10.0
 }
 
-/// The badge in `rect`: the transaction the editor's runs are in, and on
-/// a connection that takes writes the menu that switches it. In
-/// Read-write it reads in the warning tone. Returns the mode picked from
-/// the menu this frame, when it is another than the one in use.
-fn badge(ui: &mut Ui, rect: Rect, bar: &Bar<'_>) -> Option<RunMode> {
+/// The corners of the switch's track and of a segment in it, from the
+/// look's own: 7 and 5 on macOS, as the design's segmented control.
+fn track_corner(look: &Look) -> CornerRadius {
+    CornerRadius::same(look.radius.saturating_sub(1))
+}
+
+fn segment_corner(look: &Look) -> CornerRadius {
+    CornerRadius::same(look.radius.saturating_sub(3))
+}
+
+/// The switch in `rect`: Read-only and Read-write, the transaction the
+/// editor's runs are in chosen. macOS: a sunken track with the chosen
+/// segment raised, Read-only behind a lock and Read-write in the warning
+/// tone. The terminal: the chosen segment in a box, the other muted.
+/// Where no editor can write it is drawn as a control that is off, and
+/// its Read-write segment says why. Returns the mode picked this frame,
+/// when it is another than the one in use.
+fn switch(ui: &mut Ui, rect: Rect, bar: &Bar<'_>) -> Option<RunMode> {
     let Bar {
         locale,
         look,
         palette,
         mode,
+        writes,
         ..
     } = *bar;
-    let name = gettext(locale, mode_name(mode));
-    let words = look.label(&name);
-    let switches = badge_switches(bar);
-    let sense = if switches {
-        Sense::click()
+    // A control that is off, as the design's: all of it at 45 %.
+    let fade = |color: Color32| match writes {
+        Ok(()) => color,
+        Err(NoWrites::ReadOnlyConnection | NoWrites::Unconfirmed) => color.gamma_multiply(0.45),
+    };
+    let painter = ui.painter().clone();
+    let (pad, gap) = if look.terminal {
+        (0.0, 6.0)
     } else {
-        Sense::hover()
+        (2.0, 0.0)
     };
-    let response = ui.interact(rect, ui.id().with("transaction-note"), sense);
-    response.widget_info(|| {
-        if switches {
-            let menu = gettext(locale, TRANSACTION);
-            let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, menu.as_ref());
-            info.current_text_value = Some(name.to_string());
-            info
-        } else {
-            WidgetInfo::labeled(WidgetType::Label, true, name.as_ref())
-        }
-    });
-    let writing = mode == RunMode::ReadWrite;
-    let lit = switches && (response.hovered() || response.has_focus());
-    let center = rect.center().y;
-    let role = menu_role(look);
-    if look.terminal {
-        // Muted words that light up under the pointer, as the menus
-        // beside them.
-        let color = match (writing, lit) {
-            (true, _) => palette.warning,
-            (false, true) => palette.text,
-            (false, false) => palette.dim,
-        };
-        widgets::paint_text(
-            ui,
-            rect.left(),
-            center,
-            Text::one(look, role, &words, color),
+    if !look.terminal {
+        painter.rect_filled(rect, track_corner(look), fade(palette.surface));
+    }
+    let group = ui.id().with("transaction-modes");
+    let mut left = rect.left() + pad;
+    let mut picked = None;
+    for (index, (choice, width)) in MODES.into_iter().zip(switch_cells(ui, bar)).enumerate() {
+        let cell = Rect::from_min_max(
+            pos2(left, rect.top() + pad),
+            pos2(left + width, rect.bottom() - pad),
         );
-    } else {
-        let tone = states::Tone::Warning;
-        let (fill, ink) = match (writing, lit) {
-            (true, _) => (tone.fill(look, palette), palette.warning),
-            (false, true) => (palette.surface_hover, palette.secondary),
-            (false, false) => (palette.surface, palette.secondary),
+        left += width + gap;
+        let chosen = choice == mode;
+        let name = gettext(locale, mode_name(choice));
+        let id = ui.id().with(("transaction-mode", index));
+        // A segment that cannot be picked answers no press, and still
+        // takes the keyboard, as a button that cannot be pressed does: the
+        // keyboard can then read why.
+        let reason = match writes {
+            Err(no) if !chosen => Some(gettext(locale, no_writes(no))),
+            _ => None,
+        };
+        let response = match (&reason, writes) {
+            (Some(_), _) => ui.interact(cell, id, Sense::focusable_noninteractive()),
+            // One Tab stop for the switch; the arrows choose inside it.
+            (None, Ok(())) => {
+                let (response, arrow) =
+                    focus::segment(ui, cell, id, group, (index, MODES.len()), chosen);
+                picked = arrow.and_then(|index| MODES.get(index).copied()).or(picked);
+                if response.clicked() {
+                    picked = Some(choice);
+                }
+                response
+            }
+            (None, Err(_)) => ui.interact(cell, id, Sense::hover()),
+        };
+        response.widget_info(|| {
+            WidgetInfo::selected(WidgetType::Button, reason.is_none(), chosen, name.as_ref())
+        });
+        if let Some(reason) = &reason {
+            ui.ctx().accesskit_node_builder(response.id, |node| {
+                node.set_description(reason.as_ref());
+            });
+        }
+        let ring = if look.terminal {
+            focus::Ring::Inset { radius: 0 }
+        } else {
+            focus::Ring::Edge {
+                radius: look.radius.saturating_sub(3),
+            }
         };
-        let corner = CornerRadius::same(13);
-        ui.painter().rect_filled(rect, corner, fill);
-        if writing {
-            ui.painter().rect_stroke(
-                rect,
-                corner,
-                Stroke::new(widgets::hairline(ui), tone.line(look, palette)),
-                StrokeKind::Inside,
+        focus::hint(ui, &response, cell, ring);
+        let lit = reason.is_none() && response.hovered();
+        if look.terminal {
+            if chosen {
+                painter.rect_stroke(
+                    cell,
+                    CornerRadius::same(3),
+                    Stroke::new(1.0, fade(palette.outline)),
+                    StrokeKind::Inside,
+                );
+            }
+        } else if chosen {
+            let corner = segment_corner(look);
+            painter.add(
+                egui::epaint::Shadow {
+                    offset: [0, 1],
+                    blur: 2,
+                    spread: 0,
+                    color: fade(Color32::from_black_alpha(26)),
+                }
+                .as_shape(cell, corner),
             );
+            painter.rect_filled(cell, corner, fade(palette.window));
+        } else if lit {
+            painter.rect_filled(cell, segment_corner(look), palette.surface_hover);
         }
-        let mark = if writing { Icon::Pencil } else { Icon::Lock };
-        mark.image(ink, 11.0).paint_at(
-            ui,
-            Rect::from_center_size(pos2(rect.left() + 15.5, center), vec2(11.0, 11.0)),
-        );
-        widgets::paint_text(
-            ui,
-            rect.left() + 27.0,
-            center,
-            Text::one(look, role, &words, ink),
-        );
-        if switches {
-            Icon::ChevronDown.image(ink, 10.0).paint_at(
+        let ink = fade(match (chosen, choice) {
+            (true, RunMode::ReadWrite) => palette.warning,
+            (true, RunMode::ReadOnly) => palette.text,
+            (false, _) if look.terminal && lit => palette.text,
+            (false, _) if look.terminal => palette.dim,
+            (false, _) => palette.secondary,
+        });
+        let words = look.label(&name);
+        let text = Text::one(look, switch_role(look, chosen), &words, ink);
+        let center = cell.center().y;
+        if !look.terminal && chosen && choice == RunMode::ReadOnly {
+            Icon::Lock.image(ink, 10.0).paint_at(
                 ui,
-                Rect::from_center_size(pos2(rect.right() - 15.0, center), vec2(10.0, 10.0)),
+                Rect::from_center_size(pos2(cell.left() + 15.0, center), vec2(10.0, 10.0)),
             );
+            widgets::paint_text(ui, cell.left() + 25.0, center, text);
+        } else {
+            text.layout(ui.ctx()).paint_center(&painter, cell.center());
         }
-    }
-    let response = response.on_hover_text(badge_tip(bar));
-    if !switches {
-        return None;
-    }
-    let radius = if look.terminal { 0 } else { 13 };
-    crate::ui::focus::hint(
-        ui,
-        &response,
-        rect,
-        crate::ui::focus::Ring::Outer { radius },
-    );
-    let unconfirmed = bar.writes == Err(NoWrites::Unconfirmed);
-    let modes = [RunMode::ReadOnly, RunMode::ReadWrite];
-    let picked = menu::choices(&response, rect.width(), look, palette, || {
-        modes
-            .iter()
-            .map(|choice| {
-                let name = gettext(locale, mode_name(*choice));
-                let off = unconfirmed && *choice == RunMode::ReadWrite;
-                menu::Choice {
-                    text: look.label(&name),
-                    name: Some(name.into_owned()),
-                    selected: *choice == mode,
-                    disabled: off.then(|| gettext(locale, UNCONFIRMED).into_owned()),
+        let tip = match &reason {
+            Some(reason) => {
+                if focus::shown(&response) {
+                    response.show_tooltip_text(reason.as_ref());
                 }
-            })
-            .collect()
-    });
-    picked
-        .and_then(|index| modes.get(index).copied())
-        .filter(|picked| *picked != mode)
+                reason.to_string()
+            }
+            None if chosen => switch_tip(bar),
+            None => gettext(locale, mode_does(choice)).into_owned(),
+        };
+        let _ = response.on_hover_text(tip);
+    }
+    picked.filter(|picked| *picked != mode)
 }
 
 /// Run or Run all in `rect`, with what it does on hover. Neither can be
@@ -660,7 +724,7 @@
 }
 
 /// macOS: Run and Run all, a divider and Format; at the right the
-/// transaction badge, then the Limit and Timeout menus.
+/// Read-only or Read-write switch, then the Limit and Timeout menus.
 fn mac_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Action>) {
     let Bar {
         locale,
@@ -698,10 +762,10 @@
             [run, all, format]
         }
     };
-    let note_width = badge_width(ui, bar);
+    let note_width = switch_width(ui, bar);
     // Everything 8 apart, and 16 between the two ends (8 at the tightest).
     // What gives way as the room runs out: the buttons' keys (the help
-    // says them too), then the note, then Format (its key formats too),
+    // says them too), then the switch, then Format (its key formats too),
     // and only then the menus' words, which alone say what "1,000" and
     // "30 s" are; then the menus' chevrons, and last the menus (the run
     // buttons stay).
@@ -713,16 +777,16 @@
     let room = right - left;
     // The divider before Format: a rule 20 tall with 4 at its sides.
     let divider = 4.0 + 1.0 + 4.0;
-    let needs = |shape: usize, badge: bool, keys: bool, format: bool| {
+    let needs = |shape: usize, switch: bool, keys: bool, format: bool| {
         let [run, all, format_width] = widths[usize::from(keys)];
         let format = if format {
             8.0 + divider + 8.0 + format_width
         } else {
             0.0
         };
-        let badge = if badge { note_width + 8.0 } else { 0.0 };
+        let switch = if switch { note_width + 8.0 } else { 0.0 };
         let between = if shapes[shape].chevron { 16.0 } else { 8.0 };
-        run + 8.0 + all + format + between + badge + menu_sizes[shape].iter().sum::<f32>() + 8.0
+        run + 8.0 + all + format + between + switch + menu_sizes[shape].iter().sum::<f32>() + 8.0
     };
     let fit = [
         (0, true, true, true),
@@ -733,7 +797,7 @@
         (2, false, false, false),
     ]
     .into_iter()
-    .find(|(shape, badge, keys, format)| needs(*shape, *badge, *keys, *format) <= room);
+    .find(|(shape, switch, keys, format)| needs(*shape, *switch, *keys, *format) <= room);
     let keys = fit.is_some_and(|(_, _, keys, _)| keys);
     // Where each piece sits, then the pieces from left to right: the Tab
     // key and screen readers meet them in the order they are made.
@@ -744,7 +808,7 @@
     let all_place = place(run_place.right() + 8.0, all_width);
     run_button(ui, run, run_place, false, bar, actions);
     run_button(ui, all, all_place, true, bar, actions);
-    let Some((shape, badge, _, formats)) = fit else {
+    let Some((shape, switched, _, formats)) = fit else {
         return;
     };
     if formats {
@@ -774,12 +838,12 @@
     };
     let timeout = menu_rect(right, timeout_width);
     let limit = menu_rect(timeout.left() - 8.0, limit_width);
-    if badge {
+    if switched {
         let pill = Rect::from_min_size(
             pos2(limit.left() - 8.0 - note_width, center - 13.0),
             vec2(note_width, 26.0),
         );
-        if let Some(mode) = self::badge(ui, pill, bar) {
+        if let Some(mode) = switch(ui, pill, bar) {
             actions.push(Action::SetSqlMode {
                 tab: bar.tab,
                 sql_tab: bar.id,
@@ -790,10 +854,9 @@
     menus(ui, [limit, timeout], shape, bar, actions);
 }
 
-/// Omarchy: the tab's title and a muted `read-only transaction · limit
-/// 1000 · timeout 30s`, each part of which opens its menu (the first only
-/// on a connection that takes writes); at the right `run` and `run all`
-/// with their keys.
+/// Omarchy: the tab's title, the `read-only` or `read-write` switch and a
+/// muted `· limit 1000 · timeout 30s`, each part of which opens its menu;
+/// at the right `run` and `run all` with their keys.
 fn terminal_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Action>) {
     let Bar {
         locale,
@@ -826,11 +889,11 @@
     let width = |role: TextRole, text: &str| role.width(ui.ctx(), look.faces, text);
     let (title_width, note_width, dot) = (
         width(TextRole::OTableTitle, bar.title),
-        badge_width(ui, bar),
+        switch_width(ui, bar),
         width(role, " · "),
     );
     // Everything 14 apart. What gives way as the room runs out: the
-    // buttons' keys (the status line says them too), then the note, then
+    // buttons' keys (the status line says them too), then the switch, then
     // the title (the tab says it too), and only then the menus' words,
     // which alone say what "1000" and "30s" are; last the menus (the
     // buttons stay).
@@ -880,8 +943,11 @@
         }
         let line = role.row_height(ui.ctx(), look.faces);
         if noted {
-            let words = Rect::from_min_size(pos2(x, center - line / 2.0), vec2(note_width, line));
-            if let Some(mode) = badge(ui, words, bar) {
+            let words = Rect::from_min_size(
+                pos2(x, center - line / 2.0 - 2.0),
+                vec2(note_width, line + 4.0),
+            );
+            if let Some(mode) = switch(ui, words, bar) {
                 actions.push(Action::SetSqlMode {
                     tab: bar.tab,
                     sql_tab: bar.id,
@@ -891,7 +957,7 @@
             x += note_width;
             x += widgets::paint_text(ui, x, center, dot_text());
         }
-        // The menus read as one line with the note: their words, a dot
+        // The menus read as one line with the switch: their words, a dot
         // apart.
         let [limit_width, timeout_width] = menu_sizes[shape];
         let shape = shapes[shape];
```

- [ ] **Step 4: Give the scenes the switch**

The menu the two scenes opened is gone. Three scenes show the switch instead.

```diff
diff --git i/src/shots.rs w/src/shots.rs
index e24d7b7..36c9702 100644
--- i/src/shots.rs
+++ w/src/shots.rs
@@ -735,18 +735,22 @@ fn shots() {
         let refused = crate::testing::refused_write();
         harness.answer_sql(Ok(crate::testing::script_outcome(vec![refused])), None);
     });
-    // A tab in Read-write: the badge's menu, a run that writes on
-    // its way (Run and Run all wait for it), and how such a run ends.
-    both("sql-write-menu", |harness| {
-        let tab = sql_editor(harness);
-        sql_mode(harness, tab, RunMode::ReadWrite);
-        harness.click("Transaction");
+    // The switch of a tab in Read-write and of one in Read-only, a run
+    // that writes on its way (Run and Run all wait for it), and how such
+    // a run ends.
+    both("sql-write-switch", |harness| {
+        sql_editor(harness);
     });
-    // On production the choice is shown and cannot be picked yet.
-    both("sql-write-menu-production", |harness| {
+    both("sql-write-switch-read-only", |harness| {
         let tab = sql_editor(harness);
+        sql_mode(harness, tab, RunMode::ReadOnly);
+    });
+    // On production the Read-write segment is shown and cannot be picked
+    // yet.
+    both("sql-write-switch-production", |harness| {
+        let tab = sql_editor(harness);
+        sql_mode(harness, tab, RunMode::ReadOnly);
         harness.app.workspace_mut(tab).unwrap().environment = Environment::Production;
-        harness.click("Transaction");
     });
     both("sql-write-running", |harness| {
         writing(harness, WRITES);
@@ -1071,7 +1075,7 @@ const UPDATE: &str = "UPDATE book_images\n   SET kind = 'ebook'\n WHERE id = 2;"
 const WRITES: &str = "UPDATE book_images\n   SET kind = 'ebook'\n WHERE kind = 'epub';\n\n\
                       DELETE FROM book_images\n WHERE book_id IS NULL;";
 
-/// Switches the SQL editor on screen to `mode`, as its badge does.
+/// Sets the SQL editor on screen to `mode`, as its switch does.
 fn sql_mode(harness: &mut Harness, tab: ConnTabId, mode: RunMode) {
     let sql_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
     harness.app.apply(Action::SetSqlMode { tab, sql_tab, mode });
```

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected: `test result: ok. 1745 passed; 0 failed; 1 ignored`.

- [ ] **Step 6: Look at it**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots::shots -- --ignored`

Read `target/shots/sql-write-switch-*.png`, `sql-write-switch-read-only-*.png`, `sql-write-switch-production-*.png` and `sql-blocked-connection-*.png` beside `MacSql`, `MacSqlTx`, `OmarchySql` and the Components' "Segmented" row. Expected: the raised segment, the lock before a chosen Read-only, Read-write in amber, and the whole switch faded on production and on a read-only connection. The pictures stay in `target/`.

- [ ] **Step 7: Run the four checks and commit**

```bash
git add src/ui/sql_editor.rs src/ui/mod.rs src/shots.rs
git commit -m "Draw the SQL editor's mode as a segmented switch" -m "The design canvas now draws Read-only and Read-write as the two segments of one control, where the toolbar had a badge that opened a menu. The switch does what the badge did: a click sets the tab's mode, and where no editor can write its Read-write segment cannot be picked and says why. It gains the arrows, as the app's other segmented controls have them, and each segment says on hover what its mode does." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 2: The words

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-09-30-sql-editor-core-design.md` ("Toolbar")
- Modify: `docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md` (its status, "The tab's mode", "Toolbar and keys", "Steps", "Testing")
- Modify: `src/app.rs`, `src/model.rs` (comments and two test names that say "badge")

- [ ] **Step 1: Say "switch" where the documents and comments say "badge"**

The writes spec also lists what of the artboards is not built.

```diff
diff --git i/README.md w/README.md
index 3127603..ccbbea8 100644
--- i/README.md
+++ w/README.md
@@ -68,8 +68,8 @@ Linux (Omarchy and Hyprland first), macOS and Windows.
   a tab opens in Read-write: a run that changes data is one transaction,
   committed when every statement succeeded and rolled back on the first
   error, cancel or timeout, and the Messages say which. A run of reads is
-  read-only there too, and the tab's badge (or Cmd/Ctrl+Shift+M) switches
-  it to Read-only and back.
+  read-only there too, and the toolbar's switch (or Cmd/Ctrl+Shift+M)
+  sets the tab to Read-only and back.
   Format (Cmd/Ctrl+Shift+F)
   lays queries out in river style and uppercases reserved words, in the
   selection's statements or the whole script. Keywords, schemas, tables,
diff --git i/docs/superpowers/specs/2026-09-30-sql-editor-core-design.md w/docs/superpowers/specs/2026-09-30-sql-editor-core-design.md
index 7896706..2712df9 100644
--- i/docs/superpowers/specs/2026-09-30-sql-editor-core-design.md
+++ w/docs/superpowers/specs/2026-09-30-sql-editor-core-design.md
@@ -635,17 +635,19 @@ tab's result grid as on a table's.
 ### Toolbar
 
 - macOS: Run (with `Cmd+Return`), Run all (`Shift+Cmd+Return`); on the right
-  a "Read-only transaction" badge whose tooltip explains it, then
-  "Limit 1,000" and "Timeout 30 s" menus. On a connection that takes
-  writes the badge is a menu too, with "Read-only transaction" and
-  "Read-write transaction", and in Read-write it reads in the warning
-  tone.
-- Omarchy: the tab title, a muted `read-only transaction · limit 1000 ·
-  timeout 30s` whose limit and timeout parts open the same menus, then
-  `run ctrl+enter` and `run all ctrl+shift+enter`. On a connection that
-  takes writes the first part opens the badge's menu.
+  a segmented switch, "Read-only" and "Read-write", whose segments say on
+  hover what each mode does, then "Limit 1,000" and "Timeout 30 s" menus.
+  The chosen Read-only stands behind a lock, and the chosen Read-write
+  reads in the warning tone. Where no editor of the connection can write
+  the switch is drawn as a control that is off, and its Read-write segment
+  says why (see `2026-10-05-sql-editor-writes-design.md`, "Toolbar and
+  keys").
+- Omarchy: the tab title, the same switch as `read-only` and `read-write`
+  with the chosen one in a box and the other muted, then a muted `· limit
+  1000 · timeout 30s` whose parts open the menus, then `run ctrl+enter`
+  and `run all ctrl+shift+enter`.
 - Where the toolbar is too narrow, pieces give way in this order: the run
-  buttons' keys, the read-only note, on Omarchy the tab title, then the
+  buttons' keys, the switch, on Omarchy the tab title, then the
   menus' words (leaving "1,000" and "30 s"), on macOS the menus' chevrons,
   and last the menus. The run buttons stay.
 - Explain is absent until its slice, not disabled. Format, its button
diff --git i/docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md w/docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md
index 1abfed5..9c05cc0 100644
--- i/docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md
+++ w/docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md
@@ -9,7 +9,9 @@ planned, is the production confirmation and `editor.sql_new_tab`. Step 3
 is not yet planned. Since 2026-10-06 a new tab follows its connection: it
 opens in Read-write where its editors can write, which the first draft
 left to `editor.sql_new_tab` and off by default. On production it opens
-in Read-only all the same. The PostgreSQL and MySQL runs were built with no
+in Read-only all the same. Since the same day the toolbar's badge and its
+menu are a segmented switch, as the design canvas now draws it ("Toolbar
+and keys"). The PostgreSQL and MySQL runs were built with no
 server at hand: their tests have been compiled, and are first run by CI.
 
 ## Intent
@@ -31,9 +33,9 @@ This is slice 6 of editing values
 (`2026-10-03-value-editing-core-design.md`, "Editing as a whole") and it
 changes what the core editor promises (`2026-09-30-sql-editor-core-design.md`).
 
-The design canvas has no artboard for a run that writes. What it gives is
-the frame: the toolbar's "Read-only transaction" badge as a switch for the
-tab, the Editor settings "New tabs run in" and "Confirm UPDATE or DELETE
+The design canvas had no artboard for a run that writes when this was
+written. What it gave is the frame: the toolbar's "Read-only transaction"
+badge as a switch for the tab, the Editor settings "New tabs run in" and "Confirm UPDATE or DELETE
 without WHERE", and the "Write blocked" card of the "read-only connection"
 error artboards (macOS and Omarchy). They are not copied into the
 repository.
@@ -99,7 +101,7 @@ follows the connection (`RunMode::of_new_tab`): it starts `ReadWrite`
 where an editor of its workspace can write when it opens
 (`Workspace::sql_writes`), and `ReadOnly` everywhere else; with
 `editor.sql_new_tab = "read-only"` it starts `ReadOnly` everywhere. The
-user switches it with the toolbar's badge or `Mod+Shift+M`.
+user switches it with the toolbar's switch or `Mod+Shift+M`.
 
 Production is the exception: a tab there starts `ReadOnly` on a writable
 connection too, whatever `editor.sql_new_tab` says, and the user switches
@@ -121,9 +123,9 @@ before it. Until `editor.confirm_unsafe_writes` is built, that includes an
 The mode that counts is the effective one: `ReadWrite` only while the
 workspace's `access` is `Writable`. A tab whose session comes back
 read-only (the box was turned on meanwhile) runs read-only and shows the
-read-only connection's badge; its own mode is kept and counts again if the
-session is writable once more. On a read-only connection the key and the
-badge do nothing.
+read-only connection's switch; its own mode is kept and counts again if
+the session is writable once more. On a read-only connection the key and
+the switch do nothing.
 
 ### The statement's kind
 
@@ -463,21 +465,31 @@ does.
 
 ### Toolbar and keys
 
-- macOS and Windows, writable connection: the badge is a menu, as Limit and
-  Timeout are, with "Read-only transaction" and "Read-write transaction".
-  In Read-write it is drawn in the warning tone, on production in the
-  production tone. Its tooltip says what the mode does: "Runs are rolled
+- macOS and Windows, writable connection: a segmented switch, "Read-only"
+  and "Read-write", as the design's Components draw one: a sunken track
+  with the chosen segment raised. The chosen Read-only stands behind a
+  lock; the chosen Read-write reads in the warning tone. A click on a
+  segment sets the mode, and with the keyboard on the switch the arrows
+  do. Each segment says on hover what its mode does: "Runs are rolled
   back. Nothing is changed." and "A run that changes data is committed
   when every statement succeeds."
-- Read-only connection: the badge is today's, inert, and its tooltip adds
-  "This connection opens read-only."
-- Omarchy: the first part of `read-only transaction · limit 1000 · timeout
-  30s` opens the same menu and reads `read-write transaction` in the
-  warning colour, the production colour on PROD.
+- Where no editor can write (a read-only connection, and production until
+  its confirmation is built): the switch is drawn as a control that is
+  off, with Read-only chosen. Its Read-write segment cannot be picked and
+  says why, to a screen reader, to the keyboard while it is on it, and on
+  hover: "This connection opens read-only." or the sentence about
+  production.
+- Omarchy: the same two segments before `· limit 1000 · timeout 30s`, the
+  chosen one in a box and the other muted, `read-write` in the warning
+  colour when it is chosen.
 - `Mod+Shift+M` switches the mode of the active SQL tab. It is in the
   shortcuts table as "Read-only or read-write runs in the SQL editor".
-- The toolbar gives way in the core spec's order, with the badge in the
-  place of the read-only note.
+- The toolbar gives way in the core spec's order: the switch goes after
+  the run buttons' keys.
+- Not built from those artboards: the "Auto-commit" and "Transaction"
+  control beside the switch, the bar of an open transaction with Commit
+  and Rollback, and the key the Omarchy artboard writes beside the switch
+  (`ctrl+w` closes the tab here).
 
 ### Results, Messages and the footer
 
@@ -673,7 +685,7 @@ Each step ends compiling, tested and shippable, and gets its own plan run:
    the read-write run in the three drivers, `Error::LeftTransaction`. The
    backend passes `ScriptMode::ReadOnly` everywhere; nothing in the app
    writes. It needs nothing from value editing and can be built on `main`.
-2. **The tab.** The mode, the badge's menu and `Mod+Shift+M`,
+2. **The tab.** The mode, the toolbar's switch and `Mod+Shift+M`,
    `editor.sql_new_tab`, the decision in `RunSql`, the production
    confirmation in both looks, the three cards, Messages, Results and the
    footer for a read-write run, and Run held back while one is in flight.
@@ -683,9 +695,9 @@ Each step ends compiling, tested and shippable, and gets its own plan run:
    3, so it is planned once that has landed. It is built in two runs.
    The first leaves out the production confirmation and
    `editor.sql_new_tab`, and so lets no tab of a production connection
-   write: there the badge's menu shows "Read-write transaction" disabled,
-   the key does nothing, and the menu, the badge's tooltip and the card of
-   a tab in Read-only say "Read-write runs on a production connection are
+   write: there the switch's Read-write segment cannot be picked,
+   the key does nothing, and the segment, the switch's tooltip and the card
+   of a tab in Read-only say "Read-write runs on a production connection are
    not available yet." in place of the way on. So does the card of a
    production connection that opens read-only, in place of "To write, turn
    off Open read-only ..." and **Edit connection**: with the box off its
@@ -765,12 +777,12 @@ No step ships a read-write run on production without its confirmation.
   is read-only;
   stale marks and their refetch, with and without pending changes; the
   tree's refresh after DDL; the leaving guard.
-- Headless UI tests, in every look: the badge's menu and the key; the
+- Headless UI tests, in every look: the switch and the key; the
   three cards and their actions, in Messages, after a statement that
   returned rows too; the production confirmation, with
   `write` typed on Omarchy; the question about a missing `WHERE`, alone
   and inside the production sheet; Messages, Results and the footer for
-  each `ScriptEnd`; the badge on a read-only connection.
+  each `ScriptEnd`; the switch on a read-only connection.
 - Settings: an older file without the new keys loads with the defaults; an
   unknown `sql_new_tab` is an invalid line.
 - `src/shots.rs` gains scenes for review on the Bookshop demo data, whose
diff --git i/src/app.rs w/src/app.rs
index 643bdbd..c116edf 100644
--- i/src/app.rs
+++ w/src/app.rs
@@ -3182,7 +3182,7 @@ impl App {
     }
 
     /// Sets how an editor's runs end: to `mode`, or to the other one. Only
-    /// where an editor can run read-write: everywhere else the badge and
+    /// where an editor can run read-write: everywhere else the switch and
     /// the key do nothing, and a mode the tab was given before stays as it
     /// is for a session that can write again.
     fn set_sql_mode(&mut self, tab: ConnTabId, id: TabId, mode: Option<RunMode>) {
@@ -6739,7 +6739,7 @@ mod tests {
     }
 
     #[test]
-    fn the_badge_and_the_key_switch_an_editors_own_mode() {
+    fn the_switch_and_the_key_set_an_editors_own_mode() {
         let mut harness = Harness::new();
         let (tab, id) = writable_sql(&mut harness);
         set_mode(&mut harness, tab, id, RunMode::ReadOnly);
@@ -6758,7 +6758,7 @@ mod tests {
     }
 
     #[test]
-    fn the_badge_and_the_key_do_nothing_where_an_editor_cannot_write() {
+    fn the_switch_and_the_key_do_nothing_where_an_editor_cannot_write() {
         use crate::model::NoWrites;
         // A connection that opens read-only.
         let mut harness = Harness::new();
@@ -6803,7 +6803,7 @@ mod tests {
         harness.reconnect_fake_as(tab, true);
         assert_eq!(run_mode(&harness, tab, id), RunMode::ReadOnly);
         assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadWrite);
-        // Neither the badge nor the key changes what the tab was set to.
+        // Neither the switch nor the key changes what the tab was set to.
         toggle_mode(&mut harness, tab, id);
         set_mode(&mut harness, tab, id, RunMode::ReadOnly);
         assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadWrite);
diff --git i/src/model.rs w/src/model.rs
index 9238319..2a27aaa 100644
--- i/src/model.rs
+++ w/src/model.rs
@@ -367,7 +367,7 @@ pub enum Action {
         sql_tab: TabId,
         secs: Option<u32>,
     },
-    /// The badge's menu: how this editor's runs end. Nothing on a
+    /// The toolbar's switch: how this editor's runs end. Nothing on a
     /// connection whose editors cannot write (see `Workspace::sql_writes`).
     SetSqlMode {
         tab: ConnTabId,
@@ -2052,8 +2052,8 @@ pub type ShownRows<'a> = (
     bool,
 );
 
-/// How a SQL editor's runs are meant to end: what its toolbar's badge
-/// says and switches.
+/// How a SQL editor's runs are meant to end: what its toolbar's switch
+/// shows and sets.
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
 pub enum RunMode {
     /// Every run is a read-only transaction that is rolled back.
@@ -2384,7 +2384,7 @@ pub struct SqlTab {
     pub cursor: usize,
     pub limit: u32,
     pub timeout: Option<Duration>,
-    /// What the badge was set to: by `Workspace::push_sql_tab` when the
+    /// What the switch was set to: by `Workspace::push_sql_tab` when the
     /// tab opened, and by the user since. It counts only while the session
     /// can write: ask `Workspace::run_mode` for the mode a run has.
     pub mode: RunMode,
```

- [ ] **Step 2: Run the four checks and commit**

```bash
git add README.md docs/superpowers/specs src/app.rs src/model.rs
git commit -m "Say what the SQL editor's switch is and what of its artboards is not built" -m "The README, the two specs of the editor and the comments still called it a badge with a menu. The writes spec now also names what the same artboards draw and the app does not have: the Auto-commit or Transaction control, the bar of an open transaction, and the key beside the switch." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## For the next run

- The production confirmation takes `NoWrites::Unconfirmed` out. The switch then needs nothing: on production it is on, with Read-only chosen in a new tab.
- The "Auto-commit" or "Transaction" control and the open transaction it stands for are a model the writes spec rejected ("a transaction held open across runs with Commit and Rollback buttons"). Building them starts with that spec, not with the toolbar.
