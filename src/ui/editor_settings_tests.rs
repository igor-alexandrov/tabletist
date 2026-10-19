//! Settings › Editor, in both looks: what each option does to the SQL
//! editor, and the pane that sets them with a click or a key and writes
//! them to the file.

use egui::accesskit::{Role, Toggled};
use egui::{Key, Modifiers};

use crate::keymap::Command as Keyed;
use crate::model::{Action, SettingsPane};
use crate::settings::{
    Autocomplete, EditorKeys, KeywordCase, LineNumbers, NewTab, OptionValue, RunScope, Settings,
};
use crate::testing::Harness;
use crate::theme::Look;

use super::appearance_settings_tests::{settings_saves, shows};

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

/// Settings on its Editor pane.
fn on_the_pane(harness: &mut Harness) {
    harness.app.apply(Action::ShowSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::Editor));
    harness.settle();
}

fn type_key(harness: &mut Harness, key: Key, text: &str) {
    super::tests::type_key(harness, key, text);
}

#[test]
fn with_no_file_each_look_has_its_editor_defaults_and_nothing_is_written() {
    let settings = Settings::default();
    assert_eq!(settings.new_tab, NewTab::ReadOnly);
    assert_eq!(settings.sql_limit, 1_000);
    assert_eq!(settings.sql_timeout_secs, Some(30));
    assert_eq!(settings.run_scope, RunScope::Statement);
    assert!(settings.confirm_unsafe_writes);
    assert_eq!(settings.autocomplete, Autocomplete::Typing);
    assert!(settings.close_pairs && settings.highlight_line && !settings.wrap);
    assert_eq!(settings.history_keep, 1_000);
    // What each look's board draws, while the file says nothing.
    assert_eq!(settings.editor_keys_in(false), EditorKeys::Standard);
    assert_eq!(settings.editor_keys_in(true), EditorKeys::Vim);
    assert_eq!(settings.line_numbers_in(false), LineNumbers::Absolute);
    assert_eq!(settings.line_numbers_in(true), LineNumbers::Relative);
    assert_eq!(settings.keywords_in(false), KeywordCase::Upper);
    assert_eq!(settings.keywords_in(true), KeywordCase::Lower);
    // Once written, every look has it.
    let set = Settings {
        line_numbers: Some(LineNumbers::Off),
        ..Settings::default()
    };
    assert_eq!(set.line_numbers_in(false), LineNumbers::Off);
    assert_eq!(set.line_numbers_in(true), LineNumbers::Off);
    for look in looks() {
        let mut harness = Harness::new();
        harness.set_look(look);
        on_the_pane(&mut harness);
        assert_eq!(settings_saves(&harness), 0, "{}", look.name);
        assert_eq!(
            harness.app.settings_file.text,
            "# written by tabletist, safe to edit by hand\n"
        );
    }
}

#[test]
fn the_old_keys_are_read_and_give_way_to_the_new_on_a_change() {
    let text = "# mine\n[editor]\n# the editor's\nsql_limit = 300\nsql_timeout_secs = 10\n";
    let loaded = Settings::from_toml(text);
    assert_eq!(loaded.invalid, Vec::<usize>::new());
    assert_eq!(loaded.settings.sql_limit, 300);
    assert_eq!(loaded.settings.sql_timeout_secs, Some(10));
    let before = loaded.settings;
    let mut after = before.clone();
    after.sql_limit = 500;
    let written = after.write_into(text, &before.changed_keys(&after));
    assert_eq!(
        written,
        "# mine\n[editor]\n# the editor's\nsql_timeout_secs = 10\n\n\
         [editor.run]\nlimit = 500  # 0 is no limit\n"
    );
    // The new key is read over an old one that is still there.
    let both = "[editor]\nsql_limit = 300\n\n[editor.run]\nlimit = 0\n";
    assert_eq!(Settings::from_toml(both).settings.sql_limit, 0);
    // A value of the old key that the new one cannot have is no longer
    // in the menus: it is kept and shown in its place.
    assert_eq!(Settings::sql_limits_with(300)[..3], [100, 300, 500]);
    assert_eq!(
        Settings::sql_timeouts_with(Some(10))[..2],
        [Some(5), Some(10)]
    );
}

#[test]
fn a_value_an_editor_key_cannot_have_is_ignored_by_its_line() {
    let text = "[editor.run]\nnew_tab = \"rw\"\nlimit = 500\ntimeout = \"2h\"\nrun = \"all\"\n\n\
                [editor.typing]\nindent = 3\nline_numbers = \"on\"\nautocomplete = \"manual\"\n";
    let loaded = Settings::from_toml(text);
    assert_eq!(loaded.invalid, [2, 4, 8, 9]);
    let settings = loaded.settings;
    assert_eq!(settings.new_tab, NewTab::ReadOnly);
    assert_eq!(settings.sql_limit, 500);
    assert_eq!(settings.sql_timeout_secs, Some(30));
    assert_eq!(settings.run_scope, RunScope::All);
    assert_eq!(settings.indent, crate::settings::Indent::Two);
    assert_eq!(settings.line_numbers, None);
    assert_eq!(settings.autocomplete, Autocomplete::Manual);
    // Indent is two, four or a tab, as a number or a word.
    for (value, indent) in [
        ("2", crate::settings::Indent::Two),
        ("4", crate::settings::Indent::Four),
        ("\"tab\"", crate::settings::Indent::Tab),
    ] {
        let text = format!("[editor.typing]\nindent = {value}\n");
        assert_eq!(
            Settings::from_toml(&text).settings.indent,
            indent,
            "{value}"
        );
    }
    assert_eq!(
        Settings::from_toml("[editor.typing]\nindent = \"4\"\n").invalid,
        [2]
    );
}

#[test]
fn the_sheet_sets_each_editor_option_with_a_click() {
    let look = Look::macos();
    let mut harness = Harness::with_size(egui::vec2(1440.0, 1100.0));
    harness.set_look(look);
    on_the_pane(&mut harness);
    // The board's words, and the run key's from the keymap.
    let run = crate::ui::keys::written(&harness.ctx, &look, Keyed::RunStatement);
    let runs = format!("{} runs", run.as_str());
    let complete = crate::ui::keys::written(&harness.ctx, &look, Keyed::Complete);
    for words in [
        "RUNNING QUERIES",
        "TYPING",
        "The badge in each tab can still switch it",
        "Caps the rows of results you browse",
        &runs,
        "A selection always wins",
        "Tables, views, columns and functions from the schema",
        &format!("On {}", complete.as_str()),
        "Current line highlight",
    ] {
        assert!(shows(&mut harness, words), "{words}");
    }
    // The rows of what is not built yet are not there.
    for words in [
        "Keybindings",
        "Indent",
        "Wrap",
        "FORMAT",
        "HISTORY",
        "Keywords",
    ] {
        assert!(!shows(&mut harness, words), "{words}");
    }
    let saves = settings_saves(&harness);
    harness.click("New tabs run in: Follow connection");
    harness.click("Row limit");
    harness.click("Row limit 500");
    harness.click("Timeout");
    harness.click("Timeout 1 min");
    harness.click(&format!("{runs}: Whole tab"));
    harness.click("Autocomplete: Off");
    harness.click("Line numbers");
    harness.click("Current line highlight");
    harness.settle();
    let settings = &harness.app.settings;
    assert_eq!(settings.new_tab, NewTab::Connection);
    assert_eq!(settings.sql_limit, 500);
    assert_eq!(settings.sql_timeout_secs, Some(60));
    assert_eq!(settings.run_scope, RunScope::All);
    assert_eq!(settings.autocomplete, Autocomplete::Off);
    assert_eq!(settings.line_numbers, Some(LineNumbers::Off));
    assert!(!settings.highlight_line);
    assert_eq!(settings_saves(&harness), saves + 7);
    // On again, the numbers are each line's own.
    harness.click("Line numbers");
    assert_eq!(
        harness.app.settings.line_numbers,
        Some(LineNumbers::Absolute)
    );
}

#[test]
fn a_value_the_menus_do_not_list_is_shown_in_its_place() {
    // The file sets a limit of 300: the sheet's menu lists it between 100
    // and 500, and the screen steps from it to 500.
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    harness
        .app
        .apply(Action::SetOption(OptionValue::RowLimit(300)));
    harness
        .app
        .apply(Action::SetOption(OptionValue::Timeout(Some(120))));
    on_the_pane(&mut harness);
    harness.click("Row limit");
    for entry in ["Row limit 100", "Row limit 300", "Row limit 500"] {
        assert!(harness.has(entry), "{entry}");
    }
    harness.press(Key::Escape, Modifiers::NONE);
    harness.click("Timeout");
    assert!(harness.has("Timeout 2 min"));
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(Look::omarchy());
    harness
        .app
        .apply(Action::SetOption(OptionValue::RowLimit(300)));
    on_the_pane(&mut harness);
    assert!(shows(&mut harness, "300"));
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::L, "l");
    assert_eq!(harness.app.settings.sql_limit, 500);
}

#[test]
fn a_stored_relative_shows_on_in_the_sheet_and_stays() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    harness
        .app
        .apply(Action::SetOption(OptionValue::LineNumbers(Some(
            LineNumbers::Relative,
        ))));
    on_the_pane(&mut harness);
    let tree = harness.settle();
    let switch = tree
        .nodes
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.role() == Role::CheckBox && node.label() == Some("Line numbers"))
        .expect("the switch");
    assert_eq!(switch.toggled(), Some(Toggled::True));
    harness.click("Current line highlight");
    assert_eq!(
        harness.app.settings.line_numbers,
        Some(LineNumbers::Relative)
    );
}

#[test]
fn the_screen_sets_each_editor_option_with_its_keys() {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(Look::omarchy());
    on_the_pane(&mut harness);
    let read_only = crate::ui::keys::written(&harness.ctx, &Look::omarchy(), Keyed::ReadOnlyTab);
    for words in [
        "running queries",
        "typing",
        read_only.as_str(),
        "per tab",
        "caps results you browse",
        "a selection always wins",
        "schema objects",
    ] {
        assert!(shows(&mut harness, words), "{words}");
    }
    assert!(!shows(&mut harness, "current line"));
    let saves = settings_saves(&harness);
    // New tabs: follow the connection.
    type_key(&mut harness, Key::L, "l");
    // The limit a step up, the timeout a step down.
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::L, "l");
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::H, "h");
    // The whole tab, with space.
    type_key(&mut harness, Key::J, "j");
    harness.press(Key::Space, Modifiers::NONE);
    // Line numbers from relative to absolute; completion on the key.
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::L, "l");
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::L, "l");
    harness.settle();
    let settings = &harness.app.settings;
    assert_eq!(settings.new_tab, NewTab::Connection);
    assert_eq!(settings.sql_limit, 5_000);
    assert_eq!(settings.sql_timeout_secs, Some(15));
    assert_eq!(settings.run_scope, RunScope::All);
    assert_eq!(settings.line_numbers, Some(LineNumbers::Absolute));
    assert_eq!(settings.autocomplete, Autocomplete::Manual);
    assert_eq!(settings_saves(&harness), saves + 6);
    // The file has each of them, and R puts the cursor's back.
    let text = harness.app.settings_file.text.clone();
    assert!(
        text.contains("[editor.run]\nnew_tab = \"connection\""),
        "{text}"
    );
    assert!(text.contains("autocomplete = \"manual\""), "{text}");
    harness.press(Key::R, Modifiers::SHIFT);
    harness.settle();
    assert_eq!(harness.app.settings.autocomplete, Autocomplete::Typing);
}

#[test]
fn a_look_s_own_value_picked_while_none_is_set_is_not_written() {
    // Omarchy: `h` on line numbers, which show the look's relative, the
    // leftmost; a click on the relative segment.
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(Look::omarchy());
    on_the_pane(&mut harness);
    for _ in 0..4 {
        type_key(&mut harness, Key::J, "j");
    }
    type_key(&mut harness, Key::H, "h");
    harness.click("Line numbers: relative");
    harness.settle();
    assert_eq!(harness.app.settings.line_numbers, None);
    assert_eq!(settings_saves(&harness), 0);
    // Another value is written, and is then every look's.
    type_key(&mut harness, Key::L, "l");
    assert_eq!(
        harness.app.settings.line_numbers,
        Some(LineNumbers::Absolute)
    );
    harness.click("Line numbers: relative");
    assert_eq!(
        harness.app.settings.line_numbers,
        Some(LineNumbers::Relative)
    );
    // The same for the keys and the keyword case, in either look.
    let settings = Settings::default();
    for terminal in [false, true] {
        let keys = OptionValue::EditorKeys(Some(EditorKeys::of_look(terminal)));
        assert_eq!(
            keys.for_look(&settings, terminal),
            OptionValue::EditorKeys(None)
        );
        let case = OptionValue::Keywords(Some(KeywordCase::of_look(!terminal)));
        assert_eq!(case.clone().for_look(&settings, terminal), case);
    }
}

#[test]
fn the_run_scope_is_a_radio_group_named_with_the_run_key_and_described() {
    for look in looks() {
        let mut harness = Harness::with_size(egui::vec2(1920.0, 2400.0));
        harness.set_look(look);
        on_the_pane(&mut harness);
        let tree = harness.settle();
        let run = crate::ui::keys::written(&harness.ctx, &look, Keyed::RunStatement);
        let name = format!("{} runs", run.as_str());
        let name = if look.terminal {
            look.label(&name)
        } else {
            name
        };
        let (_, group) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == Role::RadioGroup && node.label() == Some(&name))
            .unwrap_or_else(|| panic!("no radio group {name} in {}", look.name));
        let children = group.children().to_vec();
        let buttons: Vec<_> = tree
            .nodes
            .iter()
            .filter(|(id, node)| children.contains(id) && node.role() == Role::RadioButton)
            .map(|(_, node)| node)
            .collect();
        assert_eq!(buttons.len(), 2, "{}", look.name);
        for button in buttons {
            assert_eq!(
                button.description(),
                Some("A selection always wins"),
                "{:?} in {}",
                button.label(),
                look.name
            );
        }
    }
}

#[test]
fn reset_to_defaults_puts_every_editor_key_back_and_nothing_else() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    for value in [
        OptionValue::NewTab(NewTab::Connection),
        OptionValue::RowLimit(0),
        OptionValue::Timeout(None),
        OptionValue::RunScope(RunScope::All),
        OptionValue::Autocomplete(Autocomplete::Off),
        OptionValue::LineNumbers(Some(LineNumbers::Off)),
        OptionValue::HighlightLine(false),
        // Keys the pane has no row for yet are its too.
        OptionValue::Wrap(true),
        OptionValue::EditorKeys(Some(EditorKeys::Vim)),
        OptionValue::HistoryKeep(0),
        OptionValue::PageSize(500),
    ] {
        harness.app.apply(Action::SetOption(value));
    }
    on_the_pane(&mut harness);
    harness.click("Reset to defaults");
    assert!(shows(
        &mut harness,
        "Reset all Editor settings to their defaults?"
    ));
    harness.app.apply(Action::ConfirmResetSettings(true));
    harness.settle();
    let settings = &harness.app.settings;
    let defaults = Settings::default();
    assert_eq!(settings.new_tab, defaults.new_tab);
    assert_eq!(settings.sql_limit, defaults.sql_limit);
    assert_eq!(settings.sql_timeout_secs, defaults.sql_timeout_secs);
    assert_eq!(settings.run_scope, defaults.run_scope);
    assert_eq!(settings.autocomplete, defaults.autocomplete);
    assert_eq!(settings.line_numbers, None);
    assert!(settings.highlight_line);
    assert!(!settings.wrap);
    assert_eq!(settings.editor_keys, None);
    assert_eq!(settings.history_keep, defaults.history_keep);
    // Another pane's options are its own.
    assert_eq!(settings.page_size, 500);
}

#[test]
fn the_file_pane_marks_the_old_keys_line_for_the_row_it_sets() {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(Look::omarchy());
    on_the_pane(&mut harness);
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::SettingsFile {
            text: "[editor]\nsql_limit = 300\n".into(),
            own: false,
        }));
    harness.settle();
    // The row limit, one under the first row.
    type_key(&mut harness, Key::J, "j");
    harness.settle();
    let selection = harness.app.palette.selection;
    let line = harness
        .painted_rect("sql_limit = ")
        .expect("the old key's line");
    assert!(harness.fills.iter().any(|(rect, color)| {
        *color == selection
            && rect.y_range().contains(line.center().y)
            && rect.x_range().contains(line.center().x)
    }));
}
