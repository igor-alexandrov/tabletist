//! The app menu's Settings item, which winit's menu does not have.

// Giving a menu item a target and an action are messages the compiler
// cannot check.
#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::{NSString, ns_string};

use crate::target::MenuTarget;

/// "Settings…" in the app menu, with `⌘,`, and the separator after it.
/// Dropping it takes both out again.
pub struct SettingsItem {
    menu: Retained<NSMenu>,
    item: Retained<NSMenuItem>,
    separator: Retained<NSMenuItem>,
    /// A menu item does not keep its target alive.
    _target: Retained<MenuTarget>,
}

impl SettingsItem {
    /// Puts the item, titled `title`, after the separator that follows
    /// About: the place the platform gives it. It calls `chosen` (on the
    /// main thread) when it is chosen, by a click or by `⌘,`.
    ///
    /// `None` off the main thread, before the menu exists, and when the app
    /// menu has no separator to put it after.
    pub fn insert(title: &str, chosen: impl Fn() + 'static) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let bar = NSApplication::sharedApplication(mtm).mainMenu()?;
        // The app menu is the first one in the menu bar.
        let menu = bar.itemArray().firstObject()?.submenu()?;
        let after = menu
            .itemArray()
            .to_vec()
            .iter()
            .position(|item| item.isSeparatorItem())?;
        let target = MenuTarget::new(mtm, Box::new(chosen));
        // SAFETY: `target` implements `menuItemChosen:` as an action, and
        // it lives as long as the item, which `Drop` takes out of the menu.
        let item = unsafe {
            let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(title),
                Some(sel!(menuItemChosen:)),
                ns_string!(","),
            );
            item.setTarget(Some(&target));
            item
        };
        let separator = NSMenuItem::separatorItem(mtm);
        let at = isize::try_from(after + 1).ok()?;
        menu.insertItem_atIndex(&item, at);
        menu.insertItem_atIndex(&separator, at + 1);
        Some(Self {
            menu,
            item,
            separator,
            _target: target,
        })
    }
}

impl Drop for SettingsItem {
    fn drop(&mut self) {
        // Still where they were put: someone else may have rebuilt the menu.
        for item in [&self.separator, &self.item] {
            if self.menu.indexOfItem(item) >= 0 {
                self.menu.removeItem(item);
            }
        }
    }
}
