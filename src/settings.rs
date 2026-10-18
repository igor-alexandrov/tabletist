//! User settings, stored as `settings.toml` in the config directory.

use std::path::Path;

use serde::Deserialize;

use crate::keymap::Layout;
use crate::paths::AppDirs;
use crate::util::toml_string;

/// How much of a timestamp the grid shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Timestamps {
    /// `2026-01-12 09:14:03`.
    #[default]
    Second,
    /// With the fraction the server sent.
    Full,
}

impl Timestamps {
    /// As the file says it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Second => "second",
            Self::Full => "full",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Second, Self::Full]
            .into_iter()
            .find(|choice| choice.name() == name)
    }
}

/// The zone a `timestamptz` value is shown in.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(from = "String")]
pub enum Zone {
    /// The computer's own, as it was when the app started.
    #[default]
    System,
    Utc,
    /// A zone of the IANA database, by its name: `America/New_York`.
    Named(String),
}

impl Zone {
    /// As the file says it: `system`, `utc` or the zone's name.
    pub fn name(&self) -> &str {
        match self {
            Self::System => "system",
            Self::Utc => "utc",
            Self::Named(name) => name,
        }
    }

    /// The zone the file names, where it is one: `system`, `utc` (in any
    /// case), or a name the zone database knows.
    pub fn from_name(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("system") {
            Some(Self::System)
        } else if name.eq_ignore_ascii_case("utc") {
            Some(Self::Utc)
        } else if !name.is_empty() && jiff::tz::db().get(name).is_ok() {
            Some(Self::Named(name.to_owned()))
        } else {
            None
        }
    }

    /// The zone itself. `system` is the zone the app found at its start.
    pub fn resolve(&self, system: &jiff::tz::TimeZone) -> jiff::tz::TimeZone {
        match self {
            Self::System => system.clone(),
            Self::Utc => jiff::tz::TimeZone::UTC,
            // Known to the database when it was read: a name that has
            // gone from it since shows as UTC.
            Self::Named(name) => jiff::tz::db().get(name).unwrap_or(jiff::tz::TimeZone::UTC),
        }
    }
}

/// The JSON of versions before the TOML file has no zone: anything else
/// it says is no zone either, and is the default.
impl From<String> for Zone {
    fn from(name: String) -> Self {
        Self::from_name(&name).unwrap_or_default()
    }
}

/// Everything the user can set. A key the file lacks takes its default and
/// one this version does not know is ignored, so older and newer files both
/// load. The serde derive reads the `settings.json` of versions before the
/// TOML file.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Rows fetched per page in the data grid.
    pub page_size: u32,
    /// Timestamps in grid cells: to the second, or as the server sent them.
    pub timestamps: Timestamps,
    /// The zone a value with an offset (a `timestamptz`) is shown in, and
    /// a value typed into such a column without one is sent from.
    pub tz: Zone,
    /// Numbers in grid cells with their integer digits in threes. Display
    /// only: a copy gives the value as it is. `None` is the look's default
    /// ([`Settings::grouped`]), which is not written: grouped on macOS and
    /// Windows, as their artboards draw it, and not on Omarchy.
    pub group_digits: Option<bool>,
    /// Enum, CHECK and boolean values drawn as coloured tags.
    pub value_tags: bool,
    /// Show `pg_catalog`, `information_schema`, `mysql`, `sys` and friends.
    pub show_system_schemas: bool,
    /// A palette file name in the themes directory. `None` follows the
    /// desktop: Omarchy when present, else the OS light/dark setting.
    pub custom_theme: Option<String>,
    /// Rows a SQL editor statement keeps (the Limit menu).
    pub sql_limit: u32,
    /// Seconds before a SQL editor run is cancelled; `None` waits forever.
    pub sql_timeout_secs: Option<u32>,
    /// The MCP server agents use the saved connections through. Off until
    /// the person turns it on: nothing listens before.
    pub mcp_enabled: bool,
    /// Days the log of agents' calls keeps a call.
    pub mcp_log_days: u32,
    /// Whether an agent that connects is told of: a notification, on
    /// macOS the Dock's bounce, on Windows the taskbar button's flash. The
    /// Agents entry says who is here either way.
    pub mcp_notify: bool,
}

/// A key of the settings file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    PageSize,
    Timestamps,
    Tz,
    GroupDigits,
    ValueTags,
    ShowSystemSchemas,
    SqlLimit,
    SqlTimeoutSecs,
    McpEnabled,
    McpLogDays,
    McpNotify,
    Theme,
}

impl Key {
    /// In the order the file has them: a table's keys together.
    pub const ALL: [Key; 12] = [
        Self::PageSize,
        Self::Timestamps,
        Self::Tz,
        Self::GroupDigits,
        Self::ValueTags,
        Self::ShowSystemSchemas,
        Self::SqlLimit,
        Self::SqlTimeoutSecs,
        Self::McpEnabled,
        Self::McpLogDays,
        Self::McpNotify,
        Self::Theme,
    ];

    /// The key's table and its name there.
    pub fn path(self) -> (&'static str, &'static str) {
        match self {
            Self::PageSize => ("data", "page_size"),
            Self::Timestamps => ("data", "timestamps"),
            Self::Tz => ("data", "tz"),
            Self::GroupDigits => ("data", "group_digits"),
            Self::ValueTags => ("data", "value_tags"),
            Self::ShowSystemSchemas => ("sidebar", "show_system_schemas"),
            Self::SqlLimit => ("editor", "sql_limit"),
            Self::SqlTimeoutSecs => ("editor", "sql_timeout_secs"),
            Self::McpEnabled => ("mcp", "enabled"),
            Self::McpLogDays => ("mcp", "log_days"),
            Self::McpNotify => ("mcp", "notify"),
            Self::Theme => ("appearance", "theme"),
        }
    }

    /// What the file says beside the key: the values of a closed set, or
    /// what a value that reads oddly means.
    fn note(self) -> Option<&'static str> {
        match self {
            Self::Timestamps => Some("second | full"),
            Self::Tz => Some("system | utc | \"Area/City\""),
            Self::SqlTimeoutSecs => Some("0 waits forever"),
            _ => None,
        }
    }
}

/// An option the Settings window shows: one row of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionId {
    PageSize,
    Timestamps,
    GroupDigits,
    ValueTags,
    /// Whether agents can reach the app: `mcp.enabled`.
    AgentsOn,
    /// How long the log of agents' calls keeps one: `mcp.log_days`.
    LogDays,
    /// Whether a new agent is told of: `mcp.notify`.
    AgentsNotify,
}

impl OptionId {
    /// Every option, the General pane's and then the Agents pane's.
    pub const ALL: [OptionId; 7] = [
        Self::PageSize,
        Self::Timestamps,
        Self::GroupDigits,
        Self::ValueTags,
        Self::AgentsOn,
        Self::LogDays,
        Self::AgentsNotify,
    ];
    /// The General pane's, in the order it has them.
    pub const GENERAL: [OptionId; 4] = [
        Self::PageSize,
        Self::Timestamps,
        Self::GroupDigits,
        Self::ValueTags,
    ];
    /// The Agents pane's, in the order it has them.
    pub const AGENTS: [OptionId; 3] = [Self::AgentsOn, Self::LogDays, Self::AgentsNotify];

    /// The key of the file the option is stored under.
    pub fn key(self) -> Key {
        match self {
            Self::PageSize => Key::PageSize,
            Self::Timestamps => Key::Timestamps,
            Self::GroupDigits => Key::GroupDigits,
            Self::ValueTags => Key::ValueTags,
            Self::AgentsOn => Key::McpEnabled,
            Self::LogDays => Key::McpLogDays,
            Self::AgentsNotify => Key::McpNotify,
        }
    }

    /// What the option is before anyone sets it.
    pub fn default_value(self) -> OptionValue {
        Settings::default().value(self)
    }
}

/// An option with a value: what a key, a click or a reset sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionValue {
    PageSize(u32),
    Timestamps(Timestamps),
    /// `None` is the look's default.
    GroupDigits(Option<bool>),
    ValueTags(bool),
    AgentsOn(bool),
    LogDays(u32),
    AgentsNotify(bool),
}

impl OptionValue {
    pub fn option(self) -> OptionId {
        match self {
            Self::PageSize(_) => OptionId::PageSize,
            Self::Timestamps(_) => OptionId::Timestamps,
            Self::GroupDigits(_) => OptionId::GroupDigits,
            Self::ValueTags(_) => OptionId::ValueTags,
            Self::AgentsOn(_) => OptionId::AgentsOn,
            Self::LogDays(_) => OptionId::LogDays,
            Self::AgentsNotify(_) => OptionId::AgentsNotify,
        }
    }

    /// Puts the value in `settings`.
    pub fn set(self, settings: &mut Settings) {
        match self {
            Self::PageSize(size) => settings.page_size = size,
            Self::Timestamps(choice) => settings.timestamps = choice,
            Self::GroupDigits(on) => settings.group_digits = on,
            Self::ValueTags(on) => settings.value_tags = on,
            Self::AgentsOn(on) => settings.mcp_enabled = on,
            Self::LogDays(days) => settings.mcp_log_days = days,
            Self::AgentsNotify(on) => settings.mcp_notify = on,
        }
    }
}

/// The file's first line.
const HEADER: &str = "# written by tabletist, safe to edit by hand\n";

/// The most a settings file is read to: about a hundred times what the app
/// writes. The reader takes a pass over the text for each bad line at worst
/// (the parser stops at a key with no `=`), which is nothing for a file a
/// person wrote and minutes, before the window opens, for a large file that
/// is something else.
const MAX_LINES: usize = 1_000;
const MAX_BYTES: usize = 64 * 1024;

/// Where a start's settings came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// `settings.toml`.
    Toml,
    /// An older version's `settings.json`: the TOML is still to be written.
    Json,
    /// No file, or one that could not be read and was kept aside.
    Defaults,
}

/// The settings as a file gave them, and what the file looked like.
#[derive(Clone, Debug, PartialEq)]
pub struct Loaded {
    pub settings: Settings,
    /// The file's text as read. With no TOML file, what would be written.
    pub text: String,
    /// The lines that were ignored, counted from 1, each once, in order.
    pub invalid: Vec<usize>,
    /// The line each key was read from, in the order of [`Key::ALL`].
    pub lines: Vec<(Key, usize)>,
    pub source: Source,
}

impl Loaded {
    /// Settings that were not read from a TOML text, brought into range
    /// first: the text is what a file would read back as, and the settings
    /// are the same.
    pub fn of(settings: Settings, source: Source) -> Self {
        let settings = settings.validated();
        let text = settings.to_toml();
        let lines = Settings::from_toml(&text).lines;
        Self {
            settings,
            text,
            invalid: Vec::new(),
            lines,
            source,
        }
    }
}

impl Loaded {
    /// Says in the log which lines of `path` were ignored. Until a window
    /// shows them, the log is where a typo is told. More of them than
    /// anyone would look up are told by their number alone.
    pub fn warn_invalid(&self, path: &Path) {
        const NAMED: usize = 20;
        if self.invalid.len() > NAMED {
            log::warn!(
                "{}: {} lines could not be read and are ignored",
                path.display(),
                self.invalid.len()
            );
            return;
        }
        for line in &self.invalid {
            log::warn!(
                "{}: line {line} could not be read and is ignored",
                path.display()
            );
        }
    }
}

impl From<Settings> for Loaded {
    fn from(settings: Settings) -> Self {
        Self::of(settings, Source::Defaults)
    }
}

/// What the app holds of the settings file while it runs: enough to know
/// its own writes when they come back from the disk, and to show the file
/// with the lines that were ignored.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SettingsFile {
    /// The text as last read or written.
    pub text: String,
    /// Where each line of the text starts, in bytes, found when the text
    /// is set ([`Loaded::into_parts`]): what shows the file knows how tall
    /// it is, and reaches the lines in view, without walking a long text on
    /// every frame.
    pub line_starts: Vec<usize>,
    /// The lines that were ignored, counted from 1, in order.
    pub invalid: Vec<usize>,
    /// The line each key is on.
    pub lines: Vec<(Key, usize)>,
    /// Whether the backend watches the file for changes made outside.
    pub live: bool,
    /// The text the app last asked to be written, if it asked: what tells
    /// its newest write, coming back from the disk, from an older one.
    pub saved: Option<String>,
    /// The texts the app handed over with the file to be opened: the
    /// backend writes one when no file is there, and that write is the
    /// app's own too. Each of them, not the last alone: two requests can
    /// wait at once, and the first may be the one that writes. None once a
    /// save was asked for since: the save is newer than all of them.
    pub offered: Vec<String>,
}

impl SettingsFile {
    /// How many lines the text has, as [`str::lines`] counts them.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// The line numbered `index` from 0, without its line break, as
    /// [`str::lines`] gives it. Nothing past the last.
    pub fn line(&self, index: usize) -> Option<&str> {
        let start = *self.line_starts.get(index)?;
        let end = self.line_starts.get(index + 1).copied();
        let line = &self.text[start..end.unwrap_or(self.text.len())];
        let line = line.strip_suffix('\n').unwrap_or(line);
        Some(line.strip_suffix('\r').unwrap_or(line))
    }
}

/// Where each line of `text` starts, in bytes: a start for each line
/// [`str::lines`] gives.
fn line_starts(text: &str) -> Vec<usize> {
    let mut start = 0;
    text.split_inclusive('\n')
        .map(|line| {
            let at = start;
            start += line.len();
            at
        })
        .collect()
}

impl Loaded {
    /// The settings, and what the app keeps of their file.
    pub fn into_parts(self) -> (Settings, SettingsFile) {
        let file = SettingsFile {
            line_starts: line_starts(&self.text),
            text: self.text,
            invalid: self.invalid,
            lines: self.lines,
            live: false,
            saved: None,
            offered: Vec::new(),
        };
        (self.settings, file)
    }
}

/// The line `offset` is on in `text`, counted from 1.
fn line_of(text: &str, offset: usize) -> usize {
    let before = &text.as_bytes()[..offset.min(text.len())];
    before.iter().filter(|byte| **byte == b'\n').count() + 1
}

/// The last line of `text` that is not blank, counted from 1. Blank as TOML
/// has it: nothing but spaces and tabs before the line's break. What only
/// Rust calls whitespace (a carriage return with no newline after it, a
/// no-break space) is what the parser rejected, so skipping it would blame
/// the good line above.
fn last_line(text: &str) -> Option<usize> {
    // `lines` takes a line's `\n` or `\r\n` with it: an empty line is blank
    // with either ending.
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_matches([' ', '\t']).is_empty())
        .map(|(index, _)| index + 1)
        .last()
}

/// `text` with the lines numbered in `lines` (from 1) blanked: each byte
/// but the line break becomes a space. Every other line keeps its number,
/// and every byte its offset, so what is found in the blanked text is at
/// the same place in `text`.
fn without_lines(text: &str, lines: &[usize]) -> String {
    text.split_inclusive('\n')
        .enumerate()
        .map(|(index, line)| {
            if !lines.contains(&(index + 1)) {
                return line.to_owned();
            }
            let body = line.strip_suffix('\n');
            let mut blank = " ".repeat(body.unwrap_or(line).len());
            if body.is_some() {
                blank.push('\n');
            }
            blank
        })
        .collect()
}

/// `text` as the reader reads it: every line the parser rejects blanked,
/// until it rejects nothing more, and those lines. The parser recovers
/// from a bad line in two ways that are not "ignored": it stops reading at
/// some (a key with no `=`), and reads past others with a guess at the
/// value (`full` unquoted). So a line it rejects is blanked and the text
/// read again.
fn readable(text: &str) -> (String, Vec<usize>) {
    let mut working = text.to_owned();
    let mut invalid: Vec<usize> = Vec::new();
    loop {
        let (_, errors) = toml::de::DeTable::parse_recoverable(&working);
        let mut rejected = Vec::new();
        for error in &errors {
            match error.span() {
                // The parser blames the end of the text for a value that
                // never closes (`"""` with no end). That is no line of the
                // file, so the last line with anything on it goes: the
                // lines are blanked from the bottom up until the opening
                // one is. With every line blank the error marks none.
                Some(span) if span.start >= working.len() => {
                    rejected.extend(last_line(&working));
                }
                Some(span) => rejected.push(line_of(&working, span.start)),
                None => log::warn!("settings: {}", error.message()),
            }
        }
        // A line already blanked has nothing left to reject: an error
        // that still names one ends the loop.
        rejected.retain(|line| !invalid.contains(line));
        rejected.sort_unstable();
        rejected.dedup();
        if rejected.is_empty() {
            return (working, invalid);
        }
        working = without_lines(&working, &rejected);
        invalid.extend(rejected);
    }
}

/// `text` without the table `name` where it has a header of its own and
/// nothing with it: no key, no table of its own, no comment beside its
/// header, and nothing of the person's (a comment, a line the reader
/// ignored) before the next header that was read. Its header goes, with
/// the blank lines under it.
fn without_empty_table(text: &str, name: &str) -> String {
    let (working, _) = readable(text);
    let (document, _) = toml::de::DeTable::parse_recoverable(&working);
    let document = document.into_inner();
    let Some(table) = document.get(name) else {
        return text.to_owned();
    };
    let header = table.span();
    let empty = table
        .get_ref()
        .as_table()
        .is_some_and(|items| items.is_empty());
    if !empty || !working[header.clone()].starts_with('[') {
        return text.to_owned();
    }
    let blank = |line: &str| line.trim_matches([' ', '\t', '\r', '\n']).is_empty();
    let start = line_start(&working, header.start);
    let mut end = line_end(&working, header.end);
    // A comment beside the header is the person's: the table stays with it.
    if !blank(&text[header.end..end]) {
        return text.to_owned();
    }
    while end < working.len() {
        let next = line_end(&working, end);
        // The next header that was read ends the table.
        if working[end..next]
            .trim_start_matches([' ', '\t'])
            .starts_with('[')
        {
            break;
        }
        // Anything else of the person's text keeps it: a comment, or a
        // line the reader ignored, which is blank only where it was read.
        if !blank(&text[end..next]) {
            return text.to_owned();
        }
        end = next;
    }
    let mut out = text.to_owned();
    out.replace_range(start..end, "");
    out
}

/// The key a line the parser rejected would have set: its name before
/// the `=` (bare, in quotes, or dotted), under the last table header above
/// it that was read (`working`, where rejected lines are blank). A line
/// with no `=`, or a name of another form, says none.
fn key_on_line(text: &str, working: &str, line: usize) -> Option<Key> {
    // The parts of a dotted name, each bare or in quotes with nothing to
    // escape.
    let parts = |name: &str| -> Option<Vec<String>> {
        name.split('.')
            .map(|part| {
                let part = part.trim_matches([' ', '\t']);
                let part = ['"', '\'']
                    .into_iter()
                    .find_map(|quote| part.strip_prefix(quote)?.strip_suffix(quote))
                    .unwrap_or(part);
                let bare = !part.is_empty()
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
                bare.then(|| part.to_owned())
            })
            .collect()
    };
    let unmarked = |line: &str| line.trim_start_matches('\u{feff}').to_owned();
    let written = unmarked(text.lines().nth(line.checked_sub(1)?)?);
    let name = parts(written.split_once('=')?.0)?;
    let table = working
        .lines()
        .take(line - 1)
        .filter_map(|above| {
            let above = unmarked(above);
            let above = above.split('#').next()?.trim_matches([' ', '\t']);
            let inner = above.strip_prefix('[')?.strip_suffix(']')?;
            (!inner.starts_with('[')).then(|| parts(inner)).flatten()
        })
        .last()
        .unwrap_or_default();
    let path = [table, name].concat();
    let [table, name] = path.as_slice() else {
        return None;
    };
    Key::ALL
        .into_iter()
        .find(|key| key.path() == (table.as_str(), name.as_str()))
}

/// Whether an item of a table is a table with a header of its own under
/// it (`[data.more]`, `[[data.list]]`), and not a value of it: an array
/// is written with a bracket too.
fn is_subtable(working: &str, item: &toml::de::DeValue<'_>, span: std::ops::Range<usize>) -> bool {
    let written = &working[span];
    match item {
        toml::de::DeValue::Table(_) => written.starts_with('['),
        toml::de::DeValue::Array(items) => {
            written.starts_with("[[")
                && items
                    .iter()
                    .all(|item| matches!(item.get_ref(), toml::de::DeValue::Table(_)))
        }
        _ => false,
    }
}

/// Where the line that `offset` is on begins.
fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map_or(0, |at| at + 1)
}

/// Where the line after the one `offset` is on begins: past its break, or
/// the end of the text.
fn line_end(text: &str, offset: usize) -> usize {
    text[offset..]
        .find('\n')
        .map_or(text.len(), |at| offset + at + 1)
}

/// Whether `range` of `text` is a line of its own (or lines): nothing but
/// spaces and tabs before it on its first line, and nothing but those and
/// a comment after it on its last.
fn own_lines(text: &str, range: std::ops::Range<usize>) -> bool {
    let before = &text[line_start(text, range.start)..range.start];
    let after = &text[range.end..line_end(text, range.end)];
    let after = after.trim_start_matches([' ', '\t']);
    before.trim_matches([' ', '\t']).is_empty()
        && (after.starts_with('#') || after.trim_matches(['\r', '\n']).is_empty())
}

impl Settings {
    pub const DEFAULT_PAGE_SIZE: u32 = 1_000;
    pub const MIN_PAGE_SIZE: u32 = 10;
    pub const MAX_PAGE_SIZE: u32 = 10_000;
    /// The page sizes the Settings window offers.
    pub const PAGE_SIZES: [u32; 5] = [100, 500, 1_000, 5_000, 10_000];
    /// The most rows a SQL editor statement keeps.
    pub const MAX_SQL_LIMIT: u32 = 10_000;
    /// The row limits the SQL editor's Limit menu offers.
    pub const SQL_LIMITS: [u32; 3] = [100, 1_000, 10_000];
    /// The timeouts (seconds) the SQL editor's Timeout menu offers; `None`
    /// is no timeout.
    pub const SQL_TIMEOUTS: [Option<u32>; 5] = [Some(10), Some(30), Some(60), Some(300), None];
    /// Days the log of agents' calls keeps a call, at least and at most: a
    /// day, and ten years.
    pub const MIN_MCP_LOG_DAYS: u32 = 1;
    pub const MAX_MCP_LOG_DAYS: u32 = 3_650;
    /// The days the Settings window offers for the log.
    pub const LOG_DAYS: [u32; 4] = [7, 30, 90, 365];

    /// Whether numbers are grouped in `layout`: as set, or the look's
    /// default.
    pub fn grouped(&self, layout: Layout) -> bool {
        self.group_digits
            .unwrap_or_else(|| Self::grouped_by_default(layout))
    }

    /// Whether numbers are grouped in `layout` before anyone sets it: as
    /// each look's artboard draws them.
    pub fn grouped_by_default(layout: Layout) -> bool {
        match layout {
            Layout::Mac | Layout::Windows => true,
            Layout::Omarchy => false,
        }
    }

    /// The settings with what is set to the look's default taken for not
    /// set, so that it is not written and follows the look.
    pub fn for_layout(mut self, layout: Layout) -> Self {
        if self.group_digits == Some(Self::grouped_by_default(layout)) {
            self.group_digits = None;
        }
        self
    }

    /// The SQL editor's timeout as a duration; `None` waits forever.
    pub fn sql_timeout(&self) -> Option<std::time::Duration> {
        Self::timeout_of(self.sql_timeout_secs)
    }

    /// A timeout in seconds as a duration.
    pub fn timeout_of(secs: Option<u32>) -> Option<std::time::Duration> {
        secs.map(|secs| std::time::Duration::from_secs(u64::from(secs)))
    }

    /// A row limit the SQL editor can run with.
    pub fn valid_sql_limit(limit: u32) -> u32 {
        limit.clamp(1, Self::MAX_SQL_LIMIT)
    }

    /// A timeout the SQL editor can run with. One of no seconds would
    /// cancel every run at once, so it counts as no timeout.
    pub fn valid_sql_timeout(secs: Option<u32>) -> Option<u32> {
        secs.filter(|secs| *secs > 0)
    }

    /// What `option` is set to.
    pub fn value(&self, option: OptionId) -> OptionValue {
        match option {
            OptionId::PageSize => OptionValue::PageSize(self.page_size),
            OptionId::Timestamps => OptionValue::Timestamps(self.timestamps),
            OptionId::GroupDigits => OptionValue::GroupDigits(self.group_digits),
            OptionId::ValueTags => OptionValue::ValueTags(self.value_tags),
            OptionId::AgentsOn => OptionValue::AgentsOn(self.mcp_enabled),
            OptionId::LogDays => OptionValue::LogDays(self.mcp_log_days),
            OptionId::AgentsNotify => OptionValue::AgentsNotify(self.mcp_notify),
        }
    }

    /// The value one step from `option`'s, towards the right (`forward`)
    /// or the left, as the window draws its choices: the next page size
    /// (none past the ends), the segment on that side, a check on to the
    /// right and off to the left.
    pub fn stepped(&self, option: OptionId, forward: bool) -> OptionValue {
        match option {
            OptionId::PageSize => {
                let size = self.page_size;
                let next = if forward {
                    Self::PAGE_SIZES.into_iter().find(|choice| *choice > size)
                } else {
                    Self::PAGE_SIZES
                        .into_iter()
                        .rev()
                        .find(|choice| *choice < size)
                };
                OptionValue::PageSize(next.unwrap_or(size))
            }
            OptionId::Timestamps => OptionValue::Timestamps(if forward {
                Timestamps::Full
            } else {
                Timestamps::Second
            }),
            // Grouped is the left of the two.
            OptionId::GroupDigits => OptionValue::GroupDigits(Some(!forward)),
            OptionId::ValueTags => OptionValue::ValueTags(forward),
            OptionId::AgentsOn => OptionValue::AgentsOn(forward),
            OptionId::LogDays => {
                let days = self.mcp_log_days;
                let next = if forward {
                    Self::LOG_DAYS.into_iter().find(|choice| *choice > days)
                } else {
                    Self::LOG_DAYS
                        .into_iter()
                        .rev()
                        .find(|choice| *choice < days)
                };
                OptionValue::LogDays(next.unwrap_or(days))
            }
            OptionId::AgentsNotify => OptionValue::AgentsNotify(forward),
        }
    }

    /// The other value of an option that has two; `None` for one with more.
    pub fn flipped(&self, option: OptionId) -> Option<OptionValue> {
        match option {
            OptionId::PageSize => None,
            OptionId::Timestamps => Some(OptionValue::Timestamps(match self.timestamps {
                Timestamps::Second => Timestamps::Full,
                Timestamps::Full => Timestamps::Second,
            })),
            OptionId::GroupDigits => Some(OptionValue::GroupDigits(Some(
                !self.group_digits.unwrap_or_default(),
            ))),
            OptionId::ValueTags => Some(OptionValue::ValueTags(!self.value_tags)),
            OptionId::AgentsOn => Some(OptionValue::AgentsOn(!self.mcp_enabled)),
            OptionId::LogDays => None,
            OptionId::AgentsNotify => Some(OptionValue::AgentsNotify(!self.mcp_notify)),
        }
    }

    /// `key`'s value as TOML writes it. `None` for a key the file leaves
    /// out: a theme that is not set.
    fn written(&self, key: Key) -> Option<String> {
        Some(match key {
            Key::PageSize => self.page_size.to_string(),
            Key::Timestamps => toml_string(self.timestamps.name()),
            Key::Tz => toml_string(self.tz.name()),
            Key::GroupDigits => self.group_digits?.to_string(),
            Key::ValueTags => self.value_tags.to_string(),
            Key::ShowSystemSchemas => self.show_system_schemas.to_string(),
            Key::SqlLimit => self.sql_limit.to_string(),
            // TOML has no null: no timeout is no seconds.
            Key::SqlTimeoutSecs => self.sql_timeout_secs.unwrap_or(0).to_string(),
            Key::McpEnabled => self.mcp_enabled.to_string(),
            Key::McpLogDays => self.mcp_log_days.to_string(),
            Key::McpNotify => self.mcp_notify.to_string(),
            Key::Theme => toml_string(self.custom_theme.as_deref()?),
        })
    }

    /// `key`'s value as the file has it: `None` where it is the default,
    /// which the file leaves out.
    fn literal(&self, key: Key) -> Option<String> {
        self.written(key)
            .filter(|written| Some(written) != Settings::default().written(key).as_ref())
    }

    /// The keys whose value differs between `self` and `other`, in the
    /// order of [`Key::ALL`]: what a write of `other` over `self` touches.
    pub fn changed_keys(&self, other: &Settings) -> Vec<Key> {
        Key::ALL
            .into_iter()
            .filter(|key| self.literal(*key) != other.literal(*key))
            .collect()
    }

    /// The text of a file that holds nothing but these settings: the
    /// header line, and a table for the keys that are not at their
    /// default, in the order of [`Key::ALL`], each with its note.
    pub fn to_toml(&self) -> String {
        self.write_into("", &Key::ALL)
    }

    /// `text` with `keys` set to their values here, and nothing else of it
    /// changed: the person's comments, the order of their keys, keys and
    /// tables this version does not know, and the lines the reader ignored
    /// all stay. A key at its default is taken out, with its line, and a
    /// table left with nothing in it goes too. A key the file has gets its
    /// new value in place, the comment beside it kept; one it lacks goes at
    /// the end of its table, with its note, and a table it lacks at the end
    /// of the file. An empty text starts as the header line.
    ///
    /// A table the file writes inline (`data = { page_size = 500 }`) or
    /// with dotted keys can have a value replaced in place and nothing
    /// more: a key to add or take out there gets the file written as
    /// [`Settings::to_toml`] has it. So does a text too long to be read.
    pub fn write_into(&self, text: &str, keys: &[Key]) -> String {
        if text.len() > MAX_BYTES || text.lines().count() > MAX_LINES {
            return self.to_toml();
        }
        let text = if text.trim_matches([' ', '\t', '\r', '\n']).is_empty() {
            HEADER
        } else {
            text
        };
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let (working, rejected) = readable(text);
        // The keys the rejected lines would set: one of them taken out of
        // the file would leave the rejected line to be read in its place.
        let shadowed: Vec<Key> = rejected
            .iter()
            .filter_map(|line| key_on_line(text, &working, *line))
            .collect();
        let (document, _) = toml::de::DeTable::parse_recoverable(&working);
        let document = document.into_inner();
        // Replacements of ranges of the text, each with its place in the
        // order they were found: inserts at one place keep that order.
        let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
        let mut new_tables: Vec<(&str, String)> = Vec::new();
        let mut emptied: Vec<&str> = Vec::new();
        let line = |key: Key, literal: &str| {
            let name = key.path().1;
            match key.note() {
                Some(note) => format!("{name} = {literal}  # {note}{newline}"),
                None => format!("{name} = {literal}{newline}"),
            }
        };
        for &key in keys {
            let (table_name, name) = key.path();
            let literal = self.literal(key);
            let Some(table) = document.get(table_name) else {
                if let Some(literal) = literal {
                    new_tables.push((table_name, line(key, &literal)));
                }
                continue;
            };
            let header = table.span();
            // A table with a header of its own, which lines can be added
            // to and taken from.
            let headed = working[header.clone()].starts_with('[');
            let Some(items) = table.get_ref().as_table() else {
                // The name is a value (`data = 5`) or a list of tables:
                // nothing of ours can be written there, and nothing of the
                // person's is taken to make room.
                log::warn!("settings: {table_name} is not a table, {name} is not written");
                continue;
            };
            // A table no header names, there because one under it is
            // (`[data.more]`): a header of its own can follow.
            let implicit = !headed
                && !items.is_empty()
                && items
                    .iter()
                    .all(|(_, item)| is_subtable(&working, item.get_ref(), item.span()));
            match (items.get_key_value(name), literal) {
                (Some((_, value)), Some(literal)) => edits.push((value.span(), literal)),
                // The default in place: taken out, the rejected line of
                // the same key would be the one read.
                (Some((_, value)), None) if shadowed.contains(&key) => {
                    if let Some(default) = Settings::default().written(key) {
                        edits.push((value.span(), default));
                    }
                }
                (Some((found, value)), None) => {
                    let whole = found.span().start..value.span().end;
                    if !headed || !own_lines(&working, whole.clone()) {
                        return self.to_toml();
                    }
                    let lines = line_start(&working, whole.start)..line_end(&working, whole.end);
                    edits.push((lines, String::new()));
                    emptied.push(table_name);
                }
                (None, None) => {}
                (None, Some(literal)) if implicit => {
                    new_tables.push((table_name, line(key, &literal)));
                }
                (None, Some(literal)) => {
                    if !headed {
                        return self.to_toml();
                    }
                    // After the table's last key, or under its header. A
                    // table under it (`[data.more]`) is no key of it.
                    let last = items
                        .iter()
                        .filter(|(_, item)| !is_subtable(&working, item.get_ref(), item.span()))
                        .map(|(_, item)| item.span().end)
                        .max()
                        .unwrap_or(header.end);
                    let at = line_end(&working, last);
                    let mut added = line(key, &literal);
                    if !working[..at].ends_with('\n') {
                        added.insert_str(0, newline);
                    }
                    edits.push((at..at, added));
                }
            }
        }
        let mut out = text.to_owned();
        let mut order: Vec<usize> = (0..edits.len()).collect();
        // From the end, so a range found in the text is where it was; at
        // one place the last found goes in first, and the ones before it
        // go in front of it.
        order.sort_by(|a, b| (edits[*b].0.start, *b).cmp(&(edits[*a].0.start, *a)));
        for index in order {
            let (range, with) = &edits[index];
            out.replace_range(range.clone(), with);
        }
        let emptied_any = !emptied.is_empty();
        for table in emptied {
            out = without_empty_table(&out, table);
        }
        if emptied_any {
            let kept = out.trim_end_matches(['\r', '\n']).len();
            out.truncate(kept);
            // A file left with nothing in it stays empty.
            if !out.is_empty() {
                out.push_str(newline);
            }
        }
        let mut open: Option<&str> = None;
        for (table, added) in new_tables {
            if open != Some(table) {
                if !out.ends_with('\n') {
                    out.push_str(newline);
                }
                if out != HEADER {
                    out.push_str(newline);
                }
                out.push_str(&format!("[{table}]{newline}"));
                open = Some(table);
            }
            out.push_str(&added);
        }
        out
    }

    /// The settings with every number in its range.
    pub fn validated(mut self) -> Self {
        self.page_size = self
            .page_size
            .clamp(Self::MIN_PAGE_SIZE, Self::MAX_PAGE_SIZE);
        self.sql_limit = Self::valid_sql_limit(self.sql_limit);
        self.sql_timeout_secs = Self::valid_sql_timeout(self.sql_timeout_secs);
        self.mcp_log_days = self
            .mcp_log_days
            .clamp(Self::MIN_MCP_LOG_DAYS, Self::MAX_MCP_LOG_DAYS);
        self
    }

    /// Takes `key`'s value from the file. False when it is no value the key
    /// can have: the setting stays as it was.
    fn take(&mut self, key: Key, value: &toml::de::DeValue<'_>) -> bool {
        // An integer of any size an `i64` holds is a number here: it is cut
        // to a `u32` and `validated` brings it into range. One an `i64`
        // cannot hold is no value for the key.
        let number = value
            .as_integer()
            .and_then(|integer| i64::from_str_radix(integer.as_str(), integer.radix()).ok())
            .and_then(|number| u32::try_from(number.clamp(0, i64::from(u32::MAX))).ok());
        let flag = value.as_bool();
        let text = value.as_str();
        match key {
            Key::PageSize => number.map(|number| self.page_size = number),
            Key::Timestamps => text
                .and_then(Timestamps::from_name)
                .map(|choice| self.timestamps = choice),
            Key::Tz => text.and_then(Zone::from_name).map(|zone| self.tz = zone),
            Key::GroupDigits => flag.map(|flag| self.group_digits = Some(flag)),
            Key::ValueTags => flag.map(|flag| self.value_tags = flag),
            Key::ShowSystemSchemas => flag.map(|flag| self.show_system_schemas = flag),
            Key::SqlLimit => number.map(|number| self.sql_limit = number),
            Key::SqlTimeoutSecs => number.map(|number| self.sql_timeout_secs = Some(number)),
            Key::McpEnabled => flag.map(|flag| self.mcp_enabled = flag),
            Key::McpLogDays => number.map(|number| self.mcp_log_days = number),
            Key::McpNotify => flag.map(|flag| self.mcp_notify = flag),
            Key::Theme => text.map(|name| self.custom_theme = Some(name.to_owned())),
        }
        .is_some()
    }

    /// Reads the file's text, keeping everything it can. A line that is not
    /// TOML, and a value its key cannot have, are ignored and remembered by
    /// line; the key keeps its default. A key or a table this version does
    /// not know is ignored without a word: a newer one may have written it.
    /// A table header that is rejected can cost the keys under it too, and
    /// those without a mark: they are read into the table above (or into
    /// none), and count only if that table knows them. A text longer than a
    /// settings file can be (`MAX_LINES`, `MAX_BYTES`) is not read at all:
    /// every line of it that says something is one that was ignored.
    pub fn from_toml(text: &str) -> Loaded {
        Self::read(text, &Settings::default())
    }

    /// Reads a new text of the file over the settings it last gave: as
    /// [`Settings::from_toml`], but a key whose line is ignored keeps its
    /// value in `last`, its last good one, and not its default. A key the
    /// text leaves out takes its default: it was taken out.
    pub fn from_toml_over(text: &str, last: &Settings) -> Loaded {
        Self::read(text, last)
    }

    fn read(text: &str, last: &Settings) -> Loaded {
        if text.len() > MAX_BYTES || text.lines().count() > MAX_LINES {
            let invalid = text
                .lines()
                .enumerate()
                .filter(|(_, line)| !line.trim_matches([' ', '\t']).is_empty())
                .map(|(index, _)| index + 1)
                .collect();
            return Loaded {
                settings: last.clone(),
                text: text.to_owned(),
                invalid,
                lines: Vec::new(),
                source: Source::Toml,
            };
        }
        let (working, mut invalid) = readable(text);
        let (document, _) = toml::de::DeTable::parse_recoverable(&working);
        let document = document.into_inner();
        let mut settings = Settings::default();
        let mut lines = Vec::new();
        // The keys the rejected lines would have set, as far as a line
        // says it: those keep their last good value, unless a line that
        // was read sets them.
        let mut kept: Vec<Key> = invalid
            .iter()
            .filter_map(|line| key_on_line(text, &working, *line))
            .collect();
        for key in Key::ALL {
            let (table, name) = key.path();
            let place = document.get(table);
            // A table's name the file gives a value (`data = 5`): its keys
            // cannot be read or written there, and keep what they had.
            if place.is_some_and(|place| place.get_ref().as_table().is_none()) {
                kept.push(key);
                continue;
            }
            let Some((found, value)) = place
                .and_then(|table| table.get_ref().as_table())
                .and_then(|table| table.get_key_value(name))
            else {
                continue;
            };
            // Blanked lines kept their breaks: a line has the same number
            // in `working` as in `text`.
            let line = line_of(&working, found.span().start);
            if settings.take(key, value.get_ref()) {
                lines.push((key, line));
                kept.retain(|other| *other != key);
            } else {
                invalid.push(line);
                kept.push(key);
            }
        }
        for key in kept {
            settings.keep(key, last);
        }
        invalid.sort_unstable();
        invalid.dedup();
        Loaded {
            settings: settings.validated(),
            text: text.to_owned(),
            invalid,
            lines,
            source: Source::Toml,
        }
    }

    /// Takes `key`'s value from `other`.
    fn keep(&mut self, key: Key, other: &Settings) {
        match key {
            Key::PageSize => self.page_size = other.page_size,
            Key::Timestamps => self.timestamps = other.timestamps,
            Key::Tz => self.tz = other.tz.clone(),
            Key::GroupDigits => self.group_digits = other.group_digits,
            Key::ValueTags => self.value_tags = other.value_tags,
            Key::ShowSystemSchemas => self.show_system_schemas = other.show_system_schemas,
            Key::SqlLimit => self.sql_limit = other.sql_limit,
            Key::SqlTimeoutSecs => self.sql_timeout_secs = other.sql_timeout_secs,
            Key::McpEnabled => self.mcp_enabled = other.mcp_enabled,
            Key::McpLogDays => self.mcp_log_days = other.mcp_log_days,
            Key::McpNotify => self.mcp_notify = other.mcp_notify,
            Key::Theme => self.custom_theme = other.custom_theme.clone(),
        }
    }

    /// The settings a start has: from `settings.toml`, else from an older
    /// version's `settings.json`, else the defaults. It writes no
    /// settings (a file it cannot use is moved aside); what came from the
    /// JSON is written as TOML by the app (see [`Source`]).
    pub fn load(dirs: &AppDirs) -> Loaded {
        let path = dirs.settings_file();
        match std::fs::read(&path) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => {
                    let loaded = Self::from_toml(&text);
                    loaded.warn_invalid(&path);
                    loaded
                }
                Err(error) => {
                    // Not text at all: kept aside like a damaged JSON file,
                    // so the next save cannot replace it.
                    crate::util::keep_aside(&path, &error.to_string());
                    Self::default().into()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let legacy = dirs.legacy_settings_file();
                if legacy.exists() {
                    Loaded::of(crate::util::load_json::<Settings>(&legacy), Source::Json)
                } else {
                    Self::default().into()
                }
            }
            Err(error) => {
                // There and not readable (permissions, an I/O error): kept
                // aside too, or the next save would rename a new file over
                // one nobody has read.
                crate::util::keep_aside(&path, &error.to_string());
                Self::default().into()
            }
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            page_size: Self::DEFAULT_PAGE_SIZE,
            timestamps: Timestamps::Second,
            tz: Zone::System,
            group_digits: None,
            value_tags: true,
            show_system_schemas: false,
            custom_theme: None,
            sql_limit: 1_000,
            sql_timeout_secs: Some(30),
            mcp_enabled: false,
            mcp_log_days: 30,
            mcp_notify: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let settings = Settings::default();
        assert_eq!(settings.page_size, 1_000);
        assert_eq!(settings.timestamps, Timestamps::Second);
        assert_eq!(settings.group_digits, None);
        assert!(settings.value_tags);
        assert!(!settings.show_system_schemas);
        assert_eq!(settings.custom_theme, None);
        assert_eq!(settings.sql_limit, 1_000);
        assert_eq!(settings.sql_timeout_secs, Some(30));
        assert!(!settings.mcp_enabled);
        assert_eq!(settings.mcp_log_days, 30);
        assert_eq!(
            settings.sql_timeout(),
            Some(std::time::Duration::from_secs(30))
        );
    }

    #[test]
    fn a_timestamps_choice_has_a_name() {
        for choice in [Timestamps::Second, Timestamps::Full] {
            assert_eq!(Timestamps::from_name(choice.name()), Some(choice));
        }
        assert_eq!(Timestamps::from_name("minute"), None);
    }

    #[test]
    fn the_defaults_are_written_as_the_header_alone() {
        assert_eq!(Settings::default().to_toml(), HEADER);
    }

    /// Every key away from its default.
    fn all_set() -> Settings {
        Settings {
            page_size: 500,
            timestamps: Timestamps::Full,
            tz: Zone::Named("America/New_York".into()),
            group_digits: Some(true),
            value_tags: false,
            show_system_schemas: true,
            custom_theme: Some("My \"Nord\".json".into()),
            sql_limit: 100,
            sql_timeout_secs: None,
            mcp_enabled: true,
            mcp_log_days: 7,
            mcp_notify: false,
        }
    }

    #[test]
    fn only_what_is_not_the_default_is_written_each_table_in_its_place() {
        assert_eq!(
            all_set().to_toml(),
            "\
# written by tabletist, safe to edit by hand
[data]
page_size = 500
timestamps = \"full\"  # second | full
tz = \"America/New_York\"  # system | utc | \"Area/City\"
group_digits = true
value_tags = false

[sidebar]
show_system_schemas = true

[editor]
sql_limit = 100
sql_timeout_secs = 0  # 0 waits forever

[mcp]
enabled = true
log_days = 7
notify = false

[appearance]
theme = \"My \\\"Nord\\\".json\"
"
        );
    }

    #[test]
    fn a_theme_gets_its_table_and_no_timeout_is_zero() {
        let settings = Settings {
            custom_theme: Some("My \"Nord\".json".into()),
            sql_timeout_secs: None,
            timestamps: Timestamps::Full,
            ..Settings::default()
        };
        let text = settings.to_toml();
        assert!(
            text.contains("timestamps = \"full\"  # second | full\n"),
            "{text}"
        );
        assert!(
            text.contains("sql_timeout_secs = 0  # 0 waits forever\n"),
            "{text}"
        );
        assert!(
            text.ends_with("\n\n[appearance]\ntheme = \"My \\\"Nord\\\".json\"\n"),
            "{text}"
        );
    }

    #[test]
    fn every_key_has_its_own_place_in_the_file() {
        let mut paths: Vec<_> = Key::ALL.iter().map(|key| key.path()).collect();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), Key::ALL.len());
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let settings = all_set();
        let text = settings.to_toml();
        let loaded = Settings::from_toml(&text);
        assert_eq!(loaded.settings, settings);
        assert_eq!(loaded.text, text);
        assert_eq!(loaded.invalid, Vec::<usize>::new());
        assert_eq!(loaded.source, Source::Toml);
        // Writing it again changes nothing.
        assert_eq!(loaded.settings.to_toml(), text);
    }

    #[test]
    fn each_key_is_found_on_its_line() {
        let loaded = Settings::from_toml(&all_set().to_toml());
        assert_eq!(
            loaded.lines,
            vec![
                (Key::PageSize, 3),
                (Key::Timestamps, 4),
                (Key::Tz, 5),
                (Key::GroupDigits, 6),
                (Key::ValueTags, 7),
                (Key::ShowSystemSchemas, 10),
                (Key::SqlLimit, 13),
                (Key::SqlTimeoutSecs, 14),
                (Key::McpEnabled, 17),
                (Key::McpLogDays, 18),
                (Key::McpNotify, 19),
                (Key::Theme, 22),
            ]
        );
    }

    /// `text` with `change` made in the app: what the file says after.
    fn written(text: &str, change: impl FnOnce(&mut Settings)) -> String {
        let before = Settings::from_toml(text).settings;
        let mut after = before.clone();
        change(&mut after);
        after.write_into(text, &before.changed_keys(&after))
    }

    #[test]
    fn a_value_is_set_in_place_and_the_comment_beside_it_stays() {
        let text = "# mine\n[data]\n# rows\npage_size = 500 # as I like it\nvalue_tags = false\n";
        assert_eq!(
            written(text, |settings| settings.page_size = 100),
            "# mine\n[data]\n# rows\npage_size = 100 # as I like it\nvalue_tags = false\n"
        );
    }

    #[test]
    fn a_key_set_to_its_default_goes_with_its_line_and_its_comments_stay() {
        let text = "[data]\n# rows\npage_size = 500 # as I like it\nvalue_tags = false\n";
        assert_eq!(
            written(text, |settings| settings.page_size =
                Settings::DEFAULT_PAGE_SIZE),
            "[data]\n# rows\nvalue_tags = false\n"
        );
    }

    #[test]
    fn a_table_left_with_nothing_in_it_goes() {
        let text = "# mine\n\n[data]\npage_size = 500\n\n[editor]\nsql_limit = 100\n";
        assert_eq!(
            written(text, |settings| settings.page_size =
                Settings::DEFAULT_PAGE_SIZE),
            "# mine\n\n[editor]\nsql_limit = 100\n"
        );
        // The last table too, and the blank lines it leaves at the end.
        assert_eq!(
            written(text, |settings| settings.sql_limit = 1_000),
            "# mine\n\n[data]\npage_size = 500\n"
        );
    }

    #[test]
    fn a_table_with_a_comment_left_in_it_stays() {
        let text = "[data]\npage_size = 500\n# keep me\n\n[editor]\nsql_limit = 100\n";
        assert_eq!(
            written(text, |settings| settings.page_size =
                Settings::DEFAULT_PAGE_SIZE),
            "[data]\n# keep me\n\n[editor]\nsql_limit = 100\n"
        );
    }

    #[test]
    fn a_new_key_goes_at_the_end_of_its_table_with_its_note() {
        let text = "[data]\npage_size = 500\n\n[data.more]\nx = 1\n\n[editor]\nsql_limit = 100\n";
        assert_eq!(
            written(text, |settings| settings.timestamps = Timestamps::Full),
            "[data]\npage_size = 500\ntimestamps = \"full\"  # second | full\n\n\
             [data.more]\nx = 1\n\n[editor]\nsql_limit = 100\n"
        );
        // Under the header of a table that has no key yet.
        assert_eq!(
            written("[data]\n", |settings| settings.page_size = 500),
            "[data]\npage_size = 500\n"
        );
    }

    #[test]
    fn a_new_table_goes_at_the_end_of_the_file() {
        let text = "[data]\npage_size = 500\n# the end";
        assert_eq!(
            written(text, |settings| settings.sql_limit = 100),
            "[data]\npage_size = 500\n# the end\n\n[editor]\nsql_limit = 100\n"
        );
    }

    #[test]
    fn unknown_keys_and_tables_and_their_comments_survive_a_write() {
        let text = "[data]\nfrom_the_future = 1 # later\npage_size = 500\n\n[tomorrow]\nkey = 1\n";
        assert_eq!(
            written(text, |settings| settings.page_size = 100),
            "[data]\nfrom_the_future = 1 # later\npage_size = 100\n\n[tomorrow]\nkey = 1\n"
        );
        assert_eq!(
            written(text, |settings| settings.page_size =
                Settings::DEFAULT_PAGE_SIZE),
            "[data]\nfrom_the_future = 1 # later\n\n[tomorrow]\nkey = 1\n"
        );
    }

    #[test]
    fn a_key_at_its_default_by_hand_stays_until_it_is_changed() {
        let text = format!("[data]\npage_size = {}\n", Settings::DEFAULT_PAGE_SIZE);
        assert_eq!(
            written(&text, |settings| settings.value_tags = false),
            format!("{text}value_tags = false\n")
        );
    }

    #[test]
    fn a_line_the_reader_ignored_stays_through_a_write() {
        let text = "[data]\ntimestamps = full\ngroup_digits = \"yes\"\npage_size = 500\n";
        let loaded = Settings::from_toml(text);
        assert_eq!(loaded.invalid, [2, 3]);
        let out = written(text, |settings| settings.page_size = 100);
        assert_eq!(
            out,
            "[data]\ntimestamps = full\ngroup_digits = \"yes\"\npage_size = 100\n"
        );
        assert_eq!(Settings::from_toml(&out).invalid, [2, 3]);
        // A key whose value was refused gets the new one in its place.
        assert_eq!(
            written(text, |settings| settings.group_digits = Some(true)),
            "[data]\ntimestamps = full\ngroup_digits = true\npage_size = 500\n"
        );
    }

    #[test]
    fn a_file_with_crlf_endings_gets_crlf_lines() {
        let text = "[data]\r\npage_size = 500\r\n";
        assert_eq!(
            written(text, |settings| settings.value_tags = false),
            "[data]\r\npage_size = 500\r\nvalue_tags = false\r\n"
        );
    }

    #[test]
    fn an_inline_table_gets_values_in_place_and_otherwise_the_whole_file() {
        let text = "data = { page_size = 500 } # inline\n";
        assert_eq!(
            written(text, |settings| settings.page_size = 100),
            "data = { page_size = 100 } # inline\n"
        );
        let mut settings = Settings::from_toml(text).settings;
        settings.value_tags = false;
        let out = written(text, |changed| changed.value_tags = false);
        assert_eq!(out, settings.to_toml());
    }

    #[test]
    fn a_value_that_is_ignored_keeps_its_last_good_value() {
        let last = Settings {
            page_size: 500,
            timestamps: Timestamps::Full,
            ..Settings::default()
        };
        // A value of the wrong type, and a line the parser rejects.
        let text = "[data]\npage_size = \"lots\"\ntimestamps = full\n";
        let loaded = Settings::from_toml_over(text, &last);
        assert_eq!(loaded.invalid, [2, 3]);
        assert_eq!(loaded.settings.page_size, 500);
        assert_eq!(loaded.settings.timestamps, Timestamps::Full);
        // Read with nothing before it, the last good value is the default.
        let loaded = Settings::from_toml(text);
        assert_eq!(loaded.settings.page_size, Settings::DEFAULT_PAGE_SIZE);
        assert_eq!(loaded.settings.timestamps, Timestamps::Second);
    }

    #[test]
    fn a_key_taken_out_of_the_file_takes_its_default() {
        let last = Settings {
            page_size: 500,
            ..Settings::default()
        };
        let loaded = Settings::from_toml_over("[data]\nvalue_tags = false\n", &last);
        assert_eq!(loaded.settings.page_size, Settings::DEFAULT_PAGE_SIZE);
        assert!(!loaded.settings.value_tags);
    }

    #[test]
    fn a_rejected_line_is_taken_for_the_key_of_the_table_above_it() {
        assert_eq!(
            key_on_line("[data] # rows\npage_size = 1 2\n", "[data] # rows\n", 2),
            Some(Key::PageSize)
        );
        assert_eq!(
            key_on_line("[editor]\npage_size = x y\n", "[editor]\n", 2),
            None
        );
        assert_eq!(key_on_line("[data]\npage_size\n", "[data]\n", 2), None);
        assert_eq!(key_on_line("page_size = x y\n", "", 1), None);
        // In quotes, dotted, and under a header after a byte order mark.
        assert_eq!(
            key_on_line("[data]\n\"page_size\" = x y\n", "[data]\n", 2),
            Some(Key::PageSize)
        );
        assert_eq!(
            key_on_line("data.page_size = x y\n", "", 1),
            Some(Key::PageSize)
        );
        assert_eq!(
            key_on_line("\u{feff}[data]\npage_size = x y\n", "\u{feff}[data]\n", 2),
            Some(Key::PageSize)
        );
        assert_eq!(
            key_on_line("[data]\nx.page_size = 1 2\n", "[data]\n", 2),
            None
        );
    }

    #[test]
    fn a_text_too_long_to_read_keeps_every_last_good_value() {
        let last = Settings {
            page_size: 500,
            ..Settings::default()
        };
        let text = "# note\n".repeat(MAX_LINES + 1);
        assert_eq!(Settings::from_toml_over(&text, &last).settings, last);
    }

    #[test]
    fn a_table_with_a_line_the_reader_ignored_stays() {
        let text = "[data]\npage_size = 500\nnot toml at all\n";
        assert_eq!(
            written(text, |settings| settings.page_size =
                Settings::DEFAULT_PAGE_SIZE),
            "[data]\nnot toml at all\n"
        );
        // A header given twice, the second rejected: it stays rejected.
        let text = "[data]\ntimestamps = \"full\"\n[data]\n";
        let out = written(text, |settings| settings.timestamps = Timestamps::Second);
        assert_eq!(out, "[data]\n[data]\n");
        assert_eq!(Settings::from_toml(&out).invalid, [2]);
    }

    #[test]
    fn a_comment_beside_a_header_keeps_its_table() {
        let text = "[data] # my table\npage_size = 500\n";
        assert_eq!(
            written(text, |settings| settings.page_size =
                Settings::DEFAULT_PAGE_SIZE),
            "[data] # my table\n"
        );
    }

    #[test]
    fn a_file_left_with_nothing_stays_empty() {
        assert_eq!(
            written("[data]\npage_size = 500\n", |settings| {
                settings.page_size = Settings::DEFAULT_PAGE_SIZE;
            }),
            ""
        );
    }

    #[test]
    fn a_key_with_a_rejected_twin_is_set_to_its_default_in_place() {
        // The second line is rejected: taking out the first would make it
        // the one read.
        let text = "[data]\npage_size = 500\npage_size = 100\n";
        let out = written(text, |settings| {
            settings.page_size = Settings::DEFAULT_PAGE_SIZE
        });
        assert_eq!(
            out,
            format!(
                "[data]\npage_size = {}\npage_size = 100\n",
                Settings::DEFAULT_PAGE_SIZE
            )
        );
        assert_eq!(
            Settings::from_toml(&out).settings.page_size,
            Settings::DEFAULT_PAGE_SIZE
        );
    }

    #[test]
    fn a_table_only_a_table_under_it_names_gets_a_header_at_the_end() {
        let text = "# mine\n[data.more]\ny = 1\n";
        let out = written(text, |settings| settings.page_size = 100);
        assert_eq!(
            out,
            "# mine\n[data.more]\ny = 1\n\n[data]\npage_size = 100\n"
        );
        assert_eq!(Settings::from_toml(&out).settings.page_size, 100);
    }

    #[test]
    fn a_name_that_is_a_value_is_left_as_the_person_wrote_it() {
        for text in ["# mine\ndata = 5\n", "# mine\n[[data]]\nx = 1\n"] {
            assert_eq!(written(text, |settings| settings.page_size = 100), text);
        }
    }

    #[test]
    fn a_table_name_given_a_value_keeps_its_keys_last_good_values() {
        let last = Settings {
            page_size: 100,
            ..Settings::default()
        };
        for text in ["data = 5\n", "[[data]]\npage_size = 500\n"] {
            let loaded = Settings::from_toml_over(text, &last);
            assert_eq!(loaded.settings.page_size, 100, "{text:?}");
        }
    }

    #[test]
    fn a_new_key_goes_after_a_key_whose_value_is_an_array() {
        let text = "[data]\npage_size = 500\nmine = [1, 2]\n";
        assert_eq!(
            written(text, |settings| settings.value_tags = false),
            "[data]\npage_size = 500\nmine = [1, 2]\nvalue_tags = false\n"
        );
        let text = "[data]\nmine = [1, 2]\n";
        assert_eq!(
            written(text, |settings| settings.value_tags = false),
            "[data]\nmine = [1, 2]\nvalue_tags = false\n"
        );
    }

    #[test]
    fn a_rejected_line_in_quotes_or_dotted_keeps_the_last_good_value() {
        let last = Settings {
            timestamps: Timestamps::Full,
            ..Settings::default()
        };
        for text in [
            "[data]\n\"timestamps\" = full\n",
            "data.timestamps = full\n",
            "\u{feff}[data]\ntimestamps = full\n",
        ] {
            let loaded = Settings::from_toml_over(text, &last);
            assert_eq!(loaded.settings.timestamps, Timestamps::Full, "{text:?}");
        }
    }

    #[test]
    fn numbers_not_set_are_grouped_as_the_look_has_them() {
        let settings = Settings::default();
        assert!(settings.grouped(Layout::Mac));
        assert!(settings.grouped(Layout::Windows));
        assert!(!settings.grouped(Layout::Omarchy));
        let set = Settings {
            group_digits: Some(false),
            ..Settings::default()
        };
        assert!(!set.grouped(Layout::Mac));
    }

    #[test]
    fn what_the_look_has_anyway_is_not_set() {
        let grouped = Settings {
            group_digits: Some(true),
            ..Settings::default()
        };
        assert_eq!(grouped.clone().for_layout(Layout::Mac).group_digits, None);
        assert_eq!(grouped.for_layout(Layout::Omarchy).group_digits, Some(true));
        // Not set, nothing is written.
        assert_eq!(Settings::default().written(Key::GroupDigits), None);
    }

    #[test]
    fn a_zone_is_system_utc_or_a_name_the_database_knows() {
        assert_eq!(Zone::from_name("system"), Some(Zone::System));
        assert_eq!(Zone::from_name("UTC"), Some(Zone::Utc));
        assert_eq!(
            Zone::from_name("Europe/Lisbon"),
            Some(Zone::Named("Europe/Lisbon".into()))
        );
        assert_eq!(Zone::from_name("Mars/Olympus"), None);
        assert_eq!(Zone::from_name(""), None);
        for zone in [Zone::System, Zone::Utc, Zone::Named("Asia/Tokyo".into())] {
            assert_eq!(Zone::from_name(zone.name()), Some(zone));
        }
        let system = jiff::tz::TimeZone::fixed(jiff::tz::offset(3));
        assert_eq!(Zone::System.resolve(&system), system);
        assert_eq!(Zone::Utc.resolve(&system), jiff::tz::TimeZone::UTC);
        assert_eq!(
            Zone::Named("Asia/Tokyo".into())
                .resolve(&system)
                .iana_name(),
            Some("Asia/Tokyo")
        );
    }

    #[test]
    fn a_zone_the_database_does_not_know_is_ignored_and_keeps_the_last_good_one() {
        let last = Settings {
            tz: Zone::Utc,
            ..Settings::default()
        };
        let text = "[data]\ntz = \"Mars/Olympus\"\npage_size = 500\n";
        let loaded = Settings::from_toml_over(text, &last);
        assert_eq!(loaded.invalid, [2]);
        assert_eq!(loaded.settings.tz, Zone::Utc);
        assert_eq!(loaded.settings.page_size, 500);
    }

    #[test]
    fn a_zone_goes_in_the_data_table_with_its_note() {
        assert_eq!(
            written("[data]\npage_size = 500\n", |settings| {
                settings.tz = Zone::Named("Europe/Lisbon".into());
            }),
            "[data]\npage_size = 500\ntz = \"Europe/Lisbon\"  # system | utc | \"Area/City\"\n"
        );
        // Back to the computer's own, which is not written.
        assert_eq!(
            written("[data]\ntz = \"utc\"\n", |settings| settings.tz =
                Zone::System),
            ""
        );
    }

    #[test]
    fn an_empty_file_starts_with_the_header() {
        assert_eq!(
            written("", |settings| settings.page_size = 500),
            "# written by tabletist, safe to edit by hand\n[data]\npage_size = 500\n"
        );
    }

    #[test]
    fn a_missing_key_keeps_its_default_and_an_unknown_one_is_ignored() {
        let loaded = Settings::from_toml(
            "[data]\npage_size = 100\nfrom_the_future = true\n\n[tomorrow]\nkey = 1\n",
        );
        assert_eq!(
            loaded.settings,
            Settings {
                page_size: 100,
                ..Settings::default()
            }
        );
        assert_eq!(loaded.invalid, Vec::<usize>::new());
        assert_eq!(loaded.lines, vec![(Key::PageSize, 2)]);
    }

    #[test]
    fn a_value_the_key_cannot_have_is_ignored_by_its_line() {
        let loaded = Settings::from_toml(
            "[data]\npage_size = \"lots\"\ntimestamps = \"minute\"\ngroup_digits = \"yes\"\nvalue_tags = false\n",
        );
        assert_eq!(loaded.invalid, vec![2, 3, 4]);
        assert_eq!(
            loaded.settings,
            Settings {
                value_tags: false,
                ..Settings::default()
            }
        );
        assert_eq!(loaded.lines, vec![(Key::ValueTags, 5)]);
    }

    #[test]
    fn a_line_that_is_not_toml_is_ignored_and_the_rest_applies() {
        let text = "[data]\npage_size = 100\nthis is not toml\ngroup_digits = true\n\n\
                    [editor]\nsql_limit = 100\n";
        let loaded = Settings::from_toml(text);
        assert_eq!(loaded.invalid, vec![3]);
        assert_eq!(loaded.settings.page_size, 100);
        // The lines and the tables under the bad one are still read.
        assert_eq!(loaded.settings.group_digits, Some(true));
        assert_eq!(loaded.settings.sql_limit, 100);
        assert_eq!(
            loaded.lines,
            vec![
                (Key::PageSize, 2),
                (Key::GroupDigits, 4),
                (Key::SqlLimit, 7)
            ]
        );
        // What the pane will show is the file, not what was parsed.
        assert_eq!(loaded.text, text);
    }

    #[test]
    fn a_line_the_parser_rejects_is_not_applied_on_a_guess() {
        // Each of these the parser reports and still gives a value for.
        let loaded = Settings::from_toml(
            "[data]\ntimestamps = full\nvalue_tags = False\n\n[appearance]\ntheme = Nord.json\n",
        );
        assert_eq!(loaded.invalid, vec![2, 3, 6]);
        assert_eq!(loaded.settings, Settings::default());
        assert!(loaded.lines.is_empty());
    }

    #[test]
    fn a_key_given_twice_keeps_its_first_value() {
        let loaded = Settings::from_toml("[data]\npage_size = 100\npage_size = 500\n");
        assert_eq!(loaded.invalid, vec![3]);
        assert_eq!(loaded.settings.page_size, 100);
        assert_eq!(loaded.lines, vec![(Key::PageSize, 2)]);
    }

    #[test]
    fn numbers_out_of_range_are_clamped_and_not_invalid() {
        let loaded = Settings::from_toml(
            "[data]\npage_size = 0\n\n[editor]\nsql_limit = 999999999\nsql_timeout_secs = 0\n",
        );
        assert_eq!(loaded.invalid, Vec::<usize>::new());
        assert_eq!(loaded.settings.page_size, Settings::MIN_PAGE_SIZE);
        assert_eq!(loaded.settings.sql_limit, Settings::MAX_SQL_LIMIT);
        assert_eq!(loaded.settings.sql_timeout_secs, None);
        let loaded = Settings::from_toml("[data]\npage_size = -5\n");
        assert_eq!(loaded.invalid, Vec::<usize>::new());
        assert_eq!(loaded.settings.page_size, Settings::MIN_PAGE_SIZE);
        let loaded = Settings::from_toml("[data]\npage_size = 999999999\n");
        assert_eq!(loaded.settings.page_size, Settings::MAX_PAGE_SIZE);
        // The editor's own floor, and a timeout that is one.
        let loaded = Settings::from_toml("[editor]\nsql_limit = 0\nsql_timeout_secs = 1\n");
        assert_eq!(loaded.settings.sql_limit, 1);
        assert_eq!(loaded.settings.sql_timeout_secs, Some(1));
        let loaded = Settings::from_toml("[editor]\nsql_timeout_secs = 0\n");
        assert_eq!(loaded.settings.sql_timeout(), None);
        // The log of agents' calls keeps a call a day at least, and ten
        // years at most.
        let loaded = Settings::from_toml("[mcp]\nlog_days = 0\n");
        assert_eq!(loaded.settings.mcp_log_days, Settings::MIN_MCP_LOG_DAYS);
        let loaded = Settings::from_toml("[mcp]\nlog_days = 100000\n");
        assert_eq!(loaded.settings.mcp_log_days, Settings::MAX_MCP_LOG_DAYS);
    }

    #[test]
    fn agents_are_turned_on_in_the_file_and_nothing_else_turns_them_on() {
        let loaded = Settings::from_toml("[mcp]\nenabled = true\nlog_days = 90\n");
        assert_eq!(loaded.invalid, Vec::<usize>::new());
        assert!(loaded.settings.mcp_enabled);
        assert_eq!(loaded.settings.mcp_log_days, 90);
        // A value that is no flag is no yes.
        for text in ["[mcp]\nenabled = \"true\"\n", "[mcp]\nenabled = 1\n"] {
            let loaded = Settings::from_toml(text);
            assert_eq!(loaded.invalid, vec![2], "{text}");
            assert!(!loaded.settings.mcp_enabled, "{text}");
        }
    }

    #[test]
    fn an_empty_text_is_the_defaults() {
        let loaded = Settings::from_toml("");
        assert_eq!(loaded.settings, Settings::default());
        assert_eq!(loaded.invalid, Vec::<usize>::new());
        assert!(loaded.lines.is_empty());
    }

    #[test]
    fn settings_from_no_toml_file_carry_the_text_that_would_be_written() {
        let settings = Settings {
            page_size: 500,
            ..Settings::default()
        };
        let loaded = Loaded::of(settings.clone(), Source::Json);
        assert_eq!(loaded.source, Source::Json);
        assert_eq!(loaded.text, settings.to_toml());
        assert_eq!(loaded.lines.first(), Some(&(Key::PageSize, 3)));
        let defaults: Loaded = Settings::default().into();
        assert_eq!(defaults.source, Source::Defaults);
        assert_eq!(defaults.settings, Settings::default());
    }

    #[test]
    fn settings_from_no_toml_file_are_in_range_like_their_text() {
        let loaded = Loaded::of(
            Settings {
                page_size: 5,
                sql_timeout_secs: Some(0),
                ..Settings::default()
            },
            Source::Json,
        );
        assert_eq!(loaded.settings.page_size, Settings::MIN_PAGE_SIZE);
        assert_eq!(loaded.settings.sql_timeout_secs, None);
        // The text says what the settings are.
        assert_eq!(Settings::from_toml(&loaded.text).settings, loaded.settings);
    }

    #[test]
    fn a_text_too_long_for_a_settings_file_is_not_read_at_all() {
        // A key with no `=` on every line: the parser stops at each, so
        // reading this would take a pass per line.
        let text = format!(
            "[data]\npage_size = 100\n\n{}",
            "not toml\n".repeat(MAX_LINES)
        );
        let loaded = Settings::from_toml(&text);
        assert_eq!(loaded.settings, Settings::default());
        assert!(loaded.lines.is_empty());
        assert_eq!(loaded.text, text);
        // Every line that says something is marked; the blank one is not.
        assert_eq!(loaded.invalid.len(), MAX_LINES + 2);
        assert_eq!(loaded.invalid[..3], [1, 2, 4]);
        assert_eq!(loaded.invalid.last(), Some(&(MAX_LINES + 3)));
        // Few lines, but more bytes than a settings file has.
        let wide = format!("[data]\npage_size = 100\n# {}\n", "x".repeat(MAX_BYTES));
        let loaded = Settings::from_toml(&wide);
        assert_eq!(loaded.settings, Settings::default());
        assert_eq!(loaded.invalid, vec![1, 2, 3]);
    }

    #[test]
    fn a_text_of_the_most_lines_a_settings_file_has_is_read() {
        let text = format!("[data]\npage_size = 100\n{}", "\n".repeat(MAX_LINES - 2));
        assert_eq!(text.lines().count(), MAX_LINES);
        let loaded = Settings::from_toml(&text);
        assert_eq!(loaded.settings.page_size, 100);
        assert!(loaded.invalid.is_empty());
    }

    #[test]
    fn what_the_app_holds_of_the_file_comes_from_what_was_loaded() {
        let text = "[data]\npage_size = 100\ngroup_digits = \"yes\"\n";
        let (settings, file) = Settings::from_toml(text).into_parts();
        assert_eq!(settings.page_size, 100);
        assert_eq!(
            file,
            SettingsFile {
                text: text.into(),
                line_starts: vec![0, 7, 23],
                invalid: vec![3],
                lines: vec![(Key::PageSize, 2)],
                live: false,
                saved: None,
                offered: Vec::new(),
            }
        );
        assert_eq!(file.line_count(), 3);
    }

    #[test]
    fn the_lines_the_app_holds_are_the_ones_the_text_has() {
        // Each kind of ending, a line with nothing on it, and a last line
        // with no ending and one with.
        for text in [
            "",
            "one",
            "one\n",
            "one\ntwo",
            "one\r\ntwo\r\n",
            "\n\none\n\n",
            "a\rb\nc",
            "ключ = 1\nä\n",
        ] {
            let (_, file) = Settings::from_toml(text).into_parts();
            let lines: Vec<&str> = (0..file.line_count())
                .filter_map(|index| file.line(index))
                .collect();
            assert_eq!(lines, text.lines().collect::<Vec<_>>(), "{text:?}");
            assert_eq!(file.line(file.line_count()), None, "{text:?}");
        }
    }

    #[test]
    fn a_byte_order_mark_is_not_a_bad_line() {
        // Windows editors write one; the parser takes it in its stride.
        let loaded = Settings::from_toml("\u{feff}[data]\npage_size = 100\n");
        assert_eq!(loaded.invalid, Vec::<usize>::new());
        assert_eq!(loaded.settings.page_size, 100);
        assert_eq!(loaded.lines, vec![(Key::PageSize, 2)]);
    }

    #[test]
    fn a_file_with_crlf_endings_is_read_line_by_line() {
        let loaded = Settings::from_toml(
            "[data]\r\npage_size = 100\r\nthis is not toml\r\ngroup_digits = true\r\n",
        );
        assert_eq!(loaded.invalid, vec![3]);
        assert_eq!(loaded.settings.page_size, 100);
        assert_eq!(loaded.settings.group_digits, Some(true));
        assert_eq!(
            loaded.lines,
            vec![(Key::PageSize, 2), (Key::GroupDigits, 4)]
        );
    }

    #[test]
    fn a_header_that_is_rejected_costs_the_keys_under_it() {
        let loaded = Settings::from_toml("[data]\npage_size = 100\n[editor\nsql_limit = 100\n");
        assert_eq!(loaded.invalid, vec![3]);
        assert_eq!(loaded.settings.page_size, 100);
        // With its header gone the key is read into the table above, where
        // it is one this version does not know: dropped, and not marked.
        assert_eq!(loaded.settings.sql_limit, Settings::default().sql_limit);
        assert_eq!(loaded.lines, vec![(Key::PageSize, 2)]);
    }

    #[test]
    fn a_string_left_open_is_not_applied_and_costs_the_lines_under_it() {
        // The parser blames the end of the text and guesses a theme of
        // everything under the quotes.
        let text = "[data]\npage_size = 100\n[appearance]\ntheme = \"\"\"Nord\n\
                    [editor]\nsql_limit = 100\n";
        let loaded = Settings::from_toml(text);
        assert_eq!(loaded.invalid, vec![4, 5, 6]);
        assert_eq!(loaded.settings.custom_theme, None);
        assert_eq!(loaded.settings.page_size, 100);
        assert_eq!(loaded.settings.sql_limit, Settings::default().sql_limit);
        assert_eq!(loaded.lines, vec![(Key::PageSize, 2)]);
        // Only lines the file has.
        let count = text.lines().count();
        assert!(
            loaded.invalid.iter().all(|line| *line <= count),
            "{:?}",
            loaded.invalid
        );
        // The same on a last line with no break after it.
        let loaded = Settings::from_toml("[appearance]\ntheme = \"\"\"Nord");
        assert_eq!(loaded.invalid, vec![2]);
        assert_eq!(loaded.settings.custom_theme, None);
    }

    #[test]
    fn a_string_left_open_costs_the_same_lines_with_crlf_endings() {
        // The empty lines at the end are blank with either ending.
        let loaded = Settings::from_toml(
            "[data]\r\npage_size = 100\r\n[appearance]\r\ntheme = \"\"\"Nord\r\n\
             [editor]\r\nsql_limit = 100\r\n\r\n\r\n",
        );
        assert_eq!(loaded.invalid, vec![4, 5, 6]);
        assert_eq!(loaded.settings.custom_theme, None);
        assert_eq!(loaded.settings.page_size, 100);
        assert_eq!(loaded.lines, vec![(Key::PageSize, 2)]);
    }

    #[test]
    fn a_lone_carriage_return_at_the_end_costs_only_its_line() {
        // The parser blames the end of the text for it.
        let loaded = Settings::from_toml("[data]\npage_size = 100\ngroup_digits = true\n\r");
        assert_eq!(loaded.invalid, vec![4]);
        assert_eq!(loaded.settings.page_size, 100);
        assert_eq!(loaded.settings.group_digits, Some(true));
        let loaded = Settings::from_toml("\r");
        assert_eq!(loaded.invalid, vec![1]);
        assert_eq!(loaded.settings, Settings::default());
    }

    #[test]
    fn a_last_line_of_space_toml_does_not_know_costs_only_itself() {
        // A no-break space is blank to Rust and a stray character to TOML.
        let loaded = Settings::from_toml("[data]\npage_size = 100\ngroup_digits = true\n\u{a0}");
        assert_eq!(loaded.invalid, vec![4]);
        assert_eq!(loaded.settings.page_size, 100);
        assert_eq!(loaded.settings.group_digits, Some(true));
    }

    fn dirs() -> (crate::paths::AppDirs, tempfile::TempDir) {
        let root = tempfile::tempdir().unwrap();
        let dirs = crate::paths::AppDirs::at(root.path());
        dirs.ensure().unwrap();
        (dirs, root)
    }

    #[test]
    fn no_file_gives_the_defaults_and_writes_nothing() {
        let (dirs, _root) = dirs();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.settings, Settings::default());
        assert_eq!(loaded.source, Source::Defaults);
        assert_eq!(loaded.text, Settings::default().to_toml());
        assert!(!dirs.settings_file().exists());
    }

    #[test]
    fn what_is_saved_is_loaded() {
        let (dirs, _root) = dirs();
        let settings = Settings {
            page_size: 500,
            group_digits: Some(true),
            sql_timeout_secs: None,
            ..Settings::default()
        };
        crate::util::write_atomic(&dirs.settings_file(), settings.to_toml().as_bytes()).unwrap();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.settings, settings);
        assert_eq!(loaded.source, Source::Toml);
        assert_eq!(loaded.text, settings.to_toml());
    }

    #[test]
    fn only_the_old_json_is_read_and_left_as_it_was() {
        let (dirs, _root) = dirs();
        let json = br#"{"version": 1, "page_size": 100, "show_system_schemas": true,
            "custom_theme": "Nord.json", "sql_limit": 100, "sql_timeout_secs": null,
            "from_the_future": true}"#;
        std::fs::write(dirs.legacy_settings_file(), json).unwrap();
        let loaded = Settings::load(&dirs);
        let expected = Settings {
            page_size: 100,
            show_system_schemas: true,
            custom_theme: Some("Nord.json".into()),
            sql_limit: 100,
            sql_timeout_secs: None,
            ..Settings::default()
        };
        assert_eq!(loaded.settings, expected);
        assert_eq!(loaded.source, Source::Json);
        assert_eq!(loaded.text, expected.to_toml());
        // Reading writes nothing, and an older Tabletist still finds its file.
        assert!(!dirs.settings_file().exists());
        assert_eq!(std::fs::read(dirs.legacy_settings_file()).unwrap(), json);
    }

    #[test]
    fn the_old_json_is_brought_into_range_as_before() {
        let (dirs, _root) = dirs();
        std::fs::write(
            dirs.legacy_settings_file(),
            br#"{"page_size": 0, "sql_limit": 999999999, "sql_timeout_secs": 0}"#,
        )
        .unwrap();
        let settings = Settings::load(&dirs).settings;
        assert_eq!(settings.page_size, Settings::MIN_PAGE_SIZE);
        assert_eq!(settings.sql_limit, Settings::MAX_SQL_LIMIT);
        assert_eq!(Settings::MAX_SQL_LIMIT, 10_000);
        assert_eq!(settings.sql_timeout_secs, None);
        assert!(
            Settings::SQL_LIMITS
                .iter()
                .all(|limit| (1..=Settings::MAX_SQL_LIMIT).contains(limit)),
            "every choice of the Limit menu loads as it was saved"
        );
    }

    #[test]
    fn an_old_json_with_keys_missing_gets_their_defaults() {
        let (dirs, _root) = dirs();
        std::fs::write(dirs.legacy_settings_file(), br#"{"page_size": 100}"#).unwrap();
        assert_eq!(
            Settings::load(&dirs).settings,
            Settings {
                page_size: 100,
                ..Settings::default()
            }
        );
    }

    #[test]
    fn with_both_files_the_toml_is_the_one_read() {
        let (dirs, _root) = dirs();
        std::fs::write(dirs.legacy_settings_file(), br#"{"page_size": 100}"#).unwrap();
        std::fs::write(dirs.settings_file(), "[data]\npage_size = 500\n").unwrap();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.settings.page_size, 500);
        assert_eq!(loaded.source, Source::Toml);
        assert_eq!(loaded.text, "[data]\npage_size = 500\n");
    }

    #[test]
    fn a_damaged_old_json_is_kept_aside_and_the_defaults_used() {
        let (dirs, _root) = dirs();
        std::fs::write(dirs.legacy_settings_file(), br#"{"page_size": "lots"}"#).unwrap();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.settings, Settings::default());
        assert_eq!(loaded.source, Source::Json);
        assert!(dirs.config.join("settings.json.bad").exists());
    }

    #[test]
    fn a_file_that_is_not_text_is_kept_aside_and_the_defaults_used() {
        let (dirs, _root) = dirs();
        std::fs::write(dirs.settings_file(), [0xff, 0xfe, 0x00]).unwrap();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.settings, Settings::default());
        assert_eq!(loaded.source, Source::Defaults);
        assert!(dirs.config.join("settings.toml.bad").exists());
        assert!(!dirs.settings_file().exists());
    }

    #[test]
    fn a_file_that_cannot_be_read_is_kept_aside_and_the_defaults_used() {
        let (dirs, _root) = dirs();
        // A directory is a file no platform can read.
        std::fs::create_dir(dirs.settings_file()).unwrap();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.settings, Settings::default());
        assert_eq!(loaded.source, Source::Defaults);
        assert!(dirs.config.join("settings.toml.bad").exists());
        assert!(!dirs.settings_file().exists());
    }

    #[test]
    fn a_file_with_a_bad_line_is_left_as_the_user_wrote_it() {
        let (dirs, _root) = dirs();
        let text = "[data]\npage_size = 100\ngroup_digits = \"yes\"\n";
        std::fs::write(dirs.settings_file(), text).unwrap();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.invalid, vec![3]);
        assert_eq!(loaded.settings.page_size, 100);
        assert_eq!(std::fs::read_to_string(dirs.settings_file()).unwrap(), text);
    }

    #[test]
    fn an_option_is_read_and_set_as_a_value() {
        let mut settings = Settings::default();
        for (option, value) in [
            (OptionId::PageSize, OptionValue::PageSize(500)),
            (
                OptionId::Timestamps,
                OptionValue::Timestamps(Timestamps::Full),
            ),
            (OptionId::GroupDigits, OptionValue::GroupDigits(Some(true))),
            (OptionId::ValueTags, OptionValue::ValueTags(false)),
        ] {
            assert_eq!(value.option(), option);
            assert_ne!(settings.value(option), value);
            value.set(&mut settings);
            assert_eq!(settings.value(option), value);
        }
        assert_eq!(settings.page_size, 500);
        assert_eq!(settings.timestamps, Timestamps::Full);
        assert_eq!(settings.group_digits, Some(true));
        assert!(!settings.value_tags);
    }

    #[test]
    fn every_option_is_stored_under_its_own_key_and_starts_at_the_default() {
        let keys: Vec<Key> = OptionId::ALL.iter().map(|option| option.key()).collect();
        assert_eq!(
            keys,
            vec![
                Key::PageSize,
                Key::Timestamps,
                Key::GroupDigits,
                Key::ValueTags,
                Key::McpEnabled,
                Key::McpLogDays,
                Key::McpNotify,
            ]
        );
        // Each pane has its own, and every one is on a pane.
        let panes = [&OptionId::GENERAL[..], &OptionId::AGENTS[..]].concat();
        assert_eq!(panes, OptionId::ALL);
        let defaults = Settings::default();
        for option in OptionId::ALL {
            assert_eq!(option.default_value(), defaults.value(option));
        }
    }

    #[test]
    fn the_page_size_steps_through_its_choices_and_stops_at_the_ends() {
        let step = |size: u32, forward: bool| {
            let settings = Settings {
                page_size: size,
                ..Settings::default()
            };
            settings.stepped(OptionId::PageSize, forward)
        };
        assert_eq!(step(1_000, true), OptionValue::PageSize(5_000));
        assert_eq!(step(500, false), OptionValue::PageSize(100));
        assert_eq!(step(100, false), OptionValue::PageSize(100));
        assert_eq!(step(10_000, true), OptionValue::PageSize(10_000));
        // A size set by hand is between two choices: it steps to the next.
        assert_eq!(step(300, true), OptionValue::PageSize(500));
        assert_eq!(step(300, false), OptionValue::PageSize(100));
        assert_eq!(step(9_000, true), OptionValue::PageSize(10_000));
        assert_eq!(step(9_000, false), OptionValue::PageSize(5_000));
    }

    #[test]
    fn the_logs_days_step_through_their_choices_and_stop_at_the_ends() {
        let step = |days: u32, forward: bool| {
            let settings = Settings {
                mcp_log_days: days,
                ..Settings::default()
            };
            settings.stepped(OptionId::LogDays, forward)
        };
        assert_eq!(step(30, true), OptionValue::LogDays(90));
        assert_eq!(step(30, false), OptionValue::LogDays(7));
        assert_eq!(step(7, false), OptionValue::LogDays(7));
        assert_eq!(step(365, true), OptionValue::LogDays(365));
        // Days set by hand are between two choices.
        assert_eq!(step(14, true), OptionValue::LogDays(30));
        assert_eq!(step(1_000, false), OptionValue::LogDays(365));
        // Nothing to flip: there are more than two.
        assert_eq!(Settings::default().flipped(OptionId::LogDays), None);
    }

    #[test]
    fn an_option_of_two_values_steps_to_the_side_it_is_drawn_on() {
        let settings = Settings::default();
        // Timestamps: to the second on the left, full on the right.
        assert_eq!(
            settings.stepped(OptionId::Timestamps, true),
            OptionValue::Timestamps(Timestamps::Full)
        );
        assert_eq!(
            settings.stepped(OptionId::Timestamps, false),
            OptionValue::Timestamps(Timestamps::Second)
        );
        // Numbers: grouped on the left, plain on the right.
        assert_eq!(
            settings.stepped(OptionId::GroupDigits, false),
            OptionValue::GroupDigits(Some(true))
        );
        assert_eq!(
            settings.stepped(OptionId::GroupDigits, true),
            OptionValue::GroupDigits(Some(false))
        );
        // A check: off to the left, on to the right.
        assert_eq!(
            settings.stepped(OptionId::ValueTags, false),
            OptionValue::ValueTags(false)
        );
        assert_eq!(
            settings.stepped(OptionId::ValueTags, true),
            OptionValue::ValueTags(true)
        );
    }

    #[test]
    fn space_flips_an_option_of_two_values_and_leaves_the_page_size() {
        let settings = Settings::default();
        assert_eq!(
            settings.flipped(OptionId::ValueTags),
            Some(OptionValue::ValueTags(false))
        );
        assert_eq!(
            settings.flipped(OptionId::GroupDigits),
            Some(OptionValue::GroupDigits(Some(true)))
        );
        assert_eq!(
            settings.flipped(OptionId::Timestamps),
            Some(OptionValue::Timestamps(Timestamps::Full))
        );
        assert_eq!(settings.flipped(OptionId::PageSize), None);
        // And back, from the other value of each.
        let settings = Settings {
            value_tags: false,
            group_digits: Some(true),
            timestamps: Timestamps::Full,
            ..Default::default()
        };
        assert_eq!(
            settings.flipped(OptionId::ValueTags),
            Some(OptionValue::ValueTags(true))
        );
        assert_eq!(
            settings.flipped(OptionId::GroupDigits),
            Some(OptionValue::GroupDigits(Some(false)))
        );
        assert_eq!(
            settings.flipped(OptionId::Timestamps),
            Some(OptionValue::Timestamps(Timestamps::Second))
        );
        assert_eq!(settings.flipped(OptionId::PageSize), None);
    }
}
