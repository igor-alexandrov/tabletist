//! JSON as text, for the editor of a document: its tokens, to colour what
//! is being typed and to lay it out in lines. Nothing here reads a value:
//! a text that is no JSON yet is scanned as far as its tokens go, nothing
//! fails, and what is laid out again differs from what was typed only in
//! the white space between its tokens.

use std::ops::Range;

use egui::Color32;
use egui::text::{LayoutJob, TextFormat};

use crate::theme::{Look, Palette};

/// What a piece of the text is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A string that names a member: a colon follows it.
    Key,
    String,
    Number,
    /// `true`, `false` or `null`.
    Literal,
    /// One of `{}[],:`.
    Punctuation,
    Space,
    /// Anything else: what a text that is no JSON yet holds.
    Other,
}

/// The pieces of `text`, in order, covering all of it.
pub fn tokens(text: &str) -> Vec<(Kind, Range<usize>)> {
    let bytes = text.as_bytes();
    let mut found: Vec<(Kind, Range<usize>)> = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let start = at;
        let kind = match bytes[at] {
            b'"' => {
                at += 1;
                // To the closing quote, or the text's end: a backslash
                // takes the byte after it.
                while at < bytes.len() && bytes[at] != b'"' {
                    at += if bytes[at] == b'\\' { 2 } else { 1 };
                }
                at = (at + 1).min(bytes.len());
                // Never inside a character: the text may end in half of
                // an escape.
                while !text.is_char_boundary(at) {
                    at += 1;
                }
                let after = bytes[at..].iter().find(|byte| !byte.is_ascii_whitespace());
                if after == Some(&b':') {
                    Kind::Key
                } else {
                    Kind::String
                }
            }
            b'{' | b'}' | b'[' | b']' | b',' | b':' => {
                at += 1;
                Kind::Punctuation
            }
            byte if byte.is_ascii_whitespace() => {
                while at < bytes.len() && bytes[at].is_ascii_whitespace() {
                    at += 1;
                }
                Kind::Space
            }
            byte if byte == b'-' || byte.is_ascii_digit() => {
                let number = |byte: &u8| byte.is_ascii_digit() || b"-+.eE".contains(byte);
                while at < bytes.len() && number(&bytes[at]) {
                    at += 1;
                }
                Kind::Number
            }
            _ => {
                // A word, or a run of whatever else was typed, up to what
                // starts another piece.
                let ends = |byte: &u8| byte.is_ascii_whitespace() || b"\"{}[],:".contains(byte);
                while at < bytes.len() && !ends(&bytes[at]) {
                    at += 1;
                }
                match &text[start..at] {
                    "true" | "false" | "null" => Kind::Literal,
                    _ => Kind::Other,
                }
            }
        };
        found.push((kind, start..at));
    }
    found
}

/// The colour a piece of `kind` is written in: a document's tree's, so it
/// reads the same folded and edited.
pub fn color(kind: Kind, look: &Look, palette: &Palette) -> Color32 {
    match kind {
        Kind::Key => crate::ui::json_view::key_color(look, palette),
        Kind::String => palette.success,
        Kind::Number => palette.orange,
        Kind::Literal => palette.orange,
        Kind::Punctuation | Kind::Space | Kind::Other => palette.text,
    }
}

/// `text` as a job to lay out: each piece in its colour, in the format
/// `format` gives a colour.
pub fn job(
    text: &str,
    format: impl Fn(Color32) -> TextFormat,
    look: &Look,
    palette: &Palette,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (kind, range) in tokens(text) {
        // `null` is quieter than a value, as in the tree.
        let ink = match (kind, &text[range.clone()]) {
            (Kind::Literal, "null") => palette.dim,
            _ => color(kind, look, palette),
        };
        job.append(&text[range], 0.0, format(ink));
    }
    job
}

/// `text` laid out a member and an element to a line, two spaces deeper
/// with each level. Only the white space between its pieces changes: no
/// key, string or number is read, so none can come out other than it went
/// in.
pub fn pretty(text: &str) -> String {
    let pieces: Vec<(Kind, &str)> = tokens(text)
        .into_iter()
        .filter(|(kind, _)| *kind != Kind::Space)
        .map(|(kind, range)| (kind, &text[range]))
        .collect();
    let mut out = String::with_capacity(text.len() * 2);
    let mut depth: usize = 0;
    let line = |out: &mut String, depth: usize| {
        out.push('\n');
        out.extend(std::iter::repeat_n("  ", depth));
    };
    for (index, (kind, piece)) in pieces.iter().enumerate() {
        let next = pieces.get(index + 1).map(|(_, piece)| *piece);
        match (kind, *piece) {
            (Kind::Punctuation, "{" | "[") => {
                out.push_str(piece);
                // An empty one stays on its line: `{}`.
                if !matches!(next, Some("}" | "]") | None) {
                    depth += 1;
                    line(&mut out, depth);
                }
            }
            (Kind::Punctuation, "}" | "]") => {
                let opener = index
                    .checked_sub(1)
                    .and_then(|before| pieces.get(before))
                    .is_some_and(|(_, before)| matches!(*before, "{" | "["));
                if !opener {
                    depth = depth.saturating_sub(1);
                    line(&mut out, depth);
                }
                out.push_str(piece);
            }
            (Kind::Punctuation, ",") => {
                out.push(',');
                line(&mut out, depth);
            }
            (Kind::Punctuation, ":") => out.push_str(": "),
            _ => {
                out.push_str(piece);
                // Two pieces with no punctuation between them (a text
                // that is no JSON) stay apart, as they were typed.
                let glue = pieces
                    .get(index + 1)
                    .is_some_and(|(kind, _)| !matches!(kind, Kind::Punctuation));
                if glue {
                    out.push(' ');
                }
            }
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<(Kind, &str)> {
        let found = tokens(text).into_iter();
        found
            .filter(|(kind, _)| *kind != Kind::Space)
            .map(|(kind, range)| (kind, &text[range]))
            .collect()
    }

    #[test]
    fn a_document_is_told_apart_by_its_tokens() {
        let text = r#"{"a": "b", "n": -1.5e3, "t": true, "z": null}"#;
        assert_eq!(
            kinds(text),
            [
                (Kind::Punctuation, "{"),
                (Kind::Key, r#""a""#),
                (Kind::Punctuation, ":"),
                (Kind::String, r#""b""#),
                (Kind::Punctuation, ","),
                (Kind::Key, r#""n""#),
                (Kind::Punctuation, ":"),
                (Kind::Number, "-1.5e3"),
                (Kind::Punctuation, ","),
                (Kind::Key, r#""t""#),
                (Kind::Punctuation, ":"),
                (Kind::Literal, "true"),
                (Kind::Punctuation, ","),
                (Kind::Key, r#""z""#),
                (Kind::Punctuation, ":"),
                (Kind::Literal, "null"),
                (Kind::Punctuation, "}"),
            ]
        );
        // A quote and a brace inside a string are the string's.
        let text = r#"["a \" { b", "c"]"#;
        assert_eq!(kinds(text)[1], (Kind::String, r#""a \" { b""#));
    }

    #[test]
    fn the_tokens_cover_every_text_and_nothing_fails() {
        for text in [
            "",
            "   ",
            r#"{"a": "#,
            r#"{"unclosed"#,
            r#""ends in a backslash\"#,
            "nope { ] :: 12abc é \"ü\"",
            "{\"k\":\"\u{1F600}\"}",
            "\"\\",
        ] {
            let found = tokens(text);
            let whole: String = found
                .iter()
                .map(|(_, range)| &text[range.clone()])
                .collect();
            assert_eq!(whole, text);
            // Laid out again it is scanned again without a panic.
            let again = pretty(text);
            let _ = tokens(&again);
        }
    }

    #[test]
    fn a_document_is_laid_out_a_member_to_a_line() {
        let text = r#"{"id":"a/b.png","meta":{"size":6024,"tags":["x","y"],"none":{}},"list":[]}"#;
        let laid = pretty(text);
        assert_eq!(
            laid,
            r#"{
  "id": "a/b.png",
  "meta": {
    "size": 6024,
    "tags": [
      "x",
      "y"
    ],
    "none": {}
  },
  "list": []
}"#
        );
        // Laid out twice it is as laid out once, and its pieces are the
        // ones that went in, in their order.
        assert_eq!(pretty(&laid), laid);
        assert_eq!(kinds(&laid), kinds(text));
    }

    #[test]
    fn no_value_is_read_so_none_changes() {
        // A number no float holds, a key twice, an escape: as typed.
        let text = r#"{"n":123456789012345678901234567890.10,"k":1,"k":2,"s":"é\n"}"#;
        let laid = pretty(text);
        for piece in [
            "123456789012345678901234567890.10",
            r#""é\n""#,
            r#""k": 1"#,
            r#""k": 2"#,
        ] {
            assert!(laid.contains(piece), "{piece}: {laid}");
        }
    }
}
