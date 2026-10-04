//! The Settings window as the terminal look draws it: a screen over the
//! whole window. A header, a nav of one item, the options in rows under a
//! cursor, and the screen's keys in the footer.

use egui::emath::GuiRounding as _;
use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::model::Action;
use crate::settings::{OptionId, OptionValue, Settings, Timestamps};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::keys::keys_label;
use crate::ui::value_tags::terminal_slots;
use crate::ui::widgets::{self, ButtonSpec};

use super::{choices, label};

/// The header's and the footer's heights, each without its rule.
const HEADER: f32 = 40.0;
const FOOTER: f32 = 30.0;
/// The nav's width.
const NAV: f32 = 220.0;
/// An option's row, and how far its words stand in from its sides.
const ROW: f32 = 30.0;
const ROW_SIDE: f32 = 16.0;
/// The label's column and the value's; the hint has what is left.
const LABEL: f32 = 280.0;
const VALUE: f32 = 320.0;
/// The space at each side of a segment's word.
const SEGMENT_PAD: f32 = 6.0;

/// How the screen draws: the look, the palette and the language.
#[derive(Clone, Copy)]
struct Skin<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: Locale,
}

impl Skin<'_> {
    /// `text` translated, in the look's case.
    fn say(&self, text: &'static str) -> String {
        self.look.label(&gettext(self.locale, text))
    }
}

/// A row's value: the row, where its column starts, and whether the cursor
/// is on it.
#[derive(Clone, Copy)]
struct Cell {
    index: usize,
    row: Rect,
    left: f32,
    on_cursor: bool,
}

pub(super) fn show(app: &App, ctx: &egui::Context, row: usize, actions: &mut Vec<Action>) {
    let skin = Skin {
        look: &app.look,
        palette: &app.palette,
        locale: app.locale,
    };
    let screen = ctx.content_rect();
    let id = egui::Id::new("settings");
    // A modal: nothing behind it can be clicked while it is open. It is
    // whole on the first frame it paints: through a fade the workspace
    // would show in every row.
    let area = egui::Modal::default_area(id)
        .anchor(egui::Align2::LEFT_TOP, egui::Vec2::ZERO)
        .fade_in(false);
    // No dialog's border, corners or margin: the screen is the window.
    let frame = egui::Frame::new().fill(skin.palette.window);
    widgets::modal(id, skin.look, skin.palette)
        .frame(frame)
        .area(area)
        .backdrop_color(skin.palette.window)
        .show(ctx, |ui| {
            ui.set_width(screen.width());
            ui.set_height(screen.height());
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            header(ui, &skin);
            // What the header, the footer and their rules leave.
            let height = (screen.height() - HEADER - FOOTER - 2.0).max(0.0);
            let (body, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let (side, pane) = body.split_left_right_at_x((body.left() + NAV).min(body.right()));
            nav(ui, side, &skin);
            // The rows scroll when the window is short, so the header and
            // the footer stay on screen. They start past the nav's rule.
            let pane = pane.with_min_x((pane.left() + 1.0).min(pane.right()));
            let mut list = ui.new_child(egui::UiBuilder::new().id_salt("rows").max_rect(pane));
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(&mut list, |ui| {
                    rows(ui, &app.settings, row, &skin, actions);
                });
            let option = OptionId::ALL.get(row).copied();
            footer(ui, option, &app.settings, &skin, actions);
        });
}

/// One tinted line: the title and the tab it shows, and at the right what
/// a change does and the key that opens the screen.
fn header(ui: &mut Ui, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), HEADER + 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette.panel);
    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.outline);
    let y = rect.top() + HEADER / 2.0;
    let mut x = rect.left() + 14.0;
    x += widgets::paint_label(
        ui,
        x,
        y,
        Text::one(
            look,
            TextRole::OScreenTitle,
            &skin.say("Settings"),
            palette.text,
        ),
    ) + 12.0;
    x += widgets::paint_text(
        ui,
        x,
        y,
        Text::one(look, TextRole::OBody, &skin.say("General"), palette.dim),
    );
    let role = widgets::secondary(look);
    let note = format!("{} · ", skin.say("Changes apply right away"));
    let note = Text::new(look)
        .add(role, &note, palette.dim)
        .add(role, &look.label(&keys_label("Mod+,")), palette.accent)
        .space(role, " ")
        .add(role, &skin.say("opens this"), palette.dim)
        .layout(ui.ctx());
    // The note gives way to the title in a window too narrow for both.
    let right = rect.right() - 14.0;
    if right - note.width() >= x + 12.0 {
        note.paint_right(ui.painter(), right, y);
    }
}

/// The mark of the nav's item and of the cursor's row: the selection's
/// fill, with a 2 pt bar in the accent at its left.
fn mark(ui: &Ui, rect: Rect, palette: &Palette) {
    let painter = ui.painter();
    let rect = snapped(ui, rect);
    painter.rect_filled(rect, CornerRadius::ZERO, palette.selection);
    let bar = snapped(ui, Rect::from_min_size(rect.min, vec2(2.0, rect.height())));
    painter.rect_filled(bar, CornerRadius::ZERO, palette.accent);
}

/// `rect` with its edges on whole pixels: a fill or an outline that ends
/// between two is blurred there. A line of text is not a whole number of
/// pixels tall at every scale, and what is centred on a row starts on half
/// of one.
fn snapped(ui: &Ui, rect: Rect) -> Rect {
    rect.round_to_pixels(ui.pixels_per_point())
}

/// The tabs, down the left: the one there is, which is the one shown.
fn nav(ui: &Ui, rect: Rect, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    widgets::vline(ui, rect.right() + 0.5, rect.y_range(), palette.outline);
    // The item is its line of text, with 3 above and below.
    let line = TextRole::OBody.row_height(ui.ctx(), look.faces);
    let item = Rect::from_min_size(
        pos2(rect.left(), rect.top() + 10.0),
        vec2(rect.width(), line + 6.0),
    );
    mark(ui, item, palette);
    widgets::paint_label(
        ui,
        item.left() + 14.0,
        item.center().y,
        Text::one(look, TextRole::OBody, &skin.say("General"), palette.text),
    );
}

/// The options, one to a row, under the heading of their group.
fn rows(ui: &mut Ui, settings: &Settings, cursor: usize, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OCaption;
    // 8 at the top, then the heading with 12 over it and 4 under: a whole
    // number of points, so the rows' fills start on a pixel.
    let line = role.row_height(ui.ctx(), look.faces);
    let height = (8.0 + 12.0 + line + 4.0).ceil();
    let (place, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    // Dim, which `widgets::section_label` is not in this look.
    widgets::paint_label(
        ui,
        place.left() + ROW_SIDE,
        place.top() + 20.0 + line / 2.0,
        Text::one(look, role, &skin.say("Data"), palette.dim),
    );
    for (index, option) in OptionId::ALL.into_iter().enumerate() {
        option_row(
            ui,
            (index, option),
            index == cursor,
            settings,
            skin,
            actions,
        );
    }
}

/// One option's row: its label, its value where a click sets it, and the
/// hint that shows what the value does.
fn option_row(
    ui: &mut Ui,
    (index, option): (usize, OptionId),
    on_cursor: bool,
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin {
        look,
        palette,
        locale,
    } = *skin;
    let role = TextRole::OBody;
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
    // The row's button comes before its values', so that a click on a
    // value is the value's.
    let name = gettext(locale, label(option));
    let button = ButtonSpec::new(&name).hidden_at(ui, row);
    // A row of a list: a ring outside it would be cut where the list ends.
    focus::hint(ui, &button, row, Ring::Inset { radius: 0 });
    if button.clicked() {
        actions.push(Action::SelectSettingsRow(index));
    }
    if on_cursor {
        mark(ui, row, palette);
    }
    let y = row.center().y;
    let left = row.left() + ROW_SIDE;
    let right = (row.right() - ROW_SIDE).max(left);
    // The cursor's mark stands where a label starts, so the label under
    // the cursor is a cell further in than the others.
    let text = skin.say(label(option));
    let text = if on_cursor {
        Text::one(look, role, &format!("▌{text}"), palette.accent)
    } else {
        Text::one(look, role, &text, palette.text)
    };
    widgets::paint_text(ui, left, y, text);
    // A narrow window takes from the hint first, then from the value's
    // column.
    let label_width = LABEL.min(right - left);
    let value_width = VALUE.min(right - left - label_width);
    let cell = Cell {
        index,
        row,
        left: left + label_width,
        on_cursor,
    };
    match option {
        OptionId::PageSize => page_size(ui, cell, settings, skin, actions),
        OptionId::Timestamps | OptionId::GroupDigits => {
            segments(ui, cell, option, settings, skin, actions);
        }
        OptionId::ValueTags => check(ui, cell, settings, skin, actions),
    }
    // The hint is whole or left out: it tells nothing cut short.
    let hint = hint(option, settings, skin).layout(ui.ctx());
    let hint_left = cell.left + value_width;
    if hint_left + hint.width() <= right {
        hint.paint_left(ui.painter(), hint_left, y);
    }
}

/// The button over a choice as it is drawn, named by its option and its
/// own word: a click there moves the cursor to its row and sets it.
fn choice(
    ui: &mut Ui,
    place: Rect,
    index: usize,
    (word, value): (&'static str, OptionValue),
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let name = format!(
        "{}: {}",
        gettext(skin.locale, label(value.option())),
        gettext(skin.locale, word)
    );
    if ButtonSpec::new(&name).hidden_at(ui, place).clicked() {
        actions.push(Action::SelectSettingsRow(index));
        actions.push(Action::SetOption(value));
    }
}

/// The page size: the number as it is, and on the cursor's row between the
/// chevrons that step it, each dim where there is no size on its side.
fn page_size(ui: &mut Ui, cell: Cell, settings: &Settings, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let y = cell.row.center().y;
    let size = settings.page_size.to_string();
    let current = settings.value(OptionId::PageSize);
    let [fewer, more] = choices(OptionId::PageSize, settings)[..] else {
        return;
    };
    let text = if cell.on_cursor {
        let chevron = |(_, step): (&str, OptionValue)| {
            if step == current {
                palette.dim
            } else {
                palette.accent
            }
        };
        Text::new(look)
            .add(role, "‹", chevron(fewer))
            .space(role, " ")
            .add(role, &size, palette.text)
            .space(role, " ")
            .add(role, "›", chevron(more))
    } else {
        Text::one(look, role, &size, palette.text)
    };
    widgets::paint_text(ui, cell.left, y, text);
    // The chevrons' buttons are on every row. On the cursor's row each is
    // over its chevron, with half the space beside it. On the others, where
    // no chevron is drawn, each keeps a point at the column's corner: a
    // click on the number there only moves the cursor.
    let places = if cell.on_cursor {
        let width = |text: &str| role.width(ui.ctx(), look.faces, text);
        let lead = width("‹ ");
        let chevron = width("‹");
        let reach = (lead - chevron) / 2.0;
        let after = cell.left + lead + width(&size) + (lead - chevron);
        [cell.left, after].map(|left| {
            Rect::from_min_max(
                pos2(left - reach, cell.row.top()),
                pos2(left + chevron + reach, cell.row.bottom()),
            )
        })
    } else {
        [0.0, 1.0].map(|along| {
            Rect::from_min_size(pos2(cell.left + along, cell.row.top()), vec2(1.0, 1.0))
        })
    };
    for (place, step) in places.into_iter().zip([fewer, more]) {
        choice(ui, place, cell.index, step, skin, actions);
    }
}

/// One of two values as segments, each as tall as its line of text and a
/// cell from the next: the chosen one filled with the accent on the
/// cursor's row and outlined on the others, the other one dim.
fn segments(
    ui: &mut Ui,
    cell: Cell,
    option: OptionId,
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let current = settings.value(option);
    let line = role.row_height(ui.ctx(), look.faces);
    let gap = role.width(ui.ctx(), look.faces, " ");
    let mut left = cell.left;
    for (word, value) in choices(option, settings) {
        let text = skin.say(word);
        let width = role.width(ui.ctx(), look.faces, &text) + 2.0 * SEGMENT_PAD;
        let place = Rect::from_min_size(
            pos2(left, cell.row.center().y - line / 2.0),
            vec2(width, line),
        );
        left += width + gap;
        let edge = snapped(ui, place);
        let color = if value != current {
            palette.dim
        } else if cell.on_cursor {
            ui.painter()
                .rect_filled(edge, CornerRadius::ZERO, palette.accent);
            // The dark of the header and the footer.
            palette.panel
        } else {
            let stroke = Stroke::new(1.0, palette.outline);
            ui.painter()
                .rect_stroke(edge, CornerRadius::ZERO, stroke, StrokeKind::Inside);
            palette.text
        };
        let text = Text::one(look, role, &text, color);
        widgets::paint_text(ui, place.left() + SEGMENT_PAD, place.center().y, text);
        choice(ui, place, cell.index, (word, value), skin, actions);
    }
}

/// An option that is on or off, as the look's check.
fn check(ui: &mut Ui, cell: Cell, settings: &Settings, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let current = settings.value(OptionId::ValueTags);
    let (glyph, color) = if settings.value_tags {
        ("[x]", palette.success)
    } else {
        ("[ ]", palette.dim)
    };
    let text = Text::one(look, TextRole::OBody, glyph, color);
    let width = widgets::paint_text(ui, cell.left, cell.row.center().y, text);
    let place = Rect::from_min_max(
        pos2(cell.left, cell.row.top()),
        pos2(cell.left + width, cell.row.bottom()),
    );
    // One mark stands for both values, and a click on it flips it: the
    // value it is not set to is the button on top.
    let mut values = choices(OptionId::ValueTags, settings);
    values.sort_by_key(|(_, value)| *value != current);
    for value in values {
        choice(ui, place, cell.index, value, skin, actions);
    }
}

/// What the row says beside `option`'s value: where it applies, or a
/// sample of what it does.
fn hint(option: OptionId, settings: &Settings, skin: &Skin) -> Text {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let dim = |text: &str| Text::one(look, role, text, palette.dim);
    match option {
        OptionId::PageSize => dim(&skin.say("table view")),
        OptionId::Timestamps => dim(match settings.timestamps {
            Timestamps::Second => "2026-01-12 09:14:03",
            Timestamps::Full => "2026-01-12 09:14:03.482915",
        }),
        OptionId::GroupDigits => dim(&skin.say(if settings.group_digits {
            "grouping on"
        } else {
            "grouping off"
        })),
        // Two tags as a grid shows them: in the first two slots' colours,
        // or as plain values.
        OptionId::ValueTags => {
            let slots = terminal_slots(palette);
            let [first, second] = if settings.value_tags {
                [slots[0], slots[1]]
            } else {
                [palette.dim; 2]
            };
            Text::new(look)
                .add(role, &skin.say("print"), first)
                .space(role, "  ")
                .add(role, &skin.say("ebook"), second)
        }
    }
}

/// The screen's keys at the left. A hint whose key does one thing is that
/// thing's button too.
fn footer(
    ui: &mut Ui,
    option: Option<OptionId>,
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin {
        look,
        palette,
        locale,
    } = *skin;
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), FOOTER + 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette.panel);
    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
    let y = rect.top() + 1.0 + FOOTER / 2.0;
    // What the keys would do on the cursor's row. Space has nothing to
    // flip on an option of more than two values.
    let flip = option
        .and_then(|option| settings.flipped(option))
        .map(Action::SetOption);
    let reset = option.map(|option| Action::SetOption(option.default_value()));
    // The key, what it does, the button it stands for, and what that does.
    let keys = [
        ("j/k", "move", None),
        ("h/l", "change", None),
        ("space", "toggle", Some(("Toggle", flip))),
        ("R", "reset option", Some(("Reset option", reset))),
        ("esc", "close", Some(("Close", Some(Action::CloseDialog)))),
    ];
    let mut left = rect.left() + 12.0;
    for (key, what, button) in keys {
        let what = skin.say(what);
        let hint = [(key, what.as_str(), true)];
        let width = widgets::key_hints(ui, (left, y), &hint, 0.0, look, palette);
        // In from the window's edge by a focus ring's reach, or the ring
        // would be cut there.
        let place = Rect::from_min_max(
            pos2(left, rect.top() + 5.0),
            pos2(left + width, rect.bottom() - 4.0),
        );
        left += width + 16.0;
        if let Some((name, action)) = button {
            let name = gettext(locale, name);
            if ButtonSpec::new(&name).hidden_at(ui, place).clicked() {
                actions.extend(action);
            }
        }
    }
}
