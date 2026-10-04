//! The app menu's About item.
//!
//! winit builds the app menu and gives its About item AppKit's own action,
//! `orderFrontStandardAboutPanel:`. That panel is filled from the bundle's
//! Info.plist, so a binary run outside Tabletist.app gets a folder icon, the
//! process name and no version. [`AboutItem::take_over`] sends the item to
//! an object of ours instead, so the app can show its own About dialog.

// Wiring a target and an action are messages the compiler cannot check.
#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSApplication, NSMenuItem};
use objc2_foundation::NSString;

use crate::target::MenuTarget;

/// The app menu's About item while it calls back instead of opening AppKit's
/// panel. Dropping it hands the item back to AppKit.
pub struct AboutItem {
    item: Retained<NSMenuItem>,
    /// A menu item does not keep its target alive.
    target: Retained<MenuTarget>,
}

impl AboutItem {
    /// Finds the About item in the app menu, titles it `title` and has it
    /// call `chosen` (on the main thread) when it is chosen.
    ///
    /// `None` off the main thread, before the menu exists, and when the menu
    /// has no item that opens AppKit's About panel.
    pub fn take_over(title: &str, chosen: impl Fn() + 'static) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let bar = NSApplication::sharedApplication(mtm).mainMenu()?;
        // The app menu is the first one in the menu bar.
        let menu = bar.itemArray().firstObject()?.submenu()?;
        let item = menu
            .itemArray()
            .to_vec()
            .into_iter()
            .find(|item| item.action() == Some(sel!(orderFrontStandardAboutPanel:)))?;
        let target = MenuTarget::new(mtm, Box::new(chosen));
        // SAFETY: `target` implements `menuItemChosen:` as an action, and it
        // lives as long as the item points at it: `Drop` takes it back.
        unsafe {
            item.setTarget(Some(&target));
            item.setAction(Some(sel!(menuItemChosen:)));
        }
        item.setTitle(&NSString::from_str(title));
        Some(Self { item, target })
    }
}

impl Drop for AboutItem {
    fn drop(&mut self) {
        let ours = self.item.target().is_some_and(|target| {
            Retained::as_ptr(&target).cast() == Retained::as_ptr(&self.target)
        });
        if !ours {
            // Someone else has the item now.
            return;
        }
        // SAFETY: no target and AppKit's own action, which NSApplication
        // implements: the item as winit made it.
        unsafe {
            self.item.setTarget(None);
            self.item
                .setAction(Some(sel!(orderFrontStandardAboutPanel:)));
        }
    }
}
