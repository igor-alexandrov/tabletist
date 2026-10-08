//! The data grid: a sticky header with resizable columns over virtualized
//! rows, one selected cell. Reads data through a closure and reports clicks;
//! it never changes application state.

use std::borrow::Cow;

use egui::{
    CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2,
};
use tabletist_db::SortDir;

use crate::edit::RowMark;
use crate::model::CellPos;
use crate::theme::{DataFont, Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus;
use crate::ui::format::{Marks, array_items, display_safe};
use crate::ui::states::Tone;
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
pub(crate) fn cell_pad(look: &crate::theme::Look) -> f32 {
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
    /// A click on its header sorts by it. A header that sorts nothing (a
    /// SQL result's) is a label: no button, no stop for the Tab key, no
    /// fill under the pointer.
    pub sortable: bool,
    /// A new row needs a value in it: the terminal's header says so.
    pub required: bool,
}

/// A row as the grid needs it besides its cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Row {
    pub mark: RowMark,
    /// Its number among the table's rows, from 1, for a screen reader.
    /// `None` for a row the table does not hold yet.
    pub number: Option<u64>,
}

/// How a cell draws its text.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Style {
    #[default]
    Plain,
    /// A value from a column's allowed list, in that palette slot (see
    /// [`crate::ui::value_tags`]).
    Tag(usize),
    /// A JSON document with this many keys: a `{ n }` chip, then the text.
    Json(usize),
    /// A colour (`#3a7bd5`): a swatch of it, then the text.
    Color(egui::Color32),
    /// Text that stands for what the cell holds rather than being it (`''`
    /// for an empty string, a mark for each space of a blank one, `{}` for
    /// an empty array): faint.
    Quiet,
    /// What is known of a value the cell does not show (a binary value's
    /// type and size): one outlined chip.
    Chip,
    /// A PostgreSQL array, its text as the database writes it: a chip for
    /// each element that fits, then `+n` for the rest.
    Array,
}

/// What a cell's pending state is, for how it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mark {
    #[default]
    None,
    Pending,
    /// To fix, or failed in the last save.
    Trouble,
    /// Pending and being saved.
    Saving,
    /// Just written.
    Saved,
    /// A cell of a computed column, in a table that can be edited.
    Locked,
    /// A cell of a row a save found gone from the server.
    Gone,
    /// The cell that marks a new row: its text in the row's tone.
    Added,
    /// A cell of a new row nothing is set in: what the database will fill
    /// it with, quieter than a value.
    Unset,
}

#[derive(Default)]
pub struct Cell<'a> {
    pub text: Cow<'a, str>,
    pub null: bool,
    pub style: Style,
    pub mark: Mark,
    /// What the cell says under the pointer: what a pending cell was, why
    /// a failed one failed. Never why a cell is locked, which is said only
    /// when asked.
    pub hint: Option<String>,
    /// What the cell says without the pointer on it: why it cannot be
    /// edited, once that was asked for.
    pub note: Option<String>,
}

/// What draws the field of a cell being edited, given the cell's place:
/// the view's, which holds the text. The grid only says where.
pub type Editor<'a> = &'a mut dyn FnMut(&mut Ui, Rect);

#[derive(Debug, Default, PartialEq)]
pub struct GridOutput {
    pub clicked: Option<CellPos>,
    /// The cell a second click landed on, soon after the first.
    pub double_clicked: Option<CellPos>,
    /// Where the cell being edited is, in view or not: what an editor that
    /// does not sit on the cell is anchored to.
    pub editing_rect: Option<Rect>,
    pub sort_clicked: Option<usize>,
    /// The keyboard came to the grid this frame (the Tab key, a screen
    /// reader): the arrows should be the grid's.
    pub focused: bool,
}

/// Which of a grid's columns were in view when it was last drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnsShown {
    /// The first and the last column in view, counted from 0. A pinned
    /// column counts among them while the column after it is in view; past
    /// that it is in sight but not of the range.
    pub first: usize,
    pub last: usize,
    pub total: usize,
    /// The grid's first column stays in sight while the others scroll.
    pub pinned: bool,
}

impl ColumnsShown {
    /// Whether some columns are out of view.
    pub fn partial(&self) -> bool {
        let apart = usize::from(self.pinned && self.first > 0);
        self.last + 1 - self.first + apart < self.total
    }
}

/// The columns the grid `id` showed when it was last drawn: what a status
/// line says of them, a frame later.
pub fn columns_shown(ctx: &egui::Context, id: Id) -> Option<ColumnsShown> {
    ctx.data(|data| data.get_temp(id.with("columns-shown")))
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

/// The marks `look`'s data face writes where text would show nothing: the
/// design's when the fonts at hand have them (see [`Marks::pick`]). Found
/// once for each set of faces, not for every cell.
pub fn marks(ctx: &egui::Context, look: &Look) -> Marks {
    let id = Id::new(("cell-marks", look.faces));
    if let Some(marks) = ctx.data(|data| data.get_temp::<Marks>(id)) {
        return marks;
    }
    let font = data_role(look).font_id(look.faces);
    let marks = ctx.fonts_mut(|fonts| Marks::pick(|character| fonts.has_glyph(&font, character)));
    ctx.data_mut(|data| data.insert_temp(id, marks));
    marks
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

/// `palette` with every colour a cell writes in set to `color`.
fn written_in(palette: &Palette, color: egui::Color32) -> Palette {
    Palette {
        text: color,
        secondary: color,
        dim: color,
        faint: color,
        ..*palette
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
                width(&display_safe(column.name)).max(width(&display_safe(&column.type_line)))
                    + 2.0 * pad
                    + 16.0
            };
            let widest = (0..rows.min(SAMPLE_ROWS))
                .map(|row| {
                    let cell = cell(row, col);
                    let chip = match cell.style {
                        Style::Plain | Style::Quiet => 0.0,
                        Style::Tag(_) => 16.0,
                        Style::Chip => 2.0 * CHIP_PAD + 2.0,
                        Style::Json(_) => 44.0,
                        Style::Color(_) => SWATCH + SWATCH_GAP,
                        // Each element's chip adds its sides and the gap
                        // to the next.
                        Style::Array => {
                            let elements =
                                array_items(&cell.text).map_or(0, |array| array.items.len());
                            (2.0 * CHIP_PAD + CHIP_GAP) * elements as f32
                        }
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

/// What a grid keeps in egui's memory under ids only it can tell, kept
/// under the grid's own id so [`forget`] finds all of it.
#[derive(Clone, Copy)]
struct Kept {
    /// How many columns the widths kept are for.
    columns: usize,
    scroll: Id,
}

/// Drops what egui's memory keeps for the grid `id`: its columns' widths,
/// the selection it last revealed and where it was scrolled to. For a grid
/// that will not be drawn again (a SQL editor's result, once the next run
/// has its own grid).
pub fn forget(ctx: &egui::Context, id: Id) {
    ctx.data_mut(|data| {
        if let Some(Kept { columns, scroll }) = data.get_temp(id) {
            data.remove::<Vec<f32>>(id.with(("widths", columns)));
            data.remove::<egui::scroll_area::State>(scroll);
        }
        data.remove::<Option<CellPos>>(id.with("last-selection"));
        data.remove::<ColumnsShown>(id.with("columns-shown"));
        data.remove::<Kept>(id);
    });
}

/// The grid a view was last drawn with, kept under a key of the view's.
#[derive(Clone, Copy, PartialEq)]
pub struct Last(pub Id);

/// Notes that the view kept under `key` now draws the grid `grid`, and
/// drops what egui kept for the grid before it. For a view whose grid
/// changes its id: an id come back to would bring the widths that fitted
/// the rows it last drew, not the ones now on screen.
pub fn keep(ctx: &egui::Context, key: Id, grid: Id) {
    let before: Option<Last> = ctx.data(|data| data.get_temp(key));
    if before == Some(Last(grid)) {
        return;
    }
    if let Some(Last(old)) = before {
        forget(ctx, old);
    }
    ctx.data_mut(|data| data.insert_temp(key, Last(grid)));
}

/// Whether egui's memory keeps anything for the grid `id`.
#[cfg(test)]
pub fn remembered(ctx: &egui::Context, id: Id) -> bool {
    ctx.data(|data| {
        data.get_temp::<Kept>(id).is_some()
            || data
                .get_temp::<Option<CellPos>>(id.with("last-selection"))
                .is_some()
    })
}

#[allow(clippy::too_many_arguments)] // one call site per view; a struct adds nothing
pub fn show<'a>(
    ui: &mut Ui,
    id: Id,
    columns: &[Column<'_>],
    row_count: usize,
    selection: Option<CellPos>,
    // Whether the arrow keys move in this grid.
    keys: bool,
    palette: &Palette,
    look: &crate::theme::Look,
    // Each row's number, and what its pending cells come to, for its mark.
    rows: &dyn Fn(usize) -> Row,
    // The cell an editor is open on, and what draws its field on the cell.
    editing: Option<CellPos>,
    mut editor: Option<Editor<'_>>,
    mut cell: impl FnMut(usize, usize) -> Cell<'a>,
) -> GridOutput {
    let mut output = GridOutput::default();
    let row_height = look.grid_row;
    let header_height = header_height(look);
    let pad = cell_pad(look);
    let gutter = if look.terminal { GUTTER } else { 0.0 };
    let widths_id = id.with(("widths", columns.len()));
    let last_id = id.with("last-selection");
    let kept: Option<Vec<f32>> = ui
        .data(|data| data.get_temp::<Vec<f32>>(widths_id))
        .filter(|widths| widths.len() == columns.len());
    // Widths measured with no rows fit the headers alone. They are not
    // kept, so the first page with rows sizes the columns; a width the
    // user drags is.
    let mut keep = kept.is_some() || row_count > 0;
    let mut widths: Vec<f32> = kept.unwrap_or_else(|| {
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
    // A cell being edited stays in view, as a selection that just moved
    // comes into it: egui takes the keyboard from a field that is not
    // drawn.
    let reveal = editing.or(selection.filter(|cell| Some(*cell) != last));
    let total = gutter + widths.iter().sum::<f32>();
    let hairline = crate::ui::widgets::hairline(ui);
    let visible = ui.max_rect();
    // A key column that leads the grid stays in sight: the others scroll
    // under it, so a row is never read without knowing whose it is.
    let pinned = columns.len() > 1 && columns[0].key;
    // Drawn last, over what scrolled under it; the others in their order.
    let order: Vec<usize> = (usize::from(pinned)..columns.len())
        .chain(pinned.then_some(0))
        .collect();
    // Each column's left edge, from the first one's: found once a frame,
    // not once a cell. A width dragged this frame moves its neighbours the
    // next.
    let lefts: Vec<f32> = widths
        .iter()
        .scan(0.0, |edge, width| {
            let left = *edge;
            *edge += width;
            Some(left)
        })
        .collect();
    // The grid is one Tab stop, not one for each row: with the keyboard on
    // it the arrows move the selected cell, which shows where they are.
    let stop = ui.interact(visible, id.with("keys"), Sense::focusable_noninteractive());
    stop.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, "Rows"));
    ui.ctx().accesskit_node_builder(stop.id, |node| {
        node.set_role(egui::accesskit::Role::Group);
    });
    focus::pane(ui, &stop);
    // The selected cell shows where the keyboard is; with none selected
    // yet the grid itself does, until an arrow picks one.
    let ring = if selection.is_some() {
        focus::Ring::Own
    } else {
        focus::Ring::Inset { radius: 0 }
    };
    focus::hint(ui, &stop, visible, ring);
    focus::region(ui, focus::Region::Grid, visible);
    focus::claim(ui, focus::Region::Grid, &stop);
    output.focused = stop.gained_focus();
    // The cell is lit while the keyboard is in use and its keys come here:
    // not while a button or a field has them. The field of a cell being
    // edited is the grid's own: the pane has not lost the keyboard to it.
    let lit =
        keys && focus::visible(ui.ctx()) && (editing.is_some() || !focus::on_control(ui.ctx()));

    let scroll = egui::ScrollArea::both()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let origin = ui.cursor().min;
            let full = total.max(ui.available_width());
            ui.allocate_space(vec2(total, header_height));

            // How far the pinned column stands off its place: as far as
            // the grid is scrolled sideways.
            let shift = if pinned {
                // Under a point it is the clip's own margin, not a scroll.
                Some(ui.clip_rect().left() - origin.x)
                    .filter(|shift| *shift >= 1.0)
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            if let Some(target) = reveal {
                let col = target.col.min(widths.len().saturating_sub(1));
                let x = origin.x + gutter + widths[..col].iter().sum::<f32>();
                let y = origin.y + header_height + target.row as f32 * row_height;
                // Include the header's height above the row so the sticky
                // header never covers it, and the pinned column's width
                // before a cell that scrolls so that never does either.
                let cover = if pinned && col > 0 {
                    gutter + widths[0]
                } else {
                    0.0
                };
                let rect = Rect::from_min_size(
                    pos2(x - cover, y - header_height),
                    vec2(
                        widths.get(col).copied().unwrap_or(0.0) + cover,
                        row_height + header_height,
                    ),
                );
                ui.scroll_to_rect(rect, None);
            }

            virtual_rows(ui, row_count, row_height, |ui, row| {
                // A row takes a click, not the Tab key: the grid is the stop.
                let (rect, response) = ui.allocate_exact_size(vec2(full, row_height), Sense::CLICK);
                let Row { mark, number } = rows(row);
                let label = match number {
                    Some(number) => format!("Row {number}"),
                    None => "New row, not saved".to_owned(),
                };
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
                if response.clicked() || response.double_clicked() {
                    let col = response
                        .interact_pointer_pos()
                        .map(|pointer| {
                            let x = pointer.x - rect.left() - gutter;
                            // Over the pinned column, whatever is under it.
                            if pinned && x - shift < widths[0] {
                                0
                            } else {
                                column_at(&widths, x)
                            }
                        })
                        .unwrap_or(0);
                    let cell = Some(CellPos { row, col });
                    if response.clicked() {
                        output.clicked = cell;
                    }
                    if response.double_clicked() {
                        output.double_clicked = cell;
                    }
                }
                let painter = ui.painter().clone();
                // A row with pending cells is marked in their tone.
                let row_tone = match mark {
                    RowMark::None => None,
                    RowMark::Changed => Some(Tone::Warning),
                    RowMark::Trouble => Some(Tone::Danger),
                    RowMark::New => Some(Tone::Success),
                };
                let new = mark == RowMark::New;
                let selected_row = selection.is_some_and(|cell| cell.row == row);
                // With the keyboard in the grid the cell takes the
                // selection's colour and its row a lighter tint of it.
                let lit_row = selected_row && lit && !look.terminal;
                let fill = if lit_row {
                    Some(palette.window.lerp_to_gamma(palette.selection, 0.6))
                } else if new && !selected_row {
                    // A new row is filled in its tone. Selected, it takes
                    // the selection's colour, which says where the keyboard
                    // is: its bar and its gutter still say it is new.
                    Some(Tone::Success.fill(look, palette))
                } else {
                    row_fill(
                        selected_row,
                        response.hovered(),
                        row % 2 == 1,
                        look,
                        palette,
                    )
                };
                if let Some(fill) = fill {
                    painter.rect_filled(rect, CornerRadius::ZERO, fill);
                }
                if !look.terminal {
                    let y = painter.round_to_pixel_center(rect.bottom() - hairline / 2.0);
                    painter.hline(rect.x_range(), y, Stroke::new(hairline, palette.surface));
                }
                for &col in &order {
                    let left = rect.left() + gutter + lefts[col];
                    let cell_rect =
                        Rect::from_min_size(pos2(left, rect.top()), vec2(widths[col], row_height));
                    // The first column, and the row's mark before it, stand
                    // where the view begins while it is pinned.
                    let cell_rect = if col == 0 {
                        let lead = Rect::from_min_max(
                            pos2(rect.left() + shift, rect.top()),
                            pos2(cell_rect.right() + shift, rect.bottom()),
                        );
                        if shift > 0.0 {
                            // Over what scrolled under: the row's own fill.
                            let under = fill.unwrap_or(palette.window);
                            painter.rect_filled(lead, CornerRadius::ZERO, under);
                            if !look.terminal {
                                let y =
                                    painter.round_to_pixel_center(rect.bottom() - hairline / 2.0);
                                painter.hline(
                                    lead.x_range(),
                                    y,
                                    Stroke::new(hairline, palette.surface),
                                );
                            }
                            painter.vline(
                                lead.right() - hairline / 2.0,
                                lead.y_range(),
                                Stroke::new(hairline, palette.outline),
                            );
                        }
                        if look.terminal {
                            let line = rect.center().y;
                            if selected_row {
                                // The cursor: a bold accent block in the
                                // gutter.
                                let cursor = Text::one(look, TextRole::OGroup, "▌", palette.accent)
                                    .layout(ui.ctx());
                                cursor
                                    .paint_center(&painter, pos2(lead.left() + GUTTER / 2.0, line));
                            }
                            if let Some(tone) = row_tone {
                                // Beside the cursor: `~` on a changed row,
                                // `!` when one of its cells is in trouble,
                                // `+` on a new one.
                                let sign = match mark {
                                    RowMark::Trouble => "!",
                                    RowMark::New => "+",
                                    RowMark::None | RowMark::Changed => "~",
                                };
                                Text::one(look, TextRole::OGroup, sign, tone.color(palette))
                                    .layout(ui.ctx())
                                    .paint_center(
                                        &painter,
                                        pos2(lead.left() + GUTTER * 0.75, line),
                                    );
                            }
                        }
                        cell_rect.translate(vec2(shift, 0.0))
                    } else {
                        cell_rect
                    };
                    let edited = editing == Some(CellPos { row, col });
                    if edited {
                        output.editing_rect = Some(cell_rect);
                    }
                    if edited && let Some(editor) = editor.as_deref_mut() {
                        // The field in place of the cell's text, in view
                        // or not: it is drawn as long as it is open.
                        editor(ui, cell_rect);
                        continue;
                    }
                    if !ui.is_rect_visible(cell_rect) {
                        continue;
                    }
                    let content = cell(row, col);
                    let here = selection == Some(CellPos { row, col });
                    if content.hint.is_some() || content.note.is_some() {
                        // Where the cell shows: not the part of it that
                        // scrolled under the pinned column.
                        let mut seen = cell_rect;
                        if pinned && col > 0 {
                            let lead = rect.left() + shift + gutter + widths[0];
                            seen.min.x = seen.min.x.max(lead);
                        }
                        if seen.is_positive() {
                            // Hovered and no more: a click is the row's.
                            let at = ui.interact(seen, id.with(("hint", row, col)), Sense::hover());
                            if let Some(note) = &content.note {
                                // It was asked for: said without waiting
                                // for the pointer.
                                at.show_tooltip_text(note.as_str());
                            } else if let Some(hint) = &content.hint {
                                let _ = at.on_hover_text(hint.as_str());
                            }
                        }
                    }
                    // What the cell's pending state tints it with. The
                    // terminal draws a computed column as any other. A row
                    // that is gone has no tint: nothing of it is pending.
                    let tone = match content.mark {
                        Mark::Pending | Mark::Saving => Some(Tone::Warning),
                        Mark::Trouble => Some(Tone::Danger),
                        Mark::Saved => Some(Tone::Success),
                        Mark::None | Mark::Locked | Mark::Gone | Mark::Added | Mark::Unset => None,
                    };
                    let locked = content.mark == Mark::Locked && !look.terminal;
                    if here && lit && look.terminal && tone.is_none() && !edited {
                        // Reverse video, as a terminal marks its cursor:
                        // the accent behind, the text in the window's tone.
                        painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.accent);
                        let reversed = written_in(palette, palette.window);
                        let plain = Cell {
                            style: Style::Plain,
                            ..content
                        };
                        draw_cell(
                            ui,
                            &painter,
                            cell_rect,
                            &columns[col],
                            &plain,
                            look,
                            &reversed,
                        );
                        continue;
                    }
                    // A tint stays under the cursor, which is then the
                    // accent line alone: what is pending reads as pending
                    // wherever the selection is.
                    if let Some(tone) = tone {
                        painter.rect_filled(
                            cell_rect,
                            CornerRadius::ZERO,
                            tone.fill(look, palette),
                        );
                    } else if here && lit {
                        painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.selection);
                    } else if locked {
                        painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.surface);
                    }
                    let height = cell_rect.height();
                    match content.mark {
                        Mark::Pending | Mark::Saving if !look.terminal => {
                            let bar = Rect::from_min_size(cell_rect.min, vec2(2.0, height));
                            painter.rect_filled(
                                bar,
                                CornerRadius::ZERO,
                                Tone::Warning.color(palette),
                            );
                        }
                        Mark::Trouble => {
                            painter.rect_stroke(
                                cell_rect,
                                CornerRadius::ZERO,
                                Stroke::new(1.0, Tone::Danger.color(palette)),
                                StrokeKind::Inside,
                            );
                        }
                        _ => {}
                    }
                    if col == 0 && !look.terminal {
                        // The row's bar, over what its first cell is filled
                        // with: of its pending cells' tone, else the
                        // accent of a selected row whose cell is not lit.
                        let color = match row_tone {
                            Some(tone) => Some(tone.color(palette)),
                            None => (selected_row && !lit_row).then_some(palette.accent),
                        };
                        if let Some(color) = color {
                            let bar = Rect::from_min_size(cell_rect.min, vec2(3.0, height));
                            painter.rect_filled(bar, CornerRadius::ZERO, color);
                        }
                    }
                    // The text's colours: dim in a row that is gone, a
                    // pending cell's tone in the terminal, a changed row's
                    // on its key, and a computed column a step quieter.
                    let written = match (tone, row_tone) {
                        _ if content.mark == Mark::Gone => written_in(palette, palette.dim),
                        // A new row's marker in the row's tone; the
                        // terminal writes it dimmed, as it does what a
                        // cell will be filled with.
                        _ if content.mark == Mark::Added && !look.terminal => {
                            written_in(palette, Tone::Success.color(palette))
                        }
                        _ if matches!(content.mark, Mark::Unset | Mark::Added) => {
                            written_in(palette, palette.dim)
                        }
                        (Some(tone), _) if look.terminal && tone != Tone::Success => {
                            written_in(palette, tone.color(palette))
                        }
                        (None, Some(tone)) if col == 0 && columns[col].key && !look.terminal => {
                            written_in(palette, tone.color(palette))
                        }
                        _ if locked => Palette {
                            text: palette.secondary,
                            ..*palette
                        },
                        _ => *palette,
                    };
                    // A cell being saved keeps room at its right for the
                    // spinner.
                    let text_rect = if content.mark == Mark::Saving {
                        let side = 12.0;
                        let at = Rect::from_center_size(
                            pos2(cell_rect.right() - pad - side / 2.0, cell_rect.center().y),
                            vec2(side, side),
                        );
                        crate::ui::states::spinner(ui, at, Tone::Warning.color(palette), palette);
                        // The text ends 6 short of it.
                        let right = at.left() - 6.0 + pad;
                        Rect::from_min_max(cell_rect.min, pos2(right, cell_rect.bottom()))
                    } else {
                        cell_rect
                    };
                    draw_cell(
                        ui,
                        &painter,
                        text_rect,
                        &columns[col],
                        &content,
                        look,
                        &written,
                    );
                    // A cell edited in an editor of its own keeps its
                    // value and the cursor's line, lit or not: the line
                    // says which cell the editor is for.
                    if (here && lit) || edited {
                        // Inside the cell: nothing the grid scrolls under
                        // cuts it.
                        painter.rect_stroke(
                            cell_rect,
                            CornerRadius::ZERO,
                            Stroke::new(2.0, palette.accent),
                            StrokeKind::Inside,
                        );
                    } else if here && col > 0 {
                        // The row is selected; a cell past the first is
                        // marked too, for the keys that act on one cell.
                        painter.rect_stroke(
                            cell_rect.shrink(1.0),
                            CornerRadius::same(look.radius.min(3)),
                            Stroke::new(1.5, palette.accent),
                            StrokeKind::Inside,
                        );
                    }
                }
            });

            // A row scrolled out of view is not built, and its cell's
            // editor must be drawn all the same to keep the keyboard: where
            // the cell is, until the grid has scrolled back to it.
            if let (Some(at), None) = (editing, output.editing_rect)
                && at.row < row_count
                && at.col < widths.len()
            {
                let place = pos2(
                    origin.x + gutter + lefts[at.col],
                    origin.y + header_height + at.row as f32 * row_height,
                );
                let rect = Rect::from_min_size(place, vec2(widths[at.col], row_height));
                output.editing_rect = Some(rect);
                if let Some(editor) = editor {
                    editor(ui, rect);
                }
            }

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
            for &col in &order {
                let column = &columns[col];
                let left = origin.x + gutter + lefts[col];
                let rect = Rect::from_min_size(pos2(left, top), vec2(widths[col], header_height));
                // The pinned column's header stands with its cells, over
                // the headers that scrolled under it.
                let rect = if col == 0 && shift > 0.0 {
                    let lead = Rect::from_min_max(
                        pos2(origin.x + shift, top),
                        pos2(rect.right() + shift, header.bottom()),
                    );
                    painter.rect_filled(lead, CornerRadius::ZERO, header_fill);
                    painter.hline(lead.x_range(), y, Stroke::new(hairline, palette.outline));
                    painter.vline(
                        lead.right() - hairline / 2.0,
                        lead.y_range(),
                        Stroke::new(hairline, palette.outline),
                    );
                    rect.translate(vec2(shift, 0.0))
                } else {
                    rect
                };
                // A header that sorts nothing still takes the pointer's
                // clicks, and drops them: a hover-only header would let
                // them through to a row scrolled under it.
                let (sense, kind) = if column.sortable {
                    (Sense::click(), WidgetType::Button)
                } else {
                    (Sense::CLICK, WidgetType::Label)
                };
                let response = ui.interact(rect, id.with(("header", col)), sense);
                focus::hint(ui, &response, rect, focus::Ring::Inset { radius: 0 });
                // Column names come from the server: nothing hidden in them.
                let name = display_safe(column.name);
                response.widget_info(|| WidgetInfo::labeled(kind, true, &*name));
                if response.clicked() && column.sortable {
                    output.sort_clicked = Some(col);
                }
                if response.hovered() && column.sortable {
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
                // Only the pointer drags it: no stop for the Tab key.
                let drag = ui.interact(handle, id.with(("resize", col)), Sense::DRAG);
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
                    keep = true;
                }
            }
            origin
        });

    // The columns in view, for the status line to say: the ones that show
    // any of themselves past the pinned one.
    {
        let offset = scroll.state.offset.x;
        let from = offset + if pinned { gutter + widths[0] } else { 0.0 };
        let to = offset + scroll.inner_rect.width();
        let mut edge = gutter;
        let mut seen: Option<(usize, usize)> = None;
        for (col, width) in widths.iter().enumerate() {
            let scrolls = !(pinned && col == 0);
            if scrolls && edge < to && edge + width > from {
                seen = Some((seen.map_or(col, |(first, _)| first), col));
            }
            edge += width;
        }
        // The pinned column leads the range while its neighbour shows.
        let seen = match seen {
            Some((1, last)) if pinned => Some((0, last)),
            None if pinned => Some((0, 0)),
            seen => seen,
        };
        if let Some((first, last)) = seen {
            let shown = ColumnsShown {
                first,
                last,
                total: widths.len(),
                pinned,
            };
            ui.data_mut(|data| data.insert_temp(id.with("columns-shown"), shown));
        }
    }

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

    if lit {
        focus::pane_border(ui, visible, look, palette);
    }
    ui.data_mut(|data| {
        if keep {
            data.insert_temp(widths_id, widths);
        }
        data.insert_temp(last_id, selection);
        data.insert_temp(
            id,
            Kept {
                columns: columns.len(),
                scroll: scroll.id,
            },
        );
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
    // Column names and types come from the server: nothing hidden in them.
    let name = display_safe(column.name);
    let type_text = display_safe(&column.type_line);
    // The terminal writes the sort's arrow after the name.
    let arrow_text = match (column.sort, look.terminal) {
        (Some(SortDir::Asc), true) => format!("{name} ↑"),
        (Some(SortDir::Desc), true) => format!("{name} ↓"),
        _ => name.into_owned(),
    };
    let name_width = text_width(ui, &arrow_text, name_role, look);
    let type_width = text_width(ui, &type_text, type_role, look);
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
    // The terminal asks for a new row's value in its header: a star a
    // space after the column's name, a piece of its own.
    let star = column.required && look.terminal;
    let star_room = if star {
        text_width(ui, " *", name_role, look)
    } else {
        0.0
    };
    // Numbers sit at the right, their header with them: the star too.
    let (name_x, type_x) = if column.numeric {
        (
            rect.right() - pad - arrow - star_room - name_width,
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
    if star {
        paint(
            &clip,
            ui,
            name_role,
            "*",
            Tone::Danger.color(palette),
            name_x + name_width + star_room,
            name_y,
            true,
            look,
        );
    }
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
        &type_text,
        palette.dim,
        type_x,
        type_y,
        false,
        look,
    );
}

/// Space inside a chip, at each side of its text.
const CHIP_PAD: f32 = 5.0;
/// Space between two chips.
const CHIP_GAP: f32 = 4.0;

/// How a chip draws: NULL is filled, an array's element and a binary
/// value's size are outlined.
#[derive(Clone, Copy)]
struct ChipSkin {
    role: TextRole,
    text: egui::Color32,
    fill: Option<egui::Color32>,
    border: Option<egui::Color32>,
}

impl ChipSkin {
    /// NULL: quieter than any value.
    fn null(palette: &Palette) -> Self {
        Self {
            role: TextRole::JsonChip,
            text: palette.faint,
            fill: Some(palette.surface),
            border: None,
        }
    }

    /// A piece of a value, or a note about one, in `role`.
    fn outlined(role: TextRole, palette: &Palette) -> Self {
        Self {
            role,
            text: palette.secondary,
            fill: None,
            border: Some(palette.outline),
        }
    }
}

/// The width of a chip holding `text`.
fn chip_width(ui: &Ui, text: &str, role: TextRole, look: &Look) -> f32 {
    text_width(ui, text, role, look) + 2.0 * CHIP_PAD
}

/// Paints a chip holding `text`, its left edge at `left`, centred on
/// `center`.
fn paint_chip(
    painter: &egui::Painter,
    ui: &Ui,
    text: &str,
    left: f32,
    center: f32,
    skin: ChipSkin,
    look: &Look,
) {
    // The text's line, and room for an outline round it.
    let line = skin.role.row_height(ui.ctx(), look.faces);
    let height = line + if skin.border.is_some() { 4.0 } else { 2.0 };
    let rect = Rect::from_min_size(
        pos2(left, center - height / 2.0),
        vec2(chip_width(ui, text, skin.role, look), height),
    );
    let corner = CornerRadius::same(4);
    if let Some(fill) = skin.fill {
        painter.rect_filled(rect, corner, fill);
    }
    if let Some(border) = skin.border {
        painter.rect_stroke(
            rect,
            corner,
            Stroke::new(crate::ui::widgets::hairline(ui), border),
            StrokeKind::Inside,
        );
    }
    Text::one(look, skin.role, text, skin.text)
        .layout(ui.ctx())
        .paint_center(painter, rect.center());
}

/// NULL where a value would be, laid out in `ui`: the grid's chip, or the
/// terminal's faint word. The row panel's fields use it.
pub fn null_label(ui: &mut Ui, look: &Look, palette: &Palette) -> egui::Response {
    let role = data_role(look);
    if look.terminal {
        return Text::one(look, role, "NULL", palette.faint)
            .layout(ui.ctx())
            .label(ui);
    }
    let skin = ChipSkin::null(palette);
    let size = vec2(
        chip_width(ui, "NULL", skin.role, look),
        role.row_height(ui.ctx(), look.faces),
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, "NULL"));
    if ui.is_rect_visible(rect) {
        paint_chip(
            ui.painter(),
            ui,
            "NULL",
            rect.left(),
            rect.center().y,
            skin,
            look,
        );
    }
    response
}

/// One cell: its text, tag, chips or colour swatch, cut to fit.
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
        // With the numbers in a numeric column, as a value would be.
        let numeric = column.numeric;
        if look.terminal {
            let x = if numeric {
                rect.right() - pad
            } else {
                rect.left() + pad
            };
            paint(
                &clip,
                ui,
                role,
                "NULL",
                palette.faint,
                x,
                center,
                numeric,
                look,
            );
        } else {
            let skin = ChipSkin::null(palette);
            let left = if numeric {
                rect.right() - pad - chip_width(ui, "NULL", skin.role, look)
            } else {
                rect.left() + pad
            };
            paint_chip(&clip, ui, "NULL", left, center, skin, look);
        }
        return;
    }
    match content.style {
        Style::Quiet => {
            let shown = ellipsize(&content.text, room, false, |text| width(text, role));
            paint(
                &clip,
                ui,
                role,
                &shown,
                palette.faint,
                rect.left() + pad,
                center,
                false,
                look,
            );
        }
        Style::Chip => {
            let skin = ChipSkin::outlined(TextRole::FieldLabel, palette);
            let shown = ellipsize(&content.text, room - 2.0 * CHIP_PAD, false, |text| {
                width(text, skin.role)
            });
            paint_chip(&clip, ui, &shown, rect.left() + pad, center, skin, look);
        }
        Style::Array => {
            // The terminal writes an array as the database does.
            let Some(array) = array_items(&content.text).filter(|_| !look.terminal) else {
                let shown = ellipsize(&content.text, room, false, |text| width(text, role));
                paint(
                    &clip,
                    ui,
                    role,
                    &shown,
                    palette.text,
                    rect.left() + pad,
                    center,
                    false,
                    look,
                );
                return;
            };
            let skin = ChipSkin::outlined(TextRole::ValueTag, palette);
            // What stands for the elements that do not fit: how many, or
            // "…" when the text was cut and nobody counted.
            let more = |rest: usize| {
                if array.cut {
                    "…".to_owned()
                } else {
                    format!("+{rest}")
                }
            };
            let right = rect.right() - pad;
            let mut left = rect.left() + pad;
            let mut shown = 0;
            for (index, item) in array.items.iter().enumerate() {
                let after = array.items.len() - index - 1;
                let reserve = if after > 0 || array.cut {
                    CHIP_GAP + chip_width(ui, &more(after), skin.role, look)
                } else {
                    0.0
                };
                let room = right - left - reserve;
                let wide = chip_width(ui, item, skin.role, look);
                if wide <= room {
                    paint_chip(&clip, ui, item, left, center, skin, look);
                    left += wide + CHIP_GAP;
                } else if index == 0 {
                    // The first element always shows, cut to its room.
                    let cut = ellipsize(item, (room - 2.0 * CHIP_PAD).max(0.0), false, |text| {
                        width(text, skin.role)
                    });
                    paint_chip(&clip, ui, &cut, left, center, skin, look);
                    left += chip_width(ui, &cut, skin.role, look) + CHIP_GAP;
                } else {
                    break;
                }
                shown += 1;
                if wide > room {
                    break;
                }
            }
            let rest = array.items.len() - shown;
            if rest > 0 || array.cut {
                paint_chip(&clip, ui, &more(rest), left, center, skin, look);
            }
        }
        Style::Tag(_) => {
            let (color, fill) = crate::ui::value_tags::style_colors(content.style, look, palette);
            // macOS and Windows: a chip in the value-tag face, 2 above and
            // below, 6 at the sides. Terminal: the text alone, in the
            // tag's colour.
            let Some(fill) = fill else {
                let shown = ellipsize(&content.text, room, false, |text| width(text, role));
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
                return;
            };
            let tag_role = TextRole::ValueTag;
            let shown = ellipsize(&content.text, room - 12.0, false, |text| {
                width(text, tag_role)
            });
            let text_width = width(&shown, tag_role);
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
                laid.paint_center(&clip, chip.center());
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

    /// A row of a page, marked `mark` and numbered from 1.
    fn numbered(mark: RowMark, row: usize) -> Row {
        Row {
            mark,
            number: Some(row as u64 + 1),
        }
    }

    /// Two columns whose headers sort, or are labels.
    fn columns_that(sortable: bool) -> Vec<Column<'static>> {
        vec![
            Column {
                name: "id",
                type_line: "INTEGER".into(),
                numeric: true,
                sort: None,
                key: true,
                flexible: false,
                sortable,
                required: false,
            },
            Column {
                name: "email",
                type_line: "TEXT".into(),
                numeric: false,
                sort: Some(SortDir::Asc),
                key: false,
                flexible: false,
                sortable,
                required: false,
            },
        ]
    }

    fn columns() -> Vec<Column<'static>> {
        columns_that(true)
    }

    /// Runs one frame of a grid with `rows` rows and returns its output and
    /// the AccessKit tree.
    fn frame(
        ctx: &egui::Context,
        rows: usize,
        events: Vec<egui::Event>,
    ) -> (GridOutput, egui::accesskit::TreeUpdate) {
        frame_of(ctx, &columns(), rows, events)
    }

    /// [`frame`] of a grid with `columns`.
    fn frame_of(
        ctx: &egui::Context,
        columns: &[Column<'_>],
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
                result = show(
                    ui,
                    egui::Id::new("grid"),
                    columns,
                    rows,
                    None,
                    false,
                    &palette,
                    &crate::theme::Look::standard(),
                    &|row| numbered(RowMark::None, row),
                    None,
                    None,
                    |row, col| Cell {
                        text: format!("r{row}c{col}").into(),
                        ..Default::default()
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

    /// What a frame of a three-row grid paints with cell (1, 1) selected:
    /// its filled rectangles and its outlines.
    fn painted(
        ctx: &egui::Context,
        look: &Look,
        palette: &Palette,
        keys: bool,
        events: Vec<egui::Event>,
    ) -> Vec<egui::epaint::RectShape> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 400.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            focus::begin_frame(ui.ctx());
            show(
                ui,
                egui::Id::new("grid"),
                &columns(),
                3,
                Some(CellPos { row: 1, col: 1 }),
                keys,
                palette,
                look,
                &|row| numbered(RowMark::None, row),
                None,
                None,
                |row, col| Cell {
                    text: format!("r{row}c{col}").into(),
                    ..Default::default()
                },
            );
        });
        output.textures_delta.clear();
        output
            .shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Rect(rect) => Some(rect),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_keyboard_lights_its_cell_and_the_pointer_only_the_row() {
        for look in Look::ALL {
            let palette = Palette::light();
            let ctx = egui::Context::default();
            crate::theme::install(&ctx, false, &look);
            crate::theme::apply(&ctx, &palette, &look);
            let key = crate::testing::key(egui::Key::ArrowDown, egui::Modifiers::NONE);
            // The cell's mark: the selection's colour on the cell alone,
            // or the terminal's accent block under its text.
            let lit = |shapes: &[egui::epaint::RectShape]| {
                shapes.iter().any(|rect| {
                    let wide = rect.rect.width();
                    let cell = rect.fill == palette.selection && wide > 20.0 && wide < 700.0;
                    let block = rect.fill == palette.accent && wide > 20.0;
                    if look.terminal { block } else { cell }
                })
            };
            let whole_row = |shapes: &[egui::epaint::RectShape]| {
                shapes
                    .iter()
                    .any(|rect| rect.fill == palette.selection && rect.rect.width() >= 700.0)
            };
            // No key yet: the row is selected, the cell is not lit.
            let shapes = painted(&ctx, &look, &palette, true, Vec::new());
            assert!(!lit(&shapes) && whole_row(&shapes), "{}", look.name);
            // A key, and the arrows are the grid's: the cell is lit.
            painted(&ctx, &look, &palette, true, vec![key.clone()]);
            let shapes = painted(&ctx, &look, &palette, true, Vec::new());
            assert!(lit(&shapes), "{}", look.name);
            // The desktop looks move the selection's colour to the cell.
            assert_eq!(whole_row(&shapes), look.terminal, "{}", look.name);
            // The arrows are the tree's: the grid shows its row alone.
            let shapes = painted(&ctx, &look, &palette, false, Vec::new());
            assert!(!lit(&shapes) && whole_row(&shapes), "{}", look.name);
        }
    }

    /// The colour a text shape is painted in.
    fn text_color(text: &egui::epaint::TextShape) -> egui::Color32 {
        text.override_text_color.unwrap_or_else(|| {
            let section = text.galley.job.sections.first();
            match section.map(|section| section.format.color) {
                Some(color) if color != egui::Color32::PLACEHOLDER => color,
                _ => text.fallback_color,
            }
        })
    }

    /// What a frame of a three-row grid paints with nothing selected, its
    /// cell (1, 1) marked `mark` and its row 1 marked `row`: the rectangles,
    /// and each text with its colour and where it starts.
    fn marked(
        ctx: &egui::Context,
        look: &Look,
        palette: &Palette,
        mark: Mark,
        row: RowMark,
    ) -> (
        Vec<egui::epaint::RectShape>,
        Vec<(String, egui::Color32, egui::Pos2)>,
    ) {
        marked_with(ctx, look, palette, (mark, row), None, Vec::new())
    }

    /// [`marked`], with `selection` selected and `events` given.
    fn marked_with(
        ctx: &egui::Context,
        look: &Look,
        palette: &Palette,
        (mark, row): (Mark, RowMark),
        selection: Option<CellPos>,
        events: Vec<egui::Event>,
    ) -> (
        Vec<egui::epaint::RectShape>,
        Vec<(String, egui::Color32, egui::Pos2)>,
    ) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 400.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            focus::begin_frame(ui.ctx());
            show(
                ui,
                egui::Id::new("grid"),
                &columns(),
                3,
                selection,
                true,
                palette,
                look,
                &|at| numbered(if at == 1 { row } else { RowMark::None }, at),
                None,
                None,
                |row, col| Cell {
                    text: format!("r{row}c{col}").into(),
                    mark: if (row, col) == (1, 1) {
                        mark
                    } else {
                        Mark::None
                    },
                    ..Default::default()
                },
            );
        });
        output.textures_delta.clear();
        let (mut rects, mut texts) = (Vec::new(), Vec::new());
        for clipped in output.shapes {
            match clipped.shape {
                egui::Shape::Rect(rect) => rects.push(rect),
                egui::Shape::Text(text) => {
                    let color = text_color(&text);
                    texts.push((text.galley.text().to_owned(), color, text.pos));
                }
                _ => {}
            }
        }
        (rects, texts)
    }

    #[test]
    fn a_new_row_is_filled_and_marked_in_the_success_tone() {
        use crate::ui::states::Tone;
        for look in Look::ALL {
            for palette in [Palette::light(), Palette::dark()] {
                let said = format!("{}, dark: {}", look.name, palette.dark);
                let ctx = egui::Context::default();
                crate::theme::install(&ctx, false, &look);
                crate::theme::apply(&ctx, &palette, &look);
                // The columns are fitted on the first frame.
                marked(&ctx, &look, &palette, Mark::None, RowMark::None);
                let (rects, texts) = marked(&ctx, &look, &palette, Mark::Added, RowMark::New);
                let (fill, color) = (
                    Tone::Success.fill(&look, &palette),
                    Tone::Success.color(&palette),
                );
                // The row's own fill, as wide as the grid: no cell's tint.
                let filled = rects
                    .iter()
                    .any(|rect| rect.fill == fill && rect.rect.width() > 400.0);
                assert!(filled, "{said}: the row's fill");
                let bar = rects.iter().any(|rect| {
                    rect.fill == color
                        && rect.rect.width() == 3.0
                        && rect.rect.height() == look.grid_row
                });
                let signed = texts
                    .iter()
                    .any(|(text, painted, _)| text == "+" && *painted == color);
                let marker = texts.iter().find(|(text, ..)| text == "r1c1");
                let marker = marker.map(|(_, color, _)| *color);
                if look.terminal {
                    // `+` in the gutter, and the marker dimmed.
                    assert!(signed && !bar, "{said}");
                    assert_eq!(marker, Some(palette.dim), "{said}");
                } else {
                    // The bar at the row's left, and the marker in the tone.
                    assert!(bar && !signed, "{said}");
                    assert_eq!(marker, Some(color), "{said}");
                }
                // A cell nothing is set in is written quieter than a value.
                let (_, texts) = marked(&ctx, &look, &palette, Mark::Unset, RowMark::New);
                let unset = texts.iter().find(|(text, ..)| text == "r1c1");
                assert_eq!(
                    unset.map(|(_, color, _)| *color),
                    Some(palette.dim),
                    "{said}"
                );
            }
        }
    }

    #[test]
    fn a_new_row_is_read_as_one_and_the_others_by_their_numbers() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let (look, palette) = (Look::standard(), Palette::dark());
        crate::theme::install(&ctx, false, &look);
        crate::theme::apply(&ctx, &palette, &look);
        let rows = |row: usize| match row {
            0 => Row {
                mark: RowMark::New,
                number: None,
            },
            // The page's rows keep their numbers under it.
            row => numbered(RowMark::None, row - 1),
        };
        let mut tree = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 400.0),
                )),
                ..Default::default()
            },
            |ui| {
                show(
                    ui,
                    egui::Id::new("grid"),
                    &columns(),
                    3,
                    None,
                    false,
                    &palette,
                    &look,
                    &rows,
                    None,
                    None,
                    |row, col| Cell {
                        text: format!("r{row}c{col}").into(),
                        ..Default::default()
                    },
                );
            },
        );
        output.textures_delta.clear();
        tree.replace(output.platform_output.accesskit_update.expect("accesskit"));
        let names = crate::testing::labels(&tree.unwrap());
        for name in ["New row, not saved", "Row 1", "Row 2"] {
            assert!(names.iter().any(|label| label == name), "{name}: {names:?}");
        }
        assert!(!names.iter().any(|label| label == "Row 3"), "{names:?}");
    }

    #[test]
    fn a_marked_cell_is_tinted_and_its_row_is_marked() {
        use crate::ui::states::Tone;
        for look in Look::ALL {
            for palette in [Palette::light(), Palette::dark()] {
                let said = format!("{}, dark: {}", look.name, palette.dark);
                let ctx = egui::Context::default();
                crate::theme::install(&ctx, false, &look);
                crate::theme::apply(&ctx, &palette, &look);
                // The columns are fitted on the first frame.
                marked(&ctx, &look, &palette, Mark::None, RowMark::None);
                type Texts = [(String, egui::Color32, egui::Pos2)];
                let (_, plain) = marked(&ctx, &look, &palette, Mark::None, RowMark::None);
                let place = |cell: &str| {
                    let found = plain.iter().find(|(text, ..)| text == cell);
                    found.map(|(_, _, pos)| *pos).expect("the cell's text")
                };
                // Where the marked cell's text starts, and its row's key's.
                // (A cell being saved may show its text cut, to make room.)
                let (cell, key) = (place("r1c1"), place("r1c0"));
                let color_at = |texts: &Texts, pos: egui::Pos2| {
                    let found = texts.iter().find(|(.., at)| *at == pos);
                    found.map(|(_, color, _)| *color)
                };
                // A fill behind the marked cell's text, a cell wide.
                let behind = |rects: &[egui::epaint::RectShape], fill: egui::Color32| {
                    rects.iter().any(|rect| {
                        rect.fill == fill && rect.rect.contains(cell) && rect.rect.width() < 400.0
                    })
                };
                for (mark, row, tone, sign) in [
                    (Mark::Pending, RowMark::Changed, Tone::Warning, "~"),
                    (Mark::Saving, RowMark::Changed, Tone::Warning, "~"),
                    (Mark::Trouble, RowMark::Trouble, Tone::Danger, "!"),
                ] {
                    let said = format!("{said}, {mark:?}");
                    let (rects, texts) = marked(&ctx, &look, &palette, mark, row);
                    assert!(
                        behind(&rects, tone.fill(&look, &palette)),
                        "{said}: the tint"
                    );
                    let color = tone.color(&palette);
                    // A bar down the left of the cell (2 pt) or the row (3).
                    let bar = |width: f32| {
                        rects.iter().any(|rect| {
                            rect.fill == color
                                && rect.rect.width() == width
                                && rect.rect.height() == look.grid_row
                        })
                    };
                    let signed = texts
                        .iter()
                        .any(|(text, painted, _)| text == sign && *painted == color);
                    if look.terminal {
                        assert!(signed, "{said}: the gutter");
                        assert_eq!(color_at(&texts, cell), Some(color), "{said}: the text");
                        assert!(!bar(2.0) && !bar(3.0), "{said}");
                    } else {
                        // The amber bar is a pending cell's: one to fix
                        // has its line instead.
                        assert_eq!(bar(2.0), mark != Mark::Trouble, "{said}: the cell's bar");
                        assert!(bar(3.0), "{said}: the row's bar");
                        assert_eq!(color_at(&texts, key), Some(color), "{said}: the key");
                        assert_eq!(color_at(&texts, cell), Some(palette.text), "{said}");
                        assert!(!signed, "{said}: no gutter");
                    }
                    // A cell to fix has a line inside it as well.
                    let lined = rects.iter().any(|rect| {
                        rect.stroke == Stroke::new(1.0, color)
                            && rect.stroke_kind == StrokeKind::Inside
                    });
                    assert_eq!(lined, mark == Mark::Trouble, "{said}: the line");
                }
                // Written: green, and no mark on the row.
                let (rects, _) = marked(&ctx, &look, &palette, Mark::Saved, RowMark::None);
                assert!(
                    behind(&rects, Tone::Success.fill(&look, &palette)),
                    "{said}: saved"
                );
                // A computed column: quieter on macOS and Windows, as it
                // was in the terminal.
                let (rects, texts) = marked(&ctx, &look, &palette, Mark::Locked, RowMark::None);
                assert_eq!(
                    behind(&rects, palette.surface),
                    !look.terminal,
                    "{said}: locked"
                );
                let quiet = if look.terminal {
                    palette.text
                } else {
                    palette.secondary
                };
                assert_eq!(color_at(&texts, cell), Some(quiet), "{said}: locked");
                // A row a save found gone: its text dim in every look, and
                // nothing else of a mark.
                let (rects, texts) = marked(&ctx, &look, &palette, Mark::Gone, RowMark::None);
                assert_eq!(color_at(&texts, cell), Some(palette.dim), "{said}: gone");
                for tone in [Tone::Warning, Tone::Danger, Tone::Success] {
                    assert!(
                        !behind(&rects, tone.fill(&look, &palette)),
                        "{said}: gone, {tone:?}"
                    );
                }
                assert!(!behind(&rects, palette.surface), "{said}: gone");
                assert!(
                    !texts.iter().any(|(text, ..)| text == "~" || text == "!"),
                    "{said}: gone"
                );
                // Nothing pending: none of it.
                let (rects, texts) = marked(&ctx, &look, &palette, Mark::None, RowMark::None);
                for tone in [Tone::Warning, Tone::Danger, Tone::Success] {
                    assert!(
                        !behind(&rects, tone.fill(&look, &palette)),
                        "{said}: {tone:?}"
                    );
                }
                assert!(
                    !texts.iter().any(|(text, ..)| text == "~" || text == "!"),
                    "{said}"
                );
                assert_eq!(color_at(&texts, cell), Some(palette.text), "{said}");
            }
        }
    }

    #[test]
    fn the_cursor_on_a_pending_cell_is_a_line_over_its_tint() {
        use crate::ui::states::Tone;
        for look in Look::ALL {
            let palette = Palette::light();
            let ctx = egui::Context::default();
            crate::theme::install(&ctx, false, &look);
            crate::theme::apply(&ctx, &palette, &look);
            let here = Some(CellPos { row: 1, col: 1 });
            let pending = (Mark::Pending, RowMark::Changed);
            // A key, and the arrows are the grid's: its cell is lit.
            let key = crate::testing::key(egui::Key::ArrowDown, egui::Modifiers::NONE);
            marked_with(&ctx, &look, &palette, pending, here, vec![key]);
            let (rects, texts) = marked_with(&ctx, &look, &palette, pending, here, Vec::new());
            let (_, _, cell) = texts
                .iter()
                .find(|(text, ..)| text == "r1c1")
                .expect("the cell's text");
            let over = |fill: egui::Color32| {
                rects.iter().any(|rect| {
                    rect.fill == fill && rect.rect.contains(*cell) && rect.rect.width() < 400.0
                })
            };
            // The tint, not the selection's colour nor the terminal's block.
            assert!(over(Tone::Warning.fill(&look, &palette)), "{}", look.name);
            assert!(
                !over(palette.selection) && !over(palette.accent),
                "{}",
                look.name
            );
            // And the cursor's line inside it.
            let lined = rects.iter().any(|rect| {
                rect.stroke == Stroke::new(2.0, palette.accent)
                    && rect.stroke_kind == StrokeKind::Inside
                    && rect.rect.contains(*cell)
                    && rect.rect.width() < 400.0
            });
            assert!(lined, "{}", look.name);
            // The terminal keeps the text in the pending colour under it.
            if look.terminal {
                let color = texts
                    .iter()
                    .find(|(text, ..)| text == "r1c1")
                    .map(|text| text.1);
                assert_eq!(color, Some(Tone::Warning.color(&palette)));
            }
        }
    }

    #[test]
    fn the_terminals_cursor_on_a_gone_cell_is_reverse_video() {
        let look = Look::omarchy();
        let palette = Palette::light();
        let ctx = egui::Context::default();
        crate::theme::install(&ctx, false, &look);
        crate::theme::apply(&ctx, &palette, &look);
        let here = Some(CellPos { row: 1, col: 1 });
        let gone = (Mark::Gone, RowMark::None);
        // A key, and the arrows are the grid's: its cell is lit.
        let key = crate::testing::key(egui::Key::ArrowDown, egui::Modifiers::NONE);
        marked_with(&ctx, &look, &palette, gone, here, vec![key]);
        let (rects, texts) = marked_with(&ctx, &look, &palette, gone, here, Vec::new());
        let (_, color, cell) = texts
            .iter()
            .find(|(text, ..)| text == "r1c1")
            .expect("the cell's text");
        // As on any cell without a mark: the accent behind, the text in the
        // window's tone. Its dim text would not be read on the accent.
        let block = rects.iter().any(|rect| {
            rect.fill == palette.accent && rect.rect.contains(*cell) && rect.rect.width() < 400.0
        });
        assert!(block);
        assert_eq!(*color, palette.window);
    }

    #[test]
    fn a_double_click_reports_its_cell() {
        let ctx = egui::Context::default();
        let look = crate::theme::Look::standard();
        crate::theme::install(&ctx, false, &look);
        ctx.enable_accesskit();
        frame(&ctx, 10, vec![]);
        frame(&ctx, 10, vec![]);
        // The second row, a little into the second column.
        let widths: Vec<f32> = ctx
            .data(|data| data.get_temp(egui::Id::new("grid").with(("widths", 2_usize))))
            .expect("the widths");
        let pos = egui::pos2(widths[0] + 5.0, header_height(&look) + look.grid_row * 1.5);
        let press = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let cell = Some(CellPos { row: 1, col: 1 });
        frame(&ctx, 10, vec![egui::Event::PointerMoved(pos)]);
        frame(&ctx, 10, vec![press(true)]);
        // One click selects, and is no double click.
        let (once, _) = frame(&ctx, 10, vec![press(false)]);
        assert_eq!((once.clicked, once.double_clicked), (cell, None));
        frame(&ctx, 10, vec![press(true)]);
        let (twice, _) = frame(&ctx, 10, vec![press(false)]);
        assert_eq!((twice.clicked, twice.double_clicked), (cell, cell));
    }

    #[test]
    fn a_leading_key_column_stays_in_sight_while_the_others_scroll() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx, false, &Look::standard());
        ctx.enable_accesskit();
        let id = egui::Id::new("grid");
        let names: Vec<String> = (0..8).map(|col| format!("column_{col}")).collect();
        let columns: Vec<Column<'_>> = names
            .iter()
            .enumerate()
            .map(|(col, name)| Column {
                name,
                type_line: "int8".into(),
                numeric: false,
                sort: None,
                key: col == 0,
                flexible: false,
                sortable: true,
                required: false,
            })
            .collect();
        // One frame of a 320 pt wide grid: its output, where each cell's
        // text was painted, and the AccessKit tree.
        let frame = |events: Vec<egui::Event>| {
            let mut result = GridOutput::default();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 300.0),
                )),
                events,
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                result = show(
                    ui,
                    id,
                    &columns,
                    3,
                    None,
                    false,
                    &Palette::light(),
                    &Look::standard(),
                    &|row| numbered(RowMark::None, row),
                    None,
                    None,
                    |row, col| Cell {
                        text: format!("r{row}c{col}").into(),
                        ..Default::default()
                    },
                );
            });
            output.textures_delta.clear();
            let texts: Vec<(String, f32)> = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) if clipped.clip_rect.contains(text.pos) => {
                        Some((text.galley.text().to_owned(), text.pos.x))
                    }
                    _ => None,
                })
                .collect();
            let tree = output.platform_output.accesskit_update.expect("accesskit");
            (result, texts, tree)
        };
        let at = |texts: &[(String, f32)], cell: &str| {
            texts
                .iter()
                .rev()
                .find(|(text, _)| text == cell)
                .map(|(_, x)| *x)
        };
        frame(Vec::new());
        let (_, texts, tree) = frame(Vec::new());
        let shown = columns_shown(&ctx, id).expect("the columns in view");
        assert_eq!((shown.first, shown.total, shown.pinned), (0, 8, true));
        assert!(shown.partial(), "{shown:?}");
        let before = at(&texts, "r0c0").expect("the key");
        // To the columns out of view: the pill at the header's end.
        let more = tree
            .nodes
            .iter()
            .find(|(_, node)| {
                node.label()
                    .is_some_and(|name| name.ends_with("more columns"))
            })
            .map(|(id, _)| *id)
            .expect("the pill");
        frame(vec![click(more)]);
        frame(Vec::new());
        let (_, texts, _) = frame(Vec::new());
        let scrolled = columns_shown(&ctx, id).unwrap();
        assert!(scrolled.last > shown.last, "{scrolled:?}");
        // The range is of the columns that scrolled into view: the key is
        // in sight beside it, and some are still out of it.
        assert!(scrolled.first > 1 && scrolled.pinned, "{scrolled:?}");
        assert!(scrolled.partial(), "{scrolled:?}");
        // The key is where it was; the column after it went under it.
        assert_eq!(at(&texts, "r0c0"), Some(before));
        assert!(at(&texts, "r0c1").is_none_or(|x| x < before));
        // A click over the key picks the key's cell, not one under it.
        let over_key = egui::pos2(before + 4.0, 45.0 + 26.0 * 1.5);
        let press = |pressed| egui::Event::PointerButton {
            pos: over_key,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(vec![egui::Event::PointerMoved(over_key), press(true)]);
        let (output, _, _) = frame(vec![press(false)]);
        assert_eq!(output.clicked, Some(CellPos { row: 1, col: 0 }));
    }

    #[test]
    fn a_grid_without_a_leading_key_pins_nothing() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx, false, &Look::standard());
        ctx.enable_accesskit();
        // A result's columns: none is a key.
        let mut columns = columns_that(false);
        columns[0].key = false;
        frame_of(&ctx, &columns, 3, Vec::new());
        frame_of(&ctx, &columns, 3, Vec::new());
        let shown = columns_shown(&ctx, egui::Id::new("grid")).expect("the columns in view");
        assert!(!shown.pinned);
        assert!(!shown.partial(), "every column fits: {shown:?}");
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
            ..Default::default()
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
            ..Default::default()
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
    fn a_forgotten_grid_leaves_nothing_in_eguis_memory() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx, false, &crate::theme::Look::standard());
        ctx.enable_accesskit();
        let id = egui::Id::new("grid");
        frame(&ctx, 10, vec![]);
        frame(&ctx, 10, vec![]);
        let kept: Kept = ctx.data(|data| data.get_temp(id)).expect("kept");
        let widths = id.with(("widths", kept.columns));
        let has_widths = || ctx.data(|data| data.get_temp::<Vec<f32>>(widths).is_some());
        let scrolled = || egui::scroll_area::State::load(&ctx, kept.scroll).is_some();
        assert_eq!(kept.columns, columns().len());
        assert!(remembered(&ctx, id) && has_widths() && scrolled());
        forget(&ctx, id);
        assert!(!remembered(&ctx, id) && !has_widths() && !scrolled());
        // Forgetting a grid never drawn, or one already forgotten, is fine.
        forget(&ctx, id);
        forget(&ctx, egui::Id::new("another grid"));
        // Drawn again, it fits its columns anew.
        frame(&ctx, 10, vec![]);
        assert!(remembered(&ctx, id) && has_widths());
        // One with no rows keeps no widths, and is forgotten all the same.
        forget(&ctx, id);
        frame(&ctx, 0, vec![]);
        assert!(remembered(&ctx, id) && !has_widths());
        forget(&ctx, id);
        assert!(!remembered(&ctx, id) && !scrolled());
    }

    #[test]
    fn a_header_that_sorts_nothing_is_a_label_that_keeps_clicks_from_the_rows() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx, false, &crate::theme::Look::standard());
        ctx.enable_accesskit();
        let columns = columns_that(false);
        frame_of(&ctx, &columns, 100, vec![]);
        let (_, tree) = frame_of(&ctx, &columns, 100, vec![]);
        assert!(crate::testing::node(&tree, "email", Role::Button).is_none());
        let email = crate::testing::node(&tree, "email", Role::Label).expect("email header");
        let (output, _) = frame_of(&ctx, &columns, 100, vec![click(email)]);
        assert_eq!(output, GridOutput::default());
        // Scrolled, rows pass under the header: a press on the header is
        // not one on the row under it.
        let header = crate::testing::bounds(&tree, "email", Role::Label).unwrap();
        let at = header.center();
        let wheel = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -300.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        };
        frame_of(&ctx, &columns, 100, vec![egui::Event::PointerMoved(at)]);
        frame_of(&ctx, &columns, 100, vec![wheel]);
        for _ in 0..60 {
            frame_of(&ctx, &columns, 100, vec![]);
        }
        let (_, tree) = frame_of(&ctx, &columns, 100, vec![]);
        assert!(
            crate::testing::node(&tree, "Row 1", Role::Button).is_none()
                || crate::testing::bounds(&tree, "Row 1", Role::Button)
                    .is_some_and(|row| row.bottom() <= header.top()),
            "the rows scrolled"
        );
        let press = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let (down, _) = frame_of(&ctx, &columns, 100, vec![press(true)]);
        let (up, _) = frame_of(&ctx, &columns, 100, vec![press(false)]);
        assert_eq!((down, up), (GridOutput::default(), GridOutput::default()));
        // A press under the header does select the row there.
        let below = at + egui::vec2(0.0, header.height() + 10.0);
        let press = |pressed| egui::Event::PointerButton {
            pos: below,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame_of(&ctx, &columns, 100, vec![egui::Event::PointerMoved(below)]);
        frame_of(&ctx, &columns, 100, vec![press(true)]);
        let (up, _) = frame_of(&ctx, &columns, 100, vec![press(false)]);
        assert!(up.clicked.is_some_and(|cell| cell.row > 0), "{up:?}");
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
