//! The Settings window: the options of its panes, General and, where
//! agents can reach the app, Agents, each changed where it is shown and
//! saved at once. The terminal look draws a screen over the whole window
//! with its keys in the footer (`terminal.rs`); the other looks draw a
//! sheet over the dimmed window, with a control for each option
//! (`sheet.rs`, and `agents.rs` for the Agents pane).

mod agents;
mod file_pane;
mod sheet;
mod terminal;

use std::borrow::Cow;

use crate::app::App;
use crate::i18n::{Locale, gettext, ngettext};
use crate::keymap::{Command, Keymap, Layout, Scope, Spelled, Stroke};
use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, SettingsRow};
use crate::settings::{
    EnvBar, OptionId, OptionValue, RowHeight, Settings, TableList, Timestamps, Zone,
};

/// The label of an option's row. English: the caller translates, and the
/// terminal look lower-cases.
pub(super) fn label(option: OptionId) -> &'static str {
    match option {
        OptionId::PageSize => "Rows per page",
        OptionId::Timestamps => "Timestamps",
        OptionId::Tz => "Time zone for timestamptz",
        OptionId::GroupDigits => "Numbers",
        OptionId::ValueTags => "Value tags",
        OptionId::ProductionReadOnly => "New production connections open read-only",
        OptionId::ConfirmCloseEdits => "Ask before closing with unsaved edits",
        OptionId::AgentsOn => "Let agents use Tabletist",
        OptionId::LogDays => "Keep the activity log",
        OptionId::AgentsNotify => "Notify when an agent connects",
        OptionId::RowHeight => "Row density",
        OptionId::Zebra => "Alternate row shading",
        OptionId::ColumnTypes => "Show column types",
        OptionId::TableList => "Table list",
        OptionId::EnvBar => "Window header",
    }
}

/// The label of an option's row in the terminal look, which names some
/// options its own way, as their artboards do (`timestamptz zone`, the
/// Appearance pane's). English, as [`label`]'s, and lower-cased by the
/// look.
pub(super) fn terminal_label(option: OptionId) -> &'static str {
    match option {
        OptionId::Tz => "timestamptz zone",
        OptionId::ProductionReadOnly => "production opens read-only",
        OptionId::ConfirmCloseEdits => "confirm close with edits",
        OptionId::RowHeight => "Row height",
        OptionId::Zebra => "Alternate rows",
        OptionId::ColumnTypes => "Column types",
        OptionId::EnvBar => "Connection bar",
        OptionId::PageSize
        | OptionId::Timestamps
        | OptionId::GroupDigits
        | OptionId::ValueTags
        | OptionId::AgentsOn
        | OptionId::LogDays
        | OptionId::AgentsNotify
        | OptionId::TableList => label(option),
    }
}

/// What a row says under its label, where it says something. English.
pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
    match option {
        OptionId::PageSize => Some("Table view; the SQL editor has its own limit"),
        OptionId::ValueTags => Some("Colors for enum, CHECK (…IN…) and boolean columns"),
        OptionId::LogDays => Some("Every call and its SQL, never the rows. On this computer only"),
        OptionId::Tz => Some("Values are shown in this zone and sent with their offset"),
        OptionId::ProductionReadOnly => Some("Default for the switch in each connection"),
        OptionId::ColumnTypes => Some("Second line in headers: int8, varchar, jsonb"),
        OptionId::TableList => Some("Default for new connections; the sidebar toggle overrides it"),
        OptionId::EnvBar => Some("Production always uses the full tint"),
        OptionId::Timestamps
        | OptionId::GroupDigits
        | OptionId::ConfirmCloseEdits
        | OptionId::AgentsOn
        | OptionId::AgentsNotify
        | OptionId::RowHeight
        | OptionId::Zebra => None,
    }
}

/// A zone as the window writes it: the computer's with its own name
/// beside it where it has one (`System · America/New_York`), UTC, or the
/// zone's name. `system` is the computer's zone's name. English: the
/// terminal look writes it its own way.
pub(super) fn zone_text(locale: Locale, zone: &Zone, system: Option<&str>) -> String {
    match zone {
        Zone::System => match system {
            Some(name) => format!("{} · {name}", gettext(locale, "System")),
            None => gettext(locale, "System").into_owned(),
        },
        Zone::Utc => "UTC".to_owned(),
        Zone::Named(name) => name.clone(),
    }
}

/// The days the log keeps a call, as the window writes them.
pub(super) fn days(locale: Locale, days: u32) -> String {
    ngettext(locale, "{n} day", "{n} days", days).replace("{n}", &days.to_string())
}

/// Whether an option of on and off is on.
pub(super) fn is_on(value: &OptionValue) -> bool {
    matches!(
        value,
        OptionValue::ValueTags(true)
            | OptionValue::ProductionReadOnly(true)
            | OptionValue::ConfirmCloseEdits(true)
            | OptionValue::AgentsOn(true)
            | OptionValue::AgentsNotify(true)
            | OptionValue::Zebra(Some(true))
            | OptionValue::ColumnTypes(true)
    )
}

/// The settings as the window shows them: the server's switch as it is in
/// force, off where a change made outside the app that turned it on is
/// held, the grouping of numbers as the look has it where it is not set,
/// and the stripes as the look draws them while they are not set.
pub(super) fn shown(app: &App) -> Settings {
    let mut settings = app.settings.clone();
    settings.mcp_enabled = app.agents_on();
    settings.group_digits = Some(app.settings.grouped(app.layout()));
    settings.zebra = Some(settings.zebra_in(app.look.terminal));
    settings
}

/// A timestamp as the grid shows one at each precision.
pub(super) fn sample_timestamp(timestamps: Timestamps) -> &'static str {
    match timestamps {
        Timestamps::Second => "2026-01-12 09:14:03",
        Timestamps::Full => "2026-01-12 09:14:03.482915",
    }
}

/// The settings file's path as the window writes it: the home directory
/// as `~`. The home directory was found at the start, with the app's own:
/// no frame looks it up.
pub(super) fn path_shown(app: &App) -> String {
    file_pane::shown_path(&app.dirs.settings_file(), app.dirs.home.as_deref())
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
/// `named` is the last named zone the session had, which the zone's steps
/// reach.
pub(super) fn choices(
    option: OptionId,
    settings: &Settings,
    named: Option<&str>,
) -> Vec<(Said, OptionValue)> {
    use Said::{Sample, Word};
    match option {
        OptionId::Tz => vec![
            (Word("fewer"), settings.stepped_zone(named, false)),
            (Word("more"), settings.stepped_zone(named, true)),
        ],
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
            (Sample("1,240.50"), OptionValue::GroupDigits(Some(true))),
            (Sample("1240.50"), OptionValue::GroupDigits(Some(false))),
        ],
        OptionId::ValueTags => vec![
            (Word("off"), OptionValue::ValueTags(false)),
            (Word("on"), OptionValue::ValueTags(true)),
        ],
        OptionId::ProductionReadOnly => vec![
            (Word("off"), OptionValue::ProductionReadOnly(false)),
            (Word("on"), OptionValue::ProductionReadOnly(true)),
        ],
        OptionId::ConfirmCloseEdits => vec![
            (Word("off"), OptionValue::ConfirmCloseEdits(false)),
            (Word("on"), OptionValue::ConfirmCloseEdits(true)),
        ],
        OptionId::AgentsOn => vec![
            (Word("off"), OptionValue::AgentsOn(false)),
            (Word("on"), OptionValue::AgentsOn(true)),
        ],
        OptionId::LogDays => vec![
            (Word("fewer"), settings.stepped(option, false)),
            (Word("more"), settings.stepped(option, true)),
        ],
        OptionId::AgentsNotify => vec![
            (Word("off"), OptionValue::AgentsNotify(false)),
            (Word("on"), OptionValue::AgentsNotify(true)),
        ],
        OptionId::RowHeight => vec![
            (Word("compact"), OptionValue::RowHeight(RowHeight::Compact)),
            (Word("regular"), OptionValue::RowHeight(RowHeight::Regular)),
            (Word("roomy"), OptionValue::RowHeight(RowHeight::Roomy)),
        ],
        OptionId::Zebra => vec![
            (Word("off"), OptionValue::Zebra(Some(false))),
            (Word("on"), OptionValue::Zebra(Some(true))),
        ],
        OptionId::ColumnTypes => vec![
            (Word("off"), OptionValue::ColumnTypes(false)),
            (Word("on"), OptionValue::ColumnTypes(true)),
        ],
        OptionId::TableList => vec![
            (Word("tree"), OptionValue::TableList(TableList::Tree)),
            (Word("flat"), OptionValue::TableList(TableList::Flat)),
        ],
        OptionId::EnvBar => vec![
            (Word("tint + label"), OptionValue::EnvBar(EnvBar::Tint)),
            (Word("label only"), OptionValue::EnvBar(EnvBar::Label)),
        ],
    }
}

/// What Enter does on `row`, where it does something: the Agents tab, its
/// log, or Connections. Showing the tab needs a window: with none open,
/// nothing.
pub(super) fn opened(app: &App, row: SettingsRow) -> Option<Vec<Action>> {
    let show = |view| {
        app.agents_window()
            .map(|_| vec![Action::ShowAgentsFromSettings(view)])
    };
    match row {
        SettingsRow::AgentsStatus => show(AgentsView::Activity),
        SettingsRow::Option(OptionId::LogDays) => show(AgentsView::Log),
        SettingsRow::AgentsAccess => Some(vec![Action::CloseDialog, Action::ShowConnections]),
        SettingsRow::Option(_) => None,
    }
}

/// What a disabled Show Agents tab and Show log say, with no window open.
pub(super) const NO_WINDOW: &str = "Open a connection to see its agents";

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
    /// Show the next pane (`true`) or the one before.
    Pane {
        forward: bool,
    },
    /// Open what the cursor's row shows.
    Open,
    /// Copy the command that connects an agent.
    Copy,
}

/// What `event` asks of the screen, if it is a press of one of its keys,
/// as the keymap has them (`held` is what the frame holds). A button of
/// the screen that has the keyboard (`free` is then false) keeps Space,
/// which presses it, and the arrows, which move focus from it, as in the
/// workspace: the screen's plain keys but its close. Its letters, the
/// chord that opens the file and the key that closes are the screen's
/// wherever the keyboard is.
fn asked(keymap: &Keymap, event: &egui::Event, held: egui::Modifiers, free: bool) -> Option<Asked> {
    // The screen is Omarchy's.
    let layout = Layout::Omarchy;
    let commands = [
        Command::SettingsOpenFile,
        Command::SettingsReset,
        Command::SettingsClose,
        Command::SettingsMove,
        Command::SettingsChange,
        Command::SettingsToggle,
        Command::SettingsPane,
        Command::SettingsOpenRow,
        Command::SettingsCopyCommand,
    ];
    // A letter is read as the key that types it, with Shift for its
    // capital and nothing else held.
    let typed = match event {
        egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } if !(modifiers.command || modifiers.ctrl || modifiers.mac_cmd || modifiers.alt) => {
            crate::keymap::char_of(*key, modifiers.shift)
        }
        _ => None,
    };
    for command in commands {
        // A pane's two chords are the one before, then the next.
        for (nth, keys) in keymap
            .chords_now(layout, command, Scope::Prompt, true)
            .enumerate()
        {
            let pressed = match crate::keymap::spell(keys) {
                Ok(Spelled::Strokes(strokes)) => match strokes.as_slice() {
                    [Stroke::Text(letter)] => typed == Some(*letter),
                    [Stroke::Key { mods, .. }] => {
                        let plain = mods.none() && command != Command::SettingsClose;
                        (free || !plain)
                            && crate::ui::keys::press_of(layout, held, event, keys).is_some()
                    }
                    _ => false,
                },
                _ => false,
            };
            if !pressed {
                continue;
            }
            return Some(match command {
                Command::SettingsOpenFile => Asked::Edit,
                Command::SettingsReset => Asked::Reset,
                Command::SettingsClose => Asked::Close,
                Command::SettingsMove if matches!(keys, "k" | "up") => Asked::Move(-1),
                Command::SettingsMove => Asked::Move(1),
                Command::SettingsChange => Asked::Step {
                    forward: matches!(keys, "l" | "right"),
                },
                Command::SettingsPane => Asked::Pane { forward: nth == 1 },
                Command::SettingsOpenRow => Asked::Open,
                Command::SettingsCopyCommand => Asked::Copy,
                _ => Asked::Flip,
            });
        }
    }
    None
}

/// The screen's keys. Nothing under the screen has them first: the app
/// runs none of the workspace's shortcuts while a dialog is open, and egui
/// takes the keyboard from what is under a modal.
///
/// Several keys can come in one frame, and each acts on what the ones
/// before it left: the cursor where they moved it, an option as they set
/// it. `j` then `l` changes the row `j` moved to, and `l` twice steps
/// twice.
fn keys(app: &App, ctx: &egui::Context, dialog: &SettingsDialog, actions: &mut Vec<Action>) {
    let free = !super::focus::on_control(ctx);
    // As `App::apply` will have them once it has run the actions so far.
    let mut cursor = dialog.row;
    let mut pane = dialog.pane;
    let mut settings = shown(app);
    let keymap = app.keymap.clone();
    // Copied once the input is let go: the clipboard is the context's.
    let mut copy = false;
    ctx.input_mut(|input| {
        let held = input.modifiers;
        input.events.retain(|event| {
            let Some(asked) = asked(&keymap, event, held, free) else {
                return true;
            };
            let repeat = matches!(event, egui::Event::Key { repeat: true, .. });
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
                    let last = pane.rows().len() - 1;
                    cursor = cursor.saturating_add_signed(by).min(last);
                    actions.push(Action::MoveSettingsRow(by));
                }
                Asked::Pane { forward } => {
                    let listed = SettingsPane::listed();
                    let at = listed.iter().position(|shown| *shown == pane).unwrap_or(0);
                    let next = match forward {
                        true => (at + 1).min(listed.len() - 1),
                        false => at.saturating_sub(1),
                    };
                    if listed[next] != pane {
                        pane = listed[next];
                        cursor = 0;
                        actions.push(Action::ShowSettingsPane(pane));
                    }
                }
                // What the row opens, on a row that opens something: else
                // the key is left.
                Asked::Open => {
                    let opens = pane.rows().get(cursor).and_then(|row| opened(app, *row));
                    let Some(opens) = opens else {
                        return true;
                    };
                    // A fresh press only: a held Enter would go on to the
                    // screen it opened.
                    if !repeat {
                        actions.extend(opens);
                    }
                }
                Asked::Copy => {
                    if pane != SettingsPane::Agents {
                        return true;
                    }
                    copy |= !repeat;
                }
                Asked::Reset | Asked::Step { .. } | Asked::Flip => {
                    // A cursor on no option has no option to change: the
                    // key is left, and the ones above still close the
                    // screen or move it back onto a row.
                    let row = pane.rows().get(cursor);
                    let Some(option) = row.and_then(|row| row.option()) else {
                        return true;
                    };
                    // The server's switch has no default to put back:
                    // off, it would end every agent's session.
                    if asked == Asked::Reset && option == OptionId::AgentsOn {
                        return true;
                    }
                    let value = match asked {
                        Asked::Reset => Some(option.default_value()),
                        Asked::Step { forward } if option == OptionId::Tz => {
                            Some(settings.stepped_zone(app.named_zone.as_deref(), forward))
                        }
                        Asked::Step { forward } => Some(settings.stepped(option, forward)),
                        _ => settings.flipped(option),
                    };
                    if let Some(value) = value {
                        value.set(&mut settings);
                        // Stripes put back to the look's are drawn as the
                        // look draws them: the next key flips those.
                        settings.zebra = Some(settings.zebra_in(app.look.terminal));
                        actions.push(Action::SetOption(value));
                    }
                }
            }
            false
        });
    });
    if copy {
        crate::ui::agents::copy_command(ctx, app.locale);
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(Dialog::Settings(dialog)) = &app.dialog else {
        return;
    };
    let mut actions = Vec::new();
    if app.look.terminal {
        // The screen's keys act on its cursor. The sheet has none: its
        // controls are reached with Tab.
        keys(app, ctx, dialog, &mut actions);
        terminal::show(app, ctx, dialog, &mut actions);
    } else {
        sheet::show(app, ctx, dialog, &mut actions);
    }
    app.actions.extend(actions);
}
