//! macOS: the tabs share a unified title bar with the window buttons.
//!
//! The window gets an empty toolbar in the compact style, so AppKit makes
//! the title bar tall enough for the tab bar and centres the window buttons
//! in it (the look of Safari's compact tabs and Xcode). Every frame reads
//! back the title bar's height and where the buttons end, so the layout
//! follows the running system (the buttons grew in macOS 26) and
//! fullscreen, where AppKit hides them. Nothing moves the buttons by hand,
//! and nothing here is `unsafe`: the window comes from `NSApplication`.

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSTitlebarSeparatorStyle, NSToolbar, NSWindow, NSWindowButton,
    NSWindowTitleVisibility, NSWindowToolbarStyle,
};

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
}
