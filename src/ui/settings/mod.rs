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

/// What one of the screen's keys asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asked {
    /// Open the file in the editor.
    Edit,
    Close,
    /// Move the cursor by this many rows.
    Move(isize),
    /// Put the cursor's option back to its default.
    Reset,
    /// Step the cursor's option to its next value, or to the one before.
    Step {
        forward: bool,
    },
    /// Flip the cursor's option, if it has two values.
    Flip,
}

/// What `key` held with `modifiers` asks of the screen, if it is one of
/// its keys. A button of the screen that has the keyboard (`free` is then
/// false) keeps Space, which presses it, and the arrows, which move focus
/// from it, as in the workspace. The letters, `R`, `ctrl+e` and Escape are
/// the screen's wherever the keyboard is.
fn asked(key: egui::Key, modifiers: egui::Modifiers, free: bool) -> Option<Asked> {
    use egui::{Key, Modifiers};
    // As egui matches a key: an extra Shift or Alt is passed over.
    let plain = modifiers.matches_logically(Modifiers::NONE);
    match key {
        Key::E if modifiers.matches_logically(Modifiers::CTRL) => Some(Asked::Edit),
        // Shift, which a plain key's match would pass over.
        Key::R if modifiers.matches_logically(Modifiers::SHIFT) => Some(Asked::Reset),
        _ if !plain => None,
        Key::Escape => Some(Asked::Close),
        Key::J => Some(Asked::Move(1)),
        Key::K => Some(Asked::Move(-1)),
        Key::L => Some(Asked::Step { forward: true }),
        Key::H => Some(Asked::Step { forward: false }),
        Key::ArrowDown if free => Some(Asked::Move(1)),
        Key::ArrowUp if free => Some(Asked::Move(-1)),
        Key::ArrowRight if free => Some(Asked::Step { forward: true }),
        Key::ArrowLeft if free => Some(Asked::Step { forward: false }),
        Key::Space if free => Some(Asked::Flip),
        _ => None,
    }
}

/// The screen's keys. Nothing under the screen has them first: the app
/// runs none of the workspace's shortcuts while a dialog is open, and egui
/// takes the keyboard from what is under a modal.
///
/// Several keys can come in one frame, and each acts on what the ones
/// before it left: the cursor where they moved it, an option as they set
/// it. `j` then `l` changes the row `j` moved to, and `l` twice steps
/// twice.
fn keys(app: &App, ctx: &egui::Context, row: usize, actions: &mut Vec<Action>) {
    let free = !super::focus::on_control(ctx);
    let last = OptionId::ALL.len() - 1;
    // As `App::apply` will have them once it has run the actions so far.
    let mut cursor = row;
    let mut settings = app.settings.clone();
    ctx.input_mut(|input| {
        input.events.retain(|event| {
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                repeat,
                ..
            } = event
            else {
                return true;
            };
            let Some(asked) = asked(*key, *modifiers, free) else {
                return true;
            };
            match asked {
                // A fresh press only: every repeat of a key held down
                // would start another editor.
                Asked::Edit => {
                    if !repeat {
                        actions.push(Action::EditSettingsFile);
                    }
                }
                Asked::Close => actions.push(Action::CloseDialog),
                Asked::Move(by) => {
                    cursor = cursor.saturating_add_signed(by).min(last);
                    actions.push(Action::MoveSettingsRow(by));
                }
                Asked::Reset | Asked::Step { .. } | Asked::Flip => {
                    // A cursor on no row has no option to change: the key
                    // is left, and the ones above still close the screen
                    // or move it back onto a row.
                    let Some(option) = OptionId::ALL.get(cursor).copied() else {
                        return true;
                    };
                    let value = match asked {
                        Asked::Reset => Some(option.default_value()),
                        Asked::Step { forward } => Some(settings.stepped(option, forward)),
                        _ => settings.flipped(option),
                    };
                    if let Some(value) = value {
                        value.set(&mut settings);
                        actions.push(Action::SetOption(value));
                    }
                }
            }
            false
        });
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
