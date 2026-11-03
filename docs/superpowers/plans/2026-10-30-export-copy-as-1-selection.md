# Export and Copy as, run 1: the grid's selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A table's grid and a SQL result's share one selection: a rectangle of cells (a drag, Shift with a click or the arrows; the terminal look's `v` and `ctrl+v`) or whole rows (a row's number, with Shift and Command; Shift+Space; Mod+A; `V`). Each row shows its number in a gutter. The range is drawn as the Components boards have it, sized in the footer, the status line and to a screen reader, and ended by a filter, a sort, a page or a reload. Delete and Duplicate act on every row it covers, and a copy takes it as TSV. Runs 2 to 4 copy and export this selection.

**Architecture:** `src/selection.rs` holds the model, free of the UI: `Range` (`Cells` with an anchor, or `Rows` with picked rows and an anchor), `Held` and `Visual` for the terminal look's modes, `Reach` (moved from `model.rs`) and `RowClick`, the pure transitions (`moved`, `toggled`, `as_rows`, `row_clicked`, `all`) and `Selection`, which reads the rows, columns, size, rectangle and whole rows a cursor and a range cover in an `edit::Order`. `ObjectTab` and `SqlTab` hold a `range: Option<Range>` beside the cursor and wrap the transitions; `App` reduces the new actions (`ExtendBy { cols }`, `SelectRows { mode }`, `SelectRowsOfCursor`, `SelectAllRows`, `RowNumberClicked`) for both tabs. The keymap gains the commands, `When::VisualRows` and `Copy the selection`; `src/ui/keys.rs` reads them in both grids. `grid::show` takes a `Picked` (the rectangle and what a screen reader is told), draws the rows' numbers in its gutter, reports a number's click and a drag, and paints the range.

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-30-export-copy-as-1-selection-design.md`, with Igor's answers and the approved departures in it.

---

## Before Task 1

Commit the spec and this plan:

```bash
git add docs/superpowers/specs/2026-10-30-export-copy-as-1-selection-design.md docs/superpowers/plans/2026-10-30-export-copy-as-1-selection.md
git commit -m "Plan Export and Copy as, run 1: the grid's selection"
```

## Where this sits

The prompt asked for one pull request; you chose four runs, a pull request each:

- **Run 1 (this plan):** the selection, in both grids and both looks. A branch of `main` at `d1ed10aa`.
- **Run 2:** the six format writers, options remembered per format, Copy as (⌥⌘C, `gy`), the inspector's Copy row ▾, the clipboard limit.
- **Run 3:** export a table: the sheet and the float, the background export and its states, the table's entry points.
- **Run 4:** export a SQL result: Run again without the limit, the results bar, Export row…, several exports at once, the portal picker.

## Decisions and departures

Answered before the spec: a row-number column in both looks; `v` selects a rectangle as `ctrl+v` does; the SQL result's grid in this run. The spec's eight departures were approved as written (Shift makes a rectangle and Delete row takes its rows; the numbers in the gutter; `V` says `-- VISUAL LINE --`; `d` in `V` alone; Duplicate copies each selected row; Set to not wired; Mod+C copies the selection as TSV in this run; `v` is a mode). Building it decided these, for a yes or a no:

| # | The spec or the boards | Built | Why, and the other way |
|---|---|---|---|
| 1 | `y` while visual is Copy cell values | A command of its own, **Copy the selection** (`copy_selection`), `y` while visual. Copy cell values keeps `v y` as its written key, made of `v` and `y` | The status line's normal mode says `v y copy` and the visual one `y copy`: one command cannot be written both ways |
| 2 | ⌘-click toggles a row | On a row's number only. A Command-click on a cell selects the cell | The prompt puts it under Rows. The other way: Command-click on any cell toggles its row |
| 3 | (a Shift-click after Command-clicks) | Runs from the cursor's row to the clicked one, the picked rows kept | As Finder does |
| 4 | (the last row a Command-click takes out) | No range: the cursor's cell alone, as with none | Nothing selected would leave Delete row with no row |
| 4a | (where a Command-click leaves the cursor) | On the row clicked; where the click takes that row out, on the nearest row still picked. So the cursor's row is always a selected one | Delete row, Copy and Duplicate then agree on the rows; the review found Delete taking a row just taken out. The other way: the cursor stays where it was, and every action reads the picked rows alone |
| 4b | The spec's Command-click "(no key)" in the terminal look | Ctrl-click on a number toggles the row there too: the pointer does the same in every look | A mouse in the terminal look works as it does elsewhere. The other way: Ctrl-click selects the row alone |
| 5 | The design's cursor row under a range of cells | Not tinted: the rectangle and the cursor's ring say it. The cursor's cell keeps its own fill (the selection's on macOS, reverse video in the terminal) inside the rectangle. A range of rows is drawn as rows are today | The board draws the rectangle with the active cell ringed; the keyboard's cell stays the one that reads as lit |
| 6 | The row's bar (pending, new, deleted, selected) at the first cell's left | At the row's start, before its number | With the numbers before the first cell, a bar in the cell would stand between the number and the value |
| 7 | (a new row's number) | None: a new row has no number among the table's, and its mark says what it is | As the screen reader is told ("New row, not saved") |
| 8 | (a drag) | Runs while the pointer is held from a cell; a drag that starts on a number selects nothing. The grid does not scroll by itself under it | Rows scrolled out of view are not built; a drag that leaves the rows ends on the nearest |
| 9 | (one row's number clicked) | A range of rows: Mod+C copies the row whole, the footer says "1 row selected", and a screen reader "1 row selected" | It is what the person asked for |
| 10 | (a copy of a new row alone) | Mod+C and `yy` copy nothing from a new row with no range, as before. Within a range a new row's cells are copied, what is not set in them empty | `duplicate_tests.rs` keeps "the clipboard is left alone" for a new row |
| 11 | The status line in a SQL result | The result's visual mode and size stand after vim's mode on the editor's line. The result has no `d` | The line is the editor's while a SQL tab is in front |
| 11a | The macOS footer's words, "in a table and in a result" | A SQL result's footer keeps its rows and time; the size is told to a screen reader there, and the Omarchy line says it | The result's footer has no selection words today. The other way: "2 rows × 2 columns selected" after its time |
| 11d | The spec's `v` and `ctrl+v` "anchor: cursor" | A range that stands keeps its anchor: a Shift rectangle becomes the visual one from where it began, and switching between `v`, `ctrl+v` and `V` keeps it | As `V` does today with a Shift range, and as vim switches modes |
| 11b | Shift-click on a cell gives a rectangle | Over a range of rows Shift keeps rows, to the clicked cell's row | As a spreadsheet's Shift after a row header's click |
| 11c | "Another view" ends a range (the spec's first draft) | Another view ends a visual selection; a range made with Shift or a click stays, as today (`the_structure_view_leaves_a_visual_selection`) | The prompt names the filter, the sort and the page; the view's rule is an earlier spec's, kept |
| 12 | `accent-faint` | A palette colour, `accent_faint`: light `#eef2fc`, dark `#2a3248`, and for a theme file that sets the accent and not it, a tenth of the way from the window to the accent | The design system's token; Omarchy's range is `surface-active` |

## Files

| File | Task | What |
|---|---|---|
| `src/selection.rs` (new), `src/lib.rs` | 1 | The model and its unit tests |
| `src/model.rs` | 1 | `Range` in place of `RowRange`; `SqlTab::range`; the tabs' selection methods; the actions |
| `src/app.rs`, `src/app/editing.rs` | 1 | The reducers for both tabs, Copy as TSV, new rows that go, Duplicate of several rows |
| `src/ui/format.rs` | 1 | `tsv_cells` |
| `src/keymap.rs` | 1 | The commands, their keys, `When::VisualRows` |
| `src/ui/keys.rs`, `src/ui/workspace.rs`, `src/ui/data_view.rs` | 1, 2 | The keys in both grids; the status line; the footer and what is told; then the grid's new arguments and outputs |
| `src/ui/delete_many_tests.rs`, `src/ui/selection_tests.rs` (new), `src/ui/mod.rs` | 1, 2 | Behaviour, headless |
| `src/ui/grid.rs`, `src/theme.rs`, `src/ui/sql_results.rs`, `src/ui/settings/preview.rs`, `src/testing.rs` | 2 | The numbers, the clicks, the drag, the range drawn; `accent_faint`; `Harness::is_row_number` |
| `docs/_guide/{browsing-data,editing-data,macos,omarchy}.md`, `docs/_reference/keyboard-shortcuts.md` | 2 | The website |

The tests run as before: `cargo test --locked --workspace --all-targets`; no server is needed. Use `~/.cargo/bin/cargo` where the mise shim fails.

---

### Task 1: The selection, its keys and its words

**Files:** `src/selection.rs` (new), `src/lib.rs`, `src/model.rs`, `src/app.rs`, `src/app/editing.rs`, `src/ui/format.rs`, `src/keymap.rs`, `src/ui/keys.rs`, `src/ui/workspace.rs`, `src/ui/data_view.rs`, `src/ui/delete_many_tests.rs`, `src/ui/selection_tests.rs` (new), `src/ui/mod.rs`.

The model and everything that reads it but the grid's drawing: Shift with the arrows makes a rectangle in both grids, Shift+Space and Mod+A whole rows, the terminal look's `v`, `ctrl+v` and `V` their modes; the footer, the status line and the screen reader's "Visual." say the size; Delete row and Duplicate row take every row; Mod+C, Mod+Shift+C and `y` copy as TSV. A table's grid draws a range of rows as before and a rectangle not yet; a SQL result's draws neither, and takes no Shift-click, though its keys select: Task 2 draws them and reads the clicks.

- [ ] **Step 1: Write the failing tests**

`src/ui/selection_tests.rs`:

```diff
diff --git a/src/ui/selection_tests.rs b/src/ui/selection_tests.rs
new file mode 100644
index 00000000..2be7f0d4
--- /dev/null
+++ b/src/ui/selection_tests.rs
@@ -0,0 +1,405 @@
+//! The grid's selection, checked against its spec
+//! (`docs/superpowers/specs/2026-10-30-export-copy-as-1-selection-design.md`:
+//! "test N" below is an entry of its Tests).
+//!
+//! Behaviour, names and words, on the Bookshop's `book_covers` and on a SQL
+//! result; the colours only as the palette names them. No picture of the
+//! design is compared.
+
+use egui::{Key, Modifiers};
+use tabletist_db::Value;
+
+use super::insert_audit_tests::{covers_in, edits, painted, select, set, type_key};
+use crate::model::{Action, CellPos, ConnTabId, RowClick, TabId};
+use crate::testing::Harness;
+use crate::theme::Look;
+
+/// The desktop looks: the keys and the clicks are the same in both.
+fn desktop() -> [Look; 2] {
+    [Look::standard(), Look::macos()]
+}
+
+/// The selected rows of the table's tab, in the grid's order.
+fn rows(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<usize> {
+    let workspace = harness.app.workspace(tab).unwrap();
+    workspace.object_tab(id).unwrap().selected_rows()
+}
+
+/// The first and the last column the table's selection covers.
+fn cols(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<(usize, usize)> {
+    let workspace = harness.app.workspace(tab).unwrap();
+    workspace.object_tab(id).unwrap().selected_cols()
+}
+
+/// Whether the table's range is of whole rows.
+fn whole_rows(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
+    let workspace = harness.app.workspace(tab).unwrap();
+    let object = workspace.object_tab(id).unwrap();
+    object
+        .range
+        .as_ref()
+        .is_some_and(crate::selection::Range::rows)
+}
+
+/// The keyboard's copy, with Shift held for the whole rows.
+fn copied(harness: &mut Harness, shift: bool) -> Option<String> {
+    harness.copied = None;
+    harness.copy(shift);
+    harness.copied.clone()
+}
+
+/// The terminal look's `ctrl+v`, as the window sends it: a paste with
+/// Control held.
+fn ctrl_v(harness: &mut Harness) {
+    harness.settle();
+    harness.frame(vec![
+        egui::Event::ModifiersChanged(Modifiers::CTRL),
+        egui::Event::Paste(String::new()),
+    ]);
+    harness.frame(vec![egui::Event::ModifiersChanged(Modifiers::NONE)]);
+    harness.settle();
+}
+
+/// What a click on the number of `row` does, with the keys `click` says
+/// were held.
+fn pick_row(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize, click: RowClick) {
+    let action = crate::model::Action::RowNumberClicked {
+        tab,
+        id,
+        row,
+        click,
+    };
+    harness.app.apply(action);
+    harness.settle();
+}
+
+/// Test 2: Shift with the arrows selects a rectangle of cells, which the
+/// footer sizes and a copy takes as TSV.
+#[test]
+fn shift_and_the_arrows_select_a_rectangle_of_cells() {
+    for look in desktop() {
+        let (mut harness, tab, id) = covers_in(look);
+        select(&mut harness, tab, id, (0, 1));
+        harness.press(Key::ArrowDown, Modifiers::SHIFT);
+        harness.press(Key::ArrowDown, Modifiers::SHIFT);
+        harness.press(Key::ArrowRight, Modifiers::SHIFT);
+        assert_eq!(rows(&harness, tab, id), [0, 1, 2], "{}", look.name);
+        assert_eq!(cols(&harness, tab, id), Some((1, 2)), "{}", look.name);
+        assert!(!whole_rows(&harness, tab, id));
+        assert!(
+            painted(&harness, "3 rows × 2 columns selected"),
+            "{}",
+            look.name
+        );
+        let tsv =
+            "9100000000000000001\tprint\n9100000000000000002\tebook\n9100000000000000003\tprint";
+        assert_eq!(
+            copied(&mut harness, false).as_deref(),
+            Some(tsv),
+            "{}",
+            look.name
+        );
+        // Shift with Left takes the column back; a plain arrow ends it.
+        harness.press(Key::ArrowLeft, Modifiers::SHIFT);
+        assert_eq!(cols(&harness, tab, id), Some((1, 1)));
+        harness.press(Key::ArrowUp, Modifiers::NONE);
+        assert_eq!(rows(&harness, tab, id), [1]);
+        // One cell copies its text, as it always did.
+        assert_eq!(
+            copied(&mut harness, false).as_deref(),
+            Some("9100000000000000002")
+        );
+    }
+}
+
+/// Test 2: Shift with Space makes the selection whole rows, and Mod+A
+/// selects every row loaded.
+#[test]
+fn shift_space_and_select_all_select_whole_rows() {
+    for look in desktop() {
+        let (mut harness, tab, id) = covers_in(look);
+        select(&mut harness, tab, id, (1, 1));
+        harness.press(Key::ArrowDown, Modifiers::SHIFT);
+        harness.press(Key::Space, Modifiers::SHIFT);
+        assert_eq!(rows(&harness, tab, id), [1, 2], "{}", look.name);
+        assert!(whole_rows(&harness, tab, id), "{}", look.name);
+        harness.press(Key::A, Modifiers::COMMAND);
+        assert_eq!(rows(&harness, tab, id), [0, 1, 2], "{}", look.name);
+        assert!(painted(&harness, "3 rows selected"), "{}", look.name);
+        // A plain move ends it.
+        harness.press(Key::ArrowUp, Modifiers::NONE);
+        assert_eq!(rows(&harness, tab, id), [1], "{}", look.name);
+    }
+}
+
+/// Test 5: delete and duplicate act on every selected row: a rectangle's
+/// rows, or the picked ones.
+#[test]
+fn delete_and_duplicate_take_every_selected_row() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    // A rectangle's rows are deleted whole.
+    select(&mut harness, tab, id, (0, 2));
+    harness.press(Key::ArrowDown, Modifiers::SHIFT);
+    harness.press(Key::ArrowRight, Modifiers::SHIFT);
+    harness.press(Key::Backspace, Modifiers::NONE);
+    let marked: Vec<usize> = edits(&harness, tab, id).deleted.keys().copied().collect();
+    assert_eq!(marked, [0, 1]);
+    harness.press(Key::Backspace, Modifiers::NONE);
+    assert!(edits(&harness, tab, id).deleted.is_empty());
+    // Two rows picked apart are copied in their order, at the top.
+    pick_row(&mut harness, tab, id, 0, RowClick::Only);
+    pick_row(&mut harness, tab, id, 1, RowClick::Toggle);
+    harness.press(Key::D, Modifiers::COMMAND);
+    harness.settle();
+    let workspace = harness.app.workspace(tab).unwrap();
+    let object = workspace.object_tab(id).unwrap();
+    assert_eq!(object.edits.added.len(), 2);
+    let order = object.order();
+    let kind = |place: usize| {
+        let row = order.row(place).unwrap();
+        let pending = object.edits.cells.get(&(row, 2)).map(|cell| &cell.new);
+        pending.map(crate::edit::shown_value)
+    };
+    assert_eq!(kind(0), Some(Value::Text("print".into())));
+    assert_eq!(kind(1), Some(Value::Text("ebook".into())));
+    // One editor, on the first copy.
+    let first = order.row(0).unwrap();
+    let editor = object.edits.editor.as_ref().map(|editor| editor.cell.row);
+    assert!(editor.is_none_or(|row| row == first), "{editor:?}");
+}
+
+/// Test 3 and 4: the terminal look's `v`, `ctrl+v` and `V` start their
+/// modes, the status line says each with its size and keys, `y` copies
+/// and leaves, and Esc leaves.
+#[test]
+fn the_visual_modes_are_said_and_copy() {
+    let (mut harness, tab, id) = covers_in(Look::omarchy());
+    harness.app.workspace_mut(tab).unwrap().row_panel = false;
+    select(&mut harness, tab, id, (0, 1));
+    type_key(&mut harness, Key::V, "v");
+    assert!(painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
+    assert!(painted(&harness, "1 row × 1 column"));
+    assert!(painted(&harness, "y copy · esc normal"));
+    type_key(&mut harness, Key::J, "j");
+    type_key(&mut harness, Key::L, "l");
+    type_key(&mut harness, Key::L, "l");
+    type_key(&mut harness, Key::H, "h");
+    assert!(painted(&harness, "2 rows × 2 columns"));
+    // A block of cells has no `d`, and `dd` acts on one row: held back,
+    // nothing is deleted.
+    type_key(&mut harness, Key::D, "d");
+    type_key(&mut harness, Key::D, "d");
+    assert!(edits(&harness, tab, id).deleted.is_empty());
+    assert!(painted(&harness, "-- VISUAL --"));
+    harness.copied = None;
+    type_key(&mut harness, Key::Y, "y");
+    let tsv = "9100000000000000001\tprint\n9100000000000000002\tebook";
+    assert_eq!(harness.copied.as_deref(), Some(tsv));
+    assert!(!painted(&harness, "-- VISUAL --"));
+    // The block, from the chord the window sends as a paste.
+    ctrl_v(&mut harness);
+    assert!(
+        painted(&harness, "-- VISUAL BLOCK --"),
+        "{:?}",
+        harness.painted
+    );
+    type_key(&mut harness, Key::K, "k");
+    assert!(painted(&harness, "2 rows × 1 column"));
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert!(!painted(&harness, "-- VISUAL BLOCK --"));
+    // Rows: their own name, and `d` deletes them.
+    type_key(&mut harness, Key::V, "V");
+    assert!(painted(&harness, "-- VISUAL LINE --"));
+    assert!(painted(&harness, "1 row"));
+    assert!(painted(&harness, "d delete · esc normal"));
+    type_key(&mut harness, Key::J, "j");
+    assert!(painted(&harness, "2 rows"));
+    type_key(&mut harness, Key::D, "d");
+    let marked: Vec<usize> = edits(&harness, tab, id).deleted.keys().copied().collect();
+    assert_eq!(marked, [0, 1]);
+}
+
+/// Test 3: `v y` copies the cursor's cell, as it did before `v` was a mode.
+#[test]
+fn v_then_y_still_copies_one_cell() {
+    let (mut harness, tab, id) = covers_in(Look::omarchy());
+    select(&mut harness, tab, id, (1, 2));
+    harness.copied = None;
+    type_key(&mut harness, Key::V, "v");
+    type_key(&mut harness, Key::Y, "y");
+    assert_eq!(harness.copied.as_deref(), Some("ebook"));
+    let workspace = harness.app.workspace(tab).unwrap();
+    assert!(!workspace.object_tab(id).unwrap().visual());
+    // `V` then `y` copies the rows whole.
+    harness.copied = None;
+    type_key(&mut harness, Key::V, "V");
+    type_key(&mut harness, Key::J, "j");
+    type_key(&mut harness, Key::Y, "y");
+    let copied = harness.copied.clone().expect("a copy");
+    let lines: Vec<&str> = copied.lines().collect();
+    assert_eq!(lines.len(), 2, "{copied}");
+    assert!(
+        lines.iter().all(|line| line.split('\t').count() == 5),
+        "{copied}"
+    );
+}
+
+/// What selects is dropped under the question about pending changes, as
+/// the moves are: each closes an editor.
+#[test]
+fn the_selection_stays_under_a_prompt_about_the_changes() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    set(&mut harness, tab, id, (0, 2), "audio");
+    select(&mut harness, tab, id, (1, 1));
+    harness
+        .app
+        .apply(crate::model::Action::CloseTab { tab, id });
+    let click = RowClick::Only;
+    let row = 2;
+    for action in [
+        crate::model::Action::RowNumberClicked {
+            tab,
+            id,
+            row,
+            click,
+        },
+        crate::model::Action::SelectAllRows { tab, id },
+        crate::model::Action::SelectRowsOfCursor { tab, id },
+    ] {
+        harness.app.apply(action);
+    }
+    let workspace = harness.app.workspace(tab).unwrap();
+    let object = workspace
+        .object_tab(id)
+        .expect("the tab, under its question");
+    assert_eq!(object.selection, Some(CellPos { row: 1, col: 1 }));
+    assert_eq!(object.range, None);
+}
+
+/// Test 4: whatever loads another page of rows leaves no range.
+#[test]
+fn a_filter_a_sort_and_a_page_leave_no_range() {
+    let range = |harness: &Harness, tab: ConnTabId, id: TabId| {
+        let workspace = harness.app.workspace(tab).unwrap();
+        workspace.object_tab(id).unwrap().range.clone()
+    };
+    let changes: [fn(&mut Harness, ConnTabId, TabId); 4] = [
+        |harness, tab, object_tab| {
+            let column = "kind".to_owned();
+            harness.app.apply(crate::model::Action::SortBy {
+                tab,
+                object_tab,
+                column,
+            });
+        },
+        |harness, tab, _| {
+            harness.app.apply(crate::model::Action::Refresh(tab));
+            harness.answer_rows(crate::testing::book_covers_page(3));
+        },
+        |harness, tab, object_tab| {
+            // A page with more after it.
+            let workspace = harness.app.workspace_mut(tab).unwrap();
+            let object = workspace.object_tab_mut(object_tab).unwrap();
+            object.rows.value.as_mut().unwrap().has_more = true;
+            harness.app.apply(Action::NextPage { tab, object_tab });
+        },
+        |harness, tab, id| {
+            let workspace = harness.app.workspace_mut(tab).unwrap();
+            let object = workspace.object_tab_mut(id).unwrap();
+            object.filter.raw = true;
+            object.filter.raw_text = "kind = 'print'".into();
+            harness.app.apply(crate::model::Action::ApplyFilters {
+                tab,
+                object_tab: id,
+            });
+        },
+    ];
+    for (index, change) in changes.into_iter().enumerate() {
+        let (mut harness, tab, id) = covers_in(Look::macos());
+        pick_row(&mut harness, tab, id, 0, RowClick::Only);
+        pick_row(&mut harness, tab, id, 2, RowClick::Extend);
+        assert!(range(&harness, tab, id).is_some());
+        change(&mut harness, tab, id);
+        harness.settle();
+        assert_eq!(range(&harness, tab, id), None, "{index}");
+    }
+}
+
+/// Test 5: a Command-click puts the cursor on the row clicked, and one
+/// that takes the row out gives the cursor to the nearest row left: Delete
+/// row takes that row, not the one taken out.
+#[test]
+fn delete_takes_the_picked_row_after_the_cursors_was_taken_out() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    pick_row(&mut harness, tab, id, 0, RowClick::Only);
+    pick_row(&mut harness, tab, id, 2, RowClick::Toggle);
+    pick_row(&mut harness, tab, id, 0, RowClick::Toggle);
+    assert_eq!(rows(&harness, tab, id), [2]);
+    let workspace = harness.app.workspace(tab).unwrap();
+    let cursor = workspace.object_tab(id).unwrap().selection;
+    assert_eq!(cursor.map(|cell| cell.row), Some(2));
+    harness.press(Key::Backspace, Modifiers::NONE);
+    let marked: Vec<usize> = edits(&harness, tab, id).deleted.keys().copied().collect();
+    assert_eq!(marked, [2]);
+}
+
+/// Test 3: `v y` copies one cell's text as it is, tabs and lines in it
+/// kept, as Mod+C does on one cell.
+#[test]
+fn one_cell_is_copied_as_it_is() {
+    let text = "line one\nline\ttwo";
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, tab, id) = covers_in(look);
+        harness.app.workspace_mut(tab).unwrap().row_panel = false;
+        set(&mut harness, tab, id, (1, 2), text);
+        select(&mut harness, tab, id, (1, 2));
+        harness.copied = None;
+        if look.terminal {
+            type_key(&mut harness, Key::V, "v");
+            type_key(&mut harness, Key::Y, "y");
+        } else {
+            harness.copy(false);
+        }
+        assert_eq!(harness.copied.as_deref(), Some(text), "{}", look.name);
+    }
+}
+
+/// Test 3: a SQL result's visual selection is said on the editor's line,
+/// with its keys in place of the editor's, and not under the Messages
+/// pane.
+#[test]
+fn a_results_visual_selection_is_said_with_its_keys() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    let tab = harness.connect_fake();
+    let id = super::tests::with_sql_result(&mut harness, tab, 3);
+    // The pointer on the first row, which gives the result's rows the
+    // keys.
+    let tree = harness.settle();
+    let row = crate::testing::bounds(&tree, "Row 1", egui::accesskit::Role::Button).unwrap();
+    let press = |pressed| egui::Event::PointerButton {
+        pos: row.center(),
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![egui::Event::PointerMoved(row.center())]);
+    harness.frame(vec![press(true)]);
+    harness.frame(vec![press(false)]);
+    type_key(&mut harness, Key::V, "v");
+    assert!(painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
+    assert!(painted(&harness, "1 row × 1 column"));
+    assert!(
+        painted(&harness, "y copy · esc normal"),
+        "{:?}",
+        harness.painted
+    );
+    let pane = crate::model::ResultPane::Messages;
+    harness.app.apply(Action::SetResultPane {
+        tab,
+        sql_tab: id,
+        pane,
+    });
+    harness.settle();
+    assert!(!painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
+}
```

`src/ui/mod.rs`:

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index af86bd56..20ada614 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -96,6 +96,8 @@ mod quick_open_tests;
 pub mod review;
 pub mod row_form;
 pub mod row_panel;
+#[cfg(test)]
+mod selection_tests;
 pub mod settings;
 #[cfg(test)]
 mod settings_access_tests;
```

`src/ui/delete_many_tests.rs`:

```diff
diff --git a/src/ui/delete_many_tests.rs b/src/ui/delete_many_tests.rs
index d0102baa..10d55a4c 100644
--- a/src/ui/delete_many_tests.rs
+++ b/src/ui/delete_many_tests.rs
@@ -48,7 +48,12 @@ fn rows(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<usize> {
 fn select_rows(harness: &mut Harness, tab: ConnTabId, id: TabId, from: usize, to: usize) {
     select(harness, tab, id, (from, 1));
     let rows = to as isize - from as isize;
-    harness.app.apply(Action::ExtendBy { tab, id, rows });
+    harness.app.apply(Action::ExtendBy {
+        tab,
+        id,
+        rows,
+        cols: 0,
+    });
     harness.settle();
 }
 
@@ -103,7 +108,7 @@ fn a_range_ends_with_its_page_and_with_what_works_on_one_row() {
     };
     let range = |harness: &Harness, tab: ConnTabId, id: TabId| {
         let workspace = harness.app.workspace(tab).unwrap();
-        workspace.object_tab(id).unwrap().range
+        workspace.object_tab(id).unwrap().range.clone()
     };
     // A reload.
     let (mut harness, tab, id) = ranged();
@@ -148,14 +153,18 @@ fn a_range_ends_with_its_page_and_with_what_works_on_one_row() {
 }
 
 /// Spec 1 and tests 1 and 11: the footer counts the selected rows, and
-/// each row of a range is told to a screen reader as selected.
+/// each row of a range of rows is told to a screen reader as selected.
 #[test]
 fn a_range_is_counted_in_the_footer_and_told_as_selected() {
     for look in desktop() {
         let (mut harness, tab, id) = covers_with(look, book_covers_page(5));
         select(&mut harness, tab, id, (1, 2));
         assert!(painted(&harness, "1 row selected"));
+        // Shift with the arrows selects cells: their rows and columns.
         select_rows(&mut harness, tab, id, 1, 3);
+        assert!(painted(&harness, "3 rows × 1 column selected"));
+        // Shift with Space makes them whole rows.
+        harness.press(Key::Space, Modifiers::SHIFT);
         assert!(painted(&harness, "3 rows selected"));
         let tree = harness.settle();
         let told = |label: &str| {
@@ -246,7 +255,12 @@ fn new_rows_of_a_range_are_dropped_with_the_mark() {
     // A new row under row 0, and the range from it down to row 2.
     let new = add_below(&mut harness, tab, id, 0);
     select(&mut harness, tab, id, (new, 1));
-    harness.app.apply(Action::ExtendBy { tab, id, rows: 2 });
+    harness.app.apply(Action::ExtendBy {
+        tab,
+        id,
+        rows: 2,
+        cols: 0,
+    });
     assert_eq!(rows(&harness, tab, id), [new, 1, 2]);
     harness.press(Key::Backspace, Modifiers::NONE);
     assert!(edits(&harness, tab, id).added.is_empty());
@@ -261,7 +275,12 @@ fn new_rows_of_a_range_are_dropped_with_the_mark() {
     // 0 again, and with it rows 1 and 2, which are both marked.
     let new = add_below(&mut harness, tab, id, 0);
     select(&mut harness, tab, id, (new, 1));
-    harness.app.apply(Action::ExtendBy { tab, id, rows: 2 });
+    harness.app.apply(Action::ExtendBy {
+        tab,
+        id,
+        rows: 2,
+        cols: 0,
+    });
     assert_eq!(rows(&harness, tab, id), [new, 1, 2]);
     harness.press(Key::Backspace, Modifiers::NONE);
     assert_eq!(edits(&harness, tab, id).added.len(), 1);
@@ -276,7 +295,12 @@ fn a_range_of_new_rows_alone_drops_them() {
     add(&mut harness, tab, id);
     let first = crate::edit::new_row(edits(&harness, tab, id).added[0].id);
     select(&mut harness, tab, id, (first, 1));
-    harness.app.apply(Action::ExtendBy { tab, id, rows: 1 });
+    harness.app.apply(Action::ExtendBy {
+        tab,
+        id,
+        rows: 1,
+        cols: 0,
+    });
     assert_eq!(rows(&harness, tab, id).len(), 2);
     harness.press(Key::Backspace, Modifiers::NONE);
     assert!(edits(&harness, tab, id).added.is_empty());
@@ -382,7 +406,11 @@ fn move_rows(harness: &mut Harness, tab: ConnTabId, id: TabId, rows: isize) {
 fn a_visual_selections_delete_marks_its_rows_and_leaves_it() {
     let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
     select(&mut harness, tab, id, (3, 2));
-    harness.app.apply(Action::SelectRows { tab, id });
+    harness.app.apply(Action::SelectRows {
+        tab,
+        id,
+        mode: crate::model::Visual::Rows,
+    });
     assert!(visual(&harness, tab, id));
     // Up from the anchor: the first row of the range is the cursor's.
     move_rows(&mut harness, tab, id, -2);
@@ -394,13 +422,21 @@ fn a_visual_selections_delete_marks_its_rows_and_leaves_it() {
     assert_eq!(rows(&harness, tab, id), [1]);
     // Down from the anchor: the cursor goes back to the first row.
     select(&mut harness, tab, id, (1, 2));
-    harness.app.apply(Action::SelectRows { tab, id });
+    harness.app.apply(Action::SelectRows {
+        tab,
+        id,
+        mode: crate::model::Visual::Rows,
+    });
     move_rows(&mut harness, tab, id, 2);
     harness.app.apply(Action::DeleteSelected { tab, id });
     assert!(marked(&harness, tab, id).is_empty());
     assert_eq!(selected(&harness, tab, id), Some((1, 2)));
     // Left, it marks nothing and the cursor stays.
-    harness.app.apply(Action::SelectRows { tab, id });
+    harness.app.apply(Action::SelectRows {
+        tab,
+        id,
+        mode: crate::model::Visual::Rows,
+    });
     move_rows(&mut harness, tab, id, 2);
     harness.app.apply(Action::LeaveVisual { tab, id });
     assert!(!visual(&harness, tab, id));
@@ -415,7 +451,11 @@ fn a_visual_selections_delete_lands_on_the_first_row_left() {
     let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(4));
     let new = add_below(&mut harness, tab, id, 0);
     select(&mut harness, tab, id, (new, 1));
-    harness.app.apply(Action::SelectRows { tab, id });
+    harness.app.apply(Action::SelectRows {
+        tab,
+        id,
+        mode: crate::model::Visual::Rows,
+    });
     move_rows(&mut harness, tab, id, 2);
     assert_eq!(rows(&harness, tab, id), [new, 1, 2]);
     harness.app.apply(Action::DeleteSelected { tab, id });
@@ -555,9 +595,9 @@ fn a_visual_selection_shows_in_the_status_line_and_is_told() {
     let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
     harness.app.workspace_mut(tab).unwrap().row_panel = false;
     select(&mut harness, tab, id, (1, 2));
-    assert!(!painted(&harness, "-- VISUAL --"));
+    assert!(!painted(&harness, "-- VISUAL LINE --"));
     type_key(&mut harness, Key::V, "V");
-    assert!(painted(&harness, "-- VISUAL --"));
+    assert!(painted(&harness, "-- VISUAL LINE --"));
     assert!(painted(&harness, "1 row"));
     assert!(polite(&mut harness, "Visual. 1 row selected."));
     type_key(&mut harness, Key::J, "j");
@@ -567,13 +607,13 @@ fn a_visual_selection_shows_in_the_status_line_and_is_told() {
     // The mode is in the palette's magenta, as the board's is.
     let magenta = harness.app.palette.magenta;
     let mode =
-        |(text, color): &(String, egui::Color32)| text == "-- VISUAL --" && *color == magenta;
+        |(text, color): &(String, egui::Color32)| text == "-- VISUAL LINE --" && *color == magenta;
     assert!(harness.painted.iter().any(mode), "{:?}", harness.painted);
     // Its keys, one piece, in place of the grid's.
     assert!(painted(&harness, "d delete · esc normal"));
     assert!(!painted(&harness, "dd delete"));
     harness.press(Key::Escape, Modifiers::NONE);
-    assert!(!painted(&harness, "-- VISUAL --"));
+    assert!(!painted(&harness, "-- VISUAL LINE --"));
     assert!(painted(&harness, "dd delete"));
 }
 
@@ -590,7 +630,7 @@ fn a_refused_delete_is_said_on_the_visual_line() {
     type_key(&mut harness, Key::D, "d");
     assert!(visual(&harness, tab, id));
     assert!(marked(&harness, tab, id).is_empty());
-    assert!(painted(&harness, "-- VISUAL --"));
+    assert!(painted(&harness, "-- VISUAL LINE --"));
     assert!(painted(&harness, "read-only connection"));
     assert!(!painted(&harness, "d delete · esc normal"));
 }
```

`src/model.rs`:

```diff
diff --git a/src/model.rs b/src/model.rs
index f6b1f1e3..eaa38738 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -6259,25 +6259,30 @@ mod tests {
     }
 
     #[test]
-    fn an_extend_selects_the_rows_from_the_anchor_to_the_cursor() {
+    fn an_extend_selects_the_cells_from_the_anchor_to_the_cursor() {
         let mut tab = covers(5);
         tab.set_cursor(at(1), Reach::End);
         assert_eq!(tab.selected_rows(), [1]);
         tab.set_cursor(at(3), Reach::Extend);
         assert_eq!(tab.selected_rows(), [1, 2, 3]);
+        assert_eq!(tab.selected_cols(), Some((1, 1)));
+        assert_eq!(tab.selection_size(), Some((3, 1)));
+        tab.set_cursor(Some(CellPos { row: 3, col: 3 }), Reach::Extend);
+        assert_eq!(tab.selected_cols(), Some((1, 3)));
+        tab.set_cursor(at(3), Reach::Extend);
         // The anchor stays where the range began.
         tab.set_cursor(at(0), Reach::Extend);
         assert_eq!(tab.selected_rows(), [0, 1]);
         // Back on the anchor one row is selected, and no range stands.
         tab.set_cursor(at(1), Reach::Extend);
-        assert_eq!((tab.selected_rows(), tab.range), (vec![1], None));
+        assert_eq!((tab.selected_rows(), tab.range.clone()), (vec![1], None));
     }
 
     #[test]
     fn with_no_cursor_an_extend_selects_one_row() {
         let mut tab = covers(5);
         tab.set_cursor(at(2), Reach::Extend);
-        assert_eq!((tab.selected_rows(), tab.range), (vec![2], None));
+        assert_eq!((tab.selected_rows(), tab.range.clone()), (vec![2], None));
     }
 
     #[test]
@@ -6286,10 +6291,11 @@ mod tests {
         tab.set_cursor(at(1), Reach::End);
         tab.set_cursor(at(2), Reach::Extend);
         tab.set_cursor(at(3), Reach::Move);
-        assert_eq!((tab.selected_rows(), tab.range), (vec![3], None));
+        assert_eq!((tab.selected_rows(), tab.range.clone()), (vec![3], None));
         // A held range follows the cursor, along the row too.
-        tab.range = Some(RowRange {
-            anchor: 3,
+        tab.range = Some(Range::Rows {
+            anchor: Some(3),
+            picked: Default::default(),
             held: true,
         });
         tab.set_cursor(at(1), Reach::Move);
@@ -6301,10 +6307,10 @@ mod tests {
         assert!(tab.range.is_some());
         // Anything else ends it, and so does a cursor that goes.
         tab.set_cursor(at(2), Reach::End);
-        assert_eq!(tab.range, None);
+        assert_eq!(tab.range.clone(), None);
         tab.set_cursor(at(1), Reach::Extend);
         tab.set_cursor(None, Reach::Keep);
-        assert_eq!((tab.selected_rows(), tab.range), (Vec::new(), None));
+        assert_eq!((tab.selected_rows(), tab.range.clone()), (Vec::new(), None));
     }
 
     #[test]
@@ -6314,7 +6320,10 @@ mod tests {
         tab.set_cursor(at(0), Reach::End);
         tab.set_cursor(at(1), Reach::Extend);
         assert_eq!(tab.selected_rows(), [0, crate::edit::new_row(new), 1]);
-        assert_eq!(tab.selected_span(), Some((0, 2)));
+        assert_eq!(
+            tab.selected().map(|selected| selected.places()),
+            Some(vec![0, 1, 2])
+        );
     }
 
     #[test]
@@ -6326,17 +6335,17 @@ mod tests {
         assert_eq!(tab.selected_rows(), [new, 1, 2]);
         tab.edits = crate::edit::Edits::default();
         tab.off_new_rows();
-        assert_eq!((tab.selected_rows(), tab.range), (vec![2], None));
+        assert_eq!((tab.selected_rows(), tab.range.clone()), (vec![2], None));
     }
 
     #[test]
     fn a_visual_selection_is_started_and_left_on_the_cursors_row() {
         let mut tab = covers(5);
         // With no cursor there is none to start.
-        tab.toggle_visual();
+        tab.toggle_visual(Visual::Rows);
         assert!(!tab.visual());
         tab.set_cursor(at(2), Reach::End);
-        tab.toggle_visual();
+        tab.toggle_visual(Visual::Rows);
         assert!(tab.visual());
         assert_eq!(tab.selected_rows(), [2]);
         // Moves extend it, up and down.
@@ -6344,11 +6353,11 @@ mod tests {
         assert_eq!(tab.selected_rows(), [2, 3, 4]);
         tab.set_cursor(at(0), Reach::Move);
         assert_eq!(tab.selected_rows(), [0, 1, 2]);
-        tab.toggle_visual();
+        tab.toggle_visual(Visual::Rows);
         assert_eq!((tab.visual(), tab.selected_rows()), (false, vec![0]));
         // A range made with Shift becomes the visual one, from its anchor.
         tab.set_cursor(at(1), Reach::Extend);
-        tab.toggle_visual();
+        tab.toggle_visual(Visual::Rows);
         assert!(tab.visual());
         tab.set_cursor(at(3), Reach::Move);
         assert_eq!(tab.selected_rows(), [0, 1, 2, 3]);
```

`src/keymap.rs`:

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
index 80f80279..38612faa 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -4776,8 +4776,9 @@ mod tests {
             command(Command::OpenReferencedRow)
         );
         assert_eq!(omarchy(Scope::Grid, "gs"), Claimed);
-        assert_eq!(omarchy(Scope::Grid, "vy"), command(Command::CopyCells));
-        assert_eq!(omarchy(Scope::Grid, "vp"), Claimed);
+        // `v` starts a selection of cells, which `y` copies.
+        assert_eq!(omarchy(Scope::Grid, "v"), command(Command::SelectCells));
+        assert_eq!(omarchy(Scope::Grid, "y"), Waiting);
         assert_eq!(omarchy(Scope::Grid, "\"+p"), Claimed);
         assert_eq!(omarchy(Scope::Grid, "D"), command(Command::SetDefault));
         assert_eq!(omarchy(Scope::Grid, "R"), command(Command::Reload));
@@ -4838,19 +4839,48 @@ mod tests {
     #[test]
     fn a_visual_selection_has_its_own_keys() {
         let keymap = Keymap::default();
-        let visual = |when: When| when == When::Visual;
+        let rows = |when: When| when == When::Visual || when == When::VisualRows;
+        let cells = |when: When| when == When::Visual;
         let nothing = |_: When| false;
         let grid = |holds: &dyn Fn(When) -> bool, typed: &str| {
             keymap.typed(Layout::Omarchy, Scope::Grid, holds, typed)
         };
         assert_eq!(grid(&nothing, "V"), Typed::Command(Command::SelectRows));
-        // `d` alone is the whole key while a selection is on, and the
-        // first of two without one.
-        assert_eq!(grid(&visual, "d"), Typed::Command(Command::DeleteSelection));
+        assert_eq!(grid(&nothing, "v"), Typed::Command(Command::SelectCells));
+        // `d` alone is the whole key while a selection of rows is on, and
+        // the first of two without one. A selection of cells has none.
+        assert_eq!(grid(&rows, "d"), Typed::Command(Command::DeleteSelection));
+        assert_eq!(grid(&cells, "d"), Typed::Waiting);
         assert_eq!(grid(&nothing, "d"), Typed::Waiting);
         assert_eq!(grid(&nothing, "dd"), Typed::Command(Command::DeleteRow));
+        // `y` alone copies a selection, and is the first of `yy` without.
+        for holds in [&rows as &dyn Fn(When) -> bool, &cells] {
+            assert_eq!(grid(holds, "y"), Typed::Command(Command::CopySelection));
+            // Pasting is not built: the key is claimed.
+            assert_eq!(grid(holds, "p"), Typed::Claimed);
+        }
+        assert_eq!(grid(&nothing, "y"), Typed::Waiting);
+        assert_eq!(grid(&nothing, "yy"), Typed::Command(Command::YankRow));
+        assert_eq!(grid(&nothing, "p"), Typed::Command(Command::PutRow));
+        // A result's rows have the same selections, and copy them.
+        let results = |typed: &str| keymap.typed(Layout::Omarchy, Scope::Results, &cells, typed);
+        assert_eq!(results("v"), Typed::Command(Command::SelectCells));
+        assert_eq!(results("V"), Typed::Command(Command::SelectRows));
+        assert_eq!(results("y"), Typed::Command(Command::CopySelection));
         assert_eq!(keymap.label(Layout::Omarchy, Command::LeaveVisual), "esc");
+        assert_eq!(
+            keymap.label(Layout::Omarchy, Command::SelectBlock),
+            "ctrl+v"
+        );
+        assert_eq!(keymap.label(Layout::Omarchy, Command::CopyCells), "v y");
+        assert_eq!(keymap.label(Layout::Omarchy, Command::CopySelection), "y");
         assert_eq!(keymap.label(Layout::Mac, Command::SelectRows), "");
+        assert_eq!(
+            keymap.label(Layout::Mac, Command::SelectRowsOfCursor),
+            "⇧Space"
+        );
+        assert_eq!(keymap.label(Layout::Mac, Command::SelectAllRows), "⌘A");
+        assert_eq!(keymap.label(Layout::Mac, Command::ExtendRight), "⇧→");
     }
 
     #[test]
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib`

Expected: the tests do not compile, as the model is not there yet:

```
error[E0432]: unresolved import `crate::model::RowClick`
error[E0433]: cannot find `Visual` in `model`
error[E0433]: cannot find type `Range` in this scope
error[E0433]: cannot find `selection` in `crate`
error[E0599]: no variant, associated function, or constant named `SelectCells` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `VisualRows` found for enum `keymap::When` in the current scope
error[E0599]: no variant, associated function, or constant named `CopySelection` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `SelectBlock` found for enum `keymap::Command` in the current scope
...
```

- [ ] **Step 3: Write the code**

`src/selection.rs`:

```diff
diff --git a/src/selection.rs b/src/selection.rs
new file mode 100644
index 00000000..dd3b5834
--- /dev/null
+++ b/src/selection.rs
@@ -0,0 +1,732 @@
+//! What a grid has selected besides its cursor's cell: a rectangle of
+//! cells, or whole rows. A table's grid and a SQL result's share it.
+//!
+//! Rows are named as [`CellPos::row`] names them: a page's row, or in a
+//! table's grid a new row by its own (`edit::new_row`). Places are the
+//! grid's order, which an [`Order`] gives: a table's has its new rows
+//! among the page's, a result's is its rows'. What the selection covers is
+//! read in places, so a range drawn over a new row covers it where it
+//! stands.
+
+use std::collections::BTreeSet;
+
+use crate::edit::Order;
+use crate::model::CellPos;
+
+/// Whether moves keep a rectangle: the terminal look's visual selections.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Held {
+    /// Moves end it: Shift with a click or an arrow, a drag.
+    No,
+    /// `v`.
+    Visual,
+    /// `ctrl+v`.
+    Block,
+}
+
+/// What is selected with the cursor's cell.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum Range {
+    /// The rectangle of cells from `anchor` to the cursor.
+    Cells { anchor: CellPos, held: Held },
+    /// Whole rows: those in `picked`, and with an anchor every row from it
+    /// to the cursor's. `held` is the terminal look's `V`.
+    Rows {
+        anchor: Option<usize>,
+        picked: BTreeSet<usize>,
+        held: bool,
+    },
+}
+
+/// The terminal look's visual modes, as its status line names them.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Visual {
+    /// `v`: cells.
+    Cells,
+    /// `ctrl+v`: a block of cells.
+    Block,
+    /// `V`: whole rows.
+    Rows,
+}
+
+impl Visual {
+    /// The mode as the status line writes it.
+    pub fn mode(self) -> &'static str {
+        match self {
+            Self::Cells => "-- VISUAL --",
+            Self::Block => "-- VISUAL BLOCK --",
+            Self::Rows => "-- VISUAL LINE --",
+        }
+    }
+}
+
+/// What becomes of the range when the cursor is set ([`moved`]).
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Reach {
+    /// Shift with a click or an arrow, or a drag: a rectangle from its
+    /// anchor, or from the cell the cursor was on, to the new cell. Rows
+    /// stay rows, to the new cell's row.
+    Extend,
+    /// A move by key: a held range follows, another ends.
+    Move,
+    /// Anything else ends it: a click, an editor that opens, a new row, a
+    /// page that goes.
+    End,
+    /// The rows under the cursor changed, not what is selected: a new row
+    /// was dropped.
+    Keep,
+}
+
+/// How a row number was clicked.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum RowClick {
+    /// That row alone.
+    Only,
+    /// With Shift: the rows from the anchor to it.
+    Extend,
+    /// With Command: that row in or out, the others kept.
+    Toggle,
+}
+
+impl Range {
+    /// The terminal look's mode this range is, where it is held.
+    pub fn visual(&self) -> Option<Visual> {
+        match self {
+            Self::Cells {
+                held: Held::Visual, ..
+            } => Some(Visual::Cells),
+            Self::Cells {
+                held: Held::Block, ..
+            } => Some(Visual::Block),
+            Self::Cells { held: Held::No, .. } => None,
+            Self::Rows { held, .. } => held.then_some(Visual::Rows),
+        }
+    }
+
+    /// The row the range runs from, where it has one.
+    pub fn anchor_row(&self) -> Option<usize> {
+        match self {
+            Self::Cells { anchor, .. } => Some(anchor.row),
+            Self::Rows { anchor, .. } => *anchor,
+        }
+    }
+
+    /// Whether the range is of whole rows.
+    pub fn rows(&self) -> bool {
+        matches!(self, Self::Rows { .. })
+    }
+
+    /// Takes the rows `gone` names out of the range: they were dropped. An
+    /// anchor among them moves to `instead`'s answer for it, or goes where
+    /// there is none: a rectangle then ends, and rows keep what is picked.
+    /// Rows left with nothing selected end too.
+    pub fn without(
+        self,
+        gone: impl Fn(usize) -> bool,
+        instead: impl Fn(usize) -> Option<usize>,
+    ) -> Option<Self> {
+        let moved = |row: usize| if gone(row) { instead(row) } else { Some(row) };
+        match self {
+            Self::Cells { anchor, held } => {
+                let row = moved(anchor.row)?;
+                let anchor = CellPos { row, ..anchor };
+                Some(Self::Cells { anchor, held })
+            }
+            Self::Rows {
+                anchor,
+                picked,
+                held,
+            } => {
+                let picked: BTreeSet<usize> =
+                    picked.into_iter().filter(|row| !gone(*row)).collect();
+                let anchor = anchor.and_then(moved);
+                (anchor.is_some() || !picked.is_empty()).then_some(Self::Rows {
+                    anchor,
+                    picked,
+                    held,
+                })
+            }
+        }
+    }
+}
+
+/// The range after the cursor went from `was` to `cell`.
+pub fn moved(
+    range: Option<Range>,
+    was: Option<CellPos>,
+    cell: CellPos,
+    reach: Reach,
+) -> Option<Range> {
+    let range = match reach {
+        Reach::Extend => match range {
+            Some(Range::Rows {
+                anchor,
+                picked,
+                held,
+            }) => Some(Range::Rows {
+                anchor: anchor.or(was.map(|was| was.row)),
+                picked,
+                held,
+            }),
+            Some(cells @ Range::Cells { .. }) => Some(cells),
+            None => was.map(|anchor| Range::Cells {
+                anchor,
+                held: Held::No,
+            }),
+        },
+        Reach::Move => range.filter(|range| range.visual().is_some()),
+        Reach::End => None,
+        Reach::Keep => range,
+    };
+    // One cell that is not held is no range.
+    range.filter(
+        |range| !matches!(range, Range::Cells { anchor, held: Held::No } if *anchor == cell),
+    )
+}
+
+/// The range after the terminal look's `v`, `ctrl+v` or `V` on the cursor
+/// `cell`: the mode it is in is left, another becomes it, from the anchor
+/// that stands.
+pub fn toggled(range: Option<Range>, cell: CellPos, mode: Visual) -> Option<Range> {
+    if range.as_ref().and_then(Range::visual) == Some(mode) {
+        return None;
+    }
+    let anchor = range.as_ref().and_then(Range::anchor_row);
+    Some(match mode {
+        Visual::Cells | Visual::Block => {
+            let anchor = match range {
+                Some(Range::Cells { anchor, .. }) => anchor,
+                _ => CellPos {
+                    row: anchor.unwrap_or(cell.row),
+                    col: cell.col,
+                },
+            };
+            let held = if mode == Visual::Block {
+                Held::Block
+            } else {
+                Held::Visual
+            };
+            Range::Cells { anchor, held }
+        }
+        Visual::Rows => as_rows(range, cell, true),
+    })
+}
+
+/// The range made of the rows the selection covers: a rectangle's rows, or
+/// the cursor's row. `⇧Space`, and `V` with `held`.
+pub fn as_rows(range: Option<Range>, cell: CellPos, held: bool) -> Range {
+    match range {
+        Some(Range::Rows { anchor, picked, .. }) => Range::Rows {
+            anchor,
+            picked,
+            held,
+        },
+        Some(Range::Cells { anchor, .. }) => Range::Rows {
+            anchor: Some(anchor.row),
+            picked: BTreeSet::new(),
+            held,
+        },
+        None => Range::Rows {
+            anchor: Some(cell.row),
+            picked: BTreeSet::new(),
+            held,
+        },
+    }
+}
+
+/// The cursor and the range after a click on the number of `row`. The
+/// cursor goes to the row, in the column it was in, unless a Command-click
+/// took that row out: then it goes to the nearest row left, so the
+/// cursor's row is always a selected one.
+pub fn row_clicked(
+    range: Option<Range>,
+    cursor: Option<CellPos>,
+    row: usize,
+    click: RowClick,
+    order: Order<'_>,
+) -> (CellPos, Option<Range>) {
+    let col = cursor.map_or(0, |cell| cell.col);
+    let mut cell = CellPos { row, col };
+    let range = match click {
+        RowClick::Only => Some(Range::Rows {
+            anchor: Some(row),
+            picked: BTreeSet::new(),
+            held: false,
+        }),
+        RowClick::Extend => {
+            let (anchor, picked) = match range {
+                Some(Range::Rows { anchor, picked, .. }) => (anchor, picked),
+                Some(Range::Cells { anchor, .. }) => (Some(anchor.row), BTreeSet::new()),
+                None => (None, BTreeSet::new()),
+            };
+            Some(Range::Rows {
+                anchor: anchor.or(cursor.map(|cell| cell.row)).or(Some(row)),
+                picked,
+                held: false,
+            })
+        }
+        RowClick::Toggle => {
+            let mut picked: BTreeSet<usize> = match cursor {
+                Some(cursor) => Selection {
+                    cursor,
+                    range: range.as_ref(),
+                    order,
+                }
+                .rows()
+                .into_iter()
+                .collect(),
+                None => BTreeSet::new(),
+            };
+            if !picked.remove(&row) {
+                picked.insert(row);
+            } else if let Some(at) = order.place(row) {
+                let near = picked
+                    .iter()
+                    .filter_map(|picked| Some((order.place(*picked)?.abs_diff(at), *picked)))
+                    .min();
+                if let Some((_, nearest)) = near {
+                    cell.row = nearest;
+                }
+            }
+            // None left: the cursor's cell alone, as with no range.
+            (!picked.is_empty()).then_some(Range::Rows {
+                anchor: None,
+                picked,
+                held: false,
+            })
+        }
+    };
+    (cell, range)
+}
+
+/// Every row the grid shows, for `⌘A`.
+pub fn all(order: Order<'_>) -> Option<Range> {
+    let picked: BTreeSet<usize> = (0..order.len())
+        .filter_map(|place| order.row(place))
+        .collect();
+    (!picked.is_empty()).then_some(Range::Rows {
+        anchor: None,
+        picked,
+        held: false,
+    })
+}
+
+/// A rectangle of cells as places of the grid: the first and the last row
+/// and column, and where its anchor stands.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct Block {
+    pub rows: (usize, usize),
+    pub cols: (usize, usize),
+    pub anchor: CellPos,
+}
+
+impl Block {
+    pub fn covers(&self, place: usize, col: usize) -> bool {
+        (self.rows.0..=self.rows.1).contains(&place) && (self.cols.0..=self.cols.1).contains(&col)
+    }
+}
+
+/// A grid's selection, read: its cursor, its range and its order.
+#[derive(Clone, Copy)]
+pub struct Selection<'a> {
+    pub cursor: CellPos,
+    pub range: Option<&'a Range>,
+    pub order: Order<'a>,
+}
+
+impl Selection<'_> {
+    /// The places the selection's rows stand at, first to last.
+    pub fn places(&self) -> Vec<usize> {
+        let place = |row: usize| self.order.place(row);
+        let Some(range) = self.range else {
+            return place(self.cursor.row).into_iter().collect();
+        };
+        let span = |from: usize| -> Vec<usize> {
+            match (place(from), place(self.cursor.row)) {
+                (Some(a), Some(b)) => (a.min(b)..=a.max(b)).collect(),
+                _ => Vec::new(),
+            }
+        };
+        match range {
+            Range::Cells { anchor, .. } => span(anchor.row),
+            Range::Rows { anchor, picked, .. } => {
+                let mut places: BTreeSet<usize> =
+                    picked.iter().filter_map(|row| place(*row)).collect();
+                places.extend(anchor.map(span).unwrap_or_default());
+                places.into_iter().collect()
+            }
+        }
+    }
+
+    /// The rows the selection covers, in the grid's order. With no range,
+    /// the cursor's.
+    pub fn rows(&self) -> Vec<usize> {
+        let places = self.places();
+        places
+            .into_iter()
+            .filter_map(|place| self.order.row(place))
+            .collect()
+    }
+
+    /// The first and the last column the selection covers, of a grid
+    /// `width` wide: a rectangle's, every one for rows, the cursor's alone
+    /// with no range. `None` for a grid with no columns.
+    pub fn cols(&self, width: usize) -> Option<(usize, usize)> {
+        let last = width.checked_sub(1)?;
+        let col = self.cursor.col.min(last);
+        Some(match self.range {
+            None => (col, col),
+            Some(Range::Cells { anchor, .. }) => {
+                let from = anchor.col.min(last);
+                (from.min(col), from.max(col))
+            }
+            Some(Range::Rows { .. }) => (0, last),
+        })
+    }
+
+    /// How many rows and columns the range covers, where one stands.
+    pub fn size(&self, width: usize) -> Option<(usize, usize)> {
+        self.range?;
+        let (first, last) = self.cols(width)?;
+        Some((self.places().len(), last - first + 1))
+    }
+
+    /// The rectangle a range of cells covers, as places.
+    pub fn block(&self, width: usize) -> Option<Block> {
+        let Some(Range::Cells { anchor, .. }) = self.range else {
+            return None;
+        };
+        let places = self.places();
+        let rows = (*places.first()?, *places.last()?);
+        let cols = self.cols(width)?;
+        let row = self.order.place(anchor.row)?;
+        Some(Block {
+            rows,
+            cols,
+            anchor: CellPos {
+                row,
+                col: anchor.col,
+            },
+        })
+    }
+
+    /// The places of whole rows a range of rows holds.
+    pub fn ranged(&self) -> BTreeSet<usize> {
+        match self.range {
+            Some(Range::Rows { .. }) => self.places().into_iter().collect(),
+            _ => BTreeSet::new(),
+        }
+    }
+}
+
+/// What the selection's size is said as: "4 rows × 2 columns", or "3
+/// rows" for whole rows. In English, the words being the app's own.
+pub fn size_words(
+    locale: crate::i18n::Locale,
+    (rows, cols): (usize, usize),
+    whole_rows: bool,
+) -> String {
+    let rows = crate::ui::pending_bar::counted(locale, rows, "row", "rows");
+    if whole_rows {
+        return rows;
+    }
+    let cols = crate::ui::pending_bar::counted(locale, cols, "column", "columns");
+    format!("{rows} × {cols}")
+}
+
+/// What a screen reader is told of a range: "4 rows by 2 columns
+/// selected", or "3 rows selected" for whole rows.
+pub fn said(locale: crate::i18n::Locale, (rows, cols): (usize, usize), whole_rows: bool) -> String {
+    let rows = crate::ui::pending_bar::counted(locale, rows, "row", "rows");
+    let selected = crate::i18n::gettext(locale, "selected");
+    if whole_rows {
+        return format!("{rows} {selected}");
+    }
+    let cols = crate::ui::pending_bar::counted(locale, cols, "column", "columns");
+    let by = crate::i18n::gettext(locale, "by");
+    format!("{rows} {by} {cols} {selected}")
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::edit::{Edits, Place, new_row};
+
+    fn at(row: usize, col: usize) -> CellPos {
+        CellPos { row, col }
+    }
+
+    /// Three rows of a page, and a new one below the first: the new row's
+    /// own, and the edits that hold it.
+    fn with_new_row() -> (Edits, usize) {
+        let mut edits = Edits::default();
+        let id = edits.add_row(Place::Below(0), 3);
+        (edits, new_row(id))
+    }
+
+    fn read<'a>(cursor: CellPos, range: Option<&'a Range>, order: Order<'a>) -> Selection<'a> {
+        Selection {
+            cursor,
+            range,
+            order,
+        }
+    }
+
+    #[test]
+    fn shift_makes_a_rectangle_from_where_the_cursor_was() {
+        let order = Order::of(&[], 6);
+        let range = moved(None, Some(at(1, 1)), at(3, 2), Reach::Extend);
+        assert_eq!(
+            range,
+            Some(Range::Cells {
+                anchor: at(1, 1),
+                held: Held::No
+            })
+        );
+        let read = read(at(3, 2), range.as_ref(), order);
+        assert_eq!(read.rows(), [1, 2, 3]);
+        assert_eq!(read.cols(5), Some((1, 2)));
+        assert_eq!(read.size(5), Some((3, 2)));
+        assert_eq!(
+            read.block(5),
+            Some(Block {
+                rows: (1, 3),
+                cols: (1, 2),
+                anchor: at(1, 1)
+            })
+        );
+        // The anchor stays where the range began.
+        let range = moved(range, Some(at(3, 2)), at(0, 0), Reach::Extend);
+        let read = self::read(at(0, 0), range.as_ref(), order);
+        assert_eq!((read.rows(), read.cols(5)), (vec![0, 1], Some((0, 1))));
+        // Back on its anchor it is one cell, and no range.
+        assert_eq!(moved(range, Some(at(0, 0)), at(1, 1), Reach::Extend), None);
+    }
+
+    #[test]
+    fn with_no_cursor_shift_selects_one_cell() {
+        assert_eq!(moved(None, None, at(2, 0), Reach::Extend), None);
+        let read = read(at(2, 0), None, Order::of(&[], 4));
+        assert_eq!(
+            (read.rows(), read.cols(3), read.size(3)),
+            (vec![2], Some((0, 0)), None)
+        );
+    }
+
+    #[test]
+    fn a_move_ends_a_range_unless_it_is_held() {
+        let cells = moved(None, Some(at(1, 1)), at(2, 1), Reach::Extend);
+        assert_eq!(moved(cells, Some(at(2, 1)), at(3, 1), Reach::Move), None);
+        for held in [Held::Visual, Held::Block] {
+            let range = Some(Range::Cells {
+                anchor: at(1, 1),
+                held,
+            });
+            // On its anchor it is still a range: it is held.
+            let kept = moved(range.clone(), Some(at(2, 1)), at(1, 1), Reach::Move);
+            assert_eq!(kept, range);
+            assert_eq!(moved(range, Some(at(2, 1)), at(1, 2), Reach::End), None);
+        }
+        let rows = Some(as_rows(None, at(1, 0), true));
+        assert_eq!(
+            moved(rows.clone(), Some(at(1, 0)), at(4, 0), Reach::Move),
+            rows
+        );
+        let loose = Some(as_rows(None, at(1, 0), false));
+        assert_eq!(moved(loose, Some(at(1, 0)), at(4, 0), Reach::Move), None);
+        // Keep leaves it as it is.
+        let kept = moved(rows.clone(), Some(at(1, 0)), at(2, 0), Reach::Keep);
+        assert_eq!(kept, rows);
+    }
+
+    #[test]
+    fn shift_extends_rows_as_rows() {
+        let order = Order::of(&[], 6);
+        let (cell, range) = row_clicked(None, Some(at(4, 2)), 1, RowClick::Only, order);
+        assert_eq!(cell, at(1, 2));
+        let range = moved(range, Some(cell), at(3, 2), Reach::Extend);
+        let read = read(at(3, 2), range.as_ref(), order);
+        assert_eq!((read.rows(), read.cols(4)), (vec![1, 2, 3], Some((0, 3))));
+        assert_eq!(read.size(4), Some((3, 4)));
+        assert_eq!(read.block(4), None);
+        assert_eq!(read.ranged(), BTreeSet::from([1, 2, 3]));
+    }
+
+    #[test]
+    fn a_row_number_selects_extends_and_toggles_rows() {
+        let order = Order::of(&[], 8);
+        let (cell, range) = row_clicked(None, None, 2, RowClick::Only, order);
+        assert_eq!(cell, at(2, 0));
+        assert_eq!(read(cell, range.as_ref(), order).rows(), [2]);
+        // One row is a range of rows: it covers every column.
+        assert_eq!(read(cell, range.as_ref(), order).size(3), Some((1, 3)));
+        let (cell, range) = row_clicked(range, Some(cell), 4, RowClick::Extend, order);
+        assert_eq!(read(cell, range.as_ref(), order).rows(), [2, 3, 4]);
+        // Command takes one out, and puts one in apart from the others.
+        let (cell, range) = row_clicked(range, Some(cell), 3, RowClick::Toggle, order);
+        assert_eq!(read(cell, range.as_ref(), order).rows(), [2, 4]);
+        // The cursor goes to the row clicked, and with that row taken out to
+        // the nearest left.
+        assert_eq!(cell.row, 2);
+        let (cell, range) = row_clicked(range, Some(cell), 7, RowClick::Toggle, order);
+        assert_eq!(read(cell, range.as_ref(), order).rows(), [2, 4, 7]);
+        // Shift then runs from the cursor's row, the picked rows kept.
+        let (cell, range) = row_clicked(range, Some(cell), 5, RowClick::Extend, order);
+        assert_eq!(read(cell, range.as_ref(), order).rows(), [2, 4, 5, 6, 7]);
+        // Taking out the cursor's row between two others: the nearer one.
+        let (cell, range) = row_clicked(range, Some(cell), 5, RowClick::Toggle, order);
+        assert_eq!(
+            (cell.row, read(cell, range.as_ref(), order).rows()),
+            (4, vec![2, 4, 6, 7])
+        );
+        // The last one out leaves no range.
+        let (cell, range) = row_clicked(None, Some(at(1, 0)), 1, RowClick::Toggle, order);
+        assert_eq!((cell, range), (at(1, 0), None));
+    }
+
+    #[test]
+    fn command_on_a_row_with_no_range_adds_it_to_the_cursors() {
+        let order = Order::of(&[], 8);
+        let (cell, range) = row_clicked(None, Some(at(1, 3)), 5, RowClick::Toggle, order);
+        assert_eq!(cell, at(5, 3));
+        assert_eq!(read(cell, range.as_ref(), order).rows(), [1, 5]);
+        // A rectangle's rows count as selected rows.
+        let cells = moved(None, Some(at(0, 0)), at(1, 1), Reach::Extend);
+        let (cell, range) = row_clicked(cells, Some(at(1, 1)), 3, RowClick::Toggle, order);
+        assert_eq!(read(cell, range.as_ref(), order).rows(), [0, 1, 3]);
+    }
+
+    #[test]
+    fn select_all_picks_every_row_with_new_ones_among_them() {
+        let (edits, new) = with_new_row();
+        let order = Order::of(&edits.added, 3);
+        let range = all(order);
+        let read = read(at(0, 0), range.as_ref(), order);
+        assert_eq!(read.rows(), [0, new, 1, 2]);
+        assert_eq!(all(Order::of(&[], 0)), None);
+    }
+
+    #[test]
+    fn a_rectangle_covers_a_new_row_where_it_stands() {
+        let (edits, new) = with_new_row();
+        let order = Order::of(&edits.added, 3);
+        let range = moved(None, Some(at(0, 0)), at(1, 1), Reach::Extend);
+        let read = read(at(1, 1), range.as_ref(), order);
+        assert_eq!(read.rows(), [0, new, 1]);
+        assert_eq!(read.block(2).map(|block| block.rows), Some((0, 2)));
+        assert!(read.block(2).is_some_and(|block| block.covers(1, 1)));
+        assert!(!read.block(2).is_some_and(|block| block.covers(3, 1)));
+    }
+
+    #[test]
+    fn the_visual_modes_start_switch_and_leave() {
+        let cell = at(2, 1);
+        let cells = toggled(None, cell, Visual::Cells);
+        assert_eq!(cells.as_ref().and_then(Range::visual), Some(Visual::Cells));
+        // One cell held is a range: the status line says its size.
+        assert_eq!(
+            read(cell, cells.as_ref(), Order::of(&[], 4)).size(3),
+            Some((1, 1))
+        );
+        // Another mode keeps the anchor.
+        let block = toggled(cells.clone(), at(3, 2), Visual::Block);
+        assert_eq!(
+            block,
+            Some(Range::Cells {
+                anchor: cell,
+                held: Held::Block
+            })
+        );
+        let rows = toggled(block, at(3, 2), Visual::Rows);
+        assert_eq!(rows.as_ref().and_then(Range::anchor_row), Some(2));
+        assert_eq!(rows.as_ref().and_then(Range::visual), Some(Visual::Rows));
+        let back = toggled(rows.clone(), at(3, 2), Visual::Cells);
+        assert_eq!(
+            back,
+            Some(Range::Cells {
+                anchor: at(2, 2),
+                held: Held::Visual
+            })
+        );
+        // The same key again leaves it.
+        assert_eq!(toggled(rows, at(3, 2), Visual::Rows), None);
+        assert_eq!(toggled(cells, cell, Visual::Cells), None);
+        // A rectangle made with Shift becomes the visual one, from its
+        // anchor.
+        let shifted = moved(None, Some(at(0, 0)), at(1, 1), Reach::Extend);
+        assert_eq!(
+            toggled(shifted, at(1, 1), Visual::Block),
+            Some(Range::Cells {
+                anchor: at(0, 0),
+                held: Held::Block
+            })
+        );
+    }
+
+    #[test]
+    fn shift_space_makes_rows_of_what_is_selected() {
+        let order = Order::of(&[], 6);
+        let cells = moved(None, Some(at(1, 1)), at(3, 2), Reach::Extend);
+        let rows = as_rows(cells, at(3, 2), false);
+        assert_eq!(read(at(3, 2), Some(&rows), order).rows(), [1, 2, 3]);
+        assert_eq!(rows.visual(), None);
+        let one = as_rows(None, at(4, 0), false);
+        assert_eq!(read(at(4, 0), Some(&one), order).rows(), [4]);
+    }
+
+    #[test]
+    fn rows_that_go_leave_the_range() {
+        let new = with_new_row().1;
+        let rows = Range::Rows {
+            anchor: Some(new),
+            picked: BTreeSet::from([0, new]),
+            held: false,
+        };
+        let gone = |row: usize| row == new;
+        assert_eq!(
+            rows.clone().without(gone, |_| Some(1)),
+            Some(Range::Rows {
+                anchor: Some(1),
+                picked: BTreeSet::from([0]),
+                held: false
+            })
+        );
+        // Nothing to take the anchor's place: the picked rows stay.
+        assert_eq!(
+            rows.without(gone, |_| None),
+            Some(Range::Rows {
+                anchor: None,
+                picked: BTreeSet::from([0]),
+                held: false
+            })
+        );
+        let only = Range::Rows {
+            anchor: None,
+            picked: BTreeSet::from([new]),
+            held: false,
+        };
+        assert_eq!(only.without(gone, |_| Some(1)), None);
+        let cells = Range::Cells {
+            anchor: at(new, 2),
+            held: Held::Visual,
+        };
+        assert_eq!(cells.clone().without(gone, |_| None), None);
+        assert_eq!(
+            cells.without(gone, |_| Some(3)),
+            Some(Range::Cells {
+                anchor: at(3, 2),
+                held: Held::Visual
+            })
+        );
+    }
+
+    #[test]
+    fn the_size_is_said_in_rows_and_columns() {
+        let locale = crate::i18n::Locale::default();
+        assert_eq!(size_words(locale, (4, 2), false), "4 rows × 2 columns");
+        assert_eq!(size_words(locale, (1, 1), false), "1 row × 1 column");
+        assert_eq!(size_words(locale, (3, 7), true), "3 rows");
+        assert_eq!(said(locale, (4, 2), false), "4 rows by 2 columns selected");
+        assert_eq!(said(locale, (1, 5), true), "1 row selected");
+    }
+}
```

`src/lib.rs`:

```diff
diff --git a/src/lib.rs b/src/lib.rs
index ec7ed9c2..af8774e3 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -31,6 +31,7 @@ pub mod relay;
 pub mod review;
 pub mod search;
 pub mod secrets;
+pub mod selection;
 pub mod session;
 pub mod settings;
 #[cfg(all(test, feature = "shots"))]
```

`src/model.rs`:

```diff
diff --git a/src/model.rs b/src/model.rs
index eaa38738..f55f3fc9 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -302,27 +302,49 @@ pub enum Action {
         id: TabId,
         cell: CellPos,
     },
-    /// Select the rows from the cursor's row, or from the range's anchor,
-    /// to `cell`'s row, and put the cursor on `cell`: Shift with a click.
-    /// With no cursor it selects the cell.
+    /// Select the cells from the cursor's, or from the range's anchor, to
+    /// `cell`, and put the cursor on `cell`: Shift with a click, a drag.
+    /// A range of rows runs to `cell`'s row. With no cursor it selects the
+    /// cell.
     ExtendTo {
         tab: ConnTabId,
         id: TabId,
         cell: CellPos,
     },
-    /// The same by rows from the cursor: Shift with an arrow.
+    /// The same by rows and columns from the cursor: Shift with an arrow.
     ExtendBy {
         tab: ConnTabId,
         id: TabId,
         rows: isize,
+        cols: isize,
     },
-    /// Start a visual selection of rows on the cursor's row, or leave the
-    /// one that is on: the terminal look's `V`.
+    /// Start a visual selection on the cursor, or leave the one that is
+    /// on: the terminal look's `v` (cells), `ctrl+v` (a block) and `V`
+    /// (rows).
     SelectRows {
         tab: ConnTabId,
         id: TabId,
+        mode: Visual,
     },
-    /// Leave a visual selection of rows. The cursor stays.
+    /// Select the rows the selection covers, whole: `⇧Space`.
+    SelectRowsOfCursor {
+        tab: ConnTabId,
+        id: TabId,
+    },
+    /// Select every row the grid shows: `⌘A`.
+    SelectAllRows {
+        tab: ConnTabId,
+        id: TabId,
+    },
+    /// A click on a row's number: that row, with Shift the rows to it, with
+    /// Command that row in or out.
+    RowNumberClicked {
+        tab: ConnTabId,
+        id: TabId,
+        row: usize,
+        click: RowClick,
+    },
+    /// Leave a visual selection. The cursor stays.
     LeaveVisual {
         tab: ConnTabId,
         id: TabId,
@@ -3781,33 +3803,7 @@ pub enum SaveBlock {
     Unsendable,
 }
 
-/// The rows selected with the cursor's: from `anchor` to the cursor's row,
-/// in the order the grid shows them.
-#[derive(Debug, Clone, Copy, PartialEq, Eq)]
-pub struct RowRange {
-    /// A row as [`CellPos::row`] names it: of the page, or a new row.
-    pub anchor: usize,
-    /// Moves keep the range until it is left: the terminal look's visual
-    /// selection.
-    pub held: bool,
-}
-
-/// What becomes of the range when the cursor is set
-/// ([`ObjectTab::set_cursor`]).
-#[derive(Debug, Clone, Copy, PartialEq, Eq)]
-pub enum Reach {
-    /// Shift with a click or an arrow: the range runs from its anchor, or
-    /// from the row the cursor was on, to the new row.
-    Extend,
-    /// A move by key: a held range follows, another ends.
-    Move,
-    /// Anything else ends it: a click, an editor that opens, a new row, a
-    /// page that goes.
-    End,
-    /// The rows under the cursor changed, not what is selected: a new row
-    /// was dropped.
-    Keep,
-}
+pub use crate::selection::{Range, Reach, RowClick, Visual};
 
 /// One open table or view.
 #[derive(Debug)]
@@ -3839,9 +3835,9 @@ pub struct ObjectTab {
     /// it is on: it runs from there to the cursor's line.
     pub ddl_visual: Option<usize>,
     pub selection: Option<CellPos>,
-    /// The rows selected with the cursor's, where there are several.
-    /// Written with the cursor, by [`ObjectTab::set_cursor`].
-    pub range: Option<RowRange>,
+    /// What is selected with the cursor's cell: a rectangle of cells, or
+    /// whole rows. Written with the cursor, by [`ObjectTab::set_cursor`].
+    pub range: Option<Range>,
     pub estimated_rows: Option<u64>,
     /// The exact row count for the current filters, when asked for.
     pub count: Fetch<u64>,
@@ -3988,70 +3984,99 @@ impl ObjectTab {
     pub fn set_cursor(&mut self, cell: Option<CellPos>, reach: Reach) {
         let was = self.selection;
         self.selection = cell;
-        let Some(cell) = cell else {
-            self.range = None;
-            return;
-        };
-        let range = match reach {
-            Reach::Extend => self.range.or_else(|| {
-                was.map(|was| RowRange {
-                    anchor: was.row,
-                    held: false,
-                })
-            }),
-            Reach::Move => self.range.filter(|range| range.held),
-            Reach::End => None,
-            Reach::Keep => self.range,
-        };
-        // One row that is not held is no range.
-        self.range = range.filter(|range| range.held || range.anchor != cell.row);
+        let range = self.range.take();
+        self.range = cell.and_then(|cell| crate::selection::moved(range, was, cell, reach));
     }
 
-    /// The places of the grid the range covers, the first and the last,
-    /// where a range stands.
-    pub fn selected_span(&self) -> Option<(usize, usize)> {
-        let (cell, range) = (self.selection?, self.range?);
+    /// The grid's order of its rows: the page's, and the new ones among
+    /// them.
+    pub fn order(&self) -> crate::edit::Order<'_> {
         let height = self.page().map_or(0, |page| page.rows.len());
-        let order = crate::edit::Order::of(&self.edits.added, height);
-        let (from, to) = (order.place(range.anchor)?, order.place(cell.row)?);
-        Some((from.min(to), from.max(to)))
+        crate::edit::Order::of(&self.edits.added, height)
+    }
+
+    /// The selection, read, where there is a cursor.
+    pub fn selected(&self) -> Option<crate::selection::Selection<'_>> {
+        Some(crate::selection::Selection {
+            cursor: self.selection?,
+            range: self.range.as_ref(),
+            order: self.order(),
+        })
     }
 
     /// The selected rows, in the grid's order: the cursor's, and with a
-    /// range every row from its anchor to the cursor's.
+    /// range the rows it covers.
     pub fn selected_rows(&self) -> Vec<usize> {
-        let Some(cell) = self.selection else {
-            return Vec::new();
-        };
-        let Some((first, last)) = self.selected_span() else {
-            return vec![cell.row];
-        };
-        let height = self.page().map_or(0, |page| page.rows.len());
-        let order = crate::edit::Order::of(&self.edits.added, height);
-        (first..=last)
-            .filter_map(|place| order.row(place))
-            .collect()
+        self.selected()
+            .map_or_else(Vec::new, |selected| selected.rows())
     }
 
-    /// Whether a visual selection of rows is on: a range that moves keep.
+    /// The first and the last column the selection covers.
+    pub fn selected_cols(&self) -> Option<(usize, usize)> {
+        let width = self.page()?.columns.len();
+        self.selected()?.cols(width)
+    }
+
+    /// How many rows and columns a range covers, where one stands.
+    pub fn selection_size(&self) -> Option<(usize, usize)> {
+        let width = self.page()?.columns.len();
+        self.selected()?.size(width)
+    }
+
+    /// Whether a visual selection is on: a range that moves keep.
     pub fn visual(&self) -> bool {
-        self.range.is_some_and(|range| range.held)
+        self.visual_mode().is_some()
     }
 
-    /// Starts a visual selection of rows on the cursor's row, or leaves
-    /// the one that is on. A range that stands becomes it, from its
-    /// anchor. With no cursor there is none to start.
-    pub fn toggle_visual(&mut self) {
-        if self.visual() {
-            self.range = None;
-            return;
-        }
+    /// The terminal look's visual mode, while one is on.
+    pub fn visual_mode(&self) -> Option<Visual> {
+        self.range.as_ref().and_then(Range::visual)
+    }
+
+    /// Starts the visual selection `mode` on the cursor, or leaves the one
+    /// that is on: `v`, `ctrl+v`, `V`. A range that stands becomes it,
+    /// from its anchor. With no cursor there is none to start.
+    pub fn toggle_visual(&mut self, mode: Visual) {
         if let Some(cell) = self.selection {
-            let anchor = self.range.map_or(cell.row, |range| range.anchor);
-            self.range = Some(RowRange { anchor, held: true });
+            self.range = crate::selection::toggled(self.range.take(), cell, mode);
+        } else {
+            self.range = None;
         }
     }
 
+    /// Selects the rows the selection covers, whole: `⇧Space`.
+    pub fn select_rows_of_cursor(&mut self) {
+        if let Some(cell) = self.selection {
+            let range = crate::selection::as_rows(self.range.take(), cell, false);
+            self.range = Some(range);
+        }
+    }
+
+    /// Selects every row the grid shows: `⌘A`. The cursor goes to the
+    /// first where there is none.
+    pub fn select_all_rows(&mut self) {
+        let order = self.order();
+        let range = crate::selection::all(order);
+        let first = order.row(0);
+        if range.is_some() && self.selection.is_none() {
+            self.selection = first.map(|row| CellPos { row, col: 0 });
+        }
+        self.range = range.filter(|_| self.selection.is_some());
+    }
+
+    /// A click on the number of `row`.
+    pub fn row_number_clicked(&mut self, row: usize, click: RowClick) {
+        let (cell, range) = crate::selection::row_clicked(
+            self.range.take(),
+            self.selection,
+            row,
+            click,
+            self.order(),
+        );
+        self.selection = Some(cell);
+        self.range = range;
+    }
+
     /// The cursor's row and the rows after it in the grid's order, `count`
     /// in all, as far as the grid goes: what `{count}dd` names.
     pub fn rows_from_cursor(&self, count: u32) -> Vec<usize> {
@@ -4174,14 +4199,12 @@ impl ObjectTab {
         {
             self.set_cursor(None, Reach::End);
         }
-        // A range that began on a new row, the cursor on a row of the
-        // page: the set that held its anchor was dropped.
-        if self
+        // The new rows of a range: the set that held them was dropped.
+        let new = |row: usize| crate::edit::new_id(row).is_some();
+        self.range = self
             .range
-            .is_some_and(|range| crate::edit::new_id(range.anchor).is_some())
-        {
-            self.range = None;
-        }
+            .take()
+            .and_then(|range| range.without(new, |_| None));
     }
 
     /// The row panel's text for the selected row, if it is up to date.
@@ -5120,6 +5143,8 @@ pub struct SqlTab {
     /// to 1. Change it through `set_split`.
     pub split: f32,
     pub selection: Option<CellPos>,
+    /// What is selected with the cursor's cell, as in a table's grid.
+    pub range: Option<Range>,
     /// The row panel's text for the selected row (see `App::format_rows`).
     pub fields: Option<RowFields>,
     /// Focus the editor on the next frame.
@@ -5189,6 +5214,7 @@ impl SqlTab {
             pane: ResultPane::default(),
             split: Self::DEFAULT_SPLIT,
             selection: None,
+            range: None,
             fields: None,
             focus_editor: true,
             go_to: None,
@@ -5406,6 +5432,80 @@ impl SqlTab {
             .map_or((0, 0), |(columns, rows, _)| (rows.len(), columns.len()))
     }
 
+    /// Puts the cursor on `cell`, or takes it away, as a table's tab does
+    /// ([`ObjectTab::set_cursor`]).
+    pub fn set_cursor(&mut self, cell: Option<CellPos>, reach: Reach) {
+        let was = self.selection;
+        self.selection = cell;
+        let range = self.range.take();
+        self.range = cell.and_then(|cell| crate::selection::moved(range, was, cell, reach));
+    }
+
+    /// The result's order of its rows: as they came.
+    pub fn order(&self) -> crate::edit::Order<'static> {
+        crate::edit::Order::of(&[], self.dims().0)
+    }
+
+    /// The selection, read, where there is a cursor in the result's
+    /// rows.
+    pub fn selected(&self) -> Option<crate::selection::Selection<'_>> {
+        let cursor = self.selection.filter(|cell| cell.row < self.dims().0)?;
+        Some(crate::selection::Selection {
+            cursor,
+            range: self.range.as_ref(),
+            order: self.order(),
+        })
+    }
+
+    /// The terminal look's visual mode, while one is on.
+    pub fn visual_mode(&self) -> Option<Visual> {
+        self.range.as_ref().and_then(Range::visual)
+    }
+
+    /// How many rows and columns a range covers, where one stands.
+    pub fn selection_size(&self) -> Option<(usize, usize)> {
+        self.selected()?.size(self.dims().1)
+    }
+
+    /// As [`ObjectTab::toggle_visual`].
+    pub fn toggle_visual(&mut self, mode: Visual) {
+        if let Some(cell) = self.selection {
+            self.range = crate::selection::toggled(self.range.take(), cell, mode);
+        } else {
+            self.range = None;
+        }
+    }
+
+    /// As [`ObjectTab::select_rows_of_cursor`].
+    pub fn select_rows_of_cursor(&mut self) {
+        if let Some(cell) = self.selection {
+            let range = crate::selection::as_rows(self.range.take(), cell, false);
+            self.range = Some(range);
+        }
+    }
+
+    /// As [`ObjectTab::select_all_rows`].
+    pub fn select_all_rows(&mut self) {
+        let range = crate::selection::all(self.order());
+        if range.is_some() && self.selection.is_none() {
+            self.selection = Some(CellPos { row: 0, col: 0 });
+        }
+        self.range = range.filter(|_| self.selection.is_some());
+    }
+
+    /// As [`ObjectTab::row_number_clicked`].
+    pub fn row_number_clicked(&mut self, row: usize, click: RowClick) {
+        let (cell, range) = crate::selection::row_clicked(
+            self.range.take(),
+            self.selection,
+            row,
+            click,
+            self.order(),
+        );
+        self.selection = Some(cell);
+        self.range = range;
+    }
+
     /// The selected row of the result, while the Results pane shows it:
     /// the row the row panel is for. None under the Messages pane, which
     /// shows no rows.
```

`src/app.rs`:

```diff
diff --git a/src/app.rs b/src/app.rs
index f578b1a9..d8bc383f 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1732,7 +1732,7 @@ impl App {
                     // that drew the cell.
                     let (height, width) = sql.dims();
                     if cell.row < height && cell.col < width {
-                        sql.selection = Some(cell);
+                        sql.set_cursor(Some(cell), Reach::End);
                     }
                 }
                 if let Some(workspace) = self.workspace_mut(tab) {
@@ -2051,14 +2051,8 @@ impl App {
                     object.focus_field = None;
                 }
                 if let Some(sql) = self.sql_tab_mut(tab, id) {
-                    let (height, width) = sql.dims();
-                    sql.selection = (height > 0 && width > 0).then(|| match sql.selection {
-                        None => CellPos { row: 0, col: 0 },
-                        Some(cell) => CellPos {
-                            row: step(cell.row, rows, height),
-                            col: step(cell.col, cols, width),
-                        },
-                    });
+                    let moved = sql_moved(sql, rows, cols);
+                    sql.set_cursor(moved, Reach::Move);
                 } else if let Some(object) = self.object_tab_mut(tab, id) {
                     let moved = moved(object, rows, cols);
                     object.pinned |= moved.is_some();
@@ -2073,29 +2067,86 @@ impl App {
                     object.focus_field = None;
                     object.set_cursor(Some(cell), Reach::Extend);
                     object.pinned = true;
+                } else if let Some(sql) = self.sql_tab_mut(tab, id) {
+                    let (height, width) = sql.dims();
+                    if cell.row < height && cell.col < width {
+                        sql.set_cursor(Some(cell), Reach::Extend);
+                    }
                 }
                 if let Some(workspace) = self.workspace_mut(tab) {
                     workspace.pane = Pane::Grid;
                 }
             }
-            Action::ExtendBy { tab, id, rows } => {
+            Action::ExtendBy {
+                tab,
+                id,
+                rows,
+                cols,
+            } => {
                 self.close_editor(tab, id, true);
                 if let Some(object) = self.object_tab_mut(tab, id) {
                     object.edits.why = None;
                     object.focus_field = None;
-                    let moved = moved(object, rows, 0);
+                    let moved = moved(object, rows, cols);
                     object.pinned |= moved.is_some();
                     object.set_cursor(moved, Reach::Extend);
+                } else if let Some(sql) = self.sql_tab_mut(tab, id) {
+                    let moved = sql_moved(sql, rows, cols);
+                    sql.set_cursor(moved, Reach::Extend);
                 }
             }
-            Action::SelectRows { tab, id } => {
+            Action::SelectRows { tab, id, mode } => {
                 // As a move does: an editor's text is kept, and the keys
                 // are the rows'.
                 self.close_editor(tab, id, true);
                 if let Some(object) = self.object_tab_mut(tab, id) {
                     object.edits.why = None;
                     object.focus_field = None;
-                    object.toggle_visual();
+                    object.toggle_visual(mode);
+                } else if let Some(sql) = self.sql_tab_mut(tab, id) {
+                    sql.toggle_visual(mode);
+                }
+            }
+            Action::SelectRowsOfCursor { tab, id } => {
+                self.close_editor(tab, id, true);
+                if let Some(object) = self.object_tab_mut(tab, id) {
+                    object.select_rows_of_cursor();
+                } else if let Some(sql) = self.sql_tab_mut(tab, id) {
+                    sql.select_rows_of_cursor();
+                }
+            }
+            Action::SelectAllRows { tab, id } => {
+                self.close_editor(tab, id, true);
+                if let Some(object) = self.object_tab_mut(tab, id) {
+                    object.edits.why = None;
+                    object.focus_field = None;
+                    object.select_all_rows();
+                } else if let Some(sql) = self.sql_tab_mut(tab, id) {
+                    sql.select_all_rows();
+                }
+                if let Some(workspace) = self.workspace_mut(tab) {
+                    workspace.pane = Pane::Grid;
+                }
+            }
+            Action::RowNumberClicked {
+                tab,
+                id,
+                row,
+                click,
+            } => {
+                self.close_editor(tab, id, true);
+                if let Some(object) = self.object_tab_mut(tab, id) {
+                    object.edits.why = None;
+                    object.focus_field = None;
+                    object.row_number_clicked(row, click);
+                    object.pinned = true;
+                } else if let Some(sql) = self.sql_tab_mut(tab, id)
+                    && row < sql.dims().0
+                {
+                    sql.row_number_clicked(row, click);
+                }
+                if let Some(workspace) = self.workspace_mut(tab) {
+                    workspace.pane = Pane::Grid;
                 }
             }
             Action::LeaveVisual { tab, id } => {
@@ -2103,6 +2154,10 @@ impl App {
                     && object.visual()
                 {
                     object.range = None;
+                } else if let Some(sql) = self.sql_tab_mut(tab, id)
+                    && sql.visual_mode().is_some()
+                {
+                    sql.range = None;
                 }
             }
             Action::ToggleRowPanel(tab) => {
@@ -4825,7 +4880,7 @@ impl App {
                 });
                 let messages = opens_messages(&result, cancel);
                 if sql.finish_run(request, result, cancel) {
-                    sql.selection = None;
+                    sql.set_cursor(None, Reach::End);
                     sql.pane = if messages {
                         ResultPane::Messages
                     } else {
@@ -7716,29 +7771,34 @@ impl App {
         }
     }
 
-    /// Text for the clipboard: the selected cell, or its whole row as TSV.
-    /// What the grid shows: a pending cell gives its new value, as the row
-    /// panel's copy does.
+    /// What a copy takes of the active tab's grid, a table's or a SQL
+    /// result's: with a range, the cells it covers as TSV, a line for each
+    /// row; with none, or one cell held by `v`, the cursor's cell's text as
+    /// it is. `whole_row` takes every
+    /// column of the rows the selection covers. What is pending reads as
+    /// its new value, and a new row's cell nothing is set in as empty.
     pub fn copy_text(&self, whole_row: bool) -> Option<String> {
         use tabletist_db::Value;
-        let (tab, id) = self.active_object()?;
-        let object = self.workspace(tab)?.object_tab(id)?;
-        let cell = object.selection?;
-        let row = object.page()?.rows.get(cell.row)?;
-        let shown = |col: usize, loaded: &Value| match object.edits.cells.get(&(cell.row, col)) {
-            Some(pending) => crate::edit::shown_value(&pending.new),
-            None => loaded.clone(),
-        };
-        Some(if whole_row {
-            let row: Vec<Value> = row
-                .iter()
-                .enumerate()
-                .map(|(col, loaded)| shown(col, loaded))
-                .collect();
-            crate::ui::format::tsv_row(&row)
-        } else {
-            crate::ui::format::plain_text(&shown(cell.col, row.get(cell.col)?))
-        })
+        let workspace = self.workspace(self.active_tab_id())?;
+        if let Some(object) = workspace.active_object_tab() {
+            let selected = object.selected()?;
+            let page = object.page()?;
+            // The cursor's cell or row alone is copied from the page's
+            // rows: a new row is none of them.
+            if selected.range.is_none() && crate::edit::new_id(selected.cursor.row).is_some() {
+                return None;
+            }
+            let value = |row: usize, col: usize| match object.edits.cells.get(&(row, col)) {
+                Some(pending) => Some(crate::edit::shown_value(&pending.new)),
+                None => page.rows.get(row)?.get(col).cloned(),
+            };
+            return copied(&selected, page.columns.len(), whole_row, value);
+        }
+        let sql = workspace.active_sql_tab()?;
+        let selected = sql.selected()?;
+        let (columns, rows, _) = sql.shown_rows()?;
+        let value = |row: usize, col: usize| -> Option<Value> { rows.get(row)?.get(col).cloned() };
+        copied(&selected, columns.len(), whole_row, value)
     }
 
     /// The palette and the look, applied to egui's style, with the caret
@@ -8417,6 +8477,50 @@ fn step(index: usize, delta: isize, len: usize) -> usize {
     moved as usize
 }
 
+/// What [`App::copy_text`] copies of `selected`, in a grid `width` wide
+/// whose cells `value` reads.
+fn copied(
+    selected: &crate::selection::Selection<'_>,
+    width: usize,
+    whole_row: bool,
+    value: impl Fn(usize, usize) -> Option<tabletist_db::Value>,
+) -> Option<String> {
+    let cursor = selected.cursor;
+    // One cell, alone or held by `v`: its text as it is, tabs and lines in
+    // it kept.
+    if !whole_row && selected.size(width).is_none_or(|size| size == (1, 1)) {
+        let value = value(cursor.row, cursor.col)?;
+        return Some(crate::ui::format::plain_text(&value));
+    }
+    let (first, last) = if whole_row {
+        (0, width.checked_sub(1)?)
+    } else {
+        selected.cols(width)?
+    };
+    let lines: Vec<String> = selected
+        .rows()
+        .into_iter()
+        .map(|row| {
+            let cells = (first..=last).map(|col| value(row, col));
+            crate::ui::format::tsv_cells(cells)
+        })
+        .collect();
+    Some(lines.join("\n"))
+}
+
+/// Where a move by `rows` and `cols` puts the cursor of a SQL result's
+/// grid. With no cursor, the first cell. `None` where it has no cell.
+fn sql_moved(sql: &SqlTab, rows: isize, cols: isize) -> Option<CellPos> {
+    let (height, width) = sql.dims();
+    (height > 0 && width > 0).then(|| match sql.selection {
+        None => CellPos { row: 0, col: 0 },
+        Some(cell) => CellPos {
+            row: step(cell.row, rows, height),
+            col: step(cell.col, cols, width),
+        },
+    })
+}
+
 /// Where a move by `rows` and `cols` puts the cursor of `object`, through
 /// the rows as the grid shows them: a new row stands among the page's, and
 /// is no row of it. With no cursor, the first cell. `None` where the grid
```

`src/app/editing.rs`:

```diff
diff --git a/src/app/editing.rs b/src/app/editing.rs
index c6d17c2d..fd4a080b 100644
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -68,6 +68,9 @@ pub(super) fn dropped_under_a_prompt(action: &Action) -> bool {
             | Action::ExtendTo { .. }
             | Action::ExtendBy { .. }
             | Action::SelectRows { .. }
+            | Action::SelectRowsOfCursor { .. }
+            | Action::SelectAllRows { .. }
+            | Action::RowNumberClicked { .. }
             | Action::MoveSelection { .. }
             // Closing the row panel closes the editor it draws, and the
             // text of that editor joins the set.
@@ -111,14 +114,11 @@ fn drop_named(object: &mut ObjectTab, named: &[usize], new: &[usize]) -> usize {
         let near = left.iter().min_by_key(|(place, _)| place.abs_diff(at));
         near.map(|(_, row)| *row)
     };
-    if let Some(anchor) = object.range.map(|range| range.anchor)
-        && new.contains(&anchor)
-    {
-        match (nearest(anchor), object.range.as_mut()) {
-            (Some(row), Some(range)) => range.anchor = row,
-            _ => object.range = None,
-        }
-    }
+    let gone = |row: usize| new.contains(&row) && new_id(row).is_some();
+    object.range = object
+        .range
+        .take()
+        .and_then(|range| range.without(gone, nearest));
     if let Some(cell) = object.selection.filter(|cell| new.contains(&cell.row)) {
         let row = nearest(cell.row).or_else(|| {
             let order = Order::of(&object.edits.added, height);
@@ -1122,17 +1122,27 @@ impl App {
         self.settle_new_row(tab, id, cell, required.is_some());
     }
 
-    /// Adds a copy of the selected row as a new row at the top.
+    /// Adds a copy of each selected row as a new row at the top.
     pub(super) fn duplicate_row(&mut self, tab: ConnTabId, id: TabId) {
         // Before the row is read: an open editor's text is part of what
         // the row shows, and closed it is pending.
         self.close_editor(tab, id, true);
-        let copy = self.table(tab, id, |table, object| {
-            let yank = table.yank(object.selection?.row, &object.edits.cells)?;
-            Some(table.duplicate(&yank))
+        // Each selected row, in the grid's order: the copies stand at the
+        // top in that order, and the first is settled as one copy is.
+        let copies = self.table(tab, id, |table, object| {
+            let rows = object.selected_rows();
+            let yanks = rows
+                .iter()
+                .filter_map(|row| table.yank(*row, &object.edits.cells));
+            yanks.map(|yank| table.duplicate(&yank)).collect::<Vec<_>>()
         });
-        if let Some(Some(copy)) = copy {
-            self.put_copy(tab, id, copy, Place::Top);
+        let mut first = None;
+        for copy in copies.unwrap_or_default() {
+            let added = self.add_copy(tab, id, copy, Place::Top);
+            first = first.or(added);
+        }
+        if let Some((cell, open)) = first {
+            self.settle_new_row(tab, id, cell, open);
         }
     }
 
@@ -1172,22 +1182,28 @@ impl App {
             (Some(cell), true) => Place::Above(cell.row),
             (None, _) => Place::Top,
         };
-        self.put_copy(tab, id, copy, place);
+        if let Some((cell, open)) = self.add_copy(tab, id, copy, place) {
+            self.settle_new_row(tab, id, cell, open);
+        }
     }
 
     /// Adds `copy` as a new row at `place`, where the table takes one, and
-    /// settles it. The editor opens on the first cell a save needs a value
-    /// in and that takes one. With none the selection goes to the copy, in
-    /// the column the cursor was in. Both are the copy's own rules: an
-    /// added row opens on its first required cell whatever it is, and
+    /// says where to settle it: the cell the editor opens on, the first a
+    /// save needs a value in and that takes one, or with none the copy's
+    /// cell in the column the cursor was in. Both are the copy's own rules:
+    /// an added row opens on its first required cell whatever it is, and
     /// otherwise selects the first cell that takes a value.
-    fn put_copy(&mut self, tab: ConnTabId, id: TabId, copy: Duplicate, place: Place) {
+    fn add_copy(
+        &mut self,
+        tab: ConnTabId,
+        id: TabId,
+        copy: Duplicate,
+        place: Place,
+    ) -> Option<(CellPos, bool)> {
         if !self.takes_rows(tab, id) {
-            return;
+            return None;
         }
-        let Some(object) = self.object_tab_mut(tab, id) else {
-            return;
-        };
+        let object = self.object_tab_mut(tab, id)?;
         let stood = object.selection.map_or(0, |cell| cell.col);
         let height = object.page().map_or(0, |page| page.rows.len());
         let new = object.edits.add_copy(place, height, copy);
@@ -1200,7 +1216,7 @@ impl App {
             row,
             col: open.unwrap_or(stood),
         };
-        self.settle_new_row(tab, id, cell, open.is_some());
+        Some((cell, open.is_some()))
     }
 
     /// Drops the selected row where it is a new one. The selection stays
@@ -1232,9 +1248,11 @@ impl App {
             .map(|row| CellPos { row, col: cell.col });
         // The rows selected with it stay selected, unless they began on
         // the row that went.
-        if object.range.is_some_and(|range| range.anchor == cell.row) {
-            object.range = None;
-        }
+        let gone = |row: usize| row == cell.row;
+        object.range = object
+            .range
+            .take()
+            .and_then(|range| range.without(gone, |_| None));
         object.set_cursor(stays, Reach::Keep);
     }
 
```

`src/ui/format.rs`:

```diff
diff --git a/src/ui/format.rs b/src/ui/format.rs
index 7bbae9bf..fbd108d5 100644
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -971,8 +971,17 @@ pub fn refuses_writes(error: &tabletist_db::Error, driver: tabletist_db::Driver)
 
 /// A row as tab-separated values on one line.
 pub fn tsv_row(row: &[Value]) -> String {
-    row.iter()
-        .map(|value| plain_text(value).replace(['\t', '\n', '\r'], " "))
+    tsv_cells(row.iter().cloned().map(Some))
+}
+
+/// Cells as one line of TSV, as [`tsv_row`] writes them: a cell with no
+/// value (a new row's that nothing is set in) is empty.
+pub fn tsv_cells(cells: impl Iterator<Item = Option<Value>>) -> String {
+    cells
+        .map(|value| {
+            let text = value.map_or_else(String::new, |value| plain_text(&value));
+            text.replace(['\t', '\n', '\r'], " ")
+        })
         .collect::<Vec<_>>()
         .join("\t")
 }
```

`src/keymap.rs`:

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
index 38612faa..59e24f2e 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -250,8 +250,11 @@ pub enum When {
     NewRow,
     /// The list of a foreign key cell's picker is open under its field.
     KeyPicker,
-    /// A visual selection of rows is on, in the terminal look.
+    /// A visual selection is on, in the terminal look: of cells, a block
+    /// or rows.
     Visual,
+    /// The visual selection of rows (`V`) is on.
+    VisualRows,
     /// The chooser of a table's label column is open: beside a picker's
     /// list, which gives it the keys.
     LabelChooser,
@@ -524,6 +527,8 @@ commands! {
     MovePage = "move_page",
     ExtendUp = "extend_up",
     ExtendDown = "extend_down",
+    ExtendLeft = "extend_left",
+    ExtendRight = "extend_right",
     OpenInspector = "open_inspector",
     ToggleInspector = "toggle_inspector",
     CloseInspector = "close_inspector",
@@ -555,6 +560,7 @@ commands! {
     UndoCell = "undo_cell",
     RedoCell = "redo_cell",
     CopyCells = "copy_cells",
+    CopySelection = "copy_selection",
     CopyRow = "copy_row",
     PasteCells = "paste_cells",
     PasteNewRows = "paste_new_rows",
@@ -567,6 +573,10 @@ commands! {
     PutRowAbove = "put_row_above",
     DeleteRow = "delete_row",
     SelectRows = "select_rows",
+    SelectCells = "select_cells",
+    SelectBlock = "select_block",
+    SelectRowsOfCursor = "select_rows_of_cursor",
+    SelectAllRows = "select_all_rows",
     DeleteSelection = "delete_selection",
     LeaveVisual = "leave_visual",
     OpenReferencedRow = "open_referenced_row",
@@ -889,6 +899,8 @@ impl Command {
             C::MovePage => info("Move by page", "", Editing),
             C::ExtendUp => info("Extend the selection up", "", Editing),
             C::ExtendDown => info("Extend the selection down", "", Editing),
+            C::ExtendLeft => info("Extend the selection left", "", Editing),
+            C::ExtendRight => info("Extend the selection right", "", Editing),
             C::OpenInspector => info(
                 "Open inspector / focus fields",
                 "Open inspector / focus fields",
@@ -930,6 +942,7 @@ impl Command {
             C::UndoCell => info("Undo one cell", UNDO, Editing),
             C::RedoCell => info("Redo one cell", UNDO, Editing).reserved(),
             C::CopyCells => info("Copy cell values", COPY, Editing),
+            C::CopySelection => info("Copy the selection", COPY, Editing),
             C::CopyRow => info("Copy row", "", Editing),
             C::PasteCells => info("Paste cell values", COPY, Editing).reserved(),
             C::PasteNewRows => info("Paste as new rows", "Paste as new rows", Editing).reserved(),
@@ -950,8 +963,12 @@ impl Command {
             ),
             C::DeleteRow => info("Delete row", "Delete row", Editing),
             C::SelectRows => info("Select rows", "", Editing),
+            C::SelectCells => info("Select cells", "", Editing),
+            C::SelectBlock => info("Select a block of cells", "", Editing),
+            C::SelectRowsOfCursor => info("Select the rows of the selection", "", Editing),
+            C::SelectAllRows => info("Select all loaded rows", "", Editing),
             C::DeleteSelection => info("Delete the selected rows", "", Editing),
-            C::LeaveVisual => info("Leave the selection of rows", "", Editing),
+            C::LeaveVisual => info("Leave the selection", "", Editing),
             C::OpenReferencedRow => info("Open referenced row", "Open referenced row", Editing),
             C::ShowSavedRow => info(
                 "Show saved row in sorted position",
@@ -1612,9 +1629,11 @@ pub const BINDINGS: &[Binding] = {
             ]),
             keys(&[k("pgup"), k("pgdn"), k("home"), k("end")]),
         ),
-        // A table's rows only: a result has one selected row.
-        bind(C::ExtendUp, GRID, keys(&[k("shift+up")]), NONE),
-        bind(C::ExtendDown, GRID, keys(&[k("shift+down")]), NONE),
+        // A rectangle of cells, in a table's rows and a result's.
+        bind(C::ExtendUp, ROWS, keys(&[k("shift+up")]), NONE),
+        bind(C::ExtendDown, ROWS, keys(&[k("shift+down")]), NONE),
+        bind(C::ExtendLeft, ROWS, keys(&[k("shift+left")]), NONE),
+        bind(C::ExtendRight, ROWS, keys(&[k("shift+right")]), NONE),
         bind(
             C::OpenInspector,
             GRID,
@@ -1794,9 +1813,25 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k("cmd+shift+z")]),
             keys(&[k("ctrl+r")]).normal(),
         ),
-        bind(C::CopyCells, GRID, keys(&[k("cmd+c")]), keys(&[k("v y")])),
-        bind(C::CopyRow, GRID, keys(&[k("cmd+shift+c")]), NONE),
-        bind(C::PasteCells, GRID, keys(&[k("cmd+v")]), keys(&[k("v p")])),
+        // The terminal look copies and pastes from a visual selection: `v`
+        // starts one on the cursor's cell, and `y` copies it, so `v y`
+        // copies that cell as it always did.
+        bind(
+            C::CopyCells,
+            GRID,
+            keys(&[k("cmd+c")]),
+            keys(&[via("v"), via("y")]).shown("v y"),
+        ),
+        bind(
+            C::CopyCells,
+            RESULTS,
+            keys(&[k("cmd+c")]),
+            keys(&[via("v"), via("y")]).shown("v y"),
+        ),
+        bind(C::CopySelection, ROWS, NONE, keys(&[k("y")])).when(When::Visual),
+        bind(C::CopyRow, ROWS, keys(&[k("cmd+shift+c")]), NONE),
+        bind(C::PasteCells, GRID, keys(&[k("cmd+v")]), NONE),
+        bind(C::PasteCells, GRID, NONE, keys(&[k("p")]).shown("v p")).when(When::Visual),
         bind(
             C::PasteNewRows,
             GRID,
@@ -1834,11 +1869,17 @@ pub const BINDINGS: &[Binding] = {
         // other looks have a button there.
         bind(C::DeleteRow, INSPECTOR, NONE, keys(&[k("dd")])).when(When::NewRow),
         bind(C::AddRow, INSPECTOR, NONE, keys(&[k("o")])).when(When::NewRow),
-        // The terminal look's visual selection of rows: `V` starts and
-        // leaves it, and while it is on `d` alone deletes its rows.
-        bind(C::SelectRows, GRID, NONE, keys(&[k("V")])),
-        bind(C::DeleteSelection, GRID, NONE, keys(&[k("d")])).when(When::Visual),
-        bind(C::LeaveVisual, GRID, NONE, keys(&[k("esc")])).when(When::Visual),
+        // The terminal look's visual selections: `v` of cells, `ctrl+v` of
+        // a block, `V` of rows. Each key starts its own and leaves it, and
+        // while the rows' is on `d` alone deletes them.
+        bind(C::SelectRows, ROWS, NONE, keys(&[k("V")])),
+        bind(C::SelectCells, ROWS, NONE, keys(&[k("v")])),
+        bind(C::SelectBlock, ROWS, NONE, keys(&[k("ctrl+v")])),
+        bind(C::DeleteSelection, GRID, NONE, keys(&[k("d")])).when(When::VisualRows),
+        bind(C::LeaveVisual, ROWS, NONE, keys(&[k("esc")])).when(When::Visual),
+        // The rows of what is selected, and every row loaded.
+        bind(C::SelectRowsOfCursor, ROWS, keys(&[k("shift+space")]), NONE),
+        bind(C::SelectAllRows, ROWS, keys(&[k("cmd+a")]), NONE),
         bind(C::OpenReferencedRow, GRID, NONE, keys(&[k("gd")])),
         bind(C::ShowSavedRow, GRID, NONE, keys(&[k("gs")])),
         // The Structure view of a table, which is the grid's tab with its
```

`src/ui/keys.rs`:

```diff
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 28d71d12..eec82ab0 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -547,7 +547,7 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     let flips = object
         .filter(|_| grid && !editing && !tree_arrows && !focused)
         .filter(|&(tab, id)| app.flips(tab, id))
-        // Not under the terminal look's visual selection of rows.
+        // Not under the terminal look's visual selection.
         .filter(|&(tab, id)| {
             let object = app
                 .workspace(tab)
@@ -1117,13 +1117,38 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
                 actions.push(Action::CloseTab { tab, id });
             }
             if !editing && any_grid && !tree_arrows && !focused {
-                // Shift with an arrow takes the row it moves to into the
-                // selection. A table's rows only.
-                if sql.is_none() {
-                    for (command, rows) in [(Command::ExtendUp, -1), (Command::ExtendDown, 1)] {
-                        let presses = on(input, command, Scope::Grid).count;
-                        actions.extend((0..presses).map(|_| Action::ExtendBy { tab, id, rows }));
-                    }
+                // Shift with an arrow takes the cell it moves to into the
+                // selection, in a table's rows and a result's.
+                let rows_scope = if sql.is_some() {
+                    Scope::Results
+                } else {
+                    Scope::Grid
+                };
+                for (command, rows, cols) in [
+                    (Command::ExtendUp, -1, 0),
+                    (Command::ExtendDown, 1, 0),
+                    (Command::ExtendLeft, 0, -1),
+                    (Command::ExtendRight, 0, 1),
+                ] {
+                    let presses = on(input, command, rows_scope).count;
+                    actions.extend((0..presses).map(|_| Action::ExtendBy {
+                        tab,
+                        id,
+                        rows,
+                        cols,
+                    }));
+                }
+                if on(input, Command::SelectRowsOfCursor, rows_scope).fresh {
+                    actions.push(Action::SelectRowsOfCursor { tab, id });
+                }
+                if on(input, Command::SelectAllRows, rows_scope).fresh {
+                    actions.push(Action::SelectAllRows { tab, id });
+                }
+                // The terminal look's block: a chord, where `v` and `V`
+                // are letters.
+                if on(input, Command::SelectBlock, rows_scope).fresh {
+                    let mode = crate::model::Visual::Block;
+                    actions.push(Action::SelectRows { tab, id, mode });
                 }
                 // Down on a table's last row, on its last page, adds a
                 // row below it where the table takes one, in the looks
@@ -1275,15 +1300,16 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     if terminal {
         held_copy(ctx, open);
     }
-    if !editing && grid {
+    if !editing && any_grid {
         // The row's chord first: with Shift exact neither is the other's.
         // They come as the window's copy, which `presses` reads. A look
-        // with no chord for them copies nothing here.
+        // with no chord for them copies nothing here. A table's grid or a
+        // result's.
+        let scope = if grid { Scope::Grid } else { Scope::Results };
         let copied = ctx.input_mut(|input| {
             [Command::CopyRow, Command::CopyCells]
                 .into_iter()
                 .find(|command| {
-                    let scope = Scope::Grid;
                     asked(input, &app.keymap, layout, *command, scope, normal).count > 0
                 })
         });
@@ -1720,7 +1746,7 @@ fn editing_keys(
         asked(input, keymap, layout, command, scope, normal)
     };
     let open = object.edits.editor.is_some();
-    // The terminal look's visual selection of rows holds back the chords
+    // The terminal look's visual selection holds back the chords
     // that act on one cell or one row.
     let visual = app.look.terminal && object.visual();
     let editor = crate::ui::cell_editor::field_id(tab, id);
@@ -2067,10 +2093,10 @@ fn on_last_row(app: &App, tab: ConnTabId, id: TabId) -> bool {
         && place.is_some_and(|place| place + 1 == order.len())
 }
 
-/// Whether a visual selection of rows holds `command` back: what changes,
-/// copies or follows one cell or one row. The moves, the inspector's
-/// show and hide, a save, a discard, Review SQL, a reload, the filters and
-/// the pages are not among them.
+/// Whether a visual selection holds `command` back: what changes or
+/// follows one cell or one row, and what copies one row. The moves, the
+/// inspector's show and hide, a save, a discard, Review SQL, a reload, the
+/// filters, the pages and the copy of the selection are not among them.
 fn held_back(command: Command) -> bool {
     matches!(
         command,
@@ -2081,7 +2107,6 @@ fn held_back(command: Command) -> bool {
             | Command::SetDefault
             | Command::UndoCell
             | Command::RedoCell
-            | Command::CopyCells
             | Command::CopyRow
             | Command::PasteCells
             | Command::PasteNewRows
@@ -2285,7 +2310,7 @@ fn agents_here(app: &App, tab: ConnTabId) -> bool {
 
 /// Whether `sql` shows a result grid for the keys to move in: its last
 /// run's rows, unless the Messages pane covers them.
-fn shows_grid(sql: &crate::model::SqlTab) -> bool {
+pub(crate) fn shows_grid(sql: &crate::model::SqlTab) -> bool {
     sql.pane == crate::model::ResultPane::Results && sql.dims().0 > 0
 }
 
@@ -2550,9 +2575,18 @@ fn letters(
     // The cursor's row is a new one: the row panel is its form, and the
     // key that drops the row is read from the form's fields too.
     let on_new = selection.is_some_and(|cell| crate::edit::new_id(cell.row).is_some());
-    // A visual selection of rows is on: its keys are read, and the keys
-    // that act on one cell or one row are held back.
-    let visual = object.is_some_and(|object| object.visual());
+    // A visual selection is on, of a table's rows or a result's: its keys
+    // are read, and the keys that act on one cell or one row are held
+    // back. `V`'s holds the key that deletes its rows.
+    let visual_mode = match object {
+        Some(object) => object.visual_mode(),
+        None => workspace
+            .active_sql_tab()
+            .filter(|sql| shows_grid(sql))
+            .and_then(|sql| sql.visual_mode()),
+    };
+    let visual = visual_mode.is_some();
+    let visual_rows = visual_mode == Some(crate::model::Visual::Rows);
     let on_sql = workspace.active_sql_tab().is_some();
     // A SQL editor showing its result takes the letters that move in it.
     let sql_grid = workspace
@@ -2589,9 +2623,9 @@ fn letters(
     // field of the panel it gave the keys back to the grid, before this.)
     let quiet = !left_field && !menu_open;
     // A visual selection first: it is the innermost thing the key leaves.
-    if let Some(id) = active.filter(|_| visual)
+    if let Some(id) = active.or(sql_grid).filter(|_| visual)
         && quiet
-        && on(Command::LeaveVisual, Scope::Grid).fresh
+        && on(Command::LeaveVisual, rows).fresh
     {
         actions.push(Action::LeaveVisual { tab, id });
     }
@@ -2710,6 +2744,7 @@ fn letters(
         crate::keymap::When::TimedOut => timed_out,
         crate::keymap::When::NewRow => on_new,
         crate::keymap::When::Visual => visual,
+        crate::keymap::When::VisualRows => visual_rows,
         // The Structure view shows where the rows would: its own keys
         // come before the rows' there.
         crate::keymap::When::Structure => structure_shows,
@@ -3132,8 +3167,18 @@ fn letters(
                 let above = command == Command::PutRowAbove;
                 actions.push(Action::PutRow { tab, id, above });
             }
-            (Some(Command::SelectRows), Some(id)) if rows_show && !tree && field.is_none() => {
-                actions.push(Action::SelectRows { tab, id });
+            // The visual selections, of a table's rows or a result's.
+            (Some(command @ (Command::SelectRows | Command::SelectCells)), _)
+                if !tree && field.is_none() =>
+            {
+                let mode = if command == Command::SelectRows {
+                    crate::model::Visual::Rows
+                } else {
+                    crate::model::Visual::Cells
+                };
+                if let Some(id) = active.filter(|_| rows_show).or(sql_grid) {
+                    actions.push(Action::SelectRows { tab, id, mode });
+                }
             }
             // The rows of the visual selection. Where none of them is
             // deleted the selection stays, to be made smaller.
@@ -3241,10 +3286,15 @@ fn letters(
             }
             // The rows' cell, where the rows show: no other view copies
             // one it hides.
-            (Some(Command::CopyCells), Some(_)) if rows_show => {
+            // A visual selection's cells, of the rows or a result, and as
+            // vim's `y` does the selection is left.
+            (Some(Command::CopySelection), _) if active.is_none() || rows_show => {
                 if let Some(text) = app.copy_text(false) {
                     ctx.copy_text(text);
                 }
+                if let Some(id) = active.or(sql_grid).filter(|_| visual) {
+                    actions.push(Action::LeaveVisual { tab, id });
+                }
             }
             // The whole statements, never the lines as the panel shows
             // them. The copy shows nowhere, so the status line says it.
```

`src/ui/workspace.rs`:

```diff
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index 905e503d..16ce7666 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -1604,8 +1604,9 @@ struct Editing {
     can_edit: bool,
     /// Insert mode: the column being edited and its type.
     insert: Option<String>,
-    /// A visual selection of rows is on, with this many rows.
-    visual: Option<usize>,
+    /// A visual selection is on, of a table's rows or a result's: its mode,
+    /// and its size in words ("4 rows × 2 columns", "3 rows").
+    visual: Option<(crate::selection::Visual, String)>,
     /// Where Tab moves on to: the next cell from an editor on a cell, the
     /// next field from one in the row panel. Nowhere from the large
     /// editor, in either place: Tab is its text's own.
@@ -1687,7 +1688,18 @@ fn editing_status(app: &App, tab: ConnTabId) -> Editing {
         return Editing::default();
     };
     let Some(object) = workspace.active_object_tab() else {
-        return Editing::default();
+        // A result's visual selection: the line says it as a table's.
+        let visual = workspace.active_sql_tab().and_then(|sql| {
+            // Not under the Messages pane, where the rows do not show.
+            let mode = sql.visual_mode().filter(|_| super::keys::shows_grid(sql))?;
+            let size = sql.selection_size().unwrap_or((1, 1));
+            let rows = mode == crate::selection::Visual::Rows;
+            Some((mode, crate::selection::size_words(locale, size, rows)))
+        });
+        return Editing {
+            visual,
+            ..Editing::default()
+        };
     };
     let say = |text: &'static str| gettext(locale, text);
     let edits = &object.edits;
@@ -1725,8 +1737,12 @@ fn editing_status(app: &App, tab: ConnTabId) -> Editing {
         });
     let can_add = data
         && crate::edit::Table::of(workspace, object).is_some_and(|table| table.no_rows().is_none());
-    // The rows of a visual selection, while one is on and the rows show.
-    let visual = (data && object.visual()).then(|| object.selected_rows().len());
+    // A visual selection, while one is on and the rows show.
+    let visual = object.visual_mode().filter(|_| data).map(|mode| {
+        let size = object.selection_size().unwrap_or((1, 1));
+        let rows = mode == crate::selection::Visual::Rows;
+        (mode, crate::selection::size_words(locale, size, rows))
+    });
     let errors = counts.to_fix + counts.failed;
     let errors = (errors > 0).then(|| counted(locale, errors, "error", "errors"));
     // What a new row still needs, by its column: said while a value is
@@ -2124,6 +2140,7 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let (keep_key, next_key) = (key(Command::KeepEdit), key(Command::CommitNext));
     let (visual_delete_key, leave_visual_key) =
         (key(Command::DeleteSelection), key(Command::LeaveVisual));
+    let visual_copy_key = key(Command::CopySelection);
     // The keys of a foreign key's list, while one is open under the field.
     let pick_keys = app
         .keymap
@@ -2610,10 +2627,39 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     let text = Text::one(&look, TextRole::OModeLine, &label, color);
                     left += widgets::paint_label(ui, left, y, text) + gap;
                 }
-                let shown =
-                    fit(rect.right() - 12.0 - summary_width - gap - (left - rect.left() - 12.0));
-                let at = (left, y);
-                widgets::key_hints(ui, at, &hints[..shown], gap, &look, &palette);
+                // A visual selection of the result: its mode, its size and
+                // its keys, in place of the editor's.
+                if let Some((visual, held)) = &editing.visual {
+                    let mode = gettext(locale, visual.mode());
+                    let mode = Text::one(&look, TextRole::OModeLine, &mode, palette.magenta);
+                    left += widgets::paint_label(ui, left, y, mode) + gap;
+                    let text = Text::one(&look, role, held, palette.secondary);
+                    left += widgets::paint_label(ui, left, y, text) + gap;
+                    let (copies, normal) = (gettext(locale, "copy"), gettext(locale, "normal"));
+                    let keys = || {
+                        Text::new(&look)
+                            .add(role, &visual_copy_key, palette.text)
+                            .space(role, " ")
+                            .add(role, &copies, palette.dim)
+                            .add(role, " · ", palette.dim)
+                            .add(role, &leave_visual_key, palette.text)
+                            .space(role, " ")
+                            .add(role, &normal, palette.dim)
+                    };
+                    if left + widgets::measure(ui, keys())
+                        <= rect.right() - 12.0 - summary_width - gap
+                    {
+                        widgets::paint_label(ui, left, y, keys());
+                    }
+                } else {
+                    let shown = fit(rect.right()
+                        - 12.0
+                        - summary_width
+                        - gap
+                        - (left - rect.left() - 12.0));
+                    let at = (left, y);
+                    widgets::key_hints(ui, at, &hints[..shown], gap, &look, &palette);
+                }
                 widgets::paint_text_right(
                     ui,
                     rect.right() - 12.0,
@@ -2645,19 +2691,24 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 }
                 x
             };
-            if let Some(rows) = editing.visual {
-                // A visual selection of rows: the mode, how many rows it
-                // holds and the counts. Its keys, at the right, give way.
-                let mode = gettext(locale, "-- VISUAL --");
+            if let Some((visual, held)) = &editing.visual {
+                // A visual selection: the mode, what it holds and the
+                // counts. Its keys, at the right, give way: `V`'s rows
+                // are deleted, the others' cells copied.
+                let mode = gettext(locale, visual.mode());
                 let mode = || Text::one(&look, TextRole::OModeLine, &mode, palette.magenta);
-                let held = crate::ui::pending_bar::counted(locale, rows, "row", "rows");
-                let deletes = gettext(locale, "delete");
+                let rows = *visual == crate::selection::Visual::Rows;
+                let (act_key, acts) = if rows {
+                    (&visual_delete_key, gettext(locale, "delete"))
+                } else {
+                    (&visual_copy_key, gettext(locale, "copy"))
+                };
                 let normal = gettext(locale, "normal");
                 let keys = || {
                     Text::new(&look)
-                        .add(role, &visual_delete_key, palette.text)
+                        .add(role, act_key, palette.text)
                         .space(role, " ")
-                        .add(role, &deletes, palette.dim)
+                        .add(role, &acts, palette.dim)
                         .add(role, " · ", palette.dim)
                         .add(role, &leave_visual_key, palette.text)
                         .space(role, " ")
@@ -2665,7 +2716,7 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 };
                 let right = rect.right() - 12.0;
                 let mut x = left + widgets::paint_label(ui, left, y, mode()) + gap;
-                let text = Text::one(&look, role, &held, palette.secondary);
+                let text = Text::one(&look, role, held, palette.secondary);
                 x += widgets::paint_label(ui, x, y, text) + gap;
                 x = counts(ui, x);
                 if let Some(said) = &editing.said {
```

`src/ui/data_view.rs`:

```diff
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
index 010de8d6..2b792b0e 100644
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -54,13 +54,20 @@ fn state_note(selected: Option<String>, read_only: bool, locale: crate::i18n::Lo
 }
 
 /// What the footer says of the tab's selection: that a row is selected,
-/// or, on a new row, that it is one and how many values it still needs.
+/// how many rows or rows and columns a range covers, or, on a new row,
+/// that it is one and how many values it still needs.
 fn selection_note(
     workspace: &crate::model::Workspace,
     object: &ObjectTab,
     locale: crate::i18n::Locale,
 ) -> Option<String> {
     let cell = object.selection?;
+    // A rectangle of cells: how many rows and columns.
+    let cells = object.range.as_ref().is_some_and(|range| !range.rows());
+    if let Some(size) = object.selection_size().filter(|_| cells) {
+        let size = crate::selection::size_words(locale, size, false);
+        return Some(format!("{size} {}", gettext(locale, "selected")));
+    }
     // Several rows: how many, whatever kind each is.
     let rows = object.selected_rows().len();
     if rows > 1 {
@@ -1610,11 +1617,12 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
         )),
         (None, _) => None,
     };
-    // A visual selection of rows, and how many it holds: said when it
-    // starts and each time it grows or shrinks.
-    let visual = (look.terminal && object.visual()).then(|| {
-        let rows = object.selected_rows().len();
-        let held = super::pending_bar::counted(locale, rows, "row selected", "rows selected");
+    // A visual selection, and what it holds: said when it starts and each
+    // time it grows or shrinks.
+    let visual = object.visual_mode().filter(|_| look.terminal).map(|mode| {
+        let whole_rows = mode == crate::selection::Visual::Rows;
+        let size = object.selection_size().unwrap_or((1, 1));
+        let held = crate::selection::said(locale, size, whole_rows);
         format!("{} {held}.", gettext(locale, "Visual."))
     });
     // The tab itself from here on: the field on a cell edits the text its
@@ -1733,11 +1741,13 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
         // among them. A place is the grid's; a row is the set's and the
         // page's, and the reducer's.
         let order = crate::edit::Order::of(&object.edits.added, page.rows.len());
-        // The rows selected with the cursor's, as places of the grid. Read
-        // here, before the tab is taken for its editor's text.
-        let span = object.selected_span();
-        let ranged =
-            |place: usize| span.is_some_and(|(first, last)| (first..=last).contains(&place));
+        // The whole rows selected with the cursor's, as places of the grid.
+        // Read here, before the tab is taken for its editor's text.
+        let whole = object
+            .selected()
+            .map(|selected| selected.ranged())
+            .unwrap_or_default();
+        let ranged = |place: usize| whole.contains(&place);
         let placed = |cell: CellPos| {
             let row = order.place(cell.row)?;
             Some(CellPos { row, col: cell.col })
@@ -1950,7 +1960,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
             });
         }
         if let Some(cell) = output.clicked.and_then(held) {
-            // With Shift held the click takes the rows up to its own into
+            // With Shift held the click takes the cells up to its own into
             // the selection.
             actions.push(if output.shift {
                 Action::ExtendTo { tab, id, cell }
```

- [ ] **Step 4: Run the checks**

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
```

Expected: all pass; the library's tests: 3715 passed.

- [ ] **Step 5: Commit**

```bash
git add src/selection.rs src/lib.rs src/model.rs src/app.rs src/app/editing.rs src/ui/format.rs src/keymap.rs src/ui/keys.rs src/ui/workspace.rs src/ui/data_view.rs src/ui/delete_many_tests.rs src/ui/selection_tests.rs src/ui/mod.rs
git commit -m "Select cells and whole rows in both grids, with the terminal look's v, ctrl+v and V"
```

---

### Task 2: The grid: numbers, clicks, a drag, and the range drawn

**Files:** `src/ui/grid.rs`, `src/theme.rs`, `src/ui/data_view.rs`, `src/ui/sql_results.rs`, `src/ui/settings/preview.rs`, `src/testing.rs`, `src/ui/selection_tests.rs`, `src/ui/mod.rs`, and the website's `docs/_guide/{browsing-data,editing-data,macos,omarchy}.md` and `docs/_reference/keyboard-shortcuts.md`.

The gutter holds each row's number (44 on macOS, 50 after the terminal's marks); a click on it is a `RowNumberClicked` with what was held; a drag from a cell extends to the cell under the pointer. A rectangle is filled `accent_faint` with a 1-point accent outline on macOS and Windows, `surface_active` with its anchor outlined in the terminal look; the grid's group tells a screen reader the size. The row's bar moves to the row's start. Tests that found a cell by its text skip the numbers (`Harness::is_row_number`), and the grid's own tests place the pointer past the gutter.

- [ ] **Step 1: Write the failing tests**

`src/ui/selection_tests.rs`:

```diff
diff --git a/src/ui/selection_tests.rs b/src/ui/selection_tests.rs
index 2be7f0d4..82253ed7 100644
--- a/src/ui/selection_tests.rs
+++ b/src/ui/selection_tests.rs
@@ -6,6 +6,7 @@
 //! result; the colours only as the palette names them. No picture of the
 //! design is compared.
 
+use egui::accesskit::{Role, TreeUpdate};
 use egui::{Key, Modifiers};
 use tabletist_db::Value;
 
@@ -403,3 +404,277 @@ fn a_results_visual_selection_is_said_with_its_keys() {
     harness.settle();
     assert!(!painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
 }
+
+// The grid: its rows' numbers, the pointer, and what it draws and says.
+
+/// Where the row labelled `label` is.
+fn row_rect(tree: &TreeUpdate, label: &str) -> egui::Rect {
+    crate::testing::bounds(tree, label, Role::Button)
+        .unwrap_or_else(|| panic!("no row labelled {label:?}"))
+}
+
+/// A click of the pointer at `pos`, with `modifiers` held, as a window
+/// tells it: the modifiers first, then the pointer.
+fn click_with(harness: &mut Harness, pos: egui::Pos2, modifiers: Modifiers) {
+    let button = |pressed| egui::Event::PointerButton {
+        pos,
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers,
+    };
+    harness.frame(vec![
+        egui::Event::ModifiersChanged(modifiers),
+        egui::Event::PointerMoved(pos),
+    ]);
+    harness.frame(vec![button(true)]);
+    harness.frame(vec![button(false)]);
+    harness.frame(vec![egui::Event::ModifiersChanged(Modifiers::NONE)]);
+    harness.settle();
+}
+
+/// A click on the number of the row labelled `label`.
+fn click_number(harness: &mut Harness, label: &str, modifiers: Modifiers) {
+    let tree = harness.settle();
+    let row = row_rect(&tree, label);
+    click_with(
+        harness,
+        row.left_center() + egui::vec2(20.0, 0.0),
+        modifiers,
+    );
+}
+
+/// Where the cell of the row labelled `label` that writes `text` is.
+fn cell_at(harness: &mut Harness, label: &str, text: &str) -> egui::Pos2 {
+    let tree = harness.settle();
+    let row = row_rect(&tree, label);
+    harness
+        .text_rects
+        .iter()
+        .find(|(piece, rect)| {
+            piece == text
+                && row.y_range().contains(rect.center().y)
+                && !harness.is_row_number(*rect)
+        })
+        .map(|(_, rect)| rect.center())
+        .unwrap_or_else(|| panic!("no {text:?} in {label:?}"))
+}
+
+/// A drag of the pointer from `from` to `to`, over a few frames.
+fn drag(harness: &mut Harness, from: egui::Pos2, to: egui::Pos2) {
+    let button = |pos, pressed| egui::Event::PointerButton {
+        pos,
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![egui::Event::PointerMoved(from)]);
+    harness.frame(vec![button(from, true)]);
+    for step in 1..=4 {
+        let pos = from + (to - from) * (step as f32 / 4.0);
+        harness.frame(vec![egui::Event::PointerMoved(pos)]);
+    }
+    harness.frame(vec![button(to, false)]);
+    harness.settle();
+}
+
+/// What a screen reader is told of the grid's range.
+fn told(harness: &mut Harness) -> Option<String> {
+    let tree = harness.settle();
+    let mut nodes = tree.nodes.iter();
+    let grid = nodes.find(|(_, node)| node.label() == Some("Rows") && node.role() == Role::Group);
+    grid.and_then(|(_, node)| node.description().map(str::to_owned))
+}
+
+/// Test 2: a screen reader is told the range's size on the grid, and
+/// nothing with no range.
+#[test]
+fn the_range_is_told_to_a_screen_reader() {
+    for look in desktop() {
+        let (mut harness, tab, id) = covers_in(look);
+        select(&mut harness, tab, id, (0, 1));
+        assert_eq!(told(&mut harness), None, "{}", look.name);
+        harness.press(Key::ArrowDown, Modifiers::SHIFT);
+        harness.press(Key::ArrowDown, Modifiers::SHIFT);
+        harness.press(Key::ArrowRight, Modifiers::SHIFT);
+        assert_eq!(
+            told(&mut harness).as_deref(),
+            Some("3 rows by 2 columns selected")
+        );
+        harness.press(Key::Space, Modifiers::SHIFT);
+        assert_eq!(told(&mut harness).as_deref(), Some("3 rows selected"));
+        harness.press(Key::ArrowUp, Modifiers::NONE);
+        assert_eq!(told(&mut harness), None);
+    }
+}
+
+/// Test 2: a click with Shift runs a rectangle from the cursor's cell, and
+/// a drag from the cell it began on.
+#[test]
+fn shift_with_a_click_and_a_drag_select_cells() {
+    for look in desktop() {
+        let (mut harness, tab, id) = covers_in(look);
+        select(&mut harness, tab, id, (0, 2));
+        let at = cell_at(&mut harness, "Row 3", "2026-01-12 09:14:03");
+        click_with(&mut harness, at, Modifiers::SHIFT);
+        assert_eq!(rows(&harness, tab, id), [0, 1, 2], "{}", look.name);
+        assert_eq!(cols(&harness, tab, id), Some((2, 4)), "{}", look.name);
+        // A drag from the second row's kind to the first row's publisher.
+        let from = cell_at(&mut harness, "Row 2", "ebook");
+        let to = cell_at(&mut harness, "Row 1", "9100000000000000001");
+        drag(&mut harness, from, to);
+        assert_eq!(rows(&harness, tab, id), [0, 1], "{}", look.name);
+        assert_eq!(cols(&harness, tab, id), Some((1, 2)), "{}", look.name);
+        let workspace = harness.app.workspace(tab).unwrap();
+        let cursor = workspace.object_tab(id).unwrap().selection;
+        assert_eq!(cursor, Some(CellPos { row: 0, col: 1 }), "{}", look.name);
+    }
+}
+
+/// Test 2: a row's number selects the row, Shift runs to another, and
+/// Command takes one out or puts one in. Each row is told as selected,
+/// and a copy takes the rows whole.
+#[test]
+fn a_row_number_selects_extends_and_toggles_rows() {
+    for look in desktop() {
+        let (mut harness, tab, id) = covers_in(look);
+        click_number(&mut harness, "Row 1", Modifiers::NONE);
+        assert_eq!(rows(&harness, tab, id), [0], "{}", look.name);
+        assert!(whole_rows(&harness, tab, id), "{}", look.name);
+        assert_eq!(told(&mut harness).as_deref(), Some("1 row selected"));
+        click_number(&mut harness, "Row 3", Modifiers::SHIFT);
+        assert_eq!(rows(&harness, tab, id), [0, 1, 2], "{}", look.name);
+        assert!(painted(&harness, "3 rows selected"), "{}", look.name);
+        click_number(&mut harness, "Row 2", Modifiers::COMMAND);
+        assert_eq!(rows(&harness, tab, id), [0, 2], "{}", look.name);
+        let tree = harness.settle();
+        let selected = |label: &str| {
+            let mut nodes = tree.nodes.iter();
+            let node = nodes.find(|(_, node)| node.label() == Some(label));
+            node.and_then(|(_, node)| node.is_selected())
+        };
+        assert_eq!(selected("Row 1"), Some(true));
+        assert_eq!(selected("Row 2"), None);
+        assert_eq!(selected("Row 3"), Some(true));
+        // Whole rows, every column, with Shift's copy and the plain one.
+        let rows_copied = copied(&mut harness, true).unwrap();
+        assert_eq!(rows_copied.lines().count(), 2, "{rows_copied}");
+        assert!(rows_copied.starts_with("1\t9100000000000000001\tprint\t"));
+        assert!(rows_copied.contains("\n3\t9100000000000000003\tprint\t"));
+        assert_eq!(copied(&mut harness, false), Some(rows_copied));
+    }
+}
+
+/// Test 2 and 3: a SQL result's rows take the same selection, copy it, and
+/// say its mode in the terminal look.
+#[test]
+fn a_results_rows_take_the_same_selection() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        let tab = harness.connect_fake();
+        let id = super::tests::with_sql_result(&mut harness, tab, 3);
+        // The pointer on the first row's first cell, past its number.
+        let tree = harness.settle();
+        let row = row_rect(&tree, "Row 1");
+        let past = crate::ui::grid::gutter(&look) + 10.0;
+        click_with(
+            &mut harness,
+            row.left_center() + egui::vec2(past, 0.0),
+            Modifiers::NONE,
+        );
+        let sql = |harness: &Harness| {
+            let workspace = harness.app.workspace(tab).unwrap();
+            let sql = workspace.sql_tab(id).unwrap();
+            let selected = sql.selected().unwrap();
+            (selected.rows(), selected.cols(3))
+        };
+        if look.terminal {
+            type_key(&mut harness, Key::V, "v");
+            type_key(&mut harness, Key::J, "j");
+            type_key(&mut harness, Key::L, "l");
+            assert!(painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
+            assert!(painted(&harness, "2 rows × 2 columns"));
+            assert_eq!(sql(&harness), (vec![0, 1], Some((0, 1))));
+            harness.copied = None;
+            type_key(&mut harness, Key::Y, "y");
+        } else {
+            harness.press(Key::ArrowDown, Modifiers::SHIFT);
+            harness.press(Key::ArrowRight, Modifiers::SHIFT);
+            assert_eq!(sql(&harness), (vec![0, 1], Some((0, 1))));
+            assert_eq!(
+                told(&mut harness).as_deref(),
+                Some("2 rows by 2 columns selected")
+            );
+            harness.copied = None;
+            harness.copy(false);
+            // Its rows: a number, Shift+Space and Mod+A, and their copy.
+            let before = harness.copied.clone();
+            click_number(&mut harness, "Row 2", Modifiers::NONE);
+            assert_eq!(sql(&harness).0, [1]);
+            harness.press(Key::Space, Modifiers::SHIFT);
+            harness.press(Key::A, Modifiers::COMMAND);
+            assert_eq!(sql(&harness), (vec![0, 1, 2], Some((0, 2))));
+            let rows = copied(&mut harness, true).expect("the rows");
+            assert_eq!(rows.lines().count(), 3, "{rows}");
+            harness.copied = before;
+        }
+        let copied = harness.copied.clone().expect("a copy");
+        let lines: Vec<&str> = copied.lines().collect();
+        assert_eq!(lines.len(), 2, "{}: {copied}", look.name);
+        assert!(lines.iter().all(|line| line.split('\t').count() == 2));
+        assert!(lines[0].starts_with("1\t"), "{copied}");
+    }
+}
+
+/// Test 6: a range of cells is filled as the palette names it, in each
+/// look, and the rows' numbers are written in theirs.
+#[test]
+fn a_range_and_the_row_numbers_are_painted_in_their_colours() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, tab, id) = covers_in(look);
+        let palette = harness.app.palette;
+        let (fill, number) = if look.terminal {
+            (palette.surface_active, palette.dim)
+        } else {
+            (palette.accent_faint, palette.faint)
+        };
+        harness.settle();
+        let numbered =
+            harness
+                .text_rects
+                .iter()
+                .zip(&harness.painted)
+                .any(|((text, rect), (_, color))| {
+                    text == "3" && *color == number && harness.is_row_number(*rect)
+                });
+        assert!(numbered, "{}: the third row's number", look.name);
+        let filled = |harness: &Harness| {
+            let cell = look.row();
+            let cells = harness.fills.iter();
+            cells
+                .filter(|(rect, color)| *color == fill && rect.height() == cell)
+                .count()
+        };
+        let before = filled(&harness);
+        select(&mut harness, tab, id, (0, 1));
+        if look.terminal {
+            type_key(&mut harness, Key::V, "v");
+            type_key(&mut harness, Key::J, "j");
+            type_key(&mut harness, Key::L, "l");
+        } else {
+            harness.press(Key::ArrowDown, Modifiers::SHIFT);
+            harness.press(Key::ArrowRight, Modifiers::SHIFT);
+        }
+        harness.settle();
+        // Four cells, the cursor's among them in its own colour.
+        assert_eq!(filled(&harness) - before, 3, "{}", look.name);
+        // The terminal outlines the range's anchor in the accent.
+        if look.terminal {
+            let accent = palette.accent;
+            let outlined = harness.outlines.iter().any(|(rect, stroke)| {
+                stroke.color == accent && stroke.width == 1.0 && rect.height() == look.row()
+            });
+            assert!(outlined, "the anchor's outline");
+        }
+    }
+}
```

`src/ui/mod.rs`:

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 20ada614..cae61c30 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -14130,11 +14130,13 @@ value_tags   = true
     }
 
     /// The middle of the cell that shows `text`.
+    /// Where a cell writes `text`: not a row's number.
     fn cell_of(harness: &Harness, text: &str) -> egui::Pos2 {
-        harness
-            .painted_rect(text)
+        let mut pieces = harness.text_rects.iter();
+        pieces
+            .find(|(piece, rect)| piece == text && !harness.is_row_number(*rect))
+            .map(|(_, rect)| rect.center())
             .unwrap_or_else(|| panic!("no cell shows {text}"))
-            .center()
     }
 
     fn edits(harness: &Harness, tab: ConnTabId, id: TabId) -> &crate::edit::Edits {
@@ -14900,7 +14902,8 @@ value_tags   = true
             // It is a note at the cell: beside the key it is about.
             // (The row panel writes the key too: the grid's is leftmost.)
             let note = harness.painted_rect(why).unwrap();
-            let keys = harness.text_rects.iter().filter(|(text, _)| text == "2");
+            let pieces = harness.text_rects.iter();
+            let keys = pieces.filter(|(text, rect)| text == "2" && !harness.is_row_number(*rect));
             let cell = keys
                 .map(|(_, rect)| *rect)
                 .min_by(|a, b| a.left().total_cmp(&b.left()))
@@ -17283,7 +17286,9 @@ value_tags   = true
         make_pending(&mut harness, tab, id, (1, 1), "ann@example.com");
         harness.press(Key::S, Modifiers::COMMAND);
         harness.answer_written(Err(tabletist_db::Error::Cancelled));
-        harness.size.x = 720.0;
+        // Narrow enough that the keys give way, wide enough for the grid
+        // and the rows' numbers to keep every column in sight.
+        harness.size.x = 820.0;
         harness.settle();
         let said = "save cancelled. nothing was written.";
         assert!(painted(&harness, said), "{:?}", harness.painted);
@@ -24423,10 +24428,14 @@ value_tags   = true
         for look in Look::ALL {
             let (mut harness, tab, id) = editing_a_document(look);
             let palette = harness.app.palette;
+            // The editor's, not the grid's row numbers: those are written
+            // in the same colour.
             let numbered = |harness: &Harness, number: &str| {
                 let pieces = harness.text_rects.iter().zip(&harness.painted);
                 pieces
-                    .filter(|((text, _), (_, color))| text == number && *color == palette.faint)
+                    .filter(|((text, rect), (_, color))| {
+                        text == number && *color == palette.faint && !harness.is_row_number(*rect)
+                    })
                     .count()
             };
             let valid = |harness: &Harness| {
```

`src/testing.rs`:

```diff
diff --git a/src/testing.rs b/src/testing.rs
index 7922289f..6c91084f 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -31,6 +31,9 @@ pub struct Harness {
     pub painted: Vec<(String, egui::Color32)>,
     /// Where the last frame painted each piece of text, in points.
     pub text_rects: Vec<(String, egui::Rect)>,
+    /// Where the last frame drew each row of a grid, as a screen reader
+    /// has it: what [`Harness::is_row_number`] reads.
+    pub grid_rows: Vec<egui::Rect>,
     /// Every piece of text the last frame painted slanted.
     pub slanted: Vec<String>,
     /// Every piece of text the last frame painted struck through.
@@ -126,10 +129,35 @@ impl Harness {
                 egui::OutputCommand::CopyImage(_) => {}
             }
         }
-        output
+        let tree = output
             .platform_output
             .accesskit_update
-            .expect("AccessKit is enabled")
+            .expect("AccessKit is enabled");
+        self.grid_rows = tree
+            .nodes
+            .iter()
+            .filter(|(_, node)| {
+                node.label()
+                    .is_some_and(|label| label.starts_with("Row ") || label == "New row, not saved")
+            })
+            .filter_map(|(_, node)| node.bounds())
+            .map(|rect| {
+                egui::Rect::from_min_max(
+                    egui::pos2(rect.x0 as f32, rect.y0 as f32),
+                    egui::pos2(rect.x1 as f32, rect.y1 as f32),
+                )
+            })
+            .collect();
+        tree
+    }
+
+    /// Whether the piece of text the last frame painted at `rect` is a
+    /// grid's row number: in the gutter its row begins with.
+    pub fn is_row_number(&self, rect: egui::Rect) -> bool {
+        let gutter = crate::ui::grid::gutter(&self.app.look);
+        self.grid_rows.iter().any(|row| {
+            row.y_range().contains(rect.center().y) && rect.right() <= row.left() + gutter
+        })
     }
 
     /// Runs one frame that draws only `add` in a central panel, for
@@ -1687,6 +1715,7 @@ impl Harness {
             close_requested: false,
             painted: Vec::new(),
             text_rects: Vec::new(),
+            grid_rows: Vec::new(),
             slanted: Vec::new(),
             struck: Vec::new(),
             fills: Vec::new(),
```

`src/ui/grid.rs`:

```diff
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
index 233c22df..78609d03 100644
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -2117,6 +2117,7 @@ mod tests {
                     columns,
                     rows,
                     None,
+                    Picked::default(),
                     false,
                     &palette,
                     &crate::theme::Look::standard(),
@@ -2172,6 +2173,7 @@ mod tests {
                 &columns(),
                 3,
                 Some(CellPos { row: 1, col: 1 }),
+                Picked::default(),
                 keys,
                 palette,
                 look,
@@ -2305,6 +2307,7 @@ mod tests {
                 &columns(),
                 3,
                 selection,
+                Picked::default(),
                 true,
                 palette,
                 look,
@@ -2698,6 +2701,7 @@ mod tests {
                     &columns(),
                     3,
                     None,
+                    Picked::default(),
                     false,
                     &palette,
                     &look,
@@ -2762,6 +2766,7 @@ mod tests {
                     &columns(),
                     3,
                     None,
+                    Picked::default(),
                     false,
                     &palette,
                     &look,
@@ -3001,7 +3006,8 @@ mod tests {
         let widths: Vec<f32> = ctx
             .data(|data| data.get_temp(egui::Id::new("grid").with(("widths", 2_usize))))
             .expect("the widths");
-        let pos = egui::pos2(widths[0] + 5.0, header_height(&look) + look.grid_row * 1.5);
+        let x = gutter(&look) + widths[0] + 5.0;
+        let pos = egui::pos2(x, header_height(&look) + look.grid_row * 1.5);
         let press = |pressed| egui::Event::PointerButton {
             pos,
             button: egui::PointerButton::Primary,
@@ -3060,6 +3066,7 @@ mod tests {
                     &columns,
                     3,
                     None,
+                    Picked::default(),
                     false,
                     &Palette::light(),
                     &Look::standard(),
@@ -3131,7 +3138,7 @@ mod tests {
         assert!(scrolled.partial(), "{scrolled:?}");
         // The key is where it was; the column after it went under it.
         assert_eq!(at(&texts, "r0c0"), Some(before));
-        assert!(at(&texts, "r0c1").is_none_or(|x| x < before));
+        assert!(at(&texts, "r0c1").is_none_or(|x| f64::from(x) < header(0).x1));
         // A click over the key picks the key's cell, not one under it.
         let over_key = egui::pos2(before + 4.0, 45.0 + 26.0 * 1.5);
         let press = |pressed| egui::Event::PointerButton {
@@ -3186,6 +3193,7 @@ mod tests {
                         &columns,
                         3,
                         None,
+                        Picked::default(),
                         false,
                         &palette,
                         &look,
@@ -3515,7 +3523,7 @@ mod tests {
         };
         let before = widths(&ctx)[0];
         // A point right of the first column's edge, inside its handle.
-        let pos = egui::pos2(before + 1.0, header_height(&look) / 2.0);
+        let pos = egui::pos2(gutter(&look) + before + 1.0, header_height(&look) / 2.0);
         let press = |pos, pressed| egui::Event::PointerButton {
             pos,
             button: egui::PointerButton::Primary,
@@ -3565,7 +3573,8 @@ mod tests {
                 egui::RawInput {
                     screen_rect: Some(egui::Rect::from_min_size(
                         egui::Pos2::ZERO,
-                        egui::vec2(160.0, 300.0),
+                        // As wide as the gutter more than the key and half the email.
+                        egui::vec2(160.0 + gutter(&look), 300.0),
                     )),
                     events,
                     ..Default::default()
@@ -3577,6 +3586,7 @@ mod tests {
                         &columns,
                         3,
                         None,
+                        Picked::default(),
                         false,
                         &Palette::light(),
                         &look,
@@ -3620,12 +3630,14 @@ mod tests {
         let (_, tree) = frame(vec![]);
         let key = crate::testing::bounds(&tree, "id", Role::Button).expect("the key's header");
         let email = crate::testing::bounds(&tree, "email", Role::Button).expect("its header");
-        assert_eq!(key.left(), 0.0);
-        assert_eq!(key.right(), widths[0]);
+        // After the rows' numbers.
+        let gutter = gutter(&look);
+        assert_eq!(key.left(), gutter);
+        assert_eq!(key.right(), gutter + widths[0]);
         // The email header begins where the key's ends, and ends where
         // its column does.
         assert_eq!(email.left(), key.right());
-        assert_eq!(email.right(), widths[0] + widths[1] - half);
+        assert_eq!(email.right(), gutter + widths[0] + widths[1] - half);
         // Over the email header's hidden half, clear of the key's handle,
         // the pointer is the key header's, and so is a click.
         let hovered = |col: usize| {
@@ -3635,7 +3647,10 @@ mod tests {
         };
         let y = header_height(&look) / 2.0;
         let covered = egui::pos2(key.right() - HANDLE_WIDTH, y);
-        assert!(covered.x > widths[0] - half, "over the email header");
+        assert!(
+            covered.x > gutter + widths[0] - half,
+            "over the email header"
+        );
         frame(vec![egui::Event::PointerMoved(covered)]);
         frame(vec![]);
         assert!(hovered(0) && !hovered(1));
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib`

Expected: the tests do not compile, as the grid's gutter and the palette's colour are not there yet:

```
error[E0425]: cannot find function `gutter` in module `crate::ui::grid`
error[E0433]: cannot find type `Picked` in this scope
error[E0425]: cannot find function `gutter` in this scope
error[E0277]: expected an `Fn(usize)` closure, found `theme::Look`
error[E0277]: expected an `FnMut(usize, usize)` closure, found `std::option::Option<_>`
error[E0061]: this function takes 12 arguments but 13 arguments were supplied
error[E0609]: no field `accent_faint` on type `theme::Palette`
```

- [ ] **Step 3: Write the code**

`src/ui/grid.rs`:

```diff
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
index 78609d03..b9d6b4cc 100644
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -31,6 +31,20 @@ pub fn header_height(look: &crate::theme::Look) -> f32 {
 }
 /// The terminal look's gutter, where the selected row's cursor sits.
 const GUTTER: f32 = 22.0;
+/// The rows' numbers, before the first column (after the terminal look's
+/// gutter): 44 wide on macOS, 50 in the terminal, as the design has them.
+fn numbers_width(look: &Look) -> f32 {
+    if look.terminal { 50.0 } else { 44.0 }
+}
+/// The space a number keeps from the column after it.
+const NUMBER_PAD: f32 = 8.0;
+
+/// What stands before the first column: the terminal look's marks, then
+/// the rows' numbers.
+pub fn gutter(look: &Look) -> f32 {
+    let signs = if look.terminal { GUTTER } else { 0.0 };
+    signs + numbers_width(look)
+}
 const MIN_WIDTH: f32 = 48.0;
 const MAX_INITIAL_WIDTH: f32 = 331.0;
 /// A numeric key column's width, as the design's grids fix it: 64 on
@@ -88,6 +102,17 @@ pub struct Row {
     pub ranged: bool,
 }
 
+/// What is selected besides the cursor's cell, as the grid draws and says
+/// it. Whole rows are each [`Row::ranged`].
+#[derive(Debug, Clone, Copy, Default)]
+pub struct Picked<'a> {
+    /// A rectangle of cells, as places of the grid.
+    pub block: Option<crate::selection::Block>,
+    /// What a screen reader is told of the range: "4 rows by 2 columns
+    /// selected".
+    pub said: Option<&'a str>,
+}
+
 /// How a cell draws its text.
 #[derive(Clone, Copy, Debug, Default, PartialEq)]
 pub enum Style {
@@ -169,7 +194,12 @@ pub type Editor<'a> = &'a mut dyn FnMut(&mut Ui, Rect);
 #[derive(Debug, Default, PartialEq)]
 pub struct GridOutput {
     pub clicked: Option<CellPos>,
-    /// Shift was held at the click: it extends the selection to its row.
+    /// A row's number was clicked: its place, and with what held.
+    pub row_clicked: Option<(usize, crate::selection::RowClick)>,
+    /// The cell a drag that began on a cell is over now, where that changed
+    /// this frame: the range runs from the pressed cell to it.
+    pub dragged: Option<CellPos>,
+    /// Shift was held at the click: it extends the selection to its cell.
     pub shift: bool,
     /// The cell a second click landed on, soon after the first.
     pub double_clicked: Option<CellPos>,
@@ -425,6 +455,39 @@ pub fn row_fill(
     }
 }
 
+/// The outline of a range of cells, drawn on its cell at `(place, col)`: on
+/// macOS a line on each side the range ends at, in the terminal a line
+/// round its anchor.
+fn block_edges(
+    painter: &egui::Painter,
+    rect: Rect,
+    block: crate::selection::Block,
+    (place, col): (usize, usize),
+    look: &Look,
+    palette: &Palette,
+) {
+    let stroke = Stroke::new(1.0, palette.accent);
+    if look.terminal {
+        if block.anchor == (CellPos { row: place, col }) {
+            painter.rect_stroke(rect, CornerRadius::ZERO, stroke, StrokeKind::Inside);
+        }
+        return;
+    }
+    let inner = rect.shrink(0.5);
+    if place == block.rows.0 {
+        painter.hline(inner.x_range(), inner.top(), stroke);
+    }
+    if place == block.rows.1 {
+        painter.hline(inner.x_range(), inner.bottom(), stroke);
+    }
+    if col == block.cols.0 {
+        painter.vline(inner.left(), inner.y_range(), stroke);
+    }
+    if col == block.cols.1 {
+        painter.vline(inner.right(), inner.y_range(), stroke);
+    }
+}
+
 /// `palette` with every colour a cell writes in set to `color`.
 fn written_in(palette: &Palette, color: egui::Color32) -> Palette {
     Palette {
@@ -649,6 +712,7 @@ pub fn show<'a>(
     columns: &[Column<'_>],
     row_count: usize,
     selection: Option<CellPos>,
+    picked: Picked<'_>,
     // Whether the arrow keys move in this grid.
     keys: bool,
     palette: &Palette,
@@ -664,7 +728,7 @@ pub fn show<'a>(
     let row_height = look.row();
     let header_height = header_height(look);
     let pad = cell_pad(look);
-    let gutter = if look.terminal { GUTTER } else { 0.0 };
+    let gutter = gutter(look);
     let widths_id = id.with(("widths", columns.len()));
     let last_id = id.with("last-selection");
     let kept: Option<Vec<f32>> = ui
@@ -729,6 +793,9 @@ pub fn show<'a>(
     stop.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, "Rows"));
     ui.ctx().accesskit_node_builder(stop.id, |node| {
         node.set_role(egui::accesskit::Role::Group);
+        if let Some(said) = picked.said {
+            node.set_description(said);
+        }
     });
     focus::pane(ui, &stop);
     // The selected cell shows where the keyboard is; with none selected
@@ -797,9 +864,27 @@ pub fn show<'a>(
             ui.scroll_to_rect(rect, None);
         }
 
+        // The cell under `pointer`, as places: what a drag is over.
+        let under = |pointer: egui::Pos2| {
+            let y = pointer.y - origin.y - header_height;
+            let place = (y / row_height).floor().max(0.0) as usize;
+            let x = pointer.x - origin.x - gutter;
+            let col = if pinned && x - shift < widths[0] {
+                0
+            } else {
+                column_at(&widths, x)
+            };
+            CellPos {
+                row: place.min(row_count.saturating_sub(1)),
+                col,
+            }
+        };
+        let drag_id = id.with("drag-over");
         virtual_rows(ui, row_count, row_height, |ui, row| {
-            // A row takes a click, not the Tab key: the grid is the stop.
-            let (rect, response) = ui.allocate_exact_size(vec2(full, row_height), Sense::CLICK);
+            // A row takes a click and a drag, not the Tab key: the grid is
+            // the stop.
+            let (rect, response) =
+                ui.allocate_exact_size(vec2(full, row_height), Sense::click_and_drag());
             let Row {
                 mark,
                 number,
@@ -817,9 +902,23 @@ pub fn show<'a>(
                     node.set_selected(true);
                 });
             }
-            if response.clicked() || response.double_clicked() {
-                let col = response
-                    .interact_pointer_pos()
+            // On the row's number, which stands with the pinned column.
+            let on_number = |pointer: egui::Pos2| pointer.x - rect.left() - shift < gutter;
+            let pressed = response.interact_pointer_pos();
+            if (response.clicked() || response.double_clicked()) && pressed.is_some_and(on_number) {
+                let held = ui.input(|input| input.modifiers);
+                let click = if held.shift {
+                    crate::selection::RowClick::Extend
+                } else if held.command {
+                    crate::selection::RowClick::Toggle
+                } else {
+                    crate::selection::RowClick::Only
+                };
+                if response.clicked() {
+                    output.row_clicked = Some((row, click));
+                }
+            } else if response.clicked() || response.double_clicked() {
+                let col = pressed
                     .map(|pointer| {
                         let x = pointer.x - rect.left() - gutter;
                         // Over the pinned column, whatever is under it.
@@ -839,6 +938,31 @@ pub fn show<'a>(
                     output.double_clicked = cell;
                 }
             }
+            // A drag from a cell: the press is its click, and the cell the
+            // pointer is over the other end of the range.
+            let origin_press = ui.input(|input| input.pointer.press_origin());
+            if response.drag_started()
+                && let Some(press) = origin_press.filter(|press| !on_number(*press))
+            {
+                let from = CellPos {
+                    row,
+                    col: under(press).col,
+                };
+                output.clicked = Some(from);
+                output.shift = ui.input(|input| input.modifiers.shift);
+                ui.data_mut(|data| data.insert_temp(drag_id, from));
+            }
+            if response.dragged()
+                && origin_press.is_some_and(|press| !on_number(press))
+                && let Some(pointer) = response.interact_pointer_pos()
+            {
+                let over = under(pointer);
+                let before = ui.data(|data| data.get_temp::<CellPos>(drag_id));
+                if before != Some(over) {
+                    output.dragged = Some(over);
+                    ui.data_mut(|data| data.insert_temp(drag_id, over));
+                }
+            }
             let painter = ui.painter().clone();
             // A row with pending cells is marked in their tone.
             let row_tone = match mark {
@@ -852,9 +976,10 @@ pub fn show<'a>(
             // having no number among the table's. Its mark is not
             // steady enough, a cell to fix in it makes it `Trouble`.
             let held_back = new || number.is_none();
-            // The cursor's row, and with it the rows selected with it.
+            // The cursor's row, and with it the rows selected with it. A
+            // rectangle of cells is drawn on its cells alone.
             let cursor_row = selection.is_some_and(|cell| cell.row == row);
-            let selected_row = cursor_row || ranged;
+            let selected_row = (cursor_row && picked.block.is_none()) || ranged;
             // With the keyboard in the grid the cell takes the
             // selection's colour and its row a lighter tint of it.
             let lit_row = selected_row && lit && !look.terminal;
@@ -919,6 +1044,19 @@ pub fn show<'a>(
                             Stroke::new(hairline, palette.outline),
                         );
                     }
+                    if let Some(number) = number {
+                        // The row's number, at the right of its room.
+                        let color = if look.terminal {
+                            palette.dim
+                        } else {
+                            palette.faint
+                        };
+                        let role = TextRole::pick(look, TextRole::ColumnType, TextRole::OBody);
+                        let right = lead.left() + gutter - NUMBER_PAD;
+                        Text::one(look, role, &number.to_string(), color)
+                            .layout(ui.ctx())
+                            .paint_right(&painter, right, rect.center().y);
+                    }
                     if look.terminal {
                         if new {
                             // The terminal's bar: 2 wide, and as far
@@ -972,6 +1110,15 @@ pub fn show<'a>(
                 }
                 let content = cell(row, col);
                 let here = selection == Some(CellPos { row, col });
+                let block = picked.block.filter(|block| block.covers(row, col));
+                let in_block = block.is_some();
+                // A range of cells: the accent's faint tint on macOS, the
+                // active surface in the terminal.
+                let block_fill = if look.terminal {
+                    palette.surface_active
+                } else {
+                    palette.accent_faint
+                };
                 if content.hint.is_some() || content.note.is_some() {
                     // Where the cell shows: not the part of it that
                     // scrolled under the pinned column.
@@ -1047,6 +1194,8 @@ pub fn show<'a>(
                     painter.rect_filled(cell_rect, CornerRadius::ZERO, tint);
                 } else if here && lit {
                     painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.selection);
+                } else if in_block {
+                    painter.rect_filled(cell_rect, CornerRadius::ZERO, block_fill);
                 } else if locked {
                     painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.surface);
                 }
@@ -1096,7 +1245,9 @@ pub fn show<'a>(
                         None => (selected_row && !lit_row).then_some(palette.accent),
                     };
                     if let Some(color) = color {
-                        let bar = Rect::from_min_size(cell_rect.min, vec2(3.0, height));
+                        // At the row's start, before its number.
+                        let at = pos2(cell_rect.left() - gutter, cell_rect.top());
+                        let bar = Rect::from_min_size(at, vec2(3.0, height));
                         painter.rect_filled(bar, CornerRadius::ZERO, color);
                     }
                 }
@@ -1155,6 +1306,9 @@ pub fn show<'a>(
                     look,
                     &written,
                 );
+                if let Some(block) = block {
+                    block_edges(&painter, cell_rect, block, (row, col), look, palette);
+                }
                 // A cell edited in an editor of its own keeps its
                 // value and the cursor's line, lit or not: the line
                 // says which cell the editor is for.
@@ -1182,11 +1336,9 @@ pub fn show<'a>(
                     // marked too, for the keys that act on one cell.
                     // A new row's first is as well: the row keeps its
                     // own fill and bar, and nothing else would say it
-                    // is selected. The line stands clear of that bar.
-                    let clear = if col == 0 { 3.0 } else { 0.0 };
-                    let marked = cell_rect.with_min_x(cell_rect.left() + clear);
+                    // is selected.
                     painter.rect_stroke(
-                        marked.shrink(1.0),
+                        cell_rect.shrink(1.0),
                         CornerRadius::same(look.radius.min(3)),
                         Stroke::new(1.5, palette.accent),
                         StrokeKind::Inside,
```

`src/theme.rs`:

```diff
diff --git a/src/theme.rs b/src/theme.rs
index ea8e17c6..2780583d 100644
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -51,6 +51,8 @@ pub struct Palette {
     pub border: Color32,
     /// The selected row or item.
     pub selection: Color32,
+    /// The cells of a selected range, a step quieter than the selected row.
+    pub accent_faint: Color32,
     /// Verified TLS, JSON strings, the dev environment.
     pub success: Color32,
     /// The terminal's second value tag.
@@ -113,6 +115,7 @@ impl Palette {
             faint: Color32::from_rgb(0x8a, 0x88, 0x82),
             border: Color32::from_rgb(0x44, 0x43, 0x3f),
             selection: Color32::from_rgb(0x2b, 0x35, 0x50),
+            accent_faint: Color32::from_rgb(0x2a, 0x32, 0x48),
             success: Color32::from_rgb(0x8f, 0xd1, 0x9e),
             info: Color32::from_rgb(0x9f, 0xbd, 0xf0),
             orange: Color32::from_rgb(0xf0, 0xa8, 0x68),
@@ -169,6 +172,7 @@ impl Palette {
             faint: Color32::from_rgb(0x8a, 0x89, 0x84),
             border: Color32::from_rgb(0xdc, 0xda, 0xd4),
             selection: Color32::from_rgb(0xe4, 0xeb, 0xfb),
+            accent_faint: Color32::from_rgb(0xee, 0xf2, 0xfc),
             success: Color32::from_rgb(0x14, 0x6b, 0x3a),
             info: Color32::from_rgb(0x1e, 0x4f, 0x8a),
             orange: Color32::from_rgb(0x9a, 0x4a, 0x0b),
@@ -227,6 +231,7 @@ impl fastframe_theme::Palette for Palette {
             "faint" => self.faint = color,
             "border" => self.border = color,
             "selection" => self.selection = color,
+            "accent_faint" => self.accent_faint = color,
             "success" => self.success = color,
             "info" => self.info = color,
             "orange" => self.orange = color,
@@ -256,6 +261,9 @@ impl fastframe_theme::Palette for Palette {
         if given.contains("accent") && !given.contains("selection") {
             self.selection = self.window.lerp_to_gamma(self.accent, 0.2);
         }
+        if given.contains("accent") && !given.contains("accent_faint") {
+            self.accent_faint = self.window.lerp_to_gamma(self.accent, 0.1);
+        }
         // Value tags follow a file's own colours: its accent as the blue,
         // and its red and green where it names no rose or olive (which
         // then fall back to muted).
```

`src/ui/data_view.rs`:

```diff
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
index 2b792b0e..093532a5 100644
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -1741,12 +1741,23 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
         // among them. A place is the grid's; a row is the set's and the
         // page's, and the reducer's.
         let order = crate::edit::Order::of(&object.edits.added, page.rows.len());
-        // The whole rows selected with the cursor's, as places of the grid.
-        // Read here, before the tab is taken for its editor's text.
-        let whole = object
-            .selected()
+        // What is selected with the cursor's cell, as places of the grid:
+        // whole rows, or a rectangle of cells. Read here, before the tab is
+        // taken for its editor's text.
+        let width = page.columns.len();
+        let selected = object.selected();
+        let whole = selected
             .map(|selected| selected.ranged())
             .unwrap_or_default();
+        let block = selected.and_then(|selected| selected.block(width));
+        let said = selected.and_then(|selected| {
+            let whole_rows = object
+                .range
+                .as_ref()
+                .is_some_and(crate::selection::Range::rows);
+            let size = selected.size(width)?;
+            Some(crate::selection::said(locale, size, whole_rows))
+        });
         let ranged = |place: usize| whole.contains(&place);
         let placed = |cell: CellPos| {
             let row = order.place(cell.row)?;
@@ -1800,6 +1811,10 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
             &columns,
             order.len(),
             object.selection.and_then(placed),
+            grid::Picked {
+                block,
+                said: said.as_deref(),
+            },
             keys,
             &palette,
             &grid_look,
@@ -1968,6 +1983,20 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
                 Action::SelectCell { tab, id, cell }
             });
         }
+        // A drag over the cells: the range runs to the one under it.
+        if let Some(cell) = output.dragged.and_then(held) {
+            actions.push(Action::ExtendTo { tab, id, cell });
+        }
+        if let Some((place, click)) = output.row_clicked
+            && let Some(row) = order.row(place)
+        {
+            actions.push(Action::RowNumberClicked {
+                tab,
+                id,
+                row,
+                click,
+            });
+        }
         // A second click edits the cell.
         if let Some(cell) = output.double_clicked.and_then(held) {
             let start = EditStart::Value;
```

`src/ui/sql_results.rs`:

```diff
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
index b78bfc22..f41cd3d2 100644
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -1937,6 +1937,21 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
         .iter()
         .map(|column| Tags::of(column, None).when(fit.value_tags))
         .collect();
+    // What is selected with the cursor's cell: whole rows, or a rectangle
+    // of cells. A result's places are its rows.
+    let selected = sql.selected();
+    let whole = selected
+        .map(|selected| selected.ranged())
+        .unwrap_or_default();
+    let block = selected.and_then(|selected| selected.block(columns.len()));
+    let said = selected.and_then(|selected| {
+        let whole_rows = sql
+            .range
+            .as_ref()
+            .is_some_and(crate::selection::Range::rows);
+        let size = selected.size(columns.len())?;
+        Some(crate::selection::said(locale, size, whole_rows))
+    });
     // The grid asks for the cells in view only.
     let output = grid::show(
         &mut child,
@@ -1944,6 +1959,10 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
         &grid_columns,
         rows.len(),
         sql.selection,
+        grid::Picked {
+            block,
+            said: said.as_deref(),
+        },
         keys,
         palette,
         &grid_look,
@@ -1953,7 +1972,7 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
             mark: crate::edit::RowMark::None,
             number: Some(row as u64 + 1),
             deleted: false,
-            ranged: false,
+            ranged: whole.contains(&row),
         },
         None,
         None,
@@ -1974,10 +1993,27 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
         },
     );
     if let Some(cell) = output.clicked {
-        actions.push(Action::SelectCell {
+        let id = sql.id;
+        // With Shift held the click takes the cells up to its own into
+        // the selection.
+        actions.push(if output.shift {
+            Action::ExtendTo { tab, id, cell }
+        } else {
+            Action::SelectCell { tab, id, cell }
+        });
+    }
+    // A drag over the cells: the range runs to the one under it.
+    if let Some(cell) = output.dragged {
+        let id = sql.id;
+        actions.push(Action::ExtendTo { tab, id, cell });
+    }
+    if let Some((row, click)) = output.row_clicked {
+        let id = sql.id;
+        actions.push(Action::RowNumberClicked {
             tab,
-            id: sql.id,
-            cell,
+            id,
+            row,
+            click,
         });
     }
     // The Tab key came to the grid: the arrows are its own now.
```

`src/ui/settings/preview.rs`:

```diff
diff --git a/src/ui/settings/preview.rs b/src/ui/settings/preview.rs
index 07a0769c..1151cbd1 100644
--- a/src/ui/settings/preview.rs
+++ b/src/ui/settings/preview.rs
@@ -230,6 +230,7 @@ fn paint_grid(ui: &mut Ui, rect: Rect, settings: &Settings, look: &Look, palette
         &columns,
         rows.len(),
         None,
+        grid::Picked::default(),
         false,
         palette,
         look,
```

`docs/_guide/browsing-data.md`:

```diff
diff --git a/docs/_guide/browsing-data.md b/docs/_guide/browsing-data.md
index aed1b8df..27c330eb 100644
--- a/docs/_guide/browsing-data.md
+++ b/docs/_guide/browsing-data.md
@@ -166,11 +166,33 @@ read-only or not as Settings › Editor says for a new tab, and always
 read-only on production or on a connection that cannot write. On
 production, the table's name has the production badge beside it.
 
+## Selecting
+
+Each row has its number at the left of the grid. On macOS and Windows:
+
+- Click a cell to select it. Drag over cells, Shift-click another cell,
+  or hold Shift and press the arrows to select the cells between them.
+  The footer says how many rows and columns they are.
+- Click a row's number to select the row. Shift-click another number to
+  select the rows between them, and Cmd/Ctrl-click one to add it or take
+  it out. Shift+Space selects the rows of the selected cells, and
+  Cmd/Ctrl+A every row loaded.
+
+In the Omarchy look, `v` starts a selection of cells on the cursor's
+cell, `Ctrl+V` a block, and `V` whole rows. The moves stretch it, the
+status line says its mode (`-- VISUAL --`, `-- VISUAL BLOCK --`,
+`-- VISUAL LINE --`) and its size, and Esc leaves it.
+
+A selection holds the rows loaded on the page. Filtering, sorting,
+reloading and going to another page end it. A SQL result's rows are
+selected the same way.
+
 ## Copying
 
-On macOS and Windows, Cmd/Ctrl+C copies the selected cell and
-Cmd/Ctrl+Shift+C the whole row, with tabs between its values. In the
-Omarchy look `v` then `y` copies the cell.
+On macOS and Windows, Cmd/Ctrl+C copies the selection with tabs between
+the values and a line for each row (the selected cell alone, without a
+selection), and Cmd/Ctrl+Shift+C its rows whole. In the Omarchy look `y`
+copies the selection, so `v` then `y` copies the cell.
 
 ## Next
 
```

`docs/_guide/editing-data.md`:

```diff
diff --git a/docs/_guide/editing-data.md b/docs/_guide/editing-data.md
index 7903c1d7..1dfe7ab5 100644
--- a/docs/_guide/editing-data.md
+++ b/docs/_guide/editing-data.md
@@ -399,6 +399,9 @@ The yanked row stays for the next `p`, through a reload too. `yy` also
 puts the row on the clipboard, unless it is a new row that is not saved
 yet.
 
+With several rows selected, Cmd/Ctrl+D copies each of them: the copies
+stand at the top in the grid's order, and the first is the one to edit.
+
 A new row is copied the same way, by the keys: its form has no
 **Duplicate** button.
 
@@ -443,9 +446,10 @@ it is.
 A new row that is not saved yet is dropped by the same key, and leaves
 nothing to save.
 
-To delete several rows at once, select them first: hold Shift and click
-another row, or hold Shift and press the Up or Down arrow. The footer says
-how many rows are selected. Backspace then marks them all. Pressed again
+To delete several rows at once, select them first: click a row's number
+and Shift-click or Cmd/Ctrl-click others, or select cells across the rows
+(hold Shift and press the Up or Down arrow). The footer says how many are
+selected. Backspace then marks every row the selection covers. Pressed again
 it restores them all, and where only some of them were marked it marks the
 rest. New rows among them are dropped. One press takes up to 1,000 rows:
 for more, delete with a statement in the SQL editor.
```

`docs/_guide/macos.md`:

```diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
index 485b7f84..7dddee52 100644
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -187,8 +187,12 @@ keyboard.
 | Cmd+Left, Cmd+Right | First and last column |
 | Cmd+Option+Left and Right | Previous and next page |
 | Cmd+F | Filter bar |
-| Cmd+C | Copy the cell |
-| Cmd+Shift+C | Copy the row |
+| Cmd+C | Copy the selection, or the cell |
+| Cmd+Shift+C | Copy the selected rows |
+| Shift+arrows, Shift-click, drag | Select cells |
+| Click a row's number | Select the row. Shift-click selects through to another, Cmd-click adds or takes out one |
+| Shift+Space | Select the rows of the selected cells |
+| Cmd+A | Select every row loaded |
 | Cmd+R | Reload |
 | Cmd+. | Cancel the running query |
 | Cmd+I | Open the row panel, with the keyboard on its fields. Esc goes back to the rows |
@@ -220,9 +224,9 @@ In its DDL view:
 | Return, F2 | Edit the cell. Typing on a selected cell starts an edit too, and so does a double-click |
 | Cmd+N | Add a row, at the top of the grid |
 | Down | On the table's last row, on its last page: add a row below it |
-| Cmd+D | Duplicate the row, as a new row at the top of the grid |
+| Cmd+D | Duplicate the selected rows, as new rows at the top of the grid |
 | Backspace, Forward Delete | Delete the row: mark it for deletion, or restore a marked row. A new row that is not saved yet is dropped |
-| Shift+Up, Shift+Down | Select the row above or below with the selected rows. Backspace then deletes them all |
+| Shift+Up, Shift+Down | Select cells across the rows above or below. Backspace then deletes their rows |
 | Return, Tab, Shift+Tab | Keep the edit and move down, right or left |
 | Esc | Cancel the edit |
 | Cmd+L | In a foreign key cell's list: choose the column that labels it. In the chooser, Up and Down move, Return chooses and Esc closes |
```

`docs/_guide/omarchy.md`:

```diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index 9fc8779d..af970776 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -159,7 +159,7 @@ With the search open, the matches replace the start lists:
 | `r`, `s`, `S` | Show the rows, the structure or the DDL, from any of them |
 | `:ddl` | The DDL view of the table under the sidebar's cursor (with the keyboard in the sidebar), or else of the table in front |
 
-Copying a whole row has no key.
+`V` then `y` copies whole rows, with tabs between the values.
 
 ### The Agents tab
 
@@ -248,9 +248,11 @@ cursor is on.
 | `p`, `P` | Put a copy of the yanked row below the cursor's row, or above it |
 | `dd` | Delete the row: mark it for deletion, or restore a marked row. A new row that is not saved yet is dropped |
 | `3dd` | The same for the cursor's row and the two after it. Any count works, up to 1,000 rows. The cursor moves to the row after them |
-| `V` | Start a selection of rows on the cursor's row. `j`, `k` and the other moves extend it. The status line reads `-- VISUAL --` and counts its rows |
+| `v`, Ctrl+V | Start a selection of cells, or a block, on the cursor's cell. The moves stretch it. The status line reads `-- VISUAL --` or `-- VISUAL BLOCK --` and its size |
+| `V` | Start a selection of rows on the cursor's row. `j`, `k` and the other moves extend it. The status line reads `-- VISUAL LINE --` and counts its rows |
+| `y` | In a selection: copy it, with tabs between the values, and leave it |
 | `d` | In a selection of rows: delete its rows, or restore them where all are marked. New rows among them are dropped |
-| Esc, `V` | Leave the selection of rows |
+| Esc, or the key that started it | Leave the selection |
 | Enter, Tab, Shift+Tab | Keep the edit and move down, right or left |
 | Esc | Leave the editor and keep the edit |
 | Ctrl+C | Drop the edit |
@@ -325,6 +327,7 @@ On a result, with the keyboard out of the editor:
 | Keys | Do |
 | --- | --- |
 | `j` `k`, `h` `l` | Move in the result |
+| `v`, Ctrl+V, `V` | Select cells, a block, or whole rows, as in a table. `y` copies the selection and Esc leaves it |
 | Enter | Show the selected row in the row panel |
 | Ctrl+Shift+R | Show or hide the row panel |
 | `[` `]` | Previous and next row, while the panel shows |
```

`docs/_reference/keyboard-shortcuts.md`:

```diff
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
index 3b6236c0..fe08c6bf 100644
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -120,8 +120,8 @@ takes what you type (Mod+P opens the same search from anywhere):
 | Mod+. | Cancel the running query |
 | Mod+I | Open the row panel, with the keyboard on its fields. Esc goes back to the rows |
 | Mod+Shift+R | Show or hide the row panel |
-| Mod+C | Copy the cell |
-| Mod+Shift+C | Copy the row |
+| Mod+C | Copy the selection, or the cell |
+| Mod+Shift+C | Copy the selected rows |
 | Ctrl+Cmd+1, 2, 3 on macOS, Alt+1, 2, 3 on Windows | Show the rows, the structure, the DDL |
 
 In a table's [Structure view]({% link _guide/browsing-data.md %}#structure):
@@ -152,9 +152,13 @@ These are the keys of the macOS and Windows looks. The Omarchy look has
 | Enter, F2 | Edit the cell. Typing on a selected cell starts an edit too, and so does a double-click |
 | Mod+N | [Add a row]({% link _guide/editing-data.md %}#add-a-row), at the top of the grid, while a table's rows are in front |
 | Down | On the table's last row, on its last page: add a row below it |
-| Mod+D | [Duplicate the row]({% link _guide/editing-data.md %}#duplicate-a-row), as a new row at the top of the grid |
+| Mod+D | [Duplicate the selected rows]({% link _guide/editing-data.md %}#duplicate-a-row), as new rows at the top of the grid |
 | Backspace, Delete | [Delete the row]({% link _guide/editing-data.md %}#delete-a-row): mark it for deletion, or restore a marked row. A new row that is not saved yet is dropped |
-| Shift+Up, Shift+Down | Select the row above or below with the selected rows. Shift with a click selects through to the clicked row. Backspace then [deletes them all]({% link _guide/editing-data.md %}#delete-a-row) |
+| Shift+arrows | [Select cells]({% link _guide/browsing-data.md %}#selecting) from the selected one. Shift with a click, or a drag, selects through to the clicked cell. Backspace then [deletes their rows]({% link _guide/editing-data.md %}#delete-a-row) |
+| Click a row's number | Select the row. Shift-click selects through to another, Mod-click adds or takes out one |
+| Shift+Space | Select the rows of the selected cells |
+| Mod+A | Select every row loaded |
+| Mod+C, Mod+Shift+C | Copy the selection as tab-separated values, or its rows whole. One cell is copied as it is |
 | Enter, Tab, Shift+Tab | Keep the edit and move down, right or left |
 | Alt+Enter | Open the large editor. In it, Mod+Enter applies and Esc cancels |
 | Esc | Cancel the edit |
```

- [ ] **Step 4: Run the checks**

As in Task 1. Expected: all pass; the library's tests: 3720 passed.

- [ ] **Step 5: Commit**

```bash
git add src/ui/grid.rs src/theme.rs src/ui/data_view.rs src/ui/sql_results.rs src/ui/settings/preview.rs src/testing.rs src/ui/selection_tests.rs src/ui/mod.rs docs/_guide/browsing-data.md docs/_guide/editing-data.md docs/_guide/macos.md docs/_guide/omarchy.md docs/_reference/keyboard-shortcuts.md
git commit -m "Number the grid's rows, select by number and by drag, and draw a range of cells"
```

---

## After Task 2

Push the branch and open the pull request. Its description: a summary; the selection model (the table in the spec's "The model", in words); the keys added and the keymap's check (its generated tests: no two commands share a key where both could be read); the decisions above and the spec's departures; what runs 2 to 4 hold. No screenshot.

## Checks by hand

Off the record, on the demo database (Bookshop) in each look: drag over cells, Shift-click, Shift with the arrows; click a row's number, Shift-click and Command-click others; Shift+Space and Mod+A; in the Omarchy look `v`, `ctrl+v` and `V` with the moves, `y`, `d`, Esc; the same in a SQL result. Paste what Mod+C copied into a spreadsheet. Compare with the boards by eye (`macOS – Copy as`, `Omarchy – Copy as`, the Components additions' cell range and visual block). Nothing of it is a test.
