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

## Departures from the canvas

| What the canvas has | What will be built | Why |
|---|---|---|
| The database menu is not drawn | The lit row (pointer or keyboard) shows a muted hint at its right: "new tab ⌘↩" (macOS look), "new tab Ctrl+Enter" (Windows/Linux), "ctrl+enter new tab" (Omarchy) | Shows the new gesture where it is used |
| Picker footer: "↩ connect ⌘E edit ⌘D duplicate ⌫ delete" / Omarchy "enter connect / go to window …" | Adds "⇧↩ open another" / "shift+enter another" | Shows the gesture that exists already |
| — (code: right-click "Connect again") | Renamed "Open another" | Says what it does to an open connection |

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
   "ctrl+enter new tab". *Why: the same as Linux and Windows, close to
   Omarchy's ctrl chords.*
6. **A keymap command, rebindable:** "Open database in a new tab", ⌘↩ on
   macOS and Ctrl+Enter elsewhere, listed in Settings › Keyboard; the hint
   is written from the keymap. The click modifier stays fixed (⌘ / Ctrl).
   *Why: as "Open the connection again" (⇧↩) is.*

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
- Agents (MCP) are per connection and unchanged; "show agents" goes to the
  first tab of the connection, as now.
- The hint is the menu row's `detail` (muted, at the right), on the lit row
  only; the menu widens to fit it. The keyboard's Enter on a row with the
  command modifier held takes the new command; a pointer click reads
  `modifiers.command`.

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

1. **Database menu: open in a new tab.** The keymap command, the hint in
   both looks, the modifier click, the new tab next to its sibling and in
   front with the source's secrets, the start screen's menu, the guide and
   the keyboard reference. Headless tests: ⌘-click and ⌘↩ open a second
   tab, both connected, each on its own database, the first tab's tables
   untouched; the chip order; no second password prompt; a rebinding
   rewrites the hint; the Omarchy hint. Until run 2, a relaunch puts the
   new tab on the saved database, as a switched tab is today.
2. **Relaunch each tab as it was.** session.json `windows`, read and
   written; tests for the round trip, an old file, and two tabs of one
   connection with different tables.
3. **Picker: "Open another".** The rename and the footer hint in both
   looks. Small, and touching the picker, so it lands after #367's picker
   work to avoid conflicts.
