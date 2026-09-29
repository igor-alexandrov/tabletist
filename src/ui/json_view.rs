//! JSON in the row panel: highlighted, with objects and arrays that fold.

use std::sync::Arc;

use egui::cache::{ComputerMut, FrameCache};
use egui::text::LayoutJob;
use egui::{Align, Color32, FontId, Id, Label, Layout, Sense, TextFormat, Ui, WidgetInfo, vec2};
use serde::de::{MapAccess, SeqAccess, Visitor};

use crate::i18n::{Locale, gettext, ngettext};
use crate::theme::{self, Icon, Palette};

/// JSON larger than this is shown as plain text, not parsed into a tree.
pub const TREE_MAX: usize = 256 * 1024;
/// Documents this short open fully; longer ones open only the top level.
const OPEN_LINES: usize = 40;
/// Most lines one value draws, so expanding a huge document stays fast.
const MAX_ROWS: usize = 1_000;
/// Characters of a string the tree shows; copying gives the whole value.
const STRING_MAX: usize = 2_000;
/// Indent per nesting level, in points.
const INDENT: f32 = 14.0;
/// The fold toggle's width, and the gutter leaves keep so keys line up.
const TOGGLE: f32 = 14.0;

/// A JSON value as the tree shows it. Keys keep their document order
/// (serde_json's own map sorts them), and strings and keys are stored as
/// JSON literals (quoted, escaped, cut at STRING_MAX characters).
#[derive(Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(Box<str>),
    String(Box<str>),
    Array(Vec<Json>),
    Object(Vec<(Box<str>, Json)>),
}

/// A parsed document and how many lines it takes fully open.
#[derive(Debug)]
pub struct Doc {
    root: Json,
    lines: usize,
    containers: usize,
}

pub fn parse(text: &str) -> Option<Doc> {
    let root: Json = serde_json::from_str(text).ok()?;
    let (lines, containers) = measure(&root);
    Some(Doc {
        root,
        lines,
        containers,
    })
}

/// Lines when fully open, and non-empty objects and arrays.
fn measure(value: &Json) -> (usize, usize) {
    let children: Box<dyn Iterator<Item = &Json>> = match value {
        Json::Array(items) if !items.is_empty() => Box::new(items.iter()),
        Json::Object(entries) if !entries.is_empty() => Box::new(entries.iter().map(|(_, v)| v)),
        _ => return (1, 0),
    };
    children.fold((2, 1), |(lines, containers), child| {
        let (child_lines, child_containers) = measure(child);
        (lines + child_lines, containers + child_containers)
    })
}

/// `text` as a JSON string literal, cut at STRING_MAX characters.
fn literal(text: &str) -> Box<str> {
    let quoted = serde_json::to_string(text).unwrap_or_else(|_| format!("{text:?}"));
    match quoted.char_indices().nth(STRING_MAX) {
        Some((cut, _)) => format!("{}…\"", &quoted[..cut]).into(),
        None => quoted.into(),
    }
}

impl<'de> serde::Deserialize<'de> for Json {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(JsonVisitor)
    }
}

struct JsonVisitor;

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Json;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_unit<E>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_bool<E>(self, flag: bool) -> Result<Json, E> {
        Ok(Json::Bool(flag))
    }

    fn visit_i64<E>(self, number: i64) -> Result<Json, E> {
        Ok(Json::Number(number.to_string().into()))
    }

    fn visit_u64<E>(self, number: u64) -> Result<Json, E> {
        Ok(Json::Number(number.to_string().into()))
    }

    fn visit_f64<E>(self, number: f64) -> Result<Json, E> {
        // serde_json's formatting: `1e+300`, not three hundred zeros.
        let text = serde_json::Number::from_f64(number)
            .map_or_else(|| number.to_string(), |number| number.to_string());
        Ok(Json::Number(text.into()))
    }

    fn visit_str<E>(self, text: &str) -> Result<Json, E> {
        Ok(Json::String(literal(text)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut items = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Json::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((key, value)) = map.next_entry::<std::borrow::Cow<'de, str>, Json>()? {
            entries.push((literal(&key), value));
        }
        Ok(Json::Object(entries))
    }
}

#[derive(Default)]
struct Parser;

impl ComputerMut<&str, Option<Arc<Doc>>> for Parser {
    fn compute(&mut self, text: &str) -> Option<Arc<Doc>> {
        parse(text).map(Arc::new)
    }
}

/// `text` parsed, from a cache kept while the value stays on screen, so a
/// document is not re-parsed every frame.
pub fn parsed(ui: &Ui, text: &str) -> Option<Arc<Doc>> {
    ui.memory_mut(|memory| {
        memory
            .caches
            .cache::<FrameCache<Option<Arc<Doc>>, Parser>>()
            .get(text)
            .clone()
    })
}

/// Expand all / Collapse all: a new generation forgets every node's own
/// state and opens or closes them all.
#[derive(Clone, Copy, Default)]
struct Folding {
    generation: u32,
    all: Option<bool>,
}

/// Draws `doc`. `id` identifies the value; `name` (the column) is the root's
/// accessible name.
pub fn show(ui: &mut Ui, id: Id, doc: &Doc, name: &str, locale: Locale, palette: &Palette) {
    let folding: Folding = ui.data(|data| data.get_temp(id)).unwrap_or_default();
    let mut view = View {
        locale,
        palette,
        font: theme::mono(theme::TEXT_MONO),
        all: folding.all,
        open_all: doc.lines <= OPEN_LINES,
        rows: 0,
        folded: 0,
    };
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
        let slot = Slot {
            key: None,
            name,
            comma: false,
        };
        view.node(ui, 0, slot, &doc.root, id.with(folding.generation));
    });
    if view.rows >= MAX_ROWS {
        ui.label(
            egui::RichText::new(format!(
                "…\n{}",
                gettext(locale, "(Copy gives the whole value.)")
            ))
            .font(view.font.clone())
            .color(palette.dim),
        );
    }
    if doc.containers > 1 {
        let open = view.folded > 0;
        let label = if open {
            gettext(locale, "Expand all")
        } else {
            gettext(locale, "Collapse all")
        };
        if ui.link(label).clicked() {
            let folding = Folding {
                generation: folding.generation.wrapping_add(1),
                all: Some(open),
            };
            ui.data_mut(|data| data.insert_temp(id, folding));
        }
    }
}

/// Where a value sits: its key in an object, its accessible name as a path
/// (`meta.tags[0]`)
/// and whether a comma follows it.
struct Slot<'s> {
    key: Option<&'s str>,
    name: &'s str,
    comma: bool,
}

struct View<'a> {
    locale: Locale,
    palette: &'a Palette,
    font: FontId,
    /// Set by Expand all / Collapse all.
    all: Option<bool>,
    /// The document is short enough to open fully.
    open_all: bool,
    rows: usize,
    /// Folded objects and arrays drawn, so Expand all has work to do.
    folded: usize,
}

impl View<'_> {
    fn node(&mut self, ui: &mut Ui, depth: usize, slot: Slot<'_>, value: &Json, id: Id) {
        if self.rows >= MAX_ROWS {
            return;
        }
        let Slot { key, name, comma } = slot;
        let mut job = LayoutJob::default();
        if let Some(key) = key {
            self.push(&mut job, key, self.palette.accent);
            self.push(&mut job, ": ", self.palette.secondary);
        }
        let (open_bracket, close_bracket, count, noun) = match value {
            Json::Array(items) if !items.is_empty() => {
                let count = items.len();
                ("[", "]", count, self.noun(count, "item", "items"))
            }
            Json::Object(entries) if !entries.is_empty() => {
                let count = entries.len();
                ("{", "}", count, self.noun(count, "key", "keys"))
            }
            leaf => {
                let (text, color) = match leaf {
                    Json::Null => ("null", self.palette.dim),
                    Json::Bool(true) => ("true", self.palette.warning),
                    Json::Bool(false) => ("false", self.palette.warning),
                    Json::Number(number) => (&**number, self.palette.warning),
                    Json::String(text) => (&**text, self.palette.text),
                    Json::Array(_) => ("[]", self.palette.secondary),
                    Json::Object(_) => ("{}", self.palette.secondary),
                };
                self.push(&mut job, text, color);
                if comma {
                    self.push(&mut job, ",", self.palette.secondary);
                }
                self.row(ui, depth, None, job);
                return;
            }
        };
        let open = ui
            .data(|data| data.get_temp(id))
            .unwrap_or(self.all.unwrap_or(depth == 0 || self.open_all));
        self.push(&mut job, open_bracket, self.palette.secondary);
        if !open {
            self.folded += 1;
            self.push(&mut job, &format!(" {count} {noun} "), self.palette.dim);
            self.push(&mut job, close_bracket, self.palette.secondary);
            if comma {
                self.push(&mut job, ",", self.palette.secondary);
            }
        }
        let verb = if open { "Collapse" } else { "Expand" };
        let toggle = format!("{} {name}", gettext(self.locale, verb));
        if self.row(ui, depth, Some((open, &toggle)), job) {
            ui.data_mut(|data| data.insert_temp(id, !open));
        }
        if !open {
            return;
        }
        match value {
            Json::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    let slot = Slot {
                        key: None,
                        name: &format!("{name}[{index}]"),
                        comma: index + 1 < items.len(),
                    };
                    self.node(ui, depth + 1, slot, item, id.with(index));
                }
            }
            Json::Object(entries) => {
                for (index, (key, item)) in entries.iter().enumerate() {
                    let slot = Slot {
                        key: Some(key),
                        // `meta.plan`: the key without its quotes.
                        name: &format!("{name}.{}", &key[1..key.len() - 1]),
                        comma: index + 1 < entries.len(),
                    };
                    self.node(ui, depth + 1, slot, item, id.with(index));
                }
            }
            _ => {}
        }
        if self.rows < MAX_ROWS {
            let mut job = LayoutJob::default();
            self.push(&mut job, close_bracket, self.palette.secondary);
            if comma {
                self.push(&mut job, ",", self.palette.secondary);
            }
            self.row(ui, depth, None, job);
        }
    }

    fn noun(&self, count: usize, one: &'static str, many: &'static str) -> String {
        let count = u32::try_from(count).unwrap_or(u32::MAX);
        ngettext(self.locale, one, many, count).into_owned()
    }

    fn push(&self, job: &mut LayoutJob, text: &str, color: Color32) {
        job.append(text, 0.0, TextFormat::simple(self.font.clone(), color));
    }

    /// One line: indent, fold toggle (or its gutter), then `job`. Returns
    /// whether the toggle was clicked.
    fn row(
        &mut self,
        ui: &mut Ui,
        depth: usize,
        toggle: Option<(bool, &str)>,
        job: LayoutJob,
    ) -> bool {
        self.rows += 1;
        let height = ui.fonts_mut(|fonts| fonts.row_height(&self.font));
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            ui.add_space(depth as f32 * INDENT);
            let clicked = match toggle {
                Some((open, label)) => self.toggle(ui, open, label, height),
                None => {
                    ui.add_space(TOGGLE);
                    false
                }
            };
            ui.add(Label::new(job).selectable(true).wrap());
            clicked
        })
        .inner
    }

    fn toggle(&self, ui: &mut Ui, open: bool, label: &str, height: f32) -> bool {
        let (rect, response) = ui.allocate_exact_size(vec2(TOGGLE, height), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, label));
        let icon = if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        };
        let tint = if response.hovered() {
            self.palette.text
        } else {
            self.palette.secondary
        };
        icon.image(tint, 12.0).paint_at(
            ui,
            egui::Rect::from_center_size(rect.center(), vec2(12.0, 12.0)),
        );
        response.on_hover_text(label).clicked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_keep_their_order_and_strings_stay_json_literals() {
        let doc = parse(r#"{"z":"a\nb","a":[1,2.5,1e300,true,null],"e":{}}"#).unwrap();
        assert_eq!(
            doc.root,
            Json::Object(vec![
                ("\"z\"".into(), Json::String(r#""a\nb""#.into())),
                (
                    "\"a\"".into(),
                    Json::Array(vec![
                        Json::Number("1".into()),
                        Json::Number("2.5".into()),
                        Json::Number("1e+300".into()),
                        Json::Bool(true),
                        Json::Null,
                    ])
                ),
                ("\"e\"".into(), Json::Object(Vec::new())),
            ])
        );
        // { "z", "a": [ five items ], "e" }
        assert_eq!(doc.lines, 11);
        assert_eq!(doc.containers, 2);
    }

    #[test]
    fn invalid_json_does_not_parse() {
        assert!(parse("not json").is_none());
        assert!(parse(r#"{"a":1"#).is_none());
    }

    #[test]
    fn long_strings_are_cut() {
        let doc = parse(&format!("\"{}\"", "x".repeat(STRING_MAX * 2))).unwrap();
        let Json::String(text) = doc.root else {
            panic!("a string")
        };
        assert_eq!(text.chars().count(), STRING_MAX + 2);
        assert!(text.ends_with("…\""));
    }
}
