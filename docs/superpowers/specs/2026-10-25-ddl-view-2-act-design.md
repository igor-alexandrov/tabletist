# The DDL view, run 2: act on it

Run 1 (`docs/superpowers/specs/2026-10-24-ddl-view-design.md`, pull
request #222) reads a table's, a view's or a materialized view's
definition and shows it as a third view beside Rows and Structure. This
run is the second of the three that spec lays out: Copy, Open in SQL
editor, the jump links and the Omarchy sections outline, and every key.
Run 3 is find in the text and the macOS View menu.

Design canvas: **Tabletist**, section **Table view · Structure**, boards
`macOS – DDL view`, `macOS – DDL, by driver and state` and
`Omarchy – DDL view`.

## Decisions taken with the user (2026-10-24)

| Question | Decision |
| --- | --- |
| Omarchy `s` is Replace cell on the rows (vim's substitute; `cc` does the same) | `s` is Structure from every view; Replace cell keeps `cc` only |
| ⌃⌘1/2/3 folds to Ctrl+1/2/3 on Windows, which goes to a connection window | ⌃⌘1/2/3 on macOS, Alt+1/2/3 on Windows: the keymap learns a Windows-only spelling |
| One pull request | One per run |

Asked and answered on 2026-10-25: on a Mac every Cmd chord is exact
about Control, so ⌃⌘1 is not ⌘1 (below, "Keys").

## Keys

| Command | macOS | Windows | Omarchy | Where |
| --- | --- | --- | --- | --- |
| Show the rows | ⌃⌘1 | Alt+1 | `r` | A table's tab, any view |
| Show the structure | ⌃⌘2 | Alt+2 | `s` | A table's tab, any view |
| Show the DDL | ⌃⌘3 | Alt+3 | `S` | A table's tab, any view |
| Open the DDL view | | | `:ddl` | The table under the sidebar's cursor while the sidebar has the keyboard, else the table in front |
| Copy the DDL | ⇧⌘C | Ctrl+Shift+C | `y` | The DDL view |
| Open the DDL in the SQL editor | ⌥↩ | Alt+Enter | `e` | The DDL view |
| Move by line | | | `j` `k` ↑ ↓ | The DDL view |
| Top, bottom | | | `gg` `G` | The DDL view |
| Next, previous section | | | `]]` `[[` | The DDL view |
| Select lines, end the selection | | | `V`, Esc | The DDL view |

- The view keys work from Rows, Structure and DDL alike. The DDL view's
  own keys hold only where a table's tab shows its DDL view (a new
  `When::Ddl`), and there they come before the rows' keys that share
  them: `j` `k` (move a row), `V` (select rows), Esc, and on macOS ⇧⌘C,
  which copies the row on the rows and the DDL in the DDL view.
- Esc is the DDL view's only while lines are selected. With none it does
  what it does in the other views: it closes Review SQL.
- In the DDL view the row panel's `[` and `]` (previous and next row) are
  not read: the view shows no row panel, and the brackets begin `[[` and
  `]]`.
- Counts are not read in the DDL view: `3j` moves one line.
- Windows gets keys of its own on a keymap line (`Binding::windows`),
  where the Mac's with Ctrl for Cmd would not do. Settings › Keyboard
  changes and removes them as any other key. The keymap's tests check
  every look's keys.
- On a Mac, a chord with Cmd now goes down only when Control is held
  exactly as the chord says, so ⌃⌘1 is not also ⌘1 (Go to connection
  window 1). This reaches every Cmd chord on a Mac: ⌘1…9, the clipboard's
  ⌘C ⌘X ⌘V, and the Windows layout drawn on a Mac. The only other chord
  spelled with both Cmd and Control is ⌃⌘F, which macOS keeps for full screen and the app leaves alone. Decided with Igor (the
  other way, reading the views' ⌃⌘ chords before Go to connection window,
  was narrower but left ⌃⌘ and ⌘ chords alike everywhere else).
- Replace cell keeps `cc` on Omarchy; `s` is Show the structure.
- Settings › Keyboard lists the commands in the scope "Table", in the
  groups Navigation (the views and the moves), Editing (Copy) and SQL
  (Open in SQL editor).

## A text the view does not show

A Reload that fails keeps the text it had under its error, and the first
read shows no text until it is done. Copy, Open in SQL editor, the moves
and `V` act only on a text the view shows; the header's buttons and the
key line's `y` and `e` show only then. Any answer to a read ends a
selection of lines.

## Copy

- **Copy** (the header's button, or ⇧⌘C / Ctrl+Shift+C) copies the
  text's selection, or all of it with nothing selected, and says "Copied
  DDL" or "Copied 4 lines".
- Every toast in the app is now read out by a screen reader when it
  appears (a polite live region), this one with them.
- ⌘C in the field copies the selection as any field does.
- Omarchy: `y` copies the lines selected with `V`, or all of the text,
  and ends the selection.

## Open in SQL editor

- **Open in SQL editor** (the header's primary button, ⌥↩ / Alt+Enter,
  Omarchy `e`) opens a new SQL editor tab of the connection with the
  whole text, and runs nothing. Its mode is a new tab's: as Settings ›
  Editor says, and read-only on production and on a connection that
  cannot write whatever that says (the app's rule for every new tab).

## The header (the looks with a pointer: macOS and Windows)

- Copy and Open in SQL editor stand at the header's end when the text
  shows. The title gives them room; where the header is short of it they
  drop their keys, then their words (icons only, still named for a screen
  reader), before the title is cut, and go where even a cut title has no
  room (the keys still work). They never cover the view switch. The
  read's spinner gives way to them.
- On production the table's name has the production badge beside it, in
  every view.
- The switch's tabs say their keys in a tooltip ("Rows · ⌃⌘1").

Omarchy: the view words say their letter, dimmed ("r rows · s structure
· S ddl"), where the title leaves room; with none, the words alone.

## The jump links (the looks with a pointer)

- Over the text, before the source line: **Table** (View, Materialized
  view), **Indexes 2**, **Triggers 2**, **Comments 3**, one for each part
  the text has, with how many statements it holds. The object's link has
  no count.
- A link scrolls its part's first line to the top of the field. The part
  at the top is marked (selected, for a screen reader too). A part near
  the end cannot come to the top: its link stays marked until the text is
  scrolled.

## The terminal look

- The text is a view of lines, not a field: the gutter (the cursor's
  number in the warning colour), the SQL editor's colours, the cursor's
  line in the selection colour with an accent bar, the lines selected
  with `V` in the active surface colour. The lines are one stop for the
  keyboard (the Grid region's), a multi-line text named "DDL of
  book_covers" whose value is the text. A click puts the cursor on a
  line.
- The cursor and the selection belong to the tab, so two tabs keep their
  own. A text read again keeps the cursor on a line of it; leaving the
  view ends the selection.
- The outline, 300 points on the right where the view is 700 or wider:
  "sections · ]] next [[ prev"; the parts with their counts (the object's
  counts its lines), the one the cursor is in marked `▌`, each a button
  that moves the cursor there; on PostgreSQL "not included" and what the
  text leaves out; then "keys": `y` copy all, `V y` copy lines, `e` open
  in sql editor, `gg G` top · bottom, `:ddl` open from anywhere, `R` read
  again.
- The status line: "-- VISUAL LINE --" while lines are selected; the
  keys (`j/k` line, `V` select lines, `y` copy, `e` open in sql, `]] [[`
  section, `r` rows, `s` structure, Esc normal while lines are selected,
  then `:w` write and `:diff` review where they apply); what is pending
  in the table's rows and the read-only tag, as in the other views; then
  "2 lines selected · 10/27", or "10/27". It strikes out none of the
  rows' keys.

## Where the build differs from the boards and the prompt

| The board or the prompt | Built | Why |
| --- | --- | --- |
| Omarchy's text as a field | A view of lines with a cursor | The terminal moves by line and selects whole lines (`V`) |
| `/ search` in the outline and the key line | Left out | Find is run 3 |
| The production badge in the DDL view's header | In every view's header | The table is on production whatever the view |
| A Cmd chord on a Mac holds with Control or without | Only with Control as the chord says | ⌃⌘1 would be ⌘1; decided with Igor |
| The board's key line | The board's, then `:w` write, `:diff` review, what is pending and the read-only tag | Run 1's spec: what is pending is saved and reviewed from the view |
| `R` nowhere on the board | In the outline's keys, "read again" | Reload is a key of the view too |
| `table N` in the outline | The object's line count | The object is one statement |
| Open in SQL editor of a selection | The whole text | The board's button names no selection |
| `:ddl` with no table to show (a SQL editor in front) | Nothing happens | The terminal's line says what the prompt refused only over a table |
| A link always takes its part to the top | One near the end stays marked where it stops | A text cannot scroll past its end |

## Tests

- `tabletist-db`: `Ddl::laid_out` gives the text and each part's lines,
  on PostgreSQL and SQLite texts.
- The keymap: the views' keys per look, Windows' Alt digits and
  `Ctrl+1…9` left to the windows, the DDL keys, `s` `r` `S` on the
  grid's letters, `cc` alone for Replace cell; every look's keys spelled
  and checked. `held_is`: ⌃⌘ is not ⌘ on a Mac.
- Headless (`src/ui/ddl_act_tests.rs`):
  - Copy, all and selected, by the button and by the chord, and what it
    says; Open in SQL editor's new tab and its mode, read-only on
    production; the production badge.
  - ⌃⌘ and a digit, and ⌘ and a digit, with two connections open; Alt
    and a digit on Windows; `r s S`; `:ddl` from the tree and from the
    tab in front through the prompt, and with nothing to show.
  - The terminal's moves, parts, `V`, `y`, `e` and what the status line
    says; a click on a line; leaving the view; a shorter text; the row
    panel open; Esc closing Review SQL with no lines selected; what is
    pending kept on the line; the rows' `v y`.
  - Nothing copied, opened or selected of a text the view does not show.
  - A link puts its part's first line at the top of the field (the
    painted line numbers), a part near the end stays marked; each
    engine's links; the outline.
- The earlier tests that read `s` as Replace cell now read it as the
  Structure view.
  - The header's buttons giving way in a narrow window, clear of the
    switch.
- Not tested: the tooltip's words (looked at by eye).
