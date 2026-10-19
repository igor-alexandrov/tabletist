# Plan: the start screen

Status: the master plan of the feature, with run 1 planned
(`2026-10-19-start-screen-1-recents-and-screen.md`). No app code changes
with it.

Written on 2026-10-19 against `main` at `c354a1b`. Designs: the four
boards at the end of the "Empty, loading and error states" section of the
Tabletist canvas (`macOS – Start screen`, `macOS – Start screen, states`,
`Omarchy – Start screen`, `Omarchy – Start screen, states`), read on
2026-10-19. They are not copied into the repository. The brief is the
prompt "build the start screen (no tab open)"; where a board and the
prompt disagree, the prompt wins, and each case is listed in the run's
plan and in its pull request.

## What is built

A connection's window with no open tab shows the start screen in place
of "Select a table or view in the sidebar": the connection in one line,
what was opened lately on the connection (on a first visit, its largest
tables), what is waiting, a search, and three ways to start. It is no tab:
opening anything replaces it, and closing the last tab brings it back. The
macOS sidebar loses its Recent list.

## The runs

Each run is its own plan, run and pull request, and each works on its own
(see `ship-in-steps`). The brief asks for one pull request; it is cut into
four because the search, the sizes and the waiting notes each touch code
the screen itself does not, and a run with all of them would take a day
of reviews.

1. **The recents and the screen** (this run's plan:
   `2026-10-19-start-screen-1-recents-and-screen.md`). `recents.json`,
   kept by connection id; the screen in both looks with the connection
   line (the production strip too), the recents or the largest tables by
   the catalog's estimate, the three actions and the status bar, the
   Omarchy dashboard with its keys (`1..9`, `t`, `d`, `e`, `a`, ctrl+h/l),
   the status line `-- START --`; Switch database built as a command
   (`ctrl+shift+d`); the sidebar's Recent list gone. Tests 1, 2, 3, 4
   (without sizes), 7, 10 and 11 of the brief.
2. **The search.** The 44 pt field, focused when the screen appears;
   typing anywhere in the window goes into it; the results from the ⌘P
   index, grouped Tables and Views (the app has no functions and no saved
   queries), with the matches highlighted; ↑↓, ↩, ⌥↩ (a `SELECT * … LIMIT`
   in a new editor), esc; the footer of keys; "Start typing to search" in
   the status bar. Omarchy: the `/` line, ctrl+n/p, enter, esc. Tests 5
   and 6.
3. **The sizes.** The database's size in the connection line and each
   table's size beside its estimate: `pg_total_relation_size` and
   `pg_database_size`, MySQL's `data_length + index_length`, a SQLite
   file's size; read with the catalog, once per connection. The rest of
   test 4.
4. **What is waiting, and reopening closed tabs.** The muted note of the
   tabs closed in the window, ⇧⌘T and `u`, which reopen them with their
   sort and filters without querying until each is shown; the agent
   write's warning note once approved writes exist (MCP Phase 2: nothing
   waits today). Tests 8 and 9.

## Where the brief and the code disagree

- The brief's search groups Tables, Views, Functions and Saved queries.
  The catalog knows tables, views and materialized views
  (`tabletist_db::ObjectKind`); the app has no functions in its index and
  no saved queries. Run 2 groups Tables and Views.
- The brief's agent note needs a write waiting for approval. Agents can
  only read (`AgentAccess::Off | Read`); approved writes are MCP Phase 2,
  which comes last (see `2026-10-13-mcp-server.md`). Run 4 draws the note
  once there is something to draw.
- The brief's "Switch database ⇧⌘D": ⇧⌘D is Review SQL's in the grid,
  and the settled keymap gives Switch database no macOS key. The button
  carries none; Omarchy has `d` on the screen and `ctrl+shift+d` anywhere.
- The brief says focus starts on the start screen on Omarchy. After a
  connect the keyboard stays in the sidebar, as today (tests pin that the
  tree has `j`, `k` and Enter right after a connect); ctrl+l moves it to
  the screen, which has it once the last tab closes. Run 1 puts this up
  for a yes.
