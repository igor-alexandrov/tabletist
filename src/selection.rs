//! What a grid has selected besides its cursor's cell: a rectangle of
//! cells, or whole rows. A table's grid and a SQL result's share it.
//!
//! Rows are named as [`CellPos::row`] names them: a page's row, or in a
//! table's grid a new row by its own (`edit::new_row`). Places are the
//! grid's order, which an [`Order`] gives: a table's has its new rows
//! among the page's, a result's is its rows'. What the selection covers is
//! read in places, so a range drawn over a new row covers it where it
//! stands.

use std::collections::BTreeSet;

use crate::edit::Order;
use crate::model::CellPos;

/// Whether moves keep a rectangle: the terminal look's visual selections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Held {
    /// Moves end it: Shift with a click or an arrow, a drag.
    No,
    /// `v`.
    Visual,
    /// `ctrl+v`.
    Block,
}

/// What is selected with the cursor's cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Range {
    /// The rectangle of cells from `anchor` to the cursor.
    Cells { anchor: CellPos, held: Held },
    /// Whole rows: those in `picked`, and with an anchor every row from it
    /// to the cursor's. `held` is the terminal look's `V`.
    Rows {
        anchor: Option<usize>,
        picked: BTreeSet<usize>,
        held: bool,
    },
}

/// The terminal look's visual modes, as its status line names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visual {
    /// `v`: cells.
    Cells,
    /// `ctrl+v`: a block of cells.
    Block,
    /// `V`: whole rows.
    Rows,
}

impl Visual {
    /// The mode as the status line writes it.
    pub fn mode(self) -> &'static str {
        match self {
            Self::Cells => "-- VISUAL --",
            Self::Block => "-- VISUAL BLOCK --",
            Self::Rows => "-- VISUAL LINE --",
        }
    }
}

/// What becomes of the range when the cursor is set ([`moved`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Shift with a click or an arrow, or a drag: a rectangle from its
    /// anchor, or from the cell the cursor was on, to the new cell. Rows
    /// stay rows, to the new cell's row.
    Extend,
    /// A move by key: a held range follows, another ends.
    Move,
    /// Anything else ends it: a click, an editor that opens, a new row, a
    /// page that goes.
    End,
    /// The rows under the cursor changed, not what is selected: a new row
    /// was dropped.
    Keep,
}

/// How a row number was clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowClick {
    /// That row alone.
    Only,
    /// With Shift: the rows from the anchor to it.
    Extend,
    /// With Command: that row in or out, the others kept.
    Toggle,
}

impl Range {
    /// The terminal look's mode this range is, where it is held.
    pub fn visual(&self) -> Option<Visual> {
        match self {
            Self::Cells {
                held: Held::Visual, ..
            } => Some(Visual::Cells),
            Self::Cells {
                held: Held::Block, ..
            } => Some(Visual::Block),
            Self::Cells { held: Held::No, .. } => None,
            Self::Rows { held, .. } => held.then_some(Visual::Rows),
        }
    }

    /// The row the range runs from, where it has one.
    pub fn anchor_row(&self) -> Option<usize> {
        match self {
            Self::Cells { anchor, .. } => Some(anchor.row),
            Self::Rows { anchor, .. } => *anchor,
        }
    }

    /// Whether the range is of whole rows.
    pub fn rows(&self) -> bool {
        matches!(self, Self::Rows { .. })
    }

    /// Takes the rows `gone` names out of the range: they were dropped. An
    /// anchor among them moves to `instead`'s answer for it, or goes where
    /// there is none: a rectangle then ends, and rows keep what is picked.
    /// Rows left with nothing selected end too.
    pub fn without(
        self,
        gone: impl Fn(usize) -> bool,
        instead: impl Fn(usize) -> Option<usize>,
    ) -> Option<Self> {
        let moved = |row: usize| if gone(row) { instead(row) } else { Some(row) };
        match self {
            Self::Cells { anchor, held } => {
                let row = moved(anchor.row)?;
                let anchor = CellPos { row, ..anchor };
                Some(Self::Cells { anchor, held })
            }
            Self::Rows {
                anchor,
                picked,
                held,
            } => {
                let picked: BTreeSet<usize> =
                    picked.into_iter().filter(|row| !gone(*row)).collect();
                let anchor = anchor.and_then(moved);
                (anchor.is_some() || !picked.is_empty()).then_some(Self::Rows {
                    anchor,
                    picked,
                    held,
                })
            }
        }
    }
}

/// The range after the cursor went from `was` to `cell`.
pub fn moved(
    range: Option<Range>,
    was: Option<CellPos>,
    cell: CellPos,
    reach: Reach,
) -> Option<Range> {
    let range = match reach {
        Reach::Extend => match range {
            Some(Range::Rows {
                anchor,
                picked,
                held,
            }) => Some(Range::Rows {
                anchor: anchor.or(was.map(|was| was.row)),
                picked,
                held,
            }),
            Some(cells @ Range::Cells { .. }) => Some(cells),
            None => was.map(|anchor| Range::Cells {
                anchor,
                held: Held::No,
            }),
        },
        Reach::Move => range.filter(|range| range.visual().is_some()),
        Reach::End => None,
        Reach::Keep => range,
    };
    // One cell that is not held is no range.
    range.filter(
        |range| !matches!(range, Range::Cells { anchor, held: Held::No } if *anchor == cell),
    )
}

/// The range after the terminal look's `v`, `ctrl+v` or `V` on the cursor
/// `cell`: the mode it is in is left, another becomes it, from the anchor
/// that stands.
pub fn toggled(range: Option<Range>, cell: CellPos, mode: Visual) -> Option<Range> {
    if range.as_ref().and_then(Range::visual) == Some(mode) {
        return None;
    }
    let anchor = range.as_ref().and_then(Range::anchor_row);
    Some(match mode {
        Visual::Cells | Visual::Block => {
            let anchor = match range {
                Some(Range::Cells { anchor, .. }) => anchor,
                _ => CellPos {
                    row: anchor.unwrap_or(cell.row),
                    col: cell.col,
                },
            };
            let held = if mode == Visual::Block {
                Held::Block
            } else {
                Held::Visual
            };
            Range::Cells { anchor, held }
        }
        Visual::Rows => as_rows(range, cell, true),
    })
}

/// The range made of the rows the selection covers: a rectangle's rows, or
/// the cursor's row. `⇧Space`, and `V` with `held`.
pub fn as_rows(range: Option<Range>, cell: CellPos, held: bool) -> Range {
    match range {
        Some(Range::Rows { anchor, picked, .. }) => Range::Rows {
            anchor,
            picked,
            held,
        },
        Some(Range::Cells { anchor, .. }) => Range::Rows {
            anchor: Some(anchor.row),
            picked: BTreeSet::new(),
            held,
        },
        None => Range::Rows {
            anchor: Some(cell.row),
            picked: BTreeSet::new(),
            held,
        },
    }
}

/// The cursor and the range after a click on the number of `row`. The
/// cursor goes to the row, in the column it was in, unless a Command-click
/// took that row out: then it goes to the nearest row left, so the
/// cursor's row is always a selected one.
pub fn row_clicked(
    range: Option<Range>,
    cursor: Option<CellPos>,
    row: usize,
    click: RowClick,
    order: Order<'_>,
) -> (CellPos, Option<Range>) {
    let col = cursor.map_or(0, |cell| cell.col);
    let mut cell = CellPos { row, col };
    let range = match click {
        RowClick::Only => Some(Range::Rows {
            anchor: Some(row),
            picked: BTreeSet::new(),
            held: false,
        }),
        RowClick::Extend => {
            let (anchor, picked) = match range {
                Some(Range::Rows { anchor, picked, .. }) => (anchor, picked),
                Some(Range::Cells { anchor, .. }) => (Some(anchor.row), BTreeSet::new()),
                None => (None, BTreeSet::new()),
            };
            Some(Range::Rows {
                anchor: anchor.or(cursor.map(|cell| cell.row)).or(Some(row)),
                picked,
                held: false,
            })
        }
        RowClick::Toggle => {
            let mut picked: BTreeSet<usize> = match cursor {
                Some(cursor) => Selection {
                    cursor,
                    range: range.as_ref(),
                    order,
                }
                .rows()
                .into_iter()
                .collect(),
                None => BTreeSet::new(),
            };
            if !picked.remove(&row) {
                picked.insert(row);
            } else if let Some(at) = order.place(row) {
                let near = picked
                    .iter()
                    .filter_map(|picked| Some((order.place(*picked)?.abs_diff(at), *picked)))
                    .min();
                if let Some((_, nearest)) = near {
                    cell.row = nearest;
                }
            }
            // None left: the cursor's cell alone, as with no range.
            (!picked.is_empty()).then_some(Range::Rows {
                anchor: None,
                picked,
                held: false,
            })
        }
    };
    (cell, range)
}

/// Every row the grid shows, for `⌘A`.
pub fn all(order: Order<'_>) -> Option<Range> {
    let picked: BTreeSet<usize> = (0..order.len())
        .filter_map(|place| order.row(place))
        .collect();
    (!picked.is_empty()).then_some(Range::Rows {
        anchor: None,
        picked,
        held: false,
    })
}

/// A rectangle of cells as places of the grid: the first and the last row
/// and column, and where its anchor stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    pub rows: (usize, usize),
    pub cols: (usize, usize),
    pub anchor: CellPos,
}

impl Block {
    pub fn covers(&self, place: usize, col: usize) -> bool {
        (self.rows.0..=self.rows.1).contains(&place) && (self.cols.0..=self.cols.1).contains(&col)
    }
}

/// A grid's selection, read: its cursor, its range and its order.
#[derive(Clone, Copy)]
pub struct Selection<'a> {
    pub cursor: CellPos,
    pub range: Option<&'a Range>,
    pub order: Order<'a>,
}

impl Selection<'_> {
    /// The places the selection's rows stand at, first to last.
    pub fn places(&self) -> Vec<usize> {
        let place = |row: usize| self.order.place(row);
        let Some(range) = self.range else {
            return place(self.cursor.row).into_iter().collect();
        };
        let span = |from: usize| -> Vec<usize> {
            match (place(from), place(self.cursor.row)) {
                (Some(a), Some(b)) => (a.min(b)..=a.max(b)).collect(),
                _ => Vec::new(),
            }
        };
        match range {
            Range::Cells { anchor, .. } => span(anchor.row),
            Range::Rows { anchor, picked, .. } => {
                let mut places: BTreeSet<usize> =
                    picked.iter().filter_map(|row| place(*row)).collect();
                places.extend(anchor.map(span).unwrap_or_default());
                places.into_iter().collect()
            }
        }
    }

    /// The rows the selection covers, in the grid's order. With no range,
    /// the cursor's.
    pub fn rows(&self) -> Vec<usize> {
        let places = self.places();
        places
            .into_iter()
            .filter_map(|place| self.order.row(place))
            .collect()
    }

    /// The first and the last column the selection covers, of a grid
    /// `width` wide: a rectangle's, every one for rows, the cursor's alone
    /// with no range. `None` for a grid with no columns.
    pub fn cols(&self, width: usize) -> Option<(usize, usize)> {
        let last = width.checked_sub(1)?;
        let col = self.cursor.col.min(last);
        Some(match self.range {
            None => (col, col),
            Some(Range::Cells { anchor, .. }) => {
                let from = anchor.col.min(last);
                (from.min(col), from.max(col))
            }
            Some(Range::Rows { .. }) => (0, last),
        })
    }

    /// How many rows and columns the range covers, where one stands.
    pub fn size(&self, width: usize) -> Option<(usize, usize)> {
        self.range?;
        let (first, last) = self.cols(width)?;
        Some((self.places().len(), last - first + 1))
    }

    /// The rectangle a range of cells covers, as places.
    pub fn block(&self, width: usize) -> Option<Block> {
        let Some(Range::Cells { anchor, .. }) = self.range else {
            return None;
        };
        let places = self.places();
        let rows = (*places.first()?, *places.last()?);
        let cols = self.cols(width)?;
        let row = self.order.place(anchor.row)?;
        Some(Block {
            rows,
            cols,
            anchor: CellPos {
                row,
                col: anchor.col,
            },
        })
    }

    /// The places of whole rows a range of rows holds.
    pub fn ranged(&self) -> BTreeSet<usize> {
        match self.range {
            Some(Range::Rows { .. }) => self.places().into_iter().collect(),
            _ => BTreeSet::new(),
        }
    }
}

/// What the selection's size is said as: "4 rows × 2 columns", or "3
/// rows" for whole rows. In English, the words being the app's own.
pub fn size_words(
    locale: crate::i18n::Locale,
    (rows, cols): (usize, usize),
    whole_rows: bool,
) -> String {
    let rows = crate::i18n::counted(locale, rows, "row", "rows");
    if whole_rows {
        return rows;
    }
    let cols = crate::i18n::counted(locale, cols, "column", "columns");
    format!("{rows} × {cols}")
}

/// What a screen reader is told of a range: "4 rows by 2 columns
/// selected", or "3 rows selected" for whole rows.
pub fn said(locale: crate::i18n::Locale, (rows, cols): (usize, usize), whole_rows: bool) -> String {
    let rows = crate::i18n::counted(locale, rows, "row", "rows");
    let selected = crate::i18n::gettext(locale, "selected");
    if whole_rows {
        return format!("{rows} {selected}");
    }
    let cols = crate::i18n::counted(locale, cols, "column", "columns");
    let by = crate::i18n::gettext(locale, "by");
    format!("{rows} {by} {cols} {selected}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::{Edits, Place, new_row};

    fn at(row: usize, col: usize) -> CellPos {
        CellPos { row, col }
    }

    /// Three rows of a page, and a new one below the first: the new row's
    /// own, and the edits that hold it.
    fn with_new_row() -> (Edits, usize) {
        let mut edits = Edits::default();
        let id = edits.add_row(Place::Below(0), 3);
        (edits, new_row(id))
    }

    fn read<'a>(cursor: CellPos, range: Option<&'a Range>, order: Order<'a>) -> Selection<'a> {
        Selection {
            cursor,
            range,
            order,
        }
    }

    #[test]
    fn shift_makes_a_rectangle_from_where_the_cursor_was() {
        let order = Order::of(&[], 6);
        let range = moved(None, Some(at(1, 1)), at(3, 2), Reach::Extend);
        assert_eq!(
            range,
            Some(Range::Cells {
                anchor: at(1, 1),
                held: Held::No
            })
        );
        let read = read(at(3, 2), range.as_ref(), order);
        assert_eq!(read.rows(), [1, 2, 3]);
        assert_eq!(read.cols(5), Some((1, 2)));
        assert_eq!(read.size(5), Some((3, 2)));
        assert_eq!(
            read.block(5),
            Some(Block {
                rows: (1, 3),
                cols: (1, 2),
                anchor: at(1, 1)
            })
        );
        // The anchor stays where the range began.
        let range = moved(range, Some(at(3, 2)), at(0, 0), Reach::Extend);
        let read = self::read(at(0, 0), range.as_ref(), order);
        assert_eq!((read.rows(), read.cols(5)), (vec![0, 1], Some((0, 1))));
        // Back on its anchor it is one cell, and no range.
        assert_eq!(moved(range, Some(at(0, 0)), at(1, 1), Reach::Extend), None);
    }

    #[test]
    fn with_no_cursor_shift_selects_one_cell() {
        assert_eq!(moved(None, None, at(2, 0), Reach::Extend), None);
        let read = read(at(2, 0), None, Order::of(&[], 4));
        assert_eq!(
            (read.rows(), read.cols(3), read.size(3)),
            (vec![2], Some((0, 0)), None)
        );
    }

    #[test]
    fn a_move_ends_a_range_unless_it_is_held() {
        let cells = moved(None, Some(at(1, 1)), at(2, 1), Reach::Extend);
        assert_eq!(moved(cells, Some(at(2, 1)), at(3, 1), Reach::Move), None);
        for held in [Held::Visual, Held::Block] {
            let range = Some(Range::Cells {
                anchor: at(1, 1),
                held,
            });
            // On its anchor it is still a range: it is held.
            let kept = moved(range.clone(), Some(at(2, 1)), at(1, 1), Reach::Move);
            assert_eq!(kept, range);
            assert_eq!(moved(range, Some(at(2, 1)), at(1, 2), Reach::End), None);
        }
        let rows = Some(as_rows(None, at(1, 0), true));
        assert_eq!(
            moved(rows.clone(), Some(at(1, 0)), at(4, 0), Reach::Move),
            rows
        );
        let loose = Some(as_rows(None, at(1, 0), false));
        assert_eq!(moved(loose, Some(at(1, 0)), at(4, 0), Reach::Move), None);
        // Keep leaves it as it is.
        let kept = moved(rows.clone(), Some(at(1, 0)), at(2, 0), Reach::Keep);
        assert_eq!(kept, rows);
    }

    #[test]
    fn shift_extends_rows_as_rows() {
        let order = Order::of(&[], 6);
        let (cell, range) = row_clicked(None, Some(at(4, 2)), 1, RowClick::Only, order);
        assert_eq!(cell, at(1, 2));
        let range = moved(range, Some(cell), at(3, 2), Reach::Extend);
        let read = read(at(3, 2), range.as_ref(), order);
        assert_eq!((read.rows(), read.cols(4)), (vec![1, 2, 3], Some((0, 3))));
        assert_eq!(read.size(4), Some((3, 4)));
        assert_eq!(read.block(4), None);
        assert_eq!(read.ranged(), BTreeSet::from([1, 2, 3]));
    }

    #[test]
    fn a_row_number_selects_extends_and_toggles_rows() {
        let order = Order::of(&[], 8);
        let (cell, range) = row_clicked(None, None, 2, RowClick::Only, order);
        assert_eq!(cell, at(2, 0));
        assert_eq!(read(cell, range.as_ref(), order).rows(), [2]);
        // One row is a range of rows: it covers every column.
        assert_eq!(read(cell, range.as_ref(), order).size(3), Some((1, 3)));
        let (cell, range) = row_clicked(range, Some(cell), 4, RowClick::Extend, order);
        assert_eq!(read(cell, range.as_ref(), order).rows(), [2, 3, 4]);
        // Command takes one out, and puts one in apart from the others.
        let (cell, range) = row_clicked(range, Some(cell), 3, RowClick::Toggle, order);
        assert_eq!(read(cell, range.as_ref(), order).rows(), [2, 4]);
        // The cursor goes to the row clicked, and with that row taken out to
        // the nearest left.
        assert_eq!(cell.row, 2);
        let (cell, range) = row_clicked(range, Some(cell), 7, RowClick::Toggle, order);
        assert_eq!(read(cell, range.as_ref(), order).rows(), [2, 4, 7]);
        // Shift then runs from the cursor's row, the picked rows kept.
        let (cell, range) = row_clicked(range, Some(cell), 5, RowClick::Extend, order);
        assert_eq!(read(cell, range.as_ref(), order).rows(), [2, 4, 5, 6, 7]);
        // Taking out the cursor's row between two others: the nearer one.
        let (cell, range) = row_clicked(range, Some(cell), 5, RowClick::Toggle, order);
        assert_eq!(
            (cell.row, read(cell, range.as_ref(), order).rows()),
            (4, vec![2, 4, 6, 7])
        );
        // The last one out leaves no range.
        let (cell, range) = row_clicked(None, Some(at(1, 0)), 1, RowClick::Toggle, order);
        assert_eq!((cell, range), (at(1, 0), None));
    }

    #[test]
    fn command_on_a_row_with_no_range_adds_it_to_the_cursors() {
        let order = Order::of(&[], 8);
        let (cell, range) = row_clicked(None, Some(at(1, 3)), 5, RowClick::Toggle, order);
        assert_eq!(cell, at(5, 3));
        assert_eq!(read(cell, range.as_ref(), order).rows(), [1, 5]);
        // A rectangle's rows count as selected rows.
        let cells = moved(None, Some(at(0, 0)), at(1, 1), Reach::Extend);
        let (cell, range) = row_clicked(cells, Some(at(1, 1)), 3, RowClick::Toggle, order);
        assert_eq!(read(cell, range.as_ref(), order).rows(), [0, 1, 3]);
    }

    #[test]
    fn select_all_picks_every_row_with_new_ones_among_them() {
        let (edits, new) = with_new_row();
        let order = Order::of(&edits.added, 3);
        let range = all(order);
        let read = read(at(0, 0), range.as_ref(), order);
        assert_eq!(read.rows(), [0, new, 1, 2]);
        assert_eq!(all(Order::of(&[], 0)), None);
    }

    #[test]
    fn a_rectangle_covers_a_new_row_where_it_stands() {
        let (edits, new) = with_new_row();
        let order = Order::of(&edits.added, 3);
        let range = moved(None, Some(at(0, 0)), at(1, 1), Reach::Extend);
        let read = read(at(1, 1), range.as_ref(), order);
        assert_eq!(read.rows(), [0, new, 1]);
        assert_eq!(read.block(2).map(|block| block.rows), Some((0, 2)));
        assert!(read.block(2).is_some_and(|block| block.covers(1, 1)));
        assert!(!read.block(2).is_some_and(|block| block.covers(3, 1)));
    }

    #[test]
    fn the_visual_modes_start_switch_and_leave() {
        let cell = at(2, 1);
        let cells = toggled(None, cell, Visual::Cells);
        assert_eq!(cells.as_ref().and_then(Range::visual), Some(Visual::Cells));
        // One cell held is a range: the status line says its size.
        assert_eq!(
            read(cell, cells.as_ref(), Order::of(&[], 4)).size(3),
            Some((1, 1))
        );
        // Another mode keeps the anchor.
        let block = toggled(cells.clone(), at(3, 2), Visual::Block);
        assert_eq!(
            block,
            Some(Range::Cells {
                anchor: cell,
                held: Held::Block
            })
        );
        let rows = toggled(block, at(3, 2), Visual::Rows);
        assert_eq!(rows.as_ref().and_then(Range::anchor_row), Some(2));
        assert_eq!(rows.as_ref().and_then(Range::visual), Some(Visual::Rows));
        let back = toggled(rows.clone(), at(3, 2), Visual::Cells);
        assert_eq!(
            back,
            Some(Range::Cells {
                anchor: at(2, 2),
                held: Held::Visual
            })
        );
        // The same key again leaves it.
        assert_eq!(toggled(rows, at(3, 2), Visual::Rows), None);
        assert_eq!(toggled(cells, cell, Visual::Cells), None);
        // A rectangle made with Shift becomes the visual one, from its
        // anchor.
        let shifted = moved(None, Some(at(0, 0)), at(1, 1), Reach::Extend);
        assert_eq!(
            toggled(shifted, at(1, 1), Visual::Block),
            Some(Range::Cells {
                anchor: at(0, 0),
                held: Held::Block
            })
        );
    }

    #[test]
    fn shift_space_makes_rows_of_what_is_selected() {
        let order = Order::of(&[], 6);
        let cells = moved(None, Some(at(1, 1)), at(3, 2), Reach::Extend);
        let rows = as_rows(cells, at(3, 2), false);
        assert_eq!(read(at(3, 2), Some(&rows), order).rows(), [1, 2, 3]);
        assert_eq!(rows.visual(), None);
        let one = as_rows(None, at(4, 0), false);
        assert_eq!(read(at(4, 0), Some(&one), order).rows(), [4]);
    }

    #[test]
    fn rows_that_go_leave_the_range() {
        let new = with_new_row().1;
        let rows = Range::Rows {
            anchor: Some(new),
            picked: BTreeSet::from([0, new]),
            held: false,
        };
        let gone = |row: usize| row == new;
        assert_eq!(
            rows.clone().without(gone, |_| Some(1)),
            Some(Range::Rows {
                anchor: Some(1),
                picked: BTreeSet::from([0]),
                held: false
            })
        );
        // Nothing to take the anchor's place: the picked rows stay.
        assert_eq!(
            rows.without(gone, |_| None),
            Some(Range::Rows {
                anchor: None,
                picked: BTreeSet::from([0]),
                held: false
            })
        );
        let only = Range::Rows {
            anchor: None,
            picked: BTreeSet::from([new]),
            held: false,
        };
        assert_eq!(only.without(gone, |_| Some(1)), None);
        let cells = Range::Cells {
            anchor: at(new, 2),
            held: Held::Visual,
        };
        assert_eq!(cells.clone().without(gone, |_| None), None);
        assert_eq!(
            cells.without(gone, |_| Some(3)),
            Some(Range::Cells {
                anchor: at(3, 2),
                held: Held::Visual
            })
        );
    }

    #[test]
    fn the_size_is_said_in_rows_and_columns() {
        let locale = crate::i18n::Locale::default();
        assert_eq!(size_words(locale, (4, 2), false), "4 rows × 2 columns");
        assert_eq!(size_words(locale, (1, 1), false), "1 row × 1 column");
        assert_eq!(size_words(locale, (3, 7), true), "3 rows");
        assert_eq!(said(locale, (4, 2), false), "4 rows by 2 columns selected");
        assert_eq!(said(locale, (1, 5), true), "1 row selected");
    }
}
