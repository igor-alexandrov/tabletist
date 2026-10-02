//! The Structure view: columns, indexes and foreign keys as plain tables.

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, TabId};
use crate::theme::{self, Look, Palette};
use crate::typography::TextRole;
use crate::ui::format::{display_safe, for_display};
use crate::ui::widgets;

/// One cell of a structure table: body text, or code.
fn cell(ui: &mut egui::Ui, text: &str, code: bool, look: &Look, palette: &Palette) {
    let role = if code {
        widgets::code(look)
    } else {
        widgets::body(look)
    };
    widgets::label(ui, role, text, palette.text, look);
}

/// Column names as a list, each as `display_safe` shows it.
fn names(columns: &[String]) -> String {
    columns
        .iter()
        .map(|column| display_safe(column))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A table's column title, or a quiet note.
fn quiet(ui: &mut egui::Ui, text: &str, look: &Look, palette: &Palette) {
    widgets::label(ui, widgets::body(look), text, palette.secondary, look);
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else {
        return;
    };
    let mut actions = Vec::new();
    let area = ui.max_rect();
    if let Some(error) = super::data_view::shown_error(&object.structure) {
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
                        heading(ui, &gettext(locale, "Columns"), &look, &palette);
                        egui::Grid::new(("structure-columns", object_tab.0))
                            .striped(true)
                            .spacing([16.0, 6.0])
                            .show(ui, |ui| {
                                for title in ["Name", "Type", "Nullable", "Default", "Key"] {
                                    quiet(ui, &gettext(locale, title), &look, &palette);
                                }
                                ui.end_row();
                                for column in &structure.columns {
                                    cell(ui, &display_safe(&column.name), false, &look, &palette);
                                    let type_name = display_safe(&column.type_name);
                                    cell(ui, &type_name, true, &look, &palette);
                                    let nullable = if column.nullable {
                                        gettext(locale, "yes")
                                    } else {
                                        gettext(locale, "no")
                                    };
                                    cell(ui, &nullable, false, &look, &palette);
                                    let default =
                                        for_display(column.default.as_deref().unwrap_or(""));
                                    cell(ui, &default, true, &look, &palette);
                                    let key = if structure.primary_key.contains(&column.name) {
                                        "PK"
                                    } else {
                                        ""
                                    };
                                    cell(ui, key, false, &look, &palette);
                                    ui.end_row();
                                }
                            });
                        ui.add_space(16.0);
                        heading(ui, &gettext(locale, "Indexes"), &look, &palette);
                        if structure.indexes.is_empty() {
                            let text = gettext(locale, "No indexes");
                            quiet(ui, &text, &look, &palette);
                        } else {
                            egui::Grid::new(("structure-indexes", object_tab.0))
                                .striped(true)
                                .spacing([16.0, 6.0])
                                .show(ui, |ui| {
                                    for title in ["Name", "Columns", "Unique", "Method"] {
                                        quiet(ui, &gettext(locale, title), &look, &palette);
                                    }
                                    ui.end_row();
                                    for index in &structure.indexes {
                                        cell(
                                            ui,
                                            &display_safe(&index.name),
                                            false,
                                            &look,
                                            &palette,
                                        );
                                        let columns = names(&index.columns);
                                        cell(ui, &columns, false, &look, &palette);
                                        let unique = if index.primary {
                                            "PK"
                                        } else if index.unique {
                                            "yes"
                                        } else {
                                            ""
                                        };
                                        cell(ui, unique, false, &look, &palette);
                                        let method =
                                            display_safe(index.method.as_deref().unwrap_or(""));
                                        cell(ui, &method, false, &look, &palette);
                                        ui.end_row();
                                    }
                                });
                        }
                        ui.add_space(16.0);
                        heading(ui, &gettext(locale, "Foreign keys"), &look, &palette);
                        if structure.foreign_keys.is_empty() {
                            let text = gettext(locale, "No foreign keys");
                            quiet(ui, &text, &look, &palette);
                        } else {
                            egui::Grid::new(("structure-fks", object_tab.0))
                                .striped(true)
                                .spacing([16.0, 6.0])
                                .show(ui, |ui| {
                                    for title in
                                        ["Name", "Columns", "References", "On update", "On delete"]
                                    {
                                        quiet(ui, &gettext(locale, title), &look, &palette);
                                    }
                                    ui.end_row();
                                    for key in &structure.foreign_keys {
                                        let name = display_safe(key.name.as_deref().unwrap_or("·"));
                                        cell(ui, &name, false, &look, &palette);
                                        let columns = names(&key.columns);
                                        cell(ui, &columns, false, &look, &palette);
                                        let table = format!(
                                            "{}.{}",
                                            display_safe(&key.ref_schema),
                                            display_safe(&key.ref_table)
                                        );
                                        let target = if key.ref_columns.is_empty() {
                                            table
                                        } else {
                                            format!("{table} ({})", names(&key.ref_columns))
                                        };
                                        cell(ui, &target, false, &look, &palette);
                                        cell(
                                            ui,
                                            &display_safe(&key.on_update),
                                            false,
                                            &look,
                                            &palette,
                                        );
                                        cell(
                                            ui,
                                            &display_safe(&key.on_delete),
                                            false,
                                            &look,
                                            &palette,
                                        );
                                        ui.end_row();
                                    }
                                });
                        }
                    });
            });
    }
    // A fetch that has lasted, over whatever is up: a refresh keeps the
    // structure it replaces on screen.
    if let Some(waited) = object.structure.running_for() {
        if super::states::lasted(waited) {
            let (text, name, keys) = (
                look.label(&gettext(locale, "Loading structure…")),
                look.label(&gettext(locale, "Cancel")),
                super::data_view::cancel_keys(&look),
            );
            let cancel = super::states::key_button(&name, &keys, &look).label("Cancel query");
            if super::states::running(ui, area, &text, waited, Some(cancel), &look, &palette) {
                actions.push(Action::CancelQuery(tab));
            }
        } else {
            // Come back when the wait is long enough to show.
            ui.ctx()
                .request_repaint_after(super::states::DELAY - waited);
        }
    }
    app.actions.extend(actions);
}

fn heading(ui: &mut egui::Ui, text: &str, look: &Look, palette: &Palette) {
    let role = TextRole::pick(look, TextRole::TableTitle, TextRole::OTableTitle);
    widgets::label(ui, role, text, palette.text, look);
    ui.add_space(4.0);
}
