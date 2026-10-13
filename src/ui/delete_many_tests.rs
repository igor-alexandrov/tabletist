//! Deleting many rows at once, checked against its spec
//! (`docs/superpowers/specs/2026-10-13-deleting-many-rows-design.md`:
//! "spec N" below is one of its sections, "test N" an entry of its Tests).
//!
//! Only behaviour, names and words are checked here, on the Bookshop's
//! `book_covers`. No colour, size or picture of the design is.

use egui::{Key, Modifiers};

use super::insert_audit_tests::{add, select, selected};
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
