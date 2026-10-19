//! The Settings window in the looks that are not a terminal's: a sheet
//! over the dimmed window, with the nav of its panes at its left and the
//! pane's options in rows, each with a control that changes it (the Agents
//! pane is `agents.rs`'s). Under the rows, the footer says where the file
//! is and what can be done with it.

use egui::{CornerRadius, Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2};

use crate::app::App;
use crate::i18n::{Locale, gettext, ngettext};
use crate::keymap::Command;
use crate::model::{Action, SettingsDialog, SettingsPane};
use crate::settings::{
    Autocomplete, Commas, EditorKeys, EnvBar, FontSizes, Idle, Indent, InterfaceSize, KeywordCase,
    Launch, LineNumbers, Mode, NewTab, OptionId, OptionValue, RowHeight, RunScope, Settings,
    TableList, ThemeChoice, Timestamps, Zone,
};
use crate::theme::{DataFace, Look, Palette, Typeface};
use crate::typography::{Laid, Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::format::group_digits;
use crate::ui::menu;
use crate::ui::sql_editor;
use crate::ui::value_tags::slot_colors;
use crate::ui::widgets::{self, Segment};

use super::{label, path_shown, said, sample_timestamp, small_print, zone_text};

/// The sheet at its widest and at its tallest, and what it leaves of the
/// window at each side.
const WIDTH: f32 = 1040.0;
const HEIGHT: f32 = 920.0;
const MARGIN: f32 = 40.0;
/// The nav, its padding, and one of its items.
const NAV: f32 = 210.0;
const NAV_PAD: egui::Vec2 = vec2(10.0, 14.0);
const NAV_ITEM: f32 = 28.0;
/// The content's padding: over the title, and at its sides.
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
/// The zone's menu: as wide as the artboard's, for `System · America/…`.
const ZONE_MENU: f32 = 220.0;
/// The data font's menu and its size's, as wide as the artboard's. A
/// narrow sheet takes from the font's, down to the least it has.
const FONT_MENU: f32 = 220.0;
const FONT_MENU_MIN: f32 = 100.0;
const SIZE_MENU: f32 = 90.0;
/// Between the data font's menu and its size's.
const MENU_GAP: f32 = 12.0;
/// How tall the zone menu's list of zones is at most: it scrolls.
const ZONE_LIST: f32 = 240.0;
/// A segment's height, and the switch's.
const SEGMENT: f32 = 22.0;
const SWITCH: f32 = 18.0;
/// A theme tile's window, and the space round it inside the tile's ring.
const TILE: egui::Vec2 = vec2(120.0, 72.0);
const TILE_PAD: f32 = 6.0;
/// What a short window leaves of the rows at the least: about one of them.
const ROWS_MIN: f32 = 56.0;
/// The footer: the space over and under what it says, and between its
/// parts.
const FOOT_PAD: f32 = 12.0;
const FOOT_GAP: f32 = 12.0;
/// Between "Stored in" and the path: a space.
const WORD_GAP: f32 = 4.0;
/// Under the footer's line about ignored lines.
const WARNING_GAP: f32 = 4.0;
/// How tall a link is to the pointer.
const LINK: f32 = 18.0;

/// How the sheet draws: the look, the palette and the language.
#[derive(Clone, Copy)]
pub(super) struct Skin<'a> {
    pub(super) look: &'a Look,
    pub(super) palette: &'a Palette,
    pub(super) locale: Locale,
    /// The computer's zone's name, which the System zone writes beside it.
    pub(super) system: Option<&'a str>,
    /// The zone database's names, which the zone menu lists: none until
    /// the backend has read it.
    pub(super) zones: &'a [String],
    /// The saved connections, which On launch lists.
    pub(super) connections: &'a [crate::connections::SavedConnection],
    /// The data fonts beyond the bundled faces, on macOS.
    pub(super) fonts: DataFonts<'a>,
}

/// The installed fonts macOS's data font menu lists after the bundled
/// faces, and the one the settings name where nothing draws it.
#[derive(Clone, Copy)]
pub(super) struct DataFonts<'a> {
    /// The installed monospace families: none until the backend has
    /// listed them.
    pub(super) installed: &'a [String],
    /// The font the settings name, where it is no bundled face and no
    /// installed one is read or on its way ([`App::data_font_missing`]):
    /// the default is drawn in its place.
    pub(super) missing: Option<&'a str>,
}

impl Skin<'_> {
    /// `text` translated.
    pub(super) fn say(&self, text: &'static str) -> String {
        gettext(self.locale, text).into_owned()
    }
}

pub(super) fn show(
    app: &App,
    ctx: &egui::Context,
    dialog: &SettingsDialog,
    actions: &mut Vec<Action>,
) {
    let resetting = dialog.resetting;
    let skin = Skin {
        look: &app.look,
        palette: &app.palette,
        locale: app.locale,
        system: app.system_zone.iana_name(),
        zones: app.zones.as_deref().map_or(&[], crate::zones::Zones::names),
        connections: &app.connections.connections,
        fonts: DataFonts {
            installed: &app.installed_fonts,
            missing: app.data_font_missing(),
        },
    };
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 2.0 * MARGIN).max(NAV + 320.0);
    // One height for every pane, whatever it holds: switching panes moves
    // neither the footer nor the sheet.
    let height = HEIGHT.min(screen.height() - 2.0 * MARGIN);
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
            // The nav is as tall as the content and the footer, which are
            // laid out after it: its fill waits here, under its words, for
            // that height.
            let fill = ui.painter().add(egui::Shape::Noop);
            nav(ui, room.min, dialog.pane, &skin, actions);
            // No bottom of its own: its rows end where `content` is told to
            // end them. The sheet's bottom is the one the frame before left
            // it, and rows held to that would get their height back a
            // footer's worth in a frame once the window had grown.
            let pane = Rect::from_min_max(
                pos2(room.left() + NAV + SIDE, room.top() + TOP),
                pos2(room.right() - SIDE, f32::INFINITY),
            );
            // The footer's height before the content's: a window too short
            // for the sheet takes from the rows, which scroll, and the
            // footer stays.
            let ignored = ignored_lines(ui, app.settings_file.invalid.len(), pane.width(), &skin);
            let foot = footer_height(ui, ignored.as_ref(), &skin);
            let limit = room.top() + height - foot;
            let builder = egui::UiBuilder::new().id_salt("content").max_rect(pane);
            let mut pane = ui.new_child(builder.layout(egui::Layout::top_down(egui::Align::Min)));
            match dialog.pane {
                SettingsPane::General => content(
                    &mut pane,
                    ("General", Some(GENERAL)),
                    (
                        &[
                            ("Startup", &OptionId::STARTUP),
                            ("Data", &OptionId::DATA),
                            ("Safety", &OptionId::SAFETY),
                        ],
                        false,
                    ),
                    // As the window shows them: what the look has where the
                    // settings do not say (the grouping of numbers).
                    &super::shown(app),
                    limit,
                    &skin,
                    actions,
                ),
                SettingsPane::Appearance => content(
                    &mut pane,
                    ("Appearance", Some(APPEARANCE)),
                    (
                        &[
                            ("Theme", &OptionId::THEME),
                            ("Text", &OptionId::TEXT),
                            ("Grid", &OptionId::GRID),
                            ("Environment color", &OptionId::ENVIRONMENT),
                        ],
                        true,
                    ),
                    &super::shown(app),
                    limit,
                    &skin,
                    actions,
                ),
                // The title alone: the board's line under it speaks of the
                // history, which is not built yet.
                SettingsPane::Editor => content(
                    &mut pane,
                    ("Editor", None),
                    (
                        &[
                            ("Running queries", &OptionId::RUNNING),
                            ("Typing", &OptionId::TYPING),
                        ],
                        false,
                    ),
                    &super::shown(app),
                    limit,
                    &skin,
                    actions,
                ),
                SettingsPane::Agents => {
                    super::agents::content(&mut pane, app, dialog.agent, limit, &skin, actions);
                }
            }
            let rows = pane.min_rect().bottom();
            // Lower than the rows' end, unless a window too short for even
            // `ROWS_MIN` of them leaves the sheet shorter than that.
            let bottom = limit.max(rows) + foot;
            // The last row leaves its rule to the footer's: where there is
            // room between them, it has one of its own. The preview, which
            // ends the Appearance pane, has its border.
            if rows < limit - 0.5 && dialog.pane != SettingsPane::Appearance {
                let across = egui::Rangef::new(room.left() + NAV + SIDE, room.right() - SIDE);
                widgets::hline(ui, across, rows - 0.5, skin.palette.surface);
            }
            ui.allocate_rect(room.with_max_y(bottom), Sense::hover());
            let under = Rect::from_min_max(
                pos2(room.left() + NAV, bottom - foot),
                pos2(room.right(), bottom),
            );
            let asking = resetting.then_some(dialog.pane);
            footer(ui, under, app, asking, ignored.as_ref(), &skin, actions);
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
    // Escape closes the sheet, and so does a click outside it, which is all
    // a pointer has to leave it with: unless they are closing the open menu
    // first. While the footer asks, they answer that first: no.
    let escape = egui::Key::Escape;
    if modal.is_top_modal
        && !modal.any_popup_open
        && (modal.backdrop_response.clicked()
            || ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, escape)))
    {
        actions.push(if resetting {
            Action::ConfirmResetSettings(false)
        } else {
            Action::CloseDialog
        });
    }
}

/// The window's name and its panes, down the left from `corner`, the one
/// shown filled: a click shows another.
fn nav(
    ui: &mut Ui,
    corner: egui::Pos2,
    shown: SettingsPane,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
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
    for (index, pane) in SettingsPane::listed().iter().copied().enumerate() {
        // Up to the nav's rule at the right, 2 apart.
        let item = Rect::from_min_size(
            pos2(
                corner.x + NAV_PAD.x,
                top + line + 10.0 + index as f32 * (NAV_ITEM + 2.0),
            ),
            vec2(NAV - 1.0 - 2.0 * NAV_PAD.x, NAV_ITEM),
        );
        let on = pane == shown;
        let name = skin.say(pane.name());
        let button = ui.interact(item, ui.id().with(pane.name()), Sense::click());
        button.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, &name));
        if button.clicked() && !on {
            actions.push(Action::ShowSettingsPane(pane));
        }
        let (fill, role, color) = if on {
            (
                palette.selection,
                TextRole::UiBodyStrong,
                palette.accent_hover,
            )
        } else if button.hovered() {
            (palette.surface_hover, TextRole::UiBody, palette.text)
        } else {
            (egui::Color32::TRANSPARENT, TextRole::UiBody, palette.text)
        };
        ui.painter().rect_filled(item, CornerRadius::same(5), fill);
        widgets::paint_text(
            ui,
            item.left() + 8.0,
            item.center().y,
            Text::one(look, role, &name, color),
        );
        // An item of a list: the ring is inside its fill, as a row's is.
        focus::hint(ui, &button, item, Ring::Inset { radius: 5 });
    }
}

/// What the General pane says under its title.
const GENERAL: &str =
    "How Tabletist starts, shows data and protects your work. Changes apply right away.";
/// What the Appearance pane says under its title.
const APPEARANCE: &str =
    "Theme, type and how dense the grid is. The preview updates as you change things.";

/// The tab: its title, what it is for where it says, and its options under
/// the heading of each group, then the preview where it has one. The rows
/// end at `limit` at the latest, and scroll when they are taller than that
/// leaves them.
#[allow(clippy::too_many_arguments)] // one pane: what it says, what it shows, what it pushes
fn content(
    ui: &mut Ui,
    (title, about): (&'static str, Option<&'static str>),
    (sections, preview): (&[(&'static str, &[OptionId])], bool),
    settings: &Settings,
    limit: f32,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    widgets::label(
        ui,
        TextRole::DialogTitle,
        &skin.say(title),
        palette.text,
        look,
    );
    if let Some(about) = about {
        ui.add_space(2.0);
        // Wrapped to the pane: a narrow sheet would otherwise grow as wide
        // as the line, and its rows with it.
        Text::one(look, TextRole::UiBody, &skin.say(about), palette.dim)
            .wrap(ui.available_width())
            .layout(ui.ctx())
            .label(ui);
    }
    ui.add_space(6.0 + 14.0);
    let Some((first, _)) = sections.first() else {
        return;
    };
    // The first heading stays over the rows as they scroll; the others
    // scroll with their rows, 14 under the last row before them.
    widgets::section_label(&skin.say(first), look, palette)
        .layout(ui.ctx())
        .label(ui);
    ui.add_space(2.0);
    let options: Vec<OptionId> = sections
        .iter()
        .flat_map(|(_, options)| options.iter().copied())
        .collect();
    let column = label_column(ui, &options, skin);
    let room = (limit - ui.cursor().top()).max(ROWS_MIN);
    egui::ScrollArea::vertical()
        .id_salt("rows")
        .max_height(room)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            // The last row stands on the footer's rule: one of its own
            // right over that would read as a double line.
            let last = options.len() - 1;
            let mut index = 0;
            for (nth, (heading, options)) in sections.iter().enumerate() {
                if nth > 0 {
                    ui.add_space(14.0);
                    widgets::section_label(&skin.say(heading), look, palette)
                        .layout(ui.ctx())
                        .label(ui);
                    ui.add_space(2.0);
                }
                for option in options.iter().copied() {
                    if option == OptionId::Theme {
                        tiles(ui, settings, skin, actions);
                    } else {
                        row(
                            ui,
                            option,
                            index < last,
                            column,
                            settings,
                            skin,
                            actions,
                            None,
                        );
                    }
                    index += 1;
                }
            }
            // Under the options, 14 under the last of them, the preview,
            // and 14 under it.
            if preview {
                let name = skin.say("Preview");
                ui.add_space(14.0);
                widgets::section_label(&name, look, palette)
                    .layout(ui.ctx())
                    .label(ui);
                ui.add_space(2.0);
                super::preview::show(ui, (&name, 0.0), settings, look, palette);
                ui.add_space(14.0);
            }
        });
}

/// The Theme section: a tile for each mode, a radio group named by the
/// option, the one set ringed in the accent, and what System does after
/// them. Each tile draws a window in its palette: the sidebar's colour
/// round the content's, System's half one and half the other.
fn tiles(ui: &mut Ui, settings: &Settings, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let name = skin.say(label(OptionId::Theme));
    let label_role = TextRole::UiBody;
    let line = label_role.row_height(ui.ctx(), look.faces);
    let tile = vec2(
        TILE.x + 2.0 * TILE_PAD,
        TILE_PAD + TILE.y + 6.0 + line + TILE_PAD,
    );
    let height = 6.0 + tile.y + 12.0 + 1.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.surface);
    // The tiles in a Ui of their own, whose node is their parent in the
    // accessibility tree: a radio group with the option's name.
    let place = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - 1.0));
    let group = ui.new_child(egui::UiBuilder::new().id_salt("theme").max_rect(place));
    group
        .ctx()
        .accesskit_node_builder(group.unique_id(), |node| {
            node.set_role(egui::accesskit::Role::RadioGroup);
            node.set_label(name.as_str());
        });
    let (light, dark) = (Palette::light(), Palette::dark());
    let mut x = rect.left();
    let top = rect.top() + 6.0;
    for (mode, word) in [
        (Mode::System, "System"),
        (Mode::Light, "Light"),
        (Mode::Dark, "Dark"),
    ] {
        let area = Rect::from_min_size(pos2(x, top), tile);
        x += tile.x + 12.0;
        let word = skin.say(word);
        let set = settings.theme.mode() == mode;
        let button = group.interact(area, group.id().with(word.as_str()), Sense::click());
        button.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, true, set, &word));
        focus::hint(&group, &button, area, Ring::Outer { radius: 10 });
        if button.clicked() && !set {
            let choice = match mode {
                Mode::System => ThemeChoice::System,
                Mode::Light => ThemeChoice::Light,
                Mode::Dark => ThemeChoice::Dark,
            };
            actions.push(Action::SetOption(OptionValue::Theme(choice)));
        }
        let painter = group.painter();
        if set {
            let ring = Stroke::new(2.0, palette.accent);
            painter.rect_stroke(
                area,
                CornerRadius::same(10),
                ring,
                egui::StrokeKind::Outside,
            );
        }
        let window = Rect::from_min_size(area.min + vec2(TILE_PAD, TILE_PAD), TILE);
        let content = Rect::from_min_max(
            window.min + vec2(22.0, 12.0),
            pos2(window.right() - 10.0, window.bottom()),
        );
        let halves = |rect: Rect| rect.split_left_right_at_x(window.center().x);
        let parts = match mode {
            Mode::System => {
                let ((left, right), (inner_left, inner_right)) = (halves(window), halves(content));
                vec![
                    (left, light.panel),
                    (right, dark.panel),
                    (inner_left, light.window),
                    (inner_right, dark.window),
                ]
            }
            Mode::Light => vec![(window, light.panel), (content, light.window)],
            Mode::Dark => vec![(window, dark.panel), (content, dark.window)],
        };
        let window_radius = CornerRadius::same(8);
        let content_radius = CornerRadius {
            nw: 6,
            ..CornerRadius::ZERO
        };
        for (index, (part, color)) in parts.into_iter().enumerate() {
            // The window's corners are its own; the content's only at its
            // top left, as the board draws it. System's halves are cut
            // down the middle.
            let mut radius = if part.top() == window.top() {
                window_radius
            } else {
                content_radius
            };
            if mode == Mode::System {
                let left = index % 2 == 0;
                if left {
                    radius.ne = 0;
                    radius.se = 0;
                } else {
                    radius.nw = 0;
                    radius.sw = 0;
                }
            }
            painter.rect_filled(part, radius, color);
        }
        let border = Stroke::new(1.0, palette.border);
        painter.rect_stroke(window, window_radius, border, egui::StrokeKind::Inside);
        let role = if set {
            TextRole::UiBodyStrong
        } else {
            label_role
        };
        let text = Text::one(look, role, &word, palette.text).layout(group.ctx());
        text.paint_left(
            painter,
            area.center().x - text.width() / 2.0,
            window.bottom() + 6.0 + line / 2.0,
        );
    }
    let hint = if cfg!(target_os = "windows") {
        "System follows Windows."
    } else {
        "System follows macOS, including Auto."
    };
    let hint = Text::one(look, TextRole::Secondary, &skin.say(hint), palette.dim).layout(ui.ctx());
    let at = x - 12.0 + 8.0;
    if at + hint.width() <= rect.right() {
        write(ui, at, top + tile.y / 2.0, &hint);
    }
}

/// How wide the rows' labels have it: `LABEL`, or what the widest small
/// print of `options` takes in a face that writes it wider, so that it
/// stays on one line. A sheet too narrow for that and the controls takes
/// from the labels, and the small print wraps.
pub(super) fn label_column(ui: &Ui, options: &[OptionId], skin: &Skin) -> f32 {
    let role = TextRole::Secondary;
    let widest = options
        .iter()
        .copied()
        .filter_map(small_print)
        .map(|print| role.width(ui.ctx(), skin.look.faces, &skin.say(print)))
        .fold(LABEL, f32::max);
    let room = ui.available_width() - GAP - CONTROLS;
    widest.ceil().min(room).max(0.0)
}

/// A button after a row's control: what it says, why it cannot be pressed
/// while it cannot, and what a press does.
pub(super) struct After<'a> {
    pub(super) text: &'a str,
    pub(super) disabled: Option<&'a str>,
    pub(super) action: Action,
}

/// One option's row: its label over its small print in a column `column`
/// wide, its control and the button `after` it, and the hint that shows
/// what the value does, over a rule when it is `ruled`.
#[allow(clippy::too_many_arguments)] // one row: its place, what it shows, what it pushes
pub(super) fn row(
    ui: &mut Ui,
    option: OptionId,
    ruled: bool,
    column: f32,
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
    after: Option<After<'_>>,
) {
    let Skin { look, palette, .. } = *skin;
    // Wrapped in its column too: a narrow sheet takes from the labels'
    // column, and a long label would run under the control.
    let name = Text::one(
        look,
        TextRole::UiBody,
        &said(ui.ctx(), look, skin.locale, label(option)),
        palette.text,
    )
    .wrap(column)
    .layout(ui.ctx());
    // A data font that is not installed is said in place of the small
    // print, where the row has the room for it.
    let print = match missing_font(option, skin) {
        Some(missing) => {
            let said = skin
                .say(super::FONT_NOT_INSTALLED)
                .replace("{name}", missing);
            Some((said, palette.warning))
        }
        None => small_print(option).map(|print| (skin.say(print), palette.dim)),
    };
    // What the row says under its label, or beside its control where it
    // says nothing under it, is what its controls are for: a screen reader
    // reads it as their description.
    let description = print
        .as_ref()
        .map(|(print, _)| print.clone())
        .or_else(|| super::helper(option).map(|words| skin.say(words)));
    let print = print.map(|(print, color)| {
        Text::one(look, TextRole::Secondary, &print, color)
            .wrap(column)
            .layout(ui.ctx())
    });
    // The label with its small print 2 under it.
    let words = name.height() + print.as_ref().map_or(0.0, |print| 2.0 + print.height());
    let inner = words.max(control_height(option, look));
    let height = ROW_PAD + inner + ROW_PAD + 1.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    if ruled {
        widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.surface);
    }
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
    let described = description.as_deref();
    control(&mut line, (option, described), settings, skin, actions);
    if let Some(after) = after {
        line.add_space(HINT_GAP);
        let mut button = widgets::ButtonSpec::new(after.text);
        if let Some(reason) = after.disabled {
            button = button.disabled(reason);
        }
        if button
            .show(&mut line, look.control_height, look, palette)
            .clicked()
        {
            actions.push(after.action);
        }
    }
    // Tab reaches a control of a row that is scrolled out of view, where
    // its ring is cut with it: the row comes to the keyboard. Jump, not
    // animate: the next Tab may come at once.
    if keyboard_came_to(ui, place) {
        ui.scroll_to_rect_animation(rect, None, egui::style::ScrollAnimation::none());
    }
    let left = line.min_rect().right() + HINT_GAP;
    hint(ui, pos2(left, middle), rect.right(), option, settings, skin);
}

/// The data font the settings name, on its row, where it is no face the
/// look has: the default is drawn in its place.
fn missing_font<'a>(option: OptionId, skin: &Skin<'a>) -> Option<&'a str> {
    let plex = skin.look.faces.typeface == Typeface::Plex;
    skin.fonts
        .missing
        .filter(|_| option == OptionId::Font && plex)
}

/// Whether the keyboard came in this frame to a control in `place`, on the
/// layer `ui` draws on: the open menu's entries hang over the rows under
/// it, on a layer of their own.
fn keyboard_came_to(ui: &Ui, place: Rect) -> bool {
    ui.memory(|memory| memory.focused())
        .and_then(|id| ui.ctx().read_response(id))
        .is_some_and(|control| {
            control.gained_focus()
                && control.layer_id == ui.layer_id()
                && place.contains(control.rect.center())
        })
}

/// How tall `option`'s control is: its row is at least as tall.
fn control_height(option: OptionId, look: &Look) -> f32 {
    match option {
        OptionId::PageSize | OptionId::Tz | OptionId::OnLaunch => look.control_height,
        // The track has 2 over its segments and under them.
        OptionId::Timestamps | OptionId::GroupDigits => SEGMENT + 4.0,
        OptionId::RowHeight | OptionId::TableList | OptionId::EnvBar => SEGMENT + 4.0,
        OptionId::InterfaceSize => SEGMENT + 4.0,
        OptionId::Font | OptionId::FontSize => look.control_height,
        OptionId::NewTab
        | OptionId::RunScope
        | OptionId::EditorKeys
        | OptionId::Indent
        | OptionId::Autocomplete
        | OptionId::Keywords
        | OptionId::Commas => SEGMENT + 4.0,
        OptionId::RowLimit | OptionId::Timeout | OptionId::HistoryKeep => look.control_height,
        OptionId::ConfirmUnsafeWrites
        | OptionId::ClosePairs
        | OptionId::LineNumbers
        | OptionId::HighlightLine
        | OptionId::Wrap => SWITCH,
        OptionId::RestoreTabs
        | OptionId::ValueTags
        | OptionId::ProductionReadOnly
        | OptionId::ConfirmCloseEdits
        | OptionId::AgentsOn
        | OptionId::AgentsNotify
        | OptionId::Zebra
        | OptionId::ColumnTypes => SWITCH,
        OptionId::LogDays | OptionId::IdleDisconnect => look.control_height,
        // The Theme section is tiles, drawn on their own: no row's.
        OptionId::Theme => 0.0,
    }
}

/// What changes `option`: a menu of the page sizes, two segments, or a
/// switch, each with `description` (the row's small print) for screen
/// readers.
fn control(
    ui: &mut Ui,
    (option, description): (OptionId, Option<&str>),
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let name = said(ui.ctx(), look, skin.locale, label(option));
    // Two or three words of segments, picked into a value.
    let mut segmented = |words: &[&'static str], selected: usize| {
        let words: Vec<String> = words.iter().map(|word| skin.say(word)).collect();
        segments(ui, (&name, description), &words, selected, skin)
    };
    match option {
        OptionId::NewTab => {
            let selected = usize::from(settings.new_tab == NewTab::Connection);
            let words = ["Read-only transaction", "Follow connection"];
            if let Some(picked) = segmented(&words, selected) {
                let choice = [NewTab::ReadOnly, NewTab::Connection][picked];
                actions.push(Action::SetOption(OptionValue::NewTab(choice)));
            }
        }
        OptionId::RunScope => {
            let selected = usize::from(settings.run_scope == RunScope::All);
            if let Some(picked) = segmented(&["Statement at cursor", "Whole tab"], selected) {
                let scope = [RunScope::Statement, RunScope::All][picked];
                actions.push(Action::SetOption(OptionValue::RunScope(scope)));
            }
        }
        OptionId::EditorKeys => {
            let keys = settings.editor_keys_in(look.terminal);
            let selected = usize::from(keys == EditorKeys::Vim);
            if let Some(picked) = segmented(&["Standard", "Vim"], selected) {
                let keys = [EditorKeys::Standard, EditorKeys::Vim][picked];
                actions.push(Action::SetOption(OptionValue::EditorKeys(Some(keys))));
            }
        }
        OptionId::Indent => {
            let selected = Indent::ALL
                .iter()
                .position(|indent| *indent == settings.indent)
                .unwrap_or(0);
            if let Some(picked) = segmented(&["2 spaces", "4 spaces", "Tab"], selected) {
                actions.push(Action::SetOption(OptionValue::Indent(Indent::ALL[picked])));
            }
        }
        OptionId::Autocomplete => {
            // The middle one names the key that asks for a list.
            let complete = crate::ui::keys::written(ui.ctx(), look, Command::Complete);
            let manual = skin.say("On {key}").replace("{key}", complete.as_str());
            let words = [skin.say("As you type"), manual, skin.say("Off")];
            let selected = Autocomplete::ALL
                .iter()
                .position(|choice| *choice == settings.autocomplete)
                .unwrap_or(0);
            if let Some(picked) = segments(ui, (&name, description), &words, selected, skin) {
                let choice = Autocomplete::ALL[picked];
                actions.push(Action::SetOption(OptionValue::Autocomplete(choice)));
            }
        }
        OptionId::Keywords => {
            let case = settings.keywords_in(look.terminal);
            let selected = KeywordCase::ALL
                .iter()
                .position(|choice| *choice == case)
                .unwrap_or(0);
            if let Some(picked) = segmented(&["UPPERCASE", "lowercase", "Keep"], selected) {
                let case = KeywordCase::ALL[picked];
                actions.push(Action::SetOption(OptionValue::Keywords(Some(case))));
            }
        }
        OptionId::Commas => {
            let selected = usize::from(settings.commas == Commas::Leading);
            if let Some(picked) = segmented(&["Trailing", "Leading"], selected) {
                let commas = [Commas::Trailing, Commas::Leading][picked];
                actions.push(Action::SetOption(OptionValue::Commas(commas)));
            }
        }
        OptionId::RowLimit => {
            let shown = sql_editor::limit_words(settings.sql_limit, look, skin.locale);
            let choices = Settings::sql_limits_with(settings.sql_limit)
                .into_iter()
                .map(|limit| {
                    let text = sql_editor::limit_words(limit, look, skin.locale);
                    (
                        text,
                        limit == settings.sql_limit,
                        OptionValue::RowLimit(limit),
                    )
                });
            choice_menu(
                ui,
                ("row-limit", &name, description),
                &shown,
                choices,
                skin,
                actions,
            );
        }
        OptionId::Timeout => {
            let words = |secs: Option<u32>| match secs {
                Some(secs) => sql_editor::time_words(secs, false, skin.locale),
                None => skin.say("Off"),
            };
            let current = settings.sql_timeout_secs;
            let choices = Settings::sql_timeouts_with(current)
                .into_iter()
                .map(|secs| (words(secs), secs == current, OptionValue::Timeout(secs)));
            let shown = words(current);
            choice_menu(
                ui,
                ("timeout", &name, description),
                &shown,
                choices,
                skin,
                actions,
            );
        }
        OptionId::HistoryKeep => {
            let words = |keep: u32| match keep {
                0 => skin.say("Don't keep"),
                keep => skin
                    .say("Last {n} queries")
                    .replace("{n}", &group_digits(u64::from(keep))),
            };
            let current = settings.history_keep;
            let choices = Settings::history_keeps_with(current)
                .into_iter()
                .map(|keep| (words(keep), keep == current, OptionValue::HistoryKeep(keep)));
            let shown = words(current);
            choice_menu(
                ui,
                ("history", &name, description),
                &shown,
                choices,
                skin,
                actions,
            );
        }
        // On is a number on each line: as they are, or absolute where
        // they were off. A stored `relative` shows on and stays.
        OptionId::LineNumbers => {
            let numbers = settings.line_numbers_in(look.terminal);
            if widgets::toggle(ui, numbers.shown(), &name, description, palette).clicked() {
                let to = if numbers.shown() {
                    LineNumbers::Off
                } else {
                    LineNumbers::Absolute
                };
                actions.push(Action::SetOption(OptionValue::LineNumbers(Some(to))));
            }
        }
        OptionId::PageSize => {
            let written = |size: u32| group_digits(u64::from(size));
            let size = written(settings.page_size);
            let dropdown = menu::Dropdown::new("page-size", &size, MENU).description(description);
            let menu = dropdown.show(ui, look, palette, |ui| {
                for size in page_sizes(settings.page_size) {
                    let chosen = size == settings.page_size;
                    // Named as the SQL editor's Limit menu names its
                    // entries: what it sets, and to what.
                    let text = written(size);
                    let entry_name = format!("{name} {text}");
                    let entry = menu::Item::choice(&text, chosen)
                        .name(&entry_name)
                        .show(ui, look, palette);
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
        OptionId::Tz => zone_menu(ui, (&name, description), settings, skin, actions),
        OptionId::OnLaunch => launch_menu(ui, (&name, description), settings, skin, actions),
        OptionId::Timestamps => {
            let words = [skin.say("To the second"), skin.say("Full precision")];
            let selected = usize::from(settings.timestamps == Timestamps::Full);
            if let Some(picked) = segments(ui, (&name, description), &words, selected, skin) {
                let value = [Timestamps::Second, Timestamps::Full][picked];
                actions.push(Action::SetOption(OptionValue::Timestamps(value)));
            }
        }
        OptionId::GroupDigits => {
            // A number as each value writes it: no language has others.
            let words = ["1,240.50".to_owned(), "1240.50".to_owned()];
            let selected = usize::from(!settings.group_digits.unwrap_or_default());
            if let Some(picked) = segments(ui, (&name, description), &words, selected, skin) {
                actions.push(Action::SetOption(OptionValue::GroupDigits(Some(
                    picked == 0,
                ))));
            }
        }
        OptionId::RowHeight => {
            // Each with its rows' height in this look, as the board writes
            // them: without the rule under a row (28, 36 and 44 on macOS).
            let words = RowHeight::ALL.map(|height| {
                let word = match height {
                    RowHeight::Compact => "Compact",
                    RowHeight::Regular => "Regular",
                    RowHeight::Roomy => "Roomy",
                };
                let rows = Look {
                    density: height,
                    ..*look
                }
                .row();
                format!("{} · {}", skin.say(word), rows - 1.0)
            });
            let selected = RowHeight::ALL
                .iter()
                .position(|height| *height == settings.row_height)
                .unwrap_or(1);
            if let Some(picked) = segments(ui, (&name, description), &words, selected, skin) {
                actions.push(Action::SetOption(OptionValue::RowHeight(
                    RowHeight::ALL[picked],
                )));
            }
        }
        OptionId::InterfaceSize => {
            // Each with the size of its body text, as the board writes
            // them.
            let words = InterfaceSize::ALL.map(|size| {
                let word = match size {
                    InterfaceSize::Small => "Small",
                    InterfaceSize::Default => "Default",
                    InterfaceSize::Large => "Large",
                };
                format!("{} · {}", skin.say(word), size.points())
            });
            let selected = InterfaceSize::ALL
                .iter()
                .position(|size| *size == settings.interface_size)
                .unwrap_or(1);
            if let Some(picked) = segments(ui, (&name, description), &words, selected, skin) {
                actions.push(Action::SetOption(OptionValue::InterfaceSize(
                    InterfaceSize::ALL[picked],
                )));
            }
        }
        // The face's menu where the look draws data in a face of its own
        // (macOS), then the size's. Windows draws its data in Inter, the
        // interface's face: the size's alone.
        OptionId::Font => {
            if look.faces.typeface == Typeface::Plex {
                let width =
                    (ui.available_width() - MENU_GAP - SIZE_MENU).clamp(FONT_MENU_MIN, FONT_MENU);
                font_menu(ui, (&name, width, description), settings, skin, actions);
                ui.add_space(MENU_GAP);
            }
            size_menu(ui, description, settings, skin, actions);
        }
        OptionId::FontSize => size_menu(ui, description, settings, skin, actions),
        OptionId::TableList => {
            let words = [skin.say("Grouped by prefix"), skin.say("Flat")];
            let selected = usize::from(settings.table_list.flat());
            if let Some(picked) = segments(ui, (&name, description), &words, selected, skin) {
                actions.push(Action::SetOption(OptionValue::TableList(TableList::of(
                    picked == 1,
                ))));
            }
        }
        OptionId::EnvBar => {
            let words = [skin.say("Tint + stripe"), skin.say("Stripe only")];
            let selected = usize::from(settings.env_bar == EnvBar::Label);
            if let Some(picked) = segments(ui, (&name, description), &words, selected, skin) {
                let bar = [EnvBar::Tint, EnvBar::Label][picked];
                actions.push(Action::SetOption(OptionValue::EnvBar(bar)));
            }
        }
        OptionId::RestoreTabs
        | OptionId::ValueTags
        | OptionId::ProductionReadOnly
        | OptionId::ConfirmCloseEdits
        | OptionId::AgentsOn
        | OptionId::AgentsNotify
        | OptionId::Zebra
        | OptionId::ColumnTypes
        | OptionId::ConfirmUnsafeWrites
        | OptionId::ClosePairs
        | OptionId::HighlightLine
        | OptionId::Wrap => {
            let on = super::is_on(&settings.value(option));
            if widgets::toggle(ui, on, &name, description, palette).clicked()
                && let Some(flipped) = settings.flipped(option)
            {
                actions.push(Action::SetOption(flipped));
            }
        }
        OptionId::IdleDisconnect => {
            let shown = super::idle_text(skin.locale, settings.idle_disconnect);
            let dropdown = menu::Dropdown::new("idle", &shown, MENU).description(description);
            let menu = dropdown.show(ui, look, palette, |ui| {
                for choice in Idle::ALL {
                    let text = super::idle_text(skin.locale, choice);
                    let entry_name = format!("{name} {text}");
                    let entry = menu::Item::choice(&text, choice == settings.idle_disconnect)
                        .name(&entry_name)
                        .show(ui, look, palette);
                    if entry.clicked() {
                        actions.push(Action::SetOption(OptionValue::IdleDisconnect(choice)));
                        ui.close();
                    }
                }
            });
            menu.response.widget_info(|| {
                let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, &name);
                info.current_text_value = Some(shown.clone());
                info
            });
        }
        OptionId::LogDays => {
            let shown = super::days(skin.locale, settings.mcp_log_days);
            let dropdown = menu::Dropdown::new("log-days", &shown, MENU).description(description);
            let menu = dropdown.show(ui, look, palette, |ui| {
                for days in log_days(settings.mcp_log_days) {
                    let text = super::days(skin.locale, days);
                    let entry_name = format!("{name} {text}");
                    let entry = menu::Item::choice(&text, days == settings.mcp_log_days)
                        .name(&entry_name)
                        .show(ui, look, palette);
                    if entry.clicked() {
                        actions.push(Action::SetOption(OptionValue::LogDays(days)));
                        ui.close();
                    }
                }
            });
            menu.response.widget_info(|| {
                let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, &name);
                info.current_text_value = Some(shown.clone());
                info
            });
        }
        // The Theme section is tiles, drawn on their own: no row's.
        OptionId::Theme => {}
    }
}

/// The data font's menu: the bundled faces, then the installed monospace
/// families. An installed font reads as the settings name it, drawn or on
/// its way; a name that nothing draws reads as the default drawn in its
/// place, which the row's small print says.
fn font_menu(
    ui: &mut Ui,
    (name, width, description): (&str, f32, Option<&str>),
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let named = DataFace::of(settings.font.as_deref());
    let installed = named.err().filter(|_| skin.fonts.missing.is_none());
    let bundled = named.unwrap_or_default();
    let shown = installed.or(bundled.name()).unwrap_or_default();
    let dropdown = menu::Dropdown::new("data-font", shown, width).description(description);
    let menu = dropdown.show(ui, look, palette, |ui| {
        let mut pick = |ui: &mut Ui, text: &str, current: bool, font: Option<String>| {
            let entry_name = format!("{name} {text}");
            let entry = menu::Item::choice(text, current)
                .name(&entry_name)
                .show(ui, look, palette);
            if entry.clicked() {
                actions.push(Action::SetOption(OptionValue::Font(font)));
                ui.close();
            }
        };
        for face in DataFace::BUNDLED {
            let text = face.name().unwrap_or_default();
            // The default is no value: the file leaves it out.
            let font = (face != DataFace::default()).then(|| text.to_owned());
            pick(ui, text, installed.is_none() && face == bundled, font);
        }
        if !skin.fonts.installed.is_empty() {
            menu::heading(ui, &skin.say("Installed"), look, palette);
        }
        for family in skin.fonts.installed {
            let current = installed.is_some_and(|name| name.eq_ignore_ascii_case(family));
            pick(ui, family, current, Some(family.clone()));
        }
    });
    menu.response.widget_info(|| {
        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, name);
        info.current_text_value = Some(shown.to_owned());
        info
    });
}

/// The data's size: a menu of every size macOS and Windows draw, by half
/// points.
fn size_menu(
    ui: &mut Ui,
    description: Option<&str>,
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let name = skin.say(label(OptionId::FontSize));
    let points = |size: crate::settings::FontSize| format!("{} pt", size.text());
    let drawn = settings.font_size_in(false);
    let shown = points(drawn);
    let dropdown =
        menu::Dropdown::new("data-font-size", &shown, SIZE_MENU).description(description);
    let menu = dropdown.show(ui, look, palette, |ui| {
        for size in FontSizes::of(false).all() {
            let text = points(size);
            let entry_name = format!("{name} {text}");
            let entry = menu::Item::choice(&text, size == drawn)
                .name(&entry_name)
                .show(ui, look, palette);
            if entry.clicked() {
                actions.push(Action::SetOption(OptionValue::FontSize(Some(size))));
                ui.close();
            }
        }
    });
    menu.response.widget_info(|| {
        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, &name);
        info.current_text_value = Some(shown.clone());
        info
    });
}

/// The days the log's menu offers: the list, with the days in use in their
/// place when the file set others.
fn log_days(current: u32) -> Vec<u32> {
    let mut days = Settings::LOG_DAYS.to_vec();
    if !days.contains(&current) {
        days.push(current);
        days.sort_unstable();
    }
    days
}

/// A segmented control of `words` for the option called `option`, the one
/// at `selected` chosen: a radio group named by the option and described
/// by `description`. Returns the index of a segment picked this frame.
/// The segments are of one width: the widest word's, with 10 at each side.
fn segments(
    ui: &mut Ui,
    (option, description): (&str, Option<&str>),
    words: &[String],
    selected: usize,
    skin: &Skin,
) -> Option<usize> {
    let Skin { look, palette, .. } = *skin;
    // The role `widgets::segmented` writes its words in.
    let role = TextRole::FieldLabel;
    let widest = words
        .iter()
        .map(|word| role.width(ui.ctx(), look.faces, word))
        .fold(0.0, f32::max);
    let size = vec2((widest + 20.0).ceil(), SEGMENT);
    // The row's label is beside the control, not a segment's own: each is
    // named by the option and its own word, as the terminal look's
    // choices are.
    let names: Vec<String> = words
        .iter()
        .map(|word| format!("{option}: {word}"))
        .collect();
    let parts: Vec<Segment> = words
        .iter()
        .zip(&names)
        .map(|(text, name)| Segment::Named { text, name })
        .collect();
    let group = widgets::Group {
        name: option,
        description,
    };
    widgets::segmented(ui, &parts, selected, size, Some(group), look, palette)
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

/// What a launch opens: the last session, Connections, or a saved
/// connection, each with where it points beside its name.
fn launch_menu(
    ui: &mut Ui,
    (name, description): (&str, Option<&str>),
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let shown = super::launch_text(skin.locale, &settings.on_launch, skin.connections);
    // Closed, the menu tells two connections of one name apart by where
    // each points, as the open one does.
    let place = match &settings.on_launch {
        Launch::Connection { id, .. } => skin
            .connections
            .iter()
            .find(|saved| saved.id == *id)
            .map(super::place)
            .filter(|place| !place.is_empty()),
        Launch::Restore | Launch::Connections => None,
    };
    let value = match place {
        Some(place) => format!("{shown}, {place}"),
        None => shown.clone(),
    };
    let dropdown = menu::Dropdown::new("launch", &shown, ZONE_MENU).description(description);
    let menu = dropdown.show(ui, look, palette, |ui| {
        for launch in super::launch_choices(skin.connections) {
            let text = super::launch_text(skin.locale, &launch, skin.connections);
            let place = match &launch {
                Launch::Connection { id, .. } => skin
                    .connections
                    .iter()
                    .find(|saved| saved.id == *id)
                    .map(super::place)
                    .unwrap_or_default(),
                Launch::Restore | Launch::Connections => String::new(),
            };
            let entry_name = format!("{name} {text} {place}");
            let mut entry =
                menu::Item::choice(&text, launch == settings.on_launch).name(entry_name.trim_end());
            if !place.is_empty() {
                entry = entry.detail(&place);
            }
            if entry.show(ui, look, palette).clicked() {
                actions.push(Action::SetOption(OptionValue::OnLaunch(launch)));
                ui.close();
            }
        }
    });
    menu.response.widget_info(|| {
        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, name);
        info.current_text_value = Some(value.clone());
        info
    });
}

/// The zone's menu: the computer's zone and UTC, then every zone of the
/// database, which a search field over them narrows. What was typed there
/// is the field's own, kept in egui's memory while the menu is open.
fn zone_menu(
    ui: &mut Ui,
    (name, description): (&str, Option<&str>),
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let shown = zone_text(skin.locale, &settings.tz, skin.system);
    let search_id = ui.id().with("zone-search");
    let dropdown = menu::Dropdown::new("zone", &shown, ZONE_MENU).description(description);
    let menu = dropdown.show(ui, look, palette, |ui| {
        let mut pick = |ui: &mut Ui, zone: Zone| {
            let text = zone_text(skin.locale, &zone, skin.system);
            let entry_name = format!("{name} {text}");
            let entry = menu::Item::choice(&text, zone == settings.tz)
                .name(&entry_name)
                .show(ui, look, palette);
            if entry.clicked() {
                actions.push(Action::SetOption(OptionValue::Tz(zone)));
                ui.close();
            }
        };
        pick(ui, Zone::System);
        pick(ui, Zone::Utc);
        let mut typed: String = ui.data(|data| data.get_temp(search_id)).unwrap_or_default();
        let search = skin.say("Search zones");
        let field = widgets::search_field(ui, &mut typed, &search, look);
        widgets::add_search(ui, field, look)
            .widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, &search));
        ui.data_mut(|data| data.insert_temp(search_id, typed.clone()));
        let typed = typed.to_lowercase();
        egui::ScrollArea::vertical()
            .max_height(ZONE_LIST)
            .show(ui, |ui| {
                for zone in skin
                    .zones
                    .iter()
                    .filter(|zone| typed.is_empty() || zone.to_lowercase().contains(&typed))
                {
                    pick(ui, Zone::Named(zone.clone()));
                }
            });
    });
    if menu.inner.is_none() {
        // Closed: the next opening starts from every zone.
        ui.data_mut(|data| data.remove::<String>(search_id));
    }
    // The row's label is beside it, not its own: named for screen readers,
    // with the zone as its value.
    menu.response.widget_info(|| {
        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, name);
        info.current_text_value = Some(shown.clone());
        info
    });
}

/// What the row says after `option`'s control, from `at` on its middle: a
/// sample of what the value does, or what it does not do. Whole or left
/// out: a hint that does not end before `right` tells nothing cut short.
fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settings, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let text = match option {
        OptionId::PageSize | OptionId::Tz => return,
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
        OptionId::RunScope => Text::one(
            look,
            TextRole::Secondary,
            &skin.say("A selection always wins"),
            palette.dim,
        ),
        // What the Appearance pane's rows say is their small print, and its
        // tiles have their own line.
        OptionId::Theme
        | OptionId::OnLaunch
        | OptionId::RestoreTabs
        | OptionId::ProductionReadOnly
        | OptionId::ConfirmCloseEdits
        | OptionId::IdleDisconnect
        | OptionId::AgentsOn
        | OptionId::LogDays
        | OptionId::AgentsNotify
        | OptionId::InterfaceSize
        | OptionId::Font
        | OptionId::FontSize
        | OptionId::RowHeight
        | OptionId::Zebra
        | OptionId::ColumnTypes
        | OptionId::TableList
        | OptionId::EnvBar
        | OptionId::NewTab
        | OptionId::RowLimit
        | OptionId::Timeout
        | OptionId::ConfirmUnsafeWrites
        | OptionId::EditorKeys
        | OptionId::Indent
        | OptionId::Autocomplete
        | OptionId::ClosePairs
        | OptionId::LineNumbers
        | OptionId::HighlightLine
        | OptionId::Wrap
        | OptionId::Keywords
        | OptionId::Commas
        | OptionId::HistoryKeep => return,
    };
    let text = text.layout(ui.ctx());
    if at.x + text.width() <= right {
        write(ui, at.x, at.y, &text);
    }
}

/// A menu of `choices` (what each says, whether it is the one set, and the
/// value it sets) named `name`, `shown` closed, with `description` for
/// screen readers. `id` tells it from the pane's other menus.
fn choice_menu(
    ui: &mut Ui,
    (id, name, description): (&str, &str, Option<&str>),
    shown: &str,
    choices: impl Iterator<Item = (String, bool, OptionValue)>,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let dropdown = menu::Dropdown::new(id, shown, MENU).description(description);
    let menu = dropdown.show(ui, look, palette, |ui| {
        for (text, chosen, value) in choices {
            let entry_name = format!("{name} {text}");
            let entry = menu::Item::choice(&text, chosen)
                .name(&entry_name)
                .show(ui, look, palette);
            if entry.clicked() {
                actions.push(Action::SetOption(value));
                // egui closes the menu on a pointer's click; a pick by key
                // or by a screen reader closes it here.
                ui.close();
            }
        }
    });
    menu.response.widget_info(|| {
        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, name);
        info.current_text_value = Some(shown.to_owned());
        info
    });
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

/// What the link that shows the file in the file manager says: the words
/// of the system the app was built for, as the program it starts is that
/// system's. English.
fn reveal_words() -> &'static str {
    if cfg!(target_os = "macos") {
        "Reveal in Finder"
    } else if cfg!(windows) {
        "Show in Explorer"
    } else {
        "Show in folder"
    }
}

/// What the footer says while the app ignores `count` lines of the file,
/// wrapped at `width`.
fn ignored_lines(ui: &Ui, count: usize, width: f32, skin: &Skin) -> Option<Laid> {
    if count == 0 {
        return None;
    }
    let lines = u32::try_from(count).unwrap_or(u32::MAX);
    let said = ngettext(
        skin.locale,
        "line in the file could not be read and was ignored",
        "lines in the file could not be read and were ignored",
        lines,
    );
    let said = format!("{count} {said}");
    let text = Text::one(skin.look, TextRole::Secondary, &said, skin.palette.warning);
    Some(text.wrap(width).layout(ui.ctx()))
}

/// How tall the footer is: its rule, and its line between its padding,
/// with what it says of `ignored` lines over that.
fn footer_height(ui: &Ui, ignored: Option<&Laid>, skin: &Skin) -> f32 {
    let line = TextRole::Secondary.row_height(ui.ctx(), skin.look.faces);
    let ignored = ignored.map_or(0.0, |ignored| ignored.height() + WARNING_GAP);
    (1.0 + FOOT_PAD + ignored + line + FOOT_PAD).ceil()
}

/// What Reset to defaults asks on `pane`. English.
fn reset_question(pane: SettingsPane) -> &'static str {
    match pane {
        SettingsPane::General => "Reset all General settings to their defaults?",
        SettingsPane::Editor => "Reset all Editor settings to their defaults?",
        SettingsPane::Appearance | SettingsPane::Agents => "Reset every option on this tab?",
    }
}

/// The footer, in `rect` under the content: where the settings file is
/// and, at the right, what can be done with it. While the pane in
/// `asking` asks it, the question Reset to defaults asks stands where
/// those links were.
fn footer(
    ui: &Ui,
    rect: Rect,
    app: &App,
    asking: Option<SettingsPane>,
    ignored: Option<&Laid>,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::Secondary;
    // The sheet's own corner, inside its border.
    let corners = CornerRadius {
        se: look.dialog_radius.saturating_sub(1),
        ..CornerRadius::ZERO
    };
    ui.painter().rect_filled(rect, corners, palette.panel);
    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
    let (left, right) = (rect.left() + SIDE, rect.right() - SIDE);
    let mut top = rect.top() + 1.0 + FOOT_PAD;
    if let Some(ignored) = ignored {
        write(ui, left, top + ignored.middle(), ignored);
        top += ignored.height() + WARNING_GAP;
    }
    let middle = top + role.row_height(ui.ctx(), look.faces) / 2.0;

    // Whether the question stood here in the frame before and is answered
    // now. Noted in every frame that asks, and read once.
    let asked = ui.id().with("asked");
    let now = ui.ctx().cumulative_frame_nr();
    let answered = if asking.is_some() {
        ui.data_mut(|data| data.insert_temp(asked, now));
        false
    } else {
        ui.data_mut(|data| data.remove_temp::<u64>(asked)) == Some(now.saturating_sub(1))
    };

    // The right side first: what it leaves is the path's.
    let parts = if let Some(pane) = asking {
        vec![
            (skin.say(reset_question(pane)), None),
            (
                skin.say("Reset"),
                Some((palette.danger, Action::ConfirmResetSettings(true))),
            ),
            (
                skin.say("Cancel"),
                Some((palette.accent, Action::ConfirmResetSettings(false))),
            ),
        ]
    } else {
        vec![
            (
                skin.say(reveal_words()),
                Some((palette.accent, Action::RevealSettingsFile)),
            ),
            (
                skin.say("Export…"),
                Some((palette.accent, Action::ExportSettings)),
            ),
            (
                skin.say("Reset to defaults"),
                Some((palette.danger, Action::ResetSettings)),
            ),
        ]
    };
    let widths: Vec<f32> = parts
        .iter()
        .map(|(text, _)| role.width(ui.ctx(), look.faces, text).ceil())
        .collect();
    let taken = widths.iter().sum::<f32>() + FOOT_GAP * (parts.len() - 1) as f32;
    let first = right - taken;
    let mut x = first;
    let count = parts.len();
    for (index, ((text, linked), width)) in parts.into_iter().zip(widths).enumerate() {
        match linked {
            None => {
                widgets::paint_label(ui, x, middle, Text::one(look, role, &text, palette.text));
            }
            Some((color, action)) => {
                // Known by its place from the right, which Reset to
                // defaults shares with Cancel: the keyboard that asked is
                // on the answer that changes nothing. Reset has an id no
                // other link has: one it shared with Export would leave
                // the keyboard there, and the key still held would export.
                let id = match action {
                    Action::ConfirmResetSettings(true) => ui.id().with("reset"),
                    _ => ui.id().with(("link", count - 1 - index)),
                };
                let link = link(ui, id, pos2(x, middle), width, &text, color, skin);
                // Whatever answered, the keyboard is back on the link that
                // asked: Reset went with the question, and Escape takes
                // the keyboard from wherever it is.
                if answered && matches!(action, Action::ResetSettings) {
                    link.request_focus();
                }
                if pressed(ui, &link) {
                    actions.push(action);
                }
            }
        }
        x += width + FOOT_GAP;
    }

    // "Stored in" whole or left out, then as much of the path as there is
    // room for before the links.
    let stored = Text::one(look, role, &skin.say("Stored in"), palette.secondary).layout(ui.ctx());
    let room = first - FOOT_GAP;
    if left + stored.width() > room {
        return;
    }
    write(ui, left, middle, &stored);
    let at = left + stored.width() + WORD_GAP;
    let path = path_shown(app);
    let mono = TextRole::MonoSecondary;
    let fits = |text: &str| at + mono.width(ui.ctx(), look.faces, text) <= room;
    if let Some(cut) = cut_start(&path, fits) {
        let width = widgets::paint_text(
            ui,
            at,
            middle,
            Text::one(look, mono, &cut, palette.secondary),
        );
        // A screen reader is told all of it.
        let place = Rect::from_center_size(pos2(at + width / 2.0, middle), vec2(width, LINK));
        widgets::announce(ui, place, &path);
    }
}

/// A link of the footer: `text`, `width` wide from `at` on its middle, in
/// `color`. Tab reaches it, and Space and Enter press it: [`pressed`] says
/// when.
fn link(
    ui: &Ui,
    id: egui::Id,
    at: egui::Pos2,
    width: f32,
    text: &str,
    color: egui::Color32,
    skin: &Skin,
) -> egui::Response {
    let Skin { look, palette, .. } = *skin;
    let rect = Rect::from_min_size(pos2(at.x, at.y - LINK / 2.0), vec2(width, LINK));
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, text));
    // The accent has a colour of its own under the pointer. The danger
    // colour has none.
    let color = if response.hovered() && color == palette.accent {
        palette.accent_hover
    } else {
        color
    };
    let role = TextRole::Secondary;
    widgets::paint_text(ui, at.x, at.y, Text::one(look, role, text, color));
    focus::hint(ui, &response, rect, Ring::Outer { radius: 3 });
    response
}

/// Whether `link` was pressed in this frame: by the pointer, by a screen
/// reader, or by a key that was not down before. egui counts every repeat
/// of a held Space or Enter as a press, and each would start another file
/// manager or open another dialog to save in.
fn pressed(ui: &Ui, link: &egui::Response) -> bool {
    if !link.clicked() {
        return false;
    }
    if link.clicked_by(egui::PointerButton::Primary) {
        return true;
    }
    let (mut fresh, mut repeated) = (false, false);
    ui.input(|input| {
        for event in &input.events {
            if let egui::Event::Key {
                key: egui::Key::Space | egui::Key::Enter,
                pressed: true,
                repeat,
                ..
            } = event
            {
                if *repeat {
                    repeated = true;
                } else {
                    fresh = true;
                }
            }
        }
    });
    // A screen reader's press comes with no key at all.
    fresh || !repeated
}

/// `text` if it `fits`, or the most of its end that does after `…`: a
/// path keeps the file's name. None when no character of it fits.
fn cut_start(text: &str, fits: impl Fn(&str) -> bool) -> Option<String> {
    if fits(text) {
        return Some(text.to_owned());
    }
    text.char_indices()
        .skip(1)
        .map(|(at, _)| format!("…{}", &text[at..]))
        .find(|cut| fits(cut))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_too_long_for_its_room_is_cut_at_its_start() {
        let path = "~/.config/tabletist/settings.toml";
        let within = |room: usize| cut_start(path, |text| text.chars().count() <= room);
        assert_eq!(within(40).as_deref(), Some(path));
        assert_eq!(within(33).as_deref(), Some(path));
        assert_eq!(within(24).as_deref(), Some("…tabletist/settings.toml"));
        assert_eq!(within(2).as_deref(), Some("…l"));
        assert_eq!(within(1), None);
        // Cut between characters, never inside one.
        let cut = cut_start("~/Büro/settings.toml", |text| text.chars().count() <= 18);
        assert_eq!(cut.as_deref(), Some("…üro/settings.toml"));
    }

    #[test]
    fn a_size_that_is_not_in_the_list_takes_its_place_among_the_others() {
        assert_eq!(log_days(30), Settings::LOG_DAYS);
        assert_eq!(log_days(14), [7, 14, 30, 90, 365]);
        assert_eq!(page_sizes(1_000), Settings::PAGE_SIZES);
        assert_eq!(page_sizes(300), [100, 300, 500, 1_000, 5_000, 10_000]);
        assert_eq!(page_sizes(20), [20, 100, 500, 1_000, 5_000, 10_000]);
    }
}
