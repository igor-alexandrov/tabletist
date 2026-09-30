//! The data grid: a sticky header with resizable columns over virtualized
//! rows, one selected cell. Reads data through a closure and reports clicks;
//! it never changes application state.

use std::borrow::Cow;

use egui::{
    CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2,
};
use tabletist_db::SortDir;

use crate::model::CellPos;
use crate::theme::{DataFont, Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::widgets::virtual_rows;

/// The header's height, per look.
pub fn header_height(look: &crate::theme::Look) -> f32 {
    // 44 (macOS) or 40 (terminal), and the rule under the header.
    if look.terminal { 41.0 } else { 45.0 }
}
/// The terminal look's gutter, where the selected row's cursor sits.
const GUTTER: f32 = 22.0;
const MIN_WIDTH: f32 = 48.0;
const MAX_INITIAL_WIDTH: f32 = 331.0;
/// A numeric key column's width, as the design's grids fix it: 64 on
/// macOS, 56 in the terminal (0 elsewhere: sized to its content).
fn key_width(look: &crate::theme::Look) -> f32 {
    match (look.terminal, look.faces) {
        (true, _) => 56.0,
        (false, crate::theme::Faces::Plex) => 64.0,
        _ => 0.0,
    }
}

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
    /// Takes the room left over when every column fits (a document).
    pub flexible: bool,
}

/// How a cell draws its text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    Plain,
    /// A short value from a small set, tagged in one of the tag colours.
    Tag(usize),
    /// A JSON document with this many keys: a `{ n }` chip, then the text.
    Json(usize),
    /// A colour (`#3a7bd5`): a swatch of it, then the text.
    Color(egui::Color32),
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

/// How wide `text` is in `role`, in points (laid out once, then cached).
fn text_width(ui: &Ui, text: &str, role: TextRole, look: &Look) -> f32 {
    role.width(ui.ctx(), look.faces, text)
}

/// The role data is drawn in: monospace on macOS and Omarchy, the body
/// elsewhere. Row-panel values use it too.
pub fn data_role(look: &Look) -> TextRole {
    match look.data_font {
        DataFont::Proportional => TextRole::UiBody,
        DataFont::Monospace => TextRole::pick(look, TextRole::GridCell, TextRole::OBody),
    }
}

/// Paints `text` in `role` at `x` (its left edge, or its right with
/// `right`), centred on `y`.
#[allow(clippy::too_many_arguments)] // the text, its place, and its look
fn paint(
    painter: &egui::Painter,
    ui: &Ui,
    role: TextRole,
    text: &str,
    color: egui::Color32,
    x: f32,
    y: f32,
    right: bool,
    look: &Look,
) {
    let laid = Text::one(look, role, text, color).layout(ui.ctx());
    if right {
        laid.paint_right(painter, x, y);
    } else {
        laid.paint_left(painter, x, y);
    }
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
        (color, None)
    } else {
        (color, Some(palette.window.lerp_to_gamma(color, 0.1)))
    }
}

/// A grid swatch's side, and the space after it.
const SWATCH: f32 = 12.0;
const SWATCH_GAP: f32 = 6.0;

/// A square of `color`, `side` points wide, centred on `center`. A
/// translucent colour sits on a checkerboard that shows through it; a
/// hairline keeps a colour close to the window's visible.
pub fn paint_swatch(
    painter: &egui::Painter,
    ui: &Ui,
    center: egui::Pos2,
    side: f32,
    color: egui::Color32,
    look: &Look,
    palette: &Palette,
) {
    let rect = Rect::from_center_size(center, vec2(side, side));
    let radius = look.radius.min(3);
    let corner = CornerRadius::same(radius);
    if !color.is_opaque() {
        // Four squares, light and dark, each rounded at its outer corner.
        let half = side / 2.0;
        for (index, (dx, dy)) in [(0.0, 0.0), (half, 0.0), (0.0, half), (half, half)]
            .into_iter()
            .enumerate()
        {
            let square = Rect::from_min_size(rect.min + vec2(dx, dy), vec2(half, half));
            let round = |at: usize| if index == at { radius } else { 0 };
            let corner = CornerRadius {
                nw: round(0),
                ne: round(1),
                sw: round(2),
                se: round(3),
            };
            let shade = if index == 0 || index == 3 {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_gray(204)
            };
            painter.rect_filled(square, corner, shade);
        }
    }
    painter.rect_filled(rect, corner, color);
    painter.rect_stroke(
        rect,
        corner,
        Stroke::new(crate::ui::widgets::hairline(ui), palette.border),
        StrokeKind::Inside,
    );
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
    (pad, key_width): (f32, f32),
    width: &impl Fn(&str) -> f32,
    cell: &mut impl FnMut(usize, usize) -> Cell<'a>,
) -> Vec<f32> {
    columns
        .iter()
        .enumerate()
        .map(|(col, column)| {
            // Room for the key icon and the sort arrow beside the name; a
            // numeric key takes the design's fixed width (its cells rarely
            // need more).
            let header = if column.key && column.numeric && key_width > 0.0 {
                key_width
            } else {
                width(column.name).max(width(&column.type_line)) + 2.0 * pad + 16.0
            };
            let widest = (0..rows.min(SAMPLE_ROWS))
                .map(|row| {
                    let cell = cell(row, col);
                    let chip = match cell.style {
                        Style::Plain => 0.0,
                        Style::Tag(_) => 16.0,
                        Style::Json(_) => 44.0,
                        Style::Color(_) => SWATCH + SWATCH_GAP,
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
            let role = data_role(look);
            let width = |text: &str| text_width(ui, text, role, look);
            let mut widths = initial_widths(
                columns,
                row_count,
                (pad, key_width(look)),
                &width,
                &mut cell,
            );
            // When every column fits, a document column takes what is left,
            // as the design's `1fr`.
            let room = ui.available_width() - gutter;
            let used: f32 = widths.iter().sum();
            if used < room
                && let Some(flexible) = columns.iter().position(|column| column.flexible)
            {
                widths[flexible] += (room - used).floor();
            }
            widths
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
                    if look.terminal {
                        // The cursor: a bold accent block in the gutter.
                        let cursor =
                            Text::one(look, TextRole::OGroup, "▌", palette.accent).layout(ui.ctx());
                        cursor.paint(
                            &painter,
                            pos2(rect.left() + GUTTER / 2.0, rect.center().y) - cursor.size() / 2.0,
                        );
                    } else {
                        let bar = Rect::from_min_size(rect.min, vec2(3.0, rect.height()));
                        painter.rect_filled(bar, CornerRadius::ZERO, palette.accent);
                    }
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
            let header_fill = if look.terminal {
                palette.window
            } else {
                palette.panel
            };
            painter.rect_filled(header, CornerRadius::ZERO, header_fill);
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
            let role = TextRole::FieldLabel;
            let text_width = text_width(ui, &text, role, look);
            // 8 in, the count, 4, a 10 pt chevron, 8; 28 tall, 8 from the
            // grid's top and right.
            let size = vec2(8.0 + text_width + 4.0 + 10.0 + 8.0, 28.0);
            let pill = Rect::from_min_size(
                pos2(
                    visible.right() - 8.0 - size.x,
                    scroll.inner_rect.top() + 8.0,
                ),
                size,
            );
            let response = ui.interact(pill, id.with("hidden-columns"), Sense::click());
            response.widget_info(|| {
                WidgetInfo::labeled(WidgetType::Button, true, format!("{hidden} more columns"))
            });
            let painter = ui.painter();
            // The fade the pill sits on: 56 wide, clear to the window.
            let fade = Rect::from_min_max(
                pos2(visible.right() - 56.0, scroll.inner_rect.top()),
                pos2(visible.right(), scroll.inner_rect.bottom()),
            );
            let mut mesh = egui::Mesh::default();
            let clear = palette.window.gamma_multiply(0.0);
            let solid_x = fade.left() + fade.width() * 0.4;
            mesh.colored_vertex(fade.left_top(), clear);
            mesh.colored_vertex(pos2(solid_x, fade.top()), palette.window);
            mesh.colored_vertex(pos2(solid_x, fade.bottom()), palette.window);
            mesh.colored_vertex(fade.left_bottom(), clear);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            painter.add(mesh);
            painter.rect_filled(
                Rect::from_min_max(pos2(solid_x, fade.top()), fade.max),
                CornerRadius::ZERO,
                palette.window,
            );
            painter.rect_filled(pill, CornerRadius::same(14), palette.text);
            paint(
                painter,
                ui,
                role,
                &text,
                palette.window,
                pill.left() + 8.0,
                pill.center().y,
                false,
                look,
            );
            Icon::ChevronRight.image(palette.window, 10.0).paint_at(
                ui,
                Rect::from_center_size(
                    pos2(pill.right() - 8.0 - 5.0, pill.center().y),
                    vec2(10.0, 10.0),
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
    // The name over its type line, the pair centred above the rule.
    let name_role = TextRole::pick(look, TextRole::UiBodySemibold, TextRole::OGroup);
    let type_role = TextRole::pick(look, TextRole::ColumnType, TextRole::OColumnType);
    let (name_line, type_line) = (
        name_role.row_height(ui.ctx(), look.faces),
        type_role.row_height(ui.ctx(), look.faces),
    );
    let top = rect.top() + (rect.height() - 1.0 - name_line - type_line) / 2.0;
    let (name_y, type_y) = (top + name_line / 2.0, top + name_line + type_line / 2.0);
    // Sorted columns name themselves in the accent; the terminal marks the
    // key in yellow.
    let name_color = if column.sort.is_some() {
        if look.terminal {
            palette.accent
        } else {
            palette.accent_hover
        }
    } else if column.key && look.terminal {
        palette.warning
    } else {
        palette.text
    };
    // The terminal writes the sort's arrow after the name.
    let arrow_text = match (column.sort, look.terminal) {
        (Some(SortDir::Asc), true) => format!("{} ↑", column.name),
        (Some(SortDir::Desc), true) => format!("{} ↓", column.name),
        _ => column.name.to_owned(),
    };
    let name_width = text_width(ui, &arrow_text, name_role, look);
    let type_width = text_width(ui, &column.type_line, type_role, look);
    // macOS: the key icon 4 before the name, the arrow 4 after it.
    let arrow = if column.sort.is_some() && !look.terminal {
        15.0
    } else {
        0.0
    };
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
    paint(
        &clip,
        ui,
        name_role,
        &arrow_text,
        name_color,
        name_x,
        name_y,
        false,
        look,
    );
    if let Some(dir) = column.sort.filter(|_| !look.terminal) {
        let icon = if dir == SortDir::Asc {
            Icon::ArrowUp
        } else {
            Icon::ArrowDown
        };
        icon.image(name_color, 11.0).paint_at(
            ui,
            Rect::from_center_size(pos2(name_x + name_width + 9.5, name_y), vec2(11.0, 11.0)),
        );
    }
    paint(
        &clip,
        ui,
        type_role,
        &column.type_line,
        palette.dim,
        type_x,
        type_y,
        false,
        look,
    );
}

/// One cell: its text, tag, JSON chip or colour swatch, cut to fit.
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
    let role = data_role(look);
    let width = |text: &str, role: TextRole| text_width(ui, text, role, look);
    let center = rect.center().y;
    let room = rect.width() - 2.0 * pad;
    if content.null {
        paint(
            &clip,
            ui,
            role,
            "NULL",
            palette.faint,
            rect.left() + pad,
            center,
            false,
            look,
        );
        return;
    }
    match content.style {
        Style::Tag(hue) => {
            let (color, fill) = tag_colors(hue, look, palette);
            // macOS: a chip in Plex Mono 11.5, 2 above and below, 6 at the
            // sides. Terminal: the text alone, in the tag's colour.
            let tag_role = if look.terminal {
                role
            } else {
                TextRole::ValueTag
            };
            let shown = ellipsize(&content.text, room - 12.0, false, |text| {
                width(text, tag_role)
            });
            let text_width = width(&shown, tag_role);
            if let Some(fill) = fill {
                let height = tag_role.row_height(ui.ctx(), look.faces) + 4.0;
                let chip = Rect::from_min_size(
                    pos2(rect.left() + pad, center - height / 2.0),
                    vec2(text_width + 12.0, height),
                );
                clip.rect_filled(chip, CornerRadius::same(4), fill);
                paint(
                    &clip,
                    ui,
                    tag_role,
                    &shown,
                    color,
                    chip.left() + 6.0,
                    center,
                    false,
                    look,
                );
            } else {
                paint(
                    &clip,
                    ui,
                    role,
                    &shown,
                    color,
                    rect.left() + pad,
                    center,
                    false,
                    look,
                );
            }
        }
        Style::Json(count) => {
            let mut left = rect.left() + pad;
            if look.terminal {
                let mark = "{…}";
                left += width("{…} ", role);
                paint(
                    &clip,
                    ui,
                    role,
                    mark,
                    palette.text,
                    rect.left() + pad,
                    center,
                    false,
                    look,
                );
            } else {
                // Plex Mono 11, 1 above and below, 5 at the sides.
                let chip_role = TextRole::JsonChip;
                let label = format!("{{ {count} }}");
                let width = width(&label, chip_role) + 10.0 + 2.0;
                let height = chip_role.row_height(ui.ctx(), look.faces) + 2.0 + 2.0;
                let chip =
                    Rect::from_min_size(pos2(left, center - height / 2.0), vec2(width, height));
                clip.rect_stroke(
                    chip,
                    CornerRadius::same(4),
                    Stroke::new(crate::ui::widgets::hairline(ui), palette.border),
                    StrokeKind::Inside,
                );
                let laid = Text::one(look, chip_role, &label, palette.dim).layout(ui.ctx());
                laid.paint(&clip, chip.center() - laid.size() / 2.0);
                left = chip.right() + 8.0;
            }
            let room = rect.right() - pad - left;
            let shown = match content.text.split_once(" · ") {
                // The first string keeps both ends in 70% of the room;
                // what follows fills the rest from its start.
                Some((first, rest)) => {
                    let first = crate::ui::format::ellipsize_middle(first, room * 0.7, |text| {
                        width(text, role)
                    });
                    let joined = format!("{first} · {rest}");
                    ellipsize(&joined, room, false, |text| width(text, role)).into_owned()
                }
                None => crate::ui::format::ellipsize_middle(&content.text, room, |text| {
                    width(text, role)
                }),
            };
            let color = if look.terminal {
                palette.dim
            } else {
                palette.secondary
            };
            paint(&clip, ui, role, &shown, color, left, center, false, look);
        }
        Style::Color(color) => {
            let left = rect.left() + pad;
            paint_swatch(
                &clip,
                ui,
                pos2(left + SWATCH / 2.0, center),
                SWATCH,
                color,
                look,
                palette,
            );
            let left = left + SWATCH + SWATCH_GAP;
            let shown = ellipsize(&content.text, rect.right() - pad - left, false, |text| {
                width(text, role)
            });
            paint(
                &clip,
                ui,
                role,
                &shown,
                palette.text,
                left,
                center,
                false,
                look,
            );
        }
        Style::Plain => {
            let numeric = column.numeric;
            let x = if numeric {
                rect.right() - pad
            } else {
                rect.left() + pad
            };
            let shown = ellipsize(&content.text, room, numeric, |text| width(text, role));
            // A key's values read a step quieter than the data.
            let color = match (column.key, look.terminal) {
                (true, true) => palette.dim,
                (true, false) => palette.secondary,
                _ => palette.text,
            };
            paint(&clip, ui, role, &shown, color, x, center, numeric, look);
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
                flexible: false,
            },
            Column {
                name: "email",
                type_line: "TEXT".into(),
                numeric: false,
                sort: Some(SortDir::Asc),
                key: false,
                flexible: false,
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
        let widths = initial_widths(&columns, 3, (6.0, 0.0), &width, &mut |_, col| Cell {
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
        let widths = initial_widths(&columns, 3, (6.0, 0.0), &width, &mut |_, col| Cell {
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
