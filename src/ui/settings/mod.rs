//! The Settings window: every option of the General tab, changed where it
//! is shown and saved at once. The terminal look draws a screen over the
//! whole window with its keys in the footer (`terminal.rs`); the other
//! looks get their window in a later step.

mod file_pane;
mod terminal;

use std::borrow::Cow;

use crate::app::App;
use crate::i18n::{Locale, gettext};
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

/// What a choice is called.
#[derive(Clone, Copy)]
pub(super) enum Said {
    /// A word. English: whoever writes it translates it.
    Word(&'static str),
    /// A value as the app shows one, the same in every language: put into
    /// another, it would say something the app does not show.
    Sample(&'static str),
}

impl Said {
    /// What a screen reader and the tests call the choice.
    pub(super) fn name(self, locale: Locale) -> Cow<'static, str> {
        match self {
            Self::Word(word) => gettext(locale, word),
            Self::Sample(sample) => Cow::Borrowed(sample),
        }
    }
}

/// The values a click can set on `option`'s row, each with what it is
/// called: what the row draws as segments or as a check, and what a screen
/// reader and the tests call them.
pub(super) fn choices(option: OptionId, settings: &Settings) -> Vec<(Said, OptionValue)> {
    use Said::{Sample, Word};
    match option {
        // Stepped, not listed: one way and the other.
        OptionId::PageSize => vec![
            (Word("fewer"), settings.stepped(option, false)),
            (Word("more"), settings.stepped(option, true)),
        ],
        OptionId::Timestamps => vec![
            (Word("second"), OptionValue::Timestamps(Timestamps::Second)),
            (Word("full"), OptionValue::Timestamps(Timestamps::Full)),
        ],
        // A number as each value writes it.
        OptionId::GroupDigits => vec![
            (Sample("1,240.50"), OptionValue::GroupDigits(true)),
            (Sample("1240.50"), OptionValue::GroupDigits(false)),
        ],
        OptionId::ValueTags => vec![
            (Word("off"), OptionValue::ValueTags(false)),
            (Word("on"), OptionValue::ValueTags(true)),
        ],
    }
}

/// The screen's keys. Nothing under the screen has them first: the app
/// runs none of the workspace's shortcuts while a dialog is open, and egui
/// takes the keyboard from what is under a modal. A button of the screen
/// that has the keyboard keeps Space, which presses it, and the arrows,
/// which move focus from it, as in the workspace. The letters, `R`,
/// `ctrl+e` and Escape are the screen's wherever the keyboard is.
fn keys(app: &App, ctx: &egui::Context, row: usize, actions: &mut Vec<Action>) {
    use egui::{Key, Modifiers};
    let option = OptionId::ALL.get(row).copied();
    let settings = &app.settings;
    let free = !super::focus::on_control(ctx);
    ctx.input_mut(|input| {
        // A fresh press only: every repeat of a key held down would start
        // another editor.
        if super::keys::consume_press(input, Modifiers::CTRL, Key::E) {
            actions.push(Action::EditSettingsFile);
        }
        let mut pressed = |modifiers, key| input.consume_key(modifiers, key);
        if pressed(Modifiers::NONE, Key::Escape) {
            actions.push(Action::CloseDialog);
        }
        if pressed(Modifiers::NONE, Key::J) || (free && pressed(Modifiers::NONE, Key::ArrowDown)) {
            actions.push(Action::MoveSettingsRow(1));
        }
        if pressed(Modifiers::NONE, Key::K) || (free && pressed(Modifiers::NONE, Key::ArrowUp)) {
            actions.push(Action::MoveSettingsRow(-1));
        }
        // The keys below change the cursor's option. A cursor on no row
        // has none, and the keys above still close the screen or move it
        // back onto one.
        let Some(option) = option else {
            return;
        };
        // Shift first: egui ignores an extra Shift when it matches a key.
        if pressed(Modifiers::SHIFT, Key::R) {
            actions.push(Action::SetOption(option.default_value()));
        }
        if pressed(Modifiers::NONE, Key::L) || (free && pressed(Modifiers::NONE, Key::ArrowRight)) {
            actions.push(Action::SetOption(settings.stepped(option, true)));
        }
        if pressed(Modifiers::NONE, Key::H) || (free && pressed(Modifiers::NONE, Key::ArrowLeft)) {
            actions.push(Action::SetOption(settings.stepped(option, false)));
        }
        if free
            && pressed(Modifiers::NONE, Key::Space)
            && let Some(value) = settings.flipped(option)
        {
            actions.push(Action::SetOption(value));
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
