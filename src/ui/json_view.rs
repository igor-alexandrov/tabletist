//! JSON in the row panel: highlighted, with objects and arrays that fold.

use std::sync::Arc;

use egui::cache::{ComputerMut, FrameCache};
use egui::{Align, Color32, Id, Label, Layout, Sense, Ui, WidgetInfo, vec2};
use serde::de::{MapAccess, SeqAccess, Visitor};
use tabletist_db::{Value, ValueKind};

use crate::i18n::{Locale, gettext, ngettext};
use crate::theme::{self, Icon, Palette};
use crate::typography::{Text, TextRole};

/// JSON larger than this is shown as plain text, not parsed into a tree.
pub const TREE_MAX: usize = 256 * 1024;
/// JSON larger than this is not summarised in a grid cell.
pub const CELL_MAX: usize = 16 * 1024;
/// Documents this short open fully; longer ones open only the top level.
const OPEN_LINES: usize = 40;
/// Most lines one value draws, so expanding a huge document stays fast.
const MAX_ROWS: usize = 1_000;
/// Characters of a string the tree shows; copying gives the whole value.
const STRING_MAX: usize = 2_000;
/// The fold toggle's width, in the indent left of its line.
const TOGGLE: f32 = 11.0;

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

/// `text` as a JSON string literal, cut at STRING_MAX characters, with
/// hidden characters written out (serde_json escapes only C0 controls, and
/// would leave a U+202E to turn the text after it around).
fn literal(text: &str) -> Box<str> {
    let quoted = serde_json::to_string(text).unwrap_or_else(|_| format!("{text:?}"));
    let cut = match quoted.char_indices().nth(STRING_MAX) {
        Some((cut, _)) => format!("{}…\"", &quoted[..cut]),
        None => quoted,
    };
    match crate::ui::format::escape_hidden(&cut) {
        std::borrow::Cow::Borrowed(_) => cut.into(),
        std::borrow::Cow::Owned(escaped) => escaped.into(),
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
fn parsed_in(ctx: &egui::Context, text: &str) -> Option<Arc<Doc>> {
    ctx.memory_mut(|memory| {
        memory
            .caches
            .cache::<FrameCache<Option<Arc<Doc>>, Parser>>()
            .get(text)
            .clone()
    })
}

/// The document `value` holds, when it is at most `max` bytes. A JSON column
/// holds one whatever its value; a text column holds one only when its value
/// is an object or an array (a number or a word there is just text). Other
/// kinds never do: PostgreSQL writes an empty array as `{}` and a range as
/// `[1,2]`.
pub fn document(
    ctx: &egui::Context,
    kind: ValueKind,
    value: &Value,
    max: usize,
) -> Option<Arc<Doc>> {
    let Value::Text(text) = value else {
        return None;
    };
    let holds = match kind {
        ValueKind::Json => true,
        ValueKind::Text => looks_like_document(text),
        _ => false,
    };
    if holds && text.len() <= max {
        parsed_in(ctx, text)
    } else {
        None
    }
}

/// Whether `text` opens and closes as a JSON object or array does. Cheap, so
/// ordinary text is never hashed or parsed.
fn looks_like_document(text: &str) -> bool {
    let bytes = text.trim_ascii().as_bytes();
    matches!(
        (bytes.first(), bytes.last()),
        (Some(b'{'), Some(b'}')) | (Some(b'['), Some(b']'))
    )
}

/// What a JSON cell shows: how many keys (or items) its top level holds,
/// and its first strings, depth first.
pub fn summary(doc: &Doc) -> (usize, Vec<String>) {
    fn strings(value: &Json, out: &mut Vec<String>) {
        if out.len() >= 2 {
            return;
        }
        match value {
            Json::String(text) => out.push(unquote(text)),
            Json::Array(items) => items.iter().for_each(|item| strings(item, out)),
            Json::Object(entries) => entries.iter().for_each(|(_, item)| strings(item, out)),
            _ => {}
        }
    }
    let count = match &doc.root {
        Json::Array(items) => items.len(),
        Json::Object(entries) => entries.len(),
        _ => 0,
    };
    let mut out = Vec::new();
    strings(&doc.root, &mut out);
    (count, out)
}

/// A JSON literal's text: `"a\"b"` becomes `a"b`.
fn unquote(literal: &str) -> String {
    serde_json::from_str::<String>(literal).unwrap_or_else(|_| literal.trim_matches('"').to_owned())
}

/// A stored file described in JSON (Shrine, CarrierWave and Active Storage
/// keep one per attachment): its name and what is known about it.
#[derive(Debug, PartialEq)]
pub struct Attachment {
    pub filename: String,
    /// `image/jpeg · 103 × 102 · 5.9 KB · store`.
    pub details: String,
    pub image: bool,
}

/// The file `doc` describes, when it names one: a `filename` at its top
/// level or in its `metadata`.
pub fn attachment(doc: &Doc) -> Option<Attachment> {
    let Json::Object(entries) = &doc.root else {
        return None;
    };
    fn get<'d>(entries: &'d [(Box<str>, Json)], key: &str) -> Option<&'d Json> {
        entries
            .iter()
            .find(|(name, _)| &name[1..name.len() - 1] == key)
            .map(|(_, value)| value)
    }
    let metadata = match get(entries, "metadata") {
        Some(Json::Object(inner)) => inner.as_slice(),
        _ => entries.as_slice(),
    };
    let text = |value: Option<&Json>| match value {
        Some(Json::String(text)) => Some(unquote(text)),
        Some(Json::Number(number)) => Some(number.to_string()),
        _ => None,
    };
    let filename = text(get(metadata, "filename"))?;
    let mime = text(get(metadata, "mime_type").or_else(|| get(metadata, "content_type")));
    let mut details = Vec::new();
    if let Some(mime) = &mime {
        details.push(mime.clone());
    }
    if let (Some(width), Some(height)) =
        (text(get(metadata, "width")), text(get(metadata, "height")))
    {
        details.push(format!("{width} × {height}"));
    }
    if let Some(size) = text(get(metadata, "size")).and_then(|size| size.parse::<usize>().ok()) {
        details.push(crate::ui::format::human_size(size));
    }
    if let Some(storage) = text(get(entries, "storage")) {
        details.push(storage);
    }
    Some(Attachment {
        image: mime.is_some_and(|mime| mime.starts_with("image/")),
        filename,
        details: details.join(" · "),
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
pub fn show(
    ui: &mut Ui,
    id: Id,
    doc: &Doc,
    name: &str,
    locale: Locale,
    palette: &Palette,
    look: &theme::Look,
) {
    let folding: Folding = ui.data(|data| data.get_temp(id)).unwrap_or_default();
    let role = TextRole::pick(look, TextRole::Json, TextRole::OJson);
    let keys = key_color(look, palette);
    let mut view = View {
        locale,
        palette,
        look,
        role,
        keys,
        all: folding.all,
        open_all: doc.lines <= OPEN_LINES,
        rows: 0,
        folded: 0,
    };
    ui.scope(|ui| {
        // The role carries the design's line height.
        ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
        let slot = Slot {
            key: None,
            name,
            comma: false,
        };
        view.node(ui, 0, slot, &doc.root, id.with(folding.generation));
    });
    if view.rows >= MAX_ROWS {
        Text::one(
            look,
            role,
            &format!("…\n{}", gettext(locale, "(Copy gives the whole value.)")),
            palette.dim,
        )
        .layout(ui.ctx())
        .label(ui);
    }
    ui.data_mut(|data| data.insert_temp(id.with("folded"), view.folded));
}

/// Collapse all (or Expand all, once something is folded), right-aligned
/// at `right` on the line centred at `y`. Nothing for a flat document.
#[allow(clippy::too_many_arguments)] // the document, its place, and its look
pub fn fold_all_link(
    ui: &mut Ui,
    id: Id,
    doc: &Doc,
    right: f32,
    y: f32,
    locale: Locale,
    look: &theme::Look,
    palette: &Palette,
) {
    if doc.containers <= 1 {
        return;
    }
    let folded: usize = ui
        .data(|data| data.get_temp(id.with("folded")))
        .unwrap_or(0);
    let open = folded > 0;
    let label = if open {
        gettext(locale, "Expand all")
    } else {
        gettext(locale, "Collapse all")
    };
    let role = TextRole::FieldLabel;
    let width = role.width(ui.ctx(), look.faces, &label);
    let height = role.row_height(ui.ctx(), look.faces);
    let rect = egui::Rect::from_min_size(
        egui::pos2(right - width, y - height / 2.0),
        vec2(width, height),
    );
    let response = ui.interact(rect, id.with("fold-all"), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Link, true, label.as_ref()));
    let color = if response.hovered() {
        palette.accent_hover
    } else {
        palette.accent
    };
    Text::one(look, role, &label, color)
        .layout(ui.ctx())
        .paint(ui.painter(), rect.min);
    if response.clicked() {
        toggle_fold_all(ui, id);
    }
}

/// What Collapse all (or Expand all) does, for the `za` key.
pub fn toggle_fold_all(ui: &Ui, id: Id) {
    let folding: Folding = ui.data(|data| data.get_temp(id)).unwrap_or_default();
    let folded: usize = ui
        .data(|data| data.get_temp(id.with("folded")))
        .unwrap_or(0);
    let folding = Folding {
        generation: folding.generation.wrapping_add(1),
        all: Some(folded > 0),
    };
    ui.data_mut(|data| data.insert_temp(id, folding));
}

/// One line of the tree as it is built.
struct Line {
    text: Option<Text>,
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
    look: &'a theme::Look,
    role: TextRole,
    /// The colour of object keys.
    keys: Color32,
    /// Set by Expand all / Collapse all.
    all: Option<bool>,
    /// The document is short enough to open fully.
    open_all: bool,
    rows: usize,
    /// Folded objects and arrays drawn, so Expand all has work to do.
    folded: usize,
}

/// The colour a member's name is written in: the tree's, and the editor's
/// of a document.
pub fn key_color(look: &theme::Look, palette: &Palette) -> egui::Color32 {
    if look.terminal {
        palette.accent
    } else {
        palette.accent_hover
    }
}

impl View<'_> {
    fn node(&mut self, ui: &mut Ui, depth: usize, slot: Slot<'_>, value: &Json, id: Id) {
        if self.rows >= MAX_ROWS {
            return;
        }
        let Slot { key, name, comma } = slot;
        let mut job = self.line();
        if let Some(key) = key {
            self.push(&mut job, key, self.keys);
            self.push(&mut job, ": ", self.palette.text);
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
                    Json::Bool(true) => ("true", self.palette.orange),
                    Json::Bool(false) => ("false", self.palette.orange),
                    Json::Number(number) => (&**number, self.palette.orange),
                    Json::String(text) => (&**text, self.palette.success),
                    Json::Array(_) => ("[]", self.palette.text),
                    Json::Object(_) => ("{}", self.palette.text),
                };
                self.push(&mut job, text, color);
                if comma {
                    self.push(&mut job, ",", self.palette.text);
                }
                self.row(ui, depth, None, job);
                return;
            }
        };
        let open = ui
            .data(|data| data.get_temp(id))
            .unwrap_or(self.all.unwrap_or(depth == 0 || self.open_all));
        self.push(&mut job, open_bracket, self.palette.text);
        if !open {
            self.folded += 1;
            self.push(&mut job, &format!(" {count} {noun} "), self.palette.dim);
            self.push(&mut job, close_bracket, self.palette.text);
            if comma {
                self.push(&mut job, ",", self.palette.text);
            }
        }
        let verb = if open { "Collapse" } else { "Expand" };
        let toggle = format!("{} {name}", gettext(self.locale, verb));
        if self.row(ui, depth, Some((open, &toggle, id)), job) {
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
            let mut job = self.line();
            self.push(&mut job, close_bracket, self.palette.text);
            if comma {
                self.push(&mut job, ",", self.palette.text);
            }
            self.row(ui, depth, None, job);
        }
    }

    fn noun(&self, count: usize, one: &'static str, many: &'static str) -> String {
        let count = u32::try_from(count).unwrap_or(u32::MAX);
        ngettext(self.locale, one, many, count).into_owned()
    }

    /// The next line's text, empty.
    fn line(&self) -> Line {
        Line {
            text: Some(Text::new(self.look)),
        }
    }

    /// Appends `text` in `color` to `line`.
    fn push(&self, line: &mut Line, text: &str, color: Color32) {
        if let Some(sofar) = line.text.take() {
            line.text = Some(sofar.add(self.role, text, color));
        }
    }

    /// One line: indent, then `job`; a fold toggle (whether its node is
    /// open, its accessible name and its node's id) sits in the indent to
    /// its left, drawn while the pointer is over the line. Returns whether
    /// the toggle was clicked.
    fn row(
        &mut self,
        ui: &mut Ui,
        depth: usize,
        toggle: Option<(bool, &str, Id)>,
        job: Line,
    ) -> bool {
        self.rows += 1;
        let height = self.role.row_height(ui.ctx(), self.look.faces);
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            // Two characters per level, as the design's `pre` indents.
            let indent = depth as f32 * self.role.width(ui.ctx(), self.look.faces, "  ");
            ui.add_space(indent);
            let left = ui.cursor().left();
            // Long strings (paths, hashes) break anywhere, as the design
            // wraps them, not at the last space.
            let mut text = job.text.unwrap_or_else(|| Text::new(self.look));
            text.job_mut().wrap.max_width = ui.available_width().max(1.0);
            text.job_mut().wrap.break_anywhere = true;
            let laid = text.layout(ui.ctx());
            let label = ui.add(Label::new(laid.galley).selectable(true));
            match toggle {
                Some((open, text, id)) => {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(left - TOGGLE, label.rect.top()),
                        vec2(TOGGLE, height),
                    );
                    let near = ui.rect_contains_pointer(label.rect.union(rect));
                    self.toggle(ui, rect, id, open, text, near)
                }
                None => false,
            }
        })
        .inner
    }

    /// The fold toggle of the node `id`. The id is the node's, not the
    /// label's: two columns of a SQL result may share a name, and so the
    /// paths of their documents.
    fn toggle(
        &self,
        ui: &mut Ui,
        rect: egui::Rect,
        id: Id,
        open: bool,
        label: &str,
        shown: bool,
    ) -> bool {
        let response = ui.interact(rect, id.with("toggle"), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, label));
        if shown || response.hovered() || response.has_focus() {
            let icon = if open {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            };
            let tint = if response.hovered() {
                self.palette.text
            } else {
                self.palette.dim
            };
            icon.image(tint, 10.0).paint_at(
                ui,
                egui::Rect::from_center_size(rect.center(), vec2(10.0, 10.0)),
            );
        }
        response.on_hover_text(label).clicked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_characters_in_keys_and_strings_are_written_out() {
        let doc = parse("{\"a\u{202E}b\":\"Total: \u{202E}00.0001\u{200B}\"}").unwrap();
        assert_eq!(
            doc.root,
            Json::Object(vec![(
                "\"a<U+202E>b\"".into(),
                Json::String("\"Total: <U+202E>00.0001<U+200B>\"".into())
            )])
        );
        // The grid's summary of the document shows them written out too.
        assert_eq!(summary(&doc).1, vec!["Total: <U+202E>00.0001<U+200B>"]);
    }

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
    fn a_stored_file_is_read_from_its_metadata() {
        let doc = parse(
            r#"{"id": "a/b.jpg", "storage": "store", "metadata": {"size": 6024, "width": 103, "height": 102, "filename": "gold.jpg", "mime_type": "image/jpeg"}}"#,
        )
        .unwrap();
        assert_eq!(
            attachment(&doc),
            Some(Attachment {
                filename: "gold.jpg".into(),
                details: "image/jpeg · 103 × 102 · 5.9 KB · store".into(),
                image: true,
            })
        );
        assert_eq!(summary(&doc), (3, vec!["a/b.jpg".into(), "store".into()]));
        assert_eq!(attachment(&parse(r#"{"plan": "pro"}"#).unwrap()), None);
    }

    #[test]
    fn invalid_json_does_not_parse() {
        assert!(parse("not json").is_none());
        assert!(parse(r#"{"a":1"#).is_none());
    }

    #[test]
    fn a_text_column_holds_a_document_only_as_an_object_or_an_array() {
        let ctx = egui::Context::default();
        let holds =
            |kind, text: &str| document(&ctx, kind, &Value::Text(text.into()), TREE_MAX).is_some();
        for text in [r#"{"a":1}"#, " [1, 2]\n", "{}", "[]"] {
            assert!(holds(ValueKind::Text, text), "{text}");
        }
        // Text that is JSON only by accident, or only looks like it.
        for text in [
            "12",
            "true",
            "null",
            r#""quoted""#,
            "[draft] notes",
            "{a,b}",
            "",
        ] {
            assert!(!holds(ValueKind::Text, text), "{text}");
        }
        // A JSON column's scalars are still documents.
        assert!(holds(ValueKind::Json, "12"));
        // A PostgreSQL empty array and an inclusive range are not.
        assert!(!holds(ValueKind::Other, "{}"));
        assert!(!holds(ValueKind::Other, "[1.5,2.5]"));
        assert!(document(&ctx, ValueKind::Text, &Value::Null, TREE_MAX).is_none());
        assert!(document(&ctx, ValueKind::Text, &Value::Text("[1, 2]".into()), 5).is_none());
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
