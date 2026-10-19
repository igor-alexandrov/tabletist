//! The Settings window as the terminal look draws it: a screen over the
//! whole window. A header, the app's notice while it has one, a nav of its
//! panes, the pane's rows under a cursor, the settings file beside them
//! where the window has the room (`file_pane.rs` draws it in the place it
//! is given here), and the screen's keys in the footer.

use egui::emath::GuiRounding as _;
use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2};

use crate::app::{App, PinState};
use crate::i18n::{Locale, gettext, ngettext};
use crate::keymap::Command;
use crate::model::{Action, SettingsDialog, SettingsPane, SettingsRow};
use crate::settings::{Idle, OptionId, OptionValue, Settings};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::value_tags::terminal_slots;
use crate::ui::widgets::{self, ButtonSpec};

use super::{Said, choices, file_pane, opened, sample_timestamp, terminal_label};

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
/// One of the palette's swatches.
const SWATCH: egui::Vec2 = vec2(18.0, 14.0);
/// The file's pane: this wide, or this share of the window where that is
/// less, and not there in a window narrower than `FILE_FROM`.
const FILE: f32 = 620.0;
const FILE_SHARE: f32 = 0.4;
const FILE_FROM: f32 = 1100.0;

/// How the screen draws: the look, the palette and the language.
#[derive(Clone, Copy)]
pub(super) struct Skin<'a> {
    pub(super) look: &'a Look,
    pub(super) palette: &'a Palette,
    pub(super) locale: Locale,
    /// Whether a connection window is open: what Enter on the Agents
    /// pane's status and log rows shows the tab of.
    pub(super) window: bool,
    /// The computer's zone's name, which the System zone writes beside it.
    pub(super) system: Option<&'a str>,
    /// The last named zone the session had, which the zone's steps reach.
    pub(super) named: Option<&'a str>,
    /// The saved connections, and what On launch steps through.
    pub(super) connections: &'a [crate::connections::SavedConnection],
    pub(super) launches: &'a [crate::settings::Launch],
    /// What the theme's rows say of the desktop.
    pub(super) desktop: Desktop<'a>,
}

/// What the Appearance pane's theme rows say beside their values: Omarchy's
/// theme of the moment, and how a pin stands.
#[derive(Clone, Copy)]
pub(super) struct Desktop<'a> {
    /// The name of Omarchy's theme of the moment, where it is known.
    pub(super) current: Option<&'a str>,
    /// The pin as the settings write it, and how it stands.
    pub(super) pin: Option<(&'a str, PinState)>,
    /// Whether Omarchy's palette is followed: else the system's light or
    /// dark is.
    pub(super) follows_omarchy: bool,
}

impl<'a> Desktop<'a> {
    pub(super) fn of(app: &'a App) -> Self {
        let pin = app.settings.theme.pin().zip(app.pin_state());
        Self {
            current: app.desktop_theme.current.as_deref(),
            pin,
            follows_omarchy: app.themes.follows_omarchy(),
        }
    }
}

impl Skin<'_> {
    /// `text` translated, in the look's case.
    pub(super) fn say(&self, text: &'static str) -> String {
        self.look.label(&gettext(self.locale, text))
    }

    /// A choice as the screen writes it: a word as [`Self::say`] has it, a
    /// sample as it is.
    fn written(&self, choice: Said) -> String {
        match choice {
            Said::Word(word) => self.say(word),
            Said::Sample(sample) => sample.to_owned(),
        }
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

pub(super) fn show(
    app: &App,
    ctx: &egui::Context,
    dialog: &SettingsDialog,
    actions: &mut Vec<Action>,
) {
    let (pane, row) = (dialog.pane, dialog.row);
    let settings = super::shown(app);
    let launches = super::launch_choices(&app.connections.connections);
    let skin = Skin {
        look: &app.look,
        palette: &app.palette,
        locale: app.locale,
        window: app.agents_window().is_some(),
        system: app.system_zone.iana_name(),
        named: app.named_zone.as_deref(),
        connections: &app.connections.connections,
        launches: &launches,
        desktop: Desktop::of(app),
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
            header(ui, pane, &skin);
            let notice = notice(ui, app, actions);
            // What the header, a notice, the footer and their rules leave.
            let height = (screen.height() - HEADER - notice - FOOTER - 2.0).max(0.0);
            let (body, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let (side, place) = body.split_left_right_at_x((body.left() + NAV).min(body.right()));
            nav(ui, side, pane, &skin, actions);
            // The rows scroll when the window is short, so the header and
            // the footer stay on screen. They start past the nav's rule.
            let place = place.with_min_x((place.left() + 1.0).min(place.right()));
            let cursor = pane.rows().get(row).copied();
            let option = cursor.and_then(SettingsRow::option);
            // The file has the right of the rows, in a window wide enough
            // for both.
            let (place, beside) = if screen.width() >= FILE_FROM {
                let width = FILE.min(screen.width() * FILE_SHARE);
                let edge = (place.right() - width).round_to_pixels(ui.pixels_per_point());
                let (place, beside) = place.split_left_right_at_x(edge);
                (place, Some(beside))
            } else {
                (place, None)
            };
            let mut list = ui.new_child(egui::UiBuilder::new().id_salt("rows").max_rect(place));
            // Each pane scrolls on its own.
            egui::ScrollArea::vertical()
                .id_salt(pane.name())
                .auto_shrink([false, false])
                .show(&mut list, |ui| match pane {
                    SettingsPane::General => rows(ui, &settings, row, &skin, actions),
                    SettingsPane::Appearance => appearance_rows(ui, &settings, row, &skin, actions),
                    SettingsPane::Agents => agent_rows(ui, app, &settings, row, &skin, actions),
                });
            if let Some(beside) = beside {
                file_pane::show(ui, beside, app, option, &skin);
            }
            let opens = cursor.and_then(|cursor| opened(app, cursor).map(|opens| (cursor, opens)));
            footer(ui, pane, option, opens, &settings, &skin, actions);
        });
}

/// One tinted line: the title and the pane it shows, and at the right
/// what a change does and the key that opens the screen.
fn header(ui: &mut Ui, pane: SettingsPane, skin: &Skin) {
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
        Text::one(look, TextRole::OBody, &skin.say(pane.name()), palette.dim),
    );
    let role = widgets::secondary(look);
    let note = format!("{} · ", skin.say("Changes apply right away"));
    let note = Text::new(look)
        .add(role, &note, palette.dim)
        .add(role, &key(ui, look, Command::Settings), palette.accent)
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
/// fill, with a 2 pt bar in the accent at its left. Neither is put on whole
/// pixels here, nor is any other fill or outline of the screen: epaint does
/// that as it draws them, the bar too where it is thin enough to be drawn
/// as a line.
fn mark(ui: &Ui, rect: Rect, palette: &Palette) {
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, palette.selection);
    let bar = Rect::from_min_size(rect.min, vec2(2.0, rect.height()));
    painter.rect_filled(bar, CornerRadius::ZERO, palette.accent);
}

/// The panes, down the left, the one shown marked: a click shows another.
/// Under them, where there are two, the key that switches.
fn nav(ui: &mut Ui, rect: Rect, shown: SettingsPane, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin {
        look,
        palette,
        locale,
        ..
    } = *skin;
    widgets::vline(ui, rect.right() + 0.5, rect.y_range(), palette.outline);
    // An item is its line of text, with 3 above and below.
    let line = TextRole::OBody.row_height(ui.ctx(), look.faces);
    let listed = SettingsPane::listed();
    for (index, pane) in listed.iter().copied().enumerate() {
        let item = Rect::from_min_size(
            pos2(rect.left(), rect.top() + 10.0 + index as f32 * (line + 6.0)),
            vec2(rect.width(), line + 6.0),
        );
        let on = pane == shown;
        let name = gettext(locale, pane.name());
        let button = ButtonSpec::new(&name).hidden_at(ui, item);
        button.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, &name));
        focus::hint(ui, &button, item, Ring::Inset { radius: 0 });
        if button.clicked() && !on {
            actions.push(Action::ShowSettingsPane(pane));
        }
        let color = if on {
            mark(ui, item, palette);
            palette.text
        } else {
            palette.dim
        };
        widgets::paint_label(
            ui,
            item.left() + 14.0,
            item.center().y,
            Text::one(look, TextRole::OBody, &skin.say(pane.name()), color),
        );
    }
    if listed.len() > 1 {
        let switch = key(ui, look, Command::SettingsPane);
        let what = skin.say("pane");
        let y = rect.bottom() - 10.0 - line / 2.0;
        let hint = [(&switch, what.as_str(), true)];
        widgets::key_hints(ui, (rect.left() + 14.0, y), &hint, 0.0, look, palette);
    }
}

/// A group's heading over its rows: 12 over it and 4 under, after `top`
/// at the pane's top or between two groups. A whole number of points, so
/// the rows' fills start on a pixel.
fn heading(ui: &mut Ui, text: &'static str, top: f32, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OCaption;
    let line = role.row_height(ui.ctx(), look.faces);
    let height = (top + 12.0 + line + 4.0).ceil();
    let (place, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    // Dim, which `widgets::section_label` is not in this look.
    widgets::paint_label(
        ui,
        place.left() + ROW_SIDE,
        place.top() + top + 12.0 + line / 2.0,
        Text::one(look, role, &skin.say(text), palette.dim),
    );
}

/// The General pane's options, one to a row, under the headings of their
/// groups. Their columns line up across the groups.
fn rows(ui: &mut Ui, settings: &Settings, cursor: usize, skin: &Skin, actions: &mut Vec<Action>) {
    let room = ui.available_width() - 2.0 * ROW_SIDE;
    let columns = columns(
        ui,
        room,
        widths(ui, &OptionId::GENERAL, settings, skin),
        skin,
    );
    let mut index = 0;
    for (heading_text, top, options) in [
        ("Startup", 8.0, &OptionId::STARTUP[..]),
        ("Data", 12.0, &OptionId::DATA[..]),
        ("Safety", 12.0, &OptionId::SAFETY[..]),
    ] {
        heading(ui, heading_text, top, skin);
        for option in options.iter().copied() {
            option_row(
                ui,
                (index, option),
                index == cursor,
                columns,
                settings,
                skin,
                actions,
            );
            index += 1;
        }
    }
}

/// The Appearance pane's options, a row each, under the headings of their
/// groups. Their columns line up across the groups.
fn appearance_rows(
    ui: &mut Ui,
    settings: &Settings,
    cursor: usize,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let room = ui.available_width() - 2.0 * ROW_SIDE;
    let columns = columns(
        ui,
        room,
        widths(ui, &OptionId::APPEARANCE, settings, skin),
        skin,
    );
    let mut index = 0;
    for (heading_text, top, options) in [
        ("Theme", 8.0, &OptionId::THEME[..]),
        ("Grid", 12.0, &OptionId::GRID[..]),
        ("Environment", 12.0, &OptionId::ENVIRONMENT[..]),
    ] {
        heading(ui, heading_text, top, skin);
        for option in options.iter().copied() {
            option_row(
                ui,
                (index, option),
                index == cursor,
                columns,
                settings,
                skin,
                actions,
            );
            index += 1;
        }
        // Under the theme, the colours it gave: no row of the cursor's.
        if options == OptionId::THEME {
            palette_row(ui, columns, skin, actions);
        }
    }
}

/// The colours the theme gave, eight of them as the board has them, and
/// where they were read from. Nothing to change: the cursor passes it.
fn palette_row(
    ui: &mut Ui,
    (label_width, value_width): (f32, f32),
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let palette = skin.palette;
    let (row, left, right, _) = row_frame(ui, (usize::MAX, "Palette"), false, skin, actions);
    let label_width = label_width.min(right - left);
    let value_width = value_width.min(right - left - label_width);
    let swatches = [
        palette.window,
        palette.accent,
        palette.success,
        palette.warning,
        palette.danger,
        palette.magenta,
        palette.info,
        palette.orange,
    ];
    let mut x = left + label_width;
    for color in swatches {
        let swatch = Rect::from_center_size(pos2(x + SWATCH.x / 2.0, row.center().y), SWATCH);
        ui.painter().rect(
            swatch,
            CornerRadius::same(2),
            color,
            Stroke::new(1.0, palette.outline),
            StrokeKind::Inside,
        );
        x += SWATCH.x + 4.0;
    }
    let hint = palette_source(skin).layout(ui.ctx());
    let hint_left = left + label_width + value_width;
    if hint_left + hint.width() <= right {
        hint.paint_left(ui.painter(), hint_left, row.center().y);
        widgets::announce(
            ui,
            Rect::from_min_size(pos2(hint_left, row.top()), vec2(hint.width(), ROW)),
            hint.galley.text(),
        );
    }
}

/// Where the drawn colours came from: an Omarchy theme, a palette file the
/// pin names, or the app's own light or dark.
fn palette_source(skin: &Skin) -> Text {
    let Skin {
        look,
        palette,
        desktop,
        ..
    } = *skin;
    let said = match desktop.pin {
        Some((_, PinState::Drawn(pinned))) if pinned.omarchy => {
            skin.say("read from the omarchy theme")
        }
        Some((name, PinState::Drawn(_))) => skin.say("read from {name}").replace("{name}", name),
        _ if desktop.follows_omarchy => skin.say("read from the omarchy theme"),
        _ => skin.say("tabletist's own light or dark"),
    };
    Text::one(look, TextRole::OBody, &said, palette.dim)
}

/// What the theme's row says beside its value: the theme drawn now, and
/// what changes it, or why a pin is not drawn.
fn theme_hint(skin: &Skin) -> Text {
    let Skin {
        look,
        palette,
        desktop,
        ..
    } = *skin;
    let role = TextRole::OBody;
    match desktop.pin {
        Some((name, PinState::Drawn(_))) => {
            let said = skin.say("now: {name} · pinned").replace("{name}", name);
            Text::one(look, role, &said, palette.dim)
        }
        Some((name, PinState::Missing)) => {
            let said = skin.say(if desktop.follows_omarchy {
                "“{name}” not found · following omarchy"
            } else {
                "“{name}” not found · following the system"
            });
            Text::one(look, role, &said.replace("{name}", name), palette.warning)
        }
        // A pin still being read is drawn as the desktop until it is.
        Some((_, PinState::Reading)) | None if desktop.follows_omarchy => {
            let name = look.label(desktop.current.unwrap_or("Omarchy"));
            let said = skin.say("now: {name} · switches with omarchy-theme");
            Text::one(look, role, &said.replace("{name}", &name), palette.dim)
        }
        Some((_, PinState::Reading)) | None => {
            let said = skin.say(if palette.dark {
                "now: dark, from the system"
            } else {
                "now: light, from the system"
            });
            Text::one(look, role, &said, palette.dim)
        }
    }
}

/// The Agents pane: the server's switch and its status, how to connect an
/// agent, where access is set, and the log. Its rows are
/// [`SettingsPane::rows`]'s, in order; the command, the line for other
/// agents and the note under them are no rows of the cursor's.
fn agent_rows(
    ui: &mut Ui,
    app: &App,
    settings: &Settings,
    cursor: usize,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin {
        look,
        palette,
        locale,
        ..
    } = *skin;
    let role = TextRole::OBody;
    let room = ui.available_width() - 2.0 * ROW_SIDE;
    let other = crate::endpoint::agent_setup_ready(crate::endpoint::Agent::Other).unwrap_or("…");
    // The rows that are no options', each its label, value and hint: made
    // again for each use, as a text is laid out once. The command for other
    // agents gives way in its middle to what `fits`, and is copied whole.
    let text_of = |ui: &Ui, which: usize, fits: f32| -> (&'static str, Text, Text) {
        match which {
            0 => {
                let (value, hint) = status_text(app.agents_on(), app.agents.sessions.len(), skin);
                ("Status", value, hint)
            }
            1 => {
                let (value, hint) = access_text(ui, skin);
                ("Set for each connection", value, hint)
            }
            _ => {
                let ctx = ui.ctx().clone();
                let shown = crate::ui::grid::ellipsize(other, fits, true, |text| {
                    role.width(&ctx, look.faces, text)
                });
                (
                    "Cursor, others",
                    Text::one(look, role, &shown, palette.text),
                    dim(skin, "same server, stdio"),
                )
            }
        }
    };
    // Their labels and values with the options': all of the pane's columns
    // line up. The command is as long as the path it holds, and lines up
    // with nothing.
    let mut widest = widths(ui, &OptionId::AGENTS, settings, skin);
    for which in 0..2 {
        let (name, value, hint) = text_of(ui, which, f32::INFINITY);
        let label = role.width(ui.ctx(), look.faces, &format!("▌{}", skin.say(name)));
        widest.0 = widest.0.max(label);
        widest.1 = widest.1.max(value.layout(ui.ctx()).width());
        widest.2 = widest.2.max(hint.layout(ui.ctx()).width());
    }
    // The status as it may come to read, not only as it does: the columns
    // stay where they are when the server is turned on or off, or agents
    // come and go.
    for (on, count) in [(false, 0), (true, 0), (true, 999)] {
        let (value, hint) = status_text(on, count, skin);
        widest.1 = widest.1.max(value.layout(ui.ctx()).width());
        widest.2 = widest.2.max(hint.layout(ui.ctx()).width());
    }
    let columns = columns(ui, room, widest, skin);
    let on = |index: usize| index == cursor;
    let fits = (room - columns.0).max(0.0);
    let text = |ui: &mut Ui, index: usize, which: usize, actions: &mut Vec<Action>| {
        let (name, value, hint) = text_of(ui, which, fits);
        let on_cursor = index != usize::MAX && on(index);
        text_row(
            ui,
            (index, name),
            on_cursor,
            columns,
            (value, hint),
            skin,
            actions,
        );
    };
    heading(ui, "MCP server", 8.0, skin);
    option_row(
        ui,
        (0, OptionId::AgentsOn),
        on(0),
        columns,
        settings,
        skin,
        actions,
    );
    text(ui, 1, 0, actions);

    heading(ui, "Connect an agent", 12.0, skin);
    command_box(ui, skin, locale);
    text(ui, usize::MAX, 2, actions);

    heading(ui, "Access", 12.0, skin);
    text(ui, 2, 1, actions);

    heading(ui, "Log", 12.0, skin);
    option_row(
        ui,
        (3, OptionId::LogDays),
        on(3),
        columns,
        settings,
        skin,
        actions,
    );
    option_row(
        ui,
        (4, OptionId::AgentsNotify),
        on(4),
        columns,
        settings,
        skin,
        actions,
    );

    ui.add_space(14.0);
    note(ui, skin);
}

/// A dim hint.
fn dim(skin: &Skin, text: &'static str) -> Text {
    Text::one(
        skin.look,
        TextRole::OBody,
        &skin.say(text),
        skin.palette.dim,
    )
}

/// The status row's value and hint: whether the server runs, with how many
/// agents are connected, and how they reach it.
fn status_text(on: bool, count: usize, skin: &Skin) -> (Text, Text) {
    let Skin {
        look,
        palette,
        locale,
        ..
    } = *skin;
    let role = TextRole::OBody;
    if !on {
        let value = Text::one(look, role, &format!("● {}", skin.say("off")), palette.dim);
        return (value, dim(skin, "agents can't connect"));
    }
    let agents = match count {
        0 => skin.say("no agents"),
        count => look.label(
            &ngettext(
                locale,
                "{n} agent",
                "{n} agents",
                u32::try_from(count).unwrap_or(u32::MAX),
            )
            .replace("{n}", &count.to_string()),
        ),
    };
    let value = Text::new(look)
        .add(role, &format!("● {}", skin.say("running")), palette.success)
        .add(role, &format!(" · {agents}"), palette.text);
    (value, dim(skin, "stdio · this computer"))
}

/// The access row's value and hint: where access is set, and the key
/// that sets it on a row of Connections.
fn access_text(ui: &Ui, skin: &Skin) -> (Text, Text) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let change = key(ui, look, Command::ChangeAgentAccess);
    let value = Text::one(look, role, &skin.say("in connections"), palette.text);
    let hint = Text::new(look).add(role, &change, palette.accent).add(
        role,
        &format!(" {}", skin.say("on a row")),
        palette.dim,
    );
    (value, hint)
}

/// The command that connects Claude Code, boxed, with the key that copies
/// it at the box's right.
fn command_box(ui: &mut Ui, skin: &Skin, locale: Locale) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let line = role.row_height(ui.ctx(), look.faces);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), line + 22.0), Sense::hover());
    let boxed = Rect::from_min_max(
        pos2(rect.left() + ROW_SIDE, rect.top() + 4.0),
        pos2(
            (rect.right() - ROW_SIDE).max(rect.left() + ROW_SIDE),
            rect.bottom() - 2.0,
        ),
    );
    ui.painter()
        .rect_filled(boxed, CornerRadius::ZERO, palette.panel);
    ui.painter().rect_stroke(
        boxed,
        CornerRadius::ZERO,
        Stroke::new(1.0, palette.outline),
        StrokeKind::Inside,
    );
    let copy = key(ui, look, Command::SettingsCopyCommand);
    let hint = Text::new(look)
        .add(role, &copy, palette.accent)
        .space(role, " ")
        .add(role, &skin.say("copy"), palette.dim)
        .layout(ui.ctx());
    let right = boxed.right() - 12.0;
    hint.paint_right(ui.painter(), right, boxed.center().y);
    // The command gives way in its middle to a pane too narrow for it, and
    // is copied whole.
    let command = crate::endpoint::agent_command_ready().unwrap_or_else(|| {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));
        "…"
    });
    let fits = (right - hint.width() - 16.0 - boxed.left() - 12.0).max(0.0);
    let ctx = ui.ctx().clone();
    let shown = crate::ui::grid::ellipsize(command, fits, true, |text| {
        role.width(&ctx, look.faces, text)
    });
    widgets::paint_text(
        ui,
        boxed.left() + 12.0,
        boxed.center().y,
        Text::one(look, role, &shown, palette.text),
    );
    widgets::announce(ui, boxed, command);
    let name = gettext(locale, "Copy");
    if ButtonSpec::new(&name)
        .hidden_at(ui, boxed.with_min_x(right - hint.width()))
        .clicked()
    {
        crate::ui::agents::copy_command(ui.ctx(), locale);
    }
}

/// What keeps agents within bounds, in a band with the accent at its left.
fn note(ui: &mut Ui, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let lines = [
        "reads run in a read-only transaction with each connection's limits",
        "no ddl, no multiple statements, no copy, no secrets sent",
    ];
    let line = role.row_height(ui.ctx(), look.faces);
    let height = (lines.len() as f32 * line * 1.2 + 20.0).ceil();
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let band = Rect::from_min_max(
        pos2(rect.left() + ROW_SIDE, rect.top()),
        pos2(
            (rect.right() - ROW_SIDE).max(rect.left() + ROW_SIDE),
            rect.bottom(),
        ),
    );
    ui.painter()
        .rect_filled(band, CornerRadius::ZERO, palette.panel);
    let bar = Rect::from_min_size(band.min, vec2(2.0, band.height()));
    ui.painter()
        .rect_filled(bar, CornerRadius::ZERO, palette.accent);
    let width = (band.width() - 24.0).max(0.0);
    let mut y = band.top() + 10.0;
    for said in lines {
        let text = Text::one(look, role, &skin.say(said), palette.dim)
            .wrap(width)
            .layout(ui.ctx());
        text.paint(ui.painter(), pos2(band.left() + 12.0, y));
        widgets::announce(
            ui,
            Rect::from_min_size(pos2(band.left() + 12.0, y), text.size()),
            text.galley.text(),
        );
        y += line * 1.2;
    }
}

/// The widest label, value and hint `options` have: the hints of every
/// value, not only the ones shown, so that the columns stay where they are
/// when a value changes.
fn widths(ui: &Ui, options: &[OptionId], settings: &Settings, skin: &Skin) -> (f32, f32, f32) {
    let role = TextRole::OBody;
    let width = |text: &str| role.width(ui.ctx(), skin.look.faces, text);
    let labels = options
        .iter()
        .map(|option| width(&format!("▌{}", skin.say(terminal_label(*option)))))
        .fold(0.0, f32::max);
    let values = options
        .iter()
        .map(|option| value_width(ui, *option, settings, skin))
        .fold(0.0, f32::max);
    let hints = options
        .iter()
        .flat_map(|option| choices(*option, settings, skin.named, skin.launches))
        .map(|(_, value)| {
            let mut changed = settings.clone();
            value.set(&mut changed);
            hint(ui, value.option(), &changed, skin)
                .layout(ui.ctx())
                .width()
        })
        .fold(0.0, f32::max);
    (labels, values, hints)
}

/// The widths of the label's column and the value's in rows with `room`
/// between their sides, from the widest label, value and hint they have:
/// `LABEL` and `VALUE` where the widest hint has room after them. Rows with
/// less (the file is beside them) take from the space each column has past
/// its widest words, from both alike, before a hint is left out.
fn columns(
    ui: &Ui,
    room: f32,
    (labels, values, hints): (f32, f32, f32),
    skin: &Skin,
) -> (f32, f32) {
    // Two cells stand between a column's widest words and the next column.
    let gap = TextRole::OBody.width(ui.ctx(), skin.look.faces, "  ");
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
        OptionId::PageSize
        | OptionId::LogDays
        | OptionId::Tz
        | OptionId::IdleDisconnect
        | OptionId::OnLaunch => width(&format!("‹ {} ›", stepped_text(option, settings, skin))),
        OptionId::Timestamps
        | OptionId::GroupDigits
        | OptionId::Theme
        | OptionId::RowHeight
        | OptionId::TableList
        | OptionId::EnvBar => {
            let words = choices(option, settings, skin.named, skin.launches);
            let between = width(" ") * words.len().saturating_sub(1) as f32;
            let segments = words.into_iter().map(|(word, _)| segment(ui, word, skin));
            segments.sum::<f32>() + between
        }
        OptionId::RestoreTabs
        | OptionId::ValueTags
        | OptionId::ProductionReadOnly
        | OptionId::ConfirmCloseEdits
        | OptionId::AgentsOn
        | OptionId::AgentsNotify
        | OptionId::Zebra
        | OptionId::ColumnTypes => width("[x]"),
    }
}

/// A stepped option's value as its row writes it: the page size, the log's
/// days, the zone, the idle time.
fn stepped_text(option: OptionId, settings: &Settings, skin: &Skin) -> String {
    match option {
        OptionId::LogDays => skin
            .look
            .label(&super::days(skin.locale, settings.mcp_log_days)),
        // As the file writes it: `30m`; never in words.
        OptionId::IdleDisconnect => match settings.idle_disconnect {
            Idle::Never => skin.say("Never"),
            choice => choice.name().to_owned(),
        },
        // As the artboard writes it: `system (america/new_york)`.
        OptionId::Tz => skin.look.label(&match (&settings.tz, skin.system) {
            (crate::settings::Zone::System, Some(name)) => {
                format!("{} ({name})", gettext(skin.locale, "System"))
            }
            (zone, system) => super::zone_text(skin.locale, zone, system),
        }),
        // A connection by its name as it is saved, and where it points:
        // three of one name are told apart.
        OptionId::OnLaunch => {
            let text = super::launch_text(skin.locale, &settings.on_launch, skin.connections);
            match &settings.on_launch {
                crate::settings::Launch::Connection { id, .. } => {
                    let saved = skin.connections.iter().find(|saved| saved.id == *id);
                    match saved {
                        // The name as it is saved, not set in the screen's case.
                        Some(saved) => {
                            let place = super::place(saved);
                            if place.is_empty() {
                                text
                            } else {
                                format!("{text} ({place})")
                            }
                        }
                        None => skin.look.label(&text),
                    }
                }
                crate::settings::Launch::Restore | crate::settings::Launch::Connections => {
                    skin.look.label(&text)
                }
            }
        }
        _ => settings.page_size.to_string(),
    }
}

/// How wide the segment of the choice `word` is.
fn segment(ui: &Ui, word: Said, skin: &Skin) -> f32 {
    let text = skin.written(word);
    TextRole::OBody.width(ui.ctx(), skin.look.faces, &text) + 2.0 * SEGMENT_PAD
}

/// A row's frame: its button, which puts the cursor on it (none for a row
/// at `usize::MAX`, which the cursor is never on), the cursor's mark, and
/// its label. Returns the row, and where its words stand between.
fn row_frame(
    ui: &mut Ui,
    (index, name): (usize, &'static str),
    on_cursor: bool,
    skin: &Skin,
    actions: &mut Vec<Action>,
) -> (Rect, f32, f32, Option<egui::Response>) {
    let Skin {
        look,
        palette,
        locale,
        ..
    } = *skin;
    let role = TextRole::OBody;
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
    let button = (index != usize::MAX).then(|| {
        let said = gettext(locale, name);
        let button = ButtonSpec::new(&said).hidden_at(ui, row);
        // A row of a list: a ring outside it would be cut where the list
        // ends.
        focus::hint(ui, &button, row, Ring::Inset { radius: 0 });
        if button.clicked() {
            actions.push(Action::SelectSettingsRow(index));
        }
        button
    });
    if on_cursor {
        mark(ui, row, palette);
    }
    let left = row.left() + ROW_SIDE;
    let right = (row.right() - ROW_SIDE).max(left);
    // The cursor's mark stands where a label starts, so the label under
    // the cursor is a cell further in than the others.
    let text = skin.say(name);
    let text = if on_cursor {
        Text::one(look, role, &format!("▌{text}"), palette.accent)
    } else {
        Text::one(look, role, &text, palette.text)
    };
    widgets::paint_text(ui, left, row.center().y, text);
    (row, left, right, button)
}

/// A row that is no option: its label, what it says, and a hint, whole or
/// left out.
fn text_row(
    ui: &mut Ui,
    (index, name): (usize, &'static str),
    on_cursor: bool,
    (label_width, value_width): (f32, f32),
    (value, hint): (Text, Text),
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let (row, left, right, _) = row_frame(ui, (index, name), on_cursor, skin, actions);
    let y = row.center().y;
    let label_width = label_width.min(right - left);
    let value_width = value_width.min(right - left - label_width);
    let value = value.layout(ui.ctx());
    let at = left + label_width;
    value.paint_left(ui.painter(), at, y);
    widgets::announce(
        ui,
        Rect::from_min_size(pos2(at, row.top()), vec2(value.width(), ROW)),
        value.galley.text(),
    );
    let hint = hint.layout(ui.ctx());
    let hint_left = at + value_width.max(value.width());
    if hint_left + hint.width() <= right {
        hint.paint_left(ui.painter(), hint_left, y);
    }
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
    let locale = skin.locale;
    // The row's button comes before its values', so that a click on a
    // value is the value's.
    let (row, left, right, button) = row_frame(
        ui,
        (index, terminal_label(option)),
        on_cursor,
        skin,
        actions,
    );
    let y = row.center().y;
    // A stepped option has no button that is its value: its row says it.
    if let Some(button) = button
        && matches!(
            option,
            OptionId::PageSize
                | OptionId::LogDays
                | OptionId::Tz
                | OptionId::IdleDisconnect
                | OptionId::OnLaunch
        )
    {
        let name = gettext(locale, terminal_label(option));
        let value = stepped_text(option, settings, skin);
        button.widget_info(|| {
            let mut info = WidgetInfo::labeled(WidgetType::Button, true, &name);
            info.current_text_value = Some(value.clone());
            info
        });
    }
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
        OptionId::PageSize
        | OptionId::LogDays
        | OptionId::Tz
        | OptionId::IdleDisconnect
        | OptionId::OnLaunch => stepper(ui, cell, option, settings, skin, actions),
        OptionId::Timestamps
        | OptionId::GroupDigits
        | OptionId::Theme
        | OptionId::RowHeight
        | OptionId::TableList
        | OptionId::EnvBar => {
            segments(ui, cell, option, settings, skin, actions);
        }
        OptionId::RestoreTabs
        | OptionId::ValueTags
        | OptionId::ProductionReadOnly
        | OptionId::ConfirmCloseEdits
        | OptionId::AgentsOn
        | OptionId::AgentsNotify
        | OptionId::Zebra
        | OptionId::ColumnTypes => {
            check(ui, cell, option, settings, skin, actions);
        }
    }
    // The hint is whole or left out: it tells nothing cut short.
    let hint = hint(ui, option, settings, skin).layout(ui.ctx());
    let hint_left = cell.left + value_width;
    if hint_left + hint.width() <= right {
        hint.paint_left(ui.painter(), hint_left, y);
    }
}

/// A stepped option, the page size, the log's days, the zone or the idle
/// time: its value as it is, and on the cursor's row between the chevrons
/// that step it, each dim where there is no value on its side.
fn stepper(
    ui: &mut Ui,
    cell: Cell,
    option: OptionId,
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let y = cell.row.center().y;
    let value = stepped_text(option, settings, skin);
    let current = settings.value(option);
    let Ok([fewer, more]) =
        <[_; 2]>::try_from(choices(option, settings, skin.named, skin.launches))
    else {
        return;
    };
    let text = if cell.on_cursor {
        let chevron = |(_, step): &(Said, OptionValue)| {
            if *step == current {
                palette.dim
            } else {
                palette.accent
            }
        };
        Text::new(look)
            .add(role, "‹", chevron(&fewer))
            .space(role, " ")
            .add(role, &value, palette.text)
            .space(role, " ")
            .add(role, "›", chevron(&more))
    } else {
        Text::one(look, role, &value, palette.text)
    };
    widgets::paint_text(ui, cell.left, y, text);
    // The chevrons' buttons are on every row. On the cursor's row each is
    // over its chevron, with half the space beside it. On the others, where
    // no chevron is drawn, each keeps a point at the column's corner: a
    // click on the value there only moves the cursor, and Tab does not
    // stop on a point.
    let places = if cell.on_cursor {
        let width = |text: &str| role.width(ui.ctx(), look.faces, text);
        let lead = width("‹ ");
        let chevron = width("‹");
        let reach = (lead - chevron) / 2.0;
        let after = cell.left + lead + width(&value) + (lead - chevron);
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
    let press = Press::Step {
        drawn: cell.on_cursor,
    };
    for (place, step) in places.into_iter().zip([fewer, more]) {
        choice(ui, place, cell.index, step, press, skin, actions);
    }
}

/// An option that is on or off, as the look's check.
fn check(
    ui: &mut Ui,
    cell: Cell,
    option: OptionId,
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let current = settings.value(option);
    let (glyph, color) = if super::is_on(&current) {
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
    let mut values = choices(option, settings, skin.named, skin.launches);
    values.sort_by_key(|(_, value)| *value != current);
    for (word, value) in values {
        let press = Press::Value {
            set: value == current,
        };
        choice(ui, place, cell.index, (word, value), press, skin, actions);
    }
}

/// What the row says beside `option`'s value: where it applies, or a
/// sample of what it does.
fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let dim = |text: &str| Text::one(look, role, text, palette.dim);
    match option {
        OptionId::PageSize => dim(&skin.say("table view")),
        OptionId::Tz => dim(&skin.say("sent with offset")),
        OptionId::Timestamps => dim(sample_timestamp(settings.timestamps)),
        OptionId::GroupDigits => dim(&skin.say(if settings.group_digits.unwrap_or_default() {
            "grouping on"
        } else {
            "grouping off"
        })),
        // Two tags as a grid shows them: in the first two slots' colours,
        // or as plain values. They are values of a column, not words:
        // no language has others.
        OptionId::ValueTags => {
            let slots = terminal_slots(palette);
            let [first, second] = if settings.value_tags {
                [slots[0], slots[1]]
            } else {
                [palette.dim; 2]
            };
            Text::new(look)
                .add(role, "print", first)
                .space(role, "  ")
                .add(role, "ebook", second)
        }
        OptionId::RestoreTabs => dim(&skin.say("per connection")),
        OptionId::OnLaunch => dim(&skin.say("or connections · a chosen connection")),
        OptionId::ProductionReadOnly => dim(&skin.say("default for new connections")),
        // As drawn: nothing to add.
        OptionId::ConfirmCloseEdits => Text::new(look),
        OptionId::IdleDisconnect => dim(&skin.say("tabs reconnect on use")),
        OptionId::AgentsOn => dim(&skin.say("while tabletist is open")),
        // The key that shows the log, from this row, where there is a
        // window to show it in.
        OptionId::LogDays if !skin.window => dim(&skin.say("on this computer")),
        OptionId::LogDays => Text::new(look)
            .add(
                role,
                &key(ui, look, Command::SettingsOpenRow),
                palette.accent,
            )
            .add(role, &format!(" {}", skin.say("shows it")), palette.dim),
        OptionId::AgentsNotify => dim(&skin.say("a desktop notification")),
        OptionId::Theme => theme_hint(skin),
        OptionId::RowHeight => dim(&skin.say("1 · 1.5 · 2 lines")),
        OptionId::Zebra => Text::new(look),
        OptionId::ColumnTypes => dim(&skin.say("second header line")),
        // The key that flips the sidebar's list, from the keymap.
        OptionId::TableList => Text::new(look)
            .add(role, &format!("{} ", skin.say("default;")), palette.dim)
            .add(role, &key(ui, look, Command::ToggleTree), palette.accent)
            .add(role, &format!(" {}", skin.say("toggles")), palette.dim),
        // Production as the bar's label writes it, in its own case.
        OptionId::EnvBar => Text::new(look)
            .add(
                role,
                crate::env::Environment::Production.label(crate::env::Platform::Omarchy),
                palette.dim,
            )
            .add(
                role,
                &format!(" {}", skin.say("always tinted")),
                palette.dim,
            ),
    }
}

/// The button a hint of the footer stands for: its name, and what a
/// press does, where it does something.
type Button<'a> = (&'a str, Option<Vec<Action>>);

/// The screen's keys at the left. A hint whose key does one thing is that
/// thing's button too. On the Agents pane: Enter, on a row that opens
/// something (`opens`), and the key that copies the command.
fn footer(
    ui: &mut Ui,
    pane: SettingsPane,
    option: Option<OptionId>,
    opens: Option<(SettingsRow, Vec<Action>)>,
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin {
        look,
        palette,
        locale,
        ..
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
    // The server's switch is not reset: off, it would end every agent's
    // session.
    let reset = option
        .filter(|option| *option != OptionId::AgentsOn)
        .map(|option| Action::SetOption(option.default_value()));
    let edit = Some(Action::EditSettingsFile);
    // `$EDITOR` is a name: the look does not lower-case it.
    let in_editor = skin
        .say("open file in {editor}")
        .replace("{editor}", "$EDITOR");
    // The key, what it does, the button it stands for, and what that does.
    let mut keys: Vec<(Command, String, Option<Button<'_>>)> = vec![
        (Command::SettingsMove, skin.say("move"), None),
        (Command::SettingsChange, skin.say("change"), None),
        (
            Command::SettingsToggle,
            skin.say("toggle"),
            Some(("Toggle", flip.map(|flip| vec![flip]))),
        ),
    ];
    if pane == SettingsPane::Agents {
        if let Some((row, opens)) = opens {
            let (what, name) = match row {
                SettingsRow::AgentsAccess => ("open connections", "Open Connections"),
                SettingsRow::Option(OptionId::LogDays) => ("show log", "Show log"),
                _ => ("show agents tab", "Show Agents tab"),
            };
            keys.push((
                Command::SettingsOpenRow,
                skin.say(what),
                Some((name, Some(opens))),
            ));
        }
        keys.push((
            Command::SettingsCopyCommand,
            skin.say("copy command"),
            Some(("Copy the command", None)),
        ));
    }
    keys.extend([
        (
            Command::SettingsOpenFile,
            in_editor,
            Some(("Open file in editor", edit.map(|edit| vec![edit]))),
        ),
        (
            Command::SettingsReset,
            skin.say("reset option"),
            Some(("Reset option", reset.map(|reset| vec![reset]))),
        ),
        (
            Command::SettingsClose,
            skin.say("close"),
            Some(("Close", Some(vec![Action::CloseDialog]))),
        ),
    ]);
    // The hints give way from the end where the window has no room for
    // them, all but the last, which closes: their keys still work.
    let width_of = |ui: &Ui, (command, what): (Command, &str)| {
        let key = key(ui, look, command);
        widgets::key_hints_width(ui, &[(&key, what, true)], 0.0, look, palette)
    };
    let room = rect.width() - 24.0;
    let mut widths: Vec<f32> = keys
        .iter()
        .map(|(command, what, _)| width_of(ui, (*command, what)))
        .collect();
    while keys.len() > 1 && widths.iter().sum::<f32>() + 16.0 * (keys.len() - 1) as f32 > room {
        let before_last = keys.len() - 2;
        keys.remove(before_last);
        widths.remove(before_last);
    }
    let mut left = rect.left() + 12.0;
    for (command, what, button) in keys {
        let key = key(ui, look, command);
        let hint = [(&key, what.as_str(), true)];
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
                if command == Command::SettingsCopyCommand {
                    crate::ui::agents::copy_command(ui.ctx(), locale);
                }
                actions.extend(action.into_iter().flatten());
            }
        }
    }
}

/// What a choice's button is to the keyboard and to a screen reader.
#[derive(Clone, Copy)]
enum Press {
    /// One of the option's values, the one it is set to or not: a radio
    /// button.
    Value { set: bool },
    /// A step from the value it has: a plain button. Tab passes one that
    /// is not drawn.
    Step { drawn: bool },
}

/// The button over a choice as it is drawn, named by its option and its
/// own word: a click there moves the cursor to its row and sets it.
fn choice(
    ui: &mut Ui,
    place: Rect,
    index: usize,
    (word, value): (Said, OptionValue),
    press: Press,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let name = format!(
        "{}: {}",
        gettext(skin.locale, terminal_label(value.option())),
        word.name(skin.locale)
    );
    let button = ButtonSpec::new(&name);
    let button = match press {
        Press::Step { drawn: false } => button.hidden_off_tab(ui, place),
        Press::Step { drawn: true } | Press::Value { .. } => button.hidden_at(ui, place),
    };
    if let Press::Value { set } = press {
        button.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, true, set, &name));
    }
    if button.clicked() {
        actions.push(Action::SelectSettingsRow(index));
        // The theme it shows already: a value of the other looks', shown
        // as the desktop followed, stays in the file.
        let shown = matches!(press, Press::Value { set: true });
        if !(shown && value.option() == OptionId::Theme) {
            actions.push(Action::SetOption(value));
        }
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
    for (word, value) in choices(option, settings, skin.named, skin.launches) {
        let text = skin.written(word);
        let width = segment(ui, word, skin);
        let place = Rect::from_min_size(
            pos2(left, cell.row.center().y - line / 2.0),
            vec2(width, line),
        );
        left += width + gap;
        let color = if value != current {
            palette.dim
        } else if cell.on_cursor {
            ui.painter()
                .rect_filled(place, CornerRadius::ZERO, palette.accent);
            palette.on_accent
        } else {
            let stroke = Stroke::new(1.0, palette.outline);
            ui.painter()
                .rect_stroke(place, CornerRadius::ZERO, stroke, StrokeKind::Inside);
            palette.text
        };
        let text = Text::one(look, role, &text, color);
        widgets::paint_text(ui, place.left() + SEGMENT_PAD, place.center().y, text);
        let press = Press::Value {
            set: value == current,
        };
        choice(ui, place, cell.index, (word, value), press, skin, actions);
    }
}

/// A key of the screen, as the keymap writes it for the look.
fn key(ui: &Ui, look: &Look, command: Command) -> crate::keymap::Written {
    crate::ui::keys::written(ui.ctx(), look, command)
}
