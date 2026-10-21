//! A table's keys that were in the keymap before they did anything: Down
//! past the last row (Add row after the last).

use egui::{Key, Modifiers};
use tabletist_db::{ObjectKind, ObjectRef};

use super::insert_audit_tests::{covers_as, covers_in, edits, select, selected};
use crate::edit::{Order, new_row};
use crate::model::{Action, ConnTabId, TabId};
use crate::testing::Harness;
use crate::theme::Look;

/// Where the row `row` is shown in the table's grid.
fn place(harness: &Harness, tab: ConnTabId, id: TabId, row: usize) -> Option<usize> {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    let height = object.page().map_or(0, |page| page.rows.len());
    Order::of(&object.edits.added, height).place(row)
}

#[test]
fn down_on_the_last_row_adds_a_row_below_it() {
    for look in [Look::macos(), Look::standard()] {
        let name = look.name;
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (2, 1));
        harness.press(Key::ArrowDown, Modifiers::NONE);
        let added = &edits(&harness, tab, id).added;
        assert_eq!(added.len(), 1, "{name}");
        let row = new_row(added[0].id);
        // After the page's three rows, and the cursor or the editor on it.
        assert_eq!(place(&harness, tab, id, row), Some(3), "{name}");
        let editing = edits(&harness, tab, id).editor.as_ref().map(|e| e.cell.row);
        let at = selected(&harness, tab, id).map(|(row, _)| row);
        assert!(editing == Some(row) || at == Some(row), "{name}");
    }
}

#[test]
fn down_elsewhere_moves_and_a_held_down_adds_one_row() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 1));
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert!(edits(&harness, tab, id).added.is_empty());
    assert_eq!(selected(&harness, tab, id), Some((2, 1)));
    // Held on the last row: the key goes down again and again, without a
    // release, and only the first press adds.
    let down = crate::testing::key(Key::ArrowDown, Modifiers::NONE);
    for _ in 0..4 {
        harness.frame(vec![down.clone()]);
        harness.app.apply(Action::CancelEdit { tab, id });
    }
    harness.frame(vec![crate::testing::release(
        Key::ArrowDown,
        Modifiers::NONE,
    )]);
    harness.settle();
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
}

#[test]
fn down_on_the_last_row_adds_none_where_no_row_can_be_added() {
    // A connection that opens read-only.
    let (mut harness, tab, id) = covers_as(Look::macos(), ObjectKind::Table, true);
    select(&mut harness, tab, id, (2, 1));
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert!(edits(&harness, tab, id).added.is_empty());
    assert_eq!(selected(&harness, tab, id), Some((2, 1)));
    // The terminal look has no key for it: `j` stays on the last row.
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (2, 1));
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert!(edits(&harness, tab, id).added.is_empty());
    assert_eq!(selected(&harness, tab, id), Some((2, 1)));
}

#[test]
fn down_on_a_pages_last_row_with_more_pages_to_come_adds_none() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "book_covers"),
        kind: ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::book_covers_structure());
    let mut page = crate::testing::book_covers_page(3);
    page.has_more = true;
    harness.answer_rows(page);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    select(&mut harness, tab, id, (2, 1));
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert!(edits(&harness, tab, id).added.is_empty());
}
