# Settings › Keyboard. Implementation Plan

**Goal:** A Keyboard pane in the Settings window of every look lists the
keymap's commands with their keys and scopes. Any row that can take a key
can be recorded again, unbound or reset; a key another row has is caught
before it is saved, and offered as a swap. The changes live in
`settings.toml` under `[keys.<scope>]` and `[keys.unbind]`, are read live,
and a line that cannot be applied is ignored and reported by its number.

**Spec:** `docs/superpowers/specs/2026-10-20-settings-keyboard-design.md`
(the brief as written, then the decisions against the tree).

**Architecture:** The keymap stays the one list. `keymap::Overrides` (the
person's lines, as data) is a field of `Settings`, read and written by the
settings store beside its other keys. `Keymap::with(layout, &overrides)`
lays them over `BINDINGS` and says which lines it refused and why; the
same function answers the recorder, so the file and the page cannot judge a
key differently. The App rebuilds its `Arc<Keymap>` whenever the settings'
lines change, which every reader of keys already takes from
(`ui::keys::publish`). The pages push actions; `App::apply` turns a
recorded key into the next `Overrides` with pure functions
(`record`, `unbind`, `reset`, `swap`) and saves through
`App::change_settings`.

**Tech stack:** Rust 2024, egui 0.36 (the fork) with AccessKit, `toml`'s
spanned reader the settings store uses. No new dependency.

## Files

| File | What changes |
|---|---|
| `src/keymap.rs` | Command ids (`Command::id`, `from_id`), scope names and tables per look (`Scope::name`, `Scope::table`, `Scope::from_table`), F1 to F19, a press spelled (`spelling`), how a key is spoken, the conflict rule moved out of the tests (`presses`, `clash`, `Entry`, `Keymap::entries`), rows (`Row`, `Keymap::rows`, `Fixed`), reserved keys (`reserved`), Explain analyze ⌥⌘E, Reopen closed tabs, `:keys cheatsheet`, the settings pane's new keys |
| `src/keymap/overrides.rs` (new) | `Overrides`, `Bind`, `Unbind`; `Keymap::with` and `Refused`/`Why`; `record`, `unbind`, `reset`, `swap`; the interning of spellings; what a refusal says |
| `src/settings.rs`, `src/settings/keys.rs` (new) | `Settings::keys`; reading `[keys.*]` (each line's number kept in `Loaded::key_lines`), writing them into the person's text (`Settings::write_keys_into`), taking them all out (`without_keys`) |
| `src/app.rs` | The keymap rebuilt from the settings; refused lines joined to the ignored ones; the Keyboard pane's actions; `:keys default`, `:keys cheatsheet`, the cheat sheet |
| `src/app/editing.rs` | The two `:keys` lines |
| `src/model.rs` | `SettingsPane::Keyboard`, `KeyboardPage`, `Recording`, the actions |
| `src/ui/settings/keyboard.rs` (new) | What both looks share: the rows shown (search, a key found, Changed), what a frame's keys come to while recording, the cheat sheet's text |
| `src/ui/settings/keyboard_sheet.rs` (new) | The macOS and Windows pane |
| `src/ui/settings/keyboard_terminal.rs` (new) | The Omarchy pane, its keys, and its part of the file pane |
| `src/ui/settings/{mod,sheet,terminal,file_pane}.rs` | The pane in the nav, its keys routed, the footer's line numbers |
| `src/testing.rs` | A clock the frames can be given |
| `tests/keys.rs` | `src/keymap/overrides.rs` may spell keys; the new test files are tests |
| `docs/_reference/keyboard-shortcuts.md`, `docs/_reference/settings-and-files.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md` | The pane, the file's tables, the changed keys |

## Tasks

Each task ends with the four checks of `AGENTS.md` passing and a commit.

### 1. The keymap knows its rows

- `commands!` takes each command's id: `DeleteRow = "delete_row"`. A test:
  every id is snake case, unique, and `from_id` gives the command back.
- `Scope::name(layout)`, `Scope::table(layout)` and `Scope::from_table`
  (every look's names, and the old ids). `Prompt` has none.
- `NAMED` gains `f1` to `f19`; `spelling(mods, key)` spells a press the way
  the file writes it (`cmd`, `ctrl`, `alt`, `shift`, then the key).
- `spoken(layout, keys)`: "Shift Command F" on macOS, the keys as written on
  Omarchy, "Ctrl+Shift+F" on Windows.
- The conflict rule leaves the tests: `presses`, `clash`, `Entry`,
  `Keymap::entries(layout)`. The keymap's own test reads them from there.
- `Row { command, scope, when }`, `Keymap::rows(layout)` in group order,
  `Keymap::fixed(layout, row) -> Option<Fixed>`, `Keymap::takes(layout, row)
  -> Kinds`, `Keymap::keys_of(layout, row)`, `Keymap::defaults_of`.
- `reserved(layout)`: the system's keys with their owner (macOS, Windows);
  the keys of fixed rows are Tabletist's.
- Keymap changes: Explain analyze ⌥⌘E, Compare with previous run none;
  `ReopenClosedTab` (⇧⌘T global, `u` on the start screen, not built);
  `KeysCheatsheet` (`:keys cheatsheet`) and `KeysDefault` built; the
  settings screen's `dd` (unbind), `/` (filter), `?` (find by key) and `s`
  (swap).

### 2. Overrides

- `Overrides { binds: Vec<Bind>, unbinds: Vec<Unbind> }` in the order read;
  `Bind { scope, table, keys, command, note }`, `Unbind { scope, table, keys }`.
  Spellings are interned (`&'static str`), and so are the chord lists the
  keymap is rebuilt with.
- `Keymap::with(layout, &overrides) -> (Keymap, Vec<Refused>)`: unbinds
  take a default key off the rows of their scope; binds replace a row's keys
  of their kind. A line is refused (`Why`) when its row does not exist or is
  fixed, its key is of a kind the row does not take, the look refuses the
  key (no modifier on macOS, reserved, never bound, vim's insert keys, a
  count), or the key is another row's where both are read (`Taken`, or
  `Tabletist` for a fixed row). Conflicts are found on the whole result and
  the later line gives way, again until none is left, so a swap's two lines
  stand together.
- `record(row, keys)`, `unbind(row)`, `reset(row)`, `swap(row, keys, other)`
  give the next `Overrides`, with nothing left that says what the defaults
  say.
- `Why::said(layout)` writes the refusal as the brief has it.

### 3. The file

- `Settings::keys`, read from `[keys.<table>]` (`"<keys>" = "<id>"`) and
  `[keys.unbind]` (`<table> = ["<keys>", …]`). A value of the wrong type, an
  unknown table, id or key name is an ignored line. `Loaded::key_lines` and
  `SettingsFile::key_lines` keep each line's number.
- `Settings::write_keys_into(text, layout)` changes the lines that changed
  and nothing else: a line in place keeps its comment, a new one goes at the
  end of its table (or of a new table, named for the look), an empty table
  goes, a line the reader ignored stays. `to_toml` writes the tables too.
- `without_keys(text)`: every `[keys.*]` table out, and nothing else.

### 4. The app's keymap

- `App::refresh_keymap` builds the keymap from the settings and the look,
  and adds the refused lines to the file's ignored ones. Called wherever the
  settings' text is taken in: the start, `change_settings`, a change from
  the disk, a connection renamed.
- The macOS and Windows footer names the ignored lines' numbers.

### 5. The pane's state and actions

- `SettingsPane::Keyboard`, listed in every look after Editor.
- `KeyboardPage { search, found, changed_only, cursor, filtering, finding,
  recording, resetting }` in `SettingsDialog`.
- Actions: start and stop recording, a press while recording (a chord, a
  typed character, backspace, a `:` line's end, unbind, cancel), Swap, reset
  a row, unbind a row, Reset all (asked, then answered), the search and its
  key, Changed, the cursor, export.

### 6. The macOS and Windows pane

Header and Reset all…, the search field (a chord with ⌘, ⌃ or ⌥ finds by
key), All / Changed · N, the table with its groups, a row's key caps (a
button), the changed dot, the hover reset, the context menu, the recorder
and its line, Swap and Cancel, the footer's link to Editor and Export cheat
sheet. AccessKit: a table, row groups, rows named as the brief says.

### 7. The Omarchy pane

The filter line (`/`), `?` then keys, the rows by group, the cursor, `enter`
records, `dd` unbinds, `R` resets, `h/l` groups, `ctrl+e` the file, `s`
swaps, `esc`; the recording line with the keys as typed and the 800 ms end;
the status line's refusals; the `super` note; the file pane with the
`[keys.*]` lines.

### 8. Cheat sheet and `:keys`

The Markdown of the rows as the pane groups them (macOS symbols on macOS,
Omarchy's notation there), saved through the save panel; `:keys cheatsheet`
the same; `:keys default` the full list, commented, opened in `$EDITOR`.
Neither touches `settings.toml`.

### 9. The website

The Keyboard pane, the `[keys.*]` tables and their notation, the changed
keys (Explain analyze, Reopen closed tabs, `:keys`).

## Tests

The brief's thirteen, each where it lives: the keymap's and the overrides'
own (1, 3, 5, 6 at the rules' level), the settings store's (8, 10 at the
file's level), the app's (2, 4, 7, 10, 12, 13), and the panes' headless
tests through `testing::Harness` (1, 2, 4, 5, 6, 7, 8, 9, 11).
