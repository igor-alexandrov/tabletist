//! Settings › Editor, in both looks: what each option does to the SQL
//! editor, and the pane that sets them with a click or a key and writes
//! them to the file.

use egui::accesskit::{Role, Toggled};
use egui::{Key, Modifiers};

use crate::backend::Command;
use crate::keymap::Command as Keyed;
use crate::model::{Action, ConnTabId, RunMode, SettingsPane, SqlTab, TabId};
use crate::settings::{
    Autocomplete, EditorKeys, KeywordCase, LineNumbers, NewTab, OptionValue, RunScope, Settings,
};
use crate::testing::{Harness, bounds};
use crate::theme::Look;

use super::appearance_settings_tests::{settings_saves, shows};

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

/// A connection that takes writes (`writable`) or not, in `look`, with a
/// new SQL editor that has the keyboard and holds `text`.
fn editor(look: Look, writable: bool, text: &str) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(!writable);
    harness.press(Key::T, Modifiers::COMMAND);
    if !text.is_empty() {
        harness.frame(vec![egui::Event::Paste(text.into())]);
    }
    harness.settle();
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    (harness, tab, id)
}

fn sql(harness: &Harness, tab: ConnTabId, id: TabId) -> &SqlTab {
    harness.app.workspace(tab).unwrap().sql_tab(id).unwrap()
}

/// The mode a run of the editor would have now.
fn run_mode(harness: &Harness, tab: ConnTabId, id: TabId) -> RunMode {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.run_mode(workspace.sql_tab(id).unwrap())
}

/// A new editor of `tab`, shown.
fn new_editor(harness: &mut Harness, tab: ConnTabId) -> TabId {
    harness.app.apply(Action::NewSqlTab(tab));
    harness.settle();
    harness.app.workspace(tab).unwrap().active_tab.unwrap()
}

/// The statements, limit and timeout of the last run sent.
fn last_run(harness: &Harness) -> (Vec<String>, u32, Option<std::time::Duration>) {
    let mut sent = harness.app.backend.sent.iter().rev();
    sent.find_map(|command| match command {
        Command::RunSql {
            statements,
            limit,
            timeout,
            ..
        } => Some((
            statements.iter().map(|s| s.text.clone()).collect(),
            *limit,
            *timeout,
        )),
        _ => None,
    })
    .expect("a run was sent")
}

/// Settings on its Editor pane.
fn on_the_pane(harness: &mut Harness) {
    harness.app.apply(Action::ShowSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::Editor));
    harness.settle();
}

/// The settings changed by `change`, as an edit of the file changes them.
fn change(harness: &mut Harness, change: impl FnOnce(&mut Settings)) {
    let mut settings = harness.app.settings.clone();
    change(&mut settings);
    harness.app.apply_settings(settings);
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
fn a_new_tab_opens_read_only_unless_the_settings_follow_the_connection() {
    for look in looks() {
        // A dev connection that takes writes: read-only all the same.
        let (mut harness, tab, id) = editor(look, true, "");
        assert_eq!(
            sql(&harness, tab, id).mode,
            RunMode::ReadOnly,
            "{}",
            look.name
        );
        // The badge still switches it.
        harness.app.apply(Action::SetSqlMode {
            tab,
            sql_tab: id,
            mode: RunMode::ReadWrite,
        });
        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
        // Follow connection: a new tab writes where its connection does,
        // and the tabs open keep their mode.
        harness
            .app
            .apply(Action::SetOption(OptionValue::NewTab(NewTab::Connection)));
        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
        let follows = new_editor(&mut harness, tab);
        assert_eq!(run_mode(&harness, tab, follows), RunMode::ReadWrite);
        harness
            .app
            .apply(Action::SetOption(OptionValue::NewTab(NewTab::ReadOnly)));
        assert_eq!(run_mode(&harness, tab, follows), RunMode::ReadWrite);
        let read = new_editor(&mut harness, tab);
        assert_eq!(run_mode(&harness, tab, read), RunMode::ReadOnly);
        // A read-only connection caps its tabs, whatever the setting.
        let (mut harness, tab, _) = editor(look, false, "");
        harness
            .app
            .apply(Action::SetOption(OptionValue::NewTab(NewTab::Connection)));
        let capped = new_editor(&mut harness, tab);
        assert_eq!(
            run_mode(&harness, tab, capped),
            RunMode::ReadOnly,
            "{}",
            look.name
        );
    }
}

#[test]
fn every_open_tab_follows_the_limit_and_the_timeout() {
    let (mut harness, tab, first) = editor(Look::macos(), false, "SELECT 1");
    let second = new_editor(&mut harness, tab);
    harness
        .app
        .apply(Action::SetOption(OptionValue::RowLimit(5_000)));
    harness
        .app
        .apply(Action::SetOption(OptionValue::Timeout(Some(300))));
    for id in [first, second] {
        assert_eq!(sql(&harness, tab, id).limit, 5_000);
        assert_eq!(
            sql(&harness, tab, id).timeout,
            Some(std::time::Duration::from_secs(300))
        );
    }
    // No limit asks for every row; Off sets no timer.
    harness
        .app
        .apply(Action::SetOption(OptionValue::RowLimit(0)));
    harness
        .app
        .apply(Action::SetOption(OptionValue::Timeout(None)));
    harness.app.apply(Action::ActivateTab { tab, id: first });
    harness.settle();
    harness.press(Key::Enter, Modifiers::COMMAND);
    let (statements, limit, timeout) = last_run(&harness);
    assert_eq!(statements, ["SELECT 1"]);
    assert_eq!(limit, u32::MAX);
    assert_eq!(timeout, None);
    // The toolbar's menus offer the setting's values and say No limit.
    harness.click("Limit");
    assert!(harness.has("No limit"));
    assert!(harness.has("Limit 5,000"));
}

#[test]
fn the_run_key_runs_the_statement_the_whole_tab_or_the_selection() {
    let text = "SELECT 1;\nSELECT 2;\nSELECT 3";
    for look in looks() {
        let (mut harness, _, _) = editor(look, false, text);
        // The cursor is at the end, in the last statement.
        harness.press(Key::Enter, Modifiers::COMMAND);
        assert_eq!(last_run(&harness).0, ["SELECT 3"], "{}", look.name);
        harness
            .app
            .apply(Action::SetOption(OptionValue::RunScope(RunScope::All)));
        harness.settle();
        harness.press(Key::Enter, Modifiers::COMMAND);
        assert_eq!(
            last_run(&harness).0,
            ["SELECT 1", "SELECT 2", "SELECT 3"],
            "{}",
            look.name
        );
    }
    // A selection wins either way: what it covers, exactly.
    for look in looks() {
        for scope in [RunScope::All, RunScope::Statement] {
            let (mut harness, _, _) = editor(look, false, text);
            harness
                .app
                .apply(Action::SetOption(OptionValue::RunScope(scope)));
            harness.settle();
            // From the end back to the `2`: "2;\nSELECT 3".
            for _ in 0..11 {
                harness.press(Key::ArrowLeft, Modifiers::SHIFT);
            }
            harness.press(Key::Enter, Modifiers::COMMAND);
            assert_eq!(
                last_run(&harness).0,
                ["2", "SELECT 3"],
                "{} {scope:?}",
                look.name
            );
        }
    }
}

#[test]
fn the_run_button_says_what_it_runs() {
    let (mut harness, _, _) = editor(Look::macos(), false, "SELECT 1;\nSELECT 2");
    let hovered = |harness: &mut Harness| {
        let tree = harness.settle();
        let at = bounds(&tree, "Run", Role::Button).expect("the Run button");
        let shown = super::tests::hover(harness, at.center());
        harness.frame(vec![egui::Event::PointerGone]);
        harness.settle();
        shown
    };
    assert!(hovered(&mut harness).contains(&"Run the statement at the cursor".to_owned()));
    harness
        .app
        .apply(Action::SetOption(OptionValue::RunScope(RunScope::All)));
    assert!(hovered(&mut harness).contains(&"Run every statement".to_owned()));
    // The editor still has the keyboard.
    for _ in 0..3 {
        harness.press(Key::ArrowLeft, Modifiers::SHIFT);
    }
    assert!(hovered(&mut harness).contains(&"Run the selection".to_owned()));
}

#[test]
fn a_selection_keeps_its_statements_lines_and_splits_where_the_sql_does() {
    let text = "SELECT 'a;b';\n-- one; two\nSELECT 2;\nSELECT 3";
    // The lines of the statements of the last run sent.
    let lines = |harness: &Harness| -> Vec<(String, usize)> {
        let mut sent = harness.app.backend.sent.iter().rev();
        sent.find_map(|command| match command {
            Command::RunSql { statements, .. } => Some(
                statements
                    .iter()
                    .map(|s| (s.text.clone(), s.first_line))
                    .collect(),
            ),
            _ => None,
        })
        .expect("a run was sent")
    };
    // From the end back to the comment: each statement where the editor
    // has it, so that an error marks the line it shows.
    let (mut harness, _, _) = editor(Look::macos(), false, text);
    for _ in 0.."-- one; two\nSELECT 2;\nSELECT 3".chars().count() {
        harness.press(Key::ArrowLeft, Modifiers::SHIFT);
    }
    harness.press(Key::Enter, Modifiers::COMMAND);
    assert_eq!(
        lines(&harness),
        [
            ("-- one; two\nSELECT 2".to_owned(), 3),
            ("SELECT 3".to_owned(), 4)
        ]
    );
    // The whole tab splits on neither the string's `;` nor the comment's.
    let (mut harness, _, _) = editor(Look::macos(), false, text);
    harness
        .app
        .apply(Action::SetOption(OptionValue::RunScope(RunScope::All)));
    harness.settle();
    harness.press(Key::Enter, Modifiers::COMMAND);
    let texts: Vec<String> = lines(&harness).into_iter().map(|(text, _)| text).collect();
    assert_eq!(texts, ["SELECT 'a;b'", "-- one; two\nSELECT 2", "SELECT 3"]);
    // Nor on PostgreSQL's dollar quote, which the run takes from the same
    // splitting (the fixture's database is SQLite).
    let quoted =
        tabletist_db::sql::statements(tabletist_db::Dialect::Postgres, "SELECT $$x;y$$; SELECT 3");
    assert_eq!(quoted.len(), 2);
}

#[test]
fn autocomplete_opens_as_set_and_typing_asks_the_database_nothing() {
    let typed = |harness: &mut Harness, text: &str| {
        harness.frame(vec![egui::Event::Text(text.into())]);
        harness.settle();
    };
    let open = |harness: &Harness, tab, id| sql(harness, tab, id).completion.is_some();
    // As you type, with the schema loaded: a list, and nothing sent.
    let (mut harness, tab, id) = editor(Look::macos(), false, "");
    let sent = harness.app.backend.sent.len();
    typed(&mut harness, "SELECT * FROM us");
    assert!(open(&harness, tab, id));
    assert_eq!(harness.app.backend.sent.len(), sent, "typing sent nothing");
    // On the key alone: typing opens nothing, the key does, and the list
    // goes on as the word is typed.
    let (mut harness, tab, id) = editor(Look::macos(), false, "");
    harness
        .app
        .apply(Action::SetOption(OptionValue::Autocomplete(
            Autocomplete::Manual,
        )));
    typed(&mut harness, "SELECT * FROM us");
    assert!(!open(&harness, tab, id));
    harness.press(Key::Space, Modifiers::CTRL);
    assert!(open(&harness, tab, id));
    typed(&mut harness, "e");
    assert!(open(&harness, tab, id));
    // Off: neither.
    harness
        .app
        .apply(Action::SetOption(OptionValue::Autocomplete(
            Autocomplete::Off,
        )));
    harness.settle();
    assert!(!open(&harness, tab, id), "turned off, the open list closes");
    typed(&mut harness, "r");
    harness.press(Key::Space, Modifiers::CTRL);
    assert!(!open(&harness, tab, id));
}

/// The numbers the gutter painted, top to bottom.
fn gutter(harness: &mut Harness) -> Vec<String> {
    let tree = harness.settle();
    let field = bounds(&tree, "SQL", Role::MultilineTextInput).expect("the editor");
    let mut numbers: Vec<(f32, String)> = harness
        .text_rects
        .iter()
        .filter(|(text, rect)| {
            rect.right() <= field.left()
                && rect.top() >= field.top()
                && text.bytes().all(|byte| byte.is_ascii_digit())
        })
        .map(|(text, rect)| (rect.top(), text.clone()))
        .collect();
    numbers.sort_by(|a, b| a.0.total_cmp(&b.0));
    numbers.into_iter().map(|(_, text)| text).collect()
}

#[test]
fn the_gutter_numbers_the_lines_as_set_in_either_look() {
    for look in looks() {
        // The cursor is on the last of three lines.
        let (mut harness, _, _) = editor(look, false, "SELECT 1\nFROM t\nWHERE x");
        let own = if look.terminal {
            ["2", "1", "3"]
        } else {
            ["1", "2", "3"]
        };
        assert_eq!(gutter(&mut harness), own, "{}", look.name);
        change(&mut harness, |settings| {
            settings.line_numbers = Some(LineNumbers::Relative)
        });
        assert_eq!(gutter(&mut harness), ["2", "1", "3"], "{}", look.name);
        change(&mut harness, |settings| {
            settings.line_numbers = Some(LineNumbers::Absolute)
        });
        assert_eq!(gutter(&mut harness), ["1", "2", "3"], "{}", look.name);
        change(&mut harness, |settings| {
            settings.line_numbers = Some(LineNumbers::Off)
        });
        assert_eq!(gutter(&mut harness), Vec::<String>::new(), "{}", look.name);
    }
}

#[test]
fn the_cursors_line_is_filled_unless_the_settings_turn_it_off() {
    for look in looks() {
        let (mut harness, _, _) = editor(look, false, "SELECT 1\nFROM t");
        let palette = harness.app.palette;
        let fill = if look.terminal {
            palette.selection
        } else {
            palette.window.lerp_to_gamma(palette.selection, 0.45)
        };
        // The cursor's line, across the editor: wider than any other fill
        // of that colour a fixture draws.
        let field = bounds(&harness.settle(), "SQL", Role::MultilineTextInput).expect("the editor");
        let filled = |harness: &mut Harness| {
            harness.settle();
            harness
                .fills
                .iter()
                .any(|(rect, color)| *color == fill && rect.width() >= field.width())
        };
        assert!(filled(&mut harness), "{}", look.name);
        change(&mut harness, |settings| settings.highlight_line = false);
        assert!(!filled(&mut harness), "{}", look.name);
        // The editor kept the keyboard: on again, the line is filled again.
        change(&mut harness, |settings| settings.highlight_line = true);
        assert!(filled(&mut harness), "{}", look.name);
    }
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
        "Caps results you browse. Show all and exports get every row.",
        &runs,
        "A selection always wins",
        "Tables, views, columns and functions from the schema",
        &format!("On {}", complete.as_str()),
        "Current line highlight",
        // History is built: its row covers every source.
        "HISTORY",
        "Keep query history",
        "Per connection, on this computer: SQL editor, grid saves, filters and agent runs. Results are never stored.",
    ] {
        assert!(shows(&mut harness, words), "{words}");
    }
    // The rows of what is not built yet are not there.
    assert!(!shows(&mut harness, "Keybindings"));
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
    // A time of minutes and seconds ("90s" in the file) reads as both.
    harness.press(Key::Escape, Modifiers::NONE);
    harness
        .app
        .apply(Action::SetOption(OptionValue::Timeout(Some(90))));
    harness.click("Timeout");
    for entry in ["Timeout 1 min", "Timeout 1 min 30 s", "Timeout 5 min"] {
        assert!(harness.has(entry), "{entry}");
    }
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(Look::omarchy());
    harness
        .app
        .apply(Action::SetOption(OptionValue::RowLimit(300)));
    harness
        .app
        .apply(Action::SetOption(OptionValue::Timeout(Some(90))));
    on_the_pane(&mut harness);
    assert!(shows(&mut harness, "300"));
    assert!(shows(&mut harness, "1m 30s"));
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
        "caps results you browse · show all and exports get every row",
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
    // Line numbers from relative to absolute; four spaces; completion on
    // the key; no closing brackets and wrapped lines, with space; keywords
    // from the look's lower case to upper; leading commas, with space.
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::L, "l");
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::L, "l");
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::L, "l");
    type_key(&mut harness, Key::J, "j");
    harness.press(Key::Space, Modifiers::NONE);
    type_key(&mut harness, Key::J, "j");
    harness.press(Key::Space, Modifiers::NONE);
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::H, "h");
    type_key(&mut harness, Key::J, "j");
    harness.press(Key::Space, Modifiers::NONE);
    harness.settle();
    let settings = &harness.app.settings;
    assert_eq!(settings.new_tab, NewTab::Connection);
    assert_eq!(settings.sql_limit, 5_000);
    assert_eq!(settings.sql_timeout_secs, Some(15));
    assert_eq!(settings.run_scope, RunScope::All);
    assert_eq!(settings.line_numbers, Some(LineNumbers::Absolute));
    assert_eq!(settings.indent, crate::settings::Indent::Four);
    assert_eq!(settings.autocomplete, Autocomplete::Manual);
    assert_eq!(settings.keywords, Some(KeywordCase::Upper));
    assert_eq!(settings.commas, crate::settings::Commas::Leading);
    assert!(!settings.close_pairs);
    assert!(settings.wrap);
    assert_eq!(settings_saves(&harness), saves + 11);
    // The file has each of them, and R puts the cursor's back.
    let text = harness.app.settings_file.text.clone();
    assert!(
        text.contains("[editor.run]\nnew_tab = \"connection\""),
        "{text}"
    );
    assert!(text.contains("autocomplete = \"manual\""), "{text}");
    harness.press(Key::R, Modifiers::SHIFT);
    harness.settle();
    assert_eq!(
        harness.app.settings.commas,
        crate::settings::Commas::Trailing
    );
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
