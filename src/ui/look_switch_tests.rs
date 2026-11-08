//! Settings › Appearance, its Interface: the OS's classic look or the
//! Omarchy experience, chosen on either screen or in the file, and drawn
//! from the next frames on.

use super::appearance_settings_tests::{on_the_pane, settings_saves, shows};
use crate::keymap::{Command, Layout};
use crate::model::{Dialog, SettingsPane};
use crate::settings::{LookChoice, OptionId, Settings};
use crate::testing::Harness;
use crate::theme::Look;
use crate::update::release::Os;

/// The settings changed by `change`, as an edit of the file changes them.
fn change(harness: &mut Harness, change: impl FnOnce(&mut Settings)) {
    let mut settings = harness.app.settings.clone();
    change(&mut settings);
    harness.app.apply_settings(settings);
    harness.settle();
}

/// The settings file as the app last wrote it.
fn file(harness: &Harness) -> &str {
    &harness.app.settings_file.text
}

/// The pane Settings shows and its row the board's keys act on.
fn shown_row(harness: &Harness) -> Option<(SettingsPane, usize)> {
    match &harness.app.dialog {
        Some(Dialog::Settings(dialog)) => Some((dialog.pane, dialog.row)),
        _ => None,
    }
}

/// Whether `harness` draws `look` as the settings choose it: its shape and
/// its keys (the text sizes are the settings').
fn draws(harness: &Harness, look: Look) -> bool {
    let drawn = harness.app.look;
    (drawn.name, drawn.terminal, drawn.keys) == (look.name, look.terminal, look.keys)
}

#[test]
fn the_sheet_switches_to_the_omarchy_look_at_once() {
    let mut harness = Harness::with_size(egui::vec2(1440.0, 1000.0));
    harness.set_look(Look::macos());
    on_the_pane(&mut harness);
    // Beside the theme's tiles, with its small print.
    assert!(shows(
        &mut harness,
        "Omarchy: keyboard-first, square, monospace"
    ));
    let saves = settings_saves(&harness);
    harness.click("Interface: Omarchy");
    harness.settle();
    assert_eq!(harness.app.settings.look, Some(LookChoice::Omarchy));
    assert!(file(&harness).contains("look = \"omarchy\"  # classic | omarchy\n"));
    assert_eq!(settings_saves(&harness), saves + 1);
    assert!(draws(&harness, Look::omarchy()));
    assert_eq!(harness.app.layout(), Layout::Omarchy);
    // The board, still on Appearance, its cursor on the interface's row.
    assert_eq!(shown_row(&harness), Some((SettingsPane::Appearance, 0)));
    assert!(shows(&mut harness, "classic is the system's own look"));
}

#[test]
fn the_board_switches_to_the_classic_look_at_once() {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(Look::omarchy());
    on_the_pane(&mut harness);
    // The cursor starts on the interface's row: h is classic.
    assert_eq!(shown_row(&harness), Some((SettingsPane::Appearance, 0)));
    super::tests::type_key(&mut harness, egui::Key::H, "h");
    harness.settle();
    assert_eq!(harness.app.settings.look, Some(LookChoice::Classic));
    assert!(file(&harness).contains("look = \"classic\"  # classic | omarchy\n"));
    let classic = Look::chosen(Some(LookChoice::Classic), Os::current());
    assert!(draws(&harness, classic));
    assert!(!harness.app.look.terminal);
    // The sheet, on Appearance, with the keyboard on the look's control.
    match &harness.app.dialog {
        Some(Dialog::Settings(dialog)) => {
            assert_eq!(dialog.pane, SettingsPane::Appearance);
            assert_eq!(dialog.asked, Some(OptionId::Look));
        }
        other => panic!("{other:?}"),
    }
    assert!(shows(
        &mut harness,
        "Omarchy: keyboard-first, square, monospace"
    ));
    let focused = harness
        .ctx
        .memory(|memory| memory.focused())
        .and_then(|id| harness.ctx.read_response(id))
        .map(|response| response.rect);
    let control = harness
        .painted_rect("Classic")
        .expect("the classic segment");
    assert!(
        focused.is_some_and(|rect| rect.intersects(control)),
        "{focused:?} {control:?}"
    );
}

#[test]
fn an_edit_of_the_file_switches_the_look_and_back() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    change(&mut harness, |settings| {
        settings.look = Some(LookChoice::Omarchy);
    });
    assert!(draws(&harness, Look::omarchy()));
    // Taken out: the OS's own look.
    change(&mut harness, |settings| settings.look = None);
    assert!(draws(&harness, Look::chosen(None, Os::current())));
}

#[test]
fn the_look_drawn_is_the_look_set_without_a_value() {
    // Choosing what is drawn while nothing is set writes nothing.
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(Look::omarchy());
    on_the_pane(&mut harness);
    let saves = settings_saves(&harness);
    super::tests::type_key(&mut harness, egui::Key::L, "l");
    assert_eq!(harness.app.settings.look, None);
    assert!(!file(&harness).contains("look ="));
    assert_eq!(settings_saves(&harness), saves);
    assert!(draws(&harness, Look::omarchy()));
}

#[test]
fn reset_to_defaults_keeps_the_look() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    // As the window sets them: written to the file.
    harness.app.change_settings(|settings| {
        settings.look = Some(LookChoice::Omarchy);
        settings.column_types = false;
    });
    on_the_pane(&mut harness);
    harness.app.apply(crate::model::Action::ResetSettings);
    harness
        .app
        .apply(crate::model::Action::ConfirmResetSettings(true));
    harness.settle();
    assert!(harness.app.settings.column_types);
    assert_eq!(harness.app.settings.look, Some(LookChoice::Omarchy));
    assert!(draws(&harness, Look::omarchy()));
}

#[test]
fn the_new_look_has_its_own_defaults_and_keeps_what_is_set() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    // Alternate rows unset: off on macOS, on in the terminal.
    assert!(!harness.app.look.with_grid(&harness.app.settings).zebra);
    change(&mut harness, |settings| {
        settings.look = Some(LookChoice::Omarchy);
    });
    assert!(harness.app.look.with_grid(&harness.app.settings).zebra);
    // Set, it stays as set in either look.
    change(&mut harness, |settings| {
        settings.zebra = Some(false);
        settings.look = Some(LookChoice::Classic);
    });
    assert!(!harness.app.look.with_grid(&harness.app.settings).zebra);
    change(&mut harness, |settings| {
        settings.look = Some(LookChoice::Omarchy);
    });
    assert!(!harness.app.look.with_grid(&harness.app.settings).zebra);
}

#[test]
fn the_classic_look_writes_its_keys_as_its_os_has_them() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    change(&mut harness, |settings| {
        settings.look = Some(LookChoice::Classic);
    });
    let written = crate::ui::keys::written(&harness.ctx, &harness.app.look, Command::FindObject);
    let written = written.as_str();
    // ⌘ on macOS alone: Linux's classic look is macOS's with Ctrl.
    if Os::current() == Os::MacOs {
        assert!(written.contains('⌘'), "{written}");
    } else {
        assert!(!written.contains('⌘'), "{written}");
        assert!(written.contains("Ctrl"), "{written}");
    }
}

#[test]
fn choosing_again_before_a_switch_lands_draws_every_frame() {
    // The fonts of both looks are in place for the frame between: a change
    // back, from either screen or the file, a frame after the first.
    for (from, to) in [
        (Look::macos(), LookChoice::Omarchy),
        (Look::omarchy(), LookChoice::Classic),
    ] {
        let mut harness = Harness::new();
        harness.set_look(from);
        harness.settle();
        let back = LookChoice::of(from.terminal);
        harness.app.apply(crate::model::Action::SetOption(
            crate::settings::OptionValue::Look(Some(to)),
        ));
        harness.frame(Vec::new());
        let mut settings = harness.app.settings.clone();
        settings.look = Some(back);
        harness.app.apply_settings(settings);
        harness.frame(Vec::new());
        harness.frame(Vec::new());
        harness.settle();
        assert_eq!(harness.app.look.terminal, from.terminal, "{}", from.name);
        // And on to the other once more, the frames in between drawn.
        harness.app.apply(crate::model::Action::SetOption(
            crate::settings::OptionValue::Look(Some(to)),
        ));
        harness.settle();
        assert_eq!(harness.app.look.terminal, to.terminal(), "{}", from.name);
    }
}

#[test]
fn a_switch_that_changes_the_keybindings_starts_each_editor_over() {
    // The Omarchy look's keys are vim's while none are set: a new tab
    // types, in insert mode.
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let tab = harness.connect_fake();
    harness.press(egui::Key::T, egui::Modifiers::COMMAND);
    harness.settle();
    let mode = |harness: &Harness| {
        let workspace = harness.app.workspace(tab).expect("a workspace");
        workspace.active_sql_tab().expect("an editor").vim.mode
    };
    assert_eq!(mode(&harness), crate::vim::Mode::Insert);
    // The classic look's are the standard ones: vim starts over.
    change(&mut harness, |settings| {
        settings.look = Some(LookChoice::Classic);
    });
    assert_eq!(mode(&harness), crate::vim::Mode::Normal);
}
