//! Reload from the tree reloads the tree whatever tab is in front: a SQL
//! editor's included, which has nothing to reload itself (issue #300).

use egui::{Key, Modifiers};

use super::pane_keys_tests::ran;
use crate::backend::Command;
use crate::model::{ConnTabId, Pane, TabId};
use crate::testing::Harness;
use crate::theme::Look;
use crate::ui::tests::{focused_name, type_key};

/// How many times the tree has been asked for again.
fn tree_loads(harness: &Harness) -> usize {
    let sent = harness.app.backend.sent.iter();
    sent.filter(|command| matches!(command, Command::ListSchemas { .. }))
        .count()
}

/// Presses the look's Reload: Omarchy's is `R`.
fn reload(harness: &mut Harness) {
    if harness.app.look.terminal {
        type_key(harness, Key::R, "R");
    } else {
        harness.press(Key::R, Modifiers::COMMAND);
    }
    harness.settle();
}

/// Gives the tree the keyboard from the SQL editor: in the terminal look
/// out of the script and to the left, as one does there; in the others
/// the editor lets go of it, as a click in the tree makes it do.
fn to_the_tree(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    if harness.app.look.terminal {
        harness.leave_editor();
        harness.press(Key::H, Modifiers::CTRL);
    } else {
        let_go(harness, tab, id);
        harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
    }
    harness.settle();
    assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Tree);
}

/// The SQL editor's script lets go of the keyboard.
fn let_go(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    let field = crate::ui::sql_text::editor_id(tab, id);
    harness
        .ctx
        .memory_mut(|memory| memory.surrender_focus(field));
}

#[test]
fn reload_from_the_tree_reloads_it_over_a_sql_editor() {
    for look in Look::ALL {
        let name = look.name;
        let (mut harness, tab, id) = ran(look, "SELECT 1");
        to_the_tree(&mut harness, tab, id);
        let before = tree_loads(&harness);
        reload(&mut harness);
        assert_eq!(tree_loads(&harness), before + 1, "{name}");
        // The SQL tab stays in front, its text as it was.
        let workspace = harness.app.workspace(tab).unwrap();
        let sql = workspace.active_sql_tab().expect("the SQL tab in front");
        assert_eq!(sql.text, "SELECT 1", "{name}");
    }
}

#[test]
fn reload_from_a_sql_editor_reloads_nothing() {
    for look in Look::ALL {
        let name = look.name;
        let (mut harness, tab, id) = ran(look, "SELECT 1");
        // The script has the keyboard: the key is its own, and the tree's
        // reload is not.
        let before = tree_loads(&harness);
        reload(&mut harness);
        assert_eq!(tree_loads(&harness), before, "{name}: from the script");
        // Nor from its results.
        if look.terminal {
            harness.press(Key::J, Modifiers::CTRL);
            assert_eq!(focused_name(&harness.settle()), "Rows", "{name}");
        } else {
            let_go(&mut harness, tab, id);
            harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
            harness.settle();
        }
        let before = tree_loads(&harness);
        reload(&mut harness);
        assert_eq!(tree_loads(&harness), before, "{name}: from the results");
    }
}
