//! The interface's language (Settings › General, Language), in both looks:
//! what the row says for the system's first language, what choosing one
//! does to the window and the file, and what Reset to defaults leaves.
//!
//! Only what is shown, said and written is checked. No colour, size or
//! picture of the design is.

use egui::{Key, Modifiers};

use crate::i18n::{LanguageTag, Locale};
use crate::model::{Action, SettingsPane, SettingsRow};
use crate::settings::{Language, OptionId, OptionValue};
use crate::testing::Harness;
use crate::theme::Look;

fn tag(text: &str) -> LanguageTag {
    LanguageTag::parse(text).expect("a tag")
}

/// Settings open on General in `look`, the system's first language
/// `system`, in a window tall enough for every row.
fn general(look: Look, system: Option<&str>) -> Harness {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 2400.0));
    harness.set_look(look);
    harness.app.system_language = system.map(tag);
    // As the app would at its start: the window in the System language.
    let settings = harness.app.settings.clone();
    harness.app.apply_settings(settings);
    harness.app.apply(Action::ShowSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::General));
    harness.settle();
    harness
}

fn painted(harness: &Harness, text: &str) -> bool {
    harness.painted_color(text).is_some()
}

#[test]
fn the_language_is_general_s_first_row_in_both_looks() {
    assert_eq!(
        SettingsPane::General.rows().first(),
        Some(&SettingsRow::Option(OptionId::Language))
    );
    assert_eq!(
        SettingsPane::General.sheet_options().first(),
        Some(&OptionId::Language)
    );
}

#[test]
fn system_says_the_language_it_stands_for() {
    let harness = general(Look::macos(), Some("en-US"));
    assert!(painted(&harness, "System (English)"));
    assert!(painted(
        &harness,
        "Menus, labels and messages; data and SQL stay as they are"
    ));
    assert!(painted(&harness, "Help translate"));
    // In English the label has no English word beside it.
    assert!(painted(&harness, "Language"));
    assert!(!painted(&harness, "Language · Language"));
    // A Russian system: the window is Russian, and System says so.
    let harness = general(Look::macos(), Some("ru-RU"));
    assert_eq!(harness.app.locale, Locale::Russian);
    assert!(painted(&harness, "Системный (Русский)"));
}

#[test]
fn a_system_language_with_no_catalog_reads_english_and_says_so() {
    let harness = general(Look::macos(), Some("fr_FR.UTF-8"));
    assert_eq!(harness.app.locale, Locale::English);
    assert_eq!(harness.app.settings.language, Language::System);
    assert!(painted(&harness, "English"));
    assert!(!painted(&harness, "System (English)"));
    assert!(painted(&harness, "Français is not translated yet"));
    // A language with no name in the list is named by its tag.
    let harness = general(Look::macos(), Some("xx-YY"));
    assert!(painted(&harness, "xx is not translated yet"));
}

#[test]
fn choosing_russian_redraws_the_window_and_writes_the_file() {
    let mut harness = general(Look::macos(), Some("en-US"));
    harness
        .app
        .apply(Action::SetOption(OptionValue::Language(Language::Fixed(
            Locale::Russian,
        ))));
    harness.settle();
    assert_eq!(harness.app.locale, Locale::Russian);
    // The sheet's title, from the catalog. (The panes' names are handed to
    // `gettext` in a variable, which no extractor reads yet.)
    assert!(painted(&harness, "Настройки"));
    // The English word stays beside the label: the way back.
    assert!(painted(&harness, "Язык · Language"));
    assert!(painted(&harness, "Русский"));
    assert!(
        harness
            .app
            .settings_file
            .text
            .contains("[interface]\nlanguage = \"ru\"")
    );
    // And back to English.
    harness
        .app
        .apply(Action::SetOption(OptionValue::Language(Language::Fixed(
            Locale::English,
        ))));
    harness.settle();
    assert_eq!(harness.app.locale, Locale::English);
    assert!(painted(&harness, "Settings"));
}

#[test]
fn reset_to_defaults_leaves_the_language() {
    let mut harness = general(Look::macos(), Some("en-US"));
    harness
        .app
        .apply(Action::SetOption(OptionValue::Language(Language::Fixed(
            Locale::Russian,
        ))));
    harness
        .app
        .apply(Action::SetOption(OptionValue::PageSize(500)));
    harness.app.apply(Action::ResetSettings);
    harness.app.apply(Action::ConfirmResetSettings(true));
    harness.settle();
    assert_eq!(harness.app.settings.page_size, 1000);
    assert_eq!(
        harness.app.settings.language,
        Language::Fixed(Locale::Russian)
    );
    assert_eq!(harness.app.locale, Locale::Russian);
}

/// The Omarchy screen's cursor on the Language row: the first.
fn omarchy(system: Option<&str>) -> Harness {
    general(Look::omarchy(), system)
}

#[test]
fn the_omarchy_row_steps_through_the_languages_and_r_puts_back_system() {
    let mut harness = omarchy(Some("en-US"));
    // On the cursor's row, between its chevrons.
    assert!(painted(&harness, "‹ system (english) ›"));
    assert!(painted(
        &harness,
        "from $LANG · help translate: assets/i18n"
    ));
    harness.press(Key::L, Modifiers::NONE);
    assert_eq!(
        harness.app.settings.language,
        Language::Fixed(Locale::English)
    );
    harness.press(Key::L, Modifiers::NONE);
    assert_eq!(
        harness.app.settings.language,
        Language::Fixed(Locale::Russian)
    );
    assert_eq!(harness.app.locale, Locale::Russian);
    assert!(painted(&harness, "‹ русский ›"));
    // The last: it stays.
    harness.press(Key::L, Modifiers::NONE);
    assert_eq!(
        harness.app.settings.language,
        Language::Fixed(Locale::Russian)
    );
    harness.press(Key::H, Modifiers::NONE);
    assert_eq!(
        harness.app.settings.language,
        Language::Fixed(Locale::English)
    );
    harness.press(Key::R, Modifiers::SHIFT);
    assert_eq!(harness.app.settings.language, Language::System);
}

#[test]
fn the_omarchy_row_says_a_system_language_is_not_translated() {
    let mut harness = omarchy(Some("fr_FR.UTF-8"));
    assert!(painted(&harness, "‹ english ›"));
    assert!(painted(
        &harness,
        "français is not translated yet · help translate: assets/i18n"
    ));
    // Shown as English, System steps on to Russian; back from there is
    // English, written.
    harness.press(Key::L, Modifiers::NONE);
    assert_eq!(
        harness.app.settings.language,
        Language::Fixed(Locale::Russian)
    );
    harness.press(Key::H, Modifiers::NONE);
    assert_eq!(
        harness.app.settings.language,
        Language::Fixed(Locale::English)
    );
}
