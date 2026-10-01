//! User settings, stored as `settings.json` in the config directory.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Everything the user can set. New fields need a default so older files
/// keep loading; unknown fields (from newer versions) are ignored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// File format version, for future migrations.
    pub version: u32,
    /// Rows fetched per page in the data grid.
    pub page_size: u32,
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
    pub const CURRENT_VERSION: u32 = 1;
    pub const DEFAULT_PAGE_SIZE: u32 = 300;
    pub const MIN_PAGE_SIZE: u32 = 10;
    pub const MAX_PAGE_SIZE: u32 = 10_000;
    /// The row limits the SQL editor's Limit menu offers.
    pub const SQL_LIMITS: [u32; 3] = [100, 1_000, 10_000];
    /// The timeouts (seconds) the SQL editor's Timeout menu offers; `None`
    /// is no timeout.
    pub const SQL_TIMEOUTS: [Option<u32>; 5] = [Some(10), Some(30), Some(60), Some(300), None];

    /// The SQL editor's timeout as a duration; `None` waits forever.
    pub fn sql_timeout(&self) -> Option<std::time::Duration> {
        self.sql_timeout_secs
            .map(|secs| std::time::Duration::from_secs(u64::from(secs)))
    }

    pub fn load(path: &Path) -> Self {
        let mut settings: Settings = crate::util::load_json(path);
        settings.page_size = settings
            .page_size
            .clamp(Self::MIN_PAGE_SIZE, Self::MAX_PAGE_SIZE);
        settings.sql_limit = settings.sql_limit.clamp(1, Self::MAX_PAGE_SIZE);
        settings.version = Self::CURRENT_VERSION;
        settings
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::util::save_json(path, self)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            page_size: Self::DEFAULT_PAGE_SIZE,
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
        assert_eq!(settings.version, Settings::CURRENT_VERSION);
        assert_eq!(settings.page_size, 300);
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
