//! The body of a picker tab: search, saved connections, New.

use egui::{Align, Layout, RichText, Sense, vec2};

use crate::app::App;
use crate::connections::ConnectionId;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabContent};
use crate::theme::{self, Icon};
use crate::ui::widgets::icon_button;

const ROW_HEIGHT: f32 = 56.0;
const LIST_WIDTH: f32 = 560.0;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let tab = app.active_tab_id();
    let index = app.active;
    ui.vertical_centered(|ui| {
        ui.add_space(64.0);
        ui.set_max_width(LIST_WIDTH);
        ui.label(
            RichText::new(gettext(locale, "Connections"))
                .font(theme::semibold(theme::TEXT_HEADING))
                .color(palette.text),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(gettext(
                locale,
                "Choose a saved connection or create a new one",
            ))
            .color(palette.secondary),
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            // The field takes what the button leaves, so the row spans the cards.
            let new = gettext(locale, "New connection");
            let button = ui
                .painter()
                .layout_no_wrap(
                    new.to_string(),
                    theme::medium(theme::TEXT),
                    palette.on_accent,
                )
                .size()
                .x
                + 2.0 * ui.spacing().button_padding.x;
            if let ConnTabContent::Picker(picker) = &mut app.tabs[index].content {
                let field = crate::ui::widgets::search_field(
                    ui,
                    &mut picker.search,
                    &gettext(locale, "Search connections"),
                    &look,
                )
                .desired_width(LIST_WIDTH - button - ui.spacing().item_spacing.x);
                crate::ui::widgets::add_search(ui, field, &look);
            }
            if crate::ui::widgets::primary_button(ui, &new, &look, &palette).clicked() {
                app.actions.push(Action::NewConnection);
            }
        });
        ui.add_space(12.0);

        let search = match &app.tabs[index].content {
            ConnTabContent::Picker(picker) => picker.search.clone(),
            ConnTabContent::Workspace(_) => String::new(),
        };
        let rows: Vec<(ConnectionId, String, String, Option<egui::Color32>)> = app
            .connections
            .search(&search)
            .into_iter()
            .map(|saved| {
                // The second line: the driver, then where it connects.
                let driver = saved.spec.driver.label();
                let summary = saved.spec.summary();
                let detail = if summary.is_empty() {
                    driver.to_owned()
                } else {
                    format!("{driver} · {summary}")
                };
                (
                    saved.id.clone(),
                    saved.name.clone(),
                    detail,
                    saved.color.color(),
                )
            })
            .collect();
        if app.connections.connections.is_empty() {
            ui.label(
                RichText::new(gettext(locale, "No saved connections yet")).color(palette.secondary),
            );
            return;
        }
        if rows.is_empty() {
            ui.label(
                RichText::new(gettext(locale, "No connections match")).color(palette.secondary),
            );
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, name, summary, color) in rows {
                let (rect, response) =
                    ui.allocate_exact_size(vec2(LIST_WIDTH, ROW_HEIGHT), Sense::click());
                let connect = format!("{} {name}", gettext(locale, "Connect to"));
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name)
                });
                crate::ui::widgets::card(ui, rect, response.hovered(), &look, &palette);
                // Double-click, or Enter/Space/screen-reader activation (egui
                // reports those as a click not made by the pointer).
                let activated = response.double_clicked()
                    || (response.clicked() && !response.clicked_by(egui::PointerButton::Primary));
                if activated {
                    app.actions.push(Action::Connect {
                        tab,
                        conn: id.clone(),
                    });
                }
                response.context_menu(|ui| {
                    if ui.button(gettext(locale, "Connect")).clicked() {
                        app.actions.push(Action::Connect {
                            tab,
                            conn: id.clone(),
                        });
                    }
                    if ui.button(gettext(locale, "Edit…")).clicked() {
                        app.actions.push(Action::EditConnection(id.clone()));
                    }
                    if ui.button(gettext(locale, "Duplicate")).clicked() {
                        app.actions.push(Action::DuplicateConnection(id.clone()));
                    }
                    if ui.button(gettext(locale, "Delete")).clicked() {
                        app.actions.push(Action::DeleteConnection(id.clone()));
                    }
                });
                let mut row = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect.shrink2(vec2(14.0, 0.0)))
                        .layout(Layout::left_to_right(Align::Center)),
                );
                let dot = color.unwrap_or(palette.dim);
                let (dot_rect, _) = row.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                row.painter().circle_filled(dot_rect.center(), 4.0, dot);
                // The two lines sit in the middle of the card, whatever its height.
                let lines = row.fonts_mut(|fonts| {
                    fonts.row_height(&theme::medium(theme::TEXT))
                        + fonts.row_height(&theme::regular(theme::TEXT_SMALL))
                });
                row.vertical(|ui| {
                    let gap = (ROW_HEIGHT - lines - ui.spacing().item_spacing.y) / 2.0;
                    ui.add_space(gap.max(0.0));
                    ui.label(
                        RichText::new(&name)
                            .font(theme::medium(theme::TEXT))
                            .color(palette.text),
                    );
                    ui.label(
                        RichText::new(&summary)
                            .font(theme::regular(theme::TEXT_SMALL))
                            .color(palette.secondary),
                    );
                });
                row.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let delete = format!("{} {name}", gettext(locale, "Delete"));
                    if icon_button(ui, Icon::Trash2, &delete, &look, &palette).clicked() {
                        app.actions.push(Action::DeleteConnection(id.clone()));
                    }
                    let edit = format!("{} {name}", gettext(locale, "Edit"));
                    if icon_button(ui, Icon::Pencil, &edit, &look, &palette).clicked() {
                        app.actions.push(Action::EditConnection(id.clone()));
                    }
                    if icon_button(ui, Icon::ChevronRight, &connect, &look, &palette).clicked() {
                        app.actions.push(Action::Connect {
                            tab,
                            conn: id.clone(),
                        });
                    }
                });
                ui.add_space(8.0);
            }
        });
    });
}
