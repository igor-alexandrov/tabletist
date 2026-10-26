//! Tabletist: a native database client.

pub mod agents_shown;
pub mod app;
pub mod backend;
pub mod clock;
pub mod completion;
pub mod connections;
pub mod dates;
pub mod desktop_theme;
pub mod edit;
pub mod endpoint;
pub mod entrypoint;
pub mod env;
pub mod history;
pub mod i18n;
pub mod key_picker;
pub mod keymap;
pub mod known_hosts;
pub mod labels;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod managed;
pub mod menu;
pub mod model;
pub mod paths;
pub mod recents;
pub mod relay;
pub mod review;
pub mod search;
pub mod secrets;
pub mod session;
pub mod settings;
#[cfg(all(test, feature = "shots"))]
mod shots;
#[cfg(test)]
pub mod testing;
pub mod theme;
pub mod typography;
pub mod ui;
pub mod util;
pub mod vim;
pub mod zones;
