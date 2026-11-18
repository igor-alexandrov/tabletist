//! Bundled gettext catalogs. English is the source language and, for now, the
//! only one. Every user-facing string goes through `gettext` so translations
//! can be added later without touching the views.

pub use fastframe_i18n::{gettext, ngettext, pgettext};

include!(concat!(env!("OUT_DIR"), "/catalogs.rs"));

/// `count` with its noun: "1 change", "3 changes".
pub fn counted(locale: Locale, count: usize, one: &'static str, many: &'static str) -> String {
    let plural = u32::try_from(count).unwrap_or(u32::MAX);
    format!("{count} {}", ngettext(locale, one, many, plural))
}

/// `text` either side of its `placeholder`: what is before it and what is
/// after, for a phrase whose placeholder is drawn in a style of its own. A
/// text that lost the placeholder (a translation's slip) is all before.
pub fn around<'a>(text: &'a str, placeholder: &str) -> (&'a str, &'a str) {
    text.split_once(placeholder).unwrap_or((text, ""))
}

/// `text` with each of its placeholders (`{table}`) put in from `values`,
/// all in one pass: a value that holds a placeholder's name (a table called
/// `{columns}`) is put in as it is, never filled itself.
pub fn fill(text: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let at = &rest[start..];
        match values.iter().find(|(name, _)| at.starts_with(name)) {
            Some((name, value)) => {
                out.push_str(value);
                rest = &at[name.len()..];
            }
            None => {
                out.push('{');
                rest = &at[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

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
