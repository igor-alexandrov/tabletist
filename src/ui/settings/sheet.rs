//! The Settings window in the looks that are not a terminal's: a sheet
//! over the dimmed window, with the nav at its left and the tab's options
//! in rows, each with a control that changes it.

use egui::{CornerRadius, Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2};

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::model::Action;
use crate::settings::{OptionId, OptionValue, Settings, Timestamps};
use crate::theme::{Look, Palette};
use crate::typography::{Laid, Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::format::group_digits;
use crate::ui::value_tags::slot_colors;
use crate::ui::widgets::{self, Segment};

use super::{label, sample_timestamp, small_print};

/// The sheet at its widest, and what it leaves of the window at each side.
const WIDTH: f32 = 1040.0;
const MARGIN: f32 = 40.0;
/// The nav, its padding, and one of its items.
const NAV: f32 = 210.0;
const NAV_PAD: egui::Vec2 = vec2(10.0, 14.0);
const NAV_ITEM: f32 = 28.0;
/// The content's padding: over the title and under the last row, and at
/// its sides.
const TOP: f32 = 22.0;
const SIDE: f32 = 28.0;
/// A row: its label's column, the gap after it, and the space over and
/// under what it holds.
const LABEL: f32 = 300.0;
const GAP: f32 = 16.0;
const ROW_PAD: f32 = 9.0;
/// What a narrow sheet keeps for the controls: the label's column gives
/// way to them.
const CONTROLS: f32 = 210.0;
/// Between a control and its hint.
const HINT_GAP: f32 = 12.0;
/// The page size menu's button.
const MENU: f32 = 120.0;
/// A segment's height, and the switch's.
const SEGMENT: f32 = 22.0;
const SWITCH: f32 = 18.0;

/// How the sheet draws: the look, the palette and the language.
#[derive(Clone, Copy)]
struct Skin<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: Locale,
}

impl Skin<'_> {
    /// `text` translated.
    fn say(&self, text: &'static str) -> String {
        gettext(self.locale, text).into_owned()
    }
}

pub(super) fn show(app: &App, ctx: &egui::Context, actions: &mut Vec<Action>) {
    let skin = Skin {
        look: &app.look,
        palette: &app.palette,
        locale: app.locale,
    };
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 2.0 * MARGIN).max(NAV + 320.0);
    let id = egui::Id::new("settings");
    // Its parts fill it edge to edge: no margin of its own. The shadow is
    // the connection dialog's, the app's other sheet.
    let frame = egui::Frame::new()
        .fill(skin.palette.window)
        .corner_radius(CornerRadius::same(skin.look.dialog_radius))
        .stroke(Stroke::new(1.0, skin.palette.border))
        .shadow(egui::epaint::Shadow {
            offset: [0, 24],
            blur: 64,
            spread: 0,
            color: skin.palette.shadow,
        });
    // Whole on the first frame it paints, as the connection dialog is:
    // through a fade the window would show in every row of a sheet this
    // large.
    let area = egui::Modal::default_area(id).fade_in(false);
    let modal = widgets::modal(id, skin.look, skin.palette)
        .frame(frame)
        .area(area)
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            let room = ui.available_rect_before_wrap();
            // The nav is as tall as the content, which is laid out after
            // it: its fill waits here, under its words, for that height.
            let fill = ui.painter().add(egui::Shape::Noop);
            nav(ui, room.min, &skin);
            let pane = Rect::from_min_max(
                pos2(room.left() + NAV + SIDE, room.top() + TOP),
                pos2(room.right() - SIDE, room.bottom()),
            );
            let builder = egui::UiBuilder::new().id_salt("content").max_rect(pane);
            let mut pane = ui.new_child(builder.layout(egui::Layout::top_down(egui::Align::Min)));
            content(&mut pane, &app.settings, &skin, actions);
            let bottom = pane.min_rect().bottom() + TOP;
            ui.allocate_rect(room.with_max_y(bottom), Sense::hover());
            let side = Rect::from_min_max(room.min, pos2(room.left() + NAV, bottom));
            // The sheet's own corners, inside its border.
            let radius = skin.look.dialog_radius.saturating_sub(1);
            let corners = CornerRadius {
                nw: radius,
                sw: radius,
                ..CornerRadius::ZERO
            };
            ui.painter().set(
                fill,
                egui::epaint::RectShape::filled(side, corners, skin.palette.panel),
            );
            widgets::vline(ui, side.right() - 0.5, side.y_range(), skin.palette.outline);
        });
    // Escape closes the sheet, unless it is closing the open menu first.
    if modal.is_top_modal
        && !modal.any_popup_open
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        actions.push(Action::CloseDialog);
    }
}

/// The window's name and its tabs, down the left from `corner`: the one
/// tab there is, which is the one shown.
fn nav(ui: &mut Ui, corner: egui::Pos2, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::StateTitle;
    let line = role.row_height(ui.ctx(), look.faces);
    let top = corner.y + NAV_PAD.y;
    widgets::paint_label(
        ui,
        corner.x + NAV_PAD.x + 10.0,
        top + line / 2.0,
        Text::one(look, role, &skin.say("Settings"), palette.text),
    );
    // Up to the nav's rule at the right.
    let item = Rect::from_min_size(
        pos2(corner.x + NAV_PAD.x, top + line + 10.0),
        vec2(NAV - 1.0 - 2.0 * NAV_PAD.x, NAV_ITEM),
    );
    let name = skin.say("General");
    let button = ui.interact(item, ui.id().with("general"), Sense::click());
    button.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, true, &name));
    ui.painter()
        .rect_filled(item, CornerRadius::same(5), palette.selection);
    widgets::paint_text(
        ui,
        item.left() + 8.0,
        item.center().y,
        Text::one(look, TextRole::UiBodyStrong, &name, palette.accent_hover),
    );
    // An item of a list: the ring is inside its fill, as a row's is.
    focus::hint(ui, &button, item, Ring::Inset { radius: 5 });
}

/// The tab: its title, what it is for, and its options under the heading
/// of their group.
fn content(ui: &mut Ui, settings: &Settings, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    widgets::label(
        ui,
        TextRole::DialogTitle,
        &skin.say("General"),
        palette.text,
        look,
    );
    ui.add_space(2.0);
    widgets::label(
        ui,
        TextRole::UiBody,
        &skin.say("How Tabletist shows data. Changes apply right away."),
        palette.dim,
        look,
    );
    ui.add_space(6.0 + 14.0);
    widgets::section_label(&skin.say("Data"), look, palette)
        .layout(ui.ctx())
        .label(ui);
    ui.add_space(2.0);
    let column = label_column(ui, skin);
    for option in OptionId::ALL {
        row(ui, option, column, settings, skin, actions);
    }
}

/// How wide the rows' labels have it: `LABEL`, or what the widest small
/// print takes in a face that writes it wider, so that it stays on one
/// line. A sheet too narrow for that and the controls takes from the
/// labels, and the small print wraps.
fn label_column(ui: &Ui, skin: &Skin) -> f32 {
    let role = TextRole::Secondary;
    let widest = OptionId::ALL
        .into_iter()
        .filter_map(small_print)
        .map(|print| role.width(ui.ctx(), skin.look.faces, &skin.say(print)))
        .fold(LABEL, f32::max);
    let room = ui.available_width() - GAP - CONTROLS;
    widest.ceil().min(room).max(0.0)
}

/// One option's row: its label over its small print in a column `column`
/// wide, its control, and the hint that shows what the value does, over a
/// rule.
fn row(
    ui: &mut Ui,
    option: OptionId,
    column: f32,
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let name = Text::one(
        look,
        TextRole::UiBody,
        &skin.say(label(option)),
        palette.text,
    )
    .layout(ui.ctx());
    let print = small_print(option).map(|print| {
        Text::one(look, TextRole::Secondary, &skin.say(print), palette.dim)
            .wrap(column)
            .layout(ui.ctx())
    });
    // The label with its small print 2 under it.
    let words = name.height() + print.as_ref().map_or(0.0, |print| 2.0 + print.height());
    let inner = words.max(control_height(option, look));
    let height = ROW_PAD + inner + ROW_PAD + 1.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.surface);
    let middle = rect.top() + ROW_PAD + inner / 2.0;
    let mut top = middle - words / 2.0;
    for line in std::iter::once(name).chain(print) {
        write(ui, rect.left(), top + line.height() / 2.0, &line);
        top += line.height() + 2.0;
    }
    // The control and its hint, on the row's middle. Each row's controls
    // have ids of their own: two segmented controls in one `Ui` would
    // share their arrow keys.
    let place = Rect::from_min_max(
        pos2(rect.left() + column + GAP, rect.top()),
        pos2(rect.right(), rect.bottom() - 1.0),
    );
    let centred = egui::Layout::left_to_right(egui::Align::Center);
    let builder = egui::UiBuilder::new()
        .id_salt(label(option))
        .max_rect(place);
    let mut line = ui.new_child(builder.layout(centred));
    control(&mut line, option, settings, skin, actions);
    let left = line.min_rect().right() + HINT_GAP;
    hint(ui, pos2(left, middle), rect.right(), option, settings, skin);
}

/// How tall `option`'s control is: its row is at least as tall.
fn control_height(option: OptionId, look: &Look) -> f32 {
    match option {
        OptionId::PageSize => look.control_height,
        // The track has 2 over its segments and under them.
        OptionId::Timestamps | OptionId::GroupDigits => SEGMENT + 4.0,
        OptionId::ValueTags => SWITCH,
    }
}

/// What changes `option`: a menu of the page sizes, two segments, or a
/// switch.
fn control(
    ui: &mut Ui,
    option: OptionId,
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let name = skin.say(label(option));
    match option {
        OptionId::PageSize => {
            let written = |ui: &Ui, size: u32| {
                let text = group_digits(u64::from(size));
                widgets::galley(ui, &text, egui::Color32::PLACEHOLDER, look)
            };
            let combo = egui::ComboBox::from_id_salt("page-size")
                .width(MENU)
                .selected_text(written(ui, settings.page_size));
            let menu = widgets::popup_button(ui, combo, look, palette, |ui| {
                for size in page_sizes(settings.page_size) {
                    let chosen = size == settings.page_size;
                    let entry = ui.add(egui::Button::selectable(chosen, written(ui, size)));
                    // Named as the SQL editor's Limit menu names its
                    // entries: what it sets, and to what.
                    let entry_name = format!("{name} {}", group_digits(u64::from(size)));
                    entry.widget_info(|| {
                        WidgetInfo::selected(WidgetType::Button, true, chosen, &entry_name)
                    });
                    if entry.clicked() {
                        actions.push(Action::SetOption(OptionValue::PageSize(size)));
                        // egui closes the menu on a pointer's click; a
                        // pick by key or by a screen reader closes it here.
                        ui.close();
                    }
                }
            });
            // The row's label is beside it, not its own: named for screen
            // readers, with the size as its value.
            let value = group_digits(u64::from(settings.page_size));
            menu.response.widget_info(|| {
                let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, &name);
                info.current_text_value = Some(value.clone());
                info
            });
        }
        OptionId::Timestamps => {
            let words = [skin.say("To the second"), skin.say("Full precision")];
            let selected = usize::from(settings.timestamps == Timestamps::Full);
            if let Some(picked) = segments(ui, &words, selected, skin) {
                let value = [Timestamps::Second, Timestamps::Full][picked];
                actions.push(Action::SetOption(OptionValue::Timestamps(value)));
            }
        }
        OptionId::GroupDigits => {
            // A number as each value writes it: no language has others.
            let words = ["1,240.50".to_owned(), "1240.50".to_owned()];
            let selected = usize::from(!settings.group_digits);
            if let Some(picked) = segments(ui, &words, selected, skin) {
                actions.push(Action::SetOption(OptionValue::GroupDigits(picked == 0)));
            }
        }
        OptionId::ValueTags => {
            if widgets::toggle(ui, settings.value_tags, &name, palette).clicked() {
                actions.push(Action::SetOption(OptionValue::ValueTags(
                    !settings.value_tags,
                )));
            }
        }
    }
}

/// A segmented control of `words`, the one at `selected` chosen. Returns
/// the index of a segment picked this frame. The segments are of one
/// width: the widest word's, with 10 at each side.
fn segments(ui: &mut Ui, words: &[String; 2], selected: usize, skin: &Skin) -> Option<usize> {
    let Skin { look, palette, .. } = *skin;
    // The role `widgets::segmented` writes its words in.
    let role = TextRole::FieldLabel;
    let widest = words
        .iter()
        .map(|word| role.width(ui.ctx(), look.faces, word))
        .fold(0.0, f32::max);
    let size = vec2((widest + 20.0).ceil(), SEGMENT);
    let parts = [Segment::Text(&words[0]), Segment::Text(&words[1])];
    widgets::segmented(ui, &parts, selected, size, look, palette)
}

/// The sizes the menu offers: the list, with the size in use in its place
/// when the file set one that is not in it.
fn page_sizes(current: u32) -> Vec<u32> {
    let mut sizes = Settings::PAGE_SIZES.to_vec();
    if !sizes.contains(&current) {
        sizes.push(current);
        sizes.sort_unstable();
    }
    sizes
}

/// What the row says after `option`'s control, from `at` on its middle: a
/// sample of what the value does, or what it does not do. Whole or left
/// out: a hint that does not end before `right` tells nothing cut short.
fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settings, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let text = match option {
        OptionId::PageSize => return,
        OptionId::Timestamps => Text::one(
            look,
            TextRole::MonoSecondary,
            sample_timestamp(settings.timestamps),
            palette.dim,
        ),
        OptionId::GroupDigits => Text::one(
            look,
            TextRole::Secondary,
            &skin.say("Grouping is display only; copy gives the raw value"),
            palette.dim,
        ),
        OptionId::ValueTags => return tags(ui, at, right, settings.value_tags, skin),
    };
    let text = text.layout(ui.ctx());
    if at.x + text.width() <= right {
        write(ui, at.x, at.y, &text);
    }
}

/// Paints `text` from `x` on, centred on `y`, and names it for screen
/// readers.
fn write(ui: &Ui, x: f32, y: f32, text: &Laid) {
    text.paint_left(ui.painter(), x, y);
    let place = Rect::from_min_size(pos2(x, y - text.height() / 2.0), text.size());
    widgets::announce(ui, place, text.galley.text());
}

/// Two values as a grid shows them: tags in the first two slots' colours,
/// or plain while tags are off. They are values of a column, not words: no
/// language has others.
fn tags(ui: &Ui, at: egui::Pos2, right: f32, on: bool, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::ValueTag;
    // A tag has 6 at each side of its value and 2 over and under it.
    let pad = if on { 6.0 } else { 0.0 };
    let height = role.row_height(ui.ctx(), look.faces) + 4.0;
    let values = ["print", "ebook"];
    let widths = values.map(|value| role.width(ui.ctx(), look.faces, value) + 2.0 * pad);
    if at.x + widths[0] + 6.0 + widths[1] > right {
        return;
    }
    let mut left = at.x;
    for (slot, (value, width)) in values.into_iter().zip(widths).enumerate() {
        let color = if on {
            let (color, fill) = slot_colors(slot, look, palette);
            let tag = Rect::from_min_size(pos2(left, at.y - height / 2.0), vec2(width, height));
            if let Some(fill) = fill {
                ui.painter().rect_filled(tag, CornerRadius::same(4), fill);
            }
            color
        } else {
            palette.dim
        };
        widgets::paint_label(ui, left + pad, at.y, Text::one(look, role, value, color));
        left += width + 6.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_that_is_not_in_the_list_takes_its_place_among_the_others() {
        assert_eq!(page_sizes(300), Settings::PAGE_SIZES);
        assert_eq!(page_sizes(250), [100, 250, 300, 500, 1_000, 5_000]);
        assert_eq!(page_sizes(9_000), [100, 300, 500, 1_000, 5_000, 9_000]);
    }
}
