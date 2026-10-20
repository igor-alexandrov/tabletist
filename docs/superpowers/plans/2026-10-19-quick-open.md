# Plan: quick open

Status: the master plan of the feature, with run 1 planned
(`2026-10-19-quick-open-1-panel.md`). No app code changes with it. Igor
approved the four runs and the departures table on 2026-10-19.

Written on 2026-10-19 against `main` at `3e716bf`. Designs: the four
boards to the right of `Omarchy – Components` in the Components section of
the Tabletist canvas (`macOS – Quick open ⌘P`, `macOS – Quick open ⌘P,
states`, `Omarchy – Find object ctrl+p`, `Omarchy – Find object ctrl+p,
states`), read on 2026-10-19. They are not copied into the repository. The
brief is the prompt "build quick open (⌘P on macOS, ctrl+p on Omarchy)".
Where a board and the brief disagree, the brief wins, and each case is
listed in the run's plan and in its pull request. The brief names
`start-screen-prompt.md` and `hotkeys-fix-prompt.md`. Neither is on this
machine, so the start screen's plans
(`2026-10-19-start-screen.md` and its runs) and the settled keymap
(`docs/superpowers/specs/2026-10-08-keyboard-shortcuts-design.md`,
`src/keymap.rs`) stand in for them.

## What is there today

Quick open exists already, as a small modal (`src/ui/quick_open.rs`,
`Dialog::QuickOpen`, `App::quick_open_found` and `quick_open_matches` in
`src/app.rs`). It holds a field titled "Open table or view", and under it
one list of the loaded tables and views ranked by `util::fuzzy_score`
(capped at 50), plus the Agents tab when what is typed begins "agents".
`↑↓`, `↩` and esc work. It opens on ⌘P (`Command::FindObject`), which is a
global key on macOS and Windows. Omarchy has no key for it. The start
screen's search (start screen run 2, #180) already uses this index:
`App::start_matches` is `quick_open_matches` grouped into tables, then
views.

What the brief needs and the app lacks:

| The brief needs | What the app has |
|---|---|
| Columns of every table, with type and foreign key | Columns are loaded per table, and only when a tab or a completion list asks (`Workspace::columns`, `Command::Describe`). Foreign keys are known only for a table open in a tab (`ObjectTab::structure`). No query lists a schema's columns at once |
| Functions, with return type and definition | No driver lists routines. No code reads a function's definition |
| Saved queries | None. SQL editors are not kept across a quit. The window's editors closed with a script are kept for the session (`Workspace::recent_sql`) and already show in the start screen's recents |
| "Every menu command", and View › Quick Open | No menu bar. macOS has winit's default menu plus Settings and About (`crates/tabletist-appkit`). Commands are the keymap's `Command::ALL` with names (`Command::info`). Each key is turned into an action inline in `ui::keys::handle`, and there is no `Command → Action` map |
| A settings row that opens Settings at that row | `SettingsDialog::row` exists, but only the terminal look draws a cursor there. The macOS sheet neither highlights nor scrolls to a row |
| Reopen closed tabs | Not built (the start screen's run 4) |
| Opening a table scrolled to a column, with its header focused | The grid scrolls to its selected cell. Column headers can't take focus |
| Results from "every open connection window" | One OS window holds every connection as a tab in the connection bar (`App::tabs`). "That connection's window" is its tab there, and "brings it forward" means making that tab active |

## The runs

Each run gets its own plan, its own run and its own pull request, and each
works on its own (see `ship-in-steps`). The brief asks for one pull
request. It is cut into runs because columns, functions and commands each
need work below quick open: a catalog query in three drivers, a command
registry, Settings opening at a row. One run with all of that would take
days of reviews.

1. **The panel, the index and the scopes.**
   - One index per connection, built from the cached tables, views and
     materialized views and kept until the catalog changes
     (`catalog_generation`). It has the brief's tiers (exact, prefix,
     word starts split on `_`, `.` and case changes so that `bc` finds
     `book_covers`, then substring) and its order within a tier (recent
     first, then shorter, then alphabetical). It returns the matched
     spans, so a qualified or word-start match is marked too. The start
     screen's search takes the same index, so it loses the scattered
     matches `fuzzy_score` allowed (the brief: no fuzzy matching beyond
     the tiers).
   - The macOS panel, as the boards draw it: floating, 640 wide, 84 from
     the top, not dimmed; the scope chip (the env dot, the production
     pill, "All open connections"); the empty query's Recent (the start
     screen's five) and its three commands; the typed query's Tables and
     Views groups, each showing 3 rows and then "N more"; the row
     estimates; no match; the footer.
   - The Omarchy picker: the prompt line with `DEV`, `PROD` or `ALL` and
     the match count, the numbered list with its kind column, `1`–`9`
     only while the query is empty, ctrl+n/p.
   - ⇥ and tab switch between this connection and all open connections:
     results grouped by connection, opening one makes its connection tab
     active, and the scope resets on each open.
   - ↩ opens or switches to the tab; ⌥↩ opens `SELECT * … LIMIT n`.
   - The keys: Omarchy `ctrl+p` from any pane, and ⌘P while open selects
     the field's text.
   - The look-native sidebar field opens quick open. Omarchy's `/` still
     filters the tree.
   - The dialog's accessible names and roles.
   - The brief's tests 1, 2, 3 (tables and views), 4, 5, 7, 8, 10, 11 and
     12.
2. **Commands.**
   - A command registry: each built `Command` that quick open can run,
     its name, its key, and whether it applies now (it is hidden if
     not).
   - The `>` prefix on macOS, and the Commands group while typing.
   - One row per setting, opening Settings at that row: the macOS sheet
     learns to highlight and scroll to a row.
   - View › Quick Open in the macOS menu, through `tabletist-appkit`. That
     code can't be built here; it is type-checked for macOS in a scratch
     crate.
   - The brief's test 6.
3. **Columns.**
   - Each driver lists a schema's columns, with their types and foreign
     keys, in one query: PostgreSQL and MySQL from the catalog, SQLite
     table by table through `pragma_table_info` and
     `pragma_foreign_key_list`.
   - The result is kept in the cache, and Reload schema refreshes it.
   - Quick open's Columns group, matched as `table.column`, with
     `→ table` for a foreign key.
   - Opening a column opens its table scrolled to that column, with that
     column's header focused. That needs a header that can take focus,
     which the grid doesn't have.
   - The brief's test 9, and the columns of test 3.
4. **Functions.**
   - PostgreSQL and MySQL list their routines with return types.
     SQLite has none.
   - Quick open's Functions group.
   - Opening a function shows its definition read-only in a SQL tab
     (`pg_get_functiondef`, `SHOW CREATE FUNCTION`), and ⌥↩ does the same.
   - The function of test 3.

Saved queries are no run of this plan (the departures below).

How long: run 1 is about the size of the start screen's run 1 (a day:
the plan with its two reviews, then the run). Runs 2 to 4 are each about
half that.

## Where the brief and the code disagree

These hold for the whole feature. Each run's plan adds its own rows. Each
row needs Igor's yes before run 1 is planned.

| What the brief or a board says | What is built | Why |
|---|---|---|
| One pull request | Four runs, four pull requests | The runs above |
| A Saved queries group, opened in the SQL editor (board: "ebook covers per publisher · Query 1") | No Saved queries group. The window's closed SQL editors stay in Recent, as on the start screen (the board's "Query 1" row is one of those) | The app has no saved queries. Building them (where they are kept, how they are named, saved and deleted) is a feature of its own, which needs its own brief. Quick open would gain the group once they exist |
| Commands: Switch database ⇧⌘D | No macOS key on the row | ⇧⌘D is Review SQL's in the grid, and the settled keymap gives Switch database no macOS key (as on the start screen) |
| Row counts: "6 rows" for a small table, "≈ 6,100 rows" when estimated | Always "≈ N rows" (Omarchy `≈ 6.1k`), from the catalog's estimate. Nothing when there is none (views, SQLite) | Every count the catalog gives is an estimate. The start screen writes them the same way. No count is run |
| Rules card: ranking is exact, word starts, then substring | Exact, prefix, word starts, substring | The brief's order |
| Omarchy typing state: the key line says `1-9 pick` | `1-9 pick` only while the query is empty | The brief |
| The macOS sidebar field opens quick open | It does so in both native looks (macOS and Windows), whose field reads "Find any object…". The Omarchy `/ filter objects` stays a filter of the tree | The Windows look draws the same field, with the same words and Ctrl+P |
| Brief: "Commands that do not apply right now (for example Reopen closed tabs with nothing closed) are hidden" | Reopen closed tabs is not listed until it is built (the start screen's run 4) | It does not exist yet |
| Brief: View › Quick Open | Run 2 | No menu bar exists. Adding one item to the macOS menu is `tabletist-appkit` work that this machine can't build |

## The keys, checked against the keymap

| Key | Where | Check |
|---|---|---|
| ⌘P | macOS, global | Already `FindObject`'s. The test of the brief's test 1 confirms it reaches the panel from the grid, the SQL editor, the start screen and the Agents tab |
| `ctrl+p` | Omarchy, global | Free as a global key. Its other bindings are the "previous" of an open list (the completion list, the key picker, the label chooser, the start screen's search), each under its own `When` and read first while that list is open. The keymap test of shared keys compares only lines with the same `When`, so it passes, and run 1 adds a test that the open list keeps `ctrl+p`. In the SQL editor's insert mode, vim's own `ctrl+p` (keyword completion) is not bound by the app |
| ⇥ (macOS), `tab` (Omarchy) | In quick open only (`When::QuickOpen`, Prompt scope) | Nothing else is read while quick open has the keyboard. In the macOS field ⇥ would move the focus, so the panel takes it first |
| ⌥↩ | macOS, in quick open | As the start screen's search. Omarchy has none |
| `1`–`9` | Omarchy, in quick open, empty query only | The picker's own. They are read only while nothing is typed |
| ctrl+n/p, ↓↑, enter, esc | Omarchy, in quick open | The brief's rule 14 (picking from a list) |

## The brief's open questions, and the defaults taken

1. Commands with an empty query: as drawn (the three under Recent).
2. The sidebar search field opens quick open (in both native looks, as
   above).
3. The keys: as the brief proposes, checked above.
