//! macOS: the connection bar shares a unified title bar with the window
//! buttons.
//!
//! The window gets an empty toolbar in the compact style, so AppKit makes
//! the title bar tall enough for a bar of its own (the look of Safari's
//! compact tabs and Xcode). Every frame reads back the title bar's height
//! and where the buttons end, so the layout follows the running system (the
//! buttons grew in macOS 26) and fullscreen, where AppKit hides them.
//!
//! AppKit centres the buttons in its own title bar, but the bar that leads
//! the window (the connection bar, the picker's header) can be taller, so
//! every frame also moves each button onto that bar's line. AppKit lays the
//! buttons out again on resizes and focus changes, and each of those draws
//! a frame that puts them back. Nothing here is `unsafe`: the window comes
//! from `NSApplication` and the buttons' title bar is their shared ancestor.
//!
//! The app menu's About item opens the app's own About dialog ([`AboutMenu`]),
//! and its Settings item, added the same way, opens the Settings window
//! ([`SettingsMenu`]). Pointing a menu item somewhere does need `unsafe`,
//! which this crate forbids, so that one call lives in `tabletist-appkit`.

use std::cell::Cell;
use std::rc::Rc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSTitlebarSeparatorStyle, NSToolbar, NSWindow, NSWindowButton,
    NSWindowTitleVisibility, NSWindowToolbarStyle,
};
use objc2_foundation::NSPoint;

use crate::app::TitleBar;

/// Space between the zoom button and the first tab.
const GAP: f32 = 12.0;

/// The app window whose title bar the tabs share.
pub struct UnifiedTitleBar {
    window: Retained<NSWindow>,
}

impl UnifiedTitleBar {
    /// Gives the app's window a unified compact title bar with no title and
    /// no separator. `None` until the window exists (or off the main thread).
    pub fn attach() -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let app = NSApplication::sharedApplication(mtm);
        let window = app.mainWindow().or_else(|| app.windows().firstObject())?;
        let toolbar = NSToolbar::new(mtm);
        window.setToolbar(Some(&toolbar));
        window.setToolbarStyle(NSWindowToolbarStyle::UnifiedCompact);
        window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
        window.setTitlebarAppearsTransparent(true);
        window.setTitlebarSeparatorStyle(NSTitlebarSeparatorStyle::None);
        Some(Self { window })
    }

    /// The title bar as the window has it now: zero in fullscreen, where
    /// AppKit hides it.
    pub fn measure(&self) -> TitleBar {
        let window = &self.window;
        let height = (window.frame().size.height - window.contentLayoutRect().size.height) as f32;
        if height < 1.0 {
            return TitleBar::default();
        }
        let inset = window
            .standardWindowButton(NSWindowButton::ZoomButton)
            .map(|zoom| {
                let frame = zoom.frame();
                (frame.origin.x + frame.size.width) as f32 + GAP
            })
            .unwrap_or(78.0);
        TitleBar { height, inset }
    }

    /// Centres the window buttons `line` window points below the window's
    /// top, as far as their title bar reaches (a button outside it would
    /// take no clicks). Nothing moves in fullscreen, where AppKit shows the
    /// buttons in a title bar of its own.
    pub fn place_buttons(&self, line: f32) {
        let window = &self.window;
        if self.measure() == TitleBar::default() {
            return;
        }
        let buttons: Vec<_> = [
            NSWindowButton::CloseButton,
            NSWindowButton::MiniaturizeButton,
            NSWindowButton::ZoomButton,
        ]
        .into_iter()
        .filter_map(|kind| window.standardWindowButton(kind))
        .collect();
        // The view the buttons sit in: the title bar.
        let [first, .., last] = buttons.as_slice() else {
            return;
        };
        let Some(bar) = first.ancestorSharedWithView(last) else {
            return;
        };
        let bounds = bar.bounds();
        // Window coordinates run up from the bottom edge.
        let target = NSPoint::new(0.0, window.frame().size.height - f64::from(line));
        let center = bar.convertPoint_fromView(target, None).y;
        for button in &buttons {
            let frame = button.frame();
            // Only a button directly in the bar has its frame in the bar's
            // coordinates; any other stays where AppKit put it.
            let seen = bar.convertRect_fromView(button.bounds(), Some(button));
            if (seen.origin.x - frame.origin.x).abs() >= 0.5
                || (seen.origin.y - frame.origin.y).abs() >= 0.5
            {
                continue;
            }
            let lowest = bounds.origin.y;
            let highest = (lowest + bounds.size.height - frame.size.height).max(lowest);
            let y = (center - frame.size.height / 2.0)
                .round()
                .clamp(lowest, highest);
            if (y - frame.origin.y).abs() >= 0.5 {
                button.setFrameOrigin(NSPoint::new(frame.origin.x, y));
            }
        }
    }
}

/// The app menu's About item, asking for the app's own About dialog.
///
/// winit points that item at AppKit's About panel, which reads the bundle's
/// Info.plist: a binary run outside Tabletist.app shows a folder icon, the
/// process name and no version there, and even the bundle shows less than
/// the dialog does.
pub struct AboutMenu {
    chosen: Rc<Cell<bool>>,
    /// The item is ours for as long as this lives.
    _item: tabletist_appkit::AboutItem,
}

impl AboutMenu {
    /// Takes the About item over, titled `title`. Choosing it draws a frame,
    /// which finds it with [`AboutMenu::take`]. `None` when the app menu has
    /// no About item (or off the main thread).
    pub fn attach(ctx: &egui::Context, title: &str) -> Option<Self> {
        let chosen = Rc::new(Cell::new(false));
        let item = tabletist_appkit::AboutItem::take_over(title, {
            let chosen = chosen.clone();
            let ctx = ctx.clone();
            move || {
                chosen.set(true);
                ctx.request_repaint();
            }
        })?;
        Some(Self {
            chosen,
            _item: item,
        })
    }

    /// Whether the item was chosen since the last call.
    pub fn take(&self) -> bool {
        self.chosen.replace(false)
    }
}

/// The app menu's Settings item, asking for the Settings window.
///
/// winit's app menu has none, so one is added where the platform puts it.
/// AppKit takes `⌘,` for the item before egui sees the key.
pub struct SettingsMenu {
    chosen: Rc<Cell<bool>>,
    /// The item is in the menu for as long as this lives.
    _item: tabletist_appkit::SettingsItem,
}

impl SettingsMenu {
    /// Adds the item, titled `title`. Choosing it draws a frame, which
    /// finds it with [`SettingsMenu::take`]. `None` when the app menu is
    /// not as winit builds it (or off the main thread).
    pub fn attach(ctx: &egui::Context, title: &str) -> Option<Self> {
        let chosen = Rc::new(Cell::new(false));
        let item = tabletist_appkit::SettingsItem::insert(title, {
            let chosen = chosen.clone();
            let ctx = ctx.clone();
            move || {
                chosen.set(true);
                ctx.request_repaint();
            }
        })?;
        Some(Self {
            chosen,
            _item: item,
        })
    }

    /// Whether the item was chosen since the last call.
    pub fn take(&self) -> bool {
        self.chosen.replace(false)
    }
}
