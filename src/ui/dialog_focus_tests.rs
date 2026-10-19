//! Closing a dialog gives the keyboard back to what had it before the
//! dialog opened: egui takes it from everything drawn behind a dialog.

use egui::{Key, Modifiers};
use tabletist_db::{ObjectKind, ObjectRef};

use crate::model::{Action, ConnTabId, Dialog, Pane};
use crate::testing::{Harness, bookshop_objects, fixture_connection};
use crate::theme::Look;

/// A Bookshop connection in `look`, its start screen showing.
fn bookshop(look: Look) -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_with(fixture_connection(), bookshop_objects());
    (harness, tab)
}

/// Opens the Bookshop's `authors` table, its first page read.
fn open_authors(harness: &mut Harness, tab: ConnTabId) {
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "authors"),
        kind: ObjectKind::Table,
        pin: true,
    });
    harness.answer_rows(crate::testing::page(3, false));
}

/// The name a screen reader would read for what has the keyboard.
fn focused(harness: &mut Harness) -> String {
    super::tests::focused_name(&harness.settle())
}

/// Steps with F6 until the keyboard is on `name`.
fn reach(harness: &mut Harness, name: &str) {
    let mut seen = vec![focused(harness)];
    while seen.last().is_none_or(|last| last != name) {
        assert!(seen.len() <= 12, "{name} not reached: {seen:?}");
        harness.press(Key::F6, Modifiers::NONE);
        seen.push(focused(harness));
    }
}

/// `?` over the tree, then Escape: the tree has the keyboard again, on
/// the start screen and with a table open, in every look.
#[test]
fn closing_the_shortcuts_gives_the_keyboard_back_to_the_tree() {
    for look in Look::ALL {
        for table in [false, true] {
            let (mut harness, tab) = bookshop(look);
            if table {
                open_authors(&mut harness, tab);
            }
            reach(&mut harness, "Objects");
            super::tests::type_key(&mut harness, Key::Questionmark, "?");
            assert!(
                matches!(harness.app.dialog, Some(Dialog::Help)),
                "{}",
                look.name
            );
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(harness.app.dialog.is_none(), "{}", look.name);
            let name = focused(&mut harness);
            assert_eq!(name, "Objects", "{}, a table open: {table}", look.name);
        }
    }
}

/// The rows have the keyboard again after the shortcuts too.
#[test]
fn closing_the_shortcuts_gives_the_keyboard_back_to_the_rows() {
    let (mut harness, tab) = bookshop(Look::macos());
    open_authors(&mut harness, tab);
    reach(&mut harness, "Rows");
    super::tests::type_key(&mut harness, Key::Questionmark, "?");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    assert_eq!(focused(&mut harness), "Rows");
}

/// Quick open closed with Escape: the tree has the keyboard again.
#[test]
fn closing_quick_open_gives_the_keyboard_back() {
    let (mut harness, _tab) = bookshop(Look::macos());
    reach(&mut harness, "Objects");
    harness.press(Key::P, Modifiers::COMMAND);
    assert!(matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    assert_eq!(focused(&mut harness), "Objects");
}

/// Settings opened from the shortcuts take their place: what had the
/// keyboard before the shortcuts has it once Settings closes.
#[test]
fn a_dialog_that_gives_way_to_another_gives_the_keyboard_back_once_closed() {
    let (mut harness, _tab) = bookshop(Look::macos());
    reach(&mut harness, "Objects");
    super::tests::type_key(&mut harness, Key::Questionmark, "?");
    harness.click("Settings");
    assert!(matches!(harness.app.dialog, Some(Dialog::Settings(_))));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    assert_eq!(focused(&mut harness), "Objects");
}

/// Quick open's answer opens a table and gives it the keys: the tree,
/// which had the keyboard, does not take it back.
#[test]
fn quick_open_leaves_the_keyboard_with_the_table_it_opens() {
    let (mut harness, tab) = bookshop(Look::macos());
    reach(&mut harness, "Objects");
    harness.press(Key::P, Modifiers::COMMAND);
    harness.frame(vec![egui::Event::Text("authors".into())]);
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    assert_ne!(focused(&mut harness), "Objects");
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.pane, Pane::Grid);
    let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
    assert_eq!(object.object.name, "authors");
}
