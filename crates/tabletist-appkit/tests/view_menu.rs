//! The View menu's item in a menu bar built the way winit builds it, chosen
//! the way a click chooses it. No test harness: AppKit's menus belong to the
//! main thread, so this runs as a plain `main` (on macOS; nothing elsewhere).

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use std::cell::Cell;
    use std::rc::Rc;

    use objc2::{MainThreadMarker, MainThreadOnly, sel};
    use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
    use objc2_foundation::ns_string;
    use tabletist_appkit::ViewItem;

    let mtm = MainThreadMarker::new().expect("a test without a harness runs on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    assert!(
        ViewItem::insert("View", "History", || {}).is_none(),
        "there is no menu bar yet"
    );

    // The menu bar as winit makes it, shorter: the app menu and Window.
    let bar = NSMenu::new(mtm);
    for title in [ns_string!("tabletist"), ns_string!("Window")] {
        let holder = NSMenuItem::new(mtm);
        holder.setTitle(title);
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), title);
        holder.setSubmenu(Some(&menu));
        bar.addItem(&holder);
    }
    app.setMainMenu(Some(&bar));
    let titles = |menu: &NSMenu| -> Vec<String> {
        let items = menu.itemArray().to_vec();
        items.iter().map(|item| item.title().to_string()).collect()
    };

    let chosen = Rc::new(Cell::new(0));
    let item = ViewItem::insert("View", "History", {
        let chosen = chosen.clone();
        move || chosen.set(chosen.get() + 1)
    })
    .expect("the History item");
    // A View menu after the app menu, with the item.
    assert_eq!(titles(&bar), ["tabletist", "View", "Window"]);
    let view = bar
        .itemAtIndex(1)
        .and_then(|holder| holder.submenu())
        .expect("the View menu");
    assert_eq!(titles(&view), ["History"]);
    let history = view.itemAtIndex(0).expect("the item");
    assert_eq!(history.action(), Some(sel!(menuItemChosen:)));
    assert!(history.target().is_some(), "History has a target");
    // No key equivalent: AppKit would take the key from the window.
    assert_eq!(history.keyEquivalent().to_string(), "");
    assert_eq!(chosen.get(), 0, "nothing is chosen yet");
    view.performActionForItemAtIndex(0);
    assert_eq!(chosen.get(), 1, "choosing History calls back");

    // A second item goes into the same menu.
    let other = ViewItem::insert("View", "Other", || {}).expect("a second item");
    assert_eq!(titles(&bar), ["tabletist", "View", "Window"]);
    assert_eq!(titles(&view), ["History", "Other"]);
    drop(other);
    assert_eq!(titles(&view), ["History"]);

    // Dropped, the menu bar is as winit made it, and the item, which this
    // test still holds, calls nothing.
    drop(item);
    assert!(history.target().is_none(), "no dangling target");
    assert_eq!(titles(&bar), ["tabletist", "Window"]);

    println!("view_menu: ok");
}
