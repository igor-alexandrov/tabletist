//! The Settings window: every option of the General tab, changed where it
//! is shown and saved at once. The terminal look draws a screen over the
//! whole window with its keys in the footer (`terminal.rs`); the other
//! looks get their window in a later step.

mod file_pane;
mod terminal;

use crate::app::App;
use crate::model::{Action, Dialog};
use crate::settings::{OptionId, OptionValue, Settings, Timestamps};

/// The label of an option's row. English: the caller translates, and the
/// terminal look lower-cases.
pub(super) fn label(option: OptionId) -> &'static str {
    match option {
        OptionId::PageSize => "Rows per page",
        OptionId::Timestamps => "Timestamps",
        OptionId::GroupDigits => "Numbers",
        OptionId::ValueTags => "Value tags",
    }
}

/// The values a click can set on `option`'s row, each with the words that
/// name it: what the row draws as segments or as a check, and what a
/// screen reader and the tests call them.
pub(super) fn choices(option: OptionId, settings: &Settings) -> Vec<(&'static str, OptionValue)> {
    match option {
        // Stepped, not listed: one way and the other.
        OptionId::PageSize => vec![
            ("fewer", settings.stepped(option, false)),
            ("more", settings.stepped(option, true)),
        ],
        OptionId::Timestamps => vec![
            ("second", OptionValue::Timestamps(Timestamps::Second)),
            ("full", OptionValue::Timestamps(Timestamps::Full)),
        ],
        OptionId::GroupDigits => vec![
            ("1,240.50", OptionValue::GroupDigits(true)),
            ("1240.50", OptionValue::GroupDigits(false)),
        ],
        OptionId::ValueTags => vec![
            ("off", OptionValue::ValueTags(false)),
            ("on", OptionValue::ValueTags(true)),
        ],
    }
}

/// The screen's keys, taken before it draws so that nothing under it and
/// no focused button in it reads them first.
fn keys(app: &App, ctx: &egui::Context, row: usize, actions: &mut Vec<Action>) {
    use egui::{Key, Modifiers};
    let Some(option) = OptionId::ALL.get(row).copied() else {
        return;
    };
    let settings = &app.settings;
    ctx.input_mut(|input| {
        // A fresh press only: every repeat of a key held down would start
        // another editor.
        if super::keys::consume_press(input, Modifiers::CTRL, Key::E) {
            actions.push(Action::EditSettingsFile);
        }
        let mut pressed = |modifiers, key| input.consume_key(modifiers, key);
        // Shift first: egui ignores an extra Shift when it matches a key.
        if pressed(Modifiers::SHIFT, Key::R) {
            actions.push(Action::SetOption(option.default_value()));
        }
        if pressed(Modifiers::NONE, Key::J) || pressed(Modifiers::NONE, Key::ArrowDown) {
            actions.push(Action::MoveSettingsRow(1));
        }
        if pressed(Modifiers::NONE, Key::K) || pressed(Modifiers::NONE, Key::ArrowUp) {
            actions.push(Action::MoveSettingsRow(-1));
        }
        if pressed(Modifiers::NONE, Key::L) || pressed(Modifiers::NONE, Key::ArrowRight) {
            actions.push(Action::SetOption(settings.stepped(option, true)));
        }
        if pressed(Modifiers::NONE, Key::H) || pressed(Modifiers::NONE, Key::ArrowLeft) {
            actions.push(Action::SetOption(settings.stepped(option, false)));
        }
        if pressed(Modifiers::NONE, Key::Space)
            && let Some(value) = settings.flipped(option)
        {
            actions.push(Action::SetOption(value));
        }
        if pressed(Modifiers::NONE, Key::Escape) {
            actions.push(Action::CloseDialog);
        }
    });
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(Dialog::Settings(dialog)) = &app.dialog else {
        return;
    };
    let row = dialog.row;
    let mut actions = Vec::new();
    keys(app, ctx, row, &mut actions);
    terminal::show(app, ctx, row, &mut actions);
    app.actions.extend(actions);
}
