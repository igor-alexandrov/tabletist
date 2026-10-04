//! The settings file as the Settings screen shows it: its own text, a line
//! of the file per line, with each part in its colour. This is no TOML
//! parser: it tells apart what the app writes and leaves the rest plain.

use std::ops::Range;
use std::path::Path;

use egui::Color32;

use crate::theme::Palette;

/// What a stretch of a line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    /// `# ...` to the end of the line.
    Comment,
    /// `[table]`.
    Table,
    /// A key and its `=`.
    Key,
    /// A quoted value.
    String,
    /// A number, `true` or `false`.
    Value,
    /// Anything else, as it is.
    Plain,
}

impl Part {
    /// The colour the part is written in.
    pub(super) fn color(self, palette: &Palette) -> Color32 {
        match self {
            Self::Comment => palette.dim,
            Self::Table => palette.magenta,
            Self::Key | Self::Plain => palette.text,
            Self::String => palette.success,
            Self::Value => palette.orange,
        }
    }
}

/// The parts of `line`, in order: every byte of it is in exactly one.
pub(super) fn spans(line: &str) -> Vec<(Range<usize>, Part)> {
    let mut out = Vec::new();
    if line.is_empty() {
        return out;
    }
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        return vec![(0..line.len(), Part::Comment)];
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() > 2 {
        return vec![(0..line.len(), Part::Table)];
    }
    // A key is bare letters, digits, `_` and `-` before the first `=`.
    let Some(equals) = line.find('=') else {
        return vec![(0..line.len(), Part::Plain)];
    };
    let key = line[..equals].trim();
    let bare = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    if key.is_empty() || !key.chars().all(bare) {
        return vec![(0..line.len(), Part::Plain)];
    }
    // The key takes the `=` and the blanks after it.
    let rest = &line[equals + 1..];
    let value_at = equals + 1 + (rest.len() - rest.trim_start_matches([' ', '\t']).len());
    out.push((0..value_at, Part::Key));
    let value = &line[value_at..];
    // Where the value ends: a string at its closing quote, anything else at
    // the first blank or `#`.
    let end = if let Some(body) = value.strip_prefix('"') {
        let mut escaped = false;
        let mut close = None;
        for (index, c) in body.char_indices() {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => {
                    close = Some(index);
                    break;
                }
                _ => {}
            }
        }
        match close {
            Some(close) => {
                out.push((value_at..value_at + close + 2, Part::String));
                value_at + close + 2
            }
            // Left open: plain to the end.
            None => {
                out.push((value_at..line.len(), Part::Plain));
                return out;
            }
        }
    } else {
        let len = value.find([' ', '\t', '#']).unwrap_or(value.len());
        if len > 0 {
            out.push((value_at..value_at + len, Part::Value));
        }
        value_at + len
    };
    // What follows the value: blanks, then a comment or something else.
    let tail = &line[end..];
    match tail.find('#') {
        Some(hash) => {
            if hash > 0 {
                out.push((end..end + hash, Part::Plain));
            }
            out.push((end + hash..line.len(), Part::Comment));
        }
        None if !tail.is_empty() => out.push((end..line.len(), Part::Plain)),
        None => {}
    }
    out
}

/// `path` as the screen writes it: the home directory as `~`.
pub(super) fn shown_path(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(line: &str) -> Vec<(&str, Part)> {
        spans(line)
            .into_iter()
            .map(|(range, part)| (&line[range], part))
            .collect()
    }

    #[test]
    fn a_line_of_the_file_is_told_apart_into_its_parts() {
        assert_eq!(parts("[data]"), vec![("[data]", Part::Table)]);
        assert_eq!(
            parts("# written by tabletist, safe to edit by hand"),
            vec![(
                "# written by tabletist, safe to edit by hand",
                Part::Comment
            )]
        );
        assert_eq!(
            parts("page_size    = 300"),
            vec![("page_size    = ", Part::Key), ("300", Part::Value)]
        );
        assert_eq!(
            parts("timestamps   = \"second\"  # second | full"),
            vec![
                ("timestamps   = ", Part::Key),
                ("\"second\"", Part::String),
                ("  ", Part::Plain),
                ("# second | full", Part::Comment),
            ]
        );
        assert_eq!(
            parts("value_tags   = true"),
            vec![("value_tags   = ", Part::Key), ("true", Part::Value)]
        );
        assert_eq!(parts(""), vec![]);
    }

    #[test]
    fn a_hash_inside_a_string_is_not_a_comment() {
        assert_eq!(
            parts("theme = \"My #1 \\\"Nord\\\".json\"  # a note"),
            vec![
                ("theme = ", Part::Key),
                ("\"My #1 \\\"Nord\\\".json\"", Part::String),
                ("  ", Part::Plain),
                ("# a note", Part::Comment),
            ]
        );
    }

    #[test]
    fn a_line_that_is_none_of_these_is_plain_and_whole() {
        assert_eq!(
            parts("this is not toml"),
            vec![("this is not toml", Part::Plain)]
        );
        // Every byte of a line is in exactly one part, in order.
        for line in [
            "x = ",
            "= 3",
            "[data",
            "  [data]  ",
            "a = \"open",
            "é = \"ü\"  # ö",
        ] {
            let joined: String = parts(line).into_iter().map(|(text, _)| text).collect();
            assert_eq!(joined, line);
        }
    }

    #[test]
    fn the_home_directory_is_written_as_a_tilde() {
        use std::path::Path;
        let home = Path::new("/home/ada");
        assert_eq!(
            shown_path(
                Path::new("/home/ada/.config/tabletist/settings.toml"),
                Some(home)
            ),
            "~/.config/tabletist/settings.toml"
        );
        assert_eq!(
            shown_path(Path::new("/etc/tabletist/settings.toml"), Some(home)),
            "/etc/tabletist/settings.toml"
        );
        assert_eq!(
            shown_path(Path::new("/home/ada/settings.toml"), None),
            "/home/ada/settings.toml"
        );
    }
}
