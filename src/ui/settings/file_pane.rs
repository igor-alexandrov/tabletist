//! The settings file as the Settings screen shows it: its own text, a line
//! of the file per line, with each part in its colour. This is no TOML
//! parser: it tells apart what the app writes and leaves the rest plain.
//! The pane is drawn at the end of this file, from the parts before it,
//! which draw nothing.

use std::ops::Range;
use std::path::Path;

use egui::{Color32, CornerRadius, Rangef, Rect, Sense, Ui, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::settings::OptionId;
use crate::theme::Palette;
use crate::typography::{Text, TextRole};
use crate::ui::widgets;

use super::terminal::Skin;

/// The pane's header without its rule, how far its words stand in from its
/// sides, and the space over the file's first line.
const HEADER: f32 = 34.0;
const SIDE: f32 = 14.0;
const TOP: f32 = 12.0;
/// The space over and under the pane's note.
const NOTE: f32 = 10.0;
/// The most of a line that is drawn, in bytes: several times what the pane
/// is wide. A text that is no settings file can be one line of megabytes,
/// and a line is laid out whole before it is clipped.
const LINE_MAX: usize = 400;

/// The pane's note, whole: a language has its own place for the word that
/// stands between the marks, which is written in the colour it names.
const NOTE_TEXT: &str =
    "Edits in the file reload live · invalid lines are shown here in {red}red{/red} and ignored";

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

/// `text` around the word it marks with `{name}` and `{/name}`: what is
/// before the word, the word, and what is after it. A text that lost a mark
/// (a translation's slip) is all before, and no word of it stands out.
pub(super) fn marked_word<'a>(text: &'a str, name: &str) -> (&'a str, &'a str, &'a str) {
    let (open, close) = (format!("{{{name}}}"), format!("{{/{name}}}"));
    let found = text.split_once(&open).and_then(|(before, rest)| {
        let (word, after) = rest.split_once(&close)?;
        Some((before, word, after))
    });
    found.unwrap_or((text, "", ""))
}

/// The lines of a text of `count` that show in `view`, counted from 0: the
/// first line starts at `top` and each is `line` tall.
pub(super) fn lines_in_view(top: f32, line: f32, count: usize, view: Rangef) -> Range<usize> {
    if line <= 0.0 || view.max <= view.min {
        return 0..0;
    }
    // In lines from the first one's top. A view that starts over the text
    // starts at its first line, and one that ends under it ends at its last.
    let within = |lines: f32| (lines.max(0.0) as usize).min(count);
    let first = within(((view.min - top) / line).floor());
    let last = within(((view.max - top) / line).ceil());
    first..last.max(first)
}

/// What is drawn of `line`: all of it, or its first `LINE_MAX` bytes, less
/// what would cut a character in two.
pub(super) fn drawn(line: &str) -> &str {
    if line.len() <= LINE_MAX {
        return line;
    }
    let mut end = LINE_MAX;
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    &line[..end]
}

/// The settings file beside its options, in `rect`: where it is and whether
/// it is watched, its text with the line of the cursor's option marked, and
/// what its colours say.
pub(super) fn show(ui: &mut Ui, rect: Rect, app: &App, option: Option<OptionId>, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let role = widgets::code(look);
    let file = &app.settings_file;
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette.panel);
    widgets::vline(ui, rect.left() + 0.5, rect.y_range(), palette.outline);
    let left = rect.left() + 1.0 + SIDE;
    let right = (rect.right() - SIDE).max(left);

    let rule = (rect.top() + HEADER).min(rect.bottom());
    widgets::hline(ui, rect.x_range(), rule + 0.5, palette.outline);
    let y = rect.top() + HEADER / 2.0;
    // Where the path ends: at the pane's side, or before the word there.
    let mut path_right = right;
    if file.live {
        let live = Text::one(look, role, &skin.say("live"), palette.dim);
        path_right -= widgets::paint_text_right(ui, right, y, live) + 12.0;
    }
    let path = super::path_shown(app);
    // The pane's heading: strong, as a heading over a rule is in this look.
    let path = Text::one(look, TextRole::OGroup, &path, palette.text).layout(ui.ctx());
    let clip = Rect::from_min_max(pos2(left, rect.top()), pos2(path_right.max(left), rule));
    let clipped = ui.painter().with_clip_rect(clip);
    // A path longer than the header keeps its end: the file's name is there.
    if path.width() > clip.width() {
        path.paint_right(&clipped, clip.right(), y);
    } else {
        path.paint_left(&clipped, left, y);
    }
    widgets::announce(ui, clip, path.galley.text());

    // The note before the text: how tall it is says where the text ends.
    // Split after it is translated, each piece in the look's case.
    let note = gettext(skin.locale, NOTE_TEXT);
    let (before, word, after) = marked_word(&note, "red");
    let note = Text::new(look)
        .add(role, &look.label(before), palette.dim)
        .add(role, &look.label(word), palette.danger)
        .add(role, &look.label(after), palette.dim)
        .wrap(right - left)
        .layout(ui.ctx());
    let foot = (rect.bottom() - (2.0 * NOTE + note.height()).ceil() - 1.0).max(rule + 1.0);
    widgets::hline(ui, rect.x_range(), foot + 0.5, palette.outline);
    let under = Rect::from_min_max(pos2(rect.left(), foot + 1.0), rect.max);
    note.paint(
        &ui.painter().with_clip_rect(under),
        pos2(left, under.top() + NOTE),
    );

    let marked = option.and_then(|option| {
        let line = file.lines.iter().find(|(key, _)| *key == option.key());
        line.map(|(_, line)| *line)
    });
    let line = role.row_height(ui.ctx(), look.faces);
    // The text scrolls between the header and the note, past the pane's
    // rule.
    let body = Rect::from_min_max(
        pos2(rect.left() + 1.0, rule + 1.0),
        pos2(rect.right(), foot),
    );
    let mut scroll = ui.new_child(egui::UiBuilder::new().id_salt("file").max_rect(body));
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(&mut scroll, |ui| {
            // As tall as all its lines, which were found when the text was
            // set: a text that is no settings file can be megabytes long,
            // and is shown all the same.
            let count = file.line_count();
            let height = 2.0 * TOP + count as f32 * line;
            let (place, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let top = place.top() + TOP;
            // For the same reason only the lines in view are told apart and
            // painted, each reached by where it starts: the ones before
            // them are not walked, and the ones after them are not reached.
            let shown = lines_in_view(top, line, count, ui.clip_rect().y_range());
            for index in shown {
                // The lines as `Settings::from_toml` counts them, from 1.
                // The end of a line as another system's editor wrote it is
                // not drawn, nor is more of a line than the pane could show.
                let Some(text) = file.line(index).map(drawn) else {
                    break;
                };
                let number = index + 1;
                let row = Rect::from_min_size(
                    pos2(place.left(), top + index as f32 * line),
                    vec2(place.width(), line),
                );
                if marked == Some(number) {
                    ui.painter()
                        .rect_filled(row, CornerRadius::ZERO, palette.selection);
                }
                let y = row.center().y;
                let mut x = row.left() + SIDE;
                // A line that was ignored is one colour: nothing in it was
                // read. The ignored lines are in order, and a text that was
                // not read has every line among them.
                let ignored = file.invalid.binary_search(&number).is_ok();
                // Painted text is nothing to a screen reader: each line in
                // view is told to it, and an ignored one is said to be, in
                // words where the eye has a colour.
                if ignored {
                    let said = format!("{}: {text}", gettext(skin.locale, "Ignored"));
                    widgets::announce(ui, row, &said);
                } else if !text.is_empty() {
                    widgets::announce(ui, row, text);
                }
                if ignored {
                    widgets::paint_text(ui, x, y, Text::one(look, role, text, palette.danger));
                    continue;
                }
                for (range, part) in spans(text) {
                    let piece = Text::one(look, role, &text[range], part.color(palette));
                    x += widgets::paint_text(ui, x, y, piece);
                }
            }
        });
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
    fn a_line_longer_than_the_pane_could_show_is_drawn_to_a_whole_character() {
        assert_eq!(drawn(""), "");
        assert_eq!(drawn("page_size = 300"), "page_size = 300");
        let exact = "x".repeat(LINE_MAX);
        assert_eq!(drawn(&exact), exact);
        let long = "x".repeat(1_000_000);
        assert_eq!(drawn(&long).len(), LINE_MAX);
        // `ä` is two bytes, and the cut would fall inside one: it is left
        // out whole.
        let accents = format!("x{}", "ä".repeat(LINE_MAX));
        let cut = drawn(&accents);
        assert_eq!(cut.len(), LINE_MAX - 1);
        assert!(accents.starts_with(cut));
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

    #[test]
    fn the_word_a_text_marks_is_told_from_what_is_around_it() {
        assert_eq!(
            marked_word("shown in {red}red{/red} and ignored", "red"),
            ("shown in ", "red", " and ignored")
        );
        // Another order of words, as another language has it.
        assert_eq!(
            marked_word("{red}rot{/red} gezeigt", "red"),
            ("", "rot", " gezeigt")
        );
        // A mark that was lost: the text is whole, and nothing stands out.
        for text in ["shown in red", "shown in {red}red", "shown in red{/red}"] {
            assert_eq!(marked_word(text, "red"), (text, "", ""));
        }
        // The pane's own note has both.
        assert_eq!(marked_word(NOTE_TEXT, "red").1, "red");
    }

    #[test]
    fn only_the_lines_in_view_are_walked() {
        // Lines of 10 from 100 down: line 0 is 100..110, line 5 150..160.
        let shown = |count, min, max| lines_in_view(100.0, 10.0, count, Rangef::new(min, max));
        // A view in the middle of a long text: the lines it cuts are in,
        // the ones before and after are not.
        assert_eq!(shown(5_000, 155.0, 185.0), 5..9);
        assert_eq!(shown(5_000, 150.0, 180.0), 5..8);
        // The start of the text, with the space over its first line.
        assert_eq!(shown(5_000, 80.0, 125.0), 0..3);
        // A text that ends in the view ends there.
        assert_eq!(shown(7, 155.0, 400.0), 5..7);
        assert_eq!(shown(0, 0.0, 400.0), 0..0);
        // A view wholly past the text, or wholly before it, has no lines.
        assert!(shown(7, 500.0, 900.0).is_empty());
        assert!(shown(7, 0.0, 50.0).is_empty());
        assert!(shown(7, 185.0, 155.0).is_empty());
    }
}
