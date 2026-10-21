# Cmd+arrows go to the ends of a list

Issue: igor-alexandrov/tabletist#189. User feedback: "Cmd+arrows: jump to
the bottom of the list, or to the next page (left and right)."

## What happens now

In the grid the first and last row are Home and End (`MovePage`), and the
previous and next page are ⌥⌘← and ⌥⌘→. ⌘↑ and ⌘↓ do nothing in the grid,
the sidebar or the connection picker. Most Mac laptop keyboards have no
Home or End key (it is Fn+←/→), and ⌘↑/⌘↓ is the macOS way to the top or
bottom of a list. Nothing goes to the first or last column.

## Decision

- ⌘↑ and ⌘↓ (Ctrl+↑ and Ctrl+↓ on Windows): the first and last row in the
  grid (a table's and a SQL result's), the top and bottom of the sidebar's
  tree, and of the connection picker's list.
- ⌘← and ⌘→ (Ctrl+← and Ctrl+→): the first and last column in the grid, as
  in Numbers and Excel. Pages stay on ⌥⌘← and ⌥⌘→.

Rejected: ⌘←/⌘→ for the previous and next page, as the feedback had it.
⌘↑/⌘↓ then go to an end and ⌘←/⌘→ go elsewhere, and the grid keeps no key
for its first and last column, which a wide table needs more.

The Omarchy look keeps its keys: it is not part of this. In a text field
the chords keep their text meaning: the grid, the tree and the picker read
their keys only when no text field has the keyboard, as they do now.

## Keys (`src/keymap.rs`)

No new command: the chords join the commands that already move there, in
the macOS column (and so Windows'), not Omarchy's.

- `MovePage` (rows: Grid and Results): `cmd+up`, `cmd+down`, beside Home
  and End.
- `MoveColumn` (rows): `cmd+left`, `cmd+right`, beside the arrows.
- `MoveInTree` (Sidebar): `cmd+up`, `cmd+down`.
- `MoveInConnections` (Connections): `cmd+up`, `cmd+down`.

No scope that can be read with these has the chords today (⌥⌘↑/↓ is the
next and previous error), and the keymap's generated tests check it.

## Where they are read (`src/ui/keys.rs`)

Only the keymap spells a modifier (`tests/keys.rs`), so the handlers do
not match `"cmd+up"`: a helper, `to_end`, reads a chord with
`keymap::spell` and gives its arrow when Mod alone is held. Each handler
asks it after its own keys.

- The grid's moves: `cmd+up` and `cmd+down` are Home's and End's rows
  (`isize::MIN`, `isize::MAX`), `cmd+left` and `cmd+right` the same as
  columns. `step` in `App::apply(MoveSelection)` already clamps.
- The tree: `cmd+up` is `TreeKey::Home`, `cmd+down` `TreeKey::End`.
- The picker: `cmd+up` and `cmd+down` are steps of `isize::MIN` and
  `isize::MAX`. `App::apply(MovePickerSelection)` adds with saturation
  rather than overflowing, and with nothing selected a step to an end goes
  to that end (an Up still starts at the bottom, a Down at the top).

## Docs

- `docs/_reference/keyboard-shortcuts.md`: the picker's, the sidebar's and
  the grid's rows name Mod+Up and Mod+Down, and the grid's Mod+Left and
  Mod+Right.
- `docs/_guide/macos.md`: the same with Cmd.
- `docs/_guide/browsing-data.md`: a line on moving in the grid by keys.

## Tests

Headless, through `src/testing.rs`:

1. In a table's grid, ⌘↓ goes to the last row and ⌘↑ to the first, ⌘→ to
   the last column and ⌘← to the first.
2. The same in a SQL result.
3. In the sidebar, ⌘↓ goes to the last row of the tree and ⌘↑ to the first.
4. In the picker, ⌘↓ picks the last connection and ⌘↑ the first.
5. With a cell's editor open, ⌘↓ moves in its text and not the grid.
6. The Omarchy look's grid does not read ⌘↓.
