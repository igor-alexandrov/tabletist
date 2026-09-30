//! Tabletist: a native database client.

pub mod app;
pub mod backend;
pub mod connections;
pub mod entrypoint;
pub mod env;
pub mod i18n;
pub mod known_hosts;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod model;
pub mod paths;
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
