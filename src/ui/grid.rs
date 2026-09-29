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

pub const ROW_HEIGHT: f32 = 24.0;
pub const HEADER_HEIGHT: f32 = 38.0;
const NUMBER_WIDTH: f32 = 52.0;
const MIN_WIDTH: f32 = 48.0;
const MAX_INITIAL_WIDTH: f32 = 320.0;
/// Space between a cell's edge and its text.
const CELL_PAD: f32 = 6.0;
const HANDLE_WIDTH: f32 = 6.0;
const SAMPLE_ROWS: usize = 50;

pub struct Column<'a> {
    pub name: &'a str,
    pub type_name: &'a str,
    pub numeric: bool,
    pub sort: Option<SortDir>,
}

pub struct Cell<'a> {
    pub text: Cow<'a, str>,
    pub null: bool,
}

#[derive(Debug, Default, PartialEq)]
pub struct GridOutput {
    pub clicked: Option<CellPos>,
    pub sort_clicked: Option<usize>,
}

/// The column under `x`, measured from the first data column's left edge.
/// How wide `text` is in `font`, in points (laid out once, then cached).
fn text_width(ui: &Ui, text: &str, font: &egui::FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
        .size()
        .x
}

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

/// A data row's background: the selected row's tint wins, then the hover
/// tint, then the zebra stripe on odd rows. `None` leaves the window colour.
pub fn row_fill(
    selected: bool,
    hovered: bool,
    odd: bool,
    palette: &Palette,
) -> Option<egui::Color32> {
    if selected {
        Some(palette.accent.gamma_multiply(0.14))
    } else if hovered {
        Some(palette.surface_hover.gamma_multiply(0.6))
    } else if odd {
        Some(palette.surface.gamma_multiply(0.45))
    } else {
        None
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
    width: &impl Fn(&str) -> f32,
    cell: &mut impl FnMut(usize, usize) -> Cell<'a>,
) -> Vec<f32> {
    columns
        .iter()
        .enumerate()
        .map(|(col, column)| {
            // Room for the sort chevron after the name.
            let header = width(column.name).max(width(column.type_name)) + 2.0 * 8.0 + 14.0;
            let widest = (0..rows.min(SAMPLE_ROWS))
                .map(|row| width(&cell(row, col).text))
                .fold(0.0, f32::max)
                + 2.0 * CELL_PAD;
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
    let widths_id = id.with(("widths", columns.len()));
    let last_id = id.with("last-selection");
    let mut widths: Vec<f32> = ui
        .data(|data| data.get_temp::<Vec<f32>>(widths_id))
        .filter(|widths| widths.len() == columns.len())
        .unwrap_or_else(|| {
            let font = theme::data(look);
            let width = |text: &str| text_width(ui, text, &font);
            initial_widths(columns, row_count, &width, &mut cell)
        });
    let last: Option<CellPos> = ui
        .data(|data| data.get_temp::<Option<CellPos>>(last_id))
        .flatten();
    let reveal = selection.filter(|cell| Some(*cell) != last);
    let total = NUMBER_WIDTH + widths.iter().sum::<f32>();

    egui::ScrollArea::both()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let origin = ui.cursor().min;
            ui.allocate_space(vec2(total, HEADER_HEIGHT));

            if let Some(target) = reveal {
                let col = target.col.min(widths.len().saturating_sub(1));
                let x = origin.x + NUMBER_WIDTH + widths[..col].iter().sum::<f32>();
                let y = origin.y + HEADER_HEIGHT + target.row as f32 * ROW_HEIGHT;
                // Include the header's height above the row so the sticky
                // header never covers it.
                let rect = Rect::from_min_size(
                    pos2(x - NUMBER_WIDTH, y - HEADER_HEIGHT),
                    vec2(
                        widths.get(col).copied().unwrap_or(0.0) + NUMBER_WIDTH,
                        ROW_HEIGHT + HEADER_HEIGHT,
                    ),
                );
                ui.scroll_to_rect(rect, None);
            }

            virtual_rows(ui, row_count, ROW_HEIGHT, |ui, row| {
                let (rect, response) =
                    ui.allocate_exact_size(vec2(total, ROW_HEIGHT), Sense::click());
                let number = first_row_number + row as u64 + 1;
                let label = format!("Row {number}");
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
                if response.clicked() {
                    let col = response
                        .interact_pointer_pos()
                        .map(|pointer| column_at(&widths, pointer.x - rect.left() - NUMBER_WIDTH))
                        .unwrap_or(0);
                    output.clicked = Some(CellPos { row, col });
                }
                let painter = ui.painter().clone();
                let selected_row = selection.is_some_and(|cell| cell.row == row);
                if let Some(fill) =
                    row_fill(selected_row, response.hovered(), row % 2 == 1, palette)
                {
                    painter.rect_filled(rect, CornerRadius::ZERO, fill);
                }
                painter.text(
                    pos2(rect.left() + NUMBER_WIDTH - 8.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    number.to_string(),
                    theme::regular(theme::TEXT_SMALL),
                    palette.dim,
                );
                let divider = Stroke::new(1.0, palette.outline.gamma_multiply(0.6));
                let mut x = rect.left() + NUMBER_WIDTH;
                for (col, width) in widths.iter().enumerate() {
                    let cell_rect =
                        Rect::from_min_size(pos2(x, rect.top()), vec2(*width, ROW_HEIGHT));
                    x += width;
                    if !ui.is_rect_visible(cell_rect) {
                        continue;
                    }
                    let content = cell(row, col);
                    let color = if content.null {
                        palette.dim
                    } else {
                        palette.text
                    };
                    let clip = painter.with_clip_rect(
                        cell_rect
                            .shrink2(vec2(CELL_PAD, 0.0))
                            .intersect(ui.clip_rect()),
                    );
                    let numeric = columns[col].numeric && !content.null;
                    let (anchor, at) = if numeric {
                        (
                            Align2::RIGHT_CENTER,
                            pos2(cell_rect.right() - CELL_PAD, cell_rect.center().y),
                        )
                    } else {
                        (
                            Align2::LEFT_CENTER,
                            pos2(cell_rect.left() + CELL_PAD, cell_rect.center().y),
                        )
                    };
                    let font = theme::data(look);
                    let room = cell_rect.width() - 2.0 * CELL_PAD;
                    let shown = ellipsize(&content.text, room, numeric, |text| {
                        text_width(ui, text, &font)
                    });
                    clip.text(at, anchor, shown, font, color);
                    painter.vline(cell_rect.right(), rect.y_range(), divider);
                    if selection == Some(CellPos { row, col }) {
                        painter.rect_stroke(
                            cell_rect.shrink(1.0),
                            // Square in the Omarchy look.
                            CornerRadius::same(look.radius.min(2)),
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
            let header = Rect::from_min_size(pos2(origin.x, top), vec2(total, HEADER_HEIGHT));
            let painter = ui.painter().clone();
            painter.rect_filled(header, CornerRadius::ZERO, palette.panel);
            painter.hline(
                header.x_range(),
                header.bottom(),
                Stroke::new(1.0, palette.outline),
            );
            let mut x = origin.x + NUMBER_WIDTH;
            for (col, column) in columns.iter().enumerate() {
                let rect = Rect::from_min_size(pos2(x, top), vec2(widths[col], HEADER_HEIGHT));
                let response = ui.interact(rect, id.with(("header", col)), Sense::click());
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, column.name));
                if response.clicked() {
                    output.sort_clicked = Some(col);
                }
                if response.hovered() {
                    painter.rect_filled(rect, CornerRadius::ZERO, palette.surface_hover);
                }
                let text = painter.with_clip_rect(rect.shrink2(vec2(8.0, 0.0)));
                text.text(
                    pos2(rect.left() + 8.0, rect.top() + 12.0),
                    Align2::LEFT_CENTER,
                    column.name,
                    theme::medium(theme::TEXT),
                    palette.text,
                );
                text.text(
                    pos2(rect.left() + 8.0, rect.top() + 27.0),
                    Align2::LEFT_CENTER,
                    column.type_name,
                    theme::regular(theme::TEXT_SMALL),
                    palette.dim,
                );
                if let Some(dir) = column.sort {
                    let icon = if dir == SortDir::Asc {
                        Icon::ChevronUp
                    } else {
                        Icon::ChevronDown
                    };
                    icon.image(palette.accent, 12.0).paint_at(
                        ui,
                        Rect::from_center_size(
                            pos2(rect.right() - 12.0, rect.top() + 12.0),
                            vec2(12.0, 12.0),
                        ),
                    );
                }
                let handle = Rect::from_min_max(
                    pos2(rect.right() - HANDLE_WIDTH / 2.0, top),
                    pos2(rect.right() + HANDLE_WIDTH / 2.0, top + HEADER_HEIGHT),
                );
                let drag = ui.interact(handle, id.with(("resize", col)), Sense::drag());
                if drag.hovered() || drag.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
                }
                if drag.dragged() {
                    widths[col] = (widths[col] + drag.drag_delta().x).max(MIN_WIDTH);
                }
                painter.vline(
                    rect.right(),
                    header.y_range(),
                    Stroke::new(1.0, palette.outline),
                );
                x += widths[col];
            }
        });

    ui.data_mut(|data| {
        data.insert_temp(widths_id, widths);
        data.insert_temp(last_id, selection);
    });
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Role;

    #[test]
    fn selection_wins_over_hover_and_hover_over_stripes() {
        let palette = Palette::light();
        let selected = Some(palette.accent.gamma_multiply(0.14));
        let hovered = Some(palette.surface_hover.gamma_multiply(0.6));
        let stripe = Some(palette.surface.gamma_multiply(0.45));
        assert_eq!(row_fill(true, true, true, &palette), selected);
        assert_eq!(row_fill(true, false, false, &palette), selected);
        assert_eq!(row_fill(false, true, true, &palette), hovered);
        assert_eq!(row_fill(false, true, false, &palette), hovered);
        assert_eq!(row_fill(false, false, true, &palette), stripe);
        assert_eq!(row_fill(false, false, false, &palette), None);
    }

    fn columns() -> Vec<Column<'static>> {
        vec![
            Column {
                name: "id",
                type_name: "INTEGER",
                numeric: true,
                sort: None,
            },
            Column {
                name: "email",
                type_name: "TEXT",
                numeric: false,
                sort: Some(SortDir::Asc),
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
        let widths = initial_widths(&columns, 3, &width, &mut |_, col| Cell {
            text: if col == 0 {
                "1628910071209526786".into()
            } else {
                "x".into()
            },
            null: false,
        });
        // Nineteen digits at eight points, plus padding: nothing clipped.
        assert!(widths[0] >= 19.0 * 8.0 + 12.0, "{widths:?}");
    }

    #[test]
    fn initial_widths_fit_content_within_limits() {
        let columns = columns();
        let widths = initial_widths(&columns, 3, &width, &mut |_, col| Cell {
            text: if col == 0 {
                "1".into()
            } else {
                "x".repeat(500).into()
            },
            null: false,
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
