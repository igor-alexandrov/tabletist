//! The keyboard shortcuts dialog's search (`?`), headless: what it keeps
//! of the list by words, typed keys and a chord pressed, what it says when
//! it keeps nothing, Esc taking the search back before it closes, and
//! the field having the keyboard from the start, in each look.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers};

use crate::i18n::Locale;
use crate::keymap::Command;
use crate::model::{Action, Dialog, HelpSearch};
use crate::testing::Harness;
use crate::theme::Look;

/// Command held, as each look's window reports it: ⌘ on macOS, Ctrl
/// elsewhere.
fn command_held(look: &Look) -> Modifiers {
    if look == &Look::macos() {
        Modifiers::MAC_CMD | Modifiers::COMMAND
    } else {
        Modifiers::CTRL | Modifiers::COMMAND
    }
}

/// The dialog open over the start screen in `look`.
fn opened(look: Look) -> Harness {
    let mut harness = Harness::new();
    harness.set_look(look);
    harness.app.apply(Action::ShowHelp);
    harness.finish_animations();
    harness
}

/// The dialog's search, or a failure if the dialog is not open.
fn search(harness: &Harness) -> (String, Option<&'static str>) {
    match &harness.app.dialog {
        Some(Dialog::Help(help)) => (help.search.clone(), help.found),
        other => panic!("the shortcuts are not open: {other:?}"),
    }
}

/// The commands the dialog lists for `text` typed and `found` pressed.
fn listed(harness: &Harness, text: &str, found: Option<&'static str>) -> Vec<Command> {
    let search = HelpSearch {
        search: text.into(),
        found,
    };
    let layout = harness.app.layout();
    crate::ui::help::listed(&harness.app.keymap, layout, Locale::default(), &search)
        .into_iter()
        .map(|row| row.command)
        .collect()
}

/// Types `text` as the window sends it.
fn type_text(harness: &mut Harness, text: &str) {
    harness.frame(vec![Event::Text(text.into())]);
    harness.settle();
}

/// Where the Close button is: the dialog's foot.
fn close_button_top(harness: &mut Harness) -> f64 {
    let tree = harness.settle();
    let id = crate::testing::node(&tree, "Close", Role::Button).expect("a Close button");
    let (_, node) = tree.nodes.iter().find(|(at, _)| *at == id).unwrap();
    node.bounds().expect("bounds").y0
}

#[test]
fn the_search_is_the_dialogs_own_and_a_find_drops_the_words() {
    let mut harness = Harness::new();
    harness.app.apply(Action::ShowHelp);
    harness.app.apply(Action::HelpSearch("row".into()));
    assert_eq!(search(&harness), ("row".into(), None));
    harness.app.apply(Action::HelpFind(Some("cmd+r")));
    assert_eq!(search(&harness), (String::new(), Some("cmd+r")));
    harness.app.apply(Action::HelpSearch("re".into()));
    assert_eq!(search(&harness), ("re".into(), None));
    // Opened again, it starts with nothing.
    harness.app.apply(Action::CloseDialog);
    harness.app.apply(Action::ShowHelp);
    assert_eq!(search(&harness), (String::new(), None));
}

#[test]
fn words_typed_keys_and_chords_cut_the_list_in_every_look() {
    for (look, typed) in [
        (Look::macos(), ["cmd+r", "⌘R", "cmd r"]),
        (Look::standard(), ["ctrl+r", "Ctrl-R", "ctrl r"]),
        (Look::omarchy(), ["R", "R", "R"]),
    ] {
        let harness = opened(look);
        let name = look.name;
        let layout = harness.app.layout();
        let all = listed(&harness, "", None);
        let rows = crate::ui::help::rows(&harness.app.keymap, layout);
        assert_eq!(all.len(), rows.len(), "{name}");
        // By the command's name, in any case.
        let row = listed(&harness, "ROW", None);
        assert!(row.contains(&Command::AddRow), "{name}: {row:?}");
        assert!(!row.contains(&Command::Reload), "{name}: {row:?}");
        // By its group's name.
        let sql = listed(&harness, "sql editor", None);
        assert!(sql.contains(&Command::RunStatement), "{name}: {sql:?}");
        // By its keys as the look writes them, or as they are spelled.
        for text in typed {
            let keys = listed(&harness, text, None);
            assert!(keys.contains(&Command::Reload), "{name}, {text}: {keys:?}");
            // One letter is a word too, in many names.
            if text.chars().count() > 1 {
                assert!(!keys.contains(&Command::AddRow), "{name}, {text}: {keys:?}");
            }
        }
        // By a chord pressed: Find any object's own.
        let keymap = &harness.app.keymap;
        let find = keymap.command_keys(layout, Command::FindObject);
        let chord = find
            .iter()
            .find(|keys| keys.contains('+'))
            .copied()
            .unwrap();
        let found = listed(&harness, "", Some(chord));
        assert!(found.contains(&Command::FindObject), "{name}: {found:?}");
        // And by the same chord typed, as the look writes it or spells it.
        let written = crate::keymap::key_written(layout, chord);
        for text in [chord, written.as_str()] {
            let typed = listed(&harness, text, None);
            assert!(
                typed.contains(&Command::FindObject),
                "{name}, {text}: {typed:?}"
            );
            assert!(
                !typed.contains(&Command::AddRow),
                "{name}, {text}: {typed:?}"
            );
        }
        assert!(found.len() < all.len(), "{name}");
        assert!(listed(&harness, "zzz", None).is_empty(), "{name}");
    }
}

#[test]
fn the_field_has_the_keyboard_as_the_dialog_opens() {
    for look in Look::ALL {
        let mut harness = opened(look);
        type_text(&mut harness, "row");
        assert_eq!(search(&harness).0, "row", "{}", look.name);
        assert!(harness.has("Add row"), "{}", look.name);
        assert!(!harness.has("Reload"), "{}", look.name);
    }
}

#[test]
fn one_letter_commands_type_into_the_field() {
    for look in Look::ALL {
        let mut harness = opened(look);
        super::tests::type_key(&mut harness, Key::Questionmark, "?");
        super::tests::type_key(&mut harness, Key::R, "R");
        super::tests::type_key(&mut harness, Key::Slash, "/");
        super::tests::type_key(&mut harness, Key::T, "t");
        assert_eq!(search(&harness).0, "?R/t", "{}", look.name);
    }
}

#[test]
fn a_chord_pressed_in_the_field_finds_its_commands() {
    for (look, found, said) in [
        (Look::macos(), "cmd+r", "Commands bound to ⌘R"),
        (Look::standard(), "cmd+r", "Commands bound to Ctrl+R"),
        (Look::omarchy(), "ctrl+r", "commands bound to ctrl+r"),
    ] {
        let mut harness = opened(look);
        type_text(&mut harness, "row");
        harness.press(Key::R, command_held(&look));
        assert_eq!(
            search(&harness),
            (String::new(), Some(found)),
            "{}",
            look.name
        );
        assert!(harness.has(said), "{}", look.name);
        let listed = listed(&harness, "", Some(found));
        assert!(!listed.is_empty(), "{}", look.name);
        if look != Look::omarchy() {
            assert!(harness.has("Reload"), "{}", look.name);
        }
        // Show all lists them all again.
        harness.app.apply(Action::HelpFind(None));
        harness.settle();
        assert!(harness.has("Add row"), "{}", look.name);
    }
}

#[test]
fn the_fields_editing_chords_edit_its_text() {
    for look in Look::ALL {
        let name = look.name;
        let mut harness = opened(look);
        type_text(&mut harness, "row");
        // Select all, then type over it.
        harness.press(Key::A, command_held(&look));
        assert_eq!(search(&harness), ("row".into(), None), "{name}");
        type_text(&mut harness, "sql");
        assert_eq!(search(&harness), ("sql".into(), None), "{name}");
        // Ctrl+U deletes back to the line's start.
        harness.press(Key::U, Modifiers::CTRL);
        assert_eq!(search(&harness), (String::new(), None), "{name}");
    }
}

#[test]
fn a_text_size_chord_is_found_not_done() {
    for (look, found) in [
        (Look::macos(), "cmd+="),
        (Look::standard(), "cmd+="),
        (Look::omarchy(), "ctrl+="),
    ] {
        let mut harness = opened(look);
        let sizes = |harness: &Harness| {
            let settings = &harness.app.settings;
            format!("{:?} {:?}", settings.ui_scale, settings.font_size)
        };
        let before = sizes(&harness);
        harness.press(Key::Equals, command_held(&look));
        assert_eq!(
            search(&harness),
            (String::new(), Some(found)),
            "{}",
            look.name
        );
        assert_eq!(sizes(&harness), before, "{}", look.name);
    }
}

#[test]
fn what_an_alt_chord_types_does_not_drop_it() {
    let mut harness = opened(Look::macos());
    type_text(&mut harness, "row");
    let alt = Modifiers::ALT;
    harness.frame(vec![
        crate::testing::key(Key::R, alt),
        Event::Text("®".into()),
    ]);
    harness.frame(vec![crate::testing::release(Key::R, alt)]);
    harness.settle();
    assert_eq!(search(&harness), (String::new(), Some("alt+r")));
    // A chord with Cmd types nothing: text after it is typed.
    let cmd = command_held(&Look::macos());
    harness.frame(vec![
        crate::testing::key(Key::R, cmd),
        Event::Text("x".into()),
    ]);
    harness.frame(vec![crate::testing::release(Key::R, cmd)]);
    harness.settle();
    assert_eq!(search(&harness).0, "x");
}

#[test]
fn a_search_that_keeps_nothing_says_so() {
    for look in Look::ALL {
        let mut harness = opened(look);
        type_text(&mut harness, "zzz");
        assert!(harness.has("No commands match “zzz”") || harness.has("no commands match “zzz”"));
        assert!(!harness.has("Add row"), "{}", look.name);
    }
}

#[test]
fn esc_takes_the_search_back_then_closes() {
    for look in Look::ALL {
        let name = look.name;
        let mut harness = opened(look);
        type_text(&mut harness, "row");
        harness.press(Key::Escape, Modifiers::NONE);
        assert_eq!(search(&harness), (String::new(), None), "{name}");
        // The field kept the keyboard.
        type_text(&mut harness, "sql");
        assert_eq!(search(&harness).0, "sql", "{name}");
        harness.press(Key::Escape, Modifiers::NONE);
        // A chord looked for is taken back the same way.
        harness.press(Key::R, command_held(&look));
        assert!(search(&harness).1.is_some(), "{name}");
        harness.press(Key::Escape, Modifiers::NONE);
        assert_eq!(search(&harness), (String::new(), None), "{name}");
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none(), "{name}");
    }
}

#[test]
fn the_dialog_keeps_its_size_while_it_is_searched() {
    for look in Look::ALL {
        let mut harness = opened(look);
        let before = close_button_top(&mut harness);
        for text in ["zzz", "row"] {
            harness.app.apply(Action::HelpSearch(text.into()));
            harness.finish_animations();
            assert_eq!(
                close_button_top(&mut harness),
                before,
                "{}, {text}",
                look.name
            );
        }
        // A chord looked for: its line takes its place from the list's.
        harness.press(Key::R, command_held(&look));
        harness.finish_animations();
        assert!(search(&harness).1.is_some(), "{}", look.name);
        assert_eq!(
            close_button_top(&mut harness),
            before,
            "{}, a chord",
            look.name
        );
    }
}
