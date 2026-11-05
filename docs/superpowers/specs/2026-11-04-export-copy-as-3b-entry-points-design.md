# Export and Copy as, run 3b: the table's other ways in

An addendum to `docs/superpowers/specs/2026-11-03-export-copy-as-3-table-design.md`.
Run 3a (PR #272, merged 2026-11-04) built the export, the sheet and the
float, every state, and the entry points that existed or were keys. Run
3b adds the rest of run 3's entry points and two ways the export tells
and is reached:

1. The sidebar's context menu on a table or view (new).
2. Quick open's export rows, and ⇧⌘E / `ctrl+e` on a picked table.
3. The grid's right-click menu: **Export selection…**.
4. Copy as's **Export…** when the selection is over the clipboard's limit
   (10,000 rows or 20 MB), and the same on the export's own clipboard
   failure (3a's B19).
5. A system notification when an export finishes while the window is not
   in front.
6. ⌘⇧' moves the keyboard to the export's toast.

Design canvas, section **Export and Copy as**: `macOS – Export, entry
points` (sidebar menu, quick open, File menu), `Omarchy – Export, entry
points` (sidebar foot, find object picker), `macOS – Export, progress and
states` and `Omarchy – Export, progress and states` (the notification),
`macOS – Copy as` (the grid menu, "Over 10,000 rows the copy stops and
offers Export instead") and `Omarchy – Copy as` (`E export the selection
to a file`).

## Where this build differs from the boards

Rows 1 to 3 were approved with run 3's spec (D14, the split into 3a, 3b
and run 4, run 2's D1) and are repeated here so the list is whole. Igor
approved rows 4 to 6 as written (2026-11-04).

| # | The board | This build | Why |
|---|---|---|---|
| E1 | Sidebar menu: Open ↩, Open in new tab, Structure, DDL, Copy name, Copy as SELECT, Export… ⇧⌘E, Import… later | Open ↩, Structure, DDL, Copy name, Export… ⇧⌘E, Import… (greyed, "later") | Run 3's D14, approved: Open in new tab and Copy as SELECT have no command to call |
| E2 | Quick open: `> Export result of Query 1…` | Not in 3b | A SQL result's export is run 4's |
| E3 | Grid menu: Copy, Copy as ›, Copy rows as TSV, Set to… ⇧⌘⌫, Export selection… ⇧⌘E | Copy, Copy as ›, Copy rows as TSV, Export selection… ⇧⌘E, in a table's grid. A SQL result's grid keeps run 2's menu | Set to has not landed on `main` (run 2's D1). A result's export is run 4's |
| E4 | macOS: a finished export in the background "posts a system notification" | macOS: the Dock icon bounces once (as when an agent connects). Linux: a desktop notification over D-Bus (what mako shows on Omarchy), as the board draws it. Windows: the taskbar button flashes | Notification Center needs `UNUserNotificationCenter`: new unsafe AppKit code, the person's permission asked on first use, and a bundled, signed app (a `cargo run` build has no bundle and would crash). The app's one notice today (an agent connected) is the Dock's bounce on macOS for the same reasons. A banner can be its own piece of work |
| E5 | The notification's words: `tabletist` over `exported 31,400 rows to reviews.csv` | The same, with the app's case: "Tabletist" over "Exported 31,400 rows to reviews.csv". Only a finished export to a file; a failure, a cancel and a copy post nothing | The board draws the done state only. A copy is over in a moment |
| E6 | ⌘⇧' moves focus to the toast, "as for other toasts" | ⌘⇧' (Windows Ctrl+Shift+') in the macOS look; the terminal look has no key: its status line has no buttons, and its keys (`:export cancel`, `go`, esc) are the toast's actions already | No other toast takes the keyboard; the one-line toast stays as it is |

## Decided while specifying

Not drawn or written anywhere. Igor approved every decision as written
(2026-11-04):

| # | Question | Decision | The other way |
|---|---|---|---|
| R1 | What Export selection… exports | The sheet opens on **Selected rows**. A block of cells (not whole rows) also opens **Columns: Choose** with the block's columns ticked, in the grid's order, and the others after them, left out. With no range, Selected rows is the cursor's row, as Copy as copies it. ⇧⌘E and `E` in the grid do the same with a selection (the Omarchy board: `E export the selection to a file`) | All columns whatever the selection, as 3a opens it |
| R2 | Quick open's rows | `> Export reviews…` with "current table" after it and ⇧⌘E (3a's "Export…", named for the table in front); `> Export a table…` with "pick one next". Both hide while an export runs (3a's "Cancel export" stands in its place) | None |
| R3 | Export a table… | Quick open stays open, empties its field, and lists this connection's tables and views only, under the placeholder "Pick a table to export"; ↩ opens the picked one and its sheet, as `E` in the tree does (3a's Q18). Esc closes it | A second panel |
| R4 | ⇧⌘E / `ctrl+e` in quick open | On a picked table or view (a result, a recent or one of the largest tables): it opens and its sheet opens over it. On a command row, nothing. The footer adds `⇧⌘E export` (`ctrl+e export the picked table`) while a table or view is picked | Always in the footer |
| R5 | The sidebar's menu | On a table or view, in both looks (the terminal look's is the picker, as the grid's menu is). Open opens it pinned (as a double click); Structure and DDL open it in that view; Copy name puts its name (no schema) on the clipboard and says "Copied reviews"; Export… opens it and its sheet, as `E` does. A right-click on a schema or folder opens no menu | Only Export… and Import… |
| R6 | Export… over Copy as's limit | The toast (Omarchy: the status line) says run 2's words and offers it: macOS **Export…** in the toast, which stays while the pointer is over it or it has the keyboard; Omarchy ` · E export`. It opens the sheet on the copy's format (TSV for ⌘C), Selected rows (R1), File. A SQL result's copy keeps run 2's words alone (run 4) | A dialog |
| R7 | The export's own clipboard failure (3a's B19) | The error toast's action is **Export…**: the sheet again with the same choices and File. Omarchy: `✗ that's more than 10,000 rows · :export to a file`, and `:export` opens the float on File | No action, as 3a |
| R8 | When the window is "not in front" | The window does not have the keyboard focus when the export ends (egui's `focused`), minimised or behind another window alike | Minimised only |
| R9 | ⌘⇧' | The keyboard goes to the toast's first button: Cancel (progress), the pill (hidden), Show in Finder (done), the failure's action or ×, Export… (the copy's offer). ⇥ and ⇧⇥ move among its buttons, ↩ and Space press, Esc gives the keyboard back to where it was. With no toast that has a button, nothing happens | Focus the toast's text |

## Keys

New in `crates/tabletist-core/src/keymap.rs`:

| Command | Scope | macOS | Omarchy |
|---|---|---|---|
| `QuickOpenExport` ("Export the picked table") | Prompt, `When::QuickOpen` | ⇧⌘E | `ctrl+e` |
| `FocusToast` ("Move to the toast") | Window | ⌘⇧' (Windows Ctrl+Shift+') | none |
| `LeaveToast` ("Give the keyboard back") | Prompt, `When::ToastFocused` | esc | none |

The sidebar's and the grid's menus name `Export`'s ⇧⌘E; they add no key.

## Tests (headless, no pixels; Bookshop data)

1. Sidebar: a right-click on `books` opens the menu with its six rows
   (Import… greyed); each row does what it says; a schema opens none.
2. Quick open: "Export books…" and "Export a table…" are listed; picking
   the first opens the sheet; the second lists tables and views only,
   and ↩ opens the picked one and its sheet; both hide while an export
   runs.
3. Quick open: ⇧⌘E and `ctrl+e` on a picked table open it and its sheet;
   on a command row nothing; the footer's hint shows only on a table.
4. Grid menu: Export selection… opens the sheet on Selected rows; a block
   opens Choose with its columns; a single cell exports its row; a SQL
   result's menu has no such row.
5. Copy as over the limit: the toast has Export…, which opens the sheet
   on the copy's format and File; Omarchy's line ends ` · E export`.
6. The export's clipboard failure: Export… (and Omarchy `:export`) opens
   the sheet on File.
7. Notification: a finished file export while the window is not focused
   sends one notice (and the attention request on macOS and Windows);
   none when focused, none for a failure, a cancel or the clipboard.
8. ⌘⇧': the keyboard goes to the toast's first button, ⇥ moves, Esc gives
   it back; nothing when no toast shows; the keys are free of clashes and
   listed in Settings › Keyboard.
