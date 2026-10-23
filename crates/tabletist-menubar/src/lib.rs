//! Tabletist's menu bar on macOS and Windows, built with muda from a
//! description the app gives: menus of items, each with the app's id and
//! key, and the platform's own items. Off those platforms there is none,
//! and attaching one gives `None`.

mod keys;

pub use keys::{KeyDown, Shortcut, Via, via_of};
