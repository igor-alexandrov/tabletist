//! The grid's selection, checked against its spec
//! (`docs/superpowers/specs/2026-10-30-export-copy-as-1-selection-design.md`:
//! "test N" below is an entry of its Tests).
//!
//! Behaviour, names and words, on the Bookshop's `book_covers` and on a SQL
//! result; the colours only as the palette names them. No picture of the
//! design is compared.

use egui::accesskit::{Role, TreeUpdate};
use egui::{Key, Modifiers};
use tabletist_db::Value;

use super::insert_audit_tests::{covers_in, edits, painted, select, set, type_key};
use crate::model::{Action, CellPos, ConnTabId, RowClick, TabId};
use crate::testing::Harness;
use crate::theme::Look;

/// The desktop looks: the keys and the clicks are the same in both.
fn desktop() -> [Look; 2] {
    [Look::standard(), Look::macos()]
}

/// The selected rows of the table's tab, in the grid's order.
fn rows(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<usize> {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.object_tab(id).unwrap().selected_rows()
}

/// The first and the last column the table's selection covers.
fn cols(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<(usize, usize)> {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.object_tab(id).unwrap().selected_cols()
}

/// Whether the table's range is of whole rows.
fn whole_rows(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace.object_tab(id).unwrap();
    object
        .range
        .as_ref()
        .is_some_and(crate::selection::Range::rows)
}

/// The keyboard's copy, with Shift held for the whole rows.
fn copied(harness: &mut Harness, shift: bool) -> Option<String> {
    harness.copied = None;
    harness.copy(shift);
    harness.copied.clone()
}

/// The terminal look's `ctrl+v`, as the window sends it: a paste with
/// Control held.
fn ctrl_v(harness: &mut Harness) {
    harness.settle();
    harness.frame(vec![
        egui::Event::ModifiersChanged(Modifiers::CTRL),
        egui::Event::Paste(String::new()),
    ]);
    harness.frame(vec![egui::Event::ModifiersChanged(Modifiers::NONE)]);
    harness.settle();
}

/// What a click on the number of `row` does, with the keys `click` says
/// were held.
fn pick_row(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize, click: RowClick) {
    let action = crate::model::Action::RowNumberClicked {
        tab,
        id,
        row,
        click,
    };
    harness.app.apply(action);
    harness.settle();
}

/// Test 2: Shift with the arrows selects a rectangle of cells, which the
/// footer sizes and a copy takes as TSV.
#[test]
fn shift_and_the_arrows_select_a_rectangle_of_cells() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (0, 1));
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        harness.press(Key::ArrowRight, Modifiers::SHIFT);
        assert_eq!(rows(&harness, tab, id), [0, 1, 2], "{}", look.name);
        assert_eq!(cols(&harness, tab, id), Some((1, 2)), "{}", look.name);
        assert!(!whole_rows(&harness, tab, id));
        assert!(
            painted(&harness, "3 rows × 2 columns selected"),
            "{}",
            look.name
        );
        let tsv =
            "9100000000000000001\tprint\n9100000000000000002\tebook\n9100000000000000003\tprint";
        assert_eq!(
            copied(&mut harness, false).as_deref(),
            Some(tsv),
            "{}",
            look.name
        );
        // Shift with Left takes the column back; a plain arrow ends it.
        harness.press(Key::ArrowLeft, Modifiers::SHIFT);
        assert_eq!(cols(&harness, tab, id), Some((1, 1)));
        harness.press(Key::ArrowUp, Modifiers::NONE);
        assert_eq!(rows(&harness, tab, id), [1]);
        // One cell copies its text, as it always did.
        assert_eq!(
            copied(&mut harness, false).as_deref(),
            Some("9100000000000000002")
        );
    }
}

/// Test 2: Shift with Space makes the selection whole rows, and Mod+A
/// selects every row loaded.
#[test]
fn shift_space_and_select_all_select_whole_rows() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (1, 1));
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        harness.press(Key::Space, Modifiers::SHIFT);
        assert_eq!(rows(&harness, tab, id), [1, 2], "{}", look.name);
        assert!(whole_rows(&harness, tab, id), "{}", look.name);
        harness.press(Key::A, Modifiers::COMMAND);
        assert_eq!(rows(&harness, tab, id), [0, 1, 2], "{}", look.name);
        assert!(painted(&harness, "3 rows selected"), "{}", look.name);
        // A plain move ends it.
        harness.press(Key::ArrowUp, Modifiers::NONE);
        assert_eq!(rows(&harness, tab, id), [1], "{}", look.name);
    }
}

/// Test 5: delete and duplicate act on every selected row: a rectangle's
/// rows, or the picked ones.
#[test]
fn delete_and_duplicate_take_every_selected_row() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // A rectangle's rows are deleted whole.
    select(&mut harness, tab, id, (0, 2));
    harness.press(Key::ArrowDown, Modifiers::SHIFT);
    harness.press(Key::ArrowRight, Modifiers::SHIFT);
    harness.press(Key::Backspace, Modifiers::NONE);
    let marked: Vec<usize> = edits(&harness, tab, id).deleted.keys().copied().collect();
    assert_eq!(marked, [0, 1]);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(edits(&harness, tab, id).deleted.is_empty());
    // Two rows picked apart are copied in their order, at the top.
    pick_row(&mut harness, tab, id, 0, RowClick::Only);
    pick_row(&mut harness, tab, id, 1, RowClick::Toggle);
    harness.press(Key::D, Modifiers::COMMAND);
    harness.settle();
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace.object_tab(id).unwrap();
    assert_eq!(object.edits.added.len(), 2);
    let order = object.order();
    let kind = |place: usize| {
        let row = order.row(place).unwrap();
        let pending = object.edits.cells.get(&(row, 2)).map(|cell| &cell.new);
        pending.map(crate::edit::shown_value)
    };
    assert_eq!(kind(0), Some(Value::Text("print".into())));
    assert_eq!(kind(1), Some(Value::Text("ebook".into())));
    // One editor, on the first copy.
    let first = order.row(0).unwrap();
    let editor = object.edits.editor.as_ref().map(|editor| editor.cell.row);
    assert!(editor.is_none_or(|row| row == first), "{editor:?}");
}

/// Test 3 and 4: the terminal look's `v`, `ctrl+v` and `V` start their
/// modes, the status line says each with its size and keys, `y` copies
/// and leaves, and Esc leaves.
#[test]
fn the_visual_modes_are_said_and_copy() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    select(&mut harness, tab, id, (0, 1));
    type_key(&mut harness, Key::V, "v");
    assert!(painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
    assert!(painted(&harness, "1 row × 1 column"));
    assert!(painted(&harness, "y copy · esc normal"));
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::L, "l");
    type_key(&mut harness, Key::L, "l");
    type_key(&mut harness, Key::H, "h");
    assert!(painted(&harness, "2 rows × 2 columns"));
    // A block of cells has no `d`, and `dd` acts on one row: held back,
    // nothing is deleted.
    type_key(&mut harness, Key::D, "d");
    type_key(&mut harness, Key::D, "d");
    assert!(edits(&harness, tab, id).deleted.is_empty());
    assert!(painted(&harness, "-- VISUAL --"));
    harness.copied = None;
    type_key(&mut harness, Key::Y, "y");
    let tsv = "9100000000000000001\tprint\n9100000000000000002\tebook";
    assert_eq!(harness.copied.as_deref(), Some(tsv));
    assert!(!painted(&harness, "-- VISUAL --"));
    // The block, from the chord the window sends as a paste.
    ctrl_v(&mut harness);
    assert!(
        painted(&harness, "-- VISUAL BLOCK --"),
        "{:?}",
        harness.painted
    );
    type_key(&mut harness, Key::K, "k");
    assert!(painted(&harness, "2 rows × 1 column"));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!painted(&harness, "-- VISUAL BLOCK --"));
    // Rows: their own name, and `d` deletes them.
    type_key(&mut harness, Key::V, "V");
    assert!(painted(&harness, "-- VISUAL LINE --"));
    assert!(painted(&harness, "1 row"));
    assert!(painted(&harness, "d delete · esc normal"));
    type_key(&mut harness, Key::J, "j");
    assert!(painted(&harness, "2 rows"));
    type_key(&mut harness, Key::D, "d");
    let marked: Vec<usize> = edits(&harness, tab, id).deleted.keys().copied().collect();
    assert_eq!(marked, [0, 1]);
}

/// Test 3: `v y` copies the cursor's cell, as it did before `v` was a mode.
#[test]
fn v_then_y_still_copies_one_cell() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    harness.copied = None;
    type_key(&mut harness, Key::V, "v");
    type_key(&mut harness, Key::Y, "y");
    assert_eq!(harness.copied.as_deref(), Some("ebook"));
    let workspace = harness.app.workspace(tab).unwrap();
    assert!(!workspace.object_tab(id).unwrap().visual());
    // `V` then `y` copies the rows whole.
    harness.copied = None;
    type_key(&mut harness, Key::V, "V");
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::Y, "y");
    let copied = harness.copied.clone().expect("a copy");
    let lines: Vec<&str> = copied.lines().collect();
    assert_eq!(lines.len(), 2, "{copied}");
    assert!(
        lines.iter().all(|line| line.split('\t').count() == 5),
        "{copied}"
    );
}

/// What selects is dropped under the question about pending changes, as
/// the moves are: each closes an editor.
#[test]
fn the_selection_stays_under_a_prompt_about_the_changes() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (0, 2), "audio");
    select(&mut harness, tab, id, (1, 1));
    harness
        .app
        .apply(crate::model::Action::CloseTab { tab, id });
    let click = RowClick::Only;
    let row = 2;
    for action in [
        crate::model::Action::RowNumberClicked {
            tab,
            id,
            row,
            click,
        },
        crate::model::Action::SelectAllRows { tab, id },
        crate::model::Action::SelectRowsOfCursor { tab, id },
    ] {
        harness.app.apply(action);
    }
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace
        .object_tab(id)
        .expect("the tab, under its question");
    assert_eq!(object.selection, Some(CellPos { row: 1, col: 1 }));
    assert_eq!(object.range, None);
}

/// Test 4: whatever loads another page of rows leaves no range.
#[test]
fn a_filter_a_sort_and_a_page_leave_no_range() {
    let range = |harness: &Harness, tab: ConnTabId, id: TabId| {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.object_tab(id).unwrap().range.clone()
    };
    let changes: [fn(&mut Harness, ConnTabId, TabId); 4] = [
        |harness, tab, object_tab| {
            let column = "kind".to_owned();
            harness.app.apply(crate::model::Action::SortBy {
                tab,
                object_tab,
                column,
            });
        },
        |harness, tab, _| {
            harness.app.apply(crate::model::Action::Refresh(tab));
            harness.answer_rows(crate::testing::book_covers_page(3));
        },
        |harness, tab, object_tab| {
            // A page with more after it.
            let workspace = harness.app.workspace_mut(tab).unwrap();
            let object = workspace.object_tab_mut(object_tab).unwrap();
            object.rows.value.as_mut().unwrap().has_more = true;
            harness.app.apply(Action::NextPage { tab, object_tab });
        },
        |harness, tab, id| {
            let workspace = harness.app.workspace_mut(tab).unwrap();
            let object = workspace.object_tab_mut(id).unwrap();
            object.filter.raw = true;
            object.filter.raw_text = "kind = 'print'".into();
            harness.app.apply(crate::model::Action::ApplyFilters {
                tab,
                object_tab: id,
            });
        },
    ];
    for (index, change) in changes.into_iter().enumerate() {
        let (mut harness, tab, id) = covers_in(Look::macos());
        pick_row(&mut harness, tab, id, 0, RowClick::Only);
        pick_row(&mut harness, tab, id, 2, RowClick::Extend);
        assert!(range(&harness, tab, id).is_some());
        change(&mut harness, tab, id);
        harness.settle();
        assert_eq!(range(&harness, tab, id), None, "{index}");
    }
}

/// Test 5: a Command-click puts the cursor on the row clicked, and one
/// that takes the row out gives the cursor to the nearest row left: Delete
/// row takes that row, not the one taken out.
#[test]
fn delete_takes_the_picked_row_after_the_cursors_was_taken_out() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    pick_row(&mut harness, tab, id, 0, RowClick::Only);
    pick_row(&mut harness, tab, id, 2, RowClick::Toggle);
    pick_row(&mut harness, tab, id, 0, RowClick::Toggle);
    assert_eq!(rows(&harness, tab, id), [2]);
    let workspace = harness.app.workspace(tab).unwrap();
    let cursor = workspace.object_tab(id).unwrap().selection;
    assert_eq!(cursor.map(|cell| cell.row), Some(2));
    harness.press(Key::Backspace, Modifiers::NONE);
    let marked: Vec<usize> = edits(&harness, tab, id).deleted.keys().copied().collect();
    assert_eq!(marked, [2]);
}

/// Test 3: `v y` copies one cell's text as it is, tabs and lines in it
/// kept, as Mod+C does on one cell.
#[test]
fn one_cell_is_copied_as_it_is() {
    let text = "line one\nline\ttwo";
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, id) = covers_in(look);
        harness.app.workspace_mut(tab).unwrap().row_panel = false;
        set(&mut harness, tab, id, (1, 2), text);
        select(&mut harness, tab, id, (1, 2));
        harness.copied = None;
        if look.terminal {
            type_key(&mut harness, Key::V, "v");
            type_key(&mut harness, Key::Y, "y");
        } else {
            harness.copy(false);
        }
        assert_eq!(harness.copied.as_deref(), Some(text), "{}", look.name);
    }
}

/// Test 3: a SQL result's visual selection is said on the editor's line,
/// with its keys in place of the editor's, and not under the Messages
/// pane.
#[test]
fn a_results_visual_selection_is_said_with_its_keys() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let tab = harness.connect_fake();
    let id = super::tests::with_sql_result(&mut harness, tab, 3);
    // The pointer on the first row, which gives the result's rows the
    // keys.
    let tree = harness.settle();
    let row = crate::testing::bounds(&tree, "Row 1", egui::accesskit::Role::Button).unwrap();
    let press = |pressed| egui::Event::PointerButton {
        pos: row.center(),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(row.center())]);
    harness.frame(vec![press(true)]);
    harness.frame(vec![press(false)]);
    type_key(&mut harness, Key::V, "v");
    assert!(painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
    assert!(painted(&harness, "1 row × 1 column"));
    assert!(
        painted(&harness, "y copy · esc normal"),
        "{:?}",
        harness.painted
    );
    let pane = crate::model::ResultPane::Messages;
    harness.app.apply(Action::SetResultPane {
        tab,
        sql_tab: id,
        pane,
    });
    harness.settle();
    assert!(!painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
}

// The grid: its rows' numbers, the pointer, and what it draws and says.

/// Where the row labelled `label` is.
fn row_rect(tree: &TreeUpdate, label: &str) -> egui::Rect {
    crate::testing::bounds(tree, label, Role::Button)
        .unwrap_or_else(|| panic!("no row labelled {label:?}"))
}

/// A click of the pointer at `pos`, with `modifiers` held, as a window
/// tells it: the modifiers first, then the pointer.
fn click_with(harness: &mut Harness, pos: egui::Pos2, modifiers: Modifiers) {
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    };
    harness.frame(vec![
        egui::Event::ModifiersChanged(modifiers),
        egui::Event::PointerMoved(pos),
    ]);
    harness.frame(vec![button(true)]);
    harness.frame(vec![button(false)]);
    harness.frame(vec![egui::Event::ModifiersChanged(Modifiers::NONE)]);
    harness.settle();
}

/// A click on the number of the row labelled `label`.
fn click_number(harness: &mut Harness, label: &str, modifiers: Modifiers) {
    let tree = harness.settle();
    let row = row_rect(&tree, label);
    click_with(
        harness,
        row.left_center() + egui::vec2(20.0, 0.0),
        modifiers,
    );
}

/// Where the cell of the row labelled `label` that writes `text` is.
fn cell_at(harness: &mut Harness, label: &str, text: &str) -> egui::Pos2 {
    let tree = harness.settle();
    let row = row_rect(&tree, label);
    harness
        .text_rects
        .iter()
        .find(|(piece, rect)| {
            piece == text
                && row.y_range().contains(rect.center().y)
                && !harness.is_row_number(*rect)
        })
        .map(|(_, rect)| rect.center())
        .unwrap_or_else(|| panic!("no {text:?} in {label:?}"))
}

/// A drag of the pointer from `from` to `to`, over a few frames.
fn drag(harness: &mut Harness, from: egui::Pos2, to: egui::Pos2) {
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(from)]);
    harness.frame(vec![button(from, true)]);
    for step in 1..=4 {
        let pos = from + (to - from) * (step as f32 / 4.0);
        harness.frame(vec![egui::Event::PointerMoved(pos)]);
    }
    harness.frame(vec![button(to, false)]);
    harness.settle();
}

/// What a screen reader is told of the grid's range.
fn told(harness: &mut Harness) -> Option<String> {
    let tree = harness.settle();
    let mut nodes = tree.nodes.iter();
    let grid = nodes.find(|(_, node)| node.label() == Some("Rows") && node.role() == Role::Group);
    grid.and_then(|(_, node)| node.description().map(str::to_owned))
}

/// Test 2: a screen reader is told the range's size on the grid, and
/// nothing with no range.
#[test]
fn the_range_is_told_to_a_screen_reader() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (0, 1));
        assert_eq!(told(&mut harness), None, "{}", look.name);
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        harness.press(Key::ArrowRight, Modifiers::SHIFT);
        assert_eq!(
            told(&mut harness).as_deref(),
            Some("3 rows by 2 columns selected")
        );
        harness.press(Key::Space, Modifiers::SHIFT);
        assert_eq!(told(&mut harness).as_deref(), Some("3 rows selected"));
        harness.press(Key::ArrowUp, Modifiers::NONE);
        assert_eq!(told(&mut harness), None);
    }
}

/// Test 2: a click with Shift runs a rectangle from the cursor's cell, and
/// a drag from the cell it began on.
#[test]
fn shift_with_a_click_and_a_drag_select_cells() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (0, 2));
        let at = cell_at(&mut harness, "Row 3", "2026-01-12 09:14:03");
        click_with(&mut harness, at, Modifiers::SHIFT);
        assert_eq!(rows(&harness, tab, id), [0, 1, 2], "{}", look.name);
        assert_eq!(cols(&harness, tab, id), Some((2, 4)), "{}", look.name);
        // A drag from the second row's kind to the first row's publisher.
        let from = cell_at(&mut harness, "Row 2", "ebook");
        let to = cell_at(&mut harness, "Row 1", "9100000000000000001");
        drag(&mut harness, from, to);
        assert_eq!(rows(&harness, tab, id), [0, 1], "{}", look.name);
        assert_eq!(cols(&harness, tab, id), Some((1, 2)), "{}", look.name);
        let workspace = harness.app.workspace(tab).unwrap();
        let cursor = workspace.object_tab(id).unwrap().selection;
        assert_eq!(cursor, Some(CellPos { row: 0, col: 1 }), "{}", look.name);
    }
}

/// Test 2: a row's number selects the row, Shift runs to another, and
/// Command takes one out or puts one in. Each row is told as selected,
/// and a copy takes the rows whole.
#[test]
fn a_row_number_selects_extends_and_toggles_rows() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_in(look);
        click_number(&mut harness, "Row 1", Modifiers::NONE);
        assert_eq!(rows(&harness, tab, id), [0], "{}", look.name);
        assert!(whole_rows(&harness, tab, id), "{}", look.name);
        assert_eq!(told(&mut harness).as_deref(), Some("1 row selected"));
        click_number(&mut harness, "Row 3", Modifiers::SHIFT);
        assert_eq!(rows(&harness, tab, id), [0, 1, 2], "{}", look.name);
        assert!(painted(&harness, "3 rows selected"), "{}", look.name);
        click_number(&mut harness, "Row 2", Modifiers::COMMAND);
        assert_eq!(rows(&harness, tab, id), [0, 2], "{}", look.name);
        let tree = harness.settle();
        let selected = |label: &str| {
            let mut nodes = tree.nodes.iter();
            let node = nodes.find(|(_, node)| node.label() == Some(label));
            node.and_then(|(_, node)| node.is_selected())
        };
        assert_eq!(selected("Row 1"), Some(true));
        assert_eq!(selected("Row 2"), None);
        assert_eq!(selected("Row 3"), Some(true));
        // Whole rows, every column, with Shift's copy and the plain one.
        let rows_copied = copied(&mut harness, true).unwrap();
        assert_eq!(rows_copied.lines().count(), 2, "{rows_copied}");
        assert!(rows_copied.starts_with("1\t9100000000000000001\tprint\t"));
        assert!(rows_copied.contains("\n3\t9100000000000000003\tprint\t"));
        assert_eq!(copied(&mut harness, false), Some(rows_copied));
    }
}

/// Test 2 and 3: a SQL result's rows take the same selection, copy it, and
/// say its mode in the terminal look.
#[test]
fn a_results_rows_take_the_same_selection() {
    for look in [Look::macos(), Look::omarchy()] {
        let mut harness = Harness::new();
        harness.set_look(look);
        let tab = harness.connect_fake();
        let id = super::tests::with_sql_result(&mut harness, tab, 3);
        // The pointer on the first row's first cell, past its number.
        let tree = harness.settle();
        let row = row_rect(&tree, "Row 1");
        let past = crate::ui::grid::gutter(&look) + 10.0;
        click_with(
            &mut harness,
            row.left_center() + egui::vec2(past, 0.0),
            Modifiers::NONE,
        );
        let sql = |harness: &Harness| {
            let workspace = harness.app.workspace(tab).unwrap();
            let sql = workspace.sql_tab(id).unwrap();
            let selected = sql.selected().unwrap();
            (selected.rows(), selected.cols(3))
        };
        if look.terminal {
            type_key(&mut harness, Key::V, "v");
            type_key(&mut harness, Key::J, "j");
            type_key(&mut harness, Key::L, "l");
            assert!(painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
            assert!(painted(&harness, "2 rows × 2 columns"));
            assert_eq!(sql(&harness), (vec![0, 1], Some((0, 1))));
            harness.copied = None;
            type_key(&mut harness, Key::Y, "y");
        } else {
            harness.press(Key::ArrowDown, Modifiers::SHIFT);
            harness.press(Key::ArrowRight, Modifiers::SHIFT);
            assert_eq!(sql(&harness), (vec![0, 1], Some((0, 1))));
            assert_eq!(
                told(&mut harness).as_deref(),
                Some("2 rows by 2 columns selected")
            );
            harness.copied = None;
            harness.copy(false);
            // Its rows: a number, Shift+Space and Mod+A, and their copy.
            let before = harness.copied.clone();
            click_number(&mut harness, "Row 2", Modifiers::NONE);
            assert_eq!(sql(&harness).0, [1]);
            harness.press(Key::Space, Modifiers::SHIFT);
            harness.press(Key::A, Modifiers::COMMAND);
            assert_eq!(sql(&harness), (vec![0, 1, 2], Some((0, 2))));
            let rows = copied(&mut harness, true).expect("the rows");
            assert_eq!(rows.lines().count(), 3, "{rows}");
            harness.copied = before;
        }
        let copied = harness.copied.clone().expect("a copy");
        let lines: Vec<&str> = copied.lines().collect();
        assert_eq!(lines.len(), 2, "{}: {copied}", look.name);
        assert!(lines.iter().all(|line| line.split('\t').count() == 2));
        assert!(lines[0].starts_with("1\t"), "{copied}");
    }
}

/// Test 6: a range of cells is filled as the palette names it, in each
/// look, and the rows' numbers are written in theirs.
#[test]
fn a_range_and_the_row_numbers_are_painted_in_their_colours() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, id) = covers_in(look);
        let palette = harness.app.palette;
        let (fill, number) = if look.terminal {
            (palette.surface_active, palette.dim)
        } else {
            (palette.accent_faint, palette.faint)
        };
        harness.settle();
        let numbered =
            harness
                .text_rects
                .iter()
                .zip(&harness.painted)
                .any(|((text, rect), (_, color))| {
                    text == "3" && *color == number && harness.is_row_number(*rect)
                });
        assert!(numbered, "{}: the third row's number", look.name);
        let filled = |harness: &Harness| {
            let cell = look.row();
            let cells = harness.fills.iter();
            cells
                .filter(|(rect, color)| *color == fill && rect.height() == cell)
                .count()
        };
        let before = filled(&harness);
        select(&mut harness, tab, id, (0, 1));
        if look.terminal {
            type_key(&mut harness, Key::V, "v");
            type_key(&mut harness, Key::J, "j");
            type_key(&mut harness, Key::L, "l");
        } else {
            harness.press(Key::ArrowDown, Modifiers::SHIFT);
            harness.press(Key::ArrowRight, Modifiers::SHIFT);
        }
        harness.settle();
        // Four cells, the cursor's among them in its own colour.
        assert_eq!(filled(&harness) - before, 3, "{}", look.name);
        // The terminal outlines the range's anchor in the accent.
        if look.terminal {
            let accent = palette.accent;
            let outlined = harness.outlines.iter().any(|(rect, stroke)| {
                stroke.color == accent && stroke.width == 1.0 && rect.height() == look.row()
            });
            assert!(outlined, "the anchor's outline");
        }
    }
}
