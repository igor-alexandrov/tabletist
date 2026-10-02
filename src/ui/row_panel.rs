//! The row panel: every field of the selected row, in full.

use egui::{
    CornerRadius, Frame, Id, Rect, Sense, Stroke, StrokeKind, TextEdit, WidgetInfo, WidgetType,
    pos2, vec2,
};
use std::sync::Arc;

use tabletist_db::{Value, ValueKind};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, CellPos, ConnTabId, RowFields, Tab, TabId, Workspace};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::format;
use crate::ui::json_view;
use crate::ui::widgets;

/// The panel's width when it opens: macOS 344 and its 1 pt rule, terminal
/// 460 and its 2 pt accent edge (the design draws both outside the width).
fn width(look: &Look) -> f32 {
    if look.terminal { 462.0 } else { 345.0 }
}

/// The least width the panel's edge can be dragged to, with room to spare.
const NARROWEST: f32 = 260.0;
/// The most it can be dragged to.
const WIDEST: f32 = 560.0;

/// The widths the panel may have when it shares `room` points with the
/// grid or the editor beside it: never more than half, so what it sits
/// beside keeps at least as much. In a narrow window that goes under the
/// panel's least width, and the panel gives way too.
fn width_range(room: f32) -> egui::Rangef {
    let most = (room / 2.0).clamp(0.0, WIDEST);
    egui::Rangef::new(NARROWEST.min(most), most)
}

/// The width egui last drew the panel `id` at, its edge included. egui
/// does not update it while the edge is being dragged: it is the width
/// from before the drag until the pointer lets go.
fn drawn_width(ui: &egui::Ui, id: Id) -> Option<f32> {
    egui::PanelState::load(ui.ctx(), id).map(|state| state.outer_rect.width())
}

/// The panel's edge: a 1 pt rule, the terminal's 2 pt accent.
fn edge(look: &Look) -> f32 {
    if look.terminal { 2.0 } else { 1.0 }
}

/// Space at the panel's sides: macOS 16, terminal 14.
fn side(look: &Look) -> f32 {
    if look.terminal { 14.0 } else { 16.0 }
}

/// The height of one line in `role`.
fn line_of(ui: &egui::Ui, role: TextRole, look: &Look) -> f32 {
    role.row_height(ui.ctx(), look.faces)
}

/// A colour value's swatch.
const SWATCH: f32 = 14.0;

/// Field labels and notes: 11.5 in both looks.
fn caption(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::FieldLabel, TextRole::OCaption)
}

/// What the row panel knows about a column besides its name and type.
struct FieldInfo {
    key: bool,
    /// The table a single-column foreign key points at.
    target: Option<tabletist_db::ObjectRef>,
    /// The column it points at there.
    target_column: String,
}

/// A column's label: `id · int8 · primary key`. The terminal's half-width
/// cells name a timestamp alone, as the design does.
/// Names and types come from the server, so nothing hidden in them shows.
fn label(name: &str, type_name: &str, kind: ValueKind, info: &FieldInfo, look: &Look) -> String {
    let name = format::display_safe(name).into_owned();
    if look.terminal && kind == ValueKind::Temporal {
        return name;
    }
    let mut parts = vec![name];
    let type_name = match (kind, type_name) {
        (ValueKind::Temporal, "timestamp") => "timestamp · no tz".to_owned(),
        (ValueKind::Temporal, "timestamptz") => "timestamp · tz".to_owned(),
        _ => format::display_safe(type_name).into_owned(),
    };
    parts.push(type_name);
    if info.key {
        parts.push(if look.terminal {
            "pk".into()
        } else {
            "primary key".into()
        });
    }
    let mut text = parts.join(" · ");
    if let (Some(target), true) = (&info.target, look.terminal) {
        text.push_str(&format!(" → {}", format::display_safe(&target.name)));
    }
    text
}

/// What the panel shows a row of: a table's page, or a SQL editor's result.
struct Source<'a> {
    /// The table's name, or "Query 3": under the title in the macOS header.
    name: String,
    columns: &'a [tabletist_db::ColumnMeta],
    rows: &'a [Vec<Value>],
    selection: Option<CellPos>,
    /// The table's structure, once described. A result has none: no key
    /// names its rows, no foreign key links them, and only its booleans
    /// are tags.
    structure: Option<&'a tabletist_db::Structure>,
    /// The rows before the first one here: a table's page offset.
    offset: u64,
    /// The selected row's text, formatted by the app.
    texts: Option<&'a RowFields>,
    /// Whether the row is a table's. A result's row has no editing
    /// controls under it, and no `y` key to copy from it.
    table: bool,
}

/// The source the tab `id` gives the panel: none for a closed tab, or a
/// SQL editor that shows no rows.
fn source(workspace: &Workspace, id: TabId, locale: crate::i18n::Locale) -> Option<Source<'_>> {
    match workspace.tab(id)? {
        Tab::Object(object) => {
            let (columns, rows) = match object.page() {
                Some(page) => (page.columns.as_slice(), page.rows.as_slice()),
                None => (&[][..], &[][..]),
            };
            Some(Source {
                name: format::object_title(
                    &object.object,
                    workspace.name_is_shared(&object.object),
                ),
                columns,
                rows,
                selection: object.selection,
                structure: object.structure.value.as_ref(),
                offset: object.query.offset,
                texts: object.selected_fields(),
                table: true,
            })
        }
        Tab::Sql(sql) => {
            let (columns, rows, _) = sql.shown_rows()?;
            Some(Source {
                name: format!("{} {}", gettext(locale, "Query"), sql.number),
                columns,
                rows,
                // The cell only while its row shows: under the Messages
                // pane there is no row for the panel to be about.
                selection: sql.selected_row().and(sql.selection),
                structure: None,
                offset: 0,
                texts: sql.selected_fields(),
                table: false,
            })
        }
    }
}

/// Draws the panel for the tab `id`: an object tab, or a SQL editor.
pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    // `za` asked to fold the documents.
    let fold = app.workspace_mut(tab).is_some_and(|workspace| {
        let asked = workspace.fold_documents == Some(id);
        if asked {
            workspace.fold_documents = None;
        }
        asked
    });
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let Some(source) = source(workspace, id, locale) else {
        return;
    };
    let mut actions = Vec::new();
    let panel = Id::new(("row-panel", tab.0));
    let range = width_range(ui.available_width());
    // egui remembers the width it last drew, which a small window cuts
    // down. The width the user gave the panel (or its default) is kept
    // beside it, to come back to when there is room again.
    let drawn = drawn_width(ui, panel);
    let wanted: f32 = ui.data_mut(|data| {
        *data.get_persisted_mut_or_insert_with(panel.with("wanted"), || {
            drawn.unwrap_or(width(&look))
        })
    });
    let target = wanted.clamp(range.min, range.max);
    // Only a drag makes egui's panel wider: with room again, its least
    // width is the target until it is drawn that wide.
    let least = if drawn.is_some_and(|drawn| drawn + 1.0 < target) {
        target
    } else {
        range.min
    };
    egui::Panel::right(panel)
        // With no width to choose from, the edge is no handle.
        .resizable(range.min < range.max)
        .default_size(target)
        .size_range(least..=range.max)
        .show_separator_line(false)
        // The edge sits outside the content, as the design's border does.
        .frame(
            Frame::new()
                .fill(palette.window)
                .inner_margin(egui::Margin {
                    left: edge(&look) as i8,
                    ..Default::default()
                }),
        )
        .show(ui, |ui| {
            let full = ui.max_rect();
            // The terminal marks the inspector's edge in the accent, as
            // Omarchy marks a focused window.
            if look.terminal {
                ui.painter().rect_filled(
                    Rect::from_min_size(
                        full.min - vec2(edge(&look), 0.0),
                        vec2(edge(&look), full.height()),
                    ),
                    CornerRadius::ZERO,
                    palette.accent,
                );
            } else {
                widgets::vline(ui, full.left() - 0.5, full.y_range(), palette.outline);
            }
            let selected = source
                .selection
                .and_then(|cell| source.rows.get(cell.row).map(|row| (cell, row)));
            // Formatted by the app when the selection changed, never here.
            let texts = source.texts;
            let Some((cell, row)) = selected else {
                ui.centered_and_justified(|ui| {
                    Text::one(
                        &look,
                        widgets::body(&look),
                        &gettext(locale, "Select a row to see its fields"),
                        palette.secondary,
                    )
                    .layout(ui.ctx())
                    .label(ui);
                });
                return;
            };
            let structure = source.structure;
            let info = |name: &str| {
                let key =
                    structure.is_some_and(|s| s.primary_key.iter().any(|column| column == name));
                let foreign = structure.and_then(|s| {
                    s.foreign_keys
                        .iter()
                        .find(|foreign| foreign.columns.len() == 1 && foreign.columns[0] == name)
                });
                FieldInfo {
                    key,
                    target: foreign.map(|f| {
                        tabletist_db::ObjectRef::new(f.ref_schema.clone(), f.ref_table.clone())
                    }),
                    target_column: foreign
                        .and_then(|f| f.ref_columns.first().cloned())
                        .unwrap_or_default(),
                }
            };
            // Values the grid shows as tags keep their colour here.
            let tags: Vec<_> = source
                .columns
                .iter()
                .map(|column| crate::ui::value_tags::Tags::of(column, structure))
                .collect();
            let tag_of = |col: usize, value: &Value| tags[col].style(value);
            // The row's name: its key, else its number. Both as the grid
            // shows them: short, and nothing hidden.
            let key_column = structure
                .and_then(|s| (s.primary_key.len() == 1).then(|| s.primary_key[0].as_str()));
            let key_value = key_column.and_then(|key| {
                source
                    .columns
                    .iter()
                    .position(|column| column.name == key)
                    .map(|col| format::cell_text(&row[col]).into_owned())
            });
            let key_column = key_column.map(|key| format::display_safe(key).into_owned());
            let number = source.offset + cell.row as u64 + 1;
            let side = side(&look);
            // Header: 52 (macOS) or 40 (terminal), and its rule.
            let header_height = if look.terminal { 41.0 } else { 53.0 };
            let (header, _) =
                ui.allocate_exact_size(vec2(full.width(), header_height), Sense::hover());
            let divider = if look.terminal {
                palette.outline
            } else {
                palette.surface_hover
            };
            widgets::hline(ui, header.x_range(), header.bottom() - 0.5, divider);
            let rows = source.rows.len();
            let can_prev = cell.row > 0;
            let can_next = cell.row + 1 < rows;
            if look.terminal {
                let y = header.top() + 20.0;
                let role = TextRole::OBody;
                let mut x = header.left() + side;
                x += widgets::paint_text(
                    ui,
                    x,
                    y,
                    Text::one(
                        &look,
                        TextRole::OGroup,
                        &gettext(locale, "row"),
                        palette.text,
                    ),
                ) + 10.0;
                // esc ×: 8 in from the right, 24 tall, 8 at its sides, 6
                // before the ×; the prev/next hint 10 before it.
                let small = TextRole::OSecondary;
                let esc_width =
                    8.0 + small.width(ui.ctx(), look.faces, "esc") + 6.0 + 10.0 + 8.0 + 2.0;
                let esc = Rect::from_min_size(
                    pos2(header.right() - 8.0 - esc_width, y - 12.0),
                    vec2(esc_width, 24.0),
                );
                // The row's name: its key and value, or its number.
                let keyed = key_column.as_ref().zip(key_value.as_ref());
                let whole = match keyed {
                    Some((key, value)) => format!("{key} {value}"),
                    None => number.to_string(),
                };
                let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
                // The hint gives way before the name does: its keys alone
                // when the whole name does not fit before its words. The
                // name is cut where the hint begins, 10 before it.
                let mut hint = [("[ ]", "prev/next", true)];
                let mut width = widgets::key_hints_width(ui, &hint, 0.0, &look, &palette);
                let mut room = esc.left() - 10.0 - width - 10.0 - x;
                if measure(&whole) > room {
                    hint = [("[ ]", "", true)];
                    width = widgets::key_hints_width(ui, &hint, 0.0, &look, &palette);
                    room = esc.left() - 10.0 - width - 10.0 - x;
                }
                // A number keeps its last digits.
                let shown = crate::ui::grid::ellipsize(&whole, room, keyed.is_none(), measure);
                let name = match keyed {
                    Some((key, _)) => {
                        let value = shown.strip_prefix(key.as_str());
                        match value.and_then(|value| value.strip_prefix(' ')) {
                            Some(value) => Text::one(&look, role, key, palette.dim)
                                .space(role, " ")
                                .add(role, value, palette.warning),
                            // Cut within the key.
                            None => Text::one(&look, role, &shown, palette.dim),
                        }
                    }
                    None => Text::one(&look, role, &shown, palette.warning),
                };
                widgets::paint_text(ui, x, y, name);
                let response = ui.interact(esc, ui.id().with("close"), Sense::click());
                let close = gettext(locale, "Close the row panel");
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &close));
                ui.painter().rect_stroke(
                    esc,
                    CornerRadius::same(3),
                    Stroke::new(1.0, palette.outline),
                    StrokeKind::Inside,
                );
                widgets::paint_text(
                    ui,
                    esc.left() + 9.0,
                    y,
                    Text::one(&look, small, "esc", palette.dim),
                );
                Icon::X.image(palette.dim, 10.0).paint_at(
                    ui,
                    Rect::from_center_size(pos2(esc.right() - 9.0 - 5.0, y), vec2(10.0, 10.0)),
                );
                if response.clicked() {
                    actions.push(Action::ToggleRowPanel(tab));
                }
                widgets::key_hints(
                    ui,
                    (esc.left() - 10.0 - width, y),
                    &hint,
                    0.0,
                    &look,
                    &palette,
                );
                for (enabled, step, label) in
                    [(can_prev, -1, "Previous row"), (can_next, 1, "Next row")]
                {
                    let hit = Rect::from_min_size(
                        pos2(
                            esc.left() - 10.0 - width + if step < 0 { 0.0 } else { 12.0 },
                            y - 9.0,
                        ),
                        vec2(12.0, 18.0),
                    );
                    let response = ui.interact(hit, ui.id().with(("step", step)), Sense::click());
                    response.widget_info(|| {
                        WidgetInfo::labeled(WidgetType::Button, enabled, gettext(locale, label))
                    });
                    if response.clicked() && enabled {
                        actions.push(Action::MoveSelection {
                            tab,
                            id,
                            rows: step,
                            cols: 0,
                        });
                    }
                }
            } else {
                let title = match (&key_column, &key_value) {
                    (Some(key), Some(value)) => {
                        format!("{} · {key} {value}", gettext(locale, "Row"))
                    }
                    _ => format!("{} {number}", gettext(locale, "Row")),
                };
                // The title over the table's name, centred in the 52.
                let (title_role, sub_role) = (TextRole::UiBodySemibold, TextRole::FieldLabel);
                let (title_line, sub_line) =
                    (line_of(ui, title_role, &look), line_of(ui, sub_role, &look));
                let top = header.top() + (52.0 - title_line - sub_line) / 2.0;
                widgets::paint_label(
                    ui,
                    header.left() + side,
                    top + title_line / 2.0,
                    Text::one(&look, title_role, &title, palette.text),
                );
                widgets::paint_text(
                    ui,
                    header.left() + side,
                    top + title_line + sub_line / 2.0,
                    Text::one(&look, sub_role, &source.name, palette.dim),
                );
                // Three 30 pt buttons, 4 apart, 8 in from the right.
                let y = header.top() + 26.0;
                let mut right = header.right() - 8.0;
                for (icon, label, enabled, action) in [
                    (
                        Icon::X,
                        "Close the row panel",
                        true,
                        Action::ToggleRowPanel(tab),
                    ),
                    (
                        Icon::ChevronDown,
                        "Next row",
                        can_next,
                        Action::MoveSelection {
                            tab,
                            id,
                            rows: 1,
                            cols: 0,
                        },
                    ),
                    (
                        Icon::ChevronUp,
                        "Previous row",
                        can_prev,
                        Action::MoveSelection {
                            tab,
                            id,
                            rows: -1,
                            cols: 0,
                        },
                    ),
                ] {
                    let place = Rect::from_min_size(pos2(right - 30.0, y - 15.0), vec2(30.0, 30.0));
                    right -= 34.0;
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
                    child.add_enabled_ui(enabled, |ui| {
                        if widgets::icon_button_sized(
                            ui,
                            icon,
                            &gettext(locale, label),
                            vec2(30.0, 30.0),
                            14.0,
                            &look,
                            &palette,
                        )
                        .clicked()
                        {
                            actions.push(action);
                        }
                    });
                }
            }
            // Footer: the editing controls, disabled until editing arrives.
            // Its rule, the buttons (28 or 32), the note under them. A SQL
            // result has none: its rows are no table's to edit.
            let note = line_of(ui, caption(&look), &look);
            let footer_height = if !source.table {
                0.0
            } else if look.terminal {
                1.0 + 10.0 + 28.0 + 8.0 + note + 10.0
            } else {
                1.0 + 12.0 + 32.0 + 8.0 + note + 12.0
            };
            let body = Rect::from_min_max(
                pos2(full.left(), header.bottom()),
                pos2(full.right(), full.bottom() - footer_height),
            );
            let foot = Rect::from_min_max(pos2(full.left(), body.bottom()), full.max);
            if source.table {
                editing_footer(ui, foot, &look, &palette, locale);
            }
            let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
            let skin = FieldSkin {
                look: &look,
                palette: &palette,
                locale,
                fold,
                texts,
                copy_key: source.table,
            };
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(&mut body_ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    let fields: Vec<(usize, &tabletist_db::ColumnMeta, &Value)> = source
                        .columns
                        .iter()
                        .zip(row.iter())
                        .enumerate()
                        .map(|(col, (column, value))| (col, column, value))
                        .collect();
                    let ctx = ui.ctx().clone();
                    let is_doc = |column: &tabletist_db::ColumnMeta, value: &Value| {
                        column.kind == ValueKind::Json && !value.is_null()
                            || json_doc(&ctx, value, column.kind).is_some()
                    };
                    if look.terminal {
                        // Short fields two to a line, documents after them.
                        // Two to a line; a foreign key takes a line of its
                        // own, for its value and its link.
                        let short: Vec<_> =
                            fields.iter().filter(|(_, c, v)| !is_doc(c, v)).collect();
                        let mut lines: Vec<Vec<&(usize, &tabletist_db::ColumnMeta, &Value)>> =
                            Vec::new();
                        // Short fields fill a line two at a time; a foreign
                        // key waits for the line in progress, then takes one
                        // of its own.
                        let mut pair = Vec::new();
                        let mut waiting = Vec::new();
                        for entry in short {
                            if info(&entry.1.name).target.is_some() {
                                if pair.is_empty() {
                                    lines.push(vec![entry]);
                                } else {
                                    waiting.push(entry);
                                }
                                continue;
                            }
                            pair.push(entry);
                            if pair.len() == 2 {
                                lines.push(std::mem::take(&mut pair));
                                lines.extend(waiting.drain(..).map(|wide| vec![wide]));
                            }
                        }
                        if !pair.is_empty() {
                            lines.push(pair);
                        }
                        lines.extend(waiting.into_iter().map(|wide| vec![wide]));
                        lines.retain(|line| !line.is_empty());
                        // Two columns, each cell 8 above and below, 14 at its
                        // sides; 6 above the first line.
                        ui.add_space(6.0);
                        let column_width = ui.available_width() / 2.0;
                        for pair in &lines {
                            ui.add_space(8.0);
                            ui.horizontal_top(|ui| {
                                let wide =
                                    pair.len() == 1 && info(&pair[0].1.name).target.is_some();
                                // A sliver of a panel has no room for them.
                                let half = if wide {
                                    2.0 * column_width - 2.0 * side
                                } else {
                                    column_width - 2.0 * side
                                }
                                .max(0.0);
                                ui.add_space(side);
                                for (index, (col, column, value)) in
                                    pair.iter().copied().enumerate()
                                {
                                    // 14 after one cell and 14 before the next.
                                    if index > 0 {
                                        ui.add_space(2.0 * side);
                                    }
                                    ui.allocate_ui_with_layout(
                                        vec2(half, 0.0),
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.set_width(half);
                                            let info = info(&column.name);
                                            field(
                                                ui,
                                                tab,
                                                id,
                                                cell.row,
                                                *col,
                                                column,
                                                value,
                                                &info,
                                                tag_of(*col, value),
                                                skin,
                                                &mut actions,
                                            );
                                        },
                                    );
                                }
                            });
                            ui.add_space(8.0);
                        }
                        ui.add_space(6.0);
                        for (col, column, value) in fields.iter().filter(|(_, c, v)| is_doc(c, v)) {
                            // Documents on the window tone, under a rule.
                            let top = ui.cursor().top();
                            let area = Rect::from_min_max(
                                pos2(ui.max_rect().left(), top),
                                ui.max_rect().max,
                            );
                            ui.painter()
                                .rect_filled(area, CornerRadius::ZERO, palette.window);
                            widgets::hline(ui, ui.max_rect().x_range(), top + 0.5, palette.outline);
                            ui.add_space(12.0);
                            ui.horizontal_top(|ui| {
                                ui.add_space(side);
                                ui.vertical(|ui| {
                                    ui.set_width((ui.available_width() - side).max(0.0));
                                    let info = info(&column.name);
                                    field(
                                        ui,
                                        tab,
                                        id,
                                        cell.row,
                                        *col,
                                        column,
                                        value,
                                        &info,
                                        tag_of(*col, value),
                                        skin,
                                        &mut actions,
                                    );
                                });
                            });
                            ui.add_space(12.0);
                        }
                    } else {
                        for (col, column, value) in &fields {
                            // Each field: 12 above and below, its rule under
                            // it; documents on the panel tone.
                            let top = ui.cursor().top();
                            let document = is_doc(column, value);
                            let backdrop = ui.painter().add(egui::Shape::Noop);
                            ui.add_space(12.0);
                            ui.horizontal_top(|ui| {
                                ui.add_space(side);
                                ui.vertical(|ui| {
                                    ui.set_width((ui.available_width() - side).max(0.0));
                                    let info = info(&column.name);
                                    field(
                                        ui,
                                        tab,
                                        id,
                                        cell.row,
                                        *col,
                                        column,
                                        value,
                                        &info,
                                        tag_of(*col, value),
                                        skin,
                                        &mut actions,
                                    );
                                });
                            });
                            ui.add_space(12.0);
                            let y = ui.cursor().top();
                            if document {
                                ui.painter().set(
                                    backdrop,
                                    egui::epaint::RectShape::filled(
                                        Rect::from_min_max(
                                            pos2(ui.max_rect().left(), top),
                                            pos2(ui.max_rect().right(), y),
                                        ),
                                        CornerRadius::ZERO,
                                        palette.panel,
                                    ),
                                );
                            }
                            ui.add_space(1.0);
                            widgets::hline(ui, ui.max_rect().x_range(), y + 0.5, palette.surface);
                        }
                    }
                });
        });
    // The edge was dragged: that is the width wanted from now on. egui
    // stores a width only when the drag is over, so a width that did not
    // change this frame is not a drag's (a window resized mid-drag).
    if let Some(after) = drawn_width(ui, panel)
        && drawn.is_some_and(|before| before != after)
        && (after - target).abs() > 1.0
    {
        ui.data_mut(|data| data.insert_persisted(panel.with("wanted"), after));
    }
    app.actions.extend(actions);
}

/// How fields draw.
#[derive(Clone, Copy)]
struct FieldSkin<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: crate::i18n::Locale,
    /// Fold or unfold documents this frame (`za`).
    fold: bool,
    /// The selected row's text, formatted by the app.
    texts: Option<&'a crate::model::RowFields>,
    /// Whether `y` copies the selected cell: in a table, not in a result.
    copy_key: bool,
}

/// One field: its label (with a copy button, or a document's controls),
/// then its value.
#[allow(clippy::too_many_arguments)] // one call site per layout
fn field(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    tab_id: TabId,
    row: usize,
    col: usize,
    column: &tabletist_db::ColumnMeta,
    value: &Value,
    info: &FieldInfo,
    tag: Option<crate::ui::grid::Style>,
    skin: FieldSkin<'_>,
    actions: &mut Vec<Action>,
) {
    let FieldSkin {
        look,
        palette,
        locale,
        fold,
        texts,
        copy_key,
    } = skin;
    // The request whose answer holds the row: the same row of another
    // page or another result keeps no folds and nothing expanded.
    let request = texts.and_then(|texts| texts.request);
    let label_role = caption(look);
    let text = label(&column.name, &column.type_name, column.kind, info, look);
    let doc = json_doc(ui.ctx(), value, column.kind);
    let width = ui.available_width();
    // A document's label line holds its 26 pt copy button (macOS); a
    // plain label is one line of its text.
    let label_height = if doc.is_some() && !look.terminal {
        26.0
    } else {
        line_of(ui, label_role, look)
    };
    let (line, response) = ui.allocate_exact_size(vec2(width, label_height), Sense::hover());
    let name_id = response.id;
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
    // Cut at the column's edge, as the field's own width allows.
    let room = line.width() - 30.0;
    let shown = crate::ui::grid::ellipsize(&text, room, false, |text| {
        label_role.width(ui.ctx(), look.faces, text)
    });
    widgets::paint_text(
        ui,
        line.left(),
        line.center().y,
        Text::one(look, label_role, &shown, palette.dim),
    );
    let column_name = format::display_safe(&column.name);
    let copy_label = format!("{} {column_name}", gettext(locale, "Copy"));
    let hovered = ui.rect_contains_pointer(line.expand2(vec2(16.0, 30.0)));
    if let Some(doc) = &doc {
        // A document's own controls: fold everything, and copy.
        let copy = Rect::from_min_size(
            pos2(line.right() - 26.0, line.center().y - 13.0),
            vec2(26.0, 26.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(copy));
        if look.terminal {
            // "za fold · y copy", keys in the text colour, at the right. A
            // SQL result has no `y`: the hint offers only the fold.
            let role = TextRole::OCaption;
            let hints = Text::new(look)
                .add(role, "za", palette.text)
                .space(role, " ");
            let hints = if copy_key {
                hints
                    .add(role, "fold ·", palette.dim)
                    .space(role, " ")
                    .add(role, "y", palette.text)
                    .space(role, " ")
                    .add(role, "copy", palette.dim)
            } else {
                hints.add(role, "fold", palette.dim)
            };
            widgets::paint_text_right(ui, line.right(), line.center().y, hints);
            if copy_button(&mut child, &copy_label, false, look, palette).clicked() {
                ui.ctx().copy_text(format::plain_text(value));
            }
        } else {
            if copy_button(&mut child, &copy_label, true, look, palette).clicked() {
                ui.ctx().copy_text(format::plain_text(value));
            }
            let id = Id::new(("row-panel-json", tab, tab_id, request, row, col));
            // "Collapse all": a 24 pt button 6 at its sides, 4 before copy.
            let link_right = copy.left() - 4.0 - 6.0;
            json_view::fold_all_link(
                ui,
                id,
                doc,
                link_right,
                line.center().y,
                locale,
                look,
                palette,
            );
        }
    } else {
        let copy = Rect::from_min_size(
            pos2(line.right() - 22.0, line.center().y - 11.0),
            vec2(22.0, 22.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(copy));
        if copy_button(&mut child, &copy_label, hovered, look, palette).clicked() {
            ui.ctx().copy_text(format::plain_text(value));
        }
    }
    ui.add_space(if doc.is_some() {
        if look.terminal { 8.0 } else { 6.0 }
    } else if look.terminal {
        2.0
    } else {
        3.0
    });
    if value.is_null() {
        Text::one(
            look,
            crate::ui::grid::data_role(look),
            "NULL",
            palette.faint,
        )
        .layout(ui.ctx())
        .label(ui);
        return;
    }
    if let Some(doc) = doc {
        if fold {
            json_view::toggle_fold_all(
                ui,
                Id::new(("row-panel-json", tab, tab_id, request, row, col)),
            );
        }
        if let Some(file) = json_view::attachment(&doc) {
            attachment_card(ui, &file, look, palette);
            ui.add_space(if look.terminal { 10.0 } else { 6.0 });
        }
        let id = Id::new(("row-panel-json", tab, tab_id, request, row, col));
        if look.terminal {
            json_view::show(ui, id, &doc, &column_name, locale, palette, look);
        } else {
            Frame::new()
                .fill(palette.window)
                // The design's 1 pt border outside 12 and 10 of padding
                // (egui counts the stroke into the margin).
                .stroke(Stroke::new(1.0, palette.surface_hover))
                .corner_radius(CornerRadius::same(look.radius))
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    json_view::show(ui, id, &doc, &column_name, locale, palette, look);
                });
        }
        return;
    }
    let Some(formatted) = texts.and_then(|texts| texts.fields.get(col)) else {
        return;
    };
    let expanded_id = Id::new(("row-panel-expanded", tab, tab_id, request, row, col));
    let expanded: bool = ui.data(|data| data.get_temp(expanded_id)).unwrap_or(false);
    let long = formatted.full.is_some();
    let shown = match &formatted.full {
        Some(full) if expanded => full,
        _ => &formatted.short,
    };
    // Values in the data face at 13; the terminal's timestamps at 12.
    let role = if matches!(column.kind, ValueKind::Json | ValueKind::Binary) {
        TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary)
    } else if look.terminal && column.kind == ValueKind::Temporal {
        TextRole::OSecondary
    } else {
        TextRole::pick(look, TextRole::InspectorValue, TextRole::OBody)
    };
    // The follow link sits at the value's right.
    let follow = info.target.as_ref().map(|target| {
        if look.terminal {
            ("gd open".to_owned(), target.clone())
        } else {
            (
                format!(
                    "{} {} →",
                    gettext(locale, "Open"),
                    singular(&format::display_safe(&target.name))
                ),
                target.clone(),
            )
        }
    });
    // A tag's value in its text colour: the grid's chips stay in the grid.
    let color = tag.map_or(palette.text, |style| {
        crate::ui::value_tags::style_colors(style, look, palette).0
    });
    let small = widgets::secondary(look);
    ui.horizontal_top(|ui| {
        // A colour (`#3a7bd5`) leads with a swatch of it, on the first line.
        if let Some(swatch) = format::color(value) {
            let (place, _) =
                ui.allocate_exact_size(vec2(SWATCH, line_of(ui, role, look)), Sense::hover());
            crate::ui::grid::paint_swatch(
                ui.painter(),
                ui,
                place.center(),
                SWATCH,
                swatch,
                look,
                palette,
            );
            ui.add_space(8.0);
        }
        let link = follow.as_ref().map_or(0.0, |(text, _)| {
            small.width(ui.ctx(), look.faces, text) + 12.0
        });
        let room = (ui.available_width() - link).max(24.0);
        ui.allocate_ui(vec2(room, 0.0), |ui| {
            let mut layouter = crate::typography::layouter(look, role, color);
            ui.add(
                TextEdit::multiline(&mut shown.as_str())
                    .font(role.font_id(look.faces))
                    // Flush with the field name above.
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO)
                    .desired_width(room)
                    .desired_rows(1)
                    .layouter(&mut layouter),
            )
            .labelled_by(name_id);
        });
        if let Some((text, target)) = follow {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                let link = Text::one(look, small, &text, palette.accent)
                    .layout(ui.ctx())
                    .label_sense(ui, Sense::click());
                let name = format!(
                    "{} {}",
                    gettext(locale, "Open"),
                    format::display_safe(&target.name)
                );
                link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &name));
                if link.clicked() {
                    actions.push(Action::FollowForeignKey {
                        tab,
                        object: target.clone(),
                        column: info.target_column.clone(),
                        value: format::plain_text(value),
                    });
                }
            });
        }
    });
    if long {
        let label = if expanded {
            gettext(locale, "Show less").into_owned()
        } else {
            format!("{} ({})", gettext(locale, "Show all"), formatted.size)
        };
        let link = Text::one(look, small, &label, palette.accent)
            .layout(ui.ctx())
            .label_sense(ui, Sense::click());
        link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &label));
        if link.clicked() {
            ui.data_mut(|data| data.insert_temp(expanded_id, !expanded));
        }
    }
}

/// `books` becomes `book`, for "Open book →".
fn singular(table: &str) -> &str {
    table
        .strip_suffix("ies")
        .map(|_| table)
        .or_else(|| table.strip_suffix('s'))
        .unwrap_or(table)
}

/// A field's copy button: bordered next to a document, otherwise a bare
/// icon shown when the pointer is near. It is always there for keyboards
/// and screen readers.
fn copy_button(
    ui: &mut egui::Ui,
    label: &str,
    shown: bool,
    look: &Look,
    palette: &Palette,
) -> egui::Response {
    let size = ui.max_rect().size();
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    if shown || response.hovered() || response.has_focus() {
        let corner = CornerRadius::same(look.radius.saturating_sub(2));
        if response.hovered() {
            ui.painter().rect_filled(rect, corner, palette.surface);
        }
        if size.x >= 24.0 && !look.terminal {
            ui.painter().rect_stroke(
                rect,
                corner,
                Stroke::new(widgets::hairline(ui), palette.border),
                StrokeKind::Inside,
            );
        }
        Icon::Copy
            .image(palette.secondary, 13.0)
            .paint_at(ui, Rect::from_center_size(rect.center(), vec2(13.0, 13.0)));
    }
    response.on_hover_text(label)
}

/// A stored file's card: an image tile, its name, and what is known of it.
fn attachment_card(
    ui: &mut egui::Ui,
    file: &json_view::Attachment,
    look: &Look,
    palette: &Palette,
) {
    let width = ui.available_width();
    if look.terminal {
        // A bordered line: a 14 pt icon, the name, its size; 6 and 8 in.
        let name_role = TextRole::OBody;
        let height = line_of(ui, name_role, look) + 12.0 + 2.0;
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0, palette.outline),
            StrokeKind::Inside,
        );
        let y = rect.center().y;
        Icon::Image.image(palette.dim, 14.0).paint_at(
            ui,
            Rect::from_center_size(pos2(rect.left() + 9.0 + 7.0, y), vec2(14.0, 14.0)),
        );
        let x = rect.left() + 9.0 + 14.0 + 8.0;
        let x =
            x + widgets::paint_text(
                ui,
                x,
                y,
                Text::one(look, name_role, &file.filename, palette.text),
            ) + 8.0;
        let details: Vec<&str> = file
            .details
            .split(" · ")
            .filter(|part| part.contains('×') || part.ends_with('B'))
            .collect();
        widgets::paint_text(
            ui,
            x,
            y,
            Text::one(
                look,
                TextRole::OSecondary,
                &details.join(" · ").replace(" × ", "×"),
                palette.dim,
            ),
        );
        return;
    }
    // macOS: 8 in, a 44 pt tile, 10, the name over its details (2 apart).
    let (rect, _) = ui.allocate_exact_size(vec2(width, 62.0), Sense::hover());
    let corner = CornerRadius::same(look.radius);
    ui.painter().rect_filled(rect, corner, palette.window);
    ui.painter().rect_stroke(
        rect,
        corner,
        Stroke::new(widgets::hairline(ui), palette.surface_hover),
        StrokeKind::Inside,
    );
    let tile = Rect::from_min_size(pos2(rect.left() + 9.0, rect.top() + 9.0), vec2(44.0, 44.0));
    ui.painter().rect_filled(
        tile,
        CornerRadius::same(6),
        palette.surface.lerp_to_gamma(palette.surface_hover, 0.3),
    );
    let icon = if file.image { Icon::Image } else { Icon::Copy };
    icon.image(palette.faint, 18.0)
        .paint_at(ui, Rect::from_center_size(tile.center(), vec2(18.0, 18.0)));
    let x = tile.right() + 10.0;
    let (name_role, detail_role) = (TextRole::UiBodyStrong, TextRole::FieldLabel);
    let (name_line, detail_line) = (line_of(ui, name_role, look), line_of(ui, detail_role, look));
    let top = rect.center().y - (name_line + 2.0 + detail_line) / 2.0;
    widgets::paint_text(
        ui,
        x,
        top + name_line / 2.0,
        Text::one(look, name_role, &file.filename, palette.text),
    );
    widgets::paint_text(
        ui,
        x,
        top + name_line + 2.0 + detail_line / 2.0,
        Text::one(look, detail_role, &file.details, palette.dim),
    );
}

/// Edit, Duplicate and Delete, disabled, and why.
fn editing_footer(
    ui: &mut egui::Ui,
    rect: Rect,
    look: &Look,
    palette: &Palette,
    locale: crate::i18n::Locale,
) {
    let side = side(look);
    if !look.terminal {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.panel);
    }
    widgets::hline(
        ui,
        rect.x_range(),
        rect.top() + 0.5,
        if look.terminal {
            palette.outline
        } else {
            palette.surface_hover
        },
    );
    let reason = gettext(locale, "Editing arrives in a later version");
    let inner = rect.shrink2(vec2(side, 0.0));
    let gap = 6.0;
    let top = rect.top() + 1.0 + if look.terminal { 10.0 } else { 12.0 };
    let role = widgets::body(look);
    let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
    let height = if look.terminal {
        // Three equal cells, dashed, at 55%: the keys in the text colour.
        let keys = [("e", "edit"), ("yy p", "duplicate"), ("dd", "delete")];
        let width = (inner.width() - 2.0 * gap) / 3.0;
        let faded = |color: egui::Color32| palette.panel.lerp_to_gamma(color, 0.55);
        for (index, (key, label)) in keys.iter().enumerate() {
            let place = Rect::from_min_size(
                pos2(inner.left() + index as f32 * (width + gap), top),
                vec2(width, 28.0),
            );
            let response = ui.interact(place, ui.id().with(("edit", index)), Sense::hover());
            let name = format!("{key} {label}");
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, false, &name));
            let _ = response.on_hover_text(reason.as_ref());
            dashed(ui, place, faded(palette.outline));
            // A cell too narrow for both shows its key alone.
            let text = Text::one(look, role, key, faded(palette.text));
            let total = measure(&name);
            let (total, text) = if total > width {
                (measure(key), text)
            } else {
                let text = text.space(role, " ");
                (total, text.add(role, label, faded(palette.dim)))
            };
            let x = place.center().x - total / 2.0;
            widgets::paint_text(ui, x, place.center().y, text);
        }
        28.0
    } else {
        // Content-wide buttons sharing what is left equally: a 13 pt icon,
        // 6, the word.
        let labels = [
            (Icon::Pencil, gettext(locale, "Edit")),
            (Icon::Copy, gettext(locale, "Duplicate")),
            (Icon::Trash2, gettext(locale, "Delete")),
        ];
        let widths: Vec<f32> = labels
            .iter()
            .map(|(_, text)| 13.0 + 6.0 + measure(text))
            .collect();
        let extra = (inner.width() - 2.0 * gap - widths.iter().sum::<f32>()) / 3.0;
        let mut x = inner.left();
        for ((icon, text), width) in labels.iter().zip(widths) {
            let place = Rect::from_min_size(pos2(x, top), vec2(width + extra, 32.0));
            x += width + extra + gap;
            widgets::ButtonSpec::new(text)
                .icon(*icon)
                .icon_size(13.0)
                .padding(0.0)
                .disabled(&reason)
                .show_at(ui, place, look, palette);
        }
        32.0
    };
    let note_role = caption(look);
    let y = top + height + 8.0 + line_of(ui, note_role, look) / 2.0;
    if look.terminal {
        let note = gettext(
            locale,
            "read-only in 0.1.0 · editing arrives in a later version",
        );
        // Cut at the panel's side, as a field's label is.
        let shown = crate::ui::grid::ellipsize(&note, inner.width(), false, |text| {
            note_role.width(ui.ctx(), look.faces, text)
        });
        widgets::paint_text(
            ui,
            inner.left(),
            y,
            Text::one(look, note_role, &shown, palette.dim),
        );
    } else {
        Icon::Lock.image(palette.dim, 11.0).paint_at(
            ui,
            Rect::from_center_size(pos2(inner.left() + 5.5, y), vec2(11.0, 11.0)),
        );
        widgets::paint_text(
            ui,
            inner.left() + 17.0,
            y,
            Text::one(look, note_role, &reason, palette.dim),
        );
    }
}

/// A dashed 1 pt outline round `rect`.
fn dashed(ui: &egui::Ui, rect: Rect, color: egui::Color32) {
    let stroke = Stroke::new(1.0, color);
    let rect = rect.shrink(0.5);
    for side in [
        [rect.left_top(), rect.right_top()],
        [rect.right_top(), rect.right_bottom()],
        [rect.right_bottom(), rect.left_bottom()],
        [rect.left_bottom(), rect.left_top()],
    ] {
        ui.painter()
            .extend(egui::Shape::dashed_line(&side, stroke, 3.0, 3.0));
    }
}

/// A value as a tree, when it holds a document that is small enough.
fn json_doc(ctx: &egui::Context, value: &Value, kind: ValueKind) -> Option<Arc<json_view::Doc>> {
    json_view::document(ctx, kind, value, json_view::TREE_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panel_takes_at_most_half_the_room() {
        let range = |room: f32| {
            let range = width_range(room);
            (range.min, range.max)
        };
        // Room to spare: the widths the edge can be dragged to.
        assert_eq!(range(1400.0), (NARROWEST, WIDEST));
        // Half the room is the most the panel takes.
        assert_eq!(range(700.0), (NARROWEST, 350.0));
        // Under its least width the panel gives way too.
        assert_eq!(range(400.0), (200.0, 200.0));
        assert_eq!(range(0.0), (0.0, 0.0));
    }
}
