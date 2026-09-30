//! Saved connections, stored as `connections.json`. Specs only: passwords
//! live in the OS keyring (batch 4), never in this file.

use std::path::Path;

use egui::Color32;
use serde::{Deserialize, Serialize};
use tabletist_db::ConnectSpec;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub String);

impl ConnectionId {
    #[allow(clippy::new_without_default)] // a new id is never a "default"
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// A colour to tell connections apart at a glance (red for production).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorTag {
    #[default]
    None,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Gray,
}

impl ColorTag {
    pub const ALL: [ColorTag; 8] = [
        Self::None,
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Blue,
        Self::Purple,
        Self::Gray,
    ];

    pub fn color(self) -> Option<Color32> {
        match self {
            Self::None => None,
            Self::Red => Some(Color32::from_rgb(0xe5, 0x48, 0x4d)),
            Self::Orange => Some(Color32::from_rgb(0xf0, 0x8c, 0x2e)),
            Self::Yellow => Some(Color32::from_rgb(0xe6, 0xc2, 0x29)),
            Self::Green => Some(Color32::from_rgb(0x3f, 0xb9, 0x50)),
            Self::Blue => Some(Color32::from_rgb(0x3b, 0x82, 0xf6)),
            Self::Purple => Some(Color32::from_rgb(0x9b, 0x5d, 0xe5)),
            Self::Gray => Some(Color32::from_rgb(0x8b, 0x93, 0x9e)),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "No color",
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
            Self::Gray => "Gray",
        }
    }
}

/// What a connection is: the environment badge on the picker and the top
/// bar, and the colour of its connection bar. Chosen on its own; older
/// connections without one take it from their colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Dev,
    Staging,
    Production,
    Local,
    Test,
    None,
}

impl Environment {
    /// In the order the dialog offers them.
    pub const ALL: [Environment; 6] = [
        Self::None,
        Self::Local,
        Self::Dev,
        Self::Test,
        Self::Staging,
        Self::Production,
    ];

    /// The name the dialog lists it by.
    pub fn name(self) -> &'static str {
        match self {
            Self::Dev => "Development",
            Self::Staging => "Staging",
            Self::Production => "Production",
            Self::Local => "Local",
            Self::Test => "Test",
            Self::None => "None",
        }
    }

    /// The colour a connection takes when it is given this environment.
    pub fn color(self) -> ColorTag {
        match self {
            Self::Dev => ColorTag::Green,
            Self::Staging => ColorTag::Orange,
            Self::Production => ColorTag::Red,
            Self::Local => ColorTag::Purple,
            Self::Test => ColorTag::Blue,
            Self::None => ColorTag::None,
        }
    }

    /// The badge text: `short` is the terminal look's (PROD).
    pub fn label(self, short: bool) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Staging => "staging",
            Self::Production if short => "prod",
            Self::Production => "production",
            Self::Local => "local",
            Self::Test => "test",
            Self::None => "none",
        }
    }
}

impl ColorTag {
    /// The environment the colour stood for before connections had one of
    /// their own (red for production): what an older connection shows.
    pub fn environment(self) -> Environment {
        match self {
            Self::Green => Environment::Dev,
            Self::Orange | Self::Yellow => Environment::Staging,
            Self::Red => Environment::Production,
            Self::Purple => Environment::Local,
            Self::Blue => Environment::Test,
            Self::Gray | Self::None => Environment::None,
        }
    }
}

/// Where a connection's password comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PasswordMode {
    /// No password (SQLite, trust or peer authentication).
    #[default]
    None,
    /// Saved in the OS keyring.
    Keyring,
    /// Asked for on every connect.
    Ask,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedConnection {
    pub id: ConnectionId,
    pub name: String,
    #[serde(default)]
    pub color: ColorTag,
    /// `None` in files written before connections had an environment: then
    /// the colour says it (see [`SavedConnection::environment`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<Environment>,
    #[serde(default)]
    pub password: PasswordMode,
    /// How the SSH password or key passphrase is kept, when there is one.
    #[serde(default)]
    pub ssh_secret: PasswordMode,
    pub spec: ConnectSpec,
}

impl SavedConnection {
    /// The connection's environment: its own, or the one its colour stood
    /// for.
    pub fn environment(&self) -> Environment {
        self.environment.unwrap_or(self.color.environment())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedConnections {
    pub version: u32,
    pub connections: Vec<SavedConnection>,
    /// When each connection last connected, in seconds since the Unix
    /// epoch, by id. Kept beside the connections so they stay as saved.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub last_used: std::collections::BTreeMap<String, u64>,
}

impl Default for SavedConnections {
    fn default() -> Self {
        Self {
            version: 1,
            connections: Vec::new(),
            last_used: std::collections::BTreeMap::new(),
        }
    }
}

impl SavedConnections {
    pub fn load(path: &Path) -> Self {
        crate::util::load_json(path)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::util::save_json(path, self)
    }

    pub fn get(&self, id: &ConnectionId) -> Option<&SavedConnection> {
        self.connections
            .iter()
            .find(|connection| &connection.id == id)
    }

    /// Replaces the connection with the same id, or appends it.
    pub fn upsert(&mut self, connection: SavedConnection) {
        match self
            .connections
            .iter_mut()
            .find(|existing| existing.id == connection.id)
        {
            Some(existing) => *existing = connection,
            None => self.connections.push(connection),
        }
    }

    /// When `id` last connected, in seconds since the Unix epoch.
    pub fn last_used(&self, id: &ConnectionId) -> Option<u64> {
        self.last_used.get(&id.0).copied()
    }

    /// Notes that `id` connected at `now` (seconds since the Unix epoch).
    pub fn mark_used(&mut self, id: &ConnectionId, now: u64) {
        self.last_used.insert(id.0.clone(), now);
    }

    pub fn remove(&mut self, id: &ConnectionId) -> Option<SavedConnection> {
        self.last_used.remove(&id.0);
        let index = self
            .connections
            .iter()
            .position(|connection| &connection.id == id)?;
        Some(self.connections.remove(index))
    }

    /// Copies a connection under a new id, right after the original.
    pub fn duplicate(&mut self, id: &ConnectionId) -> Option<ConnectionId> {
        let index = self
            .connections
            .iter()
            .position(|connection| &connection.id == id)?;
        let mut copy = self.connections[index].clone();
        copy.id = ConnectionId::new();
        copy.name = format!("{} copy", copy.name);
        let new_id = copy.id.clone();
        self.connections.insert(index + 1, copy);
        Some(new_id)
    }

    /// Connections whose name or summary contains `text`, ignoring case.
    pub fn search(&self, text: &str) -> Vec<&SavedConnection> {
        let needle = text.trim().to_lowercase();
        self.connections
            .iter()
            .filter(|connection| {
                needle.is_empty()
                    || connection.name.to_lowercase().contains(&needle)
                    || connection.spec.summary().to_lowercase().contains(&needle)
            })
            .collect()
    }
}

/// When something last happened, as the picker says it: `2 min ago`,
/// `yesterday`, `Sep 12`, or `never`. `now` and `then` are seconds since the
/// Unix epoch; days count in UTC.
pub fn when(then: Option<u64>, now: u64) -> String {
    let Some(then) = then else {
        return "never".into();
    };
    let ago = now.saturating_sub(then);
    let days = now / 86_400 - then / 86_400;
    match ago {
        0..60 => "just now".into(),
        60..3_600 => format!("{} min ago", ago / 60),
        _ if days == 0 => format!("{} h ago", ago / 3_600),
        _ if days == 1 => "yesterday".into(),
        _ => {
            const MONTHS: [&str; 12] = [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ];
            let (year, month, day) = civil(then / 86_400);
            let (this_year, _, _) = civil(now / 86_400);
            let date = format!("{} {day}", MONTHS[month as usize - 1]);
            if year == this_year {
                date
            } else {
                format!("{date}, {year}")
            }
        }
    }
}

/// The calendar date of a day number since 1970-01-01 (Howard Hinnant's
/// `civil_from_days`).
fn civil(days: u64) -> (i64, u32, u32) {
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tabletist_db::ConnectSpec;

    #[test]
    fn last_use_reads_as_the_picker_says_it() {
        // 2026-09-29 12:00 UTC.
        let now = 1_790_683_200;
        assert_eq!(when(None, now), "never");
        assert_eq!(when(Some(now - 120), now), "2 min ago");
        assert_eq!(when(Some(now - 86_400), now), "yesterday");
        // 2026-09-12.
        assert_eq!(when(Some(now - 17 * 86_400), now), "Sep 12");
        assert_eq!(when(Some(now - 400 * 86_400), now), "Aug 25, 2025");
    }

    #[test]
    fn old_files_without_last_use_still_load() {
        let text = r#"{"version": 1, "connections": []}"#;
        let store: SavedConnections = serde_json::from_str(text).unwrap();
        assert!(store.last_used.is_empty());
        let saved = serde_json::to_string(&store).unwrap();
        assert!(!saved.contains("last_used"), "{saved}");
    }

    fn saved(name: &str, file: &str) -> SavedConnection {
        SavedConnection {
            id: ConnectionId::new(),
            name: name.into(),
            color: ColorTag::None,
            environment: None,
            password: PasswordMode::None,
            ssh_secret: PasswordMode::None,
            spec: ConnectSpec::sqlite(file),
        }
    }

    #[test]
    fn connections_round_trip_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("connections.json");
        let mut store = SavedConnections::default();
        store.upsert(SavedConnection {
            color: ColorTag::Red,
            ..saved("Prod", "/prod.db")
        });
        store.save(&path).unwrap();
        let loaded = SavedConnections::load(&path);
        assert_eq!(loaded.connections, store.connections);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"red\""));
    }

    #[test]
    fn a_damaged_store_is_kept_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("connections.json");
        std::fs::write(&path, "[not json").unwrap();
        assert!(SavedConnections::load(&path).connections.is_empty());
        assert!(dir.path().join("connections.json.bad").exists());
    }

    #[test]
    fn upsert_replaces_by_id_and_keeps_order() {
        let mut store = SavedConnections::default();
        let first = saved("A", "/a.db");
        let second = saved("B", "/b.db");
        store.upsert(first.clone());
        store.upsert(second.clone());
        store.upsert(SavedConnection {
            name: "A2".into(),
            ..first.clone()
        });
        let names: Vec<&str> = store.connections.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["A2", "B"]);
    }

    #[test]
    fn remove_and_duplicate_work_by_id() {
        let mut store = SavedConnections::default();
        let original = saved("Local", "/l.db");
        store.upsert(original.clone());
        let copy = store.duplicate(&original.id).unwrap();
        assert_ne!(copy, original.id);
        assert_eq!(store.get(&copy).unwrap().name, "Local copy");
        assert_eq!(store.get(&copy).unwrap().spec, original.spec);
        assert!(store.remove(&original.id).is_some());
        assert!(store.get(&original.id).is_none());
        assert!(store.duplicate(&original.id).is_none());
    }

    #[test]
    fn search_matches_name_and_summary_case_insensitively() {
        let mut store = SavedConnections::default();
        store.upsert(saved("Production", "/srv/prod.db"));
        store.upsert(saved("Staging", "/srv/stage.db"));
        let names = |text: &str| -> Vec<String> {
            store.search(text).iter().map(|c| c.name.clone()).collect()
        };
        assert_eq!(names("PROD"), vec!["Production"]);
        assert_eq!(names("stage.db"), vec!["Staging"]);
        assert_eq!(names("  "), vec!["Production", "Staging"]);
        assert!(names("mysql").is_empty());
    }

    #[test]
    fn every_tag_but_none_has_a_colour() {
        for tag in ColorTag::ALL {
            assert_eq!(tag.color().is_none(), tag == ColorTag::None, "{tag:?}");
        }
    }

    #[test]
    fn password_modes_default_to_none_and_round_trip() {
        let old: SavedConnection = serde_json::from_str(
            r#"{"id": "x", "name": "Old", "spec": {"driver": "sqlite", "sqlite_path": "/a.db"}}"#,
        )
        .unwrap();
        assert_eq!(old.password, PasswordMode::None);
        let json = serde_json::to_string(&SavedConnection {
            password: PasswordMode::Ask,
            ..old
        })
        .unwrap();
        assert!(json.contains("\"ask\""), "{json}");
    }

    #[test]
    fn older_connections_take_their_environment_from_the_colour() {
        let old: SavedConnection = serde_json::from_str(
            r#"{"id": "x", "name": "Old", "color": "purple",
                "spec": {"driver": "sqlite", "sqlite_path": "/a.db"}}"#,
        )
        .unwrap();
        assert_eq!(old.environment, None);
        assert_eq!(old.environment(), Environment::Local);
        // Saved again unchanged, it stays as it was.
        assert!(!serde_json::to_string(&old).unwrap().contains("environment"));
    }

    #[test]
    fn a_chosen_environment_wins_over_the_colour() {
        let production = SavedConnection {
            color: ColorTag::Purple,
            environment: Some(Environment::Production),
            ..saved("Shop", "/shop.db")
        };
        assert_eq!(production.environment(), Environment::Production);
        let json = serde_json::to_string(&production).unwrap();
        assert!(json.contains("\"environment\":\"production\""), "{json}");
        let back: SavedConnection = serde_json::from_str(&json).unwrap();
        assert_eq!(back.environment(), Environment::Production);
    }

    #[test]
    fn each_environment_has_its_own_colour() {
        for environment in Environment::ALL {
            assert_eq!(
                environment.color().environment(),
                environment,
                "{environment:?}"
            );
        }
    }
}
