//! The AppKit calls Tabletist cannot make without `unsafe` code.
//!
//! The rest of the workspace forbids `unsafe`, so the few calls that need it
//! live here behind an API that does not. Empty off macOS.

#[cfg(target_os = "macos")]
mod about;

#[cfg(target_os = "macos")]
mod settings;

#[cfg(target_os = "macos")]
mod target;

#[cfg(target_os = "macos")]
pub use about::AboutItem;

#[cfg(target_os = "macos")]
pub use settings::SettingsItem;
