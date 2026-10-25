//! The bar on NSApp, chosen as a click chooses and as a key equivalent
//! does. No test harness: AppKit's menus belong to the main thread, so
//! this runs as a plain `main` (on macOS; nothing elsewhere).

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSApplication, NSEvent, NSEventModifierFlags, NSEventType, NSMenu, NSMenuDelegate,
    };
    use objc2_foundation::{NSPoint, NSString};
    use raw_window_handle::{HandleError, HasWindowHandle, WindowHandle};
    use tabletist_menubar::{Entry, Menu, MenuBar, Native, Role, Shortcut, Via};

    /// AppKit needs no window for the menu bar.
    struct NoWindow;
    impl HasWindowHandle for NoWindow {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            Err(HandleError::Unavailable)
        }
    }

    let mtm = MainThreadMarker::new().expect("a test without a harness runs on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    let cmd = |key| Shortcut {
        key,
        cmd: true,
        ctrl: false,
        alt: false,
        shift: false,
    };
    let menus = vec![
        Menu {
            title: "Tabletist".to_owned(),
            role: Role::Plain,
            entries: vec![
                Entry::Item {
                    id: 1,
                    title: "About Tabletist".to_owned(),
                    key: None,
                },
                Entry::Separator,
                Entry::Native(Native::Quit, Some("Quit Tabletist".to_owned())),
            ],
        },
        Menu {
            title: "View".to_owned(),
            role: Role::Plain,
            entries: vec![
                Entry::Item {
                    id: 2,
                    title: "History".to_owned(),
                    key: Some(cmd(egui::Key::Y)),
                },
                Entry::Item {
                    id: 3,
                    title: "Greyed".to_owned(),
                    key: Some(cmd(egui::Key::J)),
                },
                Entry::Item {
                    id: 4,
                    title: "Previous Tab".to_owned(),
                    key: Some(Shortcut {
                        shift: true,
                        ..cmd(egui::Key::OpenBracket)
                    }),
                },
                Entry::Item {
                    id: 5,
                    title: "Next Connection".to_owned(),
                    key: Some(Shortcut {
                        cmd: false,
                        ctrl: true,
                        ..cmd(egui::Key::Tab)
                    }),
                },
            ],
        },
        Menu {
            title: "Window".to_owned(),
            role: Role::Window,
            entries: vec![],
        },
    ];
    let woken = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut bar = MenuBar::attach(&NoWindow, menus, {
        let woken = woken.clone();
        move || {
            woken.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    })
    .expect("the bar attaches to NSApp");

    let main = app.mainMenu().expect("the bar is the main menu");
    let titles = |menu: &NSMenu| -> Vec<String> {
        menu.itemArray()
            .to_vec()
            .iter()
            .map(|item| item.title().to_string())
            .collect()
    };
    assert_eq!(titles(&main), ["Tabletist", "View", "Window"]);
    let view = main
        .itemAtIndex(1)
        .and_then(|holder| holder.submenu())
        .expect("View");
    assert_eq!(
        titles(&view),
        ["History", "Greyed", "Previous Tab", "Next Connection"]
    );
    let history = view.itemAtIndex(0).expect("History");
    assert_eq!(history.keyEquivalent().to_string(), "y");
    assert!(
        history
            .keyEquivalentModifierMask()
            .contains(NSEventModifierFlags::Command)
    );
    assert!(app.windowsMenu().is_some(), "the Window menu is NSApp's");

    // A click: AppKit's current event is none here.
    view.performActionForItemAtIndex(0);
    assert_eq!(bar.chosen(), [(2, Via::Click)]);
    assert!(
        woken.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "choosing wakes the app"
    );

    // A key equivalent, as a US keyboard sends it. Called directly,
    // `performKeyEquivalent` leaves AppKit's current event empty, so every
    // choice here reads as a click: `via_of` is tested on its own, and a
    // real key's `Via::Key` is for eyes in the running app.
    let key = |chars: &str, flags: NSEventModifierFlags, code: u16| {
        let chars = NSString::from_str(chars);
        NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            NSEventType::KeyDown, NSPoint::new(0.0, 0.0), flags, 0.0, 0, None, &chars, &chars, false, code,
        )
        .expect("a key event")
    };
    assert!(
        main.performKeyEquivalent(&key("y", NSEventModifierFlags::Command, 16)),
        "⌘Y"
    );
    assert_eq!(bar.chosen().len(), 1, "⌘Y chose History");

    // AppKit takes a disabled item's key equivalent and performs nothing,
    // so a greyed item stays enabled for AppKit: its key is chosen, and
    // the app replays it to itself.
    bar.set_enabled(3, false);
    let greyed = view.itemAtIndex(1).expect("Greyed");
    assert!(greyed.isEnabled(), "greyed only while its menu is open");
    assert!(
        main.performKeyEquivalent(&key("j", NSEventModifierFlags::Command, 38)),
        "⌘J"
    );
    assert_eq!(bar.chosen(), [(3, Via::Click)]);

    // Open, the menu shows what the app wants, and a change shows at once.
    let delegate = view.delegate().expect("the View menu's delegate");
    delegate.menuWillOpen(&view);
    assert!(!greyed.isEnabled(), "greyed in the open menu");
    assert!(history.isEnabled());
    bar.set_enabled(3, true);
    assert!(greyed.isEnabled(), "enabled at once in the open menu");
    bar.set_enabled(3, false);
    assert!(!greyed.isEnabled(), "greyed at once in the open menu");
    delegate.menuDidClose(&view);
    assert!(greyed.isEnabled(), "enabled again once the menu closes");

    // ⇧⌘[ has no key equivalent: AppKit would not match the `{` a
    // keyboard sends. ⌃⇥ is one: AppKit does not keep Tab for itself.
    let previous = view.itemAtIndex(2).expect("Previous Tab");
    assert_eq!(previous.keyEquivalent().to_string(), "");
    let shift_cmd = NSEventModifierFlags::Command | NSEventModifierFlags::Shift;
    assert!(!main.performKeyEquivalent(&key("{", shift_cmd, 33)), "⇧⌘[");
    assert!(
        main.performKeyEquivalent(&key("\t", NSEventModifierFlags::Control, 48)),
        "⌃⇥"
    );
    assert_eq!(bar.chosen(), [(5, Via::Click)]);

    // A title changes in place.
    bar.set_title(2, "Show History");
    assert_eq!(history.title().to_string(), "Show History");

    // Rebuilt without History's key: the new bar is the main menu, and
    // ⌘Y is no item's.
    let rebuilt = vec![Menu {
        title: "View".to_owned(),
        role: Role::Plain,
        entries: vec![Entry::Item {
            id: 2,
            title: "History".to_owned(),
            key: None,
        }],
    }];
    assert!(bar.rebuild(rebuilt));
    let main = app.mainMenu().expect("the rebuilt bar is the main menu");
    assert_eq!(titles(&main), ["View"]);
    assert!(
        !main.performKeyEquivalent(&key("y", NSEventModifierFlags::Command, 16)),
        "⌘Y is gone"
    );

    println!("macos_menu: ok");
}
