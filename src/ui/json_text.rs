//! JSON as text in the editor of a document: each of its tokens
//! (`crate::json`) in its colour.

use egui::Color32;
use egui::text::{LayoutJob, TextFormat};

use crate::json::{Kind, tokens};
use crate::theme::{Look, Palette};

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
