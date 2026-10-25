//! The CoreText calls Tabletist cannot make without `unsafe` code.
//!
//! The rest of the workspace forbids `unsafe`, so the few calls that need it
//! live here behind an API that does not. Empty off macOS.

#[cfg(target_os = "macos")]
mod fonts;

#[cfg(target_os = "macos")]
pub use fonts::{FontFace, faces_of, monospace_families};
