//! The filter bar above the grid: column, operator and value rows combined
//! with AND, and an optional raw WHERE clause.

use tabletist_db::FilterOp;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, TabId};
use crate::theme::Icon;
use crate::ui::format::display_safe;
use crate::ui::widgets::{self, icon_button};

pub const FILTER_OPS: [FilterOp; 11] = [
    FilterOp::Eq,
    FilterOp::Ne,
    FilterOp::Lt,
    FilterOp::Gt,
    FilterOp::Le,
    FilterOp::Ge,
    FilterOp::Contains,
    FilterOp::StartsWith,
    FilterOp::In,
    FilterOp::IsNull,
    FilterOp::IsNotNull,
];

pub fn op_label(op: FilterOp) -> &'static str {
    match op {
        FilterOp::Eq => "=",
        FilterOp::Ne => "≠",
        FilterOp::Lt => "<",
        FilterOp::Gt => ">",
        FilterOp::Le => "≤",
        FilterOp::Ge => "≥",
        FilterOp::Contains => "contains",
        FilterOp::StartsWith => "starts with",
        FilterOp::In => "in (a, b, …)",
        FilterOp::IsNull => "is NULL",
        FilterOp::IsNotNull => "is not NULL",
    }
}

/// Whether the active object tab's filter bar is open.
pub fn is_open(app: &App, tab: ConnTabId, object_tab: TabId) -> bool {
    app.workspace(tab)
        .and_then(|workspace| workspace.object_tab(object_tab))
        .is_some_and(|object| object.filter.open)
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(object) = app
        .workspace_mut(tab)
        .and_then(|workspace| workspace.object_tab_mut(object_tab))
    else {
        return;
    };
    if !object.filter.open {
        return;
    }
    // Columns from the page, else the structure, else the last ones seen.
    let columns: Vec<String> = object
        .page()
        .map(|page| page.columns.iter().map(|c| c.name.clone()).collect())
        .or_else(|| {
            object
                .structure
                .value
                .as_ref()
                .map(|s| s.columns.iter().map(|c| c.name.clone()).collect())
        })
        .unwrap_or_else(|| object.filter.columns.clone());
    let mut actions = Vec::new();
    let mut apply = false;
    let focus = std::mem::take(&mut object.filter.focus);
    let bar = &mut object.filter;
    let count = bar.rows.len();
    // Where focus goes: the last value field, else the raw WHERE field,
    // else "+ Condition", so something always takes the keys.
    let value_row = bar
        .rows
        .iter()
        .rposition(|row| !matches!(row.op, FilterOp::IsNull | FilterOp::IsNotNull));
    for index in 0..count {
        let row = &mut bar.rows[index];
        ui.horizontal(|ui| {
            let combo = crate::ui::widgets::popup_button(
                ui,
                egui::ComboBox::from_id_salt(("filter-column", tab.0, object_tab.0, index))
                    .selected_text(widgets::galley(
                        ui,
                        &display_safe(&row.column),
                        egui::Color32::PLACEHOLDER,
                        &look,
                    ))
                    .width(160.0),
                &look,
                &palette,
                |ui| {
                    for column in &columns {
                        let name = display_safe(column);
                        let text = widgets::galley(ui, &name, egui::Color32::PLACEHOLDER, &look);
                        ui.selectable_value(&mut row.column, column.clone(), text);
                    }
                },
            );
            // No visible label in the row: name it for screen readers,
            // keeping its selection as the value.
            let selected = Some(display_safe(&row.column).into_owned());
            combo.response.widget_info(|| {
                let mut info = egui::WidgetInfo::labeled(
                    egui::WidgetType::ComboBox,
                    true,
                    gettext(locale, "Filter column"),
                );
                info.current_text_value = selected.clone();
                info
            });
            let combo = crate::ui::widgets::popup_button(
                ui,
                egui::ComboBox::from_id_salt(("filter-op", tab.0, object_tab.0, index))
                    .selected_text(widgets::galley(
                        ui,
                        &gettext(locale, op_label(row.op)),
                        egui::Color32::PLACEHOLDER,
                        &look,
                    ))
                    .width(110.0),
                &look,
                &palette,
                |ui| {
                    for op in FILTER_OPS {
                        let label = gettext(locale, op_label(op));
                        let text = widgets::galley(ui, &label, egui::Color32::PLACEHOLDER, &look);
                        ui.selectable_value(&mut row.op, op, text);
                    }
                },
            );
            // No visible label in the row: name it for screen readers,
            // keeping its selection as the value.
            let selected = Some(gettext(locale, op_label(row.op)).into_owned());
            combo.response.widget_info(|| {
                let mut info = egui::WidgetInfo::labeled(
                    egui::WidgetType::ComboBox,
                    true,
                    gettext(locale, "Filter operator"),
                );
                info.current_text_value = selected.clone();
                info
            });
            if !matches!(row.op, FilterOp::IsNull | FilterOp::IsNotNull) {
                let hint = widgets::galley(ui, &gettext(locale, "Value"), palette.dim, &look);
                let field = ui.add(
                    crate::ui::widgets::single(ui, &mut row.value, &look)
                        .hint_text(hint)
                        .desired_width(220.0),
                );
                if focus && Some(index) == value_row {
                    field.request_focus();
                }
                if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    apply = true;
                }
            }
            if icon_button(
                ui,
                Icon::X,
                &gettext(locale, "Remove condition"),
                &look,
                &palette,
            )
            .clicked()
            {
                actions.push(Action::RemoveFilterRow {
                    tab,
                    object_tab,
                    index,
                });
            }
        });
    }
    // Wraps on a narrow window instead of pushing Apply off the edge.
    ui.horizontal_wrapped(|ui| {
        let add = widgets::button(ui, &gettext(locale, "+ Condition"), &look);
        if focus && value_row.is_none() && !bar.raw {
            add.request_focus();
        }
        if add.clicked() {
            actions.push(Action::AddFilterRow { tab, object_tab });
        }
        crate::ui::widgets::checkbox(
            ui,
            &mut bar.raw,
            &gettext(locale, "Raw WHERE"),
            &look,
            &palette,
        );
        if bar.raw {
            let hint = crate::typography::Text::one(
                &look,
                widgets::code(&look),
                "id > 10 AND name LIKE 'A%'",
                palette.dim,
            )
            .layout(ui.ctx());
            let field = ui.add(
                crate::ui::widgets::single_in(ui, &mut bar.raw_text, &look, widgets::code(&look))
                    .hint_text(hint.galley)
                    .desired_width(320.0),
            );
            if focus && value_row.is_none() {
                field.request_focus();
            }
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                apply = true;
            }
        }
        if crate::ui::widgets::primary_button(ui, &gettext(locale, "Apply"), &look, &palette)
            .clicked()
        {
            apply = true;
        }
        if widgets::button(ui, &gettext(locale, "Clear"), &look).clicked() {
            actions.push(Action::ClearFilters { tab, object_tab });
        }
    });
    if apply {
        actions.push(Action::ApplyFilters { tab, object_tab });
    }
    app.actions.extend(actions);
}
