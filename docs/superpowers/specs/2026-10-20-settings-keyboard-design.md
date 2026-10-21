# Settings › Keyboard

This is the brief the work was given on 2026-10-20, as it was written,
followed by what it comes to against the tree: the decisions where the brief,
the boards and the keymap of `src/keymap.rs` do not say the same thing. The
plan is `docs/superpowers/plans/2026-10-20-settings-keyboard.md`.

---

# Prompt: build Settings › Keyboard

You are working on Tabletist, a native database client written in Rust (PostgreSQL, MySQL, SQLite), with a macOS look and an Omarchy look. Build **Settings › Keyboard**: one list of every command with its shortcut and scope, where any shortcut can be recorded again, unbound or reset, with conflicts caught before they are saved.

## Designs

Design canvas: **[Tabletist](https://claude.ai/artifact/4FtP3GnsXevWfpBsotjVb6)**, section **Settings · Keyboard**:

| Board | File | Shows |
|---|---|---|
| `macOS – Settings, Keyboard` | `MacSettingsKeyboard.dc.html` | Header with **Reset all…**, search field, **All / Changed · 1** segmented control, the list grouped by area (Command, Scope, Shortcut), a row being recorded ("Press keys…") with a conflict and **Swap**, a changed row with its dot, and the footer with **Export cheat sheet** |
| `Omarchy – Settings, Keyboard` | `OmarchySettingsKeyboard.dc.html` | The same as a pane: `/` filter, list by scope (scope, keys, action), a recording row with a conflict and `[s] swap`, the `super` note, the `settings.toml` pane, and the key line |

Also read:

- `macOS – Components` and `Omarchy – Components` (section **Components**): the shortcut recorder, key caps and conflict line.
- `hotkeys-fix-prompt.md`: the single keymap, its scopes and `reserved()`. This screen edits that keymap; it never keeps a second list.
- `settings-general-prompt.md`: how `settings.toml` is written, watched and reloaded, and how invalid lines are reported. Keyboard follows the same rules.
- `settings-editor-prompt.md`: Editor › Keybindings (vim mode), which this screen links to and does not repeat.
- The design system **[Tabletist](https://claude.ai/artifact/C44ZLtrGKqYek9WfvpSBoq)** (`project/README.md`, `tokens.json`). Use its tokens only.

If a board and this prompt disagree, this prompt wins. List each case in the PR.

## The keymap

- One keymap, defined in code as defaults (`hotkeys-fix-prompt.md`), with the user's changes from `settings.toml` laid over it. Menus, tooltips, quick open, the start screen and this screen all read the effective keymap, so a change shows everywhere at once.
- **Scopes** (macOS): Global, Connections, Table, Cell, Editor. **Scopes** (Omarchy): window, navigate, grid, edit, sql, plus the settings pane's own keys.
- A shortcut is unique **within a scope**. The same keys may do different things in different scopes (⌘N is New connection in Connections and Add row in Table). Global conflicts with every scope.
- Every command added by recent work is in the list with its default:
  - Find any object ⌘P / `ctrl+p` (quick open);
  - History ⌘Y / `:history`;
  - Reopen closed tabs ⇧⌘T / `u` (start screen);
  - Agents ⌥⌘A;
  - New SQL editor tab ⌘T / `ctrl+t`.
- Commands with no default shortcut are listed with an empty **Shortcut** cell ("Not set" in muted text) so they can be given one.

## The screen (macOS)

- **Header:** "Keyboard", "Click a shortcut to record a new one. The same keys can do different things in different scopes." and **Reset all…** on the right.
- **Search:** "Search commands or press a shortcut". Typing filters by command name and group. While the field has focus, pressing a chord that includes ⌘, ⌃ or ⌥ filters to the commands bound to it (in every scope) instead of typing it.
- **All / Changed · N:** Changed shows only rows that differ from the defaults; N is their count. With no changes, Changed reads "Changed" and shows "No shortcuts changed".
- **List:** groups as on the board: Window and connections, Navigation, Grid and editing, SQL editor, AI assistant. Columns Command, Scope (a small tag), Shortcut (key caps).
- **Changed rows** show the dot before the key caps. Hovering a changed row shows a reset button ("Reset to ⇧⌘F") after the caps.
- **Footer:** "Vim keys for the SQL editor are set in Editor · Keybindings." (the link opens Settings › Editor at Keybindings) and **Export cheat sheet**.

## Recording

- Click the key caps (or focus the row and press ↩) to record. The cell shows "Press keys…" in the recorder style; the rest of the list stays visible.
- The next chord is the new shortcut. On macOS it needs at least one of ⌘, ⌃ or ⌥, except F1–F19; a plain letter is refused with "Add ⌘, ⌃ or ⌥".
- **esc** cancels and keeps the old shortcut. ⌫ while recording unbinds the command ("Not set").
- **Reserved keys** can't be taken. Recording one shows "⌘Q is used by macOS" (or "by Tabletist" for `reserved()` keys) and stays in recording. Reserved on macOS: ⌘Q, ⌘W, ⌘H, ⌥⌘H, ⌘M, ⌘⇥, ⌘Space, ⌃⌘F, ⌘, and the text editing set ⌘C, ⌘V, ⌘X, ⌘Z, ⇧⌘Z, ⌘A.
- **Conflicts** in the same scope (or with Global): the row shows "⌥⌘E is already “Compare with previous run”." with **Swap** and "Cancel · esc".
  - **Swap** gives the new keys to this command and the old keys of this command to the other one. If this command had no keys, the other one is left "Not set" and says so in its row.
  - Cancel keeps both as they were.
- Saving writes `settings.toml` at once; there is no Apply button.

## Reset

- Per row: the hover reset button, or **Reset to default** in the row's context menu. (⌫ unbinds; it doesn't reset.)
- **Reset all…** asks: "Reset all keyboard shortcuts? Your N changes will be removed." with **Reset all** and **Cancel**. It removes every `[keys.*]` table from `settings.toml` and leaves the rest of the file untouched.

## Omarchy

- The pane as on the board. `/` filters by action name. To find an action by its keys, press `?` and then the keys. The board's placeholder says "or press a key to find it"; change it to "/ filter commands · ? find by key".
- **Keys in the pane:** `j/k` move, `h/l` change scope column focus, `enter` record, `dd` unbind, `R` reset the row, `ctrl+e` open `settings.toml` in `$EDITOR`, `esc` close. `ctrl+e` here is the settings pane's scope and does not clash with sql `ctrl+e` (explain analyze).
- **Recording** shows "press keys…" with the keys as typed. Sequences are allowed (`gd`, `dd`, `:w`). A sequence ends after 800 ms with no key, or on `enter`. `esc` cancels.
- **Conflicts** show "D is set default · [s] swap [esc] cancel". `s` swaps as on macOS.
- **Never bound:** any key with `super` ("super is never bound: hyprland owns it"), `:` and `esc`. Recording one shows the reason in the status line.
- Insert-mode keys follow vim and are not listed here.
- The TOML pane on the right shows the `[keys.*]` part of `settings.toml` as it is now, live, read-only.

## Storage

- In `settings.toml`, only changes from the defaults:

  ```toml
  [keys.grid]
  "D" = "delete_row"
  "ctrl+d" = "set_default"

  [keys.editor]
  "ctrl+r" = "run_all"

  [keys.unbind]
  grid = ["gd"]
  ```

- One table per scope: `[keys.global]`, `[keys.connections]`, `[keys.table]`, `[keys.cell]`, `[keys.editor]`, and on Omarchy `[keys.window]`, `[keys.navigate]`, `[keys.grid]`, `[keys.edit]`, `[keys.sql]`. Keys are the chord or sequence as a string; values are stable command ids (snake_case), never menu titles.
- One key notation for both looks: `cmd`, `ctrl`, `alt`, `shift` joined with `+`, then the key (`cmd+shift+f`, `alt+cmd+e`). The UI shows macOS symbols; the file never does.
- `[keys.unbind]` lists default keys that do nothing, per scope.
- A swap writes both lines, with a comment `# after swap · was set_default` on the moved one.
- Live reload and invalid lines as in Settings › General: an unknown command id, an unknown key name, a reserved key or a conflict in the file is ignored, the default stays, and the line is reported in Settings › General's file problems list with its line number. The app never rewrites a line it couldn't read.
- `:keys default` (Omarchy) and **Export cheat sheet** never write to `settings.toml`. `:keys default` writes the full effective list, commented out, into a new buffer the user can save.

## Export cheat sheet

- Default: a Markdown file chosen with a save panel ("Tabletist shortcuts.md"), grouped as on the screen, with the effective keys in macOS symbols, changed rows marked "(changed)", and the scope of each row.
- On Omarchy, `:keys cheatsheet` writes the same with Omarchy notation.

## Accessibility

- The list is a table; group names are row group headers. Each row is announced as "Format, Editor, Shift Command F, changed".
- The key caps are a button named "Shortcut for Format: Shift Command F. Press Return to record."
- While recording, announce "Recording. Press the new shortcut, Escape to cancel." The conflict line is an alert, and **Swap** and **Cancel** take focus in that order.
- Reset buttons are named "Reset Format to Shift Command F".
- All / Changed is a segmented control with the count in its name.
- Recording never traps focus: esc always leaves it.

## Tests (headless, no pixels)

Use only the Bookshop demo data in fixtures and tests. Pixel or design snapshots must never be part of any test run, and snapshot material isn't committed.

1. The screen lists every command in the keymap, including ⌘P, ⌘Y, ⇧⌘T, ⌥⌘A and ⌘T, with defaults from code.
2. Recording a free chord saves it, writes one line to `[keys.<scope>]`, and menus, tooltips and quick open show it at once.
3. The same keys in two different scopes are allowed (⌘N in Connections and Table); in the same scope, or against Global, they raise the conflict.
4. Swap moves the keys both ways and writes both lines; Cancel and esc change nothing.
5. Reserved keys (⌘Q, ⌘W, the text editing set, `reserved()`) are refused with the reason; on Omarchy any `super` key, `:` and `esc` are refused.
6. A plain letter on macOS is refused; F-keys are accepted.
7. ⌫ while recording unbinds; the default appears in `[keys.unbind]`; the command shows "Not set".
8. Per-row reset removes that row's lines; Reset all… removes every `[keys.*]` table and nothing else.
9. Search by name and search by pressing a chord both filter correctly; Changed shows only changed rows and its count matches.
10. Editing `settings.toml` by hand reloads live; an unknown id, unknown key, reserved key or conflicting line is ignored and reported with its line number, and the default stays.
11. Omarchy sequences: `gd` and `:w` record and run; a sequence ends after 800 ms or on enter.
12. `ctrl+e` opens the file in `$EDITOR` from the pane and still runs explain analyze in sql.
13. Export cheat sheet and `:keys default` never change `settings.toml`.

## Out of scope

- Vim and insert-mode keys (Editor › Keybindings).
- Importing keymaps from other apps.
- Keymap profiles or presets.
- Per-connection shortcuts.

## Open questions

Use the stated default, and note it in the PR.

1. **⌥⌘E:** the board shows it as Explain analyze clashing with "Compare with previous run", which has no default elsewhere. Default: Explain analyze keeps ⌥⌘E; Compare with previous run ships with no shortcut, and the board's conflict is an example.
2. **Next connection window ⌘2:** default ⌘` as on macOS, with ⌘1–⌘9 for windows by position. Confirm against the keymap.
3. **Find by key on Omarchy:** default `?` then keys, since `/` takes text.
4. **Export cheat sheet format:** default Markdown. The alternative is a printable PDF.
5. **Sequence timeout:** default 800 ms, the same as the grid's `gd` and `dd` today if they already have one; use that value if it differs.

## Delivery

- One PR. Every commit passes `cargo fmt --check`, `cargo clippy -- -D warnings` and the headless tests.
- No em dashes in UI copy.
- PR description:
  - a summary;
  - the `settings.toml` format for `[keys.*]` and the key notation;
  - the reserved keys list;
  - the open questions and the defaults chosen;
  - any place the build differs from the boards.
- Screenshots only if blurred. No unblurred screenshot goes to GitHub.

---

# What the brief comes to against the tree

The brief names `hotkeys-fix-prompt.md`, `settings-general-prompt.md` and
`settings-editor-prompt.md`. In this repository those are
`2026-10-08-keyboard-shortcuts-design.md`, `2026-10-03-settings-general-design.md`
(with `2026-10-17-settings-general-rows-design.md`) and
`2026-10-19-settings-editor-design.md`. The plan of run 2 of the keyboard
shortcuts (`2026-10-08-keyboard-shortcuts-2-settings-keyboard.md`) was never
run; this work takes its place, and keeps several of its answers.

## The scopes

The keymap holds a key in one of twelve scopes, not five: what has the
keyboard decides which keys are read, and `Scope::alongside` says which
scopes are read in the same moment (the global scope with every scope but a
prompt's, a cell's editor with the rows or the inspector it was opened from,
a result with its SQL editor). Conflicts are found by that rule, the one the
keymap's own test holds the defaults to.

Each scope has a name in each look, used both for the Scope column and for
the table of `settings.toml`. The brief's names are used where it gives one;
the scopes it does not name keep their own:

| Scope | macOS and Windows | Omarchy |
|---|---|---|
| Global | Global, `global` | global, `window` |
| Connections | Connections, `connections` | connections, `connections` |
| Sidebar | Sidebar, `sidebar` | navigate, `navigate` |
| Grid (a table's rows) | Table, `table` | grid, `grid` |
| Cell editor | Cell, `cell` | edit, `edit` |
| Inspector (row panel) | Inspector, `inspector` | inspector, `inspector` |
| SQL editor | Editor, `editor` | sql, `sql` |
| Results | Results, `results` | results, `results` |
| Agents tab | Agents, `agents` | agents, `agents` |
| Start screen | Start screen, `start` | start, `start` |
| History tab | History, `history` | history, `history` |

Every table name of either look is read in both (`[keys.grid]` on macOS is
the Table scope), and so are the names `Scope::id` gave before
(`cell-editor`, `sql-editor`). A table the app adds is written with the
look's name.

## What a row is

A row is a line of the keymap: a command, the first scope of its line, and
what must be on screen for it to hold. A command with keys in two places has
a row for each (`Edit cell` on the rows and in the inspector).

Not every row can be recorded:

- **A key keeps its kind.** The key handler reads chords command by command,
  typed letters through the letters' reader, and lines of the `:` prompt
  through the prompt. A row takes the kinds of keys its command has in the
  look (`Save changes` on Omarchy: a `:` line and a chord). A row with no key
  in the look takes the kinds the other look's keys have, where the handler
  reads those in every look: a chord. A macOS row whose command has only
  Omarchy letters or `:` lines would never be read, so it is not listed on
  macOS (the Agents tab's letters, the History tab's).
- **Fixed rows** are listed, dimmed, and cannot be recorded: the keys that
  say which way (`Move by row`, `Move in the sidebar`, …), a range of digits
  (`⌘1…9`, `alt+1..9`), a key that holds only while something else shows
  (the completion list, the large editor, the review), `Help` and `Leave the
  editor`, the Settings chord on macOS (the app menu's), a row whose default
  is one of the system's reserved keys (`Copy cell values` ⌘C, `Close tab`
  ⌘W), and a row that has only another command's keys (`Duplicate row` on
  Omarchy, which is `yy` then `p`). Their keys are what "used by Tabletist"
  means: `keymap::reserved` lists them.
- Prompts' and dialogs' keys (`Confirm production write`, the settings
  screen's own) are not listed: they are the prompt's own letters.

## What a recorded key does

- A key recorded on a row replaces the row's keys of its kind, and only
  those: `ctrl+w` recorded on Omarchy's `Save changes` leaves `:w` beside it.
- ⌫ (macOS, Windows) while recording, or `dd` on Omarchy's row, unbinds the
  row: every default key it had goes into `[keys.unbind]` and it shows "Not
  set".
- Swap gives this row the key and the other row this row's keys of that
  kind. The board's file (`"ctrl+d" = "set_default"`) cannot come from a
  swap in this model: `Set DEFAULT` on Omarchy is typed letters, so it gets
  `dd`. A line of another kind than its row takes is refused when read.

## Decisions

| # | The brief or a board | Built | Why |
|---|---|---|---|
| 1 | Open question 1: ⌥⌘E | Explain analyze ⌥⌘E; Compare with previous run no key | The brief's default. It changes decision 17 of the 2026-10-08 brief, where ⌥⌘E stayed Compare with previous run. Neither command is built |
| 2 | Open question 2: Next connection window ⌘2, ⌘` | Unchanged: Go to connection window ⌘1…9 (a fixed range), Next and Previous connection ⌃Tab and ⌃⇧Tab | The keymap, settled 2026-10-08 (decision 15) |
| 3 | Open question 3 | `?` then the keys | The brief's default |
| 4 | Open question 4 | Markdown | The brief's default |
| 5 | Open question 5 | 800 ms | The grid's `gd` and `dd` wait with no timeout today: a first key waits until the next key |
| 6 | Reopen closed tabs ⇧⌘T / `u` | In the keymap, not built: the keys are claimed, nothing answers | Nothing reopens a closed tab yet (the start screen opens recents by number) |
| 7 | Board rows `Next connection window`, `Reload schema`, `Focus inspector fields`, `Ask`, `Set NULL` and `Set DEFAULT` in Cell | The keymap's names and scopes: `Go to connection window`, `Reload`, `Open inspector / focus fields`, `Ask AI`, Set NULL and DEFAULT in Table | One keymap: the screen lists its names |
| 8 | Menus show a change at once | macOS menus carry no key equivalent but Settings' ⌘, (fixed); the `?` list, hints, tooltips, quick open and the start screen read the keymap | `src/macos.rs` |
| 9 | `:keys default` writes into "a new buffer" | `keys.default.toml` in the app's cache folder, written fresh each time and opened in `$EDITOR`, where it can be saved anywhere | The app has no buffers of text of its own but SQL editors, which hold SQL |
| 10 | Omarchy recording | Typed letters gather and end after 800 ms or on `enter`; a `:` line ends on `enter` only (a command's words take longer to type); a chord is taken at once; `enter` with nothing typed records `enter`; `backspace` takes back one character | A pause in the middle of `:keys default` is not the end of it |
| 11 | Omarchy `h/l` "change scope column focus" | Move to the previous or the next group | The pane has one column the cursor can be in |
| 12 | Omarchy board's file pane comment lines | Drawn as the pane's caption, not written into the file | The file holds only what the person changed; a comment the app wrote would stay after Reset all |
| 13 | `super` | The window never receives Super on Linux, where Hyprland takes it. Cmd or Super held while recording is refused with the note, and a `super+` line in the file is an unknown key | egui reports Super only as the Mac's Cmd |
| 14 | Omarchy refusals beyond the brief | Vim's insert keys (`ctrl+w`, `ctrl+h`, `ctrl+u`, `ctrl+r`) in the cell editor and the SQL editor; typed keys that start with a digit (a count) | "Insert-mode keys follow vim"; a count before `dd` deletes that many rows |
| 15 | Settings › General's "file problems list" | An ignored key line joins the file's ignored lines: red in the Omarchy file pane, and counted, with its line numbers, in the footer of the macOS and Windows sheet | The sheet has no list, only that line; it now names the lines |
| 16 | Windows | The sheet of macOS, Ctrl in the place of ⌘ (written `cmd` in the file, as the keymap does). Reserved: Alt+F4 and the text editing set (Ctrl+C, V, X, Z, Y, A) | The brief names two looks; Windows has the Mac's keys |
| 17 | macOS board footer | The pane's footer is the board's (the vim note and Export cheat sheet), with the line of ignored lines above it when there are some. The settings file's path and links stay on the other panes | The board |
| 18 | Changed rows | A row is changed when its keys differ from its defaults. The count is of rows | |
