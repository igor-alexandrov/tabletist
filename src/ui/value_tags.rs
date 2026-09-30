//! Value tags: the colours of values from a closed set. Only three kinds of
//! column get them: a PostgreSQL enum, a text column whose CHECK constraint
//! is a plain value list (both from the catalog, never from the rows), and
//! booleans. Everything else is plain text, however its values repeat.

use egui::Color32;
use tabletist_db::{ColumnMeta, Structure, Value, ValueKind};

use crate::theme::{Look, Palette};
use crate::ui::grid::Style;

/// The most allowed values a column may have and still be coloured.
pub const SLOTS: usize = 8;

/// Which of a column's values draw as tags.
#[derive(Clone, Debug, PartialEq)]
pub enum Tags<'a> {
    None,
    /// The allowed values, in the database's order: the value at position
    /// `i` takes palette slot `i`.
    Values(&'a [String]),
    Bool,
}

impl<'a> Tags<'a> {
    /// The tags of the result column `column`, from the object's described
    /// structure (not yet described: none).
    pub fn of(column: &ColumnMeta, structure: Option<&'a Structure>) -> Self {
        if column.kind == ValueKind::Bool {
            return Self::Bool;
        }
        structure
            .and_then(|structure| structure.columns.iter().find(|c| c.name == column.name))
            .and_then(|info| info.allowed_values.as_deref())
            .filter(|values| values.len() <= SLOTS)
            .map_or(Self::None, Self::Values)
    }

    /// Each result column's tags.
    pub fn of_page(page: &tabletist_db::RowPage, structure: Option<&'a Structure>) -> Vec<Self> {
        page.columns
            .iter()
            .map(|column| Self::of(column, structure))
            .collect()
    }

    /// How `value` draws, or `None` for plain text. NULL is never a tag,
    /// nor is a value the list does not name (the constraint changed).
    pub fn style(&self, value: &Value) -> Option<Style> {
        match (self, value) {
            (Self::Values(values), Value::Text(text)) => values
                .iter()
                .position(|known| **known == **text)
                .map(Style::Tag),
            (Self::Bool, Value::Bool(flag)) => Some(bool_style(*flag)),
            // SQLite keeps booleans as 0 and 1.
            (Self::Bool, Value::Int(number @ (0 | 1))) => Some(bool_style(*number == 1)),
            _ => None,
        }
    }
}

fn bool_style(flag: bool) -> Style {
    if flag { Style::True } else { Style::False }
}

/// The desktop looks' fixed slots, (fill, text) on a light palette. They
/// keep clear of the environments' red and green.
const FIXED: [(u32, u32); SLOTS] = [
    (0xF7ECDF, 0x8A4A12), // tan
    (0xE6EFFA, 0x1E4F8A), // blue
    (0xEFEAF9, 0x5B3AA8), // violet
    (0xE1F2F1, 0x1D5E5A), // teal
    (0xFBF0D9, 0x7A5200), // amber
    (0xECEEF1, 0x3E4652), // slate
    (0xF9E6EF, 0x8A2E5A), // rose
    (0xEEF3E0, 0x4C5A1E), // olive
];

fn hex(rgb: u32) -> Color32 {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color32::from_rgb(r, g, b)
}

/// The colours of palette slot `slot`: its text, and its fill (none in the
/// terminal look, where the text alone is coloured, selected row or not).
pub fn slot_colors(slot: usize, look: &Look, palette: &Palette) -> (Color32, Option<Color32>) {
    if look.terminal {
        return (terminal_slots(palette)[slot % SLOTS], None);
    }
    let (fill, text) = FIXED[slot % SLOTS];
    let (fill, text) = (hex(fill), hex(text));
    if palette.dark {
        // The same hues on a dark window: the light tone as text over a
        // deep fill of the ink.
        (fill, Some(palette.window.lerp_to_gamma(text, 0.45)))
    } else {
        (text, Some(fill))
    }
}

/// The colours a cell styled `style` draws in: its text, and its chip's
/// fill if it has one.
pub fn style_colors(style: Style, look: &Look, palette: &Palette) -> (Color32, Option<Color32>) {
    match style {
        Style::Tag(slot) => slot_colors(slot, look, palette),
        Style::True if look.terminal => (palette.text, None),
        Style::True => (palette.text, Some(palette.surface)),
        Style::False => (palette.dim, None),
        Style::Plain | Style::Json(_) | Style::Color(_) => (palette.text, None),
    }
}

/// The terminal's slots, from the theme's own colours: orange, cyan,
/// magenta, blue, yellow, muted; then a rose and an olive only when the
/// theme has ones that stand apart from its red and green (which mark
/// environments), else muted again.
pub fn terminal_slots(palette: &Palette) -> [Color32; SLOTS] {
    let rose = distinct_hue(
        palette.rose,
        315.0..=350.0,
        &[palette.danger, palette.magenta],
    );
    let olive = distinct_hue(
        palette.olive,
        50.0..=95.0,
        &[palette.success, palette.warning],
    );
    [
        palette.orange,
        palette.info,
        palette.magenta,
        palette.blue,
        palette.warning,
        palette.dim,
        rose.unwrap_or(palette.dim),
        olive.unwrap_or(palette.dim),
    ]
}

/// `color` if it is saturated, its hue in `range`, and at least 15° from
/// each of `others`.
fn distinct_hue(
    color: Color32,
    range: std::ops::RangeInclusive<f32>,
    others: &[Color32],
) -> Option<Color32> {
    let (hue, saturation) = hue_of(color);
    let apart = |other: Color32| {
        let (other, other_saturation) = hue_of(other);
        let gap = (hue - other).abs();
        other_saturation < 0.25 || gap.min(360.0 - gap) >= 15.0
    };
    (saturation >= 0.25 && range.contains(&hue) && others.iter().copied().all(apart))
        .then_some(color)
}

/// A colour's hue in degrees and its HSV saturation.
fn hue_of(color: Color32) -> (f32, f32) {
    let [r, g, b] = [color.r(), color.g(), color.b()].map(|c| f32::from(c) / 255.0);
    let max = r.max(g).max(b);
    let delta = max - r.min(g).min(b);
    if delta == 0.0 {
        return (0.0, 0.0);
    }
    let hue = if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (hue, delta / max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tabletist_db::{ColumnInfo, RowPage};

    fn meta(name: &str, kind: ValueKind) -> ColumnMeta {
        ColumnMeta {
            name: name.into(),
            type_name: String::new(),
            kind,
        }
    }

    fn structure(columns: &[(&str, Option<&[&str]>)]) -> Structure {
        Structure {
            columns: columns
                .iter()
                .map(|(name, values)| ColumnInfo {
                    name: (*name).into(),
                    allowed_values: values
                        .map(|values| values.iter().map(|value| (*value).to_owned()).collect()),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    #[test]
    fn enum_and_check_values_take_the_slot_of_their_position() {
        // An enum reads as `Other`, a CHECK list as `Text`: both come from
        // the catalog, whatever the kind.
        let structure = structure(&[
            ("size", Some(&["small", "medium", "large"])),
            ("status", Some(&["open", "closed"])),
        ]);
        let size = Tags::of(&meta("size", ValueKind::Other), Some(&structure));
        assert_eq!(size.style(&text("small")), Some(Style::Tag(0)));
        assert_eq!(size.style(&text("large")), Some(Style::Tag(2)));
        let status = Tags::of(&meta("status", ValueKind::Text), Some(&structure));
        assert_eq!(status.style(&text("closed")), Some(Style::Tag(1)));
    }

    #[test]
    fn a_value_keeps_its_colour_whatever_the_rows_hold() {
        let structure = structure(&[("status", Some(&["open", "closed", "held"]))]);
        let status = Tags::of(&meta("status", ValueKind::Text), Some(&structure));
        // Only "held" on this page: it is still slot 2, not the first seen.
        assert_eq!(status.style(&text("held")), Some(Style::Tag(2)));
    }

    #[test]
    fn free_text_is_plain_however_its_values_repeat() {
        let structure = structure(&[("kind", None)]);
        let page = RowPage {
            columns: vec![meta("kind", ValueKind::Text)],
            rows: ["bug", "bug", "task", "task", "bug"]
                .iter()
                .map(|value| vec![text(value)])
                .collect(),
            has_more: false,
            ordered_by_key: true,
            elapsed: std::time::Duration::ZERO,
        };
        let tags = Tags::of_page(&page, Some(&structure));
        assert_eq!(tags, vec![Tags::None]);
        assert!(page.rows.iter().all(|row| tags[0].style(&row[0]).is_none()));
        assert_eq!(
            Tags::of(&meta("kind", ValueKind::Text), None),
            Tags::None,
            "not yet described"
        );
    }

    #[test]
    fn more_than_eight_values_are_plain() {
        let nine = ["a", "b", "c", "d", "e", "f", "g", "h", "i"];
        let structure = structure(&[("grade", Some(&nine)), ("eight", Some(&nine[..8]))]);
        let grade = Tags::of(&meta("grade", ValueKind::Text), Some(&structure));
        assert_eq!(grade, Tags::None);
        assert_eq!(grade.style(&text("a")), None);
        let eight = Tags::of(&meta("eight", ValueKind::Text), Some(&structure));
        assert_eq!(eight.style(&text("h")), Some(Style::Tag(7)));
    }

    #[test]
    fn null_and_unlisted_values_are_never_tags() {
        let structure = structure(&[("status", Some(&["open", "closed"]))]);
        let status = Tags::of(&meta("status", ValueKind::Text), Some(&structure));
        assert_eq!(status.style(&Value::Null), None);
        assert_eq!(
            status.style(&text("archived")),
            None,
            "the constraint changed"
        );
        assert_eq!(status.style(&text("Open")), None);
        assert_eq!(Tags::Bool.style(&Value::Null), None);
    }

    #[test]
    fn booleans_are_a_neutral_tag_or_muted() {
        let tags = Tags::of(&meta("done", ValueKind::Bool), None);
        assert_eq!(tags, Tags::Bool);
        assert_eq!(tags.style(&Value::Bool(true)), Some(Style::True));
        assert_eq!(tags.style(&Value::Bool(false)), Some(Style::False));
        assert_eq!(tags.style(&Value::Int(1)), Some(Style::True), "SQLite");
        assert_eq!(tags.style(&Value::Int(2)), None);
        for look in Look::ALL {
            for palette in [Palette::light(), Palette::dark()] {
                let (text, fill) = style_colors(Style::True, &look, &palette);
                assert_eq!(text, palette.text);
                assert_eq!(fill.is_some(), !look.terminal);
                assert_eq!(
                    style_colors(Style::False, &look, &palette),
                    (palette.dim, None)
                );
                for color in [text, fill.unwrap_or(text)] {
                    assert_ne!(color, palette.danger, "no red");
                    assert_ne!(color, palette.success, "no green");
                }
            }
        }
    }

    #[test]
    fn desktop_slots_use_the_fixed_table() {
        let light = Palette::light();
        for look in [Look::macos(), Look::standard()] {
            assert_eq!(
                slot_colors(0, &look, &light),
                (hex(0x8A4A12), Some(hex(0xF7ECDF)))
            );
            assert_eq!(
                slot_colors(7, &look, &light),
                (hex(0x4C5A1E), Some(hex(0xEEF3E0)))
            );
        }
    }

    #[test]
    fn terminal_slots_follow_the_theme_without_fills() {
        let mut palette = Palette::dark();
        let look = Look::omarchy();
        let slots = terminal_slots(&palette);
        assert_eq!(
            slots[..6],
            [
                palette.orange,
                palette.info,
                palette.magenta,
                palette.blue,
                palette.warning,
                palette.dim
            ]
        );
        assert_eq!(slots[6], palette.rose, "a distinct rose");
        assert_eq!(slots[7], palette.olive, "a distinct olive");
        assert_eq!(slot_colors(2, &look, &palette), (palette.magenta, None));
        // A theme whose bright red and green are its red and green: muted.
        palette.rose = palette.danger;
        palette.olive = palette.success;
        let slots = terminal_slots(&palette);
        assert_eq!(slots[6], palette.dim);
        assert_eq!(slots[7], palette.dim);
        // A bright red that is only a lighter red is no rose.
        palette.rose = Color32::from_rgb(0xff, 0x7a, 0x70);
        assert_eq!(terminal_slots(&palette)[6], palette.dim);
    }

    #[test]
    fn hues_are_measured_in_degrees() {
        assert_eq!(hue_of(Color32::from_rgb(255, 0, 0)).0, 0.0);
        assert_eq!(hue_of(Color32::from_rgb(0, 255, 0)).0, 120.0);
        assert_eq!(hue_of(Color32::from_rgb(0, 0, 255)).0, 240.0);
        assert!((hue_of(Color32::from_rgb(255, 0, 128)).0 - 329.9).abs() < 0.2);
        assert_eq!(hue_of(Color32::from_gray(128)), (0.0, 0.0));
    }
}
