//! The Structure view: columns, indexes and foreign keys as plain tables.

use egui::RichText;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, ObjectTabId};
use crate::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else {
        return;
    };
    let mut actions = Vec::new();
    if let Some(error) = &object.structure.error {
        egui::Frame::new()
            .inner_margin(egui::Margin::same(theme::PAD))
            .show(ui, |ui| {
                super::data_view::error_box(ui, error, &look, &palette, locale, || {
                    actions.push(Action::RetryStructure { tab, object_tab })
                });
            });
    } else if let Some(structure) = &object.structure.value {
        // Both ways: a long default (a sequence's nextval) must not push
        // the Key column out of reach.
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin::same(theme::PAD))
                    .show(ui, |ui| {
                        heading(ui, &gettext(locale, "Columns"), &palette);
                        egui::Grid::new(("structure-columns", object_tab.0))
                            .striped(true)
                            .spacing([16.0, 6.0])
                            .show(ui, |ui| {
                                for title in ["Name", "Type", "Nullable", "Default", "Key"] {
                                    ui.label(
                                        RichText::new(gettext(locale, title))
                                            .color(palette.secondary),
                                    );
                                }
                                ui.end_row();
                                for column in &structure.columns {
                                    ui.label(&column.name);
                                    ui.label(
                                        RichText::new(&column.type_name)
                                            .font(theme::mono(theme::TEXT_MONO)),
                                    );
                                    ui.label(if column.nullable {
                                        gettext(locale, "yes")
                                    } else {
                                        gettext(locale, "no")
                                    });
                                    ui.label(
                                        RichText::new(column.default.as_deref().unwrap_or(""))
                                            .font(theme::mono(theme::TEXT_MONO)),
                                    );
                                    ui.label(if structure.primary_key.contains(&column.name) {
                                        "PK"
                                    } else {
                                        ""
                                    });
                                    ui.end_row();
                                }
                            });
                        ui.add_space(16.0);
                        heading(ui, &gettext(locale, "Indexes"), &palette);
                        if structure.indexes.is_empty() {
                            ui.label(
                                RichText::new(gettext(locale, "No indexes"))
                                    .color(palette.secondary),
                            );
                        } else {
                            egui::Grid::new(("structure-indexes", object_tab.0))
                                .striped(true)
                                .spacing([16.0, 6.0])
                                .show(ui, |ui| {
                                    for title in ["Name", "Columns", "Unique", "Method"] {
                                        ui.label(
                                            RichText::new(gettext(locale, title))
                                                .color(palette.secondary),
                                        );
                                    }
                                    ui.end_row();
                                    for index in &structure.indexes {
                                        ui.label(&index.name);
                                        ui.label(index.columns.join(", "));
                                        ui.label(if index.primary {
                                            "PK"
                                        } else if index.unique {
                                            "yes"
                                        } else {
                                            ""
                                        });
                                        ui.label(index.method.as_deref().unwrap_or(""));
                                        ui.end_row();
                                    }
                                });
                        }
                        ui.add_space(16.0);
                        heading(ui, &gettext(locale, "Foreign keys"), &palette);
                        if structure.foreign_keys.is_empty() {
                            ui.label(
                                RichText::new(gettext(locale, "No foreign keys"))
                                    .color(palette.secondary),
                            );
                        } else {
                            egui::Grid::new(("structure-fks", object_tab.0))
                                .striped(true)
                                .spacing([16.0, 6.0])
                                .show(ui, |ui| {
                                    for title in
                                        ["Name", "Columns", "References", "On update", "On delete"]
                                    {
                                        ui.label(
                                            RichText::new(gettext(locale, title))
                                                .color(palette.secondary),
                                        );
                                    }
                                    ui.end_row();
                                    for key in &structure.foreign_keys {
                                        ui.label(key.name.as_deref().unwrap_or("·"));
                                        ui.label(key.columns.join(", "));
                                        let target = if key.ref_columns.is_empty() {
                                            format!("{}.{}", key.ref_schema, key.ref_table)
                                        } else {
                                            format!(
                                                "{}.{} ({})",
                                                key.ref_schema,
                                                key.ref_table,
                                                key.ref_columns.join(", ")
                                            )
                                        };
                                        ui.label(target);
                                        ui.label(&key.on_update);
                                        ui.label(&key.on_delete);
                                        ui.end_row();
                                    }
                                });
                        }
                    });
            });
    } else {
        ui.centered_and_justified(|ui| {
            ui.spinner();
        });
    }
    app.actions.extend(actions);
}

fn heading(ui: &mut egui::Ui, text: &str, palette: &crate::theme::Palette) {
    ui.label(
        RichText::new(text)
            .font(theme::semibold(theme::TEXT_TITLE))
            .color(palette.text),
    );
    ui.add_space(4.0);
}
