//! The Settings window: the options of its panes, General and, where
//! agents can reach the app, Agents, each changed where it is shown and
//! saved at once. The terminal look draws a screen over the whole window
//! with its keys in the footer (`terminal.rs`); the other looks draw a
//! sheet over the dimmed window, with a control for each option
//! (`sheet.rs`, and `agents.rs` for the Agents pane).

mod agents;
mod file_pane;
mod preview;
mod sheet;
mod terminal;

use std::borrow::Cow;

use crate::app::App;
use crate::connections::SavedConnection;
use crate::i18n::{Locale, gettext, ngettext};
use crate::keymap::{Command, Keymap, Layout, Scope, Spelled, Stroke};
use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, SettingsRow};
use crate::settings::{
    Autocomplete, Commas, EditorKeys, EnvBar, Idle, InterfaceSize, KeywordCase, Launch,
    LineNumbers, Mode, NewTab, OptionId, OptionValue, RowHeight, RunScope, Settings, TableList,
    Theme, ThemeChoice, Timestamps, Zone,
};

/// The label of an option's row. English: the caller translates, and the
/// terminal look lower-cases.
pub(super) fn label(option: OptionId) -> &'static str {
    match option {
        OptionId::OnLaunch => "On launch",
        OptionId::RestoreTabs => "Restore tabs and filters",
        OptionId::PageSize => "Rows per page",
        OptionId::Timestamps => "Timestamps",
        OptionId::Tz => "Time zone for timestamptz",
        OptionId::GroupDigits => "Numbers",
        OptionId::ValueTags => "Value tags",
        OptionId::ProductionReadOnly => "New production connections open read-only",
        OptionId::ConfirmCloseEdits => "Ask before closing with unsaved edits",
        OptionId::IdleDisconnect => "Disconnect when idle",
        OptionId::AgentsOn => "Let agents use Tabletist",
        OptionId::LogDays => "Keep the activity log",
        OptionId::AgentsNotify => "Notify when an agent connects",
        OptionId::Theme => "Theme",
        OptionId::InterfaceSize => "Interface size",
        OptionId::Font => "Data font",
        OptionId::FontSize => "Data font size",
        OptionId::RowHeight => "Row density",
        OptionId::Zebra => "Alternate row shading",
        OptionId::ColumnTypes => "Show column types",
        OptionId::TableList => "Table list",
        OptionId::EnvBar => "Window header",
        OptionId::NewTab => "New tabs run in",
        OptionId::RowLimit => "Row limit",
        OptionId::Timeout => "Timeout",
        // `{run}` is the run key, as the keymap writes it: see `said`.
        OptionId::RunScope => "{run} runs",
        OptionId::ConfirmUnsafeWrites => "Confirm UPDATE or DELETE without WHERE",
        OptionId::EditorKeys => "Keybindings",
        OptionId::Indent => "Indent",
        OptionId::Autocomplete => "Autocomplete",
        OptionId::ClosePairs => "Close brackets and quotes",
        OptionId::LineNumbers => "Line numbers",
        OptionId::HighlightLine => "Current line highlight",
        OptionId::Wrap => "Wrap",
        OptionId::Keywords => "Keywords",
        OptionId::Commas => "Commas",
        OptionId::HistoryKeep => "Keep query history",
    }
}

/// A row's label as the window writes it, translated: with the run key in
/// place of `{run}`, as the keymap writes it for the look.
pub(super) fn said(
    ctx: &egui::Context,
    look: &crate::theme::Look,
    locale: Locale,
    text: &'static str,
) -> String {
    let said = gettext(locale, text);
    if !said.contains("{run}") {
        return said.into_owned();
    }
    let key = crate::ui::keys::written(ctx, look, Command::RunStatement);
    said.replace("{run}", key.as_str())
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
        OptionId::ConfirmUnsafeWrites => "Confirm write without where",
        OptionId::ClosePairs => "Close brackets",
        OptionId::HistoryKeep => "Keep",
        // One font and one size for everything.
        OptionId::Font => "Font",
        OptionId::FontSize => "Size",
        OptionId::OnLaunch
        | OptionId::RestoreTabs
        | OptionId::PageSize
        | OptionId::Timestamps
        | OptionId::GroupDigits
        | OptionId::ValueTags
        | OptionId::IdleDisconnect
        | OptionId::AgentsOn
        | OptionId::LogDays
        | OptionId::AgentsNotify
        | OptionId::Theme
        | OptionId::InterfaceSize
        | OptionId::TableList
        | OptionId::NewTab
        | OptionId::RowLimit
        | OptionId::Timeout
        | OptionId::RunScope
        | OptionId::EditorKeys
        | OptionId::Indent
        | OptionId::Autocomplete
        | OptionId::LineNumbers
        | OptionId::HighlightLine
        | OptionId::Wrap
        | OptionId::Keywords
        | OptionId::Commas => label(option),
    }
}

/// What a row says under its label, where it says something. English.
pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
    match option {
        // As the board writes it, though hiding columns is not built: what
        // the option will cover once it is.
        OptionId::RestoreTabs => Some("Per connection, including sort and hidden columns"),
        // The board's hint: beside the menu it has no room in the sheet.
        OptionId::OnLaunch => Some("Or: show Connections · open a chosen connection"),
        OptionId::PageSize => Some("Table view; the SQL editor has its own limit"),
        OptionId::ValueTags => Some("Colors for enum, CHECK (…IN…) and boolean columns"),
        OptionId::LogDays => Some("Every call and its SQL, never the rows. On this computer only"),
        OptionId::Tz => Some("Values are shown in this zone and sent with their offset"),
        OptionId::ProductionReadOnly => Some("Default for the switch in each connection"),
        OptionId::IdleDisconnect => Some("Closes server sessions; tabs stay and reconnect on use"),
        OptionId::ColumnTypes => Some("Second line in headers: int8, varchar, jsonb"),
        OptionId::TableList => Some("Default for new connections; the sidebar toggle overrides it"),
        OptionId::EnvBar => Some("Production always uses the full tint"),
        OptionId::InterfaceSize => Some("Sidebar, tabs, buttons, inspector labels"),
        OptionId::Font => Some("Grid cells, SQL editor, JSON"),
        OptionId::NewTab => Some("The badge in each tab can still switch it"),
        // Igor's words (2026-10-19), in place of the board's "Adds LIMIT to
        // results you browse; exports get everything": the rows are capped
        // as they come, nothing is added to the SQL, and Show all under a
        // cut result runs it again with no limit.
        OptionId::RowLimit => Some("Caps results you browse. Show all and exports get every row."),
        OptionId::ConfirmUnsafeWrites => Some("On every connection that allows writes"),
        OptionId::Autocomplete => Some("Tables, views, columns and functions from the schema"),
        OptionId::HistoryKeep => Some(
            "Per connection, on this computer: SQL editor, grid saves, filters and agent runs. Results are never stored.",
        ),
        OptionId::Timeout
        | OptionId::RunScope
        | OptionId::EditorKeys
        | OptionId::Indent
        | OptionId::ClosePairs
        | OptionId::LineNumbers
        | OptionId::HighlightLine
        | OptionId::Wrap
        | OptionId::Keywords
        | OptionId::Commas
        | OptionId::Timestamps
        | OptionId::GroupDigits
        | OptionId::ConfirmCloseEdits
        | OptionId::AgentsOn
        | OptionId::AgentsNotify
        | OptionId::Theme
        | OptionId::FontSize
        | OptionId::RowHeight
        | OptionId::Zebra => None,
    }
}

/// What describes `option`'s controls to a screen reader: its small print,
/// or what is written beside its control where it has none. English.
pub(super) fn helper(option: OptionId) -> Option<&'static str> {
    small_print(option).or(match option {
        OptionId::RunScope => Some("A selection always wins"),
        _ => None,
    })
}

/// What is said of a data font the settings name that is neither bundled
/// nor installed, `{name}` for its name: the default is drawn instead.
/// English.
pub const FONT_NOT_INSTALLED: &str = "“{name}” is not installed: IBM Plex Mono is used";

/// What is said of a pinned theme that is nowhere to be found, `{name}`
/// for its name: what is drawn instead. English.
pub fn pin_missing(catalog: &crate::theme::Catalog) -> &'static str {
    if catalog.follows_omarchy() {
        "Theme “{name}” not found: following Omarchy"
    } else {
        "Theme “{name}” not found: following the system"
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

/// What a launch can open, as the menu lists it and the screen steps
/// through it: the last session, Connections, then each saved connection
/// in the order they are saved.
pub(super) fn launch_choices(connections: &[SavedConnection]) -> Vec<Launch> {
    let mut choices = vec![Launch::Restore, Launch::Connections];
    choices.extend(connections.iter().map(|saved| Launch::Connection {
        id: saved.id.clone(),
        name: saved.name.clone(),
    }));
    choices
}

/// What a launch opens, as the window writes it: a connection by its name
/// as it is saved now. English but the name. One that is not saved (the
/// file names another) is said to be gone: the next launch shows
/// Connections in its place.
pub(super) fn launch_text(
    locale: Locale,
    launch: &Launch,
    connections: &[SavedConnection],
) -> String {
    match launch {
        Launch::Restore => gettext(locale, "Restore last session").into_owned(),
        Launch::Connections => gettext(locale, "Show Connections").into_owned(),
        Launch::Connection { id, .. } => match connections.iter().find(|saved| saved.id == *id) {
            Some(saved) => saved.name.clone(),
            None => gettext(locale, "Connection not found").into_owned(),
        },
    }
}

/// Where a saved connection points, as the menu writes it beside its
/// name: so that three of one name are told apart.
pub(super) fn place(saved: &SavedConnection) -> String {
    let (host, database) = crate::ui::picker::target(saved);
    format!("{host}{database}")
}

/// How long a connection may go unused, as the menu writes it.
pub(super) fn idle_text(locale: Locale, choice: Idle) -> String {
    let (one, many, count) = match choice {
        Idle::Never => return gettext(locale, "Never").into_owned(),
        Idle::Minutes5 => ("After {n} min", "After {n} min", 5),
        Idle::Minutes15 => ("After {n} min", "After {n} min", 15),
        Idle::Minutes30 => ("After {n} min", "After {n} min", 30),
        Idle::Hour1 => ("After {n} h", "After {n} h", 1),
        Idle::Hours4 => ("After {n} h", "After {n} h", 4),
    };
    ngettext(locale, one, many, count).replace("{n}", &count.to_string())
}

/// The days the log keeps a call, as the window writes them.
pub(super) fn days(locale: Locale, days: u32) -> String {
    ngettext(locale, "{n} day", "{n} days", days).replace("{n}", &days.to_string())
}

/// Whether an option of on and off is on.
pub(super) fn is_on(value: &OptionValue) -> bool {
    matches!(
        value,
        OptionValue::RestoreTabs(true)
            | OptionValue::ValueTags(true)
            | OptionValue::ProductionReadOnly(true)
            | OptionValue::ConfirmCloseEdits(true)
            | OptionValue::AgentsOn(true)
            | OptionValue::AgentsNotify(true)
            | OptionValue::Zebra(Some(true))
            | OptionValue::ColumnTypes(true)
            | OptionValue::ConfirmUnsafeWrites(true)
            | OptionValue::ClosePairs(true)
            | OptionValue::HighlightLine(true)
            | OptionValue::Wrap(true)
            | OptionValue::LineNumbers(Some(LineNumbers::Absolute | LineNumbers::Relative))
    )
}

/// The settings as the window shows them: the server's switch as it is in
/// force, off where a change made outside the app that turned it on is
/// held, the grouping of numbers as the look has it where it is not set,
/// the stripes and the font's size as the look draws them while they are
/// not set, and the theme as the look reads it: a value of the other
/// looks' is the desktop followed.
pub(super) fn shown(app: &App) -> Settings {
    let mut settings = app.settings.clone();
    settings.mcp_enabled = app.agents_on();
    settings.group_digits = Some(app.settings.grouped(app.layout()));
    settings.zebra = Some(settings.zebra_in(app.look.terminal));
    settings.font_size = Some(settings.font_size_in(app.look.terminal));
    settings.editor_keys = Some(settings.editor_keys_in(app.look.terminal));
    settings.line_numbers = Some(settings.line_numbers_in(app.look.terminal));
    settings.keywords = Some(settings.keywords_in(app.look.terminal));
    settings.theme = match (app.look.terminal, settings.theme.mode()) {
        (true, _) if settings.theme.pin().is_some() => settings.theme,
        (true, _) => Theme::Omarchy,
        (false, Mode::System) => Theme::System,
        (false, Mode::Light) => Theme::Light,
        (false, Mode::Dark) => Theme::Dark,
    };
    settings
}

/// Puts `value` in the window's own copy of the settings, as `App::apply`
/// will put it in the app's: a new pin is Omarchy's theme of the moment.
fn set_shown(app: &App, value: &OptionValue, settings: &mut Settings) {
    value.set(settings);
    if *value == OptionValue::Theme(ThemeChoice::Pinned)
        && settings.theme.pin().is_none()
        && let Some(name) = &app.desktop_theme.current
    {
        settings.theme = Theme::Pinned(name.clone());
    }
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
    /// A command's first key, as the keymap writes it for the look.
    Key(Command),
}

impl Said {
    /// What a screen reader and the tests call the choice.
    pub(super) fn name(
        self,
        ctx: &egui::Context,
        look: &crate::theme::Look,
        locale: Locale,
    ) -> Cow<'static, str> {
        match self {
            Self::Word(word) => gettext(locale, word),
            Self::Sample(sample) => Cow::Borrowed(sample),
            Self::Key(command) => Cow::Owned(
                crate::ui::keys::written(ctx, look, command)
                    .as_str()
                    .to_owned(),
            ),
        }
    }
}

/// The values a click can set on `option`'s row, each with what it is
/// called: what the row draws as segments or as a check, and what a screen
/// reader and the tests call them.
/// `named` is the last named zone the session had, which the zone's steps
/// reach, and `launches` what On launch steps through.
pub(super) fn choices(
    option: OptionId,
    settings: &Settings,
    named: Option<&str>,
    launches: &[Launch],
) -> Vec<(Said, OptionValue)> {
    use Said::{Key, Sample, Word};
    match option {
        OptionId::OnLaunch => vec![
            (Word("fewer"), settings.stepped_launch(launches, false)),
            (Word("more"), settings.stepped_launch(launches, true)),
        ],
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
        OptionId::RestoreTabs => vec![
            (Word("off"), OptionValue::RestoreTabs(false)),
            (Word("on"), OptionValue::RestoreTabs(true)),
        ],
        OptionId::AgentsOn => vec![
            (Word("off"), OptionValue::AgentsOn(false)),
            (Word("on"), OptionValue::AgentsOn(true)),
        ],
        OptionId::LogDays | OptionId::IdleDisconnect => vec![
            (Word("fewer"), settings.stepped(option, false)),
            (Word("more"), settings.stepped(option, true)),
        ],
        OptionId::AgentsNotify => vec![
            (Word("off"), OptionValue::AgentsNotify(false)),
            (Word("on"), OptionValue::AgentsNotify(true)),
        ],
        // The terminal look's: macOS and Windows draw their tiles.
        OptionId::Theme => vec![
            (
                Word("follow omarchy"),
                OptionValue::Theme(ThemeChoice::Omarchy),
            ),
            (Word("pin"), OptionValue::Theme(ThemeChoice::Pinned)),
        ],
        OptionId::InterfaceSize => vec![
            (
                Word("small"),
                OptionValue::InterfaceSize(InterfaceSize::Small),
            ),
            (
                Word("default"),
                OptionValue::InterfaceSize(InterfaceSize::Default),
            ),
            (
                Word("large"),
                OptionValue::InterfaceSize(InterfaceSize::Large),
            ),
        ],
        // A name is set in the file: the window offers none.
        OptionId::Font => Vec::new(),
        OptionId::FontSize => vec![
            (Word("smaller"), settings.stepped(option, false)),
            (Word("larger"), settings.stepped(option, true)),
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
        OptionId::NewTab => vec![
            (Word("read-only"), OptionValue::NewTab(NewTab::ReadOnly)),
            (
                Word("follow connection"),
                OptionValue::NewTab(NewTab::Connection),
            ),
        ],
        OptionId::RowLimit | OptionId::Timeout | OptionId::Indent | OptionId::HistoryKeep => {
            vec![
                (Word("fewer"), settings.stepped(option, false)),
                (Word("more"), settings.stepped(option, true)),
            ]
        }
        OptionId::RunScope => vec![
            (
                Word("statement"),
                OptionValue::RunScope(RunScope::Statement),
            ),
            (Word("whole tab"), OptionValue::RunScope(RunScope::All)),
        ],
        OptionId::ConfirmUnsafeWrites => vec![
            (Word("off"), OptionValue::ConfirmUnsafeWrites(false)),
            (Word("on"), OptionValue::ConfirmUnsafeWrites(true)),
        ],
        // As the terminal look's board draws them: vim first.
        OptionId::EditorKeys => vec![
            (Word("vim"), OptionValue::EditorKeys(Some(EditorKeys::Vim))),
            (
                Word("standard"),
                OptionValue::EditorKeys(Some(EditorKeys::Standard)),
            ),
        ],
        OptionId::Autocomplete => vec![
            (
                Word("as you type"),
                OptionValue::Autocomplete(Autocomplete::Typing),
            ),
            (
                Key(Command::Complete),
                OptionValue::Autocomplete(Autocomplete::Manual),
            ),
            (Word("off"), OptionValue::Autocomplete(Autocomplete::Off)),
        ],
        OptionId::ClosePairs => vec![
            (Word("off"), OptionValue::ClosePairs(false)),
            (Word("on"), OptionValue::ClosePairs(true)),
        ],
        OptionId::LineNumbers => LineNumbers::TERMINAL
            .into_iter()
            .map(|numbers| {
                let word = match numbers {
                    LineNumbers::Relative => "relative",
                    LineNumbers::Absolute => "absolute",
                    LineNumbers::Off => "off",
                };
                (Word(word), OptionValue::LineNumbers(Some(numbers)))
            })
            .collect(),
        OptionId::HighlightLine => vec![
            (Word("off"), OptionValue::HighlightLine(false)),
            (Word("on"), OptionValue::HighlightLine(true)),
        ],
        OptionId::Wrap => vec![
            (Word("off"), OptionValue::Wrap(false)),
            (Word("on"), OptionValue::Wrap(true)),
        ],
        // The case is the choice: the look does not lower-case it.
        OptionId::Keywords => vec![
            (
                Sample("UPPER"),
                OptionValue::Keywords(Some(KeywordCase::Upper)),
            ),
            (
                Sample("lower"),
                OptionValue::Keywords(Some(KeywordCase::Lower)),
            ),
            (Word("keep"), OptionValue::Keywords(Some(KeywordCase::Keep))),
        ],
        OptionId::Commas => vec![
            (Word("trailing"), OptionValue::Commas(Commas::Trailing)),
            (Word("leading"), OptionValue::Commas(Commas::Leading)),
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
        SettingsRow::Option(OptionId::HistoryKeep) => app
            .agents_window()
            .map(|_| vec![Action::ShowHistoryFromSettings]),
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
    /// Answer the history's question: delete what it kept, or keep it.
    History(bool),
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
        Command::SettingsDeleteHistory,
        Command::SettingsKeepHistory,
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
                Command::SettingsDeleteHistory => Asked::History(true),
                Command::SettingsKeepHistory => Asked::History(false),
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
                // Only while the question stands.
                Asked::History(delete) => {
                    if dialog.history_question.is_none() || repeat {
                        return true;
                    }
                    actions.push(Action::AnswerHistoryQuestion(delete));
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
                        Asked::Step { forward } if option == OptionId::OnLaunch => {
                            let launches = launch_choices(&app.connections.connections);
                            Some(settings.stepped_launch(&launches, forward))
                        }
                        Asked::Step { forward } => Some(settings.stepped(option, forward)),
                        _ => settings.flipped(option),
                    };
                    // A step to the theme the row shows already changes
                    // nothing: a value of the other looks', shown as the
                    // desktop followed, stays in the file.
                    let same = |value: &OptionValue| {
                        option == OptionId::Theme
                            && asked != Asked::Reset
                            && *value == settings.value(option)
                    };
                    if let Some(value) = value.filter(|value| !same(value)) {
                        set_shown(app, &value, &mut settings);
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
