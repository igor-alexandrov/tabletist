# Keyboard shortcuts: one keymap

This is the brief the work was given on 2026-10-08, as it was written. One
thing is changed: where its table had a dash for "no key", this copy says
`none`. What the brief means against the tree as it stands (which commands
are built, which keys go) is in the plan of run 1,
`docs/superpowers/plans/2026-10-08-keyboard-shortcuts-1-keymap.md`.

---

# Prompt: make Tabletist's keyboard shortcuts consistent

You are working on Tabletist, a native database client written in Rust (macOS and Omarchy, Linux and Windows later). The shortcuts used in the app have drifted from the designs, and the designs disagreed with each other. All of that is now settled. Your job is to make the app match the keymap below, from a single source of truth, and to add tests so it can't drift again.

Build what the current code supports today. For commands that don't exist yet (editing, inserting, transactions, AI), register them in the keymap as **reserved**: they get no handler, but the key is claimed, so nothing else can take it.

## 1. One keymap

- Put every binding in one module (for example `keymap.rs`) as data: `command id → { scope, macos, omarchy }`. Menus, the Keyboard settings page, the `?` help overlay, status-line hints and tooltips all read from it. Delete any shortcut strings hard-coded in UI code.
- **Scopes:** `global`, `connections`, `sidebar`, `grid`, `cell-editor`, `inspector`, `sql-editor`, `results`, `prompt` (modal y/N and choice prompts). A key may appear twice only in scopes that are never active at the same time.
- **Omarchy:**
  - bindings are given in vim mode (`normal`, `insert`, `visual`);
  - `super` is never bound, because Hyprland owns it;
  - in insert mode, standard vim keys win: `ctrl+w` deletes a word, `ctrl+h` is backspace, `ctrl+r` is redo in normal mode.
- **Labels:** macOS shows glyphs (⌘ ⇧ ⌥ ⌃ ↩ ⌫ ⇥). Omarchy shows lowercase `ctrl+x`, `:command`, or vim keys. A shortcut is always written the same way everywhere: `↩`, never "Enter"; `ctrl+enter`, never "C-Enter".

## 2. Decisions (these resolve the conflicts)

1. **Omarchy, `enter` in the grid opens the inspector. It no longer edits the cell.**
   - Use `i` (append), `cc` / `s` (replace) or typing in insert mode to edit a cell.
   - Inside the inspector, `enter` / `i` edit the focused field.
   - On a locked cell, `i` shows the reason in the status line.
2. **Paste over cells by default; pasting new rows is explicit.**
   - macOS: ⌘V pastes a TSV block over the selected cells. ⇧⌘V is **Paste as new rows**, which opens the preview dialog.
   - Omarchy:
     - `yy` / `p` / `P` yank a row and put a duplicate below / above (vim linewise);
     - `v` selects cells, then `y` copies and `p` pastes values over a visual selection;
     - `"+p` and `ctrl+shift+v` paste clipboard rows, which opens the preview.
   - Inside an open cell editor, paste is plain text on both platforms.
3. **macOS throwing away changes is ⌥⌘⌫ everywhere:** Discard all pending (grid) and Rollback (SQL editor). ⌘. stays Cancel running query only.
4. **A production write on Omarchy is confirmed by typing the database name** (`bookshop_production`), then `enter`. This covers grid writes (`:w`), SQL editor commit (`:commit`, `ctrl+s`) and anything else that writes to a production connection. `esc` goes back. The confirmation never shows a different database's name.
5. **Omarchy read-write toggle is `:rw` / `:ro`**, per tab, not `ctrl+w`.
6. **Omarchy SQL history is `:history`**, not `ctrl+r`, which stays vim redo.
7. **Discard all pending on Omarchy is `:e!`** everywhere. `:q!` only cancels a floating editor.
8. **Review pending SQL on Omarchy is `:diff`.** `:w` always writes; it never opens a preview.
9. **Format is ⇧⌘F (macOS) / `=` (Omarchy)**, for both SQL and JSON. No `gq`.
10. **Omarchy booleans change with `space` only.** No `t` / `f`.
11. **Omarchy dates and numbers:** `ctrl+a` / `ctrl+x` step the number, or the date or time part under the cursor. Typing `now` sets the current time. No `ctrl+h/j/k/l` and no `ctrl+t` inside editors: those belong to pane navigation and new tab.
12. **macOS ⌘⌫ sets NULL only when a cell is selected and not being edited.** Inside a text editor it keeps the system meaning (delete to the start of the line). There, Set NULL is a button only.
13. **macOS next / previous error is ⌥⌘↓ / ⌥⌘↑** (not ⌘', which is Set DEFAULT). Omarchy uses `]e` / `[e`.
14. **Picking from a list is `↑↓` + `↩` (macOS) and `ctrl+n/p` or `j/k` + `enter` (Omarchy)**, including the foreign key picker on a new row. `tab` never picks: in the grid it moves to the next column.
15. **Connection windows:** ⌘1…9 (macOS) / `ctrl+shift+1..9` (Omarchy) go to window N. The command is named **Go to connection window**, not "Next connection window".
16. **Duplicate is ⌘D / `yy p`** for rows and for connections alike (Omarchy connections currently use bare `yy`).
17. **Explain:** ⌘E / `ctrl+e` runs Explain in whatever mode was last picked in the Explain ▾ menu. The command is named **Explain**. Explain analyze is a separate command, unbound by default. On macOS, ⌥⌘E stays **Compare with previous run**.
18. **Reload:** ⌘R reloads the current table's rows and its schema. Name the command **Reload**, and use the same name on the sidebar refresh tooltip.

## 3. Canonical keymap

| Command | Scope | macOS | Omarchy |
|---|---|---|---|
| Connections | global | ⌘O | `ctrl+shift+c` |
| Go to connection window N | global | ⌘1…9 | `ctrl+shift+1..9` |
| Switch database | global | none | `ctrl+shift+d` |
| Settings | global | ⌘, | `ctrl+,` |
| Find any object | global | ⌘P | `/` in the sidebar |
| New SQL editor tab | global | ⌘T | `ctrl+t` |
| Toggle sidebar | global | none | `ctrl+b` |
| Move between panes | global | none | `ctrl+h/j/k/l` (normal mode) |
| Switch tab N | global | none | `alt+1..9` |
| New connection | connections | ⌘N | `n` |
| Edit connection | connections | ⌘E | `e` |
| Duplicate connection | connections | ⌘D | `yy p` |
| Delete connection | connections | ⌫ | `dd` |
| Connect / go to window | connections | ↩ | `enter` |
| Reload | grid | ⌘R | `R` |
| Open inspector / focus fields | grid | ⌘I | `enter`, `ctrl+l` |
| Back to grid from inspector | inspector | esc | `ctrl+h`, `esc` |
| Previous / next row in inspector | inspector | ↑↓ in header | `[` / `]` |
| Edit cell | grid | ↩, F2, double-click, or type | `i`, `cc`, `s` |
| Commit cell and move | cell-editor | ↩ down, ⇥ / ⇧⇥ across | `esc` keep, `tab` next cell |
| Cancel cell edit | cell-editor | esc | `ctrl+c` |
| Set NULL | grid | ⌘⌫ (selected cell only) | `x` |
| Set DEFAULT | grid | ⌘' | `D` |
| Undo / redo one cell | grid | ⌘Z / ⇧⌘Z | `u` / `ctrl+r` |
| Copy / paste cell values | grid | ⌘C / ⌘V | `v` … `y` / `p` |
| Paste as new rows | grid | ⇧⌘V | `"+p`, `ctrl+shift+v` |
| Add row | grid | ⌘N, or ↓ on the last row | `o` below / `O` above |
| Duplicate row | grid | ⌘D | `yy p` / `yy P` |
| Delete row (drops a new row) | grid | ⌫ | `dd` |
| Open referenced row | grid | none | `gd` |
| Show saved row in sorted position | grid | none | `gs` |
| Review pending SQL | grid | Review SQL button | `:diff` (`]c` next change) |
| Save changes | grid | ⌘S | `:w`, `ctrl+s` |
| Discard all pending | grid | ⌥⌘⌫ | `:e!` |
| Next / previous error | grid, sql-editor | ⌥⌘↓ / ⌥⌘↑ | `]e` / `[e` |
| Open value in $EDITOR | cell-editor | none | `ctrl+e` |
| Apply / cancel large editor | cell-editor | ⌘↩ / esc | `:wq` / `:q!` |
| Format SQL or JSON | sql-editor, cell-editor | ⇧⌘F | `=` |
| Boolean cycle | grid | space | `space` |
| Step number / date part | cell-editor | ↑↓ | `ctrl+a` / `ctrl+x` |
| Run statement | sql-editor | ⌘↩ | `ctrl+enter` |
| Run all | sql-editor | ⇧⌘↩ | `ctrl+shift+enter` |
| Explain | sql-editor | ⌘E | `ctrl+e` |
| Compare with previous run | sql-editor | ⌥⌘E | none |
| Cancel running query | sql-editor | ⌘. | `ctrl+c` |
| Focus results | sql-editor | none | `ctrl+j` |
| Completion | sql-editor | ⌃Space, ↩ insert, ⇥ next | `ctrl+space`, `tab`, `ctrl+n/p` |
| Go to line | sql-editor | ⌘L | `:N` |
| History | sql-editor | none | `:history` |
| Read-only / read-write tab | sql-editor | segmented switch | `:ro` / `:rw` |
| Commit | sql-editor | ⌘S | `:commit`, `ctrl+s` |
| Rollback | sql-editor | ⌥⌘⌫ | `:rollback` |
| Ask AI | sql-editor | ⌘K | `ctrl+k` |
| Accept / accept and run / discard AI | sql-editor | ⇥ / ⌘↩ / esc | `tab` / `ctrl+enter` / `esc` |
| Next plan tip | results | none | `]t` |
| Confirm production write | prompt | dialog button | type the database name, then `enter` |
| Help, all keys | global | none | `?` |

In Omarchy prompts, single-letter choices (`[c]` commit, `[r]` rollback, `[w]` write, `[d]` discard, `[esc]` stay) are scoped to that prompt and don't count as conflicts.

## 4. Settings → Keyboard

- Both platforms list every command in the table above, grouped as in the designs, with the command names exactly as written here.
- Rebinding checks the same scope and any scope active alongside it, and offers **Swap**.
- Omarchy writes overrides to `~/.config/tabletist/settings.toml` under `[keys.<scope>]`. `:keys default` dumps the full map.
- Fix the sample override in the docs: don't suggest binding `ctrl+r` (that's redo).

## 5. Tests (headless, no pixels)

Use only the Bookshop demo data in fixtures and tests. Pixel or design snapshots must never be part of any test run, and snapshot material isn't committed.

1. No two commands share a key, on either platform, in scopes that can be active together. The test is generated from the keymap data, not from a hand-written list.
2. Every command in the keymap has a macOS binding, an Omarchy binding, or an explicit `None`. Every command shown in a menu, hint or tooltip exists in the keymap.
3. No Omarchy binding uses `super`. No insert-mode binding shadows `ctrl+w`, `ctrl+h`, `ctrl+u` or `ctrl+r`.
4. Omarchy `enter` on a grid row opens the inspector; `i` enters insert mode on the cell.
5. macOS ⌘V over a selection pastes over cells; ⇧⌘V opens the new-rows preview. Omarchy `yy p` duplicates the row; `v`, `y`, `p` copy cell values.
6. ⌘⌫ with the cell editor open does not set NULL.
7. Rollback is ⌥⌘⌫ and Cancel query is ⌘.; neither triggers the other.
8. The Omarchy production confirmation accepts only the exact connection's database name.
9. Rebinding to a taken key reports the conflict and swaps on request.
10. Every label rendered for a command comes from the keymap.

## 6. Delivery

- One PR. Every commit passes `cargo fmt --check`, `cargo clippy -- -D warnings` and the headless tests.
- PR description: the decisions above, the test list, and a before/after table of changed bindings. Screenshots only if blurred: no unblurred screenshot goes to GitHub.
- Out of scope: new features. Reserve keys for commands that aren't built yet; don't implement them.
