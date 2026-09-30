//! The desktop's monospace font on Linux, as fontconfig resolves it
//! (JetBrainsMono Nerd Font on Omarchy; `omarchy font set` changes it).

#[cfg(target_os = "linux")]
use std::{
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

/// How long startup waits for `fc-match` before using the bundled monospace.
#[cfg(target_os = "linux")]
const FC_MATCH_LIMIT: Duration = Duration::from_millis(500);

/// Reads `fc-match -f '%{file}\n%{index}'`: an absolute path and a face index.
/// Linux only: the path is POSIX, which Windows would not call absolute.
#[cfg(target_os = "linux")]
pub fn parse_fc_match(output: &str) -> Option<(PathBuf, u32)> {
    let mut lines = output.lines();
    let path = PathBuf::from(lines.next()?.trim());
    // Variable fonts carry a named-instance number in the upper 16 bits.
    let index = lines.next()?.trim().parse::<u32>().ok()? & 0xFFFF;
    path.is_absolute().then_some((path, index))
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
    let outlined = [b"glyf", b"CFF ", b"CFF2"]
        .into_iter()
        .any(|tag| face.table_data(skrifa::Tag::new(tag)).is_some());
    if !outlined {
        return None;
    }
    let mut data = egui::FontData::from_owned(bytes);
    data.index = index;
    Some(data)
}

/// The desktop's monospace font for a fontconfig `pattern` (`monospace`,
/// `monospace:bold`), or `None` (with one logged warning) when fontconfig is
/// missing or names a file egui could not read.
pub fn monospace(pattern: &str) -> Option<egui::FontData> {
    #[cfg(target_os = "linux")]
    {
        let output = run_bounded(
            Command::new("fc-match").args(["-f", "%{file}\n%{index}", pattern]),
            FC_MATCH_LIMIT,
        );
        let output = output?;
        let text = String::from_utf8_lossy(&output.stdout);
        let Some((path, index)) = parse_fc_match(&text) else {
            log::warn!("fc-match gave no monospace font");
            return None;
        };
        let font = load(&path, index);
        if font.is_none() {
            log::warn!(
                "could not read the monospace font {}, or it has no outlines",
                path.display()
            );
        }
        font
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
