//! The row panel: every field of the selected row, in full.

use egui::{
    CornerRadius, Frame, Id, Rect, RichText, Sense, Stroke, StrokeKind, TextEdit, WidgetInfo,
    WidgetType, pos2, vec2,
};
use std::sync::Arc;

use tabletist_db::{Value, ValueKind};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, ObjectTabId};
use crate::theme::{self, Icon, Look, Palette};
use crate::ui::format;
use crate::ui::json_view;
use crate::ui::widgets::{self, icon_button};

/// The panel's width when it opens.
const WIDTH: f32 = 320.0;
/// Space at the panel's sides.
const SIDE: f32 = 16.0;

/// What the row panel knows about a column besides its name and type.
struct FieldInfo {
    key: bool,
    /// The table a single-column foreign key points at.
    target: Option<tabletist_db::ObjectRef>,
    /// The column it points at there.
    target_column: String,
}

/// A column's label: `id · int8 · primary key`.
fn label(name: &str, type_name: &str, kind: ValueKind, info: &FieldInfo, look: &Look) -> String {
    let mut parts = vec![name.to_owned()];
    let type_name = match (kind, type_name) {
        (ValueKind::Temporal, "timestamp") => "timestamp · no tz".to_owned(),
        (ValueKind::Temporal, "timestamptz") => "timestamp · tz".to_owned(),
        _ => type_name.to_owned(),
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
        text.push_str(&format!(" → {}", target.name));
    }
    text
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    // `za` asked to fold the documents.
    let fold = app.workspace_mut(tab).is_some_and(|workspace| {
        let asked = workspace.fold_documents == Some(object_tab);
        if asked {
            workspace.fold_documents = None;
        }
        asked
    });
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else {
        return;
    };
    let mut actions = Vec::new();
    egui::Panel::right(Id::new(("row-panel", tab.0)))
        .resizable(true)
        .default_size(WIDTH)
        .size_range(260.0..=560.0)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.window))
        .show(ui, |ui| {
            let full = ui.max_rect();
            // The terminal marks the inspector's edge in the accent, as
            // Omarchy marks a focused window.
            if look.terminal {
                ui.painter().rect_filled(
                    Rect::from_min_size(full.min, vec2(2.0, full.height())),
                    CornerRadius::ZERO,
                    palette.accent,
                );
            } else {
                widgets::vline(ui, full.left() + 0.5, full.y_range(), palette.outline);
            }
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
            let structure = object.structure.value.as_ref();
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
            let tag_of = |col: usize, value: &Value| {
                let values = crate::ui::data_view::tag_hues(page, col)?;
                match value {
                    Value::Text(text) => values.iter().position(|known| known == text),
                    _ => None,
                }
            };
            // The row's name: its key, else its number.
            let key_column = structure
                .and_then(|s| (s.primary_key.len() == 1).then(|| s.primary_key[0].clone()));
            let key_value = key_column.as_ref().and_then(|key| {
                page.columns
                    .iter()
                    .position(|column| &column.name == key)
                    .map(|col| format::plain_text(&row[col]))
            });
            let number = object.query.offset + cell.row as u64 + 1;
            // Header.
            let header_height = if look.terminal { 34.0 } else { 49.0 };
            let (header, _) =
                ui.allocate_exact_size(vec2(full.width(), header_height), Sense::hover());
            let divider = if look.terminal {
                palette.outline
            } else {
                palette.surface_hover
            };
            widgets::hline(ui, header.x_range(), header.bottom() - 0.5, divider);
            let rows = page.rows.len();
            let can_prev = cell.row > 0;
            let can_next = cell.row + 1 < rows;
            if look.terminal {
                let y = header.center().y;
                let mut x = header.left() + SIDE;
                x += widgets::paint_text(
                    ui,
                    x,
                    y,
                    &gettext(locale, "row"),
                    theme::semibold(theme::TEXT),
                    palette.text,
                ) + 8.0;
                match (&key_column, &key_value) {
                    (Some(key), Some(value)) => {
                        x += widgets::paint_text(
                            ui,
                            x,
                            y,
                            key,
                            theme::regular(theme::TEXT),
                            palette.dim,
                        ) + 8.0;
                        widgets::paint_text(
                            ui,
                            x,
                            y,
                            value,
                            theme::regular(theme::TEXT),
                            palette.accent,
                        );
                    }
                    _ => {
                        widgets::paint_text(
                            ui,
                            x,
                            y,
                            &number.to_string(),
                            theme::regular(theme::TEXT),
                            palette.accent,
                        );
                    }
                }
                // esc ×, then [ ] prev/next.
                let esc = Rect::from_min_size(
                    pos2(header.right() - 11.0 - 38.0, y - 10.0),
                    vec2(38.0, 20.0),
                );
                let response = ui.interact(esc, ui.id().with("close"), Sense::click());
                let close = gettext(locale, "Close the row panel");
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &close));
                ui.painter().rect_stroke(
                    esc,
                    CornerRadius::ZERO,
                    Stroke::new(1.0, palette.outline),
                    StrokeKind::Inside,
                );
                widgets::paint_text(
                    ui,
                    esc.left() + 6.0,
                    y,
                    "esc",
                    theme::regular(theme::TEXT_SMALL),
                    palette.dim,
                );
                Icon::X.image(palette.dim, 10.0).paint_at(
                    ui,
                    Rect::from_center_size(pos2(esc.right() - 9.0, y), vec2(10.0, 10.0)),
                );
                if response.clicked() {
                    actions.push(Action::ToggleRowPanel(tab));
                }
                let hint = [("[ ]", "prev/next", true)];
                let width = widgets::key_hints_width(ui, &hint, 0.0);
                widgets::key_hints(ui, esc.left() - 10.0 - width, y, &hint, 0.0, &palette);
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
                            object_tab,
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
                widgets::paint_label(
                    ui,
                    header.left() + SIDE,
                    header.top() + 18.0,
                    &title,
                    theme::semibold(theme::TEXT),
                    palette.text,
                );
                widgets::paint_text(
                    ui,
                    header.left() + SIDE,
                    header.top() + 33.0,
                    &object.object.name,
                    theme::regular(theme::TEXT_SMALL),
                    palette.dim,
                );
                let y = header.top() + 25.0;
                let mut right = header.right() - 10.0;
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
                            object_tab,
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
                            object_tab,
                            rows: -1,
                            cols: 0,
                        },
                    ),
                ] {
                    let place = Rect::from_center_size(pos2(right - 12.0, y), vec2(24.0, 24.0));
                    right -= 31.3;
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
                    child.add_enabled_ui(enabled, |ui| {
                        if icon_button(ui, icon, &gettext(locale, label), &look, &palette).clicked()
                        {
                            actions.push(action);
                        }
                    });
                }
            }
            // Footer: the editing controls, disabled until editing arrives.
            let footer_height = if look.terminal { 58.0 } else { 73.0 };
            let body = Rect::from_min_max(
                pos2(full.left(), header.bottom()),
                pos2(full.right(), full.bottom() - footer_height),
            );
            let foot = Rect::from_min_max(pos2(full.left(), body.bottom()), full.max);
            editing_footer(ui, foot, &look, &palette, locale);
            let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(&mut body_ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    let fields: Vec<(usize, &tabletist_db::ColumnMeta, &Value)> = page
                        .columns
                        .iter()
                        .zip(row.iter())
                        .enumerate()
                        .map(|(col, (column, value))| (col, column, value))
                        .collect();
                    let is_doc = |column: &tabletist_db::ColumnMeta, value: &Value| {
                        column.kind == ValueKind::Json && !value.is_null()
                    };
                    if look.terminal {
                        // Short fields two to a line, documents after them.
                        // Two to a line; a foreign key takes a line of its
                        // own, for its value and its link.
                        let short: Vec<_> =
                            fields.iter().filter(|(_, c, v)| !is_doc(c, v)).collect();
                        let mut lines: Vec<Vec<&(usize, &tabletist_db::ColumnMeta, &Value)>> =
                            Vec::new();
                        for entry in short {
                            let wide = info(&entry.1.name).target.is_some();
                            match lines.last_mut() {
                                Some(line)
                                    if !wide
                                        && line.len() == 1
                                        && info(&line[0].1.name).target.is_none() =>
                                {
                                    line.push(entry)
                                }
                                _ => lines.push(vec![entry]),
                            }
                            if wide {
                                lines.push(Vec::new());
                            }
                        }
                        lines.retain(|line| !line.is_empty());
                        ui.add_space(10.0);
                        for pair in &lines {
                            ui.horizontal_top(|ui| {
                                let wide =
                                    pair.len() == 1 && info(&pair[0].1.name).target.is_some();
                                let half = if wide {
                                    ui.available_width() - SIDE * 2.0
                                } else {
                                    (ui.available_width() - SIDE * 2.0 - 12.0) / 2.0
                                };
                                ui.add_space(SIDE);
                                for (col, column, value) in pair.iter().copied() {
                                    ui.allocate_ui_with_layout(
                                        vec2(half, 0.0),
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.set_width(half);
                                            let info = info(&column.name);
                                            field(
                                                ui,
                                                tab,
                                                object_tab,
                                                cell.row,
                                                *col,
                                                column,
                                                value,
                                                &info,
                                                tag_of(*col, value),
                                                FieldSkin {
                                                    look: &look,
                                                    palette: &palette,
                                                    locale,
                                                    fold,
                                                },
                                                &mut actions,
                                            );
                                        },
                                    );
                                    ui.add_space(12.0);
                                }
                            });
                            ui.add_space(10.0);
                        }
                        for (col, column, value) in fields.iter().filter(|(_, c, v)| is_doc(c, v)) {
                            let top = ui.cursor().top();
                            widgets::hline(ui, ui.max_rect().x_range(), top, palette.outline);
                            ui.add_space(12.0);
                            ui.horizontal_top(|ui| {
                                ui.add_space(SIDE);
                                ui.vertical(|ui| {
                                    ui.set_width(ui.available_width() - SIDE);
                                    let info = info(&column.name);
                                    field(
                                        ui,
                                        tab,
                                        object_tab,
                                        cell.row,
                                        *col,
                                        column,
                                        value,
                                        &info,
                                        tag_of(*col, value),
                                        FieldSkin {
                                            look: &look,
                                            palette: &palette,
                                            locale,
                                            fold,
                                        },
                                        &mut actions,
                                    );
                                });
                            });
                            ui.add_space(12.0);
                        }
                    } else {
                        for (col, column, value) in &fields {
                            ui.add_space(10.0);
                            ui.horizontal_top(|ui| {
                                ui.add_space(SIDE);
                                ui.vertical(|ui| {
                                    ui.set_width(ui.available_width() - SIDE);
                                    let info = info(&column.name);
                                    field(
                                        ui,
                                        tab,
                                        object_tab,
                                        cell.row,
                                        *col,
                                        column,
                                        value,
                                        &info,
                                        tag_of(*col, value),
                                        FieldSkin {
                                            look: &look,
                                            palette: &palette,
                                            locale,
                                            fold,
                                        },
                                        &mut actions,
                                    );
                                });
                            });
                            ui.add_space(11.0);
                            let y = ui.cursor().top();
                            widgets::hline(
                                ui,
                                ui.max_rect().x_range(),
                                y - 0.5,
                                palette.surface_hover,
                            );
                        }
                    }
                });
        });
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
}

/// One field: its label (with a copy button, or a document's controls),
/// then its value.
#[allow(clippy::too_many_arguments)] // one call site per layout
fn field(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    object_tab: ObjectTabId,
    row: usize,
    col: usize,
    column: &tabletist_db::ColumnMeta,
    value: &Value,
    info: &FieldInfo,
    tag: Option<usize>,
    skin: FieldSkin<'_>,
    actions: &mut Vec<Action>,
) {
    let FieldSkin {
        look,
        palette,
        locale,
        fold,
    } = skin;
    let label_font = if look.terminal {
        theme::regular(theme::TEXT_LABEL)
    } else {
        theme::regular(theme::TEXT_SMALL)
    };
    let text = label(&column.name, &column.type_name, column.kind, info, look);
    let doc = json_doc(ui, value, column.kind);
    let width = ui.available_width();
    let (line, response) = ui.allocate_exact_size(vec2(width, 18.0), Sense::hover());
    let name_id = response.id;
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
    // Cut at the column's edge, as the field's own width allows.
    let room = line.width() - 30.0;
    let shown = crate::ui::grid::ellipsize(&text, room, false, |text| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), label_font.clone(), palette.dim)
            .size()
            .x
    });
    widgets::paint_text(
        ui,
        line.left(),
        line.center().y,
        &shown,
        label_font,
        palette.dim,
    );
    let copy_label = format!("{} {}", gettext(locale, "Copy"), column.name);
    let hovered = ui.rect_contains_pointer(line.expand2(vec2(SIDE, 30.0)));
    if let Some(doc) = &doc {
        // A document's own controls: fold everything, and copy.
        let copy = Rect::from_min_size(
            pos2(line.right() - 24.0, line.center().y - 12.0),
            vec2(24.0, 24.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(copy));
        if look.terminal {
            let hint = [("za", "fold", true), ("y", "copy", true)];
            let text_width = widgets::key_hints_width(ui, &[("za", "fold", true)], 0.0)
                + widgets::key_hints_width(ui, &[("y", "copy", true)], 0.0);
            let _ = text_width;
            let mut x = line.right();
            for (key, what, _) in hint.iter().rev() {
                let item = [(*key, *what, true)];
                let width = widgets::key_hints_width(ui, &item, 0.0);
                widgets::key_hints(ui, x - width, line.center().y, &item, 0.0, palette);
                x -= width + 14.0;
                if *key == "za" {
                    break;
                }
                widgets::paint_text(
                    ui,
                    x + 3.0,
                    line.center().y,
                    "·",
                    theme::regular(theme::TEXT_SMALL),
                    palette.dim,
                );
            }
            if copy_button(&mut child, &copy_label, false, look, palette).clicked() {
                ui.ctx().copy_text(format::plain_text(value));
            }
        } else {
            if copy_button(&mut child, &copy_label, true, look, palette).clicked() {
                ui.ctx().copy_text(format::plain_text(value));
            }
            let id = Id::new(("row-panel-json", tab, object_tab, row, col));
            let link_right = copy.left() - 12.0;
            json_view::fold_all_link(ui, id, doc, link_right, line.center().y, locale, palette);
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
    ui.add_space(if look.terminal { 1.0 } else { 2.0 });
    if value.is_null() {
        ui.label(
            RichText::new("NULL")
                .font(theme::data(look))
                .color(palette.faint),
        );
        return;
    }
    if let Some(doc) = doc {
        if fold {
            json_view::toggle_fold_all(ui, Id::new(("row-panel-json", tab, object_tab, row, col)));
        }
        if let Some(file) = json_view::attachment(&doc) {
            ui.add_space(if look.terminal { 6.0 } else { 8.0 });
            attachment_card(ui, &file, look, palette);
        }
        ui.add_space(8.0);
        let id = Id::new(("row-panel-json", tab, object_tab, row, col));
        if look.terminal {
            json_view::show(ui, id, &doc, &column.name, locale, palette, look);
        } else {
            Frame::new()
                .stroke(Stroke::new(widgets::hairline(ui), palette.outline))
                .corner_radius(CornerRadius::same(look.radius))
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    json_view::show(ui, id, &doc, &column.name, locale, palette, look);
                });
        }
        return;
    }
    let text = format::full_text(value);
    let expanded_id = Id::new(("row-panel-expanded", tab, object_tab, row, col));
    let expanded: bool = ui.data(|data| data.get_temp(expanded_id)).unwrap_or(false);
    let lines = text.lines().count();
    let size = format::human_size(text.len());
    let long = lines > format::COLLAPSE_LINES || text.len() > format::COLLAPSE_CHARS;
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
    let font = if matches!(column.kind, ValueKind::Json | ValueKind::Binary) {
        theme::mono(theme::TEXT_MONO)
    } else if look.terminal {
        theme::mono(theme::TEXT_SMALL)
    } else {
        theme::mono(look.data_size + 0.5)
    };
    // The follow link sits at the value's right.
    let follow = info.target.as_ref().map(|target| {
        if look.terminal {
            ("gd open".to_owned(), target.clone())
        } else {
            (
                format!("{} {} →", gettext(locale, "Open"), singular(&target.name)),
                target.clone(),
            )
        }
    });
    // The terminal colours a tag's text here too; macOS keeps its chips
    // to the grid.
    let color = match tag {
        Some(hue) if look.terminal => crate::ui::grid::tag_colors(hue, look, palette).0,
        _ => palette.text,
    };
    ui.horizontal_top(|ui| {
        let link = follow.as_ref().map_or(0.0, |(text, _)| {
            ui.painter()
                .layout_no_wrap(text.clone(), theme::regular(theme::TEXT), palette.accent)
                .size()
                .x
                + 12.0
        });
        let room = (ui.available_width() - link).max(24.0);
        ui.allocate_ui(vec2(room, 0.0), |ui| {
            ui.add(
                TextEdit::multiline(&mut shown.as_str())
                    .font(font)
                    .text_color(color)
                    // Flush with the field name above.
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO)
                    .desired_width(room)
                    .desired_rows(1),
            )
            .labelled_by(name_id);
        });
        if let Some((text, target)) = follow {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                let (font, color) = if look.terminal {
                    (theme::regular(theme::TEXT_SMALL), palette.text)
                } else {
                    (theme::regular(theme::TEXT), palette.accent)
                };
                let link = ui.add(
                    egui::Label::new(RichText::new(&text).font(font).color(color))
                        .sense(Sense::click()),
                );
                let name = format!("{} {}", gettext(locale, "Open"), target.name);
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
            format!("{} ({size})", gettext(locale, "Show all"))
        };
        if ui.link(label).clicked() {
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
        let (rect, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.panel);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::ZERO,
            Stroke::new(1.0, palette.outline),
            StrokeKind::Inside,
        );
        let y = rect.center().y;
        Icon::Image.image(palette.dim, 12.0).paint_at(
            ui,
            Rect::from_center_size(pos2(rect.left() + 13.0, y), vec2(12.0, 12.0)),
        );
        let x = rect.left() + 26.0;
        let x =
            x + widgets::paint_text(
                ui,
                x,
                y,
                &file.filename,
                theme::medium(theme::TEXT),
                palette.text,
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
            &details.join(" · ").replace(" × ", "×"),
            theme::regular(theme::TEXT_SMALL),
            palette.dim,
        );
        return;
    }
    let (rect, _) = ui.allocate_exact_size(vec2(width, 56.0), Sense::hover());
    let corner = CornerRadius::same(look.radius);
    ui.painter().rect_stroke(
        rect,
        corner,
        Stroke::new(widgets::hairline(ui), palette.outline),
        StrokeKind::Inside,
    );
    let tile = Rect::from_min_size(pos2(rect.left() + 8.0, rect.top() + 8.0), vec2(40.0, 40.0));
    ui.painter()
        .rect_filled(tile, CornerRadius::same(6), palette.surface);
    let icon = if file.image { Icon::Image } else { Icon::Copy };
    icon.image(palette.dim, 16.0)
        .paint_at(ui, Rect::from_center_size(tile.center(), vec2(16.0, 16.0)));
    let x = tile.right() + 12.0;
    widgets::paint_text(
        ui,
        x,
        rect.top() + 20.0,
        &file.filename,
        theme::semibold(theme::TEXT),
        palette.text,
    );
    widgets::paint_text(
        ui,
        x,
        rect.top() + 37.5,
        &file.details,
        theme::regular(theme::TEXT_SMALL),
        palette.dim,
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
    let labels: [(Icon, String); 3] = if look.terminal {
        [
            (Icon::Pencil, "e edit".to_owned()),
            (Icon::Copy, "yy p duplicate".to_owned()),
            (Icon::Trash2, "dd delete".to_owned()),
        ]
    } else {
        [
            (Icon::Pencil, gettext(locale, "Edit").into_owned()),
            (Icon::Copy, gettext(locale, "Duplicate").into_owned()),
            (Icon::Trash2, gettext(locale, "Delete").into_owned()),
        ]
    };
    let inner = rect.shrink2(vec2(SIDE, 0.0));
    let gap = if look.terminal { 6.0 } else { 8.0 };
    let height = if look.terminal { 22.0 } else { 28.0 };
    let top = rect.top() + if look.terminal { 10.0 } else { 12.0 };
    // Widths in proportion to the labels, as the design sets them.
    let widths: Vec<f32> = labels
        .iter()
        .map(|(_, text)| {
            ui.painter()
                .layout_no_wrap(text.clone(), theme::medium(theme::TEXT), palette.text)
                .size()
                .x
                + if look.terminal { 20.0 } else { 44.0 }
        })
        .collect();
    let total: f32 = widths.iter().sum();
    let room = inner.width() - gap * 2.0;
    let mut x = inner.left();
    for ((icon, text), width) in labels.iter().zip(widths) {
        let width = width / total * room;
        let place = Rect::from_min_size(pos2(x, top), vec2(width, height));
        x += width + gap;
        let mut button = widgets::ButtonSpec::new(text).disabled(&reason);
        if !look.terminal {
            button = button.icon(*icon);
        }
        button.show_at(ui, place, look, palette);
    }
    let y = top + height + if look.terminal { 13.0 } else { 15.0 };
    if look.terminal {
        widgets::paint_text(
            ui,
            inner.left(),
            y,
            &gettext(
                locale,
                "read-only in 0.1.0 · editing arrives in a later version",
            ),
            theme::regular(theme::TEXT_SMALL),
            palette.dim,
        );
    } else {
        Icon::Lock.image(palette.dim, 12.0).paint_at(
            ui,
            Rect::from_center_size(pos2(inner.left() + 6.0, y), vec2(12.0, 12.0)),
        );
        widgets::paint_text(
            ui,
            inner.left() + 18.0,
            y,
            &reason,
            theme::regular(theme::TEXT_SMALL),
            palette.dim,
        );
    }
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
