//! The desktop's monospace font on Linux, as fontconfig resolves it
//! (JetBrainsMono Nerd Font on Omarchy; `omarchy font set` changes it), or
//! the font the settings name by its fontconfig name. macOS reads the data
//! font it names into the same [`Read`] ([`super::installed_font`]).

use super::fonts::TerminalFaces;

#[cfg(target_os = "linux")]
use std::{
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

/// How long startup waits for `fc-match` before using the bundled monospace.
#[cfg(target_os = "linux")]
const FC_MATCH_LIMIT: Duration = Duration::from_millis(500);

/// What `fc-match` is asked to write: a file, a face index and the
/// families' names.
#[cfg(target_os = "linux")]
const FORMAT: &str = "%{file}\n%{index}\n%{family}";

/// Reads the first two lines of what `fc-match` is asked to write: an
/// absolute path and a face index.
/// Linux only: the path is POSIX, which Windows would not call absolute.
#[cfg(target_os = "linux")]
pub fn parse_fc_match(output: &str) -> Option<(PathBuf, u32)> {
    let mut lines = output.lines();
    let path = PathBuf::from(lines.next()?.trim());
    // Variable fonts carry a named-instance number in the upper 16 bits.
    let index = lines.next()?.trim().parse::<u32>().ok()? & 0xFFFF;
    path.is_absolute().then_some((path, index))
}

/// The family `fc-match`'s third line names, as it is asked to write it:
/// the first of its names
/// (`JetBrainsMono Nerd Font,JetBrainsMono NF` is the first).
pub fn parse_family(output: &str) -> Option<String> {
    let family = output.lines().nth(2)?.split(',').next()?.trim();
    (!family.is_empty()).then(|| family.to_owned())
}

/// Whether `family`, which fontconfig matched, is the font `name` asked
/// for: the same name, or one that begins with it, whatever the case and
/// the spaces (`JetBrains Mono` is `JetBrainsMono Nerd Font`). For a name it
/// does not know, fontconfig gives another font.
pub fn is_named(family: &str, name: &str) -> bool {
    let squeeze = |text: &str| {
        text.chars()
            .filter(|char| !char.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let name = squeeze(name);
    !name.is_empty() && squeeze(family).starts_with(&name)
}

/// `name` as a fontconfig pattern writes a family: with a backslash
/// before what the pattern reads otherwise (`-` before a size, `:` before
/// a property, `,` between families).
pub fn pattern(name: &str) -> String {
    let mut pattern = String::with_capacity(name.len());
    for char in name.trim().chars() {
        if matches!(char, '\\' | '-' | ':' | ',') {
            pattern.push('\\');
        }
        pattern.push(char);
    }
    pattern
}

/// What fontconfig gave the terminal look for the font the settings name,
/// or CoreText macOS's data for it.
#[derive(Clone, Debug, Default)]
pub struct Read {
    /// The font asked for, as the settings name it: `None` is the
    /// system's monospace.
    pub asked: Option<String>,
    /// The faces at 400, 500 and 700: `None` where fontconfig gave none
    /// that can be drawn (the bundled one is drawn then), and on macOS
    /// where nothing was read (a bundled face is drawn).
    pub faces: Option<TerminalFaces>,
    /// The family drawn, as fontconfig or CoreText names it.
    pub family: Option<String>,
    /// Whether the font asked for is not there: the system's monospace is
    /// drawn in its place, or on macOS the default data face.
    pub missing: bool,
}

impl Read {
    /// Whether the faces read are those of `font`, as the settings name it.
    pub fn has(&self, font: Option<&str>) -> bool {
        self.faces.is_some() && self.asked.as_deref() == font
    }
}

/// The font named `asked` (the system's monospace for `None`), at 400, 500
/// and 700, or the system's monospace in its place where fontconfig does
/// not have it. Each `fc-match` is bounded to half a second; a font found
/// takes three, a name not found four: two seconds at worst. It runs at the
/// start (as the fonts always did) and on the backend after.
pub fn read(asked: Option<&str>) -> Read {
    let named = asked
        .map(str::trim)
        .filter(|name| !name.is_empty() && !name.eq_ignore_ascii_case("monospace"));
    // The medium and bold of the font `base` gave as `regular`.
    let weights = |base: &str, regular: egui::FontData| {
        let medium = matched(&format!("{base}:medium")).map(|(face, _)| face);
        let bold = matched(&format!("{base}:bold")).map(|(face, _)| face);
        let regular = std::sync::Arc::new(regular);
        let medium = medium.map_or_else(|| regular.clone(), std::sync::Arc::new);
        let bold = bold.map_or_else(|| medium.clone(), std::sync::Arc::new);
        [regular, medium, bold]
    };
    let faces_of = |base: &str| {
        let (regular, family) = matched(base)?;
        Some((weights(base, regular), family))
    };
    let mut missing = false;
    let found = match named {
        Some(name) => {
            let base = pattern(name);
            // The weights are asked for only once the name is found.
            match matched(&base) {
                Some((regular, Some(family))) if is_named(&family, name) => {
                    Some((weights(&base, regular), Some(family)))
                }
                _ => {
                    missing = true;
                    log::warn!("the font {name:?} is not found: the system's monospace is drawn");
                    faces_of("monospace")
                }
            }
        }
        None => faces_of("monospace"),
    };
    let (faces, family) = found.map_or((None, None), |(faces, family)| (Some(faces), family));
    Read {
        asked: asked.map(str::to_owned),
        faces,
        family,
        missing,
    }
}

/// Runs `command` and returns its output, or `None` if it cannot start, fails,
/// or is still running after `limit` (then it is killed and reaped). Each
/// `None` logs exactly one warning. Stderr is discarded.
#[cfg(target_os = "linux")]
fn run_bounded(command: &mut Command, limit: Duration) -> Option<Output> {
    use std::io::Read as _;

    let spawned = command.stdout(Stdio::piped()).stderr(Stdio::null()).spawn();
    let Ok(mut child) = spawned else {
        log::warn!("fc-match is not available, so no desktop monospace font");
        return None;
    };
    let deadline = Instant::now() + limit;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(None) => {
                log::warn!("fc-match timed out");
                stop(&mut child);
                return None;
            }
            Err(error) => {
                log::warn!("could not wait for fc-match: {error}");
                stop(&mut child);
                return None;
            }
        }
    };
    let mut stdout = Vec::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_end(&mut stdout);
    }
    if !status.success() {
        log::warn!("fc-match failed, so no desktop monospace font");
        return None;
    }
    Some(Output {
        status,
        stdout,
        stderr: Vec::new(),
    })
}

/// Kills and reaps a child that is given up on, so it neither keeps running
/// nor stays a zombie.
#[cfg(target_os = "linux")]
fn stop(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// The face at `index` in `path`, if it exists, parses, and has outlines
/// (TrueType `glyf` or CFF). A bitmap-only face such as Terminus `.otb`
/// parses but would draw blank data.
#[cfg(target_os = "linux")]
pub fn load(path: &Path, index: u32) -> Option<egui::FontData> {
    let bytes = std::fs::read(path).ok()?;
    let face = skrifa::FontRef::from_index(&bytes, index).ok()?;
    if !super::installed_font::outlined(&face) {
        return None;
    }
    let mut data = egui::FontData::from_owned(bytes);
    data.index = index;
    Some(data)
}

/// The font fontconfig gives for `pattern` (`monospace`, `Fira
/// Code:bold`), with the family it names, or `None` (with one logged
/// warning) when fontconfig is missing or names a file egui could not read.
fn matched(pattern: &str) -> Option<(egui::FontData, Option<String>)> {
    #[cfg(target_os = "linux")]
    {
        let output = run_bounded(
            Command::new("fc-match").args(["-f", FORMAT, pattern]),
            FC_MATCH_LIMIT,
        );
        let output = output?;
        let text = String::from_utf8_lossy(&output.stdout);
        let Some((path, index)) = parse_fc_match(&text) else {
            log::warn!("fc-match gave no font for {pattern:?}");
            return None;
        };
        let font = load(&path, index);
        if font.is_none() {
            log::warn!(
                "could not read the monospace font {}, or it has no outlines",
                path.display()
            );
        }
        font.map(|font| (font, parse_family(&text)))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pattern;
        None
    }
}

#[cfg(all(test, target_os = "linux"))]
mod parse_tests {
    use super::*;

    #[test]
    fn fc_match_output_parses_path_and_index() {
        assert_eq!(
            parse_fc_match("/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf\n0"),
            Some((
                PathBuf::from("/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf"),
                0
            ))
        );
        assert_eq!(
            parse_fc_match("/usr/share/fonts/noto/NotoSansMono.ttc\n2\n"),
            Some((PathBuf::from("/usr/share/fonts/noto/NotoSansMono.ttc"), 2))
        );
        // A named-instance number in the upper 16 bits is not a face index.
        assert_eq!(
            parse_fc_match("/x.ttf\n65536"),
            Some((PathBuf::from("/x.ttf"), 0))
        );
        assert_eq!(parse_fc_match(""), None);
        assert_eq!(parse_fc_match("relative.ttf\n0"), None);
        assert_eq!(parse_fc_match("/x.ttf\nnot-a-number"), None);
    }

    #[test]
    fn the_family_is_the_first_name_on_the_third_line() {
        let output = "/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf\n0\n\
                      JetBrainsMono Nerd Font,JetBrainsMono NF\n";
        assert_eq!(
            parse_family(output).as_deref(),
            Some("JetBrainsMono Nerd Font")
        );
        assert_eq!(parse_family("/x.ttf\n0\n"), None);
        assert_eq!(parse_family("/x.ttf\n0"), None);
    }
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn a_family_is_the_font_named_when_it_begins_with_its_name() {
        assert!(is_named("Fira Code", "fira code"));
        assert!(is_named("JetBrainsMono Nerd Font", "JetBrains Mono"));
        assert!(is_named("Iosevka Term", "Iosevka"));
        // fontconfig's stand-in for a name it does not know.
        assert!(!is_named("DejaVu Sans Mono", "Fira Code"));
        assert!(!is_named("Fira Code", ""));
    }

    #[test]
    fn a_name_is_written_into_a_pattern_as_a_family() {
        assert_eq!(pattern("Fira Code"), "Fira Code");
        assert_eq!(pattern(" Source Code Pro "), "Source Code Pro");
        // A dash would start a size, a colon a property, a comma another
        // family.
        assert_eq!(pattern("Fira-Code:x,y\\z"), "Fira\\-Code\\:x\\,y\\\\z");
    }
}

#[cfg(all(test, target_os = "linux"))]
mod load_tests {
    use super::*;

    #[test]
    fn a_command_that_outlives_its_limit_is_killed() {
        let started = Instant::now();
        let output = run_bounded(Command::new("sleep").arg("5"), Duration::from_millis(100));
        assert!(output.is_none());
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn a_stopped_child_is_reaped() {
        let mut child = Command::new("sleep").arg("5").spawn().unwrap();
        stop(&mut child);
        assert!(child.try_wait().unwrap().is_some(), "killed and waited for");
    }

    #[test]
    fn a_quick_command_returns_its_output() {
        let output =
            run_bounded(Command::new("printf").arg("x"), Duration::from_secs(2)).expect("runs");
        assert_eq!(output.stdout, b"x");
        assert!(run_bounded(&mut Command::new("true"), Duration::from_secs(2)).is_some());
        assert!(run_bounded(&mut Command::new("false"), Duration::from_secs(2)).is_none());
    }

    #[test]
    fn an_unparsable_desktop_font_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.ttf");
        std::fs::write(&path, b"not a font").unwrap();
        assert!(load(&path, 0).is_none());
        assert!(load(&dir.path().join("missing.ttf"), 0).is_none());
    }

    #[test]
    fn a_real_font_loads_with_its_index() {
        // egui's bundled monospace (Hack) is a valid TTF.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hack.ttf");
        let hack = egui::FontDefinitions::default().font_data["Hack"]
            .font
            .to_vec();
        std::fs::write(&path, hack).unwrap();
        let data = load(&path, 0).expect("loads");
        assert_eq!(data.index, 0);
        assert!(load(&path, 5).is_none(), "no face 5 in a single-face file");
    }

    #[test]
    fn a_font_without_outlines_is_ignored() {
        // Hack with its glyf table renamed: it parses, like a bitmap-only
        // font (Terminus .otb), but has nothing to draw with.
        let mut bytes = egui::FontDefinitions::default().font_data["Hack"]
            .font
            .to_vec();
        let tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
        let record = (0..tables)
            .map(|table| 12 + table * 16)
            .find(|&record| &bytes[record..record + 4] == b"glyf")
            .expect("Hack has glyf outlines");
        bytes[record..record + 4].copy_from_slice(b"zzzz");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bitmap.otb");
        std::fs::write(&path, &bytes).unwrap();
        assert!(skrifa::FontRef::from_index(&bytes, 0).is_ok(), "it parses");
        assert!(load(&path, 0).is_none());
    }
}
