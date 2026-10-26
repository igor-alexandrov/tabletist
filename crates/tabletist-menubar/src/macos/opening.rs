//! The delegate that greys a menu's items while it is open. AppKit takes a
//! disabled item's key equivalent and performs nothing, so outside an open
//! menu every item stays enabled and its key reaches the app.

// Defining an Objective-C class is a message the compiler cannot check.
#![allow(unsafe_code)]

use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuDelegate};

use crate::native::MenuItems;

define_class!(
    // SAFETY: NSObject has no rules for its subclasses, and `Opening` does
    // not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Rc<MenuItems>]
    pub(crate) struct Opening;

    // SAFETY: NSObjectProtocol asks nothing of a subclass of NSObject.
    unsafe impl NSObjectProtocol for Opening {}

    // SAFETY: each method takes the menu and returns nothing, as
    // NSMenuDelegate declares it.
    unsafe impl NSMenuDelegate for Opening {
        #[unsafe(method(menuWillOpen:))]
        fn menu_will_open(&self, _menu: &NSMenu) {
            self.ivars().opened();
        }

        #[unsafe(method(menuDidClose:))]
        fn menu_did_close(&self, _menu: &NSMenu) {
            self.ivars().closed();
        }
    }
);

impl Opening {
    fn new(mtm: MainThreadMarker, items: Rc<MenuItems>) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(items);
        // SAFETY: NSObject's `init` takes nothing and returns the object.
        unsafe { msg_send![super(this), init] }
    }
}

/// Gives each menu of NSApp's main menu, in order, the delegate for its
/// items. NSMenu holds its delegate weakly, so the caller keeps them.
/// muda's own delegate stays on the main menu, where it reads it.
pub(crate) fn watch(menus: &[Rc<MenuItems>]) -> Vec<Retained<Opening>> {
    let Some(mtm) = MainThreadMarker::new() else {
        return Vec::new();
    };
    let Some(main) = NSApplication::sharedApplication(mtm).mainMenu() else {
        return Vec::new();
    };
    let watched: Vec<_> = menus
        .iter()
        .enumerate()
        .filter_map(|(index, items)| {
            let menu = main.itemAtIndex(isize::try_from(index).ok()?)?.submenu()?;
            let opening = Opening::new(mtm, items.clone());
            menu.setDelegate(Some(ProtocolObject::from_ref(&*opening)));
            Some(opening)
        })
        .collect();
    if watched.len() != menus.len() {
        log::warn!("a menu has no delegate: its items are never greyed");
    }
    watched
}
