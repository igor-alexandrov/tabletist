# Conflict Dialog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Step 5 of `docs/superpowers/specs/2026-10-03-value-editing-core-design.md`: when a save finds rows changed on the server, the user is asked about each of them (Keep mine, Use server values, Overwrite; Discard my changes for a row that is gone). Every answer rebases its row in the page and in the pending set, and the save runs again only when the answers call for it. In all three looks.

**Architecture:** The question is one more `Dialog` (`Dialog::Conflict`). It holds the save's conflicts as rows of the page, the place of the one being asked about and that row's values ready to draw. The reducer applies every answer when it is given: the server's row replaces the loaded one, and the row's pending cells stay, leave or are dropped. What a conflict shows and what an answer settles are pure functions in `src/edit.rs`. A row that is gone is marked on the tab (`Edits::gone`) and locked (`Lock::Gone`). The views draw the question and push one action, `AnswerConflict`. A question that comes up unasked takes no answer in its first half second, and so does the production confirmation where another dialog's answer opened it.

**Tech Stack:** Rust, egui (the crmne fork of 0.36), `tabletist-db` (`WriteOutcome::Conflicts`, `Conflict`), the headless UI harness in `src/testing.rs`.

---

## Before you start

- **All nine tasks are built,** on `claude/connection-write`, with what their reviews changed: see "As built", before "What this plan leaves for later". The tasks below are the draft they were built from.
- Cargo is `~/.cargo/bin/cargo`. Never a cargo target dir under `/tmp`. Export both test server URLs when you run the whole suite:

      export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
      export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist

- House rules (`AGENTS.md`): no em dashes anywhere; comments say why, in the surrounding code's voice; no `unsafe`; do not weaken a lint, delete a test or add an `allow`; a view never mutates state except the text a field is editing and a one-shot focus flag; a view never names a font (use `TextRole`s) and never paints a focus ring (use `focus::hint`); every user-visible string in a view goes through `gettext(locale, ..)`; add a focused regression test for every behaviour change.
- Four source scans fail the build and bite here (`src/env.rs`, tests at the end, from line 697): no file outside `env.rs` may match on `Environment::<name>`; `src/model.rs` may not contain `Color32`; no view may write a `#rrggbb` colour (mix palette colours instead); and only the files in the `surfaces` list may contain `env_colors(`, in a comment too (a new view takes its reds and ambers from `states::Tone::{Danger, Warning}`).
- **The design is the source of truth for how it looks.** It is not in the repository and is never copied into it. What this plan needs from it is written out under "What it looks like" below. No test compares a screen with the design.
- **This plan runs on a new branch off `claude/connection-write`** (or off `main` once pull request #76 is merged). It needs step 3 whole: all sixteen tasks of `docs/superpowers/plans/2026-10-04-grid-editing.md` and the review fixes that followed them, up to `85d1c61` ("Say in the SQLite session's settings that foreign keys are enforced") and the one commit after it, `b300475` ("Hold an editor's text in bytes, and never cut a paste to a value that passes"), which is what the plan was checked against. The line areas below are those of `b300475`; it differs from `85d1c61` only in `src/ui/cell_editor.rs` and `src/ui/mod.rs`.
- **Step 4, Review SQL, is built on that branch** (all eleven tasks of `docs/superpowers/plans/2026-10-04-review-sql.md` and the fixes its review asked for), so this plan comes second: "What this plan leaves for later" says which places it rebases. Three facts under "How the code stands" are no longer as `b300475` had them, and the line areas there have moved in every file both steps touch:
  - `WritePrompt` (`src/model.rs`) holds `review`, the lines its confirmation draws (a `crate::review::Review`), in place of `statements`. Its hand-written `Debug` and its other fields are as described.
  - `Edits` (`src/edit.rs`) has `reviewing` and `review` (the tab's Review SQL; `None` is stale, and the reducer makes it again at the end of the round of actions), and the methods `put` and `revert`, which change one cell and drop the review. Any write to `Edits::cells`, or to a row of the page under pending cells, goes through `put` or `revert`, or leaves `edits.review = None` beside it: the check's comment says what was loaded, and a row without cells has no statement.
  - `EDITING` in `src/shots.rs` holds eight scenes: `edit-review` joined the seven. Task 9's two make ten.
- Commit after every task. Subjects are plain sentences, each ending with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

  If the commit's signing agent is locked ("agent refused operation"), do not bypass it: `git add -A`, `git write-tree`, and report the tree id with the subject.
- **What was checked when this plan was written.** The code and the tests of tasks 1 to 5, the changed tests of step 3 and the stand-in view were compiled and run in a copy of the tree at `b300475`: 1520 library tests pass, and `clippy -D warnings`, `rustfmt --check` and `rustdoc -D warnings` are clean. The state after each of tasks 1 to 5 was built from this plan's own text, on a fresh copy: clean after every one, and with task 4's tests added before its code exactly the eight tests its step 2 names fail. The two tests the review asked to be able to fail were tried against the rule they pin: without the check for nothing pending, and without a kept row that is gone counting as kept, each fails. What task 8 does to the model (the terminal look asks too, two tests of step 3 change, the view's tests run in every look) was built with the stand-in still drawing, and passes. Paste the code as it is given; where it and the tree disagree by then, the tree wins and you say so in your report. The drawing of tasks 6 to 8 was not built: those tasks give behaviour, hooks and tests, as step 3's drawing tasks did.
- **How the code stands.** Read each function before you edit it.
  - `crates/tabletist-db`: `Connection::write(&self, changes, stop: &StopFlag)` (`src/lib.rs`, 282). In `src/write.rs`: `WriteOutcome::Conflicts(Vec<Conflict>)` (68) and `Conflict { row, server }` (76). `row` is the row's place in the change set, not in the page. `server` is the whole row as the database holds it now, in the page's column order, or `None` when the row is gone. Every conflicting row of the set comes back, in the set's order, and nothing was written. `ChangeSet` has a hand-written `Debug` that prints counts only (18). Each driver's save refuses two changes that read the same row (`same_row_twice`, 250): that is an error, never a conflict that names one row twice.
  - `src/edit.rs`: `Lock` (23), `Table` (58) with `Table::of` (72), `class` (122) and `lock` (130); `Problem` (225), which has `TooLarge` (254); `start_text` (467) and `is_change` (478), which decide what counts as a change; `Pending` (496, no `Debug`: what a user typed stays out of logs), `State` (502), `Edits` (530) with `holds` (566), `counts` (570) and a hand-written `Debug` (625); `Saving` (640), whose `rows` is the page's row of each row of the set and whose `then` is what was held for the save; `SAVED_FOR` (662); `Note` (666), with `Conflict { row, gone, others }`; `change_set` (691); `same_value` (738, private: floats by their bits). The tests' helpers are `structure`, `page`, `text`, `rows`, `table` and `at` (882 to 944); the page there has `id`, `email`, `meta` and two rows (`1, ada@example.com, NULL` and `2, bob@example.com, {}`).
  - `src/app/editing.rs`: a second `impl App`. `dropped_under_a_prompt` (26); `table` (50), which reads a tab as editing sees it; `dropped_by` (63), `hold` (144), `perform` (182), `hold_close` (216); `save_blocked` (238); `write_edits` (268), which closes the editor, goes on with what was held when nothing is pending, stops where `save_blocked` says so, and on production opens the confirmation only while no dialog is up, or fails the row whose statement cannot be built; `confirm_write` (361), which leaves `Note::NotSent` when the session went; `send_write` (409); `run_command` (608); `written` (635), whose `Conflicts` arm (708) keeps only a `Note::Conflict` of the first row and drops what was held (`then` is taken at 664 and performed only after `Written`).
  - `src/app.rs`: `App::apply` (332) starts with two checks: under `Dialog::Leave` or `Dialog::ConfirmWrite` an action that `dropped_under_a_prompt` names is dropped (338), and an action that would drop a page with pending changes is held (344). The editing arms are at 798 to 927 (`DiscardEdits` 860, `LeaveDiscard` 882, `LeaveSave` 902, `ConfirmWrite` 922); `CloseDialog` (1437) closes whatever dialog is up. `Event::Rows` (2191) sets `object.edits` to its default whatever the answer is; a failed fetch leaves the old page in `rows.value`. `Event::Written` (2432) calls `written`. `format_rows` (286) makes the row panel's text again whenever `object.fields` is `None`. `frame_ui` (3610) does not run `keys::handle` while a dialog is up (3625), but the views behind it still draw, and a click can be a frame behind.
  - `src/model.rs`: `Action` (47; the editing actions at 236 to 308), `Dialog` (1154), `Held` (1173), `LeavePrompt` (1181), `WritePrompt` (1195, with a hand-written `Debug` at 2763 that names four fields and ends `finish_non_exhaustive`), `SaveBlock` (1689), `ObjectTab` (1705), `RowFields` (1731), `Workspace::forget_session_requests` (2712).
  - `src/ui/write_prompts.rs`: the two prompts of step 3 in both forms. `Skin` (44) with `say` and `frame`; `fitted` (92); `button_row` (99) and `keyboard_on` (126); `leave_sheet` (165), where Enter is consumed before anything is drawn and follows the button that has the keyboard; `leave_box` (252), whose letters are read as typed text and ignored while a key is held or a text field has the keyboard (264 to 295); `confirm_write` (394), which collects the frame's answers and passes them on in its last line; `confirm_sheet` (486) and `confirm_box` (576). All of them are private to the file. Its tests begin at 709, with `change`, `writes` and `pending`.
  - `src/ui/terminal_dialog.rs`: `head` (17), `foot` (41), `Key` (63), `keyboard_on` (91), `keys` (101): a foot's key hints, right-aligned, each its button too, the key in the text colour (the accent for a `lead` key) and its words dim.
  - `src/ui/pending_bar.rs`: `row_name` (52, private) names a row by its key through `row_panel::key_parts` (`src/ui/row_panel.rs`, 175): `id 2`, several columns joined by `, `, each value a cell's short text of up to 256 characters, and the row's number where the key is not known. The key is `Structure::row_key()`, as in the row panel's title since `85d1c61`. `note_said` (84) words `Note::Conflict` (93); `said` (314) paints a line cut with "…", whole under the pointer and to a screen reader. `src/ui/workspace.rs`: `editing_status` (1604) says a note in Omarchy's status line (`Note::Conflict` at 1686, after `≠`), and `drawable` (1591, private) picks the first mark the look's font has.
  - `src/ui/grid.rs`: `Mark` (89), `Cell` (104), `data_role` (170), `marks` (180), `ellipsize` (311), `null_label` (1321), and in `show` (456) the cell loop, where a cell's tone comes from its mark (752) and its text's colours from `written` (829). `src/ui/data_view.rs`: `Changes` (1411), `Changes::of` (1470, called at 1257) and `Changes::mark` (1505). `src/ui/format.rs`: `cell_text` (21), which is not what the grid draws a text with: breaks and tabs become spaces in it; `cell_line` (93), the grid's one line of a text, with a mark for a line break; `blank_text` (100), what a cell shows for an empty or all-white text (`''`, a mark for each character); `Marks` (61); `CELL_MAX_CHARS` (10, 256); `display_safe` (135).
  - `src/ui/cell_editor.rs`: `lock_text` (690) is an exhaustive match on `Lock`; its test `every_problem_and_every_lock_has_words` (760) lists the locks by hand.
  - `src/ui/widgets.rs`: `ButtonSpec::quiet` (1278), whose colours `show_at` picks at 1550; `primary_fill` (687); `modal` (756); `announce` (842) and `paint_label` (851).
  - `src/testing.rs`: `Harness::editable()` (585) gives a writable `users` table of five rows (`id`, `email`, `meta`; row 1 is `2, user2@example.com, NULL`) on the active tab, and works again on a picker tab opened with `Action::ShowConnections`; `answer_written` (649) answers the newest `Write`; `finish_animations` (197); `click` (207); `press` (231); `text_rects` and `fills` (28, 30) say where the last frame painted each text and each fill. The harness draws in the standard look unless `set_look` says otherwise.
  - `src/app.rs`'s tests: the nested `mod editing` (9776) with `at` (9782), `type_into` (9788), `write_since` (10130), `row` (10140), `leave_prompt` (10410), `writes` (10910), `production` (11080), `confirming` (11097), `change` (11107) and `written` (11117); above it `users` (5611) and `object` (5625).
  - `src/ui/mod.rs`: the dialogs are drawn one after another at 59 to 65. Its tests: `type_text` (3569), `type_key` (5301), `focused_name` (10385), `editable_in` (11169), `make_pending` (11207), `desktop_looks` (11500), `normal_mode` (12316), `pressable` and `click_dialog` (15501 and 15518, both `pub(super)`), `focus_dialog` (15791). Three of its tests answer a save with `Conflicts` and expect the line: `a_written_save_and_a_conflict_are_said_in_the_status_line` (13666) and `the_status_line_keeps_a_keys_value_and_the_databases_words_as_they_are` (13779) in the Omarchy look, `a_conflict_is_a_line_in_the_bar_and_the_set_stays` (15305) in the other two.
  - Tests of time back-date a stored `Instant` (see `age_fetch` in `src/ui/mod.rs`, 3202); there is no injected clock.

## What it looks like

From the design's "Editing values" artboards (the macOS flow's card "Conflict", and the Omarchy panel "6 · results"). Colours are given as what they are, never as numbers: every one is a palette colour or a `states::Tone` mix.

**macOS and Windows**

- **The sheet:** the title "Row id 2 changed on the server" in the dialog title's weight. Under it, in the secondary colour: "Someone saved it after you loaded it. Nothing was written."
- **The table:** a box with a hairline border and rounded corners. Four columns: a narrow first one (80 pt) for the column's name, then three of equal width. A header row on the surface tint, its words small and in the secondary colour: nothing, "loaded", "now on server", "yours". Then one row for each column the user changed, a hairline above each, in the code face: the column's name; the loaded value, plain; the server's value on a red tint with red text where it is another value than the loaded one, and plain where it is the same; the user's value on the amber tint a pending cell has in the grid. NULL is the grid's grey chip. A value too long for its place is cut with "…".
- **The foot:** at the left "Keep mine, reload row", with no border and in the accent colour, as a link reads. At the right "Use server values" (bordered) and "Overwrite" (primary: the ink fill). 8 pt between buttons.

**Omarchy**

- **The first line:** `≠ conflict` in the warning colour, then `row id 2 changed on the server` in the text colour.
- **The values:** a grid without rules. A first column of 90 pt, then three of equal width, 10 pt apart. The header in the dim colour: nothing, `loaded`, `server`, `yours`. A row for each changed column: its name and the loaded value in the text colour, the server's value in the danger colour, the user's in the warning colour.
- **The keys:** `[o] overwrite`, `[s] use server`, `[k] keep mine, reload`. The design writes them in a line under the grid, each bracketed letter in the accent colour and its words in the text colour. The app's terminal dialogs have their keys in a foot (decision 20), and that is where these stand.

The design draws one conflict of a row that still exists. What it does not draw is decided below: the row that is gone, the line that says where the row is and which of several it is, a value that became NULL, and how a row that is gone looks in the grid.

## What this plan decides beyond the spec

Decisions 6, 16, 17, 18 and 21 are the product owner's, given on review of this plan's first draft. Decisions 9 and 11 are the spec's own words, kept as they are.

1. **The question is a `Dialog`, and holds the whole queue.** `Dialog::Conflict(Box<ConflictPrompt>)` has the tab, the save's conflicts as rows of the page (`rows`), the place of the one being asked about (`at`), that row's lines ready to draw (`lines`, decision 24), when that one came up (`shown`, for decision 16) and two flags for the rule of decision 9. The app has one dialog at a time, and everything that makes a prompt a prompt already asks that one field: no shortcut runs behind it, no other dialog takes its place, a guarded action is refused. State on the tab would need each of those checks a second time. The answers are not stored: each is applied when it is given, and the two flags are all the rule needs of them.
2. **The answer names rows by their place in the change set, and `Saving::rows` says which rows of the page those are.** `edit::conflicting` maps them once, when the question opens. It answers nothing when the conflicts cannot be asked about: one names a row the save did not send or the page does not hold, one row comes twice (the drivers refuse that save, so this is a guard and no more), or a server row is not as wide as the page (the table is no longer the one the page was read from, and there is nothing to put the row into).
3. **Where the question cannot be asked, the conflict is the line of step 3.** That is the case of decision 2, and the case of another dialog being up when the answer arrives (the question before the window closes under a running save, the shortcuts, Settings): a dialog the user is in is never replaced. `Note::Conflict` and its words in the bar and in Omarchy's status line stay for this. The set stays as it was, and the next save asks. A row the save did not send stays `Note::Lost`, as built.
4. **Once the question is asked, the tab says nothing more, with four exceptions.** While it is up the tab has no note, and after the last answer the bar shows what is still pending, or "Saving…" when the save runs again, or goes when nothing is left. A line is left only where something is still to be known: by Esc on a row that is gone (decision 6); by the question being closed unanswered (decision 14); by a save that could not be sent because the session went, "Not connected. Nothing was sent." (decision 10); and by a production save whose statement cannot be built, which fails its row with the builder's reason as `write_edits` does for any save.
5. **Every answer is applied when it is given.** The server's row replaces the loaded row in the page, in place, also when the sort or the filters would now move or hide it (as after a save that wrote). Then the row's pending cells:
   - **Keep mine** and **Overwrite:** a cell whose new value is no change against the server's value leaves the set. That is decided by `edit::is_change`, the same rule that takes a cell out of the set when the loaded text is typed back: NULL over a NULL is no change, text equal to where an editor would start is none (`true` over the 1 a SQLite boolean holds), and text over a NULL always is one.
   - **Use server values:** every cell of the row is dropped.
   - A cell that stays keeps its state. A failed cell's message is about the new value, which did not change, and the next save sends it again. A cell to fix cannot be in a set that was saved (`SaveBlock::ToFix`), and would be treated the same.
   - `object.fields` is dropped, so the row panel's text is made again from the row as it is now. The grid's marks follow from the set.
6. **A row that is gone: Discard my changes drops them, and Esc keeps them and says so.** Discard drops the row's cells and marks the row gone. Esc, which is Keep mine for the row shown, leaves the row's cells pending, marks and locks nothing, and leaves the line "Row id 2 no longer exists on the server. Nothing was written." in the bar (`Note::Conflict { row, gone: true, others: 0 }`, whose words `pending_bar.rs` already has; Omarchy's status line says it after `≠ conflict`): the row is not left looking like any other row with pending changes. It counts as kept (decision 9), so no save follows by itself. The next save says once more that the row is gone. `Answer::offered` says which answers a row has; the reducer drops any other.
7. **"Gone" is a set of page rows on `Edits`, and a lock.** `Edits::gone` holds the rows a save found gone and the user discarded. `Table::lock` answers `Lock::Gone` for every cell of such a row, after the reasons that hold for the whole table and for a while, and before anything the row's values say. Its words: "This row no longer exists on the server". The set is about the page, so it goes when a page arrives (`Event::Rows`), and it stays through what only drops pending changes: Discard all, the Leave prompt's Discard and a save that wrote (`Edits::discard`). A fetch that fails leaves the old page on screen, and its gone rows gone.
8. **A gone row does not hold the tab's page.** `Edits::holds` is unchanged: a refresh is how the row leaves the screen, and nothing of the user's would be lost by it, so nothing is asked.
9. **The save runs again when some row was answered Overwrite and none Keep mine.** Use server values and Discard count neither way. Keep mine counts whatever its rebase left, and for a row that is gone too: a user who chose it never sees an automatic save. An Overwrite counts even when every one of its cells left the set; then the rest of the set is what is saved, and with nothing pending nothing runs and nothing is said.
10. **The second save is an ordinary save.** `answer_conflict` takes the dialog away and saves through `write_edits`. So with nothing pending nothing is sent; `save_blocked` stops it where a save cannot be made, and the bar's Save and Omarchy's line say why; on production the confirmation opens again, with the statements of the set as it is now (the server's values are the loaded ones in it); and the guard is as for any save. One case gets a line of its own, as after a confirmation: when the session went while the question was up and something is pending, the tab says "Not connected. Nothing was sent." (`Note::NotSent`).
11. **What was held for the save is dropped by its conflict, for good.** `written` already drops it. The second save has nothing held: after an Overwrite's save wrote, the tab the user wanted to close is still open, and so is the window. A question came between the wish and the save, and the user closes again.
12. **Nothing changes the set or the page while the question is up.** The reducer drops what `dropped_under_a_prompt` names under `Dialog::Conflict` too (edits, saves, discards, moves of the selection, what would take the dialog's place). A guarded action is refused with the notice, as under any dialog: every row still to be asked about has pending cells, so the tab holds edits until the last answer. A request to close the window is cancelled the same way.
13. **What the backend says still arrives.** A tab that holds edits is never fetched and never described again, so no page and no structure arrives for it; if a page does arrive, it ends the question (its rows are places in the page that went, and its cells went with the set). A lost connection changes the status and nothing else: the answers need no session, since the server's rows are in hand. The second save is where a lost session shows (decision 10). A reconnect keeps the page, as it does for every tab that holds edits.
14. **`CloseDialog` closes the question, answers nothing, and leaves the line.** Rows already answered stay settled. The row shown and the rows after it stay as they were loaded, with their cells pending, and no save runs: the Overwrite of an earlier row waits for a last answer that never came. The tab then says what it says where the question is not asked at all, for the row that was shown and with the count of the rows after it (`Note::Conflict`), so the pending cells are not left over rows known to be stale without a word. The one sender of `CloseDialog` under this dialog is its own view, when the tab it asks about is no longer there to draw: then there is no tab to say anything.
15. **An answer says which row it answers.** `Action::AnswerConflict { at, answer }` carries the place of the row the view drew, and the reducer drops an answer for any other row: a click or a key a frame behind is not an answer to the next row.
16. **The question takes no answer in its first 500 ms** (`edit::ANSWER_AFTER`, for every row of the queue). This goes beyond the guards of the Leave box, for two reasons that prompt does not have. It comes up when the database answers, not when the user asks: a key or a click on its way to the grid can land on it, and on Omarchy `k`, `s` and `d` are keys of the grid (up, Structure, Data). And the next row's question takes the place of the last one, button for button: the second click of a double click would answer a row the user never saw. A click, a key or Esc in that window is dropped with no sign: the buttons look as they always do, and nothing is queued for later. The view withholds the answers (`edit::answers_taken`, which reads an instant still to come as no time at all); the reducer stays free of time.
17. **The production confirmation has the same first moment where another dialog's answer opened it.** Overwrite here and Save in the Leave prompt bring it up in the place of the dialog that was just answered, under the same hand. `WritePrompt::after_answer` holds when that was, and its view, in both looks, takes no answer in the 500 ms after: no click on "Save to production" or "Cancel", no Enter on `write`, no Esc. What is typed into Omarchy's field in that time stays typed. Opened by Save itself (the bar, `Mod+S`, Ctrl+S, `:w`) it answers at once, as today.
18. **Esc is Keep mine. Overwrite is drawn as the primary button, and Return does not press it,** as it does not press "Save to production". Enter answers only as the button that has the keyboard when that is Keep mine; on Use server values, Overwrite and Discard my changes it does nothing, and with the keyboard on no button it does nothing. Space presses the button that has the keyboard, as everywhere. On macOS and Windows Keep mine is the first button the Tab key comes to. In Omarchy's box the hints stand in the design's order, so Tab comes to `[o]` first, and Enter there does nothing.
19. **Omarchy's letters:** `k`, `s`, `o`, and `d` for a row that is gone. They are read as typed text, each hint is its button too, and they are ignored while a key is held or a text field has the keyboard, as the Leave box's are. A letter the row does not offer does nothing. Esc is Keep mine and has no hint of its own: the design shows three.
20. **Omarchy's keys stand in the box's foot,** as the keys of the Leave box, the production box and the connection dialog do (`terminal_dialog::keys`): at the foot's right end, each letter in the accent colour and its words dim. This departs from the design's line under the grid with the words in the text colour: one form for every terminal dialog of the app wins over the one artboard.
21. **Under the title, a line says where the row is:** the connection's name and the table's, as the production confirmation names the connection, and "· 1 of 2" after them when the save found more than one row ("Bookshop · book_covers · 1 of 2"). In both looks, always: the question can come up while another tab or another connection is on screen (switching is not guarded), a table of one name can be open on two connections, and "Row id 2" alone would not say of what. It has a line of its own, in the secondary colour, so the title has the sheet's whole width. The question does not switch tabs: an answer from the database takes the user nowhere.
22. **The row is named as the bar names it, and only its name gives way.** `pending_bar::row_name`, which becomes visible to the new view: "Row id 2 changed on the server", "Row org 7, id 2 changed on the server" for a key of two columns, the row's number where the key is not known. A key can be long (a UUID is 36 characters, a text key up to a cell's 256), so the title is laid out from three parts: the name is cut with "…" to what the other words leave of the line, and the whole title is what a screen reader reads and what the pointer shows over it, as `pending_bar::said` does for its line. A row that is gone: "Row id 2 no longer exists on the server" and, under the line of decision 21, "Someone deleted it after you loaded it. Nothing was written."
23. **The table lists the columns the user changed in that row,** which are the row's pending cells, in the page's column order. A text is shown as a grid cell shows one: on one line through `format::cell_line`, a mark where a line breaks, `''` for the empty text and a mark for each character of an all-white one (`format::blank_text`), cut at a cell's 256 characters and again with "…" to its place. Any other value is `format::cell_text` of it. NULL is the grid's NULL, the user's NULL the same. **Two values of a line that differ never read alike for lack of room:** where two of the three would show the same and are not the same text (they differ past the cut), each of those is shown from twelve characters before the first place two of them differ, behind a leading "…". The server's value is marked (red tint, red text; the danger colour on Omarchy) only where it is another value than the loaded one, compared as the save compared them, a float by its bits; a conflict always has at least one such column. A value that became NULL on the server is the NULL chip on the red tint, and on Omarchy the word `NULL` in the danger colour. The user's value is always on the amber tint. For a row that is gone the "now on server" column is left out. More rows than fit scroll. What still reads alike after all that (a tab against a space, a number against the text of its digits) is told apart by the tint alone.
24. **The lines are made once, by the reducer.** `edit::shown_lines` runs when a row's question comes up, and the prompt keeps what it made (`ConflictPrompt::lines`): finding where two values of megabytes first differ is not work for every frame. The view only lays the lines out.
25. **A gone row is drawn dim.** Its cells are written in the dim colour in every look, with no tint and no mark in the gutter. It says why when asked, as every locked cell does (Enter, F2, a double-click; `i` on Omarchy). The design has no drawing for it; the rows of slice 5 (delete) will bring one.
26. **The shortcuts table gains no row.** The question names its own keys, as the Leave box does, whose `[w]` and `[d]` are not in the table either, and the shortcuts cannot be opened while it is up. `src/ui/keys.rs` is not touched.
27. **The question has its own file,** `src/ui/conflict_prompt.rs`. `write_prompts.rs` lends it `Skin`, `fitted`, `button_row` and `keyboard_on` (made `pub(super)`).
28. **Nothing of this prints a value.** `ConflictPrompt` and `edit::Conflicting` have hand-written `Debug`s without the rows, which are the database's and can be megabytes. `edit::Shown` and `edit::ShownLine` hold what the user typed and have no `Debug` at all, as `edit::Pending` has none.
29. **Added here without the spec or the design asking for it,** each small, listed so none passes unseen:
    - the sentence under the title in Omarchy's box ("someone saved it after you loaded it. nothing was written."), which the design's panel does not have and the spec gives for the dialog as such;
    - `ButtonSpec::link` in `src/ui/widgets.rs`, for the design's borderless accent button, rather than painting one in the view;
    - the hover over a cut value and over a cut title, with the whole of what was cut;
    - the view closing the dialog when the tab it asks about is gone (as the production confirmation's view cancels itself);
    - the sizes the design leaves open: the sheet 520 pt wide, its rows 30 pt high, the table at most 220 pt high before it scrolls (the height the confirmation gives its statements); the box 560 pt wide;
    - the words for a row that is gone ("Someone deleted it after you loaded it. Nothing was written.", "Discard my changes" as a plain button).

## Where a run can stop

Each of these leaves the branch shippable:

- **After task 2:** a gone row can be locked and the conflict functions are there and tested; nothing a user sees has changed, since nothing marks a row gone yet.
- **After task 7:** macOS and Windows ask about a conflict, with the sheet. Omarchy still says the line of step 3: `written` asks in the terminal look only from task 8, which draws its box. A gone row is drawn, and the confirmation has its first moment.
- **After task 8:** all three looks ask.
- **After task 9:** the scenes and the documents.

Between tasks 3 and 7 the branch is not shippable on macOS and Windows: from task 3 on a conflict there opens the stand-in question, which names the row and has the answers but shows no values to answer by; task 7 draws it. The stand-in is what lets the model of tasks 3 to 5 be driven through `written` and tested as it will run; holding the question back in those looks too would need a switch that only tests turn, and the tests would then not run what ships. Task 6 can be built any time after task 1.

## File map

| File | What it holds |
|---|---|
| `src/edit.rs` | `Lock::Gone`, `Table::gone`, `Edits::gone`, `Edits::discard`; `Conflicting` and `conflicting`; `Answer`; `ConflictLine` and `conflict_lines`; `settled`; `Shown`, `ShownLine` and `shown_lines`; `ANSWER_AFTER` and `answers_taken`. Pure, with their unit tests. |
| `src/model.rs` | `ConflictPrompt`, `Dialog::Conflict`, `Action::AnswerConflict`; `WritePrompt::after_answer`. |
| `src/app/editing.rs` | `written` opens the question; `answer_conflict`; `conflict_unanswered`; `write_as_answer`. |
| `src/app.rs` | The guard's match, the new arm, `discard` in the two discarding arms, the `LeaveSave` and `CloseDialog` arms, `Event::Rows`. Its tests in `mod editing`. |
| `src/ui/conflict_prompt.rs` (new) | The question: a stand-in first, then the sheet (macOS, Windows) and the box (Omarchy). Its headless tests. |
| `src/ui/mod.rs` | The module and its place in the list of dialogs; three tests of step 3 that expect the line. |
| `src/ui/cell_editor.rs` | The words of `Lock::Gone`. |
| `src/ui/pending_bar.rs` | `row_name` becomes `pub(crate)`. Nothing else. |
| `src/ui/write_prompts.rs` | `confirm_write` withholds answers in the confirmation's first moment; `Skin`, `fitted`, `button_row`, `keyboard_on` become `pub(super)`. |
| `src/ui/widgets.rs` | `ButtonSpec::link`. |
| `src/ui/workspace.rs` | `drawable` becomes `pub(super)`. Nothing else. |
| `src/ui/grid.rs`, `src/ui/data_view.rs` | `Mark::Gone` and where it is set. |
| `src/shots.rs` | Two scenes for review. |
| `docs/superpowers/specs/2026-10-03-value-editing-core-design.md` | "Conflicts" as built. |

---

### Task 1: A row that is gone

**Files:**
- Modify: `src/edit.rs` (`Lock::Gone`, `Table::gone`, `Edits::gone`, `Edits::discard`), `src/ui/cell_editor.rs` (`lock_text`), `src/app.rs` (the two discarding arms, `Event::Rows`), `src/app/editing.rs` (`written`)
- Test: `src/edit.rs`, `src/ui/cell_editor.rs`, `src/app.rs`

A tab can know that some rows of its page no longer exist on the server. Nothing marks a row yet (task 3 does): the tests mark one by hand.

- [ ] **Step 1: Write the failing tests**

At the end of the test module of `src/edit.rs`:

```rust
    #[test]
    fn a_row_a_save_found_gone_is_locked_and_stays_gone_through_a_discard() {
        let (structure, page) = (structure(), page(rows()));
        let gone = BTreeSet::from([1]);
        let table = Table {
            gone: &gone,
            ..table(Some(&structure), &page)
        };
        // Every cell of it, the key's too: the row is not there to edit.
        assert_eq!(table.lock(at(1, 1)), Some(Lock::Gone));
        assert_eq!(table.lock(at(1, 0)), Some(Lock::Gone));
        assert_eq!(table.lock(at(0, 1)), None);
        // What holds for the whole table, or for a while, comes first.
        let read_only = Table {
            access: Access::ReadOnly,
            ..table
        };
        assert_eq!(read_only.lock(at(1, 1)), Some(Lock::ReadOnly));
        let saving = Table {
            saving: true,
            ..table
        };
        assert_eq!(saving.lock(at(1, 1)), Some(Lock::Saving));
        // A gone row is nothing of the user's: it does not hold the page.
        let mut edits = Edits {
            gone: gone.clone(),
            ..Edits::default()
        };
        assert!(!edits.holds());
        // Dropping what is pending does not bring the row back.
        edits.cells.insert(
            (0, 1),
            Pending {
                new: NewValue::Null,
                state: State::Ready,
            },
        );
        edits.note = Some(Note::Cancelled);
        edits.why = Some((at(1, 1), Lock::Gone));
        edits.discard();
        assert!(edits.cells.is_empty() && edits.note.is_none() && edits.why.is_none());
        assert_eq!(edits.gone, gone);
        assert!(!edits.holds());
        assert!(format!("{edits:?}").contains("gone: {1}"));
    }
```

In `src/ui/cell_editor.rs`, in `every_problem_and_every_lock_has_words`: after the line that asserts `why(Lock::KeyIsNull)`, add

```rust
        assert_eq!(why(Lock::Gone), "This row no longer exists on the server");
```

and add `Lock::Gone,` to the list of locks under it, after `Lock::Refreshing,`.

At the end of the nested `mod editing` in `src/app.rs`'s tests:

```rust
        #[test]
        fn a_gone_row_is_locked_until_the_page_is_loaded_again() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            let gone = |harness: &Harness| -> Vec<usize> {
                let edits = &object(harness, tab, id).edits;
                edits.gone.iter().copied().collect()
            };
            // What a save that found the row `id 2` gone leaves on the tab.
            let mark = |harness: &mut Harness| {
                let workspace = harness.app.workspace_mut(tab).unwrap();
                workspace.object_tab_mut(id).unwrap().edits.gone.insert(1);
            };
            mark(&mut harness);
            // Its cells are not edited, and say why when asked.
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none());
            assert_eq!(edits.why, Some((at(1, 1), Lock::Gone)));
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(1, 2),
            });
            harness.app.apply(Action::SetNull { tab, id });
            assert!(object(&harness, tab, id).edits.cells.is_empty());
            // It is still gone after a save of another row that wrote,
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Written {
                rows: vec![row(4, "dan@example.com")],
                elapsed: std::time::Duration::ZERO,
            }));
            assert_eq!(gone(&harness), [1]);
            // after a discard of what is pending,
            type_into(&mut harness, tab, id, at(0, 1), "ada@example.com");
            harness.app.apply(Action::DiscardEdits { tab, id });
            assert_eq!(gone(&harness), [1]);
            // and after a discard from the question before leaving, while
            // the page it leaves is still on screen.
            type_into(&mut harness, tab, id, at(0, 1), "ada@example.com");
            harness.app.apply(Action::Refresh(tab));
            harness.app.apply(Action::LeaveDiscard);
            assert_eq!(gone(&harness), [1]);
            // A fetch that fails leaves that page on screen, and the row
            // gone.
            let fetch = |harness: &Harness| {
                let sent = harness.app.backend.sent.iter().rev();
                let mut fetches = sent.filter_map(|command| match command {
                    Command::FetchRows {
                        session, request, ..
                    } => Some((*session, *request)),
                    _ => None,
                });
                fetches.next().expect("a FetchRows")
            };
            let (session, request) = fetch(&harness);
            harness.app.apply(Action::Backend(Event::Rows {
                session,
                request,
                result: Err(tabletist_db::Error::query("no such table")),
            }));
            assert_eq!(gone(&harness), [1]);
            // A gone row is nothing of the user's: it holds no page, and
            // nothing is asked before the page goes.
            assert!(!object(&harness, tab, id).edits.holds());
            harness.app.apply(Action::Refresh(tab));
            assert!(harness.app.dialog.is_none());
            // The page that arrives is of rows that are there.
            harness.answer_rows(page(5, false));
            assert!(gone(&harness).is_empty());
        }
```

- [ ] **Step 2: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: does not compile (`Lock::Gone`, `Table::gone`, `Edits::gone`, `Edits::discard`).

- [ ] **Step 3: `src/edit.rs`**

The first import becomes `use std::collections::{BTreeMap, BTreeSet};`.

`Lock` gains a variant, after `NoSuchCell`:

```rust
    /// A save found the row gone from the server.
    Gone,
```

`Table` gains a field, after `saving`:

```rust
    /// The page's rows a save found gone from the server.
    pub gone: &'a BTreeSet<usize>,
```

and `Table::of` fills it, after its `saving:` line: `gone: &object.edits.gone,`.

In `Table::lock`, after the two `NoSuchCell` returns and before the comment "A row narrower than the page has no key to read.":

```rust
        // Before anything its values say: they are of a row that is no
        // longer there.
        if self.gone.contains(&cell.row) {
            return Some(Lock::Gone);
        }
```

`Edits` gains a field, after `note`:

```rust
    /// The page's rows a save found gone from the server. They are no
    /// rows to edit until the page is loaded again, and nothing of the
    /// user's: they do not hold the page.
    pub gone: BTreeSet<usize>,
```

`holds` stays as it is. In `impl Edits`, before `counts`:

```rust
    /// Drops what is pending, the open editor and what the last save left.
    /// What is known of the page itself stays: the rows that are gone.
    pub fn discard(&mut self) {
        *self = Self {
            gone: std::mem::take(&mut self.gone),
            ..Self::default()
        };
    }
```

The `Debug` of `Edits` prints the rows too. Its format string becomes `"Edits {{ cells: {}, editor: {:?}, why: {:?}, saving: {}, gone: {:?} }}"`, and `self.gone` is its last argument, after `self.saving.is_some()`.

In the tests, the `table` helper builds a `Table` field by field. Above it add

```rust
    /// No row is gone.
    static NONE_GONE: BTreeSet<usize> = BTreeSet::new();
```

and end its literal with `gone: &NONE_GONE,`. Every other `Table { .. }` in the tests is made from that helper with `..`, and needs nothing.

- [ ] **Step 4: The words**

In `lock_text` (`src/ui/cell_editor.rs`), after the `Lock::Refreshing` arm:

```rust
        Lock::Gone => say("This row no longer exists on the server"),
```

`workspace::lock_line` lowers the same words for Omarchy and needs nothing.

- [ ] **Step 5: The reducer keeps the gone rows through a discard**

Three places set `edits` to its default where only what is pending is meant:

- `src/app.rs`, the `Action::DiscardEdits` arm: `object.edits = crate::edit::Edits::default();` becomes `object.edits.discard();`.
- `src/app.rs`, the `Action::LeaveDiscard` arm, in its loop over the tabs: the same change.
- `src/app/editing.rs`, `written`, the `Written` arm: `object.edits = Edits::default();` becomes

```rust
                // The rows an earlier save found gone are still gone.
                object.edits.discard();
```

  `Edits` is then no longer named in that file: take it out of the `use crate::edit::{..}` list.

In the `Event::Rows` arm of `src/app.rs`, a page that arrives still forgets everything, and a fetch that failed keeps the gone rows of the page that stays on screen. The lines

```rust
                object.rows.finish(request, result);
                // The marks of the last save and the note of a locked cell
                // were about the page this one replaces. Nothing is pending
                // here: a tab that holds edits is never fetched again (the
                // guard at the top of `apply` is what makes that so).
                object.edits = crate::edit::Edits::default();
```

become

```rust
                let failed = result.is_err();
                object.rows.finish(request, result);
                // The marks of the last save and the note of a locked cell
                // were about the page this one replaces. Nothing is pending
                // here: a tab that holds edits is never fetched again (the
                // guard at the top of `apply` is what makes that so).
                if failed {
                    // The page on screen is still the one a save found
                    // rows gone from.
                    object.edits.discard();
                } else {
                    object.edits = crate::edit::Edits::default();
                }
```

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: all pass, the two new tests and `every_problem_and_every_lock_has_words` among them.

- [ ] **Step 7: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A && git commit -m "Lock a row a save found gone until its page is loaded again"
```

---

### Task 2: What a conflict shows, and what an answer settles

**Files:**
- Modify: `src/edit.rs`
- Test: `src/edit.rs`

Pure functions and their types. The reducer of tasks 3 to 5 and the views of tasks 7 and 8 are built on them.

- [ ] **Step 1: Write the failing tests**

At the end of the test module of `src/edit.rs`:

```rust
    fn pending(new: NewValue) -> Pending {
        Pending {
            new,
            state: State::Ready,
        }
    }

    #[test]
    fn a_saves_conflicts_are_rows_of_the_page_in_the_saves_order() {
        let page = page(rows());
        let server = vec![Value::Int(2), text("eve@example.com"), text("{}")];
        // The set's rows 0 and 1 were the page's rows 1 and 0.
        let places = [1, 0];
        let both = vec![
            Conflict {
                row: 0,
                server: Some(server.clone()),
            },
            Conflict {
                row: 1,
                server: None,
            },
        ];
        assert_eq!(
            conflicting(&places, both, &page),
            Some(vec![
                Conflicting {
                    row: 1,
                    server: Some(server),
                },
                Conflicting {
                    row: 0,
                    server: None,
                },
            ])
        );
        let one = |row: usize, server: Option<Vec<Value>>| vec![Conflict { row, server }];
        // Nothing to ask from: a row the save did not send, a row the page
        // does not hold, one row twice, a row of another width.
        assert_eq!(conflicting(&places, one(2, None), &page), None);
        assert_eq!(conflicting(&[7], one(0, None), &page), None);
        let twice = [one(0, None), one(0, None)].concat();
        assert_eq!(conflicting(&places, twice, &page), None);
        let narrow = one(0, Some(vec![Value::Int(2)]));
        assert_eq!(conflicting(&places, narrow, &page), None);
        // No conflict is no row.
        assert_eq!(conflicting(&places, Vec::new(), &page), Some(Vec::new()));
        // A row is printed without what the database holds in it.
        let row = Conflicting {
            row: 1,
            server: Some(vec![text("secret")]),
        };
        assert_eq!(format!("{row:?}"), "Conflicting { row: 1, gone: false }");
    }

    #[test]
    fn a_conflict_shows_the_columns_the_user_changed_and_which_of_them_moved() {
        let page = page(rows());
        let mut cells = BTreeMap::new();
        cells.insert((1, 2), pending(NewValue::Null));
        cells.insert((1, 1), pending(NewValue::Text("bobby@example.com".into())));
        cells.insert((0, 1), pending(NewValue::Text("a@example.com".into())));
        let changed = Conflicting {
            row: 1,
            server: Some(vec![Value::Int(2), text("eve@example.com"), text("{}")]),
        };
        let lines = conflict_lines(&page, &cells, &changed);
        // The row's own cells and no other's, in the page's column order.
        let cols: Vec<usize> = lines.iter().map(|line| line.col).collect();
        assert_eq!(cols, [1, 2]);
        assert_eq!(lines[0].loaded, &text("bob@example.com"));
        assert_eq!(lines[0].server, Some(&text("eve@example.com")));
        assert_eq!(lines[0].yours, &NewValue::Text("bobby@example.com".into()));
        assert_eq!(lines[1].yours, &NewValue::Null);
        // The email is another on the server, and meta is as it was loaded.
        assert!(lines[0].moved && !lines[1].moved);
        // A row that is gone has nothing on the server.
        let gone = Conflicting {
            row: 1,
            server: None,
        };
        let lines = conflict_lines(&page, &cells, &gone);
        assert_eq!(lines.len(), 2);
        assert!(
            lines
                .iter()
                .all(|line| line.server.is_none() && !line.moved)
        );
        // Not a number is the value it was, as the save compares it.
        let floats = self::page(vec![vec![
            Value::Int(1),
            Value::Float(f64::NAN),
            Value::Null,
        ]]);
        let mut cells = BTreeMap::new();
        cells.insert((0, 1), pending(NewValue::Text("1.5".into())));
        cells.insert((0, 2), pending(NewValue::Text("{}".into())));
        let same = Conflicting {
            row: 0,
            server: Some(vec![Value::Int(1), Value::Float(f64::NAN), text("[]")]),
        };
        let lines = conflict_lines(&floats, &cells, &same);
        assert!(!lines[0].moved && lines[1].moved);
        // A row the page does not hold shows nothing.
        let missing = Conflicting {
            row: 9,
            server: None,
        };
        assert!(conflict_lines(&page, &cells, &missing).is_empty());
        // Each answer is offered where it means something.
        for answer in [Answer::UseServer, Answer::Overwrite] {
            assert!(answer.offered(false) && !answer.offered(true), "{answer:?}");
        }
        assert!(Answer::Discard.offered(true) && !Answer::Discard.offered(false));
        assert!(Answer::KeepMine.offered(true) && Answer::KeepMine.offered(false));
    }

    #[test]
    fn a_pending_value_the_server_holds_now_is_no_change_any_more() {
        let (structure, page) = (structure(), page(rows()));
        let table = table(Some(&structure), &page);
        let mut cells = BTreeMap::new();
        cells.insert((1, 1), pending(NewValue::Text("eve@example.com".into())));
        cells.insert((1, 2), pending(NewValue::Null));
        cells.insert((0, 1), pending(NewValue::Text("eve@example.com".into())));
        // The server holds the new email, and still a value in meta.
        let server = [Value::Int(2), text("eve@example.com"), text("{}")];
        assert_eq!(settled(&table, &cells, 1, &server), [1]);
        // It holds NULL in meta now: both cells are what it has.
        let server = [Value::Int(2), text("eve@example.com"), Value::Null];
        assert_eq!(settled(&table, &cells, 1, &server), [1, 2]);
        // Text is never NULL: the empty string over a NULL stays a change.
        cells.insert((1, 2), pending(NewValue::Text(String::new())));
        assert_eq!(settled(&table, &cells, 1, &server), [1]);
        // A cell to fix or failed is settled the same way: its state is
        // about the new value, not about what was loaded.
        cells.insert(
            (1, 1),
            Pending {
                new: NewValue::Text("eve@example.com".into()),
                state: State::Failed(Error::query("violates check")),
            },
        );
        assert_eq!(settled(&table, &cells, 1, &server), [1]);
        // Another row's cells are not this row's to settle, and a row
        // narrower than the page settles nothing.
        let other = [Value::Int(1), text("x@example.com"), Value::Null];
        assert_eq!(settled(&table, &cells, 0, &other), Vec::<usize>::new());
        assert_eq!(
            settled(&table, &cells, 1, &[Value::Int(2)]),
            Vec::<usize>::new()
        );
        // A boolean column is compared as its editor starts: `true` is the
        // 1 SQLite holds.
        let mut flags = self::structure();
        flags.columns[2].type_name = "BOOLEAN".into();
        let table = self::table(Some(&flags), &page);
        let mut cells = BTreeMap::new();
        cells.insert((1, 2), pending(NewValue::Text("true".into())));
        let server = [Value::Int(2), text("bob@example.com"), Value::Int(1)];
        assert_eq!(settled(&table, &cells, 1, &server), [2]);
        let server = [Value::Int(2), text("bob@example.com"), Value::Int(0)];
        assert_eq!(settled(&table, &cells, 1, &server), Vec::<usize>::new());
    }

    /// A shown value by its text and whether it starts inside the value.
    /// `None` is NULL.
    fn seen(shown: &Shown) -> Option<(&str, bool)> {
        match shown {
            Shown::Null => None,
            Shown::Text { text, cut } => Some((text.as_str(), *cut)),
        }
    }

    #[test]
    fn two_values_of_a_line_that_differ_are_not_shown_alike() {
        // Three documents that are the same for 300 characters.
        let long = |end: &str| format!("{}{end}", "x".repeat(300));
        let loaded = vec![Value::Int(1), text("a b"), text(&long("loaded"))];
        let page = self::page(vec![loaded]);
        let mut cells = BTreeMap::new();
        cells.insert((0, 1), pending(NewValue::Text("a\nb".into())));
        cells.insert((0, 2), pending(NewValue::Text(long("yours"))));
        let conflict = Conflicting {
            row: 0,
            server: Some(vec![Value::Int(1), text("a b"), text(&long("server"))]),
        };
        let lines = shown_lines(&page, &cells, &conflict);
        assert_eq!(lines.len(), 2);
        let (email, meta) = (&lines[0], &lines[1]);
        assert_eq!((email.name.as_str(), meta.name.as_str()), ("email", "meta"));
        // A line break is not a space: a cell marks it, so the two read
        // apart as they are, each from its start.
        assert_eq!(seen(&email.loaded), Some(("a b", false)));
        assert_eq!(seen(&email.yours), Some(("a\nb", false)));
        assert_eq!(email.server.as_ref().map(seen), Some(Some(("a b", false))));
        assert!(!email.moved);
        // The documents read alike for all a cell shows of them: each is
        // shown from twelve characters before the first place two differ.
        let piece = |end: &str| format!("{}{end}", "x".repeat(12));
        assert_eq!(seen(&meta.loaded), Some((piece("loaded").as_str(), true)));
        assert_eq!(seen(&meta.yours), Some((piece("yours").as_str(), true)));
        let server = meta.server.as_ref().map(seen);
        assert_eq!(server, Some(Some((piece("server").as_str(), true))));
        assert!(meta.moved);
        // Only what reads like another value is shown from inside: a short
        // value beside two long ones is whole.
        cells.insert((0, 2), pending(NewValue::Text("{}".into())));
        let lines = shown_lines(&page, &cells, &conflict);
        assert_eq!(seen(&lines[1].yours), Some(("{}", false)));
        assert_eq!(
            seen(&lines[1].loaded),
            Some((piece("loaded").as_str(), true))
        );
        // A value the server kept is the same text, not one that reads
        // like it: both are shown from the start, a cell's worth and one
        // character more for the cell to cut at.
        let kept = Conflicting {
            row: 0,
            server: Some(vec![Value::Int(1), text("c d"), text(&long("loaded"))]),
        };
        let lines = shown_lines(&page, &cells, &kept);
        let start = "x".repeat(257);
        assert_eq!(seen(&lines[1].loaded), Some((start.as_str(), false)));
        assert_eq!(
            lines[1].server.as_ref().map(seen),
            Some(Some((start.as_str(), false)))
        );
        assert!(lines[0].moved && !lines[1].moved);
    }

    #[test]
    fn null_numbers_and_a_row_that_is_gone_are_shown_as_they_are() {
        let page = self::page(vec![vec![Value::Int(7), Value::Null, Value::Float(1.5)]]);
        let mut cells = BTreeMap::new();
        cells.insert((0, 1), pending(NewValue::Text(String::new())));
        cells.insert((0, 2), pending(NewValue::Null));
        let conflict = Conflicting {
            row: 0,
            server: Some(vec![Value::Int(7), text("x"), Value::Null]),
        };
        let lines = shown_lines(&page, &cells, &conflict);
        // NULL is NULL and the empty text is a text: the cell that draws
        // them tells them apart.
        assert_eq!(seen(&lines[0].loaded), None);
        assert_eq!(seen(&lines[0].yours), Some(("", false)));
        assert_eq!(lines[0].server.as_ref().map(seen), Some(Some(("x", false))));
        // A number as a cell writes it, and a value that became NULL.
        assert_eq!(seen(&lines[1].loaded), Some(("1.5", false)));
        assert_eq!(seen(&lines[1].yours), None);
        assert_eq!(lines[1].server.as_ref().map(seen), Some(None));
        assert!(lines[0].moved && lines[1].moved);
        // A row that is gone has no third value.
        let gone = Conflicting {
            row: 0,
            server: None,
        };
        let lines = shown_lines(&page, &cells, &gone);
        assert_eq!(lines.len(), 2);
        assert!(
            lines
                .iter()
                .all(|line| line.server.is_none() && !line.moved)
        );
    }

    #[test]
    fn a_question_takes_answers_once_it_has_been_up_for_a_moment() {
        let now = Instant::now();
        assert!(!answers_taken(now));
        // An instant still to come has lasted no time.
        assert!(!answers_taken(now + Duration::from_secs(3600)));
        let earlier = now.checked_sub(ANSWER_AFTER).expect("an earlier instant");
        assert!(answers_taken(earlier));
        assert_eq!(ANSWER_AFTER, Duration::from_millis(500));
    }
```

- [ ] **Step 2: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib edit::`
Expected: does not compile (`conflicting`, `Conflicting`, `conflict_lines`, `Answer`, `settled`, `shown_lines`, `Shown`, `answers_taken`, `Conflict`).

- [ ] **Step 3: The first moment**

Above `Note` in `src/edit.rs`:

```rust
/// How long a question that came up unasked has been on screen before it
/// takes an answer: the question about a row a save found changed, which
/// comes when the database answers, and the production confirmation where
/// another dialog's answer opened it. What was on its way elsewhere then
/// (a key, a click) is no answer to it, and the second click of a double
/// click is none to the question that took the first one's place.
pub const ANSWER_AFTER: Duration = Duration::from_millis(500);

/// Whether a question that came up at `shown` takes answers yet. An
/// instant still to come has lasted no time.
pub fn answers_taken(shown: Instant) -> bool {
    Instant::now().saturating_duration_since(shown) >= ANSWER_AFTER
}
```

- [ ] **Step 4: The conflict's functions**

Add `Conflict` to the names taken from `tabletist_db` at the top of `src/edit.rs` (rustfmt orders and wraps the list). Then, between `Note` and `change_set`:

```rust
/// A row a save found changed on the server, by its place in the page.
#[derive(Clone, PartialEq)]
pub struct Conflicting {
    /// The page's row.
    pub row: usize,
    /// The row as the database holds it now, as wide as the page. `None`
    /// when it is gone.
    pub server: Option<Vec<Value>>,
}

/// Without the row: it is the database's, and can be megabytes.
impl std::fmt::Debug for Conflicting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Conflicting {{ row: {}, gone: {} }}",
            self.row,
            self.server.is_none()
        )
    }
}

/// A save's conflicts as rows of `page`, in the save's order. `places` is
/// the page's row of each row the save sent (`Saving::rows`): the answer
/// names rows by their place in the set. `None` when there is nothing to
/// ask from it: it names a row the save did not send or the page does not
/// hold, the same row twice, or brings a row that is not as wide as the
/// page (the table is no longer the one the page was read from).
pub fn conflicting(
    places: &[usize],
    conflicts: Vec<Conflict>,
    page: &RowPage,
) -> Option<Vec<Conflicting>> {
    let mut seen = BTreeSet::new();
    conflicts
        .into_iter()
        .map(|conflict| {
            let row = *places.get(conflict.row)?;
            page.rows.get(row)?;
            if !seen.insert(row) {
                return None;
            }
            let fits = |server: &Vec<Value>| server.len() == page.columns.len();
            if !conflict.server.as_ref().is_none_or(fits) {
                return None;
            }
            Some(Conflicting {
                row,
                server: conflict.server,
            })
        })
        .collect()
}

/// What the user answers about a row a save found changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// The server's row becomes the loaded one and the pending cells stay
    /// on top of it. For a row that is gone: its cells stay as they are.
    KeepMine,
    /// The server's row becomes the loaded one and the row's pending cells
    /// are dropped.
    UseServer,
    /// As `KeepMine`, and the save may run again once every row is
    /// answered.
    Overwrite,
    /// For a row that is gone: its pending cells are dropped.
    Discard,
}

impl Answer {
    /// Whether the question about a row offers this answer: `gone` says
    /// the row no longer exists.
    pub fn offered(self, gone: bool) -> bool {
        match self {
            Self::KeepMine => true,
            Self::UseServer | Self::Overwrite => !gone,
            Self::Discard => gone,
        }
    }
}

/// One column the user changed in a row a save found changed, as the
/// question about the row shows it.
#[derive(Clone, Copy, PartialEq)]
pub struct ConflictLine<'a> {
    /// The page's column.
    pub col: usize,
    pub loaded: &'a Value,
    /// What the database holds now. `None` when the row is gone.
    pub server: Option<&'a Value>,
    pub yours: &'a NewValue,
    /// The server's value is another than the loaded one: this column is
    /// one the conflict is about.
    pub moved: bool,
}

/// The columns the user changed in `conflict`'s row, in the page's order:
/// what the page loaded, what the server holds now and the pending value.
pub fn conflict_lines<'a>(
    page: &'a RowPage,
    cells: &'a BTreeMap<(usize, usize), Pending>,
    conflict: &'a Conflicting,
) -> Vec<ConflictLine<'a>> {
    let Some(loaded) = page.rows.get(conflict.row) else {
        return Vec::new();
    };
    let row = conflict.row;
    cells
        .range((row, 0)..=(row, usize::MAX))
        .filter_map(|(&(_, col), pending)| {
            let loaded = loaded.get(col)?;
            let server = match &conflict.server {
                Some(server) => Some(server.get(col)?),
                None => None,
            };
            Some(ConflictLine {
                col,
                loaded,
                server,
                yours: &pending.new,
                // As the save compared them: a float by its bits.
                moved: server.is_some_and(|server| !same_value(server, loaded)),
            })
        })
        .collect()
}

/// The pending cells of the page's row `row`, by their column, that are no
/// change any more once the row holds `server`: a new value equal to what
/// the server holds now is where an editor on it would start.
pub fn settled(
    table: &Table<'_>,
    cells: &BTreeMap<(usize, usize), Pending>,
    row: usize,
    server: &[Value],
) -> Vec<usize> {
    cells
        .range((row, 0)..=(row, usize::MAX))
        .filter(|&(&(_, col), pending)| {
            let class = table.class(col).unwrap_or(ColumnClass::Other);
            server
                .get(col)
                .is_some_and(|now| !is_change(now, &pending.new, class))
        })
        .map(|(&(_, col), _)| col)
        .collect()
}

/// A value as the question about a conflict shows it.
#[derive(Clone, PartialEq)]
pub enum Shown {
    Null,
    /// The value's text, or the part of it to show: at most a cell's worth
    /// of characters and one more, so the cell that draws it still marks
    /// what it cuts. `cut` says the text starts inside the value: the
    /// values of its line read alike up to there.
    Text {
        text: String,
        cut: bool,
    },
}

/// One column the user changed in a row a save found changed, made ready
/// to draw once, when its question comes up: a value can be megabytes,
/// too much to compare in every frame.
#[derive(Clone, PartialEq)]
pub struct ShownLine {
    /// The column's name, as the page has it.
    pub name: String,
    pub loaded: Shown,
    /// What the database holds now. `None` when the row is gone.
    pub server: Option<Shown>,
    pub yours: Shown,
    /// The server's value is another than the loaded one.
    pub moved: bool,
}

/// How many characters stand before the first difference of two values
/// that are shown from there: enough to find the place by.
const LEAD: usize = 12;

/// What a cell makes of `text`: one line, a cell's worth of it. Two texts
/// with the same answer read the same in the question.
fn read(text: &str) -> String {
    use crate::ui::format::{Marks, blank_text, cell_line};
    blank_text(text, Marks::PLAIN).unwrap_or_else(|| cell_line(text, Marks::PLAIN).into_owned())
}

/// The values of one line as they are shown, in the order given (`None`
/// is NULL). Each is a cell's worth of its text from the start, unless two
/// of them read alike there and are not the same text (they differ past
/// what a cell shows): those are shown from just before the first place
/// two of them differ, so the difference is on screen.
fn shown(values: &[Option<&str>]) -> Vec<Shown> {
    let reads: Vec<Option<String>> = values.iter().map(|text| text.map(read)).collect();
    // Which values read like another that they are not, and the first
    // place such a pair differs.
    let mut alike = vec![false; values.len()];
    let mut from = usize::MAX;
    for (later, theirs) in values.iter().enumerate() {
        for (earlier, ours) in values.iter().enumerate().take(later) {
            let (Some(ours), Some(theirs)) = (ours, theirs) else {
                continue;
            };
            if ours != theirs && reads[earlier] == reads[later] {
                let same = ours.chars().zip(theirs.chars());
                from = from.min(same.take_while(|(ours, theirs)| ours == theirs).count());
                alike[earlier] = true;
                alike[later] = true;
            }
        }
    }
    let from = from.saturating_sub(LEAD);
    let piece = crate::ui::format::CELL_MAX_CHARS + 1;
    values
        .iter()
        .zip(alike)
        .map(|(text, alike)| match text {
            None => Shown::Null,
            Some(text) => {
                let skip = if alike { from } else { 0 };
                Shown::Text {
                    text: text.chars().skip(skip).take(piece).collect(),
                    cut: skip > 0,
                }
            }
        })
        .collect()
}

/// The lines the question about `conflict`'s row shows: one for each
/// column the user changed, in the page's order.
pub fn shown_lines(
    page: &RowPage,
    cells: &BTreeMap<(usize, usize), Pending>,
    conflict: &Conflicting,
) -> Vec<ShownLine> {
    // A text as it is; any other value as a cell writes it, which is short.
    fn text(value: &Value) -> Option<std::borrow::Cow<'_, str>> {
        match value {
            Value::Null => None,
            Value::Text(text) => Some(std::borrow::Cow::Borrowed(text)),
            other => Some(crate::ui::format::cell_text(other)),
        }
    }
    conflict_lines(page, cells, conflict)
        .into_iter()
        .filter_map(|line| {
            let loaded = text(line.loaded);
            let server = line.server.map(text);
            let yours = match line.yours {
                NewValue::Null => None,
                NewValue::Text(text) => Some(text.as_str()),
            };
            let mut values = vec![loaded.as_deref(), yours];
            if let Some(server) = &server {
                values.push(server.as_deref());
            }
            let mut values = shown(&values).into_iter();
            Some(ShownLine {
                name: page.columns.get(line.col)?.name.clone(),
                loaded: values.next()?,
                yours: values.next()?,
                server: values.next(),
                moved: line.moved,
            })
        })
        .collect()
}
```

`same_value` is the private function further down in the file: the comparison a save makes, a float by its bits. `Shown` and `ShownLine` have no `Debug` on purpose (decision 28): the tests read them through `seen`.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib edit::`
Expected: all pass, the six new tests among them.

- [ ] **Step 6: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A && git commit -m "Work out what a conflict shows and what an answer settles"
```

---

### Task 3: The question, and what each answer does

**Files:**
- Create: `src/ui/conflict_prompt.rs`
- Modify: `src/model.rs` (`ConflictPrompt`, `Dialog::Conflict`, `Action::AnswerConflict`), `src/app/editing.rs` (`written`, `answer_conflict`, `conflict_unanswered`), `src/app.rs` (the guard's match, the new arm, `CloseDialog`, `Event::Rows`), `src/ui/mod.rs` (the module, the list of dialogs), `src/ui/pending_bar.rs` (`row_name`)
- Test: `src/app.rs`, `src/ui/conflict_prompt.rs`, and one test of `src/ui/mod.rs`

A save that ends in conflicts opens the question, and each answer rebases its row. The save that may follow the last answer is task 4: here Overwrite does what Keep mine does and no more. The question is drawn by a stand-in, as the two prompts of step 3 first were: it names the row, says where it is and has the answers, so the model can be driven and tested; tasks 7 and 8 replace its body.

**In the terminal look nothing changes yet.** `written` asks only where the question is drawn for the look, and Omarchy's box is task 8: until then a conflict there is the line of step 3, and the two Omarchy tests of step 3 that expect that line pass untouched and pin it.

Three tests of step 3 pin what this task replaces in the other looks (a conflict is a line and nothing else). They are changed here, not deleted: two in `src/app.rs` now answer the question, and one in `src/ui/mod.rs` keeps testing the line, which a conflict still is where the question cannot be asked.

- [ ] **Step 1: The reducer's tests**

In the nested `mod editing` of `src/app.rs`'s tests, extend the first import to `use crate::edit::{Answer, Conflicting, Lock, Problem, State};`. At the end of the module, the helpers:

```rust
        /// What the conflict question asks about: the place of the row
        /// among the save's conflicts, the page's row, and how many rows
        /// the save found changed.
        fn asking(harness: &Harness) -> Option<(usize, usize, usize)> {
            match &harness.app.dialog {
                Some(Dialog::Conflict(prompt)) => {
                    let row = prompt.rows.get(prompt.at)?.row;
                    Some((prompt.at, row, prompt.rows.len()))
                }
                _ => None,
            }
        }

        /// The columns the question shows, each with the loaded value it
        /// shows for it (`None` for NULL).
        fn shown(harness: &Harness) -> Vec<(String, Option<String>)> {
            let Some(Dialog::Conflict(prompt)) = &harness.app.dialog else {
                panic!("no conflict question: {:?}", harness.app.dialog);
            };
            let loaded = |line: &crate::edit::ShownLine| match &line.loaded {
                crate::edit::Shown::Null => None,
                crate::edit::Shown::Text { text, .. } => Some(text.clone()),
            };
            let lines = prompt.lines.iter();
            lines
                .map(|line| (line.name.clone(), loaded(line)))
                .collect()
        }

        /// Answers the question for the row it shows.
        fn answer(harness: &mut Harness, answer: Answer) {
            let Some((at, ..)) = asking(harness) else {
                panic!("no conflict question: {:?}", harness.app.dialog);
            };
            harness.app.apply(Action::AnswerConflict { at, answer });
        }

        /// The set's row `row` holds `email` on the server now.
        fn changed(row: usize, id: i64, email: &str) -> Conflict {
            Conflict {
                row,
                server: Some(self::row(id, email)),
            }
        }

        fn gone(row: usize) -> Conflict {
            Conflict { row, server: None }
        }

        /// The cells that are pending, by row and column.
        fn pending(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<(usize, usize)> {
            let edits = &object(harness, tab, id).edits;
            edits.cells.keys().copied().collect()
        }

        fn email(harness: &Harness, tab: ConnTabId, id: TabId, row: usize) -> Value {
            object(harness, tab, id).page().unwrap().rows[row][1].clone()
        }

        fn text(value: &str) -> Value {
            Value::Text(value.into())
        }

        /// The emails of the page's rows 1 and 3 (`id` 2 and 4) pending,
        /// saved, and the save answered with `conflicts`.
        fn conflicts(harness: &mut Harness, conflicts: Vec<Conflict>) -> (ConnTabId, TabId) {
            let (tab, id) = harness.editable();
            type_into(harness, tab, id, at(1, 1), "bob@example.com");
            type_into(harness, tab, id, at(3, 1), "dan@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(conflicts)));
            (tab, id)
        }

        /// Both rows of `conflicts` found changed on the server.
        fn two_conflicts(harness: &mut Harness) -> (ConnTabId, TabId) {
            let both = vec![
                changed(0, 2, "eve@example.com"),
                changed(1, 4, "fay@example.com"),
            ];
            conflicts(harness, both)
        }
```

and the tests:

```rust
        #[test]
        fn a_conflict_asks_about_its_row_and_nothing_changes_the_set_under_the_question() {
            let mut harness = Harness::new();
            // The set's second row is the page's row 3.
            let (tab, id) = conflicts(&mut harness, vec![changed(1, 4, "fay@example.com")]);
            match &harness.app.dialog {
                Some(Dialog::Conflict(prompt)) => {
                    assert_eq!((prompt.tab, prompt.id, prompt.at), (tab, id, 0));
                    assert_eq!(
                        prompt.rows,
                        [Conflicting {
                            row: 3,
                            server: Some(row(4, "fay@example.com")),
                        }]
                    );
                }
                other => panic!("expected the conflict question, got {other:?}"),
            }
            // It shows the one column changed in that row.
            let line = ("email".to_owned(), Some("user4@example.com".to_owned()));
            assert_eq!(shown(&harness), [line]);
            // Nothing was written and nothing is rebased before an answer.
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.saving.is_none() && edits.note.is_none());
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            assert_eq!(email(&harness, tab, id, 3), text("user4@example.com"));
            // What edits, saves, discards or moves is dropped under it.
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: at(1, 1),
            });
            harness.app.apply(Action::RevertCell { tab, id });
            harness.app.apply(Action::DiscardEdits { tab, id });
            harness.app.apply(Action::WriteEdits { tab, id });
            change(&mut harness, tab, id, at(0, 1), "ada@example.com");
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            assert!(write_since(&harness, before).is_none());
            // What would drop the page is refused, with the reason.
            harness.app.apply(Action::CloseTab { tab, id });
            assert_eq!(asking(&harness), Some((0, 3, 1)));
            assert!(harness.app.notice.is_some());
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
            // The answers of the other prompts are not answers to it.
            for other in [
                Action::LeaveDiscard,
                Action::LeaveSave,
                Action::LeaveStay,
                Action::ConfirmWrite,
                Action::CancelWrite,
            ] {
                harness.app.apply(other);
                assert_eq!(asking(&harness), Some((0, 3, 1)));
            }
            // Nor is its answer one to another dialog.
            answer(&mut harness, Answer::KeepMine);
            harness.app.apply(Action::ShowHelp);
            harness.app.apply(Action::AnswerConflict {
                at: 0,
                answer: Answer::UseServer,
            });
            assert!(matches!(harness.app.dialog, Some(Dialog::Help)));
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
        }

        #[test]
        fn keep_mine_reloads_the_row_and_keeps_the_cells_that_still_differ() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            type_into(&mut harness, tab, id, at(1, 2), "{}");
            type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            // Someone gave the row the email typed here, and another meta.
            let server = vec![Value::Int(2), text("bob@example.com"), text("[1]")];
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![Conflict {
                row: 0,
                server: Some(server.clone()),
            }])));
            // The row panel holds the row's text as it was loaded.
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .fields = Some(crate::model::RowFields {
                request: None,
                row: 1,
                fields: Vec::new(),
                pending: Vec::new(),
            });
            let before = harness.app.backend.sent.len();
            // Discard is the answer for a row that is gone, not for this one.
            answer(&mut harness, Answer::Discard);
            assert_eq!(asking(&harness), Some((0, 1, 1)));
            assert_eq!(pending(&harness, tab, id), [(1, 1), (1, 2), (3, 1)]);
            answer(&mut harness, Answer::KeepMine);
            assert!(harness.app.dialog.is_none());
            let now = object(&harness, tab, id);
            // The server's row is the loaded one now.
            assert_eq!(now.page().unwrap().rows[1], server);
            // The email is what the server holds: no change any more. The
            // meta still differs, and stays on top.
            assert_eq!(pending(&harness, tab, id), [(1, 2), (3, 1)]);
            assert_eq!(
                now.edits.cells.get(&(1, 2)).map(|cell| &cell.new),
                Some(&NewValue::Text("{}".into()))
            );
            // The row panel's text is formatted again.
            assert!(now.fields.is_none());
            assert!(now.edits.gone.is_empty() && now.edits.note.is_none());
            // Nothing is saved: what is left waits for the user.
            assert!(write_since(&harness, before).is_none());
            assert!(now.edits.saving.is_none());
        }

        #[test]
        fn use_server_values_reloads_the_row_and_drops_its_cells() {
            let mut harness = Harness::new();
            let (tab, id) = conflicts(&mut harness, vec![changed(0, 2, "eve@example.com")]);
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::UseServer);
            assert!(harness.app.dialog.is_none());
            assert_eq!(email(&harness, tab, id, 1), text("eve@example.com"));
            // The other row did not conflict: it stays pending, unsaved.
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
            assert_eq!(email(&harness, tab, id, 3), text("user4@example.com"));
            assert!(write_since(&harness, before).is_none());
        }

        #[test]
        fn a_row_that_is_gone_can_only_be_discarded_and_is_marked_gone() {
            let mut harness = Harness::new();
            let (tab, id) = conflicts(&mut harness, vec![gone(0)]);
            let before = harness.app.backend.sent.len();
            // There is no row on the server to take or to write over.
            answer(&mut harness, Answer::UseServer);
            answer(&mut harness, Answer::Overwrite);
            assert_eq!(asking(&harness), Some((0, 1, 1)));
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            answer(&mut harness, Answer::Discard);
            assert!(harness.app.dialog.is_none());
            // Its cells are dropped; the row that did not conflict stays
            // pending, unsaved.
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
            assert!(write_since(&harness, before).is_none());
            let edits = &object(&harness, tab, id).edits;
            assert_eq!(edits.gone.iter().copied().collect::<Vec<_>>(), [1]);
            // The row stays on the page as it was loaded, and is locked.
            assert_eq!(email(&harness, tab, id, 1), text("user2@example.com"));
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 1),
                start: EditStart::Value,
            });
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_none());
            assert_eq!(edits.why, Some((at(1, 1), Lock::Gone)));
        }

        #[test]
        fn keeping_a_row_that_is_gone_leaves_its_cells_and_says_so() {
            let mut harness = Harness::new();
            let (tab, id) = conflicts(&mut harness, vec![gone(0)]);
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::KeepMine);
            assert!(harness.app.dialog.is_none());
            // Nothing is dropped and nothing is marked or locked.
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.gone.is_empty());
            assert!(write_since(&harness, before).is_none());
            // The tab says that the row is not there: it is not left
            // looking like any other row with pending changes.
            assert_eq!(
                edits.note,
                Some(crate::edit::Note::Conflict {
                    row: 1,
                    gone: true,
                    others: 0
                })
            );
            // A row that changed and was kept has nothing to say: it is
            // on the page as the server has it.
            let mut harness = Harness::new();
            let (tab, id) = conflicts(&mut harness, vec![changed(0, 2, "eve@example.com")]);
            answer(&mut harness, Answer::KeepMine);
            assert_eq!(object(&harness, tab, id).edits.note, None);
        }

        #[test]
        fn several_conflicts_are_asked_one_after_another() {
            let mut harness = Harness::new();
            let (tab, id) = two_conflicts(&mut harness);
            assert_eq!(asking(&harness), Some((0, 1, 2)));
            answer(&mut harness, Answer::UseServer);
            // The first row is settled at once, whatever the second gets.
            assert_eq!(email(&harness, tab, id, 1), text("eve@example.com"));
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
            assert_eq!(asking(&harness), Some((1, 3, 2)));
            // What it shows is the second row's.
            let line = ("email".to_owned(), Some("user4@example.com".to_owned()));
            assert_eq!(shown(&harness), [line]);
            // An answer given for the row before is not one for this row.
            harness.app.apply(Action::AnswerConflict {
                at: 0,
                answer: Answer::UseServer,
            });
            assert_eq!(asking(&harness), Some((1, 3, 2)));
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
            answer(&mut harness, Answer::KeepMine);
            assert!(harness.app.dialog.is_none());
            assert_eq!(email(&harness, tab, id, 3), text("fay@example.com"));
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
        }

        #[test]
        fn a_conflict_that_cannot_be_asked_about_is_a_line() {
            // Under another dialog: the user is in it, and it stays.
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::ShowHelp);
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            assert!(matches!(harness.app.dialog, Some(Dialog::Help)));
            let line = Some(crate::edit::Note::Conflict {
                row: 1,
                gone: false,
                others: 0,
            });
            assert_eq!(object(&harness, tab, id).edits.note, line);
            assert_eq!(pending(&harness, tab, id), [(1, 1)]);
            assert_eq!(email(&harness, tab, id, 1), text("user2@example.com"));
            // The next save asks.
            harness.app.apply(Action::CloseDialog);
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            assert_eq!(asking(&harness), Some((0, 1, 1)));
            assert_eq!(object(&harness, tab, id).edits.note, None);
            answer(&mut harness, Answer::KeepMine);
            // A row that is not as wide as the page: the table is no longer
            // the one the page was read from, and there is nothing to put
            // the row into.
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![Conflict {
                row: 0,
                server: Some(vec![Value::Int(2)]),
            }])));
            assert!(harness.app.dialog.is_none());
            assert_eq!(object(&harness, tab, id).edits.note, line);
            assert_eq!(pending(&harness, tab, id), [(1, 1)]);
        }

        #[test]
        fn a_page_that_arrives_ends_the_question_about_the_one_it_replaces() {
            let mut harness = Harness::new();
            let (tab, id) = two_conflicts(&mut harness);
            // No fetch runs for a tab that holds edits: one is forged.
            let request = RequestId(u64::MAX);
            let session = harness.app.workspace(tab).unwrap().session;
            harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap()
                .rows
                .pending = Some(request);
            harness.app.apply(Action::Backend(Event::Rows {
                session,
                request,
                result: Ok(page(5, false)),
            }));
            assert!(harness.app.dialog.is_none());
            assert!(pending(&harness, tab, id).is_empty());
        }

        #[test]
        fn closing_the_question_leaves_the_rows_it_did_not_ask_about_as_they_were() {
            let mut harness = Harness::new();
            let (tab, id) = two_conflicts(&mut harness);
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::Overwrite);
            harness.app.apply(Action::CloseDialog);
            assert!(harness.app.dialog.is_none());
            // The answered row is settled, the other is as it was loaded,
            // and no save runs for an answer that was never the last.
            assert_eq!(email(&harness, tab, id, 1), text("eve@example.com"));
            assert_eq!(email(&harness, tab, id, 3), text("user4@example.com"));
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            assert!(write_since(&harness, before).is_none());
            // The tab says which row is still to be asked about, as where
            // the question is not asked at all: the next save asks.
            let line = |row: usize, others: usize| {
                Some(crate::edit::Note::Conflict {
                    row,
                    gone: false,
                    others,
                })
            };
            assert_eq!(object(&harness, tab, id).edits.note, line(3, 0));
            // Closed before any answer: the first row, and one more.
            let mut harness = Harness::new();
            let (tab, id) = two_conflicts(&mut harness);
            harness.app.apply(Action::CloseDialog);
            assert_eq!(object(&harness, tab, id).edits.note, line(1, 1));
            assert_eq!(email(&harness, tab, id, 1), text("user2@example.com"));
            // Any other dialog is closed as before, and leaves no line.
            harness.app.apply(Action::DismissNote { tab, id });
            harness.app.apply(Action::ShowHelp);
            harness.app.apply(Action::CloseDialog);
            assert!(harness.app.dialog.is_none());
            assert_eq!(object(&harness, tab, id).edits.note, None);
        }
```

- [ ] **Step 2: The two tests of step 3 in `src/app.rs`**

In `a_conflict_a_failure_and_a_refusal_keep_the_set_and_say_what_happened`, the conflict is now asked about. Replace

```rust
            // The conflict's row is the set's second, which is the page's row 3.
            assert_eq!(
                edits.note,
                Some(crate::edit::Note::Conflict {
                    row: 3,
                    gone: true,
                    others: 0
                })
            );
```

with

```rust
            // The conflict's row is the set's second, which is the page's
            // row 3. It is asked about, and no line says it too.
            assert_eq!(edits.note, None);
            match &harness.app.dialog {
                Some(Dialog::Conflict(prompt)) => assert_eq!(
                    prompt.rows,
                    [crate::edit::Conflicting {
                        row: 3,
                        server: None
                    }]
                ),
                other => panic!("expected the conflict question, got {other:?}"),
            }
            // Left as it is, the set stays.
            harness.app.apply(Action::AnswerConflict {
                at: 0,
                answer: crate::edit::Answer::KeepMine,
            });
            assert!(harness.app.dialog.is_none());
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 2);
```

In `save_from_the_prompt_runs_the_held_action_only_when_everything_was_written`, the comment and the two assertions after the conflict's answer,

```rust
            // A conflict drops the held action: the tab stays, and nothing
            // asks about it a second time.
```

```rust
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
            assert!(harness.app.dialog.is_none());
```

become

```rust
            // A conflict drops the held action: the tab stays, and once
            // the row is answered nothing asks about closing a second time.
```

```rust
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
            assert!(matches!(harness.app.dialog, Some(Dialog::Conflict(_))));
            harness.app.apply(Action::AnswerConflict {
                at: 0,
                answer: crate::edit::Answer::KeepMine,
            });
            assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
            assert!(harness.app.dialog.is_none());
```

`an_answer_naming_a_row_the_save_did_not_send_marks_no_row` stays as it is: that conflict is still `Note::Lost`.

- [ ] **Step 3: The one test of step 3 in `src/ui/mod.rs`**

`a_conflict_is_a_line_in_the_bar_and_the_set_stays` tests the words of the line on macOS and Windows, where the line now stands only when the question cannot be asked. Add this helper above it:

```rust
    /// Answers the newest save with `conflicts` while another dialog is
    /// up, and closes that dialog. No question is asked under a dialog the
    /// user is in: the conflict is the line it was before there was one.
    fn conflicts_under_a_dialog(harness: &mut Harness, conflicts: Vec<tabletist_db::Conflict>) {
        harness.app.apply(Action::ShowHelp);
        harness.answer_written(Ok(tabletist_db::WriteOutcome::Conflicts(conflicts)));
        harness.app.apply(Action::CloseDialog);
        harness.settle();
    }
```

The test is renamed `a_conflict_under_another_dialog_is_a_line_in_the_bar_and_the_set_stays`. Its `use tabletist_db::{Conflict, WriteOutcome};` becomes `use tabletist_db::Conflict;`, its `harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed])));` becomes `conflicts_under_a_dialog(&mut harness, vec![changed]);`, and its `harness.answer_written(Ok(WriteOutcome::Conflicts(conflicts)));` becomes `conflicts_under_a_dialog(&mut harness, conflicts);`. Every assertion stays.

The two Omarchy tests (`a_written_save_and_a_conflict_are_said_in_the_status_line`, `the_status_line_keeps_a_keys_value_and_the_databases_words_as_they_are`) are not touched in this task.

- [ ] **Step 4: The view's tests**

They are the test module of the file step 8 creates, and are given there with it.

- [ ] **Step 5: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: does not compile (`Dialog::Conflict`, `Action::AnswerConflict`).

- [ ] **Step 6: The model**

In `src/model.rs`, `Action` gains, after `CancelWrite`:

```rust
    /// Answer the question about a row a save found changed. `at` is the
    /// row's place among the save's conflicts as the question showed it: an
    /// answer for another row than the one asked about is dropped.
    AnswerConflict {
        at: usize,
        answer: crate::edit::Answer,
    },
```

`Dialog` gains, after `ConfirmWrite`:

```rust
    /// A save found rows changed on the server: what to do with each.
    Conflict(Box<ConflictPrompt>),
```

and after `WritePrompt`:

```rust
/// Asks what to do with each row a save found changed on the server, one
/// after another. Every answer is applied when it is given.
pub struct ConflictPrompt {
    pub tab: ConnTabId,
    pub id: TabId,
    /// The rows the save found changed, in its order. One that is answered
    /// keeps its place and gives up its row.
    pub rows: Vec<crate::edit::Conflicting>,
    /// The one being asked about.
    pub at: usize,
    /// The columns the user changed in that row, with their values ready
    /// to draw (`edit::shown_lines`).
    pub lines: Vec<crate::edit::ShownLine>,
    /// When that one came on screen: its question takes no answer in its
    /// first moment (`edit::ANSWER_AFTER`).
    pub shown: std::time::Instant,
}

/// Without the rows: they are the database's, and can be megabytes.
impl std::fmt::Debug for ConflictPrompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ConflictPrompt {{ tab: {:?}, id: {:?}, rows: {}, at: {} }}",
            self.tab,
            self.id,
            self.rows.len(),
            self.at
        )
    }
}
```

- [ ] **Step 7: The reducer**

In `src/app.rs`, the first check of `apply` covers the new dialog:

```rust
            Some(Dialog::Leave(_) | Dialog::ConfirmWrite(_) | Dialog::Conflict(_))
```

and after the `Action::ConfirmWrite` arm:

```rust
            Action::AnswerConflict { at, answer } => self.answer_conflict(at, answer),
```

The `Action::CloseDialog` arm, `Action::CloseDialog => self.dialog = None,`, becomes:

```rust
            Action::CloseDialog => {
                // Any dialog goes. The conflict question leaves its line
                // for the rows it did not get an answer for.
                if let Some(Dialog::Conflict(prompt)) = self.dialog.take() {
                    self.conflict_unanswered(&prompt);
                }
            }
```

In the `Event::Rows` arm, take the tab's id where the tab is found, before `let failed`:

```rust
                let id = object.id;
```

and end the arm, after the selection is set:

```rust
                // A question about this tab's rows was about their pending
                // cells, which went with the set just now, and names rows
                // by their place in the page: nothing is left to ask.
                if matches!(
                    &self.dialog,
                    Some(Dialog::Conflict(prompt)) if prompt.tab == tab && prompt.id == id
                ) {
                    self.dialog = None;
                }
```

In `src/app/editing.rs`, the imports gain `Answer`, `conflicting`, `settled` and `shown_lines` from `crate::edit` and `ConflictPrompt` from `crate::model`.

`written` opens the question. At its top, before the tab is looked for:

```rust
        // A dialog the user is in is never replaced: a conflict that arrives
        // under one is a line, as a failure is. So it is in the terminal
        // look, until its box is drawn there.
        let free = self.dialog.is_none() && !self.look.terminal;
```

and its `Conflicts` arm becomes:

```rust
            Ok(WriteOutcome::Conflicts(conflicts)) => {
                // What the tab says where the rows cannot be asked about.
                let line = conflicts.first().map(|first| match place(first.row) {
                    Some(row) => Note::Conflict {
                        row,
                        gone: first.server.is_none(),
                        others: conflicts.len() - 1,
                    },
                    // An answer about a row that was not sent is not one
                    // to tell the save's end by.
                    None => Note::Lost,
                });
                let asked = object.rows.value.as_ref().and_then(|page| {
                    let rows = conflicting(&saving.rows, conflicts, page).filter(|_| free)?;
                    // None when the save named no row: nothing to ask.
                    let lines = shown_lines(page, &object.edits.cells, rows.first()?);
                    Some((rows, lines))
                });
                match asked {
                    // The question says what the line would.
                    Some((rows, lines)) => {
                        self.dialog = Some(Dialog::Conflict(Box::new(ConflictPrompt {
                            tab,
                            id,
                            rows,
                            at: 0,
                            lines,
                            shown: std::time::Instant::now(),
                        })));
                    }
                    None => object.edits.note = line,
                }
            }
```

What was held for the save is already dropped for this outcome (`then` is taken above the match and performed only after `Written`): leave that as it is.

At the end of the `impl App`:

```rust
    /// Answers the question about the row a save found changed, and asks
    /// about the next. `at` is the row the answer was given for: one for
    /// another row than the one asked about is dropped, a click or a key a
    /// frame behind. The answer is applied at once, to the page and to the
    /// set, so each row is whole whatever comes of the rows after it.
    pub(super) fn answer_conflict(&mut self, at: usize, answer: Answer) {
        // The kind is checked before the dialog is taken: another dialog
        // is not closed by it.
        if !matches!(&self.dialog, Some(Dialog::Conflict(prompt)) if prompt.at == at) {
            return;
        }
        let Some(Dialog::Conflict(mut prompt)) = self.dialog.take() else {
            return;
        };
        let (tab, id) = (prompt.tab, prompt.id);
        let Some(conflict) = prompt.rows.get_mut(at) else {
            // Nothing is left to ask about.
            return;
        };
        let (row, gone) = (conflict.row, conflict.server.is_none());
        if !answer.offered(gone) {
            // Not an answer the question about this row has.
            self.dialog = Some(Dialog::Conflict(prompt));
            return;
        }
        // The row is answered once: what the server holds goes into the
        // page, or nowhere.
        let server = conflict.server.take();
        // The cells the answer takes out of the set: every one of the row,
        // or those whose new value is what the server holds now.
        let all = matches!(answer, Answer::UseServer | Answer::Discard);
        let same = match &server {
            Some(server) if !all => self
                .table(tab, id, |table, object| {
                    settled(table, &object.edits.cells, row, server)
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            // The tab went: there is no row to ask about.
            return;
        };
        if let Some(server) = server
            && let Some(loaded) = object
                .rows
                .value
                .as_mut()
                .and_then(|page| page.rows.get_mut(row))
        {
            *loaded = server;
        }
        object
            .edits
            .cells
            .retain(|&(of, col), _| of != row || !(all || same.contains(&col)));
        match answer {
            Answer::Discard => {
                object.edits.gone.insert(row);
            }
            // The row's changes stay pending on a row that is not there.
            // The tab says so, with the line a conflict is where it is not
            // asked about: the row is not left looking like any other.
            Answer::KeepMine if gone => {
                object.edits.note = Some(Note::Conflict {
                    row,
                    gone: true,
                    others: 0,
                });
            }
            Answer::KeepMine | Answer::UseServer | Answer::Overwrite => {}
        }
        // The row panel shows the row as it is now.
        object.fields = None;
        // The next row, or none: every row is answered.
        prompt.at += 1;
        if let Some(next) = prompt.rows.get(prompt.at) {
            // The next row's question is a new one on screen.
            let object = self
                .workspace(tab)
                .and_then(|workspace| workspace.object_tab(id));
            prompt.lines = object
                .and_then(|object| Some(shown_lines(object.page()?, &object.edits.cells, next)))
                .unwrap_or_default();
            prompt.shown = std::time::Instant::now();
            self.dialog = Some(Dialog::Conflict(prompt));
        }
    }

    /// The conflict question was closed without an answer for the row it
    /// showed. That row and the rows after it are as they were loaded, with
    /// their changes pending, and the tab says so as it does where the
    /// question is not asked at all: the next save asks again.
    pub(super) fn conflict_unanswered(&mut self, prompt: &ConflictPrompt) {
        let Some(shown) = prompt.rows.get(prompt.at) else {
            return;
        };
        let note = Note::Conflict {
            row: shown.row,
            gone: shown.server.is_none(),
            others: prompt.rows.len() - prompt.at - 1,
        };
        if let Some(object) = self.object_tab_mut(prompt.tab, prompt.id) {
            object.edits.note = Some(note);
        }
    }
```

- [ ] **Step 8: The stand-in**

In `src/ui/pending_bar.rs`, `fn row_name` becomes `pub(crate) fn row_name`.

In `src/ui/mod.rs`, add `pub mod conflict_prompt;` among the modules (after `complete_tests`) and, in `show`, after `write_prompts::show(app, &ui.ctx().clone());`:

```rust
    conflict_prompt::show(app, &ui.ctx().clone());
```

Create `src/ui/conflict_prompt.rs`. `Title`, `title`, `place` and `answers` stay when tasks 7 and 8 replace the body of `show`; so do the tests, which task 8 opens to every look.

```rust
//! The question about the rows a save found changed on the server: what
//! to do with each, one after another.

use egui::{Id, Key, Modifiers};

use crate::app::App;
use crate::edit::Answer;
use crate::i18n::{Locale, gettext};
use crate::model::{Action, Dialog, ObjectTab, Workspace};
use crate::theme::Look;
use crate::ui::format;
use crate::ui::widgets::{self, ButtonSpec};

/// What the question about a row is headed with, in the parts its line is
/// laid out from: only the row's name gives way when the line is too long.
pub struct Title {
    /// "Row", in the look's case.
    pub before: String,
    /// The row by its key, as the row panel names it: `id 2`. The key is
    /// the database's, and stays as it is.
    pub name: String,
    /// What became of the row, in the look's case.
    pub after: String,
}

impl Title {
    /// The whole of it: what a screen reader and the pointer are told.
    pub fn whole(&self) -> String {
        format!("{} {} {}", self.before, self.name, self.after)
    }
}

/// The heading of the question about the page's row `row`.
pub fn title(object: &ObjectTab, row: usize, gone: bool, look: &Look, locale: Locale) -> Title {
    let say = |text: &'static str| look.label(&gettext(locale, text));
    Title {
        before: say("Row"),
        name: super::pending_bar::row_name(object, row),
        after: if gone {
            say("no longer exists on the server")
        } else {
            say("changed on the server")
        },
    }
}

/// Where the row is: the connection's name and the table's, since a table
/// of one name can be open on two connections and the question can come
/// up over either. With `of` rows found changed by the save, also which of
/// them this is: "· 1 of 2".
pub fn place(
    workspace: &Workspace,
    object: &ObjectTab,
    (at, of): (usize, usize),
    look: &Look,
    locale: Locale,
) -> String {
    let table = format::display_safe(&object.object.name);
    let mut place = format!("{} · {table}", workspace.name);
    if of > 1 {
        let word = look.label(&gettext(locale, "of"));
        place.push_str(&format!(" · {} {word} {of}", at + 1));
    }
    place
}

/// The answers the question about a row offers, by the names of their
/// buttons: one for a row that is gone, three for one that changed.
pub fn answers(gone: bool) -> &'static [(&'static str, Answer)] {
    if gone {
        &[("Discard my changes", Answer::Discard)]
    } else {
        &[
            ("Keep mine, reload row", Answer::KeepMine),
            ("Use server values", Answer::UseServer),
            ("Overwrite", Answer::Overwrite),
        ]
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(Dialog::Conflict(prompt)) = &app.dialog else {
        return;
    };
    let at = prompt.at;
    let shown = app.workspace(prompt.tab).and_then(|workspace| {
        let object = workspace.object_tab(prompt.id)?;
        Some((workspace, object, prompt.rows.get(at)?))
    });
    let Some((workspace, object, conflict)) = shown else {
        // Nothing to draw, so nothing to answer it with: it closes, or it
        // would keep the keyboard for good.
        app.actions.push(Action::CloseDialog);
        return;
    };
    let gone = conflict.server.is_none();
    let title = title(object, conflict.row, gone, &look, locale).whole();
    let place = place(workspace, object, (at, prompt.rows.len()), &look, locale);
    // No answer in the question's first moment: what was on its way to
    // the grid when it came up is not one. It is dropped without a sign.
    let ripe = crate::edit::answers_taken(prompt.shown);
    let mut actions = Vec::new();
    let modal = widgets::modal(Id::new("conflict-prompt"), &look, &palette).show(ctx, |ui| {
        let role = widgets::dialog_title(&look);
        widgets::label(ui, role, &title, palette.text, &look);
        ui.add_space(4.0);
        widgets::label(ui, widgets::body(&look), &place, palette.secondary, &look);
        ui.add_space(14.0);
        for (name, answer) in answers(gone) {
            let name = gettext(locale, name);
            let button = ButtonSpec::new(&name);
            if button.show(ui, 30.0, &look, &palette).clicked() {
                let answer = *answer;
                actions.push(Action::AnswerConflict { at, answer });
            }
        }
    });
    // Esc is Keep mine for the row shown: nothing of the user's is dropped
    // and nothing is written.
    let escape = |input: &mut egui::InputState| input.consume_key(Modifiers::NONE, Key::Escape);
    if modal.is_top_modal && ctx.input_mut(escape) {
        let answer = Answer::KeepMine;
        actions.push(Action::AnswerConflict { at, answer });
    }
    if ripe {
        app.actions.extend(actions);
    }
}

#[cfg(test)]
mod tests {
    use tabletist_db::{Conflict, Value, WriteOutcome};

    use crate::backend::Command;
    use crate::model::{Action, CellPos, ConnTabId, Dialog, EditStart, TabId};
    use crate::testing::Harness;
    use crate::theme::Look;
    use crate::ui::tests::click_dialog;

    /// The looks that ask the question. The terminal look keeps the line
    /// until its box is drawn.
    fn looks() -> impl Iterator<Item = Look> {
        Look::ALL.into_iter().filter(|look| !look.terminal)
    }

    /// Makes `text` the pending email of the row `id 2` of the table `id`.
    fn retype(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: CellPos { row: 1, col: 1 },
            start: EditStart::Replace(text.into()),
        });
        harness.app.apply(Action::LeaveEdit { tab, id });
    }

    /// The fixture's row `id 2` as the server holds it with `email`.
    fn server(email: &str) -> Vec<Value> {
        vec![Value::Int(2), Value::Text(email.into()), Value::Null]
    }

    /// The fixture's table in `look`, the email of its row `id 2` pending
    /// as `bob@example.com` and saved, and the save answered: the row
    /// holds `email` now, or is gone. The question is up, and has been for
    /// long enough to take an answer.
    fn conflict_in(look: Look, email: Option<&str>) -> (Harness, ConnTabId, TabId) {
        let mut harness = Harness::new();
        harness.set_look(look);
        let (tab, id) = harness.editable();
        retype(&mut harness, tab, id, "bob@example.com");
        harness.app.apply(Action::WriteEdits { tab, id });
        let conflict = Conflict {
            row: 0,
            server: email.map(server),
        };
        harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
        harness.finish_animations();
        assert!(asking(&harness), "{}", look.name);
        shown(&mut harness, true);
        (harness, tab, id)
    }

    /// Makes the question look as if it had been on screen for a while
    /// (`long`), or as if it had only just come up, however long the test
    /// has taken.
    fn shown(harness: &mut Harness, long: bool) {
        let now = std::time::Instant::now();
        let hour = std::time::Duration::from_secs(3600);
        if let Some(Dialog::Conflict(prompt)) = &mut harness.app.dialog {
            prompt.shown = if long {
                now.checked_sub(crate::edit::ANSWER_AFTER)
                    .expect("an earlier instant")
            } else {
                now + hour
            };
        }
    }

    fn asking(harness: &Harness) -> bool {
        matches!(harness.app.dialog, Some(Dialog::Conflict(_)))
    }

    fn writes(harness: &Harness) -> usize {
        let sent = harness.app.backend.sent.iter();
        sent.filter(|command| matches!(command, Command::Write { .. }))
            .count()
    }

    /// How many cells are pending, and the email the page holds for the
    /// row `id 2`.
    fn state(harness: &Harness, tab: ConnTabId, id: TabId) -> (usize, Value) {
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(id).unwrap();
        let email = object.page().unwrap().rows[1][1].clone();
        (object.edits.cells.len(), email)
    }

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    fn painted(harness: &Harness, text: &str) -> bool {
        harness.painted.iter().any(|(piece, _)| piece == text)
    }

    #[test]
    fn each_answer_is_a_button_and_escape_keeps_mine() {
        for look in looks() {
            let said = look.name;
            let eve = || text("eve@example.com");
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            click_dialog(&mut harness, "Use server values");
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (0, eve()), "{said}");
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            click_dialog(&mut harness, "Keep mine, reload row");
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, eve()), "{said}");
            assert_eq!(writes(&harness), 1, "{said}: nothing is saved again");
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            click_dialog(&mut harness, "Overwrite");
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, eve()), "{said}");
            // Esc is Keep mine.
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, eve()), "{said}");
            assert_eq!(writes(&harness), 1, "{said}");
            // A row that is gone has one answer. Esc leaves its change
            // pending, and the bar says that the row is not there (the
            // terminal look's status line, in its own words).
            let loaded = || text("user2@example.com");
            let (mut harness, tab, id) = conflict_in(look, None);
            assert!(!harness.has("Overwrite"), "{said}");
            harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, loaded()), "{said}");
            let line = if look.terminal {
                "conflict row id 2 no longer exists on the server. nothing was written."
            } else {
                "Row id 2 no longer exists on the server. Nothing was written."
            };
            assert!(harness.has(line), "{said}");
            let (mut harness, tab, id) = conflict_in(look, None);
            click_dialog(&mut harness, "Discard my changes");
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (0, loaded()), "{said}");
            let workspace = harness.app.workspace(tab).unwrap();
            let edits = &workspace.object_tab(id).unwrap().edits;
            assert!(edits.gone.contains(&1) && edits.note.is_none(), "{said}");
        }
    }

    #[test]
    fn an_answer_in_the_questions_first_moment_is_not_taken() {
        for look in looks() {
            let said = look.name;
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            shown(&mut harness, false);
            click_dialog(&mut harness, "Use server values");
            harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            assert!(asking(&harness), "{said}");
            let loaded = text("user2@example.com");
            assert_eq!(state(&harness, tab, id), (1, loaded), "{said}");
            // Once it has been on screen for a moment, it is.
            shown(&mut harness, true);
            click_dialog(&mut harness, "Use server values");
            assert!(!asking(&harness), "{said}");
        }
    }

    #[test]
    fn the_question_is_headed_with_the_row_by_its_key() {
        let (harness, tab, id) = conflict_in(Look::standard(), Some("eve@example.com"));
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(id).unwrap();
        let locale = harness.app.locale;
        let title = |gone: bool, look: Look| super::title(object, 1, gone, &look, locale);
        let changed = title(false, Look::standard());
        assert_eq!(changed.whole(), "Row id 2 changed on the server");
        // The name is a part of its own: it is what gives way in a line
        // that is too long.
        assert_eq!(changed.name, "id 2");
        assert_eq!(
            title(true, Look::standard()).whole(),
            "Row id 2 no longer exists on the server"
        );
        // The terminal's lower case is for the app's own words.
        assert_eq!(
            title(false, Look::omarchy()).whole(),
            "row id 2 changed on the server"
        );
        assert!(
            painted(&harness, "Row id 2 changed on the server"),
            "{:?}",
            harness.painted
        );
    }

    #[test]
    fn the_question_says_which_connection_and_table_its_row_is_of() {
        for look in looks() {
            let said = look.name;
            // Two connections, each with a table `users` open.
            let mut harness = Harness::new();
            harness.set_look(look);
            let (first, users) = harness.editable();
            harness.app.apply(Action::ShowConnections);
            let (second, others) = harness.editable();
            harness.app.workspace_mut(second).unwrap().name = "Bookshop".into();
            // A save of the first is answered while the second shows.
            retype(&mut harness, first, users, "bob@example.com");
            harness.app.apply(Action::WriteEdits {
                tab: first,
                id: users,
            });
            let changed = |row: usize, email: &str| Conflict {
                row,
                server: Some(server(email)),
            };
            let conflict = changed(0, "eve@example.com");
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
            harness.finish_animations();
            assert!(painted(&harness, "Fixture · users"), "{said}");
            assert!(!painted(&harness, "Bookshop · users"), "{said}");
            shown(&mut harness, true);
            click_dialog(&mut harness, "Use server values");
            // The same row of the other connection's table.
            retype(&mut harness, second, others, "bob@example.com");
            harness.app.apply(Action::WriteEdits {
                tab: second,
                id: others,
            });
            let conflict = changed(0, "eve@example.com");
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
            harness.finish_animations();
            assert!(painted(&harness, "Bookshop · users"), "{said}");
            assert!(!painted(&harness, "Fixture · users"), "{said}");
        }
        // With several rows, which of them: in the look's case.
        let (harness, tab, id) = conflict_in(Look::standard(), Some("eve@example.com"));
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(id).unwrap();
        let locale = harness.app.locale;
        let place =
            |at: (usize, usize)| super::place(workspace, object, at, &Look::standard(), locale);
        assert_eq!(place((0, 1)), "Fixture · users");
        assert_eq!(place((0, 2)), "Fixture · users · 1 of 2");
        assert_eq!(place((1, 2)), "Fixture · users · 2 of 2");
    }
}
```

- [ ] **Step 9: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: all pass: the nine new reducer tests, the four of the view, and the three changed tests of step 3.

- [ ] **Step 10: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A && git commit -m "Ask about each row a save found changed"
```

In the commit's body, say that three tests of step 3 changed and why: on macOS and Windows a conflict is asked about now, and is a line only where it cannot be.

---

### Task 4: The save runs again

**Files:**
- Modify: `src/model.rs` (`ConflictPrompt`), `src/app/editing.rs` (`written`, `answer_conflict`)
- Test: `src/app.rs`, `src/ui/conflict_prompt.rs`

- [ ] **Step 1: Write the failing tests**

At the end of the nested `mod editing` in `src/app.rs`'s tests:

```rust
        #[test]
        fn overwrite_saves_again_with_the_servers_row_as_the_loaded_one() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::Overwrite);
            assert!(harness.app.dialog.is_none());
            // The save is sent at once, and checks against what the server
            // held when it was asked.
            let changes = write_since(&harness, before).expect("a second Write");
            assert_eq!(changes.rows.len(), 1);
            assert_eq!(changes.rows[0].set[0].loaded, text("eve@example.com"));
            assert_eq!(
                changes.rows[0].set[0].new,
                NewValue::Text("bob@example.com".into())
            );
            assert!(object(&harness, tab, id).edits.saving.is_some());
            harness.answer_written(written("bob@example.com"));
            assert!(!object(&harness, tab, id).edits.holds());
            assert_eq!(email(&harness, tab, id, 1), text("bob@example.com"));
        }

        #[test]
        fn the_save_runs_again_only_when_a_row_was_overwritten_and_none_was_kept() {
            use Answer::{KeepMine, Overwrite, UseServer};
            // The answers to the page's rows 1 and 3 (`id` 2 and 4), the
            // keys of the rows the second save sends (none: no save), and
            // the cells left pending when no save runs.
            type Case = (Answer, Answer, Option<Vec<i64>>, Vec<(usize, usize)>);
            let cases: Vec<Case> = vec![
                (Overwrite, Overwrite, Some(vec![2, 4]), vec![]),
                (Overwrite, UseServer, Some(vec![2]), vec![]),
                (UseServer, Overwrite, Some(vec![4]), vec![]),
                (Overwrite, KeepMine, None, vec![(1, 1), (3, 1)]),
                (KeepMine, Overwrite, None, vec![(1, 1), (3, 1)]),
                (KeepMine, KeepMine, None, vec![(1, 1), (3, 1)]),
                (KeepMine, UseServer, None, vec![(1, 1)]),
                (UseServer, KeepMine, None, vec![(3, 1)]),
                (UseServer, UseServer, None, vec![]),
            ];
            for (first, second, saved, left) in cases {
                let said = format!("{first:?}, then {second:?}");
                let mut harness = Harness::new();
                let (tab, id) = two_conflicts(&mut harness);
                let before = harness.app.backend.sent.len();
                answer(&mut harness, first);
                // Nothing is sent before the last answer.
                assert!(write_since(&harness, before).is_none(), "{said}");
                answer(&mut harness, second);
                assert!(harness.app.dialog.is_none(), "{said}");
                let keys = write_since(&harness, before).map(|changes| {
                    let key = |row: &tabletist_db::RowChange| row.key[0].1.clone();
                    changes.rows.iter().map(key).collect::<Vec<_>>()
                });
                let expected = saved.map(|ids| ids.into_iter().map(Value::Int).collect::<Vec<_>>());
                assert_eq!(keys, expected, "{said}");
                let edits = &object(&harness, tab, id).edits;
                assert_eq!(edits.saving.is_some(), keys.is_some(), "{said}");
                if keys.is_none() {
                    assert_eq!(pending(&harness, tab, id), left, "{said}");
                    // With nothing left, the tab holds its page no longer.
                    assert_eq!(edits.holds(), !left.is_empty(), "{said}");
                }
                // Whatever the answers, both rows are the server's now.
                let server = |row: usize| email(&harness, tab, id, row);
                assert_eq!(server(1), text("eve@example.com"), "{said}");
                assert_eq!(server(3), text("fay@example.com"), "{said}");
            }
        }

        #[test]
        fn a_discarded_row_neither_asks_for_a_save_nor_stops_one() {
            // A gone row beside an overwritten one: the save runs, without it.
            let mut harness = Harness::new();
            let both = vec![gone(0), changed(1, 4, "fay@example.com")];
            let (tab, id) = conflicts(&mut harness, both);
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::Discard);
            assert_eq!(asking(&harness), Some((1, 3, 2)));
            answer(&mut harness, Answer::Overwrite);
            let changes = write_since(&harness, before).expect("a second Write");
            assert_eq!(changes.rows.len(), 1);
            assert_eq!(changes.rows[0].key, [("id".to_owned(), Value::Int(4))]);
            assert!(object(&harness, tab, id).edits.gone.contains(&1));
            // A gone row beside one that did not conflict: no save runs.
            let mut harness = Harness::new();
            let (tab, id) = conflicts(&mut harness, vec![gone(0)]);
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::Discard);
            assert!(write_since(&harness, before).is_none());
            assert_eq!(pending(&harness, tab, id), [(3, 1)]);
        }

        #[test]
        fn a_row_that_is_gone_and_kept_stops_the_save_as_any_kept_row_does() {
            let mut harness = Harness::new();
            let both = vec![gone(0), changed(1, 4, "fay@example.com")];
            let (tab, id) = conflicts(&mut harness, both);
            let before = harness.app.backend.sent.len();
            // Esc on the row that is gone, then Overwrite on the other.
            answer(&mut harness, Answer::KeepMine);
            answer(&mut harness, Answer::Overwrite);
            assert!(harness.app.dialog.is_none());
            // The save would write the row that is not there: it waits.
            assert!(write_since(&harness, before).is_none());
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            assert_eq!(email(&harness, tab, id, 3), text("fay@example.com"));
            assert_eq!(
                object(&harness, tab, id).edits.note,
                Some(crate::edit::Note::Conflict {
                    row: 1,
                    gone: true,
                    others: 0
                })
            );
        }

        #[test]
        fn an_overwrite_that_leaves_nothing_pending_saves_nothing() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "eve@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            // Someone saved the very value typed here.
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![Conflict {
                row: 0,
                server: Some(vec![Value::Int(2), text("eve@example.com"), text("{}")]),
            }])));
            // (The conflict is real: the save compares the column with what
            // was loaded, not with what is new.)
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::Overwrite);
            assert!(harness.app.dialog.is_none());
            assert!(write_since(&harness, before).is_none());
            let now = object(&harness, tab, id);
            assert!(!now.edits.holds() && now.edits.note.is_none());
            assert_eq!(email(&harness, tab, id, 1), text("eve@example.com"));
            // The same with the session gone: there was nothing to send,
            // so nothing says that nothing was sent.
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "eve@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            let session = harness.app.workspace(tab).unwrap().session;
            harness.app.apply(Action::Backend(Event::Disconnected {
                session,
                error: tabletist_db::Error::ConnectionLost("gone".into()),
            }));
            answer(&mut harness, Answer::Overwrite);
            let now = object(&harness, tab, id);
            assert!(!now.edits.holds() && now.edits.note.is_none());
        }

        #[test]
        fn a_row_that_changed_once_more_conflicts_once_more() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            answer(&mut harness, Answer::Overwrite);
            // Changed again between the question and the second save.
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "zoe@example.com",
            )])));
            assert_eq!(asking(&harness), Some((0, 1, 1)));
            // What it shows as loaded is the row of the first answer.
            assert_eq!(email(&harness, tab, id, 1), text("eve@example.com"));
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::KeepMine);
            assert_eq!(email(&harness, tab, id, 1), text("zoe@example.com"));
            assert_eq!(pending(&harness, tab, id), [(1, 1)]);
            assert!(write_since(&harness, before).is_none());
        }

        #[test]
        fn on_production_the_save_after_an_overwrite_is_confirmed_again() {
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::ConfirmWrite);
            assert_eq!(writes(&harness), 1);
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            answer(&mut harness, Answer::Overwrite);
            // Asked again, with the statements of the save as it is now.
            assert_eq!(writes(&harness), 1);
            let shown = confirming(&harness).expect("the confirmation");
            assert_eq!(shown.rows[0].set[0].loaded, text("eve@example.com"));
            // Cancelled: nothing is sent and the cell stays pending.
            harness.app.apply(Action::CancelWrite);
            assert_eq!(writes(&harness), 1);
            assert_eq!(pending(&harness, tab, id), [(1, 1)]);
            // Confirmed: it is sent.
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.app.apply(Action::ConfirmWrite);
            assert_eq!(writes(&harness), 2);
        }

        #[test]
        fn what_a_save_was_to_be_followed_by_is_dropped_by_its_conflict_for_good() {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            answer(&mut harness, Answer::Overwrite);
            assert_eq!(writes(&harness), 2);
            harness.answer_written(written("bob@example.com"));
            // Everything is written now, and the tab is still open: the
            // close was dropped with the first save.
            assert!(harness.app.dialog.is_none());
            let now = object(&harness, tab, id);
            assert!(!now.edits.holds());
            assert_eq!(email(&harness, tab, id, 1), text("bob@example.com"));
        }

        #[test]
        fn an_overwrite_after_the_session_went_sends_nothing_and_says_so() {
            let mut harness = Harness::new();
            let (tab, id) = conflicts(&mut harness, vec![changed(0, 2, "eve@example.com")]);
            let session = harness.app.workspace(tab).unwrap().session;
            harness.app.apply(Action::Backend(Event::Disconnected {
                session,
                error: tabletist_db::Error::ConnectionLost("gone".into()),
            }));
            // The question stays, and its answer still settles the row.
            assert_eq!(asking(&harness), Some((0, 1, 1)));
            let before = harness.app.backend.sent.len();
            answer(&mut harness, Answer::Overwrite);
            assert!(harness.app.dialog.is_none());
            assert!(write_since(&harness, before).is_none());
            assert_eq!(email(&harness, tab, id, 1), text("eve@example.com"));
            assert_eq!(pending(&harness, tab, id), [(1, 1), (3, 1)]);
            assert_eq!(
                object(&harness, tab, id).edits.note,
                Some(crate::edit::Note::NotSent)
            );
        }
```

In `src/ui/conflict_prompt.rs`, in `each_answer_is_a_button_and_escape_keeps_mine`, after the three lines that follow `click_dialog(&mut harness, "Overwrite");` (the question is gone, and the state is one pending cell on the server's row), add:

```rust
            assert_eq!(writes(&harness), 2, "{said}: the save runs again");
```

- [ ] **Step 2: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: eight tests fail, each where it expects a second `Write` or its line: `overwrite_saves_again_with_the_servers_row_as_the_loaded_one`, `the_save_runs_again_only_when_a_row_was_overwritten_and_none_was_kept`, `a_discarded_row_neither_asks_for_a_save_nor_stops_one`, `a_row_that_changed_once_more_conflicts_once_more`, `on_production_the_save_after_an_overwrite_is_confirmed_again`, `what_a_save_was_to_be_followed_by_is_dropped_by_its_conflict_for_good`, `an_overwrite_after_the_session_went_sends_nothing_and_says_so`, and the view's test at its new line. Two pass already, since nothing saves yet, and pin the other side of the rule once it does: `a_row_that_is_gone_and_kept_stops_the_save_as_any_kept_row_does` (it fails when a kept row that is gone does not count as kept) and `an_overwrite_that_leaves_nothing_pending_saves_nothing` (its second half fails when the save is tried with nothing pending, by the line "Not connected. Nothing was sent.").

- [ ] **Step 3: The model**

`ConflictPrompt` (`src/model.rs`) gains two fields, after `shown`:

```rust
    /// A row was answered Overwrite: the save may run again.
    pub(crate) overwrite: bool,
    /// A row was answered Keep mine: no save runs again by itself.
    pub(crate) kept: bool,
```

and `written` (`src/app/editing.rs`) sets both to `false` where it makes the prompt, after `shown:`.

- [ ] **Step 4: The rule**

In `answer_conflict`, everything from the comment "The next row, or none: every row is answered." to the end of the function becomes:

```rust
        match answer {
            Answer::KeepMine => prompt.kept = true,
            Answer::Overwrite => prompt.overwrite = true,
            Answer::UseServer | Answer::Discard => {}
        }
        prompt.at += 1;
        if let Some(next) = prompt.rows.get(prompt.at) {
            // The next row's question is a new one on screen.
            let object = self
                .workspace(tab)
                .and_then(|workspace| workspace.object_tab(id));
            prompt.lines = object
                .and_then(|object| Some(shown_lines(object.page()?, &object.edits.cells, next)))
                .unwrap_or_default();
            prompt.shown = std::time::Instant::now();
            self.dialog = Some(Dialog::Conflict(prompt));
            return;
        }
        // Every row is answered. A save writes the whole set, so it runs
        // again only when a row was to be overwritten and none was kept to
        // look at again: that one would be written with it.
        if !prompt.overwrite || prompt.kept {
            return;
        }
        let pending = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .is_some_and(|object| !object.edits.cells.is_empty());
        if !pending {
            return;
        }
        // The session went while the question was up: the tab says that
        // nothing went out, as it does after a confirmation.
        if self.save_blocked(tab, id) == Some(SaveBlock::Disconnected) {
            if let Some(object) = self.object_tab_mut(tab, id) {
                object.edits.note = Some(Note::NotSent);
            }
            return;
        }
        // As any save: checked, and on production confirmed again with the
        // statements it would send now. What the first save was to be
        // followed by went with its conflict.
        self.write_edits(tab, id, None);
    }
```

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: all pass.

- [ ] **Step 6: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A && git commit -m "Save again after the last answer when a row was to be overwritten"
```

---

### Task 5: The confirmation's first moment

**Files:**
- Modify: `src/model.rs` (`WritePrompt`), `src/app/editing.rs` (`write_edits`, `write_as_answer`, `answer_conflict`), `src/app.rs` (the `LeaveSave` arm), `src/ui/write_prompts.rs` (`confirm_write`)
- Test: `src/app.rs`, `src/ui/write_prompts.rs`

On production the answer to one dialog can bring up another in its place: Save in the Leave prompt and Overwrite in the conflict question both lead to "Save to production". That confirmation then takes no answer in its first 500 ms, as the conflict question does (decision 17). Asked for by Save itself it behaves as today.

- [ ] **Step 1: Write the failing tests**

At the end of the nested `mod editing` in `src/app.rs`'s tests:

```rust
        /// Whether the confirmation that is up was opened by the answer to
        /// another dialog. `None` when no confirmation is up.
        fn after_an_answer(harness: &Harness) -> Option<bool> {
            match &harness.app.dialog {
                Some(Dialog::ConfirmWrite(prompt)) => Some(prompt.after_answer.is_some()),
                _ => None,
            }
        }

        #[test]
        fn a_confirmation_knows_whether_the_answer_to_another_dialog_opened_it() {
            // Asked for by Save itself, from a key, the bar or the prompt's
            // `:w`: it answers at once, as before.
            let mut harness = Harness::new();
            let (tab, id) = production(&mut harness);
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::WriteEdits { tab, id });
            assert_eq!(after_an_answer(&harness), Some(false));
            harness.app.apply(Action::CancelWrite);
            harness.app.workspace_mut(tab).unwrap().command = Some("w".into());
            harness.app.apply(Action::RunCommand(tab));
            assert_eq!(after_an_answer(&harness), Some(false));
            harness.app.apply(Action::CancelWrite);
            // Save in the Leave prompt: the confirmation takes that
            // prompt's place.
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            assert_eq!(after_an_answer(&harness), Some(true));
            // Overwrite in the conflict question: the same.
            harness.app.apply(Action::ConfirmWrite);
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed(
                0,
                2,
                "eve@example.com",
            )])));
            answer(&mut harness, Answer::Overwrite);
            assert_eq!(after_an_answer(&harness), Some(true));
            // Asked for again by Save, it is an ordinary one again.
            harness.app.apply(Action::CancelWrite);
            harness.app.apply(Action::WriteEdits { tab, id });
            assert_eq!(after_an_answer(&harness), Some(false));
            // Where no confirmation is needed, the answer opens none.
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            assert_eq!(after_an_answer(&harness), None);
            assert_eq!(writes(&harness), 1);
        }
```

At the end of the test module of `src/ui/write_prompts.rs`:

```rust
    /// Makes the confirmation look as if the answer that opened it was
    /// given a while ago (`long`), or only just, however long the test has
    /// taken.
    fn opened(harness: &mut Harness, long: bool) {
        let now = std::time::Instant::now();
        let Some(Dialog::ConfirmWrite(prompt)) = &mut harness.app.dialog else {
            panic!("expected the confirmation, got {:?}", harness.app.dialog);
        };
        assert!(prompt.after_answer.is_some(), "an answer opened it");
        prompt.after_answer = Some(if long {
            now.checked_sub(crate::edit::ANSWER_AFTER)
                .expect("an earlier instant")
        } else {
            now + std::time::Duration::from_secs(3600)
        });
    }

    #[test]
    fn a_confirmation_an_answer_opened_takes_no_answer_in_its_first_moment() {
        for look in crate::theme::Look::ALL {
            let said = look.name;
            let mut harness = Harness::new();
            harness.set_look(look);
            let (tab, id) = harness.editable();
            harness.app.workspace_mut(tab).unwrap().environment =
                crate::env::Environment::Production;
            change(&mut harness, (tab, id), 1, 1, "bob@example.com");
            // Save in the Leave prompt brings the confirmation up in that
            // prompt's place.
            harness.app.apply(Action::CloseTab { tab, id });
            harness.app.apply(Action::LeaveSave);
            harness.finish_animations();
            opened(&mut harness, false);
            let up =
                |harness: &Harness| matches!(harness.app.dialog, Some(Dialog::ConfirmWrite(_)));
            if look.terminal {
                // The field has the keyboard, and what is typed into it in
                // that moment stays typed. Enter and Esc are no answers.
                harness.frame(vec![egui::Event::Text("write".into())]);
                harness.press(egui::Key::Enter, egui::Modifiers::NONE);
                harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            } else {
                click_dialog(&mut harness, "Save to production");
                click_dialog(&mut harness, "Cancel");
                harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            }
            assert!(up(&harness), "{said}");
            assert_eq!(
                (writes(&harness), pending(&harness, (tab, id))),
                (0, 1),
                "{said}"
            );
            // After it, the same answer is taken.
            opened(&mut harness, true);
            if look.terminal {
                harness.press(egui::Key::Enter, egui::Modifiers::NONE);
            } else {
                click_dialog(&mut harness, "Save to production");
            }
            assert!(harness.app.dialog.is_none(), "{said}");
            assert_eq!(writes(&harness), 1, "{said}");
        }
    }
```

- [ ] **Step 2: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: does not compile (no field `after_answer` on `WritePrompt`).

- [ ] **Step 3: The model**

`WritePrompt` (`src/model.rs`) gains a field, after `then`:

```rust
    /// When the answer to another dialog brought the confirmation up (Save
    /// in the Leave prompt, Overwrite in the conflict question): it came up
    /// under the hand that gave that answer, and takes none in its first
    /// moment (`edit::ANSWER_AFTER`). `None` when Save itself asked for it.
    pub after_answer: Option<std::time::Instant>,
```

Its `Debug` names its fields one by one and needs nothing.

- [ ] **Step 4: The reducer**

In `write_edits` (`src/app/editing.rs`), the `WritePrompt` is made with `after_answer: None,` after `then,`.

After `answer_conflict`:

```rust
    /// Saves as the answer to another dialog asks for it: Save in the Leave
    /// prompt, Overwrite in the conflict question. The caller has taken that
    /// dialog away. Where the save is confirmed first, the confirmation
    /// comes up in the place of the dialog that was answered, under the
    /// hand that answered it, so it takes no answer in its first moment.
    pub(super) fn write_as_answer(&mut self, tab: ConnTabId, id: TabId, then: Option<Held>) {
        self.write_edits(tab, id, then);
        if let Some(Dialog::ConfirmWrite(prompt)) = &mut self.dialog {
            prompt.after_answer = Some(std::time::Instant::now());
        }
    }
```

Its two callers are the two answers that can open a confirmation. Both have taken their own dialog away before they save, so a confirmation that is up afterwards is the one this save opened:

- `answer_conflict`: its last line, `self.write_edits(tab, id, None);`, becomes `self.write_as_answer(tab, id, None);`.
- `src/app.rs`, the `Action::LeaveSave` arm: `self.write_edits(*tab, *id, Some(held));` becomes `self.write_as_answer(*tab, *id, Some(held));`.

`Action::WriteEdits` and `run_command` keep calling `write_edits`.

- [ ] **Step 5: The view**

In `confirm_write` (`src/ui/write_prompts.rs`), after the `let Some(workspace) = app.workspace(prompt.tab) else { .. };` that cancels a confirmation with no connection to draw:

```rust
    // Brought up by the answer to another dialog, it takes no answer in
    // its first moment: the click or the key that gave that answer, given
    // twice, is none to this question. What is typed into the terminal's
    // field in that moment stays typed.
    let ripe = prompt.after_answer.is_none_or(crate::edit::answers_taken);
```

and its last line, `app.actions.extend(actions);`, becomes:

```rust
    if ripe {
        app.actions.extend(actions);
    }
```

Nothing else of the two forms changes: the buttons are drawn as they are, and a click, Enter or Esc in that moment is dropped with no sign.

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: all pass. The tests of step 3 that confirm at once (`the_production_confirmation_lists_every_statement_and_sends_only_when_confirmed` in this file, `a_save_to_production_shows_its_statements_and_is_confirmed` in `src/ui/mod.rs`) pass untouched: they open the confirmation by Save itself.

- [ ] **Step 7: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A && git commit -m "Give a confirmation that an answer opened a moment before it takes one"
```

**The model of the conflict question is whole here.** On macOS and Windows the stand-in asks it, without the values. The tasks that follow draw it.

---

## The drawing tasks

From here the plan gives each task its behaviour, where it hooks into the code, the shapes it adds and the tests that pin it, not every line: the drawing has to be written against the functions as they are. Read the named functions first. Every task is test first (a headless test that fails for the stated reason), ends green on the checks, and is committed alone.

The tests of the question live in the test module of `src/ui/conflict_prompt.rs`, which task 3 began: `looks()` gives the looks that ask, `conflict_in(look, email)` opens the question on the fixture's row `id 2` with its answers ready to be taken, `shown(&mut harness, long)` back-dates or pins the moment it came up (there is no injected clock: every test that answers through the view after the question moved on to its next row calls `shown(&mut harness, true)` first), and `asking`, `writes`, `state`, `text` and `painted` read what came of it. `conflict_in` conflicts the email of row 1 and nothing else: a test that needs other columns, other values or several rows sets its own page and set up, as `the_question_says_which_connection_and_table_its_row_is_of` does. Helpers of `src/ui/mod.rs`'s test module that these tests need (`type_key`, `focus_dialog`, `focused_name`, `make_pending`) are private there: make the ones you use `pub(super)`, as `pressable` and `click_dialog` are, and say which in the report. `Harness::press` sends key events and no text; a letter the box reads as typed text needs both in one frame: `type_key`.

The grid behind the question paints the same values (a pending cell's text, and in the terminal look in the same warning colour). To find the question's own piece of a text, take the one whose place in `harness.text_rects` lies inside the modal, and look for its tint in `harness.fills` at that place; an assertion on the colour of the first text of that wording cannot fail.

### Task 6: A gone row in the grid

**Files:**
- Modify: `src/ui/grid.rs` (`Mark::Gone`), `src/ui/data_view.rs` (`Changes`)
- Test: `src/ui/grid.rs`, `src/ui/mod.rs`

**What it adds**

- `grid::Mark` gains `Gone` ("A cell of a row a save found gone from the server"). In the cell loop of `grid::show`: the match that gives a mark its tone (752) is exhaustive, and `Gone` goes with `None` and `Locked` (no tint); in the match that picks the text's colours (`written`, 829), a cell whose mark is `Gone` is written with `written_in(palette, palette.dim)`, in every look. No bar, no gutter sign, no line. The terminal's cursor on such a cell is the reverse video every unmarked cell gets.
- `data_view::Changes` gains `gone: &'a BTreeSet<usize>`, which `Changes::of` takes as one more argument (`&object.edits.gone` at its one call site, 1257). In `Changes::mark`, after the note of a cell that was asked for is set and before the pending cell is looked up: a cell whose row is in `gone` gets `Mark::Gone` and nothing else. It gets no hint: why a cell is locked is said only when it is asked for, and that already works for every `Lock` (the note at the cell on macOS and Windows, the status line on Omarchy).
- Nothing else of the row changes: it is selected, copied and shown in the row panel as loaded.

**Tests**

- `grid.rs`, in `a_marked_cell_is_tinted_and_its_row_is_marked` (1923), beside the check of `Mark::Locked`: with `Mark::Gone` and `RowMark::None`, the cell's text is painted in `palette.dim` in every look, and no fill of `Tone::Warning`, `Tone::Danger` or `Tone::Success` is behind it.
- `ui/mod.rs`, `a_row_that_is_gone_is_written_dim_and_says_why_when_asked` (every look): on `editable_in(look)`, mark row 1 gone by hand (`edits.gone.insert(1)` on the tab), settle; `user2@example.com` is painted in `palette.dim` and `user3@example.com` in the text colour it has without the mark; select `(1, 1)` and ask (Enter on macOS and Windows, `type_key(Key::I, "i")` on Omarchy): no editor opens, and "This row no longer exists on the server" is on screen (Omarchy: the same words in the status line, lower-cased).
- A SQL result is drawn as before: the tests of `sql_results.rs` pass untouched (run the file's tests).

Commit: "Draw a row that is gone from the server".

---

### Task 7: The sheet, on macOS and Windows

**Files:**
- Modify: `src/ui/conflict_prompt.rs`, `src/ui/write_prompts.rs` (four items become `pub(super)`), `src/ui/widgets.rs` (`ButtonSpec::link`)
- Test: `src/ui/conflict_prompt.rs`

`show` keeps what the stand-in does before and after the drawing: it reads the prompt, finds the connection and the tab (and closes the dialog with `Action::CloseDialog` when they are not there), makes the title and the place line, works out `ripe`, collects the answers of the frame, reads Esc as Keep mine when the modal is the one on top, and passes the answers on only when `ripe`. What it draws is now `sheet`.

**What the sheet is given.** The title's three parts (`title(..)`), the place line (`place(..)`), the sentence, whether the row is gone, and the prompt's lines (`prompt.lines`, made by the reducer: decision 24). The sentence is `gettext` of "Someone saved it after you loaded it. Nothing was written." or, for a row that is gone, of "Someone deleted it after you loaded it. Nothing was written."

**How a value of a line is drawn** (the sheet and task 8's box share it: write it once, as a function from a `Shown` and the look's marks to what a cell shows):

- `Shown::Null` is NULL: `grid::null_label`, laid out in a child `Ui` at the cell (`ui.new_child(egui::UiBuilder::new().max_rect(cell))`).
- `Shown::Text { text, cut }` is one line: `format::blank_text(text, marks)` where that answers, else `format::cell_line(text, marks)`, with `marks` from `grid::marks(ctx, look)`, as `data_view` draws a text cell; behind a leading "…" when `cut`. That line is cut once more to its cell with `grid::ellipsize` and painted with `widgets::paint_label`, so a screen reader and the tests read it; where it was cut to the cell, the whole line is the hover text of a hover-only `ui.interact` on the cell, as `pending_bar::said` does it.
- A column's name is `format::display_safe(&line.name)`.

**The sheet.** A `widgets::modal(Id::new("conflict-prompt"), look, palette)` with its usual frame, `fitted(ctx, 520.0)` wide (make `Skin`, its fields and methods, `fitted`, `button_row` and `keyboard_on` of `write_prompts.rs` `pub(super)`; change nothing else there).

- **The title,** in `widgets::dialog_title(look)` and `palette.text`, on one line as wide as the sheet. Measure its three parts; when they do not fit, cut the name (and only the name) with `grid::ellipsize` to what `before` and `after` leave. Paint the line and give it the whole title as its accessible name and, when the name was cut, as its hover text (a hover-only `ui.interact` over the line with `widget_info`, as `pending_bar::said` has it).
- **The place line,** 4 pt under it, in `widgets::body(look)` and `palette.secondary`, cut with "…" to the sheet's width, the whole under the pointer. Then 4 pt and the sentence in the same role and colour, wrapped to the sheet's width, as `leave_sheet` lays out its text. Then 12 pt.
- **The table:** a `Frame` with a 1 pt `palette.border` stroke and `look.radius` corners, holding a `ScrollArea::vertical` of at most 220 pt around the rows. Allocate each row whole (`ui.allocate_exact_size`) and paint into it: the first column is 80 pt, the others share the rest equally, a row is 30 pt high and a cell's text starts 8 pt in.
  - The header row: a fill of `palette.surface`; the words `gettext` of "loaded", "now on server" and "yours" in `widgets::secondary(look)` and `palette.secondary`. For a row that is gone the "now on server" column is not drawn and the two others share the width.
  - Each line: a hairline above it (`widgets::hline` in `palette.surface`); the name in `widgets::code(look)` and `palette.text`; the loaded value in `grid::data_role(look)` and `palette.text`; the server's value the same, and where `moved` on a fill of `Tone::Danger.fill(look, palette)` with its text in `Tone::Danger.color(palette)`; the user's value on a fill of `Tone::Warning.fill(look, palette)` with its text in `palette.text`.
  - A server value that is NULL where `moved` is the NULL chip on the danger fill: the fill says it changed, the chip says to what.
- Then 14 pt and the buttons, 32 pt high. Enter is consumed before any button is made (`consume_press(input, Modifiers::NONE, Key::Enter)` at the top of `show`, as `leave` does): a button that has the keyboard would take it as a press of itself.
  - A row that changed: "Keep mine, reload row" is made first and placed at the row's left with `show_at`, as `ButtonSpec::new(..).link()`; then `button_row` with "Use server values" (`ButtonSpec::new`) and "Overwrite" (`ButtonSpec::new(..).primary().padding(16.0)`) at the right. Made in that order, the Tab key comes to Keep mine first.
  - A row that is gone: `button_row` with the one button "Discard my changes" (`ButtonSpec::new`, not primary).
  - The names are those of `answers(gone)`, through `gettext`. A click on a button pushes its answer. Enter pushes `Answer::KeepMine` when the Keep mine button `has_keyboard(ui)`, and nothing in every other case.
- `ButtonSpec::link` (`src/ui/widgets.rs`, beside `quiet`, 1278): "A secondary button that reads as a link: no border, a fill only under the pointer, the text in the accent colour." A `link: bool` field as `quiet` has one (set in `new`), and where `show_at` picks the quiet button's colours (1550) the link's are the same with `palette.accent` for the text. The view does not paint it itself.

**Tests** (`src/ui/conflict_prompt.rs`, each over `looks()`; `conflict_in` has run `finish_animations`)

- `the_sheet_shows_what_was_loaded_what_the_server_holds_and_yours`: the frame paints the title, "Fixture · users", "Someone saved it after you loaded it. Nothing was written.", "loaded", "now on server", "yours", `email`, `user2@example.com`, `eve@example.com` and `bob@example.com`. The sheet's `eve@example.com` is painted in `Tone::Danger.color(&palette)` and a fill of `Tone::Danger.fill(&look, &palette)` contains its place; a fill of `Tone::Warning.fill(&look, &palette)` contains the place of the sheet's `bob@example.com` (found as said above the tasks).
- `only_the_columns_the_user_changed_are_listed_and_a_value_the_server_kept_is_not_marked`: with `meta` of the row pending as well (`[1]`), and a server row whose email moved and whose `meta` is still NULL: the sheet has two lines, in the page's order (`email` above `meta`), exactly one fill of the danger tint inside the modal, and "NULL" is a label twice in the `meta` line (loaded and server). With only the email pending, `meta` is painted once in the frame: by the grid's header.
- `a_value_that_became_null_on_the_server_is_the_null_chip_on_the_red_tint`: its own setup: row 0 of the fixture, whose `meta` holds a value, with `meta` pending as `[1]`, saved, and answered with a server row whose `meta` is `Value::Null`. In the `meta` line the server's cell holds the label "NULL", and a fill of the danger tint contains it.
- `two_long_values_that_differ_past_the_cut_are_shown_from_where_they_differ`: its own setup: a page whose row 1 holds 300 `x` and then `loaded` in `meta` (answer the rows with such a page before editing), `meta` pending as 300 `x` and `yours`, the server's `meta` 300 `x` and `server`. The sheet paints three texts that begin with "…", one ending in `loaded`, one in `server` and one in `yours`, and no two of the three painted texts are equal.
- `a_value_that_differs_by_a_line_break_alone_shows_its_break`: its own setup: row 1's email pending as `a` and `b` on two lines, the server's email `a b`. The sheet's "yours" text holds the line mark of `grid::marks` between `a` and `b`, and is not the text of the server's cell.
- `the_place_line_counts_several_rows`: one conflict: "Fixture · users" is painted in `palette.secondary` on a line of its own under the title, and no "of" is. Two conflicts (answer the save with two; ripen with `shown(&mut harness, true)` after each step): "Fixture · users · 1 of 2", and after `click_dialog(&mut harness, "Use server values")`, "Fixture · users · 2 of 2" and the second row's title. `the_question_says_which_connection_and_table_its_row_is_of` of task 3 passes unchanged, now on the sheet.
- `a_long_key_gives_way_and_the_title_says_it_whole`: its own setup, twice. A table keyed by `email` (as `the_status_line_keeps_a_keys_value_and_the_databases_words_as_they_are` builds one) whose row holds a 36-character UUID in it: the painted title lies inside the modal, ends with "changed on the server", and holds "…" when the name did not fit; the title's label in the accessibility tree is the whole title. A table keyed by two columns (`primary_key` of `id` and `email`): the title's label is "Row id 2, email user2@example.com changed on the server", and the painted line lies inside the modal.
- `enter_answers_only_as_keep_mine`: Enter with the keyboard on no button leaves the question up and sends nothing. With the keyboard on "Overwrite" (`focus_dialog`) it leaves it up, and `writes` is still 1. On "Use server values" it leaves it up, and the cell is still pending. On "Keep mine, reload row" it answers: the question is gone, the cell is pending and the page holds the server's email.
- `the_tab_key_comes_to_keep_mine_first`: one press of Tab, and `focused_name` is "Keep mine, reload row".
- `keep_mine_reads_as_a_link_and_overwrite_as_the_primary_button`: "Keep mine, reload row" is painted in `palette.accent` and no outline is drawn round it; the fill behind "Overwrite" is the one the Leave prompt's Save has when nothing is over it (`widgets::primary_fill(false, false, false, &palette)`).
- `a_row_that_is_gone_has_its_own_words_and_one_button`: "Row id 2 no longer exists on the server", "Someone deleted it after you loaded it. Nothing was written.", the headers "loaded" and "yours" and not "now on server"; `pressable` finds "Discard my changes" and none of the three other names; Enter with the keyboard on it does nothing.
- `a_long_value_is_cut_to_its_cell_and_whole_under_the_pointer`: its own setup: the email pending as 120 characters. The text painted in the "yours" column ends with "…" and lies inside the sheet; with the pointer over it (as `hover` in `src/ui/mod.rs` waits for a tooltip) the whole 120 characters are on screen.
- `the_sheet_fits_a_small_window`: at 720 by 480 with twelve changed columns (a page and a structure built in the test), every button's bounds are inside the window.
- `each_answer_is_a_button_and_escape_keeps_mine`, `an_answer_in_the_questions_first_moment_is_not_taken`, `the_question_is_headed_with_the_row_by_its_key` and `the_question_says_which_connection_and_table_its_row_is_of` of task 3 pass unchanged.

Two commits, each green: "Give buttons a link style" (`ButtonSpec::link`, with a test beside the existing ones of `widgets.rs` that its text is `palette.accent` and it has no border), then "Show loaded, server and yours in the conflict sheet on macOS and Windows".

**macOS and Windows ask here, and the branch is shippable:** Omarchy keeps the line of step 3 until task 8.

---

### Task 8: The box, on Omarchy

**Files:**
- Modify: `src/ui/conflict_prompt.rs`, `src/app/editing.rs` (`written`), `src/ui/workspace.rs` (`drawable` becomes `pub(super)`), `src/ui/mod.rs` (two tests of step 3)
- Test: `src/ui/conflict_prompt.rs`, `src/ui/mod.rs`

The terminal look's form of the same question, built as `write_prompts::leave_box` is: a `widgets::modal` with `skin.frame()` (no inner margin), `fitted(ctx, 560.0)` wide, a body in a `Frame` with `Margin::symmetric(18, 14)`, and a foot from `terminal_dialog::foot` whose keys are its buttons. The stand-in's body goes with this task.

**The terminal look asks from here.** In `written` (`src/app/editing.rs`) the line that task 3 wrote,

```rust
        // A dialog the user is in is never replaced: a conflict that arrives
        // under one is a line, as a failure is. So it is in the terminal
        // look, until its box is drawn there.
        let free = self.dialog.is_none() && !self.look.terminal;
```

becomes

```rust
        // A dialog the user is in is never replaced: a conflict that arrives
        // under one is a line, as a failure is.
        let free = self.dialog.is_none();
```

and three things in the tests follow it:

- `src/ui/conflict_prompt.rs`: `looks()` gives every look (`Look::ALL.into_iter()`, and its comment says so). The four tests of task 3 then run in the terminal look too. They hold there as they are: the hints' buttons carry the names of `answers(gone)`, and `each_answer_is_a_button_and_escape_keeps_mine` already expects the status line's words for a kept row that is gone.
- `src/ui/mod.rs`: in `a_written_save_and_a_conflict_are_said_in_the_status_line` and in `the_status_line_keeps_a_keys_value_and_the_databases_words_as_they_are`, the two lines `harness.answer_written(Ok(WriteOutcome::Conflicts(vec![changed])));` and `harness.settle();` become the one line `conflicts_under_a_dialog(&mut harness, vec![changed]);`. Both still use `WriteOutcome` further down, and every assertion stays: they test the words of the line, which now stands in this look too only where the question cannot be asked.

(This much was built on the stand-in when the plan was written, and passes.)

**The box**

- **The first line:** the mark and `conflict` in `palette.warning`, then the title in `palette.text`, both in `TextRole::OGroup`. The mark is `workspace::drawable(ui, TextRole::OGroup, look, &["≠"])` (make it `pub(super)`): where the look's font has no such glyph the word stands alone, as in the status line. `conflict` is `skin.say("conflict")`. The title is laid out from its parts as in the sheet: only the name is cut, the whole is its accessible name and its hover text.
- **The place line,** 4 pt under it, in `TextRole::OBody` and `palette.dim`, cut to the box's width. Then 4 pt and the sentence in the same role and colour, through `skin.say`, wrapped (decision 29: the design's panel has no sentence).
- **The values,** 10 pt under it: no rules and no fills. A first column of 90 pt and three of equal width (two for a row that is gone), 10 pt apart, each line as high as `TextRole::OBody`'s row plus 4. The header in `palette.dim`: `skin.say` of "loaded", "server" and "yours". Each line: the name and the loaded value in `palette.text`; the server's value in `palette.danger` where `moved` and in `palette.text` where not; the user's value in `palette.warning`. Each value is drawn as the sheet draws one. NULL is `grid::null_label` (the terminal's faint word), except a server value that is NULL where `moved`: the word `NULL` painted in `palette.danger`, since the colour is all this look has to say that it changed. More lines than 220 pt scroll.
- **The foot** (decision 20): `terminal_dialog::keys` with, for a row that changed, `[o]` and `skin.say("overwrite")`, `[s]` and `skin.say("use server")`, `[k]` and `skin.say("keep mine, reload")`, in that order, each with `lead: true` (the design writes all three letters in the accent colour) and with its button named as `answers(gone)` names it, through `gettext`. For a row that is gone: `[d]` and `skin.say("discard my changes")`, its button "Discard my changes". A press of a hint's button pushes its answer. The words are dim and the hints end at the foot's right, as `terminal_dialog::keys` paints every foot.
- **The letters,** read as `leave_box` reads `w` and `d` (its lines 264 to 295): take every `Event::Text` that is `k`, `s`, `o` or `d` out of the input; it is an answer unless a key is held in this frame (an `Event::Key` with `pressed` and `repeat`) or a text field has the keyboard (`ctx.text_edit_focused()`). `k` is Keep mine, `s` Use server values, `o` Overwrite, `d` Discard; a letter whose answer the row does not have (`Answer::offered`) pushes nothing. Any other text is left alone and does nothing: no key runs behind a dialog.
- **Enter** is consumed before the foot is made and pushes `Answer::KeepMine` only when `terminal_dialog::keyboard_on` says the keyboard is on the Keep mine hint's button. Esc is read by `show`, as before.

**Tests** (`src/ui/conflict_prompt.rs`, `Look::omarchy()`)

- `the_box_says_conflict_and_writes_the_values_in_their_colours`: `conflict` is painted in `palette.warning` (and the piece before it is `≠` in the same colour when the font has the glyph); "row id 2 changed on the server" in `palette.text`; "Fixture · users" and "someone saved it after you loaded it. nothing was written." in `palette.dim`; the headers `loaded`, `server`, `yours` in `palette.dim`; `eve@example.com` in `palette.danger`. The box's own `bob@example.com` is in `palette.warning`: the grid behind paints the same text in the same colour, so find the piece whose place in `harness.text_rects` lies inside the modal and beside the box's `eve@example.com` (same line, to its right), and assert there is one. `[o]`, `[s]` and `[k]` are each in `palette.accent`, with `overwrite`, `use server` and `keep mine, reload` beside them in `palette.dim`, and all three stand in the foot (below the values, in the foot's rect).
- `a_value_the_server_kept_is_written_plain_and_one_that_became_null_in_the_danger_colour`: with `meta` pending too and unchanged on the server, no text of the box's `meta` line is in `palette.danger`. With row 0's `meta` (which holds a value) pending and NULL on the server, the box's `meta` line holds `NULL` in `palette.danger`.
- `k_s_and_o_answer_and_d_discards_a_row_that_is_gone`: by `type_key`: `k` keeps (one cell pending, the server's email in the page, one `Write` sent in all), `s` takes the server's (no cell pending), `o` overwrites (`writes` is 2). On a row that is gone `d` discards and the row is in `edits.gone`.
- `a_letter_the_row_does_not_offer_does_nothing`: `d` on a row that changed, `o` and `s` on a row that is gone, and `j` and `x` on either: the question is still up, the set and the selection are as they were.
- `a_held_letter_is_no_answer`: three frames, each bringing the repeat of the key (`egui::Event::Key { key: Key::O, physical_key: None, pressed: true, repeat: true, modifiers: Modifiers::NONE }`) and its text (`egui::Event::Text("o".into())`), leave the question up and `writes` at 1; so does the same with `s`, and the cell is still pending.
- `the_letters_wait_out_the_questions_first_moment`: with `shown(&mut harness, false)`, `type_key` of `o` and of `s` leave the question up; with `shown(&mut harness, true)`, `s` answers.
- `enter_in_the_box_answers_only_as_keep_mine`: Enter with the keyboard on no button does nothing. One press of Tab puts the keyboard on the `[o]` hint's button (`focused_name` is "Overwrite"): Enter there leaves the question up and `writes` at 1. With `focus_dialog` on "Use server values" the same, and the cell is still pending. On "Keep mine, reload row" Enter answers.
- `several_rows_are_asked_one_after_another_in_the_box`: two conflicts: "Fixture · users · 1 of 2" in `palette.dim`; after `s` (ripened) the second row's title and "Fixture · users · 2 of 2"; after `k` (ripened) the question is gone.
- `a_long_key_gives_way_in_the_box_too`: as the sheet's test, with the 36-character key.
- The four tests of task 3 pass in every look, and the two changed tests of `src/ui/mod.rs` pass.

Commit: "Answer a conflict with k, s and o on Omarchy". In its body, say that two more tests of step 3 changed and why: Omarchy asks now too.

**All three looks ask here.**

---

### Task 9: Scenes, the spec and every check

**Files:**
- Modify: `src/shots.rs`, `docs/superpowers/specs/2026-10-03-value-editing-core-design.md`

- **Scenes** in `src/shots.rs`, on the Bookshop data (never other data), for review by eye. `EDITING` (359) is an array whose size is in its type: it grows by two (step 4's plan adds a scene of its own to it; whichever lands second counts the other's):
  - `edit-conflict`: `editable`, `retire_image` (two pending cells in the row at place 1), `step_aside`, `Action::WriteEdits`, and the save answered with `WriteOutcome::Conflicts` of one `Conflict { row: 0, server: Some(row) }`, where `row` is `page().rows.swap_remove(1)` with its `KIND` set to a value that is neither the loaded one nor the scene's `cover` (one the column's list allows: read `structure()` in that file) and its `DELETED_AT` as loaded. The question shows two lines, one of them marked.
  - `edit-conflict-gone`: the same up to the save, answered with `Conflict { row: 0, server: None }`.
  Shots are not run in the suite and not committed.
- **The value-editing spec:**
  - The status line: step 5 is built, with this plan among the plans it names (step 4's plan rewrites the same line: say what is true when this lands).
  - "What can be edited": a row a save found gone and the user discarded is locked until the page is loaded again ("This row no longer exists on the server"), and is drawn dim.
  - "Conflicts": as built. Keep the spec's own sentences and add this plan's decisions where they add to them or depart from them: the question is a dialog holding every conflict of the save, each answer applied when given; where it cannot be asked the conflict is the line of step 3 (another dialog up, a row that does not fit the page); Esc on a row that is gone keeps its changes and leaves the line; closing the question leaves the line; a gone row does not hold the page; the rule for the second save with what counts (decision 9) and that it is an ordinary save (decision 10); what was held is dropped for good; nothing changes under the question; the first 500 ms, for the question and for a confirmation an answer opened; Enter; Omarchy's letters, their guards and their place in the foot (decision 20 departs from the design); the line that names the connection, the table and the count; how the title gives way; what the table lists and marks, and how two values that would read alike are shown; the words for a row that is gone.
  - "Saving to production": the confirmation's first moment where an answer opened it.
  - "Leaving with pending changes", the sentence on what a prompt asks about: the conflict question is the third dialog under which the reducer drops those actions.
  - "What step 3 leaves for steps 4 and 5": the two items of step 5 that are done go; "A MySQL `TIMESTAMP` as a changed column can miss a conflict" moves under "Open in the save as the grid shows it". Step 4's plan edits the same section for its own items.
  - "Testing": the scenes' list gains `edit-conflict` and `edit-conflict-gone` (step 4's plan adds its scene to the same list).
- **Every check,** with both server URLs exported:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
~/.cargo/bin/cargo test --locked --workspace --all-targets
```

- **By hand, by the user** (no window can open in an agent's session), on the demo (`--demo` opens a writable SQLite file; a second program changes it, for instance the `sqlite3` command line on the same file): edit a cell and do not save; change the same column of that row from outside; save, and take each answer in turn (Keep mine, then save again; Use server values; Overwrite); change two rows from outside and answer them differently; delete a row from outside, save an edit of it, press Esc and read the bar's line, save again and discard, and see the row dim until a refresh; on a production-labelled connection, Overwrite and see the confirmation again, and that a second click in the same place within half a second does not confirm it; close a tab with a pending cell on that connection, choose Save, and see the same; in the Omarchy look the same with `:w`, `k`, `s`, `o` and `d`, and press `s` at once when the box comes up to see that the first moment takes no answer.

Commit: "Describe the conflict dialog as built".

---

## As built

The nine tasks as above, on `claude/connection-write` and after step 4, with these differences from the draft. The value-editing spec says the whole of it in the present tense ("Conflicts", "Saving to production", "What can be edited").

- **It ran after Review SQL,** on the same branch and not on one of its own. So `answer_conflict` leaves the tab's review stale (`object.edits.review = None`, where it drops `object.fields`), for every answer, and the review is made again in that round of actions: the check's comment says what was loaded, and a row without cells has no statement.
- **Copy SQL is withheld in the confirmation's first moment** (decision 17 named the answers alone): a click meant for the dialog before does not replace what is on the clipboard. Page Up and Page Down still move the statements then.
- **A key that is held never answers** (decision 16 held back what came in the first moment, and decision 19 a held letter; a held Space or Esc answered once that moment was over). With the keyboard on a button, a fresh Space answers a row, the next row's question has the keyboard on the same button, and a button reads what a held Space repeats as a press of itself: held on Overwrite it overwrote every row of the save, one each half second, and a held Esc kept row after row. The question takes what a held Space repeats out of the frame before its buttons are drawn (`keys::drop_repeats`) and reads Esc as a fresh press, as it reads Enter (`keys::consume_press`). The confirmation drops a held Space the same way.
- **A click counts only when its press, too, came after the first moment** (decision 16 asked only when the click arrived): the question remembers, for the press that is down, whether it took answers when the pointer went down.
- **"Alike" is decided by what fits** (decision 23 had two values alike only where their 256 characters read the same). A value's cell holds about sixteen characters, so emails that differ in their last letters, timestamps in their seconds and long texts in their middle painted the same string. The view settles what the cells of a line paint together (`conflict_prompt::told_apart`), on what a cell reads of each value: two that would be painted alike and do not read alike are each painted from before the first place they differ. The reducer still keeps the part of each value that differs past a cell's worth (`edit::shown_lines`), and `edit::Shown::Text` says how far into the value its text starts (`from`) where the draft had a flag.
- **Pair by pair** (decision 23 took one place for the whole line): of three values, two that are still alike from the first difference are shown from before their own, in the reducer and in the view. Two values that are the same are painted the same.
- **The lead gives way** (decision 23 had a fixed twelve characters before the difference): they did not fit a cell of 130 points, so a cell gives them up first, one by one, and cuts the end only when the value is too long from the difference on.
- **Page Up and Page Down move the lines** that do not all show, by the whole lines in view, and the box's foot then says `pgup/pgdn scroll` before the keys that answer. The draft had the lines scroll under the pointer alone.
- **The first line the server changed is in view when a row's question comes up** (`ConflictPrompt::fresh`, which the view takes down once it has placed the lines): lines are in the page's column order, and the marked one stood below the fold with nothing marked in view. Said for the top too, since the lines of an earlier save's question stay where they were left.
- **`k` keeps a row that is gone,** as Esc does (decision 19 gave such a row `d` alone): Keep mine is an answer every row has. The foot shows `[d]` only.
- **A screen reader is told which server value changed:** the marked cell's name ends ", changed on the server".
- **The primary button is the ink fill** (`palette.text`), as the Leave prompt's Save is; the draft's test named `widgets::primary_fill`.
- **The tests find the question's own paint by order,** not by place: a dialog is painted over everything else, and the first thing it paints is what dims the window (`own_fills`); its title, or the word `conflict`, is the first text of its own (`pieces`). The grid behind paints the same values, and may stand where the question does.
- **What `write_prompts.rs` lends** (decision 27) is `ROW`, `Skin`, `fitted` and `buttons_in`; the box asks `terminal_dialog::keyboard_on`.
- **The scenes** are not of `retire_image` (task 9): `kind` takes `cover` and `preview` and no third value, and `deleted_at` is cut in the sheet's first column. `rebook_image` changes `book_id` and `kind` of the row at place 1; in `edit-conflict` the server holds another `book_id`, in `edit-conflict-gone` the row is gone. They are the seventh and eighth of `EDITING`, which holds ten.

Left as found (the value-editing spec lists them under "What step 3 leaves for steps 4 and 5"):

- CR, LF and CRLF all read as one line-break mark, so two values that differ only in that read alike, as a tab does against a space.
- A table whose columns were put in another order, their number unchanged, between the page and the save is not noticed by `edit::conflicting`. The saves after it end in a conflict again and again, never in a silent write.
- In a window lower than about 430 points the question overflows.
- A gone row's question shows only Discard, and nothing on screen says that Esc keeps.
- A save's many rows are answered one by one.
- Where two of three values differ only past a cell's worth and the third differs early, the third is read from its start, and its cell need not reach the place it differs.
- Enter with Ctrl or Cmd held presses the button that has the keyboard, Overwrite among them, as in the two prompts of step 3: only a plain Enter is held back.
- The production confirmation takes a click that was pressed in its first moment and let go after it.

## What this plan leaves for later

- **Step 4, Review SQL (`docs/superpowers/plans/2026-10-04-review-sql.md`), touches the same files,** and was built first. What of this plan lands in the files both touch:
  - `src/edit.rs`: tasks 1 and 2 add `Lock::Gone`, `Table::gone`, `Edits::gone`, `Edits::discard`, change the `Debug` of `Edits` (its format string gains the gone rows), and add the conflict's types and functions between `Note` and `change_set`. Step 4 adds `Edits::{reviewing, review}` and the methods `put` and `revert` there.
  - `src/model.rs`: one action, one dialog variant, `ConflictPrompt`, and one field of `WritePrompt` (`after_answer`, task 5). **Both plans add to `WritePrompt`:** step 4 replaces `statements` by `review`. Neither removes the other's, and the struct's one literal, in `write_edits`, gets both.
  - `src/app.rs`: tasks 1, 3 and 5 change the first check of `apply`, the `DiscardEdits`, `LeaveDiscard` and `LeaveSave` arms (one line each), the `CloseDialog` arm, add one arm, and change `Event::Rows`.
  - `src/app/editing.rs`: tasks 1, 3, 4 and 5 change `written` (one line of its `Written` arm, the whole of its `Conflicts` arm, one line at its top), one line of `write_edits` (`after_answer: None` in the prompt it makes), the imports, and add `answer_conflict`, `write_as_answer` and `conflict_unanswered` at the end of the `impl`. `confirm_write`, `send_write` and `run_command` are not touched.
  - `src/ui/pending_bar.rs`: task 3 changes one word (`row_name` becomes `pub(crate)`). The bar itself is not touched.
  - `src/ui/write_prompts.rs`: task 5 adds one line near the top of `confirm_write` and wraps its last line; task 7 changes the visibility of `Skin` (with its fields and methods), `fitted`, `button_row` and `keyboard_on`. Step 4 changes `confirm_write`, `statements`, `confirm_sheet` and `confirm_box` for its review: the two edits of `confirm_write` are apart, and both stay.
  - `src/ui/workspace.rs`: task 8 changes the visibility of `drawable`. The status line is not touched.
  - `src/ui/widgets.rs`: task 7 adds `ButtonSpec::link`, step 4 adds `ButtonSpec::keyed`. Each adds a field to `ButtonSpec`, a line to `new` and a branch to `show_at`: the second to land keeps the first's.
  - `src/ui/keys.rs`: nothing.
  - `src/shots.rs`: task 9 adds two scenes to `EDITING`, whose size is in its type; step 4 adds one. The size is the sum.
  - `src/ui/mod.rs`: one module, one call, three tests of step 3 and whichever helpers become `pub(super)`. Both plans put tests and helpers into this file's test module and into the nested `mod editing` of `src/app.rs`, side by side.
  - The value-editing spec: both plans rewrite the status line, "What step 3 leaves for steps 4 and 5" and the scenes' list under "Testing", each for its own step.
  - **What each order of building needs.** Step 4 keeps the review on the tab (`Edits::review`) and makes it stale wherever the set changes through `Edits::put` and `Edits::revert`. This plan writes to `Edits::cells` and to a page row under pending cells without them, in one function: `answer_conflict` replaces the page's row (Keep mine, Use server values, Overwrite) and drops cells with `retain` (every answer but Keep mine on a row that is gone). A search for `insert` and `remove` does not find it. So: **if step 4 is built first,** `answer_conflict` sets `object.edits.review = None` where it drops `object.fields`, which covers every answer. **If this step is built first,** step 4's task 3 adds that line there, as its plan says. `Edits::discard` resets `reviewing` and `review` with everything but the gone rows, as `Edits::default()` did in the three places it replaces, which is what step 4 wants of a discard and of a save that wrote. Whatever step 4 adds to `dropped_under_a_prompt` holds under the conflict question too, since task 3 puts `Dialog::Conflict` into the same check. The save after an Overwrite goes through `write_edits`, so on production it gets whatever step 4 makes of the confirmation (its review, the panel beside the Omarchy box) without a change here; the first moment of task 5 is in `confirm_write`, which draws either form.
- **A conflict that cannot be asked about is still a line,** and the user saves again to be asked. Keeping the rows for a question to open once the other dialog closes was not built: it needs the conflicts on the tab as well as in the dialog.
- **Answering every row at once** ("Overwrite all", "Use server for all") is not in the spec. Twenty conflicts are twenty questions, each with its first 500 ms.
- **A row that was answered Keep mine is written by the next save without a question,** when it did not change again: that is what Keep mine followed by Save means. Nothing marks such a row apart from any other pending row.
- **The server's row is put into the page on a check of its width alone,** as a save that wrote already puts its rows there: a table whose columns were swapped for others of the same number since the page was read is not noticed.
- **After a rebase a pending cell can sit on a cell that is now locked:** the server's value there is bytes, or over 256 KiB. The cell stays pending and cannot be opened; Revert and Discard all still take it out, and a save of it fails its row with the builder's reason.
- **Two values that still read alike** after decision 23 (a tab against a space, the number 1 against the text `1`, one kind of line break against another) are told apart by the tint alone.
- **The gone row's look** is the dim text of this plan until slice 5 (rows: add, duplicate, delete) draws a deleted row; the two should then agree.
- **A MySQL `TIMESTAMP` as a changed column** can miss a conflict in a repeated daylight-saving hour (step 3 found it). The question is then not asked and the save writes.
- **On a keyboard layout without Latin letters** `k`, `s`, `o` and `d` do nothing, as Omarchy's other letters: the hints are buttons, and Esc keeps.
- **The notice of a refused guarded action** ("Save or discard the pending changes first.") is also what a close request under the question says. Its words fit the Leave prompt better than this one; it was left as found.
