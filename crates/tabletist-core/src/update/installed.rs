//! How this copy was installed, and so who updates it: itself, or the
//! tool that installed it, with the command that does it.

use std::path::Path;

use fastframe_update::{Kind, PackageManager, Unsupported};

/// Who updates this copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Installed {
    /// It replaces itself: a copy from the download page.
    Itself(Kind),
    /// Something else does, or nothing can: the pane checks and says so.
    Elsewhere(Managed),
}

/// What updates a copy that does not update itself, as the card says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Managed {
    /// The line under the card's title. English: the caller translates.
    pub said: &'static str,
    /// What to run, where it is known.
    pub command: Option<&'static str>,
}

impl Installed {
    /// What detecting the installation came to. `found` says whether a
    /// program is on the `PATH`: an AUR helper's update covers the AUR's
    /// packages, which pacman's does not. `exe` is the running program,
    /// with its links followed: Homebrew keeps a formula's in its `Cellar`,
    /// and a formula and a cask take different commands.
    pub fn of(
        detected: Result<Kind, &Unsupported>,
        found: impl Fn(&str) -> bool,
        exe: Option<&Path>,
    ) -> Self {
        let reason = match detected {
            Ok(kind) => return Self::Itself(kind),
            Err(reason) => reason,
        };
        let managed = |said, command| Self::Elsewhere(Managed { said, command });
        let package = "Your package manager updates Tabletist";
        match reason {
            Unsupported::Homebrew => managed(
                "Installed with Homebrew, so Homebrew updates it",
                Some(if exe.is_some_and(in_cellar) {
                    "brew upgrade tabletist"
                } else {
                    "brew upgrade --cask tabletist"
                }),
            ),
            Unsupported::Cargo => managed(
                "Installed with cargo, so cargo updates it",
                Some("cargo install tabletist --locked"),
            ),
            Unsupported::SystemPackage(PackageManager::Pacman) => {
                let command = ["yay", "paru"]
                    .into_iter()
                    .find(|helper| found(helper))
                    .map_or("sudo pacman -Syu", |helper| match helper {
                        "paru" => "paru -Syu",
                        _ => "yay -Syu",
                    });
                managed(package, Some(command))
            }
            Unsupported::SystemPackage(PackageManager::Apt) => {
                managed(package, Some("sudo apt update && sudo apt upgrade"))
            }
            Unsupported::SystemPackage(PackageManager::Dnf) => {
                managed(package, Some("sudo dnf upgrade"))
            }
            Unsupported::Flatpak => managed(package, Some("flatpak update")),
            Unsupported::Snap => managed(package, Some("sudo snap refresh")),
            Unsupported::Nix | Unsupported::SystemDirectory => managed(package, None),
            Unsupported::MoveToApplications => managed(
                "Move Tabletist to Applications, then open it from there to update it",
                None,
            ),
            Unsupported::NotPortable
            | Unsupported::NotWritable
            | Unsupported::ForeignBundle
            | Unsupported::Platform
            | Unsupported::Unavailable(_) => managed("Tabletist can't replace itself here", None),
            // A reason of a later fastframe-update's.
            _ => managed("Tabletist can't replace itself here", None),
        }
    }

    /// Asks fastframe-update how the running copy was installed: it reads
    /// the disk and may ask the package manager, so off the interface
    /// thread, once per launch.
    pub fn detect() -> Self {
        let detected = super::updater().installation();
        let exe = std::env::current_exe().and_then(std::fs::canonicalize).ok();
        Self::of(
            detected.as_ref().map(|installation| installation.kind),
            on_path,
            exe.as_deref(),
        )
    }

    /// The command to copy, where something else updates this copy.
    pub fn command(&self) -> Option<&'static str> {
        match self {
            Self::Itself(_) => None,
            Self::Elsewhere(managed) => managed.command,
        }
    }

    /// The kind, where it updates itself.
    pub fn kind(&self) -> Option<Kind> {
        match self {
            Self::Itself(kind) => Some(*kind),
            Self::Elsewhere(_) => None,
        }
    }
}

/// Whether `exe` is in Homebrew's `Cellar`, where a formula is installed:
/// `/opt/homebrew/Cellar/tabletist/0.5.0/bin/tabletist`. A cask's app is in
/// Applications.
fn in_cellar(exe: &Path) -> bool {
    exe.components()
        .any(|part| part.as_os_str().eq_ignore_ascii_case("Cellar"))
}

/// Whether a program of this name is in a folder of the `PATH`.
fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|folders| {
        std::env::split_paths(&folders).any(|folder| folder.join(name).is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> bool {
        false
    }

    fn command(
        detected: Result<Kind, &Unsupported>,
        found: impl Fn(&str) -> bool,
    ) -> Option<&'static str> {
        Installed::of(detected, found, None).command()
    }

    #[test]
    fn homebrew_and_cargo_say_their_own_command() {
        let homebrew = Installed::of(Err(&Unsupported::Homebrew), none, None);
        assert_eq!(
            homebrew,
            Installed::Elsewhere(Managed {
                said: "Installed with Homebrew, so Homebrew updates it",
                command: Some("brew upgrade --cask tabletist"),
            })
        );
        assert_eq!(
            command(Err(&Unsupported::Cargo), none),
            Some("cargo install tabletist --locked")
        );
    }

    #[test]
    fn a_homebrew_formula_upgrades_without_cask() {
        let upgrade = |exe: &str| {
            Installed::of(Err(&Unsupported::Homebrew), none, Some(Path::new(exe))).command()
        };
        // The formula's program, on Apple silicon, on Intel and on Linux.
        for formula in [
            "/opt/homebrew/Cellar/tabletist/0.5.0/bin/tabletist",
            "/usr/local/Cellar/tabletist/0.5.0/bin/tabletist",
            "/home/linuxbrew/.linuxbrew/Cellar/tabletist/0.5.0/bin/tabletist",
        ] {
            assert_eq!(
                upgrade(formula),
                Some("brew upgrade tabletist"),
                "{formula}"
            );
        }
        // The cask's app, and a folder that only names a cellar.
        for cask in [
            "/Applications/Tabletist.app/Contents/MacOS/tabletist",
            "/opt/homebrew/Caskroom/tabletist/0.5.0/Tabletist.app/Contents/MacOS/tabletist",
            "/Users/ada/WineCellar/Tabletist.app/Contents/MacOS/tabletist",
        ] {
            assert_eq!(
                upgrade(cask),
                Some("brew upgrade --cask tabletist"),
                "{cask}"
            );
        }
    }

    #[test]
    fn a_package_manager_updates_it_with_its_command_where_known() {
        let pacman = Unsupported::SystemPackage(PackageManager::Pacman);
        assert_eq!(command(Err(&pacman), none), Some("sudo pacman -Syu"));
        // An AUR helper's update covers the AUR's packages too.
        assert_eq!(
            command(Err(&pacman), |name| name == "yay"),
            Some("yay -Syu")
        );
        assert_eq!(
            command(Err(&pacman), |name| name == "paru"),
            Some("paru -Syu")
        );
        let apt = Unsupported::SystemPackage(PackageManager::Apt);
        assert_eq!(
            command(Err(&apt), none),
            Some("sudo apt update && sudo apt upgrade")
        );
        for unknown in [Unsupported::Nix, Unsupported::SystemDirectory] {
            let installed = Installed::of(Err(&unknown), none, None);
            assert_eq!(
                installed,
                Installed::Elsewhere(Managed {
                    said: "Your package manager updates Tabletist",
                    command: None,
                }),
                "{unknown:?}"
            );
        }
    }

    #[test]
    fn a_copy_it_cannot_write_to_or_does_not_know_cannot_replace_itself() {
        for reason in [
            Unsupported::NotWritable,
            Unsupported::NotPortable,
            Unsupported::Unavailable("no".into()),
        ] {
            assert_eq!(
                Installed::of(Err(&reason), none, None),
                Installed::Elsewhere(Managed {
                    said: "Tabletist can't replace itself here",
                    command: None,
                }),
                "{reason:?}"
            );
        }
    }

    #[test]
    fn a_copy_from_the_download_page_updates_itself() {
        for kind in [Kind::Portable, Kind::MacBundle, Kind::AppImage] {
            let installed = Installed::of(Ok(kind), none, None);
            assert_eq!(installed, Installed::Itself(kind));
            assert_eq!(installed.command(), None);
            assert_eq!(installed.kind(), Some(kind));
        }
    }

    /// The detection itself, on a file of the shape `cargo install`
    /// leaves: `~/.cargo/bin/tabletist`.
    #[cfg(unix)]
    #[test]
    fn a_binary_cargo_installed_is_cargos_to_update() {
        let home = tempfile::tempdir().unwrap();
        let bin = home.path().join(".cargo/bin");
        std::fs::create_dir_all(&bin).unwrap();
        let executable = bin.join("tabletist");
        std::fs::write(&executable, b"").unwrap();
        let detected = super::super::updater().installation_at(&executable);
        assert_eq!(detected, Err(Unsupported::Cargo));
    }
}
