//! The terminal look's status line names the keys of the pane that has
//! the keyboard: the tree's, its filter field's, the grid's, the row
//! panel's, a SQL editor's script and its results. Ctrl+h/l is among them
//! wherever it moves the keyboard.

use egui::{Key, Modifiers};
use tabletist_db::ObjectKind;

use super::insert_audit_tests::{covers_as, covers_in, select, type_key};
use super::pane_keys_tests::ran;
use crate::model::{Action, Advance, CellPos, EditStart, Pane};
use crate::testing::Harness;
use crate::theme::Look;

/// What the status line painted, left to right: its hints, then its
/// summary.
fn line(harness: &Harness) -> Vec<String> {
    let top = harness.size.y - crate::ui::workspace::STATUS_LINE;
    let mut on: Vec<&(String, egui::Rect)> = harness
        .text_rects
        .iter()
        .filter(|(_, rect)| rect.top() >= top)
        .collect();
    on.sort_by(|a, b| a.1.left().total_cmp(&b.1.left()));
    on.into_iter().map(|(text, _)| text.clone()).collect()
}

/// Whether the line's hints start with `hints`, in that order.
fn starts(harness: &Harness, hints: &[&str]) -> bool {
    let line = line(harness);
    line.len() >= hints.len() && line.iter().zip(hints).all(|(on, hint)| on == hint)
}

/// The hint the line has right after `hint`.
fn after(harness: &Harness, hint: &str) -> Option<String> {
    let line = line(harness);
    let at = line.iter().position(|on| on == hint)?;
    line.get(at + 1).cloned()
}

fn has(harness: &Harness, hint: &str) -> bool {
    line(harness).iter().any(|on| on == hint)
}

/// The tree's set, as the terminal look writes it.
const TREE: [&str; 7] = [
    "j/k move",
    "enter open",
    "/ filter",
    "t tree",
    "za fold",
    "ctrl+h/l panes",
    "E export table",
];

#[test]
fn slash_then_esc_from_the_tree_names_the_trees_keys_and_the_way_back() {
    let (mut harness, tab, _) = covers_in(Look::omarchy());
    harness.press(Key::H, Modifiers::CTRL);
    harness.settle();
    assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Tree);
    assert!(starts(&harness, &TREE), "{:?}", line(&harness));
    // In the filter field the letters are typed: its line says how out.
    type_key(&mut harness, Key::Slash, "/");
    harness.settle();
    assert!(
        starts(&harness, &["enter/esc tree"]),
        "{:?}",
        line(&harness)
    );
    assert!(!has(&harness, "j/k move"), "{:?}", line(&harness));
    harness.press(Key::Escape, Modifiers::NONE);
    harness.settle();
    assert!(starts(&harness, &TREE), "{:?}", line(&harness));
}

#[test]
fn ctrl_l_from_the_tree_names_the_grids_keys() {
    let (mut harness, tab, _) = covers_in(Look::omarchy());
    harness.press(Key::H, Modifiers::CTRL);
    harness.settle();
    harness.press(Key::L, Modifiers::CTRL);
    harness.settle();
    assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Grid);
    assert!(
        starts(&harness, &["j/k row", "h/l col"]),
        "{:?}",
        line(&harness)
    );
    assert_eq!(
        after(&harness, "ctrl+b tables").as_deref(),
        Some("ctrl+h/l panes"),
        "{:?}",
        line(&harness)
    );
    assert!(!has(&harness, "j/k move"), "{:?}", line(&harness));
}

#[test]
fn ctrl_l_from_a_row_names_the_panels_keys_and_ctrl_h_the_grids_again() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::L, Modifiers::CTRL);
    harness.settle();
    let panel = [
        "j/k field",
        "[ ] row",
        "i edit",
        "gy copy as",
        "esc grid",
        "ctrl+h/l panes",
    ];
    assert!(starts(&harness, &panel), "{:?}", line(&harness));
    harness.press(Key::H, Modifiers::CTRL);
    harness.settle();
    assert!(
        starts(&harness, &["j/k row", "h/l col"]),
        "{:?}",
        line(&harness)
    );
}

#[test]
fn a_read_only_rows_panel_takes_no_keyboard_and_the_grids_keys_stay() {
    // Every field of a row on a read-only connection is locked: ctrl+l has
    // no field to go to, and the line keeps the rows' keys.
    let (mut harness, tab, id) = covers_as(Look::omarchy(), ObjectKind::Table, true);
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::L, Modifiers::CTRL);
    harness.settle();
    assert!(
        starts(&harness, &["j/k row", "h/l col"]),
        "{:?}",
        line(&harness)
    );
    assert!(!has(&harness, "j/k field"), "{:?}", line(&harness));
}

#[test]
fn a_sql_editor_names_its_scripts_keys_then_its_results() {
    let (mut harness, _, _) = ran(Look::omarchy(), "SELECT 1");
    let line_now = line(&harness);
    let script = line_now.iter().position(|on| on == "ctrl+enter run");
    assert!(script.is_some(), "{line_now:?}");
    // Down to the results, right before the tables.
    assert_eq!(
        after(&harness, "ctrl+j results").as_deref(),
        Some("ctrl+b tables"),
        "{line_now:?}"
    );
    harness.press(Key::J, Modifiers::CTRL);
    harness.settle();
    let results = [
        "j/k row",
        "h/l col",
        "enter inspect",
        "v y copy",
        "gy copy as",
        "ctrl+k editor",
        "ctrl+h/l panes",
        "E export result",
    ];
    assert!(starts(&harness, &results), "{:?}", line(&harness));
}

#[test]
fn with_the_agents_tab_in_front_the_trees_keys_have_no_pane_keys() {
    let (mut harness, tab, _) = covers_in(Look::omarchy());
    harness.app.apply(Action::ShowAgents(tab));
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
    harness.settle();
    harness.settle();
    assert!(starts(&harness, &TREE[..5]), "{:?}", line(&harness));
    assert!(!has(&harness, "ctrl+h/l panes"), "{:?}", line(&harness));
    assert!(has(&harness, "E export table"), "{:?}", line(&harness));
}

#[test]
fn where_room_runs_out_the_export_gives_way_first_then_the_pane_keys() {
    let (mut harness, _, _) = covers_in(Look::omarchy());
    harness.press(Key::H, Modifiers::CTRL);
    harness.settle();
    let full = harness.size;
    // Narrower and narrower, until the export has gone, then the pane
    // keys; the fold before them stays the longest.
    let mut width = full.x;
    let mut gone = Vec::new();
    while has(&harness, "j/k move") && width > 200.0 {
        width -= 10.0;
        harness.size = egui::vec2(width, full.y);
        harness.settle();
        for hint in ["E export table", "ctrl+h/l panes", "za fold"] {
            if !has(&harness, hint) && !gone.contains(&hint) {
                gone.push(hint);
            }
        }
    }
    assert_eq!(gone, ["E export table", "ctrl+h/l panes", "za fold"]);
}

#[test]
fn with_a_change_pending_the_grids_write_and_review_keep_their_room() {
    // A change on the rows of a full-width window: the line's `:w` and
    // `:diff` stand before the pane keys do.
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    let cell = CellPos { row: 1, col: 2 };
    let start = EditStart::Replace("print".into());
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell,
        start,
    });
    let then = Advance::Stay;
    harness.app.apply(Action::CommitEdit { tab, id, then });
    harness.settle();
    for hint in [":w write", ":diff review"] {
        assert!(has(&harness, hint), "{hint}: {:?}", line(&harness));
    }
    // From the tree, while it is pending, the prompt's keys too.
    harness.press(Key::H, Modifiers::CTRL);
    harness.settle();
    assert!(starts(&harness, &TREE[..5]), "{:?}", line(&harness));
    for hint in [":w write", ":diff review"] {
        assert!(has(&harness, hint), "{hint}: {:?}", line(&harness));
    }
}

#[test]
fn the_other_looks_keep_their_line() {
    for look in [Look::macos(), Look::standard()] {
        let name = look.name;
        let (mut harness, tab, _) = covers_in(look);
        harness.app.workspace_mut(tab).unwrap().pane = Pane::Tree;
        harness.settle();
        assert!(!has(&harness, "j/k move"), "{name} {:?}", line(&harness));
        assert!(
            !has(&harness, "ctrl+h/l panes"),
            "{name} {:?}",
            line(&harness)
        );
    }
}

/// Clicks the tree's filter field with the pointer.
fn click_filter(harness: &mut Harness) {
    let tree = harness.settle();
    let role = egui::accesskit::Role::TextInput;
    let field = crate::testing::bounds(&tree, "Filter", role).expect("the filter field");
    let pos = field.center();
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(pos)]);
    harness.frame(vec![press(true)]);
    harness.frame(vec![press(false)]);
    harness.settle();
}

#[test]
fn the_filter_clicked_from_the_grid_leaves_for_the_tree_by_enter_or_esc() {
    for leave in [Key::Escape, Key::Enter] {
        let (mut harness, tab, _) = covers_in(Look::omarchy());
        assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Grid);
        click_filter(&mut harness);
        assert!(
            starts(&harness, &["enter/esc tree"]),
            "{leave:?}: {:?}",
            line(&harness)
        );
        harness.press(leave, Modifiers::NONE);
        harness.settle();
        assert_eq!(
            harness.app.workspace(tab).unwrap().pane,
            Pane::Tree,
            "{leave:?}"
        );
        assert!(starts(&harness, &TREE), "{leave:?}: {:?}", line(&harness));
    }
}

#[test]
fn the_filter_names_its_keys_over_a_visual_selection_of_the_grid() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Grid);
    select(&mut harness, tab, id, (0, 1));
    type_key(&mut harness, Key::V, "v");
    type_key(&mut harness, Key::J, "j");
    harness.settle();
    assert!(has(&harness, "-- VISUAL --"), "{:?}", line(&harness));
    click_filter(&mut harness);
    assert!(
        starts(&harness, &["enter/esc tree"]),
        "{:?}",
        line(&harness)
    );
    assert!(!has(&harness, "-- VISUAL --"), "{:?}", line(&harness));
}
