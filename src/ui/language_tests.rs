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

/// The Omarchy Language row's hint under System: where the system's
/// language is read from on this platform, and where its catalogs are.
pub(super) const SYSTEM_HINT: &str = if cfg!(any(target_os = "macos", target_os = "windows")) {
    "from the system · help translate: assets/i18n"
} else {
    "from $LANG · help translate: assets/i18n"
};

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

/// Opens the Language row's menu, `name` its label: the section's heading
/// has the same words, so the menu is found by what it is.
fn open_the_menu(harness: &mut Harness, name: &str) {
    use egui::accesskit;
    let tree = harness.settle();
    let menu = crate::testing::node(&tree, name, accesskit::Role::ComboBox)
        .unwrap_or_else(|| panic!("no menu labelled {name:?}"));
    harness.frame(vec![egui::Event::AccessKitActionRequest(
        accesskit::ActionRequest {
            target_tree: accesskit::TreeId::ROOT,
            target_node: menu,
            action: accesskit::Action::Click,
            data: None,
        },
    )]);
    harness.settle();
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
    // Through the row's menu, as a click chooses.
    open_the_menu(&mut harness, "Language");
    harness.click("Language Русский");
    assert_eq!(harness.app.locale, Locale::Russian);
    // The sheet's title and the nav's panes, from the catalog.
    assert!(painted(&harness, "Настройки"));
    assert!(painted(&harness, "Основные"));
    // The English word stays beside the label: the way back.
    assert!(painted(&harness, "Язык · Language"));
    assert!(painted(&harness, "Русский"));
    // Sent to be saved, not only kept in memory.
    let saved = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            crate::backend::Command::Save {
                file: crate::backend::StateFile::Settings(text),
                ..
            } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default();
    assert!(saved.contains("[interface]\nlanguage = \"ru\""), "{saved}");
    // And back to English, through the menu its label now names.
    open_the_menu(&mut harness, "Язык");
    harness.click("Язык English");
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
    assert!(painted(&harness, SYSTEM_HINT));
    harness.press(Key::L, Modifiers::NONE);
    assert_eq!(
        harness.app.settings.language,
        Language::Fixed(Locale::English)
    );
    // A language chosen here does not come from the system.
    assert!(painted(&harness, "help translate: assets/i18n"));
    assert!(!painted(&harness, SYSTEM_HINT));
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

/// Russian names every pane in the nav and the General pane's sections
/// and rows, in both looks: the Omarchy look in its lower case.
#[test]
fn the_russian_settings_name_every_pane_and_row() {
    let panes = [
        "Основные",
        "Оформление",
        "Редактор",
        "Клавиатура",
        "Агенты",
        "Обновления",
    ];
    for look in [Look::macos(), Look::omarchy()] {
        let harness = general(look, Some("ru-RU"));
        assert_eq!(harness.app.locale, Locale::Russian);
        let said = |text: &str| match look.terminal {
            true => text.to_lowercase(),
            false => text.to_owned(),
        };
        for pane in panes {
            assert!(painted(&harness, &said(pane)), "{}: {pane}", look.name);
        }
        for row in [
            "При запуске",
            "Восстанавливать вкладки и фильтры",
            "Строк на странице",
        ] {
            assert!(
                harness
                    .painted
                    .iter()
                    .any(|(text, _)| text.contains(&said(row))),
                "{}: {row}",
                look.name
            );
        }
    }
}

/// Quick open's path to a setting is said in Russian, pane and row.
#[test]
fn quick_open_says_a_setting_s_path_in_russian() {
    let path = crate::app::setting_path(Locale::Russian, "run", OptionId::OnLaunch);
    assert_eq!(path, "Настройки › Основные › При запуске");
    let path = crate::app::setting_path(Locale::English, "run", OptionId::OnLaunch);
    assert_eq!(path, "Settings › General › On launch");
}

/// The keyboard help lists the keymap's commands under Russian headings.
#[test]
fn the_keyboard_help_says_its_commands_in_russian() {
    let mut harness = general(Look::macos(), Some("ru-RU"));
    harness.app.apply(Action::CloseDialog);
    harness.app.apply(Action::ShowHelp);
    let names = crate::testing::labels(&harness.finish_animations());
    for wanted in [
        "Навигация",
        "Найти любой объект",
        "Новая вкладка редактора SQL",
    ] {
        assert!(
            names.iter().any(|name| name.contains(wanted)),
            "{wanted} is not in {names:?}"
        );
    }
}

#[test]
fn the_russian_agents_note_stacks_its_wrapped_paragraphs() {
    if !SettingsPane::listed().contains(&SettingsPane::Agents) {
        return;
    }
    // A window narrow enough that the Russian first paragraph wraps.
    let mut harness = general(Look::omarchy(), Some("ru-RU"));
    harness.size = egui::vec2(700.0, 2400.0);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::Agents));
    harness.settle();
    let rect = |start: &str| {
        let found = harness
            .text_rects
            .iter()
            .filter(|(text, _)| text.starts_with(start))
            .map(|(_, rect)| *rect)
            .reduce(|a, b| a.union(b));
        found.unwrap_or_else(|| panic!("no {start:?} in {:?}", harness.painted))
    };
    let first = rect("чтение выполняется");
    let second = rect("без ddl");
    assert!(first.bottom() <= second.top(), "{first:?} over {second:?}");
}
