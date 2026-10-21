//! The keymap the app runs with: the defaults with the keys of
//! `settings.toml` laid over them, read again whenever the file changes,
//! and the lines it does not take reported by their numbers.

use super::*;
use crate::backend::Backend;
use crate::keymap::{Command, Layout, Row, Scope, When, Why};

/// An app in the macOS look over a directory of its own.
fn mac_app(dir: &std::path::Path) -> App {
    let mut app = App::new(
        AppDirs::at(dir),
        Settings::default().into(),
        Backend::recording(),
    );
    app.look = crate::theme::Look::macos();
    app.refresh_keymap();
    app
}

/// The file as someone else wrote it, taken in as the watch brings it.
fn edited(app: &mut App, text: &str) {
    app.apply(Action::Backend(crate::backend::Event::SettingsFile {
        text: text.into(),
        own: false,
    }));
}

fn format(app: &App) -> String {
    app.keymap
        .key_in(Layout::Mac, Command::Format, Scope::SqlEditor)
        .as_str()
        .to_owned()
}

#[test]
fn a_key_written_by_hand_is_read_live() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = mac_app(dir.path());
    assert_eq!(format(&app), "⇧⌘F");
    edited(&mut app, "[keys.editor]\n\"cmd+alt+f\" = \"format\"\n");
    assert_eq!(format(&app), "⌥⌘F");
    assert!(app.settings_file.invalid.is_empty());
    // Taken out again: the default is back.
    edited(&mut app, "[data]\npage_size = 500\n");
    assert_eq!(format(&app), "⇧⌘F");
}

#[test]
fn a_line_the_keymap_cannot_take_is_ignored_by_its_number_and_the_default_stays() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = mac_app(dir.path());
    let text = "\
[keys.editor]
\"cmd+alt+f\" = \"no_such_command\"
\"cmd+nosuchkey\" = \"format\"
\"cmd+q\" = \"format\"
\"cmd+shift+enter\" = \"format\"
";
    edited(&mut app, text);
    assert_eq!(app.settings_file.invalid, [2, 3, 4, 5]);
    let whys: Vec<&Why> = app
        .keys_refused
        .iter()
        .map(|refused| &refused.why)
        .collect();
    let run_all = Row {
        command: Command::RunAll,
        scope: Scope::SqlEditor,
        when: When::Always,
    };
    assert_eq!(
        whys,
        [
            &Why::Reserved(crate::keymap::Owner::MacOs),
            &Why::Taken(run_all),
        ]
    );
    // The defaults stay.
    assert_eq!(format(&app), "⇧⌘F");
    let run = app
        .keymap
        .key_in(Layout::Mac, Command::RunAll, Scope::SqlEditor);
    assert_eq!(run, "⇧⌘↩");
    // With Run all given another key in the same file, the two lines
    // stand together: a swap written by hand.
    edited(&mut app, &format!("{text}\"cmd+alt+r\" = \"run_all\"\n"));
    assert_eq!(app.settings_file.invalid, [2, 3, 4]);
    assert_eq!(format(&app), "⇧⌘↩");
    let run = app
        .keymap
        .key_in(Layout::Mac, Command::RunAll, Scope::SqlEditor);
    assert_eq!(run, "⌥⌘R");
}

#[test]
fn a_change_made_in_the_app_writes_its_line_and_reset_all_takes_the_tables_away() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = mac_app(dir.path());
    edited(&mut app, "# mine\n[data]\npage_size = 500\n");
    let row = Row {
        command: Command::Format,
        scope: Scope::SqlEditor,
        when: When::Always,
    };
    let keys = app.settings.keys.record(Layout::Mac, row, "cmd+alt+f");
    app.change_settings(|settings| settings.keys = keys);
    assert_eq!(
        app.settings_file.text,
        "# mine\n[data]\npage_size = 500\n\n[keys.editor]\n\"cmd+alt+f\" = \"format\"\n"
    );
    assert_eq!(format(&app), "⌥⌘F");
    assert!(matches!(
        crate::testing::last_sent(&app),
        crate::backend::Command::Save { .. }
    ));
    app.reset_all_keys();
    assert_eq!(app.settings_file.text, "# mine\n[data]\npage_size = 500\n");
    assert_eq!(format(&app), "⇧⌘F");
    assert_eq!(app.settings.page_size, 500);
}

#[test]
fn the_keymap_is_made_again_for_the_look_it_is_drawn_in() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = mac_app(dir.path());
    edited(&mut app, "[keys.grid]\n\"gx\" = \"set_null\"\n");
    // macOS has no typed keys for Set NULL: refused there.
    assert_eq!(app.settings_file.invalid, [2]);
    app.look = crate::theme::Look::omarchy();
    app.refresh_keymap();
    let null = Row {
        command: Command::SetNull,
        scope: Scope::Grid,
        when: When::Always,
    };
    assert_eq!(app.keymap.keys_of(Layout::Omarchy, null), ["gx"]);
}
