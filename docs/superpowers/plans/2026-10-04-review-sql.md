# Review SQL Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Step 4 of `docs/superpowers/specs/2026-10-03-value-editing-core-design.md`: before saving, a user reads the statements a save of the tab's pending changes would run. On macOS and Windows they stand in a drawer above the pending bar, toggled by **Review SQL** and **Hide SQL**; on Omarchy in the `:diff` panel, closed with Esc. The Omarchy production box opens that panel and points at it instead of listing the statements itself, wherever the panel is on screen for it.

**Architecture:** The statement builder in `tabletist-db` says where the parts of the statement it shows stand, so nothing in the app reads SQL to lay a statement out or to shorten a value. A pure module, `src/review.rs`, turns a tab's pending set into lines: comment lines as data, statement lines as coloured pieces. The reducer keeps those lines in `Edits::review` while Review SQL is open and makes them again only when the set changed. One view, `src/ui/review.rs`, words the comments and draws the lines: as the drawer, as the panel and inside the production confirmation.

**Tech Stack:** Rust, egui (the crmne fork of 0.36), `tabletist-db` (`Dialect::update_row`, `RowUpdate`), the headless UI harness in `src/testing.rs`.

---

## Before you start

- **All eleven tasks are built,** with what their reviews changed: see "As built", before "What this plan leaves for later". The tasks below are the draft they were built from.
- Cargo is `~/.cargo/bin/cargo`. Never a cargo target dir under `/tmp`. Export both test server URLs when you run the whole suite:

      export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
      export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist

- House rules (`AGENTS.md`): no em dashes anywhere; comments say why, in the surrounding code's voice; no `unsafe`; do not weaken a lint, delete a test or add an `allow`; a view never mutates state except the text a field is editing and a one-shot focus flag; a view never names a font (use `TextRole`s) and never paints a focus ring (use `focus::hint`); every user-visible string in a view goes through `gettext(locale, ..)`; add a focused regression test for every behaviour change.
- Four source scans fail the build and bite here (`src/env.rs`, tests at the end): no file outside `env.rs` may match on `Environment::<name>`; `src/model.rs` may not contain `Color32`; no view may write a `#rrggbb` colour (mix palette colours instead); and only the files in the `surfaces` list may call `env_colors(` (a view takes the production red from `states::Tone::Danger`, which is that colour).
- **The design is the source of truth for how it looks.** It is not in the repository and is never copied into it. What this plan needs from it is written out under "What it looks like" below. No test compares a screen with the design.
- **This plan runs on a new branch off `claude/connection-write`** (or off `main` once pull request #76 is merged). It needs step 3: `ObjectTab::edits`, the pending bar, the production confirmation, Omarchy's `:` prompt.
- Commit after every task. Subjects are plain sentences, each ending with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

  If the commit's signing agent is locked ("agent refused operation"), do not bypass it: `git add -A`, `git write-tree`, and report the tree id with the subject.
- **What was checked when this plan was written.** The model code below (tasks 1 to 3 and the reducer's parts of tasks 8 to 10) was compiled and its tests run against commit `85d1c61` of `claude/connection-write`, with `cargo fmt`, `cargo clippy -D warnings` and `cargo doc -D warnings` clean: once as the tree stands after task 3 (1501 tests of the app's library pass), and once whole (1503). The chord's block of task 7 was compiled in its place in `editing_keys`. No other drawing code was written, so the drawing tasks give behaviour, hooks and tests, not every line.
- **How the code stands** (at `85d1c61`). Read each function before you edit it; where this plan's code and the code disagree, the code wins and you say so in your report.
  - `crates/tabletist-db/src/dialect.rs`: `RowUpdate { shown, sql }` (line 27) is built in one place, `Dialect::update_row` (533), from pieces joined at the end; `key_clause` (516) makes the `WHERE` for it and for `select_row` (559). `Dialect::shown` (375) is the only place a literal is written. The PostgreSQL and SQLite savers pass a `RowUpdate` on whole; MySQL takes its `sql`. The tests' helpers are `change`, `typed`, `one` and `books` (from line 1123).
  - `crates/tabletist-db/src/write.rs`: a `ChangeSet` prints its counts only (a hand-written `Debug`, line 18), so no test may lean on printing a whole set: compare and print its `rows`. `Connection::write` takes a `&StopFlag`, and each driver's save refuses two changes that read the same row. Neither is touched here.
  - `src/edit.rs`: `Edits` (530) holds `cells: BTreeMap<(usize, usize), Pending>`, `editor`, `why`, `saving`, `saved`, `note`, and prints without what was typed (625). `change_set` (691) gives the `ChangeSet` of a set and the page's row of each of its rows, for every pending cell, a cell to fix included. `Table::of` (72) is what editing may know of a tab. `Problem` (225) has `TooLarge` since step 3's review.
  - `src/app/editing.rs`: the set changes in `close_editor` (523: `cells.insert` at 558, `cells.remove` at 560) and `set_null` (567: 587 and 598); `written` (635), the two discards (`DiscardEdits` and `LeaveDiscard` in `src/app.rs`, 860 and 882) and a page that takes a tab's place (`Event::Rows`, near 2213) replace the whole `Edits`. `write_edits` (268) builds the confirmation's statements in a loop over `update_row` (319 to 339) and fails the row whose statement cannot be built. It is also what `LeaveSave` calls, for a tab that need not be in front. `edit_cell` (454) clears `save_refused` (460). `run_command` (608) takes `w` and `e!`. `dropped_under_a_prompt` (26) lists what the two prompts drop. `App::table` (50) reads a tab as a `Table`.
  - `src/app.rs`: `apply_actions` (274) applies the queue and then calls `format_rows` (286), which makes the row panel's text once per change: the house pattern for text made from state. `frame_ui` (3610) runs `apply_actions` before it draws and after. `keys::handle` is not called at all while `app.dialog` is some (3624). The `RevertCell` arm (849) removes its cell at 856; `OpenCommand` and `CloseCommand` are at 555 and 568. `App::copy_text` (3525) is how a view gets text for the clipboard: the view calls `ctx.copy_text`.
  - `src/model.rs`: `Action` (47), `Workspace::{command, focus_command, command_error, save_refused}` (556 to 564, set up near 2557), `WritePrompt` (1195) with `statements: Vec<String>` (1199) and a hand-written `Debug` (2763).
  - `src/ui/pending_bar.rs`: `show` (166) places its buttons from the right with `place` (213), and passes over one auto id when there is no bar (180). `counted` (29) and `block_text` (143) are public to the crate. Its Cancel is a `ButtonSpec::quiet()`.
  - `src/ui/write_prompts.rs`: `statements` (454) draws a `&[String]` as selectable labels in a box that scrolls both ways, at most 220 pt tall; `confirm_sheet` (486) and `confirm_box` (576) both call it. `fitted` (92) sizes a prompt to the window. `confirm_box` matches its foot's keys by their place (0 confirms, 1 cancels).
  - `src/ui/workspace.rs`: `show` (45) adds the footer and the pending bar as bottom panels of the tab's area (101 to 106), only outside the terminal look, inside a central panel that dims with a lost connection (`states::STALE`, line 83). `editing_status` (1604) decides what the terminal's line says (`Said`), `status_line` (1823) lists its keys (`table_hints`, 1885).
  - `src/ui/keys.rs`: `SHORTCUTS` (38) with `Holds`; `editing_keys` (725) reads Mod+S for every look, with `open` and `typing` (736, 738); `letters` (985) is Omarchy's normal mode: the note's Esc at 1094, the row panel's at 1194, `y` at 1220. What the prompt refused is taken away by the next key (the `refused` flag, 180 to 188).
  - `src/ui/focus.rs`: around line 223, how a view asks whether something happened in this frame or the one before (`ctx.cumulative_frame_nr()`, with a tolerance). egui counts a frame at the end of `run_ui`, and a harness frame is one `run_ui`: read after a frame, "this frame" is already the last one.
  - `src/ui/sql_text.rs`: `color_of(TokenKind, &Palette)` (30) is the SQL editor's colour for a kind of token. `src/ui/widgets.rs`: `code` (549), `modal` (756), `ButtonSpec::id` (1329, made of the button's name), `key_hints` (882). `src/ui/terminal_dialog.rs`: `Key` and `keys` (a hint without a button is allowed).
  - `src/testing.rs`: `Harness::editable()` (585) is a writable SQLite table `main.users` with `id` (key), `email`, `meta` and five rows; `answer_written` (649); `painted`, `text_rects`, `fills`, `outlines`, `copied`. A line laid out as one galley is painted as one text, in its first piece's colour.
  - `src/shots.rs`: `EDITING` (359) is an array of a fixed size, seven scenes today.
  - Tests: the nested `mod editing` in `src/app.rs` (from 9776) has `at`, `type_into`, `write_since`, `row`, `written`, `production`, `held`, `writes`, `prompt`, `run`; `object` and `open` come from the module above it. `src/ui/mod.rs`'s tests have `editable_in`, `make_pending`, `leave_pending`, `normal_mode`, `select`, `type_key`, `type_text`, `painted`, `painted_in`, `desktop_looks`, `confirming`, `click_dialog`, `writes`, `command`, `edits`.

## What it looks like

From the design's "Editing values" artboards. Colours are given as what they are, not as numbers.

**macOS and Windows**

- **The bar's button.** In the pending bar, left of "Discard all": **Review SQL**, a text button without a border, in the bar's own text colour. While the drawer is open it reads **Hide SQL**.
- **The drawer** is not drawn in the design. It is built from what the design does draw: the statement block of the production dialog. That block is a bordered box on the window colour, in the code face at a small size with generous line spacing, the statement broken into lines with its keywords ending in one column:

      UPDATE book_covers
         SET kind = 'ebook', alt_text = 'Lighthouse cover…'
       WHERE id = 2

  Keywords are in the SQL editor's keyword colour (purple), quoted values in its string colour (green), numbers in its number colour (orange-brown), names in the text colour. A long value ends in `…` before its closing quote.
- **The production dialog** keeps its statement block, between "Bookshop · bookshop_production · 1 row in book_covers" and the foot with "One transaction", **Cancel** and **Save to production**.

**Omarchy**

- **The `:diff` panel:** a pane with a head, a body and a foot. The head is a darker band with a line under it: `pending · 3 changes · 2 rows` in bold at the left, `one transaction` dimmed at the right. The body is the statements in the code face, loosely spaced: a dimmed comment `-- row id 2`, then the statement in lines, keywords in the keyword colour (magenta), quoted values in green, numbers in the number colour; then, in the danger colour, `-- row id 4 · blocked: fix publisher_id first`. The foot is a darker band of key hints, keys in the accent colour and what they do dimmed: `]c next change` and `u revert under cursor` at the left, `:w write` at the right.
- **Write on PROD:** the box as step 3 built it (2 pt danger border, the `PROD` chip and "write 2 changes?", "Bookshop · bookshop_production", "1 row in book_editions · format, alt_text" dimmed), and in place of the statements one dimmed line: `sql shown with :diff`, the `:diff` in the accent colour. Then "type write to confirm", the field, and the foot with `enter confirm` and `esc cancel`.

**Where what is built departs from the artboards,** each by a decision below:

- Both artboards draw the loaded values as a guard in the `WHERE` (`AND kind = 'print' AND alt_text IS NULL`). The spec rejected that: the `WHERE` is the key only, and the check a save makes is a comment line above the statement (decision 22).
- The artboards write the statement in lower case with bare names on Omarchy; the text here is the statement that runs (decision 4).
- The panel's `]c next change` and `u revert under cursor` are not built (decision 18).
- The bar's button is the app's quiet button, and the panel's foot draws its keys as the status line under it does, not in the accent colour (decision 25).

## What this plan decides beyond the spec

Decisions 10, 15, 18 and 19 are the product owner's answers to questions the first draft of this plan left open; they are settled.

1. **The builder says where its parts stand.** `RowUpdate` gains `parts: UpdateParts`: the byte offsets of `SET`, of every value and of every value's column in `shown`, noted as the statement is written. Shortening one literal and breaking the statement into lines need to know where a value begins and ends. Reading that back out of the text would trust a tokenizer to agree with three databases about quotes and escapes, in the one place a user checks what will be written; a name such as `a SET b` or a value holding ` WHERE ` must not move anything. The layout reads no separator of the builder's: where the parts do not fit what it expects, the statement is shown on one line, whole.
2. **The text is lines of data, and the view words it.** The spec has the reducer build the text; the house rule has every sentence of the app's worded by a view through `gettext`. So `src/review.rs` builds `Line`s: a statement's line is coloured pieces of SQL, and a comment is data (`Line::Row("id 2")`, `Line::Check`, `Line::Blocked`). `src/ui/review.rs` turns a comment into its sentence. No sentence is built in the reducer.
3. **Where it lives, and when it is made.** `Edits::reviewing` says Review SQL is open; `Edits::review` holds the lines, `None` when stale. Every change of the set goes through `Edits::put` and `Edits::revert`, which drop the review, or replaces the whole `Edits`, which closes it. `App::make_reviews` runs at the end of `apply_actions`, after `format_rows`, and builds the review of a tab only when it is open and stale. A frame that changes no set builds nothing; a tab whose review is closed builds nothing ever. One build is one pass over the set, and it runs only when the set changed, so its cost follows the editing and not the frames; it was not timed for this plan. Drawing lays out only the lines in view.
4. **The statement is shown as it is built.** Quoted names, the schema, upper-case keywords, in every look: the spec says the text is the statement that runs, so Omarchy does not lower it and no name loses its quotes. It is laid out in lines as the Omarchy artboard lays it out (`UPDATE` and the table; one line for each value set; one for each column of the key), with only the white space between its words changed, and `;` at its end. A test takes the layout back and compares with `RowUpdate::shown`.
5. **Two comment lines stand above a statement:** `-- row id 2`, as the design has it, then the spec's check, `-- only if kind is still 'print' and alt_text is still NULL`, naming every changed column with what the page loaded. A loaded value reads as the page holds it: `NULL`; a number bare (`612`, `12.5`); `true` or `false` for a boolean the driver read as one (SQLite and MySQL hold 1 and 0, and say so); text in single quotes, as it is, with no doubling of a quote inside it (a comment is read, not run). Bytes never load into a cell that can be edited; were one to, it reads as the grid shows it.
6. **No value ends a comment.** A comment ends at a line break, and what followed one would be SQL to whoever pastes the text. So every name and value in a comment has its line breaks and other hidden characters written out (`<U+000A>`), in what is shown and in what is copied. In a statement that is shown, a value's hidden characters are written out too, so a value with a line break stays on its statement's line and cannot pass for a line of its own.
7. **A row is named by its key, from the change set:** `row id 4`; a key of several columns as `row order_id 7, line 2`. It is the key the statement finds the row by (`Structure::row_key`), in the words the row panel's title and the bar's conflict line use.
8. **A row without a statement says why, in the danger colour.** Cells to fix: `-- row id 4 · blocked: fix publisher_id first`, every column to fix named, joined by a comma. A value the builder refuses (a check the app does not make, see the spec's "Open in the save"): `-- row id 4 · cannot be sent: <the builder's words>`. A set no change set comes of (the table's key is gone): one line, `-- these changes cannot be sent: the table's key is not known`, the disabled Save's sentence in a comment's lower case, in every look.
9. **The cut at 60 characters** is made on the literal as it is shown, quotes included and hidden characters counted as they are written out (a line break is eight): a longer one keeps its beginning, then `…`, then what closes it, 60 at most. What closes it is `'`, or `')` for SQLite's text around a NUL (`('aaa…')`). A quote in a value is written twice, and so is a backslash where it escapes (`E'..'`, MySQL): the cut never falls between the two, since the half that was left would read as the value's end. A check's value is cut to 59 characters and `…`, inside its quotes. Only `review::Values::Shown` cuts; the statement that runs is never built from it.
10. **The lines cannot be selected, and Copy SQL gives the whole statements** (the owner's decision). Selecting what is shown would copy a value cut at `…`, and pasting that would write the cut. So the lines are painted, not a text field, and a **Copy SQL** button puts the whole text on the clipboard: in the drawer's head, and in the production sheet on macOS and Windows, which thereby loses the text selection it had. On Omarchy the key is `Y` while the panel is open. The text is the same lines with every value whole, built when it is asked for and kept nowhere (a value can be a quarter of a megabyte).
11. **What a copied text is.** It opens with one comment line that says so: `-- What Tabletist runs to save these changes, in one transaction. Each statement runs only while its row is still as the comment above it says.` Pasted elsewhere, nothing checks a row and nothing wraps a transaction: the copy is the app's statements, not a script. There is no `BEGIN` or `COMMIT` in it (the app's own differ by driver and are not what is reviewed), and its comments are as shown, their values cut. Two forms can read otherwise in another client, which step 2 left for this step to decide: on MySQL the text assumes backslash escapes, which the app's session keeps on, so in a session with `NO_BACKSLASH_ESCAPES` a backslash is stored doubled; on SQLite a REAL with a very large exponent, written as text, can read back as a neighbouring double where the app binds the exact value. The drawer and the dialog do not say so; the spec does (task 11).
12. **Open is the tab's, and ends with the set.** Each table tab has its own review. It stays open while another tab shows, through a save that is running, a save that failed or conflicted, and a lost connection (it dims with the rest of the tab then, standing in the tab's area). It closes when nothing is pending any more (written, discarded, the last cell reverted) and does not come back by itself with the next change: a drawer that opens unasked takes rows from the grid.
13. **The drawer's size.** As tall as its lines, to at most twelve of them, and never more than half of the tab's area. Past that it scrolls, both ways: a line is never wrapped into what could read as two. The confirmation's box is measured the same way, in lines, where step 3 gave it 220 pt. Neither is resizable in this step.
14. **The drawer's head** reads "Runs in one transaction" (the spec's words) with **Copy SQL** at its right. The counts are in the bar under it. The Omarchy head carries the counts, as its design does, since Omarchy has no bar. The bar's button stays enabled while a save runs: the drawer then shows what was sent.
15. **Review SQL has a chord in every look: `Mod+Shift+D`** (approved by the owner). It shows and hides the drawer or the panel wherever the table's tab shows and something is pending or an editor is open, as `Mod+S` saves there. Step 3 left this open: Omarchy's `:` is matched by the character typed, so on a keyboard layout without it `:diff` cannot be reached; a chord is matched by its key. On macOS and Windows it spares the keyboard a walk to the bar.
16. **Showing the review takes what is being typed,** as a save does: an open editor is closed as a left edit first (its text pending, or to fix), so what is reviewed is what a save would send. An editor that was only opened is no change, and with nothing else pending nothing opens. Hiding leaves an open editor alone.
17. **`:diff` opens, and never closes.** With the panel open it changes nothing (Esc closes). With nothing pending it opens nothing and the status line says `nothing pending` until the next key or the next edit, since a panel that opened empty would say it less plainly. With a SQL editor in front it does nothing, as `:w` does. While something is pending the status line's keys include `:diff review`.
18. **The Omarchy panel** (the owner's decisions). It is a bottom panel, like the drawer: above the status line and the grid's error line, as wide as the grid. The row panel owns the right edge, and one view then serves every look. It does not take the keyboard: the grid keeps its keys, and `u` reverts the active cell while the panel follows. Esc closes the panel before it closes the row panel (a line the last save left is only dismissed with nothing pending, when no panel is open). The wheel scrolls it. `Y` copies the whole SQL while it is open. Its foot says `esc close` and `Y copy sql` at the left and `:w write` at the right; over the two left hints lie hidden buttons named "Hide SQL" and "Copy SQL", so the pointer and a screen reader reach what the keys do. The design's `]c next change` and `u revert under cursor` are left out: they need a cursor in the panel, which is a later slice.
19. **The Omarchy production box and the panel** (the owner's decision, with the review's correction). The reducer opens the panel when it opens the box (`App::look` is the terminal's), with the review the box was made with. Where that panel is on screen for the box's own tab and the window has the room, the box stands above it, in the middle of what the panel leaves, says `sql shown with :diff` in place of the statements, and draws no backdrop: a backdrop would dim the statements the user is asked to read. The pointer cannot reach the panel under a dialog, so Page Up and Page Down scroll it, and the box's foot says so. **Everywhere else the box lists the statements itself,** as step 3 built it: when its tab is not the one in front (a save asked for by the Leave prompt of a tab, a connection or the window can be of a tab that is not drawn), and when the window is too low for the box to stand clear of the panel. No production save is confirmed without its statements on screen. After Cancel the panel stays open (the statements were just declined and are still what is pending); after a save that wrote it closes with the set.
20. **The desktop confirmation keeps listing its statements,** as the spec has it, and draws them as the drawer does: `WritePrompt::statements` becomes `WritePrompt::review`, the same lines, colours and cut, with **Copy SQL** beside "One transaction" (decision 10). No drawer opens behind it.
21. **Colours are the SQL editor's:** `sql_text::color_of` for a keyword, a string, a number and a comment; `NULL` in the keyword colour, as a value and as it would be in the editor; a row without a statement in `Tone::Danger`. Not the editor's tokenizer: the pieces come from the builder (decision 1).
22. **The `WHERE` is the key only,** as the spec decided against the artboards. Nothing here adds a guard to it.
23. **Under a prompt nothing about the review changes.** `ReviewEdits` joins what `dropped_under_a_prompt` drops, so the panel the Omarchy box points at cannot be closed under it by a click that arrived a frame late.
24. **Every line is read to a screen reader:** each row of the drawer, the panel and the confirmation's box carries its text as a label, a comment as it is worded. Only the lines in view exist as widgets.
25. **Two places follow the app's own components, not the artboards' colours.** The bar's Review SQL is a `ButtonSpec::quiet()`, as the bar's Cancel is: without a border, in the secondary colour. The panel's foot draws its keys as `widgets::key_hints` draws the status line's under it: keys in the text colour, what they do dimmed.
26. **The bar's button keeps the keyboard when its name changes.** `ButtonSpec`'s id is made of its name, so a press from the keyboard would lose the keyboard as Review SQL becomes Hide SQL. `ButtonSpec::keyed` gives a button an identity apart from its name.
27. **The shortcuts screen** gains "Mod+Shift+D" in every look, and `:diff`, Esc and `Y` on Omarchy (`Holds::Terminal`).

## Where a run can stop

Each of these leaves the branch shippable:

- **After task 3:** the model is whole and tested; nothing a user sees has changed.
- **After task 5:** macOS and Windows have Review SQL in the bar. Omarchy is as step 3 left it: `:diff` is not a command, and its production box lists its statements.
- **After task 8:** every look reviews, by the chord and by `:diff`. Both production confirmations still list their statements themselves.
- **After task 10:** the step as the spec has it.
- **After task 11:** the scenes and the documents.

After task 6 or task 7 alone the branch is not shippable: the chord opens Omarchy's panel while `:diff` still answers "not a command".

## File map

| File | What it holds |
|---|---|
| `crates/tabletist-db/src/dialect.rs` | `UpdateParts`, `RowUpdate::parts`; `update_row` and `key_clause` note where each value and its column stand. |
| `crates/tabletist-db/src/lib.rs` | Exports `UpdateParts`. |
| `src/review.rs` (new) | `Review`, `Line`, `Piece`, `Ink`, `Values`; `build` and `of`. Pure, with its unit tests. |
| `src/lib.rs` | `pub mod review;` |
| `src/edit.rs` | `Edits::{reviewing, review}`, `Edits::put`, `Edits::revert`. |
| `src/model.rs` | `Action::ReviewEdits`; `Workspace::review_refused`; `WritePrompt::review` in place of `statements`. |
| `src/app.rs` | The `ReviewEdits` arm; `apply_actions` calls `make_reviews`; the `RevertCell` arm; the prompt's arms clear `review_refused`. |
| `src/app/editing.rs` | `review_edits`, `make_reviews`, `review_whole`; `:diff` in `run_command`; `write_edits` makes the confirmation's review and opens Omarchy's panel. |
| `src/ui/review.rs` (new) | The comments' words, the pieces' colours, the lines' drawing, the text for the clipboard, and the drawer or panel itself. |
| `src/ui/mod.rs` | `pub mod review;`, and the headless tests. |
| `src/ui/pending_bar.rs` | The bar's Review SQL and Hide SQL. |
| `src/ui/widgets.rs` | `ButtonSpec::keyed`. |
| `src/ui/workspace.rs` | The panel's place; "nothing pending" and `:diff review` in the status line. |
| `src/ui/keys.rs` | `Mod+Shift+D`; Omarchy's Esc and `Y`; the shortcuts table. |
| `src/ui/write_prompts.rs` | The sheet draws the review's lines and has Copy SQL; the box points at the panel where it can. |
| `src/shots.rs` | The `edit-review` scene. |
| `README.md`, the two specs | Review SQL as built. |

---

### Task 1: The builder says where a statement's parts stand

**Files:**
- Modify: `crates/tabletist-db/src/dialect.rs`, `crates/tabletist-db/src/lib.rs`

- [ ] **Step 1: Write the failing test**

In the test module of `crates/tabletist-db/src/dialect.rs`, before `what_is_shown_is_what_is_bound`:

```rust
    #[test]
    fn an_update_says_where_its_parts_stand() {
        // Names and values that hold what the statement is made of: no
        // reading of the text would find its parts.
        let row = one(
            vec![("id", Value::Int(2)), ("code", text("a WHERE b"))],
            vec![
                typed("kind", "text", "it's, SET = 'x' WHERE 1"),
                change("alt_text", "text", NewValue::Null),
                typed("note", "text", "caf\u{e9} \\ \u{1F600}"),
            ],
        );
        let object = ObjectRef::new("public", "a SET b");
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let update = dialect.update_row(&object, &row).unwrap();
            let (shown, parts) = (&update.shown, &update.parts);
            assert!(shown[parts.set..].starts_with("SET "), "{dialect:?}");
            // The table's name ends where ` SET` begins.
            assert_eq!(
                &shown[..parts.set - 1],
                format!("UPDATE {}", dialect.qualified(&object)),
                "{dialect:?}"
            );
            let values: Vec<&str> = parts
                .values
                .iter()
                .map(|range| &shown[range.clone()])
                .collect();
            assert_eq!(
                values,
                [
                    dialect.literal("it's, SET = 'x' WHERE 1").as_str(),
                    "NULL",
                    dialect.literal("caf\u{e9} \\ \u{1F600}").as_str(),
                    "2",
                    dialect.literal("a WHERE b").as_str(),
                ],
                "{dialect:?}"
            );
            // In the statement's order, the new values before `WHERE` and
            // the key's after it.
            assert!(parts.values.is_sorted_by(|a, b| a.end <= b.start));
            let key = &shown[parts.values[2].end..parts.columns[3]];
            assert_eq!(key, " WHERE ", "{dialect:?}");
            // From where its column begins to the value: the name and `=`,
            // whatever stands between two of them.
            let leads: Vec<&str> = parts
                .columns
                .iter()
                .zip(&parts.values)
                .map(|(column, value)| &shown[*column..value.start])
                .collect();
            let names = ["kind", "alt_text", "note", "id", "code"];
            let expected: Vec<String> = names
                .iter()
                .map(|name| format!("{} = ", dialect.quote_ident(name)))
                .collect();
            assert_eq!(leads, expected, "{dialect:?}");
        }
    }
```

- [ ] **Step 2: Run it, and see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib dialect::`
Expected: does not compile (no field `parts` on `RowUpdate`).

- [ ] **Step 3: Give `RowUpdate` its parts**

Replace `RowUpdate` (line 27) with:

```rust
/// A row's `UPDATE`, as a user reads it and as the driver runs it. Both
/// come from the same values, so they cannot drift apart.
#[derive(Debug, Clone, PartialEq)]
pub struct RowUpdate {
    /// The statement with its values as literals.
    pub shown: String,
    /// What the driver sends: for PostgreSQL the shown text, for MySQL and
    /// SQLite the same statement with the values bound.
    pub sql: Sql,
    /// Where the parts of `shown` stand.
    pub parts: UpdateParts,
}

/// Where the parts of a shown `UPDATE` stand, as byte offsets into its
/// text. The builder notes them as it writes the statement, so what lays
/// the statement out in lines, or shortens a long value where it is shown,
/// reads no SQL to find them: a name or a value that holds ` SET ` or a
/// quote cannot move them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateParts {
    /// Where `SET` begins: the table's name ends a space before it.
    pub set: usize,
    /// Where each value's column begins: one for each of `values`.
    pub columns: Vec<usize>,
    /// Each value's place: the new ones in the order they are set, then
    /// the key's.
    pub values: Vec<std::ops::Range<usize>>,
}

/// A row's key as a `WHERE`, shown and sent, and where its parts stand in
/// the shown clause.
struct KeyClause {
    shown: String,
    sent: String,
    columns: Vec<usize>,
    values: Vec<std::ops::Range<usize>>,
}
```

Replace `key_clause` and `update_row` with:

```rust
    /// ` WHERE "a" = .. AND "b" = ..` for a row's key.
    fn key_clause(self, key: &[(String, Value)], params: &mut Vec<Value>) -> KeyClause {
        let mut clause = KeyClause {
            shown: String::from(" WHERE "),
            sent: String::from(" WHERE "),
            columns: Vec::with_capacity(key.len()),
            values: Vec::with_capacity(key.len()),
        };
        for (index, (column, value)) in key.iter().enumerate() {
            let operand = self.key_operand(value);
            let column = self.quote_ident(column);
            let lead = if index == 0 { "" } else { " AND " };
            clause.shown.push_str(lead);
            clause.columns.push(clause.shown.len());
            let _ = write!(clause.shown, "{column} = ");
            let literal = self.shown(&operand);
            let at = clause.shown.len();
            clause.values.push(at..at + literal.len());
            clause.shown.push_str(&literal);
            let sent = self.sent(&operand, params);
            let _ = write!(clause.sent, "{lead}{column} = {sent}");
        }
        clause
    }

    /// The `UPDATE` of one row of a save. `Err` names the value that cannot
    /// be sent in its column's form.
    pub fn update_row(self, object: &ObjectRef, row: &RowChange) -> Result<RowUpdate> {
        let mut params = Vec::new();
        let mut shown = format!("UPDATE {} SET ", self.qualified(object));
        let mut sent = shown.clone();
        let mut parts = UpdateParts {
            set: shown.len() - "SET ".len(),
            ..UpdateParts::default()
        };
        for (index, change) in row.set.iter().enumerate() {
            let operand = self.new_operand(change)?;
            let column = self.quote_ident(&change.column);
            let lead = if index == 0 { "" } else { ", " };
            shown.push_str(lead);
            parts.columns.push(shown.len());
            let _ = write!(shown, "{column} = ");
            let literal = self.shown(&operand);
            parts.values.push(shown.len()..shown.len() + literal.len());
            shown.push_str(&literal);
            let value = self.sent(&operand, &mut params);
            let _ = write!(sent, "{lead}{column} = {value}");
        }
        let clause = self.key_clause(&row.key, &mut params);
        let base = shown.len();
        parts
            .columns
            .extend(clause.columns.iter().map(|column| base + column));
        parts.values.extend(
            clause
                .values
                .iter()
                .map(|value| base + value.start..base + value.end),
        );
        shown.push_str(&clause.shown);
        sent.push_str(&clause.sent);
        Ok(RowUpdate {
            shown,
            sql: Sql { text: sent, params },
            parts,
        })
    }
```

In `select_row`, the call becomes `let clause = self.key_clause(key, &mut params).sent;`. The file already has `use std::fmt::Write as _;`.

In `crates/tabletist-db/src/lib.rs` the export becomes:

```rust
pub use dialect::{Dialect, RowUpdate, Sql, UpdateParts, escape_like, quote_literal};
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib dialect::`
Expected: 26 passed. The tests that pin the shown and the sent text (`an_update_is_shown_with_its_values_as_literals`, `what_is_shown_is_what_is_bound`, `sqlite_is_shown_what_its_parser_reads`) pass untouched: the statement itself has not changed by a byte.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "Say where the parts of a shown update stand"
```

---

### Task 2: A pending set as the lines of its SQL

**Files:**
- Create: `src/review.rs`
- Modify: `src/lib.rs` (`pub mod review;`, after `pub mod paths;`)

A pure question: given a table's page, its structure and its pending cells, what would a save run, as lines to read. Nothing calls it yet.

- [ ] **Step 1: Write the failing tests**

Create `src/review.rs` with only its test module, and add `pub mod review;` to `src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::Problem;
    use crate::testing::{fixture_structure, page};
    use tabletist_db::{Access, CellChange, NewValue, ObjectKind, RowPage, Structure};

    fn users() -> ObjectRef {
        ObjectRef::new("main", "users")
    }

    fn table<'a>(structure: &'a Structure, page: &'a RowPage) -> Table<'a> {
        Table {
            access: Access::Writable,
            kind: ObjectKind::Table,
            dialect: Dialect::Sqlite,
            structure: Some(structure),
            page,
            refreshing: false,
            saving: false,
        }
    }

    fn ready(text: &str) -> Pending {
        Pending {
            new: NewValue::Text(text.into()),
            state: State::Ready,
        }
    }

    fn cells(list: Vec<((usize, usize), Pending)>) -> BTreeMap<(usize, usize), Pending> {
        list.into_iter().collect()
    }

    /// The statements' lines, as text.
    fn sql(review: &Review) -> Vec<String> {
        review.lines.iter().filter_map(Line::sql).collect()
    }

    /// The statements of `review` with the layout's white space taken back:
    /// each as the builder wrote it, and its semicolon.
    fn unlaid(review: &Review) -> Vec<String> {
        let mut statements: Vec<String> = Vec::new();
        for line in sql(review) {
            match statements.last_mut() {
                Some(statement) if !line.starts_with("UPDATE") => {
                    statement.push(' ');
                    statement.push_str(line.trim_start_matches(' '));
                }
                _ => statements.push(line),
            }
        }
        statements
    }

    #[test]
    fn a_changed_row_is_its_check_and_its_statement_in_lines() {
        let (structure, page) = (fixture_structure(), page(5, false));
        let set = cells(vec![
            ((1, 1), ready("bob@example.com")),
            ((1, 2), ready("{}")),
        ]);
        let review = build(&users(), &table(&structure, &page), &set, Values::Shown).unwrap();
        assert_eq!((review.changes, review.rows), (2, 1));
        assert_eq!(review.refused, None);
        assert_eq!(review.lines[0], Line::Row("id 2".into()));
        // What a save compares before it writes: what the page loaded.
        assert_eq!(
            review.lines[1],
            Line::Check(vec![
                ("email".into(), "'user2@example.com'".into()),
                ("meta".into(), "NULL".into()),
            ])
        );
        assert_eq!(
            sql(&review),
            [
                r#"UPDATE "main"."users""#,
                r#"   SET "email" = 'bob@example.com',"#,
                r#"       "meta" = '{}'"#,
                r#" WHERE "id" = 2;"#,
            ]
        );
        assert_eq!(review.lines.len(), 6);
        // What each piece is drawn as.
        let inks = |line: &Line| match line {
            Line::Sql(pieces) => pieces.iter().map(|piece| piece.ink).collect::<Vec<_>>(),
            other => panic!("{other:?} is no statement's line"),
        };
        assert_eq!(inks(&review.lines[2]), [Ink::Keyword, Ink::Plain]);
        assert_eq!(
            inks(&review.lines[3]),
            [Ink::Plain, Ink::Keyword, Ink::Plain, Ink::Text, Ink::Plain]
        );
        assert_eq!(
            inks(&review.lines[5]),
            [
                Ink::Plain,
                Ink::Keyword,
                Ink::Plain,
                Ink::Number,
                Ink::Plain
            ]
        );
        // The lines are the statement that runs, and nothing else.
        let whole = build(&users(), &table(&structure, &page), &set, Values::Whole).unwrap();
        assert_eq!(
            unlaid(&whole),
            [
                r#"UPDATE "main"."users" SET "email" = 'bob@example.com', "meta" = '{}' WHERE "id" = 2;"#
            ]
        );
        // NULL is a keyword, as a value and in the check.
        let null = cells(vec![(
            (0, 2),
            Pending {
                new: NewValue::Null,
                state: State::Ready,
            },
        )]);
        let review = build(&users(), &table(&structure, &page), &null, Values::Shown).unwrap();
        assert_eq!(
            review.lines[1],
            Line::Check(vec![("meta".into(), r#"'{"plan":"pro"}'"#.into())])
        );
        assert_eq!(sql(&review)[1], r#"   SET "meta" = NULL"#);
        assert_eq!(inks(&review.lines[3])[3], Ink::Keyword);
    }

    #[test]
    fn a_row_with_a_cell_to_fix_is_a_comment_only() {
        let (structure, page) = (fixture_structure(), page(5, false));
        let set = cells(vec![
            ((1, 1), ready("bob@example.com")),
            ((3, 1), ready("dan@example.com")),
            (
                (3, 2),
                Pending {
                    new: NewValue::Text("{oops".into()),
                    state: State::ToFix(Problem::WholeNumber),
                },
            ),
        ]);
        let review = build(&users(), &table(&structure, &page), &set, Values::Shown).unwrap();
        // Every pending cell counts, as the bar counts them.
        assert_eq!((review.changes, review.rows), (3, 2));
        assert_eq!(
            review.lines.last(),
            Some(&Line::Blocked {
                row: "id 4".into(),
                columns: vec!["meta".into()],
            })
        );
        // The other row has its statement, and the blocked one none.
        assert_eq!(unlaid(&review).len(), 1);
        assert!(sql(&review).iter().all(|line| !line.contains("dan@")));
        assert_eq!(review.refused, None);
    }

    #[test]
    fn a_long_value_is_cut_where_it_is_shown_and_whole_where_it_is_copied() {
        let structure = fixture_structure();
        let mut page = page(5, false);
        page.rows[1][1] = Value::Text("o".repeat(200).into());
        let new = "x".repeat(100);
        let set = cells(vec![((1, 1), ready(&new))]);
        let shown = build(&users(), &table(&structure, &page), &set, Values::Shown).unwrap();
        let line = &sql(&shown)[1];
        let literal = line.strip_prefix(r#"   SET "email" = "#).unwrap();
        assert_eq!(literal.chars().count(), VALUE_MAX_CHARS);
        assert_eq!(literal, format!("'{}…'", "x".repeat(57)));
        // The check's value is cut the same way, inside its quotes.
        assert_eq!(
            shown.lines[1],
            Line::Check(vec![("email".into(), format!("'{}…'", "o".repeat(59)))])
        );
        // Sixty characters are shown whole: the quotes are two of them.
        let fits = cells(vec![((1, 1), ready(&"y".repeat(58)))]);
        let review = build(&users(), &table(&structure, &page), &fits, Values::Shown).unwrap();
        assert_eq!(
            sql(&review)[1],
            format!(r#"   SET "email" = '{}'"#, "y".repeat(58))
        );
        // The clipboard gets the statement that runs.
        let whole = build(&users(), &table(&structure, &page), &set, Values::Whole).unwrap();
        assert_eq!(
            unlaid(&whole),
            [format!(
                r#"UPDATE "main"."users" SET "email" = '{new}' WHERE "id" = 2;"#
            )]
        );
        // Its comments are for reading, and stay cut.
        assert_eq!(whole.lines[1], shown.lines[1]);
    }

    #[test]
    fn a_cut_value_still_reads_as_one_value() {
        let cut = |literal: &str| shortened(literal);
        let count = |text: &str| text.chars().count();
        // Sixty characters at most, however the value is written.
        let long = "x".repeat(100);
        for literal in [
            format!("'{long}'"),
            format!("E'{long}'"),
            format!("x'{long}'"),
        ] {
            let shown = cut(&literal);
            assert_eq!(count(&shown), VALUE_MAX_CHARS, "{shown}");
            assert!(shown.ends_with("…'"), "{shown}");
        }
        // A quote in a value is written twice. Cut between the two, the
        // first would close the value before the `…`: it goes as well.
        let quoted = format!("'{}''{long}'", "x".repeat(56));
        assert_eq!(cut(&quoted), format!("'{}…'", "x".repeat(56)));
        // Both halves kept, the pair stays.
        let quoted = format!("'{}''{long}'", "x".repeat(55));
        assert_eq!(cut(&quoted), format!("'{}''…'", "x".repeat(55)));
        // The value's own opening quote is no half of a pair.
        let quotes = format!("'{}'", "''".repeat(40));
        assert_eq!(cut(&quotes), format!("'{}…'", "''".repeat(28)));
        // So with a backslash where it is written twice.
        let slashed = format!("E'{}\\\\{long}'", "x".repeat(55));
        assert_eq!(cut(&slashed), format!("E'{}…'", "x".repeat(55)));
        // SQLite's text around a NUL keeps its brackets.
        let nul = format!("('{long}' || char(0) || 'b')");
        assert_eq!(cut(&nul), format!("('{}…')", "x".repeat(55)));
        // Hidden characters are counted as they are shown: a value of
        // line breaks is sixty characters too, and no marker is cut in
        // two.
        let breaks = format!("'{}'", "\n".repeat(58));
        let shown = cut(&breaks);
        assert_eq!(shown, format!("'{}…'", "<U+000A>".repeat(7)));
        assert!(count(&shown) <= VALUE_MAX_CHARS);
        // What fits is shown whole, with what it hides written out.
        assert_eq!(cut("'a\nb'"), "'a<U+000A>b'");
        assert_eq!(cut("NULL"), "NULL");
        // The check's value: fifty-nine characters and `…` in its quotes.
        let text = Value::Text("o".repeat(200).into());
        assert_eq!(loaded(&text), format!("'{}…'", "o".repeat(59)));
        let fits = Value::Text("o".repeat(60).into());
        assert_eq!(loaded(&fits), format!("'{}'", "o".repeat(60)));
        let breaks = Value::Text("\n".repeat(58).into());
        assert_eq!(loaded(&breaks), format!("'{}…'", "<U+000A>".repeat(7)));
    }

    #[test]
    fn no_value_ends_a_comment_or_passes_for_a_line() {
        let structure = fixture_structure();
        let mut page = page(5, false);
        page.rows[1][1] = Value::Text("a\nDROP TABLE users; --".into());
        let new = "b\n-- row id 9\nUPDATE users SET x = 1;\u{202E}";
        let set = cells(vec![((1, 1), ready(new))]);
        for values in [Values::Shown, Values::Whole] {
            let review = build(&users(), &table(&structure, &page), &set, values).unwrap();
            // A comment is one line, in what is shown and in what is
            // copied.
            assert_eq!(
                review.lines[1],
                Line::Check(vec![(
                    "email".into(),
                    "'a<U+000A>DROP TABLE users; --'".into()
                )]),
                "{values:?}"
            );
        }
        // Shown, the value is on its statement's line, with what it hides
        // written out, and cut as it is shown: sixty characters of it.
        let shown = build(&users(), &table(&structure, &page), &set, Values::Shown).unwrap();
        assert!(sql(&shown).iter().all(|line| !line.contains('\n')));
        assert_eq!(
            sql(&shown)[1],
            r#"   SET "email" = 'b<U+000A>-- row id 9<U+000A>UPDATE users SET x = 1;…'"#
        );
        assert_eq!(sql(&shown).len(), 3);
        // Copied, it is the value itself, inside its quotes.
        let whole = build(&users(), &table(&structure, &page), &set, Values::Whole).unwrap();
        assert_eq!(sql(&whole)[1], format!(r#"   SET "email" = '{new}'"#));
        // A name is shown as safely as a value.
        let row = RowChange {
            key: vec![("id\n".into(), Value::Text("k\n1".into()))],
            set: vec![CellChange {
                column: "na\nme".into(),
                type_name: "text".into(),
                loaded: Value::Null,
                new: NewValue::Text("v".into()),
            }],
        };
        let changes = ChangeSet {
            object: users(),
            rows: vec![row],
        };
        let review = of(Dialect::Postgres, &changes, &[], Values::Shown);
        assert_eq!(review.lines[0], Line::Row("id<U+000A> k 1".into()));
        assert_eq!(
            review.lines[1],
            Line::Check(vec![("na<U+000A>me".into(), "NULL".into())])
        );
        assert!(sql(&review).iter().all(|line| !line.contains('\n')));
    }

    #[test]
    fn a_statement_the_builder_refuses_says_why_and_is_noted() {
        // A whole-number column with a text no check stopped.
        let mut structure = fixture_structure();
        structure.columns[1].type_name = "INTEGER".into();
        let page = page(5, false);
        let set = cells(vec![
            ((1, 1), ready("abc")),
            ((2, 2), ready("{}")),
            ((3, 1), ready("def")),
        ]);
        let review = build(&users(), &table(&structure, &page), &set, Values::Shown).unwrap();
        assert_eq!(
            review.lines[0],
            Line::Refused {
                row: "id 2".into(),
                reason: "email: INTEGER expects a whole number".into(),
            }
        );
        // The first of them, by its place in the set: a save fails there.
        let (index, error) = review.refused.clone().unwrap();
        assert_eq!(index, 0);
        assert_eq!(error.to_string(), "email: INTEGER expects a whole number");
        // The row between them has its statement.
        assert_eq!(unlaid(&review).len(), 1);
        assert!(matches!(review.lines.last(), Some(Line::Refused { row, .. }) if row == "id 4"));
    }

    #[test]
    fn a_set_no_change_set_comes_of_is_one_line_and_none_is_no_review() {
        let page = page(5, false);
        let keyless = Structure {
            primary_key: Vec::new(),
            ..fixture_structure()
        };
        let set = cells(vec![((1, 1), ready("bob")), ((3, 1), ready("dan"))]);
        let review = build(&users(), &table(&keyless, &page), &set, Values::Shown).unwrap();
        assert_eq!(review.lines, [Line::Unsendable]);
        assert_eq!((review.changes, review.rows), (2, 2));
        let structure = fixture_structure();
        assert_eq!(
            build(
                &users(),
                &table(&structure, &page),
                &BTreeMap::new(),
                Values::Shown
            ),
            None
        );
    }

    #[test]
    fn a_row_is_named_by_every_column_of_its_key_in_each_dialects_writing() {
        let change = |column: &str, loaded: Value, new: &str| CellChange {
            column: column.into(),
            type_name: "integer".into(),
            loaded,
            new: NewValue::Text(new.into()),
        };
        let changes = ChangeSet {
            object: ObjectRef::new("shop", "order_lines"),
            rows: vec![RowChange {
                key: vec![
                    ("order_id".into(), Value::Int(7)),
                    ("line".into(), Value::Int(2)),
                ],
                set: vec![
                    change("quantity", Value::Int(1), "3"),
                    change("price", Value::Float(12.5), "13"),
                ],
            }],
        };
        let review = of(Dialect::Postgres, &changes, &[], Values::Shown);
        assert_eq!(review.lines[0], Line::Row("order_id 7, line 2".into()));
        // Numbers the page loaded are bare, as the grid shows them.
        assert_eq!(
            review.lines[1],
            Line::Check(vec![
                ("quantity".into(), "1".into()),
                ("price".into(), "12.5".into()),
            ])
        );
        // PostgreSQL converts a quoted literal itself.
        assert_eq!(
            sql(&review),
            [
                r#"UPDATE "shop"."order_lines""#,
                r#"   SET "quantity" = '3',"#,
                r#"       "price" = '13'"#,
                r#" WHERE "order_id" = 7"#,
                r#"   AND "line" = 2;"#,
            ]
        );
        // MySQL's names, and SQLite's numbers.
        let review = of(Dialect::MySql, &changes, &[], Values::Shown);
        assert_eq!(sql(&review)[0], "UPDATE `shop`.`order_lines`");
        assert_eq!(sql(&review)[4], "   AND `line` = 2;");
        let review = of(Dialect::Sqlite, &changes, &[], Values::Shown);
        assert_eq!(sql(&review)[1], r#"   SET "quantity" = 3,"#);
        // For every dialect the lines are the builder's statement.
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let built = dialect
                .update_row(&changes.object, &changes.rows[0])
                .unwrap();
            let review = of(dialect, &changes, &[], Values::Whole);
            assert_eq!(
                unlaid(&review),
                [format!("{};", built.shown)],
                "{dialect:?}"
            );
        }
        // The review prints without what it holds.
        let printed = format!("{review:?}");
        assert_eq!(printed, "Review { changes: 2, rows: 1, lines: 7 }");
    }
}
```

- [ ] **Step 2: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib review::`
Expected: does not compile (`build`, `of`, `Line`, `Values`, `Review`, `shortened`, `loaded` not found).

- [ ] **Step 3: Write `src/review.rs` above the tests**

```rust
//! Review SQL: the statements a save of a tab's pending changes would run,
//! as lines a person reads before saving. The reducer makes them when the
//! pending set changes; a view words the comment lines and lays the rest
//! out. Nothing here is a sentence of the app's: a comment is data.

use std::collections::BTreeMap;

use tabletist_db::{ChangeSet, Dialect, Error, ObjectRef, RowChange, RowUpdate, Value};

use crate::edit::{Pending, State, Table, change_set};
use crate::ui::format;

/// The most characters of a value that are shown: a longer one is cut, with
/// `…` for the rest. Only where it is shown: the statement that runs, and
/// the text that is copied, hold the whole value.
pub const VALUE_MAX_CHARS: usize = 60;

/// How a piece of a statement reads, for the colour it is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    /// Names, `=`, commas, the semicolon.
    Plain,
    /// `UPDATE`, `SET`, `WHERE`, `AND`, and a `NULL`.
    Keyword,
    /// A quoted value.
    Text,
    Number,
}

/// A stretch of a statement's line in one colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub ink: Ink,
    pub text: String,
}

/// One line of the review. Every text in it is on one line and holds no
/// hidden character, but a whole value in a [`Line::Sql`] (see [`Values`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// `-- row id 2`: the row the statement under it is of, by its key.
    Row(String),
    /// `-- only if kind is still 'print' and alt_text is still NULL`: the
    /// check a save makes before the statement, as each changed column and
    /// what it loaded.
    Check(Vec<(String, String)>),
    /// `-- row id 4 · blocked: fix publisher_id first`: a row with cells to
    /// fix has no statement until they are.
    Blocked { row: String, columns: Vec<String> },
    /// The builder refuses a value of the row, in its own words.
    Refused { row: String, reason: String },
    /// No statement can be made of the set: the table's key is not known.
    Unsendable,
    /// A line of a statement.
    Sql(Vec<Piece>),
}

impl Line {
    /// The text of a statement's line. `None` for a comment, which a view
    /// words.
    pub fn sql(&self) -> Option<String> {
        match self {
            Self::Sql(pieces) => Some(pieces.iter().map(|piece| piece.text.as_str()).collect()),
            _ => None,
        }
    }
}

/// How a statement's values are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Values {
    /// As a person reads them: a long one cut, and every hidden character
    /// (a line break too) written out, so no value passes for a line of
    /// its own.
    Shown,
    /// As the statement holds them: for the clipboard.
    Whole,
}

/// What a save of a tab's pending changes would run.
#[derive(Clone, PartialEq)]
pub struct Review {
    /// How many cells the set changes, and in how many rows.
    pub changes: usize,
    pub rows: usize,
    pub lines: Vec<Line>,
    /// The first row whose statement the builder refused, by its place in
    /// the change set, and why.
    pub refused: Option<(usize, Error)>,
}

/// Without the lines: they hold what the user typed, which stays out of
/// logs and panics.
impl std::fmt::Debug for Review {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Review {{ changes: {}, rows: {}, lines: {} }}",
            self.changes,
            self.rows,
            self.lines.len()
        )
    }
}

/// The review of the pending `cells` of `table`. `None` when nothing is
/// pending.
pub fn build(
    object: &ObjectRef,
    table: &Table<'_>,
    cells: &BTreeMap<(usize, usize), Pending>,
    values: Values,
) -> Option<Review> {
    if cells.is_empty() {
        return None;
    }
    let Some((changes, places)) = change_set(object, table, cells) else {
        // The map is ordered by row: each row's cells are together.
        let mut rows: Vec<usize> = cells.keys().map(|&(row, _)| row).collect();
        rows.dedup();
        return Some(Review {
            changes: cells.len(),
            rows: rows.len(),
            lines: vec![Line::Unsendable],
            refused: None,
        });
    };
    // The columns to fix of each row of the set, in the set's order.
    let blocked: Vec<Vec<String>> = places
        .iter()
        .map(|&row| {
            cells
                .range((row, 0)..=(row, usize::MAX))
                .filter(|(_, cell)| matches!(cell.state, State::ToFix(_)))
                .filter_map(|(&(_, col), _)| table.page.columns.get(col))
                .map(|column| format::display_safe(&column.name).into_owned())
                .collect()
        })
        .collect();
    Some(of(table.dialect, &changes, &blocked, values))
}

/// The review of `changes`, as `dialect` writes them. `blocked` names, for
/// each row of the set, its columns that are to fix: a row with any is a
/// comment only.
pub fn of(
    dialect: Dialect,
    changes: &ChangeSet,
    blocked: &[Vec<String>],
    values: Values,
) -> Review {
    let mut lines = Vec::new();
    let mut refused = None;
    for (index, row) in changes.rows.iter().enumerate() {
        let name = row_name(row);
        if let Some(columns) = blocked.get(index).filter(|columns| !columns.is_empty()) {
            lines.push(Line::Blocked {
                row: name,
                columns: columns.clone(),
            });
            continue;
        }
        match dialect.update_row(&changes.object, row) {
            Ok(update) => {
                lines.push(Line::Row(name));
                let check = row.set.iter().map(|change| {
                    let column = format::display_safe(&change.column).into_owned();
                    (column, loaded(&change.loaded))
                });
                lines.push(Line::Check(check.collect()));
                lines.extend(statement(&update, row.set.len(), values));
            }
            Err(error) => {
                let reason = format::capped(&error.to_string()).into_owned();
                lines.push(Line::Refused {
                    row: name,
                    reason: format::escape_hidden(&reason).into_owned(),
                });
                refused.get_or_insert((index, error));
            }
        }
    }
    Review {
        changes: changes.rows.iter().map(|row| row.set.len()).sum(),
        rows: changes.rows.len(),
        lines,
        refused,
    }
}

/// A row by its key, as the row panel's title names one: `id 2`, several
/// columns joined by a comma. From the key the statement finds the row by.
fn row_name(row: &RowChange) -> String {
    let parts: Vec<String> = row
        .key
        .iter()
        .map(|(column, value)| {
            format!(
                "{} {}",
                format::display_safe(column),
                format::cell_text(value)
            )
        })
        .collect();
    parts.join(", ")
}

/// What a changed cell loaded, as the check's comment says it: text in
/// quotes, everything else as the grid shows it. A long text is cut to 59
/// characters and `…` inside its quotes. Never a line break: a comment
/// ends at one, and what followed would be read as SQL by whoever pastes
/// the text.
fn loaded(value: &Value) -> String {
    let Value::Text(text) = value else {
        return format::cell_text(value).into_owned();
    };
    let (kept, whole) = head(text, VALUE_MAX_CHARS);
    if whole {
        return format!("'{}'", format::escape_hidden(kept));
    }
    let (kept, _) = head(text, VALUE_MAX_CHARS - 1);
    format!("'{}…'", format::escape_hidden(kept))
}

/// The start of `text` that takes at most `room` characters once every
/// hidden character in it is written out (a line break is eight), and
/// whether that is all of `text`. Counted as it is shown, so a value of
/// line breaks is cut to what fits and not to sixty times eight.
fn head(text: &str, room: usize) -> (&str, bool) {
    let mut taken = 0;
    for (at, character) in text.char_indices() {
        let end = at + character.len_utf8();
        taken += format::escape_hidden(&text[at..end]).chars().count();
        if taken > room {
            return (&text[..at], false);
        }
    }
    (text, true)
}

/// The statement in lines, as the design lays one out: the table, each
/// value that is set, each column of the key. Only white space between
/// its words is changed, so the lines read as the statement that runs.
fn statement(update: &RowUpdate, set: usize, values: Values) -> Vec<Line> {
    lay(update, set, values).unwrap_or_else(|| {
        // Not the parts this was written for: the statement on one line.
        vec![Line::Sql(vec![
            plain(&update.shown, values),
            fixed(Ink::Plain, ";"),
        ])]
    })
}

fn lay(update: &RowUpdate, set: usize, values: Values) -> Option<Vec<Line>> {
    let RowUpdate { shown, parts, .. } = update;
    let last = parts.values.len().checked_sub(1)?;
    if set == 0 || set > last {
        return None;
    }
    let table = shown.get(..parts.set)?.strip_prefix("UPDATE")?.trim_end();
    let mut lines = vec![Line::Sql(vec![
        fixed(Ink::Keyword, "UPDATE"),
        plain(table, values),
    ])];
    if parts.columns.len() != parts.values.len() {
        return None;
    }
    for (index, (column, range)) in parts.columns.iter().zip(&parts.values).enumerate() {
        // What stands before the value: its column and `=`.
        let lead = shown.get(*column..range.start)?;
        let literal = shown.get(range.clone())?;
        // The keywords end in one column, as the design sets them.
        let mut pieces = match index {
            0 => vec![fixed(Ink::Plain, "   "), fixed(Ink::Keyword, "SET")],
            _ if index < set => vec![fixed(Ink::Plain, "      ")],
            _ if index == set => vec![fixed(Ink::Plain, " "), fixed(Ink::Keyword, "WHERE")],
            _ => vec![fixed(Ink::Plain, "   "), fixed(Ink::Keyword, "AND")],
        };
        pieces.push(plain(&format!(" {lead}"), values));
        pieces.push(value(literal, values));
        if index + 1 < set {
            pieces.push(fixed(Ink::Plain, ","));
        } else if index == last {
            pieces.push(fixed(Ink::Plain, ";"));
        }
        lines.push(Line::Sql(pieces));
    }
    Some(lines)
}

/// A piece of the review's own writing.
fn fixed(ink: Ink, text: &str) -> Piece {
    Piece {
        ink,
        text: text.to_owned(),
    }
}

/// A piece of the statement that is no value: a name, an `=`.
fn plain(text: &str, values: Values) -> Piece {
    let text = match values {
        Values::Shown => format::escape_hidden(text).into_owned(),
        Values::Whole => text.to_owned(),
    };
    Piece {
        ink: Ink::Plain,
        text,
    }
}

/// A value of the statement, by the literal the builder wrote.
fn value(literal: &str, values: Values) -> Piece {
    let ink = if literal == "NULL" {
        Ink::Keyword
    } else if literal.starts_with(|first: char| first.is_ascii_digit() || first == '-') {
        Ink::Number
    } else {
        Ink::Text
    };
    let text = match values {
        Values::Shown => shortened(literal),
        Values::Whole => literal.to_owned(),
    };
    Piece { ink, text }
}

/// A literal as it is shown, its hidden characters written out: one that
/// would take more than `VALUE_MAX_CHARS` characters keeps its beginning,
/// then `…`, then what closes it (`'`, or `')` for SQLite's text around a
/// NUL), so it still reads as one value.
fn shortened(literal: &str) -> String {
    let (kept, whole) = head(literal, VALUE_MAX_CHARS);
    if whole {
        return format::escape_hidden(kept).into_owned();
    }
    let open = ["E'", "x'", "('", "'"]
        .into_iter()
        .find(|open| literal.starts_with(open))
        .unwrap_or("");
    let close = if literal.ends_with("')") {
        "')"
    } else if literal.ends_with('\'') {
        "'"
    } else {
        ""
    };
    let (mut kept, _) = head(literal, VALUE_MAX_CHARS - 1 - close.len());
    // A quote in a value is written twice, and so is a backslash where it
    // escapes: half of such a pair before the `…` would read as the
    // value's end, or as an escape of what follows. The other half goes
    // too.
    let inside = kept.get(open.len()..).unwrap_or("");
    for mark in ['\'', '\\'] {
        let run = inside
            .chars()
            .rev()
            .take_while(|&last| last == mark)
            .count();
        if run % 2 == 1 {
            kept = &kept[..kept.len() - mark.len_utf8()];
            break;
        }
    }
    format!("{}…{close}", format::escape_hidden(kept))
}
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib review::`
Expected: 8 passed.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "Turn a pending set into the lines of its SQL"
```

---

### Task 3: A tab keeps its review, and makes it when its set changes

**Files:**
- Modify: `src/edit.rs` (`Edits`), `src/model.rs` (`Action::ReviewEdits`), `src/app.rs` (the arm, `apply_actions`, `RevertCell`), `src/app/editing.rs`
- Test: the test modules of `src/edit.rs` and `src/app.rs`

No view yet: the action is sent by tests. After this task a test can open a tab's review, change the set and read the lines a frame leaves.

- [ ] **Step 1: Write the failing tests**

In the test module of `src/edit.rs`, before `the_change_set_names_each_row_by_its_key_and_carries_what_was_loaded`:

```rust
    #[test]
    fn a_change_of_the_set_leaves_its_review_stale() {
        let made = || crate::review::Review {
            changes: 1,
            rows: 1,
            lines: Vec::new(),
            refused: None,
        };
        let pending = || Pending {
            new: NewValue::Text("a secret".into()),
            state: State::Ready,
        };
        let mut edits = Edits {
            reviewing: true,
            review: Some(made()),
            ..Edits::default()
        };
        edits.put((0, 1), pending());
        assert!(edits.review.is_none() && edits.reviewing);
        assert_eq!(edits.counts().changes, 1);
        edits.review = Some(made());
        edits.revert((0, 1));
        assert!(edits.review.is_none() && edits.reviewing);
        assert!(edits.cells.is_empty());
        // A failed statement changes no statement: the review stands.
        edits.put((0, 1), pending());
        edits.review = Some(made());
        edits.fail(Some(0), Error::query("no"));
        assert!(edits.review.is_some());
        // The review is not printed either.
        let printed = format!("{edits:?}");
        assert!(!printed.contains("secret"), "{printed}");
    }
```

In `src/app.rs`, in the nested `mod editing`, after `the_prompt_runs_w_and_e_bang_and_refuses_the_rest`:

```rust
        /// The statements of the tab's Review SQL, as the end of a frame
        /// leaves it: `None` while it is closed.
        fn reviewed(harness: &mut Harness, tab: ConnTabId, id: TabId) -> Option<Vec<String>> {
            // What a frame does once its actions are applied.
            harness.app.apply_actions();
            let review = object(harness, tab, id).edits.review.as_ref()?;
            Some(
                review
                    .lines
                    .iter()
                    .filter_map(crate::review::Line::sql)
                    .collect(),
            )
        }

        fn show_review(harness: &mut Harness, tab: ConnTabId, id: TabId, show: bool) {
            harness.app.apply(Action::ReviewEdits { tab, id, show });
        }

        #[test]
        fn review_sql_is_made_when_it_opens_and_again_when_the_set_changes() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // Nothing pending: there is nothing to open it on.
            show_review(&mut harness, tab, id, true);
            assert!(!object(&harness, tab, id).edits.reviewing);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            // Closed, it is not made: no statement is built for nobody.
            assert_eq!(reviewed(&mut harness, tab, id), None);
            show_review(&mut harness, tab, id, true);
            assert!(object(&harness, tab, id).edits.reviewing);
            let one = [
                r#"UPDATE "main"."users""#,
                r#"   SET "email" = 'bob@example.com'"#,
                r#" WHERE "id" = 2;"#,
            ];
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), one);
            let review = object(&harness, tab, id).edits.review.as_ref().unwrap();
            assert_eq!((review.changes, review.rows), (1, 1));
            // A frame that changes nothing makes nothing again: the review
            // is the one that was made.
            let lines = review.lines.as_ptr();
            harness.app.apply_actions();
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(3, 1),
            });
            harness.app.apply_actions();
            let review = object(&harness, tab, id).edits.review.as_ref().unwrap();
            assert_eq!(review.lines.as_ptr(), lines);
            // A second change: stale at once, and made again by the frame.
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            assert!(object(&harness, tab, id).edits.review.is_none());
            let two = reviewed(&mut harness, tab, id).unwrap();
            assert_eq!(two.len(), 6);
            assert_eq!(two[4], r#"   SET "email" = 'dan@example.com'"#);
            // NULL, and the revert of one cell, change it too.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(0, 2),
            });
            harness.app.apply(Action::SetNull { tab, id });
            let three = reviewed(&mut harness, tab, id).unwrap();
            assert_eq!(three[1], r#"   SET "meta" = NULL"#);
            harness.app.apply(Action::RevertCell { tab, id });
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), two);
            // A cell to fix blocks its row: the row has no statement.
            type_into(&mut harness, tab, id, at(3, 2), "{oops");
            harness.app.apply(Action::LeaveEdit { tab, id });
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), one);
            let review = object(&harness, tab, id).edits.review.as_ref().unwrap();
            assert_eq!(
                review.lines.last(),
                Some(&crate::review::Line::Blocked {
                    row: "id 4".into(),
                    columns: vec!["meta".into()],
                })
            );
            // Hidden, it is dropped.
            show_review(&mut harness, tab, id, false);
            let edits = &object(&harness, tab, id).edits;
            assert!(!edits.reviewing && edits.review.is_none());
            assert_eq!(reviewed(&mut harness, tab, id), None);
        }

        #[test]
        fn showing_the_review_takes_what_is_being_typed() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // Only an editor is open, and typed into: shown, the review
            // holds its text, as a save would send it.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Replace("bob@example.com".into()),
            });
            show_review(&mut harness, tab, id, true);
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none() && edits.reviewing);
            assert_eq!(
                reviewed(&mut harness, tab, id).unwrap()[1],
                r#"   SET "email" = 'bob@example.com'"#
            );
            // A text its column does not take is kept as a cell to fix,
            // and its row is blocked.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(3, 2),
                start: EditStart::Replace("{oops".into()),
            });
            assert!(object(&harness, tab, id).edits.editor.is_some());
            show_review(&mut harness, tab, id, true);
            assert!(object(&harness, tab, id).edits.editor.is_none());
            assert_eq!(object(&harness, tab, id).edits.counts().to_fix, 1);
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 3);
            // Hiding it leaves an open editor alone.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(2, 1),
                start: EditStart::Replace("cy@example.com".into()),
            });
            show_review(&mut harness, tab, id, false);
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_some() && !edits.reviewing);
            // An editor that was only opened is no change: nothing is
            // pending, and nothing opens.
            harness.app.apply(Action::DiscardEdits { tab, id });
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            show_review(&mut harness, tab, id, true);
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none() && !edits.reviewing);
        }

        #[test]
        fn the_review_closes_with_the_set_and_stays_through_a_save_that_wrote_nothing() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            show_review(&mut harness, tab, id, true);
            let made = reviewed(&mut harness, tab, id).unwrap();
            // While the save runs it shows what was sent.
            harness.app.apply(Action::WriteEdits { tab, id });
            assert!(object(&harness, tab, id).edits.saving.is_some());
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), made);
            // A statement that failed leaves the set, and the review of it.
            harness.answer_written(Ok(WriteOutcome::Failed {
                row: 0,
                error: tabletist_db::Error::query("no"),
            }));
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), made);
            assert!(object(&harness, tab, id).edits.reviewing);
            // Written: nothing is pending, and nothing is reviewed.
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(written("bob@example.com"));
            let edits = &object(&harness, tab, id).edits;
            assert!(!edits.reviewing && edits.review.is_none());
            // It does not come back by itself with the next change.
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            assert_eq!(reviewed(&mut harness, tab, id), None);
            // Discarded, and reverted to nothing, it closes as well.
            show_review(&mut harness, tab, id, true);
            harness.app.apply(Action::DiscardEdits { tab, id });
            assert!(!object(&harness, tab, id).edits.reviewing);
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            show_review(&mut harness, tab, id, true);
            assert!(reviewed(&mut harness, tab, id).is_some());
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(3, 1),
            });
            harness.app.apply(Action::RevertCell { tab, id });
            // Open until the frame ends, with nothing to show.
            assert_eq!(reviewed(&mut harness, tab, id), None);
            assert!(!object(&harness, tab, id).edits.reviewing);
        }

        #[test]
        fn each_tab_has_its_own_review_and_keeps_it_while_another_shows() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            show_review(&mut harness, tab, id, true);
            let made = reviewed(&mut harness, tab, id).unwrap();
            let orders = open(&mut harness, tab, "orders", true);
            harness.answer_structure(crate::testing::fixture_structure());
            harness.answer_rows(page(3, false));
            type_into(&mut harness, tab, orders, at(0, 1), "eve@example.com");
            // The other tab's review is its own, and closed.
            assert!(!object(&harness, tab, orders).edits.reviewing);
            harness.app.apply_actions();
            assert!(object(&harness, tab, orders).edits.review.is_none());
            // Back on the first tab it is as it was left.
            harness.app.apply(Action::ActivateTab { tab, id });
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), made);
            // Without a session the statements are still there to read.
            harness.app.workspace_mut(tab).unwrap().status =
                crate::model::SessionStatus::Disconnected(tabletist_db::Error::ConnectionLost(
                    "reset".into(),
                ));
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 6);
        }

        #[test]
        fn what_is_copied_holds_every_value_whole() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            assert!(harness.app.review_whole(tab, id).is_none());
            let long = "x".repeat(100);
            type_into(&mut harness, tab, id, at(1, 1), &long);
            show_review(&mut harness, tab, id, true);
            // Shown, the value is cut at sixty characters.
            let shown = reviewed(&mut harness, tab, id).unwrap();
            assert_eq!(
                shown[1],
                format!(r#"   SET "email" = '{}…'"#, "x".repeat(57))
            );
            // Whole, it is the statement that runs. Asked for with the
            // review closed too: the keys that copy do not open it.
            for show in [true, false] {
                show_review(&mut harness, tab, id, show);
                let whole = harness.app.review_whole(tab, id).unwrap();
                let lines: Vec<String> = whole
                    .lines
                    .iter()
                    .filter_map(crate::review::Line::sql)
                    .collect();
                assert_eq!(lines[1], format!(r#"   SET "email" = '{long}'"#));
            }
            // Asking for it leaves the tab's own review as it was.
            show_review(&mut harness, tab, id, true);
            assert_eq!(reviewed(&mut harness, tab, id).unwrap(), shown);
        }
```

- [ ] **Step 2: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- editing:: edit::`
Expected: does not compile (no fields `reviewing` and `review` on `Edits`, no `Edits::put`, no variant `Action::ReviewEdits`, no `App::review_whole`).

- [ ] **Step 3: The state, in `src/edit.rs`**

`Edits` gains two fields, after `note`:

```rust
    /// Review SQL is open: the drawer on macOS and Windows, the terminal
    /// look's `:diff` panel.
    pub reviewing: bool,
    /// What a save would run, as lines to read: made by the reducer while
    /// `reviewing`. `None` once the set changed, until it is made again.
    pub review: Option<crate::review::Review>,
```

and two methods, in `impl Edits` after `row_mark`:

```rust
    /// Makes `pending` the new value of the cell at `at` (row, column).
    /// What was made of the set, its review, is stale.
    pub fn put(&mut self, at: (usize, usize), pending: Pending) {
        self.cells.insert(at, pending);
        self.review = None;
    }

    /// Takes the cell at `at` out of the set: it is as it loaded again.
    pub fn revert(&mut self, at: (usize, usize)) {
        self.cells.remove(&at);
        self.review = None;
    }
```

`Edits`' hand-written `Debug` stays as it is: it names its fields one by one, and the review is not one of them.

- [ ] **Step 4: The action, in `src/model.rs`**

After `Action::WriteEdits`:

```rust
    /// Show or hide Review SQL: what a save of the tab's pending changes
    /// would run.
    ReviewEdits {
        tab: ConnTabId,
        id: TabId,
        show: bool,
    },
```

- [ ] **Step 5: The reducer**

In `src/app/editing.rs`, add `use crate::review::Values;` under the `crate::model` import. In `close_editor` the set is changed through the two methods:

```rust
        if changed {
            let state = problem.map_or(State::Ready, State::ToFix);
            object.edits.put(key, Pending { new, state });
        } else {
            object.edits.revert(key);
        }
```

and in `set_null`:

```rust
        if changed {
            let null = Pending {
                new: NewValue::Null,
                state: State::Ready,
            };
            object.edits.put(key, null);
            // As opening an editor does: a tab with a pending cell is no
            // preview for the next single click to replace.
            object.pinned = true;
        } else {
            object.edits.revert(key);
        }
```

Before `written`, add:

```rust
    /// Shows or hides the Review SQL of the tab. Shown, it takes the text
    /// being typed first, as a save does: what is reviewed is what a save
    /// would send. It opens only on something pending, and its text is
    /// made at the frame's end (see `make_reviews`).
    pub(super) fn review_edits(&mut self, tab: ConnTabId, id: TabId, show: bool) {
        if show {
            self.close_editor(tab, id, true);
        }
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.edits.reviewing = show && !object.edits.cells.is_empty();
            if !object.edits.reviewing {
                object.edits.review = None;
            }
        }
    }

    /// Makes the Review SQL of every table tab that shows it and whose
    /// pending set changed since it was made, and closes the review of a
    /// tab with nothing pending any more. Once per batch of actions, as
    /// the row panel's text is formatted: a frame that changes no set
    /// builds no statement, and a tab whose review is closed builds none
    /// at all.
    pub(super) fn make_reviews(&mut self) {
        let stale: Vec<(ConnTabId, TabId)> = self
            .open_connections()
            .flat_map(|(tab, workspace)| {
                workspace
                    .object_tabs()
                    .filter(|object| object.edits.reviewing && object.edits.review.is_none())
                    .map(move |object| (tab, object.id))
            })
            .collect();
        for (tab, id) in stale {
            let review = self
                .table(tab, id, |table, object| {
                    crate::review::build(&object.object, table, &object.edits.cells, Values::Shown)
                })
                .flatten();
            if let Some(object) = self.object_tab_mut(tab, id) {
                object.edits.reviewing = review.is_some();
                object.edits.review = review;
            }
        }
    }

    /// The tab's Review SQL with every value whole: what the clipboard
    /// gets. Made when it is asked for, and kept nowhere: a value can be a
    /// quarter of a megabyte, and the review that is drawn holds sixty
    /// characters of each.
    pub fn review_whole(&self, tab: ConnTabId, id: TabId) -> Option<crate::review::Review> {
        self.table(tab, id, |table, object| {
            crate::review::build(&object.object, table, &object.edits.cells, Values::Whole)
        })
        .flatten()
    }
```

In `src/app.rs`: the `RevertCell` arm calls `object.edits.revert((cell.row, cell.col));` in place of `object.edits.cells.remove(..)`; after the `WriteEdits` arm add

```rust
            Action::ReviewEdits { tab, id, show } => self.review_edits(tab, id, show),
```

and `apply_actions` ends with

```rust
        self.format_rows();
        self.make_reviews();
```

Then look for every other write to the set: `grep -rn 'edits\.cells' src` and read each hit that is not a read (`insert`, `remove`, `retain`, `clear`, `range_mut`, `entry`, a `&mut` borrow). Outside the tests and `Edits::put` and `Edits::revert` themselves there must be none left that changes a cell's value: `Edits::fail` changes states only, and no statement with them, so it leaves the review as it is. A place that replaces the whole `Edits` needs nothing: a fresh one has no review and is not reviewing.

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: all pass, the six new ones among them (`a_change_of_the_set_leaves_its_review_stale`, `review_sql_is_made_when_it_opens_and_again_when_the_set_changes`, `showing_the_review_takes_what_is_being_typed`, `the_review_closes_with_the_set_and_stays_through_a_save_that_wrote_nothing`, `each_tab_has_its_own_review_and_keeps_it_while_another_shows`, `what_is_copied_holds_every_value_whole`).

- [ ] **Step 7: The four checks, and commit**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A && git commit -m "Keep a tab's Review SQL and make it when its pending set changes"
```

**The model of Review SQL is whole here.** Nothing a user can do reaches it yet.

---

## The drawing tasks

From here the plan gives each task its behaviour, where it hooks into the code, the shapes it adds and the tests that pin it, not every line: the drawing code has to be written against the functions as they are. Read the named functions first. Every task is test first (a headless test that fails for the stated reason), ends green on the four checks, and is committed alone. Where a task has a reducer's part, that part is given whole.

The headless tests go in the test module of `src/ui/mod.rs` unless said, and use its helpers (`editable_in`, `make_pending`, `leave_pending`, `normal_mode`, `type_key`, `type_text`, `painted`, `painted_in`, `desktop_looks`, `edits`). One more, to add with the first test that needs it:

```rust
    /// Shows the tab's Review SQL, as its button, its chord or `:diff` does.
    fn review(harness: &mut Harness, tab: ConnTabId, id: TabId) {
        let show = true;
        harness.app.apply(Action::ReviewEdits { tab, id, show });
        harness.settle();
    }
```

"No panel is drawn" is asserted by what a panel always paints: its head ("Runs in one transaction" on macOS and Windows, the line that begins `pending · ` in the terminal look) is not among the frame's texts.

### Task 4: Word and colour the lines

**Files:**
- Create: `src/ui/review.rs`
- Modify: `src/ui/mod.rs` (`pub mod review;`, after `pub mod quick_open;`)
- Test: `src/ui/review.rs`

What every place that shows a review shares. No panel yet.

```rust
/// A comment line as it reads, its `--` included. `None` for a line of a
/// statement, which is drawn from its pieces.
pub fn comment(line: &Line, locale: Locale) -> Option<String>

/// The colour a piece of a statement is drawn in: the SQL editor's for
/// the same kind of token.
pub fn ink_color(ink: Ink, palette: &Palette) -> Color32

/// One line in the code face, in its colours, never wrapped.
pub fn line_text(line: &Line, look: &Look, palette: &Palette, locale: Locale) -> Text

/// The height of one line.
pub fn row_height(ctx: &egui::Context, look: &Look) -> f32

/// The most lines a place shows before it scrolls.
pub const MAX_ROWS: usize = 12;

/// The lines `range` of `lines`, one to a row: what a scroll area's
/// `show_rows` draws.
pub fn rows(
    ui: &mut egui::Ui,
    lines: &[Line],
    range: std::ops::Range<usize>,
    look: &Look,
    palette: &Palette,
    locale: Locale,
)

/// A review as the clipboard gets it: the line that says what it is, then
/// one line of text for each of its lines, a line break after each.
pub fn text(review: &Review, locale: Locale) -> String
```

- **The words** (`comment`). Each of the app's own words through `gettext`; a name, a value and the builder's reason as the line holds them (they are made safe in `src/review.rs`). Not through `look.label`: it would lower a name. The words are lower case in every look, as a comment's are.

| Line | Text |
|---|---|
| `Row("id 2")` | `-- row id 2` |
| `Check([("kind", "'print'"), ("alt_text", "NULL")])` | `-- only if kind is still 'print' and alt_text is still NULL` |
| `Blocked { row: "id 4", columns: ["publisher_id"] }` | `-- row id 4 · blocked: fix publisher_id first` (several columns joined by `, `) |
| `Refused { row: "id 4", reason }` | `-- row id 4 · cannot be sent: {reason}` |
| `Unsendable` | `-- these changes cannot be sent: the table's key is not known` |
| `Sql(..)` | `None` |

  The words to translate are `row`, `only if`, `is still`, `and`, `blocked: fix`, `first`, `cannot be sent:` and `these changes cannot be sent: the table's key is not known`. The last is the disabled Save's sentence (`pending_bar::block_text`) with a small first letter: a string of its own, since the bar's begins a sentence and this one stands in a comment, in every look.
- **The colours.** `ink_color`: `Ink::Plain` is `palette.text`; `Keyword`, `Text` and `Number` are `sql_text::color_of` of `TokenKind::Keyword`, `String` and `Number`. A `Row` and a `Check` line are drawn in `color_of(TokenKind::Comment, ..)`; `Blocked`, `Refused` and `Unsendable` in `Tone::Danger.color(palette)`.
- **A line** (`line_text`): role `widgets::code(look)`. A comment is one piece; a statement's line is its pieces through `Text::add_runs`, each in its ink's colour. Leading spaces are part of the first piece and must be drawn.
- **A row's height** (`row_height`): the code role's own (`TextRole::row_height`) and 4 pt, the room the confirmation's lines have between them today.
- **A row** (`rows`): each line takes `row_height` points: allocate the row as wide as the laid line (or the width there is, when that is more), paint the line at its left, centred on the row, and give the row the line's text as a label for a screen reader (`WidgetInfo::labeled(WidgetType::Label, ..)`; a comment as `comment` words it). No `Sense::click`, no selectable label (decision 10). **The caller sets the item spacing to zero on the `ui` that calls `show_rows`, before the call:** `show_rows` adds that `ui`'s `item_spacing.y` to the row height it is given, so zeroing it inside the rows is too late and the lines would drift from their rows.
- **The clipboard's text** (`text`): first the line `-- What Tabletist runs to save these changes, in one transaction. Each statement runs only while its row is still as the comment above it says.` (one string through `gettext`, the `-- ` put before it), then `comment(line)` or `line.sql()` for each line; a `\n` after every line.

**Tests** (in `src/ui/review.rs`):

- `every_comment_reads_as_the_spec_writes_it`: the five comment rows of the table above, to the letter; `Blocked` with two columns reads `-- row id 4 · blocked: fix publisher_id, pages first`; a `Sql` line gives `None`.
- `a_piece_is_drawn_in_the_sql_editors_colour`: for `Palette::light()` and `Palette::dark()`, `ink_color` of `Keyword`, `Text` and `Number` equals `color_of` of `TokenKind::Keyword`, `String` and `Number`, and `Plain` is `palette.text`.
- `a_line_is_one_row_and_keeps_its_leading_spaces`: on a `Harness`'s `ctx`, `line_text` of a `Line::Sql` built by `review::of` for the fixture lays out to one row (`laid.galley.rows.len() == 1`) whose text is `   SET "email" = 'bob@example.com'`, and its height is at most `row_height`.
- `rows_stand_one_row_height_apart`: in a `Harness::frame_with`, a `ScrollArea` with zero item spacing and `show_rows` over six lines: the painted lines' tops are exactly `row_height` apart, the first to the last.
- `the_clipboards_text_says_what_it_is_and_holds_the_whole_statements`: for `review::of(Dialect::Sqlite, &changes, &[], Values::Whole)` of one row of the fixture with a 100 character value, `text` is exactly

      -- What Tabletist runs to save these changes, in one transaction. Each statement runs only while its row is still as the comment above it says.
      -- row id 2
      -- only if email is still 'user2@example.com'
      UPDATE "main"."users"
         SET "email" = '<the 100 characters>'
       WHERE "id" = 2;

  with a line break after the last line, and its first line holds no line break of its own.

Commit: "Word and colour the lines of a review".

---

### Task 5: The drawer and the bar's button

**Files:**
- Modify: `src/ui/review.rs` (`show`, `copy_text`, `placed`), `src/ui/workspace.rs` (`show`), `src/ui/pending_bar.rs` (`show`), `src/ui/widgets.rs` (`ButtonSpec::keyed`)
- Test: `src/ui/mod.rs`, `src/ui/widgets.rs`

```rust
/// The Review SQL of the table tab `id`, while it is open: a bottom panel.
/// Called after the pending bar it stands on it; in the terminal look it
/// stands on the status line.
pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId)

/// What Copy SQL puts on the clipboard: the tab's review with every value
/// whole. `None` when nothing is pending.
pub fn copy_text(app: &App, tab: ConnTabId, id: TabId) -> Option<String>

/// A panel that was drawn: whose it is, and where it stood.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub tab: ConnTabId,
    pub id: TabId,
    pub rect: egui::Rect,
}

/// The panel drawn in the frame being drawn, or in the one that just
/// ended: what stands clear of it asks, and so does a test.
pub fn placed(ctx: &egui::Context) -> Option<Placed>
```

**The hook.** In `workspace::show`, in the arm of an object tab, right after the block that adds the footer and the pending bar:

```rust
                    if !look.terminal {
                        super::data_view::footer(app, ui, tab, object_tab);
                        // Bottom panels stack upwards: the bar of pending
                        // changes stands on the footer.
                        super::pending_bar::show(app, ui, tab, object_tab);
                    }
                    // And what a save would run stands on the bar: in the
                    // terminal look, on the status line.
                    super::review::show(app, ui, tab, object_tab);
```

It is called for a table tab in either view (Data and Structure), as the bar is. It stands inside the tab's central panel, so it dims with the rest of the tab when the connection is lost.

**`show`,** in this task with one head for every look (task 6 gives the terminal look its own, and nothing opens the panel there before task 7):

- Drawn only while the tab's `edits.reviewing` is set and `edits.review` is some. Otherwise `ui.skip_ahead_auto_ids(1)` and return, as `pending_bar::show` does and for its reason: a panel takes one of its parent's ids, and what is drawn after it (the tab's central panel, and everything in it) must be the same widgets with the drawer and without.
- An `egui::Panel::bottom(Id::new(("review-sql", tab.0, id.0)))`, `exact_size`, not resizable, no separator line. Its height: the head (32 pt) and the lines' body. The body is `lines.len().min(MAX_ROWS)` rows of `row_height` and 16 pt (8 above and below), and less where the whole panel would take more than half of `ui.available_height()` at the call.
- **The body:** fill `palette.window`. `egui::ScrollArea::both()` with an id salt of the tab's, `auto_shrink([false, false])`, `show_rows(ui, row_height, lines.len(), ..)` calling `review::rows`, the item spacing set to zero before the call (task 4). Left and right padding as the neighbour under it has: 20 pt on macOS and Windows (the bar's `SIDE`), 12 pt in the terminal look (the status line's).
- **The head.** Fill `palette.panel`, a hairline on the panel's top edge and one under the head in `palette.outline` (`widgets::hline`). At the left "Runs in one transaction" in `widgets::body(look)` and `palette.secondary`; at the right **Copy SQL**, a `ButtonSpec::new(..).quiet()` placed with `show_at` as the bar places its buttons. Pressed, it calls `copy_text` and `ui.ctx().copy_text(..)`.
- **Where it stood:** after the panel is drawn, keep `Placed { tab, id, rect }` and the frame's number (`ctx.cumulative_frame_nr()`) in egui's temporary data (`ctx.data_mut(|data| data.insert_temp(..))`). `placed` gives it back while the frame's number is the one it was kept with or the next: the frame being drawn, for what is drawn after the panel, and the frame that just ended, for a test that reads after `Harness::frame` (egui counts the frame at its end). Task 10 reads it.
- The view changes no state: it pushes actions and puts text on the clipboard.

**`copy_text`:** `app.review_whole(tab, id).map(|review| text(&review, app.locale))`.

**The bar's button** (`pending_bar::show`). In the branch that draws Save and Discard all (`if pending || saving`), and only while something is pending, place one more button after the others, so it stands left of them: `ButtonSpec::new(..).quiet().keyed("review-sql")`, reading **Review SQL**, or **Hide SQL** while `edits.reviewing`. Pressed, it pushes `Action::ReviewEdits { tab, id, show: !reviewing }`. It stays enabled while a save runs.

**`ButtonSpec::keyed`** (`widgets.rs`): a new optional field `key: Option<&'a str>` and

```rust
    /// The button's identity, where its name changes with what it would
    /// do next (Review SQL, Hide SQL): the keyboard stays on it through
    /// the change.
    pub fn keyed(mut self, key: &'a str) -> Self
```

`ButtonSpec::id` is then made of `self.key.or(self.label).unwrap_or(self.text)`. Every other button's id is what it was.

**Tests**

- `widgets.rs`: `a_keyed_button_keeps_the_keyboard_when_its_name_changes`: draw `ButtonSpec::new("Review SQL").keyed("review")` in a frame and give it the keyboard (`response.request_focus()`); in the next frame draw `ButtonSpec::new("Hide SQL").keyed("review")` in the same place: `has_keyboard` is true. Without `keyed` the same two frames lose it (the control case).
- `review_sql_opens_a_drawer_above_the_bar_and_hide_sql_closes_it` (`desktop_looks()`): with two cells pending in two rows, no button reads "Hide SQL" and no panel is drawn; `harness.click("Review SQL")`: `edits.reviewing` is set, the frame paints "Runs in one transaction", `-- row id 2`, `-- only if email is still 'user2@example.com'`, `UPDATE "main"."users"`, `   SET "email" = 'bob@example.com'` and ` WHERE "id" = 2;`; the last painted line is above the bar (`painted_rect(" WHERE \"id\" = 4;")` ends above where `painted_rect("2 changes in 2 rows")` begins) and the grid's first row is still painted; `review::placed(&harness.ctx)` names this tab and its rect holds every painted line; `harness.click("Hide SQL")`: no panel is drawn and the button reads "Review SQL" again.
- `the_lines_wear_the_sql_editors_colours` (`desktop_looks()`, `email` of row 2 pending and the review opened by `review`): `painted_in` of `UPDATE "main"."users"` in `palette.magenta` (a line is painted in its first piece's colour), of `-- row id 2` in `palette.dim`, and, with a cell to fix in row 4 (`leave_pending(.., (3, 2), "{oops")`), of `-- row id 4 · blocked: fix meta first` in `palette.danger`; that row has no ` WHERE "id" = 4;`.
- `a_screen_reader_reads_every_line_in_view`: `harness.has` each of `-- row id 2`, `-- only if email is still 'user2@example.com'`, `UPDATE "main"."users"`, `   SET "email" = 'bob@example.com'` and ` WHERE "id" = 2;`.
- `a_long_value_is_cut_in_the_drawer_and_copy_sql_takes_it_whole` (`desktop_looks()`): a pending value of 100 characters is painted as `   SET "email" = '` + 57 characters + `…'`; `harness.click("Copy SQL")`: `harness.copied` is the text of task 4's last test, the value whole.
- `the_drawer_scrolls_what_does_not_fit_and_leaves_the_grid_half_its_room` (`desktop_looks()`): with all five rows changed (25 lines, more than `MAX_ROWS`), `review::placed` gives a rect no taller than half the window, the first line `-- row id 1` is painted and the last statement's ` WHERE "id" = 5;` is not; the grid's first row is still painted. In a window 400 pt tall (`Harness::with_size`) the panel is at most half of the tab's area.
- `the_drawer_is_its_tabs_and_goes_with_the_set` (`desktop_looks()`): open on `users`; open `orders` beside it (no panel is drawn there); back on `users` the lines are painted again; after `Action::DiscardEdits` no panel is drawn.
- `a_field_keeps_the_keyboard_when_the_drawer_opens_and_closes` (`desktop_looks()`): open the filter bar (`Mod+F`), so one of its fields has the keyboard, with a cell pending; `review` opens the drawer: `harness.ctx.text_edit_focused()` still, and what is typed next goes into the filter's field; hide it again (`Action::ReviewEdits { show: false }`): the same. This is what `skip_ahead_auto_ids` is for (the cell editor's own field has an id of its own and would not notice).
- `a_sql_result_and_a_table_without_changes_show_no_drawer`: nothing labelled "Review SQL" on a table with nothing pending, nor on a SQL editor's result.

Two commits, each green on the four checks: the keyed button ("Let a button keep the keyboard when its name changes"); then the panel and the bar's button ("Review the SQL of the pending changes in a drawer").

**macOS and Windows have Review SQL here.**

---

### Task 6: The terminal look's head and foot

**Files:**
- Modify: `src/ui/review.rs` (`show`)
- Test: `src/ui/mod.rs`

In the terminal look the panel has the design's head, and a foot. Nothing opens it there yet but a test's action; the chord (task 7) and `:diff` (task 8) do.

- **Height:** the head (32 pt), the body, and the foot (28 pt).
- **Head:** fill `palette.panel`, hairlines above and under it in `palette.outline`; at the left `pending · {n} changes · {m} rows` (`look.label` of "pending" and of `pending_bar::counted` for each count, joined by ` · `; `1 change · 1 row` in the singular) in `TextRole::OGroup` and `palette.text`; at the right `one transaction` in `TextRole::OSecondary` and `palette.dim`. No Copy SQL button in the head: the foot has it.
- **Foot:** fill `palette.panel` under a hairline. At the left the hints `esc close` and `Y copy sql`, at the right `:w write` (left out on a read-only connection, as the status line leaves it out), each drawn as `widgets::key_hints` draws the status line's: the key in the text colour, what it does dimmed (decision 25). Over each of the two left hints a `ButtonSpec::hidden_at` named "Hide SQL" and "Copy SQL", so the pointer and a screen reader reach what the keys do: the first pushes `Action::ReviewEdits { show: false }`, the second copies.
- **Its place among the terminal look's lines.** The panel is a bottom panel of the tab's area, and the grid's error line (`! 4:meta  …`) is drawn with the grid, in what the panels leave. So from the top: the grid, the error line, the panel, the status line.

**Tests** (`Look::omarchy()`, the review opened by `review`)

- `the_terminal_panel_has_its_head_and_its_foot`: with two cells pending in two rows, `pending · 2 changes · 2 rows` and `one transaction` are painted, and the hints `esc close` and `Y copy sql`, all inside `review::placed`'s rect; "Runs in one transaction" is not painted; with one cell in one row the head reads `pending · 1 change · 1 row`; `harness.click("Hide SQL")` closes the panel, and `harness.click("Copy SQL")` leaves the whole text in `harness.copied`.
- `the_terminal_panel_stands_between_the_error_line_and_the_status_line`: with a cell to fix (`leave_pending(.., (3, 2), "{oops")`) and another pending, from the top by their painted rects: the grid's last row, the error line (the text that begins `! 4:meta`), the panel's head, the panel's last line, and the status line's `j/k row`. None overlaps the next.
- `the_terminal_panel_wears_the_same_colours_and_scrolls_the_same`: `painted_in` of `UPDATE "main"."users"` in `palette.magenta` and of the blocked row's line in `palette.danger`; with all five rows changed the first line is painted and the last is not, and the panel is no taller than half the window.
- `the_terminal_panel_is_its_tabs_and_goes_with_the_set`: as `the_drawer_is_its_tabs_and_goes_with_the_set`, by the head that begins `pending · `.

Commit: "Give Omarchy's SQL panel its head and its foot".

---

### Task 7: The chord

**Files:**
- Modify: `src/ui/keys.rs` (`editing_keys`, `SHORTCUTS`)
- Test: `src/ui/mod.rs`, `src/ui/keys.rs`

In `editing_keys`, right after the block that reads Mod+S (`open` and `typing` are the ones that block uses):

```rust
    // Review SQL by the same rule: wherever the table's tab shows, with
    // something pending or an editor open, in every look. A fresh press
    // only: a held chord would show and hide it by turns.
    let review = |input: &mut egui::InputState| {
        consume_press(input, Modifiers::COMMAND | Modifiers::SHIFT, Key::D)
    };
    if (open || !object.edits.cells.is_empty()) && ctx.input_mut(review) {
        // Shown, the review takes what is being typed, as a save does:
        // noted as typed for the reason the save notes it.
        if typing {
            actions.push(Action::EditorTyped { tab, id });
        }
        let show = !object.edits.reviewing;
        actions.push(Action::ReviewEdits { tab, id, show });
    }
```

`editing_keys` runs only while no dialog is up, and only on a table's tab: a SQL editor keeps the chord for itself. What `ReviewEdits` does with an open editor is the reducer's (task 3, decision 16): shown, the editor is closed as a left edit and its text is in the review; hidden, the editor is left alone.

`SHORTCUTS` gains, after the row of `:e!`:

```rust
    (
        "Mod+Shift+D",
        "Show or hide the SQL of the pending changes",
        ALL,
    ),
```

**Tests**

- `mod_shift_d_shows_and_hides_the_review_in_every_look` (`Look::ALL`): with a pending cell, `harness.press(Key::D, Modifiers::COMMAND | Modifiers::SHIFT)` sets `edits.reviewing` and the lines are painted; a second press hides them; with nothing pending and no editor open the chord does nothing; with the keyboard on the tree (`Pane::Tree`) and in the Structure view it works all the same.
- `the_chord_takes_what_is_being_typed` (`desktop_looks()`): with only an editor open on `(1, 1)` and `bob@example.com` typed into it (`type_text`), the chord closes the editor, the cell is pending, and the drawer's lines include `   SET "email" = 'bob@example.com'`; with the review open and an editor open on another cell, the chord hides the review and the editor keeps its text and the keyboard; with only an editor that was opened and not typed into, the chord closes it and opens nothing.
- `a_held_chord_shows_the_review_once`: the key down, then two repeats (`egui::Event::Key { repeat: true, .. }`): the review is open.
- `keys.rs`: `the_shortcut_table_names_the_keys_that_review_sql`: in every look `shortcuts(&look)` holds ("Mod+Shift+D", "Show or hide the SQL of the pending changes").

**By hand, by the user:** on macOS, that Cmd+Shift+D reaches the app with a table's tab in front (no menu item of the system's takes it).

Commit: "Show and hide Review SQL with a chord".

---

### Task 8: Omarchy's `:diff`, Esc and `Y`

**Files:**
- Modify: `src/model.rs` (`Workspace::review_refused`), `src/app.rs` (the prompt's arms), `src/app/editing.rs` (`run_command`, `edit_cell`), `src/ui/keys.rs` (`handle`, `letters`, `SHORTCUTS`), `src/ui/workspace.rs` (`editing_status`, `status_line`)
- Test: `src/app.rs`, `src/ui/mod.rs`, `src/ui/keys.rs`

**The reducer's part, whole.**

- [ ] **Step 1: Write the failing test**

In `src/app.rs`, in `mod editing`, after the tests of task 3:

```rust
        #[test]
        fn the_prompt_shows_the_review_with_diff_and_says_when_nothing_is_pending() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            let refused = |harness: &Harness| harness.app.workspace(tab).unwrap().review_refused;
            // Nothing pending: nothing opens, and the line says so. It is
            // no mistake of the typing.
            run(&mut harness, tab, "diff");
            assert!(refused(&harness));
            assert_eq!(prompt(&harness, tab), (None, None));
            assert!(!object(&harness, tab, id).edits.reviewing);
            // The prompt takes the message away, opened or closed.
            harness.app.apply(Action::OpenCommand(tab));
            assert!(!refused(&harness));
            run(&mut harness, tab, "diff");
            harness.app.apply(Action::CloseCommand(tab));
            assert!(!refused(&harness));
            // And so does an edit begun without a key: a double-click.
            run(&mut harness, tab, "diff");
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(2, 1),
                start: EditStart::Value,
            });
            assert!(!refused(&harness));
            harness.app.apply(Action::CancelEdit { tab, id });
            // With something pending it opens, the spaces round it
            // overlooked, and sends nothing.
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            run(&mut harness, tab, " diff ");
            assert!(!refused(&harness));
            assert_eq!(prompt(&harness, tab), (None, None));
            assert!(object(&harness, tab, id).edits.reviewing);
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 3);
            assert_eq!(writes(&harness), 0);
            // Again, it stays open: only Esc closes it.
            run(&mut harness, tab, "diff");
            assert!(object(&harness, tab, id).edits.reviewing);
            // With a SQL editor in front there is no table to review.
            show_review(&mut harness, tab, id, false);
            harness.app.apply(Action::NewSqlTab(tab));
            run(&mut harness, tab, "diff");
            assert!(!object(&harness, tab, id).edits.reviewing);
            assert!(!refused(&harness));
            assert_eq!(prompt(&harness, tab), (None, None));
        }
```

In `the_prompt_runs_w_and_e_bang_and_refuses_the_rest`, `diff` leaves the list of what is not a command, and two near misses join it:

```rust
            // Anything else is not a command: it is kept to say so, and
            // nothing is sent or dropped.
            for text in ["wq", "W", "e", "w!", "e !", "diff!", "Diff"] {
```

- [ ] **Step 2: Run it, and see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib editing::`
Expected: does not compile (no field `review_refused` on `Workspace`).

- [ ] **Step 3: The flag and the command**

`Workspace` gains, after `save_refused`:

```rust
    /// `:diff` was run with nothing pending. The terminal's line says so,
    /// until the next key.
    pub review_refused: bool,
```

set to `false` in `Workspace::new` beside `save_refused`. In `src/app.rs`, the `OpenCommand` and `CloseCommand` arms clear it where they clear `save_refused`:

```rust
                    workspace.save_refused = false;
                    workspace.review_refused = false;
```

and so does `edit_cell` in `src/app/editing.rs`, in the same place and for the same reason (what the edit comes to is what the line says next; a double-click begins an edit without a key):

```rust
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.save_refused = false;
            workspace.review_refused = false;
        }
```

`run_command` becomes:

```rust
    /// Runs what the terminal's `:` prompt holds, and closes it: `w` saves
    /// the pending changes of the table on screen, `e!` drops them and
    /// `diff` shows what a save would run, as their keys and the bar's
    /// buttons do in the other looks. Any other text is not a command, and
    /// is kept for the status line to say so.
    pub(super) fn run_command(&mut self, tab: ConnTabId) {
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        workspace.focus_command = false;
        workspace.command_error = None;
        workspace.save_refused = false;
        workspace.review_refused = false;
        let Some(text) = workspace.command.take() else {
            return;
        };
        // The table on screen, and whether anything is pending in it.
        let table = workspace
            .active_object_tab()
            .map(|object| (object.id, !object.edits.cells.is_empty()));
        let action = match (text.trim(), table) {
            ("", _) => return,
            ("w", Some((id, _))) => Action::WriteEdits { tab, id },
            ("e!", Some((id, _))) => Action::DiscardEdits { tab, id },
            ("diff", Some((id, true))) => Action::ReviewEdits {
                tab,
                id,
                show: true,
            },
            // Nothing to show: the line says so, where a panel that opened
            // empty would say it less plainly.
            ("diff", Some((_, false))) => {
                workspace.review_refused = true;
                return;
            }
            // A SQL editor is in front: there is no table to act on.
            ("w" | "e!" | "diff", None) => return,
            (other, _) => {
                workspace.command_error = Some(other.to_owned());
                return;
            }
        };
        // As from a key: under a question about the changes it is dropped.
        self.apply(action);
    }
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib editing::`
Expected: all pass. In the whole suite `ui::tests::an_unknown_command_says_so` now fails (it runs `diff` with a cell pending and expects "not a command: diff"): put it right as the list of tests below says, before the first commit.

**The drawing part.**

- **What was refused is taken away by the next key** (`keys::handle`, the `refused` flag near line 186): `workspace.review_refused` counts as `command_error.is_some()` does, so the next fresh key pushes `CloseCommand`, which clears it.
- **The status line** (`editing_status`): after the branch for `command_error`, a branch for `workspace.review_refused`: `Said { mark: None, text: look.label(&say("nothing pending")), tail: None, color: palette.text }`. It reads `nothing pending`.
- **Its keys** (`status_line`, `table_hints`): while something is pending (`editing.pending.is_some()`), the hint `:diff review` follows `:w write`. Like every hint it gives way to the counts.
- **Esc** (`letters`): before the Esc that dismisses a note, and under the same conditions (`!left_field`, a fresh press): when the active table tab's `edits.reviewing` is set, push `Action::ReviewEdits { tab, id, show: false }`. The press is consumed, so the same Esc does not go on to close the row panel.
- **`Y`** (`letters`, where `y` copies the cell, line 1220): a capital `Y` typed while the active table tab's review is open copies the SQL instead of the cell. Read the typed text first, then the key, so the one press does one thing:

```rust
    // `Y` with the review open is its SQL: the cell's `y` is the same key.
    let sql = reviewing.filter(|_| typed(ctx, "Y"));
    if pressed(Key::Y) || sql.is_some() {
        let text = match sql {
            Some(id) => crate::ui::review::copy_text(app, tab, id),
            None => app.copy_text(false),
        };
        if let Some(text) = text {
            ctx.copy_text(text);
        }
    }
```

  where `reviewing` is the id of the active table tab while its `edits.reviewing` is set.
- **`SHORTCUTS`** gains, after the row of task 7, `(":diff", "Show the SQL of the pending changes", TERMINAL)`, `("Esc", "Close the SQL of the pending changes", TERMINAL)` and `("Y", "Copy the SQL of the pending changes", TERMINAL)`, and the "Omarchy: vim keys" row gains `:diff` and `Y` after `:e!`.

**Tests**

- `an_unknown_command_says_so` (existing): its first loop runs `["diffs", "wq"]` in place of `["diff", "wq"]`.
- `diff_opens_the_panel_from_the_prompt` (`normal_mode((1, 1))`, a pending cell): `type_key(Key::Colon, ":")`, `type_text("diff")`, Enter: `edits.reviewing`, the prompt is closed, the panel's head and the statement's lines are painted, nothing was sent, and the grid has its keys (`j` moves the selection). The status line's keys then include `:diff review`.
- `diff_with_nothing_pending_says_so_until_the_next_key`: `:diff` with nothing pending paints `nothing pending` in the status line and opens no panel; thirty frames later it is still there; `j` takes it away and moves the selection. Run again, a double-click on a cell (no key) takes it away as well.
- `escape_closes_the_panel_before_the_row_panel`: with the row panel open (Space) and the review open, Esc closes the review and leaves the row panel; a second Esc closes the row panel. An Esc that left insert mode closes neither.
- `the_panel_follows_the_grids_keys`: with the panel open, `u` on the pending cell reverts it and the panel is gone with the set; with two cells pending, `u` on one leaves the other's statement painted and the first's not.
- `capital_y_copies_the_sql_and_y_still_copies_the_cell`: with the panel open, `type_key(Key::Y, "Y")` leaves the review's whole text in `harness.copied` (it begins with the line that says what it is); `type_key(Key::Y, "y")` leaves the cell's value; with the panel closed `Y` copies the cell, as it did.
- `keys.rs`: `the_shortcut_table_names_the_keys_that_review_sql` (task 7's) also holds that Omarchy lists `:diff`, Esc and `Y` with the three descriptions above and that macOS and Windows do not; `the_shortcut_table_names_omarchys_keys_that_edit` also looks for `:diff` and `Y` in the Omarchy row.

Two commits, each green on the four checks: the reducer's part with `an_unknown_command_says_so` put right ("Open Review SQL from Omarchy's prompt with :diff"); then the keys, the status line and the shortcuts ("Close and copy Omarchy's SQL panel from the keyboard").

**Every look reviews here.**

---

### Task 9: The confirmation draws its statements as the drawer does

**Files:**
- Modify: `src/model.rs` (`WritePrompt`), `src/app/editing.rs` (`write_edits`), `src/ui/write_prompts.rs` (`statements`, `confirm_write`, `confirm_sheet`, `confirm_box`)
- Test: `src/app.rs`, `src/ui/write_prompts.rs`, `src/ui/mod.rs`

Step 5's plan adds fields of its own to `WritePrompt` (when it was shown, and who asked). This task changes one field, `statements`, and nothing else of the struct: whichever lands second keeps the other's fields.

- [ ] **Step 1: Change the test that pins the confirmation's statements**

In `src/app.rs`, `a_save_to_production_is_asked_first_with_its_statements` reads the review in place of `prompt.statements`:

```rust
            let lines: Vec<String> = prompt
                .review
                .lines
                .iter()
                .filter_map(crate::review::Line::sql)
                .collect();
            assert_eq!(
                lines,
                [
                    r#"UPDATE "main"."users""#,
                    r#"   SET "email" = 'bob@example.com'"#,
                    r#" WHERE "id" = 2;"#,
                ]
            );
```

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib editing::`
Expected: does not compile (no field `review` on `WritePrompt`).

- [ ] **Step 2: The prompt holds a review**

In `src/model.rs`, `WritePrompt::statements` becomes:

```rust
    /// The statements the save would run, as lines to read.
    pub review: crate::review::Review,
```

Its `Debug` prints neither, as before. In `write_edits`, the loop that collected `shown` and the `match` after it become:

```rust
            // No save is offered with a statement that cannot be shown:
            // its row fails here as it would in the save, with the
            // builder's reason. No cell is to fix: Save was not disabled.
            let review = crate::review::of(dialect, &changes, &[], Values::Shown);
            if let Some((index, error)) = review.refused.clone() {
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.saved = None;
                    object.edits.fail(rows.get(index).copied(), error);
                }
                return;
            }
```

and the `WritePrompt` is made with `review` where it had `statements`.

- [ ] **Step 3: The view**

- `write_prompts::statements` takes `&[Line]` in place of `&[String]`. In its bordered box, the scroll area becomes `egui::ScrollArea::both()` with `max_height` of `review::MAX_ROWS` rows (`STATEMENTS_HEIGHT` goes: the box is measured in lines, as the drawer is), `auto_shrink([false, true])` and `show_rows(ui, review::row_height(..), lines.len(), ..)` calling `review::rows`, the item spacing set to zero before the call. The lines are no longer selectable labels (decision 10). `confirm_sheet` and `confirm_box` pass `&prompt.review.lines`. Until task 10 the terminal's box lists them too, in the same drawing.
- **Copy SQL in the sheet** (`confirm_sheet`): in the foot, after "One transaction", a `ButtonSpec::new(..).quiet()` reading **Copy SQL**, placed with `show_at`. `confirm_sheet` says whether it was pressed, and `confirm_write` (which has the app) then puts `review::copy_text(app, prompt.tab, prompt.id)` on the clipboard: the tab's set is the prompt's while the prompt is up, since nothing changes it there, and it need not be the tab in front. The sheet takes Enter before its buttons are drawn: with the keyboard on Copy SQL, Enter copies, as it cancels with the keyboard on Cancel. Copying closes nothing and sends nothing.

- [ ] **Step 4: The tests of the two views**

- `src/ui/mod.rs`, `confirming`: returns, in place of the statements, every line of `prompt.review` as it is painted (`review::comment(line, locale)` or `line.sql()`), and `a_save_to_production_shows_its_statements_and_is_confirmed` expects ten of them for its two changed rows (two comments and three lines of statement each), each painted, in every look; the assertion that each starts with `UPDATE` goes, and in its place `UPDATE "main"."users"` is painted in `palette.magenta` and `-- row id 2` in `palette.dim`.
- `src/ui/write_prompts.rs`, `the_production_confirmation_lists_every_statement_and_sends_only_when_confirmed`: the same change (ten painted lines).
- New, `the_confirmation_cuts_a_long_value_as_the_drawer_does` (every look): a pending value of 100 characters is painted in the confirmation as 57 characters and `…'`; the save that is sent on confirming carries the whole value (the `Command::Write`'s change set: compare its `rows`, a set prints its counts only).
- New, `copy_sql_in_the_sheet_takes_the_whole_statements` (`desktop_looks()`): with that 100 character value and the sheet up, `click_dialog(&mut harness, "Copy SQL")` leaves in `harness.copied` the text of task 4's last test, the value whole; the sheet is still up and nothing was sent. With the keyboard on Copy SQL, Enter copies and confirms nothing.
- New, `the_sheet_of_a_tab_that_is_not_in_front_still_lists_and_copies` (`desktop_looks()`): a change in `users`, `orders` opened beside it, `Action::CloseConnTab`, the Leave prompt's Save: the sheet is up for `users` while `orders` is the active tab; its five lines are painted (two comments and the statement's three) and Copy SQL gives the statement of `users`.

- [ ] **Step 5: The four checks, and commit**

```bash
git add -A && git commit -m "Draw the confirmation's statements as Review SQL draws them"
```

---

### Task 10: The Omarchy production box points at the panel, where the panel is

**Files:**
- Modify: `src/app/editing.rs` (`write_edits`, `dropped_under_a_prompt`), `src/ui/write_prompts.rs` (`confirm_box`), `src/ui/review.rs` (`show`)
- Test: `src/app.rs`, `src/ui/mod.rs`

**The reducer's part, whole.**

- [ ] **Step 1: Write the failing test**

In `src/app.rs`, in `mod editing`, after the test of task 8:

```rust
        #[test]
        fn the_confirmation_holds_its_review_and_the_terminal_look_opens_the_panel() {
            let lines = |review: &crate::review::Review| -> Vec<String> {
                let lines = review.lines.iter();
                lines.filter_map(crate::review::Line::sql).collect()
            };
            // macOS and Windows: the sheet lists the statements itself, and
            // no drawer opens behind it.
            let mut harness = Harness::new();
            assert!(!harness.app.look.terminal);
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let Some(Dialog::ConfirmWrite(prompt)) = &harness.app.dialog else {
                panic!("expected the confirmation");
            };
            assert_eq!(lines(&prompt.review).len(), 3);
            assert!(!object(&harness, tab, id).edits.reviewing);
            // The terminal look: its box points at the panel, so the panel
            // is open under it, with the review the box was made with.
            let mut harness = Harness::new();
            harness.set_look(crate::theme::Look::omarchy());
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            let Some(Dialog::ConfirmWrite(prompt)) = &harness.app.dialog else {
                panic!("expected the confirmation");
            };
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.reviewing);
            assert_eq!(edits.review.as_ref(), Some(&prompt.review));
            // Under the confirmation the panel is not closed.
            show_review(&mut harness, tab, id, false);
            assert!(object(&harness, tab, id).edits.reviewing);
            // Cancelled, it stays: the statements were just declined, and
            // are still what is pending.
            harness.app.apply(Action::CancelWrite);
            assert!(harness.app.dialog.is_none());
            assert!(object(&harness, tab, id).edits.reviewing);
            assert_eq!(reviewed(&mut harness, tab, id).unwrap().len(), 3);
            // Confirmed and written, it closes with the set.
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::ConfirmWrite);
            harness.answer_written(written("bob@example.com"));
            assert!(!object(&harness, tab, id).edits.reviewing);
        }
```

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib editing::`
Expected: fails at `assert!(edits.reviewing)`: the terminal look's confirmation opened no panel.

- [ ] **Step 2: Open the panel with the box, and keep it under it**

In `write_edits`, after the check of `review.refused` and before the dialog is made:

```rust
            // The terminal's box does not list the statements: its panel
            // does, beside it, open or not until now. The set is the one
            // the review was made of.
            if self.look.terminal
                && let Some(object) = self.object_tab_mut(tab, id)
            {
                object.edits.reviewing = true;
                object.edits.review = Some(review.clone());
            }
```

In `dropped_under_a_prompt`, after `Action::WriteEdits { .. }`:

```rust
            // The terminal's confirmation points at the panel: it stays as
            // the prompt found it or opened it.
            | Action::ReviewEdits { .. }
```

Run the tests again: all pass. The reducer opens the panel of the prompt's tab whether or not that tab is in front; whether the box may point at it is the view's to see.

**The drawing part.**

- **When the box points** (`confirm_box`). All of these hold in the frame it is drawn: `review::placed(ctx)` is some; it is the panel of `prompt.tab` and `prompt.id`; and the room above it, from the top of `ctx.content_rect()` to the panel's top, is at least `POINTING_ROOM`. `POINTING_ROOM` is a constant of `write_prompts.rs`, 300 pt: what the pointing box takes with 24 pt clear above and below it (a test holds the box to it). The panel is drawn before the dialogs in a frame, so `placed` answers for the frame being drawn.
- **Pointing,** the box no longer calls `statements`. In their place, after the dimmed line of rows and columns, one line in `TextRole::OBody`: `sql shown with` in `palette.dim`, a space, `:diff` in `palette.accent`, laid out as one text (so it is painted, and read, as `sql shown with :diff`). The box is the modal `widgets::modal(Id::new("write-prompt"), ..)`: give it an area of its own, `.area(egui::Modal::default_area(Id::new("write-prompt")).anchor(Align2::CENTER_CENTER, vec2(0.0, -lift)))`, where `lift` is half of what the panel takes of the window (`(ctx.content_rect().bottom() - panel.top()) / 2.0`), so the box is centred in what the panel leaves above it. And no backdrop: `.backdrop_color(Color32::TRANSPARENT)`. Its 2 pt danger border marks it, the statements under it are read at full strength, and the modal still takes every click.
- **Not pointing,** the box is the one task 9 left: it lists the review's lines itself, centred, over the backdrop every dialog has. That is the box of a tab that is not in front (the Leave prompt's Save for a tab, a connection or the window; another connection's tab in front), and of a window too low.
- **Page Up and Page Down** (`review::show`): while `app.dialog` is the `ConfirmWrite` of this very tab, read the two keys before the lines are drawn (`ctx.input_mut(|input| input.consume_key(Modifiers::NONE, ..))`) and move the lines by `MAX_ROWS` rows inside the scroll area (`ui.scroll_with_delta`, the content moving up for Page Down). Nowhere else: with no dialog up the two keys are the grid's.
- **The box's foot,** while it points, gains a third hint, after `esc cancel`: `pgup/pgdn` with the label `scroll sql`, a `terminal_dialog::Key` with `button: None`. It goes last because `confirm_box` matches the first two by their place (0 confirms, 1 cancels).

**Tests** (`Look::omarchy()`; `confirming` gives the box for the tab in front)

- `the_prod_box_points_at_the_panel_and_stands_clear_of_it`: `sql shown with :diff` is painted; every line of the review is painted exactly once (in the panel, not in the box); the box (the rect outlined with a 2 pt stroke of `Tone::Danger.color`, in `harness.outlines`) does not intersect the rect of `review::placed(&harness.ctx)`, which names the prompt's tab; the box is no taller than `POINTING_ROOM - 48.0`; "type write to confirm" is still painted and the field has the keyboard; the foot paints `pgup/pgdn scroll sql`.
- `the_prod_box_lists_its_statements_when_its_tab_is_not_in_front` (the review's steps): production, a change in `users`, `orders` opened beside it, `Action::CloseConnTab(tab)`, then the Leave prompt's write (`w`, or `Action::LeaveSave`): `Dialog::ConfirmWrite` is up with `prompt.id` the tab of `users` while `active_tab` is `orders`. `sql shown with :diff` is not painted; all five lines of the review are painted, inside the box's rect; no panel is drawn; the foot has no `pgup/pgdn`; typing `write` and Enter sends the save.
- `the_prod_box_lists_its_statements_when_another_connection_is_in_front`: with the saved connections in front (`Action::ShowConnections`, so the workspace is not drawn), the window is asked to close (`harness.close_requested`) and the Leave prompt is answered with `Action::LeaveSave`: the box of `users` is up, and the same assertions hold.
- `the_prod_box_lists_its_statements_in_a_low_window`: `Harness::with_size(egui::vec2(1280.0, 520.0))`, the tab in front, all five rows changed. The test first asserts that the panel leaves less than `POINTING_ROOM` above it (so a later change of sizes that undoes the case fails here and not silently), then that `sql shown with :diff` is not painted and the first line `-- row id 1` is painted inside the box's rect.
- `the_prod_box_draws_no_backdrop_where_it_points_and_the_leave_prompt_still_does`: with the pointing box up, no translucent fill covers the whole window (`harness.fills` has no rect of the window's size whose colour is not opaque); with the Leave prompt up (close the tab with a pending cell) one does; and with the box that lists its statements one does.
- `page_down_scrolls_the_statements_under_the_box`: with all five rows changed (25 lines, 12 in view), `-- row id 1` is painted and ` WHERE "id" = 5;` is not. After one `harness.press(Key::PageDown, Modifiers::NONE)` and `harness.finish_animations()` (scrolling animates), `-- row id 1` is not painted. After a second press and `finish_animations`, ` WHERE "id" = 5;` is painted. `prompt.typed` is still empty and nothing was sent. Two presses of Page Up, each followed by `finish_animations`, bring `-- row id 1` back. With no dialog up, Page Down moves the grid's selection and not the panel.
- `the_panel_the_box_opened_stays_after_cancel_and_closes_with_esc`: Esc cancels the box (the panel is still painted); a second Esc closes the panel; a third does not close a row panel that was never open.
- `a_confirmed_write_closes_the_panel_with_the_set`: type `write`, Enter, answer the save as written: no panel is drawn.
- The existing assertions of `a_save_to_production_shows_its_statements_and_is_confirmed` for the terminal look stay green: its ten lines are painted (now by the panel), `wri` confirms nothing, `write` sends.
- On macOS and Windows, `the_desktop_confirmation_opens_no_drawer`: with the sheet up `edits.reviewing` is false and the bar's button still reads "Review SQL".

Two commits, each green on the four checks: the reducer's part ("Open Omarchy's SQL panel with its production box"); then the box ("Point Omarchy's production box at the SQL panel where it is on screen").

**The step is whole here.**

---

### Task 11: Scenes, documents and every check

**Files:**
- Modify: `src/shots.rs`, `README.md`, `docs/superpowers/specs/2026-10-03-value-editing-core-design.md`, `docs/superpowers/specs/2026-09-27-tabletist-design.md`

- **A scene** in `src/shots.rs`, on the Bookshop data (never other data), for review by eye: `("edit-review", edit_review)` joins `EDITING`, whose size in its type grows by one (step 5's plan adds a scene to the same array: the second to land counts both). It is made as `edit_pending` is (`editable`, `retire_image`, `retype(.., (3, BOOK_ID), "107233x")` for a row that is blocked, `step_aside`) and then `Action::ReviewEdits { tab, id, show: true }`. The existing `edit-production` scene now shows, in the Omarchy look, the box above the panel. Shots are not run in the suite and not committed.
- **The README** (the bullet on editing a table's values, around lines 43 to 46): before saving, the statements can be read (Review SQL in the bar, `:diff` on Omarchy, `Mod+Shift+D` everywhere) and copied.
- **The value-editing spec:** its status ("Steps 1 to 4 are built"); "Review SQL" as built, with this plan's decisions where they add to the spec or depart from it: the two comment lines and how a loaded value reads; that no value ends a comment; the rows that have no statement and why; where the cut is made; that the lines are not selectable and where Copy SQL is; that the review is the tab's and ends with the set; `Mod+Shift+D` and what it does with an open editor; what `:diff` says with nothing pending; that the Omarchy panel is a bottom panel and does not take the keyboard, and what it leaves of the design's foot. **What a copied text is,** in so many words: the app's statements with the line that says so, not a script that checks a row or wraps a transaction, and the two forms that another client can read otherwise (MySQL under `NO_BACKSLASH_ESCAPES`; a SQLite REAL with a very large exponent). "Saving to production": both confirmations as built (the sheet draws the review's lines and has Copy SQL; the box points at the panel, stands above it without a backdrop, and Page Up and Page Down scroll the panel under it, wherever the panel of its own tab is on screen and the window has the room; everywhere else it lists the statements itself). "Keys": the Review SQL row (`Mod+Shift+D` and the bar's button; `:diff`, `Mod+Shift+D`), and the prompt's sentence (`w`, `diff` and `e!`). "What step 3 leaves for steps 4 and 5": the part for step 4 goes, and what is still open stays (see "What this plan leaves for later"). "Testing": the `edit-review` scene. In `docs/superpowers/plans/2026-10-03-connection-write.md` the bullet "What Review SQL shows is not always what another client would run" ends with what was decided here.
- **The main spec:** its keyboard table gains Review SQL (`Mod+Shift+D`, `:diff`), and `src/review.rs` and `src/ui/review.rs` join the layout of section 3.2.
- **Every check,** with both server URLs exported:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
~/.cargo/bin/cargo test --locked --workspace --all-targets
```

- **By hand, by the user** (no window opens in an agent's session), on the demo (`--demo` opens a writable SQLite file): edit three cells in two rows and one to a value its column refuses, press Review SQL, read the lines, revert a cell and watch its statement go, Copy SQL and paste it somewhere; `Mod+Shift+D` from the grid, from the tree and from an open editor; on a production-labelled connection, Save, read the sheet, Copy SQL there. In the Omarchy look: `:diff`, `u` on a pending cell, `Y`, Esc; `:diff` with nothing pending; `:w` on production with a set longer than the panel, Page Down, Esc, Esc; close the connection's tab from another table's tab with a change pending on production, and answer `w`: the box must list its statements. Whether the box without a backdrop reads as a question that must be answered is for the eye: say so in the report either way.

Commit: "Describe Review SQL as built".

---

## As built

The eleven tasks as above, with these differences from the draft. The value-editing spec says the whole of it in the present tense ("Review SQL", "What a copied text is", "Saving to production").

- **`ReviewEdits` is dropped under a prompt from task 5 on,** not from task 10 (decision 23): shown, the review closes an open editor as a left edit, and the set would then be another than the one a Leave prompt or a confirmation asks about.
- **When the Omarchy box points** (decision 19, task 10). The draft asked three things: a panel is on screen, it is the prompt's tab's, and 300 points are left above it. Built, it asks six. The panel was drawn in the very frame the box is (`review::placed_now`): one that stood there a frame ago is not on screen. It is the prompt's own tab's. It draws the review the prompt holds (`Edits::review` equals `WritePrompt::review`). It shows its lines: all of them, or three at once at the least (`Placed::lines`). The widest line of the review fits the width the lines stand in (`Placed::width`, `review::widest`): under the box neither the pointer nor a key moves the panel's lines sideways, so in a narrow window, or beside the row panel, the right part of a statement would be confirmed unread. And the window leaves the room above it. The widest line is measured once for a review and kept, by a hash of its lines, the look, the locale and the scale: a large set is not laid out again every frame.
- **The statements' height is capped by the window** (decisions 13 and 20, task 9): to twelve lines, and to what the window leaves besides the rest of the sheet or the box, never under three. In a low window the question and its answers stay on screen.
- **A page is the rows in view,** not twelve rows (task 10): a panel that shows three lines pages by three, so no line is passed over unseen.
- **Page Up and Page Down are the confirmation's** (task 10). The draft had the panel read them whenever its tab's confirmation was up, which moved the drawer dimmed behind the desktop sheet and left a list in the sheet or the box without a key. The confirmation takes them: it turns the panel's pages where the box points at it (`review::turn`, applied when the panel is next drawn), and moves its own list everywhere else.
- **What the builder refuses** (task 1). PostgreSQL text that holds a NUL and a SQLite key whose text holds U+FFFD were refused by the drivers beside `Dialect::update_row`, so the review, Copy SQL and the production confirmation showed a statement for a row the save then failed. Both are refused in `update_row`, with the same words, and the drivers' own checks are gone: each driver builds every statement first and fails the row whose statement is refused.
- **A MySQL NUL is written as `\0`** in the shown literal, after its backslashes are doubled (decision 11 left a raw NUL in the copied text, which a clipboard may end the text at). The bound save is untouched.
- **The cut** (decision 9). SQLite's text around a NUL is closed with `…')` in a string and with `…)` between two, never inside ` || char(0) || `. A MySQL `\0` is never split.
- **A copied check says the whole of what was loaded** (decision 11 kept the copy's comments cut as shown). The copy says each statement runs only while its row is still as the comment above it says, so the comment holds the whole value, on one line, with hidden characters written out. Shown, it is cut as before.
- **What a copied text is** (decision 11) names a third place another client reads it otherwise: CR before LF in a SQLite value through the `sqlite3` shell, which stores LF alone.
- **The head says when a cell is still being edited** (decision 16 covers only the moment the review is shown): an editor opened and typed into under the open review is not in its lines, and Save would send it. The head then reads "Without the cell being edited", in the warning colour. The lines are not made again per keystroke.
- **A structure that arrives drops the tab's review,** which is made again in that round of actions (decision 3 named the set alone).
- **A numeric's scale** is held to its range without negating it (`crates/tabletist-db/src/class.rs`): a type name with the least `i32` for a scale states nothing, where it panicked.
- **The scene** `edit-review` is the eighth of `EDITING`.

Left as found:

- `Mod+Shift+D` on an editor that was only opened, with nothing else pending, closes the editor and shows nothing.
- A name in a shown line is not capped: only values are cut.
- A loaded text that holds `' and ` reads as two conditions in the check's comment.
- Under the Omarchy box nothing copies the statements: `Y` and the panel's Copy SQL are not reached under a dialog, and the box has no Copy of its own.

## What this plan leaves for later

- **Step 5, the conflict question,** is built on top of this plan (`docs/superpowers/plans/2026-10-04-conflict-dialog.md`) and touches the same files. What of this plan lands in them:
  - `src/edit.rs`: task 3 adds two fields to `Edits` and the methods `put` and `revert`.
  - `src/model.rs`: task 3 adds `Action::ReviewEdits`; task 8 adds `Workspace::review_refused`; task 9 replaces `WritePrompt::statements` by `review`. **Both plans add fields to `WritePrompt`** (step 5: `after_answer`, when another dialog's answer brought the prompt up); neither removes the other's.
  - `src/app.rs`: task 3 changes the `RevertCell` arm (the set through `Edits::revert`), adds the `ReviewEdits` arm and a call at the end of `apply_actions`; task 8 adds a line to the `OpenCommand` and `CloseCommand` arms. The guard at the top of `apply` is not changed here, but what it asks (`dropped_under_a_prompt`) gains a line in task 10.
  - `src/app/editing.rs`: task 3 changes `close_editor` and `set_null` (the set through `Edits::put` and `Edits::revert`) and adds three functions before `written`; task 8 replaces `run_command` and adds a line to `edit_cell`; tasks 9 and 10 change `write_edits` (the confirmation's review; the panel opened in the terminal look) and task 10 adds one line to `dropped_under_a_prompt`. None touches `written`, which is where a conflict's answer arrives, nor `confirm_write`.
  - `src/ui/pending_bar.rs`: task 5 adds one button to `show`, in the branch that draws Save and Discard all.
  - `src/ui/write_prompts.rs`: task 9 changes `statements`, `confirm_write`, `confirm_sheet` and `confirm_box` (the review's lines in place of strings; Copy SQL in the sheet's foot); task 10 changes `confirm_box` (the line that points at the panel, the area, the backdrop, the foot's third hint). Neither touches `leave`, `leave_sheet` or `leave_box`.
  - `src/ui/workspace.rs`: task 5 adds one call in `show`; task 8 adds a branch to `editing_status` and a hint to `status_line`.
  - `src/ui/keys.rs`: task 7 adds a block to `editing_keys` and a row to `SHORTCUTS`; task 8 changes `letters` (an Esc before the note's, and the `y` site), the `refused` flag in `handle`, and `SHORTCUTS`.
  - `src/shots.rs`: task 11 adds a scene to `EDITING`, an array whose size is in its type. Step 5 adds two.
  - The two specs and the README (task 11), and the test modules of `src/app.rs` (the nested `mod editing`) and `src/ui/mod.rs`, where both plans put their tests and helpers side by side.
  - **What step 5 owes this plan:** any write to `Edits::cells`, or to a row of the page under pending cells, must leave `edits.review` as `None`: the check's comment says what was loaded, and a row without cells has no statement. One cell changed through `Edits::put` or `Edits::revert` does it. Cells dropped another way (a `retain`, a range) and a row replaced in the page (Keep mine and Overwrite put the server's row there) need `object.edits.review = None` beside them: a search for `insert` and `remove` will not find those. Step 5 does so in its one such place, `answer_conflict`, for every answer.
- **A cursor in the Omarchy panel:** the design's `]c next change` and `u revert under cursor`, and the panel as a pane that F6 and Ctrl+H/L step to. A later slice, by the owner's decision.
- **A drawer the user can resize,** and a height it remembers.
- **A row whose last save failed** is not marked in the review: the bar and the grid say it.
- **`Y`, `:diff` and the other letters on a keyboard layout without them:** matched by the character typed, as step 3 left Omarchy's letters. The chord and the panel's hidden buttons reach Review SQL there; a way to type the prompt's commands does not exist yet.
- **The checks a save makes that the grid does not** (a SQLite column whose list of allowed values holds non-numbers): the review now says such a row "cannot be sent" before Save is pressed, but Save is still offered and fails the row, as step 3 left it.
- **Undo and redo, pasting, the editors by type, the row form, adding and deleting rows:** slices 2 to 5 of the spec's "Editing as a whole". Each new kind of statement (`INSERT`, `DELETE`) needs its own lines in `src/review.rs` and its parts from the builder.
