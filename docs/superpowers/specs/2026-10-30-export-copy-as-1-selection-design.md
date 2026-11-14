# Export and Copy as, run 1: the grid's selection

Export (a table, a selection or a result to a file or the clipboard) and
Copy as (a range in a chosen format) need a selection the grid does not
have yet. Today a table's grid holds a cursor and a contiguous range of
whole rows (`RowRange { anchor, held }` in `src/model.rs`): ⇧-click and
⇧↑ ⇧↓ on macOS, `V` on Omarchy. A SQL result holds a cursor alone. This run
builds the selection both grids share, which runs 2 to 4 copy and export.

Design canvas: **Tabletist**, section **Export and Copy as**, boards
`macOS – Copy as` and `Omarchy – Copy as` (the range, the block, the
status line), and section **Components**, `macOS – Components, additions
for Export` (Cell range) and `Omarchy – Components, additions for Export`
(visual block).

## Delivery: four runs, a pull request each

Asked and answered on 2026-10-30 (the prompt asked for one pull request):

1. **Run 1 (this spec): the selection.** Cells, blocks and rows in both
   grids; the row-number column; the keys and the clicks; drawing; the
   size in the footer, the status line and to a screen reader; clearing;
   delete and duplicate acting on it; ⌘C and `y` copying it as TSV.
2. **Run 2: writers and Copy as.** The six format writers with golden
   tests, options remembered per format, the Copy as menu (⌥⌘C, `gy`),
   the inspector's Copy row ▾, the clipboard limit.
3. **Run 3: export a table.** The sheet and the float, the background
   export, its states, and the entry points of a table.
4. **Run 4: export a SQL result.** Run again without the limit, the
   results bar, the inspector's Export row…, several exports at once, the
   portal picker and path completions.

## Decided with Igor before this spec (2026-10-30)

- **Row numbers:** a row-number column in both looks, as the Copy as
  boards draw it: 44 wide in `ink-faint` on macOS, 50 wide in `ink-muted`
  in the terminal look. The number is the row's among the table's (the
  page's offset plus its place, from 1). Clicking one selects the row.
- **Omarchy `v`:** a rectangle, as `ctrl+v` is. The status line says
  `-- VISUAL --` or `-- VISUAL BLOCK --`. So every selection is rows ×
  columns, and "4 rows × 2 columns" always holds.
- **SQL results:** the same selection in this run.

## Where this build differs from the boards or the prompt

To approve or overrule before code:

| The board or the prompt | This build | Why |
|---|---|---|
| ⇧-click and ⇧↑ ⇧↓ select whole rows today | They select a rectangle of cells, as the prompt says. ⌘⌫ (Delete row) on a rectangle deletes the rows it covers, so ⇧↓ then ⌘⌫ still deletes those rows | The prompt's model. Delete keeps working on what the person reached with Shift |
| The Copy as boards draw the row numbers in a column of their own | The numbers stand in the grid's gutter, beside the terminal look's cursor and marks. They stay in sight with a pinned key column and scroll with the rows otherwise | The gutter already does that. A second column that is pinned alone would need a scroll split the grid lacks |
| `V` shows `-- VISUAL --` | `V` shows `-- VISUAL LINE --`, as the DDL view's `V` does. `v` shows `-- VISUAL --`, `ctrl+v` `-- VISUAL BLOCK --` | Vim's three names, so the three modes can be told apart |
| `d` in visual | `d` deletes the rows in `V` alone. In `v` and `ctrl+v` it does nothing, and the status line's keys are `y copy · esc normal` | Deleting the rows of a block of cells is a surprise. The block's `d` waits for a meaning |
| Duplicate acts on "selected rows" | ⌘D (Duplicate row) with several rows selected copies each of them, in the grid's order, at the top. The editor opens on the first copy's first value it needs, as for one copy | The prompt's rule. One editor, so the first copy |
| Set to acts on the selection | Not wired: Set to has not landed on `main`. Its own runs read this selection | Nothing to wire yet |
| ⌘C copies the selection as TSV (prompt, run 2) | Built in this run, with today's TSV writing (`format::tsv_row`). Run 2 moves it onto its TSV writer | A selection that ⌘C copies one cell of reads as broken |
| `v p` pastes into the cursor's cell | `v` starts the selection and `p` in visual pastes into the cursor's cell, as `v p` does today | `v` is now a mode, not the first key of a chord. The same keys do the same thing |

## The model

`src/selection.rs`, no UI. A grid's selection is its cursor (`CellPos`,
unchanged) and a `Range` beside it:

```rust
pub enum Range {
    /// The rectangle of cells from `anchor` to the cursor.
    Cells { anchor: CellPos, held: Held },
    /// Whole rows: those in `picked`, and those from `anchor` to the
    /// cursor's row where there is an anchor.
    Rows { anchor: Option<usize>, picked: BTreeSet<usize>, held: bool },
}

/// A rectangle's: moves end it (`No`), or keep it (`v`, `ctrl+v`).
pub enum Held { No, Visual, Block }

/// The terminal look's modes, as `Range::visual` reads them: `v`, `ctrl+v`
/// and `V` (rows with `held`).
pub enum Visual { Cells, Block, Rows }
```

Rows are named as `CellPos::row` names them (a page's row, or a new row
by its id). Places are the grid's order. A table's grid orders them with
`edit::Order` (new rows among the page's). A result's order is its rows'.
`Range` asks an `Order` for places when it is read.

What it answers, every answer in the grid's order:

- `rows()`: the rows the selection covers: a rectangle's, or the picked
  and anchored rows. With no range, the cursor's row.
- `cols(width)`: the columns, first to last: a rectangle's, or every
  column for rows.
- `size()`: rows × columns, for "4 rows × 2 columns".
- `covers(place, col)`: for drawing.

How each input changes it (`Reach` grows from four cases to these):

| Input | macOS | Omarchy | Range after |
|---|---|---|---|
| Click a cell | click | click | none |
| ⇧-click a cell, ⇧+arrow | ⇧-click, ⇧↑ ⇧↓ ⇧← ⇧→ | ⇧-click | `Cells` from the anchor (or the cursor before) to the new cell, not held. Over a range of rows, rows from its anchor to the new cell's row |
| Drag over cells | drag | drag | `Cells` from the pressed cell to the one under the pointer |
| Click a row number | click | click | `Rows { anchor: that row }` |
| ⇧-click a row number | ⇧-click | ⇧-click | `Rows`: the picked rows kept, anchor to that row |
| ⌘-click a row number | ⌘-click | (no key) | `Rows`: what was selected, as rows, with that row toggled; no anchor. Taken out, the cursor's row gives the cursor to the nearest row left |
| Select the cursor's rows | ⇧Space | `V` | `Rows` of what was selected (a rectangle's rows) or the cursor's row. `V` holds it |
| Select all | ⌘A | (none) | `Rows` with every loaded row picked |
| Visual cells | (none) | `v` | `Cells { anchor: cursor, held: Visual }` |
| Visual block | (none) | `ctrl+v` | `Cells { anchor: cursor, held: Block }` |
| A move by key | arrows | `h j k l` and the rest | a held range follows the cursor; another ends |
| Esc | Esc | `esc` | a held range ends (`LeaveVisual`) |
| Filter, sort, page, refresh | | | none (as today: `set_cursor(None, End)`) |
| Another view (Structure, DDL) | | | a visual selection ends; a range made with Shift or a click stays, as today |

One cell is no range: `Cells` whose anchor is the cursor, not held, is
dropped, as a one-row `RowRange` is today. A held one stays, so `v` shows
`-- VISUAL --` and "1 row × 1 column".

A new row dropped from the grid takes its place out of the range. An
anchor that goes moves to the nearest row the action named that is left,
as `RowRange`'s does today (`src/app/editing.rs`); with none left a
rectangle ends, and rows keep what is picked.

## Drawing

- **Rows:** as today: the rows of a `Rows` range are `ranged` (the
  selected row's fill and bar, selected to a screen reader).
- **Cells, macOS:** each covered cell is filled `accent-faint`, the
  rectangle has a 1-point `accent` outline on its outer edges, and the
  cursor's cell keeps its 2-point ring. `accent-faint` is a new palette
  colour from the design system's tokens: light `#eef2fc`, dark `#2a3248`.
- **Cells, terminal look:** each covered cell is filled `surface-active`,
  the anchor has a 1-point `accent` outline, and the cursor stays reverse
  video.
- **Row numbers:** right-aligned, in the role of a header's type line
  (`ColumnType`, the small text; the terminal's body text), in the
  palette's `faint` on macOS and `dim` in the terminal look (the design's
  `ink-faint` and `ink-muted`). A new row has no number. The header over
  them is blank.

## Saying it

- **macOS footer:** "12 rows selected" for rows (as today) and "4 rows ×
  2 columns selected" for a rectangle. One cell: as today. A SQL result's
  footer keeps its rows and time; a screen reader is told its range.
- **Omarchy status line:** the mode (`-- VISUAL --`, `-- VISUAL BLOCK --`,
  `-- VISUAL LINE --`) and the size ("4 rows × 2 columns", or "3 rows"
  for `V`), then the mode's keys (`d delete · esc normal` for `V`,
  `y copy · esc normal` for the others).
- **Screen reader:** the grid's group says "4 rows by 2 columns selected"
  (or "12 rows selected") while a range stands. The rows of a `Rows`
  range are selected nodes, as today.

## Acting on it

- **Delete row** (⌘⌫, `d` in `V`): every row of `rows()`. One row is
  `delete_row`'s; several are `sweep_rows`'s, as today.
- **Duplicate row** (⌘D): each row of `rows()`, see the table above.
- **Copy** (⌘C; `y` in visual): `rows()` × `cols()` as TSV, each row a
  line, what is pending in place of what was loaded. One cell, alone or
  held by `v`, copies its text as today, tabs and lines kept. In a result,
  the same over its rows.
- **Copy row** (⇧⌘C): every row of `rows()`, whole, as TSV.
- **Paste** (⌘V, `p` in visual): into the cursor's cell, as today.

## Keys

New in `src/keymap.rs`, with their scopes, so Settings › Keyboard lists
them. The keymap's generated tests check every pair:

| Command | Scope | macOS | Omarchy |
|---|---|---|---|
| `ExtendLeft`, `ExtendRight` | Grid | ⇧← ⇧→ | (none) |
| `SelectRowsOfCursor` | Grid | ⇧Space | (the existing `SelectRows`, `V`) |
| `SelectAllRows` | Grid | ⌘A | (none) |
| `SelectCells` | Grid | (none) | `v` |
| `SelectBlock` | Grid | (none) | `ctrl+v` |

Changed: `CopyCells` and `PasteCells` on Omarchy, from the chords `v y`
and `v p` to `y` and `p` while visual (`When::Visual`). `DeleteSelection`
(`d`) holds in `V` only (`When::VisualRows`). ExtendUp and ExtendDown keep
⇧↑ ⇧↓ and now extend a rectangle; ExtendUp and ExtendDown also hold in a
result's grid.

## Tests (headless, no pixels)

Bookshop data only.

1. `src/selection.rs` unit tests: every input in the table above; `rows`,
   `cols`, `size` and `covers` with new rows among the page's; one cell
   is no range unless held; a toggle off the last picked row.
2. macOS through the harness: click, ⇧-click, drag, ⇧+arrows, a row
   number's click, ⇧-click and ⌘-click, ⇧Space, ⌘A, in a table (and the
   click, ⇧+arrows, a number, ⇧Space and ⌘A in a result); the footer's
   words; the screen reader's. A macOS range is never held, so Esc has
   nothing of it to leave.
3. Omarchy through the harness: `v`, `ctrl+v`, `V` with `j k h l`, the
   status line's mode and size, `esc`, `v y` and `v p` doing what they
   did, `d` deleting in `V` alone.
4. Clearing: a filter, a sort, a page and a refresh each leave no range;
   another view leaves no visual selection.
5. Acting: delete and duplicate over a rectangle's rows and over picked
   rows; ⌘C and `y` copy a rectangle and rows as TSV, in a table and a
   result.
6. Painting from the source of truth (as the palette tests do): covered
   cells take `accent_faint` on macOS and `surface_active` in the
   terminal look; the row numbers are painted in their colours.

## Out of scope for run 1

Copy as and its formats, the clipboard limit, Export and its entry
points (runs 2 to 4), and Set to (its own runs).
