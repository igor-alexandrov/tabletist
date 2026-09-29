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
    #[serde(default)]
    pub password: PasswordMode,
    /// How the SSH password or key passphrase is kept, when there is one.
    #[serde(default)]
    pub ssh_secret: PasswordMode,
    pub spec: ConnectSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedConnections {
    pub version: u32,
    pub connections: Vec<SavedConnection>,
}

impl Default for SavedConnections {
    fn default() -> Self {
        Self {
            version: 1,
            connections: Vec::new(),
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

    pub fn remove(&mut self, id: &ConnectionId) -> Option<SavedConnection> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tabletist_db::ConnectSpec;

    fn saved(name: &str, file: &str) -> SavedConnection {
        SavedConnection {
            id: ConnectionId::new(),
            name: name.into(),
            color: ColorTag::None,
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
}
