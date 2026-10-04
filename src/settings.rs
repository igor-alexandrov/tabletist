//! User settings, stored as `settings.toml` in the config directory.

use std::path::Path;

use serde::Deserialize;

use crate::paths::AppDirs;

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
    /// Numbers in grid cells with their integer digits in threes. Display
    /// only: a copy gives the value as it is.
    pub group_digits: bool,
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
}

/// A key of the settings file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    PageSize,
    Timestamps,
    GroupDigits,
    ValueTags,
    ShowSystemSchemas,
    SqlLimit,
    SqlTimeoutSecs,
    Theme,
}

impl Key {
    /// In the order the file has them: a table's keys together.
    pub const ALL: [Key; 8] = [
        Self::PageSize,
        Self::Timestamps,
        Self::GroupDigits,
        Self::ValueTags,
        Self::ShowSystemSchemas,
        Self::SqlLimit,
        Self::SqlTimeoutSecs,
        Self::Theme,
    ];

    /// The key's table and its name there.
    pub fn path(self) -> (&'static str, &'static str) {
        match self {
            Self::PageSize => ("data", "page_size"),
            Self::Timestamps => ("data", "timestamps"),
            Self::GroupDigits => ("data", "group_digits"),
            Self::ValueTags => ("data", "value_tags"),
            Self::ShowSystemSchemas => ("sidebar", "show_system_schemas"),
            Self::SqlLimit => ("editor", "sql_limit"),
            Self::SqlTimeoutSecs => ("editor", "sql_timeout_secs"),
            Self::Theme => ("appearance", "theme"),
        }
    }

    /// What the file says beside the key: the values of a closed set, or
    /// what a value that reads oddly means.
    fn note(self) -> Option<&'static str> {
        match self {
            Self::Timestamps => Some("second | full"),
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
}

impl OptionId {
    /// In the order the window has them.
    pub const ALL: [OptionId; 4] = [
        Self::PageSize,
        Self::Timestamps,
        Self::GroupDigits,
        Self::ValueTags,
    ];

    /// The key of the file the option is stored under.
    pub fn key(self) -> Key {
        match self {
            Self::PageSize => Key::PageSize,
            Self::Timestamps => Key::Timestamps,
            Self::GroupDigits => Key::GroupDigits,
            Self::ValueTags => Key::ValueTags,
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
    GroupDigits(bool),
    ValueTags(bool),
}

impl OptionValue {
    pub fn option(self) -> OptionId {
        match self {
            Self::PageSize(_) => OptionId::PageSize,
            Self::Timestamps(_) => OptionId::Timestamps,
            Self::GroupDigits(_) => OptionId::GroupDigits,
            Self::ValueTags(_) => OptionId::ValueTags,
        }
    }

    /// Puts the value in `settings`.
    pub fn set(self, settings: &mut Settings) {
        match self {
            Self::PageSize(size) => settings.page_size = size,
            Self::Timestamps(choice) => settings.timestamps = choice,
            Self::GroupDigits(on) => settings.group_digits = on,
            Self::ValueTags(on) => settings.value_tags = on,
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

/// `text` as a TOML basic string.
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control.is_control() => {
                out.push_str(&format!("\\u{:04X}", u32::from(control)));
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

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
    /// The lines that were ignored, counted from 1.
    pub invalid: Vec<usize>,
    /// The line each key is on.
    pub lines: Vec<(Key, usize)>,
    /// Whether the backend watches the file for changes made outside.
    pub live: bool,
    /// The text the app last asked to be written, if it asked: what tells
    /// its newest write, coming back from the disk, from an older one.
    pub saved: Option<String>,
}

impl Loaded {
    /// The settings, and what the app keeps of their file.
    pub fn into_parts(self) -> (Settings, SettingsFile) {
        let file = SettingsFile {
            text: self.text,
            invalid: self.invalid,
            lines: self.lines,
            live: false,
            saved: None,
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

/// `text` with the lines numbered in `lines` (from 1) emptied. Their line
/// breaks stay, so every other line keeps its number.
fn without_lines(text: &str, lines: &[usize]) -> String {
    text.split_inclusive('\n')
        .enumerate()
        .map(|(index, line)| {
            if !lines.contains(&(index + 1)) {
                line
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            }
        })
        .collect()
}

impl Settings {
    pub const DEFAULT_PAGE_SIZE: u32 = 300;
    pub const MIN_PAGE_SIZE: u32 = 10;
    pub const MAX_PAGE_SIZE: u32 = 10_000;
    /// The page sizes the Settings window offers.
    pub const PAGE_SIZES: [u32; 5] = [100, 300, 500, 1_000, 5_000];
    /// The most rows a SQL editor statement keeps.
    pub const MAX_SQL_LIMIT: u32 = 10_000;
    /// The row limits the SQL editor's Limit menu offers.
    pub const SQL_LIMITS: [u32; 3] = [100, 1_000, 10_000];
    /// The timeouts (seconds) the SQL editor's Timeout menu offers; `None`
    /// is no timeout.
    pub const SQL_TIMEOUTS: [Option<u32>; 5] = [Some(10), Some(30), Some(60), Some(300), None];

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
            OptionId::GroupDigits => OptionValue::GroupDigits(!forward),
            OptionId::ValueTags => OptionValue::ValueTags(forward),
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
            OptionId::GroupDigits => Some(OptionValue::GroupDigits(!self.group_digits)),
            OptionId::ValueTags => Some(OptionValue::ValueTags(!self.value_tags)),
        }
    }

    /// `key`'s value as TOML writes it. `None` for a key the file leaves
    /// out: a theme that is not set.
    fn literal(&self, key: Key) -> Option<String> {
        Some(match key {
            Key::PageSize => self.page_size.to_string(),
            Key::Timestamps => quote(self.timestamps.name()),
            Key::GroupDigits => self.group_digits.to_string(),
            Key::ValueTags => self.value_tags.to_string(),
            Key::ShowSystemSchemas => self.show_system_schemas.to_string(),
            Key::SqlLimit => self.sql_limit.to_string(),
            // TOML has no null: no timeout is no seconds.
            Key::SqlTimeoutSecs => self.sql_timeout_secs.unwrap_or(0).to_string(),
            Key::Theme => quote(self.custom_theme.as_deref()?),
        })
    }

    /// The file's text: the one form the app writes, the same every time.
    /// Tables in a fixed order, a table's keys aligned, and a note beside
    /// the keys that have one.
    pub fn to_toml(&self) -> String {
        let written: Vec<(Key, String)> = Key::ALL
            .into_iter()
            .filter_map(|key| Some((key, self.literal(key)?)))
            .collect();
        let mut out = String::from(HEADER);
        let mut open = None;
        for (key, literal) in &written {
            let (table, name) = key.path();
            if open != Some(table) {
                if open.is_some() {
                    out.push('\n');
                }
                out.push_str(&format!("[{table}]\n"));
                open = Some(table);
            }
            let width = written
                .iter()
                .filter(|(other, _)| other.path().0 == table)
                .map(|(other, _)| other.path().1.len())
                .max()
                .unwrap_or(0);
            out.push_str(&format!("{name:width$} = {literal}"));
            if let Some(note) = key.note() {
                out.push_str(&format!("  # {note}"));
            }
            out.push('\n');
        }
        out
    }

    /// The settings with every number in its range.
    fn validated(mut self) -> Self {
        self.page_size = self
            .page_size
            .clamp(Self::MIN_PAGE_SIZE, Self::MAX_PAGE_SIZE);
        self.sql_limit = Self::valid_sql_limit(self.sql_limit);
        self.sql_timeout_secs = Self::valid_sql_timeout(self.sql_timeout_secs);
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
            Key::GroupDigits => flag.map(|flag| self.group_digits = flag),
            Key::ValueTags => flag.map(|flag| self.value_tags = flag),
            Key::ShowSystemSchemas => flag.map(|flag| self.show_system_schemas = flag),
            Key::SqlLimit => number.map(|number| self.sql_limit = number),
            Key::SqlTimeoutSecs => number.map(|number| self.sql_timeout_secs = Some(number)),
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
        if text.len() > MAX_BYTES || text.lines().count() > MAX_LINES {
            let invalid = text
                .lines()
                .enumerate()
                .filter(|(_, line)| !line.trim_matches([' ', '\t']).is_empty())
                .map(|(index, _)| index + 1)
                .collect();
            return Loaded {
                settings: Settings::default(),
                text: text.to_owned(),
                invalid,
                lines: Vec::new(),
                source: Source::Toml,
            };
        }
        // The parser recovers from a bad line in two ways that are not
        // "ignored": it stops reading at some (a key with no `=`), and
        // reads past others with a guess at the value (`full` unquoted).
        // So a line it rejects is emptied and the text read again, until
        // it rejects nothing more.
        let mut working = text.to_owned();
        let mut invalid: Vec<usize> = Vec::new();
        loop {
            let (_, errors) = toml::de::DeTable::parse_recoverable(&working);
            let mut rejected = Vec::new();
            for error in &errors {
                match error.span() {
                    // The parser blames the end of the text for a value
                    // that never closes (`"""` with no end). That is no
                    // line of the file, so the last line with anything on
                    // it goes: the lines are emptied from the bottom up
                    // until the opening one is. With every line blank the
                    // error marks none.
                    Some(span) if span.start >= working.len() => {
                        rejected.extend(last_line(&working));
                    }
                    Some(span) => rejected.push(line_of(&working, span.start)),
                    None => log::warn!("settings: {}", error.message()),
                }
            }
            // A line already emptied has nothing left to reject: an error
            // that still names one ends the loop.
            rejected.retain(|line| !invalid.contains(line));
            rejected.sort_unstable();
            rejected.dedup();
            if rejected.is_empty() {
                break;
            }
            working = without_lines(&working, &rejected);
            invalid.extend(rejected);
        }
        let (document, _) = toml::de::DeTable::parse_recoverable(&working);
        let document = document.into_inner();
        let mut settings = Settings::default();
        let mut lines = Vec::new();
        for key in Key::ALL {
            let (table, name) = key.path();
            let Some((found, value)) = document
                .get(table)
                .and_then(|table| table.get_ref().as_table())
                .and_then(|table| table.get_key_value(name))
            else {
                continue;
            };
            // Emptied lines kept their breaks: a line has the same number
            // in `working` as in `text`.
            let line = line_of(&working, found.span().start);
            if settings.take(key, value.get_ref()) {
                lines.push((key, line));
            } else {
                invalid.push(line);
            }
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

    /// Writes the canonical text, atomically.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::util::write_atomic(path, self.to_toml().as_bytes())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            page_size: Self::DEFAULT_PAGE_SIZE,
            timestamps: Timestamps::Second,
            group_digits: false,
            value_tags: true,
            show_system_schemas: false,
            custom_theme: None,
            sql_limit: 1_000,
            sql_timeout_secs: Some(30),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let settings = Settings::default();
        assert_eq!(settings.page_size, 300);
        assert_eq!(settings.timestamps, Timestamps::Second);
        assert!(!settings.group_digits);
        assert!(settings.value_tags);
        assert!(!settings.show_system_schemas);
        assert_eq!(settings.custom_theme, None);
        assert_eq!(settings.sql_limit, 1_000);
        assert_eq!(settings.sql_timeout_secs, Some(30));
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
    fn the_defaults_are_written_as_the_canonical_text() {
        assert_eq!(
            Settings::default().to_toml(),
            "\
# written by tabletist, safe to edit by hand
[data]
page_size    = 300
timestamps   = \"second\"  # second | full
group_digits = false
value_tags   = true

[sidebar]
show_system_schemas = false

[editor]
sql_limit        = 1000
sql_timeout_secs = 30  # 0 waits forever
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
            text.contains("timestamps   = \"full\"  # second | full\n"),
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
        let settings = Settings {
            page_size: 500,
            timestamps: Timestamps::Full,
            group_digits: true,
            value_tags: false,
            show_system_schemas: true,
            custom_theme: Some("My \"Nord\".json".into()),
            sql_limit: 100,
            sql_timeout_secs: None,
        };
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
        let loaded = Settings::from_toml(&Settings::default().to_toml());
        assert_eq!(
            loaded.lines,
            vec![
                (Key::PageSize, 3),
                (Key::Timestamps, 4),
                (Key::GroupDigits, 5),
                (Key::ValueTags, 6),
                (Key::ShowSystemSchemas, 9),
                (Key::SqlLimit, 12),
                (Key::SqlTimeoutSecs, 13),
            ]
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
        assert!(loaded.settings.group_digits);
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
                invalid: vec![3],
                lines: vec![(Key::PageSize, 2)],
                live: false,
                saved: None,
            }
        );
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
        assert!(loaded.settings.group_digits);
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
        assert!(loaded.settings.group_digits);
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
        assert!(loaded.settings.group_digits);
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
            group_digits: true,
            sql_timeout_secs: None,
            ..Settings::default()
        };
        settings.save(&dirs.settings_file()).unwrap();
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
            (OptionId::GroupDigits, OptionValue::GroupDigits(true)),
            (OptionId::ValueTags, OptionValue::ValueTags(false)),
        ] {
            assert_eq!(value.option(), option);
            assert_ne!(settings.value(option), value);
            value.set(&mut settings);
            assert_eq!(settings.value(option), value);
        }
        assert_eq!(settings.page_size, 500);
        assert_eq!(settings.timestamps, Timestamps::Full);
        assert!(settings.group_digits);
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
                Key::ValueTags
            ]
        );
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
        assert_eq!(step(300, true), OptionValue::PageSize(500));
        assert_eq!(step(300, false), OptionValue::PageSize(100));
        assert_eq!(step(100, false), OptionValue::PageSize(100));
        assert_eq!(step(5_000, true), OptionValue::PageSize(5_000));
        // A size set by hand is between two choices: it steps to the next.
        assert_eq!(step(250, true), OptionValue::PageSize(300));
        assert_eq!(step(250, false), OptionValue::PageSize(100));
        assert_eq!(step(9_000, true), OptionValue::PageSize(9_000));
        assert_eq!(step(9_000, false), OptionValue::PageSize(5_000));
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
            OptionValue::GroupDigits(true)
        );
        assert_eq!(
            settings.stepped(OptionId::GroupDigits, true),
            OptionValue::GroupDigits(false)
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
            Some(OptionValue::GroupDigits(true))
        );
        assert_eq!(
            settings.flipped(OptionId::Timestamps),
            Some(OptionValue::Timestamps(Timestamps::Full))
        );
        assert_eq!(settings.flipped(OptionId::PageSize), None);
        // And back, from the other value of each.
        let settings = Settings {
            value_tags: false,
            group_digits: true,
            timestamps: Timestamps::Full,
            ..Default::default()
        };
        assert_eq!(
            settings.flipped(OptionId::ValueTags),
            Some(OptionValue::ValueTags(true))
        );
        assert_eq!(
            settings.flipped(OptionId::GroupDigits),
            Some(OptionValue::GroupDigits(false))
        );
        assert_eq!(
            settings.flipped(OptionId::Timestamps),
            Some(OptionValue::Timestamps(Timestamps::Second))
        );
        assert_eq!(settings.flipped(OptionId::PageSize), None);
    }
}
