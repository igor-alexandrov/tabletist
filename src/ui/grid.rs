//! The data grid: a sticky header with resizable columns over virtualized
//! rows, one selected cell. Reads data through a closure and reports clicks;
//! it never changes application state.

use std::borrow::Cow;

use egui::{
    Align2, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2,
    vec2,
};
use tabletist_db::SortDir;

use crate::model::CellPos;
use crate::theme::{self, Icon, Palette};
use crate::ui::widgets::virtual_rows;

/// The header's height, per look.
pub fn header_height(look: &crate::theme::Look) -> f32 {
    if look.terminal { 32.0 } else { 41.0 }
}
/// The terminal look's gutter, where the selected row's marker sits.
const GUTTER: f32 = 16.0;
const MIN_WIDTH: f32 = 48.0;
const MAX_INITIAL_WIDTH: f32 = 331.0;
/// Space between a cell's edge and its text.
fn cell_pad(look: &crate::theme::Look) -> f32 {
    if look.terminal { 8.0 } else { 12.0 }
}
const HANDLE_WIDTH: f32 = 6.0;
const SAMPLE_ROWS: usize = 50;

pub struct Column<'a> {
    pub name: &'a str,
    /// Under the name: the type, `int8 → books` for a foreign key.
    pub type_line: String,
    pub numeric: bool,
    pub sort: Option<SortDir>,
    /// Part of the primary key.
    pub key: bool,
}

/// How a cell draws its text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    Plain,
    /// A short value from a small set, tagged in one of the tag colours.
    Tag(usize),
    /// A JSON document with this many keys: a `{ n }` chip, then the text.
    Json(usize),
}

pub struct Cell<'a> {
    pub text: Cow<'a, str>,
    pub null: bool,
    pub style: Style,
}

#[derive(Debug, Default, PartialEq)]
pub struct GridOutput {
    pub clicked: Option<CellPos>,
    pub sort_clicked: Option<usize>,
}

/// How wide `text` is in `font`, in points (laid out once, then cached).
fn text_width(ui: &Ui, text: &str, font: &egui::FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
        .size()
        .x
}

/// The column under `x`, measured from the first data column's left edge.
pub fn column_at(widths: &[f32], x: f32) -> usize {
    let mut edge = 0.0;
    for (index, width) in widths.iter().enumerate() {
        edge += width;
        if x < edge {
            return index;
        }
    }
    widths.len().saturating_sub(1)
}

/// A data row's background: the selected row's colour wins, then the hover
/// tint, then (in the terminal look) the zebra stripe on odd rows. `None`
/// leaves the window colour.
pub fn row_fill(
    selected: bool,
    hovered: bool,
    odd: bool,
    look: &crate::theme::Look,
    palette: &Palette,
) -> Option<egui::Color32> {
    if selected {
        Some(palette.selection)
    } else if hovered {
        Some(palette.panel.lerp_to_gamma(palette.surface, 0.5))
    } else if odd && look.terminal {
        Some(palette.panel)
    } else {
        None
    }
}

/// The colours of tag `hue`: its text, and its fill (none in the terminal
/// look, where the text alone is coloured).
pub fn tag_colors(
    hue: usize,
    look: &crate::theme::Look,
    palette: &Palette,
) -> (egui::Color32, Option<egui::Color32>) {
    let hues = [
        palette.orange,
        palette.info,
        palette.success,
        palette.magenta,
        palette.danger,
    ];
    let color = hues[hue % hues.len()];
    if look.terminal {
        let color = if hue.is_multiple_of(hues.len()) {
            palette.warning
        } else {
            color
        };
        (color, None)
    } else {
        (color, Some(palette.window.lerp_to_gamma(color, 0.1)))
    }
}

/// `text` shortened with "…" to fit `max` points as measured by `width`:
/// text keeps its start; numbers (`keep_end`) keep their last digits, the
/// ones that tell rows apart. Never cuts a character in half.
pub fn ellipsize<'t>(
    text: &'t str,
    max: f32,
    keep_end: bool,
    width: impl Fn(&str) -> f32,
) -> std::borrow::Cow<'t, str> {
    if width(text) <= max {
        return text.into();
    }
    let chars: Vec<char> = text.chars().collect();
    let candidate = |kept: usize| -> String {
        if keep_end {
            std::iter::once('…')
                .chain(chars[chars.len() - kept..].iter().copied())
                .collect()
        } else {
            chars[..kept]
                .iter()
                .copied()
                .chain(std::iter::once('…'))
                .collect()
        }
    };
    // The most characters that still fit, by bisection.
    let (mut fits, mut too_many) = (0, chars.len());
    while too_many - fits > 1 {
        let middle = (fits + too_many) / 2;
        if width(&candidate(middle)) <= max {
            fits = middle;
        } else {
            too_many = middle;
        }
    }
    candidate(fits).into()
}

/// Widths that fit the header and the first rows as measured by `width`
/// (header names in the header font, values in the cell font), within
/// limits.
pub fn initial_widths<'a>(
    columns: &[Column<'_>],
    rows: usize,
    pad: f32,
    width: &impl Fn(&str) -> f32,
    cell: &mut impl FnMut(usize, usize) -> Cell<'a>,
) -> Vec<f32> {
    columns
        .iter()
        .enumerate()
        .map(|(col, column)| {
            // Room for the key icon and the sort arrow beside the name.
            let header = width(column.name).max(width(&column.type_line)) + 2.0 * pad + 16.0;
            let widest = (0..rows.min(SAMPLE_ROWS))
                .map(|row| {
                    let cell = cell(row, col);
                    let chip = match cell.style {
                        Style::Plain => 0.0,
                        Style::Tag(_) => 16.0,
                        Style::Json(_) => 44.0,
                    };
                    width(&cell.text) + chip
                })
                .fold(0.0, f32::max)
                + 2.0 * pad;
            header
                .max(widest)
                .clamp(MIN_WIDTH, MAX_INITIAL_WIDTH)
                .ceil()
        })
        .collect()
}

#[allow(clippy::too_many_arguments)] // one call site per view; a struct adds nothing
pub fn show<'a>(
    ui: &mut Ui,
    id: Id,
    columns: &[Column<'_>],
    row_count: usize,
    first_row_number: u64,
    selection: Option<CellPos>,
    palette: &Palette,
    look: &crate::theme::Look,
    mut cell: impl FnMut(usize, usize) -> Cell<'a>,
) -> GridOutput {
    let mut output = GridOutput::default();
    let row_height = look.grid_row;
    let header_height = header_height(look);
    let pad = cell_pad(look);
    let gutter = if look.terminal { GUTTER } else { 0.0 };
    let widths_id = id.with(("widths", columns.len()));
    let last_id = id.with("last-selection");
    let mut widths: Vec<f32> = ui
        .data(|data| data.get_temp::<Vec<f32>>(widths_id))
        .filter(|widths| widths.len() == columns.len())
        .unwrap_or_else(|| {
            let font = theme::data(look);
            let width = |text: &str| text_width(ui, text, &font);
            initial_widths(columns, row_count, pad, &width, &mut cell)
        });
    let last: Option<CellPos> = ui
        .data(|data| data.get_temp::<Option<CellPos>>(last_id))
        .flatten();
    let reveal = selection.filter(|cell| Some(*cell) != last);
    let total = gutter + widths.iter().sum::<f32>();
    let hairline = crate::ui::widgets::hairline(ui);
    let visible = ui.max_rect();

    let scroll = egui::ScrollArea::both()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let origin = ui.cursor().min;
            let full = total.max(ui.available_width());
            ui.allocate_space(vec2(total, header_height));

            if let Some(target) = reveal {
                let col = target.col.min(widths.len().saturating_sub(1));
                let x = origin.x + gutter + widths[..col].iter().sum::<f32>();
                let y = origin.y + header_height + target.row as f32 * row_height;
                // Include the header's height above the row so the sticky
                // header never covers it.
                let rect = Rect::from_min_size(
                    pos2(x, y - header_height),
                    vec2(
                        widths.get(col).copied().unwrap_or(0.0),
                        row_height + header_height,
                    ),
                );
                ui.scroll_to_rect(rect, None);
            }

            virtual_rows(ui, row_count, row_height, |ui, row| {
                let (rect, response) =
                    ui.allocate_exact_size(vec2(full, row_height), Sense::click());
                let number = first_row_number + row as u64 + 1;
                let label = format!("Row {number}");
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
                if response.clicked() {
                    let col = response
                        .interact_pointer_pos()
                        .map(|pointer| column_at(&widths, pointer.x - rect.left() - gutter))
                        .unwrap_or(0);
                    output.clicked = Some(CellPos { row, col });
                }
                let painter = ui.painter().clone();
                let selected_row = selection.is_some_and(|cell| cell.row == row);
                if let Some(fill) = row_fill(
                    selected_row,
                    response.hovered(),
                    row % 2 == 1,
                    look,
                    palette,
                ) {
                    painter.rect_filled(rect, CornerRadius::ZERO, fill);
                }
                if !look.terminal {
                    let y = painter.round_to_pixel_center(rect.bottom() - hairline / 2.0);
                    painter.hline(rect.x_range(), y, Stroke::new(hairline, palette.surface));
                }
                if selected_row {
                    let bar = if look.terminal {
                        Rect::from_center_size(
                            pos2(rect.left() + 6.0, rect.center().y),
                            vec2(3.0, row_height * 0.55),
                        )
                    } else {
                        Rect::from_min_size(rect.min, vec2(3.0, rect.height()))
                    };
                    painter.rect_filled(bar, CornerRadius::ZERO, palette.accent);
                }
                let mut x = rect.left() + gutter;
                for (col, width) in widths.iter().enumerate() {
                    let cell_rect =
                        Rect::from_min_size(pos2(x, rect.top()), vec2(*width, row_height));
                    x += width;
                    if !ui.is_rect_visible(cell_rect) {
                        continue;
                    }
                    let content = cell(row, col);
                    draw_cell(
                        ui,
                        &painter,
                        cell_rect,
                        &columns[col],
                        &content,
                        look,
                        palette,
                    );
                    // The row is selected; a cell past the first is marked
                    // too, for the keys that act on one cell.
                    if selection == Some(CellPos { row, col }) && col > 0 {
                        painter.rect_stroke(
                            cell_rect.shrink(1.0),
                            CornerRadius::same(look.radius.min(3)),
                            Stroke::new(1.5, palette.accent),
                            StrokeKind::Inside,
                        );
                    }
                }
            });

            // The header, painted over the rows at the top of the visible
            // area so it stays put while rows scroll under it. Its widgets
            // come after the rows', so they sit on top.
            let top = ui.clip_rect().top().max(origin.y);
            let header = Rect::from_min_size(pos2(origin.x, top), vec2(full, header_height));
            let painter = ui.painter().clone();
            painter.rect_filled(header, CornerRadius::ZERO, palette.panel);
            let y = painter.round_to_pixel_center(header.bottom() - hairline / 2.0);
            painter.hline(header.x_range(), y, Stroke::new(hairline, palette.outline));
            let mut x = origin.x + gutter;
            for (col, column) in columns.iter().enumerate() {
                let rect = Rect::from_min_size(pos2(x, top), vec2(widths[col], header_height));
                let response = ui.interact(rect, id.with(("header", col)), Sense::click());
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, column.name));
                if response.clicked() {
                    output.sort_clicked = Some(col);
                }
                if response.hovered() {
                    painter.rect_filled(
                        rect.shrink2(vec2(0.0, hairline)),
                        CornerRadius::ZERO,
                        palette.surface,
                    );
                }
                draw_header(ui, &painter, rect, column, pad, look, palette);
                let handle = Rect::from_min_max(
                    pos2(rect.right() - HANDLE_WIDTH / 2.0, top),
                    pos2(rect.right() + HANDLE_WIDTH / 2.0, top + header_height),
                );
                let drag = ui.interact(handle, id.with(("resize", col)), Sense::drag());
                if drag.hovered() || drag.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
                    painter.vline(
                        rect.right(),
                        header.y_range(),
                        Stroke::new(hairline, palette.outline),
                    );
                }
                if drag.dragged() {
                    widths[col] = (widths[col] + drag.drag_delta().x).max(MIN_WIDTH);
                }
                x += widths[col];
            }
            origin
        });

    // Columns the viewport cuts off entirely: a pill at the header's right
    // edge says how many, and scrolls to them.
    if !look.terminal {
        let offset = scroll.state.offset.x;
        let view_right = offset + scroll.inner_rect.width();
        let mut edge = gutter;
        let mut hidden = 0;
        let mut first_hidden = None;
        for width in &widths {
            if edge >= view_right {
                hidden += 1;
                first_hidden.get_or_insert(edge);
            }
            edge += width;
        }
        if hidden > 0 {
            let text = format!("+{hidden}");
            let font = theme::medium(theme::TEXT_SMALL);
            let text_width = text_width(ui, &text, &font);
            let size = vec2(text_width + 30.0, 25.0);
            let pill = Rect::from_min_size(
                pos2(
                    visible.right() - 6.0 - size.x,
                    scroll.inner_rect.top() + (header_height - size.y) / 2.0,
                ),
                size,
            );
            let response = ui.interact(pill, id.with("hidden-columns"), Sense::click());
            response.widget_info(|| {
                WidgetInfo::labeled(WidgetType::Button, true, format!("{hidden} more columns"))
            });
            let painter = ui.painter();
            painter.rect_filled(pill, CornerRadius::same(13), palette.text);
            painter.text(
                pos2(pill.left() + 11.0, pill.center().y),
                Align2::LEFT_CENTER,
                text,
                font,
                palette.window,
            );
            Icon::ChevronRight.image(palette.window, 11.0).paint_at(
                ui,
                Rect::from_center_size(
                    pos2(pill.right() - 11.0, pill.center().y),
                    vec2(11.0, 11.0),
                ),
            );
            if response.clicked()
                && let Some(left) = first_hidden
            {
                let origin = scroll.inner;
                let target = Rect::from_min_size(
                    pos2(origin.x + left, scroll.inner_rect.top()),
                    vec2(1.0, 1.0),
                );
                let _ = target;
                let mut state = scroll.state;
                state.offset.x = left;
                state.store(ui.ctx(), scroll.id);
            }
        }
    }

    ui.data_mut(|data| {
        data.insert_temp(widths_id, widths);
        data.insert_temp(last_id, selection);
    });
    output
}

/// A column's header: its name (accented when sorted, with the arrow),
/// the key icon, and the type line under it.
fn draw_header(
    ui: &Ui,
    painter: &egui::Painter,
    rect: Rect,
    column: &Column<'_>,
    pad: f32,
    look: &crate::theme::Look,
    palette: &Palette,
) {
    let clip = painter.with_clip_rect(rect.shrink2(vec2(pad / 2.0, 0.0)).intersect(ui.clip_rect()));
    let (name_y, type_y) = if look.terminal {
        (rect.top() + 10.5, rect.top() + 23.0)
    } else {
        (rect.top() + 14.5, rect.top() + 28.5)
    };
    // Sorted columns, and in the terminal the key, name themselves in the
    // accent.
    let name_color = if column.sort.is_some() {
        if look.terminal {
            palette.accent
        } else {
            palette.accent_hover
        }
    } else if column.key && look.terminal {
        palette.accent
    } else {
        palette.text
    };
    let name_font = theme::semibold(theme::TEXT);
    let type_font = theme::regular(theme::TEXT_CAPTION);
    let name_width = text_width(ui, column.name, &name_font);
    let type_width = text_width(ui, &column.type_line, &type_font);
    let arrow = if column.sort.is_some() { 14.0 } else { 0.0 };
    let key = if column.key && !look.terminal {
        15.0
    } else {
        0.0
    };
    // Numbers sit at the right, their header with them.
    let (name_x, type_x) = if column.numeric {
        (
            rect.right() - pad - arrow - name_width,
            rect.right() - pad - type_width,
        )
    } else {
        (rect.left() + pad + key, rect.left() + pad)
    };
    if key > 0.0 {
        let at = if column.numeric {
            name_x - key
        } else {
            rect.left() + pad
        };
        Icon::KeyRound.image(palette.warning, 11.0).paint_at(
            ui,
            Rect::from_center_size(pos2(at + 5.5, name_y), vec2(11.0, 11.0)),
        );
    }
    clip.text(
        pos2(name_x, name_y),
        Align2::LEFT_CENTER,
        column.name,
        name_font,
        name_color,
    );
    if let Some(dir) = column.sort {
        let icon = if dir == SortDir::Asc {
            Icon::ArrowUp
        } else {
            Icon::ArrowDown
        };
        icon.image(name_color, 11.0).paint_at(
            ui,
            Rect::from_center_size(pos2(name_x + name_width + 8.5, name_y), vec2(11.0, 11.0)),
        );
    }
    clip.text(
        pos2(type_x, type_y),
        Align2::LEFT_CENTER,
        &column.type_line,
        type_font,
        palette.dim,
    );
}

/// One cell: its text, tag or JSON chip, cut to fit.
fn draw_cell(
    ui: &Ui,
    painter: &egui::Painter,
    rect: Rect,
    column: &Column<'_>,
    content: &Cell<'_>,
    look: &crate::theme::Look,
    palette: &Palette,
) {
    let pad = cell_pad(look);
    let clip = painter.with_clip_rect(rect.shrink2(vec2(pad / 2.0, 0.0)).intersect(ui.clip_rect()));
    let font = theme::data(look);
    let center = rect.center().y;
    let room = rect.width() - 2.0 * pad;
    if content.null {
        clip.text(
            pos2(rect.left() + pad, center),
            Align2::LEFT_CENTER,
            "NULL",
            font,
            palette.faint,
        );
        return;
    }
    match content.style {
        Style::Tag(hue) => {
            let (color, fill) = tag_colors(hue, look, palette);
            let shown = ellipsize(&content.text, room - 12.0, false, |text| {
                text_width(ui, text, &font)
            });
            let width = text_width(ui, &shown, &font);
            if let Some(fill) = fill {
                let chip = Rect::from_min_size(
                    pos2(rect.left() + pad, center - 10.0),
                    vec2(width + 12.0, 20.0),
                );
                clip.rect_filled(chip, CornerRadius::same(4), fill);
                clip.text(
                    pos2(chip.left() + 6.0, center),
                    Align2::LEFT_CENTER,
                    shown,
                    font,
                    color,
                );
            } else {
                clip.text(
                    pos2(rect.left() + pad, center),
                    Align2::LEFT_CENTER,
                    shown,
                    font,
                    color,
                );
            }
        }
        Style::Json(count) => {
            let mut left = rect.left() + pad;
            if look.terminal {
                let mark = "{…}";
                left += text_width(ui, mark, &font) + 8.0;
                clip.text(
                    pos2(rect.left() + pad, center),
                    Align2::LEFT_CENTER,
                    mark,
                    font.clone(),
                    palette.text,
                );
            } else {
                let chip_font = theme::mono(theme::TEXT_LABEL);
                let label = format!("{{ {count} }}");
                let width = text_width(ui, &label, &chip_font) + 12.0;
                let chip = Rect::from_min_size(pos2(left, center - 10.0), vec2(width, 20.0));
                clip.rect_stroke(
                    chip,
                    CornerRadius::same(4),
                    Stroke::new(crate::ui::widgets::hairline(ui), palette.border),
                    StrokeKind::Inside,
                );
                clip.text(
                    chip.center(),
                    Align2::CENTER_CENTER,
                    label,
                    chip_font,
                    palette.dim,
                );
                left = chip.right() + 8.0;
            }
            let room = rect.right() - pad - left;
            let shown = match content.text.split_once(" · ") {
                // The first string keeps both ends in 70% of the room;
                // what follows fills the rest from its start.
                Some((first, rest)) => {
                    let first = crate::ui::format::ellipsize_middle(first, room * 0.7, |text| {
                        text_width(ui, text, &font)
                    });
                    let joined = format!("{first} · {rest}");
                    ellipsize(&joined, room, false, |text| text_width(ui, text, &font)).into_owned()
                }
                None => crate::ui::format::ellipsize_middle(&content.text, room, |text| {
                    text_width(ui, text, &font)
                }),
            };
            clip.text(
                pos2(left, center),
                Align2::LEFT_CENTER,
                shown,
                font,
                palette.secondary,
            );
        }
        Style::Plain => {
            let numeric = column.numeric;
            let (anchor, at) = if numeric {
                (Align2::RIGHT_CENTER, pos2(rect.right() - pad, center))
            } else {
                (Align2::LEFT_CENTER, pos2(rect.left() + pad, center))
            };
            let shown = ellipsize(&content.text, room, numeric, |text| {
                text_width(ui, text, &font)
            });
            clip.text(at, anchor, shown, font, palette.text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Role;

    #[test]
    fn selection_wins_over_hover_and_hover_over_stripes() {
        let palette = Palette::light();
        let terminal = crate::theme::Look::omarchy();
        let mac = crate::theme::Look::macos();
        let selected = Some(palette.selection);
        let hovered = Some(palette.panel.lerp_to_gamma(palette.surface, 0.5));
        assert_eq!(row_fill(true, true, true, &mac, &palette), selected);
        assert_eq!(row_fill(true, false, false, &terminal, &palette), selected);
        assert_eq!(row_fill(false, true, true, &terminal, &palette), hovered);
        assert_eq!(row_fill(false, true, false, &mac, &palette), hovered);
        assert_eq!(
            row_fill(false, false, true, &terminal, &palette),
            Some(palette.panel)
        );
        assert_eq!(
            row_fill(false, false, true, &mac, &palette),
            None,
            "no stripes on macOS"
        );
        assert_eq!(row_fill(false, false, false, &terminal, &palette), None);
    }

    fn columns() -> Vec<Column<'static>> {
        vec![
            Column {
                name: "id",
                type_line: "INTEGER".into(),
                numeric: true,
                sort: None,
                key: true,
            },
            Column {
                name: "email",
                type_line: "TEXT".into(),
                numeric: false,
                sort: Some(SortDir::Asc),
                key: false,
            },
        ]
    }

    /// Runs one frame of a grid with `rows` rows and returns its output and
    /// the AccessKit tree.
    fn frame(
        ctx: &egui::Context,
        rows: usize,
        events: Vec<egui::Event>,
    ) -> (GridOutput, egui::accesskit::TreeUpdate) {
        let mut result = GridOutput::default();
        let palette = crate::theme::Palette::dark();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 400.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                let columns = columns();
                result = show(
                    ui,
                    egui::Id::new("grid"),
                    &columns,
                    rows,
                    0,
                    None,
                    &palette,
                    &crate::theme::Look::standard(),
                    |row, col| Cell {
                        text: format!("r{row}c{col}").into(),
                        null: false,
                        style: Style::Plain,
                    },
                );
            },
        );
        // egui panics if a frame's texture updates are dropped unhandled.
        output.textures_delta.clear();
        (
            result,
            output.platform_output.accesskit_update.expect("accesskit"),
        )
    }

    fn click(target: egui::accesskit::NodeId) -> egui::Event {
        egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: target,
            action: egui::accesskit::Action::Click,
            data: None,
        })
    }

    #[test]
    fn columns_are_found_by_x() {
        let widths = [100.0, 50.0, 80.0];
        assert_eq!(column_at(&widths, -10.0), 0);
        assert_eq!(column_at(&widths, 99.0), 0);
        assert_eq!(column_at(&widths, 120.0), 1);
        assert_eq!(column_at(&widths, 1000.0), 2);
    }

    /// Eight points per character, "…" included.
    fn width(text: &str) -> f32 {
        text.chars().count() as f32 * 8.0
    }

    #[test]
    fn long_text_keeps_its_start_and_numbers_their_end() {
        assert_eq!(ellipsize("short", 100.0, false, width), "short");
        assert_eq!(ellipsize("abcdefghij", 48.0, false, width), "abcde…");
        assert_eq!(
            ellipsize("1628910071209526786", 48.0, true, width),
            "…26786"
        );
        assert_eq!(ellipsize("abcdefghij", 4.0, false, width), "…");
    }

    #[test]
    fn initial_widths_fit_the_measured_text() {
        let columns = columns();
        let widths = initial_widths(&columns, 3, 6.0, &width, &mut |_, col| Cell {
            text: if col == 0 {
                "1628910071209526786".into()
            } else {
                "x".into()
            },
            null: false,
            style: Style::Plain,
        });
        // Nineteen digits at eight points, plus padding: nothing clipped.
        assert!(widths[0] >= 19.0 * 8.0 + 12.0, "{widths:?}");
    }

    #[test]
    fn initial_widths_fit_content_within_limits() {
        let columns = columns();
        let widths = initial_widths(&columns, 3, 6.0, &width, &mut |_, col| Cell {
            text: if col == 0 {
                "1".into()
            } else {
                "x".repeat(500).into()
            },
            null: false,
            style: Style::Plain,
        });
        assert!(widths[0] >= 48.0 && widths[0] < 120.0, "{widths:?}");
        assert_eq!(widths[1], MAX_INITIAL_WIDTH);
    }

    #[test]
    fn only_visible_rows_are_built() {
        let ctx = egui::Context::default();
        // The header draws with Inter's named weights, which need the fonts.
        crate::theme::install(&ctx, false, &crate::theme::Look::standard());
        ctx.enable_accesskit();
        frame(&ctx, 100_000, vec![]);
        let (_, tree) = frame(&ctx, 100_000, vec![]);
        let rows = tree
            .nodes
            .iter()
            .filter(|(_, node)| node.label().is_some_and(|label| label.starts_with("Row ")))
            .count();
        assert!(rows > 5 && rows < 40, "{rows} rows built");
    }

    #[test]
    fn clicking_a_header_reports_the_column_and_clicking_a_row_selects_it() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx, false, &crate::theme::Look::standard());
        ctx.enable_accesskit();
        frame(&ctx, 10, vec![]);
        let (_, tree) = frame(&ctx, 10, vec![]);
        let email = crate::testing::node(&tree, "email", Role::Button).expect("email header");
        let (output, _) = frame(&ctx, 10, vec![click(email)]);
        assert_eq!(output.sort_clicked, Some(1));
        let (_, tree) = frame(&ctx, 10, vec![]);
        let row = crate::testing::node(&tree, "Row 3", Role::Button).expect("row 3");
        let (output, _) = frame(&ctx, 10, vec![click(row)]);
        assert_eq!(output.clicked, Some(CellPos { row: 2, col: 0 }));
    }
}
