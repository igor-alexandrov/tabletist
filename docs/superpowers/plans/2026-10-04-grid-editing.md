# Editing in the Grid Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Step 3 of `docs/superpowers/specs/2026-10-03-value-editing-core-design.md`: a user edits cells of a table's grid as text, sees them pending, and saves them with `Connection::write`, in all three looks. Leaving with pending changes asks first, and a save to production is confirmed with its statements on screen.

**Architecture:** A table's tab holds its pending set (`ObjectTab::edits`), keyed by row and column of the loaded page. What can be edited, what a column takes and the change set a save sends are pure functions in a new `src/edit.rs`. The reducer owns every transition; one guard at the top of `App::apply` holds any action that would drop a page with pending changes. Views draw the states and push actions; the only state a view changes is the text an editor is typing.

**Tech Stack:** Rust, egui (the crmne fork of 0.36), `tabletist-db` (`Connection::write`, `column_class`, `Structure::row_key`, `Dialect::update_row`), the headless UI harness in `src/testing.rs`.

---

## Before you start

- Cargo is `~/.cargo/bin/cargo`. Never a cargo target dir under `/tmp`. Export both test server URLs when you run the whole suite:

      export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
      export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist

- House rules (`AGENTS.md`): no em dashes anywhere; comments say why, in the surrounding code's voice; no `unsafe`; do not weaken a lint, delete a test or add an `allow`; a view never mutates state except the text a field is editing; a view never names a font (use `TextRole`s) and never paints a focus ring (use `focus::hint`); every user-visible string in a view goes through `gettext(locale, ..)`; add a focused regression test for every behaviour change.
- Four source scans fail the build and bite here (`src/env.rs`, tests at the end): no file outside `env.rs` may match on `Environment::<name>` (add a method to `Environment` instead); `src/model.rs` may not contain `Color32`; no view may write a `#rrggbb` colour (mix palette colours instead); and only the files in the `surfaces` list may call `env_colors(` (a new view takes the production red from `states::Tone::Danger`, which is that colour).
- **The design is the source of truth for how it looks.** It is not in the repository and is never copied into it. What this plan needs from it is written out under "What it looks like" below. No test compares a screen with the design.
- **This plan runs on a new branch off `claude/connection-write`** (or off `main` once pull request #76 is merged). It needs step 2: `Connection::write`, `Command::Write`, `Event::Written`, `Structure::row_key`, `column_class`, `Dialect::update_row`.
- Commit after every task. Subjects are plain sentences, each ending with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

  If the commit's signing agent is locked ("agent refused operation"), do not bypass it: `git add -A`, `git write-tree`, and report the tree id with the subject.
- **How the code stands.** Read each function before you edit it; where this plan's code and the code disagree, the code wins and you say so in your report.
  - `src/model.rs`: `ObjectTab` (around line 1519) holds `rows: Fetch<RowPage>`, `structure: Fetch<Structure>`, `selection: Option<CellPos>`, `pinned`. `Action` (line 47) is neither `Clone` nor `PartialEq`. `Dialog` (around 1052) has seven variants and none is a confirmation. `Workspace` has `access`, `environment`, `driver`, `status`, `session`.
  - `src/app.rs`: `App::apply` (line 322) is one flat match with early returns and a few re-entrant `self.apply(..)` calls. `fetch_page` (around 3047) is the only place `Command::FetchRows` is sent and it leaves the old page on screen while the new one loads. `after_connect` (around 2499) always fetches the active table tab's rows again. `Event::Rows` replaces the page whole. `Event::Written` is an empty arm (around 2187). `keys::handle` is not called while `app.dialog` is `Some`.
  - `src/ui/grid.rs`: a row is one widget and cells are painted, not widgets; the column of a click comes from the pointer's x. Only the Omarchy look has a gutter (22 pt). The same `grid::show` draws SQL results (`src/ui/sql_results.rs`), which must stay as they are. The text a cell shows is not the value (cut at 256 characters, breaks marked, numbers grouped); `format::plain_text(value)` is the whole text.
  - `src/ui/keys.rs`: grid keys run only when no text field has the keyboard (`ctx.text_edit_focused()`); Omarchy's letters are in `letters` (around 568), where `i` and Enter toggle the row panel today; `SHORTCUTS` is the table the shortcuts screen lists.
  - `src/testing.rs`: `Harness::connect_fake()` gives a read-only fixture connection with no structure answered; `page(rows, has_more)` has columns `id` (INTEGER), `email` (TEXT), `meta` (JSON). There is no helper that answers a `Write`.
  - Tests of time back-date a stored `Instant` (see `age_fetch` in `src/ui/mod.rs`); there is no injected clock.

## What it looks like

From the design's "Editing values" artboards. Colours are given as what they are, not as numbers: every one is a mix of palette colours (see decision 12).

**macOS and Windows**

- **Pending cell:** an amber tint behind the value and a 2 pt amber bar at the cell's left edge. Hovering shows "was <loaded value>".
- **A changed row:** a 3 pt amber bar at the row's left edge, and the row's key value in amber, medium weight. With a cell to fix or failed, both are red.
- **Editing:** the field sits on the cell at the cell's size, with the accent border and halo every focused field has. While its text fails a check the border and halo are red and the message stands under the field in red. A length limit shows as a counter at the field's right, `27 / 200`, dimmed.
- **The large editor:** a popover anchored to the cell, about 420 by 180 pt, with the accent border; under the text a band reads `2,341 chars · 2 lines` at the left and `⌘↩ apply · esc cancel` at the right (`Ctrl+↩` on Windows).
- **Saving:** the pending cells keep their tint and show a small spinner at their right.
- **Saved:** a green tint for 1.2 seconds, then the value as re-read.
- **Failed:** a red tint with a 1 pt red line inside the cell. To fix: the same, and a dark tooltip at the cell: the message, and under it "Checked before saving · ⌘Z reverts".
- **Locked cell** (only when asked, by Enter or a double-click): a note at the cell saying why. The cell itself is drawn as today.
- **The pending bar,** above the footer, 48 pt, amber tint with an amber line on top: an 8 pt amber dot, "3 changes in 2 rows" in medium weight, then in red with a small icon "1 to fix"; at the right "Discard all" (bordered) and "Save ⌘S" (primary). Save is disabled, with the reason as its tooltip, while a cell is to fix ("Fix 1 value to save"), while disconnected, and on a session that came back read-only. Review SQL is step 4 and is not drawn.
- **Save on production:** a dialog with a 4 pt red band on top: "Save 2 changes to production?", then "Bookshop · bookshop_production · 1 row in book_covers", the statements in the code face in a bordered box that scrolls, and a foot with "One transaction" at the left, **Cancel** and **Save to production** (red) at the right.
- **Leaving:** "Save 3 changes before closing?" style prompt with **Save**, **Discard**, **Cancel**.

**Omarchy**

- **Gutter:** `~` in the warning colour on a changed row, `!` in the danger colour when one of its cells is to fix or failed.
- **Pending cell:** a warning tint behind it and the text in the warning colour. **Failed or to fix:** a danger tint, a 1 pt danger line inside, the text in the danger colour, and under the grid a line in the danger colour: `! 4:publisher_id  int8 expects a whole number` (row number, column, message).
- **Editing:** a 2 pt accent line inside the cell and a block cursor.
- **Mode line** while editing: `-- INSERT --` in the success colour and bold (`TextRole::OModeLine`), then the column and its type (`alt_text · text`), then `3 pending · 2 rows` in the warning colour, `1 error` in the danger colour, and at the right `esc normal · tab next cell`. Outside insert mode the counts stand in the status line whenever the set is not empty.
- **The tab** of a table with pending changes ends in `[+]`.
- **Write on PROD:** a box with a 2 pt danger border; a head tinted with danger holding a `PROD` chip (danger fill, bold) and "write 2 changes?"; "Bookshop · bookshop_production"; "1 row in book_editions · format, alt_text" dimmed; the statements; "type write to confirm"; a field with a danger border; a foot with "enter confirm" and "esc cancel".
- **Leaving:** "closing with pending edits: [w] write [d] discard [esc] stay".
- **After a save:** `✓ written 2 changes · 1 row · 14 ms` (the mark in the success colour); a failure: `✗ failed 23514 check book_editions_format_check`, and under it "rolled back · cells stay pending in red".

## What this plan decides beyond the spec

1. **The pending set lives in `ObjectTab::edits`** and is keyed by row and column index of the loaded page. The invariant everything else serves: **a tab that holds edits never has its page replaced.** "Holds edits" is a pending cell, an open editor, or a save in flight.
2. **One guard, at the top of `App::apply`.** It asks which tabs holding edits the action would drop. None: the action runs. Some: the action is kept in a new `Dialog::Leave` and runs only after Discard, or after a Save that wrote everything. Backend events are never guarded. A guarded action that arrives while another dialog is open is refused, with a notice ("Save or discard the pending changes first."), since a dialog the user is in is never replaced. While a tab's save runs, guarded actions on it are ignored.
3. **What replaces a page without an action skips such tabs:** `after_connect` (a tab with edits keeps its page and its structure through a reconnect), and `resize_pages` (it keeps its page size until its next fetch). `follow_foreign_key`, which can filter an already open tab, holds itself through the same guard.
4. **A cell is locked while its page is being fetched again** (a refresh keeps the old page on screen) and while a save runs, so nothing is edited on a page about to be replaced.
5. **A check's verdict is data** (`edit::Problem`), worded by the view through `gettext`. The reducer never builds a user-visible sentence for it.
6. **Pending means typed, and different from where the editor starts.** The editor starts from the value's whole text (`format::plain_text`), from `true` or `false` in a boolean column whatever the driver loaded, and empty on NULL. An editor that was opened and closed without typing changes nothing (`Editor::touched`): on a NULL cell it would otherwise make the empty string. A typed text equal to the start of a cell that was not NULL, or NULL on a NULL cell, takes the cell out of the set.
7. **A save's request is one of the tab's pending requests,** so `Mod+.` cancels it like any query. A session swapped under a save (reconnect) abandons it: the set is kept and the tab says the connection was lost while saving.
8. **After `Written` the rows are replaced in place** by what the database returned, the set is cleared, the row panel's text cache is dropped, and the cells show saved for 1.2 seconds from an `Instant` the view reads. If a returned row is not as wide as the page (the table changed), the page is fetched again instead.
9. **A conflict in this step is a line in the pending bar,** as the spec's step list says: "Row id 2 changed on the server. Nothing was written." The set stays; the dialog is step 5.
10. **Production is asked through `Environment::confirms_writes()`,** a new method in `src/env.rs` (only production). The confirmation's statements are `Dialect::update_row(..).shown` for each row, built once when the dialog opens.
11. **macOS and Windows get no gutter.** A changed row is marked by its left bar and its key value's colour, as the design draws it. The `~` and `!` marks are the Omarchy gutter's.
12. **Colours are palette mixes.** `states::Tone` gains `Success`, and pending, failed and saved cells use `Tone::{Warning, Danger, Success}::fill` and `line`. No hex colour is written.
13. **The Omarchy `:` prompt takes `w` and `e!` in this step.** `:diff` arrives with Review SQL (step 4); until then it answers "not a command" like anything else.
14. **Closing the window is held with `ViewportCommand::CancelClose`** when a tab holds edits, and the Leave prompt is shown. On macOS, Quit from the menu may not pass through a close request; that is checked by hand (see task 11) and written down either way.
15. **`Edits` has a hand-written `Debug`** that leaves out what the user typed, as `SqlTab` has.
16. **Editing does not need a live session; saving does.** While disconnected the set is kept and edited, and Save is disabled with the reason.
17. **What the save refuses, the grid locks up front** where it can tell (step 2 left these for here): on SQLite a row whose key text holds U+FFFD and a column whose name another column of the page shares; on MySQL a table keyed by a `timestamp`, `bit` or `float` column.
18. **The row panel's Edit, Duplicate and Delete stay disabled** (the spec's "Out of scope"), and so does the header's Add row.
19. **Only a computed column is drawn locked.** The spec has every locked or uneditable cell drawn as the design's "locked" on macOS and Windows; the design draws it for identity and generated columns. A whole table that cannot be edited (a view, a read-only connection, no key) would turn grey, and key columns already have their own colour. So in a table that can be edited, the cells of a generated column are drawn locked (the surface tint, the secondary text colour); every other cell that cannot be edited is drawn as today and says why when asked.
20. **Smaller additions,** each where a rule needed an answer: the checks also say when a whole number is out of its type's range and when a decimal has too many digits before the point; a save that was cancelled or refused says so in the bar ("Save cancelled. Nothing was written."); **Enter never discards**, so in a leave prompt without Save it does nothing; `Action::Connect` onto a tab whose workspace holds edits is guarded like a disconnect; a guarded action under another dialog is refused with the notice "Save or discard the pending changes first."; a tab with pending changes is named "{name} tab, unsaved" to a screen reader; Alt+Enter adds its line break at the end of the text; the checks name a column's type as the grid's header shows it.

## Where a run can stop

Each of these leaves the branch shippable:

- **After task 5:** the model is whole and tested; nothing a user sees has changed.
- **After task 12:** macOS and Windows edit, save, leave, confirm and close as the spec has it. Omarchy still reads only: nothing there opens an editor.
- **After task 15:** all three looks.
- **After task 16:** the scenes and the documents.

Between tasks 6 and 12 the branch is not shippable: editing is reachable on macOS and Windows from task 7, and what the spec requires around it (the confirmation with its statements, the question before the window closes, the row panel agreeing with the grid) arrives by task 12.

## File map

| File | What it holds |
|---|---|
| `src/edit.rs` (new) | `Edits`, `Pending`, `State`, `Editor`, `Saving`, `Saved`, `Note`; `Lock` and `Table::lock`; `Problem` and `check`; `start_text`; `change_set`; counts. Pure, with its unit tests. |
| `src/model.rs` | `ObjectTab::edits`; the new `Action`s; `Dialog::Leave`, `Dialog::ConfirmWrite`; `Held`; `ObjectTab::pending` gains the save. |
| `src/app.rs` | The guard; the editing arms; `write_edits`; `Event::Written`; the exceptions in `after_connect`, `resize_pages`, `follow_foreign_key`, `forget_session_requests`' caller. |
| `src/env.rs` | `Environment::confirms_writes()`. |
| `src/testing.rs` | `fixture_structure()`, `Harness::editable()`, `Harness::answer_written()`. |
| `src/ui/grid.rs` | Cell and row marks, the double-click, the hovered cell, the editor's place in a cell. |
| `src/ui/cell_editor.rs` (new) | The field on the cell and the popover; what keys end an edit. |
| `src/ui/pending_bar.rs` (new) | The bar above the footer (macOS, Windows). |
| `src/ui/write_prompts.rs` (new) | The Leave prompt and the production confirmation, both looks. |
| `src/ui/data_view.rs` | Wiring the grid to `edit`; the Omarchy error line. |
| `src/ui/keys.rs` | The editing keys per look, the `:` prompt, the shortcuts table. |
| `src/ui/workspace.rs` | The bar's panel; the Omarchy mode line. |
| `src/ui/object_tabs.rs` | The unsaved mark. |
| `src/ui/row_panel.rs` | A pending value in the panel. |
| `src/ui/states.rs` | `Tone::Success`. |
| `src/ui/sql_results.rs` | The refused-write card's extra line on a writable connection. |
| `src/app.rs` (`frame_ui`) | Holding a window close. |
| `src/shots.rs` | Scenes for review. |

---
### Task 1: What can be edited

**Files:**
- Create: `src/edit.rs`
- Modify: `src/lib.rs` (`pub mod edit;`)

A pure question: given the session's access, the object's kind, the structure, the loaded page and a cell, may that cell be edited, and if not, why. The reducer asks it before it opens an editor; the views ask it to say why.

- [ ] **Step 1: Write the failing tests**

At the end of `src/edit.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tabletist_db::{ColumnMeta, IndexInfo, ValueKind};

    fn column(name: &str, type_name: &str) -> ColumnInfo {
        ColumnInfo {
            name: name.into(),
            type_name: type_name.into(),
            nullable: true,
            ..ColumnInfo::default()
        }
    }

    fn structure() -> Structure {
        Structure {
            columns: vec![
                ColumnInfo {
                    nullable: false,
                    ..column("id", "INTEGER")
                },
                column("email", "TEXT"),
                column("meta", "JSON"),
            ],
            primary_key: vec!["id".into()],
            ..Structure::default()
        }
    }

    fn meta(name: &str, type_name: &str) -> ColumnMeta {
        ColumnMeta {
            name: name.into(),
            type_name: type_name.into(),
            kind: ValueKind::Text,
        }
    }

    fn page(rows: Vec<Vec<Value>>) -> RowPage {
        RowPage {
            columns: vec![
                meta("id", "INTEGER"),
                meta("email", "TEXT"),
                meta("meta", "JSON"),
            ],
            rows,
            has_more: false,
            ordered_by_key: true,
            elapsed: Duration::ZERO,
        }
    }

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    fn rows() -> Vec<Vec<Value>> {
        vec![
            vec![Value::Int(1), text("ada@example.com"), Value::Null],
            vec![Value::Int(2), text("bob@example.com"), text("{}")],
        ]
    }

    fn table<'a>(structure: Option<&'a Structure>, page: &'a RowPage) -> Table<'a> {
        Table {
            access: Access::Writable,
            kind: ObjectKind::Table,
            dialect: Dialect::Sqlite,
            structure,
            page,
            refreshing: false,
            saving: false,
        }
    }

    fn at(row: usize, col: usize) -> CellPos {
        CellPos { row, col }
    }

    #[test]
    fn a_cell_of_a_keyed_table_on_a_writable_connection_can_be_edited() {
        let (structure, page) = (structure(), page(rows()));
        let table = table(Some(&structure), &page);
        assert_eq!(table.lock(at(0, 1)), None);
        assert_eq!(table.lock(at(1, 2)), None);
        assert_eq!(table.key(), Some(vec![0]));
    }

    #[test]
    fn what_cannot_be_edited_says_why() {
        let (structure, page) = (structure(), page(rows()));
        let ok = || table(Some(&structure), &page);
        // The whole table, in the order the reasons are given.
        let read_only = Table {
            access: Access::ReadOnly,
            ..ok()
        };
        assert_eq!(read_only.lock(at(0, 1)), Some(Lock::ReadOnly));
        let view = Table {
            kind: ObjectKind::View,
            ..ok()
        };
        assert_eq!(view.lock(at(0, 1)), Some(Lock::NotATable));
        assert_eq!(
            table(None, &page).lock(at(0, 1)),
            Some(Lock::StructureLoading)
        );
        let keyless = Structure {
            primary_key: Vec::new(),
            ..structure.clone()
        };
        assert_eq!(table(Some(&keyless), &page).lock(at(0, 1)), Some(Lock::NoKey));
        let saving = Table {
            saving: true,
            ..ok()
        };
        assert_eq!(saving.lock(at(0, 1)), Some(Lock::Saving));
        let refreshing = Table {
            refreshing: true,
            ..ok()
        };
        assert_eq!(refreshing.lock(at(0, 1)), Some(Lock::Refreshing));
        // One cell.
        assert_eq!(ok().lock(at(0, 0)), Some(Lock::KeyColumn));
        assert_eq!(ok().lock(at(9, 1)), Some(Lock::NoSuchCell));
        assert_eq!(ok().lock(at(0, 9)), Some(Lock::NoSuchCell));
    }

    #[test]
    fn a_row_whose_key_is_null_is_locked() {
        let structure = structure();
        let page = page(vec![vec![Value::Null, text("a"), Value::Null]]);
        assert_eq!(
            table(Some(&structure), &page).lock(at(0, 1)),
            Some(Lock::KeyIsNull)
        );
    }

    #[test]
    fn generated_binary_and_huge_cells_are_locked() {
        let mut structure = structure();
        structure.columns[1].generated = true;
        structure.columns[2].type_name = "BLOB".into();
        let page = page(rows());
        let table = table(Some(&structure), &page);
        assert_eq!(table.lock(at(0, 1)), Some(Lock::Generated));
        assert_eq!(table.lock(at(0, 2)), Some(Lock::Binary));
        // Bytes in a column of another type are binary all the same.
        let structure = self::structure();
        let page = self::page(vec![vec![
            Value::Int(1),
            Value::Bytes(vec![1, 2].into()),
            text(&"x".repeat(MAX_EDIT_BYTES + 1)),
        ]]);
        let table = self::table(Some(&structure), &page);
        assert_eq!(table.lock(at(0, 1)), Some(Lock::Binary));
        assert_eq!(table.lock(at(0, 2)), Some(Lock::TooLarge));
    }

    #[test]
    fn a_column_the_structure_does_not_know_or_the_page_names_twice_is_locked() {
        let structure = structure();
        let mut page = page(rows());
        page.columns[2].name = "extra".into();
        assert_eq!(
            table(Some(&structure), &page).lock(at(0, 2)),
            Some(Lock::UnknownColumn)
        );
        // Two columns of one name: no statement can say which it means.
        page.columns[2].name = "email".into();
        let table = table(Some(&structure), &page);
        assert_eq!(table.lock(at(0, 1)), Some(Lock::UnknownColumn));
        assert_eq!(table.lock(at(0, 2)), Some(Lock::UnknownColumn));
    }

    #[test]
    fn a_key_the_save_would_refuse_locks_the_table_or_the_row() {
        // MySQL: a timestamp, bit or float key cannot be matched exactly.
        let mut structure = structure();
        for type_name in ["timestamp", "timestamp(6)", "bit(8)", "float", "float unsigned"] {
            structure.columns[0].type_name = type_name.into();
            let page = page(rows());
            let table = Table {
                dialect: Dialect::MySql,
                ..table(Some(&structure), &page)
            };
            assert_eq!(table.lock(at(0, 1)), Some(Lock::KeyType), "{type_name}");
        }
        structure.columns[0].type_name = "datetime".into();
        let page_ok = page(rows());
        let table_ok = Table {
            dialect: Dialect::MySql,
            ..table(Some(&structure), &page_ok)
        };
        assert_eq!(table_ok.lock(at(0, 1)), None);
        // SQLite: a key whose text may not have been read exactly.
        let structure = self::structure();
        let page = self::page(vec![vec![text("caf\u{FFFD}"), text("a"), Value::Null]]);
        assert_eq!(
            self::table(Some(&structure), &page).lock(at(0, 1)),
            Some(Lock::KeyInexact)
        );
    }

    #[test]
    fn a_unique_index_is_a_key_when_there_is_no_primary_one() {
        let structure = Structure {
            primary_key: Vec::new(),
            indexes: vec![IndexInfo {
                name: "users_email".into(),
                columns: vec!["email".into()],
                key_columns: Some(vec!["email".into()]),
                unique: true,
                ..IndexInfo::default()
            }],
            columns: vec![
                column("id", "INTEGER"),
                ColumnInfo {
                    nullable: false,
                    ..column("email", "TEXT")
                },
                column("meta", "JSON"),
            ],
            ..Structure::default()
        };
        let page = page(rows());
        let table = table(Some(&structure), &page);
        assert_eq!(table.key(), Some(vec![1]));
        assert_eq!(table.lock(at(0, 1)), Some(Lock::KeyColumn));
        assert_eq!(table.lock(at(0, 0)), None);
    }
}
```

- [ ] **Step 2: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib edit::`
Expected: does not compile (`Table`, `Lock`, `MAX_EDIT_BYTES` not found).

- [ ] **Step 3: Write `src/edit.rs` above the tests**

```rust
//! Editing a table's values: what can be edited, what a column takes, the
//! pending set a tab holds and the change set a save sends. Everything here
//! is decided from the page and the structure alone; the reducer in
//! `app.rs` owns every transition.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use tabletist_db::{
    Access, CellChange, ChangeSet, ColumnClass, ColumnInfo, Dialect, Error, NewValue, ObjectKind,
    ObjectRef, RowChange, RowPage, Structure, Value, column_class,
};

use crate::backend::RequestId;
use crate::model::CellPos;

/// The largest value an editor opens, in bytes of its text: a field that
/// held megabytes would be laid out every frame.
pub const MAX_EDIT_BYTES: usize = 256 * 1024;

/// Why a cell cannot be edited. The view words it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lock {
    /// The connection opens read-only.
    ReadOnly,
    /// A view, a materialized view.
    NotATable,
    /// The row key, the columns' types and what is generated are not known
    /// yet.
    StructureLoading,
    /// No primary key and no unique index that names a row.
    NoKey,
    /// The key has a column of a type a save cannot match exactly.
    KeyType,
    /// A save is running.
    Saving,
    /// The page is being fetched again, and what is on screen is about to
    /// be replaced.
    Refreshing,
    NoSuchCell,
    /// The row's key holds a NULL.
    KeyIsNull,
    /// The row's key holds text that may not have been read exactly.
    KeyInexact,
    /// The structure does not list the column, or the page names it twice.
    UnknownColumn,
    /// Computed by the database.
    Generated,
    /// One of the columns a save finds the row by.
    KeyColumn,
    Binary,
    /// Over `MAX_EDIT_BYTES`.
    TooLarge,
}

/// A table's page as editing sees it.
#[derive(Clone, Copy)]
pub struct Table<'a> {
    pub access: Access,
    pub kind: ObjectKind,
    pub dialect: Dialect,
    pub structure: Option<&'a Structure>,
    pub page: &'a RowPage,
    /// The page is being fetched again.
    pub refreshing: bool,
    pub saving: bool,
}

impl Table<'_> {
    /// The page's columns that make the row key, by their place. `None`
    /// when the table has no key, or the page does not hold all of it.
    pub fn key(&self) -> Option<Vec<usize>> {
        self.structure?
            .row_key()?
            .iter()
            .map(|name| self.place(name))
            .collect()
    }

    /// The one column of the page called `name`. Two of that name are none:
    /// no statement could say which it means.
    fn place(&self, name: &str) -> Option<usize> {
        let mut places = self
            .page
            .columns
            .iter()
            .enumerate()
            .filter(|(_, column)| column.name == name);
        let (place, _) = places.next()?;
        places.next().is_none().then_some(place)
    }

    /// What the structure says about the page's column `col`.
    pub fn column(&self, col: usize) -> Option<&ColumnInfo> {
        let name = &self.page.columns.get(col)?.name;
        self.place(name)?;
        self.structure?
            .columns
            .iter()
            .find(|column| column.name == *name)
    }

    /// The column's class, by the type the structure gives it.
    pub fn class(&self, col: usize) -> Option<ColumnClass> {
        self.column(col)
            .map(|column| column_class(self.dialect, &column.type_name))
    }

    /// Why `cell` cannot be edited, or `None` when it can. The reasons that
    /// hold for the whole table come first, so every cell of such a table
    /// says the same.
    pub fn lock(&self, cell: CellPos) -> Option<Lock> {
        if self.access == Access::ReadOnly {
            return Some(Lock::ReadOnly);
        }
        if self.kind != ObjectKind::Table {
            return Some(Lock::NotATable);
        }
        if self.structure.is_none() {
            return Some(Lock::StructureLoading);
        }
        let Some(key) = self.key() else {
            return Some(Lock::NoKey);
        };
        if key.iter().any(|&col| self.unmatched(col)) {
            return Some(Lock::KeyType);
        }
        if self.saving {
            return Some(Lock::Saving);
        }
        if self.refreshing {
            return Some(Lock::Refreshing);
        }
        let Some(value) = self
            .page
            .rows
            .get(cell.row)
            .and_then(|row| row.get(cell.col))
        else {
            return Some(Lock::NoSuchCell);
        };
        let row = &self.page.rows[cell.row];
        if key.iter().any(|&col| row[col].is_null()) {
            return Some(Lock::KeyIsNull);
        }
        if self.dialect == Dialect::Sqlite
            && key
                .iter()
                .any(|&col| matches!(&row[col], Value::Text(text) if text.contains('\u{FFFD}')))
        {
            return Some(Lock::KeyInexact);
        }
        let Some(column) = self.column(cell.col) else {
            return Some(Lock::UnknownColumn);
        };
        if column.generated {
            return Some(Lock::Generated);
        }
        if key.contains(&cell.col) {
            return Some(Lock::KeyColumn);
        }
        if column_class(self.dialect, &column.type_name) == ColumnClass::Binary
            || matches!(value, Value::Bytes(_))
        {
            return Some(Lock::Binary);
        }
        if matches!(value, Value::Text(text) if text.len() > MAX_EDIT_BYTES) {
            return Some(Lock::TooLarge);
        }
        None
    }

    /// Whether a save could not match a key column of this type exactly.
    /// MySQL shows a TIMESTAMP in the session's zone without it, reads a BIT
    /// bound as bytes as a number, and misses a FLOAT bound as a double; the
    /// save refuses such a key (`tabletist-db`, `mysql/write.rs`).
    fn unmatched(&self, col: usize) -> bool {
        self.dialect == Dialect::MySql
            && self.column(col).is_some_and(|column| {
                let word: String = column
                    .type_name
                    .chars()
                    .take_while(|letter| letter.is_ascii_alphabetic())
                    .collect::<String>()
                    .to_ascii_lowercase();
                matches!(word.as_str(), "timestamp" | "bit" | "float")
            })
    }
}
```

The unused imports (`BTreeMap`, `Duration`, `Instant`, `CellChange`, `ChangeSet`, `Error`, `NewValue`, `ObjectRef`, `RowChange`, `RequestId`) arrive with their first use in tasks 2 and 3: leave out of this task whatever clippy calls unused, and add each back when it is used (the tests of this task need `std::time::Duration` for the page: import it in the test module).

In `src/lib.rs`, beside the other modules:

```rust
pub mod edit;
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib edit::`
Expected: 7 passed.

- [ ] **Step 5: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A && git commit -m "Say which cells of a table can be edited, and why not"
```

---

### Task 2: What a column takes, and what counts as a change

**Files:**
- Modify: `src/edit.rs`

The checks of the spec's "Checks before sending", as data: `check` answers a `Problem` and the view words it. And the two rules a pending set rests on: where an editor starts, and when a new value is a change at all.

- [ ] **Step 1: Write the failing tests**

In the test module of `src/edit.rs`:

```rust
    fn typed(type_name: &str) -> ColumnInfo {
        column("c", type_name)
    }

    #[test]
    fn a_number_column_takes_numbers_its_type_holds() {
        // The names are the structure's: PostgreSQL's `format_type` says
        // `bigint`, not `int8`.
        let pg = |type_name: &str, text: &str| check(Dialect::Postgres, &typed(type_name), text);
        assert_eq!(pg("bigint", "12"), None);
        assert_eq!(pg("bigint", " -12 "), None);
        assert_eq!(pg("bigint", "91000000000000000a1"), Some(Problem::WholeNumber));
        assert_eq!(pg("bigint", "1.5"), Some(Problem::WholeNumber));
        assert_eq!(pg("bigint", ""), Some(Problem::WholeNumber));
        assert_eq!(
            pg("smallint", "40000"),
            Some(Problem::OutOfRange {
                min: -32_768,
                max: 32_767
            })
        );
        assert_eq!(pg("double precision", "1e300"), None);
        assert_eq!(pg("double precision", "NaN"), None);
        assert_eq!(pg("double precision", "-Infinity"), None);
        assert_eq!(pg("double precision", "one"), Some(Problem::Number));
        // Only PostgreSQL has the two words.
        assert_eq!(
            check(Dialect::MySql, &typed("double"), "NaN"),
            Some(Problem::Number)
        );
    }

    #[test]
    fn a_decimal_is_refused_with_what_would_have_been_stored() {
        let pg = |text: &str| check(Dialect::Postgres, &typed("numeric(10,2)"), text);
        assert_eq!(pg("12.5"), None);
        assert_eq!(pg("12.50"), None);
        assert_eq!(pg("-0.05"), None);
        assert_eq!(pg(".5"), None);
        assert_eq!(
            pg("12.505"),
            Some(Problem::Decimals {
                scale: 2,
                stored: "12.51".into()
            })
        );
        assert_eq!(
            pg("-9.999"),
            Some(Problem::Decimals {
                scale: 2,
                stored: "-10.00".into()
            })
        );
        assert_eq!(
            pg("-0.001"),
            Some(Problem::Decimals {
                scale: 2,
                stored: "0.00".into()
            })
        );
        assert_eq!(
            pg("007.505"),
            Some(Problem::Decimals {
                scale: 2,
                stored: "7.51".into()
            })
        );
        assert_eq!(pg("123456789.5"), Some(Problem::Digits { whole: 8 }));
        assert_eq!(pg("1e3"), Some(Problem::Number));
        assert_eq!(pg("twelve"), Some(Problem::Number));
        // Without stated digits, any number.
        assert_eq!(
            check(Dialect::Postgres, &typed("numeric"), "12.505"),
            None
        );
    }

    #[test]
    fn booleans_json_lengths_and_lists_are_checked() {
        let pg = |type_name: &str, text: &str| check(Dialect::Postgres, &typed(type_name), text);
        for ok in ["true", "false", "1", "0", "TRUE"] {
            assert_eq!(pg("boolean", ok), None, "{ok}");
        }
        assert_eq!(pg("boolean", "yes"), Some(Problem::Boolean));
        assert_eq!(pg("jsonb", r#"{"a": [1, 2]}"#), None);
        let Some(Problem::Json { line, column, .. }) = pg("jsonb", "{\n  \"a\": 1\n  \"b\": 2}")
        else {
            panic!("expected a JSON problem");
        };
        assert_eq!((line, column), (3, 3));
        assert_eq!(pg("character varying(5)", "hello"), None);
        assert_eq!(
            pg("character varying(5)", "hello!"),
            Some(Problem::TooLong { max: 5 })
        );
        // Characters, not bytes.
        assert_eq!(pg("character varying(5)", "héllo"), None);
        let listed = ColumnInfo {
            allowed_values: Some(vec!["print".into(), "ebook".into()]),
            ..typed("text")
        };
        assert_eq!(check(Dialect::Postgres, &listed, "ebook"), None);
        assert_eq!(
            check(Dialect::Postgres, &listed, "audio"),
            Some(Problem::NotOneOf(vec!["print".into(), "ebook".into()]))
        );
        // A type the app does not know has no check.
        assert_eq!(pg("tsvector", "anything"), None);
    }

    #[test]
    fn sqlite_checks_by_affinity_and_never_a_length() {
        let lite = |type_name: &str, text: &str| check(Dialect::Sqlite, &typed(type_name), text);
        assert_eq!(lite("INTEGER", "12"), None);
        assert_eq!(lite("INTEGER", "1.5"), Some(Problem::WholeNumber));
        assert_eq!(lite("VARCHAR(3)", "longer"), None);
        assert_eq!(lite("NUMERIC(10,2)", "12.505"), None);
        assert_eq!(lite("", "anything"), None);
    }

    #[test]
    fn an_editor_starts_from_the_whole_value() {
        let text_class = ColumnClass::Text { max_chars: None };
        assert_eq!(start_text(&Value::Null, text_class), "");
        assert_eq!(start_text(&text("a\nb"), text_class), "a\nb");
        assert_eq!(start_text(&Value::Float(0.1), ColumnClass::Float), "0.1");
        // A boolean reads true or false whatever the driver loaded.
        assert_eq!(start_text(&Value::Int(1), ColumnClass::Boolean), "true");
        assert_eq!(start_text(&Value::Int(0), ColumnClass::Boolean), "false");
        assert_eq!(start_text(&Value::Bool(true), ColumnClass::Boolean), "true");
        // But not what is no flag: MySQL keeps other numbers in a tinyint(1).
        assert_eq!(start_text(&Value::Int(5), ColumnClass::Boolean), "5");
    }

    #[test]
    fn a_value_is_a_change_only_when_it_differs_from_where_the_editor_starts() {
        let class = ColumnClass::Text { max_chars: None };
        let loaded = text("ada");
        assert!(!is_change(&loaded, &NewValue::Text("ada".into()), class));
        assert!(is_change(&loaded, &NewValue::Text("Ada".into()), class));
        assert!(is_change(&loaded, &NewValue::Null, class));
        assert!(!is_change(&Value::Null, &NewValue::Null, class));
        // The empty string is not NULL.
        assert!(is_change(&Value::Null, &NewValue::Text(String::new()), class));
        assert!(!is_change(
            &Value::Int(1),
            &NewValue::Text("true".into()),
            ColumnClass::Boolean
        ));
    }

    #[test]
    fn long_broken_and_json_values_open_the_large_editor() {
        let plain = ColumnClass::Text { max_chars: None };
        assert!(!opens_large("short", plain));
        assert!(opens_large("two\nlines", plain));
        assert!(opens_large(&"x".repeat(257), plain));
        assert!(!opens_large(&"x".repeat(256), plain));
        assert!(opens_large("{}", ColumnClass::Json));
    }
```

- [ ] **Step 2: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib edit::`
Expected: does not compile (`check`, `Problem`, `start_text`, `is_change`, `opens_large` not found).

- [ ] **Step 3: Write the checks**

In `src/edit.rs`, after `impl Table`:

```rust
/// Why a text is not a value its column takes. Found before anything is
/// sent; every other rule is the database's, and its refusal is a failed
/// save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// Not a whole number.
    WholeNumber,
    /// A whole number the type does not hold.
    OutOfRange { min: i128, max: i128 },
    /// Not a number.
    Number,
    /// More decimals than the type keeps: `stored` is what the database
    /// would have rounded it to.
    Decimals { scale: u32, stored: String },
    /// More digits before the point than the type holds: at most `whole`.
    Digits { whole: u32 },
    /// Not `true`, `false`, `1` or `0`.
    Boolean,
    /// Not one of the values the column allows.
    NotOneOf(Vec<String>),
    /// It does not parse: what the parser expected, and where.
    Json {
        message: String,
        line: usize,
        column: usize,
    },
    /// More than `max` characters.
    TooLong { max: u32 },
}

/// Whether `text` is a value `column` takes, as far as the app can tell
/// from its type's name. A type it does not know has no check.
pub fn check(dialect: Dialect, column: &ColumnInfo, text: &str) -> Option<Problem> {
    if let Some(allowed) = &column.allowed_values {
        return (!allowed.iter().any(|value| value == text))
            .then(|| Problem::NotOneOf(allowed.clone()));
    }
    let typed = text.trim();
    match column_class(dialect, &column.type_name) {
        ColumnClass::Integer { min, max } => match typed.parse::<i128>() {
            Err(_) => Some(Problem::WholeNumber),
            Ok(number) if number < min || number > max => Some(Problem::OutOfRange { min, max }),
            Ok(_) => None,
        },
        ColumnClass::Decimal { precision, scale } => decimal(typed, precision, scale),
        ColumnClass::Float => {
            let word = dialect == Dialect::Postgres
                && matches!(typed, "NaN" | "Infinity" | "-Infinity");
            let number = typed.parse::<f64>().is_ok_and(f64::is_finite);
            (!word && !number).then_some(Problem::Number)
        }
        ColumnClass::Boolean => (!matches!(
            typed.to_ascii_lowercase().as_str(),
            "true" | "false" | "1" | "0"
        ))
        .then_some(Problem::Boolean),
        ColumnClass::Json => serde_json::from_str::<serde::de::IgnoredAny>(text)
            .err()
            .map(|error| Problem::Json {
                message: json_message(&error.to_string()),
                line: error.line(),
                column: error.column(),
            }),
        ColumnClass::Text {
            max_chars: Some(max),
        } => (text.chars().count() > max as usize).then_some(Problem::TooLong { max }),
        ColumnClass::Text { max_chars: None } | ColumnClass::Binary | ColumnClass::Other => None,
    }
}

/// The parser's words without its own "at line 3 column 23", which the
/// view writes as `3:23`.
fn json_message(error: &str) -> String {
    error
        .split(" at line ")
        .next()
        .unwrap_or(error)
        .replace('`', "")
}

/// A plain decimal number within the digits and the scale its type states.
/// No exponent: the databases take one, but what it would be stored as is
/// not what the user sees typed.
fn decimal(typed: &str, precision: Option<u32>, scale: Option<u32>) -> Option<Problem> {
    let (negative, digits) = match typed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, typed.strip_prefix('+').unwrap_or(typed)),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty()) || !all_digits(whole) || !all_digits(fraction) {
        return Some(Problem::Number);
    }
    let scale = scale?;
    let kept = fraction.trim_end_matches('0');
    if kept.len() > scale as usize {
        return Some(Problem::Decimals {
            scale,
            stored: rounded(negative, whole, fraction, scale as usize),
        });
    }
    let whole_digits = whole.trim_start_matches('0').len() as u32;
    match precision {
        Some(precision) if whole_digits > precision.saturating_sub(scale) => {
            Some(Problem::Digits {
                whole: precision.saturating_sub(scale),
            })
        }
        _ => None,
    }
}

/// `whole.fraction` rounded half away from zero to `scale` decimals, as the
/// databases round a decimal. `fraction` is longer than `scale`.
fn rounded(negative: bool, whole: &str, fraction: &str, scale: usize) -> String {
    let whole = if whole.is_empty() { "0" } else { whole };
    let mut digits: Vec<u8> = whole
        .bytes()
        .chain(fraction.bytes().take(scale))
        .map(|byte| byte - b'0')
        .collect();
    if fraction.as_bytes()[scale] >= b'5' {
        let mut place = digits.len();
        loop {
            if place == 0 {
                digits.insert(0, 1);
                break;
            }
            place -= 1;
            if digits[place] == 9 {
                digits[place] = 0;
            } else {
                digits[place] += 1;
                break;
            }
        }
    }
    // As a number is written: no zeros ahead of it but the one before the
    // point, and no sign on zero.
    while digits.len() > scale + 1 && digits[0] == 0 {
        digits.remove(0);
    }
    let point = digits.len() - scale;
    let mut text = String::with_capacity(digits.len() + 2);
    if negative && digits.iter().any(|&digit| digit != 0) {
        text.push('-');
    }
    for (place, digit) in digits.iter().enumerate() {
        if place == point && scale > 0 {
            text.push('.');
        }
        text.push(char::from(b'0' + digit));
    }
    text
}

/// Where an editor starts: the whole value as the database gave it, never
/// the shortened text a cell shows. Empty on NULL. A boolean column reads
/// `true` or `false` whatever the driver loaded (SQLite and MySQL hold 1
/// and 0).
pub fn start_text(value: &Value, class: ColumnClass) -> String {
    match (value, class) {
        (Value::Null, _) => String::new(),
        (Value::Int(1), ColumnClass::Boolean) => "true".to_owned(),
        (Value::Int(0), ColumnClass::Boolean) => "false".to_owned(),
        (value, _) => crate::ui::format::plain_text(value),
    }
}

/// Whether `new` differs from what the cell loaded: a text equal to where
/// the editor starts, or NULL on a NULL cell, is no change.
pub fn is_change(loaded: &Value, new: &NewValue, class: ColumnClass) -> bool {
    match new {
        NewValue::Null => !loaded.is_null(),
        NewValue::Text(text) => loaded.is_null() || *text != start_text(loaded, class),
    }
}

/// Whether a value is edited in the large editor rather than on its cell:
/// every JSON column, and text with a line break or longer than the grid's
/// cut.
pub fn opens_large(text: &str, class: ColumnClass) -> bool {
    class == ColumnClass::Json
        || text.contains('\n')
        || text.chars().count() > crate::ui::format::CELL_MAX_CHARS
}
```

`serde` and `serde_json` are already dependencies of the app (`Cargo.toml`). If `serde::de::IgnoredAny` is not reachable there, parse into `serde_json::Value` instead.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib edit::`
Expected: 14 passed. The type names in these tests are the ones `column_class` knows (`crates/tabletist-db/src/class.rs`); if one of them classes as `Other` there, the test is wrong, not the classifier. If the JSON test's column differs by one, read what `serde_json` reports for that input and assert that: the plan guessed, the parser knows.

- [ ] **Step 5: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A && git commit -m "Check a typed value against what its column takes"
```

---
### Task 3: The pending set and the editor, in the model

**Files:**
- Modify: `src/edit.rs` (`Edits` and its parts), `src/model.rs` (`ObjectTab::edits`, the actions), `src/app.rs` (the arms), `src/testing.rs` (helpers)
- Test: the test modules of `src/edit.rs` and `src/app.rs`

No view yet: the actions are sent by tests. After this task a test can open an editor on a cell, commit it, and see the cell pending.

- [ ] **Step 1: The test helpers**

In `src/testing.rs`, beside `page`:

```rust
/// The structure of the table `page` holds: `id` is the primary key,
/// `email` is NOT NULL, `meta` is JSON and may be NULL.
pub fn fixture_structure() -> tabletist_db::Structure {
    let column = |name: &str, type_name: &str, nullable: bool| tabletist_db::ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        nullable,
        ..tabletist_db::ColumnInfo::default()
    };
    tabletist_db::Structure {
        columns: vec![
            column("id", "INTEGER", false),
            column("email", "TEXT", false),
            column("meta", "JSON", true),
        ],
        primary_key: vec!["id".into()],
        ..tabletist_db::Structure::default()
    }
}
```

and in `impl Harness`:

```rust
    /// A writable connection with `users` open and pinned, its structure
    /// and a page of five rows loaded: a table whose cells can be edited.
    /// The saved connection itself is writable, so a reconnect comes back
    /// writable too.
    pub fn editable(&mut self) -> (crate::model::ConnTabId, crate::model::TabId) {
        let tab = self.connect_fake_as(false);
        self.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        // The describe was sent before the rows were asked for.
        self.answer_structure(fixture_structure());
        self.answer_rows(page(5, false));
        let id = self
            .app
            .workspace(tab)
            .and_then(|workspace| workspace.active_tab)
            .expect("the table's tab is open");
        (tab, id)
    }
```

`connect_fake_as(read_only: bool)` is `connect_fake` with the saved fixture's `read_only` box set to `Some(read_only)` before it connects (the access a session gets is read from that box at every connect, see `send_connect`); `connect_fake` becomes `self.connect_fake_as(true)`. Setting `workspace.access` alone, as some older tests do, would not survive a reconnect. No `unwrap` here: this file is not test code to clippy (`unwrap_used` is on), use `expect` as `add_sql_tab` does.

Check `answer_structure` and `answer_rows`: each answers the newest request of its kind, so the order above holds. Use the imports the file already has.

- [ ] **Step 2: Write the failing tests**

In the test module of `src/app.rs`, after the table tab tests (near `fn open`):

```rust
    use crate::edit::{Lock, Problem, State};
    use crate::model::{Advance, EditStart};
    use tabletist_db::NewValue;

    fn at(row: usize, col: usize) -> CellPos {
        CellPos { row, col }
    }

    /// Opens the editor on `cell`, sets its text as a field would, and
    /// commits.
    fn type_into(harness: &mut Harness, tab: ConnTabId, id: TabId, cell: CellPos, text: &str) {
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell,
            start: EditStart::Value,
        });
        let object = harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .object_tab_mut(id)
            .unwrap();
        object.edits.editor.as_mut().expect("an editor").text = text.to_owned();
        // What a field says when its text changed.
        harness.app.apply(Action::EditorTyped { tab, id });
        harness.app.apply(Action::CommitEdit {
            tab,
            id,
            then: Advance::Stay,
        });
    }

    #[test]
    fn a_committed_edit_is_pending_and_the_loaded_text_takes_it_out_again() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: at(1, 1),
            start: EditStart::Value,
        });
        // The editor starts from the whole loaded value.
        let editor = object(&harness, tab, id).edits.editor.as_ref().unwrap();
        assert_eq!(
            (editor.cell, editor.text.as_str(), editor.large),
            (at(1, 1), "user2@example.com", false)
        );
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        let edits = &object(&harness, tab, id).edits;
        assert!(edits.editor.is_none());
        assert_eq!(
            edits.cells.get(&(1, 1)).map(|cell| &cell.new),
            Some(&NewValue::Text("bob@example.com".into()))
        );
        assert_eq!(edits.counts().changes, 1);
        // The page itself is untouched: nothing has been sent.
        assert_eq!(
            object(&harness, tab, id).page().unwrap().rows[1][1],
            tabletist_db::Value::Text("user2@example.com".into())
        );
        // Opened again, the editor holds the pending text.
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: at(1, 1),
            start: EditStart::Value,
        });
        assert_eq!(
            object(&harness, tab, id).edits.editor.as_ref().unwrap().text,
            "bob@example.com"
        );
        // Typing the loaded text back is no change.
        type_into(&mut harness, tab, id, at(1, 1), "user2@example.com");
        assert!(object(&harness, tab, id).edits.cells.is_empty());
    }

    #[test]
    fn a_locked_cell_opens_no_editor_and_says_why() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: at(0, 0),
            start: EditStart::Value,
        });
        let edits = &object(&harness, tab, id).edits;
        assert!(edits.editor.is_none());
        assert_eq!(edits.why, Some((at(0, 0), Lock::KeyColumn)));
        // Moving on forgets the note.
        harness.app.apply(Action::SelectCell {
            tab,
            id,
            cell: at(0, 1),
        });
        assert_eq!(object(&harness, tab, id).edits.why, None);
        // A read-only connection locks every cell.
        harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::ReadOnly;
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: at(0, 1),
            start: EditStart::Value,
        });
        assert_eq!(
            object(&harness, tab, id).edits.why,
            Some((at(0, 1), Lock::ReadOnly))
        );
    }

    #[test]
    fn a_text_that_fails_its_check_stays_in_the_editor_and_is_kept_when_left() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        // `meta` is JSON, so its editor is the large one.
        type_into(&mut harness, tab, id, at(1, 2), "{oops");
        let editor = object(&harness, tab, id).edits.editor.as_ref().unwrap();
        assert!(editor.large);
        assert!(matches!(editor.problem, Some(Problem::Json { .. })));
        assert!(object(&harness, tab, id).edits.cells.is_empty());
        // Clicking elsewhere never loses the typing: the cell is to fix.
        harness.app.apply(Action::SelectCell {
            tab,
            id,
            cell: at(0, 1),
        });
        let edits = &object(&harness, tab, id).edits;
        assert!(edits.editor.is_none());
        assert!(matches!(
            edits.cells.get(&(1, 2)).map(|cell| &cell.state),
            Some(State::ToFix(Problem::Json { .. }))
        ));
        assert_eq!(edits.counts().to_fix, 1);
    }

    #[test]
    fn cancel_drops_the_edit_and_typed_marks_the_problem_as_it_is_typed() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: at(1, 2),
            start: EditStart::Replace("{".into()),
        });
        harness.app.apply(Action::EditorTyped { tab, id });
        assert!(
            object(&harness, tab, id)
                .edits
                .editor
                .as_ref()
                .unwrap()
                .problem
                .is_some()
        );
        harness.app.apply(Action::CancelEdit { tab, id });
        let edits = &object(&harness, tab, id).edits;
        assert!(edits.editor.is_none() && edits.cells.is_empty());
    }

    #[test]
    fn null_is_set_only_where_the_column_allows_it_and_one_cell_is_reverted() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        // `email` is NOT NULL: the key does nothing.
        harness.app.apply(Action::SelectCell {
            tab,
            id,
            cell: at(0, 1),
        });
        harness.app.apply(Action::SetNull { tab, id });
        assert!(object(&harness, tab, id).edits.cells.is_empty());
        // `meta` of the first row holds a value and may be NULL.
        harness.app.apply(Action::SelectCell {
            tab,
            id,
            cell: at(0, 2),
        });
        harness.app.apply(Action::SetNull { tab, id });
        assert_eq!(
            object(&harness, tab, id)
                .edits
                .cells
                .get(&(0, 2))
                .map(|cell| &cell.new),
            Some(&NewValue::Null)
        );
        // NULL on a cell that was NULL is no change.
        harness.app.apply(Action::SelectCell {
            tab,
            id,
            cell: at(1, 2),
        });
        harness.app.apply(Action::SetNull { tab, id });
        assert_eq!(object(&harness, tab, id).edits.counts().changes, 1);
        // Reverting the active cell puts the loaded value back.
        harness.app.apply(Action::SelectCell {
            tab,
            id,
            cell: at(0, 2),
        });
        harness.app.apply(Action::RevertCell { tab, id });
        assert!(object(&harness, tab, id).edits.cells.is_empty());
    }

    #[test]
    fn an_editor_that_was_only_opened_changes_nothing() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        // On a NULL cell the editor starts empty, and the empty string is
        // not NULL: only typing makes it a change.
        for then in [Advance::Stay, Advance::Down] {
            harness.app.apply(Action::EditCell {
                tab,
                id,
                cell: at(1, 2),
                start: EditStart::Value,
            });
            harness.app.apply(Action::CommitEdit { tab, id, then });
            assert!(object(&harness, tab, id).edits.cells.is_empty());
        }
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: at(1, 2),
            start: EditStart::Value,
        });
        harness.app.apply(Action::LeaveEdit { tab, id });
        assert!(!object(&harness, tab, id).edits.holds());
        // A cell made NULL stays NULL when its editor is opened and left.
        harness.app.apply(Action::SelectCell {
            tab,
            id,
            cell: at(0, 2),
        });
        harness.app.apply(Action::SetNull { tab, id });
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: at(0, 2),
            start: EditStart::Value,
        });
        harness.app.apply(Action::CommitEdit {
            tab,
            id,
            then: Advance::Stay,
        });
        assert_eq!(
            object(&harness, tab, id)
                .edits
                .cells
                .get(&(0, 2))
                .map(|cell| &cell.new),
            Some(&NewValue::Null)
        );
    }

    #[test]
    fn typing_on_a_locked_cell_does_nothing() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: at(0, 0),
            start: EditStart::Typed("7".into()),
        });
        let edits = &object(&harness, tab, id).edits;
        assert!(edits.editor.is_none());
        assert_eq!(edits.why, None);
    }

    #[test]
    fn commit_moves_on_and_discard_empties_the_set() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: at(1, 1),
            start: EditStart::Replace("x@example.com".into()),
        });
        harness.app.apply(Action::CommitEdit {
            tab,
            id,
            then: Advance::Down,
        });
        assert_eq!(object(&harness, tab, id).selection, Some(at(2, 1)));
        type_into(&mut harness, tab, id, at(2, 1), "y@example.com");
        assert_eq!(
            (
                object(&harness, tab, id).edits.counts().changes,
                object(&harness, tab, id).edits.counts().rows
            ),
            (2, 2)
        );
        harness.app.apply(Action::DiscardEdits { tab, id });
        assert!(!object(&harness, tab, id).edits.holds());
    }

    #[test]
    fn editing_pins_a_preview_tab() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .object_tab_mut(id)
            .unwrap()
            .pinned = false;
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: at(1, 1),
            start: EditStart::Value,
        });
        assert!(object(&harness, tab, id).pinned);
    }
```

And in `src/edit.rs`'s tests:

```rust
    #[test]
    fn the_set_is_printed_without_what_was_typed() {
        let mut edits = Edits::default();
        edits.cells.insert(
            (0, 1),
            Pending {
                new: NewValue::Text("a secret".into()),
                state: State::Ready,
            },
        );
        edits.editor = Some(Editor {
            cell: at(0, 2),
            text: "another secret".into(),
            large: false,
            focus: false,
            touched: true,
            problem: None,
        });
        let printed = format!("{edits:?}");
        assert!(!printed.contains("secret"), "{printed}");
        assert!(printed.contains("cells: 1"), "{printed}");
    }
```

- [ ] **Step 3: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: does not compile (`Action::EditCell`, `EditStart`, `Advance`, `ObjectTab::edits`, `Edits`).

- [ ] **Step 4: The set, in `src/edit.rs`**

```rust
/// One cell's new value, not yet written.
#[derive(Clone, PartialEq)]
pub struct Pending {
    pub new: NewValue,
    pub state: State,
}

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    /// Waits for a save.
    Ready,
    /// Fails its check: a save is not offered until it is fixed.
    ToFix(Problem),
    /// The last save's statement for its row failed. It is sent again by
    /// the next save.
    Failed(Error),
}

/// The editor that is open. Only `text` is the view's to change.
pub struct Editor {
    pub cell: CellPos,
    pub text: String,
    /// The popover rather than the field on the cell.
    pub large: bool,
    /// Taken by the view when it gives the field the keyboard.
    pub focus: bool,
    /// Whether the text was typed into. An editor that was only opened and
    /// closed changes nothing: on a NULL cell it starts empty, and the
    /// empty string is not NULL.
    pub touched: bool,
    /// What the text fails, kept up to date by `Action::EditorTyped`.
    pub problem: Option<Problem>,
}

/// What a table's tab holds while its values are edited.
#[derive(Default)]
pub struct Edits {
    /// By row and column of the loaded page.
    pub cells: BTreeMap<(usize, usize), Pending>,
    pub editor: Option<Editor>,
    /// Why the cell last asked for could not be edited.
    pub why: Option<(CellPos, Lock)>,
}

/// How much is pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub changes: usize,
    pub rows: usize,
    pub to_fix: usize,
    pub failed: usize,
}

/// What a row's cells come to, for its mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowMark {
    #[default]
    None,
    Changed,
    /// One of its cells is to fix or failed.
    Trouble,
}

impl Edits {
    /// Whether the tab's page must stay: something is pending, or an
    /// editor is open.
    pub fn holds(&self) -> bool {
        !self.cells.is_empty() || self.editor.is_some()
    }

    pub fn counts(&self) -> Counts {
        let mut counts = Counts {
            changes: self.cells.len(),
            ..Counts::default()
        };
        let mut last = None;
        for (&(row, _), cell) in &self.cells {
            if last != Some(row) {
                counts.rows += 1;
                last = Some(row);
            }
            match cell.state {
                State::Ready => {}
                State::ToFix(_) => counts.to_fix += 1,
                State::Failed(_) => counts.failed += 1,
            }
        }
        counts
    }

    pub fn row_mark(&self, row: usize) -> RowMark {
        let mut cells = self.cells.range((row, 0)..=(row, usize::MAX)).peekable();
        if cells.peek().is_none() {
            RowMark::None
        } else if cells.any(|(_, cell)| cell.state != State::Ready) {
            RowMark::Trouble
        } else {
            RowMark::Changed
        }
    }
}

/// Without the texts: what a user typed stays out of logs and panics.
impl std::fmt::Debug for Edits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Edits {{ cells: {}, editor: {:?}, why: {:?} }}",
            self.cells.len(),
            self.editor.as_ref().map(|editor| editor.cell),
            self.why
        )
    }
}
```

- [ ] **Step 5: The model**

In `src/model.rs`:

- `ObjectTab` gains `pub edits: crate::edit::Edits,` (documented: "What is pending, while the tab's values are edited. A tab that holds edits keeps its page."), set to `Edits::default()` in `ObjectTab::new`.
- Beside `CellPos`:

```rust
/// Where an editor starts.
#[derive(Debug)]
pub enum EditStart {
    /// From the cell's value, or its pending one, the cursor at the end.
    Value,
    /// From this text, with nothing of the old value (Omarchy's `cc`).
    Replace(String),
    /// From the character that was typed on the cell. On a cell that cannot
    /// be edited this does nothing: only an edit that was asked for says
    /// why.
    Typed(String),
}

/// Where the selection goes after an edit is committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advance {
    Stay,
    Down,
    Right,
    Left,
}
```

- `Action` gains, beside `SelectCell`:

```rust
    /// Open the editor on a cell of a table's grid, or say why it cannot
    /// be edited.
    EditCell {
        tab: ConnTabId,
        id: TabId,
        cell: CellPos,
        start: EditStart,
    },
    /// The editor's text changed: check it again.
    EditorTyped { tab: ConnTabId, id: TabId },
    /// Take the editor's text as the cell's new value, if its column takes
    /// it, and move on.
    CommitEdit {
        tab: ConnTabId,
        id: TabId,
        then: Advance,
    },
    /// The editor lost the keyboard: keep its text, as a cell to fix when
    /// its column does not take it.
    LeaveEdit { tab: ConnTabId, id: TabId },
    /// Close the editor and drop its text.
    CancelEdit { tab: ConnTabId, id: TabId },
    /// Add a line break and move the text into the large editor.
    EditorBreak { tab: ConnTabId, id: TabId },
    /// Make the active cell NULL, where its column allows it.
    SetNull { tab: ConnTabId, id: TabId },
    /// Put back the active cell's loaded value.
    RevertCell { tab: ConnTabId, id: TabId },
    /// Drop every pending change of the tab.
    DiscardEdits { tab: ConnTabId, id: TabId },
```

- [ ] **Step 6: The reducer**

In `src/app.rs`, a private helper beside `object_tab_mut`:

```rust
    /// What editing may know of a table tab: `None` while it has no page.
    fn table<T>(
        &self,
        tab: ConnTabId,
        id: TabId,
        read: impl FnOnce(&crate::edit::Table<'_>, &ObjectTab) -> T,
    ) -> Option<T> {
        let workspace = self.workspace(tab)?;
        let object = workspace.object_tab(id)?;
        let table = crate::edit::Table {
            access: workspace.access,
            kind: object.kind,
            dialect: workspace.driver.dialect(),
            structure: object.structure.value.as_ref(),
            page: object.page()?,
            refreshing: object.rows.is_loading(),
            saving: false,
        };
        Some(read(&table, object))
    }
```

(`saving: false` until task 4 gives the tab a save in flight.)

The arms, each a call:

```rust
            Action::EditCell {
                tab,
                id,
                cell,
                start,
            } => self.edit_cell(tab, id, cell, start),
            Action::EditorTyped { tab, id } => {
                let problem = self.editor_problem(tab, id);
                if let Some(editor) = self.editor_mut(tab, id) {
                    editor.problem = problem;
                    editor.touched = true;
                }
            }
            Action::CommitEdit { tab, id, then } => {
                if self.close_editor(tab, id, false) {
                    let (rows, cols) = match then {
                        Advance::Stay => (0, 0),
                        Advance::Down => (1, 0),
                        Advance::Right => (0, 1),
                        Advance::Left => (0, -1),
                    };
                    if (rows, cols) != (0, 0) {
                        self.apply(Action::MoveSelection {
                            tab,
                            id,
                            rows,
                            cols,
                        });
                    }
                }
            }
            Action::LeaveEdit { tab, id } => {
                self.close_editor(tab, id, true);
            }
            Action::CancelEdit { tab, id } => {
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.editor = None;
                }
            }
            Action::EditorBreak { tab, id } => {
                if let Some(editor) = self.editor_mut(tab, id) {
                    // At the end of the text, where the cursor of a field
                    // that just opened is. A break elsewhere is typed in
                    // the large editor.
                    editor.text.push('\n');
                    editor.large = true;
                    editor.focus = true;
                    editor.touched = true;
                }
            }
            Action::SetNull { tab, id } => self.set_null(tab, id),
            Action::RevertCell { tab, id } => {
                // Not under a save: its answer is put into this set.
                if let Some(object) = self.object_tab_mut(tab, id)
                    && let Some(cell) = object.selection
                    && object.edits.editor.is_none()
                    && object.edits.saving.is_none()
                {
                    object.edits.cells.remove(&(cell.row, cell.col));
                    object.fields = None;
                }
            }
            Action::DiscardEdits { tab, id } => {
                // Not under a save: dropping the set would drop the save
                // with it, and its answer would find no tab to tell.
                if let Some(object) = self.object_tab_mut(tab, id)
                    && object.edits.saving.is_none()
                {
                    object.edits = crate::edit::Edits::default();
                    object.fields = None;
                }
            }
```

(`edits.saving` arrives in task 4: write the two `saving` conditions there, and add to task 4's tests that Discard and Revert do nothing while a save runs.)

and the methods:

```rust
    fn editor_mut(&mut self, tab: ConnTabId, id: TabId) -> Option<&mut crate::edit::Editor> {
        self.object_tab_mut(tab, id)?.edits.editor.as_mut()
    }

    /// What the open editor's text fails, if anything.
    fn editor_problem(&self, tab: ConnTabId, id: TabId) -> Option<crate::edit::Problem> {
        self.table(tab, id, |table, object| {
            let editor = object.edits.editor.as_ref()?;
            let column = table.column(editor.cell.col)?;
            crate::edit::check(table.dialect, column, &editor.text)
        })
        .flatten()
    }

    fn edit_cell(&mut self, tab: ConnTabId, id: TabId, cell: CellPos, start: EditStart) {
        // An editor open on another cell keeps its text.
        self.close_editor(tab, id, true);
        let (asked, touched) = match &start {
            EditStart::Value => (true, false),
            EditStart::Replace(_) => (true, true),
            EditStart::Typed(_) => (false, true),
        };
        let opened = self.table(tab, id, |table, object| {
            if let Some(lock) = table.lock(cell) {
                return Err(lock);
            }
            let class = table
                .class(cell.col)
                .unwrap_or(tabletist_db::ColumnClass::Other);
            let text = match start {
                EditStart::Replace(text) | EditStart::Typed(text) => text,
                EditStart::Value => match object.edits.cells.get(&(cell.row, cell.col)) {
                    Some(pending) => match &pending.new {
                        tabletist_db::NewValue::Text(text) => text.clone(),
                        tabletist_db::NewValue::Null => String::new(),
                    },
                    None => crate::edit::start_text(&table.page.rows[cell.row][cell.col], class),
                },
            };
            Ok(crate::edit::Editor {
                cell,
                large: crate::edit::opens_large(&text, class),
                text,
                focus: true,
                touched,
                problem: None,
            })
        });
        let Some(opened) = opened else {
            return;
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        match opened {
            Ok(editor) => {
                object.selection = Some(cell);
                object.edits.editor = Some(editor);
                object.edits.why = None;
                // A tab being edited is no preview to replace.
                object.pinned = true;
            }
            Err(crate::edit::Lock::NoSuchCell) => return,
            // Typing on a cell that cannot be edited does nothing.
            Err(_) if !asked => return,
            Err(lock) => {
                object.selection = Some(cell);
                object.edits.why = Some((cell, lock));
            }
        }
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.pane = Pane::Grid;
        }
    }

    /// Takes the open editor's text as its cell's new value and closes it.
    /// A text its column does not take keeps the editor open, unless the
    /// edit is `left` (the keyboard went elsewhere): then the text is kept
    /// as a cell to fix, so typing is never lost. Says whether the editor
    /// closed.
    fn close_editor(&mut self, tab: ConnTabId, id: TabId, left: bool) -> bool {
        let verdict = self.table(tab, id, |table, object| {
            let editor = object.edits.editor.as_ref()?;
            let cell = editor.cell;
            let column = table.column(cell.col)?;
            let class = tabletist_db::column_class(table.dialect, &column.type_name);
            let loaded = table.page.rows.get(cell.row)?.get(cell.col)?;
            if !editor.touched {
                return None;
            }
            let new = tabletist_db::NewValue::Text(editor.text.clone());
            let changed = crate::edit::is_change(loaded, &new, class);
            let problem = changed
                .then(|| crate::edit::check(table.dialect, column, &editor.text))
                .flatten();
            Some((cell, new, changed, problem))
        });
        let Some(object) = self.object_tab_mut(tab, id) else {
            return false;
        };
        let Some(Some((cell, new, changed, problem))) = verdict else {
            // Nothing was typed, or there is no page or no such cell any
            // more: the editor closes and the set stays as it was.
            return object.edits.editor.take().is_some();
        };
        if problem.is_some() && !left {
            if let Some(editor) = object.edits.editor.as_mut() {
                editor.problem = problem;
            }
            return false;
        }
        object.edits.editor = None;
        let key = (cell.row, cell.col);
        if changed {
            let state = problem.map_or(crate::edit::State::Ready, crate::edit::State::ToFix);
            object
                .edits
                .cells
                .insert(key, crate::edit::Pending { new, state });
        } else {
            object.edits.cells.remove(&key);
        }
        // The row panel shows the pending value.
        object.fields = None;
        true
    }

    fn set_null(&mut self, tab: ConnTabId, id: TabId) {
        let verdict = self.table(tab, id, |table, object| {
            let cell = object.selection?;
            if object.edits.editor.is_some() || table.lock(cell).is_some() {
                return None;
            }
            let column = table.column(cell.col)?;
            if !column.nullable {
                return None;
            }
            Some((cell, !table.page.rows[cell.row][cell.col].is_null()))
        });
        let Some(Some((cell, changed))) = verdict else {
            return;
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let key = (cell.row, cell.col);
        if changed {
            object.edits.cells.insert(
                key,
                crate::edit::Pending {
                    new: tabletist_db::NewValue::Null,
                    state: crate::edit::State::Ready,
                },
            );
        } else {
            object.edits.cells.remove(&key);
        }
        object.fields = None;
    }
```

In the `SelectCell` and `MoveSelection` arms, for a table tab, first leave the editor and forget the note: at the top of each arm,

```rust
                self.close_editor(tab, id, true);
                if let Some(object) = self.object_tab_mut(tab, id) {
                    object.edits.why = None;
                }
```

(`close_editor` answers `false` on a SQL tab, which has no editor.)

- [ ] **Step 7: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: the ten new tests pass, and every test that was green stays green.

- [ ] **Step 8: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A && git commit -m "Hold a table tab's pending cells and its open editor"
```

---
### Task 4: Save

**Files:**
- Modify: `src/edit.rs` (`change_set`, `Saving`, `Saved`, `Note`), `src/model.rs` (`Action::WriteEdits`, `ObjectTab::pending`), `src/app.rs` (`write_edits`, `Event::Written`, the session swap), `src/testing.rs` (`answer_written`)
- Test: `src/edit.rs`, `src/app.rs`, and `an_object_tab_waits_for_everything_it_loads` in `src/model.rs`

`Action::WriteEdits` turns the pending set into a `ChangeSet`, sends `Command::Write`, and `Event::Written` puts the answer into the tab. The production confirmation and the guard are task 5; here a save is sent at once.

- [ ] **Step 1: The helper**

In `impl Harness` (`src/testing.rs`):

```rust
    /// Answers the newest `Write` sent.
    pub fn answer_written(
        &mut self,
        result: Result<tabletist_db::WriteOutcome, tabletist_db::Error>,
    ) {
        let (session, request) = self
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::Write {
                    session, request, ..
                } => Some((*session, *request)),
                _ => None,
            })
            .expect("a Write was sent");
        self.app.apply(Action::Backend(Event::Written {
            session,
            request,
            result,
        }));
    }
```

- [ ] **Step 2: Write the failing tests**

In `src/edit.rs`'s tests:

```rust
    #[test]
    fn the_change_set_names_each_row_by_its_key_and_carries_what_was_loaded() {
        let (structure, page) = (structure(), page(rows()));
        let table = table(Some(&structure), &page);
        let ready = |new: NewValue| Pending {
            new,
            state: State::Ready,
        };
        let mut cells = BTreeMap::new();
        cells.insert((1, 2), ready(NewValue::Null));
        cells.insert((1, 1), ready(NewValue::Text("b@example.com".into())));
        cells.insert((0, 1), ready(NewValue::Text("a@example.com".into())));
        let (changes, places) =
            change_set(&ObjectRef::new("main", "users"), &table, &cells).unwrap();
        // One change per row, in the page's order, and where each came from.
        assert_eq!(places, [0, 1]);
        assert_eq!(changes.rows.len(), 2);
        assert_eq!(changes.rows[0].key, [("id".to_owned(), Value::Int(1))]);
        let second = &changes.rows[1];
        assert_eq!(second.key, [("id".to_owned(), Value::Int(2))]);
        assert_eq!(
            second
                .set
                .iter()
                .map(|cell| (cell.column.as_str(), cell.type_name.as_str()))
                .collect::<Vec<_>>(),
            [("email", "TEXT"), ("meta", "JSON")]
        );
        assert_eq!(second.set[1].loaded, text("{}"));
        assert_eq!(second.set[1].new, NewValue::Null);
        assert_eq!(changes.check(), Ok(()));
        // Nothing pending is nothing to send.
        assert!(change_set(&ObjectRef::new("main", "users"), &table, &BTreeMap::new()).is_none());
    }
```

In `src/app.rs`'s tests:

```rust
    use tabletist_db::{Conflict, Value, WriteOutcome};

    /// The newest `Write` sent, if any since `from`.
    fn write_since(harness: &Harness, from: usize) -> Option<&tabletist_db::ChangeSet> {
        harness.app.backend.sent[from..]
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::Write { changes, .. } => Some(changes),
                _ => None,
            })
    }

    fn row(id: i64, email: &str) -> Vec<Value> {
        vec![Value::Int(id), Value::Text(email.into()), Value::Null]
    }

    #[test]
    fn a_save_sends_the_set_and_the_written_rows_replace_the_loaded_ones() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
        let before = harness.app.backend.sent.len();
        harness.app.apply(Action::WriteEdits { tab, id });
        let changes = write_since(&harness, before).expect("a Write");
        assert_eq!(changes.object, users());
        assert_eq!(changes.rows.len(), 2);
        // While it runs the cells are locked, and the save is the tab's to
        // cancel.
        let saving = object(&harness, tab, id).edits.saving.as_ref().unwrap();
        assert_eq!(saving.rows, [1, 3]);
        assert!(object(&harness, tab, id).pending().any(|r| r == saving.request));
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: at(0, 1),
            start: EditStart::Value,
        });
        assert_eq!(
            object(&harness, tab, id).edits.why,
            Some((at(0, 1), Lock::Saving))
        );
        harness.answer_written(Ok(WriteOutcome::Written {
            rows: vec![row(2, "bob@example.com"), row(4, "dan@example.com")],
            elapsed: std::time::Duration::from_millis(14),
        }));
        let tab_now = object(&harness, tab, id);
        assert!(tab_now.edits.cells.is_empty() && tab_now.edits.saving.is_none());
        let page = tab_now.page().unwrap();
        assert_eq!(page.rows[1][1], Value::Text("bob@example.com".into()));
        assert_eq!(page.rows[3][1], Value::Text("dan@example.com".into()));
        let saved = tab_now.edits.saved.as_ref().unwrap();
        assert_eq!((saved.changes, saved.rows), (2, 2));
        assert_eq!(saved.cells, [at(1, 1), at(3, 1)]);
        // The row panel's text is formatted again.
        assert!(tab_now.fields.is_none());
    }

    #[test]
    fn a_conflict_a_failure_and_a_refusal_keep_the_set_and_say_what_happened() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        type_into(&mut harness, tab, id, at(3, 1), "dan@example.com");
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.answer_written(Ok(WriteOutcome::Conflicts(vec![Conflict {
            row: 1,
            server: None,
        }])));
        let edits = &object(&harness, tab, id).edits;
        assert_eq!(edits.cells.len(), 2);
        // The conflict's row is the set's second, which is the page's row 3.
        assert_eq!(
            edits.note,
            Some(crate::edit::Note::Conflict {
                row: 3,
                gone: true,
                others: 0
            })
        );
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.answer_written(Ok(WriteOutcome::Failed {
            row: 0,
            error: tabletist_db::Error::query("violates check"),
        }));
        let edits = &object(&harness, tab, id).edits;
        assert!(matches!(
            edits.cells.get(&(1, 1)).map(|cell| &cell.state),
            Some(State::Failed(_))
        ));
        assert!(matches!(
            edits.cells.get(&(3, 1)).map(|cell| &cell.state),
            Some(State::Ready)
        ));
        // A failed cell does not block the next save.
        let before = harness.app.backend.sent.len();
        harness.app.apply(Action::WriteEdits { tab, id });
        assert!(write_since(&harness, before).is_some());
        harness.answer_written(Err(tabletist_db::Error::Cancelled));
        let edits = &object(&harness, tab, id).edits;
        assert_eq!(edits.note, Some(crate::edit::Note::Cancelled));
        assert_eq!(edits.cells.len(), 2);
    }

    #[test]
    fn a_save_is_not_sent_while_a_cell_is_to_fix_the_session_is_down_or_one_runs() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        type_into(&mut harness, tab, id, at(1, 2), "{oops");
        harness.app.apply(Action::LeaveEdit { tab, id });
        let before = harness.app.backend.sent.len();
        harness.app.apply(Action::WriteEdits { tab, id });
        assert!(write_since(&harness, before).is_none());
        harness.app.apply(Action::DiscardEdits { tab, id });
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        // Disconnected: the set is kept, nothing is sent.
        let session = harness.app.workspace(tab).unwrap().session;
        harness.app.apply(Action::Backend(Event::Disconnected {
            session,
            error: tabletist_db::Error::ConnectionLost("gone".into()),
        }));
        harness.app.apply(Action::WriteEdits { tab, id });
        assert!(write_since(&harness, before).is_none());
        assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
    }

    #[test]
    fn a_reconnect_under_a_save_abandons_it_and_keeps_the_set() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.app.apply(Action::Reconnect(tab));
        let edits = &object(&harness, tab, id).edits;
        assert!(edits.saving.is_none());
        assert_eq!(edits.note, Some(crate::edit::Note::Lost));
        assert_eq!(edits.cells.len(), 1);
    }

    #[test]
    fn a_written_row_of_another_width_fetches_the_page_again() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        harness.app.apply(Action::WriteEdits { tab, id });
        let before = harness.app.backend.sent.len();
        harness.answer_written(Ok(WriteOutcome::Written {
            rows: vec![vec![Value::Int(2)]],
            elapsed: std::time::Duration::ZERO,
        }));
        assert!(!object(&harness, tab, id).edits.holds());
        assert!(matches!(
            harness.app.backend.sent[before..].last(),
            Some(Command::FetchRows { .. })
        ));
    }
```

`Event::Disconnected`'s fields and `Error::query` are as the backend and the crate name them: read them and correct the tests' spelling where it differs.

- [ ] **Step 3: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: does not compile (`change_set`, `Action::WriteEdits`, `Edits::saving`, `Note`).

- [ ] **Step 4: `src/edit.rs`**

`Edits` gains three fields:

```rust
    /// The save that is running.
    pub saving: Option<Saving>,
    /// The last save that wrote, for the cells' green and the status.
    pub saved: Option<Saved>,
    /// What the last save came to when it wrote nothing.
    pub note: Option<Note>,
```

`holds` becomes `!self.cells.is_empty() || self.editor.is_some() || self.saving.is_some()`, and the `Debug` text gains `saving: {}` (`self.saving.is_some()`). And:

```rust
/// A save in flight.
#[derive(Debug)]
pub struct Saving {
    pub request: RequestId,
    /// The page's row of each row of the change set, in its order: the
    /// answer names rows by their place in the set.
    pub rows: Vec<usize>,
    pub started: Instant,
}

/// A save that wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    /// When, for the cells that show it for a moment.
    pub at: Instant,
    pub cells: Vec<CellPos>,
    pub changes: usize,
    pub rows: usize,
    pub elapsed: Duration,
}

/// How long a saved cell shows it.
pub const SAVED_FOR: Duration = Duration::from_millis(1200);

/// What a save came to when it wrote nothing. The view words it.
#[derive(Debug, Clone, PartialEq)]
pub enum Note {
    /// The page's row `row` changed on the server, or is `gone`; `others`
    /// more rows conflict too.
    Conflict {
        row: usize,
        gone: bool,
        others: usize,
    },
    /// The statement of the page's row `row` failed.
    Failed { row: usize, error: Error },
    /// The connection was lost while saving: what was written is not known.
    Lost,
    /// The save was cancelled.
    Cancelled,
    /// The save was refused or undone, with the database's or the app's
    /// reason.
    Refused(Error),
}

/// The change set a save sends for the pending `cells`, and the page's row
/// of each of its rows. `None` when nothing is pending or the table has no
/// key.
pub fn change_set(
    object: &ObjectRef,
    table: &Table<'_>,
    cells: &BTreeMap<(usize, usize), Pending>,
) -> Option<(ChangeSet, Vec<usize>)> {
    let key = table.key()?;
    let mut rows: Vec<RowChange> = Vec::new();
    let mut places = Vec::new();
    // The map is ordered by row, then column, so a row's cells are together.
    for (&(row, col), pending) in cells {
        let values = table.page.rows.get(row)?;
        let column = table.column(col)?;
        if places.last() != Some(&row) {
            places.push(row);
            rows.push(RowChange {
                key: key
                    .iter()
                    .map(|&place| (table.page.columns[place].name.clone(), values[place].clone()))
                    .collect(),
                set: Vec::new(),
            });
        }
        rows.last_mut()?.set.push(CellChange {
            column: column.name.clone(),
            type_name: column.type_name.clone(),
            loaded: values.get(col)?.clone(),
            new: pending.new.clone(),
        });
    }
    (!rows.is_empty()).then(|| {
        (
            ChangeSet {
                object: object.clone(),
                rows,
            },
            places,
        )
    })
}
```

- [ ] **Step 5: The model**

- `Action::WriteEdits { tab: ConnTabId, id: TabId }`, documented "Save the tab's pending changes, in one transaction."
- `ObjectTab::pending` also yields `self.edits.saving.as_ref().map(|saving| saving.request)`. Update `an_object_tab_waits_for_everything_it_loads` to say so: a save is the tab's to cancel.
- In `App::table`, `saving: object.edits.saving.is_some()`.

- [ ] **Step 6: The reducer**

```rust
            Action::WriteEdits { tab, id } => self.write_edits(tab, id),
```

```rust
    /// Why the tab's pending changes cannot be saved now, if they cannot.
    /// The view shows it on the disabled Save.
    pub fn save_blocked(&self, tab: ConnTabId, id: TabId) -> Option<SaveBlock> {
        let workspace = self.workspace(tab)?;
        let object = workspace.object_tab(id)?;
        if object.edits.saving.is_some() {
            return Some(SaveBlock::Saving);
        }
        if object.edits.counts().to_fix > 0 {
            return Some(SaveBlock::ToFix);
        }
        if !matches!(workspace.status, SessionStatus::Connected) {
            return Some(SaveBlock::Disconnected);
        }
        if workspace.access == tabletist_db::Access::ReadOnly {
            return Some(SaveBlock::ReadOnly);
        }
        None
    }

    fn write_edits(&mut self, tab: ConnTabId, id: TabId) {
        // The text being typed is part of what is saved.
        self.close_editor(tab, id, true);
        if self.save_blocked(tab, id).is_some() {
            return;
        }
        let built = self
            .table(tab, id, |table, object| {
                crate::edit::change_set(&object.object, table, &object.edits.cells)
            })
            .flatten();
        let Some((changes, rows)) = built else {
            return;
        };
        self.send_write(tab, id, changes, rows);
    }

    fn send_write(
        &mut self,
        tab: ConnTabId,
        id: TabId,
        changes: tabletist_db::ChangeSet,
        rows: Vec<usize>,
    ) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let Some(object) = workspace.object_tab_mut(id) else {
            return;
        };
        object.edits.note = None;
        object.edits.saved = None;
        object.edits.saving = Some(crate::edit::Saving {
            request,
            rows,
            started: std::time::Instant::now(),
        });
        self.backend.send(Command::Write {
            session,
            request,
            changes,
        });
    }
```

`SaveBlock` is a small enum in `src/model.rs`: `Saving`, `ToFix`, `Disconnected`, `ReadOnly`.

The `Event::Written` arm:

```rust
            Event::Written {
                session,
                request,
                result,
            } => {
                let Some(tab) = self.tab_for_session(session) else {
                    return;
                };
                let Some(workspace) = self.workspace_mut(tab) else {
                    return;
                };
                let Some(object) = workspace.object_tabs_mut().find(|object| {
                    object
                        .edits
                        .saving
                        .as_ref()
                        .is_some_and(|saving| saving.request == request)
                }) else {
                    return;
                };
                let id = object.id;
                let Some(saving) = object.edits.saving.take() else {
                    return;
                };
                // The page's row of the set's row `index`.
                let place = |index: usize| saving.rows.get(index).copied().unwrap_or(0);
                match result {
                    Ok(tabletist_db::WriteOutcome::Written { rows, elapsed }) => {
                        let counts = object.edits.counts();
                        let cells = object
                            .edits
                            .cells
                            .keys()
                            .map(|&(row, col)| CellPos { row, col })
                            .collect();
                        object.edits = crate::edit::Edits::default();
                        object.fields = None;
                        let fits = object.rows.value.as_ref().is_some_and(|page| {
                            rows.len() == saving.rows.len()
                                && rows.iter().all(|row| row.len() == page.columns.len())
                                && saving.rows.iter().all(|&at| at < page.rows.len())
                        });
                        if fits {
                            if let Some(page) = object.rows.value.as_mut() {
                                for (row, &at) in rows.into_iter().zip(&saving.rows) {
                                    page.rows[at] = row;
                                }
                            }
                            object.edits.saved = Some(crate::edit::Saved {
                                at: std::time::Instant::now(),
                                cells,
                                changes: counts.changes,
                                rows: counts.rows,
                                elapsed,
                            });
                        } else {
                            // The table is not the one the page was read
                            // from: read it again.
                            self.fetch_rows(tab, id);
                        }
                    }
                    Ok(tabletist_db::WriteOutcome::Conflicts(conflicts)) => {
                        object.edits.note =
                            conflicts.first().map(|first| crate::edit::Note::Conflict {
                                row: place(first.row),
                                gone: first.server.is_none(),
                                others: conflicts.len() - 1,
                            });
                    }
                    Ok(tabletist_db::WriteOutcome::Failed { row, error }) => {
                        let at = place(row);
                        for (_, cell) in object.edits.cells.range_mut((at, 0)..=(at, usize::MAX)) {
                            cell.state = crate::edit::State::Failed(error.clone());
                        }
                        object.edits.note = Some(crate::edit::Note::Failed { row: at, error });
                    }
                    Err(error) => {
                        object.edits.note = Some(if error.is_connection_lost() {
                            crate::edit::Note::Lost
                        } else if error == Error::Cancelled {
                            crate::edit::Note::Cancelled
                        } else {
                            crate::edit::Note::Refused(error)
                        });
                    }
                }
            }
```

A page that arrives ends what the last save left on the tab. In the `Event::Rows` arm, after `object.rows.finish(request, result)`:

```rust
                // The marks of the last save and the note of a locked cell
                // were about the page this one replaces. Nothing is pending
                // here: a tab that holds edits is never fetched again (the
                // guard of the next task is what makes that so).
                object.edits = crate::edit::Edits::default();
```

with a test, `a_new_page_forgets_the_last_save`: on a page with `has_more` set, after a written save and an `EditCell` on the key column (which leaves a `why`), `NextPage` and its answer leave `edits.saved` and `edits.why` empty. And a test that `DiscardEdits` and `RevertCell` do nothing while a save runs: the set and `edits.saving` are as they were, and the `Written` answer still lands in the tab.

The session swap: an answer from a replaced session never arrives (`tab_for_session` finds the current one only). In `Workspace::forget_session_requests` (`src/model.rs`), which both `reconnect` and `ensure_connecting` call:

```rust
        // A save sent on the old session will not be answered here: what it
        // wrote is not known. The set is kept.
        for object in self.object_tabs_mut() {
            if object.edits.saving.take().is_some() {
                object.edits.note = Some(crate::edit::Note::Lost);
            }
        }
```

and correct its doc comment, which says object tabs are not touched.

- [ ] **Step 7: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: the eight new tests pass (the six above and the two described under the reducer's step); `an_object_tab_waits_for_everything_it_loads` passes with its new line.

- [ ] **Step 8: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A && git commit -m "Save a tab's pending cells and put the answer back into its page"
```

---

### Task 5: The page stays while a tab holds edits, and production is asked first

**Files:**
- Modify: `src/model.rs` (`Held`, `LeavePrompt`, `WritePrompt`, `Dialog::Leave`, `Dialog::ConfirmWrite`, the prompt actions), `src/app.rs` (the guard, `after_connect`, `resize_pages`, `follow_foreign_key`, `write_edits`), `src/env.rs` (`confirms_writes`), `src/ui/mod.rs` (a stand-in so the new dialogs can be left)
- Test: `src/app.rs`, `src/env.rs`

- [ ] **Step 1: Write the failing tests**

In `src/app.rs`'s tests:

```rust
    // `Dialog` is already imported in this test module.

    /// What makes an action for a tab, for a table of guarded ones.
    type Guarded = Box<dyn Fn(ConnTabId, TabId) -> Action>;

    /// Whether the Leave prompt is up, and whether it offers Save.
    fn leave_prompt(harness: &Harness) -> Option<bool> {
        match &harness.app.dialog {
            Some(Dialog::Leave(prompt)) => Some(prompt.can_save),
            _ => None,
        }
    }

    #[test]
    fn an_action_that_would_drop_the_page_is_held_until_the_user_chooses() {
        let guarded: Vec<(&str, Guarded)> = vec![
            ("next page", Box::new(|tab, object_tab| Action::NextPage { tab, object_tab })),
            ("previous page", Box::new(|tab, object_tab| Action::PrevPage { tab, object_tab })),
            ("clear sort", Box::new(|tab, object_tab| Action::ClearSort { tab, object_tab })),
            ("retry", Box::new(|tab, object_tab| Action::RetryRows { tab, object_tab })),
            ("sort", Box::new(|tab, object_tab| Action::SortBy { tab, object_tab, column: "email".into() })),
            ("filters", Box::new(|tab, object_tab| Action::ApplyFilters { tab, object_tab })),
            ("clear filters", Box::new(|tab, object_tab| Action::ClearFilters { tab, object_tab })),
            ("refresh", Box::new(|tab, _| Action::Refresh(tab))),
            ("close tab", Box::new(|tab, id| Action::CloseTab { tab, id })),
            ("disconnect", Box::new(|tab, _| Action::Disconnect(tab))),
            ("close connection", Box::new(|tab, _| Action::CloseConnTab(tab))),
            ("switch database", Box::new(|tab, _| Action::SwitchDatabase { tab, database: "other".into() })),
        ];
        for (name, action) in guarded {
            let mut harness = Harness::new();
            let (tab, id) = harness.editable();
            // A page in the middle of a sorted table, so Next, Previous and
            // Clear sort each have something to do.
            {
                let object = harness.app.workspace_mut(tab).unwrap().object_tab_mut(id).unwrap();
                object.rows.value.as_mut().unwrap().has_more = true;
                object.query.offset = 5;
                object.query.sort = vec![tabletist_db::Sort {
                    column: "email".into(),
                    dir: tabletist_db::SortDir::Asc,
                }];
            }
            type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
            let before = harness.app.backend.sent.len();
            harness.app.apply(action(tab, id));
            assert_eq!(leave_prompt(&harness), Some(true), "{name}");
            assert_eq!(harness.app.backend.sent.len(), before, "{name}: nothing ran");
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1, "{name}");
            // Staying drops the held action.
            harness.app.apply(Action::LeaveStay);
            assert!(harness.app.dialog.is_none(), "{name}");
            assert_eq!(harness.app.backend.sent.len(), before, "{name}");
            // Discard runs it.
            harness.app.apply(action(tab, id));
            harness.app.apply(Action::LeaveDiscard);
            assert!(harness.app.dialog.is_none(), "{name}");
            let gone = harness
                .app
                .workspace(tab)
                .and_then(|workspace| workspace.object_tab(id))
                .is_none_or(|object| !object.edits.holds());
            assert!(gone, "{name}: the set is dropped");
            // It ran: a command went out, or (closing a tab sends none when
            // nothing is pending) the tab is gone.
            let ran = harness.app.backend.sent.len() > before
                || harness
                    .app
                    .workspace(tab)
                    .and_then(|workspace| workspace.object_tab(id))
                    .is_none();
            assert!(ran, "{name}: the action ran");
        }
    }

    #[test]
    fn an_action_that_would_do_nothing_is_not_held() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        // The last page, the first page, no sort: the arms do nothing, and
        // asking to discard for that would throw the set away for nothing.
        for action in [
            Action::NextPage { tab, object_tab: id },
            Action::PrevPage { tab, object_tab: id },
            Action::ClearSort { tab, object_tab: id },
        ] {
            harness.app.apply(action);
            assert!(harness.app.dialog.is_none());
        }
        assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
    }

    #[test]
    fn what_does_not_drop_the_page_is_not_held() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        harness.app.apply(Action::SetView { tab, object_tab: id, view: ObjectView::Structure });
        harness.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "orders"),
            kind: tabletist_db::ObjectKind::Table,
            pin: false,
        });
        harness.app.apply(Action::ActivateTab { tab, id });
        assert!(harness.app.dialog.is_none());
        assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
    }

    #[test]
    fn save_from_the_prompt_runs_the_held_action_only_when_everything_was_written() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        harness.app.apply(Action::CloseTab { tab, id });
        harness.app.apply(Action::LeaveSave);
        assert!(harness.app.dialog.is_none());
        // A conflict drops the held action: the tab stays.
        harness.answer_written(Ok(WriteOutcome::Conflicts(vec![Conflict { row: 0, server: None }])));
        assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
        // Written: the tab closes.
        harness.app.apply(Action::CloseTab { tab, id });
        harness.app.apply(Action::LeaveSave);
        harness.answer_written(Ok(WriteOutcome::Written {
            rows: vec![row(2, "bob@example.com")],
            elapsed: std::time::Duration::ZERO,
        }));
        assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_none());
    }

    #[test]
    fn the_prompt_offers_no_save_where_save_is_disabled_and_none_for_several_tabs() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        type_into(&mut harness, tab, id, at(1, 2), "{oops");
        harness.app.apply(Action::LeaveEdit { tab, id });
        harness.app.apply(Action::CloseTab { tab, id });
        assert_eq!(leave_prompt(&harness), Some(false));
        harness.app.apply(Action::LeaveStay);
        // Two tabs with pending changes: Discard or Cancel only.
        harness.app.apply(Action::DiscardEdits { tab, id });
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        harness.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "orders"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_structure(crate::testing::fixture_structure());
        harness.answer_rows(page(5, false));
        let other = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        type_into(&mut harness, tab, other, at(0, 1), "x@example.com");
        harness.app.apply(Action::Disconnect(tab));
        assert_eq!(leave_prompt(&harness), Some(false));
    }

    #[test]
    fn a_guarded_action_is_ignored_while_the_save_runs_and_refused_under_another_dialog() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        harness.app.apply(Action::ShowHelp);
        harness.app.apply(Action::CloseTab { tab, id });
        assert!(matches!(harness.app.dialog, Some(Dialog::Help)));
        assert!(harness.app.notice.is_some());
        assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
        harness.app.apply(Action::CloseDialog);
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.app.apply(Action::CloseTab { tab, id });
        assert!(harness.app.dialog.is_none());
        assert!(harness.app.workspace(tab).unwrap().object_tab(id).is_some());
    }

    #[test]
    fn a_tab_with_pending_changes_keeps_its_page_through_a_reconnect() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        harness.app.apply(Action::Reconnect(tab));
        let before = harness.app.backend.sent.len();
        answer_connect(&mut harness);
        assert!(
            !harness.app.backend.sent[before..]
                .iter()
                .any(|command| matches!(command, Command::FetchRows { .. } | Command::Describe { .. })),
            "the active tab is not read again"
        );
        assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
        // And a save works afterwards.
        let before = harness.app.backend.sent.len();
        harness.app.apply(Action::WriteEdits { tab, id });
        assert!(write_since(&harness, before).is_some());
    }

    #[test]
    fn a_save_to_production_is_asked_first_with_its_statements() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
        type_into(&mut harness, tab, id, at(1, 1), "bob@example.com");
        let before = harness.app.backend.sent.len();
        harness.app.apply(Action::WriteEdits { tab, id });
        assert!(write_since(&harness, before).is_none());
        let Some(Dialog::ConfirmWrite(prompt)) = &harness.app.dialog else {
            panic!("expected the confirmation");
        };
        assert_eq!(prompt.statements.len(), 1);
        assert!(prompt.statements[0].starts_with("UPDATE"), "{}", prompt.statements[0]);
        assert_eq!((prompt.changes, prompt.rows), (1, 1));
        // Cancel sends nothing and keeps the set.
        harness.app.apply(Action::CancelWrite);
        assert!(harness.app.dialog.is_none());
        assert!(write_since(&harness, before).is_none());
        assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.app.apply(Action::ConfirmWrite);
        assert!(harness.app.dialog.is_none());
        assert!(write_since(&harness, before).is_some());
    }
```

`answer_connect`, `ObjectView` and `Action::ShowHelp` are what the test module already uses; correct the names where they differ. For the reconnect test, check what `answer_connect` sends besides (schemas, the tree): the assertion is only about this tab's rows and structure.

Four more tests, written against how the neighbouring tests do the same things:

- `a_new_page_size_leaves_a_tab_with_pending_changes_alone`: change the page size the way the existing `resize_pages` tests do; no `FetchRows` goes out for the tab with the pending cell, and one does for a second table tab without edits.
- `a_reconnect_that_comes_back_read_only_keeps_the_set_and_blocks_save`: turn the saved connection's "Open read-only" box on, reconnect, answer the connect: the set is kept, `save_blocked` answers `SaveBlock::ReadOnly`, `WriteEdits` sends nothing, and a guarded action's prompt has `can_save == false`.
- `dropping_one_filter_is_held_too`: with an applied filter (set up as the existing `DropFilter` test does) and a pending cell, `DropFilter` raises the prompt.
- `saving_from_the_prompt_with_only_an_untouched_editor_open_just_goes_on`: open an editor, type nothing, `CloseTab`, `LeaveSave`: nothing is sent and the tab closes.

In `src/env.rs`'s tests:

```rust
    #[test]
    fn only_production_asks_before_a_write() {
        for environment in Environment::ALL {
            assert_eq!(
                environment.confirms_writes(),
                environment == Environment::Production,
                "{environment:?}"
            );
        }
    }
```

- [ ] **Step 2: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: does not compile.

- [ ] **Step 3: `src/env.rs`**

Beside `read_only_by_default`:

```rust
    /// Whether a save to a connection in this environment is confirmed
    /// first, with its statements on screen.
    pub fn confirms_writes(self) -> bool {
        match self {
            Self::Production => true,
            Self::Local | Self::Dev | Self::Staging | Self::None => false,
        }
    }
```

- [ ] **Step 4: The model**

```rust
/// What waits for the user's answer about pending changes.
#[derive(Debug)]
pub enum Held {
    Action(Box<Action>),
}

/// Asks before pending changes are dropped.
#[derive(Debug)]
pub struct LeavePrompt {
    pub held: Held,
    /// The tabs whose pending changes the held action would drop.
    pub tabs: Vec<(ConnTabId, TabId)>,
    /// One tab, and its save is not disabled: Save is offered.
    pub can_save: bool,
    /// How many changes would be dropped, over all the tabs.
    pub changes: usize,
}

/// Asks before a save to production.
pub struct WritePrompt {
    pub tab: ConnTabId,
    pub id: TabId,
    /// The statements as a person reads them, one per row.
    pub statements: Vec<String>,
    pub changes: usize,
    pub rows: usize,
    /// What the Omarchy box's field holds: `write` confirms.
    pub typed: String,
    pub focus: bool,
    pub(crate) changeset: tabletist_db::ChangeSet,
    pub(crate) places: Vec<usize>,
    pub(crate) then: Option<Held>,
}
```

`WritePrompt` gets a hand-written `Debug` that prints the counts only: its statements hold the user's values. `Dialog` gains `Leave(Box<LeavePrompt>)` and `ConfirmWrite(Box<WritePrompt>)`. `Action` gains `LeaveSave`, `LeaveDiscard`, `LeaveStay`, `ConfirmWrite`, `CancelWrite`. `crate::edit::Saving` gains `pub then: Option<crate::model::Held>` ("what to do once everything is written").

- [ ] **Step 5: The guard**

At the top of `App::apply`:

```rust
    pub fn apply(&mut self, action: Action) {
        // An action that would drop a page with pending changes waits for
        // the user's answer (see `Dialog::Leave`).
        let dropped = self.dropped_by(&action);
        if !dropped.is_empty() {
            self.hold(Held::Action(Box::new(action)), dropped);
            return;
        }
        match action {
```

```rust
    /// The tabs holding edits whose page or whose tab `action` would drop.
    fn dropped_by(&self, action: &Action) -> Vec<(ConnTabId, TabId)> {
        let holds = |tab: ConnTabId, id: TabId| {
            self.workspace(tab)
                .and_then(|workspace| workspace.object_tab(id))
                .is_some_and(|object| object.edits.holds())
        };
        let one = |tab: ConnTabId, id: TabId| {
            if holds(tab, id) {
                vec![(tab, id)]
            } else {
                Vec::new()
            }
        };
        let all = |tab: ConnTabId| {
            self.workspace(tab)
                .map(|workspace| {
                    workspace
                        .object_tabs()
                        .filter(|object| object.edits.holds())
                        .map(|object| (tab, object.id))
                        .collect()
                })
                .unwrap_or_default()
        };
        // The arm's own condition: an action that would do nothing is not
        // worth a question whose Discard throws the set away.
        let object = |tab: ConnTabId, id: TabId| {
            self.workspace(tab)
                .and_then(|workspace| workspace.object_tab(id))
        };
        match action {
            Action::NextPage { tab, object_tab }
                if !object(*tab, *object_tab)
                    .and_then(|object| object.page())
                    .is_some_and(|page| page.has_more) =>
            {
                Vec::new()
            }
            Action::PrevPage { tab, object_tab }
                if object(*tab, *object_tab).is_none_or(|object| object.query.offset == 0) =>
            {
                Vec::new()
            }
            Action::ClearSort { tab, object_tab }
                if object(*tab, *object_tab).is_none_or(|object| object.query.sort.is_empty()) =>
            {
                Vec::new()
            }
            Action::NextPage { tab, object_tab }
            | Action::PrevPage { tab, object_tab }
            | Action::SortBy {
                tab, object_tab, ..
            }
            | Action::ClearSort { tab, object_tab }
            | Action::ApplyFilters { tab, object_tab }
            | Action::ClearFilters { tab, object_tab }
            | Action::DropFilter {
                tab, object_tab, ..
            }
            | Action::RetryRows { tab, object_tab } => one(*tab, *object_tab),
            Action::CloseTab { tab, id } => one(*tab, *id),
            Action::Refresh(tab) => self
                .workspace(*tab)
                .and_then(|workspace| workspace.active_object_tab())
                .map(|object| one(*tab, object.id))
                .unwrap_or_default(),
            Action::CloseConnTab(tab)
            | Action::Disconnect(tab)
            | Action::SwitchDatabase { tab, .. }
            | Action::Connect { tab, .. } => all(*tab),
            _ => Vec::new(),
        }
    }

    /// Keeps `held` and asks. While one of the tabs is saving the action is
    /// ignored (the actions the guard covers are disabled until a save
    /// ends), and a dialog the user is in is never replaced.
    fn hold(&mut self, held: Held, tabs: Vec<(ConnTabId, TabId)>) {
        let object = |&(tab, id): &(ConnTabId, TabId)| {
            self.workspace(tab)
                .and_then(|workspace| workspace.object_tab(id))
        };
        if tabs
            .iter()
            .any(|at| object(at).is_some_and(|object| object.edits.saving.is_some()))
        {
            return;
        }
        if self.dialog.is_some() {
            self.notice = Some("Save or discard the pending changes first.".into());
            return;
        }
        let changes = tabs
            .iter()
            .filter_map(object)
            .map(|object| object.edits.counts().changes.max(1))
            .sum();
        let can_save = match tabs.as_slice() {
            [(tab, id)] => self.save_blocked(*tab, *id).is_none(),
            _ => false,
        };
        self.dialog = Some(Dialog::Leave(Box::new(LeavePrompt {
            held,
            tabs,
            can_save,
            changes,
        })));
    }

    fn perform(&mut self, held: Held) {
        match held {
            Held::Action(action) => self.apply(*action),
        }
    }
```

`Refresh` on a workspace whose active tab is a SQL editor holds nothing, as the arm does nothing there. Check `active_object_tab`'s name and what it answers while a SQL tab is active.

The prompt's arms:

```rust
            Action::LeaveStay => {
                if matches!(self.dialog, Some(Dialog::Leave(_))) {
                    self.dialog = None;
                }
            }
            Action::LeaveDiscard => {
                // The kind is checked before the dialog is taken, as
                // `SubmitPassword` does: another dialog is not closed by it.
                if !matches!(self.dialog, Some(Dialog::Leave(_))) {
                    return;
                }
                let Some(Dialog::Leave(prompt)) = self.dialog.take() else {
                    return;
                };
                for &(tab, id) in &prompt.tabs {
                    if let Some(object) = self.object_tab_mut(tab, id) {
                        object.edits = crate::edit::Edits::default();
                        object.fields = None;
                    }
                }
                self.perform(prompt.held);
            }
            Action::LeaveSave => {
                if !matches!(self.dialog, Some(Dialog::Leave(_))) {
                    return;
                }
                let Some(Dialog::Leave(prompt)) = self.dialog.take() else {
                    return;
                };
                if let ([(tab, id)], true) = (prompt.tabs.as_slice(), prompt.can_save) {
                    self.write_edits(*tab, *id, Some(prompt.held));
                }
            }
```

`write_edits` gains `then: Option<Held>` (`Action::WriteEdits` passes `None`), hands it to `send_write`, which puts it into `Saving::then`. In the `Written` arm, after a save that fitted its page, `if let Some(held) = saving.then { self.perform(held); }` as the arm's last act for that outcome (take `then` out of `saving` before the match, so the other outcomes simply drop it). An open editor is part of the edits: `LeaveSave` reaches `write_edits`, which closes it first. When that leaves the set empty (the editor was only opened), there is nothing to save and the held action simply goes on: in `write_edits`, when the tab's `edits.cells` is empty after the editor closed, `if let Some(held) = then { self.perform(held); }` and return. Only then: `change_set` also answers `None` with cells still pending (the key is gone, a column is unknown), and performing the held action there would drop them; for the window's close (task 11), which does not pass the guard again, it would close over them.

- [ ] **Step 6: What replaces a page without an action**

- `after_connect`: in the `stale` computation, a tab with `object.edits.holds()` answers `(object.id, false, false)`: its page and its structure stay as the edits were made on them.
- `resize_pages`: skip an object tab with `object.edits.holds()`; it takes the new size at its next fetch (`fetch_page` already drops a page of another size).
- `follow_foreign_key`: before it touches the filter bar of the tab `open_object` answered (the hold must come before `opened.filter.rows` is set, not only before `apply_filters`), if that tab holds edits, call `self.hold(Held::Action(Box::new(Action::FollowForeignKey { tab, object, column, value })), vec![(tab, id)])` and return. `object` is moved into `open_object` earlier in the function: clone it for the held action. Read the function: the held action must be the one that leads back to this point with the same arguments. Add a test beside the others: an open `orders` tab with a pending cell, `FollowForeignKey` to `orders`, the prompt is up and the tab's filters are unchanged.

- [ ] **Step 7: Production**

In `write_edits`, after the change set is built and before it is sent:

```rust
        let confirm = self
            .workspace(tab)
            .is_some_and(|workspace| workspace.environment.confirms_writes());
        if confirm {
            // A dialog the user is in is not replaced: the save waits.
            if self.dialog.is_some() {
                return;
            }
            let dialect = self.workspace(tab).map(|workspace| workspace.driver.dialect());
            let statements = changes
                .rows
                .iter()
                .map(|row| {
                    dialect
                        .and_then(|dialect| dialect.update_row(&changes.object, row).ok())
                        .map_or_else(String::new, |update| update.shown)
                })
                .collect();
            let cells = changes.rows.iter().map(|row| row.set.len()).sum();
            self.dialog = Some(Dialog::ConfirmWrite(Box::new(WritePrompt {
                tab,
                id,
                statements,
                changes: cells,
                rows: rows.len(),
                typed: String::new(),
                focus: true,
                changeset: changes,
                places: rows,
                then,
            })));
            return;
        }
```

A row whose statement cannot be built shows an empty line here and fails in the save with the builder's reason; that is rare enough to leave (the checks catch what the builder refuses).

```rust
            Action::ConfirmWrite => {
                if !matches!(self.dialog, Some(Dialog::ConfirmWrite(_))) {
                    return;
                }
                let Some(Dialog::ConfirmWrite(prompt)) = self.dialog.take() else {
                    return;
                };
                let prompt = *prompt;
                // Still to be saved? The prompt was up for a while.
                if self.save_blocked(prompt.tab, prompt.id).is_none() {
                    self.send_write(prompt.tab, prompt.id, prompt.changeset, prompt.places, prompt.then);
                }
            }
            Action::CancelWrite => {
                if matches!(self.dialog, Some(Dialog::ConfirmWrite(_))) {
                    self.dialog = None;
                }
            }
```

While the confirmation is up no key reaches the grid (`keys::handle` is skipped under a dialog), so the set it was built from cannot change.

- [ ] **Step 8: A way out of the new dialogs until they are drawn**

`ui::show` draws each dialog by its own function and nothing draws these two yet. Until task 10 draws them, a test could not get past one. In `src/ui/mod.rs`'s list of dialogs add `write_prompts::show(app, &ctx)` and create `src/ui/write_prompts.rs` with the smallest thing that is honest: a `widgets::modal` with the title and the buttons, wired to the five actions, Esc for `LeaveStay` and `CancelWrite`, and, in the confirmation, every line of `prompt.statements` in the code face: no build of the app may offer a save to production without its statements on screen. Task 10 replaces its body. One headless test in `src/ui/mod.rs`: with a pending cell, `harness.click` on the tab's close leads to a dialog whose "Discard" button closes the tab.

- [ ] **Step 9: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: all pass. The whole suite: `~/.cargo/bin/cargo test --locked --workspace --all-targets`.

- [ ] **Step 10: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A && git commit -m "Ask before pending changes are dropped, and before a save to production"
```

**The model of editing and saving is whole here.** Nothing a user can do reaches it yet: the tasks that follow draw it. Three of them add state of their own: closing the window (task 11), the row panel's text (task 12) and Omarchy's `:` prompt (task 15).

---
## The drawing tasks

From here the plan gives each task its behaviour, where it hooks into the code, the shapes it adds and the tests that pin it, not every line: the drawing code has to be written against the functions as they are, and they are long. Read the named functions first. Every task is test first (a headless test that fails for the stated reason), ends green on the four checks, and is committed alone.

Helpers the drawing tests need, to add to the test module of `src/ui/mod.rs` with the first test that uses them (several exist there already under these names: `type_text`, `click_at`, `type_key`, `with_page`, `focus_grid`):

```rust
    /// A writable table with its structure and five rows, the grid holding
    /// the keyboard, in `look`.
    fn editable_in(look: crate::theme::Look) -> (Harness, ConnTabId, TabId) {
        let mut harness = Harness::new();
        harness.set_look(look);
        let (tab, id) = harness.editable();
        focus_grid(&mut harness, tab);
        harness.settle();
        (harness, tab, id)
    }

    /// The middle of the cell that shows `text`.
    fn cell_of(harness: &Harness, text: &str) -> egui::Pos2 {
        harness
            .painted_rect(text)
            .unwrap_or_else(|| panic!("no cell shows {text}"))
            .center()
    }

    fn edits(harness: &Harness, tab: ConnTabId, id: TabId) -> &crate::edit::Edits {
        &harness
            .app
            .workspace(tab)
            .unwrap()
            .object_tab(id)
            .unwrap()
            .edits
    }
```

`Harness::press` sends key events and no text; typing goes through `type_text` (an `Event::Text`). A key the app reads as typed text (`i`, `x`, `u`, `:`) needs both in one frame: `type_key`.

### Task 6: The grid shows what is pending

**Files:**
- Modify: `src/ui/grid.rs`, `src/ui/data_view.rs`, `src/ui/states.rs`, `src/ui/sql_results.rs` (its call of `grid::show`)
- Test: `src/ui/grid.rs` (its own `frame` helpers), `src/ui/mod.rs`

**What it adds**

- `states::Tone::Success`, with `color`, `fill` and `line` as the other two have them (a tint of `palette.success` mixed into the window colour at the strengths `Tone::Warning` uses; follow `env::warning_tint`). A test beside the existing ones: the three fills differ from each other and from the window, in every look and both palettes.
- In `grid.rs`:

```rust
/// What a cell's pending state is, for how it is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mark {
    #[default]
    None,
    Pending,
    /// To fix, or failed in the last save.
    Trouble,
    /// Pending and being saved.
    Saving,
    /// Just written.
    Saved,
    /// A cell of a computed column, in a table that can be edited.
    Locked,
}
```

  `Cell` gains `mark: Mark` and `hint: Option<String>` (a tooltip for the cell: "was <loaded value>", or a failed cell's message; never why a cell is locked, which is said only when asked). `Cell` is written out as a literal in about twenty places (`data_view::cell`, `plain_cell`, the grid's own tests): derive `Default` for it if its fields allow and end each literal with `..Default::default()`, or set the two fields at every site, whichever the file's style is. `grid::show` gains one parameter, `rows: &dyn Fn(usize) -> crate::edit::RowMark`, and `GridOutput` gains `double_clicked: Option<CellPos>`. `sql_results.rs` passes `&|_| RowMark::None` and ignores the new output.
- Drawing, in the cell loop of `grid::show` (after the row fill, before the text):
  - `Mark::Pending` and `Saving`: fill the cell with `Tone::Warning.fill`; macOS and Windows add the 2 pt bar at the cell's left in `Tone::Warning.color`; Omarchy draws the text in `palette.warning`. `Saving` adds the small spinner at the cell's right (`states` has one).
  - `Mark::Trouble`: `Tone::Danger.fill`, a 1 pt `Tone::Danger.color` line inside the cell; Omarchy's text in `palette.danger`.
  - `Mark::Saved`: `Tone::Success.fill`.
  - `Mark::Locked` (macOS and Windows): the cell filled with `palette.surface` and its text in `palette.secondary`. Omarchy draws it as today.
  - The selected cell keeps its own drawing on top. On Omarchy the reverse-video cursor forces plain text in the window colour (around line 643): for a marked cell keep the cursor's accent line (a 2 pt inside stroke) instead of the fill, so the mark stays readable under the cursor.
  - The row: `RowMark::Changed` or `Trouble` draws, on macOS and Windows, the 3 pt bar at the row's left edge (in place of the selected row's accent bar, which it wins over) in the warning or danger colour, and the row's first cell, when it is a key column, in that colour; on Omarchy the gutter shows `~` or `!` in that colour beside the cursor mark.
- The double-click: the row's response is the only one. Beside `response.clicked()`, `response.double_clicked()` with the same column arithmetic gives `output.double_clicked`.
- The hint: when the pointer is over a cell that has one, show it as a tooltip at the cell (a hover-only `ui.interact` on the cell's rect, or the fork's tooltip-at-pointer call; hover-only widgets let clicks through, see the comment near the header's resize handle).
- In `data_view::show`: compute each cell's mark and hint from `object.edits` and the page: a pending cell shows its **new** value (drawn through the same `cell`/`plain_cell` path from a `Value` made of the new text, or NULL) with `hint` "was <loaded>", where the loaded text is `format::cell_text` of the loaded value. `cell()` answers a `Cell<'a>` that borrows its value, so a `Value` made inside the grid's closure cannot be returned from it: build the pending cells' values before `grid::show` is called (a small map from cell to `Value`, kept for the call) and borrow from that. In a table that can be edited, the cells of a computed column get `Mark::Locked`: decide it once per column before the grid is drawn (the table-wide part of `Table::lock` is none or `Saving` or `Refreshing`, and the column's `generated` is set), not by asking `lock` for every visible cell in every frame, which would also drop the tint while a save or a refresh runs; `State::ToFix` and `Failed` give `Mark::Trouble` with the problem's or the error's words as the hint; cells in `edits.saved.cells` show `Mark::Saved` while `saved.at.elapsed() < edit::SAVED_FOR`, and the view asks for a repaint when that ends (`ui.ctx().request_repaint_after(remaining)`); while `edits.saving` is some, pending cells are `Mark::Saving`. `output.double_clicked` is not used yet: task 7, which draws the field, wires it (an editor nothing draws would hold the tab's page for no visible reason).
- The words for a `Problem` and for a `Lock`, in one place both the grid's hints and the later tasks use: `src/ui/cell_editor.rs` (created here with only these two functions):

```rust
/// What a typed value fails, as the user reads it. `type_name` is the
/// column's type as the grid's header shows it (the page's `ColumnMeta`:
/// `int8`, where the structure says `bigint`); `typed` is the text that
/// failed, where the caller has it.
pub fn problem_text(problem: &Problem, type_name: &str, typed: Option<&str>, locale: Locale) -> String

/// Why a cell cannot be edited.
pub fn lock_text(lock: Lock, table: &str, locale: Locale) -> String
```

| Problem | Text |
|---|---|
| `WholeNumber` | `{type} expects a whole number` |
| `OutOfRange` | `{type} holds {min} to {max}` |
| `Number` | `{type} expects a number` |
| `Decimals` | `Up to {scale} decimals. {typed} would be stored as {stored}.` (the typed text is passed in by the caller that has it; where it is not at hand, `Up to {scale} decimals.`) |
| `Digits` | `At most {whole} digits before the point` |
| `Boolean` | `{type} expects true or false` |
| `NotOneOf` | `Not one of: {a}, {b}, {c}` |
| `Json` | `{Message} at {line}:{column}` (first letter upper-cased) |
| `TooLong` | `At most {max} characters` |

| Lock | Text |
|---|---|
| `ReadOnly` | `This connection opens read-only` |
| `NotATable` | `Views cannot be edited` |
| `StructureLoading` | `The table's structure is still loading` |
| `NoKey` | `{table} has no primary key or unique index, so a row can't be targeted safely` |
| `KeyType` | `{table}'s key cannot be matched exactly, so a row can't be targeted safely` |
| `Saving` | `A save is running` |
| `Refreshing` | `The page is loading` |
| `KeyIsNull` | `This row's key is NULL` |
| `KeyInexact` | `This row's key holds text that was not read exactly` |
| `UnknownColumn` | `This column cannot be told apart in the table` |
| `Generated` | `Computed by the database` |
| `KeyColumn` | `Part of the row's key` |
| `Binary` | `Binary values cannot be edited yet` |
| `TooLarge` | `Values over 256 KiB cannot be edited yet` |
| `NoSuchCell` | empty |

  Each through `gettext`; in the terminal look through `look.label` where the neighbouring text is lower-cased.

**Tests**

- `grid.rs`: `a_marked_cell_is_tinted_and_its_row_is_marked`, for every look: with `Mark::Pending` on one cell and `RowMark::Changed`, the frame holds a fill of `Tone::Warning.fill` inside the cell's rect; on Omarchy the painted texts include `~` in `palette.warning`; with `Trouble`, the danger fill and `!`. `a_double_click_reports_its_cell`: two presses at one cell's position report it in `double_clicked`.
- `ui/mod.rs`: `a_computed_column_is_drawn_locked_in_a_table_that_can_be_edited` (macOS and Windows: the surface fill in a generated column's cells, and none of it on a read-only connection or while nothing is known of the structure). `a_pending_cell_shows_its_new_value_and_what_it_was`: after `type_into`-style actions (apply the actions directly), the frame paints `bob@example.com` and not `user2@example.com`, and hovering the cell (`Event::PointerMoved` at `cell_of`, then frames past the tooltip's delay, as `quick_open_shortens_long_names…` waits for its tooltip) shows "was user2@example.com". `a_saved_cell_is_green_for_a_moment`: after a written save the cell's rect holds the success fill; with `edits.saved.at` back-dated by two seconds it does not. `a_sql_result_is_drawn_as_before`: an existing SQL result test still passes untouched (run the file's tests).
- `cell_editor.rs`: `every_problem_and_every_lock_has_words`: each variant gives a non-empty text (but `NoSuchCell`), with the spec's exact sentences for `WholeNumber` (`int8 expects a whole number`), `Decimals`, `NotOneOf`, `TooLong`, `NoKey`, `ReadOnly`, `KeyIsNull`.

Two commits, each green on the four checks: first the words (`cell_editor.rs` with `problem_text`, `lock_text` and their test), "Word what a value fails and why a cell is locked"; then the drawing, "Draw pending, failed and saved cells in the grid".

---

### Task 7: The editor on the cell, on macOS and Windows

**Files:**
- Modify: `src/ui/grid.rs`, `src/ui/cell_editor.rs`, `src/ui/data_view.rs`, `src/ui/keys.rs`
- Test: `src/ui/mod.rs`, `src/ui/keys.rs`

**The field.** `grid::show` gains `editor: Option<&mut dyn FnMut(&mut egui::Ui, egui::Rect)>` and `editing: Option<CellPos>`: when it draws the cell `editing` names (and the editor is not the large one), it calls the closure with the cell's rect instead of painting the cell's text. The cell must be on screen for the field to keep the keyboard (egui drops the focus of a widget that is not drawn): while `editing` is some, treat it as a selection change for the reveal logic (around line 523), and keep `lit` true so the pane does not look as if it lost the keyboard. `sql_results.rs` passes `None`, `None`.

`cell_editor::field(ui, rect, editor: &mut Editor, column: &ColumnInfo, look, palette, locale) -> Outcome`, built on the `where_line` field in `data_view.rs` (a frameless `TextEdit::singleline` in a child `Ui` at `rect`, the data role's font, `typography::layouter`):

```rust
#[derive(Default)]
pub struct Outcome {
    pub changed: bool,
    /// Enter, Tab or Shift+Tab ended the edit.
    pub commit: Option<Advance>,
    /// Esc.
    pub cancel: bool,
    /// The keyboard went elsewhere.
    pub left: bool,
    /// Alt+Enter.
    pub large: bool,
}
```

- `editor.focus` is taken once (`std::mem::take`) to `request_focus()`, the cursor at the text's end (set the `TextEdit` state's cursor range; see how `sql_text.rs` does it).
- Keys are read **before** the field is added, with `ctx.input_mut(|input| input.consume_key(..))`, only while the field has the keyboard: Alt+Enter (`large`), Enter (`commit: Down`), Tab and Shift+Tab (`commit: Right`, `Left`), Esc (`cancel`). Without that the field would take Enter as "give up the keyboard" (its match is logical, so Alt+Enter too), Tab would move the focus to a column header, and egui would drop the focus on Esc before any app code runs: set `.lock_focus(true)` and the focus lock filter for Tab and Esc as `sql_text.rs` does (around line 744), and mind its note that the filter holds from the frame after the field first has the keyboard.
- `response.changed()` gives `changed`; `response.lost_focus()` with none of the keys above gives `left`.
- The frame: `focus::hint(ui, &response, rect, Ring::Field { .. })` for the accent border and halo, clipped to the grid's clip rect. With `editor.problem` the border and halo are in `palette.danger` (if `focus` has no ring of another colour, add a `Ring` variant there rather than painting one in the view) and the message (`problem_text`) is painted under the cell in `palette.danger`, over the row below.
- A column with a length (`ColumnClass::Text { max_chars: Some(max) }`) shows `{chars} / {max}` at the field's right in `palette.dim`.

Until task 8 draws the large editor, `data_view::show` draws a large one (`editor.large`) with this field too, so an editor is never open without being on screen. The grid's `double_clicked` becomes `Action::EditCell { start: EditStart::Value }` here, in the desktop looks only: until task 13 gives Omarchy its keys and a way to write, nothing there opens an editor, so no pending change can be made that could not be saved.

In `data_view::show`, take the tab's editor mutably up front as `filter_bar.rs` and `where_line` take their text (the page and the structure are then read from the same `ObjectTab`; restructure the borrows as those two do), pass the closure, and turn the `Outcome` into actions: `EditorTyped`, `CommitEdit { then }`, `CancelEdit`, `LeaveEdit`, `EditorBreak`. **`EditorTyped` is pushed first** whenever the text changed in the frame: an editor that was not typed into closes without a change, so a first keystroke that shares its frame with Enter or a click away would be lost if the commit went ahead of it. For the same reason, when Mod+S arrives while a field has the keyboard, `keys::handle` pushes `EditorTyped` for the open editor before `WriteEdits` (it cannot know whether this frame's text changed; marking it typed is harmless, since an unchanged text is no change).

**The note of a locked cell.** When `edits.why` names a cell, draw `lock_text` as a small note at that cell (the same dark tooltip the hint uses, shown without hover) until the selection moves.

**Keys, in `keys::handle`** for a table's grid in the desktop looks (`!look.terminal`), under the same condition the arrows run under (no text field has the keyboard, the grid has it, no control is focused):

| Key | Action |
|---|---|
| Enter, F2 | `EditCell { start: Value }` on the selected cell |
| a typed character (an `Event::Text` that is not a space, not `?`, with no Mod held) | `EditCell { start: Typed(text) }` (on a cell that cannot be edited this does nothing) |
| Mod+Backspace | `SetNull` |
| Mod+Z | `RevertCell` |
| Mod+S | `WriteEdits` (also while a field has the keyboard, so a save takes what is being typed) |
| Mod+Alt+Backspace | `DiscardEdits` |

Typed characters: consume the `Event::Text` so the field that opens does not get it twice, and ignore text that comes with a consumed shortcut (Space leaves `Text(" ")` behind). `SHORTCUTS` gains the rows: "Enter, F2" / "Edit the cell", "Tab, Shift+Tab" / "Commit and move right or left", "Esc" / "Cancel the edit", "Mod+Backspace" / "Set NULL", "Mod+Z" / "Revert the cell", "Mod+S" / "Save all pending changes", "Mod+Alt+Backspace" / "Discard all pending changes"; update `the_shortcut_table_covers_the_spec_map`.

**Tests** (`ui/mod.rs`, macOS and Windows looks unless said):

- `enter_edits_the_cell_and_enter_again_commits_and_moves_down`: select `(1, 1)`, press Enter: `harness.ctx.text_edit_focused()`, the editor holds the loaded text; `type_text("x")`, Enter: the cell is pending with the text ending in `x`, the selection is `(2, 1)`, no field has the keyboard.
- `typing_starts_the_edit_with_that_character`: `type_text("z")` on a selected cell: the editor's text is `z`. Space and `?` do not open one. On the key column, and on a read-only connection, typing opens nothing and paints no note.
- `tab_commits_and_moves_right_and_shift_tab_left`; `escape_drops_the_edit_and_leaves_the_grid_the_keyboard` (the arrows move the selection on the next press).
- `f2_and_a_double_click_edit_the_cell` (the double-click by `click_at` twice at `cell_of`).
- `a_value_the_column_does_not_take_keeps_the_field_and_shows_why`: on a table whose structure has an `int8`-like column (build the page and structure in the test), type `1a`, Enter: the field still has the keyboard, the message "INTEGER expects a whole number" is painted in `palette.danger`; clicking another cell leaves it as a cell to fix.
- `a_locked_cell_says_why_instead_of_opening`: Enter on the key column paints "Part of the row's key".
- `mod_backspace_sets_null_and_mod_z_reverts`; `mod_s_saves_and_mod_alt_backspace_discards` (a `Command::Write` is sent; the set is empty).
- `a_sql_result_takes_none_of_the_editing_keys`: Enter and typing on a SQL result open nothing.
- `keys.rs`: the table lists the new rows.

Three commits, each green on the four checks: the field and its wiring ("Edit a cell in place on macOS and Windows"); the keys and the shortcuts table ("Open, set NULL, revert, save and discard from the keyboard"); the note of a locked cell ("Say why a cell cannot be edited when it is asked for").

---

### Task 8: The large editor

**Files:**
- Modify: `src/ui/cell_editor.rs`, `src/ui/data_view.rs`, `src/ui/grid.rs` (the edited cell's rect in `GridOutput`)
- Test: `src/ui/mod.rs`

When `editor.large`, the grid draws the cell as pending-in-progress (its accent line) and reports its rect (`GridOutput::editing_rect: Option<Rect>`); `data_view::show` then calls `cell_editor::large(ctx, anchor, editor, column, look, palette, locale) -> Outcome`:

- An `egui::Area` in the foreground order at `sql_complete::place(anchor, size, screen)` (already public), about 420 by 180 pt, drawn with the completion list's `panel` (private in `sql_complete.rs` today: make it `pub(crate)`, or move it to `widgets.rs` if that reads better; raised fill, border, radius, shadow; Omarchy: `palette.panel` with a 1 pt accent border) and the accent border of a focused field.
- A `TextEdit::multiline` in a scroll area, the data role's font. Under it a band: at the left `{chars} chars · {lines} lines` (the count grouped as `format::group_number` groups), at the right `{Mod}↩ apply · esc cancel` with `look.command_key()`. The terminal look's band (`ctrl+enter apply · esc keep`) and its Esc are task 13's: this task draws the desktop band in every look.
- Keys, read before the field is added: Mod+Enter gives `commit: Stay`; Esc gives `cancel`. Enter is the field's own (a line break). Tab is the field's own.
- A problem shows as in the small field: the border in the danger colour, the message in the band's place at the left.
- A click outside the area gives `left`. The Esc owners that ask `egui::Popup::is_any_open` do not see an `Area`: nothing else may act on that Esc, which the consumed key ensures.

**Tests**

- `a_json_cell_opens_the_large_editor`: Enter on `(0, 2)`: the editor is large, the band's text is painted with the right counts.
- `alt_enter_moves_a_one_line_edit_into_the_large_editor`: the text gains a line break and `large` is set, the field keeps the keyboard.
- `mod_enter_applies_and_escape_cancels_the_large_editor`; `invalid_json_cannot_be_applied` (Mod+Enter leaves the editor open with the message painted).
- `a_long_value_is_edited_whole`: a cell holding 300 characters opens large with all 300 (the grid shows the cut, the editor the value).

Commit: "Edit long values, broken text and JSON in a popover".

---

### Task 9: The pending bar, the tab's mark and what a save says

**Files:**
- Create: `src/ui/pending_bar.rs`
- Modify: `src/ui/workspace.rs` (the panel), `src/ui/object_tabs.rs`, `src/ui/data_view.rs` (the footer's state note), `src/ui/sql_results.rs`
- Test: `src/ui/mod.rs`, `src/ui/object_tabs.rs`

- **The bar** (macOS, Windows): `pending_bar::show(app, ui, tab, object_tab)`, an `egui::Panel::bottom` of 48 pt added in `workspace::show` right after the `data_view::footer` call (bottom panels stack upward in call order), drawn only while `edits.cells` is not empty, a save runs, or `edits.note` is some. Fill `Tone::Warning.fill`, a hairline on top in `Tone::Warning.line`. Left to right: the 8 pt dot in `Tone::Warning.color`; `{n} changes in {m} rows` (singulars: `1 change in 1 row`) in the strong body role; `{n} to fix` in `palette.danger` when there are any; the note, when there is one; at the right `Discard all` (`ButtonSpec`, bordered) and `Save` (`ButtonSpec::primary` with `.shortcut("Mod+S")`), pushing `DiscardEdits` and `WriteEdits`. Discard all is disabled while a save runs (the reducer ignores it then). Save is disabled through `app.save_blocked(tab, id)` with the reason as its tooltip: `Fix {n} value(s) to save`, `Not connected`, `This connection opens read-only`, `Saving…`. While a save runs the bar shows `Saving…` with a spinner and a cancel that pushes `CancelQuery(tab)`.
- **The note's words** (`pending_bar::note_text(note, object, page, structure, locale)`), also used by Omarchy in task 14:

| Note | Text |
|---|---|
| `Conflict { gone: false }` | `Row {key} changed on the server. Nothing was written.` |
| `Conflict { gone: true }` | `Row {key} no longer exists on the server. Nothing was written.` |
| with `others > 0` | adds ` {others} more rows too.` |
| `Failed` | `{code} · {message}. Nothing was written.` (the code only when the error has one) |
| `Lost` | `The connection was lost while saving. Reload to see what was written.` |
| `Cancelled` | `Save cancelled. Nothing was written.` |
| `Refused` | `{the error's text}. Nothing was written.` |

  `{key}` is the row's key as the row panel's title writes it (`id 2`; several columns joined by `, `); read how `row_panel.rs` builds "Row · id 2" and share that.
- **After a save that wrote,** while `edits.saved` is some and nothing is pending, the footer's right side shows `written {n} changes · {m} rows · {elapsed}` (in place of "Query {elapsed}"), until the next edit or page.
- **The tab's mark:** a tab whose `edits.cells` is not empty shows an unsaved dot. On macOS and Windows the dot takes the close button's place while the tab is not hovered (the tab's width must not change: `a_mac_tab_is_as_wide_in_every_state` stays green); its a11y name becomes `{name} tab, unsaved`. Omarchy's `[+]` is task 14.
- **The footer's state note** (`state_note`): unchanged. The row panel's footer and the header's Add row: unchanged (decision 18).
- **The refused-write card** (`sql_results.rs`, `blocked`): on a writable connection the body gains the sentence "Edit values in a table's grid." (the spec's "From step 3 on"). Update `a_refused_write_says_the_editor_only_reads` for both kinds of connection.

**Tests**

- `the_bar_counts_what_is_pending_and_save_writes_it` (macOS, Windows): no bar without edits; with two cells in two rows `harness.has("2 changes in 2 rows")`; `harness.click("Save")` sends a `Write`; `harness.click("Discard all")` empties the set and the bar goes.
- `save_is_disabled_with_its_reason`: a cell to fix: the Save button's node is disabled and "Fix 1 value to save" is its description; disconnected: "Not connected".
- `a_conflict_is_a_line_in_the_bar_and_the_set_stays`: after `Conflicts`, `harness.has("Row id 2 changed on the server. Nothing was written.")`.
- `a_failed_save_marks_the_row_red_and_says_the_databases_words`.
- `a_written_save_is_summed_up_in_the_footer`.
- `object_tabs.rs`: `a_tab_with_pending_changes_carries_the_mark_and_keeps_its_width`.

Commit: "Show what is pending in a bar, with Save and Discard".

macOS and Windows edit and save here, with the stand-in prompts of task 5. The branch is not shippable yet: see "Where a run can stop".

---

### Task 10: The two prompts, in every look

**Files:**
- Modify: `src/ui/write_prompts.rs` (replacing the stand-in)
- Test: `src/ui/mod.rs`

Both are `widgets::modal`s read from `app.dialog`. Their keys are read inside their own `show` (no shortcut runs under a dialog), before the buttons are drawn where a focused button could take the key (see `connect_dialog`'s `take_mod_key`). The Omarchy layouts need the terminal-style pieces that are private to `connect_dialog` (`terminal_header`, `terminal_footer` and its hidden buttons): move what both use into `widgets.rs` or a small shared module rather than copying it, and say in the report what moved.

- **Leave** (macOS, Windows): title `Save {n} changes before {what}?` where `{what}` is worded from the held action ("closing the tab", "reloading", "changing the page", "sorting", "filtering", "disconnecting", "switching database", "closing the window"); for several tabs `Discard {n} changes in {m} tabs?`. Body: "They have not been written." Buttons: **Cancel** (Esc), **Discard**, and **Save** (primary, Enter) when `can_save`; without it **Discard** is not the default: Enter does nothing, so an unsaved set is never dropped by a stray key.
- **Leave** (Omarchy): a box with the line `closing with pending edits` (or the action's own words) and the three choices as key hints that are also hidden buttons: `[w] write` (only with `can_save`), `[d] discard`, `[esc] stay`. The letters are read as typed text.
- **Save to production** (macOS, Windows): the modal's frame with a 4 pt band on top in `Tone::Danger.color` (the production bar's red; `env_colors` may not be called from a new view, see "Before you start"); title `Save {n} changes to production?`; `{connection} · {database} · {m} rows in {table}`; the statements in `widgets::code(look)`, one per line, in a bordered box at most 220 pt tall that scrolls, each line cut at the box's width (display only); foot: "One transaction" in the secondary colour, **Cancel** (Esc), **Save to production** (a danger-filled `ButtonSpec`; if there is no such style, add one to `ButtonSpec` rather than painting it here). Enter does not confirm: the button is pressed.
- **Save to production** (Omarchy): the frame's stroke 2 pt in `palette.danger` (`.frame(..)` on the modal, as the connection dialog overrides it); a head tinted `Tone::Danger.fill` with the `PROD` chip and `write {n} changes?`; `{connection} · {database}`; `{m} rows in {table} · {columns}` dimmed; the statements as above; `type write to confirm`; a field bound to `prompt.typed` with a danger border, focused once through `prompt.focus`; foot hints `enter confirm`, `esc cancel`. Enter pushes `ConfirmWrite` only when the field holds exactly `write`; anything else does nothing.

**Tests** (each `for look in Look::ALL`, with `finish_animations()` before asserting on a just-opened modal):

- `leaving_asks_and_each_choice_does_what_it_says`: close a tab with a pending cell; Cancel keeps both; Discard closes the tab; Save sends a `Write`. In the terminal look by `type_key` of `d`, `w` and Esc.
- `no_save_is_offered_where_save_is_disabled`: with a cell to fix there is no Save button (and `w` does nothing).
- `enter_never_discards`: Enter on a prompt without Save leaves it open.
- `a_save_to_production_shows_its_statements_and_is_confirmed`: the statement's text is painted; Cancel sends nothing; the red button sends the `Write` (desktop); in the terminal look Enter with `wri` typed sends nothing and with `write` sends it.
- `the_production_prompt_wears_the_danger_colour`: a fill or stroke of `Tone::Danger.color` is in the frame, in every look.

Commit: "Ask before leaving pending changes and before a save to production".

---

### Task 11: Closing the window

**Files:**
- Modify: `src/app.rs` (`frame_ui`, `Held::CloseWindow`), `src/model.rs`, `src/testing.rs` (a close request in `Harness::frame`), `src/ui/write_prompts.rs` (the wording)
- Test: `src/ui/mod.rs`

- `Held` gains `CloseWindow`. `App` gains `closing: bool`.
- In `frame_ui`, before the views are drawn: when `ctx.input(|input| input.viewport().close_requested())` and `!self.closing`, collect every table tab holding edits over all workspaces; if there are any, send `egui::ViewportCommand::CancelClose` and `self.hold(Held::CloseWindow, tabs)`. `perform(Held::CloseWindow)` sets `self.closing = true`, and `frame_ui` sends `egui::ViewportCommand::Close` in every frame while `closing` is set (the harness keeps only the last frame's commands, and a window that is closing has nothing else to do).
- With several tabs the prompt has Discard and Cancel only, as for a connection. While a save runs the close is cancelled and nothing is asked (the guard ignores actions under a save): the user closes again once it ends.
- `Harness::frame` builds its `RawInput` by hand: add a `close_requested: bool` field on the harness that sets the root viewport's close-request event for one frame (read the fork's `ViewportInfo::close_requested` to see which event it reads).

**Tests:** `closing_the_window_with_pending_changes_asks_first`: a close request with a pending cell leaves `ViewportCommand::CancelClose` in `harness.viewport_commands` and the Leave prompt up; Discard leads to `ViewportCommand::Close` in a later frame; without edits a close request is not cancelled.

**By hand, by the user** (no window can open in an agent's session): on each platform, close the window with a pending cell, and on macOS quit with Cmd+Q. If Quit does not pass through a close request there (the app would exit without asking), say so in the report: holding it back needs the application delegate's `applicationShouldTerminate`, which belongs to `crates/tabletist-appkit` and is its own change.

Commit: "Ask before the window closes on pending changes".

---

### Task 12: The row panel shows what is pending

**Files:**
- Modify: `src/app.rs` (`format_rows`), `src/ui/row_panel.rs`
- Test: `src/ui/mod.rs`

The panel must never disagree with the grid. `format_rows` formats the selected row's fields from the page; for a table tab, format a pending cell's **new** value in place of the loaded one (a `Value` made from the new text, or NULL), and keep, beside each field's text, whether it is pending and what it was (`RowFields` gains `pending: Vec<Option<String>>`, the loaded value's short text per field, `None` where nothing is pending). The reducer already drops `object.fields` whenever the set changes, so the cache follows. In `row_panel::field`, a pending field's label line gains the pending mark (the amber dot; Omarchy `~` in the warning colour) and under the value "was {loaded}" in the caption role and `palette.dim`. The panel stays read-only; its editing footer is unchanged.

**Tests:** `the_row_panel_shows_a_pending_value_and_what_it_was` (every look): after a pending edit of the selected row, the panel paints the new text and "was user2@example.com"; after a revert it paints the loaded text and no "was".

Commit: "Show a pending value in the row panel".

---

### Task 13: Omarchy, the keys and insert mode

**Files:**
- Modify: `src/ui/keys.rs` (`letters`), `src/ui/cell_editor.rs`, `src/ui/data_view.rs`, `src/ui/workspace.rs` (the key hints)
- Test: `src/ui/mod.rs`, `src/ui/keys.rs`

A double-click edits the cell here too now (task 7 held it back for this look).

**Keys in normal mode** (in `letters`, for a table's grid; a SQL result keeps today's keys):

| Key | Action |
|---|---|
| `i`, Enter | `EditCell { start: Value }` (no longer the row panel; Space and Mod+Shift+R keep it) |
| `c` then `c` | `EditCell { start: Replace(String::new()) }` (the pending-key mechanism of `gd` and `za`) |
| `x` | `SetNull` |
| `u` | `RevertCell` |
| Ctrl+S | `WriteEdits` |

`s` stays Structure. Update the tests that assert `i` and Enter open the row panel (`the_terminal_keys_of_the_row_panel_work_on_a_table_row` and its neighbours): on a table they now edit; on a SQL result they still open the panel. The status line's hints `enter inspect` and `i inspector` (`workspace.rs`, `table_hints`) become `space inspect` and, on a table whose selected cell can be edited, `i edit`. `SHORTCUTS`' Omarchy row gains `cc`, `x`, `u`, Ctrl+S.

**Insert mode** is the editor being open. In the terminal look the field (`cell_editor::field`) differs in two keys: Esc **keeps** the edit (`left`, which the reducer takes as `LeaveEdit`) and does not close the row panel (the `was-editing` flag in `keys::after_frame` already holds that back); Ctrl+C drops it. Ctrl+C reaches the app as `egui::Event::Copy`, not as a key: while the field has the keyboard, take that event out of the input before the field sees it and answer `cancel`. The field's look: a 2 pt accent line inside the cell (`Ring::Own` and the cell's stroke, as the cursor cell is drawn), no halo. The large editor's Esc keeps too, Ctrl+C drops, and its band reads `ctrl+enter apply · esc keep`.

With a save possible (Ctrl+S) and nothing yet to show the counts, this task also shows `{n} pending · {m} rows` in the status line whenever the set is not empty; task 14 gives the line its full form.

**Tests**

- `i_and_enter_edit_the_cell_and_escape_keeps_the_change`: `type_key(Key::I, "i")` opens the editor; type, Esc: the cell is pending and no field has the keyboard; the row panel did not close.
- `ctrl_c_drops_the_edit` (send `Event::Copy` while the field has the keyboard: no pending cell, and nothing was copied to the clipboard).
- `cc_replaces_x_nulls_and_u_reverts`; `ctrl_s_writes`.
- `space_still_opens_the_row_panel_and_a_sql_result_keeps_its_keys`.
- `a_double_click_edits_in_the_terminal_look`.

Commit: "Edit and write from the keyboard on Omarchy".

---

### Task 14: Omarchy, the mode line, the error line and the tab

**Files:**
- Modify: `src/ui/workspace.rs` (`status_line`), `src/ui/data_view.rs` (the error line), `src/ui/object_tabs.rs`
- Test: `src/ui/mod.rs`

**The mode line** (`status_line`): while the active table tab's editor is open, the left side reads `-- INSERT --` in `TextRole::OModeLine` and `palette.success`, then `{column} · {type}`, then `{n} pending · {m} rows` in `palette.warning`, `{n} error(s)` in `palette.danger`, and at the right `esc normal · tab next cell`. Outside insert mode, when the set is not empty, the counts stand after the key hints (they are never dropped for width: drop hints first). In the struck-through list, `e edit` goes (the live `i edit` of task 13 replaced it) and `:w write` stays struck until task 15; `o new row` and `dd delete` stay struck. After a save: `✓ written {n} changes · {m} rows · {elapsed}` with the mark in `palette.success`; a note: `≠ conflict row {key} changed on the server`, `✗ failed {code} {message}` and "rolled back · cells stay pending in red". While a locked cell was asked for: the lock's text, in place of a note at the cell.

**The error line** under the grid (`data_view::show`, terminal look): for the first cell to fix or failed, `! {row number}:{column}  {message}` in `palette.danger`, a line of the body role's height above the status line.

**The tab:** `[+]` after the name of a table tab with pending changes, in the tab's own colour.

**Tests**

- `insert_mode_is_named_in_the_mode_line_with_the_column_and_the_counts`: `-- INSERT --` painted in `palette.success`, `email · TEXT`, and after Esc the counts `1 pending · 1 row` in `palette.warning`.
- `the_gutter_and_the_error_line_show_a_cell_to_fix` (`!` in the gutter, `! 2:meta  …` painted in the danger colour).
- `a_locked_cell_says_why_in_the_mode_line`.
- `the_tab_of_a_table_with_pending_changes_ends_in_a_plus`.
- `a_written_save_and_a_conflict_are_said_in_the_status_line`.

Commit: "Say what is pending in Omarchy's mode line".

---

### Task 15: Omarchy, the `:` prompt

**Files:**
- Modify: `src/model.rs` (`Workspace::command`, `command_error`, three actions), `src/app.rs` (the arms and `run_command`), `src/ui/keys.rs` (`:`), `src/ui/workspace.rs` (`status_line`)
- Test: `src/app.rs`, `src/ui/mod.rs`

State on the workspace: `Workspace::command: Option<String>` (the text after the colon; `None` when closed) and `command_error: Option<String>`. `Action::OpenCommand(tab)` opens it (typed `:` on a table's grid in normal mode), `Action::CloseCommand(tab)` closes it, `Action::RunCommand(tab)` runs it. While it is open the status line draws `:` and a frameless field bound to the text (the `where_line` field again), focused. Enter runs: `w` is `WriteEdits` on the active table tab, `e!` is `DiscardEdits`; anything else leaves `command_error = Some(text)`, which the status line shows as `not a command: {text}` in `palette.danger` until the next key. Esc closes. The reducer does the parsing (`fn run_command(&mut self, tab: ConnTabId)`), so its three cases are a reducer test. `:w write` in the status line is no longer struck through. `SHORTCUTS`' Omarchy row gains `:w` and `:e!`.

**Tests**

- `app.rs`: `the_prompt_runs_w_and_e_bang_and_refuses_the_rest` (a `Write` is sent; the set is emptied; `diff` and `wq` leave the error and send nothing).
- `ui/mod.rs`: `colon_opens_the_prompt_and_enter_runs_it`; `escape_closes_the_prompt`; `an_unknown_command_says_so` (painted in the danger colour).

Commit: "Write and discard from Omarchy's command prompt".

**All three looks edit and save here.**

---

### Task 16: Scenes, documents and every check

**Files:**
- Modify: `src/shots.rs`, `docs/superpowers/specs/2026-10-03-value-editing-core-design.md`, `docs/superpowers/specs/2026-09-27-tabletist-design.md`, `README.md` (if it says the app only reads)

- **Scenes** in `src/shots.rs`, on the Bookshop data (never other data), for review by eye: `edit-pending` (three pending cells in two rows, one to fix, the bar), `edit-field` (the field open with a failing value), `edit-large` (the popover on the JSON column), `edit-saved`, `edit-failed`, `edit-leave` (the prompt), `edit-production` (on `production_beside`'s connection). The page there needs a structure answered (`harness.answer_structure(structure())`). Shots are not run in the suite and not committed.
- **The value-editing spec:** the status line (steps 1 to 3 built); "Editing in the grid" as built, with this plan's twenty decisions where they add to it or depart from it (decision 19 departs: only a computed column is drawn locked): the guard and what it covers (the spec's list plus Clear sort, Retry, dropping one filter and following a foreign key into an open tab); that a cell is locked while its page loads again and while a save runs; that Mod+. cancels a save; the locks added for keys a save would refuse; what Enter does in a prompt without Save; Omarchy's `:diff` arriving in step 4; what the window-close check found.
- **The main spec:** success criterion 6, section 4.3 (the app now writes, through the grid), section 5.7 (Omarchy's `i` and Enter) and the keyboard table, as the value-editing spec's "Documents this changes" lists; `src/edit.rs` and the new `ui` files in the layout of 3.2.
- **Every check,** with both server URLs exported:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
~/.cargo/bin/cargo test --locked --workspace --all-targets
```

- **By hand, by the user,** on the demo (`--demo` opens a writable SQLite file): edit two cells in two rows and save; edit a cell to a value its column refuses; edit, then close the tab and take each of the three answers; on a production-labelled connection, save and cancel, save and confirm; in the Omarchy look, the same with `i`, Esc, `:w`, `:e!`.

Commit: "Describe editing in the grid as built".

---

## What this plan leaves for later steps

- **Step 4, Review SQL:** the bar's Review SQL button and drawer, Omarchy's `:diff` panel, the statements kept up to date in the reducer as the set changes, and the PROD box pointing at the panel instead of listing the statements.
- **Step 5, the conflict dialog:** loaded, now on server, yours; Keep mine, Use server values, Overwrite. Until then a conflict is a line and the user refreshes.
- **A cancel between two of a save's statements is lost** on PostgreSQL and MySQL (step 2 found it): the save commits and says so. The bar's cancel is honest only while a statement runs. Whether the Saving state should say more is open.
- **MySQL stores some values adjusted without a word** (`'1.6'` into a `TINYINT` is 2). The checks here catch a non-integer in an integer column; a `FLOAT`'s precision is not checked.
- **A MySQL `TIMESTAMP` as a changed column** can miss a conflict in a repeated daylight-saving hour; the key case is locked, the cell case is not.
- **A table whose engine has no transactions** (MyISAM) is known only when the save refuses it: `Structure` has no field for the engine. The same holds for a table of a database attached to a SQLite session: the grid lets it be edited and the save refuses it.
- **Undo and redo, pasting, the editors by type, the row form:** slices 2 to 4 of the spec's "Editing as a whole".
