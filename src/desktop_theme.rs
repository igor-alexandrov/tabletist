//! What the terminal look reads of Omarchy's themes beside the live palette
//! fastframe follows: the name of the theme of the moment, and a theme
//! pinned by name or by path. Files and Omarchy's own tool: the backend
//! reads them, never a frame.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::theme::{Palette, PinFrom};

/// The most a palette file, a template, a theme's name or a theme's
/// colours are read to: far more than any of them is.
const LIMIT: u64 = 64 * 1024;

/// Tabletist's own Omarchy template: what maps a theme's colours onto the
/// palette when the user has no template of their own.
const TEMPLATE: &str = include_str!("../contrib/omarchy/tabletist.json.tpl");

/// Where Omarchy keeps what is read, for one user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Places {
    /// The user's home: what `~/` in a pin's path stands for.
    pub home: PathBuf,
    /// The themes Omarchy ships, `$OMARCHY_PATH/themes`.
    pub shipped: PathBuf,
}

impl Places {
    /// The places of the user whose home is `home`, with Omarchy where
    /// `$OMARCHY_PATH` says, else where it installs itself.
    pub fn of(home: &Path) -> Self {
        let omarchy = std::env::var_os("OMARCHY_PATH")
            .filter(|path| !path.is_empty())
            .map_or_else(|| home.join(".local/share/omarchy"), PathBuf::from);
        Self {
            home: home.to_owned(),
            shipped: omarchy.join("themes"),
        }
    }

    /// The file Omarchy writes the theme of the moment's name in.
    fn current_name(&self) -> PathBuf {
        self.home.join(".local/state/omarchy/current/theme.name")
    }

    /// The user's own themes, which come before the shipped ones.
    fn own(&self) -> PathBuf {
        self.home.join(".config/omarchy/themes")
    }

    /// The user's own Tabletist template, which Omarchy renders instead
    /// of the app's.
    fn template(&self) -> PathBuf {
        self.home.join(".config/omarchy/themed/tabletist.json.tpl")
    }
}

/// A pinned theme that was read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pinned {
    pub palette: Palette,
    /// From an Omarchy theme's colours, rather than a palette file.
    pub omarchy: bool,
}

/// What one read found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Read {
    /// The name of Omarchy's theme of the moment as Omarchy shows it
    /// (`Tokyo Night`), where it says one.
    pub current: Option<String>,
    /// The pin that was read, as the settings write it, and its theme:
    /// `None` when it is no Omarchy theme or path that could be read.
    pub pin: Option<(String, Option<Pinned>)>,
}

/// Reads the theme of the moment's name and, where the settings pin one
/// by name or path, that theme. `colors` gives a theme's colours as
/// `omarchy-theme-color --all` prints them, from its `colors.toml`.
pub fn read(
    places: &Places,
    pin: Option<&str>,
    colors: impl Fn(&Path) -> io::Result<String>,
) -> Read {
    let current = read_small(&places.current_name())
        .ok()
        .and_then(|name| shown_name(name.trim()));
    let pin = pin.map(|pin| {
        let theme = pinned(places, pin, &colors).unwrap_or_else(|error| {
            log::warn!("unable to read the pinned theme: {error}");
            None
        });
        (pin.to_owned(), theme)
    });
    Read { current, pin }
}

/// The theme `pin` names: an Omarchy theme by name (the user's, then the
/// shipped one), a palette file or a theme's folder by path. `None` for a
/// name no Omarchy theme has, or a pin of the themes folder.
fn pinned(
    places: &Places,
    pin: &str,
    colors: &impl Fn(&Path) -> io::Result<String>,
) -> io::Result<Option<Pinned>> {
    match PinFrom::of(pin) {
        PinFrom::File(_) => Ok(None),
        PinFrom::Name(name) => {
            let Some(folder) = folder_name(name) else {
                return Ok(None);
            };
            // `omarchy-theme-set` lays the user's folder over the shipped
            // one, file by file: the colours are the first folder's that
            // has them.
            for themes in [places.own(), places.shipped.clone()] {
                let theme = themes.join(&folder);
                if theme.join("colors.toml").is_file() {
                    return omarchy_theme(places, &theme, colors).map(Some);
                }
            }
            Ok(None)
        }
        PinFrom::Path(path) => {
            let path = match path.strip_prefix("~/") {
                Some(rest) => places.home.join(rest),
                None => PathBuf::from(path),
            };
            if path.is_dir() {
                return omarchy_theme(places, &path, colors).map(Some);
            }
            let palette =
                fastframe_theme::parse_palette(&read_small(&path)?).map_err(io::Error::other)?;
            Ok(Some(Pinned {
                palette,
                omarchy: false,
            }))
        }
    }
}

/// The palette of the Omarchy theme in `folder`: its colours through the
/// user's template, else the app's.
fn omarchy_theme(
    places: &Places,
    folder: &Path,
    colors: &impl Fn(&Path) -> io::Result<String>,
) -> io::Result<Pinned> {
    let template = places.template();
    let template = if template.is_file() {
        read_small(&template)?
    } else {
        TEMPLATE.to_owned()
    };
    let colors = colors(&folder.join("colors.toml"))?;
    let rendered = fastframe_theme::omarchy::render_seed::<Palette>(&template, &colors)
        .map_err(io::Error::other)?;
    let palette = fastframe_theme::parse_palette(&rendered).map_err(io::Error::other)?;
    Ok(Pinned {
        palette,
        omarchy: true,
    })
}

/// The folder a theme called `name` is in, as `omarchy-theme-set` finds
/// it (`Tokyo Night` is `tokyo-night`). `None` for a name that would lead
/// out of the themes' folder.
fn folder_name(name: &str) -> Option<String> {
    // `tr '[:upper:]' '[:lower:]'`, as `omarchy-theme-set` lowers it:
    // ASCII letters only.
    let folder = name.to_ascii_lowercase().replace(' ', "-");
    let plain = !folder.is_empty() && !folder.starts_with('.') && !folder.contains(['/', '\\']);
    plain.then_some(folder)
}

/// A theme's folder name as Omarchy shows it: `tokyo-night` is
/// `Tokyo Night`.
fn shown_name(folder: &str) -> Option<String> {
    let words: Vec<String> = folder
        .split('-')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut letters = word.chars();
            letters.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(letters).collect()
            })
        })
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

/// A theme's colours from Omarchy's own tool, as fastframe reads the
/// current theme's.
pub fn omarchy_colors(file: &Path) -> io::Result<String> {
    let output = Command::new("omarchy-theme-color")
        .arg("--file")
        .arg(file)
        .arg("--all")
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() || output.stdout.len() as u64 > LIMIT {
        return Err(io::Error::other("the theme's colours could not be read"));
    }
    String::from_utf8(output.stdout).map_err(io::Error::other)
}

/// A small file's text: an error for one larger than [`LIMIT`], and for
/// what is no file (a pipe or a device would hold the read).
fn read_small(path: &Path) -> io::Result<String> {
    use std::io::Read as _;
    if !std::fs::metadata(path)?.is_file() {
        return Err(io::Error::other("not a file"));
    }
    let mut text = String::new();
    std::fs::File::open(path)?
        .take(LIMIT + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > LIMIT {
        return Err(io::Error::other("the file is too large"));
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A home with Omarchy's folders, and the places in it.
    fn home() -> (tempfile::TempDir, Places) {
        let root = tempfile::tempdir().unwrap();
        let places = Places {
            home: root.path().to_owned(),
            shipped: root.path().join("omarchy/themes"),
        };
        (root, places)
    }

    /// Writes `path` under the home, with its folders.
    fn put(places: &Places, path: &str, text: &str) {
        let path = places.home.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// Colours as `omarchy-theme-color --all` prints them, with `accent`
    /// for the accent.
    fn colors_with(accent: &str) -> String {
        format!(
            "mode\tdark\nbackground\t#1a1b26\ndark_background\t#16161e\n\
             lighter_background\t#292e42\nforeground\t#c0caf5\nmuted\t#565f89\n\
             accent\t{accent}\nselection\t#283457\nred\t#f7768e\n\
             green\t#9ece6a\nyellow\t#e0af68\ncyan\t#7dcfff\n\
             orange\t#ff9e64\nmagenta\t#bb9af7\nblue\t#7aa2f7\n\
             bright_red\t#f7768e\nbright_green\t#9ece6a\n"
        )
    }

    /// Reads a theme's colours from the accent its `colors.toml` holds:
    /// what the tool would print, without the tool.
    fn colors(file: &Path) -> io::Result<String> {
        let accent = std::fs::read_to_string(file)?;
        Ok(colors_with(accent.trim()))
    }

    fn accent(read: &Read) -> Option<egui::Color32> {
        read.pin.as_ref()?.1.map(|pinned| pinned.palette.accent)
    }

    #[test]
    fn the_theme_of_the_moment_is_named_as_omarchy_shows_it() {
        let (_root, places) = home();
        assert_eq!(read(&places, None, colors).current, None);
        put(
            &places,
            ".local/state/omarchy/current/theme.name",
            "tokyo-night\n",
        );
        assert_eq!(
            read(&places, None, colors).current.as_deref(),
            Some("Tokyo Night")
        );
        put(&places, ".local/state/omarchy/current/theme.name", "\n");
        assert_eq!(read(&places, None, colors).current, None);
    }

    #[test]
    fn a_name_is_the_users_omarchy_theme_before_the_shipped_one() {
        let (_root, places) = home();
        put(&places, "omarchy/themes/tokyo-night/colors.toml", "#111111");
        let read_pin = |pin| accent(&read(&places, Some(pin), colors));
        let shipped = egui::Color32::from_rgb(0x11, 0x11, 0x11);
        assert_eq!(read_pin("Tokyo Night"), Some(shipped));
        assert_eq!(read_pin("tokyo-night"), Some(shipped));
        put(
            &places,
            ".config/omarchy/themes/tokyo-night/colors.toml",
            "#222222",
        );
        assert_eq!(
            read_pin("Tokyo Night"),
            Some(egui::Color32::from_rgb(0x22, 0x22, 0x22))
        );
        // A folder of the user's without colours takes the shipped ones.
        put(&places, ".config/omarchy/themes/sunset/neovim.lua", "");
        put(&places, "omarchy/themes/sunset/colors.toml", "#333333");
        assert_eq!(
            read_pin("Sunset"),
            Some(egui::Color32::from_rgb(0x33, 0x33, 0x33))
        );
        let pinned = read(&places, Some("Tokyo Night"), colors).pin.unwrap();
        assert_eq!(pinned.0, "Tokyo Night");
        assert!(pinned.1.unwrap().omarchy);
        // No such theme: nothing, for the app to look in the themes folder.
        assert_eq!(
            read(&places, Some("Nord"), colors).pin,
            Some(("Nord".into(), None))
        );
    }

    #[test]
    fn the_users_template_maps_a_pinned_theme_as_omarchy_would() {
        let (_root, places) = home();
        put(&places, "omarchy/themes/sunset/colors.toml", "#111111");
        let own = TEMPLATE.replace("\"accent\": \"{{ accent }}\"", "\"accent\": \"{{ red }}\"");
        assert_ne!(own, TEMPLATE);
        put(&places, ".config/omarchy/themed/tabletist.json.tpl", &own);
        assert_eq!(
            accent(&read(&places, Some("Sunset"), colors)),
            Some(egui::Color32::from_rgb(0xf7, 0x76, 0x8e))
        );
    }

    #[test]
    fn a_path_is_a_themes_folder_or_a_palette_file() {
        let (_root, places) = home();
        put(&places, "mine/sunset/colors.toml", "#333333");
        let folder = places.home.join("mine/sunset");
        let pin = folder.to_str().unwrap();
        assert_eq!(
            accent(&read(&places, Some(pin), colors)),
            Some(egui::Color32::from_rgb(0x33, 0x33, 0x33))
        );
        put(
            &places,
            "mine/Plain.json",
            "{\"base\": \"light\", \"colors\": {\"accent\": \"#444444\"}}",
        );
        let pinned = read(&places, Some("~/mine/Plain.json"), colors)
            .pin
            .unwrap();
        assert_eq!(
            pinned.1.unwrap().palette.accent,
            egui::Color32::from_rgb(0x44, 0x44, 0x44)
        );
        assert!(!pinned.1.unwrap().omarchy);
        // What cannot be read pins nothing.
        for pin in ["~/mine/Missing.json", "~/mine/sunset/colors.toml"] {
            assert_eq!(accent(&read(&places, Some(pin), colors)), None, "{pin}");
        }
        // What is no file is not read: a device would never end.
        #[cfg(unix)]
        assert_eq!(accent(&read(&places, Some("/dev/zero"), colors)), None);
        let broken = |_: &Path| -> io::Result<String> { Err(io::Error::other("no tool")) };
        assert_eq!(accent(&read(&places, Some(pin), broken)), None);
    }

    #[test]
    fn a_name_never_leads_out_of_the_themes_folders() {
        let (_root, places) = home();
        put(&places, "omarchy/escape/colors.toml", "#555555");
        for pin in ["../escape", "..", ".hidden", "a\\b"] {
            assert_eq!(accent(&read(&places, Some(pin), colors)), None, "{pin}");
        }
        // A palette file of the themes folder is the catalog's, not read here.
        assert_eq!(
            read(&places, Some("Nord.json"), colors).pin,
            Some(("Nord.json".into(), None))
        );
    }

    #[test]
    fn a_folder_name_is_shown_in_words() {
        assert_eq!(shown_name("tokyo-night").as_deref(), Some("Tokyo Night"));
        assert_eq!(shown_name("ristretto").as_deref(), Some("Ristretto"));
        assert_eq!(
            shown_name("catppuccin-latte").as_deref(),
            Some("Catppuccin Latte")
        );
        assert_eq!(shown_name("").as_deref(), None);
        assert_eq!(folder_name("Tokyo Night").as_deref(), Some("tokyo-night"));
        // As Omarchy lowers a name: its ASCII letters.
        assert_eq!(folder_name("Ünique Dark").as_deref(), Some("Ünique-dark"));
    }
}
