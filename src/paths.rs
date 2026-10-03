//! Where Tabletist keeps its files.

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

/// The directories Tabletist writes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppDirs {
    /// Settings, saved connections, trusted SSH hosts, themes.
    pub config: PathBuf,
    /// Logs.
    pub state: PathBuf,
}

impl AppDirs {
    /// The platform's conventional directories: `~/.config/tabletist` and
    /// `~/.local/state/tabletist` on Linux.
    pub fn discover() -> Self {
        match ProjectDirs::from("dev", "tabletist", "Tabletist") {
            Some(project) => Self {
                config: project.config_dir().to_path_buf(),
                state: project
                    .state_dir()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| project.data_local_dir().to_path_buf()),
            },
            None => Self::at(
                &std::env::current_dir()
                    .unwrap_or_default()
                    .join("tabletist"),
            ),
        }
    }

    /// Directories under one root, for tests and demo mode.
    pub fn at(root: &Path) -> Self {
        Self {
            config: root.join("config"),
            state: root.join("state"),
        }
    }

    /// Creates both directories (private to the user on Unix). Call it before
    /// logging starts: fastframe-log creates a missing state directory with
    /// default permissions.
    pub fn ensure(&self) -> std::io::Result<()> {
        crate::util::create_private_dir(&self.config)?;
        crate::util::create_private_dir(&self.state)
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config.join("settings.toml")
    }

    /// Where versions before the TOML file kept the settings. Read at a
    /// start that finds no `settings.toml`, and never written.
    pub fn legacy_settings_file(&self) -> PathBuf {
        self.config.join("settings.json")
    }

    pub fn connections_file(&self) -> PathBuf {
        self.config.join("connections.json")
    }

    pub fn known_hosts_file(&self) -> PathBuf {
        self.config.join("known_hosts.json")
    }

    pub fn themes_dir(&self) -> PathBuf {
        self.config.join("themes")
    }

    pub fn log_file(&self) -> PathBuf {
        self.state.join("tabletist.log")
    }

    pub fn panic_log(&self) -> PathBuf {
        self.state.join("panic.log")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_live_in_their_directories() {
        let dirs = AppDirs::at(std::path::Path::new("/root"));
        assert_eq!(
            dirs.settings_file(),
            PathBuf::from("/root/config/settings.toml")
        );
        assert_eq!(
            dirs.legacy_settings_file(),
            PathBuf::from("/root/config/settings.json")
        );
        assert_eq!(
            dirs.connections_file(),
            PathBuf::from("/root/config/connections.json")
        );
        assert_eq!(
            dirs.known_hosts_file(),
            PathBuf::from("/root/config/known_hosts.json")
        );
        assert_eq!(dirs.themes_dir(), PathBuf::from("/root/config/themes"));
        assert_eq!(dirs.log_file(), PathBuf::from("/root/state/tabletist.log"));
        assert_eq!(dirs.panic_log(), PathBuf::from("/root/state/panic.log"));
    }

    #[test]
    fn ensure_creates_both_directories() {
        let root = tempfile::tempdir().unwrap();
        let dirs = AppDirs::at(&root.path().join("fresh"));
        dirs.ensure().unwrap();
        assert!(dirs.config.is_dir());
        assert!(dirs.state.is_dir());
    }

    #[test]
    fn discovered_directories_are_named_for_the_app() {
        let dirs = AppDirs::discover();
        let config = dirs.config.to_string_lossy().to_lowercase();
        assert!(config.contains("tabletist"), "{config}");
    }
}
