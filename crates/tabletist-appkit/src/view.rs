//! A View menu, which winit's menu bar does not have, with one item.

// Giving a menu item a target and an action are messages the compiler
// cannot check.
#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, sel};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::{NSString, ns_string};

use crate::target::MenuTarget;

/// An item of the View menu, which is made after the app menu where the
/// menu bar has none. Dropping it takes the item out, and the menu too
/// where it made it.
pub struct ViewItem {
    menu: Retained<NSMenu>,
    item: Retained<NSMenuItem>,
    /// The menu bar's item that holds the View menu, where this made it.
    holder: Option<(Retained<NSMenu>, Retained<NSMenuItem>)>,
    /// A menu item does not keep its target alive.
    _target: Retained<MenuTarget>,
}

impl ViewItem {
    /// Puts an item titled `title` in the menu bar's menu titled `menu`
    /// (making it after the app menu where there is none). It calls
    /// `chosen` (on the main thread) when it is chosen. It has no key
    /// equivalent: AppKit would take the key before the window sees it,
    /// and the app's own keys say when it holds.
    ///
    /// `None` off the main thread and before the menu bar exists.
    pub fn insert(menu: &str, title: &str, chosen: impl Fn() + 'static) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let bar = NSApplication::sharedApplication(mtm).mainMenu()?;
        let named = NSString::from_str(menu);
        let found = bar
            .itemArray()
            .to_vec()
            .iter()
            .filter_map(|item| item.submenu())
            .find(|submenu| submenu.title().isEqualToString(&named));
        let (view, holder) = match found {
            Some(view) => (view, None),
            None => {
                let view = NSMenu::initWithTitle(NSMenu::alloc(mtm), &named);
                let holder = NSMenuItem::new(mtm);
                holder.setTitle(&named);
                holder.setSubmenu(Some(&view));
                // After the app menu, the first one in the menu bar.
                let at = isize::from(bar.numberOfItems() > 0);
                bar.insertItem_atIndex(&holder, at);
                (view, Some((bar, holder)))
            }
        };
        let target = MenuTarget::new(mtm, Box::new(chosen));
        // SAFETY: `target` implements `menuItemChosen:` as an action, and
        // it lives for as long as the item is in the menu: `Drop` takes
        // the item out and its target away before the target goes.
        let item = unsafe {
            let item = NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(title),
                Some(sel!(menuItemChosen:)),
                ns_string!(""),
            );
            item.setTarget(Some(&target));
            item
        };
        view.addItem(&item);
        Some(Self {
            menu: view,
            item,
            holder,
            _target: target,
        })
    }
}

impl Drop for ViewItem {
    fn drop(&mut self) {
        // Still where it was put: someone else may have rebuilt the menu.
        if self.menu.indexOfItem(&self.item) >= 0 {
            self.menu.removeItem(&self.item);
        }
        if let Some((bar, holder)) = &self.holder
            && bar.indexOfItem(holder) >= 0
        {
            bar.removeItem(holder);
        }
        // Whoever still holds the item holds one that calls nothing.
        // SAFETY: an item may have no target.
        unsafe { self.item.setTarget(None) };
    }
}
