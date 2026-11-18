//! Bundled gettext catalogs. English is the source language; every other
//! language the app ships has a catalog in `assets/i18n` (`ru.po`), which
//! the build compiles. Every user-facing string goes through `gettext`, and
//! a string a catalog lacks shows in English.

pub use fastframe_i18n::{LanguageTag, gettext, ngettext, pgettext};

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

/// A language the interface is drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Locale {
    #[default]
    English,
    Russian,
}

impl Locale {
    /// Every language the app ships, in the order the language menu lists
    /// them.
    pub const ALL: [Locale; 2] = [Self::English, Self::Russian];

    /// The tag the settings file writes: `en`, `ru`.
    pub fn tag(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Russian => "ru",
        }
    }

    /// The language's name in itself, as the menu lists it: never
    /// translated, so a person finds theirs whatever the window shows.
    pub fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Russian => "Русский",
        }
    }

    /// The language `tag` names, where the app ships it: its language
    /// subtag alone decides (`ru-RU`, `ru_UA.UTF-8` are Russian).
    pub fn from_tag(tag: &LanguageTag) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|locale| locale.tag() == tag.language)
    }
}

impl fastframe_i18n::Locale for Locale {
    fn catalog(self) -> Option<&'static dyn fastframe_i18n::Translator> {
        match self {
            Self::English => None,
            Self::Russian => Some(&ru::Translator),
        }
    }
}

/// The system's first language, as the system lists them for the app:
/// the one "System" follows. Only the first counts: a later one never
/// stands in for it. `None` when the system lists none, or its first is
/// no language: one that does not parse, or the C locale (`C`, `POSIX`,
/// `C.UTF-8`), which sets none.
pub fn first_system_language() -> Option<LanguageTag> {
    fastframe_i18n::system_languages()
        .first()
        .and_then(|tag| language_of(tag))
}

/// The language `tag` names, as [`first_system_language`] reads it: `None`
/// for the C locale and for what does not parse.
pub fn language_of(tag: &str) -> Option<LanguageTag> {
    LanguageTag::parse(tag).filter(|tag| !matches!(tag.language.as_str(), "c" | "posix"))
}

/// A language's name in itself, for the line that says it is not
/// translated yet: `Français`, `Deutsch`. `None` for a language not in the
/// list, which the line names by its tag.
pub fn endonym(tag: &LanguageTag) -> Option<&'static str> {
    let name = match (tag.language.as_str(), tag.region.as_deref()) {
        ("pt", Some("br")) => "Português (Brasil)",
        // The script says it where it is written; else the region does,
        // Windows' legacy `zh-CHT` and `zh-CHS` among them.
        ("zh", _) if tag.script.as_deref() == Some("hant") => "繁體中文",
        ("zh", _) if tag.script.as_deref() == Some("hans") => "简体中文",
        ("zh", Some("tw" | "hk" | "mo" | "cht")) => "繁體中文",
        ("zh", _) => "简体中文",
        ("ar", _) => "العربية",
        ("bg", _) => "Български",
        ("ca", _) => "Català",
        ("cs", _) => "Čeština",
        ("da", _) => "Dansk",
        ("de", _) => "Deutsch",
        ("el", _) => "Ελληνικά",
        ("en", _) => "English",
        ("es", _) => "Español",
        ("et", _) => "Eesti",
        ("fa", _) => "فارسی",
        ("fi", _) => "Suomi",
        ("fr", _) => "Français",
        ("he", _) => "עברית",
        ("hi", _) => "हिन्दी",
        ("hr", _) => "Hrvatski",
        ("hu", _) => "Magyar",
        ("id", _) => "Bahasa Indonesia",
        ("it", _) => "Italiano",
        ("ja", _) => "日本語",
        ("ka", _) => "ქართული",
        ("kk", _) => "Қазақ тілі",
        ("ko", _) => "한국어",
        ("lt", _) => "Lietuvių",
        ("lv", _) => "Latviešu",
        ("ms", _) => "Bahasa Melayu",
        ("nb" | "no" | "nn", _) => "Norsk",
        ("nl", _) => "Nederlands",
        ("pl", _) => "Polski",
        ("pt", _) => "Português",
        ("ro", _) => "Română",
        ("ru", _) => "Русский",
        ("sk", _) => "Slovenčina",
        ("sl", _) => "Slovenščina",
        ("sr", _) => "Српски",
        ("sv", _) => "Svenska",
        ("th", _) => "ไทย",
        ("tr", _) => "Türkçe",
        ("uk", _) => "Українська",
        ("vi", _) => "Tiếng Việt",
        _ => return None,
    };
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(text: &str) -> LanguageTag {
        LanguageTag::parse(text).expect("a tag")
    }

    #[test]
    fn a_tag_names_a_shipped_language_by_its_language_alone() {
        assert_eq!(Locale::from_tag(&tag("ru")), Some(Locale::Russian));
        assert_eq!(Locale::from_tag(&tag("ru-RU")), Some(Locale::Russian));
        assert_eq!(Locale::from_tag(&tag("ru_UA.UTF-8")), Some(Locale::Russian));
        assert_eq!(Locale::from_tag(&tag("en_GB.UTF-8")), Some(Locale::English));
        assert_eq!(Locale::from_tag(&tag("fr-FR")), None);
        assert_eq!(Locale::from_tag(&tag("C")), None);
    }

    #[test]
    fn russian_has_a_catalog_and_english_none() {
        use fastframe_i18n::Locale as _;
        assert!(Locale::English.catalog().is_none());
        assert!(Locale::Russian.catalog().is_some());
        assert_eq!(gettext(Locale::Russian, "Settings"), "Настройки");
        assert_eq!(gettext(Locale::English, "Settings"), "Settings");
    }

    #[test]
    fn russian_plurals_take_three_forms() {
        let rows = |count| ngettext(Locale::Russian, "{n} row", "{n} rows", count);
        assert_eq!(rows(1), "{n} строка");
        assert_eq!(rows(3), "{n} строки");
        assert_eq!(rows(5), "{n} строк");
        assert_eq!(rows(21), "{n} строка");
        assert_eq!(rows(11), "{n} строк");
        assert_eq!(
            ngettext(Locale::English, "{n} row", "{n} rows", 1),
            "{n} row"
        );
    }

    /// Each entry of a catalog's text: its msgid with its plural, and its
    /// translations. Enough of the PO format for the catalogs here: a
    /// field's strings on its line and the lines after it.
    fn entries(text: &str) -> Vec<(Vec<String>, Vec<String>)> {
        let mut entries = Vec::new();
        let (mut ids, mut strs): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
        let mut field: Option<bool> = None;
        let quoted = |line: &str| -> String {
            let inner = line.trim();
            let inner = inner
                .strip_prefix('"')
                .and_then(|rest| rest.strip_suffix('"'));
            inner.unwrap_or_default().replace("\\\"", "\"")
        };
        for line in text.lines() {
            let line = line.trim_end();
            if line.starts_with("msgid") {
                if field == Some(false) && line.starts_with("msgid ") {
                    entries.push((std::mem::take(&mut ids), std::mem::take(&mut strs)));
                }
                field = Some(true);
                let (_, rest) = line.split_once(' ').unwrap_or_default();
                ids.push(quoted(rest));
            } else if line.starts_with("msgstr") {
                field = Some(false);
                let (_, rest) = line.split_once(' ').unwrap_or_default();
                strs.push(quoted(rest));
            } else if line.starts_with('"') {
                let target = if field == Some(true) {
                    &mut ids
                } else {
                    &mut strs
                };
                if let Some(last) = target.last_mut() {
                    last.push_str(&quoted(line));
                }
            }
        }
        entries.push((ids, strs));
        entries
    }

    /// The `{…}` placeholders of `text`, sorted.
    fn placeholders(text: &str) -> Vec<&str> {
        let mut found: Vec<&str> = text
            .match_indices('{')
            .filter_map(|(start, _)| {
                let end = text[start..].find('}')?;
                Some(&text[start..=start + end])
            })
            .collect();
        found.sort_unstable();
        found
    }

    #[test]
    fn every_russian_translation_keeps_its_placeholders() {
        let text = include_str!("../../../assets/i18n/ru.po");
        let entries = entries(text);
        assert!(entries.len() > 600, "{} entries", entries.len());
        for (ids, strs) in entries {
            let Some(id) = ids.first().filter(|id| !id.is_empty()) else {
                continue;
            };
            let wanted = placeholders(id);
            for translated in strs.iter().filter(|text| !text.is_empty()) {
                assert_eq!(placeholders(translated), wanted, "{id:?} → {translated:?}");
            }
        }
    }

    #[test]
    fn the_c_locale_is_no_language() {
        for none in ["C", "POSIX", "C.UTF-8", "c", ""] {
            assert_eq!(language_of(none), None, "{none:?}");
        }
        assert_eq!(language_of("ru_RU.UTF-8"), Some(tag("ru-RU")));
    }

    #[test]
    fn placeholders_are_filled_in_one_pass() {
        let filled = fill(
            "also clears {columns} in {table} rows",
            &[("{table}", "{columns}"), ("{columns}", "a")],
        );
        assert_eq!(filled, "also clears a in {columns} rows");
        assert_eq!(fill("{n} of {total} {", &[("{n}", "2")]), "2 of {total} {");
    }

    #[test]
    fn a_language_is_named_in_itself() {
        assert_eq!(endonym(&tag("fr_FR.UTF-8")), Some("Français"));
        assert_eq!(endonym(&tag("de")), Some("Deutsch"));
        assert_eq!(endonym(&tag("pt-BR")), Some("Português (Brasil)"));
        assert_eq!(endonym(&tag("zh-Hant-TW")), Some("繁體中文"));
        assert_eq!(endonym(&tag("zh-CN")), Some("简体中文"));
        assert_eq!(endonym(&tag("zh-Hans-TW")), Some("简体中文"));
        assert_eq!(endonym(&tag("zh-CHT")), Some("繁體中文"));
        assert_eq!(endonym(&tag("zh-CHS")), Some("简体中文"));
        assert_eq!(endonym(&tag("xx")), None);
    }
}
