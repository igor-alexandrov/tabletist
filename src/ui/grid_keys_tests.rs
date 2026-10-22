//! A table's keys that were in the keymap before they did anything: Down
//! past the last row (Add row after the last), and the keys that move
//! between errors (Next error, Previous error), in the grid and in a SQL
//! editor.

use egui::{Key, Modifiers};
use tabletist_db::{ObjectKind, ObjectRef};

use super::insert_audit_tests::{
    covers_as, covers_in, edits, select, selected, two_rows_to_fix, type_key,
};
use crate::edit::{Order, Pending, State, new_row};
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

/// The key that goes to the next error (`forward`) or the previous one, as
/// `look` has it: a chord, or the terminal's two letters.
fn to_error(harness: &mut Harness, forward: bool) {
    if harness.app.look.terminal {
        type_key(harness, Key::CloseBracket, "]");
        type_key(harness, Key::E, "e");
    } else {
        let key = if forward {
            Key::ArrowDown
        } else {
            Key::ArrowUp
        };
        harness.press(key, Modifiers::ALT | Modifiers::COMMAND);
    }
    harness.settle();
}

/// The terminal's key back, which is no chord.
fn to_previous_error(harness: &mut Harness) {
    if harness.app.look.terminal {
        type_key(harness, Key::OpenBracket, "[");
        type_key(harness, Key::E, "e");
        harness.settle();
    } else {
        to_error(harness, false);
    }
}

#[test]
fn the_error_keys_go_round_the_cells_to_fix() {
    for look in [Look::macos(), Look::standard(), Look::omarchy()] {
        let name = look.name;
        let (mut harness, tab, id) = two_rows_to_fix(look);
        let added: Vec<usize> = edits(&harness, tab, id)
            .added
            .iter()
            .map(|new| new.id)
            .collect();
        let (first, second) = (new_row(added[0]), new_row(added[1]));
        // The two new rows stand above the page's: from its first row the
        // next error is past the end, so the first one.
        to_error(&mut harness, true);
        assert_eq!(selected(&harness, tab, id), Some((first, 1)), "{name}");
        to_error(&mut harness, true);
        assert_eq!(selected(&harness, tab, id), Some((second, 1)), "{name}");
        to_error(&mut harness, true);
        assert_eq!(selected(&harness, tab, id), Some((first, 1)), "{name}");
        // And back, round to the last.
        to_previous_error(&mut harness);
        assert_eq!(selected(&harness, tab, id), Some((second, 1)), "{name}");
        to_previous_error(&mut harness);
        assert_eq!(selected(&harness, tab, id), Some((first, 1)), "{name}");
    }
}

#[test]
fn a_failed_statement_and_a_failed_delete_are_errors_in_the_grids_order() {
    for look in [Look::macos(), Look::omarchy()] {
        let name = look.name;
        let (mut harness, tab, id) = covers_in(look);
        let error = tabletist_db::Error::query("failed");
        {
            let object = harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap();
            let edits = &mut object.edits;
            let new = tabletist_db::NewValue::Text("ebook".into());
            let state = State::Failed(error.clone());
            edits.cells.insert((1, 2), Pending { new, state });
            assert!(edits.mark(0));
            edits.deleted.get_mut(&0).unwrap().failed = Some(error.clone());
        }
        select(&mut harness, tab, id, (2, 0));
        to_error(&mut harness, true);
        // A row's own error is on its first cell.
        assert_eq!(selected(&harness, tab, id), Some((0, 0)), "{name}");
        to_error(&mut harness, true);
        assert_eq!(selected(&harness, tab, id), Some((1, 2)), "{name}");
        to_previous_error(&mut harness);
        assert_eq!(selected(&harness, tab, id), Some((0, 0)), "{name}");
    }
}

#[test]
fn with_no_error_the_cursor_stays() {
    for look in [Look::macos(), Look::omarchy()] {
        let name = look.name;
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (1, 1));
        to_error(&mut harness, true);
        to_previous_error(&mut harness);
        assert_eq!(selected(&harness, tab, id), Some((1, 1)), "{name}");
    }
}

/// A SQL editor in `look` that ran `text` and failed with `position`,
/// counted in characters from 1.
fn failed_run(look: Look, text: &str, position: Option<usize>) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    let run = if look.terminal {
        Modifiers::CTRL
    } else {
        Modifiers::COMMAND
    };
    harness.set_look(look);
    harness.app.settings.new_tab = crate::settings::NewTab::Connection;
    let tab = harness.connect_fake_as(false);
    harness.app.workspace_mut(tab).unwrap().driver = tabletist_db::Driver::Sqlite;
    harness.press(Key::T, Modifiers::COMMAND);
    harness.frame(vec![egui::Event::Paste(text.into())]);
    harness.settle();
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.press(Key::Enter, run | Modifiers::SHIFT);
    let failed = crate::testing::error_outcome("no such column: c.created", position);
    let outcome = crate::testing::script_outcome(vec![failed]);
    harness.answer_sql(Ok(outcome), None);
    harness.settle();
    (harness, tab, id)
}

fn sql_cursor(harness: &Harness, tab: ConnTabId, id: TabId) -> usize {
    harness
        .app
        .workspace(tab)
        .unwrap()
        .sql_tab(id)
        .unwrap()
        .cursor
}

fn in_script(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
    let field = crate::ui::sql_text::editor_id(tab, id);
    harness.ctx.memory(|memory| memory.has_focus(field))
}

const SCRIPT: &str = "SELECT p.name\n  FROM covers c\n WHERE c.created > 1";

#[test]
fn the_error_keys_put_a_sql_editors_cursor_on_its_errors_mark() {
    // The mark is at "c.created" on the third line, column 8.
    let at = SCRIPT.find("c.created").unwrap();
    let position = SCRIPT[..at].chars().count() + 1;
    for look in [Look::macos(), Look::standard()] {
        let name = look.name;
        let (mut harness, tab, id) = failed_run(look, SCRIPT, Some(position));
        let mark = harness
            .app
            .workspace(tab)
            .unwrap()
            .sql_tab(id)
            .unwrap()
            .error_mark();
        assert_eq!(mark, Some((3, Some(8))), "{name}");
        // While typing, from the script's end.
        assert!(in_script(&harness, tab, id), "{name}");
        assert_ne!(sql_cursor(&harness, tab, id), at, "{name}");
        harness.press(Key::ArrowUp, Modifiers::ALT | Modifiers::COMMAND);
        harness.settle();
        assert_eq!(sql_cursor(&harness, tab, id), at, "{name}");
        assert!(in_script(&harness, tab, id), "{name}");
        // The next key goes there too: there is one mark.
        harness.press(Key::End, Modifiers::NONE);
        harness.press(Key::ArrowDown, Modifiers::ALT | Modifiers::COMMAND);
        harness.settle();
        assert_eq!(sql_cursor(&harness, tab, id), at, "{name}");
    }
    // The terminal's letters, out of the script.
    let (mut harness, tab, id) = failed_run(Look::omarchy(), SCRIPT, Some(position));
    harness.leave_editor();
    harness.settle();
    assert!(!in_script(&harness, tab, id));
    to_error(&mut harness, true);
    assert_eq!(sql_cursor(&harness, tab, id), at);
    assert!(in_script(&harness, tab, id), "the script has the keyboard");
}

#[test]
fn a_mark_with_no_column_is_its_lines_start_and_no_mark_moves_nothing() {
    let (mut harness, tab, id) = failed_run(Look::macos(), SCRIPT, None);
    harness.press(Key::ArrowDown, Modifiers::ALT | Modifiers::COMMAND);
    harness.settle();
    // The statement's first line.
    assert_eq!(sql_cursor(&harness, tab, id), 0);
    // Once the text changed the mark is gone, and the cursor stays.
    harness.frame(vec![egui::Event::Text(" ".into())]);
    harness.settle();
    let before = sql_cursor(&harness, tab, id);
    harness.press(Key::ArrowDown, Modifiers::ALT | Modifiers::COMMAND);
    harness.settle();
    assert_eq!(sql_cursor(&harness, tab, id), before);
}

#[test]
fn with_the_row_panel_shown_a_bracket_waits_only_where_an_error_is() {
    let panel = |harness: &Harness, tab| harness.app.workspace(tab).unwrap().row_panel;
    // No error: the bracket steps the row at once, as the panel's key.
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    assert!(panel(&harness, tab), "the row panel shows");
    select(&mut harness, tab, id, (0, 1));
    type_key(&mut harness, Key::CloseBracket, "]");
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
    // An error: the bracket waits, and `e` goes to it.
    let (mut harness, tab, id) = two_rows_to_fix(Look::omarchy());
    assert!(panel(&harness, tab), "the row panel shows");
    select(&mut harness, tab, id, (0, 1));
    type_key(&mut harness, Key::CloseBracket, "]");
    assert_eq!(
        selected(&harness, tab, id),
        Some((0, 1)),
        "the bracket waits"
    );
    // Another key: the bracket steps the row after all, and the key is
    // read itself: `j` moves down from there.
    type_key(&mut harness, Key::J, "j");
    assert_eq!(selected(&harness, tab, id), Some((2, 1)));
    type_key(&mut harness, Key::OpenBracket, "[");
    type_key(&mut harness, Key::K, "k");
    assert_eq!(selected(&harness, tab, id), Some((0, 1)));
}

#[test]
fn a_bracket_that_waited_steps_the_row_before_a_key_that_types_nothing() {
    // An arrow: the bracket steps the row, and the arrow moves on from it.
    let (mut harness, tab, id) = two_rows_to_fix(Look::omarchy());
    select(&mut harness, tab, id, (0, 1));
    type_key(&mut harness, Key::CloseBracket, "]");
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.settle();
    assert_eq!(selected(&harness, tab, id), Some((2, 1)));
    // The key of the changes, not built yet: the bracket steps the row too.
    select(&mut harness, tab, id, (0, 1));
    type_key(&mut harness, Key::CloseBracket, "]");
    type_key(&mut harness, Key::C, "c");
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
    // Esc: the bracket steps the row.
    let (mut harness, tab, id) = two_rows_to_fix(Look::omarchy());
    select(&mut harness, tab, id, (1, 1));
    type_key(&mut harness, Key::OpenBracket, "[");
    harness.press(Key::Escape, Modifiers::NONE);
    harness.settle();
    assert_eq!(selected(&harness, tab, id), Some((0, 1)));
}

#[test]
fn a_key_that_edits_after_a_bracket_that_waited_is_dropped() {
    // The step is applied after the frame: the edit would land on the row
    // the bracket leaves, so the letter is dropped, and the next one edits.
    let (mut harness, tab, id) = two_rows_to_fix(Look::omarchy());
    select(&mut harness, tab, id, (0, 1));
    type_key(&mut harness, Key::CloseBracket, "]");
    type_key(&mut harness, Key::I, "i");
    assert_eq!(selected(&harness, tab, id), Some((1, 1)));
    assert!(edits(&harness, tab, id).editor.is_none(), "no editor yet");
    type_key(&mut harness, Key::I, "i");
    let editing = edits(&harness, tab, id).editor.as_ref().map(|e| e.cell.row);
    assert_eq!(editing, Some(1), "the next i edits the row stepped to");
}
