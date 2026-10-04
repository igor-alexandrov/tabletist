//! The Settings window as the terminal look draws it: a screen over the
//! whole window. A header, the app's notice while it has one, a nav of one
//! item, the options in rows under a cursor, the settings file beside them
//! where the window has the room, and the screen's keys in the footer.

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

use super::file_pane::{shown_path, spans};
use super::{choices, label};

/// The header's and the footer's heights, each without its rule.
const HEADER: f32 = 40.0;
const FOOTER: f32 = 30.0;
/// A notice's band without its rule, and how far its line stands in from
/// the band's sides.
const NOTICE: f32 = 36.0;
const NOTICE_SIDE: f32 = 14.0;
/// The nav's width.
const NAV: f32 = 220.0;
/// An option's row, and how far its words stand in from its sides.
const ROW: f32 = 30.0;
const ROW_SIDE: f32 = 16.0;
/// The label's column and the value's at their widest; the hint has what
/// is left.
const LABEL: f32 = 280.0;
const VALUE: f32 = 320.0;
/// The space at each side of a segment's word.
const SEGMENT_PAD: f32 = 6.0;
/// The file's pane: this wide, or this share of the window where that is
/// less, and not there in a window narrower than `FILE_FROM`.
const FILE: f32 = 620.0;
const FILE_SHARE: f32 = 0.4;
const FILE_FROM: f32 = 1100.0;
/// The pane's header without its rule, how far its words stand in from its
/// sides, and the space over the file's first line.
const FILE_HEADER: f32 = 34.0;
const FILE_SIDE: f32 = 14.0;
const FILE_TOP: f32 = 12.0;
/// The space over and under the pane's note.
const FILE_NOTE: f32 = 10.0;

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
            let notice = notice(ui, app, actions);
            // What the header, a notice, the footer and their rules leave.
            let height = (screen.height() - HEADER - notice - FOOTER - 2.0).max(0.0);
            let (body, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let (side, pane) = body.split_left_right_at_x((body.left() + NAV).min(body.right()));
            nav(ui, side, &skin);
            // The rows scroll when the window is short, so the header and
            // the footer stay on screen. They start past the nav's rule.
            let pane = pane.with_min_x((pane.left() + 1.0).min(pane.right()));
            let option = OptionId::ALL.get(row).copied();
            // The file has the right of the rows, in a window wide enough
            // for both.
            let (pane, beside) = if screen.width() >= FILE_FROM {
                let width = FILE.min(screen.width() * FILE_SHARE);
                let edge = (pane.right() - width).round_to_pixels(ui.pixels_per_point());
                let (pane, beside) = pane.split_left_right_at_x(edge);
                (pane, Some(beside))
            } else {
                (pane, None)
            };
            let mut list = ui.new_child(egui::UiBuilder::new().id_salt("rows").max_rect(pane));
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(&mut list, |ui| {
                    rows(ui, &app.settings, row, &skin, actions);
                });
            if let Some(beside) = beside {
                file(ui, beside, app, option, &skin);
            }
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

/// The app's notice, in a band under the header: the bar that shows it is
/// under the screen, and a save that fails here would go unseen until the
/// screen closes. Returns the band's height with its rule, nothing when
/// there is no notice.
fn notice(ui: &mut Ui, app: &App, actions: &mut Vec<Action>) -> f32 {
    let Some(message) = &app.notice else {
        return 0.0;
    };
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), NOTICE + 1.0), Sense::hover());
    let fill = crate::ui::notice_fill(&app.palette);
    ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, app.palette.outline);
    let left = rect.left() + NOTICE_SIDE;
    let right = (rect.right() - NOTICE_SIDE).max(left);
    let line = Rect::from_min_max(pos2(left, rect.top()), pos2(right, rect.bottom() - 1.0));
    // The bar's own line, centred on the band.
    let centred = egui::Layout::left_to_right(egui::Align::Center);
    let builder = egui::UiBuilder::new().id_salt("notice").max_rect(line);
    let mut line = ui.new_child(builder.layout(centred));
    crate::ui::notice_line(app, &mut line, message, actions);
    rect.height()
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
    let room = ui.available_width() - 2.0 * ROW_SIDE;
    let columns = columns(ui, room, settings, skin);
    for (index, option) in OptionId::ALL.into_iter().enumerate() {
        option_row(
            ui,
            (index, option),
            index == cursor,
            columns,
            settings,
            skin,
            actions,
        );
    }
}

/// The widths of the label's column and the value's in rows with `room`
/// between their sides: `LABEL` and `VALUE` where the widest hint has room
/// after them. Rows with less (the file is beside them) take from the space
/// each column has past its widest words, from both alike, before a hint
/// is left out.
fn columns(ui: &Ui, room: f32, settings: &Settings, skin: &Skin) -> (f32, f32) {
    let role = TextRole::OBody;
    let width = |text: &str| role.width(ui.ctx(), skin.look.faces, text);
    // Two cells stand between a column's widest words and the next column.
    let gap = width("  ");
    let labels = OptionId::ALL
        .into_iter()
        .map(|option| width(&format!("▌{}", skin.say(label(option)))))
        .fold(0.0, f32::max);
    let values = OptionId::ALL
        .into_iter()
        .map(|option| value_width(ui, option, settings, skin))
        .fold(0.0, f32::max);
    // The widest hint any value has, not only the ones shown: the columns
    // stay where they are when a value changes.
    let hints = OptionId::ALL
        .into_iter()
        .flat_map(|option| choices(option, settings))
        .map(|(_, value)| {
            let mut changed = settings.clone();
            value.set(&mut changed);
            hint(value.option(), &changed, skin)
                .layout(ui.ctx())
                .width()
        })
        .fold(0.0, f32::max);
    let spare = [LABEL - labels - gap, VALUE - values - gap].map(|spare| spare.max(0.0));
    // The shortfall in whole points: a hint that fits by the last fraction
    // of one is not left out for a rounding.
    let short = (LABEL + VALUE + hints - room)
        .ceil()
        .clamp(0.0, spare[0] + spare[1]);
    let share = if short > 0.0 {
        short / (spare[0] + spare[1])
    } else {
        0.0
    };
    (LABEL - share * spare[0], VALUE - share * spare[1])
}

/// How wide `option`'s value is drawn, at its widest.
fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f32 {
    let role = TextRole::OBody;
    let width = |text: &str| role.width(ui.ctx(), skin.look.faces, text);
    match option {
        // Between its chevrons, as the cursor's row has it.
        OptionId::PageSize => width(&format!("‹ {} ›", settings.page_size)),
        OptionId::Timestamps | OptionId::GroupDigits => {
            let words = choices(option, settings);
            let between = width(" ") * words.len().saturating_sub(1) as f32;
            let segments = words.into_iter().map(|(word, _)| segment(ui, word, skin));
            segments.sum::<f32>() + between
        }
        OptionId::ValueTags => width("[x]"),
    }
}

/// How wide the segment of the choice `word` is.
fn segment(ui: &Ui, word: &'static str, skin: &Skin) -> f32 {
    let text = skin.say(word);
    TextRole::OBody.width(ui.ctx(), skin.look.faces, &text) + 2.0 * SEGMENT_PAD
}

/// One option's row: its label, its value where a click sets it, and the
/// hint that shows what the value does.
fn option_row(
    ui: &mut Ui,
    (index, option): (usize, OptionId),
    on_cursor: bool,
    (label_width, value_width): (f32, f32),
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
    // A window too narrow for the columns takes from the hint first, then
    // from the value's column.
    let label_width = label_width.min(right - left);
    let value_width = value_width.min(right - left - label_width);
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
        let width = segment(ui, word, skin);
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

/// The settings file beside its options: where it is and whether it is
/// watched, its text with the line of the cursor's option marked, and what
/// its colours say.
fn file(ui: &mut Ui, rect: Rect, app: &App, option: Option<OptionId>, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let role = widgets::code(look);
    let file = &app.settings_file;
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette.panel);
    widgets::vline(ui, rect.left() + 0.5, rect.y_range(), palette.outline);
    let left = rect.left() + 1.0 + FILE_SIDE;
    let right = (rect.right() - FILE_SIDE).max(left);

    let rule = (rect.top() + FILE_HEADER).min(rect.bottom());
    widgets::hline(ui, rect.x_range(), rule + 0.5, palette.outline);
    let y = rect.top() + FILE_HEADER / 2.0;
    let mut room = right;
    if file.live {
        let live = Text::one(look, role, &skin.say("live"), palette.dim);
        room -= widgets::paint_text_right(ui, right, y, live) + 12.0;
    }
    // The home directory from the environment: nothing here reads the disk.
    let home = directories::BaseDirs::new();
    let home = home.as_ref().map(directories::BaseDirs::home_dir);
    let path = shown_path(&app.dirs.settings_file(), home);
    let path = Text::one(look, role, &path, palette.text).layout(ui.ctx());
    let clip = Rect::from_min_max(pos2(left, rect.top()), pos2(room.max(left), rule));
    let clipped = ui.painter().with_clip_rect(clip);
    // A path longer than the header keeps its end: the file's name is there.
    if path.width() > clip.width() {
        path.paint_right(&clipped, clip.right(), y);
    } else {
        path.paint_left(&clipped, left, y);
    }
    widgets::announce(ui, clip, path.galley.text());

    // The note before the text: how tall it is says where the text ends.
    let before = format!(
        "{} · {} ",
        skin.say("Edits in the file reload live"),
        skin.say("invalid lines are shown here in")
    );
    let note = Text::new(look)
        .add(role, &before, palette.dim)
        .add(role, &skin.say("red"), palette.danger)
        .add(role, &format!(" {}", skin.say("and ignored")), palette.dim)
        .wrap(right - left)
        .layout(ui.ctx());
    let foot = (rect.bottom() - (2.0 * FILE_NOTE + note.height()).ceil() - 1.0).max(rule + 1.0);
    widgets::hline(ui, rect.x_range(), foot + 0.5, palette.outline);
    let under = Rect::from_min_max(pos2(rect.left(), foot + 1.0), rect.max);
    note.paint(
        &ui.painter().with_clip_rect(under),
        pos2(left, under.top() + FILE_NOTE),
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
    let mut text = ui.new_child(egui::UiBuilder::new().id_salt("file").max_rect(body));
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(&mut text, |ui| {
            // The lines as `Settings::from_toml` counts them, from 1. The
            // end of a line as another system's editor wrote it is not
            // drawn.
            let count = file.text.lines().count();
            let height = 2.0 * FILE_TOP + count as f32 * line;
            let (place, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            for (index, text) in file.text.lines().enumerate() {
                let number = index + 1;
                let top = place.top() + FILE_TOP + index as f32 * line;
                let row = Rect::from_min_size(pos2(place.left(), top), vec2(place.width(), line));
                if !ui.is_rect_visible(row) {
                    continue;
                }
                if marked == Some(number) {
                    let fill = snapped(ui, row);
                    ui.painter()
                        .rect_filled(fill, CornerRadius::ZERO, palette.selection);
                }
                let y = row.center().y;
                let mut x = row.left() + FILE_SIDE;
                // A line that was ignored is one colour: nothing in it was
                // read.
                if file.invalid.contains(&number) {
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
    let edit = Some(Action::EditSettingsFile);
    // `$EDITOR` is a name: the look does not lower-case it.
    let in_editor = skin
        .say("open file in {editor}")
        .replace("{editor}", "$EDITOR");
    // The key, what it does, the button it stands for, and what that does.
    let keys = [
        ("j/k", skin.say("move"), None),
        ("h/l", skin.say("change"), None),
        ("space", skin.say("toggle"), Some(("Toggle", flip))),
        ("ctrl+e", in_editor, Some(("Open file in editor", edit))),
        ("R", skin.say("reset option"), Some(("Reset option", reset))),
        (
            "esc",
            skin.say("close"),
            Some(("Close", Some(Action::CloseDialog))),
        ),
    ];
    let mut left = rect.left() + 12.0;
    for (key, what, button) in keys {
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
