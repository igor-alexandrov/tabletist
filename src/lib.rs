//! Tabletist: a native database client.

pub mod app;
pub mod backend;
pub mod completion;
pub mod connections;
pub mod edit;
pub mod entrypoint;
pub mod env;
pub mod i18n;
pub mod key_picker;
pub mod keymap;
pub mod known_hosts;
pub mod labels;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod model;
pub mod paths;
pub mod review;
pub mod secrets;
pub mod settings;
#[cfg(all(test, feature = "shots"))]
mod shots;
#[cfg(test)]
pub mod testing;
pub mod theme;
pub mod typography;
pub mod ui;
pub mod util;
