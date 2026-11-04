//! Tabletist: a native database client.
//!
//! The interface, the reducer and what starts the window. What they stand
//! on is `tabletist_core`'s, re-exported here under its own names, so
//! `crate::model` and the rest mean what they did.

pub mod app;
pub mod entrypoint;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod menu;
#[cfg(test)]
mod promo;
#[cfg(all(test, feature = "shots"))]
mod shots;
pub mod style;
#[cfg(test)]
pub mod testing;
pub mod typography;
pub mod ui;

#[cfg(test)]
pub use tabletist_core::fixtures;
pub use tabletist_core::{
    agents_shown, backend, clock, completion, connections, dates, desktop_theme, edit, endpoint,
    env, export, font_files, format, history, i18n, json, key_picker, keymap, known_hosts, labels,
    managed, model, paths, recents, relay, review, search, secrets, selection, session, settings,
    theme, update, util, vim, zones,
};
