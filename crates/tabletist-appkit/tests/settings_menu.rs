//! The Settings item in an app menu built the way winit builds it, chosen
//! the way a click chooses it. No test harness: AppKit's menus belong to the
//! main thread, so this runs as a plain `main` (on macOS; nothing elsewhere).

// Building winit's menu takes the same unchecked calls winit makes.
#![allow(unsafe_code)]

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use std::cell::Cell;
    use std::rc::Rc;

    use objc2::{MainThreadMarker, sel};
    use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
    use objc2_foundation::ns_string;
    use tabletist_appkit::SettingsItem;

    let mtm = MainThreadMarker::new().expect("a test without a harness runs on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    assert!(
        SettingsItem::insert("Settings…", || {}).is_none(),
        "there is no menu yet"
    );

    // The menu bar and the app menu in it, in winit's order and shorter:
    // About, a separator, Hide, a separator, Quit. (winit has Services,
    // Hide Others and Show All beside Hide.)
    let bar = NSMenu::new(mtm);
    let holder = NSMenuItem::new(mtm);
    bar.addItem(&holder);
    let menu = NSMenu::new(mtm);
    let about = NSMenuItem::new(mtm);
    about.setTitle(ns_string!("About tabletist"));
    let hide = NSMenuItem::new(mtm);
    hide.setTitle(ns_string!("Hide tabletist"));
    let quit = NSMenuItem::new(mtm);
    quit.setTitle(ns_string!("Quit tabletist"));
    // SAFETY: NSApplication implements the three actions.
    unsafe {
        about.setAction(Some(sel!(orderFrontStandardAboutPanel:)));
        hide.setAction(Some(sel!(hide:)));
        quit.setAction(Some(sel!(terminate:)));
    }
    menu.addItem(&about);
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    menu.addItem(&hide);
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    menu.addItem(&quit);
    holder.setSubmenu(Some(&menu));

    // A menu with no separator has no place for the item.
    let bare = NSMenu::new(mtm);
    let bare_holder = NSMenuItem::new(mtm);
    bare_holder.setSubmenu(Some(&bare));
    let bare_bar = NSMenu::new(mtm);
    bare_bar.addItem(&bare_holder);
    app.setMainMenu(Some(&bare_bar));
    assert!(
        SettingsItem::insert("Settings…", || {}).is_none(),
        "no separator to put it after"
    );

    app.setMainMenu(Some(&bar));
    let before = menu.numberOfItems();
    let chosen = Rc::new(Cell::new(0));
    let item = SettingsItem::insert("Settings…", {
        let chosen = chosen.clone();
        move || chosen.set(chosen.get() + 1)
    })
    .expect("the Settings item");

    // After About's separator, with a separator of its own after it.
    assert_eq!(menu.numberOfItems(), before + 2);
    // The menu as it reads, a separator as `-`: what a separator's title
    // is, AppKit does not say.
    let titles = |menu: &NSMenu| -> Vec<String> {
        let items = menu.itemArray().to_vec();
        let title = |item: &NSMenuItem| match item.isSeparatorItem() {
            true => "-".to_owned(),
            false => item.title().to_string(),
        };
        items.iter().map(|item| title(item)).collect()
    };
    assert_eq!(
        titles(&menu),
        [
            "About tabletist",
            "-",
            "Settings…",
            "-",
            "Hide tabletist",
            "-",
            "Quit tabletist"
        ]
    );
    let settings = menu.itemAtIndex(2).expect("the item");
    assert_eq!(settings.keyEquivalent().to_string(), ",");
    assert_eq!(settings.action(), Some(sel!(menuItemChosen:)));
    assert!(settings.target().is_some(), "Settings has a target");
    assert!(
        menu.itemAtIndex(3)
            .expect("its separator")
            .isSeparatorItem()
    );
    assert_eq!(about.action(), Some(sel!(orderFrontStandardAboutPanel:)));
    assert_eq!(quit.action(), Some(sel!(terminate:)), "Quit is left alone");
    assert_eq!(chosen.get(), 0, "nothing is chosen yet");

    // What a click on the item does.
    menu.performActionForItemAtIndex(2);
    assert_eq!(chosen.get(), 1, "choosing Settings calls back");
    menu.performActionForItemAtIndex(2);
    assert_eq!(chosen.get(), 2, "and again");

    // Dropped, the menu is as winit made it, and the item, which this
    // test still holds, calls nothing.
    drop(item);
    assert!(settings.target().is_none(), "no dangling target");
    assert_eq!(menu.numberOfItems(), before);
    assert_eq!(
        titles(&menu),
        [
            "About tabletist",
            "-",
            "Hide tabletist",
            "-",
            "Quit tabletist"
        ]
    );

    println!("settings_menu: ok");
}
