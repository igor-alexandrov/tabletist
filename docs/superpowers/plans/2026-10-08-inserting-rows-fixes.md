# Inserting Rows, Fixes to What Run 2 Built Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A new row in the grid looks and reads as the spec and the boards have it: its own greens, green while it is selected, a default drawn as its tag at 60%, an expression default slanted, and on Omarchy a 2px bar, `+1` in the header and what the row needs said while it is typed.

**Architecture:** The new row's colours move out of the success tone into one function, `env::new_row_colors`, which the grid and the pending bar read. What an unset cell holds gains one case, `Unset::Expression`, so the grid can slant a default the database works out and tag one it holds. The terminal's header and its insert-mode line each gain one piece of text. No state changes, and no action.

**Tech Stack:** Rust 1.98, egui/eframe (the crmne forks). Spec: `docs/superpowers/specs/2026-10-07-inserting-rows-design.md` (sections 2, 3 and 6; decisions A5 to A8 and D3). Findings: INS-06a, INS-06c, INS-06d, INS-07b, INS-08a, INS-08b, INS-10b of the audit in pull request 108.

---

## Before you start

- Cargo is `~/.cargo/bin/cargo` (the mise shim fails). Never point `CARGO_TARGET_DIR` at `/tmp`.
- The four checks, from `AGENTS.md`. "Run the four checks" below means these, and all four pass after every task:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

- **What was run where this plan was written: nothing of it.** It was written from reading the tree at `63df88f` and the boards of "Editing · Inserting rows" (canvas version `1791497417-82c0`, read on 2026-10-08). The code blocks are written against names that exist in that tree and were not compiled. Treat a block as what the code should come to, and expect to correct a name or a borrow on the first build. Where a block and the compiler disagree, the compiler is right and the task's tests say what must hold.
- **Read the boards again before task 1** (`MacInsert`, `MacInsertStates`, `OmarchyInsert`): the canvas moves. If a colour or a word below is no longer the board's, stop and say so.
- **Design material is never a test and never committed.** No test names a colour of the design: a test may hold that the grid paints what `env::new_row_colors` gives, as `src/ui/env_tests.rs` does for a connection's colours, and no more. The comparison with the boards is by eye, on throwaway scenes (task 7), and no screenshot goes to GitHub unblurred.
- **The branch.** A new one from `main` once pull request 108 is merged: task 4 takes a test of `src/ui/insert_audit_tests.rs` out of `ignore`, and that file is the pull request's. This plan is the branch's first commit.
- House rules that bite here: no em dashes; comments explain why, in the surrounding code's voice; a view pushes `Action`s and changes no state; views draw text only through `TextRole`s; a view never paints a focus ring; every behaviour change has a headless test; do not weaken a lint or delete a test to get green. Three existing tests change what they expect (two in task 1, one in task 3): each commit says why.
- `cargo test` takes one name to filter by before `--` and any number after it: the runs below that name two tests put both after `--`.
- Commits are signed, one per task, after its checks pass. If signing fails ("agent refused operation"), do not commit unsigned: stage the task and tell the user. Chain `git add` and `git commit` with `&&`. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## What the design asks, and what gets built

Where this plan does not do what the spec or a board says. **The user accepted every row on 2026-10-09.** Task 7 writes into the spec the two that change its text (rows a and e).

| | The spec and the boards | What gets built | Why |
|---|---|---|---|
| a | Italics for an expression default; the spec's Delivery row says "an italic text role and its two font files" | The same face, slanted: egui's `TextFormat::italics`. No font file | It is what the board itself shows: its page loads IBM Plex Mono at 400 and 500 and no italic face, so the browser slants the upright one. Task 7 corrects the Delivery row |
| b | `+ new` in the UI face, semibold | The marker stays in the grid's data face, in the board's `#1F6B35` | A cell's text is drawn in the grid's one data role. A second face for one cell is a new cell style for one word |
| c | The spec gives the saved flash (`#E3F1E6`) for a new row | Every cell a save wrote flashes it, a changed cell's too | One save can hold both. Two greens side by side for 1.2 s would read as two results |
| d | An expression default is told from a value | Exactly on PostgreSQL and SQLite, which write a string in its quotes. On MySQL by its text: a call (`(`) or `CURRENT_…` | MySQL writes a string default without quotes, and what says "this is an expression" (`DEFAULT_GENERATED`) is not in the structure. Reading it is a field on `ColumnInfo` for the driver's plan |
| e | Omarchy: `null` in a nullable cell (spec 3) | `NULL`, as every row of the Omarchy grid shows it today | The "Editing values" board writes `NULL`, the inserting board `null`. One grid should not show both. Accepted: `NULL` stays, and task 7 corrects the spec's table |
| f | The bar's dot is green on the inserting board | Green while new rows alone are pending, amber once a value of a loaded row is changed too | The editing boards draw it amber for changes |
| f2 | The tab's own dot (macOS and Windows) | Stays amber while only new rows are pending | It says that the tab holds something unsaved, whatever it is. `src/ui/object_tabs.rs` calls it "the dot the pending bar leads with": task 6 corrects that comment, since the two can now differ |
| g | Omarchy: `+1` at the header's right end | Left of the `d data  s structure` switch, which the board does not draw there | The switch is the app's and keeps its place |
| h | Dark mode on macOS: no board | The same greens as mixes of the dark palette's green (decision A7). Omarchy: 14% of the palette's green into the panel | As the spec says |
| i | `identity · assigned on :w`, `default`, `▪`, `now() on :w` | Not here | They are the inspector's on the board: run 2b |
| j | The red banner of a failed save, and the words of a failed cell | Not here | Run 5 |
| k | Omarchy's insert-mode line on the inserting board: `-- INSERT -- +1 new publisher_id required … tab next col · esc keep · :w save` | The app's order and keys stay (`-- INSERT -- publisher_id · INTEGER +1 new`, then what the row needs, then `esc normal · tab next cell`) | The "Editing values" board draws the line the app's way. The two boards disagree, and the line is every edit's, not a new row's |
| l | `:diff` on the inserting board: `preview`, `1 insert · 1 update`, `only set columns are sent · :w write · :e! discard all` | The panel's words stay (`pending · 1 new row`, `one transaction`, `esc close · Y copy sql · :w write`) | They are the value editing spec's panel, which the "Editing values" board draws. The spec's own decision A9 took the app's Review SQL |
| m | Omarchy after a save: `✓ inserted 1 · updated 1 · 5ms`, `new row is now id 15 · …` | Not here | Run 5 ("after the save") |
| n | Omarchy's header begins with the database's name (`bookshop_development · 13 rows + 1 new`) | Not here | No table's header names its database today. It is the header's, for every table, not a new row's |
| o | macOS: `Discard` on the inserting board | `Discard all` stays | The editing board says "Discard all" |

## File map

| File | What changes |
|---|---|
| `src/env.rs` | `NewRow`, `new_row_colors` (task 1). Not `new_row`: that is `edit`'s, for the row a new row's cells are kept under |
| `src/ui/grid.rs` | the row's fill, bar and marker from `new_row_colors`; the terminal's 2px bar; the saved flash (task 1); a default's tag at 60% (task 2); `Cell::slanted`, `paint_slanted` (task 3) |
| `src/edit.rs` | `default_is_expression`, `Unset::Expression` (task 3) |
| `src/ui/data_view.rs` | a new row's default cell (tasks 2, 3); `+1` in the terminal's header (task 5) |
| `src/ui/workspace.rs` | `Editing::needs`, said in insert mode (task 4) |
| `src/ui/pending_bar.rs` | the dot's colour, and what a row needs without the alert mark (task 6) |
| `src/ui/object_tabs.rs` | one comment (task 6) |
| `src/ui/mod.rs` (its `mod tests`) | the new tests (tasks 2, 5, 6) |
| `src/ui/insert_audit_tests.rs` | one test out of `ignore` (task 4) |
| `docs/superpowers/specs/2026-10-07-inserting-rows-design.md` | its Delivery table (task 7) |

---

### Task 1: The new row's colours, in one place

**Files:**
- Modify: `src/env.rs` (after `success_tint`)
- Modify: `src/ui/grid.rs` (the row's fill near line 662, the terminal's gutter near line 716, the cell's tint near line 815, the row's bar near line 846, the marker's colour near line 867; its tests near lines 1994 and 2172)

- [ ] **Step 1: Change the two tests that hold today's colours, and add the one for a selected row.** In `src/ui/grid.rs`, the test `a_new_row_is_filled_and_marked_in_the_success_tone` becomes `a_new_row_is_filled_and_marked_in_its_own_colours`. Replace its lines from `let (fill, color) = (` to the end of the `if look.terminal { ... } else { ... }` block with:

```rust
                let colors = crate::env::new_row_colors(crate::env::Platform::of(&look), &palette);
                // The row's own fill, as wide as the grid: no cell's tint.
                let filled = rects
                    .iter()
                    .any(|rect| rect.fill == colors.fill && rect.rect.width() > 400.0);
                assert!(filled, "{said}: the row's fill");
                // The bar at the row's left: 3 wide, and 2 in the terminal.
                let wide = if look.terminal { 2.0 } else { 3.0 };
                let bar = rects.iter().any(|rect| {
                    rect.fill == colors.bar
                        && rect.rect.width() == wide
                        && rect.rect.height() == look.grid_row
                });
                assert!(bar, "{said}: the row's bar");
                let signed = texts
                    .iter()
                    .any(|(text, painted, _)| text == "+" && *painted == palette.success);
                let marker = texts.iter().find(|(text, ..)| text == "r1c1");
                let marker = marker.map(|(_, color, _)| *color);
                if look.terminal {
                    // `+` in the gutter, and the marker dimmed.
                    assert!(signed, "{said}");
                    assert_eq!(marker, Some(palette.dim), "{said}");
                } else {
                    assert!(!signed, "{said}");
                    assert_eq!(marker, Some(colors.marker), "{said}");
                }
```

  (Delete the `use crate::ui::states::Tone;` at the test's top if nothing else in it reads `Tone`.) After that test, add:

```rust
    #[test]
    fn a_selected_new_row_keeps_its_fill_and_its_cell_takes_the_selection() {
        for look in Look::ALL {
            let palette = Palette::light();
            let said = look.name;
            let ctx = egui::Context::default();
            crate::theme::install(&ctx, false, &look);
            crate::theme::apply(&ctx, &palette, &look);
            marked(&ctx, &look, &palette, Mark::None, RowMark::None);
            let here = Some(CellPos { row: 1, col: 1 });
            let on = (Mark::None, RowMark::New);
            // A key, and the arrows are the grid's: its cell is lit, and
            // a row of the page would take the selection's tint.
            let key = crate::testing::key(egui::Key::ArrowDown, egui::Modifiers::NONE);
            marked_with(&ctx, &look, &palette, on, here, vec![key]);
            let (rects, _) = marked_with(&ctx, &look, &palette, on, here, Vec::new());
            let colors = crate::env::new_row_colors(crate::env::Platform::of(&look), &palette);
            let wide = |fill: egui::Color32| {
                rects
                    .iter()
                    .any(|rect| rect.fill == fill && rect.rect.width() > 400.0)
            };
            assert!(wide(colors.fill), "{said}: the row is still a new row's");
            // No row-wide fill in the selection's colour or its tint.
            let tint = palette.window.lerp_to_gamma(palette.selection, 0.6);
            assert!(!wide(tint) && !wide(palette.selection), "{said}");
            if !look.terminal {
                // The cell alone takes the selection's colour. (The
                // terminal's lit cell is the accent, in reverse.)
                let cell = rects
                    .iter()
                    .any(|rect| rect.fill == palette.selection && rect.rect.width() < 400.0);
                assert!(cell, "{said}: the selection is the cell's");
            }
        }
    }
```

  A third test of the file holds the colour of a cell a save wrote. In `a_marked_cell_is_tinted_and_its_row_is_marked` (near line 2172), the assertion under the comment "Written: green, and no mark on the row.", `behind(&rects, Tone::Success.fill(&look, &palette))`, becomes:

```rust
                let platform = crate::env::Platform::of(&look);
                let saved = crate::env::new_row_colors(platform, &palette).saved;
                assert!(behind(&rects, saved), "{said}: saved");
```

  These change what two existing tests expect: a new row is no longer in the success tone, and a saved cell flashes the green a saved new row does, by decision A7 of the spec and row c above. Say so in the commit.

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked --lib -- ui::grid::tests::a_new_row_is_filled ui::grid::tests::a_selected_new_row`
Expected: do not compile, "cannot find function `new_row_colors` in module `crate::env`".

- [ ] **Step 3: Write `env::new_row_colors`.** In `src/env.rs`, after `success_tint`:

```rust
/// A new row's colours in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewRow {
    /// Behind the row, for as long as it is not saved.
    pub fill: Color32,
    /// The bar at its left, and the dot of a bar that counts new rows.
    pub bar: Color32,
    /// What its marker (`+ new`) is written in.
    pub marker: Color32,
    /// Behind a cell a save just wrote, for as long as it shows it.
    pub saved: Color32,
}

/// The colours of a new row on `platform` with `palette`. macOS and
/// Windows take the design's fixed greens, the dev environment's, in the
/// light; in the dark, and in the terminal look, the palette's green is
/// mixed into what the rows stand on, so a theme's change is followed.
pub fn new_row_colors(platform: Platform, palette: &Palette) -> NewRow {
    let (base, badge, text) = native_table(Environment::Dev);
    match platform {
        Platform::Native if !palette.dark => NewRow {
            fill: Color32::from_rgb(0xee, 0xf7, 0xf0),
            bar: base,
            marker: text,
            saved: badge,
        },
        Platform::Native => NewRow {
            fill: mix(palette.window, palette.success, 0.10),
            bar: palette.success,
            marker: palette.success,
            saved: success_tint(platform, palette).0,
        },
        // 14% of the theme's green: the terminal look's mark of what is new.
        Platform::Omarchy => NewRow {
            fill: mix(palette.panel, palette.success, 0.14),
            bar: palette.success,
            marker: palette.dim,
            saved: success_tint(platform, palette).0,
        },
    }
}
```

- [ ] **Step 4: Draw the row with them.** In `src/ui/grid.rs`, in the closure that draws a row:

  Where the row's fill is chosen (`let fill = if lit_row {`), the new row comes first, selected or not:

```rust
                let fresh = crate::env::new_row_colors(crate::env::Platform::of(look), palette);
                let fill = if new {
                    // A new row keeps its own fill under the selection
                    // too: its cell alone takes the selection's colour,
                    // and says where the keyboard is.
                    Some(fresh.fill)
                } else if lit_row {
                    Some(palette.window.lerp_to_gamma(palette.selection, 0.6))
                } else {
                    row_fill(
                        selected_row,
                        response.hovered(),
                        row % 2 == 1,
                        look,
                        palette,
                    )
                };
```

  In the first column's block, the terminal's gutter (`if look.terminal { let line = rect.center().y;`, near line 716) begins with the bar. It is painted there and not with the row's fill: a pinned first column that the grid has scrolled under is filled again from the row's left, over anything painted before it.

```rust
                        if look.terminal {
                            if new {
                                // The terminal's bar: 2 wide, where the
                                // row begins.
                                let bar = Rect::from_min_size(lead.min, vec2(2.0, row_height));
                                painter.rect_filled(bar, CornerRadius::ZERO, fresh.bar);
                            }
                            let line = rect.center().y;
```

  Where a cell's tint is painted (`if let Some(tone) = tone {`):

```rust
                    if let Some(tone) = tone {
                        // What a save just wrote flashes in the colour a
                        // saved new row does, a changed cell's too.
                        let tint = match content.mark {
                            Mark::Saved => fresh.saved,
                            _ => tone.fill(look, palette),
                        };
                        painter.rect_filled(cell_rect, CornerRadius::ZERO, tint);
                    } else if here && lit {
```

  Where the row's bar takes its colour (`let color = match row_tone {`, near line 846; the bar near line 829 is a pending cell's and stays):

```rust
                        let color = match row_tone {
                            Some(_) if new => Some(fresh.bar),
                            Some(tone) => Some(tone.color(palette)),
                            None => (selected_row && !lit_row).then_some(palette.accent),
                        };
```

  Where the marker is written (`_ if content.mark == Mark::Added && !look.terminal => {`):

```rust
                        _ if content.mark == Mark::Added && !look.terminal => {
                            written_in(palette, fresh.marker)
                        }
```

- [ ] **Step 5: Run the tests, then the four checks.**

Run: `~/.cargo/bin/cargo test --locked --lib ui::grid::tests`
Expected: PASS. The tests of `src/ui/mod.rs` that hold a saved cell's colour run on the harness's dark palette, where `saved` is still the success tint: they pass unchanged.

- [ ] **Step 6: Commit.**

```bash
git add -A && git commit -m "Give a new row its own greens, and keep them under the selection

A new row was drawn in the success tone, and turned the selection's
blue while it was selected, which is the whole time its first value is
typed. Its colours are the design's now, in one place (env::new_row_colors): a
selected new row stays green and only its cell takes the selection, the
terminal look draws its 2 px bar, and a cell a save wrote flashes in the
same green whether its row was new or changed.

The grid's tests of a new row's colours and of a saved cell's hold the
new ones.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: A default the column lists is its tag, at 60%

**Files:**
- Modify: `src/ui/data_view.rs` (`Changes::new_cell`, the `Unset::Default` arm)
- Modify: `src/ui/grid.rs` (`draw_cell`, the `Style::Tag` arm)
- Test: `src/ui/mod.rs` (`mod tests`, after `a_new_rows_cells_say_what_the_database_will_do`)

- [ ] **Step 1: Write the failing test.**

```rust
    #[test]
    fn a_new_rows_default_is_its_tag_at_six_tenths() {
        for look in desktop_looks() {
            let (mut harness, tab, id) = covers_in(look);
            harness.app.workspace_mut(tab).unwrap().row_panel = false;
            let place = crate::edit::Place::Top;
            harness.app.apply(Action::AddRow { tab, id, place });
            harness.app.apply(Action::CancelEdit { tab, id });
            harness.settle();
            // `kind` is `print` in the page's first and third rows, and in
            // the new row as its default: the same tag, fainter.
            let prints: Vec<egui::Color32> = harness
                .painted
                .iter()
                .filter(|(text, _)| text == "print")
                .map(|(_, color)| *color)
                .collect();
            let faded = prints.iter().any(|faded| {
                prints
                    .iter()
                    .any(|full| faded != full && *faded == full.gamma_multiply(0.6))
            });
            assert!(faded, "{}: {prints:?}", look.name);
        }
    }
```

- [ ] **Step 2: Run it to see it fail.**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::a_new_rows_default_is_its_tag_at_six_tenths`
Expected: FAIL, `faded` false: the new row writes `print` in the dim tone.

- [ ] **Step 3: Give the default's cell its column's style.** In `src/ui/data_view.rs`, in `new_cell`, the arm `Some(Unset::Default(text)) => Cell { ... }` becomes:

```rust
                Some(Unset::Default(text)) => {
                    // Drawn as the value would be where its column lists
                    // it (a tag), and as plain text everywhere else.
                    let as_value = value(&Value::Text(text.as_str().into()));
                    let cell = match as_value.style {
                        grid::Style::Tag(_) => as_value,
                        _ => Cell {
                            text: format::cell_line(text, format::Marks::PLAIN)
                                .into_owned()
                                .into(),
                            ..Cell::default()
                        },
                    };
                    Cell {
                        mark: Mark::Unset,
                        hint: Some(say("from DEFAULT")),
                        ..cell
                    }
                }
```

  (Name the style as the file already does: `grid::Style::Tag(_)`, or `Style::Tag(_)` if `Style` is imported.)

- [ ] **Step 4: Fade a tag the row does not hold yet.** In `src/ui/grid.rs`, in `draw_cell`, the `Style::Tag(_)` arm begins `let (color, fill) = crate::ui::value_tags::style_colors(content.style, look, palette);`. Right after that line:

```rust
            // A value the row does not hold yet (a new row's default): its
            // tag at six tenths, as the design fades it. The terminal has
            // no tag to fade and writes it dimmed.
            let (color, fill) = match fill {
                Some(fill) if content.mark == Mark::Unset => {
                    (color.gamma_multiply(0.6), Some(fill.gamma_multiply(0.6)))
                }
                None if content.mark == Mark::Unset => (palette.dim, None),
                fill => (color, fill),
            };
```

- [ ] **Step 5: Run the test, then the four checks.**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::a_new_rows_default`
Expected: PASS. `a_new_rows_cells_say_what_the_database_will_do` still passes: it asks that `CURRENT_TIMESTAMP` is painted, and nothing of `print`'s colour. A default that is a truth value (`true`) stays plain dimmed text: a boolean's tag is drawn from a boolean, and a default is text.

- [ ] **Step 6: Commit.**

```bash
git add -A && git commit -m "Draw a new row's default as its tag, at six tenths

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: A default the database works out is slanted

**Files:**
- Modify: `src/edit.rs` (`Unset`, `unset`, a new `default_is_expression`; its tests)
- Modify: `src/ui/grid.rs` (`Cell::slanted`, `paint_slanted`, the `Style::Plain` arm of `draw_cell`)
- Modify: `src/ui/data_view.rs` (`Changes::new_cell`)

- [ ] **Step 1: Write the failing tests.** In `src/edit.rs`, in `mod tests`, after `a_default_reads_as_its_value_or_as_its_expression`:

```rust
    #[test]
    fn a_default_is_a_value_or_what_the_database_works_out() {
        let works_out = default_is_expression;
        // PostgreSQL and SQLite write a string in its quotes.
        for dialect in [Dialect::Postgres, Dialect::Sqlite] {
            for value in ["'print'", "0", "-1", "3.5", "true", "FALSE"] {
                assert!(!works_out(dialect, value), "{dialect:?} {value}");
            }
            for expression in ["CURRENT_TIMESTAMP", "(1 + 1)", "now()"] {
                assert!(works_out(dialect, expression), "{dialect:?} {expression}");
            }
        }
        assert!(!works_out(Dialect::Postgres, "'print'::character varying"));
        // More than a string and its cast: shown whole, and worked out.
        assert!(works_out(Dialect::Postgres, "'a'::text || 'b'::text"));
        assert!(works_out(Dialect::Postgres, "nextval('covers_id_seq'::regclass)"));
        // MySQL writes a string without them: only a call, or the moment.
        for value in ["print", "0", "current"] {
            assert!(!works_out(Dialect::MySql, value), "{value}");
        }
        for expression in ["CURRENT_TIMESTAMP", "CURRENT_TIMESTAMP(6)", "uuid()", "(now())"] {
            assert!(works_out(Dialect::MySql, expression), "{expression}");
        }
    }
```

  In `an_unset_cell_says_what_the_database_will_do`, the expectation for `created_at` (the column whose default is `CURRENT_TIMESTAMP`) changes from `Unset::Default("CURRENT_TIMESTAMP".into())` to `Unset::Expression("CURRENT_TIMESTAMP".into())`, and `kind` stays `Unset::Default("print".into())`. This changes what an existing test expects: an expression default is its own case now, so the grid can slant it. Say so in the commit.

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked --lib -- edit::tests::a_default_is_a_value edit::tests::an_unset_cell`
Expected: do not compile, "cannot find function `default_is_expression`" and "no variant named `Expression`".

- [ ] **Step 3: Tell a value from an expression.** In `src/edit.rs`, `Unset` gains a variant after `Default`:

```rust
    /// The column's default where the database works it out for each row
    /// (`now()`, `CURRENT_TIMESTAMP`): the expression as it is written.
    Expression(String),
```

  `unset` becomes:

```rust
pub fn unset(dialect: Dialect, column: &ColumnInfo) -> Unset {
    if column.identity || column.generated {
        return Unset::Assigned;
    }
    let written = column.default.as_deref();
    let shown = written.and_then(|default| default_shown(dialect, default));
    match (shown, written) {
        (Some(text), Some(written)) if default_is_expression(dialect, written) => {
            Unset::Expression(text)
        }
        (Some(text), _) => Unset::Default(text),
        (None, _) if column.nullable => Unset::Null,
        (None, _) => Unset::Required,
    }
}
```

  and after `default_shown`:

```rust
/// Whether `default`, a column's default as the database writes it, is
/// worked out by the database for each row and is no value it holds.
/// PostgreSQL and SQLite write a string in its quotes: there a default is
/// a value where [`default_shown`] reads a string out of it, or where it
/// is a number or a truth value, and an expression otherwise. MySQL writes
/// a string without its quotes: only what reads as a call or as the moment
/// is taken for one.
pub fn default_is_expression(dialect: Dialect, default: &str) -> bool {
    let text = default.trim();
    let value = |text: &str| {
        text.parse::<f64>().is_ok()
            || ["true", "false", "null"]
                .iter()
                .any(|word| text.eq_ignore_ascii_case(word))
    };
    match dialect {
        // Shown as it is written: no string was read out of it.
        Dialect::Postgres | Dialect::Sqlite => {
            default_shown(dialect, text).is_some_and(|shown| shown == text) && !value(text)
        }
        Dialect::MySql => text.contains('(') || text.to_ascii_uppercase().starts_with("CURRENT_"),
    }
}
```

- [ ] **Step 4: Let a cell be slanted.** In `src/ui/grid.rs`, `Cell` gains a field after `mark`:

```rust
    /// Written slanted: text that stands for what the database will work
    /// out (a new row's `now()`), and is no value yet.
    pub slanted: bool,
```

  After `fn paint`:

```rust
/// [`paint`], slanted. The same face leaned over, as the design's own
/// page draws it: it loads no italic one.
#[allow(clippy::too_many_arguments)]
fn paint_slanted(
    painter: &egui::Painter,
    ui: &Ui,
    role: TextRole,
    text: &str,
    color: egui::Color32,
    x: f32,
    y: f32,
    right: bool,
    look: &Look,
) {
    let laid = Text::new(look)
        .add_with(role, text, 0.0, |format| {
            format.color = color;
            format.italics = true;
        })
        .layout(ui.ctx());
    if right {
        laid.paint_right(painter, x, y);
    } else {
        laid.paint_left(painter, x, y);
    }
}
```

  (`paint` above it has the same nine arguments: give `paint_slanted` whatever `allow` or none `paint` carries, so the two agree.) In `draw_cell`, the last line of the `Style::Plain` arm, `paint(&clip, ui, role, &shown, color, x, center, numeric, look);`, becomes:

```rust
            if content.slanted {
                paint_slanted(&clip, ui, role, &shown, color, x, center, numeric, look);
            } else {
                paint(&clip, ui, role, &shown, color, x, center, numeric, look);
            }
```

- [ ] **Step 5: Slant an expression in a new row.** In `src/ui/data_view.rs`, in `new_cell`, after the `Unset::Default` arm:

```rust
                Some(Unset::Expression(text)) => Cell {
                    text: format::cell_line(text, format::Marks::PLAIN)
                        .into_owned()
                        .into(),
                    mark: Mark::Unset,
                    slanted: true,
                    hint: Some(say("from DEFAULT")),
                    ..Cell::default()
                },
```

  `new_cell` is the only `match` on `Unset`. One place builds a `Cell` field by field and must carry the new one: `kept` (near line 1660) gains `slanted: cell.slanted,`. The compiler names it (E0063), and any other.

  No slant is tested headlessly: the harness records a painted text and its colour, not its face. Task 7 looks at it.

- [ ] **Step 6: Run the tests, then the four checks.**

Run: `~/.cargo/bin/cargo test --locked --lib edit::tests`
Expected: PASS.

- [ ] **Step 7: Commit.**

```bash
git add -A && git commit -m "Slant a default the database works out for each row

A new row showed now() as it showed 'print': dimmed. What the database
works out is its own case of an unset cell now, told from a value by how
each engine writes a default, and the grid writes it slanted, in the
same face leaned over, as the design's own page does.

The test of what an unset cell says holds the new case for created_at.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The terminal says what the row needs while it is typed

**Files:**
- Modify: `src/ui/workspace.rs` (`Editing`, `editing_status`, the insert-mode line)
- Test: `src/ui/insert_audit_tests.rs` (take `the_terminals_line_says_what_the_row_needs_while_it_is_typed` out of `ignore`)

- [ ] **Step 1: Take the test out of `ignore`.** In `src/ui/insert_audit_tests.rs`, on `the_terminals_line_says_what_the_row_needs_while_it_is_typed`: delete its `#[ignore = "INS-07b: ..."]` line, and reword the first line of its doc comment from "INS-07b. The Omarchy board's status line says" to "Spec 3: the Omarchy status line says".

- [ ] **Step 2: Run it to see it fail.**

Run: `~/.cargo/bin/cargo test --locked --lib ui::insert_audit_tests::the_terminals_line_says_what_the_row_needs_while_it_is_typed`
Expected: FAIL, `painted(&harness, "publisher_id required")` is false.

- [ ] **Step 3: Carry what the row needs to the line.** In `src/ui/workspace.rs`, `Editing` gains a field after `errors`:

```rust
    /// What the tab's new rows still need, for insert mode, which has no
    /// room for why a save waits: "publisher_id required".
    needs: Option<String>,
```

  In `editing_status`, before the `let blocked = edits` statement:

```rust
    // What a new row still needs, by its column: said while a value is
    // typed too.
    let needs = {
        let lacking = app.lacking(tab, object.id);
        match lacking.as_slice() {
            [] => None,
            [one] => Some(format!("{one} {}", look.label(&say("required")))),
            more => Some(look.label(&format!("{} {}", more.len(), say("values required")))),
        }
    };
```

  and `needs,` in the `Editing { ... }` literal that ends the function. (The `SaveBlock::Required` arm of `blocked` words the same thing: leave it as it is, or have it read `needs.clone().unwrap_or_default()` if that reads better once it compiles.)

- [ ] **Step 4: Say it in insert mode.** In the insert-mode branch (`if let Some(column) = &editing.insert {`), the line `x = counts(ui, x);` and the two after it become:

```rust
                x = counts(ui, x);
                if let Some(needs) = &editing.needs {
                    // In the danger tone, as the header's star is, where
                    // there is room before the keys.
                    let danger = states::Tone::Danger.color(&palette);
                    let said = || Text::one(&look, role, needs, danger);
                    if x + widgets::measure(ui, said()) <= right {
                        x += widgets::paint_label(ui, x, y, said()) + gap;
                    }
                }
                if named && x + keys_width <= right {
                    widgets::paint_text_right(ui, right, y, keys());
                }
                return;
```

- [ ] **Step 5: Run the test, then the four checks.**

Run: `~/.cargo/bin/cargo test --locked --lib ui::insert_audit_tests`
Expected: PASS, with one test fewer ignored.

- [ ] **Step 6: Commit.**

```bash
git add -A && git commit -m "Say in insert mode what a new row still needs

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: `+1` in the terminal's header

**Files:**
- Modify: `src/ui/data_view.rs` (`header`, its `if look.terminal { ... }` branch)
- Test: `src/ui/mod.rs` (`mod tests`, after `the_terminals_line_counts_new_rows_and_says_what_one_needs`)

- [ ] **Step 1: Write the failing test.**

```rust
    #[test]
    fn the_terminals_header_counts_the_rows_to_add() {
        let (mut harness, tab, id) = covers_in(Look::omarchy());
        harness.app.workspace_mut(tab).unwrap().row_panel = false;
        let palette = harness.app.palette;
        harness.settle();
        assert!(!painted(&harness, "+1"));
        for count in ["+1", "+2"] {
            select(&mut harness, tab, id, (0, 1));
            type_key(&mut harness, Key::O, "o");
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(
                painted_in(&harness, count, palette.success),
                "{count}: {:?}",
                harness.painted
            );
        }
        // Gone with the rows.
        harness.app.apply(Action::DiscardEdits { tab, id });
        harness.settle();
        assert!(!painted(&harness, "+2"));
    }
```

- [ ] **Step 2: Run it to see it fail.**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::the_terminals_header_counts_the_rows_to_add`
Expected: FAIL at `+1`: the status line paints `+1 new`, one piece, and nothing paints `+1`.

- [ ] **Step 3: Paint it.** In `src/ui/data_view.rs`, in `header`, beside `let view = object.view;`:

```rust
    // The rows to add: the terminal's header counts them at its right.
    let added = object.edits.added.len();
```

  In the terminal branch, the statement `let room = right - (...) - 4.0 - 16.0 - left;` becomes:

```rust
                // `+1`, before the views: the summary gives way to it too.
                let added = (added > 0).then(|| format!("+{added}"));
                let added_room = added.as_ref().map_or(0.0, |text| width(role, text) + 14.0);
                let room = right
                    - (widths.iter().sum::<f32>() + 14.0 * (widths.len() - 1) as f32)
                    - 4.0
                    - 16.0
                    - added_room
                    - left;
```

  and after the `for ((target, label, key), width) in views.iter().zip(widths).rev() { ... }` loop, before the branch's `return;`:

```rust
                // In the green of a new row, 14 before the first view:
                // the loop left `x` there.
                if let Some(text) = &added {
                    let text = Text::one(&look, role, text, palette.success);
                    widgets::paint_text_right(ui, x, center, text);
                }
```

- [ ] **Step 4: Run the test, then the four checks.**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::the_terminals_header`
Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add -A && git commit -m "Count the rows to add in the terminal's header

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The pending bar's dot, and what a row needs

**Files:**
- Modify: `src/ui/pending_bar.rs` (`show`: the dot near line 444, what is to fix near line 298 and line 492)
- Modify: `src/ui/object_tabs.rs` (the comment on `MARK`, near line 288)
- Test: `src/ui/mod.rs` (`mod tests`, after `the_bar_counts_new_rows_and_says_what_one_still_needs`)

- [ ] **Step 1: Write the failing test.**

```rust
    #[test]
    fn the_bars_dot_is_a_new_rows_green_while_only_rows_are_pending() {
        let (mut harness, tab, id) = covers_in(Look::macos());
        let palette = harness.app.palette;
        let look = harness.app.look;
        let green = crate::env::new_row_colors(crate::env::Platform::of(&look), &palette).bar;
        let amber = Tone::Warning.color(&palette);
        // A circle of the dot's size in `color`, in the bar: the tab above
        // has an amber dot of its own for anything unsaved.
        let dot = |harness: &Harness, color: egui::Color32| {
            let low = harness.size.y - 90.0;
            harness.fills.iter().any(|(rect, fill)| {
                *fill == color
                    && rect.width() == rect.height()
                    && rect.width() <= 10.0
                    && rect.top() > low
            })
        };
        let place = crate::edit::Place::Top;
        harness.app.apply(Action::AddRow { tab, id, place });
        harness.app.apply(Action::CancelEdit { tab, id });
        harness.settle();
        assert!(dot(&harness, green) && !dot(&harness, amber));
        // What the row needs is said after a dot of its own, and no mark.
        assert!(painted_in(&harness, "·", palette.danger));
        assert!(harness.has("publisher_id is required"));
        // A changed value of a loaded row makes the bar a bar of changes.
        make_pending(&mut harness, tab, id, (1, 2), "audio");
        harness.settle();
        assert!(dot(&harness, amber) && !dot(&harness, green));
    }
```

- [ ] **Step 2: Run it to see it fail.**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::the_bars_dot`
Expected: FAIL at the first assertion: the dot is amber.

- [ ] **Step 3: Colour the dot.** In `src/ui/pending_bar.rs`, where the dot is painted, `Tone::Warning.color(&palette)` becomes `dot_color`, defined right before `if dot.right() <= right {`:

```rust
            // Green while nothing but new rows is pending, as a new row
            // is: amber once a value of a loaded row is changed too.
            let dot_color = if counts.added > 0 && counts.changes == 0 {
                crate::env::new_row_colors(crate::env::Platform::of(&look), &palette).bar
            } else {
                Tone::Warning.color(&palette)
            };
```

- [ ] **Step 4: Say what a row needs without the alert mark.** Where `fix` and `needs` are joined (`let fix = match (fix, &needs) {`), right before that statement:

```rust
            // Nothing is to fix, and a new row needs a value: no mistake
            // to mark. A dot and the words, as the design has it.
            let asked = fix.is_none() && needs.is_some();
```

  and where it is painted (`if let Some(fix) = fix.as_ref().filter(|_| fixes) {`), the block's body becomes:

```rust
                if asked {
                    // Painted and not named: a screen reader has the
                    // words, and a lone dot says nothing.
                    let dot = Text::one(&look, body, "·", palette.danger);
                    x += widgets::paint_text(ui, x, y, dot) + 5.0;
                } else {
                    let mark = Rect::from_center_size(pos2(x + MARK / 2.0, y), vec2(MARK, MARK));
                    Icon::CircleAlert
                        .image(palette.danger, MARK)
                        .paint_at(ui, mark);
                    x += MARK + 5.0;
                }
                x += widgets::paint_label(ui, x, y, Text::one(&look, body, fix, palette.danger))
                    + APART;
```

  The room the bar keeps for the mark (`MARK + 5.0`, where what is to fix is measured) is a little more than the dot takes: leave it.

  In `src/ui/object_tabs.rs`, the comment on `MARK` (near line 288), "The unsaved mark's size: the dot the pending bar leads with.", becomes "The unsaved mark's size: as large as the dot the pending bar leads with. Amber whatever is unsaved: the bar's own turns green for new rows alone."

- [ ] **Step 5: Run the test, then the four checks.**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::the_bar`
Expected: PASS, the new test and `the_bar_counts_new_rows_and_says_what_one_still_needs`.

- [ ] **Step 6: Commit.**

```bash
git add -A && git commit -m "Lead a bar of new rows with their green, and ask without a mark

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Look at it, and say that it is built

**Files:**
- Modify (and restore): `src/shots.rs` (a throwaway scene)
- Modify: `docs/superpowers/specs/2026-10-07-inserting-rows-design.md` (the table under "Delivery")

- [ ] **Step 1: Render the new row off screen.** Append a throwaway `#[test] #[ignore]` to `src/shots.rs`, built on `Harness::for_shots(size, 1.0, light, look)`: `harness.book_covers()`, the grid's pane (`workspace.pane = Pane::Grid`), then `Action::AddRow` at `Place::Top` (macOS) or `Place::Below(0)` (Omarchy). Render three states of each look, at the boards' sizes (1440 x 900 light for macOS, 1920 x 1080 dark for Omarchy): the row as it is added (its first cell open), the row with the selection on another row, and the row saved (answer the save with `WriteOutcome::Written` and an `inserted` row). Write the PNGs under `target/shots/`.

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots::<the scene> -- --ignored --exact`
Expected: the PNGs are written. `wgpu` renders without a display here.

- [ ] **Step 2: Compare by eye with the boards**, read again from the canvas. On macOS: the row's fill, the 3 px bar and the marker's green; the row still green with its first cell open; `print` as a faded tag; the expression default slanted; the bar's green dot and "· publisher_id is required"; and a new row selected with the grid not holding the keyboard (a click elsewhere first), where no cell is lit. On Omarchy: the 2 px bar, `+1` left of the views, `publisher_id required` in the insert-mode line, the default dimmed and the expression slanted. Write down what differs, with the measured colour or the text, for the pull request's description. A difference this plan's table does not list is a finding: stop and say so.

- [ ] **Step 3: Remove the scene.** `git checkout src/shots.rs`, and check `git status` shows no PNG and no change under `src/shots.rs`. Then the shots' clippy, since the scenes build `Cell`s and read `Unset`:

Run: `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`
Expected: passes. If a scene builds a `Cell` field by field, it needs `slanted: false`.

- [ ] **Step 4: Say that it is built.** In the spec's Delivery table, the "Fixes" row becomes:

```markdown
| Fixes | What run 2 built and the spec draws otherwise: the row's colours, a selected new row that stays green, a default as its tag at 60%, an expression default slanted (the same face leaned over: the boards load no italic one), Omarchy's 2px bar, its `+1` in the header and what the row needs in insert mode, and of the copy the audit lists the bar's green dot and its words without a mark. The rest of that list is left where the plan's table says: to run 2b, to run 5, or to the board (`docs/superpowers/plans/2026-10-08-inserting-rows-fixes.md`) | built |
```

  In "Decisions of 2026-10-08", A5's "Why" becomes: "The same face, slanted. The boards load no italic one, so no font file is added".

  In section 3's table, the Omarchy cell of "Nullable, no default" becomes `NULL` (it reads `null`), and under the table add: "Omarchy writes `NULL` as every row of its grid does: the inserting board's lower case is the board's to change."

- [ ] **Step 5: Run the four checks.** Expected: all pass. `docs/_guide/editing-data.md` already says a new row is green until it is saved: no page changes.

- [ ] **Step 6: Commit.**

```bash
git add -A && git commit -m "Say that the fixes to a new row's look are built

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## What this leaves

- **By hand, for the user:** the app in a real macOS window and on Omarchy. The tooltips ("from DEFAULT", "Assigned by the database on save") and the slant on a real screen are not seen from a session.
- **Run 2b** draws the inspector's labels (`default`, `▪`, `identity · assigned on :w`, `now() on :w`), and the pinned row on macOS.
- The pull request's description lists the finding each task closes (INS-06a, INS-06c, INS-06d, INS-07b, INS-08a, INS-08b, INS-10b), the test taken out of `ignore`, the three existing tests whose expectations changed and why, and what step 2 of task 7 found.
