//! The About item of an app menu built the way winit builds it, chosen the
//! way a click chooses it. No test harness: AppKit's menus belong to the main
//! thread, so this runs as a plain `main` (on macOS; nothing elsewhere).

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
    use tabletist_appkit::AboutItem;

    let mtm = MainThreadMarker::new().expect("a test without a harness runs on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    assert!(
        AboutItem::take_over("About Tabletist", || {}).is_none(),
        "there is no menu yet"
    );

    // The menu bar, the app menu in it, and About and Quit in that.
    let bar = NSMenu::new(mtm);
    let holder = NSMenuItem::new(mtm);
    bar.addItem(&holder);
    let menu = NSMenu::new(mtm);
    let about = NSMenuItem::new(mtm);
    about.setTitle(ns_string!("About tabletist"));
    let quit = NSMenuItem::new(mtm);
    quit.setTitle(ns_string!("Quit tabletist"));
    // SAFETY: NSApplication implements both actions.
    unsafe {
        about.setAction(Some(sel!(orderFrontStandardAboutPanel:)));
        quit.setAction(Some(sel!(terminate:)));
    }
    menu.addItem(&about);
    menu.addItem(&quit);
    holder.setSubmenu(Some(&menu));
    app.setMainMenu(Some(&bar));

    let chosen = Rc::new(Cell::new(0));
    let item = AboutItem::take_over("About Tabletist", {
        let chosen = chosen.clone();
        move || chosen.set(chosen.get() + 1)
    })
    .expect("the About item");
    assert_eq!(about.title().to_string(), "About Tabletist");
    assert_eq!(about.action(), Some(sel!(showAbout:)));
    assert!(about.target().is_some(), "About has a target");
    assert_eq!(quit.action(), Some(sel!(terminate:)), "Quit is left alone");
    assert!(quit.target().is_none(), "Quit is left alone");
    assert_eq!(chosen.get(), 0, "nothing is chosen yet");

    // What a click on the item does.
    menu.performActionForItemAtIndex(0);
    assert_eq!(chosen.get(), 1, "choosing About calls back");
    menu.performActionForItemAtIndex(0);
    assert_eq!(chosen.get(), 2, "and again");

    // Dropped, the item is AppKit's again.
    drop(item);
    assert_eq!(about.action(), Some(sel!(orderFrontStandardAboutPanel:)));
    assert!(about.target().is_none(), "no dangling target");

    println!("about_menu: ok");
}
