# Edit values in place in the row inspector

Date: 2026-10-06. Status: step 1 is built, see
`docs/superpowers/plans/2026-10-06-row-inspector-inline-edit.md`, whose
"What was decided" says what of the text below waits for the grid to have
it (editors by type, the undo stack, rows). Step 2, a tall value edited in
the panel, is not built.

This replaces `2026-10-06-row-form-design.md` (pull request #89), which kept
the row panel's fields as text to read and added a pencil, a double-click
and an Edit button. Run, that did not edit as the design does. The text
below is the user's spec as given, but for its dashes, which the house
style does not use. What of it the app can do today, and in
which step, is in the plan's "What was decided".

The designs are in the design canvas Artifact and are not copied into the
repository.

## Goal

Remove the inspector's **Edit** button and its edit mode. Every field in the row inspector is editable in place, the same way grid cells are, and both share one set of pending changes.

Design references (canvas "Tabletist"):
- `macOS – Table view, editable` (Main): inspector with hover state on `kind`, locked `id`, Duplicate / Delete only.
- `macOS – Editing a row` (MainEdit): inspector while values are pending.
- `macOS – Editing values, flow` and `…, editors by type`: cell states, editors, keys, save, conflict.
- `Omarchy – Table view, full-width / half-width` and `Omarchy – Editing values`: same behavior with vim keys.
- `Components` sheets: field, hover, focus, pending, saved and failed states.

## Behavior

### 1. No edit mode
- Delete the Edit button, its command, and any `inspector_edit_mode` (or similar) state.
- Row actions are **Duplicate** (⌘D / `yy p`) and **Delete** (⌫ / `dd`).
- Hint line under the actions: `Click a value or press ⌘I to edit · ⌘D · ⌫`.

### 2. Fields
Each column renders as a field: label row (`name · type` plus flags), then the value.

| State | macOS | Omarchy |
|---|---|---|
| Idle | plain value | plain value |
| Hover | `#F7F6F3` fill, 1px `#DCDAD4` inset border, radius 6, pencil icon at right, I-beam cursor | n/a |
| Focused (keyboard) | inset 2px accent ring (`#2C55C9`) | `sel` background + 2px accent left bar |
| Editing | the type's editor, same as the grid (see 4) | insert mode, block cursor |
| Pending | `#FBF0D9` fill, 2px `#C98A12` left bar, label shows `was <old> · revert` | yellow value, `~` in gutter |
| Failed | `#FBE9E7` fill, 1px `#C2261F` border, message under the field | red, message under the field |
| Locked | value in muted text, lock icon after the label; activating it shows the reason | muted; `enter` shows the reason in the status line |

Locked = primary key with identity always, generated columns, columns without UPDATE privilege, any field on a read-only connection.

On a read-only connection: no hover, no editors, one line at the top of the inspector: `Read-only connection`. Duplicate and Delete are disabled with the same tooltip.

### 3. Starting and ending an edit
- Click on the value, or ↩ / F2 on a focused field starts editing (macOS). `i`/`enter` insert, `cc`/`s` replace (Omarchy).
- Typing a printable character on a focused field starts editing with that character (macOS only).
- ↩ commits and moves focus to the next field; ⇥ / ⇧⇥ commit and move; Esc cancels this field's edit (Omarchy: `esc` keeps, `ctrl+c` drops).
- Commit means "add to pending", never "write to the database".
- Focus leaving the field (click elsewhere) commits if the value parses; otherwise the field stays in the editor with its error.

### 4. Editors
Reuse the grid's editor for each type; do not build inspector-only editors. The inspector gives them room:
- text: single-line field; multi-line or > 200 chars uses a multi-line editor inline in the panel (not a popover);
- json/jsonb: inline editor with line numbers, live parse, Format (⇧⌘F), cannot apply while invalid;
- arrays: chip editor; enum / CHECK IN: pick list with tag colors; boolean: segmented true / false / (NULL if nullable);
- foreign key: search the referenced table by first text column and id; keep "Open publisher →";
- dates and timestamptz: typed first, calendar helper, user's zone, sent with offset;
- bytea: Replace from file… / Save… / Set NULL; never typed.
- NULL ⌘⌫ (`x`), DEFAULT ⌘' (`D`) and empty string are separate explicit actions; hide Set NULL on NOT NULL columns.

### 5. One pending model
- Grid and inspector read and write the **same** pending-change store, keyed by (table, primary key, column).
- An edit in either place marks the cell in the grid and the field in the inspector at once.
- ⌘Z / ⇧⌘Z (`u` / `ctrl+r`) undo and redo per cell, from either place.
- The pending bar, Review SQL, Save (⌘S / `:w`), Discard all and the production confirm stay exactly as designed; the inspector does not get its own Save.
- Switching rows (↑↓ in the inspector header, `[`/`]`) keeps pending changes and shows them when you come back.
- Conflict detection is unchanged: `WHERE pk = … AND <changed column> = <loaded value>`; 0 rows is the conflict dialog.

### 6. Keyboard
- ⌘I (macOS) / `ctrl+l` (Omarchy): move focus to the first editable field of the selected row, opening the inspector if hidden. Rename the command to **Focus inspector fields** in Keyboard settings (done in the designs).
- Inside the inspector: ↑↓ (`j`/`k`) move between fields, Esc (`ctrl+h`) returns focus to the grid cell of the same column.

### 7. Accessibility
- Each field is a focusable control with label `"<column>, <type>, <value>"`; pending adds `", changed from <old>"`; locked adds `", read-only: <reason>"`.
- Focus ring rules from the Components sheet (inset ring inside the panel).

## Out of scope
- Inserting rows from the inspector (Add row stays in the toolbar).
- Editing several rows at once.
- Any change to the save, review or conflict flows.

## Implementation notes
- Move editor widgets to a shared module used by grid cells and inspector fields; inspector only supplies a wider rect and a different focus style.
- Remove dead code paths for the old edit mode and their settings/strings.
- Keep keyboard routing in one place so grid and inspector cannot both handle the same key.

## Tests (headless, no pixels)
Use only the Bookshop demo data in fixtures. Do not add pixel or design snapshot tests, and do not commit snapshot material.

1. Editing `kind` in the inspector marks the same cell pending in the grid, and vice versa.
2. Undo from the grid reverts an edit made in the inspector.
3. ↩ commits and focuses the next editable field; locked fields are skipped.
4. Esc cancels only the current field; other pending fields stay.
5. Invalid int8 input keeps the editor open with an error and blocks Save.
6. Read-only connection: no field enters edit state; Duplicate/Delete disabled.
7. ⌘I / `ctrl+l` opens the inspector if hidden and focuses the first editable field.
8. Switching rows and back preserves pending inspector edits.
9. NULL vs empty string: clearing a text field produces `''`; ⌘⌫ produces NULL; Set NULL is unavailable on NOT NULL columns.
10. Generated SQL for an inspector edit equals the SQL for the same edit made in the grid.

## Delivery
- One PR. Each commit passes `cargo fmt --check`, `cargo clippy -- -D warnings` and the headless tests.
- PR description: summary, the test list above, screenshots only if blurred (no unblurred screenshots go to GitHub).
