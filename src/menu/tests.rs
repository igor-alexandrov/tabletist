//! The menu bar's model, which items are enabled, what a click does, and
//! what a replayed key gives egui. Headless.

use crate::app::RUNNABLE;
use crate::keymap::{Command, EditItem};
use crate::model::{Action, Dialog};
use crate::testing::Harness;

use super::{Item, Platform, bar};

fn items(platform: Platform) -> Vec<Item> {
    bar(platform)
        .into_iter()
        .flat_map(|menu| menu.items)
        .collect()
}

#[test]
fn every_runnable_command_is_in_one_item() {
    for platform in [Platform::MacOs, Platform::Windows] {
        let items = items(platform);
        for command in RUNNABLE {
            if command == Command::ShowAgents && !crate::endpoint::AVAILABLE {
                assert!(!items.contains(&Item::Command(command)));
                continue;
            }
            let count = items
                .iter()
                .filter(|item| **item == Item::Command(command))
                .count();
            assert_eq!(count, 1, "{platform:?}: {command:?}");
        }
        // Nothing but RUNNABLE's commands.
        for item in &items {
            if let Item::Command(command) = item {
                assert!(RUNNABLE.contains(command), "{command:?}");
            }
        }
    }
}

#[test]
fn every_item_has_a_title_and_separators_only_between_items() {
    for platform in [Platform::MacOs, Platform::Windows] {
        for menu in bar(platform) {
            assert!(!menu.title.is_empty());
            let first = menu.items.first();
            let last = menu.items.last();
            assert_ne!(first, Some(&Item::Separator), "{}", menu.title);
            assert_ne!(last, Some(&Item::Separator), "{}", menu.title);
            for pair in menu.items.windows(2) {
                assert!(pair != [Item::Separator, Item::Separator], "{}", menu.title);
            }
            for item in &menu.items {
                if *item != Item::Separator {
                    assert!(!super::title(*item, false).is_empty(), "{item:?}");
                }
            }
        }
    }
}

#[test]
fn each_platform_has_its_own_items() {
    let mac = bar(Platform::MacOs);
    let windows = bar(Platform::Windows);
    let titles = |menus: &[super::Menu]| menus.iter().map(|menu| menu.title).collect::<Vec<_>>();
    assert_eq!(
        titles(&mac),
        [
            "Tabletist",
            "File",
            "Edit",
            "View",
            "Query",
            "Window",
            "Help"
        ]
    );
    assert_eq!(
        titles(&windows),
        ["File", "Edit", "View", "Query", "Window", "Help"]
    );
    // Exit and About in File and Help on Windows; About in the app menu
    // on the Mac, which has no Exit.
    assert!(items(Platform::Windows).contains(&Item::Exit));
    assert!(!items(Platform::MacOs).contains(&Item::Exit));
    assert_eq!(
        windows.last().map(|help| help.items.last()),
        Some(Some(&Item::About))
    );
    assert_eq!(mac[0].items[0], Item::About);
    // The Edit menu is the app's own on both.
    for platform in [Platform::MacOs, Platform::Windows] {
        for edit in EditItem::ALL {
            assert!(
                items(platform).contains(&Item::Edit(edit)),
                "{platform:?}: {edit:?}"
            );
        }
    }
}

#[test]
fn the_window_s_commands_are_enabled_on_the_connection_picker() {
    // A new harness shows the picker, which has no workspace.
    let harness = Harness::new();
    let app = &harness.app;
    for command in [
        Command::Settings,
        Command::Help,
        Command::NewConnection,
        Command::Connections,
        Command::LargerText,
        Command::SmallerText,
    ] {
        assert!(
            app.menu_state(Item::Command(command)).enabled,
            "{command:?}"
        );
    }
    // A connection's commands are not.
    assert!(
        !app.menu_state(Item::Command(Command::CloseConnection))
            .enabled
    );
}

#[test]
fn an_item_is_greyed_where_quick_open_hides_its_command() {
    let mut harness = Harness::new();
    harness.connect_fake();
    harness.settle();
    // One database: nothing to switch to.
    let app = &harness.app;
    assert!(
        !app.menu_state(Item::Command(Command::SwitchDatabase))
            .enabled
    );
    // A connection is there to close.
    assert!(
        app.menu_state(Item::Command(Command::CloseConnection))
            .enabled
    );
    // No SQL editor in front: nothing to run.
    assert!(!app.menu_state(Item::Command(Command::RunAll)).enabled);
}

#[test]
fn under_a_dialog_only_what_its_keys_reach_stays_enabled() {
    let mut harness = Harness::new();
    harness.connect_fake();
    harness.app.apply(Action::ShowAbout);
    harness.settle();
    let app = &harness.app;
    assert!(matches!(app.dialog, Some(Dialog::About)));
    // The text sizes' keys work under a dialog.
    assert!(app.menu_state(Item::Command(Command::LargerText)).enabled);
    // A new SQL editor's do not.
    assert!(!app.menu_state(Item::Command(Command::NewSqlTab)).enabled);
    // About, Quick Open and the Edit items stay enabled: their actions
    // and egui decide.
    assert!(app.menu_state(Item::About).enabled);
    assert!(app.menu_state(Item::QuickOpen).enabled);
    assert!(app.menu_state(Item::Edit(EditItem::Copy)).enabled);
}

#[test]
fn settings_stays_enabled_under_the_shortcuts_dialog() {
    let mut harness = Harness::new();
    harness.app.apply(Action::ShowHelp);
    harness.settle();
    assert!(
        harness
            .app
            .menu_state(Item::Command(Command::Settings))
            .enabled
    );
    harness.app.choose(Item::Command(Command::Settings));
    harness.settle();
    assert!(matches!(harness.app.dialog, Some(Dialog::Settings(_))));
}

#[test]
fn a_toggle_says_what_a_click_does() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.settle();
    let sidebar = Item::Command(Command::ToggleSidebar);
    assert_eq!(harness.app.menu_state(sidebar).title, "Hide Sidebar");
    harness.app.apply(Action::ToggleSidebar(tab));
    harness.settle();
    assert_eq!(harness.app.menu_state(sidebar).title, "Show Sidebar");
    // Greyed, a toggle shows its first title.
    let mode = Item::Command(Command::ToggleSqlMode);
    assert!(!harness.app.menu_state(mode).enabled);
    assert_eq!(harness.app.menu_state(mode).title, "Make Tab Read-Only");
}

#[test]
fn a_clicked_command_does_what_quick_open_s_row_does() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.settle();
    harness.app.actions.clear();
    harness.app.choose(Item::Command(Command::NewSqlTab));
    // Quick open's row for it pushes `NewSqlTab` for the window in front
    // (`run_of`). `Action` derives only `Debug`, so match it.
    assert!(
        matches!(harness.app.actions.as_slice(), [Action::NewSqlTab(t)] if *t == tab),
        "{:?}",
        harness.app.actions
    );
    // A command that does not apply pushes nothing.
    harness.app.choose(Item::Command(Command::SaveChanges));
    assert_eq!(harness.app.actions.len(), 1);
}
