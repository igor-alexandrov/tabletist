//! The menu bar's model, which items are enabled, what a click does, and
//! what a replayed key gives egui. Headless.

use crate::app::RUNNABLE;
use crate::keymap::{Command, EditItem};

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
