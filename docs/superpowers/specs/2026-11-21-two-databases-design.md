# Two databases of one connection, in tabs of their own (#366)

From user feedback: "If I want to open two databases from the same
connection at the same time, how can I do that?" Today the connection bar's
database menu switches the one tab to another database
(`Action::SwitchDatabase`). A second tab of an open connection already
exists (the picker's right-click "Connect again", or Shift+Enter), but it
opens on the saved database, and a relaunch puts every tab back on the saved
database with one set of kept tables per connection.

## What the canvas has

- The chips already tell two tabs of one connection apart: the database is
  the chip's second line on macOS ("Bookshop · dev" over
  `bookshop_development`) and follows the name in Omarchy
  ("Bookshop bookshop_development"). Nothing changes in the chips.
- The canvas draws the database chip and the "Switch database" command
  (⇧⌘D, Omarchy `ctrl+shift+d`, start screen `d`), not the menu's pop-up.
- The picker: an open connection's row reads "open" and its button "Show
  window" (macOS); Omarchy's footer reads "enter connect / go to window".

## On the canvas

Drawn after approval, on the Connections page under "Multiple open":
"macOS – Two databases of one connection" (`MacTwoDatabases`) and
"Omarchy – Two databases of one connection" (`OmarchyTwoDatabases`). They
show the database menu with its hint (macOS, Windows/Linux, Omarchy), the
bar after the new window opened, and the picker's "Open another" and
footer. They add to the canvas what it did not draw; the only change to a
drawn board is the picker footer's new hint. Keep them in step with the
code.

| What the boards show | Was on the canvas before |
|---|---|
| The lit row's muted hint: "New window ⌘↩" (macOS look), "New window Ctrl+Enter" (Windows/Linux), "ctrl+enter new window" (Omarchy) | The menu's pop-up was not drawn |
| Picker footer adds "⇧↩ open another" / "shift+enter another" | "↩ connect ⌘E edit ⌘D duplicate ⌫ delete" / "enter connect / go to window …" |
| Right-click "Open another" (code: "Connect again") | Not drawn |

## Decisions

1. **Gesture in the database menu: modifier + hint.** Click (or Enter)
   switches this tab, as now. ⌘-click / ⌘↩ on macOS, Ctrl-click /
   Ctrl+Enter on Windows and Linux, opens the database in a new tab. The lit
   row shows the hint; no new rows or buttons. *Why: one step, no new
   control the canvas lacks, and it can still be found.*
2. **Picker: footer hint only.** Rename the right-click "Connect again" to
   "Open another" and add "⇧↩ open another" (Omarchy "shift+enter
   another") to the footer. The row's layout does not change. *Why: the
   gesture exists; #367 is reworking the picker's grouping, so no row
   changes here.*
3. **Relaunch: each tab as it was.** Each tab reopens on its own database
   with its own tables, kept per tab, not per connection. session.json gets
   a new shape that still reads the old one. *Why: fixes two tabs of one
   connection sharing one kept set of tables.*
4. **New tab: next to it, in front.** The new chip goes right after the tab
   it came from, and it shows at once, with that database's tree and
   nothing open. ⌘1..9 shift for the tabs after it. *Why: it was asked for
   to be used now, and next to its sibling.*
5. **Omarchy key: ctrl+enter** (and ctrl-click), the hint reading
   "ctrl+enter new window". *Why: the same as Linux and Windows, close to
   Omarchy's ctrl chords.*
6. **A keymap command, rebindable:** "Open database in a new window", ⌘↩ on
   macOS and Ctrl+Enter elsewhere, listed in Settings › Keyboard; the hint
   is written from the keymap. The click modifier stays fixed (⌘ / Ctrl).
   *Why: as "Open the connection again" (⇧↩) is.*
7. **The word is "window", not "tab"** (asked when the boards were drawn):
   the hint reads "New window ⌘↩" / "ctrl+enter new window", and the
   command "Open database in a new window". *Why: the canvas calls a
   connection's tab a window ("Show window", "go to window",
   "ctrl+shift+1..9 window") and a table a tab; "new tab" would read as a
   table tab.*
8. **The menu's key is the open menu's own, like Copy as's letters**
   (asked while drafting run 1; this replaces decision 6's "listed in
   Settings › Keyboard" and "rebindable"): a keymap line in the prompt
   scope that holds while the database menu is open (`When::DatabaseMenu`).
   It is fixed, Settings › Keyboard does not list it, and the hint is
   written from the keymap. *Why: keys of menus and dialogs are in the
   prompt scope, which Settings does not list and an override cannot
   rebind; a global ⌘↩ would clash with the SQL editor's Run.*
9. **A menu opened by its key gives the keyboard to its chip** (found while
   drafting run 1): opened by Switch database's key or quick open, the
   keyboard was nowhere and the arrows never reached the rows, so the new
   key could not be pressed on one. Now the arrows go down the rows as
   they do after Enter on the chip, and the keyboard goes back where it
   was (a SQL editor being typed in) when the menu closes. A click leaves
   the keyboard where it was. *Why: without it, ctrl+enter does nothing
   from the keyboard in the Omarchy look.*

## Behaviour (so far)

- The new tab is the same saved connection with its own backend session,
  on the chosen database. It connects with the secrets the source tab
  holds (`Workspace::secrets`), so nothing is asked twice.
- The source tab is untouched: its tables, editors, pending edits and
  session stay. No pending-edits prompt (that is for switching).
- The current database's row takes the gesture too: a second tab on the
  same database, as "Open another" gives.
- The start screen's database button opens the same menu, so the gesture
  works there; quick open's "Switch database" opens the chip's menu, where
  it works too.
- While the database menu is open, its key is read before every other
  chord: a SQL editor in front does not run on ⌘↩.
- At quit, of two windows of one connection on its own database, the first
  in the bar is the one whose tables are kept (until run 2 keeps each).
- Agents (MCP) are per connection and unchanged; "show agents" goes to the
  first tab of the connection, as now.
- The hint is the menu row's `hint` (muted, at the right; in the terminal
  look the key leads, in the accent), on the lit row only; the room is kept
  on every row, so the menu's width does not move. The key is read before
  the rows are drawn, so the row under the keyboard does not take it as an
  Enter that switches; a pointer click reads `modifiers.command`.

## Relaunch

- session.json keeps `open` as it is (a connection id or none per tab) and
  gains `windows`, the same length: per tab its database and its kept
  tables (`Kept`). An older version (Settings › Updates' Go back) reads
  `open` and ignores `windows`, so going back still restores the tabs.
- A file without `windows` reads as today. An older version writes back
  the `windows` it does not know (`Session::rest`) with its own `open`, so
  `windows` is read only where each entry's connection is `open`'s at the
  same place; otherwise it is dropped and the file reads as today.
- `connections` (per connection) stays: it is what the connection's own
  database had when its last tab on it closed, for opening it later from
  the picker.
- A relaunch to update (`relaunch.window_of`) is per tab already and is
  unchanged.

## Runs (a PR each)

1. **Database menu: open in a new window.** The keymap command, the hint in
   both looks, the modifier click, the new window next to its sibling and in
   front with the source's secrets, the start screen's menu, the guide and
   the keyboard reference. Headless tests: ⌘-click and ⌘↩ open a second
   tab, both connected, each on its own database, the first tab's tables
   untouched; the chip order; no second password prompt; a rebinding
   rewrites the hint; the Omarchy hint. Until run 2, a relaunch puts the
   new window on the saved database, as a switched tab is today.
2. **Relaunch each tab as it was.** session.json `windows`, read and
   written; tests for the round trip, an old file, and two tabs of one
   connection with different tables.
3. **Picker: "Open another".** The rename and the footer hint in both
   looks. Small, and touching the picker, so it lands after #367's picker
   work to avoid conflicts.
