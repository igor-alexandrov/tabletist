//! User settings, stored as `settings.json` in the config directory.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// How much of a timestamp the grid shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

/// Everything the user can set. New fields need a default so older files
/// keep loading; unknown fields (from newer versions) are ignored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

impl Settings {
    pub const DEFAULT_PAGE_SIZE: u32 = 300;
    pub const MIN_PAGE_SIZE: u32 = 10;
    pub const MAX_PAGE_SIZE: u32 = 10_000;
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

    /// The settings with every number in its range.
    fn validated(mut self) -> Self {
        self.page_size = self
            .page_size
            .clamp(Self::MIN_PAGE_SIZE, Self::MAX_PAGE_SIZE);
        self.sql_limit = Self::valid_sql_limit(self.sql_limit);
        self.sql_timeout_secs = Self::valid_sql_timeout(self.sql_timeout_secs);
        self
    }

    pub fn load(path: &Path) -> Self {
        crate::util::load_json::<Settings>(path).validated()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::util::save_json(path, self)
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
    fn an_older_file_with_a_version_gets_the_new_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"version": 1, "page_size": 100}"#).unwrap();
        let settings = Settings::load(&path);
        assert_eq!(settings.page_size, 100);
        assert_eq!(settings.timestamps, Timestamps::Second);
        assert!(!settings.group_digits);
        assert!(settings.value_tags);
    }

    #[test]
    fn a_timestamps_choice_has_a_name() {
        for choice in [Timestamps::Second, Timestamps::Full] {
            assert_eq!(Timestamps::from_name(choice.name()), Some(choice));
        }
        assert_eq!(Timestamps::from_name("minute"), None);
    }

    #[test]
    fn settings_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let settings = Settings {
            page_size: 500,
            show_system_schemas: true,
            custom_theme: Some("Nord.json".into()),
            sql_limit: 100,
            sql_timeout_secs: None,
            ..Settings::default()
        };
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);
    }

    #[test]
    fn older_files_with_missing_and_unknown_fields_still_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"page_size": 100, "from_the_future": true}"#).unwrap();
        let settings = Settings::load(&path);
        assert_eq!(settings.page_size, 100);
        assert_eq!(settings.custom_theme, None);
    }

    #[test]
    fn older_files_get_the_sql_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"page_size": 100}"#).unwrap();
        let settings = Settings::load(&path);
        assert_eq!(settings.sql_limit, 1_000);
        assert_eq!(settings.sql_timeout_secs, Some(30));
    }

    #[test]
    fn the_sql_limit_is_clamped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"sql_limit": 0, "sql_timeout_secs": null}"#).unwrap();
        let settings = Settings::load(&path);
        assert_eq!(settings.sql_limit, 1);
        assert_eq!(settings.sql_timeout_secs, None);
    }

    #[test]
    fn the_sql_limit_has_its_own_ceiling() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"sql_limit": 999999999}"#).unwrap();
        assert_eq!(Settings::load(&path).sql_limit, Settings::MAX_SQL_LIMIT);
        assert_eq!(Settings::MAX_SQL_LIMIT, 10_000);
        assert!(
            Settings::SQL_LIMITS
                .iter()
                .all(|limit| (1..=Settings::MAX_SQL_LIMIT).contains(limit)),
            "every choice of the Limit menu loads as it was saved"
        );
    }

    #[test]
    fn a_timeout_of_no_seconds_loads_as_no_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"sql_timeout_secs": 0}"#).unwrap();
        let settings = Settings::load(&path);
        assert_eq!(settings.sql_timeout_secs, None);
        assert_eq!(settings.sql_timeout(), None);
        std::fs::write(&path, br#"{"sql_timeout_secs": 1}"#).unwrap();
        assert_eq!(Settings::load(&path).sql_timeout_secs, Some(1));
    }

    #[test]
    fn no_timeout_is_written_as_null_and_the_old_keys_stay() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let settings = Settings {
            sql_timeout_secs: None,
            ..Settings::default()
        };
        settings.save(&path).unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(json["sql_timeout_secs"].is_null());
        assert_eq!(json["sql_limit"], 1_000);
        assert_eq!(json["page_size"], 300);
        assert_eq!(json["show_system_schemas"], false);
        assert!(json["custom_theme"].is_null());
    }

    #[test]
    fn damaged_settings_are_kept_aside_and_defaults_used() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"page_size": "lots"}"#).unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        assert!(dir.path().join("settings.json.bad").exists());
    }

    #[test]
    fn page_size_is_clamped_to_a_sane_range() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"page_size": 0}"#).unwrap();
        assert_eq!(Settings::load(&path).page_size, Settings::MIN_PAGE_SIZE);
        std::fs::write(&path, br#"{"page_size": 999999999}"#).unwrap();
        assert_eq!(Settings::load(&path).page_size, Settings::MAX_PAGE_SIZE);
    }
}
