//! The Data view: footer (view switch, range, paging, timing, cancel) and the
//! grid, or the error or empty state.

use egui::{Frame, Id, Margin, RichText};
use tabletist_db::ValueKind;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, ObjectTabId, ObjectView};
use crate::theme::{self, Icon};
use crate::ui::format;
use crate::ui::grid::{self, Cell, Column};
use crate::ui::widgets::icon_button;

pub fn footer(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else {
        return;
    };
    let view = object.view;
    let loading =
        object.rows.is_loading() || object.structure.is_loading() || object.count.is_loading();
    let counting = object.count.is_loading();
    let count_error = object.count.error.as_ref().map(ToString::to_string);
    let can_count = object.count.value.is_none() && !counting && object.page().is_some();
    let page = object.page();
    let filtered = !object.query.filters.is_empty() || object.query.raw_where.is_some();
    let range = page.and_then(|page| {
        format::range_label(
            object.query.offset,
            page.rows.len(),
            page.has_more,
            // The table's estimate is not the filtered total.
            object.estimated_rows.filter(|_| !filtered),
            object.count.value,
        )
    });
    let can_next = page.is_some_and(|page| page.has_more) && !object.rows.is_loading();
    let can_prev = object.query.offset > 0 && !object.rows.is_loading();
    let timing = page.map(|page| format::elapsed(page.elapsed));
    let unordered = page.is_some_and(|page| !page.ordered_by_key) && object.query.sort.is_empty();
    let mut actions = Vec::new();
    egui::Panel::bottom(Id::new(("object-footer", tab.0, object_tab.0)))
        .exact_size(look.control_height + 8.0)
        .resizable(false)
        .show_separator_line(look.panel_separators)
        .frame(
            Frame::new()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(8, 4)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                for (target, label) in [
                    (ObjectView::Data, gettext(locale, "Data")),
                    (ObjectView::Structure, gettext(locale, "Structure")),
                ] {
                    if crate::ui::widgets::toggle(ui, view == target, &label, &look, &palette)
                        .clicked()
                        && view != target
                    {
                        actions.push(Action::SetView {
                            tab,
                            object_tab,
                            view: target,
                        });
                    }
                }
                ui.separator();
                if view == ObjectView::Data {
                    ui.add_enabled_ui(can_prev, |ui| {
                        if icon_button(
                            ui,
                            Icon::ChevronLeft,
                            &gettext(locale, "Previous page"),
                            &look,
                            &palette,
                        )
                        .clicked()
                        {
                            actions.push(Action::PrevPage { tab, object_tab });
                        }
                    });
                    ui.add_enabled_ui(can_next, |ui| {
                        if icon_button(
                            ui,
                            Icon::ChevronRight,
                            &gettext(locale, "Next page"),
                            &look,
                            &palette,
                        )
                        .clicked()
                        {
                            actions.push(Action::NextPage { tab, object_tab });
                        }
                    });
                    let range = range.clone().unwrap_or_else(|| {
                        if loading {
                            gettext(locale, "Loading…")
                        } else {
                            gettext(locale, "No rows")
                        }
                        .into_owned()
                    });
                    ui.label(
                        RichText::new(range)
                            .size(theme::TEXT_SMALL)
                            .color(palette.secondary),
                    );
                    if filtered {
                        ui.label(RichText::new(gettext(locale, "Filtered")).color(palette.accent))
                            .on_hover_text(gettext(locale, "Cmd/Ctrl+F edits the filter"));
                    }
                    if counting {
                        ui.label(
                            RichText::new(gettext(locale, "Counting…")).color(palette.secondary),
                        );
                    } else if can_count {
                        let button = ui.small_button(gettext(locale, "Count"));
                        let button = match &count_error {
                            Some(error) => button.on_hover_text(error),
                            None => button.on_hover_text(gettext(locale, "Count the rows exactly")),
                        };
                        if button.clicked() {
                            actions.push(Action::CountRows { tab, object_tab });
                        }
                    }
                    if unordered {
                        ui.label(
                            RichText::new(gettext(locale, "Unordered")).color(palette.secondary),
                        )
                        .on_hover_text(gettext(
                            locale,
                            "This object has no primary key, so rows may move between pages.",
                        ));
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if loading {
                        if icon_button(
                            ui,
                            Icon::CircleX,
                            &gettext(locale, "Cancel query"),
                            &look,
                            &palette,
                        )
                        .clicked()
                        {
                            actions.push(Action::CancelQuery(tab));
                        }
                        ui.spinner();
                    } else if let Some(timing) = &timing {
                        ui.label(
                            RichText::new(timing)
                                .size(theme::TEXT_SMALL)
                                .color(palette.dim),
                        );
                    }
                });
            });
        });
    app.actions.extend(actions);
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else {
        return;
    };
    let mut actions = Vec::new();
    if let Some(error) = &object.rows.error {
        Frame::new().inner_margin(Margin::same(12)).show(ui, |ui| {
            error_box(ui, error, &look, &palette, locale, || {
                actions.push(Action::RetryRows { tab, object_tab })
            });
        });
    } else if let Some(page) = object.page() {
        if page.rows.is_empty() {
            let filtered = !object.query.filters.is_empty() || object.query.raw_where.is_some();
            let table = object.kind == tabletist_db::ObjectKind::Table;
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() / 3.0);
                let text = if filtered {
                    gettext(locale, "No rows match the filter")
                } else if table {
                    gettext(locale, "This table is empty")
                } else {
                    gettext(locale, "No rows")
                };
                ui.label(RichText::new(text).color(palette.secondary));
                if filtered {
                    ui.add_space(8.0);
                    if ui.button(gettext(locale, "Clear filter")).clicked() {
                        actions.push(Action::ClearFilters { tab, object_tab });
                    }
                }
            });
        } else {
            let columns: Vec<Column<'_>> = page
                .columns
                .iter()
                .map(|column| Column {
                    name: &column.name,
                    type_name: &column.type_name,
                    numeric: column.kind == ValueKind::Numeric,
                    sort: object.sort_of(&column.name),
                })
                .collect();
            let output = grid::show(
                ui,
                Id::new(("grid", tab.0, object_tab.0)),
                &columns,
                page.rows.len(),
                object.query.offset,
                object.selection,
                &palette,
                &look,
                |row, col| {
                    let value = &page.rows[row][col];
                    Cell {
                        text: format::cell_text(value),
                        null: value.is_null(),
                    }
                },
            );
            if let Some(cell) = output.clicked {
                actions.push(Action::SelectCell {
                    tab,
                    object_tab,
                    cell,
                });
            }
            if let Some(col) = output.sort_clicked {
                actions.push(Action::SortBy {
                    tab,
                    object_tab,
                    column: page.columns[col].name.clone(),
                });
            }
        }
    } else {
        ui.centered_and_justified(|ui| {
            ui.spinner();
        });
    }
    app.actions.extend(actions);
}

/// An error with its code, detail and hint, and a Retry button.
pub fn error_box(
    ui: &mut egui::Ui,
    error: &tabletist_db::Error,
    look: &crate::theme::Look,
    palette: &crate::theme::Palette,
    locale: crate::i18n::Locale,
    mut retry: impl FnMut(),
) {
    Frame::new()
        .fill(palette.danger.gamma_multiply(0.12))
        .inner_margin(Margin::same(12))
        .corner_radius(egui::CornerRadius::same(look.tab_radius))
        .show(ui, |ui| {
            ui.label(RichText::new(error.to_string()).color(palette.text));
            if let tabletist_db::Error::Query {
                code, detail, hint, ..
            } = error
            {
                for (label, text) in [("Code", code), ("Detail", detail), ("Hint", hint)] {
                    if let Some(text) = text {
                        ui.label(
                            RichText::new(format!("{}: {text}", gettext(locale, label)))
                                .color(palette.secondary),
                        );
                    }
                }
            }
            if ui.button(gettext(locale, "Retry")).clicked() {
                retry();
            }
        });
}
