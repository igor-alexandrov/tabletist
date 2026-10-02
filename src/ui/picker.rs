//! The body of a picker tab: the saved connections, grouped, with search,
//! New connection, and each connection's actions.

use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, WidgetInfo, WidgetType, pos2, vec2};

use crate::app::App;
use crate::connections::{ConnectionId, SavedConnection};
use crate::i18n::gettext;
use crate::model::{Action, ConnTabContent};
use crate::theme::{self, Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::states;
use crate::ui::widgets::{self, ButtonSpec};

/// The header's height, per look (its rule below included).
fn header_height(look: &Look) -> f32 {
    if look.terminal { 41.0 } else { 65.0 }
}

/// The line the header's contents centre on, below its top (over the rule).
pub fn header_line(look: &Look) -> f32 {
    (header_height(look) - 1.0) / 2.0
}

/// The terminal look's filter line under the header (rule included).
const FILTER_LINE: f32 = 37.0;

/// A connection row's height: macOS 64 and its rule, terminal 52.
fn row_height(look: &Look) -> f32 {
    if look.terminal { 52.0 } else { 65.0 }
}

/// The footer's height, per look (its rule above included).
fn footer_height(look: &Look) -> f32 {
    if look.terminal { 31.0 } else { 37.0 }
}

/// Space at the sides: macOS 24, terminal 14 (the footer 12).
fn side(look: &Look) -> f32 {
    if look.terminal { 14.0 } else { 24.0 }
}

/// How far below its galley's top a text's baseline sits.
fn baseline(laid: &crate::typography::Laid) -> f32 {
    laid.galley
        .rows
        .first()
        .and_then(|row| row.glyphs.first())
        .map_or(laid.height() * 0.8, |glyph| glyph.pos.y)
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
    // macOS: the header is the title bar, beside the window buttons.
    let zoom = ui.ctx().zoom_factor();
    let inset = app.titlebar.inset / zoom;
    let backdrop = if look.terminal {
        palette.window
    } else {
        palette.panel.lerp_to_gamma(palette.surface, 0.64)
    };
    ui.painter().rect_filled(full, CornerRadius::ZERO, backdrop);
    let side = side(&look);

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
    let y = header.top() + header_line(&look);
    let mut x = header.left() + side.max(inset + 12.0);
    let title = if look.terminal {
        gettext(locale, "connections").into_owned()
    } else {
        gettext(locale, "Connections").into_owned()
    };
    // The title and its count share a baseline, 10 apart (12 in the
    // terminal, where they sit centred).
    let title_role = TextRole::pick(&look, TextRole::ScreenTitle, TextRole::OScreenTitle);
    let count_role = widgets::body(&look);
    let title_text = Text::one(&look, title_role, &title, palette.text).layout(ui.ctx());
    let count_text = Text::one(&look, count_role, &total.to_string(), palette.dim).layout(ui.ctx());
    let (title_base, count_base) = (baseline(&title_text), baseline(&count_text));
    let (title_middle, count_middle) = (title_text.middle(), count_text.middle());
    title_text.paint_left(ui.painter(), x, y);
    x += title_text.width() + if look.terminal { 12.0 } else { 10.0 };
    let count_y = if look.terminal {
        y
    } else {
        y - title_middle + title_base - count_base + count_middle
    };
    x += count_text.paint_left(ui.painter(), x, count_y);
    // New connection, at the right.
    let right = header.right() - side;
    let new_label = if look.terminal {
        "+ new".to_owned()
    } else {
        gettext(locale, "New connection").into_owned()
    };
    let command_n = format!("{}N", look.command_key());
    let new = if look.terminal {
        ButtonSpec::new(&new_label)
            .primary()
            .role(TextRole::OGroup)
            .shortcut("n")
            .shortcut_role(TextRole::OBody)
            // The design's 1 pt border and 10 of padding.
            .padding(11.0)
            .gap(8.0)
    } else {
        ButtonSpec::new(&new_label)
            .primary()
            .icon(Icon::Plus)
            .shortcut(&command_n)
            .padding(14.0)
            .gap(8.0)
    };
    let height = if look.terminal { 26.0 } else { 34.0 };
    let width = new.width(ui, &look);
    let place = Rect::from_min_size(pos2(right - width, y - height / 2.0), vec2(width, height));
    let button = new
        .label(&gettext(locale, "New connection"))
        .show_at(ui, place, &look, &palette);
    if button.clicked() {
        actions.push(Action::NewConnection);
    }
    // Search.
    if let ConnTabContent::Picker(picker) = &mut app.tabs[index].content {
        let focus = std::mem::take(&mut picker.focus_search);
        let (field, style, hint) = if look.terminal {
            // The terminal filters on its own line under the header.
            let line = Rect::from_min_size(
                pos2(full.left(), header.bottom()),
                vec2(full.width(), FILTER_LINE),
            );
            widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.outline);
            (
                Rect::from_min_max(
                    pos2(line.left() + side - 8.0, line.top()),
                    pos2(line.right() - side, line.bottom() - 1.0),
                ),
                widgets::FieldStyle {
                    fill: None,
                    boxed: false,
                    role: TextRole::OBody,
                },
                gettext(locale, "filter connections"),
            )
        } else {
            // 24 past the count (and the header's 12 gap).
            let left = x + 12.0 + 24.0;
            // The design's 320 is its content: with 10 of padding and a
            // 1 pt border each side the box is 342 by 36.
            let right = (left + 342.0).min(place.left() - 16.0);
            (
                Rect::from_min_max(pos2(left, y - 18.0), pos2(right.max(left), y + 18.0)),
                widgets::FieldStyle {
                    fill: Some(palette.panel.lerp_to_gamma(palette.surface, 0.36)),
                    boxed: true,
                    role: TextRole::UiBody,
                },
                gettext(locale, "Find connection…"),
            )
        };
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field));
        let response = widgets::filter_field(
            &mut child,
            &mut picker.search,
            &hint,
            field.size(),
            style,
            &look,
            &palette,
        );
        response
            .widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, "Search connections"));
        if focus {
            response.request_focus();
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
    let footer_y = footer.top() + 1.0 + (footer.height() - 1.0) / 2.0;
    let footer_side = if look.terminal { 12.0 } else { side };
    let command = look.command_key();
    let small = widgets::secondary(&look);
    if look.terminal {
        let hints = [
            ("j/k", "move", true),
            ("enter", "connect / show", true),
            ("e", "edit", true),
            ("n", "new", true),
            ("yy", "duplicate", true),
            ("dd", "delete", true),
            ("/", "filter", true),
        ];
        widgets::key_hints(
            ui,
            (footer.left() + footer_side, footer_y),
            &hints,
            16.0,
            &look,
            &palette,
        );
    } else {
        let hints = [
            "↩ connect".to_owned(),
            format!("{command}E edit"),
            format!("{command}D duplicate"),
            format!("{command}⌫ delete"),
        ];
        let status = palette.secondary.lerp_to_gamma(palette.dim, 0.5);
        let mut x = footer.left() + footer_side;
        for hint in hints {
            x +=
                widgets::paint_text(ui, x, footer_y, Text::one(&look, small, &hint, status)) + 18.0;
        }
    }
    let secrets = if look.terminal {
        "secrets: keyring".to_owned()
    } else if look.faces == theme::Faces::Plex {
        gettext(locale, "Passwords are stored in the macOS Keychain").into_owned()
    } else {
        gettext(locale, "Passwords are stored in the system keyring").into_owned()
    };
    let color = if look.terminal {
        palette.dim
    } else {
        palette.secondary.lerp_to_gamma(palette.dim, 0.5)
    };
    widgets::paint_text_right(
        ui,
        footer.right() - footer_side,
        footer_y,
        Text::one(&look, small, &secrets, color),
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
    if app.connections.connections.is_empty() {
        // First launch: why the list is empty, and the one next step.
        let keyring = if look.faces == theme::Faces::Plex {
            gettext(locale, "Tabletist stores passwords in the macOS Keychain.")
        } else {
            gettext(locale, "Tabletist stores passwords in the system keyring.")
        };
        let text = format!(
            "{} {keyring}",
            gettext(
                locale,
                "Add a PostgreSQL, MySQL or SQLite database to start browsing."
            )
        );
        let (title, text) = (
            look.label(&gettext(locale, "No connections yet")),
            look.label(&text),
        );
        let notice = states::Notice {
            icon: Icon::Database,
            title: &title,
            text: &text,
        };
        let name = look.label(&gettext(locale, "New connection"));
        let keys = if look.terminal { "n" } else { &command_n };
        let add = states::key_button(&name, keys, &look)
            .label("New connection")
            .salt("empty");
        let add = if look.terminal {
            add
        } else {
            add.primary().icon(Icon::Plus).padding(14.0).gap(8.0)
        };
        if states::empty(&mut list, body, &notice, vec![add], &look, &palette).is_some() {
            actions.push(Action::NewConnection);
        }
    } else if found.is_empty() {
        list.add_space(40.0);
        list.vertical_centered(|ui| {
            let text = gettext(locale, "No connections match");
            Text::one(&look, widgets::body(&look), &text, palette.secondary)
                .layout(ui.ctx())
                .label(ui);
        });
    } else {
        let now = crate::util::now_secs();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(&mut list, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                // macOS: 20 above the groups and between them. Terminal: 8.
                ui.add_space(if look.terminal { 8.0 } else { 20.0 });
                for (number, group) in groups(&found).into_iter().enumerate() {
                    if number > 0 && !look.terminal {
                        ui.add_space(20.0);
                    }
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
                                Opening {
                                    tab,
                                    open: app.tab_showing(&connection.id),
                                },
                                skin,
                                &mut actions,
                            );
                        }
                    } else {
                        let rows = group.connections.len() as f32;
                        // A card with a 1 pt border round its rows.
                        let card = Rect::from_min_size(
                            pos2(ui.max_rect().left() + side, ui.cursor().top()),
                            vec2(
                                ui.max_rect().width() - 2.0 * side,
                                rows * row_height(&look) + 2.0,
                            ),
                        );
                        let corner = CornerRadius::same(10);
                        ui.painter().rect_filled(card, corner, palette.window);
                        let border_shape = ui.painter().add(egui::Shape::Noop);
                        ui.add_space(1.0);
                        for (index, connection) in group.connections.iter().enumerate() {
                            mac_row(
                                ui,
                                card.shrink(1.0),
                                index,
                                connection,
                                &app.connections,
                                selected.as_ref(),
                                Opening {
                                    tab,
                                    open: app.tab_showing(&connection.id),
                                },
                                skin,
                                &mut actions,
                            );
                        }
                        ui.add_space(1.0);
                        ui.painter().set(
                            border_shape,
                            egui::epaint::RectShape::stroke(
                                card,
                                corner,
                                Stroke::new(widgets::hairline(ui), palette.outline),
                                StrokeKind::Inside,
                            ),
                        );
                    }
                }
                ui.add_space(20.0);
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
    // macOS: the heading's line and 8 below it. Terminal: 4 above a 30 pt
    // row.
    let height = if look.terminal { 34.0 } else { 16.0 + 8.0 };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &group.title));
    let title = &group.title;
    let count = group.connections.len().to_string();
    if look.terminal {
        let y = rect.top() + 4.0 + 15.0;
        let x = rect.left() + side;
        let glyph = if open { "▾" } else { "▸" };
        let role = TextRole::OBody;
        let mut left = x;
        left += widgets::paint_text(ui, left, y, Text::one(look, role, glyph, palette.dim)) + 6.0;
        left += widgets::paint_text(
            ui,
            left,
            y,
            Text::one(look, TextRole::OGroup, title, palette.text),
        ) + 6.0;
        widgets::paint_text(ui, left, y, Text::one(look, role, &count, palette.dim));
    } else {
        let y = rect.top() + 8.0;
        let x = rect.left() + side + 4.0;
        let icon = if open {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        };
        icon.image(palette.dim, 12.0).paint_at(
            ui,
            Rect::from_center_size(pos2(x + 6.0, y), vec2(12.0, 12.0)),
        );
        let width = widgets::paint_text(
            ui,
            x + 20.0,
            y,
            Text::one(look, TextRole::GroupLabel, title, palette.secondary),
        );
        widgets::paint_text(
            ui,
            x + 20.0 + width + 8.0,
            y,
            Text::one(look, TextRole::Secondary, &count, palette.dim),
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

/// Where a row's connection opens: the picker's tab, or the tab that has
/// it open already.
#[derive(Clone, Copy)]
struct Opening {
    tab: crate::model::ConnTabId,
    open: Option<crate::model::ConnTabId>,
}

impl Opening {
    /// What choosing the row does: an open connection shows, another
    /// connects in the picker's tab.
    fn choose(self, connection: &SavedConnection) -> Action {
        match self.open {
            Some(open) => Action::ActivateConnTab(open),
            None => self.connect(connection),
        }
    }

    /// Connects in the picker's tab, open already or not.
    fn connect(self, connection: &SavedConnection) -> Action {
        Action::Connect {
            tab: self.tab,
            conn: connection.id.clone(),
        }
    }
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
/// click, Enter or a screen reader by choosing it: an open connection
/// shows, another connects.
fn row_response(
    ui: &mut egui::Ui,
    rect: Rect,
    connection: &SavedConnection,
    opening: Opening,
    look: &Look,
    actions: &mut Vec<Action>,
) -> egui::Response {
    let tab = opening.tab;
    let response = ui.interact(
        rect,
        ui.id().with(("connection", &connection.id.0)),
        Sense::click(),
    );
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &connection.name));
    let activated = response.double_clicked()
        || (response.clicked() && !response.clicked_by(egui::PointerButton::Primary));
    if activated {
        actions.push(opening.choose(connection));
    } else if response.clicked() {
        actions.push(Action::SelectConnection {
            tab,
            conn: Some(connection.id.clone()),
        });
    }
    let id = connection.id.clone();
    response.context_menu(|ui| {
        let locale = crate::i18n::Locale::default();
        let item = |ui: &mut egui::Ui, text: &'static str| {
            widgets::button(ui, &gettext(locale, text), look).clicked()
        };
        // An open connection shows, and can be opened once more.
        if opening.open.is_some() {
            if item(ui, "Show") {
                actions.push(opening.choose(connection));
            }
            if item(ui, "Connect again") {
                actions.push(opening.connect(connection));
            }
        } else if item(ui, "Connect") {
            actions.push(opening.connect(connection));
        }
        if item(ui, "Edit…") {
            actions.push(Action::EditConnection(id.clone()));
        }
        if item(ui, "Duplicate") {
            actions.push(Action::DuplicateConnection(id.clone()));
        }
        if item(ui, "Delete") {
            actions.push(Action::DeleteConnection(id.clone()));
        }
    });
    response
}

/// The words on the button that chooses a row, and its accessible name:
/// Show for a connection that is open, Connect for another.
fn choose_labels(
    connection: &SavedConnection,
    opening: Opening,
    locale: crate::i18n::Locale,
) -> (String, String) {
    let (text, label) = if opening.open.is_some() {
        ("Show", "Show")
    } else {
        ("Connect", "Connect to")
    };
    (
        gettext(locale, text).into_owned(),
        format!("{} {}", gettext(locale, label), connection.name),
    )
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
    opening: Opening,
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
    let response = row_response(ui, rect, connection, opening, look, actions);
    let is_selected = selected == Some(&connection.id);
    let rows = (card.height() / height).round() as usize;
    // The card's inner corners: its 10 less its border.
    let round = |on: bool| if on { 9 } else { 0 };
    let corner = CornerRadius {
        nw: round(index == 0),
        ne: round(index == 0),
        sw: round(index + 1 == rows),
        se: round(index + 1 == rows),
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
    // Each row's rule under it, the grid's lightest line.
    if index + 1 < rows {
        widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.surface);
    }
    let env = connection.environment;
    let colors = crate::env::env_colors(env, crate::env::Platform::of(look), palette);
    let bar = Rect::from_min_size(rect.min, vec2(6.0, rect.height()));
    ui.painter().rect_filled(
        bar,
        CornerRadius {
            nw: corner.nw,
            sw: corner.sw,
            ne: 0,
            se: 0,
        },
        colors.base(),
    );
    // Two lines 3 apart, centred in the 64 pt row.
    let top = rect.top() + 22.7;
    let bottom = rect.top() + 42.0;
    // The columns: a 6 pt bar, 300 for the name, the host taking the rest,
    // 150 for TLS and SSH, 190 for last use, 110 of room, 250 of actions.
    let fixed = 6.0 + 300.0 + 150.0 + 190.0 + 110.0 + 250.0;
    let host_width = (rect.width() - fixed).max(120.0);
    let name_x = rect.left() + 6.0 + 16.0;
    let host_x = rect.left() + 306.0 + 12.0;
    let security_x = rect.left() + 306.0 + host_width + 12.0;
    let used_x = rect.left() + 306.0 + host_width + 150.0 + 12.0;
    let name_width = widgets::paint_text(
        ui,
        name_x,
        top,
        Text::one(
            look,
            TextRole::UiBodySemibold,
            &connection.name,
            palette.text,
        ),
    );
    let badge = super::workspace::env_badge(
        ui,
        name_x + name_width + 8.0,
        top,
        env,
        &colors,
        super::workspace::Badge::Mac,
        look,
    );
    if opening.open.is_some() {
        // Open in a tab already: an outlined tag in the accent, 8 on.
        let text = gettext(locale, "open");
        let laid = Text::one(look, TextRole::Shortcut, &text, palette.accent).layout(ui.ctx());
        let tag = Rect::from_min_size(
            pos2(
                name_x + name_width + 8.0 + badge + 8.0,
                top - (laid.height() + 2.0) / 2.0,
            ),
            vec2(laid.width() + 14.0, laid.height() + 2.0),
        );
        ui.painter().rect_stroke(
            tag,
            CornerRadius::same(10),
            Stroke::new(
                widgets::hairline(ui),
                palette.window.lerp_to_gamma(palette.accent, 0.45),
            ),
            StrokeKind::Inside,
        );
        laid.paint_left(ui.painter(), tag.left() + 7.0, top);
        widgets::announce(ui, tag, &text);
    }
    let user = &connection.spec.user;
    let second = if user.is_empty() {
        connection.spec.driver.label().to_owned()
    } else {
        format!("{} · {user}", connection.spec.driver.label())
    };
    let small = TextRole::Secondary;
    widgets::paint_label(
        ui,
        name_x,
        bottom,
        Text::one(look, small, &second, palette.dim),
    );
    let (host, database) = target(connection);
    let host_role = TextRole::GridCell;
    let shown = crate::ui::grid::ellipsize(&host, host_width - 24.0, false, |text| {
        host_role.width(ui.ctx(), look.faces, text)
    });
    widgets::paint_label(
        ui,
        host_x,
        top,
        Text::one(look, host_role, &shown, palette.text),
    );
    if !database.is_empty() {
        widgets::paint_label(
            ui,
            host_x,
            bottom,
            Text::one(look, TextRole::MonoSecondary, &database, palette.dim),
        );
    }
    let (tls, verified) = tls_label(connection);
    let tls_color = match verified {
        Some(true) => palette.success,
        Some(false) => palette.warning,
        None => palette.dim,
    };
    Icon::Lock.image(tls_color, 11.0).paint_at(
        ui,
        Rect::from_center_size(pos2(security_x + 5.5, top), vec2(11.0, 11.0)),
    );
    widgets::paint_label(
        ui,
        security_x + 16.0,
        top,
        Text::one(look, small, &tls, tls_color),
    );
    widgets::paint_label(
        ui,
        security_x,
        bottom,
        Text::one(look, small, &ssh_label(connection), palette.dim),
    );
    let when = crate::connections::when(store.last_used(&connection.id), now);
    widgets::paint_label(
        ui,
        used_x,
        rect.center().y,
        Text::one(
            look,
            small,
            &format!("{} {when}", gettext(locale, "Last used")),
            palette.dim,
        ),
    );
    // Edit, more, and Connect (Show, for a connection that is open): on
    // the selected row and under the pointer. Connect is always there for
    // keyboards and screen readers.
    let shown = is_selected || ui.rect_contains_pointer(rect);
    let y = rect.center().y;
    let (connect_text, connect_label) = choose_labels(connection, opening, locale);
    // 16 in from the right, 6 apart, 30 tall.
    let connect = ButtonSpec::new(&connect_text)
        .primary()
        .label(&connect_label)
        .salt(&connection.id.0)
        .padding(14.0)
        .radius(7);
    let width = connect.width(ui, look);
    let place = Rect::from_min_size(
        pos2(rect.right() - 16.0 - width, y - 15.0),
        vec2(width, 30.0),
    );
    let response = if shown {
        connect.show_at(ui, place, look, palette)
    } else {
        connect.hidden_at(ui, place)
    };
    if response.clicked() {
        actions.push(opening.choose(connection));
    }
    let more_place =
        Rect::from_min_size(pos2(place.left() - 6.0 - 30.0, y - 15.0), vec2(30.0, 30.0));
    let edit_label = format!("{} {}", gettext(locale, "Edit"), connection.name);
    let edit_text = gettext(locale, "Edit");
    let edit = ButtonSpec::new(&edit_text)
        .label(&edit_label)
        .salt(&connection.id.0)
        .radius(7);
    let edit_width = edit.width(ui, look);
    let edit_place = Rect::from_min_size(
        pos2(more_place.left() - 6.0 - edit_width, y - 15.0),
        vec2(edit_width, 30.0),
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
    let more = ButtonSpec::new("")
        .icon(Icon::Ellipsis)
        .gap(0.0)
        .padding(8.0)
        .label(&more_label)
        .salt(&connection.id.0)
        .radius(7);
    let response = if shown {
        more.show_at(ui, more_place, look, palette)
    } else {
        more.hidden_at(ui, more_place)
    };
    let id = connection.id.clone();
    egui::Popup::menu(&response).show(|ui| {
        let duplicate = gettext(locale, "Duplicate");
        if widgets::button(ui, &duplicate, look).clicked() {
            actions.push(Action::DuplicateConnection(id.clone()));
        }
        let delete = format!("{} {}", gettext(locale, "Delete"), connection.name);
        if widgets::button(ui, &gettext(locale, "Delete"), look)
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
    opening: Opening,
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
    let response = row_response(ui, rect, connection, opening, look, actions);
    let is_selected = selected == Some(&connection.id);
    let center = rect.center().y;
    if is_selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.selection);
        // The cursor: an accent block centred in the 22 pt first column.
        let cursor = Text::one(look, TextRole::OGroup, "▌", palette.accent).layout(ui.ctx());
        cursor.paint_center(ui.painter(), pos2(rect.left() + 11.0, center));
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.05));
    }
    // Columns: 22 for the cursor, 110 for the badge, the rest for the
    // name and its line, 96 for last use (14 in from the right).
    let env = connection.environment;
    let colors = crate::env::env_colors(env, crate::env::Platform::of(look), palette);
    super::workspace::env_badge(
        ui,
        rect.left() + 22.0,
        center,
        env,
        &colors,
        super::workspace::Badge::Plain,
        look,
    );
    let text_x = rect.left() + 22.0 + 110.0;
    // A 13 pt name and a 12 pt line, 3 apart, centred.
    let top = center - 9.8;
    let bottom = center + 9.8;
    let name_width = widgets::paint_text(
        ui,
        text_x,
        top,
        Text::one(look, TextRole::OGroup, &connection.name, palette.text),
    );
    if opening.open.is_some() {
        // Open in a tab already: the word in the accent, 10 on.
        widgets::paint_label(
            ui,
            text_x + name_width + 10.0,
            top,
            Text::one(
                look,
                TextRole::OBody,
                &gettext(locale, "open"),
                palette.accent,
            ),
        );
    }
    let spec = &connection.spec;
    let mut line = spec.summary();
    if let Some(at) = line.find(" via ") {
        line.truncate(at);
    }
    let small = TextRole::OSecondary;
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
    let ssh = match &spec.ssh {
        Some(ssh) => format!("ssh {}", ssh.host),
        None => "no ssh".into(),
    };
    // "dev@localhost:5433/bookshop_development · tls verify-full · no ssh",
    // the separators kept with their neighbours as the design writes them.
    widgets::paint_label(
        ui,
        text_x,
        bottom,
        Text::new(look)
            .add(small, &format!("{line} ·"), palette.dim)
            .space(small, " ")
            .add(small, &tls, tls_color)
            .space(small, " ")
            .add(small, &format!("· {ssh}"), palette.dim),
    );
    let when = crate::connections::when(store.last_used(&connection.id), now).to_lowercase();
    widgets::paint_text_right(
        ui,
        rect.right() - side,
        center,
        Text::one(look, small, &when, palette.dim),
    );
    // Connect (Show, for a connection that is open), for screen readers
    // and the tests; the keys do it here.
    let (_, choose) = choose_labels(connection, opening, locale);
    let hit = Rect::from_min_size(pos2(rect.right() - side - 1.0, rect.top()), vec2(1.0, 1.0));
    if ButtonSpec::new(&choose)
        .salt(&connection.id.0)
        .hidden_at(ui, hit)
        .clicked()
    {
        actions.push(opening.choose(connection));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choosing_a_row_shows_an_open_connection_and_connects_another() {
        let connection = SavedConnection {
            id: ConnectionId::new(),
            name: "Fixture".into(),
            environment: crate::env::Environment::Dev,
            read_only: None,
            password: crate::connections::PasswordMode::None,
            ssh_secret: crate::connections::PasswordMode::None,
            spec: tabletist_db::ConnectSpec::sqlite("/tmp/fixture.db"),
        };
        let picker = crate::model::ConnTabId(1);
        let open = crate::model::ConnTabId(2);
        let connects = |action: Action| {
            let Action::Connect { tab, conn } = action else {
                return false;
            };
            tab == picker && conn == connection.id
        };
        // Not open: choosing it connects in the picker's tab.
        let closed = Opening {
            tab: picker,
            open: None,
        };
        assert!(connects(closed.choose(&connection)));
        // Open: choosing it shows that tab, and it can still connect again
        // (the row menu's Connect again, and Shift+Enter).
        let opened = Opening {
            tab: picker,
            open: Some(open),
        };
        assert!(matches!(
            opened.choose(&connection),
            Action::ActivateConnTab(tab) if tab == open
        ));
        assert!(connects(opened.connect(&connection)));
    }
}
