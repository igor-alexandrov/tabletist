//! A connection tab: top bar, disconnected banner, and (batch 3) the body.

use egui::{Frame, Margin, RichText};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, ObjectView, SessionStatus};
use crate::theme::{self, Icon};
use crate::ui::widgets::icon_button;

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    top_bar(app, ui, tab);
    banner(app, ui, tab);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    if workspace.tree.schemas.value.is_none() && workspace.tree.schemas.error.is_none() {
        return; // still connecting; the banner shows progress
    }
    let active = workspace.active_object;
    let view = workspace.active_object_tab().map(|object| object.view);
    let row_panel = workspace.row_panel;
    super::sidebar::show(app, ui, tab);
    if let (Some(object_tab), Some(ObjectView::Data), true) = (active, view, row_panel) {
        super::row_panel::show(app, ui, tab, object_tab);
    }
    egui::CentralPanel::default()
        .frame(Frame::new().fill(app.palette.window))
        .show(ui, |ui| {
            super::object_tabs::show(app, ui, tab);
            match active {
                Some(object_tab) => {
                    super::data_view::footer(app, ui, tab, object_tab);
                    egui::CentralPanel::default()
                        .frame(Frame::new().fill(app.palette.window))
                        .show(ui, |ui| match view {
                            Some(ObjectView::Structure) => {
                                super::structure::show(app, ui, tab, object_tab)
                            }
                            _ => {
                                if super::filter_bar::is_open(app, tab, object_tab) {
                                    let palette = app.palette;
                                    let look = app.look;
                                    egui::Panel::top(egui::Id::new((
                                        "filter-bar",
                                        tab.0,
                                        object_tab.0,
                                    )))
                                    .resizable(false)
                                    .show_separator_line(look.panel_separators)
                                    .frame(
                                        Frame::new()
                                            .fill(palette.panel)
                                            .inner_margin(egui::Margin::symmetric(8, 6)),
                                    )
                                    .show(ui, |ui| {
                                        super::filter_bar::show(app, ui, tab, object_tab)
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

fn top_bar(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let name = workspace.name.clone();
    let summary = workspace.spec.summary();
    let color = workspace.color.color().unwrap_or(palette.dim);
    let driver = workspace.driver.label();
    let databases = workspace.databases.value.clone().unwrap_or_default();
    let current = workspace.spec.database.clone();
    let tls = (workspace.driver != tabletist_db::Driver::Sqlite).then_some(workspace.spec.tls);
    let ssh_host = workspace.spec.ssh.as_ref().map(|ssh| ssh.host.clone());
    let mut actions = Vec::new();
    egui::Panel::top(egui::Id::new(("workspace-top", tab.0)))
        .exact_size(look.control_height + 8.0)
        .resizable(false)
        .show_separator_line(look.panel_separators)
        .frame(
            Frame::new()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(10, 4)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                ui.painter().circle_filled(dot.center(), 4.0, color);
                ui.label(
                    RichText::new(&name)
                        .font(theme::medium(theme::TEXT))
                        .color(palette.text),
                );
                ui.label(RichText::new(format!("{driver} · {summary}")).color(palette.secondary));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(
                        ui,
                        Icon::LogOut,
                        &gettext(locale, "Disconnect"),
                        &look,
                        &palette,
                    )
                    .clicked()
                    {
                        actions.push(Action::Disconnect(tab));
                    }
                    if let Some(host) = &ssh_host {
                        ui.label(
                            RichText::new(format!("{} {host}", gettext(locale, "via SSH")))
                                .color(palette.secondary),
                        );
                    }
                    if let Some(mode) = tls {
                        use tabletist_db::TlsMode;
                        let (text, color) = match mode {
                            TlsMode::Disable => (gettext(locale, "Not encrypted"), palette.warning),
                            TlsMode::Prefer => {
                                (gettext(locale, "TLS if offered"), palette.secondary)
                            }
                            TlsMode::Require => (gettext(locale, "TLS"), palette.secondary),
                            TlsMode::VerifyCa | TlsMode::VerifyFull => {
                                (gettext(locale, "TLS verified"), palette.secondary)
                            }
                        };
                        ui.label(RichText::new(text).color(color));
                    }
                    if databases.len() > 1 {
                        let mut chosen = current.clone();
                        let combo = egui::ComboBox::from_id_salt(("database", tab.0))
                            .selected_text(&chosen)
                            .show_ui(ui, |ui| {
                                for database in &databases {
                                    ui.selectable_value(&mut chosen, database.clone(), database);
                                }
                            });
                        // No visible label: name it for screen readers,
                        // keeping the current database as the value.
                        combo.response.widget_info(|| {
                            let mut info = egui::WidgetInfo::labeled(
                                egui::WidgetType::ComboBox,
                                true,
                                gettext(locale, "Database"),
                            );
                            info.current_text_value = Some(current.clone());
                            info
                        });
                        combo.response.on_hover_text(gettext(locale, "Database"));
                        if chosen != current {
                            actions.push(Action::SwitchDatabase {
                                tab,
                                database: chosen,
                            });
                        }
                    }
                });
            });
        });
    app.actions.extend(actions);
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
