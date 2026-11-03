//! Reading font files from the system: the desktop's monospace through
//! fontconfig ([`desktop_font`]), and on macOS the installed font the
//! settings name for the data through CoreText ([`installed_font`]). The
//! backend reads them on its runtime; the interface draws what was read
//! (`typography`).

pub mod desktop_font;
pub mod installed_font;

use std::sync::Arc;

use egui::FontData;

/// The terminal look's monospace at 400, 500 and 700, or the installed
/// font macOS draws data in at the same.
pub type TerminalFaces = [Arc<FontData>; 3];

/// The font the settings name, read from the system: on macOS the data's,
/// an installed one through CoreText; elsewhere the terminal look's,
/// through fontconfig.
pub fn read_font(asked: Option<&str>) -> desktop_font::Read {
    if cfg!(target_os = "macos") {
        installed_font::read(asked)
    } else {
        desktop_font::read(asked)
    }
}
