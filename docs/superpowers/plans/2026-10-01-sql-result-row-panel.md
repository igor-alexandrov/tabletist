# Row panel for SQL results Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Selecting a row in a SQL editor's result grid opens the right-hand row panel on that row, as selecting a table's row fills it.

**Architecture:** One question, "which tab's row does the row panel show", gets one answer in the model (`Workspace::row_panel_tab`): an object tab in its Data view, or a SQL tab with a selected result row. The workspace view, the reducer that formats the row's text, and the keys all ask it. The panel itself stops reading an `ObjectTab` and reads a small `Source` (columns, rows, selection, optional structure, row offset, name) that either kind of tab fills.

**Tech Stack:** Rust 2024, egui (crmne fork). Headless UI tests through `src/testing.rs` (AccessKit).

**Spec:** `docs/superpowers/specs/2026-09-30-sql-editor-core-design.md` (written "as built"; Task 6 updates it with this change). The user's words: "The right panel should be opened when row is selected. Just like we open it for tables."

**Base:** `origin/main` at `8a87e42` or later (the SQL editor merged in PR #35 of igor-alexandrov/tabletist). Line numbers below are from `8a87e42`.

**Drafted once:** the code in this plan was drafted against `8a87e42` and run: the new UI tests failed before the change and passed after it, and fmt, both clippy runs, the whole test suite (742 library tests, 728 before) and rustdoc were clean. The draft was set aside so the tree is clean for execution. If a step does not behave as written, the base has moved: read the code, do not force the snippet.

---

## Product decisions (for the user, before execution)

The plan below is written for the recommended choice of each. A different choice changes the named tasks as described; say so before execution starts.

### 1. When does the panel appear on a SQL tab?

- **A (recommended, planned): only while a result row is selected.** The user's literal wording. The editor and the results keep the whole width until a row is picked. A new run clears the selection (it already does, `src/app.rs:1906`), so the panel closes until a row of the new result is picked.
- B: always there, with the "Select a row to see its fields" placeholder, as on a table tab. A table tab works this way today because `Workspace.row_panel` starts `true`. On a SQL tab it would take 345 pt (macOS) or 462 pt (Omarchy) from the editor before anything has run.

Either way the panel is the workspace's one row panel: its open or closed state (`Workspace.row_panel`, open when a connection opens) and its width are shared with table tabs. What that means once the user has closed it is decision 6.

*If B:* in Task 1 `row_panel_tab` returns `Some(sql.id)` for every SQL tab, and Task 2's `source` gives an empty `Source` when the editor has no rows; in Task 4 the keys drop the "a row is selected" gate; the tests' "no panel before a row is selected" assertions flip.

### 2. Where does it sit?

- **A (recommended, planned): full height, beside the editor and the results.** The same `egui::Panel::right` as a table's, at the same place in `workspace::show`, so one remembered width and no new layout. The editor gets narrower while a row is selected.
- B: beside the results only. The editor keeps its width, but the panel gets only the results' share of the height (55% by default), which is short for a row with many fields or a JSON document, and it needs a second placement inside `sql_results`.

*If B:* Task 3 draws the panel from `sql_results::draw` instead of `workspace::show`, and the "editor gets narrower" assertion becomes "editor keeps its width". This is the one alternative that was not drafted.

### 3. Hidden while the Messages pane shows?

- **A (recommended, planned): yes.** Messages shows no rows, so there is no row on screen for the panel to be about. The selection is kept: back on Results the panel is back.
- B: stays while Messages shows.

*If B:* in Task 1 `selected_row` drops the pane check, and Task 3's Messages test flips.

### 4. Which keys reach it on a SQL tab, and the Omarchy strip toggle

- **Recommended, planned:** every key a table has, while a result row is selected: Space (outside the editor, where it types a space) and `Mod+Shift+R` toggle it; on Omarchy `i` toggles, Enter opens, Esc closes, `za` folds its documents. With no row selected these do nothing: under 1A there is no panel to show or hide, and a silent flip of the shared state would make the next selected row open nothing. Omarchy's `[` and `]` step rows whenever the result grid shows (they move the selection, like `j` and `k`). The Omarchy strip's "Show or hide the row panel" toggle shows on SQL tabs too (its tint shows the state, so it acts even with no row selected).
- Stays table-only: `y`, `gd`, `s`, `d`, `/`, and `Mod+C`. Copying a cell or row of a result is "Copy" in the spec's Out of scope list. The panel's own per-field copy buttons do work on a result. Because `y` does nothing on a result, the Omarchy panel's hint beside a JSON field reads "za fold" there, not "za fold · y copy" (Task 2).
- Not changed: the Omarchy status line's hints on a SQL tab (it does not offer the panel's keys).

*If fewer keys:* drop the matching hunk in Task 4, or all of Task 5 for the strip toggle.

### 5. The editing footer

- **A (recommended, planned): a result's panel has no footer.** The disabled Edit / Duplicate / Delete controls and "Editing arrives in a later version" are about table rows; a result row is no table's row to edit. The fields get the height.
- B: show it as on a table. *If B:* in Task 2 the footer is drawn whatever `source.table` says, and Task 3's footer test goes.

### 6. After the user closes the panel, does selecting a result row reopen it?

- **A (recommended, planned): no, as on a table.** Closing the panel (its button, Space, `Mod+Shift+R`, Omarchy's Esc, `i` or the strip toggle) turns the workspace's one panel off, for table tabs and SQL tabs alike, until it is toggled back. Selecting a row then selects it without a panel. Note what this means in the macOS and standard looks: there is no on-screen toggle, so the way back is Space or `Mod+Shift+R` with a row selected (as on a table today).
- B: clicking a result row always opens it, which reads "should be opened when row is selected" most literally. The panel would then behave differently on the two kinds of tab, and a click would also reopen it for table tabs (the state is shared).

*If B:* Task 3 adds one line to `Action::SelectCell` in `src/app.rs` (set `workspace.row_panel = true` when the tab is a SQL tab and the cell is in its result), and the "Closed, it stays closed" assertions at the end of `the_panels_buttons_step_through_a_results_rows_and_close_it` flip.

### What a result's panel cannot show (no decision, for the record)

A result has no table structure behind it. So the panel is titled by the row's number ("Row 3", over "Query 1" on macOS), never by a key; no field is marked as primary key; there are no "Open ... →" foreign key links; and only booleans keep their tag colour. This is what `sql_results.rs` already does for the grid (`Tags::of(column, None)`).

---

## Ground rules (read first)

- Work in the worktree this plan is executed from. `cargo` through mise is broken here: always run `~/.cargo/bin/cargo`.
- **Never commit or push unless the user asks.** Each task ends with a "commit point": stop there and report. If the user has asked to commit: one topic per commit, signed (`git commit -S`); if signing fails, ask before committing unsigned. Never set `SSH_AUTH_SOCK` in git commands. Subjects follow the repo: an imperative sentence, no prefix. End messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No em dashes anywhere (code, comments, docs, commit messages). Use a full stop, comma, colon or parentheses.
- Tests are behavioural and headless (AccessKit through `src/testing.rs`). Never a design or pixel conformance test: nothing here asserts a size, a position or a colour from the design. Fixtures use the neutral demo data only (`users`, `user1@example.com`).
- Views push `Action`s and never mutate app state; `App::apply` reduces them. Views draw text only through `TextRole`s. User-facing strings go through `gettext`.
- Match the surrounding comment density: short comments saying what and why.
- The UI tests live in `mod tests` of `src/ui/mod.rs` and are in the library target: run them with `--lib`.

Full check commands (Task 6, and handy any time):

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

## File map

| File | Change |
|---|---|
| `src/model.rs` | `SqlTab.fields`, `SqlTab::selected_row`, `SqlTab::selected_fields`, `Workspace::row_panel_tab`; unit tests |
| `src/ui/row_panel.rs` | `Source` and `source()`: the panel reads a table's page or a SQL result; no footer and no `y copy` hint for a result |
| `src/ui/workspace.rs` | Draw the panel for `row_panel_tab()`, of either kind |
| `src/app.rs` | `format_rows` formats a selected result row once, frees it when no panel shows it |
| `src/ui/keys.rs` | Space, `Mod+Shift+R`, and Omarchy's `i`, Enter, Esc, `za`, `[`, `]` on a SQL result |
| `src/ui/object_tabs.rs` | The Omarchy strip's panel toggle on SQL tabs too |
| `src/ui/mod.rs` | Headless UI tests; two tests that pinned the old behaviour change |
| `docs/superpowers/specs/2026-09-30-sql-editor-core-design.md`, `docs/superpowers/specs/2026-09-27-tabletist-design.md`, `README.md` | Say what is built |

---

## Task 1: The model knows which row the panel is for

**Files:**
- Modify: `src/model.rs:1714-1738` (`SqlTab`), `:1764-1780` (`SqlTab::new`), `:1885-1888` (after `dims`), `:2012-2014` (after `active_sql_tab`)
- Test: `src/model.rs`, `mod tests` (after `a_run_that_failed_as_a_whole_drops_the_result_before_it`)

- [ ] **Step 1: Write the failing tests**

In `mod tests` of `src/model.rs`, after the test `a_run_that_failed_as_a_whole_drops_the_result_before_it`. The helpers `editor()`, `sql_tab(id)`, `object_tab(id, name, pinned)`, `run_script` and `rows_outcome` are already in scope there.

```rust
    #[test]
    fn a_sql_tabs_selected_row_is_one_its_results_pane_shows() {
        let mut sql = editor();
        sql.selection = Some(CellPos { row: 0, col: 0 });
        assert_eq!(sql.selected_row(), None, "nothing ran");
        run_script(&mut sql, "SELECT 1", vec![rows_outcome(3)]);
        sql.selection = None;
        assert_eq!(sql.selected_row(), None);
        sql.selection = Some(CellPos { row: 2, col: 1 });
        assert_eq!(sql.selected_row(), Some(2));
        sql.pane = ResultPane::Messages;
        assert_eq!(sql.selected_row(), None, "the messages show no rows");
        sql.pane = ResultPane::Results;
        // A selection past the result's rows is no row of it.
        sql.selection = Some(CellPos { row: 3, col: 0 });
        assert_eq!(sql.selected_row(), None);
    }

    #[test]
    fn a_sql_tabs_row_text_holds_for_its_row_and_its_run() {
        let mut sql = editor();
        run_script(&mut sql, "SELECT 1", vec![rows_outcome(3)]);
        sql.selection = Some(CellPos { row: 1, col: 0 });
        assert!(sql.selected_fields().is_none(), "not formatted yet");
        sql.fields = Some(RowFields {
            request: sql.run.loaded,
            row: 1,
            fields: Vec::new(),
        });
        assert!(sql.selected_fields().is_some());
        sql.selection = Some(CellPos { row: 2, col: 0 });
        assert!(sql.selected_fields().is_none(), "another row");
        sql.selection = Some(CellPos { row: 1, col: 2 });
        assert!(sql.selected_fields().is_some(), "another cell of the row");
        // The same row of the next result is not the row that was formatted.
        run_script(&mut sql, "SELECT 2", vec![rows_outcome(3)]);
        assert!(sql.selected_fields().is_none());
    }

    #[test]
    fn the_row_panel_shows_a_tables_data_view_or_a_selected_result_row() {
        let mut workspace = crate::testing::workspace();
        workspace.tabs.push(object_tab(1, "users", true));
        workspace.tabs.push(sql_tab(2));
        assert_eq!(workspace.row_panel_tab(), None, "no tab is active");
        workspace.active_tab = Some(TabId(1));
        assert_eq!(workspace.row_panel_tab(), Some(TabId(1)));
        workspace.object_tab_mut(TabId(1)).unwrap().view = ObjectView::Structure;
        assert_eq!(workspace.row_panel_tab(), None, "the Structure view");
        workspace.active_tab = Some(TabId(2));
        assert_eq!(workspace.row_panel_tab(), None, "nothing ran");
        let sql = workspace.sql_tab_mut(TabId(2)).unwrap();
        run_script(sql, "SELECT 1", vec![rows_outcome(3)]);
        assert_eq!(workspace.row_panel_tab(), None, "no row is selected");
        let sql = workspace.sql_tab_mut(TabId(2)).unwrap();
        sql.selection = Some(CellPos { row: 0, col: 0 });
        assert_eq!(workspace.row_panel_tab(), Some(TabId(2)));
        workspace.row_panel = false;
        assert_eq!(workspace.row_panel_tab(), None, "the panel is closed");
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- selected_row row_text the_row_panel_shows`
Expected: does not compile: no method `selected_row`, `selected_fields`, `row_panel_tab`, no field `fields` on `SqlTab`.

- [ ] **Step 3: Implement**

In `pub struct SqlTab`, after `pub selection: Option<CellPos>,`:

```rust
    /// The row panel's text for the selected row (see `App::format_rows`).
    pub fields: Option<RowFields>,
```

In `SqlTab::new`, after `selection: None,`:

```rust
            fields: None,
```

(The hand-written `Debug` for `SqlTab` ends in `finish_non_exhaustive()`; leave it as it is.)

In `impl SqlTab`, after `dims`:

```rust
    /// The selected row of the result, while the Results pane shows it:
    /// the row the row panel is for. None under the Messages pane, which
    /// shows no rows.
    pub fn selected_row(&self) -> Option<usize> {
        let row = self.selection?.row;
        (self.pane == ResultPane::Results && row < self.dims().0).then_some(row)
    }

    /// The row panel's text for the selected row, if it is up to date.
    pub fn selected_fields(&self) -> Option<&RowFields> {
        let row = self.selected_row()?;
        self.fields
            .as_ref()
            .filter(|fields| fields.row == row && fields.request == self.run.loaded)
    }
```

In `impl Workspace`, after `active_sql_tab`:

```rust
    /// The tab whose row the row panel shows: none while the panel is
    /// closed. A table's Data view keeps the panel open with no row
    /// selected; a SQL editor gives it room only while a row of its result
    /// is.
    pub fn row_panel_tab(&self) -> Option<TabId> {
        if !self.row_panel {
            return None;
        }
        match self.tab(self.active_tab?)? {
            Tab::Object(object) => (object.view == ObjectView::Data).then_some(object.id),
            Tab::Sql(sql) => sql.selected_row().map(|_| sql.id),
        }
    }
```

- [ ] **Step 4: Run them to see them pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- selected_row row_text the_row_panel_shows`
Expected: 3 passed.

- [ ] **Step 5: Commit point**

Stop and report. If asked to commit: `git add src/model.rs`, subject "Name the tab whose row the row panel shows".

---

## Task 2: The row panel reads its row from a source, not an object tab

A refactor with no behaviour change yet: nothing calls the panel for a SQL tab until Task 3. The existing row panel tests are the safety net.

**Files:**
- Modify: `src/ui/row_panel.rs:13` (imports), `:87-585` (`show`)

- [ ] **Step 1: Run the existing row panel tests, to know they are green**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- row_panel a_name_two_schemas_share`
Expected: all pass (10 or so).

- [ ] **Step 2: Add `Source` and `source()`**

Change the import at line 13 to:

```rust
use crate::model::{Action, CellPos, ConnTabId, RowFields, Tab, TabId, Workspace};
```

Directly above `pub fn show`, add:

```rust
/// What the panel shows a row of: a table's page, or a SQL editor's result.
struct Source<'a> {
    /// The table's name, or "Query 3": under the title in the macOS header.
    name: String,
    columns: &'a [tabletist_db::ColumnMeta],
    rows: &'a [Vec<Value>],
    selection: Option<CellPos>,
    /// The table's structure, once described. A result has none: no key
    /// names its rows, no foreign key links them, and only its booleans
    /// are tags.
    structure: Option<&'a tabletist_db::Structure>,
    /// The rows before the first one here: a table's page offset.
    offset: u64,
    /// The selected row's text, formatted by the app.
    texts: Option<&'a RowFields>,
    /// Whether the row is a table's. A result's row has no editing
    /// controls under it, and no `y` key to copy from it.
    table: bool,
}

/// The source the tab `id` gives the panel: none for a closed tab, or a
/// SQL editor that shows no rows.
fn source(workspace: &Workspace, id: TabId, locale: crate::i18n::Locale) -> Option<Source<'_>> {
    match workspace.tab(id)? {
        Tab::Object(object) => {
            let (columns, rows) = match object.page() {
                Some(page) => (page.columns.as_slice(), page.rows.as_slice()),
                None => (&[][..], &[][..]),
            };
            Some(Source {
                name: format::object_title(
                    &object.object,
                    workspace.name_is_shared(&object.object),
                ),
                columns,
                rows,
                selection: object.selection,
                structure: object.structure.value.as_ref(),
                offset: object.query.offset,
                texts: object.selected_fields(),
                table: true,
            })
        }
        Tab::Sql(sql) => {
            let (columns, rows, _) = sql.shown_rows()?;
            Some(Source {
                name: format!("{} {}", gettext(locale, "Query"), sql.number),
                columns,
                rows,
                selection: sql.selection,
                structure: None,
                offset: 0,
                texts: sql.selected_fields(),
                table: false,
            })
        }
    }
}
```

- [ ] **Step 3: Make `show` read the source**

Each change is inside `pub fn show`. The parameter `object_tab` becomes `id` throughout that function (the function `field` below it keeps its own `object_tab` parameter: do not touch `field`).

Signature and doc:

```rust
/// Draws the panel for the tab `id`: an object tab, or a SQL editor.
pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
```

The fold check: `workspace.fold_documents == Some(object_tab)` becomes `== Some(id)`.

Replace

```rust
    let Some(object) = workspace.object_tab(object_tab) else {
        return;
    };
    let object_name =
        format::object_title(&object.object, workspace.name_is_shared(&object.object));
```

with

```rust
    let Some(source) = source(workspace, id, locale) else {
        return;
    };
```

Replace

```rust
            let selected = object
                .selection
                .and_then(|cell| object.page().map(|page| (cell, page)))
                .and_then(|(cell, page)| page.rows.get(cell.row).map(|row| (cell, page, row)));
            // Formatted by the app when the selection changed, never here.
            let texts = object.selected_fields();
            let Some((cell, page, row)) = selected else {
```

with

```rust
            let selected = source
                .selection
                .and_then(|cell| source.rows.get(cell.row).map(|row| (cell, row)));
            // Formatted by the app when the selection changed, never here.
            let texts = source.texts;
            let Some((cell, row)) = selected else {
```

Then, further down the same closure:

| Old | New |
|---|---|
| `let structure = object.structure.value.as_ref();` | `let structure = source.structure;` |
| `let tags = crate::ui::value_tags::Tags::of_page(page, structure);` | see below |
| `page.columns` (in `key_value`) | `source.columns` |
| `let number = object.query.offset + cell.row as u64 + 1;` | `let number = source.offset + cell.row as u64 + 1;` |
| `let rows = page.rows.len();` | `let rows = source.rows.len();` |
| `id: object_tab,` (three `Action::MoveSelection`s: the terminal's step buttons, Next row, Previous row) | `id,` |
| `Text::one(&look, sub_role, &object_name, palette.dim)` | `Text::one(&look, sub_role, &source.name, palette.dim)` |
| `let fields: Vec<(usize, &tabletist_db::ColumnMeta, &Value)> = page` (then `.columns`) | `... = source` (then `.columns`) |
| `object_tab,` as the third argument of the three `field(ui, tab, object_tab, cell.row, ...)` calls | `id,` |

The tags:

```rust
            let tags: Vec<_> = source
                .columns
                .iter()
                .map(|column| crate::ui::value_tags::Tags::of(column, structure))
                .collect();
```

(`Tags::of_page` stays: `data_view.rs` uses it.)

The footer. Replace

```rust
            // Footer: the editing controls, disabled until editing arrives.
            // Footer: its rule, the buttons (28 or 32), the note under them.
            let note = line_of(ui, caption(&look), &look);
            let footer_height = if look.terminal {
```

with

```rust
            // Footer: the editing controls, disabled until editing arrives.
            // Its rule, the buttons (28 or 32), the note under them. A SQL
            // result has none: its rows are no table's to edit.
            let note = line_of(ui, caption(&look), &look);
            let footer_height = if !source.table {
                0.0
            } else if look.terminal {
```

and `editing_footer(ui, foot, &look, &palette, locale);` with

```rust
            if source.table {
                editing_footer(ui, foot, &look, &palette, locale);
            }
```

(Under decision 5B, leave both of these footer pieces as they were.)

After this, `grep -n "object\.\|page\.\|object_tab" src/ui/row_panel.rs` shows `object.` and `page.` only inside `source()`, and `object_tab` only inside `fn field`.

- [ ] **Step 4: No `y copy` hint beside a result's document**

Omarchy paints "za fold · y copy" beside a JSON field. `y` copies the selected cell of a table's grid and does nothing on a result, so a result's panel offers only the fold. In `struct FieldSkin`, after `texts`:

```rust
    /// Whether `y` copies the selected cell: in a table, not in a result.
    copy_key: bool,
```

In `show`, each of the three `FieldSkin { look: &look, palette: &palette, locale, fold, texts, }` literals gets one more field, after `texts,`:

```rust
                                                    copy_key: source.table,
```

In `fn field`, the destructuring `let FieldSkin { look, palette, locale, fold, texts } = skin;` gains `copy_key` after `texts`, and the hint

```rust
            // "za fold · y copy", keys in the text colour, at the right.
            let role = TextRole::OCaption;
            let hints = Text::new(look)
                .add(role, "za", palette.text)
                .space(role, " ")
                .add(role, "fold ·", palette.dim)
                .space(role, " ")
                .add(role, "y", palette.text)
                .space(role, " ")
                .add(role, "copy", palette.dim);
```

becomes

```rust
            // "za fold · y copy", keys in the text colour, at the right. A
            // SQL result has no `y`: the hint offers only the fold.
            let role = TextRole::OCaption;
            let hints = Text::new(look)
                .add(role, "za", palette.text)
                .space(role, " ");
            let hints = if copy_key {
                hints
                    .add(role, "fold ·", palette.dim)
                    .space(role, " ")
                    .add(role, "y", palette.text)
                    .space(role, " ")
                    .add(role, "copy", palette.dim)
            } else {
                hints.add(role, "fold", palette.dim)
            };
```

(Its test comes with Task 3, when a result's panel can be drawn at all.)

- [ ] **Step 5: Run the tests and clippy**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib -- row_panel a_name_two_schemas_share && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: the same tests pass as in Step 1; clippy is clean.

- [ ] **Step 6: Commit point**

Stop and report. If asked to commit: `git add src/ui/row_panel.rs`, subject "Let the row panel read a row from either kind of tab".

---

## Task 3: A selected result row opens the row panel

**Files:**
- Modify: `src/ui/workspace.rs:66-84` (`show`)
- Modify: `src/app.rs:235-263` (`format_rows`)
- Test: `src/ui/mod.rs`, `mod tests` (helpers at `:868-890`; new tests after `the_terminal_strip_has_no_row_panel_toggle_on_a_sql_tab`, `:1986-2000`)

- [ ] **Step 1: Test helpers**

In `mod tests` of `src/ui/mod.rs`, replace `with_sql_result` (`:868-890`) with these three (same behaviour for its callers; the new tests need a custom outcome and a run left in flight):

```rust
    /// Opens a SQL editor in `tab`, runs `SELECT 1` in it and answers with
    /// `rows` rows.
    fn with_sql_result(
        harness: &mut Harness,
        tab: crate::model::ConnTabId,
        rows: usize,
    ) -> crate::model::TabId {
        with_sql_outcome(harness, tab, crate::testing::rows_outcome(rows))
    }

    /// Opens a SQL editor in `tab`, runs `SELECT 1` in it and answers with
    /// `outcome`.
    fn with_sql_outcome(
        harness: &mut Harness,
        tab: crate::model::ConnTabId,
        outcome: tabletist_db::StatementOutcome,
    ) -> crate::model::TabId {
        harness.app.apply(crate::model::Action::NewSqlTab(tab));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        set_sql(harness, tab, "SELECT 1", 0);
        run_sql(harness, tab, id);
        harness.answer_sql(Ok(crate::testing::script_outcome(vec![outcome])), None);
        id
    }

    /// Starts a run of the SQL editor `id` and leaves it in flight.
    fn run_sql(harness: &mut Harness, tab: crate::model::ConnTabId, id: crate::model::TabId) {
        harness.app.apply(crate::model::Action::RunSql {
            tab,
            sql_tab: id,
            all: false,
        });
    }
```

- [ ] **Step 2: Write the failing tests**

After the test `the_terminal_strip_has_no_row_panel_toggle_on_a_sql_tab`. `rows_outcome(n)` is shaped like the fixture's users table: columns `id`, `email`, `meta`; row 1's `meta` is `{"plan":"pro"}`, the others' is NULL.

```rust
    /// The fields of the row panel that shows, by their copy buttons.
    fn panel_shows(harness: &mut Harness) -> bool {
        harness.has("Copy email")
    }

    #[test]
    fn selecting_a_result_row_opens_the_row_panel() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            let id = with_sql_result(&mut harness, tab, 3);
            // Until a row is selected the editor and the results keep the
            // whole width: no panel, and no placeholder for one.
            assert!(!panel_shows(&mut harness), "{}", look.name);
            assert!(!harness.has("Close the row panel"), "{}", look.name);
            assert!(!harness.has("Select a row to see its fields"));
            // The band under the editor is as wide as the editor.
            let wide = band(&mut harness).width();
            harness.click("Row 2");
            assert_eq!(
                sql_selection(&harness, tab, id),
                Some(crate::model::CellPos { row: 1, col: 0 })
            );
            for label in [
                "Close the row panel",
                "id · INTEGER",
                "email · TEXT",
                "Copy email",
            ] {
                assert!(harness.has(label), "{label} in {}", look.name);
            }
            // The panel stands beside the editor as well as the results.
            assert!(band(&mut harness).width() < wide, "{}", look.name);
            harness.click("Copy email");
            assert_eq!(harness.copied.as_deref(), Some("user2@example.com"));
        }
    }

    #[test]
    fn a_result_rows_panel_is_titled_by_its_number_and_its_query() {
        for look in [crate::theme::Look::standard(), crate::theme::Look::macos()] {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            with_sql_result(&mut harness, tab, 3);
            let named = |harness: &Harness| {
                harness
                    .painted
                    .iter()
                    .filter(|(text, _)| text == "Query 1")
                    .count()
            };
            harness.settle();
            let before = named(&harness);
            harness.click("Row 3");
            let tree = harness.settle();
            // A result has no key to name its row by: its number does.
            assert!(
                crate::testing::node(&tree, "Row 3", egui::accesskit::Role::Label).is_some(),
                "{}: {:?}",
                look.name,
                crate::testing::labels(&tree)
            );
            assert_eq!(named(&harness), before + 1, "under it, the query's name");
        }
    }

    #[test]
    fn a_result_rows_panel_has_no_editing_controls() {
        for look in crate::theme::Look::ALL {
            let controls = if look.terminal {
                "yy p duplicate"
            } else {
                "Duplicate"
            };
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = with_page(&mut harness);
            harness.click("Row 1");
            assert!(harness.has(controls), "a table's row in {}", look.name);
            with_sql_result(&mut harness, tab, 3);
            harness.click("Row 1");
            assert!(panel_shows(&mut harness), "{}", look.name);
            assert!(!harness.has(controls), "a result's row in {}", look.name);
        }
    }

    #[test]
    fn the_terminal_panel_offers_a_result_row_no_copy_key() {
        let hints = |harness: &Harness| -> Vec<String> {
            let painted = harness.painted.iter();
            painted
                .map(|(text, _)| text.clone())
                .filter(|text| text.starts_with("za fold"))
                .collect()
        };
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        // The first row's `meta` is a document, with its keys beside it.
        let tab = with_page(&mut harness);
        harness.click("Row 1");
        harness.settle();
        assert_eq!(hints(&harness), ["za fold · y copy"]);
        // `y` copies from a table's grid only: a result's row does not
        // offer it.
        with_sql_result(&mut harness, tab, 3);
        harness.click("Row 1");
        harness.settle();
        assert_eq!(hints(&harness), ["za fold"]);
    }

    #[test]
    fn the_panels_buttons_step_through_a_results_rows_and_close_it() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            let id = with_sql_result(&mut harness, tab, 3);
            let at = |row, col| Some(crate::model::CellPos { row, col });
            harness.click("Row 2");
            harness.click("Next row");
            assert_eq!(sql_selection(&harness, tab, id), at(2, 0), "{}", look.name);
            harness.click("Previous row");
            harness.click("Previous row");
            assert_eq!(sql_selection(&harness, tab, id), at(0, 0), "{}", look.name);
            harness.click("Close the row panel");
            assert!(!harness.app.workspace(tab).unwrap().row_panel);
            assert!(!panel_shows(&mut harness), "{}", look.name);
            // Closed, it stays closed for the next row, as a table's does.
            harness.click("Row 3");
            assert_eq!(sql_selection(&harness, tab, id), at(2, 0));
            assert!(!panel_shows(&mut harness), "{}", look.name);
        }
    }

    #[test]
    fn the_messages_pane_hides_a_result_rows_panel() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            let id = with_sql_result(&mut harness, tab, 3);
            harness.click("Row 2");
            assert!(panel_shows(&mut harness), "{}", look.name);
            harness.click("Messages");
            assert!(!panel_shows(&mut harness), "{}", look.name);
            assert!(!harness.has("Close the row panel"), "{}", look.name);
            // The row stays selected, so the panel is back with its grid.
            harness.click("Results");
            assert_eq!(
                sql_selection(&harness, tab, id),
                Some(crate::model::CellPos { row: 1, col: 0 })
            );
            assert!(panel_shows(&mut harness), "{}", look.name);
        }
    }

    #[test]
    fn a_new_result_closes_the_row_panel_until_a_row_of_it_is_selected() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 3);
        harness.click("Row 2");
        assert!(panel_shows(&mut harness));
        // The run in flight leaves the last result, and its row, in place.
        run_sql(&mut harness, tab, id);
        assert!(panel_shows(&mut harness));
        harness.answer_sql(
            Ok(crate::testing::script_outcome(vec![
                crate::testing::rows_outcome(5),
            ])),
            None,
        );
        assert_eq!(sql_selection(&harness, tab, id), None);
        assert!(!panel_shows(&mut harness));
        assert!(harness.app.workspace(tab).unwrap().row_panel, "still open");
        harness.click("Row 4");
        assert!(panel_shows(&mut harness));
        // A run that failed as a whole leaves no rows, and so no panel.
        run_sql(&mut harness, tab, id);
        let lost = tabletist_db::Error::ConnectionLost("the server went away".into());
        harness.answer_sql(Err(lost), None);
        assert!(!panel_shows(&mut harness));
    }

    #[test]
    fn a_table_tab_keeps_its_row_panel_beside_a_sql_tab() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        with_sql_result(&mut harness, tab, 3);
        assert!(!harness.has("Select a row to see its fields"));
        harness.click("users tab");
        assert!(harness.has("Select a row to see its fields"));
        harness.click("Structure");
        assert!(!harness.has("Select a row to see its fields"));
    }

    #[test]
    fn a_result_rows_text_is_formatted_once_not_every_frame() {
        use crate::ui::format::FULL_TEXTS;
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let mut page = crate::testing::page(2, false);
        page.rows[0][1] = tabletist_db::Value::Text("x".repeat(1_000_000).into());
        let columns = page.columns.len();
        let outcome = tabletist_db::StatementOutcome::Rows {
            columns: page.columns,
            rows: page.rows,
            truncated: false,
        };
        let id = with_sql_outcome(&mut harness, tab, outcome);
        harness.settle();
        let idle = FULL_TEXTS.with(std::cell::Cell::get);
        harness.click("Row 1");
        harness.settle();
        let before = FULL_TEXTS.with(std::cell::Cell::get);
        assert_eq!(before - idle, columns, "the selected row, once");
        for step in 0..5 {
            let at = egui::pos2(900.0 + step as f32 * 10.0, 300.0);
            harness.frame(vec![egui::Event::PointerMoved(at)]);
        }
        assert_eq!(FULL_TEXTS.with(std::cell::Cell::get), before);
        // Another row is formatted, once.
        harness.click("Row 2");
        let after = FULL_TEXTS.with(std::cell::Cell::get);
        assert_eq!(after - before, columns);
        harness.settle();
        assert_eq!(FULL_TEXTS.with(std::cell::Cell::get), after);
        // Nothing keeps the text of a row no panel shows.
        let fields = |harness: &Harness| {
            let workspace = harness.app.workspace(tab).unwrap();
            workspace.sql_tab(id).unwrap().fields.is_some()
        };
        assert!(fields(&harness));
        harness.click("Close the row panel");
        assert!(!fields(&harness));
    }
```

Notes on what these lean on:
- `harness.click(label)` clicks the button with that label first, so `click("Row 2")` is the grid's row even when the macOS panel's title is also "Row 2" (a label, not a button).
- `band(&mut harness)` is the existing helper (further down the same module) that finds the splitter under the editor. Its width before and after is a behaviour check for decision 2 (the panel is beside the editor too), not a size from the design.
- The title test is for the looks whose header has a title line; the terminal header paints "row" and the number without announcing them.

- [ ] **Step 3: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- result_row a_new_result the_messages_pane the_panels_buttons the_terminal_panel a_table_tab_keeps`
Expected: 8 FAILED (no panel appears: `Copy email` is missing, `nothing labelled "Next row"`, and so on). `a_table_tab_keeps_its_row_panel_beside_a_sql_tab` passes already: it guards the table side.

- [ ] **Step 4: Draw the panel for either kind of tab**

In `src/ui/workspace.rs`, `show`. Replace

```rust
    let row_panel = workspace.row_panel;
    super::sidebar::show(app, ui, tab);
    if look.terminal
        && let (Some(object_tab), Some(ObjectView::Data), true) = (active, view, row_panel)
    {
        super::row_panel::show(app, ui, tab, object_tab);
    }
```

with

```rust
    // The tab the row panel shows a row of: a table's Data view, or a SQL
    // editor with a row of its result selected.
    let row_panel = workspace.row_panel_tab();
    super::sidebar::show(app, ui, tab);
    if look.terminal
        && let Some(shown) = row_panel
    {
        super::row_panel::show(app, ui, tab, shown);
    }
```

and, inside the central panel,

```rust
                if let (Some(object_tab), Some(ObjectView::Data), true) = (active, view, row_panel)
                {
                    super::row_panel::show(app, ui, tab, object_tab);
                }
```

with

```rust
                if let Some(shown) = row_panel {
                    super::row_panel::show(app, ui, tab, shown);
                }
```

`active`, `view` and the `ObjectView` import are still used further down.

- [ ] **Step 5: Format a result row once**

In `src/app.rs`, `format_rows`. Replace its doc comment and its head, down to the object loop's `let row = ...;`:

```rust
    /// Formats the row each open row panel shows, once per selection or
    /// page (or SQL result), so drawing never reads a whole (possibly huge)
    /// value.
    fn format_rows(&mut self) {
        for tab in &mut self.tabs {
            let ConnTabContent::Workspace(workspace) = &mut tab.content else {
                continue;
            };
            let shown = workspace.row_panel_tab();
            for sql in workspace.sql_tabs_mut() {
                let row = sql.selected_row().filter(|_| shown == Some(sql.id));
                let Some(row) = row else {
                    // Nothing shows it: free the text.
                    sql.fields = None;
                    continue;
                };
                if sql.selected_fields().is_some() {
                    continue;
                }
                let fields =
                    sql.shown_rows()
                        .and_then(|(_, rows, _)| rows.get(row))
                        .map(|values| crate::model::RowFields {
                            request: sql.run.loaded,
                            row,
                            fields: values.iter().map(crate::ui::format::field_text).collect(),
                        });
                sql.fields = fields;
            }
            for object in workspace.object_tabs_mut() {
                let row = object
                    .selection
                    .filter(|_| shown == Some(object.id))
                    .map(|cell| cell.row);
```

The rest of the object loop is unchanged. (`shown` replaces `(open, active)`: for an object tab it also means the Data view, so the Structure view now frees the text, and coming back formats it again.)

- [ ] **Step 6: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib -- result_row row_panel a_new_result the_messages_pane the_panels_buttons the_terminal_panel a_table_tab_keeps a_name_two_schemas_share`
Expected: all pass, the 9 new ones and the existing row panel tests. (`the_terminal_strip_has_no_row_panel_toggle_on_a_sql_tab` still passes: Task 5 changes it.)

- [ ] **Step 7: Commit point**

Stop and report. If asked to commit: `git add src/ui/workspace.rs src/app.rs src/ui/mod.rs`, subject "Open the row panel on a selected SQL result row".

---

## Task 4: The row panel's keys on a SQL result

**Files:**
- Modify: `src/ui/keys.rs:78-92` and `:125-133` and `:257-260` (`handle`), `:416-421` and `:464-513` (`letters`)
- Test: `src/ui/mod.rs`, `mod tests`

- [ ] **Step 1: Write the failing tests**

After the tests of Task 3:

```rust
    #[test]
    fn space_and_ctrl_shift_r_toggle_a_result_rows_panel() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 3);
        let open = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
        // Out of the editor, where Space would be typed, and onto a row.
        harness.press(Key::Escape, Modifiers::NONE);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert!(panel_shows(&mut harness));
        harness.press(Key::Space, Modifiers::NONE);
        assert!(!open(&harness));
        assert!(!panel_shows(&mut harness));
        harness.press(Key::R, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(open(&harness));
        assert!(panel_shows(&mut harness));
        // Mod+Shift+R works from the editor too; Space is typed there.
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.sql_tab_mut(id).unwrap().focus_editor = true;
        harness.settle();
        assert!(harness.ctx.text_edit_focused());
        harness.press(Key::R, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(!open(&harness));
        harness.press(Key::Space, Modifiers::NONE);
        assert!(!open(&harness));
        // Under the messages no row shows, and the keys leave the panel be.
        harness.click("Messages");
        harness.press(Key::R, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(!open(&harness));
    }

    /// A key that `keys::letters` reads as the character it types.
    fn type_key(harness: &mut Harness, key: Key, text: &str) {
        harness.settle();
        harness.frame(vec![
            crate::testing::key(key, Modifiers::NONE),
            egui::Event::Text(text.into()),
        ]);
        harness.frame(vec![crate::testing::release(key, Modifiers::NONE)]);
        harness.settle();
    }

    #[test]
    fn the_terminal_keys_of_the_row_panel_work_on_a_result_row() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 3);
        let open = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
        let at = |row, col| Some(crate::model::CellPos { row, col });
        harness.press(Key::Escape, Modifiers::NONE);
        // With no row selected the panel's keys have nothing to show.
        for key in [Key::I, Key::Enter, Key::Escape] {
            harness.press(key, Modifiers::NONE);
            assert!(open(&harness), "{key:?} with no row");
        }
        // `]` and `[` step through the rows as `j` and `k` do.
        type_key(&mut harness, Key::CloseBracket, "]");
        assert_eq!(sql_selection(&harness, tab, id), at(0, 0));
        assert!(panel_shows(&mut harness));
        type_key(&mut harness, Key::CloseBracket, "]");
        assert_eq!(sql_selection(&harness, tab, id), at(1, 0));
        type_key(&mut harness, Key::OpenBracket, "[");
        assert_eq!(sql_selection(&harness, tab, id), at(0, 0));
        // Esc closes the panel, Enter opens it, `i` does either.
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!open(&harness));
        assert!(!panel_shows(&mut harness));
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!open(&harness), "Esc only closes");
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(open(&harness));
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(open(&harness), "Enter only opens");
        harness.press(Key::I, Modifiers::NONE);
        assert!(!open(&harness));
        harness.press(Key::I, Modifiers::NONE);
        assert!(open(&harness));
        assert!(panel_shows(&mut harness));
        // `za` folds the row's documents: the first row's `meta`.
        assert!(harness.has(r#""plan": "pro""#));
        harness.press(Key::Z, Modifiers::NONE);
        harness.press(Key::A, Modifiers::NONE);
        assert!(harness.has("{ 1 key }"));
        assert!(!harness.has(r#""plan": "pro""#));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- space_and_ctrl_shift_r the_terminal_keys`
Expected: the two new tests FAIL (Space leaves the panel open; `]` selects nothing). The existing `space_and_ctrl_shift_r_toggle_the_row_panel` passes.

- [ ] **Step 3: Space and Mod+Shift+R (`handle`)**

After the `sql_grid` binding (`:79-83`), add:

```rust
    // A SQL editor's row panel is its selected row's: with none selected
    // the panel's keys have nothing to show or hide.
    let sql_row = sql.is_some_and(|(tab, id)| {
        app.workspace(tab)
            .and_then(|workspace| workspace.sql_tab(id))
            .is_some_and(|sql| sql.selected_row().is_some())
    });
```

Replace

```rust
        // A SQL editor has no row panel, and nothing to refresh or filter.
        let on_sql = sql.is_some();
        if !on_sql {
            key(
                Modifiers::COMMAND | Modifiers::SHIFT,
                Key::R,
```

with

```rust
        // A SQL editor has nothing to refresh or filter, and a row panel
        // only for a selected row of its result.
        let on_sql = sql.is_some();
        if !on_sql || sql_row {
            key(
                Modifiers::COMMAND | Modifiers::SHIFT,
                Key::R,
```

Replace

```rust
                // The row panel shows a table's row, not a SQL result's.
                if grid && !focused {
```

with

```rust
                // The row panel shows a table's row, or the selected row
                // of a SQL result.
                if (grid || sql_row) && !focused {
```

- [ ] **Step 4: The Omarchy letters (`letters`)**

After the `sql_grid` binding in `letters` (`:418-421`), add:

```rust
    // And, with a row of it selected, the letters of the row panel.
    let sql_row = workspace
        .active_sql_tab()
        .filter(|sql| sql.selected_row().is_some())
        .map(|sql| sql.id);
```

Then replace this block (`:464-489`)

```rust
    // The letters below act on an object tab: none of them on a SQL editor.
    let Some(object_tab) = active else {
        ctx.data_mut(|data| data.insert_temp(pending_id(), next_pending));
        return;
    };
    let step = |rows: isize, cols: isize| Action::MoveSelection {
        tab,
        id: object_tab,
        rows,
        cols,
    };
    if !tree && !panel && pressed(Key::Enter) {
        actions.push(Action::ToggleRowPanel(tab));
    }
    if typed(ctx, "[") {
        actions.push(step(-1, 0));
    }
    if typed(ctx, "]") {
        actions.push(step(1, 0));
    }
    if pressed(Key::I) {
        actions.push(Action::ToggleRowPanel(tab));
    }
    if panel && pressed(Key::Escape) {
        actions.push(Action::ToggleRowPanel(tab));
    }
```

with

```rust
    // `[` and `]` step through the rows `j/k` move in.
    if let Some(id) = active.or(sql_grid) {
        for (text, rows) in [("[", -1), ("]", 1)] {
            if typed(ctx, text) {
                actions.push(Action::MoveSelection {
                    tab,
                    id,
                    rows,
                    cols: 0,
                });
            }
        }
    }
    // The row panel's keys: an object tab's, or a SQL result's while a row
    // of it is selected (with none its panel has nothing to show).
    if let Some(id) = active.or(sql_row) {
        if !tree && !panel && pressed(Key::Enter) {
            actions.push(Action::ToggleRowPanel(tab));
        }
        if pressed(Key::I) {
            actions.push(Action::ToggleRowPanel(tab));
        }
        if panel && pressed(Key::Escape) {
            actions.push(Action::ToggleRowPanel(tab));
        }
        if pressed(Key::Z) {
            next_pending = Some('z');
        }
        if pressed(Key::A) && pending == Some('z') {
            actions.push(Action::FoldDocuments {
                tab,
                object_tab: id,
            });
        }
    }
    // The letters below act on an object tab: none of them on a SQL editor.
    let Some(object_tab) = active else {
        ctx.data_mut(|data| data.insert_temp(pending_id(), next_pending));
        return;
    };
```

and, further down, delete the object-only `za` that the block above replaces:

```rust
    if pressed(Key::Z) {
        next_pending = Some('z');
    }
    if pressed(Key::A) && pending == Some('z') {
        actions.push(Action::FoldDocuments { tab, object_tab });
    }
```

`panel` here is still `workspace.row_panel`: on a SQL tab with a selected row, that flag is exactly "the panel shows". An object tab's keys behave as before (`g`, `z` and `d` do not depend on each other's order).

- [ ] **Step 5: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib -- space_and_ctrl_shift_r the_terminal_ sql_tab sql_result`
Expected: all pass, including the existing `the_terminal_letters_for_an_object_do_nothing_on_a_sql_editor` and `the_row_panel_and_paging_keys_do_nothing_on_a_sql_tab` (both press the keys with no result row selected, which still does nothing).

- [ ] **Step 6: Rename the test whose name is no longer true**

`the_row_panel_and_paging_keys_do_nothing_on_a_sql_tab` (`src/ui/mod.rs:1550`) still passes, but only because it selects no row. Rename it and say so:

```rust
    fn the_paging_keys_do_nothing_on_a_sql_tab_nor_the_row_panels_with_no_row_selected() {
```

and its comment

```rust
        // With the keys out of the editor, where Space would be typed.
```

becomes

```rust
        // With the keys out of the editor, where Space would be typed. No
        // row of the result is selected, so its panel has nothing to show
        // and the keys leave it as it is.
```

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- the_paging_keys`
Expected: 1 passed.

- [ ] **Step 7: Commit point**

Stop and report. If asked to commit: `git add src/ui/keys.rs src/ui/mod.rs`, subject "Give a SQL result row the row panel's keys".

---

## Task 5: The Omarchy strip keeps its row panel toggle on a SQL tab

**Files:**
- Modify: `src/ui/object_tabs.rs:61-63`, `:178`
- Test: `src/ui/mod.rs:1986-2000`

- [ ] **Step 1: Turn the old test round**

Replace the whole test `the_terminal_strip_has_no_row_panel_toggle_on_a_sql_tab` with:

```rust
    #[test]
    fn the_terminal_strip_keeps_its_row_panel_toggle_on_a_sql_tab() {
        let (mut harness, tab) = sql_harness(crate::theme::Look::omarchy());
        let toggle = "Show or hide the row panel";
        let open = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
        assert!(harness.has(toggle), "a result has a row panel too");
        assert!(open(&harness));
        harness.click(toggle);
        assert!(!open(&harness));
        // The same toggle as a table tab's: one panel for the workspace.
        harness.click("users");
        harness.answer_rows(crate::testing::page(5, false));
        assert!(harness.has(toggle));
        assert!(!harness.has("Select a row to see its fields"));
        harness.click(toggle);
        assert!(open(&harness));
        harness.click("Query 1 tab");
        assert!(harness.has(toggle));
    }
```

This replaces a test that pinned the first slice's behaviour ("an editor has no row panel"), which this plan changes on purpose.

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- the_terminal_strip_keeps`
Expected: FAIL at "a result has a row panel too".

- [ ] **Step 3: Implement**

In `src/ui/object_tabs.rs`, delete

```rust
    // A SQL editor has no row panel to show or hide.
    let on_editor = workspace.active_sql_tab().is_some();
```

and replace `if look.terminal && !on_editor {` with

```rust
            // The row panel's toggle, on every tab: a SQL editor's result
            // has a row panel too.
            if look.terminal {
```

- [ ] **Step 4: Run it to see it pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib -- the_terminal_strip`
Expected: pass.

- [ ] **Step 5: Commit point**

Stop and report. If asked to commit: `git add src/ui/object_tabs.rs src/ui/mod.rs`, subject "Keep the row panel toggle on SQL tabs in the Omarchy strip".

---

## Task 6: Say what is built, and the full check

**Files:**
- Modify: `docs/superpowers/specs/2026-09-30-sql-editor-core-design.md`
- Modify: `docs/superpowers/specs/2026-09-27-tabletist-design.md:460-475` (5.7 Row panel)
- Modify: `README.md:49-52`

- [ ] **Step 1: The SQL editor spec**

It is written "as built", so it follows the code:

1. The opening paragraph (line 3-5): after "the text follows the code." add the sentence "Since 2026-10-01 the row panel shows a selected result row (see Results and Shortcuts on a SQL tab)."
2. `## Out of scope`: remove "the row panel for SQL results, " and rewrap the paragraph:

   ```
   Autocomplete, Explain, Format, History, Copy, Export, vim modes, saving or
   restoring query text, several result sets side by side, and running anything
   outside a read-only transaction.
   ```
3. `## Model`: in the `SqlTab` listing, after `pub selection: Option<CellPos>,` add

   ```rust
       pub fields: Option<RowFields>, // the row panel's text for the selected row
   ```

   and after the paragraph that ends "and no run is in flight." add:

   ```
   `selected_row` is the selected row while the Results pane shows it: the row
   the row panel is for. `fields` holds that row's text (`RowFields`, as on
   `ObjectTab`): `App::format_rows` fills it once per selection and frees it
   when no panel shows the row, and `selected_fields` gives it while it is
   still that row of that run. `Workspace::row_panel_tab` names the tab whose
   row the row panel shows: an object tab in its Data view, or a SQL tab with a
   selected row, and none while the panel is closed.
   ```
4. `### Opening`: replace the bullet "The row panel and its toggle do not apply to SQL tabs in this slice." with

   ```
   - The Omarchy strip's row panel toggle stays on a SQL tab: a result's row
     has a row panel too (see Results).
   ```
5. `### Shortcuts on a SQL tab`: replace the bullet that starts "`Mod+R`, `Mod+F`, `Mod+Alt+Left / Right`, Space and `Mod+Shift+R` do nothing" with

   ```
   - `Mod+R`, `Mod+F` and `Mod+Alt+Left / Right` do nothing on a SQL tab (no
     refresh, filter or paging here).
   - While a row of the result is selected, the row panel's keys work as in a
     table tab: Space (outside the editor) and `Mod+Shift+R` show or hide it,
     and on Omarchy `i` does the same, Enter opens it, Esc closes it and `za`
     folds its documents. With no row selected they do nothing: there is no
     panel to show or hide. Omarchy's `[` and `]` step through the result's
     rows whenever its grid shows.
   ```
6. `### Results`: after the bullet that starts "Results shows the existing grid", add

   ```
   - Selecting a row of the result opens the row panel on it: the same right
     panel as a table tab's (one width and one open or closed state per
     workspace), beside the editor and the results. It is there only while a
     row is selected and the Results pane shows: no placeholder takes the
     editor's width before that, the Messages pane hides it (the row stays
     selected), and a finished run clears the selection and so closes it. A
     result has no table behind it: the panel is titled by the row's number
     over "Query N", shows no key and no foreign key link, tags only booleans,
     and leaves out the editing controls and Omarchy's `y copy` hint. Closed
     with its button or a key, it stays closed until it is toggled back.
   ```
7. `## Testing`, the "Headless UI tests" bullet: before its final full stop add "; selecting a result row opens the row panel, the Messages pane and a new run close it, and its keys work on a selected result row".

Two of the anchors wrap across lines in the spec ("the text follows the code." and "and no run is in flight."): find them by their last words. Apply the wording for whichever decisions the user took; the text above is for the recommended ones.

- [ ] **Step 2: The main design spec and the README**

In `docs/superpowers/specs/2026-09-27-tabletist-design.md`, section 5.7, add a bullet at the end:

```
- On a SQL tab the panel shows the selected row of the result, and only while
  one is selected (see the SQL editor spec).
```

In `README.md`, the SQL editor bullet becomes:

```
- A SQL editor per connection (Cmd/Ctrl+T): run the statement at the cursor
  (Cmd/Ctrl+Return) or the whole script, with a row limit and a timeout, and
  read a result row in full in the row panel.
  Every run happens in a read-only transaction that is rolled back, and
  statements that would leave it are refused.
```

- [ ] **Step 3: No em dashes**

Run: `git diff 8a87e42 -U0 | LC_ALL=C.UTF-8 grep -nP "\x{2014}" ; LC_ALL=C.UTF-8 grep -nP "\x{2014}" docs/superpowers/plans/2026-10-01-sql-result-row-panel.md ; echo "exit $?"`
Expected: no matches and `exit 1` (U+2014 is the em dash). An exit of 2 means grep could not read the pattern in this locale and the check did not run.

- [ ] **Step 4: The full check**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: all clean; the library has 750 tests where `8a87e42` has 728: 22 more (3 model in Task 1; 9 UI in Task 3 and 4 from its review; 2 UI in Task 4 and 2 from its review; 2 with Task 5 and its review, and Task 5 also replaces one; Task 4 renames one). The database integration tests print "skipped" without the `TABLETIST_TEST_*` variables: say so when reporting.

- [ ] **Step 5: Look at it (by eye, nothing committed)**

Run the app on the demo database and try the flow in the look this machine draws (Omarchy on this Linux machine; the macOS look was only exercised by the headless tests, say so):

```bash
~/.cargo/bin/cargo run --release -- --demo
```

Open a SQL editor, run a `SELECT`, click a row: the panel opens beside the editor and the results; Messages hides it; a new run closes it; Space, `i`, Esc, `[`, `]` and `za` act on it. Screenshots, if any, stay local and use the demo data.

- [ ] **Step 6: Commit point**

Stop and report what was run and what was only compiled. If asked to commit: `git add docs/superpowers/specs README.md docs/superpowers/plans/2026-10-01-sql-result-row-panel.md`, subject "Describe the row panel for SQL results".

---

## Task 7: The panel takes at most half the room (added after the first six)

Asked for by the user once the feature was built and its screenshots showed the Omarchy editor left about 270 pt wide in a 1000 pt window: "Cap the panel width so the editor keeps room". Unlike Tasks 1 to 6, this task's code was not drafted and run before it was written down: treat the snippets as a starting point and the tests as the requirement.

**The rule (one rule, both kinds of tab):** the row panel is never wider than half the width beside the sidebar, so the grid or the editor always keeps at least as much as the panel takes. With room to spare nothing changes (260 to 560 pt, its default 345 or 462). In a narrow window the half goes under the panel's usual least width and the panel gives way too. The width the user dragged it to (or its default) is remembered apart from the width a small window cut it to, and comes back when there is room again.

**Why it needs its own memory:** egui's panel remembers the width it last drew, clamped to the range it was given. Given a smaller range in a small window it would remember the small width and stay small in a big window.

**Files:**
- Modify: `src/ui/row_panel.rs` (the `egui::Panel::right(...)` call in `show`; a `width_range` function; a small `mod tests`)
- Test: `src/ui/mod.rs`, `mod tests`
- Modify: the two specs (the "Known limit" bullet of the SQL editor spec, and 5.7 of the main spec)

- [ ] **Step 1: Write the failing tests**

In `src/ui/row_panel.rs`, at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panel_takes_at_most_half_the_room() {
        let range = |room: f32| {
            let range = width_range(room);
            (range.min, range.max)
        };
        // Room to spare: the widths the edge can be dragged to.
        assert_eq!(range(1400.0), (260.0, 560.0));
        // Half the room is the most the panel takes.
        assert_eq!(range(700.0), (260.0, 350.0));
        // Under its least width the panel gives way too.
        assert_eq!(range(400.0), (200.0, 200.0));
        assert_eq!(range(0.0), (0.0, 0.0));
    }
}
```

In `mod tests` of `src/ui/mod.rs`, after the SQL row panel tests (`band` is the existing helper that finds the splitter under the SQL editor: it is as wide as the editor):

```rust
    #[test]
    fn the_row_panel_leaves_the_editor_half_the_room() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(egui::vec2(1000.0, 650.0));
            harness.set_look(look);
            let tab = harness.connect_fake();
            with_sql_result(&mut harness, tab, 3);
            let room = band(&mut harness).width();
            harness.click("Row 2");
            assert!(panel_shows(&mut harness), "{}", look.name);
            let editor = band(&mut harness).width();
            assert!(editor < room, "{}", look.name);
            // A point of slack for rounding to the pixel.
            assert!(
                editor >= room / 2.0 - 1.0,
                "{editor} of {room} in {}",
                look.name
            );
        }
    }

    #[test]
    fn the_row_panel_gets_its_width_back_when_the_window_grows() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = harness.connect_fake();
        with_sql_result(&mut harness, tab, 3);
        let room = band(&mut harness).width();
        harness.click("Row 2");
        let wide = band(&mut harness).width();
        // A narrower window: the panel gives way to the editor.
        let narrower = 380.0;
        harness.size.x -= narrower;
        let editor = band(&mut harness).width();
        assert!(editor >= (room - narrower) / 2.0 - 1.0, "{editor}");
        // And back: the panel is as wide as it was.
        harness.size.x += narrower;
        harness.settle();
        assert!((band(&mut harness).width() - wide).abs() < 1.0);
    }

    #[test]
    fn a_tables_row_panel_gives_way_in_a_narrow_window_and_comes_back() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        focus_grid(&mut harness, tab);
        harness.click("Row 1");
        // A field's label is as wide as the panel lets it be.
        let label = |harness: &mut Harness| {
            let tree = harness.settle();
            crate::testing::bounds(&tree, "id · INTEGER", egui::accesskit::Role::Label)
                .expect("the id field")
                .width()
        };
        let wide = label(&mut harness);
        harness.size.x = 800.0;
        assert!(label(&mut harness) < wide - 1.0, "the panel gave way");
        harness.size.x = 1280.0;
        harness.settle();
        assert!((label(&mut harness) - wide).abs() < 1.0, "and came back");
    }
```

These assert relations (half the room, narrower, back to what it was), never a size from the design.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- the_panel_takes_at_most the_row_panel_leaves the_row_panel_gets_its_width a_tables_row_panel_gives_way`
Expected: does not compile (`width_range` is missing). With a stub `width_range` returning `egui::Rangef::new(260.0, 560.0)`: the unit test fails, `the_row_panel_leaves_the_editor_half_the_room` fails in the Omarchy look (the editor has about 273 of 735), and the two window tests fail at their narrow-window assertions.

- [ ] **Step 3: Implement**

In `src/ui/row_panel.rs`, near `fn width`:

```rust
/// The widths the panel's edge can be dragged to, with room to spare.
const NARROWEST: f32 = 260.0;
const WIDEST: f32 = 560.0;

/// The widths the panel may have when it shares `room` points with the
/// grid or the editor beside it: never more than half, so what it sits
/// beside keeps at least as much. In a narrow window that goes under the
/// panel's least width, and the panel gives way too.
fn width_range(room: f32) -> egui::Rangef {
    let most = (room / 2.0).clamp(0.0, WIDEST);
    egui::Rangef::new(NARROWEST.min(most), most)
}
```

In `show`, replace

```rust
    egui::Panel::right(Id::new(("row-panel", tab.0)))
        .resizable(true)
        .default_size(width(&look))
        .size_range(260.0..=560.0)
```

with

```rust
    let panel = Id::new(("row-panel", tab.0));
    let range = width_range(ui.available_width());
    // egui remembers the width it last drew, which a small window cuts
    // down. The width the user gave the panel (or its default) is kept
    // beside it, to come back to when there is room again.
    let drawn = egui::containers::panel::PanelState::load(ui.ctx(), panel)
        .map(|state| state.outer_rect.width());
    let wanted: f32 = ui.data_mut(|data| {
        *data.get_persisted_mut_or_insert_with(panel.with("wanted"), || {
            drawn.unwrap_or(width(&look))
        })
    });
    let target = wanted.clamp(range.min, range.max);
    // Only a drag makes egui's panel wider: with room again, its least
    // width is the target until it is drawn that wide.
    let least = if drawn.is_some_and(|drawn| drawn + 1.0 < target) {
        target
    } else {
        range.min
    };
    egui::Panel::right(panel)
        .resizable(true)
        .default_size(target)
        .size_range(least..=range.max)
```

and after the panel's `.show(ui, |ui| { ... });` (before `app.actions.extend(actions);`):

```rust
    // The edge was dragged: that is the width wanted from now on.
    let drawn = egui::containers::panel::PanelState::load(ui.ctx(), panel)
        .map(|state| state.outer_rect.width());
    if let Some(drawn) = drawn
        && (drawn - target).abs() > 1.0
    {
        ui.data_mut(|data| data.insert_persisted(panel.with("wanted"), drawn));
    }
```

Things to check while implementing, because this was not run:
- The path of `PanelState` in this egui fork (it is `pub struct PanelState { pub outer_rect: Rect }` with `pub fn load(ctx, id)` in `containers/panel.rs`; use the re-export if there is one), and that `load` takes what `ui.ctx()` gives.
- egui stores the drawn width only while the edge is not being dragged, so during a drag `drawn` stays the width before it and nothing is recorded until the drag ends. A drag must still be able to make the panel narrower and wider within the range: test it by hand in a probe, or add a drag test like `dragging_the_band_changes_the_editors_share` if the panel's resize edge can be reached in the harness.
- `ui.available_width()` at both call sites of `row_panel::show` (`workspace::show`: at workspace level in the terminal look, inside the central panel otherwise) is the width beside the sidebar.
- The view keeps this in egui's memory, as it keeps scroll offsets and folds: no app state changes.

- [ ] **Step 4: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: all pass (758 tests: the 754 before and these 4). The existing tests run in a 1280 pt window, where half the room is more than either default width, so none of them should change.

- [ ] **Step 5: Say what is built**

- SQL editor spec, Results: the "Known limit" bullet no longer holds. Replace its first two sentences (down to "A table tab has the same limit.") with: "The panel takes at most half the width beside the sidebar, on a SQL tab as on a table tab, so the editor and the results keep at least as much as the panel. In a narrow window that goes under the panel's usual least width. The width it was dragged to comes back when there is room again." Keep the sentences about the grid's columns and the status line, under a lead such as "Known limits:".
- Main design spec, 5.7, first bullet ("Resizable right panel ..."): add "It takes at most half the width beside the sidebar; the width it was dragged to comes back when the window has room again."

- [ ] **Step 6: The full check and a look**

Run the five check commands. Then render the screenshots (`~/.cargo/bin/cargo test --locked --features shots --lib shots::shots -- --ignored`) and look at `target/shots/sql-row-omarchy-dark.png` and `workspace-omarchy-dark.png` (1000 pt wide scenes): the editor or the grid is as wide as the panel.

- [ ] **Step 7: Commit point**

Stop and report. If asked to commit: subject "Give the row panel at most half the room".

---

## Changes made in review (as built)

Each task was reviewed after it was built (spec compliance, then code quality). Where a review changed the code, the code is right and the task text above is the first draft:

**Task 1**
- `RowFields` and `Workspace::row_panel_tab` say in their docs that they serve a SQL result too.
- Two more assertions: `selected_fields` is none under the Messages pane, and a closed panel is none for a table tab as well.

**Task 2**
- `source()` gives a SQL tab's cell only while its row shows (`sql.selected_row().and(sql.selection)`), so the selection and the row's text cannot disagree.
- `FieldSkin` is built once in `show`, and `fn field`'s tab parameter is `tab_id` (it gets SQL tab ids too).

**Task 3**
- Keyboard focus survives the panel appearing: the SQL editor's two panes have an explicit id (`("sql-pane", salt, tab, id)` in `sql_editor::show`). Counted from the parent, their ids moved when the panel was drawn before them, and a focused result row lost the focus (`opening_the_row_panel_keeps_the_keyboard_on_the_result_row`).
- Folds and "Show all" belong to a row of one page or one result: their egui ids carry the row's request. The same row of a new result starts fresh (`a_new_result_does_not_inherit_an_expanded_value`). This holds for a table too: another page or a refresh resets the folds and "Show all" of the selected row.
- `row_fields(request, row, values)` in `src/app.rs` builds the row's text for both loops of `format_rows`.
- Tests assert the value the panel shows, not only that it shows; a new result with other data; a table's row text after the Structure view (`a_tables_row_text_is_back_after_the_structure_view`); two SQL tabs each keeping their row, and closing one (`each_sql_tab_keeps_its_own_selected_row`).

**Task 4** (Omarchy keys, table tabs included)
- An Esc that leaves a text field does only that: the next Esc closes the panel. egui takes the keyboard from the field before the keys are read, so `keys::after_frame` notes after each frame whether a field had it (`escape_out_of_the_editor_leaves_the_row_panel_open`, `escape_out_of_the_where_line_leaves_the_row_panel_open`).
- Enter belongs to a focused button, as it does for the tree: it opens the panel only when no widget has the keyboard (`enter_on_a_focused_button_is_the_buttons_not_the_row_panels`).
- `za` folds only while the panel shows the tab's row, so no fold waits for a panel opened later.
- `Action::FoldDocuments { tab, id }`: the field is `id`, as for every action that works on either kind of tab.

**Task 5 and Task 6**
- One more strip test: the toggle hides and shows a selected result row's panel (`the_terminal_strips_toggle_hides_and_shows_a_result_rows_panel`).
- `src/shots.rs` has a `sql-row` scene for the local screenshots (a result row selected, its panel open).

**Final review of the whole change**
- A result may have two columns of one name, which a table cannot. Two such columns that both hold a JSON document shared a fold toggle id (it was made from the toggle's label). The toggle's id is now its node's (`json_view.rs`), so each document folds on its own (`two_result_columns_of_one_name_fold_apart`).
- Table-side tests for what this change touched on a table tab in Omarchy: the panel's keys (`the_terminal_keys_of_the_row_panel_work_on_a_table_row`), Enter with a focused button (`enter_on_a_focused_button_is_the_buttons_on_a_table_tab`), and a refreshed page starting with nothing expanded (`a_refreshed_page_does_not_inherit_an_expanded_value`).
- A side effect of that fix, kept and tested: a fold toggle pressed from the keyboard keeps the keyboard (its id no longer changes with its label), on a table tab too (`a_fold_toggle_keeps_the_keyboard_after_it_is_pressed`).

**Task 7** (built as written above)
- `egui::PanelState` through the re-export, and one helper `drawn_width(ui, id)` for the two reads of the drawn width.
- One more test: a width set by dragging the panel's edge is the one that comes back after a narrow window (`a_dragged_row_panel_width_is_the_one_that_comes_back`).
- This replaced the limit the reviews had found (the panel taking all the width beside the sidebar in a small window, for a table tab as well).
- From its review, because the panel can now be narrower than 260 pt:
  - No panic at a sliver of a width: the sizes the fields are laid out with never go negative (`a_sliver_of_a_row_panel_does_not_panic`).
  - Omarchy: the footer's note is cut with an ellipsis, a footer cell too narrow for its words shows its key alone, and the header's row name is cut before the `[ ] prev/next` hint (`the_terminal_footer_fits_a_narrow_row_panel`). A long key value is now cut in the header at any width, where it ran under the hint.
  - The wanted width is recorded only when a drag ends, so a window resized while the edge is held keeps it (`a_window_resized_while_the_edge_is_held_keeps_the_wanted_width`).
  - The edge offers no resize when the range has collapsed (half the room under the least width); `the_row_panel_gives_way_in_the_smallest_window`.
  - After a slow window resize the panel may be up to a point short of the wanted width: the slack that decides when to bring the width back is a whole point, so rounding can never pin the panel.
  - Omarchy: the header's hint gives way before the row's name does. When the name does not fit beside `[ ] prev/next`, the hint shows its keys alone and the name gets the room, so the smallest window still says which row shows (`the_terminal_header_names_its_row_in_the_smallest_window`).
- The library ends at 765 tests (728 at `8a87e42`).

**Known limits, not changed here**
- The result grid sizes its columns before the panel opens, so a JSON column may need a horizontal scroll once it does.
- The Omarchy status line on a SQL tab does not list the panel's keys.

---

## Review focus

1. **The user's request:** clicking (or moving onto) a result row opens the panel with that row's fields, in every look (`selecting_a_result_row_opens_the_row_panel`).
2. **Table tabs keep their panel:** the placeholder, the keys and the Structure view (`a_table_tab_keeps_its_row_panel_beside_a_sql_tab`, and every existing row panel test untouched and green). What did change for a table tab is listed under "Changes made in review" above: folds and "Show all" per page, and the Omarchy Esc, Enter and `za` rules.
3. **No stale row:** a new run, a run that fails as a whole and the Messages pane all take the panel away (`a_new_result_closes_...`, `the_messages_pane_hides_...`).
4. **No per-frame formatting of a huge value**, and no text kept for a row no panel shows (`a_result_rows_text_is_formatted_once_not_every_frame`).
5. **No silent flips:** with no row selected, the panel's keys leave `Workspace.row_panel` alone on a SQL tab (`the_paging_keys_do_nothing_..._with_no_row_selected`, `the_terminal_keys_of_the_row_panel_work_on_a_result_row`).
