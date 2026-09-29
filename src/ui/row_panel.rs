//! The row panel: every field of the selected row, in full.

use egui::{Frame, Id, Margin, Rect, RichText, TextEdit, UiBuilder, pos2, vec2};
use std::sync::Arc;

use tabletist_db::{Value, ValueKind};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{ConnTabId, ObjectTabId};
use crate::theme::{self, Icon};
use crate::ui::format;
use crate::ui::json_view;
use crate::ui::widgets::icon_button;

/// A column's type as the row panel shows it after the field name.
fn type_label(type_name: &str) -> String {
    format!("({type_name})")
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else {
        return;
    };
    let filter_id = Id::new(("row-panel-filter", tab.0, object_tab.0));
    egui::Panel::right(Id::new(("row-panel", tab.0)))
        .resizable(true)
        .default_size(300.0)
        .size_range(240.0..=560.0)
        .show_separator_line(true)
        // The content colour: the panel shows the row's data, as the grid
        // does, and its labels read as secondary against it.
        .frame(
            Frame::new()
                .fill(palette.window)
                .inner_margin(Margin::same(10)),
        )
        .show(ui, |ui| {
            let selected = object
                .selection
                .and_then(|cell| object.page().map(|page| (cell, page)))
                .and_then(|(cell, page)| page.rows.get(cell.row).map(|row| (cell, page, row)));
            let Some((cell, page, row)) = selected else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new(gettext(locale, "Select a row to see its fields"))
                            .color(palette.secondary),
                    );
                });
                return;
            };
            let mut filter: String = ui.data(|data| data.get_temp(filter_id)).unwrap_or_default();
            let field = crate::ui::widgets::search_field(
                ui,
                &mut filter,
                &gettext(locale, "Filter fields"),
                &look,
            )
            .desired_width(f32::INFINITY);
            crate::ui::widgets::add_search(ui, field, &look);
            ui.data_mut(|data| data.insert_temp(filter_id, filter.clone()));
            let needle = filter.trim().to_lowercase();
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // A label sits close to its value and farther from the
                    // field above, so each pair reads as one.
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for (col, (column, value)) in page.columns.iter().zip(row.iter()).enumerate() {
                        if !needle.is_empty() && !column.name.to_lowercase().contains(&needle) {
                            continue;
                        }
                        // The field name names its value for screen readers.
                        // The row is only as tall as its small text (a
                        // horizontal row is otherwise a control tall), so the
                        // value sits right under its name.
                        let row = ui.scope(|ui| {
                            ui.spacing_mut().interact_size.y = 0.0;
                            ui.horizontal(|ui| {
                                // A small, secondary label, so the value
                                // under it leads.
                                let name = ui
                                    .label(
                                        RichText::new(&column.name)
                                            .font(theme::medium(theme::TEXT_SMALL))
                                            .color(palette.secondary),
                                    )
                                    .id;
                                ui.spacing_mut().item_spacing.x = 4.0;
                                ui.label(
                                    RichText::new(type_label(&column.type_name))
                                        .font(theme::regular(theme::TEXT_SMALL))
                                        .color(palette.dim),
                                );
                                name
                            })
                        });
                        let name = row.inner.inner;
                        // The copy button overhangs the row rather than
                        // making it taller.
                        let center = row.response.rect.center().y;
                        let button = Rect::from_center_size(
                            pos2(ui.max_rect().right() - 12.0, center),
                            vec2(24.0, 24.0),
                        );
                        let mut button_ui = ui.new_child(UiBuilder::new().max_rect(button));
                        let label = format!("{} {}", gettext(locale, "Copy"), column.name);
                        if icon_button(&mut button_ui, Icon::Copy, &label, &look, &palette)
                            .clicked()
                        {
                            ui.ctx().copy_text(format::plain_text(value));
                        }
                        if value.is_null() {
                            ui.label(
                                RichText::new("NULL")
                                    .font(theme::mono(theme::TEXT_MONO))
                                    .color(palette.dim),
                            );
                        } else if let Some(doc) = json_doc(ui, value, column.kind) {
                            let id =
                                Id::new(("row-panel-json", tab.0, object_tab.0, cell.row, col));
                            json_view::show(ui, id, &doc, &column.name, locale, &palette);
                        } else {
                            let text = format::full_text(value);
                            let expanded_id =
                                Id::new(("row-panel-expanded", tab.0, object_tab.0, cell.row, col));
                            let expanded: bool =
                                ui.data(|data| data.get_temp(expanded_id)).unwrap_or(false);
                            let lines = text.lines().count();
                            let size = format::human_size(text.len());
                            let long = lines > format::COLLAPSE_LINES
                                || text.len() > format::COLLAPSE_CHARS;
                            let shown: String = if long && !expanded {
                                text.lines()
                                    .take(format::COLLAPSE_LINES)
                                    .collect::<Vec<_>>()
                                    .join("\n")
                                    .chars()
                                    .take(format::COLLAPSE_CHARS)
                                    .collect()
                            } else {
                                text
                            };
                            let shown = format::for_display(&shown);
                            let mono = matches!(column.kind, ValueKind::Json | ValueKind::Binary);
                            let font = if mono {
                                theme::mono(theme::TEXT_MONO)
                            } else {
                                theme::data(&look)
                            };
                            ui.add(
                                TextEdit::multiline(&mut shown.as_str())
                                    .font(font)
                                    // Flush with the field name above.
                                    .frame(egui::Frame::NONE)
                                    .margin(egui::Margin::ZERO)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(1),
                            )
                            .labelled_by(name);
                            if long {
                                let label = if expanded {
                                    gettext(locale, "Show less").into_owned()
                                } else {
                                    format!("{} ({size})", gettext(locale, "Show all"))
                                };
                                if ui.link(label).clicked() {
                                    ui.data_mut(|data| data.insert_temp(expanded_id, !expanded));
                                }
                            }
                        }
                        ui.add_space(12.0);
                    }
                });
        });
}

/// A JSON column's value as a tree, when it parses and is small enough.
fn json_doc(ui: &egui::Ui, value: &Value, kind: ValueKind) -> Option<Arc<json_view::Doc>> {
    match value {
        Value::Text(text) if kind == ValueKind::Json && text.len() <= json_view::TREE_MAX => {
            json_view::parsed(ui, text)
        }
        _ => None,
    }
}
