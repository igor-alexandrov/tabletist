//! The terminal look's keys that move the keyboard up and down between
//! the panes stacked in a tab: a SQL editor's script over its results.
//! Ctrl+J from the script (Focus results), and Ctrl+J and Ctrl+K out of a
//! text field (Pane below, Pane above).

use egui::{Key, Modifiers};
use tabletist_db::Driver;

use crate::model::{ConnTabId, TabId};
use crate::settings::NewTab;
use crate::testing::{Harness, rows_outcome, script_outcome};
use crate::theme::Look;
use crate::ui::tests::focused_name;

/// A connection in `look` with a new SQL editor that holds `text`, has the
/// keyboard, and has run once: its results show three rows.
fn ran(look: Look, text: &str) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    let run = if look.terminal {
        Modifiers::CTRL
    } else {
        Modifiers::COMMAND
    };
    harness.set_look(look);
    harness.app.settings.new_tab = NewTab::Connection;
    let tab = harness.connect_fake_as(false);
    harness.app.workspace_mut(tab).unwrap().driver = Driver::Sqlite;
    harness.press(Key::T, Modifiers::COMMAND);
    harness.frame(vec![egui::Event::Paste(text.into())]);
    harness.settle();
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.press(Key::Enter, run);
    harness.answer_sql(Ok(script_outcome(vec![rows_outcome(3)])), None);
    harness.settle();
    (harness, tab, id)
}

/// Whether the script has the keyboard.
fn in_script(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
    let field = crate::ui::sql_text::editor_id(tab, id);
    harness.ctx.memory(|memory| memory.has_focus(field))
}

fn press(harness: &mut Harness, key: Key) -> String {
    harness.press(key, Modifiers::CTRL);
    focused_name(&harness.settle())
}

#[test]
fn ctrl_j_takes_the_keyboard_from_the_script_to_its_results() {
    let (mut harness, tab, id) = ran(Look::omarchy(), "SELECT 1");
    assert!(in_script(&harness, tab, id), "the script has the keyboard");
    let text = harness
        .app
        .workspace(tab)
        .unwrap()
        .sql_tab(id)
        .unwrap()
        .text
        .clone();
    assert_eq!(press(&mut harness, Key::J), "Rows");
    assert!(!in_script(&harness, tab, id));
    // The key typed nothing into the script.
    let after = &harness
        .app
        .workspace(tab)
        .unwrap()
        .sql_tab(id)
        .unwrap()
        .text;
    assert_eq!(after, &text);
    // On the rows the arrows are the results': j moves the cursor down.
    let selected = |harness: &Harness| {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace
            .sql_tab(id)
            .unwrap()
            .selection
            .map(|cell| cell.row)
    };
    let first = selected(&harness);
    crate::ui::tests::type_key(&mut harness, Key::J, "j");
    assert_ne!(selected(&harness), first, "j moved in the results");
}

#[test]
fn ctrl_k_takes_it_back_up_to_the_script_and_ctrl_j_down_again() {
    let (mut harness, tab, id) = ran(Look::omarchy(), "SELECT 1");
    press(&mut harness, Key::J);
    assert!(!in_script(&harness, tab, id));
    press(&mut harness, Key::K);
    assert!(in_script(&harness, tab, id), "up again: the script");
    // Out of the script with Esc, the keys are normal mode's: ctrl+j is
    // the pane below.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.settle();
    assert!(!in_script(&harness, tab, id));
    assert_eq!(press(&mut harness, Key::J), "Rows");
    // Nothing is below the results, nor above the script.
    assert_eq!(press(&mut harness, Key::J), "Rows");
    press(&mut harness, Key::K);
    press(&mut harness, Key::K);
    assert!(in_script(&harness, tab, id), "nothing above the script");
}

#[test]
fn the_other_looks_leave_ctrl_j_and_ctrl_k_to_the_script() {
    for look in [Look::macos(), Look::standard()] {
        let name = look.name;
        let (mut harness, tab, id) = ran(look, "SELECT 1");
        for key in [Key::J, Key::K] {
            press(&mut harness, key);
            assert!(in_script(&harness, tab, id), "{name} {key:?}");
        }
    }
}

#[test]
fn a_tables_rows_have_no_pane_above_or_below() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let tab = harness.connect_fake();
    harness.click("users");
    harness.answer_rows(crate::testing::page(5, false));
    harness.press(Key::L, Modifiers::CTRL);
    assert_eq!(focused_name(&harness.settle()), "Rows");
    for key in [Key::J, Key::K] {
        assert_eq!(press(&mut harness, key), "Rows", "{key:?}");
        let pane = harness.app.workspace(tab).unwrap().pane;
        assert_eq!(pane, crate::model::Pane::Grid);
    }
}
