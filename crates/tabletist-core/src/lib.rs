//! Tabletist beneath its interface: the app's state and its edits, the
//! backend, the settings, the keymap, the theme's data and the text they
//! write. The interface, the reducer and the entrypoint are the
//! `tabletist` crate's, which stands on this one and re-exports each
//! module at its own root; nothing here names them.

pub mod agents_shown;
pub mod backend;
pub mod clock;
pub mod completion;
pub mod connections;
pub mod dates;
pub mod desktop_theme;
pub mod edit;
pub mod endpoint;
pub mod env;
#[cfg(any(test, feature = "testing"))]
pub mod fixtures;
pub mod font_files;
pub mod format;
pub mod history;
pub mod i18n;
pub mod json;
pub mod key_picker;
pub mod keymap;
pub mod known_hosts;
pub mod labels;
pub mod managed;
pub mod model;
pub mod paths;
pub mod recents;
pub mod relay;
pub mod review;
pub mod search;
pub mod secrets;
pub mod session;
pub mod settings;
pub mod theme;
pub mod update;
pub mod util;
pub mod vim;
pub mod zones;
