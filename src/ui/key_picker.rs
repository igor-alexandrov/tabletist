//! The list of a foreign key cell's picker: the rows of the table the key
//! points at, under the cell's field. It never takes the keyboard: the
//! field keeps it, and `ui::keys` routes the list's keys. Drawn as
//! `ui::sql_complete` draws the completion list.

use std::borrow::Cow;

use egui::accesskit::Role;
use egui::{CornerRadius, Id, Rect, Sense, Ui, UiBuilder, WidgetInfo, WidgetType, pos2, vec2};
use tabletist_db::{LookupRow, Value};

use crate::i18n::{Locale, gettext, ngettext};
use crate::key_picker::{Columns, LIMIT, Picker, VISIBLE, found_in};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::format::{display_safe, plain_text};
use crate::ui::{grid, sql_complete, widgets};

/// What the pointer did to the list this frame.
#[derive(Default)]
pub struct Shown {
    /// The list is pressed or was clicked, on an option or beside one:
    /// the press landed outside the field, which must keep the keyboard.
    pub pressed: bool,
    /// The option picked: by a primary click, or by a screen reader's.
    pub picked: Option<usize>,
    /// Set NULL was clicked.
    pub null: bool,
}

/// What the list is of.
pub struct Names<'a> {
    /// The table that is searched.
    pub table: &'a str,
    /// The cell's column takes NULL: the foot offers Set NULL.
    pub nullable: bool,
}

/// The list's measures in a look.
struct Shape {
    /// The padding round its lines.
    pad: egui::Vec2,
    /// An option's height: two lines of text with a label, one without.
    option: f32,
    /// A line of words: the head, the foot, a state.
    line: f32,
    inset: f32,
    corner: u8,
    row_corner: u8,
}

impl Shape {
    fn of(look: &Look, labelled: bool) -> Self {
        if look.terminal {
            Self {
                pad: vec2(0.0, 4.0),
                option: 24.0,
                line: 22.0,
                inset: 10.0,
                corner: 3,
                row_corner: 0,
            }
        } else {
            Self {
                pad: vec2(4.0, 4.0),
                option: if labelled { 38.0 } else { 26.0 },
                line: 24.0,
                inset: 8.0,
                corner: 8,
                row_corner: 5,
            }
        }
    }
}

/// The colour of what is said beside the options.
fn muted(look: &Look, palette: &Palette) -> egui::Color32 {
    if look.terminal {
        palette.dim
    } else {
        palette.secondary
    }
}

/// The one line the list says in place of options, or under them.
/// `hidden` is how many options the list holds that are not drawn.
fn state_line(
    picker: &Picker,
    typed: &str,
    table: &str,
    hidden: usize,
    locale: Locale,
) -> Option<(String, bool)> {
    let table = display_safe(table);
    if let Some(error) = &picker.failed {
        return Some((display_safe(&error.to_string()).into_owned(), true));
    }
    if picker.listed.is_none() {
        let text = gettext(locale, "Searching {table}…").replace("{table}", &table);
        return Some((text, false));
    }
    if picker.options.is_empty() {
        let text = picker.text(typed);
        let said = if text.is_empty() {
            gettext(locale, "No {table}").replace("{table}", &table)
        } else {
            gettext(locale, "No {table} match \"{text}\"")
                .replace("{table}", &table)
                .replace("{text}", &display_safe(text))
        };
        return Some((said, false));
    }
    if picker.more {
        let text = gettext(locale, "First {count} shown. Keep typing to narrow.")
            .replace("{count}", &LIMIT.to_string());
        return Some((text, false));
    }
    // The list goes on past the rows drawn: the arrows reach the rest.
    (hidden > 0).then(|| {
        let text = gettext(locale, "{count} more").replace("{count}", &hidden.to_string());
        (text, false)
    })
}

/// What an answer is announced as: "2 matches in publishers". The
/// table's name is not made to agree with the count: it is a name.
fn count_line(picker: &Picker, table: &str, locale: Locale) -> Option<String> {
    picker.listed.as_ref()?;
    let table = display_safe(table);
    let count = picker.options.len();
    Some(if count == 0 {
        gettext(locale, "No matches in {table}").replace("{table}", &table)
    } else if picker.more {
        gettext(locale, "{count} or more matches in {table}")
            .replace("{count}", &LIMIT.to_string())
            .replace("{table}", &table)
    } else {
        let plural = u32::try_from(count).unwrap_or(u32::MAX);
        ngettext(
            locale,
            "{count} match in {table}",
            "{count} matches in {table}",
            plural,
        )
        .replace("{count}", &count.to_string())
        .replace("{table}", &table)
    })
}

/// The head line: what is listed, and the keys.
fn head_line(picker: &Picker, typed: &str, table: &str, locale: Locale) -> String {
    let table = display_safe(table);
    let keys = gettext(locale, "↑↓ choose · ↩ set");
    let text = picker.text(typed);
    if text.is_empty() {
        format!("{table} · {keys}")
    } else {
        let listed = gettext(locale, "{table} matching \"{text}\"")
            .replace("{table}", &table)
            .replace("{text}", &display_safe(text));
        format!("{listed} · {keys}")
    }
}

/// A label as the text it is written in: a text is borrowed, any other
/// value is written out.
fn label_text(label: &Value) -> Cow<'_, str> {
    match label {
        Value::Text(text) => Cow::Borrowed(text),
        other => Cow::Owned(plain_text(other)),
    }
}

/// The text an option's label is drawn as, where it has one: a NULL one
/// is the word `NULL`.
fn label_shown(option: &LookupRow) -> Option<Cow<'_, str>> {
    match option.label.as_ref()? {
        Value::Null => Some(Cow::Borrowed("NULL")),
        value => match label_text(value) {
            Cow::Borrowed(text) => Some(display_safe(text)),
            Cow::Owned(text) => Some(Cow::Owned(display_safe(&text).into_owned())),
        },
    }
}

/// The gap between a label and its key on the terminal's one line, and
/// before its "current" mark.
const LINE_GAP: f32 = 8.0;

/// The widest line that the terminal look draws for `options` needs:
/// inset, label, gap, key, the mark of the row the cell points at, inset.
fn terminal_need(
    ui: &Ui,
    options: &[LookupRow],
    picker: &Picker,
    shape: &Shape,
    (look, locale): (&Look, Locale),
) -> f32 {
    let mark_role = TextRole::pick(look, TextRole::Shortcut, TextRole::OCaption);
    let word = |role: TextRole, text: &str| role.width(ui.ctx(), look.faces, text);
    let mark = format!("· {}", gettext(locale, "current"));
    let mark = word(mark_role, &mark) + LINE_GAP;
    options
        .iter()
        .map(|option| {
            let key = plain_text(&option.key);
            let key = word(TextRole::OCode, &display_safe(&key));
            let label =
                label_shown(option).map_or(0.0, |label| word(TextRole::OCode, &label) + LINE_GAP);
            let current = if picker.is_current(&option.key) {
                mark
            } else {
                0.0
            };
            2.0 * shape.inset + label + key + current
        })
        .fold(0.0, f32::max)
}

/// What an option is called for a screen reader: its label and its key,
/// and that it is the row the cell points at. An option whose label is
/// NULL or empty is named by its key alone.
fn option_name(option: &LookupRow, current: bool, locale: Locale) -> String {
    let key = plain_text(&option.key);
    let key = display_safe(&key);
    let mut name = match &option.label {
        Some(label) if !matches!(label, Value::Null) => {
            let label = label_text(label);
            if label.is_empty() {
                key.into_owned()
            } else {
                format!("{}, {key}", display_safe(&label))
            }
        }
        _ => key.into_owned(),
    };
    if current {
        name.push_str(", ");
        name.push_str(&gettext(locale, "current"));
    }
    name
}

/// Paints one line of words at (`left`, `y`), cut with "…" to `room` and
/// clipped to `clip`, and gives a screen reader the whole of it.
fn say(
    ui: &Ui,
    (left, y): (f32, f32),
    room: f32,
    (whole, role, color): (&str, TextRole, egui::Color32),
    clip: Rect,
    look: &Look,
) {
    let width = |text: &str| role.width(ui.ctx(), look.faces, text);
    let cut = grid::ellipsize(whole, room.max(0.0), false, width);
    let laid = Text::one(look, role, &cut, color).layout(ui.ctx());
    let wide = laid.paint_left(&ui.painter().with_clip_rect(clip), left, y);
    let place = Rect::from_min_size(pos2(left, y - 8.0), vec2(wide.max(1.0), 16.0));
    widgets::announce(ui, place, whole);
}

/// What a screen reader is told of a pick: "publisher_id set to
/// 9100000000000000004, Harbor Press".
pub fn picked_text(column: &str, picked: &crate::edit::Picked, locale: Locale) -> String {
    let column = display_safe(column);
    let key = display_safe(&picked.key);
    match &picked.label {
        // A placeholder each: a translation puts them where they read.
        Some(label) => gettext(locale, "{column} set to {key}, {label}")
            .replace("{column}", &column)
            .replace("{key}", &key)
            .replace("{label}", &display_safe(label)),
        None => gettext(locale, "{column} set to {key}")
            .replace("{column}", &column)
            .replace("{key}", &key),
    }
}

/// What a foreign key cell whose value names no row says: "No publisher
/// with id 9100000000000000099". The table's name is made singular as the
/// row panel's "Open publisher" makes it. In the terminal look the app's
/// own words are the look's lower case, and the table, the column and the
/// value are as they are: the cell's hint, the row panel and the line under
/// the grid all say it this way.
pub fn doubt_text(found: &crate::key_picker::Unmatched, look: &Look, locale: Locale) -> String {
    let table = display_safe(&found.table);
    // A placeholder each: a translation puts them where they read, and
    // what they stand for is not the app's to lower.
    look.label(&gettext(locale, "No {row} with {key} {value}"))
        .replace("{row}", crate::ui::row_panel::singular(&table))
        .replace("{key}", &display_safe(&found.key))
        .replace("{value}", &display_safe(&found.value))
}

/// The warnings that stand on a table's tab, in the page's order with new
/// rows last. None while the tab has no page, and none while a save runs:
/// every view of a warning gives way together then, as the cell does, and
/// a save that fails shows the failure instead.
pub fn unmatched(
    workspace: &crate::model::Workspace,
    object: &crate::model::ObjectTab,
) -> Vec<crate::key_picker::Unmatched> {
    let Some(table) = crate::edit::Table::of(workspace, object) else {
        return Vec::new();
    };
    if table.saving {
        return Vec::new();
    }
    crate::key_picker::unmatched(&table, &object.object, &object.edits)
}

/// The warnings that stand on a table's tab, worded, by cell.
pub fn doubts(
    workspace: &crate::model::Workspace,
    object: &crate::model::ObjectTab,
    look: &Look,
    locale: Locale,
) -> std::collections::BTreeMap<(usize, usize), String> {
    unmatched(workspace, object)
        .iter()
        .map(|found| (found.at, doubt_text(found, look, locale)))
        .collect()
}

/// The size of the check that marks the row the cell points at.
const CHECK: f32 = 12.0;

/// The part of `cell` the list hangs from, `body` being where the grid's
/// rows show: none when the cell is out of sight or its top edge is under
/// the header, else the part that shows.
pub fn hung_from(cell: Rect, body: Rect) -> Option<Rect> {
    let seen = cell.intersect(body);
    let clear = cell.top() >= body.top() - 0.5;
    (clear && seen.width() > 0.0 && seen.height() > 0.0).then_some(seen)
}

/// The widest the terminal look's list grows to fit its options.
const TERMINAL_CAP: f32 = 560.0;

/// The gap between the cell and its list.
const GAP: f32 = 2.0;

/// Where the list goes, and how many of its options it can hold.
struct Place {
    /// Its top left corner.
    pos: egui::Pos2,
    /// The options drawn: as many as the room on the side it took holds,
    /// one at least.
    rows: usize,
}

/// Under `anchor` when the list fits there, else on the side of the cell
/// with more room inside `screen`, its options cut to the room it has.
/// `chrome` is the list's height without its options, `option` an
/// option's, `wanted` how many it would show. Across, the list keeps to
/// the screen.
fn place(
    anchor: Rect,
    size: egui::Vec2,
    (chrome, option, wanted): (f32, f32, usize),
    screen: Rect,
) -> Place {
    let below = screen.bottom() - (anchor.bottom() + GAP);
    let above = anchor.top() - GAP - screen.top();
    let under = chrome + wanted as f32 * option <= below || below >= above;
    let room = if under { below } else { above };
    let rows = if wanted == 0 {
        0
    } else {
        let fits = ((room - chrome) / option).floor().max(1.0) as usize;
        wanted.min(fits)
    };
    let height = chrome + rows as f32 * option;
    let top = if under {
        anchor.bottom() + GAP
    } else {
        anchor.top() - GAP - height
    };
    let left = anchor
        .left()
        .min(screen.right() - size.x)
        .max(screen.left());
    Place {
        pos: pos2(left, top),
        rows,
    }
}

/// Draws the list of `picker` at `anchor`, the cell its field `field` is
/// on, while the field holds `typed`.
pub fn show(
    ui: &Ui,
    picker: &Picker,
    typed: &str,
    anchor: Rect,
    field: Id,
    names: &Names<'_>,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Shown {
    let labelled = matches!(&picker.label, Columns::Known(Some(_)));
    let shape = Shape::of(look, labelled);
    let total = picker.options.len();
    // Whether there is a state line is decided before the list is placed,
    // from the options there are: a list of more than `VISIBLE` says that
    // it goes on. The rows that really fit are known only after it is
    // placed, and the line is worded with them. A list that planned no
    // line and is cut to fewer rows than `VISIBLE` by a short window
    // draws none for this frame, rather than being placed again.
    let planned = state_line(
        picker,
        typed,
        names.table,
        total.saturating_sub(VISIBLE),
        locale,
    );
    let head = !look.terminal;
    let foot = !look.terminal;
    let lines = usize::from(head) + usize::from(foot) + usize::from(planned.is_some());
    // As wide as the cell, and wide enough for a label and a key.
    let base = anchor.width().max(280.0);
    let chrome = 2.0 * shape.pad.y + lines as f32 * shape.line;
    let screen = ui.ctx().content_rect();
    let wanted = (chrome, shape.option, total.min(VISIBLE));
    let placed = place(anchor, vec2(base, chrome), wanted, screen);
    let rows = placed.rows;
    // The list follows the highlight: the keys scroll it. By the rows
    // that fit, so the highlighted option is always drawn.
    let first = picker
        .highlight
        .map_or(0, |at| (at + 1).saturating_sub(rows))
        .min(total.saturating_sub(rows));
    let state = planned
        .is_some()
        .then(|| state_line(picker, typed, names.table, total - rows, locale))
        .flatten();
    // The terminal look's options are one line, the key last and whole:
    // the list is as wide as the widest it draws needs, up to a cap.
    let width = if look.terminal {
        let drawn = &picker.options[first..first + rows];
        let need = terminal_need(ui, drawn, picker, &shape, (look, locale));
        let cap = TERMINAL_CAP.min(screen.width() - 16.0);
        base.max(need.min(cap))
    } else {
        base
    };
    // Placed again by the width it really has, which moves nothing down.
    let placed = place(anchor, vec2(width, chrome), wanted, screen);
    let size = vec2(width, chrome + rows as f32 * shape.option);
    let mut shown = Shown::default();
    // One area for the lists of every table: only the table on screen
    // has one, and egui keeps each area's place for good.
    let id = Id::new("key-picker");
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fade_in(false)
        .constrain(false)
        .fixed_pos(placed.pos)
        .show(ui.ctx(), |ui| {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            // A press between the options picks nothing, but it is the
            // list's: the field keeps the keyboard.
            let whole = ui.interact(rect, id.with("list"), Sense::CLICK);
            let list_name =
                gettext(locale, "Matching {table}").replace("{table}", &display_safe(names.table));
            whole.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &list_name));
            ui.ctx().accesskit_node_builder(whole.id, |node| {
                node.set_role(Role::ListBox);
                node.set_size_of_set(total);
            });
            let clicked_over = ui.input(|input| input.pointer.any_click());
            let clicked_over = clicked_over && ui.rect_contains_pointer(rect);
            shown.pressed = whole.is_pointer_button_down_on() || whole.clicked() || clicked_over;
            let border = sql_complete::panel(ui, rect, shape.corner, look, palette);
            let inner = rect.shrink2(shape.pad);
            let muted = muted(look, palette);
            let line_role = TextRole::pick(look, TextRole::Shortcut, TextRole::OCaption);
            let mut top = inner.top();
            let left = inner.left() + shape.inset;
            // What a line of words has of the list's width.
            let room = inner.width() - 2.0 * shape.inset;
            if head {
                let text = head_line(picker, typed, names.table, locale);
                let y = top + shape.line / 2.0;
                say(ui, (left, y), room, (&text, line_role, muted), inner, look);
                top += shape.line;
            }
            let options = UiBuilder::new()
                .max_rect(inner)
                .accessibility_parent(whole.id);
            let options = ui.new_child(options);
            for (slot, index) in (first..first + rows).enumerate() {
                let option = &picker.options[index];
                let row = Rect::from_min_size(
                    pos2(inner.left(), top + slot as f32 * shape.option),
                    vec2(inner.width(), shape.option),
                );
                let row_id = id.with(("option", index));
                // Clicked, never focused: an option that could take the
                // keyboard would take it from the field.
                let response = options.interact(row, row_id, Sense::CLICK);
                let selected = picker.highlight == Some(index);
                let text = picker.text(typed);
                let current = picker.is_current(&option.key);
                response.widget_info(|| {
                    let name = option_name(option, current, locale);
                    WidgetInfo::labeled(WidgetType::Other, true, name)
                });
                ui.ctx().accesskit_node_builder(row_id, |node| {
                    node.set_role(Role::ListBoxOption);
                    node.set_selected(selected);
                    node.set_position_in_set(index);
                    node.set_size_of_set(total);
                });
                let corner = CornerRadius::same(shape.row_corner);
                if selected {
                    ui.painter().rect_filled(row, corner, palette.selection);
                    // The keyboard stays in the field: its node says which
                    // option is the current one.
                    ui.ctx().accesskit_node_builder(field, |node| {
                        node.set_active_descendant(row_id.accesskit_id());
                    });
                } else if response.hovered() {
                    ui.painter().rect_filled(row, corner, palette.surface_hover);
                }
                paint_option(
                    ui,
                    row,
                    option,
                    text,
                    current,
                    &shape,
                    (look, palette, locale),
                );
                shown.pressed |= response.is_pointer_button_down_on() || response.clicked();
                if response.clicked() {
                    shown.picked = Some(index);
                }
            }
            top += rows as f32 * shape.option;
            if let Some((said, failed)) = &state {
                let color = if *failed { palette.danger } else { muted };
                let y = top + shape.line / 2.0;
                say(ui, (left, y), room, (said, line_role, color), inner, look);
                top += shape.line;
            }
            if foot {
                let y = top + shape.line / 2.0;
                let cancels = gettext(locale, "Esc cancels");
                // Set NULL stands at the foot's right end, and the words
                // end before it.
                let label = gettext(locale, "Set NULL");
                let button = names.nullable.then(|| {
                    let button = widgets::ButtonSpec::new(&label)
                        .padding(8.0)
                        .role(widgets::secondary(look));
                    let wide = button.width(ui, look);
                    (button, wide)
                });
                let taken = button.as_ref().map_or(0.0, |(_, wide)| wide + 8.0);
                let args = (&*cancels, line_role, muted);
                say(ui, (left, y), room - taken, args, inner, look);
                if let Some((button, wide)) = button {
                    let place = Rect::from_center_size(
                        pos2(inner.right() - shape.inset - wide / 2.0, y),
                        vec2(wide, shape.line - 4.0),
                    );
                    let response = button.show_at(ui, place, look, palette);
                    shown.pressed |= response.is_pointer_button_down_on() || response.clicked();
                    shown.null = response.clicked();
                }
            }
            if let Some(count) = count_line(picker, names.table, locale) {
                widgets::announce_polite(ui, rect, &count);
            }
            ui.painter().add(border);
        });
    shown
}

/// One option. On macOS two lines: the label, its match heavier, and
/// under it the key, small and muted. In the terminal look one line: the
/// label, its match in the accent, then the key, muted. An option of a
/// table with no label column is its key alone. The row the cell points
/// at is marked at its end.
fn paint_option(
    ui: &Ui,
    row: Rect,
    option: &LookupRow,
    text: &str,
    current: bool,
    shape: &Shape,
    (look, palette, locale): (&Look, &Palette, Locale),
) {
    let muted = muted(look, palette);
    let left = row.left() + shape.inset;
    let mut right = row.right() - shape.inset;
    if current && look.terminal {
        let mark = format!("· {}", gettext(locale, "current"));
        let role = TextRole::pick(look, TextRole::Shortcut, TextRole::OCaption);
        let laid = Text::one(look, role, &mark, muted).layout(ui.ctx());
        right -= laid.paint_right(ui.painter(), right, row.center().y) + LINE_GAP;
    } else if current {
        // The look's check, not a character a font may lack.
        let place = Rect::from_center_size(
            pos2(right - CHECK / 2.0, row.center().y),
            vec2(CHECK, CHECK),
        );
        Icon::Check.image(muted, CHECK).paint_at(ui, place);
        right -= CHECK + 8.0;
    }
    let room = (right - left).max(0.0);
    let clip = Rect::from_min_max(pos2(left, row.top()), pos2(right, row.bottom()));
    let painter = ui.painter().with_clip_rect(clip);
    // A label is words: the body face, its match heavier. The terminal
    // look writes everything in its one face, the match in the accent.
    let (plain, hit, hit_color) = if look.terminal {
        (TextRole::OCode, TextRole::OCode, palette.accent)
    } else {
        (TextRole::UiBody, TextRole::UiBodySemibold, palette.text)
    };
    let key_role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OCode);
    let key = plain_text(&option.key);
    let key = display_safe(&key);
    // A key that begins with the text has that start emphasised too.
    let key_hit = (!text.is_empty() && key.starts_with(text)).then_some(text.len());
    let key_text = || match key_hit {
        Some(end) => Text::new(look)
            .add(key_role, &key[..end], palette.text)
            .add(key_role, &key[end..], muted),
        None => Text::one(look, key_role, &key, muted),
    };
    // On the terminal's one line the key and the mark come first: they
    // are what Enter sets, and the label is cut to what is left of the
    // room.
    let label_room = if look.terminal {
        let key_width = key_role.width(ui.ctx(), look.faces, &key);
        (room - key_width - LINE_GAP).max(0.0)
    } else {
        room
    };
    let label = match &option.label {
        None => None,
        Some(Value::Null) => Some(Text::one(look, plain, "NULL", muted)),
        Some(value) => {
            // Borrowed where it is a text; `display_safe` writes a line
            // break as a visible code, so the label is one line.
            let label = label_text(value);
            let label = display_safe(&label);
            let label = &*label;
            let room = label_room;
            let whole = |shown: &str| match found_in(shown, text) {
                Some(found) => Text::new(look)
                    .add(plain, &shown[..found.start], palette.text)
                    .add(hit, &shown[found.clone()], hit_color)
                    .add(plain, &shown[found.end..], palette.text),
                None => Text::one(look, plain, shown, palette.text),
            };
            let width = |shown: &str| plain.width(ui.ctx(), look.faces, shown);
            Some(if width(label) <= room {
                whole(label)
            } else {
                // Cut, and drawn plain: see the plan's row e.
                let cut = crate::ui::grid::ellipsize(label, room, false, width);
                Text::one(look, plain, &cut, palette.text)
            })
        }
    };
    match (label, look.terminal) {
        (None, _) => {
            key_text()
                .layout(ui.ctx())
                .paint_left(&painter, left, row.center().y);
        }
        (Some(label), true) => {
            let wide = label
                .layout(ui.ctx())
                .paint_left(&painter, left, row.center().y);
            key_text().layout(ui.ctx()).paint_left(
                &painter,
                left + wide + LINE_GAP,
                row.center().y,
            );
        }
        (Some(label), false) => {
            let quarter = row.height() / 4.0;
            label
                .layout(ui.ctx())
                .paint_left(&painter, left, row.top() + quarter + 2.0);
            key_text()
                .layout(ui.ctx())
                .paint_left(&painter, left, row.bottom() - quarter - 1.0);
        }
    }
}
