//! Saved connections, stored as `connections.json`. Specs only: passwords
//! live in the OS keyring, never in this file.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tabletist_db::ConnectSpec;

use crate::env::Environment;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub String);

impl ConnectionId {
    #[allow(clippy::new_without_default)] // a new id is never a "default"
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
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
    /// What the connection is. Its colour everywhere comes from this.
    pub environment: Environment,
    /// Read-only as the user set it; `None` until they do, so the
    /// environment's default applies (see [`SavedConnection::read_only`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    #[serde(default)]
    pub password: PasswordMode,
    /// How the SSH password or key passphrase is kept, when there is one.
    #[serde(default)]
    pub ssh_secret: PasswordMode,
    pub spec: ConnectSpec,
}

impl SavedConnection {
    /// Whether the connection is read-only: the user's choice, else its
    /// environment's default. A user who never chose follows the
    /// environment when it changes.
    pub fn read_only(&self) -> bool {
        self.read_only
            .unwrap_or(self.environment.read_only_by_default())
    }
}

/// The file's format. 2 dropped the colour a connection used to keep
/// beside its environment, and the `test` environment.
pub const VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize)]
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
            version: VERSION,
            connections: Vec::new(),
            last_used: std::collections::BTreeMap::new(),
        }
    }
}

/// The file as any version wrote it.
#[derive(Debug, Deserialize)]
#[serde(default)]
struct StoredConnections {
    /// Files from before the field existed are version 1.
    #[serde(default = "first_version")]
    version: u32,
    connections: Vec<StoredConnection>,
    last_used: std::collections::BTreeMap<String, u64>,
}

fn first_version() -> u32 {
    1
}

impl Default for StoredConnections {
    fn default() -> Self {
        Self {
            version: VERSION,
            connections: Vec::new(),
            last_used: std::collections::BTreeMap::new(),
        }
    }
}

/// A connection as any version wrote it.
#[derive(Debug, Deserialize)]
struct StoredConnection {
    id: ConnectionId,
    name: String,
    /// Version 1's colour, read only to find the environment.
    #[serde(default)]
    color: Option<LegacyColor>,
    #[serde(default)]
    environment: Option<StoredEnvironment>,
    #[serde(default)]
    read_only: Option<bool>,
    #[serde(default)]
    password: PasswordMode,
    #[serde(default)]
    ssh_secret: PasswordMode,
    spec: ConnectSpec,
}

/// The colours version 1 offered.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum LegacyColor {
    None,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Gray,
}

impl LegacyColor {
    fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Red => "red",
            Self::Orange => "orange",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Gray => "gray",
        }
    }

    /// The environment the colour stood for.
    fn environment(self) -> Environment {
        match self {
            Self::Red => Environment::Production,
            Self::Green | Self::Blue => Environment::Dev,
            Self::Orange | Self::Yellow => Environment::Staging,
            Self::Purple => Environment::Local,
            Self::Gray | Self::None => Environment::None,
        }
    }
}

/// The environments any version wrote.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum StoredEnvironment {
    Local,
    Dev,
    Staging,
    Production,
    None,
    /// Version 1's: local when the connection stays on this machine, else
    /// dev.
    Test,
}

impl StoredConnection {
    /// The connection as this version keeps it, and what its environment
    /// was worked out from when the file did not say it.
    fn upgrade(self) -> (SavedConnection, Option<String>) {
        let (environment, from) = match (self.environment, self.color) {
            (Some(StoredEnvironment::Local), _) => (Environment::Local, None),
            (Some(StoredEnvironment::Dev), _) => (Environment::Dev, None),
            (Some(StoredEnvironment::Staging), _) => (Environment::Staging, None),
            (Some(StoredEnvironment::Production), _) => (Environment::Production, None),
            (Some(StoredEnvironment::None), _) => (Environment::None, None),
            (Some(StoredEnvironment::Test), _) => {
                let environment = match Environment::for_spec(&self.spec) {
                    Environment::Local => Environment::Local,
                    Environment::Dev
                    | Environment::Staging
                    | Environment::Production
                    | Environment::None => Environment::Dev,
                };
                (environment, Some("environment test".to_owned()))
            }
            (None, color) => {
                let color = color.unwrap_or(LegacyColor::None);
                (
                    color.environment(),
                    Some(format!("colour {}", color.name())),
                )
            }
        };
        let note = from.map(|from| {
            format!(
                "connection {:?}: {from} is now environment {}",
                self.name,
                environment.label(crate::env::Platform::Native)
            )
        });
        let saved = SavedConnection {
            id: self.id,
            name: self.name,
            environment,
            read_only: self.read_only,
            password: self.password,
            ssh_secret: self.ssh_secret,
            spec: self.spec,
        };
        (saved, note)
    }
}

impl StoredConnections {
    /// The store as this version keeps it, and a line for each connection
    /// whose environment was worked out.
    fn upgrade(self) -> (SavedConnections, Vec<String>) {
        let mut notes = Vec::new();
        let connections = self
            .connections
            .into_iter()
            .map(|stored| {
                let (saved, note) = stored.upgrade();
                notes.extend(note);
                saved
            })
            .collect();
        let store = SavedConnections {
            version: VERSION,
            connections,
            last_used: self.last_used,
        };
        (store, notes)
    }
}

impl SavedConnections {
    /// Reads the store. An older file is upgraded in memory, each
    /// connection whose environment is worked out logged.
    pub fn load(path: &Path) -> Self {
        Self::load_upgrading(path).0
    }

    /// [`SavedConnections::load`], and whether the file was an older
    /// version: the caller saves it (on the backend) so it is upgraded once.
    pub fn load_upgrading(path: &Path) -> (Self, bool) {
        let stored: StoredConnections = crate::util::load_json(path);
        let old = stored.version < VERSION;
        let (store, notes) = stored.upgrade();
        for note in &notes {
            log::info!("{note}");
        }
        (store, old)
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
        let stored: StoredConnections = serde_json::from_str(text).unwrap();
        let (store, _) = stored.upgrade();
        assert!(store.last_used.is_empty());
        let saved = serde_json::to_string(&store).unwrap();
        assert!(!saved.contains("last_used"), "{saved}");
    }

    fn saved(name: &str, file: &str) -> SavedConnection {
        SavedConnection {
            id: ConnectionId::new(),
            name: name.into(),
            environment: Environment::None,
            read_only: None,
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
            environment: Environment::Production,
            ..saved("Prod", "/prod.db")
        });
        store.save(&path).unwrap();
        let loaded = SavedConnections::load(&path);
        assert_eq!(loaded.connections, store.connections);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"environment\": \"production\""), "{text}");
        assert!(text.contains("\"version\": 2"), "{text}");
        assert!(!text.contains("color"), "{text}");
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
    fn password_modes_default_to_none_and_round_trip() {
        let old: SavedConnection = serde_json::from_str(
            r#"{"id": "x", "name": "Old", "environment": "none",
                "spec": {"driver": "sqlite", "sqlite_path": "/a.db"}}"#,
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

    /// The version 1 fixture, upgraded as a store is loaded.
    fn fixture() -> (SavedConnections, Vec<String>) {
        let text = include_str!("../tests/fixtures/connections-v1.json");
        serde_json::from_str::<StoredConnections>(text)
            .unwrap()
            .upgrade()
    }

    #[test]
    fn version_1_colours_become_environments() {
        let (store, _) = fixture();
        let environment = |id: &str| {
            store
                .get(&ConnectionId(id.into()))
                .unwrap_or_else(|| panic!("{id}"))
                .environment
        };
        for (id, expected) in [
            ("red", Environment::Production),
            ("orange", Environment::Staging),
            ("yellow", Environment::Staging),
            ("green", Environment::Dev),
            ("blue", Environment::Dev),
            ("purple", Environment::Local),
            ("gray", Environment::None),
            ("none", Environment::None),
            ("uncoloured", Environment::None),
            // A chosen environment wins over the colour.
            ("chosen", Environment::Production),
            // Test: local on this machine, else dev.
            ("test-local", Environment::Local),
            ("test-socket", Environment::Local),
            ("test-remote", Environment::Dev),
            ("test-tunnel", Environment::Dev),
        ] {
            assert_eq!(environment(id), expected, "{id}");
        }
        assert_eq!(store.version, VERSION);
        assert_eq!(
            store.last_used(&ConnectionId("red".into())),
            Some(1_790_683_200)
        );
    }

    #[test]
    fn a_connection_that_became_production_is_read_only() {
        let (store, _) = fixture();
        let red = store.get(&ConnectionId("red".into())).unwrap();
        assert_eq!(red.read_only, None, "the default, not a choice");
        assert!(red.read_only());
        assert!(
            !store
                .get(&ConnectionId("green".into()))
                .unwrap()
                .read_only()
        );
    }

    #[test]
    fn each_worked_out_environment_is_logged_once() {
        let (_, notes) = fixture();
        // Every connection but the one with a chosen environment.
        assert_eq!(notes.len(), 13, "{notes:#?}");
        assert!(
            notes.contains(
                &"connection \"Bookshop production\": colour red is now environment production"
                    .to_owned()
            ),
            "{notes:#?}"
        );
        assert!(
            notes.contains(
                &"connection \"Bookshop CI\": environment test is now environment dev".to_owned()
            ),
            "{notes:#?}"
        );
    }

    #[test]
    fn an_upgraded_file_saved_again_has_nothing_left_to_work_out() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("connections.json");
        std::fs::write(&path, include_str!("../tests/fixtures/connections-v1.json")).unwrap();
        let (first, old) = SavedConnections::load_upgrading(&path);
        assert!(old);
        first.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"version\": 2"), "{text}");
        assert!(!text.contains("color"), "{text}");
        assert!(!text.contains("\"test\""), "{text}");
        let stored: StoredConnections = serde_json::from_str(&text).unwrap();
        let (again, notes) = stored.upgrade();
        assert!(notes.is_empty(), "{notes:#?}");
        assert_eq!(again, first);
        assert!(!SavedConnections::load_upgrading(&path).1);
    }

    #[test]
    fn a_missing_file_needs_no_upgrade() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("connections.json");
        let (store, old) = SavedConnections::load_upgrading(&path);
        assert!(store.connections.is_empty());
        assert!(!old);
    }

    #[test]
    fn a_read_only_choice_is_kept_and_the_default_follows_the_environment() {
        let mut connection = saved("Bookshop", "/bookshop.db");
        assert!(!connection.read_only());
        connection.environment = Environment::Production;
        assert!(connection.read_only(), "untouched: the default applies");
        connection.read_only = Some(false);
        assert!(!connection.read_only(), "the user's choice wins");
        connection.environment = Environment::Dev;
        connection.environment = Environment::Production;
        assert!(
            !connection.read_only(),
            "and survives changes of environment"
        );
        let json = serde_json::to_string(&connection).unwrap();
        assert!(json.contains("\"read_only\":false"), "{json}");
        let untouched = serde_json::to_string(&saved("Bookshop", "/bookshop.db")).unwrap();
        assert!(!untouched.contains("read_only"), "{untouched}");
    }
}
