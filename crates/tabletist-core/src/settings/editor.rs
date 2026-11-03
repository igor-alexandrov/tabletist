//! The SQL editor's settings: the `[editor.run]`, `[editor.typing]`,
//! `[editor.format]` and `[editor.history]` tables of `settings.toml`.

/// The mode a new SQL tab opens in: read-only whatever the connection, or
/// the connection's own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NewTab {
    #[default]
    ReadOnly,
    Connection,
}

impl NewTab {
    /// As the file says it.
    pub fn name(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::Connection => "connection",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::ReadOnly, Self::Connection]
            .into_iter()
            .find(|choice| choice.name() == name)
    }
}

/// What the run key runs without a selection: the statement at the
/// cursor, or every statement of the tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunScope {
    #[default]
    Statement,
    All,
}

impl RunScope {
    /// As the file says it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Statement => "statement",
            Self::All => "all",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Statement, Self::All]
            .into_iter()
            .find(|choice| choice.name() == name)
    }
}

/// The editor's keys: the platform's text editing keys, or vim's modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorKeys {
    Standard,
    Vim,
}

impl EditorKeys {
    /// As the file says it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Vim => "vim",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Standard, Self::Vim]
            .into_iter()
            .find(|choice| choice.name() == name)
    }

    /// The look's own, as its board draws it: vim in the terminal look.
    pub fn of_look(terminal: bool) -> Self {
        if terminal { Self::Vim } else { Self::Standard }
    }
}

/// What an indent is: two spaces, four, or a tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Indent {
    #[default]
    Two,
    Four,
    Tab,
}

impl Indent {
    /// In the order the window draws them.
    pub const ALL: [Indent; 3] = [Self::Two, Self::Four, Self::Tab];

    /// As TOML writes it: a number of spaces, or `"tab"`.
    pub fn written(self) -> &'static str {
        match self {
            Self::Two => "2",
            Self::Four => "4",
            Self::Tab => "\"tab\"",
        }
    }
}

/// When the completion list opens: as you type, on the Complete key
/// alone, or never.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Autocomplete {
    #[default]
    Typing,
    Manual,
    Off,
}

impl Autocomplete {
    /// In the order the window draws them.
    pub const ALL: [Autocomplete; 3] = [Self::Typing, Self::Manual, Self::Off];

    /// As the file says it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Typing => "typing",
            Self::Manual => "manual",
            Self::Off => "off",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|choice| choice.name() == name)
    }

    /// Whether typing opens a list.
    pub fn as_you_type(self) -> bool {
        match self {
            Self::Typing => true,
            Self::Manual | Self::Off => false,
        }
    }

    /// Whether the Complete key opens a list.
    pub fn on_key(self) -> bool {
        match self {
            Self::Typing | Self::Manual => true,
            Self::Off => false,
        }
    }
}

/// How the gutter numbers the lines: each its own number, how far it is
/// from the cursor's line, or not at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineNumbers {
    Absolute,
    Relative,
    Off,
}

impl LineNumbers {
    /// In the order the terminal look draws them.
    pub const TERMINAL: [LineNumbers; 3] = [Self::Relative, Self::Absolute, Self::Off];

    /// As the file says it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Absolute => "absolute",
            Self::Relative => "relative",
            Self::Off => "off",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::TERMINAL
            .into_iter()
            .find(|choice| choice.name() == name)
    }

    /// The look's own, as its board draws it: relative in the terminal
    /// look.
    pub fn of_look(terminal: bool) -> Self {
        if terminal {
            Self::Relative
        } else {
            Self::Absolute
        }
    }

    /// Whether the gutter shows numbers.
    pub fn shown(self) -> bool {
        match self {
            Self::Absolute | Self::Relative => true,
            Self::Off => false,
        }
    }
}

/// The case Format writes keywords in: upper, lower, or as typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeywordCase {
    Upper,
    Lower,
    Keep,
}

impl KeywordCase {
    /// In the order the window draws them.
    pub const ALL: [KeywordCase; 3] = [Self::Upper, Self::Lower, Self::Keep];

    /// As the file says it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Upper => "upper",
            Self::Lower => "lower",
            Self::Keep => "keep",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|choice| choice.name() == name)
    }

    /// The look's own, as its board draws it: lower case in the terminal
    /// look.
    pub fn of_look(terminal: bool) -> Self {
        if terminal { Self::Lower } else { Self::Upper }
    }
}

/// Where Format puts the comma between two items of a list: at the end of
/// the first one's line, or at the start of the next one's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Commas {
    #[default]
    Trailing,
    Leading,
}

impl Commas {
    /// As the file says it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Trailing => "trailing",
            Self::Leading => "leading",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Trailing, Self::Leading]
            .into_iter()
            .find(|choice| choice.name() == name)
    }
}

/// The most seconds a timeout can be: an hour.
pub const MAX_TIMEOUT_SECS: u32 = 60 * 60;

/// A timeout as the file says it: `"30s"`, `"1m"`, or `"off"` for none. A
/// whole number of minutes is said in minutes.
pub fn timeout_name(secs: Option<u32>) -> String {
    match secs {
        None => "off".to_owned(),
        Some(secs) if secs >= 60 && secs.is_multiple_of(60) => format!("{}m", secs / 60),
        Some(secs) => format!("{secs}s"),
    }
}

/// The timeout a value of the file names: `"<n>s"` or `"<n>m"`, from a
/// second to an hour, or `"off"`. Anything else names none: `None` then,
/// and `Some(None)` for off.
pub fn timeout_from_name(name: &str) -> Option<Option<u32>> {
    if name == "off" {
        return Some(None);
    }
    let (number, scale) = match (name.strip_suffix('s'), name.strip_suffix('m')) {
        (Some(number), _) => (number, 1),
        (None, Some(number)) => (number, 60),
        (None, None) => return None,
    };
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let secs = number.parse::<u32>().ok()?.checked_mul(scale)?;
    (1..=MAX_TIMEOUT_SECS).contains(&secs).then_some(Some(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_choice_has_a_name_it_is_read_back_from() {
        for choice in [NewTab::ReadOnly, NewTab::Connection] {
            assert_eq!(NewTab::from_name(choice.name()), Some(choice));
        }
        for choice in [RunScope::Statement, RunScope::All] {
            assert_eq!(RunScope::from_name(choice.name()), Some(choice));
        }
        for choice in [EditorKeys::Standard, EditorKeys::Vim] {
            assert_eq!(EditorKeys::from_name(choice.name()), Some(choice));
        }
        for choice in Autocomplete::ALL {
            assert_eq!(Autocomplete::from_name(choice.name()), Some(choice));
        }
        for choice in LineNumbers::TERMINAL {
            assert_eq!(LineNumbers::from_name(choice.name()), Some(choice));
        }
        for choice in KeywordCase::ALL {
            assert_eq!(KeywordCase::from_name(choice.name()), Some(choice));
        }
        for choice in [Commas::Trailing, Commas::Leading] {
            assert_eq!(Commas::from_name(choice.name()), Some(choice));
        }
        assert_eq!(NewTab::from_name("readonly"), None);
        assert_eq!(LineNumbers::from_name("on"), None);
    }

    #[test]
    fn the_looks_defaults_are_as_their_boards_draw_them() {
        assert_eq!(EditorKeys::of_look(false), EditorKeys::Standard);
        assert_eq!(EditorKeys::of_look(true), EditorKeys::Vim);
        assert_eq!(LineNumbers::of_look(false), LineNumbers::Absolute);
        assert_eq!(LineNumbers::of_look(true), LineNumbers::Relative);
        assert_eq!(KeywordCase::of_look(false), KeywordCase::Upper);
        assert_eq!(KeywordCase::of_look(true), KeywordCase::Lower);
    }

    #[test]
    fn a_timeout_is_seconds_or_minutes_up_to_an_hour_or_off() {
        for (name, secs) in [
            ("off", None),
            ("5s", Some(5)),
            ("30s", Some(30)),
            ("90s", Some(90)),
            ("1m", Some(60)),
            ("5m", Some(300)),
            ("60m", Some(3_600)),
        ] {
            assert_eq!(timeout_from_name(name), Some(secs), "{name}");
        }
        for wrong in [
            "", "s", "0s", "61m", "3601s", "1h", "-5s", "+5s", "5 s", "5", "5é", "é", "30с",
        ] {
            assert_eq!(timeout_from_name(wrong), None, "{wrong}");
        }
        assert_eq!(timeout_name(None), "off");
        assert_eq!(timeout_name(Some(30)), "30s");
        assert_eq!(timeout_name(Some(60)), "1m");
        assert_eq!(timeout_name(Some(90)), "90s");
        assert_eq!(timeout_name(Some(300)), "5m");
        for secs in [5, 15, 30, 60, 90, 300, 3_600] {
            assert_eq!(
                timeout_from_name(&timeout_name(Some(secs))),
                Some(Some(secs))
            );
        }
    }
}
