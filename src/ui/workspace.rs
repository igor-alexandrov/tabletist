//! A connection tab: the connection bar, the disconnected banner, the
//! sidebar, the object tabs and the open object.

use egui::{CornerRadius, Frame, Margin, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};
use tabletist_db::TlsMode;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, ObjectView, SessionStatus};
use crate::theme::{self, Icon, Look, Palette};
use crate::ui::widgets;

/// The connection bar's height.
pub fn bar_height(look: &Look) -> f32 {
    // macOS: 48 and a 3 pt stripe above, a 1 pt rule below. Omarchy: 36
    // and the rule.
    if look.terminal { 37.0 } else { 52.0 }
}

/// The macOS bar's stripe in the environment colour.
const STRIPE: f32 = 3.0;

/// How wide `text` is in `font`.
fn measure(ui: &egui::Ui, text: &str, font: &egui::FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
        .size()
        .x
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let look = app.look;
    top_bar(app, ui, tab);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let connected =
        workspace.tree.schemas.value.is_some() || workspace.tree.schemas.error.is_some();
    if look.terminal && connected {
        super::object_tabs::show(app, ui, tab);
        status_line(app, ui, tab);
    }
    banner(app, ui, tab);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    if !connected {
        return; // still connecting; the banner shows progress
    }
    let active = workspace.active_object;
    let view = workspace.active_object_tab().map(|object| object.view);
    let row_panel = workspace.row_panel;
    super::sidebar::show(app, ui, tab);
    if look.terminal
        && let (Some(object_tab), Some(ObjectView::Data), true) = (active, view, row_panel)
    {
        super::row_panel::show(app, ui, tab, object_tab);
    }
    egui::CentralPanel::default()
        .frame(Frame::new().fill(app.palette.window))
        .show(ui, |ui| {
            if !look.terminal {
                super::object_tabs::show(app, ui, tab);
                if let (Some(object_tab), Some(ObjectView::Data), true) = (active, view, row_panel)
                {
                    super::row_panel::show(app, ui, tab, object_tab);
                }
            }
            match active {
                Some(object_tab) => {
                    super::data_view::header(app, ui, tab, object_tab);
                    if view == Some(ObjectView::Data) {
                        super::data_view::toolbar(app, ui, tab, object_tab);
                    }
                    if !look.terminal {
                        super::data_view::footer(app, ui, tab, object_tab);
                    }
                    egui::CentralPanel::default()
                        .frame(Frame::new().fill(app.palette.window))
                        .show(ui, |ui| match view {
                            Some(ObjectView::Structure) => {
                                super::structure::show(app, ui, tab, object_tab)
                            }
                            _ => {
                                if super::filter_bar::is_open(app, tab, object_tab) {
                                    let palette = app.palette;
                                    egui::Panel::top(egui::Id::new((
                                        "filter-bar",
                                        tab.0,
                                        object_tab.0,
                                    )))
                                    .resizable(false)
                                    .show_separator_line(false)
                                    .frame(
                                        Frame::new()
                                            .fill(palette.panel)
                                            .inner_margin(Margin::symmetric(12, 8)),
                                    )
                                    .show(ui, |ui| {
                                        super::filter_bar::show(app, ui, tab, object_tab);
                                        let rect = ui.max_rect().expand2(vec2(12.0, 8.0));
                                        widgets::hline(
                                            ui,
                                            rect.x_range(),
                                            rect.bottom(),
                                            palette.outline,
                                        );
                                    });
                                }
                                super::data_view::show(app, ui, tab, object_tab)
                            }
                        });
                }
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new(gettext(
                                app.locale,
                                "Select a table or view in the sidebar",
                            ))
                            .color(app.palette.secondary),
                        );
                    });
                }
            }
        });
}

/// What the connection bar says about one connection.
struct BarInfo {
    name: String,
    env: crate::connections::Environment,
    host: String,
    database: String,
    databases: Vec<String>,
    tls: Option<(&'static str, Tone)>,
    ssh_host: Option<String>,
}

/// How a status reads: fine, or a warning.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Good,
    Warn,
}

fn bar_info(app: &App, tab: ConnTabId) -> Option<BarInfo> {
    let workspace = app.workspace(tab)?;
    let spec = &workspace.spec;
    let sqlite = workspace.driver == tabletist_db::Driver::Sqlite;
    let host = if sqlite {
        spec.summary()
    } else {
        format!("{}:{}", spec.host, spec.port)
    };
    // Local traffic never crosses a network, so its TLS says nothing; a
    // remote connection always says how far it can be trusted.
    let remote = !sqlite && !crate::model::is_local_host(&spec.host);
    let tls = remote.then(|| {
        let encrypted =
            matches!(workspace.status, SessionStatus::Connected).then_some(workspace.encrypted);
        let (text, warn) = tls_status(spec.tls, encrypted);
        (text, if warn { Tone::Warn } else { Tone::Good })
    });
    Some(BarInfo {
        name: workspace.name.clone(),
        env: workspace.color.environment(),
        host,
        database: if sqlite {
            String::new()
        } else {
            spec.database.clone()
        },
        databases: workspace.databases.value.clone().unwrap_or_default(),
        tls,
        ssh_host: spec.ssh.as_ref().map(|ssh| ssh.host.clone()),
    })
}

/// The connection bar: the connection's environment colour behind its
/// name, where it points, and the way out.
fn top_bar(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(info) = bar_info(app, tab) else {
        return;
    };
    let env = theme::env_colors(info.env, &palette);
    let base = if palette.dark {
        palette.panel
    } else {
        egui::Color32::WHITE
    };
    let (tint, border) = if look.terminal {
        (
            theme::mix(base, env.color, 0.16),
            theme::mix(base, env.color, 0.4),
        )
    } else {
        (
            theme::mix(base, env.color, 0.12),
            theme::mix(base, env.color, 0.28),
        )
    };
    // macOS: with one connection the bar is the title bar, beside the
    // window buttons.
    let zoom = ui.ctx().zoom_factor();
    let alone = app.tabs.len() == 1;
    let inset = if alone {
        app.titlebar.inset / zoom
    } else {
        0.0
    };
    let height = if alone {
        bar_height(&look).max(app.titlebar.height / zoom)
    } else {
        bar_height(&look)
    };
    let mut actions = Vec::new();
    egui::Panel::top(egui::Id::new(("workspace-top", tab.0)))
        .exact_size(height)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(tint))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            // The empty bar moves the window, as a title bar does.
            let drag = ui.interact(rect, ui.id().with("drag"), Sense::click_and_drag());
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, border);
            if look.terminal {
                terminal_bar(ui, tab, rect, &info, &env, &palette, &mut actions, locale);
            } else {
                let stripe = Rect::from_min_size(rect.min, vec2(rect.width(), STRIPE));
                ui.painter()
                    .rect_filled(stripe, CornerRadius::ZERO, env.color);
                let body = Rect::from_min_max(
                    pos2(rect.left() + inset, rect.top() + STRIPE),
                    pos2(rect.right(), rect.bottom() - 1.0),
                );
                mac_bar(
                    ui,
                    tab,
                    body,
                    &info,
                    &env,
                    &look,
                    &palette,
                    &mut actions,
                    locale,
                );
            }
        });
    app.actions.extend(actions);
}

/// macOS: a box with the name, host and database (a pop-up of the other
/// databases), the read-only pill, and Disconnect.
#[allow(clippy::too_many_arguments)] // one call site; the pieces are unrelated
fn mac_bar(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    rect: Rect,
    info: &BarInfo,
    env: &theme::EnvColors,
    look: &Look,
    palette: &Palette,
    actions: &mut Vec<Action>,
    locale: crate::i18n::Locale,
) {
    let center = rect.center().y;
    // The bar's own rule, and white faces over its tint.
    let base = if palette.dark {
        palette.panel
    } else {
        egui::Color32::WHITE
    };
    let rim = theme::mix(base, env.color, 0.28);
    let face = |alpha: f32| {
        if palette.dark {
            palette.window.gamma_multiply(alpha)
        } else {
            egui::Color32::WHITE.gamma_multiply(alpha)
        }
    };
    let hair = Stroke::new(widgets::hairline(ui), rim);
    // The crumb: name, host, "/", database, and the pop-up's chevrons, 8
    // apart and 10 in from its edges.
    let name_font = theme::semibold(theme::TEXT);
    let host_font = theme::regular(theme::TEXT);
    let db_font = theme::mono(theme::TEXT_SMALL);
    let switchable = info.databases.len() > 1;
    let mut parts = vec![
        measure(ui, &info.name, &name_font),
        measure(ui, &info.host, &host_font),
    ];
    if !info.database.is_empty() {
        parts.push(measure(ui, "/", &host_font));
        parts.push(measure(ui, &info.database, &db_font));
    }
    if switchable {
        parts.push(12.0);
    }
    let width = 20.0 + parts.iter().sum::<f32>() + 8.0 * (parts.len() - 1) as f32;
    let crumb = Rect::from_min_size(pos2(rect.left() + 16.0, center - 16.0), vec2(width, 32.0));
    let response = ui.interact(crumb, ui.id().with("database"), Sense::click());
    response.widget_info(|| {
        let mut info_ = egui::WidgetInfo::labeled(
            egui::WidgetType::ComboBox,
            switchable,
            gettext(locale, "Database"),
        );
        info_.current_text_value = Some(info.database.clone());
        info_
    });
    let corner = CornerRadius::same(look.radius);
    ui.painter().rect_filled(crumb, corner, face(0.7));
    ui.painter()
        .rect_stroke(crumb, corner, hair, StrokeKind::Inside);
    let mut x = crumb.left() + 10.0;
    x += widgets::paint_label(ui, x, center, &info.name, name_font, palette.text) + 8.0;
    x += widgets::paint_label(ui, x, center, &info.host, host_font.clone(), palette.dim) + 8.0;
    if !info.database.is_empty() {
        x += widgets::paint_text(
            ui,
            x,
            center,
            "/",
            host_font,
            palette.border.lerp_to_gamma(palette.faint, 0.35),
        ) + 8.0;
        x += widgets::paint_label(ui, x, center, &info.database, db_font, palette.text) + 8.0;
    }
    if switchable {
        Icon::ChevronsUpDown.image(palette.dim, 12.0).paint_at(
            ui,
            Rect::from_center_size(pos2(x + 6.0, center), vec2(12.0, 12.0)),
        );
        egui::Popup::menu(&response).show(|ui| {
            ui.set_min_width(crumb.width());
            for database in &info.databases {
                if ui
                    .selectable_label(*database == info.database, database)
                    .clicked()
                    && *database != info.database
                {
                    actions.push(Action::SwitchDatabase {
                        tab,
                        database: database.clone(),
                    });
                }
            }
        });
    }
    // Pills after the crumb, 12 on: read-only, then TLS and SSH when remote.
    let mut left = crumb.right() + 12.0;
    let mut pill = |icon: Option<Icon>, text: &str, color: egui::Color32| {
        let font = theme::regular(theme::TEXT_SMALL);
        let icon_width = if icon.is_some() { 12.0 + 6.0 } else { 0.0 };
        let rect = Rect::from_min_size(
            pos2(left, center - 13.0),
            vec2(20.0 + icon_width + measure(ui, text, &font), 26.0),
        );
        let corner = CornerRadius::same(13);
        ui.painter().rect_filled(rect, corner, face(0.8));
        ui.painter()
            .rect_stroke(rect, corner, hair, StrokeKind::Inside);
        let mut x = rect.left() + 10.0;
        if let Some(icon) = icon {
            icon.image(color, 12.0).paint_at(
                ui,
                Rect::from_center_size(pos2(x + 6.0, center), vec2(12.0, 12.0)),
            );
            x += icon_width;
        }
        widgets::paint_label(ui, x, center, text, font, color);
        left = rect.right() + 12.0;
    };
    pill(
        Some(Icon::Lock),
        &gettext(locale, "Read-only"),
        palette.secondary,
    );
    if let Some((text, tone)) = info.tls {
        let color = match tone {
            Tone::Good => palette.success,
            Tone::Warn => palette.warning,
        };
        pill(Some(Icon::Lock), &gettext(locale, text), color);
    }
    if let Some(host) = &info.ssh_host {
        pill(
            None,
            &format!("{} {host}", gettext(locale, "via SSH")),
            palette.secondary,
        );
    }
    // Disconnect, 12 in from the right: an icon and its label, 6 apart.
    let label = gettext(locale, "Disconnect");
    let font = theme::regular(theme::TEXT);
    let width = 10.0 + 14.0 + 6.0 + measure(ui, &label, &font) + 10.0;
    let button = Rect::from_min_size(
        pos2(rect.right() - 12.0 - width, center - 16.0),
        vec2(width, 32.0),
    );
    let response = ui.interact(button, ui.id().with("disconnect"), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
    if response.hovered() {
        ui.painter()
            .rect_filled(button, CornerRadius::same(6), face(0.7));
    }
    Icon::LogIn.image(palette.secondary, 14.0).paint_at(
        ui,
        Rect::from_center_size(pos2(button.left() + 17.0, center), vec2(14.0, 14.0)),
    );
    widgets::paint_text(
        ui,
        button.left() + 30.0,
        center,
        &label,
        font,
        palette.secondary,
    );
    if response.clicked() {
        actions.push(Action::Disconnect(tab));
    }
}

/// Omarchy: the environment badge, the name, where it points, read-only,
/// and the key that closes the connection.
#[allow(clippy::too_many_arguments)] // one call site; the pieces are unrelated
fn terminal_bar(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    rect: Rect,
    info: &BarInfo,
    env: &theme::EnvColors,
    palette: &Palette,
    actions: &mut Vec<Action>,
    locale: crate::i18n::Locale,
) {
    let center = rect.center().y;
    let connection_line = theme::mix(palette.panel, env.color, 0.4);
    // Twelve in and twelve apart, as the design's row.
    let mut x = rect.left() + 12.0;
    x += env_badge(ui, x, center, info.env, env, true) + 12.0;
    x += widgets::paint_label(
        ui,
        x,
        center,
        &info.name,
        theme::semibold(theme::TEXT),
        palette.text,
    ) + 12.0;
    let target = if info.database.is_empty() {
        info.host.clone()
    } else {
        format!("{}/{}", info.host, info.database)
    };
    x += widgets::paint_label(
        ui,
        x,
        center,
        &target,
        theme::regular(theme::TEXT),
        palette.dim,
    ) + 12.0;
    let mut tag = |text: &str, color: egui::Color32| {
        let font = theme::regular(theme::TEXT_LABEL);
        let line = ui.fonts_mut(|fonts| fonts.row_height(&font));
        let rect = Rect::from_min_size(
            pos2(x, center - (line + 2.0) / 2.0),
            vec2(measure(ui, text, &font) + 14.0 + 2.0, line + 2.0),
        );
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0, connection_line),
            StrokeKind::Inside,
        );
        widgets::paint_label(ui, x + 8.0, center, text, font, color);
        x = rect.right() + 12.0;
    };
    tag(&gettext(locale, "read-only"), palette.text);
    if let Some((text, tone)) = info.tls {
        let color = match tone {
            Tone::Good => palette.success,
            Tone::Warn => palette.warning,
        };
        tag(&gettext(locale, text), color);
    }
    if let Some(host) = &info.ssh_host {
        tag(
            &format!("{} {host}", gettext(locale, "via SSH")),
            palette.dim,
        );
    }
    // The way out, a muted note that also answers a click.
    let note = "ctrl+shift+w disconnect";
    let font = theme::regular(theme::TEXT_SMALL);
    let width = measure(ui, note, &font);
    let left = rect.right() - 12.0 - width;
    let hit = Rect::from_min_size(
        pos2(left - 4.0, rect.top()),
        vec2(width + 8.0, rect.height()),
    );
    let response = ui.interact(hit, ui.id().with("disconnect"), Sense::click());
    let label = gettext(locale, "Disconnect");
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
    let color = if response.hovered() {
        palette.text
    } else {
        palette.dim
    };
    widgets::paint_text(ui, left, center, note, font, color);
    if response.clicked() {
        actions.push(Action::Disconnect(tab));
    }
}

/// An environment badge at `x`, centred on `y`: a rounded tint on macOS, a
/// solid upper-case tag in the terminal look. Returns its width.
pub fn env_badge(
    ui: &egui::Ui,
    x: f32,
    y: f32,
    env: crate::connections::Environment,
    colors: &theme::EnvColors,
    terminal: bool,
) -> f32 {
    if terminal {
        return env_badge_in(ui, x, y, env, colors, theme::TEXT_LABEL);
    }
    env_badge_styled(ui, x, y, env, colors, None)
}

/// The terminal's badge without letter spacing, as the connections list
/// draws it.
pub fn env_badge_plain(
    ui: &egui::Ui,
    x: f32,
    y: f32,
    env: crate::connections::Environment,
    colors: &theme::EnvColors,
) -> f32 {
    env_badge_styled(ui, x, y, env, colors, Some((theme::TEXT_LABEL, 0.0)))
}

/// The terminal's badge with its text at `size` points.
pub fn env_badge_in(
    ui: &egui::Ui,
    x: f32,
    y: f32,
    env: crate::connections::Environment,
    colors: &theme::EnvColors,
    size: f32,
) -> f32 {
    env_badge_styled(ui, x, y, env, colors, Some((size, 0.04 * size)))
}

fn env_badge_styled(
    ui: &egui::Ui,
    x: f32,
    y: f32,
    env: crate::connections::Environment,
    colors: &theme::EnvColors,
    terminal: Option<(f32, f32)>,
) -> f32 {
    // One point above and below the text, seven at its sides.
    let (text, font, tracking, corner) = if let Some((size, tracking)) = terminal {
        (
            env.label(true).to_uppercase(),
            theme::semibold(size),
            tracking,
            3,
        )
    } else {
        (
            env.label(false).to_owned(),
            theme::medium(theme::TEXT_CAPTION),
            0.0,
            10,
        )
    };
    let pad = 7.0;
    let job = widgets::styled(&text, font, colors.badge_text, tracking);
    let galley = ui.painter().layout_job(job);
    let height = galley.size().y + 2.0;
    let rect = Rect::from_min_size(
        pos2(x, y - height / 2.0),
        vec2(galley.size().x + 2.0 * pad, height),
    );
    ui.painter()
        .rect_filled(rect, CornerRadius::same(corner), colors.badge);
    ui.painter().galley(
        pos2(x + pad, y - galley.size().y / 2.0),
        galley,
        egui::Color32::PLACEHOLDER,
    );
    rect.width()
}

/// Omarchy's bottom line: the keys that work here, and the page's range.
fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let palette = app.palette;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let summary = workspace
        .active_object_tab()
        .and_then(|object| {
            let page = object.page()?;
            let first = object.query.offset + u64::from(!page.rows.is_empty());
            let last = object.query.offset + page.rows.len() as u64;
            let total = object
                .count
                .value
                .map(|count| count.to_string())
                .or_else(|| (!page.has_more).then(|| last.to_string()))
                .unwrap_or_else(|| "?".into());
            Some(format!(
                "{first}–{last}/{total} · {}",
                crate::ui::format::elapsed(page.elapsed)
            ))
        })
        .unwrap_or_default();
    egui::Panel::bottom(egui::Id::new(("status-line", tab.0)))
        .exact_size(31.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
            let y = rect.top() + 1.0 + 15.0;
            let hints = [
                ("j/k", "row", true),
                ("h/l", "col", true),
                ("enter", "inspect", true),
                ("i", "inspector", true),
                ("/", "filter", true),
                ("ctrl+b", "tables", true),
                ("y", "copy", true),
                ("s", "structure", true),
            ];
            let font = theme::regular(theme::TEXT_SMALL);
            let measure = |text: &str| {
                ui.painter()
                    .layout_no_wrap(text.to_owned(), font.clone(), palette.text)
                    .size()
                    .x
            };
            let tag = gettext(app.locale, "read-only");
            let tag_width = measure(&tag) + 12.0 + 2.0;
            let summary_width = measure(&summary);
            // Everything 18 apart, 12 in from the ends. What does not fit
            // before the range gives way from the end.
            let gap = 18.0;
            let limit = rect.right() - 12.0 - summary_width - gap - tag_width - gap;
            let disabled = ["e edit", "o new row", "dd delete", ":w write"];
            let disabled_width = disabled.iter().map(|text| measure(text)).sum::<f32>()
                + 14.0 * (disabled.len() - 1) as f32;
            let mut x = rect.left() + 12.0;
            let shown = (0..=hints.len())
                .rev()
                .find(|&count| x + widgets::key_hints_width(ui, &hints[..count], gap) <= limit)
                .unwrap_or(0);
            x += widgets::key_hints(ui, x, y, &hints[..shown], gap, &palette) + gap;
            let separator = measure("│");
            if shown == hints.len() && x + separator + gap + disabled_width <= limit {
                x += widgets::paint_text(ui, x, y, "│", font.clone(), palette.outline) + gap;
                // Editing's keys, struck through at half strength.
                let faded = palette.panel.lerp_to_gamma(palette.dim, 0.5);
                for (index, text) in disabled.iter().enumerate() {
                    if index > 0 {
                        x += 14.0;
                    }
                    let width = widgets::paint_text(ui, x, y, text, font.clone(), faded);
                    ui.painter().hline(
                        egui::Rangef::new(x, x + width),
                        y + 0.5,
                        Stroke::new(1.0, faded),
                    );
                    x += width;
                }
                x += gap;
            }
            let pill = Rect::from_min_size(pos2(x, y - 9.0), vec2(tag_width, 18.0));
            ui.painter().rect_stroke(
                pill,
                CornerRadius::same(3),
                Stroke::new(1.0, palette.outline),
                StrokeKind::Inside,
            );
            widgets::paint_text(ui, x + 7.0, y, &tag, font.clone(), palette.dim);
            widgets::paint_text_right(ui, rect.right() - 12.0, y, &summary, font, palette.dim);
        });
}

/// What the top bar says about TLS, and whether to say it as a warning.
/// `encrypted` is what the session negotiated, once it has connected.
/// `prefer` and `require` do not check the certificate (as in libpq), so
/// anyone on the network path could be the server.
fn tls_status(mode: TlsMode, encrypted: Option<bool>) -> (&'static str, bool) {
    match (mode, encrypted) {
        (TlsMode::Disable, _) | (_, Some(false)) => ("Not encrypted", true),
        (TlsMode::Prefer | TlsMode::Require, _) => ("TLS, not verified", true),
        (TlsMode::VerifyCa, _) => ("TLS, host name not checked", false),
        (TlsMode::VerifyFull, _) => ("TLS verified", false),
    }
}

fn banner(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let message = match &workspace.status {
        SessionStatus::Connecting { .. } => {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.spinner();
                ui.label(gettext(locale, "Connecting…"));
            });
            return;
        }
        SessionStatus::Connected => return,
        SessionStatus::Disconnected(error) => (
            crate::ui::format::describe_error(locale, error),
            error.to_string(),
        ),
        SessionStatus::Cancelled => (
            gettext(locale, "Connection cancelled.").into_owned(),
            String::new(),
        ),
    };
    let (described, raw) = message;
    let conn = workspace.conn_id.clone();
    let mut reconnect = false;
    let mut edit = false;
    // Rounded corners sit inside the window; Omarchy's square banner spans it.
    let inset = if look.tab_radius == 0 { 0 } else { 8 };
    Frame::new()
        .fill(palette.danger.gamma_multiply(0.12))
        .corner_radius(egui::CornerRadius::same(look.tab_radius))
        .inner_margin(Margin::symmetric(12, 10))
        .outer_margin(Margin::same(inset))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // The plain sentence, and under it the exact error when it
                // says more (visible, so keyboards and screen readers get it).
                ui.vertical(|ui| {
                    ui.label(RichText::new(&described).color(palette.text));
                    if !raw.is_empty() && raw != described {
                        ui.label(
                            RichText::new(&raw)
                                .size(crate::theme::TEXT_SMALL)
                                .color(palette.secondary),
                        );
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    reconnect = crate::ui::widgets::primary_button(
                        ui,
                        &gettext(locale, "Reconnect"),
                        &look,
                        &palette,
                    )
                    .clicked();
                    edit = ui.button(gettext(locale, "Edit connection")).clicked();
                });
            });
        });
    if reconnect {
        app.actions.push(Action::Reconnect(tab));
    }
    if edit {
        app.actions.push(Action::EditConnection(conn));
    }
}
