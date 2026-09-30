//! The body of a picker tab: the saved connections, grouped, with search,
//! New connection, and each connection's actions.

use egui::{
    Align2, CornerRadius, Rect, Sense, Stroke, StrokeKind, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::app::App;
use crate::connections::{ConnectionId, SavedConnection};
use crate::i18n::gettext;
use crate::model::{Action, ConnTabContent};
use crate::theme::{self, Icon, Look, Palette};
use crate::ui::widgets::{self, ButtonSpec};

/// The header's height, per look.
fn header_height(look: &Look) -> f32 {
    if look.terminal { 38.0 } else { 60.0 }
}

/// The terminal picker's text: a size up from the workspace's, as the
/// design sets a screen of its own.
const TERMINAL_TEXT: f32 = 14.0;

/// The terminal look's filter line under the header.
const FILTER_LINE: f32 = 36.0;

/// A connection row's height, per look.
fn row_height(look: &Look) -> f32 {
    if look.terminal { 50.0 } else { 60.0 }
}

/// The footer's height, per look.
fn footer_height(look: &Look) -> f32 {
    if look.terminal { 30.0 } else { 33.0 }
}

/// Connections under one heading: those sharing a name, then the local
/// and the remote rest.
pub struct Group<'a> {
    pub title: String,
    pub connections: Vec<&'a SavedConnection>,
}

/// `connections` in groups: a name two or more share heads a group of its
/// own; the others go under Local or Remote by where they connect.
pub fn groups<'a>(connections: &[&'a SavedConnection]) -> Vec<Group<'a>> {
    let mut named: Vec<Group<'a>> = Vec::new();
    let mut local = Vec::new();
    let mut remote = Vec::new();
    for connection in connections {
        let shared = connections
            .iter()
            .filter(|other| other.name == connection.name)
            .count()
            > 1;
        if shared {
            match named
                .iter_mut()
                .find(|group| group.title == connection.name)
            {
                Some(group) => group.connections.push(connection),
                None => named.push(Group {
                    title: connection.name.clone(),
                    connections: vec![connection],
                }),
            }
        } else if is_local(connection) {
            local.push(*connection);
        } else {
            remote.push(*connection);
        }
    }
    named.sort_by_key(|group| group.title.to_lowercase());
    for (title, connections) in [("Local", local), ("Remote", remote)] {
        if !connections.is_empty() {
            named.push(Group {
                title: title.into(),
                connections,
            });
        }
    }
    named
}

/// Whether a connection stays on this machine: a file, or a local host
/// without a tunnel.
fn is_local(connection: &SavedConnection) -> bool {
    connection.spec.driver == tabletist_db::Driver::Sqlite
        || (connection.spec.ssh.is_none() && crate::model::is_local_host(&connection.spec.host))
}

/// The connections the picker lists, in its order: grouped and filtered by
/// the search. Keys move through this list.
pub fn visible(app: &App, search: &str) -> Vec<ConnectionId> {
    let found = app.connections.search(search);
    groups(&found)
        .into_iter()
        .flat_map(|group| group.connections.into_iter().map(|c| c.id.clone()))
        .collect()
}

/// How a connection's TLS reads in the list, and whether it is fine.
fn tls_label(connection: &SavedConnection) -> (String, Option<bool>) {
    use tabletist_db::TlsMode;
    if is_local(connection) {
        return ("Local".into(), None);
    }
    let text = match connection.spec.tls {
        TlsMode::Disable => "no TLS",
        TlsMode::Prefer => "prefer",
        TlsMode::Require => "require",
        TlsMode::VerifyCa => "verify-ca",
        TlsMode::VerifyFull => "verify-full",
    };
    (
        text.into(),
        Some(matches!(
            connection.spec.tls,
            TlsMode::VerifyFull | TlsMode::VerifyCa
        )),
    )
}

/// How a connection's tunnel reads in the list.
fn ssh_label(connection: &SavedConnection) -> String {
    match &connection.spec.ssh {
        Some(ssh) => format!("SSH via {}", ssh.host),
        None => "No SSH".into(),
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let tab = app.active_tab_id();
    let index = app.active;
    let total = app.connections.connections.len();
    let mut actions = Vec::new();
    let full = ui.max_rect();
    // macOS: alone, the header is the title bar, beside the window buttons.
    let zoom = ui.ctx().zoom_factor();
    let inset = if app.tabs.len() == 1 {
        app.titlebar.inset / zoom
    } else {
        0.0
    };
    let backdrop = if look.terminal {
        palette.window
    } else {
        palette.panel.lerp_to_gamma(palette.surface, 0.64)
    };
    ui.painter().rect_filled(full, CornerRadius::ZERO, backdrop);

    // Header.
    let header = Rect::from_min_size(full.min, vec2(full.width(), header_height(&look)));
    let drag = ui.interact(header, ui.id().with("picker-drag"), Sense::click_and_drag());
    if drag.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    let header_fill = if look.terminal {
        palette.panel
    } else {
        palette.window
    };
    ui.painter()
        .rect_filled(header, CornerRadius::ZERO, header_fill);
    widgets::hline(ui, header.x_range(), header.bottom() - 0.5, palette.outline);
    let side: f32 = if look.terminal { 13.5 } else { 22.7 };
    let y = header.center().y;
    let mut x = header.left() + side.max(inset + 12.0);
    let title = if look.terminal {
        gettext(locale, "connections").into_owned()
    } else {
        gettext(locale, "Connections").into_owned()
    };
    let title_font = theme::semibold(if look.terminal { 16.5 } else { look.heading });
    x += widgets::paint_text(ui, x, y, &title, title_font, palette.text) + 10.0;
    let count_font = if look.terminal {
        theme::regular(TERMINAL_TEXT)
    } else {
        theme::regular(theme::TEXT_SMALL)
    };
    x += widgets::paint_text(
        ui,
        x,
        y + if look.terminal { 0.0 } else { 2.0 },
        &total.to_string(),
        count_font,
        palette.dim,
    );
    // The buttons at the right: New connection, then Paste URL (terminal
    // hint only names the key when it works).
    let right = header.right() - side;
    let new_label = if look.terminal {
        "+ new".to_owned()
    } else {
        gettext(locale, "New connection").into_owned()
    };
    let command_n = format!("{}N", look.command_key());
    let shortcut = if look.terminal {
        "n"
    } else {
        command_n.as_str()
    };
    let mut new = ButtonSpec::new(&new_label).primary().shortcut(shortcut);
    if !look.terminal {
        new = new.icon(Icon::Plus);
    }
    let height = if look.terminal { 22.0 } else { 31.0 };
    let width = new.width(ui);
    let place = Rect::from_min_size(pos2(right - width, y - height / 2.0), vec2(width, height));
    let button = new
        .label(&gettext(locale, "New connection"))
        .show_at(ui, place, &look, &palette);
    if button.clicked() {
        actions.push(Action::NewConnection);
    }
    // Search.
    if let ConnTabContent::Picker(picker) = &mut app.tabs[index].content {
        if look.terminal {
            // The terminal filters on its own line under the header: an
            // accent slash, then the text.
            let line = Rect::from_min_size(
                pos2(full.left(), header.bottom()),
                vec2(full.width(), FILTER_LINE),
            );
            widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.outline);
            let font = theme::regular(TERMINAL_TEXT);
            widgets::paint_text(
                ui,
                line.left() + side,
                line.center().y,
                "/",
                font.clone(),
                palette.accent,
            );
            let row = ui.fonts_mut(|fonts| fonts.row_height(&font));
            let field = Rect::from_min_max(
                pos2(line.left() + side + 18.0, line.center().y - row / 2.0),
                pos2(line.right() - side, line.center().y + row / 2.0),
            );
            let focus = std::mem::take(&mut picker.focus_search);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field));
            let response = child.add(
                egui::TextEdit::singleline(&mut picker.search)
                    .font(font.clone())
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO)
                    .desired_width(field.width())
                    .hint_text(
                        egui::RichText::new(gettext(locale, "filter connections"))
                            .color(palette.dim)
                            .font(font),
                    ),
            );
            response.widget_info(|| {
                WidgetInfo::labeled(WidgetType::TextEdit, true, "Search connections")
            });
            if focus {
                response.request_focus();
            }
        } else {
            let field = Rect::from_min_size(pos2(full.left() + 178.7, y - 16.5), vec2(316.0, 33.0));
            let field = if field.left() < x + 16.0 {
                field.translate(vec2(x + 16.0 - field.left(), 0.0))
            } else {
                field
            };
            let field = Rect::from_min_max(
                field.min,
                pos2(field.right().min(place.left() - 16.0), field.bottom()),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field));
            widgets::filter_field(
                &mut child,
                &mut picker.search,
                &gettext(locale, "Find connection…"),
                field.size(),
                &look,
                &palette,
            )
            .widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, "Search connections"));
        }
    }
    let (search, selected, folded) = match &app.tabs[index].content {
        ConnTabContent::Picker(picker) => (
            picker.search.clone(),
            picker.selected.clone(),
            picker.folded.clone(),
        ),
        ConnTabContent::Workspace(_) => (String::new(), None, Vec::new()),
    };

    // Footer.
    let footer = Rect::from_min_max(
        pos2(full.left(), full.bottom() - footer_height(&look)),
        full.max,
    );
    ui.painter()
        .rect_filled(footer, CornerRadius::ZERO, palette.panel);
    widgets::hline(ui, footer.x_range(), footer.top() + 0.5, palette.outline);
    let command = look.command_key();
    if look.terminal {
        let hints = [
            ("j/k", "move", true),
            ("enter", "connect", true),
            ("e", "edit", true),
            ("n", "new", true),
            ("yy", "duplicate", true),
            ("dd", "delete", true),
            ("/", "filter", true),
        ];
        widgets::key_hints_in(
            ui,
            footer.left() + side,
            footer.center().y,
            &hints,
            22.0,
            TERMINAL_TEXT,
            &palette,
        );
    } else {
        let edit = format!("{command}E");
        let duplicate = format!("{command}D");
        let delete = format!("{command}⌫");
        let hints = [
            ("↩", "connect", true),
            (edit.as_str(), "edit", true),
            (duplicate.as_str(), "duplicate", true),
            (delete.as_str(), "delete", true),
        ];
        let mut x = footer.left() + side;
        let font = theme::regular(theme::TEXT_SMALL);
        for (key, what, _) in hints {
            x += widgets::paint_text(ui, x, footer.center().y, key, font.clone(), palette.dim);
            x += widgets::paint_text(
                ui,
                x,
                footer.center().y,
                &format!(" {}", gettext(locale, what)),
                font.clone(),
                palette.dim,
            ) + 16.0;
        }
    }
    let secrets_font = theme::regular(if look.terminal {
        TERMINAL_TEXT
    } else {
        theme::TEXT_SMALL
    });
    let secrets = if look.terminal {
        "secrets: keyring".to_owned()
    } else if look.faces == theme::Faces::Plex {
        gettext(locale, "Passwords are stored in the macOS Keychain").into_owned()
    } else {
        gettext(locale, "Passwords are stored in the system keyring").into_owned()
    };
    widgets::paint_text_right(
        ui,
        footer.right() - side,
        footer.center().y,
        &secrets,
        secrets_font,
        palette.dim,
    );

    // The list.
    let top = if look.terminal {
        header.bottom() + FILTER_LINE
    } else {
        header.bottom()
    };
    let body = Rect::from_min_max(pos2(full.left(), top), pos2(full.right(), footer.top()));
    let mut list = ui.new_child(egui::UiBuilder::new().max_rect(body));
    let found = app.connections.search(&search);
    if app.connections.connections.is_empty() || found.is_empty() {
        let text = if app.connections.connections.is_empty() {
            gettext(locale, "No saved connections yet")
        } else {
            gettext(locale, "No connections match")
        };
        list.add_space(40.0);
        list.vertical_centered(|ui| {
            ui.label(egui::RichText::new(text).color(palette.secondary));
        });
    } else {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut list, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                ui.add_space(if look.terminal { 9.0 } else { 0.0 });
                for group in groups(&found) {
                    let open = !folded.contains(&group.title);
                    group_header(ui, &group, open, side, &look, &palette, &mut actions, tab);
                    if !open {
                        continue;
                    }
                    let skin = RowSkin {
                        look: &look,
                        palette: &palette,
                        locale,
                        now,
                        side,
                    };
                    if look.terminal {
                        for connection in &group.connections {
                            terminal_row(
                                ui,
                                connection,
                                &app.connections,
                                selected.as_ref(),
                                tab,
                                skin,
                                &mut actions,
                            );
                        }
                    } else {
                        let rows = group.connections.len() as f32;
                        let card = Rect::from_min_size(
                            pos2(ui.max_rect().left() + side, ui.cursor().top()),
                            vec2(ui.max_rect().width() - 2.0 * side, rows * row_height(&look)),
                        );
                        let corner = CornerRadius::same(look.radius);
                        ui.painter().rect_filled(card, corner, palette.window);
                        let backdrop_shape = ui.painter().add(egui::Shape::Noop);
                        for (index, connection) in group.connections.iter().enumerate() {
                            mac_row(
                                ui,
                                card,
                                index,
                                connection,
                                &app.connections,
                                selected.as_ref(),
                                tab,
                                skin,
                                &mut actions,
                            );
                        }
                        ui.painter().set(
                            backdrop_shape,
                            egui::epaint::RectShape::stroke(
                                card,
                                corner,
                                Stroke::new(widgets::hairline(ui), palette.outline),
                                StrokeKind::Outside,
                            ),
                        );
                    }
                }
                ui.add_space(12.0);
            });
    }
    app.actions.extend(actions);
}

/// A group's heading: a chevron that folds it, its title and count.
#[allow(clippy::too_many_arguments)] // one call site per look
fn group_header(
    ui: &mut egui::Ui,
    group: &Group<'_>,
    open: bool,
    side: f32,
    look: &Look,
    palette: &Palette,
    actions: &mut Vec<Action>,
    tab: crate::model::ConnTabId,
) {
    let height = if look.terminal { 34.0 } else { 42.0 };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &group.title));
    let y = rect.center().y + if look.terminal { 0.0 } else { 3.0 };
    let x = rect.left() + side;
    if look.terminal {
        let glyph = if open { "▾" } else { "▸" };
        ui.painter().text(
            pos2(x + 3.0, y),
            Align2::CENTER_CENTER,
            glyph,
            theme::regular(theme::TEXT_SMALL),
            palette.dim,
        );
        let width = widgets::paint_text(
            ui,
            x + 13.0,
            y,
            &group.title,
            theme::semibold(TERMINAL_TEXT),
            palette.text,
        );
        widgets::paint_text(
            ui,
            x + 13.0 + width + 6.0,
            y,
            &group.connections.len().to_string(),
            theme::regular(TERMINAL_TEXT),
            palette.dim,
        );
    } else {
        let icon = if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        };
        icon.image(palette.dim, 12.0).paint_at(
            ui,
            Rect::from_center_size(pos2(x + 8.0, y), vec2(12.0, 12.0)),
        );
        let width = widgets::paint_job(
            ui,
            x + 22.0,
            y,
            widgets::section_label(&group.title, look, palette),
        );
        widgets::paint_text(
            ui,
            x + 22.0 + width + 9.0,
            y,
            &group.connections.len().to_string(),
            theme::regular(theme::TEXT_SMALL),
            palette.dim,
        );
    }
    if response.clicked() {
        actions.push(Action::FoldConnectionGroup {
            tab,
            group: group.title.clone(),
        });
    }
}

/// How rows draw.
#[derive(Clone, Copy)]
struct RowSkin<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: crate::i18n::Locale,
    now: u64,
    side: f32,
}

/// Where a connection points, as the rows say it: host and port, then the
/// database (a file's name for SQLite).
fn target(connection: &SavedConnection) -> (String, String) {
    let spec = &connection.spec;
    if spec.driver == tabletist_db::Driver::Sqlite {
        return (spec.summary(), String::new());
    }
    (
        format!("{}:{}", spec.host, spec.port),
        format!("/{}", spec.database),
    )
}

/// The row's own response, answering a click by selecting and a double
/// click, Enter or a screen reader by connecting.
fn row_response(
    ui: &mut egui::Ui,
    rect: Rect,
    connection: &SavedConnection,
    tab: crate::model::ConnTabId,
    actions: &mut Vec<Action>,
) -> egui::Response {
    let response = ui.interact(
        rect,
        ui.id().with(("connection", &connection.id.0)),
        Sense::click(),
    );
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &connection.name));
    let activated = response.double_clicked()
        || (response.clicked() && !response.clicked_by(egui::PointerButton::Primary));
    if activated {
        actions.push(Action::Connect {
            tab,
            conn: connection.id.clone(),
        });
    } else if response.clicked() {
        actions.push(Action::SelectConnection {
            tab,
            conn: Some(connection.id.clone()),
        });
    }
    let id = connection.id.clone();
    response.context_menu(|ui| {
        let locale = crate::i18n::Locale::default();
        if ui.button(gettext(locale, "Connect")).clicked() {
            actions.push(Action::Connect {
                tab,
                conn: id.clone(),
            });
        }
        if ui.button(gettext(locale, "Edit…")).clicked() {
            actions.push(Action::EditConnection(id.clone()));
        }
        if ui.button(gettext(locale, "Duplicate")).clicked() {
            actions.push(Action::DuplicateConnection(id.clone()));
        }
        if ui.button(gettext(locale, "Delete")).clicked() {
            actions.push(Action::DeleteConnection(id.clone()));
        }
    });
    response
}

/// macOS: a row of the card, a colour bar at its left edge, columns for
/// the name, where it points, its security and when it was last used.
#[allow(clippy::too_many_arguments)] // one call site
fn mac_row(
    ui: &mut egui::Ui,
    card: Rect,
    index: usize,
    connection: &SavedConnection,
    store: &crate::connections::SavedConnections,
    selected: Option<&ConnectionId>,
    tab: crate::model::ConnTabId,
    skin: RowSkin<'_>,
    actions: &mut Vec<Action>,
) {
    let RowSkin {
        look,
        palette,
        locale,
        now,
        ..
    } = skin;
    let height = row_height(look);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let rect = Rect::from_min_max(
        pos2(card.left(), rect.top()),
        pos2(card.right(), rect.bottom()),
    );
    let response = row_response(ui, rect, connection, tab, actions);
    let is_selected = selected == Some(&connection.id);
    let rows = (card.height() / height).round() as usize;
    let corner = CornerRadius {
        nw: if index == 0 { look.radius } else { 0 },
        sw: if index + 1 == rows { look.radius } else { 0 },
        ne: if index == 0 { look.radius } else { 0 },
        se: if index + 1 == rows { look.radius } else { 0 },
    };
    if is_selected {
        ui.painter().rect_filled(
            rect,
            corner,
            palette.window.lerp_to_gamma(palette.accent, 0.08),
        );
    } else if response.hovered() {
        ui.painter().rect_filled(rect, corner, palette.panel);
    }
    if index > 0 {
        widgets::hline(ui, rect.x_range(), rect.top(), palette.surface_hover);
    }
    let env = connection.color.environment();
    let colors = theme::env_colors(env, palette);
    let bar = Rect::from_min_size(rect.min, vec2(4.0, rect.height()));
    ui.painter().rect_filled(
        bar,
        CornerRadius {
            nw: corner.nw,
            sw: corner.sw,
            ne: 0,
            se: 0,
        },
        colors.color,
    );
    let top = rect.top() + 20.7;
    let bottom = rect.top() + 38.7;
    let x = rect.left() + 20.6;
    let name_width = widgets::paint_text(
        ui,
        x,
        top,
        &connection.name,
        theme::semibold(theme::TEXT),
        palette.text,
    );
    super::workspace::env_badge(ui, x + name_width + 8.0, top, env, &colors, false);
    let user = &connection.spec.user;
    let second = if user.is_empty() {
        connection.spec.driver.label().to_owned()
    } else {
        format!("{} · {user}", connection.spec.driver.label())
    };
    widgets::paint_label(
        ui,
        x,
        bottom,
        &second,
        theme::regular(theme::TEXT_SMALL),
        palette.dim,
    );
    let width = card.width();
    let host_x = card.left() + width * 0.2287;
    let (host, database) = target(connection);
    widgets::paint_label(
        ui,
        host_x,
        top,
        &host,
        theme::mono(theme::TEXT),
        palette.text,
    );
    if !database.is_empty() {
        widgets::paint_label(
            ui,
            host_x,
            bottom,
            &database,
            theme::mono(theme::TEXT),
            palette.dim,
        );
    }
    let security_x = card.left() + width * 0.505;
    let (tls, verified) = tls_label(connection);
    let tls_color = match verified {
        Some(true) => palette.success,
        Some(false) => palette.warning,
        None => palette.secondary,
    };
    Icon::Lock.image(tls_color, 11.0).paint_at(
        ui,
        Rect::from_center_size(pos2(security_x + 5.5, top), vec2(11.0, 11.0)),
    );
    widgets::paint_label(
        ui,
        security_x + 15.0,
        top,
        &tls,
        theme::regular(theme::TEXT_SMALL),
        tls_color,
    );
    widgets::paint_label(
        ui,
        security_x,
        bottom,
        &ssh_label(connection),
        theme::regular(theme::TEXT_SMALL),
        palette.secondary,
    );
    let used_x = card.left() + width * 0.6128;
    let when = crate::connections::when(store.last_used(&connection.id), now);
    widgets::paint_label(
        ui,
        used_x,
        rect.center().y,
        &format!("{} {when}", gettext(locale, "Last used")),
        theme::regular(theme::TEXT_SMALL),
        palette.secondary,
    );
    // Edit, more, and Connect: on the selected row and under the pointer.
    // Connect is always there for keyboards and screen readers.
    let shown = is_selected || ui.rect_contains_pointer(rect);
    let y = rect.center().y;
    let connect_label = format!("{} {}", gettext(locale, "Connect to"), connection.name);
    let connect_text = gettext(locale, "Connect");
    let connect = ButtonSpec::new(&connect_text)
        .primary()
        .label(&connect_label)
        .salt(&connection.id.0);
    let width = connect.width(ui);
    let place = Rect::from_min_size(
        pos2(rect.right() - 23.0 - width, y - 14.5),
        vec2(width, 29.0),
    );
    let response = if shown {
        connect.show_at(ui, place, look, palette)
    } else {
        connect.hidden_at(ui, place)
    };
    if response.clicked() {
        actions.push(Action::Connect {
            tab,
            conn: connection.id.clone(),
        });
    }
    let more_place =
        Rect::from_min_size(pos2(place.left() - 8.0 - 27.0, y - 14.5), vec2(27.0, 29.0));
    let edit_label = format!("{} {}", gettext(locale, "Edit"), connection.name);
    let edit_text = gettext(locale, "Edit");
    let edit = ButtonSpec::new(&edit_text)
        .label(&edit_label)
        .salt(&connection.id.0);
    let edit_width = edit.width(ui);
    let edit_place = Rect::from_min_size(
        pos2(more_place.left() - 8.0 - edit_width, y - 14.5),
        vec2(edit_width, 29.0),
    );
    let response = if shown {
        edit.show_at(ui, edit_place, look, palette)
    } else {
        edit.hidden_at(ui, edit_place)
    };
    if response.clicked() {
        actions.push(Action::EditConnection(connection.id.clone()));
    }
    let more_label = format!(
        "{} {}",
        gettext(locale, "More actions for"),
        connection.name
    );
    let more = ButtonSpec::new("⋯")
        .label(&more_label)
        .salt(&connection.id.0);
    let response = if shown {
        more.show_at(ui, more_place, look, palette)
    } else {
        more.hidden_at(ui, more_place)
    };
    let id = connection.id.clone();
    egui::Popup::menu(&response).show(|ui| {
        if ui.button(gettext(locale, "Duplicate")).clicked() {
            actions.push(Action::DuplicateConnection(id.clone()));
        }
        let delete = format!("{} {}", gettext(locale, "Delete"), connection.name);
        if ui
            .add(egui::Button::new(gettext(locale, "Delete")))
            .on_hover_text(&delete)
            .clicked()
        {
            actions.push(Action::DeleteConnection(id.clone()));
        }
    });
}

/// The terminal look: the environment tag, the name over where it points
/// and how, and when it was last used at the right.
#[allow(clippy::too_many_arguments)] // one call site
fn terminal_row(
    ui: &mut egui::Ui,
    connection: &SavedConnection,
    store: &crate::connections::SavedConnections,
    selected: Option<&ConnectionId>,
    tab: crate::model::ConnTabId,
    skin: RowSkin<'_>,
    actions: &mut Vec<Action>,
) {
    let RowSkin {
        look,
        palette,
        locale,
        now,
        side,
    } = skin;
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), row_height(look)), Sense::hover());
    let response = row_response(ui, rect, connection, tab, actions);
    let is_selected = selected == Some(&connection.id);
    if is_selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.selection);
        let marker =
            Rect::from_center_size(pos2(rect.left() + 9.0, rect.center().y), vec2(3.0, 14.0));
        ui.painter()
            .rect_filled(marker, CornerRadius::ZERO, palette.accent);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.05));
    }
    let env = connection.color.environment();
    let colors = theme::env_colors(env, palette);
    let x = rect.left() + side + 7.0;
    super::workspace::env_badge_in(ui, x, rect.center().y, env, &colors, 13.0);
    let text_x = rect.left() + side + 112.5;
    let top = rect.center().y - 9.5;
    let bottom = rect.center().y + 9.5;
    widgets::paint_text(
        ui,
        text_x,
        top,
        &connection.name,
        theme::semibold(TERMINAL_TEXT),
        palette.text,
    );
    let spec = &connection.spec;
    let mut line = spec.summary();
    if let Some(at) = line.find(" via ") {
        line.truncate(at);
    }
    let small = theme::regular(TERMINAL_TEXT);
    let mut x =
        text_x + widgets::paint_label(ui, text_x, bottom, &line, small.clone(), palette.dim);
    let (tls, verified) = tls_label(connection);
    let tls = if verified.is_some() {
        format!("tls {tls}")
    } else {
        tls.to_lowercase()
    };
    let tls_color = match verified {
        Some(true) => palette.success,
        Some(false) => palette.warning,
        None => palette.dim,
    };
    x += widgets::paint_text(ui, x, bottom, " · ", small.clone(), palette.dim);
    x += widgets::paint_text(ui, x, bottom, &tls, small.clone(), tls_color);
    let ssh = match &spec.ssh {
        Some(ssh) => format!("ssh {}", ssh.host),
        None => "no ssh".into(),
    };
    x += widgets::paint_text(ui, x, bottom, " · ", small.clone(), palette.dim);
    widgets::paint_text(ui, x, bottom, &ssh, small.clone(), palette.dim);
    let when = crate::connections::when(store.last_used(&connection.id), now).to_lowercase();
    widgets::paint_text_right(
        ui,
        rect.right() - side,
        rect.center().y,
        &when,
        small,
        palette.dim,
    );
    // Connect, for screen readers and the tests; the keys do it here.
    let connect = format!("{} {}", gettext(locale, "Connect to"), connection.name);
    let hit = Rect::from_min_size(pos2(rect.right() - side - 1.0, rect.top()), vec2(1.0, 1.0));
    if ButtonSpec::new(&connect)
        .salt(&connection.id.0)
        .hidden_at(ui, hit)
        .clicked()
    {
        actions.push(Action::Connect {
            tab,
            conn: connection.id.clone(),
        });
    }
}
