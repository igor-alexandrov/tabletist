//! Tabletist's menu bar on macOS and Windows, built with muda from a
//! description the app gives: menus of items, each with the app's id and
//! key, and the platform's own items. Off those platforms there is none,
//! and attaching one gives `None`.

mod keys;

pub use keys::{KeyDown, Shortcut, Via, via_of};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod native;

use std::hash::Hash;

use raw_window_handle::HasWindowHandle;

/// An item the platform answers itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Native {
    Services,
    Hide,
    HideOthers,
    ShowAll,
    Quit,
    Minimize,
    Zoom,
    BringAllToFront,
}

/// What AppKit makes of a menu besides showing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Plain,
    /// AppKit lists the windows here (`windowsMenu`).
    Window,
    /// AppKit adds its search field here (`helpMenu`).
    Help,
}

/// One line of a menu.
pub enum Entry<T> {
    Item {
        id: T,
        title: String,
        key: Option<Shortcut>,
    },
    /// The platform's own item, with its title where the default will not
    /// do ("Quit Tabletist").
    Native(Native, Option<String>),
    Separator,
}

pub struct Menu<T> {
    pub title: String,
    pub role: Role,
    pub entries: Vec<Entry<T>>,
}

/// The menu bar, attached. Dropped, its items are gone with it: on macOS
/// NSApp is left with no main menu, not even winit's.
pub struct MenuBar<T> {
    #[cfg(target_os = "macos")]
    bar: native::Bar<T>,
    #[cfg(not(target_os = "macos"))]
    never: std::convert::Infallible,
    #[cfg(not(target_os = "macos"))]
    _ids: std::marker::PhantomData<T>,
}

impl<T: Copy + Eq + Hash + 'static> MenuBar<T> {
    /// Builds the bar and attaches it: to NSApp on macOS, where `window` is
    /// not read. `wake` runs, on the main thread, after each choice.
    /// `None` off macOS, or if muda refuses an item.
    pub fn attach(
        window: &impl HasWindowHandle,
        menus: Vec<Menu<T>>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Option<Self> {
        #[cfg(target_os = "macos")]
        {
            let window = window.window_handle().ok().map(|handle| handle.as_raw());
            native::Bar::attach(window, menus, wake).map(|bar| Self { bar })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (window, menus, wake);
            None
        }
    }

    /// Builds the bar again from `menus` and puts it in the old one's
    /// place, on the same window. How a key changes: muda cannot take a
    /// key equivalent off an item on macOS (`set_accelerator(None)` does
    /// nothing there), so a key that goes away needs a new item. False if
    /// muda refused it; the old bar stays.
    pub fn rebuild(&mut self, menus: Vec<Menu<T>>) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.bar.rebuild(menus)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = menus;
            match self.never {}
        }
    }

    /// Greys an item or enables it. On macOS it is greyed only while its
    /// menu is open: AppKit takes a disabled item's key equivalent and
    /// performs nothing, so outside an open menu its key reaches the app.
    pub fn set_enabled(&mut self, id: T, enabled: bool) {
        #[cfg(target_os = "macos")]
        self.bar.set_enabled(id, enabled);
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, enabled);
            match self.never {}
        }
    }

    pub fn set_title(&mut self, id: T, title: &str) {
        #[cfg(target_os = "macos")]
        self.bar.set_title(id, title);
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, title);
            match self.never {}
        }
    }

    /// The items chosen since the last call, oldest first, each with how.
    pub fn chosen(&mut self) -> Vec<(T, Via)> {
        #[cfg(target_os = "macos")]
        {
            self.bar.chosen()
        }
        #[cfg(not(target_os = "macos"))]
        match self.never {}
    }
}
