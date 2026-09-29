//! The row panel: every field of the selected row, in full.

use egui::{Frame, Id, Margin, RichText, TextEdit};
use tabletist_db::ValueKind;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{ConnTabId, ObjectTabId};
use crate::theme::{self, Icon};
use crate::ui::format;
use crate::ui::widgets::icon_button;

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
        .show_separator_line(look.panel_separators)
        .frame(
            Frame::new()
                .fill(palette.panel)
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
            ui.label(
                RichText::new(format!(
                    "{} · {} {}",
                    object.object.name,
                    gettext(locale, "Row"),
                    object.query.offset + cell.row as u64 + 1
                ))
                .font(theme::semibold(theme::TEXT_TITLE))
                .color(palette.text),
            );
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
                    for (col, (column, value)) in page.columns.iter().zip(row.iter()).enumerate() {
                        if !needle.is_empty() && !column.name.to_lowercase().contains(&needle) {
                            continue;
                        }
                        // The field name names its value for screen readers.
                        let name = ui
                            .horizontal(|ui| {
                                let name = ui
                                    .label(
                                        RichText::new(&column.name)
                                            .font(theme::medium(theme::TEXT))
                                            .color(palette.text),
                                    )
                                    .id;
                                ui.label(
                                    RichText::new(&column.type_name)
                                        .font(theme::regular(theme::TEXT_SMALL))
                                        .color(palette.dim),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let label =
                                            format!("{} {}", gettext(locale, "Copy"), column.name);
                                        if icon_button(ui, Icon::Copy, &label, &look, &palette)
                                            .clicked()
                                        {
                                            ui.ctx().copy_text(format::plain_text(value));
                                        }
                                    },
                                );
                                name
                            })
                            .inner;
                        if value.is_null() {
                            ui.label(
                                RichText::new("NULL")
                                    .font(theme::mono(theme::TEXT_MONO))
                                    .color(palette.dim),
                            );
                        } else {
                            let text = format::full_text(value, column.kind);
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
                        ui.add_space(6.0);
                    }
                });
        });
}
