//! The SQL editor's completion list: a small panel under the word being
//! typed. It never takes the keyboard: the editor keeps it, and `ui::keys`
//! routes the list's keys.

use std::borrow::Cow;
use std::ops::Range;

use egui::accesskit::Role;
use egui::{
    Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, WidgetInfo,
    WidgetType, pos2, vec2,
};

use crate::completion::{Candidate, Kind};
use crate::i18n::{Locale, gettext};
use crate::model::Completion;
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::widgets;

/// How many rows show at once; the list follows the highlight.
pub const VISIBLE: usize = 5;

/// The least room between a row's name and what is at its right.
const NAME_GAP: f32 = 8.0;

/// How much of a row the kind or type at its right may take from a name
/// that needs the room.
const DETAIL_SHARE: f32 = 0.4;

/// The list's measures in a look.
struct Shape {
    width: f32,
    /// The padding round the rows: across, then down.
    pad: egui::Vec2,
    row: f32,
    /// A row's own padding at each end.
    inset: f32,
    /// The kind letter's column and the gap after it; none in the terminal.
    letter: Option<(f32, f32)>,
    corner: u8,
    row_corner: u8,
    /// From the last row to the footer's hairline.
    footer_gap: f32,
    /// The footer's height under the hairline.
    footer: f32,
}

impl Shape {
    /// macOS: 330 pt wide, 5 pt of padding, 26 pt rows with a 20 pt letter
    /// column, corners of 8 and 5. The terminal: 360 pt wide, 4 pt above
    /// and below, 24 pt rows across the whole list, corners of 3.
    fn of(look: &Look) -> Self {
        if look.terminal {
            Self {
                width: 360.0,
                pad: vec2(0.0, 4.0),
                row: 24.0,
                inset: 10.0,
                letter: None,
                corner: 3,
                row_corner: 0,
                footer_gap: 3.0,
                footer: 22.0,
            }
        } else {
            Self {
                width: 330.0,
                pad: vec2(5.0, 5.0),
                row: 26.0,
                inset: 10.0,
                letter: Some((20.0, 8.0)),
                corner: 8,
                row_corner: 5,
                footer_gap: 4.0,
                footer: 24.0,
            }
        }
    }

    fn size(&self, rows: usize) -> egui::Vec2 {
        let height = 2.0 * self.pad.y + rows as f32 * self.row + self.footer_gap + self.footer;
        vec2(self.width, height)
    }
}

/// What the list is drawn with.
struct Style<'a> {
    shape: Shape,
    look: &'a Look,
    palette: &'a Palette,
    locale: Locale,
}

impl Style<'_> {
    /// The muted colour of a kind, a type and the footer.
    fn muted(&self) -> Color32 {
        if self.look.terminal {
            self.palette.dim
        } else {
            self.palette.secondary
        }
    }
}

/// Where a list of `size` goes: under `anchor` (the start of its word, as
/// tall as the line), above it when only there is room, and inside
/// `screen` across.
pub fn place(anchor: Rect, size: egui::Vec2, screen: Rect) -> Pos2 {
    let gap = 2.0;
    let below = anchor.bottom() + gap;
    let above = anchor.top() - gap - size.y;
    let top = if below + size.y > screen.bottom() && above >= screen.top() {
        above
    } else {
        below
    };
    let left = anchor
        .left()
        .min(screen.right() - size.x)
        .max(screen.left());
    pos2(left, top)
}

/// The first row in view: the view that started at `first`, moved as
/// little as it takes to show `selected` among `rows` rows.
pub fn window(first: usize, selected: usize, rows: usize) -> usize {
    first
        .min(selected)
        .max((selected + 1).saturating_sub(VISIBLE))
        .min(rows.saturating_sub(VISIBLE))
}

/// How a row `span` points wide is shared by its name and the kind or
/// type at its right, `name` and `detail` points wide when whole: the room
/// of each. The name comes first: the detail has what the name leaves,
/// and when that is less than it needs, no more than its share of the
/// row. Neither room is ever negative.
fn rooms(span: f32, name: f32, detail: f32) -> (f32, f32) {
    let span = span.max(0.0);
    let left_over = (span - NAME_GAP - name).max(0.0);
    let detail_room = detail.min(left_over.max(span * DETAIL_SHARE));
    let name_room = (span - NAME_GAP - detail_room).max(0.0);
    (name_room, detail_room)
}

/// A row's name as it is drawn: `shown` is `label` whole, or the start of
/// it and the "…" it was cut with. Returns what comes before the part
/// that `matched`, that part, what comes after it, and the "…": a cut
/// name keeps as much of its match as is left of it. A range that does
/// not fit the label is drawn as no match.
fn name_pieces<'a>(shown: &'a str, label: &str, matched: &Range<usize>) -> [&'a str; 4] {
    let kept = if shown == label {
        shown
    } else {
        shown.strip_suffix('…').unwrap_or(shown)
    };
    let tail = &shown[kept.len()..];
    let unmatched = [kept, "", "", tail];
    if label.get(matched.clone()).is_none() {
        return unmatched;
    }
    // What is kept is a start of the label: the ends of the match that
    // are inside it are between its characters too.
    let end = matched.end.min(kept.len());
    let start = matched.start.min(end);
    match (kept.get(..start), kept.get(start..end), kept.get(end..)) {
        (Some(before), Some(found), Some(after)) => [before, found, after, tail],
        _ => unmatched,
    }
}

/// A kind in words: what a row shows at its right, and how it is named.
fn kind_name(kind: Kind, locale: Locale) -> Cow<'static, str> {
    match kind {
        Kind::Keyword => gettext(locale, "keyword"),
        Kind::Schema => gettext(locale, "schema"),
        Kind::Table => gettext(locale, "table"),
        Kind::View => gettext(locale, "view"),
        Kind::MaterializedView => gettext(locale, "materialized view"),
        Kind::Column => gettext(locale, "column"),
    }
}

/// The letter macOS shows before a row.
fn kind_letter(kind: Kind) -> &'static str {
    match kind {
        Kind::Keyword => "K",
        Kind::Schema => "S",
        Kind::Table => "T",
        Kind::View | Kind::MaterializedView => "V",
        Kind::Column => "C",
    }
}

/// What a row is called for a screen reader: "books, table", and a column
/// with its type, "title, column, text".
pub fn row_name(candidate: &Candidate, locale: Locale) -> String {
    let kind = kind_name(candidate.kind, locale);
    if candidate.detail.is_empty() {
        format!("{}, {kind}", candidate.label)
    } else {
        format!("{}, {kind}, {}", candidate.label, candidate.detail)
    }
}

/// What the pointer did to the list this frame.
pub struct Shown {
    /// The list is pressed or was clicked, on a row or beside one, with
    /// any button: the press landed outside the editor's field, which
    /// must keep the keyboard.
    pub pressed: bool,
    /// The row picked: by a primary click, or by a screen reader's.
    pub picked: Option<usize>,
}

/// Draws `list` for the editor `editor`, at `anchor`: the start of its
/// word on screen, as tall as the line.
pub fn show(
    ui: &Ui,
    list: &Completion,
    anchor: Rect,
    editor: Id,
    look: &Look,
    palette: &Palette,
    locale: Locale,
) -> Shown {
    let style = Style {
        shape: Shape::of(look),
        look,
        palette,
        locale,
    };
    let shape = &style.shape;
    let total = list.candidates.len();
    let rows = total.min(VISIBLE);
    let size = shape.size(rows);
    let first = kept(ui.ctx(), editor, list).map_or(0, |kept| kept.first);
    let first = window(first, list.selected, total);
    let drawn = Kept {
        list: list.serial,
        first,
    };
    ui.data_mut(|data| data.insert_temp(kept_id(editor), drawn));
    let mut shown = Shown {
        pressed: false,
        picked: None,
    };
    // One area for the lists of all editors: only the editor on screen has
    // a list. egui sizes a new area in a pass that paints nothing, and
    // keeps every area's place for good: with an area each, the first list
    // of every editor would show a frame late and leave its place behind.
    let id = Id::new("sql-completions");
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        // Whole at once: a list that fades in lags the typing.
        .fade_in(false)
        // `place` keeps the list inside the screen, by the size it has this
        // frame. egui would do it by the size the area had a frame ago,
        // and push a list that shrank over the line of its word.
        .constrain(false)
        .fixed_pos(place(anchor, size, ui.ctx().content_rect()))
        .show(ui.ctx(), |ui| {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            // A press between the rows or on the footer picks nothing, but
            // it is the list's: the editor keeps the keyboard. Clicked and
            // never focused, as the rows are.
            let whole = ui.interact(rect, id.with("list"), Sense::CLICK);
            whole.widget_info(|| {
                WidgetInfo::labeled(WidgetType::Other, true, gettext(locale, "Completions"))
            });
            ui.ctx().accesskit_node_builder(whole.id, |node| {
                node.set_role(Role::ListBox);
                node.set_size_of_set(total);
            });
            // Any button counts: the field gives the keyboard up on a
            // click of any of them, and only a primary click is a row's.
            let clicked_over = ui.input(|input| input.pointer.any_click());
            let clicked_over = clicked_over && ui.rect_contains_pointer(rect);
            shown.pressed = whole.is_pointer_button_down_on() || whole.clicked() || clicked_over;
            let border = panel(ui, rect, shape.corner, look, palette);
            let inner = rect.shrink2(shape.pad);
            // The rows are drawn in a `Ui` of their own whose accessibility
            // parent is the list, so their nodes hang under the list's:
            // through the generic container egui makes for a `Ui`, which
            // AccessKit leaves out of what a screen reader is told.
            let options = UiBuilder::new()
                .max_rect(inner)
                .accessibility_parent(whole.id);
            let options = ui.new_child(options);
            for (slot, index) in (first..first + rows).enumerate() {
                let candidate = &list.candidates[index];
                let top = inner.top() + slot as f32 * shape.row;
                let row =
                    Rect::from_min_size(pos2(inner.left(), top), vec2(inner.width(), shape.row));
                let row_id = id.with(("row", index));
                // Clicked, never focused: a row that could take the
                // keyboard would take it from the editor, and close the
                // list, when a screen reader asks to focus it.
                let response = options.interact(row, row_id, Sense::CLICK);
                let selected = index == list.selected;
                response.widget_info(|| {
                    WidgetInfo::labeled(WidgetType::Other, true, row_name(candidate, locale))
                });
                ui.ctx().accesskit_node_builder(row_id, |node| {
                    node.set_role(Role::ListBoxOption);
                    node.set_selected(selected);
                    // Counted from zero, among the rows the keys reach.
                    node.set_position_in_set(index);
                    node.set_size_of_set(total);
                });
                let corner = CornerRadius::same(shape.row_corner);
                if selected {
                    ui.painter().rect_filled(row, corner, palette.selection);
                    // The keyboard stays in the editor: its node says
                    // which row is the current one, and a screen reader
                    // takes that row for the focused object. A list that
                    // opened by itself says nothing until the user moves
                    // in it, or a reader would leave the editor on nearly
                    // every word typed.
                    if list.manual || list.moved {
                        ui.ctx().accesskit_node_builder(editor, |node| {
                            node.set_active_descendant(row_id.accesskit_id());
                        });
                    }
                } else if response.hovered() {
                    ui.painter().rect_filled(row, corner, palette.surface_hover);
                }
                paint_row(ui, row, candidate, &style);
                shown.pressed |= response.is_pointer_button_down_on() || response.clicked();
                if response.clicked() {
                    shown.picked = Some(index);
                }
            }
            footer(ui, rect, rows, list, &style);
            ui.painter().add(border);
        });
    shown
}

/// What egui's memory keeps for an editor's open list between frames: it
/// is there once the list drew.
#[derive(Clone, Copy)]
struct Kept {
    /// The serial of the list it is kept for.
    list: u64,
    /// The first row in view.
    first: usize,
}

/// The id it is kept under for the editor `editor`.
fn kept_id(editor: Id) -> Id {
    editor.with("completions")
}

/// What is kept for `list`, the open list of `editor`. Nothing when the
/// list has not drawn, even if it took the place of one that had with no
/// frame between them: what that one left is not this one's.
fn kept(ctx: &egui::Context, editor: Id, list: &Completion) -> Option<Kept> {
    let kept: Option<Kept> = ctx.data(|data| data.get_temp(kept_id(editor)));
    kept.filter(|kept| kept.list == list.serial)
}

/// Whether `list`, the open list of `editor`, drew since it opened. One
/// that did not may be waiting for the editor to scroll its caret into
/// view.
pub fn was_shown(ctx: &egui::Context, editor: Id, list: &Completion) -> bool {
    kept(ctx, editor, list).is_some()
}

/// Drops what egui's memory keeps for the list of `editor`: it closed, or
/// the editor did.
pub fn forget(ctx: &egui::Context, editor: Id) {
    ctx.data_mut(|data| data.remove::<Kept>(kept_id(editor)));
}

/// Whether egui's memory keeps anything for the list of `editor`.
#[cfg(test)]
pub fn remembered(ctx: &egui::Context, editor: Id) -> bool {
    ctx.data(|data| data.get_temp::<Kept>(kept_id(editor)).is_some())
}

/// The panel behind the rows: raised and rounded on macOS, the terminal's
/// darker surface with an accent border. Returns the border, which goes
/// over the rows: the terminal's reach the panel's edges. A cell's large
/// editor stands on the same panel.
pub(crate) fn panel(
    ui: &Ui,
    rect: Rect,
    corner: u8,
    look: &Look,
    palette: &Palette,
) -> egui::Shape {
    let corner = CornerRadius::same(corner);
    let (offset, blur, fill, border) = if look.terminal {
        ([0, 12], 32, palette.panel, palette.accent)
    } else {
        ([0, 8], 24, widgets::raised_fill(palette), palette.border)
    };
    let shadow = egui::epaint::Shadow {
        offset,
        blur,
        spread: 0,
        color: palette.shadow,
    };
    let painter = ui.painter();
    painter.add(shadow.as_shape(rect, corner));
    painter.rect_filled(rect, corner, fill);
    let border = Stroke::new(1.0, border);
    egui::Shape::rect_stroke(rect, corner, border, StrokeKind::Inside)
}

/// One row: on macOS the kind's letter, then the name with what matched
/// emphasised (heavier on macOS, in the accent in the terminal), and at
/// the right the kind in words or a column's type. What does not fit ends
/// in "…": the name has the row first (see [`rooms`]).
fn paint_row(ui: &Ui, row: Rect, candidate: &Candidate, style: &Style<'_>) {
    let Style {
        shape,
        look,
        locale,
        ..
    } = style;
    let y = row.center().y;
    let muted = style.muted();
    let mut x = row.left() + shape.inset;
    if let Some((column, gap)) = shape.letter {
        Text::one(look, TextRole::TagSmall, kind_letter(candidate.kind), muted)
            .layout(ui.ctx())
            .paint_left(ui.painter(), x, y);
        x += column + gap;
    }
    let right = row.right() - shape.inset;
    let label = candidate.label.as_str();
    let name = |shown: &str| name_text(shown, candidate, style).layout(ui.ctx());
    let whole = name(label);
    let detail = if candidate.detail.is_empty() {
        kind_name(candidate.kind, *locale)
    } else {
        Cow::Borrowed(candidate.detail.as_str())
    };
    let detail_role = TextRole::pick(look, TextRole::ColumnType, TextRole::OCaption);
    let detail_width = |text: &str| detail_role.width(ui.ctx(), look.faces, text);
    let (name_room, detail_room) = rooms(right - x, whole.width(), detail_width(&detail));
    let detail = crate::ui::grid::ellipsize(&detail, detail_room, false, detail_width);
    Text::one(look, detail_role, &detail, muted)
        .layout(ui.ctx())
        .paint_right(ui.painter(), right, y);
    let name = if whole.width() <= name_room {
        whole
    } else {
        let width = |shown: &str| name(shown).width();
        name(&crate::ui::grid::ellipsize(label, name_room, false, width))
    };
    // The "…" alone may be wider than what room is left: the name stops
    // before the kind, not under it.
    let room = Rect::from_min_max(pos2(x, row.top()), pos2(x + name_room, row.bottom()));
    name.paint_left(&ui.painter().with_clip_rect(room), x, y);
}

/// The name of `candidate`'s row as text: `shown` is its label, or the
/// start of it and a "…" (see [`name_pieces`]).
fn name_text(shown: &str, candidate: &Candidate, style: &Style<'_>) -> Text {
    let (look, palette) = (style.look, style.palette);
    let (plain, hit, hit_color) = if look.terminal {
        (TextRole::OCode, TextRole::OCode, palette.accent)
    } else {
        (
            TextRole::CompletionName,
            TextRole::CompletionMatch,
            palette.text,
        )
    };
    let [before, found, after, tail] = name_pieces(shown, &candidate.label, &candidate.matched);
    Text::new(look)
        .add(plain, before, palette.text)
        .add(hit, found, hit_color)
        .add(plain, after, palette.text)
        .add(plain, tail, palette.text)
}

/// Under a hairline: the keys, while there is a row for them to act on,
/// and how many matches are out of sight, or that names are on their way.
fn footer(ui: &Ui, rect: Rect, rows: usize, list: &Completion, style: &Style<'_>) {
    let Style {
        shape,
        look,
        palette,
        locale,
    } = style;
    let locale = *locale;
    let top = rect.top() + shape.pad.y + rows as f32 * shape.row + shape.footer_gap;
    let across = egui::Rangef::new(rect.left() + shape.pad.x, rect.right() - shape.pad.x);
    let line = if look.terminal {
        palette.outline
    } else {
        palette.border
    };
    widgets::hline(ui, across, top, line);
    let y = top + shape.footer / 2.0;
    let muted = style.muted();
    let has_row = rows > 0;
    let hidden = (list.candidates.len() + list.more).saturating_sub(rows);
    let count = if list.loading {
        Some(gettext(locale, "Loading").into_owned())
    } else if hidden > 0 {
        // A placeholder: a translation puts the number where it reads.
        Some(gettext(locale, "{count} more").replace("{count}", &hidden.to_string()))
    } else {
        None
    };
    let left = rect.left() + shape.pad.x + shape.inset;
    let right = rect.right() - shape.pad.x - shape.inset;
    if look.terminal {
        let role = TextRole::OCaption;
        let mut text = Text::new(look);
        if has_row {
            let complete = format!(" {} · ", gettext(locale, "complete"));
            let moves = format!(" {}", gettext(locale, "move"));
            use crate::keymap::Command;
            let insert = crate::ui::keys::written(ui.ctx(), look, Command::InsertCompletion);
            let rows = [Command::NextCompletion, Command::PreviousCompletion];
            let moving = crate::ui::keys::written_together(ui.ctx(), look, &rows);
            text = text
                .add(role, &insert, palette.text)
                .add(role, &complete, muted)
                .add(role, &moving, palette.text)
                .add(role, &moves, muted);
        }
        if let Some(count) = &count {
            let lead = if has_row { " · " } else { "" };
            text = text.add(role, &format!("{lead}{}", count.to_lowercase()), muted);
        }
        if !text.is_empty() {
            widgets::paint_text(ui, left, y, text);
        }
    } else {
        let role = TextRole::Shortcut;
        if has_row {
            // Every key that inserts, as the look writes them.
            let insert = crate::keymap::Command::InsertCompletion;
            let keys = crate::ui::keys::written_all(ui.ctx(), look, insert);
            let keys = format!("{keys} {}", gettext(locale, "insert"));
            widgets::paint_text(ui, left, y, Text::one(look, role, &keys, muted));
        }
        if let Some(count) = &count {
            widgets::paint_text_right(ui, right, y, Text::one(look, role, count, muted));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_goes_under_its_word_or_above_it_when_there_is_no_room() {
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0));
        let size = vec2(330.0, 160.0);
        let word = |x: f32, y: f32| Rect::from_min_max(pos2(x, y), pos2(x, y + 22.0));
        let placed =
            |word: Rect, screen: Rect| Rect::from_min_size(place(word, size, screen), size);
        // Room below: under the word's line, from where the word starts.
        let high = word(100.0, 200.0);
        let list = placed(high, screen);
        assert!(list.top() >= high.bottom(), "{list:?}");
        assert_eq!(list.left(), high.left());
        assert!(screen.contains_rect(list));
        // No room below: above the line.
        let low = word(100.0, 500.0);
        let list = placed(low, screen);
        assert!(list.bottom() <= low.top(), "{list:?}");
        assert_eq!(list.left(), low.left());
        assert!(screen.contains_rect(list));
        // Never far from its line, either way.
        assert!(placed(high, screen).top() - high.bottom() < high.height());
        assert!(low.top() - placed(low, screen).bottom() < low.height());
        // Too far right, or left of the screen: pushed back inside across,
        // and still clear of the line.
        for far in [word(700.0, 200.0), word(-50.0, 200.0)] {
            let list = placed(far, screen);
            assert!(screen.x_range().contains(list.left()), "{list:?}");
            assert!(screen.x_range().contains(list.right()), "{list:?}");
            assert!(list.top() >= far.bottom(), "{list:?}");
        }
        // No room either way: below, as far as it goes. It never covers
        // the line it completes.
        let small = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 200.0));
        let middle = word(100.0, 90.0);
        assert!(placed(middle, small).top() >= middle.bottom());
    }

    #[test]
    fn the_rows_in_view_follow_the_highlight() {
        // Fewer rows than fit: all of them.
        assert_eq!(window(0, 2, 3), 0);
        // Moving down past the last row in view scrolls by one.
        assert_eq!(window(0, 4, 20), 0);
        assert_eq!(window(0, 5, 20), 1);
        // Moving back up inside the view scrolls nothing.
        assert_eq!(window(1, 3, 20), 1);
        // Above the view: the highlight is the first row.
        assert_eq!(window(6, 2, 20), 2);
        // The list shrank under the view.
        assert_eq!(window(15, 0, 4), 0);
        assert_eq!(window(15, 7, 8), 3);
    }

    #[test]
    fn a_row_is_named_by_its_label_kind_and_type() {
        let locale = Locale::English;
        let row = |kind, detail: &str| Candidate {
            kind,
            label: "title".into(),
            insert: "title".into(),
            matched: 0..0,
            detail: detail.into(),
        };
        assert_eq!(row_name(&row(Kind::Table, ""), locale), "title, table");
        assert_eq!(
            row_name(&row(Kind::Column, "text"), locale),
            "title, column, text"
        );
        assert_eq!(
            row_name(&row(Kind::MaterializedView, ""), locale),
            "title, materialized view"
        );
    }

    #[test]
    fn a_rows_name_has_the_row_before_what_is_at_its_right() {
        let span = 300.0;
        // Both fit: each has what it needs, with the gap between them.
        let (name, detail) = rooms(span, 100.0, 50.0);
        assert_eq!(detail, 50.0);
        assert!(name >= 100.0);
        assert!(name + detail < span);
        // A long name beside a short kind: the kind stays whole, and the
        // name has all the rest.
        let (name, detail) = rooms(span, 900.0, 50.0);
        assert_eq!(detail, 50.0);
        assert!(name > 200.0 && name + detail < span);
        // A long type beside a short name takes what the name leaves.
        let (name, detail) = rooms(span, 60.0, 900.0);
        assert!(name >= 60.0);
        assert!(detail > span / 2.0 && name + detail < span);
        // Both long: the type is held to its share, and the name has more.
        let (name, detail) = rooms(span, 900.0, 900.0);
        assert!(detail < name, "{name} and {detail}");
        assert!(detail > 0.0 && name + detail < span);
        // No room at all: neither is negative, whatever the row.
        for span in [0.0, 4.0, -20.0] {
            let (name, detail) = rooms(span, 900.0, 900.0);
            assert!(name >= 0.0 && detail >= 0.0, "{span}: {name} and {detail}");
        }
    }

    #[test]
    fn a_cut_name_keeps_what_is_left_of_its_match() {
        // Whole.
        assert_eq!(
            name_pieces("users", "users", &(0..2)),
            ["", "us", "ers", ""]
        );
        assert_eq!(
            name_pieces("users", "users", &(2..5)),
            ["us", "ers", "", ""]
        );
        // Cut after the match, inside it, and before it.
        assert_eq!(
            name_pieces("user…", "users_archive", &(0..2)),
            ["", "us", "er", "…"]
        );
        assert_eq!(
            name_pieces("use…", "users_archive", &(1..5)),
            ["u", "se", "", "…"]
        );
        assert_eq!(
            name_pieces("us…", "users_archive", &(6..9)),
            ["us", "", "", "…"]
        );
        // Nothing left but the "…".
        assert_eq!(name_pieces("…", "users", &(0..2)), ["", "", "", "…"]);
        // Characters of more than one byte.
        assert_eq!(name_pieces("żó…", "żółw", &(2..6)), ["ż", "ó", "", "…"]);
        // A range that does not fit the label: no match.
        assert_eq!(
            name_pieces("users", "users", &(3..9)),
            ["users", "", "", ""]
        );
        assert_eq!(name_pieces("żółw", "żółw", &(1..3)), ["żółw", "", "", ""]);
        // A name that itself ends in "…" is whole.
        assert_eq!(name_pieces("et…", "et…", &(0..2)), ["", "et", "…", ""]);
    }
}
