# Sidebar table groups: indent, guide, tree (issue #286, part 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make an open prefix group in the sidebar read as a group: its tables one step in with a guide along them and the prefix dropped, the rows a tree for a screen reader, and the Omarchy look's vim folds.

**Architecture:** The model already groups by prefix, sorts groups and single tables together, and drops the prefix from a grouped object's label (`Tree::visible_rows`, `short_name`). This plan changes the drawing (`src/ui/sidebar.rs`: one `places` function says where a row draws its chevron, name, guide and highlight), names the rows as tree items, adds three tree keys (`TreeKey::Fold`, `FoldAll`, `OpenAll`) bound to `za`, `zM`, `zR` in the Omarchy look, and has the Omarchy status line name the table under the tree's cursor in full.

**Tech Stack:** Rust, egui (crmne fork), AccessKit, the headless `crate::testing::Harness`.

**Issue:** https://github.com/igor-alexandrov/tabletist/issues/286 (point 2). Design: canvas page "Table and editing", group "Sidebar table groups" (`MacTableGroups`, `OmarchyTableGroups`).

---

## Where the build and the boards differ

The prompt wins over a board where they disagree. Each of these goes in the PR.

| The design asks | What is built | Why |
|---|---|---|
| macOS board: grouped names at 39 px, single tables at 34 | Every table name at 34 (inset 8, chevron at 16, one 18 px step): grouped, single and the group's own name in one column | The prompt: "The table name starts where the group name starts", and single tables line up with grouped ones |
| macOS board: guide at x 20 | At the chevron's middle, 22 | The prompt: "aligned with the group's chevron" |
| "13 more" row, the guide ending on it | No such row; the guide ends on the group's last table | The tree has no "N more" row; adding one is out of scope |
| Tooltip after 500 ms | After 0.4 s | egui reads one delay for every tooltip (`Context::global_style`); 0.4 s is the app's for all of them |
| Interface scale (90 to 250%) | The indent, the guide and the cells are in points, so they scale with the window's zoom | Interface scale is not built yet; its board scales the whole window, which is egui's zoom. The terminal look's font size already zooms |
| Highlight and focus ring "start at the indent" | For a grouped table, 4 past the guide (26 in); single tables and groups keep theirs (8 in) | The macOS board draws single rows full width; only grouped rows sit inside the guide |
| Omarchy: single tables and grouped ones two cells right of the group's name | As drawn; the flat list keeps today's column | "Flat layout is unchanged" |
| Omarchy `za` on a table | Folds the table's group and puts the cursor on it | Vim's `za` folds the fold the cursor is in |
| Omarchy status line: the full name | `book_covers · table · 13 rows` while the tree has the keys; the rows only where the database has an estimate | A view has no estimate |
| Windows look | Follows macOS (18 px step, guide, inset highlight) | The prompt names macOS and Omarchy; Windows shares the macOS tree |

Open questions, with the prompt's defaults taken:

1. Single tables at the child indent: **yes**, names line up in one column.
2. Dropping the prefix inside a group: **yes**, as the boards draw it (already built).

A side change the keys need: a window letter (`R` reloads) typed after the first key of a chord (`z`) now goes to the chord (`zR`), as in vim. A count (`3`) before it does not change this.

## What exists and what changes

- `crates/tabletist-core/src/model.rs`: `group_objects` sorts groups and single objects together by name; `visible_rows` gives a grouped object `depth: 1` and its name without the prefix (`short_name`), so `book_cover_sizes` is already `cover_sizes` under `book_`, one level. No change there but the tree's keys (Task 3).
- `src/ui/sidebar.rs` draws each row. Today a grouped object's name, a single object's and a group's own name all start at the same x, so an open group's tables do not stand apart: the issue. Rows are `Button`s named with the object's full name.
- `src/testing.rs`'s `Harness::click("orders")` finds a sidebar row by that name. Rows renamed "orders, table" need it to find a tree item by the name before its kind (Task 1).

## File map

| File | Change |
|---|---|
| `crates/tabletist-core/src/fixtures.rs` | `bookshop_grouped_objects()`: the Bookshop catalog with `book_` and `publisher_` groups |
| `src/ui/tree_groups_tests.rs` (new) | The headless tests of this plan, one file |
| `src/ui/mod.rs` | `mod tree_groups_tests;`, and its tests that name sidebar rows by their new names |
| `src/testing.rs` | `tree_row()`; `Harness::click` finds a tree item by name |
| `src/ui/export_ways_tests.rs` | Its `tree_menu` helper finds rows with `tree_row` |
| `src/ui/sidebar.rs` | Tree roles and names (Task 1); `INDENT`, `Places`, `places()`, the guide and the inset highlight (Task 2) |
| `crates/tabletist-core/src/model.rs` | `TreeKey::Fold`, `FoldAll`, `OpenAll` |
| `src/app.rs` | `tree_key` answers them; `set_every_group` |
| `crates/tabletist-core/src/keymap.rs` | `FoldGroup` `za`, `FoldAllGroups` `zM`, `OpenAllGroups` `zR`, Omarchy, sidebar |
| `src/ui/keys.rs` | The three letters to `TreeKey`s; a letter after a chord's first key goes to the chord |
| `src/ui/workspace.rs` | `tree_summary()`: the Omarchy status line names the table under the cursor |
| `docs/_guide/*.md`, `docs/_reference/keyboard-shortcuts.md` | The new layout and keys |

Checks for every commit, from the worktree (cargo through rustup, the target dir in the worktree):

```bash
export CARGO_TARGET_DIR=$PWD/target
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --lib
~/.cargo/bin/cargo test --locked -p tabletist-core --lib
```

The whole suite (`cargo test --locked --workspace --all-targets`) runs once at the end (Task 5).

---

### Task 1: The sidebar's rows are a tree

A screen reader hears the sidebar as a tree: groups are expandable items named with the prefix and count, objects are named in full with their kind, and each says its level. Nothing moves on screen yet.

**Files:**
- Modify: `crates/tabletist-core/src/fixtures.rs`
- Create: `src/ui/tree_groups_tests.rs`
- Modify: `src/ui/mod.rs`, `src/testing.rs`, `src/ui/export_ways_tests.rs`
- Modify: `src/ui/sidebar.rs`

- [ ] **Step 1: Add the grouped Bookshop catalog**

```diff
--- a/crates/tabletist-core/src/fixtures.rs
+++ b/crates/tabletist-core/src/fixtures.rs
@@ -60,6 +60,35 @@
     ]
 }
 
+/// The Bookshop's objects as a larger catalog lists them, where names
+/// share prefixes: the sidebar groups `book_` and `publisher_`, and keeps
+/// `authors`, `orders` and `reviews` on their own. In schema `main`.
+pub fn bookshop_grouped_objects() -> Vec<ObjectInfo> {
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
+        table("book_awards", 9_100),
+        table("book_cover_sizes", 4),
+        table("book_covers", 13),
+        table("book_editions", 9_400),
+        view("book_excerpts"),
+        table("book_genres", 40),
+        table("orders", 12_300),
+        table("publisher_contacts", 18),
+        table("publisher_imprints", 9),
+        table("reviews", 48_200),
+    ]
+}
+
 /// The saved connection the fake session stands for.
 pub fn fixture_connection() -> SavedConnection {
     SavedConnection {
```

- [ ] **Step 2: Write the failing tests**

Create `src/ui/tree_groups_tests.rs`:

```rust
//! The sidebar's prefix groups (issue #286): a group's tables one step in
//! under it, its prefix dropped where drawn, a guide along the open group,
//! the rows a tree for a screen reader, and the terminal look's folds.

use egui::accesskit::{Role, TreeUpdate};

use super::tests::hover;
use crate::model::ConnTabId;
use crate::testing::{Harness, node};
use crate::theme::Look;

/// The `book_` group's row, as a screen reader names it.
const BOOK: &str = "book_, 6 tables";

/// A connected tab in `look` whose `main` schema lists the Bookshop's
/// grouped catalog, with `book_` open.
fn grouped(look: Look) -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake();
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.tree.schemas.value = Some(vec!["main".into()]);
    let node = workspace.tree.nodes.entry("main".into()).or_default();
    node.expanded = true;
    node.objects.value = Some(crate::testing::bookshop_grouped_objects());
    node.open_groups.insert("book".into());
    harness.settle();
    (harness, tab)
}

fn levels(tree: &TreeUpdate) -> Vec<(String, Option<usize>)> {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::TreeItem)
        .map(|(_, node)| (node.label().unwrap_or_default().to_owned(), node.level()))
        .collect()
}

#[test]
fn a_grouped_table_drops_its_prefix_where_drawn_and_keeps_it_for_a_screen_reader() {
    let (mut harness, _) = grouped(Look::macos());
    assert!(harness.painted_rect("covers").is_some());
    assert!(harness.painted_rect("book_covers").is_none());
    let tree = harness.settle();
    assert!(node(&tree, "book_covers, table", Role::TreeItem).is_some());
    assert!(node(&tree, "book_excerpts, view", Role::TreeItem).is_some());
    assert!(node(&tree, BOOK, Role::TreeItem).is_some());
    assert!(!crate::testing::labels(&tree).contains(&"book_covers".to_owned()));
    // The pointer over it shows the full name.
    let at = harness.painted_rect("covers").unwrap().center();
    assert!(hover(&mut harness, at).contains(&"book_covers".to_owned()));
}

#[test]
fn the_tree_says_each_rows_level_and_whether_a_group_is_open() {
    let (mut harness, _) = grouped(Look::macos());
    let tree = harness.settle();
    let tree_node = node(&tree, "Objects", Role::Tree);
    assert!(tree_node.is_some(), "the sidebar's list is a tree");
    let levels = levels(&tree);
    let level = |name: &str| {
        levels
            .iter()
            .find(|(label, _)| label == name)
            .and_then(|(_, level)| *level)
    };
    assert_eq!(level(BOOK), Some(1));
    assert_eq!(level("authors, table"), Some(1));
    assert_eq!(level("book_covers, table"), Some(2));
    assert_eq!(level("publisher_, 2 tables"), Some(1));
    let expanded = |name: &str| {
        let id = node(&tree, name, Role::TreeItem).unwrap();
        let (_, found) = tree.nodes.iter().find(|(node, _)| *node == id).unwrap();
        found.is_expanded()
    };
    assert_eq!(expanded(BOOK), Some(true));
    assert_eq!(expanded("publisher_, 2 tables"), Some(false));
    assert_eq!(expanded("orders, table"), None);
}
```

Register it, and have the tests that look for a sidebar row by name use the row's new name (`src/ui/mod.rs`):

```diff
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -135,6 +135,8 @@
 mod theme_settings_tests;
 pub mod toast;
 #[cfg(test)]
+mod tree_groups_tests;
+#[cfg(test)]
 mod typing_settings_tests;
 #[cfg(test)]
 mod update_settings_tests;
@@ -3059,11 +3061,11 @@
     #[test]
     fn ctrl_b_hides_and_shows_the_sidebar() {
         let (mut harness, _tab) = tree_harness();
-        assert!(harness.has("orders"));
+        assert!(harness.has("orders, table"));
         harness.press(Key::B, Modifiers::COMMAND);
-        assert!(!harness.has("orders"));
+        assert!(!harness.has("orders, table"));
         harness.press(Key::B, Modifiers::COMMAND);
-        assert!(harness.has("orders"));
+        assert!(harness.has("orders, table"));
     }
 
     #[test]
@@ -4680,7 +4682,7 @@
         let mut harness = Harness::new();
         harness.connect_fake();
         assert!(harness.has("Schema"));
-        assert!(harness.has("active_users"));
+        assert!(harness.has("active_users, view"));
         harness.click("users");
         assert_eq!(fetches(&harness), 1);
         assert!(harness.has("users tab"));
@@ -6223,9 +6225,9 @@
         assert_eq!(harness.app.active, 1);
         harness.press(Key::Num1, Modifiers::COMMAND);
         assert_eq!(harness.app.active, 0);
-        assert!(harness.has("orders"));
+        assert!(harness.has("orders, table"));
         harness.press(Key::B, Modifiers::COMMAND);
-        assert!(!harness.has("orders"));
+        assert!(!harness.has("orders, table"));
         harness.press(Key::B, Modifiers::COMMAND);
         harness.press(Key::P, Modifiers::COMMAND);
         assert!(harness.has("Quick open"));
@@ -11321,7 +11323,7 @@
         // One flat list: in groups, all would fold into `t_`.
         harness.app.workspace_mut(tab).unwrap().tree.flat = true;
         harness.settle();
-        assert!(harness.has("t_000"));
+        assert!(harness.has("t_000, table"));
         harness.app.apply(crate::model::Action::SetTreeCursor {
             tab,
             node: crate::model::TreeNode::Object(
@@ -11331,7 +11333,7 @@
         });
         harness.press(Key::End, Modifiers::NONE);
         harness.settle();
-        assert!(harness.has("t_199"), "the cursor row is on screen");
+        assert!(harness.has("t_199, table"), "the cursor row is on screen");
     }
 
     #[test]
@@ -13062,8 +13064,12 @@
         main.objects.value = Some(vec![table(spoof), table("users"), table("users\u{200B}")]);
 
         let labels = crate::testing::labels(&harness.settle());
-        assert!(labels.iter().any(|label| label == "users_<U+202E>atad"));
-        assert!(labels.iter().any(|label| label == "users<U+200B>"));
+        assert!(
+            labels
+                .iter()
+                .any(|label| label == "users_<U+202E>atad, table")
+        );
+        assert!(labels.iter().any(|label| label == "users<U+200B>, table"));
         harness.click("users_<U+202E>atad");
         let mut page = crate::testing::page(1, false);
         page.columns[1].name = "e\u{202E}liam".into();
```

Teach the harness to click a row by the name before its kind (`src/testing.rs`), and use that in `export_ways_tests.rs`:

```diff
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -270,11 +270,13 @@
     }
 
     /// Clicks the widget labelled `label` through AccessKit: a button if
-    /// there is one, else any node with that label (selectable labels and
-    /// toggles get other roles).
+    /// there is one, else the sidebar's row of that table, view or group,
+    /// else any node with that label (selectable labels and toggles get
+    /// other roles).
     pub fn click(&mut self, label: &str) {
         let tree = self.settle();
         let target = node(&tree, label, Role::Button)
+            .or_else(|| tree_row(&tree, label))
             .or_else(|| node(&tree, label, Role::Tab))
             .or_else(|| {
                 tree.nodes
@@ -537,6 +539,19 @@
         })
         .map(|(id, _)| *id)
 }
+
+/// The sidebar's row of the object or group `name`: a tree item, named
+/// with what it is after its name ("orders, table", "book_, 2 tables").
+pub fn tree_row(tree: &TreeUpdate, name: &str) -> Option<NodeId> {
+    let named = format!("{name}, ");
+    tree.nodes
+        .iter()
+        .find(|(_, node)| {
+            node.role() == Role::TreeItem
+                && node.label().is_some_and(|label| label.starts_with(&named))
+        })
+        .map(|(id, _)| *id)
+}
 
 /// Where the node labelled `label` with `role` sits, in points.
 pub fn bounds(tree: &TreeUpdate, label: &str, role: Role) -> Option<egui::Rect> {
```

```diff
--- a/src/ui/export_ways_tests.rs
+++ b/src/ui/export_ways_tests.rs
@@ -73,9 +73,15 @@
 /// A secondary click on the tree's row of `name`.
 fn tree_menu(harness: &mut Harness, name: &str) {
     let tree = harness.settle();
-    let row = crate::testing::bounds(&tree, name, Role::Button)
+    let id = crate::testing::tree_row(&tree, name);
+    let row = id
+        .and_then(|id| tree.nodes.iter().find(|(node, _)| *node == id))
+        .and_then(|(_, node)| node.bounds())
         .unwrap_or_else(|| panic!("no tree row {name:?}"));
-    secondary(harness, row.center());
+    secondary(
+        harness,
+        egui::pos2(row.x0 as f32, row.y0 as f32) + egui::vec2(20.0, 4.0),
+    );
 }
 
 /// A secondary click on the cell of the grid's `row` (from 1) and first
@@ -540,7 +546,7 @@
         !listed.is_empty() && listed.iter().all(Option::is_some),
         "{listed:?}"
     );
-    assert!(harness.has("reviews"));
+    assert!(harness.has("reviews, table"));
     assert!(
         !painted(&harness, "Commands"),
         "no commands: {:?}",
```

- [ ] **Step 3: Run the new tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib tree_groups`

Expected: both fail; no row is a tree item yet.

```text
test ui::tree_groups_tests::a_grouped_table_drops_its_prefix_where_drawn_and_keeps_it_for_a_screen_reader ... FAILED
test ui::tree_groups_tests::the_tree_says_each_rows_level_and_whether_a_group_is_open ... FAILED
thread 'ui::tree_groups_tests::a_grouped_table_drops_its_prefix_where_drawn_and_keeps_it_for_a_screen_reader' (2005158) panicked at src/ui/tree_groups_tests.rs:45:5:
assertion failed: node(&tree, "book_covers, table", Role::TreeItem).is_some()
thread 'ui::tree_groups_tests::the_tree_says_each_rows_level_and_whether_a_group_is_open' (2005160) panicked at src/ui/tree_groups_tests.rs:59:5:
the sidebar's list is a tree
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2876 filtered out; finished in 0.03s
error: test failed, to rerun pass `--lib`
```

- [ ] **Step 4: Name the rows as a tree's items**

```diff
--- a/src/ui/sidebar.rs
+++ b/src/ui/sidebar.rs
@@ -6,7 +6,7 @@
 use tabletist_db::{ObjectKind, ObjectRef};
 
 use crate::format::display_safe;
-use crate::i18n::{Locale, gettext};
+use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Scope, Written};
 use crate::model::{Action, AgentsHere, ConnTabId, TreeNode, TreeRow, Workspace};
 use crate::style::Icon;
@@ -145,6 +145,16 @@
     }
 }
 
+/// What an object is, as a screen reader names its row and the terminal's
+/// status line says it.
+pub(super) fn kind_name(locale: Locale, kind: ObjectKind) -> std::borrow::Cow<'static, str> {
+    match kind {
+        ObjectKind::Table => gettext(locale, "table"),
+        ObjectKind::View => gettext(locale, "view"),
+        ObjectKind::MaterializedView => gettext(locale, "materialized view"),
+    }
+}
+
 /// The name kind of an object as the tree tags it, if not a table.
 fn kind_tag(locale: Locale, kind: ObjectKind) -> Option<String> {
     match kind {
@@ -410,7 +420,7 @@
                     WidgetInfo::labeled(WidgetType::Other, true, gettext(locale, "Objects"))
                 });
                 ui.ctx().accesskit_node_builder(stop.id, |node| {
-                    node.set_role(egui::accesskit::Role::Group);
+                    node.set_role(egui::accesskit::Role::Tree);
                 });
                 focus::pane(ui, &stop);
                 focus::hint(ui, &stop, area, focus::Ring::Own);
@@ -1071,8 +1081,31 @@
         _ => display_safe(&row.label),
     };
     let label = display_safe(&row.label);
-    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &*full_name));
     let selected = matches!(&row.node, TreeNode::Object(object, _) if Some(object) == marks.active);
+    // A tree's item, named in full with what it is, though a grouped
+    // object's prefix is not drawn: "book_, 21 tables", "book_covers, table".
+    let name = match &row.node {
+        TreeNode::Group(..) => {
+            let count = row.count.unwrap_or_default();
+            let plural = u32::try_from(count).unwrap_or(u32::MAX);
+            ngettext(locale, "{name}, {n} table", "{name}, {n} tables", plural)
+                .replace("{name}", &label)
+                .replace("{n}", &count.to_string())
+        }
+        TreeNode::Object(_, kind) => format!("{full_name}, {}", kind_name(locale, *kind)),
+        TreeNode::Empty(_) => label.to_string(),
+    };
+    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &name));
+    ui.ctx().accesskit_node_builder(response.id, |node| {
+        node.set_role(egui::accesskit::Role::TreeItem);
+        node.set_level(usize::from(row.depth) + 1);
+        if let Some(open) = row.expanded {
+            node.set_expanded(open);
+        }
+        if selected {
+            node.set_selected(true);
+        }
+    });
     // Rows sit in the tree's inset: the macOS pill fills it, the terminal
     // bar spans it.
     let band = if look.terminal {
```

- [ ] **Step 5: Run the tests and the checks**

Run: `~/.cargo/bin/cargo test --locked --lib tree_groups`, then the four checks above.
Expected: all pass, clippy clean.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-core/src/fixtures.rs src/ui/tree_groups_tests.rs src/ui/mod.rs src/testing.rs src/ui/export_ways_tests.rs src/ui/sidebar.rs
git commit -m "Name the sidebar's rows as a tree's items, in full with their kind"
```

---

### Task 2: An open group's tables stand in, along a guide

macOS: every table name one 18 pt step past the group's chevron, where the group's name starts; a 1 pt line in the subtle border's colour at the chevron's middle along the group's tables; a grouped table's highlight and focus ring 4 past the line. Omarchy: tables two cells right of the group's name, `│` in the first. The flat list keeps today's places.

**Files:**
- Modify: `src/ui/tree_groups_tests.rs`
- Modify: `src/ui/sidebar.rs`

- [ ] **Step 1: Write the failing tests**

```diff
--- a/src/ui/tree_groups_tests.rs
+++ b/src/ui/tree_groups_tests.rs
@@ -2,16 +2,23 @@
 //! under it, its prefix dropped where drawn, a guide along the open group,
 //! the rows a tree for a screen reader, and the terminal look's folds.
 
+use egui::Rect;
 use egui::accesskit::{Role, TreeUpdate};
+use tabletist_db::{ObjectKind, ObjectRef};
 
+use super::sidebar::INDENT;
 use super::tests::hover;
-use crate::model::ConnTabId;
-use crate::testing::{Harness, node};
+use crate::model::{Action, ConnTabId, TreeNode};
+use crate::testing::{Harness, bounds, node};
 use crate::theme::Look;
+use crate::typography::TextRole;
 
 /// The `book_` group's row, as a screen reader names it.
 const BOOK: &str = "book_, 6 tables";
 
+/// Where the macOS chevron begins in its row: the tree's inset, then 8.
+const FOLD: f32 = 8.0 + 8.0;
+
 /// A connected tab in `look` whose `main` schema lists the Bookshop's
 /// grouped catalog, with `book_` open.
 fn grouped(look: Look) -> (Harness, ConnTabId) {
@@ -28,6 +35,59 @@
     (harness, tab)
 }
 
+fn table(name: &str) -> TreeNode {
+    TreeNode::Object(ObjectRef::new("main", name), ObjectKind::Table)
+}
+
+fn group(prefix: &str) -> TreeNode {
+    TreeNode::Group("main".into(), prefix.into())
+}
+
+/// Where the row a screen reader names `name` sits.
+fn row(harness: &mut Harness, name: &str) -> Rect {
+    let tree = harness.settle();
+    bounds(&tree, name, Role::TreeItem).unwrap_or_else(|| panic!("no row {name:?}"))
+}
+
+/// Where the last frame began to paint `text`.
+fn drawn(harness: &Harness, text: &str) -> f32 {
+    harness
+        .painted_rect(text)
+        .unwrap_or_else(|| panic!("{text:?} is not drawn"))
+        .left()
+}
+
+fn close(a: f32, b: f32) -> bool {
+    (a - b).abs() < 0.5
+}
+
+/// The pieces of the macOS guide the last frame drew: 1 pt wide, in the
+/// subtle border's colour.
+fn guides(harness: &Harness) -> Vec<Rect> {
+    let color = harness.app.palette.surface_active;
+    harness
+        .fills
+        .iter()
+        .filter(|(rect, fill)| *fill == color && close(rect.width(), 1.0))
+        .map(|(rect, _)| *rect)
+        .collect()
+}
+
+/// Where the terminal look drew its `│` guides.
+fn bars(harness: &Harness) -> Vec<Rect> {
+    harness
+        .text_rects
+        .iter()
+        .filter(|(text, _)| text == "│")
+        .map(|(_, rect)| *rect)
+        .collect()
+}
+
+/// One character of the terminal's face.
+fn cell(harness: &Harness) -> f32 {
+    TextRole::OBody.width(&harness.ctx, harness.app.look.faces, " ")
+}
+
 fn levels(tree: &TreeUpdate) -> Vec<(String, Option<usize>)> {
     tree.nodes
         .iter()
@@ -37,6 +97,63 @@
 }
 
 #[test]
+fn grouped_tables_stand_one_step_in_and_single_tables_line_up_with_them() {
+    let (mut harness, _) = grouped(Look::macos());
+    let child = row(&mut harness, "book_covers, table");
+    let header = row(&mut harness, BOOK);
+    // One step past the group's chevron: where the group's name starts.
+    let covers = drawn(&harness, "covers");
+    assert!(close(covers - child.left(), FOLD + INDENT), "{covers}");
+    assert!(close(
+        drawn(&harness, "book_") - header.left(),
+        FOLD + INDENT
+    ));
+    // Single tables, grouped or not, in the same column.
+    for single in ["authors", "orders", "reviews"] {
+        assert!(close(drawn(&harness, single), covers), "{single}");
+    }
+}
+
+#[test]
+fn the_terminal_indents_a_groups_tables_two_cells_and_single_tables_with_them() {
+    let (mut harness, _) = grouped(Look::omarchy());
+    harness.settle();
+    let name = drawn(&harness, "book_");
+    let covers = drawn(&harness, "covers");
+    assert!(
+        close(covers - name, 2.0 * cell(&harness)),
+        "{name} {covers}"
+    );
+    for single in ["authors", "orders", "reviews"] {
+        assert!(close(drawn(&harness, single), covers), "{single}");
+    }
+}
+
+#[test]
+fn a_grouped_tables_highlight_begins_inside_the_indent() {
+    let (mut harness, _) = grouped(Look::macos());
+    let selection = harness.app.palette.selection;
+    let pill = |harness: &Harness, row: Rect| {
+        harness
+            .fills
+            .iter()
+            .find(|(rect, fill)| *fill == selection && row.contains(rect.center()))
+            .map(|(rect, _)| *rect)
+            .expect("the selected row's pill")
+    };
+    harness.click("book_covers, table");
+    let child = row(&mut harness, "book_covers, table");
+    // Past the guide at the chevron's middle, 4 after it.
+    let begins = pill(&harness, child).left() - child.left();
+    assert!(close(begins, FOLD + 6.0 + 4.0), "{begins}");
+    // A single table's pill fills the tree's inset, as before.
+    harness.click("orders, table");
+    let single = row(&mut harness, "orders, table");
+    let begins = pill(&harness, single).left() - single.left();
+    assert!(close(begins, 8.0), "{begins}");
+}
+
+#[test]
 fn a_grouped_table_drops_its_prefix_where_drawn_and_keeps_it_for_a_screen_reader() {
     let (mut harness, _) = grouped(Look::macos());
     assert!(harness.painted_rect("covers").is_some());
@@ -77,3 +194,117 @@
     assert_eq!(expanded("publisher_, 2 tables"), Some(false));
     assert_eq!(expanded("orders, table"), None);
 }
+
+#[test]
+fn a_longer_prefix_stays_in_its_group_and_groups_never_nest() {
+    let (harness, tab) = grouped(Look::macos());
+    let workspace = harness.app.workspace(tab).unwrap();
+    let rows = workspace.tree.visible_rows(workspace.driver, false);
+    let sizes = rows
+        .iter()
+        .find(|row| row.node == table("book_cover_sizes"))
+        .expect("book_cover_sizes");
+    assert_eq!((sizes.depth, sizes.label.as_str()), (1, "cover_sizes"));
+    let groups: Vec<&TreeNode> = rows
+        .iter()
+        .map(|row| &row.node)
+        .filter(|node| matches!(node, TreeNode::Group(..)))
+        .collect();
+    assert_eq!(groups, [&group("book"), &group("publisher")]);
+}
+
+#[test]
+fn groups_and_single_tables_sort_together() {
+    let (harness, tab) = grouped(Look::macos());
+    let workspace = harness.app.workspace(tab).unwrap();
+    let top: Vec<String> = workspace
+        .tree
+        .visible_rows(workspace.driver, false)
+        .into_iter()
+        .filter(|row| row.depth == 0)
+        .map(|row| row.label)
+        .collect();
+    assert_eq!(top, ["authors", "book_", "orders", "publisher_", "reviews"]);
+}
+
+#[test]
+fn the_guide_runs_along_the_open_groups_tables_and_no_further() {
+    let (mut harness, tab) = grouped(Look::macos());
+    let first = row(&mut harness, "book_awards, table");
+    let last = row(&mut harness, "book_genres, table");
+    let pieces = guides(&harness);
+    assert_eq!(pieces.len(), 6, "a piece on each of the group's tables");
+    let top = pieces
+        .iter()
+        .map(|rect| rect.top())
+        .fold(f32::MAX, f32::min);
+    let bottom = pieces
+        .iter()
+        .map(|rect| rect.bottom())
+        .fold(f32::MIN, f32::max);
+    assert!(close(top, first.top()) && close(bottom, last.bottom()));
+    // At the chevron's middle.
+    assert!(
+        pieces
+            .iter()
+            .all(|rect| close(rect.left(), first.left() + FOLD + 6.0))
+    );
+    // A folded group has none.
+    harness.app.apply(Action::ToggleGroup {
+        tab,
+        schema: "main".into(),
+        prefix: "book".into(),
+    });
+    harness.settle();
+    assert!(guides(&harness).is_empty());
+
+    // The terminal's `│`, under the group's name.
+    let (mut harness, _) = grouped(Look::omarchy());
+    harness.settle();
+    let name = drawn(&harness, "book_");
+    let bars = bars(&harness);
+    assert_eq!(bars.len(), 6);
+    assert!(bars.iter().all(|rect| close(rect.left(), name)));
+}
+
+#[test]
+fn the_flat_list_keeps_full_names_with_no_groups_or_guides() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, tab) = grouped(look);
+        harness.app.apply(Action::ToggleFlatTree(tab));
+        let tree = harness.settle();
+        assert!(node(&tree, BOOK, Role::TreeItem).is_none(), "{}", look.name);
+        assert!(levels(&tree).iter().all(|(_, level)| *level == Some(1)));
+        assert!(guides(&harness).is_empty() && bars(&harness).is_empty());
+        let covers = row(&mut harness, "book_covers, table");
+        let x = drawn(&harness, "book_covers");
+        assert!(close(x, drawn(&harness, "authors")), "{}", look.name);
+        // Where a flat list has always drawn its names.
+        let glyph = if look.terminal {
+            TextRole::OBody.width(&harness.ctx, harness.app.look.faces, "▾")
+        } else {
+            12.0
+        };
+        let inset = if look.terminal { 4.0 } else { 8.0 };
+        assert!(
+            close(x - covers.left(), inset + 8.0 + glyph + 6.0),
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn the_indent_scales_with_the_windows_zoom() {
+    for zoom in [1.0, 1.5, 2.0] {
+        let (mut harness, _) = grouped(Look::macos());
+        harness.ctx.set_zoom_factor(zoom);
+        harness.settle();
+        // Points stay the same; the screen's pixels grow with the zoom.
+        let guide = guides(&harness)[0];
+        let step = drawn(&harness, "covers") - (guide.left() - 6.0);
+        assert!(close(step, INDENT), "{zoom}: {step}");
+        assert!(close(step * harness.ctx.pixels_per_point(), INDENT * zoom));
+        assert!(close(guide.width() * harness.ctx.pixels_per_point(), zoom));
+    }
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib tree_groups`

Expected: compile error, `INDENT` is not in `sidebar` yet.

```text
error[E0432]: unresolved import `super::sidebar::INDENT`
error: could not compile `tabletist` (lib test) due to 1 previous error
```

- [ ] **Step 3: Place the rows with `places`, draw the guide and the inset highlight**

```diff
--- a/src/ui/sidebar.rs
+++ b/src/ui/sidebar.rs
@@ -465,6 +465,7 @@
                                     cursor: cursor.as_ref(),
                                     keys,
                                     lit,
+                                    flat,
                                 },
                                 locale,
                                 Skin {
@@ -994,6 +995,8 @@
     keys: bool,
     /// And the keyboard is in use: the cursor shows.
     lit: bool,
+    /// The objects are one flat list: no groups, no indent.
+    flat: bool,
 }
 
 /// How rows are drawn: the platform look and the colour palette.
@@ -1003,17 +1006,66 @@
     palette: &'a Palette,
 }
 
-/// Where a tree row draws its label. macOS: the chevron 8 into the row
-/// (itself 8 in), then 6 to the name; grouped objects 26 into theirs.
-/// Terminal: 4 and 8 in, the glyph, 6; objects 24 into theirs.
-fn label_x(rect: Rect, depth: u8, look: &Look, glyph: f32) -> f32 {
+/// One step of the macOS tree's indent: the chevron, 12, and the 6 after
+/// it. A grouped object's name starts where its group's name does.
+pub(crate) const INDENT: f32 = 18.0;
+
+/// Where a tree row draws, from its left edge.
+#[derive(Debug, Clone, Copy, PartialEq)]
+pub(crate) struct Places {
+    /// The fold mark: macOS's chevron, the terminal's ▸.
+    pub fold: f32,
+    /// The name.
+    pub label: f32,
+    /// The guide along an open group's objects, on each of its rows:
+    /// macOS's line at the chevron's middle, the terminal's `│` under its
+    /// group's name.
+    pub guide: Option<f32>,
+    /// Where the highlight and the focus ring begin: inside the indent,
+    /// past the guide, for a grouped object.
+    pub band: f32,
+}
+
+/// Where `row` draws, in the look. `glyph` is the fold mark's width and
+/// `cell` one character's of the terminal's face. macOS: the chevron 8
+/// into the row (itself 8 in), every name one step past it, so tables
+/// line up whether grouped or not. Terminal: the mark 4 and 8 in, a
+/// group's name 6 after it, and in the tree every object two cells
+/// further, a grouped one with `│` in the first of them.
+pub(crate) fn places(row: &TreeRow, flat: bool, look: &Look, glyph: f32, cell: f32) -> Places {
     let inset = tree_inset(look);
-    let (pad, nested) = if look.terminal {
-        (8.0, 24.0)
-    } else {
-        (8.0, 26.0)
-    };
-    rect.left() + inset + if depth > 0 { nested } else { pad + glyph + 6.0 }
+    let fold = inset + 8.0;
+    let grouped = row.depth > 0;
+    if look.terminal {
+        let name = fold + glyph + 6.0;
+        let label = match row.node {
+            TreeNode::Object(..) if !flat => name + 2.0 * cell,
+            _ => name,
+        };
+        return Places {
+            fold,
+            label,
+            guide: grouped.then_some(name),
+            band: inset,
+        };
+    }
+    let guide = fold + 6.0;
+    // The pill stands 8 inside its band.
+    let pill = match look.selection {
+        crate::theme::Selection::Pill => 8.0,
+        crate::theme::Selection::Tint | crate::theme::Selection::Bar => 0.0,
+    };
+    Places {
+        fold,
+        label: fold + INDENT,
+        guide: grouped.then_some(guide),
+        band: if grouped { guide + 4.0 - pill } else { 0.0 },
+    }
+}
+
+/// One character's width in the terminal's face.
+fn cell_width(ui: &egui::Ui, look: &Look) -> f32 {
+    TextRole::OBody.width(ui.ctx(), look.faces, " ")
 }
 
 /// The width of the fold mark: macOS's 12 pt chevron, the terminal's ▾.
@@ -1045,8 +1097,8 @@
         ui.allocate_ui_with_layout(size, layout, |ui| {
             ui.set_min_size(size);
             ui.spacing_mut().item_spacing.x = 6.0;
-            let left = label_x(ui.max_rect(), 0, look, glyph_width(ui, look));
-            ui.add_space(left - ui.max_rect().left());
+            let at = places(row, marks.flat, look, glyph_width(ui, look), 0.0);
+            ui.add_space(at.label);
             let refresh = gettext(locale, "Reload");
             let link_width = small.width(ui.ctx(), look.faces, &refresh);
             let room = ui.available_width() - link_width - ui.spacing().item_spacing.x;
@@ -1106,13 +1158,17 @@
             node.set_selected(true);
         }
     });
+    let glyph = glyph_width(ui, look);
+    let at = places(row, marks.flat, look, glyph, cell_width(ui, look));
     // Rows sit in the tree's inset: the macOS pill fills it, the terminal
-    // bar spans it.
-    let band = if look.terminal {
-        rect.shrink2(vec2(tree_inset(look), 0.0))
-    } else {
-        rect
-    };
+    // bar spans it. A grouped object's begins past its guide.
+    let band = Rect::from_min_max(
+        pos2(rect.left() + at.band, rect.top()),
+        pos2(
+            rect.right() - if look.terminal { at.band } else { 0.0 },
+            rect.bottom(),
+        ),
+    );
     widgets::selection(ui, band, selected, response.hovered(), look, palette);
     let highlight = widgets::selection_rect(band, look);
     // The terminal's bar on the open object's row is muted while the keys
@@ -1134,9 +1190,21 @@
         );
     }
     let center = rect.center().y;
-    let glyph = glyph_width(ui, look);
-    let x = label_x(rect, row.depth, look, glyph);
-    let fold_x = rect.left() + tree_inset(look) + 8.0;
+    let x = rect.left() + at.label;
+    let fold_x = rect.left() + at.fold;
+    // The guide along the open group, a piece on each of its rows: one
+    // line from its first object to its last.
+    if let Some(guide) = at.guide {
+        let guide = rect.left() + guide;
+        if look.terminal {
+            let mark = Text::one(look, TextRole::OBody, "│", palette.dim);
+            widgets::paint_text(ui, guide, center, mark);
+        } else {
+            let line = Rect::from_min_size(pos2(guide, rect.top()), vec2(1.0, rect.height()));
+            ui.painter()
+                .rect_filled(line, CornerRadius::ZERO, palette.surface_active);
+        }
+    }
     if let Some(expanded) = row.expanded {
         if look.terminal {
             let mark = if expanded { "▾" } else { "▸" };
```

- [ ] **Step 4: Run the tests and the checks**

Run: `~/.cargo/bin/cargo test --locked --lib tree_groups`, then the four checks.
Expected: all pass. `a_longer_prefix_stays_in_its_group_and_groups_never_nest` and `groups_and_single_tables_sort_together` passed before this task too: they hold what the model already does.

- [ ] **Step 5: Commit**

```bash
git add src/ui/tree_groups_tests.rs src/ui/sidebar.rs
git commit -m "Indent an open group's tables in the sidebar, with a guide along them"
```

---

### Task 3: The tree's folds: arrows on macOS, `za` `zM` `zR` on Omarchy

`←`/`→` already close and open a group and climb from a table to its group; this task tests that on the grouped rows. Omarchy gets vim's folds: `za` on a group folds or opens it, on a grouped table folds its group and moves the cursor up to it; `zM` folds every group, `zR` opens every group. `R` alone still reloads: a letter after a chord's first key now goes to the chord.

**Files:**
- Modify: `src/ui/tree_groups_tests.rs`
- Modify: `crates/tabletist-core/src/model.rs`, `crates/tabletist-core/src/keymap.rs`
- Modify: `src/app.rs`, `src/ui/keys.rs`

- [ ] **Step 1: Write the failing tests**

```diff
--- a/src/ui/tree_groups_tests.rs
+++ b/src/ui/tree_groups_tests.rs
@@ -2,12 +2,12 @@
 //! under it, its prefix dropped where drawn, a guide along the open group,
 //! the rows a tree for a screen reader, and the terminal look's folds.
 
-use egui::Rect;
 use egui::accesskit::{Role, TreeUpdate};
+use egui::{Key, Modifiers, Rect};
 use tabletist_db::{ObjectKind, ObjectRef};
 
 use super::sidebar::INDENT;
-use super::tests::hover;
+use super::tests::{hover, type_key};
 use crate::model::{Action, ConnTabId, TreeNode};
 use crate::testing::{Harness, bounds, node};
 use crate::theme::Look;
@@ -88,6 +88,20 @@
     TextRole::OBody.width(&harness.ctx, harness.app.look.faces, " ")
 }
 
+fn cursor(harness: &Harness, tab: ConnTabId) -> Option<TreeNode> {
+    harness.app.workspace(tab).unwrap().tree.cursor.clone()
+}
+
+fn open_groups(harness: &Harness, tab: ConnTabId) -> Vec<String> {
+    let mut open: Vec<String> = harness.app.workspace(tab).unwrap().tree.nodes["main"]
+        .open_groups
+        .iter()
+        .cloned()
+        .collect();
+    open.sort();
+    open
+}
+
 fn levels(tree: &TreeUpdate) -> Vec<(String, Option<usize>)> {
     tree.nodes
         .iter()
@@ -228,6 +242,52 @@
 }
 
 #[test]
+fn left_and_right_close_and_open_a_group_and_left_climbs_to_it() {
+    let (mut harness, tab) = grouped(Look::macos());
+    harness.click("book_covers, table");
+    harness.press(Key::ArrowLeft, Modifiers::NONE);
+    assert_eq!(cursor(&harness, tab), Some(group("book")));
+    harness.press(Key::ArrowLeft, Modifiers::NONE);
+    assert_eq!(open_groups(&harness, tab), Vec::<String>::new());
+    harness.press(Key::ArrowRight, Modifiers::NONE);
+    assert_eq!(open_groups(&harness, tab), ["book"]);
+    harness.press(Key::ArrowRight, Modifiers::NONE);
+    assert_eq!(cursor(&harness, tab), Some(table("book_awards")));
+}
+
+#[test]
+fn omarchys_folds_work_on_the_groups_and_t_still_flattens() {
+    let (mut harness, tab) = grouped(Look::omarchy());
+    harness.app.apply(Action::SetTreeCursor {
+        tab,
+        node: table("book_covers"),
+    });
+    // `za` on a table folds its group, and the cursor goes up to it.
+    type_key(&mut harness, Key::Z, "z");
+    type_key(&mut harness, Key::A, "a");
+    assert_eq!(open_groups(&harness, tab), Vec::<String>::new());
+    assert_eq!(cursor(&harness, tab), Some(group("book")));
+    // On the group it opens it again.
+    type_key(&mut harness, Key::Z, "z");
+    type_key(&mut harness, Key::A, "a");
+    assert_eq!(open_groups(&harness, tab), ["book"]);
+    // `zR` opens every group, `zM` folds them all.
+    type_key(&mut harness, Key::Z, "z");
+    type_key(&mut harness, Key::R, "R");
+    assert_eq!(open_groups(&harness, tab), ["book", "publisher"]);
+    harness.app.apply(Action::SetTreeCursor {
+        tab,
+        node: table("publisher_imprints"),
+    });
+    type_key(&mut harness, Key::Z, "z");
+    type_key(&mut harness, Key::M, "M");
+    assert_eq!(open_groups(&harness, tab), Vec::<String>::new());
+    assert_eq!(cursor(&harness, tab), Some(group("publisher")));
+    type_key(&mut harness, Key::T, "t");
+    assert!(harness.app.workspace(tab).unwrap().tree.flat);
+}
+
+#[test]
 fn the_guide_runs_along_the_open_groups_tables_and_no_further() {
     let (mut harness, tab) = grouped(Look::macos());
     let first = row(&mut harness, "book_awards, table");
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib tree_groups`

Expected: the arrows test passes (the keys exist); the folds test fails, `za` does nothing.

```text
test ui::tree_groups_tests::omarchys_folds_work_on_the_groups_and_t_still_flattens ... FAILED
thread 'ui::tree_groups_tests::omarchys_folds_work_on_the_groups_and_t_still_flattens' (989455) panicked at src/ui/tree_groups_tests.rs:268:5:
assertion `left == right` failed
  left: ["book"]
 right: []
test result: FAILED. 11 passed; 1 failed; 0 ignored; 0 measured; 2876 filtered out; finished in 0.07s
error: test failed, to rerun pass `--lib`
```

- [ ] **Step 3: Add the tree's fold keys to the model**

```diff
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -3830,6 +3830,13 @@
     Enter,
     Home,
     End,
+    /// Fold or unfold the group under the cursor, or fold the group of
+    /// the object under it, as vim's `za` does.
+    Fold,
+    /// Fold every group.
+    FoldAll,
+    /// Unfold every group.
+    OpenAll,
 }
 
 #[derive(Debug, Default)]
```

- [ ] **Step 4: Bind them in the Omarchy keymap**

```diff
--- a/crates/tabletist-core/src/keymap.rs
+++ b/crates/tabletist-core/src/keymap.rs
@@ -525,6 +525,9 @@
     MoveInTree = "move_in_tree",
     OpenObject = "open_object",
     ToggleTree = "toggle_tree",
+    FoldGroup = "fold_group",
+    FoldAllGroups = "fold_all_groups",
+    OpenAllGroups = "open_all_groups",
     Reload = "reload",
     FilterBar = "filter_bar",
     WhereFilter = "where_filter",
@@ -943,6 +946,9 @@
             C::MoveInTree => info("Move in the sidebar", "", Navigation),
             C::OpenObject => info("Open the table or view", "", Navigation),
             C::ToggleTree => info("Tree or flat list", "", Navigation),
+            C::FoldGroup => info("Fold or open the group", "", Navigation),
+            C::FoldAllGroups => info("Fold every group", "", Navigation),
+            C::OpenAllGroups => info("Open every group", "", Navigation),
             C::Reload => info("Reload", "Reload", Navigation),
             C::FilterBar => info("Filter bar", "", Navigation),
             C::WhereFilter => info("Filter with a WHERE line", "", Navigation),
@@ -1674,6 +1680,10 @@
             keys(&[k("enter")]),
         ),
         bind(C::ToggleTree, SIDEBAR, NONE, keys(&[k("t")])),
+        // Vim's folds, on the tree's groups.
+        bind(C::FoldGroup, SIDEBAR, NONE, keys(&[k("za")])),
+        bind(C::FoldAllGroups, SIDEBAR, NONE, keys(&[k("zM")])),
+        bind(C::OpenAllGroups, SIDEBAR, NONE, keys(&[k("zR")])),
         bind(
             C::Reload,
             &[Scope::Grid, Scope::Sidebar],
```

- [ ] **Step 5: Answer them in `tree_key`**

```diff
--- a/src/app.rs
+++ b/src/app.rs
@@ -20,7 +20,7 @@
     Advance, CellPos, Completion, ConnectionForm, Dialog, Fetch, FilterBar, FilterRow, Found, Held,
     HostKeyPrompt, LeavePrompt, ObjectTab, ObjectView, Pane, PasswordPrompt, PickTarget, Reach,
     ResultPane, RunMode, SecretKind, SessionStatus, SqlTab, StartRecent, Tab, TabId, TestState,
-    TextPrint, Tree, TreeKey, TreeNode, Wanted, Workspace,
+    TextPrint, Tree, TreeKey, TreeNode, TreeRow, Wanted, Workspace,
 };
 use crate::paths::AppDirs;
 use crate::secrets::{SecretString, password_account, ssh_account};
@@ -7675,6 +7675,27 @@
             .as_ref()
             .and_then(|cursor| rows.iter().position(|row| &row.node == cursor));
         let last = rows.len() - 1;
+        // Up to the parent: the nearest row above that is shallower.
+        let parent = |at: usize| {
+            (0..at)
+                .rev()
+                .find(|&i| rows[i].depth < rows[at].depth)
+                .unwrap_or(at)
+        };
+        if let TreeKey::FoldAll | TreeKey::OpenAll = key {
+            let open = key == TreeKey::OpenAll;
+            // An object of a group folded now leaves its cursor to the group.
+            let target = at.map(|at| if open { at } else { parent(at) });
+            self.set_every_group(tab, &rows, open);
+            if let Some(index) = target
+                && let Some(workspace) = self.workspace_mut(tab)
+            {
+                workspace.tree.cursor = Some(rows[index].node.clone());
+                workspace.tree.reveal_cursor = true;
+                workspace.pane = Pane::Tree;
+            }
+            return;
+        }
         let target = match (key, at) {
             // No cursor, or it is on a row no longer shown: End and Up start
             // at the bottom, anything else at the top.
@@ -7697,14 +7718,23 @@
                     self.toggle_tree_row(tab, &rows[at].node);
                     Some(at)
                 }
-                // Up to the parent: the nearest row above that is shallower.
-                _ => Some(
-                    (0..at)
-                        .rev()
-                        .find(|&i| rows[i].depth < rows[at].depth)
-                        .unwrap_or(at),
-                ),
+                _ => Some(parent(at)),
+            },
+            // A group folds or unfolds; an object of one folds its group,
+            // and the cursor goes up to it.
+            (TreeKey::Fold, Some(at)) => match rows[at].expanded {
+                Some(_) => {
+                    self.toggle_tree_row(tab, &rows[at].node);
+                    Some(at)
+                }
+                None if rows[at].depth > 0 => {
+                    let group = parent(at);
+                    self.toggle_tree_row(tab, &rows[group].node);
+                    Some(group)
+                }
+                None => Some(at),
             },
+            (TreeKey::FoldAll | TreeKey::OpenAll, Some(at)) => Some(at),
             (TreeKey::Enter, Some(at)) => {
                 match &rows[at].node {
                     TreeNode::Object(object, kind) => self.apply(Action::OpenObject {
@@ -7727,6 +7757,23 @@
         }
     }
 
+    /// Folds every group of the tree `rows` shows, or unfolds them all.
+    fn set_every_group(&mut self, tab: ConnTabId, rows: &[TreeRow], open: bool) {
+        let Some(workspace) = self.workspace_mut(tab) else {
+            return;
+        };
+        for row in rows {
+            if let TreeNode::Group(schema, prefix) = &row.node {
+                let node = workspace.tree.nodes.entry(schema.clone()).or_default();
+                if open {
+                    node.open_groups.insert(prefix.clone());
+                } else {
+                    node.open_groups.remove(prefix);
+                }
+            }
+        }
+    }
+
     /// Folds or unfolds a schema or group row.
     fn toggle_tree_row(&mut self, tab: ConnTabId, node: &TreeNode) {
         match node {
```

- [ ] **Step 6: Read the letters, and give a chord the letter after its first key**

```diff
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -2483,6 +2483,18 @@
     }
 }
 
+/// Whether `char` begins a key of several in the first of `scopes` that
+/// reads it: the keymap waits for the next.
+fn begins_chord(keymap: &Keymap, layout: Layout, scopes: &[Scope], char: char) -> bool {
+    let nothing = |_: crate::keymap::When| false;
+    let typed = char.to_string();
+    scopes
+        .iter()
+        .map(|scope| keymap.typed(layout, *scope, &nothing, &typed))
+        .find(|read| *read != Typed::Nothing)
+        == Some(Typed::Waiting)
+}
+
 /// Single letters: the picker's keys, and in the terminal look the
 /// workspace's vim keys. Only when no text field has the keyboard. `field`
 /// is the column of the row panel's field that has it: `j` and `k` step
@@ -2500,8 +2512,22 @@
     // taken from the frame, its key with it; any other is left for the
     // code below.
     let layout = app.layout();
+    // A key of several begun (`z` of `zR`) takes the letter after it,
+    // though alone it is one of these (`R`). A count does not.
+    let mut begun: String = ctx
+        .data(|data| data.get_temp(typed_id()))
+        .unwrap_or_default();
     for char in typed_chars(ctx) {
+        if !begun.is_empty() && !crate::keymap::is_count(&begun) {
+            begun.clear();
+            continue;
+        }
         let Some(command) = window_letter(&app.keymap, layout, scopes, char) else {
+            begun = if begins_chord(&app.keymap, layout, scopes, char) {
+                char.to_string()
+            } else {
+                String::new()
+            };
             continue;
         };
         take_char(ctx, char);
@@ -3533,6 +3559,22 @@
                 };
                 actions.push(Action::TreeKey { tab, key });
             }
+            // Vim's folds, on the tree's groups.
+            (
+                Some(
+                    command
+                    @ (Command::FoldGroup | Command::FoldAllGroups | Command::OpenAllGroups),
+                ),
+                _,
+            ) => {
+                use crate::model::TreeKey;
+                let key = match command {
+                    Command::FoldGroup => TreeKey::Fold,
+                    Command::FoldAllGroups => TreeKey::FoldAll,
+                    _ => TreeKey::OpenAll,
+                };
+                actions.push(Action::TreeKey { tab, key });
+            }
             // The row before and the one after, from a field of the
             // panel or from the rows it shows one of.
             (Some(command @ (Command::PreviousRow | Command::NextRow)), _) => {
```

- [ ] **Step 7: Run the tests and the checks**

Run: `~/.cargo/bin/cargo test --locked --lib tree_groups`, then the four checks (the keymap's generated tests are in `tabletist-core`: no two commands share a key).
Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src/ui/tree_groups_tests.rs crates/tabletist-core/src/model.rs crates/tabletist-core/src/keymap.rs src/app.rs src/ui/keys.rs
git commit -m "Fold the sidebar's groups with za, zM and zR in the Omarchy look"
```

---

### Task 4: The Omarchy status line names the table under the cursor

**Files:**
- Modify: `src/ui/tree_groups_tests.rs`
- Modify: `src/ui/workspace.rs`

- [ ] **Step 1: Write the failing test**

```diff
--- a/src/ui/tree_groups_tests.rs
+++ b/src/ui/tree_groups_tests.rs
@@ -368,3 +368,32 @@
         assert!(close(guide.width() * harness.ctx.pixels_per_point(), zoom));
     }
 }
+
+#[test]
+fn the_terminal_status_line_names_the_table_under_the_cursor_in_full() {
+    let (mut harness, tab) = grouped(Look::omarchy());
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: ObjectRef::new("main", "orders"),
+        kind: ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_rows(crate::testing::page(3, false));
+    harness.app.apply(Action::SetTreeCursor {
+        tab,
+        node: table("book_covers"),
+    });
+    harness.settle();
+    assert!(
+        harness
+            .painted_rect("book_covers · table · 13 rows")
+            .is_some()
+    );
+    // A view has no estimate: no rows said.
+    harness.app.apply(Action::SetTreeCursor {
+        tab,
+        node: TreeNode::Object(ObjectRef::new("main", "book_excerpts"), ObjectKind::View),
+    });
+    harness.settle();
+    assert!(harness.painted_rect("book_excerpts · view").is_some());
+}
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib tree_groups`

```text
test ui::tree_groups_tests::the_terminal_status_line_names_the_table_under_the_cursor_in_full ... FAILED
thread 'ui::tree_groups_tests::the_terminal_status_line_names_the_table_under_the_cursor_in_full' (1476997) panicked at src/ui/tree_groups_tests.rs:387:5:
assertion failed: harness.painted_rect("book_covers · table · 13 rows").is_some()
test result: FAILED. 12 passed; 1 failed; 0 ignored; 0 measured; 2876 filtered out; finished in 0.07s
error: test failed, to rerun pass `--lib`
```

- [ ] **Step 3: Say it in the place of the page's range**

```diff
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2717,6 +2717,37 @@
 /// The `:` prompt takes the whole of it while it is open (`command_line`).
 /// `asked` is what it asks of the app; `settings` say whether the editor
 /// keys are vim's, and how the columns in view are fitted.
+/// What the status line says of the object under the tree's cursor while
+/// the tree has the keys, in full though the sidebar drops its group's
+/// prefix: "book_covers · table · 13 rows", the rows where the database
+/// has an estimate.
+fn tree_summary(
+    workspace: &crate::model::Workspace,
+    locale: crate::i18n::Locale,
+) -> Option<String> {
+    if workspace.pane != crate::model::Pane::Tree {
+        return None;
+    }
+    let Some(crate::model::TreeNode::Object(object, kind)) = &workspace.tree.cursor else {
+        return None;
+    };
+    let kind = super::sidebar::kind_name(locale, *kind);
+    let name = crate::format::display_safe(&object.name);
+    let rows = workspace
+        .tree
+        .object_info(object)
+        .and_then(|info| info.estimated_rows)
+        .map(|rows| {
+            let plural = u32::try_from(rows).unwrap_or(u32::MAX);
+            crate::i18n::ngettext(locale, "{n} row", "{n} rows", plural)
+                .replace("{n}", &rows.to_string())
+        });
+    Some(match rows {
+        Some(rows) => format!("{name} · {kind} · {rows}"),
+        None => format!("{name} · {kind}"),
+    })
+}
+
 fn status_line(
     ui: &mut egui::Ui,
     paint: super::Paint,
@@ -2867,7 +2898,8 @@
             None => at,
         })
     });
-    let summary = editor
+    let summary = tree_summary(workspace, locale)
+        .or(editor)
         .or(lines)
         .or_else(|| {
             let object = workspace.active_object_tab()?;
```

- [ ] **Step 4: Run the tests and the checks**

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/ui/tree_groups_tests.rs src/ui/workspace.rs
git commit -m "Name the table under the sidebar's cursor in full on the Omarchy status line"
```

---

### Task 5: The guide pages, and the whole suite

**Files:**
- Modify: `docs/_guide/browsing-data.md`, `docs/_guide/omarchy.md`, `docs/_reference/keyboard-shortcuts.md`

- [ ] **Step 1: Write the pages**

```diff
--- a/docs/_guide/browsing-data.md
+++ b/docs/_guide/browsing-data.md
@@ -7,8 +7,12 @@
 ## The sidebar
 
 The sidebar shows the tables and views of one schema at a time. Tables
-that share a prefix, such as `book_`, fold into a group. You can also
-list them flat. On macOS and Windows, the objects you opened recently
+that share a prefix, such as `book_`, fold into a group. Inside an open
+group the prefix is dropped (`book_covers` shows as `covers`) and a line
+runs along the group's tables; the pointer over one shows its full name.
+Every table name, grouped or not, starts in the same column. Groups have
+one level: `book_cover_sizes` sits under `book_` as `cover_sizes`. You can
+also list them flat, with full names. On macOS and Windows, the objects you opened recently
 are listed above them.
 
 Quick open (Cmd+P on macOS, Ctrl+P on Windows and Omarchy) finds any
```

```diff
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -105,7 +105,9 @@
 | Keys | Do |
 | --- | --- |
 | `j` `k`, arrows, Home, End | Move |
-| Enter | Open the table or view |
+| Enter | Open the table or view, or fold or open a group |
+| `za` | Fold or open the group under the cursor; on a table of a group, fold its group |
+| `zM` `zR` | Fold every group, open every group |
 | `/` | Filter the list |
 | `t` | Switch between tree and flat. The connection keeps its choice; Settings › Appearance sets the default |
 | `R` | Reload the list |
@@ -114,6 +116,10 @@
 | Right-click | A table's or view's menu: open, structure, ddl, copy name, export… |
 | Ctrl+B | Hide the sidebar |
 
+A group's tables are drawn two cells in, with a `│` before them and
+without the group's prefix. The status line names the table under the
+cursor in full: `book_covers · table · 13 rows`.
+
 ### The start screen
 
 A window with no tab open shows the start screen in the main pane: the
```

```diff
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -111,7 +111,7 @@
 
 | Keys | Do |
 | --- | --- |
-| Arrows, Home, End, Enter | Move in the sidebar. Mod+Up and Mod+Down go to the top and the bottom, as Home and End do |
+| Arrows, Home, End, Enter | Move in the sidebar. Right opens a group, Left closes it or goes from a table up to its group. Mod+Up and Mod+Down go to the top and the bottom, as Home and End do |
 | Arrows, Page Up, Page Down, Home, End | Move in the grid |
 | Mod+Up, Mod+Down | First and last row, as Home and End |
 | Mod+Left, Mod+Right | First and last column |
```

- [ ] **Step 2: Run the whole suite and the docs build**

```bash
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: all pass (the database suites print "skipped" without their servers).

- [ ] **Step 3: Commit**

```bash
git add docs/_guide/browsing-data.md docs/_guide/omarchy.md docs/_reference/keyboard-shortcuts.md
git commit -m "Say how the sidebar's groups read and fold in the guide"
```

---

### Task 6: The pull request

- [ ] **Step 1: Push the branch and open the PR, linking #286**

The description: a summary; the layout rules (18 pt step, guide at the chevron's middle, highlight 4 past it, Omarchy two cells and `│`); the keys; the open questions and the defaults taken; the table "Where the build and the boards differ" above. No screenshot unless blurred; none is needed.

- [ ] **Step 2: Check the PR is open before any later push** (Igor merges on green).
