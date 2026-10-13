//! Deleting many rows at once, checked against its spec
//! (`docs/superpowers/specs/2026-10-13-deleting-many-rows-design.md`:
//! "spec N" below is one of its sections, "test N" an entry of its Tests).
//!
//! Only behaviour, names and words are checked here, on the Bookshop's
//! `book_covers`. No colour, size or picture of the design is.

use egui::{Key, Modifiers};

use super::duplicate_tests::button;
use super::insert_audit_tests::{add, covers_as, edits, painted, select, selected, set};
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
