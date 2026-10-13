# Foreign key label: let the user choose the column

This is the brief the work was given on 2026-10-12, as it was written. Two
things it names are called otherwise in the tree:

- `hotkeys-fix-prompt.md` is the brief kept as
  `docs/superpowers/specs/2026-10-08-keyboard-shortcuts-design.md`. The single
  keymap it describes is `src/keymap.rs` (pull request 109), and its collision
  test is `keymap::tests::no_two_commands_share_a_key_where_both_could_be_read`.
- The picker it extends is the one of
  `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`.

What the brief means against the tree as it stands, the runs it is built in,
and what was decided where the boards and the app differ, is in the plan of
run 1, `docs/superpowers/plans/2026-10-12-foreign-key-label-1-choice.md`.

---

# Prompt: let the user choose the foreign key label column

You are working on Tabletist, a native database client written in Rust (PostgreSQL, MySQL, SQLite). It has two looks: the macOS look (also used on Windows and Linux) and the Omarchy look (keyboard-only, terminal style). Build this feature for both.

## The feature in one paragraph

The foreign key picker already labels and searches the referenced table's rows by one column, which the app picks for the whole table. Today that pick is invisible and can't be changed. Add:

- a quiet indicator that shows which column is used and whether the app or the user picked it;
- a chooser to change it from the picker without losing the edit;
- a second way to change it from the referenced table's Structure view, which also works on read-only connections;
- a way back to Automatic;
- the loading, missing-column and key-only states.

## Designs

Canvas "Tabletist" (Design artifact `https://claude.ai/artifact/4FtP3GnsXevWfpBsotjVb6`), section **Editing · Values**, the six boards after "Omarchy – Editing values, editors by type":

| Board | What it shows |
|---|---|
| `macOS – Foreign key label, picker and chooser` | 1 label shown (Automatic) · 2 chooser open · 3 after your choice · 4 back to Automatic |
| `macOS – Foreign key label, Structure and states` | Structure door on a read-only connection, Use Automatic, and the states: loading, column gone, only a key, just changed |
| `Omarchy – Foreign key label, picker and chooser` | the same four steps in the Omarchy look |
| `Omarchy – Foreign key label, structure and states` | the structure pane with `gl` / `:label`, plus the same states |
| `Foreign key label, two columns (variant)` | two-column label. **Not in this PR** (see Out of scope) |
| `Foreign key label, notes` | every string, proposed keys, screen reader names, announcements, open questions |

Related boards to match:

- `macOS – Editing values, editors by type` (card 6);
- `macOS – Editing a row`;
- `macOS – Inserting a row`;
- `Omarchy – Editing values, editors by type` (card 6);
- `Omarchy – Components` (floats, cursor bar ▌, ✓ current, status line).

If the boards and this prompt disagree, this prompt wins for behavior and the boards win for layout and copy. Take every string from the **notes** board word for word.

Read `hotkeys-fix-prompt.md` first. Every key here goes into the single keymap described there.

## Rules

- Use only the Bookshop demo data in fixtures and tests. The boards use some placeholder publishers columns (`legal_name`, `email`, `city`, `founded_on`, `logo`) and invented authors rows. In tests, use the columns the Bookshop fixture actually has. If a test needs a table with only its key column, use the fixture's `isbns` table if it has that shape; otherwise add the smallest such table to the fixture and say so in the PR.
- Pixel or design snapshots must never be part of any test run, and snapshot material isn't committed.
- No unblurred screenshot goes to GitHub.
- One PR. Every commit passes `cargo fmt --check`, `cargo clippy -- -D warnings` and the headless tests.
- The UI says "Automatic". Never mention AI, models or confidence; no sparkle icons. Don't add settings for how the automatic pick works.
- Never use em dashes in UI copy.
- Use only existing type styles, colours and the existing focus ring. No custom focus ring.

## Behavior

### 1. The choice and where it lives

- One choice per **referenced table**, per **saved connection**: `(connection_id, schema, table) → LabelChoice`.
- `LabelChoice` is `Automatic`, `NoLabel` or `Column(name)`. The `[name, name]` form is reserved for the variant; don't build it.
- Store it in the app's own local state on this machine: macOS Application Support, `$XDG_STATE_HOME/tabletist/` on Linux. **Nothing is written to the database.** It works the same on read-only connections.
- Every foreign key that points at that table uses the choice: grid cell editors, inspector fields, new-row cells.
- **Allowed:** any column except the key column(s) and binary columns (`bytea`, `blob`, `binary`/`varbinary`). The chooser lists the excluded ones as disabled, with the reason "the key" or "binary".
- **Automatic** keeps today's pick unchanged. Expose that pick so the UI can name it ("uses name"). A table with no usable column resolves to keys only.

### 2. Picker: seeing the label

- **macOS:** the picker foot gets a button between "Esc cancels" and "Set NULL":
  - `Label {column} · Automatic ▾` or `Label {column} · Set by you ▾`;
  - `Label keys only` (disabled) when there's nothing to choose;
  - tooltip `Change label column ⌘L`.
- **Omarchy:** the float has no head or foot. The status line shows `label {column} · automatic` or `label {column} · set by you`, and `ctrl+l label` joins the key hints.
- **Labels:** options are labelled by the chosen column; numbers and dates are drawn as the grid draws them. The search runs on that column and on the key, as today.
- **No label:** options show the key alone.

### 3. Chooser

- **Opening:** ⌘L / `ctrl+l` from the picker, or a click on the foot button (macOS). It opens beside the list:
  - macOS: a menu;
  - Omarchy: a second float with an accent border. The picker float's border goes to the border colour.
- **Rows:**
  1. **Automatic · uses {column}**;
  2. **No label · keys only**;
  3. a "Columns" section with each column and its type, in table order;
  4. the excluded columns, disabled.

  The current choice has a check mark (✓).
- **Foot:** `For {table} in {connection}, on this computer. Nothing is written to the database.`
- **Keys:** the chooser has no text field.
  - macOS: ↑↓ move, a letter jumps to a column, ↩ chooses, esc goes back to the picker.
  - Omarchy: `j/k` or `ctrl+n/p` move, `enter` chooses, `esc` goes back.
- **Esc order:** esc closes the chooser first, then the picker on the next press. The typed text and caret position are kept exactly.
- **Choosing:** closes the chooser, saves the choice, re-runs the search for the text already typed, and returns focus to the field with the editor still open. The highlighted option stays on the same key if that key still matches. The cell's pending value doesn't change.
- **Picker keys while the chooser is closed:** every letter is search text, and a click on the list never takes focus from the field. Don't change this.

### 4. Second door: Structure of the referenced table

- **macOS:**
  - a "Label for foreign keys" line above Columns: `{column} · Automatic` or `· Set by you`, with **Change… ⌘L** (opens the same chooser) and, when set by you, **Use Automatic**;
  - a `label` tag on the column's name cell;
  - **Use as label ⌥⌘L** in each column row's menu (disabled for the key and binary columns).
- **Omarchy:**
  - the line `fk label  {column} · automatic` in the structure pane, and `◂ label` on the column;
  - `gl` on a column line uses it as the label;
  - `:label` opens the chooser, `:label auto` goes back to Automatic, `:label none` shows keys only, and `:label {column}` sets a column directly.
- Works on read-only connections and with no cell being edited.
- Optional, only if it's cheap: **Label {table} by…** in the header menu of a foreign key column in the grid, and `:label` on a foreign key cell. List it in the PR if you skip it.

### 5. States

- **Columns still loading** (cold schema cache): Automatic shows `· checking…`; Automatic and No label can be chosen at once; the columns area shows `Loading columns…` with the existing spinner.
- **Chosen column renamed or dropped:** detected when the schema loads or the picker opens. Clear the saved choice, fall back to Automatic, and say so **once**:
  - macOS: an amber line inside the picker, `{column} is gone, so labels use Automatic ({auto}).`;
  - Omarchy: the status line, `{column} is gone · labels use automatic ({auto})`.

  Store a "told" flag so it never repeats.
- **Only a key:** options are keys alone. The foot button is disabled with the tooltip `{table} has only its key column`. ⌘L / `ctrl+l` shows that message (tooltip on macOS, status line on Omarchy) instead of opening the chooser. The Structure line says the same.
- **Just changed:**
  - macOS: the foot button flashes once with the existing selection fill;
  - Omarchy: the status line shows `✓ label {column} · {n} matches` for 4 s, then the usual hints.

### 6. Accessibility

Use the names and announcements on the notes board:

- **Foot button:** button with a menu popup, named "Label column: name, Automatic".
- **Chooser:** a menu named "Label publishers by", with radio items. Disabled items say why.
- **Announcements:** made through the polite live region.
  - "Labelled by {column}. {n} matches."
  - "Labelled by {column}, Automatic. {n} matches."
  - "Keys only. {n} matches."
  - "{column} is gone. Labels use Automatic, {auto}." (once)

## Keys (proposals)

Add these to the keymap from `hotkeys-fix-prompt.md`. If the keymap's collision test fails on any of them, stop and report it; don't pick a different key on your own.

| Command | macOS | Omarchy | Scope |
|---|---|---|---|
| Change label column | ⌘L | `ctrl+l` | fk-picker (Omarchy: insert-mode float) |
| Move in chooser | ↑↓, letter jumps | `j/k`, `ctrl+n/p` | fk-chooser |
| Choose / back | ↩ / esc | `enter` / `esc` | fk-chooser |
| Open chooser from Structure | ⌘L | `:label` | structure |
| Use focused column as label | ⌥⌘L | `gl` | structure |
| Back to Automatic / keys only | Use Automatic button, or chooser | `:label auto` / `:label none` | structure |
| Add as part 2 (variant, reserve only) | ⇧↩ | `shift+enter` | fk-chooser |

Known overlaps that should pass because the scopes never overlap:

- ⌘L is Go to line in the SQL editor;
- `ctrl+l` is move to the right pane in normal mode;
- `ctrl+shift+enter` is Run all in the SQL editor.

## Implementation notes

- **Data:** add a `LabelChoice` to the column-metadata layer the picker already uses, with one resolver: `resolve(conn, table) -> ResolvedLabel { column: Option<Column>, source: Automatic | User | KeysOnly }`. The picker, inspector, new-row cell and Structure all read from it. No feature reads the store directly.
- **Search:** keep the existing search query and limit (first 50, at most 8 shown), parameterized by the resolved column. Quote identifiers with each driver's rules. Values are always bound. Non-text columns are matched on their text form, cast per driver.
- **Re-search:** changing the choice invalidates that table's picker results and re-runs the current query once. Don't re-query other tables.
- **Missing column:** detect it against the cached schema. Only call it gone when the schema is fresh, never while it's still loading.
- **UI:** the chooser is one component shared by the picker and Structure on each look. Keyboard routing stays in the single place described in the hotkeys prompt.

## Tests (headless, no pixels)

1. Automatic resolves to today's pick, and the picker reports `source = Automatic` with that column.
2. Choosing a column saves `(connection, table) → Column`, re-runs the search for the same typed text, keeps the editor open and the caret position unchanged, and doesn't change the cell's pending value.
3. The choice applies to every foreign key that points at the table, and only for that saved connection.
4. Nothing is sent to the database when choosing: the query log shows only the re-search `SELECT`.
5. Works on a read-only connection from Structure with no cell open.
6. The key and binary columns can't be chosen, through the UI or through `:label {column}`. Both report the reason.
7. Esc with the chooser open closes only the chooser. A second esc closes the picker.
8. With the picker open, letters, including `j` and `k` on Omarchy, go to the search text. In the chooser, `j/k` move.
9. A renamed or dropped column falls back to Automatic, clears the stored choice, and shows the notice exactly once.
10. A key-only table: ⌘L / `ctrl+l` shows the message and doesn't open a chooser. Options are keys.
11. While columns are loading, Automatic and No label are selectable, and choosing one works before the load finishes.
12. `gl`, `:label auto`, `:label none` and `:label {column}` give the same stored state as the macOS controls.
13. Announcements carry the exact strings, with the match count.
14. Search over a date or number label matches the text as the grid draws it, on PostgreSQL, MySQL and SQLite.
15. Every new key is registered in the keymap, and the collision test passes.

## Out of scope

- The two-column variant (reserve ⇧↩ / `shift+enter` only).
- Showing labels in grid cells or the row panel.
- Fuzzy matching.
- Composite foreign keys.
- Creating a referenced row from the picker.
- A global Settings entry.
- Settings for the automatic pick.

## Open questions

Proceed with the default for each; note any you changed in the PR.

1. dev and production of the same database keep separate choices. Default: separate, per saved connection.
2. Date and number labels are searched only as drawn. Default: yes.
3. No sample value per column in the chooser. Default: none.
4. The referencing table's Structure shows nothing new. Default: nothing new.
5. Omarchy storage: the app state file, not `settings.toml`. Default: state file.
6. Omarchy copy is lowercase ("automatic"), as on the boards. Default: lowercase.

## PR description

- A summary.
- The board list above.
- The keys added.
- The test list.
- Any skipped optional door.
- Placeholder columns you replaced.
- Screenshots only if blurred.
