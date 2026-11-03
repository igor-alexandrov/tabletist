//! Bundled gettext catalogs. English is the source language and, for now, the
//! only one. Every user-facing string goes through `gettext` so translations
//! can be added later without touching the views.

pub use fastframe_i18n::{gettext, ngettext, pgettext};

include!(concat!(env!("OUT_DIR"), "/catalogs.rs"));

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Locale {
    #[default]
    English,
}

impl fastframe_i18n::Locale for Locale {
    fn catalog(self) -> Option<&'static dyn fastframe_i18n::Translator> {
        match self {
            Self::English => None,
        }
    }
}
