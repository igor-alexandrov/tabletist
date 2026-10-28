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
    /// The user's runtime directory, where there is one (Linux's
    /// `/run/user/<id>`): what lives as long as the session, the agents'
    /// endpoint. Elsewhere, and in tests, that is in `state`.
    pub runtime: Option<PathBuf>,
    /// The administrator's policy (`managed.toml`), at one fixed path on
    /// each system and never moved by a variable: an agent may start
    /// Tabletist with variables of its choosing. Under the root in tests
    /// and the demo, so that neither reads the machine's. None where it is
    /// not read yet (Windows).
    pub managed: Option<PathBuf>,
}

impl AppDirs {
    /// The platform's conventional directories: `~/.config/tabletist` and
    /// `~/.local/state/tabletist` on Linux.
    pub fn discover() -> Self {
        let home = BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
        let runtime = runtime_dir();
        match ProjectDirs::from("dev", "tabletist", "Tabletist") {
            Some(project) => Self {
                config: project.config_dir().to_path_buf(),
                state: project
                    .state_dir()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| project.data_local_dir().to_path_buf()),
                home,
                runtime,
                managed: managed_file(),
            },
            None => Self {
                managed: managed_file(),
                ..Self::at(
                    &std::env::current_dir()
                        .unwrap_or_default()
                        .join("tabletist"),
                )
            },
        }
    }

    /// Directories under one root, for tests and demo mode. No home: a
    /// path is shown as it is.
    pub fn at(root: &Path) -> Self {
        Self {
            config: root.join("config"),
            state: root.join("state"),
            home: None,
            runtime: None,
            managed: Some(root.join("managed.toml")),
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

    /// What the checks for updates found: when the last one ran, and the
    /// newest release with its notes. Of this computer, and no setting.
    pub fn updates_file(&self) -> PathBuf {
        self.state.join("updates.json")
    }

    /// A release's notes, written for the pager to show.
    pub fn release_notes_file(&self, version: &str) -> PathBuf {
        self.state.join(format!("release-notes-{version}.md"))
    }

    /// How each connection lists its tables where its user chose: tree or
    /// flat, by its id. Of this computer, and no setting: the setting is
    /// the default for a connection with no choice.
    pub fn table_lists_file(&self) -> PathBuf {
        self.state.join("table_lists.json")
    }

    /// When each saved connection last connected. Of this computer, and
    /// written at every connect: kept with the state, so that the file of
    /// connections is written only when one of them changes.
    pub fn last_used_file(&self) -> PathBuf {
        self.state.join("last_used.json")
    }

    /// The connection tabs open at the last quit, and each connection's
    /// tables as its window last had them. Of this computer, and written
    /// at quit, when a connection's window closes or has another
    /// connection put in it, when a database switch drops its tables, and
    /// when a connection is deleted.
    pub fn session_file(&self) -> PathBuf {
        self.state.join("session.json")
    }

    /// What was opened lately on each saved connection, for the start
    /// screen. Of this computer, and written when something opens, when
    /// Clear is pressed, when an object listed no more is dropped, and when
    /// a connection is deleted.
    pub fn recents_file(&self) -> PathBuf {
        self.state.join("recents.json")
    }

    /// The note that the connections are kept in `connections.toml`: made
    /// at the first start that finds that file. Once it is there, what an
    /// older version kept in `connections.json` is never read again,
    /// whatever becomes of the new file.
    pub fn connections_moved_file(&self) -> PathBuf {
        self.state.join("connections.moved")
    }

    /// The log of every call agents made, kept for the days the settings
    /// say (`[mcp] log_days`).
    pub fn agents_log_file(&self) -> PathBuf {
        self.state.join("agents.sqlite3")
    }

    /// Every statement run on each saved connection, from every source,
    /// kept to the settings' `[editor.history] keep` per connection.
    /// Beside the Agents Log, which an agent's entry links to.
    pub fn history_file(&self) -> PathBuf {
        self.state.join("history.sqlite3")
    }

    /// Where `tabletist mcp` reaches the running app: the directory of its
    /// socket, `mcp.sock`, and of the key a relay sends first, `mcp.key`.
    /// Private to the user: made 0700 by the app before it listens.
    /// `tabletist` in the runtime directory where there is one. Else, on
    /// Linux, `~/.local/state/tabletist`, which no variable moves, and
    /// elsewhere the state directory. On macOS a socket's path may have 103 bytes, which
    /// a long home directory can pass: then `.tabletist` in the home
    /// directory, which is shorter by 45.
    ///
    /// The relay finds it from what an agent starts it with, which may be
    /// no more than `HOME`, `PATH` and a few others: so nothing here comes
    /// from `XDG_RUNTIME_DIR` or `TMPDIR`.
    pub fn agents_dir(&self) -> PathBuf {
        let dir = match (&self.runtime, &self.home) {
            (Some(runtime), _) => runtime.join("tabletist"),
            // Where the state directory is when `XDG_STATE_HOME` is not
            // set, whether it is set or not.
            #[cfg(target_os = "linux")]
            (None, Some(home)) => home.join(".local/state/tabletist"),
            _ => self.state.clone(),
        };
        #[cfg(target_os = "macos")]
        if let Some(home) = &self.home
            && dir.join(crate::endpoint::SOCKET).as_os_str().len()
                > crate::endpoint::MAX_SOCKET_PATH
        {
            return home.join(".tabletist");
        }
        dir
    }

    /// What agents may do, as the app last showed it: what a change made
    /// outside the app is held against (`agents_shown`).
    pub fn agents_shown_file(&self) -> PathBuf {
        self.state.join("agents_shown.json")
    }

    pub fn log_file(&self) -> PathBuf {
        self.state.join("tabletist.log")
    }

    pub fn panic_log(&self) -> PathBuf {
        self.state.join("panic.log")
    }
}

/// `/run/user/<id>`, where the system made it for this user: found by the
/// user's id, which is the owner of the process's own `/proc` entry.
#[cfg(target_os = "linux")]
fn runtime_dir() -> Option<PathBuf> {
    use std::os::unix::fs::MetadataExt;
    let user = std::fs::metadata("/proc/self").ok()?.uid();
    let dir = PathBuf::from(format!("/run/user/{user}"));
    let found = std::fs::metadata(&dir).ok()?;
    (found.is_dir() && found.uid() == user).then_some(dir)
}

/// None but on Linux.
#[cfg(not(target_os = "linux"))]
fn runtime_dir() -> Option<PathBuf> {
    None
}

/// Where an administrator's policy is kept on this system: a folder only
/// an administrator can write to, as each system has it. Windows asks the
/// system for its folder, which takes code of its own (run 1c of the
/// policy), so nothing is read there yet.
fn managed_file() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let file = Some(PathBuf::from(
        "/Library/Application Support/Tabletist/managed.toml",
    ));
    #[cfg(all(unix, not(target_os = "macos")))]
    let file = Some(PathBuf::from("/etc/tabletist/managed.toml"));
    #[cfg(not(unix))]
    let file = None;
    file
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
            dirs.session_file(),
            PathBuf::from("/root/state/session.json")
        );
        assert_eq!(
            dirs.known_hosts_file(),
            PathBuf::from("/root/config/known_hosts.json")
        );
        assert_eq!(dirs.themes_dir(), PathBuf::from("/root/config/themes"));
        assert_eq!(dirs.labels_file(), PathBuf::from("/root/state/labels.json"));
        assert_eq!(dirs.log_file(), PathBuf::from("/root/state/tabletist.log"));
        assert_eq!(dirs.panic_log(), PathBuf::from("/root/state/panic.log"));
        assert_eq!(
            dirs.agents_log_file(),
            PathBuf::from("/root/state/agents.sqlite3")
        );
        assert_eq!(
            dirs.agents_shown_file(),
            PathBuf::from("/root/state/agents_shown.json")
        );
        // Without a runtime directory, with the state.
        assert_eq!(dirs.agents_dir(), PathBuf::from("/root/state"));
        let dirs = AppDirs {
            runtime: Some(PathBuf::from("/run/user/1000")),
            ..dirs
        };
        assert_eq!(dirs.agents_dir(), PathBuf::from("/run/user/1000/tabletist"));
        // Without one, on Linux, where the state is without a variable.
        #[cfg(target_os = "linux")]
        assert_eq!(
            AppDirs {
                runtime: None,
                home: Some(PathBuf::from("/home/ada")),
                ..dirs
            }
            .agents_dir(),
            PathBuf::from("/home/ada/.local/state/tabletist")
        );
    }

    #[test]
    fn the_managed_policy_is_under_a_root_and_at_a_fixed_path_otherwise() {
        assert_eq!(
            AppDirs::at(Path::new("/root")).managed,
            Some(PathBuf::from("/root/managed.toml"))
        );
        #[cfg(target_os = "macos")]
        let fixed = Some(PathBuf::from(
            "/Library/Application Support/Tabletist/managed.toml",
        ));
        #[cfg(all(unix, not(target_os = "macos")))]
        let fixed = Some(PathBuf::from("/etc/tabletist/managed.toml"));
        #[cfg(not(unix))]
        let fixed = None;
        assert_eq!(AppDirs::discover().managed, fixed);
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

    /// The relay and the app find one directory, whatever the agent left
    /// out of the relay's environment.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_runtime_directory_is_the_users_own_and_no_variables() {
        use std::os::unix::fs::MetadataExt;
        if let Some(dir) = runtime_dir() {
            let user = std::fs::metadata("/proc/self").unwrap().uid();
            assert_eq!(dir, PathBuf::from(format!("/run/user/{user}")));
        }
        assert_eq!(AppDirs::discover().runtime, runtime_dir());
    }

    #[test]
    fn discovered_directories_are_named_for_the_app() {
        let dirs = AppDirs::discover();
        let config = dirs.config.to_string_lossy().to_lowercase();
        assert!(config.contains("tabletist"), "{config}");
    }
}
