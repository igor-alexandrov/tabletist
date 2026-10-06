# Editing values, slice 3: the row panel as a row form

Date: 2026-10-06. Status: step 1 (edit a field in place) is built, see
`docs/superpowers/plans/2026-10-06-row-form-fields.md`. Steps 2 and 3 are
designed and not built.

## Intent

A table's values can be edited in its grid (slice 1,
`2026-10-03-value-editing-core-design.md`). The row panel shows the
selected row in full, pending values included, and edits nothing. This
slice lets a user edit a row's values in the panel: where a value has room
to be read whole, it can be changed whole.

Success: on a writable connection a user selects a row, edits several of
its fields in the panel without touching the grid, sees each as pending in
the panel and in the grid at once, and saves them with the same Save, the
same Review SQL and the same questions as an edit made in the grid. A
field that cannot be edited says why. A SQL editor's result row shows the
panel exactly as today, and so do a view and a read-only connection but
for the footer's note, which now gives the reason.

The designs are the "Editing a row" artboard (macOS), the inspectors of the
"Table view, editable" artboards (macOS and Omarchy), "Editing values, flow"
and "editors by type", and the Components artboards, in the design canvas
Artifact. They are not copied into the repository.

### What the design gives, and where this departs from it

The macOS "Editing a row" artboard draws the form: a header that reads
"Row · id 2" over "2 unsaved changes"; every field a caption over a boxed
control; a key column as a grey box with a lock; a field whose cell is
being edited in the grid as a dashed box, "Editing in the grid…"; a pending
field with an amber border and "was print · revert" in its caption line; a
"NULL" checkbox in a nullable field's caption line; no footer. The
read-only inspector has a footer of Edit, Duplicate and Delete with "⌘I
edit". The flow artboard says the keys are "same in grid and inspector".
Omarchy has no artboard of the form: its inspector's head says `e edit`.

This spec keeps the parts and not the mode:

- The artboards imply a mode the panel is put into (Edit, a header and a
  footer that change). Here the panel is editable whenever its row is, and
  Edit is one more way to start.
- The artboard boxes every field at rest and drops previous and next from
  the header. Here a field reads as it does today until it is edited, and
  the header keeps its buttons.
- The artboard's `kind` dropdown, `now` button, JSON line numbers and
  colours and "Valid JSON · 1 line changed" are slice 2. Here every
  editable value is text.
- The artboard draws `created_at` locked and gives no reason. A field is
  locked only by `edit::Table::lock`, and always says why.

## Decisions

| Question | Decision |
|---|---|
| Form or mode | No mode. On a row that can be edited, each editable field of the panel is edited in place. |
| A field at rest | As today: the JSON tree, the array list, tags, Show all, Copy, the follow link. An editable value shows a field's outline under the pointer and with the keyboard on it. |
| The editor | The tab's one editor (`Edits::editor`), drawn in the panel instead of on the cell. Its text becomes the same pending cell. |
| Starting with the pointer | A double-click on the value, or the pencil in the field's caption line. A single click still selects text and folds a tree. |
| Starting with a key | Enter or F2 on a field that has the keyboard. `Mod+I`, on Omarchy `e`, and the footer's Edit: the selected cell's field, or the row's first editable one. |
| Ending | Enter commits and the keyboard stays on the field. Tab and Shift+Tab commit and edit the next or previous editable field. Esc drops (Omarchy: Esc keeps, Ctrl+C drops). |
| Tall values | A field in the panel that grows with its text, with the popover's band under it. |
| NULL | A `NULL` checkbox in the caption line of a nullable, editable field, shown with the pencil; `Mod+Backspace` (Omarchy `x`) as on a cell. |
| Revert | A `revert` link after "was <loaded value>"; `Mod+Z` (Omarchy `u`) as on a cell. |
| Omarchy | The grid's cell cursor stays the only cursor. `e` edits its cell in the panel. |
| Footer | Edit works. Duplicate and Delete stay disabled until slice 5. The note says why a row cannot be edited. |

Rejected:

- A mode, as the artboards draw it, and a mode that stays on while the
  selection moves: more state, two looks of one panel to keep right, and a
  way out to invent.
- Boxed inputs at rest, for every field or for short values only: the
  reading views that fit no box (a JSON tree, an array list, an attachment
  card, Show all) would go or the panel would double in length.
- A text per field, held by the panel as a form's own state: a second
  pending set beside `Edits::cells`, with its own checks, its own unsaved
  mark and its own part in the leaving guard. Values can be a quarter of a
  megabyte each.
- A single click to edit: a click meant to select and copy would start an
  edit. The pencil alone: editing one step further away than in the grid.
- Enter that walks to the next field: nowhere to go from the last, and a
  mistyped Enter lands in the next value. Nothing that walks: slowest for
  a whole row.
- On Omarchy a field cursor of the panel's own (`j`/`k` between fields): a
  second cursor to draw and keep in step with the grid's. Pointer only: the
  keyboard's look would get the least.
- The `NULL` checkbox always drawn: thirty of them in a panel that is
  mostly read. Keys only: no way to NULL with the pointer.
- The grid's popover anchored to the panel's field, or tall values not
  edited from the panel at all: the values that most want the panel's room.
- No footer buttons until slice 5, and the footer without Edit: `Mod+I` and
  `e` would have no control on screen.

## Out of scope

- Slice 2: pickers for enums and CHECK lists, boolean cycling, foreign key
  search, the calendar and `now`, JSON colours, line numbers and "1 line
  changed", array chips, binary from a file, `DEFAULT`, the "Empty ''"
  button.
- Slice 4: the undo and redo stack (`Mod+Z` here reverts one cell, as in
  the grid), paste of a block, the rest of the vim set, `$EDITOR`.
- Slice 5: Duplicate, Delete and Add row. Their buttons stay disabled.
- A SQL editor's result row. It has no table behind it: its panel has no
  pencil, no `NULL`, no footer, as today.
- The green of a saved cell: the grid's alone. After a save that wrote, the
  panel shows the values the database returned.
- A height the tall field remembers, or a handle to resize it.

## The model

### One editor, drawn in one of two places

    pub enum EditorPlace { Grid, Panel }

    pub struct Editor { ..., pub place: EditorPlace }

    Action::EditCell { tab, id, cell, start }   // in the grid, as before
    Action::EditField { tab, id, cell }         // in the panel, from the value
    Action::EditRow { tab, id }

- **`Editor::place`** says which view draws the tab's editor. Nothing else
  about the editor changes: its cell, its text, `large`, `focus`,
  `touched`, `problem`. `close_editor`, which makes a pending cell of an
  editor's text, does not ask where it is. So a text committed in the
  panel is the pending cell a text committed in the grid would be, with
  the same check, and Save, Review SQL, the production confirmation,
  conflicts, the unsaved mark and the leaving guard (`unwritten` counts
  the open editor) need no change.
- **Where the place does count:**
  - *After a commit.* In the grid Enter moves the selection down and Tab
    right or left (`Advance::Down`, `Right`, `Left`). The panel's field
    asks for `Advance::Stay` on Enter, and on Tab until step 3. Step 3 adds
    `Advance::NextField` and `PrevField`, which the reducer performs, and
    only when the commit closed the editor: a text that fails its check
    keeps its field.
  - *Alt+Enter.* `EditorBreak` on an editor in the panel also sets its
    place to `Grid` until step 2, where a tall editor has its only view.
  - *Leaving.* `LeaveEdit` names the cell and the place it leaves, and
    does nothing for any other editor. The click that takes the keyboard
    from a field can ask for another editor in the same frame (a pencil,
    Edit), and a view drawn earlier queues its action first: without the
    name, the `LeaveEdit` of the old field would close the new one.
    `edit_cell` already closes the editor it replaces, with its text kept.
    The name is for the grid's field, which is drawn after the panel. The
    panel's own field can be left for a control of the panel drawn before
    it (the footer's Edit, a caption above), on the very cell it edits. So
    the panel queues what ends its open field (`LeaveEdit`, `CommitEdit`,
    `CancelEdit`) ahead of everything else it asks for in the frame.
  - *After a commit or a cancel.* For an editor in the panel, on macOS
    and Windows, the reducer notes the field the keyboard goes back to
    (`ObjectTab::focus_field`, the column), which the panel takes when it
    draws, as a field takes `Editor::focus`. Not after an edit that was
    left: the keyboard is already where the user put it. Not on Omarchy,
    where the keys are the grid's again.
- **The field is one field.** The panel's field has the id the grid's has
  (`cell_editor::field_id`): `ui/keys.rs` asks that id whether an editor
  has the keyboard, and goes on asking it.
- **The editor's cell is the selected cell,** as `edit_cell` already makes
  it. Editing a field in the panel therefore selects that column's cell of
  the row. The grid is given no editor to draw. To it the cell is a
  selection that moved: it is brought into view once, sideways too, as any
  moved selection is, and its cursor is lit again when the keys are the
  grid's.
- **`Action::EditRow`** is "edit this row in the panel": `Mod+I`, `e` and
  the Edit button. It shows the panel when it is closed, takes the
  selected cell when it can be edited and otherwise the first cell of the
  row that can, in the page's column order, and opens the editor there in
  the panel. With no such cell it asks for the selected cell, which says
  why (below). Under a row lock (below) it shows the panel and records
  the reason for the selected cell: Omarchy's mode line says it, as for a
  cell, and the panel draws no note under a field for a row's reason,
  which its footer gives. With no row selected it does nothing.
- **A tall value in the panel.** `edit::opens_large` decides, as for a
  cell: a JSON column, a line break, more than 256 characters. In step 1
  such an edit asked for in the panel opens in the grid instead (`place`
  becomes `Grid`, the popover on the cell). From step 2 it stays in the
  panel, `large` meaning the tall field there.
- **Closing the panel closes its editor.** `ToggleRowPanel` closes, as
  left, the panel's editor of every table tab of the workspace: the panel
  is one for the workspace, and an editor waits in a tab that is not in
  front. The Data and Structure switch already closes a tab's editor
  (`SetView`), wherever it is drawn.
- **Under a prompt** about pending changes, `EditRow` is dropped with the
  other editing actions (`dropped_under_a_prompt`), and so is
  `ToggleRowPanel`, which can now change the set.

### What can be edited

`edit::Table::lock` stays the only rule. It already asks in order what
holds for the table, for a while, for the row, and for the column. The
panel needs the first three apart from the last, so the function is cut
where that order turns:

    Table::row_lock(row) -> Option<Lock>   // the table, a while, the row
    Table::lock(cell)    -> Option<Lock>   // row_lock, then the column's

- **A row lock** is every reason up to and including the row's key:
  `ReadOnly`, `NotATable`, `StructureLoading`, `NoKey`, `KeyType`,
  `Saving`, `Refreshing`, `NoSuchCell` for a row the page does not hold,
  `Gone`, `KeyIsNull`, `KeyInexact`. Under one, no field of the panel can
  be edited, and the footer says why, once.
- **A field's own lock** is what is left: `NoSuchCell` for a column the
  row does not hold, `UnknownColumn`, `Generated`, `KeyColumn`, `Binary`,
  `TooLarge`. The field is marked, and says why. (Today `lock` asks for
  that column before it asks whether the row is gone or its key readable.
  After the cut such a cell of such a row answers the row's reason. No
  view asks for a cell the page does not hold.)
- **Why an edit that was asked for was refused** is kept with the place it
  was asked from (`Edits::why_place`, beside `Edits::why`), so the reason
  shows where the user is looking: at the cell, or under the field.
- **What the panel knows of a pending cell** (`model::PendingField`) gains
  the cell's state in step 3, for what a field says of a cell to fix or
  failed.

## The panel (`src/ui/row_panel.rs`)

Everything below is for a table's row. Under a row lock the fields are
drawn as today, with no outline, pencil or checkbox.

### A field at rest

- As today. A value that can be edited shows the Components' text field
  border round it while the pointer is over it, and the field's focus ring
  (`focus::hint`, the field form) while its text has the keyboard. Omarchy
  draws the accent border of its text input.
- **The caption line** holds, at its right, Copy as today, then a pencil
  ("Edit <column>"), then from step 3 `NULL` for a nullable column. They
  show while the pointer is near the field or one of them has the
  keyboard, as Copy does today, and are always there for the Tab key and a
  screen reader. A document's caption line keeps "Collapse all" and its
  bordered Copy, and gains the pencil beside them. Omarchy draws the pencil
  as the same icon; in a document's caption it stands before the hints
  (`za fold · y copy`).
- **A field has the keyboard** when its value's text has it or its pencil
  does. A value with no text of its own (a NULL, the stand-in of an empty
  or a blank text, a JSON tree, an array list) takes no keyboard: its
  pencil is the field's stop.
- **A field with a lock of its own** has a small lock after its caption, in
  the dim colour, always drawn, and no pencil. The pointer over the lock
  shows the reason (`cell_editor::lock_text`), and a screen reader hears
  it with the caption.
- **A field whose cell the grid is editing** (the tab's editor is on this
  row, in the grid) shows a dashed box in the accent colour in its value's
  place, "Editing in the grid…", and neither pencil nor checkbox.

### Starting an edit

- A double-click on the value's place: its text, the NULL mark, the
  stand-in of an empty or a blank text. A JSON tree, an array list and an
  attachment card take no double-click: the pencil is their way in.
- The pencil, by click, Space or Enter.
- On macOS and Windows, Enter or F2 on a field that has the keyboard: on
  its pencil Enter is the button's press, and on its value's text the
  panel reads the two keys itself, before it adds the text. A caret in a
  value counts as a text field to `ui/keys.rs`, which leaves the grid's
  keys alone in such a frame.
- `Mod+I`, Omarchy's `e`, and the footer's Edit: `Action::EditRow`.
- Typing on a value does not start an edit: the panel's values take a
  caret to select and copy from, and `Mod+C` there must stay a copy.
- Each of these asks for `EditField`, which is `EditCell` with
  `EditStart::Value`, drawn in the panel.
  The editor starts from the pending value where the cell has one, and
  otherwise from the value's whole text as the database gave it
  (`edit::start_text`), never from the text the panel cut. The cursor is
  at its end. On a NULL it starts empty.
- The panel brings into view, once, the field whose editor opens and the
  field under which a reason is shown: `EditRow` can name a field far down
  a long row.
- An edit asked for on a field with a lock of its own opens nothing and
  says why: on macOS and Windows in a note under the field, until the
  selection moves; on Omarchy in the mode line, as for a cell.

### The one-line field

The Components' text field in the value's place, as wide as the field, its
text in the role the value is read in: the
border and halo of a focused field, red while the text fails its check,
with what it fails under it in the danger colour
(`cell_editor::problem_text`) and, for a column with a length, the counter
at its right (`27 / 200`). Omarchy draws its text input: the accent border,
the block cursor, the message below in red. The follow link gives its room
to the field while it is open.

The field's keys are the cell editor's, from the same code
(`cell_editor`'s key handling is shared, not copied):

- Enter commits. Where the text fails its check it stays open and red, as
  on a cell.
- Tab and Shift+Tab commit and, from step 3, edit the next or the previous
  editable field. Until then they commit as Enter does.
- Esc drops the edit. On Omarchy Esc leaves insert mode with the text kept
  and Ctrl+C drops it.
- Alt+Enter adds a line break and, from step 2, makes the field tall.
  Until then it does what it does for a cell: the edit moves to the grid's
  popover.
- The keyboard going elsewhere (a click, another field's pencil) keeps the
  text: pending, or a cell to fix.
- `Mod+S` saves and `Mod+Shift+D` reviews from inside the field, taking
  what is typed, as from a cell's.

After Enter or Esc the keyboard is on the field on macOS and Windows
(`focus_field`): on its value's text, or on its pencil where the value now
has none (a NULL, an empty text, a document). Enter edits it again and Tab
goes on from it. On Omarchy the keys are the grid's again, in normal mode,
on the cell that was edited.

### The tall field (step 2)

For a value `edit::opens_large` names. The value's place becomes a field
of several lines, as wide as the field, as tall as its text from three
lines to twelve, scrolling past that. Under it the band the popover has:
the character and line count, or what the text fails, and the keys
(`⌘↩ apply · esc cancel`; Omarchy `ctrl+enter apply · esc keep`). Enter
and Tab are the text's own; `Mod+Enter` applies. JSON is plain text in the
data face. The JSON tree, the array list and the attachment card of the
field are not drawn while it is edited.

### Walking (step 3)

Tab commits and opens the next field of the row that can be edited, in the
page's column order, which is the order macOS and Windows draw them
(Omarchy draws documents last, and the walk reaches each in its column's
turn). Shift+Tab goes the other way. A field that is locked is passed
over. A tall field is entered and ends the walk: Tab is its text's. Past
the last field, and before the first, the edit is committed and the
keyboard moves on as Tab moves it everywhere. A text that fails its check
keeps the field open, as Tab does on a cell.

### NULL and revert (step 3)

- **`NULL`** is a checkbox at the caption's right of a nullable field that
  can be edited (Omarchy: `[ ] null`). It shows with the pencil, and
  always while the value is NULL or pending. A NOT NULL column has none.
  Its four cases:
  - ticked on a value the cell loaded: the cell is pending, NULL;
  - ticked on a pending text of a cell that loaded NULL: the cell is as it
    loaded again, and leaves the set;
  - unticked on a pending NULL: the cell is as it loaded again;
  - unticked on a loaded NULL: the field opens empty. The empty text is
    not NULL until something is typed (`Editor::touched`), as in the grid.
- **`revert`** follows "was <loaded value>" under a pending field, as a
  link in the warning colour. It takes the cell out of the set.
- **Keys.** `Mod+Backspace` and `Mod+Z` act on the field that has the
  keyboard (its value's text or its pencil), as they act on the active
  cell with the keys on the grid. On Omarchy `x` and `u` stay the cursor's
  cell's.
- The checkbox and the link act on their own field's cell, whatever cell
  is selected, and select it: `SetNull` and `RevertCell` gain the cell
  they act on (none is the selection's, as their keys ask today).
- **With an editor open.** The pencil of the field being edited is not
  drawn; its checkbox and its `revert` are. Either drops what is typed
  (the editor closes as cancelled) and then acts. On another field they
  take the keyboard, so that field's editor closes with its text kept, and
  then act.

### What a field says of its cell (step 3)

- A cell to fix shows the kept text, the pending mark in the danger
  colour, and under the value what the column refuses.
- A cell whose row's statement failed shows the database's code and
  message under the value, in the danger colour: the panel has the room
  the grid gives a tooltip.
- While a save runs every field is under a row lock (`Saving`).

### The header

- macOS and Windows: while the row has pending cells the line under the
  title reads "2 unsaved changes" in the warning colour, in place of the
  table's name (step 3). Previous, Next and Close stay.
- Omarchy: its head has one line. On a row that can be edited its hint
  reads `[ ] prev/next · e edit`; `e edit` gives way first where the room
  runs out, then the words of `prev/next`, as today.

### The footer

- **Edit** is enabled on a row with no row lock. It asks for `EditRow`.
  Omarchy's `e edit` cell is its button, no longer dashed or faded.
- **Duplicate** and **Delete** stay disabled: "Duplicating and deleting
  rows arrive in a later version".
- **The note** under the buttons is the row lock's reason in the grid's
  own words, with the lock icon on macOS and Windows and in lower case on
  Omarchy ("this connection opens read-only", "views cannot be edited",
  "book_covers has no primary key or unique index, so a row can't be
  targeted safely", "a save is running"). Edit is then disabled with the
  same reason. With no row lock the note names the key on macOS and
  Windows ("⌘I edit") and is empty on Omarchy, whose cells name theirs.

## Keys

| | macOS, Windows | Omarchy |
|---|---|---|
| Edit the row in the panel | `Mod+I`, Edit | `e`, `Mod+I`, `e edit` |
| Edit a field | double-click, pencil, Enter or F2 on its value | double-click, pencil |
| Commit | Enter | Enter |
| Commit and edit the next, previous field | Tab, Shift+Tab | Tab, Shift+Tab |
| Leave | Esc drops | Esc keeps, Ctrl+C drops |
| Apply a tall value | `Mod+Enter` | Ctrl+Enter |
| Make a field tall | Alt+Enter | Alt+Enter |
| Set NULL | `Mod+Backspace` on a field, the checkbox | `x`, the checkbox |
| Revert | `Mod+Z` on a field, `revert` | `u`, `revert` |

- `Mod+I` is read where `Mod+S` is (`editing_keys`), in every look:
  wherever a table's Data view shows, whatever has the keyboard (the grid,
  the tree, a button, a caret in one of the panel's values, a filter's
  field). Unlike `Mod+S` it needs nothing pending, and it is not read in
  the Structure view or while an editor is open. It is matched by its
  key, so on Omarchy it reaches the form on a keyboard layout that cannot
  type `e`. A SQL editor keeps `Mod+I` for its completions: there is no
  table in front.
- A field's own keys (Enter and F2, and from step 3 `Mod+Backspace` and
  `Mod+Z`) are the panel's to read, while that field has the keyboard.
  Enter on a pencil is the button's press.
- `e` is read with the grid's other letters (`editing_letters`), under
  their rule: alone in its frame, with the keys on the grid.
- **A key acts only on the cell it was meant for,** as in the grid:
  `Mod+I` and `e` do nothing in a frame that brings a click.
- On Omarchy the mode line reads `-- INSERT --`, the column and its type
  while a field of the panel is edited, as for a cell. Its hint "tab next
  cell" is left out for an editor in the panel until step 3, and reads
  "tab next field" from then.
- The shortcuts screen (`keys::shortcuts(look)`) lists `Mod+I`, "Edit the
  row in the row panel", in every look, and `e` on Omarchy.

## Errors and edge cases

Each follows from a rule slice 1 has, and is tested here for the panel:

- **The selection moves** (Previous, Next, a click in the grid; Omarchy's
  letters are not read while an editor is open): the editor closes with
  its text kept, pending or to fix (`close_editor`, left), and the panel
  shows the new row.
- **The panel closes, or the view goes to Structure,** while a field is
  edited: the editor closes the same way. The first is new with this slice
  (see "The model"); the second is `SetView`'s, as for a cell's editor.
- **Another tab comes in front:** the editor waits in its tab with its
  text, and the tab wears the unsaved mark, as with a cell's.
- **The page would be replaced** (refresh, sort, filters, paging): the
  leaving guard asks, since the tab holds edits.
- **A prompt about pending changes is up** (Leave, the production
  confirmation, a conflict): the editing actions are dropped. Under any
  dialog an open field gives up the keyboard, and takes it back when the
  dialog goes (`Target::hold`).
- **The session is lost:** editing does not need one. The field stays, and
  Save says why it cannot run.
- **The row or the column is no longer on the page** when the editor
  closes: it closes and the set stays as it was.
- **An editor that was only opened** changes nothing, on a NULL too.
- **The value was shown cut** ("Show all"): the editor holds the whole
  text. A value over 256 KiB is locked (`TooLarge`).
- **A preview tab** is pinned by an edit started in the panel, as by one
  in the grid.
- **Review SQL is open:** its head says "Without the cell being edited"
  while the panel's field holds typed text.
- **A narrow panel:** the field is as wide as the value was; the caption
  gives way to its controls as it does to Copy today.

## Steps

Each step ends compiling, tested and shippable, and gets its own plan run.

1. **Edit a field in place.** `EditorPlace`, `EditCell`'s place and
   `EditRow`; `Table::row_lock`; the one-line field in the panel; the
   double-click, the pencil, Enter and F2, Edit, `Mod+I` and `e`; Enter,
   Esc, Ctrl+C and leaving; "Editing in the grid…"; the lock mark and the
   reason under a field; the footer's Edit and its note; Omarchy's head
   hint; the shortcuts screen. Tab commits as Enter does. A tall value's
   edit opens the grid's popover on its cell. A cell left to fix reads in
   the panel as any pending field until step 3: its cell in the grid and
   the pending bar say what it fails.
2. **The tall field** in the panel, and Alt+Enter making a field tall.
3. **Walking and the rest:** Tab and Shift+Tab; the `NULL` checkbox;
   `revert`; `Mod+Backspace` and `Mod+Z` on a field; the header's count;
   what a field says of a cell to fix or failed.

After step 1 every editable value has a way in from the panel, and nothing
on screen offers what is not built.

### What step 1 leaves for steps 2 and 3

- `Mod+I`, `e` and Edit go by the selected cell, not by the field a caret
  is in: with a caret in one value of the panel they can open another
  field. Enter and F2 are the keys of the value the caret is in.
- The panel's field is drawn in the look's own text field, on its fill.
  The red border of a text that fails its check is drawn at once; the
  focus ring and its halo show once a key was pressed, as everywhere.
- A locked value takes a double-click as an editable one does, and answers
  with its reason: the word under the pointer is selected as well, and the
  grid's selection moves to that cell.
- Edit pressed while the grid's popover is open on the selected cell's
  tall value closes the popover, its text kept: the click takes the
  keyboard from it, and until step 2 a tall value has no editor in the
  panel to open instead.
- On Omarchy the pencil of a document stands before the `za fold` hint and
  shows only while the pointer is near, so the caption's hints do not move.
- The header's Add row still says "Editing arrives in a later version",
  as it did while the grid alone was edited.

## Testing

- `src/edit.rs`: `row_lock` against `lock` for every reason, in each
  dialect it depends on (the SQLite key that was not read exactly).
- Reducer tests: an edit asked for in the panel opens there and one asked
  for in the grid opens there; a `LeaveEdit` for another cell or place
  closes nothing; `EditRow` takes the selected cell, the
  first editable one, or says why, and opens a closed panel; each ending
  makes of the set what it makes for a cell; a tall value's edit goes to
  the grid in step 1; closing the panel and switching to Structure keep
  the text; the reason of a refused edit is kept with its place;
  `EditRow` is dropped under a prompt.
- Headless UI tests (`src/testing.rs`), in every look that has it: each
  way in (the
  double-click, the pencil by click and by key, Enter and F2 on a value,
  Edit, `Mod+I`, `e`); typing and Enter making a pending field that the
  grid shows too; Esc dropping, Omarchy's Esc keeping and Ctrl+C dropping;
  a failing text keeping the field open with its message; "Editing in the
  grid…" while a cell of the row is edited; no pencil on a locked field,
  on a read-only connection, on a view or on a SQL result's row; the lock
  mark's reason and the footer's note; Edit enabled and disabled; the
  keyboard after Enter; `Mod+S` from the field; `Mod+I` leaving a SQL
  editor's completions alone; the Omarchy mode line; a pencil clicked
  while another field is edited, above it and below it, opening its own
  editor and keeping the other's text; Edit clicked while a field is
  edited, leaving an editor open on that cell with the text kept; the panel closed from another tab
  closing a waiting editor; the field brought into view.
- Steps 2 and 3 add theirs: the tall field's keys and band; the walk, its
  ends and a locked field passed over; the checkbox in its four cases, and
  the checkbox of a field above the one being edited;
  `revert`; the header's count; the messages under a field.
- `src/shots.rs` gains scenes to look at by hand, on its Bookshop data and
  in every look: `row-form-field` (a field being edited, its text refused,
  beside a pending one), `row-form-locked` (a locked field saying why; a
  row lock's note shows in the scenes of a read-only connection), and with
  the later steps `row-form-tall` and `row-form-pending`. No test compares
  a screen with the design.
- Nothing here changes what a save sends, so no test needs a PostgreSQL or
  a MySQL server: the suite that does is as it was.

## Documents this changes

Slice 1's spec (`2026-10-03-value-editing-core-design.md`): its "Out of
scope" ("Editing in the row panel"), the row panel's line under "Pending
changes" ("It stays read-only"), its keys table, and item 5 of "Editing as
a whole", which gives the "Editing a row" artboards to slice 5: the form
this slice takes is drawn on one of them. The main spec's
section 5.7 and its keyboard table. The README's line on the row panel.
The shortcuts table in `ui/keys.rs`.
