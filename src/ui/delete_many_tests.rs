//! Deleting many rows at once, checked against its spec
//! (`docs/superpowers/specs/2026-10-13-deleting-many-rows-design.md`:
//! "spec N" below is one of its sections, "test N" an entry of its Tests).
//!
//! Only behaviour, names and words are checked here, on the Bookshop's
//! `book_covers`. No colour, size or picture of the design is.

use egui::{Key, Modifiers};

use super::duplicate_tests::button;
use super::insert_audit_tests::{add, covers_as, edits, painted, select, selected, set, type_key};
use crate::model::{Action, CellPos, ConnTabId, EditStart, Pane, TabId};
use crate::testing::{Harness, book_covers_page, book_covers_structure};
use crate::theme::Look;

/// The desktop looks: the keys and the buttons are the same in both.
fn desktop() -> [Look; 2] {
    [Look::standard(), Look::macos()]
}

/// The Bookshop's covers holding `page`, writable, the grid holding the
/// keyboard.
fn covers_with(look: Look, page: tabletist_db::RowPage) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "book_covers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(book_covers_structure());
    harness.answer_rows(page);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// The selected rows, in the grid's order.
fn rows(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<usize> {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.object_tab(id).unwrap().selected_rows()
}

/// Selects the rows from `from` to `to`, as Shift with the arrows does.
fn select_rows(harness: &mut Harness, tab: ConnTabId, id: TabId, from: usize, to: usize) {
    select(harness, tab, id, (from, 1));
    let rows = to as isize - from as isize;
    harness.app.apply(Action::ExtendBy { tab, id, rows });
    harness.settle();
}

/// Spec 1 and test 1: Shift with an arrow and with a click selects a run
/// of rows, and a plain click or arrow selects one again.
#[test]
fn shift_with_an_arrow_or_a_click_selects_a_run_of_rows() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_with(look, book_covers_page(5));
        select(&mut harness, tab, id, (1, 2));
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
        assert_eq!(selected(&harness, tab, id), Some((3, 2)));
        // Back over the anchor, to the row above it.
        for _ in 0..3 {
            harness.press(Key::ArrowUp, Modifiers::SHIFT);
        }
        assert_eq!(rows(&harness, tab, id), [0, 1]);
        // A plain arrow selects one row, down or along the row.
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(rows(&harness, tab, id), [1]);
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        harness.press(Key::ArrowRight, Modifiers::NONE);
        assert_eq!(rows(&harness, tab, id), [2]);
        // Shift with a click runs from the cursor's row to the clicked one.
        harness.shift_click("Row 5");
        assert_eq!(rows(&harness, tab, id), [2, 3, 4]);
        // And a plain click selects one.
        select(&mut harness, tab, id, (0, 1));
        assert_eq!(rows(&harness, tab, id), [0]);
    }
}

/// Test 1: with no cursor yet, Shift with a click is a plain click.
#[test]
fn shift_with_a_click_and_no_cursor_selects_one_row() {
    let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(5));
    harness.shift_click("Row 3");
    assert_eq!(rows(&harness, tab, id), [2]);
}

/// Spec 1 and test 10: a range does not outlive its page, and ends where
/// something works on the cursor's row alone.
#[test]
fn a_range_ends_with_its_page_and_with_what_works_on_one_row() {
    let ranged = || {
        let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(5));
        select_rows(&mut harness, tab, id, 1, 3);
        assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
        (harness, tab, id)
    };
    let range = |harness: &Harness, tab: ConnTabId, id: TabId| {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.object_tab(id).unwrap().range
    };
    // A reload.
    let (mut harness, tab, id) = ranged();
    harness.app.apply(Action::Refresh(tab));
    harness.answer_rows(book_covers_page(5));
    assert_eq!(range(&harness, tab, id), None);
    // A sort.
    let (mut harness, tab, id) = ranged();
    harness.app.apply(Action::SortBy {
        tab,
        object_tab: id,
        column: "kind".into(),
    });
    assert_eq!(range(&harness, tab, id), None);
    // A new row.
    let (mut harness, tab, id) = ranged();
    add(&mut harness, tab, id);
    assert_eq!(range(&harness, tab, id), None);
    // An editor that opens.
    let (mut harness, tab, id) = ranged();
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 3, col: 2 },
        start: EditStart::Value,
    });
    assert_eq!(range(&harness, tab, id), None);
    // A save that takes a row out: the first row is marked, then the range
    // is made again.
    let (mut harness, tab, id) = ranged();
    select(&mut harness, tab, id, (0, 1));
    harness.app.apply(Action::DeleteRow { tab, id });
    select_rows(&mut harness, tab, id, 1, 3);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
        inserted: Vec::new(),
        rows: Vec::new(),
        gone: Vec::new(),
        elapsed: std::time::Duration::from_millis(14),
    }));
    assert_eq!(range(&harness, tab, id), None);
}

/// Spec 1 and tests 1 and 11: the footer counts the selected rows, and
/// each row of a range is told to a screen reader as selected.
#[test]
fn a_range_is_counted_in_the_footer_and_told_as_selected() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_with(look, book_covers_page(5));
        select(&mut harness, tab, id, (1, 2));
        assert!(painted(&harness, "1 row selected"));
        select_rows(&mut harness, tab, id, 1, 3);
        assert!(painted(&harness, "3 rows selected"));
        let tree = harness.settle();
        let told = |label: &str| {
            let mut nodes = tree.nodes.iter();
            let node = nodes.find(|(_, node)| node.label() == Some(label));
            node.and_then(|(_, node)| node.is_selected())
        };
        assert_eq!(told("Row 2"), Some(true));
        assert_eq!(told("Row 4"), Some(true));
        assert_eq!(told("Row 1"), None);
        assert_eq!(told("Row 5"), None);
    }
}

/// The rows marked for deletion.
fn marked(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<usize> {
    edits(harness, tab, id).deleted.keys().copied().collect()
}

/// What the toast says, where one shows.
fn said(harness: &Harness) -> Option<String> {
    crate::ui::toast::said(&harness.ctx)
}

/// Whether `words` are said to a screen reader, as a polite live region.
fn polite(harness: &mut Harness, words: &str) -> bool {
    let tree = harness.settle();
    let mut nodes = tree.nodes.iter();
    let node = nodes.find(|(_, node)| node.label().or_else(|| node.value()) == Some(words));
    node.map(|(_, node)| node.live()) == Some(Some(egui::accesskit::Live::Polite))
}

/// A new row right under the page's row `row`, its editor closed. Says
/// its row, as a cell names it.
fn add_below(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize) -> usize {
    let place = crate::edit::Place::Below(row);
    harness.app.apply(Action::AddRow { tab, id, place });
    harness.app.apply(Action::CancelEdit { tab, id });
    harness.settle();
    crate::edit::new_row(edits(harness, tab, id).added.last().unwrap().id)
}

/// Spec 2 and test 2: Backspace on several rows marks them all and keeps
/// the range, and a second one restores them. With one of them unmarked,
/// that one is marked and none is restored.
#[test]
fn backspace_marks_the_selected_rows_and_again_restores_them() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_with(look, book_covers_page(5));
        select_rows(&mut harness, tab, id, 1, 3);
        harness.press(Key::Backspace, Modifiers::NONE);
        assert_eq!(marked(&harness, tab, id), [1, 2, 3]);
        assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
        assert_eq!(selected(&harness, tab, id), Some((3, 1)));
        assert_eq!(said(&harness), None);
        harness.press(Key::Backspace, Modifiers::NONE);
        assert!(marked(&harness, tab, id).is_empty());
        // One row of the range marked alone, then the range.
        select(&mut harness, tab, id, (2, 1));
        harness.press(Key::Backspace, Modifiers::NONE);
        select_rows(&mut harness, tab, id, 1, 3);
        harness.press(Key::Backspace, Modifiers::NONE);
        assert_eq!(marked(&harness, tab, id), [1, 2, 3]);
    }
}

/// Spec 2 and test 6: one action takes 1,000 rows and no more.
#[test]
fn more_than_a_thousand_rows_are_refused_with_the_message() {
    let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(1_001));
    select_rows(&mut harness, tab, id, 0, 1_000);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(marked(&harness, tab, id).is_empty());
    assert_eq!(
        said(&harness).as_deref(),
        Some("Select fewer rows, or delete in the SQL editor.")
    );
    select_rows(&mut harness, tab, id, 0, 999);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(marked(&harness, tab, id).len(), 1_000);
}

/// Spec 2 and test 7: new rows of a range are dropped with a mark and left
/// at a restore, and the range that is left stays selected.
#[test]
fn new_rows_of_a_range_are_dropped_with_the_mark() {
    let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(3));
    // A new row under row 0, and the range from it down to row 2.
    let new = add_below(&mut harness, tab, id, 0);
    select(&mut harness, tab, id, (new, 1));
    harness.app.apply(Action::ExtendBy { tab, id, rows: 2 });
    assert_eq!(rows(&harness, tab, id), [new, 1, 2]);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(edits(&harness, tab, id).added.is_empty());
    assert_eq!(marked(&harness, tab, id), [1, 2]);
    // The anchor was the row that went: the rows left are still selected.
    assert_eq!(rows(&harness, tab, id), [1, 2]);
    assert!(polite(
        &mut harness,
        "2 rows marked for deletion. 2 to delete. 1 new row dropped."
    ));
    // A restore leaves a new row of the range alone: a new row under row
    // 0 again, and with it rows 1 and 2, which are both marked.
    let new = add_below(&mut harness, tab, id, 0);
    select(&mut harness, tab, id, (new, 1));
    harness.app.apply(Action::ExtendBy { tab, id, rows: 2 });
    assert_eq!(rows(&harness, tab, id), [new, 1, 2]);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    assert!(marked(&harness, tab, id).is_empty());
}

/// Test 7: a range of new rows alone drops them and says so.
#[test]
fn a_range_of_new_rows_alone_drops_them() {
    let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(3));
    add(&mut harness, tab, id);
    add(&mut harness, tab, id);
    let first = crate::edit::new_row(edits(&harness, tab, id).added[0].id);
    select(&mut harness, tab, id, (first, 1));
    harness.app.apply(Action::ExtendBy { tab, id, rows: 1 });
    assert_eq!(rows(&harness, tab, id).len(), 2);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(edits(&harness, tab, id).added.is_empty());
    assert!(marked(&harness, tab, id).is_empty());
    assert!(polite(&mut harness, "2 new rows dropped."));
    // The cursor was on the second of them, the grid's second row: it is
    // on the row that stands there now, as after one new row is dropped.
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
}

/// Spec 2 and test 8: a row that cannot be deleted is left out and
/// counted, and a table that takes no delete says why.
#[test]
fn a_row_that_cannot_be_deleted_is_left_out_and_said() {
    let mut page = book_covers_page(5);
    page.rows[2][0] = tabletist_db::Value::Null;
    let (mut harness, tab, id) = covers_with(Look::macos(), page);
    select_rows(&mut harness, tab, id, 1, 3);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(marked(&harness, tab, id), [1, 3]);
    assert_eq!(
        said(&harness).as_deref(),
        Some("2 rows marked for deletion. 1 can't be deleted.")
    );
    assert!(polite(
        &mut harness,
        "2 rows marked for deletion. 1 can't be deleted. 2 to delete."
    ));
    let table = tabletist_db::ObjectKind::Table;
    let (mut harness, tab, id) = covers_as(Look::macos(), table, true);
    select_rows(&mut harness, tab, id, 0, 2);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(marked(&harness, tab, id).is_empty());
    assert_eq!(said(&harness).as_deref(), Some("Read-only connection"));
}

/// Spec 2 and test 9: each marked row keeps its pending cells under the
/// mark, and a restore of the range gives them back.
#[test]
fn the_rows_of_a_range_keep_their_edits_under_the_mark() {
    let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(5));
    set(&mut harness, tab, id, (1, 2), "audio");
    set(&mut harness, tab, id, (3, 2), "audio");
    select_rows(&mut harness, tab, id, 1, 3);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(edits(&harness, tab, id).cells.is_empty());
    harness.press(Key::Backspace, Modifiers::NONE);
    let cells: Vec<(usize, usize)> = edits(&harness, tab, id).cells.keys().copied().collect();
    assert_eq!(cells, [(1, 2), (3, 2)]);
}

/// Spec 2 and test 11: what a screen reader is told of several rows.
#[test]
fn a_screen_reader_is_told_how_many_rows_were_marked_and_restored() {
    let (mut harness, tab, id) = covers_with(Look::macos(), book_covers_page(5));
    select(&mut harness, tab, id, (0, 1));
    harness.press(Key::Backspace, Modifiers::NONE);
    select_rows(&mut harness, tab, id, 1, 3);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(polite(
        &mut harness,
        "3 rows marked for deletion. 4 to delete."
    ));
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(polite(&mut harness, "3 rows restored."));
}

/// Spec 2, M8 and test 12: the inspector's Delete is of the row it shows.
#[test]
fn the_panels_delete_marks_the_cursors_row_alone() {
    for look in desktop() {
        let (mut harness, tab, id) = covers_with(look, book_covers_page(5));
        select_rows(&mut harness, tab, id, 1, 3);
        assert_eq!(button(&mut harness, "Delete"), (true, None));
        harness.click("Delete");
        harness.settle();
        assert_eq!(marked(&harness, tab, id), [3]);
        assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
    }
}

/// Whether a visual selection of rows is on.
fn visual(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.object_tab(id).unwrap().visual()
}

/// Moves the cursor by `rows`, as `j` and `k` do.
fn move_rows(harness: &mut Harness, tab: ConnTabId, id: TabId, rows: isize) {
    let cols = 0;
    harness.app.apply(Action::MoveSelection {
        tab,
        id,
        rows,
        cols,
    });
}

/// Spec 1 (Omarchy) and spec 2: the delete of a visual selection marks its
/// rows, leaves visual and puts the cursor on the first of them. Leaving
/// it marks nothing and keeps the cursor.
#[test]
fn a_visual_selections_delete_marks_its_rows_and_leaves_it() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    select(&mut harness, tab, id, (3, 2));
    harness.app.apply(Action::SelectRows { tab, id });
    assert!(visual(&harness, tab, id));
    // Up from the anchor: the first row of the range is the cursor's.
    move_rows(&mut harness, tab, id, -2);
    assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
    harness.app.apply(Action::DeleteSelected { tab, id });
    assert_eq!(marked(&harness, tab, id), [1, 2, 3]);
    assert!(!visual(&harness, tab, id));
    assert_eq!(selected(&harness, tab, id), Some((1, 2)));
    assert_eq!(rows(&harness, tab, id), [1]);
    // Down from the anchor: the cursor goes back to the first row.
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::SelectRows { tab, id });
    move_rows(&mut harness, tab, id, 2);
    harness.app.apply(Action::DeleteSelected { tab, id });
    assert!(marked(&harness, tab, id).is_empty());
    assert_eq!(selected(&harness, tab, id), Some((1, 2)));
    // Left, it marks nothing and the cursor stays.
    harness.app.apply(Action::SelectRows { tab, id });
    move_rows(&mut harness, tab, id, 2);
    harness.app.apply(Action::LeaveVisual { tab, id });
    assert!(!visual(&harness, tab, id));
    assert!(marked(&harness, tab, id).is_empty());
    assert_eq!(selected(&harness, tab, id), Some((3, 2)));
}

/// Spec 2: a visual selection's delete drops its new rows, and the cursor
/// is on the first row of it that is left.
#[test]
fn a_visual_selections_delete_lands_on_the_first_row_left() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(4));
    let new = add_below(&mut harness, tab, id, 0);
    select(&mut harness, tab, id, (new, 1));
    harness.app.apply(Action::SelectRows { tab, id });
    move_rows(&mut harness, tab, id, 2);
    assert_eq!(rows(&harness, tab, id), [new, 1, 2]);
    harness.app.apply(Action::DeleteSelected { tab, id });
    assert!(edits(&harness, tab, id).added.is_empty());
    assert_eq!(marked(&harness, tab, id), [1, 2]);
    assert!(!visual(&harness, tab, id));
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
}

/// Spec 1 (Omarchy) and test 3: `V` starts a visual selection, `j` and `k`
/// extend it, the keys that act on one cell or row do nothing in it, a
/// move along the row keeps its rows, and `d` marks them and leaves it.
#[test]
fn v_selects_rows_and_d_marks_them() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::V, "V");
    assert!(visual(&harness, tab, id));
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::J, "j");
    assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
    // Held back: an editor, a new row, Set NULL, a followed key, the
    // inspector's fields.
    type_key(&mut harness, Key::I, "i");
    type_key(&mut harness, Key::O, "o");
    type_key(&mut harness, Key::X, "x");
    type_key(&mut harness, Key::G, "g");
    type_key(&mut harness, Key::D, "d");
    harness.press(Key::Enter, Modifiers::NONE);
    let pending = edits(&harness, tab, id);
    assert!(pending.editor.is_none() && pending.added.is_empty() && pending.cells.is_empty());
    assert!(marked(&harness, tab, id).is_empty());
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.object_tab(id).unwrap().focus_field, None);
    assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
    // Along the row the cell changes and the rows do not.
    type_key(&mut harness, Key::L, "l");
    assert_eq!(selected(&harness, tab, id), Some((3, 3)));
    assert_eq!(rows(&harness, tab, id), [1, 2, 3]);
    type_key(&mut harness, Key::D, "d");
    assert_eq!(marked(&harness, tab, id), [1, 2, 3]);
    assert!(!visual(&harness, tab, id));
    assert_eq!(selected(&harness, tab, id), Some((1, 3)));
    // On one row too: `V`, `d` marks it and leaves the selection.
    select(&mut harness, tab, id, (4, 2));
    type_key(&mut harness, Key::V, "V");
    type_key(&mut harness, Key::D, "d");
    assert_eq!(marked(&harness, tab, id), [1, 2, 3, 4]);
    assert!(!visual(&harness, tab, id));
}

/// Test 3: `esc` and a second `V` leave a visual selection and mark
/// nothing, and a save is not held back.
#[test]
fn esc_and_v_leave_a_visual_selection() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::V, "V");
    type_key(&mut harness, Key::J, "j");
    assert!(visual(&harness, tab, id));
    assert_eq!(rows(&harness, tab, id), [1, 2]);
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!visual(&harness, tab, id));
    assert!(marked(&harness, tab, id).is_empty());
    assert_eq!(selected(&harness, tab, id), Some((2, 2)));
    type_key(&mut harness, Key::V, "V");
    assert!(visual(&harness, tab, id));
    type_key(&mut harness, Key::V, "V");
    assert!(!visual(&harness, tab, id));
    // With something pending, the save's chord works in it.
    set(&mut harness, tab, id, (0, 2), "audio");
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::V, "V");
    harness.press(Key::S, Modifiers::COMMAND);
    assert!(edits(&harness, tab, id).saving.is_some());
}

/// Spec 1 (Omarchy) and test 4: with something else for `esc` to close,
/// the first `esc` leaves the visual selection and the next does what it
/// does today.
#[test]
fn esc_leaves_a_visual_selection_before_it_closes_anything() {
    // The Review SQL panel.
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    set(&mut harness, tab, id, (0, 2), "audio");
    let show = true;
    harness.app.apply(Action::ReviewEdits { tab, id, show });
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::V, "V");
    assert!(visual(&harness, tab, id));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!visual(&harness, tab, id));
    assert!(edits(&harness, tab, id).reviewing);
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!edits(&harness, tab, id).reviewing);
    // The row panel.
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    select(&mut harness, tab, id, (1, 2));
    harness.app.workspace_mut(tab).unwrap().row_panel = true;
    type_key(&mut harness, Key::V, "V");
    assert!(visual(&harness, tab, id));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!visual(&harness, tab, id));
    assert!(harness.app.workspace(tab).unwrap().row_panel);
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!harness.app.workspace(tab).unwrap().row_panel);
    // What the last save came to: a row it found already deleted, which
    // is what the line keeps saying until it is dismissed.
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    select(&mut harness, tab, id, (0, 1));
    harness.app.apply(Action::DeleteRow { tab, id });
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
        inserted: Vec::new(),
        rows: Vec::new(),
        gone: vec![0],
        elapsed: std::time::Duration::from_millis(14),
    }));
    harness.settle();
    assert!(edits(&harness, tab, id).note.is_some());
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::V, "V");
    assert!(visual(&harness, tab, id));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!visual(&harness, tab, id));
    assert!(edits(&harness, tab, id).note.is_some());
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(edits(&harness, tab, id).note.is_none());
}

/// Spec 1 (Omarchy), spec 2 and test 3: the status line says the mode, how
/// many rows the selection holds and its two keys, and a screen reader is
/// told the same as it grows.
#[test]
fn a_visual_selection_shows_in_the_status_line_and_is_told() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(5));
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    select(&mut harness, tab, id, (1, 2));
    assert!(!painted(&harness, "-- VISUAL --"));
    type_key(&mut harness, Key::V, "V");
    assert!(painted(&harness, "-- VISUAL --"));
    assert!(painted(&harness, "1 row"));
    assert!(polite(&mut harness, "Visual. 1 row selected."));
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::J, "j");
    assert!(painted(&harness, "3 rows"));
    assert!(polite(&mut harness, "Visual. 3 rows selected."));
    // The mode is in the palette's magenta, as the board's is.
    let magenta = harness.app.palette.magenta;
    let mode =
        |(text, color): &(String, egui::Color32)| text == "-- VISUAL --" && *color == magenta;
    assert!(harness.painted.iter().any(mode), "{:?}", harness.painted);
    // Its keys, one piece, in place of the grid's.
    assert!(painted(&harness, "d delete · esc normal"));
    assert!(!painted(&harness, "dd delete"));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!painted(&harness, "-- VISUAL --"));
    assert!(painted(&harness, "dd delete"));
}

/// A delete the table refuses leaves the visual selection on, and the
/// status line says why in place of its keys.
#[test]
fn a_refused_delete_is_said_on_the_visual_line() {
    let table = tabletist_db::ObjectKind::Table;
    let (mut harness, tab, id) = covers_as(Look::omarchy(), table, true);
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    select(&mut harness, tab, id, (0, 1));
    type_key(&mut harness, Key::V, "V");
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::D, "d");
    assert!(visual(&harness, tab, id));
    assert!(marked(&harness, tab, id).is_empty());
    assert!(painted(&harness, "-- VISUAL --"));
    assert!(painted(&harness, "read-only connection"));
    assert!(!painted(&harness, "d delete · esc normal"));
}

/// Types `text` on the grid, a key at a time, as digits and letters come.
fn type_keys(harness: &mut Harness, text: &str) {
    for char in text.chars() {
        let key = Key::from_name(&char.to_uppercase().to_string()).expect("a key of that name");
        type_key(harness, key, &char.to_string());
    }
}

/// Spec 2 and 3, M10 and test 5: a count before `dd` marks the cursor's
/// row and the rows after it, stops at the last row, and puts the cursor
/// after the last row it named.
#[test]
fn a_count_before_dd_marks_that_many_rows_down() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(6));
    select(&mut harness, tab, id, (1, 2));
    type_keys(&mut harness, "3dd");
    assert_eq!(marked(&harness, tab, id), [1, 2, 3]);
    assert_eq!(selected(&harness, tab, id), Some((4, 2)));
    assert_eq!(rows(&harness, tab, id), [4]);
    // Two rows from the end, five asked for: two are marked, and the
    // cursor stays on the last.
    type_keys(&mut harness, "5dd");
    assert_eq!(marked(&harness, tab, id), [1, 2, 3, 4, 5]);
    assert_eq!(selected(&harness, tab, id), Some((5, 2)));
    // The same rows again are all marked: the count restores them.
    select(&mut harness, tab, id, (4, 2));
    type_keys(&mut harness, "2dd");
    assert_eq!(marked(&harness, tab, id), [1, 2, 3]);
    // A count of one moves on, where plain `dd` stays.
    select(&mut harness, tab, id, (0, 2));
    type_keys(&mut harness, "1dd");
    assert_eq!(marked(&harness, tab, id), [0, 1, 2, 3]);
    assert_eq!(selected(&harness, tab, id), Some((1, 2)));
}

/// Spec 3 and test 5: nought is no count, a count before another key is
/// dropped, and `:` after one opens the prompt. But for the prompt this
/// holds what was so before a count was read: a digit was nobody's.
#[test]
fn a_count_is_dropped_before_any_key_but_dd() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(6));
    select(&mut harness, tab, id, (1, 2));
    type_keys(&mut harness, "0dd");
    assert_eq!(marked(&harness, tab, id), [1]);
    assert_eq!(selected(&harness, tab, id), Some((1, 2)));
    type_keys(&mut harness, "3j");
    assert_eq!(selected(&harness, tab, id), Some((2, 2)));
    assert_eq!(marked(&harness, tab, id), [1]);
    type_key(&mut harness, Key::Num3, "3");
    type_key(&mut harness, Key::Colon, ":");
    assert!(harness.app.workspace(tab).unwrap().command.is_some());
    harness.press(Key::Escape, Modifiers::NONE);
    // The count went with the prompt: `dd` marks one row.
    type_keys(&mut harness, "dd");
    assert_eq!(marked(&harness, tab, id), [1, 2]);
}

/// Spec 2 and test 6, by count: one action takes 1,000 rows and no more.
#[test]
fn a_count_over_a_thousand_rows_is_refused_with_the_message() {
    let (mut harness, tab, id) = covers_with(Look::omarchy(), book_covers_page(1_001));
    select(&mut harness, tab, id, (0, 1));
    type_keys(&mut harness, "1001dd");
    assert!(marked(&harness, tab, id).is_empty());
    assert_eq!(selected(&harness, tab, id), Some((0, 1)));
    assert_eq!(
        said(&harness).as_deref(),
        Some("select fewer rows, or delete in the sql editor.")
    );
    type_keys(&mut harness, "1000dd");
    assert_eq!(marked(&harness, tab, id).len(), 1_000);
    assert_eq!(selected(&harness, tab, id), Some((1_000, 1)));
}
