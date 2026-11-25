//! A Files connection that macOS's privacy settings did not let Tabletist
//! open: what the connection dialog's banner and the window's connect card
//! say of it, and the button that opens System Settings where the folder
//! may be allowed (Privacy & Security, Files and Folders).

use tabletist_db::files::Refused;

use crate::i18n::{Locale, fill, gettext};
use crate::theme::Look;
use crate::ui::banner::{Action, Banner};
use crate::ui::states::Tone;

/// The "not allowed" state's words, made for one frame.
pub struct NotAllowed {
    /// "Can't read ~/Data/bookshop-exports."
    pub lead: String,
    /// "macOS hasn't allowed Tabletist to open this folder."
    pub text: String,
    /// "Open Privacy & Security".
    pub button: String,
    /// What macOS refused: the folder, or one of its files.
    pub what: Refused,
}

impl NotAllowed {
    /// The state the backend's `message` says, in `look` and `locale`:
    /// `None` where it is not macOS's privacy settings that refused.
    pub fn of(message: &str, look: &Look, locale: Locale) -> Option<Self> {
        let (path, what) = tabletist_db::files::refused_by_macos(message)?;
        // The path is put in after the look's case: it keeps its own.
        let lead = fill(
            &look.label(&gettext(locale, "Can't read {path}.")),
            &[("{path}", path)],
        );
        let text = look.label(&match what {
            Refused::Folder => gettext(
                locale,
                "macOS hasn't allowed Tabletist to open this folder.",
            ),
            Refused::File => gettext(locale, "macOS hasn't allowed Tabletist to open this file."),
        });
        let button = look.label(&gettext(locale, "Open Privacy & Security"));
        Some(Self {
            lead,
            text,
            button,
            what,
        })
    }

    /// The danger banner, its one action the button.
    pub fn banner(&self) -> Banner<'_> {
        Banner {
            tone: Tone::Danger,
            lead: &self.lead,
            text: &self.text,
            actions: vec![Action {
                label: &self.button,
                key: None,
                quiet: false,
            }],
            salt: "not-allowed",
        }
    }
}
