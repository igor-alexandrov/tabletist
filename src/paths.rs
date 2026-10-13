//! Where Tabletist keeps its files.

use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};

/// The directories Tabletist writes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppDirs {
    /// Settings, saved connections, trusted SSH hosts, themes.
    pub config: PathBuf,
    /// Logs, and the label columns chosen for foreign keys.
    pub state: PathBuf,
    /// The user's home directory, where there is one: a path under it is
    /// shown with `~`. Found with the others, at the start: without `HOME`
    /// Unix asks the passwd database, which can be a lookup over the
    /// network, and Windows makes several system calls. None of that
    /// belongs in a frame.
    pub home: Option<PathBuf>,
}

impl AppDirs {
    /// The platform's conventional directories: `~/.config/tabletist` and
    /// `~/.local/state/tabletist` on Linux.
    pub fn discover() -> Self {
        let home = BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
        match ProjectDirs::from("dev", "tabletist", "Tabletist") {
            Some(project) => Self {
                config: project.config_dir().to_path_buf(),
                state: project
                    .state_dir()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| project.data_local_dir().to_path_buf()),
                home,
            },
            None => Self::at(
                &std::env::current_dir()
                    .unwrap_or_default()
                    .join("tabletist"),
            ),
        }
    }

    /// Directories under one root, for tests and demo mode. No home: a
    /// path is shown as it is.
    pub fn at(root: &Path) -> Self {
        Self {
            config: root.join("config"),
            state: root.join("state"),
            home: None,
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
        self.config.join("connections.toml")
    }

    /// Where versions before the TOML file kept the connections, and when
    /// each last connected. Read at a start that finds no
    /// `connections.toml`, and never written.
    pub fn legacy_connections_file(&self) -> PathBuf {
        self.config.join("connections.json")
    }

    pub fn known_hosts_file(&self) -> PathBuf {
        self.config.join("known_hosts.json")
    }

    pub fn themes_dir(&self) -> PathBuf {
        self.config.join("themes")
    }

    /// The columns chosen to label the rows foreign keys point at. Of
    /// this computer, and no setting: it is kept with the state.
    pub fn labels_file(&self) -> PathBuf {
        self.state.join("labels.json")
    }

    /// When each saved connection last connected. Of this computer, and
    /// written at every connect: kept with the state, so that the file of
    /// connections is written only when one of them changes.
    pub fn last_used_file(&self) -> PathBuf {
        self.state.join("last_used.json")
    }

    /// The note that the connections are kept in `connections.toml`: made
    /// at the first start that finds that file. Once it is there, what an
    /// older version kept in `connections.json` is never read again,
    /// whatever becomes of the new file.
    pub fn connections_moved_file(&self) -> PathBuf {
        self.state.join("connections.moved")
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
            PathBuf::from("/root/config/connections.toml")
        );
        assert_eq!(
            dirs.legacy_connections_file(),
            PathBuf::from("/root/config/connections.json")
        );
        assert_eq!(
            dirs.last_used_file(),
            PathBuf::from("/root/state/last_used.json")
        );
        assert_eq!(
            dirs.connections_moved_file(),
            PathBuf::from("/root/state/connections.moved")
        );
        assert_eq!(
            dirs.known_hosts_file(),
            PathBuf::from("/root/config/known_hosts.json")
        );
        assert_eq!(dirs.themes_dir(), PathBuf::from("/root/config/themes"));
        assert_eq!(dirs.labels_file(), PathBuf::from("/root/state/labels.json"));
        assert_eq!(dirs.log_file(), PathBuf::from("/root/state/tabletist.log"));
        assert_eq!(dirs.panic_log(), PathBuf::from("/root/state/panic.log"));
    }

    #[test]
    fn the_home_directory_is_found_with_the_others_and_tests_have_none() {
        assert_eq!(AppDirs::at(Path::new("/root")).home, None);
        let home = BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
        assert_eq!(AppDirs::discover().home, home);
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
