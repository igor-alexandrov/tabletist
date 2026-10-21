# The keymap's small unbuilt commands. Implementation Plan

**Goal:** The keymap (`src/keymap.rs`) lists commands with their keys and
`built: false` (`.reserved()`): nothing answers their keys, so a key
Settings › Keyboard shows does nothing when pressed. Build the small ones,
step by step, so each key does what its row says in every look that has a
key for it. Building a command means three things: its `.reserved()` goes
from `Command::info`, its key acts in its scope, and the tests that said
"not built" become tests of what it does.

**Out of scope, left reserved:** the AI assistant (`AskAi`, `AcceptAi`,
`AcceptAiAndRun`, `DiscardAi`), the plan viewer (`Explain`,
`ExplainAnalyze`, `ComparePreviousRun`, `NextPlanTip`), hand-run
transactions (`Commit`, `Rollback`), and the agents' commands that
`agents()` gates. Another branch
(`claude/keyboard-keys-availability-4135f4`) marks those as not built in
Settings › Keyboard, the cheat sheet and the docs. This plan does not touch
how unbuilt rows are shown (`src/ui/settings/keyboard*.rs`): dropping a
command's `.reserved()` is all that display needs.

**Spec:** `docs/superpowers/specs/2026-10-08-keyboard-shortcuts-design.md`
(the keys and their scopes), `2026-10-07-inserting-rows-design.md`
(add row past the end, errors after a save, the paste preview, show the
saved row), `2026-10-03-value-editing-core-design.md` (slice 4: undo and
redo, paste over cells, `$EDITOR`), `2026-10-12-foreign-key-label-design.md`
(the label's second part, reserved there), and the design canvas
(OmarchySql: "ctrl+j focus results"; MacErrors: "Go to line ⌘L" beside a
query's error; MacInsertStates: "On the last row, ↓ past the end adds an
empty row"; OmarchyInsert: "]e next error", "gs jumps to its sorted
position"; OmarchyEditTypes: "ctrl+a / ctrl+x add / subtract", "ctrl+e
opens $EDITOR").

**Tech stack:** Rust 2024, egui 0.36 (the fork) with AccessKit. No new
dependency.

## Steps

Each step is one pull request against `main`, and stands alone. The plan
ends a run at the end of a step.

| Step | Commands | Size | Open questions |
|---|---|---|---|
| 1 | `PaneUp`, `PaneDown`, `FocusResults`, `AddRowAtEnd`, `NextError`, `PreviousError` | small | none |
| 2 | `GoToLine`, `StepUp`, `StepDown` | medium | what ⌘L opens on the Mac where no error names a line |
| 3 | `ReopenClosedTab`, `NextChange`, `ShowSavedRow` | medium | which closed tabs reopen (SQL editors are kept, tables only as recents); a cursor in the Review SQL panel for `]c` |
| 4 | `RedoCell` | large | the undo stack of value editing's slice 4, with `UndoCell` moved onto it |
| 5 | `PasteCells`, `PasteNewRows` | large | the paste preview dialog and its TSV/CSV reader (inserting rows, run 4) |
| 6 | `OpenInEditor` | medium | waiting for the terminal editor and reading the file back |
| 7 | `AddLabelPart` | large | no spec yet: the second part is "reserve only" in the label spec |

Steps 4 to 7 each want their own spec or a decision before a plan, and are
planned when they come up.

## Step 1: moving between panes, a row past the end, and the errors

### What each command does

- **Pane below / Pane above** (Omarchy `ctrl+j` / `ctrl+k`, out of a text
  field). The terminal look already steps left and right between its panes
  (`ctrl+h` / `ctrl+l`, `focus::Region::PANES`). The panes stacked in a tab
  are a SQL editor's script over its results: `ctrl+j` takes the keyboard
  from the script to the results, `ctrl+k` from the results back to the
  script. Nothing is below the results or above the script, and a table's
  rows have nothing above or below: there the keys do nothing. The other
  looks have no key for either.
- **Focus results** (Omarchy `ctrl+j` in the SQL editor, while typing). The
  same move as Pane below, from inside the script, as the canvas's footer
  says ("ctrl+j focus results"). The key types nothing.
- **Add row after the last** (the Mac's and Windows' ↓ on the last row).
  On the table's last row (the last row the grid shows, on the last page),
  ↓ adds a new row below it, as Add row does there: the editor opens on
  the first cell a save needs, else the cursor goes to the row's first cell
  that takes a value. Only where the table takes rows; elsewhere ↓ on the
  last row stays there, as it does now. A fresh press only: a held ↓ that
  reaches the end does not go on adding rows. Omarchy has no key: `j` on
  the last row adds nothing (inserting rows, section 1).
- **Next error / Previous error** (⌥⌘↓ / ⌥⌘↑, Omarchy `]e` / `[e`).
  - In a table's grid: the errors are the cells a save refuses to send
    (`State::ToFix`), the cells of a row whose statement failed
    (`State::Failed`), a new row whose `INSERT` failed with nothing set in
    it, and a row whose `DELETE` failed. They are taken in the grid's order
    (row by row, cell by cell), and the cursor goes to the next one after
    it, or the one before it. Past the last it starts again at the first,
    and before the first at the last. A row's own error is on its first
    cell. With no error the keys do nothing.
  - The terminal look's brackets step the row panel's row at once while
    the panel shows (`[`, `]`), and the panel shows by default, so `]e`
    would never get its `e`. Decided with the user: while the panel shows
    and there is an error to go to (a grid's error, or a SQL editor's
    mark), a bracket waits for the key after it. `e` goes to the error;
    any other key steps the row and is then read itself, except a key that
    edits and `:`, which are dropped: the step is applied after the frame,
    so an edit read in it would land on the row left. With no error the
    bracket steps the row at once, as before.
  - In a SQL editor: the editor's one error mark (`SqlTab::error_mark`, the
    line and column the database named). Either key puts the editor's
    cursor there and gives it the keyboard. With no mark, nothing.

### Files

| File | What changes |
|---|---|
| `src/keymap.rs` | The six commands lose `.reserved()`; the comments that said why go |
| `src/ui/focus.rs` | `Region::STACKED`, the panes one above the other: the script and the rows |
| `src/ui/keys.rs` | `ctrl+j` and `ctrl+k` read beside `ctrl+h` and `ctrl+l`; Focus results in the SQL editor's keys; ↓ on the last row before the moves; the error keys in the grid's chords, in the letters, and in the SQL editor's keys |
| `src/model.rs` | `Action::NextError { tab, id, forward }`, `Action::SqlErrorMark { tab, sql_tab }`; `SqlTab::go_to`, a cursor the editor takes on its next frame |
| `src/edit.rs` | `Edits::errors(order)`: the places of the errors, in the grid's order |
| `src/app.rs` | The two actions applied |
| `src/ui/sql_text.rs` | The editor puts its cursor at `go_to` and takes the keyboard |
| `src/ui/pane_keys_tests.rs` (new), `src/ui/grid_keys_tests.rs` (new), `src/ui/mod.rs` | The tests, and their modules |
| `src/ui/insert_audit_tests.rs` | The audit's INS-01d, INS-33a and INS-33b are no longer ignored |
| `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md` | The keys that now work, where the pages said they did not |

### Tasks

1. **Panes.** Tests first (`pane_keys_tests.rs`): in the Omarchy look, a SQL
   editor that ran has the keyboard in its script; `ctrl+j` puts it on the
   results (the focused widget is "Rows"), types nothing, and `j` then
   moves the results' cursor; `ctrl+k` puts it back in the script; after
   Esc, `ctrl+j` (now Pane below) goes down again; `ctrl+j` on the results
   and `ctrl+k` in the script stay where they are. The macOS and standard
   looks leave both keys to the script. A table's rows stay where they are.
   Then the code: `focus::Region::STACKED = [Editor, Grid]`, and in
   `ui::keys`, Pane below / Pane above (and Focus results while the script
   has the keyboard) step among the stacked regions in a SQL editor's tab
   only, by `focus::step` with that list, the way left and right step among
   `PANES`. Drop the three `.reserved()`.
2. **A row past the end.** Tests first (`grid_keys_tests.rs`, and the
   audit's INS-01d no longer ignored): on the Mac look, with a writable
   table's last row selected, ↓ adds a new row below it, and the editor or
   the cursor is on it; a held ↓ adds one row; ↓ on another row moves as
   before; on a read-only table ↓ on the last row adds nothing and stays;
   with more pages to come it adds nothing; Omarchy's `j` on the last row
   adds nothing. Then the code: before the moves in `ui::keys`, a fresh
   Add row after the last on the last row of the last page of a table
   that takes rows sends `Action::AddRow` with `Place::Below(last)`. Drop
   the `.reserved()`.
3. **Errors.** Tests first (`grid_keys_tests.rs`): after a save that
   failed on one row, ⌥⌘↓ (Mac) and `]e` (Omarchy) put the cursor on the
   failed row's first failing cell, and again on the next, round to the
   first; ⌥⌘↑ and `[e` go the other way; cells that fail their check count
   too, in grid order; a new row whose insert failed and a row whose delete
   failed are reached on their first cell; with no error the cursor stays.
   In a SQL editor whose run failed with a position, either key puts the
   editor's cursor on the mark's line and column and the keyboard in the
   script; with no mark the cursor stays. Then the code: `Edits::errors`,
   the two actions, `SqlTab::go_to` taken by the editor, the keys read in
   the grid (chords and letters) and the SQL editor (chords and letters).
   Drop the two `.reserved()`.
4. **Pages.** The website's keyboard page and the two guides say what the
   six keys do.

Each task ends with the four checks of `AGENTS.md` passing and a commit.

### Verification

The four checks of `AGENTS.md`. By hand, on a real backend (the demo
database), if a display is free: the Omarchy look's `ctrl+j` / `ctrl+k` in
a SQL editor, ↓ past a table's last row on the macOS look, and `]e` after a
save that fails a unique key.
