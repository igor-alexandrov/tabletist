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
    let saved = saved_settings(&harness);
    assert!(saved.contains("[interface]\nlanguage = \"ru\""), "{saved}");
    // And back to English, through the menu its label now names.
    open_the_menu(&mut harness, "Язык");
    harness.click("Язык English");
    assert_eq!(harness.app.locale, Locale::English);
    assert!(painted(&harness, "Settings"));
}

/// The settings file's text as the window last sent it to be saved.
fn saved_settings(harness: &Harness) -> String {
    harness
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
        .unwrap_or_default()
}

#[test]
fn a_portuguese_system_reads_brazilian_portuguese() {
    for system in ["pt_BR.UTF-8", "pt-PT"] {
        let harness = general(Look::macos(), Some(system));
        assert_eq!(harness.app.locale, Locale::BrazilianPortuguese, "{system}");
        assert_eq!(harness.app.settings.language, Language::System);
        assert!(
            painted(&harness, "Sistema (Português (Brasil))"),
            "{system}"
        );
        // The English word stays beside the label: the way back.
        assert!(painted(&harness, "Idioma · Language"), "{system}");
    }
}

#[test]
fn choosing_portuguese_writes_its_tag_and_names_the_others_in_portuguese() {
    let mut harness = general(Look::macos(), Some("en-US"));
    open_the_menu(&mut harness, "Language");
    harness.click("Language Português (Brasil)");
    assert_eq!(harness.app.locale, Locale::BrazilianPortuguese);
    assert!(painted(&harness, "Configurações"));
    assert!(painted(&harness, "Geral"));
    let saved = saved_settings(&harness);
    assert!(
        saved.contains("[interface]\nlanguage = \"pt-BR\""),
        "{saved}"
    );
    // The menu names each other language in Portuguese beside its own.
    open_the_menu(&mut harness, "Idioma");
    assert!(painted(&harness, "Inglês"));
    assert!(painted(&harness, "Russo"));
}

#[test]
fn a_russian_window_names_portuguese_in_russian() {
    let mut harness = general(Look::macos(), Some("ru-RU"));
    open_the_menu(&mut harness, "Язык");
    assert!(painted(&harness, "Português (Brasil)"));
    assert!(painted(&harness, "Португальский (Бразилия)"));
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
        Language::Fixed(Locale::BrazilianPortuguese)
    );
    assert_eq!(harness.app.locale, Locale::BrazilianPortuguese);
    assert!(painted(&harness, "‹ português (brasil) ›"));
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
        Language::Fixed(Locale::BrazilianPortuguese)
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
    // Shown as English, System steps on to the next language; back from
    // there is English, written.
    harness.press(Key::L, Modifiers::NONE);
    assert_eq!(
        harness.app.settings.language,
        Language::Fixed(Locale::BrazilianPortuguese)
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

/// `harness`'s window in Russian, as the app draws it at its start when
/// the system's first language is Russian.
pub(super) fn in_russian(harness: &mut Harness) {
    harness.app.system_language = Some(tag("ru-RU"));
    let settings = harness.app.settings.clone();
    harness.app.apply_settings(settings);
    harness.settle();
    assert_eq!(harness.app.locale, Locale::Russian);
}

/// The looks whose grid is edited in place.
fn desktop_looks() -> impl Iterator<Item = Look> {
    Look::ALL.into_iter().filter(|look| !look.terminal)
}

/// A row the review cannot send says why in one Russian phrase, its key
/// and its column put in where the translation has them.
#[test]
fn the_review_s_comments_are_whole_russian_phrases() {
    for look in desktop_looks() {
        let (mut harness, tab, id) = super::tests::editable_in(look);
        in_russian(&mut harness);
        super::tests::make_pending(&mut harness, tab, id, (1, 1), "bob@example.com");
        super::tests::review(&mut harness, tab, id);
        super::tests::leave_pending(&mut harness, tab, id, (3, 1), "dan@example.com");
        super::tests::leave_pending(&mut harness, tab, id, (3, 2), "{oops");
        harness.settle();
        let blocked = "-- строка id 4 · заблокирована: сначала исправьте meta";
        assert!(
            painted(&harness, blocked),
            "{}: {:?}",
            look.name,
            harness.painted
        );
    }
}

/// A comment's count takes the Russian form for its number.
#[test]
fn a_review_comment_counts_its_rows_in_russian() {
    use crate::review::Line;
    let said = |rows| crate::ui::review::comment(&Line::DeleteRun(rows), Locale::Russian);
    assert_eq!(said(1).as_deref(), Some("-- удалить 1 строку"));
    assert_eq!(said(3).as_deref(), Some("-- удалить 3 строки"));
    assert_eq!(said(5).as_deref(), Some("-- удалить 5 строк"));
    assert_eq!(said(21).as_deref(), Some("-- удалить 21 строку"));
    let english = crate::ui::review::comment(&Line::DeleteRun(3), Locale::English);
    assert_eq!(english.as_deref(), Some("-- delete 3 rows"));
}

/// A number past what the column holds is said with the type's limits,
/// in Russian and as one phrase: the type, then from and to.
#[test]
fn the_cell_editor_says_a_type_s_limits_in_russian() {
    for look in desktop_looks() {
        let (mut harness, tab, id) = super::tests::with_quantities(look);
        in_russian(&mut harness);
        super::tests::open_editor(&mut harness, tab, id, (1, 2));
        super::tests::type_text(&mut harness, "99999999999999999999");
        let limits = harness
            .painted
            .iter()
            .find(|(text, _)| text.starts_with("INTEGER: от "))
            .map(|(text, _)| text.clone());
        let limits = limits.unwrap_or_else(|| panic!("{}: {:?}", look.name, harness.painted));
        let (from, to) = limits["INTEGER: от ".len()..]
            .split_once(" до ")
            .unwrap_or_else(|| panic!("{}: {limits:?}", look.name));
        assert!(from.parse::<i128>().is_ok(), "{}: {limits:?}", look.name);
        assert!(to.parse::<i128>().is_ok(), "{}: {limits:?}", look.name);
    }
}

/// `book_covers` open on a page whose `publisher_id` holds an amount, in
/// `look` with grouping on, the window's language `locale`, counted to
/// 12,480 rows.
fn grouped_covers(look: Look, locale: Locale) -> Harness {
    let mut harness = Harness::with_size(egui::vec2(1400.0, 1200.0));
    harness.set_look(look);
    harness.app.settings.group_digits = Some(true);
    if locale == Locale::Russian {
        in_russian(&mut harness);
    }
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "book_covers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::book_covers_structure());
    let mut page = crate::testing::book_covers_page(3);
    page.rows[0][1] = tabletist_db::Value::Text("1240.50".into());
    harness.answer_rows(page);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    let object = harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .object_tab_mut(id)
        .unwrap();
    object.count.value = Some(12_480);
    harness.settle();
    harness
}

/// In Russian a grouped grid number and the page's count take Russian's
/// marks, a no-break space between the thousands and a decimal comma, and
/// the grid's dates stay ISO. English is as it was.
#[test]
fn russian_numbers_take_its_marks_and_grid_dates_stay_iso() {
    for look in Look::ALL {
        let harness = grouped_covers(look, Locale::Russian);
        let shown = |text: &str| painted(&harness, text);
        assert!(
            shown("1\u{a0}240,50"),
            "{}: {:?}",
            look.name,
            harness.painted
        );
        assert!(
            shown("9\u{a0}100\u{a0}000\u{a0}000\u{a0}000\u{a0}000\u{a0}002"),
            "{}: {:?}",
            look.name,
            harness.painted
        );
        assert!(
            shown("2026-01-12 09:14:03"),
            "{}: grid dates stay ISO",
            look.name
        );
        let range = match look.terminal {
            true => "1–3/12\u{a0}480",
            false => "1–3 из 12\u{a0}480",
        };
        let counted = harness.painted.iter().any(|(text, _)| text.contains(range));
        assert!(counted, "{}: {:?}", look.name, harness.painted);
        let english = grouped_covers(look, Locale::English);
        let shown = |text: &str| painted(&english, text);
        assert!(shown("1,240.50"), "{}: {:?}", look.name, english.painted);
        assert!(shown("9,100,000,000,000,000,002"), "{}", look.name);
        let range = match look.terminal {
            true => "1–3/12,480",
            false => "1–3 of 12,480",
        };
        let counted = english.painted.iter().any(|(text, _)| text.contains(range));
        assert!(counted, "{}: {:?}", look.name, english.painted);
    }
}
